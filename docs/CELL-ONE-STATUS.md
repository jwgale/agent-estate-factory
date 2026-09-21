# Cell One status (for Jason)

Snapshot of **what is on `main` after this merge**. Not a live-box report.
Not a release. Workspace crates are `0.1.0` (crate version, not crates.io).

Read with [`../README.md`](../README.md) -> [`OPERATOR-DAY.md`](OPERATOR-DAY.md)
-> [`GATE-90.md`](GATE-90.md). Parked boxes: [`DAY90-PLUS.md`](DAY90-PLUS.md).

## On `main` (PR #1-#17 plus this slice)

Day 0-90 factory is merged. Horizon / Research / Sanctum on separate lanes.
Sacred dual-layer KEEP: Cyera CI and Rust classroom stay out of the estate.
Sanctum is first-class and is not Cyera.

Operator entrypoint is local `make gate-90`. Hosted Actions stays one
`pull_request` job: `cargo check --workspace --locked`. Do not put
`cargo test` or `make gate-90` on Actions.

## SKU launder story (#15–#17)

Hardware SKUs (`rtx-5090`, `4090`) are not host classes. The locked names
are `consumer-nvidia` | `apple-silicon` | `rented-nvidia` | `any`. For a
while the factory could take a tampered SKU on disk and quietly turn it
into portable `any`. That is a launder: a 5090-shaped row looks like it
can hop anywhere.

What happened, in order:

1. **#15.** `convey sync` used a slim parser that called
   `canonical_host_class`. Unknown / SKU `host_class` became `any`, and
   that `any` could seed a hop. Slim-parse is now `refuse:bad-host-class`
   and writes no mesh.
2. **#16.** `convey call` swallowed that slim-parse with `if let Ok`, so
   a SKU skipped the not-live check. Floor claim/record could stamp
   `any`. Status / leases / reconcile stayed silent. Call, status,
   leases, reconcile, and record now refuse. Claim keeps the raw SKU
   (does not rewrite it).
3. **#17.** Tampered `conveyor-mesh.json` and a backup that already
   contained a SKU still got through. Call / list / expire / sync /
   declare / forget refuse and write nothing. Restore of a SKU
   placement-actual / mesh / hops / leases is the same refuse. No
   `any`-invent. Production callers of `canonical_host_class` are gone;
   untrusted disk uses `canonical_host_class_opt` and refuses.

Empty / unset `host_class` is still portable `any`. A SKU or an unknown
string is not.

## As of #14 (clean hunts)

#14 locked plan exits, dry-run refuse writes, and curator clap vs
`refuse:curator`. No new product bug on those three paths.

## Bug fix this slice

| What was broken | What it does now |
| --- | --- |
| `estate leases` printed the SKU `placement-actual` JSON, then refused. The operator still saw the bad actual. | Refuse first. No JSON dump on `refuse:bad-host-class`. Status already refused before print; the test now locks that. |
| `backup` / `restore` used `load_estate(...).ok()`. A file that existed but did not parse was treated as "no estate", so restore could use a locked-only sacred set. | If the estate path is a file, parse it or refuse. Missing file stays optional. |
| `suspend` swallowed `actual-state.json` parse errors and journal write failures (`if let Ok` / `let _ =`), so unspawn lines could vanish from `sessions.jsonl`. | Load and journal `?`. Corrupt actual-state fails closed. |
| Feed import wrote `{id}.redaction.json` with `let _ =` and never re-checked the report bytes. | Report write fails closed. Serialized report is kind counts only; a raw secret in the report is `refuse:raw-secret`. Hand-written dirty packs write no accepted pack and no report. |
| #17 floor `src` tests used `rtx-5090`, so `doctor --strict` failed the vendor-needle scan. | Floor fixtures use `not-a-host`. Estate-control e2e still uses `rtx-5090`. |

#17 left `rtx-5090` in floor `src` tests, so `doctor --strict` failed
(floor src must not contain vendor / SKU needles). Those fixtures now
use `not-a-host`. Estate-control e2e still uses `rtx-5090`.

Sacred, curator, and policy refuse paths had no new silent `Ok()` swallows.

`lifecycle.jsonl` / `sessions.jsonl` stay append-only under suspend /
resume / `expire --forget`. Forget rewrites leases, not the journals.

## Bug fixes on #10-#17 (plain English)

| PR | What was broken | What it does now |
| --- | --- |
| #10 | After a placement lease expired, `expire --forget` dropped the row. The next `apply` treated that as drift and demanded `--force`. | Same desired estate restamps the lease (`lease-refresh`). No `--force`. Isolated TTL e2e on `ttl-short.yaml`. |
| #11 | Overlay e2e was missing. Easy to believe `locked: []` in a sacred file would drop Cyera CI / Rust classroom. | Isolated sacred overlay e2e: omit-locked file still refuses those two on convey. `lab-notebook` refuses only with the overlay installed. |
| #12 | `convey expire --forget` deleted hop *declarations* with the leases. The next `call` said `refuse:no-lease`. | Forget keeps the hop decl. `call` restamps (`lease-refresh`), same idea as #10. Expired still refuses until forget. |
| #13 | `restore` treated empty/missing backup `sacred_ids` as "matches anything" and could write files. | Empty set is `refuse:sacred-mismatch`. Dry-run and live restore both write nothing. |
| #15 | `convey sync` slim-parsed a SKU `host_class` as portable `any` and could seed a hop. | Slim-parse is `refuse:bad-host-class`. No mesh write. |
| #16 | `convey call` swallowed slim-parse, so a SKU skipped not-live. Floor claim/record could stamp `any`. Status/leases/reconcile stayed silent. | Call, status, leases, reconcile, record refuse. Claim keeps the raw SKU. |
| #17 | Mesh readers and restore still copied a SKU `host_class`. `canonical_host_class` could still invent `any` on a production path. | Mesh + restore refuse. Production callers of the unwrap are gone. |

## Known-good local commands

Hosted CI is compile-only. These stay on the box:

```bash
make gate-90          # smoke + day90 + doctor --strict + checklist (local)
make smoke            # doctor + fixtures-check + operator-day + cargo test + day90
make day90            # isolated operator loop
make feed-loop        # scrubbed trace -> pack -> propose -> accept (not in smoke)
make fixtures-check   # fixture files only
make doctor-strict    # pre-merge extras
make check            # cargo check --workspace --locked (same as Actions)
estate help           # Day-90 topics
```

`make gate-90` does not invoke `gh` or GitHub Actions.

Isolated loops without live boxes:

- Dual-layer-demo: `crates/estate-control/tests/day90_e2e.rs`
- Two dry-runs + backup/restore: `tests/day90_props.rs`
- Short TTL: `examples/fixtures/ttl-short.yaml` + `tests/day90_ttl.rs`
- Sacred overlay: `sacred-omit-locked.yaml` + `tests/day90_sacred.rs`
- Hop TTL: `tests/day90_hop.rs`
- Contracts: `tests/day90_contracts.rs` / `tests/day90_plan.rs`
- Bad-host-class readers: `tests/day90_sku.rs`
- Mesh / restore SKU: `tests/day90_mesh.rs`
- Swallows / journals / redaction: `tests/day90_honesty.rs`
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

## Rails that still hold

GitHub is the only source of truth. No Origin. No auto-promote. Cloud
never spawned. Pause-safe disk. Not Dual PE, not a studio, not an
AI-gateway product. `examples/estate.yaml` hash stays
`sha256:dcd7164f04c83f514185e77d2d4f6c23cae6dbb27a9b5da96a28ba1f3c724930`.
