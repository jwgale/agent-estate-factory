# North star

Next-generation harness and custom AI creator suite: first-class agents under a security-first control suite, a Grok Bot–like harness, and on-spot specialty SLMs, on one box with sacred isolation (Cyera CI + Rust classroom out; Sanctum is not Cyera) and equal-class frontier+local.

Locked defaults: [`../charter.md`](../charter.md). Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). `estate help north-star` prints this sentence.

Cell One is that suite. Training a specialty model is one facet. Overnight packing may stay SLM-heavy. This page is the vision. It does not rebalance that overnight work.

## Suite

Pillars, in this order:

1. **Agents.** Spin up and run agents as first-class under security-as-IaC. Sacred isolation, plan then apply, fail closed. Control does not complete. Promote stays refused. The curator is Jason.
2. **Harness.** Grok Bot–like look and feel. That is the aspirational product surface. The UI may still be deferred as implementation.
3. **On-spot specialty SLMs.** Create, train, enrich, and seat purpose-built small models when specialty work needs them. One facet.
4. **Optional later.** A central model brain that learns from the user. Parked. Not a commitment to ship now.

## Posture

**Security.** Envision the suite as akin to a giant service mesh for agents. Security-first control keeps agent spin-up, hops, tools, and model seats robust. IaC security controls on that path are a core product surface. The exact control catalog is still being designed. This page states the intent. It does not list that catalog, and it does not ship a mesh. `estate convey` stays a lease-bound hop stub.

**Sanctum credentials.** Whether Sanctum holds credentials is an open design call. This page does not answer yes or no. It does not make Sanctum a credential vault. Sanctum stays a lane. Cyera CI stays out. Sanctum is not Cyera.

**Placement.** Folks may run fully local, bring parts to the cloud, or mix the two by budget and need. Ultimate flexibility of host and placement is a north-star requirement: boxes, rented GPU, frontier, and mixed. Equal-class frontier and local, and `host_class`, already carry that. One estate stays the control plane.

**Horizon.** Keep the planning posture 12–18 months ahead of where the market is going, so the architecture is ready when agent, harness, and security patterns shift. That posture is not a thin clone of today's tools.

## Cell One

One estate. One box. Lanes: Horizon, Research, Sanctum. You plan, then apply. Control does not complete.

Frontier (`xai_grok`) and local (`local_slm`) are equal-class bindings. The local runtime is an ecosystem seat. Ollama is today's entrant. llama.cpp swaps on the same specialist protocol. Local down audits `model.local.down` and stops.

Sacred exclusions stay dual-layer. Cyera CI and the Rust classroom stay out. Sanctum stays a lane.

## Real-world middle layer (host → select → receipt)

The steal from Keel is the decision loop, not the UI and not an ACP workspace: host-prepared candidates, a validated select, bounded execute, and versioned receipts the estate can export and report. That loop sits in the estate control plane. Cursor, Keel, and a Grok Bot–like harness stay swappable seats. The problem this fixes is a trustworthy multi-agent estate: frontier and one or more purpose-built local SLMs share the chain, security fails closed, decisions leave receipts, and any harness is a seat. A Rust specialty SLM is only a probe. Specialty locals are any domain.

On `main` through #281 (`9ff2ed50`), the loop already runs with shipped CLI and schema. Honesty, Authority, Agents, and hop cites (#252–#277) already sit on that same tip. Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md) (decision receipt). The five real-world steps:

1. **Fuel seats.** A purpose-built local SLM joins the chain as a bindable `class: local` id beside `local_slm` and beside frontier `xai_grok` (#278, #280). `estate classify journey` prints `estate enrich import-trained`. `import-trained` writes a `cell-one.enrich-binding-proposal.v0` with `trained_shape` `gguf` and `auto_apply=false`. `estate enrich apply-proposal` stages it. `estate plan` then `estate apply --require-plan` writes the source estate. Import does not apply, does not promote, and does not rewrite the locked example. Agent `models` allow-lists and Model intentions can name one specialty seat. Binding ids still refuse a hardware SKU.

2. **Host prepares candidates.** After coverage and intention gates, the host lists eligible model-binding ids as opaque candidates. Frontier and local stay equal class. Zero eligible ids and two or more eligible ids stay an abstain. One scoped specialty seat is the one selectable id.

3. **Select and validate.** A selector chooses one id or abstains. `{state-dir}/decision-select.json` is an optional hint and is not a grant. A bad hint is `refuse:decision-select` before the check and before the receipt. The host re-validates that choice as `ok`, `stale`, `ineligible`, or `expired`. A fallback id may be recorded. The selector does not grant permission. A fallback is not a grant.

4. **Receipt.** `estate authorize` (#281), `estate convey call` (#279), and `estate complete` append one `cell-one.decision-receipt.v0` line at `{state-dir}/decisions/receipts.jsonl`. Authorize sets `surface=authorize`, `hop_id` to the intention kind, and `capability` to the object. Complete sets `surface=complete`, `hop_id` `model`, and `capability` to the binding complete targeted. Convey-call lines omit `surface` and keep the hop id. `estate convey hop` remains the lease-bound hop stub that names the population. Success prints one `decision receipt:` cite. `estate decisions export` writes JSONL replay cases. `estate decisions report` counts stage, validation, and fallback.

5. **Bounded execute, or fail closed.** Authorize still decides allow or deny. A convey call still needs a granted hop lease and an allow intention. `estate complete` is the thin host → select → receipt operator path that then drives a real complete: it authorizes, delegates to the data-plane driver for the chosen binding (`complete_via_binding`: local Ollama or frontier), and journals `surface=complete`. Control does not invent the text. Omit `--object` when the selector chooses one eligible id. Name `--object` to complete a specific binding; the selector still does not grant. Equal-class abstain without `--object` is `refuse:decision-abstain`. Missing `CELL_LOCAL_ENDPOINT` or `XAI_API_KEY` fail-closes. `estate specialist` stays the unbound delegate and does not journal. Coverage deny, intention deny, `model.local.down`, a placement-actual SKU, a mesh that does not parse, and `refuse:agent-unplaced` stop before a grant. A journal write that fails after commit still prints the allow, deny, or completion JSON. The hop, authorize, or complete exit stands. Authority stays a file check (`would-allow` / `would-deny` / `not-enforced`) and does not claim mediation. Control does not complete. `estate convey` stays a lease-bound hop stub.

### Honest demo (AG News cohesion)

Throwaway estate only: `.cell/cohesion-agnews-20260926/` on the 5090. Not locked `examples/estate.yaml` (cksum `43770130 3391`). Bind `ag_news` with `estate enrich import-trained` → `apply-proposal` → `plan` → `apply --require-plan`. Align throwaway `params.model` to the live Ollama seat `specialist-agnews-all`. Chain seats: `xai_grok` + `local_slm` + `ag_news`. The selector chose the one eligible specialty seat. Frontier and local remain equal class.

Receipts in that throwaway journal:

- `r-1-cb40873d` — `result=ag_news` `validation=ok` `surface=authorize`
- `r-2-a2f05740` — `result=ag_news` `validation=ok` on `estate convey call` (hop `cohesion-agnews-hop`; convey-call lines omit `surface`)

`estate decisions report` after both: receipts=2, validate=2, validation ok=2, fallback none=2. Convey is lease and select. It is not a live model ping. `estate complete` is the one operator path that can add a `surface=complete` line after that select. This page does not invent that live generate. Report track: `cohesion-agnews-20260926.md`. `READY_FOR_LIVE_TEST` stays no. Not a live PASS.

### Still not claimed

- A live generate PASS. This page does not invent a live PASS. `READY_FOR_LIVE_TEST` stays no.
- Keel UI, or an ACP workspace clone
- Promote, auto-apply, or a rewrite of locked `examples/estate.yaml`
- A dual rust_idiom launch
- `estate convey` as more than a lease-bound hop stub

## Beachhead

The beachhead under the suite is still the local seat and purpose-built SLMs. Agents and the harness sit above that work. They are not a later add-on.

1. A local runtime is an ecosystem seat. Ollama fills it today. Another process can fill it tomorrow. Catalog, route, and bind take any entrant. Integrate that driver.
2. Facilitate training and enrichment of purpose-built small-parameter models. Open-source SLMs will get more common.

Today's beachhead is curator packs, the specialist path, and `TrainEnrichDriver`. `estate enrich prepare` writes artifacts for the seated runtime (Ollama Modelfile today) and a portable manifest for a later trainer. `--all-drivers` writes every card the job allows. `estate enrich from-pack` prepares an accepted pack. Modelfile `FROM` is the seated model (`params.model` or a model-tag hint), never the binding id `local_slm`. `estate enrich list` reads `.cell/enrich`. `estate enrich import-prepared` writes a `local_slm` binding proposal. `estate enrich apply-proposal` stages it for `estate plan` and `estate apply --require-plan`. The source estate is written when that apply succeeds. The curator is Jason. `estate packs accept` writes curator edit instructions. Jason pastes them into the estate file. Promote stays refused. Train facilitation on this beachhead is `llamafactory-lora` for a LLaMA-Factory LoRA recipe with no quantization, `llamafactory-qlora` for the QLoRA recipe (train base separate from the Ollama seat tag), `axolotl-lora` for a bf16 Axolotl YAML, and `axolotl-qlora` for a 4-bit Axolotl YAML. `base_model` on both Axolotl cards is that same train base. Prepare writes the config, the operator runs `axolotl train` outside the factory, and `import-trained` returns the adapter through the existing apply-proposal loop. The factory still does not run the trainer. `--from-feed` copies dataset rows that are already under the cell state directory and does not download them. Target C is the Qwen / LLaMA-Factory QLoRA ladder: prepare, the external train and export lines, a printed GGUF convert, a printed Ollama create, and `import-trained`. Target A is the Qwen / LLaMA-Factory LoRA ladder: prepare, the external train and export lines, a printed merge, a printed GGUF convert, a printed Ollama create, and `import-trained`. `make seat-journey` prints the Target C merge, convert, seat, and import lines against fixture stubs. When an SLM fits mid-software-build, or on demand, the same print-only entry is `make purpose-build-journey`. `make purpose-build-journey` is the print-only purpose-build on-demand entry. It runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. It does not train, convert, seat, promote, or apply. It is not in `make smoke`, `make gate-90`, or GitHub Actions. The re-prove card stays `make uniqueness-prove-checklist`. The recorded Target C PASS in [`LIVE-PROBES.md`](LIVE-PROBES.md) stays the only live uniqueness prove. `READY_FOR_LIVE_TEST` stays no. Prepare: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md) sections 15–20.

Build rule: a feature earns its keep. If `ollama` or llama.cpp already does the job, tighten that integration.

Day 0–90 (A1–A4, A5–A9, and the A10–A12 beachhead) is on `main`. Day 90+ is a recorded proof on a real box, plus parked stubs.

## Boundaries

- Control does not complete.
- `estate convey` is a lease-bound hop stub.
- A `cursor-cloud` placement is a declared lease. Floor records the lease.
- Native MLX, vLLM, and TRT stay parked catalog cards.
- Anti-shrink: Ollama wrapper-as-product, LM Studio-alone, AI gateway, a thin Grok Bot clone without the estate. The Grok Bot–like look and feel stays. Also MCP catalog, a UI-only shell, weight browser, undirected agent sprawl, an undirected agent farm, frontier-proxy-only, local-studio-only. Agents and controlled spin-up stay first-class. The mesh-like security posture, the harness, and the creator suite stay the product. An eval harness stays refused. A Sanctum credential vault stays unshipped. The exact IaC control catalog stays undesigned.

## Operator loop

Local gate first. Live steps are opt-in. An unset endpoint prints SKIP and exits 0.

1. **`make gate-90`** — local entrypoint. Smoke (includes `make day90`), then `estate doctor --strict`, then the checklist. Off GitHub Actions.
2. **`make day90`** — inside gate-90. Status, plan, dry-run, apply, reconcile on an isolated cell.
3. **`make feed-loop`** — fixtures. Scrubbed trace, then pack, propose, and accept. Accept writes curator edit instructions. Off smoke.
4. **`make real-world`** — after the gate. Prints the north-star line, runs `cargo check --workspace --locked` (same check as `make check`), and runs vanilla `estate doctor` on the checkout that holds `examples/estate.yaml`. With `CELL_LOCAL_ENDPOINT` set, the env in [`LIVE-PROBES.md`](LIVE-PROBES.md) runs `estate probes --live` and `estate specialist --driver ollama --prompt "Reply with the single word pong."`. Unset: those two steps print SKIP and the command exits 0. It points at train/enrich prepare and does not run a train.
5. **`make enrich-prepare`** — opt-in fixture. Example pack, both drivers, list, a binding proposal, then apply-proposal, plan, and require-plan apply on a throwaway lab copy that sets `params.model`. `FROM` is that seated model. `examples/estate.yaml` stays hash-locked. Not a live train. Off smoke, `gate-90`, and Actions. Prepare: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md).
6. **`make enrich-live-prove`** — opt-in seated handoff. `ollama create` when the seat is up, then the tag is removed. Not a factory-wide live test. Off smoke, `gate-90`, and Actions. Paste target: [`LIVE-PROBES.md`](LIVE-PROBES.md).
7. **`make train-prepare`** — opt-in LLaMA-Factory LoRA and QLoRA recipes and Axolotl LoRA and QLoRA recipes. Writes `recipe.yaml` and `axolotl.yml` on a throwaway copy, checks `prepare.json` `job=train`, and leaves `examples/estate.yaml` alone. Prints `SKIP live train`. Does not run either trainer. Off smoke, `gate-90`, and Actions.
8. **`make purpose-build-journey`** — when an SLM fits mid-software-build, or on demand, this is the same print-only entry. Print-only purpose-build on demand. Runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. Does not train, convert, seat, promote, or apply. `READY_FOR_LIVE_TEST` stays no. The re-prove card stays `make uniqueness-prove-checklist`. The recorded Target C PASS in [`LIVE-PROBES.md`](LIVE-PROBES.md) stays the only live uniqueness prove. Off smoke, `gate-90`, and Actions. Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md) sections 15–20.

`make real-world` stays out of `make smoke`, `make gate-90`, and GitHub Actions. Hosted CI is one `pull_request` job: `cargo check --workspace --locked`.

Fixtures: [`OPERATOR-DAY.md`](OPERATOR-DAY.md). Paste target: [`LIVE-PROBES.md`](LIVE-PROBES.md). Parking lot: [`DAY90-PLUS.md`](DAY90-PLUS.md).

## Ready

`make gate-90` green on the box, and the live output already pasted in [`LIVE-PROBES.md`](LIVE-PROBES.md).

`READY_FOR_LIVE_TEST` is yes only for a concrete command on that page with no paste yet. Recorded rows stay recorded: frontier `pong`, 5090-class probes, 5090-class specialist `Pong`, Mac `probes --live`, and Mac `estate specialist` `"completion": "Pong"` (reason `compat completion`, MacBook Air, tip `2ab78a4`). `READY_FOR_LIVE_TEST` for that Mac command is no. Native MLX stays a stub. SKIP exits 0.
