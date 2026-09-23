#!/usr/bin/env bash
# Print-only Axolotl QLoRA uniqueness chain.
# Runs the prepare-assert phase, then the seat-print phase, of
# make axolotl-qlora-journey. If the prepare phase fails, this script
# exits nonzero before the seat print.
# Does not train, merge, convert, seat, or promote.
# Does not run axolotl, ollama, or llama.cpp.
# Does not run make qlora-journey, make seat-journey, make train-next,
# or make lf-beachhead-prepare.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

echo "== uniqueness-axolotl (print-only Axolotl QLoRA chain) =="
echo "Print-only. READY_FOR_LIVE_TEST: no"
echo "Does not train, merge, convert, seat, or promote."
echo "Does not run axolotl, ollama, or llama.cpp."
echo "Chain: prepare-assert, then seat-print (make axolotl-qlora-journey)."
echo "Does not run make qlora-journey, make seat-journey, make train-next, or make lf-beachhead-prepare."
echo

echo "-- prepare-assert (AXOLOTL_QLORA_PHASE=prepare) --"
if ! AXOLOTL_QLORA_PHASE=prepare make -C "$ROOT" axolotl-qlora-journey; then
  echo "FAIL  uniqueness-axolotl (prepare-assert)" >&2
  exit 1
fi

echo
echo "-- seat-print (AXOLOTL_QLORA_PHASE=seat) --"
if ! AXOLOTL_QLORA_PHASE=seat make -C "$ROOT" axolotl-qlora-journey; then
  echo "FAIL  uniqueness-axolotl (seat-print)" >&2
  exit 1
fi

echo
echo "Live train, live convert, and live seat still need a human GPU host and stay skipped."
echo "PASS  uniqueness-axolotl (prepare-assert then seat-print; SKIP live train; SKIP live convert; SKIP live seat)"
echo "READY_FOR_LIVE_TEST: no"
