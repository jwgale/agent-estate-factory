# Day-90 gate (local only)

A10–A12 plus overnight waves are **on `main`** (PR #1 Day 0–60, PR #2 Day 61–90, PR #3 live probes + `make day90`, PR #4 heal/accept). Hosted CI is compile-only (`cargo check --workspace --locked` on `pull_request`). Real cargo test stays local.

```bash
make smoke      # doctor + fixtures-check + operator-day + cargo test + day90
make day90      # status → plan → dry-run → apply → reconcile → --suggest
make gate-90    # smoke (includes day90) + doctor --strict + this checklist
estate doctor --strict
```

## Green on main / this slice

| Item | Status | Local command |
| --- | --- | --- |
| A10 feed pack / import gate / no auto-promote | green | `make operator-day` |
| A10 estate unchanged on import/propose | green | operator-day cksum |
| A11 suspend / resume / pause-kit | green | `estate suspend` / `resume` / `pause-proof` |
| A12 plan / gated apply / cloud-agent stub | green | `estate plan` / `status` |
| Waves 2–8 (convey, dry-run, doctor, smoke, sacred file, mixed proof) | green | `make smoke` |
| Day 90+ operator loop | green | `make day90` |
| Live probes SKIP without endpoints | green | `estate probes --live` |
| Probe ids refuse SKUs | green | fixtures-check + unit tests |
| Compile-only CI | green | `.github/workflows/ci.yml` |
| `estate doctor --strict` | green | pre-merge operator checks |
| Dual-layer sacred demo (Sanctum is not Cyera) | green | `examples/fixtures/dual-layer-demo.yaml` |
| Omit-locked sacred file still refuses Cyera CI | green | `--sacred sacred-omit-locked.yaml` |
| Sanctum-as-Cyera display-name bleed | green | `refuse-sanctum-as-cyera.yaml` |
| Thin `make gate-90` | green | smoke + day90 + checklist print |

## Remaining Day-90+ (honest; needs live box or a lock)

| Item | State |
| --- | --- |
| Live Mac MLX | Probe path exists (`CELL_MLX_ENDPOINT`). No Mac in CI. Still a stub runtime. |
| Live rented / consumer GPU | Same specialist protocol (`CELL_LOCAL_ENDPOINT` / `CELL_RENTED_ENDPOINT`). Not required in CI. |
| `estate reconcile --suggest` | Patch file only. Jason still applies by hand. Not an auto-heal. |
| `estate packs accept` | Writes enrich_packs **edit instructions**. Does not rewrite `estate.yaml`. Needs `--curator jason`. |
| Cloud-agent spawn | Locked off. Declared only. |
| Convey hop transport | Lease-bound mesh, not a gateway. |
| Auto-promote / curator UI | Locked off / not built. |
| vLLM / TRT | Experimental catalog cards until Jason verifies. |
| Actions | One compile-only job forever unless Jason expands it. |

Fail closed: sacred exclusions, SKU in ids (including probe ids), unknown apiVersion/kind, missing local (no frontier fallback), cloud-agent spawn, auto-promote, Sanctum-as-Cyera bleed, omit-locked sacred file.

Live Grok / GPU are not required to keep `make smoke` / `make gate-90` green.
