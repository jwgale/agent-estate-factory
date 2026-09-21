#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
STATE="${STATE_DIR:-.cell}"
ESTATE="${ESTATE:-examples/estate.yaml}"

usage() {
  echo "usage: $0 {stop|start|status}"
  echo "  stop    estate suspend: drop sessions/PIDs; keep lifecycle.json"
  echo "  start   estate resume: validate + apply from persisted estate file"
  echo "  status  estate status: persisted vs disposable + placements"
}

stop() {
  cargo run -q -p estate-control -- suspend --state-dir "$STATE"
  echo "pause-kit stop: runtime discarded; lifecycle.json durable"
  echo "persisted: charter.md $ESTATE schema/ lanes/ plans/ gate-reports/ $STATE/lifecycle.json $STATE/placement-actual.json $STATE/apply-audit.jsonl"
}

start() {
  cargo run -q -p estate-control -- resume --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT"
  echo "pause-kit start: rebound from $ESTATE"
}

status() {
  cargo run -q -p estate-control -- status --estate "$ESTATE" --state-dir "$STATE" --roots-base "$ROOT" || true
  echo "lane roots:"
  find lanes -type f 2>/dev/null | sort || true
  echo "plans:"
  find plans -type f 2>/dev/null | sort || true
}

cmd="${1:-status}"
case "$cmd" in
  stop) stop ;;
  start) start ;;
  status) status ;;
  -h|--help|help) usage ;;
  *) usage; exit 2 ;;
esac
