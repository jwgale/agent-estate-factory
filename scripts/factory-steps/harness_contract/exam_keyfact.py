"""Key-fact exam grade. Sits beside the overlap scorer. Does not start a server.

Exam rows may carry key_facts, must_not_say, and refusal_ok. When key_facts
is missing, draft facts are taken from answerable_because and marked derived.
Words that also sit in the question are not facts, so echoing the question
earns nothing. A refusal earns credit only when refusal_ok is true and the
reply does not assert a must_not_say phrase. Overlap is always reported
alongside. An optional judge posts a fixed rubric to a caller-supplied ask.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

from .common import one_sentence, read_jsonl
from .exam_score import covers
from .fields import row_text

_TOKEN = re.compile(r"[a-z0-9]+")
_SPLIT = re.compile(r"[.;]")
_REFUSAL = re.compile(
    r"\b(?:i don['’]t have|i do not have|i can['’]t|i cannot|i can not|"
    r"not in my context|don['’]t have access|do not have access|"
    r"cannot see|can['’]t see|i won['’]t|i will not|unable to|"
    r"no access to|not able to|i am not able|i'm not able)\b",
    re.I,
)
_STOP = frozenset(
    """
    that this with from have been were they them their what when where which
    than then into over also only such some says said does about after before
    there these those would could should being other more most very just like
    make made your will shall each both same
    """.split()
)
RUBRIC = (
    "Grade one reply against the key facts. Echoing the question is not a hit. "
    "A refusal is a hit only when refusal_ok is true and the reply does not assert "
    "a must_not_say phrase. Otherwise the reply must state the key facts. "
    'Answer with one JSON object only: {"hit": true or false, "reason": "one sentence"}.'
)
_RATIO = 0.34


def content_tokens(text: str) -> list[str]:
    return [tok for tok in _TOKEN.findall((text or "").lower()) if len(tok) > 3 and tok not in _STOP]


def novel_tokens(expected: str, question: str) -> list[str]:
    question_tokens = set(content_tokens(question))
    out = []
    for tok in content_tokens(expected):
        if tok not in question_tokens and tok not in out:
            out.append(tok)
    return out


def is_refusal(reply: str) -> bool:
    return bool(_REFUSAL.search(reply or ""))


def draft_facts(expected: str, question: str) -> list[dict]:
    """Draft facts from the expected line. Marked derived by the caller."""
    sentences = [part.strip() for part in _SPLIT.split(expected or "") if part.strip()]
    facts = []
    for sentence in sentences:
        tokens = novel_tokens(sentence, question)
        if tokens:
            facts.append({"text": sentence, "tokens": tokens, "derived": True, "echo_only": False})
    if facts:
        return facts
    tokens = novel_tokens(expected, question)
    if not tokens:
        return []
    return [{
        "text": (expected or "").strip(),
        "tokens": tokens,
        "derived": True,
        "echo_only": False,
    }]


def explicit_facts(raw, question: str) -> list[dict]:
    if not isinstance(raw, list):
        return []
    facts = []
    for item in raw:
        if not isinstance(item, str) or not item.strip():
            continue
        tokens = novel_tokens(item, question)
        facts.append({
            "text": item.strip(),
            "tokens": tokens or content_tokens(item),
            "derived": False,
            "echo_only": not tokens,
        })
    return facts


def row_facts(row: dict, question: str, expected: str) -> tuple[list[dict], bool]:
    explicit = explicit_facts(row.get("key_facts"), question)
    if explicit:
        return explicit, False
    return draft_facts(expected, question), True


def _covered(reply: str, tokens: list[str]) -> bool:
    if not tokens:
        return False
    got = set(content_tokens(reply))
    hit = sum(1 for tok in tokens if tok in got)
    return hit >= 1 and (hit / len(tokens)) >= _RATIO


def fact_hit(reply: str, fact: dict, question: str) -> bool:
    if fact.get("echo_only"):
        phrase = fact["text"].lower()
        if phrase not in (reply or "").lower():
            return False
        question_tokens = set(content_tokens(question))
        return any(tok not in question_tokens for tok in content_tokens(reply))
    return _covered(reply, fact.get("tokens") or [])


def _phrases(row: dict) -> list[str]:
    raw = row.get("must_not_say")
    if not isinstance(raw, list):
        return []
    return [item.strip() for item in raw if isinstance(item, str) and item.strip()]


def forbidden(reply: str, phrases: list[str]) -> str | None:
    text = (reply or "").lower()
    for phrase in phrases:
        if phrase.lower() in text:
            return phrase
    return None


def grade_reply(row: dict, reply: str, *, question: str = "", expected: str = "") -> dict:
    question = question or _question(row)
    expected = expected if expected != "" else _expected(row)
    facts, derived = row_facts(row, question, expected)
    refusal_ok = bool(row.get("refusal_ok"))
    refusing = is_refusal(reply)
    ban = forbidden(reply, _phrases(row))
    covered_n = sum(1 for fact in facts if fact_hit(reply, fact, question))
    if ban:
        hit, reason = False, "must-not-say"
    elif refusal_ok and refusing:
        hit, reason = True, "refusal"
    elif facts and covered_n == len(facts):
        hit, reason = True, "key-fact"
    elif not facts:
        hit, reason = False, "no key facts"
    else:
        hit, reason = False, "miss"
    return {
        "hit": hit,
        "reason": reason,
        "derived": derived,
        "key_facts": [fact["text"] for fact in facts],
        "facts_required": len(facts),
        "facts_covered": covered_n,
        "refusal": refusing,
        "refusal_ok": refusal_ok,
    }


def _question(row: dict, field: str = "question") -> str:
    question = row.get(field)
    if isinstance(question, str) and question.strip():
        return question
    return row_text(row, field)


def _expected(row: dict, field: str = "answerable_because") -> str:
    expected = row.get(field)
    return expected if isinstance(expected, str) else ""


def _parse_judge(text: str) -> dict:
    raw = (text or "").strip()
    data = None
    try:
        data = json.loads(raw)
    except json.JSONDecodeError:
        start = raw.find("{")
        end = raw.rfind("}")
        if start >= 0 and end > start:
            try:
                data = json.loads(raw[start:end + 1])
            except json.JSONDecodeError:
                data = None
    if not isinstance(data, dict) or "hit" not in data:
        return {"hit": False, "reason": one_sentence("The judge reply was not a grade.")}
    return {"hit": bool(data.get("hit")), "reason": one_sentence(str(data.get("reason") or "The judge gave no reason."))}


def judge_reply(row: dict, reply: str, graded: dict, ask) -> dict:
    payload = {
        "question": _question(row),
        "reply": reply,
        "key_facts": graded.get("key_facts") or [],
        "must_not_say": _phrases(row),
        "refusal_ok": bool(row.get("refusal_ok")),
        "facts_derived": bool(graded.get("derived")),
    }
    text = ask(RUBRIC + "\n" + json.dumps(payload, ensure_ascii=False))
    if not isinstance(text, str):
        text = "" if text is None else str(text)
    return _parse_judge(text)


def score_pairs(pairs: list[tuple[dict, str]], *, judge=None, use_judge: bool = False) -> dict:
    items = []
    judge_failed = None
    for row, reply in pairs:
        question = _question(row)
        expected = _expected(row)
        graded = grade_reply(row, reply, question=question, expected=expected)
        overlap_hit = covers(reply, expected)
        item = {
            "id": row.get("id"),
            "hit": graded["hit"],
            "reason": graded["reason"],
            "derived": graded["derived"],
            "key_facts": graded["key_facts"],
            "overlap_hit": overlap_hit,
            "keyfact_hit": graded["hit"],
            "refusal": graded["refusal"],
            "reply": reply,
        }
        if use_judge:
            if judge is None:
                judge_failed = "No judge was attached, so this is not a judge score."
                break
            try:
                judged = judge_reply(row, reply, graded, judge)
            except (OSError, TimeoutError, ConnectionError) as exc:
                judge_failed = f"The judge did not answer ({exc.__class__.__name__}), so this is not a judge score."
                break
            item["judge_hit"] = judged["hit"]
            item["judge_reason"] = judged["reason"]
            item["hit"] = judged["hit"]
            item["reason"] = judged["reason"]
        items.append(item)
    if judge_failed:
        return _dead(judge_failed)
    return _summary(items, use_judge=use_judge)


def score_saved(exam_path: Path, replies_path: Path) -> dict:
    """Grade saved replies. No generation. No server."""
    if not exam_path.is_file():
        return _dead("The exam file was not on disk, so this is not a score.")
    if not replies_path.is_file():
        return _dead("The replies file was not on disk, so this is not a score.")
    rows = read_jsonl(exam_path)
    replies = {row.get("id"): str(row.get("reply") or "") for row in read_jsonl(replies_path)}
    pairs = [(row, replies.get(row.get("id"), "")) for row in rows]
    scored = score_pairs(pairs)
    scored["replies"] = replies_path.name
    return scored


def score_with_ask(exam_path: Path, ask, *, judge=None, use_judge: bool = False) -> dict:
    if not exam_path.is_file():
        return _dead("The exam file was not on disk, so this is not a score.")
    rows = read_jsonl(exam_path)
    if not rows:
        return _dead("The exam file had no rows, so this is not a score.")
    pairs = []
    for row in rows:
        question = _question(row)
        if not question.strip():
            pairs.append((row, ""))
            continue
        try:
            reply = ask(question, row)
        except (OSError, TimeoutError, ConnectionError) as exc:
            return _dead(f"The local server did not answer ({exc.__class__.__name__}), so this is not a score.")
        if not isinstance(reply, str):
            reply = "" if reply is None else str(reply)
        pairs.append((row, reply))
    scored = score_pairs(pairs, judge=judge, use_judge=use_judge)
    if scored.get("ok"):
        scored["server_started"] = True
    return scored


def _summary(items: list[dict], *, use_judge: bool) -> dict:
    correct = sum(1 for item in items if item["hit"])
    overlap = sum(1 for item in items if item["overlap_hit"])
    keyfact = sum(1 for item in items if item["keyfact_hit"])
    total = len(items)
    derived = any(item.get("derived") for item in items)
    method = "judge" if use_judge else "key-fact"
    if use_judge:
        reason = (
            f"Judge scored {correct} of {total}. Key-fact scored {keyfact} of {total}. "
            f"Overlap scored {overlap} of {total}. This is not a clinician grade."
        )
    else:
        derived_note = " Draft key facts were derived." if derived else ""
        reason = (
            f"Key-fact scored {correct} of {total}. Overlap scored {overlap} of {total}."
            f"{derived_note} This is not a clinician grade."
        )
    return {
        "ok": True,
        "server_started": True,
        "scored": True,
        "scorer_exit": 0,
        "correct": correct,
        "total": total,
        "score": f"{correct}/{total}",
        "overlap_correct": overlap,
        "overlap_score": f"{overlap}/{total}",
        "keyfact_correct": keyfact,
        "keyfact_score": f"{keyfact}/{total}",
        "items": items,
        "misses": [item["id"] for item in items if not item["hit"]],
        "reason": one_sentence(reason),
        "method": method,
        "facts_derived": derived,
    }


def _dead(reason: str) -> dict:
    return {
        "ok": False,
        "server_started": False,
        "scored": False,
        "scorer_exit": None,
        "correct": 0,
        "total": 0,
        "score": "",
        "overlap_correct": 0,
        "overlap_score": "",
        "keyfact_correct": 0,
        "keyfact_score": "",
        "items": [],
        "misses": [],
        "reason": one_sentence(reason),
        "method": "key-fact",
        "facts_derived": False,
    }


def main(argv=None) -> int:
    import argparse
    import sys

    ap = argparse.ArgumentParser(prog="exam_keyfact")
    ap.add_argument("--exam", required=True)
    ap.add_argument("--replies", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args(argv)
    scored = score_saved(Path(args.exam), Path(args.replies))
    dest = Path(args.out)
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(json.dumps(scored, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(scored.get("score") or scored.get("reason"))
    return 0 if scored.get("ok") else 1


if __name__ == "__main__":
    raise SystemExit(main())
