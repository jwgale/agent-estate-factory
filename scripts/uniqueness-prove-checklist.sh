#!/usr/bin/env bash
# Print-only operator checklist for the recorded Target C live uniqueness ladder.
# Reads docs/LIVE-PROBES.md section "Target C live uniqueness (5090-class)"
# and prints the ordered operator steps. Does not train, convert, shell out
# to ollama, or promote. Does not invent a live PASS.
# Resolves estate fail-closed: executable ESTATE_BIN, then target/release/estate,
# then target/debug/estate, then cargo on PATH. Does not invent a binary.
# Does not execute that binary.
# CELL_TRAIN_LIVE=1 stays print-only. CELL_SEAT_LIVE=1 stays print-only.
# Not native MLX.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

PROBES="$ROOT/docs/LIVE-PROBES.md"
BIN="${ESTATE_BIN:-}"

# Fail closed when cargo is not on PATH.
# Order: executable ESTATE_BIN, then target/release/estate,
# then target/debug/estate, then `cargo run -q -p estate-control --`.
# A set ESTATE_BIN that is not executable does not fall through.
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

echo "== uniqueness-prove-checklist (print-only operator steps for the recorded Target C live uniqueness ladder) =="
echo "Print-only. READY_FOR_LIVE_TEST: no"
echo "The factory does not train, convert, shell out to ollama, or promote."
echo "CELL_TRAIN_LIVE=1 stays print-only."
echo "CELL_SEAT_LIVE=1 stays print-only."
echo "Not native MLX."
echo "Not in make smoke, make gate-90, or GitHub Actions."
echo "Recorded PASS: docs/LIVE-PROBES.md section Target C live uniqueness (5090-class)."
echo "This checklist does not invent a new live PASS."
echo "examples/estate.yaml stays unchanged."
echo "This checklist does not run make qlora-journey, make train-next, make seat-journey, make uniqueness-ladder, or make uniqueness-full."

if [[ "${CELL_TRAIN_LIVE:-}" == "1" ]]; then
  echo "CELL_TRAIN_LIVE=1 is set. This journey stays print-only."
fi
if [[ "${CELL_SEAT_LIVE:-}" == "1" ]]; then
  echo "CELL_SEAT_LIVE=1 is set. This journey stays print-only."
fi

echo

section="$(awk '
  $0 == "## Target C live uniqueness (5090-class)" { flag = 1; next }
  flag && /^## / { exit }
  flag { print }
' "$PROBES")"
if [[ -z "${section}" ]]; then
  echo "FAIL  docs/LIVE-PROBES.md missing Target C live uniqueness (5090-class)" >&2
  exit 1
fi

require_phrase() {
  local needle="$1"
  if ! grep -q -F -- "$needle" <<<"$section"; then
    echo "FAIL  recorded prove missing phrase: $needle" >&2
    exit 1
  fi
}

require_phrase 'llamafactory-qlora'
require_phrase 'Seat tag `llama3`'
require_phrase 'Qwen/Qwen2.5-0.5B-Instruct'
require_phrase 'outside the factory'
require_phrase 'refuse:tokenizer'
require_phrase 'cp -aL'
require_phrase 'cp --dereference'
require_phrase 'did not write `$PREPARED/Modelfile`'
require_phrase 'from the printed contents'
require_phrase 'import-trained'
require_phrase 'trained_shape` `gguf`'
require_phrase 'auto_apply=false'
require_phrase 'ollama rm'
require_phrase 'did not use `examples/estate.yaml` as the'
require_phrase '43770130 3391'
require_phrase 'The factory did not train, convert, shell out to ollama, or promote.'
require_phrase 'not native MLX'
require_phrase 'not in `make smoke`'
require_phrase 'make gate-90'
require_phrase 'GitHub Actions'
require_phrase 'cell-target-c-qlora-prove'

flat="$(printf '%s\n' "$section" | tr '\n' ' ' | sed -e 's/[[:space:]]\+/ /g')"
order="$(printf '%s\n' "$flat" | sed -n 's/.*\(Prepare → .*→ cleanup\.\).*/\1/p')"
if [[ -z "$order" ]]; then
  echo "FAIL  recorded prove missing the operator arrow order" >&2
  exit 1
fi

resolve_estate
prefix="$(printf '%s ' "${ESTATE_CMD[@]}")"
prefix="${prefix%" "}"

echo "Estate command prefix (resolved, not executed): $prefix"
echo "Recorded order (not a new live PASS): $order"
echo
echo "Ordered operator steps:"
echo "1. Prepare the LLaMA-Factory QLoRA card (llamafactory-qlora). Seat tag llama3. Train base Qwen/Qwen2.5-0.5B-Instruct."
echo "   Prepare against a throwaway copy. The recorded prove did not use examples/estate.yaml as the prepare target."
echo "   Print check (does not train): make qlora-journey. This checklist does not run it."
echo "2. Train and export outside the factory. Print the NEXT.md recipe. SKIP live train."
echo "   Print check (does not train): make train-next. This checklist does not run it."
echo "   CELL_TRAIN_LIVE=1 stays print-only."
echo "3. After refuse:tokenizer, restore tokenizer files with dereference (cp -aL or cp --dereference)."
echo "   A plain cp -a leaves symlinks. Enrich does not follow a symlinked tokenizer_config.json."
echo "4. $prefix enrich gguf-convert"
echo "   This checklist does not convert."
echo "5. $prefix enrich local-seat"
echo "   local-seat is print-only. It prints the Modelfile and does not write \$PREPARED/Modelfile."
echo "   CELL_SEAT_LIVE=1 stays print-only."
echo "6. The operator writes the Modelfile from the printed contents."
echo "7. ollama create outside the factory. The recorded prove tag was cell-target-c-qlora-prove."
echo "   This checklist does not shell out to ollama."
echo "8. $prefix enrich import-trained"
echo "   trained_shape gguf. auto_apply=false. This checklist does not import and does not promote."
echo "9. Cleanup with ollama rm. examples/estate.yaml unchanged."

sum="$(cksum "$ROOT/examples/estate.yaml")"
echo "   cksum: $sum"
case "$sum" in
  "43770130 3391"*) ;;
  *)
    echo "FAIL  examples/estate.yaml cksum is not 43770130 3391: $sum" >&2
    exit 1
    ;;
esac

echo
echo "Recorded PASS stays in docs/LIVE-PROBES.md. This print is not a live PASS."
