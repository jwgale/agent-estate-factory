# Changelog

Local wrap: `make smoke`. Hosted CI is compile-only (`cargo check --workspace --locked` on pull_request). Day 0–90 is on `main`.

## This slice — print the no-merge adapter seat

- `estate enrich local-seat --adapter <output_dir>` prints a Modelfile for an adapter directory (`adapter_config.json`, the same marker `import-trained` accepts for `trained_shape=adapter`, plus adapter weights when the train wrote them). `FROM` is `prepare.json` `seat_tag` (the same string as `base_model`). `ADAPTER` is that directory. The report then prints `ollama create cell-enrich-{pack} -f <dir>/Modelfile`. The command does not write the file, does not run the line, does not merge, and does not promote.
- `--weights` stays the merged export or GGUF path and still refuses an adapter directory (`refuse:seat`). A merged export or a GGUF passed to `--adapter` is `refuse:adapter`. A missing `adapter_config.json` is `refuse:adapter`. A symlinked adapter path or a symlinked marker is refused with `O_NOFOLLOW`, the same way `local-seat` and `import-trained` already refuse symlinks.
- `PREPARE.md` and `NEXT.md` on `llamafactory-lora` and `llamafactory-qlora` name this print beside the merged and GGUF seat. `READY_FOR_LIVE_TEST`: no.

## This slice — Axolotl convert and seat print path

- `estate enrich gguf-convert` and `estate enrich local-seat` accept an `axolotl-lora` or `axolotl-qlora` train prepare plus a merged Hugging Face directory (`config.json` and a `.safetensors` file whose name does not start with `adapter_model`). `gguf-convert` prints `python3 convert_hf_to_gguf.py <dir> --outfile <sibling>.gguf --outtype auto`. `local-seat` prints the same `ollama create` line it prints for a LLaMA-Factory export, including a sibling `.gguf`. Neither command shells out, writes a GGUF, or promotes.
- The operator owns the merge into that directory. In-tree Axolotl docs name `axolotl train` and do not name a merge command, so `PREPARE.md` and `NEXT.md` do not invent one. Axolotl does not write GGUF. The ladder is train, operator merge, `gguf-convert`, `local-seat`, `import-trained`.
- An adapter directory, `config.json` plus only `adapter_model*.safetensors`, a directory that only holds `export.yaml`, and a symlink stay `refuse:seat`. `unsloth-qlora` and `mlx-lm-lora` stay `refuse:driver`. Sacred, SKU, job, and promoted refuses are unchanged.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Target C Qwen QLoRA operator journey

- The popular path is one ladder of commands that already exist: `estate enrich prepare --driver llamafactory-qlora` (seat tag separate from the train base), the `NEXT.md` `llamafactory-cli train` and `llamafactory-cli export` lines, `estate enrich gguf-convert` (prints `python3 convert_hf_to_gguf.py` with `--outtype auto`), `estate enrich local-seat` (prints `ollama create`), and `estate enrich import-trained` (records `trained_shape` and `trained_paths`). A 5090 smoke seated `llama3` and trained `Qwen/Qwen2.5-0.5B-Instruct`. `template` is `qwen`. A missing train base is `refuse:train-base`.
- The walk is section 8 of `docs/operator-enrich-journeys.md`. `docs/TRAIN-ENRICH.md` and `estate help enrich` / `estate help train` print the same ladder.
- `make qlora-journey` prints that ladder and checks the prepare artifacts on a throwaway estate copy. A missing export is `refuse:seat` and writes no GGUF. The script does not run LLaMA-Factory, llama.cpp, or Ollama, and it does not promote. It is not in `make smoke`, `make gate-90`, or GitHub Actions.
- `READY_FOR_LIVE_TEST`: no.

## This slice — status and doctor report the enrich prepare tree

- `estate status` and `estate doctor` read `{state_dir}/enrich` when that directory is present. Each `prepare.json` prints pack, driver, job, `seat_tag` and `train_base` when those fields are present, `trained_shape` when `import-trained` recorded it, and the out path. The line is the prepare record. The factory did not train, merge, convert, or seat that model.
- A missing enrich directory stays silent. Status and doctor do not invent a prepare count or claim zero packs. An empty directory notes that no `prepare.json` is present.
- A `prepare.json` that does not parse, or whose schema or required fields fail, refuses before the status page and FAILs doctor before `factory ready`. A symlink in the enrich tree is `refuse:enrich-index`. The walk opens `prepare.json` with `O_NOFOLLOW` and keeps the opened file inside the cell state directory.
- Train catalog lines print the in-tree card status (`integration`, `optional`, or `portable`) with `live=false`. `mlx-lm-lora` is optional, the same way `unsloth-qlora` is. A prepare probe is not live. `READY_FOR_LIVE_TEST`: no.

## This slice — optional mlx-lm LoRA handoff

- `mlx-lm-lora` is an optional `TrainEnrichDriver` card (`status=optional`). `estate enrich prepare --driver mlx-lm-lora` writes `MLX.md`, `PREPARE.md`, `NEXT.md`, and `prepare.json` when `host_class_affinity` is `apple-silicon`. `MLX.md` is an operator-owned handoff. It records the Ollama seat tag, the train base, and `host_class_affinity: apple-silicon`. It is not an mlx-lm config and not a training script.
- The card is the Apple Silicon LoRA handoff. It is not the product, and it does not make the `mlx` local-runtime card live. `NEXT.md` points at the public mlx-lm LoRA page (https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md), the install line that page publishes (`pip install "mlx-lm[train]"`), the command name `mlx_lm.lora`, and the fuse command that page publishes (`mlx_lm.fuse --model <path_to_model>`). This factory does not run that install, does not call mlx-lm, does not choose ranks or iterations, and does not shell out.
- Another affinity (`any`, `consumer-nvidia`, `rented-nvidia`, or any other string) is `refuse:host` and writes nothing. `--all-drivers` omits this card unless the affinity is `apple-silicon`, and it prints that omission. The other train cards still prepare. A missing train base, a bare Ollama tag, or a seat-looking local leaf is `refuse:train-base` and writes nothing. `import-trained` accepts the prepare and refuses when `MLX.md` `train_base_model`, `seat_tag`, or `host_class_affinity` does not match `prepare.json`, and when that affinity is not `apple-silicon`.
- This card does not write `dataset.jsonl`, a YAML recipe, or `max_steps`. `--from-feed` on this card alone is `refuse:dataset`. `--official-scale` on this card alone is `refuse:official-scale`. `--max-steps 0` is still `refuse:max-steps`. `--job enrich` is `refuse:job`. LLaMA-Factory `recipe.yaml` is unchanged. `NEXT.md` on the LLaMA-Factory and Axolotl cards names this handoff. `gguf-convert` and `local-seat` still read a LLaMA-Factory prepare.
- Deliberately not invented: an MLX script, a recipe DSL, ranks, iterations, a dataset writer, a download, a process spawn, or a trainer call. `READY_FOR_LIVE_TEST`: no.

## This slice — optional Unsloth QLoRA handoff

- `unsloth-qlora` is an optional `TrainEnrichDriver` card (`status=optional`). `estate enrich prepare --driver unsloth-qlora` writes `UNSLOTH.md`, `PREPARE.md`, `NEXT.md`, and `prepare.json`. `UNSLOTH.md` is an operator-owned handoff. It records the Ollama seat tag and the train base. It is not an Unsloth config and not a training script.
- The card is the Nvidia-only QLoRA alternate for a faster single-GPU run. It is not the product. Portable local runtimes stay swappable. `NEXT.md` points at the Unsloth install page, the fine-tuning guide, and the README install line `uv pip install unsloth --torch-backend=auto`. This factory does not run that install, does not call Unsloth, and does not shell out.
- A missing train base, a bare Ollama tag, or a seat-looking local leaf is `refuse:train-base` and writes nothing. `import-trained` accepts the prepare and refuses when `UNSLOTH.md` `train_base_model` or `seat_tag` does not match `prepare.json`.
- This card does not write `dataset.jsonl`, a YAML recipe, or `max_steps`. `--from-feed` on this card alone is `refuse:dataset`. `--official-scale` on this card alone is `refuse:official-scale`. `--max-steps 0` is still `refuse:max-steps`. `--job enrich` is `refuse:job`. `--all-drivers --job train` writes the handoff beside the recipe cards. The enrich default skips it.
- Deliberately not invented: an Unsloth script, a recipe DSL, ranks, sequence length, save knobs, a dataset writer, an official-scale cutoff, a download, or a trainer call. `READY_FOR_LIVE_TEST`: no.

## This slice — print the llama.cpp GGUF convert line

- `estate enrich gguf-convert --prepared <dir> --weights <merged-export-dir>` checks a `llamafactory-lora` or `llamafactory-qlora` train prepare and prints the llama.cpp line `python3 convert_hf_to_gguf.py <dir> --outfile <sibling>.gguf --outtype auto`. `--outtype auto` is that script's default (highest-fidelity 16-bit float). The outfile is a sibling of the merged directory. The report then prints `estate enrich local-seat` for that sibling file. The command does not convert, does not shell out, and does not write a GGUF.
- An adapter directory, `config.json` plus only `adapter_model*.safetensors`, a symlinked `--weights` path, a symlinked marker, a path that is already a GGUF, and a directory that matches more than one shape are `refuse:seat`. The same classify path as `local-seat` decides those stops. A GGUF points at `local-seat`. `refuse:train-base`, `--from-feed`, and `import-trained` are unchanged.
- `PREPARE.md` and `NEXT.md` on `llamafactory-lora` and `llamafactory-qlora` name `gguf-convert` and the same convert line. `local-seat` on a merged directory prints that line too. `READY_FOR_LIVE_TEST`: no.

## This slice — official SFT scale and an honest merge card

- `--official-scale` on `estate enrich prepare` and `estate enrich from-pack` applies to LLaMA-Factory cards only. It writes the `examples/train_lora/qwen3_lora_sft.yaml` scale into `llamafactory-lora` and `llamafactory-qlora`: `cutoff_len` 2048, `num_train_epochs` 3.0, `gradient_accumulation_steps` 8, `warmup_ratio` 0.1. Rank, packing, and quantization stay on the selected card. `axolotl-lora` and `axolotl-qlora` stay on their example files (`examples/llama-3/lora-1b.yml` and `examples/llama-3/qlora.yml`). Omit the flag for the short LLaMA-Factory recipe (`cutoff_len` 512, one epoch, grad accum 4, warmup 0.03). `--max-steps` still overrides `num_train_epochs` and still lowers `save_steps` when the count is under 50.
- `--official-scale` with no train recipe card is `refuse:official-scale` and writes nothing. `--all-drivers --job train` still prepares the Modelfile and the external manifest.
- `export.yaml` stays the merge card. `adapter_name_or_path` equals recipe `output_dir`. Comments record `merge_status: not_run`. `NEXT.md` and `PREPARE.md` say this prepare did not merge and that `llamafactory-cli export` has not run. An early stop can leave the adapter under `checkpoint-<step>`. Point `adapter_name_or_path` at that directory. Prepare does not rewrite `export.yaml` after train. A Modelfile written into the export directory by `llamafactory-cli export` belongs to that tool.
- `import-trained` refuses `export.yaml` when a real key `quantization_bit` or `quantization_method` is set (`refuse:export`). A comment line does not trip that refuse. Retargeting `adapter_name_or_path` at a checkpoint directory stays allowed. The file still omits quantization when prepare writes it.
- The factory does not run `llamafactory-cli` and does not merge. `READY_FOR_LIVE_TEST`: no.

## This slice — import-trained records the artifact shape

- `estate enrich import-trained` accepts three shapes from a `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, or `axolotl-qlora` train prepare: an adapter `output_dir` (`adapter_config.json`, plus adapter weights when they are there), a merged `export_dir` (`config.json` and at least one `.safetensors` file, optional `Modelfile`), or a `.gguf` file. A file that is not GGUF, a directory that matches none of those shapes, or a directory that matches more than one, is `refuse:adapter` and writes no proposal.
- `prepare.json` and `binding-proposal.json` record `trained_shape` and `trained_paths` for the shape that was accepted. `promoted`, `auto_apply`, and `estate_rewritten` stay false. The command does not rewrite the estate and does not promote. Curator stays `jason`. Sacred, SKU, and frontier-without-a-binding still refuse.
- `NEXT.md` on `llamafactory-lora` and `llamafactory-qlora` prints the import-trained command for the recipe `outputs/` directory, the `export/` directory, and a GGUF path. `estate help enrich` shows the same command for both cards with those directories filled in under `.cell/enrich`.
- A merged export counts `.safetensors` files whose names do not start with `adapter_model`. `config.json` plus `adapter_model.safetensors` and no `adapter_config.json` is `refuse:adapter`.
- Marker files are regular files. `import-trained` opens each one with `O_NOFOLLOW` and pins that handle inside the artifact directory, the same spirit as `--from-feed`. A symlinked marker or a symlinked `--adapter` path is `refuse:adapter`. The Modelfile and the primary file are read from those handles. The suite does not reproduce a concurrent swap; a path that becomes a symlink before the open fails closed.
- Classify and scan finish before any write. The proposal and the `prepare.json` trained fields publish together. A failed publish restores the previous prepare and proposal bytes and removes a partial proposal, so apply cannot accept an import that omitted `trained_shape` and `trained_paths`.
- A GGUF directory with more than one top-level `.gguf` is `refuse:adapter`. Point `--adapter` at one `.gguf` file, or at a directory that holds exactly one. A `.gguf` in a subdirectory does not count.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Axolotl LoRA and QLoRA prepare

- `axolotl-lora` writes bf16 LoRA. `axolotl.yml` sets `adapter: lora`, `load_in_8bit: false`, and `load_in_4bit: false`. `sequence_len` is 2048, `micro_batch_size` is 2, `gradient_accumulation_steps` is 2, and `lora_r` is 16, matching Axolotl `examples/llama-3/lora-1b.yml`. `lora_alpha` is 32, `num_epochs` is 1, and `optimizer` is `adamw_8bit`, matching that file.
- `axolotl-qlora` is the 4-bit card. `axolotl.yml` sets `load_in_8bit: false`, `load_in_4bit: true`, and `adapter: qlora`. `sequence_len` is 4096, `micro_batch_size` is 2, `gradient_accumulation_steps` is 4, and `lora_r` is 32, matching Axolotl `examples/llama-3/qlora.yml`. `lora_alpha` is 16, `num_epochs` is 4, and `optimizer` is `paged_adamw_32bit`, matching that file.
- Both cards write `base_model` from the train base. `lora_target_linear` is true. `sample_packing` is true. `val_set_size` is 0.0 and `evals_per_epoch` is 0 so a short scaffold does not ask Axolotl to eval an empty split. Flash attention and a Llama pad token stay unset. The QLoRA note names upstream `attn_implementation: flash_attention_2` and leaves that field unset. The default recipe leaves `max_steps` unset and writes `saves_per_epoch: 1`.
- `--max-steps 10` writes `max_steps` and `save_steps` and omits `saves_per_epoch`. Axolotl refuses those two save fields together, and `max_steps` precedes `num_epochs`. `--max-steps 0` is `refuse:max-steps`.
- `--from-feed` applies to `axolotl-lora` and `axolotl-qlora` the same way it applies to `llamafactory-lora` and `llamafactory-qlora`. A prepare with no train recipe card refuses and names those four drivers.
- `NEXT.md` names `axolotl train` and the Axolotl quickstart. The factory does not run Axolotl and does not install a GPU stack. `refuse:train-base`, sacred, SKU, and frontier are unchanged. `READY_FOR_LIVE_TEST`: no.

## This slice — local seat after LLaMA-Factory export

- `estate enrich local-seat` validates a merged export directory (`config.json` and at least one `.safetensors` file, optional `Modelfile`) or a `.gguf` file and prints the `ollama create` line for `cell-enrich-{pack}`. The seat tag is `prepare.json` `seat_tag`. A GGUF prints a Modelfile whose `FROM` is that file, copying TEMPLATE lines when a LLaMA-Factory Modelfile is in the same directory. The command does not write, does not shell out to ollama or llama.cpp, and does not promote.
- `prepare.json` records `export_yaml` when prepare wrote `export.yaml`, and `modelfile` when prepare wrote `Modelfile`. `PREPARE.md` and `NEXT.md` on `llamafactory-lora` and `llamafactory-qlora` name the chain: export, optional llama.cpp `convert_hf_to_gguf.py`, then `ollama create` FROM the GGUF or FROM the merged directory when LLaMA-Factory wrote the Modelfile. `import-trained` records that same merged directory or GGUF and writes `trained_shape` and `trained_paths`. An adapter directory stays on `import-trained`.
- A `.safetensors` name that starts with `adapter_model` is not merged evidence. `config.json` plus `adapter_model-00001-of-00002.safetensors` and no other merged weight is `refuse:seat` and does not print `ollama create`. A symlinked `--weights` path or a symlinked marker (`config.json`, `Modelfile`, `.gguf`, `.safetensors`) is `refuse:seat`. Opens use `O_NOFOLLOW`.
- Printed paths are single-quoted when they contain whitespace, a newline, a quote, or any of `;`, `|`, `&`, `<`, `>`, `(`, `)`, `!`, `*`, `?`. The command still does not run.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Phi-3 Instruct QLoRA reproduce target

- `llamafactory-qlora` infers LLaMA-Factory template `phi` for `microsoft/Phi-3-mini*`, `microsoft/Phi-3-medium*`, and Phi-3.5 (`microsoft/Phi-3.5-mini-instruct`, `microsoft/Phi-3.5-MoE-instruct`), including those ids as nested path segments and HF cache directories (`models--microsoft--Phi-3-mini-4k-instruct`). Phi-3-small infers `phi_small`. Phi-4 infers `phi4`. Phi-4-mini infers `phi4_mini`. The names follow LLaMA-Factory `register_model_group` in `constants.py`.
- The QLoRA card still writes `quantization_method: bnb` and `quantization_bit: 4`. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Phi repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as a reproduce target beside Qwen LoRA/QLoRA.
- `examples/fixtures/phi3-instruct.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `microsoft/Phi-3-mini-4k-instruct`). Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — LLaMA-Factory LoRA without quantization

- `llamafactory-lora` is a `TrainEnrichDriver` card beside `llamafactory-qlora`. `estate enrich prepare --driver llamafactory-lora` writes the same files (`recipe.yaml`, `export.yaml`, `dataset_info.json`, instruct chat `dataset.jsonl`). The recipe is SFT LoRA: `finetuning_type: lora`, no `quantization_bit`, no `quantization_method`, `lora_rank: 8`, `lora_alpha: 16`, `packing: false`. Rank 8 matches LLaMA-Factory `examples/train_lora/qwen3_lora_sft.yaml`. `cutoff_len` stays 512. `NEXT.md` names the official longer values (`cutoff_len` 2048, `num_train_epochs` 3.0, `gradient_accumulation_steps` 8, `warmup_ratio` 0.1).
- Select QLoRA with `--driver llamafactory-qlora`. That card still writes `quantization_bit: 4`, `quantization_method: bnb`, and rank 16, and `NEXT.md` still installs `pip install 'bitsandbytes>=0.49'`. The LoRA card's `NEXT.md` says that path does not require bitsandbytes.
- `template` is inferred by scanning path segments of the train base on both LLaMA-Factory cards, starting at the last segment. A leaf such as `weights` or an HF snapshot hash uses the nearest ancestor that names a family. A Qwen3 name that contains `instruct` and does not contain `thinking`, or that contains `nothink`, uses `qwen3_nothink` (`Qwen/Qwen3-4B-Instruct-2507`). Other Qwen3 names use `qwen3`. Older Qwen names stay `qwen`.
- `--from-feed` applies to `llamafactory-lora` the same way it applies to `llamafactory-qlora` and `axolotl-lora`. Default prepare still writes a scaffold (or a stub when `source_paths` is empty). `PREPARE.md` and `NEXT.md` on the LoRA card use that dataset honesty note and name `refuse:dataset`.
- `export.yaml` still omits quantization on both cards. `refuse:train-base` and the seat-tag versus train-base split are unchanged, and they apply to `llamafactory-lora`. `import-trained` accepts that prepare. `--all-drivers --job train` writes the LoRA directory beside QLoRA.
- The factory does not run `llamafactory-cli`. `READY_FOR_LIVE_TEST`: no.

## This slice — `--from-feed` checks the raw record, pins the opened file, and caps bytes

- A line with `kind` or `object_class` is classified before ShareGPT or Alpaca dispatch. A frontier event wrapped as `messages` or `instruction` is `refuse:frontier-invent` when the estate has no frontier binding. Sacred, SKU, and raw-secret checks see the raw record, including fields that are not copied into the row.
- Prepare opens each source and reads that same file handle. The opened file must stay inside the cell state directory. A path that cannot be pinned is `refuse:dataset`.
- Each source is at most 8 MiB. All sources together are at most 8 MiB. Each chat and Alpaca copy is at most 16 MiB. A total over that cap is `refuse:dataset`.
- `READY_FOR_LIVE_TEST`: no.

## This slice — dataset scaffold is honest, and `--from-feed` copies rows already on disk

- Default `llamafactory-qlora` and `axolotl-lora` prepare still writes a scaffold `dataset.jsonl` (or a three-row stub when `source_paths` is empty). `prepare.json` records `dataset_mode`, `dataset_rows`, `dataset_from_feed` false, `dataset_skipped`, and `dataset_read_paths`. `PREPARE.md` and `NEXT.md` on both cards say those rows are not training data and name `refuse:dataset`.
- `--from-feed` reads pack `source_paths` under `--state-dir` and copies instruct rows. ShareGPT `messages`, Alpaca `instruction` / `output`, and a scrubbed feed event with a `note` are the shapes it accepts. Events with no note are skipped. A missing file, a path outside the cell directory, or a line that is not an instruct row is `refuse:dataset` and writes nothing. Sacred, SKU, raw secret, and frontier-without-a-binding still refuse. This factory does not download pack sources and does not train.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Axolotl train base matches the seat split

- `axolotl-lora` writes `base_model` in `axolotl.yml` from the train base (pack `train_base_model`, or `params.train_base_model` on the local binding; the pack wins). That value is a Hugging Face repo id (`namespace/name`) or a local directory of HF weights. A relative directory is stored as an absolute path.
- The Ollama seat tag stays `base_model` and `seat_tag` in `prepare.json`, and in `NEXT.md`, for Modelfile `FROM` and the adapter join. `--all-drivers --job train` writes that train base into `axolotl.yml` and leaves Modelfile `FROM` as the seat tag.
- A missing train base, a bare Ollama tag, or a local path whose directory name is an Ollama seat tag is `refuse:train-base` and writes nothing. `import-trained` refuses the same gap, and refuses when `axolotl.yml` `base_model` does not match `prepare.json`. This factory does not map a seat tag onto a Hub repo and does not download weights.
- Unsloth stays a `NEXT.md` pointer. `READY_FOR_LIVE_TEST`: no.

## This slice — absolute local train base

- A relative LLaMA-Factory train base (`./…` or `../…`) is stored as an absolute path in `recipe.yaml`, `export.yaml`, `prepare.json`, and `NEXT.md`. A Hugging Face repo id stays as typed. The weights directory does not need to exist at prepare time.
- A local path whose directory name is an Ollama seat tag (`./llama3`, `../llama3`) is `refuse:train-base` and writes nothing.
- `READY_FOR_LIVE_TEST`: no.

## This slice — seat tag and LLaMA-Factory train base

- `llamafactory-qlora` keeps the Ollama seat tag (`prepare.json` `base_model` / `seat_tag`, Modelfile `FROM`) separate from the train base (`model_name_or_path`). Set pack `train_base_model` or `params.train_base_model` on the local binding to a Hugging Face repo id (`namespace/name`) or a local directory of HF weights. The pack field wins. `template` is inferred from the train base.
- A missing train base, or a bare Ollama tag such as `llama3`, is `refuse:train-base` and writes nothing. `import-trained` refuses the same gap, and refuses when `recipe.yaml` or `export.yaml` `model_name_or_path` does not match `prepare.json`. This factory does not map a seat tag onto a Hub repo.
- QLoRA install notes name `pip install 'bitsandbytes>=0.49'` because `pip install llamafactory` and `llamafactory[torch,metrics]` 0.9.5 did not pull it. A 5090 smoke used torch `2.11.0+cu128` and bitsandbytes 0.50.2. That install did not replace torch.
- `--max-steps N` writes a gauge recipe. The default recipe stays `num_train_epochs: 1.0`, `save_steps: 50`, and leaves `max_steps` unset. `quantization_method` stays `bnb`.
- `READY_FOR_LIVE_TEST`: no.

## This slice — LLaMA-Factory QLoRA method token

- `llamafactory-qlora` writes `quantization_method: bnb`. LLaMA-Factory 0.9 selects the 4-bit bitsandbytes branch only for that token.
- `READY_FOR_LIVE_TEST`: no.

## This slice — LLaMA-Factory QLoRA is the train card

- `llamafactory-qlora` is the primary train `TrainEnrichDriver`. `estate enrich prepare --driver llamafactory-qlora` writes `recipe.yaml` (SFT QLoRA, 4-bit, LoRA rank 16, `cutoff_len` 512, packing on), `export.yaml`, `dataset_info.json`, instruct chat `dataset.jsonl`, `PREPARE.md`, `NEXT.md` with `pip install llamafactory`, `llamafactory-cli train`, and `llamafactory-cli export`, and `prepare.json` (`job: train`). The factory does not run the CLI, does not download weights, and does not call CUDA.
- `axolotl-lora` stays the YAML recipe card for a config-driven or multi-GPU run. It still does not run Axolotl.
- Unsloth QLoRA is a `NEXT.md` pointer on the LLaMA-Factory card (Nvidia only). It is not a registered driver.
- `import-trained` accepts either train recipe and writes the same `local_slm` proposal. Ollama seating stays outside the factory. Merge drops `quantization_bit`. GGUF conversion stays with llama.cpp after that merge.
- Train hosts are `consumer-nvidia` and `rented-nvidia`. `apple-silicon` can prepare. `NEXT.md` says the card expects CUDA LLaMA-Factory. `make train-prepare` prints `SKIP live train` and is off smoke, `gate-90`, and Actions.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Unsloth QLoRA is the train card

- `unsloth-qlora` is the primary train `TrainEnrichDriver`. `estate enrich prepare --driver unsloth-qlora` writes `train_unsloth.py` (QLoRA 4-bit, LoRA r=16, `max_seq_length` 512), instruct chat `dataset.jsonl`, `PREPARE.md`, `NEXT.md` with `pip install unsloth` and `python train_unsloth.py`, and `prepare.json` (`job: train`). The factory does not run the script, does not install Unsloth, and does not export GGUF.
- `axolotl-lora` stays the YAML recipe card for a config-driven or multi-GPU run. It still does not run Axolotl.
- `import-trained` accepts either train recipe and writes the same `local_slm` proposal. Ollama seating stays outside the factory. GGUF and Ollama export stay on Unsloth's docs (`save_pretrained_gguf`).
- Train hosts are `consumer-nvidia` and `rented-nvidia`. `apple-silicon` can prepare. `NEXT.md` says the card expects CUDA. `make train-prepare` prints `SKIP live train` and is off smoke, `gate-90`, and Actions.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Axolotl train recipe

- `axolotl-lora` is a `TrainEnrichDriver` card beside `ollama-modelfile` and `external-manifest`. `estate enrich prepare --driver axolotl-lora` writes `axolotl.yml` (QLoRA: `load_in_4bit` and `adapter: qlora`), `dataset.jsonl`, `PREPARE.md`, `NEXT.md` with the exact `axolotl train` line, and `prepare.json` (`job: train`). The factory does not run Axolotl, does not download a dataset, and does not rewrite `estate.yaml`.
- Default job for that card is `train`. `--job enrich` is `refuse:job` and writes nothing. `--all-drivers` includes the card when the job is `train`. The enrich default still prepares the other two cards.
- `estate enrich import-trained` checks an adapter directory or a merged GGUF and writes the same `local_slm` binding proposal as `import-prepared`. `apply-proposal`, `plan`, and `apply --require-plan` stay the join. Ollama stays the local-run seat.
- Train hosts for the card are `consumer-nvidia` and `rented-nvidia`. `apple-silicon` can prepare; `NEXT.md` says the Axolotl GPU path expects CUDA. No MLX trainer.
- Opt-in `make train-prepare` asserts the recipe, the `axolotl train` line, `prepare.json` `job=train`, and an untouched `examples/estate.yaml`. It prints `SKIP live train`. Off smoke, `gate-90`, and Actions.
- `READY_FOR_LIVE_TEST`: no.

## This slice — seated FROM, from-pack, live prove

- Keeps `estate enrich apply-proposal`. The proposal still stages `{state}/enrich-stage/staged-estate.yaml` for `estate plan` and `estate apply --require-plan`. The source estate is written only when that apply succeeds. Status and doctor still name a pending join. `auto_apply` stays false.
- `ollama-modelfile` writes `FROM` as the seated model. That is `params.model` on the local binding, or a pack `model_hint` that is already a model tag (`llama3`). The binding id `local_slm` is not a model tag. A missing seated name is `refuse:base-model` and writes nothing. Sacred, SKU, curator, and frontier-invent still refuse before write.
- `estate enrich from-pack` prepares an accepted pack (id in `packs/` or `packs/accepted/`, or a pack JSON path) into `{state_dir}/enrich`. Omitting `--driver` prepares every card. Same refuses. Does not apply, train, or rewrite `estate.yaml`.
- Opt-in `make enrich-live-prove` copies the example estate into `/tmp/cell-one-enrich-live-prove`, sets `params.model` from the seat, runs `from-pack`, `ollama create`, `ollama show`, and `import-prepared`, then removes the tag. `examples/estate.yaml` stays untouched. Seat down prints `SKIP` and exits 0. Not in smoke, `gate-90`, or Actions. A hardware SKU anywhere in the output path still refuses.
- No third train/enrich card. llama.cpp already reads an INI preset, and that preset needs a GGUF path this estate does not carry.
- `READY_FOR_LIVE_TEST`: no. The live script is an opt-in seated-runtime enrich handoff. It is not a factory-wide live test.

## Day 0–30 (PR #1)

One-box factory proving A1–A4. Horizon / Research / Sanctum on separate lanes. Sacred exclusions (Cyera CI, Rust classroom) fail closed. Isolation is a driver (profile-dir today). No live provider required. Dual PE, vault, multi-box, and AI-gateway stay out of altitude.

## Day 31–60 (on `main` with #1)

Mixed model estate A7–A9. Equal-class frontier (`grok-4.7`) + local. Ollama-first, llama.cpp swap-proof. Fail-closed when local is down — no silent `grok-4.7` fallback. Hardware is a driver (`consumer-nvidia` / `apple-silicon` / `rented-nvidia`), not a product fork. GitHub is the only source of truth.

## Day 61–90 (PR #2)

Beachhead toward A10–A12. Feed packs never auto-promote. Cloud-agent is declared, not spawned. Overnight waves added convey mesh, dry-run apply, lease TTL, doctor, smoke, dual-layer sacred file, and the GATE-90 checklist. Curator is Jason / manual.

## PR #3 — live probes and the day90 loop

Optional `estate probes --live` pings a specialist endpoint when you set `CELL_*`. Unset endpoints print SKIP and exit 0. CI never needs a Mac or a GPU. `make day90` walks status → plan → dry-run → apply → reconcile on an isolated cell. Probe ids refuse hardware SKUs the same way bindings do.

## PR #4 — suggest and accept stay instructions-only

`estate reconcile --suggest` writes a patch file. It does not rewrite leases. `estate packs accept --curator jason` writes enrich-pack edit instructions. It does not rewrite `estate.yaml`. Wrong curator refuses. Jason still pastes by hand.

## PR #5 — doctor --strict and a thin gate-90

`estate doctor --strict` is the pre-merge operator check: compile-only CI body, locked sacred file, dual-layer demo, refuse fixtures, floor has no vendor needles. `make gate-90` is a thin local alias (smoke + day90 + that checklist). The dual-layer demo keeps Sanctum first-class; Sanctum is not Cyera. Omitting a locked sacred id from the overlay file still refuses Cyera CI.

## PR #6 — feed-loop and honest parking

README leads with `make gate-90` as the Day-90 operator entrypoint. `make feed-loop` walks scrubbed traces → pack → propose → accept on an isolated cell. The feed cursor stays on disk; rematerialize does not auto-promote. Placement-actual JSON round-trips every reconcile refuse code. `docs/DAY90-PLUS.md` parks live Mac MLX, live GPU, and cloud-spawn until Jason has boxes. Those rows are not green.

## PR #7 — help, backup prune, convey policy

`estate help [topic]` prints Day-90 pages for status, plan, apply, reconcile, feed-loop, and backup. Unknown topics refuse. `estate backup --prune N` keeps the newest N archives; `N=0` refuses. Convey `call` refuses on `policy-deny.yaml`.

## PR #8 — layout, local-only honesty, operator runbook

`docs/cell-layout.md` matches the paths the code writes. `docs/OPERATOR-DAY.md` walks `make gate-90` → `make feed-loop` → `estate backup --prune` on isolated cells. A hardening test locks `make gate-90` off Actions (it wraps `cargo test --workspace`). No leftover Origin URLs. No new `estate version` command.

## PR #9 — dual-layer-demo e2e and Cell One snapshot

Isolated dual-layer-demo loop: validate → plan → dry-run → apply → status → reconcile → backup → prune. README cross-links OPERATOR-DAY and FEED-LOOP. Dead leftover `ops.rs` wrappers removed. Snapshot: `docs/CELL-ONE-STATUS.md`.

## PR #10 — lease-refresh after expire --forget

Hole: `estate expire --forget` dropped leases, then apply treated that as `refuse:drift` and demanded `--force`. Apply now restamps (`lease-refresh`). Isolated TTL e2e on `examples/fixtures/ttl-short.yaml`. Not a new verb.

## PR #11 — sacred overlay e2e

Isolated sacred overlay e2e: `sacred-omit-locked.yaml` (`locked: []`) still refuses `cyera-ci` / `rust-classroom` on convey hop. `lab-notebook` refuses only with the overlay installed. Dual-layer-demo still validates. No new verb.

## After PR #11 (this slice)

- Hole: `estate convey expire --forget` dropped hop decls with the leases, then call returned `refuse:no-lease`. Forget now keeps decls; call restamps (`lease-refresh`). Mirrors placement apply after forget. Not a new verb.
- Isolated hop TTL e2e: declare `ttl-secs: 1` → expire lists → call `refuse:expired` → `--forget` → call restamps. No JSON mutation.
- Pause-kit still holds after an unchanged apply → suspend → resume.

## After PR #12 (this slice)

- Hole: `estate restore` treated empty/missing backup `sacred_ids` as a match and could write. Empty set is now `refuse:sacred-mismatch` (fail closed). Dry-run and live restore both write nothing.
- Packs propose → accept → promote still leaves `promoted=false` and does not write the estate. `reconcile --suggest` on drift still does not rewrite leases. Vanilla `doctor` stays thinner than `--strict`.

## After PR #13 (PR #14)

No new hole. `plan diff --allow-wider` / `plan export-pr` exits locked. `apply --dry-run` under refuse writes nothing (snapshot covers conveyor/sessions too). Curator: wrong → `refuse:curator`; accept missing flag is clap; import still defaults to jason. CELL-ONE-STATUS states #10–#13 in plain English.

## This slice — apply-proposal stages the local_slm join

- `estate enrich apply-proposal` reads `binding-proposal.json`. It checks schema, curator, sacred, hardware, frontier, and `prepare.json`. It writes `{state}/enrich-stage/staged-estate.yaml` and `stage.json` (`cell-one.enrich-binding-stage.v0`). That staged file is the input for the existing `estate plan` and `estate apply --require-plan`. The command does not apply. The source estate stays unchanged until that require-plan apply succeeds.
- The same tag and binding again is a no-op. A missing proposal, a prepare mismatch, a wrong tag, a stale estate hash, or a different pending stage refuses before any stage write.
- `--verify-local-tag` is off by default. When set, the seated runtime must list the tag (`GET /v1/models` or `/api/tags`). A failed probe is `refuse:local-tag` before any stage write. No new local server.
- `estate status` and `estate doctor` name a pending enrich join when `.cell/enrich` holds a prepare or proposal and `local_slm` is still unbound. An unreadable proposal, prepare, or stage refuses before the page. A pending note is not a factory-ready failure.
- Opt-in `make enrich-prepare` walks prepare, list, import-prepared, apply-proposal, plan, and require-plan apply on a throwaway lab copy. `examples/estate.yaml` stays unchanged. Off smoke, gate-90, and Actions.
- llama.cpp stays on the existing OpenAI-compat seat. `--verify-local-tag` uses that probe. `READY_FOR_LIVE_TEST`: no.

## This slice — prepare, list, import-prepared

- `estate enrich prepare --all-drivers` writes every `TrainEnrichDriver` card into sibling directories. One refuse writes none of them. Each directory gains `NEXT.md`: artifact paths, the exact handoff (`ollama create … -f <Modelfile>` or the external manifest files), the `import-prepared` line, and fail-closed reminders. Prepare still does not shell out, train, POST, promote, or rewrite `estate.yaml`.
- `estate enrich list` reads `{state_dir}/enrich/{pack}/{driver}/prepare.json` and prints pack, driver, job, tag, and out path. A missing directory is `refuse:enrich-index`. List does not create it and does not write.
- `estate enrich import-prepared` checks `prepare.json`, the tag `cell-enrich-{pack_id}`, and an operator file. It writes `binding-proposal.json` and `binding-proposal.md` (`cell-one.enrich-binding-proposal.v0`) for the existing `local_slm` seat. Paste the snippet, then `estate plan` and `estate apply --require-plan`. Sacred, SKU, curator, missing path, and frontier-invent refuse before that proposal exists. `auto_apply` stays false.
- Opt-in `make enrich-prepare` walks prepare, list, and import on a throwaway directory. Not in smoke, gate-90, or Actions.
- `READY_FOR_LIVE_TEST`: no.

## This slice — spine links

- North star, ubiquitous language, charter, and README point at the prepare page [`docs/TRAIN-ENRICH.md`](docs/TRAIN-ENRICH.md) and the walks [`docs/operator-enrich-journeys.md`](docs/operator-enrich-journeys.md). `estate help enrich` names the journeys page. No new command. No trainer. `READY_FOR_LIVE_TEST`: no.

## This slice

- Train/enrich beachhead. `TrainEnrichDriver` in `model-estate`. Drivers: `ollama-modelfile` (Modelfile + `ollama create` steps) and `external-manifest` (JSON/YAML). `estate enrich prepare` writes artifacts under `.cell/enrich/` or `--out`. Sacred, SKU, curator, missing pack, and frontier-invent refuse before write. Does not train, POST, auto-promote, or rewrite `estate.yaml`.
- `estate help enrich` (alias `train`). Opt-in `make enrich-prepare`. Not in smoke, gate-90, or Actions.
- Schema [`schema/train-enrich.v0.json`](schema/train-enrich.v0.json) (`cell-one.enrich-prepare.v0`). Docs: [`docs/TRAIN-ENRICH.md`](docs/TRAIN-ENRICH.md).
- `READY_FOR_LIVE_TEST`: no.

## After PR #85

- Canonical glossary: [`docs/UBIQUITOUS_LANGUAGE.md`](docs/UBIQUITOUS_LANGUAGE.md). A local runtime is an ecosystem seat. Ollama is today's entrant; catalog/route/bind take the next process. Suite goal: facilitate train/enrich of purpose-built small models. Beachhead today: enrich packs and the specialist path. No training stack. Anti-shrink stays gateway, Ollama wrapper-as-product, LM Studio-alone, Grok Bot clone.
- `estate help north-star` / `charter` match that page. README and [`docs/NORTH-STAR.md`](docs/NORTH-STAR.md) use the same words.
- `READY_FOR_LIVE_TEST`: no.

## This slice — operator enrich journeys

- [`docs/operator-enrich-journeys.md`](docs/operator-enrich-journeys.md) walks train/enrich facilitation: an Ollama Modelfile and `ollama create` after pack accept, a later entrant on the same `local_slm` id, and an external manifest for a trainer outside the factory. Fail-closed stops stay sacred, SKU, `model.local.down`, and no auto-promote. Integrate-vs-invent stays the build rule. No new command. No training stack. `READY_FOR_LIVE_TEST`: no.

## After PR #83

- `estate help north-star` (alias `northstar`) and `estate help charter` print the locked product sentence, short anti-shrink bullets, and pointers to `charter.md`, `make gate-90`, `make day90`, and `docs/LIVE-PROBES.md`.
- Unknown help topics still refuse. Not on smoke or `gate-90`. No Actions change. `READY_FOR_LIVE_TEST`: no.

## After PR #84

- Vision reset. README leads with the one-box north-star. [`docs/NORTH-STAR.md`](docs/NORTH-STAR.md) is the one-page product story. Experimental catalog cards (MLX / vLLM / TRT), cloud-agent spawn, and the lease-bound hop stub sit under "Parked / not the product". Packs stay curator edit instructions. Control does not complete.
- Charter status line: Day 0–90 is on `main`. Day 90+ is real-world proof plus parked stubs. Locked defaults are unchanged.
- Opt-in `make real-world` (`scripts/real-world.sh`): north-star line, `cargo check --workspace --locked`, vanilla `estate doctor` on the checkout that holds `examples/estate.yaml`, then live probes and the Ollama specialist only when `CELL_LOCAL_ENDPOINT` is set. Unset prints SKIP and exits 0. Not in smoke, gate-90, or Actions. Does not print the frontier API key. Does not invent PASS.
- Mac specialist complete is recorded on the MacBook Air against tip `2ab78a4`: `"completion": "Pong"`, reason `compat completion`. `READY_FOR_LIVE_TEST` for that command is no. Frontier `pong` and the 5090 `Pong` stay as already recorded. Native MLX stays a stub.
- `READY_FOR_LIVE_TEST`: no.

## After PR #82

- `estate convey leases` refuses before it prints hop lease JSON when a cloud-mesh hop lease is spawned. An unspawned file still prints. A missing mesh still says there are no hop leases. The mesh is not rewritten.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #81

- `estate convey expire` refuses before it lists or forgets an expired cloud hop lease that is spawned. `expire --forget` does not drop that row.
- A missing mesh is not a spawned lease. An expired box hop still drops when that cloud row is not in the drop.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #80

- `estate convey declare` refuses before it writes a cloud hop lease when that placement lease is spawned. It does not record `spawned: false`.
- A missing placement file is not a spawned lease. An unspawned cloud hop still declares as not spawned. The placement file is not rewritten.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #79

- `estate convey call` does not say a cloud hop is not spawned when the placement lease is spawned. A missing hop lease is not restamped to `spawned: false`.
- A missing placement file is not a spawned lease. An unspawned cloud hop still refuses as declared, not spawned.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #78

- `estate convey sync` refuses before it writes a cloud hop lease when the placement lease is spawned. It does not record that hop as `spawned: false`.
- A missing placement file is not a spawned lease. An unspawned cloud placement still syncs. The placement file is not rewritten.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #76 (this slice)

- `estate expire` refuses before it lists leases or forgets them when an expired cloud-agent lease is spawned. `expire --forget` does not drop that row.
- A missing placement file is not a spawned lease. An expired box lease still drops when no spawned cloud row is in that drop.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #75 (this slice)

- `estate suspend` and `floor suspend` refuse before they drop sessions or rewrite leases when a cloud-agent lease is spawned. They do not restamp that lease to unspawned.
- A missing placement file is not a spawned lease. A wired box lease still drops `spawned` on suspend.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #74 (this slice)

- `estate pause-proof` does not print the pause-proof JSON when the cell is drifted. That drift is `pause-proof: drift (fail closed)` with the drift notes.
- The clean note stays on a proof that is in sync. A spawned cloud lease and a lost lease count already refuse before that JSON.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #73 (this slice)

- Apply and resume refuse before they write when a cloud-agent lease is spawned, or `placement-actual.json` does not parse. They do not restamp that lease to unspawned. `--force` does not.
- A missing file is not a spawned lease. An unspawned file still applies.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #72 (this slice)

- `estate leases` and `floor leases` refuse before they print placement JSON when a cloud-agent lease is spawned.
- An unspawned file still prints. A missing file still says there is no placement-actual. The lease file is not rewritten.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #71 (this slice)

- `estate status` refuses before it prints `cloud-agent: declared, not spawned` when a cloud-agent lease is spawned.
- An unspawned cell still prints that line. The lease file is not rewritten.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #70 (this slice)

- `estate backup` and `estate restore` refuse before they write when `placement-actual.json` does not parse, or a cloud-agent lease in that file is spawned.
- A missing file is not a spawned lease. The archive meta does not record `cloud_agent_spawned: false` over that file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #69 (this slice)

- Apply, resume, and suspend do not journal `from: suspended` when `lifecycle.json` was missing. That `from` stays empty.
- A present file still supplies `from`. A file that does not parse is a refuse before the write. The loader default is not a prior state.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #68 (this slice)

- `floor status` does not invent `suspended` when `lifecycle.json` is missing. It prints `lifecycle: -`.
- A present file that does not parse is a refuse before that line. A file that parses prints `lifecycle:` and `durable=` from the file. `estate status` already prints `paused: -` for a missing file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #67 (this slice)

- `estate status` does not invent `suspended` when `lifecycle.json` is missing. It prints `paused: -` and `lifecycle: -`.
- A present file that does not parse is a refuse before the status page. A file that parses prints `paused:` and `lifecycle:` from the file. `estate doctor` already refused that file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #66 (this slice)

- `estate status` refuses (`refuse:model-actual`) when a present `model-actual.json` does not parse. It does that before the status page. A missing file is not a failure, and status does not invent a binding count.
- A file that parses is not a new status line. `estate drift` already refused that file. `estate doctor` already FAILs it before `factory ready`.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #65 (this slice)

- `estate doctor` FAILs a present `model-actual.json` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a binding count.
- A file that parses prints `model-actual.json bindings=` from the file. `estate drift` already refused that file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #64 (this slice)

- `estate doctor` FAILs a present `desired-snapshot.yaml` that does not parse when no cell catalog is present. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a frontier model from it.
- A file that parses prints `desired-snapshot.yaml name=` from the file. When a cell catalog is present, doctor already reads this snapshot. `estate apply` already refused an unreadable snapshot.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #63 (this slice)

- `estate doctor` FAILs a present `actual-state.json` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a session count.
- A file that parses prints `actual-state.json sessions=` from the file. `estate status` already refused that file through drift.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #62 (this slice)

- `estate doctor` FAILs a present `sessions.jsonl` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a line count.
- A file that parses prints `sessions.jsonl lines=` from the file. `estate sessions` already refused that file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #61 (this slice)

- `estate doctor` FAILs a present `lifecycle.jsonl` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a line count.
- A file that parses prints `lifecycle.jsonl lines=` from the file. `estate history` already refused that file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #60 (this slice)

- `estate doctor` FAILs a present `apply-audit.jsonl` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a line count.
- A file that parses prints `apply-audit.jsonl lines=` from the file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #59 (this slice)

- `estate doctor` FAILs a present `lifecycle.json` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent `suspended`.
- A file that parses prints `lifecycle.json state=` from the file. `suspended` is not a failure.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #58 (this slice)

- `estate doctor` FAILs a present `conveyor-mesh.json` that does not parse, or whose hop `host_class` is not a class. It does that before `factory ready`. A missing mesh is not a failure.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #57 (this slice)

- `estate status` refuses (`refuse:proposal-unreadable`) when a `*.proposal.json` does not parse, or its id does not match the filename. It does that before the status page. A missing proposals directory is not a proposal. A parsed proposal still lists.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #56 (this slice)

- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) holds a Mac `estate specialist` result row. It is **Pending**, not **PASS**, until a completion is pasted. Do not invent `pong` or `Pong` for the Mac.
- The #55 Mac command stays the open hand-off. This slice does not add another live test.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #55 (this slice)

- `estate status` and `estate doctor` refuse (`refuse:frontier-model`) when the cell `catalog.json` does not parse. They do that before the cell catalog success line. Apply, resume, and pause-proof already refused. A missing catalog is not a disagreement.
- The schema card is not the binding. The refuse does not invent `grok-4.7`.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #54 (this slice)

- Mac `estate specialist` complete is still unrecorded. [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) has the copy-paste for the MacBook Air and the open 5090-class box: `PATH` includes `~/.cargo/bin`, `CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434`, `CELL_LOCAL_MODEL=llama3`, then `estate specialist --driver ollama --prompt "Reply with the single word pong."`.
- The 5090 `Pong` stays recorded. Native MLX stays a stub. `mlx` / `vllm` / `trt` stay `not live-ok`.
- `READY_FOR_LIVE_TEST`: yes for that Mac command only. No new CLI. No new smoke or gate-90 step.

## After PR #53 (this slice)

- `estate probes --live` does not print `live ok` for `mlx`, `vllm`, or `trt`. An answering HTTP endpoint stays `not live-ok`. Ollama, llama.cpp, and http-remote still print `live ok` when their endpoint answers.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #52 (this slice)

- `docs/GATE-90.md` does not call the Ollama complete path ready. Mock completion stays green. The 5090 `Pong` is already recorded. Mac complete is not. `mlx` / `vllm` / `trt` refuse a frontier POST and are not live-ok.
- `docs/DAY90-PLUS.md` parks vLLM and TRT with native MLX. `READY_FOR_LIVE_TEST` is yes only when a concrete live-probe command is still unblocked. None is.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #51 (this slice)

- `estate backup` and `estate restore` refuse before they write. A cell catalog whose frontier model disagrees with the binding is `refuse:frontier-model`. A frontier `source_driver` with no frontier binding is `refuse:frontier-invent`. A desired snapshot whose sacred set disagrees is `refuse:sacred-mismatch`.
- A missing catalog is not a disagreement. A local-only pack still archives. `CELL_FRONTIER_MODEL` is not the binding. The refuse does not invent `grok-4.7` unless that model is already in the catalog file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #50 (this slice)

- `estate packs import` and `estate feed import` refuse (`refuse:frontier-invent`) when the pack tags `frontier` and the estate has no frontier binding. They do that before an accepted pack, a redaction report, or an index rewrite.
- A local-only pack still imports. A mixed-fixture import keeps `source_drivers` frontier and local and writes a redaction report of kind counts. The report does not store a raw secret and does not invent `grok-4.7`. `CELL_FRONTIER_MODEL` is not the binding.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #49 (this slice)

- `estate resume` and `estate pause-proof` refuse (`refuse:frontier-model`) when the cell `catalog.json` frontier model disagrees with the binding. They do that before leases or a catalog write.
- A missing catalog is not a disagreement. A matching empty catalog still resumes, and the cell model stays empty. `pause-proof` does not invent a cell catalog. `CELL_FRONTIER_MODEL` is not the binding.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #48 (this slice)

- `make day90-mixed` prints `frontier plan: model=grok-4.7 source_drivers=frontier,local` when the mixed fixture is bound. Apply, status, and doctor keep that bound model. `CELL_FRONTIER_MODEL` does not become the binding.
- The same opt-in walk checks a throwaway local-only estate. Plan and dry-run are `refuse:frontier-invent`. Live apply writes no catalog and does not print `grok-4.7`. Status prints no frontier binding. Doctor does not print a cell catalog model.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #47 (this slice)

- `estate models` prints `model=-` when the binding sets no model. `CELL_FRONTIER_MODEL=grok-4.7` does not become the binding.
- `estate catalog` labels the schema card `(schema card, not a binding)`. It refuses (`refuse:frontier-model`) before overwriting a catalog whose frontier model is not that card. A missing file still receives the schema dump. An unset binding stays empty.
- Operator-day and fixtures-check write that schema dump beside the cell catalog. Same checks. No new CLI. No new smoke or gate-90 step.
- `READY_FOR_LIVE_TEST`: no.

## After PR #46 (this slice)

- Live `estate apply` refuses (`refuse:frontier-model`) when the cell `catalog.json` frontier model disagrees with the binding. It does that before leases, an unchanged audit, or a catalog rewrite. A missing catalog is not a disagreement. `--force` does not overwrite it.
- A matching catalog still applies. Unset `params.model` stays empty. The schema card is not copied.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #45 (this slice)

- `estate reconcile` and `estate resume` refuse (`refuse:frontier-invent`) when the estate has no frontier binding. They do that before `reconcile.json`, a suggest patch, or a resume catalog write. They do not copy the schema card or `CELL_FRONTIER_MODEL`.
- An estate that already has a frontier binding still reconciles and resumes. Unset `params.model` stays empty.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #44 (this slice)

- `estate status` and `estate doctor` refuse (`refuse:frontier-model`) when the cell `catalog.json` frontier model disagrees with the binding. Empty and missing are the same (`model=-`). The schema card stays `grok-4.7` and is not treated as the binding.
- A matching cell catalog still prints `model=grok-4.7` or `model=-`.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #43 (this slice)

- `packs propose` and `packs accept` refuse (`refuse:frontier-invent`) when `source_drivers` names `frontier` and the estate has no frontier binding. They do not write the proposal or the enrich-edit file, and they do not copy the schema card.
- A pack that is only `local` still proposes and accepts as `local`. It does not gain a frontier driver.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #42 (this slice)

- `estate plan` and `apply --dry-run` print `frontier plan: model=` from the binding (`model=-` when `params.model` is unset). They do not copy the schema card and they do not read `CELL_FRONTIER_MODEL`.
- An estate with no frontier binding refuses (`refuse:frontier-invent`) before any plan file or dry-run preview. That refuse does not invent a frontier `source_driver` or `grok-4.7`.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #41 (this slice)

- Apply and resume write the cell `catalog.json` frontier model from the estate binding. No `params.model` stays empty. Status prints `catalog frontier: cell model=-`. Doctor does not report that file as `grok-4.7`.
- The schema catalog card stays `grok-4.7`. `estate catalog` still dumps that card. A binding that sets `grok-4.7` still writes it. Two different frontier models refuse.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #40 (this slice)

- `make day90-mixed` validates `examples/hosts/frontier-http.yaml` and prints status (`frontier: frontier_http model=grok-4.7`). No apply. No live key. A greenfield status does not invent a cell catalog. `examples/estate.yaml` cksum is unchanged.
- The host file stays off `scripts/fixtures-check.sh`, smoke, and gate-90.
- `estate models` prints `model=` from `params.model` when set, and `model=-` when it is not. The default estate does not invent `model=grok-4.7`.
- No new CLI.
- `READY_FOR_LIVE_TEST`: no.

## After PR #39 (this slice)

- `examples/hosts/frontier-http.yaml` names `model: grok-4.7` on a frontier `http-remote` binding, with a local `ollama` card. It is not a host-class alias and it is not on smoke or gate-90. `examples/estate.yaml` stays hash-locked.
- A frontier specialist prompt that mentions Cyera or Rust classroom still refuses as sacred when `CELL_FRONTIER_MODEL` is a hardware SKU. No POST. No invented completion. The SKU model path is not the refusal.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## #38–#39 in plain English

#38. `make feed-loop` checks that the pack, the proposal, and `enrich-edit.json` carry the same `source_drivers`. A missing tag with a nonzero frontier or local count refuses the index rewrite and leaves the previous INDEX. Accept serializes the enrich-edit JSON before it writes either file.

#39. Frontier and local `estate specialist` both refuse a Cyera or Rust classroom prompt before POST, and do not invent a completion. README and OPERATOR-DAY point at `estate help frontier` and `make day90-mixed`. Those stay off smoke and gate-90.

## After PR #38 (this slice)

- Frontier `estate specialist` refuses a Cyera or Rust classroom prompt before POST, and does not invent `"completion": "ok"`. The local specialist test locks the same two prompts.
- README start-here and `docs/OPERATOR-DAY.md` point at `estate help frontier` and `make day90-mixed`. Those stay off smoke and gate-90. `examples/fixtures/mixed-frontier-local.yaml` already names `model: grok-4.7` on the frontier `http-remote` binding. `examples/estate.yaml` stays hash-locked.
- No new CLI.
- `READY_FOR_LIVE_TEST`: no.

## #35–#37 in plain English

#35. Frontier refuse text names `CELL_FRONTIER_MODEL` and `CELL_FRONTIER_ENDPOINT`. A hardware SKU in the model id says which setting it came from and still refuses before any POST. GATE-90 and DAY90-PLUS keep green factory checks, recorded live proofs, and parked rows separate. Mac specialist stays optional. Native MLX and cloud-spawn stay parked.

#36. Pack INDEX and `estate feed list` print `drivers=` when `source_drivers` is present, and `drivers=-` when it is empty. A failed INDEX rewrite is an error. `make feed-loop` greps that line. There is no `make feed-loop-mixed`.

#37. `packs accept` copies that tag into the enrich-edit instructions. An empty list stays `-`. A tag that does not match `path_counts` refuses before the edit file is rewritten. A failed proposal INDEX rewrite is an error. `source_drivers` stays an additive v0 field.

## After PR #37 (this slice)

- `make feed-loop` checks the pack, the proposal, and `enrich-edit.json` carry the same `source_drivers` (`frontier` then `local`).
- Hole: a pack or proposal that omitted `source_drivers` while frontier or local counts were nonzero was indexed as `drivers=-`. The index rewrite now refuses and leaves the previous INDEX in place.
- Accept serializes the enrich-edit JSON before it writes either file, so a serialize failure does not leave a new markdown next to a stale JSON.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #36 (this slice)

- `packs accept` copies `source_drivers` from the proposal into the enrich-edit markdown, JSON, and paste comment. Empty stays `-` and is not invented as frontier. A tag that does not match `path_counts` refuses before the edit file is rewritten.
- Hole: `propose_enrich` swallowed a failed proposal INDEX rewrite, and an unreadable proposal was still listed by name. The rewrite is now an error, and the index line lists `drivers=`.
- Schema freeze: `source_drivers` stays additive on pack, specialist-pack, and enrich-proposal v0. Only `frontier` and `local`. Missing field defaults to `[]`. A rename is a v1.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #35 (this slice)

- Pack INDEX and `estate feed list` / `estate packs list` print `drivers=` from `source_drivers` (`frontier,local` when both are present, `-` when empty). Empty packs do not invent a source.
- Hole: `write_drop_pack`, import, and `feed list` swallowed a failed INDEX rewrite. A failed rewrite is now an error.
- `make feed-loop` greps INDEX for `drivers=frontier,local`. Did not add `make feed-loop-mixed`; the existing walk already tags mixed frontier and local traces. Still off smoke and gate-90. No new CLI.
- `READY_FOR_LIVE_TEST`: no.

## After PR #34 (this slice)

- `estate specialist --driver frontier` refuse text names `CELL_FRONTIER_MODEL` and `CELL_FRONTIER_ENDPOINT`. A missing `XAI_API_KEY` already did. A hardware SKU in the model id now says whether it came from `CELL_FRONTIER_MODEL`, `XAI_MODEL`, or a binding model param, and still refuses before POST.
- `docs/GATE-90.md` and `docs/DAY90-PLUS.md` split green factory checks, recorded live proofs, and parked rows. Mac specialist complete is optional and not recorded. Native MLX stays a stub. Cloud-spawn stays off. The recorded 5090 proof is not called parked.
- CELL-ONE-STATUS states #32–#34 in plain English.
- `READY_FOR_LIVE_TEST`: no.

## After PR #33 (this slice)

- `make feed-loop` asserts the produced pack `source_drivers` is `frontier` then `local`, counts are non-zero, and propose copies the tag. Live keys are unset. Still off smoke and Actions.
- `estate help frontier` names `grok-4.7`, the `XAI_API_KEY` gate, and that local down does not POST frontier. `estate help day90-mixed` stays opt-in.
- Hole: `run_task` swallowed a failed feed append and could still complete. A failed audit now refuses before frontier.
- `READY_FOR_LIVE_TEST`: no.

## After PR #32 (this slice)

- Feed packs tag `source_drivers` (`frontier` and/or `local`) from the events. The tag must match `path_counts`. Unknown drivers, a tag with a zero count, and a count with no tag refuse.
- Explicit `object_class: local` stays local even when the kind or note mentions frontier. Local-down does not tag frontier.
- Propose copies `source_drivers` onto the diff. `auto_apply` stays false. Promote stays off. The estate file is not rewritten.
- `READY_FOR_LIVE_TEST`: no.

## After PR #31 (this slice)

- `make day90-mixed` walks the mixed fixture: status → plan → `apply --require-plan` → status → doctor. Isolated cell. No live key. Not in smoke or Actions.
- `estate status` prints `frontier: <id> model=…` only when the binding sets `params.model`. Catalog lines print `grok-4.7` when `frontier.model` is in the schema catalog or the cell `catalog.json`. Doctor prints the same. A SKU model fails doctor.
- The default estate binding has no model param, so status does not invent `frontier: xai_grok model=grok-4.7`.
- `READY_FOR_LIVE_TEST`: no.

## After PR #30 (this slice)

- Mixed fixture operator path: `estate plan` then `estate apply --require-plan` writes model-actual, placement-actual, and catalog. Mock only. No `XAI_API_KEY`. No frontier POST. Local `ollama` specialist after apply still skips frontier.
- Catalog file SoT sibling card: model `grok-4.7`, streaming/tools/vision false, completion budget 64. Not a local probe. Not a context window.
- Requested local specialist does not POST frontier: `ollama` up, `http-remote` up, `llama.cpp` down, `mlx` / `vllm` / `trt` refuse.
- `READY_FOR_LIVE_TEST`: no.

## After PR #29 (this slice)

- Recorded frontier specialist live PASS: `--driver frontier`, model `grok-4.7`, `completion` `pong`, reason `frontier completion`. Key never printed. Env-gated `XAI_API_KEY`. No box hostname.
- `READY_FOR_LIVE_TEST`: no for that surface.
- Mixed fixture `frontier_http` (`http-remote`, model `grok-4.7`) + local `ollama`: `validate` and `apply --dry-run` stay green with no live key and no POST.
- Local specialist (`--driver ollama`) down or unset does not POST to frontier even when `XAI_API_KEY` and `CELL_FRONTIER_ENDPOINT` are set.

## After PR #28 (this slice)

- Frontier model id is **`grok-4.7`** (`CELL_FRONTIER_MODEL` or `XAI_MODEL`). The old `grok-3-mini` default is gone.
- `estate specialist --driver frontier` requires `XAI_API_KEY`. Optional `CELL_FRONTIER_ENDPOINT` (default `https://api.x.ai/v1`). Unset key refuses. Sacred and SKU refuse before POST. Mock-locked. No key in CI.
- `--driver http-remote` stays the local `CELL_LOCAL_ENDPOINT` card. Local down does not fall through to frontier.
- Cloud-agent standing default `reasoning_effort` xhigh is documented only. The factory chat POST sends `grok-4.7`.
- `READY_FOR_LIVE_TEST`: yes. One command with a real `XAI_API_KEY`.

## After PR #27 (this slice)

- `--driver frontier` is env-gated on `CELL_FRONTIER_ENDPOINT` (or `--endpoint`). `CELL_LOCAL_ENDPOINT` and `XAI_API_KEY` do not unlock it. Mock-locked OpenAI chat. Not in CI / smoke.
- Operator fixture: `estate apply` records `local_slm` in `model-actual.json`, then `estate specialist` complete against mock-local.
- Driver is resolved before the local endpoint, so `--driver frontier` no longer dies as a missing `CELL_LOCAL_ENDPOINT`.
- `READY_FOR_LIVE_TEST`: no. Mac specialist is the same Ollama complete already proven on 5090.

## After PR #26 (this slice)

- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) records Mac `probes --live` PASS, 5090 `probes --live` PASS, and 5090 `estate specialist` `completion` `Pong`. Native MLX stays stub.
- `make live-specialist` requires `CELL_LOCAL_ENDPOINT` (SKU endpoint refuses). Not in `make smoke` / `make gate-90` / Actions.
- Mixed-estate apply `--dry-run` stays write-free; `local_slm` binds `HttpLocal` against in-process mock (`/v0/specialist`).
- Apply / resume no longer swallow `catalog.json` write failure.
- `READY_FOR_LIVE_TEST`: no. Recorded surfaces already ran. Mac specialist chat and live Grok are still unrecorded.

## After PR #25 (this slice)

- OpenAI `/v1/chat/completions` with empty / missing / whitespace `message.content` falls through to Ollama `/api/chat` (same as probes try both shapes).
- Accepts content-array parts, `text`, or reasoning-only when the text is clearly non-empty.
- Both chat paths fail: HTTP status, model id used, `ollama pull llama3` / `CELL_LOCAL_MODEL`.
- Unset `CELL_LOCAL_MODEL` prefers first `/api/tags` id, then `/v1/models`.
- `READY_FOR_LIVE_TEST`: yes. 5090 retry of `estate specialist --driver ollama --prompt "Reply with the single word pong."`

## After PR #24 (this slice)

- `estate specialist --driver ollama --prompt` is a thin `HttpLocal` delegate. Default job `complete` returns model `completion`. Same helper as `model-estate specialist --job complete`.
- Sacred / empty / SKU refuse before any HTTP POST. Empty `message.content` refuses. A bad OpenAI body still does not try Ollama.
- Mock-local complete is `mock:{text}`. Compat OpenAI / Ollama complete is the model body (`ok` in-process).
- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) has the exact Mac Ollama command.
- `READY_FOR_LIVE_TEST` for this complete verb: yes. Jason already PASSed `probes --live`.

## After PR #23 (PR #24 specialist round-trip)

- Specialist chat posts the real request text (not dummy `ping`) through `HttpLocal` against mock HTTP. Last-POST capture locks it.
- `model-estate specialist` was the data-plane equivalent before `estate specialist` existed.
- llama.cpp server OpenAI path smoke: same adapter, `HttpLocal { runtime: LlamaCpp }` + CLI `--runtime llama.cpp`.
- Fail-closed: v0 200 unparseable refuses (no compat fall-through); OpenAI choices require `message.content`; SKU `CELL_LOCAL_MODEL` / listed model ids refuse.
- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) now has exact SKIP / live ok / down / specialist JSON lines.
- `READY_FOR_LIVE_TEST` for that policy-precheck verb: no. Do not ping Jason.

## After PR #22 (PR #23 adapter)

- Live probes GET `/v1/models` or Ollama `/api/tags`. Empty models list is up. Garbage / empty body is down. No invent success.
- `HttpLocal` adapter: factory `/v0/specialist`, then OpenAI chat / Ollama chat, then factory-owned policy. Native MLX `specialist()` stays stub. Mac proof is Ollama-on-Mac.
- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) is copy-paste for Jason Mac and Jason Linux/5090. In-process mock HTTP locks the adapter. CI stays SKIP without env.

## After PR #21 (this slice)

- Hole: `record_bindings` used `unwrap_or_default` and could wipe `model-actual.json`. Isolation `session.json` was the same class. Serialize or refuse. Garbage model-actual is refuse, not "run apply".
- Hole: accept / import / propose compared estate bytes with `unwrap_or_default`, so an unreadable file looked unchanged. `read_estate_text` refuses.
- Live probe hand-off: [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md). Dry fixture `schema/live-probe-shapes.v0.json` locks SKIP vs would-live without network. Not a live-box proof. Do not ping Jason yet.

## After PR #20 (this slice)

- Hole: `propose_enrich` used `unwrap_or_default` and could write an empty proposal. Serialize or refuse. No empty `.proposal.json`.
- Hole: `append_event` invented `"{}"` on serialize failure and could append junk to `events.jsonl`. Journal / audit lines serialize or refuse. Empty object is refuse.
- Hole: `write_placements` used `unwrap_or_default` and could wipe `placement-actual.json` empty. Same class closed on lifecycle, session journal, apply-audit, reconcile, backup meta, actual-state, mesh persist, and accept enrich-edit.
- CELL-ONE-STATUS records #20 and this hunt. Isolated locks stay crate tests plus `tests/day90_honesty.rs`.

## After PR #19 (this slice)

- Hole: `covering_plan` / `latest_plan` / `list_plans` treated unreadable plan JSON as empty. `apply --require-plan` could say "no plan"; `status` could invent last-plan. Present plan JSON parses or refuses. APIs return `Result`.
- Hole: `write_cursor` used `unwrap_or_default` and could write empty `feed-cursor.json`. A present garbage cursor could look missing. Load: missing → none; exists but not a file or parse fail → refuse. Write serializes or refuses.
- Hole: `apply` without `--force` could still walk a SKU `placement-actual`. `--force` now claims estate `host_class` (`rtx_consumer` → `consumer-nvidia`) and does not launder the SKU to `any`. Without `--force`, `refuse:bad-host-class` before dry-run.
- Remaining `.ok()` on Path-exists loads in estate-control / floor / conveyor / feed are listing skips, not SoT. `load_placements` stays permissive so `--force` can overwrite.
- CELL-ONE-STATUS records #19 and this hunt. Isolated lock stays `tests/day90_honesty.rs`.

## After PR #18 (this slice)

- Hole: `suspend` / `resume` / `apply` used `load_lifecycle(...).ok()`. A present-but-unreadable `lifecycle.json` was treated as greenfield and overwritten. Parse or refuse. Apply refuses before lease writes.
- Hole: Feed import wrote the accepted pack, then swallowed `append_import_audit` (`let _ =`). Audit append is fail-closed. Serialize no longer invents an empty line.
- Hole: `apply --dry-run --import-pack --curator robot` skipped the curator check. `refuse_import_pack` runs first. Live and dry-run write no leases, no accepted pack, no audit.
- `estate catalog` writes first, then prints. `estate probes` refuses every card first, then prints.
- Status no longer invents `expired=0` / empty proposals on reader failure. Plan `export-pr` no longer invents an empty expired list.
- CELL-ONE-STATUS records #18 and this hunt. Isolated lock stays `tests/day90_honesty.rs`.

## #15–#18 (SKU then honesty)

- #15: `convey sync` slim-parse laundered a SKU `host_class` to portable `any`.
- #16: `convey call` swallowed that refuse; floor / status / leases / reconcile stayed silent.
- #17: mesh readers and restore still copied a SKU. Production `canonical_host_class` callers gone.
- #18: print-then-refuse on leases; backup/restore estate `.ok()`; suspend swallowed actual-state / journal; feed redaction write was `let _ =`.

## After PR #17 (this slice)

- Hole: `estate leases` printed the SKU `placement-actual` JSON, then refused. Readers now refuse first. Status already refused before print; tests lock both.
- Hole: `backup` / `restore` swallowed a present-but-unreadable estate file (`load_estate(...).ok()`), so restore could invent a locked-only sacred set. A file that exists must parse or refuse.
- Hole: `suspend` swallowed `actual-state.json` parse errors and journal write failures, so unspawn lines could vanish. Load and journal fail closed.
- Feed import: `{id}.redaction.json` write is no longer `let _ =`. Report bytes are kind counts only; a raw secret in the report is `refuse:raw-secret`. Hand-written dirty packs write no accepted pack and no report.
- `lifecycle.jsonl` / `sessions.jsonl` stay append-only under suspend / resume / `expire --forget`. Forget does not truncate journals.
- Hole leftover from #17: floor `src` tests used `rtx-5090`, so `doctor --strict` failed the vendor-needle scan. Fixtures now use `not-a-host`.
- Sacred / curator / policy refuse paths had no new silent `Ok()` swallows.
- CELL-ONE-STATUS states the #15–#17 SKU launder story in plain English.
- Isolated lock: `tests/day90_honesty.rs`.

## After PR #16 (this slice)

- Hole: `convey call` / `list` / `expire` / `sync` loaded a tampered `conveyor-mesh.json` without checking `host_class`. A SKU hop/lease could allow a call or get written back. Readers now `refuse:bad-host-class` and write nothing.
- Hole: `restore` copied a SKU `placement-actual` (or mesh) onto disk. Restore is now `refuse:bad-host-class` (dry-run and live). Does not invent `any`.
- Remaining `canonical_host_class` is the trusted unwrap only. Hop declare stamps via opt after `refuse_hop`.
- Isolated lock: `tests/day90_mesh.rs`.

## After PR #15 (this slice)

- Hole: `convey call` swallowed slim-parse `refuse:bad-host-class`, so a tampered SKU `host_class` skipped the placement not-live check. Call now fails closed. Status / leases / reconcile use the same refuse. `record_placements` writes nothing; claim does not rewrite the SKU to `any`.
- Validate on multi-host + mixed fixtures stays green. Probe / catalog card ids still refuse SKUs the same way hop ids do.
- Isolated lock: `tests/day90_sku.rs`.

## After PR #14 (this slice)

- Hole: `convey sync` rewrote unknown / SKU `host_class` on a tampered `placement-actual.json` to `any` and seeded a hop. Slim-parse now `refuse:bad-host-class` and writes no mesh files. Alias round-trips stay (`rtx_consumer` → `consumer-nvidia`).
- Overlay with `locked: []` still cannot drop Cyera CI / Rust classroom (property lock).
- Two `apply --dry-run` after a real apply leave an identical `.cell` tree. Dual-layer-demo backup → restore with matching sacred writes the leases back.
- Makefile contract: `gate-90` / `smoke` / `day90` / `feed-loop` / `fixtures-check` / `doctor-strict` exist; `gate-90` does not invoke `gh`.

## Still stubbed

MLX stays a stub. vLLM and TRT stay experimental catalog cards, not live-ok. Cloud-agent spawn. Convey hop transport (lease-bound mesh only). Auto-promote. Curator UI. See `docs/DAY90-PLUS.md`.

## How to run

```bash
make gate-90    # Day-90 operator entrypoint (local)
make smoke      # doctor + fixtures-check + operator-day + cargo test + make day90
make day90      # operator loop only
make feed-loop  # scrubbed trace → pack → propose → accept (fixtures only)
estate help     # Day-90 topic pages
# walk: docs/OPERATOR-DAY.md
# snapshot: docs/CELL-ONE-STATUS.md
```
