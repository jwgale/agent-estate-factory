# Cell One status (for Jason)

Snapshot of **what is on `main` after this merge**. Not a live-box report.
Not a release. Workspace crates are `0.1.0` (crate version, not crates.io).

Read with [`../README.md`](../README.md) → [`OPERATOR-DAY.md`](OPERATOR-DAY.md)
→ [`GATE-90.md`](GATE-90.md). Parked boxes: [`DAY90-PLUS.md`](DAY90-PLUS.md).

## On `main` (PR #1–#12 plus this slice)

Day 0–90 factory is merged. Horizon / Research / Sanctum on separate lanes.
Sacred dual-layer KEEP: Cyera CI and Rust classroom stay out of the estate.
Sanctum is first-class and is not Cyera.

| PR | What landed |
| --- | --- |
| #7 | `estate help` topics; `backup --prune N`; convey policy deny |
| #8 | cell-layout sync; `OPERATOR-DAY.md`; gate-90 stays off Actions |
| #9 | Dual-layer-demo e2e (validate → prune); README ↔ operator/feed links; `CELL-ONE-STATUS.md` |
| #10 | Placement TTL e2e. `expire --forget` then apply restamps (was `refuse:drift`) |
| #11 | Sacred overlay e2e: omit-locked file still refuses Cyera CI / Rust classroom on convey |
| #12 | Hop expire `--forget` kept decls; call restamps (`lease-refresh`). Pause-kit still holds after unchanged apply |
| this | Restore empty `sacred_ids` now `refuse:sacred-mismatch`. Pack leftover / suggest-on-drift / doctor vanilla↔strict locked |

Operator entrypoint is local `make gate-90`. Hosted Actions stays one
`pull_request` job: `cargo check --workspace --locked`. Do not put
`cargo test` or `make gate-90` on Actions.

Isolated loops without live boxes:

- Dual-layer-demo: `crates/estate-control/tests/day90_e2e.rs`
- Short TTL: `examples/fixtures/ttl-short.yaml` + `tests/day90_ttl.rs`
- Sacred overlay: `sacred-omit-locked.yaml` + `tests/day90_sacred.rs`
- Hop TTL: `tests/day90_hop.rs` (declare → expire → forget → call restamp)
- Feed: [`FEED-LOOP.md`](FEED-LOOP.md)

Cloud-agent stays declared, not spawned. Feed never auto-promotes.
`estate.yaml` is never rewritten by rematerialize.

## Still parked (not green)

| Item | State |
| --- | --- |
| Live Mac MLX | Probe path only. No Mac attached. |
| Live consumer / rented GPU | Same specialist protocol. Not required for gates. |
| Cloud-agent spawn | Declared only. Floor does not spawn. |
| Auto-promote / curator UI | Locked off / not built. Jason pastes pack ids. |
| Convey hop transport | Lease-bound mesh, not a gateway. |
| vLLM / TRT | Catalog cards until you verify. |
| Actions expansion | Compile-only unless you expand it. |

## How to prove it here

```bash
make gate-90     # local only
make feed-loop   # fixtures; not in smoke
estate help      # Day-90 topics
```

`.cell/` paths: [`cell-layout.md`](cell-layout.md).

## Rails that still hold

GitHub is the only source of truth. No Origin. No auto-promote. Cloud
never spawned. Pause-safe disk. Not Dual PE, not a studio, not an
AI-gateway product. `examples/estate.yaml` hash stays
`sha256:dcd7164f04c83f514185e77d2d4f6c23cae6dbb27a9b5da96a28ba1f3c724930`.
