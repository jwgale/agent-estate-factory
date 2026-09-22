#!/usr/bin/env bash
# Opt-in real-world kit. Not in make smoke / make gate-90 / GitHub Actions.
# Unset CELL_LOCAL_ENDPOINT: live steps SKIP and exit 0. A SKIP is not a success line.
# Does not read or print the frontier API key.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "One-box Agent Estate Factory: plan/apply IaC, sacred isolation (Cyera CI + Rust classroom out; Sanctum is not Cyera), equal-class frontier+local, manual enrich packs."
echo "train/enrich: make enrich-prepare writes artifacts only (docs/TRAIN-ENRICH.md). Not a live train."
echo

echo "== cargo check --workspace --locked (make check) =="
cargo check --workspace --locked

echo
echo "== estate doctor (vanilla) examples/estate.yaml =="
cargo run -q -p estate-control -- doctor --root "$ROOT" --state-dir "${STATE:-.cell}"

if [[ -z "${CELL_LOCAL_ENDPOINT:-}" ]]; then
  echo
  echo "SKIP live probes (CELL_LOCAL_ENDPOINT unset; not a PASS)"
  echo "SKIP live specialist (CELL_LOCAL_ENDPOINT unset; not a PASS)"
  exit 0
fi

if echo "$CELL_LOCAL_ENDPOINT" | grep -qiE '5090|4090|m3-max'; then
  echo "refuse: CELL_LOCAL_ENDPOINT encodes a hardware SKU" >&2
  exit 1
fi

echo
echo "== estate probes --live =="
cargo run -q -p estate-control -- probes --live

echo
echo "== estate specialist --driver ollama =="
cargo run -q -p estate-control -- specialist --driver ollama \
  --prompt "Reply with the single word pong."
