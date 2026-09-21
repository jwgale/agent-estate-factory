#!/usr/bin/env bash
# Mixed frontier+local fixture: plan → apply --require-plan.
# Isolated cell. Fixtures only. No live keys. No frontier POST.
# Local only. Do not add to make smoke or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

ESTATE="${ESTATE:-$ROOT/examples/fixtures/mixed-frontier-local.yaml}"
STATE="${STATE_DIR:-$ROOT/target/day90-mixed-cell}"
PLANS="${PLANS_DIR:-$ROOT/target/day90-mixed-plans}"
BIN="${ESTATE_BIN:-}"

rm -rf "$STATE" "$PLANS"
mkdir -p "$STATE" "$PLANS"

estate() {
  if [[ -n "$BIN" ]]; then
    "$BIN" "$@"
  else
    cargo run -q -p estate-control -- "$@"
  fi
}

echo "== day90-mixed (fixtures only) =="
echo "estate: $ESTATE"
echo "state: $STATE"

echo "-- status (greenfield) --"
before="$(estate status --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT" --plans-dir "$PLANS")"
printf '%s\n' "$before"
printf '%s\n' "$before" | grep -q 'frontier: frontier_http model=grok-4.7'
if printf '%s\n' "$before" | grep -q 'catalog frontier: cell'; then
  echo "FAIL  greenfield status invented a cell catalog"
  exit 1
fi

echo "-- plan --"
estate plan --estate "$ESTATE" --plans-dir "$PLANS" --state-dir "$STATE"

echo "-- apply --require-plan --"
estate apply --require-plan --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT" --plans-dir "$PLANS"

test -f "$STATE/placement-actual.json"
test -f "$STATE/model-actual.json"
test -f "$STATE/catalog.json"
grep -q 'frontier_http' "$STATE/model-actual.json"
grep -q '"driver": "ollama"' "$STATE/model-actual.json"
grep -q '"model": "grok-4.7"' "$STATE/catalog.json"

echo "-- status (after apply) --"
after="$(estate status --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT" --plans-dir "$PLANS")"
printf '%s\n' "$after"
printf '%s\n' "$after" | grep -q 'frontier: frontier_http model=grok-4.7'
printf '%s\n' "$after" | grep -q 'catalog frontier: cell model=grok-4.7'
printf '%s\n' "$after" | grep -q 'catalog frontier: schema model=grok-4.7'

echo "-- doctor --"
doc="$(estate doctor --root "$ROOT" --state-dir "$STATE")"
printf '%s\n' "$doc"
printf '%s\n' "$doc" | grep -q 'schema/local-catalog.v0.json model=grok-4.7'
printf '%s\n' "$doc" | grep -q 'catalog.json model=grok-4.7'

if printf '%s\n' "$before" "$after" "$doc" | grep -Eiq 'sk-|Bearer |XAI_API_KEY=.+'; then
  echo "FAIL  mixed walk printed a key"
  exit 1
fi

# Host fixture only. Validate + status. No apply, no keys.
# Not added to fixtures-check: that script is inside make smoke.
HOST="$ROOT/examples/hosts/frontier-http.yaml"
LOCKED="$ROOT/examples/estate.yaml"
lock_before="$(cksum "$LOCKED")"
HOST_STATE="${STATE}-frontier-http"
HOST_PLANS="${PLANS}-frontier-http"
rm -rf "$HOST_STATE" "$HOST_PLANS"
mkdir -p "$HOST_STATE" "$HOST_PLANS"

echo "-- frontier-http host fixture (validate + status, no keys) --"
estate validate --estate "$HOST"
host_status="$(estate status --estate "$HOST" --state-dir "$HOST_STATE" --roots-base "$ROOT" --plans-dir "$HOST_PLANS")"
printf '%s\n' "$host_status"
printf '%s\n' "$host_status" | grep -q 'estate: cell-one-frontier-http'
printf '%s\n' "$host_status" | grep -q 'frontier: frontier_http model=grok-4.7'
if printf '%s\n' "$host_status" | grep -q 'catalog frontier: cell'; then
  echo "FAIL  frontier-http status invented a cell catalog"
  exit 1
fi
if [[ -f "$HOST_STATE/placement-actual.json" || -f "$HOST_STATE/catalog.json" ]]; then
  echo "FAIL  frontier-http status wrote cell files"
  exit 1
fi
lock_after="$(cksum "$LOCKED")"
if [[ "$lock_before" != "$lock_after" ]]; then
  echo "FAIL  examples/estate.yaml changed"
  exit 1
fi
if printf '%s\n' "$host_status" | grep -Eiq 'sk-|Bearer |XAI_API_KEY=.+'; then
  echo "FAIL  frontier-http status printed a key"
  exit 1
fi

echo
echo "DAY90-MIXED GREEN (fixtures only)"
