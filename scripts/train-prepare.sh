#!/usr/bin/env bash
# Train prepare fixture: example pack -> LLaMA-Factory recipe, plus an Axolotl recipe.
# Throwaway dir. No LLaMA-Factory install. No Axolotl binary. No GPU. No live train.
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
SEATED_ONLY="$WORKDIR/estate-seat-only.yaml"
SEATED="$WORKDIR/estate.yaml"
python3 - "$ESTATE" "$SEATED_ONLY" "$SEATED" <<'PY'
import sys
src, seat_only, seated = sys.argv[1:]
text = open(src).read()
needle = "  - id: local_slm\n    class: local\n    driver: ollama\n    params:\n"
if needle not in text:
    raise SystemExit("FAIL  local_slm params block missing")
open(seat_only, "w").write(text.replace(needle, needle + '      model: "llama3"\n', 1))
open(seated, "w").write(text.replace(
    needle,
    needle + '      model: "llama3"\n      train_base_model: "Qwen/Qwen2.5-0.5B-Instruct"\n',
    1,
))
PY

echo "== train-prepare (LLaMA-Factory recipe and Axolotl recipe; not a live train) =="
echo "workdir: $WORKDIR"
echo "SKIP live train"

echo "-- stock estate refuses a binding-id base model --"
set +e
estate enrich prepare \
  --estate "$ESTATE" \
  --pack "$PACK" \
  --driver llamafactory-qlora \
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

echo "-- seated tag without a train base refuses --"
set +e
estate enrich prepare \
  --estate "$SEATED_ONLY" \
  --pack "$PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/seat-only" \
  >/tmp/train-prepare-seat-only.out 2>/tmp/train-prepare-seat-only.err
seat_rc=$?
set -e
if [[ "$seat_rc" -eq 0 ]]; then
  echo "FAIL  seat tag without a train base must refuse:train-base"
  exit 1
fi
if ! grep -q "refuse:train-base" /tmp/train-prepare-seat-only.out /tmp/train-prepare-seat-only.err; then
  echo "FAIL  seat tag without a train base did not refuse:train-base"
  cat /tmp/train-prepare-seat-only.out /tmp/train-prepare-seat-only.err
  exit 1
fi
if grep -q "meta-llama" /tmp/train-prepare-seat-only.out /tmp/train-prepare-seat-only.err; then
  echo "FAIL  refuse must not invent a Llama-3 Hub repo"
  exit 1
fi
if [[ -e "$WORKDIR/seat-only" ]]; then
  echo "FAIL  seat-only prepare wrote an output directory"
  exit 1
fi

echo "-- llamafactory-qlora default job is train --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/llamafactory"

test -f "$WORKDIR/llamafactory/recipe.yaml"
test -f "$WORKDIR/llamafactory/export.yaml"
test -f "$WORKDIR/llamafactory/dataset_info.json"
test -f "$WORKDIR/llamafactory/dataset.jsonl"
test -f "$WORKDIR/llamafactory/PREPARE.md"
test -f "$WORKDIR/llamafactory/NEXT.md"
test -f "$WORKDIR/llamafactory/prepare.json"
if [[ -e "$WORKDIR/llamafactory/train_unsloth.py" ]]; then
  echo "FAIL  llamafactory-qlora must not write an Unsloth script"
  exit 1
fi
grep -q "stage: sft" "$WORKDIR/llamafactory/recipe.yaml"
grep -q "quantization_bit: 4" "$WORKDIR/llamafactory/recipe.yaml"
grep -q "lora_rank: 16" "$WORKDIR/llamafactory/recipe.yaml"
grep -q "cutoff_len: 512" "$WORKDIR/llamafactory/recipe.yaml"
grep -q "packing: true" "$WORKDIR/llamafactory/recipe.yaml"
grep -q "template: qwen" "$WORKDIR/llamafactory/recipe.yaml"
grep -q 'model_name_or_path: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/llamafactory/recipe.yaml"
grep -q "quantization_method: bnb" "$WORKDIR/llamafactory/recipe.yaml"
if grep -q "quantization_method: bitsandbytes" "$WORKDIR/llamafactory/recipe.yaml"; then
  echo "FAIL  recipe.yaml must keep quantization_method bnb"
  exit 1
fi
if grep -Eq '^max_steps:' "$WORKDIR/llamafactory/recipe.yaml"; then
  echo "FAIL  default recipe must leave max_steps unset"
  exit 1
fi
grep -q '^save_steps: 50$' "$WORKDIR/llamafactory/recipe.yaml"
grep -q 'model_name_or_path: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/llamafactory/export.yaml"
grep -q "template: qwen" "$WORKDIR/llamafactory/export.yaml"
grep -q "feed/events.jsonl" "$WORKDIR/llamafactory/dataset.jsonl"
grep -q "llamafactory-cli train $WORKDIR/llamafactory/recipe.yaml" "$WORKDIR/llamafactory/NEXT.md"
grep -q "llamafactory-cli export $WORKDIR/llamafactory/export.yaml" "$WORKDIR/llamafactory/NEXT.md"
grep -q "pip install llamafactory" "$WORKDIR/llamafactory/NEXT.md"
grep -q "bitsandbytes>=0.49" "$WORKDIR/llamafactory/NEXT.md"
grep -q "2.11.0+cu128" "$WORKDIR/llamafactory/NEXT.md"
grep -q "\-\-max-steps 10" "$WORKDIR/llamafactory/NEXT.md"
grep -q "CUDA LLaMA-Factory" "$WORKDIR/llamafactory/NEXT.md"
grep -q "Faster single-GPU alternate" "$WORKDIR/llamafactory/NEXT.md"
grep -q "llamafactory-cli train recipe.yaml" "$WORKDIR/llamafactory/PREPARE.md"
grep -q "dataset_mode: scaffold" "$WORKDIR/llamafactory/PREPARE.md"
grep -q "dataset_mode: scaffold" "$WORKDIR/llamafactory/NEXT.md"
grep -q "refuse:dataset" "$WORKDIR/llamafactory/NEXT.md"
grep -q "not training data" "$WORKDIR/llamafactory/NEXT.md"
grep -q "Dataset mode: scaffold" "$WORKDIR/llamafactory/PREPARE.md"
if grep -q "quantization_bit" "$WORKDIR/llamafactory/export.yaml"; then
  echo "FAIL  export.yaml must not set quantization_bit"
  exit 1
fi
if grep -q "llamafactory-cli" "$WORKDIR/llamafactory/prepare.json"; then
  echo "FAIL  prepare.json must not embed a train command"
  exit 1
fi

python3 - "$WORKDIR/llamafactory/prepare.json" "$WORKDIR/llamafactory/dataset.jsonl" "$WORKDIR/llamafactory/dataset_info.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("schema") != "cell-one.enrich-prepare.v0":
    raise SystemExit(f"FAIL  schema={prepare.get('schema')}")
if prepare.get("driver") != "llamafactory-qlora":
    raise SystemExit(f"FAIL  driver={prepare.get('driver')}")
if prepare.get("job") != "train":
    raise SystemExit(f"FAIL  job={prepare.get('job')}")
if prepare.get("base_model") != "llama3":
    raise SystemExit(f"FAIL  base_model={prepare.get('base_model')}")
if prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  seat_tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "Qwen/Qwen2.5-0.5B-Instruct":
    raise SystemExit(f"FAIL  train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False:
    raise SystemExit("FAIL  prepare.json must stay unpromoted")
if prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  prepare.json claims an estate rewrite")
if prepare.get("dataset_mode") != "scaffold":
    raise SystemExit(f"FAIL  dataset_mode={prepare.get('dataset_mode')}")
if prepare.get("dataset_from_feed") is not False:
    raise SystemExit("FAIL  default prepare must not read the feed")
if prepare.get("dataset_rows") != 1:
    raise SystemExit(f"FAIL  dataset_rows={prepare.get('dataset_rows')}")
if prepare.get("dataset_read_paths") != []:
    raise SystemExit(f"FAIL  dataset_read_paths={prepare.get('dataset_read_paths')}")
artifacts = prepare.get("artifacts", [])
for name in ("recipe.yaml", "export.yaml", "dataset_info.json", "dataset.jsonl", "NEXT.md", "PREPARE.md", "prepare.json"):
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
info = json.load(open(sys.argv[3]))
card = info.get("cell_enrich")
if not isinstance(card, dict) or card.get("formatting") != "sharegpt":
    raise SystemExit(f"FAIL  dataset_info={info}")
PY

echo "-- enrich job on llamafactory-qlora refuses before write --"
set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver llamafactory-qlora \
  --job enrich \
  --out "$WORKDIR/enrich-job" \
  >/tmp/train-prepare-enrich.out 2>/tmp/train-prepare-enrich.err
enrich_rc=$?
set -e
if [[ "$enrich_rc" -eq 0 ]]; then
  echo "FAIL  --job enrich on llamafactory-qlora must refuse"
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

echo "-- gauge run writes max_steps without changing the default recipe --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver llamafactory-qlora \
  --max-steps 10 \
  --out "$WORKDIR/smoke"
grep -q '^max_steps: 10$' "$WORKDIR/smoke/recipe.yaml"
grep -q '^save_steps: 10$' "$WORKDIR/smoke/recipe.yaml"
grep -q '^num_train_epochs: 1.0$' "$WORKDIR/smoke/recipe.yaml"
grep -q "quantization_method: bnb" "$WORKDIR/smoke/recipe.yaml"
grep -q "template: qwen" "$WORKDIR/smoke/recipe.yaml"

echo "-- axolotl-lora writes the train base, not the seat tag --"
set +e
estate enrich prepare \
  --estate "$SEATED_ONLY" \
  --pack "$PACK" \
  --driver axolotl-lora \
  --out "$WORKDIR/axolotl-seat-only" \
  >/tmp/train-prepare-axolotl-seat.out 2>/tmp/train-prepare-axolotl-seat.err
ax_seat_rc=$?
set -e
if [[ "$ax_seat_rc" -eq 0 ]]; then
  echo "FAIL  axolotl-lora without a train base must refuse:train-base"
  exit 1
fi
if ! grep -q "refuse:train-base" /tmp/train-prepare-axolotl-seat.out /tmp/train-prepare-axolotl-seat.err; then
  echo "FAIL  axolotl-lora seat tag did not refuse:train-base"
  cat /tmp/train-prepare-axolotl-seat.out /tmp/train-prepare-axolotl-seat.err
  exit 1
fi
if grep -q "meta-llama" /tmp/train-prepare-axolotl-seat.out /tmp/train-prepare-axolotl-seat.err; then
  echo "FAIL  axolotl refuse must not invent a Llama-3 Hub repo"
  exit 1
fi
if [[ -e "$WORKDIR/axolotl-seat-only" ]]; then
  echo "FAIL  axolotl seat-only prepare wrote an output directory"
  exit 1
fi

estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-lora \
  --job train \
  --out "$WORKDIR/axolotl"
test -f "$WORKDIR/axolotl/axolotl.yml"
grep -q "adapter: qlora" "$WORKDIR/axolotl/axolotl.yml"
grep -q "load_in_4bit: true" "$WORKDIR/axolotl/axolotl.yml"
grep -q 'base_model: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/axolotl/axolotl.yml"
if grep -Eq '^base_model: "llama3"' "$WORKDIR/axolotl/axolotl.yml"; then
  echo "FAIL  axolotl.yml base_model must be the train base, not the seat tag"
  exit 1
fi
python3 - "$WORKDIR/axolotl/axolotl.yml" "$WORKDIR/axolotl/prepare.json" "$WORKDIR/axolotl/NEXT.md" <<'PY'
import json, sys
text = open(sys.argv[1]).read()
if "\n  - path: " not in text or "\n    ds_type: json\n    type: alpaca\n" not in text:
    raise SystemExit("FAIL  axolotl.yml datasets list is not indented")
prepare = json.load(open(sys.argv[2]))
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  axolotl seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "Qwen/Qwen2.5-0.5B-Instruct":
    raise SystemExit(f"FAIL  axolotl train_base={prepare.get('train_base_model')}")
if prepare.get("dataset_mode") != "scaffold" or prepare.get("dataset_from_feed") is not False:
    raise SystemExit(f"FAIL  axolotl dataset_mode={prepare.get('dataset_mode')} from_feed={prepare.get('dataset_from_feed')}")
next_md = open(sys.argv[3]).read()
if "Seat tag is llama3" not in next_md or "Train base is Qwen/Qwen2.5-0.5B-Instruct" not in next_md:
    raise SystemExit("FAIL  axolotl NEXT.md is missing the seat and train base split")
if "dataset_mode: scaffold" not in next_md or "refuse:dataset" not in next_md:
    raise SystemExit("FAIL  axolotl NEXT.md is missing the scaffold honesty")
if "not training data" not in next_md:
    raise SystemExit("FAIL  axolotl NEXT.md does not say the scaffold is not training data")
PY
grep -q "dataset_mode: scaffold" "$WORKDIR/axolotl/axolotl.yml"
grep -q "dataset_mode: scaffold" "$WORKDIR/axolotl/PREPARE.md"
grep -q "axolotl train $WORKDIR/axolotl/axolotl.yml" "$WORKDIR/axolotl/NEXT.md"

echo "-- missing feed with --from-feed is refuse:dataset and writes nothing --"
set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver llamafactory-qlora \
  --from-feed \
  --state-dir "$WORKDIR/no-feed" \
  --out "$WORKDIR/no-feed-out" \
  >/tmp/train-prepare-nofeed.out 2>/tmp/train-prepare-nofeed.err
nofeed_rc=$?
set -e
if [[ "$nofeed_rc" -eq 0 ]]; then
  echo "FAIL  --from-feed with a missing source must refuse"
  exit 1
fi
if ! grep -q "refuse:dataset" /tmp/train-prepare-nofeed.out /tmp/train-prepare-nofeed.err; then
  echo "FAIL  missing feed did not refuse:dataset"
  cat /tmp/train-prepare-nofeed.out /tmp/train-prepare-nofeed.err
  exit 1
fi
if ! grep -q "feed/events.jsonl" /tmp/train-prepare-nofeed.out /tmp/train-prepare-nofeed.err; then
  echo "FAIL  missing feed refuse did not name feed/events.jsonl"
  cat /tmp/train-prepare-nofeed.out /tmp/train-prepare-nofeed.err
  exit 1
fi
if [[ -e "$WORKDIR/no-feed-out" ]]; then
  echo "FAIL  missing feed prepare wrote an output directory"
  exit 1
fi

echo "-- fixture feed rows hydrate; the same files without --from-feed stay a scaffold --"
HYDRATE="$WORKDIR/hydrate-cell"
mkdir -p "$HYDRATE/feed"
python3 - "$HYDRATE/feed/events.jsonl" <<'PY'
import json, sys
rows = [
    {
        "kind": "model.local.precheck",
        "agent_id": "research",
        "decision": "allow",
        "object_class": "local",
        "note": "job=policy-precheck",
        "ts": "2026-09-21T00:00:00Z",
    },
    {
        "kind": "model.local.skip",
        "agent_id": "research",
        "decision": "allow",
        "object_class": "local",
        "note": None,
        "ts": "2026-09-21T00:00:01Z",
    },
]
with open(sys.argv[1], "w", encoding="utf-8") as handle:
    for row in rows:
        handle.write(json.dumps(row) + "\n")
PY
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver llamafactory-qlora \
  --state-dir "$HYDRATE" \
  --out "$WORKDIR/scaffold-despite-feed"
if grep -q "job=policy-precheck" "$WORKDIR/scaffold-despite-feed/dataset.jsonl"; then
  echo "FAIL  default prepare copied a feed note"
  exit 1
fi
grep -q "Replace this scaffold" "$WORKDIR/scaffold-despite-feed/dataset.jsonl"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver llamafactory-qlora \
  --from-feed \
  --state-dir "$HYDRATE" \
  --out "$WORKDIR/hydrated-lf"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-lora \
  --from-feed \
  --state-dir "$HYDRATE" \
  --out "$WORKDIR/hydrated-ax"
python3 - "$WORKDIR/hydrated-lf" "$WORKDIR/hydrated-ax" <<'PY'
import json, sys
from pathlib import Path
for directory, shape in ((sys.argv[1], "messages"), (sys.argv[2], "instruction")):
    root = Path(directory)
    prepare = json.loads((root / "prepare.json").read_text())
    if prepare.get("dataset_mode") != "feed" or prepare.get("dataset_from_feed") is not True:
        raise SystemExit(f"FAIL  {directory} mode={prepare.get('dataset_mode')}")
    if prepare.get("dataset_rows") != 1 or prepare.get("dataset_skipped") != 1:
        raise SystemExit(f"FAIL  {directory} rows={prepare.get('dataset_rows')} skipped={prepare.get('dataset_skipped')}")
    if prepare.get("dataset_read_paths") != ["feed/events.jsonl"]:
        raise SystemExit(f"FAIL  {directory} paths={prepare.get('dataset_read_paths')}")
    jsonl = (root / "dataset.jsonl").read_text()
    if "job=policy-precheck" not in jsonl or "model.local.skip" in jsonl:
        raise SystemExit(f"FAIL  {directory} jsonl did not hydrate the noted event")
    if "Replace this scaffold" in jsonl:
        raise SystemExit(f"FAIL  {directory} jsonl is still a scaffold")
    if shape not in jsonl:
        raise SystemExit(f"FAIL  {directory} jsonl missing {shape}")
    for name in ("PREPARE.md", "NEXT.md"):
        text = (root / name).read_text()
        if "dataset_mode: feed" not in text or "refuse:dataset" not in text:
            raise SystemExit(f"FAIL  {directory} {name} missing feed honesty")
PY

echo "-- all-drivers train writes the train base into axolotl.yml --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --all-drivers \
  --job train \
  --state-dir "$WORKDIR/all-state"
grep -q 'base_model: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/all-state/enrich/overnight-traces/axolotl-lora/axolotl.yml"
if grep -Eq '^base_model: "llama3"' "$WORKDIR/all-state/enrich/overnight-traces/axolotl-lora/axolotl.yml"; then
  echo "FAIL  all-drivers axolotl.yml still points base_model at the seat tag"
  exit 1
fi
grep -q "FROM llama3" "$WORKDIR/all-state/enrich/overnight-traces/ollama-modelfile/Modelfile"

echo "-- import-trained records the LLaMA-Factory adapter on local_slm --"
ADAPTER="$WORKDIR/adapter"
mkdir -p "$ADAPTER"
printf '{}\n' > "$ADAPTER/adapter_config.json"
estate enrich import-trained \
  --estate "$SEATED" \
  --prepared "$WORKDIR/llamafactory" \
  --tag cell-enrich-overnight-traces \
  --adapter "$ADAPTER" \
  | tee "$WORKDIR/import.out"
test -f "$WORKDIR/llamafactory/binding-proposal.json"
grep -q "import-trained did not apply" "$WORKDIR/import.out"
grep -q "auto_apply=false" "$WORKDIR/import.out"
python3 - "$WORKDIR/llamafactory/binding-proposal.json" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
if doc.get("schema") != "cell-one.enrich-binding-proposal.v0":
    raise SystemExit(f"FAIL  schema={doc.get('schema')}")
if doc.get("driver") != "llamafactory-qlora" or doc.get("job") != "train":
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

echo "PASS  train-prepare (LLaMA-Factory recipe, Axolotl recipe, feed hydrate, import-trained; SKIP live train)"
