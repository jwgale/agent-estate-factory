# Cell One schema freeze (v0)

Documentary snapshots. Fail-closed SoT is the Rust validator, not these files.

## Compatibility rule

- **Additive fields are ok** on a v0 schema (serde `default`, optional). Existing fixtures stay valid.
- **Renames, type changes, or removed required fields need a v1** (`cell-one.*.v1`) and an upgrade hint. Do not silently reinterpret.

Unknown `apiVersion` / `kind` / pack schema fail closed.

`examples/estate.yaml` hash is locked at `sha256:dcd7164f04c83f514185e77d2d4f6c23cae6dbb27a9b5da96a28ba1f3c724930` (see `estate-schema` `example_estate_hash_is_locked`). YAML comments are ok. Renames need a new hash.

## v0 files

| File | Schema id | Used by |
| --- | --- | --- |
| `estate.v0.schema.json` | `cell-one.estate.v0` | Desired-state YAML |
| `estate-plan.v0.json` | `cell-one.plan.v0` | `estate plan` |
| `placement-actual.v0.json` | `cell-one.placement-actual.v0` | Durable leases |
| `lifecycle.v0.json` | `cell-one.lifecycle.v0` | Suspend / resume |
| `session-journal.v0.json` | `cell-one.session-journal.v0` | `.cell/sessions.jsonl` |
| `apply-dry-run.v0.json` | `cell-one.apply-dry-run.v0` | `estate apply --dry-run` |
| `reconcile.v0.json` | `cell-one.reconcile.v0` | `estate reconcile` |
| `conveyor-mesh.v0.json` | `cell-one.conveyor-mesh.v0` | Hop mesh / leases |
| `local-catalog.v0.json` | `cell-one.local-catalog.v0` | Portable driver catalog |
| `pack.v0.json` | `cell-one.pack.v0` | Feed pack drop zone |
| `specialist-pack.v0.json` | `cell-one.specialist-pack.v0` | Additive pack metadata |
| `enrich-proposal.v0.json` | `cell-one.enrich-proposal.v0` | Propose, never apply |
| `feed-cursor.v0.json` | `cell-one.feed-cursor.v0` | Feed watermark |
| `policy.v0.json` | `cell-one.policy.v0` | Deny/allow stub |
| `cell-backup.v0.json` | `cell-one.cell-backup.v0` | Local cell archive |
| `sacred.v0.json` | `cell-one.sacred.v0` | Dual-layer sacred file |

Policy files (not schema snapshots): `policy/cell-one.policy.v0.yaml`, `policy/sacred.yaml`.

Documentary ids without a snapshot file: `cell-one.reconcile-suggest.v0` (`reconcile --suggest`), `cell-one.enrich-accept.v0` (`packs accept`), `cell-one.backup-prune.v0` (`backup --prune`). Same freeze rule: additive ok, rename → v1.
