# Day 61–90 beachhead (toward A10–A12)

Local gate. Do not add this to GitHub Actions.

```bash
cargo test --workspace
make gate-90
```

Live Grok / live local are not required. CI stays PR-only `cargo test --workspace`.

## A10 — feed packs

Scrubbed traces from both model paths land in `.cell/feed/events.jsonl`. Materialize a **candidate** pack:

```bash
cargo run -p estate-control -- feed pack \
  --feed-dir .cell/feed \
  --drop-dir examples/enrich-packs/drop \
  --id overnight-traces
cargo run -p estate-control -- feed list
cargo run -p estate-control -- feed promote --id overnight-traces   # fails
```

Jason promotes by editing `examples/estate.yaml` `enrich_packs.packs`. Feed never writes the estate.

## A11 — suspend / resume

```bash
make suspend    # or make pause-stop
make resume     # or make pause-start
make status
```

`.cell/lifecycle.json` survives session discard. Re-apply from the estate file. Control does not complete.

## A12 — plan + placement stub

```bash
cargo run -p estate-control -- plan --estate examples/estate.yaml
cargo run -p estate-control -- plans
```

Blast-radius markdown is PR-reviewable. `placements` lists `cell-one-box` and `cursor-cloud` (`cloud-agent`, unwired). Floor does not spawn the cloud stub.

See [`overnight-decisions.md`](overnight-decisions.md).
