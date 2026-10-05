"""Judge-exit and dead-start refuses. Rank 7. A DONE file is not a pass."""

from __future__ import annotations

from .common import EXIT_JUDGE, one_sentence


def judge(*, scorer_exit: int | None, server_started: bool, done_file_exists: bool) -> dict:
    if not server_started:
        return {
            "ok": False,
            "state": "refused",
            "result": "refused",
            "scored": False,
            "exit_code": EXIT_JUDGE,
            "reason": one_sentence(
                "The server never started, so this is not a score."
                + (" A DONE file is present and is not a pass." if done_file_exists else "")
            ),
        }
    if scorer_exit is None or scorer_exit != 0:
        return {
            "ok": False,
            "state": "failed",
            "result": "fail",
            "scored": True,
            "exit_code": EXIT_JUDGE,
            "reason": one_sentence(
                "The scorer did not exit 0, so this is not a pass."
                + (" A DONE file is present and is not a pass." if done_file_exists else "")
            ),
        }
    return {
        "ok": True,
        "state": "succeeded",
        "result": "pass",
        "scored": True,
        "exit_code": 0,
        "reason": one_sentence("The scorer exited 0 and the server had started, so this score stands."),
    }
