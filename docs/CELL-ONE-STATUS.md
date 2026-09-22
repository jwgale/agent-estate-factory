# Cell One status (for Jason)

Snapshot of **what is on `main` after this merge**. Not a live-box report.
Not a release. Workspace crates are `0.1.0` (crate version, not crates.io).

Read with [`../README.md`](../README.md) -> [`OPERATOR-DAY.md`](OPERATOR-DAY.md)
-> [`GATE-90.md`](GATE-90.md). Parked boxes: [`DAY90-PLUS.md`](DAY90-PLUS.md).
Live probe hand-off: [`LIVE-PROBES.md`](LIVE-PROBES.md).

## On `main` (PR #1-#49 plus this slice)

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

## Frontier grok-4.7 (#29–#31)

#29 pinned the frontier model to `grok-4.7` and required `XAI_API_KEY` for `estate specialist --driver frontier`. `--driver http-remote` stayed the local card. `READY_FOR_LIVE_TEST` was yes for that hand-off; the key was never printed.

#30 recorded the live PASS (`completion` `pong`, reason `frontier completion`) and set `READY_FOR_LIVE_TEST` back to no. The mixed fixture validates and dry-runs with no POST. Local `--driver ollama` down or unset does not call frontier.

#31 plans and applies that fixture (`--require-plan`) on a mock cell. The catalog sibling card names `grok-4.7` with streaming/tools/vision false and completion budget 64. `ollama`, `http-remote`, `llama.cpp`, `mlx`, `vllm`, and `trt` do not POST frontier. `reasoning_effort` xhigh stays docs-only.

## #32–#34 in plain English

#32. `make day90-mixed` walks the mixed fixture on a throwaway cell: status, plan, apply with a plan, status, doctor. No live key. It is not part of `make smoke` or Actions. Status and doctor print `grok-4.7` only when a catalog file or an estate binding actually has that model. The default estate binding does not, so status does not invent one.

#33. A feed pack now says whether its events were frontier, local, or both (`source_drivers`). The tag has to match the counts. An event marked local stays local even if the event name mentions frontier. A proposal shows the same tag. Promote stays off.

#34. `make feed-loop` checks that tag on the pack it just wrote, and checks the proposal copies it. Live keys are unset. `estate help frontier` and `estate help day90-mixed` explain those paths. If writing a feed event fails, the task stops before it calls frontier.

## #35–#37 in plain English

#35. `estate specialist --driver frontier` already refused without `XAI_API_KEY`. Stderr names `CELL_FRONTIER_MODEL` and `CELL_FRONTIER_ENDPOINT` for a missing key and for a hardware SKU in the model id. The SKU still refuses before any POST. `docs/GATE-90.md` and `docs/DAY90-PLUS.md` separate green factory checks, recorded live proofs, and parked rows. Mac specialist complete is optional and not recorded. Native MLX stays a stub. Cloud-agent spawn stays off.

#36. Pack `INDEX.md` and `estate feed list` / `estate packs list` print `drivers=frontier,local` when the pack has both classes, and `drivers=-` when it has none. An empty pack does not invent `frontier`. Writing a drop pack, importing one, and listing the drop return an error if the INDEX rewrite fails. `make feed-loop` greps that line. There is no `make feed-loop-mixed`.

#37. `packs accept` copies `source_drivers` into the enrich-edit instructions. An empty list stays `source_drivers: -`. A tag that does not match `path_counts` refuses before that file is rewritten. A failed proposal INDEX rewrite is an error. `source_drivers` stays an additive v0 field. Only `frontier` and `local`. A missing field defaults to `[]`.

## #38–#39 in plain English

#38. `make feed-loop` checks that the pack, the proposal, and `enrich-edit.json` carry the same `source_drivers`. A pack or proposal that omitted the tag while the frontier or local count was nonzero is no longer indexed as `drivers=-`. The index rewrite refuses and leaves the previous INDEX. Accept serializes the enrich-edit JSON before it writes either file.

#39. Frontier specialist sacred refuse matches local. A prompt that mentions Cyera or Rust classroom refuses before any POST, and the output does not invent `"completion": "ok"`. The README start-here and [`OPERATOR-DAY.md`](OPERATOR-DAY.md) point at `estate help frontier` and `make day90-mixed`. Those stay off smoke and `make gate-90`.

## #40 in plain English

`examples/hosts/frontier-http.yaml` names `model: grok-4.7` on a frontier `http-remote` binding, next to a local `ollama` card. It is not a host-class alias. `examples/estate.yaml` stays hash-locked. A frontier specialist prompt that mentions Cyera or Rust classroom still refuses as sacred when `CELL_FRONTIER_MODEL` is a hardware SKU.

## #41 in plain English

`make day90-mixed` validates `examples/hosts/frontier-http.yaml` and prints status (`frontier: frontier_http model=grok-4.7`). It does not apply that file or invent a cell catalog. `estate models` prints `model=grok-4.7` or `model=-`. The host file stays off fixtures-check, smoke, and `make gate-90`.

## #42 in plain English

Apply and resume write the cell catalog frontier model from the estate binding. A binding with no `params.model` leaves that field empty. Status prints `catalog frontier: cell model=-`. Doctor does not report that cell as `grok-4.7`. The schema catalog card stays `grok-4.7`. Two different frontier models refuse instead of picking one. `estate catalog` still dumps the schema card.

## #43 in plain English

`estate plan` and `apply --dry-run` print `frontier plan: model=` from the binding. Unset `params.model` stays `model=-`. They do not copy the schema card and they do not read `CELL_FRONTIER_MODEL`. An estate with no frontier binding refuses (`refuse:frontier-invent`) before a plan file or a dry-run preview, and that refuse does not invent a frontier `source_driver` or `grok-4.7`.

## #44 in plain English

`packs propose` and `packs accept` refuse (`refuse:frontier-invent`) when `source_drivers` names `frontier` and the estate has no frontier binding. They do not write the proposal or the enrich-edit file. A local-only pack stays `local`. The schema card is not copied.

## #45 in plain English

`estate status` and `estate doctor` refuse (`refuse:frontier-model`) when the cell `catalog.json` frontier model disagrees with the binding. Empty and missing are the same (`model=-`). The schema card stays `grok-4.7` and is not treated as the binding. A matching cell catalog still prints `model=grok-4.7` or `model=-`.

## #46 in plain English

`estate reconcile` and `estate resume` refuse (`refuse:frontier-invent`) when the estate has no frontier binding. They do that before `reconcile.json`, a suggest patch, or a resume catalog write. The schema card and `CELL_FRONTIER_MODEL` are not copied. An estate that already has a frontier binding still reconciles and resumes. Unset `params.model` stays empty.

## #47 in plain English

Live `estate apply` refuses (`refuse:frontier-model`) when the cell `catalog.json` frontier model disagrees with the binding. It does that before leases, an unchanged audit, or a catalog rewrite. A missing catalog is not a disagreement. A matching catalog still applies. `--force` does not overwrite the disagreement. The schema card is not the binding.

## #48 in plain English

`estate models` prints `model=-` when the binding sets no model, even if `CELL_FRONTIER_MODEL` is `grok-4.7`. `estate catalog` still dumps the schema card and labels it `(schema card, not a binding)`. It refuses (`refuse:frontier-model`) before overwriting a catalog whose frontier model is not that card. A missing file still receives the schema dump. An unset binding stays empty.

## #49 in plain English

`make day90-mixed` prints `frontier plan: model=grok-4.7 source_drivers=frontier,local` on the mixed fixture. Apply, status, and doctor keep that bound model. `CELL_FRONTIER_MODEL=grok-4.7` does not become the binding. The same walk uses a throwaway local-only estate. Plan and `apply --dry-run` are `refuse:frontier-invent`. Live apply writes no catalog. Status does not print a frontier binding. Doctor does not print a cell catalog model.

## This slice

`estate resume` and `estate pause-proof` refuse (`refuse:frontier-model`) when the cell `catalog.json` frontier model disagrees with the binding. They do that before resume writes leases or a catalog, and before pause-proof applies. A missing catalog is not a disagreement. A matching empty catalog still resumes, and the cell model stays empty. `pause-proof` does not invent a cell catalog. `CELL_FRONTIER_MODEL` is not the binding. No new CLI. No new smoke or gate-90 step.

`READY_FOR_LIVE_TEST`: **no**. No new live surface.

Remaining `unwrap_or_default` in estate-control / floor / conveyor / feed are file-name / host_class display / doctor reads, not serialize-then-write.

## Bug fixes on #10-#50 (plain English)

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
| #32 | Mixed plan/apply was only a crate test. Status and doctor never named the frontier model. | `make day90-mixed` walks the fixture on an isolated cell. Status/doctor print `grok-4.7` when the catalog or estate binding has it. Default estate does not invent a binding model. READY no. |
| #33 | Packs counted frontier and local events but did not name the source. A kind mentioning frontier could override `object_class: local`. | `source_drivers` is `frontier` and/or `local` and must match `path_counts`. Local class wins. Propose shows the tag. Promote stays off. READY no. |
| #34 | `make feed-loop` never checked the new tag. A failed feed append still let the task succeed. Help had no frontier or day90-mixed page. | The fixture walk asserts `source_drivers` frontier then local, with no live keys. Task feed failures refuse before frontier. `estate help frontier` and `estate help day90-mixed`. READY no. |
| #35 | A SKU frontier model id refused without saying which env set it. Gate pages called recorded GPU proof parked. | Frontier refuse stderr names `CELL_FRONTIER_MODEL` and `CELL_FRONTIER_ENDPOINT`. Green, recorded, and parked are separate. Mac specialist stays optional. Native MLX and cloud-spawn stay parked. READY no. |
| #36 | INDEX wrote `drivers=` but `estate feed list` hid them. A failed INDEX rewrite was swallowed. | List prints `drivers=frontier,local` when present and `drivers=-` when empty. Pack write, import, and list fail if the index rewrite fails. `make feed-loop` greps INDEX. No `make feed-loop-mixed`. No new CLI. READY no. |
| #37 | Propose copied `source_drivers` and accept dropped them. A failed proposal INDEX rewrite was swallowed. | Enrich-edit instructions keep the same list. Empty stays `-`. A tag that does not match `path_counts` refuses before the edit file is rewritten. Proposal index lists `drivers=` and fails closed. Schema freeze notes the additive v0 field. READY no. |
| #38 | The feed walk checked each file on its own. A missing `source_drivers` with a nonzero count was indexed as `drivers=-`. Accept wrote the markdown before the JSON existed. | Pack, proposal, and `enrich-edit.json` must carry the same tag. The index rewrite refuses that mismatch and leaves the old INDEX. Accept serializes both edit files before it writes either. READY no. |
| #39 | Frontier sacred refuse did not lock the same no-invented-completion check as local, and Rust classroom was not in that prompt test. The operator start page did not point at frontier help or `make day90-mixed`. | Both drivers refuse Cyera and Rust classroom before POST and do not invent a completion. README and OPERATOR-DAY point at the opt-in help and mixed walk. The mixed fixture already names `grok-4.7`. The hash-locked estate file is unchanged. READY no. |
| #40 | Host estates did not name `model: grok-4.7` on frontier `http-remote`. A sacred prompt plus a SKU `CELL_FRONTIER_MODEL` was not locked as sacred-first. | `examples/hosts/frontier-http.yaml` names that binding. The hash-locked estate file is unchanged. Sacred refuse still wins over the SKU model id, with no POST and no invented completion. READY no. |
| #41 | The host fixture was not on an opt-in walk. `estate models` hid `params.model`, so a catalog card could be read as the binding. | `make day90-mixed` validates the host file and status prints `frontier: frontier_http model=grok-4.7`. No apply, no cell catalog, hash-locked estate unchanged. `estate models` prints `model=grok-4.7` or `model=-`. Not in smoke or fixtures-check. READY no. |
| #42 | Apply copied the schema card `grok-4.7` into the cell catalog even when the frontier binding set no model. Status and doctor then printed that model. | The cell catalog frontier model is the binding's `params.model`, or empty. Status prints `model=-`. Doctor does not call that cell `grok-4.7`. The schema card is unchanged. Two different models refuse. READY no. |
| #43 | Plan and dry-run never said which frontier model was bound, so a local-only estate could be planned while the schema card still said `grok-4.7`. | Plan and dry-run print the binding model, or `model=-`. No frontier binding is `refuse:frontier-invent` before any plan file. The schema card and `CELL_FRONTIER_MODEL` are not copied. READY no. |
| #44 | Propose and accept copied a frontier `source_driver` onto an estate with no frontier binding. | That copy is `refuse:frontier-invent` before the proposal or enrich-edit file is written. A local-only pack stays `local`. READY no. |
| #45 | A cell catalog could name `grok-4.7` (the schema card) while the binding set no model, and status and doctor still looked successful. | That disagreement is `refuse:frontier-model` before the cell success line. Empty and missing stay `model=-`. The schema card stays labeled schema. READY no. |
| #46 | Reconcile and resume still ran on an estate with no frontier binding, and resume could write a cell catalog from the schema card. | That estate is `refuse:frontier-invent` before `reconcile.json`, a suggest patch, or a resume catalog write. A frontier binding with no model still resumes, and the cell model stays empty. READY no. |
| #47 | Live apply rewrote a cell catalog that disagreed with the binding, including a schema-card `grok-4.7` on an unset model. | That disagreement is `refuse:frontier-model` before leases, an unchanged audit, or a catalog rewrite. A missing catalog still applies. `--force` does not overwrite it. READY no. |
| #48 | `estate catalog` wrote the schema card `grok-4.7` over a cell catalog whose binding set no model. `estate models` could be read as that card when `CELL_FRONTIER_MODEL` was set. | Models print `model=-` for an unset binding. Catalog labels the schema card and refuses the overwrite. A missing file still gets the schema dump. READY no. |
| #49 | `make day90-mixed` printed the bound model on status and doctor but did not lock the plan line, and it never walked a local-only estate. | The mixed plan line is `frontier plan: model=grok-4.7 source_drivers=frontier,local`. A throwaway local-only estate is `refuse:frontier-invent`. Live apply writes no catalog. Status does not print a frontier binding. Doctor does not print a cell catalog model. READY no. |
| #50 | Resume rewrote a cell catalog that disagreed with the binding, and pause-proof kept going. A schema-card `grok-4.7` could sit on an unset model through the pause kit. | That disagreement is `refuse:frontier-model` before resume or pause-proof writes. A missing catalog is not a disagreement. Pause-proof does not invent a cell catalog. A matching empty catalog still resumes with an empty model. READY no. |

## Known-good local commands

Hosted CI is compile-only. These stay on the box:

```bash
make gate-90          # smoke + day90 + doctor --strict + checklist (local)
make smoke            # doctor + fixtures-check + operator-day + cargo test + day90
make day90            # isolated operator loop
make feed-loop        # scrubbed trace -> pack source_drivers -> propose -> accept (not in smoke)
make day90-mixed      # opt-in mixed fixture; not in smoke or gate-90
make fixtures-check   # fixture files only
make doctor-strict    # pre-merge extras
make check            # cargo check --workspace --locked (same as Actions)
estate help           # Day-90 topics, including frontier and day90-mixed
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
- Mixed estate dry-run + plan/apply + `make day90-mixed`: `scripts/day90-mixed.sh` + `crates/estate-control/tests/day90_mixed.rs`
- Apply records `local_slm` then specialist complete: `crates/estate-control/tests/day90_apply_specialist.rs`
- Feed: [`FEED-LOOP.md`](FEED-LOOP.md)

Cloud-agent stays declared, not spawned. Feed never auto-promotes.
`estate.yaml` is never rewritten by rematerialize.

## Still parked (not green)

| Item | State |
| --- | --- |
| Mac specialist | Optional. Mac `probes --live` is recorded. Mac `estate specialist` complete is not. Do not mark it green. |
| Native MLX | `specialist()` stays stub. Not the Ollama-on-Mac probe. |
| Live consumer / rented GPU | 5090 probes and specialist `Pong` are recorded. Not required to re-run for gates. Not native MLX. |
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
