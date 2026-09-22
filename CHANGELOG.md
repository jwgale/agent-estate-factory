# Changelog

Local wrap: `make smoke`. Hosted CI is compile-only (`cargo check --workspace --locked` on pull_request). Day 0–90 is on `main`.

## After PR #75 (this slice)

- `estate suspend` and `floor suspend` refuse before they drop sessions or rewrite leases when a cloud-agent lease is spawned. They do not restamp that lease to unspawned.
- A missing placement file is not a spawned lease. A wired box lease still drops `spawned` on suspend.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #74 (this slice)

- `estate pause-proof` does not print the pause-proof JSON when the cell is drifted. That drift is `pause-proof: drift (fail closed)` with the drift notes.
- The clean note stays on a proof that is in sync. A spawned cloud lease and a lost lease count already refuse before that JSON.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.
