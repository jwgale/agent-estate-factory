# Operator journeys: train/enrich

Walks for the suite goal: facilitate train and enrich of purpose-built small-parameter models. Today's beachhead is enrich packs and the specialist path. No training stack ships on `main`.

Locked defaults: [`../charter.md`](../charter.md). Product page: [`NORTH-STAR.md`](NORTH-STAR.md). Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). Prepare command surface: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Live paste target: [`LIVE-PROBES.md`](LIVE-PROBES.md).

Fixture accept loop: [`FEED-LOOP.md`](FEED-LOOP.md). Seated drivers: [`operator-local.md`](operator-local.md).

This page adds no command, no crate, and no trainer. `READY_FOR_LIVE_TEST` stays no. Recorded specialist rows stay on the live-probes page.

## What stays fixed

| Piece | Stays |
| --- | --- |
| Binding id | `local_slm`, class `local` |
| Equal-class pair | `xai_grok` (frontier) and `local_slm` (local) |
| Curator | Jason. `enrich_packs.curator: jason`. `policy: manual` |
| Seat | A local runtime. Ollama is today's entrant. Catalog, route, and bind take the next process |
| Hardware | `host_class`: `consumer-nvidia`, `apple-silicon`, `rented-nvidia`, or `any` |
| Example estate | `examples/estate.yaml` stays `driver: ollama` and hash-locked, with `packs: []` |

Agents that already allow `local_slm` keep that id. Horizon and Research list it. Sanctum lists no models. Sanctum is the lane `sanctum`. The exclusion id is `cyera-ci`.

## 1. Prepare a purpose-built SLM on Ollama

Accept writes curator edit instructions. Jason pastes the pack id into the estate by hand. Ollama, the entrant already on the box, builds the model image with its own Modelfile and `ollama create`.

1. Accept the proposal. A wrong curator is `refuse:curator`.

```bash
estate packs accept --id <pack-id> --curator jason
```

That writes `packs/accepted/<pack-id>.enrich-edit.md` and the matching `.json`. `auto_apply` is false. `applied_to_estate` is false. `source_drivers` (`frontier` and/or `local`) is copied onto the instructions. It is not an estate field. The estate file is unchanged. Promote stays refused: `estate feed promote` and `estate packs promote` fail on purpose.

2. Paste the snippet from the enrich-edit file under `enrich_packs.packs` on the estate file you apply. `examples/estate.yaml` on `main` stays hash-locked. Use a lab copy for the paste. Read the blast radius, then apply that file.

```bash
estate plan --estate <your-estate.yaml>
estate apply --estate <your-estate.yaml> --require-plan
```

The pasted entry is an id and a description. `source_drivers` stays a comment on the instruction file.

3. On the host, the Modelfile is `FROM` plus `SYSTEM`. `estate enrich prepare` writes that file and the `ollama create` line. It does not shell out. You can write the same file by hand. Command page: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). `FROM` is a model Ollama already has, or weights a trainer returned (journey 3). `SYSTEM` is the curator text for that pack's job (`policy-precheck`, `redact`, or `complete`). Keep `cyera`, `rust-classroom`, and `rust_classroom` out of the file. The model name is a slug. A hardware SKU in that name refuses when the specialist runs.

```text
FROM llama3
SYSTEM You are the Cell One specialist for this pack. Answer the job and stop.
```

```bash
ollama create cell-one-specialist -f Modelfile
```

4. Point the seat at that process and complete once. Same verb as the recorded live rows.

```bash
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
export CELL_LOCAL_MODEL=cell-one-specialist
estate specialist --driver ollama --prompt "Reply with the single word pong."
```

Expect `"job": "complete"` and a non-empty `"completion"`. An empty completion refuses. If the base is missing, `ollama pull` the `FROM` model first. Copy-paste for the recorded base `llama3`, and for the Mac and Linux boxes, is on [`LIVE-PROBES.md`](LIVE-PROBES.md). `cell-one-specialist` is an operator name. It is not a recorded live proof until Jason pastes a run.

The live local model is `CELL_LOCAL_MODEL`. It does not become the binding id. Unset `CELL_LOCAL_MODEL` uses the first id from Ollama `GET /api/tags`, then `/v1/models`.

## 2. Swap the entrant. Keep `local_slm`

The binding id is the estate contract. The process in the local-runtime seat changes. Agents that list `local_slm` stay as they are.

On the estate file you apply (leave the hash-locked example on `main` as `driver: ollama`):

```yaml
- id: local_slm
  class: local
  driver: llama.cpp
  params:
    host_class: any
    endpoint_env: CELL_LOCAL_ENDPOINT
    job: policy-precheck
  wired: true
```

`llama.cpp` is the swap-proof entrant on the same specialist protocol. Catalog route reads `driver`. Bind attaches that card. `host_class` stays one of the four names. `5090`, `4090`, and `m3-max` stay out of the id.

Point `CELL_LOCAL_ENDPOINT` at the llama.cpp server. The adapter posts to `/v1/chat/completions` or `/api/chat` after a models list answers. The factory stand-in is `POST {CELL_LOCAL_ENDPOINT}/v0/specialist`. Then:

```bash
estate specialist --driver llama.cpp --prompt "Reply with the single word pong."
```

A later process uses the same shape: a `driver` string that catalog, route, and bind already take, still on id `local_slm`. Ollama-on-Mac keeps driver `ollama` on `apple-silicon`. Native MLX stays a stub on that host class. vLLM and TRT stay experimental catalog cards. `specialist()` on a stub or experimental card fails closed. `estate probes --live` does not print `live ok` for `mlx`, `vllm`, or `trt`.

Supported means the seated driver is green on the box. Today that is `ollama` and llama.cpp. `http-remote` is the `CELL_LOCAL_ENDPOINT` pattern on `host_class: any`.

Plan the driver edit before apply. The id string `local_slm` does not change, so the allow-list on each agent stays valid.

## 3. Hand an external manifest to a trainer

GPU training stays off this factory. `estate enrich prepare --driver external-manifest` writes a portable JSON/YAML hatch. See [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Jason also writes an operator note and hands that file to a trainer outside the factory. The factory does not parse that note, apply it, or store it as estate source of truth.

Write it outside the estate file and outside `.cell/`. Name the file `external-manifest.md`. Copy the accept file for the pack id, curator, policy, `source_drivers`, and estate hash. `host_class` and `job` come from the estate binding you apply. `serve_with` and `serve_as` are notes for the trainer: which entrant will load the weights, and the slug from journey 1.

```text
pack_id: <pack-id>
curator: jason
policy: manual
binding_id: local_slm
class: local
host_class: consumer-nvidia
source_drivers: local
job: policy-precheck
estate_hash: <hash printed on the enrich-edit file>
serve_with: ollama
serve_as: cell-one-specialist
```

`host_class` is `consumer-nvidia`, `apple-silicon`, `rented-nvidia`, or `any`. `source_drivers` is `frontier` and/or `local`, copied from the accept file, and it has to match the pack's `path_counts`. `serve_with` names the entrant that will load the weights when they return: `ollama` today, `llama.cpp` after journey 2, or a later driver that has a catalog card. `serve_as` is the slug from journey 1. A SKU in `serve_as` or `pack_id` is refused when that name later hits a binding, a probe, or `CELL_LOCAL_MODEL`.

The trainer trains on their own hardware. Weights come back to the operator. Journey 1 loads them with `ollama create`, or journey 2 points `CELL_LOCAL_ENDPOINT` at the other runtime. The pack id enters the estate only when Jason pastes it. Returned weights do not promote themselves.

No dataset pipeline and no trainer crate ship with this page.

## 4. Fail-closed moments

| Moment | Stop |
| --- | --- |
| Sacred | `cyera-ci` and `rust-classroom` always apply. Aliases `cyera`, `cyera_ci`, and `rust_classroom` resolve to those ids and refuse. `policy/sacred.yaml` can add names (`lab-notebook` in fixtures). A file cannot drop a locked id. An allow intention cannot punch through. A specialist prompt that contains `cyera`, `rust-classroom`, or `rust_classroom` refuses before any POST, on the local seat and on frontier. |
| SKU | `5090`, `4090`, and `m3-max` in a binding id, probe id, pack id, hop id, or model id (`CELL_LOCAL_MODEL`, `CELL_FRONTIER_MODEL`) refuse before the write or the POST. Name the box with `host_class`. A 5090-class box is `consumer-nvidia` or `rented-nvidia`. |
| Local down | Estate-bound local work audits `model.local.down` and stops. The task path ends `local:down`. Frontier is not called. An unset `CELL_LOCAL_ENDPOINT` on `estate probes --live` or `make real-world` prints SKIP and exits 0. That SKIP is the opt-in ladder. The estate-bound deny is the `model.local.down` row. |
| No auto-promote | Feed materializes candidate packs. `estate packs propose` leaves `auto_apply: false`. Accept writes edit instructions and does not rewrite the estate. Promote stays refused. A second `estate feed pack` keeps the feed cursor and does not promote. A `frontier` tag in `source_drivers` on an estate with no frontier binding is `refuse:frontier-invent`. |
| Stub or experimental seat | Native MLX, vLLM, and TRT fail closed at `specialist()` until Jason verifies. Estate-bound local work does not fall through to `grok-4.7`. |

Text over 16KiB refuses before the POST. Credentials stay in the environment (`XAI_API_KEY`, `CELL_LOCAL_ENDPOINT`). The estate file does not carry them.

## 5. Integrate a driver, or let a new one earn its keep

Build rule (`integrate-vs-invent`): a feature earns its keep. If `ollama` or llama.cpp already does the job, tighten that driver.

Use Ollama for running a model on the host, including Ollama-on-Mac, and for building a purpose-built image from a base or from weights the trainer returned. The Modelfile and `ollama create` already do that job. Use llama.cpp when that same specialist protocol should run in the llama.cpp process: change `driver` on `local_slm` and point `CELL_LOCAL_ENDPOINT` at it.

A new driver earns a catalog card when `ollama` and llama.cpp both lack the job. The card goes through catalog, route, and bind. The binding id stays `local_slm`. Floor core does not gain a vendor string. The driver stays a trait. The specialist process may be any language. Jason verifies before the card is Supported. Until that verification, the card stays stub or experimental and fails closed.

Anti-shrink keeps these out of the factory: an AI gateway, an Ollama wrapper-as-product, LM Studio-alone, a Grok Bot clone, a chat UI, a weight browser. GPU training stays with the trainer who holds the external manifest.

Control does not complete. Completion stays on the data plane (`estate specialist`, `model-estate`).

## Flexibility that must survive

From [`../charter.md`](../charter.md):

- Swap isolation, frontier, or local drivers without rewriting floor core.
- Add or rename lanes in the estate file. Sacred names stay locked.
- Pause and resume from files. Runtime is regenerable.
- Keep frontier and local equal class in the schema.
- Compile intentions from the estate. Policy does not learn in silence.
- Keep `CELL_LOCAL_ENDPOINT` as config. The host is not compiled in.
- Swap Ollama, llama.cpp, or a later entrant through catalog, route, and bind. Hardware stays `host_class`.
