#!/usr/bin/env bash
# Opt-in live specialist complete. Never add to smoke or GitHub Actions.
# Runs only when CELL_LIVE_PROBE=1|true|yes. Never requires XAI_API_KEY.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

probe="$(printf '%s' "${CELL_LIVE_PROBE:-}" | tr '[:upper:]' '[:lower:]')"
case "$probe" in
  1|true|yes) ;;
  *)
    echo "live-specialist-smoke SKIP (set CELL_LIVE_PROBE=1; not in smoke or CI)"
    exit 0
    ;;
esac

if [[ -z "${CELL_LOCAL_ENDPOINT:-}" && -z "${CELL_RENTED_ENDPOINT:-}" ]]; then
  echo "live-specialist-smoke SKIP (no CELL_LOCAL_ENDPOINT / CELL_RENTED_ENDPOINT)"
  exit 0
fi

# Frontier / Grok is a separate stub. Do not read XAI_API_KEY here.
PROMPT="${LIVE_SPECIALIST_PROMPT:-Reply with the single word pong.}"
echo "== live-specialist-smoke (CELL_LIVE_PROBE set; XAI_API_KEY not used) =="
echo "CELL_LOCAL_MODEL=${CELL_LOCAL_MODEL:-<unset; first portable /v1/models or /api/tags id>}"
cargo run -q -p estate-control -- specialist --driver ollama --prompt "$PROMPT"
