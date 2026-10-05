"""Read-only idle or busy check. Does not start a job and does not set CUDA.

Tests pass a fake runner. The live path runs nvidia-smi query flags only.
"""

from __future__ import annotations

import subprocess
from pathlib import Path


def probe_nvidia_smi(run=None, max_util_pct: int = 30) -> dict:
    """Return state idle, busy, or unknown. seen is true only after a real read."""
    run = run or _run
    try:
        apps = run(["--query-compute-apps=pid,process_name,used_memory", "--format=csv,noheader"])
        util_raw = run(["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"])
    except (OSError, subprocess.SubprocessError, RuntimeError):
        return _unseen()
    if apps is None or util_raw is None:
        return _unseen()
    apps_text = str(apps).strip()
    util_text = str(util_raw).strip()
    if not util_text or "No devices" in apps_text or "No devices" in util_text:
        return _unseen()
    if apps_text.lower().startswith("nvidia-smi") or "failed" in apps_text.lower():
        return _unseen()
    first = (util_text.splitlines() or ["0"])[0].strip()
    try:
        util = int(first)
    except ValueError:
        return _unseen()
    apps = _parse_apps(apps_text)
    pids = [row["pid"] for row in apps if row["pid"] is not None]
    if apps_text:
        holder = _holder(apps_text)
        return {
            "state": "busy",
            "holder": holder,
            "pids": pids,
            "seen": True,
            "source": "nvidia-smi",
            "util": util,
        }
    if util > max_util_pct:
        return {
            "state": "busy",
            "holder": f"utilization {util}%",
            "pids": [],
            "seen": True,
            "source": "nvidia-smi",
            "util": util,
        }
    return {
        "state": "idle",
        "holder": None,
        "pids": [],
        "seen": True,
        "source": "nvidia-smi",
        "util": util,
    }


def _parse_apps(apps_text: str) -> list[dict]:
    rows = []
    for line in apps_text.splitlines():
        line = line.strip()
        if not line:
            continue
        parts = [part.strip() for part in line.split(",")]
        pid = int(parts[0]) if parts and parts[0].isdigit() else None
        name = parts[1] if len(parts) >= 2 else ""
        rows.append({"pid": pid, "name": name})
    return rows


def pid_is_live(pid) -> bool:
    """True when /proc still has this pid. Does not start a job."""
    try:
        number = int(pid)
    except (TypeError, ValueError):
        return False
    if number <= 0:
        return False
    return Path(f"/proc/{number}").exists()


def classify_own_train(probe: dict, train_pid, alive=None) -> dict:
    """Tell this run's own train pid from a foreign holder.

    kind is own-live, own-finished, foreign, idle, or unseen.
    blocks is true when eval must not start because of this probe.
    sentence is reading-page wording. Empty when there is nothing to add.
    A probe may set pid_alive so a test does not consult /proc.
    """
    probe = probe or {}
    pid = _as_pid(train_pid)
    pids = []
    for raw in list(probe.get("pids") or []):
        number = _as_pid(raw)
        if number is not None:
            pids.append(number)
    holder_pid = _as_pid(probe.get("pid"))
    if holder_pid is not None and holder_pid not in pids:
        pids.append(holder_pid)
    seen = bool(probe.get("seen"))
    state = probe.get("state") if seen else "unseen"
    if not seen:
        return {"kind": "unseen", "blocks": True, "pid": pid, "sentence": ""}
    own = pid is not None and pid in pids
    live = _pid_live(probe, pid, alive)
    if own and live:
        return {
            "kind": "own-live",
            "blocks": True,
            "pid": pid,
            "sentence": (
                f"The GPU is busy, held by this run's own train pid {pid}, which is still live."
            ),
        }
    if own and not live:
        return {
            "kind": "own-finished",
            "blocks": False,
            "pid": pid,
            "sentence": (
                f"This run's own train pid {pid} has finished. It is not holding the card."
            ),
        }
    if pid is not None and not live and state != "busy":
        return {
            "kind": "own-finished",
            "blocks": False,
            "pid": pid,
            "sentence": (
                f"This run's own train pid {pid} has finished. It is not holding the card."
            ),
        }
    if pid is not None and not live and state == "busy":
        return {
            "kind": "foreign",
            "blocks": True,
            "pid": pid,
            "sentence": (
                f"This run's own train pid {pid} has finished. It is not the holder."
            ),
        }
    if pid is not None and live and state == "busy":
        return {
            "kind": "foreign",
            "blocks": True,
            "pid": pid,
            "sentence": (
                f"This run's own train pid {pid} is still live. It is not the holder the probe named."
            ),
        }
    if pid is not None and live:
        return {
            "kind": "own-live",
            "blocks": False,
            "pid": pid,
            "sentence": (
                f"This run's own train pid {pid} is still live. The probe did not name it as the holder."
            ),
        }
    if state == "busy":
        return {"kind": "foreign", "blocks": True, "pid": pid, "sentence": ""}
    if state == "idle":
        return {"kind": "idle", "blocks": False, "pid": pid, "sentence": ""}
    return {"kind": "unseen", "blocks": True, "pid": pid, "sentence": ""}


def _pid_live(probe: dict, pid, alive) -> bool:
    if pid is None:
        return False
    if "pid_alive" in probe:
        return bool(probe.get("pid_alive"))
    checker = alive or pid_is_live
    return bool(checker(pid))


def _as_pid(raw):
    try:
        number = int(raw)
    except (TypeError, ValueError):
        return None
    if number <= 0:
        return None
    return number


def _holder(apps_text: str) -> str:
    line = apps_text.splitlines()[0]
    parts = [part.strip() for part in line.split(",")]
    if len(parts) >= 2 and parts[1]:
        return parts[1]
    return line


def _unseen() -> dict:
    return {"state": "unknown", "holder": None, "seen": False, "source": "nvidia-smi"}


def _run(args: list[str]) -> str:
    proc = subprocess.run(
        ["nvidia-smi", *args],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or "nvidia-smi failed")
    return proc.stdout
