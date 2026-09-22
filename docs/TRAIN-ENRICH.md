# Train/enrich prepare

Operator page for the first durable train/enrich beachhead. Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). Product story: [`NORTH-STAR.md`](NORTH-STAR.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md).

`estate enrich prepare` writes artifacts for a purpose-built SLM. It does not train. It does not POST. It does not rewrite `estate.yaml`. Promote stays off.

## What exists today

| Piece | Role |
| --- | --- |
| `TrainEnrichDriver` | Data-plane trait in `model-estate`. `id()`, `prepare(job)`, catalog `status` / `probe`. |
| `ollama-modelfile` | Integration. Writes a Modelfile (`FROM` + `SYSTEM`), `PREPARE.md`, and `NEXT.md` with the exact `ollama create` line. `FROM` is the seated model. Does not shell out. |
| `external-manifest` | Portable JSON and YAML. Base model ref, purpose, host class affinity, dataset path hints from the pack `source_paths`. No vendor lock. `NEXT.md` names the files to hand off. |
| `unsloth-qlora` | Primary train card. Writes `train_unsloth.py` (QLoRA: `load_in_4bit=True`, LoRA `r=16`, `max_seq_length` 512) and instruct chat `dataset.jsonl`. Default job is `train`. `NEXT.md` has `pip install unsloth` and `python train_unsloth.py`. Does not shell out. |
| `axolotl-lora` | YAML recipe for a config-driven or multi-GPU run. Writes `axolotl.yml` (QLoRA: `load_in_4bit: true`, `adapter: qlora`) and Alpaca `dataset.jsonl`. Default job is `train`. `NEXT.md` has the exact `axolotl train` line. Does not shell out. |
| `NEXT.md` | Operator card in the output directory. Artifact paths, the handoff command, the `import-prepared` or `import-trained` line, and the fail-closed reminders. |
| `estate enrich drivers` | Prints the catalog. `live=false`. A probe here does not train. |
| `estate enrich prepare --all-drivers` | One call. Each card the job allows writes a sibling directory. A refuse writes none of them. The enrich default skips `unsloth-qlora` and `axolotl-lora`. `--job train` includes them. |
| `estate enrich list` | Reads `{state_dir}/enrich/{pack}/{driver}/prepare.json`. Prints pack, driver, job, tag, and out path. Does not create the directory. |
| `estate enrich import-prepared` | Checks `prepare.json` plus the tag and file you created outside the factory. Writes `binding-proposal.json` and `binding-proposal.md` for the existing `local_slm` seat. Does not apply. |
| `estate enrich import-trained` | Same proposal, for an `unsloth-qlora` or `axolotl-lora` prepare whose job is `train`. `--adapter` is an adapter directory or a merged GGUF / safetensors file. Does not apply. |
| `estate enrich from-pack` | After an accepted pack. Same prepare, into `{state_dir}/enrich/{pack}/{driver}`. Omitting `--driver` prepares every card the job allows. The default job is enrich, so the train cards wait for `--job train`. Does not apply. |
| `estate enrich apply-proposal` | Reads that proposal. Checks schema, curator, sacred, hardware, frontier, and `prepare.json`. Writes `{state}/enrich-stage/staged-estate.yaml` for `estate plan` and `estate apply --require-plan`. Does not apply. Does not rewrite the source estate. |
| `estate help enrich` | Same page as `estate help train`. |
| `make enrich-prepare` | Opt-in fixture walk. Not in `make smoke`, `make gate-90`, or GitHub Actions. |
| `make train-prepare` | Opt-in Unsloth script and Axolotl recipe. Prints `SKIP live train`. Does not run either trainer. Not in `make smoke`, `make gate-90`, or GitHub Actions. |
| `make enrich-live-prove` | Opt-in seated handoff. Runs `ollama create` when the seat is up, then removes the tag. Not a factory-wide live test. Off smoke, `gate-90`, and Actions. |

Job field: `enrich` (default) or `train`. `unsloth-qlora` and `axolotl-lora` default to `train` and refuse `enrich`. Both jobs only prepare. The factory does not run a trainer.

Default output is `.cell/enrich/{pack_id}/{driver}/`. Pass `--out` to write somewhere else, including `packs/prepared/`. Layout: [`cell-layout.md`](cell-layout.md). Schema: [`../schema/train-enrich.v0.json`](../schema/train-enrich.v0.json) (`cell-one.enrich-prepare.v0`).

## Facilitated vs invented

Ollama already creates a model from a Modelfile. This factory writes that file and the next command. It does not invent a local inference server.

Unsloth already runs QLoRA supervised fine-tuning on one CUDA GPU. `unsloth-qlora` writes that script and the `python train_unsloth.py` line. Axolotl already trains from a YAML recipe, including multi-GPU runs. `axolotl-lora` writes that recipe and the `axolotl train` line. Neither card shells out. This factory does not invent an in-process trainer, a dataset downloader, a GGUF exporter, or a GPU scheduler.

## Train / fine-tune

Supported train hosts for `unsloth-qlora` and `axolotl-lora` are `consumer-nvidia` and `rented-nvidia`. `host_class_affinity` comes from the pack when that field is set, otherwise from `params.host_class` on `local_slm`, otherwise from the pack `host_class`. An `apple-silicon` affinity still prepares. `NEXT.md` says the card expects a CUDA host. This factory does not write an MLX trainer.

`MODEL_NAME` in `train_unsloth.py`, and `base_model` in `axolotl.yml`, are the seated model tag. Same resolution as Modelfile `FROM`: a pack `model_hint` that is already a model tag, otherwise `params.model` on the local binding. The binding id `local_slm` is `refuse:base-model` and writes nothing. Unsloth expects a Hugging Face repo id. Prefer an instruct model, often an `unsloth/*-bnb-4bit` id. Axolotl expects a Hugging Face repo id or a local weights directory. If the seated tag is only an Ollama name, edit that field before you train. This factory does not download weights and does not map the tag.

On `unsloth-qlora`, `dataset.jsonl` is instruct chat JSONL (`messages` of `role` and `content`). The script applies the tokenizer chat template into a `text` field, which is what Unsloth's SFT examples train. On `axolotl-lora`, `dataset.jsonl` is Alpaca JSONL (`instruction`, `input`, `output`), which Axolotl reads with `type: alpaca` and `ds_type: json`. When the pack has `source_paths`, each row names one of those paths. The factory does not read or download the files. When `source_paths` is empty, the file is a three-row stub and `NEXT.md` says to replace the rows. An empty source path string is `refuse:dataset`.

The Unsloth script is QLoRA (`load_in_4bit=True`, LoRA `r=16`, `lora_alpha=16`, `lora_dropout=0`). `MAX_SEQ_LENGTH` is `512` so a first run stays short. Unsloth's fine-tuning guide uses `2048` for a longer test. Raise that constant before a real run. The Axolotl recipe is QLoRA (`load_in_4bit: true`, `adapter: qlora`). On the CUDA host you can edit the yaml to 8-bit LoRA (`load_in_8bit: true`, `load_in_4bit: false`, `adapter: lora`) before you run Axolotl.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id-or-pack.json> \
  --driver unsloth-qlora \
  --job train \
  --state-dir .cell
```

Omitting `--job` on `--driver unsloth-qlora` is the same train job. `--job enrich` is `refuse:job` and writes nothing. `--all-drivers --job train` writes both train cards next to the Modelfile and the external manifest. `--all-drivers` without `--job` stays on `enrich` and skips the train cards.

The output directory is `.cell/enrich/{pack_id}/unsloth-qlora/`:

| File | Role |
| --- | --- |
| `train_unsloth.py` | Unsloth QLoRA entrypoint. Dataset and output paths are absolute. |
| `dataset.jsonl` | Instruct chat scaffold the script loads. |
| `PREPARE.md` | What this step wrote, including `python train_unsloth.py` from that directory. |
| `NEXT.md` | `pip install unsloth` and `python <absolute>/train_unsloth.py`, the host note, the GGUF/Ollama doc links, and the import line. |
| `prepare.json` | `job` is `train`. `promoted`, `auto_apply`, and `estate_rewritten` are false. |

Run the command from `NEXT.md` on a CUDA host. This factory does not run it.

```bash
pip install unsloth
python .cell/enrich/<pack-id>/unsloth-qlora/train_unsloth.py
```

If that pip line does not match the CUDA wheel on the box, follow https://unsloth.ai/docs/get-started/install. The script saves the LoRA adapter under `outputs/` (`adapter_config.json` and the adapter weights). Ollama stays the local-run seat. Unsloth documents the GGUF and Ollama export. This factory does not export and does not run `ollama create`.

- https://unsloth.ai/docs/basics/inference-and-deployment/saving-to-gguf
- https://unsloth.ai/docs/basics/inference-and-deployment/saving-to-ollama

Their local GGUF call, which you run yourself after training, is `model.save_pretrained_gguf("directory", tokenizer, quantization_method="q4_k_m")`. Then create tag `cell-enrich-{pack_id}` on Ollama from the Modelfile Unsloth writes, or `FROM` the GGUF.

`axolotl-lora` is the same loop with a YAML recipe. Use it when you want a config file or a multi-GPU run.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id-or-pack.json> \
  --driver axolotl-lora \
  --job train \
  --state-dir .cell

axolotl train .cell/enrich/<pack-id>/axolotl-lora/axolotl.yml
```

That directory holds `axolotl.yml` and an Alpaca `dataset.jsonl`. `val_set_size` is `0.0` so a short scaffold does not try to split an eval set. Axolotl writes the adapter under `output_dir` in the yaml.

When the adapter directory or a GGUF exists, record the join. The prepared directory is the one you trained from.

```bash
estate enrich import-trained \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/unsloth-qlora \
  --tag cell-enrich-<pack-id> \
  --adapter <adapter-dir-or-gguf>

estate enrich apply-proposal \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/unsloth-qlora \
  --tag cell-enrich-<pack-id> \
  --state-dir .cell

estate plan --estate .cell/enrich-stage/staged-estate.yaml --state-dir .cell
estate apply --estate .cell/enrich-stage/staged-estate.yaml --state-dir .cell --require-plan
```

`import-trained` writes the same `binding-proposal.json` as `import-prepared`. A prepare that is not `unsloth-qlora` or `axolotl-lora`, a job that is not `train`, or an adapter path with no `adapter_config.json`, adapter weights, or GGUF is a refuse before that proposal exists. `apply-proposal` does not apply. The source estate is written only when `estate apply --require-plan` succeeds. Point `--estate` at a lab copy. `examples/estate.yaml` on `main` stays hash-locked.

```bash
make train-prepare
```

Uses `examples/fixtures/specialist-overnight.pack.json`. Copies `examples/estate.yaml` into `/tmp/cell-one-train-prepare` (or `$TMPDIR`) and sets `params.model: llama3` on that copy. Asserts `train_unsloth.py`, the chat `dataset.jsonl`, the `python train_unsloth.py` line in `NEXT.md`, `prepare.json` with `job` `train`, and that `axolotl.yml` still has an indented datasets list. A stock estate is `refuse:base-model` and writes nothing. `--job enrich` is `refuse:job` and writes nothing. `import-trained` on a fixture adapter directory writes the proposal and does not apply. Leaves `examples/estate.yaml` unchanged. Prints `SKIP live train`. Does not run Unsloth or Axolotl. Not in `make smoke`, `make gate-90`, or GitHub Actions.

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
| `unsloth-qlora` or `axolotl-lora` with `--job enrich` | `refuse:job` |
| A train source path is empty | `refuse:dataset` |
| `import-trained` on a prepare that is not a train recipe with job `train` | `refuse:driver` or `refuse:job` |
| Adapter path is missing, or has no adapter config, weights, or GGUF | `refuse:adapter` |
| `{state_dir}/enrich` is missing on list | `refuse:enrich-index` |
| `prepare.json` missing, unreadable, or flagged promoted | `refuse:missing-prepare`, `refuse:prepare-unreadable`, `refuse:prepared` |
| Tag is not `cell-enrich-{pack_id}` | `refuse:tag` |
| Operator path is missing or not a file | `refuse:path` |
| Estate has no `local_slm` seat | `refuse:binding` |
| `FROM` would be a binding id, a driver id, or empty | `refuse:base-model` |
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
