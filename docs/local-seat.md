# Local seat after a LLaMA-Factory export

Operator page for seating a merged SLM on the local runtime. Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). Prepare: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walk: [`operator-enrich-journeys.md`](operator-enrich-journeys.md).

Ollama already creates a model from a Modelfile. llama.cpp already converts a Hugging Face directory to GGUF. LLaMA-Factory already writes the merged directory and a Modelfile. This factory prints the next command. It does not invent a converter and it does not shell out. The Qwen QLoRA ladder that reaches this page is Target C in [`operator-enrich-journeys.md`](operator-enrich-journeys.md) (section 8): prepare, the `NEXT.md` train and export lines, `gguf-convert`, `local-seat`, then `import-trained`. The Qwen LoRA ladder is Target A (section 9): prepare, the `NEXT.md` train and export lines, `merge-adapt`, `gguf-convert`, `local-seat`, then `import-trained`. `make seat-journey` (section 10) asserts `refuse:tokenizer` on a 5090-shaped export, then prints that convert, seat, and import once the good fixture stubs stand in for the merged export and the GGUF. It does not convert and does not create a model.

`READY_FOR_LIVE_TEST`: no. The command does not promote.

## Chain

1. `estate enrich prepare --driver llamafactory-lora` or `llamafactory-qlora` writes `export.yaml`. `prepare.json` records that path as `export_yaml`. The Ollama seat tag stays `base_model` and `seat_tag`. The train base stays `model_name_or_path`.
2. `estate enrich merge-adapt` prints the `llamafactory-cli export` line for `export.yaml`. The yaml shape is `examples/merge_lora/qwen3_lora_sft.yaml`. On the CUDA host, run that line. `export_dir` in that file is the merged directory (`config.json` and at least one `.safetensors` file). Current LLaMA-Factory `export_model` writes `Modelfile` in that directory. The file starts with `FROM .` and carries TEMPLATE from the train chat template (`get_ollama_modelfile`). This factory does not write that Modelfile and does not run the export.
3. When you want a GGUF, `estate enrich gguf-convert` prints the llama.cpp line for that directory. Run it from a llama.cpp checkout:

```bash
python3 convert_hf_to_gguf.py <merged-dir> --outfile <sibling>.gguf --outtype auto
```

`--outtype auto` is `convert_hf_to_gguf.py`'s default (highest-fidelity 16-bit float, f16 or bf16). The outfile is a sibling of the merged directory (`export.gguf` beside `export`). A `.gguf` inside the merged directory makes that directory match two shapes. This factory does not run the script and does not choose a quantization type. `q8_0`, `tq1_0`, and `tq2_0` stay off this card. `llama-quantize` is a later llama.cpp tool. This factory does not print a quant command.

A live 5090 prove on 2026-09-23 ran that convert on a LLaMA-Factory export of `Qwen/Qwen2.5-0.5B-Instruct`. `tokenizer_config.json` had `extra_special_tokens` as a list, and transformers raised `AttributeError: 'list' object has no attribute 'keys'`. The export also omitted `vocab.json` and `merges.txt`. Restoring the tokenizer files from the HF cache snapshot already on disk, and keeping the export `tokenizer_config.json` as `tokenizer_config.json.bak`, let the convert write a 949M BF16 GGUF. A later copy the same day used plain `cp -a` from that HF hub snapshot and left `tokenizer_config.json` as a symlink into the HF cache. `gguf-convert` returned `refuse:tokenizer` because enrich does not follow a symlinked `tokenizer_config.json`. `cp -aL` put real files in the export directory. `estate enrich gguf-convert` returns `refuse:tokenizer` for that list, for JSON null under `extra_special_tokens`, and when the export is Qwen-family (`config.json` `model_type` or `architectures`, or `tokenizer_class`) and `vocab.json` or `merges.txt` is missing. JSON null is a non-object: transformers calls `.keys()` on that value. Copy those files from the HF cache snapshot already on disk, or the equivalent base checkout, into the export directory. HF hub snapshots are often symlinks into the HF cache. Copy with dereference (`cp -aL` or `cp --dereference`, or the equivalent) so the files in the export directory are real files, not symlinks. A plain `cp -a` leaves `tokenizer_config.json` as a symlink. Enrich does not follow a symlinked `tokenizer_config.json`. Then re-run `estate enrich gguf-convert` on that directory. This factory does not download them and does not copy them. An object `extra_special_tokens` with those two files present still prints the convert line.
4. Seat with Ollama. `ollama create` uses FROM the GGUF, or the merged directory when LLaMA-Factory wrote the Modelfile. A GGUF also prints `llama-cli -m` and `llama-server -m` for that file. A merged directory is not a llama.cpp seat until the convert writes the sibling GGUF.
5. After that GGUF `local-seat` print, run the printed `ollama create` line yourself. The same step stands when you already ran `ollama create` outside this factory. This factory did not run `ollama create`. The standing next step is `estate enrich import-trained` for that GGUF. `trained_shape` is `gguf`. The proposal stays `auto_apply=false`. `import-trained` does not apply the estate. `estate enrich apply-proposal`, then `estate plan` and `estate apply --require-plan`, stay the join.

`PREPARE.md` and `NEXT.md` on both LLaMA-Factory cards repeat this chain.

## Convert

```bash
estate enrich gguf-convert \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export
```

`--prepared` is the directory that holds `prepare.json` for `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, or `unsloth-qlora` with job `train`. `--weights` is the merged export directory. An adapter directory, a directory that only holds `export.yaml`, a symlink, or a path that is already a GGUF is `refuse:seat`. On `unsloth-qlora` the report also prints the three manual convert lines from Unsloth's saving-to-gguf page (`--outtype f16`, `bf16`, and `q8_0`). The factory card line stays `--outtype auto`. Unsloth's page does not publish `--outtype auto`. The command writes nothing.

## Command

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export
```

`--prepared` is the directory that holds `prepare.json` for `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, `unsloth-qlora`, or `mlx-lm-lora` with job `train`. `--weights` is the merged export directory or a `.gguf` file. `unsloth-qlora` seats the merged 16-bit directory or one `.gguf` file. `mlx-lm-lora` seats one `.gguf` file. A fused MLX directory is `refuse:seat`. `import-trained` on this card records the adapter directory or that GGUF file. The fused directory is `refuse:adapter` there.

The report names:

- `seat_tag`, the Ollama id this cell already runs (`prepare.json` `seat_tag`, the same string as `base_model`)
- `local_tag`, the create name `cell-enrich-{pack_id}`
- `shape`, `merged` or `gguf`

When the merged directory contains `Modelfile`, the report prints:

```bash
ollama create cell-enrich-<pack-id> -f <export>/Modelfile
```

`modelfile_on_disk=true` means that file is already the one to pass. `FROM .` names the merged directory.

When `--weights` is a `.gguf` file, the report prints a Modelfile whose FROM is that file, then the `ollama create` line. When a LLaMA-Factory Modelfile is in the same directory, TEMPLATE and PARAMETER lines are copied into the printed text. The file on disk is left as it was. Save the printed text, then run the create line. This factory does not invent a chat template.

The same GGUF report also prints the documented llama.cpp lines for that file:

```bash
llama-cli -m <gguf>
llama-server -m <gguf> --port 8080
```

`llama-cli -m` and `llama-server -m` are the programs ggml-org/llama.cpp documents for an existing GGUF. `--port 8080` is that server example's port. llama.cpp does not read the Modelfile. Ollama stays the default print, so the `ollama create` line stays in the report. `--runtime llama.cpp` prints the llama.cpp lines first and still prints the Ollama line. A directory that holds one `.gguf` file names that file, not the directory. This factory does not run `llama-cli` or `llama-server`.

A GGUF directory that holds one `.gguf` file is the same shape. Point `--weights` at the file when the directory also holds other shapes.

The report also prints the `import-trained` line for the same path. On a GGUF that line is the standing next step, after you run the printed `ollama create` line yourself. The same step stands when you already ran `ollama create` outside this factory. This factory did not run `ollama create`. `trained_shape` is `gguf`. The proposal stays `auto_apply=false`. `import-trained` does not apply the estate and does not promote. A merged export_dir is `config.json` and at least one `.safetensors` file whose name does not start with `adapter_model`. `adapter_model-00001-of-00002.safetensors` is an adapter shard, not merged evidence. A Modelfile in that directory is part of that shape. A GGUF is a `.gguf` file. The seat tag on the proposal stays the prepare seat tag. `import-trained` records `trained_shape` and `trained_paths`. A symlinked `--weights` path, and a symlinked marker (`config.json`, `Modelfile`, `.gguf`, `.safetensors`), is `refuse:seat`.

## Axolotl

`axolotl-lora` and `axolotl-qlora` use the same seat commands after `estate enrich merge-adapt`. That print names `axolotl merge-lora`. Axolotl writes `outputs/merged`. `--weights` is that Hugging Face directory, or a `.gguf` file for `local-seat`. Axolotl does not write a Modelfile and does not write GGUF. `PREPARE.md` and `NEXT.md` on those cards name the ladder: `axolotl train`, `merge-adapt`, `gguf-convert`, `local-seat`, `import-trained`.

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

## Adapter seat without a merge

`--adapter` prints the no-merge seat for an adapter `output_dir`. That directory holds `adapter_config.json` (the same marker `import-trained` accepts for `trained_shape=adapter`) and the adapter weights when the train wrote them (`adapter_model.safetensors`, `adapter_model.bin`, or `adapter_model*.safetensors`).

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --adapter .cell/enrich/<pack-id>/llamafactory-qlora/outputs
```

llama.cpp does not load this adapter directory in one line. The print stays the Ollama `ADAPTER` Modelfile. `--runtime llama.cpp` with `--adapter` is `refuse:runtime`. This factory does not convert the adapter.

The report shape is `adapter`. It prints a Modelfile:

```
FROM <seat_tag>
ADAPTER <adapter-directory>
```

`FROM` is `prepare.json` `seat_tag` (the same string as `base_model`). That Ollama model must already be the train base. `ADAPTER` is the adapter directory. The report then prints `ollama create cell-enrich-<pack-id> -f <directory>/Modelfile`. The command does not write that file, does not run the line, does not merge, and does not promote.

`--weights` still refuses this directory (`refuse:seat`). A merged export or a GGUF passed to `--adapter` is `refuse:adapter`. A symlinked `--adapter` path, or a symlinked marker (`adapter_config.json`, `adapter_model*.safetensors`, `Modelfile`), is `refuse:adapter`. The open uses `O_NOFOLLOW`.

## Refuses

| Weights | Stop |
| --- | --- |
| Missing path | `refuse:seat` |
| Empty directory | `refuse:seat` |
| `config.json` without a `.safetensors` file whose name does not start with `adapter_model` | `refuse:seat` |
| `config.json` plus only `adapter_model*.safetensors` | `refuse:seat` (not a merged export) |
| `.safetensors` without `config.json` | `refuse:seat` |
| Adapter directory on `--weights` (`adapter_config.json`) | `refuse:seat` (pass `--adapter` to print the no-merge seat) |
| `--adapter` path missing `adapter_config.json` | `refuse:adapter` |
| `--adapter` path is a merged export or a GGUF | `refuse:adapter` (pass `--weights`) |
| Symlinked `--adapter`, or a symlinked adapter marker | `refuse:adapter` |
| Symlinked `--weights`, or a symlinked marker (`config.json`, `Modelfile`, `.gguf`, `.safetensors`) | `refuse:seat` |
| More than one of adapter, merged, and GGUF | `refuse:seat` |
| More than one `.gguf` file in a directory | `refuse:seat` |
| File that is not `.gguf`, or a `.gguf` that does not start with GGUF magic | `refuse:seat` |
| `--runtime` other than `ollama` or `llama.cpp` (`llama-cpp` and `llamacpp` are the `llama.cpp` aliases) | `refuse:runtime` (after the shape checks) |
| `--runtime llama.cpp` with `--adapter` | `refuse:runtime` (llama.cpp does not load an adapter directory in one line) |
| Modelfile with no `FROM` line, empty, or not utf-8 | `refuse:modelfile` |
| Prepare driver is not `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, `unsloth-qlora`, or `mlx-lm-lora` | `refuse:driver` |
| `unsloth-qlora` `--weights` is the merged 16-bit directory or one `.gguf` file | prints `ollama create`; a GGUF also prints `llama-cli -m` |
| `unsloth-qlora` `--adapter` | `refuse:adapter` (Unsloth seats a GGUF, not an Ollama adapter line) |
| `unsloth-qlora` `--weights` is the PEFT directory | `refuse:seat` (does not point at `--adapter`) |
| `unsloth-qlora` missing `UNSLOTH.md` or train base, or a symlinked `UNSLOTH.md` | `refuse:train-base` |
| `mlx-lm-lora` `--weights` is one `.gguf` file | prints `ollama create` and the llama.cpp lines for that file |
| `mlx-lm-lora` `--weights` is a fused directory (`config.json` and `model.safetensors`) | `refuse:seat` (MLX weights; seat `ggml-model-f16.gguf`) |
| `mlx-lm-lora` `--adapter` | `refuse:adapter` (not an Ollama `ADAPTER` directory) |
| `mlx-lm-lora` `host_class_affinity` is not `apple-silicon` | `refuse:host` |
| Job is not `train` | `refuse:job` |
| `seat_tag` missing, or different from `base_model` | `refuse:seat` or `refuse:prepare` |
| `promoted`, `auto_apply`, or `estate_rewritten` is true | `refuse:prepared` |
| Sacred token or hardware SKU | `refuse:sacred` or `refuse:sku-banned` |
| Raw secret in the Modelfile | `refuse:raw-secret` |

The command writes nothing. `promoted`, `auto_apply`, and `estate_rewritten` stay false.
