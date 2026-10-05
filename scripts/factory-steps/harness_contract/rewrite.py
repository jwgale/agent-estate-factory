"""Rewrite-loop detector. Rank 8.

A second version needs a one-line delta and a yes. A third version without
that yes is a refuse.
"""

from __future__ import annotations

from pathlib import Path

from .common import EXIT_REWRITE, one_sentence, write_json


def claim(
    run_dir: Path,
    logical: str,
    version: int,
    *,
    delta: str | None = None,
    approved: bool = False,
) -> dict:
    if version < 1:
        return _no("A version number below 1 is a refuse.")
    if version == 1:
        _record(run_dir, logical, version, delta, approved)
        return {
            "ok": True,
            "exit_code": 0,
            "reason": one_sentence(f"Version 1 of {logical} is recorded."),
        }
    delta_ok = isinstance(delta, str) and len(delta.strip()) > 0 and "\n" not in delta.strip()
    if version == 2 and not (delta_ok and approved):
        return _no(
            f"Version 2 of {logical} needs a one-line delta and an approve file. "
            "It was not written."
        )
    if version >= 3 and not approved:
        return _no(
            f"Version {version} of {logical} has no approve file, so it is a refuse."
        )
    if version >= 3 and not delta_ok:
        return _no(
            f"Version {version} of {logical} needs a one-line delta and a yes. It was not written."
        )
    _record(run_dir, logical, version, delta, approved)
    return {
        "ok": True,
        "exit_code": 0,
        "reason": one_sentence(
            f"Version {version} of {logical} is recorded with a one-line delta and a yes."
        ),
    }


def _record(run_dir: Path, logical: str, version: int, delta: str | None, approved: bool) -> None:
    run_dir.mkdir(parents=True, exist_ok=True)
    path = run_dir / "rewrite-log.json"
    from .common import read_json
    if path.is_file():
        try:
            body = read_json(path)
        except ValueError:
            body = {"versions": []}
    else:
        body = {"versions": []}
    body.setdefault("versions", []).append({
        "logical": logical,
        "version": version,
        "delta": (delta or "").strip(),
        "approved": bool(approved),
    })
    write_json(path, body)


def _no(reason: str) -> dict:
    return {"ok": False, "exit_code": EXIT_REWRITE, "reason": one_sentence(reason)}
