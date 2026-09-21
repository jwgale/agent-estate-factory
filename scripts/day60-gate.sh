#!/usr/bin/env bash
# Day-60 A5–A9 demo. Mock paths must pass. Live keys / live local host are optional and noted.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
ESTATE="${ESTATE:-examples/estate.yaml}"
STATE="${STATE_DIR:-.cell}"
FEED="${FEED_DIR:-.cell/feed}"
REPORT="gate-reports/day60.md"
PASS=0
FAIL=0
SKIP=0
MOCK_PID=""

note() { echo "$*"; }
ok() { echo "PASS  $*"; PASS=$((PASS + 1)); }
bad() { echo "FAIL  $*"; FAIL=$((FAIL + 1)); }
skip() { echo "SKIP  $*"; SKIP=$((SKIP + 1)); }

cleanup() {
  if [[ -n "${MOCK_PID}" ]] && kill -0 "${MOCK_PID}" 2>/dev/null; then
    kill "${MOCK_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

note "== Cell One Day-60 gate (A5–A9) =="
note

note "-- A1–A4 still hold --"
cargo run -q -p estate-control -- validate --estate "$ESTATE"
ok "estate validates"

if cargo run -q -p conveyor-proxy -- check --estate "$ESTATE" --agent horizon --kind memory_read --object lane:research >/tmp/cell60.json 2>/tmp/cell60.err; then
  bad "A3 cross-lane still denied"
else
  ok "A3 cross-lane still denied"
fi
if cargo run -q -p conveyor-proxy -- check --estate "$ESTATE" --agent horizon --kind model --object xai_grok >/tmp/cell60.json 2>/tmp/cell60.err; then
  ok "A9 horizon may use frontier binding"
else
  bad "A9 horizon may use frontier binding"
fi
if cargo run -q -p conveyor-proxy -- check --estate "$ESTATE" --agent sanctum --kind model --object xai_grok >/tmp/cell60.json 2>/tmp/cell60.err; then
  bad "A9 sanctum must not use undeclared frontier"
else
  ok "A9 sanctum deny-default on frontier"
fi

note
note "-- A5 plan --"
cargo run -q -p estate-control -- plan --estate "$ESTATE" --plans-dir plans --state-dir "$STATE"
ok "A5 plan wrote blast-radius text"

note
note "-- A6 apply + drift --"
cargo run -q -p estate-control -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT"
if cargo run -q -p estate-control -- drift --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT"; then
  ok "A6 apply converges (drift in-sync)"
else
  bad "A6 apply converges"
fi
cargo run -q -p floor-supervisor -- stop --state-dir "$STATE"
if cargo run -q -p estate-control -- drift --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT"; then
  bad "A6 drift should detect discarded sessions"
else
  ok "A6 drift detects discarded session dirs"
fi
cargo run -q -p estate-control -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT"
if cargo run -q -p estate-control -- drift --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT"; then
  ok "A6 re-apply converges"
else
  bad "A6 re-apply converges"
fi

note
note "-- A5 plan vs snapshot (empty blast) --"
cargo run -q -p estate-control -- plan --estate "$ESTATE" --plans-dir plans --state-dir "$STATE" | tee /tmp/cell60-plan.txt
if grep -q "blast radius is empty" /tmp/cell60-plan.txt; then
  ok "A5 plan vs last apply is empty"
else
  bad "A5 plan vs last apply is empty"
fi

note
note "-- A7/A8 mock mixed path --"
mkdir -p "$FEED"
if cargo run -q -p model-estate -- task --estate "$ESTATE" --agent horizon --act model --object xai_grok --mock --feed-dir "$FEED"; then
  ok "A7 mock: horizon completes via frontier after local precheck"
else
  bad "A7 mock frontier task"
fi
if cargo run -q -p model-estate -- task --estate "$ESTATE" --agent research --act tool --object notes-append --payload "append a note" --mock --feed-dir "$FEED"; then
  ok "A8 mock: research tool runs local precheck first"
else
  bad "A8 mock local precheck"
fi
if cargo run -q -p model-estate -- task --estate "$ESTATE" --agent sanctum --act model --object xai_grok --mock --feed-dir "$FEED"; then
  bad "A9 sanctum mock must deny"
else
  ok "A9 sanctum mock denied (A3–A4)"
fi
if grep -q "model.local.precheck" "$FEED/events.jsonl" && grep -q "model.frontier.complete" "$FEED/events.jsonl"; then
  ok "feed has scrubbed traces from both paths"
else
  bad "feed traces from both paths"
fi

note
note "-- A8 fail-closed when local is down --"
FAILCLOSED_FEED="${FEED}/fail-closed"
mkdir -p "$FAILCLOSED_FEED"
set +e
(
  unset CELL_LOCAL_ENDPOINT
  cargo run -q -p model-estate -- task --estate "$ESTATE" --agent horizon --act model --object xai_grok --feed-dir "$FAILCLOSED_FEED" >/tmp/cell60-failclosed.json 2>/tmp/cell60-failclosed.err
)
fc_status=$?
set -e
if [[ "$fc_status" -eq 0 ]]; then
  bad "A8 fail-closed: local down must deny"
else
  if grep -q "model.local.down" "$FAILCLOSED_FEED/events.jsonl" && grep -q "fail-closed" /tmp/cell60-failclosed.json /tmp/cell60-failclosed.err; then
    ok "A8 fail-closed: local down audited, no frontier fallback"
  else
    bad "A8 fail-closed audit (model.local.down)"
  fi
fi

note
note "-- A8 HTTP specialist (mock-local protocol) --"
cargo run -q -p model-estate -- mock-local --bind 127.0.0.1:47831 >/tmp/cell60-mocklocal.log 2>&1 &
MOCK_PID=$!
sleep 0.4
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:47831
if cargo run -q -p model-estate -- task --estate "$ESTATE" --agent research --act tool --object notes-append --payload "append a note" --feed-dir "$FEED"; then
  ok "A8 HTTP driver via CELL_LOCAL_ENDPOINT (mock-local)"
else
  bad "A8 HTTP driver via mock-local"
fi

note
note "-- live notes --"
if [[ -n "${XAI_API_KEY:-}" ]]; then
  if cargo run -q -p model-estate -- task --estate "$ESTATE" --agent horizon --act model --object xai_grok --payload "Reply with the single word pong." --feed-dir "$FEED"; then
    ok "A7 live frontier (XAI_API_KEY present)"
  else
    bad "A7 live frontier call failed"
  fi
else
  skip "A7 live Grok (set XAI_API_KEY; never bake secrets)"
fi

if [[ -n "${CELL_LOCAL_LIVE:-}" ]]; then
  ok "A8 live local flagged by CELL_LOCAL_LIVE"
else
  skip "A8 live local (point CELL_LOCAL_ENDPOINT at a green Ollama/llama.cpp box; see docs/operator-local.md)"
fi

note
note "-- cargo test --"
cargo test --workspace --quiet
ok "cargo test --workspace"

mkdir -p gate-reports
{
  echo "# Cell One Day-60 gate report"
  echo
  echo "generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "estate: $ESTATE"
  echo "pass: $PASS"
  echo "fail: $FAIL"
  echo "skip: $SKIP"
  echo
  echo "A5 plan, A6 apply/drift, A7/A8 mixed path (mock required; live optional), A9 same estate + A3–A4."
  echo "Feed: scrubbed traces only. No auto-promote. Not a gateway."
} > "$REPORT"

echo
echo "report: $REPORT  pass=$PASS fail=$FAIL skip=$SKIP"
if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "A5–A9 GREEN (live skips noted above if any)"
