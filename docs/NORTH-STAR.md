# North star

One-box Agent Estate Factory: plan/apply IaC, sacred isolation (Cyera CI + Rust classroom out; Sanctum is not Cyera), equal-class frontier+local, manual enrich packs.

Charter is the source of truth for locked defaults: [`../charter.md`](../charter.md). This page is the product story. It does not change those defaults.

## What Cell One is

A factory on one box. The estate declares Horizon, Research, and Sanctum on separate lanes. You plan, then apply. The control plane does not complete a model call.

Frontier (`xai_grok`) and local (`local_slm`, Ollama-first) are equal class in the same estate. Estate-bound local work fail-closes when local is down. It does not silently fall through to frontier.

Sacred exclusions stay dual-layer. Cyera CI and the Rust classroom are not agents. Sanctum is first-class and is not Cyera.

Enrich packs are manual. Jason curates. `estate packs accept` writes edit instructions. Those instructions are not a training set, and accept does not rewrite `examples/estate.yaml` by itself.

Day 0–90 (A1–A4, A5–A9, and the A10–A12 beachhead) is on `main`. Day 90+ is proof on a real box, plus stubs that stay parked.

## What it is not

- A Grok Bot clone
- A training lab, a distillation pipeline, or a purpose-built SLM factory (no LoRA, no dataset build)
- An AI gateway or LM Studio
- Dual PE, a vault, or a multi-box control plane
- Native MLX, vLLM, or TensorRT as the product
- A cloud-agent spawner (`cursor-cloud` is declared, not spawned)
- A real convey hop transport (the mesh is lease-bound)
- Auto-promote or a curator UI

## Operator loop

Local gate first. Live steps are opt-in and SKIP-safe.

1. **`make gate-90`** — local entrypoint. Runs smoke (that includes `make day90`), then `estate doctor --strict`, then the checklist. No Mac, no GPU, no API key. Stays off GitHub Actions.
2. **`make day90`** — already inside gate-90. Status, plan, dry-run, apply, reconcile on an isolated cell.
3. **`make feed-loop`** — fixtures only. Scrubbed trace, then pack, propose, and accept. Edit instructions, not training. Not in smoke.
4. **Live probes** — `make real-world` after the gate. It prints the north-star line, runs `cargo check --workspace --locked` (the same check as `make check`), and runs vanilla `estate doctor` on the checkout that holds `examples/estate.yaml`. If `CELL_LOCAL_ENDPOINT` is unset, live probes and the Ollama specialist print SKIP and the command exits 0. If it is set, the same env as [`LIVE-PROBES.md`](LIVE-PROBES.md) runs `estate probes --live` and `estate specialist --driver ollama --prompt "Reply with the single word pong."`.

`make real-world` is not in `make smoke`, `make gate-90`, or GitHub Actions. Hosted CI stays one `pull_request` job: `cargo check --workspace --locked`.

Walk without a box: [`OPERATOR-DAY.md`](OPERATOR-DAY.md). Paste target for a real box: [`LIVE-PROBES.md`](LIVE-PROBES.md). Parking lot: [`DAY90-PLUS.md`](DAY90-PLUS.md).

## How you know real-world testing is ready

Both of these, together:

- The local gate is green: `make gate-90` on the box.
- A live command's output is pasted into [`LIVE-PROBES.md`](LIVE-PROBES.md) from Jason's Mac or Linux box.

A SKIP is not ready. A catalog card is not ready. `READY_FOR_LIVE_TEST` is yes only for a concrete command on that page that has not been pasted yet. Recorded rows stay recorded: frontier `pong`, 5090-class probes, 5090-class specialist `Pong`, and Mac `probes --live`. Mac `estate specialist` complete stays pending until a completion is pasted. Do not mark it PASS from the other box.

This page does not invent that paste.
