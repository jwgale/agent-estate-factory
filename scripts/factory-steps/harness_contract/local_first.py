"""Local-first refuse. Rank 6. A cloud session may read only the small files."""

from __future__ import annotations

from pathlib import Path

from .common import CLOUD_BYTE_CAP, EXIT_LOCAL, file_bytes, one_sentence

CLOUD_NAME_BITS = ("manifest", "cleaned", "batch-report", "batch_report")


def cloud_read(path: Path, *, kind: str, job_log: Path | None = None) -> dict:
    """kind is status, card, log, or other. other is always a refuse for a cloud caller."""
    path = Path(path)
    if kind == "log":
        return _log_tail(job_log or path)
    if kind not in ("status", "card"):
        return _no(f"A cloud session may not open {path.name}. Allowed reads are the status file, the card, and the last 30 error lines of that job's log.")
    name = path.name.lower()
    if any(bit in name for bit in CLOUD_NAME_BITS):
        return _no(f"A cloud session may not open {path.name}.")
    size = file_bytes(path)
    if size >= CLOUD_BYTE_CAP:
        return _no(
            f"{path.name} is {size} bytes. A cloud session does not open a multi-megabyte file."
        )
    if not path.is_file():
        return _no(f"The cloud read could not see {path.name}.")
    text = path.read_text(encoding="utf-8")
    return {"ok": True, "text": text, "reason": one_sentence(f"Read {path.name}."), "exit_code": 0}


def assert_local_config(path: Path, caller: str) -> str | None:
    if caller != "cloud":
        return None
    size = file_bytes(path)
    name = path.name.lower()
    if size >= CLOUD_BYTE_CAP or any(bit in name for bit in CLOUD_NAME_BITS):
        return one_sentence(
            f"A cloud session cannot open {path.name} as part of this job. "
            "The file was not read."
        )
    return None


def _log_tail(path: Path) -> dict:
    if not path.is_file():
        return _no("The job log was not there, so no error lines were returned.")
    if file_bytes(path) >= 50_000_000:
        return _no("The job log is too large to open from a cloud session.")
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    errors = [
        line for line in lines
        if any(bit in line.lower() for bit in ("error", "traceback", "refuse", "fail"))
    ]
    tail = errors[-30:]
    return {
        "ok": True,
        "text": "\n".join(tail),
        "reason": one_sentence(f"Returned {len(tail)} error lines, not the log body."),
        "exit_code": 0,
    }


def _no(reason: str) -> dict:
    return {"ok": False, "text": "", "reason": one_sentence(reason), "exit_code": EXIT_LOCAL}
