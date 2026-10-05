"""Local digest and usage guard. Rank 2.

Rewrites quota.json from status records. Does not SSH. Does not open manifests.
A cloud status check increments cloud_helper_turns_today. If that count is
above zero, new jobs wait.
"""

from __future__ import annotations

from .common import SCHEMA_QUOTA, Inbox, now_iso, one_sentence, read_json, today, write_json


def note_cloud_status(inbox: Inbox) -> int:
    inbox.root.mkdir(parents=True, exist_ok=True)
    day = today()
    if inbox.turns.is_file():
        body = read_json(inbox.turns)
    else:
        body = {"date": day, "count": 0}
    if body.get("date") != day:
        body = {"date": day, "count": 0}
    body["count"] = int(body.get("count") or 0) + 1
    write_json(inbox.turns, body)
    return body["count"]


def turns_today(inbox: Inbox) -> int:
    if not inbox.turns.is_file():
        return 0
    body = read_json(inbox.turns)
    if body.get("date") != today():
        return 0
    return int(body.get("count") or 0)


def clear_quota(inbox: Inbox, decision: str, by: str, at: str) -> dict:
    if decision != "yes":
        return {"ok": False, "reason": one_sentence("A quota clear needs decision yes. This is not a clear.")}
    write_json(inbox.quota_clear, {"decision": "yes", "by": by, "at": at})
    write_json(inbox.turns, {"date": today(), "count": 0})
    if inbox.pause_note.exists():
        inbox.pause_note.unlink()
    return rewrite(inbox)


def paused(inbox: Inbox) -> bool:
    if not inbox.quota.is_file():
        return False
    body = read_json(inbox.quota)
    return bool(body.get("pause"))


def rewrite(inbox: Inbox) -> dict:
    inbox.root.mkdir(parents=True, exist_ok=True)
    day = today()
    gpu_minutes = 0.0
    running = 0
    records = inbox.root / "records"
    if records.is_dir():
        for path in sorted(records.glob("*.json")):
            try:
                rec = read_json(path)
            except ValueError:
                continue
            if rec.get("ended") == day:
                gpu_minutes += float(rec.get("gpu_minutes") or 0)
            if rec.get("state") == "running":
                running += 1
    status_dir = inbox.root / "status"
    if status_dir.is_dir():
        running = 0
        for path in status_dir.glob("*.json"):
            if path.name.endswith(".card.json"):
                continue
            try:
                st = read_json(path)
            except ValueError:
                continue
            if st.get("state") == "running":
                running += 1
    cloud = turns_today(inbox)
    pause = cloud > 0
    note = ""
    if pause:
        note = (
            f"cloud_helper_turns_today is {cloud}. "
            "A status check spent a cloud-helper turn. New jobs wait until a person clears it."
        )
        inbox.pause_note.write_text(note + "\n", encoding="utf-8")
    elif inbox.pause_note.exists():
        inbox.pause_note.unlink()
    body = {
        "schema": SCHEMA_QUOTA,
        "gpu_minutes_today": round(gpu_minutes, 3),
        "jobs_running": running,
        "cloud_helper_turns_today": cloud,
        "pause": pause,
        "pause_note": note,
        "updated": now_iso(),
    }
    write_json(inbox.quota, body)
    return body
