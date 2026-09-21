#!/usr/bin/env bash
# Local-only factory smoke. Never add to GitHub Actions.
# doctor + fixtures-check + operator-day + cargo test --workspace
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== smoke (local only; no Actions) =="

echo "-- doctor --"
cargo run -q -p estate-control -- doctor --root "$ROOT" --state-dir "${STATE:-$ROOT/.cell}"

echo "-- fixtures-check --"
bash "$ROOT/scripts/fixtures-check.sh"

echo "-- operator-day --"
bash "$ROOT/scripts/operator-day.sh"

echo "-- cargo test --workspace --"
cargo test --workspace

echo
echo "SMOKE GREEN (local only)"
