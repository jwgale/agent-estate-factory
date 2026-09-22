use anyhow::{bail, Context, Result};
use conveyor_proxy::{
    call_hop, declare_hop, forget_expired_hop_leases, hop_now_unix, list_expired_hop_leases,
    list_hop_leases, list_hops, sync_from_placements, HopDecl,
};
use estate_schema::{
    describe_placements,
    estate_hash, list_plans, load_estate, load_estate_unvalidated,
    load_policy, policy_allows,
};
use feed_collector::{
    import_pack_for, list_drop_packs, load_cursor, materialize_from_feed,
    propose_enrich, refuse_promote, refuse_propose_frontier_invent, write_pack_index,
    LOCKED_CURATOR,
};
use floor_supervisor::{
    backup_cell, drift_with_roots, list_apply_audits, list_lifecycle_events,
    list_session_events, load_placements, pause_kit_proof, prune_cell_backups,
    refuse_lease_host_classes,
    reconcile_placements, record_placements, render_restore, restore_cell,
    resume, suspend, tail_session_events, write_reconcile,
};
use std::path::{Path, PathBuf};

use crate::helpers::{copy_if_exists, copy_tree_files, enforce_policy, snapshot_state_files};

pub(crate) fn cmd_plans(plans_dir: &Path) -> Result<()> {
    let entries = list_plans(plans_dir)?;
    if entries.is_empty() {
        println!("no plans in {}", plans_dir.display());
        return Ok(());
    }
    println!("plan history ({})", plans_dir.display());
    for entry in entries {
        println!(
            "  {} hash={} against={}",
            entry.markdown,
            entry.desired_hash.as_deref().unwrap_or("-"),
            entry.against_hash.as_deref().unwrap_or("(greenfield)")
        );
    }
    Ok(())
}

pub(crate) fn cmd_suspend(state_dir: &Path) -> Result<()> {
    let record = suspend(state_dir)?;
    println!("lifecycle: {}", record.state.as_str());
    println!("{}", record.note);
    Ok(())
}

pub(crate) fn cmd_resume(path: &Path, state_dir: &Path, roots_base: &Path) -> Result<()> {
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    let (actual, record) = resume(&estate, state_dir, roots_base)?;
    model_estate::record_bindings(&estate, state_dir)?;
    record_placements(&estate, state_dir)?;
    model_estate::write_bound_catalog(&state_dir.join("catalog.json"), &estate)?;
    println!(
        "resumed {} sessions; lifecycle {}",
        actual.sessions.len(),
        record.state.as_str()
    );
    println!("{}", describe_placements(&estate));
    Ok(())
}

pub(crate) fn cmd_feed_pack(feed_dir: &Path, drop_dir: &Path, id: &str) -> Result<()> {
    let (pack, path) = materialize_from_feed(feed_dir, drop_dir, id)?;
    println!(
        "wrote candidate pack {} (promoted={}, events={}) -> {}",
        pack.id,
        pack.promoted,
        pack.from_events,
        path.display()
    );
    println!("{}", pack.note);
    Ok(())
}

pub(crate) fn cmd_catalog(out: &Path) -> Result<()> {
    let written = model_estate::write_catalog(out)?;
    println!("{}", model_estate::render_catalog());
    println!("wrote catalog file {}", written.display());
    Ok(())
}

pub(crate) fn cmd_policy_check(path: &Path, action: &str, hop: Option<&str>) -> Result<()> {
    if !path.is_file() {
        bail!("policy file missing: {}", path.display());
    }
    let pack = load_policy(path).map_err(|e| anyhow::anyhow!("{e}"))?;
    policy_allows(&pack, action, hop).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("policy ok action={action} hop={}", hop.unwrap_or("-"));
    Ok(())
}

pub(crate) fn cmd_backup(
    state_dir: &Path,
    plans_dir: &Path,
    out: &Path,
    estate_path: &Path,
    policy: &Path,
    prune: Option<usize>,
) -> Result<()> {
    enforce_policy(policy, "backup", None)?;
    let estate = crate::helpers::load_estate_if_present(estate_path)?;
    let (dest, meta) = backup_cell(
        state_dir,
        Some(plans_dir),
        out,
        estate.as_ref(),
    )?;
    println!("{}", serde_json::to_string_pretty(&meta)?);
    println!("Wrote {}", dest.display());
    if let Some(keep) = prune {
        let report = prune_cell_backups(out, keep)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        println!(
            "prune keep={} kept={} removed={}",
            report.keep,
            report.kept.len(),
            report.removed.len()
        );
    }
    Ok(())
}

pub(crate) fn cmd_restore(
    from: &Path,
    state_dir: &Path,
    plans_dir: &Path,
    estate_path: &Path,
    dry_run: bool,
    policy: &Path,
) -> Result<()> {
    enforce_policy(policy, "restore", None)?;
    let estate = crate::helpers::load_estate_if_present(estate_path)?;
    let before = if dry_run {
        snapshot_state_files(state_dir)
    } else {
        Vec::new()
    };
    let report = restore_cell(from, state_dir, Some(plans_dir), estate.as_ref(), dry_run)?;
    if dry_run {
        let after = snapshot_state_files(state_dir);
        if before != after {
            bail!("restore dry-run must not write state files");
        }
    }
    print!("{}", render_restore(&report));
    if report.would_refuse {
        bail!("restore would-refuse");
    }
    if dry_run {
        println!("dry-run ok (no writes)");
    }
    Ok(())
}

pub(crate) fn cmd_pause_proof(estate_path: &Path, state_dir: &Path, roots_base: &Path) -> Result<()> {
    let estate = load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
    let proof = pause_kit_proof(&estate, state_dir, roots_base)?;
    println!("{}", serde_json::to_string_pretty(&proof)?);
    if proof.cloud_spawned || !proof.leases_survived || !proof.in_sync {
        bail!("pause-proof failed");
    }
    Ok(())
}

pub(crate) fn cmd_leases(state_dir: &Path) -> Result<()> {
    match load_placements(state_dir)? {
        None => {
            println!("no placement-actual.json under {}", state_dir.display());
            println!("run apply or resume to record leases");
            Ok(())
        }
        Some(places) => {
            refuse_lease_host_classes(&places)?;
            println!("{}", serde_json::to_string_pretty(&places)?);
            for lease in &places.leases {
                if lease.kind == "cloud-agent" && lease.spawned {
                    bail!("cloud-agent lease spawned (fail closed)");
                }
            }
            Ok(())
        }
    }
}

pub(crate) fn cmd_audits(state_dir: &Path) -> Result<()> {
    let audits = list_apply_audits(state_dir)?;
    if audits.is_empty() {
        println!("no apply-audit.jsonl under {}", state_dir.display());
        return Ok(());
    }
    println!("apply audits ({})", audits.len());
    for audit in audits {
        println!(
            "  {} hash={} sessions={} require_plan={} covering={} spawned_cloud={}",
            audit.created_at,
            audit.desired_hash,
            audit.sessions,
            audit.require_plan,
            audit.covering_plan.as_deref().unwrap_or("-"),
            audit.cloud_agent_spawned
        );
    }
    Ok(())
}

pub(crate) fn cmd_feed_list(drop_dir: &Path) -> Result<()> {
    let packs = list_drop_packs(drop_dir)?;
    let index = write_pack_index(drop_dir)?;
    if packs.is_empty() {
        println!("no candidate packs in {}", drop_dir.display());
        println!("index {}", index.display());
        return Ok(());
    }
    for pack in packs {
        let drivers = if pack.source_drivers.is_empty() {
            "-".to_string()
        } else {
            pack.source_drivers.join(",")
        };
        println!(
            "  {} events={} drivers={} promoted={} policy={}/{}",
            pack.id, pack.from_events, drivers, pack.promoted, pack.curator, pack.policy
        );
    }
    println!("index {}", index.display());
    Ok(())
}

pub(crate) fn cmd_feed_import(
    id: &str,
    drop_dir: &Path,
    accepted_dir: &Path,
    estate_path: &Path,
    curator: &str,
) -> Result<()> {
    let estate = load_estate(estate_path)
        .with_context(|| format!("load {}", estate_path.display()))?;
    let ids: Vec<String> = estate
        .enrich_packs
        .packs
        .iter()
        .map(|p| p.id.clone())
        .collect();
    let before = crate::helpers::read_estate_text(estate_path)?;
    let (rec, dest) = import_pack_for(
        drop_dir,
        accepted_dir,
        id,
        &ids,
        curator,
        &estate.enrich_packs.curator,
    )?;
    let after = crate::helpers::read_estate_text(estate_path)?;
    if before != after {
        bail!("import must not rewrite the estate file");
    }
    println!(
        "imported pack {} schema={} estate_bound={} promoted={} curator={} (locked={}) -> {}",
        rec.pack.id,
        rec.pack.schema,
        rec.estate_bound,
        rec.pack.promoted,
        curator,
        LOCKED_CURATOR,
        dest.display()
    );
    let redaction = accepted_dir.join(format!("{}.redaction.json", rec.pack.id));
    if redaction.is_file() {
        if let Ok(text) = std::fs::read_to_string(&redaction) {
            println!("redaction report {}", redaction.display());
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                println!(
                    "  redacted={} raw_secrets_found={}",
                    v.get("redacted").and_then(|x| x.as_u64()).unwrap_or(0),
                    v.get("raw_secrets_found")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(0)
                );
            }
        }
    }
    println!("estate file unchanged. Jason still lists pack ids on enrich_packs by hand.");
    Ok(())
}

pub(crate) fn cmd_feed_promote(id: &str) -> Result<()> {
    match refuse_promote(id) {
        Ok(()) => unreachable!("promote has no success path"),
        Err(err) => bail!("{err}"),
    }
}

pub(crate) fn cmd_feed_cursor(feed_dir: &Path) -> Result<()> {
    match load_cursor(feed_dir)? {
        None => {
            println!("no feed-cursor.json under {}", feed_dir.display());
            Ok(())
        }
        Some(cursor) => {
            println!("{}", serde_json::to_string_pretty(&cursor)?);
            Ok(())
        }
    }
}

pub(crate) fn cmd_history(state_dir: &Path) -> Result<()> {
    let events = list_lifecycle_events(state_dir)?;
    if events.is_empty() {
        println!("no lifecycle.jsonl under {}", state_dir.display());
        return Ok(());
    }
    println!("lifecycle history ({})", events.len());
    for ev in events {
        println!(
            "  {} {} -> {} hash={}",
            ev.action,
            ev.from.as_deref().unwrap_or("-"),
            ev.to,
            ev.desired_hash.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

pub(crate) fn cmd_convey_hop(
    id: &str,
    kind: &str,
    capability: &str,
    host_class: &str,
    wired: bool,
    ttl_secs: Option<u64>,
    state_dir: &Path,
) -> Result<()> {
    let lease = declare_hop(
        state_dir,
        HopDecl {
            id: id.to_string(),
            kind: kind.to_string(),
            capability: capability.to_string(),
            host_class: host_class.to_string(),
            wired,
            note: None,
            ttl_secs,
        },
    )?;
    println!("{}", serde_json::to_string_pretty(&lease)?);
    Ok(())
}

pub(crate) fn cmd_convey_call(id: &str, capability: &str, state_dir: &Path, policy: &Path) -> Result<()> {
    enforce_policy(policy, "convey-call", Some(id))?;
    let call = call_hop(state_dir, id, capability)?;
    println!("{}", serde_json::to_string_pretty(&call)?);
    if !call.allow {
        bail!("hop call denied");
    }
    Ok(())
}

pub(crate) fn cmd_convey_list(state_dir: &Path) -> Result<()> {
    let hops = list_hops(state_dir)?;
    if hops.is_empty() {
        println!("no hops under {}", state_dir.display());
        return Ok(());
    }
    println!("{}", serde_json::to_string_pretty(&hops)?);
    Ok(())
}

pub(crate) fn cmd_convey_leases(state_dir: &Path) -> Result<()> {
    let leases = list_hop_leases(state_dir)?;
    if leases.is_empty() {
        println!("no hop leases under {}", state_dir.display());
        return Ok(());
    }
    println!("{}", serde_json::to_string_pretty(&leases)?);
    Ok(())
}

pub(crate) fn cmd_convey_sync(state_dir: &Path) -> Result<()> {
    let mesh = sync_from_placements(state_dir)?;
    println!("{}", serde_json::to_string_pretty(&mesh)?);
    Ok(())
}

pub(crate) fn cmd_convey_expire(state_dir: &Path, forget: bool) -> Result<()> {
    let expired = list_expired_hop_leases(state_dir, hop_now_unix())?;
    if expired.is_empty() {
        println!("no expired hop leases under {}", state_dir.display());
        return Ok(());
    }
    println!("expired hop leases ({})", expired.len());
    for lease in &expired {
        println!(
            "  refuse:expired: {} kind={} expires_at={}",
            lease.hop_id,
            lease.kind,
            lease
                .expires_at
                .map(|e| e.to_string())
                .unwrap_or_else(|| "-".into())
        );
    }
    if forget {
        let forgotten = forget_expired_hop_leases(state_dir)?;
        println!(
            "forgot {} expired hop lease(s); call restamps from remaining hop decls",
            forgotten.len()
        );
        return Ok(());
    }
    bail!("refuse:expired: convey call refuses until estate convey expire --forget");
}

pub(crate) fn cmd_sessions_list(state_dir: &Path) -> Result<()> {
    let events = list_session_events(state_dir)?;
    if events.is_empty() {
        println!("no sessions.jsonl under {}", state_dir.display());
        return Ok(());
    }
    println!("session journal ({})", events.len());
    for ev in events {
        println!(
            "  {} {} agent={} hash={}",
            ev.ts,
            ev.action,
            ev.agent_id.as_deref().unwrap_or("-"),
            ev.desired_hash.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

pub(crate) fn cmd_sessions_tail(state_dir: &Path, n: usize) -> Result<()> {
    let events = tail_session_events(state_dir, n)?;
    if events.is_empty() {
        println!("no sessions.jsonl under {}", state_dir.display());
        return Ok(());
    }
    println!("session journal tail ({}/{})", events.len(), n);
    for ev in events {
        println!(
            "  {} {} agent={} hash={}",
            ev.ts,
            ev.action,
            ev.agent_id.as_deref().unwrap_or("-"),
            ev.desired_hash.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

pub(crate) fn cmd_packs_index(drop_dir: &Path) -> Result<()> {
    let path = write_pack_index(drop_dir)?;
    println!("wrote {}", path.display());
    Ok(())
}

pub(crate) fn cmd_packs_propose(
    id: &str,
    drop_dir: &Path,
    accepted_dir: &Path,
    proposed_dir: &Path,
    estate_path: &Path,
) -> Result<()> {
    let parsed = load_estate_unvalidated(estate_path)
        .with_context(|| format!("load {}", estate_path.display()))?;
    refuse_propose_frontier_invent(drop_dir, accepted_dir, id, &parsed)?;
    let estate = load_estate(estate_path)
        .with_context(|| format!("load {}", estate_path.display()))?;
    let before = crate::helpers::read_estate_text(estate_path)?;
    let (proposal, dest) = propose_enrich(drop_dir, accepted_dir, proposed_dir, id, &estate)?;
    let after = crate::helpers::read_estate_text(estate_path)?;
    if before != after {
        bail!("propose must not rewrite the estate file");
    }
    if proposal.auto_apply {
        bail!("proposal must not auto-apply");
    }
    print!("{}", feed_collector::render_proposal(&proposal));
    println!("Wrote {}", dest.display());
    println!("proposal only; Jason edits the estate by hand. auto_apply=false.");
    Ok(())
}

pub(crate) fn cmd_audit_export(
    estate_path: &Path,
    state_dir: &Path,
    plans_dir: &Path,
    packs_dir: &Path,
    out: &Path,
    tar: bool,
) -> Result<()> {
    let estate = load_estate(estate_path)
        .with_context(|| format!("load {}", estate_path.display()))?;
    let report = reconcile_placements(&estate, state_dir)?;
    write_reconcile(state_dir, &report)?;
    if out.exists() {
        std::fs::remove_dir_all(out)?;
    }
    std::fs::create_dir_all(out)?;
    let mut copied = Vec::new();
    let mut missing = Vec::new();
    let files = [
        (state_dir.join("lifecycle.json"), out.join("lifecycle.json")),
        (state_dir.join("lifecycle.jsonl"), out.join("lifecycle.jsonl")),
        (
            state_dir.join("apply-audit.jsonl"),
            out.join("apply-audit.jsonl"),
        ),
        (
            state_dir.join("placement-actual.json"),
            out.join("placement-actual.json"),
        ),
        (state_dir.join("reconcile.json"), out.join("reconcile.json")),
        (state_dir.join("reconcile.md"), out.join("reconcile.md")),
        (
            state_dir.join("conveyor-leases.json"),
            out.join("conveyor-leases.json"),
        ),
        (state_dir.join("sessions.jsonl"), out.join("sessions.jsonl")),
        (
            packs_dir.join("accepted").join("import-audit.jsonl"),
            out.join("import-audit.jsonl"),
        ),
    ];
    for (src, dest) in &files {
        if !copy_if_exists(src, dest, &mut copied)? {
            missing.push(src.display().to_string());
        }
    }
    copy_tree_files(plans_dir, &out.join("plans"), &mut copied)?;
    let proposed = packs_dir.join("proposed");
    if proposed.is_dir() {
        copy_tree_files(&proposed, &out.join("proposed"), &mut copied)?;
    }
    let mut manifest = String::from("Cell One audit export (local only)\n");
    manifest.push_str("===================================\n");
    manifest.push_str("Not a remote upload. Not a gateway dump. Review on this box.\n\n");
    manifest.push_str(&format!("estate: {}\n", estate.name));
    manifest.push_str(&format!("estate_file: {}\n", estate_path.display()));
    manifest.push_str(&format!("estate_hash: {}\n", estate_hash(&estate)));
    manifest.push_str(&format!("reconcile_in_sync: {}\n", report.in_sync));
    manifest.push_str(&format!("refuses: {}\n\n", report.refuses.len()));
    manifest.push_str("Copied\n------\n");
    if copied.is_empty() {
        manifest.push_str("(none)\n");
    } else {
        for item in &copied {
            manifest.push_str(&format!("  {item}\n"));
        }
    }
    manifest.push_str("\nMissing (ok if never applied)\n----------------------------\n");
    if missing.is_empty() {
        manifest.push_str("(none)\n");
    } else {
        for item in &missing {
            manifest.push_str(&format!("  {item}\n"));
        }
    }
    std::fs::write(out.join("MANIFEST.md"), &manifest)?;
    let meta = serde_json::json!({
        "schema": "cell-one.audit-export.v0",
        "estate": estate.name,
        "estate_hash": estate_hash(&estate),
        "reconcile_in_sync": report.in_sync,
        "refuses": report.refuses.len(),
        "copied": copied.len(),
        "missing": missing.len(),
        "remote": false,
    });
    std::fs::write(out.join("audit-export.json"), serde_json::to_string_pretty(&meta)?)?;
    println!("{manifest}");
    println!("Wrote {}", out.display());
    if tar {
        let parent = out.parent().unwrap_or_else(|| Path::new("."));
        let name = out.file_name().and_then(|s| s.to_str()).unwrap_or("audit-export");
        let tarball = PathBuf::from(format!("{}.tar.gz", out.display()));
        match std::process::Command::new("tar")
            .arg("-czf")
            .arg(&tarball)
            .arg("-C")
            .arg(parent)
            .arg(name)
            .status()
        {
            Ok(status) if status.success() && tarball.is_file() => {
                println!("Wrote {}", tarball.display());
            }
            Ok(_) | Err(_) => {
                println!("tar not available or failed; folder is the review artifact");
            }
        }
    }
    Ok(())
}

pub(crate) fn cmd_drift(path: &Path, state_dir: &Path, roots_base: &Path) -> Result<()> {
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    let report = drift_with_roots(&estate, state_dir, Some(roots_base))?;
    let models = model_estate::drift_bindings(&estate, state_dir)?;
    println!("floor:\n{}", serde_json::to_string_pretty(&report)?);
    println!("models:\n{}", serde_json::to_string_pretty(&models)?);
    if !report.in_sync || !models.in_sync {
        bail!("drift detected");
    }
    Ok(())
}

pub(crate) fn cmd_models(path: &Path) -> Result<()> {
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    println!("{}", model_estate::describe_bindings(&estate));
    println!("{}", model_estate::readiness(&estate));
    for binding in &estate.model_bindings {
        match model_estate::ping(&estate, &binding.id) {
            Ok(()) => println!("  ping {}: wired (control will not complete)", binding.id),
            Err(err) => println!("  refuse: {err}"),
        }
    }
    Ok(())
}
