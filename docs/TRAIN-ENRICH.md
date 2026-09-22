# Train/enrich prepare

Operator page for the first durable train/enrich beachhead. Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). Product story: [`NORTH-STAR.md`](NORTH-STAR.md).

`estate enrich prepare` writes artifacts for a purpose-built SLM. It does not train. It does not POST. It does not rewrite `estate.yaml`. Promote stays off.

## What exists today

| Piece | Role |
| --- | --- |
| `TrainEnrichDriver` | Data-plane trait in `model-estate`. `id()`, `prepare(job)`, catalog `status` / `probe`. |
| `ollama-modelfile` | Integration. Writes a Modelfile (`FROM` + `SYSTEM`) and `PREPARE.md` with `ollama create`. Does not shell out. |
| `external-manifest` | Portable JSON and YAML. Base model ref, purpose, host class affinity, dataset path hints from the pack `source_paths`. No vendor lock. |
| `estate enrich drivers` | Prints the catalog. `live=false`. A probe here does not train. |
| `estate help enrich` | Same page as `estate help train`. |
| `make enrich-prepare` | Opt-in fixture walk. Not in `make smoke`, `make gate-90`, or GitHub Actions. |

Job field: `enrich` (default) or `train`. The field is a label on the artifact. Both jobs only prepare.

Default output is `.cell/enrich/{pack_id}/{driver}/`. Pass `--out` to write somewhere else, including `packs/prepared/`. Layout: [`cell-layout.md`](cell-layout.md). Schema: [`../schema/train-enrich.v0.json`](../schema/train-enrich.v0.json) (`cell-one.enrich-prepare.v0`).

## Facilitated vs invented

Ollama already creates a model from a Modelfile. This factory writes that file and the next command. It does not invent a local inference server and it does not invent a GPU training stack.

A full LoRA / SFT / DPO loop, a dataset downloader, and a GPU scheduler stay out of this beachhead.

```bash
estate enrich prepare \
  --estate examples/estate.yaml \
  --pack examples/fixtures/specialist-overnight.pack.json \
  --driver ollama-modelfile

# on the seated box, after FROM names a model that runtime already has:
ollama create cell-enrich-overnight-traces -f Modelfile
```

`FROM` is the portable base ref (`model_hint`, or `local_slm` when the pack leaves the hint empty). The factory does not pull weights.

## Swap a driver

```bash
estate enrich prepare \
  --pack examples/fixtures/specialist-overnight.pack.json \
  --driver external-manifest \
  --out target/enrich-manifest
```

`manifest.json` and `manifest.yaml` are the market-shift hatch. A later trainer reads `base_model`, `purpose`, `host_class_affinity`, and `dataset_paths`. Cell One does not call that trainer.

## Add a third entrant

1. Implement `TrainEnrichDriver` in `model-estate` (a sibling of the two drivers).
2. Add one `RegisteredDriver` card in `crates/model-estate/src/train_enrich.rs`.
3. Leave floor-supervisor and `estate-control` dispatch alone. They pass the driver id through.

`estate enrich drivers` prints the new card. No estate file change is required to register it.

## Refuse

Prepare loads the estate the same way pack import does: parsed, then the enrich gates, before the Cell One equal-class shape check. A local-only estate can still hit `refuse:frontier-invent`.

| Condition | Code |
| --- | --- |
| Pack id or path is missing | `refuse:missing-pack` |
| Curator is not `jason`, or the estate curator is not `jason` | `refuse:curator` |
| Sacred token in the pack note, system text, or paths | `refuse:sacred` |
| Hardware SKU in a model id or path | `refuse:sku-banned` or `refuse:model-hint` / `refuse:source-path` |
| Pack tags `frontier` and the estate has no frontier binding | `refuse:frontier-invent` |
| Unknown driver id | `refuse:driver` |
| Job is not `train` or `enrich` | `refuse:job` |

Those stops happen before the output directory is created.

## Opt-in walk

```bash
make enrich-prepare
```

Uses `examples/fixtures/specialist-overnight.pack.json`. Writes both drivers under `target/enrich-prepare-cell`. Asserts a Modelfile and an external manifest. Leaves `examples/estate.yaml` unchanged. Does not need an Ollama binary. The script prints `SKIP live train` because this walk is not a train.

`make real-world` points here and does not run a train. See [`OPERATOR-DAY.md`](OPERATOR-DAY.md).

`READY_FOR_LIVE_TEST`: no.
