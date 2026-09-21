#!/usr/bin/env bash
# Thin fixture library check: estate validate/doctor against happy + refuse fixtures.
# No live Grok / GPU / Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

ESTATE=(cargo run -q -p estate-control --)
STATE="${STATE:-$ROOT/target/fixtures-check-state}"
rm -rf "$STATE"
mkdir -p "$STATE"

echo "== fixtures-check =="

happy=(
  "$ROOT/examples/estate.yaml"
  "$ROOT/examples/hosts/rtx-consumer.yaml"
  "$ROOT/examples/hosts/apple-silicon.yaml"
  "$ROOT/examples/hosts/nvidia-rental.yaml"
  "$ROOT/examples/hosts/multi-host.yaml"
  "$ROOT/examples/fixtures/happy.yaml"
)
for f in "${happy[@]}"; do
  echo "-- validate ok  ${f#$ROOT/}"
  "${ESTATE[@]}" validate --estate "$f"
done

refuse=(
  "$ROOT/examples/fixtures/refuse-api-version.yaml"
  "$ROOT/examples/fixtures/refuse-kind.yaml"
  "$ROOT/examples/fixtures/refuse-version.yaml"
  "$ROOT/examples/fixtures/refuse-too-few-agents.yaml"
  "$ROOT/examples/fixtures/refuse-shared-lane.yaml"
  "$ROOT/examples/fixtures/refuse-sacred-as-agent.yaml"
  "$ROOT/examples/fixtures/refuse-frontier-only.yaml"
  "$ROOT/examples/fixtures/refuse-missing-sacred.yaml"
  "$ROOT/examples/fixtures/refuse-allow-sacred.yaml"
  "$ROOT/examples/fixtures/refuse-default-allow.yaml"
  "$ROOT/examples/fixtures/refuse-sku-binding.yaml"
  "$ROOT/examples/fixtures/refuse-placement-sku.yaml"
  "$ROOT/examples/fixtures/refuse-placement-unknown-agent.yaml"
  "$ROOT/examples/fixtures/refuse-placement-sacred-cloud.yaml"
  "$ROOT/examples/fixtures/refuse-host-class.yaml"
)
for f in "${refuse[@]}"; do
  echo "-- validate refuse  ${f#$ROOT/}"
  set +e
  "${ESTATE[@]}" validate --estate "$f" >/tmp/fixtures-check.out 2>/tmp/fixtures-check.err
  rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then
    echo "FAIL  ${f#$ROOT/} must fail closed"
    exit 1
  fi
done

echo "-- doctor --"
"${ESTATE[@]}" doctor --root "$ROOT" --state-dir "$STATE"

echo
echo "FIXTURES-CHECK GREEN"
