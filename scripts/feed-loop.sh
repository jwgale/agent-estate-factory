#!/usr/bin/env bash
# Feed loop fixture walk: scrubbed trace → pack → propose → accept.
# Isolated cell. Fixtures only. No live Grok / Mac / GPU.
# Local only. Do not add to make smoke or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

WORKDIR="${WORKDIR:-$ROOT/target/feed-loop-cell}"
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
FEED="$WORKDIR/feed"
DROP="$WORKDIR/packs"
STATE="$WORKDIR/state"

rm -rf "$WORKDIR"
mkdir -p "$FEED" "$DROP" "$STATE"

echo "== feed-loop (fixtures only) =="
echo "workdir: $WORKDIR"

echo "-- mock traces (mixed path; no live keys) --"
cargo run -q -p model-estate -- task --estate "$ESTATE" \
  --agent horizon --act model --object xai_grok --mock \
  --payload "Reply with the single word pong." \
  --feed-dir "$FEED"
cargo run -q -p model-estate -- task --estate "$ESTATE" \
  --agent research --act tool --object notes-append --mock \
  --payload "append a note" \
  --feed-dir "$FEED"

if [[ ! -f "$FEED/events.jsonl" ]]; then
  echo "FAIL  events.jsonl missing after mock traces"
  exit 1
fi
if grep -Eiq 'xai-|sk-|api_key=|XAI_API_KEY|5090|4090' "$FEED/events.jsonl"; then
  echo "FAIL  feed events must stay scrubbed (no keys / SKUs)"
  cat "$FEED/events.jsonl"
  exit 1
fi
echo "PASS  scrubbed traces"

echo "-- feed pack --"
BEFORE="$(cksum "$ESTATE")"
cargo run -q -p estate-control -- feed pack --feed-dir "$FEED" --drop-dir "$DROP" --id overnight-traces
if [[ ! -f "$DROP/overnight-traces.pack.json" ]]; then
  echo "FAIL  candidate pack missing"
  exit 1
fi
echo "PASS  pack"

echo "-- feed cursor (durable watermark) --"
cargo run -q -p estate-control -- feed cursor --feed-dir "$FEED" | tee "$WORKDIR/cursor-1.json"
if [[ ! -f "$FEED/feed-cursor.json" ]]; then
  echo "FAIL  feed-cursor.json missing after pack"
  exit 1
fi
if ! grep -q 'cell-one.feed-cursor.v0' "$FEED/feed-cursor.json"; then
  echo "FAIL  cursor schema must be cell-one.feed-cursor.v0"
  cat "$FEED/feed-cursor.json"
  exit 1
fi
if ! grep -q '"packed_id": "overnight-traces"' "$FEED/feed-cursor.json"; then
  echo "FAIL  pack must stamp packed_id"
  cat "$FEED/feed-cursor.json"
  exit 1
fi
EVENTS="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("events",0))' "$FEED/feed-cursor.json")"
if [[ "$EVENTS" -lt 1 ]]; then
  echo "FAIL  cursor events must be > 0"
  exit 1
fi
echo "PASS  cursor schema + packed_id + events=$EVENTS"

echo "-- rematerialize (cursor stays; no auto-promote) --"
cargo run -q -p estate-control -- feed pack --feed-dir "$FEED" --drop-dir "$DROP" --id overnight-traces
if [[ ! -f "$FEED/feed-cursor.json" ]]; then
  echo "FAIL  rematerialize dropped feed-cursor.json"
  exit 1
fi
if ! grep -q '"packed_id": "overnight-traces"' "$FEED/feed-cursor.json"; then
  echo "FAIL  rematerialize must keep packed_id"
  cat "$FEED/feed-cursor.json"
  exit 1
fi
echo "PASS  rematerialize keeps cursor"

echo "-- packs propose (never auto-apply) --"
cargo run -q -p estate-control -- packs propose --id overnight-traces \
  --drop-dir "$DROP" --accepted-dir "$DROP/accepted" --proposed-dir "$DROP/proposed" --estate "$ESTATE"
if [[ ! -f "$DROP/proposed/overnight-traces.proposal.json" ]]; then
  echo "FAIL  proposal missing"
  exit 1
fi
if ! grep -q '"auto_apply": false' "$DROP/proposed/overnight-traces.proposal.json"; then
  echo "FAIL  proposal must set auto_apply=false"
  exit 1
fi
echo "PASS  propose"

echo "-- packs accept --curator jason (instructions only) --"
set +e
cargo run -q -p estate-control -- packs accept --id overnight-traces \
  --curator robot --proposed-dir "$DROP/proposed" --accepted-dir "$DROP/accepted" --estate "$ESTATE" \
  >/tmp/feed-loop-curator.out 2>/tmp/feed-loop-curator.err
bad_curator=$?
set -e
if [[ "$bad_curator" -eq 0 ]]; then
  echo "FAIL  wrong curator must refuse"
  exit 1
fi
if ! grep -q "refuse:curator" /tmp/feed-loop-curator.out /tmp/feed-loop-curator.err; then
  echo "FAIL  wrong curator must print refuse:curator"
  cat /tmp/feed-loop-curator.out /tmp/feed-loop-curator.err
  exit 1
fi
cargo run -q -p estate-control -- packs accept --id overnight-traces \
  --curator jason --proposed-dir "$DROP/proposed" --accepted-dir "$DROP/accepted" --estate "$ESTATE"
if [[ ! -f "$DROP/accepted/overnight-traces.enrich-edit.md" ]]; then
  echo "FAIL  accept must write enrich-edit instructions"
  exit 1
fi
if ! grep -q 'applied_to_estate: false' "$DROP/accepted/overnight-traces.enrich-edit.md" \
  && ! grep -q '"applied_to_estate": false' "$DROP/accepted/overnight-traces.enrich-edit.json"; then
  echo "FAIL  accept must record applied_to_estate=false"
  exit 1
fi
echo "PASS  accept"

echo "-- promote refuse + estate unchanged --"
set +e
cargo run -q -p estate-control -- packs promote --id overnight-traces \
  >/tmp/feed-loop-promo.out 2>/tmp/feed-loop-promo.err
promo=$?
set -e
if [[ "$promo" -eq 0 ]]; then
  echo "FAIL  packs promote must refuse"
  exit 1
fi
AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  feed-loop rewrote the estate"
  exit 1
fi
if [[ ! -f "$FEED/feed-cursor.json" ]]; then
  echo "FAIL  cursor missing at end of loop"
  exit 1
fi
echo "PASS  promote refused; estate unchanged; cursor durable"

echo
echo "FEED-LOOP GREEN (fixtures only; no live Grok / Mac / GPU)"
