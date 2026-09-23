# Cell One — Agent Estate Factory charter

Status: Day 0–90 **on main** (A1–A4 locked, A5–A9 locked, Day 61–90 beachhead toward A10–A12). Day 90+ is **real-world proof** plus parked stubs, and a train/enrich facilitation beachhead (`TrainEnrichDriver` prepares artifacts; it does not run a trainer). Edits to this file are how defaults change. Factory altitude, not a product spine. Locked SLM runtime defaults below are unchanged.

Schema (desired-state shape): [`schema/estate.v0.schema.json`](schema/estate.v0.schema.json)  
Example estate: [`examples/estate.yaml`](examples/estate.yaml)  
Fail-closed validator: Rust `estate-schema` (JSON Schema is documentary).

## Locked defaults

1. **Product:** Agent Estate Factory. One-box Cell One. Source of truth: [github.com/jwgale/agent-estate-factory](https://github.com/jwgale/agent-estate-factory).
2. **Lanes:** Horizon / Research / Sanctum are separate. Sanctum is not Cyera. Rust classroom is not an estate lane.
3. **Sacred exclusions (dual-layer through Day 60):** Cyera CI and Rust classroom must never appear as estate agents. The estate declares them; the validator also hard-denies their ids and aliases. An allow intention cannot punch through.
4. **Models (equal class):** `frontier` and `local` bindings in the same estate. Day 31–60 wires `xai_grok` (frontier-http) and portable `local_slm` (driver `ollama`) with `wired: true` behind swappable traits. Credentials via env (`XAI_API_KEY`, `CELL_LOCAL_ENDPOINT`). Never bake secrets. Never put vendor strings or hardware SKUs in floor-supervisor sources or estate binding ids.
5. **SLM / local-runtime locks (do not reopen):**
   1. Ollama-first; llama.cpp is swap-proof; vLLM is optional — not required for the first green demo.
   2. Local process on a host + `CELL_LOCAL_ENDPOINT` remote pattern for other machines.
   3. Fail closed for estate-bound local work when local is down. No silent frontier fallback. Audit the deny (`model.local.down`).
   4. Jason curates the first specialist enrich packs. Policy is **manual**. Feed does not auto-promote.
   5. “Supported” = the Ollama (+ llama.cpp) path is green on the box. vLLM / TRT stay experimental until Jason verifies.
6. **Portability (critical):** The factory must work equally via drivers on (a) consumer-grade RTX, (b) Apple Silicon laptop, (c) rented latest Nvidia. Hardware is a **driver choice**, not a product fork. Estate contracts use `host_class`: `consumer-nvidia` | `apple-silicon` | `rented-nvidia` | `any`. Do not encode `5090` / `4090` / `m3-max` in binding ids or drivers. Apple path: Ollama-on-Mac is Supported (same `ollama` card); MLX is a Stub behind the same catalog / route / bind API (live Mac proof may come later).
7. **Intentions:** Deny-default for tool, MCP, mount, model, and cross-lane memory. Own-lane memory read is allowed. Model use is a declared allow-list on the agent (`models:`), same class as tools.
8. **Mixed path:** authorize (A3–A4) → local specialist (A8, policy-precheck) → tool or frontier (A7). Data plane (`model-estate`) only. Control does not complete. Keep A7–A9 thin: Grok + local endpoint drivers in the estate registry. Do not turn Cell One into LM Studio.
9. **Isolation:** Swappable `IsolationDriver`. Cell One ships a profile-dir driver. Floor core does not hard-code vendor ids.
10. **Pause-safe SoT:** charter, estate file, schema, lane roots, `plans/`, gate reports. Disposable: PIDs, warm desktops/session dirs, caches. Apply writes regenerable `actual-state.json`, `desired-snapshot.yaml`, `model-actual.json`, `placement-actual.json`. Operator lifecycle (`estate suspend` / `resume`) writes durable `.cell/lifecycle.json` + `lifecycle.jsonl` — not estate SoT. Apply may be gated on a covering plan (`--require-plan` / `--require-fresh-plan`) and is audited.
11. **Language:** Rust default on the hot path (conveyor allow/deny, isolation, supervisor core, mixed-path authorize). Escape hatches allowed. Not forever-Rust. Model drivers are traits; the local specialist process may be any language.
12. **Day 61–90 beachhead (toward A10–A12, not a full workday):** feed materializes candidate enrich packs (manual, no auto-promote). Operator suspend/resume is file-durable. Plans are the human control surface (PR-reviewable blast-radius). `placements[]` declares `box` and a `cloud-agent` stub so Day-90 operator day is not schema-blocked. Floor does not spawn cloud agents. Wave 3 adds `estate reconcile`, `estate packs propose` (never applied), multi-host fixture, and local `estate audit export`. Overnight assumptions: [`docs/overnight-decisions.md`](docs/overnight-decisions.md).
13. **Day 90+ train/enrich beachhead:** `TrainEnrichDriver` lives in the data plane (`model-estate`). `estate enrich prepare` writes artifacts. `--all-drivers` writes every registered card into sibling directories. `estate enrich list` reads `.cell/enrich` and refuses when that directory is missing. `estate enrich import-prepared` writes a proposal for the existing `local_slm` seat and does not apply. `estate enrich apply-proposal` stages that proposal. The source estate is written only when `estate apply --require-plan` succeeds on the staged file. `ollama-modelfile` joins the seated Ollama runtime (`ollama create`, Modelfile `FROM` + `SYSTEM`) without shelling out. `FROM` is the seated model (`params.model` on the local binding, or a pack `model_hint` that is already a model tag). The binding id is not a `FROM`. `estate enrich from-pack` prepares an accepted pack into `.cell/enrich`. `external-manifest` is a portable JSON/YAML hatch for a later trainer. `llamafactory-qlora` writes a LLaMA-Factory QLoRA recipe (`recipe.yaml`, instruct chat `dataset.jsonl`, and the exact `llamafactory-cli train` line) and does not run LLaMA-Factory. `model_name_or_path` is a train base the operator sets (Hugging Face repo id or local HF weights). The Ollama seat tag stays Modelfile `FROM`. A bare seat tag is `refuse:train-base`. `axolotl-lora` writes an Axolotl YAML recipe. `base_model` is that same train base. The card does not run Axolotl. `dataset.jsonl` on both train cards stays a scaffold unless `--from-feed` copies rows already under the cell state directory. A missing file is `refuse:dataset`. The factory does not download pack sources. Unsloth stays a `NEXT.md` pointer, not a card. `estate enrich import-trained` records the adapter or GGUF on the same `local_slm` proposal. A later entrant is another catalog card. This does not run LoRA/SFT/DPO, does not POST a train job, and does not auto-promote. Prepare, import, and apply-proposal leave the source estate unchanged. Locked SLM runtime rules above stay closed. Prepare: [`docs/TRAIN-ENRICH.md`](docs/TRAIN-ENRICH.md). Walks: [`docs/operator-enrich-journeys.md`](docs/operator-enrich-journeys.md). Ollama stays the local-run seat. The suite is that portable seat and facilitation of purpose-built SLMs. Integrate the driver that already does the job.

## Flexibility (must survive)

- Swap isolation / frontier / local drivers without rewriting floor core.
- Add or rename lanes in the estate file; do not bake lane names into binaries except Cell One example fixtures and locked sacred names.
- Pause and resume from files. Runtime is regenerable.
- Equal-class model bindings: neither frontier nor local is a sidecar in the schema.
- Compiled intentions are pure functions of the estate (plus hash). No silent policy learning.
- Local specialist endpoint is config (`CELL_LOCAL_ENDPOINT`), not a compiled host.
- Swap the local *runtime* (Ollama ↔ llama.cpp ↔ later MLX) through catalog / route / bind. Do not fork the product per GPU or SoC.
- Swap the train/enrich driver (`ollama-modelfile`, `external-manifest`, `llamafactory-qlora`, `axolotl-lora`, a later entrant) by registering a `TrainEnrichDriver` card. Floor core and control dispatch do not learn the trainer.

## Anti-shrink

Refuse to let this factory become any of:

- a forensics product
- ephemeral IAM
- an eval harness
- a computer-use farm
- an approval-gate product
- an MCP catalog or AI gateway
- Dual PE / vault
- ChatGPT Team + permissions
- a local LLM studio alone
- a multi-provider proxy alone / frontier-proxy-only

Those may exist later as *consumers* of the factory. They are not the factory.

## Non-goals (through Day 60)

- Dual PE, vault, multi-box control plane
- AI-gateway / MCP-catalog product surface (conveyor does not complete)
- Mesh, Kubernetes, frozen public API
- Feed auto-promote (feed is one-way scrubbed traces + candidate packs; Jason edits the estate)
- Treating PIDs or warm desktops as source of truth
- Cyera CI or Rust classroom as agents
- Tool slug taxonomy (still deferred)
- LM Studio / weight browser / chat UI
- Requiring vLLM or TensorRT for a green demo

## Planes (do not collapse)

| Plane | Owns | Must not |
| --- | --- | --- |
| Control (`estate-control`) | validate, plan, apply, drift, compile intentions | execute tools/models; own agent memory |
| Data (`floor-supervisor`, `model-estate`, `conveyor-proxy`, workers) | spawn/bind, deny-default, mixed model path | rewrite the estate file as SoT; silently learn policy; silently fall back to frontier when local is down |
| Feed (`feed-collector`) | append scrubbed traces; materialize candidate packs | block the data plane; auto-promote |

Boundaries: Control→Data = apply; Data→Control = drift/acks; Data→Feed = scrubbed events; Feed→Control = explicit pack import (never silent; never rewrites the estate file); Feed→Data = nothing in Cell One.

## How defaults change

Change this charter, then the example estate and `estate-schema` validator. Do not sneak defaults into floor-supervisor. Do not teach conveyor to complete.
