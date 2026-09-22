# Cell One — agent notes

GitHub is the only source of truth: https://github.com/jwgale/agent-estate-factory

Launch Cursor cloud agents with `repo: https://github.com/jwgale/agent-estate-factory`. Do not attach to Origin remotes.

## Commands

```bash
cargo test --workspace
make gate
make gate-60
make gate-90    # local Day 61–90 beachhead; do not add to GHA
```

Primary gate is local (or this cloud-agent VM). Hosted CI is one `pull_request` `cargo check --workspace --locked` job (rustc 1.88, timeout ≤ 10). Real `cargo test` stays local. Do not add more workflows.

`make gate-60` skips live Grok and live local when `XAI_API_KEY` / `CELL_LOCAL_LIVE` are unset. Never bake secrets.

## Product locks

Factory altitude only. Equal-class frontier + local. Sacred exclusions (Cyera CI, Rust classroom) stay dual-layer. Ollama is a local runtime on portable `local_slm` (driver `ollama`; llama.cpp swaps). Fail closed when local is down. Suite goal: those runtimes in the estate flow, and train/enrich of purpose-built small models. Not a gateway. Not LM Studio. Not an Ollama wrapper.

Details: `charter.md`, `README.md`, `docs/UBIQUITOUS_LANGUAGE.md`, `CONTRIBUTING.md`.
