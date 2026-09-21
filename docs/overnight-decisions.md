# Overnight decisions (Day 61–90 beachhead)

Jason was asleep. These are the seams we picked for **maximum flexibility**. Everything can change later. Not a product freeze.

## What we shipped

1. **Feed → pack format, no auto-promote.** Scrubbed `events.jsonl` (frontier + local + proxy) materializes to `packs/{id}.pack.json` (`schema=cell-one.pack.v0`). Curator stays `jason`, policy stays `manual`, `promoted` is always false. `estate feed import` is explicit apply (Feed→Control) and does not rewrite the estate. `estate feed promote` exists only to fail closed. Notion beachhead already exists (do not recreate): https://app.notion.com/p/3e2b09f5050681c79063ca36c0affd87
2. **Suspend / resume as the operator lifecycle.** `estate suspend` discards sessions/PIDs and writes durable `.cell/lifecycle.json`. `estate resume` re-applies from the estate file. Pause kit calls those commands. Lifecycle is *not* SoT for desired-state; the estate file is.
3. **Plan is the human control surface.** `estate plan` writes markdown + JSON plus a `Reviewable diff` block and regenerates `plans/INDEX.md`. `estate apply --require-plan` is gated on a covering plan hash and writes `plans/apply-*.json` plus `.cell/apply-audit.jsonl`. `estate plans` lists history. Generated plans stay gitignored; commit a specific `.md` into a PR when Jason should review apply.
4. **Drivers stay swappable.** Ollama / llama.cpp / http-remote bind the same specialist protocol via `CELL_LOCAL_ENDPOINT`. MLX stays a stub. `host_class` is validated. Wrong host on a card fail-closes at bind (no frontier fallback).
5. **Cloud-agent placement is declared, not wired.** `placements[]` on the estate: `box` (this Cell One) and `cloud-agent` (`cursor-cloud`, `wired: false`, no agents). Apply/resume write `.cell/placement-actual.json` leases (`box` spawned, cloud-agent not). Floor still binds one session per estate agent. Day-90 operator day can assign agents without a schema break.

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

## Anti-shrink (still)

This is not a gateway, not LM Studio, not an auto-promote feed, not a cloud-agent farm. Cloud-agent is a **declared placement**, same class as `box`, behind the estate file.

## CI (overnight quiet)

Jason’s inbox was filling with Actions failure mail. Until ~7am America/Chicago:

- No GitHub Actions workflows. `ci.yml` is gone on `main` and on `cursor/day61-90-beachhead-2950`.
- Do not add a workflow file overnight. Do not open extra PRs that would retrigger CI.
- Gate is local only: `cargo check --workspace --locked`, `cargo test --workspace`, `make gate` / `make gate-60` / `make gate-90`.
- Tomorrow, if Jason wants hosted CI back: one `pull_request` job, `cargo check --workspace --locked` only, timeout ≤ 10. Never `cargo test` on Actions overnight.

## Origin

Zero Origin remotes or URLs. SoT is https://github.com/jwgale/agent-estate-factory
