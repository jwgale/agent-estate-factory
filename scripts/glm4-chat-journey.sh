#!/usr/bin/env bash
# GLM-4 Chat family ladder on the LLaMA-Factory cards already
# on tip. QLoRA is llamafactory-qlora. The LoRA twin is llamafactory-lora.
# Seat tag stays llama3. Train base is the fixture Hugging Face id
# zai-org/glm-4-9b-chat. Template is glm4.
# Phase prepare stops after the prepare asserts and the missing-path refuses.
# Phase seat (and the default phase all) then prints merge-adapt, gguf-convert,
# local-seat, and import-trained against fixture stubs.
# Does not install LLaMA-Factory, does not train, does not merge, does not
# run convert_hf_to_gguf.py, does not run ollama create, and does not promote.
# CELL_TRAIN_LIVE=1 and CELL_SEAT_LIVE=1 stay print-only.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
# Resolves estate fail-closed: executable ESTATE_BIN, then target/release/estate,
# then target/debug/estate, then cargo on PATH. Does not invent a binary.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

GLM_CARD="${GLM_CARD:-llamafactory-qlora}"
case "$GLM_CARD" in
  llamafactory-qlora|llamafactory-lora) ;;
  *)
    echo "FAIL  GLM_CARD must be llamafactory-qlora or llamafactory-lora: $GLM_CARD" >&2
    exit 1
    ;;
esac

PHASE="${GLM4_CHAT_PHASE:-all}"
case "$PHASE" in
  prepare|seat|all) ;;
  *)
    echo "FAIL  GLM4_CHAT_PHASE must be prepare, seat, or all (got: $PHASE)" >&2
    exit 1
    ;;
esac

TRAIN_BASE="zai-org/glm-4-9b-chat"
SEAT_TAG="llama3"
TEMPLATE="glm4"
QLORA_NOTE="Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, Qwen3 Instruct, and DeepSeek-R1-Distill chat QLoRA."
LORA_NOTE="Reproduce target on the unquantized LoRA card, the non-quant twin of the GLM-4 Chat QLoRA prepare."

if [[ "$GLM_CARD" == "llamafactory-lora" ]]; then
  PACK_REL="examples/fixtures/glm4-chat-lora.pack.json"
  PACK_ID="glm4-chat-lora"
  DEFAULT_WORKDIR="${TMPDIR:-/tmp}/cell-one-glm4-chat-lora-journey"
  BANNER="GLM-4 Chat / LLaMA-Factory LoRA"
else
  PACK_REL="examples/fixtures/glm4-chat.pack.json"
  PACK_ID="glm4-chat"
  DEFAULT_WORKDIR="${TMPDIR:-/tmp}/cell-one-glm4-chat-journey"
  BANNER="GLM-4 Chat / LLaMA-Factory QLoRA"
fi

WORKDIR="${WORKDIR:-$DEFAULT_WORKDIR}"
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
PACK="${PACK:-$ROOT/$PACK_REL}"
BIN="${ESTATE_BIN:-}"
TAG="cell-enrich-${PACK_ID}"

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

if [[ ! -f "$PACK" ]]; then
  echo "SKIP glm4-chat-journey (fixture pack missing; not a PASS)"
  exit 0
fi

echo "== glm4-chat-journey (${BANNER}; print-only; phase=${PHASE}) =="
echo "workdir: $WORKDIR"
echo "card: $GLM_CARD"
echo "fixture: $PACK_REL"
echo "READY_FOR_LIVE_TEST: no"
if [[ "${CELL_TRAIN_LIVE:-}" == "1" || "${CELL_SEAT_LIVE:-}" == "1" ]]; then
  echo "CELL_TRAIN_LIVE or CELL_SEAT_LIVE is set. This journey stays print-only."
  echo "Live train, merge, convert, and seat stay on the operator host."
fi
echo "SKIP live train"
echo "SKIP live convert"
echo "SKIP live seat"
echo
echo "Ladder (existing commands only; fixture stubs, not weights):"
echo "1. estate enrich prepare --driver ${GLM_CARD}"
echo "   Seat tag stays the Ollama id. Train base is a Hugging Face repo id."
echo "   Example seat ${SEAT_TAG}. Train base ${TRAIN_BASE}. template ${TEMPLATE}."
echo "   An Ollama tag glm4:9b is a seat tag, not this train base."
echo "2. llamafactory-cli train <prepared>/recipe.yaml"
echo "   Those lines are in NEXT.md. This factory does not run them."
echo "3. estate enrich merge-adapt --prepared <prepared> --adapter <prepared>/outputs"
echo "   Prints llamafactory-cli export <prepared>/export.yaml."
echo "   The yaml shape is examples/merge_lora/qwen3_lora_sft.yaml."
echo "   The adapter stub is adapter_config.json. This factory does not merge."
echo "4. estate enrich gguf-convert --prepared <prepared> --weights <prepared>/export"
echo "   Before the good stub, a 5090-shaped export is refuse:tokenizer."
echo "   Prints python3 convert_hf_to_gguf.py ... --outfile <prepared>/export.gguf --outtype auto"
echo "5. estate enrich local-seat --prepared <prepared> --weights <prepared>/export.gguf"
echo "   local-seat is print-only. It prints the Modelfile and does not write \$PREPARED/Modelfile."
echo "   Prints ollama create. This factory does not run it."
echo "6. estate enrich import-trained --adapter <prepared>/export.gguf"
echo "   Records trained_shape gguf. The proposal stays auto_apply=false."
echo

resolve_estate

BEFORE="$(cksum "$ESTATE")"
PACK_BEFORE="$(cksum "$PACK")"
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR/logs"
SEATED_ONLY="$WORKDIR/estate-seat-only.yaml"
SEATED="$WORKDIR/estate.yaml"
if [[ "$SEATED" == "$ESTATE" ]]; then
  echo "FAIL  throwaway estate must not be examples/estate.yaml" >&2
  exit 1
fi
python3 - "$ESTATE" "$SEATED_ONLY" "$SEATED" <<'PY'
import sys
src, seat_only, seated = sys.argv[1:]
text = open(src, encoding="utf-8").read()
needle = "  - id: local_slm\n    class: local\n    driver: ollama\n    params:\n"
if needle not in text:
    raise SystemExit("FAIL  local_slm params block missing")
# Seat tag only. The fixture pack train_base_model is the train base.
open(seat_only, "w", encoding="utf-8").write(text.replace(needle, needle + '      model: "llama3"\n', 1))
open(seated, "w", encoding="utf-8").write(text.replace(needle, needle + '      model: "llama3"\n', 1))
PY

echo "-- seat tag without a train base refuses --"
set +e
estate enrich prepare \
  --estate "$SEATED_ONLY" \
  --pack "$ROOT/examples/fixtures/specialist-overnight.pack.json" \
  --driver "$GLM_CARD" \
  --job train \
  --out "$WORKDIR/seat-only" \
  >"$WORKDIR/logs/seat-only.out" 2>"$WORKDIR/logs/seat-only.err"
seat_rc=$?
set -e
if [[ "$seat_rc" -eq 0 ]]; then
  echo "FAIL  seat tag without a train base must refuse:train-base" >&2
  exit 1
fi
if ! grep -q "refuse:train-base" "$WORKDIR/logs/seat-only.out" "$WORKDIR/logs/seat-only.err"; then
  echo "FAIL  seat tag without a train base did not refuse:train-base" >&2
  cat "$WORKDIR/logs/seat-only.out" "$WORKDIR/logs/seat-only.err" >&2
  exit 1
fi
if [[ -e "$WORKDIR/seat-only" ]]; then
  echo "FAIL  seat-only prepare wrote an output directory" >&2
  exit 1
fi

echo "-- ${GLM_CARD} prepare writes the GLM-4 Chat card --"
PREPARED="$WORKDIR/$GLM_CARD"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver "$GLM_CARD" \
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
if [[ -e "$PREPARED/train_unsloth.py" || -e "$PREPARED/convert_hf_to_gguf.py" || -e "$PREPARED/outputs" || -e "$PREPARED/export" || -e "$PREPARED/export.gguf" ]]; then
  echo "FAIL  prepare must not invent a trainer, a converter, or weights" >&2
  exit 1
fi

grep -q "stage: sft" "$PREPARED/recipe.yaml"
grep -q "^template: ${TEMPLATE}$" "$PREPARED/recipe.yaml"
grep -q "^template: ${TEMPLATE}$" "$PREPARED/export.yaml"
grep -q "model_name_or_path: \"${TRAIN_BASE}\"" "$PREPARED/recipe.yaml"
grep -q "model_name_or_path: \"${TRAIN_BASE}\"" "$PREPARED/export.yaml"
grep -q "adapter_name_or_path: \"${PREPARED}/outputs\"" "$PREPARED/export.yaml"
grep -q "export_dir: \"${PREPARED}/export\"" "$PREPARED/export.yaml"
if grep -q "^template: qwen$" "$PREPARED/recipe.yaml" "$PREPARED/export.yaml"; then
  echo "FAIL  GLM-4 Chat must not take template qwen" >&2
  exit 1
fi
if grep -q "^template: default$" "$PREPARED/recipe.yaml" "$PREPARED/export.yaml"; then
  echo "FAIL  GLM-4 Chat must not take template default" >&2
  exit 1
fi
if grep -q "^template: deepseekr1$" "$PREPARED/recipe.yaml" "$PREPARED/export.yaml"; then
  echo "FAIL  GLM-4 Chat must not take template deepseekr1" >&2
  exit 1
fi
if [[ "$GLM_CARD" == "llamafactory-lora" ]]; then
  grep -q "finetuning_type: lora" "$PREPARED/recipe.yaml"
  grep -q "^lora_rank: 8$" "$PREPARED/recipe.yaml"
  grep -q "^packing: false$" "$PREPARED/recipe.yaml"
  if grep -q "quantization_bit" "$PREPARED/recipe.yaml" "$PREPARED/export.yaml"; then
    echo "FAIL  llamafactory-lora recipe and export must omit quantization_bit" >&2
    exit 1
  fi
  if grep -q "quantization_method" "$PREPARED/recipe.yaml" "$PREPARED/export.yaml"; then
    echo "FAIL  llamafactory-lora recipe and export must omit quantization_method" >&2
    exit 1
  fi
  if grep -q "bitsandbytes>=0.49" "$PREPARED/NEXT.md"; then
    echo "FAIL  llamafactory-lora NEXT.md must not install bitsandbytes" >&2
    exit 1
  fi
  grep -q "does not require bitsandbytes" "$PREPARED/NEXT.md"
  grep -F -q "$LORA_NOTE" "$PREPARED/NEXT.md"
  grep -F -q "$LORA_NOTE" "$PREPARED/PREPARE.md"
  if grep -F -q "$QLORA_NOTE" "$PREPARED/NEXT.md" "$PREPARED/PREPARE.md"; then
    echo "FAIL  GLM-4 Chat LoRA prepare took the QLoRA reproduce note" >&2
    exit 1
  fi
else
  grep -q "quantization_bit: 4" "$PREPARED/recipe.yaml"
  grep -q "quantization_method: bnb" "$PREPARED/recipe.yaml"
  grep -q "^lora_rank: 16$" "$PREPARED/recipe.yaml"
  grep -q "^packing: true$" "$PREPARED/recipe.yaml"
  if grep -q "quantization_bit" "$PREPARED/export.yaml"; then
    echo "FAIL  export.yaml must not set quantization_bit" >&2
    exit 1
  fi
  grep -q "bitsandbytes>=0.49" "$PREPARED/NEXT.md"
  grep -F -q "$QLORA_NOTE" "$PREPARED/NEXT.md"
  grep -F -q "$QLORA_NOTE" "$PREPARED/PREPARE.md"
  if grep -F -q "$LORA_NOTE" "$PREPARED/NEXT.md" "$PREPARED/PREPARE.md"; then
    echo "FAIL  GLM-4 Chat QLoRA prepare took the LoRA reproduce note" >&2
    exit 1
  fi
fi
if grep -q "llamafactory-cli" "$PREPARED/prepare.json"; then
  echo "FAIL  prepare.json must not embed a train command" >&2
  exit 1
fi

grep -q "Seat tag: ${SEAT_TAG}" "$PREPARED/PREPARE.md"
grep -q "Train base: ${TRAIN_BASE}" "$PREPARED/PREPARE.md"
grep -q "Dataset mode: scaffold" "$PREPARED/PREPARE.md"
grep -q "not training data" "$PREPARED/NEXT.md"
grep -q "llamafactory-cli train ${PREPARED}/recipe.yaml" "$PREPARED/NEXT.md"
grep -q "llamafactory-cli export ${PREPARED}/export.yaml" "$PREPARED/NEXT.md"
grep -q "estate enrich merge-adapt --prepared ${PREPARED} --adapter ${PREPARED}/outputs" "$PREPARED/NEXT.md"
grep -q "examples/merge_lora/qwen3_lora_sft.yaml" "$PREPARED/NEXT.md"
grep -q "estate enrich gguf-convert --prepared ${PREPARED} --weights ${PREPARED}/export" "$PREPARED/NEXT.md"
grep -q "python3 convert_hf_to_gguf.py ${PREPARED}/export --outfile ${PREPARED}/export.gguf --outtype auto" "$PREPARED/NEXT.md"
grep -q "estate enrich local-seat --prepared ${PREPARED} --weights ${PREPARED}/export" "$PREPARED/NEXT.md"
grep -q "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag ${TAG} --adapter <gguf>" "$PREPARED/NEXT.md"
grep -q "trained_shape" "$PREPARED/NEXT.md"
grep -q "READY_FOR_LIVE_TEST: no" "$PREPARED/NEXT.md"
grep -q "local-seat is print-only" "$PREPARED/NEXT.md"
grep -q "does not write ${PREPARED}/Modelfile" "$PREPARED/NEXT.md"

python3 - "$PREPARED/prepare.json" "$TRAIN_BASE" "$SEAT_TAG" "$GLM_CARD" "$PACK_ID" <<'PY'
import json, sys
prepare_path, train_base, seat, card, pack_id = sys.argv[1:]
prepare = json.load(open(prepare_path))
if prepare.get("schema") != "cell-one.enrich-prepare.v0":
    raise SystemExit(f"FAIL  schema={prepare.get('schema')}")
if prepare.get("driver") != card:
    raise SystemExit(f"FAIL  driver={prepare.get('driver')}")
if prepare.get("job") != "train":
    raise SystemExit(f"FAIL  job={prepare.get('job')}")
if prepare.get("pack_id") != pack_id:
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
if prepare.get("dataset_mode") != "scaffold":
    raise SystemExit(f"FAIL  dataset_mode={prepare.get('dataset_mode')}")
if "trained_shape" in prepare or "trained_paths" in prepare:
    raise SystemExit("FAIL  prepare must not record a trained shape before import-trained")
PY

SEATED_BEFORE="$(cksum "$SEATED")"

echo "-- missing adapter, merge, and GGUF refuse (no convert, no seat) --"
set +e
estate enrich merge-adapt \
  --prepared "$PREPARED" \
  --adapter "$PREPARED/outputs" \
  >"$WORKDIR/logs/merge-missing.out" 2>"$WORKDIR/logs/merge-missing.err"
merge_missing_rc=$?
set -e
if [[ "$merge_missing_rc" -eq 0 ]]; then
  echo "FAIL  merge-adapt must refuse before an adapter directory exists" >&2
  exit 1
fi
if ! grep -q "refuse:adapter" "$WORKDIR/logs/merge-missing.out" "$WORKDIR/logs/merge-missing.err"; then
  echo "FAIL  merge-adapt did not refuse:adapter" >&2
  cat "$WORKDIR/logs/merge-missing.out" "$WORKDIR/logs/merge-missing.err" >&2
  exit 1
fi
if [[ -e "$PREPARED/outputs" || -e "$PREPARED/export" ]]; then
  echo "FAIL  merge-adapt wrote an adapter directory or an export directory" >&2
  exit 1
fi

set +e
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$PREPARED/export" \
  >"$WORKDIR/logs/gguf-missing.out" 2>"$WORKDIR/logs/gguf-missing.err"
gguf_missing_rc=$?
set -e
if [[ "$gguf_missing_rc" -eq 0 ]]; then
  echo "FAIL  gguf-convert must refuse before a merged export exists" >&2
  exit 1
fi
if ! grep -q "refuse:seat" "$WORKDIR/logs/gguf-missing.out" "$WORKDIR/logs/gguf-missing.err"; then
  echo "FAIL  gguf-convert did not refuse:seat" >&2
  exit 1
fi
if [[ -e "$PREPARED/export.gguf" || -e "$PREPARED/export" ]]; then
  echo "FAIL  gguf-convert wrote a GGUF or an export directory" >&2
  exit 1
fi

set +e
estate enrich local-seat \
  --prepared "$PREPARED" \
  --weights "$PREPARED/export.gguf" \
  >"$WORKDIR/logs/seat-missing.out" 2>"$WORKDIR/logs/seat-missing.err"
local_missing_rc=$?
set -e
if [[ "$local_missing_rc" -eq 0 ]]; then
  echo "FAIL  local-seat must refuse before a GGUF exists" >&2
  exit 1
fi
if ! grep -q "refuse:seat" "$WORKDIR/logs/seat-missing.out" "$WORKDIR/logs/seat-missing.err"; then
  echo "FAIL  local-seat did not refuse:seat" >&2
  exit 1
fi
if [[ -e "$PREPARED/export.gguf" ]]; then
  echo "FAIL  local-seat wrote a GGUF" >&2
  exit 1
fi

finish_locked() {
  local after seated_after pack_after
  after="$(cksum "$ESTATE")"
  if [[ "$BEFORE" != "$after" ]]; then
    echo "FAIL  glm4-chat-journey rewrote examples/estate.yaml" >&2
    exit 1
  fi
  if [[ -f "$SEATED" ]]; then
    seated_after="$(cksum "$SEATED")"
    if [[ "$SEATED_BEFORE" != "$seated_after" ]]; then
      echo "FAIL  import-trained rewrote the lab estate copy" >&2
      exit 1
    fi
  fi
  pack_after="$(cksum "$PACK")"
  if [[ "$PACK_BEFORE" != "$pack_after" ]]; then
    echo "FAIL  glm4-chat-journey rewrote the fixture pack" >&2
    exit 1
  fi
}

if [[ "$PHASE" == "prepare" ]]; then
  finish_locked
  echo
  echo "PASS  glm4-chat-journey (prepare-assert; template ${TEMPLATE}; fixture ${PACK_ID}; SKIP live train; SKIP live convert; SKIP live seat)"
  echo "READY_FOR_LIVE_TEST: no"
  exit 0
fi

echo "-- fixture stub: adapter dir --"
mkdir -p "$PREPARED/outputs"
printf '%s\n' '{}' > "$PREPARED/outputs/adapter_config.json"

echo "-- merge-adapt prints llamafactory-cli export --"
set +e
estate enrich merge-adapt \
  --prepared "$PREPARED" \
  --adapter "$PREPARED/outputs" \
  >"$WORKDIR/logs/merge.out" 2>"$WORKDIR/logs/merge.err"
merge_rc=$?
set -e
if [[ "$merge_rc" -ne 0 ]]; then
  echo "FAIL  merge-adapt did not print" >&2
  cat "$WORKDIR/logs/merge.out" "$WORKDIR/logs/merge.err" >&2
  exit 1
fi
grep -q "llamafactory-cli export ${PREPARED}/export.yaml" "$WORKDIR/logs/merge.out"
grep -q "merge-adapt did not merge" "$WORKDIR/logs/merge.out"
grep -q "READY_FOR_LIVE_TEST: no" "$WORKDIR/logs/merge.out"
if [[ -e "$PREPARED/export" || -e "$PREPARED/export.gguf" ]]; then
  echo "FAIL  merge-adapt wrote an export directory or a GGUF" >&2
  exit 1
fi

echo "-- 5090-shaped export tokenizer is refuse:tokenizer --"
mkdir -p "$PREPARED/export"
printf '%s\n' '{"model_type":"qwen2","architectures":["Qwen2ForCausalLM"]}' > "$PREPARED/export/config.json"
printf '%s\n' 'not-a-real-tensor' > "$PREPARED/export/model.safetensors"
printf '%s\n' '{"extra_special_tokens":["<|im_start|>","<|im_end|>"],"tokenizer_class":"Qwen2Tokenizer"}' > "$PREPARED/export/tokenizer_config.json"

set +e
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$PREPARED/export" \
  >"$WORKDIR/logs/gguf-tokenizer.out" 2>"$WORKDIR/logs/gguf-tokenizer.err"
tokenizer_rc=$?
set -e
if [[ "$tokenizer_rc" -eq 0 ]]; then
  echo "FAIL  gguf-convert must refuse:tokenizer on the 5090-shaped export" >&2
  exit 1
fi
if ! grep -q "refuse:tokenizer" "$WORKDIR/logs/gguf-tokenizer.out" "$WORKDIR/logs/gguf-tokenizer.err"; then
  echo "FAIL  5090-shaped export did not refuse:tokenizer" >&2
  cat "$WORKDIR/logs/gguf-tokenizer.out" "$WORKDIR/logs/gguf-tokenizer.err" >&2
  exit 1
fi
if grep -q "python3 convert_hf_to_gguf.py" "$WORKDIR/logs/gguf-tokenizer.out" "$WORKDIR/logs/gguf-tokenizer.err"; then
  echo "FAIL  refuse:tokenizer printed the convert line" >&2
  exit 1
fi
if [[ -e "$PREPARED/export.gguf" ]]; then
  echo "FAIL  refuse:tokenizer wrote a GGUF" >&2
  exit 1
fi

echo "-- replace the broken tokenizer with the good merged stub --"
rm -rf "$PREPARED/export"
mkdir -p "$PREPARED/export"
printf '%s\n' '{}' > "$PREPARED/export/config.json"
printf '%s\n' 'not-a-real-tensor' > "$PREPARED/export/model.safetensors"

echo "-- gguf-convert prints convert_hf_to_gguf.py --"
set +e
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$PREPARED/export" \
  >"$WORKDIR/logs/gguf-print.out" 2>"$WORKDIR/logs/gguf-print.err"
gguf_rc=$?
set -e
if [[ "$gguf_rc" -ne 0 ]]; then
  echo "FAIL  gguf-convert did not print" >&2
  cat "$WORKDIR/logs/gguf-print.out" "$WORKDIR/logs/gguf-print.err" >&2
  exit 1
fi
grep -q "python3 convert_hf_to_gguf.py ${PREPARED}/export --outfile ${PREPARED}/export.gguf --outtype auto" "$WORKDIR/logs/gguf-print.out"
grep -q "gguf-convert did not convert" "$WORKDIR/logs/gguf-print.out"
grep -q "READY_FOR_LIVE_TEST: no" "$WORKDIR/logs/gguf-print.out"
if [[ -e "$PREPARED/export.gguf" ]]; then
  echo "FAIL  gguf-convert wrote a GGUF" >&2
  exit 1
fi

python3 - "$PREPARED/export.gguf" <<'PY'
import pathlib, sys
pathlib.Path(sys.argv[1]).write_bytes(b"GGUF" + bytes(12))
PY

echo "-- local-seat prints ollama create for the GGUF stub --"
set +e
estate enrich local-seat \
  --prepared "$PREPARED" \
  --weights "$PREPARED/export.gguf" \
  >"$WORKDIR/logs/seat-gguf.out" 2>"$WORKDIR/logs/seat-gguf.err"
local_rc=$?
set -e
if [[ "$local_rc" -ne 0 ]]; then
  echo "FAIL  local-seat did not print" >&2
  cat "$WORKDIR/logs/seat-gguf.out" "$WORKDIR/logs/seat-gguf.err" >&2
  exit 1
fi
grep -q "shape=gguf" "$WORKDIR/logs/seat-gguf.out"
grep -q "ollama create ${TAG} -f ${PREPARED}/Modelfile" "$WORKDIR/logs/seat-gguf.out"
grep -q "local-seat did not create a model" "$WORKDIR/logs/seat-gguf.out"
grep -q "READY_FOR_LIVE_TEST: no" "$WORKDIR/logs/seat-gguf.out"
grep -q "does not write ${PREPARED}/Modelfile" "$WORKDIR/logs/seat-gguf.out"
if [[ -e "$PREPARED/Modelfile" ]]; then
  echo "FAIL  local-seat wrote a Modelfile" >&2
  exit 1
fi

echo "-- import-trained records trained_shape gguf --"
set +e
estate enrich import-trained \
  --estate "$SEATED" \
  --prepared "$PREPARED" \
  --tag "$TAG" \
  --adapter "$PREPARED/export.gguf" \
  --curator jason \
  >"$WORKDIR/logs/import.out" 2>"$WORKDIR/logs/import.err"
import_rc=$?
set -e
if [[ "$import_rc" -ne 0 ]]; then
  echo "FAIL  import-trained did not record the GGUF stub" >&2
  cat "$WORKDIR/logs/import.out" "$WORKDIR/logs/import.err" >&2
  exit 1
fi
grep -q "shape=gguf" "$WORKDIR/logs/import.out"
grep -q "auto_apply=false" "$WORKDIR/logs/import.out"
grep -q "import-trained did not apply" "$WORKDIR/logs/import.out"

python3 - "$PREPARED/prepare.json" "$PREPARED/binding-proposal.json" "$PREPARED/export.gguf" <<'PY'
import json, sys
prepare_path, proposal_path, gguf = sys.argv[1:]
prepare = json.load(open(prepare_path))
proposal = json.load(open(proposal_path))
if prepare.get("trained_shape") != "gguf":
    raise SystemExit(f"FAIL  prepare trained_shape={prepare.get('trained_shape')}")
if prepare.get("promoted") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  import-trained rewrote promotion flags")
if proposal.get("trained_shape") != "gguf":
    raise SystemExit(f"FAIL  proposal trained_shape={proposal.get('trained_shape')}")
if proposal.get("promoted") is not False or proposal.get("auto_apply") is not False:
    raise SystemExit("FAIL  proposal must stay unpromoted")
if proposal.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  proposal claims an estate rewrite")
magic = open(gguf, "rb").read(4)
if magic != b"GGUF":
    raise SystemExit("FAIL  GGUF stub lost its magic")
PY

finish_locked

echo
echo "Printed lines (not executed):"
grep -h -E '^(llamafactory-cli export |python3 convert_hf_to_gguf.py |ollama create |estate enrich import-trained )' "$WORKDIR/logs/merge.out" "$WORKDIR/logs/gguf-print.out" "$WORKDIR/logs/seat-gguf.out" || true
echo
echo "PASS  glm4-chat-journey (${BANNER} ladder printed; template ${TEMPLATE}; fixture stubs; SKIP live train; SKIP live convert; SKIP live seat)"
echo "READY_FOR_LIVE_TEST: no"
echo "This print is not a live PASS."
echo "The recorded Target C PASS stays the only live uniqueness prove."
