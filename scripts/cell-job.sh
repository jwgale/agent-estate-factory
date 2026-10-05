#!/usr/bin/env bash
# Opt-in harness job contract. Writes a job file or runs one job once.
# Not a scheduler. Not a chat UI. Not in make smoke, make gate-90, or GitHub Actions.
# A standalone train starts only when that job's config sets allow_launch.
# A run step uses one approve file for its GPU steps and does not need allow_launch.
# That train step calls train-step.sh with --wait. A detached launch is not a pass.
# A job that ended refused can be run again by id. It is not re-submitted.
# The harness process blanks CUDA. A passed launch lock clears that blank
# for the train-step.sh child only. The default GPU check is a read-only
# nvidia-smi query, not a training job. Does not flip READY.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
BEFORE="$(cksum "$ESTATE")"
export CUDA_VISIBLE_DEVICES=""
export PYTHONDONTWRITEBYTECODE=1
set +e
python3 "$ROOT/scripts/factory-steps/harness_contract/cli.py" "$@"
RC=$?
set -e
AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  examples/estate.yaml cksum changed" >&2
  exit 1
fi
echo "READY_FOR_LIVE_TEST: no"
exit "$RC"
