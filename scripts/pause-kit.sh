#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
STATE="${STATE_DIR:-.cell}"
ESTATE="${ESTATE:-examples/estate.yaml}"

usage() {
  echo "usage: $0 {stop|start|status}"
  echo "  stop    drop .cell/runtime and .cell/sessions (lane roots stay)"
  echo "  start   validate + apply from persisted estate file"
  echo "  status  show what is persisted vs disposable"
}

stop() {
  cargo run -q -p floor-supervisor -- stop --state-dir "$STATE"
  echo "pause-kit stop: runtime discarded"
  echo "persisted: charter.md $ESTATE schema/ lanes/ plans/ gate-reports/"
}

start() {
  cargo run -q -p estate-control -- validate --estate "$ESTATE"
  cargo run -q -p estate-control -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT"
  echo "pause-kit start: rebound from $ESTATE"
}

status() {
  echo "persisted estate: $ESTATE"
  if [[ -f "$ESTATE" ]]; then
    cargo run -q -p estate-control -- validate --estate "$ESTATE" || true
  fi
  echo "lane roots:"
  find lanes -type f | sort
  echo "plans:"
  find plans -type f | sort
  if [[ -d "$STATE/runtime" ]]; then
    echo "runtime present (disposable): $STATE/runtime"
  else
    echo "runtime absent (paused or never spawned)"
  fi
  if [[ -f "$STATE/actual-state.json" ]]; then
    echo "actual-state present (regenerable): $STATE/actual-state.json"
  else
    echo "actual-state absent"
  fi
}

cmd="${1:-status}"
case "$cmd" in
  stop) stop ;;
  start) start ;;
  status) status ;;
  -h|--help|help) usage ;;
  *) usage; exit 2 ;;
esac
