# Day 61–90 beachhead (toward A10–A12)

Local gate. Do not add this to GitHub Actions.

```bash
cargo test --workspace
make gate-90
```

Live Grok / live local are not required. Hosted CI is disabled overnight; this gate is local only.

## A10 — feed packs

Scrubbed traces from both model paths land in `.cell/feed/events.jsonl`. Materialize a **candidate** pack, then import explicitly:

```bash
cargo run -p estate-control -- feed pack \
  --feed-dir .cell/feed \
  --drop-dir packs \
  --id overnight-traces
cargo run -p estate-control -- feed list --drop-dir packs
cargo run -p estate-control -- feed import --id overnight-traces --drop-dir packs
cargo run -p estate-control -- feed promote --id overnight-traces   # fails
```

Import does not rewrite `examples/estate.yaml`. Jason binds a pack by editing `enrich_packs.packs` by hand. Pack ids that encode a hardware SKU (`5090`, …) fail closed. `packs/INDEX.md` lists candidates.

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

Blast-radius markdown is PR-reviewable. `estate apply --require-plan` fails without a covering plan and writes an apply audit (`covering_plan` stem + `cloud_agent_spawned=false`). `placements` lists `cell-one-box` and `cursor-cloud` (`cloud-agent`, unwired). Floor records leases on disk via `PlacementDriver`; it does not spawn the cloud stub. Drift fail-closes a spawned cloud-agent lease.

```bash
cargo run -p estate-control -- catalog --out .cell/catalog.json
cargo run -p estate-control -- leases
cargo run -p estate-control -- audits
cargo run -p floor-supervisor -- suspend
cargo run -p floor-supervisor -- resume
```

Reviewed catalog snapshot: [`schema/local-catalog.v0.json`](../schema/local-catalog.v0.json).

```bash
cargo run -p estate-control -- history
cargo run -p estate-control -- probes
cargo run -p estate-control -- feed cursor --feed-dir .cell/feed
cargo run -p estate-control -- apply --require-plan --require-fresh-plan
```

Durable watermarks: `.cell/feed/feed-cursor.json`, `.cell/lifecycle.jsonl`, `packs/accepted/import-audit.jsonl`. Cloud-agent still does not spawn. Probes are catalog-level, not live pings.

See [`overnight-decisions.md`](overnight-decisions.md).
