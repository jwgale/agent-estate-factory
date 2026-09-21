# Day-90 beachhead gate (local only)

Checklist for A10–A12 plus overnight waves. **No GitHub Actions.** Run from the repo root.

Primary wrap:

```bash
make smoke    # doctor + fixtures-check + operator-day + cargo test --workspace
```

Pieces:

```bash
cargo test --workspace
make fixtures-check
make operator-day
make gate-90
```

| Item | Pass | Local command |
| --- | --- | --- |
| A10 feed pack | Candidate pack, `promoted=false`, curator=jason | `estate feed pack` / `make operator-day` |
| A10 import gate | `--curator jason` required (or default jason); wrong curator `refuse:curator` | `estate packs import --curator jason …` |
| A10 no auto-promote | `estate feed promote` / `packs promote` exit 1 | operator-day |
| A10 estate unchanged | Import / propose do not rewrite `estate.yaml` | operator-day cksum |
| A11 suspend / resume | `lifecycle.json` durable; `sessions/` discarded | `estate suspend` / `estate resume` |
| A11 pause-kit proof | apply → suspend → drop sessions → resume; leases survive | `estate pause-proof` |
| A12 plan | Reviewable diff + Security-as-IaC | `estate plan` |
| A12 gated apply | `--require-plan` / `--require-fresh-plan` | operator-day |
| A12 cloud-agent stub | Declared; never spawned | `estate status` / `estate leases` |
| Wave 2 convey | Lease-bound call; cloud-mesh refuse | `estate convey sync` / `call` |
| Wave 2 host aliases | rtx-consumer / nvidia-rental map | `estate validate --estate examples/hosts/…` |
| Wave 3 reconcile | Report, not a fixer | `estate reconcile` |
| Wave 3 propose | `auto_apply=false` | `estate packs propose` |
| Wave 4 dry-run | No writes; would-refuse exit 1 | `estate apply --dry-run` |
| Wave 4 lease TTL | expire / refuse / `--forget` | `estate expire` |
| Wave 4 doctor | Schemas present; no workflow yml | `estate doctor` |
| Wave 5 journal | `.cell/sessions.jsonl` | `estate sessions list` |
| Wave 5 hop TTL | expired call refuses | `estate convey expire` |
| Wave 5 plan diff | Wider blast without `--allow-wider` refuses | `estate plan diff` |
| Wave 5 fixtures | happy + each refuse | `make fixtures-check` |
| Wave 6 backup | Timestamped archive; sacred-mismatch refuse | `estate backup` / `restore --dry-run` |
| Wave 6 policy | allow / deny / unknown-action | `estate policy check` |
| Wave 6 catalog caps | streaming / tools / vision / context | `estate catalog` |
| Wave 7 status | one-pager: paused, leases, expired, plan, apply, proposals, policy, doctor | `estate status` |
| Wave 7 convey sync | Placement kinds seed hops; extra hops stay; sacred refuse | `estate convey sync` |
| Schema freeze | Additive ok; rename → v1 | `schema/README.md` |

Fail closed: sacred exclusions, SKU in ids, unknown apiVersion/kind, missing local (no frontier fallback), cloud-agent spawn, auto-promote.

Live Grok / GPU / hosted CI are not required.
