#!/usr/bin/env bash
# Train prepare fixture: example pack -> Unsloth script, plus an Axolotl recipe.
# Throwaway dir. No Unsloth install. No Axolotl binary. No GPU. No live train.
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

echo "== train-prepare (Unsloth script and Axolotl recipe; not a live train) =="
echo "workdir: $WORKDIR"
echo "SKIP live train"

echo "-- stock estate refuses a binding-id base model --"
set +e
estate enrich prepare \
  --estate "$ESTATE" \
  --pack "$PACK" \
  --driver unsloth-qlora \
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

echo "-- unsloth-qlora default job is train --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver unsloth-qlora \
  --out "$WORKDIR/unsloth"

test -f "$WORKDIR/unsloth/train_unsloth.py"
test -f "$WORKDIR/unsloth/dataset.jsonl"
test -f "$WORKDIR/unsloth/PREPARE.md"
test -f "$WORKDIR/unsloth/NEXT.md"
test -f "$WORKDIR/unsloth/prepare.json"
grep -q "load_in_4bit=True" "$WORKDIR/unsloth/train_unsloth.py"
grep -q "LORA_R = 16" "$WORKDIR/unsloth/train_unsloth.py"
grep -q "MAX_SEQ_LENGTH = 512" "$WORKDIR/unsloth/train_unsloth.py"
grep -q 'MODEL_NAME = "llama3"' "$WORKDIR/unsloth/train_unsloth.py"
grep -q "feed/events.jsonl" "$WORKDIR/unsloth/dataset.jsonl"
grep -q "python $WORKDIR/unsloth/train_unsloth.py" "$WORKDIR/unsloth/NEXT.md"
grep -q "pip install unsloth" "$WORKDIR/unsloth/NEXT.md"
grep -q "saving-to-ollama" "$WORKDIR/unsloth/NEXT.md"
grep -q "python train_unsloth.py" "$WORKDIR/unsloth/PREPARE.md"
if grep -q "python train" "$WORKDIR/unsloth/prepare.json"; then
  echo "FAIL  prepare.json must not embed a train command"
  exit 1
fi
python3 -m py_compile "$WORKDIR/unsloth/train_unsloth.py"

python3 - "$WORKDIR/unsloth/prepare.json" "$WORKDIR/unsloth/dataset.jsonl" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("schema") != "cell-one.enrich-prepare.v0":
    raise SystemExit(f"FAIL  schema={prepare.get('schema')}")
if prepare.get("driver") != "unsloth-qlora":
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
for name in ("train_unsloth.py", "dataset.jsonl", "NEXT.md", "PREPARE.md", "prepare.json"):
    if name not in artifacts:
        raise SystemExit(f"FAIL  artifacts missing {name}: {artifacts}")
rows = [line for line in open(sys.argv[2]) if line.strip()]
if not rows:
    raise SystemExit("FAIL  dataset.jsonl is empty")
for line in rows:
    row = json.loads(line)
    messages = row.get("messages")
    if not isinstance(messages, list) or len(messages) < 2:
        raise SystemExit(f"FAIL  chat row: {row}")
    roles = [item.get("role") for item in messages]
    if "user" not in roles or "assistant" not in roles:
        raise SystemExit(f"FAIL  chat roles: {roles}")
PY

echo "-- enrich job on unsloth-qlora refuses before write --"
set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver unsloth-qlora \
  --job enrich \
  --out "$WORKDIR/enrich-job" \
  >/tmp/train-prepare-enrich.out 2>/tmp/train-prepare-enrich.err
enrich_rc=$?
set -e
if [[ "$enrich_rc" -eq 0 ]]; then
  echo "FAIL  --job enrich on unsloth-qlora must refuse"
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

echo "-- axolotl-lora still writes a YAML recipe --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-lora \
  --job train \
  --out "$WORKDIR/axolotl"
test -f "$WORKDIR/axolotl/axolotl.yml"
grep -q "adapter: qlora" "$WORKDIR/axolotl/axolotl.yml"
grep -q "load_in_4bit: true" "$WORKDIR/axolotl/axolotl.yml"
python3 - "$WORKDIR/axolotl/axolotl.yml" <<'PY'
import sys
text = open(sys.argv[1]).read()
if "\n  - path: " not in text or "\n    ds_type: json\n    type: alpaca\n" not in text:
    raise SystemExit("FAIL  axolotl.yml datasets list is not indented")
PY
grep -q "axolotl train $WORKDIR/axolotl/axolotl.yml" "$WORKDIR/axolotl/NEXT.md"

echo "-- import-trained records the Unsloth adapter on local_slm --"
ADAPTER="$WORKDIR/adapter"
mkdir -p "$ADAPTER"
printf '{}\n' > "$ADAPTER/adapter_config.json"
estate enrich import-trained \
  --estate "$SEATED" \
  --prepared "$WORKDIR/unsloth" \
  --tag cell-enrich-overnight-traces \
  --adapter "$ADAPTER" \
  | tee "$WORKDIR/import.out"
test -f "$WORKDIR/unsloth/binding-proposal.json"
grep -q "import-trained did not apply" "$WORKDIR/import.out"
grep -q "auto_apply=false" "$WORKDIR/import.out"
python3 - "$WORKDIR/unsloth/binding-proposal.json" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
if doc.get("schema") != "cell-one.enrich-binding-proposal.v0":
    raise SystemExit(f"FAIL  schema={doc.get('schema')}")
if doc.get("driver") != "unsloth-qlora" or doc.get("job") != "train":
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

echo "PASS  train-prepare (Unsloth script, Axolotl recipe, import-trained; SKIP live train)"
