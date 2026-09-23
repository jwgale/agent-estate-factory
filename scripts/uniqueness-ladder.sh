#!/usr/bin/env bash
# Print-only Target C uniqueness chain.
# Runs make qlora-journey, then make seat-journey (the Makefile targets).
# Does not run make train-next. That target is the opt-in train-recipe print.
# Does not train, merge, convert, seat, or promote.
# Does not run lf-beachhead-prepare. Does not flip READY_FOR_LIVE_TEST.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

echo "== uniqueness-ladder (print-only Target C uniqueness chain) =="
echo "Print-only. READY_FOR_LIVE_TEST: no"
echo "Does not train, merge, convert, seat, or promote."
echo "Does not run lf-beachhead-prepare. Does not run a live train."
echo "Chain: make qlora-journey, then make seat-journey."
echo "The train step is a separate opt-in: make train-next. This chain does not run it."
echo

echo "-- make qlora-journey --"
if ! make -C "$ROOT" qlora-journey; then
  echo "FAIL  uniqueness-ladder (qlora-journey)"
  exit 1
fi

echo
echo "-- make seat-journey --"
if ! make -C "$ROOT" seat-journey; then
  echo "FAIL  uniqueness-ladder (seat-journey)"
  exit 1
fi

echo
echo "Live train, live convert, and live seat still need a human GPU host and stay skipped."
