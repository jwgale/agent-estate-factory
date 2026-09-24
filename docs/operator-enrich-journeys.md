# Operator journeys: train/enrich

Walks for the suite goal: facilitate train and enrich of purpose-built small-parameter models. Today's beachhead is enrich packs, the specialist path, and prepare cards the operator runs outside the factory: LLaMA-Factory LoRA and QLoRA, plus Axolotl bf16 LoRA and 4-bit QLoRA.

Locked defaults: [`../charter.md`](../charter.md). Product page: [`NORTH-STAR.md`](NORTH-STAR.md). Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). Prepare command surface: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Live paste target: [`LIVE-PROBES.md`](LIVE-PROBES.md).

Fixture accept loop: [`FEED-LOOP.md`](FEED-LOOP.md). Seated drivers: [`operator-local.md`](operator-local.md).

Commands on this page: `estate enrich from-pack`, `estate enrich prepare`, `estate enrich list`, `estate enrich import-prepared`, `estate enrich import-trained`, `estate enrich merge-adapt`, `estate enrich gguf-convert`, `estate enrich local-seat`, and `estate enrich apply-proposal`. No new crate. The factory does not run Unsloth, Axolotl, or llama.cpp. `READY_FOR_LIVE_TEST` stays no. Recorded specialist rows stay on the live-probes page. The opt-in `ollama create` handoff is [`LIVE-PROBES.md`](LIVE-PROBES.md).

Popular path (Target C): Qwen through LLaMA-Factory QLoRA, then a printed GGUF convert, a printed Ollama create, and `import-trained` to record the shape. Section 8. The seat tag and the train base stay separate. Opt-in check: `make qlora-journey`. It prints that ladder and checks the prepare artifacts. It does not train, does not convert, and does not promote. `make train-next` prints the live train recipe from that prepare's `NEXT.md` and does not run it.

Unquantized path (Target A): Qwen through LLaMA-Factory LoRA, then a printed merge, a printed GGUF convert, a printed Ollama create, and `import-trained` to record the shape. Section 9. The seat tag and the train base stay separate. Opt-in check: `make lora-journey`. It prints that ladder and checks the prepare artifacts. It does not train, does not merge, does not convert, and does not promote.

Print path once a merged export and a GGUF exist (Target C seat ladder): the same Qwen QLoRA prepare, then a 5090-shaped export that is `refuse:tokenizer`, then fixture stubs, then the printed `merge-adapt`, `gguf-convert`, `local-seat`, and `import-trained` lines. Section 10. Opt-in check: `make seat-journey`. It prints those lines. It does not train, does not convert, does not create a model, and does not promote.

`make uniqueness-ladder` runs that Target C print path in order: `make qlora-journey`, then `make seat-journey`. Print-only. It does not train, merge, convert, seat, or promote, and it does not run `make lf-beachhead-prepare` or `make train-next`. The train step is the separate opt-in `make train-next`. Live train, live convert, and live seat still need a human GPU host and stay skipped. It is not in `make smoke`, `make gate-90`, or GitHub Actions.

`make uniqueness-full` runs the same prints with the train recipe in the middle: `make qlora-journey`, then `make train-next`, then `make seat-journey`. If one step fails, it exits nonzero before the next step. Print-only. It does not train, merge, convert, seat, or promote, and it does not run `make lf-beachhead-prepare`. It does not change `make uniqueness-ladder`. Live train, live convert, and live seat still need a human GPU host and stay skipped. It is not in `make smoke`, `make gate-90`, or GitHub Actions.

`make uniqueness-full-lora` runs the Target A print chain in that same order on the unquantized card: `make lora-journey`, then `make train-next-lora`, then `make seat-journey-lora`. `make train-next-lora` prepares `llamafactory-lora` (seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`, template `qwen`, rank 8, no quantization) and prints the `NEXT.md` train and export lines. It does not print a bitsandbytes install and does not run the lines. `CELL_TRAIN_LIVE=1` stays print-only. `make seat-journey-lora` is the seat script on that LoRA card. It keeps `refuse:adapter`, `refuse:tokenizer`, and `refuse:seat`, then prints `merge-adapt`, `gguf-convert`, `local-seat`, and `import-trained` against the same fixture stubs. If one step fails, the script exits nonzero before the next step. Print-only. It does not train, merge, convert, seat, or promote, and it does not run `make lf-beachhead-prepare`. It does not change `make uniqueness-full` or `make uniqueness-ladder`. It does not invent a live PASS. Live train, live convert, and live seat still need a human GPU host and stay skipped. It is not in `make smoke`, `make gate-90`, or GitHub Actions.

`make deepseek-r1-distill-journey` is the print-only DeepSeek-R1-Distill chat QLoRA ladder on `llamafactory-qlora` (section 19). Seat tag `llama3`. Train base `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`. Template `deepseekr1`. Fixture `examples/fixtures/deepseek-r1-distill.pack.json`. `make uniqueness-deepseek` runs the prepare-assert phase, then the seat-print phase. `make deepseek-r1-distill-lora-journey` and `make uniqueness-deepseek-lora` are the non-quant twin on `llamafactory-lora`. Print-only. They do not train, merge, convert, or promote. They do not invent a live PASS. They are not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no.

`make purpose-build-journey` is the print-only purpose-build on-demand entry (section 18). It runs `make purpose-build-pick` (section 17), then `make purpose-build-checklist` (section 15). It calls those targets through make and does not inline their bodies. It does not train, fuse, convert, shell out to ollama, promote, or apply the estate, and it does not invent a live PASS. The recorded PASS stays the only live uniqueness prove. It is not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no.

`make purpose-build-checklist` prints the ordered operator steps for purpose-build on demand (section 15). It points at the print-only cards already on tip: a beachhead prepare row or `make qlora-journey` / `make lora-journey` / the Axolotl and Unsloth journeys, then a train-next style `SKIP live train`, merge-adapt and export print honesty, `gguf-convert`, the local-seat print, `import-trained` (`trained_shape` `gguf`, `auto_apply=false`), and Standing next (estate). Print-only. It does not run those cards. It does not train, convert, shell out to ollama, promote, or apply the estate, and it does not invent a live PASS. The recorded PASS stays the only live uniqueness prove. The re-prove card stays `make uniqueness-prove-checklist`. It is not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no.

`make uniqueness-prove-checklist` prints the ordered operator steps for the recorded PASS in [`LIVE-PROBES.md`](LIVE-PROBES.md) (Target C live uniqueness (5090-class)). Print-only. It does not train, convert, shell out to ollama, or promote, and it does not invent a new live PASS. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. It is not in `make smoke`, `make gate-90`, or GitHub Actions. It is not native MLX. After step 8 (`import-trained`, `trained_shape` `gguf`, `auto_apply=false`) it prints Standing next (estate). The proposal stays `auto_apply=false`. The factory does not apply the estate without an explicit operator `--require-plan` path. The curator path is `packs accept --curator jason`. No promote and no auto-promote. `examples/estate.yaml` stays unchanged unless the operator deliberately applies a plan. The coda names `apply-proposal`, `plan`, `apply --require-plan`, and `reconcile` and does not execute them. The recorded PASS stays on that LIVE-PROBES section. The re-prove card is `make uniqueness-prove-checklist`.

Axolotl QLoRA print path: the same overnight pack, seat tag `llama3`, and train base `Qwen/Qwen2.5-0.5B-Instruct`, on `axolotl-qlora`. Section 11. Opt-in check: `make axolotl-qlora-journey`. It checks `axolotl.yml` (`adapter: qlora`, `load_in_4bit: true`, `sequence_len` 4096, `lora_r` 32, train base in `base_model`) and prints the merge, convert, seat, and import lines against fixture stubs. It does not run Axolotl. `make uniqueness-axolotl` runs the prepare-assert phase, then the seat-print phase. Print-only. It is not in `make smoke`, `make gate-90`, or GitHub Actions.

Unsloth QLoRA print path: the same overnight pack, seat tag `llama3`, and train base `Qwen/Qwen2.5-0.5B-Instruct`, on the optional NEXT card `unsloth-qlora`. Section 12. Opt-in check: `make unsloth-qlora-journey`. It checks `UNSLOTH.md` (seat tag and train base) and prints `save_pretrained_merged` (`merged_16bit`), the convert, the seat, and the import against fixture stubs. It does not call Unsloth. `make uniqueness-unsloth` runs the prepare-assert phase, then the seat-print phase. Print-only. Status stays `optional`. It is not in `make smoke`, `make gate-90`, or GitHub Actions.

Unsloth LoRA print path: the same overnight pack, seat tag `llama3`, and train base `Qwen/Qwen2.5-0.5B-Instruct`, on the optional NEXT card `unsloth-lora`. Section 14. That card is the non-quant twin of `unsloth-qlora`. Opt-in check: `make unsloth-lora-journey`. It checks `UNSLOTH.md` and prints `save_pretrained_merged` (`merged_16bit` and `save_method` `lora`), the convert, the seat, and the import against fixture stubs. It does not call Unsloth. `make uniqueness-unsloth-lora` runs the prepare-assert phase, then the seat-print phase. It does not run `make unsloth-qlora-journey` or `make uniqueness-unsloth`. Print-only. Status stays `optional`. It is not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no.

Axolotl LoRA print path: the same overnight pack, seat tag `llama3`, and train base `Qwen/Qwen2.5-0.5B-Instruct`, on `axolotl-lora`. Section 13. Opt-in check: `make axolotl-lora-journey`. It checks `axolotl.yml` (`adapter: lora`, `load_in_8bit: false`, `load_in_4bit: false`, `sequence_len` 2048, `lora_r` 16, train base in `base_model`) and prints `axolotl merge-lora` without `--dequant`, then the convert, seat, and import lines against the same fixture stubs. It does not run Axolotl. `make uniqueness-axolotl-lora` runs the prepare-assert phase, then the seat-print phase. Print-only. It does not run `make axolotl-qlora-journey` or `make uniqueness-axolotl`. It is not in `make smoke`, `make gate-90`, or GitHub Actions. It does not invent a live PASS. `READY_FOR_LIVE_TEST`: no.

The smoke pairs for Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, Qwen3 Instruct, DeepSeek-R1-Distill chat, and GLM-4 Chat are one table: [`lf-beachhead-matrix.md`](lf-beachhead-matrix.md). `estate help enrich` prints that file. A bare Ollama seat tag on those train bases is `refuse:train-base`. The table does not add a journey to `make smoke`, `make gate-90`, or GitHub Actions. Opt-in prepare walk: `make lf-beachhead-prepare`. It prepares every row on a throwaway copy of `examples/estate.yaml`, checks the matrix knobs, and prints `SKIP live train`. It does not train, merge, convert, seat, or promote. Phi-3-small stays QLoRA-only and is not a row.

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

The beachhead matrix for the current LoRA and QLoRA smoke pairs is [`lf-beachhead-matrix.md`](lf-beachhead-matrix.md). `estate help train` prints that file.

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

`<your-estate.yaml>` is a lab copy. It needs `params.model` on `local_slm` (a model the seat already has, such as `llama3`) or a pack `model_hint` that is already a model tag. It also needs a train base: pack field `train_base_model`, or `params.train_base_model` on that same binding. The train base is a Hugging Face repo id (`namespace/name`) or a local directory of HF weights. A `./` or `../` directory is written into the recipe as an absolute path. A directory named like an Ollama seat tag (`./llama3`) is `refuse:train-base`. A 5090 smoke used `Qwen/Qwen2.5-0.5B-Instruct` while the seat tag stayed `llama3`. Phi-3 and Phi-3.5 Instruct are a reproduce target beside that Qwen LoRA/QLoRA pair: `examples/fixtures/phi3-instruct.pack.json` sets `train_base_model` to `microsoft/Phi-3-mini-4k-instruct`, and `llamafactory-qlora` writes template `phi` (`phi_small` for Phi-3-small) with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target. A nested path segment such as `./weights/Phi-3.5-mini-instruct` uses the same template. `examples/fixtures/phi3-instruct-lora.pack.json` is the same train base on `llamafactory-lora`. That card writes template `phi`, `lora_rank` 8, `packing: false`, and no quantization keys. Its reproduce line is LoRA-only and only for Phi-3 mini, Phi-3 medium, and Phi-3.5. Phi-3-small keeps template `phi_small` and does not get that line. Phi-4 and Phi-4-mini do not get that line. A lowercase directory leaf `phi-3-mini-4k-instruct` stays `refuse:train-base`. Llama-3.2 Instruct is a reproduce target beside Phi-3 and that Qwen pair: `examples/fixtures/llama32-instruct.pack.json` sets `train_base_model` to `meta-llama/Llama-3.2-3B-Instruct`, and `llamafactory-qlora` writes template `llama3` (`mllama` for Llama-3.2 vision) with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target. A nested path segment such as `./weights/Llama-3.2-3B-Instruct` uses the same template. `examples/fixtures/llama32-instruct-lora.pack.json` is the same train base on `llamafactory-lora`. That card writes template `llama3`, `lora_rank` 8, `packing: false`, and no quantization keys. Its reproduce line is LoRA-only. Llama-3.2 vision, a Llama-3.2 base, and Llama-3.1 Instruct do not get that line. A lowercase directory leaf `llama-3.2-3b-instruct` stays `refuse:train-base`. Gemma-2 Instruct is a reproduce target beside Phi-3, Llama-3.2, and that Qwen pair: `examples/fixtures/gemma2-instruct.pack.json` sets `train_base_model` to `google/gemma-2-2b-it`, and `llamafactory-qlora` writes template `gemma2` (`gemma` for original Gemma 2B and 7B) with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target only for a Gemma-2 Instruct id. `examples/fixtures/gemma2-instruct-lora.pack.json` is the same train base on `llamafactory-lora`. That card writes template `gemma2`, `lora_rank` 8, and no quantization keys. Its reproduce line is LoRA-only. A Gemma-2 base keeps template `gemma2` and does not get either line. A nested path whose leaf is `weights` or a snapshot hash, such as `./weights/google/gemma-2-9b-it/weights`, uses the same template. A lowercase directory leaf `gemma-2-2b-it` stays `refuse:train-base`. Mistral Instruct is a reproduce target beside Phi-3, Llama-3.2, Gemma-2, and that Qwen pair: `examples/fixtures/mistral-instruct.pack.json` sets `train_base_model` to `mistralai/Mistral-7B-Instruct-v0.3`, and `llamafactory-qlora` writes template `mistral` (`mistral_small` for Mistral-Small, `ministral` for Mistral-Nemo) with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target only for a Mistral-7B Instruct id. `examples/fixtures/mistral-instruct-lora.pack.json` is the same train base on `llamafactory-lora`. That card writes template `mistral`, `lora_rank` 8, `packing: false`, and no quantization keys. Its reproduce line is LoRA-only. A Mistral-7B base keeps template `mistral` and does not get either line. Mistral-Small, Mistral-Nemo, Mixtral, and LLaVA-NeXT-Mistral do not get either line. A nested path whose leaf is `weights` or a snapshot hash, such as `./weights/mistralai/Mistral-7B-Instruct-v0.3/weights`, uses the same template. A lowercase directory leaf `mistral-7b-instruct-v0.3` stays `refuse:train-base`. Qwen2.5 Instruct is a reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct: `examples/fixtures/qwen25-instruct.pack.json` sets `train_base_model` to `Qwen/Qwen2.5-0.5B-Instruct`, and `llamafactory-qlora` writes template `qwen` with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target only for a Qwen2.5 Instruct text id. `examples/fixtures/qwen25-instruct-lora.pack.json` is the same train base on `llamafactory-lora`. That card writes template `qwen`, `lora_rank` 8, `packing: false`, and no quantization keys. Its reproduce line is LoRA-only and only for that Qwen2.5 Instruct shape. A Qwen2.5 base, a name containing thinking, Qwen2, Qwen2.5-Coder, Qwen2.5-Math, and Qwen2.5-VL do not get either line. A Qwen3 Instruct id does not get that LoRA line. A nested path whose leaf is `weights` or a snapshot hash, such as `./weights/Qwen/Qwen2.5-0.5B-Instruct/weights` or `models--Qwen--Qwen2.5-0.5B-Instruct`, uses the same template. A lowercase directory leaf `qwen2.5-0.5b-instruct` stays `refuse:train-base`. Qwen3 Instruct is a reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x: `examples/fixtures/qwen3-instruct.pack.json` sets `train_base_model` to `Qwen/Qwen3-4B-Instruct-2507`, and `llamafactory-qlora` writes template `qwen3_nothink` (`qwen3` for a Qwen3 thinking or base id, `qwen` for Qwen2.5) with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target only for that Qwen3 Instruct shape. `examples/fixtures/qwen3-instruct-lora.pack.json` is the same train base on `llamafactory-lora`. That card matches `examples/train_lora/qwen3_lora_sft.yaml`: template `qwen3_nothink`, `lora_rank` 8, and no quantization keys. Its reproduce line is LoRA-only. `Qwen/Qwen3-4B` does not get the Qwen3 lines. `Qwen/Qwen2.5-0.5B-Instruct` does not get the Qwen3 lines. It is the Qwen2.5 Instruct reproduce target on `llamafactory-qlora` and the non-quant twin on `llamafactory-lora`. A nested path whose leaf is `weights` or a snapshot hash, such as `./weights/Qwen/Qwen3-4B-Instruct-2507/weights` or `models--Qwen--Qwen3-4B-Instruct-2507`, uses the same template. A lowercase directory leaf `qwen3-4b-instruct-2507` stays `refuse:train-base`. DeepSeek-R1-Distill chat is a QLoRA reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, and Qwen3 Instruct: `examples/fixtures/deepseek-r1-distill.pack.json` sets `train_base_model` to `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`, and `llamafactory-qlora` writes template `deepseekr1` with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target only for the six official distill chat ids. A Qwen or Llama substring in those ids stays `deepseekr1`. DeepSeek-R1, DeepSeek-R1-Zero, and DeepSeek-R1-0528 use `deepseekr1` and do not get either line. `examples/fixtures/deepseek-r1-distill-lora.pack.json` is the same train base on `llamafactory-lora`. That card writes template `deepseekr1`, `lora_rank` 8, `packing: false`, and no quantization keys. Its reproduce line is LoRA-only and only for those six distill chat ids. The QLoRA note stays on `llamafactory-qlora`. `examples/train_lora` does not ship a DeepSeek yaml. The Ollama tag `deepseek-r1:1.5b` is a seat tag. The smoke seat stays `llama3`. A nested path whose leaf is `weights` or a snapshot hash, such as `./weights/deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B/weights` or `models--deepseek-ai--DeepSeek-R1-Distill-Qwen-1.5B`, uses the same template. A lowercase directory leaf `deepseek-r1-distill-qwen-1.5b` stays `refuse:train-base`. A bare `deepseek-r1` or `deepseek-r1:1.5b` stays `refuse:train-base`. GLM-4 Chat is a QLoRA reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, Qwen3 Instruct, and DeepSeek-R1-Distill chat: `examples/fixtures/glm4-chat.pack.json` sets `train_base_model` to `zai-org/glm-4-9b-chat`, and `llamafactory-qlora` writes template `glm4` with `quantization_method: bnb` and `quantization_bit: 4`. `NEXT.md` and `PREPARE.md` name that reproduce target only for the official GLM-4 Chat ids in the `template="glm4"` group. A GLM-4 base keeps template `glm4` and does not get either line. GLM-Z1, GLM-4.1V, GLM-4.5, and ChatGLM3 do not get either line. `examples/fixtures/glm4-chat-lora.pack.json` is the same train base on `llamafactory-lora`. That card writes template `glm4`, `lora_rank` 8, `packing: false`, and no quantization keys. Its reproduce line is LoRA-only and only for those GLM-4 Chat ids. The QLoRA note stays on `llamafactory-qlora`. `examples/train_lora` does not ship a GLM-4 yaml. The Ollama tags `glm4`, `glm4:9b`, and `glm-4:9b` are seat tags. The smoke seat stays `llama3`. A nested path whose leaf is `weights` or a snapshot hash, such as `./weights/zai-org/glm-4-9b-chat/weights` or `models--zai-org--glm-4-9b-chat`, uses the same template. A lowercase directory leaf `glm-4-9b-chat` stays `refuse:train-base`. A bare `glm4` or `glm4:9b` stays `refuse:train-base`. Prepare does not download weights. `examples/estate.yaml` on `main` stays hash-locked. A binding id as `FROM` is `refuse:base-model`. A missing train base, or a bare Ollama tag in that field, is `refuse:train-base`.

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

On Apple Silicon, mlx-lm already documents LoRA and fuse. The optional card is `mlx-lm-lora`. The print-only check is section 16. It writes `MLX.md` when `host_class_affinity` is `apple-silicon`. Another affinity is `refuse:host` and writes nothing. `NEXT.md` points at https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md and the fuse command that page publishes (`mlx_lm.fuse --model <path_to_model>`). After `mlx_lm.lora`, `estate enrich merge-adapt` prints `mlx_lm.fuse` with `--adapter-path` and `--save-path`, plus `--export-gguf`. That writes `fused_model/ggml-model-f16.gguf`. `local-seat` prints the Ollama line for that file. `import-trained` records the adapter directory or that GGUF file. A fused MLX directory is `refuse:adapter`. This factory does not call mlx-lm. The `mlx` runtime card stays a stub.

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

When you want a second YAML recipe or a multi-GPU run, prepare `axolotl-lora` or `axolotl-qlora` the same way, including `--from-feed` when the sources are already on disk. Each card writes `axolotl.yml` and an Alpaca `dataset.jsonl`. `axolotl-lora` is bf16 LoRA matching Axolotl `examples/llama-3/lora-1b.yml` (`adapter: lora`, `load_in_4bit: false`, `sequence_len` 2048, `micro_batch_size` 2, `gradient_accumulation_steps` 2, `lora_r` 16). `axolotl-qlora` is 4-bit QLoRA matching `examples/llama-3/qlora.yml` (`adapter: qlora`, `load_in_4bit: true`, `sequence_len` 4096, `micro_batch_size` 2, `gradient_accumulation_steps` 4, `lora_r` 32). `base_model` in that yaml is the train base. `prepare.json` keeps the Ollama seat tag for Modelfile `FROM` and for the adapter join (`FROM` a merged GGUF, or `FROM` an Ollama model of that train base plus `ADAPTER`), and it stores the same `dataset_mode` fields. `PREPARE.md` and `NEXT.md` use the same dataset paragraph as the LLaMA-Factory cards. A missing train base, a bare seat tag, or a local directory named like a seat tag (`./llama3`) is `refuse:train-base` and writes nothing. A missing feed file with `--from-feed` is `refuse:dataset` and writes nothing. `--all-drivers --job train` writes the train base into both Axolotl yamls and leaves Modelfile `FROM` as the seat tag. On the CUDA host, run `axolotl train` on that yaml. A short gauge run passes `--max-steps`. `estate enrich merge-adapt` then prints `axolotl merge-lora` for the adapter directory. Axolotl writes `output_dir/merged`. `axolotl-qlora` also prints `--dequant`. This factory does not run the merge. Axolotl does not write GGUF. `NEXT.md` names `merge-adapt`, `gguf-convert`, `local-seat`, and `import-trained` for that directory. `import-trained` also takes the adapter directory. Unsloth stays the `NEXT.md` pointer on the LLaMA-Factory card. Opt-in check, with no LLaMA-Factory process and no Axolotl process: `make train-prepare`. It prints `SKIP live train`. Opt-in print check for the 4-bit Axolotl card: `make axolotl-qlora-journey` (section 11). `make uniqueness-axolotl` chains its prepare-assert phase and its seat-print phase. Neither runs Axolotl.

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

The printed create name is `cell-enrich-<pack-id>`. The seat tag in the report is `prepare.json` `seat_tag`. `--weights` on that local-seat line is the sibling `.gguf`. On that GGUF print-only path, `local-seat` is print-only. It prints the Modelfile and does not write `$PREPARED/Modelfile`. Write that file from the printed contents before `ollama create`. To seat the merged directory itself, point `--weights` at `export` when that directory contains the LLaMA-Factory Modelfile (`FROM .`). The report uses the on-disk Modelfile when FROM already names the artifact. Do not write `$PREPARED/Modelfile` again. The command prints `ollama create cell-enrich-<pack-id> -f <export>/Modelfile`. It does not create the model.

After a GGUF `local-seat` print, run the printed `ollama create` line yourself. The same step stands when you already ran `ollama create` outside this factory. This factory did not run `ollama create`. The standing next step is `import-trained --adapter` on that same GGUF. `trained_shape` is `gguf`. The proposal stays `auto_apply=false`. `import-trained` does not apply the estate. A merged export_dir is `config.json` plus a `.safetensors` file whose name does not start with `adapter_model`, with an optional Modelfile. `import-trained` records `trained_shape` and `trained_paths`. The seat tag on the proposal stays the prepare seat tag. `apply-proposal` and `estate apply --require-plan` stay the join. `READY_FOR_LIVE_TEST`: no. Page: [`local-seat.md`](local-seat.md).

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

`make train-next` prints the install lines and the `llamafactory-cli train` line from a throwaway `llamafactory-qlora` prepare. It also prints the export line. It does not run either line. `CELL_TRAIN_LIVE=1` stays print-only.

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

A live 5090 prove on 2026-09-23 hit two tokenizer problems in that export of `Qwen/Qwen2.5-0.5B-Instruct` before the convert could write a GGUF. `tokenizer_config.json` had `extra_special_tokens` as a list, and transformers raised `AttributeError: 'list' object has no attribute 'keys'`. The export also omitted `vocab.json` and `merges.txt`, which that train-base tokenizer includes. Copy the tokenizer files from the train base already on disk into `export/`. The source is the HF cache snapshot for that repo, or the equivalent base checkout (a local HF directory). HF hub snapshots are often symlinks into the HF cache. Copy with dereference (`cp -aL` or `cp --dereference`, or the equivalent) so the files in the export directory are real files, not symlinks. A plain `cp -a` leaves `tokenizer_config.json` as a symlink. Enrich does not follow a symlinked `tokenizer_config.json`. Keep the export `tokenizer_config.json` as `tokenizer_config.json.bak`. `gguf-convert` returns `refuse:tokenizer` for that list, for JSON null under `extra_special_tokens`, and when the export is Qwen-family and either BPE file is missing. JSON null is a non-object: transformers calls `.keys()` on that value. Qwen-family is `config.json` `model_type` or `architectures`, or `tokenizer_class`, naming Qwen. Then re-run `estate enrich gguf-convert` on that export directory. This factory does not download those files, does not copy them, and does not run the convert. The same export shape is recorded in [LLaMA-Factory issue 10169](https://github.com/hiyouga/LlamaFactory/issues/10169). After the files are restored, the convert on that prove wrote a 949M BF16 GGUF.

### 4. Print the Ollama create

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export.gguf
```

The create name is `cell-enrich-<pack-id>`. The seat tag in the report is `prepare.json` `seat_tag` (`llama3` in the example). On that GGUF print-only path, `local-seat` is print-only. It prints the Modelfile and does not write `$PREPARED/Modelfile`. Write that file from the printed contents before `ollama create`. The printed Modelfile `FROM` is that GGUF, then the report prints `ollama create cell-enrich-<pack-id> -f` that file. The same report prints `llama-cli -m` and `llama-server -m` for that GGUF. `--runtime llama.cpp` selects those lines. Ollama stays the default print. It does not create the model and does not run llama.cpp. After you write `$PREPARED/Modelfile`, run the printed `ollama create` line yourself. The same step stands when you already ran `ollama create` outside this factory. This factory did not run `ollama create`. The standing next step records that GGUF. To seat the merged directory itself, point `--weights` at `export` when that directory contains the LLaMA-Factory Modelfile (`FROM .`). The report uses the on-disk Modelfile when FROM already names the artifact. Do not write `$PREPARED/Modelfile` again. That directory still points at `gguf-convert` before any llama.cpp load line. A missing GGUF is `refuse:seat`.

To seat `outputs/` without a merge, pass `--adapter` instead of `--weights`. When that Modelfile is not already on disk, `local-seat` is print-only. It prints the Modelfile and does not write that file. Write that file from the printed contents before `ollama create`. When FROM is the seat tag and ADAPTER already names that directory, the report uses that on-disk Modelfile. The printed Modelfile uses `FROM` `prepare.json` `seat_tag` and `ADAPTER` that directory. The command prints `ollama create` and does not run it. `--weights` still refuses the adapter directory (`refuse:seat`). A merged export or a GGUF passed to `--adapter` is `refuse:adapter`.

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --adapter .cell/enrich/<pack-id>/llamafactory-qlora/outputs
```

Page: [`local-seat.md`](local-seat.md).

### 5. Record the shape

`import-trained` is the standing next step after the GGUF `local-seat` print. It writes `binding-proposal.json` for the existing `local_slm` seat and records `trained_shape` and `trained_paths` on that proposal and on `prepare.json`. After the GGUF exists, point `--adapter` at that file. `trained_shape` is `gguf`. The proposal stays `auto_apply=false`. `import-trained` does not apply the estate and does not promote.

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

That opt-in script prints this ladder, prepares `llamafactory-qlora` on a throwaway copy of `examples/estate.yaml`, and checks the artifacts: seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`, the `NEXT.md` train and export lines, the printed convert line, the printed seat line, and the import lines. A seat tag with no train base is `refuse:train-base` and writes nothing. `gguf-convert` and `local-seat` against a missing export are `refuse:seat` and write no GGUF. The script prints `SKIP live train`. It leaves `examples/estate.yaml` unchanged. It is not in `make smoke`, `make gate-90`, or GitHub Actions. `make train-next` is the opt-in that prints the live train recipe from `NEXT.md` on that same card and does not run it. `make seat-journey` (section 10) is the opt-in that asserts `refuse:tokenizer` on a 5090-shaped export, then prints `merge-adapt`, `gguf-convert`, `local-seat`, and `import-trained` after the good fixture stubs stand in for the merged export and the GGUF. `make uniqueness-ladder` runs `make qlora-journey`, then `make seat-journey`. It does not run `make train-next`. `make uniqueness-full` runs `make qlora-journey`, then `make train-next`, then `make seat-journey`. It does not train.

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

The same tokenizer check as Target C applies after `llamafactory-cli export`. A JSON list or JSON null under `extra_special_tokens`, or a Qwen-family export missing `vocab.json` or `merges.txt`, is `refuse:tokenizer`. Copy the tokenizer files from `Qwen/Qwen2.5-0.5B-Instruct` already on disk into `export/`. The source is the HF cache snapshot for that repo, or the equivalent base checkout. HF hub snapshots are often symlinks into the HF cache. Copy with dereference (`cp -aL` or `cp --dereference`, or the equivalent) so the files in the export directory are real files, not symlinks. A plain `cp -a` leaves `tokenizer_config.json` as a symlink. Enrich does not follow a symlinked `tokenizer_config.json`. Keep the export `tokenizer_config.json` as `tokenizer_config.json.bak`. Then re-run `estate enrich gguf-convert` on that export directory. A live 5090 prove on 2026-09-23 used that restore and then wrote a 949M BF16 GGUF. This factory does not download those files and does not copy them. Detail is in section 8.

### 4. Print the Ollama create

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --weights .cell/enrich/<pack-id>/llamafactory-lora/export.gguf
```

The create name is `cell-enrich-<pack-id>`. The seat tag in the report is `prepare.json` `seat_tag` (`llama3` in the example). On that GGUF print-only path, `local-seat` is print-only. It prints the Modelfile and does not write `$PREPARED/Modelfile`. Write that file from the printed contents before `ollama create`. The printed Modelfile `FROM` is that GGUF, then the report prints `ollama create cell-enrich-<pack-id> -f` that file. The same report prints `llama-cli -m` and `llama-server -m` for that GGUF. `--runtime llama.cpp` selects those lines. Ollama stays the default print. It does not create the model and does not run llama.cpp. After you write `$PREPARED/Modelfile`, run the printed `ollama create` line yourself. The same step stands when you already ran `ollama create` outside this factory. This factory did not run `ollama create`. The standing next step records that GGUF. To seat the merged directory itself, point `--weights` at `export` when that directory contains the LLaMA-Factory Modelfile (`FROM .`). The report uses the on-disk Modelfile when FROM already names the artifact. Do not write `$PREPARED/Modelfile` again. That directory still points at `gguf-convert` before any llama.cpp load line. A missing GGUF is `refuse:seat`.

To seat `outputs/` without a merge, pass `--adapter` instead of `--weights`. When that Modelfile is not already on disk, `local-seat` is print-only. It prints the Modelfile and does not write that file. Write that file from the printed contents before `ollama create`. When FROM is the seat tag and ADAPTER already names that directory, the report uses that on-disk Modelfile. The printed Modelfile uses `FROM` `prepare.json` `seat_tag` and `ADAPTER` that directory. The command prints `ollama create` and does not run it. `--weights` still refuses the adapter directory (`refuse:seat`). A merged export or a GGUF passed to `--adapter` is `refuse:adapter`.

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --adapter .cell/enrich/<pack-id>/llamafactory-lora/outputs
```

Page: [`local-seat.md`](local-seat.md).

### 5. Record the shape

`import-trained` is the standing next step after the GGUF `local-seat` print. It writes `binding-proposal.json` for the existing `local_slm` seat and records `trained_shape` and `trained_paths` on that proposal and on `prepare.json`. After the GGUF exists, point `--adapter` at that file. `trained_shape` is `gguf`. The proposal stays `auto_apply=false`. `import-trained` does not apply the estate and does not promote.

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

`make train-next-lora` is the opt-in that prints the live train recipe from `NEXT.md` on that same LoRA card and does not run it. The printed lines are `pip install llamafactory` and `llamafactory-cli train` on `recipe.yaml`, then the export line. There is no `pip install 'bitsandbytes>=0.49'` line. `CELL_TRAIN_LIVE=1` stays print-only. A missing `llamafactory-cli` is an informational SKIP. `make uniqueness-full-lora` runs `make lora-journey`, then `make train-next-lora`, then `make seat-journey-lora`. `make seat-journey-lora` is section 10 on this card.

## 10. Target C seat ladder — fixture stubs print the merge, convert, seat, and import

`make qlora-journey` prints the Target C ladder and stops at `refuse:seat` when `export/` and `export.gguf` are missing. This section is the opt-in print path that runs after fixture stubs stand in for that merged export and the sibling GGUF. The stubs are files the script writes in a throwaway directory. They are not weights. The factory prints the commands that already exist. It does not train, does not run `llamafactory-cli`, does not run `convert_hf_to_gguf.py`, does not run `ollama create`, and does not promote.

The prepare is the same Target C card as section 8: `llamafactory-qlora`, seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`, pack `examples/fixtures/specialist-overnight.pack.json`. `<your-estate.yaml>` is a lab copy. `examples/estate.yaml` stays hash-locked.

A seat tag with no train base is `refuse:train-base` and writes nothing. Before the stubs exist, `merge-adapt` on a missing `outputs/` is `refuse:adapter`. `gguf-convert` on a missing `export/` is `refuse:seat`. `local-seat` on a missing `export.gguf` is `refuse:seat`. Those refuses write no export directory and no GGUF.

After `merge-adapt` prints, the script writes a 5090-shaped export under `export/`. `config.json` sets `model_type` to `qwen2` and `architectures` to `Qwen2ForCausalLM`. `tokenizer_config.json` sets `extra_special_tokens` to a JSON list (`<|im_start|>` and `<|im_end|>`). `vocab.json` and `merges.txt` are absent. `model.safetensors` is the merged-weight marker, so the directory is a merged export. `gguf-convert` returns `refuse:tokenizer`, names the JSON list and both missing BPE files, names restoring tokenizer files from the HF cache snapshot for train base `Qwen/Qwen2.5-0.5B-Instruct` (or the equivalent base checkout) into the export directory. HF hub snapshots are often symlinks into the HF cache. Copy with dereference (`cp -aL` or `cp --dereference`, or the equivalent) so the files in the export directory are real files, not symlinks. A plain `cp -a` leaves `tokenizer_config.json` as a symlink. Enrich does not follow a symlinked `tokenizer_config.json`. Then re-run `estate enrich gguf-convert`. The refuse does not print `python3 convert_hf_to_gguf.py`. The script does not write `tokenizer_config.json.bak`, `vocab.json`, or `merges.txt`.

The script then removes that fixture and writes the good stubs. The good `config.json` is `{}`. It has no Qwen marker and no `tokenizer_config.json`, so `gguf-convert` prints the convert line.

The good stubs under the prepared directory are:

| Stub | What the printer needs |
| --- | --- |
| `outputs/adapter_config.json` | `merge-adapt` reads an adapter directory. `export.yaml` `adapter_name_or_path` is that directory. |
| `export/config.json` and `export/model.safetensors` | A merged export is `config.json` plus a `.safetensors` file whose name does not start with `adapter_model`. `gguf-convert` prints for that directory. |
| `export.gguf` | The first four bytes are `GGUF`. `local-seat` refuses a file that does not start with that magic. `import-trained` records the file. |

`export.gguf` stays beside `export/`. A `.gguf` inside `export/` makes the directory match two shapes, and both `local-seat` and `import-trained` then refuse the directory.

The printed lines are the same shapes section 8 names. For the overnight pack the create name is `cell-enrich-overnight-traces`:

```bash
llamafactory-cli export <prepared>/export.yaml
python3 convert_hf_to_gguf.py <prepared>/export --outfile <prepared>/export.gguf --outtype auto
ollama create cell-enrich-overnight-traces -f <prepared>/Modelfile
estate enrich import-trained --estate <your-estate.yaml> --prepared <prepared> --tag cell-enrich-overnight-traces --adapter <prepared>/export.gguf
```

`gguf-convert` also prints the `local-seat` line for `export.gguf`, then the standing next step: `estate enrich import-trained` for that same file. `local-seat` on that file prints `ollama create`, `llama-cli -m`, and `llama-server -m`, then the same import line. The report says to run `ollama create` yourself. This factory did not run `ollama create`. The proposal stays `auto_apply=false`. `import-trained` records `trained_shape` `gguf` and `trained_paths` on the throwaway prepare and writes `binding-proposal.json`. It does not apply the estate and does not rewrite `examples/estate.yaml`.

The script prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` does not start a convert or an `ollama create`. Live train, merge, convert, and seat stay on the operator host, the CUDA steps in section 8. `make enrich-live-prove` still covers a from-pack Modelfile and `ollama create`. It does not cover a trained GGUF. `READY_FOR_LIVE_TEST`: no.

```bash
make seat-journey
```

That opt-in script prepares `llamafactory-qlora` on a throwaway copy of `examples/estate.yaml`, checks the Qwen QLoRA card (seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`, `quantization_bit: 4`, `quantization_method: bnb`), asserts `refuse:train-base`, `refuse:adapter`, `refuse:seat`, and `refuse:tokenizer`, replaces the 5090-shaped export with the good stubs, and checks the printed lines. It leaves `examples/estate.yaml` unchanged. It is not in `make smoke`, `make gate-90`, or GitHub Actions.

```bash
make seat-journey-lora
```

`make seat-journey` pins `SEAT_CARD=llamafactory-qlora`. `make seat-journey-lora` runs the same script with `SEAT_CARD=llamafactory-lora`. The prepare card is the unquantized LoRA recipe (template `qwen`, `lora_rank` 8, `packing: false`, no quantization keys, no bitsandbytes install line). The refuse path is the same: a missing adapter is `refuse:adapter`, a missing export or GGUF is `refuse:seat`, and the 5090-shaped export is `refuse:tokenizer` before the good stubs. The script does not copy tokenizer files, does not write `tokenizer_config.json.bak`, and does not print `python3 convert_hf_to_gguf.py` on that refuse. `import-trained` still records `trained_shape` `gguf` with `auto_apply=false`. It does not weaken those refuses. It does not train, merge, convert, seat, or promote.

## 11. Axolotl QLoRA — popular-config print journey

This is the print-only check for the 4-bit Axolotl card. Axolotl already trains from `examples/llama-3/qlora.yml`. This factory writes `axolotl.yml` and prints the next command. It does not install Axolotl, does not run `axolotl`, does not merge, does not run `convert_hf_to_gguf.py`, does not run `ollama create`, and does not promote.

The prepare uses the overnight pack `examples/fixtures/specialist-overnight.pack.json` on a throwaway copy of `examples/estate.yaml`. The seat tag is `llama3`. The train base is `Qwen/Qwen2.5-0.5B-Instruct`. `axolotl.yml` sets `base_model` to that train base. The documented card is `adapter: qlora`, `load_in_8bit: false`, `load_in_4bit: true`, `sequence_len: 4096`, `micro_batch_size: 2`, `gradient_accumulation_steps: 4`, and `lora_r: 32`, matching Axolotl `examples/llama-3/qlora.yml`. `examples/estate.yaml` stays hash-locked.

A seat tag with no train base is `refuse:train-base` and writes nothing. Before the stubs exist, `merge-adapt` on a missing `outputs/` is `refuse:adapter`. `gguf-convert` on a missing `outputs/merged` is `refuse:seat`. `local-seat` on a missing `outputs/merged.gguf` is `refuse:seat`. Those refuses write no merged directory and no GGUF.

After `merge-adapt` prints `axolotl merge-lora` (the line without `--dequant`, and the `--dequant` line that writes a bf16 checkpoint), the script writes a Qwen-shaped directory under `outputs/merged`. `config.json` sets `model_type` to `qwen2` and `architectures` to `Qwen2ForCausalLM`. `tokenizer_config.json` sets `extra_special_tokens` to a JSON list. `vocab.json` and `merges.txt` are absent. `gguf-convert` returns `refuse:tokenizer` and does not print `python3 convert_hf_to_gguf.py`. The script then replaces that fixture with `config.json` `{}` plus `model.safetensors`. That safetensors name does not start with `adapter_model`.

The good stubs under the prepared directory are:

| Stub | What the printer needs |
| --- | --- |
| `outputs/adapter_config.json` | `merge-adapt` reads an adapter directory. Axolotl's `output_dir` is that directory. |
| `outputs/merged/config.json` and `outputs/merged/model.safetensors` | A merged Hugging Face directory is `config.json` plus a `.safetensors` file whose name does not start with `adapter_model`. `gguf-convert` prints for that directory. |
| `outputs/merged.gguf` | The first four bytes are `GGUF`. `local-seat` refuses a file that does not start with that magic. `import-trained` records the file. |

`outputs/merged.gguf` stays beside `outputs/merged/`. A `.gguf` inside the merged directory makes that directory match two shapes.

The printed lines for the overnight pack (`cell-enrich-overnight-traces`) are:

```bash
axolotl merge-lora <prepared>/axolotl.yml --lora-model-dir=<prepared>/outputs --dequant
python3 convert_hf_to_gguf.py <prepared>/outputs/merged --outfile <prepared>/outputs/merged.gguf --outtype auto
ollama create cell-enrich-overnight-traces -f <prepared>/outputs/Modelfile
estate enrich import-trained --estate <your-estate.yaml> --prepared <prepared> --tag cell-enrich-overnight-traces --adapter <prepared>/outputs/merged.gguf
```

`local-seat` is print-only. It prints the Modelfile and does not write `<prepared>/outputs/Modelfile`. Write that file from the printed contents before `ollama create`. The report says this factory did not run `ollama create`. The proposal stays `auto_apply=false`. `import-trained` records `trained_shape` `gguf`. It does not apply the estate.

The script prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` and `CELL_TRAIN_LIVE=1` do not start a convert or an `ollama create`. `READY_FOR_LIVE_TEST`: no.

```bash
make axolotl-qlora-journey
```

That opt-in script runs the prepare asserts and the seat prints in one process. `AXOLOTL_QLORA_PHASE=prepare` stops after the card check and the missing-path refuses. `AXOLOTL_QLORA_PHASE=seat` prints the fixture ladder.

```bash
make uniqueness-axolotl
```

That opt-in script chains the prepare-assert phase, then the seat-print phase. If the prepare phase fails, it exits nonzero before the seat print. It does not run `make qlora-journey`, `make seat-journey`, or `make train-next`. It leaves `examples/estate.yaml` unchanged. It is not in `make smoke`, `make gate-90`, or GitHub Actions.

## 12. Unsloth QLoRA — optional NEXT print journey

This is the print-only check for the optional Nvidia-only QLoRA handoff. The card writes `UNSLOTH.md`. It does not write a script, a recipe, or `dataset.jsonl`. It does not call Unsloth, does not merge, does not run `convert_hf_to_gguf.py`, does not run `ollama create`, and does not promote. Status stays `optional`.

The prepare uses the overnight pack `examples/fixtures/specialist-overnight.pack.json` on a throwaway copy of `examples/estate.yaml`. The seat tag is `llama3`. The train base is `Qwen/Qwen2.5-0.5B-Instruct`. `UNSLOTH.md` records both. `prepare.json` keeps them split. `examples/estate.yaml` stays hash-locked.

A seat tag with no train base is `refuse:train-base` and writes nothing. Before the stubs exist, `merge-adapt` on a missing adapter directory is `refuse:adapter`. An adapter directory that holds `adapter_config.json` and no `adapter_model.safetensors` is `refuse:adapter`. `adapters.safetensors` is the same refuse. `gguf-convert` on a missing `merged` directory is `refuse:seat`. `local-seat` on a missing `merged.gguf` is `refuse:seat`. Those refuses write no merged directory and no GGUF.

After the adapter stub exists (`adapter_config.json` plus `adapter_model.safetensors`), `local-seat --adapter` stays `refuse:adapter`. Passing that directory to `--weights` is `refuse:seat`. `merge-adapt` then prints `model.save_pretrained_merged` with `save_method` `merged_16bit`. The directory argument is `merged` beside the prepare. The script writes a Qwen-shaped directory there. `config.json` sets `model_type` to `qwen2` and `architectures` to `Qwen2ForCausalLM`. `tokenizer_config.json` sets `extra_special_tokens` to a JSON list. `vocab.json` and `merges.txt` are absent. `gguf-convert` returns `refuse:tokenizer` and does not print `python3 convert_hf_to_gguf.py`. The script then replaces that fixture with `config.json` `{}` plus `model.safetensors`. That safetensors name does not start with `adapter_model`. A merged directory or a GGUF passed as `--adapter` stays `refuse:adapter`.

The good stubs under the prepared directory are:

| Stub | What the printer needs |
| --- | --- |
| `adapter/adapter_config.json` and `adapter/adapter_model.safetensors` | `merge-adapt` on `unsloth-qlora` reads both. The operator chooses this directory. This journey uses `adapter` beside the prepare. |
| `merged/config.json` and `merged/model.safetensors` | A merged Hugging Face directory is `config.json` plus a `.safetensors` file whose name does not start with `adapter_model`. `gguf-convert` prints for that directory. |
| `merged.gguf` | The first four bytes are `GGUF`. `local-seat` refuses a file that does not start with that magic. `import-trained` records the file. |

`merged.gguf` stays beside `merged/`. A `.gguf` inside the merged directory makes that directory match two shapes.

The printed lines for the overnight pack (`cell-enrich-overnight-traces`) are:

```bash
model.save_pretrained_merged("<prepared>/merged", tokenizer, save_method = "merged_16bit")
python3 convert_hf_to_gguf.py <prepared>/merged --outfile <prepared>/merged.gguf --outtype auto
ollama create cell-enrich-overnight-traces -f <prepared>/Modelfile
estate enrich import-trained --estate <your-estate.yaml> --prepared <prepared> --tag cell-enrich-overnight-traces --adapter <prepared>/merged.gguf
```

`gguf-convert` also prints the three manual lines from the saving-to-gguf page (`f16`, `bf16`, `q8_0`). Unsloth's page does not publish `--outtype auto`. `local-seat` is print-only. It prints the Modelfile and does not write `<prepared>/Modelfile`. Write that file from the printed contents before `ollama create`. The report says this factory did not run `ollama create`. The proposal stays `auto_apply=false`. `import-trained` records `trained_shape` `gguf`. It does not apply the estate.

The script prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` and `CELL_TRAIN_LIVE=1` do not start a convert or an `ollama create`. `READY_FOR_LIVE_TEST`: no.

```bash
make unsloth-qlora-journey
```

That opt-in script runs the prepare asserts and the seat prints in one process. `UNSLOTH_QLORA_PHASE=prepare` stops after the handoff check and the missing-path and wrong-shape refuses. `UNSLOTH_QLORA_PHASE=seat` prints the fixture ladder.

```bash
make uniqueness-unsloth
```

That opt-in script chains the prepare-assert phase, then the seat-print phase. If the prepare phase fails, it exits nonzero before the seat print. It does not run `make qlora-journey`, `make seat-journey`, `make train-next`, or `make axolotl-qlora-journey`. It leaves `examples/estate.yaml` unchanged. It is not in `make smoke`, `make gate-90`, or GitHub Actions.

## 13. Axolotl LoRA — popular-config print journey

This is the print-only check for the bf16 Axolotl card. Axolotl already trains from `examples/llama-3/lora-1b.yml`. This factory writes `axolotl.yml` and prints the next command. It does not install Axolotl, does not run `axolotl`, does not merge, does not run `convert_hf_to_gguf.py`, does not run `ollama create`, and does not promote.

The prepare uses the overnight pack `examples/fixtures/specialist-overnight.pack.json` on a throwaway copy of `examples/estate.yaml`. The seat tag is `llama3`. The train base is `Qwen/Qwen2.5-0.5B-Instruct`. `axolotl.yml` sets `base_model` to that train base. The documented card is `adapter: lora`, `load_in_8bit: false`, `load_in_4bit: false`, `sequence_len: 2048`, `micro_batch_size: 2`, `gradient_accumulation_steps: 2`, and `lora_r: 16`, matching Axolotl `examples/llama-3/lora-1b.yml`. `examples/estate.yaml` stays hash-locked.

A seat tag with no train base is `refuse:train-base` and writes nothing. Before the stubs exist, `merge-adapt` on a missing `outputs/` is `refuse:adapter`. `gguf-convert` on a missing `outputs/merged` is `refuse:seat`. `local-seat` on a missing `outputs/merged.gguf` is `refuse:seat`. Those refuses write no merged directory and no GGUF.

`merge-adapt` prints `axolotl merge-lora` without `--dequant`. That flag stays on the `axolotl-qlora` card. After the print, the script writes a Qwen-shaped directory under `outputs/merged`. `config.json` sets `model_type` to `qwen2` and `architectures` to `Qwen2ForCausalLM`. `tokenizer_config.json` sets `extra_special_tokens` to a JSON list. `vocab.json` and `merges.txt` are absent. `gguf-convert` returns `refuse:tokenizer` and does not print `python3 convert_hf_to_gguf.py`. The script then replaces that fixture with `config.json` `{}` plus `model.safetensors`. That safetensors name does not start with `adapter_model`.

The good stubs are the same files section 11 names: `outputs/adapter_config.json`, `outputs/merged/config.json` plus `outputs/merged/model.safetensors`, and `outputs/merged.gguf` whose first four bytes are `GGUF`.

The printed lines for the overnight pack (`cell-enrich-overnight-traces`) are:

```bash
axolotl merge-lora <prepared>/axolotl.yml --lora-model-dir=<prepared>/outputs
python3 convert_hf_to_gguf.py <prepared>/outputs/merged --outfile <prepared>/outputs/merged.gguf --outtype auto
ollama create cell-enrich-overnight-traces -f <prepared>/outputs/Modelfile
estate enrich import-trained --estate <your-estate.yaml> --prepared <prepared> --tag cell-enrich-overnight-traces --adapter <prepared>/outputs/merged.gguf
```

`local-seat` is print-only. It prints the Modelfile and does not write `<prepared>/outputs/Modelfile`. Write that file from the printed contents before `ollama create`. The report says this factory did not run `ollama create`. The proposal stays `auto_apply=false`. `import-trained` records `trained_shape` `gguf`. It does not apply the estate.

The script prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` and `CELL_TRAIN_LIVE=1` do not start a convert or an `ollama create`. `READY_FOR_LIVE_TEST`: no.

```bash
make axolotl-lora-journey
```

That opt-in script runs the prepare asserts and the seat prints in one process. `AXOLOTL_LORA_PHASE=prepare` stops after the card check and the missing-path refuses. `AXOLOTL_LORA_PHASE=seat` prints the fixture ladder.

```bash
make uniqueness-axolotl-lora
```

That opt-in script chains the prepare-assert phase, then the seat-print phase. If the prepare phase fails, it exits nonzero before the seat print. It does not run `make axolotl-qlora-journey`, `make uniqueness-axolotl`, `make qlora-journey`, `make seat-journey`, or `make train-next`. It leaves `examples/estate.yaml` unchanged. It is not in `make smoke`, `make gate-90`, or GitHub Actions. It does not invent a live PASS.

## 14. Unsloth LoRA — optional NEXT print journey

This is the print-only check for the optional Nvidia-only non-quant LoRA handoff. The card is `unsloth-lora`, the twin of `unsloth-qlora`. It writes `UNSLOTH.md`. It does not write a script, a recipe, `dataset.jsonl`, or `load_in_4bit`. It does not call Unsloth, does not merge, does not run `convert_hf_to_gguf.py`, does not run `ollama create`, and does not promote. Status stays `optional`. `--official-scale` on this card alone is `refuse:official-scale`. `--from-feed` on this card alone is `refuse:dataset`.

The prepare uses the overnight pack `examples/fixtures/specialist-overnight.pack.json` on a throwaway copy of `examples/estate.yaml`. The seat tag is `llama3`. The train base is `Qwen/Qwen2.5-0.5B-Instruct`. `UNSLOTH.md` records both. `prepare.json` keeps them split. `examples/estate.yaml` stays hash-locked.

The refuse ladder and the fixture stubs match section 12: missing adapter, an adapter directory without `adapter_model.safetensors`, and `adapters.safetensors` are `refuse:adapter`. `local-seat --adapter` stays `refuse:adapter`. A missing merged directory or GGUF is `refuse:seat`. A Qwen-shaped merged directory is `refuse:tokenizer` before the good stub. `merge-adapt` prints `save_pretrained_merged` with `save_method` `merged_16bit`, and the documented LoRA save (`save_method` `lora`). `gguf-convert`, `local-seat`, and `import-trained` then print against `merged.gguf`. `import-trained` records `trained_shape` `gguf`. The proposal stays `auto_apply=false`.

The script prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` and `CELL_TRAIN_LIVE=1` do not start a convert or an `ollama create`. `READY_FOR_LIVE_TEST`: no.

```bash
make unsloth-lora-journey
```

`UNSLOTH_LORA_PHASE=prepare` stops after the handoff check and the refuses. `UNSLOTH_LORA_PHASE=seat` prints the fixture ladder.

```bash
make uniqueness-unsloth-lora
```

That opt-in script chains the prepare-assert phase, then the seat-print phase. If the prepare phase fails, it exits nonzero before the seat print. It does not run `make unsloth-qlora-journey`, `make uniqueness-unsloth`, `make axolotl-lora-journey`, `make uniqueness-axolotl-lora`, `make qlora-journey`, `make seat-journey`, or `make train-next`. It leaves `examples/estate.yaml` unchanged. It is not in `make smoke`, `make gate-90`, or GitHub Actions.

## 15. Purpose-build on demand — operator checklist

`make purpose-build-checklist` is the print-only card for purpose-building an SLM when one fits, including mid-software-build. It prints the prepare → train → merge → seat → import loop in operator order and points at the print-only journeys already on tip. It does not run those journeys. It does not train, convert, shell out to ollama, promote, or apply the estate. It does not invent a live PASS. The recorded 5090-class Target C uniqueness PASS in [`LIVE-PROBES.md`](LIVE-PROBES.md) stays the only live uniqueness prove. The re-prove card stays `make uniqueness-prove-checklist`. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. It is not native MLX. It is not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no. `examples/estate.yaml` stays hash-locked (`43770130 3391`).

1. Choose and prepare a train card. The host and stack picker is `make purpose-build-pick` (section 17). This checklist does not run it. A beachhead row is `make lf-beachhead-prepare` (`SKIP live train`). The QLoRA path is `make qlora-journey`. The LoRA path is `make lora-journey`. Optional paths are `make axolotl-qlora-journey`, `make axolotl-lora-journey`, `make unsloth-qlora-journey`, and `make unsloth-lora-journey`. The Apple Silicon print pointer is `make mlx-lm-lora-journey` (section 16). This checklist does not run it. Not native MLX. Prepare against a throwaway copy.
2. Train handoff, train-next style. Print the `NEXT.md` recipe. `SKIP live train`. Print checks: `make train-next` or `make train-next-lora`.
3. Merge and export print honesty: `estate enrich merge-adapt`. The checklist does not merge and does not export.
4. `estate enrich gguf-convert`. The checklist does not convert.
5. `estate enrich local-seat`. local-seat is print-only. It prints the Modelfile and does not write `$PREPARED/Modelfile`. Write that file from the printed contents before `ollama create`. The checklist does not write it and does not shell out to ollama.
6. `estate enrich import-trained`. `trained_shape` `gguf`. The proposal stays `auto_apply=false`. The checklist does not import and does not promote.
7. Standing next (estate). The proposal stays `auto_apply=false`. The factory does not apply the estate without an explicit operator `--require-plan` path. The curator path is `packs accept --curator jason`. No promote and no auto-promote. The coda names `apply-proposal`, `plan`, `apply --require-plan`, and `reconcile` and does not execute them. `examples/estate.yaml` stays unchanged unless the operator deliberately applies a plan.

```bash
make purpose-build-checklist
```

## 16. mlx-lm LoRA — optional Apple Silicon print journey

This is the print-only check for the optional Apple Silicon LoRA handoff. The card is `mlx-lm-lora`. It writes `MLX.md` when `host_class_affinity` is `apple-silicon`. Another affinity is `refuse:host` and writes nothing. It does not write a script, a recipe, or `dataset.jsonl`. It does not call mlx-lm, does not fuse, does not run `convert_hf_to_gguf.py`, does not run `ollama create`, and does not promote. Status stays `optional`. The `mlx` runtime card stays a stub. This journey is not native MLX. `--official-scale` on this card alone is `refuse:official-scale`. `--from-feed` on this card alone is `refuse:dataset`.

The prepare uses a throwaway copy of the overnight pack `examples/fixtures/specialist-overnight.pack.json` with `host_class_affinity` set to `apple-silicon`, on a throwaway copy of `examples/estate.yaml`. The stock pack stays `any` and is `refuse:host`. The seat tag is `llama3`. The train base is `Qwen/Qwen2.5-0.5B-Instruct`. `MLX.md` records both. `prepare.json` keeps them split and records `host_class_affinity` `apple-silicon`. `examples/estate.yaml` stays hash-locked.

A seat tag with no train base is `refuse:train-base` and writes nothing. Before the stubs exist, `merge-adapt` on a missing `adapters` directory is `refuse:adapter`. An adapter directory that holds `adapter_config.json` and a checkpoint `*_adapters.safetensors`, and no `adapters.safetensors`, is `refuse:adapter`. `adapter_model.safetensors` is the same refuse. `gguf-convert` on a missing `fused_model` directory is `refuse:seat`. `local-seat` on a missing `fused_model/ggml-model-f16.gguf` is `refuse:seat`. Those refuses write no fused directory and no GGUF. `gguf-convert` stays `refuse:seat` on this card. The documented GGUF path is `mlx_lm.fuse --export-gguf`.

After the adapter stub exists (`adapter_config.json` plus `adapters.safetensors`), `local-seat --adapter` stays `refuse:adapter`. Passing that directory to `--weights` is `refuse:seat`. `merge-adapt` then prints `mlx_lm.fuse` with `--model` set to the train base, `--adapter-path`, `--save-path` `fused_model` beside the prepare, and `--export-gguf`. The GGUF name is `ggml-model-f16.gguf` inside that directory. A fused MLX directory (`config.json` plus `model.safetensors`) passed to `merge-adapt`, `local-seat`, `gguf-convert`, or `import-trained` refuses. It does not record `trained_shape` `merged`. An empty GGUF is `refuse:seat`. A GGUF file passed to `gguf-convert` is `refuse:seat`. A GGUF file passed as `--adapter` is `refuse:adapter`.

The good stubs under the prepared directory are:

| Stub | What the printer needs |
| --- | --- |
| `adapters/adapter_config.json` and `adapters/adapters.safetensors` | `merge-adapt` on `mlx-lm-lora` reads both. `adapter_model.safetensors` is the wrong shape. |
| `fused_model/ggml-model-f16.gguf` | The first four bytes are `GGUF`. `local-seat` refuses a file that does not start with that magic. The directory must not also hold fused MLX weights when you pass the file. `import-trained` records the file. |

The printed lines for the overnight pack (`cell-enrich-overnight-traces`) are:

```bash
mlx_lm.fuse --model Qwen/Qwen2.5-0.5B-Instruct --adapter-path <prepared>/adapters --save-path <prepared>/fused_model
mlx_lm.fuse --model Qwen/Qwen2.5-0.5B-Instruct --adapter-path <prepared>/adapters --save-path <prepared>/fused_model --export-gguf
ollama create cell-enrich-overnight-traces -f <prepared>/fused_model/Modelfile
estate enrich import-trained --estate <your-estate.yaml> --prepared <prepared> --tag cell-enrich-overnight-traces --adapter <prepared>/fused_model/ggml-model-f16.gguf
```

`local-seat` is print-only. It prints the Modelfile and does not write `<prepared>/fused_model/Modelfile`. Write that file from the printed contents before `ollama create`. The report says this factory did not run `ollama create`. The proposal stays `auto_apply=false`. `import-trained` records `trained_shape` `gguf`. It does not apply the estate.

The script prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` and `CELL_TRAIN_LIVE=1` do not start a fuse or an `ollama create`. `READY_FOR_LIVE_TEST`: no. This path does not invent a live PASS.

```bash
make mlx-lm-lora-journey
```

That opt-in script runs the prepare asserts and the seat prints in one process. `MLX_LM_LORA_PHASE=prepare` stops after the handoff check and the missing-path and wrong-shape refuses. `MLX_LM_LORA_PHASE=seat` prints the fixture ladder.

```bash
make uniqueness-mlx
```

That opt-in script chains the prepare-assert phase, then the seat-print phase. If the prepare phase fails, it exits nonzero before the seat print. It does not run `make unsloth-qlora-journey`, `make uniqueness-unsloth`, `make qlora-journey`, `make seat-journey`, `make train-next`, or `make axolotl-qlora-journey`. It leaves `examples/estate.yaml` unchanged. It is not in `make smoke`, `make gate-90`, or GitHub Actions.

## 17. Purpose-build pick — host and stack table

`make purpose-build-pick` prints which journey to take for which host. It names make targets already on tip and does not run them. It does not resolve or execute estate. It does not train, fuse, convert, shell out to ollama, promote, or apply the estate. It does not invent a live PASS. The recorded 5090-class Target C uniqueness PASS in [`LIVE-PROBES.md`](LIVE-PROBES.md) stays the only live uniqueness prove. The ordered steps after the pick stay `make purpose-build-checklist` (section 15). The print-only chain that runs the pick, then the checklist, is `make purpose-build-journey` (section 18). This picker does not run that chain. The re-prove card stays `make uniqueness-prove-checklist`. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. It is not native MLX. It is not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no. `examples/estate.yaml` stays hash-locked (`43770130 3391`).

| Host / stack | Role | Make target (not executed) |
| --- | --- | --- |
| Nvidia / CUDA | primary | `make lf-beachhead-prepare` |
| Nvidia / CUDA | primary | `make qlora-journey` |
| Nvidia / CUDA | primary | `make uniqueness-full` |
| Nvidia / CUDA | optional | `make unsloth-qlora-journey` and `make uniqueness-unsloth` |
| Nvidia / CUDA | integration | `make axolotl-qlora-journey` and `make uniqueness-axolotl` |
| Apple Silicon | optional | `make mlx-lm-lora-journey` and `make uniqueness-mlx` (section 16) |
| Apple Silicon | refuse:host | stock any-affinity packs. `mlx-lm-lora` writes nothing |
| Target A LoRA twin | primary | `make lora-journey` and `make uniqueness-full-lora` |
| Target A LoRA twin | optional | `make unsloth-lora-journey` and `make uniqueness-unsloth-lora` |
| Target A LoRA twin | integration | `make axolotl-lora-journey` and `make uniqueness-axolotl-lora` |

Unsloth stays optional. Axolotl stays integration. The Apple Silicon card stays optional. A stock pack whose `host_class_affinity` is `any` is `refuse:host` for `mlx-lm-lora`.

```bash
make purpose-build-pick
```

## 18. Purpose-build journey — pick then checklist

`make purpose-build-journey` is the print-only purpose-build on-demand entry. It runs `make purpose-build-pick` (section 17), then `make purpose-build-checklist` (section 15). It calls those targets through make. It does not inline their bodies. It does not resolve or execute estate beyond what those targets already do. It does not train, fuse, convert, shell out to ollama, promote, or apply the estate. It does not invent a live PASS. The recorded 5090-class Target C uniqueness PASS in [`LIVE-PROBES.md`](LIVE-PROBES.md) stays the only live uniqueness prove. The re-prove card stays `make uniqueness-prove-checklist`. This journey does not run it. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. It is not native MLX. It is not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no. `examples/estate.yaml` stays hash-locked (`43770130 3391`).

If the pick fails, the script exits nonzero before the checklist.

```bash
make purpose-build-journey
```

## 19. DeepSeek-R1-Distill chat — LLaMA-Factory print journey

`make deepseek-r1-distill-journey` is the print-only DeepSeek-R1-Distill chat QLoRA ladder on the card that already exists, `llamafactory-qlora`. It uses `examples/fixtures/deepseek-r1-distill.pack.json` on a throwaway copy of `examples/estate.yaml`. The seat tag is `llama3`. The train base is `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`. `template` is `deepseekr1`. An Ollama tag `deepseek-r1:1.5b` stays a seat tag. The recipe keeps `quantization_method: bnb`, `quantization_bit: 4`, `lora_rank` 16, and `packing` true. A seat tag with no train base is `refuse:train-base`. A missing adapter is `refuse:adapter`. A missing merged export or GGUF is `refuse:seat`. Before the good stub, a 5090-shaped export is `refuse:tokenizer`. `merge-adapt` prints `llamafactory-cli export`. `gguf-convert` prints `convert_hf_to_gguf.py`. `local-seat` prints `ollama create` and does not write a Modelfile. `import-trained` records `trained_shape` `gguf` and the proposal stays `auto_apply=false`. It does not train, merge, convert, shell out to ollama, or promote.

`make uniqueness-deepseek` runs the prepare-assert phase, then the seat-print phase, of that QLoRA journey. It does not run `make qlora-journey`, `make uniqueness-full`, or `make lf-beachhead-prepare`.

`make deepseek-r1-distill-lora-journey` is the non-quant twin on `llamafactory-lora` and `examples/fixtures/deepseek-r1-distill-lora.pack.json`. The same seat tag and the same train base. `template` stays `deepseekr1`. The recipe omits `quantization_bit` and `quantization_method`, keeps `lora_rank` 8 and `packing` false, and names the LoRA reproduce line. It does not run `make deepseek-r1-distill-journey`. `make uniqueness-deepseek-lora` runs the prepare-assert phase, then the seat-print phase, of that LoRA journey. It does not run `make uniqueness-deepseek`.

`CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. This path does not invent a live PASS. The recorded 5090-class Target C uniqueness PASS in [`LIVE-PROBES.md`](LIVE-PROBES.md) stays the only live uniqueness prove. These targets are not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no. `examples/estate.yaml` stays hash-locked (`43770130 3391`).

```bash
make deepseek-r1-distill-journey
make uniqueness-deepseek
make deepseek-r1-distill-lora-journey
make uniqueness-deepseek-lora
```
