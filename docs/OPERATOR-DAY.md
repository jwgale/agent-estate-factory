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
`packs accept --curator jason`. The locked estate is deny-default
(`intentions: []`), so the pack lists `drivers=-` and does not invent
`source_drivers` `frontier` or `local`. Feed cursor stays on disk.
`estate.yaml` cksum is unchanged. Promote stays refused. No live keys.

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

## 3b. Agent pack list / orchestrator handoff (fixture)

Opt-in. Not part of `make smoke` or `make gate-90`. Uses throwaway
[`../examples/fixtures/agent-pack-handoff.yaml`](../examples/fixtures/agent-pack-handoff.yaml).
Does not touch locked `examples/estate.yaml` (cksum `43770130 3391`).

```bash
estate pack list --estate examples/fixtures/agent-pack-handoff.yaml
estate pack show --id research-crew --estate examples/fixtures/agent-pack-handoff.yaml
estate complete --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-handoff-cell --agent horizon --pack research-crew \
  --prompt "ping" --mock
estate decisions report --state-dir target/pack-handoff-cell --pack research-crew
```

`--agent` is the pack orchestrator. Complete hands off to the other member
(`research`) and journals `pack_id`, `handoff_from`, `handoff_to` on the
`surface=complete` receipt. `--mock` stays in-process. No live key. No
group-chat runtime. Opt-in mixed select without `--pack` is
`--select equal-class` or agent `select: equal-class`: one complete
turn may consider specialty + frontier and journals the chosen binding
plus rejected peers. Default disjoint allow-lists still abstain. That
mirrors waking the specialist then the frontier peer. Not a Grok Bot
chat UI.

## 3e. Purpose-seat sidecar + equal-class mixed select (fixture)

Opt-in. Not part of `make smoke` or `make gate-90`. Throwaway only.
Does not touch locked `examples/estate.yaml` (cksum `43770130 3391`).
`READY_FOR_LIVE_TEST`: no.

Classify journey `--print` writes `purpose-seat.json` beside `--prepared`
(the import dir) and names the live seat on stdout. `--run` also writes
`{out}/purpose-seat.json`. Enrich prepare writes the sidecar when the
seat tag is already `specialist-*` / `classify-*`. Import reads it so
throwaway AG News auto-bind needs no hand copy.

```bash
# classify journey --print --dataset ag_news --train-size all --prepared $PREPARED
# $PREPARED/purpose-seat.json holds {"purpose_seat":"specialist-agnews-all"}
# Keep that file next to prepare.json; import-trained consumes it.

estate complete --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/mixed-select-cell --agent research \
  --select equal-class --prompt "ping" --mock
```

On a throwaway estate whose research allow-list names `ag_news` and
`xai_grok` (or `frontier_http`), that flag chooses the specialty seat
and the receipt lists the frontier peer under `rejected`. Horizon on
the locked example (disjoint `xai_grok` + `local_slm`, no policy)
still abstains. Not live Grok Bot sync.

## 3c. Pack packages + standing routines (fixture)

Opt-in. Not part of `make smoke` or `make gate-90`. Same throwaway
[`../examples/fixtures/agent-pack-handoff.yaml`](../examples/fixtures/agent-pack-handoff.yaml)
(now also declares `pack_packages` + `routines`). Does not touch locked
`examples/estate.yaml` (cksum `43770130 3391`). `READY_FOR_LIVE_TEST`: no.

```bash
estate package list --estate examples/fixtures/agent-pack-handoff.yaml
estate package show --id classify-ping --estate examples/fixtures/agent-pack-handoff.yaml
estate package run --id classify-ping --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-packages-cell --mock
estate routine list --estate examples/fixtures/agent-pack-handoff.yaml
estate routine run --id standing-classify --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-routines-cell --mock
estate decisions report --state-dir target/pack-routines-cell
```

`package run` defaults `--agent` to the pack orchestrator, runs
`complete --pack`, and journals `package_id` (plus pack handoff fields).
`routine run` resolves the declared package and also stamps `routine_id`.
Authorize still uses existing model intentions. Plugin scaffold is
`estate pack export-plugin` (MCP wired to `estate complete`; `live_sync`
stays false). No live Cursor / Grok Bot routine sync.
No multi-step DAG.

## 3d. Scheduled tick + multi-hop chain (fixture)

Same throwaway fixture. Optional `schedule` on `standing-classify` (`@hourly`).
`estate routine tick` runs it when due and writes `{state-dir}/routine-state.json`
(`last_run` / `next_due`). A second tick in the same hour is a skip (idempotent).
`estate package run --chain` wakes specialty then frontier and journals one
`chain_id` plus ordered `handoffs` (`handoff_from` / `handoff_to` / `binding`).
That is the Grok Bot shape — a routine waking a group skill — without claiming
live Grok Bot sync. `--mock` stays in-process. Locked `examples/estate.yaml`
untouched. `READY_FOR_LIVE_TEST`: no.

```bash
estate routine status --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-routines-cell
estate routine tick --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-routines-cell --mock
estate routine digest --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-routines-cell
estate routine tick --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-routines-cell --mock --report
estate package run --id classify-ping --chain \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-chain-cell --mock
estate decisions report --state-dir target/pack-chain-cell
```

`digest` and `tick --report` glance at the last local wake: ran/skipped,
package/chain ids, receipt ids, and `completion_label` when a receipt has
one. They read `{state-dir}/routine-state.json` plus the decision journal.
They do not sync to Grok Bot.

## 3f. Pack export-plugin scaffold (fixture)

Opt-in. Not part of `make smoke` or `make gate-90`. Same throwaway
[`../examples/fixtures/agent-pack-handoff.yaml`](../examples/fixtures/agent-pack-handoff.yaml).
Does not touch locked `examples/estate.yaml` (cksum `43770130 3391`).
`READY_FOR_LIVE_TEST`: no.

```bash
estate pack export-plugin --id research-crew \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --out target/pack-plugin-stub
```

Writes an Agent Plugin stub Cursor can load: `plugin.json` (pack → group),
`mcp.json` (one stdio server per member: `estate pack mcp-serve`),
`skills/<package>/SKILL.md` (package → skill body stub), and commented
cron/trigger notes for `standing-classify` (`@hourly` → `0 * * * *`).
Each member tool runs `estate complete --agent <member> --pack research-crew`
against the source estate. Env carries pack / member / role / estate path.
Non-orchestrator members refuse `refuse:pack-orchestrator` the same as
pack complete. `wired_mcp: true`. `live_sync: false`. `estate` must be
on PATH. This is a bridge, not a live Cursor or Grok Bot install. Routines
stay comments. Not a cron daemon.

`estate pack mcp-serve` is stdio MCP — it waits for JSON-RPC frames and
is not an interactive complete. Inspect `target/pack-plugin-stub/mcp.json`,
then call tool `complete` with `{ "prompt": "ping", "mock": true }`.
That writes a decision receipt on the source estate state-dir. Cargo
test `pack_mcp_serve` covers the mock path. Not a live PASS.

## 4. Enrich prepare (opt-in, not a train)

Not part of `make smoke` or `make gate-90`.

```bash
estate help enrich
make enrich-prepare
```

`make enrich-prepare` uses `examples/fixtures/specialist-overnight.pack.json` and writes both drivers under `/tmp/cell-one-enrich-prepare` (or `$TMPDIR`). The copy of the estate sets `params.model: llama3`, so the Modelfile says `FROM llama3`. You also get `NEXT.md` with the `ollama create` line, a portable manifest, `estate enrich list`, and an `import-prepared` binding proposal. The script then runs `estate enrich apply-proposal`, `estate plan`, and `estate apply --require-plan` on that seated copy. Apply without `--require-plan` leaves the lab copy unchanged. It does not call Ollama. `examples/estate.yaml` stays unchanged. `make enrich-live-prove` is the opt-in that does call `ollama create` when the seat is up. `make train-prepare` writes LLaMA-Factory LoRA and QLoRA recipes and Axolotl LoRA and QLoRA recipes on another throwaway directory and prints `SKIP live train`. It does not run either trainer. `make qlora-journey` prints the Qwen QLoRA ladder (Target C) and checks the `llamafactory-qlora` prepare artifacts. It does not train, convert, or promote. `make lora-journey` prints the Qwen LoRA ladder (Target A) and checks the `llamafactory-lora` prepare artifacts. It does not train, merge, convert, or promote. `make seat-journey` prints the Target C merge, convert, seat, and import lines after fixture stubs stand in for the merged export and the GGUF. It does not train, convert, or create a model. When an SLM fits mid-software-build, or on demand, the same print-only entry is `make purpose-build-journey`. Print-only purpose-build on demand. It runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. It does not train, convert, seat, promote, or apply. It is not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no. The re-prove card stays `make uniqueness-prove-checklist`. The recorded Target C PASS in [`LIVE-PROBES.md`](LIVE-PROBES.md) stays the only live uniqueness prove. Prepare: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md) sections 15–20.

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
