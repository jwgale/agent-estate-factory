//! Gated apply of one standing improvement-package proposal.
//!
//! `estate decisions apply-package` applies one specialty-seat proposal
//! from a `cell-one.improvement-package.v0` through the existing enrich
//! apply-proposal → plan → apply `--require-plan` path. `--require-plan`
//! is required. When the package has more than one specialty-seat,
//! `--proposal specialty-seat:<id>` is required (`refuse:proposal:ambiguous`
//! without it) and the lab stays unchanged. A single specialty-seat
//! still defaults. Before any mutation it reads
//! `{prepared}/binding-proposal.json` and requires `binding_id` (and
//! the picked proposal id) to match the picked specialty-seat.
//! Mismatch is `refuse:proposal:` (or `refuse:prepared:`) and leaves
//! the lab estate unchanged. Train is not invoked. `auto_train` stays
//! false.
//!
//! `estate decisions improvement-apply-prove` reuses host-validate
//! receipts → export-package, then applies `--proposal specialty-seat:ag_news`
//! on a throwaway apply lab. Standing next is joinable after the gated
//! apply. Locked
//! `examples/estate.yaml` is refused. Apply without a plan is refused.
//! A prepared `binding_id` that does not match the picked seat is
//! refused before mutation. `READY_FOR_LIVE_TEST` stays no. Sibling of
//! improvement-export-prove.
//! `estate pack cohesion-prove` composes the same gated apply after
//! its export stage, reusing `{out}/improvement` without a second
//! host-validate. `estate decisions report`, `estate pack session
//! show`, and `estate status` on a state-dir that holds that local
//! receipt cite it so the operator sees the closed loop without
//! digging files. Cohesion-prove asserts those three cites.
//! Digest / `tick --report` / runner status cite the same receipt
//! when it is local and are not invoked by cohesion-prove.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::control_plane_prove;
use crate::improvement_export::{
    self, file_cksum, looks_like_locked_estate, proposal_id, refuse_report_inventions,
    EnrichProposal, ImprovementPackage,
};
use crate::specialty_bind;

const LOCKED_CKSUM: &str = "43770130 3391";
const APPLY_SCHEMA: &str = "cell-one.improvement-apply.v0";
const PROVE_SCHEMA: &str = "cell-one.improvement-apply-prove.v0";
const KIND_SEAT: &str = "specialty-seat";
const KIND_DATASET: &str = "dataset";
pub(crate) const REFUSE_WITHOUT_PLAN: &str = "refuse:plan: apply-package requires --require-plan";

pub(crate) fn cmd_decisions_apply_package(
    package_path: &Path,
    estate: &Path,
    prepared: &Path,
    state_dir: &Path,
    plans_dir: &Path,
    roots_base: &Path,
    require_plan: bool,
    proposal: Option<&str>,
    tag: Option<&str>,
    root: &Path,
    policy: &Path,
    curator: &str,
    receipt_out: Option<&Path>,
) -> Result<Value> {
    if !require_plan {
        bail!(REFUSE_WITHOUT_PLAN);
    }
    let root = root
        .canonicalize()
        .with_context(|| format!("refuse:root: {}", root.display()))?;
    if refuses_locked_write(estate, &root)? {
        bail!("refuse:out: apply-package does not write examples/estate.yaml");
    }
    if let Some(out) = receipt_out {
        if refuses_locked_write(out, &root)? {
            bail!("refuse:out: apply-package does not write examples/estate.yaml");
        }
    }
    let locked = root.join("examples/estate.yaml");
    let locked_before = if locked.is_file() {
        let cksum = file_cksum(&locked)?;
        if !cksum.starts_with(LOCKED_CKSUM) {
            bail!("refuse:estate: examples/estate.yaml cksum is {cksum}, want {LOCKED_CKSUM}");
        }
        Some((fs::read(&locked)?, cksum))
    } else {
        None
    };

    let package = improvement_export::load_package(package_path)?;
    let picked = improvement_export::pick_specialty_seat(&package, proposal)?.clone();
    refuse_train_on_proposal(&picked)?;
    let applied = apply_picked_proposal(
        &package,
        &picked,
        estate,
        prepared,
        state_dir,
        plans_dir,
        roots_base,
        tag,
        policy,
        curator,
        &root,
    )?;
    if let Some((before, cksum_before)) = locked_before {
        if fs::read(&locked)? != before {
            bail!("refuse:estate: apply-package rewrote examples/estate.yaml");
        }
        let cksum_after = file_cksum(&locked)?;
        if cksum_after != cksum_before {
            bail!("refuse:estate: examples/estate.yaml cksum changed to {cksum_after}");
        }
    }
    if let Some(out) = receipt_out {
        write_apply_receipt(out, &applied)?;
    }
    print_apply_cite(&applied);
    Ok(applied)
}

pub(crate) fn cmd_decisions_improvement_apply_prove(root: &Path, out: Option<&Path>) -> Result<()> {
    std::env::remove_var("CELL_LOCAL_ENDPOINT");
    std::env::remove_var("CELL_LOCAL_LIVE");
    let root = root
        .canonicalize()
        .with_context(|| format!("refuse:root: {}", root.display()))?;
    let locked = root.join("examples/estate.yaml");
    let before =
        fs::read(&locked).with_context(|| format!("refuse:estate: {}", locked.display()))?;
    let cksum_before = file_cksum(&locked)?;
    if !cksum_before.starts_with(LOCKED_CKSUM) {
        bail!("refuse:estate: examples/estate.yaml cksum is {cksum_before}, want {LOCKED_CKSUM}");
    }

    let (out_hint, wipe) = match out {
        Some(path) => (path.to_path_buf(), false),
        None => (default_out(), true),
    };
    if control_plane_prove::refuses_locked_target(&out_hint, &root, &locked)? {
        bail!("refuse:out: improvement-apply-prove does not write examples/estate.yaml");
    }
    if wipe {
        let _ = fs::remove_dir_all(&out_hint);
    }
    fs::create_dir_all(&out_hint).with_context(|| format!("refuse:out: {}", out_hint.display()))?;
    let out = out_hint
        .canonicalize()
        .with_context(|| format!("refuse:out: {}", out_hint.display()))?;
    if control_plane_prove::refuses_locked_target(&out, &root, &locked)? {
        bail!("refuse:out: improvement-apply-prove does not write examples/estate.yaml");
    }

    println!("improvement-apply-prove: host-validate");
    crate::decision_prove::cmd_decisions_host_validate_prove(&root, Some(&out))?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: host-validate rewrote examples/estate.yaml");
    }

    let state = out.join("state");
    let package_dir = out.join("improvement");
    println!("improvement-apply-prove: export-package");
    let package = improvement_export::cmd_decisions_export_package(&state, &package_dir, &root)?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: export-package rewrote examples/estate.yaml");
    }
    if package.auto_train || package.train_invoked {
        bail!("refuse:improvement: package invented auto-train");
    }
    let package_path = package_dir.join("improvement-package.json");
    let outcome = run_gated_apply(
        &package,
        &package_path,
        &out.join("apply"),
        &root,
        "improvement-apply-prove",
    )?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: gated apply rewrote examples/estate.yaml");
    }

    let cksum_after = file_cksum(&locked)?;
    if cksum_after != cksum_before {
        bail!("refuse:estate: examples/estate.yaml cksum changed to {cksum_after}");
    }

    let body = json!({
        "schema": PROVE_SCHEMA,
        "apply_schema": APPLY_SCHEMA,
        "ok": true,
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "live_sync": false,
        "auto_train": false,
        "train_invoked": false,
        "estate_cksum": cksum_after,
        "host_validate_report": out.join("host-validate-prove.json").display().to_string(),
        "package_path": package_path.display().to_string(),
        "applied_proposal_id": outcome.proposal_id,
        "applied_proposal_kind": outcome.proposal_kind,
        "binding_id": outcome.binding_id,
        "joinable": true,
        "standing": "joinable: yes",
        "require_plan": true,
        "refuse_without_plan": REFUSE_WITHOUT_PLAN,
        "apply_receipt": outcome.receipt_path.display().to_string(),
        "lab_estate": outcome.lab.display().to_string(),
        "note": "Fixture prove. Reuses host-validate authorize / convey / complete --mock receipts, export-package, then gated apply of one specialty-seat proposal through apply-proposal → plan → apply --require-plan. Standing next is joinable. auto_train=false. Train not invoked. estate decisions report on that lab cites the apply receipt. estate status / digest / tick --report / runner status cite the same receipt when it is local to the state-dir (not invoked here). Not a live PASS."
    });
    write_prove_report(&out, &body)?;
    println!("improvement-apply-prove: decisions report");
    crate::decisions::cmd_decisions_report(&state, None)?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    println!(
        "applied: {} kind={} binding={} joinable=yes auto_train=false train_invoked=no",
        outcome.proposal_id, outcome.proposal_kind, outcome.binding_id
    );
    println!("improvement-apply-prove: ok");
    println!("READY_FOR_LIVE_TEST: no");
    Ok(())
}

struct GatedApplyOutcome {
    proposal_id: String,
    proposal_kind: String,
    binding_id: String,
    receipt_path: PathBuf,
    lab: PathBuf,
}

/// One gated specialty-seat apply on a throwaway lab.
///
/// Reuses `package` already loaded from disk. Does not host-validate
/// and does not export-package. Passes `--proposal specialty-seat:ag_news`
/// when that proposal is present so the apply stays deterministic.
/// Apply without `--require-plan` is `refuse:plan` and leaves the lab
/// unchanged.
fn run_gated_apply(
    package: &ImprovementPackage,
    package_path: &Path,
    apply_out: &Path,
    root: &Path,
    log_prefix: &str,
) -> Result<GatedApplyOutcome> {
    let root = root
        .canonicalize()
        .with_context(|| format!("refuse:root: {}", root.display()))?;
    let package_before = fs::read(package_path)
        .with_context(|| format!("refuse:package: read {}", package_path.display()))?;
    let want = prove_proposal_arg(package);
    let picked = improvement_export::pick_specialty_seat(package, want)?.clone();
    refuse_train_on_proposal(&picked)?;
    if picked.kind != KIND_SEAT {
        bail!(
            "refuse:proposal: apply-package applies one specialty-seat proposal; got {}",
            picked.kind
        );
    }
    println!(
        "{log_prefix}: pick {} kind={} binding={}",
        proposal_id(&picked),
        picked.kind,
        picked.binding_id
    );

    let locked = root.join("examples/estate.yaml");
    let locked_before = if locked.is_file() {
        Some(fs::read(&locked)?)
    } else {
        None
    };

    let _ = fs::remove_dir_all(apply_out);
    fs::create_dir_all(apply_out)
        .with_context(|| format!("refuse:out: {}", apply_out.display()))?;
    println!("{log_prefix}: stage");
    let staged =
        specialty_bind::stage_one_specialty_for_apply(&root, apply_out, &picked.binding_id)?;
    if staged.binding_id != picked.binding_id {
        bail!(
            "refuse:proposal: staged {} want {}",
            staged.binding_id, picked.binding_id
        );
    }
    if estate_has_binding(&staged.lab, &picked.binding_id)? {
        bail!(
            "refuse:binding: {} landed before gated apply",
            picked.binding_id
        );
    }
    if let Some(before) = &locked_before {
        if fs::read(&locked)? != *before {
            bail!("refuse:estate: stage rewrote examples/estate.yaml");
        }
    }

    let policy = root.join("policy/cell-one.policy.v0.yaml");
    let lab_before = fs::read(&staged.lab)?;
    println!("{log_prefix}: refuse-without-plan");
    match cmd_decisions_apply_package(
        package_path,
        &staged.lab,
        &staged.prepared,
        &staged.state,
        &staged.plans,
        &staged.roots,
        false,
        want,
        Some(&staged.tag),
        &root,
        &policy,
        "jason",
        None,
    ) {
        Ok(_) => bail!("refuse:plan: apply-package without --require-plan must refuse"),
        Err(err) => {
            let text = format!("{err:#}");
            if !text.contains(REFUSE_WITHOUT_PLAN) {
                bail!("refuse:plan: unexpected refuse without plan: {text}");
            }
            println!("{REFUSE_WITHOUT_PLAN}");
        }
    }
    if fs::read(&staged.lab)? != lab_before {
        bail!("refuse:estate: apply without plan rewrote the lab estate");
    }
    if estate_has_binding(&staged.lab, &picked.binding_id)? {
        bail!(
            "refuse:binding: {} landed without --require-plan",
            picked.binding_id
        );
    }

    println!("{log_prefix}: apply --require-plan");
    let receipt_path = apply_out.join("improvement-apply.json");
    let applied = cmd_decisions_apply_package(
        package_path,
        &staged.lab,
        &staged.prepared,
        &staged.state,
        &staged.plans,
        &staged.roots,
        true,
        want,
        Some(&staged.tag),
        &root,
        &policy,
        "jason",
        Some(&receipt_path),
    )?;
    if let Some(before) = &locked_before {
        if fs::read(&locked)? != *before {
            bail!("refuse:estate: gated apply rewrote examples/estate.yaml");
        }
    }
    if fs::read(package_path)? != package_before {
        bail!("refuse:package: gated apply rewrote the improvement package");
    }
    if !estate_has_binding(&staged.lab, &picked.binding_id)? {
        bail!(
            "refuse:binding: {} missing after gated apply",
            picked.binding_id
        );
    }
    if !estate_has_binding(&staged.lab, "local_slm")? {
        bail!("refuse:binding: local_slm missing after gated apply");
    }
    for row in package.proposals.iter().filter(|row| row.kind == KIND_SEAT) {
        if row.binding_id == picked.binding_id {
            continue;
        }
        if estate_has_binding(&staged.lab, &row.binding_id)? {
            bail!(
                "refuse:proposal: {} landed but only one specialty-seat is applied",
                row.binding_id
            );
        }
    }
    if package
        .proposals
        .iter()
        .any(|row| row.kind == KIND_DATASET && row.auto_train)
    {
        bail!("refuse:improvement: a dataset proposal invented auto-train");
    }
    if package.proposals.iter().any(|row| {
        row.kind == KIND_DATASET && proposal_id(row) == proposal_id(&picked)
    }) {
        bail!("refuse:proposal: dataset proposal was selected for apply");
    }

    println!("{log_prefix}: standing-next");
    crate::enrich::cmd_enrich_standing_next(&staged.lab, &staged.prepared)?;
    let joinable = applied
        .get("joinable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !joinable {
        bail!("refuse:standing: gated apply is not joinable");
    }
    if applied.get("auto_train") != Some(&Value::Bool(false))
        || applied.get("train_invoked") != Some(&Value::Bool(false))
        || applied.get("require_plan") != Some(&Value::Bool(true))
    {
        bail!("refuse:improvement: apply receipt invented auto-train or dropped require-plan");
    }
    install_apply_receipt_for_report(&staged.state, &receipt_path)?;
    if let Some(parent) = apply_out.parent() {
        install_apply_receipt_for_report(&parent.join("state"), &receipt_path)?;
    }

    Ok(GatedApplyOutcome {
        proposal_id: proposal_id(&picked),
        proposal_kind: picked.kind,
        binding_id: picked.binding_id,
        receipt_path,
        lab: staged.lab,
    })
}

/// Prove / cohesion pass `--proposal specialty-seat:ag_news` when that
/// seat is in the package so apply-package does not silently pick.
fn prove_proposal_arg(package: &ImprovementPackage) -> Option<&'static str> {
    if package.proposals.iter().any(|row| {
        row.kind == KIND_SEAT
            && (row.binding_id == "ag_news" || proposal_id(row) == "specialty-seat:ag_news")
    }) {
        Some("specialty-seat:ag_news")
    } else {
        None
    }
}

/// Apply stage for `estate pack cohesion-prove`.
///
/// Loads the standing package already written under `{out}/improvement`.
/// Does not re-run host-validate and does not export-package again.
pub(crate) fn run_apply_stage(out: &Path, root: &Path) -> Result<Value> {
    let package_path = out.join("improvement").join("improvement-package.json");
    if !package_path.is_file() {
        bail!(
            "refuse:package: cohesion apply reuses {} and does not re-run host-validate",
            package_path.display()
        );
    }
    let package = improvement_export::load_package(&package_path)?;
    let journal = Path::new(&package.journal.path);
    if !journal.is_file() {
        bail!(
            "refuse:package: cohesion apply journal {} is missing; does not re-run host-validate",
            journal.display()
        );
    }
    if !package.proposals.iter().any(|row| row.kind == KIND_DATASET) {
        bail!(
            "refuse:proposal: cohesion apply package has no dataset proposals to leave proposal-only"
        );
    }
    let locked = root.join("examples").join("estate.yaml");
    let apply_out = out.join("apply");
    if control_plane_prove::refuses_locked_target(&apply_out, root, &locked)?
        || looks_like_locked_estate(&apply_out)
    {
        bail!("refuse:out: cohesion apply does not write examples/estate.yaml");
    }
    let outcome = run_gated_apply(&package, &package_path, &apply_out, root, "cohesion-prove")?;
    if package.proposals.iter().any(|row| {
        row.kind == KIND_SEAT && row.binding_id == "ag_news"
    }) && outcome.binding_id != "ag_news"
    {
        bail!("refuse:proposal: cohesion apply did not pass --proposal specialty-seat:ag_news");
    }
    if outcome.proposal_kind != KIND_SEAT {
        bail!("refuse:proposal: cohesion apply kind is not specialty-seat");
    }
    Ok(apply_stage_cite(&outcome, &package, &package_path))
}

fn apply_stage_cite(
    outcome: &GatedApplyOutcome,
    package: &ImprovementPackage,
    package_path: &Path,
) -> Value {
    let dataset_ids: Vec<String> = package
        .proposals
        .iter()
        .filter(|row| row.kind == KIND_DATASET)
        .map(proposal_id)
        .collect();
    json!({
        "schema": APPLY_SCHEMA,
        "apply_schema": APPLY_SCHEMA,
        "ok": true,
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "live_sync": false,
        "auto_train": false,
        "train_invoked": false,
        "require_plan": true,
        "refuse_without_plan": REFUSE_WITHOUT_PLAN,
        "apply_receipt": outcome.receipt_path.display().to_string(),
        "applied_proposal_id": outcome.proposal_id,
        "applied_proposal_kind": outcome.proposal_kind,
        "binding_id": outcome.binding_id,
        "joinable": true,
        "standing": "joinable: yes",
        "local_slm": true,
        "dataset_proposals": "proposal-only",
        "dataset_proposal_ids": dataset_ids,
        "package_path": package_path.display().to_string(),
        "lab_estate": outcome.lab.display().to_string(),
        "composed_by": "cohesion-prove",
        "reused_existing_package": true,
        "host_validate_rerun": false,
        "note": "Gated apply of one specialty-seat proposal from the package already written by the cohesion export stage. Does not re-run host-validate. apply-proposal → plan → apply --require-plan. Standing next is joinable. local_slm stays. Dataset proposals stay proposal-only. auto_train=false. Train not invoked. estate decisions report, estate pack session show, and estate status cite this receipt on the lab / crew / apply state-dir. Digest / tick --report / runner status cite the same receipt when it is local (not invoked by cohesion-prove). Not a live PASS."
    })
}

fn apply_picked_proposal(
    package: &ImprovementPackage,
    picked: &EnrichProposal,
    estate: &Path,
    prepared: &Path,
    state_dir: &Path,
    plans_dir: &Path,
    roots_base: &Path,
    tag: Option<&str>,
    policy: &Path,
    curator: &str,
    root: &Path,
) -> Result<Value> {
    if picked.kind != KIND_SEAT {
        bail!(
            "refuse:proposal: apply-package applies one specialty-seat proposal; got {}",
            picked.kind
        );
    }
    if picked.auto_train || package.auto_train || package.train_invoked {
        bail!("refuse:improvement: apply-package invented auto-train");
    }
    require_prepared_matches_picked(prepared, picked)?;
    let tag = resolve_tag(prepared, tag)?;
    let source_before = fs::read(estate)
        .with_context(|| format!("refuse:estate: read {}", estate.display()))?;
    crate::enrich::cmd_enrich_apply_proposal(
        estate, prepared, &tag, state_dir, plans_dir, curator, false,
    )?;
    if fs::read(estate)? != source_before {
        bail!("refuse:estate: apply-proposal rewrote the source estate");
    }
    let staged = state_dir.join("enrich-stage/staged-estate.yaml");
    if !staged.is_file() {
        bail!("refuse:stage: apply-proposal did not write staged-estate.yaml");
    }
    crate::plan_apply::cmd_plan(
        &staged,
        None,
        plans_dir,
        state_dir,
        false,
        &plans_dir.join("reviewed"),
    )?;
    crate::plan_apply::cmd_apply(
        &staged,
        state_dir,
        roots_base,
        plans_dir,
        true,
        None,
        &root.join("packs"),
        false,
        false,
        policy,
        curator,
        false,
    )?;
    if !estate_has_binding(estate, &picked.binding_id)? {
        bail!(
            "refuse:binding: {} missing after apply --require-plan",
            picked.binding_id
        );
    }
    if !estate_has_binding(estate, "local_slm")? {
        bail!("refuse:binding: local_slm missing after apply --require-plan");
    }
    let agent = specialty_bind::named_seat_request(&picked.binding_id, PathBuf::new())
        .map(|row| row.agent)
        .unwrap_or("research");
    specialty_bind::graft_one_scope(estate, &picked.binding_id, agent, &picked.dataset)?;
    crate::enrich::cmd_enrich_standing_next(estate, prepared)?;
    let join = model_estate::assess_specialty_join(
        &estate_schema::load_estate(estate)
            .with_context(|| format!("refuse:estate: load {}", estate.display()))?,
        prepared,
    )
    .map_err(|err| anyhow::anyhow!("{err}"))?
    .ok_or_else(|| anyhow::anyhow!("refuse:standing: specialty join missing after apply"))?;
    if !join.joinable || join.binding_id != picked.binding_id {
        bail!(
            "refuse:standing: {} joinable={} binding={}",
            picked.binding_id,
            join.joinable,
            join.binding_id
        );
    }
    if join.auto_apply || join.ready_for_live_test || join.live_pass_recorded {
        bail!("refuse:standing: join invented auto-apply or a live-test ready flag");
    }
    Ok(json!({
        "schema": APPLY_SCHEMA,
        "ok": true,
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "live_sync": false,
        "auto_train": false,
        "train_invoked": false,
        "require_plan": true,
        "refuse_without_plan": REFUSE_WITHOUT_PLAN,
        "proposal_id": proposal_id(picked),
        "proposal_kind": picked.kind,
        "binding_id": picked.binding_id,
        "dataset": picked.dataset,
        "action": picked.action,
        "joinable": true,
        "standing": "joinable: yes",
        "agents": join.agents,
        "seat_model": join.seat_model,
        "trained_shape": "gguf",
        "estate": estate.display().to_string(),
        "package_id": package.id,
        "note": "Gated apply of one specialty-seat proposal. apply-proposal → plan → apply --require-plan. Standing next is joinable. auto_train=false. Train not invoked. Not a live PASS."
    }))
}

/// Refuse before apply-proposal / plan / apply when `--prepared`
/// names a different specialty seat than the picked proposal.
fn require_prepared_matches_picked(prepared: &Path, picked: &EnrichProposal) -> Result<()> {
    let path = prepared.join("binding-proposal.json");
    let text = fs::read_to_string(&path)
        .map_err(|err| anyhow::anyhow!("refuse:prepared: {}: {err}", path.display()))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|err| anyhow::anyhow!("refuse:prepared: parse {}: {err}", path.display()))?;
    let prepared_binding = value
        .get("binding_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("refuse:prepared: binding-proposal.json has no binding_id")
        })?;
    if prepared_binding != picked.binding_id {
        bail!(
            "refuse:proposal: prepared binding_id {prepared_binding} does not match picked {}",
            picked.binding_id
        );
    }
    if let Some(proposed_id) = value
        .get("proposed_binding")
        .and_then(|row| row.get("id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if proposed_id != picked.binding_id {
            bail!(
                "refuse:proposal: prepared proposed_binding.id {proposed_id} does not match picked {}",
                picked.binding_id
            );
        }
    }
    let picked_id = proposal_id(picked);
    let prepared_id = format!("{KIND_SEAT}:{prepared_binding}");
    if picked_id != prepared_id {
        bail!(
            "refuse:proposal: prepared proposal {prepared_id} does not match picked {picked_id}"
        );
    }
    Ok(())
}

fn resolve_tag(prepared: &Path, tag: Option<&str>) -> Result<String> {
    if let Some(tag) = tag.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(tag.to_string());
    }
    let text = fs::read_to_string(prepared.join("binding-proposal.json"))
        .with_context(|| format!("refuse:proposal: {}", prepared.display()))?;
    let value: Value = serde_json::from_str(&text)
        .with_context(|| format!("refuse:proposal: parse {}", prepared.display()))?;
    value
        .get("local_tag")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("refuse:proposal: binding-proposal.json has no local_tag"))
}

fn refuse_train_on_proposal(row: &EnrichProposal) -> Result<()> {
    if row.auto_train {
        bail!("refuse:improvement: a proposal invented auto-train");
    }
    if row.action != "enrich-prepare" {
        bail!("refuse:improvement: a proposal is not enrich-prepare");
    }
    Ok(())
}

fn estate_has_binding(path: &Path, id: &str) -> Result<bool> {
    let estate = estate_schema::load_estate(path)
        .with_context(|| format!("refuse:estate: load {}", path.display()))?;
    Ok(estate.model_bindings.iter().any(|row| row.id == id))
}

fn refuses_locked_write(path: &Path, root: &Path) -> Result<bool> {
    if looks_like_locked_estate(path) {
        return Ok(true);
    }
    let locked = root.join("examples").join("estate.yaml");
    control_plane_prove::refuses_locked_target(path, root, &locked)
}

/// Copy the apply receipt beside a lab journal so `estate decisions
/// report`, `estate pack session show`, `estate status`,
/// `estate routine digest`, `estate routine tick --report`, and
/// `estate routine runner status` cite it without the operator
/// opening `{out}/apply`. Local copy only — not a sibling walk.
pub(crate) fn install_apply_receipt_for_report(state_dir: &Path, receipt_path: &Path) -> Result<()> {
    if !state_dir.is_dir() {
        return Ok(());
    }
    let dest = state_dir.join("decisions").join("improvement-apply.json");
    if dest.canonicalize().ok().as_deref() == receipt_path.canonicalize().ok().as_deref() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("refuse:out: {}", parent.display()))?;
    }
    fs::copy(receipt_path, &dest)
        .with_context(|| format!("refuse:out: copy apply receipt to {}", dest.display()))?;
    Ok(())
}

fn write_apply_receipt(out: &Path, body: &Value) -> Result<()> {
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let pretty = serde_json::to_string_pretty(body)?;
    refuse_report_inventions(&pretty, "improvement-apply")?;
    fs::write(out, format!("{pretty}\n"))
        .with_context(|| format!("refuse:out: write {}", out.display()))?;
    Ok(())
}

fn write_prove_report(out: &Path, body: &Value) -> Result<()> {
    let pretty = serde_json::to_string_pretty(body)?;
    refuse_report_inventions(&pretty, "improvement-apply")?;
    fs::write(out.join("improvement-apply-prove.json"), format!("{pretty}\n"))?;
    Ok(())
}

fn print_apply_cite(applied: &Value) {
    println!(
        "applied improvement proposal {} kind={} binding={} standing=joinable: yes require_plan=true refuse_without_plan={} auto_train=false train_invoked=no schema={}",
        applied
            .get("proposal_id")
            .and_then(Value::as_str)
            .unwrap_or("-"),
        applied
            .get("proposal_kind")
            .and_then(Value::as_str)
            .unwrap_or("-"),
        applied
            .get("binding_id")
            .and_then(Value::as_str)
            .unwrap_or("-"),
        REFUSE_WITHOUT_PLAN,
        APPLY_SCHEMA
    );
}

fn default_out() -> PathBuf {
    let mut token = std::process::id().to_string();
    for needle in ["5090", "4090", "4080", "3090"] {
        token = token.replace(needle, "0000");
    }
    std::env::temp_dir().join(format!("cell-one-improvement-apply-{token}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::improvement_export::{JournalCite, REFUSE_AMBIGUOUS_PROPOSAL};

    fn proposal(kind: &str, binding: &str) -> EnrichProposal {
        EnrichProposal {
            id: format!("{kind}:{binding}"),
            kind: kind.into(),
            binding_id: binding.into(),
            dataset: binding.into(),
            action: "enrich-prepare".into(),
            auto_train: false,
            note: "test".into(),
        }
    }

    #[test]
    fn apply_package_refuses_without_require_plan() {
        let err = cmd_decisions_apply_package(
            Path::new("/tmp/missing-package.json"),
            Path::new("/tmp/missing-estate.yaml"),
            Path::new("/tmp/missing-prepared"),
            Path::new("/tmp/missing-state"),
            Path::new("/tmp/missing-plans"),
            Path::new("/tmp/missing-roots"),
            false,
            None,
            None,
            Path::new("."),
            Path::new("policy/cell-one.policy.v0.yaml"),
            "jason",
            None,
        )
        .unwrap_err();
        assert!(
            format!("{err:#}").contains("refuse:plan: apply-package requires --require-plan"),
            "{err:#}"
        );
    }

    #[test]
    fn pick_specialty_seat_skips_dataset_and_refuses_train() {
        let package = ImprovementPackage {
            schema: "cell-one.improvement-package.v0".into(),
            id: "improvement-from-decisions".into(),
            kind: "standing-improvement".into(),
            auto_train: false,
            train_invoked: false,
            ready_for_live_test: false,
            live_pass_recorded: false,
            live_sync: false,
            journal: JournalCite {
                path: "journal".into(),
                receipts: 1,
                surface_authorize: 1,
                surface_convey: 1,
                surface_complete: 1,
                specialty_seats: vec!["ag_news".into()],
            },
            proposals: vec![
                proposal("dataset", "ag_news"),
                proposal(KIND_SEAT, "ag_news"),
            ],
            note: "none".into(),
        };
        let picked = improvement_export::pick_specialty_seat(&package, None).unwrap();
        assert_eq!(picked.kind, KIND_SEAT);
        assert_eq!(proposal_id(picked), "specialty-seat:ag_news");
        let err = improvement_export::pick_specialty_seat(&package, Some("dataset:ag_news"))
            .unwrap_err();
        assert!(format!("{err:#}").contains("specialty-seat"), "{err:#}");
    }

    #[test]
    fn prove_proposal_arg_is_explicit_ag_news() {
        let package = ImprovementPackage {
            schema: "cell-one.improvement-package.v0".into(),
            id: "improvement-from-decisions".into(),
            kind: "standing-improvement".into(),
            auto_train: false,
            train_invoked: false,
            ready_for_live_test: false,
            live_pass_recorded: false,
            live_sync: false,
            journal: JournalCite {
                path: "journal".into(),
                receipts: 1,
                surface_authorize: 1,
                surface_convey: 1,
                surface_complete: 1,
                specialty_seats: vec!["rust_idiom".into(), "ag_news".into()],
            },
            proposals: vec![
                proposal(KIND_DATASET, "ag_news"),
                proposal(KIND_SEAT, "rust_idiom"),
                proposal(KIND_SEAT, "ag_news"),
            ],
            note: "none".into(),
        };
        assert_eq!(prove_proposal_arg(&package), Some("specialty-seat:ag_news"));
        let picked =
            improvement_export::pick_specialty_seat(&package, prove_proposal_arg(&package))
                .unwrap();
        assert_eq!(proposal_id(picked), "specialty-seat:ag_news");
        assert_eq!(picked.kind, KIND_SEAT);
        let err = improvement_export::pick_specialty_seat(&package, None).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains(REFUSE_AMBIGUOUS_PROPOSAL), "{text}");
        assert!(text.contains("specialty-seat:ag_news"), "{text}");
        assert!(text.contains("specialty-seat:rust_idiom"), "{text}");
    }

    fn multi_seat_package_json() -> String {
        r#"{
  "schema": "cell-one.improvement-package.v0",
  "id": "improvement-from-decisions",
  "kind": "standing-improvement",
  "auto_train": false,
  "train_invoked": false,
  "ready_for_live_test": false,
  "live_pass_recorded": false,
  "live_sync": false,
  "journal": {
    "path": "journal",
    "receipts": 1,
    "surface_authorize": 1,
    "surface_convey": 1,
    "surface_complete": 1,
    "specialty_seats": ["ag_news", "rust_idiom"]
  },
  "proposals": [
    {
      "id": "specialty-seat:ag_news",
      "kind": "specialty-seat",
      "binding_id": "ag_news",
      "dataset": "ag_news",
      "action": "enrich-prepare",
      "auto_train": false,
      "note": "test"
    },
    {
      "id": "dataset:ag_news",
      "kind": "dataset",
      "binding_id": "ag_news",
      "dataset": "ag_news",
      "action": "enrich-prepare",
      "auto_train": false,
      "note": "test"
    },
    {
      "id": "specialty-seat:rust_idiom",
      "kind": "specialty-seat",
      "binding_id": "rust_idiom",
      "dataset": "rust_idiom",
      "action": "enrich-prepare",
      "auto_train": false,
      "note": "test"
    },
    {
      "id": "dataset:rust_idiom",
      "kind": "dataset",
      "binding_id": "rust_idiom",
      "dataset": "rust_idiom",
      "action": "enrich-prepare",
      "auto_train": false,
      "note": "test"
    }
  ],
  "note": "test"
}
"#
        .into()
    }

    #[test]
    fn apply_package_refuses_ambiguous_multi_seat_without_rewriting_lab() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let locked = root.join("examples/estate.yaml");
        let locked_before = fs::read(&locked).unwrap();
        let cksum_before = file_cksum(&locked).unwrap();
        assert!(cksum_before.starts_with(LOCKED_CKSUM), "{cksum_before}");
        let out = std::env::temp_dir().join(format!(
            "cell-one-apply-ambiguous-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(&out).unwrap();
        let lab = out.join("lab-estate.yaml");
        fs::copy(&locked, &lab).unwrap();
        let lab_before = fs::read(&lab).unwrap();
        let package_path = out.join("improvement-package.json");
        fs::write(&package_path, multi_seat_package_json()).unwrap();
        let prepared = out.join("prepared");
        write_binding_proposal(&prepared, "ag_news", Some("ag_news"));
        let err = cmd_decisions_apply_package(
            &package_path,
            &lab,
            &prepared,
            &out.join("state"),
            &out.join("plans"),
            &out.join("roots"),
            true,
            None,
            None,
            &root,
            &root.join("policy/cell-one.policy.v0.yaml"),
            "jason",
            None,
        )
        .unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains(REFUSE_AMBIGUOUS_PROPOSAL), "{text}");
        assert!(text.contains("specialty-seat:ag_news"), "{text}");
        assert!(text.contains("specialty-seat:rust_idiom"), "{text}");
        assert_eq!(fs::read(&lab).unwrap(), lab_before);
        assert!(!estate_has_binding(&lab, "ag_news").unwrap());
        assert!(!estate_has_binding(&lab, "rust_idiom").unwrap());
        assert!(!out.join("state/enrich-stage/staged-estate.yaml").is_file());
        assert_eq!(fs::read(&locked).unwrap(), locked_before);
        assert_eq!(file_cksum(&locked).unwrap(), cksum_before);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn apply_package_applies_explicit_proposal_when_multi_seat() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let locked = root.join("examples/estate.yaml");
        let locked_before = fs::read(&locked).unwrap();
        let cksum_before = file_cksum(&locked).unwrap();
        assert!(cksum_before.starts_with(LOCKED_CKSUM), "{cksum_before}");
        let out = std::env::temp_dir().join(format!(
            "cell-one-apply-explicit-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&out);
        let staged =
            specialty_bind::stage_one_specialty_for_apply(&root, &out, "ag_news").unwrap();
        let package_path = out.join("improvement-package.json");
        fs::write(&package_path, multi_seat_package_json()).unwrap();
        let applied = cmd_decisions_apply_package(
            &package_path,
            &staged.lab,
            &staged.prepared,
            &staged.state,
            &staged.plans,
            &staged.roots,
            true,
            Some("specialty-seat:ag_news"),
            Some(&staged.tag),
            &root,
            &root.join("policy/cell-one.policy.v0.yaml"),
            "jason",
            None,
        )
        .unwrap();
        assert_eq!(applied["proposal_id"], "specialty-seat:ag_news");
        assert_eq!(applied["proposal_kind"], "specialty-seat");
        assert_eq!(applied["binding_id"], "ag_news");
        assert_eq!(applied["auto_train"], false);
        assert_eq!(applied["train_invoked"], false);
        assert_eq!(applied["ready_for_live_test"], false);
        assert_eq!(applied["require_plan"], true);
        assert!(estate_has_binding(&staged.lab, "ag_news").unwrap());
        assert!(estate_has_binding(&staged.lab, "local_slm").unwrap());
        assert!(!estate_has_binding(&staged.lab, "rust_idiom").unwrap());
        assert_eq!(fs::read(&locked).unwrap(), locked_before);
        assert_eq!(file_cksum(&locked).unwrap(), cksum_before);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn cohesion_apply_stage_refuses_missing_package_without_host_validate() {
        let missing = std::env::temp_dir().join(format!(
            "cell-one-cohesion-apply-missing-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&missing);
        let err = run_apply_stage(&missing, Path::new(".")).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains("refuse:package"), "{text}");
        assert!(text.contains("does not re-run host-validate"), "{text}");
        assert!(!text.contains("decision-host-validate-prove"), "{text}");
    }

    fn write_binding_proposal(dir: &Path, binding_id: &str, proposed_id: Option<&str>) {
        fs::create_dir_all(dir).unwrap();
        let proposed = proposed_id
            .map(|id| format!(r#", "proposed_binding": {{ "id": "{id}" }}"#))
            .unwrap_or_default();
        fs::write(
            dir.join("binding-proposal.json"),
            format!(
                r#"{{"schema":"cell-one.enrich-binding-proposal.v0","binding_id":"{binding_id}","local_tag":"cell-enrich-overnight-traces"{proposed}}}"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn prepared_matches_picked_binding_and_proposal_id() {
        let dir = std::env::temp_dir().join(format!(
            "cell-one-prepared-match-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        write_binding_proposal(&dir, "ag_news", Some("ag_news"));
        require_prepared_matches_picked(&dir, &proposal(KIND_SEAT, "ag_news")).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn prepared_mismatch_binding_id_refuses() {
        let dir = std::env::temp_dir().join(format!(
            "cell-one-prepared-mismatch-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        write_binding_proposal(&dir, "rust_idiom", Some("rust_idiom"));
        let err = require_prepared_matches_picked(&dir, &proposal(KIND_SEAT, "ag_news")).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains("refuse:proposal:"), "{text}");
        assert!(text.contains("prepared binding_id rust_idiom"), "{text}");
        assert!(text.contains("picked ag_news"), "{text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn prepared_mismatch_proposed_binding_id_refuses() {
        let dir = std::env::temp_dir().join(format!(
            "cell-one-prepared-proposed-mismatch-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        write_binding_proposal(&dir, "ag_news", Some("rust_idiom"));
        let err = require_prepared_matches_picked(&dir, &proposal(KIND_SEAT, "ag_news")).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains("refuse:proposal:"), "{text}");
        assert!(text.contains("proposed_binding.id rust_idiom"), "{text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn prepared_mismatch_proposal_id_refuses() {
        let dir = std::env::temp_dir().join(format!(
            "cell-one-prepared-id-mismatch-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        write_binding_proposal(&dir, "ag_news", Some("ag_news"));
        let mut picked = proposal(KIND_SEAT, "ag_news");
        picked.id = "specialty-seat:rust_idiom".into();
        let err = require_prepared_matches_picked(&dir, &picked).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains("refuse:proposal:"), "{text}");
        assert!(text.contains("prepared proposal specialty-seat:ag_news"), "{text}");
        assert!(text.contains("picked specialty-seat:rust_idiom"), "{text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn prepared_missing_binding_proposal_refuses() {
        let dir = std::env::temp_dir().join(format!(
            "cell-one-prepared-missing-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let err = require_prepared_matches_picked(&dir, &proposal(KIND_SEAT, "ag_news")).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains("refuse:prepared:"), "{text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_package_refuses_mismatched_prepared_without_rewriting_lab() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let locked = root.join("examples/estate.yaml");
        let locked_before = fs::read(&locked).unwrap();
        let cksum_before = file_cksum(&locked).unwrap();
        assert!(
            cksum_before.starts_with(LOCKED_CKSUM),
            "{cksum_before}"
        );
        let out = std::env::temp_dir().join(format!(
            "cell-one-apply-mismatch-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&out);
        let staged =
            specialty_bind::stage_one_specialty_for_apply(&root, &out, "rust_idiom").unwrap();
        let lab_before = fs::read(&staged.lab).unwrap();
        let package_path = out.join("improvement-package.json");
        fs::write(
            &package_path,
            r#"{
  "schema": "cell-one.improvement-package.v0",
  "id": "improvement-from-decisions",
  "kind": "standing-improvement",
  "auto_train": false,
  "train_invoked": false,
  "ready_for_live_test": false,
  "live_pass_recorded": false,
  "live_sync": false,
  "journal": {
    "path": "journal",
    "receipts": 1,
    "surface_authorize": 1,
    "surface_convey": 1,
    "surface_complete": 1,
    "specialty_seats": ["ag_news"]
  },
  "proposals": [
    {
      "id": "specialty-seat:ag_news",
      "kind": "specialty-seat",
      "binding_id": "ag_news",
      "dataset": "ag_news",
      "action": "enrich-prepare",
      "auto_train": false,
      "note": "test"
    }
  ],
  "note": "test"
}
"#,
        )
        .unwrap();
        let err = cmd_decisions_apply_package(
            &package_path,
            &staged.lab,
            &staged.prepared,
            &staged.state,
            &staged.plans,
            &staged.roots,
            true,
            Some("specialty-seat:ag_news"),
            Some(&staged.tag),
            &root,
            &root.join("policy/cell-one.policy.v0.yaml"),
            "jason",
            None,
        )
        .unwrap_err();
        let text = format!("{err:#}");
        assert!(
            text.contains("refuse:proposal:") || text.contains("refuse:prepared:"),
            "{text}"
        );
        assert!(text.contains("prepared binding_id rust_idiom"), "{text}");
        assert!(text.contains("picked ag_news"), "{text}");
        assert_eq!(fs::read(&staged.lab).unwrap(), lab_before);
        assert!(!estate_has_binding(&staged.lab, "ag_news").unwrap());
        assert!(!estate_has_binding(&staged.lab, "rust_idiom").unwrap());
        assert!(!staged.state.join("enrich-stage/staged-estate.yaml").is_file());
        assert_eq!(fs::read(&locked).unwrap(), locked_before);
        assert_eq!(file_cksum(&locked).unwrap(), cksum_before);
        let _ = fs::remove_dir_all(&out);
    }
}
