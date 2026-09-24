#!/usr/bin/env bash
# Print-only GLM-4 Chat LoRA uniqueness chain.
# Runs the prepare-assert phase, then the seat-print phase, of
# make glm4-chat-lora-journey. If the prepare phase fails,
# this script exits nonzero before the seat print.
# Does not train, merge, convert, seat, or promote.
# Does not run make glm4-chat-journey or make uniqueness-glm.
# Does not run make deepseek-r1-distill-lora-journey or make uniqueness-deepseek-lora.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

echo "== uniqueness-glm-lora (print-only GLM-4 Chat LoRA chain) =="
echo "Print-only. READY_FOR_LIVE_TEST: no"
echo "Does not train, merge, convert, seat, or promote."
echo "Chain: prepare-assert, then seat-print (make glm4-chat-lora-journey)."
echo "Does not run make glm4-chat-journey or make uniqueness-glm."
echo "Does not run make deepseek-r1-distill-lora-journey or make uniqueness-deepseek-lora."
echo "This print is not a live PASS."
echo "The recorded Target C PASS stays the only live uniqueness prove."
echo

echo "-- prepare-assert (GLM4_CHAT_PHASE=prepare) --"
if ! GLM_CARD=llamafactory-lora GLM4_CHAT_PHASE=prepare make -C "$ROOT" glm4-chat-lora-journey; then
  echo "FAIL  uniqueness-glm-lora (prepare-assert)" >&2
  exit 1
fi

echo
echo "-- seat-print (GLM4_CHAT_PHASE=seat) --"
if ! GLM_CARD=llamafactory-lora GLM4_CHAT_PHASE=seat make -C "$ROOT" glm4-chat-lora-journey; then
  echo "FAIL  uniqueness-glm-lora (seat-print)" >&2
  exit 1
fi

echo
echo "Live train, live convert, and live seat still need a human host and stay skipped."
echo "PASS  uniqueness-glm-lora (prepare-assert then seat-print; SKIP live train; SKIP live convert; SKIP live seat)"
echo "READY_FOR_LIVE_TEST: no"
