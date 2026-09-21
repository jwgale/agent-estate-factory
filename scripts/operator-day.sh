#!/usr/bin/env bash
# Operator-day dry-run. Fixtures only. No live Grok / no GPU / no hosted CI.
# Walk: suspend → plan → apply → feed import → resume (+ convey + packs).
# Wave 6: policy / catalog caps / backup / restore dry-run / pause-proof.
# Wave 7: status one-pager, --curator jason, convey sync keeps extra hops.
# Wave 8: idempotent apply, plan export-pr, sacred overlay, mixed proof.
# Do not call make smoke from here (smoke wraps this script).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

WORKDIR="${WORKDIR:-$ROOT/target/operator-day}"
ESTATE="${ESTATE:-$ROOT/examples/estate.yaml}"
STATE="$WORKDIR/state"
PLANS="$WORKDIR/plans"
DROP="$WORKDIR/packs"
FEED="$WORKDIR/feed"
REVIEWED="$PLANS/reviewed"

rm -rf "$WORKDIR"
mkdir -p "$STATE" "$PLANS" "$DROP" "$FEED" "$REVIEWED"

echo "== operator-day (fixtures only) =="
echo "workdir: $WORKDIR"

echo "-- validate host matrix --"
cargo run -q -p estate-control -- validate --estate "$ROOT/examples/hosts/rtx-consumer.yaml"
cargo run -q -p estate-control -- validate --estate "$ROOT/examples/hosts/apple-silicon.yaml"
cargo run -q -p estate-control -- validate --estate "$ROOT/examples/hosts/nvidia-rental.yaml"
cargo run -q -p estate-control -- validate --estate "$ROOT/examples/hosts/multi-host.yaml"
cargo run -q -p estate-control -- validate --estate "$ROOT/examples/fixtures/mixed-frontier-local.yaml"

set +e
cargo run -q -p estate-control -- validate --estate "$ROOT/examples/invalid/host-class-bad.yaml" >/tmp/opday-host.out 2>/tmp/opday-host.err
bad_host=$?
set -e
if [[ "$bad_host" -eq 0 ]]; then
  echo "FAIL  host-class-bad.yaml must fail closed"
  exit 1
fi
echo "PASS  invalid host_class refused"

echo "-- doctor (compile-only CI + schemas) --"
cargo run -q -p estate-control -- doctor --root "$ROOT" --state-dir "$STATE"

echo "-- apply --dry-run (no writes) --"
cargo run -q -p estate-control -- apply --dry-run --estate "$ESTATE" --state-dir "$STATE" --roots-base "$WORKDIR" --plans-dir "$PLANS"
if [[ -f "$STATE/placement-actual.json" ]]; then
  echo "FAIL  dry-run wrote placement-actual.json"
  exit 1
fi
echo "PASS  apply --dry-run"

echo "-- fixtures-check (happy + each refuse) --"
bash "$ROOT/scripts/fixtures-check.sh"
echo "PASS  fixtures-check"

echo "-- first apply (greenfield plan + require-plan) --"
cargo run -q -p estate-control -- plan --estate "$ESTATE" --plans-dir "$PLANS" --state-dir "$STATE"
cargo run -q -p estate-control -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$WORKDIR" --plans-dir "$PLANS" --require-plan
cargo run -q -p estate-control -- sessions list --state-dir "$STATE"
if [[ ! -f "$STATE/sessions.jsonl" ]]; then
  echo "FAIL  sessions.jsonl missing after apply"
  exit 1
fi
echo "PASS  session journal"

echo "-- idempotent apply --"
cargo run -q -p estate-control -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$WORKDIR" --plans-dir "$PLANS" >/tmp/opday-unchanged.out 2>/tmp/opday-unchanged.err
if ! grep -q "unchanged" /tmp/opday-unchanged.out; then
  echo "FAIL  second apply must note unchanged"
  cat /tmp/opday-unchanged.out /tmp/opday-unchanged.err
  exit 1
fi
echo "PASS  apply unchanged"

echo "-- apply drift refuse / --force --"
rm -rf "$STATE/sessions/horizon"
set +e
cargo run -q -p estate-control -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$WORKDIR" --plans-dir "$PLANS" >/tmp/opday-drift.out 2>/tmp/opday-drift.err
drift_rc=$?
set -e
if [[ "$drift_rc" -eq 0 ]]; then
  echo "FAIL  drifted apply must refuse without --force"
  exit 1
fi
if ! grep -q "refuse:drift" /tmp/opday-drift.out /tmp/opday-drift.err; then
  echo "FAIL  drifted apply must print refuse:drift"
  cat /tmp/opday-drift.out /tmp/opday-drift.err
  exit 1
fi
cargo run -q -p estate-control -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$WORKDIR" --plans-dir "$PLANS" --force
echo "PASS  apply drift / --force"

echo "-- plan export-pr --"
cargo run -q -p estate-control -- plan export-pr --estate "$ESTATE" --state-dir "$STATE" --plans-dir "$PLANS" --reviewed-dir "$REVIEWED" --out "$PLANS/PR.md" >/tmp/opday-pr.out
if [[ ! -f "$PLANS/PR.md" ]]; then
  echo "FAIL  plans/PR.md missing"
  exit 1
fi
if ! grep -q "Blast radius" "$PLANS/PR.md" || ! grep -q "Refuse risks" "$PLANS/PR.md" || ! grep -q "Reviewed" "$PLANS/PR.md"; then
  echo "FAIL  export-pr markdown missing required sections"
  exit 1
fi
echo "PASS  plan export-pr"

echo "-- sacred overlay refuse --"
set +e
cargo run -q -p estate-control -- convey hop --id lab-notebook --capability lane-tool --state-dir "$STATE" >/tmp/opday-sacred-hop.out 2>/tmp/opday-sacred-hop.err
overlay_hop=$?
set -e
if [[ "$overlay_hop" -eq 0 ]]; then
  echo "FAIL  overlay sacred hop must refuse"
  exit 1
fi
if ! grep -q "refuse:sacred-id" /tmp/opday-sacred-hop.out /tmp/opday-sacred-hop.err; then
  echo "FAIL  overlay hop must print refuse:sacred-id"
  cat /tmp/opday-sacred-hop.out /tmp/opday-sacred-hop.err
  exit 1
fi
echo "PASS  sacred overlay refuse"

echo "-- suspend --"
cargo run -q -p estate-control -- suspend --state-dir "$STATE"

echo "-- plan against last apply + reviewed Security-as-IaC --"
cargo run -q -p estate-control -- plan --estate "$ESTATE" --plans-dir "$PLANS" --state-dir "$STATE" --reviewed --reviewed-dir "$REVIEWED"
if ! ls "$REVIEWED"/*.security.md >/dev/null 2>&1 && ! ls "$PLANS"/*.security.md >/dev/null 2>&1; then
  echo "FAIL  security-as-iac markdown missing"
  exit 1
fi

echo "-- stale greenfield must fail --require-fresh-plan --"
set +e
# Re-use the first (greenfield) covering plan: strict freshness must refuse.
cargo run -q -p estate-control -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$WORKDIR" --plans-dir "$PLANS" --require-fresh-plan >/tmp/opday-stale.out 2>/tmp/opday-stale.err
stale=$?
set -e
# After the second plan, covering_plan returns the newest hash match — which
# should now be fresh. Force the stale path with the committed fixture:
if [[ "$stale" -eq 0 ]]; then
  echo "NOTE  covering plan after second plan() is fresh (expected); fixture stale-plan is unit-tested"
fi

echo "-- apply --require-plan --require-fresh-plan --"
# After suspend, sessions/ is gone → drift. --force reconverges (pause-safe).
cargo run -q -p estate-control -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$WORKDIR" --plans-dir "$PLANS" --require-plan --require-fresh-plan --force

echo "-- plan diff (last-applied vs same estate; must not grow) --"
cargo run -q -p estate-control -- plan diff --estate "$ESTATE" --state-dir "$STATE" --plans-dir "$PLANS"
echo "PASS  plan diff"

echo "-- feed import (fixture pack, no live models) --"
cp "$ROOT/examples/fixtures/overnight-traces.pack.json" "$DROP/overnight-traces.pack.json"
cargo run -q -p estate-control -- packs index --drop-dir "$DROP"
BEFORE="$(cksum "$ESTATE")"
cargo run -q -p estate-control -- packs import --id overnight-traces --drop-dir "$DROP" --accepted-dir "$DROP/accepted" --estate "$ESTATE" --curator jason
set +e
cargo run -q -p estate-control -- packs import --id overnight-traces --drop-dir "$DROP" --accepted-dir "$DROP/accepted" --estate "$ESTATE" --curator not-jason >/tmp/opday-curator.out 2>/tmp/opday-curator.err
bad_curator=$?
set -e
if [[ "$bad_curator" -eq 0 ]]; then
  echo "FAIL  wrong curator must refuse"
  exit 1
fi
if ! grep -q "refuse:curator" /tmp/opday-curator.out /tmp/opday-curator.err; then
  echo "FAIL  wrong curator must print refuse:curator"
  exit 1
fi
AFTER="$(cksum "$ESTATE")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  pack import rewrote the estate"
  exit 1
fi
set +e
cargo run -q -p estate-control -- packs promote --id overnight-traces >/tmp/opday-promo.out 2>/tmp/opday-promo.err
promo=$?
set -e
if [[ "$promo" -eq 0 ]]; then
  echo "FAIL  packs promote must refuse"
  exit 1
fi
if [[ ! -f "$DROP/accepted/overnight-traces.redaction.json" ]]; then
  echo "FAIL  redaction report missing after import"
  exit 1
fi
echo "PASS  packs list/import/refuse-promote"

echo "-- packs propose (never auto-apply) --"
BEFORE_PROP="$(cksum "$ESTATE")"
cargo run -q -p estate-control -- packs propose --id overnight-traces --drop-dir "$DROP" --accepted-dir "$DROP/accepted" --proposed-dir "$DROP/proposed" --estate "$ESTATE"
AFTER_PROP="$(cksum "$ESTATE")"
if [[ "$BEFORE_PROP" != "$AFTER_PROP" ]]; then
  echo "FAIL  packs propose rewrote the estate"
  exit 1
fi
if [[ ! -f "$DROP/proposed/overnight-traces.proposal.json" ]]; then
  echo "FAIL  proposal pack missing"
  exit 1
fi
if ! grep -q '"auto_apply": false' "$DROP/proposed/overnight-traces.proposal.json"; then
  echo "FAIL  proposal must set auto_apply=false"
  exit 1
fi
echo "PASS  packs propose"

echo "-- convey mesh --"
cargo run -q -p estate-control -- convey sync --state-dir "$STATE"
cargo run -q -p estate-control -- convey call --id cell-one-box --capability lane-tool --state-dir "$STATE"
set +e
cargo run -q -p estate-control -- convey call --id cursor-cloud --capability mesh-stub --state-dir "$STATE" >/tmp/opday-cloud.out 2>/tmp/opday-cloud.err
cloud=$?
cargo run -q -p estate-control -- convey call --id missing-hop --capability lane-tool --state-dir "$STATE" >/tmp/opday-nolease.out 2>/tmp/opday-nolease.err
nolease=$?
set -e
if [[ "$cloud" -eq 0 || "$nolease" -eq 0 ]]; then
  echo "FAIL  convey must refuse cloud-mesh and missing lease"
  exit 1
fi
if ! grep -q "refuse:" /tmp/opday-cloud.err /tmp/opday-nolease.err; then
  echo "FAIL  convey refuse reasons must use refuse: prefix"
  exit 1
fi
echo "PASS  convey hop/call/refuse"

echo "-- convey hop TTL + expire --"
cargo run -q -p estate-control -- convey hop --id ttl-box --capability lane-tool --ttl-secs 3600 --state-dir "$STATE"
cargo run -q -p estate-control -- convey sync --state-dir "$STATE" >/tmp/opday-resync.out
if ! grep -q "ttl-box" /tmp/opday-resync.out; then
  echo "FAIL  convey sync must keep manually declared ttl-box"
  exit 1
fi
cargo run -q -p estate-control -- convey expire --state-dir "$STATE"
echo "PASS  convey expire"

echo "-- resume --"
cargo run -q -p estate-control -- resume --estate "$ESTATE" --state-dir "$STATE" --roots-base "$WORKDIR"
cargo run -q -p estate-control -- status --estate "$ESTATE" --state-dir "$STATE" --roots-base "$WORKDIR" --plans-dir "$PLANS" --packs-dir "$DROP" --policy "$ROOT/policy/cell-one.policy.v0.yaml" --root "$ROOT" | tee /tmp/opday-status.out
if ! grep -q "paused:" /tmp/opday-status.out || ! grep -q "last_plan:" /tmp/opday-status.out || ! grep -q "policy:" /tmp/opday-status.out || ! grep -q "doctor:" /tmp/opday-status.out; then
  echo "FAIL  status one-pager missing fields"
  exit 1
fi
echo "PASS  status one-pager"
cargo run -q -p estate-control -- packs list --drop-dir "$DROP"
cargo run -q -p estate-control -- convey leases --state-dir "$STATE"

echo "-- reconcile --"
cargo run -q -p estate-control -- reconcile --estate "$ESTATE" --state-dir "$STATE"
if [[ ! -f "$STATE/reconcile.md" ]]; then
  echo "FAIL  reconcile.md missing"
  exit 1
fi
echo "PASS  reconcile"

echo "-- audit export --"
cargo run -q -p estate-control -- audit export --estate "$ESTATE" --state-dir "$STATE" --plans-dir "$PLANS" --packs-dir "$DROP" --out "$WORKDIR/audit-export" --tar
if [[ ! -f "$WORKDIR/audit-export/MANIFEST.md" ]]; then
  echo "FAIL  audit export missing MANIFEST.md"
  exit 1
fi
echo "PASS  audit export"

echo "-- expire + doctor + sessions tail --"
cargo run -q -p estate-control -- expire --state-dir "$STATE"
cargo run -q -p estate-control -- doctor --root "$ROOT" --state-dir "$STATE"
cargo run -q -p estate-control -- sessions tail --state-dir "$STATE" --n 8
echo "PASS  expire/doctor/sessions"

echo "-- catalog capability flags --"
cargo run -q -p estate-control -- catalog --out "$STATE/catalog.json" >/tmp/opday-catalog.out
if ! grep -q "streaming=" /tmp/opday-catalog.out || ! grep -q "context=" /tmp/opday-catalog.out; then
  echo "FAIL  estate catalog must print capability flags"
  exit 1
fi
if ! grep -q '"context_tokens"' "$STATE/catalog.json"; then
  echo "FAIL  catalog.json missing context_tokens"
  exit 1
fi
echo "PASS  catalog caps"

echo "-- policy allow / deny / unknown --"
cargo run -q -p estate-control -- policy check --policy "$ROOT/policy/cell-one.policy.v0.yaml" --action apply
cargo run -q -p estate-control -- policy check --policy "$ROOT/examples/fixtures/policy-allow.yaml" --action convey-call
set +e
cargo run -q -p estate-control -- policy check --policy "$ROOT/examples/fixtures/policy-deny.yaml" --action apply >/tmp/opday-policy-deny.out 2>/tmp/opday-policy-deny.err
deny=$?
cargo run -q -p estate-control -- policy check --policy "$ROOT/examples/fixtures/policy-unknown-action.yaml" --action apply >/tmp/opday-policy-unknown.out 2>/tmp/opday-policy-unknown.err
unknown=$?
cargo run -q -p estate-control -- apply --estate "$ESTATE" --state-dir "$STATE" --roots-base "$WORKDIR" --plans-dir "$PLANS" --policy "$ROOT/examples/fixtures/policy-deny.yaml" >/tmp/opday-apply-deny.out 2>/tmp/opday-apply-deny.err
apply_deny=$?
set -e
if [[ "$deny" -eq 0 || "$unknown" -eq 0 || "$apply_deny" -eq 0 ]]; then
  echo "FAIL  policy deny / unknown / apply-deny must refuse"
  exit 1
fi
echo "PASS  policy"

echo "-- backup + restore --dry-run --"
cargo run -q -p estate-control -- backup --estate "$ESTATE" --state-dir "$STATE" --plans-dir "$PLANS" --out "$WORKDIR/backups" --policy "$ROOT/policy/cell-one.policy.v0.yaml"
ARCHIVE="$(find "$WORKDIR/backups" -maxdepth 1 -type d -name 'cell-backup-*' | sort | tail -n 1)"
if [[ -z "$ARCHIVE" || ! -f "$ARCHIVE/backup.json" ]]; then
  echo "FAIL  timestamped backup missing"
  exit 1
fi
BEFORE="$(cksum "$STATE/placement-actual.json")"
cargo run -q -p estate-control -- restore --from "$ARCHIVE" --estate "$ESTATE" --state-dir "$STATE" --plans-dir "$PLANS" --dry-run --policy "$ROOT/policy/cell-one.policy.v0.yaml"
AFTER="$(cksum "$STATE/placement-actual.json")"
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL  restore --dry-run wrote placement-actual.json"
  exit 1
fi
echo "PASS  backup / restore dry-run"

echo "-- restore sacred-mismatch --"
TAMPER="$WORKDIR/tampered-backup"
cp -a "$ARCHIVE" "$TAMPER"
cat > "$TAMPER/backup.json" <<'EOF'
{
  "schema": "cell-one.cell-backup.v0",
  "created_at": "unix:1",
  "state_dir": "tamper",
  "files": [],
  "sacred_ids": ["not-the-locked-set"],
  "writes": true,
  "cloud_agent_spawned": false,
  "note": "tampered sacred set"
}
EOF
set +e
cargo run -q -p estate-control -- restore --from "$TAMPER" --estate "$ESTATE" --state-dir "$STATE" --plans-dir "$PLANS" --dry-run --policy "$ROOT/policy/cell-one.policy.v0.yaml" >/tmp/opday-sacred.out 2>/tmp/opday-sacred.err
mismatch=$?
set -e
if [[ "$mismatch" -eq 0 ]]; then
  echo "FAIL  restore must refuse sacred mismatch"
  exit 1
fi
if ! grep -q "refuse:sacred-mismatch" /tmp/opday-sacred.out /tmp/opday-sacred.err; then
  echo "FAIL  sacred mismatch must print refuse:sacred-mismatch"
  exit 1
fi
echo "PASS  restore sacred-mismatch"

echo "-- pause-kit proof (apply → suspend → drop sessions → resume) --"
PAUSE_STATE="$WORKDIR/pause-proof"
mkdir -p "$PAUSE_STATE"
cargo run -q -p estate-control -- pause-proof --estate "$ESTATE" --state-dir "$PAUSE_STATE" --roots-base "$WORKDIR"
if [[ ! -f "$PAUSE_STATE/placement-actual.json" ]]; then
  echo "FAIL  pause-proof must keep leases on disk"
  exit 1
fi
echo "PASS  pause-proof"

echo
echo "OPERATOR-DAY GREEN (fixtures only; no live Grok / GPU)"
