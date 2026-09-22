use anyhow::{bail, Context, Result};
use estate_schema::{
    blast_grows, covering_plan, describe_placements, diff_estates,
    estate_hash, latest_plan, load_estate, load_plan_json, mark_plan_reviewed, overlay_sacred_ids,
    plan_against_is_fresh, plan_against_is_fresh_strict, plan_blast_width, plan_is_reviewable,
    render_plan, render_plan_diff, render_plan_pr, write_plan,
};
use feed_collector::{import_pack_for, refuse_import_pack};
use floor_supervisor::{
    append_apply_audit, apply_dry_run, apply_with_profile_dir, classify_apply, list_expired_leases,
    load_desired_snapshot, load_lifecycle, load_placements, mark_running, now_unix,
    record_placements, refuse_expired_leases, refuse_lease_host_classes, render_dry_run,
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
    if !plan_is_reviewable(&plan) {
        bail!("plan is not reviewable (need schema cell-one.plan.v0 + blast radius)");
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
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    load_lifecycle(state_dir)?;
    if !force {
        if let Some(places) = load_placements(state_dir)? {
            refuse_lease_host_classes(&places)?;
        }
    }
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
    let identity = classify_apply(&estate, state_dir, roots_base)?;
    if import_pack_id.is_none() {
        match &identity {
            ApplyIdentity::Unchanged if !force => {
                let audit = ApplyAudit {
                    created_at: chrono_stamp(),
                    desired_hash: hash,
                    sessions: 0,
                    imported_packs: vec![],
                    require_plan,
                    note: "unchanged: identical desired state; in_sync. No-op apply.".into(),
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
    let audit = ApplyAudit {
        created_at: chrono_stamp(),
        desired_hash: hash,
        sessions: actual.sessions.len(),
        imported_packs: imported,
        require_plan,
        note: "Apply audit. Estate file unchanged. Cloud-agent placements not spawned.".into(),
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
