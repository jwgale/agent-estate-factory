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

Factory altitude only. Equal-class frontier + local. Sacred exclusions (Cyera CI, Rust classroom) stay dual-layer. The local runtime is an ecosystem seat on portable `local_slm` (Ollama today; llama.cpp swaps; catalog/route/bind take the next entrant). Fail closed when local is down. Integrate a driver that already does the job. Suite goal: facilitate train/enrich of purpose-built small models. Beachhead: enrich packs, the specialist path, and `TrainEnrichDriver` (`estate enrich prepare` writes artifacts; it does not train). Not a gateway. Not LM Studio-alone. Not an Ollama wrapper-as-product. Not a Grok Bot clone.

Details: `charter.md`, `README.md`, `docs/UBIQUITOUS_LANGUAGE.md`, `CONTRIBUTING.md`.
