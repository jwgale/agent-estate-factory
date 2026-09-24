#!/usr/bin/env bash
# Print-only purpose-build picker.
# Short host and stack table. Points at make targets already on tip.
# Does not run them. Does not resolve or execute estate.
# Does not train, fuse, convert, shell out to ollama, promote, or apply the estate.
# Does not invent a live PASS. The recorded 5090-class Target C uniqueness
# PASS in docs/LIVE-PROBES.md stays the only live uniqueness prove.
# The ordered steps stay make purpose-build-checklist. This picker does not run it.
# The re-prove card stays make uniqueness-prove-checklist. This picker does not run it.
# CELL_TRAIN_LIVE=1 stays print-only. CELL_SEAT_LIVE=1 stays print-only.
# Not native MLX.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

echo "== purpose-build-pick (print-only host and stack picker) =="
echo "Print-only. READY_FOR_LIVE_TEST: no"
echo "Purpose-build on demand. This picker is operator UX."
echo "It names make targets already on tip. It does not run them."
echo "It does not resolve or execute estate."
echo "It is not a re-prove of the recorded Target C live PASS."
echo "The factory does not train, fuse, convert, shell out to ollama, or promote."
echo "The factory does not apply the estate."
echo "CELL_TRAIN_LIVE=1 stays print-only."
echo "CELL_SEAT_LIVE=1 stays print-only."
echo "Not native MLX."
echo "Not in make smoke, make gate-90, or GitHub Actions."
echo "Recorded PASS: docs/LIVE-PROBES.md section Target C live uniqueness (5090-class)."
echo "That recorded PASS stays the only live uniqueness prove."
echo "This picker does not invent a new live PASS."
echo "Ordered steps after the pick: make purpose-build-checklist. This picker does not run it."
echo "The re-prove card stays make uniqueness-prove-checklist. This picker does not run it."
echo "examples/estate.yaml stays unchanged."

if [[ "${CELL_TRAIN_LIVE:-}" == "1" ]]; then
  echo "CELL_TRAIN_LIVE=1 is set. This journey stays print-only."
fi
if [[ "${CELL_SEAT_LIVE:-}" == "1" ]]; then
  echo "CELL_SEAT_LIVE=1 is set. This journey stays print-only."
fi

sum="$(cksum "$ROOT/examples/estate.yaml")"
echo "cksum: $sum"
case "$sum" in
  "43770130 3391"*) ;;
  *)
    echo "FAIL  examples/estate.yaml cksum is not 43770130 3391: $sum" >&2
    exit 1
    ;;
esac

echo
echo "Branching table. Each make target is a name. This picker does not execute it."
echo
printf '%-22s %-14s %s\n' "Host / stack" "Role" "Make target (not executed)"
printf '%-22s %-14s %s\n' "Nvidia / CUDA" "primary" "make lf-beachhead-prepare"
printf '%-22s %-14s %s\n' "Nvidia / CUDA" "primary" "make qlora-journey"
printf '%-22s %-14s %s\n' "Nvidia / CUDA" "primary" "make uniqueness-full"
printf '%-22s %-14s %s\n' "Nvidia / CUDA" "optional" "make unsloth-qlora-journey"
printf '%-22s %-14s %s\n' "Nvidia / CUDA" "optional" "make uniqueness-unsloth"
printf '%-22s %-14s %s\n' "Nvidia / CUDA" "integration" "make axolotl-qlora-journey"
printf '%-22s %-14s %s\n' "Nvidia / CUDA" "integration" "make uniqueness-axolotl"
printf '%-22s %-14s %s\n' "Apple Silicon" "optional" "make mlx-lm-lora-journey"
printf '%-22s %-14s %s\n' "Apple Silicon" "optional" "make uniqueness-mlx"
printf '%-22s %-14s %s\n' "Apple Silicon" "refuse:host" "stock any-affinity packs (mlx-lm-lora writes nothing)"
printf '%-22s %-14s %s\n' "Target A LoRA twin" "primary" "make lora-journey"
printf '%-22s %-14s %s\n' "Target A LoRA twin" "primary" "make uniqueness-full-lora"
printf '%-22s %-14s %s\n' "Target A LoRA twin" "optional" "make unsloth-lora-journey"
printf '%-22s %-14s %s\n' "Target A LoRA twin" "optional" "make uniqueness-unsloth-lora"
printf '%-22s %-14s %s\n' "Target A LoRA twin" "integration" "make axolotl-lora-journey"
printf '%-22s %-14s %s\n' "Target A LoRA twin" "integration" "make uniqueness-axolotl-lora"
echo
echo "Nvidia / CUDA primary is the LLaMA-Factory beachhead, make qlora-journey, and make uniqueness-full."
echo "Unsloth on that host stays optional. Axolotl on that host stays integration."
echo "Apple Silicon is make mlx-lm-lora-journey and make uniqueness-mlx. Status stays optional."
echo "A stock pack whose host_class_affinity is any is refuse:host for mlx-lm-lora. Not native MLX."
echo "Target A LoRA twins are the pairs that already exist: LLaMA-Factory, Unsloth, and Axolotl."
echo "After the pick, the ordered steps are make purpose-build-checklist. This picker does not run it."
echo "Recorded PASS stays in docs/LIVE-PROBES.md. This print is not a live PASS."
echo "Purpose-build picker: make purpose-build-pick."
