# Quant ladder, grammar-constrained eval, determinism check

Three opt-in factory steps for purpose-built SLMs. They answer one question before a quantized build is seated anywhere: does the smaller quant decide the same way as the reference build, and does the same build decide the same way twice? One YAML config (schema `cell-one.quant-ladder.v0`) drives all three. Domain logic lives only in the scorer command the config names, so any project brings its own scorer.

`READY_FOR_LIVE_TEST`: no. The steps do not seat, promote, apply, run Ollama, train, or download weights. They are not part of `make smoke`, `make gate-90`, or GitHub Actions.

## Steps

| Step | Command | What it does |
|---|---|---|
| Quant ladder build | `scripts/quant-ladder.sh build CONFIG` | Converts the cached HF model to an f16 (or bf16) GGUF with `~/llama.cpp/convert_hf_to_gguf.py` under `~/venvs/llamafactory`, then runs `llama-quantize` once per listed quant (no imatrix). Writes `manifests/gguf.json` (HF file sha256s, GGUF sha256s, llama.cpp commit). Optional `gguf_mirror` copies the GGUFs (`cp`, never overwrite). Refuses when the model is not already in the HF cache. |
| Grammar-constrained eval | `scripts/grammar-eval.sh CONFIG` | Renders every task's prompts with the model's own chat template, checks the eval-set and grammar sha256s, then for each repeat and each quant starts one `llama-server` (batch 1, `-np 1`, flash attention off, fixed threads, fixed seed, ctx from the config, `--cache-ram 0`), sends one request at a time (`temperature 0`, `top_k 1`, `cache_prompt false`, GBNF `grammar` or `json_schema`), stops the server by its own PID, and runs the scorer command. `nvidia-smi` runs before every server start; any compute app or utilization over `gpu_guard.max_util_pct` is `refuse:gpu-busy` (exit 3). `gpu_guard.max_minutes` caps GPU time (exit 4). |
| Determinism check | `scripts/determinism-check.sh CONFIG` | Byte-compares every task output and its token ids across the N repeats of each quant. Writes `determinism.json` and `DETERMINISM.md`; exit 5 on any difference. |
| Ladder compare | `scripts/quant-ladder.sh compare CONFIG` | Aggregates the scorer summaries per quant and repeat, counts per-item flips (`compare.flip_fields`, false in the other build and true in this one) and agreement (`compare.agree_fields`) between every pair of quants, exact-output agreement per task, and per-item latency (server prompt + decode ms). Writes `ladder.json` and `LADDER.md`. |

## Scorer contract

`scorer.command` is a shell template with `{python} {run_dir} {metrics} {eval} {config_dir} {out_dir} {quant} {repeat}`. It reads `{run_dir}/<task>.jsonl` (one record per item: `id`, `output`, `format`, `input_hash`, `tokens`, `n_prompt_tokens`, `n_out_tokens`, `prompt_ms`, `predicted_ms`, `wall_ms`, `stop_type`, `truncated`) and writes `{metrics}` as JSON: `{"summary": {name: value}, "items": {id: {field: value}}}`. A non-zero exit or a missing metrics file stops the eval.

## Layout under `out_dir`

`gguf/<basename>-<quant>.gguf`, `manifests/{gguf,prompts,eval-run}.json`, `prompts/<task>.jsonl`, `outputs/<quant>/rep<N>/<task>.jsonl`, `metrics/<quant>-rep<N>.json`, `logs/` (build, server per run, scorer, gpu-checks), `determinism.json`, `DETERMINISM.md`, `ladder.json`, `LADDER.md`.

## Config

Copy [`quant-ladder.example.yaml`](quant-ladder.example.yaml) into a run folder under `.cell/` and edit it. Relative paths resolve against the config file. Wrappers use `QUANT_LADDER_PYTHON` (default `~/venvs/llamafactory/bin/python`; needs PyYAML and transformers). The CUDA `llama-server` is a separate additive build: `cmake -S ~/llama.cpp -B ~/llama.cpp/build-cuda -DGGML_CUDA=ON -DCMAKE_CUDA_COMPILER=/usr/local/cuda-12.8/bin/nvcc -DCMAKE_CUDA_ARCHITECTURES=120 -DLLAMA_CURL=OFF && cmake --build ~/llama.cpp/build-cuda --target llama-server -j 12` (the existing CPU `build/` is untouched).

Optional, additive keys: `decode.stop` (a list of stop strings, sent as `stop` to llama-server `/completion`, e.g. `["<|im_end|>"]` for a model whose runtime must stop on its turn-end token) and `toolchain.convert_script` (a drop-in for `convert_hf_to_gguf.py` with the same CLI, run under `convert_python`; its path and sha256 are recorded in `manifests/gguf.json`). Example: `.cell/lunatron-model-scout-20261002/drafts/patches/llama_convert_nemotron_h_dense.py` fixes dense Nemotron-H being detected as `nemotron_h_moe` under transformers 5.x.

## Honest limits

GGUF quants on llama.cpp are a proxy for other runtimes (TensorRT-LLM AWQ, vLLM). Determinism here is one host, one binary, one GPU, repeated runs; it is not a cross-board check. Q4_K_M is built without an importance matrix.
