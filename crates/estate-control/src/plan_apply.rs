use anyhow::{bail, Context, Result};
use estate_schema::{
    blast_grows, covering_plan, describe_placements, diff_estates,
    estate_hash, latest_plan, load_estate, load_plan_json, mark_plan_reviewed, plan_against_is_fresh, plan_against_is_fresh_strict,
    plan_blast_width, plan_is_reviewable, render_plan, render_plan_diff,
    write_plan,
};
use feed_collector::import_pack_for;
use floor_supervisor::{
    append_apply_audit, apply_dry_run, apply_with_profile_dir, load_desired_snapshot, mark_running, record_placements, refuse_expired_leases,
    render_dry_run, ApplyAudit,
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
    let desired = load_estate(path).with_context(|| format!("desired {}", path.display()))?;
    let previous = match against {
        Some(p) => Some(load_estate(p).with_context(|| format!("against {}", p.display()))?),
        None => load_desired_snapshot(state_dir)?,
    };
    let plan = diff_estates(&desired, previous.as_ref());
    let written = write_plan(plans_dir, &plan)?;
    print!("{}", render_plan(&plan));
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
        if let Some(covering) = covering_plan(plans_dir, &estate_hash(&snap)) {
            return Ok(Some(covering.plan));
        }
        if let Some(latest) = latest_plan(plans_dir) {
            return Ok(Some(latest));
        }
        return Ok(Some(diff_estates(&snap, None)));
    }
    Ok(latest_plan(plans_dir))
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
) -> Result<()> {
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    enforce_policy(policy, "apply", None)?;
    if dry_run {
        return cmd_apply_dry_run(
            &estate,
            path,
            state_dir,
            plans_dir,
            require_plan,
            require_fresh_plan,
        );
    }
    refuse_expired_leases(state_dir)?;
    let hash = estate_hash(&estate);
    let covering = covering_plan(plans_dir, &hash);
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
    let _ = model_estate::write_catalog(&state_dir.join("catalog.json"));
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
    };
    let audit_path = append_apply_audit(plans_dir, state_dir, &audit)?;
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
) -> Result<()> {
    let hash = estate_hash(estate);
    let covering = covering_plan(plans_dir, &hash);
    let last_applied = load_desired_snapshot(state_dir)?
        .as_ref()
        .map(estate_hash);
    let mut extra_refuses = Vec::new();
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
    println!("dry-run ok (no writes)");
    Ok(())
}

