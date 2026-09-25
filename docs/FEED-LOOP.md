# Feed loop (fixtures only)

One local walk: **scrubbed trace → pack → propose → accept**.

Start here: [`../README.md`](../README.md) · [`NORTH-STAR.md`](NORTH-STAR.md) · operator walk (gate-90 → this loop → `make real-world`): [`OPERATOR-DAY.md`](OPERATOR-DAY.md). This walk is fixtures. Parked rows: [`DAY90-PLUS.md`](DAY90-PLUS.md). Accept writes curator edit instructions. It does not rewrite the estate.

```bash
make feed-loop
```

The script uses an isolated `target/feed-loop-cell`. It does not touch the
operator `.cell/`. It unsets `XAI_API_KEY` and `CELL_*_ENDPOINT`. It is not
part of `make smoke` (that gate is already long). Hosted CI never runs it.

## What the walk proves

1. **Scrubbed traces.** `model-estate task --mock` against locked
   `examples/estate.yaml` (`intentions: []`) is deny-default. Horizon
   `model` / `xai_grok` refuses because the frontier model class has no
   allow Model intention. Research `tool` / `notes-append` refuses the same
   way. The script expects that refuse. It does not add intentions. Feed
   events are those deny lines. They are not an allow, a local precheck, or
   a frontier complete. The durability test still appends a secret-shaped
   note and asserts the stored event is `[redacted]`.
2. **Pack.** `estate feed pack` materializes a candidate pack
   (`overnight-traces`) and stamps `feed-cursor.json` with
   `schema=cell-one.feed-cursor.v0`, `events > 0`, and `packed_id`.
   `source_drivers` stays empty because the locked estate did not authorize
   frontier or local. `INDEX.md` lists `drivers=-`. The script checks the
   pack, the proposal, and `enrich-edit.json` carry that empty list and do
   not invent `frontier` or `local`. `promoted` stays false. Live keys are
   unset. There is no `make feed-loop-mixed`.
3. **Propose.** `estate packs propose` writes `packs/proposed/` with
   `auto_apply: false` and copies `source_drivers` onto the diff. The estate
   file is unchanged.
4. **Accept.** `estate packs accept --curator jason` writes enrich-pack
   *edit instructions* (`applied_to_estate: false`) and copies
   `source_drivers` onto them. The script checks the pack, the proposal,
   and `enrich-edit.json` carry the same list. An empty list stays `-`.
   A tag that does not match `path_counts` refuses before the edit file
   is rewritten, and the pack or proposal index is left unchanged.
   Wrong curator refuses. Promote still fails closed.

## Cursor durability

`feed-cursor.json` is a watermark, not estate SoT. Rematerialize (a second
`feed pack`) keeps the cursor on disk and does not auto-promote. `estate
feed cursor` pretty-prints the JSON.

Convey allow and deny lines use that same file. `estate convey call` and a
coverage or intention refuse on `estate convey hop` append `proxy.hop` to
`.cell/feed/events.jsonl` and restamp `.cell/feed/feed-cursor.json`.
An Agent intention on those paths, and `conveyor-proxy check --feed-dir`,
appends `proxy.agent` the same way (`allow`, `deny`, or `deny-default`).
`conveyor-proxy check --feed-dir .cell/feed` appends `proxy.{kind}` for
`tool`, `mcp`, `mount`, `model`, `memory_read`, and `agent`.
`estate feed cursor --feed-dir .cell/feed` shows the watermark. Nothing
in that path promotes a pack.

## Rails

Jason still pastes pack ids onto `estate.enrich_packs` by hand. Feed never
rewrites `estate.yaml`. Auto-promote stays locked off.
