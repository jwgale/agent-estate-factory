#!/usr/bin/env bash
# Print-only Target C full uniqueness print chain.
# Runs make qlora-journey, then make train-next, then make seat-journey.
# Does not train, merge, convert, seat, or promote.
# Does not run lf-beachhead-prepare. Does not flip READY_FOR_LIVE_TEST.
# make uniqueness-ladder stays qlora-journey then seat-journey.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

echo "== uniqueness-full (print-only Target C full uniqueness print chain) =="
echo "Print-only. READY_FOR_LIVE_TEST: no"
echo "Does not train, merge, convert, seat, or promote."
echo "Does not run lf-beachhead-prepare. Does not run a live train."
echo "Chain: make qlora-journey, then make train-next, then make seat-journey."
echo "make uniqueness-ladder stays qlora-journey then seat-journey and does not run train-next."
echo

echo "-- make qlora-journey --"
if ! make -C "$ROOT" qlora-journey; then
  echo "FAIL  uniqueness-full (qlora-journey)"
  exit 1
fi

echo
echo "-- make train-next --"
if ! make -C "$ROOT" train-next; then
  echo "FAIL  uniqueness-full (train-next)"
  exit 1
fi

echo
echo "-- make seat-journey --"
if ! make -C "$ROOT" seat-journey; then
  echo "FAIL  uniqueness-full (seat-journey)"
  exit 1
fi

echo
echo "Live train, live convert, and live seat still need a human GPU host and stay skipped."
echo "PASS  uniqueness-full (qlora-journey, train-next, seat-journey; SKIP live train)"
