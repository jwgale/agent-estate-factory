#!/usr/bin/env bash
# Opt-in live specialist complete. Requires CELL_LOCAL_ENDPOINT.
# Not in make smoke / make gate-90 / GitHub Actions. Does not invent success.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -z "${CELL_LOCAL_ENDPOINT:-}" ]]; then
  echo "refuse: set CELL_LOCAL_ENDPOINT (make live-specialist is opt-in; not in smoke)" >&2
  exit 1
fi
if echo "$CELL_LOCAL_ENDPOINT" | grep -qiE '5090|4090|m3-max'; then
  echo "refuse: CELL_LOCAL_ENDPOINT encodes a hardware SKU" >&2
  exit 1
fi

exec cargo run -q -p estate-control -- specialist --driver ollama \
  --prompt "Reply with the single word pong."
