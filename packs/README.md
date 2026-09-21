# Pack drop zone (`packs/`)

Live Feed→Control drop zone. `estate feed pack` writes `{id}.pack.json` here from scrubbed traces.

Rules:

- curator: `jason`
- policy: `manual`
- `schema`: `cell-one.pack.v0`
- `promoted: false` always from the feed
- `estate feed promote` fails on purpose
- `estate feed import` / `estate packs import` is the explicit apply. It copies a candidate into `packs/accepted/` and **does not rewrite** `examples/estate.yaml`
- **Import gate:** pass `--curator jason` (default). The flag must match the overnight lock **and** `estate.enrich_packs.curator`. Wrong curator → `refuse:curator`. Config curator on the estate is `jason`.
- `estate packs propose` writes `packs/proposed/{id}.proposal.json` after import. `auto_apply` is always false. Jason reviews the diff and edits the estate by hand.
- Specialist fields (`source_paths`, `model_hint`, `host_class_affinity`) validate on import/propose. Schema may be `cell-one.pack.v0` or `cell-one.specialist-pack.v0`. Raw secrets are refused; import writes `{id}.redaction.json` (kind counts only).
- Jason still lists a pack id on `enrich_packs.packs` by hand before it is estate-bound

Generated `*.pack.json` files stay gitignored. Documentary copies live under `examples/enrich-packs/`.

Pack ids must be slugs and must not encode a hardware SKU. `host_class` is `consumer-nvidia|apple-silicon|rented-nvidia|any`. `INDEX.md` is regenerated next to the drop.
