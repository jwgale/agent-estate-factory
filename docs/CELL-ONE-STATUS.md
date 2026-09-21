# Cell One status (for Jason)

Snapshot of **what is on `main` after this merge**. Not a live-box report.
Not a release. Workspace crates are `0.1.0` (crate version, not crates.io).

Read with [`../README.md`](../README.md) → [`OPERATOR-DAY.md`](OPERATOR-DAY.md)
→ [`GATE-90.md`](GATE-90.md). Parked boxes: [`DAY90-PLUS.md`](DAY90-PLUS.md).

## On `main` (PR #1–#13 plus this slice)

Day 0–90 factory is merged. Horizon / Research / Sanctum on separate lanes.
Sacred dual-layer KEEP: Cyera CI and Rust classroom stay out of the estate.
Sanctum is first-class and is not Cyera.

Operator entrypoint is local `make gate-90`. Hosted Actions stays one
`pull_request` job: `cargo check --workspace --locked`. Do not put
`cargo test` or `make gate-90` on Actions.

## Bug fixes on #10–#13 (plain English)

| PR | What was broken | What it does now |
| --- | --- | --- |
| #10 | After a placement lease expired, `expire --forget` dropped the row. The next `apply` treated that as drift and demanded `--force`. | Same desired estate restamps the lease (`lease-refresh`). No `--force`. Isolated TTL e2e on `ttl-short.yaml`. |
| #11 | Overlay e2e was missing. Easy to believe `locked: []` in a sacred file would drop Cyera CI / Rust classroom. | Isolated sacred overlay e2e: omit-locked file still refuses those two on convey. `lab-notebook` refuses only with the overlay installed. |
| #12 | `convey expire --forget` deleted hop *declarations* with the leases. The next `call` said `refuse:no-lease`. | Forget keeps the hop decl. `call` restamps (`lease-refresh`), same idea as #10. Expired still refuses until forget. |
| #13 | `restore` treated empty/missing backup `sacred_ids` as “matches anything” and could write files. | Empty set is `refuse:sacred-mismatch`. Dry-run and live restore both write nothing. |

This slice found no new exit/write/curator bugs. Locked: `plan diff --allow-wider` exits 0 only when you passed the flag (or the blast did not grow); `plan export-pr` exits 0 with risks listed and does not write `.cell`; `apply --dry-run` under refuse (`--require-plan`, policy-deny, expired) writes nothing; wrong `--curator` is `refuse:curator` on feed import / packs import / packs accept / `apply --import-pack`. Missing `--curator` on accept is clap (required). Import still defaults to `jason`.

## Earlier landings (#7–#9)

| PR | What landed |
| --- | --- |
| #7 | `estate help` topics; `backup --prune N`; convey policy deny |
| #8 | cell-layout sync; `OPERATOR-DAY.md`; gate-90 stays off Actions |
| #9 | Dual-layer-demo e2e (validate → prune); README ↔ operator/feed links; this snapshot file |

Isolated loops without live boxes:

- Dual-layer-demo: `crates/estate-control/tests/day90_e2e.rs`
- Short TTL: `examples/fixtures/ttl-short.yaml` + `tests/day90_ttl.rs`
- Sacred overlay: `sacred-omit-locked.yaml` + `tests/day90_sacred.rs`
- Hop TTL: `tests/day90_hop.rs` (declare → expire → forget → call restamp)
- Contracts: `tests/day90_contracts.rs` / `tests/day90_plan.rs`
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
