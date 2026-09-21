# Overnight decisions (Day 61–90 beachhead)

Jason was asleep. These are the seams we picked for **maximum flexibility**. Everything can change later. Not a product freeze.

## What we shipped

1. **Feed → pack format, no auto-promote.** Scrubbed `events.jsonl` (frontier + local + proxy) materializes to `examples/enrich-packs/drop/{id}.pack.json`. Curator stays `jason`, policy stays `manual`, `promoted` is always false. `estate feed promote` exists only to fail closed.
2. **Suspend / resume as the operator lifecycle.** `estate suspend` discards sessions/PIDs and writes durable `.cell/lifecycle.json`. `estate resume` re-applies from the estate file. Pause kit calls those commands. Lifecycle is *not* SoT for desired-state; the estate file is.
3. **Plan is the human control surface.** `estate plan` writes markdown + JSON plus a `Reviewable diff` block and regenerates `plans/INDEX.md`. `estate plans` lists history. Generated plans stay gitignored; commit a specific `.md` into a PR when Jason should review apply.
4. **Drivers stay swappable.** Ollama / llama.cpp / http-remote bind the same specialist protocol via `CELL_LOCAL_ENDPOINT`. MLX stays a stub. `host_class` is validated. Wrong host on a card fail-closes at bind (no frontier fallback).
5. **Cloud-agent placement is declared, not wired.** `placements[]` on the estate: `box` (this Cell One) and `cloud-agent` (`cursor-cloud`, `wired: false`, no agents). Floor still binds one session per estate agent. Day-90 operator day can assign agents without a schema break.

## Assumptions (safe to reopen)

| Assumption | Why | Revisit |
| --- | --- | --- |
| One overnight branch, not stacked PRs | Less CI, one review surface | Split later if noisy |
| `lifecycle.json` lives under `.cell/` and is durable | Operator intent survives restart; still not estate SoT | Move next to `plans/` if Jason wants it committed |
| Plans are local artifacts unless force-added | Avoid noisy generated diffs | Track `plans/reviewed/` if that becomes the ritual |
| Cloud-agent `wired:true` with empty agents is invalid | Stops a half-spawn | Allow when a real spawn driver exists |
| Pack ids are slugs Jason chooses | Drop zone is a file convention, not a registry | Add a pack schema crate later |
| No hosted `make gate-90` | Cost lock | Keep thin PR-only `cargo test` |
| A10–A12 here means *beachhead*, not a full workday | Progress > polish | Next cell: assign cloud agents, curator UI is still forbidden |

## Anti-shrink (still)

This is not a gateway, not LM Studio, not an auto-promote feed, not a cloud-agent farm. Cloud-agent is a **declared placement**, same class as `box`, behind the estate file.

## Origin

Zero Origin remotes or URLs. SoT is https://github.com/jwgale/agent-estate-factory
