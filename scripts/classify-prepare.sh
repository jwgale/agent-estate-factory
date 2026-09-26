#!/usr/bin/env bash
# Opt-in one-letter classify prepare. Offline. Does not train.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

INPUT="${INPUT:-$ROOT/examples/fixtures/classify-decisions.jsonl}"
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

BEFORE="$(cksum "$ESTATE")"
echo "== classify-prepare (offline; not a live train) =="
echo "SKIP live train"
"${ESTATE_CMD[@]}" classify prepare \
  --input "$INPUT" \
  --out "$OUT" \
  --format "${FORMAT:-sharegpt}" \
  --force
AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  examples/estate.yaml cksum changed" >&2
  exit 1
fi
echo "READY_FOR_LIVE_TEST: no"
echo "out: $OUT"
