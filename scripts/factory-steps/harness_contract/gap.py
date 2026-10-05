"""Coverage gap report that can say no. Domain-free.

Also opens the exam file when a path is given, finds a contrast phrase when
the config does not name one, and reports rows outside the label list.
"""

from __future__ import annotations

from pathlib import Path

from .common import EXIT_GAP, EXIT_MIX, SCHEMA_GAP, one_sentence, read_jsonl, write_json
from .fields import find_contrast, is_dose_ask, row_label, row_text


def mix_reds(task_path: Path | None, refusal_path: Path | None, marker: str | None, target_field: str = "target") -> list[dict]:
    """A refusal row in the task set, or a task line in the refusal set, is a red row."""
    reds = []
    if not marker:
        return reds
    if task_path and task_path.is_file():
        for i, row in enumerate(read_jsonl(task_path)):
            if marker in _target(row, target_field):
                reds.append({
                    "kind": "mix",
                    "sentence": (
                        f"Row {i} in the task set has the refusal marker, "
                        "so a refusal row is mixed into the task set."
                    ),
                })
                break
    if refusal_path and refusal_path.is_file():
        for i, row in enumerate(read_jsonl(refusal_path)):
            if marker not in _target(row, target_field):
                reds.append({
                    "kind": "mix",
                    "sentence": (
                        f"Row {i} in the refusal set does not have the refusal marker, "
                        "so a task line is mixed into the refusal set."
                    ),
                })
                break
    return reds


def _target(row: dict, field: str) -> str:
    val = row.get(field)
    if isinstance(val, str):
        return val
    messages = row.get("messages")
    if isinstance(messages, list):
        parts = []
        for m in messages:
            if isinstance(m, dict) and m.get("role") == "assistant":
                parts.append(str(m.get("content") or ""))
        return "\n".join(parts)
    return ""


def report(
    dataset: Path,
    labels: list[str],
    label_field: str,
    exam_shape: str,
    shape_field: str,
    contrast_marker: str | None = None,
    mix: dict | None = None,
    exam: Path | None = None,
) -> dict:
    if not dataset.is_file():
        return _fail("The gap step could not see the dataset, so this is not a pass.", unverified=True)
    if not labels:
        return _fail("The gap step was not given a label list, so this is not a pass.", unverified=True)
    rows = read_jsonl(dataset)
    counts = {str(label): 0 for label in labels}
    unknown = 0
    outside_names: dict[str, int] = {}
    shape_hits = 0
    contrast_hits = 0
    dose_rows = []
    texts = []
    for i, row in enumerate(rows):
        lab = row_label(row, label_field)
        if lab is not None and str(lab) in counts:
            counts[str(lab)] += 1
        else:
            unknown += 1
            key = str(lab) if lab else "(no label)"
            outside_names[key] = outside_names.get(key, 0) + 1
        text = row_text(row, shape_field)
        texts.append(text)
        if exam_shape and exam_shape in text:
            shape_hits += 1
        if contrast_marker and contrast_marker in text:
            contrast_hits += 1
        if is_dose_ask(text):
            dose_rows.append(i)
    reds = []
    for label, n in counts.items():
        if n == 0:
            reds.append({
                "kind": "zero",
                "label": label,
                "sentence": f"Class {label} has zero training rows, so the miss is expected.",
            })
    exam_meta = _check_exam(exam, exam_shape)
    if exam is not None and not exam_meta["opened"]:
        reds.extend(exam_meta["reds"])
    elif exam_meta["questions"]:
        reds.extend(exam_meta["reds"])
    elif not exam_shape:
        reds.append({
            "kind": "shape",
            "sentence": "No exam shape was given, so this run could not check the answer line.",
        })
    elif shape_hits == 0:
        reds.append({
            "kind": "shape",
            "sentence": "The exam shape does not appear in the dataset, so a train cannot start.",
        })
    contrast_found = None
    if _tied(counts):
        if contrast_marker:
            if contrast_hits == 0:
                reds.append({
                    "kind": "tie",
                    "sentence": "The label split is a tie with no contrast, so this is a red row, not a pass.",
                })
        else:
            contrast_found = find_contrast(texts)
            if not contrast_found:
                reds.append({
                    "kind": "tie",
                    "sentence": (
                        "The label split is a tie and this step did not find a contrast phrase. "
                        "Name one in the config, or this stays a red row."
                    ),
                })
    if mix:
        reds.extend(mix_reds(
            Path(mix["task_set"]) if mix.get("task_set") else None,
            Path(mix["refusal_set"]) if mix.get("refusal_set") else None,
            mix.get("refusal_marker"),
            mix.get("target_field") or "target",
        ))
    notes = _notes(unknown, outside_names, dose_rows, mix)
    ok = not reds
    if ok:
        reason = "Every scored class has rows, the exam shape appears, and the split is not a tie without contrast."
        if contrast_found:
            reason = (
                "Every scored class has rows. This step found the contrast phrase "
                f"\"{contrast_found}\" without a hand-named marker."
            )
        if notes:
            reason = reason + " " + " ".join(notes)
        exit_code = 0
    else:
        reason = reds[0]["sentence"]
        exit_code = EXIT_MIX if any(r["kind"] == "mix" for r in reds) else EXIT_GAP
    return {
        "schema": SCHEMA_GAP,
        "ok": ok,
        "reds": reds,
        "notes": notes,
        "counts": counts,
        "unknown": unknown,
        "outside_labels": outside_names,
        "dose_asks": len(dose_rows),
        "dose_ask_rows": dose_rows[:20],
        "shape_hits": shape_hits,
        "contrast_hits": contrast_hits,
        "contrast_found": contrast_found,
        "rows": len(rows),
        "exam_opened": exam_meta["opened"],
        "exam_questions": exam_meta["questions"],
        "exam_real_shape": exam_meta["real_shape"],
        "reason": one_sentence(reason),
        "exit_code": exit_code,
        "unverified": bool(exam_meta.get("unverified")),
    }


def _notes(unknown: int, outside_names: dict, dose_rows: list, mix: dict | None) -> list[str]:
    notes = []
    if unknown:
        sit = "row sits" if unknown == 1 else "rows sit"
        preview = ", ".join(f"{name} {n}" for name, n in list(outside_names.items())[:8])
        notes.append(
            f"{unknown} {sit} outside the label list ({preview}). "
            "They are reported and do not fail this step."
        )
    has_refusal = bool(mix and mix.get("refusal_set"))
    if dose_rows and not has_refusal:
        n = len(dose_rows)
        word = "row asks" if n == 1 else "rows ask"
        notes.append(
            f"{n} {word} for a dose and no refusal set was named, "
            "so those asks were not checked against a refusal file."
        )
    return notes


def _check_exam(exam: Path | None, typed: str) -> dict:
    if exam is None:
        return {"opened": False, "questions": 0, "real_shape": "", "reds": [], "unverified": False}
    if not exam.is_file():
        return {
            "opened": False,
            "questions": 0,
            "real_shape": "",
            "unverified": True,
            "reds": [{
                "kind": "shape",
                "sentence": "The gap step could not open the exam file, so the exam shape was not checked.",
            }],
        }
    questions = []
    opened_rows = 0
    for row in read_jsonl(exam):
        opened_rows += 1
        question = row.get("question")
        if isinstance(question, str) and question.strip():
            questions.append(question)
    if not questions:
        if opened_rows == 0:
            return {
                "opened": True,
                "questions": 0,
                "real_shape": "",
                "unverified": True,
                "reds": [{
                    "kind": "shape",
                    "sentence": "The exam file opened but had no questions, so the exam shape was not checked.",
                }],
            }
        return {
            "opened": True,
            "questions": 0,
            "real_shape": "",
            "reds": [],
            "unverified": False,
        }
    clinic = sum(1 for question in questions if "?" in question)
    real = "clinic questions" if clinic else "questions"
    reds = []
    if typed and not any(typed in question for question in questions):
        reds.append({
            "kind": "shape",
            "sentence": (
                f"The typed exam shape does not appear in the exam file. "
                f"The exam has {len(questions)} {real}. That phrase is not the exam shape."
            ),
        })
    return {
        "opened": True,
        "questions": len(questions),
        "real_shape": real,
        "reds": reds,
        "unverified": False,
    }


def _tied(counts: dict) -> bool:
    positive = [n for n in counts.values() if n > 0]
    if len(positive) < 2:
        return False
    if sum(counts.values()) <= 0:
        return False
    top = sorted(positive, reverse=True)
    return top[0] == top[1]


def _fail(reason: str, unverified: bool) -> dict:
    return {
        "schema": SCHEMA_GAP,
        "ok": False,
        "reds": [{"kind": "unseen", "sentence": reason}],
        "notes": [],
        "counts": {},
        "unknown": 0,
        "outside_labels": {},
        "dose_asks": 0,
        "dose_ask_rows": [],
        "shape_hits": 0,
        "contrast_hits": 0,
        "contrast_found": None,
        "rows": 0,
        "exam_opened": False,
        "exam_questions": 0,
        "exam_real_shape": "",
        "reason": one_sentence(reason),
        "exit_code": EXIT_GAP,
        "unverified": unverified,
    }


def write_report(path: Path, body: dict) -> None:
    write_json(path, body)
