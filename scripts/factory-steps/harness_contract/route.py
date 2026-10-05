"""Route table for ask. Rank 13. Data, not a planner. No cloud fallback."""

from __future__ import annotations

from pathlib import Path

from .common import one_sentence, read_json


def lookup(table_path: Path, task: str) -> dict | None:
    if not table_path.is_file():
        return None
    body = read_json(table_path)
    for row in body.get("rows") or []:
        if isinstance(row, dict) and row.get("task") == task:
            return {
                "task": task,
                "tag": str(row.get("tag") or ""),
                "registry_id": str(row.get("registry_id") or ""),
                "reason": one_sentence(row.get("reason") or "The table names this seat."),
            }
    return None


def outcome(*, row: dict | None, called: bool, seat_text: str | None = None) -> dict:
    if row is None:
        return {
            "ok": False,
            "state": "refused",
            "result": "refused",
            "needs_you": False,
            "reason": one_sentence(
                "The route table has no row for this task, so no model was called and no cloud model was used."
            ),
        }
    tag = row["tag"] or "unnamed"
    if not called:
        return {
            "ok": False,
            "state": "refused",
            "result": "refused",
            "needs_you": False,
            "tag": tag,
            "reason": one_sentence(
                f"The table names {tag}. The seat was not called. No cloud model was used."
            ),
        }
    brief = " ".join(str(seat_text or "").split())
    if len(brief) > 80:
        brief = brief[:77] + "..."
    return {
        "ok": True,
        "state": "succeeded",
        "result": "pass",
        "needs_you": False,
        "tag": tag,
        "reason": one_sentence(
            f"The local seat {tag} answered. {row['reason']}"
            + (f" Reply note: {brief}" if brief else "")
        ),
    }
