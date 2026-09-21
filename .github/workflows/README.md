# Workflows

One workflow only: `ci.yml`.

- `pull_request` → `main`
- concurrency cancel-in-progress
- one job: `cargo check --workspace --locked`
- rustc 1.88 (`rust-toolchain.toml` + `dtolnay/rust-toolchain@1.88.0`)
- timeout 10 minutes

Do not add `cargo test`, a matrix, clippy/fmt/coverage, or push-to-main jobs. Real tests stay local:

```bash
cargo check --workspace --locked
cargo test --workspace
make smoke
```
