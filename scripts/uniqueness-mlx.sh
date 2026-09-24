#!/usr/bin/env bash
# Print-only mlx-lm LoRA uniqueness chain. Apple Silicon affinity only.
# Runs the prepare-assert phase, then the seat-print phase, of
# make mlx-lm-lora-journey. If the prepare phase fails, this script
# exits nonzero before the seat print.
# Does not train, fuse, convert, seat, or promote.
# Does not call mlx-lm, ollama, or llama.cpp.
# Does not run make unsloth-qlora-journey, make uniqueness-unsloth,
# make unsloth-lora-journey, make uniqueness-unsloth-lora,
# make qlora-journey, make seat-journey, make train-next,
# make axolotl-qlora-journey, or make lf-beachhead-prepare.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

echo "== uniqueness-mlx (print-only mlx-lm LoRA chain) =="
echo "Print-only. READY_FOR_LIVE_TEST: no"
echo "Apple Silicon affinity only. The stock overnight pack is refuse:host."
echo "Does not train, fuse, convert, seat, or promote."
echo "Does not call mlx-lm, ollama, or llama.cpp."
echo "Chain: prepare-assert, then seat-print (make mlx-lm-lora-journey)."
echo "Does not run make unsloth-qlora-journey, make uniqueness-unsloth, make qlora-journey, make seat-journey, make train-next, make axolotl-qlora-journey, or make lf-beachhead-prepare."
echo

echo "-- prepare-assert (MLX_LM_LORA_PHASE=prepare) --"
if ! MLX_LM_LORA_PHASE=prepare make -C "$ROOT" mlx-lm-lora-journey; then
  echo "FAIL  uniqueness-mlx (prepare-assert)" >&2
  exit 1
fi

echo
echo "-- seat-print (MLX_LM_LORA_PHASE=seat) --"
if ! MLX_LM_LORA_PHASE=seat make -C "$ROOT" mlx-lm-lora-journey; then
  echo "FAIL  uniqueness-mlx (seat-print)" >&2
  exit 1
fi

echo
echo "Live train, live fuse, and live seat still need a human Apple Silicon host and stay skipped."
echo "PASS  uniqueness-mlx (prepare-assert then seat-print; SKIP live train; SKIP live convert; SKIP live seat)"
echo "READY_FOR_LIVE_TEST: no"
