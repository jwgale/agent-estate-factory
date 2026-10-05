#!/usr/bin/env python3
"""CPU tests for CLI resolution and a crashed step. Fakes only. No GPU. No train."""

from __future__ import annotations

import json
import stat
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from harness_contract.common import ESTATE_CKSUM, estate_path, read_cksum, read_json, read_jsonl  # noqa: E402
from harness_contract.eval_serve import (  # noqa: E402
    CliMissing,
    build_spec,
    guarded_serve,
    resolve_cli,
    start_live,
)
from harness_contract.runner import Harness, can_rerun  # noqa: E402

IDLE = {
    "state": "idle",
    "holder": None,
    "seen": True,
    "source": "test",
    "pid_alive": False,
}
NVIDIA_IDLE = {
    "state": "idle",
    "holder": None,
    "seen": True,
    "source": "nvidia-smi",
    "pid_alive": False,
}


class BoomServer:
    def __init__(self):
        self.started = []
        self.stops = 0
        self.stopped_ok = False
        self.blow = True

    def start(self, spec):
        self.started.append(spec)
        return {"pid": 9}

    def stop(self, handle):
        self.stops += 1
        if self.blow and self.stops == 1:
            raise RuntimeError("stop failed once")
        self.stopped_ok = True

    def health(self, url):
        if self.blow:
            raise RuntimeError("health blew up after start")
        return True


class NoStart:
    def start(self, spec):
        raise AssertionError("the server must not start")

    def stop(self, handle):
        raise AssertionError("stop must not run")

    def health(self, url):
        raise AssertionError("health must not run")


class Build5Tests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.inbox = self.root / "inbox"
        self.cell = self.root / "cell"
        self.nas = self.root / "nas"
        for path in (self.inbox, self.cell, self.nas):
            path.mkdir()
        self.before = read_cksum(estate_path(ROOT))
        self.calls = []

    def tearDown(self):
        self.assertEqual(read_cksum(estate_path(ROOT)), self.before)
        self.assertEqual(self.before, ESTATE_CKSUM)
        self.tmp.cleanup()

    def executable(self, path: Path) -> Path:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        path.chmod(path.stat().st_mode | stat.S_IEXEC)
        return path

    def harness(self, server=None, probe=None):
        return Harness(
            self.inbox,
            self.cell,
            gpu_probe=probe or (lambda: dict(IDLE)),
            exam_ask=lambda question, row: row["answerable_because"],
            launch_runner=self.launcher,
            server=server,
        )

    def launcher(self, command):
        self.calls.append(command)
        if self.calls and getattr(self, "blow_train", False):
            raise RuntimeError("train step blew up")
        return {"rc": 0, "result": "ok", "output": "train: pid 4242 ended rc=0 result=ok"}

    def rows(self, name, lines):
        path = self.root / name
        path.write_text("".join(json.dumps(row) + "\n" for row in lines), encoding="utf-8")
        return path

    def write_json(self, path, obj):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(obj), encoding="utf-8")
        return path

    def job(self, job_id, step, config, gpu="ask", minutes=15):
        nas = self.nas / job_id
        nas.mkdir(exist_ok=True)
        return {
            "id": job_id,
            "step": step,
            "config": str(config),
            "gpu": gpu,
            "max_minutes": minutes,
            "nas_out": str(nas),
        }

    def approve(self, job_id):
        dest = self.inbox / "approve"
        dest.mkdir(parents=True, exist_ok=True)
        (dest / f"{job_id}.json").write_text(json.dumps({
            "decision": "yes", "by": "jason", "at": "2026-10-04T14:40:00-05:00",
        }), encoding="utf-8")

    def run_cfg(self, cli: Path):
        train = self.rows("train.jsonl", [
            {"label": "a", "text": "left note ANSWER: one about oak desks"},
            {"label": "b", "text": "What dose is named ANSWER: two about pine shelves"},
        ])
        exam = self.rows("exam.jsonl", [{
            "id": "q",
            "question": "Why do pottery wheels matter in clinic?",
            "answerable_because": "The wheel turns clay into a bowl after the council vote.",
        }])
        receipt = self.root / "slot90.md"
        receipt.write_text("Base: .cell/classify-base-cache/Qwen--Qwen3.5-4B\n", encoding="utf-8")
        return self.write_json(self.root / "run.json", {
            "dataset": str(train),
            "labels": ["a", "b"],
            "label_field": "label",
            "exam_shape": "pottery",
            "shape_field": "text",
            "exam": str(exam),
            "compare_field": "text",
            "exam_field": "question",
            "recipe": "recipe.yaml",
            "base": ".cell/classify-base-cache/Qwen--Qwen3.5-4B",
            "base_pin": str(receipt),
            "adapter": "/tmp/adapter",
            "self_serve": True,
            "grade": "key-fact",
            "serve_minutes": 15,
            "min_correct": 1,
            "llamafactory_cli": str(cli),
        })

    def test_cli_resolves_and_a_missing_cli_is_a_gate(self):
        home = self.root / "home"
        sibling_py = home / "q" / "bin" / "python"
        env_cli = home / "env" / "bin" / "llamafactory-cli"
        cfg_cli = self.executable(home / "cfg" / "bin" / "llamafactory-cli")
        chosen = resolve_cli(
            {"llamafactory_cli": str(cfg_cli)},
            environ={
                "LLAMAFACTORY_CLI": str(env_cli),
                "QUANT_LADDER_PYTHON": str(sibling_py),
            },
            home=home,
        )
        self.assertEqual(chosen["source"], "config")
        self.assertEqual(chosen["path"], str(cfg_cli))
        self.assertIsNone(chosen["gate"])

        alias = resolve_cli({"cli": "~/cfg/bin/llamafactory-cli"}, environ={}, home=home)
        self.assertEqual(alias["path"], str(cfg_cli))
        self.assertEqual(alias["source"], "config")

        from_env = resolve_cli(
            {},
            environ={
                "LLAMAFACTORY_CLI": "~/env/bin/llamafactory-cli",
                "QUANT_LADDER_PYTHON": str(sibling_py),
            },
            home=home,
        )
        self.assertEqual(from_env["source"], "env")
        self.assertEqual(from_env["path"], str(env_cli))
        self.assertIn("not executable", from_env["gate"])

        sibling = resolve_cli({}, environ={"QUANT_LADDER_PYTHON": str(sibling_py)}, home=home)
        self.assertEqual(sibling["source"], "sibling")
        self.assertEqual(sibling["path"], str(home / "q" / "bin" / "llamafactory-cli"))

        default = resolve_cli({}, environ={}, home=home)
        self.assertEqual(default["source"], "default")
        self.assertEqual(
            default["path"],
            str(home / "venvs" / "llamafactory" / "bin" / "llamafactory-cli"),
        )
        self.assertNotEqual(default["path"], "llamafactory-cli")
        self.assertIn("not executable", default["gate"])
        self.assertNotIn("Traceback", default["gate"])
        self.assertNotIn("FileNotFoundError", default["gate"])

        missing = self.root / "missing-cli"
        spec = build_spec(
            {"llamafactory_cli": str(missing), "base": "b", "adapter": "a"},
            yaml_path=self.root / "serve.yaml",
            environ={},
            home=home,
        )
        self.assertEqual(spec["argv"][0], str(missing))
        self.assertNotEqual(spec["argv"][0], "llamafactory-cli")
        self.assertIn("not executable", spec["cli_gate"])
        called = []
        out = guarded_serve(
            spec,
            starter=lambda item: called.append(item) or {"pid": 1},
            stopper=lambda handle: None,
            health=lambda url: True,
            work=lambda: {"ok": True, "scored": True},
            clock=lambda: 0,
            sleep=lambda _n: None,
        )
        self.assertEqual(called, [])
        self.assertFalse(out["started"])
        self.assertIn("not executable", out["reason"])
        with self.assertRaises(CliMissing) as caught:
            start_live(spec)
        self.assertIn("not executable", str(caught.exception))
        self.assertNotIsInstance(caught.exception, FileNotFoundError)
        self.assertFalse((self.root / "serve.yaml").exists())

        exam = self.rows("gate-exam.jsonl", [{
            "id": "q",
            "question": "Why do pottery wheels matter?",
            "answerable_because": "The wheel turns clay.",
        }])
        cfg = self.write_json(self.root / "gate.json", {
            "exam": str(exam),
            "adapter": "/tmp/adapter",
            "base": "/tmp/base",
            "self_serve": True,
            "llamafactory_cli": str(missing),
        })
        quiet = NoStart()
        h = self.harness(server=quiet, probe=lambda: dict(NVIDIA_IDLE))
        h.submit(self.job("cli-gate", "eval", cfg))
        self.approve("cli-gate")
        stale = self.inbox / "status" / "cli-gate.json"
        stale.write_text(json.dumps({
            "state": "refused",
            "reason": "from an earlier attempt",
            "step": "run",
        }), encoding="utf-8")
        (self.nas / "cli-gate" / "reading.md").write_text("STALE earlier refuse\n", encoding="utf-8")
        gated = h.run("cli-gate")
        self.assertEqual(gated["state"], "refused", gated.get("reason"))
        self.assertNotIn("replayed", gated)
        self.assertNotIn("crashed", gated["reason"])
        self.assertNotIn("Traceback", gated["reason"])
        self.assertIn("not executable", gated["reason"])
        self.assertIn(str(missing), gated["reason"])
        status = read_json(self.inbox / "status" / "cli-gate.json")
        card = read_json(self.inbox / "status" / "cli-gate.card.json")
        page = (self.nas / "cli-gate" / "reading.md").read_text(encoding="utf-8")
        outbox = read_jsonl(self.inbox / "outbox.jsonl")
        self.assertEqual(status["state"], "refused")
        self.assertNotIn("earlier attempt", status["reason"])
        self.assertIn("not executable", status["reason"])
        self.assertEqual(card["result"], "refused")
        self.assertIn("not executable", card["reason"])
        self.assertIn("not executable", outbox[-1]["reason"])
        self.assertNotIn("STALE", page)
        self.assertIn("not executable", page)
        self.assertIn("What we tried", page)

    def test_step_crash_writes_failed_stops_the_server_and_can_run_again(self):
        self.assertFalse(can_rerun({"state": "failed", "reason": "a gate"}))
        self.assertFalse(can_rerun({"state": "succeeded"}))
        self.assertTrue(can_rerun({"state": "refused"}))
        self.assertTrue(can_rerun({"state": "failed", "crash": True}))

        zero = self.rows("zero.jsonl", [{"label": "a", "text": "hello ANSWER: yes"}])
        gap_cfg = self.write_json(self.root / "gap-zero.json", {
            "dataset": str(zero),
            "labels": ["a", "b"],
            "label_field": "label",
            "exam_shape": "ANSWER:",
            "shape_field": "text",
        })
        gap = self.harness()
        gap.submit(self.job("gap-stay", "gap", gap_cfg, gpu="forbid", minutes=0))
        stayed = gap.run("gap-stay")
        self.assertEqual(stayed["state"], "failed")
        self.assertNotIn("crash", read_json(self.inbox / "status" / "gap-stay.json"))
        again = gap.run("gap-stay")
        self.assertTrue(again.get("replayed"))

        cli = self.executable(self.root / "bin" / "llamafactory-cli")
        cfg = self.run_cfg(cli)
        self.blow_train = True
        train_h = self.harness(server=BoomServer())
        train_h.submit(self.job("run-train-boom", "run", cfg, minutes=45))
        self.approve("run-train-boom")
        trained = train_h.run("run-train-boom")
        self.assertEqual(trained["state"], "failed", trained.get("reason"))
        self.assertTrue(trained.get("crash"))
        self.assertIn("Step train crashed", trained["reason"])
        self.assertIn("RuntimeError", trained["reason"])
        self.assertFalse(train_h.server.started)
        self.assertEqual(read_json(self.inbox / "status" / "run-train-boom.json")["step"], "train")
        rerun_train = train_h.run("run-train-boom")
        self.assertNotIn("replayed", rerun_train)
        self.assertEqual(rerun_train["state"], "failed")

        self.blow_train = False
        self.calls.clear()
        server = BoomServer()
        h = self.harness(server)
        h.submit(self.job("run-crash", "run", cfg, minutes=45))
        self.approve("run-crash")
        (self.inbox / "status" / "run-crash.json").write_text(json.dumps({
            "state": "refused",
            "reason": "from an earlier attempt",
            "step": "run",
        }), encoding="utf-8")
        (self.nas / "run-crash" / "reading.md").write_text("STALE earlier refuse\n", encoding="utf-8")
        crashed = h.run("run-crash")
        self.assertEqual(crashed["state"], "failed", crashed.get("reason"))
        self.assertEqual(crashed["exit_code"], 30)
        self.assertTrue(crashed.get("crash"))
        self.assertIn("Step eval crashed", crashed["reason"])
        self.assertIn("stop failed once", crashed["reason"])
        self.assertNotIn("Traceback", crashed["reason"])
        self.assertNotIn("earlier attempt", crashed["reason"])
        self.assertTrue(server.started)
        self.assertEqual(server.started[0]["argv"][0], str(cli))
        self.assertNotEqual(server.started[0]["argv"][0], "llamafactory-cli")
        self.assertTrue(server.stopped_ok)
        self.assertGreaterEqual(server.stops, 2)

        status = read_json(self.inbox / "status" / "run-crash.json")
        card = read_json(self.inbox / "status" / "run-crash.card.json")
        page = (self.nas / "run-crash" / "reading.md").read_text(encoding="utf-8")
        outbox = read_jsonl(self.inbox / "outbox.jsonl")
        self.assertEqual(status["state"], "failed")
        self.assertEqual(status["step"], "eval")
        self.assertTrue(status.get("crash"))
        self.assertIn("Step eval crashed", status["reason"])
        self.assertEqual(card["result"], "fail")
        self.assertIn("Step eval crashed", card["reason"])
        self.assertEqual(outbox[-1]["result"], "fail")
        self.assertIn("Step eval crashed", outbox[-1]["reason"])
        flat = " ".join(page.split())
        self.assertNotIn("STALE", page)
        self.assertIn("Step eval crashed", flat)
        self.assertIn("Jason needs to look", flat)
        self.assertIn("stopped the server", flat)

        server.blow = True
        held = h.run("run-crash")
        self.assertNotIn("replayed", held)
        self.assertEqual(held["state"], "failed")
        self.assertTrue(held.get("crash"))

        server.blow = False
        fixed = h.run("run-crash")
        self.assertEqual(fixed["state"], "succeeded", fixed.get("reason"))
        self.assertNotIn("replayed", fixed)
        self.assertFalse(read_json(self.inbox / "status" / "run-crash.json").get("crash"))
        n = len(self.calls)
        replayed = h.run("run-crash")
        self.assertTrue(replayed.get("replayed"))
        self.assertEqual(len(self.calls), n)


if __name__ == "__main__":
    unittest.main()
