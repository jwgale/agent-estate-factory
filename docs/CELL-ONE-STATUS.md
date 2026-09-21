# Cell One status (for Jason)

Snapshot of **what is on `main` after this merge**. Not a live-box report.
Not a release. Workspace crates are `0.1.0` (crate version, not crates.io).

Read with [`../README.md`](../README.md) -> [`OPERATOR-DAY.md`](OPERATOR-DAY.md)
-> [`GATE-90.md`](GATE-90.md). Parked boxes: [`DAY90-PLUS.md`](DAY90-PLUS.md).
Live probe hand-off: [`LIVE-PROBES.md`](LIVE-PROBES.md).

## On `main` (PR #1-#24 plus this slice)

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

## #20 (plan / cursor / force honesty)

#20 closed the plan / cursor / force hunt. Unreadable `plan-*.json` is refuse, not empty. `write_cursor` does not invent an empty `feed-cursor.json`. `apply --force` restamps estate `host_class` (`rtx_consumer` -> `consumer-nvidia`) and does not launder a SKU actual to `any`.

## #21 (serialize-then-write)

#21 closed the named invent-on-error holes after #20. `propose_enrich`,
`append_event` (`"{}"`), and `write_placements` serialize or refuse.
Same class on journals, mesh persist, and accept.

## #22 (runbook + dry shapes)

#22 shipped [`LIVE-PROBES.md`](LIVE-PROBES.md) and dry SKIP / would-live
fixtures. Live was still blocked: probes spoke factory `/v0/specialist`
only. `mock-local` was not a Mac/GPU proof.

## #23 (live adapter)

#23 shipped the OpenAI / Ollama HTTP adapter. Probes GET `/v1/models`
or `/api/tags`. Jason was pinged for live Ollama probes.
`READY_FOR_LIVE_TEST` for that surface: yes (already handed off).

## This slice

Operator path on `examples/fixtures/mixed-frontier-local.yaml`: `estate plan` then `estate apply --require-plan` writes `model-actual.json` (`frontier_http` / `http-remote`, `local_slm` / `ollama`), `placement-actual.json`, and `catalog.json`. No live key. Plan and apply do not POST. The next `estate specialist --driver ollama` completes on the local mock and still does not POST frontier.

Catalog file SoT lists frontier as a sibling card: model `grok-4.7`, streaming/tools/vision false, completion budget 64. Not a local probe and not a context window. `reasoning_effort` xhigh stays docs-only and is not sent.

Requested local specialist does not fall through: `ollama` up, `http-remote` up, `llama.cpp` down, and `mlx` / `vllm` / `trt` refuse. `XAI_API_KEY` and `CELL_FRONTIER_ENDPOINT` set still means no frontier POST.

`READY_FOR_LIVE_TEST`: **no**. No new live surface.

Remaining `unwrap_or_default` in estate-control / floor / conveyor / feed are file-name / host_class display / doctor reads, not serialize-then-write.

## Bug fixes on #10-#31 (plain English)

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
| #20 | Plan readers treated garbage JSON as empty. `write_cursor` could write empty. `apply --force` had to restamp estate class, not SKU to `any`. | Present plan JSON parses or refuses. Cursor write refuses empty. `--force` writes `consumer-nvidia`. |
| #21 | Propose / journal / placements serialize used `unwrap_or_default` or invented `"{}"`. | Serialize or refuse. No empty proposal, journal junk, or wiped actual. |
| #22 | Live probe runbook + dry SKIP/would-live fixtures. Still `/v0/specialist` only. | Hand-off page. Live still needed a real Ollama adapter. |
| #23 | Probes POSTed factory `/v0/specialist`. A running Ollama looked down. | GET `/v1/models` or `/api/tags`. `HttpLocal` adapter. Jason pinged for live Ollama. |
| #24 | Chat posted dummy ping. v0 200 garbage fell through. OpenAI choices without content counted as up. SKU model ids bound. | Request text round-trip. v0 / content / SKU refuse. `model-estate specialist` + llama.cpp OpenAI smoke. |
| #25 | Complete discarded model text. No `estate specialist`. Sacred text still POSTed. | `estate specialist --prompt` returns `completion`. Sacred refuse first. READY_FOR_LIVE_TEST yes. |
| #26 | 5090 OpenAI chat 200 with empty `message.content` hard-failed. | Fall through to `/api/chat`. Both-fail names status + model + pull. READY yes. |
| #27 | Live proof was chat-only. Mixed dry-run never hit HttpLocal. Apply swallowed catalog write. | Recorded Mac/5090 proof. `make live-specialist`. Mixed mock dry-run. Catalog write `?`. READY no. |
| #28 | No frontier specialist env. Apply and complete were separate. | `CELL_FRONTIER_ENDPOINT` mock path. Apply+specialist fixture. READY no. |
| #29 | Frontier default was `grok-3-mini`. Key did not unlock specialist. | Model `grok-4.7`. `XAI_API_KEY` required. READY yes (handed off). |
| #30 | Frontier live PASS was not on the hand-off page. | Recorded `pong` / `frontier completion`. READY no. Mixed `grok-4.7` dry-run. Local down does not POST frontier. |
| #31 | Mixed fixture stopped at dry-run. Catalog did not name `grok-4.7`. Stub local drivers were not locked off frontier. | `plan` + `apply --require-plan` on the mixed fixture (mock, no key, no POST). Frontier catalog card lists `grok-4.7` and completion caps. `ollama` / `http-remote` / `llama.cpp` / `mlx` / `vllm` / `trt` do not POST frontier. READY no. |

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
- Swallows / journals / redaction / plan / cursor / force-SKU / nonempty placement write: `tests/day90_honesty.rs`
- model-actual serialize + garbage refuse: `crates/model-estate` actual tests
- Live probe SKIP vs would-live (no network): `schema/live-probe-shapes.v0.json` + catalog unit test
- OpenAI / Ollama adapter ping + specialist (in-process mock): `crates/model-estate` adapter tests
- Specialist chat round-trip + llama.cpp OpenAI smoke: `crates/model-estate` adapter + `tests/specialist_cli.rs`
- Live specialist complete (`estate specialist`): `crates/estate-control/tests/specialist_cli.rs` + adapter complete tests
- Mixed estate dry-run + plan/apply + local specialist skips frontier: `crates/estate-control/tests/day90_mixed.rs`
- Apply records `local_slm` then specialist complete: `crates/estate-control/tests/day90_apply_specialist.rs`
- Feed: [`FEED-LOOP.md`](FEED-LOOP.md)

Cloud-agent stays declared, not spawned. Feed never auto-promotes.
`estate.yaml` is never rewritten by rematerialize.

## Still parked (not green)

| Item | State |
| --- | --- |
| Live Mac MLX | Native MLX `specialist()` stays stub. Mac proof is Ollama-on-Mac. Copy-paste: [`LIVE-PROBES.md`](LIVE-PROBES.md). No Mac attached here. |
| Live consumer / rented GPU | Same adapter (`/v1/models` or `/api/tags`). Not required for gates. |
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
