# Ubiquitous language

Canonical Cell One words. Locked defaults stay in [`../charter.md`](../charter.md). `estate help north-star` and `estate help charter` print the product sentence.

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

Curator work on a pack today: edit instructions Jason pastes into the estate. The suite goal is wider: train and distill purpose-built small-parameter models for a specific purpose. No trainer command ships on `main`.

### purpose-built SLM

A small-parameter model crafted for one job. The suite trains and enriches these models. The outcome belongs to the estate.

### frontier

Equal-class model binding. Cell One id `xai_grok`. Driver `frontier-http`. Live model `grok-4.7` when `XAI_API_KEY` is set. The binding model is `params.model` on the estate. The catalog schema card prints `grok-4.7`. `CELL_FRONTIER_MODEL` names the live POST. Three fields. A cell catalog that disagrees with the binding is `refuse:frontier-model`.

### local

Equal-class model binding. Cell One id `local_slm`. Class `local`. The binding is the estate contract. The process behind it is a local runtime.

### local runtime

Software that runs a model on a host. Drivers on the estate flow: `ollama`, `llama.cpp`, later others. `ollama` is the first supported driver. llama.cpp swaps on the same specialist protocol. Endpoint: `CELL_LOCAL_ENDPOINT`.

Ollama is local-run software. It lets a user run models. It is an example implementation of a local runtime. The estate is the product. The binding id stays `local_slm`.

Supported means `ollama` and llama.cpp are green on the box. vLLM and TRT stay experimental catalog cards. Native MLX stays a stub. Apple path: Ollama-on-Mac uses the same `ollama` driver.

Local down audits `model.local.down`. Estate-bound local work stops.

### sacred

Dual-layer exclusions. Hardcoded `cyera-ci` and `rust-classroom` always apply. `policy/sacred.yaml` overlays add names (`lab-notebook` in fixtures). A file cannot drop a locked id. An allow intention cannot punch through.

### refuse

Fail-closed stop. CLI and drift use the `refuse:` prefix. Examples: `refuse:help-topic`, `refuse:curator`, `refuse:frontier-model`, `refuse:frontier-invent`, `refuse:sacred-id`, `refuse:cloud-spawned`, `model.local.down`. Unknown actions, hardware SKUs in binding or probe ids, and sacred names stop before the write or the POST.

### placement

Where agents run. `box` is this Cell One. `cloud-agent` (`cursor-cloud`) is a declared lease. Floor records the lease. Spawn of that cloud lease stays parked. `host_class` is `consumer-nvidia` | `apple-silicon` | `rented-nvidia` | `any`. Hardware is a driver choice.

### convey

`estate convey` is a lease-bound hop stub. Verbs: hop, call, list, leases, sync, expire. Files: `.cell/conveyor-mesh.json`, `conveyor-hops.json`, `conveyor-leases.json`. Call requires a granted lease. A spawned cloud hop lease refuses before JSON, restamp, or forget. Hop transport stays parked.

### control

`estate-control`. Validate, plan, apply, drift, compile intentions. Charter sentence: control does not complete. Completion runs on the data plane (`model-estate`). `estate specialist` is a thin delegate into that plane.

## Suite

Both are first-class:

1. Incorporate Ollama-like local runtimes into the estate flow. Portable drivers: `ollama`, `llama.cpp`, later others.
2. Train and enrich purpose-built small-parameter models crafted for a specific purpose.

Shipped today: wired local runtimes, and manual curator packs. Train and distill stay the suite direction.

## Aliases to avoid

Draft word on the left. Charter term on the right.

| Draft word | Charter term |
| --- | --- |
| gateway, AI gateway, MCP catalog | control does not complete; conveyor allow/deny |
| Ollama wrapper, Ollama-only studio, LM Studio, chat UI, weight browser | local runtime driver inside the estate |
| Grok Bot clone, agent farm, computer-use farm | estate; lanes Horizon / Research / Sanctum |
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

On `main`: status, plan, apply, reconcile, feed-loop (`feed`), backup, frontier, day90-mixed (`mixed`), north-star (`northstar`), charter. `north-star` and `charter` print one page. An unknown topic is `refuse:help-topic`.

## Planes

Control, data, and feed stay separate. The boundary table is in [`../charter.md`](../charter.md). Control to data is apply. Data to control is drift. Data to feed is scrubbed events. Feed to control is an explicit pack import. Feed to data is empty in Cell One.
