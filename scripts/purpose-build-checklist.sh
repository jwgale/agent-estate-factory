#!/usr/bin/env bash
# Print-only purpose-build operator checklist.
# Ordered steps for any purpose-built SLM need: choose a train card,
# SKIP live train, merge/export print, gguf convert print, local-seat print,
# import-trained, then Standing next (estate).
# Points at print-only cards already on tip. Does not run them.
# Does not train, convert, shell out to ollama, promote, or apply the estate.
# Does not invent a live PASS. The recorded 5090-class Target C uniqueness
# PASS in docs/LIVE-PROBES.md stays the only live uniqueness prove.
# The re-prove card stays make uniqueness-prove-checklist.
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

echo "== purpose-build-checklist (print-only operator steps for purpose-build on demand) =="
echo "Print-only. READY_FOR_LIVE_TEST: no"
echo "Purpose-build on demand. This checklist is operator UX."
echo "It is not a re-prove of the recorded Target C live PASS."
echo "The factory does not train, convert, shell out to ollama, or promote."
echo "The factory does not apply the estate."
echo "CELL_TRAIN_LIVE=1 stays print-only."
echo "CELL_SEAT_LIVE=1 stays print-only."
echo "Not native MLX."
echo "Not in make smoke, make gate-90, or GitHub Actions."
echo "Recorded PASS: docs/LIVE-PROBES.md section Target C live uniqueness (5090-class)."
echo "That recorded PASS stays the only live uniqueness prove."
echo "This checklist does not invent a new live PASS."
echo "The re-prove card stays make uniqueness-prove-checklist. This checklist does not run it."
echo "examples/estate.yaml stays unchanged."
echo "This checklist does not run make lf-beachhead-prepare, make qlora-journey, make lora-journey, make train-next, make train-next-lora, make seat-journey, make axolotl-qlora-journey, make axolotl-lora-journey, make unsloth-qlora-journey, make unsloth-lora-journey, or make mlx-lm-lora-journey."

if [[ "${CELL_TRAIN_LIVE:-}" == "1" ]]; then
  echo "CELL_TRAIN_LIVE=1 is set. This journey stays print-only."
fi
if [[ "${CELL_SEAT_LIVE:-}" == "1" ]]; then
  echo "CELL_SEAT_LIVE=1 is set. This journey stays print-only."
fi

echo

resolve_estate
prefix="$(printf '%s ' "${ESTATE_CMD[@]}")"
prefix="${prefix%" "}"

echo "Estate command prefix (resolved, not executed): $prefix"
echo
echo "Ordered operator steps:"
echo "1. Choose and prepare a train card. Print-only. This checklist does not prepare."
echo "   Beachhead rows: make lf-beachhead-prepare (SKIP live train). This checklist does not run it."
echo "   LLaMA-Factory QLoRA: make qlora-journey. LLaMA-Factory LoRA: make lora-journey."
echo "   Optional paths: make axolotl-qlora-journey, make axolotl-lora-journey, make unsloth-qlora-journey, make unsloth-lora-journey."
echo "   Apple Silicon print pointer: make mlx-lm-lora-journey. This checklist does not run it. Not native MLX."
echo "   Host and stack picker: make purpose-build-pick (operator section 17). This checklist does not run it."
echo "   Prepare against a throwaway copy. examples/estate.yaml stays the hash-locked example."
echo "2. Train handoff, train-next style. Print the NEXT.md recipe. SKIP live train."
echo "   Print check (does not train): make train-next or make train-next-lora. This checklist does not run them."
echo "   CELL_TRAIN_LIVE=1 stays print-only."
echo "3. Merge and export print honesty. $prefix enrich merge-adapt"
echo "   This checklist does not merge and does not export."
echo "4. $prefix enrich gguf-convert"
echo "   This checklist does not convert."
echo "5. $prefix enrich local-seat"
echo "   local-seat is print-only. It prints the Modelfile and does not write \$PREPARED/Modelfile."
echo "   Write that file from the printed contents before ollama create. This checklist does not write it."
echo "   CELL_SEAT_LIVE=1 stays print-only. This checklist does not shell out to ollama."
echo "6. $prefix enrich import-trained"
echo "   trained_shape gguf. auto_apply=false. This checklist does not import and does not promote."
echo "7. Standing next (estate). Print only. This checklist does not execute it."

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
Standing next (estate) — after step 6 (import-trained, trained_shape gguf, auto_apply=false):
Print-only. READY_FOR_LIVE_TEST: no
Purpose-build on demand. This print is operator UX. It is not a re-prove.
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
4. Recorded PASS stays in docs/LIVE-PROBES.md section Target C live uniqueness (5090-class). This print is not a live PASS. That recorded PASS stays the only live uniqueness prove. Re-prove card: make uniqueness-prove-checklist.
The factory does not train, convert, shell out to ollama, or promote.
The factory does not apply the estate.
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
coda_require "only live uniqueness prove"
coda_require "READY_FOR_LIVE_TEST: no"
coda_require "The factory does not train, convert, shell out to ollama, or promote."
coda_require "The factory does not apply the estate."
coda_require "This checklist does not write a Modelfile and does not shell out to ollama."
coda_require "CELL_TRAIN_LIVE=1 stays print-only."
coda_require "CELL_SEAT_LIVE=1 stays print-only."
coda_require "Not native MLX."
coda_require "Purpose-build on demand."
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
echo "Purpose-build card: make purpose-build-checklist."
