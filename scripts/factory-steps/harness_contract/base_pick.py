"""Base-pick probe on cached models only. Rank 10. No download. No live load."""

from __future__ import annotations

from pathlib import Path

from .common import one_sentence
from .reading import write_page


def probe(models: list[str], exam: Path, marker: str, scorer, minute_cap) -> dict:
    """scorer(path) -> True if that cached file wrote the marker. Missing paths are not fetched."""
    rows = []
    wrote = []
    missed = []
    unseen = []
    for raw in models:
        path = Path(raw).expanduser()
        if not path.is_file():
            unseen.append(path.name or str(path))
            rows.append(f"{path.name or path}: not on disk. It was not downloaded.")
            continue
        try:
            hit = bool(scorer(path))
        except Exception as exc:  # a stand-in failure is a miss, not a download
            hit = False
            rows.append(f"{path.name}: the stand-in scorer failed ({exc.__class__.__name__}).")
            missed.append(path.name)
            continue
        if hit:
            wrote.append(path.name)
            rows.append(f"{path.name}: wrote the answer marker.")
        else:
            missed.append(path.name)
            rows.append(f"{path.name}: did not write the answer marker.")
    if not models:
        reason = "No cached model paths were given, so this is not a comparison."
        ok = False
    elif not exam.is_file():
        reason = "The exam file was not on disk, so this comparison did not check it."
        ok = False
    elif unseen and not wrote and not missed:
        reason = "None of the named models were on disk. Nothing was downloaded."
        ok = False
    elif wrote and missed:
        reason = f"{wrote[0]} wrote the answer marker; {missed[0]} did not."
        ok = True
    elif wrote:
        reason = f"{wrote[0]} wrote the answer marker."
        ok = True
    else:
        reason = "No cached model wrote the answer marker."
        ok = False
    return {
        "ok": ok,
        "reason": one_sentence(reason),
        "rows": rows,
        "wrote": wrote,
        "missed": missed,
        "unseen": unseen,
        "minute_cap": minute_cap,
        "downloaded": False,
        "exam_seen": exam.is_file(),
    }


def write_comparison(path: Path, body: dict, *, needs_you: bool) -> str:
    tried = "Compared cached model files already on disk against the exam marker. No download."
    why = " ".join(body["rows"]) if body["rows"] else body["reason"]
    nxt = (
        "A full train still needs its own approve file. "
        "This page is the short comparison, not a leaderboard."
    )
    unverified = None
    if body["unseen"] or not body["exam_seen"]:
        unverified = body["reason"] if not body["exam_seen"] else (
            "This run could not see every named model, so those files were not checked. Nothing was downloaded."
        )
    return write_page(
        path,
        reason=body["reason"],
        needs_you=needs_you,
        tried=tried,
        why=why,
        nxt=nxt,
        unverified=unverified,
    )


def page_missing(path: Path | None) -> bool:
    return path is None or not path.is_file() or path.stat().st_size == 0
