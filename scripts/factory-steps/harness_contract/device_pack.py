"""Device pack. Rank 14. NAS folder only. No SSH. No ollama create. ready stays no."""

from __future__ import annotations

import shutil
from pathlib import Path

from .common import EXIT_PACK, file_bytes, one_sentence, sha256_file


def pack(src: Path, dest: Path, *, approved: bool) -> dict:
    if not approved:
        return {
            "ok": False,
            "exit_code": EXIT_PACK,
            "needs_you": True,
            "state": "needs-approval",
            "reason": one_sentence(
                "A device pack needs an approve file. The folder was not written. "
                "Nothing was copied to a Mac."
            ),
        }
    if not src.is_file():
        return {
            "ok": False,
            "exit_code": EXIT_PACK,
            "needs_you": True,
            "state": "refused",
            "reason": one_sentence(
                "The device pack could not see the GGUF, so nothing was packed and nothing was downloaded."
            ),
        }
    if dest.exists() and any(dest.iterdir()):
        return {
            "ok": False,
            "exit_code": EXIT_PACK,
            "needs_you": False,
            "state": "refused",
            "reason": one_sentence(
                "The device folder already has files. They were not overwritten."
            ),
        }
    dest.mkdir(parents=True, exist_ok=True)
    target = dest / src.name
    shutil.copy2(src, target)
    digest = sha256_file(target)
    (dest / "SHA256").write_text(f"{digest}  {src.name}\n", encoding="utf-8")
    (dest / "Modelfile").write_text(
        f"FROM ./{src.name}\n",
        encoding="utf-8",
    )
    how = dest / "HOW-TO.md"
    how.write_text(
        "\n".join([
            "This folder is a device pack. Jason needs to look before seating it.",
            "",
            "What we tried",
            "Copied the GGUF that was already on disk, with its hash and a Modelfile. "
            "No download. No connection to a Mac.",
            "",
            "Why it passed or failed",
            "The copy is on the NAS. It is not seated. The registry ready flag was not flipped.",
            "",
            "What is next",
            "On the Mac, a person can point Ollama at this folder later. "
            "This pack did not run ollama create. Do not expect a live model from this folder alone. "
            "A second approve is required before any later seat job.",
            "",
            f"File: {src.name}",
            f"Bytes: {file_bytes(target)}",
            f"SHA256: {digest}",
            "",
        ]),
        encoding="utf-8",
    )
    return {
        "ok": True,
        "exit_code": 0,
        "needs_you": True,
        "state": "succeeded",
        "reason": one_sentence(
            "Wrote a device folder on the NAS with the GGUF, its hash, a Modelfile, and a how-to page. "
            "Did not run ollama create. Did not mark the registry ready."
        ),
        "artifacts": [
            {"name": src.name, "path": str(target), "bytes": file_bytes(target)},
            {"name": "HOW-TO.md", "path": str(how), "bytes": file_bytes(how)},
            {"name": "Modelfile", "path": str(dest / "Modelfile"), "bytes": file_bytes(dest / "Modelfile")},
        ],
        "ready": "no",
    }
