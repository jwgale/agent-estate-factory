"""Failure inbox. Rank 11. One corrections file. No new version."""

from __future__ import annotations

from pathlib import Path

from .common import append_jsonl, now_iso, one_sentence


def append_miss(path: Path, *, job_id: str, expected_shape: str, reason: str) -> Path:
    """Append to the one corrections file. Does not open a sibling version."""
    if path.name not in ("corrections.jsonl",):
        raise ValueError(
            "refuse:rewrite: misses append to corrections.jsonl only. A new version was not opened."
        )
    append_jsonl(path, {
        "id": job_id,
        "expected_shape": expected_shape,
        "reason": one_sentence(reason),
        "at": now_iso(),
    })
    return path
