#!/usr/bin/env bash
# Enrich prepare fixture: example pack -> Modelfile and external manifest.
# Throwaway dir. No Ollama binary. No live train. No GPU.
# Local only. Do not add to make smoke or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

WORKDIR="${WORKDIR:-$ROOT/target/enrich-prepare-cell}"
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
PACK="${PACK:-$ROOT/examples/fixtures/specialist-overnight.pack.json}"
BIN="${ESTATE_BIN:-}"

estate() {
  if [[ -n "$BIN" ]]; then
    "$BIN" "$@"
  else
    cargo run -q -p estate-control -- "$@"
  fi
}

if [[ ! -f "$PACK" ]]; then
  echo "SKIP enrich-prepare (example pack missing; not a PASS)"
  exit 0
fi

BEFORE="$(cksum "$ESTATE")"
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"

echo "== enrich-prepare (fixtures only; not a live train) =="
echo "workdir: $WORKDIR"
echo "SKIP live train (prepare writes artifacts only; not a PASS)"

echo "-- ollama-modelfile --"
estate enrich prepare \
  --estate "$ESTATE" \
  --pack "$PACK" \
  --driver ollama-modelfile \
  --job enrich \
  --out "$WORKDIR/ollama"

echo "-- external-manifest --"
estate enrich prepare \
  --estate "$ESTATE" \
  --pack "$PACK" \
  --driver external-manifest \
  --job enrich \
  --out "$WORKDIR/manifest"

test -f "$WORKDIR/ollama/Modelfile"
test -f "$WORKDIR/ollama/PREPARE.md"
test -f "$WORKDIR/ollama/prepare.json"
grep -q "FROM local_slm" "$WORKDIR/ollama/Modelfile"
grep -q "ollama create cell-enrich-overnight-traces -f Modelfile" "$WORKDIR/ollama/PREPARE.md"

test -f "$WORKDIR/manifest/manifest.json"
test -f "$WORKDIR/manifest/manifest.yaml"
test -f "$WORKDIR/manifest/PREPARE.md"
test -f "$WORKDIR/manifest/prepare.json"

python3 - "$WORKDIR/manifest/manifest.json" "$WORKDIR/ollama/prepare.json" <<'PY'
import json, sys
manifest = json.load(open(sys.argv[1]))
prepare = json.load(open(sys.argv[2]))
if manifest.get("vendor") is not None:
    raise SystemExit(f"FAIL  vendor lock: {manifest.get('vendor')}")
if manifest.get("promoted") is not False or manifest.get("auto_apply") is not False:
    raise SystemExit("FAIL  manifest must stay unpromoted")
if manifest.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  manifest claims an estate rewrite")
if "ollama" in json.dumps(manifest).lower():
    raise SystemExit("FAIL  external manifest names ollama")
if prepare.get("schema") != "cell-one.enrich-prepare.v0":
    raise SystemExit(f"FAIL  schema={prepare.get('schema')}")
if prepare.get("promoted") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  prepare.json must stay unpromoted and off the estate")
if "Modelfile" not in prepare.get("artifacts", []):
    raise SystemExit(f"FAIL  artifacts={prepare.get('artifacts')}")
PY

echo "-- missing pack refuses before write --"
set +e
estate enrich prepare \
  --estate "$ESTATE" \
  --pack missing-enrich-pack \
  --packs-dir "$WORKDIR/none" \
  --out "$WORKDIR/missing" \
  >/tmp/enrich-prepare-missing.out 2>/tmp/enrich-prepare-missing.err
missing_rc=$?
set -e
if [[ "$missing_rc" -eq 0 ]]; then
  echo "FAIL  missing pack must refuse"
  exit 1
fi
if ! grep -q "refuse:missing-pack" /tmp/enrich-prepare-missing.out /tmp/enrich-prepare-missing.err; then
  echo "FAIL  missing pack did not refuse:missing-pack"
  cat /tmp/enrich-prepare-missing.out /tmp/enrich-prepare-missing.err
  exit 1
fi
if [[ -d "$WORKDIR/missing" ]]; then
  echo "FAIL  missing pack wrote an output directory"
  exit 1
fi

AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  enrich-prepare rewrote the estate"
  exit 1
fi

echo "PASS  enrich-prepare (artifacts only; not a live train)"
