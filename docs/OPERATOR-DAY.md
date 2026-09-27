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
Authorize still uses existing model intentions. Plugin export is
`estate pack export-plugin` (MCP wired to `estate complete`; skill
bodies call tool `complete` with the package prompt; `live_sync`
stays false). Human gate before Cursor install is
`estate pack plugin-prove`. `estate pack plugin-install-local` copies
a proved export into `~/.cursor/plugins/local/<name>` (see 3h2). That
copy does not claim Cursor Customize loaded the plugin. No live Cursor / Grok Bot routine
sync.
No multi-step DAG.

## 3d. Scheduled tick + multi-hop chain (fixture)

Same throwaway fixture. Optional `schedule` on `standing-classify` (`@hourly`).
`estate routine tick` runs it when due and writes `{state-dir}/routine-state.json`
(`last_run` / `next_due`, plus a bound `session_id` when the package
chain has two or more hops). A second tick in the same hour is a skip (idempotent).
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
package/chain ids, receipt ids, `completion_label` when a receipt has
one, and `session_id` / `context=applied|none` when a crew session is
bound. They read `{state-dir}/routine-state.json` plus the decision journal.
They do not sync to Grok Bot.

## 3e. Standing routine watch (fixture)

Same throwaway fixture. `estate routine watch` is the local operator loop:
it calls the same idempotent `tick` path and prints a digest each cycle.
Default interval is 5m (same as the schedule minimum). Stop with SIGINT
(Ctrl-C) or `--max-cycles`. State stays in `{state-dir}/routine-state.json`.
`--mock` stays in-process. This is not a cloud cron and does not claim
Cursor / Grok Bot install or sync. `live_sync` stays false. Locked
`examples/estate.yaml` untouched. `READY_FOR_LIVE_TEST`: no.

```bash
# One cycle: tick + digest, then quit (5090 live-prove / no 5m wait)
estate routine watch --id standing-classify \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-routines-cell --mock --max-cycles 1

# Short two-cycle hook (tests / local prove; not an operator default)
CELL_ROUTINE_WATCH_INTERVAL_SECS=0 estate routine watch \
  --id standing-classify \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-routines-cell --mock --max-cycles 2
```

`--interval 1m` refuses (`refuse:watch-interval`). The env short interval
is a test hook and requires `--max-cycles`. Standing operators use the
5m default and Ctrl-C.

## 3e2. Supervised routine runner (fixture)

Same throwaway fixture. `estate routine runner` is the detached host
supervisor for that same watch loop: start / stop / status / restart.
The invoking terminal can exit. One child / one pidfile still — not N
daemons. Repeat `--id` or comma-separate (`--id a,b`) to name the
standing set; empty select stays all enabled. Named ids are
all-or-nothing: missing / disabled / invalid refuses before spawn
(`refuse:runner-routine-unknown` / `disabled` / `invalid`) so a
half-configured runner never starts. Status lists `selected:` plus
per-id `last_outcome` after a tick. Digest each cycle covers every
selected routine. Pidfile, status, and the digest log live under
`{state-dir}/routine-runner/` (throwaway-safe, not estate SoT).
Double-start is `refuse:runner-already-running`. Stop of a missing
runner is `refuse:runner-not-running`. Min interval stays 5m.
`--mock` stays in-process. This is not a cloud cron, not a systemd
unit, and does not claim Cursor / Grok Bot install or sync.
`live_sync` stays false. Locked `examples/estate.yaml` untouched.
`READY_FOR_LIVE_TEST`: no.

```bash
# Detach (operator default is 5m; 5090 prove uses the test hook)
CELL_ROUTINE_WATCH_INTERVAL_SECS=2 estate routine runner start \
  --id standing-classify --id standing-once \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-runner-cell --mock --max-cycles 30
# same select: --id standing-classify,standing-once

estate routine runner status --state-dir target/pack-runner-cell
# expect status: running, selected: standing-classify,standing-once,
# then last_digest + last_outcome per id after the first tick

estate routine runner stop --state-dir target/pack-runner-cell
estate routine runner status --state-dir target/pack-runner-cell
# expect status: stopped
```

A second `start` while running refuses `refuse:runner-already-running`.
`restart` stops a live child (if any) then starts. A named set that
includes a missing / disabled / unscheduled id refuses
`refuse:runner-routine-…` and writes no pidfile. When a selected
routine's package has a multi-hop chain, the same tick path auto-binds
a pack-scoped crew session (see 3e3). Fixture `standing-once` is the
single-hop partner for multi-id prove. Prove notes for the 5090 parent:
`.cell/cohesion-agnews-20260926/routine-runner-prove.md` (local
throwaway estate; gitignored). Not a live PASS.

## 3e3. Runner + crew session stitch (fixture)

Same throwaway fixture. `standing-classify` runs package `classify-ping`
(2-hop: research/`ag_news` → horizon/`frontier_http`). The tick path
used by `watch` and `runner` auto-creates a pack-scoped crew session
on the first due wake, persists `session_id` on
`{state-dir}/routine-state.json`, and reuses it on later ticks until
the session is ended, expired, or bound. Hop 1 journals
`context=none`; hop 2 journals `context=applied`. Digest and runner
status cite `session_id=…` and `context=applied|none`. Single-hop
packages stay unchanged. `--mock` stays in-process. Not a live PASS.
`READY_FOR_LIVE_TEST`: no.

```bash
# Detached stitch (test hook; operator default stays 5m)
CELL_ROUTINE_WATCH_INTERVAL_SECS=2 estate routine runner start \
  --id standing-classify \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-runner-session-cell --mock --max-cycles 30

estate routine runner status --state-dir target/pack-runner-session-cell
# expect last_digest … session_id=sess-… context=applied

estate routine digest --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-runner-session-cell
estate routine runner stop --state-dir target/pack-runner-session-cell

# Mock gate: start both standing ids → hop2 context=applied on
# standing-classify → digest/status cover both → reuse →
# ended→fresh → refuse:runner-routine-unknown on a partial set →
# refuse:runner-already-running → stop
estate routine runner-prove \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-runner-prove-cell
```

An ended session on the next due tick mints a fresh id. Explicit
`--session` of an ended / expired / bound id still refuses. No secrets
in the digest log. Locked `examples/estate.yaml` untouched.

## 3e4. Dual-specialty under runner (fixture)

Same throwaway fixture. `standing-dual` (`@hourly`) runs package
`dual-specialty` under the same runner tick + session stitch: hop 1
research/`ag_news` journals `context=none`, hop 2 idiom/`rust_idiom`
journals `context=applied`, hop 3 horizon/`frontier_http` journals
`context=applied`. One `session_id` across hops. Digest cites
`session_id` / `context=applied` plus `package=dual-specialty` and
`chain=chain-dual-specialty-…`. `--mock` stays in-process. Not a live PASS.
`READY_FOR_LIVE_TEST`: no. Locked `examples/estate.yaml` untouched.

```bash
# One due tick (same auto-chain + session bind as the runner)
estate routine tick --id standing-dual \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-dual-runner-cell --mock --report

# Mock gate: standing-dual alone
estate routine runner-prove --dual \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-dual-runner-prove-cell

# Same hops with another standing id on one supervise child
estate routine runner-prove --dual --id standing-once \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-dual-runner-multi-cell
```

`--id standing-dual` is the same prove as `--dual`. Default
`runner-prove` without `--dual` stays `standing-classify,standing-once`.
No LIVE PASS. No live GPU. No Grok Bot sync.

## 3f. Pack export-plugin (fixture)

Opt-in. Not part of `make smoke` or `make gate-90`. Same throwaway
[`../examples/fixtures/agent-pack-handoff.yaml`](../examples/fixtures/agent-pack-handoff.yaml).
Does not touch locked `examples/estate.yaml` (cksum `43770130 3391`).
`READY_FOR_LIVE_TEST`: no.

```bash
estate pack export-plugin --id research-crew \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --out target/pack-plugin
```

Writes an Agent Plugin Cursor can load: `plugin.json` (pack → group),
`mcp.json` (one stdio server per member: absolute `estate` `command` +
`pack mcp-serve` args),
`skills/<package>/SKILL.md` (package → skill body that calls the wired
member MCP tool `complete` with the package prompt), `INSTALL.md`
(literal Cursor smoke steps), `RUNNER.md` (supervised routine runner:
start / stop / status / restart, pidfile under `{state-dir}/routine-runner/`,
refuse codes, `runner-prove` / `--dual`), `SESSION.md` (pack session
create / reuse / end, `context=none` then `context=applied`, dual-specialty
hop example), and commented cron/trigger notes for
`standing-classify` and `standing-once` (`@hourly` → `0 * * * *`).
Each member tool runs `estate complete --agent <member> --pack research-crew`
against the source estate. Env carries pack / member / role / estate path
and `CELL_MCP_COMPLETE_TIMEOUT_SECS` so Cursor-spawned MCP inherits the
same complete cap as CLI prove.
Non-orchestrator members refuse `refuse:pack-orchestrator` the same as
pack complete. Skill bodies are not stubs: mock uses
`{ "prompt": "ping", "mock": true, "object": "ag_news" }`; live omits
`mock` when endpoints/keys are set. `wired_mcp: true`. `live_sync: false`.
Labels say pack plugin / Agent Plugin export — not "pack plugin stub".
Exported `mcp.json` `command` is the absolute `estate` binary resolved
from `current_exe` at export time — Cursor does not need `estate` on
PATH. Export refuses (`refuse:export-estate-bin`) when that binary cannot
be resolved and writes nothing. This is a bridge, not a live Cursor or
Grok Bot install. Routines stay comments. Not a cloud cron daemon.
`estate routine watch` is the local operator loop (see 3e).

`estate pack mcp-serve` is stdio MCP — it waits for JSON-RPC frames and
is not an interactive complete. Inspect `target/pack-plugin/mcp.json`,
then call tool `complete` with `{ "prompt": "ping", "mock": true }`.
That writes a decision receipt on the source estate state-dir. Child
`estate complete` is capped at 120s (`CELL_MCP_COMPLETE_TIMEOUT_SECS` or
`--complete-timeout-secs`); expiry kills the process tree and returns
`refuse:mcp-complete-timeout`. Export refuses (`refuse:export-estate-path`)
when the estate path cannot be canonicalized — it does not write a
relative `CELL_ESTATE_PATH`. Cargo test `pack_mcp_serve` covers the mock
path. Not a live PASS.

## 3g. Crew session / multi-hop pack memory (fixture)

Opt-in. Not part of `make smoke` or `make gate-90`. Same throwaway
[`../examples/fixtures/agent-pack-handoff.yaml`](../examples/fixtures/agent-pack-handoff.yaml).
Does not touch locked `examples/estate.yaml` (cksum `43770130 3391`).
`READY_FOR_LIVE_TEST`: no.

A pack-scoped session holds a short transcript so successive
`estate complete --pack` (or pack MCP `complete`) hops share context
the way a crew holds a thread. Files live under
`{state-dir}/pack-sessions/{pack}/{id}.json`. Not estate SoT. No secrets.

```bash
estate pack session create --pack research-crew \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-session-cell --id sess-crewdemo01

estate complete --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-session-cell --agent horizon --pack research-crew \
  --session sess-crewdemo01 --prompt "unique-hop-alpha-token" --mock

estate complete --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/pack-session-cell --agent horizon --pack research-crew \
  --session sess-crewdemo01 --prompt "follow-up that should see prior turn" --mock

estate pack session show --id sess-crewdemo01 --state-dir target/pack-session-cell
estate decisions report --state-dir target/pack-session-cell --pack research-crew
estate pack session end --id sess-crewdemo01 --state-dir target/pack-session-cell
```

Hop 1 journals `session_id`, `turns=1`, `context=none`. Hop 2 prepends
the prior turn into the specialist prompt (mock completion contains
`unique-hop-alpha-token`) and journals `turns=2`, `context=applied`.
A new session id does not leak the prior transcript. End is
`refuse:session-ended` on resume. TTL expiry is `refuse:session-expired`.
Max turns / bytes (`CELL_PACK_SESSION_MAX_TURNS` default 8,
`CELL_PACK_SESSION_MAX_BYTES` default 16384) refuse with
`refuse:session-bound`. `--session` without `--pack` is
`refuse:session-requires-pack`. MCP tool `complete` accepts `session` /
`session_id` / `session_create`. `live_sync` stays false. Not Grok Bot
server sync. Not a live PASS. Prove notes for the 5090 parent:
`.cell/cohesion-agnews-20260926/crew-session-prove.md`.

## 3h. Pack plugin-prove (human gate)

Same throwaway fixture. This is the human gate before Cursor install.
Does not touch locked `examples/estate.yaml` (cksum `43770130 3391`).
`READY_FOR_LIVE_TEST`: no. Not a live PASS.

```bash
estate pack plugin-prove --id research-crew \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --out target/pack-plugin
```

Default `--id research-crew` and the handoff fixture. Omit `--out` for
a throwaway directory (kept so you can point Cursor at it). The command:

1. Exports the pack plugin (absolute baked `estate` bin, timeout env,
   `INSTALL.md`, `RUNNER.md`, `SESSION.md`).
2. Asserts `mcp.json` `command` is an absolute file — not bare `estate`.
3. Mock MCP `complete` as horizon → expects a decision receipt.
4. Mock MCP `complete` as research → expects `refuse:pack-orchestrator`.
5. Asserts `RUNNER.md` + `SESSION.md` are present and non-thin
   (CLI tokens + refuse codes). Compact report cites `runner_docs: yes`
   / `session_docs: yes`. Missing or thin docs fail the prove.
6. Optionally reports a cheap session two-hop (create + two MCP hops).
   That check does not block the prove.
7. Prints a compact JSON/summary prove report. Exit non-zero on any
   required fail.

Operator runner + session loop on the export is those two files (see
3e2 / 3e3 / 3g). `plugin-prove` does not start a runner. The local
directory install is `estate pack plugin-install-local` (see 3h2). That
command does not claim Cursor Customize loaded the plugin.

`--check-only` re-proves an existing `--out` without rewriting it.
5090 parent live-prove notes: `.cell/cohesion-agnews-20260926/plugin-prove.md`
(local throwaway estate; gitignored). Not live Grok Bot sync. `live_sync`
stays false.

## 3h2. Pack plugin-install-local

Same throwaway fixture. This closes the loop `plugin-prove` gates:
export, prove, copy into Cursor's local plugin folder. Not part of
`make smoke` or `make gate-90`. Does not touch locked
`examples/estate.yaml` (cksum `43770130 3391`). `READY_FOR_LIVE_TEST`:
no. Not a live PASS. `live_sync` stays false.

```bash
estate pack plugin-install-local --id research-crew \
  --estate examples/fixtures/agent-pack-handoff.yaml
```

Default `--id` is `research-crew` and the default estate is the handoff
fixture. Omit `--out` and the export stays in a throwaway directory
(kept). The install path is `$HOME/.cursor/plugins/local/<plugin-name>`
(the plugin.json `name`, kebab-case for this pack). `--target <dir>`
overrides that local-plugins root. Tests and operators who must not
touch a real home set `HOME` (and may set `XDG_CONFIG_HOME` /
`XDG_DATA_HOME` beside it). The command reads `HOME`, not
`XDG_CONFIG_HOME`, because Cursor's folder is `~/.cursor/plugins/local`.

The command:

1. Exports the pack plugin (or reuses `--out` with `--check-only`).
2. Runs the same asserts as `plugin-prove` (absolute estate bin, horizon
   receipt, research `refuse:pack-orchestrator`, non-thin `RUNNER.md` /
   `SESSION.md`). A failed prove does not copy.
3. Validates `plugin.json` against the Agent Plugins 1.0 manifest rules
   when `$schema` is that schema, and `mcp.json` the same way.
4. Copies files into a real directory. A symlink at the install path
   whose target is outside the local-plugins root is
   `refuse:plugin-install-symlink` (still refused with `--force`).
   Symlinks inside the export that point outside the export refuse the
   same way. Prefer the copy.
5. Refuses a directory that has no `.estate-pack-install.json` estate
   marker (`refuse:plugin-install-foreign`) unless `--force`. A marker
   this command wrote reinstalls without `--force`.
6. Writes `.estate-pack-install.json` (`pack_id`, tip, `estate_bin`,
   skills, mcp server ids, `live_sync: false`).
7. If `node` can load `@anysphere/cursor-plugins` `loadUserLocalPlugins`
   (or `CELL_CURSOR_PLUGINS_MODULE`), records the names it returns.
   `loaded: yes` means that list includes this plugin name. If the
   loader is not available, the report says
   `loaded: skipped:loader-unavailable`. That does not claim Cursor Customize loaded the plugin
   and not an IDE reload.

The compact report cites `install_path`, `plugin_name`, `skills`,
`mcp_servers`, `runner_docs`, `session_docs`, and `loaded`. Exit
non-zero when prove fails, the manifest does not validate, the symlink
is outside, or a foreign install would be overwritten. Exported
`INSTALL.md` documents this same loop.

CLI smoke is first-class in that `INSTALL.md` and in the export
`README.md`. It runs successive `estate complete --mock` hops on one
`--session` / same `session_id`: hop 1 as the orchestrator (decision
receipt, outcome allow, `context=none`), hop 2 cites hop 1
(`context=applied`), and a member stays `refuse:pack-orchestrator`.
A Cursor MCP loader hang is out of scope. `estate pack cohesion-prove`
and `estate pack crew-session-prove` run the install under a throwaway
`HOME` and then that multi-hop CLI crew session. `loaded:
skipped:loader-unavailable` is not a live PASS.

```bash
estate pack plugin-install-local --id research-crew \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --out target/pack-plugin \
  --target "$HOME/.cursor/plugins/local"
```

`--check-only` re-proves an existing `--out` and then copies. Not
Marketplace publish. Not Grok Bot server sync. Not systemd / cloud cron.

## 3i. Dual specialty chain (mock prove)

Opt-in. Not part of `make smoke` or `make gate-90`. Same throwaway
[`../examples/fixtures/agent-pack-handoff.yaml`](../examples/fixtures/agent-pack-handoff.yaml)
now also seats `rust_idiom` beside `ag_news`. Does not touch locked
`examples/estate.yaml` (cksum `43770130 3391`). `READY_FOR_LIVE_TEST`: no.
Not a live PASS. No network. No Ollama.

Pack `dual-specialty` members: horizon (orchestrator), research, idiom.
Package `dual-specialty` chain:

1. research → `ag_news` (`params.model` `specialist-agnews-all`)
2. idiom → `rust_idiom` (`params.model` `specialist-rustidiom-all`)
3. horizon → `frontier_http`

Each hop allow-list names one binding, so the selector records that seat
without `--select equal-class`. Two specialty locals still abstain, with
or without the flag. Sacred exclusions `cyera-ci` and `rust-classroom`
stay out. `completion_label` stays opt-in.

```bash
estate package show --id dual-specialty \
  --estate examples/fixtures/agent-pack-handoff.yaml

estate package run --id dual-specialty --chain \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/dual-specialty-cell --mock --session-create

estate decisions report --state-dir target/dual-specialty-cell \
  --pack dual-specialty

estate package dual-prove \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir target/dual-specialty-prove
```

`dual-prove` is the one-shot gate. It runs the chain under `--mock`,
checks receipts name `ag_news` then `rust_idiom` then `frontier_http`
(same `chain_id`, `handoff_from` / `handoff_to`), checks hop 2
`context=applied`, checks `decisions report --pack` shows both specialty
capabilities, and checks a full session still refuses when bound, expired,
or ended. Exit non-zero on any fail. The summary ends with JSON
`cell-one.dual-specialty-prove.v0`. `live_sync: no`.
`READY_FOR_LIVE_TEST: no`.

Fixture routine `standing-dual` puts that same package under the
runner. See 3e4. `estate routine runner-prove --dual` is the mock
runner gate (hop 1 `context=none`, hops 2–3 `context=applied`, same
`session_id`).

## 4. Enrich prepare (opt-in, not a train)

Not part of `make smoke` or `make gate-90`.

```bash
estate help enrich
make enrich-prepare
```

`make enrich-prepare` uses `examples/fixtures/specialist-overnight.pack.json` and writes both drivers under `/tmp/cell-one-enrich-prepare` (or `$TMPDIR`). The copy of the estate sets `params.model: llama3`, so the Modelfile says `FROM llama3`. You also get `NEXT.md` with the `ollama create` line, a portable manifest, `estate enrich list`, and an `import-prepared` binding proposal. The script then runs `estate enrich apply-proposal`, `estate plan`, and `estate apply --require-plan` on that seated copy. Apply without `--require-plan` leaves the lab copy unchanged. It does not call Ollama. `examples/estate.yaml` stays unchanged. `make enrich-live-prove` is the opt-in that does call `ollama create` when the seat is up. `make train-prepare` writes LLaMA-Factory LoRA and QLoRA recipes and Axolotl LoRA and QLoRA recipes on another throwaway directory and prints `SKIP live train`. It does not run either trainer. `make qlora-journey` prints the Qwen QLoRA ladder (Target C) and checks the `llamafactory-qlora` prepare artifacts. It does not train, convert, or promote. `make lora-journey` prints the Qwen LoRA ladder (Target A) and checks the `llamafactory-lora` prepare artifacts. It does not train, merge, convert, or promote. `make seat-journey` prints the Target C merge, convert, seat, and import lines after fixture stubs stand in for the merged export and the GGUF. It does not train, convert, or create a model. When an SLM fits mid-software-build, or on demand, the same print-only entry is `make purpose-build-journey`. Print-only purpose-build on demand. It runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. It does not train, convert, seat, promote, or apply. It is not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no. The re-prove card stays `make uniqueness-prove-checklist`. The recorded Target C PASS in [`LIVE-PROBES.md`](LIVE-PROBES.md) stays the only live uniqueness prove. Prepare: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md) sections 15–20.

## 4b. Bindable specialty (opt-in, not a train)

`estate enrich import-trained` on a classify-journey GGUF records `trained_shape` gguf and `auto_apply=false`, writes `specialty-join.json` for the prepared binding, and leaves the source estate unchanged. Default without `--binding-id` stays `local_slm`. `--binding-id ag_news` then `--binding-id rust_idiom` records a join for each portable id. `estate enrich standing-next` prints whether that binding is joinable: the GGUF is a regular file and an agent models allow-list names it. The file alone is not joinable. `estate enrich bind-prove --root . --out <throwaway>` runs the `local_slm` loop on a lab copy with a mocked GGUF: import, `apply --require-plan`, Standing next, and a mock complete as `research`. The lab copy records an allow Model intention for research on `local_slm` so that receipt names the seat. `estate enrich bind-prove --dual` lands `ag_news` and `rust_idiom` beside `local_slm` on a lab copy, records scoped allow-lists and Model intentions, prints Standing next `joinable: yes` for each, and leaves mock receipts that name those seats. An allow-list that names both specialty ids still abstains. `make ag-news-journey` prints that opt-in path, including `--binding-id`, and does not invoke the prove. Neither command starts a GPU train. Locked `examples/estate.yaml` stays deny-default and cksum `43770130 3391`. `READY_FOR_LIVE_TEST`: no.

```bash
make ag-news-journey
estate enrich bind-prove --root . --out /tmp/cell-one-specialty-bind
estate enrich bind-prove --dual --root . --out /tmp/cell-one-specialty-bind-dual
```

## 4c. Host-validate prove (lab copy, not a train)

`estate decisions host-validate-prove` copies the locked estate into a throwaway directory and seeds `ag_news` and `rust_idiom` beside `local_slm` (mocked GGUF files, fixture bindings). Research may use `ag_news`. Idiom may use `rust_idiom`. Sanctum lists both, so the selector abstains (`refuse:decision-abstain`). The prove then runs `estate authorize`, a granted `estate convey call` on hop `specialty-hop`, and `estate complete --mock` with no `--object`. Research mock-completes `ag_news` (`validation=ok`, `surface=complete`). Idiom mock-completes `rust_idiom` the same way. Sanctum abstains, the receipt is `result=abstain` with `refuse:decision-abstain`, and no completion text is granted. A stale hint and an ineligible hint each record `fallback=ag_news` and do not grant the hinted seat as the complete target. `estate decisions report` counts `surface authorize`, `surface convey`, and `surface complete` (complete is greater than zero). `estate decisions export` replays the journal. `estate decisions export-package` writes a standing improvement package from that journal (see 4f). The command ends with `decision-host-validate-prove: ok` and `READY_FOR_LIVE_TEST: no`. It does not rewrite `examples/estate.yaml` (cksum `43770130 3391`). It does not start a GPU train. It does not call Ollama.

```bash
estate decisions host-validate-prove --root . --out /tmp/cell-one-decision-host-validate
```

## 4d. Control-plane prove (one lab, not a train)

`estate control-plane-prove` is the feelable fuel → decide → run loop on one throwaway lab. It does not rewrite `examples/estate.yaml` (cksum `43770130 3391`).

Fuel mock-imports two specialty GGUFs (`trained_shape` gguf, `auto_apply=false`) as `ag_news` (`specialist-agnews-3000`) and `rust_idiom` (`specialist-rustidiom-3000`) beside `local_slm`, then runs apply-proposal, plan, and apply `--require-plan` on the lab only. Standing next prints `joinable: yes` for both seats. The GGUF must be a regular file and an agent models allow-list must name the binding. The file alone is not joinable.

Decide runs the host-validate cases on that same estate and state-dir: authorize, a granted convey call, and `complete --mock`. Research selects `ag_news`. Idiom selects `rust_idiom`. Sanctum abstains (`refuse:decision-abstain`) and gets no completion text. A stale hint and an ineligible hint record `fallback=ag_news` and do not grant the hinted seat. `estate decisions report` shows complete greater than zero and both specialty capabilities.

Run starts and stops the supervised runner on `standing-dual` in that same state-dir. One walk-away tick journals hop 1 `ag_news` `context=none`, hop 2 `rust_idiom` `context=applied`, hop 3 `frontier_http` `context=applied`, with one `session_id` on the hops. The digest cites the session, `package=dual-specialty`, and `chain=chain-dual-specialty-…`. A second start is `refuse:runner-already-running`.

The compact report is `cell-one.control-plane-prove.v0`. `ok` is true only when fuel, decide, and run all pass. `ready_for_live_test` and `live_pass_recorded` stay false. The command ends with `control-plane-prove: ok` and `READY_FOR_LIVE_TEST: no`. In-process mock. No network. No Ollama. No GPU train. This is not a live PASS.

```bash
estate control-plane-prove --root . --out /tmp/cell-one-control-plane
```

## 4e. Cohesion prove (one lab, not a train)

`estate pack cohesion-prove` is the feelable fuel → decide → run →
specialty-real → pack install → multi-hop CLI crew session →
improvement export → gated specialty-seat apply loop on one throwaway
directory. It composes `estate control-plane-prove` (section 4d), then
an optional specialty-real bind, then `estate pack crew-session-prove`,
then the export-package stage of `estate decisions improvement-export-prove`
(section 4f), then one gated apply from section 4g on a throwaway apply
lab under the same `--out`. When import-trained AG News / rust_idiom GGUF artifacts
are present on the machine they bind as named specialty seats
(`ag_news`, `rust_idiom`) beside `local_slm` and `complete --mock`
names those seats (not generic `local_slm`). When absent the stage is
`skipped:gguf-absent` and the prove stays ok. It does not rewrite
`examples/estate.yaml` (cksum `43770130 3391`). It does not rewrite
`examples/fixtures/agent-pack-handoff.yaml`.

Pack install is `estate pack plugin-install-local` for `research-crew`
on that fixture. `HOME` for the install is `<out>/home`, so the copy
lands at `<out>/home/.cursor/plugins/local/research-crew` as a real
directory. The operator's own home is left alone.

Multi-hop CLI crew session smoke is first-class. The baked estate
binary from the install marker creates one pack-scoped `session_id`
and runs successive `estate complete --mock` hops:

```bash
estate pack session create --pack research-crew \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir <throwaway> --id sess-cohesion01

estate complete --mock --agent horizon --pack research-crew \
  --session sess-cohesion01 \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir <throwaway> --prompt unique-hop-alpha-token

estate complete --mock --agent horizon --pack research-crew \
  --session sess-cohesion01 \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir <throwaway> --prompt "follow-up that should see prior turn"
```

Hop 1 exits 0. Stdout includes `decision receipt:` and
`session=sess-cohesion01 turns=1 context=none`. The journal row is
`outcome` allow, `surface` complete, capability `ag_news`, result
`ag_news` (the named specialty seat, not generic `local_slm`).

Hop 2 uses the same `session_id`. Stdout includes
`turns=2 context=applied`. The mock completion cites
`unique-hop-alpha-token`. `estate pack session show` and
`estate decisions report --pack` cite that session.

```bash
estate complete --mock --agent research --pack research-crew \
  --session sess-cohesion01 \
  --estate examples/fixtures/agent-pack-handoff.yaml \
  --state-dir <throwaway> --prompt ping
```

Research exits non-zero with `refuse:pack-orchestrator` and writes no
extra receipt. Session turns stay 2. A fresh session id does not leak
hop 1. A Cursor MCP loader hang is out of scope. The report records
`cursor_loader: out-of-scope` and `loader_is_live_pass: false`.

After export, cohesion stages one specialty seat with
`specialty_bind::stage_one_specialty_for_apply` under `{out}/apply`
and reuses the package already written at `{out}/improvement`. It does not re-run host-validate
for the apply. Apply without `--require-plan`
prints `refuse:plan: apply-package requires --require-plan` and leaves
the apply lab unchanged. `apply-package --require-plan` then applies
one `specialty-seat:*` (prefer `ag_news` when that proposal is present)
through `apply-proposal` → `plan` → `apply --require-plan`. Standing
next for that seat is `joinable: yes`. `local_slm` stays on the apply
lab. Dataset proposals stay proposal-only. `auto_train=false`.
`train_invoked=false`. Train is not invoked.

The compact report is `cell-one.cohesion-prove.v0` (`cohesion-prove.json`).
`ok` is true only when fuel, decide, run, install, multi-hop CLI crew
session smoke, the standing improvement export, and the gated
specialty-seat apply all pass. The `improvement` cite names the package
path, proposal count/kind (`specialty-seat`, `dataset`),
`auto_train=false`, and `train_invoked=false`. The `apply` cite names
the apply receipt path (`cell-one.improvement-apply.v0`), the applied
proposal id/kind/binding, `joinable: yes`, `require_plan=true`,
`auto_train=false`, `train_invoked=false`, and
`refuse:plan: apply-package requires --require-plan`. `estate
decisions report` on that lab state-dir cites the same receipt
(applied proposal id/kind/binding, joinable standing,
`require_plan=true`, the refuse-without-plan string,
`auto_train=false`, `train_invoked=false`, schema path) so the
closed loop is one operator surface without digging files. Specialty-real
may be `skipped:gguf-absent` and still counts as ok.
`ready_for_live_test` and `live_pass_recorded` stay false. `live_sync`
stays false. The command ends with `cohesion-prove: ok` and
`READY_FOR_LIVE_TEST: no`. In-process mock. No network. No Ollama. No
GPU train. This is not a live PASS. `--out` that is
`examples/estate.yaml` is `refuse:out`. Sibling surfaces stay callable
alone (`improvement-export-prove`, `improvement-apply-prove`, and
`apply-package`):

```bash
estate pack crew-session-prove --root . --out /tmp/cell-one-crew-session
estate decisions improvement-export-prove --root . --out /tmp/cell-one-improvement-export
estate decisions improvement-apply-prove --root . --out /tmp/cell-one-improvement-apply
```

```bash
estate pack cohesion-prove --root . --out /tmp/cell-one-cohesion
estate decisions report --state-dir /tmp/cell-one-cohesion/state
```

## 4f. Improvement-export prove (standing package, not a train)

`estate decisions improvement-export-prove` remains callable alone.
It reuses the host-validate authorize / convey / `complete --mock`
surfaces on a throwaway lab, then writes a standing improvement
package from that journal. `estate pack cohesion-prove` composes that
same export-package stage after crew-session on one lab.
`estate decisions export-package --state-dir <dir> --out <throwaway>`
is the operator command: JSON plus YAML under `--out`
(`improvement-package.json` / `improvement-package.yaml`, schema
`cell-one.improvement-package.v0`). The package proposes the next
enrich candidately — specialty seat and dataset for each validated
specialty result (`ag_news`, `rust_idiom`). `auto_train=false`.
Train is not invoked. READY stays no.

The compact report is `cell-one.improvement-export-prove.v0`
(`improvement-export-prove.json`). It cites the package path,
proposal count/kind (`specialty-seat`, `dataset`),
`auto_train=false`, and `train_invoked=false`. Surfaces stay
authorize / convey / complete from host-validate. The command ends
with `improvement-export-prove: ok` and `READY_FOR_LIVE_TEST: no`.
`--out` that is `examples/estate.yaml` is `refuse:out`. Locked
`examples/estate.yaml` stays cksum `43770130 3391`. Not in make
smoke, make gate-90, or GitHub Actions. This is not a live PASS.

```bash
estate decisions improvement-export-prove --root . --out /tmp/cell-one-improvement-export
estate decisions export-package --state-dir /tmp/cell-one-improvement-export/state \
  --out /tmp/cell-one-improvement-export/improvement
```

## 4g. Improvement-apply prove (gated apply, not a train)

`estate decisions improvement-apply-prove` is the human-gated half
after export-package. It reuses host-validate authorize / convey /
`complete --mock` receipts, writes the standing package, picks one
specialty-seat proposal (`specialty-seat:ag_news`), and applies that
proposal on a fresh throwaway apply lab through the existing
`apply-proposal` → `plan` → `apply --require-plan` path. Standing
next is `joinable: yes` after the gated apply. Dataset proposals stay
proposal-only. Train is not invoked. `auto_train=false`.
`estate decisions apply-package --package <package.json> --estate
<lab> --prepared <prepared> --require-plan` is the operator command.
Before apply-proposal / plan / apply it reads
`{prepared}/binding-proposal.json` and requires `binding_id` (and the
picked proposal id) to match the picked specialty-seat. A mismatch is
`refuse:proposal:` (or `refuse:prepared:`) and the lab estate stays
unchanged. Without `--require-plan` it is `refuse:plan` and the lab
estate stays unchanged. `--out` that is `examples/estate.yaml` is
`refuse:out`.
`estate pack cohesion-prove` composes this same gated apply after the
export stage on one throwaway lab (section 4e). It reuses the package
already written under `{out}/improvement` and does not re-run host-validate
for the apply. The apply lab is `{out}/apply`. This
sibling prove stays callable alone, as do `improvement-export-prove`
and `apply-package`.

The compact report is `cell-one.improvement-apply-prove.v0`
(`improvement-apply-prove.json`). It cites the applied proposal
id/kind, Standing next joinable, `auto_train=false`, and
`train_invoked=false`. `estate decisions report` on that lab
state-dir cites the apply receipt (`cell-one.improvement-apply.v0`)
with the same fields. The command ends with
`improvement-apply-prove: ok` and `READY_FOR_LIVE_TEST: no`. Locked
`examples/estate.yaml` stays cksum `43770130 3391`. Not in make
smoke, make gate-90, or GitHub Actions. This is not a live PASS.

```bash
estate decisions improvement-apply-prove --root . --out /tmp/cell-one-improvement-apply
estate decisions apply-package \
  --package /tmp/cell-one-improvement-apply/improvement/improvement-package.json \
  --estate /tmp/cell-one-improvement-apply/apply/lab-estate.yaml \
  --prepared /tmp/cell-one-improvement-apply/apply/state/enrich/overnight-traces/llamafactory-lora \
  --state-dir /tmp/cell-one-improvement-apply/apply/state \
  --plans-dir /tmp/cell-one-improvement-apply/apply/plans \
  --roots-base /tmp/cell-one-improvement-apply/apply/roots \
  --require-plan
```

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
