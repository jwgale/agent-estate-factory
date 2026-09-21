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
  "$ROOT/examples/fixtures/mixed-frontier-local.yaml"
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
  "$ROOT/examples/fixtures/refuse-sacred-overlay.yaml"
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

echo "-- mixed proof dry-run (no live calls) --"
MIXED="$ROOT/examples/fixtures/mixed-frontier-local.yaml"
"${ESTATE[@]}" validate --estate "$MIXED"
"${ESTATE[@]}" apply --dry-run --estate "$MIXED" --state-dir "$STATE" --roots-base "$STATE" --plans-dir "$STATE/plans"
if [[ -f "$STATE/placement-actual.json" ]]; then
  echo "FAIL  mixed dry-run wrote placement-actual.json"
  exit 1
fi
"${ESTATE[@]}" catalog --out "$STATE/catalog.json" >/tmp/fixtures-catalog.out
if ! grep -q "http-remote" /tmp/fixtures-catalog.out || ! grep -q "ollama" /tmp/fixtures-catalog.out; then
  echo "FAIL  catalog must list http-remote and ollama"
  exit 1
fi
echo "PASS  mixed-frontier-local validate + dry-run + catalog"

echo "-- probes --live SKIP (no endpoints) --"
"${ESTATE[@]}" probes --live >/tmp/fixtures-probes-live.out
if ! grep -q "SKIP" /tmp/fixtures-probes-live.out; then
  echo "FAIL  probes --live must SKIP without endpoints"
  exit 1
fi
if grep -qE "5090|4090|m3-max" /tmp/fixtures-probes-live.out; then
  echo "FAIL  probe ids must not encode a hardware SKU"
  exit 1
fi
echo "PASS  probes --live SKIP"

echo "-- doctor --"
"${ESTATE[@]}" doctor --root "$ROOT" --state-dir "$STATE"

echo "-- policy allow --"
"${ESTATE[@]}" policy check --policy "$ROOT/examples/fixtures/policy-allow.yaml" --action apply
"${ESTATE[@]}" policy check --policy "$ROOT/examples/fixtures/policy-allow.yaml" --action convey-call
"${ESTATE[@]}" policy check --policy "$ROOT/policy/cell-one.policy.v0.yaml" --action backup
echo "PASS  policy allow"

echo "-- policy deny --"
set +e
"${ESTATE[@]}" policy check --policy "$ROOT/examples/fixtures/policy-deny.yaml" --action apply >/tmp/fixtures-policy-deny.out 2>/tmp/fixtures-policy-deny.err
deny_rc=$?
set -e
if [[ "$deny_rc" -eq 0 ]]; then
  echo "FAIL  policy-deny.yaml must refuse apply"
  exit 1
fi
if ! grep -q "refuse:policy" /tmp/fixtures-policy-deny.out /tmp/fixtures-policy-deny.err; then
  echo "FAIL  policy deny must print refuse:policy"
  exit 1
fi
echo "PASS  policy deny"

echo "-- policy unknown action --"
set +e
"${ESTATE[@]}" policy check --policy "$ROOT/examples/fixtures/policy-unknown-action.yaml" --action apply >/tmp/fixtures-policy-unknown.out 2>/tmp/fixtures-policy-unknown.err
unknown_rc=$?
set -e
if [[ "$unknown_rc" -eq 0 ]]; then
  echo "FAIL  policy-unknown-action.yaml must fail closed"
  exit 1
fi
if ! grep -q "refuse:unknown-action" /tmp/fixtures-policy-unknown.out /tmp/fixtures-policy-unknown.err; then
  echo "FAIL  unknown action must print refuse:unknown-action"
  exit 1
fi
echo "PASS  policy unknown-action"

echo
echo "FIXTURES-CHECK GREEN"
