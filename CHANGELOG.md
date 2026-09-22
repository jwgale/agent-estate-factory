# Changelog

Local wrap: `make smoke`. Hosted CI is compile-only (`cargo check --workspace --locked` on pull_request). Day 0–90 is on `main`.

## Day 0–30 (PR #1)

One-box factory proving A1–A4. Horizon / Research / Sanctum on separate lanes. Sacred exclusions (Cyera CI, Rust classroom) fail closed. Isolation is a driver (profile-dir today). No live provider required. Dual PE, vault, multi-box, and AI-gateway stay out of altitude.

## Day 31–60 (on `main` with #1)

Mixed model estate A7–A9. Equal-class frontier (`grok-4.7`) + local. Ollama-first, llama.cpp swap-proof. Fail-closed when local is down — no silent `grok-4.7` fallback. Hardware is a driver (`consumer-nvidia` / `apple-silicon` / `rented-nvidia`), not a product fork. GitHub is the only source of truth.

## Day 61–90 (PR #2)

Beachhead toward A10–A12. Feed packs never auto-promote. Cloud-agent is declared, not spawned. Overnight waves added convey mesh, dry-run apply, lease TTL, doctor, smoke, dual-layer sacred file, and the GATE-90 checklist. Curator is Jason / manual.

## PR #3 — live probes and the day90 loop

Optional `estate probes --live` pings a specialist endpoint when you set `CELL_*`. Unset endpoints print SKIP and exit 0. CI never needs a Mac or a GPU. `make day90` walks status → plan → dry-run → apply → reconcile on an isolated cell. Probe ids refuse hardware SKUs the same way bindings do.

## PR #4 — suggest and accept stay instructions-only

`estate reconcile --suggest` writes a patch file. It does not rewrite leases. `estate packs accept --curator jason` writes enrich-pack edit instructions. It does not rewrite `estate.yaml`. Wrong curator refuses. Jason still pastes by hand.

## PR #5 — doctor --strict and a thin gate-90

`estate doctor --strict` is the pre-merge operator check: compile-only CI body, locked sacred file, dual-layer demo, refuse fixtures, floor has no vendor needles. `make gate-90` is a thin local alias (smoke + day90 + that checklist). The dual-layer demo keeps Sanctum first-class; Sanctum is not Cyera. Omitting a locked sacred id from the overlay file still refuses Cyera CI.

## PR #6 — feed-loop and honest parking

README leads with `make gate-90` as the Day-90 operator entrypoint. `make feed-loop` walks scrubbed traces → pack → propose → accept on an isolated cell. The feed cursor stays on disk; rematerialize does not auto-promote. Placement-actual JSON round-trips every reconcile refuse code. `docs/DAY90-PLUS.md` parks live Mac MLX, live GPU, and cloud-spawn until Jason has boxes. Those rows are not green.

## PR #7 — help, backup prune, convey policy

`estate help [topic]` prints Day-90 pages for status, plan, apply, reconcile, feed-loop, and backup. Unknown topics refuse. `estate backup --prune N` keeps the newest N archives; `N=0` refuses. Convey `call` refuses on `policy-deny.yaml`.

## PR #8 — layout, local-only honesty, operator runbook

`docs/cell-layout.md` matches the paths the code writes. `docs/OPERATOR-DAY.md` walks `make gate-90` → `make feed-loop` → `estate backup --prune` on isolated cells. A hardening test locks `make gate-90` off Actions (it wraps `cargo test --workspace`). No leftover Origin URLs. No new `estate version` command.

## PR #9 — dual-layer-demo e2e and Cell One snapshot

Isolated dual-layer-demo loop: validate → plan → dry-run → apply → status → reconcile → backup → prune. README cross-links OPERATOR-DAY and FEED-LOOP. Dead leftover `ops.rs` wrappers removed. Snapshot: `docs/CELL-ONE-STATUS.md`.

## PR #10 — lease-refresh after expire --forget

Hole: `estate expire --forget` dropped leases, then apply treated that as `refuse:drift` and demanded `--force`. Apply now restamps (`lease-refresh`). Isolated TTL e2e on `examples/fixtures/ttl-short.yaml`. Not a new verb.

## PR #11 — sacred overlay e2e

Isolated sacred overlay e2e: `sacred-omit-locked.yaml` (`locked: []`) still refuses `cyera-ci` / `rust-classroom` on convey hop. `lab-notebook` refuses only with the overlay installed. Dual-layer-demo still validates. No new verb.

## After PR #11 (this slice)

- Hole: `estate convey expire --forget` dropped hop decls with the leases, then call returned `refuse:no-lease`. Forget now keeps decls; call restamps (`lease-refresh`). Mirrors placement apply after forget. Not a new verb.
- Isolated hop TTL e2e: declare `ttl-secs: 1` → expire lists → call `refuse:expired` → `--forget` → call restamps. No JSON mutation.
- Pause-kit still holds after an unchanged apply → suspend → resume.

## After PR #12 (this slice)

- Hole: `estate restore` treated empty/missing backup `sacred_ids` as a match and could write. Empty set is now `refuse:sacred-mismatch` (fail closed). Dry-run and live restore both write nothing.
- Packs propose → accept → promote still leaves `promoted=false` and does not write the estate. `reconcile --suggest` on drift still does not rewrite leases. Vanilla `doctor` stays thinner than `--strict`.

## After PR #13 (PR #14)

No new hole. `plan diff --allow-wider` / `plan export-pr` exits locked. `apply --dry-run` under refuse writes nothing (snapshot covers conveyor/sessions too). Curator: wrong → `refuse:curator`; accept missing flag is clap; import still defaults to jason. CELL-ONE-STATUS states #10–#13 in plain English.

## After PR #41 (this slice)

- Apply and resume write the cell `catalog.json` frontier model from the estate binding. No `params.model` stays empty. Status prints `catalog frontier: cell model=-`. Doctor does not report that file as `grok-4.7`.
- The schema catalog card stays `grok-4.7`. `estate catalog` still dumps that card. A binding that sets `grok-4.7` still writes it. Two different frontier models refuse.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #40 (this slice)

- `make day90-mixed` validates `examples/hosts/frontier-http.yaml` and prints status (`frontier: frontier_http model=grok-4.7`). No apply. No live key. A greenfield status does not invent a cell catalog. `examples/estate.yaml` cksum is unchanged.
- The host file stays off `scripts/fixtures-check.sh`, smoke, and gate-90.
- `estate models` prints `model=` from `params.model` when set, and `model=-` when it is not. The default estate does not invent `model=grok-4.7`.
- No new CLI.
- `READY_FOR_LIVE_TEST`: no.

## After PR #39 (this slice)

- `examples/hosts/frontier-http.yaml` names `model: grok-4.7` on a frontier `http-remote` binding, with a local `ollama` card. It is not a host-class alias and it is not on smoke or gate-90. `examples/estate.yaml` stays hash-locked.
- A frontier specialist prompt that mentions Cyera or Rust classroom still refuses as sacred when `CELL_FRONTIER_MODEL` is a hardware SKU. No POST. No invented completion. The SKU model path is not the refusal.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## #38–#39 in plain English

#38. `make feed-loop` checks that the pack, the proposal, and `enrich-edit.json` carry the same `source_drivers`. A missing tag with a nonzero frontier or local count refuses the index rewrite and leaves the previous INDEX. Accept serializes the enrich-edit JSON before it writes either file.

#39. Frontier and local `estate specialist` both refuse a Cyera or Rust classroom prompt before POST, and do not invent a completion. README and OPERATOR-DAY point at `estate help frontier` and `make day90-mixed`. Those stay off smoke and gate-90.

## After PR #38 (this slice)

- Frontier `estate specialist` refuses a Cyera or Rust classroom prompt before POST, and does not invent `"completion": "ok"`. The local specialist test locks the same two prompts.
- README start-here and `docs/OPERATOR-DAY.md` point at `estate help frontier` and `make day90-mixed`. Those stay off smoke and gate-90. `examples/fixtures/mixed-frontier-local.yaml` already names `model: grok-4.7` on the frontier `http-remote` binding. `examples/estate.yaml` stays hash-locked.
- No new CLI.
- `READY_FOR_LIVE_TEST`: no.

## #35–#37 in plain English

#35. Frontier refuse text names `CELL_FRONTIER_MODEL` and `CELL_FRONTIER_ENDPOINT`. A hardware SKU in the model id says which setting it came from and still refuses before any POST. GATE-90 and DAY90-PLUS keep green factory checks, recorded live proofs, and parked rows separate. Mac specialist stays optional. Native MLX and cloud-spawn stay parked.

#36. Pack INDEX and `estate feed list` print `drivers=` when `source_drivers` is present, and `drivers=-` when it is empty. A failed INDEX rewrite is an error. `make feed-loop` greps that line. There is no `make feed-loop-mixed`.

#37. `packs accept` copies that tag into the enrich-edit instructions. An empty list stays `-`. A tag that does not match `path_counts` refuses before the edit file is rewritten. A failed proposal INDEX rewrite is an error. `source_drivers` stays an additive v0 field.

## After PR #37 (this slice)

- `make feed-loop` checks the pack, the proposal, and `enrich-edit.json` carry the same `source_drivers` (`frontier` then `local`).
- Hole: a pack or proposal that omitted `source_drivers` while frontier or local counts were nonzero was indexed as `drivers=-`. The index rewrite now refuses and leaves the previous INDEX in place.
- Accept serializes the enrich-edit JSON before it writes either file, so a serialize failure does not leave a new markdown next to a stale JSON.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #36 (this slice)

- `packs accept` copies `source_drivers` from the proposal into the enrich-edit markdown, JSON, and paste comment. Empty stays `-` and is not invented as frontier. A tag that does not match `path_counts` refuses before the edit file is rewritten.
- Hole: `propose_enrich` swallowed a failed proposal INDEX rewrite, and an unreadable proposal was still listed by name. The rewrite is now an error, and the index line lists `drivers=`.
- Schema freeze: `source_drivers` stays additive on pack, specialist-pack, and enrich-proposal v0. Only `frontier` and `local`. Missing field defaults to `[]`. A rename is a v1.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #35 (this slice)

- Pack INDEX and `estate feed list` / `estate packs list` print `drivers=` from `source_drivers` (`frontier,local` when both are present, `-` when empty). Empty packs do not invent a source.
- Hole: `write_drop_pack`, import, and `feed list` swallowed a failed INDEX rewrite. A failed rewrite is now an error.
- `make feed-loop` greps INDEX for `drivers=frontier,local`. Did not add `make feed-loop-mixed`; the existing walk already tags mixed frontier and local traces. Still off smoke and gate-90. No new CLI.
- `READY_FOR_LIVE_TEST`: no.

## After PR #34 (this slice)

- `estate specialist --driver frontier` refuse text names `CELL_FRONTIER_MODEL` and `CELL_FRONTIER_ENDPOINT`. A missing `XAI_API_KEY` already did. A hardware SKU in the model id now says whether it came from `CELL_FRONTIER_MODEL`, `XAI_MODEL`, or a binding model param, and still refuses before POST.
- `docs/GATE-90.md` and `docs/DAY90-PLUS.md` split green factory checks, recorded live proofs, and parked rows. Mac specialist complete is optional and not recorded. Native MLX stays a stub. Cloud-spawn stays off. The recorded 5090 proof is not called parked.
- CELL-ONE-STATUS states #32–#34 in plain English.
- `READY_FOR_LIVE_TEST`: no.

## After PR #33 (this slice)

- `make feed-loop` asserts the produced pack `source_drivers` is `frontier` then `local`, counts are non-zero, and propose copies the tag. Live keys are unset. Still off smoke and Actions.
- `estate help frontier` names `grok-4.7`, the `XAI_API_KEY` gate, and that local down does not POST frontier. `estate help day90-mixed` stays opt-in.
- Hole: `run_task` swallowed a failed feed append and could still complete. A failed audit now refuses before frontier.
- `READY_FOR_LIVE_TEST`: no.

## After PR #32 (this slice)

- Feed packs tag `source_drivers` (`frontier` and/or `local`) from the events. The tag must match `path_counts`. Unknown drivers, a tag with a zero count, and a count with no tag refuse.
- Explicit `object_class: local` stays local even when the kind or note mentions frontier. Local-down does not tag frontier.
- Propose copies `source_drivers` onto the diff. `auto_apply` stays false. Promote stays off. The estate file is not rewritten.
- `READY_FOR_LIVE_TEST`: no.

## After PR #31 (this slice)

- `make day90-mixed` walks the mixed fixture: status → plan → `apply --require-plan` → status → doctor. Isolated cell. No live key. Not in smoke or Actions.
- `estate status` prints `frontier: <id> model=…` only when the binding sets `params.model`. Catalog lines print `grok-4.7` when `frontier.model` is in the schema catalog or the cell `catalog.json`. Doctor prints the same. A SKU model fails doctor.
- The default estate binding has no model param, so status does not invent `frontier: xai_grok model=grok-4.7`.
- `READY_FOR_LIVE_TEST`: no.

## After PR #30 (this slice)

- Mixed fixture operator path: `estate plan` then `estate apply --require-plan` writes model-actual, placement-actual, and catalog. Mock only. No `XAI_API_KEY`. No frontier POST. Local `ollama` specialist after apply still skips frontier.
- Catalog file SoT sibling card: model `grok-4.7`, streaming/tools/vision false, completion budget 64. Not a local probe. Not a context window.
- Requested local specialist does not POST frontier: `ollama` up, `http-remote` up, `llama.cpp` down, `mlx` / `vllm` / `trt` refuse.
- `READY_FOR_LIVE_TEST`: no.

## After PR #29 (this slice)

- Recorded frontier specialist live PASS: `--driver frontier`, model `grok-4.7`, `completion` `pong`, reason `frontier completion`. Key never printed. Env-gated `XAI_API_KEY`. No box hostname.
- `READY_FOR_LIVE_TEST`: no for that surface.
- Mixed fixture `frontier_http` (`http-remote`, model `grok-4.7`) + local `ollama`: `validate` and `apply --dry-run` stay green with no live key and no POST.
- Local specialist (`--driver ollama`) down or unset does not POST to frontier even when `XAI_API_KEY` and `CELL_FRONTIER_ENDPOINT` are set.

## After PR #28 (this slice)

- Frontier model id is **`grok-4.7`** (`CELL_FRONTIER_MODEL` or `XAI_MODEL`). The old `grok-3-mini` default is gone.
- `estate specialist --driver frontier` requires `XAI_API_KEY`. Optional `CELL_FRONTIER_ENDPOINT` (default `https://api.x.ai/v1`). Unset key refuses. Sacred and SKU refuse before POST. Mock-locked. No key in CI.
- `--driver http-remote` stays the local `CELL_LOCAL_ENDPOINT` card. Local down does not fall through to frontier.
- Cloud-agent standing default `reasoning_effort` xhigh is documented only. The factory chat POST sends `grok-4.7`.
- `READY_FOR_LIVE_TEST`: yes. One command with a real `XAI_API_KEY`.

## After PR #27 (this slice)

- `--driver frontier` is env-gated on `CELL_FRONTIER_ENDPOINT` (or `--endpoint`). `CELL_LOCAL_ENDPOINT` and `XAI_API_KEY` do not unlock it. Mock-locked OpenAI chat. Not in CI / smoke.
- Operator fixture: `estate apply` records `local_slm` in `model-actual.json`, then `estate specialist` complete against mock-local.
- Driver is resolved before the local endpoint, so `--driver frontier` no longer dies as a missing `CELL_LOCAL_ENDPOINT`.
- `READY_FOR_LIVE_TEST`: no. Mac specialist is the same Ollama complete already proven on 5090.

## After PR #26 (this slice)

- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) records Mac `probes --live` PASS, 5090 `probes --live` PASS, and 5090 `estate specialist` `completion` `Pong`. Native MLX stays stub.
- `make live-specialist` requires `CELL_LOCAL_ENDPOINT` (SKU endpoint refuses). Not in `make smoke` / `make gate-90` / Actions.
- Mixed-estate apply `--dry-run` stays write-free; `local_slm` binds `HttpLocal` against in-process mock (`/v0/specialist`).
- Apply / resume no longer swallow `catalog.json` write failure.
- `READY_FOR_LIVE_TEST`: no. Recorded surfaces already ran. Mac specialist chat and live Grok are still unrecorded.

## After PR #25 (this slice)

- OpenAI `/v1/chat/completions` with empty / missing / whitespace `message.content` falls through to Ollama `/api/chat` (same as probes try both shapes).
- Accepts content-array parts, `text`, or reasoning-only when the text is clearly non-empty.
- Both chat paths fail: HTTP status, model id used, `ollama pull llama3` / `CELL_LOCAL_MODEL`.
- Unset `CELL_LOCAL_MODEL` prefers first `/api/tags` id, then `/v1/models`.
- `READY_FOR_LIVE_TEST`: yes. 5090 retry of `estate specialist --driver ollama --prompt "Reply with the single word pong."`

## After PR #24 (this slice)

- `estate specialist --driver ollama --prompt` is a thin `HttpLocal` delegate. Default job `complete` returns model `completion`. Same helper as `model-estate specialist --job complete`.
- Sacred / empty / SKU refuse before any HTTP POST. Empty `message.content` refuses. A bad OpenAI body still does not try Ollama.
- Mock-local complete is `mock:{text}`. Compat OpenAI / Ollama complete is the model body (`ok` in-process).
- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) has the exact Mac Ollama command.
- `READY_FOR_LIVE_TEST` for this complete verb: yes. Jason already PASSed `probes --live`.

## After PR #23 (PR #24 specialist round-trip)

- Specialist chat posts the real request text (not dummy `ping`) through `HttpLocal` against mock HTTP. Last-POST capture locks it.
- `model-estate specialist` was the data-plane equivalent before `estate specialist` existed.
- llama.cpp server OpenAI path smoke: same adapter, `HttpLocal { runtime: LlamaCpp }` + CLI `--runtime llama.cpp`.
- Fail-closed: v0 200 unparseable refuses (no compat fall-through); OpenAI choices require `message.content`; SKU `CELL_LOCAL_MODEL` / listed model ids refuse.
- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) now has exact SKIP / live ok / down / specialist JSON lines.
- `READY_FOR_LIVE_TEST` for that policy-precheck verb: no. Do not ping Jason.

## After PR #22 (PR #23 adapter)

- Live probes GET `/v1/models` or Ollama `/api/tags`. Empty models list is up. Garbage / empty body is down. No invent success.
- `HttpLocal` adapter: factory `/v0/specialist`, then OpenAI chat / Ollama chat, then factory-owned policy. Native MLX `specialist()` stays stub. Mac proof is Ollama-on-Mac.
- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) is copy-paste for Jason Mac and Jason Linux/5090. In-process mock HTTP locks the adapter. CI stays SKIP without env.

## After PR #21 (this slice)

- Hole: `record_bindings` used `unwrap_or_default` and could wipe `model-actual.json`. Isolation `session.json` was the same class. Serialize or refuse. Garbage model-actual is refuse, not "run apply".
- Hole: accept / import / propose compared estate bytes with `unwrap_or_default`, so an unreadable file looked unchanged. `read_estate_text` refuses.
- Live probe hand-off: [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md). Dry fixture `schema/live-probe-shapes.v0.json` locks SKIP vs would-live without network. Not a live-box proof. Do not ping Jason yet.

## After PR #20 (this slice)

- Hole: `propose_enrich` used `unwrap_or_default` and could write an empty proposal. Serialize or refuse. No empty `.proposal.json`.
- Hole: `append_event` invented `"{}"` on serialize failure and could append junk to `events.jsonl`. Journal / audit lines serialize or refuse. Empty object is refuse.
- Hole: `write_placements` used `unwrap_or_default` and could wipe `placement-actual.json` empty. Same class closed on lifecycle, session journal, apply-audit, reconcile, backup meta, actual-state, mesh persist, and accept enrich-edit.
- CELL-ONE-STATUS records #20 and this hunt. Isolated locks stay crate tests plus `tests/day90_honesty.rs`.

## After PR #19 (this slice)

- Hole: `covering_plan` / `latest_plan` / `list_plans` treated unreadable plan JSON as empty. `apply --require-plan` could say "no plan"; `status` could invent last-plan. Present plan JSON parses or refuses. APIs return `Result`.
- Hole: `write_cursor` used `unwrap_or_default` and could write empty `feed-cursor.json`. A present garbage cursor could look missing. Load: missing → none; exists but not a file or parse fail → refuse. Write serializes or refuses.
- Hole: `apply` without `--force` could still walk a SKU `placement-actual`. `--force` now claims estate `host_class` (`rtx_consumer` → `consumer-nvidia`) and does not launder the SKU to `any`. Without `--force`, `refuse:bad-host-class` before dry-run.
- Remaining `.ok()` on Path-exists loads in estate-control / floor / conveyor / feed are listing skips, not SoT. `load_placements` stays permissive so `--force` can overwrite.
- CELL-ONE-STATUS records #19 and this hunt. Isolated lock stays `tests/day90_honesty.rs`.

## After PR #18 (this slice)

- Hole: `suspend` / `resume` / `apply` used `load_lifecycle(...).ok()`. A present-but-unreadable `lifecycle.json` was treated as greenfield and overwritten. Parse or refuse. Apply refuses before lease writes.
- Hole: Feed import wrote the accepted pack, then swallowed `append_import_audit` (`let _ =`). Audit append is fail-closed. Serialize no longer invents an empty line.
- Hole: `apply --dry-run --import-pack --curator robot` skipped the curator check. `refuse_import_pack` runs first. Live and dry-run write no leases, no accepted pack, no audit.
- `estate catalog` writes first, then prints. `estate probes` refuses every card first, then prints.
- Status no longer invents `expired=0` / empty proposals on reader failure. Plan `export-pr` no longer invents an empty expired list.
- CELL-ONE-STATUS records #18 and this hunt. Isolated lock stays `tests/day90_honesty.rs`.

## #15–#18 (SKU then honesty)

- #15: `convey sync` slim-parse laundered a SKU `host_class` to portable `any`.
- #16: `convey call` swallowed that refuse; floor / status / leases / reconcile stayed silent.
- #17: mesh readers and restore still copied a SKU. Production `canonical_host_class` callers gone.
- #18: print-then-refuse on leases; backup/restore estate `.ok()`; suspend swallowed actual-state / journal; feed redaction write was `let _ =`.

## After PR #17 (this slice)

- Hole: `estate leases` printed the SKU `placement-actual` JSON, then refused. Readers now refuse first. Status already refused before print; tests lock both.
- Hole: `backup` / `restore` swallowed a present-but-unreadable estate file (`load_estate(...).ok()`), so restore could invent a locked-only sacred set. A file that exists must parse or refuse.
- Hole: `suspend` swallowed `actual-state.json` parse errors and journal write failures, so unspawn lines could vanish. Load and journal fail closed.
- Feed import: `{id}.redaction.json` write is no longer `let _ =`. Report bytes are kind counts only; a raw secret in the report is `refuse:raw-secret`. Hand-written dirty packs write no accepted pack and no report.
- `lifecycle.jsonl` / `sessions.jsonl` stay append-only under suspend / resume / `expire --forget`. Forget does not truncate journals.
- Hole leftover from #17: floor `src` tests used `rtx-5090`, so `doctor --strict` failed the vendor-needle scan. Fixtures now use `not-a-host`.
- Sacred / curator / policy refuse paths had no new silent `Ok()` swallows.
- CELL-ONE-STATUS states the #15–#17 SKU launder story in plain English.
- Isolated lock: `tests/day90_honesty.rs`.

## After PR #16 (this slice)

- Hole: `convey call` / `list` / `expire` / `sync` loaded a tampered `conveyor-mesh.json` without checking `host_class`. A SKU hop/lease could allow a call or get written back. Readers now `refuse:bad-host-class` and write nothing.
- Hole: `restore` copied a SKU `placement-actual` (or mesh) onto disk. Restore is now `refuse:bad-host-class` (dry-run and live). Does not invent `any`.
- Remaining `canonical_host_class` is the trusted unwrap only. Hop declare stamps via opt after `refuse_hop`.
- Isolated lock: `tests/day90_mesh.rs`.

## After PR #15 (this slice)

- Hole: `convey call` swallowed slim-parse `refuse:bad-host-class`, so a tampered SKU `host_class` skipped the placement not-live check. Call now fails closed. Status / leases / reconcile use the same refuse. `record_placements` writes nothing; claim does not rewrite the SKU to `any`.
- Validate on multi-host + mixed fixtures stays green. Probe / catalog card ids still refuse SKUs the same way hop ids do.
- Isolated lock: `tests/day90_sku.rs`.

## After PR #14 (this slice)

- Hole: `convey sync` rewrote unknown / SKU `host_class` on a tampered `placement-actual.json` to `any` and seeded a hop. Slim-parse now `refuse:bad-host-class` and writes no mesh files. Alias round-trips stay (`rtx_consumer` → `consumer-nvidia`).
- Overlay with `locked: []` still cannot drop Cyera CI / Rust classroom (property lock).
- Two `apply --dry-run` after a real apply leave an identical `.cell` tree. Dual-layer-demo backup → restore with matching sacred writes the leases back.
- Makefile contract: `gate-90` / `smoke` / `day90` / `feed-loop` / `fixtures-check` / `doctor-strict` exist; `gate-90` does not invoke `gh`.

## Still stubbed

MLX / vLLM / TRT live runtimes (probe path only). Cloud-agent spawn. Convey hop transport (lease-bound mesh only). Auto-promote. Curator UI. See `docs/DAY90-PLUS.md`.

## How to run

```bash
make gate-90    # Day-90 operator entrypoint (local)
make smoke      # doctor + fixtures-check + operator-day + cargo test + make day90
make day90      # operator loop only
make feed-loop  # scrubbed trace → pack → propose → accept (fixtures only)
estate help     # Day-90 topic pages
# walk: docs/OPERATOR-DAY.md
# snapshot: docs/CELL-ONE-STATUS.md
```
