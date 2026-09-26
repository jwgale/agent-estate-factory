use anyhow::{bail, Context, Result};
use conveyor_proxy::{
    append_proxy_audit, authority_report, call_hop, declare_hop_covering,
    describe_authority_section, forget_expired_hop_leases, hop_now_unix, list_expired_hop_leases,
    list_hop_leases, list_hops, load_mesh, refuse_mesh_host_classes, sync_from_placements,
    sync_from_placements_covering, HopDecl, MeshError,
};
use estate_schema::{
    convey_hop_declared_capability, convey_intention_coverage, describe_agents_section,
    describe_agent_edge_coverage, describe_declared_coverage, describe_hop_coverage,
    describe_intention_coverage, describe_model_class_coverage, describe_placements, estate_hash,
    list_plans, load_estate,
    load_estate_unvalidated, load_policy, policy_allows,
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

pub(crate) fn cmd_leases(estate_path: &Path, state_dir: &Path) -> Result<()> {
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
    // After the estate loads, before the placement list. Same stack as
    // status, doctor, reconcile, and audit export. Hop cites do not change
    // this command's exit code. A placement-actual SKU still reaches the
    // lease reader below, which refuses before the JSON.
    print!("{}", honesty_stack(&estate, state_dir)?);
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

pub(crate) fn cmd_audits(estate_path: &Path, state_dir: &Path) -> Result<()> {
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
    // After the estate loads, before the apply-audit list. Same stack as
    // estate leases, convey leases, status, doctor, reconcile, and audit
    // export. Hop cites do not change this command's exit code. The stack
    // reads placement-actual for mesh interpretation and Authority. A
    // placement-actual SKU omits Authority and the apply-audit list still
    // prints. There is no second placement refuse before that list. A
    // placement-actual parse failure refuses here, before the list.
    print!("{}", honesty_stack(&estate, state_dir)?);
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

pub(crate) fn cmd_history(estate_path: &Path, state_dir: &Path) -> Result<()> {
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
    // After the estate loads, before the lifecycle history list. Same stack
    // as estate leases, convey leases, and audits. Hop cites do not change
    // this command's exit code. The stack reads placement-actual for mesh
    // interpretation and Authority. A placement-actual SKU omits Authority
    // and the history body still prints. There is no second placement
    // refuse before that list. A placement-actual parse failure refuses
    // here, before the list.
    print!("{}", honesty_stack(&estate, state_dir)?);
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

fn record_proxy_gate(
    state_dir: &Path,
    hop_id: &str,
    agent: Option<&str>,
    capability: &str,
    word: &str,
    line: &str,
    kind: Option<estate_schema::IntentionKind>,
) -> Result<()> {
    let note = format!("hop={hop_id} capability={capability} {line}");
    // Hop coverage and an unresolved capability stay `proxy.hop`. A resolved
    // Agent intention is `proxy.agent`, the same stamp `check` writes.
    let (feed_kind, object_class) = if kind == Some(estate_schema::IntentionKind::Agent) {
        ("proxy.agent", "agent")
    } else {
        ("proxy.hop", "proxy")
    };
    append_proxy_audit(
        &state_dir.join("feed"),
        &feed_kind,
        agent,
        word,
        object_class,
        Some(&note),
    )
    .map_err(|err| anyhow::anyhow!("{err}"))?;
    Ok(())
}

fn refuse_named_intentions(
    estate_path: &Path,
    state_dir: &Path,
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
            record_proxy_gate(
                state_dir,
                hop_id,
                Some(agent.as_str()),
                capability,
                gate.word,
                &gate.line,
                gate.kind,
            )?;
            bail!("refuse:intention: {} ({})", gate.line, gate.word);
        }
    }
    Ok(())
}

fn refuse_convey_coverage(
    estate_path: &Path,
    state_dir: &Path,
    hop_id: &str,
    agent: Option<&str>,
    capability: &str,
) -> Result<()> {
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
        record_proxy_gate(
            state_dir,
            hop_id,
            agent,
            capability,
            "deny",
            &format!("estate {why}"),
            None,
        )?;
        bail!(
            "refuse:hop-coverage: estate {why}: {} (coverage is mandatory; not a grant)",
            estate_path.display()
        );
    }
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
    match convey_hop_declared_capability(&estate, hop_id, agent, capability) {
        Ok(_) => Ok(()),
        Err(gate) => {
            // `mismatch` is the refuse word. The feed decision stays
            // allow | deny | deny-default, same as hop audit.
            let decision = estate_schema::coverage_word_for_reason(false, &gate.line);
            let noted = format!("{} ({})", gate.line, gate.word);
            record_proxy_gate(state_dir, hop_id, agent, capability, decision, &noted, None)?;
            bail!("refuse:hop-coverage: {} ({})", gate.line, gate.word)
        }
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
    refuse_named_intentions(
        estate_path,
        state_dir,
        id,
        capability,
        agents,
        parsed_kind,
        true,
    )?;
    if agents.is_empty() {
        refuse_convey_coverage(estate_path, state_dir, id, None, capability)?;
    } else {
        for agent in agents {
            refuse_convey_coverage(estate_path, state_dir, id, Some(agent.as_str()), capability)?;
        }
    }
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
    let lease = declare_hop_covering(
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
        Some(&estate),
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
            state_dir,
            id,
            capability,
            &[agent.to_string()],
            parsed_kind,
            false,
        )?;
    }
    refuse_convey_coverage(estate_path, state_dir, id, agent, capability)?;
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

pub(crate) fn cmd_convey_list(estate_path: &Path, state_dir: &Path) -> Result<()> {
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
    // After the estate loads, before the hop decl list. Same stack as
    // estate leases, convey leases, audits, and history. Hop cites do not
    // change this command's exit code. The stack reads placement-actual for
    // mesh interpretation and Authority. A placement-actual SKU omits
    // Authority. `list_hops` then loads the interpreted mesh, which
    // slim-parses placement-actual, so that SKU still refuses before the
    // hop JSON. Audits and history do not take that second refuse.
    print!("{}", honesty_stack(&estate, state_dir)?);
    let hops = list_hops(state_dir)?;
    if hops.is_empty() {
        println!("no hops under {}", state_dir.display());
        return Ok(());
    }
    println!("{}", serde_json::to_string_pretty(&hops)?);
    Ok(())
}

pub(crate) fn cmd_convey_leases(estate_path: &Path, state_dir: &Path) -> Result<()> {
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
    // After the estate loads, before the hop lease list. Same stack as
    // estate leases, status, doctor, reconcile, and audit export. Hop cites
    // do not change this command's exit code. A placement-actual SKU still
    // reaches the hop lease reader below, which refuses before the JSON.
    print!("{}", honesty_stack(&estate, state_dir)?);
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

pub(crate) fn cmd_convey_sync(state_dir: &Path, estate_path: &Path) -> Result<()> {
    // A missing estate keeps the placement-kind restamp. A loaded estate
    // refuses a placement hop whose stamped capability disagrees with
    // placement-derived coverage and writes nothing. Deny stays deny.
    let mesh = if estate_path.is_file() {
        let estate =
            load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
        sync_from_placements_covering(state_dir, Some(&estate))?
    } else {
        sync_from_placements(state_dir)?
    };
    println!("{}", serde_json::to_string_pretty(&mesh)?);
    Ok(())
}

/// File check. Same Agents section as plan, drift, apply, status, and doctor
/// (`describe_agents_section`), then the same hop coverage cites those
/// commands print (`print_hop_coverage_cites`), then the same Authority
/// section. A capability mismatch prints `FAIL`. Deny and deny-default print
/// `note`. Those cites do not fail this command. A match stays quiet. A
/// missing mesh is an empty cite list. Does not write the mesh, the leases,
/// the estate, or the apply audit. Does not spawn. Does not claim mediation.
/// Not an identity lookup. A missing or unreadable estate refuses before any
/// section. A present mesh that does not parse refuses before any section and
/// does not invent cites, Agents, or Authority rows. A would-deny row does
/// not fail this command.
pub(crate) fn cmd_convey_authority(state_dir: &Path, estate_path: &Path) -> Result<()> {
    let estate = estate_schema::load_estate(estate_path)
        .with_context(|| format!("load {}", estate_path.display()))?;
    // Read first. A mesh that does not parse, including a bad host class,
    // refuses before any print and does not invent cites, Agents, or
    // Authority rows. A missing mesh stays not-enforced inside the Authority
    // section. `load_mesh` then feeds the shared cite printer: a missing
    // file is the empty mesh, so the cite list is empty.
    let rows = authority_report(state_dir, &estate)?;
    let mesh = load_mesh(state_dir)?;
    println!("{}", describe_agents_section(&estate));
    // Same lines doctor and status print. Mismatch is `FAIL`. Deny and
    // deny-default are `note`. Discard the mismatch lines. This command
    // does not bail on them. Does not write. Does not spawn.
    let _mismatches = crate::watch::print_hop_coverage_cites(&estate, &mesh);
    println!("{}", describe_authority_section(&rows, state_dir));
    Ok(())
}

pub(crate) fn cmd_convey_expire(estate_path: &Path, state_dir: &Path, forget: bool) -> Result<()> {
    let estate =
        load_estate(estate_path).with_context(|| format!("load {}", estate_path.display()))?;
    // After the estate loads, before the expired hop lease list and before
    // `--forget` writes. Same stack as convey list and convey leases. Hop
    // cites do not change this command's exit code. The stack reads
    // placement-actual for mesh interpretation and Authority. A
    // placement-actual SKU omits Authority. `list_expired_hop_leases` then
    // loads the interpreted mesh, which slim-parses placement-actual, so
    // that SKU still refuses before the expired list and before `--forget`
    // rewrites the mesh. An expired spawned cloud hop still refuses there
    // too. Audits and history do not take that second refuse.
    print!("{}", honesty_stack(&estate, state_dir)?);
    let expired = list_expired_hop_leases(state_dir, hop_now_unix())?;
    if expired.is_empty() {
        println!("no expired hop leases under {}", state_dir.display());
        // `--forget` does not rewrite when nothing is expired.
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

/// Local review bundle. After the estate loads, the honesty snapshot is
/// built before `write_reconcile` and before `out` is wiped or written.
/// `MANIFEST.md` names `honesty.md`. `conveyor-mesh.json` is copied when
/// that file is present. A mesh that does not parse, a bad `host_class` on
/// that file, or `refuse:agent-unplaced` returns before any of those writes
/// (empty stdout, no partial export). A placement-actual SKU `host_class`
/// still exports. Does not spawn. Does not rewrite leases, the estate, the
/// apply audit, or the live mesh beyond `write_reconcile`.
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
    // Refuse paths return here: no reconcile refresh, no out-dir wipe, no
    // honesty file, no tar. Cites inside a successful snapshot do not fail
    // this command.
    let honesty = honesty_stack(&estate, state_dir)?;
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
            state_dir.join("conveyor-mesh.json"),
            out.join("conveyor-mesh.json"),
        ),
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
    manifest.push_str(&format!("refuses: {}\n", report.refuses.len()));
    manifest.push_str(&format!("honesty: {AUDIT_HONESTY_FILE}\n\n"));
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
    // Next to MANIFEST.md. Named by the `honesty:` line above. Written
    // before the manifest and before tar, so the bundle and the archive
    // both hold the snapshot.
    std::fs::write(out.join(AUDIT_HONESTY_FILE), &honesty)?;
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

const AUDIT_HONESTY_FILE: &str = "honesty.md";

/// Agents, hop coverage cites, then Authority.
///
/// Shared by `estate leases` (printed before the placement list),
/// `estate convey leases` (printed before the hop lease list),
/// `estate convey list` (printed before the hop decl list),
/// `estate convey expire` (printed before the expired hop lease list and
/// before `--forget` writes),
/// `estate audits` (printed before the apply-audit list),
/// `estate history` (printed before the lifecycle history list), and
/// `estate audit export` (`honesty.md`). A present mesh that does not parse,
/// or a bad `host_class` on that file, refuses before any section. Callers
/// do not invent cites, Agents, or Authority rows. A missing mesh is the
/// empty mesh from `load_mesh`: the cite list is empty and Authority stays
/// `not-enforced` (`missing-mesh`).
///
/// `authority_report` reads placement-actual for mesh interpretation and
/// Authority (`load_interpreted_mesh` then `slim_parse_placement_actual`).
/// Mesh host classes are already refused above, so the only
/// `MeshError::BadHostClass` that reaches the match is the placement-actual
/// slim-parse (a SKU or other bad `host_class`). That one error continues:
/// this text still includes Agents and hop cites from `load_mesh` and omits
/// Authority rows. `estate leases` then still refuses that SKU before the
/// placement JSON. `estate convey leases` then still refuses that SKU before
/// the hop lease JSON. `estate convey list` then still refuses that SKU
/// before the hop JSON, because `list_hops` calls `load_interpreted_mesh`.
/// `estate convey expire` then still refuses that SKU before the expired
/// list, because `list_expired_hop_leases` calls `load_interpreted_mesh`.
/// `--forget` is not reached, so that SKU does not rewrite the mesh.
/// `estate audits` then still prints the apply-audit list. `estate history`
/// then still prints the lifecycle history list. There is no second
/// placement refuse before those two lists. Every other
/// mesh error, including a population ahead of the floor
/// (`refuse:agent-unplaced`) and a placement-actual parse failure
/// (`MeshError::Parse`), refuses here before any section. Capability
/// mismatch is `FAIL`. Deny and deny-default are `note`. A match stays
/// quiet. Those cites do not fail the caller.
/// Does not rewrite the mesh, the leases, the estate, or the apply audit.
/// Does not spawn. No `enforced` status.
fn honesty_stack(estate: &estate_schema::Estate, state_dir: &Path) -> Result<String> {
    let mesh = load_mesh(state_dir)?;
    refuse_mesh_host_classes(&mesh)?;
    let authority = match authority_report(state_dir, estate) {
        Ok(rows) => Some(describe_authority_section(&rows, state_dir)),
        Err(MeshError::BadHostClass(_)) => None,
        Err(err) => return Err(err.into()),
    };
    let mut text = format!("{}\n", describe_agents_section(estate));
    let (cites, _mismatches) = crate::watch::render_hop_coverage_cites(estate, &mesh);
    text.push_str(&cites);
    if let Some(section) = authority {
        text.push_str(&section);
        text.push('\n');
    }
    Ok(text)
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
    println!("agent call:\n{}", describe_agent_edge_coverage(&estate));
    println!("intention:\n{}", describe_intention_coverage(&estate));
    println!("hop:\n{}", describe_hop_coverage(&estate));
    // Missing mesh is an empty cite list, not a failure. A present file
    // that does not parse stays the mesh error and is not rewritten.
    // Floor and models JSON above are already printed and are not edited.
    let mut hop_mismatch: Option<String> = None;
    match load_mesh(state_dir) {
        Ok(mesh) => {
            for cite in crate::watch::hop_coverage_cites(&estate, &mesh) {
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
    // After the Agents section and the coverage lines, including hop
    // refuse. Same text as `estate plan` and `estate convey authority`.
    // Print-only. Drift detected and a hop mismatch still bail below.
    // An authority read error does not hide those. Does not write the
    // mesh, the leases, or the estate, and does not invent a lease.
    let authority_text = drift_authority_text(&estate, state_dir);
    if let Ok(text) = &authority_text {
        println!("{text}");
    }
    if !report.in_sync || !models.in_sync {
        bail!("drift detected");
    }
    if let Some(line) = hop_mismatch {
        bail!(line);
    }
    authority_text?;
    Ok(())
}

/// File check printed after hop coverage cites. Same text as `estate plan`
/// and `estate convey authority`. Does not write the mesh, the leases, or
/// the estate. Does not invent a lease.
fn drift_authority_text(estate: &estate_schema::Estate, state_dir: &Path) -> Result<String> {
    let rows = authority_report(state_dir, estate).map_err(|err| anyhow::anyhow!("{err}"))?;
    Ok(describe_authority_section(&rows, state_dir))
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

    fn state_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cell-convey-audit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_estate_refuses_convey_coverage() {
        let path =
            std::env::temp_dir().join(format!("cell-missing-estate-{}.yaml", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let state = state_dir("missing");
        let err = refuse_convey_coverage(&path, &state, "ttl-box", None, "lane-tool").unwrap_err();
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
        let state = state_dir("non-file");
        let err = refuse_convey_coverage(&path, &state, "ttl-box", None, "lane-tool").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("refuse:hop-coverage"), "{msg}");
        assert!(msg.contains("estate not a file"), "{msg}");
        assert!(msg.contains("coverage is mandatory"), "{msg}");
        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn loaded_non_placement_stays_lease_stub() {
        let estate = repo_root().join("examples/estate.yaml");
        let state = state_dir("stub");
        refuse_convey_coverage(
            &estate,
            &state,
            "ttl-box",
            None,
            "not-a-placement-capability",
        )
        .unwrap();
        assert!(!state.join("feed/events.jsonl").exists());
    }

    #[test]
    fn loaded_example_box_still_deny_default() {
        let estate = repo_root().join("examples/estate.yaml");
        let state = state_dir("box-deny");
        let msg = refuse_convey_coverage(&estate, &state, "cell-one-box", None, "not-lane-tool")
            .unwrap_err()
            .to_string();
        assert!(msg.contains("refuse:hop-coverage"), "{msg}");
        assert!(msg.contains("(deny-default)"), "{msg}");
        let lines = feed_collector::proxy_audit_events(&state.join("feed")).unwrap();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines[0].decision.as_deref(), Some("deny-default"));
        assert_eq!(lines[0].kind, "proxy.hop");
        let cursor = feed_collector::load_cursor(&state.join("feed"))
            .unwrap()
            .expect("cursor");
        assert_eq!(cursor.events, 1);
        assert!(cursor.packed_id.is_none());
    }

    #[test]
    fn loaded_example_cloud_still_deny() {
        let estate = repo_root().join("examples/estate.yaml");
        let state = state_dir("cloud-deny");
        let msg = refuse_convey_coverage(&estate, &state, "cursor-cloud", None, "lane-tool")
            .unwrap_err()
            .to_string();
        assert!(
            msg.contains("refuse:hop-coverage") && msg.contains("(deny)"),
            "{msg}"
        );
        assert!(!msg.contains("deny-default"), "{msg}");
        let lines = feed_collector::proxy_audit_events(&state.join("feed")).unwrap();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines[0].decision.as_deref(), Some("deny"));
        assert_eq!(lines[0].kind, "proxy.hop");
    }

    #[test]
    fn capability_mismatch_audits_deny_and_keeps_mismatch_in_the_message() {
        let mut estate =
            estate_schema::load_estate(&repo_root().join("examples/estate.yaml")).unwrap();
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
        let yaml = estate_schema::render_estate_yaml(&estate).unwrap();
        let path = std::env::temp_dir().join(format!(
            "cell-mismatch-estate-{}.yaml",
            std::process::id()
        ));
        std::fs::write(&path, yaml).unwrap();
        let state = state_dir("mismatch");
        refuse_convey_coverage(&path, &state, "cell-one-box", Some("research"), "lane-tool")
            .unwrap();
        assert!(!state.join("feed/events.jsonl").exists());
        let msg = refuse_convey_coverage(
            &path,
            &state,
            "cell-one-box",
            Some("research"),
            "notes-append",
        )
        .unwrap_err()
        .to_string();
        assert!(msg.contains("refuse:hop-coverage"), "{msg}");
        assert!(msg.contains("(mismatch)"), "{msg}");
        assert!(msg.contains("does not match"), "{msg}");
        let lines = feed_collector::proxy_audit_events(&state.join("feed")).unwrap();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines[0].decision.as_deref(), Some("deny"));
        assert_eq!(lines[0].kind, "proxy.hop");
        let note = lines[0].note.as_deref().unwrap_or("");
        assert!(note.contains("does not match"), "{note}");
        assert!(note.contains("(mismatch)"), "{note}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn agent_intention_refuse_audits_proxy_agent() {
        let mut estate =
            estate_schema::load_estate(&repo_root().join("examples/estate.yaml")).unwrap();
        estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == "horizon")
            .unwrap()
            .calls
            .push(estate_schema::CallDecl {
                id: "research".into(),
                description: None,
            });
        let yaml = estate_schema::render_estate_yaml(&estate).unwrap();
        let path = std::env::temp_dir().join(format!(
            "cell-agent-feed-estate-{}.yaml",
            std::process::id()
        ));
        std::fs::write(&path, yaml).unwrap();
        let state = state_dir("agent-feed");
        let err = super::refuse_named_intentions(
            &path,
            &state,
            "peer-hop",
            "research",
            &["horizon".to_string()],
            Some(estate_schema::IntentionKind::Agent),
            false,
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("refuse:intention"), "{msg}");
        assert!(msg.contains("deny-default"), "{msg}");
        let lines = feed_collector::proxy_audit_events(&state.join("feed")).unwrap();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines[0].kind, "proxy.agent");
        assert_eq!(lines[0].decision.as_deref(), Some("deny-default"));
        assert_eq!(lines[0].object_class.as_deref(), Some("agent"));
        assert_eq!(lines[0].agent_id.as_deref(), Some("horizon"));
        let blocked = state_dir("agent-feed-blocked");
        std::fs::write(blocked.join("feed"), "not-a-dir").unwrap();
        let err = super::refuse_named_intentions(
            &path,
            &blocked,
            "peer-hop",
            "agent:research",
            &["horizon".to_string()],
            Some(estate_schema::IntentionKind::Agent),
            true,
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:proxy-audit"), "{err}");
        let _ = std::fs::remove_file(&path);
    }
}

#[cfg(test)]
mod drift_hop_coverage_tests {
    use super::cmd_drift;
    use crate::watch::hop_coverage_cites;
    use conveyor_proxy::{persist_mesh, ConveyorMesh, HopDecl, HopLease, MESH_FILE};
    use estate_schema::load_estate;
    use floor_supervisor::{apply_with_profile_dir, drift_with_roots};
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
        let dir = std::env::temp_dir().join(format!("cell-drift-hop-{nanos}"));
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

    fn synced_example(dir: &Path) -> (PathBuf, PathBuf) {
        let estate_path = dir.join("estate.yaml");
        fs::copy(repo_root().join("examples/estate.yaml"), &estate_path).unwrap();
        let state = dir.join("state");
        let roots = dir.join("roots");
        fs::create_dir_all(&roots).unwrap();
        let estate = load_estate(&estate_path).unwrap();
        apply_with_profile_dir(&estate, &state, &roots).unwrap();
        model_estate::record_bindings(&estate, &state).unwrap();
        assert!(
            drift_with_roots(&estate, &state, Some(&roots))
                .unwrap()
                .in_sync
        );
        assert!(
            model_estate::drift_bindings(&estate, &state)
                .unwrap()
                .in_sync
        );
        (estate_path, state)
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
        let estate = load_estate(&path).unwrap();
        let state = dir.join("state");
        let roots = dir.join("roots");
        apply_with_profile_dir(&estate, &state, &roots).unwrap();
        model_estate::record_bindings(&estate, &state).unwrap();
        path
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
    fn drift_cites_mismatch_and_stays_quiet_on_a_match() {
        let dir = scratch();
        let estate_path = allow_copy(&dir);
        let state = dir.join("state");
        let roots = dir.join("roots");
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
        let err = cmd_drift(&estate_path, &state, &roots)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert!(!err.contains("drift detected"), "{err}");
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);
        let floor = drift_with_roots(&estate, &state, Some(&roots)).unwrap();
        let models = model_estate::drift_bindings(&estate, &state).unwrap();
        assert!(floor.in_sync, "{floor:?}");
        assert!(models.in_sync, "{models:?}");

        let matched = mesh_with(
            vec![box_lease("cell-one-box", "lane-tool", &["research"])],
            vec![],
        );
        persist_mesh(&state, &matched).unwrap();
        assert!(hop_coverage_cites(&estate, &matched).is_empty());
        cmd_drift(&estate_path, &state, &roots).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn drift_cites_deny_and_deny_default_without_failing_alone() {
        let dir = scratch();
        let (estate_path, state) = synced_example(&dir);
        let roots = dir.join("roots");
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
        cmd_drift(&estate_path, &state, &roots).unwrap();
        assert_eq!(snapshot(&state), before);
        assert!(
            drift_with_roots(&estate, &state, Some(&roots))
                .unwrap()
                .in_sync
        );
        assert!(
            model_estate::drift_bindings(&estate, &state)
                .unwrap()
                .in_sync
        );

        let mut denied = estate.clone();
        denied
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap()
            .tools
            .push(estate_schema::ToolDecl {
                id: "lane-tool".into(),
                description: None,
            });
        denied.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "lane-tool".into(),
            kind: estate_schema::IntentionKind::Tool,
            effect: estate_schema::Effect::Deny,
            note: None,
        });
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
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn drift_does_not_cite_a_cloud_kind_variant() {
        let dir = scratch();
        let estate_path = allow_copy(&dir);
        let state = dir.join("state");
        let roots = dir.join("roots");
        let estate = load_estate(&estate_path).unwrap();
        for kind in ["cloud_mesh", " Cloud-Mesh ", "CLOUD-AGENT"] {
            let mut lease = box_lease("cell-one-box", "notes-append", &["research"]);
            lease.kind = kind.into();
            let leased = mesh_with(vec![lease], vec![]);
            assert!(hop_coverage_cites(&estate, &leased).is_empty(), "{kind}");
            persist_mesh(&state, &leased).unwrap();
            cmd_drift(&estate_path, &state, &roots).unwrap();

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
            cmd_drift(&estate_path, &state, &roots).unwrap();
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn drift_cites_a_hop_declaration_when_no_lease_is_present() {
        let dir = scratch();
        let estate_path = allow_copy(&dir);
        let state = dir.join("state");
        let roots = dir.join("roots");
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
        let err = cmd_drift(&estate_path, &state, &roots)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn example_estate_drift_stays_green_without_a_mesh() {
        let dir = scratch();
        let (estate_path, state) = synced_example(&dir);
        let roots = dir.join("roots");
        cmd_drift(&estate_path, &state, &roots).unwrap();
        let locked = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
        assert_eq!(fs::read(&estate_path).unwrap(), locked);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn drift_fails_closed_when_the_mesh_file_does_not_parse() {
        let dir = scratch();
        let (estate_path, state) = synced_example(&dir);
        let roots = dir.join("roots");
        let mesh_path = state.join(MESH_FILE);
        let corrupt = b"{not-json";
        fs::write(&mesh_path, corrupt).unwrap();
        let before = snapshot(&state);
        let estate_bytes = fs::read(&estate_path).unwrap();
        let err = cmd_drift(&estate_path, &state, &roots)
            .unwrap_err()
            .to_string();
        assert!(err.contains("parse") && err.contains(MESH_FILE), "{err}");
        assert!(!err.contains("drift detected"), "{err}");
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&mesh_path).unwrap(), corrupt);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);
        let estate = load_estate(&estate_path).unwrap();
        assert!(
            drift_with_roots(&estate, &state, Some(&roots))
                .unwrap()
                .in_sync
        );
        assert!(
            model_estate::drift_bindings(&estate, &state)
                .unwrap()
                .in_sync
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod drift_authority_section_tests {
    use super::{cmd_drift, drift_authority_text};
    use conveyor_proxy::{persist_mesh, ConveyorMesh, HopLease, MESH_FILE};
    use estate_schema::load_estate;
    use floor_supervisor::apply_with_profile_dir;
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
        let dir = std::env::temp_dir().join(format!("cell-drift-authority-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn synced_example(dir: &Path) -> (PathBuf, PathBuf) {
        let estate_path = dir.join("estate.yaml");
        fs::copy(repo_root().join("examples/estate.yaml"), &estate_path).unwrap();
        let state = dir.join("state");
        let roots = dir.join("roots");
        let estate = load_estate(&estate_path).unwrap();
        apply_with_profile_dir(&estate, &state, &roots).unwrap();
        model_estate::record_bindings(&estate, &state).unwrap();
        (estate_path, state)
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

    fn no_enforced_status_token(text: &str) -> bool {
        !text.split_whitespace().any(|word| {
            let token = word.trim_matches(|c: char| c == ':' || c == ',' || c == '.' || c == ';');
            token == "enforced"
        })
    }

    fn granted_box(capability: &str) -> ConveyorMesh {
        ConveyorMesh {
            schema: conveyor_proxy::MESH_SCHEMA.into(),
            hops: vec![],
            leases: vec![HopLease {
                hop_id: "cell-one-box".into(),
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
                agents: vec!["research".into()],
            }],
        }
    }

    #[test]
    fn authority_section_is_not_enforced_when_no_mesh_exists() {
        let dir = scratch();
        let (estate_path, state) = synced_example(&dir);
        let roots = dir.join("roots");
        assert!(!state.join(MESH_FILE).exists());
        let estate = load_estate(&estate_path).unwrap();
        let section = drift_authority_text(&estate, &state).unwrap();
        assert!(section.starts_with("Authority\n---------\n"), "{section}");
        assert!(
            section.contains("authority would-allow=0 would-deny=0 not-enforced="),
            "{section}"
        );
        assert!(
            section.contains("not-enforced reasons: missing-mesh=5 cloud=1"),
            "{section}"
        );
        assert!(
            !section.contains("would-deny=0 not-enforced=0"),
            "{section}"
        );
        assert!(
            section.contains("research notes-append cell-one-box: not-enforced --"),
            "{section}"
        );
        assert!(
            section.contains("conveyor-mesh.json is absent"),
            "{section}"
        );
        assert!(
            !section.contains("no hop lease names this capability"),
            "{section}"
        );
        assert!(
            section.contains("cursor-cloud") && section.contains("not spawned"),
            "{section}"
        );
        assert!(no_enforced_status_token(&section), "{section}");
        assert!(
            section.contains(
                "uncertain: a hop lease is a file. This report does not show that a worker called the conveyor."
            ),
            "{section}"
        );
        let before = snapshot(&state);
        let estate_bytes = fs::read(&estate_path).unwrap();
        cmd_drift(&estate_path, &state, &roots).unwrap();
        assert_eq!(snapshot(&state), before);
        assert!(!state.join(MESH_FILE).exists());
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);
        let locked = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
        assert_eq!(fs::read(&estate_path).unwrap(), locked);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn authority_section_would_deny_when_hop_coverage_denies_a_granted_box_lease() {
        let dir = scratch();
        let estate_path = dir.join("deny.yaml");
        let mut text = fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
        text = text.replace(
            "      - id: notes-append\n        description: Append a note inside the Research lane\n",
            "      - id: notes-append\n        description: Append a note inside the Research lane\n      - id: lane-tool\n",
        );
        text = text.replace(
            "intentions: []\n",
            "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: deny\n",
        );
        fs::write(&estate_path, &text).unwrap();
        let estate = load_estate(&estate_path).unwrap();
        let state = dir.join("state");
        let roots = dir.join("roots");
        apply_with_profile_dir(&estate, &state, &roots).unwrap();
        model_estate::record_bindings(&estate, &state).unwrap();
        persist_mesh(&state, &granted_box("lane-tool")).unwrap();
        let section = drift_authority_text(&estate, &state).unwrap();
        assert!(
            section.contains("authority would-allow=0 would-deny="),
            "{section}"
        );
        let denied = section
            .lines()
            .find(|line| line.contains("research lane-tool cell-one-box:"))
            .unwrap_or_else(|| panic!("missing deny row\n{section}"));
        assert!(
            denied.contains(": would-deny --")
                && denied.contains("refuse:hop-coverage")
                && denied.contains("(deny).")
                && !denied.contains("deny-default")
                && !denied.contains("(mismatch)")
                && denied.contains("Not mediated"),
            "{denied}"
        );
        assert!(no_enforced_status_token(&section), "{section}");
        let before = snapshot(&state);
        let estate_bytes = fs::read(&estate_path).unwrap();
        cmd_drift(&estate_path, &state, &roots).unwrap();
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&estate_path).unwrap(), estate_bytes);
        assert!(
            floor_supervisor::drift_with_roots(&estate, &state, Some(&roots))
                .unwrap()
                .in_sync
        );
        assert!(
            model_estate::drift_bindings(&estate, &state)
                .unwrap()
                .in_sync
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hop_mismatch_still_refuses_and_the_section_never_prints_enforced() {
        let dir = scratch();
        let mut text = fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
        text = text.replace(
            "      - id: notes-append\n        description: Append a note inside the Research lane\n",
            "      - id: notes-append\n        description: Append a note inside the Research lane\n      - id: lane-tool\n",
        );
        text = text.replace(
            "intentions: []\n",
            "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        );
        let mismatch_path = dir.join("mismatch.yaml");
        fs::write(&mismatch_path, &text).unwrap();
        let estate = load_estate(&mismatch_path).unwrap();
        let state = dir.join("state");
        let roots = dir.join("roots");
        apply_with_profile_dir(&estate, &state, &roots).unwrap();
        model_estate::record_bindings(&estate, &state).unwrap();
        persist_mesh(&state, &granted_box("notes-append")).unwrap();
        let section = drift_authority_text(&estate, &state).unwrap();
        assert!(
            section.contains("research notes-append cell-one-box: would-deny --"),
            "{section}"
        );
        assert!(
            section.contains("refuse:hop-coverage") && section.contains("(mismatch)"),
            "{section}"
        );
        assert!(no_enforced_status_token(&section), "{section}");
        let before = snapshot(&state);
        let estate_bytes = fs::read(&mismatch_path).unwrap();
        let err = cmd_drift(&mismatch_path, &state, &roots)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("refuse:hop-coverage") && err.contains("(mismatch)"),
            "{err}"
        );
        assert!(!err.contains("drift detected"), "{err}");
        assert!(no_enforced_status_token(&err), "{err}");
        assert_eq!(snapshot(&state), before);
        assert_eq!(fs::read(&mismatch_path).unwrap(), estate_bytes);
        assert!(
            floor_supervisor::drift_with_roots(&estate, &state, Some(&roots))
                .unwrap()
                .in_sync
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
