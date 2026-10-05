"""Score an exam JSONL against a local server. Tests pass a fake ask.

llama.cpp, vLLM, and Ollama are the same three seats the factory already uses.
This module does not start a server and does not load weights.
"""

from __future__ import annotations

import json
import re
import urllib.error
import urllib.request
from pathlib import Path

from .common import one_sentence, read_jsonl
from .fields import row_text

_TOKEN = re.compile(r"[a-z0-9]+")


def covers(reply: str, expected: str, min_ratio: float = 0.34) -> bool:
    expected_tokens = [tok for tok in _TOKEN.findall((expected or "").lower()) if len(tok) > 3]
    if not expected_tokens:
        return bool((reply or "").strip())
    got = set(_TOKEN.findall((reply or "").lower()))
    hit = sum(1 for tok in expected_tokens if tok in got)
    return (hit / len(expected_tokens)) >= min_ratio


def score_exam(
    exam_path: Path,
    ask,
    *,
    expected_field: str = "answerable_because",
    question_field: str = "question",
    min_ratio: float = 0.34,
) -> dict:
    if not exam_path.is_file():
        return _dead("The exam file was not on disk, so this is not a score.")
    rows = read_jsonl(exam_path)
    if not rows:
        return _dead("The exam file had no rows, so this is not a score.")
    items = []
    for row in rows:
        question = row.get(question_field)
        if not isinstance(question, str) or not question.strip():
            question = row_text(row, question_field)
        expected = row.get(expected_field)
        expected_text = expected if isinstance(expected, str) else ""
        if not question.strip():
            items.append({"id": row.get("id"), "hit": False, "reason": "no question"})
            continue
        try:
            reply = ask(question, row)
        except (OSError, urllib.error.URLError, TimeoutError, ConnectionError) as exc:
            return _dead(
                f"The local server did not answer ({exc.__class__.__name__}), so this is not a score."
            )
        if not isinstance(reply, str):
            reply = "" if reply is None else str(reply)
        hit = covers(reply, expected_text, min_ratio)
        items.append({
            "id": row.get("id"),
            "hit": hit,
            "reason": "overlap" if hit else "miss",
        })
    correct = sum(1 for item in items if item["hit"])
    total = len(items)
    reason = (
        f"Scored {correct} of {total} exam questions by overlap with the exam's own "
        f"{expected_field} field. This is not a clinician grade."
    )
    return {
        "ok": True,
        "server_started": True,
        "scored": True,
        "scorer_exit": 0,
        "correct": correct,
        "total": total,
        "score": f"{correct}/{total}",
        "items": items,
        "misses": [item["id"] for item in items if not item["hit"]],
        "reason": one_sentence(reason),
        "method": "token-overlap",
    }


def ask_local(question: str, *, backend: str, url: str, model: str | None = None, timeout: int = 120, poster=None) -> str:
    """One completion against an already running local server. Does not start it."""
    poster = poster or _post
    kind = (backend or "llamacpp").lower()
    root = url.rstrip("/")
    if kind in ("llamacpp", "llama.cpp", "llama-cpp"):
        data = poster(root + "/completion", {
            "prompt": question,
            "n_predict": 256,
            "temperature": 0,
            "stream": False,
        }, timeout)
        return str(data.get("content") or "")
    if kind == "ollama":
        data = poster(root + "/api/chat", {
            "model": model or "",
            "messages": [{"role": "user", "content": question}],
            "stream": False,
            "options": {"temperature": 0},
        }, timeout)
        return str((data.get("message") or {}).get("content") or "")
    data = poster(root + "/v1/chat/completions", {
        "model": model or "local",
        "messages": [{"role": "user", "content": question}],
        "temperature": 0,
    }, timeout)
    choices = data.get("choices") or [{}]
    return str((choices[0].get("message") or {}).get("content") or "")


def _post(url: str, body: dict, timeout: int) -> dict:
    req = urllib.request.Request(
        url,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode())


def _dead(reason: str) -> dict:
    return {
        "ok": False,
        "server_started": False,
        "scored": False,
        "scorer_exit": None,
        "correct": 0,
        "total": 0,
        "score": "",
        "items": [],
        "misses": [],
        "reason": one_sentence(reason),
        "method": "token-overlap",
    }
