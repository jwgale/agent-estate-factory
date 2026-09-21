# `.cell/` layout (Wave 3)

Durable operator state lives under `.cell/`. Runtime is regenerable. The estate file is SoT. This is not a gateway cache and not a remote store.

| Path | Kind | Notes |
| --- | --- | --- |
| `actual-state.json` | regenerable | Bound sessions from last apply. |
| `desired-snapshot.yaml` | regenerable | Last applied estate snapshot (plan `against`). |
| `placement-actual.json` | durable lease | Desired vs actual placement leases. `box` may spawn. `cloud-agent` is declared and never spawned. Optional `ttl_secs` / `issued_at` / `expires_at`. |
| `reconcile.json` / `reconcile.md` | regenerable report | `estate reconcile` desired-vs-actual. Refuse codes: `missing-lease`, `extra-lease`, `kind-mismatch`, `host-class-mismatch`, `cloud-spawned`, `sacred-id`. |
| `lifecycle.json` | durable | Operator intent (`running` / `suspended`). Not estate SoT. |
| `lifecycle.jsonl` | durable history | Append-only suspend / resume / apply. |
| `apply-audit.jsonl` | durable | Gated apply history. Cloud-agent spawned is always refuse. |
| `catalog.json` | regenerable | Portable local catalog dump. File SoT is `schema/local-catalog.v0.json`. |
| `model-actual.json` | regenerable | Binding actual after apply. |
| `conveyor-mesh.json` | durable | Capability mesh (not a gateway). |
| `conveyor-hops.json` | durable | Declared hops. |
| `conveyor-leases.json` | durable | Hop leases. Call refuses without a granted lease. |
| `feed/events.jsonl` | durable | Scrubbed traces. No prompts, no keys. |
| `feed/feed-cursor.json` | durable watermark | Survives rematerialize. |
| `sessions/` | disposable | Profile-dir desktops. Discarded on suspend. |
| `runtime/` | disposable | Heartbeats / PIDs. Never SoT. |
| `audit-export/` | local review bundle | `estate audit export`. Plans + lifecycle + import-audit + convey leases. Not uploaded. |

Lease file shape (`placement-actual.json`):

```
schema: cell-one.placement-actual.v0
desired_hash: sha256:…
leases[]:
  placement_id, kind (box|cloud-agent), host_class (canonical),
  agents[], wired, spawned, durable, driver, note,
  ttl_secs?, issued_at?, expires_at?
```

Optional TTL: absent means no expiry. `estate expire` lists elapsed leases. apply/resume refuse them. `estate expire --forget` drops expired rows (does not spawn) so apply can record fresh leases.

`host_class` on disk is the canonical name (`consumer-nvidia` | `apple-silicon` | `rented-nvidia` | `any`). Aliases are normalize-only.

Sacred-id deny stays: an actual lease that binds a sacred exclusion fail-closes even if `estate.yaml` is clean (tamper). Reconcile does not rewrite leases. It reports.

Related drop-zone (not under `.cell/`):

| Path | Kind |
| --- | --- |
| `packs/{id}.pack.json` | Candidate only. curator=jason policy=manual. |
| `packs/accepted/{id}.pack.json` | Explicit import. Does not rewrite the estate. |
| `packs/accepted/import-audit.jsonl` | Import audit. |
| `packs/proposed/{id}.proposal.json` | Enrich proposal. `auto_apply=false`. Jason edits the estate. |
