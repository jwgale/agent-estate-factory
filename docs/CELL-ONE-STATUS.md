# Cell One status (for Jason)

Snapshot of **what is on `main` through PR #82**. Tip honesty: `convey leases`
refuses a spawned cloud hop before it prints JSON. Not a live-box report.
Not a release. Workspace crates are `0.1.0` (crate version, not crates.io).

Read with [`../README.md`](../README.md) -> [`NORTH-STAR.md`](NORTH-STAR.md)
-> [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md)
-> [`OPERATOR-DAY.md`](OPERATOR-DAY.md) -> [`GATE-90.md`](GATE-90.md).
Parked boxes: [`DAY90-PLUS.md`](DAY90-PLUS.md).
Live probe hand-off: [`LIVE-PROBES.md`](LIVE-PROBES.md).

## Vision reset

Day 90+ is real-world proof plus parked stubs, not more beachhead invent.
One page: [`NORTH-STAR.md`](NORTH-STAR.md). Opt-in ladder after
`make gate-90`: `make real-world` (SKIP without `CELL_LOCAL_ENDPOINT`;
not a PASS; not in smoke or Actions). Feed packs stay curator edit
instructions. Control does not complete. Mac specialist
complete is a recorded PASS (`Pong` on the MacBook Air, tip `2ab78a4`).

## Train/enrich prepare (this slice)

You can prepare an enrich job for a purpose-built SLM. `estate enrich prepare` reads a pack and writes files. `--all-drivers` writes every registered card into sibling directories, or writes none if one card refuses. Each directory has `prepare.json`, `PREPARE.md`, and `NEXT.md` (paths, the handoff command, fail-closed reminders). `estate enrich list` reads `.cell/enrich` and refuses when that directory is missing. After you create the local model outside the factory, `estate enrich import-prepared` checks the tag and the file and writes a `local_slm` binding proposal. `estate enrich apply-proposal` stages that binding. `estate plan` and `estate apply --require-plan` write the source estate. Apply without `--require-plan` leaves the source unchanged. The factory does not run the trainer. The `estate` binary does not call Ollama. `examples/estate.yaml` on `main` stays hash-locked and has no `params.model`, so prepare against it is `refuse:base-model`. `make enrich-prepare` checks the loop on a throwaway directory and expects `FROM llama3` on a copy that sets `params.model`, then stages the join and applies it with `--require-plan` on that copy. `estate enrich from-pack` is the same prepare for an accepted pack. `make enrich-live-prove` is opt-in: it runs `ollama create` when the seat is up, checks `ollama show`, and removes the tag. Promote stays off. A sacred line, a hardware SKU, a missing pack, the wrong curator, or a frontier tag with no frontier binding stops before those files exist. Neither walk is part of smoke or GitHub Actions. `READY_FOR_LIVE_TEST`: no. Page: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Live handoff: [`LIVE-PROBES.md`](LIVE-PROBES.md).

## On `main` (PR #1–#82)

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

#35. `estate specialist --driver frontier` already refused without `XAI_API_KEY`. Stderr names `CELL_FRONTIER_MODEL` and `CELL_FRONTIER_ENDPOINT` for a missing key and for a hardware SKU in the model id. The SKU still refuses before any POST. `docs/GATE-90.md` and `docs/DAY90-PLUS.md` separate green factory checks, recorded live proofs, and parked rows. At #35, Mac specialist complete was optional and not recorded. It is now a recorded PASS (`Pong` on the MacBook Air). Native MLX stays a stub. Cloud-agent spawn stays off.

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

## #50 in plain English

`estate resume` and `estate pause-proof` refuse (`refuse:frontier-model`) when the cell `catalog.json` frontier model disagrees with the binding. They do that before resume writes leases or a catalog, and before pause-proof applies. A missing catalog is not a disagreement. A matching empty catalog still resumes, and the cell model stays empty. `pause-proof` does not invent a cell catalog. `CELL_FRONTIER_MODEL` is not the binding.

## #51 in plain English

`estate feed import` and `estate packs import` refuse (`refuse:frontier-invent`) when the pack tags a frontier source driver and the estate has no frontier binding. They do that before an accepted pack, a redaction report, or an index rewrite. A missing frontier tag still imports. `CELL_FRONTIER_MODEL` is not a binding. The redaction report is kind counts: a present report must parse, and the command does not invent zero counts. A mixed-fixture import keeps `source_drivers` frontier and local. The report does not store a raw secret and does not invent `grok-4.7`.

## #52 in plain English

`estate backup` and `estate restore` refuse before they write. A cell `catalog.json` whose frontier model disagrees with the binding is `refuse:frontier-model`. A frontier `source_driver` on an estate with no frontier binding is `refuse:frontier-invent`. A desired snapshot whose sacred set disagrees with the estate is `refuse:sacred-mismatch`. A missing catalog is not a disagreement. A local-only pack still archives. `CELL_FRONTIER_MODEL` is not the binding, and the refuse does not invent `grok-4.7` unless the catalog file itself names that model. No new CLI. No new smoke or gate-90 step.

## #53 in plain English

`docs/GATE-90.md` stops calling the Ollama complete path ready. The mock completion stays green. The 5090 `Pong` stays recorded. At #53, Mac complete was still unrecorded. It is now a recorded PASS (`"completion": "Pong"` on the MacBook Air, tip `2ab78a4`). `mlx`, `vllm`, and `trt` refuse a frontier POST and are not live-ok. `docs/DAY90-PLUS.md` parks vLLM and TRT with the other stubs. `READY_FOR_LIVE_TEST` for the Mac command is no.

## #54 in plain English

`estate probes --live` does not print `live ok` for `mlx`, `vllm`, or `trt`. A stub or experimental card stays `live_probed=false` and says `not live-ok`, even when an HTTP endpoint answers. Ollama, llama.cpp, and http-remote still print `live ok` when their endpoint answers. No new CLI. No new smoke or gate-90 step.

## #55 in plain English

Mac `estate specialist` complete is a recorded **PASS** on the MacBook Air (`"completion": "Pong"`, reason `compat completion`, tip `2ab78a4`). [`LIVE-PROBES.md`](LIVE-PROBES.md) keeps the copy-paste: `PATH` includes `~/.cargo/bin`, `CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434`, `CELL_LOCAL_MODEL=llama3`, then `estate specialist --driver ollama --prompt "Reply with the single word pong."`. The 5090 `Pong` stays recorded. Native MLX stays a stub. `mlx`, `vllm`, and `trt` stay `not live-ok`. `READY_FOR_LIVE_TEST` for that Mac command is no.

## #56 in plain English

`estate status` and `estate doctor` refuse (`refuse:frontier-model`) when the cell `catalog.json` does not parse. They do that before the cell catalog success line. Apply, resume, and pause-proof already refused an unreadable catalog. A missing catalog is not a disagreement. The schema card stays the schema card. The refuse does not invent `grok-4.7`.

## #57 in plain English

[`LIVE-PROBES.md`](LIVE-PROBES.md) holds the Mac `estate specialist` result. The pasted completion is **PASS** (`"completion": "Pong"`, reason `compat completion`, tip `2ab78a4`). `READY_FOR_LIVE_TEST` for that command is no. The 5090 `Pong` stays recorded. Native MLX stays a stub. `mlx`, `vllm`, and `trt` stay `not live-ok`.

## #58 in plain English

`estate status` refuses (`refuse:proposal-unreadable`) when a `*.proposal.json` does not parse, or its id does not match the filename. It does that before the status page. A missing proposals directory is not a proposal. A parsed proposal still lists. The filename is not the proposal.

## #59 in plain English

`estate doctor` FAILs a present `conveyor-mesh.json` that does not parse, or whose hop `host_class` is not a class. It does that before `factory ready`. A missing mesh is not a failure: no expired hop leases. A note is not a pass.

## #60 in plain English

`estate doctor` FAILs a present `lifecycle.json` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent `suspended`. A file that parses prints `lifecycle.json state=` from the file. `suspended` is not a failure.

## #61 in plain English

`estate doctor` FAILs a present `apply-audit.jsonl` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a line count. A file that parses prints `apply-audit.jsonl lines=` from the file.

## #62 in plain English

`estate doctor` FAILs a present `lifecycle.jsonl` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a line count. A file that parses prints `lifecycle.jsonl lines=` from the file. `estate history` already refused that file.

## #63 in plain English

`estate doctor` FAILs a present `sessions.jsonl` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a line count. A file that parses prints `sessions.jsonl lines=` from the file. `estate sessions` already refused that file.

## #64 in plain English

`estate doctor` FAILs a present `actual-state.json` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a session count. A file that parses prints `actual-state.json sessions=` from the file. `estate status` already refused that file through drift.

## #65 in plain English

`estate doctor` FAILs a present `desired-snapshot.yaml` that does not parse when no cell catalog is present. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a frontier model from it. A file that parses prints `desired-snapshot.yaml name=` from the file. When a cell catalog is present, doctor already reads this snapshot. `estate apply` already refused an unreadable snapshot.

## #66 in plain English

`estate doctor` FAILs a present `model-actual.json` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a binding count. A file that parses prints `model-actual.json bindings=` from the file. `estate drift` already refused that file.

## #67 in plain English

`estate status` refuses (`refuse:model-actual`) when a present `model-actual.json` does not parse. It does that before the status page. A missing file is not a failure, and status does not invent a binding count. A file that parses is not a new status line. `estate drift` already refused that file. `estate doctor` already FAILs it before `factory ready`.

## #68 in plain English

`estate status` does not invent `suspended` when `lifecycle.json` is missing. It prints `paused: -` and `lifecycle: -`. A present file that does not parse is a refuse before the status page. A file that parses prints `paused:` and `lifecycle:` from the file. `estate doctor` already refused that file and does not invent `suspended` for a missing one.

## #69 in plain English

`floor status` (the `floor-supervisor` binary) does not invent `suspended` when `lifecycle.json` is missing. It prints `lifecycle: -`. A present file that does not parse is a refuse before that line. A file that parses prints `lifecycle:` and `durable=` from the file. `estate status` already prints `paused: -` for a missing file. The loader default stays for apply and resume.

## #70 in plain English

Apply, resume, and suspend do not journal `from: suspended` when `lifecycle.json` was missing. That `from` stays empty. A present file still supplies `from`. A file that does not parse is a refuse before the write. The loader default stays for a missing file that is not journaled as a prior state.

## #71 in plain English

`estate backup` and `estate restore` refuse before they write when `placement-actual.json` does not parse, or a cloud-agent lease in that file is spawned. A missing file is not a spawned lease. The archive meta does not record `cloud_agent_spawned: false` over that file.

## #72 in plain English

`estate status` refuses before it prints `cloud-agent: declared, not spawned` when a cloud-agent lease is spawned. An unspawned cell still prints that line. The lease file is not rewritten.

## #73 in plain English

`estate leases` and `floor leases` refuse before they print placement JSON when a cloud-agent lease is spawned. An unspawned file still prints that JSON. A missing file still says there is no placement-actual. The lease file is not rewritten.

## #74 in plain English

Apply and resume refuse before they write when a cloud-agent lease is spawned, or `placement-actual.json` does not parse. They do not restamp that lease to unspawned. `--force` does not. A missing file is not a spawned lease. An unspawned file still applies.

## #75 in plain English

`estate pause-proof` does not print the pause-proof JSON when the cell is drifted. That drift is `pause-proof: drift (fail closed)` with the drift notes. The clean note stays on a proof that is in sync. A spawned cloud lease and a lost lease count already refuse before that JSON.

## #76 in plain English

`estate suspend` and `floor suspend` refuse before they drop sessions or rewrite leases when a cloud-agent lease is spawned. They do not restamp that lease to unspawned. A missing placement file is not a spawned lease. A wired box lease still drops `spawned` on suspend.

## #77 in plain English

`estate expire` refuses before it lists leases or forgets them when an expired cloud-agent lease is spawned. `expire --forget` does not drop that row. A missing placement file is not a spawned lease. An expired box lease still drops when no spawned cloud row is in that drop.

## #78 in plain English

`estate convey sync` refuses before it writes a cloud hop lease when the placement lease is spawned. It does not record that hop as `spawned: false`. A missing placement file is not a spawned lease. An unspawned cloud placement still syncs. The placement file is not rewritten.

## #79 in plain English

`estate convey call` does not say a cloud hop is not spawned when the placement lease is spawned. A missing hop lease is not restamped to `spawned: false`. A missing placement file is not a spawned lease. An unspawned cloud hop still refuses as declared, not spawned. The placement file is not rewritten.

## #80 in plain English

`estate convey declare` refuses before it writes a cloud hop lease when that placement lease is spawned. It does not record `spawned: false`. A missing placement file is not a spawned lease. An unspawned cloud hop still declares as not spawned. The placement file is not rewritten.

## This slice

`estate convey expire` refuses before it lists or forgets an expired cloud hop lease that is spawned. `expire --forget` does not drop that row. A missing mesh is not a spawned lease. An expired box hop still drops when that cloud row is not in the drop.

`READY_FOR_LIVE_TEST`: no.

## Bug fixes on #10–#82 plus this slice (plain English)

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
| #51 | Import copied a frontier `source_driver` onto an estate with no frontier binding, and a bad redaction file could print counts of zero. | That copy is `refuse:frontier-invent` before the accepted pack or redaction report. A present report must parse. A local-only pack still imports. A mixed-fixture import keeps the tag. READY no. |
| #52 | Backup and restore copied a cell catalog that disagreed with the binding, and a frontier `source_driver`, onto an estate with no frontier binding. A desired snapshot could name sacred exclusions the estate does not. | Those are `refuse:frontier-model`, `refuse:frontier-invent`, and `refuse:sacred-mismatch` before the archive or the restore write. A local-only pack still archives. A missing catalog is not a disagreement. READY no. |
| #53 | The gate called Ollama specialist complete ready, and the parking lot did not name vLLM or TRT, so a stub card could be read as a live hand-off. | The completion row is mock-locked. 5090 `Pong` stays recorded. Mac complete is a recorded PASS (`Pong`, MacBook Air, tip `2ab78a4`). vLLM and TRT are parked and not live-ok. READY no. |
| #54 | `mlx` could print `live ok` by falling back to `CELL_LOCAL_ENDPOINT`. vLLM and TRT could do the same when their endpoint answered. | Stub and experimental probes stay `not live-ok` and do not open that ping. Supported cards still print `live ok`. READY no. |
| #55 | The live-probe page told the operator not to run Mac specialist complete, while that command was still unrecorded and both boxes were up. | The MacBook Air completion is a recorded PASS (`"completion": "Pong"`, reason `compat completion`, tip `2ab78a4`). READY no. The 5090 `Pong` stays recorded. |
| #56 | Status and doctor treated a cell `catalog.json` that does not parse as a note and still exited 0. Apply already refused that file. | Status and doctor are `refuse:frontier-model` before the cell success line. A missing catalog is not a disagreement. READY no. |
| #57 | The live-probe page had the Mac specialist command and no place to write the result, so a later edit could mark PASS before a completion was pasted. | The result row is **PASS** (`"completion": "Pong"`). READY no. The 5090 `Pong` stays recorded. |
| #58 | Status listed an unreadable `*.proposal.json` by filename and still printed the page. | That file is `refuse:proposal-unreadable` before the status page. A missing directory is not a proposal. A parsed proposal still lists. READY no. |
| #59 | Doctor printed a note for an unreadable conveyor mesh and could still say factory ready. A mesh with no placement file was not read. | That file is FAIL before factory ready. A missing mesh is not a failure. A hop host class that is not a class is the same FAIL. READY no. |
| #60 | Doctor treated a present `lifecycle.json` that does not parse as a layout note and could still say factory ready. A missing file could be read as `suspended`. | That file is FAIL before factory ready. A missing file is not a failure and is not invented as `suspended`. A parsed file prints its state. READY no. |
| #61 | Doctor treated a present `apply-audit.jsonl` that does not parse as a layout note and could still say factory ready. A missing file could be read as zero lines. | That file is FAIL before factory ready. A missing file is not a failure and is not invented as zero lines. A parsed file prints its line count. READY no. |
| #62 | Doctor treated a present `lifecycle.jsonl` that does not parse as a layout note and could still say factory ready. A missing file could be read as zero lines. | That file is FAIL before factory ready. A missing file is not a failure and is not invented as zero lines. A parsed file prints its line count. READY no. |
| #63 | Doctor treated a present `sessions.jsonl` that does not parse as a layout note and could still say factory ready. A missing file could be read as zero lines. | That file is FAIL before factory ready. A missing file is not a failure and is not invented as zero lines. A parsed file prints its line count. READY no. |
| #64 | Doctor treated a present `actual-state.json` that does not parse as a layout note and could still say factory ready. A missing file could be read as zero sessions. | That file is FAIL before factory ready. A missing file is not a failure and is not invented as zero sessions. A parsed file prints its session count. READY no. |
| #65 | Doctor treated a present `desired-snapshot.yaml` that does not parse as a layout note when no cell catalog was present, and could still say factory ready. A missing file could be read as a frontier model. | That file is FAIL before factory ready. A missing file is not a failure and is not invented as a frontier model. A parsed file prints its name. READY no. |
| #66 | Doctor ignored a present `model-actual.json` that does not parse and could still say factory ready. A missing file could be read as zero bindings. | That file is FAIL before factory ready. A missing file is not a failure and is not invented as zero bindings. A parsed file prints its binding count. READY no. |
| #67 | Status printed the page when a present `model-actual.json` did not parse. A missing file could be read as zero bindings. | That file is `refuse:model-actual` before the status page. A missing file is not a failure and is not invented as zero bindings. A parsed file is not a new status line. READY no. |
| #68 | Status printed `paused: yes` and `lifecycle: suspended (durable=true)` when `lifecycle.json` was missing. That is the default record, not a file. | A missing file prints `paused: -` and `lifecycle: -`. A present file that does not parse is a refuse before the status page. A parsed file prints its state. READY no. |
| #69 | `floor status` printed `lifecycle: suspended durable=true` when `lifecycle.json` was missing. That is the default record, not a file. | A missing file prints `lifecycle: -`. A present file that does not parse is a refuse before that line. A parsed file prints its state. READY no. |
| #70 | Apply, resume, and suspend journaled `from: suspended` when `lifecycle.json` was missing. That is the default record, not a prior state. | A missing file journals `from` empty. A present file supplies `from`. A file that does not parse is a refuse before the write. READY no. |
| #71 | Backup wrote `cloud_agent_spawned: false` without reading `placement-actual.json`. An unreadable file or a spawned cloud-agent lease was still archived. | That file is a refuse before the archive or the restore write. A missing file is not a spawned lease. READY no. |
| #72 | Status printed `cloud-agent: declared, not spawned` and then failed when a cloud-agent lease was spawned. | That lease is a refuse before the line. An unspawned cell still prints it. The lease file is not rewritten. READY no. |
| #73 | `estate leases` and `floor leases` printed placement JSON and then failed when a cloud-agent lease was spawned. | That lease is a refuse before the JSON. An unspawned file still prints. A missing file is unchanged. The lease file is not rewritten. READY no. |
| #74 | Apply and resume rewrote a spawned cloud-agent lease as unspawned and recorded that it was not spawned. `--force` did the same. An unreadable placement file could be replaced on `--force`. | That lease, and a file that does not parse, is a refuse before the write. A missing file is not a spawned lease. An unspawned file still applies. READY no. |
| #75 | Pause-proof printed the clean kit JSON, including "Cloud-agent not spawned", and then failed when the cell was drifted. | That drift is a refuse before the JSON. The error names the drift notes. An in-sync proof still prints. READY no. |
| #76 | Suspend restamped a spawned cloud-agent lease to unspawned after it had already dropped sessions. | That lease is a refuse before sessions drop and before the lease rewrite. A missing file is not a spawned lease. A wired box lease still drops. READY no. |
| #77 | `expire --forget` dropped an expired spawned cloud-agent lease, so the next apply could record a fresh unspawned row. | That lease is a refuse before the list and before the rewrite. A missing file is not a spawned lease. An expired box still drops when that cloud row is not in the drop. READY no. |
| #78 | `convey sync` wrote a cloud hop lease with `spawned: false` when the placement lease was spawned. | That lease is a refuse before the hop write. A missing file is not a spawned lease. An unspawned cloud placement still syncs. The placement file is not rewritten. READY no. |
| #79 | `convey call` said a cloud hop was not spawned, and restamped a missing hop lease to `spawned: false`, when the placement lease was spawned. | That lease is a refuse before the message and before the restamp. A missing file is not a spawned lease. An unspawned cloud hop still refuses as declared, not spawned. READY no. |
| #80 | `convey declare` wrote a cloud hop lease with `spawned: false` when that placement lease was spawned. | That lease is a refuse before the hop write. A missing file is not a spawned lease. An unspawned cloud hop still declares. The placement file is not rewritten. READY no. |
| #81 | `convey expire --forget` dropped an expired spawned cloud hop lease. | That lease is a refuse before the list and before the rewrite. A missing mesh is not a spawned lease. An expired box hop still drops when that cloud row is not in the drop. READY no. |
| #82 | `convey leases` printed hop lease JSON when a cloud-mesh hop lease was spawned. | That lease is a refuse before the JSON. An unspawned file still prints. A missing mesh is not a spawned lease. The mesh is not rewritten. READY no. |
| #84 | The README led with the Day-90 gate and catalog experiments, so the one-box factory was easy to miss. | North-star one-pager. `make real-world` is opt-in: check, vanilla doctor, live SKIP without an endpoint. Not in smoke or Actions. Distillation and a gateway stay non-goals. READY no. |
| #85 | `estate help north-star` banned a distillation lab, and Ollama read as the local product. | Glossary [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). Local runtime is an ecosystem seat; Ollama is today's entrant. Purpose-built SLM train/enrich is facilitated. Beachhead stays packs and the specialist path. Anti-shrink stays gateway, Ollama wrapper-as-product, LM Studio-alone, Grok Bot clone. READY no. |
| this slice | Packs and the specialist path were the only train/enrich beachhead. Nothing wrote a Modelfile or a portable trainer manifest. | `TrainEnrichDriver` prepares artifacts. `ollama-modelfile` joins Ollama create. `external-manifest` is the hatch. Sacred, SKU, curator, missing pack, and frontier-invent refuse before write. No train POST. No estate rewrite. `make enrich-prepare` is opt-in. READY no. |
| prepare loop | Prepare wrote files and stopped. The operator had no index, and no proposal that joined a created local tag back to `local_slm`. | `--all-drivers` writes every card or none. `NEXT.md` has the handoff. `estate enrich list` reads `.cell/enrich` and refuses when the directory is missing. `estate enrich import-prepared` writes a binding proposal and does not apply. Sacred, SKU, curator, tag, path, and frontier-invent still refuse. READY no. |
| apply-proposal | After import, the operator still pasted the `local_slm` snippet by hand. | `estate enrich apply-proposal` writes `.cell/enrich-stage/staged-estate.yaml` for the existing `estate plan` and `estate apply --require-plan`. The source estate is written only when that apply succeeds. Same tag again is a no-op. `--verify-local-tag` is off unless set. Status and doctor name a pending join. `make enrich-prepare` walks the throwaway lab copy. `examples/estate.yaml` stays hash-locked. READY no. |
| seated FROM | Modelfile `FROM` used the binding id `local_slm`. Ollama cannot create from that id. | `FROM` is `params.model` or a model-tag hint. The binding id is `refuse:base-model`. `estate enrich from-pack` prepares an accepted pack. `make enrich-live-prove` is opt-in `ollama create` on a throwaway cell, then the tag is removed. READY no. |

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
make real-world       # opt-in: check + vanilla doctor; live SKIP without CELL_LOCAL_ENDPOINT (not in smoke)
make enrich-prepare   # opt-in: prepare through require-plan apply on a throwaway lab copy (not in smoke)
estate help           # Day-90 topics, including enrich, frontier, and day90-mixed
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

Mac specialist complete is a recorded **PASS** on the MacBook Air: `"completion": "Pong"`, reason `compat completion`, tip `2ab78a4`. `READY_FOR_LIVE_TEST` for that command is no. Details: [`LIVE-PROBES.md`](LIVE-PROBES.md).

## Still parked (not green)

| Item | State |
| --- | --- |
| Native MLX | `specialist()` stays stub. Not the Ollama-on-Mac probe. |
| Live consumer / rented GPU | 5090 probes and specialist `Pong` are recorded. Not required to re-run for gates. Not native MLX. |
| Cloud-agent spawn | Declared only. Floor does not spawn. |
| Auto-promote / curator UI | Locked off / not built. Jason pastes pack ids. |
| Convey hop transport | Lease-bound mesh, not a gateway. |
| vLLM / TRT | Catalog cards until you verify. Probe does not print `live ok`. |
| Actions expansion | Compile-only unless you expand it. |

## Rails that still hold

GitHub is the only source of truth. No Origin. No auto-promote. Cloud
never spawned. Pause-safe disk. Not Dual PE, not a studio, not an
AI-gateway product. `examples/estate.yaml` hash stays
`sha256:dcd7164f04c83f514185e77d2d4f6c23cae6dbb27a9b5da96a28ba1f3c724930`.
