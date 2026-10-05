#!/usr/bin/env python3
"""Cell One factory steps: quant ladder, grammar-constrained eval, determinism check (schema cell-one.quant-ladder.v0).

Generic and opt-in. One YAML config names the model, quant levels, eval set (path + sha256), tasks (prompt template,
GBNF grammar or JSON schema), decode settings, repeats and a pluggable scorer command. Domain logic lives only in the
scorer command, never here. Local only: not part of make smoke, make gate-90 or GitHub Actions. Does not seat, promote,
apply, run Ollama or download weights (the HF model must already be in the local HF cache or a local directory).

  factory_steps.py build        --config cfg.yaml   # HF -> f16 GGUF (convert_hf_to_gguf.py) -> llama-quantize per quant
  factory_steps.py eval         --config cfg.yaml   # llama-server per quant x repeat, GBNF / JSON schema, greedy, batch-1
  factory_steps.py determinism  --config cfg.yaml   # byte-compare outputs + token ids across repeats -> pass/fail
  factory_steps.py compare      --config cfg.yaml   # ladder report vs the reference quant (scores, flips, agreement, latency)
  factory_steps.py train        --config recipe.yaml [--dry-run|--status] [--wait] [--max-minutes N]   # LLaMA-Factory train
  factory_steps.py export       --config export.yaml [--dry-run|--status] [--wait] [--max-minutes N]   # LLaMA-Factory export

Scorer contract: the command (placeholders {python} {run_dir} {metrics} {eval} {config_dir} {out_dir} {quant} {repeat})
reads the task prediction files in {run_dir} (<task>.jsonl: {id, output, format, input_hash, timings...}) and writes
{metrics} as JSON: {"summary": {name: number|string}, "items": {id: {field: value}}}.
"""
from __future__ import annotations

import argparse, datetime, hashlib, json, os, shlex, signal, statistics, subprocess, sys, time, urllib.request
from pathlib import Path

import yaml

SCHEMA = "cell-one.quant-ladder.v0"
FLOAT_TYPES = {"f16": "f16", "bf16": "bf16", "f32": "f32"}


def now() -> str:
    return datetime.datetime.now().astimezone().isoformat(timespec="seconds")


def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for b in iter(lambda: f.read(1 << 22), b""):
            h.update(b)
    return h.hexdigest()


def refuse(msg: str, code: int = 2):
    print(f"refuse:{msg}", file=sys.stderr)
    sys.exit(code)


class Cfg:
    def __init__(self, path: str):
        self.path = Path(path).resolve()
        self.dir = self.path.parent
        self.c = yaml.safe_load(self.path.read_text())
        if self.c.get("schema") != SCHEMA:
            refuse(f"config: schema must be {SCHEMA}")
        self.out = self.p(self.c.get("out_dir", "."))
        for d in ("gguf", "manifests", "logs", "prompts", "outputs", "metrics"):
            (self.out / d).mkdir(parents=True, exist_ok=True)

    def p(self, v) -> Path:
        v = Path(os.path.expanduser(str(v)))
        return v if v.is_absolute() else (self.dir / v).resolve()

    def get(self, *keys, default=None):
        x = self.c
        for k in keys:
            if not isinstance(x, dict) or k not in x:
                return default
            x = x[k]
        return x

    @property
    def quants(self):
        return list(self.c["quants"])

    @property
    def base(self):
        return self.get("model", "gguf_basename") or self.get("model", "hf_id").split("/")[-1]

    def gguf(self, q: str) -> Path:
        return self.out / "gguf" / f"{self.base}-{q}.gguf"


def hf_dir(cfg: Cfg) -> Path:
    if cfg.get("model", "hf_path"):
        d = cfg.p(cfg.get("model", "hf_path"))
    else:
        hub = Path(os.path.expanduser(os.environ.get("HF_HUB_CACHE", "~/.cache/huggingface/hub")))
        repo = hub / ("models--" + cfg.get("model", "hf_id").replace("/", "--"))
        snaps = sorted((repo / "snapshots").glob("*")) if (repo / "snapshots").is_dir() else []
        if len(snaps) != 1:
            refuse(f"model: need exactly one cached snapshot under {repo} (found {len(snaps)}); this step never downloads")
        d = snaps[0]
    if not (d / "config.json").exists():
        refuse(f"model: no config.json in {d}")
    return d


def tool(cfg: Cfg, key: str, default: str) -> Path:
    return cfg.p(cfg.get("toolchain", key, default=default))


def run_logged(cmd: list[str], log: Path, nice: bool = True):
    full = (["nice", "-n", "5"] if nice else []) + cmd
    with open(log, "a") as f:
        f.write(f"\n$ {shlex.join(full)}  # {now()}\n")
        f.flush()
        r = subprocess.run(full, stdout=f, stderr=subprocess.STDOUT)
    if r.returncode != 0:
        refuse(f"command failed ({r.returncode}), see {log}: {shlex.join(cmd)}", 1)


# ------------------------------------------------------------------ build (quant ladder)
def cmd_build(cfg: Cfg, a):
    src = hf_dir(cfg)
    lc = tool(cfg, "llama_cpp_dir", "~/llama.cpp")
    py = tool(cfg, "convert_python", "~/venvs/llamafactory/bin/python")
    qbin = tool(cfg, "quantize_bin", "~/llama.cpp/build/bin/llama-quantize")
    log = cfg.out / "logs" / "build.log"
    float_q = next((q for q in cfg.quants if q in FLOAT_TYPES), "f16")
    fsrc = cfg.gguf(float_q)
    conv = tool(cfg, "convert_script", "") if cfg.get("toolchain", "convert_script") else None
    if conv is not None and not conv.is_file():   # optional drop-in for convert_hf_to_gguf.py (same CLI); default unchanged
        refuse(f"toolchain.convert_script: {conv} is not a file")
    if conv is not None and not fsrc.exists():
        run_logged([str(py), str(conv), str(src), "--outfile", str(fsrc), "--outtype", FLOAT_TYPES[float_q],
                    *[str(x) for x in (cfg.get("toolchain", "convert_args", default=None) or [])]], log)
    # optional extra convert_hf_to_gguf.py flags, e.g. [--no-mtp] for a merged Qwen3.5 export whose config still declares
    # mtp_num_hidden_layers but whose save_pretrained dropped the mtp.* tensors (else llama.cpp fails: blk.N tensor not found)
    xargs = [str(x) for x in (cfg.get("toolchain", "convert_args", default=None) or [])]
    if not fsrc.exists():
        run_logged([str(py), str(lc / "convert_hf_to_gguf.py"), str(src), "--outfile", str(fsrc), "--outtype", FLOAT_TYPES[float_q], *xargs], log)
    for q in cfg.quants:
        if q in FLOAT_TYPES or cfg.gguf(q).exists():
            continue
        run_logged([str(qbin), str(fsrc), str(cfg.gguf(q)), q, str(cfg.get("toolchain", "quantize_threads", default=8))], log)
    man = {"schema": SCHEMA + ".gguf-manifest", "ts": now(), "hf_id": cfg.get("model", "hf_id"), "hf_snapshot": str(src),
           "hf_files": {}, "gguf": {}, "llama_cpp_commit": subprocess.run(["git", "-C", str(lc), "describe", "--tags", "--always"],
                                                                         capture_output=True, text=True).stdout.strip(),
           "convert": shlex.join([f"{lc}/convert_hf_to_gguf.py", "--outtype", FLOAT_TYPES[float_q], *xargs]), "quantize_bin": str(qbin),
           "imatrix": None}
    if conv is not None:
        man["convert"] = shlex.join([str(conv), "--outtype", FLOAT_TYPES[float_q], *xargs])
        man["convert_script_sha256"] = sha256_file(conv)
    for f in sorted(src.iterdir()):
        man["hf_files"][f.name] = sha256_file(f.resolve())
    for q in cfg.quants:
        g = cfg.gguf(q)
        man["gguf"][q] = {"path": str(g), "bytes": g.stat().st_size, "sha256": sha256_file(g)}
    (cfg.out / "manifests" / "gguf.json").write_text(json.dumps(man, indent=1))
    mirror = cfg.get("gguf_mirror")
    if mirror:
        m = cfg.p(mirror)
        m.mkdir(parents=True, exist_ok=True)
        for q in cfg.quants:
            dst = m / cfg.gguf(q).name
            if not dst.exists():
                subprocess.run(["cp", str(cfg.gguf(q)), str(dst)], check=True)
        (m / "gguf.json").write_text(json.dumps(man, indent=1))
    print(json.dumps({q: man["gguf"][q]["sha256"] for q in cfg.quants}, indent=1))


# ------------------------------------------------------------------ prompts
def build_prompts(cfg: Cfg) -> dict:
    from transformers import AutoTokenizer
    ev = cfg.p(cfg.get("eval", "set"))
    want = cfg.get("eval", "sha256")
    got = sha256_file(ev)
    if want and got != want:
        refuse(f"eval: sha256 mismatch for {ev}: {got} != {want}")
    tok = AutoTokenizer.from_pretrained(str(hf_dir(cfg)))
    rows = [json.loads(l) for l in ev.read_text().splitlines() if l.strip()]
    ctx = int(cfg.get("decode", "ctx", default=4096))
    meta = {"eval_set": str(ev), "eval_sha256": got, "n_rows": len(rows), "tasks": {},
            "chat_template_sha256": hashlib.sha256((tok.chat_template or "").encode()).hexdigest()}
    base_kw = dict(cfg.get("eval", "chat_template_kwargs", default=None) or {})
    if base_kw:
        meta["chat_template_kwargs"] = base_kw
    for t in cfg.get("eval", "tasks"):
        # optional chat_template_kwargs (eval-level, task-level overrides), passed to apply_chat_template so the eval prompt
        # matches the training render (e.g. {enable_thinking: false} for Qwen3 trained with LLaMA-Factory enable_thinking false)
        kw = {**base_kw, **(t.get("chat_template_kwargs") or {})}
        system = cfg.p(t["system_file"]).read_text() if t.get("system_file") else t.get("system")
        for k in ("grammar", "json_schema"):
            if t.get(k) and t.get(k + "_sha256") and sha256_file(cfg.p(t[k])) != t[k + "_sha256"]:
                refuse(f"task {t['name']}: {k} sha256 mismatch")
        out = []
        for r in rows:
            if t.get("where_field") and not r.get(t["where_field"]):
                continue
            user = t["user_template"].format_map({k: ("" if v is None else v) for k, v in r.items()})
            msgs = ([{"role": "system", "content": system}] if system else []) + [{"role": "user", "content": user}]
            text = tok.apply_chat_template(msgs, tokenize=False, add_generation_prompt=True, **kw)
            n = len(tok(text, add_special_tokens=False)["input_ids"])
            if n + int(t.get("n_predict", 64)) > ctx:
                refuse(f"task {t['name']}: item {r['id']} needs {n}+{t.get('n_predict', 64)} tokens > ctx {ctx}")
            out.append({"id": r["id"], "task": t["name"], "prompt": text, "input_hash": hashlib.sha256(text.encode()).hexdigest(), "n_prompt_tokens": n})
        (cfg.out / "prompts" / f"{t['name']}.jsonl").write_text("".join(json.dumps(x) + "\n" for x in out))
        meta["tasks"][t["name"]] = {"n": len(out), "max_prompt_tokens": max((x["n_prompt_tokens"] for x in out), default=0),
                                    **({"chat_template_kwargs": kw} if kw else {}),
                                    "file_sha256": sha256_file(cfg.out / "prompts" / f"{t['name']}.jsonl")}
    (cfg.out / "manifests" / "prompts.json").write_text(json.dumps(meta, indent=1))
    return meta


# ------------------------------------------------------------------ GPU guard
def gpu_check(cfg: Cfg, label: str):
    q = lambda *a: subprocess.run(["nvidia-smi", *a], capture_output=True, text=True).stdout.strip()  # noqa: E731
    lim = int(cfg.get("gpu_guard", "max_util_pct", default=30))
    for _ in range(int(cfg.get("gpu_guard", "settle_samples", default=10))):
        # utilization is a trailing average: after our own server exits it can read high for a few seconds with no
        # compute app left, so re-sample (3 s apart) before calling the GPU busy
        apps = q("--query-compute-apps=pid,process_name,used_memory", "--format=csv,noheader")
        util = int((q("--query-gpu=utilization.gpu", "--format=csv,noheader,nounits").splitlines() or ["0"])[0])
        if apps or util <= lim:
            break
        time.sleep(3)
    line = f"{now()} {label} util={util}% compute_apps=[{apps}]"
    with open(cfg.out / "logs" / "gpu-checks.log", "a") as f:
        f.write(line + "\n" + q("--query-gpu=utilization.gpu,memory.used,clocks.sm,temperature.gpu,power.draw", "--format=csv") + "\n")
    print(line)
    if apps or util > lim:
        refuse(f"gpu-busy: {line}", 3)


# ------------------------------------------------------------------ eval
def post(url: str, body: dict, timeout=600) -> dict:
    req = urllib.request.Request(url, data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
    return json.loads(urllib.request.urlopen(req, timeout=timeout).read())


def run_one(cfg: Cfg, q: str, rep: int, prompts: dict):
    d = cfg.get("decode", default={})
    port = int(d.get("port", 18431))
    srv = tool(cfg, "server_bin", "~/llama.cpp/build-cuda/bin/llama-server")
    rd = cfg.out / "outputs" / q / f"rep{rep}"
    rd.mkdir(parents=True, exist_ok=True)
    cmd = [str(srv), "-m", str(cfg.gguf(q)), "-ngl", str(d.get("n_gpu_layers", 999)), "-c", str(d.get("ctx", 4096)), "-np", "1",
           "-fa", str(d.get("flash_attn", "off")), "-t", str(d.get("threads", 8)), "-tb", str(d.get("threads", 8)),
           "-b", str(d.get("batch", 2048)), "-ub", str(d.get("ubatch", 512)), "--seed", str(d.get("seed", 42)),
           "--cache-ram", "0", "--no-webui", "--host", "127.0.0.1", "--port", str(port)]
    gpu_check(cfg, f"before {q} rep{rep}")
    log = open(cfg.out / "logs" / f"server-{q}-rep{rep}.log", "w")
    log.write(f"$ {shlex.join(cmd)}  # {now()}\n"); log.flush()
    proc = subprocess.Popen(cmd, stdout=log, stderr=subprocess.STDOUT)
    t0 = time.time()
    try:
        for _ in range(300):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=2)
                break
            except Exception:
                if proc.poll() is not None:
                    refuse(f"server exited early, see logs/server-{q}-rep{rep}.log", 1)
                time.sleep(1)
        for t in cfg.get("eval", "tasks"):
            cons = {}
            if t.get("grammar"):
                cons["grammar"] = cfg.p(t["grammar"]).read_text()
            if t.get("json_schema"):
                cons["json_schema"] = json.loads(cfg.p(t["json_schema"]).read_text())
            with open(rd / f"{t['name']}.jsonl", "w") as fo:
                for p in prompts[t["name"]]:
                    body = dict(prompt=p["prompt"], n_predict=int(t.get("n_predict", 64)), temperature=float(d.get("temperature", 0.0)),
                                top_k=1, top_p=1.0, min_p=0.0, repeat_penalty=1.0, seed=int(d.get("seed", 42)), cache_prompt=False,
                                stream=False, return_tokens=True, **cons)
                    if d.get("stop"):   # optional decode.stop: extra stop strings (e.g. a chat end-of-turn token)
                        body["stop"] = [str(x) for x in d["stop"]]
                    ts = time.perf_counter()
                    r = post(f"http://127.0.0.1:{port}/completion", body)
                    wall = (time.perf_counter() - ts) * 1000
                    tm = r.get("timings", {})
                    fo.write(json.dumps({"id": p["id"], "output": r.get("content", ""), "format": t.get("format", t["name"]),
                                         "input_hash": p["input_hash"], "tokens": r.get("tokens"), "n_prompt_tokens": tm.get("prompt_n"),
                                         "n_out_tokens": tm.get("predicted_n"), "prompt_ms": tm.get("prompt_ms"),
                                         "predicted_ms": tm.get("predicted_ms"), "wall_ms": round(wall, 2), "stop_type": r.get("stop_type"),
                                         "truncated": r.get("truncated")}) + "\n")
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=30)
        except subprocess.TimeoutExpired:
            proc.kill(); proc.wait()
        log.close()
    return time.time() - t0, rd


def cmd_eval(cfg: Cfg, a):
    meta = build_prompts(cfg)
    prompts = {t: [json.loads(l) for l in (cfg.out / "prompts" / f"{t}.jsonl").read_text().splitlines()] for t in meta["tasks"]}
    srv = tool(cfg, "server_bin", "~/llama.cpp/build-cuda/bin/llama-server")
    ver = subprocess.run([str(srv), "--version"], capture_output=True, text=True)
    runlog = {"schema": SCHEMA + ".eval-run", "started": now(), "server_version": (ver.stdout + ver.stderr).strip(), "runs": []}
    cap = float(cfg.get("gpu_guard", "max_minutes", default=90)) * 60
    used = 0.0
    quants = a.quant or cfg.quants
    reps = int(cfg.get("repeats", default=2))
    for rep in range(1, reps + 1):          # interleave quants within each repeat
        for q in quants:
            if used > cap:
                refuse(f"gpu-cap: {used / 60:.1f} min used of {cap / 60:.0f}", 4)
            m = cfg.out / "metrics" / f"{q}-rep{rep}.json"
            rd0 = cfg.out / "outputs" / q / f"rep{rep}"
            if m.exists() and all((rd0 / f"{t}.jsonl").exists() and len((rd0 / f"{t}.jsonl").read_text().splitlines()) == len(prompts[t])
                                  for t in prompts):
                print(f"skip {q} rep{rep}: complete outputs and metrics already present (resume)")
                runlog["runs"].append({"quant": q, "repeat": rep, "skipped": "resume", "ended": now()})
                continue
            secs, rd = run_one(cfg, q, rep, prompts)
            used += secs
            m = cfg.out / "metrics" / f"{q}-rep{rep}.json"
            sc = cfg.get("scorer", default={})
            if sc.get("command"):
                c = sc["command"].format(python=sc.get("python", sys.executable), run_dir=rd, metrics=m, eval=cfg.p(cfg.get("eval", "set")),
                                         config_dir=cfg.dir, out_dir=cfg.out, quant=q, repeat=rep)
                with open(cfg.out / "logs" / "scorer.log", "a") as f:
                    f.write(f"\n$ {c}  # {now()}\n"); f.flush()
                    r = subprocess.run(c, shell=True, stdout=f, stderr=subprocess.STDOUT)
                if r.returncode != 0 or not m.exists():
                    refuse(f"scorer failed for {q} rep{rep}, see logs/scorer.log", 1)
            runlog["runs"].append({"quant": q, "repeat": rep, "seconds": round(secs, 1), "ended": now()})
            (cfg.out / "manifests" / "eval-run.json").write_text(json.dumps(runlog, indent=1))
    gpu_check(cfg, "after eval")
    runlog["ended"] = now(); runlog["gpu_seconds"] = round(used, 1)
    (cfg.out / "manifests" / "eval-run.json").write_text(json.dumps(runlog, indent=1))


# ------------------------------------------------------------------ determinism
def load_jsonl(p: Path):
    return [json.loads(l) for l in p.read_text().splitlines() if l.strip()]


def cmd_determinism(cfg: Cfg, a):
    reps = int(cfg.get("repeats", default=2))
    res = {"schema": SCHEMA + ".determinism", "ts": now(), "repeats": reps, "quants": {}, "pass": True}
    for q in cfg.quants:
        rq = {}
        for t in cfg.get("eval", "tasks"):
            files = [cfg.out / "outputs" / q / f"rep{i}" / f"{t['name']}.jsonl" for i in range(1, reps + 1)]
            if not all(f.exists() for f in files):
                rq[t["name"]] = {"pass": False, "error": "missing repeat output"}
                continue
            runs = [load_jsonl(f) for f in files]
            base = runs[0]
            diff = sorted({b["id"] for r in runs[1:] for b, x in zip(base, r) if b["output"].encode() != x["output"].encode()})
            tdiff = sorted({b["id"] for r in runs[1:] for b, x in zip(base, r) if b.get("tokens") != x.get("tokens")})
            ok = all(len(r) == len(base) for r in runs) and not diff and not tdiff
            rq[t["name"]] = {"n": len(base), "identical_outputs": len(base) - len(diff), "output_diff_ids": diff,
                             "token_diff_ids": tdiff, "pass": ok}
        qok = all(v["pass"] for v in rq.values())
        res["quants"][q] = {"tasks": rq, "pass": qok}
        res["pass"] &= qok
    (cfg.out / "determinism.json").write_text(json.dumps(res, indent=1))
    lines = [f"# Determinism check ({reps} repeats, byte-compared outputs and token ids)", "", "| quant | task | identical | pass |", "|---|---|---|---|"]
    for q, v in res["quants"].items():
        for t, r in v["tasks"].items():
            lines.append(f"| {q} | {t} | {r.get('identical_outputs', '-')}/{r.get('n', '-')} | {'PASS' if r['pass'] else 'FAIL'} |")
    lines += ["", f"Overall: **{'PASS' if res['pass'] else 'FAIL'}**", ""]
    (cfg.out / "DETERMINISM.md").write_text("\n".join(lines))
    print("\n".join(lines))
    return 0 if res["pass"] else 5


# ------------------------------------------------------------------ compare (ladder report)
def cmd_compare(cfg: Cfg, a):
    reps = int(cfg.get("repeats", default=2))
    ref = cfg.get("reference") or cfg.quants[-1]
    cmpc = cfg.get("compare", default={})
    met = {(q, r): json.loads((cfg.out / "metrics" / f"{q}-rep{r}.json").read_text())
           for q in cfg.quants for r in range(1, reps + 1) if (cfg.out / "metrics" / f"{q}-rep{r}.json").exists()}
    rep = {"schema": SCHEMA + ".ladder", "ts": now(), "reference": ref, "quants": cfg.quants, "summary": {}, "pairs": {}, "latency": {}}
    for (q, r), m in met.items():
        rep["summary"][f"{q}/rep{r}"] = m.get("summary", {})
    for q in cfg.quants:
        for other in [x for x in cfg.quants if x != q]:
            if (q, 1) not in met or (other, 1) not in met:
                continue
            A, B = met[(other, 1)].get("items", {}), met[(q, 1)].get("items", {})
            pr = {"flips": {}, "agree": {}, "exact_output": {}}
            for f in cmpc.get("flip_fields", []):
                pr["flips"][f] = sorted(k for k in A if not A[k].get(f) and B.get(k, {}).get(f))
            for f in cmpc.get("agree_fields", []):
                ks = [k for k in A if f in A[k]]
                same = sum(1 for k in ks if A[k].get(f) == B.get(k, {}).get(f))
                pr["agree"][f] = {"n": len(ks), "agree": same, "pct": round(100 * same / len(ks), 1) if ks else None}
            for t in cfg.get("eval", "tasks"):
                xa = load_jsonl(cfg.out / "outputs" / other / "rep1" / f"{t['name']}.jsonl")
                xb = {x["id"]: x for x in load_jsonl(cfg.out / "outputs" / q / "rep1" / f"{t['name']}.jsonl")}
                pr["exact_output"][t["name"]] = {"n": len(xa), "same": sum(1 for x in xa if xb.get(x["id"], {}).get("output") == x["output"])}
            rep["pairs"][f"{q}_vs_{other}"] = pr
    for q in cfg.quants:
        for t in cfg.get("eval", "tasks"):
            xs, ws = [], []
            for r in range(1, reps + 1):
                f = cfg.out / "outputs" / q / f"rep{r}" / f"{t['name']}.jsonl"
                if f.exists():
                    for x in load_jsonl(f):
                        xs.append((x["prompt_ms"] or 0) + (x["predicted_ms"] or 0)); ws.append(x["wall_ms"])
            if xs:
                xs.sort(); ws.sort()
                rep["latency"][f"{q}/{t['name']}"] = {"n": len(xs), "p50_ms": round(statistics.median(xs), 1),
                                                      "p95_ms": round(xs[int(0.95 * (len(xs) - 1))], 1), "max_ms": round(xs[-1], 1),
                                                      "wall_p50_ms": round(statistics.median(ws), 1)}
    det = cfg.out / "determinism.json"
    if det.exists():
        rep["determinism"] = {q: v["pass"] for q, v in json.loads(det.read_text())["quants"].items()}
    (cfg.out / "ladder.json").write_text(json.dumps(rep, indent=1))
    fields = cmpc.get("summary_fields") or sorted({k for m in met.values() for k in m.get("summary", {})})
    L = [f"# Quant ladder: {cfg.base} (reference {ref})", "", "| run | " + " | ".join(fields) + " |", "|---|" + "---|" * len(fields)]
    for k, s in rep["summary"].items():
        L.append(f"| {k} | " + " | ".join(str(s.get(f, "")) for f in fields) + " |")
    L += ["", "| pair (rep1) | " + " | ".join([f"flips {f}" for f in cmpc.get("flip_fields", [])] + [f"agree {f}" for f in cmpc.get("agree_fields", [])]) + " |",
          "|---|" + "---|" * (len(cmpc.get("flip_fields", [])) + len(cmpc.get("agree_fields", [])))]
    for k, pr in rep["pairs"].items():
        L.append(f"| {k} | " + " | ".join([str(len(pr["flips"][f])) for f in cmpc.get("flip_fields", [])] +
                                           [f"{pr['agree'][f]['agree']}/{pr['agree'][f]['n']}" for f in cmpc.get("agree_fields", [])]) + " |")
    L += ["", "| quant/task | p50 ms | p95 ms | max ms |", "|---|---|---|---|"]
    for k, v in rep["latency"].items():
        L.append(f"| {k} | {v['p50_ms']} | {v['p95_ms']} | {v['max_ms']} |")
    if "determinism" in rep:
        L += ["", "Determinism: " + ", ".join(f"{q} {'PASS' if v else 'FAIL'}" for q, v in rep["determinism"].items())]
    (cfg.out / "LADDER.md").write_text("\n".join(L) + "\n")
    print("\n".join(L))


# ------------------------------------------------------------------ train / export (LLaMA-Factory wrapper)
# Exit codes 20-29 belong to train/export (0-5 are the quant-ladder steps; 6-19 are reserved for Lunatron Corpus).
EXIT_TRAIN_INVALID = 20    # recipe / path validation failed
EXIT_TRAIN_RUNNING = 21    # the cell's pid file points at a live llamafactory-cli job
EXIT_TRAIN_LAUNCH = 22     # llamafactory-cli missing, or the job died within the startup window
EXIT_TRAIN_WATCHDOG = 23   # --max-minutes exceeded; the job was stopped by its own PID
EXIT_TRAIN_FAILED = 24     # the job ended without its artifacts (non-zero exit, Traceback, missing outputs)
EXIT_TRAIN_STILL = 25      # --status: the job is still running
EXIT_TRAIN_NONE = 26       # --status / --wait: no pid file for this job
WRAP_SCHEMAS = ("cell-one.train-step.v0", SCHEMA)
DENY_CELLS = {"decisions", "classify-base-cache"}   # shared state / base-model cache: never a train or export target
LF_STEPS = {
    "train": {"pid": "train.pid", "log": "train.log", "record": "train-step.json"},
    "export": {"pid": "export.pid", "log": "export.log", "record": "export-step.json"},
}


def factory_root() -> Path:
    """Repo root. LLaMA-Factory recipe paths are relative to it (the entry is `cd ~/Projects/agent-estate-factory`)."""
    return Path(os.environ.get("FACTORY_ROOT") or Path(__file__).resolve().parents[2]).resolve()


def _rel(root: Path, v) -> Path:
    v = Path(os.path.expanduser(str(v)))
    return (v if v.is_absolute() else root / v).resolve()


def _cell_of(root: Path, p: Path):
    """Return .cell/<cell>/ when p is strictly inside it, else None."""
    try:
        parts = p.resolve().relative_to(root / ".cell").parts
    except ValueError:
        return None
    return (root / ".cell" / parts[0]) if len(parts) >= 2 else None


def _gpu_probe(lim: int, samples: int):
    """Same rule as gpu_check (compute apps or util over lim = busy), without needing a quant-ladder Cfg. Read-only."""
    q = lambda *a: subprocess.run(["nvidia-smi", *a], capture_output=True, text=True).stdout.strip()  # noqa: E731
    apps, util = "", 0
    for _ in range(max(1, samples)):
        apps = q("--query-compute-apps=pid,process_name,used_memory", "--format=csv,noheader")
        util = int((q("--query-gpu=utilization.gpu", "--format=csv,noheader,nounits").splitlines() or ["0"])[0])
        if apps or util <= lim:
            break
        time.sleep(3)
    return (not apps and util <= lim), f"{now()} util={util}% compute_apps=[{apps}]"


def _pid_alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    st = Path(f"/proc/{pid}/stat")
    return not (st.exists() and st.read_text().split(") ")[-1].startswith("Z"))


def _pid_is_job(pid: int, recipe: Path) -> bool:
    """Guard against PID reuse: the process must be llamafactory-cli on this recipe."""
    try:
        cmd = Path(f"/proc/{pid}/cmdline").read_bytes().replace(b"\0", b" ").decode(errors="replace")
    except OSError:
        return False
    return "llamafactory-cli" in cmd and (recipe.name in cmd)


def _load_lf(step: str, config: str, root: Path) -> dict:
    """Load a LLaMA-Factory yaml directly, or a cell-one wrapper config that points at one."""
    path = Path(config).resolve()
    if not path.is_file():
        refuse(f"{step}: config not found: {config}", EXIT_TRAIN_INVALID)
    raw = yaml.safe_load(path.read_text()) or {}
    wrap = {}
    if "schema" in raw:
        key = "recipe" if step == "train" else "config"
        if raw["schema"] not in WRAP_SCHEMAS or not raw.get(step, {}).get(key):
            refuse(f"{step}: {path} has schema {raw['schema']!r}; need a LLaMA-Factory yaml or one of {WRAP_SCHEMAS} "
                   f"with {step}.{key}", EXIT_TRAIN_INVALID)
        wrap = raw
        p = Path(os.path.expanduser(str(raw[step][key])))
        path = (p if p.is_absolute() else path.parent / p).resolve()
        if not path.is_file():
            refuse(f"{step}: {step}.{key} not found: {path}", EXIT_TRAIN_INVALID)
        raw = yaml.safe_load(path.read_text()) or {}
    g = wrap.get("gpu_guard", {})
    s = wrap.get(step, {})
    return {"lf_path": path, "lf": raw, "wrap": wrap, "max_util_pct": int(g.get("max_util_pct", 30)),
            "settle_samples": int(g.get("settle_samples", 10)), "max_minutes": s.get("max_minutes"),
            "cli": str(_rel(root, s.get("cli") or wrap.get("toolchain", {}).get("llamafactory_cli")
                            or "~/venvs/llamafactory/bin/llamafactory-cli"))}


def _validate_model_dir(root: Path, v, what: str, errs: list, info: dict):
    if not v:
        errs.append(f"{what}: missing")
        return None
    d = _rel(root, v)
    if not (d / "config.json").is_file():
        errs.append(f"{what}: {v} is not a local model directory with config.json (these steps never download)")
        return None
    w = sorted(x.name for x in d.glob("*.safetensors")) or sorted(x.name for x in d.glob("*.bin"))
    if not w:
        errs.append(f"{what}: no *.safetensors / *.bin weights in {d}")
    info[what] = {"path": str(d), "weights": len(w)}
    return d


def _validate_train(root: Path, lf: dict, errs: list, warns: list, info: dict):
    if lf.get("stage") not in ("sft", "pt", "rm", "ppo", "dpo", "kto"):
        errs.append(f"stage: {lf.get('stage')!r} is not a LLaMA-Factory training stage")
    if not lf.get("do_train"):
        errs.append("do_train: must be true")
    _validate_model_dir(root, lf.get("model_name_or_path"), "model_name_or_path", errs, info)
    for k in ("template", "finetuning_type"):
        if not lf.get(k):
            errs.append(f"{k}: missing")
    dd = _rel(root, lf.get("dataset_dir") or "data")
    di_path = dd / "dataset_info.json"
    names = {k: [n.strip() for n in str(lf[k]).split(",") if n.strip()] for k in ("dataset", "eval_dataset") if lf.get(k)}
    if not names.get("dataset"):
        errs.append("dataset: missing")
    if not di_path.is_file():
        errs.append(f"dataset_dir: no dataset_info.json in {dd}")
    else:
        try:
            di = json.loads(di_path.read_text())
        except json.JSONDecodeError as e:
            di = {}
            errs.append(f"dataset_info.json: invalid JSON ({e})")
        info["datasets"] = {}
        for k, ns in names.items():
            for n in ns:
                ent = di.get(n)
                if ent is None:
                    errs.append(f"{k}: {n!r} not in {di_path}")
                    continue
                if not ent.get("file_name"):
                    errs.append(f"{k}: {n!r} has no local file_name (hub/remote datasets are not allowed)")
                    continue
                f = dd / ent["file_name"]
                if not f.exists():
                    errs.append(f"{k}: {n!r} file missing: {f}")
                    continue
                rows = None
                if f.is_file():
                    with open(f, "rb") as fh:
                        rows = sum(1 for _ in fh)
                info["datasets"][n] = {"file": str(f), "formatting": ent.get("formatting", "alpaca"), "lines": rows,
                                       "sha256": sha256_file(f) if f.is_file() else None}
    if lf.get("eval_dataset") and float(lf.get("val_size", 0) or 0) > 0:
        errs.append("eval_dataset and val_size are mutually exclusive in LLaMA-Factory")
    if not (lf.get("eval_dataset") or float(lf.get("val_size", 0) or 0) > 0):
        warns.append("no held-out eval (neither eval_dataset nor val_size)")
    if lf.get("finetuning_type") == "lora" and lf.get("adapter_name_or_path"):
        warns.append("adapter_name_or_path set: training continues from an existing adapter")


def _validate_export(root: Path, lf: dict, errs: list, warns: list, info: dict):
    _validate_model_dir(root, lf.get("model_name_or_path"), "model_name_or_path", errs, info)
    if not lf.get("template"):
        errs.append("template: missing")
    a = lf.get("adapter_name_or_path")
    if lf.get("finetuning_type", "lora") == "lora":
        if not a:
            errs.append("adapter_name_or_path: missing (LoRA export merges an adapter)")
        else:
            ad = _rel(root, a)
            if not (ad / "adapter_config.json").is_file() or not list(ad.glob("adapter_model.*")):
                errs.append(f"adapter_name_or_path: no adapter_config.json + adapter_model.* in {ad} (train not finished?)")
            info["adapter"] = str(ad)
    if str(lf.get("export_device", "cpu")) != "cpu":
        warns.append(f"export_device={lf.get('export_device')}: merge runs on the GPU")
    if lf.get("export_quantization_bit"):
        warns.append("export_quantization_bit set: GPTQ export needs the GPU and a calibration set")


def _plan(step: str, a, root: Path):
    c = _load_lf(step, a.config, root)
    lf, errs, warns, info = c["lf"], [], [], {}
    out_key = "output_dir" if step == "train" else "export_dir"
    out = _rel(root, lf[out_key]) if lf.get(out_key) else None
    cell = _cell_of(root, out) if out else None
    if out is None:
        errs.append(f"{out_key}: missing")
    elif cell is None:
        errs.append(f"{out_key}: {lf[out_key]} must be inside .cell/<cell>/ of {root}")
    elif cell.name in DENY_CELLS:
        errs.append(f"{out_key}: .cell/{cell.name}/ is not a writable cell")
    elif out.exists() and any(out.iterdir()):
        if step == "train" and not lf.get("overwrite_output_dir"):
            errs.append(f"output_dir: {out} is not empty and overwrite_output_dir is not true")
        elif step == "train":
            warns.append(f"output_dir: {out} is not empty; overwrite_output_dir=true will replace it")
        else:
            errs.append(f"export_dir: {out} is not empty (export never overwrites)")
    (_validate_train if step == "train" else _validate_export)(root, lf, errs, warns, info)
    if not os.access(c["cli"], os.X_OK):
        errs.append(f"llamafactory-cli not executable: {c['cli']}")
    names = LF_STEPS[step]
    # run dir = parent of output_dir / export_dir (.cell/<cell>/outputs -> .cell/<cell>; nested runs such as
    # .cell/fm-research-bot/slm-20261001/outputs -> .cell/fm-research-bot/slm-20261001), matching the existing cells
    run = out.parent if cell else None
    pidf = run / names["pid"] if run else None
    logf = run / "logs" / names["log"] if run else None
    _venv_toolchain(step, root, c, errs, info)   # optional <step>.venv; leaves c["cli"] unchanged without it
    cmd = ["setsid", "nohup", c["cli"], step, str(c["lf_path"])]
    _lf_launcher(step, a, root, c, cmd, errs, warns, info)   # optional; leaves cmd unchanged without the keys
    mm = a.max_minutes if a.max_minutes is not None else c["max_minutes"]
    return dict(c=c, lf=lf, errs=errs, warns=warns, info=info, out=out, cell=cell, run=run, pidf=pidf, logf=logf, cmd=cmd,
                record=(run / names["record"]) if run else None, max_minutes=float(mm) if mm else None)


# ------------------------------------------------------------------ recipe-supplied python for train/export (additive, opt-in)
LF_LAUNCHER = Path(__file__).resolve().parent / "lf_launch.py"


def _wrap_path(a, v) -> Path:
    """Wrapper-relative path (like train.recipe / export.config)."""
    p = Path(os.path.expanduser(str(v)))
    return (p if p.is_absolute() else Path(a.config).resolve().parent / p).resolve()


def _venv_toolchain(step: str, root: Path, c: dict, errs: list, info: dict):
    """Wrapper key `<step>.venv` (fallback `toolchain.llamafactory_venv`): run this step from another venv, e.g. an
    additive sidecar with extra kernels. cli = <venv>/bin/llamafactory-cli and the launcher python = <venv>/bin/python
    (paths are not symlink-resolved, so the venv stays active). Root-relative like toolchain.llamafactory_cli.
    Conflicts with an explicit `<step>.cli`. Without the key nothing changes."""
    w, s = c["wrap"], (c["wrap"].get(step) or {})
    v = s.get("venv") or w.get("toolchain", {}).get("llamafactory_venv")
    if not v:
        return
    if s.get("cli"):
        errs.append(f"{step}.venv and {step}.cli are both set; set one")
        return
    d = Path(os.path.expanduser(str(v)))
    d = d if d.is_absolute() else root / d
    errs[:] = [e for e in errs if not e.startswith("llamafactory-cli not executable:")]   # the default cli no longer applies
    cli, py = d / "bin" / "llamafactory-cli", d / "bin" / "python"
    if not (d / "pyvenv.cfg").is_file():
        errs.append(f"{step}.venv: {d} is not a venv (no pyvenv.cfg)")
    for x in (cli, py):
        if not os.access(x, os.X_OK):
            errs.append(f"{step}.venv: not executable: {x}")
    c["cli"], c["venv_python"] = str(cli), str(py)
    info["venv"] = {"path": str(d), "cli": str(cli), "python": str(py)}


def _lf_launcher(step: str, a, root: Path, c: dict, cmd: list, errs: list, warns: list, info: dict):
    """Optional wrapper keys (relative to the wrapper file; top-level fallback for custom_templates / preload):
      <step>.custom_templates  python file that calls llamafactory.data.template.register_template(...)
      <step>.preload           list of python files imported first (runtime patches; e.g. a model capability flag)
      export.json_patches      {<file>.json: {set: {...}, drop: [...]}} applied in <export_dir> after a successful export
      export.generation_config shorthand for json_patches {generation_config.json: {set: <mapping>}}
    When any is set, the job runs as `<python> lf_launch.py [--preload F]... [--custom-templates F]
    [--json-patches JSON] --cli <llamafactory-cli> <step> <yaml>`, which imports the files and then runs the
    llamafactory-cli entry in-process, so a recipe can bring its own template / patches without editing the venv.
    Python: `<step>.python`, else the `<step>.venv` python, else `toolchain.llamafactory_python`, else the `python`
    next to the cli (not symlink-resolved). These keys live in the wrapper, never in the LLaMA-Factory yaml (LF refuses
    unknown keys). Without them, cmd is left exactly as before."""
    import re
    w, s = c["wrap"], (c["wrap"].get(step) or {})
    v = s.get("custom_templates") or w.get("custom_templates")
    pre = s.get("preload") if s.get("preload") is not None else w.get("preload")
    pre = [pre] if isinstance(pre, str) else list(pre or [])
    gen = s.get("generation_config") if step == "export" else None
    jp = s.get("json_patches") if step == "export" else None
    for k in ("generation_config", "json_patches"):
        if step == "train" and s.get(k):
            errs.append(f"train.{k}: only export.{k} is supported")
    if not (v or pre or gen or jp):
        return
    extra = []
    for f in pre:
        p = _wrap_path(a, f)
        if p.suffix != ".py" or not p.is_file():
            errs.append(f"preload: {p} must be an existing .py file")
            continue
        info.setdefault("preload", []).append({"path": str(p), "sha256": sha256_file(p)})
        extra += ["--preload", str(p)]
    patches = {}
    if jp is not None:
        ok = isinstance(jp, dict) and jp and all(
            isinstance(k, str) and k.endswith(".json") and "/" not in k and isinstance(e, dict) and not set(e) - {"set", "drop"}
            and isinstance(e.get("set", {}), dict) and isinstance(e.get("drop", []), list) for k, e in jp.items())
        if not ok:
            errs.append("export.json_patches: must map <file>.json (no path) to {set: {...}, drop: [...]}")
        else:
            patches = {k: dict(e) for k, e in jp.items()}
    if gen is not None:
        if not isinstance(gen, dict) or not gen:
            errs.append("export.generation_config: must be a non-empty mapping, e.g. {eos_token_id: [2, 11]}")
        else:
            info["generation_config"] = gen
            e = patches.setdefault("generation_config.json", {})
            e["set"] = {**e.get("set", {}), **gen}
    if patches:
        info["json_patches"] = patches
        extra += ["--json-patches", json.dumps(patches, separators=(",", ":"))]
    tpl = c["lf"].get("template")
    if v:
        p = _wrap_path(a, v)
        if p.suffix != ".py":
            errs.append(f"custom_templates: {p} must be a .py file that calls register_template")
            return
        if not p.is_file():
            errs.append(f"custom_templates: not found: {p}")
            return
        text = p.read_text(errors="replace")
        if "register_template" not in text:
            warns.append(f"custom_templates: {p.name} never calls register_template")
        if tpl and not re.search(r"""["']%s["']""" % re.escape(str(tpl)), text):
            warns.append(f"custom_templates: template {tpl!r} is not a string literal in {p.name}; lf_launch.py refuses "
                         f"(exit 2) if it is still unregistered after the import")
        info["custom_templates"] = {"path": str(p), "sha256": sha256_file(p), "template": tpl}
        extra += ["--custom-templates", str(p)]
    py = s.get("python") or c.get("venv_python") or w.get("toolchain", {}).get("llamafactory_python")
    py = Path(os.path.expanduser(str(py))) if py else Path(c["cli"]).parent / "python"
    py = py if py.is_absolute() else root / py
    if not os.access(py, os.X_OK):
        errs.append(f"lf_launch: python not executable: {py} (set {step}.python, {step}.venv or toolchain.llamafactory_python)")
    if not LF_LAUNCHER.is_file():
        errs.append(f"lf_launch: launcher missing: {LF_LAUNCHER}")
    info["launcher"] = {"path": str(LF_LAUNCHER), "python": str(py)}
    if "custom_templates" in info:
        info["custom_templates"].update(python=str(py), launcher=str(LF_LAUNCHER))
    cmd[:] = ["setsid", "nohup", str(py), str(LF_LAUNCHER), *extra, "--cli", c["cli"], step, str(c["lf_path"])]


def _read_pid(pidf: Path):
    try:
        return int(pidf.read_text().split()[0])
    except (OSError, ValueError, IndexError):
        return None


def _done_ok(step: str, out: Path) -> bool:
    if step == "train":
        results = any((out / f).is_file() for f in ("train_results.json", "all_results.json"))
        return results and (any(out.glob("adapter_model.*")) or any(out.glob("model*.safetensors")))
    return (out / "config.json").is_file() and any(out.glob("*.safetensors"))


def _log_has_traceback(logf: Path) -> bool:
    try:
        with open(logf, "rb") as f:
            f.seek(max(0, logf.stat().st_size - 200_000))
            return b"Traceback (most recent call last)" in f.read()
    except OSError:
        return False


def _update_record(pl: dict, **kv):
    r = pl["record"]
    try:
        rec = json.loads(r.read_text()) if r.exists() else {}
    except json.JSONDecodeError:
        rec = {}
    rec.update(kv)
    r.write_text(json.dumps(rec, indent=1) + "\n")


def _stop_pid(pid: int, recipe: Path):
    """Stop the job by its own PID only (SIGTERM, then SIGKILL after 30 s). Never pkill, never by name."""
    if not _pid_is_job(pid, recipe):
        return "not-ours"
    os.kill(pid, signal.SIGTERM)
    for _ in range(30):
        if not _pid_alive(pid):
            return "terminated"
        time.sleep(1)
    os.kill(pid, signal.SIGKILL)
    return "killed"


def _watch(step: str, pl: dict, pid: int, proc, started: float, poll: float) -> int:
    """Block until the job ends; enforce --max-minutes by PID. Returns an exit code."""
    recipe = pl["c"]["lf_path"]
    cap = pl["max_minutes"] * 60 if pl["max_minutes"] else None
    while True:
        alive = (proc.poll() is None) if proc is not None else _pid_alive(pid)
        if not alive:
            break
        if cap and time.time() - started > cap:
            how = _stop_pid(pid, recipe)
            msg = f"watchdog: {step} pid {pid} over {pl['max_minutes']} min, {how}"
            print(f"refuse:{msg}", file=sys.stderr)
            _update_record(pl, ended=now(), result="watchdog", watchdog=how)
            return EXIT_TRAIN_WATCHDOG
        time.sleep(poll)
    rc = proc.returncode if proc is not None else None
    ok = _done_ok(step, pl["out"]) and rc in (0, None) and not _log_has_traceback(pl["logf"])
    _update_record(pl, ended=now(), result="ok" if ok else "failed", returncode=rc)
    print(f"{step}: pid {pid} ended rc={rc} result={'ok' if ok else 'failed'} (log {pl['logf']})")
    return 0 if ok else EXIT_TRAIN_FAILED


def _status(step: str, pl: dict, a) -> int:
    if not pl["pidf"] or not pl["pidf"].exists():
        print(f"{step}: no pid file ({pl['pidf']})")
        return EXIT_TRAIN_NONE
    pid = _read_pid(pl["pidf"])
    recipe = pl["c"]["lf_path"]
    if pid and _pid_alive(pid) and _pid_is_job(pid, recipe):
        print(f"{step}: running pid {pid} (log {pl['logf']})")
        if not a.wait:
            return EXIT_TRAIN_STILL
        try:
            st = Path(f"/proc/{pid}").stat().st_mtime
        except OSError:
            st = time.time()
        rec = json.loads(pl["record"].read_text()) if pl["record"] and pl["record"].exists() else {}
        started = rec.get("started_epoch", st)
        return _watch(step, pl, pid, None, started, a.poll_seconds)
    ok = _done_ok(step, pl["out"]) and not _log_has_traceback(pl["logf"])
    print(f"{step}: pid {pid} not running; result={'ok' if ok else 'failed'} (log {pl['logf']})")
    return 0 if ok else EXIT_TRAIN_FAILED


def _cmd_lf(step: str, cfg, a) -> int:
    root = factory_root()
    pl = _plan(step, a, root)
    if a.status:
        if pl["cell"] is None:
            refuse(f"{step}: cannot locate the cell: {pl['errs']}", EXIT_TRAIN_INVALID)
        return _status(step, pl, a)
    if pl["pidf"] and pl["pidf"].exists():
        pid = _read_pid(pl["pidf"])
        if pid and _pid_alive(pid) and _pid_is_job(pid, pl["c"]["lf_path"]):
            pl["errs"].append(f"already running: pid {pid} from {pl['pidf']}")
            running = True
        else:
            running = False
            pl["warns"].append(f"stale pid file {pl['pidf']} (pid {pid} not a live llamafactory-cli job); it will be replaced")
    else:
        running = False
    gpu_ok, gpu_line = _gpu_probe(pl["c"]["max_util_pct"], pl["c"]["settle_samples"] if not a.dry_run else 1)
    report = {"schema": f"cell-one.{step}-step.v0.plan", "ts": now(), "root": str(root), "config": str(Path(a.config).resolve()),
              "llamafactory_yaml": str(pl["c"]["lf_path"]), "llamafactory_yaml_sha256": sha256_file(pl["c"]["lf_path"]),
              "cell": str(pl["cell"]) if pl["cell"] else None, "run_dir": str(pl["run"]) if pl["run"] else None, ("output_dir" if step == "train" else "export_dir"): str(pl["out"]),
              "pid_file": str(pl["pidf"]), "log": str(pl["logf"]), "max_minutes": pl["max_minutes"],
              "gpu_guard": {"ok": gpu_ok, "probe": gpu_line, "max_util_pct": pl["c"]["max_util_pct"]},
              "checks": pl["info"], "warnings": pl["warns"], "errors": pl["errs"],
              "command": f"cd {shlex.quote(str(root))} && {shlex.join(pl['cmd'])} > {shlex.quote(str(pl['logf']))} 2>&1 & "
                         f"echo $! > {shlex.quote(str(pl['pidf']))}"}
    print(json.dumps(report, indent=1))
    if pl["errs"]:
        print(f"refuse:{step}: {'; '.join(pl['errs'])}", file=sys.stderr)
        return EXIT_TRAIN_RUNNING if running and len(pl["errs"]) == 1 else EXIT_TRAIN_INVALID
    if not gpu_ok:
        print(f"refuse:gpu-busy: {gpu_line}", file=sys.stderr)
        return 3
    if a.dry_run:
        print(f"DRY-RUN {step}: valid; not launched.")
        return 0
    pl["logf"].parent.mkdir(parents=True, exist_ok=True)
    with open(pl["logf"].parent / "gpu-checks.log", "a") as f:
        f.write(f"{gpu_line} before {step}\n")
    if pl["logf"].exists() and pl["logf"].stat().st_size:
        pl["logf"].rename(pl["logf"].with_name(f"{pl['logf'].name}.prev-{datetime.datetime.now():%Y%m%dT%H%M%S}"))
    logh = open(pl["logf"], "w")
    # setsid execs in place (the child is not a group leader), so $! / proc.pid is the llamafactory-cli PID
    proc = subprocess.Popen(pl["cmd"], cwd=root, stdin=subprocess.DEVNULL, stdout=logh, stderr=subprocess.STDOUT)
    started = time.time()
    pl["pidf"].write_text(f"{proc.pid}\n")
    _update_record(pl, **{k: report[k] for k in ("llamafactory_yaml", "llamafactory_yaml_sha256", "command",
                                                 "gpu_guard", "max_minutes")},
                   schema=f"cell-one.{step}-step.v0", pid=proc.pid, started=now(), started_epoch=started, result="running")
    time.sleep(a.startup_seconds)
    if proc.poll() is not None and proc.returncode != 0:
        _update_record(pl, ended=now(), result="launch-failed", returncode=proc.returncode)
        print(f"refuse:{step}: exited rc={proc.returncode} within {a.startup_seconds}s, see {pl['logf']}", file=sys.stderr)
        return EXIT_TRAIN_LAUNCH
    print(f"{step}: launched pid {proc.pid} (pid file {pl['pidf']}, log {pl['logf']})")
    if not a.wait:
        return 0
    return _watch(step, pl, proc.pid, proc, started, a.poll_seconds)


def cmd_train(cfg, a):
    return _cmd_lf("train", cfg, a)


def cmd_export(cfg, a):
    return _cmd_lf("export", cfg, a)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("step", choices=["build", "eval", "determinism", "compare", "train", "export"])
    ap.add_argument("--config", required=True)
    ap.add_argument("--quant", action="append", help="eval: limit to these quants (repeatable)")
    ap.add_argument("--dry-run", action="store_true", help="train/export: validate, probe the GPU guard, print the command; no launch")
    ap.add_argument("--status", action="store_true", help="train/export: report the job from the cell pid file (0 ok, 24 failed, 25 running, 26 none)")
    ap.add_argument("--wait", action="store_true", help="train/export: block until the job ends (after launch, or with --status)")
    ap.add_argument("--max-minutes", type=float, default=None, help="train/export with --wait: stop the job by its PID after N minutes")
    ap.add_argument("--poll-seconds", type=float, default=30.0, help="train/export: poll interval for --wait")
    ap.add_argument("--startup-seconds", type=float, default=10.0, help="train/export: early-exit window after launch")
    a = ap.parse_args(argv)
    if a.step in ("train", "export"):   # LLaMA-Factory yaml or a cell-one wrapper; not a quant-ladder Cfg
        return {"train": cmd_train, "export": cmd_export}[a.step](None, a) or 0
    cfg = Cfg(a.config)
    return {"build": cmd_build, "eval": cmd_eval, "determinism": cmd_determinism, "compare": cmd_compare}[a.step](cfg, a) or 0


if __name__ == "__main__":
    sys.exit(main())
