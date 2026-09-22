# Day 90+ — parked until Jason has boxes

Real-world testing is a recorded live proof on Jason's boxes plus a green
local gate (`make gate-90`). An AI gateway stays out of altitude. Facilitation of train and
enrich for purpose-built small models is the suite goal
([`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md)). Today's beachhead is packs,
the specialist path, and `estate enrich prepare` (artifacts only; `--all-drivers`, `from-pack`, seated Modelfile `FROM`, `axolotl-lora` writes an Axolotl recipe and does not run it, `estate enrich list`, `import-prepared` and `import-trained` as a `local_slm` proposal, and `apply-proposal` staging that proposal for `estate plan` / `estate apply --require-plan`).
`make enrich-prepare` is opt-in and not in smoke or `gate-90`. The opt-in ladder after the gate is
`make real-world`. It is not in smoke or Actions. Unset
`CELL_LOCAL_ENDPOINT` prints SKIP and exits 0. That SKIP is not a proof.

This is a parking lot, not a progress report. Nothing in the live-box list
below is green. Do not treat probe paths, catalog cards, or env-var names as
proof that a Mac, a GPU, or a cloud spawn ran.

`make gate-90` is the Day-90 operator entrypoint. It stays local. Hosted CI
stays compile-only. Live `grok-4.7` / GPU / Mac are not required to keep
`make smoke` / `make gate-90` green. Product story: [`NORTH-STAR.md`](NORTH-STAR.md).

## Green without a box

These stay green on this factory. They do not prove a Mac, a GPU, or a spawn.

| Item | What is true |
| --- | --- |
| `make gate-90` | Local only: smoke, `day90`, `doctor --strict`, checklist. Not on Actions. |
| `make day90-mixed` | Opt-in fixture walk. Not in smoke or `gate-90`. No live key. |
| `make feed-loop` | Fixture walk. Pack `source_drivers` is `frontier` then `local`. No live key. |
| `make enrich-prepare` | Opt-in fixture. Writes a Modelfile (`FROM` is the seated model on a throwaway estate copy) and an external manifest, then stages the join and applies it with `--require-plan` on that copy. Does not train. Not in smoke or `gate-90`. |
| `make enrich-live-prove` | Opt-in seated handoff when Ollama is up. Not a factory-wide live test. Not in smoke or `gate-90`. Seat down prints SKIP. |
| Frontier specialist in tests | Mock HTTP. Model id `grok-4.7`. Missing key and a SKU model refuse before POST and name `CELL_FRONTIER_MODEL` / `CELL_FRONTIER_ENDPOINT`. |
| `estate help frontier` / `day90-mixed` | Topic pages. Not a live run. |

## Recorded live proofs (not a gate)

These already ran. Do not re-ping. They are not required to keep `make gate-90` green. Details: [`LIVE-PROBES.md`](LIVE-PROBES.md).

| Item | What is true | What is not true |
| --- | --- | --- |
| Frontier `grok-4.7` | `completion` `pong`, reason `frontier completion`. Key never printed. | `READY_FOR_LIVE_TEST` is no. CI has no key. |
| 5090-class GPU | `probes --live` PASS and `estate specialist --driver ollama` `Pong` are recorded. Host class is `consumer-nvidia` or `rented-nvidia`. | Not native MLX. Not a binding id. Not required for the gate. |
| Mac probes | Ollama-on-Mac `probes --live` PASS. | That GET is not the specialist complete. |
| Mac specialist | MacBook Air `estate specialist --driver ollama` `"completion": "Pong"`, reason `compat completion`. Tip `2ab78a4`. | Native MLX stays stub. `READY_FOR_LIVE_TEST` is no. |

## Parked — still not green

`READY_FOR_LIVE_TEST` is yes only for a concrete command in
[`LIVE-PROBES.md`](LIVE-PROBES.md) that Jason has not run. Every recorded
proof and every parked row on this page is no.

| Item | What exists today | What is not true |
| --- | --- | --- |
| Native MLX | Catalog card is a stub. `CELL_MLX_ENDPOINT` is only an OpenAI-compatible server, if you set it. | Native `specialist()` is not Supported. No Mac in CI. Not live-ok. |
| vLLM | Experimental catalog card. Unset endpoint is SKIP. | Not live-ok. `READY_FOR_LIVE_TEST` is no. |
| TRT | Experimental catalog card. Unset endpoint is SKIP. | Not live-ok. `READY_FOR_LIVE_TEST` is no. |
| Cloud-agent spawn | `cursor-cloud` is declared. Floor records a lease. `estate status` says declared, not spawned. | Floor does not spawn. Locked off until Jason accepts a spawn driver. |

Exact env vars and commands: [`LIVE-PROBES.md`](LIVE-PROBES.md).
Adapter is on `main`. `estate specialist --driver ollama --prompt` is
the live complete verb (same `HttpLocal` helper). This factory VM still
has no Mac and no GPU. Jason's Mac PASSed Ollama `probes --live` and
Mac specialist `Pong` (`compat completion`, tip `2ab78a4`). His 5090
PASSed probes and specialist `Pong`. Native MLX `specialist()` stays stub.

When Jason has an Apple Silicon box: Ollama-on-Mac is the Supported path.
MLX stays a stub. `estate probes --live` does not print `live ok` for
mlx, vllm, or trt, even when `CELL_MLX_ENDPOINT` or `CELL_LOCAL_ENDPOINT`
is set. Supported cards SKIP until an endpoint is set.

When Jason has a consumer RTX or a rented Nvidia box: set
`CELL_LOCAL_ENDPOINT` or `CELL_RENTED_ENDPOINT` and run `estate probes --live`.
Do not put `5090`, `4090`, or `m3-max` in estate binding ids or probe ids.

When Jason is ready for cloud spawn: that is a later lock. This slice does
not invent a spawn driver.

## Not parked — already on main, still honest

| Item | State |
| --- | --- |
| `make gate-90` | Thin local alias: smoke (includes `day90`) + `doctor --strict` + GATE-90 checklist. |
| `make day90-mixed` | Fixture walk: mixed frontier+local plan → `apply --require-plan`. Not a live box. Not in smoke. |
| `make feed-loop` | Fixture walk: scrubbed trace → pack (`source_drivers` frontier+local) → propose → accept. No live keys. See [`FEED-LOOP.md`](FEED-LOOP.md). |
| `estate doctor --strict` | Pre-merge operator checks. Vanilla `doctor` unchanged. |
| `estate reconcile --suggest` | Patch file only. Jason still applies by hand. |
| `estate packs accept` | Enrich-pack edit instructions. Does not rewrite `estate.yaml`. Needs `--curator jason`. |
| Compile-only CI | One `pull_request` job. `cargo check --workspace --locked`. No `cargo test` on Actions. |
| Dual-layer sacred | Locked Cyera CI + Rust classroom. Sanctum is not Cyera. |
| `estate help` | Topic pages for the Day-90 loop, including `frontier` and `day90-mixed`. Not a studio. |
| `estate backup --prune N` | Local rotate. Not a remote vault. |
| Operator day runbook | [`OPERATOR-DAY.md`](OPERATOR-DAY.md). Walk only. Not a live-box proof. |
| `make gate-90` on Actions | Not green and not planned. It wraps `cargo test --workspace`. |

## Rails

GitHub is source of truth. No Origin. No auto-promote. Cloud never spawned.
Skinny CI. No fake live-box checkmarks.
