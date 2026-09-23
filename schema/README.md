# Cell One schema freeze (v0)

Documentary snapshots. Fail-closed SoT is the Rust validator, not these files.

## Compatibility rule

- **Additive fields are ok** on a v0 schema (serde `default`, optional). Existing fixtures stay valid.
- **Renames, type changes, or removed required fields need a v1** (`cell-one.*.v1`) and an upgrade hint. Do not silently reinterpret.

Unknown `apiVersion` / `kind` / pack schema fail closed.

`source_drivers` is an additive v0 field on `pack.v0.json`, `specialist-pack.v0.json`, and `enrich-proposal.v0.json`. Values are only `frontier` and `local`, sorted and unique, and they must match `path_counts`. A missing field deserializes to `[]`, so older packs stay valid. A rename, a new driver string, or a required non-empty list is a v1. `cell-one.enrich-accept.v0` copies that same list into the edit instructions. It does not invent a class and it does not rewrite the estate.

`train-enrich.v0.json` is the prepare envelope (`cell-one.enrich-prepare.v0`). `promoted`, `auto_apply`, and `estate_rewritten` stay false. `ollama-modelfile` adds a Modelfile. `FROM` and `base_model` are the seated model (`params.model` or a model-tag hint), not the binding id `local_slm`. `seat_tag` repeats that seated model when prepare writes it. `external-manifest` adds `manifest.json` and `manifest.yaml` (`base_model`, `purpose`, `host_class_affinity`, `dataset_paths`). `llamafactory-qlora` adds `recipe.yaml`, `export.yaml`, `dataset_info.json`, and instruct chat `dataset.jsonl`. On that card `train_base_model` is the Hugging Face repo id or local HF weights directory written to `model_name_or_path`. `llamafactory-lora` adds the same files with no quantization keys in `recipe.yaml`. `export.yaml` is the merge card and does not claim the merge ran. `axolotl-lora` adds `axolotl.yml` and Alpaca `dataset.jsonl`, and writes that same train base to `base_model`. Gauge knobs (`max_steps` and the related fields) are written into those recipes when the operator sets them. The Ollama seat tag stays `base_model` and `seat_tag` in `prepare.json`. The operator runs those trainers outside the factory. `NEXT.md` is the operator handoff. A new required field on this envelope is a v1.

`train_base_model` is an additive optional field on `pack.v0` and `specialist-pack.v0`. Empty means unset. `llamafactory-qlora`, `llamafactory-lora`, and `axolotl-lora` read `params.train_base_model` on the local binding when the pack field is empty. A bare Ollama seat tag in that field is `refuse:train-base`. A relative directory is written into the prepare envelope as an absolute path. A directory name that is an Ollama seat tag is the same refuse.

`enrich-binding-proposal.v0.json` is the `local_slm` join (`cell-one.enrich-binding-proposal.v0`). `estate enrich import-prepared` writes it next to `prepare.json`. `auto_apply`, `promoted`, and `estate_rewritten` stay false. The command does not apply.

`enrich-binding-stage.v0.json` is the plan input (`cell-one.enrich-binding-stage.v0`). `estate enrich apply-proposal` writes `staged-estate.yaml` plus this receipt under `{state}/enrich-stage/`. `applied` and `estate_rewritten` stay false until `estate apply --require-plan` succeeds. `auto_apply` stays false.

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
| `train-enrich.v0.json` | `cell-one.enrich-prepare.v0` | `estate enrich prepare` artifacts. Does not train |
| `enrich-binding-proposal.v0.json` | `cell-one.enrich-binding-proposal.v0` | `estate enrich import-prepared`. Proposal for `local_slm`. Does not apply |
| `enrich-binding-stage.v0.json` | `cell-one.enrich-binding-stage.v0` | `estate enrich apply-proposal`. Plan input. Source estate waits for `apply --require-plan` |
| `feed-cursor.v0.json` | `cell-one.feed-cursor.v0` | Feed watermark |
| `policy.v0.json` | `cell-one.policy.v0` | Deny/allow stub |
| `cell-backup.v0.json` | `cell-one.cell-backup.v0` | Local cell archive |
| `sacred.v0.json` | `cell-one.sacred.v0` | Dual-layer sacred file |

Policy files (not schema snapshots): `policy/cell-one.policy.v0.yaml`, `policy/sacred.yaml`.

Documentary ids without a snapshot file: `cell-one.reconcile-suggest.v0` (`reconcile --suggest`), `cell-one.enrich-accept.v0` (`packs accept`), `cell-one.backup-prune.v0` (`backup --prune`). Same freeze rule: additive ok, rename → v1.
