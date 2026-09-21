# Live probes (hand-off page)

Copy-paste for Jason at a Mac or a Linux/5090-class box. CI never sets
these variables. Without an endpoint, `estate probes --live` prints SKIP
and exits 0.

Parked status: [`DAY90-PLUS.md`](DAY90-PLUS.md). Dry shapes (no network):
`schema/live-probe-shapes.v0.json`.

Do not put `5090`, `4090`, or `m3-max` in estate binding ids or probe ids.
A 5090 is one `consumer-nvidia` or `rented-nvidia` host.

There is no `estate specialist`. Control does not execute tools or models.
The equivalent is `model-estate specialist` (data plane). Mock-locked here;
do not ping Jason to run it live.

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

## Specialist chat (not `estate specialist`)

Control does not have this verb. Data-plane equivalent:

```bash
# Terminal 1 - factory protocol, no GPU
cargo run -q -p model-estate -- mock-local --bind 127.0.0.1:47831
```

```bash
# Terminal 2
cargo run -q -p model-estate -- specialist --endpoint http://127.0.0.1:47831 \
  --text "hello from the factory"
```

Expect exit 0:

```
{
  "allow": true,
  "redacted_text": "hello from the factory",
  "reason": "policy-precheck allow",
  "job": "policy-precheck"
}
```

Sacred refuse (exit 1):

```bash
cargo run -q -p model-estate -- specialist --endpoint http://127.0.0.1:47831 \
  --text "please mention cyera"
```

```
{
  "allow": false,
  "redacted_text": "",
  "reason": "policy-precheck denied sacred token 'cyera'",
  "job": "policy-precheck"
}
specialist denied
```

Missing `--endpoint` and unset `CELL_LOCAL_ENDPOINT` refuses (exit 1).
No silent allow.

Compat chat (OpenAI / Ollama / llama.cpp server) posts the **request
text**, not a dummy `ping`. Policy stays `builtin_specialist`. In-process
lock: `HttpLocal` + `CompatServer` (including `llama.cpp` runtime against
OpenAI `/v1/chat/completions`).

Optional later, on a box that already has Ollama from the #23 probe
(not a new Jason ping):

```bash
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
# after `ollama pull llama3`, or set CELL_LOCAL_MODEL=llama3
cargo run -q -p model-estate -- specialist --text "hello from the factory"
```

llama.cpp server (same OpenAI adapter; mock-locked here):

```bash
cargo run -q -p model-estate -- specialist --runtime llama.cpp \
  --endpoint http://127.0.0.1:8080 --text "hello from the factory"
```

`READY_FOR_LIVE_TEST` for this chat verb: **no**. Mock HTTP is the proof.
Do not ping Jason unless a new surface needs a box (native MLX, a real
live specialist round-trip he has not already been asked for, or a
5090-specific path).

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
| `model-estate specialist` allow on mock-local | A live Ollama chat Jason ran |
