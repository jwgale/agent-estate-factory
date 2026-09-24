#!/usr/bin/env bash
# Opt-in tev1 classify journey. Default is print.
# Set TEV1_RUN=1 to execute train, merge, GGUF, Ollama, and eval.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
# Does not invent a live PASS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

INPUT="${INPUT:-$ROOT/examples/fixtures/tev1-decisions.jsonl}"
OUT="${OUT:-${TMPDIR:-/tmp}/cell-one-tev1-journey}"
BASE="${BASE:-Qwen/Qwen3.5-4B}"
BASE_TAG="${BASE_TAG:-qwen3.5:4b}"
TAG="${TAG:-tev1-specialist}"
ENDPOINT="${ENDPOINT:-http://127.0.0.1:11434}"
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
BIN="${ESTATE_BIN:-}"

if [[ -n "$BIN" && -x "$BIN" ]]; then
  ESTATE_CMD=("$BIN")
elif [[ -n "$BIN" ]]; then
  echo "FAIL  ESTATE_BIN is set but not executable: $BIN" >&2
  exit 1
elif [[ -x "$ROOT/target/release/estate" ]]; then
  ESTATE_CMD=("$ROOT/target/release/estate")
elif [[ -x "$ROOT/target/debug/estate" ]]; then
  ESTATE_CMD=("$ROOT/target/debug/estate")
elif command -v cargo >/dev/null 2>&1; then
  ESTATE_CMD=(cargo run -q -p estate-control --)
else
  echo "FAIL  estate binary unresolved" >&2
  exit 1
fi

BEFORE="$(cksum "$ESTATE")"
echo "== tev1-journey (local output; not a live PASS) =="
ARGS=(
  classify journey
  --input "$INPUT"
  --out "$OUT"
  --base "$BASE"
  --base-tag "$BASE_TAG"
  --tag "$TAG"
  --endpoint "$ENDPOINT"
)
if [[ "${TEV1_RUN:-}" == "1" ]]; then
  echo "run requested; this still does not invent a live PASS"
  ARGS+=(--run)
else
  echo "SKIP live train (print)"
  ARGS+=(--print)
fi
"${ESTATE_CMD[@]}" "${ARGS[@]}"
AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  examples/estate.yaml cksum changed" >&2
  exit 1
fi
echo "READY_FOR_LIVE_TEST: no"
echo "out: $OUT"
