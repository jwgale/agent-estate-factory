#!/usr/bin/env bash
# Day 61–90 beachhead (toward A10–A12). Local only. Do not add to GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
ESTATE="${ESTATE:-examples/estate.yaml}"
STATE="${STATE_DIR:-.cell}"
FEED="${FEED_DIR:-.cell/feed}"
DROP="${DROP_DIR:-packs}"
REPORT="gate-reports/day90.md"
PASS=0
FAIL=0

ok() { echo "PASS  $*"; PASS=$((PASS + 1)); }
bad() { echo "FAIL  $*"; FAIL=$((FAIL + 1)); }

note() { echo "$*"; }

note "== Cell One Day-90 beachhead (A10–A12 toward) =="

cargo run -q -p estate-control -- validate --estate "$ESTATE"
ok "estate validates (placements declared)"

note "-- A12 plan as human control surface --"
cargo run -q -p estate-control -- plan --estate "$ESTATE" --plans-dir plans --state-dir "$STATE" | tee /tmp/cell90-plan.txt
if grep -q "Reviewable diff" /tmp/cell90-plan.txt && grep -q "cloud-agent stub" /tmp/cell90-plan.txt; then
  ok "A12 plan is PR-reviewable and names cloud-agent stub"
else
  bad "A12 reviewable plan"
fi
cargo run -q -p estate-control -- plans --plans-dir plans >/tmp/cell90-plans.txt
ok "A12 plan history lists"

note "-- A11 suspend / resume --"
cargo run -q -p estate-control -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT" --plans-dir plans --require-plan
if [[ -f "$STATE/placement-actual.json" ]]; then
  ok "A12 placement-actual.json durable after gated apply"
else
  bad "A12 placement-actual.json missing"
fi
if ls plans/apply-*.json >/dev/null 2>&1; then
  ok "A12 apply audit written"
else
  bad "A12 apply audit written"
fi
cargo run -q -p estate-control -- suspend --state-dir "$STATE"
if [[ -f "$STATE/lifecycle.json" ]]; then
  ok "A11 lifecycle.json durable after suspend"
else
  bad "A11 lifecycle.json missing"
fi
if [[ -d "$STATE/sessions" ]]; then
  bad "A11 sessions should be discarded"
else
  ok "A11 sessions discarded; lanes stay"
fi
cargo run -q -p estate-control -- resume --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT"
if cargo run -q -p estate-control -- drift --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT"; then
  ok "A11 resume converges"
else
  bad "A11 resume converges"
fi
cargo run -q -p estate-control -- status --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT" | tee /tmp/cell90-status.txt
if grep -q "cloud-agent" /tmp/cell90-status.txt; then
  ok "A12 status shows cloud-agent stub"
else
  bad "A12 status shows cloud-agent stub"
fi
cargo run -q -p estate-control -- catalog --out "$STATE/catalog.json" >/tmp/cell90-catalog.txt
if [[ -f "$STATE/catalog.json" ]] && grep -q "cell-one.local-catalog.v0" "$STATE/catalog.json"; then
  ok "A12 catalog file SoT dumped"
else
  bad "A12 catalog file SoT dumped"
fi
cargo run -q -p estate-control -- leases --state-dir "$STATE" >/tmp/cell90-leases.txt
if grep -q "cloud-agent" /tmp/cell90-leases.txt; then
  ok "A12 leases surface"
else
  bad "A12 leases surface"
fi
cargo run -q -p estate-control -- history --state-dir "$STATE" >/tmp/cell90-history.txt
if grep -q "lifecycle history" /tmp/cell90-history.txt; then
  ok "A11 lifecycle history"
else
  bad "A11 lifecycle history"
fi
cargo run -q -p estate-control -- probes >/tmp/cell90-probes.txt
if grep -q "live_probed=false" /tmp/cell90-probes.txt && grep -q "ollama" /tmp/cell90-probes.txt; then
  ok "A12 catalog probes are not live pings"
else
  bad "A12 catalog probes are not live pings"
fi
set +e
cargo run -q -p estate-control -- validate --estate examples/invalid/placement-sacred-cloud.yaml >/tmp/cell90-sacred.out 2>/tmp/cell90-sacred.err
sacred=$?
set -e
if [[ "$sacred" -ne 0 ]]; then
  ok "A12 sacred cloud-agent assignment refused"
else
  bad "A12 sacred cloud-agent assignment must fail"
fi
set +e
cargo run -q -p estate-control -- feed pack --feed-dir "$FEED" --drop-dir "$DROP" --id local-5090 >/tmp/cell90-sku.out 2>/tmp/cell90-sku.err
sku=$?
set -e
if [[ "$sku" -ne 0 ]]; then
  ok "A10 SKU pack id refused"
else
  bad "A10 SKU pack id must fail"
fi

note "-- A10 feed pack drop zone --"
mkdir -p "$FEED"
cargo run -q -p model-estate -- task --estate "$ESTATE" --agent horizon --act model --object xai_grok --mock --feed-dir "$FEED"
cargo run -q -p model-estate -- task --estate "$ESTATE" --agent research --act tool --object notes-append --payload "append a note" --mock --feed-dir "$FEED"
cargo run -q -p estate-control -- feed pack --feed-dir "$FEED" --drop-dir "$DROP" --id overnight-traces
if [[ -f "$DROP/overnight-traces.pack.json" ]]; then
  ok "A10 candidate pack written"
else
  bad "A10 candidate pack written"
fi
BEFORE_ESTATE="$(cksum "$ESTATE")"
cargo run -q -p estate-control -- feed import --id overnight-traces --drop-dir "$DROP" --accepted-dir "$DROP/accepted" --estate "$ESTATE" | tee /tmp/cell90-import.txt
if grep -q "estate file unchanged" /tmp/cell90-import.txt && [[ "$(cksum "$ESTATE")" == "$BEFORE_ESTATE" ]]; then
  ok "A10 explicit import does not rewrite estate"
else
  bad "A10 explicit import does not rewrite estate"
fi
set +e
cargo run -q -p estate-control -- feed promote --id overnight-traces >/tmp/cell90-promote.out 2>/tmp/cell90-promote.err
promo=$?
set -e
if [[ "$promo" -ne 0 ]]; then
  ok "A10 promote refused (manual curator)"
else
  bad "A10 promote must fail"
fi

note "-- cargo test (workspace) --"
cargo test --workspace --quiet
ok "cargo test --workspace"

mkdir -p gate-reports
{
  echo "# Cell One Day-90 beachhead report"
  echo
  echo "generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "pass: $PASS"
  echo "fail: $FAIL"
  echo
  echo "A10 feed pack + explicit import (no auto-promote, SKU refuse, INDEX), A11 suspend/resume + unspawned leases, A12 gated apply + PlacementDriver + catalog file SoT."
  echo "Live Grok / GPU not required. Hosted CI is disabled overnight. Gate is local cargo test / make gate-90."
} > "$REPORT"

echo
echo "report: $REPORT  pass=$PASS fail=$FAIL"
if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "A10–A12 BEACHHEAD GREEN"
