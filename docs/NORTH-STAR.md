# North star

One-box Agent Estate Factory: plan/apply IaC, sacred isolation (Cyera CI + Rust classroom out; Sanctum is not Cyera), equal-class frontier+local, manual enrich packs.

Charter is the source of truth for locked defaults: [`../charter.md`](../charter.md). This page is the product story. It does not change those defaults.

## What Cell One is

One estate on one box. Horizon, Research, and Sanctum are separate lanes. You plan, then apply. Control does not complete.

Frontier (`xai_grok`) and local (`local_slm`, Ollama-first) are equal class in the same estate. Local work refuses when local is down (`model.local.down`). It does not fall through to frontier.

Sacred exclusions stay dual-layer. Cyera CI and the Rust classroom are not agents. Sanctum is not Cyera.

Packs are manual. The curator is Jason. `estate packs accept` writes curator edit instructions. Accept does not rewrite `examples/estate.yaml`.

Day 0–90 (A1–A4, A5–A9, and the A10–A12 beachhead) is on `main`. Day 90+ is a recorded proof on a real box, plus stubs that stay parked.

## Boundaries

The charter anti-shrink list stands. In estate words:

- A pack is a curator edit instruction. Accept does not rewrite the estate.
- Control does not complete.
- `estate convey` is a lease-bound hop stub. It records a lease. It does not move a hop.
- A `cursor-cloud` placement is a declared lease. Apply does not spawn it.
- Native MLX, vLLM, and TRT stay parked stubs.
- Promote stays refused. There is no curator UI.

## Operator loop

Local gate first. Live steps are opt-in and SKIP-safe.

1. **`make gate-90`** — local entrypoint. Runs smoke (that includes `make day90`), then `estate doctor --strict`, then the checklist. No Mac, no GPU, no API key. Stays off GitHub Actions.
2. **`make day90`** — already inside gate-90. Status, plan, dry-run, apply, reconcile on an isolated cell.
3. **`make feed-loop`** — fixtures only. Scrubbed trace, then pack, propose, and accept. Accept writes curator edit instructions. Not in smoke.
4. **Live probes** — `make real-world` after the gate. It prints the north-star line, runs `cargo check --workspace --locked` (the same check as `make check`), and runs vanilla `estate doctor` on the checkout that holds `examples/estate.yaml`. If `CELL_LOCAL_ENDPOINT` is unset, live probes and the Ollama specialist print SKIP and the command exits 0. If it is set, the same env as [`LIVE-PROBES.md`](LIVE-PROBES.md) runs `estate probes --live` and `estate specialist --driver ollama --prompt "Reply with the single word pong."`.

`make real-world` is not in `make smoke`, `make gate-90`, or GitHub Actions. Hosted CI stays one `pull_request` job: `cargo check --workspace --locked`.

Walk without a box: [`OPERATOR-DAY.md`](OPERATOR-DAY.md). Paste target for a real box: [`LIVE-PROBES.md`](LIVE-PROBES.md). Parking lot: [`DAY90-PLUS.md`](DAY90-PLUS.md).

## How you know real-world testing is ready

Both of these, together:

- The local gate is green: `make gate-90` on the box.
- A live command's output is pasted into [`LIVE-PROBES.md`](LIVE-PROBES.md) from Jason's Mac or Linux box.

A SKIP is not ready. A catalog card is not ready. `READY_FOR_LIVE_TEST` is yes only for a concrete command on that page that has not been pasted yet. Recorded rows stay recorded: frontier `pong`, 5090-class probes, 5090-class specialist `Pong`, Mac `probes --live`, and Mac `estate specialist` `"completion": "Pong"` (reason `compat completion`, MacBook Air, tip `2ab78a4`). `READY_FOR_LIVE_TEST` for that Mac command is no. Native MLX stays a stub.

This page does not invent a proof that was not pasted.
