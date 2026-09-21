# Day 90+ — parked until Jason has boxes

This is a parking lot, not a progress report. Nothing in the live-box list
below is green. Do not treat probe paths, catalog cards, or env-var names as
proof that a Mac, a GPU, or a cloud spawn ran.

`make gate-90` is the Day-90 operator entrypoint. It stays local. Hosted CI
stays compile-only. Live `grok-4.7` / GPU / Mac are not required to keep
`make smoke` / `make gate-90` green.

## Parked — needs a box or a Jason lock

| Item | What exists today | What is not true |
| --- | --- | --- |
| Live Mac MLX | Probe path (`CELL_MLX_ENDPOINT`, falls back to `CELL_LOCAL_ENDPOINT`). Catalog card is a stub behind the same catalog / route / bind API. | No Mac in CI. No Mac attached to this factory. MLX is not Supported. Do not mark it live. |
| Live consumer / rented GPU | Same specialist protocol. 5090 `probes --live` and `estate specialist` `Pong` are recorded in [`LIVE-PROBES.md`](LIVE-PROBES.md). Host class is `consumer-nvidia` or `rented-nvidia`. | Not required for local gates. Not native MLX. A 5090 is one host, not a binding id or product fork. |
| Cloud-agent spawn | `cursor-cloud` is declared. Floor records a lease. `estate status` says declared, not spawned. | Floor does not spawn. Cloud agents are locked off until Jason assigns agents and accepts a spawn driver. |
| Frontier specialist (`grok-4.7`) | Live PASS recorded (`completion` `pong`, reason `frontier completion`). Key never printed. Mixed plan + apply is mock-locked. Catalog card names `grok-4.7`. | Not required in CI. `READY_FOR_LIVE_TEST` no. Local specialist does not fall through. |

Exact env vars and commands: [`LIVE-PROBES.md`](LIVE-PROBES.md).
Adapter is on `main`. `estate specialist --driver ollama --prompt` is
the live complete verb (same `HttpLocal` helper). This factory VM still
has no Mac and no GPU. Jason's Mac PASSed Ollama `probes --live`. His
5090 PASSed probes and specialist `Pong`. Native MLX `specialist()`
stays stub. Mac specialist chat is not recorded.

When Jason has an Apple Silicon box: Ollama-on-Mac is the Supported path;
MLX stays a stub until he verifies it. Set `CELL_MLX_ENDPOINT` (or
`CELL_LOCAL_ENDPOINT`) and run `estate probes --live`. Until then: SKIP.

When Jason has a consumer RTX or a rented Nvidia box: set
`CELL_LOCAL_ENDPOINT` or `CELL_RENTED_ENDPOINT` and run `estate probes --live`.
Do not put `5090`, `4090`, or `m3-max` in estate binding ids or probe ids.

When Jason is ready for cloud spawn: that is a later lock. This slice does
not invent a spawn driver.

## Not parked — already on main, still honest

| Item | State |
| --- | --- |
| `make gate-90` | Thin local alias: smoke (includes `day90`) + `doctor --strict` + GATE-90 checklist. |
| `make feed-loop` | Fixture walk: scrubbed trace → pack → propose → accept. See [`FEED-LOOP.md`](FEED-LOOP.md). |
| `estate doctor --strict` | Pre-merge operator checks. Vanilla `doctor` unchanged. |
| `estate reconcile --suggest` | Patch file only. Jason still applies by hand. |
| `estate packs accept` | Enrich-pack edit instructions. Does not rewrite `estate.yaml`. Needs `--curator jason`. |
| Compile-only CI | One `pull_request` job. `cargo check --workspace --locked`. No `cargo test` on Actions. |
| Dual-layer sacred | Locked Cyera CI + Rust classroom. Sanctum is not Cyera. |
| `estate help` | Topic pages for the Day-90 loop. Not a studio. |
| `estate backup --prune N` | Local rotate. Not a remote vault. |
| Operator day runbook | [`OPERATOR-DAY.md`](OPERATOR-DAY.md). Walk only. Not a live-box proof. |
| `make gate-90` on Actions | Not green and not planned. It wraps `cargo test --workspace`. |

## Rails

GitHub is source of truth. No Origin. No auto-promote. Cloud never spawned.
Skinny CI. No fake live-box checkmarks.
