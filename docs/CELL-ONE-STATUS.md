# Cell One status (for Jason)

Snapshot of **what is on `main` after this merge**. Not a live-box report.
Not a release. Workspace crates are `0.1.0` (crate version, not crates.io).

Read with [`../README.md`](../README.md) -> [`OPERATOR-DAY.md`](OPERATOR-DAY.md)
-> [`GATE-90.md`](GATE-90.md). Parked boxes: [`DAY90-PLUS.md`](DAY90-PLUS.md).

## On `main` (PR #1-#19 plus this slice)

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

## #18 (fail-closed swallows)

#18 closed the first honesty hunt after the SKU series. `estate leases`
printed a SKU actual, then refused. Backup / restore treated a present
but unreadable estate file as "no estate". `suspend` swallowed a corrupt
`actual-state.json` and journal write failures. Feed import wrote the
redaction report with `let _ =`. Floor src tests still said `rtx-5090`,
so `doctor --strict` failed. Those are closed. Journals stay append-only.

## #19 (lifecycle / audit / curator-dry-run / catalog / probes)

#19 closed the next honesty hunt. `suspend` / `resume` / `apply` treated
a present but unreadable `lifecycle.json` as greenfield. Feed import
wrote the accepted pack, then swallowed `append_import_audit`.
`apply --dry-run --import-pack` skipped curator. Catalog printed, then
failed the write. Probes printed while still checking SKU / host_class.
Status invented `expired=0` / empty proposals on reader failure. Those
are closed.

## Bug fix this slice

| What was broken | What it does now |
| --- | --- |
| `covering_plan` / `latest_plan` / `list_plans` treated unreadable plan JSON as empty (skip / `.ok()`). `apply --require-plan` could say "no plan". `status` could invent last-plan. | Present plan JSON parses or refuses. APIs return `Result`. Status / apply / last-applied fail closed. |
| `write_cursor` used `unwrap_or_default` and could write empty `feed-cursor.json`. A present garbage cursor could look missing. | Load: missing → none; exists but not a file or parse fail → refuse. Write serializes or refuses. No empty blob. |
| `apply` without `--force` could still walk a SKU `placement-actual`. `--force` must overwrite from the estate, not launder the SKU to `any`. | Without `--force`, `refuse_lease_host_classes` before dry-run. `--force` claims estate `host_class` (`rtx_consumer` → `consumer-nvidia`). |

Remaining `.ok()` on Path-exists loads in estate-control / floor / conveyor / feed are listing skips (`read_dir` / `filter_map`), not sacred / SKU / plan SoT. `load_placements` stays permissive so `--force` can overwrite. Regenerable INDEX writes stay best-effort.

## Bug fixes on #10-#19 (plain English)

| PR | What was broken | What it does now |
| --- | --- | --- |
| #10 | After a placement lease expired, `expire --forget` dropped the row. The next `apply` treated that as drift and demanded `--force`. | Same desired estate restamps the lease (`lease-refresh`). No `--force`. Isolated TTL e2e on `ttl-short.yaml`. |
| #11 | Overlay e2e was missing. Easy to believe `locked: []` in a sacred file would drop Cyera CI / Rust classroom. | Isolated sacred overlay e2e: omit-locked file still refuses those two on convey. `lab-notebook` refuses only with the overlay installed. |
| #12 | `convey expire --forget` deleted hop *declarations* with the leases. The next `call` said `refuse:no-lease`. | Forget keeps the hop decl. `call` restamps (`lease-refresh`), same idea as #10. Expired still refuses until forget. |
| #13 | `restore` treated empty/missing backup `sacred_ids` as "matches anything" and could write files. | Empty set is `refuse:sacred-mismatch`. Dry-run and live restore both write nothing. |
| #15 | `convey sync` slim-parsed a SKU `host_class` as portable `any` and could seed a hop. | Slim-parse is `refuse:bad-host-class`. No mesh write. |
| #16 | `convey call` swallowed slim-parse, so a SKU skipped not-live. Floor claim/record could stamp `any`. Status/leases/reconcile stayed silent. | Call, status, leases, reconcile, record refuse. Claim keeps the raw SKU. |
| #17 | Mesh readers and restore still copied a SKU `host_class`. `canonical_host_class` could still invent `any` on a production path. | Mesh + restore refuse. Production callers of the unwrap are gone. |
| #18 | Leases printed then refused. Backup/restore swallowed a garbage estate. Suspend swallowed actual-state / journal errors. Feed redaction write was `let _ =`. | Refuse first. Present file must parse. Journal and redaction writes fail closed. |
| #19 | Lifecycle `.ok()` treated garbage as greenfield. Import audit was `let _ =`. Dry-run skipped curator. Catalog/probes printed first. Status invented expired/proposals. | Parse or refuse. Audit fail-closed. Curator before dry-run. Write/refuse first, then print. |

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
- Swallows / journals / redaction / plan / cursor / force-SKU: `tests/day90_honesty.rs`
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
