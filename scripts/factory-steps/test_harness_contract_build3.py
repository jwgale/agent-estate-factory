#!/usr/bin/env python3
"""CPU tests for the build-3 harness additions. Fakes only. No GPU. No train."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from harness_contract.common import ESTATE_CKSUM, estate_path, read_cksum, read_json  # noqa: E402
from harness_contract.eval_serve import build_spec, guarded_serve, may_serve, start_live  # noqa: E402
from harness_contract.exam_keyfact import grade_reply, score_pairs, score_saved  # noqa: E402
from harness_contract.runner import Harness  # noqa: E402

IDLE = lambda: {"state": "idle", "holder": None, "seen": True, "source": "test"}


class Clock:
    def __init__(self):
        self.t = 0.0

    def __call__(self):
        return self.t

    def sleep(self, seconds):
        self.t += seconds


class FakeServer:
    def __init__(self, *, healthy=True, boom=False):
        self.started = []
        self.stopped = []
        self.healthy = healthy
        self.boom = boom

    def start(self, spec):
        self.started.append(spec)
        return {"pid": 7}

    def stop(self, handle):
        self.stopped.append(handle)

    def health(self, url):
        if self.boom:
            raise OSError("health down")
        return self.healthy


class Build3Tests(unittest.TestCase):
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

    def harness(self, **kw):
        return Harness(
            self.inbox,
            self.cell,
            gpu_probe=kw.get("probe", IDLE),
            exam_ask=kw.get("exam_ask"),
            launch_runner=kw.get("launch_runner"),
            server=kw.get("server"),
            judge_ask=kw.get("judge_ask"),
            clock=kw.get("clock"),
            sleep=kw.get("sleep"),
        )

    def rows(self, name, lines):
        path = self.root / name
        path.write_text("".join(json.dumps(row) + "\n" for row in lines), encoding="utf-8")
        return path

    def write_json(self, path, obj):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(obj), encoding="utf-8")
        return path

    def job(self, job_id, step, config, gpu="forbid", minutes=15):
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

    def approve(self, job_id, decision="yes"):
        dest = self.inbox / "approve"
        dest.mkdir(parents=True, exist_ok=True)
        (dest / f"{job_id}.json").write_text(json.dumps({
            "decision": decision, "by": "jason", "at": "2026-10-04T14:00:00-05:00",
        }), encoding="utf-8")

    def test_echo_earns_nothing_and_refusal_earns_credit(self):
        row = {
            "id": "q1",
            "question": "Does the note name the river capital?",
            "answerable_because": "The note names Springfield after the council vote.",
            "refusal_ok": True,
            "must_not_say": ["Springfield is the state capital"],
        }
        echo = grade_reply(row, row["question"])
        self.assertFalse(echo["hit"])
        self.assertTrue(echo["derived"])
        self.assertTrue(echo["key_facts"])
        refusal = grade_reply(row, "I don't have access to that note, so I can't answer.")
        self.assertTrue(refusal["hit"])
        self.assertEqual(refusal["reason"], "refusal")
        banned = grade_reply(row, "Springfield is the state capital, after the council vote.")
        self.assertFalse(banned["hit"])
        self.assertEqual(banned["reason"], "must-not-say")
        stated = grade_reply(
            {**row, "refusal_ok": False, "key_facts": ["Springfield", "council vote"]},
            "The council vote named Springfield.",
        )
        self.assertFalse(stated["derived"])
        self.assertTrue(stated["hit"])
        pairs = score_pairs([(row, row["question"]), (row, "I cannot see the note.")])
        self.assertEqual(pairs["overlap_score"].count("/"), 1)
        self.assertIn("Overlap", pairs["reason"])
        self.assertTrue(pairs["facts_derived"])

    def test_judge_records_one_sentence(self):
        row = {
            "id": "j",
            "question": "Where is the spare key kept?",
            "answerable_because": "The spare key is in the blue tin.",
            "key_facts": ["blue tin"],
        }

        def judge(_prompt):
            return '{"hit": false, "reason": "The reply never names the tin."}'

        scored = score_pairs([(row, "I don't know.")], judge=judge, use_judge=True)
        self.assertEqual(scored["method"], "judge")
        self.assertFalse(scored["items"][0]["hit"])
        self.assertEqual(scored["items"][0]["judge_reason"], "The reply never names the tin.")
        self.assertIn("overlap_hit", scored["items"][0])
        saved_exam = self.rows("exam.jsonl", [row])
        saved_replies = self.rows("replies.jsonl", [{"id": "j", "reply": "The spare key is in the blue tin."}])
        disk = score_saved(saved_exam, saved_replies)
        self.assertEqual(disk["score"], "1/1")
        self.assertEqual(disk["overlap_score"], "1/1")

    def test_serve_stops_on_failure_and_does_not_start_when_unseen(self):
        spec = build_spec({
            "base": "/tmp/base",
            "adapter": "/tmp/adapter",
            "template": "qwen3_5",
            "serve_minutes": 15,
        }, yaml_path=self.root / "serve.yaml")
        self.assertEqual(spec["yaml"]["infer_backend"], "huggingface")
        self.assertFalse(spec["yaml"]["enable_thinking"])
        self.assertEqual(spec["yaml"]["max_new_tokens"], 256)
        self.assertEqual(spec["max_minutes"], 15)
        self.assertIn("llamafactory-cli", spec["command"])
        blocked, why = may_serve({"state": "idle", "seen": True, "source": "test"}, fake_server=False)
        self.assertFalse(blocked)
        self.assertIn("nvidia-smi", why)
        idle, _ = may_serve({"state": "idle", "seen": True, "source": "nvidia-smi"}, fake_server=False)
        self.assertTrue(idle)

        server = FakeServer(boom=True)
        clock = Clock()

        def work():
            raise RuntimeError("score blew up")

        out = guarded_serve(
            spec, starter=server.start, stopper=server.stop, health=server.health,
            work=work, clock=clock, sleep=clock.sleep,
        )
        self.assertTrue(server.started)
        self.assertTrue(server.stopped)
        self.assertTrue(out["stopped"])
        self.assertFalse(out.get("scored"))

        quiet = FakeServer()
        calls = []

        def no_start(spec_in):
            calls.append(spec_in)
            raise AssertionError("starter must not run when the probe is not nvidia-smi")

        self.assertFalse(may_serve({"state": "busy", "seen": True, "source": "nvidia-smi"}, fake_server=True)[0])
        self.assertEqual(calls, [])
        self.assertFalse(quiet.started)

    def test_cap_stops_the_server(self):
        spec = build_spec({
            "serve_minutes": 15,
            "adapter": "a",
            "base": "b",
            "health_timeout_s": 20 * 60,
        })
        clock = Clock()
        server = FakeServer(healthy=False)
        out = guarded_serve(
            spec, starter=server.start, stopper=server.stop, health=server.health,
            work=lambda: {"ok": True, "scored": True}, clock=clock, sleep=clock.sleep,
        )
        self.assertTrue(out["capped"])
        self.assertTrue(out["stopped"])
        self.assertTrue(server.stopped)
        self.assertGreaterEqual(clock.t, 15 * 60)

    def test_start_live_uses_a_fake_popen(self):
        spec = build_spec(
            {"base": "base", "adapter": "adapter", "api_port": 18080},
            yaml_path=self.root / "live.yaml",
        )
        seen = {}

        class Proc:
            pid = 4

            def terminate(self):
                seen["term"] = True

            def wait(self, timeout=None):
                return 0

        def popen(argv, **kw):
            seen["argv"] = argv
            seen["env"] = kw["env"]
            return Proc()

        handle = start_live(spec, popen=popen)
        self.assertEqual(handle["pid"], 4)
        self.assertTrue(seen["argv"][0].endswith("llamafactory-cli"))
        self.assertNotEqual(seen["argv"][0], "llamafactory-cli")
        self.assertNotIn("CUDA_VISIBLE_DEVICES", seen["env"])
        self.assertEqual(seen["env"]["API_PORT"], "18080")
        text = (self.root / "live.yaml").read_text(encoding="utf-8")
        self.assertIn("qwen3_5", text)
        self.assertIn("huggingface", text)

    def test_eval_self_serve_stops_and_refuses_a_test_probe(self):
        exam = self.rows("exam.jsonl", [{
            "id": "q",
            "question": "Why do pottery wheels matter in clinic?",
            "answerable_because": "The wheel turns clay into a bowl.",
        }])
        cfg = self.write_json(self.root / "ev.json", {
            "exam": str(exam),
            "adapter": "/tmp/adapter",
            "base": "/tmp/base",
            "self_serve": True,
            "grade": "key-fact",
            "serve_minutes": 15,
            "min_correct": 1,
        })
        bare = self.harness()
        bare.submit(self.job("ev-lock", "eval", cfg, gpu="ask"))
        self.approve("ev-lock")
        locked = bare.run("ev-lock")
        self.assertEqual(locked["state"], "refused")
        self.assertIn("nvidia-smi", locked["reason"])

        server = FakeServer()
        h = self.harness(server=server, exam_ask=lambda question, row: row["answerable_because"])
        h.submit(self.job("ev-go", "eval", cfg, gpu="ask"))
        self.approve("ev-go")
        passed = h.run("ev-go")
        self.assertEqual(passed["state"], "succeeded")
        self.assertTrue(server.started)
        self.assertTrue(server.stopped)
        score = read_json(self.cell / "harness-job-ev-go" / "score.json")
        self.assertIn("overlap_score", score)
        self.assertEqual(score["method"], "key-fact")
        receipt = (self.cell / "harness-job-ev-go" / "receipts" / "ev-go-serve-stop.md").read_text(encoding="utf-8")
        self.assertIn("stopped: True", receipt)
        self.assertIn("READY_FOR_LIVE_TEST: no", receipt)

    def test_one_approve_carries_the_run_and_stops_on_a_failed_gate(self):
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
        cfg = self.write_json(self.root / "run.json", {
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
        calls = []
        server = FakeServer()

        def launcher(command):
            calls.append(command)
            return {"rc": 0, "result": "ok", "output": "train: pid 9 ended rc=0 result=ok"}

        h = self.harness(
            launch_runner=launcher,
            server=server,
            exam_ask=lambda question, row: row["answerable_because"],
        )
        refused = h.submit(self.job("run-forbid", "run", cfg, gpu="forbid"))
        self.assertNotEqual(refused.get("state"), "queued")
        h.submit(self.job("run-wait", "run", cfg, gpu="ask"))
        waiting = h.run("run-wait")
        self.assertEqual(waiting["state"], "needs-approval")
        self.assertEqual(calls, [])
        self.approve("run-go")
        h.submit(self.job("run-go", "run", cfg, gpu="ask"))
        passed = h.run("run-go")
        self.assertEqual(passed["state"], "succeeded", passed.get("reason"))
        self.assertEqual(calls, ["scripts/train-step.sh recipe.yaml --wait --max-minutes 15"])
        self.assertTrue(server.started)
        self.assertTrue(server.stopped)
        page = (self.nas / "run-go" / "reading.md").read_text(encoding="utf-8")
        self.assertIn("reading note", page)
        train_receipt = (self.cell / "harness-job-run-go" / "receipts" / "run-go-train.md").read_text(encoding="utf-8")
        self.assertIn("run approve file", train_receipt)
        self.assertIn("READY_FOR_LIVE_TEST: no", train_receipt)
        self.assertTrue((self.cell / "harness-job-run-go" / "receipts" / "run-go-gap.md").is_file())
        self.assertTrue((self.cell / "harness-job-run-go" / "receipts" / "run-go-decontaminate.md").is_file())

        calls.clear()
        server2 = FakeServer()
        seq = iter([
            {"state": "idle", "holder": None, "seen": True, "source": "test"},
            {"state": "busy", "holder": "other-job", "seen": True, "source": "test"},
        ])
        h2 = self.harness(probe=lambda: next(seq), launch_runner=launcher, server=server2)
        h2.submit(self.job("run-busy", "run", cfg, gpu="ask"))
        self.approve("run-busy")
        held = h2.run("run-busy")
        self.assertEqual(held["state"], "refused")
        self.assertEqual(calls, [])
        self.assertFalse(server2.started)

        bad = self.write_json(self.root / "run-bad.json", {
            "dataset": str(self.rows("empty.jsonl", [])),
            "labels": ["a"],
            "label_field": "label",
            "exam_shape": "missing",
            "shape_field": "text",
            "exam": str(exam),
            "recipe": "recipe.yaml",
            "base": "base",
            "base_pin": str(receipt),
        })
        h3 = self.harness(launch_runner=launcher, server=FakeServer())
        h3.submit(self.job("run-gap", "run", bad, gpu="ask"))
        self.approve("run-gap")
        failed = h3.run("run-gap")
        self.assertNotEqual(failed["state"], "succeeded")
        self.assertEqual(calls, [])
        self.assertTrue((self.nas / "run-gap" / "reading.md").is_file())


if __name__ == "__main__":
    unittest.main()
