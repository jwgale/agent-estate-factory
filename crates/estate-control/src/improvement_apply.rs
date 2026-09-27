//! Gated apply of one standing improvement-package proposal.
//!
//! `estate decisions apply-package` picks one specialty-seat proposal
//! from a `cell-one.improvement-package.v0` and drives it through the
//! existing enrich apply-proposal → plan → apply `--require-plan`
//! path. `--require-plan` is required. Train is not invoked.
//! `auto_train` stays false.
//!
//! `estate decisions improvement-apply-prove` reuses host-validate
//! receipts → export-package, then applies one proposal on a throwaway
//! apply lab. Standing next is joinable after the gated apply. Locked
//! `examples/estate.yaml` is refused. Apply without a plan is refused.
//! `READY_FOR_LIVE_TEST` stays no. Sibling of improvement-export-prove;
//! cohesion-prove can compose this later.

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
        bail!("refuse:plan: apply-package requires --require-plan");
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
    let picked = improvement_export::pick_specialty_seat(&package, None)?.clone();
    refuse_train_on_proposal(&picked)?;
    println!(
        "improvement-apply-prove: pick {} kind={} binding={}",
        proposal_id(&picked),
        picked.kind,
        picked.binding_id
    );

    let apply_out = out.join("apply");
    let _ = fs::remove_dir_all(&apply_out);
    fs::create_dir_all(&apply_out)?;
    println!("improvement-apply-prove: stage");
    let staged = specialty_bind::stage_one_specialty_for_apply(&root, &apply_out, &picked.binding_id)?;
    if staged.binding_id != picked.binding_id {
        bail!(
            "refuse:proposal: staged {} want {}",
            staged.binding_id,
            picked.binding_id
        );
    }
    if estate_has_binding(&staged.lab, &picked.binding_id)? {
        bail!(
            "refuse:binding: {} landed before gated apply",
            picked.binding_id
        );
    }
    if fs::read(&locked)? != before {
        bail!("refuse:estate: stage rewrote examples/estate.yaml");
    }

    let package_path = package_dir.join("improvement-package.json");
    let policy = root.join("policy/cell-one.policy.v0.yaml");
    let lab_before = fs::read(&staged.lab)?;

    println!("improvement-apply-prove: refuse-without-plan");
    match cmd_decisions_apply_package(
        &package_path,
        &staged.lab,
        &staged.prepared,
        &staged.state,
        &staged.plans,
        &staged.roots,
        false,
        Some(&proposal_id(&picked)),
        Some(&staged.tag),
        &root,
        &policy,
        "jason",
        None,
    ) {
        Ok(_) => bail!("refuse:plan: apply-package without --require-plan must refuse"),
        Err(err) => {
            let text = format!("{err:#}");
            if !text.contains("refuse:plan: apply-package requires --require-plan") {
                bail!("refuse:plan: unexpected refuse without plan: {text}");
            }
            println!("refuse:plan: apply-package requires --require-plan");
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

    println!("improvement-apply-prove: apply --require-plan");
    let receipt_path = apply_out.join("improvement-apply.json");
    let applied = cmd_decisions_apply_package(
        &package_path,
        &staged.lab,
        &staged.prepared,
        &staged.state,
        &staged.plans,
        &staged.roots,
        true,
        Some(&proposal_id(&picked)),
        Some(&staged.tag),
        &root,
        &policy,
        "jason",
        Some(&receipt_path),
    )?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: gated apply rewrote examples/estate.yaml");
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

    println!("improvement-apply-prove: standing-next");
    crate::enrich::cmd_enrich_standing_next(&staged.lab, &staged.prepared)?;
    let joinable = applied
        .get("joinable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !joinable {
        bail!("refuse:standing: gated apply is not joinable");
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
        "applied_proposal_id": proposal_id(&picked),
        "applied_proposal_kind": picked.kind,
        "binding_id": picked.binding_id,
        "joinable": true,
        "standing": "joinable: yes",
        "require_plan": true,
        "refuse_without_plan": "refuse:plan: apply-package requires --require-plan",
        "apply_receipt": receipt_path.display().to_string(),
        "lab_estate": staged.lab.display().to_string(),
        "note": "Fixture prove. Reuses host-validate authorize / convey / complete --mock receipts, export-package, then gated apply of one specialty-seat proposal through apply-proposal → plan → apply --require-plan. Standing next is joinable. auto_train=false. Train not invoked. Not a live PASS."
    });
    write_prove_report(&out, &body)?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    println!(
        "applied: {} kind={} binding={} joinable=yes auto_train=false train_invoked=no",
        proposal_id(&picked),
        picked.kind,
        picked.binding_id
    );
    println!("improvement-apply-prove: ok");
    println!("READY_FOR_LIVE_TEST: no");
    Ok(())
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
        "applied improvement proposal {} kind={} binding={} joinable=yes auto_train=false train_invoked=no",
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
            .unwrap_or("-")
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
    use crate::improvement_export::JournalCite;

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
}
