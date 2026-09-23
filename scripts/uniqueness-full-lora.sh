#!/usr/bin/env bash
# Print-only Target A full uniqueness print chain.
# Runs make lora-journey, then make train-next-lora, then make seat-journey-lora.
# Does not train, merge, convert, seat, or promote.
# Does not run lf-beachhead-prepare. Does not flip READY_FOR_LIVE_TEST.
# make uniqueness-full stays qlora-journey, then train-next, then seat-journey.
# make uniqueness-ladder stays qlora-journey then seat-journey.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

echo "== uniqueness-full-lora (print-only Target A full uniqueness print chain) =="
echo "Print-only. READY_FOR_LIVE_TEST: no"
echo "Does not train, merge, convert, seat, or promote."
echo "Does not run lf-beachhead-prepare. Does not run a live train."
echo "Chain: make lora-journey, then make train-next-lora, then make seat-journey-lora."
echo "make uniqueness-full stays qlora-journey, then train-next, then seat-journey."
echo "make uniqueness-ladder stays qlora-journey then seat-journey and does not run train-next."
echo

echo "-- make lora-journey --"
if ! make -C "$ROOT" lora-journey; then
  echo "FAIL  uniqueness-full-lora (lora-journey)"
  exit 1
fi

echo
echo "-- make train-next-lora --"
if ! make -C "$ROOT" train-next-lora; then
  echo "FAIL  uniqueness-full-lora (train-next-lora)"
  exit 1
fi

echo
echo "-- make seat-journey-lora --"
if ! make -C "$ROOT" seat-journey-lora; then
  echo "FAIL  uniqueness-full-lora (seat-journey-lora)"
  exit 1
fi

echo
echo "Live train, live convert, and live seat still need a human GPU host and stay skipped."
echo "PASS  uniqueness-full-lora (lora-journey, train-next-lora, seat-journey-lora; SKIP live train)"
