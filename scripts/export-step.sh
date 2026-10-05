#!/usr/bin/env bash
# Opt-in factory step: export-step.sh CONFIG.yaml [--dry-run|--status] [--wait] [--max-minutes N] [--poll-seconds S]
# Wraps `llamafactory-cli export` (docs/FACTORY-STEPS-INTERFACE.md). CONFIG is a LLaMA-Factory yaml or a
# cell-one.train-step.v0 wrapper that points at one. Paths inside the LLaMA-Factory yaml are relative to the repo root.
# Launches detached (setsid nohup), writes .cell/<cell>/export.pid and .cell/<cell>/logs/export.log; stops only by PID.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions. Does not seat, promote, apply, or run Ollama.
# Does not download weights. Does not invent a live PASS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
STEP=export; CONFIG="${1:-}"
if [[ -z "$CONFIG" || ! -f "$CONFIG" ]]; then
  echo "FAIL  config not found: ${CONFIG:-<none>} (usage: scripts/export-step.sh CONFIG.yaml [--dry-run|--status] [--wait] [--max-minutes N])" >&2
  exit 2
fi
shift
PY="${QUANT_LADDER_PYTHON:-$HOME/venvs/llamafactory/bin/python}"
if [[ ! -x "$PY" ]]; then
  echo "FAIL  python not executable: $PY (set QUANT_LADDER_PYTHON; needs PyYAML)" >&2
  exit 1
fi
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
BEFORE="$(cksum "$ESTATE")"
echo "== export-step ($STEP; local output; not a live PASS) =="
set +e
"$PY" "$ROOT/scripts/factory-steps/factory_steps.py" "$STEP" --config "$CONFIG" "$@"
RC=$?
set -e
AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  examples/estate.yaml cksum changed" >&2
  exit 1
fi
echo "READY_FOR_LIVE_TEST: no"
exit $RC
