# Day-90 gate (local only)

A10–A12 plus overnight waves are **on `main`** (PR #1–#23 plus this slice). Hosted CI is compile-only (`cargo check --workspace --locked` on `pull_request`). Real cargo test stays local. `make gate-90` is local on purpose - it wraps `cargo test --workspace`. See [`OPERATOR-DAY.md`](OPERATOR-DAY.md). Snapshot: [`CELL-ONE-STATUS.md`](CELL-ONE-STATUS.md).

`make gate-90` is the Day-90 operator entrypoint. Live Mac MLX / GPU / cloud-spawn wait in [`DAY90-PLUS.md`](DAY90-PLUS.md). Hand-off: [`LIVE-PROBES.md`](LIVE-PROBES.md).

```bash
make gate-90    # Day-90 operator entrypoint (local)
make smoke      # doctor + fixtures-check + operator-day + cargo test + day90
make day90      # status → plan → dry-run → apply → reconcile → --suggest
make feed-loop  # scrubbed trace → pack → propose → accept (not in smoke)
estate help     # Day-90 topic pages
estate doctor --strict
# walk without live boxes: docs/OPERATOR-DAY.md
```

## Green on main / this slice

| Item | Status | Local command |
| --- | --- | --- |
| A10 feed pack / import gate / no auto-promote | green | `make operator-day` |
| A10 estate unchanged on import/propose | green | operator-day cksum |
| A11 suspend / resume / pause-kit | green | `estate suspend` / `resume` / `pause-proof` |
| A12 plan / gated apply / cloud-agent stub | green | `estate plan` / `status` |
| Waves 2–8 (convey, dry-run, doctor, smoke, sacred file, mixed proof) | green | `make smoke` |
| Day 90+ operator loop | green | `make day90` |
| Live probes SKIP without endpoints | green | `estate probes --live` |
| Probe ids refuse SKUs | green | fixtures-check + unit tests |
| Compile-only CI | green | `.github/workflows/ci.yml` |
| `estate doctor --strict` | green | pre-merge operator checks |
| Dual-layer sacred demo (Sanctum is not Cyera) | green | `examples/fixtures/dual-layer-demo.yaml` |
| Omit-locked sacred file still refuses Cyera CI | green | `--sacred sacred-omit-locked.yaml` |
| Sanctum-as-Cyera display-name bleed | green | `refuse-sanctum-as-cyera.yaml` |
| Thin `make gate-90` | green | smoke + day90 + checklist print |
| README Day-90 operator entrypoint | green | leads with `make gate-90` |
| `make feed-loop` | green | scrubbed trace → pack → propose → accept |
| Feed cursor durability | green | schema + packed_id + rematerialize keeps cursor |
| Placement-actual refuse round-trip | green | every reconcile refuse code, schema preserved |
| Honest live-box parking lot | green | [`DAY90-PLUS.md`](DAY90-PLUS.md) |
| `estate help` topic pages | green | `estate help status` / `plan` / `apply` / `reconcile` / `feed-loop` / `backup` |
| `estate backup --prune N` | green | keep newest N cell archives; `N=0` refuses |
| Convey call policy deny | green | `policy-deny.yaml` refuses `convey-call` |
| `.cell/` layout doc matches code | green | [`cell-layout.md`](cell-layout.md) |
| Operator day runbook (no live boxes) | green | [`OPERATOR-DAY.md`](OPERATOR-DAY.md) |
| `make gate-90` stays off Actions | green | wraps `cargo test`; hosted stays compile-only |
| Dual-layer-demo operator e2e | green | validate → plan → dry-run → apply → status → reconcile → backup → prune |
| README ↔ OPERATOR-DAY / FEED-LOOP | green | start-here cross-links both loops |
| Cell One snapshot (Jason) | green | [`CELL-ONE-STATUS.md`](CELL-ONE-STATUS.md) |
| Placement TTL expire → forget → re-apply | green | `ttl-short.yaml` stamps `ttl_secs`; apply restamps after `--forget` |
| Sacred overlay e2e (omit-locked KEEP) | green | `sacred-omit-locked.yaml` + convey hop; hardcoded ids still refuse |
| Hop TTL expire → forget → call restamp | green | hop decls survive `--forget`; call restamps `lease-refresh` |
| Pause-kit after unchanged apply | green | apply no-op → suspend → resume stays in_sync |
| Restore empty `sacred_ids` fail-closed | green | empty/missing backup set is `refuse:sacred-mismatch`; no writes |
| Pack propose→accept→promote leftover | green | no `promoted=true`; estate unchanged; promote writes nothing |
| `reconcile --suggest` on drift | green | patch only; `placement-actual.json` bytes unchanged |
| Vanilla doctor vs `--strict` | green | vanilla stays schema+CI; `--strict` is the pre-merge extras |
| Plan diff / export-pr exits | green | wider refuses unless `--allow-wider`; export-pr exits 0 with risks; no `.cell` write |
| Dry-run refuse writes nothing | green | `--require-plan`, policy-deny, expired; snapshot includes conveyor/sessions |
| Curator clap vs `refuse:curator` | green | wrong curator refuses on import/accept/apply-import; accept missing flag is clap |
| Tampered SKU `host_class` on leases | green | slim-parse `refuse:bad-host-class`; aliases still round-trip |
| Call / status / leases / reconcile on SKU `host_class` | green | `refuse:bad-host-class`; no rewrite to `any`; record writes nothing |
| Tampered mesh / restore SKU `host_class` | green | call/list/expire/sync and restore `refuse:bad-host-class`; no write |
| Leases / status refuse before print | green | SKU actual is not dumped, then refused |
| Backup / restore estate parse | green | present-but-garbage estate file refuses; no locked-only invent |
| Feed redaction report | green | pack + `redaction.json` never store the raw secret |
| Journals append-only | green | suspend / resume / `expire --forget` do not truncate jsonl |
| Floor src SKU needles | green | `doctor --strict` floor scan stays clean (`not-a-host` in src tests) |
| Garbage `lifecycle.json` | green | suspend / resume / apply refuse; no overwrite; apply writes no leases |
| Import audit fail-closed | green | accepted pack write does not swallow `import-audit.jsonl` |
| Apply `--import-pack` curator | green | live and `--dry-run` refuse before any lease / pack / audit write |
| Catalog / probes print | green | catalog writes first; probes refuse every card first |
| Garbage plan JSON | green | `covering_plan` / `latest_plan` / `list_plans` refuse; apply / status write nothing |
| Feed-cursor load/write | green | present garbage refuses; write does not invent empty `feed-cursor.json` |
| Apply `--force` SKU actual | green | `rtx-consumer.yaml` restamps `consumer-nvidia`; no SKU→`any` launder |
| Propose / append / placements serialize | green | serialize-then-write refuses empty; no `"{}"` journal junk |
| model-actual / session.json serialize | green | refuse empty blob; garbage model-actual is not greenfield |
| Estate compare-read | green | accept / import / propose refuse unreadable estate bytes |
| Live probe runbook + dry shapes | green | [`LIVE-PROBES.md`](LIVE-PROBES.md); SKIP vs would-live fixture, no network |
| OpenAI / Ollama live adapter | green | GET `/v1/models` or `/api/tags`; in-process mock; SKIP without env |
| Specialist chat round-trip | green | `HttpLocal` posts request text; `model-estate specialist`; llama.cpp OpenAI smoke; mock HTTP |
| Overlay omit-locked KEEP (property) | green | `locked: []` / overlay collision cannot drop hardcoded ids |
| Two dry-runs identical `.cell` | green | after apply, two `--dry-run` leave path+bytes unchanged |
| Dual-layer backup → restore | green | matching sacred writes leases back; dry-run restore writes nothing |
| Makefile contract | green | `gate-90` / `smoke` / `day90` / `feed-loop` / `fixtures-check` / `doctor-strict`; no `gh` |

## Remaining Day-90+ (honest; parked, not green)

| Item | State |
| --- | --- |
| Live Mac MLX | Parked. See [`DAY90-PLUS.md`](DAY90-PLUS.md) + [`LIVE-PROBES.md`](LIVE-PROBES.md). Probe path exists. No Mac in CI. |
| Live rented / consumer GPU | Parked. Same specialist protocol. Not required in CI. |
| Cloud-agent spawn | Parked / locked off. Declared only. Floor does not spawn. |
| `estate reconcile --suggest` | Patch file only. Jason still applies by hand. Not an auto-heal. |
| `estate packs accept` | Writes enrich_packs **edit instructions**. Does not rewrite `estate.yaml`. Needs `--curator jason`. |
| Convey hop transport | Lease-bound mesh, not a gateway. |
| Auto-promote / curator UI | Locked off / not built. |
| vLLM / TRT | Experimental catalog cards until Jason verifies. |
| Actions | One compile-only job forever unless Jason expands it. |

Fail closed: sacred exclusions, SKU in ids (including probe ids), unknown apiVersion/kind, missing local (no frontier fallback), cloud-agent spawn, auto-promote, Sanctum-as-Cyera bleed, omit-locked sacred file.

Live Grok / GPU are not required to keep `make smoke` / `make gate-90` green.
