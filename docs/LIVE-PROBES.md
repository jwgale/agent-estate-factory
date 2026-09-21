# Live probes (hand-off page)

Copy-paste for Jason at a Mac or a Linux/5090-class box. CI never sets
these variables. Without an endpoint, `estate probes --live` prints SKIP
and exits 0.

Parked status: [`DAY90-PLUS.md`](DAY90-PLUS.md). Dry shapes (no network):
`schema/live-probe-shapes.v0.json`.

Do not put `5090`, `4090`, or `m3-max` in estate binding ids or probe ids.
A 5090 is one `consumer-nvidia` or `rented-nvidia` host.

## What a live probe actually pings

Not the Ollama chat UI. Not native MLX. Not `POST /v0/specialist`.

```
GET {endpoint}/v1/models          # OpenAI-compatible (Ollama, vLLM, MLX servers)
GET {endpoint}/api/tags           # Ollama native, if /v1/models is missing
```

Timeout is 800ms. A 2xx JSON **models list** is `live_probed=true` /
`live ok (openai /v1/models)` or `live ok (ollama /api/tags)`. An empty
list is up (server running, no weights pulled). Garbage, empty body, or
connect fail is `live_probed=false` / `down: ...`. Unset endpoint is
SKIP. The command still exits 0 in all three cases. It does not invent
success.

Native `mlx` `specialist()` stays **Stub**. Mac live proof is
**Ollama-on-Mac** (same `ollama` card) or any OpenAI-compatible MLX
*server* on `CELL_MLX_ENDPOINT`. A green mlx ping only means that HTTP
answered. It does not flip the catalog card to Supported.

## Env

| Variable | Mac | Linux / 5090-class |
| --- | --- | --- |
| `CELL_LIVE_PROBE` | `1` / `true` / `yes` if you omit `--live` | same |
| `CELL_LOCAL_ENDPOINT` | Ollama-on-Mac Supported path (`http://127.0.0.1:11434`) | Ollama / llama.cpp / http-remote |
| `CELL_RENTED_ENDPOINT` | unused | alias for a rented (or any) box. Not a SKU. |
| `CELL_MLX_ENDPOINT` | OpenAI-compatible MLX *server* only; native MLX stays stub | unused (mlx SKIP) |
| `CELL_VLLM_ENDPOINT` | experimental; unset = SKIP | experimental `/v1/models` |
| `CELL_TRT_ENDPOINT` | experimental; unset = SKIP | experimental; unset = SKIP |
| `CELL_LOCAL_MODEL` | optional model id for `specialist()` | optional |

`XAI_*` is frontier A7. Probes do not use it.

## Dry on this box (no GPU, no Jason)

```bash
cargo run -q -p estate-control -- probes --live
# expect: SKIP, live_probed=false, exit 0, no 5090 / 4090 / m3-max

cargo test -q -p model-estate --lib ping_
cargo test -q -p model-estate --lib specialist_
cargo test -q -p estate-control --test day90_heal probes_live_skip
```

## Jason Mac (Apple Silicon) - Ollama-on-Mac

Supported path. Native MLX stays stub.

```bash
# Terminal 1 - if ollama is not already a service
ollama serve
```

```bash
# Terminal 2
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
cargo run -q -p estate-control -- probes --live
```

Expect ollama / llama.cpp / http-remote: `live_probed=true` and
`live ok (openai /v1/models)` or `live ok (ollama /api/tags)`.
mlx SKIP (no `CELL_MLX_ENDPOINT`).

Optional OpenAI-compatible MLX *server* (still not native MLX):

```bash
export CELL_MLX_ENDPOINT=http://127.0.0.1:8080
cargo run -q -p estate-control -- probes --live
```

Expect mlx: `status=stub bindable=false live_probed=true` if that server
answers `/v1/models`. `specialist()` on the mlx card still refuses Stub.

## Jason Linux / 5090-class (consumer-nvidia or rented-nvidia)

Never encode `5090` in a binding id.

```bash
# Terminal 1 - if ollama is not already a service
ollama serve
```

```bash
# Terminal 2 - desktop / laptop RTX = consumer-nvidia
# rented / lab GPU (including a 5090 host) = rented-nvidia
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
# or: export CELL_RENTED_ENDPOINT=http://127.0.0.1:11434
cargo run -q -p estate-control -- probes --live
```

Expect ollama / llama.cpp / http-remote `live_probed=true`. mlx SKIP.
vLLM SKIP unless `CELL_VLLM_ENDPOINT` is set.

Optional vLLM (experimental card; live ping only):

```bash
export CELL_VLLM_ENDPOINT=http://127.0.0.1:8000
cargo run -q -p estate-control -- probes --live
```

## Down vs SKIP

Stop Ollama and re-run `probes --live` with the same env: expect
`live_probed=false` and `down:`. Unset the env: expect SKIP. Both exit 0.

Optional A8 after a model is pulled (`ollama pull llama3` or set
`CELL_LOCAL_MODEL`):

```bash
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent research --act tool --object notes-append --payload "append a note"
```

Down local is `local:down` / `model.local.down`. No silent Grok fallback.

## What green is not

| You saw | What it is not |
| --- | --- |
| SKIP in CI or on this cloud box | A failed live test |
| `live_probed=true` from `mock-local` | MLX Supported, or weights green |
| mlx note `live ok` | Catalog card flipped off stub; native MLX |
| Empty models list `live ok` | A pulled model; only the HTTP server is up |
