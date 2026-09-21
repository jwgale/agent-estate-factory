# Day-60 exit checklist (A5–A9)

Gate = A5–A9 on the same Horizon / Research / Sanctum estate. A1–A4 stay green. Sacred dual-layer stays. SLM locks stay: Ollama-first, portable host classes, fail closed, Jason-curated enrich packs.

```bash
cargo test --workspace
make gate-60
```

## A5 — plan blast radius

`estate plan` writes human blast-radius text to `plans/`. After apply, a second plan diffs against `.cell/desired-snapshot.yaml` (empty blast if unchanged).

## A6 — apply converges; drift detectable

`estate apply` binds sessions, writes `actual-state.json`, `desired-snapshot.yaml`, and `model-actual.json`.  
`estate drift` is in-sync after apply. `make pause-stop` drops session dirs → drift. `make pause-start` converges again.

## A7 — frontier task under the estate

```bash
# mock path (always)
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent horizon --act model --object xai_grok --mock

# live Grok (needs XAI_API_KEY and a local specialist endpoint)
export XAI_API_KEY=...
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:47831   # mock-local or a green Ollama/llama.cpp host
cargo run -p model-estate -- mock-local --bind 127.0.0.1:47831 &
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent horizon --act model --object xai_grok \
  --payload "Reply with the single word pong."
```

Control (`estate`) never completes. Sanctum cannot invoke `xai_grok`.

## A8 — local specialist in-path

The portable local driver runs `policy-precheck` **before** a tool or frontier act. When local is down, the path fail-closes and audits `model.local.down` — no silent frontier fallback.

```bash
# protocol stand-in
cargo run -p model-estate -- mock-local --bind 127.0.0.1:47831
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:47831
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent research --act tool --object notes-append --payload "append a note"
```

Operator proof (any host class): see [`operator-local.md`](operator-local.md).

## A9 — both bindings, same estate, A3–A4

`examples/estate.yaml` names `xai_grok` and `local_slm`, both `wired: true`. Horizon may use both; Research uses local on `notes-append`; Sanctum has no models. Undeclared tool/model and cross-lane reads still deny. Binding ids do not encode a GPU SKU.

## Live vs mock in the gate report

`gate-reports/day60.md` marks A7 live `SKIP` unless `XAI_API_KEY` is set, and A8 live-local `SKIP` unless `CELL_LOCAL_LIVE` is set. Mock/HTTP paths and fail-closed audit must be green either way.
