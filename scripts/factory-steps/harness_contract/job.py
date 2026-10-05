"""Job file, status, card, approve file, GPU notice. Rank 1."""

from __future__ import annotations

from pathlib import Path

from .common import (
    APPROVE_KEYS,
    CARD_MAX,
    EXIT_GPU_BUSY,
    EXIT_GPU_CAP,
    EXIT_JOB,
    GPU_REQUIRED,
    GPU_WORDS,
    ID_RE,
    JOB_KEYS,
    SCHEMA_CARD,
    SCHEMA_STATUS,
    STEPS,
    STATUS_MAX,
    Inbox,
    compact,
    file_bytes,
    fit_obj,
    guarded,
    now_iso,
    one_sentence,
    write_json,
)


def validate_job(raw: dict) -> tuple[dict | None, str | None]:
    if not isinstance(raw, dict):
        return None, "The job file is not a JSON object."
    extra = [k for k in raw if k not in JOB_KEYS]
    missing = [k for k in JOB_KEYS if k not in raw]
    if extra or missing:
        return None, (
            "A job file has only id, step, config, gpu, max_minutes, and nas_out. "
            f"Missing: {missing or 'none'}. Extra: {extra or 'none'}."
        )
    job_id = raw["id"]
    if not isinstance(job_id, str) or not ID_RE.match(job_id):
        return None, "The job id must be a short token with no slashes."
    step = raw["step"]
    if step not in STEPS:
        return None, f"The step {step!r} is not in the contract list, so this job is refused."
    gpu = raw["gpu"]
    if gpu not in GPU_WORDS:
        return None, "gpu must be the word forbid or the word ask. Anything else is a refuse."
    if step in GPU_REQUIRED and gpu == "forbid":
        return None, (
            f"A {step} step with gpu forbid is a refuse, not a silent CPU fallback."
        )
    minutes = raw["max_minutes"]
    if isinstance(minutes, bool) or not isinstance(minutes, (int, float)) or minutes < 0:
        return None, "max_minutes must be a number of minutes, zero or more."
    config = raw["config"]
    nas_out = raw["nas_out"]
    if not isinstance(config, str) or not config.strip():
        return None, "config must be a path that already exists. A corpus is not pasted into the job."
    if config.strip()[:1] in "{[":
        return None, "config must be a path. Do not paste a corpus into the job."
    if not isinstance(nas_out, str) or not nas_out.strip():
        return None, "nas_out must be a path for the reading page and the big artifacts."
    cfg_path = Path(config).expanduser()
    if not cfg_path.is_file():
        return None, f"The config path is missing, so this job is refused: {config}"
    return {
        "id": job_id,
        "step": step,
        "config": str(cfg_path.resolve()),
        "gpu": gpu,
        "max_minutes": minutes,
        "nas_out": str(Path(nas_out).expanduser()),
    }, None


def validate_approve(raw: dict) -> tuple[dict | None, str | None]:
    if not isinstance(raw, dict):
        return None, "The approve file is not a JSON object."
    if raw.get("decision") not in ("yes", "no"):
        return None, "An approve file needs decision yes or no. A missing decision is not a yes."
    if not isinstance(raw.get("by"), str) or not raw["by"].strip():
        return None, "An approve file needs a by name."
    if not isinstance(raw.get("at"), str) or not raw["at"].strip():
        return None, "An approve file needs an at timestamp."
    return {"decision": raw["decision"], "by": raw["by"].strip(), "at": raw["at"].strip()}, None


def read_approve(inbox: Inbox, job_id: str) -> tuple[dict | None, str | None]:
    path = inbox.approve_path(job_id)
    if not path.is_file():
        return None, None
    try:
        from .common import read_json
        raw = read_json(path)
    except (OSError, ValueError, json_error()) as exc:
        return None, f"The approve file could not be read, so this is not a yes. {exc}"
    return validate_approve(raw)


def json_error():
    import json
    return json.JSONDecodeError


def gpu_notice(probe: dict, max_minutes) -> dict:
    """Notice written before any GPU touch. probe: state, holder, seen."""
    seen = bool(probe.get("seen"))
    state = probe.get("state") if seen else "unseen"
    holder = probe.get("holder") if seen else None
    cap = max_minutes
    if not seen:
        reason = (
            "The guard could not see whether the GPU is idle or busy, "
            "so this job did not start."
        )
        exit_code = EXIT_GPU_BUSY
        gpu = "unknown"
    elif state == "busy":
        who = holder or "a job the guard could not name"
        reason = (
            f"The GPU is busy, held by {who}. The minute cap is {cap}. "
            "An approve file does not override a busy card."
        )
        exit_code = EXIT_GPU_BUSY
        gpu = "busy"
    else:
        reason = (
            f"The GPU is idle. The guard did not see another holder. "
            f"The minute cap is {cap}."
        )
        exit_code = 0
        gpu = "idle"
    if isinstance(cap, (int, float)) and not isinstance(cap, bool) and cap < 1 and seen and state != "busy":
        reason = f"The minute cap is {cap}, so this GPU step is refuse:gpu-cap."
        exit_code = EXIT_GPU_CAP
        state = "cap"
    return {
        "gpu": gpu,
        "seen": seen,
        "holder": holder,
        "blocked": exit_code != 0,
        "exit_code": exit_code,
        "source": probe.get("source") or ("unseen" if not seen else "probe"),
        "reason": one_sentence(reason),
    }


def make_card(
    *,
    job_id: str,
    reason: str,
    result: str,
    needs_you: bool,
    reading_page: str,
    artifacts: list | None = None,
    gpu_minutes: float = 0,
) -> dict:
    card = {
        "schema": SCHEMA_CARD,
        "id": job_id,
        "reason": one_sentence(reason),
        "result": result if result in ("pass", "fail", "refused", "queued", "needs-approval") else "refused",
        "needs_you": bool(needs_you),
        "reading_page": reading_page or "",
        "artifacts": [
            {"name": a.get("name", ""), "path": a.get("path", ""), "bytes": int(a.get("bytes") or 0)}
            for a in (artifacts or [])
        ][:8],
        "gpu_minutes": float(gpu_minutes or 0),
    }
    card = fit_obj(card, CARD_MAX)
    if len(compact(card).encode()) > CARD_MAX:
        card["artifacts"] = []
        card["reason"] = one_sentence(card.get("reason", ""), 80)
        card = fit_obj(card, CARD_MAX)
    return card


def make_status(
    *,
    job_id: str,
    state: str,
    step: str,
    pid_alive: str,
    gpu: str,
    reason: str,
    reading_page: str,
    card_path: str,
) -> dict:
    if pid_alive not in ("yes", "no", "not-a-gpu-job"):
        pid_alive = "no"
    if gpu not in ("idle", "busy", "unknown", "this-job"):
        gpu = "unknown"
    status = {
        "schema": SCHEMA_STATUS,
        "id": job_id,
        "state": state,
        "step": step,
        "pid_alive": pid_alive,
        "gpu": gpu,
        "reason": one_sentence(reason),
        "updated": now_iso(),
        "reading_page": reading_page or "",
        "card": card_path,
    }
    return fit_obj(status, STATUS_MAX)


def publish(inbox: Inbox, cell: Path, status: dict, card: dict) -> None:
    """Rewrite status only, append the card to the outbox, copy both beside the cell."""
    job_id = status["id"]
    card_file = guarded(inbox.card_path(job_id))
    status_file = guarded(inbox.status_path(job_id))
    write_json(card_file, card)
    status = dict(status)
    status["card"] = str(card_file)
    status = fit_obj(status, STATUS_MAX)
    if len(compact(status).encode()) > STATUS_MAX:
        raise ValueError("refuse:status-size: status file would exceed 2 KB")
    if len(compact(card).encode()) > CARD_MAX:
        raise ValueError("refuse:card-size: card would exceed 1 KB")
    write_json(status_file, status)
    from .common import append_jsonl
    append_jsonl(inbox.outbox, card)
    cell = guarded(cell)
    cell.mkdir(parents=True, exist_ok=True)
    write_json(cell / "status.json", status)
    write_json(cell / "card.json", card)
    rec = {
        "id": job_id,
        "state": status["state"],
        "step": status["step"],
        "gpu_minutes": card.get("gpu_minutes", 0),
        "ended": status["updated"][:10],
        "updated": status["updated"],
    }
    write_json(inbox.record_path(job_id), rec)


def submit_job(inbox: Inbox, raw: dict, cell_root: Path) -> dict:
    job, err = validate_job(raw)
    inbox.root.mkdir(parents=True, exist_ok=True)
    if job is None:
        # Still leave a refused card when the id is usable, so Grok Bot can read why.
        job_id = raw.get("id") if isinstance(raw, dict) else None
        out = {
            "ok": False,
            "state": "refused",
            "reason": one_sentence(err or "The job was refused."),
            "exit_code": EXIT_JOB,
            "needs_you": False,
            "result": "refused",
            "id": job_id if isinstance(job_id, str) else "",
        }
        if isinstance(job_id, str) and ID_RE.match(job_id) and not inbox.job_path(job_id).exists():
            _write_refused_shell(inbox, cell_root, job_id, raw, out["reason"])
        return out
    dest = inbox.job_path(job["id"])
    if dest.exists():
        return {
            "ok": False,
            "state": "refused",
            "reason": one_sentence(f"Job {job['id']} already exists. It was not overwritten."),
            "exit_code": EXIT_JOB,
            "needs_you": False,
            "result": "refused",
            "id": job["id"],
        }
    guarded(dest)
    write_json(dest, job)
    cell = cell_root / f"harness-job-{job['id']}"
    status = make_status(
        job_id=job["id"],
        state="queued",
        step=job["step"],
        pid_alive="not-a-gpu-job" if job["step"] not in GPU_REQUIRED else "no",
        gpu="unknown",
        reason=f"Job {job['id']} is queued. Nothing has started.",
        reading_page="",
        card_path="",
    )
    card = make_card(
        job_id=job["id"],
        reason=status["reason"],
        result="refused",
        needs_you=False,
        reading_page="",
        gpu_minutes=0,
    )
    # A queued job has not passed or failed. The card result stays refused until a terminal state,
    # but a queue notice is not a refuse of the job. Use a neutral sentence and result refused
    # only for real refuses. For queued, the outbox still gets a line because the state changed.
    card["result"] = "queued"
    card["reason"] = one_sentence(f"Job {job['id']} is queued for {job['step']}. Nothing has started.")
    card["needs_you"] = False
    status["reason"] = card["reason"]
    publish(inbox, cell, status, card)
    return {
        "ok": True,
        "state": "queued",
        "reason": card["reason"],
        "exit_code": 0,
        "needs_you": False,
        "result": "queued",
        "id": job["id"],
        "job": job,
    }


def _write_refused_shell(inbox: Inbox, cell_root: Path, job_id: str, raw: dict, reason: str) -> None:
    step = raw.get("step") if isinstance(raw, dict) and isinstance(raw.get("step"), str) else "gap"
    if step not in STEPS:
        step = "gap"
    cell = cell_root / f"harness-job-{job_id}"
    status = make_status(
        job_id=job_id,
        state="refused",
        step=step,
        pid_alive="not-a-gpu-job",
        gpu="unknown",
        reason=reason,
        reading_page="",
        card_path="",
    )
    card = make_card(
        job_id=job_id,
        reason=reason,
        result="refused",
        needs_you=False,
        reading_page="",
    )
    try:
        publish(inbox, cell, status, card)
    except ValueError:
        return
