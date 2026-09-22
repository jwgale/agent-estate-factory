# Operator day

A walk Jason can run on this box after Day 90. The numbered steps are fixtures.
They do not prove a Mac, a GPU, or a cloud spawn. Those rows stay parked in
[`DAY90-PLUS.md`](DAY90-PLUS.md). The live-box ladder after `make gate-90`
is `make real-world` (below). Live paste target:
[`LIVE-PROBES.md`](LIVE-PROBES.md). Product story:
[`NORTH-STAR.md`](NORTH-STAR.md). README start-here: [`../README.md`](../README.md).
Feed-only walk: [`FEED-LOOP.md`](FEED-LOOP.md).
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

## Live-box ladder (after gate-90): `make real-world`

Opt-in. Not part of `make smoke`, `make gate-90`, or GitHub Actions.

```bash
make real-world
```

Prints the north-star one-liner, runs `cargo check --workspace --locked`
(same as `make check`), then vanilla `estate doctor` on this checkout
(`examples/estate.yaml` lives here). If `CELL_LOCAL_ENDPOINT` is unset,
live probes and the Ollama specialist print SKIP and the script exits 0.
That SKIP is not a PASS. If the endpoint is set, it runs
`estate probes --live` and
`estate specialist --driver ollama --prompt "Reply with the single word pong."`
with the env already in the shell (same names as
[`LIVE-PROBES.md`](LIVE-PROBES.md)). It does not print the frontier API
key. It does not invent a completion. The MacBook Air `Pong` is already
recorded on the live-probes page.

## 2. `make feed-loop`

```bash
make feed-loop
```

Isolated `target/feed-loop-cell`. Scrubbed mock traces → pack → propose →
`packs accept --curator jason`. The produced pack must tag `source_drivers`
`frontier` then `local`. Feed cursor stays on disk. `estate.yaml` cksum is
unchanged. Promote stays refused. No live keys.

Not part of `make smoke`. See [`FEED-LOOP.md`](FEED-LOOP.md).

## 3. Opt-in frontier help and mixed walk

Not part of `make smoke` or `make gate-90`.

```bash
estate help frontier
estate help day90-mixed
make day90-mixed
```

`estate help frontier` names the grok-4.7 specialist. `XAI_API_KEY` is required. A prompt that mentions Cyera or Rust classroom refuses before any POST, the same way a local specialist does. Local down does not call frontier.

`make day90-mixed` walks `examples/fixtures/mixed-frontier-local.yaml` on an isolated cell: status, plan, apply with a plan, status, doctor. It then validates `examples/hosts/frontier-http.yaml` and prints status. Both name `model: grok-4.7` on the frontier `http-remote` binding. The host file is not applied and is not a host-class alias. It is not on `make smoke` or fixtures-check. `examples/estate.yaml` stays hash-locked and does not invent a binding model. No live key. A sacred prompt still refuses when `CELL_FRONTIER_MODEL` is a hardware SKU; the SKU model path is not the refusal.

## 4. Enrich prepare (opt-in, not a train)

Not part of `make smoke` or `make gate-90`.

```bash
estate help enrich
make enrich-prepare
```

`make enrich-prepare` uses `examples/fixtures/specialist-overnight.pack.json` and writes both drivers under `target/enrich-prepare-cell`. You get a Modelfile plus `ollama create` steps, and a portable manifest. The script does not call Ollama and does not change `examples/estate.yaml`. Page: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md).

## 5. Backup rotate

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
