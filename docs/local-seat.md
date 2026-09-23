# Local seat after a LLaMA-Factory export

Operator page for seating a merged SLM on the local runtime. Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). Prepare: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walk: [`operator-enrich-journeys.md`](operator-enrich-journeys.md).

Ollama already creates a model from a Modelfile. llama.cpp already converts a Hugging Face directory to GGUF. LLaMA-Factory already writes the merged directory and a Modelfile. This factory prints the next command. It does not invent a converter and it does not shell out.

`READY_FOR_LIVE_TEST`: no. The command does not promote.

## Chain

1. `estate enrich prepare --driver llamafactory-lora` or `llamafactory-qlora` writes `export.yaml`. `prepare.json` records that path as `export_yaml`. The Ollama seat tag stays `base_model` and `seat_tag`. The train base stays `model_name_or_path`.
2. On the CUDA host, run the `llamafactory-cli export` line from `NEXT.md`. `export_dir` in that file is the merged directory (`config.json` and at least one `.safetensors` file). Current LLaMA-Factory `export_model` writes `Modelfile` in that directory. The file starts with `FROM .` and carries TEMPLATE from the train chat template (`get_ollama_modelfile`). This factory does not write that Modelfile.
3. When you want a GGUF, `estate enrich gguf-convert` prints the llama.cpp line for that directory. Run it from a llama.cpp checkout:

```bash
python3 convert_hf_to_gguf.py <merged-dir> --outfile <sibling>.gguf --outtype auto
```

`--outtype auto` is `convert_hf_to_gguf.py`'s default (highest-fidelity 16-bit float, f16 or bf16). The outfile is a sibling of the merged directory (`export.gguf` beside `export`). A `.gguf` inside the merged directory makes that directory match two shapes. This factory does not run the script and does not choose a quantization type. `q8_0`, `tq1_0`, and `tq2_0` stay off this card. `llama-quantize` is a later llama.cpp tool. This factory does not print a quant command.
4. Seat with Ollama. `ollama create` uses FROM the GGUF, or the merged directory when LLaMA-Factory wrote the Modelfile.
5. `estate enrich import-trained` records that same path on the `local_slm` proposal. `estate enrich apply-proposal`, then `estate plan` and `estate apply --require-plan`, stay the join.

`PREPARE.md` and `NEXT.md` on both LLaMA-Factory cards repeat this chain.

## Convert

```bash
estate enrich gguf-convert \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export
```

`--prepared` is the directory that holds `prepare.json` for `llamafactory-lora` or `llamafactory-qlora` with job `train`. `--weights` is the merged export directory. An adapter directory, a symlink, or a path that is already a GGUF is `refuse:seat`. The command writes nothing.

## Command

```bash
estate enrich local-seat \
  --prepared .cell/enrich/<pack-id>/llamafactory-qlora \
  --weights .cell/enrich/<pack-id>/llamafactory-qlora/export
```

`--prepared` is the directory that holds `prepare.json` for `llamafactory-lora` or `llamafactory-qlora` with job `train`. `--weights` is the merged export directory or a `.gguf` file.

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

A GGUF directory that holds one `.gguf` file is the same shape. Point `--weights` at the file when the directory also holds other shapes.

The report also prints the `import-trained` line for the same path. A merged export_dir is `config.json` and at least one `.safetensors` file whose name does not start with `adapter_model`. `adapter_model-00001-of-00002.safetensors` is an adapter shard, not merged evidence. A Modelfile in that directory is part of that shape. A GGUF is a `.gguf` file. The seat tag on the proposal stays the prepare seat tag. `import-trained` records `trained_shape` and `trained_paths`. `import-trained` does not apply and does not promote. A symlinked `--weights` path, and a symlinked marker (`config.json`, `Modelfile`, `.gguf`, `.safetensors`), is `refuse:seat`.

## Refuses

| Weights | Stop |
| --- | --- |
| Missing path | `refuse:seat` |
| Empty directory | `refuse:seat` |
| `config.json` without a `.safetensors` file whose name does not start with `adapter_model` | `refuse:seat` |
| `config.json` plus only `adapter_model*.safetensors` | `refuse:seat` (not a merged export) |
| `.safetensors` without `config.json` | `refuse:seat` |
| Adapter directory (`adapter_config.json`) | `refuse:seat` (record it with `import-trained`) |
| Symlinked `--weights`, or a symlinked marker (`config.json`, `Modelfile`, `.gguf`, `.safetensors`) | `refuse:seat` |
| More than one of adapter, merged, and GGUF | `refuse:seat` |
| More than one `.gguf` file in a directory | `refuse:seat` |
| File that is not `.gguf`, or a `.gguf` that does not start with GGUF magic | `refuse:seat` |
| Modelfile with no `FROM` line, empty, or not utf-8 | `refuse:modelfile` |
| Prepare driver is not `llamafactory-lora` or `llamafactory-qlora` | `refuse:driver` |
| Job is not `train` | `refuse:job` |
| `seat_tag` missing, or different from `base_model` | `refuse:seat` or `refuse:prepare` |
| `promoted`, `auto_apply`, or `estate_rewritten` is true | `refuse:prepared` |
| Sacred token or hardware SKU | `refuse:sacred` or `refuse:sku-banned` |
| Raw secret in the Modelfile | `refuse:raw-secret` |

The command writes nothing. `promoted`, `auto_apply`, and `estate_rewritten` stay false.
