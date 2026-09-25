#!/usr/bin/env bash
# Day-30 A1–A4 demo. No live model calls. Fail closed.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
ESTATE="${ESTATE:-examples/estate.yaml}"
STATE="${STATE_DIR:-.cell}"
REPORT="gate-reports/latest.md"
PASS=0
FAIL=0

note() { echo "$*"; }
ok() { echo "PASS  $*"; PASS=$((PASS + 1)); }
bad() { echo "FAIL  $*"; FAIL=$((FAIL + 1)); }

check_allow() {
  local agent="$1" kind="$2" object="$3" label="$4"
  if cargo run -q -p conveyor-proxy -- check --estate "$ESTATE" --agent "$agent" --kind "$kind" --object "$object" >/tmp/cell-one-proxy.json 2>/tmp/cell-one-proxy.err; then
    ok "$label"
  else
    bad "$label (expected allow)"
    cat /tmp/cell-one-proxy.err || true
  fi
}

check_deny() {
  local agent="$1" kind="$2" object="$3" label="$4"
  if cargo run -q -p conveyor-proxy -- check --estate "$ESTATE" --agent "$agent" --kind "$kind" --object "$object" >/tmp/cell-one-proxy.json 2>/tmp/cell-one-proxy.err; then
    bad "$label (expected deny)"
    cat /tmp/cell-one-proxy.json || true
  else
    ok "$label"
  fi
}

note "== Cell One Day-30 gate =="
note

note "-- A1 validate --"
if cargo run -q -p estate-control -- validate --estate "$ESTATE"; then
  ok "A1 example estate validates (Horizon / Research / Sanctum)"
else
  bad "A1 example estate validates"
fi

if ! cargo run -q -p estate-control -- validate --estate examples/invalid/shared-lane.yaml >/tmp/cell-one-val.out 2>/tmp/cell-one-val.err; then
  ok "A1 invalid shared-lane fails closed"
else
  bad "A1 invalid shared-lane should fail"
fi

if ! cargo run -q -p estate-control -- validate --estate examples/invalid/sacred-as-agent.yaml >/tmp/cell-one-val.out 2>/tmp/cell-one-val.err; then
  ok "A1 Cyera-as-agent fails closed"
else
  bad "A1 Cyera-as-agent should fail"
fi

note
note "-- plan --"
cargo run -q -p estate-control -- plan --estate "$ESTATE" --plans-dir plans
ok "plan appended under plans/"

note
note "-- A2 supervisor apply --"
cargo run -q -p floor-supervisor -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT"
ok "A2 bound per-agent sessions"

note
note "-- A3 memory firewall --"
check_deny horizon memory_read lane:research "A3 horizon cannot read research lane"
check_allow horizon memory_read lane:horizon "A3 horizon can read own lane"
check_deny horizon memory_read cyera-ci "A3 Cyera CI excluded"
check_deny sanctum memory_read rust-classroom "A3 Rust classroom excluded"

note
note "-- A4 deny-default tools --"
check_deny horizon tool shell "A4 undeclared tool denied"
check_deny research tool notes-append "A4 declared tool without allow intention denied"
check_deny research mount secrets "A4 undeclared mount denied"
check_deny research mount notes "A4 declared mount without allow intention denied"

note
note "-- models stay unwired --"
cargo run -q -p estate-control -- models --estate "$ESTATE"
ok "model bindings listed, not invoked"

note
note "-- pause kit --"
./scripts/pause-kit.sh stop
test -f lanes/horizon/.gitkeep
./scripts/pause-kit.sh start
ok "pause stop/start rebound from files"

note
note "-- cargo test --"
cargo test --workspace --quiet
ok "cargo test --workspace"

mkdir -p gate-reports
{
  echo "# Cell One Day-30 gate report"
  echo
  echo "generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "estate: $ESTATE"
  echo "pass: $PASS"
  echo "fail: $FAIL"
  echo
  echo "A1 agents/lanes, A2 sessions, A3 firewall + sacred exclusions, A4 deny-default."
  echo "No live frontier or local model calls."
} > "$REPORT"

echo
echo "report: $REPORT  pass=$PASS fail=$FAIL"
if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "A1–A4 GREEN"
