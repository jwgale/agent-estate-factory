# Contributing to Cell One

Clone from GitHub. That is the only remote that matters.

```bash
git clone https://github.com/jwgale/agent-estate-factory.git
cd agent-estate-factory
```

Rust 1.88 is pinned in `rust-toolchain.toml`.

```bash
cargo test --workspace
make gate      # A1–A4
make gate-60   # A5–A9; live Grok/local SKIP without secrets
make gate-90   # A10–A12 beachhead; local only
```

Hosted CI is compile-only (`cargo check --workspace --locked` on pull requests). Run `cargo test --workspace` and `make gate` / `make gate-60` / `make gate-90` / `make smoke` locally. Do not add `cargo test` or a second workflow.

Open pull requests against `main` on `jwgale/agent-estate-factory`. Cursor cloud agents should start from `repo: https://github.com/jwgale/agent-estate-factory`.

Do not add a second forge remote. Clone and push only this GitHub repository.
