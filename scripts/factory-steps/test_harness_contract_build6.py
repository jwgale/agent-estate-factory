#!/usr/bin/env python3
"""CPU tests for the serve log and the health wait. Fakes only. No GPU. No train."""

from __future__ import annotations

import json
import socket
import stat
import subprocess
import sys
import tempfile
import unittest
import urllib.error
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from harness_contract.common import ESTATE_CKSUM, estate_path, read_cksum, read_json  # noqa: E402
from harness_contract.eval_serve import (  # noqa: E402
    PortBusy,
    bind_target,
    build_spec,
    guarded_serve,
    health_target,
    port_free,
    start_live,
    stop_live,
)
from harness_contract.runner import Harness  # noqa: E402

IDLE = {
    "state": "idle",
    "holder": None,
    "seen": True,
    "source": "test",
    "pid_alive": False,
}


class Clock:
    def __init__(self):
        self.t = 0.0

    def __call__(self):
        return self.t

    def sleep(self, seconds):
        self.t += seconds


class Build6Tests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.inbox = self.root / "inbox"
        self.cell = self.root / "cell"
        self.nas = self.root / "nas"
        for path in (self.inbox, self.cell, self.nas):
            path.mkdir()
        self.before = read_cksum(estate_path(ROOT))

    def tearDown(self):
        self.assertEqual(read_cksum(estate_path(ROOT)), self.before)
        self.assertEqual(self.before, ESTATE_CKSUM)
        self.tmp.cleanup()

    def executable(self, path: Path) -> Path:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        path.chmod(path.stat().st_mode | stat.S_IEXEC)
        return path

    def write_json(self, path, obj):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(obj), encoding="utf-8")
        return path

    def job(self, job_id, config):
        nas = self.nas / job_id
        nas.mkdir(exist_ok=True)
        return {
            "id": job_id,
            "step": "eval",
            "config": str(config),
            "gpu": "ask",
            "max_minutes": 15,
            "nas_out": str(nas),
        }

    def approve(self, job_id):
        dest = self.inbox / "approve"
        dest.mkdir(parents=True, exist_ok=True)
        (dest / f"{job_id}.json").write_text(json.dumps({
            "decision": "yes", "by": "jason", "at": "2026-10-04T15:00:00-05:00",
        }), encoding="utf-8")

    def test_health_url_uses_the_same_port_the_cli_binds(self):
        spec = build_spec(
            {"api_port": 18080, "api_host": "127.0.0.1", "base": "b", "adapter": "a"},
            yaml_path=self.root / "serve.yaml",
        )
        self.assertEqual(spec["env"]["API_HOST"], "127.0.0.1")
        self.assertEqual(spec["env"]["API_PORT"], "18080")
        self.assertEqual(spec["url"], "http://127.0.0.1:18080")
        self.assertEqual(spec["health_url"], "http://127.0.0.1:18080/v1/models")
        self.assertEqual(spec["health_timeout"], 300)
        self.assertEqual(spec["health_poll"], 3)
        self.assertEqual(health_target(spec["url"]), spec["health_url"])
        self.assertEqual(health_target(spec["health_url"]), spec["health_url"])
        wide = bind_target("0.0.0.0", 8000)
        self.assertEqual(wide["env"]["API_HOST"], "0.0.0.0")
        self.assertEqual(wide["env"]["API_PORT"], "8000")
        self.assertEqual(wide["health_url"], "http://127.0.0.1:8000/v1/models")
        self.assertNotIn(":18080", wide["health_url"])
        tuned = build_spec({"health_timeout_s": 90, "base": "b", "adapter": "a"})
        self.assertEqual(tuned["health_timeout"], 90)

    def test_port_free_sees_a_listener_and_a_busy_port_does_not_start(self):
        sock = socket.socket()
        sock.bind(("127.0.0.1", 0))
        sock.listen(1)
        port = sock.getsockname()[1]
        try:
            self.assertFalse(port_free("127.0.0.1", port))
            self.assertFalse(port_free("0.0.0.0", port))
        finally:
            sock.close()
        self.assertTrue(port_free("127.0.0.1", port))

        spec = build_spec(
            {"api_port": 18081, "base": "b", "adapter": "a"},
            yaml_path=self.root / "live.yaml",
        )
        spec["log_path"] = str(self.root / "runs" / "job" / "serve.log")
        called = []

        def popen(argv, **kw):
            called.append(argv)
            raise AssertionError("a busy port must not start the server")

        with self.assertRaises(PortBusy) as caught:
            start_live(spec, popen=popen, port_check=lambda host, number: False)
        self.assertEqual(called, [])
        self.assertIn("already in use", str(caught.exception))
        self.assertNotIn("Traceback", str(caught.exception))
        note = Path(spec["log_path"]).read_text(encoding="utf-8")
        self.assertIn("already in use", note)
        self.assertFalse((self.root / "live.yaml").exists())

    def test_start_live_writes_stdout_and_stderr_to_the_serve_log(self):
        cli = self.executable(self.root / "bin" / "llamafactory-cli")
        spec = build_spec(
            {"llamafactory_cli": str(cli), "api_port": 18080, "base": "b", "adapter": "a"},
            yaml_path=self.root / "live.yaml",
            environ={},
            home=self.root,
        )
        spec["log_path"] = str(self.root / "runs" / "e2e-fm-run" / "serve.log")
        seen = {}

        class Proc:
            pid = 9

            def terminate(self):
                seen["term"] = True

            def wait(self, timeout=None):
                return 0

        def popen(argv, **kw):
            seen["stdout"] = kw["stdout"]
            seen["stderr"] = kw["stderr"]
            seen["env"] = kw["env"]
            kw["stdout"].write(b"Visit http://localhost:18080/docs\n")
            kw["stdout"].write(b"OSError: model not ready\n")
            return Proc()

        handle = start_live(spec, popen=popen, port_check=lambda host, number: True)
        self.assertIsNot(seen["stdout"], subprocess.DEVNULL)
        self.assertEqual(seen["stderr"], subprocess.STDOUT)
        self.assertEqual(seen["env"]["API_PORT"], "18080")
        self.assertEqual(seen["env"]["API_HOST"], "127.0.0.1")
        stop_live(handle)
        text = Path(spec["log_path"]).read_text(encoding="utf-8")
        self.assertIn("model not ready", text)
        self.assertIn("localhost:18080", text)
        self.assertTrue(spec["log_path"].endswith("runs/e2e-fm-run/serve.log"))

    def test_a_connection_error_waits_and_a_dead_process_stops_early(self):
        spec = build_spec(
            {"base": "b", "adapter": "a", "api_port": 18080, "health_timeout_s": 30},
            yaml_path=self.root / "serve.yaml",
        )
        spec["log_path"] = str(self.root / "runs" / "wait" / "serve.log")
        Path(spec["log_path"]).parent.mkdir(parents=True)
        Path(spec["log_path"]).write_text("still loading the 4B weights\n", encoding="utf-8")
        clock = Clock()
        calls = {"n": 0}

        def health(url):
            calls["n"] += 1
            self.assertEqual(url, "http://127.0.0.1:18080/v1/models")
            if calls["n"] < 3:
                raise urllib.error.URLError("connection refused")
            return True

        out = guarded_serve(
            spec,
            starter=lambda item: {"pid": 1},
            stopper=lambda handle: None,
            health=health,
            work=lambda: {"ok": True, "scored": True, "reason": "scored"},
            clock=clock,
            sleep=clock.sleep,
        )
        self.assertGreaterEqual(calls["n"], 3)
        self.assertTrue(out.get("scored"))
        self.assertGreaterEqual(clock.t, 6)
        self.assertLess(clock.t, 30)

        dead_spec = build_spec(
            {"base": "b", "adapter": "a", "health_timeout_s": 300},
            yaml_path=self.root / "dead.yaml",
        )
        dead_spec["log_path"] = str(self.root / "runs" / "dead" / "serve.log")
        Path(dead_spec["log_path"]).parent.mkdir(parents=True)
        Path(dead_spec["log_path"]).write_text(
            "Loading weights\nOSError: CUDA out of memory\n",
            encoding="utf-8",
        )

        class Dead:
            pid = 44

            def poll(self):
                return 2

        slept = []
        failed = guarded_serve(
            dead_spec,
            starter=lambda item: {"pid": 44, "proc": Dead()},
            stopper=lambda handle: None,
            health=lambda url: (_ for _ in ()).throw(AssertionError("health must not run")),
            work=lambda: {"ok": True, "scored": True},
            clock=lambda: 0,
            sleep=lambda seconds: slept.append(seconds),
        )
        self.assertEqual(slept, [])
        self.assertFalse(failed.get("scored"))
        self.assertIn("exited (2)", failed["reason"])
        self.assertIn("CUDA out of memory", failed["log_tail"])

        never = build_spec(
            {"base": "b", "adapter": "a", "serve_minutes": 15},
            yaml_path=self.root / "never.yaml",
        )
        never["log_path"] = str(self.root / "runs" / "never" / "serve.log")
        Path(never["log_path"]).parent.mkdir(parents=True)
        Path(never["log_path"]).write_text("URLError: [Errno 111] Connection refused\n", encoding="utf-8")
        slow = Clock()
        timed = guarded_serve(
            never,
            starter=lambda item: {"pid": 2},
            stopper=lambda handle: None,
            health=lambda url: (_ for _ in ()).throw(urllib.error.URLError("refused")),
            work=lambda: {"ok": True, "scored": True},
            clock=slow,
            sleep=slow.sleep,
        )
        self.assertFalse(timed.get("scored"))
        self.assertFalse(timed["capped"])
        self.assertGreaterEqual(slow.t, 300)
        self.assertLess(slow.t, 15 * 60)
        self.assertIn("300 seconds", timed["reason"])
        self.assertIn("Connection refused", timed["log_tail"])

    def test_reading_page_quotes_the_serve_log_on_failure(self):
        exam = self.root / "exam.jsonl"
        exam.write_text(json.dumps({
            "id": "q",
            "question": "Why do pottery wheels matter?",
            "answerable_because": "The wheel turns clay.",
        }) + "\n", encoding="utf-8")
        cli = self.executable(self.root / "bin" / "llamafactory-cli")
        cfg = self.write_json(self.root / "ev.json", {
            "exam": str(exam),
            "adapter": "/tmp/adapter",
            "base": "/tmp/base",
            "self_serve": True,
            "llamafactory_cli": str(cli),
            "api_port": 18080,
            "health_timeout_s": 6,
            "grade": "key-fact",
            "min_correct": 1,
        })

        class LoggingServer:
            def __init__(self):
                self.health_calls = 0
                self.stopped = False
                self.seen_url = ""

            def start(self, spec):
                path = Path(spec["log_path"])
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(
                    "loading weights\nURLError: [Errno 111] Connection refused\n",
                    encoding="utf-8",
                )
                self.log_path = path
                return {"pid": 8}

            def stop(self, handle):
                self.stopped = True

            def health(self, url):
                self.health_calls += 1
                self.seen_url = url
                raise urllib.error.URLError("connection refused")

        server = LoggingServer()
        clock = Clock()
        harness = Harness(
            self.inbox,
            self.cell,
            gpu_probe=lambda: dict(IDLE),
            server=server,
            clock=clock,
            sleep=clock.sleep,
        )
        harness.submit(self.job("e2e-fm-run", cfg))
        self.approve("e2e-fm-run")
        failed = harness.run("e2e-fm-run")
        self.assertEqual(failed["state"], "refused", failed.get("reason"))
        self.assertNotIn("crash", failed)
        self.assertIn("6 seconds", failed["reason"])
        self.assertNotIn("Traceback", failed["reason"])
        self.assertTrue(server.stopped)
        self.assertGreater(server.health_calls, 1)
        self.assertEqual(server.seen_url, "http://127.0.0.1:18080/v1/models")
        self.assertGreaterEqual(clock.t, 6)
        self.assertLess(clock.t, 15 * 60)
        log = self.nas / "e2e-fm-run" / "serve.log"
        self.assertTrue(log.is_file())
        self.assertIn("Connection refused", log.read_text(encoding="utf-8"))
        page = (self.nas / "e2e-fm-run" / "reading.md").read_text(encoding="utf-8")
        self.assertIn("Server log", page)
        self.assertIn("Connection refused", page)
        self.assertIn(str(log), page)
        self.assertIn("> ", page)
        receipt = (self.cell / "harness-job-e2e-fm-run" / "receipts" / "e2e-fm-run-serve.md").read_text(
            encoding="utf-8",
        )
        self.assertIn("http://127.0.0.1:18080/v1/models", receipt)
        self.assertIn("API_PORT: 18080", receipt)
        self.assertIn(str(log), receipt)
        self.assertIn("READY_FOR_LIVE_TEST: no", receipt)


if __name__ == "__main__":
    unittest.main()
