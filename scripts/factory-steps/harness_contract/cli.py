#!/usr/bin/env python3
"""cell-job: write a harness job file, or run one job once.

Does not loop. A train starts only when the config sets allow_launch, an approve
file says yes, and the GPU probe reports idle. The default probe is a read-only
nvidia-smi query. It does not start a train by itself.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from harness_contract.common import estate_path, read_cksum, read_json, repo_root, write_json  # noqa: E402
from harness_contract.prepare_cards import write_preference, write_refusal  # noqa: E402
from harness_contract.runner import Harness  # noqa: E402

ESTATE_CKSUM = "43770130 3391"


def _probe():
    mode = os.environ.get("HARNESS_GPU_PROBE", "nvidia-smi")
    if mode == "busy":
        holder = os.environ.get("HARNESS_GPU_HOLDER", "other-job")
        return lambda: {"state": "busy", "holder": holder, "seen": True, "source": "test"}
    if mode == "idle":
        return lambda: {"state": "idle", "holder": None, "seen": True, "source": "test"}
    if mode == "skip":
        return lambda: {"state": "unknown", "holder": None, "seen": False, "source": "skip"}
    from harness_contract.gpu_probe import probe_nvidia_smi
    return probe_nvidia_smi


def _harness(args) -> Harness:
    cell = Path(args.cell_root)
    return Harness(args.inbox, cell, gpu_probe=_probe(), caller=args.caller)


def _emit(result: dict) -> int:
    print(json.dumps({k: result[k] for k in result if k in (
        "ok", "id", "state", "reason", "exit_code", "needs_you", "result", "reading_page",
    )}, ensure_ascii=False))
    return int(result.get("exit_code") or 0)


def main(argv=None) -> int:
    root = repo_root()
    estate = estate_path(root)
    before = read_cksum(estate) if estate.is_file() else None
    if before and before != ESTATE_CKSUM:
        print(f"refuse:estate: cksum is {before}, expected {ESTATE_CKSUM}", file=sys.stderr)
        return 1
    ap = argparse.ArgumentParser(prog="cell-job")
    ap.add_argument("--inbox", default=os.environ.get("HARNESS_INBOX", str(Path.home() / "harness-inbox")))
    ap.add_argument("--cell-root", default=os.environ.get("HARNESS_CELL_ROOT", str(root / ".cell")))
    ap.add_argument("--caller", default=os.environ.get("HARNESS_CALLER", "local"))
    sub = ap.add_subparsers(dest="cmd", required=True)

    sub.add_parser("submit")
    run = sub.add_parser("run")
    run.add_argument("id")
    st = sub.add_parser("status")
    st.add_argument("id")
    st.add_argument("--via", default="local", choices=("local", "cloud"))
    sub.add_parser("digest")
    clr = sub.add_parser("clear-quota")
    clr.add_argument("--by", required=True)
    clr.add_argument("--at", required=True)
    appr = sub.add_parser("approve")
    appr.add_argument("id")
    prep = sub.add_parser("prepare")
    prep.add_argument("kind", choices=("preference", "refusal"))
    prep.add_argument("--out", required=True)
    prep.add_argument("--config", required=True)
    bp = sub.add_parser("base-pick")
    bp.add_argument("--id", required=True)
    bp.add_argument("--config", required=True)

    args = ap.parse_args(argv)
    os.environ["CUDA_VISIBLE_DEVICES"] = ""
    h = _harness(args)
    if args.cmd == "submit":
        raw = json.load(sys.stdin)
        result = h.submit(raw)
        if result.get("id"):
            print(result["id"])
        else:
            print(result.get("reason", "refused"), file=sys.stderr)
        code = int(result.get("exit_code") or 0)
    elif args.cmd == "run":
        code = _emit(h.run(args.id))
    elif args.cmd == "status":
        result = h.status(args.id, via=args.via)
        print(json.dumps(result.get("status", result), ensure_ascii=False))
        code = int(result.get("exit_code") or 0)
    elif args.cmd == "digest":
        print(json.dumps(h.digest(), ensure_ascii=False))
        code = 0
    elif args.cmd == "clear-quota":
        from harness_contract.digest import clear_quota
        print(json.dumps(clear_quota(h.inbox, "yes", args.by, args.at), ensure_ascii=False))
        code = 0
    elif args.cmd == "approve":
        raw = json.load(sys.stdin)
        dest = h.inbox.approve_path(args.id)
        write_json(dest, {"decision": raw.get("decision"), "by": raw.get("by"), "at": raw.get("at")})
        print(args.id)
        code = 0
    elif args.cmd == "prepare":
        cfg = read_json(Path(args.config))
        out = Path(args.out)
        if args.kind == "preference":
            card = write_preference(out, list(cfg.get("rows") or []), stage=str(cfg.get("stage") or "dpo"))
        else:
            card = write_refusal(out, list(cfg.get("rows") or []), marker=str(cfg.get("marker") or ""))
        print(card["reason"])
        code = 0 if card.get("auto_train") is False and card.get("train_invoked") is False else 1
    else:
        cfg = read_json(Path(args.config))
        code = _emit(h.base_pick_run(args.id, cfg, scorer=None))
    if before:
        after = read_cksum(estate)
        if after != before:
            print("refuse:estate: examples/estate.yaml cksum changed", file=sys.stderr)
            return 1
    return code


if __name__ == "__main__":
    raise SystemExit(main())
