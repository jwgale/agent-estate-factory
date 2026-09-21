#!/usr/bin/env bash
# Day 90+ operator loop: status → plan → dry-run → apply → reconcile.
# Fixtures only. Live Grok / Mac MLX / rented GPU are optional SKIP.
# Never add this to GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
ESTATE="${ESTATE:-examples/estate.yaml}"
STATE="${STATE_DIR:-$ROOT/target/day90-cell}"
PLANS="${PLANS_DIR:-$ROOT/target/day90-plans}"
rm -rf "$STATE" "$PLANS"
mkdir -p "$STATE" "$PLANS"

echo "== day90 operator loop (local only) =="

echo "-- status (greenfield) --"
cargo run -q -p estate-control -- status --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT" --plans-dir "$PLANS"

echo "-- plan --"
cargo run -q -p estate-control -- plan --estate "$ESTATE" --plans-dir "$PLANS" --state-dir "$STATE"

echo "-- apply --dry-run --"
cargo run -q -p estate-control -- apply --dry-run --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT" --plans-dir "$PLANS"

echo "-- apply --"
cargo run -q -p estate-control -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT" --plans-dir "$PLANS"

echo "-- reconcile --"
cargo run -q -p estate-control -- reconcile --estate "$ESTATE" --state-dir "$STATE"

echo "-- reconcile --suggest (not applied) --"
cargo run -q -p estate-control -- reconcile --suggest --estate "$ESTATE" --state-dir "$STATE"

echo "-- status (after apply) --"
cargo run -q -p estate-control -- status --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT" --plans-dir "$PLANS"

echo "-- probes (catalog) --"
cargo run -q -p estate-control -- probes

echo "-- probes --live (SKIP without endpoints) --"
cargo run -q -p estate-control -- probes --live

echo
echo "DAY90 GREEN (local only)"
