"""Pin a base from a prior receipt. Does not load weights and does not download."""

from __future__ import annotations

from pathlib import Path

from .reading import write_page


def write_pin(receipt: Path, base: str, dest: Path) -> dict:
    if not base or not str(base).strip():
        return _no("No base path was named, so a prior receipt could not pin one.")
    if not receipt.is_file():
        return _no("The prior receipt was not on disk, so the base was not pinned.")
    text = receipt.read_text(encoding="utf-8", errors="replace")
    if str(base) not in text:
        return _no("The prior receipt does not name this base, so the pin was not used.")
    reason = (
        f"The base {base} is already pinned by {receipt.name}. "
        "No weights were loaded for this pick."
    )
    write_page(
        dest,
        reason=reason,
        needs_you=False,
        tried="Read a prior receipt that already names this base. Did not load weights.",
        why=reason,
        nxt="A train can use this page. It is a pin, not a new comparison.",
    )
    return {
        "ok": True,
        "reason": reason,
        "page": str(dest),
        "loaded_weights": False,
        "downloaded": False,
    }


def _no(reason: str) -> dict:
    return {
        "ok": False,
        "reason": reason,
        "page": "",
        "loaded_weights": False,
        "downloaded": False,
    }
