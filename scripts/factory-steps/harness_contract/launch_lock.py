"""Launch lock. Off unless the job config sets allow_launch.

execute_recorded is the only place that runs the recorded command. Tests pass
a fake runner. This file does not call train-step.sh on import.
"""

from __future__ import annotations

import os
import shlex
import subprocess
import sys

from .common import repo_root


def execute_recorded(plan: dict, runner=None) -> dict:
    if not plan.get("allow_launch"):
        return {"launched": False, "rc": None, "server_started": False, "gpu_minutes": 0}
    command = str(plan.get("command") or "")
    if not command or "<missing" in command:
        return {
            "launched": False,
            "rc": None,
            "server_started": False,
            "gpu_minutes": 0,
            "reason": "The plan has no command, so the launch lock stayed shut.",
        }
    run = runner or _subprocess
    rc, output, result = _unpack_run(run(command))
    return {
        "launched": True,
        "rc": rc,
        "output": output,
        "result": result,
        "server_started": False,
        "gpu_minutes": 0,
    }


def _unpack_run(ran):
    """A fake may return an int, or rc plus the train-step text."""
    if isinstance(ran, dict):
        return ran.get("rc"), str(ran.get("output") or ""), ran.get("result")
    if isinstance(ran, tuple):
        rc = ran[0] if ran else None
        output = ran[1] if len(ran) > 1 else ""
        result = ran[2] if len(ran) > 2 else None
        return rc, str(output or ""), result
    return ran, "", None


def _subprocess(command: str) -> dict:
    env = os.environ.copy()
    # The harness process blanks CUDA so it does not touch the GPU.
    # The recorded train script is the child that is allowed to see the card.
    env.pop("CUDA_VISIBLE_DEVICES", None)
    proc = subprocess.run(
        shlex.split(command),
        cwd=repo_root(),
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )
    output = (proc.stdout or "") + (proc.stderr or "")
    if output:
        sys.stdout.write(output)
        if not output.endswith("\n"):
            sys.stdout.write("\n")
    return {"rc": int(proc.returncode), "output": output}
