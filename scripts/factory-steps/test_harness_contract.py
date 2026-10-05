#!/usr/bin/env python3
"""CPU tests for the harness job contract. No GPU, no trainer launch, no network."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from harness_contract.common import (  # noqa: E402
    CARD_MAX,
    ESTATE_CKSUM,
    STATUS_MAX,
    estate_path,
    read_cksum,
    read_json,
    read_jsonl,
)
from harness_contract.digest import note_cloud_status  # noqa: E402
from harness_contract.failures import append_miss  # noqa: E402
from harness_contract.judge import judge  # noqa: E402
from harness_contract.local_first import cloud_read  # noqa: E402
from harness_contract.prepare_cards import write_preference, write_refusal  # noqa: E402
from harness_contract.rewrite import claim  # noqa: E402
from harness_contract.runner import Harness  # noqa: E402

IDLE = lambda: {"state": "idle", "holder": None, "seen": True}
BUSY = lambda: {"state": "busy", "holder": "other-job", "seen": True}
UNSEEN = lambda: {"state": "idle", "holder": None, "seen": False}

LONG = (
    "alpha bravo charlie delta echo foxtrot golf hotel india juliet "
    "kilo lima mike november oscar papa quebec romeo sierra tango"
)


class ContractTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.inbox = self.root / "inbox"
        self.cell = self.root / "cell"
        self.nas = self.root / "nas"
        self.inbox.mkdir()
        self.cell.mkdir()
        self.nas.mkdir()
        self.estate_before = read_cksum(estate_path(ROOT))

    def tearDown(self):
        self.assertEqual(read_cksum(estate_path(ROOT)), self.estate_before)
        self.assertEqual(self.estate_before, ESTATE_CKSUM)
        self.tmp.cleanup()

    def harness(self, **kw):
        return Harness(
            self.inbox,
            self.cell,
            gpu_probe=kw.get("probe", IDLE),
            trainer=kw.get("trainer"),
            seat=kw.get("seat"),
            caller=kw.get("caller", "local"),
        )

    def write_json(self, path: Path, obj) -> Path:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(obj), encoding="utf-8")
        return path

    def rows(self, name: str, lines: list[dict]) -> Path:
        path = self.root / name
        path.write_text("".join(json.dumps(r) + "\n" for r in lines), encoding="utf-8")
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
            "decision": decision, "by": "jason", "at": "2026-10-04T12:00:00-05:00",
        }), encoding="utf-8")

    def test_submit_and_status_card_size(self):
        cfg = self.write_json(self.root / "gap.json", {"dataset": "x"})
        h = self.harness()
        out = h.submit(self.job("job-1", "gap", cfg))
        self.assertTrue(out["ok"])
        self.assertEqual(out["state"], "queued")
        status = read_json(self.inbox / "status" / "job-1.json")
        card = read_json(self.inbox / "status" / "job-1.card.json")
        self.assertLessEqual(status_bytes(status), STATUS_MAX)
        self.assertLessEqual(len(json.dumps(card, separators=(",", ":")).encode()), CARD_MAX)
        self.assertEqual(status["state"], "queued")
        self.assertTrue((self.inbox / "outbox.jsonl").is_file())
        cell_status = self.cell / "harness-job-job-1" / "status.json"
        self.assertTrue(cell_status.is_file())

    def test_refuse_unknown_step_extra_field_and_train_forbid(self):
        cfg = self.write_json(self.root / "c.json", {})
        h = self.harness()
        bad = self.job("job-x", "chat", cfg)
        self.assertFalse(h.submit(bad)["ok"])
        extra = self.job("job-y", "gap", cfg)
        extra["note"] = "nope"
        self.assertFalse(h.submit(extra)["ok"])
        train = self.job("job-t", "train", cfg, gpu="forbid")
        refused = h.submit(train)
        self.assertFalse(refused["ok"])
        self.assertIn("forbid", refused["reason"])
        self.assertFalse((self.inbox / "jobs" / "job-t.json").exists())

    def test_gpu_ask_stops_for_approve_and_no_does_not_delete(self):
        cfg = self.write_json(self.root / "c.json", {"dataset": "x"})
        h = self.harness()
        body = self.job("job-g", "gap", cfg, gpu="ask", minutes=10)
        h.submit(body)
        stopped = h.run("job-g")
        self.assertEqual(stopped["state"], "needs-approval")
        self.assertTrue(stopped["needs_you"])
        self.assertTrue((self.inbox / "jobs" / "job-g.json").is_file())
        self.approve("job-g", "no")
        refused = h.run("job-g")
        self.assertEqual(refused["state"], "refused")
        self.assertTrue((self.inbox / "jobs" / "job-g.json").is_file())

    def test_approve_does_not_override_busy_or_unseen(self):
        cfg = self.write_json(self.root / "c.json", {"dataset": "x"})
        body = self.job("job-b", "gap", cfg, gpu="ask", minutes=10)
        self.harness(probe=BUSY).submit(body)
        self.approve("job-b", "yes")
        out = self.harness(probe=BUSY).run("job-b")
        self.assertEqual(out["state"], "refused")
        self.assertIn("busy", out["reason"])
        body2 = self.job("job-u", "gap", cfg, gpu="ask", minutes=10)
        self.harness(probe=UNSEEN).submit(body2)
        self.approve("job-u", "yes")
        unseen = self.harness(probe=UNSEEN).run("job-u")
        self.assertEqual(unseen["state"], "refused")
        self.assertIn("could not see", unseen["reason"])

    def test_gap_zero_class_and_tie_and_pass(self):
        zero = self.rows("zero.jsonl", [
            {"label": "a", "text": "hello ANSWER: yes"},
        ])
        cfg = self.write_json(self.root / "gap-zero.json", {
            "dataset": str(zero),
            "labels": ["a", "b"],
            "label_field": "label",
            "exam_shape": "ANSWER:",
            "shape_field": "text",
        })
        h = self.harness()
        h.submit(self.job("gap-z", "gap", cfg))
        out = h.run("gap-z")
        self.assertEqual(out["state"], "failed")
        self.assertIn("zero", out["reason"])
        page = (self.nas / "gap-z" / "reading.md").read_text(encoding="utf-8")
        flat = " ".join(page.split())
        self.assertIn("Jason needs to look", flat)
        self.assertIn("What we tried", page)
        tie = self.rows("tie.jsonl", [
            {"label": "a", "text": "left ANSWER: one"},
            {"label": "b", "text": "right ANSWER: two"},
        ])
        cfg2 = self.write_json(self.root / "gap-tie.json", {
            "dataset": str(tie),
            "labels": ["a", "b"],
            "label_field": "label",
            "exam_shape": "ANSWER:",
            "shape_field": "text",
            "contrast_marker": "CONTRAST",
        })
        h.submit(self.job("gap-t", "gap", cfg2))
        tied = h.run("gap-t")
        self.assertEqual(tied["state"], "failed")
        self.assertIn("tie", tied["reason"])
        good = self.rows("good.jsonl", [
            {"label": "a", "text": "left ANSWER: one CONTRAST"},
            {"label": "b", "text": "right ANSWER: two"},
            {"label": "a", "text": "more ANSWER: one"},
        ])
        cfg3 = self.write_json(self.root / "gap-ok.json", {
            "dataset": str(good),
            "labels": ["a", "b"],
            "label_field": "label",
            "exam_shape": "ANSWER:",
            "shape_field": "text",
            "contrast_marker": "CONTRAST",
        })
        h.submit(self.job("gap-ok", "gap", cfg3))
        passed = h.run("gap-ok")
        self.assertEqual(passed["state"], "succeeded")
        self.assertIn("Jason does not need to look", (self.nas / "gap-ok" / "reading.md").read_text())

    def test_digest_pauses_on_cloud_status(self):
        cfg = self.write_json(self.root / "c.json", {"dataset": "x"})
        h = self.harness()
        h.submit(self.job("q1", "gap", cfg))
        h.status("q1", via="cloud")
        quota = h.digest()
        self.assertEqual(quota["cloud_helper_turns_today"], 1)
        self.assertTrue(quota["pause"])
        self.assertLess(len(json.dumps(quota).encode()), 2048)
        h.submit(self.job("q2", "gap", cfg))
        waited = h.run("q2")
        self.assertEqual(waited["state"], "needs-approval")
        self.assertIn("cloud_helper_turns_today", waited["reason"])
        from harness_contract.digest import clear_quota
        clear_quota(h.inbox, "yes", "jason", "2026-10-04T12:30:00-05:00")
        self.assertFalse(h.digest()["pause"])

    def test_decontaminate_hit_and_threshold(self):
        exam = self.rows("exam.jsonl", [{"id": "e1", "text": LONG}])
        train = self.rows("train.jsonl", [{"id": "t1", "text": LONG}])
        cfg = self.write_json(self.root / "de.json", {
            "train": str(train), "exam": str(exam), "field": "text",
        })
        h = self.harness()
        h.submit(self.job("de-hit", "decontaminate", cfg))
        hit = h.run("de-hit")
        self.assertEqual(hit["state"], "refused")
        self.assertEqual(hit["exit_code"], 35)
        clean = self.rows("clean.jsonl", [{"id": "t2", "text": "unrelated pottery notes about clay"}])
        cfg2 = self.write_json(self.root / "de2.json", {
            "train": str(clean), "exam": str(exam), "field": "text",
        })
        h.submit(self.job("de-ok", "decontaminate", cfg2))
        self.assertEqual(h.run("de-ok")["state"], "succeeded")
        weak = self.write_json(self.root / "de3.json", {
            "train": str(clean), "exam": str(exam), "field": "text", "jaccard": 0.1,
        })
        h.submit(self.job("de-weak", "decontaminate", weak))
        weakened = h.run("de-weak")
        self.assertEqual(weakened["state"], "refused")
        self.assertIn("0.5", weakened["reason"])
        answer = self.write_json(self.root / "de4.json", {
            "train": str(clean), "exam": str(exam), "field": "answer",
        })
        h.submit(self.job("de-ans", "decontaminate", answer))
        self.assertIn("answer", h.run("de-ans")["reason"])

    def test_local_first_and_cloud_run(self):
        big = self.root / "manifest.json"
        big.write_bytes(b"x" * 1_000_000)
        got = cloud_read(big, kind="other")
        self.assertFalse(got["ok"])
        log = self.root / "job.log"
        log.write_text("noise\nerror: boom\nfail: later\n", encoding="utf-8")
        tail = cloud_read(log, kind="log")
        self.assertTrue(tail["ok"])
        self.assertIn("boom", tail["text"])
        self.assertNotIn("noise", tail["text"])
        cfg = self.write_json(self.root / "c.json", {"dataset": "x"})
        h = self.harness(caller="cloud")
        h.submit(self.job("cloud-1", "gap", cfg))
        out = h.run("cloud-1")
        self.assertEqual(out["state"], "refused")
        self.assertIn("cloud session", out["reason"])

    def test_judge_done_is_not_a_pass_and_dead_start(self):
        dead = judge(scorer_exit=0, server_started=False, done_file_exists=True)
        self.assertEqual(dead["state"], "refused")
        self.assertFalse(dead["scored"])
        self.assertIn("DONE", dead["reason"])
        failed = judge(scorer_exit=1, server_started=True, done_file_exists=True)
        self.assertEqual(failed["state"], "failed")
        self.assertNotEqual(failed["result"], "pass")
        ok = judge(scorer_exit=0, server_started=True, done_file_exists=False)
        self.assertEqual(ok["state"], "succeeded")

    def test_eval_job_uses_judge_and_corrections(self):
        cfg = self.write_json(self.root / "ev.json", {"expected_shape": "ANSWER:"})
        done = self.cell / "harness-job-ev1" / "DONE"
        done.parent.mkdir(parents=True)
        done.write_text("done\n")

        def dead(_plan):
            return {"launched": True, "server_started": False, "scorer_exit": 0, "rc": 0}

        h = self.harness(trainer=dead)
        h.submit(self.job("ev1", "eval", cfg, gpu="ask", minutes=10))
        self.approve("ev1")
        out = h.run("ev1")
        self.assertEqual(out["state"], "refused")
        self.assertIn("never started", out["reason"])
        self.assertFalse((self.inbox / "corrections.jsonl").exists())
        self.assertFalse((self.inbox / "registry.jsonl").exists())

        def bad(_plan):
            return {"launched": True, "server_started": True, "scorer_exit": 2, "rc": 2}

        h2 = self.harness(trainer=bad)
        h2.submit(self.job("ev2", "eval", cfg, gpu="ask", minutes=10))
        self.approve("ev2")
        failed = h2.run("ev2")
        self.assertEqual(failed["state"], "failed")
        misses = read_jsonl(self.inbox / "corrections.jsonl")
        self.assertEqual(misses[0]["id"], "ev2")
        reg = read_jsonl(self.inbox / "registry.jsonl")
        self.assertEqual(reg[0]["ready"], "no")
        self.assertEqual(reg[0]["gate"], "fail")

    def test_rewrite_loop(self):
        run = self.root / "run"
        self.assertTrue(claim(run, "notes.md", 1)["ok"])
        second = claim(run, "notes.md", 2, delta=None, approved=False)
        self.assertFalse(second["ok"])
        self.assertTrue(claim(run, "notes.md", 2, delta="added one line", approved=True)["ok"])
        third = claim(run, "notes.md", 3, delta="another line", approved=False)
        self.assertFalse(third["ok"])
        self.assertEqual(third["exit_code"], 38)

    def test_train_refuses_without_base_pick_and_does_not_call_trainer(self):
        data = self.rows("ds.jsonl", [
            {"label": "a", "text": "left ANSWER: one CONTRAST"},
            {"label": "b", "text": "right ANSWER: two"},
            {"label": "a", "text": "more ANSWER: one"},
        ])
        exam = self.rows("ex.jsonl", [{"id": "e", "text": "unrelated clay notes"}])
        calls = []

        def trainer(plan):
            calls.append(plan)
            return {"launched": False}

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
        })
        h = self.harness(trainer=trainer)
        h.submit(self.job("tr1", "train", cfg, gpu="ask", minutes=20))
        self.approve("tr1")
        out = h.run("tr1")
        self.assertEqual(out["state"], "refused")
        self.assertIn("base-pick", out["reason"])
        self.assertEqual(calls, [])
        page = self.root / "base.md"
        page.write_text("comparison\n", encoding="utf-8")
        cfg2 = self.write_json(self.root / "tr2.json", json.loads(cfg.read_text()))
        body = json.loads(cfg2.read_text())
        body["base_pick_page"] = str(page)
        cfg2.write_text(json.dumps(body), encoding="utf-8")
        h.submit(self.job("tr2", "train", cfg2, gpu="ask", minutes=20))
        self.approve("tr2")
        planned = h.run("tr2")
        self.assertEqual(planned["state"], "refused")
        self.assertEqual(planned["exit_code"], 39)
        plan = read_json(self.cell / "harness-job-tr2" / "plan.json")
        self.assertIn("train-step.sh", plan["command"])
        self.assertFalse(plan["launched"])
        self.assertIn("gap:pass", plan["gates"])
        self.assertIn("decontaminate:pass", plan["gates"])

    def test_registry_ready_stays_no_on_export_stand_in(self):
        cfg = self.write_json(self.root / "ex.json", {
            "recipe": "export.yaml", "base": "cached", "quant": "Q4_K_M",
        })

        def trainer(_plan):
            return {"launched": True, "rc": 0, "gpu_minutes": 0}

        h = self.harness(trainer=trainer)
        h.submit(self.job("ex1", "export", cfg, gpu="forbid", minutes=5))
        out = h.run("ex1")
        self.assertEqual(out["state"], "succeeded")
        self.assertIn("not a live train", out["reason"])
        line = read_jsonl(self.inbox / "registry.jsonl")[0]
        self.assertEqual(line["ready"], "no")
        self.assertNotEqual(line["ready"], "yes")

    def test_quant_page_words_and_missing_ladder(self):
        ladder = self.write_json(self.root / "ladder.json", {"passed": False, "pick": "Q4_K_M"})
        cfg = self.write_json(self.root / "q.json", {"ladder": str(ladder)})
        h = self.harness()
        h.submit(self.job("q1", "quant", cfg))
        out = h.run("q1")
        self.assertEqual(out["state"], "failed")
        page = " ".join((self.nas / "q1" / "reading.md").read_text(encoding="utf-8").split())
        self.assertIn("not for a device", page)
        missing = self.write_json(self.root / "q2.json", {"ladder": str(self.root / "nope.json")})
        h.submit(self.job("q2", "quant", missing))
        unseen = h.run("q2")
        self.assertIn("could not see", unseen["reason"])

    def test_ask_no_cloud_fallback(self):
        table = self.write_json(self.root / "routes.json", {"rows": [
            {"task": "news", "tag": "ag_news", "registry_id": "r1", "reason": "The table names this seat."},
        ]})
        cfg = self.write_json(self.root / "ask.json", {"task": "other", "route_table": str(table)})
        h = self.harness(seat=lambda row: "should-not-run")
        h.submit(self.job("ask1", "ask", cfg, gpu="ask", minutes=5))
        self.approve("ask1")
        out = h.run("ask1")
        self.assertEqual(out["state"], "refused")
        self.assertIn("no row", out["reason"])
        self.assertIn("cloud", out["reason"])
        cfg2 = self.write_json(self.root / "ask2.json", {"task": "news", "route_table": str(table)})
        h2 = self.harness()
        h2.submit(self.job("ask2", "ask", cfg2, gpu="forbid"))
        named = h2.run("ask2")
        self.assertIn("ag_news", named["reason"])
        self.assertIn("not called", named["reason"])
        self.assertNotIn("cloud model was used.", named["reason"].replace("No cloud model was used.", ""))

    def test_device_pack_needs_approve_and_does_not_seat(self):
        gguf = self.root / "model.gguf"
        gguf.write_bytes(b"gguf-bytes")
        cfg = self.write_json(self.root / "pack.json", {"gguf": str(gguf)})
        h = self.harness()
        h.submit(self.job("pk1", "pack-device", cfg, gpu="forbid"))
        stopped = h.run("pk1")
        self.assertEqual(stopped["state"], "needs-approval")
        self.assertFalse((self.nas / "pk1" / "device").exists())
        self.approve("pk1")
        packed = h.run("pk1")
        self.assertEqual(packed["state"], "succeeded")
        folder = self.nas / "pk1" / "device"
        self.assertTrue((folder / "model.gguf").is_file())
        self.assertTrue((folder / "Modelfile").is_file())
        how = (folder / "HOW-TO.md").read_text(encoding="utf-8")
        self.assertIn("did not run ollama create", how.lower())
        self.assertFalse((self.inbox / "registry.jsonl").exists())

    def test_preference_and_refusal_do_not_train(self):
        pref = write_preference(self.root / "pref", [
            {"prompt": "a", "chosen": "yes", "rejected": "no"},
        ])
        self.assertFalse(pref["auto_train"])
        self.assertFalse(pref["train_invoked"])
        recipe = (self.root / "pref" / "recipe.yaml").read_text(encoding="utf-8")
        self.assertIn("stage: dpo", recipe)
        self.assertIn("do_train: false", recipe)
        ref = write_refusal(self.root / "ref", [
            {"target": "I cannot do that"},
        ], marker="I cannot do that")
        self.assertFalse(ref["train_invoked"])
        task = self.rows("task.jsonl", [{"target": "I cannot do that", "label": "a", "text": "ANSWER:"}])
        refusal = self.rows("refusal.jsonl", [{"target": "classify this", "label": "a", "text": "ANSWER:"}])
        cfg = self.write_json(self.root / "mix.json", {
            "dataset": str(task),
            "labels": ["a"],
            "label_field": "label",
            "exam_shape": "ANSWER:",
            "shape_field": "text",
            "task_set": str(task),
            "refusal_set": str(refusal),
            "refusal_marker": "I cannot do that",
        })
        h = self.harness()
        h.submit(self.job("mix1", "gap", cfg))
        mixed = h.run("mix1")
        self.assertEqual(mixed["state"], "failed")
        self.assertIn("mixed", mixed["reason"])

    def test_base_pick_does_not_download(self):
        exam = self.rows("exam.jsonl", [{"id": "e", "text": "ANSWER:"}])
        cached = self.root / "base-a"
        cached.write_text("weights", encoding="utf-8")
        missing = self.root / "not-here.bin"
        h = self.harness()
        self.approve("bp1")
        out = h.base_pick_run("bp1", {
            "models": [str(cached), str(missing)],
            "exam": str(exam),
            "marker": "ANSWER:",
            "nas_out": str(self.nas / "bp1"),
            "max_minutes": 10,
        }, scorer=lambda path: path.name == "base-a")
        self.assertIn("wrote the answer marker", out["reason"])
        page = (self.nas / "bp1" / "base-pick.md").read_text(encoding="utf-8")
        self.assertIn("not downloaded", page.lower())
        bare = h.base_pick_run("bp2", {
            "models": [str(cached)],
            "exam": str(exam),
            "nas_out": str(self.nas / "bp2"),
            "max_minutes": 10,
        }, scorer=None)
        self.approve("bp2")
        bare = h.base_pick_run("bp2", {
            "models": [str(cached)],
            "exam": str(exam),
            "nas_out": str(self.nas / "bp2"),
            "max_minutes": 10,
        }, scorer=None)
        self.assertIn("did not load", bare["reason"])

    def test_failure_inbox_rejects_a_new_version_name(self):
        path = self.inbox / "corrections.jsonl"
        append_miss(path, job_id="m1", expected_shape="ANSWER:", reason="The scorer did not exit 0.")
        with self.assertRaises(ValueError):
            append_miss(self.inbox / "corrections.v2.jsonl", job_id="m2", expected_shape="x", reason="no")

    def test_skill_has_no_timer_sentence_and_smoke_is_untouched(self):
        skill = (ROOT / "docs" / "grok-bot-harness-skill.md").read_text(encoding="utf-8")
        self.assertNotIn("five minutes", skill.lower())
        self.assertNotIn("check every", skill.lower())
        makefile = (ROOT / "Makefile").read_text(encoding="utf-8")
        self.assertNotIn("cell-job", makefile)
        factory = (ROOT / "scripts" / "factory-steps" / "factory_steps.py").read_text(encoding="utf-8")
        self.assertNotIn("harness_contract", factory)

    def test_cli_submit_prints_id_and_leaves_estate(self):
        cfg = self.write_json(self.root / "c.json", {"dataset": "x"})
        body = self.job("cli1", "gap", cfg)
        env = os.environ.copy()
        env["HARNESS_INBOX"] = str(self.inbox)
        env["HARNESS_CELL_ROOT"] = str(self.cell)
        env["HARNESS_GPU_PROBE"] = "idle"
        env["CUDA_VISIBLE_DEVICES"] = ""
        proc = subprocess.run(
            [sys.executable, str(ROOT / "scripts/factory-steps/harness_contract/cli.py"), "submit"],
            input=json.dumps(body),
            text=True,
            capture_output=True,
            cwd=ROOT,
            env=env,
            check=False,
        )
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn("cli1", proc.stdout)
        self.assertEqual(read_cksum(estate_path(ROOT)), ESTATE_CKSUM)


def status_bytes(obj) -> int:
    return len(json.dumps(obj, separators=(",", ":")).encode())


if __name__ == "__main__":
    unittest.main()
