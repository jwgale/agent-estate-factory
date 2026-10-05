#!/usr/bin/env python3
"""Launch `llamafactory-cli train|export` with recipe-supplied python (no edits to the llamafactory install).

  <venv>/bin/python lf_launch.py [--preload FILE.py ...] [--custom-templates FILE.py] [--json-patches JSON]
                                 --cli <venv>/bin/llamafactory-cli train|export recipe.yaml

factory_steps.py builds this command when a cell-one wrapper sets `<step>.custom_templates`, `<step>.preload` or
`export.json_patches` / `export.generation_config`. Steps:
  1. import each --preload file in order (runtime patches, e.g. enabling a capability flag on a model class);
  2. import --custom-templates (it calls llamafactory.data.template.register_template(...) at import time);
  3. check that the yaml's `template:` is registered (exit 2 otherwise, before any model load);
  4. refuse the multi-process paths (FORCE_TORCHRUN / USE_MCA / more than one visible device for train / USE_RAY /
     USE_V1): torchrun or ray workers are fresh interpreters that would not see the imports (exit 2);
  5. run the llamafactory-cli entry in this same process: sys.argv = [llamafactory-cli, step, yaml]; cli.main();
  6. export only, with --json-patches {"<file>.json": {"set": {...}, "drop": [...]}}: after a successful export, edit
     those JSON files in <export_dir> (e.g. generation_config.json set eos_token_id [2, 11] so runtimes stop on the chat
     template's end-of-turn token; config.json drop keys a converter misreads). Each change is printed to the log.
`--cli` is not executed; it stays in argv so the job's /proc cmdline still names llamafactory-cli (factory_steps
uses that, plus the recipe file name, to recognise its own PID before --status or the watchdog touches it).
"""
from __future__ import annotations

import argparse, hashlib, importlib.util, json, os, sys
from pathlib import Path


def _env_on(k: str) -> bool:
    return os.environ.get(k, "0").lower() in ("true", "y", "1")


def _refuse(msg: str):
    print(f"refuse:lf_launch: {msg}", file=sys.stderr, flush=True)
    sys.exit(2)


def _import_file(f: Path, kind: str):
    if f.suffix != ".py" or not f.is_file():
        _refuse(f"{kind} file not found or not .py: {f}")
    spec = importlib.util.spec_from_file_location(f"lf_{kind}_{hashlib.sha256(str(f).encode()).hexdigest()[:8]}", f)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    print(f"lf_launch: imported {kind} {f} (sha256 {hashlib.sha256(f.read_bytes()).hexdigest()})", flush=True)


def _load_yaml(p: str) -> dict:
    try:
        import yaml
        return yaml.safe_load(Path(p).read_text()) or {}
    except ImportError:
        from omegaconf import OmegaConf
        return OmegaConf.to_container(OmegaConf.load(p)) or {}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--preload", action="append", default=[], help="python file imported first (repeatable, in order)")
    ap.add_argument("--custom-templates", help="python file that calls register_template(...)")
    ap.add_argument("--json-patches", help='export: {"<file>.json": {"set": {...}, "drop": [...]}} applied in <export_dir>')
    ap.add_argument("--cli", default="llamafactory-cli", help="llamafactory-cli path (recorded in argv only)")
    ap.add_argument("step", choices=["train", "export"])
    ap.add_argument("config", help="LLaMA-Factory yaml")
    a = ap.parse_args(argv)
    patches = None
    if a.json_patches:
        if a.step != "export":
            _refuse("--json-patches applies to export only")
        try:
            patches = json.loads(a.json_patches)
        except json.JSONDecodeError as e:
            _refuse(f"--json-patches is not JSON: {e}")
        bad = [k for k, v in (patches.items() if isinstance(patches, dict) else [])
               if "/" in k or not k.endswith(".json") or not isinstance(v, dict) or set(v) - {"set", "drop"}
               or not isinstance(v.get("set", {}), dict) or not isinstance(v.get("drop", []), list)]
        if not isinstance(patches, dict) or not patches or bad:
            _refuse(f'--json-patches must be {{"<file>.json": {{"set": {{...}}, "drop": [...]}}}}; bad entries: {bad}')
    imports = [Path(p).resolve().name for p in a.preload] + ([Path(a.custom_templates).name] if a.custom_templates else [])
    for k in ("USE_V1", "USE_RAY", "USE_MCA", "FORCE_TORCHRUN"):
        if _env_on(k):
            _refuse(f"{k} is set: workers would not import {imports}; lf_launch is single-process only")
    try:
        from llamafactory.extras.misc import get_device_count
        n = int(get_device_count())
    except Exception:   # noqa: BLE001  (fake or partial installs in tests: assume one device)
        n = 1
    if a.step == "train" and n > 1:
        _refuse(f"{n} visible devices: llamafactory-cli would re-launch under torchrun, which would not import "
                f"{imports}; set CUDA_VISIBLE_DEVICES to one GPU")
    sys.dont_write_bytecode = True   # keep the cell's template / patch folders free of __pycache__
    for p in a.preload:
        _import_file(Path(p).resolve(), "preload")
    if a.custom_templates:
        _import_file(Path(a.custom_templates).resolve(), "custom_templates")
    cfg = _load_yaml(a.config)
    name = cfg.get("template")
    from llamafactory.data.template import TEMPLATES
    if name and name not in TEMPLATES:
        _refuse(f"template {name!r} from {a.config} is not registered (custom templates: {a.custom_templates})")
    print(f"lf_launch: template {name!r} registered; running llamafactory-cli {a.step} {a.config} in-process", flush=True)
    sys.argv = [a.cli, a.step, a.config]
    from llamafactory.cli import main as lf_main
    rc = lf_main() or 0
    if rc == 0 and patches:
        out = Path(os.path.expanduser(str(cfg.get("export_dir") or "")))
        if not out.is_dir():
            _refuse(f"export_dir {out} missing after export; json patches not applied")
        for name, p in patches.items():
            gp = out / name
            cur = json.loads(gp.read_text()) if gp.is_file() else {}
            sets, drops = p.get("set", {}), [k for k in p.get("drop", []) if k in cur]
            before = {k: cur.get(k) for k in sets}
            for k in drops:
                cur.pop(k)
            cur.update(sets)
            gp.write_text(json.dumps(cur, indent=2) + "\n")
            print(f"lf_launch: {gp} set {before} -> {sets}; dropped {drops}", flush=True)
    return rc


if __name__ == "__main__":
    sys.exit(main())
