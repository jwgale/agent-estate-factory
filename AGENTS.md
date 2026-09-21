# Cell One — agent notes

GitHub is the only source of truth: https://github.com/jwgale/agent-estate-factory

Launch Cursor cloud agents with `repo: https://github.com/jwgale/agent-estate-factory`. Do not attach to Origin remotes.

## Commands

```bash
cargo test --workspace
make gate
make gate-60
```

Primary gate is local (or this cloud-agent VM). GitHub Actions is one `pull_request` job: `cargo test --workspace` only. Do not add matrices, cron, or `make gate-60` on hosted CI.

`make gate-60` skips live Grok and live local when `XAI_API_KEY` / `CELL_LOCAL_LIVE` are unset. Never bake secrets.

## Product locks

Factory altitude only. Equal-class frontier + local. Sacred exclusions (Cyera CI, Rust classroom) stay dual-layer. Ollama-first, portable `local_slm`, fail closed when local is down. Not a gateway. Not LM Studio.

Details: `charter.md`, `README.md`, `CONTRIBUTING.md`.
