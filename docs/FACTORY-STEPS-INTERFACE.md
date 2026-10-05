# Factory steps interface (for agents adding steps)

How the opt-in Cell One factory steps are built, so a new step (for example from Lunatron Corpus) follows the same conventions. The first three steps are documented in [QUANT-LADDER.md](QUANT-LADDER.md). `READY_FOR_LIVE_TEST: no`.

## Where things live

| What | Path |
|---|---|
| Step implementation (one Python module) | `scripts/factory-steps/factory_steps.py` |
| Shell wrappers (one per user-facing step) | `scripts/quant-ladder.sh` (build, compare), `scripts/grammar-eval.sh` (eval), `scripts/determinism-check.sh` (determinism), `scripts/train-step.sh` (train), `scripts/export-step.sh` (export) |
| Unit tests (CPU only, fake llamafactory-cli) | `scripts/factory-steps/test_factory_steps.py` |
| Docs and example config | `docs/QUANT-LADDER.md`, `docs/quant-ladder.example.yaml`, this file |
| A run (config + all outputs) | `.cell/<run-name>/config.yaml`, for example `.cell/lunatron-quant-check-20261002/` |
| Receipts for a run | `.cell/<run-name>/receipts/slotNNN-<slug>.md` (format of `.cell/fm-research-bot/receipts/slot100-b3-eval-v2.md`) |
| Changelog | `CHANGELOG.md`, one bullet per feature under `## Unreleased` / `### Added` |

## Registering a step

1. Write `def cmd_<step>(cfg: Cfg, a) -> int | None` in `factory_steps.py`. Return `None` or `0` for success, or a step-specific non-zero code. Stop with `refuse(msg, code)` (prints `refuse:<msg>` to stderr, exits `code`).
2. Add the name to the argparse `choices` and to the dispatch dict in `main()`:
   `{"build": cmd_build, "eval": cmd_eval, "determinism": cmd_determinism, "compare": cmd_compare}[a.step](cfg, a)`.
   Steps that don't take a quant-ladder config (`train`, `export`) dispatch before `Cfg` is built:
   `{"train": cmd_train, "export": cmd_export}[a.step](None, a)`.
   Step-specific CLI flags go on the same parser (example: `--quant`, repeatable, limits `eval`).
3. Add a wrapper `scripts/<step>.sh CONFIG` by copying `scripts/grammar-eval.sh`. The wrapper:
   - checks the config exists (exit 2)
   - picks `$QUANT_LADDER_PYTHON` (default `~/venvs/llamafactory/bin/python`; needs PyYAML and transformers; exit 1 if missing)
   - records `cksum examples/estate.yaml` before and after, and fails with exit 1 if it changed
   - prints `READY_FOR_LIVE_TEST: no`
   - passes through the Python exit code
4. Add a CHANGELOG bullet, a row in the docs step table, and a receipt. Add files only: don't edit existing lanes, scripts, smoke, gate-90 or GHA.

## Config schema `cell-one.quant-ladder.v0`

`Cfg` refuses (exit 2) unless `schema: cell-one.quant-ladder.v0`. Relative paths resolve against the config file's directory, and `~` is expanded. Read keys with `cfg.get("a", "b", default=...)` and paths with `cfg.p(...)`.

| Key | Meaning (default) |
|---|---|
| `name` | run name |
| `out_dir` | output root (`.`); `gguf/ manifests/ logs/ prompts/ outputs/ metrics/` are created on load |
| `model.hf_id` / `model.hf_path` | HF model id, resolved to exactly one snapshot in `$HF_HUB_CACHE` (never downloads); or a local directory |
| `model.gguf_basename` | GGUF name stem (last part of `hf_id`) |
| `toolchain.llama_cpp_dir`, `convert_python`, `quantize_bin`, `quantize_threads`, `server_bin` | `~/llama.cpp`, `~/venvs/llamafactory/bin/python`, `~/llama.cpp/build/bin/llama-quantize`, 8, `~/llama.cpp/build-cuda/bin/llama-server` |
| `quants` | list; one float type (`f16`/`bf16`/`f32`) is the convert output, the rest go through `llama-quantize` |
| `reference` | reference quant for compare (last quant in the list) |
| `gguf_mirror` | optional copy target (cp, never overwrite; also writes `gguf.json`) |
| `eval.set`, `eval.sha256` | JSONL with `id` plus template fields; a sha mismatch refuses |
| `eval.tasks[]` | `name`, `format`, `where_field` (optional row filter), `system_file` or `system`, `user_template` (Python `format_map` over row fields), `grammar` + `grammar_sha256` or `json_schema` + `json_schema_sha256`, `n_predict` (64) |
| `decode` | `seed` 42, `temperature` 0, `ctx` 4096, `threads` 8, `batch` 2048, `ubatch` 512, `flash_attn` "off", `n_gpu_layers` 999, `port` 18431 |
| `repeats` | N repeats per quant (2) |
| `scorer.python`, `scorer.command` | pluggable scorer (see below) |
| `compare.summary_fields`, `flip_fields`, `agree_fields` | what compare tabulates |
| `gpu_guard.max_util_pct`, `settle_samples`, `max_minutes` | 30, 10, 90 |

New steps should add their own top-level key (for example `corpus:`) and not change the meaning of existing keys. A breaking change needs a new schema string (`...v1`).

## Inputs, outputs, naming

- GGUFs: `gguf/<basename>-<quant>.gguf`. Manifests: `manifests/gguf.json` (HF and GGUF sha256s, llama.cpp commit), `manifests/prompts.json` (eval sha, chat-template sha, per-task counts and file shas), `manifests/eval-run.json` (server version, per-run seconds).
- Prompts: `prompts/<task>.jsonl`, rendered with the model's own chat template: `{id, task, prompt, input_hash, n_prompt_tokens}`.
- Predictions: `outputs/<quant>/rep<N>/<task>.jsonl`, one line per item: `{id, output, format, input_hash, tokens, n_prompt_tokens, n_out_tokens, prompt_ms, predicted_ms, wall_ms, stop_type, truncated}`.
- Metrics: `metrics/<quant>-rep<N>.json`. Reports: `determinism.json` / `DETERMINISM.md`, `ladder.json` / `LADDER.md`. Logs: `logs/build.log`, `logs/server-<quant>-rep<N>.log`, `logs/scorer.log`, `logs/gpu-checks.log`.
- Timestamps are local ISO 8601 with offset. Every output carries a `schema` string `cell-one.quant-ladder.v0.<kind>`.
- Resume: a quant×repeat with complete outputs and metrics is skipped. Build skips GGUFs that already exist.

## Pluggable scorer contract

- `scorer.command` is a shell template. Placeholders: `{python} {run_dir} {metrics} {eval} {config_dir} {out_dir} {quant} {repeat}`.
- It reads `{run_dir}/<task>.jsonl` and writes `{metrics}` as `{"summary": {name: number|string}, "items": {id: {field: value}}}`.
- `compare.flip_fields` must be per-item booleans where true means bad. `agree_fields` are any comparable values.
- Non-zero exit or a missing metrics file stops the eval (exit 1). Domain logic lives only in the scorer (in the run folder), never in `factory_steps.py`.

## GPU guard

- `gpu_check()` runs nvidia-smi before every llama-server start and after the eval. It samples up to `settle_samples` times, 3 s apart, because utilization is a trailing average.
- Any compute app, or utilization still over `max_util_pct`, gives `refuse:gpu-busy` (exit 3). Every check is logged to `logs/gpu-checks.log`.
- Cumulative server time over `max_minutes` gives `refuse:gpu-cap` (exit 4).
- Servers are stopped by their own PID (terminate, then kill after 30 s). Never pkill. The steps never start, stop or remove Ollama models.
- A GPU job still needs the operator's OK for GPU time. CPU-only steps (determinism, compare) never touch the GPU.

## Train and export steps (LLaMA-Factory)

`factory_steps.py train|export --config CONFIG` (wrappers: `scripts/train-step.sh CONFIG [flags]`, `scripts/export-step.sh CONFIG [flags]`).

- **CONFIG** is a LLaMA-Factory yaml (recipe or export yaml, as in `.cell/fm-research-bot/slm-20261001/recipe.yaml` and `.cell/rust-qa-sft-20260930/export.yaml`). It can also be a wrapper with `schema: cell-one.train-step.v0` (or `cell-one.quant-ladder.v0`) holding `train.recipe` / `export.config` (relative to the wrapper), optional `train.max_minutes` / `export.max_minutes`, `train.cli` / `toolchain.llamafactory_cli`, and `gpu_guard`. Paths inside the LLaMA-Factory yaml resolve against the repo root, because the job runs from the repo root. `FACTORY_ROOT` overrides the root (tests only).
- **Validation, train:**
  - `stage` and `do_train` are set; `model_name_or_path` is a local directory with `config.json` and weights (never downloads).
  - `template` and `finetuning_type` are set.
  - `dataset_dir/dataset_info.json` exists, and every name in `dataset` / `eval_dataset` is in it with a local `file_name` that exists (lines and sha256 are reported).
  - `eval_dataset` and `val_size` are not both set; a warning appears when neither is.
  - `output_dir` is inside `.cell/<cell>/` (not `.cell/decisions/` or `.cell/classify-base-cache/`), and a non-empty `output_dir` needs `overwrite_output_dir: true`.
- **Validation, export:**
  - Base model as above.
  - For LoRA, `adapter_name_or_path` holds `adapter_config.json` and `adapter_model.*`.
  - `export_dir` is inside `.cell/<cell>/` and empty (export never overwrites). A warning appears when `export_device` isn't `cpu`.
- **GPU guard:** the same rule as `gpu_check`: any compute app, or utilization over `gpu_guard.max_util_pct` (30) after `settle_samples` re-samples, gives exit 3. The probe is a read-only `nvidia-smi` query; `--dry-run` samples once.
- **Launch:**
  - Command: `cd <root> && setsid nohup <llamafactory-cli> train|export <yaml> > <run>/logs/<step>.log 2>&1`. `<run>` is the parent of `output_dir` / `export_dir`: `.cell/<cell>/outputs` gives `.cell/<cell>`, and nested runs such as `.cell/fm-research-bot/slm-20261001/outputs` give `.cell/fm-research-bot/slm-20261001`, matching the existing cells.
  - The PID goes to `<run>/<step>.pid`. It's the llamafactory-cli PID: setsid execs in place. On one GPU, llamafactory-cli trains in-process.
  - A previous non-empty log is renamed `<step>.log.prev-<timestamp>`. The probe line goes to `<run>/logs/gpu-checks.log`.
  - A record goes to `<run>/<step>-step.json`, holding the command, yaml sha256, pid, started, result, and ended.
  - A job that exits non-zero within `--startup-seconds` (10) gives exit 22.
- **Recipe-supplied chat templates (optional):** a wrapper may set `train.custom_templates` / `export.custom_templates` (fallback: top-level `custom_templates`), a `.py` file relative to the wrapper that calls `llamafactory.data.template.register_template(...)` at import time. It must live in the wrapper, never in the LLaMA-Factory yaml, because LLaMA-Factory refuses unknown keys.
  - With it, the command becomes `setsid nohup <python> scripts/factory-steps/lf_launch.py --custom-templates <file> --cli <llamafactory-cli> train|export <yaml>`. `<python>` is `<step>.python`, else `toolchain.llamafactory_python`, else the `python` next to the cli (not symlink-resolved, so the venv stays active).
  - `lf_launch.py` imports the file, then checks that the yaml's `template` is registered, and refuses with exit 2 if it isn't. It also refuses with exit 2 under `FORCE_TORCHRUN`, `USE_MCA`, `USE_RAY`, `USE_V1`, or more than one visible device for train, because those workers are fresh interpreters that wouldn't see the registration. Otherwise it runs the llamafactory-cli entry in the same process. A launcher refusal shows up as exit 22 (startup window).
  - The cli path stays in argv, so the PID guard for `--status` and the watchdog is unchanged. The plan JSON reports `checks.custom_templates` {path, sha256, template, python, launcher}. There's a warning when the template name isn't a string literal in the file.
  - Without the key, the command is exactly as before. The llamafactory install is never edited. Verify a new template token-for-token against the model's own `apply_chat_template` before training (example: `.cell/lunatron-model-scout-20261002/tools/template_check.py`).
- **Sidecar venv, preload, json patches (optional, additive; receipt `.cell/lunatron-model-scout-20261002/receipts/slot114-nemotron-sidecar-venv.md`):**
  - `train.venv` / `export.venv` (fallback `toolchain.llamafactory_venv`) names a virtualenv (root-relative or `~`, like `toolchain.llamafactory_cli`). The step then uses `<venv>/bin/llamafactory-cli`, and `<venv>/bin/python` for the launcher. Python precedence is `<step>.python`, then the venv's python, then `toolchain.llamafactory_python`, then the cli's sibling. The venv must have `pyvenv.cfg` and both executables. Setting it together with `<step>.cli` is refused. The plan reports `checks.venv`. Use it for a model that needs extra packages (for example mamba-ssm / causal-conv1d) without touching the shared `~/venvs/llamafactory`.
  - `<step>.preload`: a list of `.py` files relative to the wrapper. `lf_launch.py --preload` imports them in order before the templates and the cli, in the same process, for runtime patches such as enabling gradient checkpointing on a model class. Each file's sha256 is in `checks.preload`.
  - `export.json_patches`: `{file.json: {set: {...}, drop: [...]}}`, applied by `lf_launch.py --json-patches` inside `export_dir` after a successful export. Every change is logged. `export.generation_config` is shorthand for `json_patches["generation_config.json"].set` (for example `{eos_token_id: [2, 11]}` so the runtime stops on `<|im_end|>`). Both are refused for train (exit 20).
  - Without these keys, the command is exactly as before.
- **Eval stop strings and converter shim (quant-ladder config, optional):** `decode.stop` (a list of strings) is passed as `stop` in every llama-server `/completion` body. `toolchain.convert_script` replaces `<llama_cpp_dir>/convert_hf_to_gguf.py` in `build`; it must take the same CLI, its path and sha256 go in `manifests/gguf.json`, and a missing file is refused. Without them, build and eval are unchanged.
- **Flags:**

| Flag | Effect |
|---|---|
| `--dry-run` | validate, probe the guard once, print the plan JSON and the exact command; writes nothing, launches nothing |
| `--wait` | after launch, block until the job ends; result `ok` needs rc 0, no Traceback in the log tail, and the artifacts (train: `train_results.json`/`all_results.json` + adapter or model weights; export: `config.json` + `*.safetensors`) |
| `--status` | read the cell pid file: 0 finished ok, 24 finished failed, 25 still running, 26 no pid file; with `--wait`, block until it ends |
| `--max-minutes N` | watchdog while a `--wait` process is attached (launch or `--status --wait`); over N minutes, SIGTERM then SIGKILL after 30 s, **by that PID only**, and only if `/proc/<pid>/cmdline` is llamafactory-cli on this yaml (guards against PID reuse) |
| `--poll-seconds S` | poll interval for `--wait` (30) |
| `--startup-seconds S` | early-exit window after launch (10) |

- A live job in the pid file blocks a second launch (exit 21). A stale pid file is reported and replaced.
- The record and guard follow the quant-ladder conventions. The steps never start, stop, create or remove Ollama models, and never seat or promote.
- After export: build the GGUFs with a `cell-one.quant-ladder.v0` config (`model.hf_path` = export_dir, `gguf_mirror` under `/mnt/nas/projects/lunatron/models/`), then eval. Ollama tags must be new `lunatron-*` names, created by hand with Jason's OK.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | ok |
| 1 | a command failed (convert, quantize, server, scorer), the python is missing, or the estate cksum changed (wrapper) |
| 2 | refuse: bad config, schema, sha mismatch, model not cached, prompt over ctx, usage |
| 3 | `refuse:gpu-busy` |
| 4 | `refuse:gpu-cap` |
| 5 | determinism FAIL |
| 6–19 | reserved for Lunatron Corpus steps (documented by that module) |
| 20 | train/export: validation failed (paths, datasets, output/export dir, missing llamafactory-cli, foreign schema) |
| 21 | train/export: a live llamafactory-cli job already holds the cell pid file |
| 22 | train/export: the job exited non-zero within the startup window |
| 23 | train/export: `--max-minutes` watchdog stopped the job by its PID |
| 24 | train/export: the job ended without its artifacts (non-zero rc, Traceback, missing outputs) |
| 25 | train/export `--status`: still running |
| 26 | train/export `--status`: no pid file |

Codes 27–29 are free in the train/export block. Other new steps should take a free block from 30 upward and document it here.

## Don't touch

- READY flags / `READY_FOR_LIVE_TEST` (always `no`)
- `examples/estate.yaml` (cksum `43770130 3391`)
- `catalog.json`, `desired-snapshot.yaml`, actual-state / placement / `model-actual.json`, lifecycle files
- `.cell/decisions/receipts.jsonl`, other agents' `.cell/` run folders, the genome folder
- `make smoke`, `make gate-90`, GitHub Actions, Auth0, Sanctum, cloud agents
- Existing scripts and lanes (add, don't edit)
- No promotion, seating or applying
- Ollama: never remove models

Git: Jason merges via PRs. Agents don't commit, push or branch; leave changes uncommitted and list them. After harness changes, the live check is `cd .cell/harness-v0-20260929/app && python3 focused_prove.py`, which must print `routing 62/62` and `prove_ok True 453/453`. It loads Ollama models on the GPU, so run it only with an OK for GPU time, and unload with `ollama stop` afterwards.
