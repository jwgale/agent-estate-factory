#!/usr/bin/env bash
# Opt-in seated-runtime enrich handoff prove.
# prepare/from-pack on a throwaway cell, ollama create, import-prepared, ollama show.
# Removes the tag it created. Does not edit examples/estate.yaml.
# Not a factory-wide live test. READY_FOR_LIVE_TEST stays no.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
PACK="${PACK:-$ROOT/examples/fixtures/specialist-overnight.pack.json}"
# Throwaway dir stays out of the checkout. prepare refuses a hardware SKU
# anywhere in the output path, including a parent directory name.
WORKDIR="${WORKDIR:-${TMPDIR:-/tmp}/cell-one-enrich-live-prove}"
BIN="${ESTATE_BIN:-}"
CREATED=0
TAG=""

estate() {
  if [[ -n "$BIN" ]]; then
    "$BIN" "$@"
  else
    cargo run -q -p estate-control -- "$@"
  fi
}

cleanup() {
  local rc=$?
  if [[ "$CREATED" == 1 && -n "$TAG" ]] && command -v ollama >/dev/null 2>&1; then
    echo "-- remove tag ${TAG} (prove does not keep the derived model) --"
    ollama rm "$TAG" >/dev/null 2>&1 || true
  fi
  exit "$rc"
}
trap cleanup EXIT

if [[ ! -f "$PACK" || ! -f "$ESTATE" ]]; then
  echo "SKIP enrich-live-prove (fixture missing; skipped)"
  exit 0
fi

if ! command -v ollama >/dev/null 2>&1; then
  echo "SKIP enrich-live-prove (ollama not on PATH; skipped)"
  exit 0
fi

if ! ollama list >/tmp/enrich-live-ollama-list.out 2>/tmp/enrich-live-ollama-list.err; then
  echo "SKIP enrich-live-prove (ollama seat is down; skipped)"
  exit 0
fi

pick_base() {
  if [[ -n "${CELL_LOCAL_MODEL:-}" ]]; then
    printf '%s' "$CELL_LOCAL_MODEL"
    return
  fi
  if awk 'NR>1 {print $1}' /tmp/enrich-live-ollama-list.out | grep -Eq '^(llama3|llama3:latest)$'; then
    printf 'llama3'
    return
  fi
  local first
  first="$(awk 'NR>1 && $1 != "" {print $1; exit}' /tmp/enrich-live-ollama-list.out)"
  if [[ -z "$first" ]]; then
    return 1
  fi
  if [[ "$first" == *:latest ]]; then
    first="${first%:latest}"
  fi
  printf '%s' "$first"
}

if ! BASE="$(pick_base)"; then
  echo "SKIP enrich-live-prove (no seated model in ollama list; skipped)"
  exit 0
fi

if echo "$BASE" | grep -qiE '5090|4090|m3-max'; then
  echo "refuse:base-model: seated model encodes a hardware SKU" >&2
  exit 1
fi
if [[ ! "$BASE" =~ ^[A-Za-z0-9._:-]+$ ]]; then
  echo "refuse:base-model: seated model '${BASE}' is not a single FROM token" >&2
  exit 1
fi

BEFORE="$(cksum "$ESTATE")"
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"
SEATED="$WORKDIR/estate.yaml"
python3 - "$ESTATE" "$SEATED" "$BASE" <<'PY'
import sys
src, dest, model = sys.argv[1:]
text = open(src).read()
needle = "  - id: local_slm\n    class: local\n    driver: ollama\n    params:\n"
if needle not in text:
    raise SystemExit("FAIL  local_slm params block missing")
escaped = model.replace("\\", "\\\\").replace('"', '\\"')
open(dest, "w").write(text.replace(needle, needle + f'      model: "{escaped}"\n', 1))
PY

PACK_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["id"])' "$PACK")"
TAG="cell-enrich-${PACK_ID}"
CELL="$WORKDIR/cell"

echo "== enrich-live-prove (seated handoff only; not a factory-wide live test) =="
echo "workdir: $WORKDIR"
echo "base: $BASE"
echo "tag: $TAG"

echo "-- from-pack --"
estate enrich from-pack \
  --estate "$SEATED" \
  --pack "$PACK" \
  --all-drivers \
  --state-dir "$CELL" \
  --job enrich

PREPARED="$CELL/enrich/${PACK_ID}/ollama-modelfile"
MODELFILE="$PREPARED/Modelfile"
test -f "$MODELFILE"
test -f "$CELL/enrich/${PACK_ID}/external-manifest/manifest.json"
grep -F -q "FROM ${BASE}" "$MODELFILE"
if grep -q "FROM local_slm" "$MODELFILE"; then
  echo "FAIL  Modelfile FROM is the binding id"
  exit 1
fi

echo "-- ollama create --"
ollama rm "$TAG" >/dev/null 2>&1 || true
CREATED=1
if command -v timeout >/dev/null 2>&1; then
  timeout 180 ollama create "$TAG" -f "$MODELFILE"
else
  ollama create "$TAG" -f "$MODELFILE"
fi

echo "-- ollama show --"
ollama show "$TAG" | tee "$WORKDIR/show.out"
if ! grep -q . "$WORKDIR/show.out"; then
  echo "FAIL  ollama show returned an empty card"
  exit 1
fi

echo "-- ollama list --"
ollama list | tee "$WORKDIR/list.out"
if ! grep -q "$TAG" "$WORKDIR/list.out"; then
  echo "FAIL  ollama list is missing ${TAG}"
  exit 1
fi

if [[ -n "${CELL_LOCAL_ENDPOINT:-}" ]] && command -v curl >/dev/null 2>&1; then
  echo "-- openai-compat tags --"
  if curl -fsS --max-time 3 "${CELL_LOCAL_ENDPOINT%/}/api/tags" -o "$WORKDIR/api-tags.json"; then
    if ! grep -q "$TAG" "$WORKDIR/api-tags.json"; then
      echo "FAIL  ${CELL_LOCAL_ENDPOINT}/api/tags is missing ${TAG}"
      exit 1
    fi
  else
    echo "SKIP openai-compat (endpoint did not answer; ollama show already ran)"
  fi
fi

echo "-- import-prepared --"
estate enrich import-prepared \
  --estate "$SEATED" \
  --prepared "$PREPARED" \
  --tag "$TAG" \
  --path "$MODELFILE" \
  | tee "$WORKDIR/import.out"
test -f "$PREPARED/binding-proposal.json"
grep -q "auto_apply=false" "$WORKDIR/import.out"
grep -q "import-prepared did not apply" "$WORKDIR/import.out"

python3 - "$PREPARED/binding-proposal.json" "$TAG" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
tag = sys.argv[2]
if doc.get("schema") != "cell-one.enrich-binding-proposal.v0":
    raise SystemExit(f"FAIL  schema={doc.get('schema')}")
if doc.get("auto_apply") is not False or doc.get("promoted") is not False:
    raise SystemExit("FAIL  binding proposal must stay unapplied")
if doc.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  binding proposal claims an estate rewrite")
if doc.get("binding_id") != "local_slm":
    raise SystemExit(f"FAIL  binding_id={doc.get('binding_id')}")
params = doc.get("proposed_binding", {}).get("params", {})
if params.get("model") != tag:
    raise SystemExit(f"FAIL  model={params.get('model')}")
PY

AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  enrich-live-prove rewrote examples/estate.yaml"
  exit 1
fi

echo "PASS  enrich-live-prove (seated handoff only; not a factory-wide live test)"
