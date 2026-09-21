# Operator day (no live boxes)

A walk Jason can run on this box after Day 90. Fixtures only. It does not
prove a Mac, a GPU, or a cloud spawn. Those rows stay parked in
[`DAY90-PLUS.md`](DAY90-PLUS.md). Live probe hand-off (not green):
[`LIVE-PROBES.md`](LIVE-PROBES.md). README start-here: [`../README.md`](../README.md).
Feed-only walk (this is step 2): [`FEED-LOOP.md`](FEED-LOOP.md).
`.cell/` paths: [`cell-layout.md`](cell-layout.md).

Hosted CI does **not** run this walk. See “Why `make gate-90` stays local”
below. The same isolated loop on `examples/fixtures/dual-layer-demo.yaml`
is covered by `crates/estate-control/tests/day90_e2e.rs`. Snapshot:
[`CELL-ONE-STATUS.md`](CELL-ONE-STATUS.md).

## 1. `make gate-90`

```bash
make gate-90
```

Runs `make smoke` (doctor + fixtures-check + operator-day +
`cargo test --workspace` + `make day90`), then `estate doctor --strict`,
then prints the GATE-90 checklist. Isolated cells under `target/`. Does not
spawn `cursor-cloud`. Does not auto-promote packs.

Expect `GATE-90 GREEN (local only)`. Live probes print SKIP when `CELL_*`
is unset.

## 2. `make feed-loop`

```bash
make feed-loop
```

Isolated `target/feed-loop-cell`. Scrubbed mock traces → pack → propose →
`packs accept --curator jason`. The produced pack must tag `source_drivers`
`frontier` then `local`. Feed cursor stays on disk. `estate.yaml` cksum is
unchanged. Promote stays refused. No live keys.

Not part of `make smoke`. See [`FEED-LOOP.md`](FEED-LOOP.md).

## 3. Backup rotate

Use an isolated cell so the walk does not touch a real `.cell/`.

```bash
ROOT=target/op-run
rm -rf "$ROOT"
mkdir -p "$ROOT"

cargo run -q -p estate-control -- apply \
  --estate examples/estate.yaml \
  --state-dir "$ROOT/state" \
  --roots-base "$ROOT" \
  --plans-dir "$ROOT/plans"

cargo run -q -p estate-control -- backup \
  --estate examples/estate.yaml \
  --state-dir "$ROOT/state" \
  --plans-dir "$ROOT/plans" \
  --out "$ROOT/backups"

cargo run -q -p estate-control -- backup \
  --prune 1 \
  --estate examples/estate.yaml \
  --state-dir "$ROOT/state" \
  --plans-dir "$ROOT/plans" \
  --out "$ROOT/backups"
```

`--prune 1` keeps the newest archive and deletes the rest. `--prune 0`
refuses. Restore is fail-closed on sacred mismatch. Nothing is uploaded.

What each file under `.cell/` means: [`cell-layout.md`](cell-layout.md).

## Why `make gate-90` stays local

`make gate-90` → `scripts/day90-gate.sh` → `scripts/smoke.sh`, which runs
`cargo test --workspace`. That is the real gate. It is minutes of compile +
tests on a laptop, not a 10-minute compile-only Actions job.

Jason locked hosted CI after Sanctum CI cost: one `pull_request` job,
`cargo check --workspace --locked`, rustc 1.88, timeout ≤ 10. No
`cargo test`, no matrix, no push-to-main. Putting `make gate-90` on Actions
would break that lock.

So: run `make gate-90` here. Hosted CI only proves the workspace still
compiles. `estate doctor --strict` checks the workflow body stays
compile-only. This document is not a green live-box report.
