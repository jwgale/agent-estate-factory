#!/usr/bin/env bash
# Target C train step: print the live QLoRA train recipe from NEXT.md.
# Prepares llamafactory-qlora on a throwaway seated estate, then prints the
# install lines and llamafactory-cli train line that prepare wrote.
# Does not install LLaMA-Factory, does not train, does not merge, does not
# convert, does not create an Ollama model, and does not promote.
# CELL_TRAIN_LIVE=1 does not start a live train. Live train stays on the
# operator host.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

# Throwaway dir stays out of the checkout. prepare refuses a hardware SKU
# anywhere in the output path, including a parent directory name.
WORKDIR="${WORKDIR:-${TMPDIR:-/tmp}/cell-one-train-next}"
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
  echo "SKIP train-next (example pack missing; not a PASS)"
  exit 0
fi

echo "== train-next (Target C: Qwen / LLaMA-Factory QLoRA train recipe; print-only) =="
echo "workdir: $WORKDIR"
echo "Print-only. READY_FOR_LIVE_TEST: no"
if [[ "${CELL_TRAIN_LIVE:-}" == "1" ]]; then
  echo "CELL_TRAIN_LIVE=1 is set. This journey stays print-only."
  echo "Live train stays on the operator host (docs/operator-enrich-journeys.md section 8)."
fi
echo "SKIP live train"
echo
echo "Train step (existing prepare card only):"
echo "1. estate enrich prepare --driver llamafactory-qlora"
echo "   Seat tag stays the Ollama id. Train base is a Hugging Face repo id."
echo "   Example seat ${SEAT_TAG}. Train base ${TRAIN_BASE}."
echo "2. Print the NEXT.md train recipe. This factory does not run it."
echo "   make uniqueness-ladder stays qlora-journey then seat-journey."
echo "   This target is the opt-in middle step. The chain does not run it."
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
  --driver llamafactory-qlora \
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

echo "-- llamafactory-qlora prepare writes the Qwen QLoRA card --"
PREPARED="$WORKDIR/llamafactory-qlora"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver llamafactory-qlora \
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
  echo "FAIL  prepare must not invent a trainer, a converter, or weights"
  exit 1
fi

grep -q "stage: sft" "$PREPARED/recipe.yaml"
grep -q "quantization_bit: 4" "$PREPARED/recipe.yaml"
grep -q "quantization_method: bnb" "$PREPARED/recipe.yaml"
grep -q "lora_rank: 16" "$PREPARED/recipe.yaml"
grep -q "template: qwen" "$PREPARED/recipe.yaml"
grep -q "model_name_or_path: \"${TRAIN_BASE}\"" "$PREPARED/recipe.yaml"
if grep -q "llamafactory-cli" "$PREPARED/prepare.json"; then
  echo "FAIL  prepare.json must not embed a train command"
  exit 1
fi

grep -q "Seat tag: ${SEAT_TAG}" "$PREPARED/PREPARE.md"
grep -q "Train base: ${TRAIN_BASE}" "$PREPARED/PREPARE.md"

if ! grep -q -F -x "pip install llamafactory" "$PREPARED/NEXT.md"; then
  echo "FAIL  NEXT.md missing train recipe line: pip install llamafactory"
  exit 1
fi
if ! grep -q -F -x "pip install 'bitsandbytes>=0.49'" "$PREPARED/NEXT.md"; then
  echo "FAIL  NEXT.md missing train recipe line: pip install 'bitsandbytes>=0.49'"
  exit 1
fi
if ! grep -q -F -x "llamafactory-cli train ${PREPARED}/recipe.yaml" "$PREPARED/NEXT.md"; then
  echo "FAIL  NEXT.md missing train recipe line: llamafactory-cli train ${PREPARED}/recipe.yaml"
  exit 1
fi
if ! grep -q -F -x "llamafactory-cli export ${PREPARED}/export.yaml" "$PREPARED/NEXT.md"; then
  echo "FAIL  NEXT.md missing the later export line"
  exit 1
fi
if ! grep -q -F -x "READY_FOR_LIVE_TEST: no" "$PREPARED/NEXT.md"; then
  echo "FAIL  NEXT.md must keep READY_FOR_LIVE_TEST no"
  exit 1
fi

python3 - "$PREPARED/prepare.json" "$TRAIN_BASE" "$SEAT_TAG" <<'PY'
import json, sys
prepare_path, train_base, seat = sys.argv[1:]
prepare = json.load(open(prepare_path))
if prepare.get("schema") != "cell-one.enrich-prepare.v0":
    raise SystemExit(f"FAIL  schema={prepare.get('schema')}")
if prepare.get("driver") != "llamafactory-qlora":
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
if "trained_shape" in prepare or "trained_paths" in prepare:
    raise SystemExit("FAIL  prepare must not record a trained shape")
PY

echo
echo "Live train recipe from NEXT.md (not executed):"
grep -F -x -e "pip install llamafactory" -e "pip install 'bitsandbytes>=0.49'" -e "llamafactory-cli train ${PREPARED}/recipe.yaml" "$PREPARED/NEXT.md"
echo
echo "Later merge line in NEXT.md (not executed):"
grep -F -x -e "llamafactory-cli export ${PREPARED}/export.yaml" "$PREPARED/NEXT.md"
echo
echo "This target does not run the train line or the export line."

echo "-- host notes (informational; this target does not install or train) --"
if ! command -v llamafactory-cli >/dev/null 2>&1; then
  echo "SKIP  llamafactory-cli (not on PATH; informational)"
else
  echo "NOTE  llamafactory-cli is on PATH. This target does not run it."
fi
if python3 -c 'import importlib.util, sys; sys.exit(0 if importlib.util.find_spec("bitsandbytes") else 1)'; then
  echo "NOTE  bitsandbytes is importable. This target does not train."
else
  echo "SKIP  bitsandbytes (not importable; informational)"
fi

AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  train-next rewrote examples/estate.yaml"
  exit 1
fi

echo
echo "PASS  train-next (Target C train recipe printed from NEXT.md; SKIP live train)"
echo "READY_FOR_LIVE_TEST: no"
