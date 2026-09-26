#!/usr/bin/env bash
# Opt-in one-letter classify eval.
# Default is --mock so this target does not call the network.
# Set CLASSIFY_ENDPOINT and CLASSIFY_MODEL to score a seated or hosted model.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
# Does not record a live PASS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

OUT="${OUT:-${TMPDIR:-/tmp}/cell-one-classify}"
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

if [[ -n "${CLASSIFY_ENDPOINT:-}" && -z "${CLASSIFY_MODEL:-}" ]]; then
  echo "FAIL  CLASSIFY_ENDPOINT is set but CLASSIFY_MODEL is empty. Set CLASSIFY_MODEL." >&2
  exit 1
fi

if [[ ! -f "$OUT/heldout.jsonl" ]]; then
  echo "== classify-eval needs a held-out file; running classify-prepare =="
  OUT="$OUT" bash "$ROOT/scripts/classify-prepare.sh"
fi

BEFORE="$(cksum "$ESTATE")"
REPORT="${REPORT:-$OUT/report.json}"
echo "== classify-eval (not a live PASS) =="
if [[ -n "${CLASSIFY_ENDPOINT:-}" ]]; then
  echo "endpoint set; this is still not a recorded live PASS"
  ARGS=(classify eval --records "$OUT/heldout.jsonl" --endpoint "$CLASSIFY_ENDPOINT" --model "${CLASSIFY_MODEL:-}" --report "$REPORT")
  if [[ -n "${CLASSIFY_API_KEY_ENV:-}" ]]; then
    ARGS+=(--api-key-env "$CLASSIFY_API_KEY_ENV")
  fi
  "${ESTATE_CMD[@]}" "${ARGS[@]}"
else
  echo "SKIP live endpoint (mock mode)"
  "${ESTATE_CMD[@]}" classify eval \
    --records "$OUT/heldout.jsonl" \
    --mock \
    --model mock \
    --report "$REPORT"
fi
AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  examples/estate.yaml cksum changed" >&2
  exit 1
fi
echo "READY_FOR_LIVE_TEST: no"
echo "report: $REPORT"
