#!/usr/bin/env python3
"""CPU tests for the improve-data step. Fakes only. No GPU. No train."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from harness_contract.common import ESTATE_CKSUM, estate_path, read_cksum, sha256_file  # noqa: E402
from harness_contract.improve_data import apply_proposal, cause_of, propose  # noqa: E402
from harness_contract.runner import Harness  # noqa: E402

IDLE = lambda: {"state": "idle", "holder": None, "seen": True, "source": "test"}

QUESTION = "Why do pottery wheels matter in the Tuesday clinic after the council vote on oak desks?"
ANSWER = "The wheel turns clay into a bowl after the council vote on oak desks."


class Build7Tests(unittest.TestCase):
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
        )

    def rows(self, name, lines):
        path = self.root / name
        path.write_text("".join(json.dumps(row) + "\n" for row in lines), encoding="utf-8")
        return path

    def write_json(self, path, obj):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(obj), encoding="utf-8")
        return path

    def job(self, job_id, step, config, gpu="forbid"):
        nas = self.nas / job_id
        nas.mkdir(exist_ok=True)
        return {
            "id": job_id,
            "step": step,
            "config": str(config),
            "gpu": gpu,
            "max_minutes": 15,
            "nas_out": str(nas),
        }

    def approve(self, job_id, decision="yes"):
        dest = self.inbox / "approve"
        dest.mkdir(parents=True, exist_ok=True)
        (dest / f"{job_id}.json").write_text(json.dumps({
            "decision": decision, "by": "jason", "at": "2026-10-04T15:20:00-05:00",
        }), encoding="utf-8")

    def exam_row(self, **extra):
        row = {
            "id": "q1",
            "theme_id": "clay",
            "question": QUESTION,
            "answerable_because": ANSWER,
            "key_facts": ["council vote", "oak desks"],
            "refusal_ok": False,
            "must_not_say": ["prescribe 5000"],
        }
        row.update(extra)
        return row

    def train_row(self):
        return {
            "messages": [
                {"role": "system", "content": "Use only the source notes."},
                {"role": "user", "content": "Theme: Clay (clay).\nExcerpt: kiln shelves hold greenware overnight."},
                {"role": "assistant", "content": "The notes mention kiln shelves."},
            ]
        }

    def test_causes_cover_the_five_miss_kinds(self):
        row = self.exam_row()
        self.assertEqual(cause_of(row, "The clinic keeps paper charts in a drawer."), "missing fact")
        self.assertEqual(cause_of(row, "You should prescribe 5000 units today."), "wrong fact")
        self.assertEqual(
            cause_of(row, "I do not have access to that note."),
            "over-refused",
        )
        self.assertEqual(cause_of(row, QUESTION), "echoed question")
        refused = self.exam_row(refusal_ok=True)
        self.assertEqual(
            cause_of(refused, "The council vote named oak desks in the bowl."),
            "should-have-refused",
        )

    def test_proposal_does_not_edit_the_training_set_or_copy_the_exam(self):
        exam = self.rows("exam.jsonl", [self.exam_row()])
        train = self.rows("train.jsonl", [self.train_row()])
        score = self.write_json(self.root / "score.json", {"misses": ["q1"], "method": "key-fact"})
        replies = self.rows("replies.jsonl", [{"id": "q1", "reply": "The clinic keeps paper charts in a drawer."}])
        before = sha256_file(train)
        out = self.root / "improve"
        result = propose({
            "dataset": str(train),
            "exam": str(exam),
            "score": str(score),
            "replies": str(replies),
            "exam_field": "question",
            "train_field": "messages.user",
            "allow_cloud": False,
        }, out, job_id="t1")
        self.assertTrue(result["ok"], result["reason"])
        self.assertEqual(sha256_file(train), before)
        self.assertEqual(result["causes"]["missing fact"], 1)
        self.assertEqual(result["kept"], 1)
        kept = json.loads((out / "proposed-rows.jsonl").read_text(encoding="utf-8"))
        blob = json.dumps(kept)
        self.assertNotIn(QUESTION, blob)
        self.assertNotIn(ANSWER, blob)
        page = (out / "improve.md").read_text(encoding="utf-8")
        self.assertIn("missing fact: 1", page)
        self.assertIn("does not change the training set", page)
        self.assertIn("clay", page)

    def test_a_copied_exam_question_is_dropped(self):
        exam = self.rows("exam.jsonl", [self.exam_row()])
        train = self.rows("train.jsonl", [self.train_row()])
        score = self.write_json(self.root / "score.json", {"misses": ["q1"]})
        replies = self.rows("replies.jsonl", [{"id": "q1", "reply": "paper charts only"}])

        def copy_exam(miss, sources, system):
            return {"messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": miss["question"]},
                {"role": "assistant", "content": "A short teaching line about kiln shelves."},
            ]}

        out = self.root / "drop"
        result = propose({
            "dataset": str(train),
            "exam": str(exam),
            "score": str(score),
            "replies": str(replies),
        }, out, job_id="t2", drafter=copy_exam)
        self.assertEqual(result["kept"], 0)
        self.assertEqual(result["dropped"], 1)
        self.assertIn("exam question", result["dropped_why"][0]["why"])
        self.assertEqual((out / "proposed-rows.jsonl").read_text(encoding="utf-8"), "")

    def test_cloud_drafting_stays_off_and_a_local_ask_can_be_used(self):
        exam = self.rows("exam.jsonl", [self.exam_row()])
        train = self.rows("train.jsonl", [self.train_row()])
        score = self.write_json(self.root / "score.json", {"misses": ["q1"]})
        replies = self.rows("replies.jsonl", [{"id": "q1", "reply": "paper charts only"}])
        called = []

        def ask(prompt):
            called.append(prompt)
            return json.dumps({
                "user": "Theme clay. Practice from kiln notes only.",
                "assistant": "The source notes mention kiln shelves, not a dose.",
            })

        cloud = propose({
            "dataset": str(train),
            "exam": str(exam),
            "score": str(score),
            "replies": str(replies),
            "draft_url": "https://api.x.ai/v1",
        }, self.root / "cloud", job_id="cloud", ask=ask)
        self.assertEqual(called, [])
        self.assertIn("Cloud drafting is off", cloud["reason"] + (self.root / "cloud" / "improve.md").read_text(encoding="utf-8"))

        local = propose({
            "dataset": str(train),
            "exam": str(exam),
            "score": str(score),
            "replies": str(replies),
            "draft_url": "http://127.0.0.1:9",
        }, self.root / "local", job_id="local", ask=ask)
        self.assertEqual(len(called), 1)
        self.assertTrue(local["ok"], local["reason"])
        self.assertEqual(local["kept"], 1)

    def test_missing_replies_do_not_invent_rows(self):
        exam = self.rows("exam.jsonl", [self.exam_row()])
        train = self.rows("train.jsonl", [self.train_row()])
        score = self.write_json(self.root / "score.json", {"misses": ["q1"], "score": "0/1"})
        before = sha256_file(train)
        result = propose({
            "dataset": str(train),
            "exam": str(exam),
            "score": str(score),
        }, self.root / "none", job_id="none")
        self.assertFalse(result["ok"])
        self.assertEqual(result["kept"], 0)
        self.assertEqual(sha256_file(train), before)
        page = (self.root / "none" / "improve.md").read_text(encoding="utf-8")
        self.assertIn("replies file was not on disk", page)
        self.assertIn("clay", page)

    def test_apply_needs_a_yes_and_writes_a_new_folder(self):
        train = self.rows("train.jsonl", [self.train_row()])
        proposal = self.rows("proposed-rows.jsonl", [{
            "messages": [
                {"role": "user", "content": "Theme clay. Practice the missing behavior."},
                {"role": "assistant", "content": "The notes mention kiln shelves."},
            ]
        }])
        before = sha256_file(train)
        cfg = self.write_json(self.root / "apply.json", {
            "dataset": str(train),
            "proposal": str(proposal),
            "version_root": str(self.root / "versions"),
        })
        h = self.harness()
        submitted = h.submit(self.job("ap1", "improve-apply", cfg, gpu="forbid"))
        self.assertEqual(submitted["state"], "queued")
        waiting = h.run("ap1")
        self.assertEqual(waiting["state"], "needs-approval")
        self.assertFalse((self.root / "versions").exists())
        self.assertEqual(sha256_file(train), before)
        self.approve("ap1", "no")
        refused = h.run("ap1")
        self.assertEqual(refused["state"], "refused")
        self.assertIn("not edited", refused["reason"])
        self.assertEqual(sha256_file(train), before)

        h.submit(self.job("ap2", "improve-apply", cfg, gpu="forbid"))
        self.approve("ap2", "yes")
        applied = h.run("ap2")
        self.assertEqual(applied["state"], "succeeded", applied.get("reason"))
        self.assertEqual(sha256_file(train), before)
        new_file = self.root / "versions" / "ap2" / "train.jsonl"
        self.assertTrue(new_file.is_file())
        lines = [json.loads(line) for line in new_file.read_text(encoding="utf-8").splitlines() if line.strip()]
        self.assertEqual(len(lines), 2)
        receipt = (self.root / "versions" / "ap2" / "receipt.md").read_text(encoding="utf-8")
        self.assertIn("READY_FOR_LIVE_TEST: no", receipt)
        self.assertIn("original unchanged: true", receipt)
        direct = apply_proposal(
            {"dataset": str(train), "proposal": str(proposal), "version_root": str(self.root / "versions")},
            job_id="ap2",
            approved_by="jason",
            approved_at="2026-10-04T15:20:00-05:00",
        )
        self.assertFalse(direct["ok"])
        self.assertEqual(sha256_file(train), before)

    def test_run_can_write_a_proposal_after_eval_without_applying(self):
        train = self.rows("train.jsonl", [
            {"label": "a", "text": "left note about oak desks"},
            {"label": "b", "text": "What dose is named in the pine shelf note"},
        ])
        exam = self.rows("exam.jsonl", [self.exam_row()])
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
            "min_correct": 0,
            "improve": True,
        })
        before = sha256_file(train)

        class Server:
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

        calls = []

        def launcher(command):
            calls.append(command)
            return {"rc": 0, "result": "ok", "output": "train: pid 9 ended rc=0 result=ok"}

        server = Server()
        h = self.harness(
            launch_runner=launcher,
            server=server,
            exam_ask=lambda question, row: "The clinic keeps paper charts in a drawer.",
        )
        h.submit(self.job("run-imp", "run", cfg, gpu="ask"))
        self.approve("run-imp")
        passed = h.run("run-imp")
        self.assertEqual(passed["state"], "succeeded", passed.get("reason"))
        self.assertEqual(sha256_file(train), before)
        self.assertTrue((self.nas / "run-imp" / "improve" / "improve.md").is_file())
        self.assertTrue((self.nas / "run-imp" / "improve" / "proposed-rows.jsonl").is_file())
        self.assertTrue((self.cell / "harness-job-run-imp" / "replies.jsonl").is_file())
        self.assertFalse((self.root / "versions").exists())
        self.assertEqual(calls, ["scripts/train-step.sh recipe.yaml --wait --max-minutes 15"])
        self.assertTrue(server.stopped)
        page = (self.nas / "run-imp" / "improve" / "improve.md").read_text(encoding="utf-8")
        self.assertIn("missing fact: 1", page)
        self.assertIn("not edited", (self.nas / "run-imp" / "reading.md").read_text(encoding="utf-8").lower() + passed["reason"].lower())


if __name__ == "__main__":
    unittest.main()
