#!/usr/bin/env bash
# Opt-in print-only prepare walk of the LLaMA-Factory beachhead matrix.
# Reads docs/lf-beachhead-matrix.md and runs estate enrich prepare once
# per smoke fixture, on a throwaway copy of examples/estate.yaml.
# Does not train, merge, convert, create an Ollama model, or promote.
# Local only. Do not add to make smoke, make gate-90, or GitHub Actions.
# Resolves estate fail-closed: executable ESTATE_BIN, then target/release/estate,
# then target/debug/estate, then cargo on PATH. Does not invent a binary.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

# Throwaway dir stays out of the checkout. prepare refuses a hardware SKU
# anywhere in the output path, including a parent directory name.
WORKDIR="${WORKDIR:-${TMPDIR:-/tmp}/cell-one-lf-beachhead-prepare}"
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
MATRIX="${MATRIX:-$ROOT/docs/lf-beachhead-matrix.md}"
BIN="${ESTATE_BIN:-}"

# Phi-3-small is the tip rule, not a matrix row. The QLoRA reproduce line
# covers template phi_small. The LoRA reproduce line does not.
PHI_SMALL_BASE="microsoft/Phi-3-small-8k-instruct"
PHI_QLORA_NOTE="Reproduce target beside Qwen LoRA/QLoRA."
PHI_LORA_NOTE="Reproduce target on the unquantized LoRA card, the non-quant twin of the Phi-3 Instruct QLoRA prepare."

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

estate() {
  resolve_estate
  "${ESTATE_CMD[@]}" "$@"
}

# Same column split as the beachhead lock: six cells, backticks stripped.
parse_matrix() {
  python3 - "$MATRIX" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read()
rows = []
for raw in text.splitlines():
    line = raw.strip()
    if not line.startswith("| ") or "---" in line or "Family |" in line:
        continue
    cells = [cell.strip().strip("`") for cell in line.strip("|").split("|")]
    if len(cells) != 6:
        raise SystemExit(f"FAIL  matrix row must keep six columns: {line}")
    rows.append(cells)
if not rows:
    raise SystemExit("FAIL  matrix has no beachhead rows")
seen = []
for cells in rows:
    card = cells[1]
    fixture = cells[5]
    if card not in ("llamafactory-lora", "llamafactory-qlora"):
        raise SystemExit(f"FAIL  card {card} is not a LLaMA-Factory beachhead driver")
    if not fixture.endswith(".pack.json"):
        raise SystemExit(f"FAIL  fixture is not a pack: {fixture}")
    if fixture in seen:
        raise SystemExit(f"FAIL  duplicate fixture {fixture}")
    seen.append(fixture)
    print("\t".join(cells))
PY
}

if [[ "${1:-}" == "--list" ]]; then
  parse_matrix
  exit 0
fi

if [[ ! -f "$MATRIX" ]]; then
  echo "FAIL  beachhead matrix missing: $MATRIX"
  exit 1
fi
if [[ ! -f "$ESTATE" ]]; then
  echo "FAIL  example estate missing: $ESTATE"
  exit 1
fi

echo "== lf-beachhead-prepare (LLaMA-Factory beachhead matrix; print-only) =="
echo "workdir: $WORKDIR"
echo "matrix: $MATRIX"
echo "READY_FOR_LIVE_TEST: no"
echo "SKIP live train"
echo

resolve_estate

BEFORE="$(cksum "$ESTATE")"
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR/logs" "$WORKDIR/rows"
SEATED="$WORKDIR/estate.yaml"
if [[ "$SEATED" == "$ESTATE" ]]; then
  echo "FAIL  throwaway estate must not be examples/estate.yaml"
  exit 1
fi
python3 - "$ESTATE" "$SEATED" <<'PY'
import sys
src, seated = sys.argv[1:]
text = open(src, encoding="utf-8").read()
needle = "  - id: local_slm\n    class: local\n    driver: ollama\n    params:\n"
if needle not in text:
    raise SystemExit("FAIL  local_slm params block missing")
# Seat tag only. Each pack's train_base_model is the train base.
open(seated, "w", encoding="utf-8").write(
    text.replace(needle, needle + '      model: "llama3"\n', 1)
)
PY

ROWS_FILE="$WORKDIR/rows.tsv"
parse_matrix >"$ROWS_FILE"

assert_prepare() {
  local prepared="$1" card="$2" train_base="$3" seat="$4" template="$5" knobs="$6"
  python3 - "$prepared" "$card" "$train_base" "$seat" "$template" "$knobs" <<'PY'
import json, re, sys
from pathlib import Path
prepared, card, train_base, seat, template, knobs = sys.argv[1:]
root = Path(prepared)
prepare = json.loads((root / "prepare.json").read_text(encoding="utf-8"))
if prepare.get("schema") != "cell-one.enrich-prepare.v0":
    raise SystemExit(f"FAIL  schema={prepare.get('schema')}")
if prepare.get("driver") != card:
    raise SystemExit(f"FAIL  driver={prepare.get('driver')} expected {card}")
if prepare.get("job") != "train":
    raise SystemExit(f"FAIL  job={prepare.get('job')}")
if prepare.get("base_model") != seat or prepare.get("seat_tag") != seat:
    raise SystemExit(
        f"FAIL  seat={prepare.get('base_model')}/{prepare.get('seat_tag')} expected {seat}"
    )
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

def lines(name):
    return (root / name).read_text(encoding="utf-8").splitlines()

def has(text_lines, exact):
    return any(line.strip() == exact for line in text_lines)

recipe = lines("recipe.yaml")
export = lines("export.yaml")
quoted = f'model_name_or_path: "{train_base}"'
if not has(recipe, f"template: {template}"):
    raise SystemExit(f"FAIL  recipe template is not {template}")
if not has(export, f"template: {template}"):
    raise SystemExit(f"FAIL  export template is not {template}")
if not has(recipe, quoted) or not has(export, quoted):
    raise SystemExit(f"FAIL  model_name_or_path is not {train_base}")
rank = re.search(r"\brank (\d+)\b", knobs)
if not rank:
    raise SystemExit(f"FAIL  knobs missing rank: {knobs}")
if not has(recipe, f"lora_rank: {rank.group(1)}"):
    raise SystemExit(f"FAIL  lora_rank is not {rank.group(1)}")
if "packing true" in knobs:
    packing = "true"
elif "packing false" in knobs:
    packing = "false"
else:
    raise SystemExit(f"FAIL  knobs missing packing: {knobs}")
if not has(recipe, f"packing: {packing}"):
    raise SystemExit(f"FAIL  packing is not {packing}")

def yaml_key(text_lines, key):
    return any(line.strip().startswith(key + ":") or line.strip().startswith(key + " ") for line in text_lines)

recipe_text = "\n".join(recipe)
export_text = "\n".join(export)
for key, value in (("quantization_bit", "4"), ("quantization_method", "bnb")):
    if f"no {key}" in knobs:
        if key in recipe_text or key in export_text:
            raise SystemExit(f"FAIL  {key} must be absent")
    elif f"{key} {value}" in knobs:
        if not has(recipe, f"{key}: {value}"):
            raise SystemExit(f"FAIL  recipe missing {key}: {value}")
    else:
        raise SystemExit(f"FAIL  knobs missing {key} rule: {knobs}")
if yaml_key(export, "quantization_bit") or yaml_key(export, "quantization_method"):
    raise SystemExit("FAIL  export.yaml must omit quantization keys")
PY
}

count=0
while IFS=$'\t' read -r family card train_base template knobs fixture; do
  [[ -n "$family" ]] || continue
  pack="$ROOT/$fixture"
  if [[ ! -f "$pack" ]]; then
    echo "FAIL  missing fixture $fixture"
    exit 1
  fi
  seat="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["model_hint"])' "$pack" < /dev/null)"
  pack_base="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["train_base_model"])' "$pack" < /dev/null)"
  if [[ -z "$seat" ]]; then
    echo "FAIL  $fixture has no model_hint seat tag"
    exit 1
  fi
  if [[ "$pack_base" != "$train_base" ]]; then
    echo "FAIL  $fixture train_base_model=$pack_base matrix=$train_base"
    exit 1
  fi
  slug="$(basename "$fixture" .pack.json)"
  prepared="$WORKDIR/rows/$slug"
  echo "-- $family / $card / $fixture --"
  if ! estate enrich prepare \
    --estate "$SEATED" \
    --pack "$pack" \
    --driver "$card" \
    --job train \
    --out "$prepared" \
    < /dev/null \
    >"$WORKDIR/logs/${slug}.out" 2>"$WORKDIR/logs/${slug}.err"; then
    echo "FAIL  prepare $fixture ($card)"
    cat "$WORKDIR/logs/${slug}.out" "$WORKDIR/logs/${slug}.err"
    exit 1
  fi
  test -f "$prepared/prepare.json"
  test -f "$prepared/recipe.yaml"
  test -f "$prepared/export.yaml"
  if [[ -e "$prepared/export.gguf" || -e "$prepared/train_unsloth.py" || -e "$prepared/convert_hf_to_gguf.py" ]]; then
    echo "FAIL  prepare wrote a train, convert, or seat artifact"
    exit 1
  fi
  assert_prepare "$prepared" "$card" "$train_base" "$seat" "$template" "$knobs"
  count=$((count + 1))
  echo "ok  $fixture  $card  template=$template  seat=$seat  train_base=$train_base"
  echo "SKIP live train"
done <"$ROWS_FILE"

if [[ "$count" -lt 1 ]]; then
  echo "FAIL  walked no matrix fixtures"
  exit 1
fi
echo
echo "walked $count matrix fixtures"
echo

echo "-- Phi-3-small stays QLoRA-only (not a matrix row) --"
grep -F -q "Phi-3-small stays template \`phi_small\`." "$MATRIX"
grep -F -q "The QLoRA reproduce line still covers that id." "$MATRIX"
grep -F -q "The LoRA row does not." "$MATRIX"
PHI_PACK="$ROOT/examples/fixtures/phi3-instruct.pack.json"
PHI_SMALL_PACK="$WORKDIR/phi3-small-rule.pack.json"
python3 - "$PHI_PACK" "$PHI_SMALL_PACK" "$PHI_SMALL_BASE" <<'PY'
import json, sys
src, dest, train_base = sys.argv[1:]
pack = json.load(open(src, encoding="utf-8"))
pack["train_base_model"] = train_base
json.dump(pack, open(dest, "w", encoding="utf-8"), indent=2)
open(dest, "a", encoding="utf-8").write("\n")
PY

phi_small_prepare() {
  local card="$1" out="$2" knobs="$3"
  if ! estate enrich prepare \
    --estate "$SEATED" \
    --pack "$PHI_SMALL_PACK" \
    --driver "$card" \
    --job train \
    --out "$out" \
    < /dev/null \
    >"$WORKDIR/logs/phi-small-${card}.out" 2>"$WORKDIR/logs/phi-small-${card}.err"; then
    echo "FAIL  Phi-3-small prepare $card"
    cat "$WORKDIR/logs/phi-small-${card}.out" "$WORKDIR/logs/phi-small-${card}.err"
    exit 1
  fi
  assert_prepare "$out" "$card" "$PHI_SMALL_BASE" "llama3" "phi_small" "$knobs"
}

phi_small_prepare "llamafactory-qlora" "$WORKDIR/phi-small-qlora" "rank 16, packing true, quantization_method bnb, quantization_bit 4"
grep -F -q "$PHI_QLORA_NOTE" "$WORKDIR/phi-small-qlora/NEXT.md"
grep -F -q "$PHI_QLORA_NOTE" "$WORKDIR/phi-small-qlora/PREPARE.md"
grep -F -q "phi_small" "$WORKDIR/phi-small-qlora/NEXT.md"
if grep -F -q "$PHI_LORA_NOTE" "$WORKDIR/phi-small-qlora/NEXT.md" "$WORKDIR/phi-small-qlora/PREPARE.md"; then
  echo "FAIL  Phi-3-small QLoRA wrote the LoRA reproduce line"
  exit 1
fi

phi_small_prepare "llamafactory-lora" "$WORKDIR/phi-small-lora" "rank 8, packing false, no quantization_bit, no quantization_method"
if grep -F -q "$PHI_LORA_NOTE" "$WORKDIR/phi-small-lora/NEXT.md" "$WORKDIR/phi-small-lora/PREPARE.md"; then
  echo "FAIL  Phi-3-small LoRA got the Phi-3 Instruct LoRA reproduce line"
  exit 1
fi
if grep -F -q "$PHI_QLORA_NOTE" "$WORKDIR/phi-small-lora/NEXT.md" "$WORKDIR/phi-small-lora/PREPARE.md"; then
  echo "FAIL  Phi-3-small LoRA got the QLoRA reproduce line"
  exit 1
fi
echo "ok  Phi-3-small template=phi_small QLoRA-only"
echo "SKIP live train"
echo

AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  lf-beachhead-prepare rewrote examples/estate.yaml"
  exit 1
fi

echo "PASS  lf-beachhead-prepare ($count matrix fixtures; SKIP live train)"
echo "READY_FOR_LIVE_TEST: no"
