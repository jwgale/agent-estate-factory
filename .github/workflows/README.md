# Workflows

**Overnight quiet hours:** no GitHub Actions workflows are registered.

Do not add `ci.yml` (or any `*.yml`) until Jason re-enables hosted CI. Preferred re-enable shape, if he wants it: one `pull_request` job, `cargo check --workspace --locked` only, timeout ≤ 10, no `cargo test`, no matrix, no clippy/fmt/coverage.

Until then the gate is local:

```bash
cargo check --workspace --locked
cargo test --workspace
make gate-90
```
