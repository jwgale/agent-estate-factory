#!/usr/bin/env bash
# Train prepare fixture: example pack -> Axolotl recipe, then import-trained.
# Throwaway dir. No Axolotl binary. No GPU. No live train.
# Local only. Do not add to make smoke or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

# Throwaway dir stays out of the checkout. prepare refuses a hardware SKU
# anywhere in the output path, including a parent directory name.
WORKDIR="${WORKDIR:-${TMPDIR:-/tmp}/cell-one-train-prepare}"
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
  echo "SKIP train-prepare (example pack missing; not a PASS)"
  exit 0
fi

BEFORE="$(cksum "$ESTATE")"
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"
SEATED="$WORKDIR/estate.yaml"
python3 - "$ESTATE" "$SEATED" <<'PY'
import sys
src, dest = sys.argv[1:]
text = open(src).read()
needle = "  - id: local_slm\n    class: local\n    driver: ollama\n    params:\n"
if needle not in text:
    raise SystemExit("FAIL  local_slm params block missing")
open(dest, "w").write(text.replace(needle, needle + '      model: "llama3"\n', 1))
PY

echo "== train-prepare (Axolotl recipe only; not a live train) =="
echo "workdir: $WORKDIR"
echo "SKIP live train"

echo "-- stock estate refuses a binding-id base model --"
set +e
estate enrich prepare \
  --estate "$ESTATE" \
  --pack "$PACK" \
  --driver axolotl-lora \
  --job train \
  --out "$WORKDIR/stock" \
  >/tmp/train-prepare-stock.out 2>/tmp/train-prepare-stock.err
stock_rc=$?
set -e
if [[ "$stock_rc" -eq 0 ]]; then
  echo "FAIL  stock estate must refuse:base-model"
  exit 1
fi
if ! grep -q "refuse:base-model" /tmp/train-prepare-stock.out /tmp/train-prepare-stock.err; then
  echo "FAIL  stock estate did not refuse:base-model"
  cat /tmp/train-prepare-stock.out /tmp/train-prepare-stock.err
  exit 1
fi
if [[ -e "$WORKDIR/stock" ]]; then
  echo "FAIL  stock estate wrote an output directory"
  exit 1
fi

echo "-- axolotl-lora default job is train --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-lora \
  --out "$WORKDIR/axolotl"

test -f "$WORKDIR/axolotl/axolotl.yml"
test -f "$WORKDIR/axolotl/dataset.jsonl"
test -f "$WORKDIR/axolotl/PREPARE.md"
test -f "$WORKDIR/axolotl/NEXT.md"
test -f "$WORKDIR/axolotl/prepare.json"
grep -q "adapter: qlora" "$WORKDIR/axolotl/axolotl.yml"
grep -q "load_in_4bit: true" "$WORKDIR/axolotl/axolotl.yml"
grep -q 'base_model: "llama3"' "$WORKDIR/axolotl/axolotl.yml"
python3 - "$WORKDIR/axolotl/axolotl.yml" <<'PY'
import sys
text = open(sys.argv[1]).read()
needle = "\n  - path: "
if needle not in text or "\n    ds_type: json\n    type: alpaca\n" not in text:
    raise SystemExit("FAIL  axolotl.yml datasets list is not indented")
PY
grep -q "feed/events.jsonl" "$WORKDIR/axolotl/dataset.jsonl"
grep -q "axolotl train $WORKDIR/axolotl/axolotl.yml" "$WORKDIR/axolotl/NEXT.md"
grep -q "import-trained" "$WORKDIR/axolotl/NEXT.md"
grep -q "axolotl train axolotl.yml" "$WORKDIR/axolotl/PREPARE.md"
if grep -q "axolotl train" "$WORKDIR/axolotl/prepare.json"; then
  echo "FAIL  prepare.json must not embed a train command"
  exit 1
fi

python3 - "$WORKDIR/axolotl/prepare.json" "$WORKDIR/axolotl/dataset.jsonl" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("schema") != "cell-one.enrich-prepare.v0":
    raise SystemExit(f"FAIL  schema={prepare.get('schema')}")
if prepare.get("driver") != "axolotl-lora":
    raise SystemExit(f"FAIL  driver={prepare.get('driver')}")
if prepare.get("job") != "train":
    raise SystemExit(f"FAIL  job={prepare.get('job')}")
if prepare.get("base_model") != "llama3":
    raise SystemExit(f"FAIL  base_model={prepare.get('base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False:
    raise SystemExit("FAIL  prepare.json must stay unpromoted")
if prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  prepare.json claims an estate rewrite")
artifacts = prepare.get("artifacts", [])
for name in ("axolotl.yml", "dataset.jsonl", "NEXT.md", "PREPARE.md", "prepare.json"):
    if name not in artifacts:
        raise SystemExit(f"FAIL  artifacts missing {name}: {artifacts}")
rows = [line for line in open(sys.argv[2]) if line.strip()]
if not rows:
    raise SystemExit("FAIL  dataset.jsonl is empty")
for line in rows:
    row = json.loads(line)
    for key in ("instruction", "input", "output"):
        if key not in row:
            raise SystemExit(f"FAIL  dataset row missing {key}")
PY

echo "-- enrich job on axolotl-lora refuses before write --"
set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-lora \
  --job enrich \
  --out "$WORKDIR/enrich-job" \
  >/tmp/train-prepare-enrich.out 2>/tmp/train-prepare-enrich.err
enrich_rc=$?
set -e
if [[ "$enrich_rc" -eq 0 ]]; then
  echo "FAIL  --job enrich on axolotl-lora must refuse"
  exit 1
fi
if ! grep -q "refuse:job" /tmp/train-prepare-enrich.out /tmp/train-prepare-enrich.err; then
  echo "FAIL  --job enrich did not refuse:job"
  cat /tmp/train-prepare-enrich.out /tmp/train-prepare-enrich.err
  exit 1
fi
if [[ -e "$WORKDIR/enrich-job" ]]; then
  echo "FAIL  --job enrich wrote an output directory"
  exit 1
fi

echo "-- import-trained records the adapter on local_slm --"
ADAPTER="$WORKDIR/adapter"
mkdir -p "$ADAPTER"
printf '{}\n' > "$ADAPTER/adapter_config.json"
estate enrich import-trained \
  --estate "$SEATED" \
  --prepared "$WORKDIR/axolotl" \
  --tag cell-enrich-overnight-traces \
  --adapter "$ADAPTER" \
  | tee "$WORKDIR/import.out"
test -f "$WORKDIR/axolotl/binding-proposal.json"
grep -q "import-trained did not apply" "$WORKDIR/import.out"
grep -q "auto_apply=false" "$WORKDIR/import.out"
python3 - "$WORKDIR/axolotl/binding-proposal.json" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
if doc.get("schema") != "cell-one.enrich-binding-proposal.v0":
    raise SystemExit(f"FAIL  schema={doc.get('schema')}")
if doc.get("driver") != "axolotl-lora" or doc.get("job") != "train":
    raise SystemExit(f"FAIL  driver={doc.get('driver')} job={doc.get('job')}")
if doc.get("binding_id") != "local_slm":
    raise SystemExit(f"FAIL  binding_id={doc.get('binding_id')}")
if doc.get("auto_apply") is not False or doc.get("promoted") is not False:
    raise SystemExit("FAIL  binding proposal must stay unapplied")
if doc.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  binding proposal claims an estate rewrite")
params = doc.get("proposed_binding", {}).get("params", {})
if params.get("model") != "cell-enrich-overnight-traces":
    raise SystemExit(f"FAIL  model={params.get('model')}")
PY

AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  train-prepare rewrote examples/estate.yaml"
  exit 1
fi

echo "PASS  train-prepare (Axolotl recipe and import-trained; SKIP live train)"
