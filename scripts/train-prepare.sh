#!/usr/bin/env bash
# Train prepare fixture: example pack -> LLaMA-Factory LoRA and QLoRA, Axolotl LoRA and QLoRA, the optional Unsloth handoff, and the optional mlx-lm handoff.
# Throwaway dir. No LLaMA-Factory install. No Axolotl binary. No Unsloth install. No mlx-lm install. No GPU. No live train.
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

echo "== train-prepare (LLaMA-Factory LoRA and QLoRA, Axolotl LoRA and QLoRA; not a live train) =="
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
if grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x LoRA/QLoRA." "$WORKDIR/llamafactory/NEXT.md" "$WORKDIR/llamafactory/PREPARE.md"; then
  echo "FAIL  qwen2.5 prepare took the qwen3 instruct reproduce note"
  exit 1
fi
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct QLoRA." "$WORKDIR/llamafactory/NEXT.md"
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct QLoRA." "$WORKDIR/llamafactory/PREPARE.md"
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
grep -q "estate enrich merge-adapt --prepared $WORKDIR/llamafactory --adapter $WORKDIR/llamafactory/outputs" "$WORKDIR/llamafactory/NEXT.md"
grep -q "examples/merge_lora/qwen3_lora_sft.yaml" "$WORKDIR/llamafactory/NEXT.md"
grep -q "estate enrich merge-adapt --prepared $WORKDIR/llamafactory --adapter $WORKDIR/llamafactory/outputs" "$WORKDIR/llamafactory/PREPARE.md"
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
grep -q "estate enrich gguf-convert --prepared $WORKDIR/llamafactory --weights $WORKDIR/llamafactory/export" "$WORKDIR/llamafactory/NEXT.md"
grep -q "python3 convert_hf_to_gguf.py $WORKDIR/llamafactory/export --outfile $WORKDIR/llamafactory/export.gguf --outtype auto" "$WORKDIR/llamafactory/NEXT.md"
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

echo "-- llamafactory-lora default job is unquantized LoRA --"
set +e
estate enrich prepare \
  --estate "$SEATED_ONLY" \
  --pack "$PACK" \
  --driver llamafactory-lora \
  --out "$WORKDIR/lora-seat-only" \
  >/tmp/train-prepare-lora-seat.out 2>/tmp/train-prepare-lora-seat.err
lora_seat_rc=$?
set -e
if [[ "$lora_seat_rc" -eq 0 ]]; then
  echo "FAIL  llamafactory-lora without a train base must refuse:train-base"
  exit 1
fi
if ! grep -q "refuse:train-base" /tmp/train-prepare-lora-seat.out /tmp/train-prepare-lora-seat.err; then
  echo "FAIL  llamafactory-lora seat tag did not refuse:train-base"
  cat /tmp/train-prepare-lora-seat.out /tmp/train-prepare-lora-seat.err
  exit 1
fi
if [[ -e "$WORKDIR/lora-seat-only" ]]; then
  echo "FAIL  llamafactory-lora seat-only prepare wrote an output directory"
  exit 1
fi

estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver llamafactory-lora \
  --out "$WORKDIR/llamafactory-lora"
test -f "$WORKDIR/llamafactory-lora/recipe.yaml"
test -f "$WORKDIR/llamafactory-lora/export.yaml"
grep -q "finetuning_type: lora" "$WORKDIR/llamafactory-lora/recipe.yaml"
grep -q "^lora_rank: 8$" "$WORKDIR/llamafactory-lora/recipe.yaml"
grep -q "^packing: false$" "$WORKDIR/llamafactory-lora/recipe.yaml"
grep -q "^template: qwen$" "$WORKDIR/llamafactory-lora/recipe.yaml"
grep -q 'model_name_or_path: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/llamafactory-lora/recipe.yaml"
if grep -q "quantization_bit" "$WORKDIR/llamafactory-lora/recipe.yaml"; then
  echo "FAIL  llamafactory-lora recipe.yaml must omit quantization_bit"
  exit 1
fi
if grep -q "quantization_method" "$WORKDIR/llamafactory-lora/recipe.yaml"; then
  echo "FAIL  llamafactory-lora recipe.yaml must omit quantization_method"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/llamafactory-lora/export.yaml"; then
  echo "FAIL  llamafactory-lora export.yaml must omit quantization_bit"
  exit 1
fi
grep -q "does not require bitsandbytes" "$WORKDIR/llamafactory-lora/NEXT.md"
if grep -q "bitsandbytes>=0.49" "$WORKDIR/llamafactory-lora/NEXT.md"; then
  echo "FAIL  llamafactory-lora NEXT.md must not install bitsandbytes"
  exit 1
fi
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen3 Instruct QLoRA prepare." "$WORKDIR/llamafactory-lora/NEXT.md" "$WORKDIR/llamafactory-lora/PREPARE.md"; then
  echo "FAIL  qwen2.5 LoRA prepare took the qwen3 instruct LoRA reproduce note"
  exit 1
fi
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the DeepSeek-R1-Distill chat QLoRA prepare." "$WORKDIR/llamafactory-lora/NEXT.md" "$WORKDIR/llamafactory-lora/PREPARE.md"; then
  echo "FAIL  qwen2.5 LoRA prepare took the DeepSeek-R1-Distill LoRA reproduce note"
  exit 1
fi
if grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct QLoRA." "$WORKDIR/llamafactory-lora/NEXT.md" "$WORKDIR/llamafactory-lora/PREPARE.md"; then
  echo "FAIL  qwen2.5 LoRA prepare took the Qwen2.5 Instruct QLoRA reproduce note"
  exit 1
fi
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen2.5 Instruct QLoRA prepare." "$WORKDIR/llamafactory-lora/NEXT.md"
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen2.5 Instruct QLoRA prepare." "$WORKDIR/llamafactory-lora/PREPARE.md"
grep -q "pip install llamafactory" "$WORKDIR/llamafactory-lora/NEXT.md"
grep -q "examples/train_lora/qwen3_lora_sft.yaml" "$WORKDIR/llamafactory-lora/NEXT.md"
grep -q "^cutoff_len: 512$" "$WORKDIR/llamafactory-lora/recipe.yaml"
grep -q "^num_train_epochs: 1.0$" "$WORKDIR/llamafactory-lora/recipe.yaml"
grep -q "^gradient_accumulation_steps: 4$" "$WORKDIR/llamafactory-lora/recipe.yaml"
grep -q "^warmup_ratio: 0.03$" "$WORKDIR/llamafactory-lora/recipe.yaml"
grep -q "# merge_status: not_run" "$WORKDIR/llamafactory-lora/export.yaml"
grep -q "This prepare did not merge" "$WORKDIR/llamafactory-lora/export.yaml"
grep -q "This prepare did not merge" "$WORKDIR/llamafactory-lora/NEXT.md"
grep -q "The merge has not happened." "$WORKDIR/llamafactory-lora/NEXT.md"
grep -q "Do not set quantization_bit on export.yaml" "$WORKDIR/llamafactory-lora/NEXT.md"
grep -q "llamafactory-cli export has not run" "$WORKDIR/llamafactory-lora/PREPARE.md"
grep -q "llamafactory-cli train $WORKDIR/llamafactory-lora/recipe.yaml" "$WORKDIR/llamafactory-lora/NEXT.md"
grep -q "llamafactory-cli export $WORKDIR/llamafactory-lora/export.yaml" "$WORKDIR/llamafactory-lora/NEXT.md"
grep -q "estate enrich merge-adapt --prepared $WORKDIR/llamafactory-lora --adapter $WORKDIR/llamafactory-lora/outputs" "$WORKDIR/llamafactory-lora/NEXT.md"
grep -q "examples/merge_lora/qwen3_lora_sft.yaml" "$WORKDIR/llamafactory-lora/NEXT.md"
grep -q "estate enrich merge-adapt --prepared $WORKDIR/llamafactory-lora --adapter $WORKDIR/llamafactory-lora/outputs" "$WORKDIR/llamafactory-lora/PREPARE.md"
grep -q "estate enrich gguf-convert --prepared $WORKDIR/llamafactory-lora --weights $WORKDIR/llamafactory-lora/export" "$WORKDIR/llamafactory-lora/NEXT.md"
grep -q "python3 convert_hf_to_gguf.py $WORKDIR/llamafactory-lora/export --outfile $WORKDIR/llamafactory-lora/export.gguf --outtype auto" "$WORKDIR/llamafactory-lora/NEXT.md"
python3 - "$WORKDIR/llamafactory-lora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("driver") != "llamafactory-lora":
    raise SystemExit(f"FAIL  driver={prepare.get('driver')}")
if prepare.get("job") != "train":
    raise SystemExit(f"FAIL  job={prepare.get('job')}")
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "Qwen/Qwen2.5-0.5B-Instruct":
    raise SystemExit(f"FAIL  train_base_model={prepare.get('train_base_model')}")
PY

QWEN3="$WORKDIR/estate-qwen3.yaml"
python3 - "$ESTATE" "$QWEN3" <<'PY'
import sys
src, seated = sys.argv[1:]
text = open(src).read()
needle = "  - id: local_slm\n    class: local\n    driver: ollama\n    params:\n"
if needle not in text:
    raise SystemExit("FAIL  local_slm params block missing")
open(seated, "w").write(text.replace(
    needle,
    needle + '      model: "llama3"\n      train_base_model: "Qwen/Qwen3-4B-Instruct-2507"\n',
    1,
))
PY
estate enrich prepare \
  --estate "$QWEN3" \
  --pack "$PACK" \
  --driver llamafactory-lora \
  --out "$WORKDIR/llamafactory-lora-qwen3"
grep -q "^template: qwen3_nothink$" "$WORKDIR/llamafactory-lora-qwen3/recipe.yaml"
grep -q "^template: qwen3_nothink$" "$WORKDIR/llamafactory-lora-qwen3/export.yaml"
if grep -q "quantization_bit" "$WORKDIR/llamafactory-lora-qwen3/recipe.yaml"; then
  echo "FAIL  qwen3 LoRA recipe must omit quantization_bit"
  exit 1
fi
if grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x LoRA/QLoRA." "$WORKDIR/llamafactory-lora-qwen3/NEXT.md" "$WORKDIR/llamafactory-lora-qwen3/PREPARE.md"; then
  echo "FAIL  qwen3 LoRA prepare must not write the QLoRA reproduce note"
  exit 1
fi
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen3 Instruct QLoRA prepare." "$WORKDIR/llamafactory-lora-qwen3/NEXT.md"
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen3 Instruct QLoRA prepare." "$WORKDIR/llamafactory-lora-qwen3/PREPARE.md"
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen2.5 Instruct QLoRA prepare." "$WORKDIR/llamafactory-lora-qwen3/NEXT.md" "$WORKDIR/llamafactory-lora-qwen3/PREPARE.md"; then
  echo "FAIL  qwen3 LoRA prepare took the Qwen2.5 Instruct LoRA reproduce note"
  exit 1
fi
if grep -q "quantization_method" "$WORKDIR/llamafactory-lora-qwen3/recipe.yaml"; then
  echo "FAIL  qwen3 LoRA recipe must omit quantization_method"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/llamafactory-lora-qwen3/NEXT.md" "$WORKDIR/llamafactory-lora-qwen3/PREPARE.md"; then
  echo "FAIL  qwen3 LoRA prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
estate enrich prepare \
  --estate "$QWEN3" \
  --pack "$PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/llamafactory-qlora-qwen3"
grep -q "^template: qwen3_nothink$" "$WORKDIR/llamafactory-qlora-qwen3/recipe.yaml"
grep -q "quantization_bit: 4" "$WORKDIR/llamafactory-qlora-qwen3/recipe.yaml"
grep -q "quantization_method: bnb" "$WORKDIR/llamafactory-qlora-qwen3/recipe.yaml"
grep -q "bitsandbytes>=0.49" "$WORKDIR/llamafactory-qlora-qwen3/NEXT.md"
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x LoRA/QLoRA." "$WORKDIR/llamafactory-qlora-qwen3/NEXT.md"
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x LoRA/QLoRA." "$WORKDIR/llamafactory-qlora-qwen3/PREPARE.md"
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen3 Instruct QLoRA prepare." "$WORKDIR/llamafactory-qlora-qwen3/NEXT.md" "$WORKDIR/llamafactory-qlora-qwen3/PREPARE.md"; then
  echo "FAIL  qwen3 QLoRA prepare must not write the LoRA reproduce note"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/llamafactory-qlora-qwen3/NEXT.md" "$WORKDIR/llamafactory-qlora-qwen3/PREPARE.md"; then
  echo "FAIL  qwen3 qlora prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi

PHI_PACK="$ROOT/examples/fixtures/phi3-instruct.pack.json"
echo "-- phi3 instruct qlora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PHI_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/phi3-qlora"
grep -q "^template: phi$" "$WORKDIR/phi3-qlora/recipe.yaml"
grep -q "^template: phi$" "$WORKDIR/phi3-qlora/export.yaml"
grep -q "quantization_bit: 4" "$WORKDIR/phi3-qlora/recipe.yaml"
grep -q "quantization_method: bnb" "$WORKDIR/phi3-qlora/recipe.yaml"
grep -q 'model_name_or_path: "microsoft/Phi-3-mini-4k-instruct"' "$WORKDIR/phi3-qlora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/phi3-qlora/recipe.yaml"; then
  echo "FAIL  phi3 recipe named the seat tag as model_name_or_path"
  exit 1
fi
grep -q "Reproduce target beside Qwen LoRA/QLoRA." "$WORKDIR/phi3-qlora/NEXT.md"
grep -q "Reproduce target beside Qwen LoRA/QLoRA." "$WORKDIR/phi3-qlora/PREPARE.md"
python3 - "$WORKDIR/phi3-qlora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  phi seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "microsoft/Phi-3-mini-4k-instruct":
    raise SystemExit(f"FAIL  phi train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False:
    raise SystemExit("FAIL  phi prepare must stay unpromoted")
PY

PHI_LORA_PACK="$ROOT/examples/fixtures/phi3-instruct-lora.pack.json"
echo "-- phi3 instruct lora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PHI_LORA_PACK" \
  --driver llamafactory-lora \
  --out "$WORKDIR/phi3-lora"
grep -q "^template: phi$" "$WORKDIR/phi3-lora/recipe.yaml"
grep -q "^template: phi$" "$WORKDIR/phi3-lora/export.yaml"
grep -q "^lora_rank: 8$" "$WORKDIR/phi3-lora/recipe.yaml"
grep -q "^packing: false$" "$WORKDIR/phi3-lora/recipe.yaml"
if grep -q "^template: phi_small$" "$WORKDIR/phi3-lora/recipe.yaml"; then
  echo "FAIL  phi3 instruct LoRA recipe used phi_small"
  exit 1
fi
if grep -q "^template: phi4" "$WORKDIR/phi3-lora/recipe.yaml"; then
  echo "FAIL  phi3 instruct LoRA recipe used a phi4 template"
  exit 1
fi
grep -q 'model_name_or_path: "microsoft/Phi-3-mini-4k-instruct"' "$WORKDIR/phi3-lora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/phi3-lora/recipe.yaml"; then
  echo "FAIL  phi3 LoRA recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/phi3-lora/recipe.yaml" "$WORKDIR/phi3-lora/export.yaml"; then
  echo "FAIL  phi3 LoRA recipe must omit quantization_bit"
  exit 1
fi
if grep -q "quantization_method" "$WORKDIR/phi3-lora/recipe.yaml" "$WORKDIR/phi3-lora/export.yaml"; then
  echo "FAIL  phi3 LoRA recipe must omit quantization_method"
  exit 1
fi
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Phi-3 Instruct QLoRA prepare." "$WORKDIR/phi3-lora/NEXT.md"
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Phi-3 Instruct QLoRA prepare." "$WORKDIR/phi3-lora/PREPARE.md"
if grep -q "Reproduce target beside Qwen LoRA/QLoRA." "$WORKDIR/phi3-lora/NEXT.md" "$WORKDIR/phi3-lora/PREPARE.md"; then
  echo "FAIL  phi3 LoRA fixture prepare wrote the QLoRA reproduce note"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/phi3-lora/NEXT.md" "$WORKDIR/phi3-lora/PREPARE.md"; then
  echo "FAIL  phi3 LoRA prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
if [[ -e "$WORKDIR/phi3-lora/train.py" || -e "$WORKDIR/phi3-lora/train.sh" ]]; then
  echo "FAIL  phi3 LoRA prepare must not write a train script"
  exit 1
fi
python3 - "$WORKDIR/phi3-lora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("driver") != "llamafactory-lora":
    raise SystemExit(f"FAIL  phi3 lora driver={prepare.get('driver')}")
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  phi3 lora seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "microsoft/Phi-3-mini-4k-instruct":
    raise SystemExit(f"FAIL  phi3 lora train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  phi3 lora prepare must stay unpromoted")
PY
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PHI_LORA_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/phi3-lora-pack-qlora"
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Phi-3 Instruct QLoRA prepare." "$WORKDIR/phi3-lora-pack-qlora/NEXT.md" "$WORKDIR/phi3-lora-pack-qlora/PREPARE.md"; then
  echo "FAIL  phi3 LoRA fixture on the QLoRA card wrote the LoRA reproduce note"
  exit 1
fi
grep -q "Reproduce target beside Qwen LoRA/QLoRA." "$WORKDIR/phi3-lora-pack-qlora/NEXT.md"
grep -q "quantization_method: bnb" "$WORKDIR/phi3-lora-pack-qlora/recipe.yaml"
grep -q "quantization_bit: 4" "$WORKDIR/phi3-lora-pack-qlora/recipe.yaml"
grep -q "^template: phi$" "$WORKDIR/phi3-lora-pack-qlora/recipe.yaml"

LLAMA32_PACK="$ROOT/examples/fixtures/llama32-instruct.pack.json"
echo "-- llama 3.2 instruct qlora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$LLAMA32_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/llama32-qlora"
grep -q "^template: llama3$" "$WORKDIR/llama32-qlora/recipe.yaml"
grep -q "^template: llama3$" "$WORKDIR/llama32-qlora/export.yaml"
if grep -q "^template: llama3_" "$WORKDIR/llama32-qlora/recipe.yaml"; then
  echo "FAIL  llama32 recipe used a llama3_ template name"
  exit 1
fi
if grep -q "^template: mllama$" "$WORKDIR/llama32-qlora/recipe.yaml"; then
  echo "FAIL  llama32 instruct recipe used the vision template"
  exit 1
fi
grep -q "quantization_bit: 4" "$WORKDIR/llama32-qlora/recipe.yaml"
grep -q "quantization_method: bnb" "$WORKDIR/llama32-qlora/recipe.yaml"
grep -q 'model_name_or_path: "meta-llama/Llama-3.2-3B-Instruct"' "$WORKDIR/llama32-qlora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/llama32-qlora/recipe.yaml"; then
  echo "FAIL  llama32 recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/llama32-qlora/export.yaml"; then
  echo "FAIL  llama32 export.yaml must not set quantization_bit"
  exit 1
fi
grep -q "Reproduce target beside Phi-3 and Qwen LoRA/QLoRA." "$WORKDIR/llama32-qlora/NEXT.md"
grep -q "Reproduce target beside Phi-3 and Qwen LoRA/QLoRA." "$WORKDIR/llama32-qlora/PREPARE.md"
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Llama-3.2 Instruct QLoRA prepare." "$WORKDIR/llama32-qlora/NEXT.md" "$WORKDIR/llama32-qlora/PREPARE.md"; then
  echo "FAIL  llama32 QLoRA prepare must not write the LoRA reproduce note"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/llama32-qlora/NEXT.md" "$WORKDIR/llama32-qlora/PREPARE.md"; then
  echo "FAIL  llama32 prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
python3 - "$WORKDIR/llama32-qlora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  llama32 seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "meta-llama/Llama-3.2-3B-Instruct":
    raise SystemExit(f"FAIL  llama32 train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  llama32 prepare must stay unpromoted")
PY

GEMMA2_PACK="$ROOT/examples/fixtures/gemma2-instruct.pack.json"
echo "-- gemma 2 instruct qlora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$GEMMA2_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/gemma2-qlora"
grep -q "^template: gemma2$" "$WORKDIR/gemma2-qlora/recipe.yaml"
grep -q "^template: gemma2$" "$WORKDIR/gemma2-qlora/export.yaml"
if grep -q "^template: gemma$" "$WORKDIR/gemma2-qlora/recipe.yaml"; then
  echo "FAIL  gemma2 instruct recipe used the original gemma template"
  exit 1
fi
if grep -q "^template: gemma_" "$WORKDIR/gemma2-qlora/recipe.yaml"; then
  echo "FAIL  gemma2 recipe used a gemma_ template name"
  exit 1
fi
grep -q "quantization_bit: 4" "$WORKDIR/gemma2-qlora/recipe.yaml"
grep -q "quantization_method: bnb" "$WORKDIR/gemma2-qlora/recipe.yaml"
grep -q 'model_name_or_path: "google/gemma-2-2b-it"' "$WORKDIR/gemma2-qlora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/gemma2-qlora/recipe.yaml"; then
  echo "FAIL  gemma2 recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/gemma2-qlora/export.yaml"; then
  echo "FAIL  gemma2 export.yaml must not set quantization_bit"
  exit 1
fi
grep -q "Reproduce target beside Phi-3, Llama-3.2, and Qwen LoRA/QLoRA." "$WORKDIR/gemma2-qlora/NEXT.md"
grep -q "Reproduce target beside Phi-3, Llama-3.2, and Qwen LoRA/QLoRA." "$WORKDIR/gemma2-qlora/PREPARE.md"
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/gemma2-qlora/NEXT.md" "$WORKDIR/gemma2-qlora/PREPARE.md"; then
  echo "FAIL  gemma2 prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
python3 - "$WORKDIR/gemma2-qlora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  gemma2 seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "google/gemma-2-2b-it":
    raise SystemExit(f"FAIL  gemma2 train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  gemma2 prepare must stay unpromoted")
PY

GEMMA2_LORA_PACK="$ROOT/examples/fixtures/gemma2-instruct-lora.pack.json"
echo "-- gemma 2 instruct lora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$GEMMA2_LORA_PACK" \
  --driver llamafactory-lora \
  --out "$WORKDIR/gemma2-lora"
grep -q "^template: gemma2$" "$WORKDIR/gemma2-lora/recipe.yaml"
grep -q "^template: gemma2$" "$WORKDIR/gemma2-lora/export.yaml"
grep -q "^lora_rank: 8$" "$WORKDIR/gemma2-lora/recipe.yaml"
grep -q "^packing: false$" "$WORKDIR/gemma2-lora/recipe.yaml"
if grep -q "^template: gemma$" "$WORKDIR/gemma2-lora/recipe.yaml"; then
  echo "FAIL  gemma2 instruct LoRA recipe used the original gemma template"
  exit 1
fi
if grep -q "^template: gemma_" "$WORKDIR/gemma2-lora/recipe.yaml"; then
  echo "FAIL  gemma2 LoRA recipe used a gemma_ template name"
  exit 1
fi
grep -q 'model_name_or_path: "google/gemma-2-2b-it"' "$WORKDIR/gemma2-lora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/gemma2-lora/recipe.yaml"; then
  echo "FAIL  gemma2 LoRA recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/gemma2-lora/recipe.yaml" "$WORKDIR/gemma2-lora/export.yaml"; then
  echo "FAIL  gemma2 LoRA recipe must omit quantization_bit"
  exit 1
fi
if grep -q "quantization_method" "$WORKDIR/gemma2-lora/recipe.yaml" "$WORKDIR/gemma2-lora/export.yaml"; then
  echo "FAIL  gemma2 LoRA recipe must omit quantization_method"
  exit 1
fi
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Gemma-2 Instruct QLoRA prepare." "$WORKDIR/gemma2-lora/NEXT.md"
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Gemma-2 Instruct QLoRA prepare." "$WORKDIR/gemma2-lora/PREPARE.md"
if grep -q "Reproduce target beside Phi-3, Llama-3.2, and Qwen LoRA/QLoRA." "$WORKDIR/gemma2-lora/NEXT.md" "$WORKDIR/gemma2-lora/PREPARE.md"; then
  echo "FAIL  gemma2 LoRA fixture prepare wrote the QLoRA reproduce note"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/gemma2-lora/NEXT.md" "$WORKDIR/gemma2-lora/PREPARE.md"; then
  echo "FAIL  gemma2 LoRA prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
if [[ -e "$WORKDIR/gemma2-lora/train.py" || -e "$WORKDIR/gemma2-lora/train.sh" ]]; then
  echo "FAIL  gemma2 LoRA prepare must not write a train script"
  exit 1
fi
python3 - "$WORKDIR/gemma2-lora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("driver") != "llamafactory-lora":
    raise SystemExit(f"FAIL  gemma2 lora driver={prepare.get('driver')}")
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  gemma2 lora seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "google/gemma-2-2b-it":
    raise SystemExit(f"FAIL  gemma2 lora train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  gemma2 lora prepare must stay unpromoted")
PY
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$GEMMA2_LORA_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/gemma2-lora-pack-qlora"
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Gemma-2 Instruct QLoRA prepare." "$WORKDIR/gemma2-lora-pack-qlora/NEXT.md" "$WORKDIR/gemma2-lora-pack-qlora/PREPARE.md"; then
  echo "FAIL  gemma2 LoRA fixture on the QLoRA card wrote the LoRA reproduce note"
  exit 1
fi
grep -q "quantization_method: bnb" "$WORKDIR/gemma2-lora-pack-qlora/recipe.yaml"
grep -q "quantization_bit: 4" "$WORKDIR/gemma2-lora-pack-qlora/recipe.yaml"

MISTRAL_PACK="$ROOT/examples/fixtures/mistral-instruct.pack.json"
echo "-- mistral instruct qlora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$MISTRAL_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/mistral-qlora"
grep -q "^template: mistral$" "$WORKDIR/mistral-qlora/recipe.yaml"
grep -q "^template: mistral$" "$WORKDIR/mistral-qlora/export.yaml"
if grep -q "^template: mistral_" "$WORKDIR/mistral-qlora/recipe.yaml"; then
  echo "FAIL  mistral instruct recipe used a mistral_ template name"
  exit 1
fi
if grep -q "^template: ministral" "$WORKDIR/mistral-qlora/recipe.yaml"; then
  echo "FAIL  mistral instruct recipe used the ministral template"
  exit 1
fi
grep -q "quantization_bit: 4" "$WORKDIR/mistral-qlora/recipe.yaml"
grep -q "quantization_method: bnb" "$WORKDIR/mistral-qlora/recipe.yaml"
grep -q 'model_name_or_path: "mistralai/Mistral-7B-Instruct-v0.3"' "$WORKDIR/mistral-qlora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/mistral-qlora/recipe.yaml"; then
  echo "FAIL  mistral recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/mistral-qlora/export.yaml"; then
  echo "FAIL  mistral export.yaml must not set quantization_bit"
  exit 1
fi
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, and Qwen LoRA/QLoRA." "$WORKDIR/mistral-qlora/NEXT.md"
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, and Qwen LoRA/QLoRA." "$WORKDIR/mistral-qlora/PREPARE.md"
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/mistral-qlora/NEXT.md" "$WORKDIR/mistral-qlora/PREPARE.md"; then
  echo "FAIL  mistral prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
python3 - "$WORKDIR/mistral-qlora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  mistral seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "mistralai/Mistral-7B-Instruct-v0.3":
    raise SystemExit(f"FAIL  mistral train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  mistral prepare must stay unpromoted")
PY

MISTRAL_LORA_PACK="$ROOT/examples/fixtures/mistral-instruct-lora.pack.json"
echo "-- mistral instruct lora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$MISTRAL_LORA_PACK" \
  --driver llamafactory-lora \
  --out "$WORKDIR/mistral-lora"
grep -q "^template: mistral$" "$WORKDIR/mistral-lora/recipe.yaml"
grep -q "^template: mistral$" "$WORKDIR/mistral-lora/export.yaml"
grep -q "^lora_rank: 8$" "$WORKDIR/mistral-lora/recipe.yaml"
grep -q "^packing: false$" "$WORKDIR/mistral-lora/recipe.yaml"
if grep -q "^template: mistral_" "$WORKDIR/mistral-lora/recipe.yaml"; then
  echo "FAIL  mistral instruct LoRA recipe used a mistral_ template name"
  exit 1
fi
if grep -q "^template: ministral" "$WORKDIR/mistral-lora/recipe.yaml"; then
  echo "FAIL  mistral instruct LoRA recipe used the ministral template"
  exit 1
fi
grep -q 'model_name_or_path: "mistralai/Mistral-7B-Instruct-v0.3"' "$WORKDIR/mistral-lora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/mistral-lora/recipe.yaml"; then
  echo "FAIL  mistral LoRA recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/mistral-lora/recipe.yaml" "$WORKDIR/mistral-lora/export.yaml"; then
  echo "FAIL  mistral LoRA recipe must omit quantization_bit"
  exit 1
fi
if grep -q "quantization_method" "$WORKDIR/mistral-lora/recipe.yaml" "$WORKDIR/mistral-lora/export.yaml"; then
  echo "FAIL  mistral LoRA recipe must omit quantization_method"
  exit 1
fi
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Mistral Instruct QLoRA prepare." "$WORKDIR/mistral-lora/NEXT.md"
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Mistral Instruct QLoRA prepare." "$WORKDIR/mistral-lora/PREPARE.md"
if grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, and Qwen LoRA/QLoRA." "$WORKDIR/mistral-lora/NEXT.md" "$WORKDIR/mistral-lora/PREPARE.md"; then
  echo "FAIL  mistral LoRA fixture prepare wrote the QLoRA reproduce note"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/mistral-lora/NEXT.md" "$WORKDIR/mistral-lora/PREPARE.md"; then
  echo "FAIL  mistral LoRA prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
if [[ -e "$WORKDIR/mistral-lora/train.py" || -e "$WORKDIR/mistral-lora/train.sh" ]]; then
  echo "FAIL  mistral LoRA prepare must not write a train script"
  exit 1
fi
python3 - "$WORKDIR/mistral-lora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("driver") != "llamafactory-lora":
    raise SystemExit(f"FAIL  mistral lora driver={prepare.get('driver')}")
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  mistral lora seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "mistralai/Mistral-7B-Instruct-v0.3":
    raise SystemExit(f"FAIL  mistral lora train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  mistral lora prepare must stay unpromoted")
PY
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$MISTRAL_LORA_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/mistral-lora-pack-qlora"
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Mistral Instruct QLoRA prepare." "$WORKDIR/mistral-lora-pack-qlora/NEXT.md" "$WORKDIR/mistral-lora-pack-qlora/PREPARE.md"; then
  echo "FAIL  mistral LoRA fixture on the QLoRA card wrote the LoRA reproduce note"
  exit 1
fi
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, and Qwen LoRA/QLoRA." "$WORKDIR/mistral-lora-pack-qlora/NEXT.md"
grep -q "quantization_method: bnb" "$WORKDIR/mistral-lora-pack-qlora/recipe.yaml"
grep -q "quantization_bit: 4" "$WORKDIR/mistral-lora-pack-qlora/recipe.yaml"
grep -q "^template: mistral$" "$WORKDIR/mistral-lora-pack-qlora/recipe.yaml"

QWEN3_PACK="$ROOT/examples/fixtures/qwen3-instruct.pack.json"
echo "-- qwen3 instruct qlora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$QWEN3_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/qwen3-qlora"
grep -q "^template: qwen3_nothink$" "$WORKDIR/qwen3-qlora/recipe.yaml"
grep -q "^template: qwen3_nothink$" "$WORKDIR/qwen3-qlora/export.yaml"
if grep -q "^template: qwen3$" "$WORKDIR/qwen3-qlora/recipe.yaml"; then
  echo "FAIL  qwen3 instruct recipe used the thinking template"
  exit 1
fi
if grep -q "^template: qwen$" "$WORKDIR/qwen3-qlora/recipe.yaml"; then
  echo "FAIL  qwen3 instruct recipe used the qwen2 template"
  exit 1
fi
grep -q "quantization_bit: 4" "$WORKDIR/qwen3-qlora/recipe.yaml"
grep -q "quantization_method: bnb" "$WORKDIR/qwen3-qlora/recipe.yaml"
grep -q 'model_name_or_path: "Qwen/Qwen3-4B-Instruct-2507"' "$WORKDIR/qwen3-qlora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/qwen3-qlora/recipe.yaml"; then
  echo "FAIL  qwen3 recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/qwen3-qlora/export.yaml"; then
  echo "FAIL  qwen3 export.yaml must not set quantization_bit"
  exit 1
fi
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x LoRA/QLoRA." "$WORKDIR/qwen3-qlora/NEXT.md"
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x LoRA/QLoRA." "$WORKDIR/qwen3-qlora/PREPARE.md"
if grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct QLoRA." "$WORKDIR/qwen3-qlora/NEXT.md" "$WORKDIR/qwen3-qlora/PREPARE.md"; then
  echo "FAIL  qwen3 prepare took the qwen2.5 instruct reproduce note"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/qwen3-qlora/NEXT.md" "$WORKDIR/qwen3-qlora/PREPARE.md"; then
  echo "FAIL  qwen3 prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
python3 - "$WORKDIR/qwen3-qlora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  qwen3 seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "Qwen/Qwen3-4B-Instruct-2507":
    raise SystemExit(f"FAIL  qwen3 train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  qwen3 prepare must stay unpromoted")
PY

QWEN25_PACK="$ROOT/examples/fixtures/qwen25-instruct.pack.json"
echo "-- qwen2.5 instruct qlora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$QWEN25_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/qwen25-qlora"
grep -q "^template: qwen$" "$WORKDIR/qwen25-qlora/recipe.yaml"
grep -q "^template: qwen$" "$WORKDIR/qwen25-qlora/export.yaml"
grep -q "^lora_rank: 16$" "$WORKDIR/qwen25-qlora/recipe.yaml"
grep -q "^packing: true$" "$WORKDIR/qwen25-qlora/recipe.yaml"
if grep -q "^template: qwen3_nothink$" "$WORKDIR/qwen25-qlora/recipe.yaml"; then
  echo "FAIL  qwen2.5 instruct recipe used qwen3_nothink"
  exit 1
fi
if grep -q "^template: qwen3$" "$WORKDIR/qwen25-qlora/recipe.yaml"; then
  echo "FAIL  qwen2.5 instruct recipe used the qwen3 template"
  exit 1
fi
grep -q "quantization_bit: 4" "$WORKDIR/qwen25-qlora/recipe.yaml"
grep -q "quantization_method: bnb" "$WORKDIR/qwen25-qlora/recipe.yaml"
grep -q 'model_name_or_path: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/qwen25-qlora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/qwen25-qlora/recipe.yaml"; then
  echo "FAIL  qwen2.5 recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/qwen25-qlora/export.yaml"; then
  echo "FAIL  qwen2.5 export.yaml must not set quantization_bit"
  exit 1
fi
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct QLoRA." "$WORKDIR/qwen25-qlora/NEXT.md"
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct QLoRA." "$WORKDIR/qwen25-qlora/PREPARE.md"
if grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x LoRA/QLoRA." "$WORKDIR/qwen25-qlora/NEXT.md" "$WORKDIR/qwen25-qlora/PREPARE.md"; then
  echo "FAIL  qwen2.5 fixture prepare wrote the qwen3 instruct reproduce note"
  exit 1
fi
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen2.5 Instruct QLoRA prepare." "$WORKDIR/qwen25-qlora/NEXT.md" "$WORKDIR/qwen25-qlora/PREPARE.md"; then
  echo "FAIL  qwen2.5 QLoRA prepare took the Qwen2.5 Instruct LoRA reproduce note"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/qwen25-qlora/NEXT.md" "$WORKDIR/qwen25-qlora/PREPARE.md"; then
  echo "FAIL  qwen2.5 prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
if [[ -e "$WORKDIR/qwen25-qlora/train.py" || -e "$WORKDIR/qwen25-qlora/train.sh" ]]; then
  echo "FAIL  qwen2.5 prepare must not write a train script"
  exit 1
fi
python3 - "$WORKDIR/qwen25-qlora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("driver") != "llamafactory-qlora":
    raise SystemExit(f"FAIL  qwen2.5 driver={prepare.get('driver')}")
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  qwen2.5 seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "Qwen/Qwen2.5-0.5B-Instruct":
    raise SystemExit(f"FAIL  qwen2.5 train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  qwen2.5 prepare must stay unpromoted")
PY
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$QWEN25_PACK" \
  --driver llamafactory-lora \
  --out "$WORKDIR/qwen25-pack-lora"
if grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct QLoRA." "$WORKDIR/qwen25-pack-lora/NEXT.md" "$WORKDIR/qwen25-pack-lora/PREPARE.md"; then
  echo "FAIL  qwen2.5 fixture on the LoRA card wrote the QLoRA reproduce note"
  exit 1
fi
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen2.5 Instruct QLoRA prepare." "$WORKDIR/qwen25-pack-lora/NEXT.md"
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen2.5 Instruct QLoRA prepare." "$WORKDIR/qwen25-pack-lora/PREPARE.md"
grep -q "^template: qwen$" "$WORKDIR/qwen25-pack-lora/recipe.yaml"
if grep -q "quantization_bit" "$WORKDIR/qwen25-pack-lora/recipe.yaml"; then
  echo "FAIL  qwen2.5 LoRA recipe must omit quantization_bit"
  exit 1
fi

QWEN25_LORA_PACK="$ROOT/examples/fixtures/qwen25-instruct-lora.pack.json"
echo "-- qwen2.5 instruct lora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$QWEN25_LORA_PACK" \
  --driver llamafactory-lora \
  --out "$WORKDIR/qwen25-lora"
grep -q "^template: qwen$" "$WORKDIR/qwen25-lora/recipe.yaml"
grep -q "^template: qwen$" "$WORKDIR/qwen25-lora/export.yaml"
grep -q "^lora_rank: 8$" "$WORKDIR/qwen25-lora/recipe.yaml"
grep -q "^packing: false$" "$WORKDIR/qwen25-lora/recipe.yaml"
if grep -q "^template: qwen3_nothink$" "$WORKDIR/qwen25-lora/recipe.yaml"; then
  echo "FAIL  qwen2.5 instruct LoRA recipe used qwen3_nothink"
  exit 1
fi
if grep -q "^template: qwen3$" "$WORKDIR/qwen25-lora/recipe.yaml"; then
  echo "FAIL  qwen2.5 instruct LoRA recipe used the qwen3 template"
  exit 1
fi
grep -q 'model_name_or_path: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/qwen25-lora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/qwen25-lora/recipe.yaml"; then
  echo "FAIL  qwen2.5 LoRA recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/qwen25-lora/recipe.yaml" "$WORKDIR/qwen25-lora/export.yaml"; then
  echo "FAIL  qwen2.5 LoRA recipe must omit quantization_bit"
  exit 1
fi
if grep -q "quantization_method" "$WORKDIR/qwen25-lora/recipe.yaml" "$WORKDIR/qwen25-lora/export.yaml"; then
  echo "FAIL  qwen2.5 LoRA recipe must omit quantization_method"
  exit 1
fi
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen2.5 Instruct QLoRA prepare." "$WORKDIR/qwen25-lora/NEXT.md"
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen2.5 Instruct QLoRA prepare." "$WORKDIR/qwen25-lora/PREPARE.md"
if grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct QLoRA." "$WORKDIR/qwen25-lora/NEXT.md" "$WORKDIR/qwen25-lora/PREPARE.md"; then
  echo "FAIL  qwen2.5 LoRA fixture prepare wrote the QLoRA reproduce note"
  exit 1
fi
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen3 Instruct QLoRA prepare." "$WORKDIR/qwen25-lora/NEXT.md" "$WORKDIR/qwen25-lora/PREPARE.md"; then
  echo "FAIL  qwen2.5 LoRA fixture prepare wrote the Qwen3 LoRA reproduce note"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/qwen25-lora/NEXT.md" "$WORKDIR/qwen25-lora/PREPARE.md"; then
  echo "FAIL  qwen2.5 LoRA prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
if [[ -e "$WORKDIR/qwen25-lora/train.py" || -e "$WORKDIR/qwen25-lora/train.sh" ]]; then
  echo "FAIL  qwen2.5 LoRA prepare must not write a train script"
  exit 1
fi
python3 - "$WORKDIR/qwen25-lora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("driver") != "llamafactory-lora":
    raise SystemExit(f"FAIL  qwen2.5 lora driver={prepare.get('driver')}")
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  qwen2.5 lora seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "Qwen/Qwen2.5-0.5B-Instruct":
    raise SystemExit(f"FAIL  qwen2.5 lora train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  qwen2.5 lora prepare must stay unpromoted")
PY
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$QWEN25_LORA_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/qwen25-lora-pack-qlora"
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen2.5 Instruct QLoRA prepare." "$WORKDIR/qwen25-lora-pack-qlora/NEXT.md" "$WORKDIR/qwen25-lora-pack-qlora/PREPARE.md"; then
  echo "FAIL  qwen2.5 LoRA fixture on the QLoRA card wrote the LoRA reproduce note"
  exit 1
fi
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct QLoRA." "$WORKDIR/qwen25-lora-pack-qlora/NEXT.md"
grep -q "quantization_method: bnb" "$WORKDIR/qwen25-lora-pack-qlora/recipe.yaml"
grep -q "quantization_bit: 4" "$WORKDIR/qwen25-lora-pack-qlora/recipe.yaml"
grep -q "^template: qwen$" "$WORKDIR/qwen25-lora-pack-qlora/recipe.yaml"

DEEPSEEK_PACK="$ROOT/examples/fixtures/deepseek-r1-distill.pack.json"
echo "-- deepseek r1 distill qlora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$DEEPSEEK_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/deepseek-r1-distill-qlora"
grep -q "^template: deepseekr1$" "$WORKDIR/deepseek-r1-distill-qlora/recipe.yaml"
grep -q "^template: deepseekr1$" "$WORKDIR/deepseek-r1-distill-qlora/export.yaml"
grep -q "^lora_rank: 16$" "$WORKDIR/deepseek-r1-distill-qlora/recipe.yaml"
grep -q "^packing: true$" "$WORKDIR/deepseek-r1-distill-qlora/recipe.yaml"
if grep -q "^template: qwen$" "$WORKDIR/deepseek-r1-distill-qlora/recipe.yaml"; then
  echo "FAIL  deepseek r1 distill recipe used the qwen template"
  exit 1
fi
if grep -q "^template: llama3$" "$WORKDIR/deepseek-r1-distill-qlora/recipe.yaml"; then
  echo "FAIL  deepseek r1 distill recipe used the llama3 template"
  exit 1
fi
grep -q "quantization_bit: 4" "$WORKDIR/deepseek-r1-distill-qlora/recipe.yaml"
grep -q "quantization_method: bnb" "$WORKDIR/deepseek-r1-distill-qlora/recipe.yaml"
grep -q 'model_name_or_path: "deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B"' "$WORKDIR/deepseek-r1-distill-qlora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/deepseek-r1-distill-qlora/recipe.yaml"; then
  echo "FAIL  deepseek recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/deepseek-r1-distill-qlora/export.yaml"; then
  echo "FAIL  deepseek export.yaml must not set quantization_bit"
  exit 1
fi
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, and Qwen3 Instruct QLoRA." "$WORKDIR/deepseek-r1-distill-qlora/NEXT.md"
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, and Qwen3 Instruct QLoRA." "$WORKDIR/deepseek-r1-distill-qlora/PREPARE.md"
if grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct QLoRA." "$WORKDIR/deepseek-r1-distill-qlora/NEXT.md" "$WORKDIR/deepseek-r1-distill-qlora/PREPARE.md"; then
  echo "FAIL  deepseek prepare took the qwen2.5 instruct reproduce note"
  exit 1
fi
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the DeepSeek-R1-Distill chat QLoRA prepare." "$WORKDIR/deepseek-r1-distill-qlora/NEXT.md" "$WORKDIR/deepseek-r1-distill-qlora/PREPARE.md"; then
  echo "FAIL  deepseek QLoRA prepare took the DeepSeek-R1-Distill LoRA reproduce note"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/deepseek-r1-distill-qlora/NEXT.md" "$WORKDIR/deepseek-r1-distill-qlora/PREPARE.md"; then
  echo "FAIL  deepseek prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
if [[ -e "$WORKDIR/deepseek-r1-distill-qlora/train.py" || -e "$WORKDIR/deepseek-r1-distill-qlora/train.sh" ]]; then
  echo "FAIL  deepseek prepare must not write a train script"
  exit 1
fi
python3 - "$WORKDIR/deepseek-r1-distill-qlora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("driver") != "llamafactory-qlora":
    raise SystemExit(f"FAIL  deepseek driver={prepare.get('driver')}")
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  deepseek seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B":
    raise SystemExit(f"FAIL  deepseek train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  deepseek prepare must stay unpromoted")
PY
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$DEEPSEEK_PACK" \
  --driver llamafactory-lora \
  --out "$WORKDIR/deepseek-r1-distill-pack-lora"
if grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, and Qwen3 Instruct QLoRA." "$WORKDIR/deepseek-r1-distill-pack-lora/NEXT.md" "$WORKDIR/deepseek-r1-distill-pack-lora/PREPARE.md"; then
  echo "FAIL  deepseek fixture on the LoRA card wrote the QLoRA reproduce note"
  exit 1
fi
grep -q "^template: deepseekr1$" "$WORKDIR/deepseek-r1-distill-pack-lora/recipe.yaml"
if grep -q "quantization_bit" "$WORKDIR/deepseek-r1-distill-pack-lora/recipe.yaml"; then
  echo "FAIL  deepseek LoRA recipe must omit quantization_bit"
  exit 1
fi
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the DeepSeek-R1-Distill chat QLoRA prepare." "$WORKDIR/deepseek-r1-distill-pack-lora/NEXT.md"
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the DeepSeek-R1-Distill chat QLoRA prepare." "$WORKDIR/deepseek-r1-distill-pack-lora/PREPARE.md"
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen2.5 Instruct QLoRA prepare." "$WORKDIR/deepseek-r1-distill-pack-lora/NEXT.md" "$WORKDIR/deepseek-r1-distill-pack-lora/PREPARE.md"; then
  echo "FAIL  deepseek fixture on the LoRA card wrote the Qwen2.5 Instruct LoRA reproduce note"
  exit 1
fi

DEEPSEEK_LORA_PACK="$ROOT/examples/fixtures/deepseek-r1-distill-lora.pack.json"
echo "-- deepseek r1 distill lora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$DEEPSEEK_LORA_PACK" \
  --driver llamafactory-lora \
  --out "$WORKDIR/deepseek-r1-distill-lora"
grep -q "^template: deepseekr1$" "$WORKDIR/deepseek-r1-distill-lora/recipe.yaml"
grep -q "^template: deepseekr1$" "$WORKDIR/deepseek-r1-distill-lora/export.yaml"
grep -q "^lora_rank: 8$" "$WORKDIR/deepseek-r1-distill-lora/recipe.yaml"
grep -q "^packing: false$" "$WORKDIR/deepseek-r1-distill-lora/recipe.yaml"
if grep -q "^template: qwen$" "$WORKDIR/deepseek-r1-distill-lora/recipe.yaml"; then
  echo "FAIL  deepseek r1 distill LoRA recipe used the qwen template"
  exit 1
fi
if grep -q "^template: llama3$" "$WORKDIR/deepseek-r1-distill-lora/recipe.yaml"; then
  echo "FAIL  deepseek r1 distill LoRA recipe used the llama3 template"
  exit 1
fi
grep -q 'model_name_or_path: "deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B"' "$WORKDIR/deepseek-r1-distill-lora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/deepseek-r1-distill-lora/recipe.yaml"; then
  echo "FAIL  deepseek LoRA recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/deepseek-r1-distill-lora/recipe.yaml" "$WORKDIR/deepseek-r1-distill-lora/export.yaml"; then
  echo "FAIL  deepseek LoRA recipe must omit quantization_bit"
  exit 1
fi
if grep -q "quantization_method" "$WORKDIR/deepseek-r1-distill-lora/recipe.yaml" "$WORKDIR/deepseek-r1-distill-lora/export.yaml"; then
  echo "FAIL  deepseek LoRA recipe must omit quantization_method"
  exit 1
fi
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the DeepSeek-R1-Distill chat QLoRA prepare." "$WORKDIR/deepseek-r1-distill-lora/NEXT.md"
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the DeepSeek-R1-Distill chat QLoRA prepare." "$WORKDIR/deepseek-r1-distill-lora/PREPARE.md"
if grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, and Qwen3 Instruct QLoRA." "$WORKDIR/deepseek-r1-distill-lora/NEXT.md" "$WORKDIR/deepseek-r1-distill-lora/PREPARE.md"; then
  echo "FAIL  deepseek LoRA fixture prepare wrote the QLoRA reproduce note"
  exit 1
fi
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen2.5 Instruct QLoRA prepare." "$WORKDIR/deepseek-r1-distill-lora/NEXT.md" "$WORKDIR/deepseek-r1-distill-lora/PREPARE.md"; then
  echo "FAIL  deepseek LoRA fixture prepare wrote the Qwen2.5 Instruct LoRA reproduce note"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/deepseek-r1-distill-lora/NEXT.md" "$WORKDIR/deepseek-r1-distill-lora/PREPARE.md"; then
  echo "FAIL  deepseek LoRA prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
if [[ -e "$WORKDIR/deepseek-r1-distill-lora/train.py" || -e "$WORKDIR/deepseek-r1-distill-lora/train.sh" ]]; then
  echo "FAIL  deepseek LoRA prepare must not write a train script"
  exit 1
fi
python3 - "$WORKDIR/deepseek-r1-distill-lora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("driver") != "llamafactory-lora":
    raise SystemExit(f"FAIL  deepseek lora driver={prepare.get('driver')}")
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  deepseek lora seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B":
    raise SystemExit(f"FAIL  deepseek lora train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  deepseek lora prepare must stay unpromoted")
PY
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$DEEPSEEK_LORA_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/deepseek-r1-distill-lora-pack-qlora"
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the DeepSeek-R1-Distill chat QLoRA prepare." "$WORKDIR/deepseek-r1-distill-lora-pack-qlora/NEXT.md" "$WORKDIR/deepseek-r1-distill-lora-pack-qlora/PREPARE.md"; then
  echo "FAIL  deepseek LoRA fixture on the QLoRA card wrote the LoRA reproduce note"
  exit 1
fi
grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, and Qwen3 Instruct QLoRA." "$WORKDIR/deepseek-r1-distill-lora-pack-qlora/NEXT.md"
grep -q "quantization_method: bnb" "$WORKDIR/deepseek-r1-distill-lora-pack-qlora/recipe.yaml"
grep -q "quantization_bit: 4" "$WORKDIR/deepseek-r1-distill-lora-pack-qlora/recipe.yaml"
grep -q "^template: deepseekr1$" "$WORKDIR/deepseek-r1-distill-lora-pack-qlora/recipe.yaml"

QWEN3_LORA_PACK="$ROOT/examples/fixtures/qwen3-instruct-lora.pack.json"
echo "-- qwen3 instruct lora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$QWEN3_LORA_PACK" \
  --driver llamafactory-lora \
  --out "$WORKDIR/qwen3-lora"
grep -q "^template: qwen3_nothink$" "$WORKDIR/qwen3-lora/recipe.yaml"
grep -q "^template: qwen3_nothink$" "$WORKDIR/qwen3-lora/export.yaml"
grep -q "^lora_rank: 8$" "$WORKDIR/qwen3-lora/recipe.yaml"
grep -q "^packing: false$" "$WORKDIR/qwen3-lora/recipe.yaml"
if grep -q "^template: qwen3$" "$WORKDIR/qwen3-lora/recipe.yaml"; then
  echo "FAIL  qwen3 instruct LoRA recipe used the thinking template"
  exit 1
fi
if grep -q "^template: qwen$" "$WORKDIR/qwen3-lora/recipe.yaml"; then
  echo "FAIL  qwen3 instruct LoRA recipe used the qwen2 template"
  exit 1
fi
grep -q 'model_name_or_path: "Qwen/Qwen3-4B-Instruct-2507"' "$WORKDIR/qwen3-lora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/qwen3-lora/recipe.yaml"; then
  echo "FAIL  qwen3 LoRA recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/qwen3-lora/recipe.yaml" "$WORKDIR/qwen3-lora/export.yaml"; then
  echo "FAIL  qwen3 LoRA recipe must omit quantization_bit"
  exit 1
fi
if grep -q "quantization_method" "$WORKDIR/qwen3-lora/recipe.yaml" "$WORKDIR/qwen3-lora/export.yaml"; then
  echo "FAIL  qwen3 LoRA recipe must omit quantization_method"
  exit 1
fi
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen3 Instruct QLoRA prepare." "$WORKDIR/qwen3-lora/NEXT.md"
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen3 Instruct QLoRA prepare." "$WORKDIR/qwen3-lora/PREPARE.md"
if grep -q "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x LoRA/QLoRA." "$WORKDIR/qwen3-lora/NEXT.md" "$WORKDIR/qwen3-lora/PREPARE.md"; then
  echo "FAIL  qwen3 LoRA fixture prepare wrote the QLoRA reproduce note"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/qwen3-lora/NEXT.md" "$WORKDIR/qwen3-lora/PREPARE.md"; then
  echo "FAIL  qwen3 LoRA prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
if [[ -e "$WORKDIR/qwen3-lora/train.py" || -e "$WORKDIR/qwen3-lora/train.sh" ]]; then
  echo "FAIL  qwen3 LoRA prepare must not write a train script"
  exit 1
fi
python3 - "$WORKDIR/qwen3-lora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("driver") != "llamafactory-lora":
    raise SystemExit(f"FAIL  qwen3 lora driver={prepare.get('driver')}")
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  qwen3 lora seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "Qwen/Qwen3-4B-Instruct-2507":
    raise SystemExit(f"FAIL  qwen3 lora train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  qwen3 lora prepare must stay unpromoted")
PY
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$QWEN3_LORA_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/qwen3-lora-pack-qlora"
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen3 Instruct QLoRA prepare." "$WORKDIR/qwen3-lora-pack-qlora/NEXT.md" "$WORKDIR/qwen3-lora-pack-qlora/PREPARE.md"; then
  echo "FAIL  qwen3 LoRA fixture on the QLoRA card wrote the LoRA reproduce note"
  exit 1
fi
grep -q "quantization_method: bnb" "$WORKDIR/qwen3-lora-pack-qlora/recipe.yaml"
grep -q "quantization_bit: 4" "$WORKDIR/qwen3-lora-pack-qlora/recipe.yaml"

LLAMA32_LORA_PACK="$ROOT/examples/fixtures/llama32-instruct-lora.pack.json"
echo "-- llama 3.2 instruct lora reproduce target --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$LLAMA32_LORA_PACK" \
  --driver llamafactory-lora \
  --out "$WORKDIR/llama32-lora"
grep -q "^template: llama3$" "$WORKDIR/llama32-lora/recipe.yaml"
grep -q "^template: llama3$" "$WORKDIR/llama32-lora/export.yaml"
grep -q "^lora_rank: 8$" "$WORKDIR/llama32-lora/recipe.yaml"
grep -q "^packing: false$" "$WORKDIR/llama32-lora/recipe.yaml"
if grep -q "^template: llama3_" "$WORKDIR/llama32-lora/recipe.yaml"; then
  echo "FAIL  llama32 instruct LoRA recipe used a llama3_ template name"
  exit 1
fi
if grep -q "^template: mllama$" "$WORKDIR/llama32-lora/recipe.yaml"; then
  echo "FAIL  llama32 instruct LoRA recipe used the vision template"
  exit 1
fi
grep -q 'model_name_or_path: "meta-llama/Llama-3.2-3B-Instruct"' "$WORKDIR/llama32-lora/recipe.yaml"
if grep -q 'model_name_or_path: "llama3"' "$WORKDIR/llama32-lora/recipe.yaml"; then
  echo "FAIL  llama32 LoRA recipe named the seat tag as model_name_or_path"
  exit 1
fi
if grep -q "quantization_bit" "$WORKDIR/llama32-lora/recipe.yaml" "$WORKDIR/llama32-lora/export.yaml"; then
  echo "FAIL  llama32 LoRA recipe must omit quantization_bit"
  exit 1
fi
if grep -q "quantization_method" "$WORKDIR/llama32-lora/recipe.yaml" "$WORKDIR/llama32-lora/export.yaml"; then
  echo "FAIL  llama32 LoRA recipe must omit quantization_method"
  exit 1
fi
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Llama-3.2 Instruct QLoRA prepare." "$WORKDIR/llama32-lora/NEXT.md"
grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Llama-3.2 Instruct QLoRA prepare." "$WORKDIR/llama32-lora/PREPARE.md"
if grep -q "Reproduce target beside Phi-3 and Qwen LoRA/QLoRA." "$WORKDIR/llama32-lora/NEXT.md" "$WORKDIR/llama32-lora/PREPARE.md"; then
  echo "FAIL  llama32 LoRA fixture prepare wrote the QLoRA reproduce note"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/llama32-lora/NEXT.md" "$WORKDIR/llama32-lora/PREPARE.md"; then
  echo "FAIL  llama32 LoRA prepare must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
if [[ -e "$WORKDIR/llama32-lora/train.py" || -e "$WORKDIR/llama32-lora/train.sh" ]]; then
  echo "FAIL  llama32 LoRA prepare must not write a train script"
  exit 1
fi
python3 - "$WORKDIR/llama32-lora/prepare.json" <<'PY'
import json, sys
prepare = json.load(open(sys.argv[1]))
if prepare.get("driver") != "llamafactory-lora":
    raise SystemExit(f"FAIL  llama32 lora driver={prepare.get('driver')}")
if prepare.get("base_model") != "llama3" or prepare.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  llama32 lora seat={prepare.get('base_model')} tag={prepare.get('seat_tag')}")
if prepare.get("train_base_model") != "meta-llama/Llama-3.2-3B-Instruct":
    raise SystemExit(f"FAIL  llama32 lora train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  llama32 lora prepare must stay unpromoted")
PY
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$LLAMA32_LORA_PACK" \
  --driver llamafactory-qlora \
  --out "$WORKDIR/llama32-lora-pack-qlora"
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Llama-3.2 Instruct QLoRA prepare." "$WORKDIR/llama32-lora-pack-qlora/NEXT.md" "$WORKDIR/llama32-lora-pack-qlora/PREPARE.md"; then
  echo "FAIL  llama32 LoRA fixture on the QLoRA card wrote the LoRA reproduce note"
  exit 1
fi
grep -q "Reproduce target beside Phi-3 and Qwen LoRA/QLoRA." "$WORKDIR/llama32-lora-pack-qlora/NEXT.md"
grep -q "quantization_method: bnb" "$WORKDIR/llama32-lora-pack-qlora/recipe.yaml"
grep -q "quantization_bit: 4" "$WORKDIR/llama32-lora-pack-qlora/recipe.yaml"
grep -q "^template: llama3$" "$WORKDIR/llama32-lora-pack-qlora/recipe.yaml"

echo "-- axolotl cards refuse a seat tag with no train base --"
for driver in axolotl-lora axolotl-qlora; do
  set +e
  estate enrich prepare \
    --estate "$SEATED_ONLY" \
    --pack "$PACK" \
    --driver "$driver" \
    --out "$WORKDIR/${driver}-seat-only" \
    >/tmp/train-prepare-axolotl-seat.out 2>/tmp/train-prepare-axolotl-seat.err
  ax_seat_rc=$?
  set -e
  if [[ "$ax_seat_rc" -eq 0 ]]; then
    echo "FAIL  $driver without a train base must refuse:train-base"
    exit 1
  fi
  if ! grep -q "refuse:train-base" /tmp/train-prepare-axolotl-seat.out /tmp/train-prepare-axolotl-seat.err; then
    echo "FAIL  $driver seat tag did not refuse:train-base"
    cat /tmp/train-prepare-axolotl-seat.out /tmp/train-prepare-axolotl-seat.err
    exit 1
  fi
  if grep -q "meta-llama" /tmp/train-prepare-axolotl-seat.out /tmp/train-prepare-axolotl-seat.err; then
    echo "FAIL  $driver refuse must not invent a Llama-3 Hub repo"
    exit 1
  fi
  if [[ -e "$WORKDIR/${driver}-seat-only" ]]; then
    echo "FAIL  $driver seat-only prepare wrote an output directory"
    exit 1
  fi
done

estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-lora \
  --job train \
  --out "$WORKDIR/axolotl"
test -f "$WORKDIR/axolotl/axolotl.yml"
grep -q "^adapter: lora$" "$WORKDIR/axolotl/axolotl.yml"
grep -q "^load_in_8bit: false$" "$WORKDIR/axolotl/axolotl.yml"
grep -q "^load_in_4bit: false$" "$WORKDIR/axolotl/axolotl.yml"
grep -q "^sequence_len: 2048$" "$WORKDIR/axolotl/axolotl.yml"
grep -q "^micro_batch_size: 2$" "$WORKDIR/axolotl/axolotl.yml"
grep -q "^gradient_accumulation_steps: 2$" "$WORKDIR/axolotl/axolotl.yml"
grep -q "^lora_r: 16$" "$WORKDIR/axolotl/axolotl.yml"
grep -q "^lora_alpha: 32$" "$WORKDIR/axolotl/axolotl.yml"
grep -q "^num_epochs: 1$" "$WORKDIR/axolotl/axolotl.yml"
grep -q "^optimizer: adamw_8bit$" "$WORKDIR/axolotl/axolotl.yml"
grep -q "^saves_per_epoch: 1$" "$WORKDIR/axolotl/axolotl.yml"
grep -q "examples/llama-3/lora-1b.yml" "$WORKDIR/axolotl/axolotl.yml"
if grep -q "^max_steps:" "$WORKDIR/axolotl/axolotl.yml"; then
  echo "FAIL  default axolotl-lora must leave max_steps unset"
  exit 1
fi
if grep -q "^adapter: qlora$" "$WORKDIR/axolotl/axolotl.yml"; then
  echo "FAIL  axolotl-lora must stay bf16 LoRA"
  exit 1
fi
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
grep -q "examples/llama-3/lora-1b.yml" "$WORKDIR/axolotl/NEXT.md"
if grep -q "Edit axolotl.yml" "$WORKDIR/axolotl/NEXT.md"; then
  echo "FAIL  axolotl-lora NEXT.md must not ask for a hand edit"
  exit 1
fi
grep -q "estate enrich merge-adapt --prepared $WORKDIR/axolotl --adapter $WORKDIR/axolotl/outputs" "$WORKDIR/axolotl/NEXT.md"
grep -q "axolotl merge-lora $WORKDIR/axolotl/axolotl.yml --lora-model-dir=$WORKDIR/axolotl/outputs" "$WORKDIR/axolotl/NEXT.md"
if grep -q -- "--dequant" "$WORKDIR/axolotl/NEXT.md" "$WORKDIR/axolotl/PREPARE.md"; then
  echo "FAIL  axolotl-lora merge print must stay the bf16 line"
  exit 1
fi
grep -q "estate enrich gguf-convert --prepared $WORKDIR/axolotl --weights $WORKDIR/axolotl/outputs/merged" "$WORKDIR/axolotl/NEXT.md"
grep -q "estate enrich local-seat --prepared $WORKDIR/axolotl --weights $WORKDIR/axolotl/outputs/merged" "$WORKDIR/axolotl/NEXT.md"
grep -q "python3 convert_hf_to_gguf.py $WORKDIR/axolotl/outputs/merged --outfile $WORKDIR/axolotl/outputs/merged.gguf --outtype auto" "$WORKDIR/axolotl/NEXT.md"
grep -q "Axolotl does not write GGUF" "$WORKDIR/axolotl/NEXT.md"
grep -q "Axolotl does not write GGUF" "$WORKDIR/axolotl/PREPARE.md"
grep -q "estate enrich merge-adapt --prepared $WORKDIR/axolotl --adapter $WORKDIR/axolotl/outputs" "$WORKDIR/axolotl/PREPARE.md"
grep -q "axolotl merge-lora $WORKDIR/axolotl/axolotl.yml --lora-model-dir=$WORKDIR/axolotl/outputs" "$WORKDIR/axolotl/PREPARE.md"

echo "-- axolotl-qlora matches examples/llama-3/qlora.yml --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-qlora \
  --out "$WORKDIR/axolotl-qlora"
grep -q "^adapter: qlora$" "$WORKDIR/axolotl-qlora/axolotl.yml"
grep -q "^load_in_8bit: false$" "$WORKDIR/axolotl-qlora/axolotl.yml"
grep -q "^load_in_4bit: true$" "$WORKDIR/axolotl-qlora/axolotl.yml"
grep -q "^sequence_len: 4096$" "$WORKDIR/axolotl-qlora/axolotl.yml"
grep -q "^micro_batch_size: 2$" "$WORKDIR/axolotl-qlora/axolotl.yml"
grep -q "^gradient_accumulation_steps: 4$" "$WORKDIR/axolotl-qlora/axolotl.yml"
grep -q "^lora_r: 32$" "$WORKDIR/axolotl-qlora/axolotl.yml"
grep -q "^lora_alpha: 16$" "$WORKDIR/axolotl-qlora/axolotl.yml"
grep -q "^num_epochs: 4$" "$WORKDIR/axolotl-qlora/axolotl.yml"
grep -q "^optimizer: paged_adamw_32bit$" "$WORKDIR/axolotl-qlora/axolotl.yml"
grep -q "examples/llama-3/qlora.yml" "$WORKDIR/axolotl-qlora/axolotl.yml"
grep -q 'base_model: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/axolotl-qlora/axolotl.yml"
if grep -Eq '^base_model: "llama3"' "$WORKDIR/axolotl-qlora/axolotl.yml"; then
  echo "FAIL  axolotl-qlora base_model must be the train base"
  exit 1
fi
grep -q "axolotl train $WORKDIR/axolotl-qlora/axolotl.yml" "$WORKDIR/axolotl-qlora/NEXT.md"
if grep -q "Edit axolotl.yml" "$WORKDIR/axolotl-qlora/NEXT.md"; then
  echo "FAIL  axolotl-qlora NEXT.md must not ask for a hand edit"
  exit 1
fi
grep -q "estate enrich merge-adapt --prepared $WORKDIR/axolotl-qlora --adapter $WORKDIR/axolotl-qlora/outputs" "$WORKDIR/axolotl-qlora/NEXT.md"
grep -q "axolotl merge-lora $WORKDIR/axolotl-qlora/axolotl.yml --lora-model-dir=$WORKDIR/axolotl-qlora/outputs --dequant" "$WORKDIR/axolotl-qlora/NEXT.md"
grep -q "estate enrich gguf-convert --prepared $WORKDIR/axolotl-qlora --weights $WORKDIR/axolotl-qlora/outputs/merged" "$WORKDIR/axolotl-qlora/NEXT.md"
grep -q "Axolotl does not write GGUF" "$WORKDIR/axolotl-qlora/PREPARE.md"
grep -q "axolotl merge-lora $WORKDIR/axolotl-qlora/axolotl.yml --lora-model-dir=$WORKDIR/axolotl-qlora/outputs --dequant" "$WORKDIR/axolotl-qlora/PREPARE.md"

echo "-- axolotl --max-steps writes max_steps and omits saves_per_epoch --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-lora \
  --max-steps 10 \
  --out "$WORKDIR/axolotl-steps"
grep -q "^max_steps: 10$" "$WORKDIR/axolotl-steps/axolotl.yml"
grep -q "^save_steps: 10$" "$WORKDIR/axolotl-steps/axolotl.yml"
if grep -q "^saves_per_epoch:" "$WORKDIR/axolotl-steps/axolotl.yml"; then
  echo "FAIL  axolotl gauge must omit saves_per_epoch"
  exit 1
fi
set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-qlora \
  --max-steps 0 \
  --out "$WORKDIR/axolotl-zero" \
  >/tmp/train-prepare-axolotl-zero.out 2>/tmp/train-prepare-axolotl-zero.err
ax_zero_rc=$?
set -e
if [[ "$ax_zero_rc" -eq 0 ]]; then
  echo "FAIL  axolotl --max-steps 0 must refuse"
  exit 1
fi
if ! grep -q "refuse:max-steps" /tmp/train-prepare-axolotl-zero.out /tmp/train-prepare-axolotl-zero.err; then
  echo "FAIL  axolotl --max-steps 0 did not refuse:max-steps"
  cat /tmp/train-prepare-axolotl-zero.out /tmp/train-prepare-axolotl-zero.err
  exit 1
fi
if [[ -e "$WORKDIR/axolotl-zero" ]]; then
  echo "FAIL  axolotl --max-steps 0 wrote an output directory"
  exit 1
fi

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
  --driver llamafactory-lora \
  --from-feed \
  --state-dir "$HYDRATE" \
  --out "$WORKDIR/hydrated-lora"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-lora \
  --from-feed \
  --state-dir "$HYDRATE" \
  --out "$WORKDIR/hydrated-ax"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-qlora \
  --from-feed \
  --state-dir "$HYDRATE" \
  --out "$WORKDIR/hydrated-axq"
python3 - "$WORKDIR/hydrated-lf" "$WORKDIR/hydrated-lora" "$WORKDIR/hydrated-ax" "$WORKDIR/hydrated-axq" <<'PY'
import json, sys
from pathlib import Path
for directory, shape in ((sys.argv[1], "messages"), (sys.argv[2], "messages"), (sys.argv[3], "instruction"), (sys.argv[4], "instruction")):
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
    recipe = root / "recipe.yaml"
    if recipe.is_file() and "llamafactory-lora" in str(root):
        body = recipe.read_text()
        if "quantization_bit" in body or "quantization_method" in body:
            raise SystemExit(f"FAIL  {directory} LoRA feed recipe quantized")
        if "dataset_mode: feed" not in body:
            raise SystemExit(f"FAIL  {directory} LoRA recipe missing feed mode")
PY

echo "-- all-drivers train writes the train base into axolotl.yml --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --all-drivers \
  --job train \
  --state-dir "$WORKDIR/all-state" \
  >"$WORKDIR/all-drivers.out"
grep -q "omit driver=mlx-lm-lora" "$WORKDIR/all-drivers.out"
grep -q "refuse:host" "$WORKDIR/all-drivers.out"
grep -q "prepared=7" "$WORKDIR/all-drivers.out"
if [[ -e "$WORKDIR/all-state/enrich/overnight-traces/mlx-lm-lora" ]]; then
  echo "FAIL  all-drivers on host any must omit mlx-lm-lora"
  exit 1
fi
grep -q 'base_model: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/all-state/enrich/overnight-traces/axolotl-lora/axolotl.yml"
grep -q "^adapter: lora$" "$WORKDIR/all-state/enrich/overnight-traces/axolotl-lora/axolotl.yml"
grep -q "^load_in_4bit: false$" "$WORKDIR/all-state/enrich/overnight-traces/axolotl-lora/axolotl.yml"
if grep -Eq '^base_model: "llama3"' "$WORKDIR/all-state/enrich/overnight-traces/axolotl-lora/axolotl.yml"; then
  echo "FAIL  all-drivers axolotl-lora still points base_model at the seat tag"
  exit 1
fi
test -f "$WORKDIR/all-state/enrich/overnight-traces/axolotl-qlora/axolotl.yml"
grep -q 'base_model: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/all-state/enrich/overnight-traces/axolotl-qlora/axolotl.yml"
grep -q "^adapter: qlora$" "$WORKDIR/all-state/enrich/overnight-traces/axolotl-qlora/axolotl.yml"
grep -q "^load_in_4bit: true$" "$WORKDIR/all-state/enrich/overnight-traces/axolotl-qlora/axolotl.yml"
if grep -Eq '^base_model: "llama3"' "$WORKDIR/all-state/enrich/overnight-traces/axolotl-qlora/axolotl.yml"; then
  echo "FAIL  all-drivers axolotl-qlora still points base_model at the seat tag"
  exit 1
fi
test -f "$WORKDIR/all-state/enrich/overnight-traces/unsloth-qlora/UNSLOTH.md"
test -f "$WORKDIR/all-state/enrich/overnight-traces/unsloth-qlora/NEXT.md"
test -f "$WORKDIR/all-state/enrich/overnight-traces/unsloth-qlora/prepare.json"
if [[ -e "$WORKDIR/all-state/enrich/overnight-traces/unsloth-qlora/train_unsloth.py" ]]; then
  echo "FAIL  all-drivers unsloth-qlora must not write a script"
  exit 1
fi
if [[ -e "$WORKDIR/all-state/enrich/overnight-traces/unsloth-qlora/dataset.jsonl" ]]; then
  echo "FAIL  all-drivers unsloth-qlora must not write dataset.jsonl"
  exit 1
fi
grep -q 'train_base_model: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/all-state/enrich/overnight-traces/unsloth-qlora/UNSLOTH.md"
grep -q 'seat_tag: "llama3"' "$WORKDIR/all-state/enrich/overnight-traces/unsloth-qlora/UNSLOTH.md"
grep -q "does not call Unsloth" "$WORKDIR/all-state/enrich/overnight-traces/unsloth-qlora/NEXT.md"
grep -q "FROM llama3" "$WORKDIR/all-state/enrich/overnight-traces/ollama-modelfile/Modelfile"
test -f "$WORKDIR/all-state/enrich/overnight-traces/llamafactory-lora/recipe.yaml"
grep -q "^lora_rank: 8$" "$WORKDIR/all-state/enrich/overnight-traces/llamafactory-lora/recipe.yaml"
if grep -q "quantization_bit" "$WORKDIR/all-state/enrich/overnight-traces/llamafactory-lora/recipe.yaml"; then
  echo "FAIL  all-drivers LoRA recipe must omit quantization_bit"
  exit 1
fi
grep -q "quantization_method: bnb" "$WORKDIR/all-state/enrich/overnight-traces/llamafactory-qlora/recipe.yaml"

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

echo "-- official-scale writes the qwen3 lora sft fields and keeps the gauge path --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver llamafactory-lora \
  --official-scale \
  --out "$WORKDIR/lora-official"
grep -q "^cutoff_len: 2048$" "$WORKDIR/lora-official/recipe.yaml"
grep -q "^num_train_epochs: 3.0$" "$WORKDIR/lora-official/recipe.yaml"
grep -q "^gradient_accumulation_steps: 8$" "$WORKDIR/lora-official/recipe.yaml"
grep -q "^warmup_ratio: 0.1$" "$WORKDIR/lora-official/recipe.yaml"
grep -q "^lora_rank: 8$" "$WORKDIR/lora-official/recipe.yaml"
grep -q "^packing: false$" "$WORKDIR/lora-official/recipe.yaml"
if grep -q "quantization_bit" "$WORKDIR/lora-official/recipe.yaml"; then
  echo "FAIL  official-scale LoRA recipe must omit quantization_bit"
  exit 1
fi
if grep -q "quantization_method" "$WORKDIR/lora-official/recipe.yaml"; then
  echo "FAIL  official-scale LoRA recipe must omit quantization_method"
  exit 1
fi
if grep -q "^max_steps:" "$WORKDIR/lora-official/recipe.yaml"; then
  echo "FAIL  official-scale without --max-steps must leave max_steps unset"
  exit 1
fi
grep -q "This prepare did not merge" "$WORKDIR/lora-official/NEXT.md"
grep -q "The merge has not happened." "$WORKDIR/lora-official/NEXT.md"

estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver llamafactory-lora \
  --official-scale \
  --max-steps 10 \
  --out "$WORKDIR/lora-official-gauge"
grep -q "^max_steps: 10$" "$WORKDIR/lora-official-gauge/recipe.yaml"
grep -q "^save_steps: 10$" "$WORKDIR/lora-official-gauge/recipe.yaml"
grep -q "^cutoff_len: 2048$" "$WORKDIR/lora-official-gauge/recipe.yaml"
grep -q "^num_train_epochs: 3.0$" "$WORKDIR/lora-official-gauge/recipe.yaml"
if grep -q "quantization_bit" "$WORKDIR/lora-official-gauge/recipe.yaml"; then
  echo "FAIL  official-scale gauge LoRA recipe must omit quantization_bit"
  exit 1
fi

estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver llamafactory-qlora \
  --official-scale \
  --out "$WORKDIR/qlora-official"
grep -q "^cutoff_len: 2048$" "$WORKDIR/qlora-official/recipe.yaml"
grep -q "quantization_bit: 4" "$WORKDIR/qlora-official/recipe.yaml"
grep -q "quantization_method: bnb" "$WORKDIR/qlora-official/recipe.yaml"
if grep -q "quantization_bit" "$WORKDIR/qlora-official/export.yaml"; then
  echo "FAIL  official-scale QLoRA export.yaml must omit quantization_bit"
  exit 1
fi

estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-lora \
  --official-scale \
  --out "$WORKDIR/axolotl-official"
grep -q "^adapter: lora$" "$WORKDIR/axolotl-official/axolotl.yml"
grep -q "^load_in_4bit: false$" "$WORKDIR/axolotl-official/axolotl.yml"
grep -q "^num_epochs: 1$" "$WORKDIR/axolotl-official/axolotl.yml"
grep -q "^gradient_accumulation_steps: 2$" "$WORKDIR/axolotl-official/axolotl.yml"
grep -q "^sequence_len: 2048$" "$WORKDIR/axolotl-official/axolotl.yml"
grep -q "^warmup_ratio: 0.1$" "$WORKDIR/axolotl-official/axolotl.yml"
grep -q "This card stays on examples/llama-3/lora-1b.yml" "$WORKDIR/axolotl-official/NEXT.md"
if grep -q "^adapter: qlora$" "$WORKDIR/axolotl-official/axolotl.yml"; then
  echo "FAIL  official-scale axolotl-lora must stay bf16 LoRA"
  exit 1
fi

estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver axolotl-qlora \
  --official-scale \
  --out "$WORKDIR/axolotl-qlora-official"
grep -q "^adapter: qlora$" "$WORKDIR/axolotl-qlora-official/axolotl.yml"
grep -q "^load_in_4bit: true$" "$WORKDIR/axolotl-qlora-official/axolotl.yml"
grep -q "^sequence_len: 4096$" "$WORKDIR/axolotl-qlora-official/axolotl.yml"
grep -q "^num_epochs: 4$" "$WORKDIR/axolotl-qlora-official/axolotl.yml"
grep -q "^gradient_accumulation_steps: 4$" "$WORKDIR/axolotl-qlora-official/axolotl.yml"

set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver ollama-modelfile \
  --official-scale \
  --out "$WORKDIR/ollama-official" \
  >/tmp/train-prepare-official-ollama.out 2>/tmp/train-prepare-official-ollama.err
official_ollama_rc=$?
set -e
if [[ "$official_ollama_rc" -eq 0 ]]; then
  echo "FAIL  --official-scale on ollama-modelfile must refuse"
  exit 1
fi
if ! grep -q "refuse:official-scale" /tmp/train-prepare-official-ollama.out /tmp/train-prepare-official-ollama.err; then
  echo "FAIL  ollama official-scale did not refuse:official-scale"
  cat /tmp/train-prepare-official-ollama.out /tmp/train-prepare-official-ollama.err
  exit 1
fi
if [[ -e "$WORKDIR/ollama-official" ]]; then
  echo "FAIL  official-scale refuse wrote an output directory"
  exit 1
fi

QUANT_DIR="$WORKDIR/lora-quant-export"
cp -a "$WORKDIR/lora-official" "$QUANT_DIR"
printf '\nquantization_bit: 4\n' >> "$QUANT_DIR/export.yaml"
set +e
estate enrich import-trained \
  --estate "$SEATED" \
  --prepared "$QUANT_DIR" \
  --tag cell-enrich-overnight-traces \
  --adapter "$ADAPTER" \
  >/tmp/train-prepare-quant-export.out 2>/tmp/train-prepare-quant-export.err
quant_rc=$?
set -e
if [[ "$quant_rc" -eq 0 ]]; then
  echo "FAIL  quantized export.yaml must refuse:export"
  exit 1
fi
if ! grep -q "refuse:export" /tmp/train-prepare-quant-export.out /tmp/train-prepare-quant-export.err; then
  echo "FAIL  quantized export did not refuse:export"
  cat /tmp/train-prepare-quant-export.out /tmp/train-prepare-quant-export.err
  exit 1
fi

echo "-- unsloth-qlora is an optional handoff and refuses a missing train base --"
set +e
estate enrich prepare \
  --estate "$SEATED_ONLY" \
  --pack "$PACK" \
  --driver unsloth-qlora \
  --out "$WORKDIR/unsloth-seat-only" \
  >/tmp/train-prepare-unsloth-seat.out 2>/tmp/train-prepare-unsloth-seat.err
unsloth_seat_rc=$?
set -e
if [[ "$unsloth_seat_rc" -eq 0 ]]; then
  echo "FAIL  unsloth-qlora without a train base must refuse:train-base"
  exit 1
fi
if ! grep -q "refuse:train-base" /tmp/train-prepare-unsloth-seat.out /tmp/train-prepare-unsloth-seat.err; then
  echo "FAIL  unsloth-qlora seat-only did not refuse:train-base"
  cat /tmp/train-prepare-unsloth-seat.out /tmp/train-prepare-unsloth-seat.err
  exit 1
fi
if [[ -e "$WORKDIR/unsloth-seat-only" ]]; then
  echo "FAIL  unsloth-qlora refuse wrote an output directory"
  exit 1
fi

set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver unsloth-qlora \
  --official-scale \
  --out "$WORKDIR/unsloth-official" \
  >/tmp/train-prepare-unsloth-official.out 2>/tmp/train-prepare-unsloth-official.err
unsloth_official_rc=$?
set -e
if [[ "$unsloth_official_rc" -eq 0 ]]; then
  echo "FAIL  --official-scale on unsloth-qlora must refuse"
  exit 1
fi
if ! grep -q "refuse:official-scale" /tmp/train-prepare-unsloth-official.out /tmp/train-prepare-unsloth-official.err; then
  echo "FAIL  unsloth-qlora official-scale did not refuse:official-scale"
  cat /tmp/train-prepare-unsloth-official.out /tmp/train-prepare-unsloth-official.err
  exit 1
fi
if [[ -e "$WORKDIR/unsloth-official" ]]; then
  echo "FAIL  unsloth official-scale refuse wrote an output directory"
  exit 1
fi

SEATED_CKSUM="$(cksum "$SEATED")"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver unsloth-qlora \
  --out "$WORKDIR/unsloth"
test -f "$WORKDIR/unsloth/UNSLOTH.md"
test -f "$WORKDIR/unsloth/PREPARE.md"
test -f "$WORKDIR/unsloth/NEXT.md"
test -f "$WORKDIR/unsloth/prepare.json"
if [[ -e "$WORKDIR/unsloth/train_unsloth.py" ]]; then
  echo "FAIL  unsloth-qlora must not write train_unsloth.py"
  exit 1
fi
if compgen -G "$WORKDIR/unsloth/"'*.py' > /dev/null; then
  echo "FAIL  unsloth-qlora must not write a python script"
  exit 1
fi
if [[ -e "$WORKDIR/unsloth/dataset.jsonl" || -e "$WORKDIR/unsloth/recipe.yaml" || -e "$WORKDIR/unsloth/axolotl.yml" ]]; then
  echo "FAIL  unsloth-qlora must not write a recipe"
  exit 1
fi
grep -q "operator-owned" "$WORKDIR/unsloth/UNSLOTH.md"
grep -q "does not call Unsloth" "$WORKDIR/unsloth/UNSLOTH.md"
grep -q "Nvidia-only" "$WORKDIR/unsloth/UNSLOTH.md"
grep -q 'train_base_model: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/unsloth/UNSLOTH.md"
grep -q 'seat_tag: "llama3"' "$WORKDIR/unsloth/UNSLOTH.md"
grep -q "https://unsloth.ai/docs/get-started/install" "$WORKDIR/unsloth/NEXT.md"
grep -q "https://unsloth.ai/docs/get-started/fine-tuning-llms-guide" "$WORKDIR/unsloth/NEXT.md"
grep -q "uv pip install unsloth --torch-backend=auto" "$WORKDIR/unsloth/NEXT.md"
grep -q "import-trained" "$WORKDIR/unsloth/NEXT.md"
grep -q "estate enrich merge-adapt" "$WORKDIR/unsloth/NEXT.md"
grep -q "save_pretrained_merged" "$WORKDIR/unsloth/NEXT.md"
grep -q "merged_16bit" "$WORKDIR/unsloth/PREPARE.md"
grep -q "estate enrich gguf-convert" "$WORKDIR/unsloth/NEXT.md"
grep -q "READY_FOR_LIVE_TEST: no" "$WORKDIR/unsloth/NEXT.md"
if [[ -e "$WORKDIR/unsloth/merged" ]]; then
  echo "FAIL  unsloth-qlora must not write a merged directory"
  exit 1
fi
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/unsloth/NEXT.md"; then
  echo "FAIL  unsloth-qlora must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
python3 - "$WORKDIR/unsloth/prepare.json" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
if doc.get("driver") != "unsloth-qlora" or doc.get("job") != "train":
    raise SystemExit(f"FAIL  driver={doc.get('driver')} job={doc.get('job')}")
if doc.get("base_model") != "llama3" or doc.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  seat={doc.get('base_model')} tag={doc.get('seat_tag')}")
if doc.get("train_base_model") != "Qwen/Qwen2.5-0.5B-Instruct":
    raise SystemExit(f"FAIL  train_base={doc.get('train_base_model')}")
if doc.get("dataset_mode") is not None:
    raise SystemExit(f"FAIL  dataset_mode={doc.get('dataset_mode')}")
if doc.get("promoted") is not False or doc.get("auto_apply") is not False or doc.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  unsloth prepare claims a promote or an estate rewrite")
PY
if [[ "$(cksum "$SEATED")" != "$SEATED_CKSUM" ]]; then
  echo "FAIL  unsloth-qlora prepare rewrote the seated estate"
  exit 1
fi
AFTER_UNSLOTH="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER_UNSLOTH" ]]; then
  echo "FAIL  unsloth-qlora prepare rewrote examples/estate.yaml"
  exit 1
fi

echo "-- mlx-lm-lora refuses a non-apple host and writes a handoff on apple-silicon --"
set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver mlx-lm-lora \
  --out "$WORKDIR/mlx-wrong-host" \
  >/tmp/train-prepare-mlx-host.out 2>/tmp/train-prepare-mlx-host.err
mlx_host_rc=$?
set -e
if [[ "$mlx_host_rc" -eq 0 ]]; then
  echo "FAIL  mlx-lm-lora on host any must refuse:host"
  exit 1
fi
if ! grep -q "refuse:host" /tmp/train-prepare-mlx-host.out /tmp/train-prepare-mlx-host.err; then
  echo "FAIL  mlx-lm-lora wrong host did not refuse:host"
  cat /tmp/train-prepare-mlx-host.out /tmp/train-prepare-mlx-host.err
  exit 1
fi
if [[ -e "$WORKDIR/mlx-wrong-host" ]]; then
  echo "FAIL  mlx-lm-lora refuse:host wrote an output directory"
  exit 1
fi

APPLE_PACK="$WORKDIR/apple.pack.json"
python3 - "$PACK" "$APPLE_PACK" <<'PY'
import json, sys
src, dest = sys.argv[1:]
doc = json.load(open(src))
doc["host_class_affinity"] = "apple-silicon"
json.dump(doc, open(dest, "w"), indent=2)
open(dest, "a").write("\n")
PY

set +e
estate enrich prepare \
  --estate "$SEATED_ONLY" \
  --pack "$APPLE_PACK" \
  --driver mlx-lm-lora \
  --out "$WORKDIR/mlx-seat-only" \
  >/tmp/train-prepare-mlx-seat.out 2>/tmp/train-prepare-mlx-seat.err
mlx_seat_rc=$?
set -e
if [[ "$mlx_seat_rc" -eq 0 ]]; then
  echo "FAIL  mlx-lm-lora without a train base must refuse:train-base"
  exit 1
fi
if ! grep -q "refuse:train-base" /tmp/train-prepare-mlx-seat.out /tmp/train-prepare-mlx-seat.err; then
  echo "FAIL  mlx-lm-lora seat-only did not refuse:train-base"
  cat /tmp/train-prepare-mlx-seat.out /tmp/train-prepare-mlx-seat.err
  exit 1
fi
if [[ -e "$WORKDIR/mlx-seat-only" ]]; then
  echo "FAIL  mlx-lm-lora seat-only wrote an output directory"
  exit 1
fi

set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$APPLE_PACK" \
  --driver mlx-lm-lora \
  --official-scale \
  --out "$WORKDIR/mlx-official" \
  >/tmp/train-prepare-mlx-official.out 2>/tmp/train-prepare-mlx-official.err
mlx_official_rc=$?
set -e
if [[ "$mlx_official_rc" -eq 0 ]]; then
  echo "FAIL  --official-scale on mlx-lm-lora must refuse"
  exit 1
fi
if ! grep -q "refuse:official-scale" /tmp/train-prepare-mlx-official.out /tmp/train-prepare-mlx-official.err; then
  echo "FAIL  mlx-lm-lora official-scale did not refuse:official-scale"
  cat /tmp/train-prepare-mlx-official.out /tmp/train-prepare-mlx-official.err
  exit 1
fi
if [[ -e "$WORKDIR/mlx-official" ]]; then
  echo "FAIL  mlx official-scale refuse wrote an output directory"
  exit 1
fi

set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$APPLE_PACK" \
  --driver mlx-lm-lora \
  --from-feed \
  --state-dir "$WORKDIR" \
  --out "$WORKDIR/mlx-feed" \
  >/tmp/train-prepare-mlx-feed.out 2>/tmp/train-prepare-mlx-feed.err
mlx_feed_rc=$?
set -e
if [[ "$mlx_feed_rc" -eq 0 ]]; then
  echo "FAIL  --from-feed on mlx-lm-lora alone must refuse"
  exit 1
fi
if ! grep -q "refuse:dataset" /tmp/train-prepare-mlx-feed.out /tmp/train-prepare-mlx-feed.err; then
  echo "FAIL  mlx-lm-lora from-feed did not refuse:dataset"
  cat /tmp/train-prepare-mlx-feed.out /tmp/train-prepare-mlx-feed.err
  exit 1
fi
if [[ -e "$WORKDIR/mlx-feed" ]]; then
  echo "FAIL  mlx from-feed refuse wrote an output directory"
  exit 1
fi

SEATED_CKSUM_MLX="$(cksum "$SEATED")"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$APPLE_PACK" \
  --driver mlx-lm-lora \
  --out "$WORKDIR/mlx"
test -f "$WORKDIR/mlx/MLX.md"
test -f "$WORKDIR/mlx/PREPARE.md"
test -f "$WORKDIR/mlx/NEXT.md"
test -f "$WORKDIR/mlx/prepare.json"
if compgen -G "$WORKDIR/mlx/"'*.py' > /dev/null; then
  echo "FAIL  mlx-lm-lora must not write a python script"
  exit 1
fi
if [[ -e "$WORKDIR/mlx/dataset.jsonl" || -e "$WORKDIR/mlx/recipe.yaml" || -e "$WORKDIR/mlx/axolotl.yml" ]]; then
  echo "FAIL  mlx-lm-lora must not write a recipe"
  exit 1
fi
grep -q "operator-owned" "$WORKDIR/mlx/MLX.md"
grep -q "does not call mlx-lm" "$WORKDIR/mlx/MLX.md"
grep -q "apple-silicon" "$WORKDIR/mlx/MLX.md"
grep -q 'train_base_model: "Qwen/Qwen2.5-0.5B-Instruct"' "$WORKDIR/mlx/MLX.md"
grep -q 'seat_tag: "llama3"' "$WORKDIR/mlx/MLX.md"
grep -q "https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md" "$WORKDIR/mlx/NEXT.md"
grep -q 'pip install "mlx-lm\[train\]"' "$WORKDIR/mlx/NEXT.md"
grep -q "mlx_lm.fuse --model <path_to_model>" "$WORKDIR/mlx/NEXT.md"
grep -q "mlx_lm.fuse --export-gguf" "$WORKDIR/mlx/NEXT.md"
grep -q "adapters.safetensors" "$WORKDIR/mlx/NEXT.md"
grep -q "ggml-model-f16.gguf" "$WORKDIR/mlx/MLX.md"
grep -q "estate enrich merge-adapt" "$WORKDIR/mlx/PREPARE.md"
grep -q "import-trained" "$WORKDIR/mlx/NEXT.md"
grep -q "READY_FOR_LIVE_TEST: no" "$WORKDIR/mlx/NEXT.md"
if grep -q "READY_FOR_LIVE_TEST: yes" "$WORKDIR/mlx/NEXT.md"; then
  echo "FAIL  mlx-lm-lora must keep READY_FOR_LIVE_TEST no"
  exit 1
fi
python3 - "$WORKDIR/mlx/prepare.json" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
if doc.get("driver") != "mlx-lm-lora" or doc.get("job") != "train":
    raise SystemExit(f"FAIL  driver={doc.get('driver')} job={doc.get('job')}")
if doc.get("base_model") != "llama3" or doc.get("seat_tag") != "llama3":
    raise SystemExit(f"FAIL  seat={doc.get('base_model')} tag={doc.get('seat_tag')}")
if doc.get("train_base_model") != "Qwen/Qwen2.5-0.5B-Instruct":
    raise SystemExit(f"FAIL  train_base={doc.get('train_base_model')}")
if doc.get("host_class_affinity") != "apple-silicon":
    raise SystemExit(f"FAIL  host={doc.get('host_class_affinity')}")
if doc.get("dataset_mode") is not None:
    raise SystemExit(f"FAIL  dataset_mode={doc.get('dataset_mode')}")
if doc.get("promoted") is not False or doc.get("auto_apply") is not False or doc.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  mlx prepare claims a promote or an estate rewrite")
PY
if [[ "$(cksum "$SEATED")" != "$SEATED_CKSUM_MLX" ]]; then
  echo "FAIL  mlx-lm-lora prepare rewrote the seated estate"
  exit 1
fi
AFTER_MLX="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER_MLX" ]]; then
  echo "FAIL  mlx-lm-lora prepare rewrote examples/estate.yaml"
  exit 1
fi

echo "PASS  train-prepare (LLaMA-Factory LoRA and QLoRA, Axolotl LoRA and QLoRA, optional Unsloth handoff, optional mlx-lm handoff, official scale, feed hydrate, import-trained; SKIP live train)"
