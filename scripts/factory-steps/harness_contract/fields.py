"""Read a training row and an exam row without a hand-built view.

One field name cannot describe both files. A ShareGPT user turn and a
question field are read natively. Answer text is never returned.
"""

from __future__ import annotations

import re

_QUESTION_SPAN = re.compile(r"\b(?:What|Which|How|When|Why|Where)\b[^?.!\n]{0,80}", re.I)
_THEME_PAREN = re.compile(r"\(([A-Za-z][A-Za-z0-9_]*)\)")
_THEME_BARE = re.compile(r"^Theme:\s*([A-Za-z][A-Za-z0-9_]*)\.", re.M)
_DOSE = re.compile(r"\bwhat dose\b|\bwhich dose\b", re.I)


def user_turns(row: dict) -> str:
    parts = []
    for message in row.get("messages") or []:
        if isinstance(message, dict) and message.get("role") == "user":
            parts.append(str(message.get("content") or ""))
    return "\n".join(parts)


def row_text(row: dict, field: str | None = None) -> str:
    """Named field if it has text, otherwise the user turn, otherwise question.

    An empty named field must not win. That is how a question field on a
    ShareGPT file used to compare nothing and still pass.
    """
    if field == "messages.user":
        return user_turns(row)
    if field:
        val = row.get(field)
        if isinstance(val, str) and val.strip():
            return val
    user = user_turns(row)
    if user.strip():
        return user
    question = row.get("question")
    if isinstance(question, str) and question.strip():
        return question
    return ""


def row_label(row: dict, field: str) -> str | None:
    val = row.get(field)
    if isinstance(val, str) and val.strip():
        return val.strip()
    if isinstance(val, (int, float)) and not isinstance(val, bool):
        return str(val)
    text = user_turns(row)
    found = _THEME_PAREN.search(text)
    if found:
        return found.group(1)
    found = _THEME_BARE.search(text)
    if found:
        return found.group(1)
    return None


def is_dose_ask(text: str) -> bool:
    return bool(_DOSE.search(text or ""))


def find_contrast(texts: list[str]) -> str | None:
    """A question-like phrase that appears in some rows but not all.

    Returns None when no such phrase is found. The caller must say that a
    named phrase is needed. This does not invent one.
    """
    cleaned = [t for t in texts if t and t.strip()]
    if len(cleaned) < 2:
        return None
    lowered = [t.lower() for t in cleaned]
    seen = []
    known = set()
    for text in cleaned:
        for match in _QUESTION_SPAN.finditer(text):
            phrase = " ".join(match.group(0).split())
            key = phrase.lower()
            if len(phrase) < 8 or key in known:
                continue
            known.add(key)
            seen.append(phrase)
    for phrase in seen:
        hits = sum(1 for text in lowered if phrase.lower() in text)
        if 0 < hits < len(cleaned):
            return phrase
    return None
