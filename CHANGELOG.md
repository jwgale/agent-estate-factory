# Changelog

Local wrap: `make smoke`. Hosted CI is compile-only (`cargo check --workspace --locked` on pull_request). Day 61–90 is on `main` via PR **#2**.

## Day 0–30 (on `main` via PR #1)

One-box factory proving A1–A4. Desired-state `estate.yaml` (Horizon / Research / Sanctum), deny-default intentions, sacred exclusions (Cyera CI, Rust classroom), isolation as a driver (profile-dir). No live provider required. Stubs: Dual PE, vault, multi-box, AI-gateway — out of altitude.

## Day 31–60 (on `main`)

Mixed model estate A7–A9. Equal-class frontier + local. Ollama-first catalog, llama.cpp swap-proof, MLX/vLLM/TRT stub/experimental. Fail-closed when local is down (no silent frontier fallback). Portable `host_class` (consumer-nvidia / apple-silicon / rented-nvidia / any). Hardware is a driver choice, not a product fork. GitHub is source of truth. Origin stripped.

## Day 61–90 (on `main` via PR #2)

Beachhead toward A10–A12. Cloud-agent is **declared, not spawned**. Feed packs never auto-promote. Curator is Jason / manual.

### Wave 1 — A10–A12 beachhead

Feed drop zone + explicit import. Suspend / resume + durable `lifecycle.json`. Plan / gated apply / apply audit. Driver stubs. `docs/overnight-decisions.md`.

### Wave 2 — conveyors, Security-as-IaC, packs, operator-day

Lease-bound convey hops. Plan blast-radius markdown. Pack list/import/refuse-promote. `make operator-day`. Host-class aliases.

### Wave 3 — reconcile, propose, multi-host, audit export

Desired-vs-actual reconcile (report, not a fixer). Enrich propose never applies. Multi-host fixture. Local audit export bundle.

### Wave 4 — dry-run, lease TTL, doctor

`estate apply --dry-run` writes nothing. Lease expiry refuse. Specialist pack metadata. `estate doctor`.

### Wave 5 — journal, hop TTL, plan diff, fixtures, apiVersion

Session journal. Convey hop TTL. `estate plan diff` (`refuse:wider`). Fixture library + `make fixtures-check`. Optional `apiVersion` / `kind`; unknown fail-closed.

### Wave 6 — backup, policy, catalog caps, pause-proof, smoke

`estate backup` / `restore` (sacred-mismatch refuse). Policy pack stub. Catalog capability flags. Pause-kit proof. `make smoke`.

### Wave 7 — status, curator gate, convey sync, schema freeze

`estate status` one-pager. Import `--curator jason`. Convey sync from placements (sacred refuse; extra hops stay). `schema/README.md`. `docs/GATE-90.md`.

### Wave 8 — idempotent apply, plan PR artifact, sacred file, mixed proof

- Second apply with identical desired state is a no-op (`unchanged` audit). Drift refuses unless `--force`.
- `estate plan export-pr` writes one markdown to paste into a GitHub PR body.
- `policy/sacred.yaml` overlays on hardcoded sacred ids. Apply / convey / cloud refuse on hit.
- Mixed fixture: http-remote frontier + local ollama; validate + dry-run, no live calls.

## Day 90+ (PR #3 on `main`)

- `estate probes --live` / `CELL_LIVE_PROBE=1`: optional HTTP ping. Unset endpoints print SKIP. CI does not require a Mac or a rented GPU. Rented box uses `CELL_LOCAL_ENDPOINT` or `CELL_RENTED_ENDPOINT` (no SKU in the id). MLX uses `CELL_MLX_ENDPOINT`.
- `make day90`: status → plan → dry-run → apply → reconcile, then catalog + live probes.

## After PR #3

- Probe env vars documented in README. `probes --live` SKIP fixture. Probe ids refuse SKUs.
- `estate reconcile --suggest` writes a patch file only (never auto-apply).
- `estate packs accept --curator jason` writes enrich_packs edit instructions (no estate rewrite).

## After PR #4

- `estate doctor --strict`: pre-merge operator checks (compile-only CI body, locked sacred file, dual-layer demo, refuse fixtures, floor no-vendor, hash lock).
- `examples/fixtures/dual-layer-demo.yaml`: dual-path + dual-layer sacred demo. Sanctum is not Cyera. Not a Dual PE product.
- Refuse fixtures: Sanctum-as-Cyera display-name bleed; omit-locked sacred file still refuses Cyera CI as an agent.
- `make gate-90` is a thin local alias: smoke (includes day90) + `doctor --strict` + GATE-90 checklist print.

## After PR #5

- README: `make gate-90` is the Day-90 operator entrypoint. Live Mac / GPU / cloud-spawn wait in `docs/DAY90-PLUS.md` — parking lot, not fake progress.
- `make feed-loop`: fixture walk of scrubbed trace → pack → propose → accept. Cursor stays on disk; rematerialize does not auto-promote; `estate.yaml` is unchanged.
- Placement-actual schema round-trip tests for every reconcile refuse code still thin (`missing-lease`, `extra-lease`, `kind-mismatch`, `host-class-mismatch`, `cloud-spawned`, `sacred-id`, `expired`).
- `estate doctor --strict` now requires `DAY90-PLUS.md` + the feed-loop fixture walk.

## Still stubbed

MLX / vLLM / TRT live runtimes (probe path only). Cloud-agent spawn. Convey hop transport (lease-bound mesh only). Auto-promote. Curator UI. See `docs/DAY90-PLUS.md`.

## How to run

```bash
make gate-90    # Day-90 operator entrypoint (local)
make smoke      # doctor + fixtures-check + operator-day + cargo test + make day90
make day90      # operator loop only
make feed-loop  # scrubbed trace → pack → propose → accept (fixtures only)
```
