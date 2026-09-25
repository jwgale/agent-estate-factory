# Cell One — Agent Estate Factory

Next-generation harness and custom AI creator suite: first-class agents under a security-first control suite, a Grok Bot–like harness, and on-spot specialty SLMs, on one box with sacred isolation (Cyera CI + Rust classroom out; Sanctum is not Cyera) and equal-class frontier+local.

Suite pillars, in order: agents under security-as-IaC, a Grok Bot–like harness (the UI may still be deferred), and on-spot specialty SLMs. A central learning brain stays parked. Security posture is akin to a service mesh for agents; the IaC control catalog is still being designed. Whether Sanctum holds credentials is an open call, not a vault. Placement may be fully local, cloud, or mixed. Plan 12–18 months ahead of the market. The local-runtime seat (Ollama today, another process tomorrow) and train/enrich for purpose-built small models are one facet. Ollama is the local-run seat. Integrate the driver that already does the job. `estate enrich prepare` writes artifacts (one driver, or `--all-drivers`). `estate enrich from-pack` does that for an accepted pack. Modelfile `FROM` is the seated model (`params.model` or a model-tag hint), never the binding id `local_slm`. `estate enrich list` reads `.cell/enrich`. `estate enrich import-prepared` writes a `local_slm` binding proposal. `llamafactory-lora` writes a LLaMA-Factory LoRA recipe with no quantization, and `llamafactory-qlora` writes the QLoRA recipe (`estate enrich prepare --job train`). `model_name_or_path` is a train base, separate from the Ollama seat tag. `axolotl-lora` writes the bf16 Axolotl YAML and `axolotl-qlora` writes the 4-bit Axolotl YAML, each with that same train base in `base_model`. `import-trained` records the adapter on that same proposal. `estate enrich gguf-convert` prints the llama.cpp `convert_hf_to_gguf.py` line for a merged export (`--outtype auto`, outfile beside the directory) and does not convert. `estate enrich local-seat` validates a merged export directory or a GGUF and prints the `ollama create` next step. Neither command runs Ollama or llama.cpp. Optional `--from-feed` copies instruct rows already under the cell state directory into `dataset.jsonl` on those four train cards. The default prepare keeps a scaffold and does not download sources. The factory does not run LLaMA-Factory or Axolotl. `estate enrich apply-proposal` stages that binding for `estate plan` and `estate apply --require-plan`. The source estate is written when that apply succeeds. Packs stay curator edit instructions. Control does not complete. Prepare: [`docs/TRAIN-ENRICH.md`](docs/TRAIN-ENRICH.md). Walks: [`docs/operator-enrich-journeys.md`](docs/operator-enrich-journeys.md).

**Start here:** [`docs/NORTH-STAR.md`](docs/NORTH-STAR.md) · [`docs/UBIQUITOUS_LANGUAGE.md`](docs/UBIQUITOUS_LANGUAGE.md) · `make gate-90` (local Day-90 entrypoint) · `make purpose-build-journey` (when an SLM fits mid-software-build, or on demand; print-only; runs `make purpose-build-pick`, then `make purpose-build-checklist`) · [`docs/OPERATOR-DAY.md`](docs/OPERATOR-DAY.md) (gate-90 → day90 → feed-loop → `make real-world`) · [`docs/TRAIN-ENRICH.md`](docs/TRAIN-ENRICH.md) · [`docs/local-seat.md`](docs/local-seat.md) · [`docs/operator-enrich-journeys.md`](docs/operator-enrich-journeys.md) · [`docs/FEED-LOOP.md`](docs/FEED-LOOP.md) · [`docs/GATE-90.md`](docs/GATE-90.md) · [`docs/DAY90-PLUS.md`](docs/DAY90-PLUS.md) · [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) · [`CHANGELOG.md`](CHANGELOG.md). Snapshot: [`docs/CELL-ONE-STATUS.md`](docs/CELL-ONE-STATUS.md).

Day 0–90 is on `main` (A1–A4, A5–A9, A10–A12 beachhead) on the same Horizon / Research / Sanctum estate. Day 90+ is real-world proof on Jason's boxes, plus parked stubs. Pause-safe. Charter (locked defaults): [`charter.md`](charter.md).

Opt-in, off the gate: `make real-world`, `estate help frontier`, `make day90-mixed`, `estate help enrich`, `make enrich-prepare`, `make qlora-journey`, `make lora-journey`, `make seat-journey`, `make train-next`, `make uniqueness-ladder`, `make uniqueness-full`, `make uniqueness-prove-checklist`, `make purpose-build-checklist`, `make purpose-build-pick`, `make purpose-build-journey`, `make deepseek-r1-distill-journey`, `make uniqueness-deepseek`, `make deepseek-r1-distill-lora-journey`, `make uniqueness-deepseek-lora`, `make glm4-chat-journey`, `make uniqueness-glm`, `make glm4-chat-lora-journey`, `make uniqueness-glm-lora`, `make mlx-lm-lora-journey`, `make uniqueness-mlx`, and `make enrich-live-prove`. `make deepseek-r1-distill-journey` is the print-only DeepSeek-R1-Distill chat QLoRA ladder (operator section 19). `make uniqueness-deepseek` is the print-only chain of that journey. `make deepseek-r1-distill-lora-journey` and `make uniqueness-deepseek-lora` are the LoRA twin. `make glm4-chat-journey` is the print-only GLM-4 Chat QLoRA ladder (operator section 20). `make uniqueness-glm` is the print-only chain of that journey. `make glm4-chat-lora-journey` and `make uniqueness-glm-lora` are the LoRA twin. `make qlora-journey` prints the Qwen / LLaMA-Factory QLoRA ladder (Target C) and checks prepare artifacts. `make lora-journey` prints the Qwen / LLaMA-Factory LoRA ladder (Target A) and checks prepare artifacts. `make seat-journey` asserts `refuse:tokenizer` on a 5090-shaped export fixture, then prints the Target C merge, convert, seat, and import lines against fixture stubs. None of those three trains, converts, or creates an Ollama model. `make train-next` prints the Target C train recipe from `NEXT.md` and does not train. `make uniqueness-ladder` runs the qlora and seat prints and does not run `make train-next`. `make uniqueness-full` runs `make qlora-journey`, then `make train-next`, then `make seat-journey`, and does not train. `make uniqueness-prove-checklist` prints the recorded Target C live uniqueness operator steps, then Standing next (estate) after `import-trained`, and does not invent a live PASS. It does not execute `plan`, `apply`, or `reconcile`. `make purpose-build-checklist` is the print-only operator path for purpose-building an SLM on demand (operator section 15). It points at the print-only cards already on tip, including `make mlx-lm-lora-journey`, `make deepseek-r1-distill-journey`, `make uniqueness-deepseek`, `make glm4-chat-journey`, and `make uniqueness-glm`, plus the LoRA twins, and does not run them. `make purpose-build-pick` is the print-only host and stack picker for purpose-build journeys (operator section 17). It names those targets and does not run them. When an SLM fits mid-software-build, or on demand, the same print-only entry is `make purpose-build-journey`. `make purpose-build-journey` is the print-only purpose-build on-demand entry (operator section 18). It runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. It does not invent a live PASS. `make mlx-lm-lora-journey` is the print-only Apple Silicon mlx-lm LoRA journey (operator section 16). `make uniqueness-mlx` is the print-only chain of that journey. It does not invent a live PASS. The mixed fixture and `examples/hosts/frontier-http.yaml` name `model: grok-4.7` on the frontier `http-remote` binding. `examples/estate.yaml` stays hash-locked and does not set `params.model` on `local_slm`. Enrich prepare writes artifacts and does not train. The live prove runs `ollama create` only when you ask, then removes the tag. It is not a factory-wide live test.

**Source of truth:** [github.com/jwgale/agent-estate-factory](https://github.com/jwgale/agent-estate-factory) (private). Future Cursor cloud agents launch with `repo: https://github.com/jwgale/agent-estate-factory`.

```bash
git clone https://github.com/jwgale/agent-estate-factory.git
cd agent-estate-factory
cargo test --workspace
make gate
make gate-60
make gate-90      # local Day-90 entrypoint: smoke + day90 + doctor --strict + checklist
make feed-loop    # fixtures only: scrubbed trace → pack → propose → accept
make real-world   # opt-in live-box ladder: check + vanilla doctor; live SKIP without CELL_LOCAL_ENDPOINT
make enrich-prepare # opt-in: Modelfile + external manifest; not a live train; not in smoke or gate-90
make enrich-live-prove # opt-in: ollama create on a throwaway cell when the seat is up; not in smoke or gate-90
make operator-day # fixtures only: suspend → plan → apply → feed import → resume
estate help       # Day-90 topics, including north-star, charter, and enrich
```

Hosted CI is **compile-only** (`cargo check --workspace --locked` on `pull_request`). Real `cargo test --workspace` and `make gate*` / `make smoke` / `make real-world` stay local. Do not add `cargo test`, `make gate-90`, or `make real-world` to Actions — gate-90 wraps the local test suite. Walk without live boxes: [`docs/OPERATOR-DAY.md`](docs/OPERATOR-DAY.md). Product page: [`docs/NORTH-STAR.md`](docs/NORTH-STAR.md). Words: [`docs/UBIQUITOUS_LANGUAGE.md`](docs/UBIQUITOUS_LANGUAGE.md). `.cell/` paths: [`docs/cell-layout.md`](docs/cell-layout.md).

## Parked

Stubs, experimental catalog cards, and declared-only rows.

| Item | State |
| --- | --- |
| Native MLX | Catalog stub. Ollama-on-Mac is the supported Apple runtime (`ollama` driver). |
| vLLM / TRT | Experimental catalog cards. Unset endpoint is SKIP. Probe string stays `not live-ok`. |
| Cloud-agent spawn | `cursor-cloud` is a declared lease. Floor records the lease. Spawn stays parked. |
| `estate convey` | Lease-bound hop stub. |
| Packs | Curator edit instructions. `policy: manual`. Jason pastes them into the estate. |
| Control | Control does not complete. |

Recorded live proofs, including the MacBook Air specialist `Pong`: [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md). Parking lot: [`docs/DAY90-PLUS.md`](docs/DAY90-PLUS.md). Morning brief (21 Sep): [`docs/MORNING-BRIEF-2026-09-21.md`](docs/MORNING-BRIEF-2026-09-21.md).

## Day-90 operator entrypoint

Local only. Hosted CI stays compile-only. This is the command Jason runs after Day 90:

```bash
make gate-90
```

That alias runs `make smoke` (doctor + fixtures-check + operator-day + `cargo test --workspace` + `make day90`), then `estate doctor --strict`, then prints the GATE-90 checklist. It does not spawn cloud agents. It does not auto-promote packs. It does not require a Mac, a GPU, or live Grok.

After that gate, the live-box ladder is `make real-world`. It is opt-in. Unset `CELL_LOCAL_ENDPOINT` prints SKIP and exits 0. That SKIP is not a PASS. Exact env + commands: [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md). Recorded proofs stay recorded. Do not fake a new one.

```bash
make feed-loop   # isolated fixture walk: scrubbed trace → pack → propose → accept
```

Walk both loops without live boxes: [`docs/OPERATOR-DAY.md`](docs/OPERATOR-DAY.md) · [`docs/FEED-LOOP.md`](docs/FEED-LOOP.md). Checklist: [`docs/GATE-90.md`](docs/GATE-90.md).

Locked defaults: [`charter.md`](charter.md). Documentary schema: [`schema/estate.v0.schema.json`](schema/estate.v0.schema.json). Fail-closed SoT: the Rust validator. See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`AGENTS.md`](AGENTS.md). Lease files: [`docs/cell-layout.md`](docs/cell-layout.md).

## Works tonight

GitHub is the source of truth.

| On this checkout | Held |
| --- | --- |
| `estate validate` / `plan` / `apply` on `examples/estate.yaml` and the host matrix | Multi-box control plane |
| `estate suspend` / `resume` / `status` / `history` / `reconcile` | Auto-heal of leases |
| `estate packs` propose + accept — curator edit instructions | Promote stays refused |
| `make gate-90` local entrypoint; `make feed-loop` fixtures | Actions expansion (Jason lock) |
| `make real-world` — check + vanilla doctor; live SKIP without an endpoint | Off smoke, gate-90, and Actions |
| `estate doctor --strict` and the dual-layer sacred demo (Sanctum is not Cyera). Doctor, `estate drift`, and `estate plan` cite a placement hop capability that disagrees with hop coverage (`refuse:hop-coverage` mismatch) and do not write the mesh. Drift and plan deny and deny-default cites do not fail those commands alone | Dual PE |
| `estate convey` lease-bound hop stub. `sync` stamps placement agents. `hop` and `sync` refuse a placement capability that disagrees with hop coverage (`refuse:hop-coverage` mismatch) and do not write the mesh. `hop` and `call` with `--agent` refuse intention deny and deny-default (`refuse:intention`) before hop coverage and the lease. Kind `agent` (`agent_call`, `agent-call` load in estate YAML) is who may call whom: object is an estate agent (`agent:` or bare id) declared on `calls`; missing coverage is deny-default. When the id overlaps a tool, MCP, mount, or model, pass `--kind agent` or `--intention-kind agent`, or use `agent:`. `lane-tool` and `mesh-stub` are not inferred as Agent. Plan, drift, and doctor cite `own` or `peer`. Allow continues. Ambiguous hop capability: `hop --intention-kind` (not hop `--kind`). `authority` is a file check (`would-allow`, `would-deny`, `not-enforced`); a granted box lease cites hop-coverage deny, deny-default, or capability mismatch as `would-deny` | Hop transport; whether a hop lease is the right station for authority; identity equation (parked) |
| `ollama` and llama.cpp on `CELL_LOCAL_ENDPOINT` | Experimental catalog cards — [parked](#parked) |
| Profile-dir isolation and placement leases under `.cell/` | Containers |

Anti-shrink: AI gateway, Ollama wrapper-as-product, LM Studio-alone, a thin Grok Bot clone without the estate. Also MCP catalog, a UI-only shell, weight browser, undirected agent sprawl, frontier-proxy-only, local-studio-only. The harness look and feel is the product surface. Agents and controlled spin-up stay first-class. Promote stays manual. `cursor-cloud` stays a declared lease. Train/enrich facilitation of purpose-built small models stays one facet of the suite.

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

# A7 live frontier (model grok-4.7; never bake the key; local must be up or the path fail-closes)
export XAI_API_KEY=...
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:47831
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent horizon --act model --object xai_grok \
  --payload "Reply with the single word pong."
```

`estate-control` lists bindings and env *names* only. The thin `estate specialist` complete delegate is `HttpLocal` on the data plane. See [`docs/day60-gate.md`](docs/day60-gate.md) and [`docs/operator-local.md`](docs/operator-local.md).

| Gate | What you should see |
| --- | --- |
| **A5** | Human blast-radius: sessions to bind, equal-class live-capable bindings, control will not invoke. |
| **A6** | Apply in-sync; pause-stop drifts; re-apply converges. |
| **A7** | Horizon completes via frontier `grok-4.7` after local precheck. Live needs `XAI_API_KEY`. |
| **A8** | Research `notes-append` (and Horizon frontier) run portable `policy-precheck` first. Local down → audited deny, no silent `grok-4.7` fallback. |
| **A9** | Both bindings in `examples/estate.yaml` (`xai_grok` + `local_slm`). Sanctum cannot use them. A3–A4 still deny. |

## Local runtime locks

| Rule | Meaning |
| --- | --- |
| Ollama-first | Today's entrant in the local-runtime seat: the `ollama` driver. llama.cpp swaps on the same protocol. The seat stays open. |
| Remote pattern | Local process on a host; other machines set `CELL_LOCAL_ENDPOINT`. |
| Fail closed | Estate-bound local work stops on `model.local.down`. |
| Enrich packs | Jason curates; `policy: manual`. Curator edit instructions. `estate enrich prepare` writes artifacts from a pack and does not train. `estate enrich apply-proposal` stages the `local_slm` join. `estate plan` and `estate apply --require-plan` write the source estate. `examples/estate.yaml` stays hash-locked; the fixture uses a lab copy. Drop zone: [`packs/`](packs/). Prepare: [`docs/TRAIN-ENRICH.md`](docs/TRAIN-ENRICH.md). Walks: [`docs/operator-enrich-journeys.md`](docs/operator-enrich-journeys.md). |
| Supported | `ollama` and llama.cpp green on the box. |
| Portable hosts | `consumer-nvidia` / `apple-silicon` / `rented-nvidia` / `any`. Hardware is a driver choice. |
| Apple | Ollama-on-Mac is the supported Apple runtime. Same `ollama` driver. |

Do not put `5090`, `4090`, or `m3-max` in estate binding ids **or probe ids**. A 5090 box is one `rented-nvidia` host. Native MLX, vLLM, and TRT stay parked catalog cards.

## Live probe env (optional)

One-page hand-off (not a live-box proof): [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md).

CI never sets these. `estate probes --live` (or `CELL_LIVE_PROBE=1`) SKIPs when the endpoint is unset and exits 0.

| Variable | Use |
| --- | --- |
| `CELL_LIVE_PROBE` | `1` / `true` / `yes` to opt into live HTTP |
| `CELL_LOCAL_ENDPOINT` | `ollama`, llama.cpp, or http-remote (`GET /v1/models` or `/api/tags`) |
| `CELL_RENTED_ENDPOINT` | Rented or any box. Probe ids stay free of hardware SKUs. |
| `CELL_MLX_ENDPOINT` | Apple MLX; falls back to `CELL_LOCAL_ENDPOINT` |
| `CELL_VLLM_ENDPOINT` | Experimental; unset = SKIP |
| `CELL_TRT_ENDPOINT` | Experimental; unset = SKIP |
| `CELL_FRONTIER_ENDPOINT` | Optional base for `--driver frontier`. Default `https://api.x.ai/v1`. |
| `CELL_FRONTIER_MODEL` | Optional frontier model id. Default `grok-4.7`. SKU ids refuse. |
| `XAI_API_KEY` / `XAI_API_BASE` / `XAI_MODEL` | Frontier A7 and `--driver frontier`. Key required. Model default `grok-4.7`. Not used by probes or Ollama specialist. |

## Day-30 gate demo (A1–A4)

```bash
make gate
```

| Gate | What you should see |
| --- | --- |
| **A1** | `estate validate` lists horizon, research, sanctum on separate lanes. Invalid fixtures exit 1. |
| **A2** | `estate apply` binds three profile-dir sessions. |
| **A3** | Horizon → `lane:research` denied. Cyera CI and Rust classroom cannot be read. |
| **A4** | Undeclared `shell` / `secrets` denied. Research `notes-append` and mount `notes` are declared and still denied: no allow intention covers them (deny-default). |

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
Disposable: `.cell/runtime/`, `.cell/sessions/`, PIDs. Regenerable: `.cell/actual-state.json`, `.cell/desired-snapshot.yaml`, `.cell/model-actual.json`, `.cell/catalog.json`, `.cell/reconcile.json`, `.cell/reconcile.md`. Patch file only: `.cell/reconcile-suggest.md`. Local review stays in `.cell/audit-export/` on this box. Cell archives live under `backups/`. See [`docs/cell-layout.md`](docs/cell-layout.md).

## Stubbed and wired

| Piece | State |
| --- | --- |
| Isolation | Profile dirs. Trait is swappable. |
| Conveyor HTTP | `POST /v0/check` only. `estate convey` is a lease-bound hop stub. |
| Frontier `xai_grok` | Wired driver. Live when `XAI_API_KEY` is set. Tests use mock/HTTP fake. |
| Local `local_slm` | Wired `ollama` driver + `CELL_LOCAL_ENDPOINT`. `mock-local` speaks the protocol. |
| llama.cpp | Swap-proof card; same specialist protocol. |
| Enrich packs | Curator jason, policy manual. Curator edit instructions. Jason pastes them into the estate. |
| Feed | Scrubbed jsonl, both paths. Candidate packs. Promote stays refused. |
| A10–A12 | Beachhead on `main`: feed packs + import, suspend/resume + placement leases, gated/auditable apply. |
| MLX / vLLM / TRT / cloud spawn / convey transport | Parked. See [Parked](#parked). |

## Sharp choices (Jev bait)

Day 0–30 locks kept: multi-crate, Rust validate SoT, own-lane free, vendor-out-of-floor, hash, tiny_http, thin apply/drift (now with snapshot + session-dir drift), Rust-default-not-law, dual-layer sacred, tool slug taxonomy deferred.

Day 60 additions:

1. **`models:` on agents** is a deny-default allow-list, same shape as tools.
2. **Mixed path is data-plane only.** Control does not complete when bindings are wired.
3. **A8 is mandatory in-path** when a local binding is wired. Frontier-proxy-only stays refused.
4. **Local protocol is HTTP JSON**, language-free and host-class-free. `mock-local` stands in for any host class.
5. **Feed events omit prompts and keys**; they record kind/decision/byte counts only.
6. **Live A7 with local down** audits `model.local.down` and stops.
7. **Hardware SKUs stay out** of binding ids and drivers. Supported runtimes: `ollama` and llama.cpp. MLX, vLLM, and TRT stay parked catalog cards.
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
