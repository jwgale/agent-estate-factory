use anyhow::{bail, Context, Result};
use conveyor_proxy::load_mesh;
use estate_schema::{
    blast_grows, covering_plan, describe_agents_section, describe_placements, diff_estates,
    estate_hash, latest_plan, load_estate, load_plan_json, mark_plan_reviewed, overlay_sacred_ids,
    plan_against_is_fresh, plan_against_is_fresh_strict, plan_blast_width, plan_is_reviewable,
    render_plan, render_plan_diff, render_plan_pr, write_plan,
};
use feed_collector::{import_pack_for, refuse_import_pack};
use floor_supervisor::{
    append_apply_audit, apply_dry_run, apply_with_profile_dir, classify_apply, list_expired_leases,
    load_desired_snapshot, load_lifecycle, load_placements, mark_running, now_unix,
    record_placements, refuse_expired_leases, refuse_lease_host_classes,
    refuse_spawned_cloud_placement, render_dry_run,
    ApplyAudit, ApplyIdentity,
};
use std::path::Path;

use crate::helpers::{chrono_stamp, enforce_policy, snapshot_state_files};

pub(crate) fn cmd_plan(
    path: &Path,
    against: Option<&Path>,
    plans_dir: &Path,
    state_dir: &Path,
    reviewed: bool,
    reviewed_dir: &Path,
) -> Result<()> {
    let parsed = estate_schema::load_estate_unvalidated(path)
        .with_context(|| format!("desired {}", path.display()))?;
    let frontier_plan = model_estate::frontier_plan_view(&parsed)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let desired = load_estate(path).with_context(|| format!("desired {}", path.display()))?;
    let previous = match against {
        Some(p) => Some(load_estate(p).with_context(|| format!("against {}", p.display()))?),
        None => load_desired_snapshot(state_dir)?,
    };
    let plan = diff_estates(&desired, previous.as_ref());
    let written = write_plan(plans_dir, &plan)?;
    print!("{}", render_plan(&plan));
    println!("{}", model_estate::render_frontier_plan(&frontier_plan));
    println!("Wrote {}", written.display());
    // Model-class cites, then declared tool / MCP / mount cites, sit after
    // the Agents / Security-as-IaC blast already printed by render_plan,
    // and before hop refuse. Allow is quiet. Deny and deny-default are
    // notes. A hard cite bails. Plan does not yet refuse agent-call or
    // memory. Missing mesh is an empty hop cite list, not a failure. A
    // present file that does not parse stays the mesh error and is not
    // rewritten. The plan file above is the plan artifact. Mesh, leases,
    // and the estate stay unread-only. A reviewed copy stays unwritten
    // on bail.
    let hop_mismatch = plan_cites_before_hop(&desired, state_dir)?;
    if !plan_is_reviewable(&plan) {
        bail!("plan is not reviewable (need schema cell-one.plan.v0 + blast radius)");
    }
    if let Some(line) = hop_mismatch {
        bail!(line);
    }
    if reviewed {
        let stem = written
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let dest = mark_plan_reviewed(plans_dir, reviewed_dir, Some(stem))?;
        println!("reviewed copy {}", dest.display());
    }
    Ok(())
}

pub(crate) fn cmd_plan_diff(
    from: Option<&Path>,
    to: Option<&Path>,
    estate: &Path,
    state_dir: &Path,
    plans_dir: &Path,
    allow_wider: bool,
) -> Result<()> {
    let (from_plan, to_plan) = resolve_plan_pair(from, to, estate, state_dir, plans_dir)?;
    print!("{}", render_plan_diff(&from_plan, &to_plan));
    let from_w = plan_blast_width(&from_plan);
    let to_w = plan_blast_width(&to_plan);
    if blast_grows(&from_plan, &to_plan) && !allow_wider {
        bail!("refuse:wider: blast radius grew {from_w} -> {to_w} (pass --allow-wider)");
    }
    if blast_grows(&from_plan, &to_plan) && allow_wider {
        println!("wider allowed (--allow-wider)");
    } else {
        println!("plan diff ok (width {from_w} -> {to_w})");
    }
    Ok(())
}

pub(crate) fn cmd_plan_export_pr(
    estate_path: &Path,
    state_dir: &Path,
    plans_dir: &Path,
    reviewed_dir: &Path,
    out: &Path,
) -> Result<()> {
    let desired = load_estate(estate_path)
        .with_context(|| format!("desired {}", estate_path.display()))?;
    let previous = load_desired_snapshot(state_dir)?;
    let plan = diff_estates(&desired, previous.as_ref());
    let hash = estate_hash(&desired);
    let covering = covering_plan(plans_dir, &hash)?;
    let covering_stem = covering.as_ref().map(|c| c.stem.clone());
    let reviewed = covering_stem.as_ref().is_some_and(|stem| {
        reviewed_dir.join(format!("{stem}.md")).is_file()
            || reviewed_dir.join(format!("{stem}.json")).is_file()
    });
    let mut risks = Vec::new();
    if desired
        .placements
        .iter()
        .any(|p| p.kind == estate_schema::PlacementKind::CloudAgent)
    {
        risks.push("cloud-agent: declared, not spawned".into());
    }
    if !desired.sacred_exclusions.is_empty() {
        risks.push(format!(
            "sacred exclusions on estate: {}",
            desired
                .sacred_exclusions
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let overlays = overlay_sacred_ids();
    if !overlays.is_empty() {
        risks.push(format!("sacred overlays: {}", overlays.join(", ")));
    }
    if covering.is_none() {
        risks.push("no covering plan on disk; gated apply will refuse".into());
    } else if covering
        .as_ref()
        .is_some_and(|c| !plan_is_reviewable(&c.plan))
    {
        risks.push("covering plan is not reviewable".into());
    }
    if !reviewed {
        risks.push("plan is not marked reviewed".into());
    }
    let expired = list_expired_leases(state_dir, now_unix())?;
    if !expired.is_empty() {
        risks.push(format!(
            "expired placement leases: {}",
            expired
                .iter()
                .map(|l| l.placement_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let md = render_plan_pr(&plan, covering_stem.as_deref(), reviewed, &risks);
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(out, &md)?;
    print!("{md}");
    println!("Wrote {}", out.display());
    Ok(())
}

pub(crate) fn looks_plan_json(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
}

pub(crate) fn resolve_plan_pair(
    from: Option<&Path>,
    to: Option<&Path>,
    estate: &Path,
    state_dir: &Path,
    plans_dir: &Path,
) -> Result<(estate_schema::EstatePlan, estate_schema::EstatePlan)> {
    match (from, to) {
        (Some(a), Some(b)) if looks_plan_json(a) && looks_plan_json(b) => {
            Ok((load_plan_json(a)?, load_plan_json(b)?))
        }
        (Some(a), Some(b)) if !looks_plan_json(a) && !looks_plan_json(b) => {
            let old_e = load_estate(a).with_context(|| format!("from {}", a.display()))?;
            let new_e = load_estate(b).with_context(|| format!("to {}", b.display()))?;
            Ok((diff_estates(&old_e, None), diff_estates(&new_e, Some(&old_e))))
        }
        (Some(a), Some(b)) if looks_plan_json(a) && !looks_plan_json(b) => {
            let old_p = load_plan_json(a)?;
            let new_e = load_estate(b).with_context(|| format!("to {}", b.display()))?;
            let against = load_desired_snapshot(state_dir)?;
            Ok((old_p, diff_estates(&new_e, against.as_ref())))
        }
        (Some(a), Some(b)) => {
            let old_e = load_estate(a).with_context(|| format!("from {}", a.display()))?;
            let to_p = load_plan_json(b)?;
            Ok((diff_estates(&old_e, None), to_p))
        }
        (Some(a), None) if looks_plan_json(a) => {
            let old_p = load_plan_json(a)?;
            let new_e = load_estate(estate).with_context(|| format!("estate {}", estate.display()))?;
            let against = load_desired_snapshot(state_dir)?;
            Ok((old_p, diff_estates(&new_e, against.as_ref())))
        }
        (Some(a), None) => {
            let old_e = load_estate(a).with_context(|| format!("from {}", a.display()))?;
            let new_e = load_estate(estate).with_context(|| format!("estate {}", estate.display()))?;
            Ok((diff_estates(&old_e, None), diff_estates(&new_e, Some(&old_e))))
        }
        (None, Some(b)) if looks_plan_json(b) => {
            let to_p = load_plan_json(b)?;
            let from_p = last_applied_plan(state_dir, plans_dir)?
                .ok_or_else(|| anyhow::anyhow!("no last-applied plan; pass --from"))?;
            Ok((from_p, to_p))
        }
        (None, to_path) => {
            let new_path = to_path.unwrap_or(estate);
            let new_e = load_estate(new_path)
                .with_context(|| format!("desired {}", new_path.display()))?;
            let last = load_desired_snapshot(state_dir)?;
            let from_p = last_applied_plan(state_dir, plans_dir)?.ok_or_else(|| {
                anyhow::anyhow!("no last-applied snapshot or plan; pass --from")
            })?;
            Ok((from_p, diff_estates(&new_e, last.as_ref())))
        }
    }
}

pub(crate) fn last_applied_plan(
    state_dir: &Path,
    plans_dir: &Path,
) -> Result<Option<estate_schema::EstatePlan>> {
    if let Some(snap) = load_desired_snapshot(state_dir)? {
        if let Some(covering) = covering_plan(plans_dir, &estate_hash(&snap))? {
            return Ok(Some(covering.plan));
        }
        if let Some(latest) = latest_plan(plans_dir)? {
            return Ok(Some(latest));
        }
        return Ok(Some(diff_estates(&snap, None)));
    }
    Ok(latest_plan(plans_dir)?)
}

pub(crate) fn cmd_apply(
    path: &Path,
    state_dir: &Path,
    roots_base: &Path,
    plans_dir: &Path,
    require_plan: bool,
    import_pack_id: Option<&str>,
    packs_dir: &Path,
    require_fresh_plan: bool,
    dry_run: bool,
    policy: &Path,
    curator: &str,
    force: bool,
) -> Result<()> {
    if dry_run {
        let parsed = estate_schema::load_estate_unvalidated(path)
            .with_context(|| format!("load {}", path.display()))?;
        model_estate::frontier_plan_view(&parsed).map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    enforce_policy(policy, "apply", None)?;
    if let Some(id) = import_pack_id {
        refuse_import_pack(
            packs_dir,
            id,
            curator,
            &estate.enrich_packs.curator,
            &estate,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    load_lifecycle(state_dir)?;
    // Even --force. A spawned cloud lease is not restamped to unspawned.
    refuse_spawned_cloud_placement(state_dir).map_err(|e| anyhow::anyhow!("{e}"))?;
    if !force {
        if let Some(places) = load_placements(state_dir)? {
            refuse_lease_host_classes(&places)?;
        }
    }
    refuse_apply_catalog_mismatch(&estate, state_dir)?;
    // Cites before any estate, mesh, lease, or apply-audit write, including
    // dry-run. Order: Agents section, agent-call, memory, model class,
    // declared tool/mcp/mount, hop. Allow is quiet. Deny and deny-default
    // are notes. A hard cite bails here.
    apply_cites_before_writes(&estate, state_dir)?;
    if dry_run {
        return cmd_apply_dry_run(
            &estate,
            path,
            state_dir,
            plans_dir,
            require_plan,
            require_fresh_plan,
            roots_base,
            force,
        );
    }
    refuse_expired_leases(state_dir)?;
    let hash = estate_hash(&estate);
    let covering = covering_plan(plans_dir, &hash)?;
    let covering_stem = covering.as_ref().map(|c| c.stem.clone());
    if (require_plan || require_fresh_plan) && covering.is_none() {
        bail!("apply gated: no plan on disk for {hash}; run estate plan first");
    }
    let last_applied = load_desired_snapshot(state_dir)?
        .as_ref()
        .map(estate_hash);
    if let Some(c) = covering.as_ref() {
        if (require_plan || require_fresh_plan) && !plan_is_reviewable(&c.plan) {
            bail!("apply gated: covering plan is not reviewable (need schema cell-one.plan.v0 + blast radius)");
        }
    }
    let fresh = covering
        .as_ref()
        .map(|c| {
            if require_fresh_plan {
                plan_against_is_fresh_strict(&c.plan, last_applied.as_deref())
            } else {
                plan_against_is_fresh(&c.plan, last_applied.as_deref())
            }
        })
        .unwrap_or(true);
    if require_fresh_plan && !fresh {
        bail!(
            "apply gated: covering plan is stale; against_hash does not match last apply {} (greenfield after apply is refuse)",
            last_applied.unwrap_or_else(|| "-".into())
        );
    }
    model_estate::refuse_staged_apply(path, state_dir, &hash, require_plan)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let identity = classify_apply(&estate, state_dir, roots_base)?;
    if import_pack_id.is_none() {
        match &identity {
            ApplyIdentity::Unchanged if !force => {
                let note = enrich_stage_note(
                    path,
                    state_dir,
                    require_plan,
                    &hash,
                    "unchanged: identical desired state; in_sync. No-op apply.",
                )?;
                let audit = ApplyAudit {
                    created_at: chrono_stamp(),
                    desired_hash: hash.clone(),
                    sessions: 0,
                    imported_packs: vec![],
                    require_plan,
                    note,
                    covering_plan: covering_stem,
                    cloud_agent_spawned: false,
                    fresh_plan: fresh,
                    unchanged: true,
                    forced: false,
                };
                let audit_path = append_apply_audit(plans_dir, state_dir, &audit)?;
                println!("apply unchanged (desired hash matches last apply; in_sync)");
                println!("audit {}", audit_path.display());
                return Ok(());
            }
            ApplyIdentity::Drift { notes } if !force => {
                eprintln!("refuse:drift: identical desired hash but actual drifted");
                for note in notes {
                    eprintln!("  {note}");
                }
                bail!("refuse:drift: pass --force to reconverge (clear reason required)");
            }
            ApplyIdentity::LeaseRefresh { missing } => {
                println!(
                    "apply lease-refresh (forgot expired; restamping {})",
                    missing.join(", ")
                );
            }
            _ => {}
        }
    }
    let mut imported = Vec::new();
    if let Some(id) = import_pack_id {
        let ids: Vec<String> = estate
            .enrich_packs
            .packs
            .iter()
            .map(|p| p.id.clone())
            .collect();
        let (rec, dest) = import_pack_for(
            packs_dir,
            &packs_dir.join("accepted"),
            id,
            &ids,
            curator,
            &estate.enrich_packs.curator,
            &estate,
        )?;
        imported.push(id.to_string());
        println!(
            "imported pack {} estate_bound={} -> {}",
            rec.pack.id,
            rec.estate_bound,
            dest.display()
        );
    }
    let actual = apply_with_profile_dir(&estate, state_dir, roots_base)?;
    model_estate::record_bindings(&estate, state_dir)?;
    record_placements(&estate, state_dir)?;
    mark_running(&estate, state_dir)?;
    model_estate::write_bound_catalog(&state_dir.join("catalog.json"), &estate)?;
    let note = enrich_stage_note(
        path,
        state_dir,
        require_plan,
        &hash,
        "Apply audit. Estate file unchanged. Cloud-agent placements not spawned.",
    )?;
    let audit = ApplyAudit {
        created_at: chrono_stamp(),
        desired_hash: hash,
        sessions: actual.sessions.len(),
        imported_packs: imported,
        require_plan,
        note,
        covering_plan: covering_stem,
        cloud_agent_spawned: false,
        fresh_plan: fresh,
        unchanged: false,
        forced: force && matches!(identity, ApplyIdentity::Drift { .. } | ApplyIdentity::Unchanged),
    };
    let audit_path = append_apply_audit(plans_dir, state_dir, &audit)?;
    if audit.forced {
        println!("apply --force (drift or unchanged override)");
    }
    println!(
        "applied {} sessions; actual-state {}; audit {}",
        actual.sessions.len(),
        state_dir.join("actual-state.json").display(),
        audit_path.display()
    );
    for session in &actual.sessions {
        println!(
            "  {} -> {} ({})",
            session.agent_id,
            session.desktop,
            session.isolation_handle.as_token()
        );
    }
    println!("{}", describe_placements(&estate));
    Ok(())
}

/// Agents section, then coverage cites, before any estate, mesh, lease, or
/// apply-audit write. Order matches doctor on model class versus declared
/// coverage: model class, then tool / MCP / mount. Agent-call and memory
/// stay ahead of that pair. Hop stays last. Allow is quiet. Deny and
/// deny-default are notes. A hard cite bails. Does not write.
fn apply_cites_before_writes(estate: &estate_schema::Estate, state_dir: &Path) -> Result<()> {
    println!("{}", describe_agents_section(estate));
    refuse_apply_agent_call_coverage(estate)?;
    refuse_apply_memory_coverage(estate)?;
    refuse_apply_model_class_coverage(estate)?;
    refuse_apply_declared_coverage(estate)?;
    refuse_apply_hop_coverage(estate, state_dir)?;
    Ok(())
}

/// Declared agent-call rows. The line matches plan, drift, and doctor
/// (`own` or `peer`). Allow is quiet. Deny and deny-default do not fail
/// apply. A target that is not an estate agent fails before writes.
/// Does not write.
fn refuse_apply_agent_call_coverage(estate: &estate_schema::Estate) -> Result<()> {
    let mut hard: Option<String> = None;
    for cite in estate_schema::agent_call_coverage_cites(estate) {
        if cite.fail {
            println!("  FAIL  {}", cite.line);
            hard.get_or_insert(cite.line);
        } else {
            println!("  note  {}", cite.line);
        }
    }
    if let Some(line) = hard {
        bail!(line);
    }
    Ok(())
}

/// Memory-read rows. The line matches plan, drift, and doctor
/// (`own-lane` or `cross-lane`). Allow is quiet. Deny and deny-default
/// do not fail apply. A missing edge or an unrecognized word fails before
/// writes. Does not write.
fn refuse_apply_memory_coverage(estate: &estate_schema::Estate) -> Result<()> {
    let mut hard: Option<String> = None;
    for cite in estate_schema::memory_coverage_cites(estate) {
        if cite.fail {
            println!("  FAIL  {}", cite.line);
            hard.get_or_insert(cite.line);
        } else {
            println!("  note  {}", cite.line);
        }
    }
    if let Some(line) = hard {
        bail!(line);
    }
    Ok(())
}

/// Model-class rows. The line matches plan, drift, and doctor (`frontier`
/// or `local`). Allow is quiet. Deny and deny-default do not fail apply.
/// A missing row, a class token other than frontier or local, or an
/// unrecognized word fails before writes (`refuse:model-class`). Does not
/// write. Model rows have no capability-mismatch class.
fn refuse_apply_model_class_coverage(estate: &estate_schema::Estate) -> Result<()> {
    refuse_model_class_coverage(estate)
}

/// Model-class rows on plan, after the Agents / Security-as-IaC blast and
/// before hop. Same lines as apply. Allow is quiet. Deny and deny-default
/// are notes and do not fail plan. A missing row, a class token other than
/// frontier or local, or an unrecognized word fails closed
/// (`refuse:model-class`). Does not write the mesh, the leases, or the
/// estate. Model rows have no capability-mismatch class.
fn refuse_plan_model_class_coverage(estate: &estate_schema::Estate) -> Result<()> {
    refuse_model_class_coverage(estate)
}

/// Shared FAIL / note printer. Lines come from `model_class_coverage_cites`
/// (the describe row, with `refuse:model-class` on a hard cite). Allow is
/// omitted by that citer.
fn refuse_model_class_coverage(estate: &estate_schema::Estate) -> Result<()> {
    let mut hard: Option<String> = None;
    for cite in estate_schema::model_class_coverage_cites(estate) {
        if cite.fail {
            println!("  FAIL  {}", cite.line);
            hard.get_or_insert(cite.line);
        } else {
            println!("  note  {}", cite.line);
        }
    }
    if let Some(line) = hard {
        bail!(line);
    }
    Ok(())
}

/// Model-class cites, then declared tool / MCP / mount cites, then hop.
/// A hard model-class or declared cite bails before hop is read. Hop
/// mismatch is returned so an unreviewable plan can still fail first.
/// Does not write the mesh, the leases, or the estate.
fn plan_cites_before_hop(
    estate: &estate_schema::Estate,
    state_dir: &Path,
) -> Result<Option<String>> {
    refuse_plan_model_class_coverage(estate)?;
    refuse_plan_declared_coverage(estate)?;
    hop_cites(estate, state_dir)
}

/// Same order as [`plan_cites_before_hop`], with a supplied declared-cite
/// list. Production uses [`plan_cites_before_hop`]. A test passes a missing
/// row or an unrecognized word, which the row builder does not emit.
#[cfg(test)]
fn plan_cites_before_hop_with(
    estate: &estate_schema::Estate,
    state_dir: &Path,
    declared: &[estate_schema::DeclaredCoverageCite],
) -> Result<Option<String>> {
    refuse_plan_model_class_coverage(estate)?;
    refuse_declared_cites(declared)?;
    hop_cites(estate, state_dir)
}

fn hop_cites(estate: &estate_schema::Estate, state_dir: &Path) -> Result<Option<String>> {
    let mut hop_mismatch: Option<String> = None;
    match load_mesh(state_dir) {
        Ok(mesh) => {
            for cite in crate::watch::hop_coverage_cites(estate, &mesh) {
                if cite.fail {
                    println!("  FAIL  {}", cite.line);
                    hop_mismatch.get_or_insert(cite.line);
                } else {
                    println!("  note  {}", cite.line);
                }
            }
        }
        Err(err) => bail!("{err}"),
    }
    Ok(hop_mismatch)
}

/// Declared tool, MCP, and mount rows. The line matches plan, drift, and
/// doctor. Allow is quiet. Deny and deny-default do not fail apply. A
/// missing row or an unrecognized word fails before writes
/// (`refuse:tool`, `refuse:mcp`, or `refuse:mount`). Does not write.
fn refuse_apply_declared_coverage(estate: &estate_schema::Estate) -> Result<()> {
    refuse_declared_coverage(estate)
}

/// Declared tool, MCP, and mount rows on plan, after model-class cites and
/// before hop. Same lines as apply. Allow is quiet. Deny and deny-default
/// are notes and do not fail plan. A missing row or an unrecognized word
/// fails closed (`refuse:tool`, `refuse:mcp`, or `refuse:mount`). Does not
/// write the mesh, the leases, or the estate. These rows have no
/// capability-mismatch class.
fn refuse_plan_declared_coverage(estate: &estate_schema::Estate) -> Result<()> {
    refuse_declared_coverage(estate)
}

/// Shared FAIL / note printer. Lines come from `declared_coverage_cites`
/// (the describe row, with `refuse:tool`, `refuse:mcp`, or `refuse:mount`
/// on a hard cite). Allow is omitted by that citer.
fn refuse_declared_coverage(estate: &estate_schema::Estate) -> Result<()> {
    let cites = estate_schema::declared_coverage_cites(estate);
    refuse_declared_cites(&cites)
}

fn refuse_declared_cites(cites: &[estate_schema::DeclaredCoverageCite]) -> Result<()> {
    let mut hard: Option<String> = None;
    for cite in cites {
        if cite.fail {
            println!("  FAIL  {}", cite.line);
            hard.get_or_insert_with(|| cite.line.clone());
        } else {
            println!("  note  {}", cite.line);
        }
    }
    if let Some(line) = hard {
        bail!(line);
    }
    Ok(())
}

/// Shared with plan, doctor, and drift. Prints `refuse:hop-coverage` cites.
/// Mismatch fails apply. Deny and deny-default do not. Does not write.
fn refuse_apply_hop_coverage(estate: &estate_schema::Estate, state_dir: &Path) -> Result<()> {
    let mut hop_mismatch: Option<String> = None;
    match load_mesh(state_dir) {
        Ok(mesh) => {
            for cite in crate::watch::hop_coverage_cites(estate, &mesh) {
                if cite.fail {
                    println!("  FAIL  {}", cite.line);
                    hop_mismatch.get_or_insert(cite.line);
                } else {
                    println!("  note  {}", cite.line);
                }
            }
        }
        Err(err) => bail!("{err}"),
    }
    if let Some(line) = hop_mismatch {
        bail!(line);
    }
    Ok(())
}

/// Cell `catalog.json` versus the estate being applied, resumed, or
/// pause-proved. Missing file is not a disagreement. A present file that
/// does not parse, or whose frontier model disagrees, refuses before any
/// write. The schema card is not the binding. `--force` does not bypass this.
pub(crate) fn refuse_apply_catalog_mismatch(
    estate: &estate_schema::Estate,
    state_dir: &Path,
) -> Result<()> {
    let path = state_dir.join("catalog.json");
    if !path.is_file() {
        return Ok(());
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let catalog = model_estate::frontier_model_from_catalog_json(&text).map_err(|err| {
        anyhow::anyhow!(
            "refuse:frontier-model: cell catalog unreadable ({err}); schema card is not the binding"
        )
    })?;
    let bound =
        model_estate::bound_frontier_model(estate).map_err(|err| anyhow::anyhow!("{err}"))?;
    model_estate::refuse_catalog_frontier_mismatch(bound.as_deref(), catalog.as_deref())
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    Ok(())
}

pub(crate) fn cmd_apply_dry_run(
    estate: &estate_schema::Estate,
    path: &Path,
    state_dir: &Path,
    plans_dir: &Path,
    require_plan: bool,
    require_fresh_plan: bool,
    roots_base: &Path,
    force: bool,
) -> Result<()> {
    let frontier_plan =
        model_estate::frontier_plan_view(estate).map_err(|e| anyhow::anyhow!("{e}"))?;
    let hash = estate_hash(estate);
    let covering = covering_plan(plans_dir, &hash)?;
    let last_applied = load_desired_snapshot(state_dir)?
        .as_ref()
        .map(estate_hash);
    let identity = classify_apply(estate, state_dir, roots_base)?;
    let mut extra_refuses = Vec::new();
    if let ApplyIdentity::Drift { notes } = &identity {
        if !force {
            extra_refuses.push(format!(
                "refuse:drift: identical desired hash but actual drifted ({})",
                notes.join("; ")
            ));
        }
    }
    if (require_plan || require_fresh_plan) && covering.is_none() {
        extra_refuses.push(format!(
            "refuse:no-plan: no covering plan for {hash}; run estate plan first"
        ));
    }
    if let Err(err) = model_estate::refuse_staged_apply(path, state_dir, &hash, require_plan) {
        extra_refuses.push(err.to_string());
    }
    if let Some(c) = covering.as_ref() {
        if (require_plan || require_fresh_plan) && !plan_is_reviewable(&c.plan) {
            extra_refuses.push(
                "refuse:plan: covering plan is not reviewable".into(),
            );
        }
        if require_fresh_plan
            && !plan_against_is_fresh_strict(&c.plan, last_applied.as_deref())
        {
            extra_refuses.push("refuse:stale-plan: covering plan is stale".into());
        }
    }
    let before = snapshot_state_files(state_dir);
    let report = apply_dry_run(estate, state_dir)?;
    let after = snapshot_state_files(state_dir);
    if before != after {
        bail!("dry-run must not write leases or state files");
    }
    print!("{}", render_dry_run(&report));
    println!("{}", model_estate::render_frontier_plan(&frontier_plan));
    println!("estate: {}", path.display());
    if !extra_refuses.is_empty() {
        for line in &extra_refuses {
            println!("  {line}");
        }
        bail!("apply dry-run would-refuse");
    }
    if report.would_refuse {
        bail!("apply dry-run would-refuse");
    }
    if matches!(identity, ApplyIdentity::Unchanged) && !force {
        println!("dry-run unchanged (desired hash matches last apply; in_sync)");
        println!("estate: {}", path.display());
        return Ok(());
    }
    println!("dry-run ok (no writes)");
    Ok(())
}

fn enrich_stage_note(
    path: &Path,
    state_dir: &Path,
    require_plan: bool,
    hash: &str,
    unchanged_note: &str,
) -> Result<String> {
    match model_estate::commit_enrich_stage(path, state_dir, require_plan, hash)
        .map_err(|err| anyhow::anyhow!("{err}"))?
    {
        model_estate::EnrichStageCommit::Wrote { source } => {
            println!("enrich stage wrote {}", source.display());
            Ok(
                "Apply audit. Enrich stage wrote the source estate under --require-plan. Cloud-agent placements not spawned."
                    .into(),
            )
        }
        model_estate::EnrichStageCommit::Held => {
            println!("enrich stage held: source estate not written (pass --require-plan)");
            Ok(
                "Apply audit. Enrich stage held; source estate not written. Cloud-agent placements not spawned."
                    .into(),
            )
        }
        _ => Ok(unchanged_note.to_string()),
    }
}

#[cfg(test)]
mod plan_hop_coverage_tests {
    use super::cmd_plan;
    use crate::watch::hop_coverage_cites;
    use conveyor_proxy::{persist_mesh, ConveyorMesh, HopDecl, HopLease, MESH_FILE};
    use estate_schema::load_estate;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn scratch() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-plan-hop-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn box_lease(hop_id: &str, capability: &str, agents: &[&str]) -> HopLease {
        HopLease {
            hop_id: hop_id.into(),
            kind: "box".into(),
            capability: capability.into(),
            host_class: "any".into(),
            granted: true,
            spawned: true,
            durable: true,
            driver: "box".into(),
            note: None,
            ttl_secs: None,
            issued_at: None,
            expires_at: None,
            agents: agents.iter().map(|agent| (*agent).to_string()).collect(),
        }
    }

    fn mesh_with(leases: Vec<HopLease>, hops: Vec<HopDecl>) -> ConveyorMesh {
        ConveyorMesh {
            schema: conveyor_proxy::MESH_SCHEMA.into(),
            hops,
            leases,
        }
    }

    fn allow_copy(dir: &Path) -> PathBuf {
        let path = dir.join("allow.yaml");
        let mut text = fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
        text = text.replace(
            "      - id: notes-append\n",
            "      - id: notes-append\n      - id: lane-tool\n",
        );
        text = text.replace(
            "intentions: []\n",
            "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        );
        fs::write(&path, text).unwrap();
        path
    }

    fn plan(dir: &Path, estate: &Path, state: &Path) -> anyhow::Result<()> {
        cmd_plan(
            estate,
            None,
            &dir.join("plans"),
            state,
            false,
            &dir.join("reviewed"),
        )
    }

    fn snapshot(state: &Path) -> Vec<(String, Vec<u8>)> {
        [
            "placement-actual.json",
            "model-actual.json",
            MESH_FILE,
            "conveyor-hops.json",
            "conveyor-leases.json",
        ]
        .into_iter()
        .filter_map(|name| {
            let path = state.join(name);
            path.is_file()
                .then(|| (name.to_string(), fs::read(&path).unwrap()))
        })
        .collect()
    }

    #[test]
    fn plan_cites_mismatch_and_stays_quiet_on_a_match() {
        let dir = scratch();
        let estate_path = allow_copy(&dir);
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&estate_path).unwrap();
        let mismatch = mesh_with(
            vec![box_lease("cell-one-box", "notes-append", &["research"])],
            vec![],
        );
        persist_mesh(&state, &mismatch).unwrap();
        let cites = hop_coverage_cites(&estate, &mismatch);
        assert_eq!(cites.len(), 1);
        assert!(cites[0].fail);
        assert!(
            cites[0].line.contains("refuse:hop-coverage")
                && cites[0].line.contains("(mismatch)")
                && cites[0].line.contains(
                    "capability 'notes-append' does not match hop coverage capability 'lane-tool'"
                ),
            "{}",
            cites[0].line
        );
        let before = snapshot(&state);
        let estate_bytes = fs::read(&estate_path).unwrap();
        let err = plan(&dir, &estate_path, &state).unwrap_err().to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert!(!err.contains("not reviewable"), "{err}");
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);

        let matched = mesh_with(
            vec![box_lease("cell-one-box", "lane-tool", &["research"])],
            vec![],
        );
        persist_mesh(&state, &matched).unwrap();
        assert!(hop_coverage_cites(&estate, &matched).is_empty());
        plan(&dir, &estate_path, &state).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_reviewed_mismatch_leaves_reviewed_dir_untouched() {
        let dir = scratch();
        let estate_path = allow_copy(&dir);
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let reviewed = dir.join("reviewed");
        fs::create_dir_all(&reviewed).unwrap();
        fs::write(reviewed.join("keep.txt"), b"sentinel").unwrap();
        let mismatch = mesh_with(
            vec![box_lease("cell-one-box", "notes-append", &["research"])],
            vec![],
        );
        persist_mesh(&state, &mismatch).unwrap();
        let before = snapshot(&state);
        let estate_bytes = fs::read(&estate_path).unwrap();
        let err = cmd_plan(
            &estate_path,
            None,
            &dir.join("plans"),
            &state,
            true,
            &reviewed,
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);
        assert_eq!(fs::read(reviewed.join("keep.txt")).unwrap(), b"sentinel");
        assert!(!reviewed.join("INDEX.md").exists());
        let extras: Vec<_> = fs::read_dir(&reviewed)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name != "keep.txt")
            .collect();
        assert!(extras.is_empty(), "{extras:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_cites_deny_and_deny_default_without_failing_alone() {
        let dir = scratch();
        let estate_path = dir.join("estate.yaml");
        fs::copy(repo_root().join("examples/estate.yaml"), &estate_path).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&estate_path).unwrap();
        let mesh = mesh_with(
            vec![box_lease("cell-one-box", "notes-append", &["research"])],
            vec![],
        );
        persist_mesh(&state, &mesh).unwrap();
        let cites = hop_coverage_cites(&estate, &mesh);
        assert_eq!(cites.len(), 1);
        assert!(!cites[0].fail);
        assert!(
            cites[0].line.contains("refuse:hop-coverage")
                && cites[0].line.contains("(deny-default)")
                && !cites[0].line.contains("(mismatch)"),
            "{}",
            cites[0].line
        );
        let before = snapshot(&state);
        let estate_bytes = fs::read(&estate_path).unwrap();
        plan(&dir, &estate_path, &state).unwrap();
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);

        let deny_path = dir.join("deny.yaml");
        let mut text = fs::read_to_string(&estate_path).unwrap();
        text = text.replace(
            "      - id: notes-append\n",
            "      - id: notes-append\n      - id: lane-tool\n",
        );
        text = text.replace(
            "intentions: []\n",
            "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: deny\n",
        );
        fs::write(&deny_path, text).unwrap();
        let denied = load_estate(&deny_path).unwrap();
        let deny = hop_coverage_cites(&denied, &mesh);
        assert_eq!(deny.len(), 1);
        assert!(!deny[0].fail);
        assert!(
            deny[0].line.contains("refuse:hop-coverage")
                && deny[0].line.contains("(deny)")
                && !deny[0].line.contains("(mismatch)"),
            "{}",
            deny[0].line
        );
        let deny_before = snapshot(&state);
        let deny_bytes = fs::read(&deny_path).unwrap();
        plan(&dir, &deny_path, &state).unwrap();
        assert_eq!(snapshot(&state), deny_before);
        assert_eq!(fs::read(&deny_path).unwrap(), deny_bytes);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_does_not_cite_a_cloud_kind_variant() {
        let dir = scratch();
        let estate_path = allow_copy(&dir);
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&estate_path).unwrap();
        for kind in ["cloud_mesh", " Cloud-Mesh ", "CLOUD-AGENT"] {
            let mut lease = box_lease("cell-one-box", "notes-append", &["research"]);
            lease.kind = kind.into();
            let leased = mesh_with(vec![lease], vec![]);
            assert!(hop_coverage_cites(&estate, &leased).is_empty(), "{kind}");
            persist_mesh(&state, &leased).unwrap();
            plan(&dir, &estate_path, &state).unwrap();

            let declared = mesh_with(
                vec![],
                vec![HopDecl {
                    id: "cell-one-box".into(),
                    kind: kind.into(),
                    capability: "notes-append".into(),
                    host_class: "any".into(),
                    wired: false,
                    note: None,
                    ttl_secs: None,
                    agents: vec!["research".into()],
                }],
            );
            assert!(hop_coverage_cites(&estate, &declared).is_empty(), "{kind}");
            persist_mesh(&state, &declared).unwrap();
            plan(&dir, &estate_path, &state).unwrap();
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_cites_a_hop_declaration_when_no_lease_is_present() {
        let dir = scratch();
        let estate_path = allow_copy(&dir);
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&estate_path).unwrap();
        let mesh = mesh_with(
            vec![],
            vec![HopDecl {
                id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: vec!["research".into()],
            }],
        );
        persist_mesh(&state, &mesh).unwrap();
        let cites = hop_coverage_cites(&estate, &mesh);
        assert!(
            cites
                .iter()
                .any(|cite| cite.fail && cite.line.contains("(mismatch)")),
            "{cites:?}"
        );
        let before = snapshot(&state);
        let estate_bytes = fs::read(&estate_path).unwrap();
        let err = plan(&dir, &estate_path, &state).unwrap_err().to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn example_estate_plan_stays_green_without_a_mesh() {
        let dir = scratch();
        let estate_path = dir.join("estate.yaml");
        fs::copy(repo_root().join("examples/estate.yaml"), &estate_path).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        plan(&dir, &estate_path, &state).unwrap();
        let locked = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
        assert_eq!(fs::read(&estate_path).unwrap(), locked);
        assert!(!state.join(MESH_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_fails_closed_when_the_mesh_file_does_not_parse() {
        let dir = scratch();
        let estate_path = dir.join("estate.yaml");
        fs::copy(repo_root().join("examples/estate.yaml"), &estate_path).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let mesh_path = state.join(MESH_FILE);
        let corrupt = b"{not-json";
        fs::write(&mesh_path, corrupt).unwrap();
        let before = snapshot(&state);
        let estate_bytes = fs::read(&estate_path).unwrap();
        let err = plan(&dir, &estate_path, &state).unwrap_err().to_string();
        assert!(err.contains("parse") && err.contains(MESH_FILE), "{err}");
        assert!(!err.contains("not reviewable"), "{err}");
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&mesh_path).unwrap(), corrupt);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);
        let _ = fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod apply_hop_coverage_tests {
    use super::cmd_apply;
    use crate::watch::hop_coverage_cites;
    use conveyor_proxy::{persist_mesh, ConveyorMesh, HopDecl, HopLease, MESH_FILE};
    use estate_schema::load_estate;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn scratch() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-apply-hop-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn box_lease(hop_id: &str, capability: &str, agents: &[&str]) -> HopLease {
        HopLease {
            hop_id: hop_id.into(),
            kind: "box".into(),
            capability: capability.into(),
            host_class: "any".into(),
            granted: true,
            spawned: true,
            durable: true,
            driver: "box".into(),
            note: None,
            ttl_secs: None,
            issued_at: None,
            expires_at: None,
            agents: agents.iter().map(|agent| (*agent).to_string()).collect(),
        }
    }

    fn mesh_with(leases: Vec<HopLease>, hops: Vec<HopDecl>) -> ConveyorMesh {
        ConveyorMesh {
            schema: conveyor_proxy::MESH_SCHEMA.into(),
            hops,
            leases,
        }
    }

    fn allow_copy(dir: &Path) -> PathBuf {
        let path = dir.join("allow.yaml");
        let mut text = fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
        text = text.replace(
            "      - id: notes-append\n",
            "      - id: notes-append\n      - id: lane-tool\n",
        );
        text = text.replace(
            "intentions: []\n",
            "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        );
        fs::write(&path, text).unwrap();
        path
    }

    fn apply(dir: &Path, estate: &Path, state: &Path, dry_run: bool) -> anyhow::Result<()> {
        cmd_apply(
            estate,
            state,
            &dir.join("roots"),
            &dir.join("plans"),
            false,
            None,
            &dir.join("packs"),
            false,
            dry_run,
            &dir.join("missing-policy.yaml"),
            "curator",
            false,
        )
    }

    fn snapshot(dir: &Path, state: &Path) -> Vec<(String, Vec<u8>)> {
        let mut names = vec![
            state.join("placement-actual.json"),
            state.join("model-actual.json"),
            state.join(MESH_FILE),
            state.join("conveyor-hops.json"),
            state.join("conveyor-leases.json"),
            state.join("apply-audit.jsonl"),
            state.join("actual-state.json"),
            state.join("desired-snapshot.yaml"),
            state.join("catalog.json"),
        ];
        if let Ok(entries) = fs::read_dir(dir.join("plans")) {
            for entry in entries.flatten() {
                names.push(entry.path());
            }
        }
        names
            .into_iter()
            .filter(|path| path.is_file())
            .map(|path| {
                (
                    path.strip_prefix(dir).unwrap().display().to_string(),
                    fs::read(&path).unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn apply_cites_mismatch_and_stays_quiet_on_a_match() {
        let dir = scratch();
        let estate_path = allow_copy(&dir);
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&estate_path).unwrap();
        let mismatch = mesh_with(
            vec![box_lease("cell-one-box", "notes-append", &["research"])],
            vec![],
        );
        persist_mesh(&state, &mismatch).unwrap();
        let cites = hop_coverage_cites(&estate, &mismatch);
        assert_eq!(cites.len(), 1);
        assert!(cites[0].fail);
        assert!(
            cites[0].line.contains("refuse:hop-coverage")
                && cites[0].line.contains("(mismatch)")
                && cites[0].line.contains(
                    "capability 'notes-append' does not match hop coverage capability 'lane-tool'"
                ),
            "{}",
            cites[0].line
        );
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&estate_path).unwrap();
        let err = apply(&dir, &estate_path, &state, false)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);
        assert!(!state.join("apply-audit.jsonl").exists());

        let matched = mesh_with(
            vec![box_lease("cell-one-box", "lane-tool", &["research"])],
            vec![],
        );
        persist_mesh(&state, &matched).unwrap();
        assert!(hop_coverage_cites(&estate, &matched).is_empty());
        let mesh_bytes = fs::read(state.join(MESH_FILE)).unwrap();
        apply(&dir, &estate_path, &state, false).unwrap();
        assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_dry_run_cites_mismatch_and_writes_nothing() {
        let dir = scratch();
        let estate_path = allow_copy(&dir);
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let mismatch = mesh_with(
            vec![box_lease("cell-one-box", "notes-append", &["research"])],
            vec![],
        );
        persist_mesh(&state, &mismatch).unwrap();
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&estate_path).unwrap();
        let err = apply(&dir, &estate_path, &state, true)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert!(!err.contains("would-refuse"), "{err}");
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);
        assert!(!state.join("apply-audit.jsonl").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_cites_deny_and_deny_default_without_failing_alone() {
        let dir = scratch();
        let estate_path = dir.join("estate.yaml");
        fs::copy(repo_root().join("examples/estate.yaml"), &estate_path).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&estate_path).unwrap();
        let mesh = mesh_with(
            vec![box_lease("cell-one-box", "notes-append", &["research"])],
            vec![],
        );
        persist_mesh(&state, &mesh).unwrap();
        let cites = hop_coverage_cites(&estate, &mesh);
        assert_eq!(cites.len(), 1);
        assert!(!cites[0].fail);
        assert!(
            cites[0].line.contains("refuse:hop-coverage")
                && cites[0].line.contains("(deny-default)")
                && !cites[0].line.contains("(mismatch)"),
            "{}",
            cites[0].line
        );
        let mesh_bytes = fs::read(state.join(MESH_FILE)).unwrap();
        let estate_bytes = fs::read(&estate_path).unwrap();
        apply(&dir, &estate_path, &state, false).unwrap();
        assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);

        let deny_path = dir.join("deny.yaml");
        let mut text = fs::read_to_string(&estate_path).unwrap();
        text = text.replace(
            "      - id: notes-append\n",
            "      - id: notes-append\n      - id: lane-tool\n",
        );
        text = text.replace(
            "intentions: []\n",
            "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: deny\n",
        );
        fs::write(&deny_path, text).unwrap();
        let denied = load_estate(&deny_path).unwrap();
        let deny = hop_coverage_cites(&denied, &mesh);
        assert_eq!(deny.len(), 1);
        assert!(!deny[0].fail);
        assert!(
            deny[0].line.contains("refuse:hop-coverage")
                && deny[0].line.contains("(deny)")
                && !deny[0].line.contains("(mismatch)"),
            "{}",
            deny[0].line
        );
        let deny_bytes = fs::read(&deny_path).unwrap();
        apply(&dir, &deny_path, &state, false).unwrap();
        assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);
        assert_eq!(fs::read(&deny_path).unwrap(), deny_bytes);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_does_not_cite_a_cloud_kind_variant() {
        let dir = scratch();
        let estate_path = allow_copy(&dir);
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&estate_path).unwrap();
        for kind in ["cloud_mesh", " Cloud-Mesh ", "CLOUD-AGENT"] {
            let mut lease = box_lease("cell-one-box", "notes-append", &["research"]);
            lease.kind = kind.into();
            let leased = mesh_with(vec![lease], vec![]);
            assert!(hop_coverage_cites(&estate, &leased).is_empty(), "{kind}");
            persist_mesh(&state, &leased).unwrap();
            let mesh_bytes = fs::read(state.join(MESH_FILE)).unwrap();
            apply(&dir, &estate_path, &state, false).unwrap();
            assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);

            let declared = mesh_with(
                vec![],
                vec![HopDecl {
                    id: "cell-one-box".into(),
                    kind: kind.into(),
                    capability: "notes-append".into(),
                    host_class: "any".into(),
                    wired: false,
                    note: None,
                    ttl_secs: None,
                    agents: vec!["research".into()],
                }],
            );
            assert!(hop_coverage_cites(&estate, &declared).is_empty(), "{kind}");
            persist_mesh(&state, &declared).unwrap();
            let mesh_bytes = fs::read(state.join(MESH_FILE)).unwrap();
            apply(&dir, &estate_path, &state, true).unwrap();
            assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_cites_a_hop_declaration_when_no_lease_is_present() {
        let dir = scratch();
        let estate_path = allow_copy(&dir);
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&estate_path).unwrap();
        let mesh = mesh_with(
            vec![],
            vec![HopDecl {
                id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: vec!["research".into()],
            }],
        );
        persist_mesh(&state, &mesh).unwrap();
        let cites = hop_coverage_cites(&estate, &mesh);
        assert!(
            cites
                .iter()
                .any(|cite| cite.fail && cite.line.contains("(mismatch)")),
            "{cites:?}"
        );
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&estate_path).unwrap();
        let err = apply(&dir, &estate_path, &state, false)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);
        let dry = apply(&dir, &estate_path, &state, true)
            .unwrap_err()
            .to_string();
        assert!(
            dry.contains("refuse:hop-coverage") && dry.contains("(mismatch)"),
            "{dry}"
        );
        assert_eq!(snapshot(&dir, &state), before);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn example_estate_apply_stays_green_without_a_mesh() {
        let dir = scratch();
        let estate_path = dir.join("estate.yaml");
        fs::copy(repo_root().join("examples/estate.yaml"), &estate_path).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        apply(&dir, &estate_path, &state, false).unwrap();
        apply(&dir, &estate_path, &state, true).unwrap();
        let locked = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
        assert_eq!(fs::read(&estate_path).unwrap(), locked);
        assert!(!state.join(MESH_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_fails_closed_when_the_mesh_file_does_not_parse() {
        let dir = scratch();
        let estate_path = dir.join("estate.yaml");
        fs::copy(repo_root().join("examples/estate.yaml"), &estate_path).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let mesh_path = state.join(MESH_FILE);
        let corrupt = b"{not-json";
        fs::write(&mesh_path, corrupt).unwrap();
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&estate_path).unwrap();
        let err = apply(&dir, &estate_path, &state, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("parse") && err.contains(MESH_FILE), "{err}");
        assert!(!err.contains("refuse:hop-coverage"), "{err}");
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&mesh_path).unwrap(), corrupt);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);
        let dry = apply(&dir, &estate_path, &state, true)
            .unwrap_err()
            .to_string();
        assert!(dry.contains("parse") && dry.contains(MESH_FILE), "{dry}");
        assert_eq!(fs::read(&mesh_path).unwrap(), corrupt);
        let _ = fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod apply_agent_call_coverage_tests {
    use super::{cmd_apply, refuse_apply_agent_call_coverage};
    use conveyor_proxy::{persist_mesh, ConveyorMesh, HopLease, MESH_FILE};
    use estate_schema::{agent_call_coverage_cites, load_estate};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn scratch() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-apply-agent-call-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn estate_with(dir: &Path, name: &str, calls: &str, intentions: &str) -> PathBuf {
        let path = dir.join(name);
        let mut text = fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
        let needle = "    mcp: []\n    models:\n      - id: xai_grok\n";
        let insert = format!("    mcp: []\n{calls}    models:\n      - id: xai_grok\n");
        assert!(text.contains(needle), "horizon mcp block missing");
        text = text.replacen(needle, &insert, 1);
        text = text.replace("intentions: []\n", intentions);
        fs::write(&path, text).unwrap();
        path
    }

    fn apply(dir: &Path, estate: &Path, state: &Path, dry_run: bool) -> anyhow::Result<()> {
        cmd_apply(
            estate,
            state,
            &dir.join("roots"),
            &dir.join("plans"),
            false,
            None,
            &dir.join("packs"),
            false,
            dry_run,
            &dir.join("missing-policy.yaml"),
            "curator",
            false,
        )
    }

    fn snapshot(dir: &Path, state: &Path) -> Vec<(String, Vec<u8>)> {
        let names = [
            state.join("placement-actual.json"),
            state.join("model-actual.json"),
            state.join(MESH_FILE),
            state.join("conveyor-hops.json"),
            state.join("conveyor-leases.json"),
            state.join("apply-audit.jsonl"),
            state.join("actual-state.json"),
            state.join("desired-snapshot.yaml"),
            state.join("catalog.json"),
        ];
        names
            .into_iter()
            .filter(|path| path.is_file())
            .map(|path| {
                (
                    path.strip_prefix(dir).unwrap().display().to_string(),
                    fs::read(&path).unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn apply_cites_deny_and_deny_default_without_failing_and_stays_quiet_on_allow() {
        let dir = scratch();
        let calls = "    calls:\n      - id: research\n      - id: horizon\n";
        let deny_path = estate_with(
            &dir,
            "deny.yaml",
            calls,
            "intentions:\n  - subject_agent: horizon\n    object: horizon\n    kind: agent\n    effect: deny\n",
        );
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&deny_path).unwrap();
        let cites = agent_call_coverage_cites(&estate);
        assert_eq!(cites.len(), 2, "{cites:?}");
        assert!(cites.iter().all(|cite| !cite.fail), "{cites:?}");
        assert!(
            cites
                .iter()
                .any(|cite| cite.line == "horizon agent research: deny-default (peer)"),
            "{cites:?}"
        );
        assert!(
            cites
                .iter()
                .any(|cite| cite.line == "horizon agent horizon: deny (own)"),
            "{cites:?}"
        );
        let mesh = ConveyorMesh {
            schema: conveyor_proxy::MESH_SCHEMA.into(),
            hops: vec![],
            leases: vec![HopLease {
                hop_id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                granted: true,
                spawned: true,
                durable: true,
                driver: "box".into(),
                note: None,
                ttl_secs: None,
                issued_at: None,
                expires_at: None,
                agents: vec!["research".into()],
            }],
        };
        persist_mesh(&state, &mesh).unwrap();
        let mesh_bytes = fs::read(state.join(MESH_FILE)).unwrap();
        let estate_bytes = fs::read(&deny_path).unwrap();
        apply(&dir, &deny_path, &state, false).unwrap();
        assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);
        assert_eq!(fs::read(&deny_path).unwrap(), estate_bytes);

        let allow_path = estate_with(
            &dir,
            "allow.yaml",
            "    calls:\n      - id: research\n",
            "intentions:\n  - subject_agent: horizon\n    object: agent:research\n    kind: agent_call\n    effect: allow\n",
        );
        let allowed = load_estate(&allow_path).unwrap();
        assert!(agent_call_coverage_cites(&allowed).is_empty());
        let allow_bytes = fs::read(&allow_path).unwrap();
        apply(&dir, &allow_path, &state, false).unwrap();
        assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);
        assert_eq!(fs::read(&allow_path).unwrap(), allow_bytes);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_dry_run_cites_deny_default_and_writes_nothing() {
        let dir = scratch();
        let path = estate_with(
            &dir,
            "default.yaml",
            "    calls:\n      - id: research\n",
            "intentions: []\n",
        );
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&path).unwrap();
        apply(&dir, &path, &state, true).unwrap();
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        assert!(!state.join("apply-audit.jsonl").exists());
        assert!(!state.join(MESH_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_bails_before_writes_when_the_call_target_is_not_an_agent() {
        let dir = scratch();
        let path = estate_with(
            &dir,
            "ghost.yaml",
            "    calls:\n      - id: ghost\n",
            "intentions: []\n",
        );
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        persist_mesh(
            &state,
            &ConveyorMesh {
                schema: conveyor_proxy::MESH_SCHEMA.into(),
                hops: vec![],
                leases: vec![],
            },
        )
        .unwrap();
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&path).unwrap();
        let err = format!("{:#}", apply(&dir, &path, &state, false).unwrap_err());
        assert!(err.contains("not an estate agent"), "{err}");
        assert!(!err.contains("applied"), "{err}");
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        assert!(!state.join("apply-audit.jsonl").exists());
        let dry = format!("{:#}", apply(&dir, &path, &state, true).unwrap_err());
        assert!(dry.contains("not an estate agent"), "{dry}");
        assert_eq!(snapshot(&dir, &state), before);

        let mut reached = load_estate(&repo_root().join("examples/estate.yaml")).unwrap();
        reached
            .agents
            .iter_mut()
            .find(|agent| agent.id == "horizon")
            .unwrap()
            .calls
            .push(estate_schema::CallDecl {
                id: "ghost".into(),
                description: None,
            });
        let cite = agent_call_coverage_cites(&reached);
        assert!(cite.iter().any(|row| row.fail), "{cite:?}");
        let bail = refuse_apply_agent_call_coverage(&reached)
            .unwrap_err()
            .to_string();
        assert!(
            bail.contains("refuse:agent-call") && bail.contains("not an estate agent"),
            "{bail}"
        );
        assert_eq!(snapshot(&dir, &state), before);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn example_without_calls_has_no_agent_call_cite() {
        let estate = load_estate(&repo_root().join("examples/estate.yaml")).unwrap();
        assert!(agent_call_coverage_cites(&estate).is_empty());
    }

    #[test]
    fn hop_mismatch_still_bails_when_an_agent_call_is_allowed() {
        let dir = scratch();
        let path = estate_with(
            &dir,
            "allow.yaml",
            "    calls:\n      - id: research\n",
            "intentions:\n  - subject_agent: horizon\n    object: research\n    kind: agent-call\n    effect: allow\n",
        );
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&path).unwrap();
        assert!(agent_call_coverage_cites(&estate).is_empty());
        let mut text = fs::read_to_string(&path).unwrap();
        text = text.replace(
            "      - id: notes-append\n",
            "      - id: notes-append\n      - id: lane-tool\n",
        );
        text = text.replace(
            "    effect: allow\n",
            "    effect: allow\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        );
        fs::write(&path, text).unwrap();
        let estate = load_estate(&path).unwrap();
        assert!(agent_call_coverage_cites(&estate).is_empty());
        persist_mesh(
            &state,
            &ConveyorMesh {
                schema: conveyor_proxy::MESH_SCHEMA.into(),
                hops: vec![],
                leases: vec![HopLease {
                    hop_id: "cell-one-box".into(),
                    kind: "box".into(),
                    capability: "notes-append".into(),
                    host_class: "any".into(),
                    granted: true,
                    spawned: true,
                    durable: true,
                    driver: "box".into(),
                    note: None,
                    ttl_secs: None,
                    issued_at: None,
                    expires_at: None,
                    agents: vec!["research".into()],
                }],
            },
        )
        .unwrap();
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&path).unwrap();
        let err = apply(&dir, &path, &state, false).unwrap_err().to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        let dry = apply(&dir, &path, &state, true).unwrap_err().to_string();
        assert!(
            dry.contains("refuse:hop-coverage") && dry.contains("(mismatch)"),
            "{dry}"
        );
        assert_eq!(snapshot(&dir, &state), before);
        let _ = fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod apply_memory_coverage_tests {
    use super::{cmd_apply, refuse_apply_memory_coverage};
    use conveyor_proxy::{persist_mesh, ConveyorMesh, HopLease, MESH_FILE};
    use estate_schema::{load_estate, memory_coverage_cites};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn scratch() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-apply-memory-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_estate(dir: &Path, name: &str, intentions: &str) -> PathBuf {
        let path = dir.join(name);
        let mut text = fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
        text = text.replace("intentions: []\n", intentions);
        fs::write(&path, text).unwrap();
        path
    }

    fn apply(dir: &Path, estate: &Path, state: &Path, dry_run: bool) -> anyhow::Result<()> {
        cmd_apply(
            estate,
            state,
            &dir.join("roots"),
            &dir.join("plans"),
            false,
            None,
            &dir.join("packs"),
            false,
            dry_run,
            &dir.join("missing-policy.yaml"),
            "curator",
            false,
        )
    }

    fn snapshot(dir: &Path, state: &Path) -> Vec<(String, Vec<u8>)> {
        let names = [
            state.join("placement-actual.json"),
            state.join("model-actual.json"),
            state.join(MESH_FILE),
            state.join("conveyor-hops.json"),
            state.join("conveyor-leases.json"),
            state.join("apply-audit.jsonl"),
            state.join("actual-state.json"),
            state.join("desired-snapshot.yaml"),
            state.join("catalog.json"),
        ];
        names
            .into_iter()
            .filter(|path| path.is_file())
            .map(|path| {
                (
                    path.strip_prefix(dir).unwrap().display().to_string(),
                    fs::read(&path).unwrap(),
                )
            })
            .collect()
    }

    fn cross_lane_allow_intentions() -> String {
        let mut text = String::from("intentions:\n");
        for (agent, lane) in [
            ("horizon", "research"),
            ("horizon", "sanctum"),
            ("research", "horizon"),
            ("research", "sanctum"),
            ("sanctum", "horizon"),
            ("sanctum", "research"),
        ] {
            text.push_str(&format!(
                "  - subject_agent: {agent}\n    object: lane:{lane}\n    kind: memory_read\n    effect: allow\n"
            ));
        }
        text
    }

    #[test]
    fn apply_cites_cross_lane_deny_default_and_stays_quiet_on_allow() {
        let dir = scratch();
        let deny_path = write_estate(&dir, "deny.yaml", "intentions: []\n");
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&deny_path).unwrap();
        let cites = memory_coverage_cites(&estate);
        assert!(cites.iter().all(|cite| !cite.fail), "{cites:?}");
        assert!(
            cites.iter().any(|cite| {
                cite.line == "horizon memory_read lane:research: deny-default (cross-lane)"
            }),
            "{cites:?}"
        );
        assert!(
            cites.iter().all(|cite| !cite.line.contains(": allow (")),
            "{cites:?}"
        );
        refuse_apply_memory_coverage(&estate).unwrap();
        let mesh = ConveyorMesh {
            schema: conveyor_proxy::MESH_SCHEMA.into(),
            hops: vec![],
            leases: vec![HopLease {
                hop_id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                granted: true,
                spawned: true,
                durable: true,
                driver: "box".into(),
                note: None,
                ttl_secs: None,
                issued_at: None,
                expires_at: None,
                agents: vec!["research".into()],
            }],
        };
        persist_mesh(&state, &mesh).unwrap();
        let mesh_bytes = fs::read(state.join(MESH_FILE)).unwrap();
        let estate_bytes = fs::read(&deny_path).unwrap();
        apply(&dir, &deny_path, &state, false).unwrap();
        assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);
        assert_eq!(fs::read(&deny_path).unwrap(), estate_bytes);

        let allow_path = write_estate(&dir, "allow.yaml", &cross_lane_allow_intentions());
        let allowed = load_estate(&allow_path).unwrap();
        assert!(memory_coverage_cites(&allowed).is_empty());
        refuse_apply_memory_coverage(&allowed).unwrap();
        let allow_bytes = fs::read(&allow_path).unwrap();
        apply(&dir, &allow_path, &state, false).unwrap();
        assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);
        assert_eq!(fs::read(&allow_path).unwrap(), allow_bytes);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_dry_run_cites_cross_lane_deny_default_and_writes_nothing() {
        let dir = scratch();
        let path = write_estate(&dir, "default.yaml", "intentions: []\n");
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&path).unwrap();
        assert!(memory_coverage_cites(&estate).iter().any(|cite| {
            cite.line == "horizon memory_read lane:research: deny-default (cross-lane)"
        }));
        refuse_apply_memory_coverage(&estate).unwrap();
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&path).unwrap();
        apply(&dir, &path, &state, true).unwrap();
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        assert!(!state.join("apply-audit.jsonl").exists());
        assert!(!state.join(MESH_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hop_mismatch_and_agent_call_refuse_still_bail_when_memory_is_clean() {
        let dir = scratch();
        let mut intentions = cross_lane_allow_intentions();
        intentions.push_str(
            "  - subject_agent: horizon\n    object: agent:research\n    kind: agent\n    effect: allow\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        );
        let path = write_estate(&dir, "clean.yaml", &intentions);
        let mut text = fs::read_to_string(&path).unwrap();
        let needle = "    mcp: []\n    models:\n      - id: xai_grok\n";
        let insert =
            "    mcp: []\n    calls:\n      - id: research\n    models:\n      - id: xai_grok\n";
        assert!(text.contains(needle), "horizon mcp block missing");
        text = text.replacen(needle, insert, 1);
        text = text.replace(
            "      - id: notes-append\n",
            "      - id: notes-append\n      - id: lane-tool\n",
        );
        fs::write(&path, text).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&path).unwrap();
        assert!(
            memory_coverage_cites(&estate).is_empty(),
            "memory allow stays quiet"
        );
        persist_mesh(
            &state,
            &ConveyorMesh {
                schema: conveyor_proxy::MESH_SCHEMA.into(),
                hops: vec![],
                leases: vec![HopLease {
                    hop_id: "cell-one-box".into(),
                    kind: "box".into(),
                    capability: "notes-append".into(),
                    host_class: "any".into(),
                    granted: true,
                    spawned: true,
                    durable: true,
                    driver: "box".into(),
                    note: None,
                    ttl_secs: None,
                    issued_at: None,
                    expires_at: None,
                    agents: vec!["research".into()],
                }],
            },
        )
        .unwrap();
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&path).unwrap();
        let err = apply(&dir, &path, &state, false).unwrap_err().to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert!(!err.contains("refuse:memory"), "{err}");
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        let dry = apply(&dir, &path, &state, true).unwrap_err().to_string();
        assert!(
            dry.contains("refuse:hop-coverage") && dry.contains("(mismatch)"),
            "{dry}"
        );
        assert_eq!(snapshot(&dir, &state), before);

        let ghost_path = write_estate(&dir, "ghost.yaml", &cross_lane_allow_intentions());
        let mut ghost_text = fs::read_to_string(&ghost_path).unwrap();
        let needle = "    mcp: []\n    models:\n      - id: xai_grok\n";
        let insert =
            "    mcp: []\n    calls:\n      - id: ghost\n    models:\n      - id: xai_grok\n";
        assert!(ghost_text.contains(needle), "horizon mcp block missing");
        ghost_text = ghost_text.replacen(needle, insert, 1);
        fs::write(&ghost_path, ghost_text).unwrap();
        let err = format!("{:#}", apply(&dir, &ghost_path, &state, false).unwrap_err());
        assert!(err.contains("not an estate agent"), "{err}");
        assert!(!err.contains("refuse:memory"), "{err}");
        assert!(!err.contains("applied"), "{err}");
        assert_eq!(snapshot(&dir, &state), before);
        assert!(!state.join("apply-audit.jsonl").exists());
        let mut reached = load_estate(&repo_root().join("examples/estate.yaml")).unwrap();
        for (agent, lane) in [
            ("horizon", "research"),
            ("horizon", "sanctum"),
            ("research", "horizon"),
            ("research", "sanctum"),
            ("sanctum", "horizon"),
            ("sanctum", "research"),
        ] {
            reached.intentions.push(estate_schema::Intention {
                subject_agent: agent.into(),
                object: format!("lane:{lane}"),
                kind: estate_schema::IntentionKind::MemoryRead,
                effect: estate_schema::Effect::Allow,
                note: None,
            });
        }
        reached
            .agents
            .iter_mut()
            .find(|agent| agent.id == "horizon")
            .unwrap()
            .calls
            .push(estate_schema::CallDecl {
                id: "ghost".into(),
                description: None,
            });
        assert!(memory_coverage_cites(&reached).is_empty());
        let bail = super::refuse_apply_agent_call_coverage(&reached)
            .unwrap_err()
            .to_string();
        assert!(
            bail.contains("refuse:agent-call") && bail.contains("not an estate agent"),
            "{bail}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod apply_declared_coverage_tests {
    use super::{
        cmd_apply, refuse_apply_agent_call_coverage, refuse_apply_declared_coverage,
        refuse_apply_memory_coverage,
    };
    use conveyor_proxy::{persist_mesh, ConveyorMesh, HopLease, MESH_FILE};
    use estate_schema::{declared_coverage_cites, load_estate, memory_coverage_cites};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn scratch() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-apply-declared-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_estate(dir: &Path, name: &str, intentions: &str) -> PathBuf {
        let path = dir.join(name);
        let mut text = fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
        let needle = "    mcp: []\n    models:\n      - id: local_slm\n";
        let insert = "    mcp:\n      - id: docs\n    models:\n      - id: local_slm\n";
        assert!(text.contains(needle), "research mcp block missing");
        text = text.replacen(needle, insert, 1);
        text = text.replace("intentions: []\n", intentions);
        fs::write(&path, text).unwrap();
        path
    }

    fn apply(dir: &Path, estate: &Path, state: &Path, dry_run: bool) -> anyhow::Result<()> {
        cmd_apply(
            estate,
            state,
            &dir.join("roots"),
            &dir.join("plans"),
            false,
            None,
            &dir.join("packs"),
            false,
            dry_run,
            &dir.join("missing-policy.yaml"),
            "curator",
            false,
        )
    }

    fn snapshot(dir: &Path, state: &Path) -> Vec<(String, Vec<u8>)> {
        let names = [
            state.join("placement-actual.json"),
            state.join("model-actual.json"),
            state.join(MESH_FILE),
            state.join("conveyor-hops.json"),
            state.join("conveyor-leases.json"),
            state.join("apply-audit.jsonl"),
            state.join("actual-state.json"),
            state.join("desired-snapshot.yaml"),
            state.join("catalog.json"),
        ];
        names
            .into_iter()
            .filter(|path| path.is_file())
            .map(|path| {
                (
                    path.strip_prefix(dir).unwrap().display().to_string(),
                    fs::read(&path).unwrap(),
                )
            })
            .collect()
    }

    fn box_mesh() -> ConveyorMesh {
        ConveyorMesh {
            schema: conveyor_proxy::MESH_SCHEMA.into(),
            hops: vec![],
            leases: vec![HopLease {
                hop_id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                granted: true,
                spawned: true,
                durable: true,
                driver: "box".into(),
                note: None,
                ttl_secs: None,
                issued_at: None,
                expires_at: None,
                agents: vec!["research".into()],
            }],
        }
    }

    fn allow_declared() -> &'static str {
        "intentions:\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes\n    kind: mount\n    effect: allow\n  - subject_agent: research\n    object: mcp:docs\n    kind: mcp\n    effect: allow\n"
    }

    fn cross_lane_allow_intentions() -> String {
        let mut text = String::from("intentions:\n");
        for (agent, lane) in [
            ("horizon", "research"),
            ("horizon", "sanctum"),
            ("research", "horizon"),
            ("research", "sanctum"),
            ("sanctum", "horizon"),
            ("sanctum", "research"),
        ] {
            text.push_str(&format!(
                "  - subject_agent: {agent}\n    object: lane:{lane}\n    kind: memory_read\n    effect: allow\n"
            ));
        }
        text
    }

    #[test]
    fn apply_cites_deny_and_deny_default_without_failing_and_stays_quiet_on_allow() {
        let dir = scratch();
        let deny_path = write_estate(
            &dir,
            "deny.yaml",
            "intentions:\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: deny\n",
        );
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&deny_path).unwrap();
        let cites = declared_coverage_cites(&estate);
        assert!(cites.iter().all(|cite| !cite.fail), "{cites:?}");
        assert!(
            cites
                .iter()
                .any(|cite| cite.line == "research tool notes-append: deny"),
            "{cites:?}"
        );
        assert!(
            cites
                .iter()
                .any(|cite| cite.line == "research mount notes: deny-default"),
            "{cites:?}"
        );
        assert!(
            cites
                .iter()
                .any(|cite| cite.line == "research mcp docs: deny-default"),
            "{cites:?}"
        );
        assert!(
            cites.iter().all(|cite| !cite.line.ends_with(": allow")),
            "{cites:?}"
        );
        refuse_apply_declared_coverage(&estate).unwrap();
        persist_mesh(&state, &box_mesh()).unwrap();
        let mesh_bytes = fs::read(state.join(MESH_FILE)).unwrap();
        let estate_bytes = fs::read(&deny_path).unwrap();
        apply(&dir, &deny_path, &state, false).unwrap();
        assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);
        assert_eq!(fs::read(&deny_path).unwrap(), estate_bytes);

        let allow_path = write_estate(&dir, "allow.yaml", allow_declared());
        let allowed = load_estate(&allow_path).unwrap();
        assert!(declared_coverage_cites(&allowed).is_empty());
        refuse_apply_declared_coverage(&allowed).unwrap();
        let allow_bytes = fs::read(&allow_path).unwrap();
        apply(&dir, &allow_path, &state, false).unwrap();
        assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);
        assert_eq!(fs::read(&allow_path).unwrap(), allow_bytes);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_dry_run_cites_deny_default_and_writes_nothing() {
        let dir = scratch();
        let path = write_estate(&dir, "default.yaml", "intentions: []\n");
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&path).unwrap();
        assert!(declared_coverage_cites(&estate).iter().any(|cite| {
            !cite.fail && cite.line == "research tool notes-append: deny-default"
        }));
        assert!(declared_coverage_cites(&estate).iter().any(|cite| {
            !cite.fail && cite.line == "research mount notes: deny-default"
        }));
        assert!(declared_coverage_cites(&estate).iter().any(|cite| {
            !cite.fail && cite.line == "research mcp docs: deny-default"
        }));
        refuse_apply_declared_coverage(&estate).unwrap();
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&path).unwrap();
        apply(&dir, &path, &state, true).unwrap();
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        assert!(!state.join("apply-audit.jsonl").exists());
        assert!(!state.join(MESH_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hop_agent_call_and_memory_still_bail_when_declared_coverage_is_clean() {
        let dir = scratch();
        let mut intentions = cross_lane_allow_intentions();
        intentions.push_str(
            "  - subject_agent: horizon\n    object: agent:research\n    kind: agent\n    effect: allow\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes\n    kind: mount\n    effect: allow\n  - subject_agent: research\n    object: docs\n    kind: mcp\n    effect: allow\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        );
        let path = write_estate(&dir, "clean.yaml", &intentions);
        let mut text = fs::read_to_string(&path).unwrap();
        let needle = "    mcp: []\n    models:\n      - id: xai_grok\n";
        let insert =
            "    mcp: []\n    calls:\n      - id: research\n    models:\n      - id: xai_grok\n";
        assert!(text.contains(needle), "horizon mcp block missing");
        text = text.replacen(needle, insert, 1);
        text = text.replace(
            "      - id: notes-append\n",
            "      - id: notes-append\n      - id: lane-tool\n",
        );
        fs::write(&path, text).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&path).unwrap();
        assert!(
            declared_coverage_cites(&estate).is_empty(),
            "tool mcp mount allow stays quiet"
        );
        assert!(
            memory_coverage_cites(&estate).is_empty(),
            "memory allow stays quiet"
        );
        refuse_apply_declared_coverage(&estate).unwrap();
        refuse_apply_memory_coverage(&estate).unwrap();
        persist_mesh(&state, &box_mesh()).unwrap();
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&path).unwrap();
        let err = apply(&dir, &path, &state, false).unwrap_err().to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert!(!err.contains("refuse:tool"), "{err}");
        assert!(!err.contains("refuse:mcp"), "{err}");
        assert!(!err.contains("refuse:mount"), "{err}");
        assert!(!err.contains("refuse:memory"), "{err}");
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        let dry = apply(&dir, &path, &state, true).unwrap_err().to_string();
        assert!(
            dry.contains("refuse:hop-coverage") && dry.contains("(mismatch)"),
            "{dry}"
        );
        assert_eq!(snapshot(&dir, &state), before);

        let mut ghost_intentions = cross_lane_allow_intentions();
        ghost_intentions.push_str(
            "  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes\n    kind: mount\n    effect: allow\n  - subject_agent: research\n    object: docs\n    kind: mcp\n    effect: allow\n",
        );
        let ghost_path = write_estate(&dir, "ghost.yaml", &ghost_intentions);
        let mut ghost_text = fs::read_to_string(&ghost_path).unwrap();
        let needle = "    mcp: []\n    models:\n      - id: xai_grok\n";
        let insert =
            "    mcp: []\n    calls:\n      - id: ghost\n    models:\n      - id: xai_grok\n";
        assert!(ghost_text.contains(needle), "horizon mcp block missing");
        ghost_text = ghost_text.replacen(needle, insert, 1);
        fs::write(&ghost_path, ghost_text).unwrap();
        let err = format!("{:#}", apply(&dir, &ghost_path, &state, false).unwrap_err());
        assert!(err.contains("not an estate agent"), "{err}");
        assert!(!err.contains("refuse:tool"), "{err}");
        assert!(!err.contains("refuse:memory"), "{err}");
        assert!(!err.contains("applied"), "{err}");
        assert_eq!(snapshot(&dir, &state), before);
        assert!(!state.join("apply-audit.jsonl").exists());

        let mut reached = load_estate(&repo_root().join("examples/estate.yaml")).unwrap();
        for (agent, lane) in [
            ("horizon", "research"),
            ("horizon", "sanctum"),
            ("research", "horizon"),
            ("research", "sanctum"),
            ("sanctum", "horizon"),
            ("sanctum", "research"),
        ] {
            reached.intentions.push(estate_schema::Intention {
                subject_agent: agent.into(),
                object: format!("lane:{lane}"),
                kind: estate_schema::IntentionKind::MemoryRead,
                effect: estate_schema::Effect::Allow,
                note: None,
            });
        }
        for (kind, object) in [
            (estate_schema::IntentionKind::Tool, "notes-append"),
            (estate_schema::IntentionKind::Mount, "notes"),
        ] {
            reached.intentions.push(estate_schema::Intention {
                subject_agent: "research".into(),
                object: object.into(),
                kind,
                effect: estate_schema::Effect::Allow,
                note: None,
            });
        }
        reached
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap()
            .mcp
            .push(estate_schema::McpDecl {
                id: "docs".into(),
                description: None,
            });
        reached.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "docs".into(),
            kind: estate_schema::IntentionKind::Mcp,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
        reached
            .agents
            .iter_mut()
            .find(|agent| agent.id == "horizon")
            .unwrap()
            .calls
            .push(estate_schema::CallDecl {
                id: "ghost".into(),
                description: None,
            });
        assert!(declared_coverage_cites(&reached).is_empty());
        assert!(memory_coverage_cites(&reached).is_empty());
        refuse_apply_declared_coverage(&reached).unwrap();
        refuse_apply_memory_coverage(&reached).unwrap();
        let bail = refuse_apply_agent_call_coverage(&reached)
            .unwrap_err()
            .to_string();
        assert!(
            bail.contains("refuse:agent-call") && bail.contains("not an estate agent"),
            "{bail}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn examples_estate_yaml_stays_hash_locked() {
        let path = repo_root().join("examples/estate.yaml");
        let sum = Command::new("cksum").arg(&path).output().unwrap();
        let text = String::from_utf8_lossy(&sum.stdout);
        assert!(
            text.starts_with("43770130 3391"),
            "examples/estate.yaml cksum changed: {text}"
        );
    }
}

#[cfg(test)]
mod apply_model_class_coverage_tests {
    use super::{
        apply_cites_before_writes, cmd_apply, refuse_apply_agent_call_coverage,
        refuse_apply_declared_coverage, refuse_apply_memory_coverage,
        refuse_apply_model_class_coverage,
    };
    use conveyor_proxy::{persist_mesh, ConveyorMesh, HopLease, MESH_FILE};
    use estate_schema::{
        declared_coverage_cites, load_estate, memory_coverage_cites, model_class_coverage_cites,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn scratch() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-apply-model-class-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_estate(dir: &Path, name: &str, intentions: &str) -> PathBuf {
        let path = dir.join(name);
        let mut text = fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
        text = text.replace("intentions: []\n", intentions);
        fs::write(&path, text).unwrap();
        path
    }

    fn apply(dir: &Path, estate: &Path, state: &Path, dry_run: bool) -> anyhow::Result<()> {
        cmd_apply(
            estate,
            state,
            &dir.join("roots"),
            &dir.join("plans"),
            false,
            None,
            &dir.join("packs"),
            false,
            dry_run,
            &dir.join("missing-policy.yaml"),
            "curator",
            false,
        )
    }

    fn snapshot(dir: &Path, state: &Path) -> Vec<(String, Vec<u8>)> {
        let names = [
            state.join("placement-actual.json"),
            state.join("model-actual.json"),
            state.join(MESH_FILE),
            state.join("conveyor-hops.json"),
            state.join("conveyor-leases.json"),
            state.join("apply-audit.jsonl"),
            state.join("actual-state.json"),
            state.join("desired-snapshot.yaml"),
            state.join("catalog.json"),
        ];
        names
            .into_iter()
            .filter(|path| path.is_file())
            .map(|path| {
                (
                    path.strip_prefix(dir).unwrap().display().to_string(),
                    fs::read(&path).unwrap(),
                )
            })
            .collect()
    }

    fn box_mesh() -> ConveyorMesh {
        ConveyorMesh {
            schema: conveyor_proxy::MESH_SCHEMA.into(),
            hops: vec![],
            leases: vec![HopLease {
                hop_id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                granted: true,
                spawned: true,
                durable: true,
                driver: "box".into(),
                note: None,
                ttl_secs: None,
                issued_at: None,
                expires_at: None,
                agents: vec!["research".into()],
            }],
        }
    }

    fn model_allow_lines() -> &'static str {
        "  - subject_agent: horizon\n    object: class:frontier\n    kind: model\n    effect: allow\n  - subject_agent: horizon\n    object: class:local\n    kind: model\n    effect: allow\n  - subject_agent: research\n    object: class:local\n    kind: model\n    effect: allow\n"
    }

    fn cross_lane_allow_intentions() -> String {
        let mut text = String::from("intentions:\n");
        for (agent, lane) in [
            ("horizon", "research"),
            ("horizon", "sanctum"),
            ("research", "horizon"),
            ("research", "sanctum"),
            ("sanctum", "horizon"),
            ("sanctum", "research"),
        ] {
            text.push_str(&format!(
                "  - subject_agent: {agent}\n    object: lane:{lane}\n    kind: memory_read\n    effect: allow\n"
            ));
        }
        text
    }

    #[test]
    fn apply_cites_deny_and_deny_default_without_failing_and_stays_quiet_on_allow() {
        let dir = scratch();
        let deny_path = write_estate(
            &dir,
            "deny.yaml",
            "intentions:\n  - subject_agent: horizon\n    object: xai_grok\n    kind: model\n    effect: deny\n",
        );
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&deny_path).unwrap();
        let cites = model_class_coverage_cites(&estate);
        assert!(cites.iter().all(|cite| !cite.fail), "{cites:?}");
        assert!(
            cites
                .iter()
                .any(|cite| cite.line == "horizon frontier xai_grok: deny"),
            "{cites:?}"
        );
        assert!(
            cites
                .iter()
                .any(|cite| cite.line == "horizon local local_slm: deny-default"),
            "{cites:?}"
        );
        assert!(
            cites
                .iter()
                .any(|cite| cite.line == "research local local_slm: deny-default"),
            "{cites:?}"
        );
        assert!(
            cites.iter().all(|cite| !cite.line.ends_with(": allow")),
            "{cites:?}"
        );
        refuse_apply_model_class_coverage(&estate).unwrap();
        persist_mesh(&state, &box_mesh()).unwrap();
        let mesh_bytes = fs::read(state.join(MESH_FILE)).unwrap();
        let estate_bytes = fs::read(&deny_path).unwrap();
        apply(&dir, &deny_path, &state, false).unwrap();
        assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);
        assert_eq!(fs::read(&deny_path).unwrap(), estate_bytes);

        let mut allow = String::from("intentions:\n");
        allow.push_str(model_allow_lines());
        let allow_path = write_estate(&dir, "allow.yaml", &allow);
        let allowed = load_estate(&allow_path).unwrap();
        assert!(model_class_coverage_cites(&allowed).is_empty());
        refuse_apply_model_class_coverage(&allowed).unwrap();
        let allow_bytes = fs::read(&allow_path).unwrap();
        apply(&dir, &allow_path, &state, false).unwrap();
        assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);
        assert_eq!(fs::read(&allow_path).unwrap(), allow_bytes);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_dry_run_cites_deny_default_and_writes_nothing() {
        let dir = scratch();
        let path = write_estate(&dir, "default.yaml", "intentions: []\n");
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&path).unwrap();
        assert!(model_class_coverage_cites(&estate)
            .iter()
            .any(|cite| { !cite.fail && cite.line == "horizon frontier xai_grok: deny-default" }));
        assert!(model_class_coverage_cites(&estate)
            .iter()
            .any(|cite| { !cite.fail && cite.line == "horizon local local_slm: deny-default" }));
        assert!(model_class_coverage_cites(&estate)
            .iter()
            .any(|cite| { !cite.fail && cite.line == "research local local_slm: deny-default" }));
        refuse_apply_model_class_coverage(&estate).unwrap();
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&path).unwrap();
        apply(&dir, &path, &state, true).unwrap();
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        assert!(!state.join("apply-audit.jsonl").exists());
        assert!(!state.join(MESH_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuse_model_class_bails_before_writes() {
        let dir = scratch();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let mut estate = load_estate(&repo_root().join("examples/estate.yaml")).unwrap();
        estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap()
            .tools
            .push(estate_schema::ToolDecl {
                id: "lane-tool".into(),
                description: None,
            });
        estate.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "lane-tool".into(),
            kind: estate_schema::IntentionKind::Tool,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
        estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == "horizon")
            .unwrap()
            .models
            .push(estate_schema::ModelUseDecl {
                id: "ghost_slm".into(),
                description: None,
            });
        persist_mesh(&state, &box_mesh()).unwrap();
        let before = snapshot(&dir, &state);
        let mesh_bytes = fs::read(state.join(MESH_FILE)).unwrap();
        let err = apply_cites_before_writes(&estate, &state)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("refuse:model-class") && err.contains("ghost_slm"),
            "{err}"
        );
        assert!(err.contains("undeclared"), "{err}");
        assert!(!err.contains("refuse:hop-coverage"), "{err}");
        assert!(!err.contains("(mismatch)"), "{err}");
        assert!(!err.contains("refuse:tool"), "{err}");
        assert!(!err.contains("refuse:memory"), "{err}");
        assert!(!err.contains("refuse:agent-call"), "{err}");
        let helper = refuse_apply_model_class_coverage(&estate)
            .unwrap_err()
            .to_string();
        assert!(helper.contains("refuse:model-class"), "{helper}");
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(state.join(MESH_FILE)).unwrap(), mesh_bytes);

        let path = write_estate(&dir, "ghost-model.yaml", "intentions: []\n");
        let mut text = fs::read_to_string(&path).unwrap();
        let needle =
            "        description: Frontier complete under the estate (A7)\n      - id: local_slm\n";
        let insert = "        description: Frontier complete under the estate (A7)\n      - id: ghost_slm\n      - id: local_slm\n";
        assert!(text.contains(needle), "horizon model block missing");
        text = text.replacen(needle, insert, 1);
        fs::write(&path, &text).unwrap();
        let estate_bytes = fs::read(&path).unwrap();
        let load_err = load_estate(&path).unwrap_err().to_string();
        assert!(load_err.contains("not a model_binding"), "{load_err}");
        let err = format!("{:#}", apply(&dir, &path, &state, false).unwrap_err());
        assert!(err.contains("not a model_binding"), "{err}");
        assert!(!err.contains("applied"), "{err}");
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        let dry = format!("{:#}", apply(&dir, &path, &state, true).unwrap_err());
        assert!(dry.contains("not a model_binding"), "{dry}");
        assert_eq!(snapshot(&dir, &state), before);
        assert!(!state.join("apply-audit.jsonl").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hop_agent_call_memory_and_declared_still_bail_when_model_rows_are_allow() {
        let dir = scratch();
        let mut intentions = cross_lane_allow_intentions();
        intentions.push_str(model_allow_lines());
        intentions.push_str(
            "  - subject_agent: horizon\n    object: agent:research\n    kind: agent\n    effect: allow\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes\n    kind: mount\n    effect: allow\n  - subject_agent: research\n    object: docs\n    kind: mcp\n    effect: allow\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        );
        let path = write_estate(&dir, "clean.yaml", &intentions);
        let mut text = fs::read_to_string(&path).unwrap();
        let needle = "    mcp: []\n    models:\n      - id: xai_grok\n";
        let insert =
            "    mcp: []\n    calls:\n      - id: research\n    models:\n      - id: xai_grok\n";
        assert!(text.contains(needle), "horizon mcp block missing");
        text = text.replacen(needle, insert, 1);
        let research_mcp = "    mcp: []\n    models:\n      - id: local_slm\n";
        let research_docs = "    mcp:\n      - id: docs\n    models:\n      - id: local_slm\n";
        assert!(text.contains(research_mcp), "research mcp block missing");
        text = text.replacen(research_mcp, research_docs, 1);
        text = text.replace(
            "      - id: notes-append\n",
            "      - id: notes-append\n      - id: lane-tool\n",
        );
        fs::write(&path, text).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&path).unwrap();
        assert!(
            model_class_coverage_cites(&estate).is_empty(),
            "model allow stays quiet"
        );
        assert!(
            memory_coverage_cites(&estate).is_empty(),
            "memory allow stays quiet"
        );
        assert!(
            declared_coverage_cites(&estate).is_empty(),
            "tool mcp mount allow stays quiet"
        );
        refuse_apply_model_class_coverage(&estate).unwrap();
        refuse_apply_memory_coverage(&estate).unwrap();
        refuse_apply_declared_coverage(&estate).unwrap();
        persist_mesh(&state, &box_mesh()).unwrap();
        let before = snapshot(&dir, &state);
        let estate_bytes = fs::read(&path).unwrap();
        let err = apply(&dir, &path, &state, false).unwrap_err().to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert!(!err.contains("refuse:model-class"), "{err}");
        assert!(!err.contains("refuse:tool"), "{err}");
        assert!(!err.contains("refuse:mcp"), "{err}");
        assert!(!err.contains("refuse:mount"), "{err}");
        assert!(!err.contains("refuse:memory"), "{err}");
        assert!(!err.contains("refuse:agent-call"), "{err}");
        assert_eq!(snapshot(&dir, &state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        let dry = apply(&dir, &path, &state, true).unwrap_err().to_string();
        assert!(
            dry.contains("refuse:hop-coverage") && dry.contains("(mismatch)"),
            "{dry}"
        );
        assert!(!dry.contains("refuse:model-class"), "{dry}");
        assert_eq!(snapshot(&dir, &state), before);

        let mut ghost_intentions = cross_lane_allow_intentions();
        ghost_intentions.push_str(model_allow_lines());
        ghost_intentions.push_str(
            "  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes\n    kind: mount\n    effect: allow\n  - subject_agent: research\n    object: docs\n    kind: mcp\n    effect: allow\n",
        );
        let ghost_path = write_estate(&dir, "ghost.yaml", &ghost_intentions);
        let mut ghost_text = fs::read_to_string(&ghost_path).unwrap();
        let needle = "    mcp: []\n    models:\n      - id: xai_grok\n";
        let insert =
            "    mcp: []\n    calls:\n      - id: ghost\n    models:\n      - id: xai_grok\n";
        assert!(ghost_text.contains(needle), "horizon mcp block missing");
        ghost_text = ghost_text.replacen(needle, insert, 1);
        let research_mcp = "    mcp: []\n    models:\n      - id: local_slm\n";
        let research_docs = "    mcp:\n      - id: docs\n    models:\n      - id: local_slm\n";
        assert!(
            ghost_text.contains(research_mcp),
            "research mcp block missing"
        );
        ghost_text = ghost_text.replacen(research_mcp, research_docs, 1);
        fs::write(&ghost_path, ghost_text).unwrap();
        let err = format!("{:#}", apply(&dir, &ghost_path, &state, false).unwrap_err());
        assert!(err.contains("not an estate agent"), "{err}");
        assert!(!err.contains("refuse:model-class"), "{err}");
        assert!(!err.contains("refuse:memory"), "{err}");
        assert!(!err.contains("refuse:tool"), "{err}");
        assert!(!err.contains("applied"), "{err}");
        assert_eq!(snapshot(&dir, &state), before);
        assert!(!state.join("apply-audit.jsonl").exists());

        let mut reached = load_estate(&path).unwrap();
        reached
            .agents
            .iter_mut()
            .find(|agent| agent.id == "horizon")
            .unwrap()
            .calls
            .push(estate_schema::CallDecl {
                id: "ghost".into(),
                description: None,
            });
        assert!(model_class_coverage_cites(&reached).is_empty());
        assert!(memory_coverage_cites(&reached).is_empty());
        assert!(declared_coverage_cites(&reached).is_empty());
        refuse_apply_model_class_coverage(&reached).unwrap();
        refuse_apply_memory_coverage(&reached).unwrap();
        refuse_apply_declared_coverage(&reached).unwrap();
        let bail = apply_cites_before_writes(&reached, &state)
            .unwrap_err()
            .to_string();
        assert!(
            bail.contains("refuse:agent-call") && bail.contains("not an estate agent"),
            "{bail}"
        );
        assert!(!bail.contains("refuse:model-class"), "{bail}");
        assert_eq!(snapshot(&dir, &state), before);
        let direct = refuse_apply_agent_call_coverage(&reached)
            .unwrap_err()
            .to_string();
        assert!(direct.contains("refuse:agent-call"), "{direct}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn examples_estate_yaml_stays_hash_locked() {
        let path = repo_root().join("examples/estate.yaml");
        let sum = Command::new("cksum").arg(&path).output().unwrap();
        let text = String::from_utf8_lossy(&sum.stdout);
        assert!(
            text.starts_with("43770130 3391"),
            "examples/estate.yaml cksum changed: {text}"
        );
    }
}

#[cfg(test)]
mod plan_model_class_coverage_tests {
    use super::{cmd_plan, plan_cites_before_hop, refuse_plan_model_class_coverage};
    use conveyor_proxy::{persist_mesh, ConveyorMesh, HopLease, MESH_FILE};
    use estate_schema::{describe_model_class_coverage, load_estate, model_class_coverage_cites};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn scratch() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-plan-model-class-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_estate(dir: &Path, name: &str, intentions: &str) -> PathBuf {
        let path = dir.join(name);
        let mut text = fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
        text = text.replace("intentions: []\n", intentions);
        fs::write(&path, text).unwrap();
        path
    }

    fn plan(dir: &Path, estate: &Path, state: &Path, reviewed: bool) -> anyhow::Result<()> {
        cmd_plan(
            estate,
            None,
            &dir.join("plans"),
            state,
            reviewed,
            &dir.join("reviewed"),
        )
    }

    fn snapshot(state: &Path) -> Vec<(String, Vec<u8>)> {
        [
            "placement-actual.json",
            "model-actual.json",
            MESH_FILE,
            "conveyor-hops.json",
            "conveyor-leases.json",
        ]
        .into_iter()
        .filter_map(|name| {
            let path = state.join(name);
            path.is_file()
                .then(|| (name.to_string(), fs::read(&path).unwrap()))
        })
        .collect()
    }

    fn box_mesh() -> ConveyorMesh {
        ConveyorMesh {
            schema: conveyor_proxy::MESH_SCHEMA.into(),
            hops: vec![],
            leases: vec![HopLease {
                hop_id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                granted: true,
                spawned: true,
                durable: true,
                driver: "box".into(),
                note: None,
                ttl_secs: None,
                issued_at: None,
                expires_at: None,
                agents: vec!["research".into()],
            }],
        }
    }

    fn model_allow_lines() -> &'static str {
        "  - subject_agent: horizon\n    object: class:frontier\n    kind: model\n    effect: allow\n  - subject_agent: horizon\n    object: class:local\n    kind: model\n    effect: allow\n  - subject_agent: research\n    object: class:local\n    kind: model\n    effect: allow\n"
    }

    fn reviewed_sentinel(dir: &Path) -> PathBuf {
        let reviewed = dir.join("reviewed");
        fs::create_dir_all(&reviewed).unwrap();
        fs::write(reviewed.join("keep.txt"), b"sentinel").unwrap();
        reviewed
    }

    fn assert_reviewed_untouched(reviewed: &Path) {
        assert_eq!(fs::read(reviewed.join("keep.txt")).unwrap(), b"sentinel");
        assert!(!reviewed.join("INDEX.md").exists());
        let extras: Vec<_> = fs::read_dir(reviewed)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name != "keep.txt")
            .collect();
        assert!(extras.is_empty(), "{extras:?}");
    }

    #[test]
    fn plan_cites_deny_and_deny_default_without_failing_and_stays_quiet_on_allow() {
        let dir = scratch();
        let deny_path = write_estate(
            &dir,
            "deny.yaml",
            "intentions:\n  - subject_agent: horizon\n    object: xai_grok\n    kind: model\n    effect: deny\n",
        );
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&deny_path).unwrap();
        let described = describe_model_class_coverage(&estate);
        let cites = model_class_coverage_cites(&estate);
        assert!(cites.iter().all(|cite| !cite.fail), "{cites:?}");
        for line in [
            "horizon frontier xai_grok: deny",
            "horizon local local_slm: deny-default",
            "research local local_slm: deny-default",
        ] {
            assert!(cites.iter().any(|cite| cite.line == line), "{cites:?}");
            assert!(described.contains(line), "{described}");
            assert!(!line.contains("(mismatch)"), "{line}");
        }
        assert!(
            cites.iter().all(|cite| !cite.line.ends_with(": allow")),
            "{cites:?}"
        );
        refuse_plan_model_class_coverage(&estate).unwrap();
        persist_mesh(&state, &box_mesh()).unwrap();
        let before = snapshot(&state);
        let estate_bytes = fs::read(&deny_path).unwrap();
        let reviewed = reviewed_sentinel(&dir);
        plan(&dir, &deny_path, &state, false).unwrap();
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&deny_path).unwrap(), estate_bytes);
        assert_reviewed_untouched(&reviewed);

        let mut allow = String::from("intentions:\n");
        allow.push_str(model_allow_lines());
        let allow_path = write_estate(&dir, "allow.yaml", &allow);
        let allowed = load_estate(&allow_path).unwrap();
        assert!(model_class_coverage_cites(&allowed).is_empty());
        assert!(
            describe_model_class_coverage(&allowed).contains("horizon frontier xai_grok: allow")
        );
        assert!(describe_model_class_coverage(&allowed).contains("research local local_slm: allow"));
        refuse_plan_model_class_coverage(&allowed).unwrap();
        let allow_before = snapshot(&state);
        let allow_bytes = fs::read(&allow_path).unwrap();
        plan(&dir, &allow_path, &state, false).unwrap();
        assert_eq!(snapshot(&state), allow_before);
        assert_eq!(fs::read(&allow_path).unwrap(), allow_bytes);
        assert_reviewed_untouched(&reviewed);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuse_model_class_bails_before_hop_and_leaves_dirs_untouched() {
        let dir = scratch();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let mut estate = load_estate(&repo_root().join("examples/estate.yaml")).unwrap();
        estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == "horizon")
            .unwrap()
            .models
            .push(estate_schema::ModelUseDecl {
                id: "ghost_slm".into(),
                description: None,
            });
        persist_mesh(&state, &box_mesh()).unwrap();
        let before = snapshot(&state);
        let reviewed = reviewed_sentinel(&dir);
        let err = plan_cites_before_hop(&estate, &state)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("refuse:model-class") && err.contains("ghost_slm"),
            "{err}"
        );
        assert!(err.contains("undeclared"), "{err}");
        assert!(!err.contains("refuse:hop-coverage"), "{err}");
        assert!(!err.contains("(mismatch)"), "{err}");
        assert!(!err.contains("refuse:tool"), "{err}");
        assert!(!err.contains("refuse:mcp"), "{err}");
        assert!(!err.contains("refuse:mount"), "{err}");
        assert!(!err.contains("refuse:memory"), "{err}");
        assert!(!err.contains("refuse:agent-call"), "{err}");
        let helper = refuse_plan_model_class_coverage(&estate)
            .unwrap_err()
            .to_string();
        assert!(helper.contains("refuse:model-class"), "{helper}");
        assert_eq!(snapshot(&state), before);
        assert_reviewed_untouched(&reviewed);

        let path = write_estate(&dir, "ghost-model.yaml", "intentions: []\n");
        let mut text = fs::read_to_string(&path).unwrap();
        let needle =
            "        description: Frontier complete under the estate (A7)\n      - id: local_slm\n";
        let insert = "        description: Frontier complete under the estate (A7)\n      - id: ghost_slm\n      - id: local_slm\n";
        assert!(text.contains(needle), "horizon model block missing");
        text = text.replacen(needle, insert, 1);
        fs::write(&path, &text).unwrap();
        let estate_bytes = fs::read(&path).unwrap();
        let load_err = load_estate(&path).unwrap_err().to_string();
        assert!(load_err.contains("not a model_binding"), "{load_err}");
        let err = format!("{:#}", plan(&dir, &path, &state, true).unwrap_err());
        assert!(err.contains("not a model_binding"), "{err}");
        assert!(!err.contains("refuse:hop-coverage"), "{err}");
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        assert_reviewed_untouched(&reviewed);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hop_mismatch_still_bails_when_model_rows_are_allow() {
        let dir = scratch();
        let mut intentions = String::from(
            "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        );
        intentions.push_str(model_allow_lines());
        let path = write_estate(&dir, "clean.yaml", &intentions);
        let mut text = fs::read_to_string(&path).unwrap();
        text = text.replace(
            "      - id: notes-append\n",
            "      - id: notes-append\n      - id: lane-tool\n",
        );
        fs::write(&path, text).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&path).unwrap();
        assert!(
            model_class_coverage_cites(&estate).is_empty(),
            "model allow stays quiet"
        );
        assert!(describe_model_class_coverage(&estate).contains(": allow"));
        refuse_plan_model_class_coverage(&estate).unwrap();
        persist_mesh(&state, &box_mesh()).unwrap();
        let before = snapshot(&state);
        let estate_bytes = fs::read(&path).unwrap();
        let reviewed = reviewed_sentinel(&dir);
        let cited = plan_cites_before_hop(&estate, &state).unwrap();
        let hop = cited.expect("hop mismatch is returned after quiet model rows");
        assert!(
            hop.contains("refuse:hop-coverage") && hop.contains("(mismatch)"),
            "{hop}"
        );
        assert!(!hop.contains("refuse:model-class"), "{hop}");
        let err = plan(&dir, &path, &state, true).unwrap_err().to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert!(!err.contains("refuse:model-class"), "{err}");
        assert!(!err.contains("refuse:tool"), "{err}");
        assert!(!err.contains("refuse:mcp"), "{err}");
        assert!(!err.contains("refuse:mount"), "{err}");
        assert!(!err.contains("refuse:memory"), "{err}");
        assert!(!err.contains("refuse:agent-call"), "{err}");
        assert!(!err.contains("not reviewable"), "{err}");
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        assert_reviewed_untouched(&reviewed);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn example_estate_plan_stays_green_without_a_mesh() {
        let dir = scratch();
        let estate_path = dir.join("estate.yaml");
        fs::copy(repo_root().join("examples/estate.yaml"), &estate_path).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&estate_path).unwrap();
        let cites = model_class_coverage_cites(&estate);
        assert!(cites.iter().all(|cite| !cite.fail), "{cites:?}");
        assert!(
            cites
                .iter()
                .any(|cite| cite.line == "horizon frontier xai_grok: deny-default"),
            "{cites:?}"
        );
        refuse_plan_model_class_coverage(&estate).unwrap();
        plan(&dir, &estate_path, &state, false).unwrap();
        let locked = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
        assert_eq!(fs::read(&estate_path).unwrap(), locked);
        assert!(!state.join(MESH_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn examples_estate_yaml_stays_hash_locked() {
        let path = repo_root().join("examples/estate.yaml");
        let sum = Command::new("cksum").arg(&path).output().unwrap();
        let text = String::from_utf8_lossy(&sum.stdout);
        assert!(
            text.starts_with("43770130 3391"),
            "examples/estate.yaml cksum changed: {text}"
        );
    }
}

#[cfg(test)]
mod plan_declared_coverage_tests {
    use super::{
        cmd_plan, plan_cites_before_hop, plan_cites_before_hop_with, refuse_declared_cites,
        refuse_plan_declared_coverage,
    };
    use conveyor_proxy::{persist_mesh, ConveyorMesh, HopLease, MESH_FILE};
    use estate_schema::{
        declared_coverage_cites, declared_coverage_cites_from_rows, describe_declared_coverage,
        load_estate, CoverageRow, DeclaredCoverageCite,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn scratch() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-plan-declared-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_estate(dir: &Path, name: &str, intentions: &str) -> PathBuf {
        let path = dir.join(name);
        let mut text = fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
        let needle = "    mcp: []\n    models:\n      - id: local_slm\n";
        let insert = "    mcp:\n      - id: docs\n    models:\n      - id: local_slm\n";
        assert!(text.contains(needle), "research mcp block missing");
        text = text.replacen(needle, insert, 1);
        text = text.replace("intentions: []\n", intentions);
        fs::write(&path, text).unwrap();
        path
    }

    fn plan(dir: &Path, estate: &Path, state: &Path, reviewed: bool) -> anyhow::Result<()> {
        cmd_plan(
            estate,
            None,
            &dir.join("plans"),
            state,
            reviewed,
            &dir.join("reviewed"),
        )
    }

    fn snapshot(state: &Path) -> Vec<(String, Vec<u8>)> {
        [
            "placement-actual.json",
            "model-actual.json",
            MESH_FILE,
            "conveyor-hops.json",
            "conveyor-leases.json",
        ]
        .into_iter()
        .filter_map(|name| {
            let path = state.join(name);
            path.is_file()
                .then(|| (name.to_string(), fs::read(&path).unwrap()))
        })
        .collect()
    }

    fn box_mesh() -> ConveyorMesh {
        ConveyorMesh {
            schema: conveyor_proxy::MESH_SCHEMA.into(),
            hops: vec![],
            leases: vec![HopLease {
                hop_id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                granted: true,
                spawned: true,
                durable: true,
                driver: "box".into(),
                note: None,
                ttl_secs: None,
                issued_at: None,
                expires_at: None,
                agents: vec!["research".into()],
            }],
        }
    }

    fn allow_declared() -> &'static str {
        "intentions:\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes\n    kind: mount\n    effect: allow\n  - subject_agent: research\n    object: mcp:docs\n    kind: mcp\n    effect: allow\n"
    }

    fn reviewed_sentinel(dir: &Path) -> PathBuf {
        let reviewed = dir.join("reviewed");
        fs::create_dir_all(&reviewed).unwrap();
        fs::write(reviewed.join("keep.txt"), b"sentinel").unwrap();
        reviewed
    }

    fn assert_reviewed_untouched(reviewed: &Path) {
        assert_eq!(fs::read(reviewed.join("keep.txt")).unwrap(), b"sentinel");
        assert!(!reviewed.join("INDEX.md").exists());
        let extras: Vec<_> = fs::read_dir(reviewed)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name != "keep.txt")
            .collect();
        assert!(extras.is_empty(), "{extras:?}");
    }

    fn fact(agent: &str, line: &str) -> CoverageRow {
        CoverageRow {
            agent_id: agent.into(),
            line: line.into(),
            hop_id: String::new(),
            word: "",
            capability: String::new(),
        }
    }

    fn with_docs(estate: &estate_schema::Estate) -> estate_schema::Estate {
        let mut estate = estate.clone();
        estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap()
            .mcp
            .push(estate_schema::McpDecl {
                id: "docs".into(),
                description: None,
            });
        estate
    }

    #[test]
    fn plan_cites_deny_and_deny_default_without_failing_and_stays_quiet_on_allow() {
        let dir = scratch();
        let deny_path = write_estate(
            &dir,
            "deny.yaml",
            "intentions:\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: deny\n",
        );
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&deny_path).unwrap();
        let described = describe_declared_coverage(&estate);
        let cites = declared_coverage_cites(&estate);
        assert!(cites.iter().all(|cite| !cite.fail), "{cites:?}");
        for line in [
            "research tool notes-append: deny",
            "research mount notes: deny-default",
            "research mcp docs: deny-default",
        ] {
            assert!(cites.iter().any(|cite| cite.line == line), "{cites:?}");
            assert!(described.contains(line), "{described}");
            assert!(!line.contains("(mismatch)"), "{line}");
        }
        assert!(
            cites.iter().all(|cite| !cite.line.ends_with(": allow")),
            "{cites:?}"
        );
        refuse_plan_declared_coverage(&estate).unwrap();
        persist_mesh(&state, &box_mesh()).unwrap();
        let before = snapshot(&state);
        let estate_bytes = fs::read(&deny_path).unwrap();
        let reviewed = reviewed_sentinel(&dir);
        plan(&dir, &deny_path, &state, false).unwrap();
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&deny_path).unwrap(), estate_bytes);
        assert_reviewed_untouched(&reviewed);

        let allow_path = write_estate(&dir, "allow.yaml", allow_declared());
        let allowed = load_estate(&allow_path).unwrap();
        assert!(declared_coverage_cites(&allowed).is_empty());
        assert!(describe_declared_coverage(&allowed).contains("research tool notes-append: allow"));
        assert!(describe_declared_coverage(&allowed).contains("research mount notes: allow"));
        assert!(describe_declared_coverage(&allowed).contains("research mcp docs: allow"));
        refuse_plan_declared_coverage(&allowed).unwrap();
        let allow_before = snapshot(&state);
        let allow_bytes = fs::read(&allow_path).unwrap();
        plan(&dir, &allow_path, &state, false).unwrap();
        assert_eq!(snapshot(&state), allow_before);
        assert_eq!(fs::read(&allow_path).unwrap(), allow_bytes);
        assert_reviewed_untouched(&reviewed);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuse_declared_bails_before_hop_and_leaves_dirs_untouched() {
        let dir = scratch();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = with_docs(&load_estate(&repo_root().join("examples/estate.yaml")).unwrap());
        assert!(
            declared_coverage_cites(&estate)
                .iter()
                .all(|cite| !cite.fail),
            "a loaded estate's own rows are deny or deny-default, not a hard cite"
        );
        refuse_plan_declared_coverage(&estate).unwrap();
        persist_mesh(&state, &box_mesh()).unwrap();
        let before = snapshot(&state);
        let reviewed = reviewed_sentinel(&dir);

        let missing = declared_coverage_cites_from_rows(&estate, &[]);
        assert!(missing.iter().all(|cite| cite.fail), "{missing:?}");
        assert!(
            missing.iter().any(|cite| {
                cite.line == "refuse:tool: research tool notes-append: deny-default (missing edge)"
            }),
            "{missing:?}"
        );
        assert!(
            missing
                .iter()
                .any(|cite| cite.line.contains("refuse:mcp") && cite.line.contains("docs")),
            "{missing:?}"
        );
        assert!(
            missing.iter().any(|cite| {
                cite.line == "refuse:mount: research mount notes: deny-default (missing edge)"
            }),
            "{missing:?}"
        );
        assert!(missing.iter().all(|cite| !cite.line.contains("(mismatch)")));
        let err = plan_cites_before_hop_with(&estate, &state, &missing)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("refuse:tool")
                && err.contains("notes-append")
                && err.contains("(missing edge)"),
            "{err}"
        );
        assert!(!err.contains("refuse:hop-coverage"), "{err}");
        assert!(!err.contains("(mismatch)"), "{err}");
        assert!(!err.contains("refuse:model-class"), "{err}");
        assert!(!err.contains("refuse:memory"), "{err}");
        assert!(!err.contains("refuse:agent-call"), "{err}");
        let helper = refuse_declared_cites(&missing).unwrap_err().to_string();
        assert!(helper.contains("refuse:tool"), "{helper}");
        assert_eq!(snapshot(&state), before);
        assert_reviewed_untouched(&reviewed);

        let mcp_rows = vec![
            fact("research", "research tool notes-append: allow"),
            fact("research", "research mcp docs: bogon"),
            fact("research", "research mount notes: allow"),
        ];
        let mcp = declared_coverage_cites_from_rows(&estate, &mcp_rows);
        let mcp_hard: Vec<_> = mcp.iter().filter(|cite| cite.fail).collect();
        assert_eq!(mcp_hard.len(), 1, "{mcp:?}");
        assert_eq!(mcp_hard[0].line, "refuse:mcp: research mcp docs: bogon");
        assert!(!mcp_hard[0].line.contains("(mismatch)"));
        let err = plan_cites_before_hop_with(&estate, &state, &mcp)
            .unwrap_err()
            .to_string();
        assert!(err.contains("refuse:mcp") && err.contains("bogon"), "{err}");
        assert!(!err.contains("refuse:tool"), "{err}");
        assert!(!err.contains("refuse:mount"), "{err}");
        assert!(!err.contains("refuse:hop-coverage"), "{err}");
        assert!(!err.contains("(mismatch)"), "{err}");
        assert_eq!(snapshot(&state), before);
        assert_reviewed_untouched(&reviewed);

        let mount_rows = vec![
            fact("research", "research tool notes-append: allow"),
            fact("research", "research mcp docs: allow"),
            fact("research", "research mount notes: bogon"),
        ];
        let mount = declared_coverage_cites_from_rows(&estate, &mount_rows);
        let mount_hard: Vec<_> = mount.iter().filter(|cite| cite.fail).collect();
        assert_eq!(mount_hard.len(), 1, "{mount:?}");
        assert_eq!(
            mount_hard[0].line,
            "refuse:mount: research mount notes: bogon"
        );
        let err = plan_cites_before_hop_with(&estate, &state, &mount)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("refuse:mount") && err.contains("bogon"),
            "{err}"
        );
        assert!(!err.contains("refuse:hop-coverage"), "{err}");
        assert!(!err.contains("(mismatch)"), "{err}");
        assert!(!err.contains("refuse:memory"), "{err}");
        assert!(!err.contains("refuse:agent-call"), "{err}");
        assert_eq!(snapshot(&state), before);
        assert_reviewed_untouched(&reviewed);

        let mut ghost = estate.clone();
        ghost
            .agents
            .iter_mut()
            .find(|agent| agent.id == "horizon")
            .unwrap()
            .models
            .push(estate_schema::ModelUseDecl {
                id: "ghost_slm".into(),
                description: None,
            });
        let err = plan_cites_before_hop_with(&ghost, &state, &missing)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("refuse:model-class") && err.contains("ghost_slm"),
            "{err}"
        );
        assert!(!err.contains("refuse:tool"), "{err}");
        assert!(!err.contains("refuse:hop-coverage"), "{err}");
        assert_eq!(snapshot(&state), before);
        assert_reviewed_untouched(&reviewed);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hop_mismatch_still_bails_when_declared_rows_are_allow() {
        let dir = scratch();
        let intentions = "intentions:\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes\n    kind: mount\n    effect: allow\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: mcp:docs\n    kind: mcp\n    effect: allow\n";
        let path = write_estate(&dir, "clean.yaml", intentions);
        let mut text = fs::read_to_string(&path).unwrap();
        text = text.replace(
            "      - id: notes-append\n",
            "      - id: notes-append\n      - id: lane-tool\n",
        );
        fs::write(&path, text).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&path).unwrap();
        assert!(
            declared_coverage_cites(&estate).is_empty(),
            "tool mcp mount allow stays quiet"
        );
        assert!(describe_declared_coverage(&estate).contains("research tool notes-append: allow"));
        assert!(describe_declared_coverage(&estate).contains("research tool lane-tool: allow"));
        assert!(describe_declared_coverage(&estate).contains("research mount notes: allow"));
        assert!(describe_declared_coverage(&estate).contains("research mcp docs: allow"));
        refuse_plan_declared_coverage(&estate).unwrap();
        persist_mesh(&state, &box_mesh()).unwrap();
        let before = snapshot(&state);
        let estate_bytes = fs::read(&path).unwrap();
        let reviewed = reviewed_sentinel(&dir);
        let cited = plan_cites_before_hop(&estate, &state).unwrap();
        let hop = cited.expect("hop mismatch is returned after quiet declared rows");
        assert!(
            hop.contains("refuse:hop-coverage") && hop.contains("(mismatch)"),
            "{hop}"
        );
        assert!(!hop.contains("refuse:tool"), "{hop}");
        assert!(!hop.contains("refuse:mcp"), "{hop}");
        assert!(!hop.contains("refuse:mount"), "{hop}");
        let err = plan(&dir, &path, &state, true).unwrap_err().to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert!(!err.contains("refuse:tool"), "{err}");
        assert!(!err.contains("refuse:mcp"), "{err}");
        assert!(!err.contains("refuse:mount"), "{err}");
        assert!(!err.contains("refuse:model-class"), "{err}");
        assert!(!err.contains("refuse:memory"), "{err}");
        assert!(!err.contains("refuse:agent-call"), "{err}");
        assert!(!err.contains("not reviewable"), "{err}");
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&path).unwrap(), estate_bytes);
        assert_reviewed_untouched(&reviewed);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn example_estate_plan_stays_green_without_a_mesh() {
        let dir = scratch();
        let estate_path = dir.join("estate.yaml");
        fs::copy(repo_root().join("examples/estate.yaml"), &estate_path).unwrap();
        let state = dir.join("state");
        fs::create_dir_all(&state).unwrap();
        let estate = load_estate(&estate_path).unwrap();
        let cites = declared_coverage_cites(&estate);
        assert!(cites.iter().all(|cite| !cite.fail), "{cites:?}");
        assert!(
            cites
                .iter()
                .any(|cite| cite.line == "research tool notes-append: deny-default"),
            "{cites:?}"
        );
        assert!(
            cites
                .iter()
                .any(|cite| cite.line == "research mount notes: deny-default"),
            "{cites:?}"
        );
        refuse_plan_declared_coverage(&estate).unwrap();
        plan(&dir, &estate_path, &state, false).unwrap();
        let locked = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
        assert_eq!(fs::read(&estate_path).unwrap(), locked);
        assert!(!state.join(MESH_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn examples_estate_yaml_stays_hash_locked() {
        let path = repo_root().join("examples/estate.yaml");
        let sum = Command::new("cksum").arg(&path).output().unwrap();
        let text = String::from_utf8_lossy(&sum.stdout);
        assert!(
            text.starts_with("43770130 3391"),
            "examples/estate.yaml cksum changed: {text}"
        );
    }

    #[test]
    fn hard_cite_line_is_the_bail_not_a_note() {
        let note = DeclaredCoverageCite {
            fail: false,
            line: "research mount notes: deny".into(),
        };
        let hard = DeclaredCoverageCite {
            fail: true,
            line: "refuse:tool: research tool notes-append: deny-default (missing edge)".into(),
        };
        let err = refuse_declared_cites(&[note, hard])
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "refuse:tool: research tool notes-append: deny-default (missing edge)"
        );
    }
}
