"""Domain-free exam decontamination. Rank 5.

Lifts the existing gate (exact hash, 13-gram, MinHash) without the specialist
schema. Thresholds are not weakened.
"""

from __future__ import annotations

import sys
from pathlib import Path

from .common import ANSWER_FIELDS, EXIT_DECONTAM, one_sentence, read_jsonl, repo_root
from .fields import row_text

JACCARD = 0.5
NGRAM = 13


def _gate():
    scripts = repo_root() / "scripts"
    if str(scripts) not in sys.path:
        sys.path.insert(0, str(scripts))
    from lunatron_corpus.dedupe import Gate, build_index
    return Gate, build_index


def extract(row: dict, field: str) -> str:
    if field in ANSWER_FIELDS:
        raise ValueError(
            "refuse:decontaminate-field: the compared field must be input text, never the answer."
        )
    return row_text(row, field)


def check(
    train_path: Path,
    exam_path: Path,
    field: str,
    *,
    jaccard: float = JACCARD,
    train_field: str | None = None,
    exam_field: str | None = None,
) -> dict:
    train_field = train_field or field
    exam_field = exam_field or field
    if not str(train_field or "").strip() or not str(exam_field or "").strip():
        return {
            "ok": False,
            "hits": [],
            "reason": one_sentence(
                "No input field was named, so overlap was not checked."
            ),
            "exit_code": EXIT_DECONTAM,
            "params": {"jaccard": JACCARD, "ngram": NGRAM},
            "unverified": True,
        }
    if jaccard < JACCARD:
        return {
            "ok": False,
            "hits": [],
            "reason": one_sentence(
                "The decontaminate threshold was asked to go below 0.5. That request is refused."
            ),
            "exit_code": EXIT_DECONTAM,
            "params": {"jaccard": JACCARD, "ngram": NGRAM, "weakened": False},
            "unverified": False,
        }
    if field in ANSWER_FIELDS or train_field in ANSWER_FIELDS or exam_field in ANSWER_FIELDS:
        return {
            "ok": False,
            "hits": [],
            "reason": one_sentence(
                "The compared field is an answer field. Decontaminate hashes input text only."
            ),
            "exit_code": EXIT_DECONTAM,
            "params": {"jaccard": JACCARD, "ngram": NGRAM},
            "unverified": True,
        }
    if not train_path.is_file() or not exam_path.is_file():
        missing = "training file" if not train_path.is_file() else "exam file"
        return {
            "ok": False,
            "hits": [],
            "reason": one_sentence(
                f"The decontaminate step could not see the {missing}, so this is not a pass."
            ),
            "exit_code": EXIT_DECONTAM,
            "params": {"jaccard": JACCARD, "ngram": NGRAM},
            "unverified": True,
        }
    Gate, build_index = _gate()
    exam_rows = []
    exam_empty = 0
    for row in read_jsonl(exam_path):
        text = extract(row, exam_field)
        if not text.strip():
            exam_empty += 1
        exam_rows.append({
            "eval_file": exam_path.name,
            "id": row.get("id"),
            "text": text,
        })
    if not exam_rows:
        return {
            "ok": False,
            "hits": [],
            "reason": one_sentence("The exam file had no rows, so overlap was not checked."),
            "exit_code": EXIT_DECONTAM,
            "params": {"jaccard": JACCARD, "ngram": NGRAM},
            "unverified": True,
        }
    if exam_empty == len(exam_rows):
        return {
            "ok": False,
            "hits": [],
            "reason": one_sentence(
                "The exam text was empty, so nothing was compared. This is not a pass."
            ),
            "exit_code": EXIT_DECONTAM,
            "params": {"jaccard": JACCARD, "ngram": NGRAM, "train_field": train_field, "exam_field": exam_field},
            "unverified": True,
            "output_name": "decontaminate.json",
        }
    train_rows = read_jsonl(train_path)
    train_texts = [extract(row, train_field) for row in train_rows]
    if not any(text.strip() for text in train_texts):
        return {
            "ok": False,
            "hits": [],
            "reason": one_sentence(
                "The training text was empty, so nothing was compared. This is not a pass."
            ),
            "exit_code": EXIT_DECONTAM,
            "params": {"jaccard": JACCARD, "ngram": NGRAM, "train_field": train_field, "exam_field": exam_field},
            "unverified": True,
            "output_name": "decontaminate.json",
        }
    index = build_index(exam_rows, template_df=5)
    gate = Gate(index, jaccard=JACCARD, min_shared_13=1)
    hits = []
    for n, text in enumerate(train_texts):
        found = gate.check_text(text)
        if found:
            hits.append({"row": n, "id": train_rows[n].get("id"), "hits": found})
    ok = not hits
    named = " Wrote decontaminate.json."
    if ok:
        reason = "No training row overlapped the exam on the input field." + named
        exit_code = 0
    else:
        kind = hits[0]["hits"][0].get("kind", "overlap")
        reason = (
            f"Training row {hits[0]['row']} overlaps the exam ({kind}). "
            "The row stays refused. The threshold was not lowered."
            + named
        )
        exit_code = EXIT_DECONTAM
    return {
        "ok": ok,
        "hits": hits[:20],
        "hit_count": len(hits),
        "reason": one_sentence(reason),
        "exit_code": exit_code,
        "params": {
            "jaccard": JACCARD,
            "ngram": NGRAM,
            "shingle": 5,
            "perms": 128,
            "train_field": train_field,
            "exam_field": exam_field,
        },
        "unverified": False,
        "output_name": "decontaminate.json",
    }
