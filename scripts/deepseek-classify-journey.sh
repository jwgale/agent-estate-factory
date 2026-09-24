#!/usr/bin/env bash
# Opt-in DeepSeek-R1-Distill classify journey. Default is print and local llamafactory-cli train.
# Set DEEPSEEK_CLASSIFY_RUN=1 to execute train, merge, GGUF, Ollama, and eval.
# Together stays on the tev1 Qwen path unless TRAIN_DRIVER=together and TOGETHER_MODEL are both set.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
# Does not invent a live PASS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

INPUT="${INPUT:-$ROOT/examples/fixtures/tev1-decisions.jsonl}"
OUT="${OUT:-${TMPDIR:-/tmp}/cell-one-deepseek-classify-journey}"
BASE="${BASE:-deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B}"
TAG="${TAG:-deepseek-r1-distill-specialist}"
QUANT="${QUANT:-Q4_K_M}"
ENDPOINT="${ENDPOINT:-http://127.0.0.1:11434}"
TRAIN_DRIVER="${TRAIN_DRIVER:-local}"
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
echo "== deepseek-classify-journey (local output; not a live PASS) =="
ARGS=(
  classify journey
  --preset deepseek-r1-distill
  --input "$INPUT"
  --out "$OUT"
  --base "$BASE"
  --tag "$TAG"
  --quant "$QUANT"
  --endpoint "$ENDPOINT"
)
if [[ -n "${LLAMA_CPP_DIR:-}" ]]; then
  ARGS+=(--llama-cpp-dir "$LLAMA_CPP_DIR")
fi
if [[ -n "${BASE_TAG:-}" ]]; then
  ARGS+=(--base-tag "$BASE_TAG")
fi
if [[ "$TRAIN_DRIVER" != "local" && "$TRAIN_DRIVER" != "together" ]]; then
  echo "FAIL  TRAIN_DRIVER must be local or together" >&2
  exit 1
fi
if [[ "$TRAIN_DRIVER" == "together" && -z "${TOGETHER_MODEL:-}" ]]; then
  echo "FAIL  DeepSeek-R1-Distill journey uses local llamafactory-cli train. Together stays on the tev1 Qwen path unless TOGETHER_MODEL is set" >&2
  exit 1
fi
ARGS+=(--train-driver "$TRAIN_DRIVER")
if [[ -n "${TOGETHER_MODEL:-}" ]]; then
  ARGS+=(--together-model "$TOGETHER_MODEL")
fi
if [[ -n "${TOGETHER_API_KEY_ENV:-}" ]]; then
  ARGS+=(--api-key-env "$TOGETHER_API_KEY_ENV")
fi
if [[ "${DEEPSEEK_CLASSIFY_RUN:-}" == "1" ]]; then
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
