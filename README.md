# Cell One — Agent Estate Factory (Day 0–30)

One-box factory skeleton. Proves **A1–A4** on a synthetic Horizon / Research / Sanctum estate. Pause-safe. No live Grok, no 5090, no Dual PE, no multi-box control, no AI-gateway product.

Source of truth: https://github.com/jwgale/agent-estate-factory

Locked defaults live in [`charter.md`](charter.md). Documentary schema: [`schema/estate.v0.schema.json`](schema/estate.v0.schema.json). Fail-closed SoT: the Rust validator.

## Day-30 gate demo

From the repo root (Rust 1.85+, pinned in `rust-toolchain.toml`):

```bash
cargo test --workspace
make gate
```

`make gate` runs validate, plan, supervisor apply, conveyor deny/allow checks, sacred-exclusion denies, pause stop/start, and `cargo test`. It writes `gate-reports/latest.md`. Nothing here calls a model provider.

| Gate | What you should see |
| --- | --- |
| **A1** | `estate validate` lists horizon, research, sanctum on separate lanes. Invalid fixtures exit 1. |
| **A2** | `estate apply` / `floor-supervisor apply` binds three profile-dir sessions. |
| **A3** | Horizon → `lane:research` denied. Cyera CI and Rust classroom are not agents and cannot be read. |
| **A4** | Undeclared `shell` / `secrets` denied. Research `notes-append` and mount `notes` allowed. |

Manual pieces:

```bash
# A1
cargo run -p estate-control -- validate --estate examples/estate.yaml

# blast-radius plan (append-only plans/)
cargo run -p estate-control -- plan --estate examples/estate.yaml

# A2
cargo run -p floor-supervisor -- apply --estate examples/estate.yaml --state-dir .cell

# A3 deny
cargo run -p conveyor-proxy -- check --estate examples/estate.yaml \
  --agent horizon --kind memory_read --object lane:research

# A3 sacred
cargo run -p conveyor-proxy -- check --estate examples/estate.yaml \
  --agent horizon --kind memory_read --object cyera-ci

# A4 deny / allow
cargo run -p conveyor-proxy -- check --estate examples/estate.yaml \
  --agent horizon --kind tool --object shell
cargo run -p conveyor-proxy -- check --estate examples/estate.yaml \
  --agent research --kind tool --object notes-append

# placeholders only
cargo run -p estate-control -- models --estate examples/estate.yaml

# optional HTTP stub (workers POST /v0/check) — port 47821
cargo run -p conveyor-proxy -- serve --estate examples/estate.yaml --bind 127.0.0.1:47821

# pause
make pause-stop
make pause-start
```

Invalid estates used by tests live in `examples/invalid/`. They must fail closed.

## Layout

| Crate | Plane | Role |
| --- | --- | --- |
| `estate-schema` | shared | types, validate, hash, compiled intentions, plan, firewall |
| `estate-control` | control | `estate` CLI: validate, plan, apply, drift, models |
| `isolation-driver` | data | `IsolationDriver` trait + profile-dir + in-memory |
| `floor-supervisor` | data | bind sessions; regenerable actual-state; stop runtime |
| `conveyor-proxy` | data | deny-default tool/mcp/mount/memory; worker client |
| `model-estate` | data | equal-class frontier/local bindings; unwired stubs |
| `feed-collector` | feed | append-only scrubbed jsonl; no auto-promote |

Workers are expected to call `conveyor-proxy` (`WorkerClient` or `POST /v0/check`). Floor core has no vendor ids.

## Persist vs disposable

Survives pause: charter, estate file, schema, `lanes/`, `plans/`, `gate-reports/`.  
Disposable: `.cell/runtime/`, `.cell/sessions/`, PIDs. See [`docs/pause-kit.md`](docs/pause-kit.md).

## What is still stubbed

- Frontier (`xai_grok`) and local (`gpu_5090`) are schema-equal bindings with `wired: false`. `complete()` / `estate models` refuse.
- Isolation is profile directories, not OS containers or VMs.
- Conveyor HTTP is a tiny check endpoint, not a mesh or gateway.
- Feed collector appends jsonl and never feeds control.
- Apply/drift is a thin stretch: sessions + `actual-state.json`, not a multi-box controller.
- No live workers, no cloud agents, no A7–A12.

## Sharp choices (Jev bait)

1. **Workspace of small crates** matching the spine modules, not a single binary blob.
2. **JSON Schema is documentary**; Rust `validate()` is fail-closed SoT.
3. **Sacred exclusions are declared on the estate and hardcoded** (`cyera-ci`, `rust-classroom` + aliases). An allow intention cannot punch through.
4. **Own-lane memory read is allowed** without an intention; cross-lane is not. That is the A3 line.
5. **Vendor hints live only in estate `params`**. Floor-supervisor sources are tested to reject `xai` / `grok` / `5090` / `cyera` strings.
6. **Canonical JSON SHA-256** of the desired estate is the plan/apply hash.
7. **Apply/drift shipped thin** so pause/rebind has something to converge; skip-able later if Jev hates it.
8. **Tool ids are slugs** (`notes-append`), not dotted names — keeps the v0 slug lock simple.
9. **tiny_http proxy stub** instead of axum/gRPC. Escape hatch is the `WorkerClient` library.
10. **Not forever-Rust.** Drivers are traits; model processes are language-free later.

Anti-shrink list is in the charter. Do not turn this into forensics, an MCP catalog, Dual PE, or a studio.
