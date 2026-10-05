#!/usr/bin/env python3
"""CPU tests for the build-2 job-contract fixes. Fakes only. No GPU. No train."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from harness_contract.common import ESTATE_CKSUM, estate_path, read_cksum, read_json  # noqa: E402
from harness_contract.exam_score import ask_local, score_exam  # noqa: E402
from harness_contract.gpu_probe import probe_nvidia_smi  # noqa: E402
from harness_contract.launch_lock import execute_recorded  # noqa: E402
from harness_contract.runner import Harness  # noqa: E402

IDLE = lambda: {"state": "idle", "holder": None, "seen": True, "source": "test"}
UNSEEN = lambda: {"state": "idle", "holder": None, "seen": False, "source": "skip"}


class Build2Tests(unittest.TestCase):
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
            trainer=kw.get("trainer"),
            exam_ask=kw.get("exam_ask"),
            launch_runner=kw.get("launch_runner"),
        )

    def rows(self, name, lines):
        path = self.root / name
        path.write_text("".join(json.dumps(row) + "\n" for row in lines), encoding="utf-8")
        return path

    def write_json(self, path, obj):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(obj), encoding="utf-8")
        return path

    def job(self, job_id, step, config, gpu="forbid", minutes=5):
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
            "decision": decision, "by": "jason", "at": "2026-10-04T13:00:00-05:00",
        }), encoding="utf-8")

    def test_queued_is_not_a_pass_and_waiting_is_not_refused(self):
        cfg = self.write_json(self.root / "c.json", {"dataset": "x"})
        h = self.harness()
        out = h.submit(self.job("q", "gap", cfg))
        self.assertEqual(out["result"], "queued")
        self.assertNotEqual(out["result"], "pass")
        card = read_json(self.inbox / "status" / "q.card.json")
        self.assertEqual(card["result"], "queued")
        status = read_json(self.inbox / "status" / "q.json")
        self.assertEqual(status["gpu"], "unknown")
        h.submit(self.job("w", "gap", cfg, gpu="ask", minutes=5))
        waited = h.run("w")
        self.assertEqual(waited["state"], "needs-approval")
        self.assertEqual(waited["result"], "needs-approval")
        self.assertNotEqual(waited["result"], "refused")
        unseen = self.harness(probe=UNSEEN)
        unseen.submit(self.job("u", "train", cfg, gpu="ask", minutes=5))
        self.approve("u")
        stopped = unseen.run("u")
        self.assertEqual(stopped["state"], "refused")
        gpu = read_json(self.inbox / "status" / "u.json")["gpu"]
        self.assertEqual(gpu, "unknown")
        self.assertNotIn(gpu, ("idle", "busy"))

    def test_sharegpt_and_question_are_read_and_empty_text_is_not_a_pass(self):
        train = self.rows("train.jsonl", [{
            "messages": [
                {"role": "user", "content": "Theme: Vitamin D (vitamin_d). What does the fact sheet say?"},
                {"role": "assistant", "content": "secret answer"},
            ],
        }])
        exam = self.rows("exam.jsonl", [{
            "id": "q1",
            "question": "Theme: Vitamin D (vitamin_d). What does the fact sheet say?",
            "answerable_because": "do not hash this",
        }])
        cfg = self.write_json(self.root / "de.json", {
            "train": str(train),
            "exam": str(exam),
            "field": "question",
            "train_field": "messages.user",
            "exam_field": "question",
        })
        h = self.harness()
        h.submit(self.job("de1", "decontaminate", cfg))
        hit = h.run("de1")
        self.assertEqual(hit["state"], "refused")
        self.assertIn("decontaminate.json", hit["reason"])
        card = read_json(self.inbox / "status" / "de1.card.json")
        names = [item["name"] for item in card["artifacts"]]
        self.assertIn("decontaminate.json", names)
        empty = self.rows("empty.jsonl", [{"id": "t", "note": "no text"}])
        cfg2 = self.write_json(self.root / "de-empty.json", {
            "train": str(empty),
            "exam": str(exam),
            "field": "question",
        })
        h.submit(self.job("de2", "decontaminate", cfg2))
        blank = h.run("de2")
        self.assertNotEqual(blank["state"], "succeeded")
        self.assertIn("empty", blank["reason"])
        page = (self.nas / "de1" / "reading.md").read_text(encoding="utf-8")
        self.assertIn("Remove the overlapping rows", page)
        clean = self.rows("clean.jsonl", [{
            "messages": [{"role": "user", "content": "unrelated pottery notes about clay wheels"}],
        }])
        cfg3 = self.write_json(self.root / "de-ok.json", {
            "train": str(clean),
            "exam": str(exam),
            "train_field": "messages.user",
            "exam_field": "question",
        })
        h.submit(self.job("de3", "decontaminate", cfg3))
        passed = h.run("de3")
        self.assertEqual(passed["state"], "succeeded")
        ok_page = (self.nas / "de3" / "reading.md").read_text(encoding="utf-8")
        self.assertNotIn("Remove the overlapping rows", ok_page)
        self.assertIn("Do not remove rows", ok_page)

    def test_gap_opens_exam_finds_contrast_and_reports_outside_rows(self):
        train = self.rows("train.jsonl", [
            {"messages": [{"role": "user", "content": "Theme: Vitamin D (vitamin_d).\nWhat dose is listed?"}]},
            {"messages": [{"role": "user", "content": "Theme: Iron (iron).\nWrite an education brief."}]},
            {"messages": [{"role": "user", "content": "Please analyze this raw 23andMe genome zip."}]},
        ])
        exam = self.rows("exam.jsonl", [
            {"id": "q1", "question": "Why do we look at 25-hydroxyvitamin D in clinic?"},
            {"id": "q2", "question": "What does the fact sheet say about iron?"},
        ])
        cfg = self.write_json(self.root / "gap.json", {
            "dataset": str(train),
            "labels": ["vitamin_d", "iron"],
            "label_field": "label",
            "exam_shape": "education brief",
            "shape_field": "text",
            "exam": str(exam),
        })
        h = self.harness()
        h.submit(self.job("g1", "gap", cfg))
        out = h.run("g1")
        self.assertEqual(out["state"], "failed")
        self.assertIn("exam file", out["reason"])
        self.assertIn("clinic", out["reason"])
        body = read_json(self.cell / "harness-job-g1" / "gap.json")
        self.assertTrue(body["exam_opened"])
        self.assertEqual(body["exam_questions"], 2)
        self.assertGreaterEqual(body["unknown"], 1)
        self.assertGreaterEqual(body["dose_asks"], 1)
        page = (self.nas / "g1" / "reading.md").read_text(encoding="utf-8")
        self.assertIn("outside the label list", page)
        self.assertIn("dose", page.lower())
        tied = self.rows("tie.jsonl", [
            {"label": "a", "text": "left side note"},
            {"label": "b", "text": "right side note"},
        ])
        cfg2 = self.write_json(self.root / "gap-tie.json", {
            "dataset": str(tied),
            "labels": ["a", "b"],
            "label_field": "label",
            "exam_shape": "note",
            "shape_field": "text",
        })
        h.submit(self.job("g2", "gap", cfg2))
        needs = h.run("g2")
        self.assertEqual(needs["state"], "failed")
        self.assertIn("Name one", needs["reason"])
        found = self.rows("found.jsonl", [
            {"label": "a", "text": "left note ANSWER: one"},
            {"label": "b", "text": "What dose is named in the note ANSWER: two"},
        ])
        cfg3 = self.write_json(self.root / "gap-found.json", {
            "dataset": str(found),
            "labels": ["a", "b"],
            "label_field": "label",
            "exam_shape": "ANSWER:",
            "shape_field": "text",
        })
        h.submit(self.job("g3", "gap", cfg3))
        passed = h.run("g3")
        self.assertEqual(passed["state"], "succeeded")
        self.assertIn("What dose", passed["reason"])

    def test_probe_is_read_only_and_does_not_call_nvidia_smi(self):
        calls = []

        def fake(args):
            calls.append(list(args))
            if "compute-apps" in args[0]:
                return ""
            return "0\n"

        idle = probe_nvidia_smi(run=fake)
        self.assertEqual(idle["state"], "idle")
        self.assertTrue(idle["seen"])
        self.assertEqual(idle["source"], "nvidia-smi")
        self.assertTrue(calls)
        self.assertNotIn("nvidia-smi", " ".join(calls[0][:1]))

        def busy(args):
            if "compute-apps" in args[0]:
                return "123, python, 100 MiB\n"
            return "4\n"

        held = probe_nvidia_smi(run=busy)
        self.assertEqual(held["state"], "busy")
        self.assertIn("python", held["holder"])

        def broken(_args):
            raise OSError("no device")

        unseen = probe_nvidia_smi(run=broken)
        self.assertFalse(unseen["seen"])
        self.assertEqual(unseen["state"], "unknown")

    def test_pinned_base_and_launch_flag_do_not_start_train_step(self):
        data = self.rows("ds.jsonl", [
            {"label": "a", "text": "left ANSWER: one CONTRAST"},
            {"label": "b", "text": "right ANSWER: two"},
            {"label": "a", "text": "more ANSWER: one"},
        ])
        exam = self.rows("ex.jsonl", [{"id": "e", "text": "unrelated clay notes"}])
        receipt = self.root / "slot90.md"
        receipt.write_text("Base: .cell/classify-base-cache/Qwen--Qwen3.5-4B\n", encoding="utf-8")
        calls = []

        def launcher(command):
            calls.append(command)
            self.fail("launch runner must not be the real train script in this test")
            return 0

        cfg = self.write_json(self.root / "tr.json", {
            "dataset": str(data),
            "labels": ["a", "b"],
            "label_field": "label",
            "exam_shape": "ANSWER:",
            "shape_field": "text",
            "contrast_marker": "CONTRAST",
            "exam": str(exam),
            "compare_field": "text",
            "recipe": "recipe.yaml",
            "base": ".cell/classify-base-cache/Qwen--Qwen3.5-4B",
            "base_pin": str(receipt),
        })
        h = self.harness(launch_runner=launcher)
        h.submit(self.job("tr-pin", "train", cfg, gpu="ask", minutes=12))
        self.approve("tr-pin")
        out = h.run("tr-pin")
        self.assertEqual(out["exit_code"], 39)
        self.assertEqual(calls, [])
        page = (self.cell / "harness-job-tr-pin" / "base-pick.md").read_text(encoding="utf-8")
        self.assertIn("No weights were loaded", page)
        self.assertIn(receipt.name, page)
        plan = read_json(self.cell / "harness-job-tr-pin" / "plan.json")
        self.assertIn("base-pin:receipt", plan["gates"])
        self.assertFalse(plan["allow_launch"])
        self.assertIn("train-step.sh", plan["command"])

        def started(command):
            calls.append(command)
            return 0

        body = json.loads(cfg.read_text())
        body["allow_launch"] = True
        cfg2 = self.write_json(self.root / "tr2.json", body)
        h2 = self.harness(launch_runner=started)
        h2.submit(self.job("tr-go", "train", cfg2, gpu="ask", minutes=12))
        self.approve("tr-go")
        launched = h2.run("tr-go")
        self.assertEqual(launched["state"], "succeeded")
        self.assertEqual(calls, ["scripts/train-step.sh recipe.yaml"])
        receipt_path = self.cell / "harness-job-tr-go" / "receipts" / "tr-go-launch.md"
        text = receipt_path.read_text(encoding="utf-8")
        self.assertIn("allow_launch", text)
        self.assertIn("READY_FOR_LIVE_TEST: no", text)
        self.assertIn("idle", text)
        locked = execute_recorded({"allow_launch": False, "command": "scripts/train-step.sh x"}, runner=started)
        self.assertFalse(locked["launched"])
        self.assertEqual(calls, ["scripts/train-step.sh recipe.yaml"])

    def test_eval_scores_exam_with_a_fake_server(self):
        exam = self.rows("exam.jsonl", [
            {
                "id": "q1",
                "question": "Why do we look at 25-hydroxyvitamin D?",
                "answerable_because": "dietary vitamin D is inert until the liver makes calcidiol",
            },
            {
                "id": "q2",
                "question": "What does the fact sheet say about iron?",
                "answerable_because": "iron carries oxygen in hemoglobin",
            },
        ])
        cfg = self.write_json(self.root / "ev.json", {
            "exam": str(exam),
            "adapter": "/tmp/adapter",
            "backend": "llamacpp",
            "min_correct": 2,
        })

        def ask(question, row):
            return row["answerable_because"]

        h = self.harness(exam_ask=ask)
        h.submit(self.job("ev-ok", "eval", cfg, gpu="ask", minutes=12))
        self.approve("ev-ok")
        passed = h.run("ev-ok")
        self.assertEqual(passed["state"], "succeeded")
        self.assertIn("2 of 2", passed["reason"])
        score = read_json(self.cell / "harness-job-ev-ok" / "score.json")
        self.assertEqual(score["score"], "2/2")
        card = read_json(self.inbox / "status" / "ev-ok.card.json")
        self.assertIn("score.json", [item["name"] for item in card["artifacts"]])
        reg = (self.inbox / "registry.jsonl").read_text(encoding="utf-8")
        self.assertIn('"ready":"no"', reg.replace(" ", ""))

        def miss(_question, _row):
            return "I do not know"

        h2 = self.harness(exam_ask=miss)
        h2.submit(self.job("ev-miss", "eval", cfg, gpu="ask", minutes=12))
        self.approve("ev-miss")
        failed = h2.run("ev-miss")
        self.assertEqual(failed["state"], "failed")
        self.assertTrue((self.cell / "harness-job-ev-miss" / "score.json").is_file())
        self.assertIn("0 of 2", failed["reason"])

        bare = self.write_json(self.root / "ev-dead.json", {"exam": str(exam)})
        h3 = self.harness()
        h3.submit(self.job("ev-dead", "eval", bare, gpu="ask", minutes=12))
        self.approve("ev-dead")
        dead = h3.run("ev-dead")
        self.assertEqual(dead["state"], "refused")
        self.assertIn("not a score", dead["reason"])

        seen = {}

        def poster(url, body, timeout):
            seen["url"] = url
            seen["body"] = body
            return {"content": "calcidiol"}

        text = ask_local("question", backend="llamacpp", url="http://127.0.0.1:9", poster=poster)
        self.assertEqual(text, "calcidiol")
        self.assertTrue(seen["url"].endswith("/completion"))
        got = score_exam(exam, lambda question, row: row["answerable_because"])
        self.assertEqual(got["score"], "2/2")
        self.assertTrue(got["server_started"])


if __name__ == "__main__":
    unittest.main()
