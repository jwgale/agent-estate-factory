#!/usr/bin/env bash
# Target C seat ladder: Qwen / LLaMA-Factory QLoRA print path.
# Prepares llamafactory-qlora, asserts refuse:tokenizer on a 5090-shaped
# export fixture, then prints merge-adapt, gguf-convert, local-seat, and
# import-trained once the good fixture stubs stand in for the merged
# export and the sibling GGUF.
# Does not install LLaMA-Factory, does not train, does not merge, does not
# run convert_hf_to_gguf.py, does not run ollama create, and does not promote.
# CELL_SEAT_LIVE=1 does not start a live phase. Live convert and live seat
# stay on the operator host.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

# Throwaway dir stays out of the checkout. prepare refuses a hardware SKU
# anywhere in the output path, including a parent directory name.
WORKDIR="${WORKDIR:-${TMPDIR:-/tmp}/cell-one-seat-journey}"
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
PACK="${PACK:-$ROOT/examples/fixtures/specialist-overnight.pack.json}"
BIN="${ESTATE_BIN:-}"
TRAIN_BASE="Qwen/Qwen2.5-0.5B-Instruct"
SEAT_TAG="llama3"
TAG="cell-enrich-overnight-traces"

estate() {
  if [[ -n "$BIN" ]]; then
    "$BIN" "$@"
  else
    cargo run -q -p estate-control -- "$@"
  fi
}

if [[ ! -f "$PACK" ]]; then
  echo "SKIP seat-journey (example pack missing; not a PASS)"
  exit 0
fi

echo "== seat-journey (Target C: Qwen / LLaMA-Factory QLoRA seat ladder; print-only) =="
echo "workdir: $WORKDIR"
echo "READY_FOR_LIVE_TEST: no"
if [[ "${CELL_SEAT_LIVE:-}" == "1" ]]; then
  echo "CELL_SEAT_LIVE=1 is set. This journey stays print-only."
  echo "Live train, merge, convert, and seat stay on the operator host (docs/operator-enrich-journeys.md section 10)."
fi
echo "SKIP live train"
echo "SKIP live convert"
echo "SKIP live seat"
echo
echo "Ladder (existing commands only; fixture stubs, not weights):"
echo "1. estate enrich prepare --driver llamafactory-qlora"
echo "   Seat tag stays the Ollama id. Train base is a Hugging Face repo id."
echo "   Example seat ${SEAT_TAG}. Train base ${TRAIN_BASE}."
echo "2. estate enrich merge-adapt --prepared <prepared> --adapter <prepared>/outputs"
echo "   Prints llamafactory-cli export <prepared>/export.yaml."
echo "   The yaml shape is examples/merge_lora/qwen3_lora_sft.yaml."
echo "   The adapter stub is adapter_config.json. This factory does not merge."
echo "3. estate enrich gguf-convert --prepared <prepared> --weights <prepared>/export"
echo "   Before the good stub, a 5090-shaped export is refuse:tokenizer."
echo "   config.json names Qwen. extra_special_tokens is a JSON list."
echo "   vocab.json and merges.txt are missing. The convert line does not print."
echo "   That refuse names restoring tokenizer files from the HF cache snapshot"
echo "   for the train base, or the equivalent base checkout, into the export directory,"
echo "   then re-running estate enrich gguf-convert. This script does not copy them."
echo "   The good merged stub is config.json ({}) plus model.safetensors."
echo "   That stub has no Qwen marker and no tokenizer_config.json."
echo "   Prints python3 convert_hf_to_gguf.py ... --outfile <prepared>/export.gguf --outtype auto"
echo "4. estate enrich local-seat --prepared <prepared> --weights <prepared>/export.gguf"
echo "   The GGUF stub starts with GGUF magic. Prints ollama create."
echo "   This factory does not run it."
echo "   After that print, the standing next step is import-trained for that GGUF."
echo "   The same step stands when ollama create already ran outside this factory."
echo "5. estate enrich import-trained --adapter <prepared>/export.gguf"
echo "   Records trained_shape gguf and trained_paths."
echo "   The proposal stays auto_apply=false. Does not apply the estate. Does not promote."
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
grep -q "template: qwen" "$PREPARED/recipe.yaml"
grep -q "model_name_or_path: \"${TRAIN_BASE}\"" "$PREPARED/recipe.yaml"
grep -q "model_name_or_path: \"${TRAIN_BASE}\"" "$PREPARED/export.yaml"
grep -q "adapter_name_or_path: \"${PREPARED}/outputs\"" "$PREPARED/export.yaml"
grep -q "export_dir: \"${PREPARED}/export\"" "$PREPARED/export.yaml"
if grep -q "quantization_bit" "$PREPARED/export.yaml"; then
  echo "FAIL  export.yaml must not set quantization_bit"
  exit 1
fi
if grep -q "llamafactory-cli" "$PREPARED/prepare.json"; then
  echo "FAIL  prepare.json must not embed a train command"
  exit 1
fi

grep -q "Seat tag: ${SEAT_TAG}" "$PREPARED/PREPARE.md"
grep -q "Train base: ${TRAIN_BASE}" "$PREPARED/PREPARE.md"
grep -q "llamafactory-cli export ${PREPARED}/export.yaml" "$PREPARED/NEXT.md"
grep -q "estate enrich merge-adapt --prepared ${PREPARED} --adapter ${PREPARED}/outputs" "$PREPARED/NEXT.md"
grep -q "estate enrich gguf-convert --prepared ${PREPARED} --weights ${PREPARED}/export" "$PREPARED/NEXT.md"
grep -q "python3 convert_hf_to_gguf.py ${PREPARED}/export --outfile ${PREPARED}/export.gguf --outtype auto" "$PREPARED/NEXT.md"
grep -q "estate enrich local-seat --prepared ${PREPARED} --weights ${PREPARED}/export" "$PREPARED/NEXT.md"
grep -q "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag ${TAG} --adapter <gguf>" "$PREPARED/NEXT.md"
grep -q "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag ${TAG} --adapter ${PREPARED}/export.gguf" "$PREPARED/NEXT.md"
grep -q "standing next step" "$PREPARED/NEXT.md"
grep -q "auto_apply=false" "$PREPARED/NEXT.md"
grep -q "did not run ollama create" "$PREPARED/NEXT.md"
grep -q "does not apply the estate" "$PREPARED/NEXT.md"
grep -q "READY_FOR_LIVE_TEST: no" "$PREPARED/NEXT.md"

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
if prepare.get("pack_id") != "overnight-traces":
    raise SystemExit(f"FAIL  pack_id={prepare.get('pack_id')}")
if prepare.get("base_model") != seat or prepare.get("seat_tag") != seat:
    raise SystemExit(f"FAIL  seat={prepare.get('base_model')}/{prepare.get('seat_tag')}")
if prepare.get("train_base_model") != train_base:
    raise SystemExit(f"FAIL  train_base_model={prepare.get('train_base_model')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False:
    raise SystemExit("FAIL  prepare.json must stay unpromoted")
if prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  prepare.json claims an estate rewrite")
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
  echo "FAIL  merge-adapt must refuse before an adapter directory exists"
  exit 1
fi
if ! grep -q "refuse:adapter" "$WORKDIR/logs/merge-missing.out" "$WORKDIR/logs/merge-missing.err"; then
  echo "FAIL  merge-adapt did not refuse:adapter"
  cat "$WORKDIR/logs/merge-missing.out" "$WORKDIR/logs/merge-missing.err"
  exit 1
fi
if [[ -e "$PREPARED/outputs" || -e "$PREPARED/export" ]]; then
  echo "FAIL  merge-adapt wrote an adapter directory or an export directory"
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
  echo "FAIL  gguf-convert must refuse before a merged export exists"
  exit 1
fi
if ! grep -q "refuse:seat" "$WORKDIR/logs/gguf-missing.out" "$WORKDIR/logs/gguf-missing.err"; then
  echo "FAIL  gguf-convert did not refuse:seat"
  cat "$WORKDIR/logs/gguf-missing.out" "$WORKDIR/logs/gguf-missing.err"
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
  >"$WORKDIR/logs/seat-missing.out" 2>"$WORKDIR/logs/seat-missing.err"
local_missing_rc=$?
set -e
if [[ "$local_missing_rc" -eq 0 ]]; then
  echo "FAIL  local-seat must refuse before a GGUF exists"
  exit 1
fi
if ! grep -q "refuse:seat" "$WORKDIR/logs/seat-missing.out" "$WORKDIR/logs/seat-missing.err"; then
  echo "FAIL  local-seat did not refuse:seat"
  cat "$WORKDIR/logs/seat-missing.out" "$WORKDIR/logs/seat-missing.err"
  exit 1
fi
if [[ -e "$PREPARED/export.gguf" ]]; then
  echo "FAIL  local-seat wrote a GGUF"
  exit 1
fi

echo "-- fixture stub: adapter dir --"
mkdir -p "$PREPARED/outputs"
printf '%s\n' '{}' > "$PREPARED/outputs/adapter_config.json"
if [[ -e "$PREPARED/outputs/adapter_model.safetensors" ]]; then
  echo "FAIL  adapter stub must stay a marker file"
  exit 1
fi

echo "-- merge-adapt prints llamafactory-cli export --"
set +e
estate enrich merge-adapt \
  --prepared "$PREPARED" \
  --adapter "$PREPARED/outputs" \
  >"$WORKDIR/logs/merge.out" 2>"$WORKDIR/logs/merge.err"
merge_rc=$?
set -e
if [[ "$merge_rc" -ne 0 ]]; then
  echo "FAIL  merge-adapt did not print"
  cat "$WORKDIR/logs/merge.out" "$WORKDIR/logs/merge.err"
  exit 1
fi
grep -q "llamafactory-cli export ${PREPARED}/export.yaml" "$WORKDIR/logs/merge.out"
grep -q "python3 convert_hf_to_gguf.py ${PREPARED}/export --outfile ${PREPARED}/export.gguf --outtype auto" "$WORKDIR/logs/merge.out"
grep -q "estate enrich gguf-convert --prepared ${PREPARED} --weights ${PREPARED}/export" "$WORKDIR/logs/merge.out"
grep -q "estate enrich local-seat --prepared ${PREPARED} --weights ${PREPARED}/export" "$WORKDIR/logs/merge.out"
grep -q "merge-adapt did not merge" "$WORKDIR/logs/merge.out"
grep -q "READY_FOR_LIVE_TEST: no" "$WORKDIR/logs/merge.out"
if [[ -e "$PREPARED/export" || -e "$PREPARED/export.gguf" ]]; then
  echo "FAIL  merge-adapt wrote an export directory or a GGUF"
  exit 1
fi

echo "-- 5090-shaped export tokenizer is refuse:tokenizer --"
mkdir -p "$PREPARED/export"
printf '%s\n' '{"model_type":"qwen2","architectures":["Qwen2ForCausalLM"]}' > "$PREPARED/export/config.json"
printf '%s\n' 'not-a-real-tensor' > "$PREPARED/export/model.safetensors"
printf '%s\n' '{"extra_special_tokens":["<|im_start|>","<|im_end|>"],"tokenizer_class":"Qwen2Tokenizer"}' > "$PREPARED/export/tokenizer_config.json"
if [[ -e "$PREPARED/export/vocab.json" || -e "$PREPARED/export/merges.txt" || -e "$PREPARED/export.gguf" || -e "$PREPARED/export/tokenizer_config.json.bak" ]]; then
  echo "FAIL  5090-shaped fixture must omit vocab.json, merges.txt, and a GGUF"
  exit 1
fi

set +e
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$PREPARED/export" \
  >"$WORKDIR/logs/gguf-tokenizer.out" 2>"$WORKDIR/logs/gguf-tokenizer.err"
tokenizer_rc=$?
set -e
if [[ "$tokenizer_rc" -eq 0 ]]; then
  echo "FAIL  gguf-convert must refuse:tokenizer on the 5090-shaped export"
  cat "$WORKDIR/logs/gguf-tokenizer.out" "$WORKDIR/logs/gguf-tokenizer.err"
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
  "re-run estate enrich gguf-convert" \
  "${TRAIN_BASE}"
do
  if ! grep -q "$needle" "$WORKDIR/logs/gguf-tokenizer.out" "$WORKDIR/logs/gguf-tokenizer.err"; then
    echo "FAIL  5090-shaped export did not report: $needle"
    cat "$WORKDIR/logs/gguf-tokenizer.out" "$WORKDIR/logs/gguf-tokenizer.err"
    exit 1
  fi
done
if grep -q "python3 convert_hf_to_gguf.py" "$WORKDIR/logs/gguf-tokenizer.out" "$WORKDIR/logs/gguf-tokenizer.err"; then
  echo "FAIL  refuse:tokenizer printed the convert line"
  exit 1
fi
if [[ -e "$PREPARED/export.gguf" || -e "$PREPARED/export/model.gguf" || -e "$PREPARED/export/tokenizer_config.json.bak" || -e "$PREPARED/export/vocab.json" || -e "$PREPARED/export/merges.txt" ]]; then
  echo "FAIL  refuse:tokenizer wrote a GGUF or copied tokenizer files"
  exit 1
fi
if ! grep -q '"extra_special_tokens"' "$PREPARED/export/tokenizer_config.json"; then
  echo "FAIL  refuse:tokenizer rewrote tokenizer_config.json"
  exit 1
fi

echo "-- replace the broken tokenizer with the good merged stub --"
rm -rf "$PREPARED/export"
mkdir -p "$PREPARED/export"
printf '%s\n' '{}' > "$PREPARED/export/config.json"
printf '%s\n' 'not-a-real-tensor' > "$PREPARED/export/model.safetensors"
if [[ -e "$PREPARED/export/tokenizer_config.json" || -e "$PREPARED/export/vocab.json" || -e "$PREPARED/export/merges.txt" ]]; then
  echo "FAIL  good merged stub must not keep the broken tokenizer"
  exit 1
fi
if ! grep -qx '{}' "$PREPARED/export/config.json"; then
  echo "FAIL  good merged stub config.json must stay {}"
  exit 1
fi
if [[ -e "$PREPARED/export/adapter_config.json" || -e "$PREPARED/export/adapter_model.safetensors" || -e "$PREPARED/export/model.gguf" ]]; then
  echo "FAIL  merged stub must be config.json plus model.safetensors"
  exit 1
fi

echo "-- gguf-convert prints convert_hf_to_gguf.py --"
set +e
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$PREPARED/export" \
  >"$WORKDIR/logs/gguf-print.out" 2>"$WORKDIR/logs/gguf-print.err"
gguf_rc=$?
set -e
if [[ "$gguf_rc" -ne 0 ]]; then
  echo "FAIL  gguf-convert did not print"
  cat "$WORKDIR/logs/gguf-print.out" "$WORKDIR/logs/gguf-print.err"
  exit 1
fi
grep -q "shape=merged" "$WORKDIR/logs/gguf-print.out"
grep -q "Tokenizer check passed" "$WORKDIR/logs/gguf-print.out"
grep -q "this directory has no tokenizer_config.json" "$WORKDIR/logs/gguf-print.out"
grep -q "python3 convert_hf_to_gguf.py ${PREPARED}/export --outfile ${PREPARED}/export.gguf --outtype auto" "$WORKDIR/logs/gguf-print.out"
grep -q "estate enrich local-seat --prepared ${PREPARED} --weights ${PREPARED}/export.gguf" "$WORKDIR/logs/gguf-print.out"
grep -q "standing next step" "$WORKDIR/logs/gguf-print.out"
grep -q "auto_apply=false" "$WORKDIR/logs/gguf-print.out"
grep -q "did not run ollama create" "$WORKDIR/logs/gguf-print.out"
grep -q "does not apply the estate" "$WORKDIR/logs/gguf-print.out"
grep -q "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag ${TAG} --adapter ${PREPARED}/export.gguf" "$WORKDIR/logs/gguf-print.out"
grep -q "gguf-convert did not convert" "$WORKDIR/logs/gguf-print.out"
grep -q "READY_FOR_LIVE_TEST: no" "$WORKDIR/logs/gguf-print.out"
if [[ -e "$PREPARED/export.gguf" || -e "$PREPARED/export/model.gguf" ]]; then
  echo "FAIL  gguf-convert wrote a GGUF"
  exit 1
fi

echo "-- empty GGUF is refuse:seat (magic required) --"
: > "$PREPARED/export.gguf"
set +e
estate enrich local-seat \
  --prepared "$PREPARED" \
  --weights "$PREPARED/export.gguf" \
  >"$WORKDIR/logs/seat-empty.out" 2>"$WORKDIR/logs/seat-empty.err"
empty_rc=$?
set -e
if [[ "$empty_rc" -eq 0 ]]; then
  echo "FAIL  local-seat must refuse an empty GGUF"
  exit 1
fi
if ! grep -q "refuse:seat" "$WORKDIR/logs/seat-empty.out" "$WORKDIR/logs/seat-empty.err"; then
  echo "FAIL  empty GGUF did not refuse:seat"
  cat "$WORKDIR/logs/seat-empty.out" "$WORKDIR/logs/seat-empty.err"
  exit 1
fi
if grep -q "ollama create" "$WORKDIR/logs/seat-empty.out" "$WORKDIR/logs/seat-empty.err"; then
  echo "FAIL  empty GGUF printed a create line"
  exit 1
fi
rm -f "$PREPARED/export.gguf"

# local-seat reads the first four bytes. An empty file is refuse:seat.
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
  echo "FAIL  local-seat did not print"
  cat "$WORKDIR/logs/seat-gguf.out" "$WORKDIR/logs/seat-gguf.err"
  exit 1
fi
grep -q "shape=gguf" "$WORKDIR/logs/seat-gguf.out"
grep -q "ollama create ${TAG} -f ${PREPARED}/Modelfile" "$WORKDIR/logs/seat-gguf.out"
grep -q "llama-cli -m " "$WORKDIR/logs/seat-gguf.out"
grep -q "llama-server -m " "$WORKDIR/logs/seat-gguf.out"
grep -q "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag ${TAG} --adapter ${PREPARED}/export.gguf" "$WORKDIR/logs/seat-gguf.out"
grep -q "standing next step" "$WORKDIR/logs/seat-gguf.out"
grep -q "auto_apply=false" "$WORKDIR/logs/seat-gguf.out"
grep -q "did not run ollama create" "$WORKDIR/logs/seat-gguf.out"
grep -q "does not apply the estate" "$WORKDIR/logs/seat-gguf.out"
grep -q "trained_shape is gguf" "$WORKDIR/logs/seat-gguf.out"
grep -q "local-seat did not create a model" "$WORKDIR/logs/seat-gguf.out"
grep -q "READY_FOR_LIVE_TEST: no" "$WORKDIR/logs/seat-gguf.out"
if [[ -e "$PREPARED/Modelfile" ]]; then
  echo "FAIL  local-seat wrote a Modelfile"
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
  echo "FAIL  import-trained did not record the GGUF stub"
  cat "$WORKDIR/logs/import.out" "$WORKDIR/logs/import.err"
  exit 1
fi
grep -q "shape=gguf" "$WORKDIR/logs/import.out"
grep -q "import-trained did not apply" "$WORKDIR/logs/import.out"
grep -q "promoted=false" "$WORKDIR/logs/import.out"
grep -q "auto_apply=false" "$WORKDIR/logs/import.out"

python3 - "$PREPARED/prepare.json" "$PREPARED/binding-proposal.json" "$PREPARED/export.gguf" <<'PY'
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

AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  seat-journey rewrote examples/estate.yaml"
  exit 1
fi
SEATED_AFTER="$(cksum "$SEATED")"
if [[ "$SEATED_BEFORE" != "$SEATED_AFTER" ]]; then
  echo "FAIL  import-trained rewrote the lab estate copy"
  exit 1
fi

echo
echo "Printed lines (not executed):"
grep -h -E '^(llamafactory-cli export |python3 convert_hf_to_gguf.py |ollama create |llama-cli -m |llama-server -m |estate enrich import-trained )' "$WORKDIR/logs/merge.out" "$WORKDIR/logs/gguf-print.out" "$WORKDIR/logs/seat-gguf.out"
echo
echo "PASS  seat-journey (Target C seat ladder printed; refuse:tokenizer then fixture stubs; SKIP live train; SKIP live convert; SKIP live seat)"
echo "READY_FOR_LIVE_TEST: no"
