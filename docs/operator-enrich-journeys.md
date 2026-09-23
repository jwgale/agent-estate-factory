# Operator journeys: train/enrich

Walks for the suite goal: facilitate train and enrich of purpose-built small-parameter models. Today's beachhead is enrich packs, the specialist path, and prepare cards the operator runs outside the factory: LLaMA-Factory LoRA and QLoRA, plus Axolotl bf16 LoRA and 4-bit QLoRA.

Locked defaults: [`../charter.md`](../charter.md). Product page: [`NORTH-STAR.md`](NORTH-STAR.md). Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). Prepare command surface: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Live paste target: [`LIVE-PROBES.md`](LIVE-PROBES.md).

Fixture accept loop: [`FEED-LOOP.md`](FEED-LOOP.md). Seated drivers: [`operator-local.md`](operator-local.md).

Commands on this page: `estate enrich from-pack`, `estate enrich prepare`, `estate enrich list`, `estate enrich import-prepared`, `estate enrich import-trained`, and `estate enrich apply-proposal`. No new crate. The factory does not run Unsloth or Axolotl. `READY_FOR_LIVE_TEST` stays no. Recorded specialist rows stay on the live-probes page. The opt-in `ollama create` handoff is [`LIVE-PROBES.md`](LIVE-PROBES.md).

## What stays fixed

| Piece | Stays |
| --- | --- |
| Binding id | `local_slm`, class `local` |
| Equal-class pair | `xai_grok` (frontier) and `local_slm` (local) |
| Curator | Jason. `enrich_packs.curator: jason`. `policy: manual` |
| Seat | A local runtime. Ollama is today's entrant. Catalog, route, and bind take the next process |
| Hardware | `host_class`: `consumer-nvidia`, `apple-silicon`, `rented-nvidia`, or `any` |
| Example estate | `examples/estate.yaml` stays `driver: ollama` and hash-locked, with `packs: []` |

Agents that already allow `local_slm` keep that id. Horizon and Research list it. Sanctum lists no models. Sanctum is the lane `sanctum`. The exclusion id is `cyera-ci`.

## 1. Prepare a purpose-built SLM on Ollama

Accept writes curator edit instructions. Jason pastes the pack id into the estate by hand. Ollama, the entrant already on the box, builds the model image with its own Modelfile and `ollama create`.

1. Accept the proposal. A wrong curator is `refuse:curator`.

```bash
estate packs accept --id <pack-id> --curator jason
```

That writes `packs/accepted/<pack-id>.enrich-edit.md` and the matching `.json`. `auto_apply` is false. `applied_to_estate` is false. `source_drivers` (`frontier` and/or `local`) is copied onto the instructions. It is not an estate field. The estate file is unchanged. Promote stays refused: `estate feed promote` and `estate packs promote` fail on purpose.

2. Paste the snippet from the enrich-edit file under `enrich_packs.packs` on the estate file you apply. `examples/estate.yaml` on `main` stays hash-locked. Use a lab copy for the paste. Read the blast radius, then apply that file.

```bash
estate plan --estate <your-estate.yaml>
estate apply --estate <your-estate.yaml> --require-plan
```

The pasted entry is an id and a description. `source_drivers` stays a comment on the instruction file.

3. On the host, the Modelfile is `FROM` plus `SYSTEM`. `estate enrich from-pack` (or `estate enrich prepare`) writes that file and the `ollama create` line. It does not shell out. You can write the same file by hand. Command page: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). `FROM` is a model Ollama already has (`params.model` on `local_slm`, or a pack `model_hint` that is already a model tag such as `llama3`), or weights a trainer returned (journey 3). The binding id `local_slm` is not that name. A missing seated name is `refuse:base-model`. `SYSTEM` is the curator text for that pack's job (`policy-precheck`, `redact`, or `complete`). Keep `cyera`, `rust-classroom`, and `rust_classroom` out of the file. A hardware SKU in that name refuses.

```text
FROM llama3
SYSTEM You are the Cell One specialist for this pack. Answer the job and stop.
```

```bash
ollama create cell-one-specialist -f Modelfile
```

### Factory loop

The same join, with the factory writing the files, the proposal, and the plan input. Prepare does not run `ollama create`. List reads `.cell/enrich` and refuses when that directory is missing. Import writes a proposal for the existing `local_slm` seat. `apply-proposal` stages that binding. You still run plan and apply.

```bash
estate enrich from-pack \
  --estate <your-estate.yaml> \
  --pack <pack-id> \
  --state-dir .cell

estate enrich list --state-dir .cell

ollama create cell-enrich-<pack-id> -f .cell/enrich/<pack-id>/ollama-modelfile/Modelfile

estate enrich import-prepared \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/ollama-modelfile \
  --tag cell-enrich-<pack-id> \
  --path .cell/enrich/<pack-id>/ollama-modelfile/Modelfile

estate enrich apply-proposal \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/ollama-modelfile \
  --tag cell-enrich-<pack-id> \
  --state-dir .cell

estate plan --estate .cell/enrich-stage/staged-estate.yaml --state-dir .cell
estate apply --estate .cell/enrich-stage/staged-estate.yaml --state-dir .cell --require-plan
```

`NEXT.md` in the prepared directory has those paths filled in. The tag is `cell-enrich-{pack_id}`. A different tag is `refuse:tag`. A hardware SKU or a sacred token in the tag or the file is a refuse before `binding-proposal.json` exists. The proposal's `auto_apply` is false. `apply-proposal` does not apply. It writes `.cell/enrich-stage/staged-estate.yaml`. The source estate changes when `estate apply --require-plan` succeeds. Apply without `--require-plan` leaves that file unchanged. `examples/estate.yaml` on `main` stays hash-locked. Point `--estate` at a lab copy. `--verify-local-tag` checks the seated runtime for the tag and is off unless you set it.

4. Point the seat at that process and complete once. Same verb as the recorded live rows.

```bash
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
export CELL_LOCAL_MODEL=cell-one-specialist
estate specialist --driver ollama --prompt "Reply with the single word pong."
```

Expect `"job": "complete"` and a non-empty `"completion"`. An empty completion refuses. If the base is missing, `ollama pull` the `FROM` model first. Copy-paste for the recorded base `llama3`, and for the Mac and Linux boxes, is on [`LIVE-PROBES.md`](LIVE-PROBES.md). `cell-one-specialist` is an operator name. It is not a recorded live proof until Jason pastes a run.

The live local model is `CELL_LOCAL_MODEL`. It does not become the binding id. Unset `CELL_LOCAL_MODEL` uses the first id from Ollama `GET /api/tags`, then `/v1/models`.

## 2. Swap the entrant. Keep `local_slm`

The binding id is the estate contract. The process in the local-runtime seat changes. Agents that list `local_slm` stay as they are.

On the estate file you apply (leave the hash-locked example on `main` as `driver: ollama`):

```yaml
- id: local_slm
  class: local
  driver: llama.cpp
  params:
    host_class: any
    endpoint_env: CELL_LOCAL_ENDPOINT
    job: policy-precheck
  wired: true
```

`llama.cpp` is the swap-proof entrant on the same specialist protocol. Catalog route reads `driver`. Bind attaches that card. `host_class` stays one of the four names. `5090`, `4090`, and `m3-max` stay out of the id.

Point `CELL_LOCAL_ENDPOINT` at the llama.cpp server. The adapter posts to `/v1/chat/completions` or `/api/chat` after a models list answers. The factory stand-in is `POST {CELL_LOCAL_ENDPOINT}/v0/specialist`. Then:

```bash
estate specialist --driver llama.cpp --prompt "Reply with the single word pong."
```

A later process uses the same shape: a `driver` string that catalog, route, and bind already take, still on id `local_slm`. Ollama-on-Mac keeps driver `ollama` on `apple-silicon`. Native MLX stays a stub on that host class. vLLM and TRT stay experimental catalog cards. `specialist()` on a stub or experimental card fails closed. `estate probes --live` does not print `live ok` for `mlx`, `vllm`, or `trt`.

Supported means the seated driver is green on the box. Today that is `ollama` and llama.cpp. `http-remote` is the `CELL_LOCAL_ENDPOINT` pattern on `host_class: any`.

Plan the driver edit before apply. The id string `local_slm` does not change, so the allow-list on each agent stays valid.

## 3. Hand an external manifest to a trainer

GPU training stays off this factory. `estate enrich prepare --driver external-manifest` writes a portable JSON/YAML hatch. `--all-drivers` writes that hatch next to the Modelfile. See [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). `NEXT.md` names `manifest.json` and `manifest.yaml`. Jason hands those files to a trainer outside the factory. The factory does not parse an operator note, apply it, or store it as estate source of truth. When weights return, load them on the seated runtime as `cell-enrich-{pack_id}` and run `estate enrich import-prepared` with `--path` pointing at that file. `estate enrich apply-proposal` stages that proposal. Plan and `estate apply --require-plan` write the source estate.

Write it outside the estate file and outside `.cell/`. Name the file `external-manifest.md`. Copy the accept file for the pack id, curator, policy, `source_drivers`, and estate hash. `host_class` and `job` come from the estate binding you apply. `serve_with` and `serve_as` are notes for the trainer: which entrant will load the weights, and the slug from journey 1.

```text
pack_id: <pack-id>
curator: jason
policy: manual
binding_id: local_slm
class: local
host_class: consumer-nvidia
source_drivers: local
job: policy-precheck
estate_hash: <hash printed on the enrich-edit file>
serve_with: ollama
serve_as: cell-one-specialist
```

`host_class` is `consumer-nvidia`, `apple-silicon`, `rented-nvidia`, or `any`. `source_drivers` is `frontier` and/or `local`, copied from the accept file, and it has to match the pack's `path_counts`. `serve_with` names the entrant that will load the weights when they return: `ollama` today, `llama.cpp` after journey 2, or a later driver that has a catalog card. `serve_as` is the slug from journey 1. A SKU in `serve_as` or `pack_id` is refused when that name later hits a binding, a probe, or `CELL_LOCAL_MODEL`.

The trainer trains on their own hardware. Weights come back to the operator. Journey 1 loads them with `ollama create`, or journey 2 points `CELL_LOCAL_ENDPOINT` at the other runtime. The pack id still enters through the curator paste. The prepared tag enters when `apply-proposal` stages `local_slm` and `estate apply --require-plan` writes that file. Returned weights do not promote themselves.

No trainer crate ships with this page. Prepare does not download a dataset. `--from-feed` copies rows that are already under the cell state directory. It checks `kind` and `object_class` before it copies a ShareGPT or Alpaca line, reads the opened file, and refuses when the sources together exceed 8 MiB.

## 4. Train LoRA or QLoRA with LLaMA-Factory, then seat it

On a consumer or rented Nvidia box, LLaMA-Factory already runs LoRA and QLoRA supervised fine-tuning from a YAML recipe. This journey writes that recipe from a pack and brings the adapter back onto `local_slm`. The factory does not run `llamafactory-cli train`.

Select the card with `--driver`. `llamafactory-lora` is the 16-bit LoRA quickstart (`examples/train_lora/qwen3_lora_sft.yaml` shape: `finetuning_type: lora`, no `quantization_bit`, `lora_rank` 8, `packing: false`). It does not require bitsandbytes. `llamafactory-qlora` is 4-bit QLoRA (`quantization_bit: 4`, `quantization_method: bnb`, rank 16) and still requires bitsandbytes. Both cards infer `template` by scanning path segments of the train base, starting at the last segment. A leaf such as `weights` or an HF snapshot hash uses the nearest ancestor that names a family. `Qwen/Qwen3-4B-Instruct-2507` uses `qwen3_nothink`. Other Qwen3 names use `qwen3`. `export.yaml` omits quantization on both.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id> \
  --driver llamafactory-lora \
  --job train \
  --state-dir .cell

pip install llamafactory
llamafactory-cli train .cell/enrich/<pack-id>/llamafactory-lora/recipe.yaml
llamafactory-cli export .cell/enrich/<pack-id>/llamafactory-lora/export.yaml
```

`<your-estate.yaml>` is a lab copy. It needs `params.model` on `local_slm` (a model the seat already has, such as `llama3`) or a pack `model_hint` that is already a model tag. It also needs a train base: pack field `train_base_model`, or `params.train_base_model` on that same binding. The train base is a Hugging Face repo id (`namespace/name`) or a local directory of HF weights. A `./` or `../` directory is written into the recipe as an absolute path. A directory named like an Ollama seat tag (`./llama3`) is `refuse:train-base`. A 5090 smoke used `Qwen/Qwen2.5-0.5B-Instruct` while the seat tag stayed `llama3`. Phi-3 and Phi-3.5 Instruct are a reproduce target beside that Qwen LoRA/QLoRA pair: `examples/fixtures/phi3-instruct.pack.json` sets `train_base_model` to `microsoft/Phi-3-mini-4k-instruct`, and `llamafactory-qlora` writes template `phi` (`phi_small` for Phi-3-small) with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target. A nested path segment such as `./weights/Phi-3.5-mini-instruct` uses the same template. Prepare does not download weights. `examples/estate.yaml` on `main` stays hash-locked. A binding id as `FROM` is `refuse:base-model`. A missing train base, or a bare Ollama tag in that field, is `refuse:train-base`.

The train hosts for this card are `consumer-nvidia` and `rented-nvidia`. Prepare on `apple-silicon` still writes the files. `NEXT.md` says the card expects CUDA LLaMA-Factory. There is no MLX trainer in this journey.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id> \
  --driver llamafactory-qlora \
  --job train \
  --state-dir .cell
```

That writes `.cell/enrich/<pack-id>/llamafactory-qlora/recipe.yaml`, `export.yaml`, `dataset_info.json`, and `dataset.jsonl`. The JSONL is instruct chat (`messages` of `role` and `content`). The recipe is 4-bit QLoRA with LoRA rank 16, packing on, `quantization_method: bnb`, and a short `cutoff_len` of 512. `model_name_or_path` is the train base. `template` is inferred from path segments of that train base, starting at the last segment. Use that same chat template when you seat. `prepare.json` says `job` `train`, stores `seat_tag` and `train_base_model`, and keeps `promoted`, `auto_apply`, and `estate_rewritten` false. It also stores `dataset_mode`. With no `--from-feed`, a pack that lists `source_paths` gets `dataset_mode` `scaffold`: the JSONL names those paths and leaves the files unread. An empty list is `dataset_mode` `stub` (three example rows). `PREPARE.md` and `NEXT.md` say these rows are not training data, and they name `refuse:dataset` for a later `--from-feed` whose file is missing.

When those paths are already files under the cell state directory, hydrate and then train:

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id> \
  --driver llamafactory-qlora \
  --job train \
  --from-feed \
  --state-dir .cell
```

For source path `feed/events.jsonl`, the file is `.cell/feed/events.jsonl`. ShareGPT messages and Alpaca `instruction` / `output` lines are copied. A scrubbed feed event becomes a row when its `note` is present. Events with no note are skipped and counted. A missing file is `refuse:dataset` and writes nothing. This step does not download the source. `llamafactory-lora` takes the same `--from-feed` flag and writes those instruct rows into its chat `dataset.jsonl`. Its `PREPARE.md` and `NEXT.md` use the same dataset paragraph.

The default recipe is one epoch and does not set `max_steps`. A short gauge run is the same prepare with `--max-steps 10`. LLaMA-Factory then overrides `num_train_epochs`. When that count is under 50, `save_steps` matches it so a checkpoint is written during the short run.

On the CUDA host, run the lines from `NEXT.md`:

```bash
pip install llamafactory
pip install 'bitsandbytes>=0.49'
llamafactory-cli train .cell/enrich/<pack-id>/llamafactory-qlora/recipe.yaml
llamafactory-cli export .cell/enrich/<pack-id>/llamafactory-qlora/export.yaml
```

QLoRA needs bitsandbytes. `pip install llamafactory` and `llamafactory[torch,metrics]` 0.9.5 did not install it. On a consumer RTX host, keep the torch CUDA wheel you already installed. A 5090 smoke used torch 2.11.0+cu128 (CUDA 12.8) and bitsandbytes 0.50.2. That bitsandbytes install did not replace torch. If the torch wheel still does not match the CUDA install, use https://github.com/hiyouga/LLaMA-Factory#installation. This factory does not download weights and does not map the seat tag onto a Hub repo.

The train saves the adapter under `outputs/` (`adapter_config.json` inside it). Merge with `export.yaml`. Do not set `quantization_bit` on that merge. LLaMA-Factory does not write GGUF. Convert the merge with llama.cpp if you want a GGUF, then seat on Ollama with `FROM` that GGUF. To load the adapter without a merge, `FROM` must be an Ollama model of the same train base, plus `ADAPTER`. The seat tag is the id the cell already runs. After the tag is seated, send a short prompt that checks the pack purpose. This factory does not run that smoke eval.

On Nvidia only, Unsloth QLoRA is a faster single-GPU alternate. `NEXT.md` points at the Unsloth docs. This journey does not register an Unsloth card and does not write a script.

After you create tag `cell-enrich-<pack-id>` on Ollama, record the join. `import-trained` writes `binding-proposal.json` for the existing `local_slm` seat. It does not apply.

```bash
estate enrich import-trained \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --tag cell-enrich-<pack-id> \
  --adapter .cell/enrich/<pack-id>/llamafactory-qlora/outputs

estate enrich import-trained \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --tag cell-enrich-<pack-id> \
  --adapter .cell/enrich/<pack-id>/llamafactory-lora/outputs

estate enrich apply-proposal \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --tag cell-enrich-<pack-id> \
  --state-dir .cell

estate plan --estate .cell/enrich-stage/staged-estate.yaml --state-dir .cell
estate apply --estate .cell/enrich-stage/staged-estate.yaml --state-dir .cell --require-plan
```

`NEXT.md` on `llamafactory-lora` and `llamafactory-qlora` prints the same import for `outputs/` (adapter `adapter_config.json`), `export/` (`config.json` plus a `.safetensors` file whose name does not start with `adapter_model`, optional `Modelfile`), and one `.gguf` file or a directory with exactly one top-level `.gguf`. A directory with more than one is `refuse:adapter`. `prepare.json` records `trained_shape` and `trained_paths` on the same write as the proposal. A path that matches none of those shapes, or more than one, or a symlinked marker, is `refuse:adapter` before a new proposal exists. A sacred token or a hardware SKU still refuses. `apply --require-plan` is the only step that writes the source estate.

When you want a second YAML recipe or a multi-GPU run, prepare `axolotl-lora` or `axolotl-qlora` the same way, including `--from-feed` when the sources are already on disk. Each card writes `axolotl.yml` and an Alpaca `dataset.jsonl`. `axolotl-lora` is bf16 LoRA matching Axolotl `examples/llama-3/lora-1b.yml` (`adapter: lora`, `load_in_4bit: false`, `sequence_len` 2048, `micro_batch_size` 2, `gradient_accumulation_steps` 2, `lora_r` 16). `axolotl-qlora` is 4-bit QLoRA matching `examples/llama-3/qlora.yml` (`adapter: qlora`, `load_in_4bit: true`, `sequence_len` 4096, `micro_batch_size` 2, `gradient_accumulation_steps` 4, `lora_r` 32). `base_model` in that yaml is the train base. `prepare.json` keeps the Ollama seat tag for Modelfile `FROM` and for the adapter join (`FROM` a merged GGUF, or `FROM` an Ollama model of that train base plus `ADAPTER`), and it stores the same `dataset_mode` fields. `PREPARE.md` and `NEXT.md` use the same dataset paragraph as the LLaMA-Factory cards. A missing train base, a bare seat tag, or a local directory named like a seat tag (`./llama3`) is `refuse:train-base` and writes nothing. A missing feed file with `--from-feed` is `refuse:dataset` and writes nothing. `--all-drivers --job train` writes the train base into both Axolotl yamls and leaves Modelfile `FROM` as the seat tag. On the CUDA host, run `axolotl train` on that yaml. A short gauge run passes `--max-steps`. `import-trained` takes either directory. Unsloth stays the `NEXT.md` pointer on the LLaMA-Factory card. Opt-in check, with no LLaMA-Factory process and no Axolotl process: `make train-prepare`. It prints `SKIP live train`.

## 5. Fail-closed moments

| Moment | Stop |
| --- | --- |
| Sacred | `cyera-ci` and `rust-classroom` always apply. Aliases `cyera`, `cyera_ci`, and `rust_classroom` resolve to those ids and refuse. `policy/sacred.yaml` can add names (`lab-notebook` in fixtures). A file cannot drop a locked id. An allow intention cannot punch through. A specialist prompt that contains `cyera`, `rust-classroom`, or `rust_classroom` refuses before any POST, on the local seat and on frontier. |
| SKU | `5090`, `4090`, and `m3-max` in a binding id, probe id, pack id, hop id, or model id (`CELL_LOCAL_MODEL`, `CELL_FRONTIER_MODEL`) refuse before the write or the POST. Name the box with `host_class`. A 5090-class box is `consumer-nvidia` or `rented-nvidia`. |
| Local down | Estate-bound local work audits `model.local.down` and stops. The task path ends `local:down`. Frontier is not called. An unset `CELL_LOCAL_ENDPOINT` on `estate probes --live` or `make real-world` prints SKIP and exits 0. That SKIP is the opt-in ladder. The estate-bound deny is the `model.local.down` row. |
| No auto-promote | Feed materializes candidate packs. `estate packs propose` leaves `auto_apply: false`. Accept writes edit instructions and does not rewrite the estate. Promote stays refused. A second `estate feed pack` keeps the feed cursor and does not promote. A `frontier` tag in `source_drivers` on an estate with no frontier binding is `refuse:frontier-invent`. |
| Stub or experimental seat | Native MLX, vLLM, and TRT fail closed at `specialist()` until Jason verifies. Estate-bound local work does not fall through to `grok-4.7`. |

Text over 16KiB refuses before the POST. Credentials stay in the environment (`XAI_API_KEY`, `CELL_LOCAL_ENDPOINT`). The estate file does not carry them.

## 6. Integrate a driver, or let a new one earn its keep

Build rule (`integrate-vs-invent`): a feature earns its keep. If `ollama` or llama.cpp already does the job, tighten that driver.

Use Ollama for running a model on the host, including Ollama-on-Mac, and for building a purpose-built image from a base or from weights the trainer returned. The Modelfile and `ollama create` already do that job. Use llama.cpp when that same specialist protocol should run in the llama.cpp process: change `driver` on `local_slm` and point `CELL_LOCAL_ENDPOINT` at it. Use LLaMA-Factory when the job is LoRA or QLoRA from a durable recipe: `llamafactory-lora` writes the unquantized recipe and `llamafactory-qlora` writes the 4-bit recipe, and you run `llamafactory-cli train` outside the factory (journey 4). Use Axolotl when you want a second YAML recipe or a multi-GPU run: `axolotl-lora` writes the bf16 LoRA `axolotl.yml` and `axolotl-qlora` writes the 4-bit `axolotl.yml`, and you run `axolotl train` outside the factory. Unsloth QLoRA stays a `NEXT.md` pointer on Nvidia, not a registered card.

A new driver earns a catalog card when `ollama` and llama.cpp both lack the job. The card goes through catalog, route, and bind. The binding id stays `local_slm`. Floor core does not gain a vendor string. The driver stays a trait. The specialist process may be any language. Jason verifies before the card is Supported. Until that verification, the card stays stub or experimental and fails closed.

Anti-shrink keeps these out of the factory: an AI gateway, an Ollama wrapper-as-product, LM Studio-alone, a Grok Bot clone, a chat UI, a weight browser. LLaMA-Factory and Axolotl stay the trainers. This factory writes the recipe and does not run it.

Control does not complete. Completion stays on the data plane (`estate specialist`, `model-estate`).

## Flexibility that must survive

From [`../charter.md`](../charter.md):

- Swap isolation, frontier, or local drivers without rewriting floor core.
- Add or rename lanes in the estate file. Sacred names stay locked.
- Pause and resume from files. Runtime is regenerable.
- Keep frontier and local equal class in the schema.
- Compile intentions from the estate. Policy does not learn in silence.
- Keep `CELL_LOCAL_ENDPOINT` as config. The host is not compiled in.
- Swap Ollama, llama.cpp, or a later entrant through catalog, route, and bind. Hardware stays `host_class`.

## 7. Seat the merged export on Ollama

After journey 4's `llamafactory-cli export`, the merged directory is `export_dir` from `export.yaml`. Current LLaMA-Factory writes `Modelfile` there (`FROM .`, plus the chat TEMPLATE). GGUF conversion stays `convert_hf_to_gguf.py` on a llama.cpp checkout. This factory does not run either tool.

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export
```

The printed create name is `cell-enrich-<pack-id>`. The seat tag in the report is `prepare.json` `seat_tag`. When the export directory contains the LLaMA-Factory Modelfile, the command prints `ollama create cell-enrich-<pack-id> -f <export>/Modelfile`. When `--weights` is a `.gguf` file, it prints a Modelfile whose FROM is that file. It does not create the model.

Then `import-trained --adapter` points at that same directory or GGUF. A merged export_dir is `config.json` plus `.safetensors`, with an optional Modelfile. The seat tag on the proposal stays the prepare seat tag. `apply-proposal` and `estate apply --require-plan` stay the join. `READY_FOR_LIVE_TEST`: no. Page: [`local-seat.md`](local-seat.md).
