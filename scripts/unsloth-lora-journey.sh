#!/usr/bin/env bash
# Unsloth LoRA optional NEXT path. Print-only.
# Prepares unsloth-lora on a throwaway copy of examples/estate.yaml
# (seat tag llama3, train base Qwen/Qwen2.5-0.5B-Instruct, overnight pack).
# Asserts UNSLOTH.md, PREPARE.md, NEXT.md, and the seat_tag / train_base split.
# Phase prepare stops after refuse:train-base, refuse:adapter (missing and
# wrong shape), and refuse:seat on missing paths. Phase seat (and the default
# phase all) then prints merge-adapt, gguf-convert, local-seat, and
# import-trained against fixture stubs, including refuse:tokenizer on a
# Qwen-shaped merged directory. local-seat --adapter stays refuse:adapter.
# Does not install Unsloth, does not call Unsloth, does not merge, does not
# run convert_hf_to_gguf.py, does not run ollama, and does not promote.
# CELL_SEAT_LIVE=1 and CELL_TRAIN_LIVE=1 do not start a live phase.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
# Resolves estate fail-closed: executable ESTATE_BIN, then target/release/estate,
# then target/debug/estate, then cargo on PATH. Does not invent a binary.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

PHASE="${UNSLOTH_LORA_PHASE:-all}"
case "$PHASE" in
  prepare|seat|all) ;;
  *)
    echo "FAIL  UNSLOTH_LORA_PHASE must be prepare, seat, or all (got: $PHASE)" >&2
    exit 1
    ;;
esac

# Throwaway dir stays out of the checkout. prepare refuses a hardware SKU
# anywhere in the output path, including a parent directory name.
WORKDIR="${WORKDIR:-${TMPDIR:-/tmp}/cell-one-unsloth-lora-journey}"
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
PACK="${PACK:-$ROOT/examples/fixtures/specialist-overnight.pack.json}"
BIN="${ESTATE_BIN:-}"
TRAIN_BASE="Qwen/Qwen2.5-0.5B-Instruct"
SEAT_TAG="llama3"
TAG="cell-enrich-overnight-traces"

resolve_estate() {
  if [[ -n "${ESTATE_RESOLVED:-}" ]]; then
    return 0
  fi
  if [[ -n "$BIN" && -x "$BIN" ]]; then
    ESTATE_CMD=("$BIN")
  elif [[ -n "$BIN" ]]; then
    echo "FAIL  ESTATE_BIN is set but not executable: $BIN" >&2
    echo "FAIL  refusing $ROOT/target/release/estate and $ROOT/target/debug/estate" >&2
    exit 1
  elif [[ -x "$ROOT/target/release/estate" ]]; then
    ESTATE_CMD=("$ROOT/target/release/estate")
  elif [[ -x "$ROOT/target/debug/estate" ]]; then
    ESTATE_CMD=("$ROOT/target/debug/estate")
  elif command -v cargo >/dev/null 2>&1; then
    ESTATE_CMD=(cargo run -q -p estate-control --)
  else
    echo "FAIL  estate binary unresolved. Set ESTATE_BIN, or build $ROOT/target/release/estate or $ROOT/target/debug/estate. cargo is not on PATH." >&2
    exit 1
  fi
  ESTATE_RESOLVED=1
}

estate() {
  resolve_estate
  "${ESTATE_CMD[@]}" "$@"
}

require() {
  local needle="$1"
  shift
  if ! grep -F -q -- "$needle" "$@"; then
    echo "FAIL  missing: $needle" >&2
    exit 1
  fi
}

finish() {
  local after seated_after
  after="$(cksum "$ESTATE")"
  if [[ "$BEFORE" != "$after" ]]; then
    echo "FAIL  unsloth-lora-journey rewrote examples/estate.yaml" >&2
    exit 1
  fi
  if [[ -n "${SEATED_BEFORE:-}" && -f "${SEATED:-}" ]]; then
    seated_after="$(cksum "$SEATED")"
    if [[ "$SEATED_BEFORE" != "$seated_after" ]]; then
      echo "FAIL  import-trained rewrote the lab estate copy" >&2
      exit 1
    fi
  fi
}

if [[ ! -f "$PACK" ]]; then
  echo "SKIP unsloth-lora-journey (example pack missing; not a PASS)"
  exit 0
fi

echo "== unsloth-lora-journey (optional Unsloth LoRA NEXT path; print-only; phase=${PHASE}) =="
echo "workdir: $WORKDIR"
echo "READY_FOR_LIVE_TEST: no"
if [[ "${CELL_SEAT_LIVE:-}" == "1" || "${CELL_TRAIN_LIVE:-}" == "1" ]]; then
  echo "CELL_SEAT_LIVE or CELL_TRAIN_LIVE is set. This journey stays print-only."
  echo "Live train, merge, convert, and seat stay on the operator host."
fi
echo "SKIP live train"
echo "SKIP live convert"
echo "SKIP live seat"
echo
echo "Ladder (existing commands only; fixture stubs, not weights):"
echo "1. estate enrich prepare --driver unsloth-lora"
echo "   Seat tag stays the Ollama id. Train base is a Hugging Face repo id."
echo "   Example seat ${SEAT_TAG}. Train base ${TRAIN_BASE}."
echo "   Writes UNSLOTH.md, PREPARE.md, NEXT.md, and prepare.json."
echo "   Does not write a script, a recipe, or dataset.jsonl."
echo "2. estate enrich merge-adapt --prepared <prepared> --adapter <prepared>/adapter"
echo "   The adapter stub holds adapter_config.json and adapter_model.safetensors."
echo "   Prints save_pretrained_merged with save_method merged_16bit."
echo "   The directory argument is <prepared>/merged. This factory does not merge."
echo "3. estate enrich gguf-convert --prepared <prepared> --weights <prepared>/merged"
echo "   Before the good stub, a Qwen-shaped merged directory is refuse:tokenizer."
echo "   The good stub is config.json ({}) plus model.safetensors."
echo "   Prints python3 convert_hf_to_gguf.py ... --outfile <prepared>/merged.gguf --outtype auto"
echo "   Also prints the three manual lines (f16, bf16, q8_0)."
echo "4. estate enrich local-seat --prepared <prepared> --weights <prepared>/merged.gguf"
echo "   local-seat --adapter stays refuse:adapter on this card."
echo "   local-seat is print-only. It prints the Modelfile and does not write it."
echo "   The GGUF stub starts with GGUF magic. Prints ollama create."
echo "   This factory does not run it and does not write that file."
echo "5. estate enrich import-trained --adapter <prepared>/merged.gguf"
echo "   Records trained_shape gguf and trained_paths."
echo "   The proposal stays auto_apply=false. Does not apply the estate. Does not promote."
echo

resolve_estate

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
  --driver unsloth-lora \
  --job train \
  --out "$WORKDIR/seat-only" \
  >"$WORKDIR/logs/seat-only.out" 2>"$WORKDIR/logs/seat-only.err"
seat_rc=$?
set -e
if [[ "$seat_rc" -eq 0 ]]; then
  echo "FAIL  seat tag without a train base must refuse:train-base" >&2
  exit 1
fi
require "refuse:train-base" "$WORKDIR/logs/seat-only.out" "$WORKDIR/logs/seat-only.err"
if grep -q "meta-llama" "$WORKDIR/logs/seat-only.out" "$WORKDIR/logs/seat-only.err"; then
  echo "FAIL  refuse must not invent a Llama-3 Hub repo" >&2
  exit 1
fi
if [[ -e "$WORKDIR/seat-only" ]]; then
  echo "FAIL  seat-only prepare wrote an output directory" >&2
  exit 1
fi

echo "-- official-scale on this card alone is refuse:official-scale --"
set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver unsloth-lora \
  --job train \
  --official-scale \
  --out "$WORKDIR/official-scale" \
  >"$WORKDIR/logs/official-scale.out" 2>"$WORKDIR/logs/official-scale.err"
official_rc=$?
set -e
if [[ "$official_rc" -eq 0 ]]; then
  echo "FAIL  --official-scale on unsloth-lora must refuse:official-scale" >&2
  exit 1
fi
require "refuse:official-scale" "$WORKDIR/logs/official-scale.out" "$WORKDIR/logs/official-scale.err"
if [[ -e "$WORKDIR/official-scale" ]]; then
  echo "FAIL  official-scale refuse wrote an output directory" >&2
  exit 1
fi

echo "-- from-feed on this card alone is refuse:dataset --"
set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver unsloth-lora \
  --job train \
  --from-feed \
  --state-dir "$WORKDIR" \
  --out "$WORKDIR/from-feed" \
  >"$WORKDIR/logs/from-feed.out" 2>"$WORKDIR/logs/from-feed.err"
feed_rc=$?
set -e
if [[ "$feed_rc" -eq 0 ]]; then
  echo "FAIL  --from-feed on unsloth-lora must refuse:dataset" >&2
  exit 1
fi
require "refuse:dataset" "$WORKDIR/logs/from-feed.out" "$WORKDIR/logs/from-feed.err"
if [[ -e "$WORKDIR/from-feed" ]]; then
  echo "FAIL  from-feed refuse wrote an output directory" >&2
  exit 1
fi

echo "-- unsloth-lora prepare writes the operator-owned handoff --"
PREPARED="$WORKDIR/unsloth-lora"
ADAPTER="$PREPARED/adapter"
MERGED="$PREPARED/merged"
GGUF="$PREPARED/merged.gguf"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver unsloth-lora \
  --job train \
  --out "$PREPARED" \
  >"$WORKDIR/logs/prepare.out"
echo "prepared: $PREPARED"

test -f "$PREPARED/UNSLOTH.md"
test -f "$PREPARED/PREPARE.md"
test -f "$PREPARED/NEXT.md"
test -f "$PREPARED/prepare.json"
if [[ -e "$PREPARED/train_unsloth.py" || -e "$PREPARED/recipe.yaml" || -e "$PREPARED/axolotl.yml" || -e "$PREPARED/dataset.jsonl" || -e "$PREPARED/export.yaml" || -e "$PREPARED/convert_hf_to_gguf.py" || -e "$PREPARED/merged" || -e "$PREPARED/adapter" || -e "$GGUF" ]]; then
  echo "FAIL  prepare must not invent a script, a recipe, a dataset, or weights" >&2
  exit 1
fi

require "operator-owned" "$PREPARED/UNSLOTH.md"
require "does not call Unsloth" "$PREPARED/UNSLOTH.md"
require "not a training script" "$PREPARED/UNSLOTH.md"
require "Nvidia-only" "$PREPARED/UNSLOTH.md"
require "Unsloth LoRA handoff" "$PREPARED/UNSLOTH.md"
require "driver: unsloth-lora" "$PREPARED/UNSLOTH.md"
require "does not write load_in_4bit" "$PREPARED/UNSLOTH.md"
require "unsloth-qlora" "$PREPARED/UNSLOTH.md"
if grep -q "Unsloth QLoRA handoff" "$PREPARED/UNSLOTH.md"; then
  echo "FAIL  unsloth-lora handoff must not use the QLoRA title" >&2
  exit 1
fi
require "status: optional" "$PREPARED/UNSLOTH.md"
require "train_base_model: \"${TRAIN_BASE}\"" "$PREPARED/UNSLOTH.md"
require "seat_tag: \"${SEAT_TAG}\"" "$PREPARED/UNSLOTH.md"
require "local-seat --adapter is refuse:adapter" "$PREPARED/UNSLOTH.md"
require "READY_FOR_LIVE_TEST: no" "$PREPARED/UNSLOTH.md"
require "Seat tag: ${SEAT_TAG}" "$PREPARED/PREPARE.md"
require "Train base: ${TRAIN_BASE}" "$PREPARED/PREPARE.md"
require "did not call Unsloth" "$PREPARED/PREPARE.md"
require "merged_16bit" "$PREPARED/PREPARE.md"
require "estate enrich merge-adapt --prepared ${PREPARED} --adapter '<adapter-dir>'" "$PREPARED/NEXT.md"
require "model.save_pretrained_merged(\"${MERGED}\", tokenizer, save_method = \"merged_16bit\")" "$PREPARED/NEXT.md"
require "estate enrich gguf-convert --prepared ${PREPARED} --weights ${MERGED}" "$PREPARED/NEXT.md"
require "python3 convert_hf_to_gguf.py ${MERGED} --outfile ${GGUF} --outtype auto" "$PREPARED/NEXT.md"
require "does not publish --outtype auto" "$PREPARED/NEXT.md"
require "--outtype f16" "$PREPARED/NEXT.md"
require "--outtype bf16" "$PREPARED/NEXT.md"
require "--outtype q8_0" "$PREPARED/NEXT.md"
require "estate enrich local-seat --prepared ${PREPARED} --weights ${MERGED}" "$PREPARED/NEXT.md"
require "local-seat --adapter" "$PREPARED/NEXT.md"
require "refuse:adapter" "$PREPARED/NEXT.md"
require "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag ${TAG} --adapter ${GGUF}" "$PREPARED/NEXT.md"
require "does not call Unsloth" "$PREPARED/NEXT.md"
require "READY_FOR_LIVE_TEST: no" "$PREPARED/NEXT.md"
if grep -q "train_unsloth.py" "$PREPARED/UNSLOTH.md" "$PREPARED/PREPARE.md" "$PREPARED/NEXT.md" "$PREPARED/prepare.json"; then
  echo "FAIL  unsloth-lora must not name a training script" >&2
  exit 1
fi

python3 - "$PREPARED/prepare.json" "$TRAIN_BASE" "$SEAT_TAG" <<'PY'
import json, sys
prepare_path, train_base, seat = sys.argv[1:]
prepare = json.load(open(prepare_path))
if prepare.get("schema") != "cell-one.enrich-prepare.v0":
    raise SystemExit(f"FAIL  schema={prepare.get('schema')}")
if prepare.get("driver") != "unsloth-lora":
    raise SystemExit(f"FAIL  driver={prepare.get('driver')}")
if prepare.get("job") != "train":
    raise SystemExit(f"FAIL  job={prepare.get('job')}")
if prepare.get("pack_id") != "overnight-traces":
    raise SystemExit(f"FAIL  pack_id={prepare.get('pack_id')}")
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
if prepare.get("dataset_mode") not in (None,):
    raise SystemExit(f"FAIL  dataset_mode={prepare.get('dataset_mode')}")
if "trained_shape" in prepare or "trained_paths" in prepare:
    raise SystemExit("FAIL  prepare must not record a trained shape before import-trained")
PY

SEATED_BEFORE="$(cksum "$SEATED")"

echo "-- missing adapter refuses (no merge) --"
set +e
estate enrich merge-adapt \
  --prepared "$PREPARED" \
  --adapter "$ADAPTER" \
  >"$WORKDIR/logs/merge-missing.out" 2>"$WORKDIR/logs/merge-missing.err"
merge_missing_rc=$?
set -e
if [[ "$merge_missing_rc" -eq 0 ]]; then
  echo "FAIL  merge-adapt must refuse before an adapter directory exists" >&2
  exit 1
fi
require "refuse:adapter" "$WORKDIR/logs/merge-missing.out" "$WORKDIR/logs/merge-missing.err"
if [[ -e "$ADAPTER" || -e "$MERGED" ]]; then
  echo "FAIL  merge-adapt wrote an adapter directory or a merged directory" >&2
  exit 1
fi

echo "-- adapter_config.json without adapter_model.safetensors is refuse:adapter --"
mkdir -p "$ADAPTER"
printf '%s\n' '{}' > "$ADAPTER/adapter_config.json"
set +e
estate enrich merge-adapt \
  --prepared "$PREPARED" \
  --adapter "$ADAPTER" \
  >"$WORKDIR/logs/merge-shape.out" 2>"$WORKDIR/logs/merge-shape.err"
shape_rc=$?
set -e
if [[ "$shape_rc" -eq 0 ]]; then
  echo "FAIL  merge-adapt must refuse an adapter directory that has no weight file" >&2
  exit 1
fi
require "refuse:adapter" "$WORKDIR/logs/merge-shape.out" "$WORKDIR/logs/merge-shape.err"
require "adapter_model.safetensors" "$WORKDIR/logs/merge-shape.out" "$WORKDIR/logs/merge-shape.err"
if grep -q "save_pretrained_merged" "$WORKDIR/logs/merge-shape.out" "$WORKDIR/logs/merge-shape.err"; then
  echo "FAIL  wrong-shape adapter printed the merge line" >&2
  exit 1
fi
rm -rf "$ADAPTER"

echo "-- mlx-shaped adapters.safetensors is the wrong shape --"
mkdir -p "$ADAPTER"
printf '%s\n' '{}' > "$ADAPTER/adapter_config.json"
printf '%s\n' 'not-an-unsloth-weight' > "$ADAPTER/adapters.safetensors"
set +e
estate enrich merge-adapt \
  --prepared "$PREPARED" \
  --adapter "$ADAPTER" \
  >"$WORKDIR/logs/merge-mlx-shape.out" 2>"$WORKDIR/logs/merge-mlx-shape.err"
mlx_shape_rc=$?
set -e
if [[ "$mlx_shape_rc" -eq 0 ]]; then
  echo "FAIL  merge-adapt must refuse adapters.safetensors on unsloth-lora" >&2
  exit 1
fi
require "refuse:adapter" "$WORKDIR/logs/merge-mlx-shape.out" "$WORKDIR/logs/merge-mlx-shape.err"
require "adapter_model.safetensors" "$WORKDIR/logs/merge-mlx-shape.out" "$WORKDIR/logs/merge-mlx-shape.err"
rm -rf "$ADAPTER"
if [[ -e "$MERGED" || -e "$GGUF" ]]; then
  echo "FAIL  wrong-shape refuses wrote a merged directory or a GGUF" >&2
  exit 1
fi

echo "-- missing merged directory and GGUF refuse (no convert, no seat) --"
set +e
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$MERGED" \
  >"$WORKDIR/logs/gguf-missing.out" 2>"$WORKDIR/logs/gguf-missing.err"
gguf_missing_rc=$?
set -e
if [[ "$gguf_missing_rc" -eq 0 ]]; then
  echo "FAIL  gguf-convert must refuse before a merged directory exists" >&2
  exit 1
fi
require "refuse:seat" "$WORKDIR/logs/gguf-missing.out" "$WORKDIR/logs/gguf-missing.err"
if [[ -e "$GGUF" || -e "$MERGED" ]]; then
  echo "FAIL  gguf-convert wrote a GGUF or a merged directory" >&2
  exit 1
fi

set +e
estate enrich local-seat \
  --prepared "$PREPARED" \
  --weights "$GGUF" \
  >"$WORKDIR/logs/seat-missing.out" 2>"$WORKDIR/logs/seat-missing.err"
local_missing_rc=$?
set -e
if [[ "$local_missing_rc" -eq 0 ]]; then
  echo "FAIL  local-seat must refuse before a GGUF exists" >&2
  exit 1
fi
require "refuse:seat" "$WORKDIR/logs/seat-missing.out" "$WORKDIR/logs/seat-missing.err"
if [[ -e "$GGUF" ]]; then
  echo "FAIL  local-seat wrote a GGUF" >&2
  exit 1
fi

if [[ "$PHASE" == "prepare" ]]; then
  finish
  echo
  echo "PASS  unsloth-lora-journey (prepare-assert; UNSLOTH.md handoff checked; refuse:adapter; refuse:seat; SKIP live train; SKIP live convert; SKIP live seat)"
  echo "READY_FOR_LIVE_TEST: no"
  exit 0
fi

echo "-- fixture stub: adapter dir with adapter_model.safetensors --"
mkdir -p "$ADAPTER"
printf '%s\n' '{}' > "$ADAPTER/adapter_config.json"
printf '%s\n' 'not-a-real-tensor' > "$ADAPTER/adapter_model.safetensors"
if [[ -e "$MERGED" || -e "$GGUF" ]]; then
  echo "FAIL  adapter stub must not create a merged directory or a GGUF" >&2
  exit 1
fi

echo "-- local-seat --adapter stays refuse:adapter --"
set +e
estate enrich local-seat \
  --prepared "$PREPARED" \
  --adapter "$ADAPTER" \
  >"$WORKDIR/logs/seat-adapter.out" 2>"$WORKDIR/logs/seat-adapter.err"
adapter_seat_rc=$?
set -e
if [[ "$adapter_seat_rc" -eq 0 ]]; then
  echo "FAIL  local-seat --adapter must refuse on unsloth-lora" >&2
  exit 1
fi
require "refuse:adapter" "$WORKDIR/logs/seat-adapter.out" "$WORKDIR/logs/seat-adapter.err"
if grep -q "ollama create" "$WORKDIR/logs/seat-adapter.out" "$WORKDIR/logs/seat-adapter.err"; then
  echo "FAIL  local-seat --adapter printed a create line" >&2
  exit 1
fi
if [[ -e "$PREPARED/Modelfile" || -e "$ADAPTER/Modelfile" ]]; then
  echo "FAIL  local-seat --adapter wrote a Modelfile" >&2
  exit 1
fi

echo "-- adapter directory as --weights is refuse:seat --"
set +e
estate enrich local-seat \
  --prepared "$PREPARED" \
  --weights "$ADAPTER" \
  >"$WORKDIR/logs/seat-weights-adapter.out" 2>"$WORKDIR/logs/seat-weights-adapter.err"
weights_adapter_rc=$?
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$ADAPTER" \
  >"$WORKDIR/logs/gguf-adapter.out" 2>"$WORKDIR/logs/gguf-adapter.err"
gguf_adapter_rc=$?
set -e
if [[ "$weights_adapter_rc" -eq 0 || "$gguf_adapter_rc" -eq 0 ]]; then
  echo "FAIL  --weights on the PEFT directory must refuse:seat" >&2
  exit 1
fi
require "refuse:seat" "$WORKDIR/logs/seat-weights-adapter.out" "$WORKDIR/logs/seat-weights-adapter.err"
require "refuse:seat" "$WORKDIR/logs/gguf-adapter.out" "$WORKDIR/logs/gguf-adapter.err"
if grep -q "ollama create" "$WORKDIR/logs/seat-weights-adapter.out" "$WORKDIR/logs/seat-weights-adapter.err"; then
  echo "FAIL  adapter --weights printed a create line" >&2
  exit 1
fi
if grep -q "python3 convert_hf_to_gguf.py" "$WORKDIR/logs/gguf-adapter.out" "$WORKDIR/logs/gguf-adapter.err"; then
  echo "FAIL  adapter --weights printed the convert line" >&2
  exit 1
fi

echo "-- merge-adapt prints save_pretrained_merged merged_16bit --"
set +e
estate enrich merge-adapt \
  --prepared "$PREPARED" \
  --adapter "$ADAPTER" \
  >"$WORKDIR/logs/merge.out" 2>"$WORKDIR/logs/merge.err"
merge_rc=$?
set -e
if [[ "$merge_rc" -ne 0 ]]; then
  echo "FAIL  merge-adapt did not print" >&2
  cat "$WORKDIR/logs/merge.out" "$WORKDIR/logs/merge.err" >&2
  exit 1
fi
require "model.save_pretrained_merged(\"${MERGED}\", tokenizer, save_method = \"merged_16bit\")" "$WORKDIR/logs/merge.out"
require "save_method = \"lora\"" "$WORKDIR/logs/merge.out"
require "adapter_model.safetensors" "$WORKDIR/logs/merge.out"
require "python3 convert_hf_to_gguf.py ${MERGED} --outfile ${GGUF} --outtype auto" "$WORKDIR/logs/merge.out"
require "python llama.cpp/convert_hf_to_gguf.py ${MERGED} --outfile model-F16.gguf --outtype f16 --split-max-size 50G" "$WORKDIR/logs/merge.out"
require "--outtype bf16" "$WORKDIR/logs/merge.out"
require "--outtype q8_0" "$WORKDIR/logs/merge.out"
require "does not publish --outtype auto" "$WORKDIR/logs/merge.out"
require "estate enrich gguf-convert --prepared ${PREPARED} --weights ${MERGED}" "$WORKDIR/logs/merge.out"
require "estate enrich local-seat --prepared ${PREPARED} --weights ${MERGED}" "$WORKDIR/logs/merge.out"
require "local-seat --adapter on this prepare is refuse:adapter" "$WORKDIR/logs/merge.out"
require "merge-adapt did not merge" "$WORKDIR/logs/merge.out"
require "READY_FOR_LIVE_TEST: no" "$WORKDIR/logs/merge.out"
if grep -q "axolotl merge-lora" "$WORKDIR/logs/merge.out"; then
  echo "FAIL  unsloth merge-adapt printed an Axolotl merge line" >&2
  exit 1
fi
if [[ -e "$MERGED" || -e "$GGUF" ]]; then
  echo "FAIL  merge-adapt wrote a merged directory or a GGUF" >&2
  exit 1
fi

echo "-- Qwen-shaped merged tokenizer is refuse:tokenizer --"
mkdir -p "$MERGED"
printf '%s\n' '{"model_type":"qwen2","architectures":["Qwen2ForCausalLM"]}' > "$MERGED/config.json"
printf '%s\n' 'not-a-real-tensor' > "$MERGED/model.safetensors"
printf '%s\n' '{"extra_special_tokens":["<|im_start|>","<|im_end|>"],"tokenizer_class":"Qwen2Tokenizer"}' > "$MERGED/tokenizer_config.json"
if [[ -e "$MERGED/vocab.json" || -e "$MERGED/merges.txt" || -e "$GGUF" || -e "$MERGED/tokenizer_config.json.bak" ]]; then
  echo "FAIL  Qwen-shaped fixture must omit vocab.json, merges.txt, and a GGUF" >&2
  exit 1
fi

set +e
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$MERGED" \
  >"$WORKDIR/logs/gguf-tokenizer.out" 2>"$WORKDIR/logs/gguf-tokenizer.err"
tokenizer_rc=$?
set -e
if [[ "$tokenizer_rc" -eq 0 ]]; then
  echo "FAIL  gguf-convert must refuse:tokenizer on the Qwen-shaped merged directory" >&2
  cat "$WORKDIR/logs/gguf-tokenizer.out" "$WORKDIR/logs/gguf-tokenizer.err" >&2
  exit 1
fi
for needle in \
  "refuse:tokenizer" \
  "JSON list" \
  "missing vocab.json" \
  "missing merges.txt" \
  "tokenizer_config.json.bak" \
  "HF cache snapshot" \
  "equivalent base checkout" \
  "into the export directory" \
  "HF hub snapshots are often symlinks" \
  "cp -aL" \
  "cp --dereference" \
  "real files, not symlinks" \
  "plain cp -a" \
  "does not follow a symlinked tokenizer_config.json" \
  "re-run estate enrich gguf-convert" \
  "${TRAIN_BASE}"
do
  if ! grep -F -q -- "$needle" "$WORKDIR/logs/gguf-tokenizer.out" "$WORKDIR/logs/gguf-tokenizer.err"; then
    echo "FAIL  Qwen-shaped merged directory did not report: $needle" >&2
    cat "$WORKDIR/logs/gguf-tokenizer.out" "$WORKDIR/logs/gguf-tokenizer.err" >&2
    exit 1
  fi
done
if grep -q "python3 convert_hf_to_gguf.py" "$WORKDIR/logs/gguf-tokenizer.out" "$WORKDIR/logs/gguf-tokenizer.err"; then
  echo "FAIL  refuse:tokenizer printed the convert line" >&2
  exit 1
fi
if [[ -e "$GGUF" || -e "$MERGED/model.gguf" || -e "$MERGED/tokenizer_config.json.bak" || -e "$MERGED/vocab.json" || -e "$MERGED/merges.txt" ]]; then
  echo "FAIL  refuse:tokenizer wrote a GGUF or copied tokenizer files" >&2
  exit 1
fi
if ! grep -q '"extra_special_tokens"' "$MERGED/tokenizer_config.json"; then
  echo "FAIL  refuse:tokenizer rewrote tokenizer_config.json" >&2
  exit 1
fi

echo "-- replace the broken tokenizer with the good merged stub --"
rm -rf "$MERGED"
mkdir -p "$MERGED"
printf '%s\n' '{}' > "$MERGED/config.json"
printf '%s\n' 'not-a-real-tensor' > "$MERGED/model.safetensors"
if [[ -e "$MERGED/tokenizer_config.json" || -e "$MERGED/vocab.json" || -e "$MERGED/merges.txt" ]]; then
  echo "FAIL  good merged stub must not keep the broken tokenizer" >&2
  exit 1
fi
if ! grep -qx '{}' "$MERGED/config.json"; then
  echo "FAIL  good merged stub config.json must stay {}" >&2
  exit 1
fi
if [[ -e "$MERGED/adapter_config.json" || -e "$MERGED/adapter_model.safetensors" || -e "$MERGED/model.gguf" ]]; then
  echo "FAIL  merged stub must be config.json plus a non-adapter safetensors file" >&2
  exit 1
fi
if [[ ! -f "$ADAPTER/adapter_config.json" || ! -f "$ADAPTER/adapter_model.safetensors" ]]; then
  echo "FAIL  replacing the merged stub removed the adapter marker" >&2
  exit 1
fi

echo "-- merged directory passed as --adapter is refuse:adapter --"
set +e
estate enrich merge-adapt \
  --prepared "$PREPARED" \
  --adapter "$MERGED" \
  >"$WORKDIR/logs/merge-merged.out" 2>"$WORKDIR/logs/merge-merged.err"
merge_merged_rc=$?
set -e
if [[ "$merge_merged_rc" -eq 0 ]]; then
  echo "FAIL  merge-adapt must refuse a merged directory" >&2
  exit 1
fi
require "refuse:adapter" "$WORKDIR/logs/merge-merged.out" "$WORKDIR/logs/merge-merged.err"
require "merged export" "$WORKDIR/logs/merge-merged.out" "$WORKDIR/logs/merge-merged.err"

echo "-- gguf-convert prints convert_hf_to_gguf.py --"
set +e
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$MERGED" \
  >"$WORKDIR/logs/gguf-print.out" 2>"$WORKDIR/logs/gguf-print.err"
gguf_rc=$?
set -e
if [[ "$gguf_rc" -ne 0 ]]; then
  echo "FAIL  gguf-convert did not print" >&2
  cat "$WORKDIR/logs/gguf-print.out" "$WORKDIR/logs/gguf-print.err" >&2
  exit 1
fi
require "shape=merged" "$WORKDIR/logs/gguf-print.out"
require "Tokenizer check passed" "$WORKDIR/logs/gguf-print.out"
require "this directory has no tokenizer_config.json" "$WORKDIR/logs/gguf-print.out"
require "python3 convert_hf_to_gguf.py ${MERGED} --outfile ${GGUF} --outtype auto" "$WORKDIR/logs/gguf-print.out"
require "python llama.cpp/convert_hf_to_gguf.py ${MERGED} --outfile model-F16.gguf --outtype f16 --split-max-size 50G" "$WORKDIR/logs/gguf-print.out"
require "model-BF16.gguf" "$WORKDIR/logs/gguf-print.out"
require "model-Q8_0.gguf" "$WORKDIR/logs/gguf-print.out"
require "does not publish --outtype auto" "$WORKDIR/logs/gguf-print.out"
require "estate enrich local-seat --prepared ${PREPARED} --weights ${GGUF}" "$WORKDIR/logs/gguf-print.out"
require "standing next step" "$WORKDIR/logs/gguf-print.out"
require "auto_apply=false" "$WORKDIR/logs/gguf-print.out"
require "did not run ollama create" "$WORKDIR/logs/gguf-print.out"
require "does not apply the estate" "$WORKDIR/logs/gguf-print.out"
require "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag ${TAG} --adapter ${GGUF}" "$WORKDIR/logs/gguf-print.out"
require "gguf-convert did not convert" "$WORKDIR/logs/gguf-print.out"
require "READY_FOR_LIVE_TEST: no" "$WORKDIR/logs/gguf-print.out"
if [[ -e "$GGUF" || -e "$MERGED/model.gguf" ]]; then
  echo "FAIL  gguf-convert wrote a GGUF" >&2
  exit 1
fi

echo "-- empty GGUF is refuse:seat (magic required) --"
: > "$GGUF"
set +e
estate enrich local-seat \
  --prepared "$PREPARED" \
  --weights "$GGUF" \
  >"$WORKDIR/logs/seat-empty.out" 2>"$WORKDIR/logs/seat-empty.err"
empty_rc=$?
set -e
if [[ "$empty_rc" -eq 0 ]]; then
  echo "FAIL  local-seat must refuse an empty GGUF" >&2
  exit 1
fi
require "refuse:seat" "$WORKDIR/logs/seat-empty.out" "$WORKDIR/logs/seat-empty.err"
if grep -q "ollama create" "$WORKDIR/logs/seat-empty.out" "$WORKDIR/logs/seat-empty.err"; then
  echo "FAIL  empty GGUF printed a create line" >&2
  exit 1
fi
rm -f "$GGUF"

# local-seat reads the first four bytes. An empty file is refuse:seat.
python3 - "$GGUF" <<'PY'
import pathlib, sys
pathlib.Path(sys.argv[1]).write_bytes(b"GGUF" + bytes(12))
PY

echo "-- GGUF passed as --adapter is refuse:adapter --"
set +e
estate enrich merge-adapt \
  --prepared "$PREPARED" \
  --adapter "$GGUF" \
  >"$WORKDIR/logs/merge-gguf.out" 2>"$WORKDIR/logs/merge-gguf.err"
merge_gguf_rc=$?
set -e
if [[ "$merge_gguf_rc" -eq 0 ]]; then
  echo "FAIL  merge-adapt must refuse a GGUF file" >&2
  exit 1
fi
require "refuse:adapter" "$WORKDIR/logs/merge-gguf.out" "$WORKDIR/logs/merge-gguf.err"
require "GGUF" "$WORKDIR/logs/merge-gguf.out" "$WORKDIR/logs/merge-gguf.err"

echo "-- local-seat prints ollama create for the GGUF stub --"
set +e
estate enrich local-seat \
  --prepared "$PREPARED" \
  --weights "$GGUF" \
  >"$WORKDIR/logs/seat-gguf.out" 2>"$WORKDIR/logs/seat-gguf.err"
local_rc=$?
set -e
if [[ "$local_rc" -ne 0 ]]; then
  echo "FAIL  local-seat did not print" >&2
  cat "$WORKDIR/logs/seat-gguf.out" "$WORKDIR/logs/seat-gguf.err" >&2
  exit 1
fi
require "shape=gguf" "$WORKDIR/logs/seat-gguf.out"
require "ollama create ${TAG} -f ${PREPARED}/Modelfile" "$WORKDIR/logs/seat-gguf.out"
require "llama-cli -m " "$WORKDIR/logs/seat-gguf.out"
require "llama-server -m " "$WORKDIR/logs/seat-gguf.out"
require "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag ${TAG} --adapter ${GGUF}" "$WORKDIR/logs/seat-gguf.out"
require "standing next step" "$WORKDIR/logs/seat-gguf.out"
require "auto_apply=false" "$WORKDIR/logs/seat-gguf.out"
require "did not run ollama create" "$WORKDIR/logs/seat-gguf.out"
require "does not apply the estate" "$WORKDIR/logs/seat-gguf.out"
require "trained_shape is gguf" "$WORKDIR/logs/seat-gguf.out"
require "local-seat did not create a model" "$WORKDIR/logs/seat-gguf.out"
require "READY_FOR_LIVE_TEST: no" "$WORKDIR/logs/seat-gguf.out"
require "local-seat is print-only" "$WORKDIR/logs/seat-gguf.out"
require "does not write ${PREPARED}/Modelfile" "$WORKDIR/logs/seat-gguf.out"
require "Write that file from the printed contents before ollama create" "$WORKDIR/logs/seat-gguf.out"
if [[ -e "$PREPARED/Modelfile" || -e "$MERGED/Modelfile" ]]; then
  echo "FAIL  local-seat wrote a Modelfile" >&2
  exit 1
fi

echo "-- import-trained records trained_shape gguf --"
set +e
estate enrich import-trained \
  --estate "$SEATED" \
  --prepared "$PREPARED" \
  --tag "$TAG" \
  --adapter "$GGUF" \
  --curator jason \
  >"$WORKDIR/logs/import.out" 2>"$WORKDIR/logs/import.err"
import_rc=$?
set -e
if [[ "$import_rc" -ne 0 ]]; then
  echo "FAIL  import-trained did not record the GGUF stub" >&2
  cat "$WORKDIR/logs/import.out" "$WORKDIR/logs/import.err" >&2
  exit 1
fi
require "shape=gguf" "$WORKDIR/logs/import.out"
require "import-trained did not apply" "$WORKDIR/logs/import.out"
require "promoted=false" "$WORKDIR/logs/import.out"
require "auto_apply=false" "$WORKDIR/logs/import.out"

python3 - "$PREPARED/prepare.json" "$PREPARED/binding-proposal.json" "$GGUF" <<'PY'
import json, sys
prepare_path, proposal_path, gguf = sys.argv[1:]
prepare = json.load(open(prepare_path))
proposal = json.load(open(proposal_path))
if prepare.get("trained_shape") != "gguf":
    raise SystemExit(f"FAIL  prepare trained_shape={prepare.get('trained_shape')}")
paths = prepare.get("trained_paths") or []
if gguf not in paths:
    raise SystemExit(f"FAIL  prepare trained_paths={paths}")
if prepare.get("promoted") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  import-trained rewrote promotion flags")
if prepare.get("auto_apply") is not False:
    raise SystemExit("FAIL  prepare auto_apply flipped")
if proposal.get("trained_shape") != "gguf":
    raise SystemExit(f"FAIL  proposal trained_shape={proposal.get('trained_shape')}")
proposal_paths = proposal.get("trained_paths") or []
if gguf not in proposal_paths:
    raise SystemExit(f"FAIL  proposal trained_paths={proposal_paths}")
if proposal.get("promoted") is not False or proposal.get("auto_apply") is not False:
    raise SystemExit("FAIL  proposal must stay unpromoted")
if proposal.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  proposal claims an estate rewrite")
magic = open(gguf, "rb").read(4)
if magic != b"GGUF":
    raise SystemExit("FAIL  GGUF stub lost its magic")
PY

finish

echo
echo "Printed lines (not executed):"
grep -h -E '^(model.save_pretrained_merged|python3 convert_hf_to_gguf.py |python llama.cpp/convert_hf_to_gguf.py |ollama create |llama-cli -m |llama-server -m |estate enrich import-trained )' "$WORKDIR/logs/merge.out" "$WORKDIR/logs/gguf-print.out" "$WORKDIR/logs/seat-gguf.out"
echo
echo "PASS  unsloth-lora-journey (Unsloth LoRA handoff checked; merge, convert, seat, and import printed; refuse:adapter; refuse:seat; refuse:tokenizer; SKIP live train; SKIP live convert; SKIP live seat)"
echo "READY_FOR_LIVE_TEST: no"
