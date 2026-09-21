# Cell One status (for Jason)

Snapshot of **what is on `main` after this merge**. Not a live-box report.
Not a release. Workspace crates are `0.1.0` (crate version, not crates.io).

Read with [`../README.md`](../README.md) → [`OPERATOR-DAY.md`](OPERATOR-DAY.md)
→ [`GATE-90.md`](GATE-90.md). Parked boxes: [`DAY90-PLUS.md`](DAY90-PLUS.md).

## On `main`

Day 0–90 factory is merged (PR #1–#8 plus this slice). Horizon / Research /
Sanctum on separate lanes. Sacred dual-layer KEEP: Cyera CI and Rust
classroom stay out of the estate. Sanctum is first-class and is not Cyera.

Operator entrypoint is local `make gate-90` (smoke + `day90` +
`estate doctor --strict` + GATE-90 print). Hosted Actions stays one
`pull_request` job: `cargo check --workspace --locked`. Do not put
`cargo test` or `make gate-90` on Actions — gate-90 wraps the local suite.

This slice adds an isolated dual-layer-demo loop
(`crates/estate-control/tests/day90_e2e.rs`): validate → plan → dry-run →
apply → status → reconcile → backup → prune. Dry-run writes no leases.
Cloud-agent stays declared, not spawned. The demo estate file is not
rewritten. No live Grok / Mac / GPU.

Walk the same loops by hand: [`OPERATOR-DAY.md`](OPERATOR-DAY.md) and
[`FEED-LOOP.md`](FEED-LOOP.md). Feed never auto-promotes. `estate.yaml`
is never rewritten by rematerialize.

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
