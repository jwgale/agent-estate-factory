use anyhow::{bail, Context, Result};
use conveyor_proxy::{
    authority_report, call_hop, declare_hop, forget_expired_hop_leases, hop_now_unix,
    list_expired_hop_leases, list_hop_leases, list_hops, sync_from_placements, HopDecl,
};
use estate_schema::{
    convey_hop_coverage, convey_intention_coverage, describe_agents_section,
    describe_declared_coverage,
    describe_hop_coverage, describe_intention_coverage, describe_model_class_coverage,
    describe_placements, estate_hash, list_plans, load_estate, load_estate_unvalidated,
    load_policy, policy_allows,
};
use feed_collector::{
    import_pack_for, list_drop_packs, load_cursor, materialize_from_feed, propose_enrich,
    refuse_promote, refuse_propose_frontier_invent, write_pack_index, LOCKED_CURATOR,
};
use floor_supervisor::{
    backup_cell, drift_with_roots, list_apply_audits, list_lifecycle_events, list_session_events,
    load_placements, pause_kit_proof, prune_cell_backups, reconcile_placements, record_placements,
    refuse_lease_host_classes, render_restore, restore_cell, resume, suspend, tail_session_events,
    write_reconcile,
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
    let parsed = estate_schema::load_estate_unvalidated(path)
        .with_context(|| format!("load {}", path.display()))?;
    model_estate::frontier_plan_view(&parsed).map_err(|e| anyhow::anyhow!("{e}"))?;
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    crate::plan_apply::refuse_apply_catalog_mismatch(&estate, state_dir)?;
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
    model_estate::refuse_schema_catalog_overwrite(out).map_err(|err| anyhow::anyhow!("{err}"))?;
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
    let (dest, meta) = backup_cell(state_dir, Some(plans_dir), out, estate.as_ref())?;
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

pub(crate) fn cmd_pause_proof(
    estate_path: &Path,
    state_dir: &Path,
    roots_base: &Path,
) -> Result<()> {
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
    crate::plan_apply::refuse_apply_catalog_mismatch(&estate, state_dir)?;
    let proof = pause_kit_proof(&estate, state_dir, roots_base)?;
    // Drift, a spawned cloud lease, or lost leases is a refuse before
    // the proof JSON. That JSON's note is the clean kit, not a failure.
    if proof.cloud_spawned || !proof.leases_survived || !proof.in_sync {
        bail!("pause-proof failed");
    }
    println!("{}", serde_json::to_string_pretty(&proof)?);
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
            // A spawned cloud-agent lease is a refuse before the
            // placement JSON. An unspawned file still prints. The
            // lease file is not rewritten.
            let mut spawned = Vec::new();
            for lease in &places.leases {
                if lease.kind == "cloud-agent" && lease.spawned {
                    spawned.push(lease.placement_id.as_str());
                }
            }
            if !spawned.is_empty() {
                bail!(
                    "cloud-agent lease spawned (fail closed): {}",
                    spawned.join(", ")
                );
            }
            println!("{}", serde_json::to_string_pretty(&places)?);
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
    // Unvalidated on purpose. Cell One shape is an apply gate. A one-agent
    // local-only estate must hit refuse:frontier-invent before that shape
    // check, and a local-only pack must still import. CELL_FRONTIER_MODEL
    // is not the binding.
    let estate = load_estate_unvalidated(estate_path)
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
        &estate,
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
    if !redaction.is_file() {
        bail!(
            "import wrote no redaction report at {}",
            redaction.display()
        );
    }
    let text = std::fs::read_to_string(&redaction)
        .with_context(|| format!("read redaction {}", redaction.display()))?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("parse redaction {}", redaction.display()))?;
    let redacted = v
        .get("redacted")
        .and_then(|x| x.as_u64())
        .with_context(|| format!("redaction {} missing redacted count", redaction.display()))?;
    let raw_found = v
        .get("raw_secrets_found")
        .and_then(|x| x.as_u64())
        .with_context(|| {
            format!(
                "redaction {} missing raw_secrets_found",
                redaction.display()
            )
        })?;
    println!("redaction report {}", redaction.display());
    println!("  redacted={redacted} raw_secrets_found={raw_found}");
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

fn refuse_named_intentions(
    estate_path: &Path,
    hop_id: &str,
    capability: &str,
    agents: &[String],
    kind: Option<estate_schema::IntentionKind>,
    on_hop: bool,
) -> Result<()> {
    if agents.is_empty() || !estate_path.is_file() {
        return Ok(());
    }
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
    for agent in agents {
        if let Err(gate) =
            convey_intention_coverage(&estate, hop_id, agent, capability, kind, on_hop)
        {
            bail!("refuse:intention: {} ({})", gate.line, gate.word);
        }
    }
    Ok(())
}

fn refuse_convey_coverage(estate_path: &Path, hop_id: &str, agent: Option<&str>) -> Result<()> {
    // Coverage is mandatory on convey hop and convey call. A missing path
    // or a non-file (wrong-cwd default examples/estate.yaml included) is
    // not a grant and must not fall through to the lease stub.
    // A loaded estate still allows only `allow`. A hop id that is not a
    // placement stays the lease stub.
    if !estate_path.is_file() {
        let why = if estate_path.exists() {
            "not a file"
        } else {
            "missing"
        };
        bail!(
            "refuse:hop-coverage: estate {why}: {} (coverage is mandatory; not a grant)",
            estate_path.display()
        );
    }
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
    match convey_hop_coverage(&estate, hop_id, agent) {
        Ok(_) => Ok(()),
        Err(gate) => bail!("refuse:hop-coverage: {} ({})", gate.line, gate.word),
    }
}

pub(crate) fn cmd_convey_hop(
    id: &str,
    kind: &str,
    capability: &str,
    host_class: &str,
    wired: bool,
    ttl_secs: Option<u64>,
    agents: &[String],
    intention_kind: Option<&str>,
    estate_path: &Path,
    state_dir: &Path,
) -> Result<()> {
    // `--intention-kind` is the intention gate, the same tokens as
    // `convey call --kind`. Hop `--kind` stays the hop declaration kind.
    let parsed_kind = match intention_kind {
        Some(raw) => Some(conveyor_proxy::parse_kind(raw).map_err(anyhow::Error::msg)?),
        None => None,
    };
    if parsed_kind.is_some() && agents.is_empty() {
        bail!("refuse:agent-unbound: --intention-kind requires --agent");
    }
    refuse_named_intentions(estate_path, id, capability, agents, parsed_kind, true)?;
    if agents.is_empty() {
        refuse_convey_coverage(estate_path, id, None)?;
    } else {
        for agent in agents {
            refuse_convey_coverage(estate_path, id, Some(agent.as_str()))?;
        }
    }
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
            agents: agents.to_vec(),
        },
    )?;
    println!("{}", serde_json::to_string_pretty(&lease)?);
    Ok(())
}

pub(crate) fn cmd_convey_call(
    id: &str,
    capability: &str,
    agent: Option<&str>,
    kind: Option<&str>,
    estate_path: &Path,
    state_dir: &Path,
    policy: &Path,
) -> Result<()> {
    enforce_policy(policy, "convey-call", Some(id))?;
    let parsed_kind = match kind {
        Some(raw) => Some(conveyor_proxy::parse_kind(raw).map_err(anyhow::Error::msg)?),
        None => None,
    };
    if let Some(agent) = agent {
        refuse_named_intentions(
            estate_path,
            id,
            capability,
            &[agent.to_string()],
            parsed_kind,
            false,
        )?;
    }
    refuse_convey_coverage(estate_path, id, agent)?;
    let call = if let Some(agent) = agent {
        let estate = estate_schema::load_estate(estate_path)
            .with_context(|| format!("load {}", estate_path.display()))?;
        conveyor_proxy::call_hop_for_agent(state_dir, id, capability, agent, parsed_kind, &estate)?
    } else {
        if kind.is_some() {
            bail!("refuse:agent-unbound: --kind requires --agent");
        }
        call_hop(state_dir, id, capability)?
    };
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
    // list_hop_leases refuses a spawned cloud hop before the JSON.
    // An unspawned file still prints. A missing mesh is empty, not spawned.
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

/// File check. Does not write. Does not claim mediation. Not an identity lookup.
pub(crate) fn cmd_convey_authority(state_dir: &Path, estate_path: &Path) -> Result<()> {
    let estate = estate_schema::load_estate(estate_path)
        .with_context(|| format!("load {}", estate_path.display()))?;
    let rows = authority_report(state_dir, &estate)?;
    let allow = rows
        .iter()
        .filter(|row| row.status == "would-allow")
        .count();
    let deny = rows.iter().filter(|row| row.status == "would-deny").count();
    let pending = rows
        .iter()
        .filter(|row| row.status == "not-enforced")
        .count();
    println!("authority would-allow={allow} would-deny={deny} not-enforced={pending}");
    println!(
        "uncertain: a hop lease is a file. This report does not show that a worker called the conveyor."
    );
    if rows.is_empty() {
        println!("no hop leases under {}", state_dir.display());
        return Ok(());
    }
    println!("{}", serde_json::to_string_pretty(&rows)?);
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
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
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
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
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
        (
            state_dir.join("lifecycle.jsonl"),
            out.join("lifecycle.jsonl"),
        ),
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
    std::fs::write(
        out.join("audit-export.json"),
        serde_json::to_string_pretty(&meta)?,
    )?;
    println!("{manifest}");
    println!("Wrote {}", out.display());
    if tar {
        let parent = out.parent().unwrap_or_else(|| Path::new("."));
        let name = out
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("audit-export");
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
    println!("{}", describe_agents_section(&estate));
    println!("model class:\n{}", describe_model_class_coverage(&estate));
    println!("tool mcp mount:\n{}", describe_declared_coverage(&estate));
    println!("intention:\n{}", describe_intention_coverage(&estate));
    println!("hop:\n{}", describe_hop_coverage(&estate));
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

#[cfg(test)]
mod convey_coverage_tests {
    use super::refuse_convey_coverage;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn missing_estate_refuses_convey_coverage() {
        let path =
            std::env::temp_dir().join(format!("cell-missing-estate-{}.yaml", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let err = refuse_convey_coverage(&path, "ttl-box", None).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("refuse:hop-coverage"), "{msg}");
        assert!(msg.contains("estate missing"), "{msg}");
        assert!(msg.contains("coverage is mandatory"), "{msg}");
        assert!(msg.contains(&path.display().to_string()), "{msg}");
    }

    #[test]
    fn non_file_estate_refuses_convey_coverage() {
        let path = std::env::temp_dir().join(format!("cell-estate-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        let err = refuse_convey_coverage(&path, "ttl-box", None).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("refuse:hop-coverage"), "{msg}");
        assert!(msg.contains("estate not a file"), "{msg}");
        assert!(msg.contains("coverage is mandatory"), "{msg}");
        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn loaded_non_placement_stays_lease_stub() {
        let estate = repo_root().join("examples/estate.yaml");
        refuse_convey_coverage(&estate, "ttl-box", None).unwrap();
    }

    #[test]
    fn loaded_example_box_still_deny_default() {
        let estate = repo_root().join("examples/estate.yaml");
        let msg = refuse_convey_coverage(&estate, "cell-one-box", None)
            .unwrap_err()
            .to_string();
        assert!(msg.contains("refuse:hop-coverage"), "{msg}");
        assert!(msg.contains("(deny-default)"), "{msg}");
    }

    #[test]
    fn loaded_example_cloud_still_deny() {
        let estate = repo_root().join("examples/estate.yaml");
        let msg = refuse_convey_coverage(&estate, "cursor-cloud", None)
            .unwrap_err()
            .to_string();
        assert!(
            msg.contains("refuse:hop-coverage") && msg.contains("(deny)"),
            "{msg}"
        );
        assert!(!msg.contains("deny-default"), "{msg}");
    }
}
