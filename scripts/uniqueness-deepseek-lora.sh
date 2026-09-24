#!/usr/bin/env bash
# Print-only DeepSeek-R1-Distill chat LoRA uniqueness chain.
# Runs the prepare-assert phase, then the seat-print phase, of
# make deepseek-r1-distill-lora-journey. If the prepare phase fails,
# this script exits nonzero before the seat print.
# Does not train, merge, convert, seat, or promote.
# Does not run make deepseek-r1-distill-journey or make uniqueness-deepseek.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

echo "== uniqueness-deepseek-lora (print-only DeepSeek-R1-Distill chat LoRA chain) =="
echo "Print-only. READY_FOR_LIVE_TEST: no"
echo "Does not train, merge, convert, seat, or promote."
echo "Chain: prepare-assert, then seat-print (make deepseek-r1-distill-lora-journey)."
echo "Does not run make deepseek-r1-distill-journey or make uniqueness-deepseek."
echo "This print is not a live PASS."
echo "The recorded Target C PASS stays the only live uniqueness prove."
echo

echo "-- prepare-assert (DEEPSEEK_R1_DISTILL_PHASE=prepare) --"
if ! DEEPSEEK_CARD=llamafactory-lora DEEPSEEK_R1_DISTILL_PHASE=prepare make -C "$ROOT" deepseek-r1-distill-lora-journey; then
  echo "FAIL  uniqueness-deepseek-lora (prepare-assert)" >&2
  exit 1
fi

echo
echo "-- seat-print (DEEPSEEK_R1_DISTILL_PHASE=seat) --"
if ! DEEPSEEK_CARD=llamafactory-lora DEEPSEEK_R1_DISTILL_PHASE=seat make -C "$ROOT" deepseek-r1-distill-lora-journey; then
  echo "FAIL  uniqueness-deepseek-lora (seat-print)" >&2
  exit 1
fi

echo
echo "Live train, live convert, and live seat still need a human host and stay skipped."
echo "PASS  uniqueness-deepseek-lora (prepare-assert then seat-print; SKIP live train; SKIP live convert; SKIP live seat)"
echo "READY_FOR_LIVE_TEST: no"
