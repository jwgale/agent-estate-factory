# Feed loop (fixtures only)

One local walk: **scrubbed trace → pack → propose → accept**.

Start here: [`../README.md`](../README.md) · full operator walk (gate-90 → this loop → backup prune): [`OPERATOR-DAY.md`](OPERATOR-DAY.md). Not a live-box proof. Parked Mac / GPU / cloud-spawn: [`DAY90-PLUS.md`](DAY90-PLUS.md).

```bash
make feed-loop
```

The script uses an isolated `target/feed-loop-cell`. It does not touch the
operator `.cell/`. It unsets `XAI_API_KEY` and `CELL_*_ENDPOINT`. It is not
part of `make smoke` (that gate is already long). Hosted CI never runs it.

## What the walk proves

1. **Scrubbed traces.** `model-estate task --mock` writes feed events for the
   mixed path (authorize → local precheck → tool or frontier). Notes are
   job/byte counts, not prompts or keys. The durability test also appends a
   secret-shaped note and asserts the stored event is `[redacted]`.
2. **Pack.** `estate feed pack` materializes a candidate pack
   (`overnight-traces`) and stamps `feed-cursor.json` with
   `schema=cell-one.feed-cursor.v0`, `events > 0`, and `packed_id`.
   `source_drivers` lists `frontier` and/or `local` and matches `path_counts`.
   The script asserts the produced pack is exactly `frontier` then `local`,
   and that propose copies the same tag. `promoted` stays false. Live keys
   are unset.
3. **Propose.** `estate packs propose` writes `packs/proposed/` with
   `auto_apply: false` and copies `source_drivers` onto the diff. The estate
   file is unchanged.
4. **Accept.** `estate packs accept --curator jason` writes enrich-pack
   *edit instructions* (`applied_to_estate: false`). Wrong curator refuses.
   Promote still fails closed.

## Cursor durability

`feed-cursor.json` is a watermark, not estate SoT. Rematerialize (a second
`feed pack`) keeps the cursor on disk and does not auto-promote. `estate
feed cursor` pretty-prints the JSON.

## Rails

Jason still pastes pack ids onto `estate.enrich_packs` by hand. Feed never
rewrites `estate.yaml`. Auto-promote stays locked off.
