"""Reading page. Rank 4. One file, bottom line first, plain words."""

from __future__ import annotations

import textwrap
from pathlib import Path


def _wrap(text: str) -> str:
    lines = []
    for para in str(text or "").split("\n"):
        para = para.strip()
        if not para:
            lines.append("")
            continue
        lines.extend(textwrap.wrap(para, width=68) or [""])
    return "\n".join(lines)


def write_page(
    path: Path,
    *,
    reason: str,
    needs_you: bool,
    tried: str,
    why: str,
    nxt: str,
    unverified: str | None = None,
    legend: str | None = None,
    gap: dict | None = None,
    quote: str | None = None,
) -> str:
    look = "Jason needs to look." if needs_you else "Jason does not need to look."
    parts = []
    if unverified:
        parts.append(_wrap(unverified))
        parts.append("")
    parts.append(_wrap(f"{reason} {look}"))
    parts.append("")
    parts.append("What we tried")
    parts.append(_wrap(tried))
    parts.append("")
    parts.append("Why it passed or failed")
    parts.append(_wrap(why))
    if gap and gap.get("reds"):
        parts.append("")
        parts.append("Red rows")
        for red in gap["reds"]:
            parts.append(_wrap(red.get("sentence", "")))
        if gap.get("counts"):
            bits = [f"{k}: {v} rows" for k, v in gap["counts"].items()]
            parts.append(_wrap("Label split: " + ", ".join(bits) + "."))
    parts.append("")
    parts.append("What is next")
    parts.append(_wrap(nxt))
    if quote:
        parts.append("")
        parts.append("Server log")
        for line in str(quote).splitlines():
            text = line.rstrip()
            if len(text) > 200:
                text = text[:199] + "."
            if text.startswith(">"):
                parts.append(text)
            else:
                parts.append("> " + text)
    if legend:
        parts.append("")
        parts.append("Legend")
        parts.append(_wrap(legend))
    parts.append("")
    parts.append("This page is a reading note. It is not a pass by itself.")
    text = "\n".join(parts) + "\n"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return text
