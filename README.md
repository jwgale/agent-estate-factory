# Cell One — Agent Estate Factory

One-box factory. Day 0–30 proves **A1–A4**. Day 31–60 proves **A5–A9** (mixed frontier + local) on the same Horizon / Research / Sanctum estate. Pause-safe. Not Dual PE, not multi-box control, not an AI-gateway product, not a local studio.

**Source of truth:** [github.com/jwgale/agent-estate-factory](https://github.com/jwgale/agent-estate-factory) (private). Future Cursor cloud agents launch with `repo: https://github.com/jwgale/agent-estate-factory`.

```bash
git clone https://github.com/jwgale/agent-estate-factory.git
cd agent-estate-factory
cargo test --workspace
make gate
make gate-60
```

CI is intentionally thin (one `ubuntu-latest` job, `pull_request` only, `cargo test --workspace`). Run gates locally.

Locked defaults: [`charter.md`](charter.md). Documentary schema: [`schema/estate.v0.schema.json`](schema/estate.v0.schema.json). Fail-closed SoT: the Rust validator. See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`AGENTS.md`](AGENTS.md).

## Day-60 gate demo (A5–A9)

Rust 1.88+ (`rust-toolchain.toml`):

```bash
cargo test --workspace
make gate-60
```

`make gate-60` strengthens plan/apply/drift, runs the mixed path (authorize → local precheck → tool/frontier), fail-closes when local is down, checks A3–A4 still hold, writes scrubbed feed traces from **both** paths, and records `gate-reports/day60.md`. Live Grok / live local host are optional and marked `SKIP` when env is absent.

```bash
# A5
cargo run -p estate-control -- plan --estate examples/estate.yaml

# A6
cargo run -p estate-control -- apply --estate examples/estate.yaml --state-dir .cell
cargo run -p estate-control -- drift --estate examples/estate.yaml --state-dir .cell
make pause-stop
cargo run -p estate-control -- drift --estate examples/estate.yaml --state-dir .cell   # expect drift
make pause-start

# catalog (portable drivers — not a model library)
cargo run -p model-estate -- catalog

# A7 / A8 mock (no keys, no GPU)
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent horizon --act model --object xai_grok --mock
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent research --act tool --object notes-append --payload "append a note" --mock

# A8 HTTP specialist protocol (stand-in for any host class)
cargo run -p model-estate -- mock-local --bind 127.0.0.1:47831
# other terminal:
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:47831
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent research --act tool --object notes-append --payload "append a note"

# A7 live Grok (never bake the key; local must be up or the path fail-closes)
export XAI_API_KEY=...
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:47831
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent horizon --act model --object xai_grok \
  --payload "Reply with the single word pong."
```

`estate-control` lists bindings and env *names* only. It does not complete. See [`docs/day60-gate.md`](docs/day60-gate.md) and [`docs/operator-local.md`](docs/operator-local.md).

| Gate | What you should see |
| --- | --- |
| **A5** | Human blast-radius: sessions to bind, equal-class live-capable bindings, control will not invoke. |
| **A6** | Apply in-sync; pause-stop drifts; re-apply converges. |
| **A7** | Horizon completes via frontier after local precheck. Live needs `XAI_API_KEY`. |
| **A8** | Research `notes-append` (and Horizon frontier) run portable `policy-precheck` first. Local down → audited deny, no Grok fallback. |
| **A9** | Both bindings in `examples/estate.yaml` (`xai_grok` + `local_slm`). Sanctum cannot use them. A3–A4 still deny. |

## Local runtime locks

| Rule | Meaning |
| --- | --- |
| Ollama-first | First green local path. llama.cpp is swap-proof. vLLM optional. |
| Remote pattern | Local process on a host; other machines set `CELL_LOCAL_ENDPOINT`. |
| Fail closed | Estate-bound local work does not silently fall through to frontier. Feed: `model.local.down`. |
| Enrich packs | Jason curates; `policy: manual`. See [`examples/enrich-packs/`](examples/enrich-packs/). |
| Supported | Ollama (+ llama.cpp) green on the box. vLLM / TRT experimental until Jason verifies. |
| Portable hosts | `consumer-nvidia` / `apple-silicon` / `rented-nvidia` / `any`. Hardware is a driver, not a fork. |
| Apple | Ollama-on-Mac = Supported. MLX = Stub behind the same catalog / route / bind API. |

Do not put `5090`, `4090`, or `m3-max` in estate binding ids. A 5090 box is one `rented-nvidia` host.

## Day-30 gate demo (A1–A4)

```bash
make gate
```

| Gate | What you should see |
| --- | --- |
| **A1** | `estate validate` lists horizon, research, sanctum on separate lanes. Invalid fixtures exit 1. |
| **A2** | `estate apply` binds three profile-dir sessions. |
| **A3** | Horizon → `lane:research` denied. Cyera CI and Rust classroom cannot be read. |
| **A4** | Undeclared `shell` / `secrets` denied. Research `notes-append` and mount `notes` allowed. |

## Layout

| Crate | Plane | Role |
| --- | --- | --- |
| `estate-schema` | shared | types, validate, hash, compiled intentions, plan, firewall, SKU ban |
| `estate-control` | control | `estate` CLI: validate, plan, apply, drift, models (no complete) |
| `isolation-driver` | data | `IsolationDriver` trait + profile-dir + in-memory |
| `floor-supervisor` | data | bind sessions; snapshot + drift; stop runtime |
| `conveyor-proxy` | data | deny-default tool/mcp/mount/memory/model; not a completer |
| `model-estate` | data | frontier + local catalog/route/bind; mixed path; mock-local protocol |
| `feed-collector` | feed | append-only scrubbed jsonl from proxy **and** both model paths |

Workers call conveyor for allow/deny. Completions go through `model-estate`, which calls the same firewall first. Floor core has no vendor ids.

## Persist vs disposable

Survives pause: charter, estate file, schema, `lanes/`, `plans/`, `gate-reports/`.  
Disposable: `.cell/runtime/`, `.cell/sessions/`, PIDs. Regenerable: `.cell/actual-state.json`, `.cell/desired-snapshot.yaml`, `.cell/model-actual.json`.

## What is stubbed vs live

| Piece | State |
| --- | --- |
| Isolation | Profile dirs (not containers). Trait is swappable. |
| Conveyor HTTP | `POST /v0/check` only. Not a mesh or gateway. |
| Frontier `xai_grok` | Wired driver. Live when `XAI_API_KEY` is set. Tests use mock/HTTP fake. |
| Local `local_slm` | Wired `ollama` driver + `CELL_LOCAL_ENDPOINT`. `mock-local` speaks the protocol. |
| llama.cpp | Swap-proof card; same specialist protocol. |
| MLX | Stub. Same catalog/route/bind. Live Mac proof later. |
| vLLM / TRT | Experimental. Fail closed until Jason verifies. |
| Enrich packs | Curator jason, policy manual, packs empty. |
| Feed | Scrubbed jsonl, both paths. No auto-promote. |
| A10–A12 | Not built. |

## Sharp choices (Jev bait)

Day 0–30 locks kept: multi-crate, Rust validate SoT, own-lane free, vendor-out-of-floor, hash, tiny_http, thin apply/drift (now with snapshot + session-dir drift), Rust-default-not-law, dual-layer sacred, tool slug taxonomy deferred.

Day 60 additions:

1. **`models:` on agents** is a deny-default allow-list (same shape as tools), not a new product surface.
2. **Mixed path is data-plane only.** Control will not complete even when bindings are wired.
3. **A8 is mandatory in-path** when a local binding is wired: no frontier-proxy-only shortcut.
4. **Local protocol is HTTP JSON**, language-free and host-class-free. `mock-local` is a stand-in, not a studio.
5. **Feed events omit prompts and keys**; they record kind/decision/byte counts only.
6. **Live A7 without a local endpoint fails closed** (audited `model.local.down`) so the estate cannot shrink to frontier-only.
7. **Hardware SKUs are banned** from binding ids/drivers. Catalog / route / bind picks Ollama, llama.cpp, MLX, vLLM, or TRT.
8. **Enrich packs stay manual.** Jason curates; feed does not auto-promote.

Anti-shrink list is in the charter.
