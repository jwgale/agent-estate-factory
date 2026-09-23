# Train/enrich prepare

Operator page for the first durable train/enrich beachhead. Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). Product story: [`NORTH-STAR.md`](NORTH-STAR.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md).

`estate enrich prepare` writes artifacts for a purpose-built SLM. It does not train. It does not POST. It does not rewrite `estate.yaml`. Promote stays off.

## What exists today

| Piece | Role |
| --- | --- |
| `TrainEnrichDriver` | Data-plane trait in `model-estate`. `id()`, `prepare(job)`, catalog `status` / `probe`. |
| `ollama-modelfile` | Integration. Writes a Modelfile (`FROM` + `SYSTEM`), `PREPARE.md`, and `NEXT.md` with the exact `ollama create` line. `FROM` is the seated model. Does not shell out. |
| `external-manifest` | Portable JSON and YAML. Base model ref, purpose, host class affinity, dataset path hints from the pack `source_paths`. No vendor lock. `NEXT.md` names the files to hand off. |
| `llamafactory-qlora` | Primary train card. Writes `recipe.yaml` (LLaMA-Factory SFT QLoRA: `quantization_bit: 4`, `quantization_method: bnb`, LoRA rank 16, `cutoff_len` 512, `packing: true`), `export.yaml`, `dataset_info.json`, and instruct chat `dataset.jsonl`. `model_name_or_path` is the train base. The Ollama seat tag stays separate. Default job is `train`. `NEXT.md` has `pip install llamafactory`, `pip install 'bitsandbytes>=0.49'`, `llamafactory-cli train`, and `llamafactory-cli export`. Does not shell out. |
| `axolotl-lora` | YAML recipe for a config-driven or multi-GPU run. Writes `axolotl.yml` (QLoRA: `load_in_4bit: true`, `adapter: qlora`) and Alpaca `dataset.jsonl`. `base_model` is the train base. The Ollama seat tag stays in `prepare.json`. Default job is `train`. `NEXT.md` has the exact `axolotl train` line. Does not shell out. |
| `NEXT.md` | Operator card in the output directory. Artifact paths, the handoff command, the `import-prepared` or `import-trained` line, and the fail-closed reminders. |
| `estate enrich drivers` | Prints the catalog. `live=false`. A probe here does not train. |
| `estate enrich prepare --all-drivers` | One call. Each card the job allows writes a sibling directory. A refuse writes none of them. The enrich default skips `llamafactory-qlora` and `axolotl-lora`. `--job train` includes them. |
| `estate enrich list` | Reads `{state_dir}/enrich/{pack}/{driver}/prepare.json`. Prints pack, driver, job, tag, and out path. Does not create the directory. |
| `estate enrich import-prepared` | Checks `prepare.json` plus the tag and file you created outside the factory. Writes `binding-proposal.json` and `binding-proposal.md` for the existing `local_slm` seat. Does not apply. |
| `estate enrich import-trained` | Same proposal, for a `llamafactory-qlora` or `axolotl-lora` prepare whose job is `train`. `--adapter` is an adapter directory or a merged GGUF / safetensors file. Does not apply. |
| `estate enrich from-pack` | After an accepted pack. Same prepare, into `{state_dir}/enrich/{pack}/{driver}`. Omitting `--driver` prepares every card the job allows. The default job is enrich, so the train cards wait for `--job train`. Does not apply. |
| `estate enrich apply-proposal` | Reads that proposal. Checks schema, curator, sacred, hardware, frontier, and `prepare.json`. Writes `{state}/enrich-stage/staged-estate.yaml` for `estate plan` and `estate apply --require-plan`. Does not apply. Does not rewrite the source estate. |
| `estate help enrich` | Same page as `estate help train`. |
| `make enrich-prepare` | Opt-in fixture walk. Not in `make smoke`, `make gate-90`, or GitHub Actions. |
| `make train-prepare` | Opt-in LLaMA-Factory recipe and Axolotl recipe. Prints `SKIP live train`. Does not run either trainer. Not in `make smoke`, `make gate-90`, or GitHub Actions. |
| `make enrich-live-prove` | Opt-in seated handoff. Runs `ollama create` when the seat is up, then removes the tag. Not a factory-wide live test. Off smoke, `gate-90`, and Actions. |

Job field: `enrich` (default) or `train`. `llamafactory-qlora` and `axolotl-lora` default to `train` and refuse `enrich`. Both jobs only prepare. The factory does not run a trainer.

Default output is `.cell/enrich/{pack_id}/{driver}/`. Pass `--out` to write somewhere else, including `packs/prepared/`. Layout: [`cell-layout.md`](cell-layout.md). Schema: [`../schema/train-enrich.v0.json`](../schema/train-enrich.v0.json) (`cell-one.enrich-prepare.v0`).

## Facilitated vs invented

Ollama already creates a model from a Modelfile. This factory writes that file and the next command. It does not invent a local inference server.

LLaMA-Factory already runs QLoRA supervised fine-tuning from a YAML recipe. `llamafactory-qlora` writes that recipe and the `llamafactory-cli train` line. Axolotl already trains from a YAML recipe, including multi-GPU runs. `axolotl-lora` writes that recipe and the `axolotl train` line. Neither card shells out. Unsloth QLoRA stays a `NEXT.md` pointer for a faster single-GPU Nvidia run. This factory does not invent an in-process trainer, a dataset downloader, a GGUF exporter, or a GPU scheduler.

## Train / fine-tune

Supported train hosts for `llamafactory-qlora` and `axolotl-lora` are `consumer-nvidia` and `rented-nvidia`. `host_class_affinity` comes from the pack when that field is set, otherwise from `params.host_class` on `local_slm`, otherwise from the pack `host_class`. An `apple-silicon` affinity still prepares. `NEXT.md` says the LLaMA-Factory card expects CUDA LLaMA-Factory. This factory does not write an MLX trainer.

The seat tag and the train base are two fields. `llamafactory-qlora` and `axolotl-lora` both use that split.

The seat tag is the Ollama id for Modelfile `FROM`. Resolution: a pack `model_hint` that is already a model tag, otherwise `params.model` on the local binding. `prepare.json` stores it as `base_model` and `seat_tag`. The binding id `local_slm` is `refuse:base-model` and writes nothing.

The train base is `model_name_or_path` in `recipe.yaml` and `export.yaml`, and `base_model` in `axolotl.yml`. It is a Hugging Face repo id (`namespace/name`) or a local directory of HF weights (an absolute path, or a path that starts with `./` or `../`). Set it on the pack as `train_base_model`, or on the local binding as `params.train_base_model`. The pack field wins when both are set. A relative directory is stored as an absolute path in `recipe.yaml`, `export.yaml`, `axolotl.yml`, `prepare.json`, and `NEXT.md`. LLaMA-Factory resolves a relative `model_name_or_path` from the process working directory, and Axolotl resolves a relative `base_model` the same way, so both recipes keep the absolute path. The directory does not need to exist at prepare time. `prepare.json` stores the resolved value as `train_base_model`. On the LLaMA-Factory card, `template` is inferred from that train base. A 5090 smoke seated `llama3` and trained `Qwen/Qwen2.5-0.5B-Instruct` with template `qwen`. That repo id is an example an operator supplies. This factory does not turn the seat tag `llama3` into a Llama-3 Hub repo, and it does not download weights.

A missing train base, a bare Ollama tag (`llama3`, `llama3:latest`), or a local path whose directory name is an Ollama seat tag (`./llama3`, `../llama3`) is `refuse:train-base` and writes nothing. That refuse applies to `llamafactory-qlora` and to `axolotl-lora`. `--all-drivers --job train` refuses the whole set in that case, so no sibling directory is left behind.

`base_model` in `axolotl.yml` is that same train base. `prepare.json` `base_model` and `seat_tag` stay the Ollama id for Modelfile `FROM` and for the adapter join. `NEXT.md` names the seat tag and the train base.

On `llamafactory-qlora`, `dataset.jsonl` is instruct chat JSONL (`messages` of `role` and `content`). `dataset_info.json` marks it sharegpt so LLaMA-Factory applies the recipe `template`. That same chat template is what you seat. On `axolotl-lora`, `dataset.jsonl` is Alpaca JSONL (`instruction`, `input`, `output`), which Axolotl reads with `type: alpaca` and `ds_type: json`. When the pack has `source_paths`, each row names one of those paths. The factory does not read or download the files. When `source_paths` is empty, the file is a three-row stub and `NEXT.md` says to replace the rows. An empty source path string is `refuse:dataset`.

The LLaMA-Factory recipe is QLoRA (`finetuning_type: lora`, `quantization_bit: 4`, `quantization_method: bnb`, `lora_rank: 16`, `packing: true`). `bnb` is the LLaMA-Factory 0.9 token that selects the 4-bit bitsandbytes branch. `cutoff_len` is `512` so a first run stays short. Official SFT examples use `2048` for a longer run. Raise that field before a real run. The default recipe sets `num_train_epochs: 1.0` and `save_steps: 50`, and leaves `max_steps` unset. A short gauge run passes `--max-steps 10`. LLaMA-Factory overrides `num_train_epochs` when `max_steps` is set. When that count is under 50, prepare also sets `save_steps` to the same count so a checkpoint exists during the short run. A later preference stage (`stage: dpo` or `stage: orpo`, with `ranking: true` in `dataset_info.json`) is a comment in the recipe. This card does not build that dataset. The Axolotl recipe is QLoRA (`load_in_4bit: true`, `adapter: qlora`). On the CUDA host you can edit the yaml to 8-bit LoRA (`load_in_8bit: true`, `load_in_4bit: false`, `adapter: lora`) before you run Axolotl.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id-or-pack.json> \
  --driver llamafactory-qlora \
  --job train \
  --state-dir .cell
```

Omitting `--job` on `--driver llamafactory-qlora` is the same train job. `--job enrich` is `refuse:job` and writes nothing. `--all-drivers --job train` writes both train cards next to the Modelfile and the external manifest. `--all-drivers` without `--job` stays on `enrich` and skips the train cards.

The output directory is `.cell/enrich/{pack_id}/llamafactory-qlora/`:

| File | Role |
| --- | --- |
| `recipe.yaml` | LLaMA-Factory SFT QLoRA recipe. Dataset dir and output dir are absolute. |
| `export.yaml` | Merge recipe. No `quantization_bit`. |
| `dataset_info.json` | ShareGPT column map for `dataset.jsonl`. |
| `dataset.jsonl` | Instruct chat scaffold. |
| `PREPARE.md` | What this step wrote, including `llamafactory-cli train recipe.yaml` from that directory. |
| `NEXT.md` | `pip install llamafactory`, `pip install 'bitsandbytes>=0.49'`, `llamafactory-cli train <absolute>/recipe.yaml`, `llamafactory-cli export`, the seat tag, the train base, the gauge note, and the import line. |
| `prepare.json` | `job` is `train`. `base_model` and `seat_tag` are the Ollama id. `train_base_model` is the Hugging Face repo or local HF directory. `promoted`, `auto_apply`, and `estate_rewritten` are false. |

Run the commands from `NEXT.md` on a CUDA host. This factory does not run them.

```bash
pip install llamafactory
pip install 'bitsandbytes>=0.49'
llamafactory-cli train .cell/enrich/<pack-id>/llamafactory-qlora/recipe.yaml
llamafactory-cli export .cell/enrich/<pack-id>/llamafactory-qlora/export.yaml
```

QLoRA needs bitsandbytes. `pip install llamafactory` and `llamafactory[torch,metrics]` 0.9.5 did not install it. Install bitsandbytes in that same environment. On a consumer RTX host, keep the torch CUDA wheel you already installed. A 5090 smoke used torch 2.11.0+cu128 (CUDA 12.8) and bitsandbytes 0.50.2. That bitsandbytes install did not replace torch. This factory does not install either package. If the torch wheel still does not match the CUDA install on the box, follow https://github.com/hiyouga/LLaMA-Factory#installation.

A short gauge run adds `--max-steps 10` to the prepare command. The default recipe stays one epoch.

The train writes the LoRA adapter under `outputs/` (`adapter_config.json` and the adapter weights). Merge with the export file. Do not set `quantization_bit` on that merge, and do not merge a quantized base. LLaMA-Factory does not write GGUF. After the merge, convert with llama.cpp if you want a GGUF, then seat tag `cell-enrich-{pack_id}` on Ollama with `FROM` that GGUF. To load the adapter without a merge, `FROM` must be an Ollama model of the same train base, plus `ADAPTER` for the adapter directory. The seat tag is the id the cell already runs. Use the chat template the recipe named. After the tag is seated, send a short prompt that checks the pack purpose. This factory does not run `ollama create` and does not run that smoke eval.

On Nvidia only, Unsloth QLoRA is a faster single-GPU alternate. `NEXT.md` points at the Unsloth docs. This card does not call Unsloth and does not write a script.

`axolotl-lora` is the same loop with a YAML recipe. Use it when you want a config file or a multi-GPU run. `base_model` in `axolotl.yml` is the train base. The seat tag stays in `prepare.json` for Ollama seating.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id-or-pack.json> \
  --driver axolotl-lora \
  --job train \
  --state-dir .cell

axolotl train .cell/enrich/<pack-id>/axolotl-lora/axolotl.yml
```

That directory holds `axolotl.yml` and an Alpaca `dataset.jsonl`. `val_set_size` is `0.0` so a short scaffold does not try to split an eval set. Axolotl writes the adapter under `output_dir` in the yaml. `prepare.json` stores `base_model` and `seat_tag` as the Ollama id, and `train_base_model` as the value written to `base_model` in the yaml. After training, seat tag `cell-enrich-{pack_id}` on Ollama with `FROM` a merged GGUF, or `FROM` an Ollama model of this same train base plus `ADAPTER` for the adapter directory. The seat tag already on the cell is the id Ollama is running. This factory does not run `ollama create`.

When the adapter directory or a GGUF exists, record the join. The prepared directory is the one you trained from.

```bash
estate enrich import-trained \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --tag cell-enrich-<pack-id> \
  --adapter <adapter-dir-or-gguf>

estate enrich apply-proposal \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --tag cell-enrich-<pack-id> \
  --state-dir .cell

estate plan --estate .cell/enrich-stage/staged-estate.yaml --state-dir .cell
estate apply --estate .cell/enrich-stage/staged-estate.yaml --state-dir .cell --require-plan
```

`import-trained` writes the same `binding-proposal.json` as `import-prepared`. A prepare that is not `llamafactory-qlora` or `axolotl-lora`, a job that is not `train`, or an adapter path with no `adapter_config.json`, adapter weights, or GGUF is a refuse before that proposal exists. `apply-proposal` does not apply. The source estate is written only when `estate apply --require-plan` succeeds. Point `--estate` at a lab copy. `examples/estate.yaml` on `main` stays hash-locked. There is no second apply path and no auto-promote.

```bash
make train-prepare
```

Uses `examples/fixtures/specialist-overnight.pack.json`. Copies `examples/estate.yaml` into `/tmp/cell-one-train-prepare` (or `$TMPDIR`). A copy with only `params.model: llama3` is `refuse:train-base` and writes nothing. The success copy also sets `params.train_base_model` to `Qwen/Qwen2.5-0.5B-Instruct`. That prepare asserts `recipe.yaml` (`template: qwen`, `quantization_method: bnb`, no `max_steps`), the chat `dataset.jsonl`, the `llamafactory-cli train` line and the bitsandbytes install line in `NEXT.md`, and `prepare.json` with `job` `train`, `base_model` `llama3`, and that train base. A second prepare with `--max-steps 10` writes `max_steps` and `save_steps` 10. `axolotl.yml` keeps an indented datasets list and sets `base_model` to that same train base. `prepare.json` for that card keeps `base_model` `llama3` and `seat_tag` `llama3`. A seat-only copy is `refuse:train-base` for `axolotl-lora` too. `--all-drivers --job train` writes the train base into `axolotl.yml` and leaves Modelfile `FROM` as `llama3`. A stock estate is `refuse:base-model` and writes nothing. `--job enrich` is `refuse:job` and writes nothing. `import-trained` on a fixture adapter directory writes the proposal and does not apply. Leaves `examples/estate.yaml` unchanged. Prints `SKIP live train`. Does not run LLaMA-Factory or Axolotl. Not in `make smoke`, `make gate-90`, or GitHub Actions.

```bash
estate enrich from-pack \
  --estate <your-estate.yaml> \
  --pack overnight-traces \
  --state-dir .cell

estate enrich list --state-dir .cell
```

`from-pack` reads a pack id from `packs/` or `packs/accepted/`, or a pack JSON path. It writes `.cell/enrich/{pack_id}/{driver}/`. `--driver ollama-modelfile` writes one card. `--driver` together with `--all-drivers` is `refuse:driver`.

`examples/estate.yaml` is hash-locked and does not set `params.model` on `local_slm`. The specialist fixture's `model_hint` is the binding id. Prepare on that pair is `refuse:base-model` and writes nothing. Copy the estate, set `params.model` to a model the seat already has, and prepare that copy. `make enrich-prepare` does this under `/tmp/cell-one-enrich-prepare` (or `$TMPDIR`) and leaves the example file alone.

`list` prints one row per prepared driver. A missing `.cell/enrich` is `refuse:enrich-index`. The command does not create that directory.

`NEXT.md` for `ollama-modelfile` prints the create line with the Modelfile path. Run that on the seated host. This factory does not run it.

```bash
ollama create cell-enrich-overnight-traces -f .cell/enrich/overnight-traces/ollama-modelfile/Modelfile
```

The local tag is `cell-enrich-{pack_id}`. `FROM` is the seated model the runtime already has. Resolution order:

1. Pack `model_hint`, when that hint is already a model tag (`llama3`). A hint must be a slug, so `llama3:latest` belongs on the binding, not the pack.
2. Otherwise `params.model` on the local binding the hint names, or on `local_slm` when the hint is empty or a driver id such as `ollama`.

The binding id `local_slm` is not a model tag. `params.model: local_slm` is `refuse:base-model`. A missing `params.model` with no model-tag hint is the same refuse, before any output directory. A hardware SKU in the resolved name is `refuse:sku-banned`. The factory does not pull weights.

After that model exists, record the join. The tag must match. `--path` must be a file.

```bash
estate enrich import-prepared \
  --estate examples/estate.yaml \
  --prepared .cell/enrich/overnight-traces/ollama-modelfile \
  --tag cell-enrich-overnight-traces \
  --path .cell/enrich/overnight-traces/ollama-modelfile/Modelfile
```

That writes `binding-proposal.json` and `binding-proposal.md` in the prepared directory. The snippet names the existing `local_slm` seat and keeps the id. `params.model` is the tag. Other params on that binding are copied. `import-prepared` does not apply.

Stage that proposal into the plan input. The source estate stays as it is.

```bash
estate enrich apply-proposal \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/overnight-traces/ollama-modelfile \
  --tag cell-enrich-overnight-traces \
  --state-dir .cell
```

That writes `.cell/enrich-stage/staged-estate.yaml` and `.cell/enrich-stage/stage.json` (`cell-one.enrich-binding-stage.v0`). `auto_apply` stays false. Running it again with the same tag and binding prints `no-op:`. A missing proposal, a prepare.json that does not match, a wrong tag, or a stale estate hash refuses before that directory exists.

`--verify-local-tag` is off unless you pass it. When you do, the command reads the seat's `endpoint_env` (normally `CELL_LOCAL_ENDPOINT`) and checks that the seated runtime lists the tag, using the same OpenAI `/v1/models` or Ollama `/api/tags` read as the live probe. A down runtime or a missing tag refuses before the stage exists. The flag does not start a server.

Then use the staged file with the commands you already use:

```bash
estate plan --estate .cell/enrich-stage/staged-estate.yaml --state-dir .cell
estate apply --estate .cell/enrich-stage/staged-estate.yaml --state-dir .cell --require-plan
```

`apply` without `--require-plan` converges the cell and leaves the source estate unchanged. `apply --require-plan` copies the staged estate onto `<your-estate.yaml>` only after that apply succeeds. Leave the hash-locked example on `main` alone; point `--estate` at a lab copy.

`import-prepared` and `apply-proposal` do not run plan or apply. `auto_apply`, `promoted`, and the proposal's `estate_rewritten` stay false. A catalog file is not written by those two commands.

## Swap a driver

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack examples/fixtures/specialist-overnight.pack.json \
  --driver external-manifest \
  --out target/enrich-manifest
```

`<your-estate.yaml>` needs `params.model` on `local_slm`, or a pack `model_hint` that is already a model tag. The hash-locked example has neither, so that pair is `refuse:base-model`.

`manifest.json` and `manifest.yaml` are the market-shift hatch. A later trainer reads `base_model`, `purpose`, `host_class_affinity`, and `dataset_paths`. Cell One does not call that trainer. When weights come back, load them on the seated runtime as `cell-enrich-{pack_id}` and point `import-prepared --path` at that file.

## Add another entrant

1. Implement `TrainEnrichDriver` in `model-estate` (a sibling of the cards already registered).
2. Add one `RegisteredDriver` card in `crates/model-estate/src/train_enrich.rs`. Set `jobs` and `default_job` so `--all-drivers` includes the card only when the job allows it.
3. Leave floor-supervisor and `estate-control` dispatch alone. They pass the driver id through.

`estate enrich drivers` prints the new card. No estate file change is required to register it.

llama.cpp on a host already reads an INI preset (`llama-server --models-preset`). That preset needs a GGUF path or a Hugging Face repo. This estate does not carry either, so that preset stays off this catalog. `external-manifest` stays the portable hatch until a pack or binding names that file.

## Refuse

Prepare loads the estate the same way pack import does: parsed, then the enrich gates, before the Cell One equal-class shape check. A local-only estate can still hit `refuse:frontier-invent`.

| Condition | Code |
| --- | --- |
| Pack id or path is missing | `refuse:missing-pack` |
| Curator is not `jason`, or the estate curator is not `jason` | `refuse:curator` |
| Sacred token in the pack note, system text, or paths | `refuse:sacred` |
| Hardware SKU in a model id or path | `refuse:sku-banned` or `refuse:model-hint` / `refuse:source-path` |
| Pack tags `frontier` and the estate has no frontier binding | `refuse:frontier-invent` |
| Unknown driver id | `refuse:driver` |
| `--driver` and `--all-drivers` together | `refuse:driver` |
| Job is not `train` or `enrich` | `refuse:job` |
| `llamafactory-qlora` or `axolotl-lora` with `--job enrich` | `refuse:job` |
| A train source path is empty | `refuse:dataset` |
| `import-trained` on a prepare that is not a train recipe with job `train` | `refuse:driver` or `refuse:job` |
| Adapter path is missing, or has no adapter config, weights, or GGUF | `refuse:adapter` |
| `{state_dir}/enrich` is missing on list | `refuse:enrich-index` |
| `prepare.json` missing, unreadable, or flagged promoted | `refuse:missing-prepare`, `refuse:prepare-unreadable`, `refuse:prepared` |
| Tag is not `cell-enrich-{pack_id}` | `refuse:tag` |
| Operator path is missing or not a file | `refuse:path` |
| Estate has no `local_slm` seat | `refuse:binding` |
| `FROM` would be a binding id, a driver id, or empty | `refuse:base-model` |
| `llamafactory-qlora` or `axolotl-lora` has no train base, or the value is a bare Ollama tag or a seat-looking local leaf, or `model_name_or_path` / `axolotl.yml` `base_model` does not match `prepare.json` `train_base_model` | `refuse:train-base` |
| `--max-steps 0` | `refuse:max-steps` |
| `binding-proposal.json` is missing | `refuse:missing-proposal` |
| Proposal schema, flags, or curator are wrong | `refuse:proposal` or `refuse:curator` |
| `prepare.json` does not match the proposal | `refuse:prepare` |
| Proposal hash does not match the estate | `refuse:estate-hash` |
| A different pending stage is already on disk | `refuse:stage` |
| `--verify-local-tag` and the tag is not seated, or the endpoint is unset | `refuse:local-tag` |

Those prepare stops happen before the output directory is created. `--all-drivers` stages every card first, so one refuse leaves no sibling directory from that call. List does not create `.cell/enrich`. Import writes the proposal only after the gates pass. `apply-proposal` writes `enrich-stage/` only after its gates pass. Same tag and binding again is a no-op and does not rewrite the stage.

## Opt-in walk

```bash
make enrich-prepare
```

Uses `examples/fixtures/specialist-overnight.pack.json`. Copies `examples/estate.yaml` into `/tmp/cell-one-enrich-prepare` (or `$TMPDIR`) and sets `params.model: llama3` on that copy. The output path must not contain a hardware SKU. The hash-locked example has no `params.model`, so prepare against it is `refuse:base-model` and writes nothing. The copy then runs both drivers, `estate enrich list`, and `estate enrich import-prepared`. Asserts `FROM llama3`, an external manifest, `NEXT.md`, the index, and a binding proposal. A wrong tag refuses before `binding-proposal.json` exists. It then runs `estate enrich apply-proposal`, `estate plan`, and `estate apply --require-plan` on that seated copy. A missing proposal and an unset `--verify-local-tag` refuse before `enrich-stage/` exists. Apply without `--require-plan` leaves the lab copy unchanged. `--require-plan` writes `cell-enrich-overnight-traces` into that copy. Leaves `examples/estate.yaml` unchanged. Does not need an Ollama binary. The script prints `SKIP live train` because this walk is not a train.

`make real-world` points here and does not run a train. See [`OPERATOR-DAY.md`](OPERATOR-DAY.md).

## Opt-in live handoff

```bash
make enrich-live-prove
```

When `ollama list` works, the script copies the example estate into `/tmp/cell-one-enrich-live-prove` (or `$TMPDIR`), sets `params.model` from `CELL_LOCAL_MODEL` or from a model already on the seat (`llama3` when that tag is present), and runs `estate enrich from-pack --all-drivers`. It then runs `ollama create cell-enrich-<pack> -f Modelfile`, `ollama show`, `estate enrich import-prepared`, and removes the tag it created. `examples/estate.yaml` stays untouched. If `ollama` is missing or the seat is down, the script prints `SKIP` and exits 0. That skip is not a pass.

This is an opt-in seated-runtime enrich handoff. It is not a factory-wide live test. It is not in `make smoke`, `make gate-90`, or GitHub Actions. Paste target: [`LIVE-PROBES.md`](LIVE-PROBES.md).

`READY_FOR_LIVE_TEST`: no.
