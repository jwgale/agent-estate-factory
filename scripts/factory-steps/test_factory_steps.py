#!/usr/bin/env python3
"""Unit tests for the train / export factory steps (CPU only: fake llamafactory-cli, mocked GPU probe, temp repo root).

Run: python3 scripts/factory-steps/test_factory_steps.py   (needs PyYAML; never touches the GPU, the repo, or Ollama)
"""
import contextlib, io, json, os, sys, tempfile, textwrap, time, unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import factory_steps as fs  # noqa: E402

FAKE_CLI = """#!/bin/bash
# fake llamafactory-cli: $1 = train|export, $2 = yaml. Behaviour from FAKE_MODE.
case "${FAKE_MODE:-ok}" in
  fail) echo "Traceback (most recent call last):"; exit 1 ;;
  sleep) trap 'kill $! 2>/dev/null; exit 143' TERM; sleep 120 & wait ;;
  ok) out=$(python3 -c "import yaml,sys;d=yaml.safe_load(open(sys.argv[1]));print(d.get('output_dir') or d.get('export_dir'))" "$2")
      mkdir -p "$out"; sleep 1
      if [ "$1" = train ]; then echo '{}' > "$out/all_results.json"; echo x > "$out/adapter_model.safetensors";
      else echo '{}' > "$out/config.json"; echo x > "$out/model.safetensors"; fi ;;
esac
"""


class LFSteps(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.env = mock.patch.dict(os.environ, {"FACTORY_ROOT": str(self.root)})
        self.env.start()
        self.gpu = mock.patch.object(fs, "_gpu_probe", return_value=(True, "test util=0% compute_apps=[]"))
        self.gpu.start()
        base = self.root / ".cell/classify-base-cache/base"
        base.mkdir(parents=True)
        (base / "config.json").write_text("{}")
        (base / "model.safetensors").write_text("w")
        self.cell = self.root / ".cell/test-cell"
        self.cell.mkdir(parents=True)
        (self.cell / "data.jsonl").write_text('{"messages": []}\n' * 3)
        (self.cell / "dataset_info.json").write_text(json.dumps(
            {"ds": {"file_name": "data.jsonl", "formatting": "sharegpt"}, "ho": {"file_name": "data.jsonl", "formatting": "sharegpt"}}))
        self.cli = self.root / "bin/llamafactory-cli"
        self.cli.parent.mkdir()
        self.cli.write_text(FAKE_CLI)
        self.cli.chmod(0o755)

    def tearDown(self):
        self.gpu.stop()
        self.env.stop()
        self.tmp.cleanup()

    def recipe(self, **over):
        r = dict(model_name_or_path=".cell/classify-base-cache/base", stage="sft", do_train=True, finetuning_type="lora",
                 template="qwen3_5", dataset="ds", dataset_dir=".cell/test-cell", val_size=0.1,
                 output_dir=".cell/test-cell/outputs", overwrite_output_dir=True)
        r.update(over)
        r = {k: v for k, v in r.items() if v is not None}
        p = self.cell / "recipe.yaml"
        p.write_text(fs.yaml.safe_dump(r))
        w = self.cell / "wrap.yaml"
        w.write_text(fs.yaml.safe_dump({"schema": "cell-one.train-step.v0", "train": {"recipe": "recipe.yaml", "cli": str(self.cli)},
                                        "export": {"config": "export.yaml", "cli": str(self.cli)}}))
        return str(w)

    def export_cfg(self, **over):
        e = dict(model_name_or_path=".cell/classify-base-cache/base", adapter_name_or_path=".cell/test-cell/outputs",
                 template="qwen3_5", finetuning_type="lora", export_dir=".cell/test-cell/export", export_device="cpu")
        e.update(over)
        (self.cell / "export.yaml").write_text(fs.yaml.safe_dump(e))
        return str(self.cell / "wrap.yaml")

    def run_main(self, *argv, mode="ok"):
        out, err = io.StringIO(), io.StringIO()
        with mock.patch.dict(os.environ, {"FAKE_MODE": mode}), contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            try:
                rc = fs.main(list(argv))
            except SystemExit as e:
                rc = e.code
        return rc, out.getvalue(), err.getvalue()

    # --- train validation
    def test_dry_run_valid_prints_command_and_writes_nothing(self):
        w = self.recipe()
        before = sorted(p.name for p in self.cell.rglob("*"))
        rc, out, _ = self.run_main("train", "--config", w, "--dry-run")
        self.assertEqual(rc, 0, out)
        self.assertIn("setsid nohup", out)
        self.assertIn("train.pid", out)
        self.assertEqual(before, sorted(p.name for p in self.cell.rglob("*")))

    def test_output_outside_cell_refused(self):
        rc, *_ = self.run_main("train", "--config", self.recipe(output_dir="outputs"), "--dry-run")
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)

    def test_output_in_base_cache_refused(self):
        rc, *_ = self.run_main("train", "--config", self.recipe(output_dir=".cell/classify-base-cache/x"), "--dry-run")
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)

    def test_missing_dataset_refused(self):
        rc, *_ = self.run_main("train", "--config", self.recipe(dataset="nope"), "--dry-run")
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)

    def test_missing_dataset_info_refused(self):
        rc, *_ = self.run_main("train", "--config", self.recipe(dataset_dir=".cell/test-cell/none"), "--dry-run")
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)

    def test_eval_dataset_and_val_size_conflict(self):
        rc, *_ = self.run_main("train", "--config", self.recipe(eval_dataset="ho"), "--dry-run")
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)

    def test_eval_dataset_ok(self):
        rc, out, _ = self.run_main("train", "--config", self.recipe(eval_dataset="ho", val_size=None), "--dry-run")
        self.assertEqual(rc, 0, out)

    def test_missing_base_model_refused(self):
        rc, *_ = self.run_main("train", "--config", self.recipe(model_name_or_path="Qwen/NotCached"), "--dry-run")
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)

    def test_gpu_busy_refused(self):
        w = self.recipe()
        with mock.patch.object(fs, "_gpu_probe", return_value=(False, "util=97% compute_apps=[123, x, 1 MiB]")):
            rc, *_ = self.run_main("train", "--config", w, "--dry-run")
        self.assertEqual(rc, 3)

    def test_direct_recipe_path(self):
        self.recipe()
        rc, out, _ = self.run_main("train", "--config", str(self.cell / "recipe.yaml"), "--dry-run")
        # default cli (~/venvs/...) may not exist on a test host: either valid or refused only for the cli
        self.assertIn(rc, (0, fs.EXIT_TRAIN_INVALID))
        if rc:
            self.assertIn("llamafactory-cli not executable", out)

    def test_foreign_schema_refused(self):
        p = self.cell / "other.yaml"
        p.write_text("schema: cell-one.something.v9\n")
        rc, *_ = self.run_main("train", "--config", str(p), "--dry-run")
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)

    # --- train launch / status / watchdog (fake cli)
    def test_launch_wait_ok(self):
        w = self.recipe()
        rc, out, err = self.run_main("train", "--config", w, "--wait", "--poll-seconds", "0.2", "--startup-seconds", "0.2")
        self.assertEqual(rc, 0, out + err)
        rec = json.loads((self.cell / "train-step.json").read_text())
        self.assertEqual(rec["result"], "ok")
        self.assertTrue((self.cell / "train.pid").exists())
        self.assertEqual(self.run_main("train", "--config", w, "--status")[0], 0)

    def test_launch_failure(self):
        rc, *_ = self.run_main("train", "--config", self.recipe(), "--startup-seconds", "1", mode="fail")
        self.assertEqual(rc, fs.EXIT_TRAIN_LAUNCH)

    def test_watchdog_kills_by_pid_and_running_refused(self):
        w = self.recipe()
        rc, *_ = self.run_main("train", "--config", w, "--startup-seconds", "0.2", mode="sleep")
        self.assertEqual(rc, 0)
        pid = int((self.cell / "train.pid").read_text())
        self.assertTrue(fs._pid_alive(pid))
        self.assertEqual(self.run_main("train", "--config", w, "--status")[0], fs.EXIT_TRAIN_STILL)
        self.assertEqual(self.run_main("train", "--config", w, "--dry-run")[0], fs.EXIT_TRAIN_RUNNING)
        rc, *_ = self.run_main("train", "--config", w, "--status", "--wait", "--max-minutes", "0.001", "--poll-seconds", "0.2")
        self.assertEqual(rc, fs.EXIT_TRAIN_WATCHDOG)
        for _ in range(50):
            if not fs._pid_alive(pid):
                break
            time.sleep(0.1)
        self.assertFalse(fs._pid_alive(pid))

    def test_nested_run_dir_holds_pid_and_log(self):
        rc, out, _ = self.run_main("train", "--config", self.recipe(output_dir=".cell/test-cell/run1/outputs"), "--dry-run")
        self.assertEqual(rc, 0, out)
        plan = json.loads(out[:out.rindex("}") + 1])
        self.assertTrue(plan["pid_file"].endswith(".cell/test-cell/run1/train.pid"))
        self.assertTrue(plan["log"].endswith(".cell/test-cell/run1/logs/train.log"))

    def test_status_none(self):
        self.assertEqual(self.run_main("train", "--config", self.recipe(), "--status")[0], fs.EXIT_TRAIN_NONE)

    def test_stale_pid_not_ours_is_not_killed(self):
        w = self.recipe()
        (self.cell / "train.pid").write_text(f"{os.getpid()}\n")   # this test process: alive, but not llamafactory-cli
        self.assertEqual(self.run_main("train", "--config", w, "--status")[0], fs.EXIT_TRAIN_FAILED)
        self.assertEqual(fs._stop_pid(os.getpid(), self.cell / "recipe.yaml"), "not-ours")

    # --- export
    def test_export_requires_adapter(self):
        self.recipe()
        rc, *_ = self.run_main("export", "--config", self.export_cfg(), "--dry-run")
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)

    def test_export_dry_run_then_run(self):
        self.recipe()
        w = self.export_cfg()
        ad = self.cell / "outputs"
        ad.mkdir()
        (ad / "adapter_config.json").write_text("{}")
        (ad / "adapter_model.safetensors").write_text("x")
        rc, out, _ = self.run_main("export", "--config", w, "--dry-run")
        self.assertEqual(rc, 0, out)
        self.assertIn("export.pid", out)
        rc, out, err = self.run_main("export", "--config", w, "--wait", "--poll-seconds", "0.2", "--startup-seconds", "0.2")
        self.assertEqual(rc, 0, out + err)
        rc, *_ = self.run_main("export", "--config", w, "--dry-run")   # export_dir now non-empty: never overwrite
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)

    # --- existing steps still parse
    def test_existing_steps_help(self):
        for step in ("build", "eval", "determinism", "compare", "train", "export"):
            with self.assertRaises(SystemExit) as cm, contextlib.redirect_stdout(io.StringIO()):
                fs.main([step, "--help"])
            self.assertEqual(cm.exception.code, 0)

    def test_quant_ladder_config_still_requires_schema(self):
        p = self.cell / "bad.yaml"
        p.write_text("schema: nope\n")
        rc, _, err = self.run_main("compare", "--config", str(p))
        self.assertEqual(rc, 2)
        self.assertIn("refuse:config", err)



FAKE_LF = {
    "llamafactory/__init__.py": "",
    "llamafactory/data/__init__.py": "",
    "llamafactory/data/template.py": "TEMPLATES = {}\n\ndef register_template(name, **kw):\n    TEMPLATES[name] = kw\n",
    "llamafactory/cli.py": textwrap.dedent("""\
        import os, sys, time, yaml
        from pathlib import Path

        def main():
            step, cfg = sys.argv[1], sys.argv[2]
            mode = os.environ.get("FAKE_MODE", "ok")
            if mode == "fail":
                print("Traceback (most recent call last):"); sys.exit(1)
            if mode == "sleep":
                time.sleep(120)
            d = yaml.safe_load(open(cfg))
            out = Path(d.get("output_dir") or d.get("export_dir")); out.mkdir(parents=True, exist_ok=True); time.sleep(1)
            if step == "train":
                (out / "all_results.json").write_text("{}"); (out / "adapter_model.safetensors").write_text("x")
            else:
                (out / "config.json").write_text("{}"); (out / "model.safetensors").write_text("x")
            print("fake-lf", step, d.get("template"))
            return 0
        """),
}
CUSTOM_TPL = 'from llamafactory.data.template import register_template\nregister_template(name="my_tpl", stop_words=["<e>"])\n'


class CustomTemplates(unittest.TestCase):
    """train/export with a recipe-supplied template file: lf_launch.py + a fake llamafactory package (no venv, no GPU)."""
    recipe, export_cfg, run_main = LFSteps.recipe, LFSteps.export_cfg, LFSteps.run_main   # same fixture, own tests

    def setUp(self):
        LFSteps.setUp(self)
        pkg = self.root / "fakepkgs"
        for rel, src in FAKE_LF.items():
            (pkg / rel).parent.mkdir(parents=True, exist_ok=True)
            (pkg / rel).write_text(src)
        self.pp = mock.patch.dict(os.environ, {"PYTHONPATH": os.pathsep.join([str(pkg), os.environ.get("PYTHONPATH", "")])})
        self.pp.start()
        (self.cell / "templates").mkdir()
        (self.cell / "templates/my_tpl.py").write_text(CUSTOM_TPL)

    def tearDown(self):
        self.pp.stop()
        LFSteps.tearDown(self)

    def ct_wrap(self, ct="templates/my_tpl.py", where="train", **over):
        w = Path(self.recipe(**{"template": "my_tpl", **over}))
        d = fs.yaml.safe_load(w.read_text())
        if ct is not None:
            (d if where == "top" else d[where])["custom_templates"] = ct
        for s in ("train", "export"):
            d[s]["python"] = sys.executable
        w.write_text(fs.yaml.safe_dump(d))
        return str(w)

    def plan(self, out):
        return json.loads(out[:out.rindex("}") + 1])

    def test_dry_run_uses_launcher_and_records_sha(self):
        rc, out, _ = self.run_main("train", "--config", self.ct_wrap(), "--dry-run")
        self.assertEqual(rc, 0, out)
        p = self.plan(out)
        ct = p["checks"]["custom_templates"]
        self.assertEqual(ct["sha256"], fs.sha256_file(self.cell / "templates/my_tpl.py"))
        self.assertEqual(ct["template"], "my_tpl")
        self.assertIn("lf_launch.py --custom-templates", p["command"])
        self.assertIn("--cli " + str(self.cli) + " train", p["command"])
        self.assertFalse(p["warnings"])

    def test_top_level_key_and_literal_warning(self):
        rc, out, _ = self.run_main("train", "--config", self.ct_wrap(where="top", template="other_tpl"), "--dry-run")
        self.assertEqual(rc, 0, out)
        self.assertTrue(any("not a string literal" in x for x in self.plan(out)["warnings"]))

    def test_missing_file_refused(self):
        rc, out, err = self.run_main("train", "--config", self.ct_wrap(ct="templates/nope.py"), "--dry-run")
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)
        self.assertIn("custom_templates: not found", err)

    def test_non_py_refused(self):
        (self.cell / "templates/my_tpl.txt").write_text(CUSTOM_TPL)
        rc, *_ = self.run_main("train", "--config", self.ct_wrap(ct="templates/my_tpl.txt"), "--dry-run")
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)

    def test_bad_python_refused(self):
        w = Path(self.ct_wrap())
        d = fs.yaml.safe_load(w.read_text())
        d["train"]["python"] = str(self.root / "bin/no-python")
        w.write_text(fs.yaml.safe_dump(d))
        rc, *_ = self.run_main("train", "--config", str(w), "--dry-run")
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)

    def test_without_key_command_unchanged(self):
        rc, out, _ = self.run_main("train", "--config", self.ct_wrap(ct=None), "--dry-run")
        self.assertEqual(rc, 0, out)
        p = self.plan(out)
        self.assertNotIn("custom_templates", p["checks"])
        self.assertIn(f"setsid nohup {self.cli} train ", p["command"])
        self.assertNotIn("lf_launch", p["command"])

    def test_launch_wait_ok_through_launcher(self):
        w = self.ct_wrap()
        rc, out, err = self.run_main("train", "--config", w, "--wait", "--poll-seconds", "0.2", "--startup-seconds", "0.2")
        self.assertEqual(rc, 0, out + err)
        log = (self.cell / "logs/train.log").read_text()
        self.assertIn("template 'my_tpl' registered", log)
        self.assertIn("fake-lf train my_tpl", log)
        self.assertEqual(self.run_main("train", "--config", w, "--status")[0], 0)
        self.assertFalse((self.cell / "templates/__pycache__").exists())

    def test_unregistered_template_fails_launch(self):
        (self.cell / "templates/my_tpl.py").write_text("# registers nothing\n")
        rc, *_ = self.run_main("train", "--config", self.ct_wrap(), "--startup-seconds", "3")
        self.assertEqual(rc, fs.EXIT_TRAIN_LAUNCH)
        self.assertIn("is not registered", (self.cell / "logs/train.log").read_text())

    def test_torchrun_env_refused_by_launcher(self):
        w = self.ct_wrap()
        with mock.patch.dict(os.environ, {"FORCE_TORCHRUN": "1"}):
            rc, *_ = self.run_main("train", "--config", w, "--startup-seconds", "3")
        self.assertEqual(rc, fs.EXIT_TRAIN_LAUNCH)
        self.assertIn("single-process only", (self.cell / "logs/train.log").read_text())

    def test_watchdog_and_status_see_launcher_pid(self):
        w = self.ct_wrap()
        rc, *_ = self.run_main("train", "--config", w, "--startup-seconds", "0.5", mode="sleep")
        self.assertEqual(rc, 0)
        pid = int((self.cell / "train.pid").read_text())
        self.assertTrue(fs._pid_is_job(pid, self.cell / "recipe.yaml"))
        self.assertEqual(self.run_main("train", "--config", w, "--status")[0], fs.EXIT_TRAIN_STILL)
        rc, *_ = self.run_main("train", "--config", w, "--status", "--wait", "--max-minutes", "0.001", "--poll-seconds", "0.2")
        self.assertEqual(rc, fs.EXIT_TRAIN_WATCHDOG)
        for _ in range(50):
            if not fs._pid_alive(pid):
                break
            time.sleep(0.1)
        self.assertFalse(fs._pid_alive(pid))

    def test_export_through_launcher(self):
        self.ct_wrap(ct="templates/my_tpl.py", where="export")
        w = self.export_cfg(template="my_tpl")
        ad = self.cell / "outputs"
        ad.mkdir()
        (ad / "adapter_config.json").write_text("{}")
        (ad / "adapter_model.safetensors").write_text("x")
        rc, out, _ = self.run_main("export", "--config", w, "--dry-run")
        self.assertEqual(rc, 0, out)
        self.assertIn("--cli " + str(self.cli) + " export", self.plan(out)["command"])
        rc, out, err = self.run_main("export", "--config", w, "--wait", "--poll-seconds", "0.2", "--startup-seconds", "0.2")
        self.assertEqual(rc, 0, out + err)
        self.assertIn("fake-lf export my_tpl", (self.cell / "logs/export.log").read_text())

    # --- slot114: <step>.venv, <step>.preload, export.generation_config
    def fake_venv(self, name="sidecar"):
        v = self.root / "venvs" / name
        (v / "bin").mkdir(parents=True)
        (v / "pyvenv.cfg").write_text("home = /usr/bin\n")
        (v / "bin/python").symlink_to(sys.executable)
        (v / "bin/llamafactory-cli").write_text(FAKE_CLI)
        (v / "bin/llamafactory-cli").chmod(0o755)
        return v

    def set_wrap(self, w, step, **kv):
        d = fs.yaml.safe_load(Path(w).read_text())
        for k, val in kv.items():
            if val is None:
                d[step].pop(k, None)
            else:
                d[step][k] = val
        Path(w).write_text(fs.yaml.safe_dump(d))
        return w

    def test_venv_uses_sidecar_cli_without_launcher(self):
        v = self.fake_venv()
        w = self.set_wrap(self.ct_wrap(ct=None), "train", cli=None, python=None, venv=str(v))
        rc, out, _ = self.run_main("train", "--config", w, "--dry-run")
        self.assertEqual(rc, 0, out)
        p = self.plan(out)
        self.assertEqual(p["checks"]["venv"]["cli"], str(v / "bin/llamafactory-cli"))
        self.assertIn(f"setsid nohup {v}/bin/llamafactory-cli train ", p["command"])
        self.assertNotIn("lf_launch", p["command"])
        rc, out, err = self.run_main("train", "--config", w, "--wait", "--poll-seconds", "0.2", "--startup-seconds", "0.2")
        self.assertEqual(rc, 0, out + err)

    def test_venv_python_runs_launcher(self):
        v = self.fake_venv()
        w = self.set_wrap(self.ct_wrap(), "train", cli=None, python=None, venv=str(v))
        rc, out, _ = self.run_main("train", "--config", w, "--dry-run")
        self.assertEqual(rc, 0, out)
        p = self.plan(out)
        self.assertIn(f"setsid nohup {v}/bin/python ", p["command"])   # not symlink-resolved: the venv stays active
        self.assertIn(f"--cli {v}/bin/llamafactory-cli train", p["command"])
        self.assertEqual(p["checks"]["custom_templates"]["python"], str(v / "bin/python"))
        rc, out, err = self.run_main("train", "--config", w, "--wait", "--poll-seconds", "0.2", "--startup-seconds", "0.2")
        self.assertEqual(rc, 0, out + err)
        self.assertIn("fake-lf train my_tpl", (self.cell / "logs/train.log").read_text())

    def test_venv_conflicts_and_invalid(self):
        v = self.fake_venv()
        w = self.set_wrap(self.ct_wrap(ct=None), "train", venv=str(v))   # cli still set
        self.assertEqual(self.run_main("train", "--config", w, "--dry-run")[0], fs.EXIT_TRAIN_INVALID)
        (self.root / "notavenv").mkdir()
        w = self.set_wrap(w, "train", cli=None, venv=str(self.root / "notavenv"))
        rc, out, _ = self.run_main("train", "--config", w, "--dry-run")
        self.assertEqual(rc, fs.EXIT_TRAIN_INVALID)
        self.assertIn("not a venv", out)

    def test_preload_runs_before_template_and_alone(self):
        (self.cell / "patches").mkdir()
        (self.cell / "patches/flag.py").write_text('print("PRELOAD-RAN", flush=True)\n')
        w = self.set_wrap(self.ct_wrap(ct=None, template="qwen3_5"), "train", preload=["patches/flag.py"])
        rc, out, _ = self.run_main("train", "--config", w, "--dry-run")
        self.assertEqual(rc, 0, out)
        p = self.plan(out)
        self.assertEqual(p["checks"]["preload"][0]["sha256"], fs.sha256_file(self.cell / "patches/flag.py"))
        self.assertIn("lf_launch.py --preload ", p["command"])
        # the fake package has no qwen3_5: preload alone + an unregistered template refuses at launch (22)
        rc, *_ = self.run_main("train", "--config", w, "--startup-seconds", "3")
        self.assertEqual(rc, fs.EXIT_TRAIN_LAUNCH)
        log = (self.cell / "logs/train.log").read_text()
        self.assertIn("PRELOAD-RAN", log)
        w = self.set_wrap(self.ct_wrap(), "train", preload=["patches/flag.py"])
        rc, out, err = self.run_main("train", "--config", w, "--wait", "--poll-seconds", "0.2", "--startup-seconds", "0.2")
        self.assertEqual(rc, 0, out + err)
        log = (self.cell / "logs/train.log").read_text()
        self.assertLess(log.index("PRELOAD-RAN"), log.index("imported custom_templates"))
        w = self.set_wrap(w, "train", preload=["patches/nope.py"])
        self.assertEqual(self.run_main("train", "--config", w, "--dry-run")[0], fs.EXIT_TRAIN_INVALID)

    def test_export_generation_config_applied(self):
        w = self.ct_wrap(ct="templates/my_tpl.py", where="export")
        self.set_wrap(w, "export", generation_config={"eos_token_id": [2, 11]})
        w = self.export_cfg(template="my_tpl")
        ad = self.cell / "outputs"
        ad.mkdir()
        (ad / "adapter_config.json").write_text("{}")
        (ad / "adapter_model.safetensors").write_text("x")
        rc, out, _ = self.run_main("export", "--config", w, "--dry-run")
        self.assertEqual(rc, 0, out)
        p = self.plan(out)
        self.assertEqual(p["checks"]["generation_config"], {"eos_token_id": [2, 11]})
        self.assertIn('--json-patches \'{"generation_config.json":{"set":{"eos_token_id":[2,11]}}}\'', p["command"])
        rc, out, err = self.run_main("export", "--config", w, "--wait", "--poll-seconds", "0.2", "--startup-seconds", "0.2")
        self.assertEqual(rc, 0, out + err)
        g = json.loads((self.cell / "export/generation_config.json").read_text())
        self.assertEqual(g["eos_token_id"], [2, 11])

    def test_export_json_patches_set_and_drop(self):
        w = self.ct_wrap(ct="templates/my_tpl.py", where="export")
        self.set_wrap(w, "export", json_patches={"config.json": {"drop": ["num_experts_per_tok", "absent_key"], "set": {"x": 1}}},
                      generation_config={"eos_token_id": [2, 11]})
        w = self.export_cfg(template="my_tpl")
        ad = self.cell / "outputs"
        ad.mkdir()
        (ad / "adapter_config.json").write_text("{}")
        (ad / "adapter_model.safetensors").write_text("x")
        rc, out, _ = self.run_main("export", "--config", w, "--dry-run")
        self.assertEqual(rc, 0, out)
        self.assertEqual(set(self.plan(out)["checks"]["json_patches"]), {"config.json", "generation_config.json"})
        # the fake export writes config.json = {}, so the drop list matches nothing (logged as dropped [])
        rc, out, err = self.run_main("export", "--config", w, "--wait", "--poll-seconds", "0.2", "--startup-seconds", "0.2")
        self.assertEqual(rc, 0, out + err)
        self.assertEqual(json.loads((self.cell / "export/config.json").read_text()), {"x": 1})
        self.assertEqual(json.loads((self.cell / "export/generation_config.json").read_text())["eos_token_id"], [2, 11])
        self.assertIn("dropped []", (self.cell / "logs/export.log").read_text())

    def test_export_json_patches_invalid(self):
        w = self.ct_wrap(ct=None)
        for bad in ({"../x.json": {"set": {}}}, {"config.yaml": {"drop": []}}, {"config.json": {"rename": {}}}, {"config.json": {"drop": "k"}}):
            self.set_wrap(w, "export", json_patches=bad)
            self.export_cfg()
            rc, out, _ = self.run_main("export", "--config", w, "--dry-run")
            self.assertEqual(rc, fs.EXIT_TRAIN_INVALID, bad)
            self.assertIn("export.json_patches", out)

    def test_train_generation_config_refused(self):
        w = self.set_wrap(self.ct_wrap(), "train", generation_config={"eos_token_id": [2, 11]})
        self.assertEqual(self.run_main("train", "--config", w, "--dry-run")[0], fs.EXIT_TRAIN_INVALID)


class DecodeStop(unittest.TestCase):
    """eval: optional decode.stop is passed to llama-server /completion (mocked server; no GPU, no llama.cpp)."""

    def run_eval(self, decode):
        with tempfile.TemporaryDirectory() as td:
            d = Path(td)
            (d / "m.gguf").write_text("x")
            c = {"schema": fs.SCHEMA, "out_dir": "out", "model": {"hf_id": "x/m"}, "quants": ["f16"], "decode": decode,
                 "toolchain": {"server_bin": "/bin/true"}, "eval": {"tasks": [{"name": "t1", "n_predict": 8}]}}
            (d / "cfg.yaml").write_text(fs.yaml.safe_dump(c))
            cfg = fs.Cfg(str(d / "cfg.yaml"))
            bodies = []
            proc = mock.MagicMock()
            proc.poll.return_value = None
            with mock.patch.object(fs.subprocess, "Popen", return_value=proc), \
                 mock.patch.object(fs.urllib.request, "urlopen"), mock.patch.object(fs, "gpu_check"), \
                 mock.patch.object(fs, "post", side_effect=lambda u, b, **k: bodies.append(b) or {"content": "A"}), \
                 contextlib.redirect_stdout(io.StringIO()):
                fs.run_one(cfg, "f16", 0, {"t1": [{"id": "a", "prompt": "p", "input_hash": "h"}]})
            return bodies

    def test_stop_passed_when_set(self):
        self.assertEqual(self.run_eval({"stop": ["<|im_end|>"]})[0]["stop"], ["<|im_end|>"])

    def test_no_stop_by_default(self):
        self.assertNotIn("stop", self.run_eval({})[0])


class ConvertScript(unittest.TestCase):
    """build: optional toolchain.convert_script replaces convert_hf_to_gguf.py (same CLI); default path unchanged (fakes; no llama.cpp)."""

    def build(self, convert_script=None, make_script=True):
        td = tempfile.TemporaryDirectory(); self.addCleanup(td.cleanup)
        d = Path(td.name)
        (d / "hf").mkdir(); (d / "hf" / "config.json").write_text("{}")
        lc = d / "lc"; lc.mkdir()
        fake = "import sys, pathlib\na = sys.argv\npathlib.Path(a[a.index('--outfile') + 1]).write_text(TAG)\n"
        (lc / "convert_hf_to_gguf.py").write_text("TAG = 'stock'\n" + fake)
        if make_script:
            (d / "shim.py").write_text("TAG = 'shim'\n" + fake)
        tc = {"llama_cpp_dir": str(lc), "convert_python": sys.executable, "quantize_bin": "/bin/false"}
        if convert_script:
            tc["convert_script"] = convert_script
        c = {"schema": fs.SCHEMA, "out_dir": "out", "model": {"hf_id": "x/m", "hf_path": "hf"}, "quants": ["f16"], "toolchain": tc}
        (d / "cfg.yaml").write_text(fs.yaml.safe_dump(c))
        cfg = fs.Cfg(str(d / "cfg.yaml"))
        with mock.patch.object(fs, "run_logged", side_effect=lambda cmd, log, nice=True: fs.subprocess.run(cmd, check=True)), \
             contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            fs.cmd_build(cfg, None)
        return cfg.gguf("f16").read_text(), json.loads((cfg.out / "manifests" / "gguf.json").read_text())

    def test_default_uses_stock_converter(self):
        out, man = self.build()
        self.assertEqual(out, "stock")
        self.assertNotIn("convert_script_sha256", man)

    def test_convert_script_used_and_recorded(self):
        out, man = self.build("shim.py")
        self.assertEqual(out, "shim")
        self.assertIn("shim.py", man["convert"])
        self.assertEqual(len(man["convert_script_sha256"]), 64)

    def test_missing_convert_script_refused(self):
        with self.assertRaises(SystemExit) as cm:
            self.build("shim.py", make_script=False)
        self.assertEqual(cm.exception.code, 2)


class PromptKwargs(unittest.TestCase):
    """build_prompts passes eval / task chat_template_kwargs to apply_chat_template (fake tokenizer; no transformers, no GPU)."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        d = Path(self.tmp.name)
        (d / "model").mkdir()
        (d / "model/config.json").write_text("{}")
        (d / "eval.jsonl").write_text('{"id": "a", "q": "hi"}\n{"id": "b", "q": "yo"}\n')
        self.d = d
        self.calls = []
        calls = self.calls

        class Tok:
            chat_template = "fake"

            def apply_chat_template(self, msgs, tokenize=False, add_generation_prompt=True, **kw):
                calls.append(kw)
                think = "" if kw.get("enable_thinking", True) else "<think>\n\n</think>\n\n"
                return "".join(f"<{m['role']}>{m['content']}" for m in msgs) + "<assistant>" + think

            def __call__(self, text, add_special_tokens=False):
                return {"input_ids": text.split()}

        fake = type(sys)("transformers")
        fake.AutoTokenizer = type("AT", (), {"from_pretrained": staticmethod(lambda p: Tok())})
        self.mods = mock.patch.dict(sys.modules, {"transformers": fake})
        self.mods.start()

    def tearDown(self):
        self.mods.stop()
        self.tmp.cleanup()

    def cfg(self, eval_kw=None, task_kw=None):
        ev = {"set": "eval.jsonl", "tasks": [{"name": "t1", "system": "S", "user_template": "{q}"},
                                             {"name": "t2", "user_template": "{q}", **({"chat_template_kwargs": task_kw} if task_kw else {})}]}
        if eval_kw:
            ev["chat_template_kwargs"] = eval_kw
        c = {"schema": fs.SCHEMA, "out_dir": "out", "model": {"hf_path": "model", "hf_id": "x/m"}, "quants": ["f16"], "eval": ev}
        (self.d / "cfg.yaml").write_text(fs.yaml.safe_dump(c))
        return fs.Cfg(str(self.d / "cfg.yaml"))

    def prompts(self, name):
        return [json.loads(l) for l in (self.d / "out/prompts" / f"{name}.jsonl").read_text().splitlines()]

    def test_default_no_kwargs(self):
        meta = fs.build_prompts(self.cfg())
        self.assertTrue(all(k == {} for k in self.calls))
        self.assertNotIn("chat_template_kwargs", meta)
        self.assertTrue(self.prompts("t1")[0]["prompt"].endswith("<assistant>"))

    def test_eval_kwargs_and_task_override(self):
        meta = fs.build_prompts(self.cfg(eval_kw={"enable_thinking": False}, task_kw={"enable_thinking": True}))
        self.assertEqual(meta["chat_template_kwargs"], {"enable_thinking": False})
        self.assertEqual(meta["tasks"]["t1"]["chat_template_kwargs"], {"enable_thinking": False})
        self.assertEqual(meta["tasks"]["t2"]["chat_template_kwargs"], {"enable_thinking": True})
        self.assertTrue(self.prompts("t1")[0]["prompt"].endswith("<assistant><think>\n\n</think>\n\n"))
        self.assertTrue(self.prompts("t2")[0]["prompt"].endswith("<assistant>"))



class BuildConvertArgs(unittest.TestCase):
    """build passes optional toolchain.convert_args to convert_hf_to_gguf.py (mocked commands; no llama.cpp, no GPU)."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        d = Path(self.tmp.name)
        (d / "model").mkdir()
        (d / "model/config.json").write_text("{}")
        self.d, self.cmds = d, []

    def tearDown(self):
        self.tmp.cleanup()

    def build(self, conv=None):
        tc = {"llama_cpp_dir": str(self.d), "convert_python": "py", "quantize_bin": "q"}
        if conv is not None:
            tc["convert_args"] = conv
        c = {"schema": fs.SCHEMA, "out_dir": "out", "model": {"hf_path": "model", "gguf_basename": "m"}, "quants": ["f16", "Q8_0"], "toolchain": tc}
        (self.d / "cfg.yaml").write_text(fs.yaml.safe_dump(c))
        cfg = fs.Cfg(str(self.d / "cfg.yaml"))

        def fake(cmd, log, nice=True):
            self.cmds.append(cmd)
            out = cmd[cmd.index("--outfile") + 1] if "--outfile" in cmd else cmd[2]
            Path(out).write_text("gguf")
        with mock.patch.object(fs, "run_logged", side_effect=fake), contextlib.redirect_stdout(io.StringIO()):
            fs.cmd_build(cfg, None)
        return json.loads((self.d / "out/manifests/gguf.json").read_text())

    def test_default_no_extra_args(self):
        man = self.build()
        self.assertEqual(self.cmds[0][-2:], ["--outtype", "f16"])
        self.assertTrue(man["convert"].endswith("--outtype f16"))

    def test_convert_args_appended_and_recorded(self):
        man = self.build(["--no-mtp"])
        self.assertEqual(self.cmds[0][-3:], ["--outtype", "f16", "--no-mtp"])
        self.assertNotIn("--no-mtp", self.cmds[1])   # llama-quantize unchanged
        self.assertTrue(man["convert"].endswith("--outtype f16 --no-mtp"))


if __name__ == "__main__":
    unittest.main(verbosity=2)
