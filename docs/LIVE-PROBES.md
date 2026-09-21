# Live probes (hand-off page)

Not green. No Mac and no GPU is attached to this factory. This page is
the exact env + command list for when Jason is at a box. Until then,
`estate probes --live` prints SKIP and exits 0. CI never sets these
variables.

Parked status: [`DAY90-PLUS.md`](DAY90-PLUS.md). Catalog cards:
`schema/local-catalog.v0.json`. Dry shapes (no network):
`schema/live-probe-shapes.v0.json`.

Do not put `5090`, `4090`, or `m3-max` in estate binding ids or probe ids.
A 5090 is one `consumer-nvidia` or `rented-nvidia` host.

## What a live probe actually pings

Not the Ollama chat API. Not an MLX Python REPL. The factory speaks one
JSON protocol:

```
POST {endpoint}/v0/specialist
```

```json
{"job":"policy-precheck","agent_id":"probe","kind":"probe","text":"ping"}
```

Timeout is 800ms. A 2xx JSON body is `live_probed=true` / `live ok`.
Connect or HTTP error is `live_probed=false` / `down: ...`. Unset
endpoint is `SKIP`. The command still exits 0 in all three cases.

`model-estate mock-local` speaks this protocol **without a GPU**. That is
a protocol stand-in, not a live-box checkmark. A real Ollama or MLX
process still needs a specialist that speaks this JSON. Cell One does
not ship an Ollama-chat-to-specialist adapter.

MLX `specialist()` stays `Stub` until Jason verifies. A green mlx ping
only means the HTTP specialist answered. It does not flip the catalog
card to Supported.

## Env

| Variable | Mac MLX | Linux / 5090-class |
| --- | --- | --- |
| `CELL_LIVE_PROBE` | `1` / `true` / `yes` opts into live HTTP if you omit `--live` | same |
| `CELL_MLX_ENDPOINT` | first choice for the mlx card | unused (mlx SKIP) |
| `CELL_LOCAL_ENDPOINT` | mlx fallback; Ollama-on-Mac Supported path | Ollama / llama.cpp / http-remote |
| `CELL_RENTED_ENDPOINT` | unused | alias for a rented (or any) box. Not a SKU. |
| `CELL_VLLM_ENDPOINT` | experimental; unset = SKIP | experimental; unset = SKIP |
| `CELL_TRT_ENDPOINT` | experimental; unset = SKIP | experimental; unset = SKIP |

`XAI_*` is frontier A7. Probes do not use it.

## Dry on this box (no network, no Jason)

```bash
cargo run -q -p estate-control -- probes --live
# expect: SKIP, live_probed=false, exit 0, no 5090 / 4090 / m3-max

cargo test -q -p model-estate --lib live_probe_shapes_skip_vs_would_live_without_network
cargo test -q -p estate-control --test day90_heal probes_live_skip
```

Would-live is a fixture overlay (`LiveOverlay::WouldLive`). It does not
open a socket.

## Mac MLX (Apple Silicon)

host_class on the mlx card is `apple-silicon`. Catalog status stays
`stub`, `bindable=false`. Ollama-on-Mac is the Supported Apple path
(same `ollama` card, `CELL_LOCAL_ENDPOINT`).

Terminal 1 - specialist that already speaks `/v0/specialist` (stand-in
until a real MLX specialist exists):

```bash
cargo run -p model-estate -- mock-local --bind 127.0.0.1:47831
```

Terminal 2:

```bash
export CELL_MLX_ENDPOINT=http://127.0.0.1:47831
cargo run -q -p estate-control -- probes --live
```

Expect:

- mlx: `status=stub bindable=false live_probed=true host_class=apple-silicon` and note `live ok`
- ollama / llama.cpp / http-remote: SKIP unless `CELL_LOCAL_ENDPOINT` is also set

Supported Ollama-on-Mac (not MLX):

```bash
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:47831
cargo run -q -p estate-control -- probes --live
```

Expect ollama / llama.cpp / http-remote `live_probed=true`. mlx SKIP.

## Linux / 5090-class (consumer-nvidia or rented-nvidia)

Pick the portable host class for the box. Never encode `5090` in a
binding id.

```bash
# desktop / laptop RTX
# host_class=consumer-nvidia
# rented / lab GPU (including a 5090 host)
# host_class=rented-nvidia

cargo run -p model-estate -- mock-local --bind 127.0.0.1:47831
```

```bash
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:47831
# or: export CELL_RENTED_ENDPOINT=http://127.0.0.1:47831
cargo run -q -p estate-control -- probes --live
```

Expect ollama / llama.cpp / http-remote `live_probed=true`. mlx SKIP
(no `CELL_MLX_ENDPOINT`). vLLM / TRT SKIP unless their env is set.

Optional A8 task after a real specialist (not mock-local-as-proof):

```bash
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent research --act tool --object notes-append --payload "append a note"
```

Down local is `local:down` / `model.local.down`. No silent Grok fallback.

## What green is not

| You saw | What it is not |
| --- | --- |
| SKIP in CI or on this cloud box | A failed live test |
| `live_probed=true` from `mock-local` on a Mac or 5090 | MLX Supported, or Ollama weights green |
| mlx note `live ok` | Catalog card flipped off stub |
| This page existing | A Mac or GPU attached to the factory |

When a real specialist (not mock-local) on a Mac or Linux GPU answers
`POST /v0/specialist`, re-run the commands above and keep this page.
Until that adapter exists on the box, do not ping Jason.
