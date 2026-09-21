# Live probes (hand-off page)

Copy-paste for Jason at a Mac or a Linux/5090-class box. CI never sets
these variables. Without an endpoint, `estate probes --live` prints SKIP
and exits 0.

Parked status: [`DAY90-PLUS.md`](DAY90-PLUS.md). Dry shapes (no network):
`schema/live-probe-shapes.v0.json`.

Do not put `5090`, `4090`, or `m3-max` in estate binding ids or probe ids.
A 5090 is one `consumer-nvidia` or `rented-nvidia` host.

`estate specialist` is a thin data-plane delegate (`HttpLocal`). It is
not a gateway. Default job is `complete`: Jason gets real model text
back. Sacred tokens refuse before any HTTP POST. SKU endpoint / model
ids / prompt / completion refuse. Missing endpoint refuses. Native MLX
/ vLLM / TRT stay stub or experimental.

## What a live probe actually pings

Not the Ollama chat UI. Not native MLX. Not `POST /v0/specialist`.

```
GET {endpoint}/v1/models          # OpenAI-compatible (Ollama, llama.cpp, vLLM)
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
*server* on `CELL_MLX_ENDPOINT`. mlx also falls back to
`CELL_LOCAL_ENDPOINT`, so an Ollama-on-Mac env may print mlx `live ok`.
That is HTTP, not native MLX. The catalog card stays stub.

## Env

| Variable | Mac | Linux / 5090-class |
| --- | --- | --- |
| `CELL_LIVE_PROBE` | `1` / `true` / `yes` if you omit `--live` | same |
| `CELL_LOCAL_ENDPOINT` | Ollama-on-Mac Supported path (`http://127.0.0.1:11434`) | Ollama / llama.cpp / http-remote |
| `CELL_RENTED_ENDPOINT` | unused | alias for a rented (or any) box. Not a SKU. |
| `CELL_MLX_ENDPOINT` | OpenAI-compatible MLX *server* only; native MLX stays stub | unused unless you set it |
| `CELL_VLLM_ENDPOINT` | experimental; unset = SKIP | experimental `/v1/models` |
| `CELL_TRT_ENDPOINT` | experimental; unset = SKIP | experimental; unset = SKIP |
| `CELL_LOCAL_MODEL` | optional model id for `specialist()`; SKU ids refuse | optional; SKU ids refuse |

`XAI_*` is frontier A7. Probes do not use it.

## Sample output (exact lines)

Driver column is 12 chars. Status column is 12 chars. Notes wrap on the
next indented line.

### SKIP (no endpoint env; this cloud box / CI)

```
live probe (SKIP without endpoints; not used in CI)
  ollama       status=supported    bindable=true live_probed=false host_class=any
    Ollama-first. Catalog-level probe; not a live ping. SKIP (no endpoint env; CI never requires a live box)
  llama.cpp    status=swap-proof   bindable=true live_probed=false host_class=any
    llama.cpp swap-proof sibling. Same specialist protocol. SKIP (no endpoint env; CI never requires a live box)
  mlx          status=stub         bindable=false live_probed=false host_class=apple-silicon
    Native MLX specialist() is stub. Live Mac proof is Ollama-on-Mac (or OpenAI-compatible) via the HTTP adapter. SKIP (no endpoint env; CI never requires a live box)
  vllm         status=experimental bindable=false live_probed=false host_class=any
    Experimental until Jason verifies. Fail closed; no frontier fallback. SKIP (no endpoint env; CI never requires a live box)
  trt          status=experimental bindable=false live_probed=false host_class=any
    Experimental until Jason verifies. Fail closed; no frontier fallback. SKIP (no endpoint env; CI never requires a live box)
  http-remote  status=supported    bindable=true live_probed=false host_class=any
    CELL_LOCAL_ENDPOINT remote pattern. Catalog-level probe. SKIP (no endpoint env; CI never requires a live box)
```

Exit 0. No `5090` / `4090` / `m3-max`.

### live ok (OpenAI-compatible `/v1/models`, including empty list)

Ollama, llama.cpp, and http-remote share `CELL_LOCAL_ENDPOINT`. mlx may
also show `live ok` because it falls back to that env. vLLM / TRT stay
SKIP unless their own env is set.

```
live probe (SKIP without endpoints; not used in CI)
  ollama       status=supported    bindable=true live_probed=true host_class=any
    Ollama-first. Catalog-level probe; not a live ping. live ok (openai /v1/models)
  llama.cpp    status=swap-proof   bindable=true live_probed=true host_class=any
    llama.cpp swap-proof sibling. Same specialist protocol. live ok (openai /v1/models)
  mlx          status=stub         bindable=false live_probed=true host_class=apple-silicon
    Native MLX specialist() is stub. Live Mac proof is Ollama-on-Mac (or OpenAI-compatible) via the HTTP adapter. live ok (openai /v1/models)
  vllm         status=experimental bindable=false live_probed=false host_class=any
    Experimental until Jason verifies. Fail closed; no frontier fallback. SKIP (no endpoint env; CI never requires a live box)
  trt          status=experimental bindable=false live_probed=false host_class=any
    Experimental until Jason verifies. Fail closed; no frontier fallback. SKIP (no endpoint env; CI never requires a live box)
  http-remote  status=supported    bindable=true live_probed=true host_class=any
    CELL_LOCAL_ENDPOINT remote pattern. Catalog-level probe. live ok (openai /v1/models)
```

If the server has no `/v1/models` and answers Ollama `GET /api/tags`,
the note is `live ok (ollama /api/tags)` instead.

### down (endpoint set, nothing listening)

`down:` is followed by the ureq connect error (wording varies by OS).

```
live probe (SKIP without endpoints; not used in CI)
  ollama       status=supported    bindable=true live_probed=false host_class=any
    Ollama-first. Catalog-level probe; not a live ping. down: http://127.0.0.1:1/v1/models: Connection Failed: Connect error: Connection refused (os error 111)
```

llama.cpp / mlx / http-remote look the same. vLLM / TRT still SKIP
without their env. Exit 0.

## Dry on this box (no GPU, no Jason)

```bash
cargo run -q -p estate-control -- probes --live
# expect: the SKIP sample above, exit 0

cargo test -q -p model-estate --lib
cargo test -q -p model-estate --test specialist_cli
cargo test -q -p estate-control --test day90_heal probes_live_skip
```

## Specialist complete (new live surface)

Default job is `complete`. Jason gets a non-empty `completion` field
from the model. Control does not become a gateway; this is the same
`HttpLocal` adapter as `model-estate specialist --job complete`.

### Mock HTTP (this box / CI)

Factory protocol (`mock-local` / `/v0/specialist`):

```bash
# Terminal 1
cargo run -q -p model-estate -- mock-local --bind 127.0.0.1:47831
```

```bash
# Terminal 2
cargo run -q -p estate-control -- specialist --endpoint http://127.0.0.1:47831 \
  --driver ollama --prompt "hello from the factory"
```

Expect exit 0:

```
{
  "allow": true,
  "redacted_text": "hello from the factory",
  "reason": "complete allow",
  "job": "complete",
  "completion": "mock:hello from the factory"
}
```

OpenAI-compatible mock (`CompatServer`) returns `"completion": "ok"`.
Same adapter, `--driver llama.cpp`, posts `/v1/chat/completions`.

Policy-precheck (no model text) still works:

```bash
cargo run -q -p model-estate -- specialist --endpoint http://127.0.0.1:47831 \
  --job policy-precheck --text "hello from the factory"
```

```
{
  "allow": true,
  "redacted_text": "hello from the factory",
  "reason": "policy-precheck allow",
  "job": "policy-precheck"
}
```

Sacred refuse (exit 1, no HTTP POST):

```bash
cargo run -q -p estate-control -- specialist --endpoint http://127.0.0.1:47831 \
  --prompt "please mention cyera"
```

```
{
  "allow": false,
  "redacted_text": "",
  "reason": "policy-precheck denied sacred token 'cyera'",
  "job": "complete"
}
specialist denied
```

Missing `--endpoint` and unset `CELL_LOCAL_ENDPOINT` refuses (exit 1).
SKU endpoint / `CELL_LOCAL_MODEL` / listed model id / prompt /
completion refuses. No silent allow.

### Jason Mac (Ollama already PASSed probes --live)

This is the one new command. Same `CELL_LOCAL_ENDPOINT` as the #23 probe.

```bash
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
# after `ollama pull llama3`, or: export CELL_LOCAL_MODEL=llama3
cargo run -q -p estate-control -- specialist --driver ollama \
  --prompt "Reply with the single word pong."
```

Equivalent:

```bash
cargo run -q -p model-estate -- specialist --job complete \
  --prompt "Reply with the single word pong."
```

Expect exit 0, `"allow": true`, `"job": "complete"`, and a **non-empty
`completion`**. Wording varies by model (often `pong` / `Pong.`). It
must not be the prompt echoed as policy `redacted_text` only. Empty
`completion` is refuse.

llama.cpp server (same OpenAI adapter):

```bash
cargo run -q -p estate-control -- specialist --driver llama.cpp \
  --endpoint http://127.0.0.1:8080 --prompt "Reply with the single word pong."
```

`READY_FOR_LIVE_TEST` for this complete verb: **yes**. Mock HTTP locks
the shape. Jason can run the Mac command above against the Ollama that
already PASSed `probes --live` and get real text back.

## Jason Mac (Apple Silicon) - Ollama-on-Mac

Supported path. Native MLX stays stub. Same commands as #23.

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
mlx may also print `live ok` (fallback to `CELL_LOCAL_ENDPOINT`).
That is not native MLX.

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

Expect ollama / llama.cpp / http-remote `live_probed=true`. vLLM SKIP
unless `CELL_VLLM_ENDPOINT` is set.

Optional vLLM (experimental card; live ping only):

```bash
export CELL_VLLM_ENDPOINT=http://127.0.0.1:8000
cargo run -q -p estate-control -- probes --live
```

Optional llama.cpp server (OpenAI `/v1/models` + `/v1/chat/completions`):

```bash
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:8080
cargo run -q -p estate-control -- probes --live
# expect live ok (openai /v1/models)
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
| `estate specialist` `completion` on mock-local (`mock:...`) | A live Ollama chat Jason ran |
| `estate specialist` `completion` against Ollama | Native MLX, or a 5090-specific path |
