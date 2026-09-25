# Ubiquitous language

Canonical Cell One words. Locked defaults stay in [`../charter.md`](../charter.md). `estate help north-star` and `estate help charter` print the product sentence. Prepare: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md).

## Terms

### estate

Desired-state file. `kind: agent-estate`. It names lanes, agents, intentions, model bindings, sacred exclusions, enrich packs, and placements. `examples/estate.yaml` is hash-locked. The estate file is the source of truth for desired state. Code source of truth is [github.com/jwgale/agent-estate-factory](https://github.com/jwgale/agent-estate-factory). Fail-closed check is the Rust validator. JSON Schema files are documentary.

### lane

Horizon, Research, and Sanctum. Separate. Own-lane memory read is allowed. Cross-lane memory is deny-default. Sanctum is the lane `sanctum`. The exclusion id is `cyera-ci`. Rust classroom stays an exclusion.

### plan

Human control surface. `estate plan` writes reviewable markdown under `plans/`. Blast radius a person can read before apply. `apply --require-plan` and `--require-fresh-plan` read those files.

### apply

Converge desired state. Binds box sessions. Records placement leases. Writes regenerable actual state, the desired snapshot, model actual, and placement actual. Audited. `--dry-run` writes nothing.

### lease

Durable grant. Optional TTL. Placement leases: `.cell/placement-actual.json`. Hop leases: `.cell/conveyor-leases.json`. Expired leases refuse. `estate expire --forget` drops expired placement rows so apply can restamp. Convey forget keeps hop declarations.

### pack

Enrich pack. Curator edit instructions. `policy: manual`. `estate packs propose` writes a proposal and leaves it unapplied. `estate packs accept --curator jason` writes the instruction file. Jason pastes into the estate file. `source_drivers` are `frontier` and/or `local`. Promote stays refused.

### curator

Jason. Locked `enrich_packs.curator: jason`. A wrong curator is `refuse:curator`.

### enrich

Curator work on a pack: edit instructions Jason pastes into the estate. `policy: manual`. `estate enrich prepare` writes artifacts for a purpose-built SLM (`--all-drivers` writes every card the job allows). `estate enrich from-pack` runs that prepare for an accepted pack into `.cell/enrich`. `estate enrich list` reads `.cell/enrich`. `estate enrich import-prepared` writes a `local_slm` binding proposal and does not apply. `estate enrich apply-proposal` stages that proposal under `.cell/enrich-stage/`. `estate apply --require-plan` writes the source estate. The job field is `train` or `enrich`. The default job is `enrich`. `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, and `axolotl-qlora` default to `train`. Prepare does not train, does not POST, and does not rewrite the estate. Modelfile `FROM` is the seated model: `params.model` on the local binding, or a pack `model_hint` that is already a model tag. The binding id `local_slm` is not that tag. A missing seated name is `refuse:base-model`. The train base for `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, and `axolotl-qlora` is a separate field (`train_base_model` on the pack, or `params.train_base_model` on the local binding): a Hugging Face repo id or a local directory of HF weights. A bare Ollama seat tag in that field is `refuse:train-base`. `dataset.jsonl` is a scaffold unless `--from-feed` copies rows already under the cell state directory. A missing file with that flag is `refuse:dataset`. Command surface: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md). Live handoff: [`LIVE-PROBES.md`](LIVE-PROBES.md).

### purpose-built SLM

A small-parameter model crafted for one job. Open-source SLMs will get more common. The suite facilitates training and enrichment of these models. The outcome belongs to the estate.

### TrainEnrichDriver

Data-plane trait in `model-estate`. Methods: `id()`, `prepare(job)`, catalog `status` and `probe`. Cards on `main`: `ollama-modelfile` (integrate Ollama `create` / Modelfile `FROM` + `SYSTEM`; `FROM` is the seated model, never the binding id), `external-manifest` (portable JSON/YAML for a later trainer), `llamafactory-lora` (a LLaMA-Factory LoRA recipe with no quantization; `model_name_or_path` is the train base, separate from the Ollama seat tag; rank 8; default job `train`; does not require bitsandbytes; the factory does not run LLaMA-Factory; after train, `merge-adapt` prints `llamafactory-cli export`), `llamafactory-qlora` (a LLaMA-Factory QLoRA recipe; `model_name_or_path` is that same train base; default job `train`; the factory does not run LLaMA-Factory; `merge-adapt` prints that same export), `axolotl-lora` (bf16 Axolotl LoRA matching `examples/llama-3/lora-1b.yml`; `base_model` is that same train base, separate from the Ollama seat tag; default job `train`; the factory does not run Axolotl; after train, `estate enrich merge-adapt` prints `axolotl merge-lora`), and `axolotl-qlora` (4-bit Axolotl QLoRA matching `examples/llama-3/qlora.yml`; that same train base; default job `train`; the factory does not run Axolotl; `merge-adapt` also prints `--dequant`). `unsloth-qlora` is an optional NEXT card (`status` `optional`): an Nvidia-only QLoRA handoff that writes `UNSLOTH.md` and does not write a script or call Unsloth. After that train, `merge-adapt` prints `save_pretrained_merged` with `save_method` `merged_16bit`. `mlx-lm-lora` is an optional NEXT card (`status` `optional`): an Apple Silicon LoRA handoff that writes `MLX.md` when `host_class_affinity` is `apple-silicon` and does not write a script or call mlx-lm. Another affinity is `refuse:host`. After train, `merge-adapt` prints `mlx_lm.fuse` and `local-seat` prints the GGUF from `--export-gguf`. A later train/enrich entrant is another card. llama.cpp INI presets stay off this catalog until a pack or binding names a GGUF path. Floor and control dispatch do not match driver ids.

### train/enrich facilitation

The factory makes room for training and enrichment of purpose-built SLMs. The durable beachhead is `TrainEnrichDriver`, curator packs, and the specialist path (`estate specialist`, `local_slm`). Prepare writes artifacts. `llamafactory-lora` writes the unquantized LLaMA-Factory LoRA recipe an operator runs outside the factory, `llamafactory-qlora` writes the QLoRA recipe, `axolotl-lora` writes the bf16 Axolotl YAML, `axolotl-qlora` writes the 4-bit Axolotl YAML, and `import-trained` hands the adapter back through the existing proposal. Target C is the popular Qwen QLoRA ladder: that `llamafactory-qlora` prepare, the external train and export lines, a printed GGUF convert, a printed Ollama create, and `import-trained` to record the shape. Target A is the Qwen LoRA ladder: that `llamafactory-lora` prepare, the external train and export lines, a printed merge, a printed GGUF convert, a printed Ollama create, and `import-trained` to record the shape. `make seat-journey` prints the Target C merge, convert, seat, and import lines against fixture stubs. After the GGUF `local-seat` print, the standing next step is `import-trained` for that GGUF. The proposal stays `auto_apply=false`. The script does not run `ollama create`, does not apply the estate, and does not convert. `--from-feed` copies instruct rows already on disk. List reads them. Import-prepared proposes the `local_slm` join and leaves plan/apply to Jason. A dataset downloader and an in-process trainer stay unshipped. No new crate. Command surface: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md).

### frontier

Equal-class model binding. Cell One id `xai_grok`. Driver `frontier-http`. Live model `grok-4.7` when `XAI_API_KEY` is set. The binding model is `params.model` on the estate. The catalog schema card prints `grok-4.7`. `CELL_FRONTIER_MODEL` names the live POST. Three fields. A cell catalog that disagrees with the binding is `refuse:frontier-model`.

### local

Equal-class model binding. Cell One id `local_slm`. Class `local`. The binding is the estate contract. The process behind it sits in the local-runtime seat.

### local runtime (driver)

A local runtime is an ecosystem seat, held by a driver. Software on a host runs a model. Catalog, route, and bind take any entrant. Endpoint: `CELL_LOCAL_ENDPOINT`.

Ollama fills the seat today. Another process can fill it tomorrow. llama.cpp is the swap-proof entrant on the same specialist protocol. Ollama is local-run software: it lets a user run models. The estate is the product. The binding id stays `local_slm`.

Supported means the seated driver is green on the box. Today that is `ollama` and llama.cpp. vLLM and TRT stay experimental catalog cards. Native MLX stays a stub. Apple path: Ollama-on-Mac uses the `ollama` driver in that seat.

Local down audits `model.local.down`. Estate-bound local work stops.

### entrant

A process that can fill the local-runtime seat. `ollama` is the entrant on `main`. llama.cpp enters on the same specialist protocol. A later process enters through catalog / route / bind. A train/enrich entrant is separate: another `TrainEnrichDriver` card. It does not replace the seat.

### integrate-vs-invent

Build rule. A feature earns its keep. If `ollama` or llama.cpp already does the job, tighten that driver. Ollama already creates a model from a Modelfile. `ollama-modelfile` writes that file. A from-scratch local server, and a from-scratch trainer, wait until the entrant does not already do the job.

### sacred

Dual-layer exclusions. Hardcoded `cyera-ci` and `rust-classroom` always apply. `policy/sacred.yaml` overlays add names (`lab-notebook` in fixtures). A file cannot drop a locked id. An allow intention cannot punch through.

### refuse

Fail-closed stop. CLI and drift use the `refuse:` prefix. Examples: `refuse:help-topic`, `refuse:curator`, `refuse:frontier-model`, `refuse:frontier-invent`, `refuse:sacred-id`, `refuse:cloud-spawned`, `model.local.down`. Unknown actions, hardware SKUs in binding or probe ids, and sacred names stop before the write or the POST.

### placement

Where agents run. `box` is this Cell One. `cloud-agent` (`cursor-cloud`) is a declared lease. Floor records the lease. Spawn of that cloud lease stays parked. `host_class` is `consumer-nvidia` | `apple-silicon` | `rented-nvidia` | `any`. Hardware is a driver choice.

### convey

`estate convey` is a lease-bound hop stub. Verbs: hop, call, list, leases, sync, expire. Files: `.cell/conveyor-mesh.json`, `conveyor-hops.json`, `conveyor-leases.json`. Call requires a granted lease. A spawned cloud hop lease refuses before JSON, restamp, or forget. Hop transport stays parked. Hop uses `--intention-kind` (distinct from hop `--kind`). An allow hop still refuses when the declared capability does not match the placement-derived coverage capability.

An `agent` intention is who may call whom. Kind `agent` (estate YAML and CLI aliases `agent_call`, `agent-call`). The object is an estate agent id, bare or `agent:`. The subject lists that id under `calls`. Missing coverage is deny-default. An explicit deny wins. Sacred ids are not call targets. When that id is also a tool, MCP, mount, or model on the same agent, pass `--kind agent` or `--intention-kind agent`, or use the `agent:` prefix. `lane-tool` and `mesh-stub` stay hop capabilities and are not inferred as Agent. Plan, drift, and doctor cite each edge as `own` (the same agent) or `peer` (another agent).

`estate convey hop --agent` names the population on the hop lease. Empty agents are not a grant. `estate convey hop` and `estate convey sync` refuse a placement hop whose capability disagrees with placement-derived coverage (`lane-tool` on box, `mesh-stub` on cloud) and do not write the mesh. A hop id that is not a placement stays the lease stub. `estate convey sync` copies `agents` from `placement-actual.json` onto the matching hop. A lease that lists an agent the placement row does not is `refuse:agent-unplaced` and writes nothing. `estate convey call --agent <id>` refuses when that agent is not on the lease (`refuse:agent-unbound`). When the agent is on the lease, the call checks the estate intention for the capability (`refuse:intention` on deny). That check runs only if something invokes the call. `estate convey authority` is a file check: `would-allow`, `would-deny`, or `not-enforced`. A granted box lease whose hop coverage is deny, deny-default, or a capability mismatch is `would-deny` and cites that hop coverage. A match continues to the estate intention. A hop id that is not a placement stays the lease stub. It does not write, and it does not show that a worker called the conveyor. `estate plan` lists desired rows as not-enforced. Whether a hop lease is the right shape for authority next to the workload is still uncertain. It is not an identity lookup. The identity equation stays parked. Not a gateway.

### control

`estate-control`. Validate, plan, apply, drift, compile intentions. Charter sentence: control does not complete. Completion runs on the data plane (`model-estate`). `estate specialist` is a thin delegate into that plane.

## Suite

Next-generation harness and custom AI creator suite. Pillars, in order:

1. Agents. Spin up and run agents as first-class under security-as-IaC. Sacred isolation, plan/apply, fail closed.
2. Harness. Grok Bot–like look and feel. The UI may still be deferred.
3. On-spot specialty SLMs. Create, train, enrich, and seat purpose-built small models when the work needs them. One facet. A local-runtime seat in the estate flow. Ollama today. Another process tomorrow. Catalog, route, and bind take the entrant. Facilitate training and enrichment of purpose-built small-parameter models. Open-source SLMs will get more common.
4. Optional later. A central model brain that learns from the user. Parked. Not a commitment.

Beachhead on `main`: the seated drivers, enrich packs, the specialist path, and `TrainEnrichDriver`. `estate enrich prepare` writes artifacts. `llamafactory-lora` facilitates LoRA without quantization by writing a LLaMA-Factory recipe, `llamafactory-qlora` facilitates QLoRA the same way, `axolotl-lora` writes the bf16 Axolotl YAML, and `axolotl-qlora` writes the 4-bit Axolotl YAML; the factory does not run the trainer. Ollama is the local-run seat. That portable seat and facilitation of purpose-built SLMs are one facet of the suite. Integrate the driver that already does the job. `READY_FOR_LIVE_TEST` stays no. Command surface: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md).

## Aliases to avoid

Draft word on the left. Charter term on the right.

| Draft word | Charter term |
| --- | --- |
| gateway, AI gateway, MCP catalog | control does not complete; conveyor allow/deny |
| Ollama wrapper-as-product | local-runtime seat; integrate the driver |
| LM Studio-alone, weight browser, UI-only shell | local-runtime seat inside the estate; harness look and feel stays in the suite |
| thin Grok Bot clone (look without the estate), undirected agent sprawl, computer-use farm | agents under security-as-IaC; lanes Horizon / Research / Sanctum |
| frontier-proxy-only | equal-class frontier and local; `model.local.down` |
| local-studio-only | equal-class bindings; purpose-built SLM stays in the suite |
| capability mesh (as the product) | convey, lease-bound hop stub. Path `.cell/conveyor-mesh.json` stays a filename |
| model factory | Agent Estate Factory |
| SKU in a binding or probe id (`5090`, `4090`, `m3-max`) | `host_class` |
| cloud spawn | placement `cloud-agent`, declared lease |

Suite words, kept: distillation, training path, purpose-built SLM.

Host-class spellings that normalize: `rtx-consumer` / `rtx_consumer` → `consumer-nvidia`; `nvidia-rental` / `nvidia_rental` → `rented-nvidia`.

Sacred spellings that refuse: `cyera` / `cyera_ci` → `cyera-ci`; `rust_classroom` → `rust-classroom`.

## help

On `main`: status, plan, apply, reconcile, feed-loop (`feed`), backup, frontier, day90-mixed (`mixed`), north-star (`northstar`), charter, enrich (`train`). `north-star` and `charter` print one page. `enrich` and `train` print the prepare page. An unknown topic is `refuse:help-topic`. Prepare: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md).

## Planes

Control, data, and feed stay separate. The boundary table is in [`../charter.md`](../charter.md). Control to data is apply. Data to control is drift. Data to feed is scrubbed events. Feed to control is an explicit pack import. Feed to data is empty in Cell One.
