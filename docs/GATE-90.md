# Day-90 gate (local only)

A10–A12 plus overnight waves are **on `main`** (PR #1–#7). Hosted CI is compile-only (`cargo check --workspace --locked` on `pull_request`). Real cargo test stays local. `make gate-90` is local on purpose — it wraps `cargo test --workspace`. See [`OPERATOR-DAY.md`](OPERATOR-DAY.md).

`make gate-90` is the Day-90 operator entrypoint. Live Mac MLX / GPU / cloud-spawn wait in [`DAY90-PLUS.md`](DAY90-PLUS.md).

```bash
make gate-90    # Day-90 operator entrypoint (local)
make smoke      # doctor + fixtures-check + operator-day + cargo test + day90
make day90      # status → plan → dry-run → apply → reconcile → --suggest
make feed-loop  # scrubbed trace → pack → propose → accept (not in smoke)
estate help     # Day-90 topic pages
estate doctor --strict
# walk without live boxes: docs/OPERATOR-DAY.md
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
| README Day-90 operator entrypoint | green | leads with `make gate-90` |
| `make feed-loop` | green | scrubbed trace → pack → propose → accept |
| Feed cursor durability | green | schema + packed_id + rematerialize keeps cursor |
| Placement-actual refuse round-trip | green | every reconcile refuse code, schema preserved |
| Honest live-box parking lot | green | [`DAY90-PLUS.md`](DAY90-PLUS.md) |
| `estate help` topic pages | green | `estate help status` / `plan` / `apply` / `reconcile` / `feed-loop` / `backup` |
| `estate backup --prune N` | green | keep newest N cell archives; `N=0` refuses |
| Convey call policy deny | green | `policy-deny.yaml` refuses `convey-call` |
| `.cell/` layout doc matches code | green | [`cell-layout.md`](cell-layout.md) |
| Operator day runbook (no live boxes) | green | [`OPERATOR-DAY.md`](OPERATOR-DAY.md) |
| `make gate-90` stays off Actions | green | wraps `cargo test`; hosted stays compile-only |

## Remaining Day-90+ (honest; parked, not green)

| Item | State |
| --- | --- |
| Live Mac MLX | Parked. See [`DAY90-PLUS.md`](DAY90-PLUS.md). Probe path exists. No Mac in CI. |
| Live rented / consumer GPU | Parked. Same specialist protocol. Not required in CI. |
| Cloud-agent spawn | Parked / locked off. Declared only. Floor does not spawn. |
| `estate reconcile --suggest` | Patch file only. Jason still applies by hand. Not an auto-heal. |
| `estate packs accept` | Writes enrich_packs **edit instructions**. Does not rewrite `estate.yaml`. Needs `--curator jason`. |
| Convey hop transport | Lease-bound mesh, not a gateway. |
| Auto-promote / curator UI | Locked off / not built. |
| vLLM / TRT | Experimental catalog cards until Jason verifies. |
| Actions | One compile-only job forever unless Jason expands it. |

Fail closed: sacred exclusions, SKU in ids (including probe ids), unknown apiVersion/kind, missing local (no frontier fallback), cloud-agent spawn, auto-promote, Sanctum-as-Cyera bleed, omit-locked sacred file.

Live Grok / GPU are not required to keep `make smoke` / `make gate-90` green.
