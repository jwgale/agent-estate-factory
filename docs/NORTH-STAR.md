# North star

One-box Agent Estate Factory: plan/apply IaC, sacred isolation (Cyera CI + Rust classroom out; Sanctum is not Cyera), equal-class frontier+local, manual enrich packs.

Locked defaults: [`../charter.md`](../charter.md). Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md). `estate help north-star` prints this sentence.

## Cell One

One estate. One box. Lanes: Horizon, Research, Sanctum. You plan, then apply. Control does not complete.

Frontier (`xai_grok`) and local (`local_slm`) are equal-class bindings. The local runtime is an ecosystem seat. Ollama is today's entrant. llama.cpp swaps on the same specialist protocol. Local down audits `model.local.down` and stops.

Sacred exclusions stay dual-layer. Cyera CI and the Rust classroom stay out. Sanctum stays a lane.

## Suite

Both jobs are first-class.

1. A local runtime is an ecosystem seat. Ollama fills it today. Another process can fill it tomorrow. Catalog, route, and bind take any entrant. Integrate that driver.
2. Facilitate training and enrichment of purpose-built small-parameter models. Open-source SLMs will get more common.

Today's beachhead is curator packs, the specialist path, and `TrainEnrichDriver`. `estate enrich prepare` writes artifacts for the seated runtime (Ollama Modelfile today) and a portable manifest for a later trainer. `--all-drivers` writes both. `estate enrich from-pack` prepares an accepted pack. Modelfile `FROM` is the seated model (`params.model` or a model-tag hint), never the binding id `local_slm`. `estate enrich list` reads `.cell/enrich`. `estate enrich import-prepared` writes a `local_slm` binding proposal. `estate enrich apply-proposal` stages it for `estate plan` and `estate apply --require-plan`. The source estate is written when that apply succeeds. The curator is Jason. `estate packs accept` writes curator edit instructions. Jason pastes them into the estate file. Promote stays refused. No GPU training stack ships on `main`. Prepare: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md).

Build rule: a feature earns its keep. If `ollama` or llama.cpp already does the job, tighten that integration.

Day 0–90 (A1–A4, A5–A9, and the A10–A12 beachhead) is on `main`. Day 90+ is a recorded proof on a real box, plus parked stubs.

## Boundaries

- Control does not complete.
- `estate convey` is a lease-bound hop stub.
- A `cursor-cloud` placement is a declared lease. Floor records the lease.
- Native MLX, vLLM, and TRT stay parked catalog cards.
- Anti-shrink: Ollama wrapper-as-product, LM Studio-alone, AI gateway, Grok Bot clone. Also MCP catalog, chat UI, weight browser, agent farm, frontier-proxy-only, local-studio-only.

## Operator loop

Local gate first. Live steps are opt-in. An unset endpoint prints SKIP and exits 0.

1. **`make gate-90`** — local entrypoint. Smoke (includes `make day90`), then `estate doctor --strict`, then the checklist. Off GitHub Actions.
2. **`make day90`** — inside gate-90. Status, plan, dry-run, apply, reconcile on an isolated cell.
3. **`make feed-loop`** — fixtures. Scrubbed trace, then pack, propose, and accept. Accept writes curator edit instructions. Off smoke.
4. **`make real-world`** — after the gate. Prints the north-star line, runs `cargo check --workspace --locked` (same check as `make check`), and runs vanilla `estate doctor` on the checkout that holds `examples/estate.yaml`. With `CELL_LOCAL_ENDPOINT` set, the env in [`LIVE-PROBES.md`](LIVE-PROBES.md) runs `estate probes --live` and `estate specialist --driver ollama --prompt "Reply with the single word pong."`. Unset: those two steps print SKIP and the command exits 0. It points at train/enrich prepare and does not run a train.
5. **`make enrich-prepare`** — opt-in fixture. Example pack, both drivers, list, a binding proposal, then apply-proposal, plan, and require-plan apply on a throwaway lab copy that sets `params.model`. `FROM` is that seated model. `examples/estate.yaml` stays hash-locked. Not a live train. Off smoke, `gate-90`, and Actions. Prepare: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md).
6. **`make enrich-live-prove`** — opt-in seated handoff. `ollama create` when the seat is up, then the tag is removed. Not a factory-wide live test. Off smoke, `gate-90`, and Actions. Paste target: [`LIVE-PROBES.md`](LIVE-PROBES.md).

`make real-world` stays out of `make smoke`, `make gate-90`, and GitHub Actions. Hosted CI is one `pull_request` job: `cargo check --workspace --locked`.

Fixtures: [`OPERATOR-DAY.md`](OPERATOR-DAY.md). Paste target: [`LIVE-PROBES.md`](LIVE-PROBES.md). Parking lot: [`DAY90-PLUS.md`](DAY90-PLUS.md).

## Ready

`make gate-90` green on the box, and the live output already pasted in [`LIVE-PROBES.md`](LIVE-PROBES.md).

`READY_FOR_LIVE_TEST` is yes only for a concrete command on that page with no paste yet. Recorded rows stay recorded: frontier `pong`, 5090-class probes, 5090-class specialist `Pong`, Mac `probes --live`, and Mac `estate specialist` `"completion": "Pong"` (reason `compat completion`, MacBook Air, tip `2ab78a4`). `READY_FOR_LIVE_TEST` for that Mac command is no. Native MLX stays a stub. SKIP exits 0.
