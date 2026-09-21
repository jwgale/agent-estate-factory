# Cell One — Agent Estate Factory charter

Status: locked defaults for Day 0–30 (A1–A4). Edits to this file are how defaults change. This is factory altitude, not a product spine.

Schema (desired-state shape): [`schema/estate.v0.schema.json`](schema/estate.v0.schema.json)  
Example estate: [`examples/estate.yaml`](examples/estate.yaml)  
Fail-closed validator: Rust `estate-schema` (JSON Schema is documentary).

## Locked defaults

1. **Product:** Agent Estate Factory. One-box Cell One skeleton only.
2. **Lanes:** Horizon / Research / Sanctum are separate. Sanctum is not Cyera. Rust classroom is not an estate lane.
3. **Sacred exclusions:** Cyera CI and Rust classroom must never appear as estate agents. The example estate declares them; the validator also hard-denies their ids and aliases.
4. **Models:** Equal-class `frontier` and `local` bindings in schema. Day 0–30 placeholders only: `xai_grok` (frontier) and `gpu_5090` (local), both `wired: false`. No live provider or 5090 calls.
5. **Intentions:** Deny-default for tool, MCP, mount, and cross-lane memory. Own-lane memory read is allowed so an agent can work. Everything else needs a declared tool/mount/mcp or an allow intention.
6. **Isolation:** Swappable `IsolationDriver`. Cell One ships a profile-dir driver (per-agent session directory). Floor core does not hard-code vendor ids.
7. **Pause-safe SoT:** charter, estate file, schema, lane roots, `plans/`, gate reports. Disposable: PIDs, warm desktops/session dirs, caches.
8. **Language:** Rust default on the hot path (conveyor allow/deny, isolation, supervisor core). Escape hatches allowed. Not forever-Rust. Model drivers are separate processes and language-free.
9. **Day-90 horizon (not this cell):** full multi-agent workday + cloud agents. Do not build A7–A12 here.

## Flexibility (must survive)

- Swap isolation / frontier / local drivers without rewriting floor core.
- Add or rename lanes in the estate file; do not bake lane names into binaries except Cell One example fixtures and locked sacred names.
- Pause and resume from files. Runtime is regenerable.
- Equal-class model bindings: neither frontier nor local is a sidecar in the schema.
- Compiled intentions are pure functions of the estate (plus hash). No silent policy learning.

## Anti-shrink

Refuse to let this factory become any of:

- a forensics product
- ephemeral IAM
- an eval harness
- a computer-use farm
- an approval-gate product
- an MCP catalog or AI gateway
- Dual PE / vault
- ChatGPT Team + permissions
- a local LLM studio alone
- a multi-provider proxy alone

Those may exist later as *consumers* of the factory. They are not the factory.

## Non-goals (Day 0–30)

- Live Grok, live 5090, or any real model completion
- Dual PE, vault, multi-box control plane
- AI-gateway / MCP-catalog product surface
- Mesh, Kubernetes, frozen public API
- Feed auto-promote (feed is a reserved one-way seam only)
- Treating PIDs or warm desktops as source of truth
- Cyera CI or Rust classroom as agents

## Planes (do not collapse)

| Plane | Owns | Must not |
| --- | --- | --- |
| Control (`estate-control`) | validate, plan, apply, drift, compile intentions | execute tools/models; own agent memory |
| Data (`floor-supervisor`, `model-estate`, `conveyor-proxy`, workers) | spawn/bind, deny-default enforcement | rewrite the estate file as SoT; silently learn policy |
| Feed (`feed-collector`) | append scrubbed traces | block the data plane; auto-promote in Cell One |

Boundaries: Control→Data = apply; Data→Control = drift/acks; Data→Feed = scrubbed events; Feed→Control = pack manifests later (never silent); Feed→Data = nothing in Cell One.

## How defaults change

Change this charter, then the example estate and `estate-schema` validator. Do not sneak defaults into floor-supervisor or conveyor-proxy.
