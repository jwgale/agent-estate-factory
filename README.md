# Cell One — Agent Estate Factory

One-box factory. Day 0–30 proves **A1–A4**. Day 31–60 proves **A5–A9** (mixed frontier + local) on the same Horizon / Research / Sanctum estate. Pause-safe. Not Dual PE, not multi-box control, not an AI-gateway product, not a local studio.

**Source of truth:** [github.com/jwgale/agent-estate-factory](https://github.com/jwgale/agent-estate-factory) (private). Future Cursor cloud agents launch with `repo: https://github.com/jwgale/agent-estate-factory`.

```bash
git clone https://github.com/jwgale/agent-estate-factory.git
cd agent-estate-factory
cargo test --workspace
make gate
make gate-60
make gate-90    # local only; not in GitHub Actions
make operator-day   # fixtures only: suspend → plan → apply → feed import → resume
```

Hosted CI is **disabled overnight** (no Actions workflows; no failure emails). Real `cargo test --workspace` and `make gate*` stay local. Re-enable tomorrow as compile-only if Jason wants.

Locked defaults: [`charter.md`](charter.md). Documentary schema: [`schema/estate.v0.schema.json`](schema/estate.v0.schema.json). Fail-closed SoT: the Rust validator. See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`AGENTS.md`](AGENTS.md). Lease files: [`docs/cell-layout.md`](docs/cell-layout.md).

## What works tonight vs stubs

This is a factory, not a gateway, not LM Studio, not a shrink-to-frontier proxy. Origin is not in the loop. GitHub is SoT.

| Works tonight (local cargo) | Still a stub |
| --- | --- |
| `estate validate` on `examples/estate.yaml` + host matrix + `examples/hosts/multi-host.yaml` | Live Mac MLX proof |
| `estate plan` / `apply --require-plan` / `--require-fresh-plan` + Security-as-IaC markdown | Cloud-agent spawn (declared only) |
| `estate suspend` / `resume` / `status` / `history` | Multi-box control plane |
| `estate reconcile` desired vs actual + sacred-id deny on tampered leases | Auto-heal / rewrite of leases |
| `estate packs list\|import\|propose` — propose writes `packs/proposed/`, never applies | Feed auto-promote (locked off) |
| `estate convey hop\|call\|leases` — lease-bound; refuse codes prefixed `refuse:` | Real hop transport |
| `estate audit export` — local folder / optional tarball | Remote audit upload |
| Ollama + `CELL_LOCAL_ENDPOINT` + llama.cpp swap-proof card | vLLM / TRT until Jason verifies |
| Isolation profile-dir + placement leases under `.cell/` | Containers / vendor isolation |

Do not turn this into an AI gateway. Do not auto-promote enrich packs. Do not spawn `cursor-cloud`.
