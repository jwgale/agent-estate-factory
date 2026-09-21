# Cell One — Agent Estate Factory

**Start here:** `make gate-90` (Day-90 operator entrypoint) · [`docs/OPERATOR-DAY.md`](docs/OPERATOR-DAY.md) (gate-90 → feed-loop → backup prune) · [`docs/FEED-LOOP.md`](docs/FEED-LOOP.md) · [`docs/GATE-90.md`](docs/GATE-90.md) · [`docs/DAY90-PLUS.md`](docs/DAY90-PLUS.md) · [`CHANGELOG.md`](CHANGELOG.md). Morning brief (21 Sep): [`docs/MORNING-BRIEF-2026-09-21.md`](docs/MORNING-BRIEF-2026-09-21.md). Snapshot: [`docs/CELL-ONE-STATUS.md`](docs/CELL-ONE-STATUS.md).

One-box factory. Day 0–30 proves **A1–A4**. Day 31–60 proves **A5–A9** (mixed frontier + local) on the same Horizon / Research / Sanctum estate. Pause-safe. Not Dual PE, not multi-box control, not an AI-gateway product, not a local studio.

**Source of truth:** [github.com/jwgale/agent-estate-factory](https://github.com/jwgale/agent-estate-factory) (private). Future Cursor cloud agents launch with `repo: https://github.com/jwgale/agent-estate-factory`.

```bash
git clone https://github.com/jwgale/agent-estate-factory.git
cd agent-estate-factory
cargo test --workspace
make gate
make gate-60
make gate-90    # Day-90 operator entrypoint: smoke + day90 + doctor --strict + checklist
make feed-loop  # fixtures only: scrubbed trace → pack → propose → accept
make operator-day   # fixtures only: suspend → plan → apply → feed import → resume
estate help     # Day-90 topic pages (status / plan / apply / reconcile / feed-loop / backup)
```

Hosted CI is **compile-only** (`cargo check --workspace --locked` on `pull_request`). Real `cargo test --workspace` and `make gate*` / `make smoke` stay local. Do not add `cargo test` or `make gate-90` to Actions — gate-90 wraps the local test suite. Walk without live boxes: [`docs/OPERATOR-DAY.md`](docs/OPERATOR-DAY.md). `.cell/` paths: [`docs/cell-layout.md`](docs/cell-layout.md).

## Day-90 operator entrypoint

Local only. Hosted CI stays compile-only. This is the command Jason runs after Day 90:

```bash
make gate-90
```

That alias runs `make smoke` (doctor + fixtures-check + operator-day + `cargo test --workspace` + `make day90`), then `estate doctor --strict`, then prints the GATE-90 checklist. It does not spawn cloud agents. It does not auto-promote packs. It does not require a Mac, a GPU, or live Grok.

Live Mac MLX, live consumer/rented GPU (including a 5090-class box), and cloud-agent spawn are parked in [`docs/DAY90-PLUS.md`](docs/DAY90-PLUS.md) until Jason has boxes ready. Exact env + commands: [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md). Those rows are not green. Do not fake them.

```bash
make feed-loop   # isolated fixture walk: scrubbed trace → pack → propose → accept
```

Walk both loops without live boxes: [`docs/OPERATOR-DAY.md`](docs/OPERATOR-DAY.md) · [`docs/FEED-LOOP.md`](docs/FEED-LOOP.md). Checklist: [`docs/GATE-90.md`](docs/GATE-90.md).

Locked defaults: [`charter.md`](charter.md). Documentary schema: [`schema/estate.v0.schema.json`](schema/estate.v0.schema.json). Fail-closed SoT: the Rust validator. See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`AGENTS.md`](AGENTS.md). Lease files: [`docs/cell-layout.md`](docs/cell-layout.md).

## What works tonight vs stubs

This is a factory, not a gateway, not LM Studio, not a shrink-to-frontier proxy. Origin is not in the loop. GitHub is SoT.

| Works tonight (local cargo) | Still a stub |
| --- | --- | --- |
| `estate validate` on `examples/estate.yaml` + host matrix + `examples/hosts/multi-host.yaml` | Live Mac MLX proof |
| `estate plan` / `apply --require-plan` / `--require-fresh-plan` + Security-as-IaC markdown | Cloud-agent spawn (declared only) |
| `estate suspend` / `resume` / `status` / `history` | Multi-box control plane |
| `estate reconcile` desired vs actual + sacred-id deny on tampered leases | Auto-heal / rewrite of leases |
| `estate packs list\|import\|propose` — propose writes `packs/proposed/`, never applies | Feed auto-promote (locked off) |
| `make gate-90` — Day-90 operator entrypoint (local) | Live Mac MLX / GPU / cloud-spawn ([`DAY90-PLUS.md`](docs/DAY90-PLUS.md)) |
| `make feed-loop` — scrubbed trace → pack → propose → accept | Actions expansion (Jason lock) |
| `estate doctor --strict` — pre-merge operator checks (compile-only CI, locked sacred, demo estate) | Curator UI (not built) |
| Dual-layer sacred demo `examples/fixtures/dual-layer-demo.yaml` (Sanctum is not Cyera) | Dual PE product (out of altitude) |
| `estate convey hop\|call\|leases` — lease-bound; refuse codes prefixed `refuse:` | Real hop transport |
| `estate audit export` — local folder / optional tarball | Remote audit upload |
| Ollama + `CELL_LOCAL_ENDPOINT` + llama.cpp swap-proof card | vLLM / TRT until Jason verifies |
| Isolation profile-dir + placement leases under `.cell/` | Containers / vendor isolation |

Do not turn this into an AI gateway. Do not auto-promote enrich packs. Do not spawn `cursor-cloud`.

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
cargo run -p estate-control -- specialist --driver ollama --prompt "hello from the factory"
# live box (opt-in; not in smoke): make live-specialist
cargo run -p model-estate -- specialist --job complete --prompt "hello from the factory"
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent research --act tool --object notes-append --payload "append a note"

# A7 live Grok (never bake the key; local must be up or the path fail-closes)
export XAI_API_KEY=...
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:47831
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent horizon --act model --object xai_grok \
  --payload "Reply with the single word pong."
```

`estate-control` lists bindings and env *names* only. The exception is the thin `estate specialist` complete delegate (`HttpLocal`, not a gateway). See [`docs/day60-gate.md`](docs/day60-gate.md) and [`docs/operator-local.md`](docs/operator-local.md).

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
| Enrich packs | Jason curates; `policy: manual`. Live drop zone: [`packs/`](packs/). |
| Supported | Ollama (+ llama.cpp) green on the box. vLLM / TRT experimental until Jason verifies. |
| Portable hosts | `consumer-nvidia` / `apple-silicon` / `rented-nvidia` / `any`. Hardware is a driver, not a fork. |
| Apple | Ollama-on-Mac = Supported. MLX = Stub behind the same catalog / route / bind API. |

Do not put `5090`, `4090`, or `m3-max` in estate binding ids **or probe ids**. A 5090 box is one `rented-nvidia` host.

## Live probe env (optional)

One-page hand-off (not a live-box proof): [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md).

CI never sets these. `estate probes --live` (or `CELL_LIVE_PROBE=1`) SKIPs when the endpoint is unset and exits 0.

| Variable | Use |
| --- | --- |
| `CELL_LIVE_PROBE` | `1` / `true` / `yes` to opt into live HTTP |
| `CELL_LOCAL_ENDPOINT` | Ollama / llama.cpp / http-remote (`GET /v1/models` or `/api/tags`) |
| `CELL_RENTED_ENDPOINT` | Alias for a rented (or any) box. Not a SKU. |
| `CELL_MLX_ENDPOINT` | Apple MLX; falls back to `CELL_LOCAL_ENDPOINT` |
| `CELL_VLLM_ENDPOINT` | Experimental; unset = SKIP |
| `CELL_TRT_ENDPOINT` | Experimental; unset = SKIP |
| `CELL_FRONTIER_ENDPOINT` | `estate specialist --driver frontier` / AI-gateway. Not probes. `XAI_API_KEY` does not unlock. |
| `XAI_API_KEY` / `XAI_API_BASE` / `XAI_MODEL` | Frontier live A7. Not used by probes or `--driver frontier`. |

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

Survives pause: charter, estate file, schema, `lanes/`, `plans/`, `plans/reviewed/`, `gate-reports/`, `.cell/lifecycle.json`, `.cell/lifecycle.jsonl`, `.cell/placement-actual.json`, `.cell/apply-audit.jsonl`, `.cell/sessions.jsonl`, `.cell/feed/feed-cursor.json`, `.cell/conveyor-mesh.json`, `.cell/conveyor-hops.json`, `.cell/conveyor-leases.json`.  
Disposable: `.cell/runtime/`, `.cell/sessions/`, PIDs. Regenerable: `.cell/actual-state.json`, `.cell/desired-snapshot.yaml`, `.cell/model-actual.json`, `.cell/catalog.json`, `.cell/reconcile.json`, `.cell/reconcile.md`. Patch file only: `.cell/reconcile-suggest.md`. Local review: `.cell/audit-export/` (not uploaded). Cell archives live under `backups/`, not `.cell/`. See [`docs/cell-layout.md`](docs/cell-layout.md).

## What is stubbed vs live

| Piece | State |
| --- | --- |
| Isolation | Profile dirs (not containers). Trait is swappable. |
| Conveyor HTTP | `POST /v0/check` only. Capability mesh is a lease-bound stub (`estate convey`). Not a gateway. |
| Frontier `xai_grok` | Wired driver. Live when `XAI_API_KEY` is set. Tests use mock/HTTP fake. |
| Local `local_slm` | Wired `ollama` driver + `CELL_LOCAL_ENDPOINT`. `mock-local` speaks the protocol. |
| llama.cpp | Swap-proof card; same specialist protocol. |
| MLX | Stub. Same catalog/route/bind. Live Mac proof later. |
| vLLM / TRT | Experimental. Fail closed until Jason verifies. |
| Enrich packs | Curator jason, policy manual, packs empty. Live drop zone: `packs/`. Import is explicit and does not rewrite the estate. |
| Feed | Scrubbed jsonl, both paths. Candidate packs. No auto-promote. |
| A10–A12 | Beachhead: feed packs + import, suspend/resume + placement leases, gated/auditable apply, cloud-agent stub. |

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

Day 61–90 beachhead (local `make gate-90`):

9. **Feed packs are candidates.** `estate feed pack` writes `packs/`; `estate feed import` is explicit apply; `estate feed promote` fails. Jason edits the estate.
10. **Suspend/resume is the operator lifecycle.** `.cell/lifecycle.json` survives session discard. Placement leases are regenerable on disk.
11. **Plans are the human control surface.** Reviewable markdown + `estate plans` history. `apply --require-plan` is gated and audited. Commit a plan file when apply needs a PR review.
12. **`placements[]` declares `box` and a `cloud-agent` stub.** `PlacementDriver` records leases; it does not spawn cloud agents. Drift fail-closes a spawned cloud lease or host_class mismatch. `schema/local-catalog.v0.json` is the catalog file SoT.
13. **Feed cursor + lifecycle history + plan freshness** are file-durable. `apply --require-fresh-plan` checks `against_hash`. Driver `probe()` is catalog-level (`live_probed=false`).
14. **Wave 2 (same PR):** `estate convey` mesh, Security-as-IaC `{stem}.security.md` + `--require-fresh-plan` strict, `estate packs`, `make operator-day`, host-class aliases + `examples/hosts/`.
15. **Wave 3 (same PR):** `estate reconcile` + refuse codes + sacred-id tamper deny; `estate packs propose` (never auto-apply); multi-host fixture; `estate audit export`; `.cell/` layout doc.

Overnight assumptions: [`docs/overnight-decisions.md`](docs/overnight-decisions.md). Anti-shrink list is in the charter.
