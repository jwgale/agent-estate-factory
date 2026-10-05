"""Append-only model registry. Rank 9. Never seats. ready stays no."""

from __future__ import annotations

from pathlib import Path

from .common import SCHEMA_REGISTRY, append_jsonl, one_sentence, sha256_file


def append(path: Path, record: dict) -> dict:
    line = {
        "schema": SCHEMA_REGISTRY,
        "id": str(record.get("id") or ""),
        "base": str(record.get("base") or ""),
        "recipe": str(record.get("recipe") or ""),
        "dataset_hash": _hash_or_unseen(record.get("dataset")),
        "exam_hash": _hash_or_unseen(record.get("exam")),
        "quant": str(record.get("quant") or ""),
        "gate": "pass" if record.get("gate") == "pass" else "fail",
        "reason": one_sentence(record.get("reason") or "No reason was recorded."),
        "nas_path": str(record.get("nas_path") or ""),
        "ready": "no",
    }
    if line["ready"] != "no":
        line["ready"] = "no"
    append_jsonl(path, line)
    return line


def _hash_or_unseen(value) -> str:
    if not value:
        return "not-seen"
    path = Path(str(value))
    if not path.is_file():
        return "not-seen"
    return sha256_file(path)
