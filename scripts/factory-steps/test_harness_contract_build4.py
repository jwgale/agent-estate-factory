#!/usr/bin/env python3
"""CPU tests for the run-step wait fix. Fakes only. No GPU. No train."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from harness_contract.common import ESTATE_CKSUM, estate_path, read_cksum  # noqa: E402
from harness_contract.gpu_probe import classify_own_train, probe_nvidia_smi  # noqa: E402
from harness_contract.runner import Harness, parse_train_result, run_train_command  # noqa: E402

IDLE = {
    "state": "idle",
    "holder": None,
    "pids": [],
    "seen": True,
    "source": "test",
    "pid_alive": False,
}


class ProbeQueue:
    def __init__(self):
        self.items = []

    def __call__(self):
        if self.items:
            return self.items.pop(0)
        return dict(IDLE)

    def set(self, *items):
        self.items = [dict(item) for item in items]


class FakeServer:
    def __init__(self):
        self.started = []
        self.stopped = []

    def start(self, spec):
        self.started.append(spec)
        return {"pid": 7}

    def stop(self, handle):
        self.stopped.append(handle)

    def health(self, url):
        return True


class Build4Tests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.inbox = self.root / "inbox"
        self.cell = self.root / "cell"
        self.nas = self.root / "nas"
        for path in (self.inbox, self.cell, self.nas):
            path.mkdir()
        self.before = read_cksum(estate_path(ROOT))
        self.probes = ProbeQueue()
        self.calls = []
        self.mode = "wait"

    def tearDown(self):
        self.assertEqual(read_cksum(estate_path(ROOT)), self.before)
        self.assertEqual(self.before, ESTATE_CKSUM)
        self.tmp.cleanup()

    def harness(self, server=None):
        return Harness(
            self.inbox,
            self.cell,
            gpu_probe=self.probes,
            exam_ask=lambda question, row: row["answerable_because"],
            launch_runner=self.launcher,
            server=server,
        )

    def launcher(self, command):
        self.calls.append(command)
        if self.mode == "detached":
            return {"rc": 0, "output": "train: launched pid 4242 (pid file x, log y)"}
        if self.mode == "failed":
            return {
                "rc": 24,
                "result": "failed",
                "output": "train: pid 4242 ended rc=1 result=failed",
            }
        return {"rc": 0, "result": "ok", "output": "train: pid 4242 ended rc=0 result=ok"}

    def rows(self, name, lines):
        path = self.root / name
        path.write_text("".join(json.dumps(row) + "\n" for row in lines), encoding="utf-8")
        return path

    def write_json(self, path, obj):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(obj), encoding="utf-8")
        return path

    def job(self, job_id, config, minutes=45):
        nas = self.nas / job_id
        nas.mkdir(exist_ok=True)
        return {
            "id": job_id,
            "step": "run",
            "config": str(config),
            "gpu": "ask",
            "max_minutes": minutes,
            "nas_out": str(nas),
        }

    def approve(self, job_id):
        dest = self.inbox / "approve"
        dest.mkdir(parents=True, exist_ok=True)
        (dest / f"{job_id}.json").write_text(json.dumps({
            "decision": "yes", "by": "jason", "at": "2026-10-04T14:00:00-05:00",
        }), encoding="utf-8")

    def run_cfg(self):
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
        })

    def test_probe_tells_own_finished_pid_from_own_live_pid(self):
        def busy(args):
            if "compute-apps" in args[0]:
                return "4242, /home/jason/venvs/llamafactory/bin/python, 8000 MiB\n"
            return "40\n"

        held = probe_nvidia_smi(run=busy)
        self.assertEqual(held["pids"], [4242])
        self.assertIn("python", held["holder"])
        live = classify_own_train(held, 4242, alive=lambda pid: True)
        done = classify_own_train(held, 4242, alive=lambda pid: False)
        self.assertEqual(live["kind"], "own-live")
        self.assertTrue(live["blocks"])
        self.assertIn("still live", live["sentence"])
        self.assertIn("4242", live["sentence"])
        self.assertNotIn("bin/python", live["sentence"])
        self.assertEqual(done["kind"], "own-finished")
        self.assertFalse(done["blocks"])
        self.assertIn("has finished", done["sentence"])
        self.assertIn("not holding the card", done["sentence"])
        foreign = classify_own_train(
            {"state": "busy", "holder": "other-job", "pids": [99], "seen": True, "pid_alive": False},
            4242,
        )
        self.assertEqual(foreign["kind"], "foreign")
        self.assertTrue(foreign["blocks"])
        self.assertIn("has finished", foreign["sentence"])
        self.assertIn("not the holder", foreign["sentence"])

    def test_detached_train_stops_before_eval_and_wait_is_the_fix(self):
        self.assertEqual(
            run_train_command("recipe.yaml", 45),
            "scripts/train-step.sh recipe.yaml --wait --max-minutes 45",
        )
        detached = parse_train_result(0, "train: launched pid 4242 (pid file x)")
        self.assertFalse(detached["pass"])
        self.assertEqual(detached["pid"], 4242)
        waited = parse_train_result(0, "train: pid 4242 ended rc=0 result=ok")
        self.assertTrue(waited["pass"])
        failed = parse_train_result(0, "train: pid 4242 ended rc=0 result=failed")
        self.assertFalse(failed["pass"])

        cfg = self.run_cfg()
        server = FakeServer()
        h = self.harness(server)
        self.mode = "detached"
        h.submit(self.job("run-detach", cfg))
        self.approve("run-detach")
        stopped = h.run("run-detach")
        self.assertEqual(stopped["state"], "failed", stopped.get("reason"))
        self.assertEqual(stopped["exit_code"], 24)
        self.assertIn("--wait", self.calls[0])
        self.assertIn("--max-minutes 45", self.calls[0])
        self.assertIn("detached launch", stopped["reason"])
        self.assertFalse(server.started)
        page = (self.nas / "run-detach" / "reading.md").read_text(encoding="utf-8")
        self.assertIn("detached launch", page)
        self.assertIn("4242", page)
        self.assertIn("Eval did not start", page)

        self.mode = "failed"
        self.calls.clear()
        server2 = FakeServer()
        h2 = self.harness(server2)
        h2.submit(self.job("run-bad", cfg))
        self.approve("run-bad")
        bad = h2.run("run-bad")
        self.assertEqual(bad["state"], "failed")
        self.assertIn("result=failed", bad["reason"])
        self.assertFalse(server2.started)

        busy = {
            "state": "busy",
            "holder": "/home/jason/venvs/llamafactory/bin/python",
            "pids": [4242],
            "seen": True,
            "source": "test",
            "pid_alive": True,
        }
        self.mode = "wait"
        self.calls.clear()
        self.probes.set(IDLE, IDLE, busy)
        server3 = FakeServer()
        h3 = self.harness(server3)
        h3.submit(self.job("run-live", cfg))
        self.approve("run-live")
        held = h3.run("run-live")
        self.assertEqual(held["state"], "refused", held.get("reason"))
        self.assertEqual(held["exit_code"], 31)
        self.assertFalse(server3.started)
        live_page = (self.nas / "run-live" / "reading.md").read_text(encoding="utf-8")
        self.assertIn("own train pid 4242", live_page)
        self.assertIn("still live", live_page)
        self.assertNotIn("bin/python", live_page)
        self.assertIn("--wait --max-minutes 45", self.calls[0])

        stale = live_page
        (self.nas / "run-live" / "reading.md").write_text("STALE\n" + stale, encoding="utf-8")
        self.probes.set(IDLE, IDLE, busy)
        self.calls.clear()
        again = h3.run("run-live")
        self.assertNotIn("replayed", again)
        self.assertEqual(again["exit_code"], 31)
        self.assertFalse(server3.started)
        fresh = (self.nas / "run-live" / "reading.md").read_text(encoding="utf-8")
        self.assertNotIn("STALE", fresh)
        self.assertIn("still live", fresh)
        self.assertTrue(self.calls)

        self.calls.clear()
        self.probes.set()
        server4 = FakeServer()
        h4 = self.harness(server4)
        fixed = h4.run("run-live")
        self.assertEqual(fixed["state"], "succeeded", fixed.get("reason"))
        self.assertNotIn("replayed", fixed)
        self.assertTrue(server4.started)
        self.assertTrue(server4.stopped)
        fixed_page = (self.nas / "run-live" / "reading.md").read_text(encoding="utf-8")
        self.assertIn("has finished", fixed_page)
        self.assertIn("4242", fixed_page)
        self.assertNotIn("still live", fixed_page)
        self.assertIn("reading note", fixed_page)
        self.assertIn("--wait --max-minutes 45", self.calls[0])

        n = len(self.calls)
        replayed = h4.run("run-live")
        self.assertTrue(replayed.get("replayed"))
        self.assertEqual(len(self.calls), n)


if __name__ == "__main__":
    unittest.main()
