# Operator journeys: train/enrich

Walks for the suite goal: facilitate train and enrich of purpose-built small-parameter models. Today's beachhead is enrich packs, the specialist path, and prepare cards the operator runs outside the factory: LLaMA-Factory LoRA and QLoRA, plus Axolotl bf16 LoRA and 4-bit QLoRA.

Locked defaults: [`../charter.md`](../charter.md). Product page: [`NORTH-STAR.md`](NORTH-STAR.md). Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). Prepare command surface: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Live paste target: [`LIVE-PROBES.md`](LIVE-PROBES.md).

Fixture accept loop: [`FEED-LOOP.md`](FEED-LOOP.md). Seated drivers: [`operator-local.md`](operator-local.md).

Commands on this page: `estate enrich from-pack`, `estate enrich prepare`, `estate enrich list`, `estate enrich import-prepared`, `estate enrich import-trained`, `estate enrich merge-adapt`, `estate enrich gguf-convert`, `estate enrich local-seat`, and `estate enrich apply-proposal`. No new crate. The factory does not run Unsloth, Axolotl, or llama.cpp. `READY_FOR_LIVE_TEST` stays no. Recorded specialist rows stay on the live-probes page. The opt-in `ollama create` handoff is [`LIVE-PROBES.md`](LIVE-PROBES.md).

Popular path (Target C): Qwen through LLaMA-Factory QLoRA, then a printed GGUF convert, a printed Ollama create, and `import-trained` to record the shape. Section 8. The seat tag and the train base stay separate. Opt-in check: `make qlora-journey`. It prints that ladder and checks the prepare artifacts. It does not train, does not convert, and does not promote.

Unquantized path (Target A): Qwen through LLaMA-Factory LoRA, then a printed merge, a printed GGUF convert, a printed Ollama create, and `import-trained` to record the shape. Section 9. The seat tag and the train base stay separate. Opt-in check: `make lora-journey`. It prints that ladder and checks the prepare artifacts. It does not train, does not merge, does not convert, and does not promote.

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

On a consumer or rented Nvidia box, LLaMA-Factory already runs LoRA and QLoRA supervised fine-tuning from a YAML recipe. This journey writes that recipe from a pack and brings the adapter back onto `local_slm`. The factory does not run `llamafactory-cli train`. The popular Qwen QLoRA order, including the printed convert and the printed seat, is section 8. The Qwen LoRA order, including the printed merge, the printed convert, and the printed seat, is section 9.

Select the card with `--driver`. `llamafactory-lora` is the 16-bit LoRA quickstart (`examples/train_lora/qwen3_lora_sft.yaml` shape: `finetuning_type: lora`, no `quantization_bit`, `lora_rank` 8, `packing: false`). It does not require bitsandbytes. `llamafactory-qlora` is 4-bit QLoRA (`quantization_bit: 4`, `quantization_method: bnb`, rank 16) and still requires bitsandbytes. Both cards infer `template` by scanning path segments of the train base, starting at the last segment. A leaf such as `weights` or an HF snapshot hash uses the nearest ancestor that names a family. `Qwen/Qwen3-4B-Instruct-2507` uses `qwen3_nothink`. Other Qwen3 names use `qwen3`. `export.yaml` omits quantization on both. Prepare does not merge. `export.yaml` is the merge card, `adapter_name_or_path` matches recipe `output_dir`, and `NEXT.md` says the merge has not happened. `import-trained` refuses a real `quantization_bit` or `quantization_method` key on that file (`refuse:export`).

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

`estate enrich merge-adapt --prepared .cell/enrich/<pack-id>/llamafactory-lora --adapter .cell/enrich/<pack-id>/llamafactory-lora/outputs` prints that same `llamafactory-cli export` line and the keys from `examples/merge_lora/qwen3_lora_sft.yaml`. It does not run the export.

`<your-estate.yaml>` is a lab copy. It needs `params.model` on `local_slm` (a model the seat already has, such as `llama3`) or a pack `model_hint` that is already a model tag. It also needs a train base: pack field `train_base_model`, or `params.train_base_model` on that same binding. The train base is a Hugging Face repo id (`namespace/name`) or a local directory of HF weights. A `./` or `../` directory is written into the recipe as an absolute path. A directory named like an Ollama seat tag (`./llama3`) is `refuse:train-base`. A 5090 smoke used `Qwen/Qwen2.5-0.5B-Instruct` while the seat tag stayed `llama3`. Phi-3 and Phi-3.5 Instruct are a reproduce target beside that Qwen LoRA/QLoRA pair: `examples/fixtures/phi3-instruct.pack.json` sets `train_base_model` to `microsoft/Phi-3-mini-4k-instruct`, and `llamafactory-qlora` writes template `phi` (`phi_small` for Phi-3-small) with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target. A nested path segment such as `./weights/Phi-3.5-mini-instruct` uses the same template. `examples/fixtures/phi3-instruct-lora.pack.json` is the same train base on `llamafactory-lora`. That card writes template `phi`, `lora_rank` 8, `packing: false`, and no quantization keys. Its reproduce line is LoRA-only and only for Phi-3 mini, Phi-3 medium, and Phi-3.5. Phi-3-small keeps template `phi_small` and does not get that line. Phi-4 and Phi-4-mini do not get that line. A lowercase directory leaf `phi-3-mini-4k-instruct` stays `refuse:train-base`. Llama-3.2 Instruct is a reproduce target beside Phi-3 and that Qwen pair: `examples/fixtures/llama32-instruct.pack.json` sets `train_base_model` to `meta-llama/Llama-3.2-3B-Instruct`, and `llamafactory-qlora` writes template `llama3` (`mllama` for Llama-3.2 vision) with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target. A nested path segment such as `./weights/Llama-3.2-3B-Instruct` uses the same template. `examples/fixtures/llama32-instruct-lora.pack.json` is the same train base on `llamafactory-lora`. That card writes template `llama3`, `lora_rank` 8, `packing: false`, and no quantization keys. Its reproduce line is LoRA-only. Llama-3.2 vision, a Llama-3.2 base, and Llama-3.1 Instruct do not get that line. A lowercase directory leaf `llama-3.2-3b-instruct` stays `refuse:train-base`. Gemma-2 Instruct is a reproduce target beside Phi-3, Llama-3.2, and that Qwen pair: `examples/fixtures/gemma2-instruct.pack.json` sets `train_base_model` to `google/gemma-2-2b-it`, and `llamafactory-qlora` writes template `gemma2` (`gemma` for original Gemma 2B and 7B) with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target only for a Gemma-2 Instruct id. `examples/fixtures/gemma2-instruct-lora.pack.json` is the same train base on `llamafactory-lora`. That card writes template `gemma2`, `lora_rank` 8, and no quantization keys. Its reproduce line is LoRA-only. A Gemma-2 base keeps template `gemma2` and does not get either line. A nested path whose leaf is `weights` or a snapshot hash, such as `./weights/google/gemma-2-9b-it/weights`, uses the same template. A lowercase directory leaf `gemma-2-2b-it` stays `refuse:train-base`. Mistral Instruct is a reproduce target beside Phi-3, Llama-3.2, Gemma-2, and that Qwen pair: `examples/fixtures/mistral-instruct.pack.json` sets `train_base_model` to `mistralai/Mistral-7B-Instruct-v0.3`, and `llamafactory-qlora` writes template `mistral` (`mistral_small` for Mistral-Small, `ministral` for Mistral-Nemo) with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target only for a Mistral-7B Instruct id. `examples/fixtures/mistral-instruct-lora.pack.json` is the same train base on `llamafactory-lora`. That card writes template `mistral`, `lora_rank` 8, `packing: false`, and no quantization keys. Its reproduce line is LoRA-only. A Mistral-7B base keeps template `mistral` and does not get either line. Mistral-Small, Mistral-Nemo, Mixtral, and LLaVA-NeXT-Mistral do not get either line. A nested path whose leaf is `weights` or a snapshot hash, such as `./weights/mistralai/Mistral-7B-Instruct-v0.3/weights`, uses the same template. A lowercase directory leaf `mistral-7b-instruct-v0.3` stays `refuse:train-base`. Qwen3 Instruct is a reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x: `examples/fixtures/qwen3-instruct.pack.json` sets `train_base_model` to `Qwen/Qwen3-4B-Instruct-2507`, and `llamafactory-qlora` writes template `qwen3_nothink` (`qwen3` for a Qwen3 thinking or base id, `qwen` for Qwen2.5) with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target only for that Qwen3 Instruct shape. `examples/fixtures/qwen3-instruct-lora.pack.json` is the same train base on `llamafactory-lora`. That card matches `examples/train_lora/qwen3_lora_sft.yaml`: template `qwen3_nothink`, `lora_rank` 8, and no quantization keys. Its reproduce line is LoRA-only. `Qwen/Qwen3-4B` and `Qwen/Qwen2.5-0.5B-Instruct` do not get either line. A nested path whose leaf is `weights` or a snapshot hash, such as `./weights/Qwen/Qwen3-4B-Instruct-2507/weights` or `models--Qwen--Qwen3-4B-Instruct-2507`, uses the same template. A lowercase directory leaf `qwen3-4b-instruct-2507` stays `refuse:train-base`. Prepare does not download weights. `examples/estate.yaml` on `main` stays hash-locked. A binding id as `FROM` is `refuse:base-model`. A missing train base, or a bare Ollama tag in that field, is `refuse:train-base`.

The train hosts for this card are `consumer-nvidia` and `rented-nvidia`. Prepare on `apple-silicon` still writes the files. `NEXT.md` says the card expects CUDA LLaMA-Factory. This journey does not write an MLX trainer. The optional handoff for `apple-silicon` is `mlx-lm-lora`.

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

The default recipe is one epoch and does not set `max_steps`. A short gauge run is the same prepare with `--max-steps 10`. LLaMA-Factory then overrides `num_train_epochs`. When that count is under 50, `save_steps` matches it so a checkpoint is written during the short run. `--official-scale` writes `cutoff_len` 2048, `num_train_epochs` 3.0, `gradient_accumulation_steps` 8, and `warmup_ratio` 0.1, matching LLaMA-Factory `examples/train_lora/qwen3_lora_sft.yaml`. Rank, packing, and quantization stay on the selected card. Combine it with `--max-steps` when you want that scale and a short step cap. Omit the flag for the short recipe.

On the CUDA host, run the lines from `NEXT.md`:

```bash
pip install llamafactory
pip install 'bitsandbytes>=0.49'
llamafactory-cli train .cell/enrich/<pack-id>/llamafactory-qlora/recipe.yaml
llamafactory-cli export .cell/enrich/<pack-id>/llamafactory-qlora/export.yaml
```

`estate enrich merge-adapt --prepared .cell/enrich/<pack-id>/llamafactory-qlora --adapter .cell/enrich/<pack-id>/llamafactory-qlora/outputs` prints that same `llamafactory-cli export` line and the keys from `examples/merge_lora/qwen3_lora_sft.yaml`. It does not run the export.

QLoRA needs bitsandbytes. `pip install llamafactory` and `llamafactory[torch,metrics]` 0.9.5 did not install it. On a consumer RTX host, keep the torch CUDA wheel you already installed. A 5090 smoke used torch 2.11.0+cu128 (CUDA 12.8) and bitsandbytes 0.50.2. That bitsandbytes install did not replace torch. If the torch wheel still does not match the CUDA install, use https://github.com/hiyouga/LLaMA-Factory#installation. This factory does not download weights and does not map the seat tag onto a Hub repo.

The train saves the adapter under `outputs/` (`adapter_config.json` inside it). Merge with `export.yaml`. Do not set `quantization_bit` on that merge. LLaMA-Factory does not write GGUF. `estate enrich gguf-convert` prints the llama.cpp `convert_hf_to_gguf.py` line for that merged directory (`--outtype auto`, outfile beside the directory). Then seat on Ollama with `FROM` that GGUF. To load the adapter without a merge, `estate enrich local-seat --adapter` prints a Modelfile. `FROM` is the seat tag (an Ollama model of the same train base). `ADAPTER` is the adapter directory. The command does not run that line. `--weights` stays the merged or GGUF path. After the tag is seated, send a short prompt that checks the pack purpose. This factory does not run that smoke eval.

On Nvidia only, Unsloth QLoRA is a faster single-GPU alternate. The optional card is `unsloth-qlora`. It writes `UNSLOTH.md`, an operator-owned handoff, and does not write a script. `NEXT.md` points at the Unsloth install page and the fine-tuning guide. After that train, `estate enrich merge-adapt` prints `save_pretrained_merged` with `save_method` `merged_16bit`. `gguf-convert` and `local-seat` print the next lines. `local-seat --adapter` is `refuse:adapter`. This factory does not call Unsloth.

On Apple Silicon, mlx-lm already documents LoRA and fuse. The optional card is `mlx-lm-lora`. It writes `MLX.md` when `host_class_affinity` is `apple-silicon`. Another affinity is `refuse:host` and writes nothing. `NEXT.md` points at https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md and the fuse command that page publishes (`mlx_lm.fuse --model <path_to_model>`). After `mlx_lm.lora`, `estate enrich merge-adapt` prints `mlx_lm.fuse` with `--adapter-path` and `--save-path`, plus `--export-gguf`. That writes `fused_model/ggml-model-f16.gguf`. `local-seat` prints the Ollama line for that file. `import-trained` records the adapter directory or that GGUF file. A fused MLX directory is `refuse:adapter`. This factory does not call mlx-lm. The `mlx` runtime card stays a stub.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id-or-pack.json> \
  --driver unsloth-qlora \
  --job train \
  --state-dir .cell
```

A missing train base is `refuse:train-base`. `--official-scale` on this card alone is `refuse:official-scale`. `--from-feed` on this card alone is `refuse:dataset`. After you train outside the factory, `estate enrich import-trained --adapter` takes the artifact you saved.

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

When you want a second YAML recipe or a multi-GPU run, prepare `axolotl-lora` or `axolotl-qlora` the same way, including `--from-feed` when the sources are already on disk. Each card writes `axolotl.yml` and an Alpaca `dataset.jsonl`. `axolotl-lora` is bf16 LoRA matching Axolotl `examples/llama-3/lora-1b.yml` (`adapter: lora`, `load_in_4bit: false`, `sequence_len` 2048, `micro_batch_size` 2, `gradient_accumulation_steps` 2, `lora_r` 16). `axolotl-qlora` is 4-bit QLoRA matching `examples/llama-3/qlora.yml` (`adapter: qlora`, `load_in_4bit: true`, `sequence_len` 4096, `micro_batch_size` 2, `gradient_accumulation_steps` 4, `lora_r` 32). `base_model` in that yaml is the train base. `prepare.json` keeps the Ollama seat tag for Modelfile `FROM` and for the adapter join (`FROM` a merged GGUF, or `FROM` an Ollama model of that train base plus `ADAPTER`), and it stores the same `dataset_mode` fields. `PREPARE.md` and `NEXT.md` use the same dataset paragraph as the LLaMA-Factory cards. A missing train base, a bare seat tag, or a local directory named like a seat tag (`./llama3`) is `refuse:train-base` and writes nothing. A missing feed file with `--from-feed` is `refuse:dataset` and writes nothing. `--all-drivers --job train` writes the train base into both Axolotl yamls and leaves Modelfile `FROM` as the seat tag. On the CUDA host, run `axolotl train` on that yaml. A short gauge run passes `--max-steps`. `estate enrich merge-adapt` then prints `axolotl merge-lora` for the adapter directory. Axolotl writes `output_dir/merged`. `axolotl-qlora` also prints `--dequant`. This factory does not run the merge. Axolotl does not write GGUF. `NEXT.md` names `merge-adapt`, `gguf-convert`, `local-seat`, and `import-trained` for that directory. `import-trained` also takes the adapter directory. Unsloth stays the `NEXT.md` pointer on the LLaMA-Factory card. Opt-in check, with no LLaMA-Factory process and no Axolotl process: `make train-prepare`. It prints `SKIP live train`.

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

Use Ollama for running a model on the host, including Ollama-on-Mac, and for building a purpose-built image from a base or from weights the trainer returned. The Modelfile and `ollama create` already do that job. Use llama.cpp when that same specialist protocol should run in the llama.cpp process: change `driver` on `local_slm` and point `CELL_LOCAL_ENDPOINT` at it. Use LLaMA-Factory when the job is LoRA or QLoRA from a durable recipe: `llamafactory-lora` writes the unquantized recipe and `llamafactory-qlora` writes the 4-bit recipe, and you run `llamafactory-cli train` outside the factory (journey 4). Use Axolotl when you want a second YAML recipe or a multi-GPU run: `axolotl-lora` writes the bf16 LoRA `axolotl.yml` and `axolotl-qlora` writes the 4-bit `axolotl.yml`, and you run `axolotl train` outside the factory. Use `unsloth-qlora` when you want the optional Nvidia-only QLoRA handoff. That card writes `UNSLOTH.md` and does not write a script. You follow Unsloth's docs outside the factory. After that train, `merge-adapt` prints the documented `merged_16bit` save. Use `mlx-lm-lora` when `host_class_affinity` is `apple-silicon` and you want the optional mlx-lm LoRA handoff. That card writes `MLX.md` and does not write a script. You follow the mlx-lm LoRA page outside the factory. Another host class is `refuse:host`.

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

After journey 4's `llamafactory-cli export`, the merged directory is `export_dir` from `export.yaml`. Current LLaMA-Factory writes `Modelfile` there (`FROM .`, plus the chat TEMPLATE). Print the convert line, then seat. This factory does not run llama.cpp or Ollama. Section 8 places this seat on the Qwen QLoRA ladder, after prepare and the `NEXT.md` train and export lines. Section 9 places this seat on the Qwen LoRA ladder the same way.

```bash
estate enrich gguf-convert \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export
```

That prints `python3 convert_hf_to_gguf.py` on the merged directory, with `--outfile` set to a sibling `<export>.gguf` and `--outtype auto` (the script default: highest-fidelity 16-bit float). Run that line from a llama.cpp checkout. The outfile stays outside the merged directory so the directory remains one shape. Then:

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export.gguf
```

The printed create name is `cell-enrich-<pack-id>`. The seat tag in the report is `prepare.json` `seat_tag`. `--weights` on that local-seat line is the sibling `.gguf`. local-seat prints a Modelfile whose FROM is that file. To seat the merged directory itself, point `--weights` at `export` when that directory contains the LLaMA-Factory Modelfile (`FROM .`). The command prints `ollama create cell-enrich-<pack-id> -f <export>/Modelfile`. It does not create the model.

Then `import-trained --adapter` points at that same directory or GGUF. A merged export_dir is `config.json` plus a `.safetensors` file whose name does not start with `adapter_model`, with an optional Modelfile. `import-trained` records `trained_shape` and `trained_paths`. The seat tag on the proposal stays the prepare seat tag. `apply-proposal` and `estate apply --require-plan` stay the join. `READY_FOR_LIVE_TEST`: no. Page: [`local-seat.md`](local-seat.md).

## 8. Target C — Qwen / LLaMA-Factory QLoRA to the local seat

This is the popular path. LLaMA-Factory already runs 4-bit QLoRA. llama.cpp already converts a merged Hugging Face directory to GGUF. Ollama already creates a model from a Modelfile. This factory writes the recipe and prints the next command. It does not train, does not convert, does not create the model, and does not promote.

The example that a 5090 smoke used keeps two names. The seat tag is `llama3` (a model Ollama already has: `params.model` on `local_slm`, or a pack `model_hint` that is already a model tag). The train base is `Qwen/Qwen2.5-0.5B-Instruct` (pack `train_base_model`, or `params.train_base_model` on that same binding). `recipe.yaml` and `export.yaml` set `model_name_or_path` to the train base. `template` for that Qwen2.5 name is `qwen`. The seat tag stays `prepare.json` `base_model` and `seat_tag`. A missing train base, a bare seat tag, or a local directory named like a seat tag is `refuse:train-base` and writes nothing. This factory does not map `llama3` onto a Hub repo and does not download weights. `<your-estate.yaml>` is a lab copy. `examples/estate.yaml` on `main` stays hash-locked.

`dataset.jsonl` with no `--from-feed` is a scaffold (or a three-row stub when `source_paths` is empty). Those rows are not training data. `--from-feed` copies instruct rows that are already under the cell state directory. A missing file is `refuse:dataset` and writes nothing.

### 1. Prepare

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id> \
  --driver llamafactory-qlora \
  --job train \
  --state-dir .cell
```

Omitting `--job` is the same train job. `--job enrich` is `refuse:job` and writes nothing. The output directory is `.cell/enrich/<pack-id>/llamafactory-qlora/`.

| File | What the Qwen card holds |
| --- | --- |
| `recipe.yaml` | SFT QLoRA. `quantization_bit: 4`, `quantization_method: bnb`, LoRA rank 16, `packing: true`, `cutoff_len` 512, `template: qwen` for `Qwen/Qwen2.5-0.5B-Instruct`. `model_name_or_path` is the train base. |
| `export.yaml` | Merge card. Same train base. No `quantization_bit`. This prepare did not merge. |
| `dataset.jsonl` | Instruct chat. `dataset_mode` is `scaffold` until `--from-feed`. |
| `prepare.json` | `job` `train`. `seat_tag` `llama3`. `train_base_model` `Qwen/Qwen2.5-0.5B-Instruct`. `promoted`, `auto_apply`, and `estate_rewritten` are false. `trained_shape` is absent until step 5. |
| `NEXT.md` | The train line, the export line, the convert line, the seat line, and the import lines. |

A short gauge run adds `--max-steps 10`. `--official-scale` writes the longer SFT scale from LLaMA-Factory `examples/train_lora/qwen3_lora_sft.yaml` and leaves rank, packing, and quantization on this card. The default recipe stays one epoch. Command page: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md).

### 2. Train and export, outside this factory

Run the lines `NEXT.md` prints. On a CUDA host (`consumer-nvidia` or `rented-nvidia`):

```bash
pip install llamafactory
pip install 'bitsandbytes>=0.49'
llamafactory-cli train .cell/enrich/<pack-id>/llamafactory-qlora/recipe.yaml
llamafactory-cli export .cell/enrich/<pack-id>/llamafactory-qlora/export.yaml
```

`estate enrich merge-adapt --prepared .cell/enrich/<pack-id>/llamafactory-qlora --adapter .cell/enrich/<pack-id>/llamafactory-qlora/outputs` prints that same `llamafactory-cli export` line and the keys from `examples/merge_lora/qwen3_lora_sft.yaml`. It does not run the export. `gguf-convert` and `local-seat` then print the next lines for `export/`.

QLoRA needs bitsandbytes. `pip install llamafactory` did not install it. A 5090 smoke used torch 2.11.0+cu128 and bitsandbytes 0.50.2. This factory does not install either package and does not run `llamafactory-cli`. Prepare on `apple-silicon` still writes the files. `NEXT.md` says the card expects CUDA LLaMA-Factory.

The train writes the adapter under `outputs/` (`adapter_config.json`). The merge writes `export/` (`config.json` and a `.safetensors` file whose name does not start with `adapter_model`). Do not set `quantization_bit` on that merge. `import-trained` refuses a real `quantization_bit` or `quantization_method` key on `export.yaml` (`refuse:export`). LLaMA-Factory does not write GGUF.

### 3. Print the GGUF convert

After `llamafactory-cli export` exits 0, the merged directory is `export/`. Print the llama.cpp line. This command does not convert and does not write a GGUF. A missing directory, an adapter directory, or a path that is already a GGUF is `refuse:seat`.

```bash
estate enrich gguf-convert \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export
```

That prints:

```bash
python3 convert_hf_to_gguf.py .cell/enrich/<pack-id>/llamafactory-qlora/export --outfile .cell/enrich/<pack-id>/llamafactory-qlora/export.gguf --outtype auto
```

Run that line from a llama.cpp checkout. `--outtype auto` is the script default (highest-fidelity 16-bit float). The outfile is a sibling of the merged directory. This factory does not choose a quantization type.

### 4. Print the Ollama create

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export.gguf
```

The create name is `cell-enrich-<pack-id>`. The seat tag in the report is `prepare.json` `seat_tag` (`llama3` in the example). The command prints a Modelfile whose `FROM` is that GGUF, then `ollama create cell-enrich-<pack-id> -f` that file. The same report prints `llama-cli -m` and `llama-server -m` for that GGUF. `--runtime llama.cpp` selects those lines. Ollama stays the default print. It does not create the model and does not run llama.cpp. To seat the merged directory itself, point `--weights` at `export` when that directory contains the LLaMA-Factory Modelfile (`FROM .`). That directory still points at `gguf-convert` before any llama.cpp load line. A missing GGUF is `refuse:seat`.

To seat `outputs/` without a merge, pass `--adapter` instead of `--weights`. The printed Modelfile uses `FROM` `prepare.json` `seat_tag` and `ADAPTER` that directory. The command prints `ollama create` and does not run it. `--weights` still refuses the adapter directory (`refuse:seat`). A merged export or a GGUF passed to `--adapter` is `refuse:adapter`.

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --adapter .cell/enrich/<pack-id>/llamafactory-qlora/outputs
```

Page: [`local-seat.md`](local-seat.md).

### 5. Record the shape

`import-trained` writes `binding-proposal.json` for the existing `local_slm` seat and records `trained_shape` and `trained_paths` on that proposal and on `prepare.json`. It does not apply and does not promote. After the GGUF exists, point `--adapter` at that file. `trained_shape` is `gguf`.

```bash
estate enrich import-trained \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --tag cell-enrich-<pack-id> \
  --adapter .cell/enrich/<pack-id>/llamafactory-qlora/export.gguf
```

The same command records the other two shapes when that is the artifact you have. An adapter directory (`outputs/`, `adapter_config.json`) is `trained_shape` `adapter`. A merged directory (`export/`, `config.json` plus a non-adapter `.safetensors` file) is `trained_shape` `merged`. A path that matches none of those shapes, or more than one, is `refuse:adapter` before a new proposal exists. `NEXT.md` prints all three lines with the prepared directory filled in.

`apply-proposal`, then `estate plan` and `estate apply --require-plan`, stay the join. Promote stays refused. `READY_FOR_LIVE_TEST`: no.

```bash
make qlora-journey
```

That opt-in script prints this ladder, prepares `llamafactory-qlora` on a throwaway copy of `examples/estate.yaml`, and checks the artifacts: seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`, the `NEXT.md` train and export lines, the printed convert line, the printed seat line, and the import lines. A seat tag with no train base is `refuse:train-base` and writes nothing. `gguf-convert` and `local-seat` against a missing export are `refuse:seat` and write no GGUF. The script prints `SKIP live train`. It leaves `examples/estate.yaml` unchanged. It is not in `make smoke`, `make gate-90`, or GitHub Actions.

## 9. Target A — Qwen / LLaMA-Factory LoRA to the local seat

This is the unquantized path. LLaMA-Factory already runs LoRA supervised fine-tuning. llama.cpp already converts a merged Hugging Face directory to GGUF. Ollama already creates a model from a Modelfile. This factory writes the recipe and prints the next command. It does not train, does not merge, does not convert, does not create the model, and does not promote.

The example that a 5090 smoke passed keeps two names. The seat tag is `llama3` (a model Ollama already has: `params.model` on `local_slm`, or a pack `model_hint` that is already a model tag). The train base is `Qwen/Qwen2.5-0.5B-Instruct` (pack `train_base_model`, or `params.train_base_model` on that same binding). `recipe.yaml` and `export.yaml` set `model_name_or_path` to the train base. `template` for that Qwen2.5 name is `qwen`. The seat tag stays `prepare.json` `base_model` and `seat_tag`. That smoke trained, exported, and seated this gauge from the files prepare wrote, with no hand-edits. A missing train base, a bare seat tag, or a local directory named like a seat tag is `refuse:train-base` and writes nothing. This factory does not map `llama3` onto a Hub repo and does not download weights. `<your-estate.yaml>` is a lab copy. `examples/estate.yaml` on `main` stays hash-locked.

`dataset.jsonl` with no `--from-feed` is a scaffold (or a three-row stub when `source_paths` is empty). Those rows are not training data. `--from-feed` copies instruct rows that are already under the cell state directory. A missing file is `refuse:dataset` and writes nothing.

### 1. Prepare

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id> \
  --driver llamafactory-lora \
  --job train \
  --state-dir .cell
```

Omitting `--job` is the same train job. `--job enrich` is `refuse:job` and writes nothing. The output directory is `.cell/enrich/<pack-id>/llamafactory-lora/`.

| File | What the Qwen card holds |
| --- | --- |
| `recipe.yaml` | SFT LoRA. `finetuning_type: lora`, no `quantization_bit`, no `quantization_method`, LoRA rank 8, `packing: false`, `cutoff_len` 512, `template: qwen` for `Qwen/Qwen2.5-0.5B-Instruct`. `model_name_or_path` is the train base. |
| `export.yaml` | Merge card. Same train base and `template: qwen`. No `quantization_bit`. This prepare did not merge. |
| `dataset.jsonl` | Instruct chat. `dataset_mode` is `scaffold` until `--from-feed`. |
| `prepare.json` | `job` `train`. `seat_tag` `llama3`. `train_base_model` `Qwen/Qwen2.5-0.5B-Instruct`. `promoted`, `auto_apply`, and `estate_rewritten` are false. `trained_shape` is absent until step 5. |
| `NEXT.md` | The train line, the export line, the merge-adapt line, the convert line, the seat line, and the import lines. |

A short gauge run adds `--max-steps 10`. `--official-scale` writes the longer SFT scale from LLaMA-Factory `examples/train_lora/qwen3_lora_sft.yaml` and leaves rank, packing, and the omitted quantization on this card. The default recipe stays one epoch. Command page: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md).

### 2. Train and export, outside this factory

Run the lines `NEXT.md` prints. On a CUDA host (`consumer-nvidia` or `rented-nvidia`):

```bash
pip install llamafactory
llamafactory-cli train .cell/enrich/<pack-id>/llamafactory-lora/recipe.yaml
llamafactory-cli export .cell/enrich/<pack-id>/llamafactory-lora/export.yaml
```

`estate enrich merge-adapt --prepared .cell/enrich/<pack-id>/llamafactory-lora --adapter .cell/enrich/<pack-id>/llamafactory-lora/outputs` prints that same `llamafactory-cli export` line and the keys from `examples/merge_lora/qwen3_lora_sft.yaml`. It does not run the export. `gguf-convert` and `local-seat` then print the next lines for `export/`.

This LoRA path does not require bitsandbytes. `recipe.yaml` leaves the base at 16-bit and omits quantization. Prepare on `apple-silicon` still writes the files. `NEXT.md` says the card expects CUDA LLaMA-Factory. This factory does not install LLaMA-Factory and does not run `llamafactory-cli`.

The train writes the adapter under `outputs/` (`adapter_config.json`). The merge writes `export/` (`config.json` and a `.safetensors` file whose name does not start with `adapter_model`). Do not set `quantization_bit` on that merge. `import-trained` refuses a real `quantization_bit` or `quantization_method` key on `export.yaml` (`refuse:export`). LLaMA-Factory does not write GGUF.

### 3. Print the GGUF convert

After `llamafactory-cli export` exits 0, the merged directory is `export/`. Print the llama.cpp line. This command does not convert and does not write a GGUF. A missing directory, an adapter directory, or a path that is already a GGUF is `refuse:seat`.

```bash
estate enrich gguf-convert \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --weights .cell/enrich/<pack-id>/llamafactory-lora/export
```

That prints:

```bash
python3 convert_hf_to_gguf.py .cell/enrich/<pack-id>/llamafactory-lora/export --outfile .cell/enrich/<pack-id>/llamafactory-lora/export.gguf --outtype auto
```

Run that line from a llama.cpp checkout. `--outtype auto` is the script default (highest-fidelity 16-bit float). The outfile is a sibling of the merged directory. This factory does not choose a quantization type.

### 4. Print the Ollama create

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --weights .cell/enrich/<pack-id>/llamafactory-lora/export.gguf
```

The create name is `cell-enrich-<pack-id>`. The seat tag in the report is `prepare.json` `seat_tag` (`llama3` in the example). The command prints a Modelfile whose `FROM` is that GGUF, then `ollama create cell-enrich-<pack-id> -f` that file. The same report prints `llama-cli -m` and `llama-server -m` for that GGUF. `--runtime llama.cpp` selects those lines. Ollama stays the default print. It does not create the model and does not run llama.cpp. To seat the merged directory itself, point `--weights` at `export` when that directory contains the LLaMA-Factory Modelfile (`FROM .`). That directory still points at `gguf-convert` before any llama.cpp load line. A missing GGUF is `refuse:seat`.

To seat `outputs/` without a merge, pass `--adapter` instead of `--weights`. The printed Modelfile uses `FROM` `prepare.json` `seat_tag` and `ADAPTER` that directory. The command prints `ollama create` and does not run it. `--weights` still refuses the adapter directory (`refuse:seat`). A merged export or a GGUF passed to `--adapter` is `refuse:adapter`.

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --adapter .cell/enrich/<pack-id>/llamafactory-lora/outputs
```

Page: [`local-seat.md`](local-seat.md).

### 5. Record the shape

`import-trained` writes `binding-proposal.json` for the existing `local_slm` seat and records `trained_shape` and `trained_paths` on that proposal and on `prepare.json`. It does not apply and does not promote. After the GGUF exists, point `--adapter` at that file. `trained_shape` is `gguf`.

```bash
estate enrich import-trained \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --tag cell-enrich-<pack-id> \
  --adapter .cell/enrich/<pack-id>/llamafactory-lora/export.gguf
```

The same command records the other two shapes when that is the artifact you have. An adapter directory (`outputs/`, `adapter_config.json`) is `trained_shape` `adapter`. A merged directory (`export/`, `config.json` plus a non-adapter `.safetensors` file) is `trained_shape` `merged`. A path that matches none of those shapes, or more than one, is `refuse:adapter` before a new proposal exists. `NEXT.md` prints all three lines with the prepared directory filled in.

`apply-proposal`, then `estate plan` and `estate apply --require-plan`, stay the join. Promote stays refused. `READY_FOR_LIVE_TEST`: no.

```bash
make lora-journey
```

That opt-in script prints this ladder, prepares `llamafactory-lora` on a throwaway copy of `examples/estate.yaml`, and checks the artifacts: seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`, `template: qwen`, rank 8, `packing: false`, no quantization keys, the `NEXT.md` train and export lines, the printed merge-adapt line, the printed convert line, the printed seat line, and the import lines. A seat tag with no train base is `refuse:train-base` and writes nothing. `gguf-convert` and `local-seat` against a missing export are `refuse:seat` and write no GGUF. The script prints `SKIP live train`. It leaves `examples/estate.yaml` unchanged. It is not in `make smoke`, `make gate-90`, or GitHub Actions.
