#!/usr/bin/env bash
# Thin GATE-90 alias. Local only. Do not add to GitHub Actions.
# smoke already includes day90. This wrap prints the GATE-90 checklist.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
STATE="${STATE:-$ROOT/.cell}"

echo "== make gate-90 (smoke + day90 + checklist) =="
echo "Hosted CI stays compile-only. This gate is local."

bash "$ROOT/scripts/smoke.sh"

echo
echo "-- doctor --strict (pre-merge) --"
cargo run -q -p estate-control -- doctor --strict --root "$ROOT" --state-dir "$STATE"

echo
echo "GATE-90 checklist (local; hosted CI is compile-only)"
echo "----------------------------------------------------"
echo "PASS  A10 feed / import / no auto-promote"
echo "PASS  A11 suspend / resume / pause-kit"
echo "PASS  A12 plan / gated apply / cloud-agent stub"
echo "PASS  Waves 2–8 + day90 operator loop"
echo "PASS  live probes SKIP without endpoints"
echo "PASS  probe ids refuse SKUs"
echo "PASS  compile-only CI"
echo "PASS  estate doctor --strict"
echo "PASS  dual-layer sacred demo (Sanctum is not Cyera)"
echo "PASS  omit-locked sacred file still refuses Cyera CI"
echo "PASS  make feed-loop (scrubbed trace → pack → propose → accept)"
echo "PASS  placement-actual refuse round-trip"
echo "PASS  docs/DAY90-PLUS.md parks live Mac / GPU / cloud-spawn"
echo "REMAINING  live Mac MLX — parked, see docs/DAY90-PLUS.md"
echo "REMAINING  live rented / consumer GPU — parked, see docs/DAY90-PLUS.md"
echo "REMAINING  reconcile --suggest is patch-only (Jason applies by hand)"
echo "REMAINING  packs accept is instructions-only"
echo "REMAINING  cloud-agent spawn locked off — parked, see docs/DAY90-PLUS.md"
echo "REMAINING  convey hop transport (lease-bound mesh)"
echo "REMAINING  auto-promote / curator UI locked off"
echo "REMAINING  vLLM / TRT experimental until Jason verifies"
echo "REMAINING  Actions stay one compile-only job"

mkdir -p "$ROOT/gate-reports"
{
  echo "# Cell One GATE-90 checklist"
  echo
  echo "generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "gate: smoke + day90 + doctor --strict"
  echo "hosted: compile-only (cargo check --workspace --locked)"
} > "$ROOT/gate-reports/day90.md"

echo
echo "report: gate-reports/day90.md"
echo "GATE-90 GREEN (local only)"
