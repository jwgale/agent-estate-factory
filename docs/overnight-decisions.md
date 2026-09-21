# Overnight decisions (Day 61–90 beachhead)

Jason was asleep. These are the seams we picked for **maximum flexibility**. Everything can change later. Not a product freeze.

## What we shipped

1. **Feed → pack format, no auto-promote.** Scrubbed `events.jsonl` (frontier + local + proxy) materializes to `packs/{id}.pack.json` (`schema=cell-one.pack.v0`). Curator stays `jason`, policy stays `manual`, `promoted` is always false. `estate feed import` is explicit apply (Feed→Control) and does not rewrite the estate. `estate feed promote` exists only to fail closed. Notion beachhead already exists (do not recreate): https://app.notion.com/p/3e2b09f5050681c79063ca36c0affd87
2. **Suspend / resume as the operator lifecycle.** `estate suspend` discards sessions/PIDs and writes durable `.cell/lifecycle.json`. `estate resume` re-applies from the estate file. Pause kit calls those commands. Lifecycle is *not* SoT for desired-state; the estate file is.
3. **Plan is the human control surface.** `estate plan` writes markdown + JSON plus a `Reviewable diff` block and regenerates `plans/INDEX.md`. `estate apply --require-plan` is gated on a covering plan hash and writes `plans/apply-*.json` plus `.cell/apply-audit.jsonl`. `estate plans` lists history. Generated plans stay gitignored; commit a specific `.md` into a PR when Jason should review apply.
4. **Drivers stay swappable.** Ollama / llama.cpp / http-remote bind the same specialist protocol via `CELL_LOCAL_ENDPOINT`. MLX stays a stub. `host_class` is validated. Wrong host on a card fail-closes at bind (no frontier fallback).
5. **Cloud-agent placement is declared, not wired.** `placements[]` on the estate: `box` (this Cell One) and `cloud-agent` (`cursor-cloud`, `wired: false`, no agents). Apply/resume write `.cell/placement-actual.json` leases (`box` spawned, cloud-agent not). Floor still binds one session per estate agent. Day-90 operator day can assign agents without a schema break.
6. **Placement is a driver.** `PlacementDriver` (`box` / `cloud-agent`) is the swappable seam. `CloudAgentDriver.claim` never sets `spawned=true`, even if the estate row is wired. Floor CLI now has `suspend` / `resume` / `leases`. Drift fail-closes if a cloud-agent lease is spawned or if declared leases are missing.
7. **Pack drop zone refuses SKUs and writes INDEX.** Pack ids must be slugs and must not encode a hardware SKU. `host_class` must be portable. `write_pack_index` regenerates `packs/INDEX.md`.
8. **Catalog is file SoT.** `schema/local-catalog.v0.json` matches `model_estate::catalog_file()`. Apply/resume dump `.cell/catalog.json`. `estate catalog` / `estate leases` / `estate audits` are control-surface reads (no model invoke).
9. **Feed cursor + import audit.** `feed-cursor.json` is a durable watermark over `events.jsonl`. Packs carry `path_counts` (frontier/local/proxy). Events that encode a SKU fail closed. Explicit import appends `import-audit.jsonl`. Still no auto-promote.
10. **Lifecycle history.** `lifecycle.json` is v1 (`cell-one.lifecycle.v0`). Transitions append `lifecycle.jsonl`. `estate history` / `floor history` read it. Pause-safe: sessions die, history stays.
11. **Plan freshness.** Plans are `cell-one.plan.v0`. `covering_plan` returns the plan, not just a stem. `apply --require-fresh-plan` fails when `against_hash` does not match last apply. INDEX lists hashes.
12. **Placement release + host_class drift.** `PlacementDriver::release` unspawns without deleting the lease file. Drift fail-closes host_class mismatch. Cloud placements refuse sacred agent ids. Driver `probe()` is catalog-level (`live_probed=false`); not a live ping.

## Wave 2 (same PR #2 — deepen factory substance)

Jason still asleep. Same branch. No Actions. Local cargo only.

13. **Conveyor capability mesh is a driver.** `ConveyorHop` (`box` / `cloud-mesh`) matches `PlacementDriver` / `LocalDriver`. Declare a hop, persist `.cell/conveyor-mesh.json` + hops/leases files, lease-bound `estate convey call`. No lease → refuse. Cloud-mesh declare is fine; call always refuse (not spawned). Slim-parses `placement-actual.json` so conveyor-proxy does not depend on floor. File SoT: `schema/conveyor-mesh.v0.json`. Not a gateway.
14. **Security-as-IaC is the plan markdown.** `write_plan` also writes `{stem}.security.md`. `estate plan --reviewed` copies md/json/security into `plans/reviewed/`. `plan_is_reviewable` requires `cell-one.plan.v0` + blast radius. `--require-fresh-plan` now uses `plan_against_is_fresh_strict`: a greenfield covering plan after an apply is stale. Fixtures: `examples/valid/covering-plan.json`, `examples/invalid/stale-plan.json`, `examples/invalid/host-class-bad.yaml`.
15. **Pack curator path is first-class CLI.** `estate packs list|import|promote|index` (promote still fails). `scrub_pii` redacts `sk-` / `xai-` / bearer / emails on feed notes. Pack INDEX already had `path_counts`; still no auto-promote. Fixture pack: `examples/fixtures/overnight-traces.pack.json`.
16. **Operator day is a local dry-run.** `make operator-day` / `scripts/operator-day.sh` walks validate-hosts → plan → apply → suspend → plan --reviewed → apply --require-fresh-plan → packs import → convey sync/call/refuse → resume. Fixtures only. No live Grok, no 5090, no Actions.
17. **Host aliases map; locked names stay.** `rtx_consumer` / `rtx-consumer` → `consumer-nvidia`; `nvidia_rental` / `nvidia-rental` → `rented-nvidia`. Placement leases store the canonical name. Drift compares via `host_class_eq` (alias vs canonical is in-sync; apple vs consumer fail-closes). Matrix: `examples/hosts/{rtx-consumer,apple-silicon,nvidia-rental}.yaml`. MLX stays stub, catalog-complete.

## Wave 3 (same PR #2 — factory hardening)

Jason still asleep. Same branch. No Actions. Local cargo only. Wave 2 stays.

18. **Placement reconcile is a report, not a fixer.** `estate reconcile` writes `.cell/reconcile.json` + `reconcile.md` (`cell-one.reconcile.v0`). Desired vs actual rows, refuse codes (`missing-lease`, `extra-lease`, `kind-mismatch`, `host-class-mismatch`, `cloud-spawned`, `sacred-id`). Drift notes use the same `refuse:` prefix. Sacred-id is scanned on *actual* lease agents even if `estate.yaml` is clean (tamper fail-closed). Conveyor `MeshError` Display is now `refuse:no-lease` / `refuse:ungranted` / `refuse:cloud-not-spawned` / … Variant matching is unchanged.
19. **Enrich propose never applies.** After an explicit pack import, `estate packs propose` writes `packs/proposed/{id}.proposal.json` + `.md` (`cell-one.enrich-proposal.v0`). `auto_apply` is always false. Diff summary is for curator Jason. Estate file is not rewritten. `refuse_apply_proposal` exists only to fail closed.
20. **Multi-host fixture is one estate.** `examples/hosts/multi-host.yaml` spans frontier-http + ollama + http-remote + llama.cpp stub + mlx stub, plus empty unwired box placements (`box-rtx` / `box-apple` / `box-rental`) and the cloud-agent stub. `estate validate` is green. Extra unused bindings are valid. Empty box placements are valid. Wired cloud-agent + empty agents is still invalid.
21. **Audit export is local-only.** `estate audit export` bundles plan history, `lifecycle.jsonl`, `apply-audit.jsonl`, `import-audit.jsonl`, convey leases, placement-actual, and reconcile into a folder (`MANIFEST.md`). `--tar` writes `{out}.tar.gz` when `tar` exists. Not uploaded. Not a gateway dump.
22. **`.cell/` lease layout is documented.** [`docs/cell-layout.md`](cell-layout.md). Leases store canonical `host_class`. Reconcile does not rewrite them.
23. **Operator-day grew one notch.** Validate multi-host, packs propose, refuse-prefix check, reconcile, audit export. Still fixtures only. Still no live Grok / GPU / Actions.

## Wave 4 (same PR #2 — apply preview + lease ttl)

Jason still asleep. Same branch. No Actions. Local cargo only. Waves 2–3 stay.

24. **Apply dry-run does not write.** `estate apply --dry-run` prints blast radius + preview leases + would-refuse. No `placement-actual`, sessions, snapshots, or audits. Exit 1 on expired / sacred-id / cloud-spawned (and plan gates if those flags are set). Missing leases are not would-refuse (apply would record them).
25. **Lease TTL is optional.** `ttl_secs` on a placement stamps `issued_at` / `expires_at` on the lease. `estate expire` lists elapsed rows (exit 1). apply/resume refuse expired. `estate expire --forget` drops expired rows so apply can record fresh leases. Does not spawn. Reconcile reports `refuse:expired`.
26. **Scrub is wider; import writes a redaction report.** `scrub_pii` covers GitHub/HF/AWS/Slack/PEM/`password=` plus the Wave 2 keys. Import refuses raw secrets (never stored in accepted packs) and writes `{id}.redaction.json` (kind counts only).
27. **Specialist pack v0 is additive.** `source_paths`, `model_hint`, `host_class_affinity` validate on import/propose. `cell-one.pack.v0` and `cell-one.specialist-pack.v0` both load. SKU / absolute source paths / bad host affinity fail closed.
28. **`estate doctor` is one page.** Schema files present, `.github/workflows/*.yml` absent (quiet hours), `.cell` layout notes, expired/cloud-spawned fail. Greenfield `.cell` is a note, not a fail.
29. **Operator-day grew dry-run / doctor / expire / redaction.** Still fixtures only.

## Wave 5 (same PR #2 — journal, hop ttl, plan diff)

Jason still asleep. Same branch. No Actions. Local cargo only. Waves 2–4 stay.

30. **Session journal is append-only.** Apply/suspend/resume write `.cell/sessions.jsonl` (`cell-one.session-journal.v0`) for spawn / unspawn / suspend / resume. `estate sessions list|tail`. Journal is not SoT. `sessions/` profiles stay disposable. Pause-safe: the journal survives suspend.
31. **Convey hop TTL mirrors placement TTL.** Optional `ttl_secs` on `HopDecl` stamps `issued_at` / `expires_at` on the hop lease. `estate convey call` refuses expired (`refuse:expired`). `estate convey expire` lists (exit 1); `--forget` drops rows (does not spawn).
32. **`estate plan` still generates.** `estate plan --estate` is unchanged. `estate plan diff` compares two plan JSON files or last-applied vs new. Human-readable width. Exit 1 if blast radius grows without `--allow-wider`.
33. **Fixture library is a thin gate.** `examples/fixtures/happy.yaml` plus `refuse-*.yaml` for each validate refuse code. `make fixtures-check` / `scripts/fixtures-check.sh` runs `estate validate` (ok / fail-closed) and `estate doctor`.
34. **`apiVersion` / `kind` fail closed.** Absent `apiVersion` is legacy `version: 0`. Known: `cell-one.estate.v0` / `v0` and `kind: agent-estate`. Unknown values refuse with an upgrade hint. `examples/estate.yaml` is unchanged (hash-stable).
35. **Operator-day grew sessions / plan-diff / hop expire / fixtures-check.** Still fixtures only.

## Wave 6 (same PR #2 — backup, policy, catalog caps)

Jason still asleep. Same branch. No Actions. Local cargo only. Waves 2–5 stay.

36. **Backup/restore is a local cell archive.** `estate backup` copies durable `.cell/` files (plans history, lifecycle, leases, audits, journals, conveyor mesh) into `backups/cell-backup-unix{secs}/` with `backup.json` (`cell-one.cell-backup.v0`) + `MANIFEST.md`. `estate restore --dry-run` writes nothing. Restore refuses `refuse:sacred-mismatch` when backup sacred ids are not a symmetric match to the current estate (canonical exclusion ids only; aliases omitted so backup-with-estate matches restore-with-estate). Not uploaded.
37. **Policy pack is a declarative deny/allow stub.** `policy/cell-one.policy.v0.yaml` is checked on apply and convey-call (also backup/restore). Known actions: `apply` / `convey-call` / `backup` / `restore`. Unknown actions fail closed (`refuse:unknown-action`). Missing file allows (crate-cwd tests). Default is deny without a matching allow. Fixtures: `examples/fixtures/policy-{allow,deny,unknown-action}.yaml`. `estate policy check`.
38. **Catalog cards are flag-complete.** Each driver has `streaming` / `tools` / `vision` / `context_tokens`. `estate catalog` prints them. File SoT `schema/local-catalog.v0.json` stays eq to `catalog_file()`. MLX/vLLM/TRT stay stub/experimental; flags are filled in.
39. **Pause-kit proof is automated.** `estate pause-proof` / `pause_kit_proof`: apply → suspend → kill-process simulation (`rm -rf sessions/`) → resume. Leases survive on disk. Cloud-agent stays unspawned. Drift in_sync. Operator-day runs it on a separate state dir.
40. **`make smoke` is local only.** `scripts/smoke.sh` = doctor + fixtures-check + operator-day + `cargo test --workspace`. Operator-day does not call smoke (no recursion). Never add to Actions.
41. **Operator-day / fixtures-check grew Wave 6.** Catalog flags, policy allow/deny/unknown, backup + dry-run restore + sacred-mismatch, pause-proof. Still fixtures only.

## Wave 7 (same PR #2 — status watch, import gate, sync)

Jason still asleep. Same branch. No Actions. Local cargo only. Waves 2–6 stay.

42. **`estate status` is a desired-state one-pager.** paused?, lease counts (box / cloud-agent / spawned), expired placement+hop counts, last plan hash, last apply, open proposals, policy present?, doctor summary line, in_sync. Still fail-closes if a cloud-agent lease is spawned. `cloud-agent: declared, not spawned` stays in the output for gate-90.
43. **Import is curator-gated.** `estate packs import` / `estate feed import` / `apply --import-pack` take `--curator` (default `jason`). Must match locked curator **and** `estate.enrich_packs.curator`. Wrong curator → `refuse:curator`. Library `import_pack` still assumes jason. Documented in `packs/README.md`.
44. **Convey sync upserts from placements.** Matching kinds only (`box` → box hop, `cloud-agent` → cloud-mesh). Unknown kinds skipped. Sacred placement id or agent → `refuse:sacred-id`. Manually declared hops (e.g. ttl-box) survive a later sync.
45. **Schema freeze note.** `schema/README.md` lists every v0 snapshot. Additive fields ok; renames need v1.
46. **Day-90 gate doc.** `docs/GATE-90.md` maps A10–A12 + waves to local commands. Not an Actions workflow.
47. **Operator-day grew Wave 7.** Status one-pager, `--curator jason`, wrong-curator refuse, convey sync keeps extra hops.
48. **`estate-control` split is push-safe, not a behavior change.** `main.rs` stays clap + `run` + validate. Commands live in `helpers.rs` / `plan_apply.rs` / `watch.rs` / `ops.rs` so GitHub MCP can land full files (no truncation).

## Wave 8 (same PR #2 — idempotent apply, export-pr, sacred file)

Jason still asleep. Same branch. No Actions. Local cargo only. Waves 2–7 stay.

49. **Apply is idempotent.** Second apply with identical desired hash + in_sync is a no-op (exit 0, audit note `unchanged`). Hash match + drift → `refuse:drift` unless `--force`. After suspend, sessions/ is gone so reconverge is `--force` (pause-safe, explicit). `--import-pack` skips the no-op (explicit Feed→Control).
50. **`estate plan export-pr` is a paste artifact.** One markdown: blast radius, covering hash, reviewed flag, refuse risks. Does not open a GitHub PR. Does not auto-promote.
51. **Sacred is dual-layer.** Hardcoded `LOCKED_SACRED` always applies. `policy/sacred.yaml` overlays are additive (`lab-notebook` fixture). File cannot remove a locked id. Apply / convey / cloud-agent assignment refuse on hit. Missing file = hardcoded only.
52. **Mixed proof fixture.** `examples/fixtures/mixed-frontier-local.yaml` declares http-remote frontier + local ollama. `estate validate` + `apply --dry-run` green. No live calls. Catalog still lists both cards.
53. **`CHANGELOG.md` is the morning read.** Day 0–30, 31–60, waves 1–8. Not a product essay.
54. **Operator-day / fixtures-check / doctor grew Wave 8.** Unchanged apply, drift/`--force`, export-pr, overlay hop refuse, mixed dry-run. Doctor requires `policy/sacred.yaml`, `schema/sacred.v0.json`, `CHANGELOG.md`.

## Wave 9 (hardening / morning-prep)

55. **Refuse-code tests, not new CLIs.** Mesh: kind / bad-id / host-class / capability / not-live / sku display. Feed: missing-pack / no-auto-apply / raw-secret. Reconcile: extra-lease + kind-mismatch. `examples/estate.yaml` hash locked (`sha256:dcd7164f…`); YAML comments allowed. Doctor required-file lists are one const.
56. **Morning artifacts.** `docs/MORNING-BRIEF-2026-09-21.md` + `docs/PR2-DESCRIPTION.md`. README start-here. `gh` not logged in; PR body via GitHub MCP + paste file. No extra PRs.

## Assumptions (safe to reopen)

| Assumption | Why | Revisit |
| --- | --- | --- |
| One overnight branch, not stacked PRs | Less CI, one review surface | Split later if noisy |
| `lifecycle.json` lives under `.cell/` and is durable | Operator intent survives restart; still not estate SoT | Move next to `plans/` if Jason wants it committed |
| Plans are local artifacts unless force-added | Avoid noisy generated diffs | Track `plans/reviewed/` if that becomes the ritual |
| Cloud-agent `wired:true` with empty agents is invalid | Stops a half-spawn | Allow when a real spawn driver exists |
| Pack ids are slugs Jason chooses | Drop zone is a file convention, not a registry | Add a pack schema crate later |
| No hosted Actions overnight | Quiet hours + cost lock | `.github/workflows/ci.yml` **deleted on this branch and on main**. Pushes/PRs must not email Jason. Re-enable tomorrow as compile-only (`cargo check --workspace --locked`) only if he wants. Real `cargo test` stays local / Makefile. |
| A10–A12 here means *beachhead*, not a full workday | Progress > polish | Next cell: assign cloud agents, curator UI is still forbidden |
| Wave 2 mesh is lease-bound, not a hop runtime | Same class as cloud-agent placement stub | Real hop transport later behind `ConveyorHop` |
| `plan_against_is_fresh` stays greenfield-friendly | Existing tests / `--require-plan` | `--require-fresh-plan` is the strict path |
| Host aliases are normalize-only | Do not reopen locked `host_class` names | Add aliases, not new product forks |
| Reconcile reports; it does not heal | Operator reads refuse codes and applies/edits | Auto-heal only if Jason asks |
| Enrich proposals never apply | Same lock as no auto-promote | Curator UI is still forbidden |
| Audit export stays on the box | Quiet hours + no remote dump | Zip-to-PR later if Jason wants a review pack |
| Extra unused bindings / empty box placements are valid | Multi-host fixture needs them | Do not require every binding to be assigned |
| Apply dry-run never writes | Preview is the control surface next to plan | Keep it that way |
| Expired leases refuse apply/resume | Fail closed; `--forget` is the refresh | Auto-renew only if Jason asks |
| Specialist pack fields are optional | Existing `cell-one.pack.v0` fixtures stay valid | Do not require model_hint |
| Session journal is not SoT | Same class as lifecycle.jsonl | Do not treat it as desired-state |
| Plan blast width is added+removed+changed counts | Human gate, not a session-count heuristic | Revisit if Jason wants session-count width |
| `apiVersion` is optional | Legacy `version: 0` estates stay valid | Require it only after a hash-stable cut |
| Missing policy file allows | Crate-cwd CLI tests have no `policy/` | Repo-root operator-day enforces the shipped pack |
| Backup sacred compare uses canonical ids | Aliases would mismatch backup-with-estate vs locked-only | Do not store alias strings in `backup.json` |
| `make smoke` is not in operator-day | Recursion / double cargo test | Keep smoke as the outer local wrap |
| `--curator` defaults to jason | Existing CLI/gate scripts stay green | Require the flag only if Jason wants ceremony |
| Convey sync skips unknown placement kinds | Not every future kind is a hop | Add a mapping, do not invent a product fork |
| Unchanged apply requires in_sync | Hash-only no-op would hide discarded sessions | `--force` after suspend |
| Sacred overlays are thread-local | CLI loads `policy/sacred.yaml`; crate tests stay hardcoded-only | Do not put overlays in `locked_sacred_ids()` |
| `lab-notebook` is a fixture overlay | Documents the file layer | Not a new product fork |
| `export-pr` does not open a PR | Quiet hours; paste-only | Human pastes into PR #2 if wanted |

## Anti-shrink (still)

This is not a gateway, not LM Studio, not an auto-promote feed, not a cloud-agent farm. Cloud-agent is a **declared placement**, same class as `box`, behind the estate file.

## CI (overnight quiet)

Jason’s inbox was filling with Actions failure mail. Until ~7am America/Chicago:

- No GitHub Actions workflows. `ci.yml` is gone on `main` and on `cursor/day61-90-beachhead-2950`.
- Do not add .workflow file overnight. Do not open extra PRs that would retrigger CI.
- Gate is local only: `cargo check --workspace --locked`, `cargo test --workspace`, `make gate` / `make gate-60` / `make gate-90` / `make smoke`.
- Tomorrow, if Jason wants hosted CI back: one `pull_request` job, `cargo check --workspace --locked` only, timeout ≤ 10. Never `cargo test` on Actions overnight.

## Origin

Zero Origin remotes or URLs. SoT is https://github.com/jwgale/agent-estate-factory
