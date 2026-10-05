"""One-shot job runner. No loop, no trainer launch, no GPU.

The train path calls the gap report and the decontaminate gate here, in this
file, before it writes a plan. It does not edit train-step.sh. It executes
that script only when the config sets allow_launch, the approve file says yes,
and the GPU probe has reported idle.
"""

from __future__ import annotations

import json
import re
import time
from pathlib import Path

from . import (
    base_pick,
    decontaminate,
    device_pack,
    eval_serve,
    exam_keyfact,
    exam_score,
    failures,
    gap,
    improve_data,
    judge,
    launch_lock,
    local_first,
    pinned_base,
    reading,
    registry,
    route,
)
from .common import (
    EXIT_BASE,
    EXIT_GPU_BUSY,
    EXIT_GPU_CAP,
    EXIT_IMPROVE,
    EXIT_JOB,
    EXIT_JUDGE,
    EXIT_LAUNCH,
    GPU_REQUIRED,
    TERMINAL,
    Inbox,
    file_bytes,
    guarded,
    now_iso,
    one_sentence,
    read_json,
    write_json,
)
from .digest import paused, rewrite as rewrite_quota
from .gpu_probe import classify_own_train
from .job import (
    gpu_notice,
    make_card,
    make_status,
    publish,
    read_approve,
    submit_job,
    validate_job,
)

PLAN_ONLY = {
    "launched": False,
    "rc": None,
    "server_started": False,
    "gpu_minutes": 0,
}

_TRAIN_RESULT_RE = re.compile(r"\bresult=(ok|failed|watchdog|launch-failed)\b")
_TRAIN_PID_RE = re.compile(r"\bpid (\d+)\b")


def format_minutes(minutes) -> str:
    try:
        number = float(minutes)
    except (TypeError, ValueError):
        return str(minutes)
    if number.is_integer():
        return str(int(number))
    return str(number)


def run_train_command(recipe: str, minutes) -> str:
    """The run step waits. A detached launch is not a pass."""
    if not recipe:
        return "scripts/train-step.sh <missing recipe>"
    return (
        f"scripts/train-step.sh {recipe} --wait --max-minutes {format_minutes(minutes)}"
    )


def parse_train_result(rc, output="", result=None) -> dict:
    """rc=0 and result=ok is the only train pass. Anything else is a failed gate."""
    text = output or ""
    if result is None:
        found = _TRAIN_RESULT_RE.search(text)
        result = found.group(1) if found else None
    pid_found = _TRAIN_PID_RE.search(text)
    pid = int(pid_found.group(1)) if pid_found else None
    try:
        rc_n = int(rc) if rc is not None else None
    except (TypeError, ValueError):
        rc_n = None
    return {
        "pass": rc_n == 0 and result == "ok",
        "rc": rc_n,
        "result": result,
        "pid": pid,
    }


def can_rerun(existing: dict | None) -> bool:
    """A refused job, or a failed job that crashed, can be run again by id."""
    if not existing:
        return False
    state = existing.get("state")
    if state == "refused":
        return True
    return state == "failed" and bool(existing.get("crash"))


def crash_reason(step: str, exc: BaseException) -> str:
    detail = " ".join(str(exc).split()) or "no message"
    return f"Step {step} crashed ({exc.__class__.__name__}): {detail}"


def _exam_for_gap(cfg: dict):
    """Prefer the file that has question rows. A text-only view is not the exam shape."""
    candidates = [cfg.get("source_exam"), cfg.get("exam")]
    opened = None
    for raw in candidates:
        if not raw:
            continue
        path = Path(raw)
        opened = path
        if not path.is_file():
            continue
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            question = row.get("question") if isinstance(row, dict) else None
            if isinstance(question, str) and question.strip():
                return path
    return opened


class Harness:
    def __init__(self, inbox, cell_root, gpu_probe=None, trainer=None, seat=None, caller="local", exam_ask=None, launch_runner=None, server=None, judge_ask=None, clock=None, sleep=None):
        self.inbox = Inbox(Path(inbox))
        self.cell_root = Path(cell_root)
        self.gpu_probe = gpu_probe or (lambda: {"state": "unknown", "holder": None, "seen": False})
        self.trainer = trainer
        self.seat = seat
        self.caller = caller
        self.exam_ask = exam_ask
        self.launch_runner = launch_runner
        self.server = server
        self.judge_ask = judge_ask
        self.clock = clock or time.monotonic
        self.sleep = sleep or time.sleep
        self.probed_gpu = "unknown"
        self.probe_source = None
        self.train_pid = None
        self.train_pid_note = ""
        self._force_page = False
        self._active_step = ""
        self._serve_handle = None
        self._serve_log = ""

    def cell_for(self, job_id: str) -> Path:
        return self.cell_root / f"harness-job-{job_id}"

    def submit(self, raw: dict) -> dict:
        return submit_job(self.inbox, raw, self.cell_root)

    def status(self, job_id: str, via: str = "local") -> dict:
        path = self.inbox.status_path(job_id)
        if via == "cloud":
            from .digest import note_cloud_status
            note_cloud_status(self.inbox)
            got = local_first.cloud_read(path, kind="status")
            if not got["ok"]:
                return {"ok": False, "reason": got["reason"], "exit_code": got["exit_code"]}
        if not path.is_file():
            return {
                "ok": False,
                "reason": one_sentence(f"No status file for {job_id}."),
                "exit_code": EXIT_JOB,
            }
        return {"ok": True, "status": read_json(path), "exit_code": 0}

    def digest(self) -> dict:
        return rewrite_quota(self.inbox)

    def run(self, job_id: str) -> dict:
        self.probed_gpu = "unknown"
        self.probe_source = None
        self.train_pid = None
        self.train_pid_note = ""
        self._force_page = False
        self._active_step = ""
        self._serve_handle = None
        self._serve_log = ""
        path = self.inbox.job_path(job_id)
        if not path.is_file():
            return self._bare(job_id, "refused", "refused", f"No job file for {job_id}.", False, EXIT_JOB)
        loaded = read_json(path)
        job, bad = validate_job(loaded)
        if job is None:
            shell = dict(loaded)
            shell.setdefault("id", job_id)
            shell.setdefault("step", "gap")
            shell.setdefault("nas_out", str(self.cell_for(job_id)))
            shell.setdefault("config", loaded.get("config") or str(path))
            shell.setdefault("gpu", "forbid")
            shell.setdefault("max_minutes", 0)
            return self._finish(
                shell, state="refused", result="refused",
                reason=bad or "The job file is not valid.",
                needs_you=False, exit_code=EXIT_JOB,
                tried="Read the job file.",
                why=bad or "The job file is not valid.",
                nxt="Fix the job file. This runner did not start it.",
                pid_alive="not-a-gpu-job", gpu="unknown",
            )
        if self.caller == "cloud":
            return self._finish(
                job, state="refused", result="refused",
                reason="A cloud session does not run a job. Read the status file. The config was not opened.",
                needs_you=False, exit_code=36,
                tried="Refused a cloud caller before opening the config.",
                why="Allowed cloud reads are the status file, the card, and the last 30 error lines.",
                nxt="Run the job on the box. Do not open the corpus from a cloud session.",
                unverified="This cloud call did not open the config, so the step was not checked.",
                pid_alive="not-a-gpu-job", gpu="unknown",
            )
        existing = self._existing(job_id)
        if existing and existing.get("state") in TERMINAL and not can_rerun(existing):
            return {
                "ok": existing.get("state") == "succeeded",
                "state": existing.get("state"),
                "reason": existing.get("reason"),
                "exit_code": 0,
                "id": job_id,
                "replayed": True,
            }
        if can_rerun(existing):
            self._force_page = True
        if paused(self.inbox):
            note = ""
            if self.inbox.pause_note.is_file():
                note = self.inbox.pause_note.read_text(encoding="utf-8").strip()
            reason = note or "New jobs wait. A person has not cleared the quota pause."
            return self._finish(
                job,
                state="needs-approval",
                result="needs-approval",
                reason=reason,
                needs_you=True,
                exit_code=0,
                tried="Checked the local quota file before starting.",
                why=reason,
                nxt="A person clears the quota pause. This runner does not page anyone.",
                pid_alive="not-a-gpu-job",
                gpu="unknown",
            )
        blocked = local_first.assert_local_config(Path(job["config"]), self.caller)
        if blocked:
            return self._finish(
                job,
                state="refused",
                result="refused",
                reason=blocked,
                needs_you=False,
                exit_code=36,
                tried="A cloud caller asked to open the config.",
                why=blocked,
                nxt="Read the status file from the box. Do not open the config from a cloud session.",
                unverified=blocked,
                pid_alive="not-a-gpu-job",
                gpu="unknown",
            )
        gate = self._approval_gate(job)
        if gate is not None:
            return gate
        handler = {
            "gap": self._gap,
            "decontaminate": self._decontaminate,
            "train": self._train,
            "export": self._export,
            "eval": self._eval,
            "quant": self._quant,
            "ask": self._ask,
            "pack-device": self._pack,
            "run": self._run_chain,
            "improve": self._improve,
            "improve-apply": self._improve_apply,
        }.get(job["step"])
        if handler is None:
            return self._finish(
                job, state="refused", result="refused",
                reason=f"Step {job['step']} is not in the contract list.",
                needs_you=False, exit_code=EXIT_JOB,
                tried="Read the job file.", why="The step is not one of the listed contract steps.",
                nxt="Submit a new job with a listed step.",
                pid_alive="not-a-gpu-job", gpu="unknown",
            )
        self._active_step = job["step"]
        self._serve_handle = None
        try:
            return handler(job)
        except eval_serve.CliMissing as exc:
            self._stop_any_server()
            reason = str(exc)
            return self._finish(
                job, state="refused", result="refused", reason=reason,
                needs_you=True, exit_code=EXIT_JOB,
                tried="Looked up llamafactory-cli before starting. Did not start it.",
                why=reason,
                nxt="Set llamafactory_cli or LLAMAFACTORY_CLI to an executable, or place it beside QUANT_LADDER_PYTHON.",
                pid_alive="no", gpu=self.probed_gpu,
            )
        except eval_serve.PortBusy as exc:
            self._stop_any_server()
            reason = str(exc)
            return self._finish(
                job, state="refused", result="refused", reason=reason,
                needs_you=True, exit_code=EXIT_JOB,
                tried="Checked the port before starting the server. Did not start it.",
                why=reason,
                nxt="Free the port, then run this job again. A DONE file is not a score.",
                pid_alive="no", gpu=self.probed_gpu,
                quote=self._serve_quote() or None,
            )
        except Exception as exc:
            self._stop_any_server()
            step = self._active_step or job["step"]
            reason = crash_reason(step, exc)
            return self._finish(
                job, state="failed", result="fail", reason=reason,
                needs_you=True, exit_code=EXIT_JOB, crash=True, step_name=step,
                tried=f"Step {step} raised {exc.__class__.__name__}. The runner stopped the server if one was up.",
                why=reason,
                nxt="This crash can be run again by id. A gate failure cannot. Nothing was seated.",
                pid_alive="no", gpu=self.probed_gpu,
                quote=self._serve_quote() or None,
            )

    def _approval_gate(self, job: dict):
        kind = None
        if job["step"] == "pack-device":
            kind = "pack"
        elif job["step"] == "improve-apply":
            kind = "apply"
        elif job["gpu"] == "ask":
            kind = "gpu"
        if kind is None:
            return None
        approve, err = read_approve(self.inbox, job["id"])
        if kind == "gpu":
            notice = gpu_notice(self.gpu_probe(), job["max_minutes"])
            self.probed_gpu = notice["gpu"] if notice.get("seen") else "unknown"
            self.probe_source = notice.get("source")
            if approve is None or err:
                reason = err or notice["reason"]
                if approve is None and not err:
                    reason = notice["reason"] + " Waiting for an approve file. A missing file is not a yes."
                return self._finish(
                    job, state="needs-approval", result="needs-approval",
                    reason=one_sentence(reason), needs_you=True, exit_code=0,
                    tried="Wrote a GPU notice before starting.",
                    why=notice["reason"],
                    nxt="Grok Bot asks Jason, then writes the approve file. This runner does not start the GPU.",
                    pid_alive="no", gpu=notice["gpu"],
                )
            if approve["decision"] != "yes":
                return self._finish(
                    job, state="refused", result="refused",
                    reason="A no was written on the approve file. The job file was kept.",
                    needs_you=False, exit_code=EXIT_JOB,
                    tried="Read the approve file before starting.",
                    why="The decision was no. The job was not deleted.",
                    nxt="Fix the request and submit a new job if it should run.",
                    pid_alive="no", gpu=notice["gpu"],
                )
            if notice["blocked"]:
                code = notice["exit_code"] or EXIT_GPU_CAP
                return self._finish(
                    job, state="refused", result="refused",
                    reason=notice["reason"], needs_you=notice["gpu"] == "busy" or not notice["seen"],
                    exit_code=code,
                    tried="Read the approve file and the GPU notice.",
                    why=notice["reason"],
                    nxt="An approve file does not override a busy card or an unseen guard.",
                    pid_alive="no", gpu=notice["gpu"],
                )
            return None
        if approve is None or err or approve.get("decision") != "yes":
            applying = kind == "apply"
            if approve and approve.get("decision") == "no":
                reason = (
                    "A no was written on the approve file. The training set was not edited."
                    if applying
                    else "A no was written on the approve file. The device folder was not written."
                )
                tried = (
                    "Read the approve file before apply. Did not edit the training set."
                    if applying
                    else "Read the approve file before copying."
                )
                return self._finish(
                    job, state="refused", result="refused",
                    reason=reason,
                    needs_you=False, exit_code=EXIT_JOB,
                    tried=tried,
                    why="The decision was no. The job file was kept.",
                    nxt="A later yes is a new approve file, not a guess from silence.",
                    pid_alive="not-a-gpu-job", gpu="unknown",
                )
            if applying:
                reason = err or "An improve apply needs an approve file. A missing file is not a yes."
                tried = "Stopped before writing a new dataset version. The training set was not edited."
                nxt = "A person writes the approve file with decision yes. This runner does not apply from silence."
            else:
                reason = err or "A device pack needs an approve file. A missing file is not a yes."
                tried = "Stopped before writing the device folder."
                nxt = "Grok Bot asks, then writes the approve file. This runner does not copy the model yet."
            return self._finish(
                job, state="needs-approval", result="needs-approval",
                reason=reason,
                needs_you=True, exit_code=0,
                tried=tried,
                why="There is no yes on the approve file.",
                nxt=nxt,
                pid_alive="not-a-gpu-job", gpu="unknown",
            )
        return None

    def _gap(self, job: dict) -> dict:
        cfg, err = self._cfg(job)
        if err:
            return err
        body = self._gap_body(cfg)
        write_json(self.cell_for(job["id"]) / "gap.json", body)
        ok = body["ok"]
        return self._finish(
            job,
            state="succeeded" if ok else "failed",
            result="pass" if ok else "fail",
            reason=body["reason"],
            needs_you=not ok,
            exit_code=0 if ok else body["exit_code"],
            tried="Counted rows per class and opened the exam file when it was named. No train was started.",
            why=self._gap_why(body),
            nxt="Fix the data in a new job. A yes on this job does not ignore a red row." if not ok
            else "A train job can call this report. This gap job did not train.",
            gap=body,
            unverified=body["reason"] if body.get("unverified") else None,
            pid_alive="not-a-gpu-job",
            gpu="unknown",
            artifacts=[{
                "name": "gap.json",
                "path": str(self.cell_for(job["id"]) / "gap.json"),
                "bytes": file_bytes(self.cell_for(job["id"]) / "gap.json"),
            }],
        )

    def _decontaminate(self, job: dict) -> dict:
        cfg, err = self._cfg(job)
        if err:
            return err
        body = decontaminate.check(
            Path(cfg.get("train") or ""),
            Path(cfg.get("exam") or ""),
            str(cfg.get("field") or ""),
            jaccard=float(cfg.get("jaccard") or 0.5),
            train_field=cfg.get("train_field"),
            exam_field=cfg.get("exam_field"),
        )
        out_path = self.cell_for(job["id"]) / "decontaminate.json"
        write_json(out_path, {
            "ok": body["ok"],
            "hit_count": body.get("hit_count", 0),
            "hits": body.get("hits", []),
            "params": body.get("params", {}),
            "reason": body["reason"],
        })
        if body["ok"]:
            nxt = "Nothing overlapped. Do not remove rows. The file is decontaminate.json."
        elif body.get("unverified"):
            nxt = "The comparison did not run. Fix the field names and submit a new job."
        else:
            nxt = "Remove the overlapping rows and submit a new job. The threshold stays."
        return self._finish(
            job,
            state="succeeded" if body["ok"] else "refused",
            result="pass" if body["ok"] else "refused",
            reason=body["reason"],
            needs_you=not body["ok"],
            exit_code=body["exit_code"],
            tried="Compared training text to exam text. ShareGPT user turns and question fields are read natively. Answers were not hashed.",
            why=body["reason"],
            nxt=nxt,
            unverified=body["reason"] if body.get("unverified") else None,
            pid_alive="not-a-gpu-job",
            gpu="unknown",
            artifacts=[{
                "name": "decontaminate.json",
                "path": str(out_path),
                "bytes": file_bytes(out_path),
            }],
        )

    def _train(self, job: dict) -> dict:
        cfg, err = self._cfg(job)
        if err:
            return err
        gates = []
        body = self._gap_body(cfg)
        write_json(self.cell_for(job["id"]) / "gap.json", body)
        gates.append("gap:" + ("pass" if body["ok"] else "fail"))
        if not body["ok"]:
            return self._finish(
                job, state="failed", result="fail", reason=body["reason"],
                needs_you=True, exit_code=body["exit_code"],
                tried="Ran the gap report before any train plan.",
                why=body["reason"],
                nxt="Fix the data in a new job. This train did not start.",
                gap=body, unverified=body["reason"] if body.get("unverified") else None,
                pid_alive="no", gpu="unknown",
            )
        decon = decontaminate.check(
            Path(cfg.get("dataset") or cfg.get("train") or ""),
            Path(cfg.get("exam") or ""),
            str(cfg.get("compare_field") or cfg.get("field") or "text"),
            train_field=cfg.get("train_field"),
            exam_field=cfg.get("exam_field"),
        )
        gates.append("decontaminate:" + ("pass" if decon["ok"] else "fail"))
        write_json(self.cell_for(job["id"]) / "decontaminate.json", {
            "ok": decon["ok"], "reason": decon["reason"], "params": decon.get("params", {}),
        })
        if not decon["ok"]:
            return self._finish(
                job, state="refused", result="refused", reason=decon["reason"],
                needs_you=True, exit_code=decon["exit_code"],
                tried="Ran decontaminate after the gap report and before any train plan.",
                why=decon["reason"],
                nxt="The train did not start. The threshold was not lowered.",
                unverified=decon["reason"] if decon.get("unverified") else None,
                pid_alive="no", gpu="unknown",
            )
        page = Path(cfg["base_pick_page"]) if cfg.get("base_pick_page") else None
        pin_reason = None
        if base_pick.page_missing(page):
            pin = cfg.get("base_pin") or cfg.get("prior_receipt")
            if pin:
                pinned = pinned_base.write_pin(
                    Path(pin),
                    str(cfg.get("base") or ""),
                    self.cell_for(job["id"]) / "base-pick.md",
                )
                if pinned["ok"]:
                    page = Path(pinned["page"])
                    pin_reason = pinned["reason"]
                    gates.append("base-pin:receipt")
                else:
                    return self._finish(
                        job, state="refused", result="refused", reason=pinned["reason"],
                        needs_you=True, exit_code=EXIT_BASE,
                        tried="Looked for a prior receipt that already names this base. Did not load weights.",
                        why=pinned["reason"],
                        nxt="Point base_pin at a receipt that names the same base, or write a base-pick page.",
                        unverified=pinned["reason"],
                        pid_alive="no", gpu="unknown",
                    )
            else:
                reason = "A full train refuses because the base-pick page is missing."
                return self._finish(
                    job, state="refused", result="refused", reason=reason,
                    needs_you=True, exit_code=EXIT_BASE,
                    tried="Looked for the base-pick page before any train plan.",
                    why=reason,
                    nxt="Run the base-pick probe, or pin the base from a prior receipt. This runner does not load weights.",
                    unverified="This run could not see a base-pick page, so the train comparison was not checked.",
                    pid_alive="no", gpu="unknown",
                )
        if "base-pin:receipt" not in gates:
            gates.append("base-pick-page:present")
        recipe = str(cfg.get("recipe") or "")
        command = f"scripts/train-step.sh {recipe}" if recipe else "scripts/train-step.sh <missing recipe>"
        plan = {
            "command": command,
            "gates": gates,
            "launched": False,
            "allow_launch": bool(cfg.get("allow_launch")),
            "pin_reason": pin_reason,
        }
        write_json(self.cell_for(job["id"]) / "plan.json", plan)
        return self._after_plan(job, plan, kind="train")

    def _export(self, job: dict) -> dict:
        cfg, err = self._cfg(job)
        if err:
            return err
        recipe = str(cfg.get("recipe") or "")
        command = f"scripts/export-step.sh {recipe}" if recipe else "scripts/export-step.sh <missing recipe>"
        plan = {"command": command, "gates": ["export:plan-only"], "launched": False}
        write_json(self.cell_for(job["id"]) / "plan.json", plan)
        out = self._after_plan(job, plan, kind="export")
        if out.get("state") == "succeeded" and out.get("launched"):
            registry.append(self.inbox.registry, {
                "id": job["id"],
                "base": cfg.get("base"),
                "recipe": recipe,
                "dataset": cfg.get("dataset"),
                "exam": cfg.get("exam"),
                "quant": cfg.get("quant"),
                "gate": "pass",
                "reason": out["reason"],
                "nas_path": cfg.get("nas_path") or job["nas_out"],
            })
        return out

    def _eval(self, job: dict) -> dict:
        cfg, err = self._cfg(job)
        if err:
            return err
        if cfg.get("exam"):
            return self._eval_exam(job, cfg)
        done = Path(cfg["done_file"]) if cfg.get("done_file") else self.cell_for(job["id"]) / "DONE"
        ran = self.trainer(self._plan(job, "eval")) if self.trainer else dict(PLAN_ONLY)
        judged = judge.judge(
            scorer_exit=ran.get("scorer_exit", ran.get("rc")),
            server_started=bool(ran.get("server_started")),
            done_file_exists=done.is_file(),
        )
        if judged["state"] == "failed":
            failures.append_miss(
                self.inbox.corrections,
                job_id=job["id"],
                expected_shape=str(cfg.get("expected_shape") or ""),
                reason=judged["reason"],
            )
        if judged["scored"]:
            registry.append(self.inbox.registry, {
                "id": job["id"],
                "base": cfg.get("base"),
                "recipe": cfg.get("recipe"),
                "dataset": cfg.get("dataset"),
                "exam": cfg.get("exam"),
                "quant": cfg.get("quant"),
                "gate": "pass" if judged["result"] == "pass" else "fail",
                "reason": judged["reason"],
                "nas_path": job["nas_out"],
            })
        return self._finish(
            job,
            state=judged["state"],
            result=judged["result"],
            reason=judged["reason"],
            needs_you=not judged["ok"],
            exit_code=judged["exit_code"],
            tried="Asked the stand-in whether a server started and what the scorer returned. No live server was started by this runner.",
            why=judged["reason"],
            nxt="A failed score is appended to the one corrections file. Nothing retrains from this job.",
            pid_alive="no",
            gpu="unknown",
            gpu_minutes=float(ran.get("gpu_minutes") or 0),
        )

    def _eval_exam(self, job: dict, cfg: dict) -> dict:
        if cfg.get("self_serve"):
            return self._eval_served(job, cfg)
        grade = str(cfg.get("grade") or "")
        if grade in ("key-fact", "keyfact", "judge"):
            return self._eval_keyfact(job, cfg, use_judge=grade == "judge")
        from .common import EXIT_JUDGE
        done = Path(cfg["done_file"]) if cfg.get("done_file") else self.cell_for(job["id"]) / "DONE"
        score_path = self.cell_for(job["id"]) / "score.json"
        if self.exam_ask is None and not cfg.get("server_url"):
            judged = judge.judge(scorer_exit=None, server_started=False, done_file_exists=done.is_file())
            extra = " No local server URL was named."
            judged["reason"] = one_sentence(judged["reason"] + extra)
            return self._finish(
                job, state=judged["state"], result=judged["result"], reason=judged["reason"],
                needs_you=True, exit_code=judged["exit_code"],
                tried="Looked for a local server URL before scoring the exam. Did not start a server.",
                why=judged["reason"],
                nxt="Name server_url for a llama.cpp, vLLM, or Ollama server that is already up. A DONE file is not a score.",
                pid_alive="no", gpu="unknown",
            )

        def ask(question, row):
            if self.exam_ask:
                return self.exam_ask(question, row)
            return exam_score.ask_local(
                question,
                backend=str(cfg.get("backend") or "llamacpp"),
                url=str(cfg.get("server_url") or ""),
                model=cfg.get("model"),
            )

        scored = exam_score.score_exam(
            Path(cfg["exam"]),
            ask,
            expected_field=str(cfg.get("expected_field") or "answerable_because"),
            question_field=str(cfg.get("question_field") or "question"),
        )
        body = {
            "score": scored.get("score"),
            "correct": scored.get("correct"),
            "total": scored.get("total"),
            "method": scored.get("method"),
            "reason": scored.get("reason"),
            "misses": scored.get("misses"),
            "server_started": scored.get("server_started"),
            "adapter": cfg.get("adapter"),
            "backend": cfg.get("backend") or "llamacpp",
            "server_url": cfg.get("server_url"),
        }
        write_json(score_path, body)
        if not scored.get("server_started"):
            judged = judge.judge(scorer_exit=None, server_started=False, done_file_exists=done.is_file())
            judged["reason"] = scored["reason"]
            return self._finish(
                job, state="refused", result="refused", reason=judged["reason"],
                needs_you=True, exit_code=judged["exit_code"],
                tried="Asked the local server to score the exam. It did not answer.",
                why=judged["reason"],
                nxt="Start the local server, then run this job again. A DONE file is not a score.",
                pid_alive="no", gpu="unknown",
                artifacts=[{"name": "score.json", "path": str(score_path), "bytes": file_bytes(score_path)}],
            )
        total = int(scored.get("total") or 0)
        correct = int(scored.get("correct") or 0)
        bar = cfg.get("min_correct")
        bar_n = total if bar is None else int(bar)
        passed = total > 0 and correct >= bar_n
        if not passed:
            failures.append_miss(
                self.inbox.corrections,
                job_id=job["id"],
                expected_shape=str(cfg.get("expected_shape") or scored.get("score") or ""),
                reason=scored["reason"],
            )
        registry.append(self.inbox.registry, {
            "id": job["id"],
            "base": cfg.get("base"),
            "recipe": cfg.get("recipe"),
            "dataset": cfg.get("dataset"),
            "exam": cfg.get("exam"),
            "quant": cfg.get("quant"),
            "gate": "pass" if passed else "fail",
            "reason": scored["reason"],
            "nas_path": job["nas_out"],
        })
        return self._finish(
            job,
            state="succeeded" if passed else "failed",
            result="pass" if passed else "fail",
            reason=scored["reason"],
            needs_you=not passed,
            exit_code=0 if passed else EXIT_JUDGE,
            tried="Scored the exam file against a local server. This step did not start the server and did not load weights.",
            why=scored["reason"],
            nxt="The score file is score.json. A miss is one line in the corrections file. Nothing retrains from this job.",
            pid_alive="no",
            gpu="unknown",
            artifacts=[{"name": "score.json", "path": str(score_path), "bytes": file_bytes(score_path)}],
        )

    def _note_probe(self, probe: dict) -> None:
        seen = bool(probe.get("seen"))
        self.probed_gpu = probe.get("state") if seen and probe.get("state") in ("idle", "busy") else "unknown"
        self.probe_source = probe.get("source")

    def _step_receipt(self, job: dict, step: str, lines: list[str]) -> Path:
        path = self.cell_for(job["id"]) / "receipts" / f"{job['id']}-{step}.md"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            "\n".join([
                f"# {job['id']} {step}",
                "",
                *lines,
                "",
                "READY_FOR_LIVE_TEST: no",
                "This receipt does not flip READY and does not seat a model.",
                "",
            ]),
            encoding="utf-8",
        )
        return path

    def _judge_fn(self, cfg: dict):
        if self.judge_ask:
            return self.judge_ask
        url = cfg.get("judge_url")
        if not url:
            return None

        def ask(prompt: str) -> str:
            return exam_score.ask_local(
                prompt,
                backend=str(cfg.get("judge_backend") or "vllm"),
                url=str(url),
                model=cfg.get("judge_model"),
            )

        return ask

    def _score_only(self, cfg: dict, *, url: str | None = None, use_judge: bool = False) -> dict:
        grade = str(cfg.get("grade") or "overlap")
        keyfact = grade in ("key-fact", "keyfact", "judge") or use_judge

        def ask(question, row):
            if self.exam_ask:
                return self.exam_ask(question, row)
            return exam_score.ask_local(
                question,
                backend=str(cfg.get("backend") or "vllm"),
                url=str(url or cfg.get("server_url") or ""),
                model=cfg.get("model"),
            )

        if keyfact:
            return exam_keyfact.score_with_ask(
                Path(cfg["exam"]),
                ask,
                judge=self._judge_fn(cfg),
                use_judge=use_judge or grade == "judge",
            )
        return exam_score.score_exam(
            Path(cfg["exam"]),
            ask,
            expected_field=str(cfg.get("expected_field") or "answerable_because"),
            question_field=str(cfg.get("question_field") or "question"),
        )

    def _finish_scored(self, job: dict, cfg: dict, scored: dict, *, tried: str, score_path: Path, hold: bool = False) -> dict:
        from .common import EXIT_JUDGE
        body = {
            "score": scored.get("score"),
            "correct": scored.get("correct"),
            "total": scored.get("total"),
            "method": scored.get("method"),
            "reason": scored.get("reason"),
            "misses": scored.get("misses"),
            "server_started": scored.get("server_started"),
            "adapter": cfg.get("adapter"),
            "backend": cfg.get("backend") or "llamacpp",
            "server_url": cfg.get("server_url"),
        }
        if scored.get("overlap_score"):
            body["overlap_score"] = scored.get("overlap_score")
            body["keyfact_score"] = scored.get("keyfact_score")
            body["facts_derived"] = scored.get("facts_derived")
        write_json(score_path, body)
        replies_path = self._write_replies(job, scored, score_path)
        if not scored.get("server_started"):
            return self._finish(
                job, state="refused", result="refused", reason=scored["reason"],
                needs_you=True, exit_code=EXIT_JUDGE,
                tried=tried, why=scored["reason"],
                nxt="Start the local server, then run this job again. A DONE file is not a score.",
                pid_alive="no", gpu="unknown",
                artifacts=[{"name": "score.json", "path": str(score_path), "bytes": file_bytes(score_path)}],
            )
        total = int(scored.get("total") or 0)
        correct = int(scored.get("correct") or 0)
        bar = cfg.get("min_correct")
        bar_n = total if bar is None else int(bar)
        passed = total > 0 and correct >= bar_n
        if not passed:
            failures.append_miss(
                self.inbox.corrections,
                job_id=job["id"],
                expected_shape=str(cfg.get("expected_shape") or scored.get("score") or ""),
                reason=scored["reason"],
            )
        registry.append(self.inbox.registry, {
            "id": job["id"],
            "base": cfg.get("base"),
            "recipe": cfg.get("recipe"),
            "dataset": cfg.get("dataset"),
            "exam": cfg.get("exam"),
            "quant": cfg.get("quant"),
            "gate": "pass" if passed else "fail",
            "reason": scored["reason"],
            "nas_path": job["nas_out"],
        })
        if hold:
            return {
                "held": True,
                "passed": passed,
                "reason": scored["reason"],
                "score_path": score_path,
                "replies_path": replies_path,
                "tried": tried,
            }
        return self._finish(
            job,
            state="succeeded" if passed else "failed",
            result="pass" if passed else "fail",
            reason=scored["reason"],
            needs_you=not passed,
            exit_code=0 if passed else EXIT_JUDGE,
            tried=tried,
            why=scored["reason"],
            nxt="The score file is score.json. Overlap is recorded beside the key-fact score. Nothing retrains from this job.",
            pid_alive="no",
            gpu="unknown",
            artifacts=[{"name": "score.json", "path": str(score_path), "bytes": file_bytes(score_path)}],
        )

    def _eval_keyfact(self, job: dict, cfg: dict, *, use_judge: bool = False) -> dict:
        done = Path(cfg["done_file"]) if cfg.get("done_file") else self.cell_for(job["id"]) / "DONE"
        score_path = self.cell_for(job["id"]) / "score.json"
        if self.exam_ask is None and not cfg.get("server_url") and not (use_judge and self.judge_ask and cfg.get("replies")):
            judged = judge.judge(scorer_exit=None, server_started=False, done_file_exists=done.is_file())
            judged["reason"] = one_sentence(judged["reason"] + " No local server URL was named.")
            return self._finish(
                job, state=judged["state"], result=judged["result"], reason=judged["reason"],
                needs_you=True, exit_code=judged["exit_code"],
                tried="Looked for a local server URL before the key-fact grade. Did not start a server.",
                why=judged["reason"],
                nxt="Name server_url for a server that is already up, or set self_serve. A DONE file is not a score.",
                pid_alive="no", gpu="unknown",
            )
        scored = self._score_only(cfg, use_judge=use_judge)
        return self._finish_scored(
            job, cfg, scored, score_path=score_path,
            tried="Scored the exam by key facts against a server that was already up. This step did not start the server.",
        )

    def _serve_hooks(self):
        if self.server is not None:
            return self.server.start, self.server.stop, self.server.health
        return eval_serve.start_live, eval_serve.stop_live, eval_serve.health_live

    def _eval_served(self, job: dict, cfg: dict, hold: bool = False) -> dict:
        probe = self.gpu_probe()
        self._note_probe(probe)
        fake = self.server is not None
        ok, why = eval_serve.may_serve(probe, fake_server=fake)
        spec = eval_serve.build_spec(cfg, yaml_path=self.cell_for(job["id"]) / "serve.yaml")
        spec["log_path"] = str(Path(job["nas_out"]) / "serve.log")
        self._serve_log = spec["log_path"]
        self._step_receipt(job, "serve", [
            why,
            f"probe source: {probe.get('source') or 'unseen'}",
            f"command: {spec['command']}",
            f"health url: {spec['health_url']}",
            f"API_HOST: {spec['env']['API_HOST']}",
            f"API_PORT: {spec['env']['API_PORT']}",
            f"health timeout seconds: {spec['health_timeout']}",
            f"log: {spec['log_path']}",
            f"cap minutes: {spec['max_minutes']}",
            "The server was not started yet." if not ok else "A start was allowed.",
        ])
        if not ok:
            return self._finish(
                job, state="refused", result="refused", reason=why,
                needs_you=True, exit_code=EXIT_GPU_BUSY if not probe.get("seen") or probe.get("state") != "idle" else EXIT_JOB,
                tried="Probed the GPU before starting a server. Did not start it.",
                why=why,
                nxt="A real nvidia-smi idle read is required before a live server. A test hook is not that read.",
                pid_alive="no", gpu=self.probed_gpu,
            )
        gate = spec.get("cli_gate")
        if gate:
            return self._finish(
                job, state="refused", result="refused", reason=gate,
                needs_you=True, exit_code=EXIT_JOB,
                tried="Looked up llamafactory-cli before starting the server. Did not start it.",
                why=gate,
                nxt="Set llamafactory_cli or LLAMAFACTORY_CLI to an executable, or place it beside QUANT_LADDER_PYTHON.",
                pid_alive="no", gpu=self.probed_gpu,
            )
        starter, stopper, health = self._serve_hooks()
        grade = str(cfg.get("grade") or "overlap")

        def work():
            return self._score_only(cfg, url=spec["url"], use_judge=grade == "judge")

        def remembering(spec_in):
            handle = starter(spec_in)
            self._serve_handle = handle
            return handle

        result = eval_serve.guarded_serve(
            spec,
            starter=remembering,
            stopper=stopper,
            health=health,
            work=work,
            clock=self.clock,
            sleep=self.sleep,
            max_minutes=spec["max_minutes"],
        )
        self._step_receipt(job, "serve-stop", [
            result.get("reason") or "The serve step finished.",
            f"started: {bool(result.get('started'))}",
            f"stopped: {bool(result.get('stopped'))}",
            f"healthy: {bool(result.get('healthy'))}",
            f"log: {spec.get('log_path') or 'none'}",
            "READY stays no. This receipt is not a score by itself.",
        ])
        if not result.get("scored"):
            started = bool(result.get("started"))
            return self._finish(
                job, state="refused", result="refused",
                reason=result.get("reason") or "The server did not produce a score.",
                needs_you=True, exit_code=EXIT_JUDGE,
                tried=(
                    "Started the local server, wrote its log, then stopped it. This is not a score."
                    if started
                    else "Checked the port and the CLI before starting the server. Did not start it."
                ),
                why=result.get("reason") or "The server did not produce a score.",
                nxt=(
                    "The server log is quoted below. The trap stopped the server. A DONE file is not a score."
                    if started
                    else "Free the port, or set llamafactory-cli, then run this job again. A DONE file is not a score."
                ),
                pid_alive="no", gpu=self.probed_gpu,
                quote=eval_serve.page_quote(spec.get("log_path")) or None,
            )
        score_path = self.cell_for(job["id"]) / "score.json"
        return self._finish_scored(
            job, cfg, result, score_path=score_path, hold=hold,
            tried=self._with_train_note(
                "Started a local LLaMA-Factory API, scored, and stopped it."
            ),
        )

    def _run_chain(self, job: dict) -> dict:
        cfg, err = self._cfg(job)
        if err:
            return err
        stopped = self._run_gates(job, cfg)
        if stopped is not None:
            return stopped
        trained = self._run_train_step(job, cfg)
        if trained is not None:
            return trained
        if cfg and cfg.get("improve"):
            return self._run_eval_and_improve(job, cfg)
        return self._run_eval_step(job, cfg)

    def _run_gates(self, job: dict, cfg: dict):
        self._active_step = "gap"
        body = self._gap_body(cfg)
        write_json(self.cell_for(job["id"]) / "gap.json", body)
        self._step_receipt(job, "gap", [
            body["reason"],
            "This step did not use the GPU.",
            "Authorized by the run approve file. No second approve file was read.",
        ])
        if not body["ok"]:
            return self._finish(
                job, state="failed", result="fail", reason=body["reason"],
                needs_you=True, exit_code=body["exit_code"],
                tried="Ran the gap report. The run stopped. Train and eval did not start.",
                why=body["reason"],
                nxt="Fix the data in a new job. This run did not train.",
                gap=body, pid_alive="no", gpu="unknown",
            )
        self._active_step = "decontaminate"
        decon = decontaminate.check(
            Path(cfg.get("dataset") or cfg.get("train") or ""),
            Path(cfg.get("exam") or ""),
            str(cfg.get("compare_field") or cfg.get("field") or "text"),
            train_field=cfg.get("train_field"),
            exam_field=cfg.get("exam_field"),
        )
        write_json(self.cell_for(job["id"]) / "decontaminate.json", {
            "ok": decon["ok"], "reason": decon["reason"], "params": decon.get("params", {}),
        })
        self._step_receipt(job, "decontaminate", [
            decon["reason"],
            "This step did not use the GPU.",
            "Authorized by the run approve file. No second approve file was read.",
        ])
        if not decon["ok"]:
            return self._finish(
                job, state="refused", result="refused", reason=decon["reason"],
                needs_you=True, exit_code=decon["exit_code"],
                tried="Decontaminate failed. The run stopped before train.",
                why=decon["reason"],
                nxt="The train did not start. The threshold was not lowered.",
                pid_alive="no", gpu="unknown",
            )
        return None

    def _run_train_step(self, job: dict, cfg: dict):
        self._active_step = "train"
        pin_stop = self._run_pin(job, cfg)
        if pin_stop is not None:
            return pin_stop
        probe = self.gpu_probe()
        self._note_probe(probe)
        allowed, why = self._gpu_child_allowed(
            probe, fake_child=self.launch_runner is not None, minutes=job["max_minutes"],
        )
        recipe = str(cfg.get("recipe") or "")
        command = run_train_command(recipe, job["max_minutes"])
        self._step_receipt(job, "train", [
            why,
            f"probe source: {probe.get('source') or 'unseen'}",
            f"command: {command}",
            "Authorized by the run approve file. This step probed again. No second approve file was read.",
            "allow_launch on the config is not required for this run.",
        ])
        if not allowed:
            return self._finish(
                job, state="refused", result="refused", reason=why,
                needs_you=True, exit_code=EXIT_GPU_BUSY,
                tried="Probed again before train. The run stopped. Eval did not start.",
                why=why,
                nxt="An approve file does not override a busy card or an unseen guard.",
                pid_alive="no", gpu=self.probed_gpu,
            )
        if not recipe:
            reason = "The run has no recipe, so train did not start."
            return self._finish(
                job, state="refused", result="refused", reason=reason,
                needs_you=True, exit_code=EXIT_LAUNCH,
                tried="Looked for a recipe before train. Did not start it.",
                why=reason, nxt="Name a recipe in the config. This run did not train.",
                pid_alive="no", gpu=self.probed_gpu,
            )
        plan = {
            "command": command,
            "gates": ["gap:pass", "decontaminate:pass", "run-approve:yes"],
            "launched": False,
            "allow_launch": True,
        }
        write_json(self.cell_for(job["id"]) / "plan.json", plan)
        ran = dict(self._run_plan(plan))
        parsed = parse_train_result(ran.get("rc"), ran.get("output") or "", ran.get("result"))
        ran["result"] = parsed["result"]
        ran["train_pid"] = parsed["pid"]
        self.train_pid = parsed["pid"]
        self._write_launch_receipt(job, plan, ran)
        if not ran.get("launched") or not parsed["pass"]:
            if not ran.get("launched"):
                reason = "Train did not start, so the run stopped."
                state, result, code = "refused", "refused", EXIT_LAUNCH
            elif parsed["rc"] == 0 and not parsed["result"]:
                if parsed["pid"]:
                    reason = (
                        f"Train launched pid {parsed['pid']} and returned rc=0 without result=ok, "
                        "so this was a detached launch. The run stopped before eval."
                    )
                else:
                    reason = (
                        "Train returned rc=0 without result=ok, so this was a detached launch. "
                        "The run stopped before eval."
                    )
                state, result, code = "failed", "fail", 24
            else:
                word = parsed["result"] or "missing"
                reason = (
                    f"Train returned rc={parsed['rc']} result={word}, so the run stopped. "
                    "A pass needs rc=0 and result=ok."
                )
                state, result = "failed", "fail"
                code = parsed["rc"] if parsed["rc"] not in (0, None) else 24
            return self._finish(
                job, state=state, result=result, reason=reason, needs_you=True,
                exit_code=code,
                tried="The run approve file authorized train. Eval did not start.",
                why=reason, nxt="Read the train receipt. Nothing was seated.",
                pid_alive="no", gpu=self.probed_gpu,
            )
        return None

    def _run_pin(self, job: dict, cfg: dict):
        page = Path(cfg["base_pick_page"]) if cfg.get("base_pick_page") else None
        if not base_pick.page_missing(page):
            return None
        pin = cfg.get("base_pin") or cfg.get("prior_receipt")
        if pin:
            pinned = pinned_base.write_pin(
                Path(pin),
                str(cfg.get("base") or ""),
                self.cell_for(job["id"]) / "base-pick.md",
            )
            if pinned["ok"]:
                self._step_receipt(job, "base-pin", [pinned["reason"], "No weights were loaded."])
                return None
            return self._finish(
                job, state="refused", result="refused", reason=pinned["reason"],
                needs_you=True, exit_code=EXIT_BASE,
                tried="Looked for a prior receipt that names this base. Did not load weights. The run stopped.",
                why=pinned["reason"],
                nxt="Point base_pin at a receipt that names the same base.",
                pid_alive="no", gpu="unknown",
            )
        reason = "A full train refuses because the base-pick page is missing."
        return self._finish(
            job, state="refused", result="refused", reason=reason,
            needs_you=True, exit_code=EXIT_BASE,
            tried="Looked for the base-pick page. The run stopped before train.",
            why=reason, nxt="Pin the base from a prior receipt. This runner does not load weights.",
            pid_alive="no", gpu="unknown",
        )

    def _gpu_child_allowed(self, probe: dict, *, fake_child: bool, minutes=15) -> tuple[bool, str]:
        notice = gpu_notice(probe, minutes)
        if notice["blocked"] or notice["gpu"] != "idle":
            return False, notice["reason"]
        source = probe.get("source") or ""
        if source == "nvidia-smi":
            return True, notice["reason"]
        if fake_child and source in ("fake", "test"):
            return True, notice["reason"]
        return False, one_sentence("The probe was not a nvidia-smi read, so this GPU step did not start.")

    def _with_train_note(self, tried: str) -> str:
        note = self.train_pid_note or ""
        if not note or note in tried:
            return tried
        return f"{tried} {note}"

    def _run_eval_step(self, job: dict, cfg: dict, hold: bool = False) -> dict:
        self._active_step = "eval"
        probe = self.gpu_probe()
        note = classify_own_train(probe, self.train_pid)
        adjusted = dict(probe)
        if note["kind"] == "own-finished" and not note["blocks"]:
            adjusted["state"] = "idle"
            adjusted["holder"] = None
        self._note_probe(probe if note["kind"] == "own-live" or note["blocks"] else adjusted)
        fake = self.server is not None or self.exam_ask is not None
        allowed, why = self._gpu_child_allowed(
            adjusted, fake_child=fake, minutes=job["max_minutes"],
        )
        if note["kind"] == "own-live" and note["blocks"]:
            allowed = False
            cap = format_minutes(job["max_minutes"])
            why = (
                f"{note['sentence']} The minute cap is {cap}. "
                "An approve file does not override a busy card."
            )
            self._note_probe(probe)
        elif note["sentence"]:
            why = f"{note['sentence']} {why}"
        self.train_pid_note = note["sentence"]
        self._step_receipt(job, "eval", [
            why,
            f"probe source: {probe.get('source') or 'unseen'}",
            f"train pid: {self.train_pid if self.train_pid else 'none'}",
            note["kind"],
            "Authorized by the run approve file. This step probed again. No second approve file was read.",
        ])
        if not allowed:
            return self._finish(
                job, state="refused", result="refused", reason=why,
                needs_you=True, exit_code=EXIT_GPU_BUSY,
                tried="Probed again before eval. The run stopped. The server was not started.",
                why=why, nxt="An approve file does not override a busy card or an unseen guard.",
                pid_alive="no", gpu=self.probed_gpu,
            )
        if cfg.get("self_serve"):
            return self._eval_served(job, cfg, hold=hold)
        if self.exam_ask is None and not cfg.get("server_url"):
            reason = "The run reached eval with no server and no self_serve flag, so this is not a score."
            return self._finish(
                job, state="refused", result="refused", reason=reason,
                needs_you=True, exit_code=EXIT_JUDGE,
                tried=self._with_train_note(
                    "Train finished. Eval did not start a server because self_serve was not set."
                ),
                why=reason, nxt="Set self_serve, or name a server that is already up.",
                pid_alive="no", gpu=self.probed_gpu,
            )
        grade = str(cfg.get("grade") or "")
        scored = self._score_only(cfg, use_judge=grade == "judge")
        score_path = self.cell_for(job["id"]) / "score.json"
        return self._finish_scored(
            job, cfg, scored, score_path=score_path, hold=hold,
            tried=self._with_train_note(
                "The run scored against a server that was already up. This step did not start one."
            ),
        )

    def _write_replies(self, job: dict, scored: dict, score_path: Path):
        items = scored.get("items") or []
        rows = []
        for item in items:
            if "reply" not in item:
                continue
            rows.append({"id": item.get("id"), "reply": item.get("reply") or ""})
        if not rows:
            return None
        text = "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in rows)
        dest = score_path.parent / "replies.jsonl"
        dest.write_text(text, encoding="utf-8")
        nas = Path(job["nas_out"]) / "replies.jsonl"
        try:
            guarded(nas)
            nas.parent.mkdir(parents=True, exist_ok=True)
            nas.write_text(text, encoding="utf-8")
        except ValueError:
            pass
        return dest

    def _run_eval_and_improve(self, job: dict, cfg: dict) -> dict:
        held = self._run_eval_step(job, cfg, hold=True)
        if not held.get("held"):
            return held
        out_dir = Path(cfg["proposal_dir"]) if cfg.get("proposal_dir") else Path(job["nas_out"]) / "improve"
        proposal = improve_data.propose(
            cfg,
            out_dir,
            job_id=job["id"],
            replies_path=held.get("replies_path"),
            score_path=held.get("score_path"),
        )
        self._step_receipt(job, "improve", [
            proposal.get("reason") or "Improve finished.",
            "This step did not use the GPU and did not edit the training set.",
            "No second approve file was read. Apply was not run.",
        ])
        passed = bool(held.get("passed"))
        reason = held.get("reason") or ""
        extra = proposal.get("reason") or "Improve did not write a proposal."
        reason = one_sentence(f"{reason} {extra}")
        artifacts = [{"name": "score.json", "path": str(held.get("score_path")), "bytes": file_bytes(Path(held["score_path"]))}]
        artifacts.extend(proposal.get("artifacts") or [])
        if proposal.get("ok"):
            nxt = (
                "The score file is score.json. The improve proposal is under improve/. "
                "The training set was not edited. Apply needs its own approve file."
            )
        else:
            nxt = "The score stands. Improve did not edit the training set. Apply was not started."
        return self._finish(
            job,
            state="succeeded" if passed else "failed",
            result="pass" if passed else "fail",
            reason=reason,
            needs_you=(not passed) or bool(proposal.get("needs_you")),
            exit_code=0 if passed else EXIT_JUDGE,
            tried=self._with_train_note(
                "Scored, then wrote an improve proposal. Did not edit the training set. Did not apply."
            ),
            why=reason,
            nxt=nxt,
            artifacts=artifacts,
            pid_alive="no",
            gpu=self.probed_gpu or "unknown",
        )

    def _improve(self, job: dict) -> dict:
        cfg, err = self._cfg(job)
        if err or cfg is None:
            return err or {"ok": False, "state": "refused", "reason": "The config could not be read.", "exit_code": EXIT_JOB, "id": job["id"]}
        out_dir = Path(cfg["proposal_dir"]) if cfg.get("proposal_dir") else Path(job["nas_out"]) / "improve"
        result = improve_data.propose(cfg, out_dir, job_id=job["id"])
        self._step_receipt(job, "improve", [
            result.get("reason") or "Improve finished.",
            "The training set was not edited.",
            "Cloud drafting stays off unless allow_cloud is true.",
        ])
        artifacts = result.get("artifacts") or []
        if not result.get("ok"):
            return self._finish(
                job, state="refused", result="refused", reason=result["reason"],
                needs_you=True, exit_code=result.get("exit_code") or EXIT_IMPROVE,
                tried="Read the score card and looked for replies. Did not edit the training set.",
                why=result["reason"],
                nxt="Point replies at the saved replies file, then run this job again. Apply was not started.",
                artifacts=artifacts, pid_alive="not-a-gpu-job", gpu="unknown",
            )
        return self._finish(
            job, state="succeeded", result="pass", reason=result["reason"],
            needs_you=True, exit_code=0,
            tried="Grouped misses, drafted rows from source notes, and ran decontaminate. Did not edit the training set.",
            why=result["reason"],
            nxt="A person writes an approve file with decision yes, then runs improve-apply. This step did not apply.",
            artifacts=artifacts, pid_alive="not-a-gpu-job", gpu="unknown",
        )

    def _improve_apply(self, job: dict) -> dict:
        approve, err = read_approve(self.inbox, job["id"])
        if approve is None or err or approve.get("decision") != "yes":
            return self._finish(
                job, state="needs-approval", result="needs-approval",
                reason=err or "An improve apply needs an approve file. A missing file is not a yes.",
                needs_you=True, exit_code=0,
                tried="Stopped before writing a new dataset version. The training set was not edited.",
                why="There is no yes on the approve file.",
                nxt="A person writes the approve file with decision yes. This runner does not apply from silence.",
                pid_alive="not-a-gpu-job", gpu="unknown",
            )
        cfg, cfg_err = self._cfg(job)
        if cfg_err or cfg is None:
            return cfg_err or {"ok": False, "state": "refused", "reason": "The config could not be read.", "exit_code": EXIT_JOB, "id": job["id"]}
        result = improve_data.apply_proposal(
            cfg,
            job_id=job["id"],
            approved_by=approve["by"],
            approved_at=approve["at"],
            receipt_dir=self.cell_for(job["id"]) / "receipts",
        )
        self._step_receipt(job, "apply", [
            result.get("reason") or "Apply finished.",
            f"original unchanged: {str(result.get('dataset_unchanged')).lower()}",
            "READY stays no. This step did not train.",
        ])
        if not result.get("ok"):
            return self._finish(
                job, state="refused", result="refused", reason=result["reason"],
                needs_you=True, exit_code=result.get("exit_code") or EXIT_IMPROVE,
                tried="Read the approve file. Did not edit the original training file.",
                why=result["reason"],
                nxt="Fix the proposal path. The original training file was left as it was.",
                artifacts=result.get("artifacts") or [],
                pid_alive="not-a-gpu-job", gpu="unknown",
            )
        return self._finish(
            job, state="succeeded", result="pass", reason=result["reason"],
            needs_you=False, exit_code=0,
            tried="Appended approved rows into a new dataset version folder. Did not edit the original.",
            why=result["reason"],
            nxt="The new folder has its own receipt. Nothing was seated. READY stays no.",
            artifacts=result.get("artifacts") or [],
            pid_alive="not-a-gpu-job", gpu="unknown",
        )

    def _quant(self, job: dict) -> dict:
        cfg, err = self._cfg(job)
        if err:
            return err
        ladder = Path(cfg.get("ladder") or "")
        if not ladder.is_file():
            reason = "This run could not see a ladder result, so nothing was checked."
            return self._finish(
                job, state="refused", result="refused", reason=reason,
                needs_you=True, exit_code=EXIT_JOB,
                tried="Looked for an existing quant-ladder result. Did not run a quantizer.",
                why=reason, nxt="Point the config at a ladder file that already exists.",
                unverified=reason, pid_alive="not-a-gpu-job", gpu="unknown",
            )
        body = read_json(ladder)
        passed = bool(body.get("passed"))
        pick = body.get("pick") or "the ranked pick"
        if passed:
            reason = (
                f"This size passed the gate. {pick} is the one to copy to the Mac "
                "only after a device-pack approve. It is not seated."
            )
            state, result, needs = "succeeded", "pass", False
        else:
            reason = (
                f"This size failed the gate. {pick} is for the next try, not for a device."
            )
            state, result, needs = "failed", "fail", True
        return self._finish(
            job, state=state, result=result, reason=reason, needs_you=needs, exit_code=0 if passed else 1,
            tried="Read the existing ladder result and wrote it in words. Did not quantize.",
            why=reason,
            nxt="Device copy stays behind the device-pack step and an approve file.",
            pid_alive="not-a-gpu-job", gpu="unknown",
        )

    def _ask(self, job: dict) -> dict:
        cfg, err = self._cfg(job)
        if err:
            return err
        table = Path(cfg.get("route_table") or "")
        row = route.lookup(table, str(cfg.get("task") or "")) if table.is_file() else None
        called = False
        seat_text = None
        if row and self.seat and job["gpu"] == "ask":
            seat_text = self.seat(row)
            called = True
        got = route.outcome(row=row, called=called, seat_text=seat_text)
        return self._finish(
            job, state=got["state"], result=got["result"], reason=got["reason"],
            needs_you=got["needs_you"], exit_code=0 if got["ok"] else 41,
            tried="Read the route table. Did not plan with a model. Did not call a cloud model.",
            why=got["reason"],
            nxt="If the table has no row, add a row and submit a new ask. This harness will not jump to a cloud model.",
            pid_alive="not-a-gpu-job", gpu="unknown",
        )

    def _pack(self, job: dict) -> dict:
        cfg, err = self._cfg(job)
        if err:
            return err
        src = Path(cfg.get("gguf") or "")
        dest = Path(job["nas_out"]) / "device"
        got = device_pack.pack(src, dest, approved=True)
        return self._finish(
            job, state=got["state"], result="pass" if got["ok"] else "refused",
            reason=got["reason"], needs_you=got["needs_you"], exit_code=got["exit_code"],
            tried="Copied a GGUF that was already on disk into a NAS folder.",
            why=got["reason"],
            nxt="Jason seats it later, or a later job does, with a second approve. This pack did not run ollama create.",
            artifacts=got.get("artifacts"),
            pid_alive="not-a-gpu-job", gpu="unknown",
        )

    def base_pick_run(self, job_id: str, cfg: dict, scorer=None) -> dict:
        """Short comparison of cached files. Does not download and does not load a live model."""
        fake_job = {
            "id": job_id,
            "step": "base-pick",
            "config": cfg.get("config") or "",
            "gpu": "ask",
            "max_minutes": cfg.get("max_minutes", 10),
            "nas_out": cfg.get("nas_out") or str(self.cell_for(job_id)),
        }
        approve, err = read_approve(self.inbox, job_id)
        notice = gpu_notice(self.gpu_probe(), fake_job["max_minutes"])
        if approve is None or err or approve.get("decision") != "yes":
            reason = err or "A base-pick probe needs an approve file before it runs. A missing file is not a yes."
            if approve and approve.get("decision") == "no":
                reason = "A no was written. The probe did not run."
            waiting = not (approve and approve.get("decision") == "no")
            return self._finish(
                fake_job,
                state="needs-approval" if waiting else "refused",
                result="needs-approval" if waiting else "refused",
                reason=reason, needs_you=True, exit_code=0,
                tried="Stopped before comparing cached models.",
                why=reason, nxt="Write the approve file. This probe does not start itself.",
                pid_alive="no", gpu=notice["gpu"],
            )
        if notice["blocked"]:
            return self._finish(
                fake_job, state="refused", result="refused", reason=notice["reason"],
                needs_you=True, exit_code=notice["exit_code"],
                tried="Read the GPU notice before the probe.",
                why=notice["reason"], nxt="An approve file does not override a busy card.",
                pid_alive="no", gpu=notice["gpu"],
            )
        if scorer is None:
            reason = "The probe did not load a model, so this is not a comparison."
            return self._finish(
                fake_job, state="refused", result="refused", reason=reason, needs_you=True,
                exit_code=EXIT_BASE,
                tried="Refused to load a cached model in this build.",
                why=reason,
                nxt="A stand-in scorer can record the comparison. This runner does not load weights.",
                unverified=reason, pid_alive="no", gpu="unknown",
            )
        body = base_pick.probe(
            list(cfg.get("models") or []),
            Path(cfg.get("exam") or ""),
            str(cfg.get("marker") or ""),
            scorer,
            fake_job["max_minutes"],
        )
        page = Path(fake_job["nas_out"]) / "base-pick.md"
        base_pick.write_comparison(page, body, needs_you=not body["ok"])
        return self._finish(
            fake_job,
            state="succeeded" if body["ok"] else "failed",
            result="pass" if body["ok"] else "fail",
            reason=body["reason"],
            needs_you=not body["ok"],
            exit_code=0 if body["ok"] else EXIT_BASE,
            tried="Compared cached files with a stand-in scorer. No download.",
            why=body["reason"],
            nxt="A full train refuses if this page is missing. This probe did not train.",
            unverified=None if body["exam_seen"] and not body["unseen"] else body["reason"],
            pid_alive="no",
            gpu="unknown",
            artifacts=[{"name": "base-pick.md", "path": str(page), "bytes": file_bytes(page)}],
        )

    def _after_plan(self, job: dict, plan: dict, *, kind: str) -> dict:
        ran = self._run_plan(plan)
        if plan.get("allow_launch") and self.trainer is None:
            self._write_launch_receipt(job, plan, ran)
        minutes = float(ran.get("gpu_minutes") or 0)
        pin = plan.get("pin_reason")
        def _why(text: str) -> str:
            return f"{pin} {text}" if pin else text
        if minutes > float(job["max_minutes"]):
            reason = (
                f"The stand-in reported {minutes} GPU minutes, over the cap of {job['max_minutes']}. "
                "refuse:gpu-cap. Nothing was killed by name."
            )
            return self._finish(
                job, state="refused", result="refused", reason=reason, needs_you=True,
                exit_code=EXIT_GPU_CAP,
                tried="Checked the minute cap on the stand-in result.",
                why=_why(reason), nxt="Raise the cap in a new job, or shorten the run. This runner did not kill by name.",
                pid_alive="no", gpu="unknown", gpu_minutes=minutes,
            )
        if not ran.get("launched"):
            reason = (
                f"Gates passed. The {kind} command was recorded and not started, so this is not a {kind}."
            )
            return self._finish(
                job, state="refused", result="refused", reason=reason, needs_you=True,
                exit_code=EXIT_LAUNCH,
                tried=f"Ran the gates, then wrote the {kind} command into the plan. Did not execute it.",
                why=_why(reason),
                nxt=f"The plan names {plan['command']}. Set allow_launch only after an approve file says yes and the probe reports idle. Otherwise a person runs that existing script. This runner will not.",
                pid_alive="no", gpu="unknown",
                artifacts=[{
                    "name": "plan.json",
                    "path": str(self.cell_for(job["id"]) / "plan.json"),
                    "bytes": file_bytes(self.cell_for(job["id"]) / "plan.json"),
                }],
            )
        if ran.get("rc") != 0:
            if plan.get("allow_launch") and self.trainer is None:
                reason = f"The recorded command returned {ran.get('rc')}, so this is not a pass."
            else:
                reason = f"The stand-in {kind} returned {ran.get('rc')}, so this is not a pass."
            return self._finish(
                job, state="failed", result="fail", reason=reason, needs_you=True,
                exit_code=ran.get("rc") or 1,
                tried=f"The {kind} command was started after the gates." if plan.get("allow_launch") and self.trainer is None else f"The stand-in {kind} ran in-process. llamafactory-cli was not started.",
                why=_why(reason), nxt="Read the card. Nothing was seated.",
                pid_alive="no", gpu="unknown", gpu_minutes=minutes,
            )
        if plan.get("allow_launch") and self.trainer is None:
            reason = (
                f"The launch lock was passed. Approve yes, probe {self.probed_gpu}, allow_launch true. "
                f"Command {plan['command']} returned 0."
            )
            tried = "Passed the launch lock and ran the recorded command. The harness did not load weights for the base pick."
            nxt = "Read the train log beside the recipe output. This card is not a score."
        else:
            reason = (
                f"Gates passed and the stand-in {kind} returned 0. "
                "llamafactory-cli was not started. This is not a live train."
            )
            tried = f"The stand-in {kind} ran in-process after the gates. No live trainer was started."
            nxt = "A live train is still a person running the recorded command. This runner did not."
        out = self._finish(
            job, state="succeeded", result="pass", reason=reason, needs_you=False,
            exit_code=0,
            tried=tried,
            why=_why(reason), nxt=nxt,
            pid_alive="no", gpu="unknown", gpu_minutes=minutes,
        )
        out["launched"] = True
        return out

    def _gap_body(self, cfg: dict) -> dict:
        mix = None
        if cfg.get("task_set") or cfg.get("refusal_set"):
            mix = {
                "task_set": cfg.get("task_set"),
                "refusal_set": cfg.get("refusal_set"),
                "refusal_marker": cfg.get("refusal_marker"),
                "target_field": cfg.get("target_field") or "target",
            }
        return gap.report(
            Path(cfg.get("dataset") or ""),
            list(cfg.get("labels") or []),
            str(cfg.get("label_field") or "label"),
            str(cfg.get("exam_shape") or ""),
            str(cfg.get("shape_field") or "text"),
            cfg.get("contrast_marker"),
            mix,
            exam=_exam_for_gap(cfg),
        )

    def _gap_why(self, body: dict) -> str:
        reason = body.get("reason") or ""
        extra = [note for note in (body.get("notes") or []) if note not in reason]
        if not extra:
            return reason
        return reason + " " + " ".join(extra)

    def _run_plan(self, plan: dict) -> dict:
        if self.trainer:
            return self.trainer(plan)
        if plan.get("allow_launch"):
            return launch_lock.execute_recorded(plan, runner=self.launch_runner)
        return dict(PLAN_ONLY)

    def _write_launch_receipt(self, job: dict, plan: dict, ran: dict) -> None:
        path = self.cell_for(job["id"]) / "receipts" / f"{job['id']}-launch.md"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            "\n".join([
                f"# {job['id']} launch",
                "",
                "The launch lock was passed because the approve file said yes, the probe reported "
                f"{self.probed_gpu}, and allow_launch is true.",
                f"probe source: {self.probe_source or 'probe'}",
                f"command: {plan.get('command')}",
                f"launched: {bool(ran.get('launched'))}",
                f"rc: {ran.get('rc')}",
                f"result: {ran.get('result') or 'missing'}",
                f"train pid: {ran.get('train_pid') if ran.get('train_pid') else 'none'}",
                plan.get("pin_reason") or "No prior-receipt pin was recorded.",
                "READY_FOR_LIVE_TEST: no",
                "This receipt does not flip READY and does not load weights.",
                "",
            ]),
            encoding="utf-8",
        )

    def _cfg(self, job: dict):
        path = Path(job["config"])
        try:
            return read_json(path), None
        except (OSError, ValueError) as exc:
            reason = f"The config could not be read, so this job did not run. {exc.__class__.__name__}."
            return None, self._finish(
                job, state="refused", result="refused", reason=reason, needs_you=False,
                exit_code=EXIT_JOB,
                tried="Opened the config path named by the job.",
                why=reason, nxt="Point config at a small JSON file that exists.",
                unverified=reason, pid_alive="not-a-gpu-job", gpu="unknown",
            )

    def _plan(self, job: dict, kind: str) -> dict:
        return {"command": f"not-launched:{kind}", "job_id": job["id"], "launched": False}

    def _existing(self, job_id: str):
        path = self.inbox.status_path(job_id)
        if not path.is_file():
            return None
        try:
            return read_json(path)
        except ValueError:
            return None

    def _bare(self, job_id, state, result, reason, needs_you, exit_code) -> dict:
        return {
            "ok": False,
            "id": job_id,
            "state": state,
            "result": result,
            "reason": one_sentence(reason),
            "needs_you": needs_you,
            "exit_code": exit_code,
        }

    def _serve_quote(self) -> str:
        return eval_serve.page_quote(self._serve_log)

    def _stop_any_server(self) -> None:
        handle = self._serve_handle
        self._serve_handle = None
        if handle is None:
            return
        stopper = self.server.stop if self.server is not None else eval_serve.stop_live
        try:
            stopper(handle)
        except Exception:
            return

    def _finish(
        self, job, *, state, result, reason, needs_you, exit_code,
        tried, why, nxt, unverified=None, gap=None, artifacts=None,
        pid_alive="not-a-gpu-job", gpu="unknown", gpu_minutes=0, legend=None,
        crash=False, step_name=None, quote=None,
    ) -> dict:
        reason = one_sentence(reason)
        if gpu == "unknown" and self.probed_gpu in ("idle", "busy"):
            gpu = self.probed_gpu
        existing = self._existing(job["id"])
        if (
            not self._force_page
            and existing
            and existing.get("state") == state
            and existing.get("reason") == reason
        ):
            return {
                "ok": state == "succeeded",
                "id": job["id"],
                "state": state,
                "result": result,
                "reason": reason,
                "needs_you": needs_you,
                "exit_code": exit_code,
                "replayed": True,
            }
        page = guarded(Path(job["nas_out"]) / "reading.md")
        reading.write_page(
            page, reason=reason, needs_you=needs_you, tried=tried, why=why, nxt=nxt,
            unverified=unverified, legend=legend, gap=gap, quote=quote,
        )
        arts = list(artifacts or [])
        arts.append({"name": "reading.md", "path": str(page), "bytes": file_bytes(page)})
        cell = self.cell_for(job["id"])
        card = make_card(
            job_id=job["id"], reason=reason, result=result, needs_you=needs_you,
            reading_page=str(page), artifacts=arts, gpu_minutes=gpu_minutes,
        )
        status = make_status(
            job_id=job["id"], state=state, step=step_name or job["step"], pid_alive=pid_alive,
            gpu=gpu, reason=reason, reading_page=str(page), card_path="",
        )
        if crash:
            status["crash"] = True
            status["step"] = step_name or self._active_step or job["step"]
        publish(self.inbox, cell, status, card)
        log = cell / "logs" / "job.log"
        log.parent.mkdir(parents=True, exist_ok=True)
        kind = "error" if state in ("failed", "refused") else "note"
        with open(log, "a", encoding="utf-8") as f:
            f.write(f"{now_iso()} {kind}: {reason}\n")
        receipt = cell / "receipts" / f"{job['id']}.md"
        receipt.parent.mkdir(parents=True, exist_ok=True)
        receipt.write_text(
            "\n".join([
                f"# {job['id']}",
                "",
                reason,
                "",
                f"state: {state}",
                "READY_FOR_LIVE_TEST: no",
                "This receipt does not flip READY and does not seat a model.",
                "",
            ]),
            encoding="utf-8",
        )
        out = {
            "ok": state == "succeeded",
            "id": job["id"],
            "state": state,
            "result": result,
            "reason": reason,
            "needs_you": needs_you,
            "exit_code": exit_code,
            "reading_page": str(page),
            "launched": False,
        }
        if crash:
            out["crash"] = True
        return out
