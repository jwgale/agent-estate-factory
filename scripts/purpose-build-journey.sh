#!/usr/bin/env bash
# Print-only purpose-build on-demand entry (operator section 18).
# Chains make purpose-build-pick, then make purpose-build-checklist.
# Calls those targets through make. Does not inline their bodies.
# Does not resolve or execute estate beyond what those targets already do.
# Does not train, fuse, convert, shell out to ollama, promote, or apply the estate.
# Does not invent a live PASS. The recorded 5090-class Target C uniqueness
# PASS in docs/LIVE-PROBES.md stays the only live uniqueness prove.
# The re-prove card stays make uniqueness-prove-checklist. This journey does not run it.
# CELL_TRAIN_LIVE=1 stays print-only. CELL_SEAT_LIVE=1 stays print-only.
# Not native MLX.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

echo "== purpose-build-journey (print-only purpose-build on-demand entry) =="
echo "Print-only. READY_FOR_LIVE_TEST: no"
echo "When an SLM fits mid-software-build, or on demand, the same print-only entry is make purpose-build-journey."
echo "Purpose-build on demand. This journey is operator UX."
echo "Chain: make purpose-build-pick, then make purpose-build-checklist. Those two targets are the parts."
echo "It calls those targets through make. It does not inline their bodies."
echo "It does not resolve or execute estate beyond what those targets already do."
echo "It is not a re-prove of the recorded Target C live PASS."
echo "The factory does not train, fuse, convert, shell out to ollama, or promote."
echo "The factory does not apply the estate."
echo "CELL_TRAIN_LIVE=1 stays print-only."
echo "CELL_SEAT_LIVE=1 stays print-only."
echo "Not native MLX."
echo "Not in make smoke, make gate-90, or GitHub Actions."
echo "Recorded PASS: docs/LIVE-PROBES.md section Target C live uniqueness (5090-class)."
echo "That recorded PASS stays the only live uniqueness prove."
echo "This journey does not invent a new live PASS."
echo "The re-prove card stays make uniqueness-prove-checklist. This journey does not run it."
echo "examples/estate.yaml stays unchanged."

if [[ "${CELL_TRAIN_LIVE:-}" == "1" ]]; then
  echo "CELL_TRAIN_LIVE=1 is set. This journey stays print-only."
fi
if [[ "${CELL_SEAT_LIVE:-}" == "1" ]]; then
  echo "CELL_SEAT_LIVE=1 is set. This journey stays print-only."
fi

sum="$(cksum "$ROOT/examples/estate.yaml")"
echo "cksum: $sum"
case "$sum" in
  "43770130 3391"*) ;;
  *)
    echo "FAIL  examples/estate.yaml cksum is not 43770130 3391: $sum" >&2
    exit 1
    ;;
esac

echo
echo "-- make purpose-build-pick --"
if ! make -C "$ROOT" purpose-build-pick; then
  echo "FAIL  purpose-build-journey (purpose-build-pick)" >&2
  exit 1
fi

echo
echo "-- make purpose-build-checklist --"
if ! make -C "$ROOT" purpose-build-checklist; then
  echo "FAIL  purpose-build-journey (purpose-build-checklist)" >&2
  exit 1
fi

echo
echo "This print is not a live PASS."
echo "Purpose-build journey: make purpose-build-journey."
