"""Self-serving eval. Starts a local LLaMA-Factory API, scores, always stops.

Tests pass a fake starter, stopper, health check, and probe. This module does
not call nvidia-smi and does not start a process unless start_live is called.
The live path is not used by the CPU tests. The CLI is a resolved path, never a
bare name on PATH.

llamafactory-cli api binds os.environ API_HOST (default 0.0.0.0) and API_PORT
(default 8000), then serves GET /v1/models. The health URL uses that same port.
A bind of 0.0.0.0 is probed at 127.0.0.1, because connecting to 0.0.0.0 is not
a health check. The child process always receives API_HOST and API_PORT.
"""

from __future__ import annotations

import json
import os
import socket
import subprocess
from pathlib import Path

from .common import one_sentence, repo_root

DEFAULT_MINUTES = 15
DEFAULT_HEALTH_TIMEOUT = 300
HEALTH_POLL_SECONDS = 3
LOG_TAIL_LINES = 12


class CliMissing(RuntimeError):
    """The resolved llamafactory-cli is not executable. A gate, not a traceback."""


class PortBusy(RuntimeError):
    """The API port is already taken. A gate, not a traceback."""


def _expand(raw: str, home: Path) -> str:
    text = str(raw).strip()
    if text == "~":
        return str(home)
    if text.startswith("~/"):
        return str(home / text[2:])
    return text


def _executable(path: str) -> bool:
    try:
        return Path(path).is_file() and os.access(path, os.X_OK)
    except OSError:
        return False


def missing_cli_gate(path: str) -> str:
    return (
        f"llamafactory-cli not executable: {path}. "
        "Set llamafactory_cli or LLAMAFACTORY_CLI, or place it beside QUANT_LADDER_PYTHON."
    )


def port_busy_gate(host: str, port: int) -> str:
    return f"Port {int(port)} on {host} is already in use, so the server was not started."


def _seconds(raw, default: float = DEFAULT_HEALTH_TIMEOUT) -> int:
    try:
        value = float(raw)
    except (TypeError, ValueError):
        value = float(default)
    if value < 1:
        value = float(default)
    return int(value)


def bind_target(host: str, port: int) -> dict:
    """The env the CLI binds, and the URL a health check can actually open.

    llamafactory.api.app.run_api reads API_HOST and API_PORT, then uvicorn binds
    that pair. GET /v1/models is the ready check. Probe 127.0.0.1 when the bind
    host is every interface.
    """
    bind = (host or "127.0.0.1").strip() or "127.0.0.1"
    probe = "127.0.0.1" if bind in ("0.0.0.0", "::", "[::]") else bind
    base = f"http://{probe}:{int(port)}"
    return {
        "bind_host": bind,
        "probe_host": probe,
        "port": int(port),
        "url": base,
        "health_url": base + "/v1/models",
        "env": {"API_HOST": bind, "API_PORT": str(int(port))},
    }


def health_target(url: str) -> str:
    """Full /v1/models URL. A base URL gets the path. A full URL is not doubled."""
    base = str(url or "").rstrip("/")
    if base.endswith("/v1/models"):
        return base
    return base + "/v1/models"


def resolve_cli(cfg: dict | None = None, environ: dict | None = None, home: Path | None = None) -> dict:
    """Config key or LLAMAFACTORY_CLI, else the sibling of QUANT_LADDER_PYTHON, else the venv default.

    A set path that is missing is a gate. It does not fall through to the next place.
    """
    env = os.environ if environ is None else environ
    root = Path.home() if home is None else Path(home)
    data = cfg or {}
    raw = None
    source = "default"
    for key in ("llamafactory_cli", "cli"):
        val = data.get(key)
        if isinstance(val, str) and val.strip():
            raw, source = val.strip(), "config"
            break
    if raw is None:
        val = env.get("LLAMAFACTORY_CLI")
        if isinstance(val, str) and val.strip():
            raw, source = val.strip(), "env"
    if raw is not None:
        path = _expand(raw, root)
    else:
        py = env.get("QUANT_LADDER_PYTHON")
        if isinstance(py, str) and py.strip():
            path = str(Path(_expand(py.strip(), root)).parent / "llamafactory-cli")
            source = "sibling"
        else:
            path = str(root / "venvs" / "llamafactory" / "bin" / "llamafactory-cli")
            source = "default"
    gate = None if _executable(path) else missing_cli_gate(path)
    return {"path": path, "source": source, "gate": gate}


def build_spec(cfg: dict, *, yaml_path: Path | None = None, environ: dict | None = None, home: Path | None = None) -> dict:
    port = int(cfg.get("api_port") or cfg.get("port") or 18080)
    host = str(cfg.get("api_host") or "127.0.0.1")
    minutes = cfg.get("serve_minutes", DEFAULT_MINUTES)
    try:
        minutes = float(minutes)
    except (TypeError, ValueError):
        minutes = float(DEFAULT_MINUTES)
    yaml = {
        "model_name_or_path": str(cfg.get("base") or cfg.get("model_name_or_path") or ""),
        "adapter_name_or_path": str(cfg.get("adapter") or ""),
        "template": str(cfg.get("template") or "qwen3_5"),
        "infer_backend": str(cfg.get("infer_backend") or "huggingface"),
        "finetuning_type": str(cfg.get("finetuning_type") or "lora"),
        "enable_thinking": bool(cfg.get("enable_thinking", False)),
        "max_new_tokens": int(cfg.get("max_new_tokens") or 256),
    }
    path = Path(yaml_path) if yaml_path else None
    chosen = resolve_cli(cfg, environ=environ, home=home)
    argv = [chosen["path"], "api", str(path) if path else "<serve.yaml>"]
    bound = bind_target(host, port)
    return {
        "argv": argv,
        "command": " ".join(argv),
        "cli": chosen["path"],
        "cli_source": chosen["source"],
        "cli_gate": chosen["gate"],
        "env": bound["env"],
        "url": bound["url"],
        "health_url": bound["health_url"],
        "health_path": "/v1/models",
        "bind_host": bound["bind_host"],
        "probe_host": bound["probe_host"],
        "health_timeout": _seconds(
            cfg.get("health_timeout_s", cfg.get("health_timeout", DEFAULT_HEALTH_TIMEOUT))
        ),
        "health_poll": HEALTH_POLL_SECONDS,
        "yaml": yaml,
        "yaml_path": str(path) if path else "",
        "max_minutes": minutes,
        "template": yaml["template"],
        "infer_backend": yaml["infer_backend"],
    }


def may_serve(probe: dict, *, fake_server: bool) -> tuple[bool, str]:
    """A live start needs a seen idle nvidia-smi probe. A fake server may use a test probe."""
    if not probe or not probe.get("seen"):
        return False, "The guard could not see the GPU, so the server was not started."
    if probe.get("state") != "idle":
        return False, "The GPU is not idle, so the server was not started."
    source = probe.get("source") or ""
    if source == "nvidia-smi":
        return True, "The GPU is idle on a nvidia-smi read."
    if fake_server and source in ("fake", "test"):
        return True, "A fake server is attached, so the test probe may start it."
    return False, "The probe was not a nvidia-smi read, so the server was not started."


def render_yaml(data: dict) -> str:
    lines = []
    for key, val in data.items():
        if isinstance(val, bool):
            text = "true" if val else "false"
        elif isinstance(val, int):
            text = str(val)
        else:
            text = json.dumps(str(val))
        lines.append(f"{key}: {text}")
    return "\n".join(lines) + "\n"


def port_free(host: str, port: int) -> bool:
    """True when nothing is accepting on the port the CLI would bind."""
    probe = "127.0.0.1" if host in ("0.0.0.0", "::", "[::]", "") else host
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        sock.settimeout(0.5)
        return sock.connect_ex((probe, int(port))) != 0
    except OSError:
        return False
    finally:
        sock.close()


def resolve_log_path(spec: dict) -> str:
    raw = str((spec or {}).get("log_path") or "").strip()
    if raw:
        return raw
    yaml = str((spec or {}).get("yaml_path") or "").strip()
    if yaml:
        return str(Path(yaml).parent / "serve.log")
    return ""


def _append_log(path: str, line: str) -> None:
    if not path:
        return
    dest = Path(path)
    dest.parent.mkdir(parents=True, exist_ok=True)
    with open(dest, "a", encoding="utf-8") as handle:
        handle.write(line.rstrip() + "\n")


def tail_log(path, n: int = LOG_TAIL_LINES) -> str:
    if not path:
        return ""
    dest = Path(path)
    if not dest.is_file():
        return ""
    try:
        text = dest.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""
    lines = [line for line in text.splitlines() if line.strip()]
    if not lines:
        return ""
    return "\n".join(lines[-n:])


def page_quote(path, tail: str = "") -> str:
    """Last log lines for the reading page. Empty when there is no log path."""
    text = tail or tail_log(path)
    if text:
        return f"{path or 'serve.log'}\n{text}"
    if not path:
        return ""
    if Path(path).is_file():
        return f"{path}\nThe server log was empty."
    return f"{path}\nThe server log was missing."


def process_exit_code(handle) -> int | None:
    proc = (handle or {}).get("proc")
    if proc is None:
        return None
    try:
        code = proc.poll()
    except Exception:
        return None
    if code is None:
        return None
    return int(code)


def start_live(spec: dict, popen=None, port_check=None):
    """Start the recorded API command. Tests pass a fake popen and do not call this with the real one.

    A live start checks the port, then writes stdout and stderr to the serve log.
    A taken port does not start the process.
    """
    path = Path(spec["yaml_path"]) if spec.get("yaml_path") else None
    if path is None:
        raise RuntimeError("A serve yaml path is required before a live start.")
    live = popen is None
    if live:
        gate = spec.get("cli_gate")
        cli = str((spec.get("argv") or [""])[0])
        if not gate and cli and not _executable(cli):
            gate = missing_cli_gate(cli)
        if gate:
            raise CliMissing(gate)
    host = str((spec.get("env") or {}).get("API_HOST") or spec.get("bind_host") or "127.0.0.1")
    port = int((spec.get("env") or {}).get("API_PORT") or spec.get("port") or 18080)
    log_path = resolve_log_path(spec) if (live or spec.get("log_path")) else ""
    if live and not log_path:
        raise RuntimeError("A serve log path is required before a live start.")
    check = port_check if port_check is not None else (port_free if live else None)
    if check is not None and not check(host, port):
        note = port_busy_gate(host, port)
        _append_log(log_path, note)
        raise PortBusy(note)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(render_yaml(spec["yaml"]), encoding="utf-8")
    env = os.environ.copy()
    env.pop("CUDA_VISIBLE_DEVICES", None)
    env.update(spec.get("env") or {})
    log_fh = None
    if log_path:
        dest = Path(log_path)
        dest.parent.mkdir(parents=True, exist_ok=True)
        log_fh = open(dest, "ab", buffering=0)
        stdout = log_fh
        stderr = subprocess.STDOUT
    else:
        stdout = subprocess.DEVNULL
        stderr = subprocess.DEVNULL
    run = popen or subprocess.Popen
    try:
        proc = run(
            spec["argv"],
            cwd=str(repo_root()),
            env=env,
            stdout=stdout,
            stderr=stderr,
            start_new_session=True,
        )
    except Exception:
        if log_fh is not None:
            log_fh.close()
        raise
    return {
        "pid": proc.pid,
        "proc": proc,
        "log_fh": log_fh,
        "log_path": log_path,
    }


def stop_live(handle: dict) -> None:
    proc = (handle or {}).get("proc")
    log_fh = (handle or {}).get("log_fh")
    try:
        if proc is None:
            return
        try:
            proc.terminate()
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            try:
                proc.kill()
            except OSError:
                return
        except OSError:
            return
    finally:
        if log_fh is not None:
            try:
                log_fh.close()
            except OSError:
                pass


def health_live(url: str, timeout: int = 2) -> bool:
    import urllib.request
    req = urllib.request.Request(health_target(url))
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return int(resp.status) == 200


def _fail(out: dict, spec: dict, reason: str, *, capped: bool = False) -> dict:
    out["capped"] = capped
    out["reason"] = one_sentence(reason)
    out["log_tail"] = tail_log(spec.get("log_path"))
    return out


def guarded_serve(
    spec: dict,
    *,
    starter,
    stopper,
    health,
    work,
    clock,
    sleep,
    max_minutes: float | None = None,
) -> dict:
    """Start, wait, run work, always stop. A failed health check or a blown cap is not a score.

    Connection errors during the wait mean the model is still loading. They are
    not a score by themselves. The wait polls every 3 seconds until the health
    timeout (default 300 seconds) or the serve cap, and stops early if the
    process has already exited.
    """
    raw_cap = spec.get("max_minutes", DEFAULT_MINUTES) if max_minutes is None else max_minutes
    cap = float(DEFAULT_MINUTES if raw_cap is None else raw_cap)
    health_timeout = _seconds(spec.get("health_timeout", DEFAULT_HEALTH_TIMEOUT))
    poll = _seconds(spec.get("health_poll", HEALTH_POLL_SECONDS), default=HEALTH_POLL_SECONDS)
    handle = None
    out = {
        "ok": False,
        "started": False,
        "stopped": False,
        "healthy": False,
        "capped": False,
        "reason": one_sentence("The server was not started."),
        "log_tail": "",
    }
    started_at = clock()
    try:
        if cap < 1:
            out["reason"] = one_sentence(f"The serve cap is {cap} minutes, so the server was not started.")
            out["capped"] = True
            return out
        gate = spec.get("cli_gate")
        if gate:
            out["reason"] = one_sentence(gate)
            return out
        try:
            handle = starter(spec)
        except PortBusy as exc:
            return _fail(out, spec, str(exc))
        out["started"] = True
        target = spec.get("health_url") or spec.get("url") or ""
        while True:
            now = clock()
            elapsed = now - started_at
            code = process_exit_code(handle)
            if code is not None:
                return _fail(
                    out,
                    spec,
                    f"The server process exited ({code}) before it answered, so this is not a score.",
                )
            if elapsed >= cap * 60:
                return _fail(
                    out,
                    spec,
                    "The serve cap was reached before the server answered, so this is not a score.",
                    capped=True,
                )
            if elapsed >= health_timeout:
                return _fail(
                    out,
                    spec,
                    f"The server did not answer within {health_timeout} seconds, so this is not a score.",
                )
            try:
                answered = bool(health(target))
            except (OSError, TimeoutError, ConnectionError):
                answered = False
            if answered:
                out["healthy"] = True
                break
            remaining = min(health_timeout, cap * 60) - (clock() - started_at)
            if remaining <= 0:
                continue
            sleep(min(poll, remaining))
        if not out["healthy"]:
            return _fail(out, spec, "The server did not answer, so this is not a score.")
        try:
            body = work()
        except Exception as exc:
            return _fail(
                out,
                spec,
                f"The score step failed ({exc.__class__.__name__}), so this is not a score.",
            )
        if not isinstance(body, dict):
            body = {"ok": False, "reason": "The score step returned nothing."}
        if clock() - started_at > cap * 60:
            body["ok"] = False
            body["capped"] = True
            body["reason"] = one_sentence("The serve cap was reached, so this is not a score.")
        body.setdefault("ok", True)
        body["started"] = True
        body["healthy"] = True
        body["stopped"] = False
        body.setdefault("log_tail", "")
        out = body
        return out
    finally:
        if handle is not None:
            stopper(handle)
            out["stopped"] = True
