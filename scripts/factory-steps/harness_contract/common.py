"""Shared locks, paths, and small JSON writers. Stdlib only. No GPU."""

from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path

ESTATE_CKSUM = "43770130 3391"
READY_LINE = "READY_FOR_LIVE_TEST: no"
SCHEMA_STATUS = "cell-one.harness-status.v0"
SCHEMA_CARD = "cell-one.harness-card.v0"
SCHEMA_QUOTA = "cell-one.harness-quota.v0"
SCHEMA_REGISTRY = "cell-one.harness-registry.v0"
SCHEMA_GAP = "cell-one.harness-gap.v0"

STEPS = (
    "gap",
    "decontaminate",
    "train",
    "export",
    "eval",
    "quant",
    "ask",
    "pack-device",
    "run",
    "improve",
    "improve-apply",
)
GPU_WORDS = ("forbid", "ask")
# A forbid on these is a refuse, not a silent CPU fallback.
GPU_REQUIRED = ("train", "eval", "run")
JOB_KEYS = ("id", "step", "config", "gpu", "max_minutes", "nas_out")
APPROVE_KEYS = ("decision", "by", "at")
STATES = (
    "queued",
    "needs-approval",
    "running",
    "succeeded",
    "failed",
    "refused",
)
TERMINAL = ("succeeded", "failed", "refused")
STATUS_MAX = 2048
CARD_MAX = 1024
CLOUD_BYTE_CAP = 1_000_000
ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$")
ANSWER_FIELDS = {
    "answer",
    "label",
    "target",
    "target_output",
    "assistant",
    "completion",
    "controller_action",
    "fault_id",
    "answerable_because",
}

# Exit codes. 0-29 stay with the existing factory steps.
EXIT_JOB = 30
EXIT_GPU_BUSY = 31
EXIT_GPU_CAP = 32
EXIT_GAP = 34
EXIT_DECONTAM = 35
EXIT_LOCAL = 36
EXIT_JUDGE = 37
EXIT_REWRITE = 38
EXIT_LAUNCH = 39
EXIT_ROUTE = 41
EXIT_MIX = 42
EXIT_BASE = 43
EXIT_PACK = 44
EXIT_IMPROVE = 45


def now_iso() -> str:
    return _dt.datetime.now().astimezone().isoformat(timespec="seconds")


def today() -> str:
    return now_iso()[:10]


def one_sentence(text: str, limit: int = 220) -> str:
    text = " ".join(str(text or "").split())
    if not text:
        text = "No reason was recorded."
    if len(text) > limit:
        text = text[: limit - 1].rstrip() + "."
    if text[-1] not in ".!?":
        text += "."
    return text


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def read_cksum(path: Path) -> str:
    out = subprocess.check_output(["cksum", str(path)], text=True).strip()
    crc, nbytes, *_ = out.split()
    return f"{crc} {nbytes}"


def read_json(path: Path) -> dict:
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise ValueError(f"not an object: {path}")
    return data


def read_jsonl(path: Path) -> list[dict]:
    rows = []
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            if line.strip():
                rows.append(json.loads(line))
    return rows


def write_json(path: Path, obj: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")


def append_jsonl(path: Path, obj: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    line = json.dumps(obj, separators=(",", ":"), ensure_ascii=False)
    with open(path, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def compact(obj: dict) -> str:
    return json.dumps(obj, separators=(",", ":"), ensure_ascii=False)


def fit_obj(obj: dict, limit: int) -> dict:
    """Shrink reason and artifact lists until the compact JSON fits."""
    obj = json.loads(json.dumps(obj))
    if len(compact(obj).encode()) <= limit:
        return obj
    reason = str(obj.get("reason", ""))
    arts = obj.get("artifacts")
    while len(compact(obj).encode()) > limit and (reason or arts):
        if isinstance(arts, list) and len(arts) > 1:
            obj["artifacts"] = arts[:1]
            arts = obj["artifacts"]
            continue
        if reason and len(reason) > 24:
            reason = reason[: max(24, len(reason) - 16)].rstrip()
            if not reason.endswith("."):
                reason += "."
            obj["reason"] = reason
            continue
        break
    if len(compact(obj).encode()) > limit:
        # Keep the contract fields. Drop the embedded extras.
        keep = {k: obj[k] for k in obj if k in (
            "schema", "id", "state", "step", "pid_alive", "gpu", "reason",
            "updated", "reading_page", "card", "result", "needs_you",
            "gpu_minutes", "crash",
        )}
        keep["reason"] = one_sentence(str(keep.get("reason", "truncated")), 80)
        obj = keep
    return obj


def guarded(path: Path) -> Path:
    """Refuse writes that would touch the locked estate, the live console, or decisions."""
    resolved = path.resolve()
    text = str(resolved)
    banned = (
        "/examples/estate.yaml",
        "/harness-v0-",
        "/.cell/decisions",
        "/.cell/decisions/",
    )
    for bit in banned:
        if bit in text or text.endswith(bit.rstrip("/")):
            raise ValueError(f"refuse:path: will not write {resolved}")
    return resolved


def ensure_dir(path: Path) -> Path:
    dest = guarded(path)
    dest.mkdir(parents=True, exist_ok=True)
    return dest


def file_bytes(path: Path) -> int:
    try:
        return path.stat().st_size
    except OSError:
        return 0


class Inbox:
    def __init__(self, root: Path):
        self.root = Path(root)

    def job_path(self, job_id: str) -> Path:
        return self.root / "jobs" / f"{job_id}.json"

    def status_path(self, job_id: str) -> Path:
        return self.root / "status" / f"{job_id}.json"

    def card_path(self, job_id: str) -> Path:
        return self.root / "status" / f"{job_id}.card.json"

    def approve_path(self, job_id: str) -> Path:
        return self.root / "approve" / f"{job_id}.json"

    @property
    def outbox(self) -> Path:
        return self.root / "outbox.jsonl"

    @property
    def quota(self) -> Path:
        return self.root / "quota.json"

    @property
    def registry(self) -> Path:
        return self.root / "registry.jsonl"

    @property
    def corrections(self) -> Path:
        return self.root / "corrections.jsonl"

    @property
    def turns(self) -> Path:
        return self.root / "cloud-turns.json"

    @property
    def quota_clear(self) -> Path:
        return self.root / "quota-clear.json"

    @property
    def pause_note(self) -> Path:
        return self.root / "pause-note.txt"

    def record_path(self, job_id: str) -> Path:
        return self.root / "records" / f"{job_id}.json"


def repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def estate_path(root: Path | None = None) -> Path:
    return (root or repo_root()) / "examples" / "estate.yaml"
