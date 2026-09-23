#!/usr/bin/env bash
# Target A: Qwen / LLaMA-Factory LoRA operator ladder.
# Prints the existing commands and checks llamafactory-lora prepare artifacts.
# Does not install LLaMA-Factory, does not train, does not merge, does not
# convert, does not create an Ollama model, and does not promote.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

# Throwaway dir stays out of the checkout. prepare refuses a hardware SKU
# anywhere in the output path, including a parent directory name.
WORKDIR="${WORKDIR:-${TMPDIR:-/tmp}/cell-one-lora-journey}"
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
PACK="${PACK:-$ROOT/examples/fixtures/specialist-overnight.pack.json}"
BIN="${ESTATE_BIN:-}"
TRAIN_BASE="Qwen/Qwen2.5-0.5B-Instruct"
SEAT_TAG="llama3"

estate() {
  if [[ -n "$BIN" ]]; then
    "$BIN" "$@"
  else
    cargo run -q -p estate-control -- "$@"
  fi
}

if [[ ! -f "$PACK" ]]; then
  echo "SKIP lora-journey (example pack missing; not a PASS)"
  exit 0
fi

echo "== lora-journey (Target A: Qwen / LLaMA-Factory LoRA; print-only) =="
echo "workdir: $WORKDIR"
echo "READY_FOR_LIVE_TEST: no"
echo "SKIP live train"
echo
echo "Ladder (existing commands only):"
echo "1. estate enrich prepare --driver llamafactory-lora"
echo "   Seat tag stays the Ollama id. Train base is a Hugging Face repo id."
echo "   Example seat ${SEAT_TAG}. Train base ${TRAIN_BASE}. template qwen."
echo "2. llamafactory-cli train <prepared>/recipe.yaml"
echo "   llamafactory-cli export <prepared>/export.yaml"
echo "   Those lines are in NEXT.md. This factory does not run them."
echo "3. estate enrich merge-adapt --prepared <prepared> --adapter <prepared>/outputs"
echo "   Prints llamafactory-cli export <prepared>/export.yaml."
echo "   The yaml shape is examples/merge_lora/qwen3_lora_sft.yaml."
echo "   This factory does not merge."
echo "4. estate enrich gguf-convert --prepared <prepared> --weights <prepared>/export"
echo "   Prints python3 convert_hf_to_gguf.py ... --outfile <prepared>/export.gguf --outtype auto"
echo "5. estate enrich local-seat --prepared <prepared> --weights <prepared>/export.gguf"
echo "   Prints ollama create. This factory does not run it."
echo "6. estate enrich import-trained --adapter <gguf-or-export-or-outputs>"
echo "   Records trained_shape and trained_paths. Does not apply. Does not promote."
echo

BEFORE="$(cksum "$ESTATE")"
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR/logs"
SEATED_ONLY="$WORKDIR/estate-seat-only.yaml"
SEATED="$WORKDIR/estate.yaml"
python3 - "$ESTATE" "$SEATED_ONLY" "$SEATED" "$TRAIN_BASE" <<'PY'
import sys
src, seat_only, seated, train_base = sys.argv[1:]
text = open(src).read()
needle = "  - id: local_slm\n    class: local\n    driver: ollama\n    params:\n"
if needle not in text:
    raise SystemExit("FAIL  local_slm params block missing")
open(seat_only, "w").write(text.replace(needle, needle + '      model: "llama3"\n', 1))
open(seated, "w").write(text.replace(
    needle,
    needle + '      model: "llama3"\n      train_base_model: "' + train_base + '"\n',
    1,
))
PY

echo "-- seat tag without a train base refuses --"
set +e
estate enrich prepare \
  --estate "$SEATED_ONLY" \
  --pack "$PACK" \
  --driver llamafactory-lora \
  --job train \
  --out "$WORKDIR/seat-only" \
  >"$WORKDIR/logs/seat-only.out" 2>"$WORKDIR/logs/seat-only.err"
seat_rc=$?
set -e
if [[ "$seat_rc" -eq 0 ]]; then
  echo "FAIL  seat tag without a train base must refuse:train-base"
  exit 1
fi
if ! grep -q "refuse:train-base" "$WORKDIR/logs/seat-only.out" "$WORKDIR/logs/seat-only.err"; then
  echo "FAIL  seat tag without a train base did not refuse:train-base"
  cat "$WORKDIR/logs/seat-only.out" "$WORKDIR/logs/seat-only.err"
  exit 1
fi
if grep -q "meta-llama" "$WORKDIR/logs/seat-only.out" "$WORKDIR/logs/seat-only.err"; then
  echo "FAIL  refuse must not invent a Llama-3 Hub repo"
  exit 1
fi
if [[ -e "$WORKDIR/seat-only" ]]; then
  echo "FAIL  seat-only prepare wrote an output directory"
  exit 1
fi

echo "-- llamafactory-lora prepare writes the Qwen LoRA card --"
PREPARED="$WORKDIR/llamafactory-lora"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver llamafactory-lora \
  --job train \
  --out "$PREPARED" \
  >"$WORKDIR/logs/prepare.out"
echo "prepared: $PREPARED"

test -f "$PREPARED/recipe.yaml"
test -f "$PREPARED/export.yaml"
test -f "$PREPARED/dataset_info.json"
test -f "$PREPARED/dataset.jsonl"
test -f "$PREPARED/PREPARE.md"
test -f "$PREPARED/NEXT.md"
test -f "$PREPARED/prepare.json"
if [[ -e "$PREPARED/train_unsloth.py" || -e "$PREPARED/convert_hf_to_gguf.py" ]]; then
  echo "FAIL  prepare must not invent a trainer or a converter"
  exit 1
fi

grep -q "stage: sft" "$PREPARED/recipe.yaml"
grep -q "finetuning_type: lora" "$PREPARED/recipe.yaml"
grep -q "^lora_rank: 8$" "$PREPARED/recipe.yaml"
grep -q "^packing: false$" "$PREPARED/recipe.yaml"
grep -q "^template: qwen$" "$PREPARED/recipe.yaml"
grep -q "model_name_or_path: \"${TRAIN_BASE}\"" "$PREPARED/recipe.yaml"
grep -q "^template: qwen$" "$PREPARED/export.yaml"
grep -q "model_name_or_path: \"${TRAIN_BASE}\"" "$PREPARED/export.yaml"
if grep -q "quantization_bit" "$PREPARED/recipe.yaml" "$PREPARED/export.yaml"; then
  echo "FAIL  llamafactory-lora recipe and export must omit quantization_bit"
  exit 1
fi
if grep -q "quantization_method" "$PREPARED/recipe.yaml" "$PREPARED/export.yaml"; then
  echo "FAIL  llamafactory-lora recipe and export must omit quantization_method"
  exit 1
fi
if grep -q "llamafactory-cli" "$PREPARED/prepare.json"; then
  echo "FAIL  prepare.json must not embed a train command"
  exit 1
fi

grep -q "Seat tag: ${SEAT_TAG}" "$PREPARED/PREPARE.md"
grep -q "Train base: ${TRAIN_BASE}" "$PREPARED/PREPARE.md"
grep -q "Dataset mode: scaffold" "$PREPARED/PREPARE.md"
grep -q "not training data" "$PREPARED/NEXT.md"
grep -q "does not require bitsandbytes" "$PREPARED/NEXT.md"
if grep -q "bitsandbytes>=0.49" "$PREPARED/NEXT.md"; then
  echo "FAIL  llamafactory-lora NEXT.md must not install bitsandbytes"
  exit 1
fi
if grep -q "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen3 Instruct QLoRA prepare." "$PREPARED/NEXT.md" "$PREPARED/PREPARE.md"; then
  echo "FAIL  qwen2.5 LoRA prepare took the qwen3 instruct LoRA reproduce note"
  exit 1
fi
grep -q "pip install llamafactory" "$PREPARED/NEXT.md"
grep -q "llamafactory-cli train ${PREPARED}/recipe.yaml" "$PREPARED/NEXT.md"
grep -q "llamafactory-cli export ${PREPARED}/export.yaml" "$PREPARED/NEXT.md"
grep -q "estate enrich merge-adapt --prepared ${PREPARED} --adapter ${PREPARED}/outputs" "$PREPARED/NEXT.md"
grep -q "examples/merge_lora/qwen3_lora_sft.yaml" "$PREPARED/NEXT.md"
grep -q "This prepare did not merge" "$PREPARED/NEXT.md"
grep -q "estate enrich gguf-convert --prepared ${PREPARED} --weights ${PREPARED}/export" "$PREPARED/NEXT.md"
grep -q "python3 convert_hf_to_gguf.py ${PREPARED}/export --outfile ${PREPARED}/export.gguf --outtype auto" "$PREPARED/NEXT.md"
grep -q "estate enrich local-seat --prepared ${PREPARED} --weights ${PREPARED}/export" "$PREPARED/NEXT.md"
grep -q "estate enrich local-seat --prepared ${PREPARED} --adapter ${PREPARED}/outputs" "$PREPARED/NEXT.md"
grep -q "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag cell-enrich-overnight-traces --adapter ${PREPARED}/outputs" "$PREPARED/NEXT.md"
grep -q "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag cell-enrich-overnight-traces --adapter ${PREPARED}/export" "$PREPARED/NEXT.md"
grep -q "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag cell-enrich-overnight-traces --adapter <gguf>" "$PREPARED/NEXT.md"
grep -q "trained_shape" "$PREPARED/NEXT.md"
grep -q "READY_FOR_LIVE_TEST: no" "$PREPARED/NEXT.md"

python3 - "$PREPARED/prepare.json" "$TRAIN_BASE" "$SEAT_TAG" <<'PY'
import json, sys
prepare_path, train_base, seat = sys.argv[1:]
prepare = json.load(open(prepare_path))
if prepare.get("schema") != "cell-one.enrich-prepare.v0":
    raise SystemExit(f"FAIL  schema={prepare.get('schema')}")
if prepare.get("driver") != "llamafactory-lora":
    raise SystemExit(f"FAIL  driver={prepare.get('driver')}")
if prepare.get("job") != "train":
    raise SystemExit(f"FAIL  job={prepare.get('job')}")
if prepare.get("base_model") != seat or prepare.get("seat_tag") != seat:
    raise SystemExit(f"FAIL  seat={prepare.get('base_model')}/{prepare.get('seat_tag')}")
if prepare.get("train_base_model") != train_base:
    raise SystemExit(f"FAIL  train_base_model={prepare.get('train_base_model')}")
if prepare.get("train_base_model") == prepare.get("seat_tag"):
    raise SystemExit("FAIL  train base collapsed onto the seat tag")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False:
    raise SystemExit("FAIL  prepare.json must stay unpromoted")
if prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  prepare.json claims an estate rewrite")
if prepare.get("dataset_mode") != "scaffold":
    raise SystemExit(f"FAIL  dataset_mode={prepare.get('dataset_mode')}")
if prepare.get("dataset_from_feed") is not False:
    raise SystemExit("FAIL  default prepare must not read the feed")
if "trained_shape" in prepare or "trained_paths" in prepare:
    raise SystemExit("FAIL  prepare must not record a trained shape before import-trained")
PY

echo "-- missing merge is refuse:seat (no convert, no GGUF) --"
set +e
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$PREPARED/export" \
  >"$WORKDIR/logs/gguf.out" 2>"$WORKDIR/logs/gguf.err"
gguf_rc=$?
set -e
if [[ "$gguf_rc" -eq 0 ]]; then
  echo "FAIL  gguf-convert must refuse before a merged export exists"
  exit 1
fi
if ! grep -q "refuse:seat" "$WORKDIR/logs/gguf.out" "$WORKDIR/logs/gguf.err"; then
  echo "FAIL  gguf-convert did not refuse:seat"
  cat "$WORKDIR/logs/gguf.out" "$WORKDIR/logs/gguf.err"
  exit 1
fi
if [[ -e "$PREPARED/export.gguf" || -e "$PREPARED/export" ]]; then
  echo "FAIL  gguf-convert wrote a GGUF or an export directory"
  exit 1
fi

set +e
estate enrich local-seat \
  --prepared "$PREPARED" \
  --weights "$PREPARED/export.gguf" \
  >"$WORKDIR/logs/seat.out" 2>"$WORKDIR/logs/seat.err"
local_rc=$?
set -e
if [[ "$local_rc" -eq 0 ]]; then
  echo "FAIL  local-seat must refuse before a GGUF exists"
  exit 1
fi
if ! grep -q "refuse:seat" "$WORKDIR/logs/seat.out" "$WORKDIR/logs/seat.err"; then
  echo "FAIL  local-seat did not refuse:seat"
  cat "$WORKDIR/logs/seat.out" "$WORKDIR/logs/seat.err"
  exit 1
fi
if [[ -e "$PREPARED/export.gguf" ]]; then
  echo "FAIL  local-seat wrote a GGUF"
  exit 1
fi

AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  lora-journey rewrote examples/estate.yaml"
  exit 1
fi

echo
echo "Filled NEXT.md lines (not executed):"
grep -E '^(llamafactory-cli (train|export) |python3 convert_hf_to_gguf.py |estate enrich (merge-adapt|gguf-convert|local-seat|import-trained) )' "$PREPARED/NEXT.md"
echo
echo "PASS  lora-journey (Target A ladder printed; prepare artifacts checked; SKIP live train)"
echo "READY_FOR_LIVE_TEST: no"
