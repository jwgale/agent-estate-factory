#!/usr/bin/env bash
# Feed loop fixture walk: scrubbed trace → pack → propose → accept.
# Isolated cell. Fixtures only. No live Grok / Mac / GPU.
# Local only. Do not add to make smoke or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

WORKDIR="${WORKDIR:-$ROOT/target/feed-loop-cell}"
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
FEED="$WORKDIR/feed"
DROP="$WORKDIR/packs"
STATE="$WORKDIR/state"
BIN="${ESTATE_BIN:-}"
MODEL_BIN="${MODEL_ESTATE_BIN:-}"

estate() {
  if [[ -n "$BIN" ]]; then
    "$BIN" "$@"
  else
    cargo run -q -p estate-control -- "$@"
  fi
}

model_estate() {
  if [[ -n "$MODEL_BIN" ]]; then
    "$MODEL_BIN" "$@"
  else
    cargo run -q -p model-estate -- "$@"
  fi
}

assert_source_drivers() {
  local path="$1"
  python3 - "$path" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
if "diff" in doc:
    drivers = doc["diff"].get("source_drivers")
    counts = doc["diff"].get("path_counts") or {}
    if doc.get("auto_apply") is not False:
        raise SystemExit(f"FAIL  auto_apply must stay false: {doc.get('auto_apply')}")
else:
    drivers = doc.get("source_drivers")
    counts = doc.get("path_counts") or {}
    if doc.get("promoted") is not False:
        raise SystemExit(f"FAIL  promoted must stay false: {doc.get('promoted')}")
if drivers != ["frontier", "local"]:
    raise SystemExit(f"FAIL  source_drivers={drivers} in {sys.argv[1]}")
if int(counts.get("frontier") or 0) < 1 or int(counts.get("local") or 0) < 1:
    raise SystemExit(f"FAIL  path_counts={counts} in {sys.argv[1]}")
PY
}

rm -rf "$WORKDIR"
mkdir -p "$FEED" "$DROP" "$STATE"

echo "== feed-loop (fixtures only) =="
echo "workdir: $WORKDIR"

echo "-- mock traces (mixed path; no live keys) --"
model_estate task --estate "$ESTATE" \
  --agent horizon --act model --object xai_grok --mock \
  --payload "Reply with the single word pong." \
  --feed-dir "$FEED"
model_estate task --estate "$ESTATE" \
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
estate feed pack --feed-dir "$FEED" --drop-dir "$DROP" --id overnight-traces
if [[ ! -f "$DROP/overnight-traces.pack.json" ]]; then
  echo "FAIL  candidate pack missing"
  exit 1
fi
assert_source_drivers "$DROP/overnight-traces.pack.json"
if ! grep -q 'drivers=frontier,local' "$DROP/INDEX.md"; then
  echo "FAIL  INDEX.md must list source_drivers"
  cat "$DROP/INDEX.md"
  exit 1
fi
echo "PASS  pack source_drivers frontier, local"

echo "-- feed cursor (durable watermark) --"
estate feed cursor --feed-dir "$FEED" | tee "$WORKDIR/cursor-1.json"
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
estate feed pack --feed-dir "$FEED" --drop-dir "$DROP" --id overnight-traces
if [[ ! -f "$FEED/feed-cursor.json" ]]; then
  echo "FAIL  rematerialize dropped feed-cursor.json"
  exit 1
fi
if ! grep -q '"packed_id": "overnight-traces"' "$FEED/feed-cursor.json"; then
  echo "FAIL  rematerialize must keep packed_id"
  cat "$FEED/feed-cursor.json"
  exit 1
fi
assert_source_drivers "$DROP/overnight-traces.pack.json"
echo "PASS  rematerialize keeps cursor and source_drivers"

echo "-- packs propose (never auto-apply) --"
estate packs propose --id overnight-traces \
  --drop-dir "$DROP" --accepted-dir "$DROP/accepted" --proposed-dir "$DROP/proposed" --estate "$ESTATE"
if [[ ! -f "$DROP/proposed/overnight-traces.proposal.json" ]]; then
  echo "FAIL  proposal missing"
  exit 1
fi
if ! grep -q '"auto_apply": false' "$DROP/proposed/overnight-traces.proposal.json"; then
  echo "FAIL  proposal must set auto_apply=false"
  exit 1
fi
assert_source_drivers "$DROP/proposed/overnight-traces.proposal.json"
echo "PASS  propose source_drivers frontier, local"

echo "-- packs accept --curator jason (instructions only) --"
set +e
estate packs accept --id overnight-traces \
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
estate packs accept --id overnight-traces \
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
python3 - \
  "$DROP/overnight-traces.pack.json" \
  "$DROP/proposed/overnight-traces.proposal.json" \
  "$DROP/accepted/overnight-traces.enrich-edit.json" <<'PY'
import json, sys
pack, proposal, edit = [json.load(open(p)) for p in sys.argv[1:]]
survived = [
    pack.get("source_drivers"),
    (proposal.get("diff") or {}).get("source_drivers"),
    edit.get("source_drivers"),
]
if survived != [["frontier", "local"]] * 3:
    raise SystemExit(f"FAIL  source_drivers must survive pack -> propose -> enrich-edit: {survived}")
if edit.get("applied_to_estate") is not False or edit.get("auto_apply") is not False:
    raise SystemExit("FAIL  enrich-edit must stay unapplied")
PY
if ! grep -q 'source_drivers: frontier, local' "$DROP/accepted/overnight-traces.enrich-edit.md"; then
  echo "FAIL  enrich-edit instructions must keep source_drivers"
  exit 1
fi
echo "PASS  accept source_drivers frontier, local"

echo "-- promote refuse + estate unchanged --"
set +e
estate packs promote --id overnight-traces \
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
