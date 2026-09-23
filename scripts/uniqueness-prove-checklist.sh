#!/usr/bin/env bash
# Print-only operator checklist for the recorded Target C live uniqueness ladder.
# Reads docs/LIVE-PROBES.md section "Target C live uniqueness (5090-class)"
# and prints the ordered operator steps. After step 8 (import-trained,
# trained_shape gguf, auto_apply=false) it prints Standing next (estate).
# That coda names plan, apply --require-plan, and reconcile. It does not
# execute them. Does not train, convert, shell out to ollama, or promote.
# Does not invent a live PASS.
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

# Standing next (estate) is print-only. The checks below refuse a coda that
# claims this factory applied, promoted, trained, converted, or shelled out.
standing_next_coda() {
  cat <<EOF
Standing next (estate) — after step 8 (import-trained, trained_shape gguf, auto_apply=false):
Print-only. READY_FOR_LIVE_TEST: no
This checklist resolves the estate command and does not execute it.
1. The proposal stays auto_apply=false. The factory does not apply the estate without an explicit operator --require-plan path. The curator path is packs accept --curator jason. That accept writes edit instructions and does not rewrite estate.yaml.
2. No promote. No auto-promote. examples/estate.yaml stays unchanged unless the operator deliberately applies a plan. Promote commands stay refused.
3. Existing entrypoints (print only; this checklist does not execute them):
   $prefix enrich apply-proposal --estate <lab-estate.yaml> --prepared <prepared> --tag <tag> --state-dir .cell
      Writes {state}/enrich-stage/staged-estate.yaml. Does not apply. Does not rewrite the source estate.
   $prefix plan --estate <lab-estate.yaml> --plans-dir plans --state-dir .cell
      Writes reviewable markdown under plans/. Does not apply.
   $prefix plan diff --estate <lab-estate.yaml> --plans-dir plans --state-dir .cell
      Compares plans. Does not apply.
   $prefix plan export-pr --estate <lab-estate.yaml> --plans-dir plans --out plans/PR.md
      Writes markdown for a human PR. Does not apply.
   $prefix apply --dry-run --estate <lab-estate.yaml> --state-dir .cell
      Prints blast radius and a reconcile preview. Does not write leases.
   $prefix apply --estate <lab-estate.yaml> --state-dir .cell
      Without --require-plan, converges the cell and leaves the source estate unchanged.
   $prefix apply --estate <lab-estate.yaml> --state-dir .cell --require-plan --curator jason
      The source estate is written only when this apply succeeds. Point --estate at a lab copy.
   $prefix apply --estate <lab-estate.yaml> --state-dir .cell --require-plan --require-fresh-plan --curator jason
      Also fails when the covering plan against_hash does not match the last apply. This checklist does not execute it.
   $prefix reconcile --estate <lab-estate.yaml> --state-dir .cell
      Report only. Does not apply.
   $prefix reconcile --suggest --estate <lab-estate.yaml> --state-dir .cell
      Writes a patch file. Never auto-applies. The operator still applies by hand.
   $prefix packs accept --id <pack-id> --curator jason
      Writes enrich-pack edit instructions. Does not rewrite estate.yaml. Jason pastes them by hand.
   $prefix packs promote --id <pack-id>
      Always fails. Auto-promote is locked off. This checklist does not execute it.
   $prefix feed promote --id <pack-id>
      Always fails. Auto-promote is locked off. This checklist does not execute it.
4. Recorded PASS stays in docs/LIVE-PROBES.md section Target C live uniqueness (5090-class). This print is not a live PASS. Re-prove card: make uniqueness-prove-checklist.
The factory does not train, convert, shell out to ollama, or promote.
CELL_TRAIN_LIVE=1 stays print-only.
CELL_SEAT_LIVE=1 stays print-only.
Not native MLX.
Not in make smoke, make gate-90, or GitHub Actions.
This checklist does not write a Modelfile and does not shell out to ollama.
5. Phrase-check passed: this coda does not claim the factory applied, promoted, trained, converted, or shelled out to ollama.
EOF
}

coda_require() {
  local needle="$1"
  if ! grep -q -F -- "$needle" <<<"$coda"; then
    echo "FAIL  standing next coda missing phrase: $needle" >&2
    exit 1
  fi
}

coda_forbid() {
  local needle="$1"
  if grep -q -F -- "$needle" <<<"$coda"; then
    echo "FAIL  standing next coda claims: $needle" >&2
    exit 1
  fi
}

coda_before() {
  local earlier="$1"
  local later="$2"
  local a b
  a="$(grep -n -F -m1 -- "$earlier" <<<"$coda" | cut -d: -f1 || true)"
  b="$(grep -n -F -m1 -- "$later" <<<"$coda" | cut -d: -f1 || true)"
  if [[ -z "$a" || -z "$b" || "$a" -ge "$b" ]]; then
    echo "FAIL  standing next coda order: $earlier must precede $later" >&2
    exit 1
  fi
}

coda="$(standing_next_coda)"

coda_require "Standing next (estate)"
coda_require "The proposal stays auto_apply=false."
coda_require "does not apply the estate without an explicit operator --require-plan path"
coda_require "packs accept --curator jason"
coda_require "No promote. No auto-promote."
coda_require "examples/estate.yaml stays unchanged unless the operator deliberately applies a plan."
coda_require "Existing entrypoints (print only; this checklist does not execute them):"
coda_require "$prefix enrich apply-proposal --estate <lab-estate.yaml>"
coda_require "$prefix plan --estate <lab-estate.yaml> --plans-dir plans --state-dir .cell"
coda_require "$prefix apply --estate <lab-estate.yaml> --state-dir .cell --require-plan --curator jason"
coda_require "$prefix reconcile --estate <lab-estate.yaml> --state-dir .cell"
coda_require "$prefix reconcile --suggest --estate <lab-estate.yaml> --state-dir .cell"
coda_require "$prefix packs accept --id <pack-id> --curator jason"
coda_require "Always fails. Auto-promote is locked off."
coda_require "Recorded PASS stays in docs/LIVE-PROBES.md section Target C live uniqueness (5090-class)."
coda_require "Re-prove card: make uniqueness-prove-checklist."
coda_require "This print is not a live PASS."
coda_require "READY_FOR_LIVE_TEST: no"
coda_require "The factory does not train, convert, shell out to ollama, or promote."
coda_require "This checklist does not write a Modelfile and does not shell out to ollama."
coda_require "CELL_TRAIN_LIVE=1 stays print-only."
coda_require "CELL_SEAT_LIVE=1 stays print-only."
coda_require "Not native MLX."
coda_require "Phrase-check passed:"

coda_before "The proposal stays auto_apply=false." "No promote. No auto-promote."
coda_before "No promote. No auto-promote." "Existing entrypoints (print only; this checklist does not execute them):"
coda_before "enrich apply-proposal --estate <lab-estate.yaml>" " plan --estate <lab-estate.yaml> --plans-dir plans"
coda_before " plan --estate <lab-estate.yaml> --plans-dir plans" " apply --estate <lab-estate.yaml> --state-dir .cell --require-plan --curator jason"
coda_before " apply --estate <lab-estate.yaml> --state-dir .cell --require-plan --curator jason" " reconcile --estate <lab-estate.yaml> --state-dir .cell"
coda_before " reconcile --estate <lab-estate.yaml> --state-dir .cell" " reconcile --suggest"
coda_before "packs accept --id <pack-id> --curator jason" "Recorded PASS stays in docs/LIVE-PROBES.md"
coda_before "Re-prove card: make uniqueness-prove-checklist." "Phrase-check passed:"

coda_forbid "READY_FOR_LIVE_TEST: yes"
coda_forbid "The factory applied"
coda_forbid "The factory promoted"
coda_forbid "The factory trained"
coda_forbid "The factory converted"
coda_forbid "This checklist applied"
coda_forbid "This checklist promoted"
coda_forbid "This checklist trained"
coda_forbid "This checklist converted"
coda_forbid "The factory shelled out"
coda_forbid "This checklist shelled out"
coda_forbid "wrote examples/estate.yaml"
coda_forbid "wrote the Modelfile"
coda_forbid "--estate examples/estate.yaml"
coda_forbid "Kimi"
coda_forbid "kimi"

while IFS= read -r line; do
  if [[ -z "$line" ]]; then
    continue
  fi
  if printf '%s\n' "$line" | grep -Eq '(^|[^[:alnum:]_-])(applied|promoted|trained|converted)([^[:alnum:]_-]|$)|shelled out to ollama'; then
    if ! printf '%s\n' "$line" | grep -Eq 'does not|did not|Never |No '; then
      echo "FAIL  standing next coda claims an action: $line" >&2
      exit 1
    fi
  fi
done <<<"$coda"

echo
printf '%s\n' "$coda"
echo
echo "Recorded PASS stays in docs/LIVE-PROBES.md. This print is not a live PASS."
