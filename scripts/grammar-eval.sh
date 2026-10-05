#!/usr/bin/env bash
# Opt-in factory step: grammar-eval.sh CONFIG.yaml (schema cell-one.quant-ladder.v0, docs/QUANT-LADDER.md).
# Generic: model, quants, eval set + sha256, grammar, scorer command, repeats, ctx and threads come from the config.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions. Does not seat, promote, apply, or run Ollama.
# Does not download weights. Does not invent a live PASS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
STEP=eval; CONFIG="${1:-}"
if [[ -z "$CONFIG" || ! -f "$CONFIG" ]]; then
  echo "FAIL  config not found: ${CONFIG:-<none>} (usage: scripts/grammar-eval.sh CONFIG.yaml)" >&2
  exit 2
fi
PY="${QUANT_LADDER_PYTHON:-$HOME/venvs/llamafactory/bin/python}"
if [[ ! -x "$PY" ]]; then
  echo "FAIL  python not executable: $PY (set QUANT_LADDER_PYTHON; needs PyYAML and transformers)" >&2
  exit 1
fi
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
BEFORE="$(cksum "$ESTATE")"
echo "== grammar-eval ($STEP; local output; not a live PASS) =="
set +e
"$PY" "$ROOT/scripts/factory-steps/factory_steps.py" "$STEP" --config "$CONFIG"
RC=$?
set -e
AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  examples/estate.yaml cksum changed" >&2
  exit 1
fi
echo "READY_FOR_LIVE_TEST: no"
exit $RC
