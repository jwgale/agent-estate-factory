# Enrich-pack drop zone

Documentary drop zone. The live default is `packs/` at the repo root.

Rules:

- curator: jason
- policy: manual
- `promoted: false` always from the feed
- `estate feed promote` fails on purpose
- `estate feed import` copies a candidate to `accepted/` and does not rewrite the estate

To bind a pack, Jason copies the id into `examples/estate.yaml` `enrich_packs.packs`. That is the only estate-bound path.
