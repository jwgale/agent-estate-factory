#!/usr/bin/env bash
# Print-only Unsloth QLoRA uniqueness chain.
# Runs the prepare-assert phase, then the seat-print phase, of
# make unsloth-qlora-journey. If the prepare phase fails, this script
# exits nonzero before the seat print.
# Does not train, merge, convert, seat, or promote.
# Does not call Unsloth, ollama, or llama.cpp.
# Does not run make qlora-journey, make seat-journey, make train-next,
# make axolotl-qlora-journey, or make lf-beachhead-prepare.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

echo "== uniqueness-unsloth (print-only Unsloth QLoRA chain) =="
echo "Print-only. READY_FOR_LIVE_TEST: no"
echo "Does not train, merge, convert, seat, or promote."
echo "Does not call Unsloth, ollama, or llama.cpp."
echo "Chain: prepare-assert, then seat-print (make unsloth-qlora-journey)."
echo "Does not run make qlora-journey, make seat-journey, make train-next, make axolotl-qlora-journey, or make lf-beachhead-prepare."
echo

echo "-- prepare-assert (UNSLOTH_QLORA_PHASE=prepare) --"
if ! UNSLOTH_QLORA_PHASE=prepare make -C "$ROOT" unsloth-qlora-journey; then
  echo "FAIL  uniqueness-unsloth (prepare-assert)" >&2
  exit 1
fi

echo
echo "-- seat-print (UNSLOTH_QLORA_PHASE=seat) --"
if ! UNSLOTH_QLORA_PHASE=seat make -C "$ROOT" unsloth-qlora-journey; then
  echo "FAIL  uniqueness-unsloth (seat-print)" >&2
  exit 1
fi

echo
echo "Live train, live convert, and live seat still need a human GPU host and stay skipped."
echo "PASS  uniqueness-unsloth (prepare-assert then seat-print; SKIP live train; SKIP live convert; SKIP live seat)"
echo "READY_FOR_LIVE_TEST: no"
