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

## Day 90+ (PR after #2)

- `estate probes --live` / `CELL_LIVE_PROBE=1`: optional HTTP ping. Unset endpoints print SKIP. CI does not require a Mac or a rented GPU. Rented box uses `CELL_LOCAL_ENDPOINT` or `CELL_RENTED_ENDPOINT` (no SKU in the id). MLX uses `CELL_MLX_ENDPOINT`.
- `make day90`: status → plan → dry-run → apply → reconcile, then catalog + live probes.

## Still stubbed

MLX / vLLM / TRT live runtimes (probe path only). Cloud-agent spawn. Convey hop transport (lease-bound mesh only). Auto-promote. Curator UI.

## How to run

```bash
make smoke    # doctor + fixtures-check + operator-day + cargo test + make day90
make day90    # operator loop only
```
