#!/usr/bin/env bash
# Mixed frontier+local fixture: plan → apply --require-plan.
# Isolated cell. Fixtures only. No live keys. No frontier POST.
# Local only. Do not add to make smoke or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT
# Set on purpose. Must not become a binding when the estate has none.
export CELL_FRONTIER_MODEL=grok-4.7

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
plan="$(estate plan --estate "$ESTATE" --plans-dir "$PLANS" --state-dir "$STATE")"
printf '%s\n' "$plan"
printf '%s\n' "$plan" | grep -q 'frontier plan: model=grok-4.7 source_drivers=frontier,local'
if printf '%s\n' "$plan" | grep -q 'frontier plan: model=-'; then
  echo "FAIL  mixed plan printed an unbound frontier model"
  exit 1
fi

echo "-- apply --require-plan --"
applied="$(estate apply --require-plan --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT" --plans-dir "$PLANS")"
printf '%s\n' "$applied"
printf '%s\n' "$applied" | grep -q 'applied '
if printf '%s\n' "$applied" | grep -q 'frontier plan: model=-'; then
  echo "FAIL  mixed apply printed an unbound frontier model"
  exit 1
fi

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

if printf '%s\n' "$before" "$plan" "$applied" "$after" "$doc" | grep -Eiq 'sk-|Bearer |XAI_API_KEY=.+'; then
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

# Throwaway estate. Not a fixture file. Cell One load requires a frontier
# binding, so live apply and status fail closed. Plan and dry-run refuse
# frontier-invent before any write. CELL_FRONTIER_MODEL must not be copied.
echo "-- local-only (no frontier binding) --"
LOCAL_ESTATE="$ROOT/target/day90-mixed-local-only.yaml"
LOCAL_STATE="${STATE}-local-only"
LOCAL_PLANS="${PLANS}-local-only"
rm -rf "$LOCAL_STATE" "$LOCAL_PLANS"
mkdir -p "$LOCAL_STATE" "$LOCAL_PLANS"
cat > "$LOCAL_ESTATE" <<'EOF'
version: 0
name: local-only
default_effect: deny
agents:
  - id: horizon
    display_name: Horizon
    lane: horizon
    desktop: horizon-desktop
lanes:
  - id: horizon
    root_path: lanes/horizon
    owner_agent_id: horizon
model_bindings:
  - id: local_slm
    class: local
    driver: ollama
    wired: true
EOF

if local_plan="$(estate plan --estate "$LOCAL_ESTATE" --plans-dir "$LOCAL_PLANS" --state-dir "$LOCAL_STATE" 2>&1)"; then
  echo "FAIL  local-only plan invented a frontier path"
  printf '%s\n' "$local_plan"
  exit 1
fi
printf '%s\n' "$local_plan"
printf '%s\n' "$local_plan" | grep -q 'refuse:frontier-invent'
if printf '%s\n' "$local_plan" | grep -q 'grok-4.7'; then
  echo "FAIL  local-only plan printed grok-4.7"
  exit 1
fi
if [[ -e "$LOCAL_PLANS/INDEX.md" ]] || [[ -n "$(find "$LOCAL_PLANS" -name 'plan-*.md' -print -quit)" ]]; then
  echo "FAIL  local-only plan wrote a plan file"
  exit 1
fi

if local_dry="$(estate apply --dry-run --require-plan --estate "$LOCAL_ESTATE" --state-dir "$LOCAL_STATE" --roots-base "$ROOT" --plans-dir "$LOCAL_PLANS" 2>&1)"; then
  echo "FAIL  local-only dry-run invented a frontier path"
  printf '%s\n' "$local_dry"
  exit 1
fi
printf '%s\n' "$local_dry"
printf '%s\n' "$local_dry" | grep -q 'refuse:frontier-invent'
if printf '%s\n' "$local_dry" | grep -q 'grok-4.7'; then
  echo "FAIL  local-only dry-run printed grok-4.7"
  exit 1
fi
if [[ -f "$LOCAL_STATE/catalog.json" || -f "$LOCAL_STATE/placement-actual.json" ]]; then
  echo "FAIL  local-only dry-run wrote cell files"
  exit 1
fi

if local_live="$(estate apply --require-plan --estate "$LOCAL_ESTATE" --state-dir "$LOCAL_STATE" --roots-base "$ROOT" --plans-dir "$LOCAL_PLANS" 2>&1)"; then
  echo "FAIL  local-only apply invented a frontier path"
  printf '%s\n' "$local_live"
  exit 1
fi
printf '%s\n' "$local_live"
if printf '%s\n' "$local_live" | grep -q 'grok-4.7'; then
  echo "FAIL  local-only apply printed grok-4.7"
  exit 1
fi
if [[ -f "$LOCAL_STATE/catalog.json" || -f "$LOCAL_STATE/placement-actual.json" ]]; then
  echo "FAIL  local-only apply wrote cell files"
  exit 1
fi

if local_status="$(estate status --estate "$LOCAL_ESTATE" --state-dir "$LOCAL_STATE" --roots-base "$ROOT" --plans-dir "$LOCAL_PLANS" 2>&1)"; then
  echo "FAIL  local-only status loaded an estate with no frontier binding"
  printf '%s\n' "$local_status"
  exit 1
fi
printf '%s\n' "$local_status"
if printf '%s\n' "$local_status" | grep -q 'grok-4.7\|frontier:'; then
  echo "FAIL  local-only status printed a frontier binding"
  exit 1
fi

local_doc="$(estate doctor --root "$ROOT" --state-dir "$LOCAL_STATE")"
printf '%s\n' "$local_doc"
printf '%s\n' "$local_doc" | grep -q 'schema/local-catalog.v0.json model=grok-4.7'
if printf '%s\n' "$local_doc" | grep -q 'catalog.json model='; then
  echo "FAIL  local-only doctor invented a cell catalog model"
  exit 1
fi
if printf '%s\n' "$local_plan" "$local_dry" "$local_live" "$local_status" "$local_doc" | grep -Eiq 'sk-|Bearer |XAI_API_KEY=.+'; then
  echo "FAIL  local-only walk printed a key"
  exit 1
fi
rm -rf "$LOCAL_STATE" "$LOCAL_PLANS"
rm -f "$LOCAL_ESTATE"

echo
echo "DAY90-MIXED GREEN (fixtures only)"
