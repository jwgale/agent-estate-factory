# Pack drop zone (`packs/`)

Live Feed\u2192Control drop zone. `estate feed pack` writes `{id}.pack.json` here from scrubbed traces.

Rules:

- curator: `jason`
- policy: `manual`
- `schema`: `cell-one.pack.v0`
- `promoted: false` always from the feed
- `estate feed promote` fails on purpose
- `estate feed import` is the explicit apply. It copies a candidate into `packs/accepted/` and **does not rewrite** `examples/estate.yaml`
- `estate packs propose` writes `packs/proposed/{id}.proposal.json` after import. `auto_apply` is always false. Jason reviews the diff and edits the estate by hand.
- Jason still lists a pack id on `enrich_packs.packs` by hand before it is estate-bound

Generated `*.pack.json` files stay gitignored. Documentary copies live under `examples/enrich-packs/`.

Pack ids must be slugs and must not encode a hardware SKU. `host_class` is `consumer-nvidia|apple-silicon|rented-nvidia|any`. `INDEX.md` is regenerated next to the drop.
