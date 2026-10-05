"""Improve-data proposal after an eval. Does not edit the training set.

Reads a key-fact score card and the saved replies, groups misses by cause,
maps them to exam themes and the nearest training rows, and drafts new
chat rows from source notes already in the cell. Drafts that overlap the
exam, or that copy an exam question or answer, are dropped. A person
approves with an approve file. A separate apply step appends the kept
rows into a new dataset version folder.

Drafting uses a template from source text unless an already-served local
model is named. Cloud and frontier calls stay off unless allow_cloud is
true. This module does not start a server and does not train.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from urllib.parse import urlparse

from .common import EXIT_IMPROVE, guarded, one_sentence, read_json, read_jsonl, sha256_file, write_json
from .decontaminate import check as decontaminate_check
from .exam_keyfact import content_tokens, grade_reply, is_refusal, novel_tokens
from .fields import row_label, row_text, user_turns

CAUSES = (
    "missing fact",
    "wrong fact",
    "should-have-refused",
    "over-refused",
    "echoed question",
)
CAUSE_LABEL = {
    "missing fact": "missing fact",
    "wrong fact": "wrong fact / must_not_say hit",
    "should-have-refused": "should-have-refused",
    "over-refused": "over-refused",
    "echoed question": "echoed question",
}
LOCAL_HOSTS = {"127.0.0.1", "localhost", "::1"}
_THEME_LINE = re.compile(r"^Theme:\s*.+?\(([A-Za-z][A-Za-z0-9_]*)\)", re.M)
_SENTENCE = re.compile(r"[^.!?]+[.!?]?")
THIN_DEFAULT = 3
SCHEMA_APPLY = "cell-one.harness-improve-apply.v0"


def cause_of(row: dict, reply: str, *, question: str = "", expected: str = "") -> str:
    """One miss cause. A hit is not passed here."""
    graded = grade_reply(row, reply, question=question, expected=expected)
    refusing = bool(graded.get("refusal")) or is_refusal(reply)
    if graded.get("reason") == "must-not-say":
        return "wrong fact"
    if refusing and not graded.get("refusal_ok"):
        return "over-refused"
    if graded.get("refusal_ok") and not refusing:
        return "should-have-refused"
    if _echoed(reply, question or _question(row)):
        return "echoed question"
    return "missing fact"


def _echoed(reply: str, question: str) -> bool:
    tokens = content_tokens(reply)
    if not tokens:
        return False
    novel = [tok for tok in tokens if tok not in set(content_tokens(question))]
    return (len(novel) / len(tokens)) < 0.25


def _question(row: dict, field: str = "question") -> str:
    text = row.get(field)
    if isinstance(text, str) and text.strip():
        return text
    return row_text(row, field)


def _expected(row: dict) -> str:
    text = row.get("answerable_because")
    return text if isinstance(text, str) else ""


def theme_of(row: dict, field: str = "theme_id") -> str:
    for key in (field, "theme_id", "theme", "bucket", "label"):
        val = row.get(key)
        if isinstance(val, str) and val.strip():
            return val.strip()
    found = row_label(row, field or "label")
    return found or "unlabeled"


def train_theme(row: dict, field: str = "label") -> str:
    val = row.get(field)
    if isinstance(val, str) and val.strip():
        return val.strip()
    text = user_turns(row) or (row.get("text") if isinstance(row.get("text"), str) else "")
    found = _THEME_LINE.search(text or "")
    if found:
        return found.group(1)
    labeled = row_label(row, field or "label")
    return labeled or "unlabeled"


def load_replies(path: Path | None) -> dict[str, str]:
    if path is None or not path.is_file():
        return {}
    out = {}
    for row in read_jsonl(path):
        key = row.get("id")
        if key is None:
            continue
        reply = row.get("reply")
        if not isinstance(reply, str):
            reply = row.get("content") if isinstance(row.get("content"), str) else row.get("output")
        out[key] = reply if isinstance(reply, str) else ""
    return out


def load_corpus(path: Path | None) -> dict[str, dict]:
    if path is None or not path.is_dir():
        return {}
    found: dict[str, dict] = {}
    index = path / "index.jsonl"
    if index.is_file():
        for row in read_jsonl(index):
            if row.get("id"):
                found[str(row["id"])] = dict(row)
    for file in path.rglob("*.json"):
        if file.name.startswith("."):
            continue
        try:
            data = json.loads(file.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        if isinstance(data, dict) and data.get("id"):
            found.setdefault(str(data["id"]), {}).update(data)
    return found


def _source_ids(row: dict) -> list[str]:
    ids = []
    for key in ("expected_source_ids", "source_ids", "cite_ids"):
        raw = row.get(key)
        if isinstance(raw, list):
            ids.extend(str(item) for item in raw if item)
    return ids


def source_bits(row: dict, corpus: dict[str, dict], nearest: list[dict]) -> list[str]:
    """Excerpts and answerable_because already in the cell. Not the exam question."""
    bits = []
    for key in ("excerpt", "excerpts"):
        raw = row.get(key)
        if isinstance(raw, str) and raw.strip():
            bits.append(raw.strip())
        elif isinstance(raw, list):
            bits.extend(str(item).strip() for item in raw if str(item).strip())
    for sid in _source_ids(row):
        note = corpus.get(sid) or {}
        excerpt = note.get("excerpt") or note.get("text") or ""
        if isinstance(excerpt, str) and excerpt.strip():
            bits.append(excerpt.strip())
    expected = _expected(row)
    if expected.strip():
        bits.append(expected.strip())
    for train in nearest:
        text = train.get("text") or ""
        for part in text.split("Excerpt:"):
            part = part.strip()
            if part and part not in bits:
                bits.append(part[:500])
            if len(bits) >= 6:
                break
    return bits


def _tokens(text: str) -> set[str]:
    return set(content_tokens(text))


def nearest_rows(source: str, train_rows: list[dict], theme: str, label_field: str, k: int = 3) -> list[dict]:
    want = _tokens(source)
    scored = []
    for i, row in enumerate(train_rows):
        text = user_turns(row) or (row.get("text") if isinstance(row.get("text"), str) else "")
        text = str(text or "")
        assistant = _assistant(row)
        got = _tokens(text + " " + assistant)
        if not want or not got:
            score = 0.0
        else:
            score = len(want & got) / len(want)
        scored.append((score + (0.15 if train_theme(row, label_field) == theme else 0.0), score, i, row, text))
    scored.sort(key=lambda item: (-item[0], item[2]))
    out = []
    for _rank, score, i, row, text in scored[:k]:
        if score <= 0:
            continue
        out.append({
            "index": i,
            "id": row.get("id") or f"row-{i}",
            "theme": train_theme(row, label_field),
            "score": round(score, 3),
            "text": text,
        })
    return out


def _assistant(row: dict) -> str:
    parts = []
    for message in row.get("messages") or []:
        if isinstance(message, dict) and message.get("role") == "assistant":
            parts.append(str(message.get("content") or ""))
    return "\n".join(parts)


def coverage(train_rows: list[dict], themes: list[str], label_field: str, thin: int) -> dict[str, dict]:
    counts: dict[str, int] = {theme: 0 for theme in themes}
    for row in train_rows:
        label = train_theme(row, label_field)
        if label in counts:
            counts[label] += 1
    out = {}
    for theme, count in counts.items():
        out[theme] = {
            "train_rows": count,
            "thin": count < thin,
            "uncovered": count == 0,
        }
    return out


def system_turn(train_rows: list[dict]) -> str:
    for row in train_rows:
        for message in row.get("messages") or []:
            if isinstance(message, dict) and message.get("role") == "system":
                text = str(message.get("content") or "").strip()
                if text:
                    return text
    return (
        "Use only the source notes in the user message. "
        "Do not invent facts. Do not prescribe."
    )


def _norm(text: str) -> str:
    return " ".join((text or "").lower().split())


def copies_verbatim(text: str, banned: str, min_len: int) -> bool:
    if not text or not banned:
        return False
    left = _norm(text)
    right = _norm(banned)
    if not right:
        return False
    if left == right:
        return True
    return len(right) >= min_len and right in left


def verbatim_why(user: str, assistant: str, question: str, answer: str) -> str | None:
    if copies_verbatim(user, question, 24) or copies_verbatim(assistant, question, 24):
        return "copied the exam question"
    if copies_verbatim(user, answer, 40) or copies_verbatim(assistant, answer, 40):
        return "copied the exam answer"
    return None


def _short_note(text: str, question: str, limit: int = 8) -> str:
    question_words = set(_norm(question).split())
    kept = []
    for word in (text or "").split():
        bare = word.lower().strip(".,;:()[]\"'")
        if not bare or bare in question_words:
            continue
        if bare not in kept:
            kept.append(word.strip(".,;:"))
        if len(kept) >= limit:
            break
    return " ".join(kept)


def draft_row(miss: dict, sources: list[str], train_system: str) -> dict:
    """Template row. Grounded in source notes. Not a copy of the exam line."""
    theme = miss.get("theme") or "unlabeled"
    cause = miss.get("cause") or "missing fact"
    question = miss.get("question") or ""
    notes = []
    for bit in sources:
        short = _short_note(bit, question)
        if short and short not in notes:
            notes.append(short)
        if len(notes) >= 3:
            break
    note_line = "; ".join(notes) if notes else "the attached source notes"
    user = (
        f"Theme {theme}. Practice the missing behavior ({CAUSE_LABEL.get(cause, cause)}). "
        f"Use only these source words: {note_line}. "
        "Write a short teaching reply. Do not copy an exam line."
    )
    if cause == "should-have-refused":
        lead = "The source notes do not support answering that ask. Say so, and do not invent a claim."
    elif cause == "over-refused":
        lead = "This theme can be answered from the source notes. Do not refuse when the notes cover the point."
    elif cause == "echoed question":
        lead = "Do not repeat the question. Add the point the source notes actually make."
    elif cause == "wrong fact":
        lead = "Do not assert a claim the source notes rule out. State only what the notes support."
    else:
        lead = "State the point the source notes support. Do not stop at the question's own words."
    facts = []
    for fact in miss.get("facts") or []:
        for tok in novel_tokens(str(fact), question):
            if tok not in facts:
                facts.append(tok)
    tail = ""
    if facts and cause != "should-have-refused":
        tail = " The notes' distinctive words include " + ", ".join(facts[:6]) + "."
    source_line = ""
    if notes:
        source_line = " Source note words: " + notes[0] + "."
    assistant = lead + source_line + tail
    return {
        "messages": [
            {"role": "system", "content": train_system},
            {"role": "user", "content": user},
            {"role": "assistant", "content": assistant},
        ]
    }


def cloud_block(url: str | None, allow_cloud: bool) -> str | None:
    if not url:
        return None
    host = (urlparse(url).hostname or "").lower()
    if host in LOCAL_HOSTS:
        return None
    if allow_cloud:
        return None
    return "Cloud drafting is off, so no frontier call was made."


def _write_jsonl(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [json.dumps(row, ensure_ascii=False, separators=(",", ":")) for row in rows]
    path.write_text(("\n".join(lines) + ("\n" if lines else "")), encoding="utf-8")


def _page(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def _sample(row: dict) -> str:
    user = ""
    assistant = ""
    for message in row.get("messages") or []:
        if message.get("role") == "user":
            user = str(message.get("content") or "")
        elif message.get("role") == "assistant":
            assistant = str(message.get("content") or "")
    return "User: " + _clip(user, 220) + "\nAssistant: " + _clip(assistant, 220)


def _clip(text: str, limit: int) -> str:
    text = " ".join(str(text or "").split())
    if len(text) <= limit:
        return text
    return text[: limit - 1].rstrip() + "."


def render_page(summary: dict) -> str:
    lines = [
        f"Improve proposal for {summary.get('id') or 'this run'}",
        "",
        "This file is a proposal. It does not change the training set.",
        "",
        "Counts by cause",
    ]
    causes = summary.get("causes") or {}
    if summary.get("replies_missing"):
        lines.append(
            "The replies file was not on disk, so misses were not grouped by cause. "
            "No replies were invented."
        )
    else:
        for cause in CAUSES:
            lines.append(f"{CAUSE_LABEL[cause]}: {int(causes.get(cause) or 0)}")
        extra = int(summary.get("unclassified") or 0)
        if extra:
            lines.append(f"unclassified: {extra}")
    lines.append("")
    lines.append("Themes")
    themes = summary.get("themes") or {}
    if not themes:
        lines.append("No exam theme was named.")
    for name in sorted(themes):
        item = themes[name]
        mark = ""
        if item.get("uncovered"):
            mark = " No training rows."
        elif item.get("thin"):
            mark = " Thin training coverage."
        n = int(item.get("misses") or 0)
        word = "miss" if n == 1 else "misses"
        rows = int(item.get("train_rows") or 0)
        row_word = "row" if rows == 1 else "rows"
        lines.append(
            f"{name}: {n} {word}, {rows} training {row_word}.{mark}"
        )
    thin = summary.get("thin") or []
    uncovered = summary.get("uncovered") or []
    lines.append("")
    if thin or uncovered:
        lines.append(
            "Thin or no training coverage: "
            + ", ".join(uncovered + [name for name in thin if name not in uncovered])
            + "."
        )
    else:
        lines.append("No exam theme was thin or missing from the training set.")
    lines.append("")
    lines.append("Misses and nearest training rows")
    mapped = summary.get("mapped") or []
    if not mapped:
        lines.append("No miss was mapped.")
    for item in mapped:
        nearest = item.get("nearest") or []
        if nearest:
            who = ", ".join(f"{hit['id']} ({hit['score']})" for hit in nearest[:2])
        else:
            who = "none"
        cause = item.get("cause") or "unclassified"
        lines.append(f"{item.get('id')} theme {item.get('theme')} cause {cause}. Nearest: {who}.")
    lines.append("")
    lines.append("Five sample rows")
    samples = summary.get("samples") or []
    if not samples:
        lines.append("No row was kept.")
    for i, sample in enumerate(samples[:5], start=1):
        lines.append(f"{i}.")
        lines.append(sample)
        lines.append("")
    lines.append("Dropped")
    dropped = summary.get("dropped_why") or []
    if not dropped:
        lines.append("Nothing was dropped.")
    for item in dropped:
        lines.append(f"{item.get('id')}: {item.get('why')}")
    lines.append("")
    lines.append("What this step did not do")
    lines.append(summary.get("did_not") or "The training set was not edited. Apply was not run.")
    if summary.get("draft_note"):
        lines.append(summary["draft_note"])
    lines.append("")
    lines.append("This page is a proposal. It is not a pass by itself.")
    lines.append("")
    return "\n".join(lines)


def propose(
    cfg: dict,
    out_dir: Path,
    *,
    job_id: str,
    replies_path: Path | None = None,
    score_path: Path | None = None,
    ask=None,
    drafter=None,
) -> dict:
    """Write the proposal. Never writes the training file."""
    out_dir = Path(out_dir)
    dataset = Path(str(cfg.get("dataset") or cfg.get("train") or ""))
    exam = Path(str(cfg.get("exam") or ""))
    before = sha256_file(dataset) if dataset.is_file() else ""
    score_file = Path(score_path) if score_path else Path(str(cfg.get("score") or ""))
    if not score_file.is_file():
        return _fail(job_id, out_dir, "The score card was not on disk, so this is not a proposal.", dataset, before)
    if not exam.is_file():
        return _fail(job_id, out_dir, "The exam file was not on disk, so this is not a proposal.", dataset, before)
    if not dataset.is_file():
        return _fail(job_id, out_dir, "The training file was not on disk, so this is not a proposal.", dataset, before)

    score = read_json(score_file)
    exam_rows = read_jsonl(exam)
    train_rows = read_jsonl(dataset)
    label_file = Path(str(cfg.get("label_file") or ""))
    label_rows = read_jsonl(label_file) if label_file.is_file() else []
    if label_rows and len(label_rows) == len(train_rows):
        for train, label in zip(train_rows, label_rows):
            if "label" not in train and label.get("label"):
                train["label"] = label["label"]
            if not train.get("id") and label.get("id"):
                train["id"] = label["id"]
    replies_file = Path(replies_path) if replies_path else Path(str(cfg.get("replies") or ""))
    if not replies_file.is_file():
        replies_file = score_file.parent / "replies.jsonl"
    replies = load_replies(replies_file if replies_file.is_file() else None)
    replies_missing = not replies
    exam_by_id = {row.get("id"): row for row in exam_rows}
    miss_ids = list(score.get("misses") or [])
    if not miss_ids and score.get("items"):
        miss_ids = [item.get("id") for item in score["items"] if not item.get("hit")]

    label_field = str(cfg.get("label_field") or "label")
    theme_field = str(cfg.get("theme_field") or "theme_id")
    thin_n = int(cfg.get("thin") or THIN_DEFAULT)
    corpus = load_corpus(Path(cfg["corpus"]) if cfg.get("corpus") else None)
    themes = []
    for row in exam_rows:
        theme = theme_of(row, theme_field)
        if theme not in themes:
            themes.append(theme)
    covered = coverage(train_rows, themes, label_field, thin_n)
    system = system_turn(train_rows)
    allow_cloud = bool(cfg.get("allow_cloud"))
    draft_url = cfg.get("draft_url") or cfg.get("server_url") if cfg.get("draft") else cfg.get("draft_url")
    block = cloud_block(str(draft_url) if draft_url else None, allow_cloud)
    draft_note = block or "Drafting used a template from source text. No cloud call was made."
    if replies_missing:
        log_path = Path(str(cfg.get("serve_log") or ""))
        if log_path.is_file():
            draft_note = (
                "The server log lists the questions. It does not keep the replies, so it was not used as a score. "
                + draft_note
            )
    use_model = bool(draft_url) and block is None and (ask is not None or True)
    if draft_url and block is None and ask is None:
        use_model = True
    if not draft_url:
        use_model = False

    causes = {cause: 0 for cause in CAUSES}
    mapped = []
    drafts = []
    dropped = []
    unclassified = 0
    drafter = drafter or draft_row
    for miss_id in miss_ids:
        row = exam_by_id.get(miss_id)
        if row is None:
            unclassified += 1
            mapped.append({"id": miss_id, "theme": "unlabeled", "cause": "unclassified", "nearest": []})
            continue
        question = _question(row, str(cfg.get("exam_field") or "question"))
        expected = _expected(row)
        theme = theme_of(row, theme_field)
        covered.setdefault(theme, {"train_rows": 0, "thin": True, "uncovered": True, "misses": 0})
        covered[theme]["misses"] = int(covered[theme].get("misses") or 0) + 1
        nearest = nearest_rows(expected or question, train_rows, theme, label_field)
        reply = replies.get(miss_id)
        if replies_missing or reply is None:
            unclassified += 1
            mapped.append({"id": miss_id, "theme": theme, "cause": "unclassified", "nearest": nearest})
            continue
        cause = cause_of(row, reply, question=question, expected=expected)
        causes[cause] = causes.get(cause, 0) + 1
        graded = grade_reply(row, reply, question=question, expected=expected)
        mapped.append({"id": miss_id, "theme": theme, "cause": CAUSE_LABEL.get(cause, cause), "nearest": nearest})
        miss = {
            "id": miss_id,
            "theme": theme,
            "cause": cause,
            "question": question,
            "answer": expected,
            "facts": graded.get("key_facts") or [],
        }
        sources = source_bits(row, corpus, nearest)
        if use_model and drafter is draft_row:
            row_out, note = _draft_with_model(miss, sources, system, str(draft_url), cfg, ask)
            if note:
                draft_note = note
        else:
            row_out = drafter(miss, sources, system)
        if not isinstance(row_out, dict) or not row_out.get("messages"):
            dropped.append({"id": miss_id, "why": "the drafter returned no chat row"})
            continue
        user = user_turns(row_out)
        assistant = _assistant(row_out)
        why = verbatim_why(user, assistant, question, expected)
        if why:
            dropped.append({"id": miss_id, "why": why})
            continue
        if not user.strip():
            dropped.append({"id": miss_id, "why": "the user turn was empty"})
            continue
        row_out = {"messages": row_out["messages"]}
        row_out["_miss"] = miss_id
        drafts.append(row_out)

    kept, decon_drops, decon_note = _drop_overlap(drafts, exam, cfg, out_dir / "drafts.jsonl")
    dropped.extend(decon_drops)
    for row in kept:
        row.pop("_miss", None)
    proposal_path = out_dir / "proposed-rows.jsonl"
    page_path = out_dir / "improve.md"
    _write_jsonl(proposal_path, kept)
    thin = [name for name, item in covered.items() if item.get("thin") and not item.get("uncovered")]
    uncovered = [name for name, item in covered.items() if item.get("uncovered")]
    after = sha256_file(dataset)
    unchanged = before == after
    summary = {
        "id": job_id,
        "replies_missing": replies_missing,
        "causes": causes,
        "unclassified": unclassified,
        "themes": covered,
        "thin": thin,
        "uncovered": uncovered,
        "mapped": mapped,
        "samples": [_sample(row) for row in kept[:5]],
        "dropped_why": dropped,
        "draft_note": draft_note,
        "did_not": (
            "The training set was not edited. No approve file was written. "
            "Apply was not run. No cloud call was made."
            if not allow_cloud
            else "The training set was not edited. No approve file was written. Apply was not run."
        ),
    }
    _page(page_path, render_page(summary))
    write_json(out_dir / "summary.json", {
        "id": job_id,
        "misses": len(miss_ids),
        "causes": causes,
        "unclassified": unclassified,
        "replies_missing": replies_missing,
        "drafted": len(drafts) + len([item for item in dropped if item.get("why", "").startswith("copied")]),
        "kept": len(kept),
        "dropped": len(dropped),
        "thin": thin,
        "uncovered": uncovered,
        "themes": {name: {"misses": item.get("misses", 0), "train_rows": item.get("train_rows", 0)} for name, item in covered.items()},
        "dataset_unchanged": unchanged,
        "allow_cloud": allow_cloud,
    })
    if not unchanged:
        reason = "The training file changed during the proposal. That is a refuse. The proposal was not trusted."
        ok = False
    elif replies_missing:
        reason = (
            f"The score card lists {len(miss_ids)} misses. The replies file was not on disk, "
            "so causes were not grouped and no rows were drafted. The training set was not edited."
        )
        ok = False
    else:
        reason = (
            f"Kept {len(kept)} proposed rows. Dropped {len(dropped)}. "
            f"{_cause_phrase(causes)} The training set was not edited."
        )
        if decon_note:
            reason = reason + " " + decon_note
        ok = True
    return {
        "ok": ok,
        "exit_code": 0 if ok else EXIT_IMPROVE,
        "reason": one_sentence(reason),
        "causes": causes,
        "unclassified": unclassified,
        "misses": len(miss_ids),
        "themes": covered,
        "thin": thin,
        "uncovered": uncovered,
        "drafted": len(kept) + len(dropped),
        "kept": len(kept),
        "dropped": len(dropped),
        "dropped_why": dropped,
        "needs_you": True,
        "dataset_unchanged": unchanged,
        "dataset_sha256": after,
        "artifacts": [
            {"name": "proposed-rows.jsonl", "path": str(proposal_path), "bytes": proposal_path.stat().st_size},
            {"name": "improve.md", "path": str(page_path), "bytes": page_path.stat().st_size},
        ],
    }


def _cause_phrase(causes: dict) -> str:
    bits = [f"{CAUSE_LABEL[cause]} {int(causes.get(cause) or 0)}" for cause in CAUSES]
    return "Causes: " + ", ".join(bits) + "."


def _draft_with_model(miss, sources, system, url, cfg, ask):
    prompt = (
        "Write one training example that teaches the missing behavior. "
        "Ground it only in the source notes. Do not copy the exam question or the exam answer. "
        'Reply with one JSON object: {"user": "...", "assistant": "..."}.'
        "\n" + json.dumps({
            "cause": miss.get("cause"),
            "theme": miss.get("theme"),
            "source_notes": [_short_note(bit, miss.get("question") or "") for bit in sources[:3]],
        }, ensure_ascii=False)
    )
    try:
        if ask is not None:
            text = ask(prompt)
        else:
            from .exam_score import ask_local
            text = ask_local(
                prompt,
                backend=str(cfg.get("draft_backend") or "vllm"),
                url=url,
                model=cfg.get("draft_model"),
            )
    except (OSError, TimeoutError, ConnectionError) as exc:
        return draft_row(miss, sources, system), (
            f"The local drafter did not answer ({exc.__class__.__name__}). "
            "A template was used instead. No server was started."
        )
    parsed = _parse_draft(text)
    if parsed is None:
        return draft_row(miss, sources, system), "The local drafter reply was not a chat row. A template was used instead."
    row = {
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": parsed["user"]},
            {"role": "assistant", "content": parsed["assistant"]},
        ]
    }
    return row, "Drafting used an already-served local model. No server was started. No cloud call was made."


def _parse_draft(text: str) -> dict | None:
    raw = (text or "").strip()
    try:
        data = json.loads(raw)
    except json.JSONDecodeError:
        start = raw.find("{")
        end = raw.rfind("}")
        if start < 0 or end <= start:
            return None
        try:
            data = json.loads(raw[start:end + 1])
        except json.JSONDecodeError:
            return None
    if not isinstance(data, dict):
        return None
    user = data.get("user")
    assistant = data.get("assistant")
    if not isinstance(user, str) or not isinstance(assistant, str):
        return None
    if not user.strip() or not assistant.strip():
        return None
    return {"user": user.strip(), "assistant": assistant.strip()}


def _drop_overlap(drafts: list[dict], exam: Path, cfg: dict, dest: Path) -> tuple[list[dict], list[dict], str]:
    if not drafts:
        return [], [], ""
    field = str(cfg.get("train_field") or "messages.user")
    exam_field = str(cfg.get("exam_field") or "question")
    clean = []
    ids = []
    for row in drafts:
        ids.append(row.get("_miss"))
        clean.append({"messages": row.get("messages") or []})
    _write_jsonl(dest, clean)
    checked = decontaminate_check(dest, exam, field, train_field=field, exam_field=exam_field)
    try:
        dest.unlink()
    except OSError:
        pass
    if checked.get("unverified") or (not checked.get("ok") and not checked.get("hits")):
        why = checked.get("reason") or "Decontaminate could not check the drafts."
        return [], [{"id": ids[i] or i, "why": why} for i in range(len(drafts))], why
    hit_rows = {hit.get("row") for hit in checked.get("hits") or []}
    if int(checked.get("hit_count") or 0) > len(hit_rows):
        hit_rows = set()
        for n, row in enumerate(clean):
            one = dest.parent / f".one-{n}.jsonl"
            _write_jsonl(one, [row])
            one_check = decontaminate_check(one, exam, field, train_field=field, exam_field=exam_field)
            if one_check.get("hits"):
                hit_rows.add(n)
            try:
                one.unlink()
            except OSError:
                pass
    kept = []
    dropped = []
    for n, row in enumerate(drafts):
        if n in hit_rows:
            kind = "overlap"
            for hit in checked.get("hits") or []:
                if hit.get("row") == n and hit.get("hits"):
                    kind = hit["hits"][0].get("kind") or kind
                    break
            dropped.append({"id": ids[n] or n, "why": f"overlapped the exam ({kind})"})
            continue
        kept.append(row)
    return kept, dropped, ""


def _fail(job_id: str, out_dir: Path, reason: str, dataset: Path, before: str) -> dict:
    page = out_dir / "improve.md"
    proposal = out_dir / "proposed-rows.jsonl"
    _write_jsonl(proposal, [])
    summary = {
        "id": job_id,
        "replies_missing": True,
        "causes": {},
        "themes": {},
        "mapped": [],
        "samples": [],
        "dropped_why": [],
        "did_not": "The training set was not edited. No rows were drafted.",
        "draft_note": reason,
    }
    _page(page, render_page(summary))
    after = sha256_file(dataset) if dataset.is_file() else ""
    return {
        "ok": False,
        "exit_code": EXIT_IMPROVE,
        "reason": one_sentence(reason),
        "causes": {},
        "unclassified": 0,
        "misses": 0,
        "themes": {},
        "thin": [],
        "uncovered": [],
        "drafted": 0,
        "kept": 0,
        "dropped": 0,
        "dropped_why": [],
        "needs_you": True,
        "dataset_unchanged": (not before) or before == after,
        "dataset_sha256": after,
        "artifacts": [
            {"name": "proposed-rows.jsonl", "path": str(proposal), "bytes": proposal.stat().st_size if proposal.is_file() else 0},
            {"name": "improve.md", "path": str(page), "bytes": page.stat().st_size if page.is_file() else 0},
        ],
    }


def apply_proposal(
    cfg: dict,
    *,
    job_id: str,
    approved_by: str,
    approved_at: str,
    receipt_dir: Path | None = None,
) -> dict:
    """Append approved rows into a new dataset version folder. Does not edit the original."""
    dataset = Path(str(cfg.get("dataset") or ""))
    proposal = Path(str(cfg.get("proposal") or cfg.get("proposed_rows") or ""))
    if not proposal.is_file():
        guess = Path(str(cfg.get("proposal_dir") or "")) / "proposed-rows.jsonl"
        if guess.is_file():
            proposal = guess
    if not dataset.is_file():
        return {"ok": False, "exit_code": EXIT_IMPROVE, "reason": one_sentence("The training file was not on disk, so nothing was applied."), "dataset_unchanged": True}
    if not proposal.is_file():
        return {"ok": False, "exit_code": EXIT_IMPROVE, "reason": one_sentence("The proposal file was not on disk, so nothing was applied."), "dataset_unchanged": True}
    before = sha256_file(dataset)
    original = read_jsonl(dataset)
    added = read_jsonl(proposal)
    if not added:
        return {
            "ok": False,
            "exit_code": EXIT_IMPROVE,
            "reason": one_sentence("The proposal had no rows, so no new dataset version was written."),
            "dataset_unchanged": sha256_file(dataset) == before,
        }
    root = Path(cfg["version_root"]) if cfg.get("version_root") else dataset.parent / "versions"
    dest_dir = root / job_id
    try:
        guarded(dest_dir)
    except ValueError as exc:
        return {
            "ok": False,
            "exit_code": EXIT_IMPROVE,
            "reason": one_sentence(str(exc)),
            "dataset_unchanged": sha256_file(dataset) == before,
        }
    if dest_dir.exists() and any(dest_dir.iterdir()):
        return {
            "ok": False,
            "exit_code": EXIT_IMPROVE,
            "reason": one_sentence("That dataset version folder already has files, so it was not overwritten."),
            "dataset_unchanged": sha256_file(dataset) == before,
        }
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest = dest_dir / dataset.name
    combined = original + added
    _write_jsonl(dest, combined)
    info = dataset.parent / "dataset_info.json"
    if info.is_file():
        try:
            meta = json.loads(info.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            meta = None
        if isinstance(meta, dict):
            for entry in meta.values():
                if isinstance(entry, dict) and "file_name" in entry:
                    entry["file_name"] = dest.name
            write_json(dest_dir / "dataset_info.json", meta)
    after = sha256_file(dataset)
    unchanged = before == after
    receipt = dest_dir / "receipt.md"
    lines = [
        f"# {job_id} improve-apply",
        "",
        f"schema: {SCHEMA_APPLY}",
        f"approved by: {approved_by}",
        f"approved at: {approved_at}",
        f"original: {dataset}",
        f"original sha256: {before}",
        f"original rows: {len(original)}",
        f"appended: {len(added)}",
        f"new file: {dest}",
        f"new sha256: {sha256_file(dest)}",
        f"new rows: {len(combined)}",
        f"original unchanged: {str(unchanged).lower()}",
        "",
        "The original training file was not edited.",
        "READY_FOR_LIVE_TEST: no",
        "This receipt does not flip READY and does not seat a model.",
        "",
    ]
    receipt.write_text("\n".join(lines), encoding="utf-8")
    if receipt_dir is not None:
        extra = Path(receipt_dir) / f"{job_id}-apply.md"
        extra.parent.mkdir(parents=True, exist_ok=True)
        extra.write_text(receipt.read_text(encoding="utf-8"), encoding="utf-8")
    if not unchanged:
        return {
            "ok": False,
            "exit_code": EXIT_IMPROVE,
            "reason": one_sentence("The original training file changed during apply. That is a refuse."),
            "dataset_unchanged": False,
            "version_dir": str(dest_dir),
        }
    return {
        "ok": True,
        "exit_code": 0,
        "reason": one_sentence(
            f"Appended {len(added)} approved rows into a new dataset version. "
            f"The original training file was not edited. {len(combined)} rows are in the new folder."
        ),
        "dataset_unchanged": True,
        "dataset_sha256": after,
        "version_dir": str(dest_dir),
        "new_file": str(dest),
        "appended": len(added),
        "original_rows": len(original),
        "artifacts": [
            {"name": dest.name, "path": str(dest), "bytes": dest.stat().st_size},
            {"name": "receipt.md", "path": str(receipt), "bytes": receipt.stat().st_size},
        ],
    }
