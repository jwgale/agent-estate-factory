# Train/enrich prepare

Operator page for the first durable train/enrich beachhead. Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). Product story: [`NORTH-STAR.md`](NORTH-STAR.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md).

`estate enrich prepare` writes artifacts for a purpose-built SLM. It does not train. It does not POST. It does not rewrite `estate.yaml`. Promote stays off.

The LLaMA-Factory LoRA and QLoRA reproduce targets are one table: [`lf-beachhead-matrix.md`](lf-beachhead-matrix.md). `estate help enrich` and `estate help train` print that file. Each row is a smoke prepare on the existing ladder (prepare, the `NEXT.md` train and export lines, `merge-adapt`, `gguf-convert`, `local-seat`, `import-trained`). The table does not add a train family. A bare Ollama seat tag on those train bases is `refuse:train-base`. Opt-in check: `make lf-beachhead-prepare`. It prepares every row on a throwaway copy of `examples/estate.yaml`, checks the row knobs, and prints `SKIP live train`. It does not train, merge, convert, seat, or promote. Phi-3-small stays QLoRA-only and is not a row. `READY_FOR_LIVE_TEST`: no.

## Target C — Qwen QLoRA operator journey

The popular path is one ladder of commands that already exist. LLaMA-Factory trains. llama.cpp converts. Ollama creates. This factory writes the QLoRA recipe and prints the next line. Walk: section 8 of [`operator-enrich-journeys.md`](operator-enrich-journeys.md). `estate help enrich` prints the same ladder. Opt-in check: `make qlora-journey`. `make train-next` is the opt-in middle step: it prepares the same Target C card and prints the `NEXT.md` train recipe. It does not train. Once a merged export and a GGUF exist, `make seat-journey` prints `merge-adapt`, `gguf-convert`, `local-seat`, and `import-trained` against fixture stubs. Walk: section 10 of that same page. `make uniqueness-ladder` runs the qlora and seat print journeys in that order. It does not run `make train-next`. It does not train. It is not in smoke or Actions. It is not a live train. `make uniqueness-full` runs `make qlora-journey`, then `make train-next`, then `make seat-journey`. It does not train. It is not in smoke or Actions. It is not a live train.

The seat tag and the train base stay separate. A 5090 smoke seated `llama3` and trained `Qwen/Qwen2.5-0.5B-Instruct`. `template` for that name is `qwen`. `recipe.yaml` sets `model_name_or_path` to the train base, `quantization_bit: 4`, and `quantization_method: bnb`. A missing train base or a bare seat tag is `refuse:train-base` and writes nothing. This factory does not map the seat tag onto a Hub repo and does not download weights. `<your-estate.yaml>` is a lab copy. `examples/estate.yaml` on `main` stays hash-locked.

1. Prepare the QLoRA card.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id> \
  --driver llamafactory-qlora \
  --job train \
  --state-dir .cell
```

2. On a CUDA host, run the train and export lines from `NEXT.md`. This factory does not run them.

```bash
pip install llamafactory
pip install 'bitsandbytes>=0.49'
llamafactory-cli train .cell/enrich/<pack-id>/llamafactory-qlora/recipe.yaml
llamafactory-cli export .cell/enrich/<pack-id>/llamafactory-qlora/export.yaml
```

3. Print the llama.cpp convert. The command does not convert.

```bash
estate enrich gguf-convert \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export
```

That prints `python3 convert_hf_to_gguf.py` on the merged directory, with `--outfile` set to the sibling `export.gguf` and `--outtype auto`. Run that line from a llama.cpp checkout. A missing export directory is `refuse:seat`.

4. Print the Ollama create. When `--weights` is the GGUF, the same report also prints `llama-cli -m` and `llama-server -m` for that file. `--runtime llama.cpp` selects those lines. Ollama stays the default print. A merged directory still points at `gguf-convert` first. The command does not create the model and does not run llama.cpp. After that GGUF print, run the printed `ollama create` line yourself. The same step stands when you already ran `ollama create` outside this factory. This factory did not run `ollama create`. The standing next step records that GGUF.

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export.gguf
```

To seat the adapter in `outputs/` without a merge, pass `--adapter` instead of `--weights`. The command prints a Modelfile whose `FROM` is `prepare.json` `seat_tag` and whose `ADAPTER` is that directory, then the `ollama create` line. It does not run the line. llama.cpp does not load that directory in one line. `--runtime llama.cpp` with `--adapter` is `refuse:runtime`. `--weights` still refuses that adapter directory (`refuse:seat`). A merged export or a GGUF passed to `--adapter` is `refuse:adapter`.

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --adapter .cell/enrich/<pack-id>/llamafactory-qlora/outputs
```

5. Record the artifact shape. That is the standing next step after the GGUF `local-seat` print. `import-trained` writes `trained_shape` and `trained_paths`. After the GGUF exists, `trained_shape` is `gguf`. The proposal stays `auto_apply=false`. `import-trained` does not apply the estate and does not promote. `outputs/` records `adapter`. `export/` records `merged`.

```bash
estate enrich import-trained \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --tag cell-enrich-<pack-id> \
  --adapter .cell/enrich/<pack-id>/llamafactory-qlora/export.gguf
```

`apply-proposal`, `estate plan`, and `estate apply --require-plan` stay the join. `READY_FOR_LIVE_TEST`: no.

## Target A — Qwen LoRA operator journey

The unquantized path is one ladder of commands that already exist. LLaMA-Factory trains. llama.cpp converts. Ollama creates. This factory writes the LoRA recipe and prints the next line. Walk: section 9 of [`operator-enrich-journeys.md`](operator-enrich-journeys.md). `estate help enrich` prints the same ladder. Opt-in check: `make lora-journey`.

The seat tag and the train base stay separate. A 5090 smoke seated `llama3` and trained `Qwen/Qwen2.5-0.5B-Instruct` on `llamafactory-lora`, then exported and seated that gauge from the files prepare wrote. `template` for that name is `qwen`. `recipe.yaml` sets `model_name_or_path` to the train base, `finetuning_type: lora`, `lora_rank: 8`, and `packing: false`. It omits `quantization_bit` and `quantization_method`. This path does not require bitsandbytes. A missing train base or a bare seat tag is `refuse:train-base` and writes nothing. This factory does not map the seat tag onto a Hub repo and does not download weights. `<your-estate.yaml>` is a lab copy. `examples/estate.yaml` on `main` stays hash-locked.

1. Prepare the LoRA card.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id> \
  --driver llamafactory-lora \
  --job train \
  --state-dir .cell
```

2. On a CUDA host, run the train and export lines from `NEXT.md`. This factory does not run them.

```bash
pip install llamafactory
llamafactory-cli train .cell/enrich/<pack-id>/llamafactory-lora/recipe.yaml
llamafactory-cli export .cell/enrich/<pack-id>/llamafactory-lora/export.yaml
```

`estate enrich merge-adapt` prints that same export line for `outputs/`. It does not merge.

```bash
estate enrich merge-adapt \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --adapter .cell/enrich/<pack-id>/llamafactory-lora/outputs
```

3. Print the llama.cpp convert. The command does not convert.

```bash
estate enrich gguf-convert \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --weights .cell/enrich/<pack-id>/llamafactory-lora/export
```

That prints `python3 convert_hf_to_gguf.py` on the merged directory, with `--outfile` set to the sibling `export.gguf` and `--outtype auto`. Run that line from a llama.cpp checkout. A missing export directory is `refuse:seat`.

4. Print the Ollama create. When `--weights` is the GGUF, the same report also prints `llama-cli -m` and `llama-server -m` for that file. `--runtime llama.cpp` selects those lines. Ollama stays the default print. A merged directory still points at `gguf-convert` first. The command does not create the model and does not run llama.cpp. After that GGUF print, run the printed `ollama create` line yourself. The same step stands when you already ran `ollama create` outside this factory. This factory did not run `ollama create`. The standing next step records that GGUF.

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --weights .cell/enrich/<pack-id>/llamafactory-lora/export.gguf
```

To seat the adapter in `outputs/` without a merge, pass `--adapter` instead of `--weights`. The command prints a Modelfile whose `FROM` is `prepare.json` `seat_tag` and whose `ADAPTER` is that directory, then the `ollama create` line. It does not run the line. llama.cpp does not load that directory in one line. `--runtime llama.cpp` with `--adapter` is `refuse:runtime`. `--weights` still refuses that adapter directory (`refuse:seat`). A merged export or a GGUF passed to `--adapter` is `refuse:adapter`.

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --adapter .cell/enrich/<pack-id>/llamafactory-lora/outputs
```

5. Record the artifact shape. That is the standing next step after the GGUF `local-seat` print. `import-trained` writes `trained_shape` and `trained_paths`. After the GGUF exists, `trained_shape` is `gguf`. The proposal stays `auto_apply=false`. `import-trained` does not apply the estate and does not promote. `outputs/` records `adapter`. `export/` records `merged`.

```bash
estate enrich import-trained \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --tag cell-enrich-<pack-id> \
  --adapter .cell/enrich/<pack-id>/llamafactory-lora/export.gguf
```

`apply-proposal`, `estate plan`, and `estate apply --require-plan` stay the join. `READY_FOR_LIVE_TEST`: no.

## Target C seat ladder — fixture print path

`make qlora-journey` leaves `gguf-convert` and `local-seat` at `refuse:seat` because the merged export and the GGUF are not there yet. `make seat-journey` is the opt-in that asserts `refuse:tokenizer` on a 5090-shaped export, then prints the happy path once the good fixture stubs stand in for those files. Walk: section 10 of [`operator-enrich-journeys.md`](operator-enrich-journeys.md). `estate help enrich` names the same check.

The prepare is the Target C card above: `llamafactory-qlora`, seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`. The script uses `examples/fixtures/specialist-overnight.pack.json` and a throwaway copy of `examples/estate.yaml`. It does not rewrite the hash-locked example.

Before the stubs exist, a missing `outputs/` is `refuse:adapter` from `merge-adapt`, and a missing `export/` or `export.gguf` is `refuse:seat`. The script then writes:

| Stub | Printer |
| --- | --- |
| `outputs/adapter_config.json` | `merge-adapt` prints `llamafactory-cli export` for `export.yaml`. |
| `export/config.json` and `export/model.safetensors` | `gguf-convert` prints `python3 convert_hf_to_gguf.py` with `--outfile` the sibling `export.gguf` and `--outtype auto`. |
| `export.gguf` starting with `GGUF` | `local-seat` prints `ollama create` plus `llama-cli -m` and `llama-server -m`. An empty file is `refuse:seat`. |

`local-seat` on that GGUF names the standing next step: `estate enrich import-trained` for `export.gguf`. The proposal stays `auto_apply=false`. The report says this factory did not run `ollama create`. `import-trained` records `trained_shape` `gguf` and `trained_paths` and does not apply the estate. The script prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` does not run `llamafactory-cli`, `convert_hf_to_gguf.py`, or `ollama create`. Live train, merge, convert, and seat stay on the operator host (section 8). `make enrich-live-prove` still covers a from-pack Modelfile. `READY_FOR_LIVE_TEST`: no. Not in `make smoke`, `make gate-90`, or GitHub Actions.

## What exists today

| Piece | Role |
| --- | --- |
| `TrainEnrichDriver` | Data-plane trait in `model-estate`. `id()`, `prepare(job)`, catalog `status` / `probe`. |
| `ollama-modelfile` | Integration. Writes a Modelfile (`FROM` + `SYSTEM`), `PREPARE.md`, and `NEXT.md` with the exact `ollama create` line. `FROM` is the seated model. Does not shell out. |
| `external-manifest` | Portable JSON and YAML. Base model ref, purpose, host class affinity, dataset path hints from the pack `source_paths`. No vendor lock. `NEXT.md` names the files to hand off. |
| `llamafactory-lora` | LoRA train card. Writes the same files as `llamafactory-qlora`: `recipe.yaml` is SFT LoRA (`finetuning_type: lora`, no `quantization_bit`, no `quantization_method`, LoRA rank 8, `cutoff_len` 512, `packing: false`). `template` comes from the train base (`qwen3_nothink` for a non-thinking Qwen3 instruct name such as `Qwen/Qwen3-4B-Instruct-2507`, `qwen3` for other Qwen3 names). `NEXT.md` and `PREPARE.md` name this card as the non-quant twin of the Qwen3 Instruct QLoRA prepare only when the winning segment is `Qwen/Qwen3-4B-Instruct-2507`, `Qwen/Qwen3-30B-A3B-Instruct-2507`, `Qwen/Qwen3-235B-A22B-Instruct-2507`, or `Qwen/Qwen3-Next-80B-A3B-Instruct`, matching `examples/train_lora/qwen3_lora_sft.yaml`. A thinking id does not get that line. A Qwen2.5 id does not get that Qwen3 LoRA line. The same card names the non-quant twin of the Llama-3.2 Instruct QLoRA prepare only when the winning segment is `Llama-3.2-1B-Instruct` or `Llama-3.2-3B-Instruct`, with template `llama3`. Llama-3.2 vision, a Llama-3.2 base, and Llama-3.1 Instruct do not get that line. They also name this card as the non-quant twin of the Gemma-2 Instruct QLoRA prepare only when the winning segment is `google/gemma-2-2b-it`, `google/gemma-2-9b-it`, or `google/gemma-2-27b-it` (template `gemma2`). A Gemma-2 base, original Gemma, and Gemma-3 do not get that line. They also name this card as the non-quant twin of the Mistral Instruct QLoRA prepare only when the winning segment is `mistralai/Mistral-7B-Instruct-v0.1`, `mistralai/Mistral-7B-Instruct-v0.2`, or `mistralai/Mistral-7B-Instruct-v0.3` (template `mistral`). A Mistral-7B base, Mistral-Small, Mistral-Nemo, Mixtral, and LLaVA-NeXT-Mistral do not get that line. The same card names the non-quant twin of the Phi-3 Instruct QLoRA prepare only when the winning segment is Phi-3 mini, Phi-3 medium, or Phi-3.5 (template `phi`), including `microsoft/Phi-3-mini-4k-instruct`. Phi-3-small stays `phi_small`. Phi-4 stays `phi4`. Phi-4-mini stays `phi4_mini`. Those ids do not get that line. The same card names the non-quant twin of the Qwen2.5 Instruct QLoRA prepare only when the winning segment is `Qwen/Qwen2.5-0.5B-Instruct`, `Qwen/Qwen2.5-1.5B-Instruct`, `Qwen/Qwen2.5-3B-Instruct`, `Qwen/Qwen2.5-7B-Instruct`, `Qwen/Qwen2.5-14B-Instruct`, `Qwen/Qwen2.5-32B-Instruct`, `Qwen/Qwen2.5-72B-Instruct`, `Qwen/Qwen2.5-7B-Instruct-1M`, or `Qwen/Qwen2.5-14B-Instruct-1M` (template `qwen`). A Qwen2.5 base, a thinking-shaped id, Qwen2, Qwen2.5-Coder, Qwen2.5-Math, and Qwen2.5-VL do not get that line. Qwen3 Instruct stays `qwen3_nothink` and does not get that line. `NEXT.md` has `pip install llamafactory`, `llamafactory-cli train`, and `llamafactory-cli export`. This path does not require bitsandbytes. Does not shell out. |
| `llamafactory-qlora` | QLoRA train card. Writes `recipe.yaml` (LLaMA-Factory SFT QLoRA: `quantization_bit: 4`, `quantization_method: bnb`, LoRA rank 16, `cutoff_len` 512, `packing: true`), `export.yaml`, `dataset_info.json`, and instruct chat `dataset.jsonl`. `model_name_or_path` is the train base. The Ollama seat tag stays separate. Default job is `train`. `NEXT.md` has `pip install llamafactory`, `pip install 'bitsandbytes>=0.49'`, `llamafactory-cli train`, and `llamafactory-cli export`. Does not shell out. |
| `axolotl-lora` | bf16 LoRA YAML for a config-driven or multi-GPU run. Writes `axolotl.yml` (`adapter: lora`, `load_in_8bit: false`, `load_in_4bit: false`, `sequence_len: 2048`, `micro_batch_size: 2`, `gradient_accumulation_steps: 2`, `lora_r: 16`) and Alpaca `dataset.jsonl`. Those knobs match Axolotl `examples/llama-3/lora-1b.yml`. `base_model` is the train base. The Ollama seat tag stays in `prepare.json`. Default job is `train`. `NEXT.md` has the exact `axolotl train` line. After train, `estate enrich merge-adapt` prints `axolotl merge-lora`. Axolotl writes `output_dir/merged`. `NEXT.md` then names `gguf-convert`, `local-seat`, and `import-trained`. Axolotl does not write GGUF. Does not shell out. |
| `axolotl-qlora` | 4-bit QLoRA YAML for the same kind of run. Writes `axolotl.yml` (`load_in_8bit: false`, `load_in_4bit: true`, `adapter: qlora`, `sequence_len: 4096`, `micro_batch_size: 2`, `gradient_accumulation_steps: 4`, `lora_r: 32`) and Alpaca `dataset.jsonl`. Those knobs match Axolotl `examples/llama-3/qlora.yml`. `base_model` is the train base. Default job is `train`. `NEXT.md` has the exact `axolotl train` line. After train, `estate enrich merge-adapt` prints `axolotl merge-lora`, including `--dequant`. Axolotl writes `output_dir/merged`. `NEXT.md` then names `gguf-convert`, `local-seat`, and `import-trained`. Axolotl does not write GGUF. Does not shell out. |
| `unsloth-qlora` | Optional NEXT card (`status=optional`). Nvidia-only QLoRA alternate for a faster single-GPU run. Writes `UNSLOTH.md` (operator-owned handoff), `PREPARE.md`, and `NEXT.md`. Records the train base and the Ollama seat tag. Does not write a script, a recipe, or `dataset.jsonl`. Does not call Unsloth. After that train, `merge-adapt` prints `save_pretrained_merged` with `save_method` `merged_16bit` for an adapter directory that holds `adapter_config.json` and `adapter_model.safetensors`. `gguf-convert` and `local-seat` then print the convert and the seat for `merged` beside the prepare. `local-seat --adapter` is `refuse:adapter`. Not the product. |
| `mlx-lm-lora` | Optional NEXT card (`status=optional`). Apple Silicon LoRA handoff. Writes `MLX.md`, `PREPARE.md`, and `NEXT.md` only when `host_class_affinity` is `apple-silicon`. Another affinity is `refuse:host` and writes nothing. Records the train base, the Ollama seat tag, and that host. Does not write a script, a recipe, or `dataset.jsonl`. Does not call mlx-lm. After train, `merge-adapt` prints `mlx_lm.fuse` for an adapter directory that holds `adapter_config.json` and `adapters.safetensors`. `--save-path` is `fused_model` beside the prepare. `--export-gguf` writes `ggml-model-f16.gguf` in that directory. `local-seat` prints the Ollama line for that GGUF file. The fused directory is MLX weights. This card does not print a Hugging Face convert line for it. `import-trained` records the adapter directory or that GGUF file. A fused directory is `refuse:adapter`. Not the product. The `mlx` runtime card stays a stub. |
| `NEXT.md` | Operator card in the output directory. Artifact paths, the handoff command, the `import-prepared` or `import-trained` line, and the fail-closed reminders. |
| `estate enrich drivers` | Prints the catalog. `live=false`. A probe here does not train. |
| `estate enrich prepare --all-drivers` | One call. Each card the job allows writes a sibling directory. A refuse writes none of them. The enrich default skips `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, `unsloth-qlora`, and `mlx-lm-lora`. `--job train` includes them. `mlx-lm-lora` is included only when `host_class_affinity` is `apple-silicon`. On any other affinity the command prints that it omitted the card and still writes the rest. |
| `estate enrich list` | Reads `{state_dir}/enrich/{pack}/{driver}/prepare.json`. Prints pack, driver, job, tag, and out path. Does not create the directory. |
| `estate enrich import-prepared` | Checks `prepare.json` plus the tag and file you created outside the factory. Writes `binding-proposal.json` and `binding-proposal.md` for the existing `local_slm` seat. Does not apply. |
| `estate enrich import-trained` | Same proposal, for a `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, `unsloth-qlora`, or `mlx-lm-lora` prepare whose job is `train`. `--adapter` is an adapter `output_dir` (`adapter_config.json`), a merged `export_dir` (`config.json` and a `.safetensors` file whose name does not start with `adapter_model`, optional `Modelfile`), or one `.gguf` file or a directory with exactly one top-level `.gguf`. On `mlx-lm-lora` that merged-looking directory is fused MLX weights (`config.json` and `model.safetensors`) and is `refuse:adapter`. That card records the adapter directory or a GGUF file. After a GGUF `local-seat` print, this command is the standing next step for that file. Run the printed `ollama create` line yourself. This factory did not run `ollama create`. The proposal stays `auto_apply=false`. Records `trained_shape` and `trained_paths` on the same write as the proposal. Does not apply the estate. |
| `estate enrich merge-adapt` | Print-only adapter merge for an `axolotl-lora`, `axolotl-qlora`, `llamafactory-lora`, or `llamafactory-qlora` train prepare, the documented fuse for an `mlx-lm-lora` train prepare on `apple-silicon`, and Unsloth's `save_pretrained_merged` (`merged_16bit`) for an `unsloth-qlora` train prepare. `--adapter` is an adapter directory (`adapter_config.json`). On `mlx-lm-lora` that directory also holds `adapters.safetensors`. On `unsloth-qlora` it also holds `adapter_model.safetensors` or `adapter_model.bin`. A merged Hugging Face directory or a GGUF is `refuse:adapter`. Axolotl prints `axolotl merge-lora` with `--lora-model-dir`. Axolotl writes `{output_dir}/merged`. This prepare sets `output_dir` to `outputs`, so the directory is `outputs/merged`. `axolotl-qlora` also prints `--dequant`. A LLaMA-Factory adapter prints `llamafactory-cli export` on the prepare's `export.yaml`. The report lists the keys from `examples/merge_lora/qwen3_lora_sft.yaml` (`model_name_or_path`, `adapter_name_or_path`, `template`, `trust_remote_code`, `export_dir`, `export_size`, `export_device`, `export_legacy_format`). LLaMA-Factory writes that `export_dir`. A missing `export.yaml`, a quantized export key, or a train base that does not match `model_name_or_path` refuses. Then those cards print `gguf-convert` and `local-seat` for that `export_dir`. The LLaMA-Factory report also names the tokenizer restore: copy tokenizer files from the HF cache snapshot for the train base already on disk, or the equivalent base checkout, into the export directory, then re-run `estate enrich gguf-convert`, when `extra_special_tokens` is a JSON list or JSON null, or a Qwen export is missing `vocab.json` or `merges.txt`. `gguf-convert` returns `refuse:tokenizer` for that directory. `unsloth-qlora` prints the merged_16bit line for `{prepared}/merged` and does not print `merge_and_unload`. `mlx-lm-lora` prints `mlx_lm.fuse --model <train base> --adapter-path <adapter> --save-path <prepared>/fused_model` and the same line with `--export-gguf`. mlx-lm writes `fused_model/ggml-model-f16.gguf`. The report then names `local-seat` for that file. It does not print a Hugging Face convert line. Does not merge, does not fuse, does not shell out, and does not promote. |
| `estate enrich gguf-convert` | Print-only convert card for a `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, or `unsloth-qlora` train prepare. `--weights` must be a merged Hugging Face directory (`config.json` and a `.safetensors` file whose name does not start with `adapter_model`). Prints `python3 convert_hf_to_gguf.py <dir> --outfile <sibling>.gguf --outtype auto`, then the `local-seat` line for that sibling file. `--outtype auto` is the llama.cpp script default (highest-fidelity 16-bit float). For `unsloth-qlora` the report also prints the three manual lines on the saving-to-gguf page (`f16`, `bf16`, `q8_0`). Unsloth's page does not publish `--outtype auto`. Axolotl does not write GGUF. A JSON list or JSON null under `extra_special_tokens` in `tokenizer_config.json`, or a Qwen-family export missing `vocab.json` or `merges.txt`, is `refuse:tokenizer`. Qwen-family is `config.json` `model_type` or `architectures`, or `tokenizer_class`, naming Qwen. Copy those tokenizer files from the HF cache snapshot for the train base already on disk, or the equivalent base checkout, into the export directory, keep the export `tokenizer_config.json` as `tokenizer_config.json.bak`, then re-run `estate enrich gguf-convert`. Does not convert, does not shell out, does not download tokenizer files, does not copy them, and does not promote. |
| `estate enrich from-pack` | After an accepted pack. Same prepare, into `{state_dir}/enrich/{pack}/{driver}`. Omitting `--driver` prepares every card the job allows. The default job is enrich, so the train cards wait for `--job train`. Does not apply. |
| `estate enrich apply-proposal` | Reads that proposal. Checks schema, curator, sacred, hardware, frontier, and `prepare.json`. Writes `{state}/enrich-stage/staged-estate.yaml` for `estate plan` and `estate apply --require-plan`. Does not apply. Does not rewrite the source estate. |
| `estate help enrich` | Same page as `estate help train`. |
| `make enrich-prepare` | Opt-in fixture walk. Not in `make smoke`, `make gate-90`, or GitHub Actions. |
| `make train-prepare` | Opt-in LLaMA-Factory LoRA and QLoRA recipes, Axolotl LoRA and QLoRA recipes, the optional Unsloth handoff, and the optional mlx-lm handoff. Prints `SKIP live train`. Does not run a trainer. Not in `make smoke`, `make gate-90`, or GitHub Actions. |
| `make qlora-journey` | Opt-in Target C ladder. Prints prepare, the `NEXT.md` train and export lines, `gguf-convert`, `local-seat`, and `import-trained`. Checks a `llamafactory-qlora` prepare whose seat tag is `llama3` and whose train base is `Qwen/Qwen2.5-0.5B-Instruct`. A missing export is `refuse:seat` and writes no GGUF. Prints `SKIP live train`. Does not train, convert, or promote. Not in `make smoke`, `make gate-90`, or GitHub Actions. |
| `make train-next` | Opt-in print-only Target C train step. Prepares `llamafactory-qlora` (seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`) and prints the `NEXT.md` lines `pip install llamafactory`, `pip install 'bitsandbytes>=0.49'`, and `llamafactory-cli train` on that `recipe.yaml`. Does not run them. `CELL_TRAIN_LIVE=1` stays print-only. A missing `llamafactory-cli` or bitsandbytes is an informational SKIP. Does not train, merge, convert, seat, or promote. Not in `make smoke`, `make gate-90`, or GitHub Actions. |
| `make lora-journey` | Opt-in Target A ladder. Prints prepare, the `NEXT.md` train and export lines, `merge-adapt`, `gguf-convert`, `local-seat`, and `import-trained`. Checks a `llamafactory-lora` prepare whose seat tag is `llama3` and whose train base is `Qwen/Qwen2.5-0.5B-Instruct` (`template: qwen`, rank 8, no quantization). A missing export is `refuse:seat` and writes no GGUF. Prints `SKIP live train`. Does not train, merge, convert, or promote. Not in `make smoke`, `make gate-90`, or GitHub Actions. |
| `make seat-journey` | Opt-in Target C seat ladder. Prepares `llamafactory-qlora` (seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`), then prints `merge-adapt`, `gguf-convert`, `local-seat`, and `import-trained` against fixture stubs (`adapter_config.json`, `config.json` plus `model.safetensors`, and a GGUF that starts with GGUF magic). A missing adapter is `refuse:adapter`. A missing export or GGUF is `refuse:seat`. Before the good merged stub, a 5090-shaped export (Qwen `config.json`, list `extra_special_tokens`, missing `vocab.json` and `merges.txt`) is `refuse:tokenizer` and does not print the convert line. The script then replaces that fixture with `config.json` `{}` plus `model.safetensors`. The GGUF `local-seat` report names `import-trained` for `export.gguf` as the standing next step. The proposal stays `auto_apply=false`. `import-trained` records `trained_shape` `gguf`. Prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` does not run a convert or `ollama create`. Does not train, merge, convert, or promote. Not in `make smoke`, `make gate-90`, or GitHub Actions. |
| `make uniqueness-ladder` | Opt-in print-only Target C uniqueness chain. Runs `make qlora-journey`, then `make seat-journey`. Does not run `make train-next`. Does not train, merge, convert, seat, or promote. Does not run `make lf-beachhead-prepare`. Live train, live convert, and live seat still need a human GPU host and stay skipped. Not in `make smoke`, `make gate-90`, or GitHub Actions. Not a live train. |
| `make uniqueness-full` | Opt-in print-only Target C full uniqueness print chain. Runs `make qlora-journey`, then `make train-next`, then `make seat-journey`. If one step fails, the script exits nonzero before the next step. Does not train, merge, convert, seat, or promote. Does not run `make lf-beachhead-prepare`. `make uniqueness-ladder` stays `make qlora-journey` then `make seat-journey` and does not run `make train-next`. Live train, live convert, and live seat still need a human GPU host and stay skipped. Not in `make smoke`, `make gate-90`, or GitHub Actions. Not a live train. |
| `make enrich-live-prove` | Opt-in seated handoff. Runs `ollama create` when the seat is up, then removes the tag. Not a factory-wide live test. Off smoke, `gate-90`, and Actions. |

Job field: `enrich` (default) or `train`. `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, `unsloth-qlora`, and `mlx-lm-lora` default to `train` and refuse `enrich`. Both jobs only prepare. The factory does not run a trainer.

Default output is `.cell/enrich/{pack_id}/{driver}/`. Pass `--out` to write somewhere else, including `packs/prepared/`. Layout: [`cell-layout.md`](cell-layout.md). Schema: [`../schema/train-enrich.v0.json`](../schema/train-enrich.v0.json) (`cell-one.enrich-prepare.v0`).

## Facilitated vs invented

Ollama already creates a model from a Modelfile. This factory writes that file and the next command. It does not invent a local inference server.

LLaMA-Factory already runs LoRA and QLoRA supervised fine-tuning from a YAML recipe. `llamafactory-lora` writes the unquantized LoRA recipe (`examples/train_lora/qwen3_lora_sft.yaml` shape: `finetuning_type: lora`, no quantization). `llamafactory-qlora` writes the 4-bit bitsandbytes recipe. Axolotl already trains from a YAML recipe, including multi-GPU runs. `axolotl-lora` writes the bf16 LoRA YAML that matches `examples/llama-3/lora-1b.yml`. `axolotl-qlora` writes the 4-bit YAML that matches `examples/llama-3/qlora.yml`. `NEXT.md` names `axolotl train`, then `estate enrich merge-adapt` (Axolotl's `axolotl merge-lora` into `output_dir/merged`), `gguf-convert`, `local-seat`, and `import-trained`. None of these cards shell out. `unsloth-qlora` is the optional Nvidia-only QLoRA handoff for a faster single-GPU run. It writes `UNSLOTH.md` and does not write a script. After that train, `merge-adapt` prints Unsloth's documented `merged_16bit` save. `mlx-lm-lora` is the optional Apple Silicon LoRA handoff. It writes `MLX.md` and does not write a script. mlx-lm already documents LoRA and fuse; this card points at that page. `--from-feed` copies instruct rows that are already under the cell state directory onto the recipe cards. This factory does not invent an in-process trainer, a dataset downloader, a GGUF exporter, or a GPU scheduler. `estate enrich gguf-convert` prints the llama.cpp `convert_hf_to_gguf.py` line for a merged export. It does not run that script.

## Train / fine-tune

Supported train hosts for `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, and `unsloth-qlora` are `consumer-nvidia` and `rented-nvidia`. `host_class_affinity` comes from the pack when that field is set, otherwise from `params.host_class` on `local_slm`, otherwise from the pack `host_class`. An `apple-silicon` affinity still prepares those cards. `NEXT.md` says the LLaMA-Factory card expects CUDA LLaMA-Factory. Those cards do not write an MLX trainer. `mlx-lm-lora` prepares only when that affinity is the canonical name `apple-silicon`. `any`, `consumer-nvidia`, `rented-nvidia`, and an alias such as `apple_silicon` are `refuse:host` on that card and write nothing.

The seat tag and the train base are two fields. `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, `unsloth-qlora`, and `mlx-lm-lora` use that split.

The seat tag is the Ollama id for Modelfile `FROM`. Resolution: a pack `model_hint` that is already a model tag, otherwise `params.model` on the local binding. `prepare.json` stores it as `base_model` and `seat_tag`. The binding id `local_slm` is `refuse:base-model` and writes nothing.

The train base is `model_name_or_path` in `recipe.yaml` and `export.yaml`, and `base_model` in `axolotl.yml`. It is a Hugging Face repo id (`namespace/name`) or a local directory of HF weights (an absolute path, or a path that starts with `./` or `../`). Set it on the pack as `train_base_model`, or on the local binding as `params.train_base_model`. The pack field wins when both are set. A relative directory is stored as an absolute path in `recipe.yaml`, `export.yaml`, `axolotl.yml`, `prepare.json`, and `NEXT.md`. LLaMA-Factory resolves a relative `model_name_or_path` from the process working directory, and Axolotl resolves a relative `base_model` the same way, so both recipes keep the absolute path. The directory does not need to exist at prepare time. `prepare.json` stores the resolved value as `train_base_model`. On the LLaMA-Factory cards, `template` is inferred by scanning path segments of that train base, starting at the last segment. A leaf such as `weights` or an HF snapshot hash uses the nearest ancestor that names a family. A name containing `qwen3` and `instruct` and not `thinking`, or a name containing `nothink`, uses `qwen3_nothink` (`Qwen/Qwen3-4B-Instruct-2507` is the LoRA quickstart). Other Qwen3 names, including `Qwen/Qwen3-4B` and `Thinking-2507`, use `qwen3`. Older Qwen names use `qwen`. A DeepSeek segment is classified before a Qwen or Llama substring in that same segment. `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B` uses `deepseekr1`. A GLM-4 segment uses `glm4`. `zai-org/glm-4-9b-chat` uses `glm4`. `glm-4` does not take GLM-4.5, GLM-4.1V, or GLM-Z1. A 5090 smoke seated `llama3` and trained `Qwen/Qwen2.5-0.5B-Instruct` with template `qwen`. Confirm the hint before train. That repo id is an example an operator supplies. This factory does not turn the seat tag `llama3` into a Llama-3 Hub repo, and it does not download weights.

Phi-3 and Phi-3.5 Instruct are a reproduce target beside that Qwen LoRA/QLoRA pair. `microsoft/Phi-3-mini-4k-instruct`, `microsoft/Phi-3-mini-128k-instruct`, `microsoft/Phi-3-medium-4k-instruct`, and `microsoft/Phi-3-medium-128k-instruct` infer template `phi`. `microsoft/Phi-3.5-mini-instruct` and `microsoft/Phi-3.5-MoE-instruct` infer template `phi` as well. `microsoft/Phi-3-small-8k-instruct` and `microsoft/Phi-3-small-128k-instruct` infer template `phi_small`. `microsoft/phi-4` infers `phi4`. `microsoft/Phi-4-mini-instruct` infers `phi4_mini`. A nested local path uses the same scan: `/opt/hf/microsoft/Phi-3-mini-4k-instruct`, `./weights/Phi-3.5-mini-instruct`, and an HF cache segment `models--microsoft--Phi-3-mini-4k-instruct` under `snapshots/<rev>`. LLaMA-Factory `src/llamafactory/extras/constants.py` registers those groups with those template names. `src/llamafactory/data/template.py` registers `phi` as the `<|user|>` / `<|end|>` / `<|assistant|>` chat. Current `examples/train_qlora` has no Phi yaml. The bitsandbytes fields stay the shape in `examples/train_qlora/qwen3_lora_sft_otfq.yaml`: `quantization_method: bnb` and `quantization_bit: 4`. `examples/fixtures/phi3-instruct.pack.json` sets `train_base_model` to `microsoft/Phi-3-mini-4k-instruct` and `model_hint` to `llama3`. `estate enrich prepare --driver llamafactory-qlora` on that pack writes template `phi`, those QLoRA fields, and a line in `NEXT.md` and `PREPARE.md` that names this reproduce target beside Qwen LoRA/QLoRA. The QLoRA note is QLoRA-only. It covers template `phi` and template `phi_small`. `llamafactory-lora` on Phi-3 mini, Phi-3 medium, and Phi-3.5 is the non-quant twin of that prepare. It writes template `phi`, `lora_rank` 8, `packing: false`, and no `quantization_bit` and no `quantization_method`. `NEXT.md` and `PREPARE.md` name that LoRA reproduce target only on `llamafactory-lora` and only for that Instruct shape. Phi-3-small keeps template `phi_small` and does not get the LoRA line. Phi-4 keeps `phi4`. Phi-4-mini keeps `phi4_mini`. Neither gets the LoRA line. `examples/fixtures/phi3-instruct-lora.pack.json` is that smoke pack (`model_hint` `llama3`, `train_base_model` `microsoft/Phi-3-mini-4k-instruct`). The seat tag stays separate. A bare Ollama tag is `refuse:train-base`. A local path whose leaf is the lowercase id `phi-3-mini-4k-instruct` is `refuse:train-base`, the same seat-tag check as `./llama3`. Prepare does not download the weights.

Llama-3.2 Instruct is a reproduce target beside Phi-3 and that Qwen pair. `meta-llama/Llama-3.2-1B-Instruct` and `meta-llama/Llama-3.2-3B-Instruct` infer template `llama3`. LLaMA-Factory `src/llamafactory/extras/constants.py` registers that group with `template="llama3"`, the same name as Llama-3, Llama-3.1, and Llama-3.3 text. `src/llamafactory/data/template.py` registers `llama3`. There is no `llama3_2` template. `meta-llama/Llama-3.2-11B-Vision-Instruct` and `meta-llama/Llama-3.2-90B-Vision-Instruct` infer `mllama`. `llava-hf/llama3-llava-next-8b-hf` infers `llava_next_llama3`. `huggyllama/llama-30b` stays `default`. A short `llama-3` stem does not take those names. A nested local path uses the same scan: `/opt/hf/meta-llama/Llama-3.2-3B-Instruct`, `./weights/Llama-3.2-1B-Instruct`, and an HF cache segment `models--meta-llama--Llama-3.2-3B-Instruct` under `snapshots/<rev>`. The last segment that names a family wins. `examples/fixtures/llama32-instruct.pack.json` sets `train_base_model` to `meta-llama/Llama-3.2-3B-Instruct` and `model_hint` to `llama3`. `estate enrich prepare --driver llamafactory-qlora` on that pack writes template `llama3`, `quantization_method: bnb`, `quantization_bit: 4`, and a line in `NEXT.md` and `PREPARE.md` that names this reproduce target beside Phi-3 and Qwen. The QLoRA note is QLoRA-only and only for Llama-3.2-1B-Instruct and Llama-3.2-3B-Instruct. `llamafactory-lora` on the same ids is the non-quant twin of that prepare. It writes template `llama3`, `lora_rank` 8, `packing: false`, and no `quantization_bit` and no `quantization_method`. `NEXT.md` and `PREPARE.md` name that LoRA reproduce target only on `llamafactory-lora` and only for that Instruct shape. `examples/fixtures/llama32-instruct-lora.pack.json` is that smoke pack (`model_hint` `llama3`, `train_base_model` `meta-llama/Llama-3.2-3B-Instruct`). Llama-3.2 vision keeps template `mllama` and does not get either line. A Llama-3.2 base, Llama-3.1 Instruct, and Llama-3.3 Instruct keep template `llama3` and do not get either line. The seat tag stays separate. A bare Ollama tag is `refuse:train-base`. A local path whose leaf is the lowercase id `llama-3.2-3b-instruct` is `refuse:train-base`, the same seat-tag check as `./llama3`. Prepare does not download the weights.

Gemma-2 Instruct is a reproduce target beside Phi-3, Llama-3.2, and that Qwen pair. `google/gemma-2-2b-it`, `google/gemma-2-9b-it`, and `google/gemma-2-27b-it` infer template `gemma2`. LLaMA-Factory `src/llamafactory/extras/constants.py` registers that Gemma-2 group with `template="gemma2"`. `src/llamafactory/data/template.py` registers `gemma2`. There is no `gemma_2` template. A short `gemma` stem does not take those names. `google/gemma-2b`, `google/gemma-2b-it`, and `google/gemma-7b` stay template `gemma`. `google/gemma-3-4b-it` stays off `gemma2`. A nested local path uses the same scan. The family segment can sit above a layout leaf: `/opt/hf/google/gemma-2-2b-it/weights`, `./weights/google/gemma-2-9b-it/weights`, and an HF cache segment `models--google--gemma-2-2b-it` under `snapshots/<rev>`. The last segment that names a family wins. A local path whose leaf is the lowercase id `gemma-2-2b-it` is `refuse:train-base`, the same seat-tag check as `./llama3`. `examples/fixtures/gemma2-instruct.pack.json` sets `train_base_model` to `google/gemma-2-2b-it` and `model_hint` to `llama3`. `estate enrich prepare --driver llamafactory-qlora` on that pack writes template `gemma2`, `quantization_method: bnb`, `quantization_bit: 4`, and a line in `NEXT.md` and `PREPARE.md` that names this reproduce target beside Phi-3, Llama-3.2, and Qwen. The QLoRA note is QLoRA-only and only for a Gemma-2 Instruct id. `llamafactory-lora` on the same ids is the non-quant twin of that prepare. It writes template `gemma2`, `lora_rank` 8, `packing: false`, and no `quantization_bit` and no `quantization_method`. `NEXT.md` and `PREPARE.md` name that LoRA reproduce target only on `llamafactory-lora` and only for a Gemma-2 Instruct id. `examples/fixtures/gemma2-instruct-lora.pack.json` is that smoke pack (`model_hint` `llama3`, `train_base_model` `google/gemma-2-2b-it`). A Gemma-2 base such as `google/gemma-2-2b` keeps template `gemma2` and does not get either line. Original Gemma and Gemma-3 do not get either line. The seat tag stays separate. A bare Ollama tag is `refuse:train-base`. Prepare does not download the weights.

Mistral Instruct is a reproduce target beside Phi-3, Llama-3.2, Gemma-2, and that Qwen pair. `mistralai/Mistral-7B-Instruct-v0.1`, `mistralai/Mistral-7B-Instruct-v0.2`, and `mistralai/Mistral-7B-Instruct-v0.3` infer template `mistral`. LLaMA-Factory `src/llamafactory/extras/constants.py` registers that Mistral-7B group, base and Instruct together, with `template="mistral"`. `src/llamafactory/data/template.py` registers `mistral`. There is no `mistral_7` template. A short `mistral` stem does not take the other groups. `mistralai/Mistral-7B-v0.1`, `alpindale/Mistral-7B-v0.2-hf`, and `mistralai/Mistral-7B-v0.3` keep template `mistral` and are not this reproduce target. `mistralai/Mistral-Small-24B-Instruct-2501` and the Small 3.1 / 3.2 ids infer `mistral_small`. `mistralai/Mistral-Nemo-Base-2407` and `mistralai/Mistral-Nemo-Instruct-2407` infer `ministral`. `mistralai/Mixtral-8x7B-Instruct-v0.1` infers `mistral` and is not this reproduce target. `llava-hf/llava-v1.6-mistral-7b-hf` infers `llava_next_mistral`. Ministral, Ministral-3, Codestral, Devstral, and Pixtral stay off `mistral`. A nested local path uses the same scan. The family segment can sit above a layout leaf: `/opt/hf/mistralai/Mistral-7B-Instruct-v0.3/weights`, `./weights/mistralai/Mistral-7B-Instruct-v0.2/weights`, and an HF cache segment `models--mistralai--Mistral-7B-Instruct-v0.3` under `snapshots/<rev>`. The last segment that names a family wins. A local path whose leaf is the lowercase id `mistral-7b-instruct-v0.3` is `refuse:train-base`, the same seat-tag check as `./llama3`. Current `examples/train_qlora` has no Mistral yaml. The bitsandbytes fields stay `quantization_method: bnb` and `quantization_bit: 4`. `examples/fixtures/mistral-instruct.pack.json` sets `train_base_model` to `mistralai/Mistral-7B-Instruct-v0.3` and `model_hint` to `llama3`. That 7B Instruct id is the smallest Instruct checkpoint in the Mistral-7B group. `estate enrich prepare --driver llamafactory-qlora` on that pack writes template `mistral`, those QLoRA fields, and a line in `NEXT.md` and `PREPARE.md` that names this reproduce target beside Phi-3, Llama-3.2, Gemma-2, and Qwen. The QLoRA note is QLoRA-only and only for a Mistral-7B Instruct id. `llamafactory-lora` on the same ids is the non-quant twin of that prepare. It writes template `mistral`, `lora_rank` 8, `packing: false`, and no `quantization_bit` and no `quantization_method`. `NEXT.md` and `PREPARE.md` name that LoRA reproduce target only on `llamafactory-lora` and only for a Mistral-7B Instruct id. `examples/fixtures/mistral-instruct-lora.pack.json` is that smoke pack (`model_hint` `llama3`, `train_base_model` `mistralai/Mistral-7B-Instruct-v0.3`). A Mistral-7B base keeps template `mistral` and does not get either line. Mistral-Small, Mistral-Nemo, Mixtral, and LLaVA-NeXT-Mistral do not get either line. The seat tag stays separate. A bare Ollama tag is `refuse:train-base`. Prepare does not download the weights.

Qwen2.5 Instruct is a reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct. `Qwen/Qwen2.5-0.5B-Instruct`, `Qwen/Qwen2.5-1.5B-Instruct`, `Qwen/Qwen2.5-3B-Instruct`, `Qwen/Qwen2.5-7B-Instruct`, `Qwen/Qwen2.5-14B-Instruct`, `Qwen/Qwen2.5-32B-Instruct`, `Qwen/Qwen2.5-72B-Instruct`, `Qwen/Qwen2.5-7B-Instruct-1M`, and `Qwen/Qwen2.5-14B-Instruct-1M` infer template `qwen`. LLaMA-Factory `src/llamafactory/extras/constants.py` registers that text group with `template="qwen"`. `src/llamafactory/data/template.py` registers `qwen`. There is no `qwen2_5` template. A Qwen2.5 base uses that same template and is not this reproduce target. A name that contains `thinking` is not this reproduce target. Qwen2 Instruct stays template `qwen` and is not this reproduce target. Qwen2.5-Coder and Qwen2.5-Math sit in that same `template="qwen"` group and are not this reproduce target. Qwen2.5-VL and Qwen2-VL are `qwen2_vl`. Qwen2.5-Omni is `qwen2_omni`. GPTQ and AWQ checkpoints of these Instruct ids are not this reproduce target. Qwen3 Instruct stays `qwen3_nothink` and is not this reproduce target. A nested local path uses the same scan. The family segment can sit above a layout leaf: `/opt/hf/Qwen/Qwen2.5-0.5B-Instruct/weights`, `./weights/Qwen/Qwen2.5-7B-Instruct/weights`, and an HF cache segment `models--Qwen--Qwen2.5-0.5B-Instruct` under `snapshots/<rev>`. The last segment that names a family wins. A local path whose leaf is the lowercase id `qwen2.5-0.5b-instruct` is `refuse:train-base`, the same seat-tag check as `./llama3`. `examples/fixtures/qwen25-instruct.pack.json` sets `train_base_model` to `Qwen/Qwen2.5-0.5B-Instruct` and `model_hint` to `llama3`. That 0.5B Instruct id is the 5090 smoke train base and the smallest checkpoint in this Instruct group. `estate enrich prepare --driver llamafactory-qlora` on that pack writes template `qwen`, `quantization_method: bnb`, `quantization_bit: 4`, LoRA rank 16, `packing: true`, and a line in `NEXT.md` and `PREPARE.md` that names this reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct. The note is on `llamafactory-qlora` and only for that Qwen2.5 Instruct shape. `llamafactory-lora` on the same ids is the non-quant twin of that prepare. It writes template `qwen`, `lora_rank` 8, `packing: false`, and no `quantization_bit` and no `quantization_method`. `NEXT.md` and `PREPARE.md` name that LoRA reproduce target only on `llamafactory-lora` and only for that Instruct shape. `examples/fixtures/qwen25-instruct-lora.pack.json` is that smoke pack (`model_hint` `llama3`, `train_base_model` `Qwen/Qwen2.5-0.5B-Instruct`). A Qwen2.5 base, a thinking-shaped id, Qwen2, Qwen2.5-Coder, Qwen2.5-Math, and Qwen2.5-VL do not get either line. A Qwen3 Instruct id keeps `qwen3_nothink` and does not get either line. The seat tag stays separate. A bare Ollama tag is `refuse:train-base`. Prepare does not download the weights.

DeepSeek-R1-Distill chat is a QLoRA reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, and Qwen3 Instruct. `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`, `deepseek-ai/DeepSeek-R1-Distill-Qwen-7B`, `deepseek-ai/DeepSeek-R1-Distill-Llama-8B`, `deepseek-ai/DeepSeek-R1-Distill-Qwen-14B`, `deepseek-ai/DeepSeek-R1-Distill-Qwen-32B`, and `deepseek-ai/DeepSeek-R1-Distill-Llama-70B` infer template `deepseekr1`. LLaMA-Factory `src/llamafactory/extras/constants.py` registers that distill group with `template="deepseekr1"`, in the same call as DeepSeek-R1, DeepSeek-R1-Zero, and DeepSeek-R1-0528. `src/llamafactory/data/template.py` registers `deepseekr1` as a `ReasoningTemplate`. There is no `deepseek_r1` template. `examples/train_qlora` does not ship a DeepSeek yaml. The bitsandbytes fields stay the shape in `examples/train_qlora/qwen3_lora_sft_otfq.yaml`: `quantization_method: bnb` and `quantization_bit: 4`. These ids are chat models. They are not named Instruct. `DeepSeek-R1-Distill-Qwen-1.5B` is the smallest checkpoint in that group and the common SLM distill (Ollama tag `deepseek-r1:1.5b`). The Qwen and Llama substrings in these ids do not select `qwen` or `llama3`. The student checkpoints stay on their own templates and are not this reproduce target: `Qwen/Qwen2.5-Math-1.5B`, `Qwen/Qwen2.5-Math-7B`, `Qwen/Qwen2.5-14B`, and `Qwen/Qwen2.5-32B` stay template `qwen`. `meta-llama/Llama-3.1-8B` and `meta-llama/Llama-3.3-70B-Instruct` stay template `llama3`. Qwen2.5 Instruct stays template `qwen` and is a different reproduce target. Llama-3.2 Instruct stays template `llama3` and is a different reproduce target. DeepSeek-R1 and DeepSeek-R1-Zero use template `deepseekr1` and are not this reproduce target. DeepSeek-R1-0528 and `deepseek-ai/DeepSeek-R1-0528-Qwen3-8B` use template `deepseekr1` and are not this reproduce target. DeepSeek-V2 and DeepSeek-Coder-V2 use template `deepseek`. DeepSeek-V2.5 and DeepSeek-V3 use template `deepseek3`. DeepSeek-Coder uses template `deepseekcoder`. GPTQ, AWQ, and GGUF checkpoints of these distill ids are not this reproduce target. A nested local path uses the same scan. The family segment can sit above a layout leaf: `/opt/hf/deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B/weights`, `./weights/deepseek-ai/DeepSeek-R1-Distill-Qwen-7B/weights`, and an HF cache segment `models--deepseek-ai--DeepSeek-R1-Distill-Qwen-1.5B` under `snapshots/<rev>`. The last segment that names a family wins. A local path whose leaf is the lowercase id `deepseek-r1-distill-qwen-1.5b` is `refuse:train-base`, the same seat-tag check as `./llama3`. `examples/fixtures/deepseek-r1-distill.pack.json` sets `train_base_model` to `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B` and `model_hint` to `llama3`. `estate enrich prepare --driver llamafactory-qlora` on that pack writes template `deepseekr1`, `quantization_method: bnb`, `quantization_bit: 4`, LoRA rank 16, `packing: true`, and a line in `NEXT.md` and `PREPARE.md` that names this reproduce target. The note is on `llamafactory-qlora` and only for those six distill chat ids. `llamafactory-lora` on the same ids is the non-quant twin of that prepare. It writes template `deepseekr1`, `lora_rank` 8, `packing: false`, and no `quantization_bit` and no `quantization_method`. `examples/train_lora` does not ship a DeepSeek yaml. `NEXT.md` and `PREPARE.md` name that LoRA reproduce target only on `llamafactory-lora` and only for those six distill chat ids. The QLoRA note stays on `llamafactory-qlora`. `examples/fixtures/deepseek-r1-distill-lora.pack.json` is that smoke pack (`model_hint` `llama3`, `train_base_model` `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`). DeepSeek-R1, DeepSeek-R1-Zero, and DeepSeek-R1-0528 do not get either line. The seat tag stays `llama3`. An Ollama tag such as `deepseek-r1` or `deepseek-r1:1.5b` is a seat tag for this checkpoint. It is not the Hugging Face train base. A bare Ollama tag is `refuse:train-base`. Prepare does not download the weights.

GLM-4 Chat is a QLoRA reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, Qwen3 Instruct, and DeepSeek-R1-Distill chat. `zai-org/glm-4-9b-chat`, `zai-org/glm-4-9b-chat-1m`, `zai-org/GLM-4-9B-0414`, and `zai-org/GLM-4-32B-0414` infer template `glm4`. LLaMA-Factory `src/llamafactory/extras/constants.py` registers that group with `template="glm4"`, in the same call as the base ids `zai-org/glm-4-9b` and `zai-org/GLM-4-32B-Base-0414`. The DEFAULT DownloadSource for GLM-4-9B-Chat is `zai-org/glm-4-9b-chat`. ModelScope in that group is `ZhipuAI/glm-4-9b-chat`. `src/llamafactory/data/template.py` registers `glm4`. There is no `glm_4` template. ChatGLM3 is `chatglm3`. `examples/train_qlora` does not ship a GLM-4 yaml. The bitsandbytes fields stay the shape in `examples/train_qlora/qwen3_lora_sft_otfq.yaml`: `quantization_method: bnb` and `quantization_bit: 4`. These ids are chat models. `GLM-4-9B-Chat` is the smallest popular chat checkpoint in that group. A GLM-4 base uses template `glm4` and is not this reproduce target. GLM-Z1 uses template `glmz1`. GLM-4.1V uses template `glm4v`. GLM-4.5 uses template `glm4_moe`. GLM-4.5V and GLM-4.6V use template `glm4_5v`. GLM-4.7 uses template `glm4_7`. GLM-OCR uses template `glm_ocr`. Kimi is not in this group. This scan does not claim Kimi. GPTQ, AWQ, and GGUF checkpoints of these chat ids are not this reproduce target. `zai-org/glm-4-9b-chat-hf` is not listed in that group and is not this reproduce target. The older `THUDM/glm-4-9b-chat` id is not the DEFAULT source in current `constants.py`. A model segment `glm-4-9b-chat` still matches this chat shape. A nested local path uses the same scan. The family segment can sit above a layout leaf: `/opt/hf/zai-org/glm-4-9b-chat/weights`, `./weights/zai-org/glm-4-9b-chat/weights`, and an HF cache segment `models--zai-org--glm-4-9b-chat` under `snapshots/<rev>`. The last segment that names a family wins. A local path whose leaf is the lowercase id `glm-4-9b-chat` is `refuse:train-base`, the same seat-tag check as `./llama3`. `examples/fixtures/glm4-chat.pack.json` sets `train_base_model` to `zai-org/glm-4-9b-chat` and `model_hint` to `llama3`. `estate enrich prepare --driver llamafactory-qlora` on that pack writes template `glm4`, `quantization_method: bnb`, `quantization_bit: 4`, LoRA rank 16, `packing: true`, and a line in `NEXT.md` and `PREPARE.md` that names this reproduce target. The note is on `llamafactory-qlora` and only for those GLM-4 Chat ids. `llamafactory-lora` on the same ids is the non-quant twin of that prepare. It writes template `glm4`, `lora_rank` 8, `packing: false`, and no `quantization_bit` and no `quantization_method`. `examples/train_lora` does not ship a GLM-4 yaml. `NEXT.md` and `PREPARE.md` name that LoRA reproduce target only on `llamafactory-lora` and only for those GLM-4 Chat ids. The QLoRA note stays on `llamafactory-qlora`. `examples/fixtures/glm4-chat-lora.pack.json` is that smoke pack (`model_hint` `llama3`, `train_base_model` `zai-org/glm-4-9b-chat`). A GLM-4 base, GLM-Z1, GLM-4.1V, GLM-4.5, and ChatGLM3 do not get either line. Llama-3.2 Instruct, Gemma-2 Instruct, Mistral Instruct, Qwen3 Instruct, Phi-3 Instruct, Qwen2.5 Instruct, and DeepSeek-R1-Distill chat stay on their own LoRA lines. The seat tag stays `llama3`. An Ollama library tag such as `glm4`, `glm4:9b`, `glm4:latest`, or `glm4:9b-chat-q2_K` is a seat tag for this checkpoint. `glm-4` and `glm-4:9b` are seat tags too. They are not the Hugging Face train base. A bare Ollama tag is `refuse:train-base`. Prepare does not download the weights.

Qwen3 Instruct is a reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x. `Qwen/Qwen3-4B-Instruct-2507`, `Qwen/Qwen3-30B-A3B-Instruct-2507`, `Qwen/Qwen3-235B-A22B-Instruct-2507`, and `Qwen/Qwen3-Next-80B-A3B-Instruct` infer template `qwen3_nothink`. LLaMA-Factory `src/llamafactory/extras/constants.py` registers that text group with `template="qwen3_nothink"`. `src/llamafactory/data/template.py` registers `qwen3_nothink`. `examples/train_lora/qwen3_lora_sft.yaml` and `examples/train_qlora/qwen3_lora_sft_otfq.yaml` set `model_name_or_path` to `Qwen/Qwen3-4B-Instruct-2507` and `template: qwen3_nothink`. The QLoRA example also sets `quantization_method: bnb` and `quantization_bit: 4`. A Qwen3 thinking or base checkpoint uses template `qwen3` and is not this reproduce target. That includes `Qwen/Qwen3-4B` (the hub id for display name `Qwen3-4B-Thinking`), `Qwen/Qwen3-4B-Thinking-2507`, `Qwen/Qwen3-4B-Base`, and `Qwen/Qwen3-Next-80B-A3B-Thinking`. Qwen2 stays template `qwen` and is not this reproduce target. Qwen2.5 Instruct, including `Qwen/Qwen2.5-0.5B-Instruct`, stays template `qwen` and is a different reproduce target on `llamafactory-qlora` and on `llamafactory-lora`. Qwen3-VL Instruct and Qwen3-Omni Instruct are other groups in `constants.py` (`qwen3_vl_nothink` and `qwen3_omni_nothink`) and are not this reproduce target. A nested local path uses the same scan. The family segment can sit above a layout leaf: `/opt/hf/Qwen/Qwen3-4B-Instruct-2507/weights`, `./weights/Qwen/Qwen3-30B-A3B-Instruct-2507/weights`, and an HF cache segment `models--Qwen--Qwen3-4B-Instruct-2507` under `snapshots/<rev>`. The last segment that names a family wins. A local path whose leaf is the lowercase id `qwen3-4b-instruct-2507` is `refuse:train-base`, the same seat-tag check as `./llama3`. `examples/fixtures/qwen3-instruct.pack.json` sets `train_base_model` to `Qwen/Qwen3-4B-Instruct-2507` and `model_hint` to `llama3`. That 4B Instruct id is the smallest checkpoint in the `qwen3_nothink` group and the id in the official yaml. `estate enrich prepare --driver llamafactory-qlora` on that pack writes template `qwen3_nothink`, `quantization_method: bnb`, `quantization_bit: 4`, and a line in `NEXT.md` and `PREPARE.md` that names this reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x. The QLoRA note is QLoRA-only and only for that Qwen3 Instruct shape. `llamafactory-lora` on the same ids is the non-quant twin of that prepare. It writes template `qwen3_nothink`, `lora_rank` 8, `packing: false`, and no `quantization_bit` and no `quantization_method`, matching `examples/train_lora/qwen3_lora_sft.yaml`. `NEXT.md` and `PREPARE.md` name that LoRA reproduce target only on `llamafactory-lora` and only for that Instruct shape. `examples/fixtures/qwen3-instruct-lora.pack.json` is that smoke pack (`model_hint` `llama3`, `train_base_model` `Qwen/Qwen3-4B-Instruct-2507`). A Qwen3 thinking id keeps template `qwen3` and does not get either line. `Qwen/Qwen2.5-0.5B-Instruct` keeps template `qwen` and does not get the Qwen3 lines. The seat tag stays separate. A bare Ollama tag is `refuse:train-base`. Prepare does not download the weights.

A missing train base, a bare Ollama tag (`llama3`, `llama3:latest`), or a local path whose directory name is an Ollama seat tag (`./llama3`, `../llama3`) is `refuse:train-base` and writes nothing. That refuse applies to `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, `unsloth-qlora`, and `mlx-lm-lora`. `--all-drivers --job train` refuses the whole set in that case, so no sibling directory is left behind. On an affinity other than `apple-silicon`, `mlx-lm-lora` is not in that set.

`base_model` in `axolotl.yml` is that same train base. `prepare.json` `base_model` and `seat_tag` stay the Ollama id for Modelfile `FROM` and for the adapter join. `NEXT.md` names the seat tag and the train base.

On `llamafactory-lora` and `llamafactory-qlora`, `dataset.jsonl` is instruct chat JSONL (`messages` of `role` and `content`). `dataset_info.json` marks it sharegpt so LLaMA-Factory applies the recipe `template`. That same chat template is what you seat. On `axolotl-lora` and `axolotl-qlora`, `dataset.jsonl` is Alpaca JSONL (`instruction`, `input`, `output`), which Axolotl reads with `type: alpaca` and `ds_type: json`.

Default prepare does not read pack sources and does not download them. The train recipe cards say so in the same paragraph in `PREPARE.md` and `NEXT.md`. `prepare.json` records `dataset_mode`, `dataset_rows`, `dataset_from_feed`, `dataset_skipped`, and `dataset_read_paths`.

| `dataset_mode` | When | What `dataset.jsonl` holds |
| --- | --- | --- |
| `stub` | `source_paths` is empty | Three example rows. These rows are not training data. |
| `scaffold` | `source_paths` is set and `--from-feed` is omitted | One row per path. The row names the path. The file was not read. |
| `feed` | `--from-feed`, and every source path is already a file under `--state-dir` | Instruct rows copied from those files. |

An empty source path string is `refuse:dataset`. `--from-feed` with an empty `source_paths` list is the same refuse. A missing file, a path that resolves outside the cell state directory, a line that is not JSON, or a line that is not an instruct row is `refuse:dataset` and writes nothing. Omit `--from-feed` to keep the scaffold.

A line already on disk is an instruct row when it is one of the shapes the feed and the trainers already use:

- ShareGPT: `messages` of `role` and `content` (`system`, `user`, `assistant`).
- Alpaca: `instruction` and `output`, with optional `input`.
- A scrubbed feed event (`kind`, plus a `note`). The note is the completion. Events with no note are counted in `dataset_skipped` and left out. If every event is skipped, prepare is `refuse:dataset`.

This factory does not invent a completion for an event that has no note. Before a line is copied as ShareGPT or Alpaca, prepare checks the raw record. `kind` and `object_class` still classify a frontier event when the same line also has `messages` or `instruction`. A frontier event on an estate with no frontier binding is `refuse:frontier-invent`. Sacred text is `refuse:sacred`. A hardware SKU is `refuse:sku-banned`. A raw secret is `refuse:raw-secret`. Those checks see the event fields, not only the copied row.

Prepare opens each source and reads that file handle. On Linux it resolves `/proc/self/fd` for the opened file (macOS uses the opened-file path) and refuses when that file sits outside the cell state directory. A missing pin is `refuse:dataset`. Each file is at most 8 MiB. All source files together are at most 8 MiB. Each of the chat and Alpaca copies is at most 16 MiB. A file, a total, or a copy over that cap is `refuse:dataset`.

Operator loop: set the train base and prepare → optionally hydrate `dataset.jsonl` with `--from-feed` when the pack sources are already under the cell state directory → run the trainer named in `NEXT.md` on the CUDA host → `estate enrich import-trained`. The popular Qwen order is Target C, above: `llamafactory-qlora` prepare, the `NEXT.md` train and export lines, `gguf-convert`, `local-seat`, then `import-trained` for the GGUF shape. The unquantized Qwen order is Target A: `llamafactory-lora` prepare, the `NEXT.md` train and export lines, `merge-adapt`, `gguf-convert`, `local-seat`, then `import-trained` for the GGUF shape. Prepare does not train. `--from-feed` does not download.

Select the LLaMA-Factory card with `--driver`. `llamafactory-lora` is 16-bit LoRA (`finetuning_type: lora`, no `quantization_bit`, no `quantization_method`, `lora_rank: 8`, `lora_alpha: 16`, `packing: false`). Rank 8 matches `examples/train_lora/qwen3_lora_sft.yaml`. `lora_alpha` 16 is `lora_rank` times 2, which is what LLaMA-Factory uses when that field is unset. That official file leaves `packing` unset, and the LLaMA-Factory default is false. `llamafactory-qlora` is 4-bit QLoRA (`finetuning_type: lora`, `quantization_bit: 4`, `quantization_method: bnb`, `lora_rank: 16`, `packing: true`). `bnb` is the LLaMA-Factory 0.9 token that selects the 4-bit bitsandbytes branch. Both cards keep `cutoff_len` at `512` so a first run stays short. Official SFT examples use `2048`, `num_train_epochs` 3.0, `gradient_accumulation_steps` 8, and `warmup_ratio` 0.1. The prepared recipe keeps `num_train_epochs: 1.0`, `gradient_accumulation_steps: 4`, `warmup_ratio: 0.03`, and `save_steps: 50`, and leaves `max_steps` unset. Pass `--official-scale` to write `cutoff_len` 2048, `num_train_epochs` 3.0, `gradient_accumulation_steps` 8, and `warmup_ratio` 0.1 from `examples/train_lora/qwen3_lora_sft.yaml`. Rank, packing, and quantization stay on the selected card. `--max-steps` still overrides `num_train_epochs`. A prepare with `--official-scale` and no train recipe card is `refuse:official-scale` and writes nothing. A short gauge run passes `--max-steps 10`. LLaMA-Factory overrides `num_train_epochs` when `max_steps` is set. When that count is under 50, prepare also sets `save_steps` to the same count so a checkpoint exists during the short run. A later preference stage (`stage: dpo` or `stage: orpo`, with `ranking: true` in `dataset_info.json`) is a comment in the recipe. These cards do not build that dataset. Axolotl bf16 LoRA is `--driver axolotl-lora`. Axolotl 4-bit QLoRA is `--driver axolotl-qlora`. `--official-scale` leaves both Axolotl cards on their example files. The knobs for both cards are in the Axolotl section below.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id-or-pack.json> \
  --driver llamafactory-lora \
  --job train \
  --state-dir .cell
```

Omitting `--job` on `--driver llamafactory-lora` is the same train job. `--job enrich` is `refuse:job` and writes nothing. The output directory is `.cell/enrich/{pack_id}/llamafactory-lora/`, with the same file names as the QLoRA card. `recipe.yaml` omits quantization. `export.yaml` also omits it: do not set `quantization_bit` on the merge, and do not merge a quantized base. `NEXT.md` installs with `pip install llamafactory` and says this LoRA path does not require bitsandbytes.

```bash
pip install llamafactory
llamafactory-cli train .cell/enrich/<pack-id>/llamafactory-lora/recipe.yaml
llamafactory-cli export .cell/enrich/<pack-id>/llamafactory-lora/export.yaml
```

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id-or-pack.json> \
  --driver llamafactory-qlora \
  --job train \
  --state-dir .cell
```

Omitting `--job` on `--driver llamafactory-qlora` is the same train job. `--job enrich` is `refuse:job` and writes nothing. `--all-drivers --job train` writes both LLaMA-Factory cards and both Axolotl cards next to the Modelfile and the external manifest. `--all-drivers` without `--job` stays on `enrich` and skips the train cards.

The output directory is `.cell/enrich/{pack_id}/llamafactory-qlora/`:

| File | Role |
| --- | --- |
| `recipe.yaml` | LLaMA-Factory SFT QLoRA recipe. Dataset dir and output dir are absolute. |
| `export.yaml` | Merge recipe. No `quantization_bit`. |
| `dataset_info.json` | ShareGPT column map for `dataset.jsonl`. |
| `dataset.jsonl` | Instruct chat rows. `dataset_mode` is `stub`, `scaffold`, or `feed`. |
| `PREPARE.md` | What this step wrote, including `dataset_mode` and `llamafactory-cli train recipe.yaml` from that directory. |
| `NEXT.md` | `pip install llamafactory`, `pip install 'bitsandbytes>=0.49'`, `llamafactory-cli train <absolute>/recipe.yaml`, `llamafactory-cli export`, the seat tag, the train base, the same dataset paragraph as `PREPARE.md`, the gauge note, and the import line. |
| `prepare.json` | `job` is `train`. `base_model` and `seat_tag` are the Ollama id. `train_base_model` is the Hugging Face repo or local HF directory. `dataset_mode`, `dataset_rows`, `dataset_from_feed`, `dataset_skipped`, and `dataset_read_paths` describe `dataset.jsonl`. `promoted`, `auto_apply`, and `estate_rewritten` are false. |

Run the commands from `NEXT.md` on a CUDA host. This factory does not run them.

```bash
pip install llamafactory
pip install 'bitsandbytes>=0.49'
llamafactory-cli train .cell/enrich/<pack-id>/llamafactory-qlora/recipe.yaml
llamafactory-cli export .cell/enrich/<pack-id>/llamafactory-qlora/export.yaml
```

QLoRA needs bitsandbytes. `pip install llamafactory` and `llamafactory[torch,metrics]` 0.9.5 did not install it. Install bitsandbytes in that same environment. On a consumer RTX host, keep the torch CUDA wheel you already installed. A 5090 smoke used torch 2.11.0+cu128 (CUDA 12.8) and bitsandbytes 0.50.2. That bitsandbytes install did not replace torch. This factory does not install either package. If the torch wheel still does not match the CUDA install on the box, follow https://github.com/hiyouga/LLaMA-Factory#installation.

A short gauge run adds `--max-steps 10` to the prepare command. The default recipe stays one epoch.

The train writes the LoRA adapter under `outputs/` (`adapter_config.json` and the adapter weights). Prepare does not merge. `export.yaml` is the merge card. `adapter_name_or_path` is the same path as recipe `output_dir`. `NEXT.md` says the merge has not happened, and `export/` has no merged weights until `llamafactory-cli export` exits 0. An early stop may leave the adapter only under `checkpoint-<step>` inside that output directory. Point `adapter_name_or_path` at that checkpoint directory. Prepare does not rewrite `export.yaml` after train. A Modelfile that `llamafactory-cli export` writes into the export directory belongs to that tool. Do not set `quantization_bit` on that merge, and do not merge a quantized base. `import-trained` refuses `export.yaml` when a real key `quantization_bit` or `quantization_method` is set (`refuse:export`). A comment line does not trip that refuse. LLaMA-Factory does not write GGUF. A live 5090 prove on 2026-09-23 exported `Qwen/Qwen2.5-0.5B-Instruct` and `convert_hf_to_gguf.py` failed: `tokenizer_config.json` had `extra_special_tokens` as a list (`AttributeError: 'list' object has no attribute 'keys'`), and the export omitted `vocab.json` and `merges.txt`. Restoring the tokenizer files from the HF cache snapshot already on disk, and keeping the export `tokenizer_config.json` as `tokenizer_config.json.bak`, let that convert write a 949M BF16 GGUF. The same export shape is [LLaMA-Factory issue 10169](https://github.com/hiyouga/LlamaFactory/issues/10169). `estate enrich gguf-convert` returns `refuse:tokenizer` for that list, for JSON null, and for a Qwen-family export missing `vocab.json` or `merges.txt`. Qwen-family is `config.json` `model_type` or `architectures`, or `tokenizer_class`, naming Qwen. `NEXT.md` and `PREPARE.md` name that restore: copy the tokenizer files from the HF cache snapshot, or the equivalent base checkout, into the export directory, then re-run `estate enrich gguf-convert`. This factory does not download those files and does not copy them. After the merge, `estate enrich gguf-convert` prints the llama.cpp `convert_hf_to_gguf.py` line (`--outtype auto`, outfile beside the export directory) when that check passes. Then seat tag `cell-enrich-{pack_id}` on Ollama with `FROM` that GGUF. To load the adapter without a merge, `estate enrich local-seat --adapter` prints a Modelfile. `FROM` is the seat tag (an Ollama model of the same train base). `ADAPTER` is the adapter directory. The command does not run `ollama create`. `--weights` on that command stays the merged or GGUF path. Use the chat template the recipe named. After the tag is seated, send a short prompt that checks the pack purpose. This factory does not run `ollama create`, does not run `convert_hf_to_gguf.py`, and does not run that smoke eval.

On Nvidia only, Unsloth QLoRA is a faster single-GPU alternate. `NEXT.md` points at the Unsloth docs. This card does not call Unsloth and does not write a script.

`axolotl-lora` and `axolotl-qlora` are the same loop with a YAML recipe. Use them when you want a config file or a multi-GPU run. `base_model` in `axolotl.yml` is the train base. The seat tag stays in `prepare.json` for Ollama seating. The handoff command is the official `axolotl train` from the Axolotl quickstart (https://docs.axolotl.ai/docs/getting-started.html). This factory does not install Axolotl and does not install flash attention.

`axolotl-lora` is bf16 LoRA. It matches Axolotl `examples/llama-3/lora-1b.yml`, the file that quickstart runs. `adapter` is `lora`. `load_in_8bit` and `load_in_4bit` are both false. That example leaves both flags unset, which is the same bf16 LoRA. This card writes them false so the mode is visible in the file. `sequence_len` is 2048, `micro_batch_size` is 2, `gradient_accumulation_steps` is 2, and `lora_r` is 16, matching that file. `lora_alpha` is 32, `lora_dropout` is 0.05, `num_epochs` is 1, `optimizer` is `adamw_8bit`, `learning_rate` is 0.0002, and `warmup_ratio` is 0.1, matching that file. `sample_packing` is true. `lora_target_linear` is true. That example lists Llama-3 projections. This card targets every linear module so a Qwen or Llama train base loads without a module-list edit. `pad_to_sequence_len` stays unset, matching that file. `val_set_size` is 0.0 and `evals_per_epoch` is 0. That file uses 0.1 and 4. A short scaffold does not split an eval set, and Axolotl refuses eval settings when `val_set_size` is 0. That file sets `attn_implementation: flash_attention_2` and a Llama pad token. This card leaves both unset. The train base is not pinned to Llama-3.

`axolotl-qlora` is 4-bit QLoRA. It matches Axolotl `examples/llama-3/qlora.yml`. `load_in_8bit` is false, `load_in_4bit` is true, and `adapter` is `qlora`. `sequence_len` is 4096, `micro_batch_size` is 2, `gradient_accumulation_steps` is 4, and `lora_r` is 32, matching that file. `lora_alpha` is 16, `lora_dropout` is 0.05, `num_epochs` is 4, `optimizer` is `paged_adamw_32bit`, `learning_rate` is 0.0002, and `warmup_ratio` is 0.1, matching that file. `sample_packing` is true and `lora_target_linear` is true, matching that file. `val_set_size` is 0.0, matching that file's 0. `evals_per_epoch` is 0. That file sets 4, which asks for eval on an empty split. That file sets `attn_implementation: flash_attention_2` and a Llama pad token. This card leaves both unset.

The default recipe leaves `max_steps` unset and writes `saves_per_epoch: 1`. A short gauge run passes `--max-steps 10`. Axolotl's `max_steps` precedes `num_epochs`. When that count is under 50, prepare sets `save_steps` to the same count and omits `saves_per_epoch`. Axolotl refuses to set both. `--max-steps 0` is `refuse:max-steps`.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id-or-pack.json> \
  --driver axolotl-lora \
  --job train \
  --state-dir .cell

axolotl train .cell/enrich/<pack-id>/axolotl-lora/axolotl.yml
```

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id-or-pack.json> \
  --driver axolotl-qlora \
  --job train \
  --state-dir .cell

axolotl train .cell/enrich/<pack-id>/axolotl-qlora/axolotl.yml
```

After `axolotl train`, `estate enrich merge-adapt` prints Axolotl's `axolotl merge-lora` line for the adapter directory (`adapter_config.json`). Axolotl writes the merged Hugging Face directory to `output_dir/merged` (`config.json` and a `.safetensors` file whose name does not start with `adapter_model`). This prepare sets `output_dir` to `outputs`, so that directory is `outputs/merged`. `axolotl merge-lora` does not take `--out`. `axolotl-qlora` also prints `--dequant`, the CLI flag that writes a bf16 checkpoint. This factory does not run the merge. Axolotl does not write GGUF. `gguf-convert` prints `python3 convert_hf_to_gguf.py outputs/merged --outfile outputs/merged.gguf --outtype auto`. `local-seat` prints the `ollama create` line for that directory or the sibling `.gguf`. `import-trained` records the adapter directory, the merged directory, or the `.gguf` file. `PREPARE.md` and `NEXT.md` name that ladder. `unsloth-qlora` prints its own ladder after the operator-owned train: `merge-adapt` prints `save_pretrained_merged` with `save_method` `merged_16bit`, then `gguf-convert` and `local-seat`. `local-seat --adapter` on that card is `refuse:adapter`. `mlx-lm-lora` prints `mlx_lm.fuse` instead of this Hugging Face merge.

```bash
estate enrich merge-adapt \
  --prepared .cell/enrich/<pack-id>/axolotl-qlora \
  --adapter .cell/enrich/<pack-id>/axolotl-qlora/outputs

estate enrich gguf-convert \
  --prepared .cell/enrich/<pack-id>/axolotl-qlora \
  --weights .cell/enrich/<pack-id>/axolotl-qlora/outputs/merged

estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/axolotl-qlora \
  --weights .cell/enrich/<pack-id>/axolotl-qlora/outputs/merged
```

`unsloth-qlora` is the optional card for a faster single-GPU Nvidia QLoRA run. It does not replace LLaMA-Factory or Axolotl, and it is not the local runtime.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id-or-pack.json> \
  --driver unsloth-qlora \
  --job train \
  --state-dir .cell
```

That writes `.cell/enrich/<pack-id>/unsloth-qlora/UNSLOTH.md`, `PREPARE.md`, `NEXT.md`, and `prepare.json`. `UNSLOTH.md` is operator-owned. It records `seat_tag` and `train_base_model`. It is not an Unsloth config and not a script. `NEXT.md` names the Unsloth install page (https://unsloth.ai/docs/get-started/install), the fine-tuning guide (https://unsloth.ai/docs/get-started/fine-tuning-llms-guide), and the Linux install line the Unsloth README publishes (`uv pip install unsloth --torch-backend=auto`). This factory does not run that install and does not call Unsloth. After you train outside the factory, `estate enrich merge-adapt` prints Unsloth's documented `save_pretrained_merged` line (`save_method` `merged_16bit`) for an adapter directory that holds `adapter_config.json` and `adapter_model.safetensors`. The merged directory named in that print is `merged` beside the prepare. `gguf-convert` prints `python3 convert_hf_to_gguf.py` with `--outtype auto` and also prints the three manual lines on https://unsloth.ai/docs/basics/inference-and-deployment/saving-to-gguf (`f16`, `bf16`, `q8_0`). Unsloth's page does not publish `--outtype auto`. `local-seat` prints the Ollama line for that merged directory or for a GGUF file. `local-seat --adapter` is `refuse:adapter`. `import-trained` still records the adapter directory, the merged directory, or one `.gguf` file. This factory does not choose Unsloth save knobs and does not run those lines. `--official-scale` on this card alone is `refuse:official-scale`. `--from-feed` on this card alone is `refuse:dataset`. The card does not write `max_steps` or `dataset.jsonl`.

`mlx-lm-lora` is the optional card for LoRA on Apple Silicon. mlx-lm already documents that path. This card does not replace LLaMA-Factory, Axolotl, or Unsloth, and it is not the local runtime. The `mlx` catalog card stays a stub.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id-or-pack.json> \
  --driver mlx-lm-lora \
  --job train \
  --state-dir .cell
```

That writes `.cell/enrich/<pack-id>/mlx-lm-lora/MLX.md`, `PREPARE.md`, `NEXT.md`, and `prepare.json` when `host_class_affinity` is `apple-silicon`. `MLX.md` is operator-owned. It records `seat_tag`, `train_base_model`, and `host_class_affinity: apple-silicon`. It is not an mlx-lm config and not a script. `NEXT.md` names the public LoRA page (https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md), the install line that page publishes (`pip install "mlx-lm[train]"`), the command name `mlx_lm.lora`, and the fuse command that page publishes (`mlx_lm.fuse --model <path_to_model>`). That default loads adapters from `adapters/` and writes `fused_model/`. `mlx_lm.fuse --help` lists the flags. After `mlx_lm.lora`, the adapter directory holds `adapter_config.json` and `adapters.safetensors`. `estate enrich merge-adapt` prints `mlx_lm.fuse --model <train base> --adapter-path <adapter-dir> --save-path <prepared>/fused_model` and the same line with `--export-gguf`. mlx-lm writes `<prepared>/fused_model/ggml-model-f16.gguf`. LORA.md limits that GGUF export to Mistral, Mixtral, and Llama style models in fp16. `estate enrich local-seat --weights` points at that file. The fused directory is MLX weights. This card does not print a Hugging Face convert line for it. This factory does not run that install, does not call mlx-lm, and does not write the weights. A quantized `--model` on that page is QLoRA. This factory does not choose iterations, rank, or a data directory.

`any`, `consumer-nvidia`, `rented-nvidia`, and any other affinity are `refuse:host` and write nothing. `--all-drivers --job train` omits this card on those affinities and still writes the other cards. It prints the omission. On `apple-silicon`, `--all-drivers --job train` writes the handoff beside the recipe cards. `--official-scale` on this card alone is `refuse:official-scale`. `--from-feed` on this card alone is `refuse:dataset`. The card does not write `max_steps` or `dataset.jsonl`. After you train outside the factory, `merge-adapt` prints the fuse line, then `local-seat` prints the Ollama line for `ggml-model-f16.gguf`. `import-trained` records the adapter directory or that GGUF file. A fused MLX directory passed to `import-trained` is `refuse:adapter`. A fused MLX directory passed to `local-seat` or `gguf-convert` is `refuse:seat`. LLaMA-Factory `NEXT.md` names this card. `recipe.yaml` does not.

Each Axolotl directory holds `axolotl.yml` and an Alpaca `dataset.jsonl`. Axolotl writes the adapter under `output_dir` in the yaml. `prepare.json` stores `base_model` and `seat_tag` as the Ollama id, `train_base_model` as the value written to `base_model` in the yaml, and the same `dataset_mode` fields as the LLaMA-Factory cards. `PREPARE.md` and `NEXT.md` use the same dataset paragraph on these cards. After `axolotl train`, `estate enrich merge-adapt` prints `axolotl merge-lora`. Axolotl writes `output_dir/merged`. `gguf-convert` prints the llama.cpp line for that directory. `local-seat` prints `ollama create`. `import-trained` records the adapter, the merged directory, or a `.gguf` file. Axolotl does not write GGUF. To load the adapter without a merge, seat tag `cell-enrich-{pack_id}` on Ollama with `FROM` an Ollama model of this same train base plus `ADAPTER` for the adapter directory. The seat tag already on the cell is the id Ollama is running. This factory does not run `ollama create`.

When the pack `source_paths` are already files under the cell state directory, prepare again with `--from-feed`. For the usual pack path `feed/events.jsonl` and `--state-dir .cell`, that file is `.cell/feed/events.jsonl`.

```bash
estate enrich prepare \
  --estate <your-estate.yaml> \
  --pack <pack-id-or-pack.json> \
  --driver llamafactory-qlora \
  --job train \
  --from-feed \
  --state-dir .cell
```

`llamafactory-lora` takes the same flag and writes the same instruct rows. `axolotl-lora` and `axolotl-qlora` take the same flag and write Alpaca rows from the same files. `--from-feed` on a prepare that has no train recipe card is `refuse:dataset`. The refuse names `llamafactory-qlora`, `llamafactory-lora`, `axolotl-lora`, and `axolotl-qlora`. `unsloth-qlora` alone is that refuse. `mlx-lm-lora` alone is that refuse. `--all-drivers --job train --from-feed` hydrates the four recipe cards. One refuse writes none of the sibling directories. On an affinity other than `apple-silicon`, that set does not include `mlx-lm-lora`.

When the adapter directory or a GGUF exists, record the join. The prepared directory is the one you trained from.

```bash
estate enrich import-trained \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-lora \
  --tag cell-enrich-<pack-id> \
  --adapter .cell/enrich/<pack-id>/llamafactory-lora/outputs

estate enrich import-trained \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --tag cell-enrich-<pack-id> \
  --adapter .cell/enrich/<pack-id>/llamafactory-qlora/outputs

estate enrich import-trained \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --tag cell-enrich-<pack-id> \
  --adapter .cell/enrich/<pack-id>/llamafactory-qlora/export

estate enrich import-trained \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --tag cell-enrich-<pack-id> \
  --adapter <gguf>

estate enrich apply-proposal \
  --estate <your-estate.yaml> \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --tag cell-enrich-<pack-id> \
  --state-dir .cell

estate plan --estate .cell/enrich-stage/staged-estate.yaml --state-dir .cell
estate apply --estate .cell/enrich-stage/staged-estate.yaml --state-dir .cell --require-plan
```

`import-trained` writes the same `binding-proposal.json` as `import-prepared`, plus `trained_shape` and `trained_paths` on that proposal and on `prepare.json`, in one success path. The three shapes are an adapter directory with `adapter_config.json`, a merged directory with `config.json` and at least one `.safetensors` file whose name does not start with `adapter_model` (a `Modelfile` there is recorded), and one `.gguf` file or a directory with exactly one top-level `.gguf`. On `mlx-lm-lora` that merged-looking directory is fused MLX weights and is `refuse:adapter`. That card records the adapter directory or a GGUF file. A directory with more than one is `refuse:adapter`. `adapter_model.safetensors` is not merged evidence. Marker files are regular files inside that directory; a symlinked marker is `refuse:adapter`. The open uses `O_NOFOLLOW` and the scan reads that handle. `NEXT.md` on `llamafactory-lora` and `llamafactory-qlora` prints those three commands with the prepared directory filled in. A prepare that is not `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, `unsloth-qlora`, or `mlx-lm-lora`, a job that is not `train`, or a path that matches none of the shapes or more than one is a refuse before a new proposal exists. A publish that fails removes the partial proposal and restores the previous prepare and proposal bytes. `apply-proposal` does not apply. The source estate is written only when `estate apply --require-plan` succeeds. Point `--estate` at a lab copy. `examples/estate.yaml` on `main` stays hash-locked. There is no second apply path and no auto-promote.

```bash
make qlora-journey
```

Prints the Target C ladder and checks one `llamafactory-qlora` prepare. Uses `examples/fixtures/specialist-overnight.pack.json`. Copies `examples/estate.yaml` into `/tmp/cell-one-qlora-journey` (or `$TMPDIR`). A copy with only `params.model: llama3` is `refuse:train-base` and writes nothing. The success copy sets `params.train_base_model` to `Qwen/Qwen2.5-0.5B-Instruct`. The script asserts `recipe.yaml` (`template: qwen`, `quantization_method: bnb`, `quantization_bit: 4`, `model_name_or_path` that train base), `prepare.json` with `seat_tag` `llama3` and that train base, and `NEXT.md` lines for `llamafactory-cli train`, `llamafactory-cli export`, `gguf-convert`, `python3 convert_hf_to_gguf.py` with `--outtype auto`, `local-seat`, and `import-trained`. `gguf-convert` and `local-seat` against the missing export directory are `refuse:seat` and write no GGUF. `prepare.json` has no `trained_shape` until `import-trained` runs, and this script does not run `import-trained`. Leaves `examples/estate.yaml` unchanged. Prints `SKIP live train`. Does not run LLaMA-Factory, llama.cpp, or Ollama. Not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no.

```bash
make train-next
```

Prints the Target C train recipe from `NEXT.md` after the same `llamafactory-qlora` prepare. Uses `examples/fixtures/specialist-overnight.pack.json`. Copies `examples/estate.yaml` into `/tmp/cell-one-train-next` (or `$TMPDIR`). A copy with only `params.model: llama3` is `refuse:train-base` and writes nothing. The success copy sets `params.train_base_model` to `Qwen/Qwen2.5-0.5B-Instruct`. The script asserts `recipe.yaml` (`template: qwen`, `quantization_method: bnb`, `quantization_bit: 4`, `model_name_or_path` that train base) and `prepare.json` with `seat_tag` `llama3` and that train base. It prints the `NEXT.md` lines `pip install llamafactory`, `pip install 'bitsandbytes>=0.49'`, and `llamafactory-cli train` on the absolute `recipe.yaml` path. It also prints the `llamafactory-cli export` line and does not run it. `CELL_TRAIN_LIVE=1` stays print-only. A missing `llamafactory-cli` or bitsandbytes is an informational SKIP. `prepare.json` has no `trained_shape`. Leaves `examples/estate.yaml` unchanged. Prints `SKIP live train`. Does not run LLaMA-Factory. Not in `make smoke`, `make gate-90`, or GitHub Actions. `make uniqueness-ladder` does not run it. `READY_FOR_LIVE_TEST`: no.

```bash
make lora-journey
```

Prints the Target A ladder and checks one `llamafactory-lora` prepare. Uses `examples/fixtures/specialist-overnight.pack.json`. Copies `examples/estate.yaml` into `/tmp/cell-one-lora-journey` (or `$TMPDIR`). A copy with only `params.model: llama3` is `refuse:train-base` and writes nothing. The success copy sets `params.train_base_model` to `Qwen/Qwen2.5-0.5B-Instruct`. The script asserts `recipe.yaml` (`template: qwen`, `lora_rank: 8`, `packing: false`, no `quantization_bit`, no `quantization_method`, `model_name_or_path` that train base), `prepare.json` with `seat_tag` `llama3` and that train base, and `NEXT.md` lines for `llamafactory-cli train`, `llamafactory-cli export`, `merge-adapt`, `gguf-convert`, `python3 convert_hf_to_gguf.py` with `--outtype auto`, `local-seat`, and `import-trained`. `gguf-convert` and `local-seat` against the missing export directory are `refuse:seat` and write no GGUF. `prepare.json` has no `trained_shape` until `import-trained` runs, and this script does not run `import-trained`. Leaves `examples/estate.yaml` unchanged. Prints `SKIP live train`. Does not run LLaMA-Factory, llama.cpp, or Ollama. Not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no.

```bash
make seat-journey
```

Prints the Target C seat ladder and checks one `llamafactory-qlora` prepare, then the print path. Uses `examples/fixtures/specialist-overnight.pack.json`. Copies `examples/estate.yaml` into `/tmp/cell-one-seat-journey` (or `$TMPDIR`). A copy with only `params.model: llama3` is `refuse:train-base` and writes nothing. The success copy sets `params.train_base_model` to `Qwen/Qwen2.5-0.5B-Instruct`. The script asserts `recipe.yaml` (`template: qwen`, `quantization_method: bnb`, `quantization_bit: 4`, `model_name_or_path` that train base) and `prepare.json` with `seat_tag` `llama3` and that train base. Before the stubs, `merge-adapt` on a missing `outputs/` is `refuse:adapter`, and `gguf-convert` and `local-seat` are `refuse:seat` and write no GGUF. The script writes `outputs/adapter_config.json` and `merge-adapt` prints `llamafactory-cli export`. It then writes a 5090-shaped export (`config.json` with `model_type` `qwen2` and architecture `Qwen2ForCausalLM`, `tokenizer_config.json` with `extra_special_tokens` as a JSON list, `model.safetensors`, and no `vocab.json` or `merges.txt`). `gguf-convert` on that directory is `refuse:tokenizer` and does not print `python3 convert_hf_to_gguf.py`. The script removes that fixture and writes `export/config.json` as `{}` plus `export/model.safetensors`, and `export.gguf` whose first four bytes are `GGUF`. `gguf-convert` prints `python3 convert_hf_to_gguf.py` with `--outtype auto` and does not write the GGUF. `local-seat` prints `ollama create cell-enrich-overnight-traces -f` the sibling Modelfile, plus `llama-cli -m` and `llama-server -m`. `import-trained` records `trained_shape` `gguf` on the throwaway prepare and does not apply. Leaves `examples/estate.yaml` unchanged. Prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` does not run a trainer, a convert, or `ollama create`. Not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no.

```bash
make train-prepare
```

Uses `examples/fixtures/specialist-overnight.pack.json`. Copies `examples/estate.yaml` into `/tmp/cell-one-train-prepare` (or `$TMPDIR`). A copy with only `params.model: llama3` is `refuse:train-base` and writes nothing. The success copy also sets `params.train_base_model` to `Qwen/Qwen2.5-0.5B-Instruct`. That prepare asserts `recipe.yaml` (`template: qwen`, `quantization_method: bnb`, no `max_steps`), the chat `dataset.jsonl` in scaffold mode, the `llamafactory-cli train` line and the bitsandbytes install line in `NEXT.md`, and `prepare.json` with `job` `train`, `base_model` `llama3`, and that train base. A second prepare with `--max-steps 10` writes `max_steps` and `save_steps` 10. `--official-scale` writes `cutoff_len` 2048, `num_train_epochs` 3.0, `gradient_accumulation_steps` 8, and `warmup_ratio` 0.1 on the LoRA card and still omits quantization. The same flag with `--max-steps 10` keeps those scale fields and writes `max_steps` 10. The default recipe stays at cutoff 512 and one epoch. `export.yaml` records `merge_status: not_run` in a comment, and `NEXT.md` says this prepare did not merge. A hand-added `quantization_bit` key on `export.yaml` is `refuse:export` from `import-trained`. `--official-scale` on `ollama-modelfile` alone is `refuse:official-scale` and writes nothing. `llamafactory-lora` on that same copy writes rank 8, `packing: false`, template `qwen`, and no quantization fields. Its `NEXT.md` says the LoRA path does not require bitsandbytes and names the non-quant twin of the Qwen2.5 Instruct QLoRA prepare. A copy whose train base is `Qwen/Qwen3-4B-Instruct-2507` writes `template: qwen3_nothink` on both LLaMA-Factory cards. The QLoRA card still sets `quantization_method: bnb` and names the reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x. The LoRA card omits quantization and names the non-quant twin of that prepare. It does not get the QLoRA line. `examples/fixtures/phi3-instruct.pack.json` prepares `llamafactory-qlora` with template `phi`, `quantization_method: bnb`, `quantization_bit: 4`, seat tag `llama3`, train base `microsoft/Phi-3-mini-4k-instruct`, and the reproduce-target line in `NEXT.md` and `PREPARE.md`. `examples/fixtures/phi3-instruct-lora.pack.json` prepares `llamafactory-lora` with that same train base, template `phi`, `lora_rank` 8, `packing: false`, no quantization keys, and the LoRA-only reproduce line. The QLoRA card does not get that LoRA line. Phi-3-small, Phi-4, and Phi-4-mini do not get that line. `examples/fixtures/llama32-instruct.pack.json` prepares `llamafactory-qlora` with template `llama3`, `quantization_method: bnb`, `quantization_bit: 4`, seat tag `llama3`, train base `meta-llama/Llama-3.2-3B-Instruct`, and the reproduce-target line beside Phi-3 and Qwen in `NEXT.md` and `PREPARE.md`. `examples/fixtures/llama32-instruct-lora.pack.json` prepares `llamafactory-lora` with that same train base, template `llama3`, `lora_rank` 8, `packing: false`, no quantization keys, and the LoRA-only reproduce line. The QLoRA card does not get that LoRA line. Llama-3.2 vision, a Llama-3.2 base, and Llama-3.1 Instruct do not get that line. `examples/fixtures/gemma2-instruct.pack.json` prepares `llamafactory-qlora` with template `gemma2`, `quantization_method: bnb`, `quantization_bit: 4`, seat tag `llama3`, train base `google/gemma-2-2b-it`, and the reproduce-target line beside Phi-3, Llama-3.2, and Qwen in `NEXT.md` and `PREPARE.md`. `examples/fixtures/gemma2-instruct-lora.pack.json` prepares `llamafactory-lora` with that same train base, template `gemma2`, `lora_rank` 8, `packing: false`, no quantization keys, and the LoRA-only reproduce line. The QLoRA card does not get that LoRA line. `examples/fixtures/mistral-instruct.pack.json` prepares `llamafactory-qlora` with template `mistral`, `quantization_method: bnb`, `quantization_bit: 4`, seat tag `llama3`, train base `mistralai/Mistral-7B-Instruct-v0.3`, and the reproduce-target line beside Phi-3, Llama-3.2, Gemma-2, and Qwen in `NEXT.md` and `PREPARE.md`. `examples/fixtures/mistral-instruct-lora.pack.json` prepares `llamafactory-lora` with that same train base, template `mistral`, `lora_rank` 8, `packing: false`, no quantization keys, and the LoRA-only reproduce line. The QLoRA card does not get that LoRA line. `examples/fixtures/qwen3-instruct.pack.json` prepares `llamafactory-qlora` with template `qwen3_nothink`, `quantization_method: bnb`, `quantization_bit: 4`, seat tag `llama3`, train base `Qwen/Qwen3-4B-Instruct-2507`, and the reproduce-target line beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x in `NEXT.md` and `PREPARE.md`. `examples/fixtures/qwen3-instruct-lora.pack.json` prepares `llamafactory-lora` with that same train base, template `qwen3_nothink`, `lora_rank` 8, `packing: false`, no quantization keys, and the LoRA-only reproduce line. The QLoRA card does not get that LoRA line. `examples/fixtures/qwen25-instruct-lora.pack.json` prepares `llamafactory-lora` with template `qwen`, `lora_rank` 8, `packing: false`, no quantization keys, seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`, and the LoRA-only reproduce line. The QLoRA card does not get that LoRA line. A Qwen2.5 base, a thinking-shaped id, Qwen2, Qwen2.5-Coder, and Qwen2.5-VL do not get that line. A Qwen3 Instruct id does not get that line. `examples/fixtures/deepseek-r1-distill.pack.json` prepares `llamafactory-qlora` with template `deepseekr1`, `quantization_method: bnb`, `quantization_bit: 4`, seat tag `llama3`, train base `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`, and the QLoRA reproduce line beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, and Qwen3 Instruct. The LoRA card does not get that QLoRA line. `examples/fixtures/deepseek-r1-distill-lora.pack.json` prepares `llamafactory-lora` with that same train base, template `deepseekr1`, `lora_rank` 8, `packing: false`, no quantization keys, and the LoRA-only reproduce line. The QLoRA card does not get that LoRA line. A Qwen or Llama substring in that distill id stays `deepseekr1`. A bare `deepseek-r1` or `deepseek-r1:1.5b` stays `refuse:train-base`. `axolotl-lora` keeps an indented datasets list, sets `base_model` to the Qwen2.5 train base, and writes bf16 LoRA (`adapter: lora`, `load_in_4bit: false`, `sequence_len: 2048`, `micro_batch_size: 2`, `gradient_accumulation_steps: 2`, `lora_r: 16`, `saves_per_epoch: 1`, no `max_steps`). `prepare.json` for that card keeps `base_model` `llama3` and `seat_tag` `llama3`. `axolotl-qlora` writes 4-bit QLoRA (`adapter: qlora`, `load_in_4bit: true`, `sequence_len: 4096`, `gradient_accumulation_steps: 4`, `lora_r: 32`) with that same train base. `--official-scale` leaves `axolotl-lora` on `examples/llama-3/lora-1b.yml` (`num_epochs` 1, `gradient_accumulation_steps` 2, `sequence_len` 2048) and `axolotl-qlora` on `examples/llama-3/qlora.yml` (`adapter: qlora`, `load_in_4bit: true`, `sequence_len` 4096, `num_epochs` 4). `--max-steps 10` on `axolotl-lora` writes `max_steps` and `save_steps` 10 and omits `saves_per_epoch`. `--max-steps 0` on `axolotl-qlora` is `refuse:max-steps` and writes nothing. A seat-only copy is `refuse:train-base` for `llamafactory-lora`, `axolotl-lora`, and `axolotl-qlora` too. `--from-feed` with no `feed/events.jsonl` under the state directory is `refuse:dataset` and writes nothing. A fixture feed with one noted event and one event that has no note hydrates one instruct row on `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, and `axolotl-qlora` (`dataset_mode` `feed`, `dataset_skipped` 1). The same files left in place without `--from-feed` stay a scaffold and do not copy the note. `--all-drivers --job train` writes the train base into both `axolotl.yml` files, with `adapter: lora` on `axolotl-lora` and `adapter: qlora` on `axolotl-qlora`, writes the unquantized LoRA recipe beside the QLoRA recipe, and leaves Modelfile `FROM` as `llama3`. A stock estate is `refuse:base-model` and writes nothing. `--job enrich` is `refuse:job` and writes nothing. `import-trained` on a fixture adapter directory writes the proposal and does not apply. Leaves `examples/estate.yaml` unchanged. `unsloth-qlora` without a train base is `refuse:train-base`. With the train base it writes `UNSLOTH.md` and does not write a script. `--official-scale` on that card is `refuse:official-scale`. The overnight pack affinity is `any`, so `mlx-lm-lora` on that pack is `refuse:host` and `--all-drivers` omits it. A copy with `host_class_affinity` `apple-silicon` writes `MLX.md` and does not write a script. Prints `SKIP live train`. Does not run LLaMA-Factory, Axolotl, Unsloth, or mlx-lm. Not in `make smoke`, `make gate-90`, or GitHub Actions.

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
| `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, `unsloth-qlora`, or `mlx-lm-lora` with `--job enrich` | `refuse:job` |
| `mlx-lm-lora` when `host_class_affinity` is not `apple-silicon` | `refuse:host` |
| A train source path is empty, `--from-feed` has nothing to read, a source line is not an instruct row, a source resolves outside the cell directory, or the sources together exceed the byte cap | `refuse:dataset` |
| `import-trained` on a prepare that is not `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, `unsloth-qlora`, or `mlx-lm-lora` with job `train` | `refuse:driver` or `refuse:job` |
| `import-trained` path is missing, is not one of adapter `output_dir` / merged `export_dir` / GGUF, matches more than one of those shapes, uses `adapter_model*.safetensors` as the only weight next to `config.json`, is a symlinked marker, or is a directory with more than one top-level `.gguf` | `refuse:adapter` |
| `merge-adapt` prepare is not `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, or `mlx-lm-lora` with job `train`, or `--adapter` is missing `adapter_config.json`, is a merged export, is a GGUF, is a symlink, or the Axolotl yaml disagrees with the card | `refuse:driver`, `refuse:job`, `refuse:adapter`, or `refuse:merge` |
| `merge-adapt` on `mlx-lm-lora` when the host is not `apple-silicon`, `MLX.md` disagrees, the adapter is missing `adapters.safetensors`, or the adapter is a PEFT `adapter_model` file | `refuse:host`, `refuse:train-base`, or `refuse:adapter` |
| `gguf-convert` or `local-seat` prepare is not `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, or (for a GGUF seat) `mlx-lm-lora` with job `train` | `refuse:driver` or `refuse:job` |
| `mlx-lm-lora` `gguf-convert`, or `local-seat` on the fused MLX directory | `refuse:seat` (seat the `ggml-model-f16.gguf` file; this factory does not print a Hugging Face convert line) |
| `mlx-lm-lora` `import-trained` on a fused MLX directory (`config.json` and a `.safetensors` file whose name does not start with `adapter_model`, including one that also holds `ggml-model-f16.gguf`) | `refuse:adapter` (record the adapter directory or the GGUF file; this card does not record `trained_shape` `merged`) |
| `mlx-lm-lora` `local-seat --adapter` | `refuse:adapter` |
| `gguf-convert` weights are missing, are not a merged export, are an adapter directory, are only `adapter_model*.safetensors`, are only an `export.yaml` card, are a symlink, are already a GGUF, or match more than one shape | `refuse:seat` |
| `gguf-convert` `tokenizer_config.json` has `extra_special_tokens` as a JSON list or JSON null (or another non-object), is a symlink, is not JSON, or a Qwen-family export is missing `vocab.json` or `merges.txt` | `refuse:tokenizer` (copy tokenizer files from the HF cache snapshot for the train base already on disk, or the equivalent base checkout, into the export directory; keep the export `tokenizer_config.json` as `tokenizer_config.json.bak`; then re-run `estate enrich gguf-convert`; this factory does not download or copy them) |
| `{state_dir}/enrich` is missing on list | `refuse:enrich-index` |
| `prepare.json` missing, unreadable, or flagged promoted | `refuse:missing-prepare`, `refuse:prepare-unreadable`, `refuse:prepared` |
| Tag is not `cell-enrich-{pack_id}` | `refuse:tag` |
| Operator path is missing or not a file | `refuse:path` |
| Estate has no `local_slm` seat | `refuse:binding` |
| `FROM` would be a binding id, a driver id, or empty | `refuse:base-model` |
| `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, `unsloth-qlora`, or `mlx-lm-lora` has no train base, or the value is a bare Ollama tag or a seat-looking local leaf, or `model_name_or_path` / `axolotl.yml` `base_model` / `UNSLOTH.md` `train_base_model` / `MLX.md` `train_base_model` does not match `prepare.json` `train_base_model` | `refuse:train-base` |
| `--max-steps 0` | `refuse:max-steps` |
| `binding-proposal.json` is missing | `refuse:missing-proposal` |
| Proposal schema, flags, or curator are wrong | `refuse:proposal` or `refuse:curator` |
| `prepare.json` does not match the proposal | `refuse:prepare` |
| Proposal hash does not match the estate | `refuse:estate-hash` |
| A different pending stage is already on disk | `refuse:stage` |
| `--verify-local-tag` and the tag is not seated, or the endpoint is unset | `refuse:local-tag` |

Those prepare stops happen before the output directory is created. `--all-drivers` stages every card first, so one refuse leaves no sibling directory from that call. List does not create `.cell/enrich`. Import writes the proposal only after the gates pass. `apply-proposal` writes `enrich-stage/` only after its gates pass. Same tag and binding again is a no-op and does not rewrite the stage.

## Local seat after LLaMA-Factory export

`llamafactory-cli export` writes the merged directory named by `export_dir` in `export.yaml`. Current LLaMA-Factory `export_model` also writes `Modelfile` in that directory. The file starts with `FROM .` and carries TEMPLATE from the train chat template (`template.get_ollama_modelfile`). This factory does not write that Modelfile and does not invent a second template.

`prepare.json` records `export_yaml` when prepare wrote `export.yaml`, and `modelfile` when prepare wrote `Modelfile`. The `ollama-modelfile` card records `modelfile`. The LLaMA-Factory cards record `export_yaml`. The Modelfile that export writes later, inside `export_dir`, is not the prepare-time `modelfile` field.

Seat the merged weights on the local runtime outside the factory:

1. Export with the `llamafactory-cli export` line in `NEXT.md`.
2. `estate enrich gguf-convert` checks that merged directory and prints the llama.cpp line. Run that line from a llama.cpp checkout. `--outtype auto` is `convert_hf_to_gguf.py`'s default (highest-fidelity 16-bit float, f16 or bf16). The outfile is a sibling of the export directory (`export.gguf` beside `export`). This factory does not run the script and does not choose a quantization type. Before that line, check `tokenizer_config.json` in the export directory. A JSON list under `extra_special_tokens` makes transformers raise `AttributeError: 'list' object has no attribute 'keys'` inside `convert_hf_to_gguf.py`. JSON null under that key is the same `refuse:tokenizer` case: transformers calls `.keys()` on that non-object value. A Qwen-family export can also be missing `vocab.json` and `merges.txt`, which the Qwen2.5 train-base tokenizer includes. Copy the tokenizer files from the train base already on disk into the export directory. The source is the HF cache snapshot for that repo, or the equivalent base checkout (a local HF directory). Keep the export `tokenizer_config.json` as `tokenizer_config.json.bak`. Then re-run `estate enrich gguf-convert` on that export directory. `gguf-convert` returns `refuse:tokenizer` for that list, for JSON null, and when `config.json` `model_type` or `architectures`, or `tokenizer_class`, names Qwen and either BPE file is missing. An object `extra_special_tokens` with those two files present still prints the convert line. This factory does not download the tokenizer files and does not copy them.
3. `ollama create` uses FROM the GGUF, or the merged directory when LLaMA-Factory wrote the Modelfile.

```bash
estate enrich gguf-convert \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export
```

The report prints:

```bash
python3 convert_hf_to_gguf.py <export-dir> --outfile <export-dir-sibling>.gguf --outtype auto
```

Then it prints `estate enrich local-seat` with `--weights` pointing at that sibling file. The command does not write the GGUF.

`estate enrich local-seat` validates the directory or the GGUF and prints the create line. The create name is `cell-enrich-{pack_id}`. The seat tag is `prepare.json` `seat_tag` (the same string as `base_model`). A GGUF also prints `llama-cli -m <file>` and `llama-server -m <file> --port 8080`. `--runtime llama.cpp` selects those lines and still prints `ollama create`. A merged directory does not get a llama.cpp load line; the convert line stays first. The command does not create the model, does not shell out, and does not promote.

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export
```

Point `--weights` at a `.gguf` file after the llama.cpp convert. The command prints a Modelfile whose FROM is that file. When a LLaMA-Factory Modelfile sits in the same directory, TEMPLATE and PARAMETER lines are copied into the printed text. The bytes on disk stay as they were.

`import-trained` records the same path on the `local_slm` proposal and writes `trained_shape` and `trained_paths`. A merged export_dir is `config.json` and at least one `.safetensors` file whose name does not start with `adapter_model`, and a Modelfile there is part of that shape. A GGUF is a `.gguf` file. An adapter `output_dir` (`adapter_config.json`) is the `--adapter` print on `local-seat` (`FROM` the seat tag, `ADAPTER` that directory). `--weights` still refuses that directory, and it refuses `config.json` plus only `adapter_model*.safetensors`. A merged export or a GGUF passed to `--adapter` is `refuse:adapter`. A symlinked weights path, a symlinked adapter path, or a symlinked marker is `refuse:seat` or `refuse:adapter`. The seat tag on the proposal stays the prepare seat tag. `import-trained` does not apply and does not promote.

`PREPARE.md` and `NEXT.md` on `llamafactory-lora` and `llamafactory-qlora` carry this chain.

`axolotl-lora` and `axolotl-qlora` print `estate enrich merge-adapt` after train. That command prints `axolotl merge-lora`. Axolotl writes `outputs/merged`. `--weights` on the later commands is that directory, or a `.gguf` file for `local-seat`. Axolotl does not write GGUF. `PREPARE.md` and `NEXT.md` on those cards name the ladder: `axolotl train`, `merge-adapt`, `gguf-convert`, `local-seat`, `import-trained`.

Page: [`local-seat.md`](local-seat.md). `READY_FOR_LIVE_TEST`: no.

## Status and doctor

`estate status` and `estate doctor` read `{state_dir}/enrich/{pack}/{driver}/prepare.json` when that directory exists. A missing directory is silence. They do not invent a prepare count. An empty directory says that no `prepare.json` is present.

A present file prints one line: pack, driver, job (`enrich` or `train`), `seat_tag` and the train base when those fields are on the file, `trained_shape` when `import-trained` recorded it, and the out path. That line is the prepare record. It does not mean this factory trained, merged, converted, or seated the model.

A file that does not parse, or that fails the prepare schema, refuses before the status page. Doctor FAILs that file before `factory ready`. A symlink in the tree is `refuse:enrich-index`. The open uses `O_NOFOLLOW` and the opened file must stay inside the cell state directory.

Catalog lines on the same page name each in-tree card: `integration`, `portable`, or `optional`, with `live=false`. `mlx-lm-lora` is optional, beside `unsloth-qlora`. A prepare probe does not run. `READY_FOR_LIVE_TEST`: no.

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
