# Changelog

Local wrap: `make smoke`. Hosted CI is compile-only (`cargo check --workspace --locked` on pull_request). Day 0–90 is on `main`.

## Day 0–30 (PR #1)

One-box factory proving A1–A4. Horizon / Research / Sanctum on separate lanes. Sacred exclusions (Cyera CI, Rust classroom) fail closed. Isolation is a driver (profile-dir today). No live provider required. Dual PE, vault, multi-box, and AI-gateway stay out of altitude.

## Day 31–60 (on `main` with #1)

Mixed model estate A7–A9. Equal-class frontier + local. Ollama-first, llama.cpp swap-proof. Fail-closed when local is down — no silent Grok fallback. Hardware is a driver (`consumer-nvidia` / `apple-silicon` / `rented-nvidia`), not a product fork. GitHub is the only source of truth.

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

## After PR #7 (this slice)

- `docs/cell-layout.md` matches the paths the code writes, including `reconcile-suggest.md`, `expired`, backups outside `.cell/`, and enrich-edit accept files.
- `docs/OPERATOR-DAY.md` walks `make gate-90` → `make feed-loop` → `estate backup --prune` on isolated cells. No live boxes.
- Why `make gate-90` stays off Actions: it wraps `cargo test --workspace`. Hosted CI stays compile-only. A hardening test locks that contract.
- Scan: no `TODO`/`FIXME` in crate sources, no leftover Origin URLs.
- Workspace crates are already `0.1.0`. That is a crate version, not a crates.io release. No new `estate version` command (clap `--version` is enough).

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
```
