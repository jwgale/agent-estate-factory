#!/usr/bin/env bash
# mlx-lm LoRA optional NEXT path. Print-only. Apple Silicon affinity only.
# Prepares mlx-lm-lora on a throwaway copy of examples/estate.yaml
# (seat tag llama3, train base Qwen/Qwen2.5-0.5B-Instruct, overnight pack
# rewritten to host_class_affinity apple-silicon). The stock overnight pack
# stays host any and is refuse:host.
# Asserts MLX.md, PREPARE.md, NEXT.md, and the seat_tag / train_base split.
# Phase prepare stops after refuse:host, refuse:train-base,
# refuse:official-scale, refuse:dataset, refuse:adapter (missing and wrong
# shape), and refuse:seat on missing paths. Phase seat (and the default
# phase all) then prints merge-adapt and local-seat, and import-trained,
# against fixture stubs. gguf-convert stays refuse:seat. A fused MLX
# directory is refuse:adapter and refuse:seat. local-seat --adapter stays
# refuse:adapter.
# Does not install mlx-lm, does not call mlx-lm, does not fuse, does not
# train, does not merge, does not convert, does not run ollama, and does
# not promote.
# CELL_SEAT_LIVE=1 and CELL_TRAIN_LIVE=1 do not start a live phase.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
# Resolves estate fail-closed: executable ESTATE_BIN, then target/release/estate,
# then target/debug/estate, then cargo on PATH. Does not invent a binary.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

PHASE="${MLX_LM_LORA_PHASE:-all}"
case "$PHASE" in
  prepare|seat|all) ;;
  *)
    echo "FAIL  MLX_LM_LORA_PHASE must be prepare, seat, or all (got: $PHASE)" >&2
    exit 1
    ;;
esac

# Throwaway dir stays out of the checkout. prepare refuses a hardware SKU
# anywhere in the output path, including a parent directory name.
WORKDIR="${WORKDIR:-${TMPDIR:-/tmp}/cell-one-mlx-lm-lora-journey}"
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
    echo "FAIL  mlx-lm-lora-journey rewrote examples/estate.yaml" >&2
    exit 1
  fi
  if [[ -n "${SEATED_BEFORE:-}" && -f "${SEATED:-}" ]]; then
    seated_after="$(cksum "$SEATED")"
    if [[ "$SEATED_BEFORE" != "$seated_after" ]]; then
      echo "FAIL  import-trained rewrote the lab estate copy" >&2
      exit 1
    fi
  fi
  pack_after="$(cksum "$PACK")"
  if [[ "$PACK_BEFORE" != "$pack_after" ]]; then
    echo "FAIL  mlx-lm-lora-journey rewrote the overnight pack" >&2
    exit 1
  fi
}

if [[ ! -f "$PACK" ]]; then
  echo "SKIP mlx-lm-lora-journey (example pack missing; not a PASS)"
  exit 0
fi

echo "== mlx-lm-lora-journey (optional mlx-lm LoRA NEXT path; print-only; apple-silicon; phase=${PHASE}) =="
echo "workdir: $WORKDIR"
echo "READY_FOR_LIVE_TEST: no"
if [[ "${CELL_SEAT_LIVE:-}" == "1" || "${CELL_TRAIN_LIVE:-}" == "1" ]]; then
  echo "CELL_SEAT_LIVE or CELL_TRAIN_LIVE is set. This journey stays print-only."
  echo "Live train, fuse, convert, and seat stay on the operator host."
fi
echo "SKIP live train"
echo "SKIP live convert"
echo "SKIP live seat"
echo
echo "Ladder (existing commands only; fixture stubs, not weights):"
echo "1. estate enrich prepare --driver mlx-lm-lora"
echo "   The stock overnight pack is host any. That prepare is refuse:host."
echo "   This journey rewrites a throwaway pack to host_class_affinity apple-silicon."
echo "   Seat tag stays the Ollama id. Train base is a Hugging Face repo id."
echo "   Example seat ${SEAT_TAG}. Train base ${TRAIN_BASE}."
echo "   Writes MLX.md, PREPARE.md, NEXT.md, and prepare.json."
echo "   Does not write a script, a recipe, or dataset.jsonl."
echo "2. estate enrich merge-adapt --prepared <prepared> --adapter <prepared>/adapters"
echo "   The adapter stub holds adapter_config.json and adapters.safetensors."
echo "   adapter_model.safetensors is the wrong shape (refuse:adapter)."
echo "   Prints mlx_lm.fuse with --adapter-path, --save-path, and --export-gguf."
echo "   The save path is <prepared>/fused_model. The GGUF name is ggml-model-f16.gguf."
echo "   This factory does not fuse."
echo "3. estate enrich gguf-convert stays refuse:seat on this card."
echo "   The documented GGUF path is mlx_lm.fuse --export-gguf."
echo "   This factory does not invent a convert script and does not run one."
echo "4. estate enrich local-seat --prepared <prepared> --weights <prepared>/fused_model/ggml-model-f16.gguf"
echo "   A fused MLX directory (config.json plus model.safetensors) is refuse:seat."
echo "   local-seat --adapter stays refuse:adapter on this card."
echo "   local-seat is print-only. It prints the Modelfile and does not write it."
echo "   The GGUF stub starts with GGUF magic. Prints ollama create."
echo "   This factory does not run it and does not write that file."
echo "5. estate enrich import-trained --adapter <prepared>/fused_model/ggml-model-f16.gguf"
echo "   Records trained_shape gguf and trained_paths."
echo "   A fused MLX directory is refuse:adapter. It does not record trained_shape merged."
echo "   The proposal stays auto_apply=false. Does not apply the estate. Does not promote."
echo

resolve_estate

BEFORE="$(cksum "$ESTATE")"
PACK_BEFORE="$(cksum "$PACK")"
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR/logs"
SEATED_ONLY="$WORKDIR/estate-seat-only.yaml"
SEATED="$WORKDIR/estate.yaml"
APPLE_PACK="$WORKDIR/apple.pack.json"
python3 - "$ESTATE" "$SEATED_ONLY" "$SEATED" "$TRAIN_BASE" "$PACK" "$APPLE_PACK" <<'PY'
import sys
src, seat_only, seated, train_base, pack_src, apple_pack = sys.argv[1:]
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
pack = open(pack_src).read()
old = '"host_class_affinity": "any"'
new = '"host_class_affinity": "apple-silicon"'
if old not in pack:
    raise SystemExit("FAIL  overnight pack has no host_class_affinity any")
open(apple_pack, "w").write(pack.replace(old, new, 1))
PY

echo "-- host any refuses (no handoff) --"
set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver mlx-lm-lora \
  --job train \
  --out "$WORKDIR/wrong-host" \
  >"$WORKDIR/logs/wrong-host.out" 2>"$WORKDIR/logs/wrong-host.err"
host_rc=$?
set -e
if [[ "$host_rc" -eq 0 ]]; then
  echo "FAIL  host any must refuse:host" >&2
  exit 1
fi
require "refuse:host" "$WORKDIR/logs/wrong-host.out" "$WORKDIR/logs/wrong-host.err"
require "apple-silicon" "$WORKDIR/logs/wrong-host.out" "$WORKDIR/logs/wrong-host.err"
if [[ -e "$WORKDIR/wrong-host" ]]; then
  echo "FAIL  wrong-host prepare wrote an output directory" >&2
  exit 1
fi
if grep -q "mlx_lm.fuse --model ${TRAIN_BASE}" "$WORKDIR/logs/wrong-host.out" "$WORKDIR/logs/wrong-host.err"; then
  echo "FAIL  refuse:host printed the fuse line" >&2
  exit 1
fi

echo "-- seat tag without a train base refuses --"
set +e
estate enrich prepare \
  --estate "$SEATED_ONLY" \
  --pack "$APPLE_PACK" \
  --driver mlx-lm-lora \
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
  --pack "$APPLE_PACK" \
  --driver mlx-lm-lora \
  --job train \
  --official-scale \
  --out "$WORKDIR/official" \
  >"$WORKDIR/logs/official.out" 2>"$WORKDIR/logs/official.err"
official_rc=$?
set -e
if [[ "$official_rc" -eq 0 ]]; then
  echo "FAIL  --official-scale on mlx-lm-lora must refuse:official-scale" >&2
  exit 1
fi
require "refuse:official-scale" "$WORKDIR/logs/official.out" "$WORKDIR/logs/official.err"
if [[ -e "$WORKDIR/official" ]]; then
  echo "FAIL  official-scale prepare wrote an output directory" >&2
  exit 1
fi

echo "-- from-feed on this card alone is refuse:dataset --"
set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$APPLE_PACK" \
  --driver mlx-lm-lora \
  --job train \
  --from-feed \
  --state-dir "$WORKDIR/state" \
  --out "$WORKDIR/from-feed" \
  >"$WORKDIR/logs/from-feed.out" 2>"$WORKDIR/logs/from-feed.err"
feed_rc=$?
set -e
if [[ "$feed_rc" -eq 0 ]]; then
  echo "FAIL  --from-feed on mlx-lm-lora must refuse:dataset" >&2
  exit 1
fi
require "refuse:dataset" "$WORKDIR/logs/from-feed.out" "$WORKDIR/logs/from-feed.err"
if [[ -e "$WORKDIR/from-feed" ]]; then
  echo "FAIL  from-feed prepare wrote an output directory" >&2
  exit 1
fi

echo "-- mlx-lm-lora prepare writes the operator-owned handoff --"
PREPARED="$WORKDIR/mlx-lm-lora"
ADAPTER="$PREPARED/adapters"
FUSED="$PREPARED/fused_model"
GGUF="$FUSED/ggml-model-f16.gguf"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$APPLE_PACK" \
  --driver mlx-lm-lora \
  --job train \
  --out "$PREPARED" \
  >"$WORKDIR/logs/prepare.out"
echo "prepared: $PREPARED"

test -f "$PREPARED/MLX.md"
test -f "$PREPARED/PREPARE.md"
test -f "$PREPARED/NEXT.md"
test -f "$PREPARED/prepare.json"
if [[ -e "$PREPARED/recipe.yaml" || -e "$PREPARED/axolotl.yml" || -e "$PREPARED/dataset.jsonl" || -e "$PREPARED/export.yaml" || -e "$PREPARED/convert_hf_to_gguf.py" || -e "$FUSED" || -e "$ADAPTER" || -e "$GGUF" || -e "$PREPARED/UNSLOTH.md" ]]; then
  echo "FAIL  prepare must not invent a script, a recipe, a dataset, or weights" >&2
  exit 1
fi
if compgen -G "$PREPARED/*.py" >/dev/null; then
  echo "FAIL  prepare must not invent a Python script" >&2
  exit 1
fi

require "operator-owned" "$PREPARED/MLX.md"
require "does not call mlx-lm" "$PREPARED/MLX.md"
require "not a training script" "$PREPARED/MLX.md"
require "apple_silicon_only: true" "$PREPARED/MLX.md"
require "host_class_affinity: apple-silicon" "$PREPARED/MLX.md"
require "status: optional" "$PREPARED/MLX.md"
require "train_base_model: \"${TRAIN_BASE}\"" "$PREPARED/MLX.md"
require "seat_tag: \"${SEAT_TAG}\"" "$PREPARED/MLX.md"
require "https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md" "$PREPARED/MLX.md"
require "mlx_lm.fuse --model <path_to_model>" "$PREPARED/MLX.md"
require "mlx_lm.lora" "$PREPARED/MLX.md"
require "adapters.safetensors" "$PREPARED/MLX.md"
require "ggml-model-f16.gguf" "$PREPARED/MLX.md"
require "refuse:adapter" "$PREPARED/MLX.md"
require "fused MLX" "$PREPARED/MLX.md"
require "READY_FOR_LIVE_TEST: no" "$PREPARED/MLX.md"
require "Seat tag: ${SEAT_TAG}" "$PREPARED/PREPARE.md"
require "Train base: ${TRAIN_BASE}" "$PREPARED/PREPARE.md"
require "did not call mlx-lm" "$PREPARED/PREPARE.md"
require "--export-gguf" "$PREPARED/PREPARE.md"
require "estate enrich merge-adapt --prepared ${PREPARED} --adapter '<adapter-dir>'" "$PREPARED/NEXT.md"
require "mlx_lm.fuse --model ${TRAIN_BASE} --adapter-path '<adapter-dir>' --save-path ${FUSED}" "$PREPARED/NEXT.md"
require "mlx_lm.fuse --model ${TRAIN_BASE} --adapter-path '<adapter-dir>' --save-path ${FUSED} --export-gguf" "$PREPARED/NEXT.md"
require "ggml-model-f16.gguf" "$PREPARED/NEXT.md"
require "estate enrich local-seat --prepared ${PREPARED} --weights ${GGUF}" "$PREPARED/NEXT.md"
require "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag ${TAG} --adapter '<adapter-dir>'" "$PREPARED/NEXT.md"
require "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag ${TAG} --adapter ${GGUF}" "$PREPARED/NEXT.md"
require "## After the mlx-lm train" "$PREPARED/NEXT.md"
require "fused MLX" "$PREPARED/NEXT.md"
require "refuse:adapter" "$PREPARED/NEXT.md"
require "does not call mlx-lm" "$PREPARED/NEXT.md"
require "READY_FOR_LIVE_TEST: no" "$PREPARED/NEXT.md"
if grep -q "python3 convert_hf_to_gguf.py" "$PREPARED/MLX.md" "$PREPARED/PREPARE.md" "$PREPARED/NEXT.md"; then
  echo "FAIL  mlx-lm-lora must not print a Hugging Face convert line" >&2
  exit 1
fi
if grep -q "estate enrich gguf-convert" "$PREPARED/MLX.md" "$PREPARED/PREPARE.md" "$PREPARED/NEXT.md"; then
  echo "FAIL  mlx-lm-lora must not point at gguf-convert" >&2
  exit 1
fi

python3 - "$PREPARED/prepare.json" "$TRAIN_BASE" "$SEAT_TAG" <<'PY'
import json, sys
prepare_path, train_base, seat = sys.argv[1:]
prepare = json.load(open(prepare_path))
if prepare.get("schema") != "cell-one.enrich-prepare.v0":
    raise SystemExit(f"FAIL  schema={prepare.get('schema')}")
if prepare.get("driver") != "mlx-lm-lora":
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
if prepare.get("host_class_affinity") != "apple-silicon":
    raise SystemExit(f"FAIL  host_class_affinity={prepare.get('host_class_affinity')}")
if prepare.get("promoted") is not False or prepare.get("auto_apply") is not False:
    raise SystemExit("FAIL  prepare.json must stay unpromoted")
if prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  prepare.json claims an estate rewrite")
if "trained_shape" in prepare or "trained_paths" in prepare:
    raise SystemExit("FAIL  prepare must not record a trained shape before import-trained")
PY

SEATED_BEFORE="$(cksum "$SEATED")"

echo "-- missing adapter refuses (no fuse) --"
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
if [[ -e "$ADAPTER" || -e "$FUSED" ]]; then
  echo "FAIL  merge-adapt wrote an adapter directory or a fused directory" >&2
  exit 1
fi

echo "-- adapter_config.json without adapters.safetensors is refuse:adapter --"
mkdir -p "$ADAPTER"
printf '%s\n' '{}' > "$ADAPTER/adapter_config.json"
printf '%s\n' 'checkpoint' > "$ADAPTER/0000001_adapters.safetensors"
set +e
estate enrich merge-adapt \
  --prepared "$PREPARED" \
  --adapter "$ADAPTER" \
  >"$WORKDIR/logs/merge-shape.out" 2>"$WORKDIR/logs/merge-shape.err"
shape_rc=$?
set -e
if [[ "$shape_rc" -eq 0 ]]; then
  echo "FAIL  merge-adapt must refuse an adapter directory that has no adapters.safetensors" >&2
  exit 1
fi
require "refuse:adapter" "$WORKDIR/logs/merge-shape.out" "$WORKDIR/logs/merge-shape.err"
require "adapters.safetensors" "$WORKDIR/logs/merge-shape.out" "$WORKDIR/logs/merge-shape.err"
require "_adapters.safetensors" "$WORKDIR/logs/merge-shape.out" "$WORKDIR/logs/merge-shape.err"
if grep -q -- "--save-path" "$WORKDIR/logs/merge-shape.out" "$WORKDIR/logs/merge-shape.err"; then
  echo "FAIL  wrong-shape adapter printed the fuse line" >&2
  exit 1
fi
rm -rf "$ADAPTER"

echo "-- PEFT adapter_model.safetensors is the wrong shape --"
mkdir -p "$ADAPTER"
printf '%s\n' '{}' > "$ADAPTER/adapter_config.json"
printf '%s\n' 'not-an-mlx-weight' > "$ADAPTER/adapter_model.safetensors"
printf '%s\n' 'also-mlx' > "$ADAPTER/adapters.safetensors"
set +e
estate enrich merge-adapt \
  --prepared "$PREPARED" \
  --adapter "$ADAPTER" \
  >"$WORKDIR/logs/merge-peft-shape.out" 2>"$WORKDIR/logs/merge-peft-shape.err"
peft_shape_rc=$?
set -e
if [[ "$peft_shape_rc" -eq 0 ]]; then
  echo "FAIL  merge-adapt must refuse adapter_model.safetensors on mlx-lm-lora" >&2
  exit 1
fi
require "refuse:adapter" "$WORKDIR/logs/merge-peft-shape.out" "$WORKDIR/logs/merge-peft-shape.err"
require "adapter_model.safetensors" "$WORKDIR/logs/merge-peft-shape.out" "$WORKDIR/logs/merge-peft-shape.err"
if grep -q -- "--save-path" "$WORKDIR/logs/merge-peft-shape.out" "$WORKDIR/logs/merge-peft-shape.err"; then
  echo "FAIL  PEFT-shaped adapter printed the fuse line" >&2
  exit 1
fi
rm -rf "$ADAPTER"
if [[ -e "$FUSED" || -e "$GGUF" ]]; then
  echo "FAIL  wrong-shape refuses wrote a fused directory or a GGUF" >&2
  exit 1
fi

echo "-- missing GGUF and fused directory refuse (no convert, no seat) --"
set +e
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$FUSED" \
  >"$WORKDIR/logs/gguf-missing.out" 2>"$WORKDIR/logs/gguf-missing.err"
gguf_missing_rc=$?
set -e
if [[ "$gguf_missing_rc" -eq 0 ]]; then
  echo "FAIL  gguf-convert must refuse before a fused directory exists" >&2
  exit 1
fi
require "refuse:seat" "$WORKDIR/logs/gguf-missing.out" "$WORKDIR/logs/gguf-missing.err"
if grep -q "python3 convert_hf_to_gguf.py" "$WORKDIR/logs/gguf-missing.out" "$WORKDIR/logs/gguf-missing.err"; then
  echo "FAIL  missing fused directory printed a convert line" >&2
  exit 1
fi
if [[ -e "$GGUF" || -e "$FUSED" ]]; then
  echo "FAIL  gguf-convert wrote a GGUF or a fused directory" >&2
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
  echo "PASS  mlx-lm-lora-journey (prepare-assert; MLX.md handoff checked; refuse:host; refuse:adapter; refuse:seat; SKIP live train; SKIP live convert; SKIP live seat)"
  echo "READY_FOR_LIVE_TEST: no"
  exit 0
fi

echo "-- fixture stub: adapters dir with adapters.safetensors --"
mkdir -p "$ADAPTER"
printf '%s\n' '{}' > "$ADAPTER/adapter_config.json"
printf '%s\n' 'not-a-real-tensor' > "$ADAPTER/adapters.safetensors"
if [[ -e "$FUSED" || -e "$GGUF" ]]; then
  echo "FAIL  adapter stub must not create a fused directory or a GGUF" >&2
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
  echo "FAIL  local-seat --adapter must refuse on mlx-lm-lora" >&2
  exit 1
fi
require "refuse:adapter" "$WORKDIR/logs/seat-adapter.out" "$WORKDIR/logs/seat-adapter.err"
require "adapters.safetensors" "$WORKDIR/logs/seat-adapter.out" "$WORKDIR/logs/seat-adapter.err"
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
  echo "FAIL  --weights on the mlx adapter directory must refuse:seat" >&2
  exit 1
fi
require "refuse:seat" "$WORKDIR/logs/seat-weights-adapter.out" "$WORKDIR/logs/seat-weights-adapter.err"
require "refuse:seat" "$WORKDIR/logs/gguf-adapter.out" "$WORKDIR/logs/gguf-adapter.err"
if grep -q "ollama create" "$WORKDIR/logs/seat-weights-adapter.out" "$WORKDIR/logs/seat-weights-adapter.err"; then
  echo "FAIL  adapter --weights printed a create line" >&2
  exit 1
fi
if grep -q "python3 convert_hf_to_gguf.py" "$WORKDIR/logs/gguf-adapter.out" "$WORKDIR/logs/gguf-adapter.err"; then
  echo "FAIL  adapter --weights printed a convert line" >&2
  exit 1
fi

echo "-- merge-adapt prints mlx_lm.fuse --"
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
require "shape=mlx-adapter" "$WORKDIR/logs/merge.out"
require "mlx_lm.fuse --model ${TRAIN_BASE}" "$WORKDIR/logs/merge.out"
require "--adapter-path" "$WORKDIR/logs/merge.out"
require "--save-path" "$WORKDIR/logs/merge.out"
require "${FUSED}" "$WORKDIR/logs/merge.out"
require "--export-gguf" "$WORKDIR/logs/merge.out"
require "${GGUF}" "$WORKDIR/logs/merge.out"
require "adapters.safetensors" "$WORKDIR/logs/merge.out"
require "estate enrich local-seat --prepared ${PREPARED} --weights ${GGUF}" "$WORKDIR/logs/merge.out"
require "merge-adapt did not fuse" "$WORKDIR/logs/merge.out"
require "READY_FOR_LIVE_TEST: no" "$WORKDIR/logs/merge.out"
if grep -q "python3 convert_hf_to_gguf.py" "$WORKDIR/logs/merge.out"; then
  echo "FAIL  mlx merge-adapt printed a Hugging Face convert line" >&2
  exit 1
fi
if grep -q "estate enrich gguf-convert" "$WORKDIR/logs/merge.out"; then
  echo "FAIL  mlx merge-adapt printed gguf-convert" >&2
  exit 1
fi
if grep -q "save_pretrained_merged" "$WORKDIR/logs/merge.out"; then
  echo "FAIL  mlx merge-adapt printed an Unsloth save line" >&2
  exit 1
fi
if grep -q "axolotl merge-lora" "$WORKDIR/logs/merge.out"; then
  echo "FAIL  mlx merge-adapt printed an Axolotl merge line" >&2
  exit 1
fi
if [[ -e "$FUSED" || -e "$GGUF" ]]; then
  echo "FAIL  merge-adapt wrote a fused directory or a GGUF" >&2
  exit 1
fi

echo "-- fused MLX directory is refuse:adapter and refuse:seat --"
mkdir -p "$FUSED"
printf '%s\n' '{}' > "$FUSED/config.json"
printf '%s\n' 'not-a-real-tensor' > "$FUSED/model.safetensors"
if [[ -e "$GGUF" ]]; then
  echo "FAIL  fused stub must not include a GGUF yet" >&2
  exit 1
fi

set +e
estate enrich merge-adapt \
  --prepared "$PREPARED" \
  --adapter "$FUSED" \
  >"$WORKDIR/logs/merge-fused.out" 2>"$WORKDIR/logs/merge-fused.err"
merge_fused_rc=$?
estate enrich local-seat \
  --prepared "$PREPARED" \
  --weights "$FUSED" \
  >"$WORKDIR/logs/seat-fused.out" 2>"$WORKDIR/logs/seat-fused.err"
seat_fused_rc=$?
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$FUSED" \
  >"$WORKDIR/logs/gguf-fused.out" 2>"$WORKDIR/logs/gguf-fused.err"
gguf_fused_rc=$?
estate enrich import-trained \
  --estate "$SEATED" \
  --prepared "$PREPARED" \
  --tag "$TAG" \
  --adapter "$FUSED" \
  --curator jason \
  >"$WORKDIR/logs/import-fused.out" 2>"$WORKDIR/logs/import-fused.err"
import_fused_rc=$?
set -e
if [[ "$merge_fused_rc" -eq 0 || "$seat_fused_rc" -eq 0 || "$gguf_fused_rc" -eq 0 || "$import_fused_rc" -eq 0 ]]; then
  echo "FAIL  a fused MLX directory must refuse" >&2
  exit 1
fi
require "refuse:adapter" "$WORKDIR/logs/merge-fused.out" "$WORKDIR/logs/merge-fused.err"
require "refuse:seat" "$WORKDIR/logs/seat-fused.out" "$WORKDIR/logs/seat-fused.err"
require "MLX weights" "$WORKDIR/logs/seat-fused.out" "$WORKDIR/logs/seat-fused.err"
require "refuse:seat" "$WORKDIR/logs/gguf-fused.out" "$WORKDIR/logs/gguf-fused.err"
require "MLX weights" "$WORKDIR/logs/gguf-fused.out" "$WORKDIR/logs/gguf-fused.err"
require "refuse:adapter" "$WORKDIR/logs/import-fused.out" "$WORKDIR/logs/import-fused.err"
require "fused MLX" "$WORKDIR/logs/import-fused.out" "$WORKDIR/logs/import-fused.err"
if grep -q "ollama create" "$WORKDIR/logs/seat-fused.out" "$WORKDIR/logs/seat-fused.err"; then
  echo "FAIL  fused directory printed a create line" >&2
  exit 1
fi
if grep -q "python3 convert_hf_to_gguf.py" "$WORKDIR/logs/gguf-fused.out" "$WORKDIR/logs/gguf-fused.err"; then
  echo "FAIL  fused directory printed a convert line" >&2
  exit 1
fi
if [[ -e "$PREPARED/binding-proposal.json" ]]; then
  echo "FAIL  fused import wrote a proposal" >&2
  exit 1
fi
if [[ -e "$GGUF" || -e "$FUSED/Modelfile" ]]; then
  echo "FAIL  fused refuses wrote a GGUF or a Modelfile" >&2
  exit 1
fi

echo "-- replace the fused weights with the GGUF file stub --"
rm -rf "$FUSED"
mkdir -p "$FUSED"

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

echo "-- gguf-convert on the GGUF file stays refuse:seat --"
set +e
estate enrich gguf-convert \
  --prepared "$PREPARED" \
  --weights "$GGUF" \
  >"$WORKDIR/logs/gguf-file.out" 2>"$WORKDIR/logs/gguf-file.err"
gguf_file_rc=$?
set -e
if [[ "$gguf_file_rc" -eq 0 ]]; then
  echo "FAIL  gguf-convert must refuse a file that is already GGUF" >&2
  exit 1
fi
require "refuse:seat" "$WORKDIR/logs/gguf-file.out" "$WORKDIR/logs/gguf-file.err"
require "is a GGUF" "$WORKDIR/logs/gguf-file.out" "$WORKDIR/logs/gguf-file.err"
require "--export-gguf" "$WORKDIR/logs/gguf-file.out" "$WORKDIR/logs/gguf-file.err"
if grep -q "python3 convert_hf_to_gguf.py" "$WORKDIR/logs/gguf-file.out" "$WORKDIR/logs/gguf-file.err"; then
  echo "FAIL  GGUF file printed a convert line" >&2
  exit 1
fi
if grep -q -- "--outtype" "$WORKDIR/logs/gguf-file.out" "$WORKDIR/logs/gguf-file.err"; then
  echo "FAIL  GGUF file printed an outtype" >&2
  exit 1
fi

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
if grep -q -- "--export-gguf" "$WORKDIR/logs/merge-gguf.out" "$WORKDIR/logs/merge-gguf.err"; then
  echo "FAIL  GGUF --adapter printed the export line" >&2
  exit 1
fi

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
require "driver=mlx-lm-lora" "$WORKDIR/logs/seat-gguf.out"
require "ollama create ${TAG} -f ${FUSED}/Modelfile" "$WORKDIR/logs/seat-gguf.out"
require "llama-cli -m " "$WORKDIR/logs/seat-gguf.out"
require "llama-server -m " "$WORKDIR/logs/seat-gguf.out"
require "mlx_lm.fuse --export-gguf" "$WORKDIR/logs/seat-gguf.out"
require "estate enrich import-trained --estate <estate.yaml> --prepared ${PREPARED} --tag ${TAG} --adapter ${GGUF}" "$WORKDIR/logs/seat-gguf.out"
require "standing next step" "$WORKDIR/logs/seat-gguf.out"
require "auto_apply=false" "$WORKDIR/logs/seat-gguf.out"
require "did not run ollama create" "$WORKDIR/logs/seat-gguf.out"
require "does not apply the estate" "$WORKDIR/logs/seat-gguf.out"
require "trained_shape is gguf" "$WORKDIR/logs/seat-gguf.out"
require "local-seat did not create a model" "$WORKDIR/logs/seat-gguf.out"
require "READY_FOR_LIVE_TEST: no" "$WORKDIR/logs/seat-gguf.out"
require "local-seat is print-only" "$WORKDIR/logs/seat-gguf.out"
require "does not write ${FUSED}/Modelfile" "$WORKDIR/logs/seat-gguf.out"
require "Write that file from the printed contents before ollama create" "$WORKDIR/logs/seat-gguf.out"
if grep -q "python3 convert_hf_to_gguf.py" "$WORKDIR/logs/seat-gguf.out"; then
  echo "FAIL  local-seat printed a Hugging Face convert line" >&2
  exit 1
fi
if [[ -e "$PREPARED/Modelfile" || -e "$FUSED/Modelfile" ]]; then
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
grep -h -E '^(mlx_lm.fuse |ollama create |llama-cli -m |llama-server -m |estate enrich import-trained )' "$WORKDIR/logs/merge.out" "$WORKDIR/logs/seat-gguf.out"
echo
echo "PASS  mlx-lm-lora-journey (mlx-lm LoRA handoff checked; fuse, seat, and import printed; refuse:host; refuse:adapter; refuse:seat; SKIP live train; SKIP live convert; SKIP live seat)"
echo "READY_FOR_LIVE_TEST: no"
