# Day-30 exit checklist (A1–A4)

No A7–A12. Gate = A1–A4 + pause kit. Run from repo root:

```bash
make gate
# or
./scripts/day30-gate.sh
cargo test --workspace
```

## A1 — ≥3 agents, separate lanes

`estate validate --estate examples/estate.yaml` lists Horizon, Research, Sanctum. Each owns one lane. Invalid shared-lane fixtures exit 1.

## A2 — own desktop / session

`estate apply` (or `floor-supervisor apply`) binds each agent to a profile-dir session. Three `session.json` files, three desktops.

## A3 — memory firewall

`conveyor-proxy check --agent horizon --kind memory_read --object lane:research` denies. Own-lane reads allow. Cyera CI and Rust classroom are not agents and cannot be read.

## A4 — deny-default tools / mounts

Undeclared `shell` / `secrets` deny. Research `notes-append` and mount `notes` allow.

## Pause

`make pause-stop` then `make pause-start`. Lane files persist. Sessions rebind from the estate file. PIDs stay disposable.

## Still out of scope

Live Grok / live local GPU box, Dual PE/vault, multi-box control, AI-gateway product, feed auto-promote.
