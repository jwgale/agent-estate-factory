# Enrich-pack drop zone

Usable now. `estate feed pack` writes `{id}.pack.json` here from scrubbed feed traces.

Rules:

- curator: jason
- policy: manual
- `promoted: false` always from the feed
- `estate feed promote` fails on purpose

To use a pack, Jason copies the id into `examples/estate.yaml` `enrich_packs.packs`. That is the only promotion path.
