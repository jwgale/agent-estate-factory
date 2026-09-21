# PR #2 — paste into GitHub if the body is stale

**Title:** Cell One Day 61–90 beachhead (toward A10–A12)

**Branch:** `cursor/day61-90-beachhead-2950` → `main`  
**Do not merge unless Jason asks.** Actions are off. Local gate only.

## Summary

Day 61–90 factory beachhead on the same Cell One estate. Not a gateway or studio. Cloud-agent is declared, not spawned. Feed never auto-promotes. Curator is Jason / manual.

Waves 1–8 (same PR): A10–A12 feed/lifecycle/plan; convey mesh + Security-as-IaC; reconcile/propose/audit export; dry-run + lease TTL + doctor; session journal + plan diff + fixtures; backup/policy/catalog/pause-proof/`make smoke`; status + curator gate + convey sync; idempotent apply + `plan export-pr` + sacred file + mixed proof.

Wave 9: refuse-code tests, locked example-estate hash, morning brief. No new CLI.

## Test plan (local only)

```bash
make smoke    # doctor + fixtures-check + operator-day + cargo test --workspace
```

Optional pieces: `make gate-90`, `make fixtures-check`, `make operator-day`.

Hosted CI is **disabled** (no `.github/workflows/*.yml`). Do not add a workflow on this PR. If Jason wants CI later: one `pull_request` job, `cargo check --workspace --locked`, timeout ≤ 10.

## Risk notes

- **Cloud-agent** stays unspawned. A spawned lease fail-closes.
- **Apply after suspend** is `refuse:drift` unless `--force` (sessions discarded). `estate resume` is the pause path.
- **`policy/sacred.yaml`** adds overlay `lab-notebook`. Locked Cyera CI / Rust classroom unchanged.
- **`examples/estate.yaml` hash** is locked (`sha256:dcd7164f…`). Import/propose must not rewrite it.
- **No Origin.** GitHub is SoT.
- **Cost:** do not turn Actions back on as `cargo test`.

Assumptions: `docs/overnight-decisions.md`. Morning read: `docs/MORNING-BRIEF-2026-09-21.md`. Gate map: `docs/GATE-90.md`.
