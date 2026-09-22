#!/usr/bin/env bash
# Enrich prepare fixture: example pack -> Modelfile, proposal, staged plan, require-plan apply.
# Throwaway dir. No Ollama binary. No live train. No GPU.
# Local only. Do not add to make smoke or GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

unset XAI_API_KEY CELL_FRONTIER_ENDPOINT CELL_LOCAL_ENDPOINT CELL_RENTED_ENDPOINT

# Throwaway dir stays out of the checkout. prepare refuses a hardware SKU
# anywhere in the output path, including a parent directory name.
WORKDIR="${WORKDIR:-${TMPDIR:-/tmp}/cell-one-enrich-prepare}"
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
PACK="${PACK:-$ROOT/examples/fixtures/specialist-overnight.pack.json}"
BIN="${ESTATE_BIN:-}"

estate() {
  if [[ -n "$BIN" ]]; then
    "$BIN" "$@"
  else
    cargo run -q -p estate-control -- "$@"
  fi
}

if [[ ! -f "$PACK" ]]; then
  echo "SKIP enrich-prepare (example pack missing; not a PASS)"
  exit 0
fi

BEFORE="$(cksum "$ESTATE")"
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"
SEATED="$WORKDIR/estate.yaml"
python3 - "$ESTATE" "$SEATED" <<'PY'
import sys
src, dest = sys.argv[1:]
text = open(src).read()
needle = "  - id: local_slm\n    class: local\n    driver: ollama\n    params:\n"
if needle not in text:
    raise SystemExit("FAIL  local_slm params block missing")
open(dest, "w").write(text.replace(needle, needle + '      model: "llama3"\n', 1))
PY

echo "== enrich-prepare (fixtures only; not a live train) =="
echo "workdir: $WORKDIR"
echo "SKIP live train (prepare writes artifacts only; not a PASS)"

echo "-- stock estate refuses a binding-id FROM --"
set +e
estate enrich prepare \
  --estate "$ESTATE" \
  --pack "$PACK" \
  --driver ollama-modelfile \
  --out "$WORKDIR/stock" \
  >/tmp/enrich-prepare-stock.out 2>/tmp/enrich-prepare-stock.err
stock_rc=$?
set -e
if [[ "$stock_rc" -eq 0 ]]; then
  echo "FAIL  stock estate must refuse:base-model"
  exit 1
fi
if ! grep -q "refuse:base-model" /tmp/enrich-prepare-stock.out /tmp/enrich-prepare-stock.err; then
  echo "FAIL  stock estate did not refuse:base-model"
  cat /tmp/enrich-prepare-stock.out /tmp/enrich-prepare-stock.err
  exit 1
fi
if [[ -e "$WORKDIR/stock" ]]; then
  echo "FAIL  stock estate wrote an output directory"
  exit 1
fi

echo "-- ollama-modelfile --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver ollama-modelfile \
  --job enrich \
  --out "$WORKDIR/ollama"

echo "-- external-manifest --"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --driver external-manifest \
  --job enrich \
  --out "$WORKDIR/manifest"

test -f "$WORKDIR/ollama/Modelfile"
test -f "$WORKDIR/ollama/PREPARE.md"
test -f "$WORKDIR/ollama/NEXT.md"
test -f "$WORKDIR/ollama/prepare.json"
grep -q "FROM llama3" "$WORKDIR/ollama/Modelfile"
if grep -q "FROM local_slm" "$WORKDIR/ollama/Modelfile"; then
  echo "FAIL  Modelfile FROM is the binding id"
  exit 1
fi
grep -q "ollama create cell-enrich-overnight-traces -f Modelfile" "$WORKDIR/ollama/PREPARE.md"
grep -q "ollama create cell-enrich-overnight-traces -f $WORKDIR/ollama/Modelfile" "$WORKDIR/ollama/NEXT.md"
grep -q "import-prepared" "$WORKDIR/ollama/NEXT.md"

test -f "$WORKDIR/manifest/manifest.json"
test -f "$WORKDIR/manifest/manifest.yaml"
test -f "$WORKDIR/manifest/PREPARE.md"
test -f "$WORKDIR/manifest/NEXT.md"
test -f "$WORKDIR/manifest/prepare.json"
if grep -q "ollama create" "$WORKDIR/manifest/NEXT.md"; then
  echo "FAIL  external NEXT.md must not name ollama create"
  exit 1
fi

python3 - "$WORKDIR/manifest/manifest.json" "$WORKDIR/ollama/prepare.json" <<'PY'
import json, sys
manifest = json.load(open(sys.argv[1]))
prepare = json.load(open(sys.argv[2]))
if manifest.get("vendor") is not None:
    raise SystemExit(f"FAIL  vendor lock: {manifest.get('vendor')}")
if manifest.get("promoted") is not False or manifest.get("auto_apply") is not False:
    raise SystemExit("FAIL  manifest must stay unpromoted")
if manifest.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  manifest claims an estate rewrite")
if "ollama" in json.dumps(manifest).lower():
    raise SystemExit("FAIL  external manifest names ollama")
if prepare.get("schema") != "cell-one.enrich-prepare.v0":
    raise SystemExit(f"FAIL  schema={prepare.get('schema')}")
if prepare.get("promoted") is not False or prepare.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  prepare.json must stay unpromoted and off the estate")
if "Modelfile" not in prepare.get("artifacts", []):
    raise SystemExit(f"FAIL  artifacts={prepare.get('artifacts')}")
PY

echo "-- missing pack refuses before write --"
set +e
estate enrich prepare \
  --estate "$SEATED" \
  --pack missing-enrich-pack \
  --packs-dir "$WORKDIR/none" \
  --out "$WORKDIR/missing" \
  >/tmp/enrich-prepare-missing.out 2>/tmp/enrich-prepare-missing.err
missing_rc=$?
set -e
if [[ "$missing_rc" -eq 0 ]]; then
  echo "FAIL  missing pack must refuse"
  exit 1
fi
if ! grep -q "refuse:missing-pack" /tmp/enrich-prepare-missing.out /tmp/enrich-prepare-missing.err; then
  echo "FAIL  missing pack did not refuse:missing-pack"
  cat /tmp/enrich-prepare-missing.out /tmp/enrich-prepare-missing.err
  exit 1
fi
if [[ -d "$WORKDIR/missing" ]]; then
  echo "FAIL  missing pack wrote an output directory"
  exit 1
fi

echo "-- all-drivers, list, import-prepared --"
CELL="$WORKDIR/cell"
estate enrich prepare \
  --estate "$SEATED" \
  --pack "$PACK" \
  --all-drivers \
  --state-dir "$CELL"

test -f "$CELL/enrich/overnight-traces/ollama-modelfile/Modelfile"
test -f "$CELL/enrich/overnight-traces/ollama-modelfile/NEXT.md"
test -f "$CELL/enrich/overnight-traces/external-manifest/manifest.json"
test -f "$CELL/enrich/overnight-traces/external-manifest/NEXT.md"

estate enrich list --state-dir "$CELL" | tee "$WORKDIR/list.out"
grep -q "pack=overnight-traces driver=external-manifest" "$WORKDIR/list.out"
grep -q "pack=overnight-traces driver=ollama-modelfile" "$WORKDIR/list.out"
grep -q "tag=cell-enrich-overnight-traces" "$WORKDIR/list.out"
grep -q "count=2" "$WORKDIR/list.out"

set +e
estate enrich list --state-dir "$WORKDIR/missing-cell" \
  >/tmp/enrich-list-missing.out 2>/tmp/enrich-list-missing.err
list_rc=$?
set -e
if [[ "$list_rc" -eq 0 ]]; then
  echo "FAIL  missing enrich dir must refuse"
  exit 1
fi
if ! grep -q "refuse:enrich-index" /tmp/enrich-list-missing.out /tmp/enrich-list-missing.err; then
  echo "FAIL  missing enrich dir did not refuse:enrich-index"
  cat /tmp/enrich-list-missing.out /tmp/enrich-list-missing.err
  exit 1
fi
if [[ -d "$WORKDIR/missing-cell" ]]; then
  echo "FAIL  list created a missing state dir"
  exit 1
fi

PREPARED="$CELL/enrich/overnight-traces/ollama-modelfile"
set +e
estate enrich import-prepared \
  --estate "$SEATED" \
  --prepared "$PREPARED" \
  --tag other-tag \
  --path "$PREPARED/Modelfile" \
  >/tmp/enrich-import-tag.out 2>/tmp/enrich-import-tag.err
tag_rc=$?
set -e
if [[ "$tag_rc" -eq 0 ]]; then
  echo "FAIL  wrong tag must refuse"
  exit 1
fi
if ! grep -q "refuse:tag" /tmp/enrich-import-tag.out /tmp/enrich-import-tag.err; then
  echo "FAIL  wrong tag did not refuse:tag"
  cat /tmp/enrich-import-tag.out /tmp/enrich-import-tag.err
  exit 1
fi
if [[ -f "$PREPARED/binding-proposal.json" ]]; then
  echo "FAIL  wrong tag wrote a binding proposal"
  exit 1
fi

estate enrich import-prepared \
  --estate "$SEATED" \
  --prepared "$PREPARED" \
  --tag cell-enrich-overnight-traces \
  --path "$PREPARED/Modelfile" \
  | tee "$WORKDIR/import.out"
test -f "$PREPARED/binding-proposal.json"
test -f "$PREPARED/binding-proposal.md"
grep -q "auto_apply=false" "$WORKDIR/import.out"
grep -q "import-prepared did not apply" "$WORKDIR/import.out"
grep -q "local_slm" "$PREPARED/binding-proposal.md"

python3 - "$PREPARED/binding-proposal.json" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
if doc.get("schema") != "cell-one.enrich-binding-proposal.v0":
    raise SystemExit(f"FAIL  schema={doc.get('schema')}")
if doc.get("auto_apply") is not False or doc.get("promoted") is not False:
    raise SystemExit("FAIL  binding proposal must stay unapplied")
if doc.get("estate_rewritten") is not False:
    raise SystemExit("FAIL  binding proposal claims an estate rewrite")
if doc.get("binding_id") != "local_slm":
    raise SystemExit(f"FAIL  binding_id={doc.get('binding_id')}")
params = doc.get("proposed_binding", {}).get("params", {})
if params.get("model") != "cell-enrich-overnight-traces":
    raise SystemExit(f"FAIL  model={params.get('model')}")
PY

echo "-- apply-proposal, plan, require-plan apply --"
# The proposal hash is the seated copy. Stage and require-plan apply that
# same content. examples/estate.yaml stays the hash-locked file.
LAB="$WORKDIR/lab-estate.yaml"
cp "$SEATED" "$LAB"
LAB_BEFORE="$(cksum "$LAB")"
MANIFEST="$CELL/enrich/overnight-traces/external-manifest"

set +e
estate enrich apply-proposal \
  --estate "$LAB" \
  --prepared "$MANIFEST" \
  --tag cell-enrich-overnight-traces \
  --state-dir "$CELL" \
  --plans-dir "$WORKDIR/plans" \
  >/tmp/enrich-apply-missing.out 2>/tmp/enrich-apply-missing.err
missing_proposal_rc=$?
set -e
if [[ "$missing_proposal_rc" -eq 0 ]]; then
  echo "FAIL  missing proposal must refuse"
  exit 1
fi
if ! grep -q "refuse:missing-proposal" /tmp/enrich-apply-missing.out /tmp/enrich-apply-missing.err; then
  echo "FAIL  missing proposal did not refuse:missing-proposal"
  cat /tmp/enrich-apply-missing.out /tmp/enrich-apply-missing.err
  exit 1
fi
if [[ -d "$CELL/enrich-stage" ]]; then
  echo "FAIL  missing proposal wrote an enrich stage"
  exit 1
fi

set +e
estate enrich apply-proposal \
  --estate "$LAB" \
  --prepared "$PREPARED" \
  --tag cell-enrich-overnight-traces \
  --state-dir "$WORKDIR/verify-state" \
  --plans-dir "$WORKDIR/plans" \
  --verify-local-tag \
  >/tmp/enrich-apply-verify.out 2>/tmp/enrich-apply-verify.err
verify_rc=$?
set -e
if [[ "$verify_rc" -eq 0 ]]; then
  echo "FAIL  verify-local-tag must refuse when the endpoint is unset"
  exit 1
fi
if ! grep -q "refuse:local-tag" /tmp/enrich-apply-verify.out /tmp/enrich-apply-verify.err; then
  echo "FAIL  verify-local-tag did not refuse:local-tag"
  cat /tmp/enrich-apply-verify.out /tmp/enrich-apply-verify.err
  exit 1
fi
if [[ -d "$WORKDIR/verify-state/enrich-stage" ]]; then
  echo "FAIL  failed verify wrote an enrich stage"
  exit 1
fi

estate enrich apply-proposal \
  --estate "$LAB" \
  --prepared "$PREPARED" \
  --tag cell-enrich-overnight-traces \
  --state-dir "$CELL" \
  --plans-dir "$WORKDIR/plans" \
  | tee "$WORKDIR/apply-proposal.out"
grep -q "apply-proposal did not apply" "$WORKDIR/apply-proposal.out"
grep -q "auto_apply=false" "$WORKDIR/apply-proposal.out"
grep -q "require-plan" "$WORKDIR/apply-proposal.out"
test -f "$CELL/enrich-stage/staged-estate.yaml"
test -f "$CELL/enrich-stage/stage.json"
if [[ "$(cksum "$LAB")" != "$LAB_BEFORE" ]]; then
  echo "FAIL  apply-proposal rewrote the lab estate"
  exit 1
fi

estate enrich apply-proposal \
  --estate "$LAB" \
  --prepared "$PREPARED" \
  --tag cell-enrich-overnight-traces \
  --state-dir "$CELL" \
  --plans-dir "$WORKDIR/plans" \
  | tee "$WORKDIR/apply-proposal-again.out"
grep -q "no-op:" "$WORKDIR/apply-proposal-again.out"

estate status \
  --estate "$LAB" \
  --state-dir "$CELL" \
  --plans-dir "$WORKDIR/plans" \
  --root "$ROOT" \
  | tee "$WORKDIR/status-pending.out"
grep -q "enrich_binding: pending" "$WORKDIR/status-pending.out"
grep -q "enrich_stage: applied=false" "$WORKDIR/status-pending.out"

estate doctor --root "$ROOT" --state-dir "$CELL" | tee "$WORKDIR/doctor-pending.out"
grep -q "source estate not written" "$WORKDIR/doctor-pending.out"
grep -q "binding proposal" "$WORKDIR/doctor-pending.out"

STAGED="$CELL/enrich-stage/staged-estate.yaml"
estate plan \
  --estate "$STAGED" \
  --state-dir "$CELL" \
  --plans-dir "$WORKDIR/plans" \
  | tee "$WORKDIR/plan.out"
grep -q "local_slm" "$WORKDIR/plan.out"

estate apply \
  --dry-run \
  --require-plan \
  --estate "$STAGED" \
  --state-dir "$CELL" \
  --plans-dir "$WORKDIR/plans" \
  --roots-base "$WORKDIR"
if [[ "$(cksum "$LAB")" != "$LAB_BEFORE" ]]; then
  echo "FAIL  dry-run rewrote the lab estate"
  exit 1
fi

estate apply \
  --estate "$STAGED" \
  --state-dir "$CELL" \
  --plans-dir "$WORKDIR/plans" \
  --roots-base "$WORKDIR" \
  | tee "$WORKDIR/apply-held.out"
grep -q "enrich stage held" "$WORKDIR/apply-held.out"
if [[ "$(cksum "$LAB")" != "$LAB_BEFORE" ]]; then
  echo "FAIL  apply without --require-plan rewrote the lab estate"
  exit 1
fi

estate apply \
  --require-plan \
  --estate "$STAGED" \
  --state-dir "$CELL" \
  --plans-dir "$WORKDIR/plans" \
  --roots-base "$WORKDIR" \
  | tee "$WORKDIR/apply.out"
grep -q "enrich stage wrote" "$WORKDIR/apply.out"
grep -q "cell-enrich-overnight-traces" "$LAB"

estate status \
  --estate "$LAB" \
  --state-dir "$CELL" \
  --plans-dir "$WORKDIR/plans" \
  --root "$ROOT" \
  | tee "$WORKDIR/status-joined.out"
grep -q "enrich_binding: local_slm model=cell-enrich-overnight-traces" "$WORKDIR/status-joined.out"
grep -q "enrich_stage: applied=true" "$WORKDIR/status-joined.out"

python3 - "$CELL/enrich-stage/stage.json" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
if doc.get("schema") != "cell-one.enrich-binding-stage.v0":
    raise SystemExit(f"FAIL  stage schema={doc.get('schema')}")
if doc.get("auto_apply") is not False or doc.get("promoted") is not False:
    raise SystemExit("FAIL  stage must stay manual")
if doc.get("applied") is not True or doc.get("estate_rewritten") is not True:
    raise SystemExit("FAIL  require-plan apply did not record the source write")
if doc.get("binding_id") != "local_slm":
    raise SystemExit(f"FAIL  stage binding_id={doc.get('binding_id')}")
PY

AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  enrich-prepare rewrote the estate"
  exit 1
fi

echo "PASS  enrich-prepare (staged join applied on the throwaway estate; not a live train)"
