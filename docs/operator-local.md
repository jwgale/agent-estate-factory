# Operator proof: portable local specialist

Hardware is a **driver choice**. The same estate binding (`local_slm` / driver `ollama`) works on:

| Host class | Typical box | Driver |
| --- | --- | --- |
| `consumer-nvidia` | RTX desktop / laptop | Ollama (Supported) or llama.cpp (swap-proof) |
| `apple-silicon` | Mac laptop | Ollama-on-Mac (Supported) or MLX (Stub; live proof later) |
| `rented-nvidia` | cloud / lab GPU, including a 5090 host | Ollama (Supported) or llama.cpp (swap-proof) |
| `any` | unknown / mixed | `CELL_LOCAL_ENDPOINT` remote pattern |

A 5090 is one rented-Nvidia host, not a product fork. Do not put `5090` / `4090` / `m3-max` in estate binding ids.

Cell One ships:

1. `LocalDriver` trait (`specialist()` + `runtime()`)
2. catalog / route / bind (`model-estate catalog`)
3. `HttpLocal` talking to `$CELL_LOCAL_ENDPOINT` (`/v0/specialist` or OpenAI / Ollama)
4. `model-estate mock-local` - factory protocol stand-in, no GPU
5. `model-estate specialist` - data-plane equivalent of `estate specialist` (control does not execute models)
6. Fail-closed audited deny when local is down (`model.local.down`; frontier hits 0)

This is not LM Studio. No weight browser. No chat UI.

Live probes (up/down only): `GET /v1/models` or Ollama `GET /api/tags`. See [`LIVE-PROBES.md`](LIVE-PROBES.md).

## Protocol

Factory stand-in: `POST {CELL_LOCAL_ENDPOINT}/v0/specialist`

OpenAI-compatible / Ollama / llama.cpp server: the adapter posts the real request text to `/v1/chat/completions` or `/api/chat` after a models list answers. Policy stays factory-owned. A dummy ping is not a round-trip.

```json
{"job":"policy-precheck","agent_id":"horizon","kind":"model","text":"..."}
```

Response:

```json
{"allow":true,"redacted_text":"...","reason":"policy-precheck allow","job":"policy-precheck"}
```

Jobs: `policy-precheck` (A8 default) or `redact`. Bound: fail closed over 16KiB. Do not echo secrets into the feed.

## On the local host (any class)

1. Run a process that speaks this JSON (any language — model drivers are language-free). First green path: Ollama (or llama.cpp as swap-proof). vLLM / TRT are experimental until Jason verifies.
2. Point other machines at it:

```bash
export CELL_LOCAL_ENDPOINT=http://<local-host>:<port>
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent research --act tool --object notes-append --payload "append a note"
```

A green task prints `"path": ["authorize:allow", "local:allow", "tool:allow"]` (or `frontier:complete` for Horizon). That is the A8 operator proof.

If the endpoint is down, the task **denies** with `local:down` and a feed event `model.local.down`. It does not silently complete on Grok.

## Catalog

```bash
cargo run -p model-estate -- catalog
```

Supported = Ollama (+ llama.cpp) green on the box. MLX is stubbed. vLLM / TRT stay experimental.
