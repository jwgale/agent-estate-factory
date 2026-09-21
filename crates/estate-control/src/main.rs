use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use estate_schema::{
    covering_plan, describe, describe_placements, diff_estates, estate_hash,
    list_plans, load_estate, load_estate_unvalidated, plan_against_is_fresh, render_plan, validate,
    write_plan,
};
use feed_collector::{
    import_pack, list_drop_packs, load_cursor, materialize_from_feed, refuse_promote,
    write_pack_index,
};
use floor_supervisor::{
    append_apply_audit, apply_with_profile_dir, drift_with_roots, list_apply_audits,
    list_lifecycle_events, load_desired_snapshot, load_lifecycle, load_placements, mark_running,
    record_placements, resume, suspend, ApplyAudit,
};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "estate",
    about = "Cell One estate-control: validate, plan, apply, drift. Does not execute tools or models."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fail closed if the estate file is invalid.
    Validate {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    /// Human-readable blast-radius plan; append-only write to plans/.
    Plan {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        /// Previous estate YAML to diff against. Default: last apply snapshot, else greenfield.
        #[arg(long)]
        against: Option<PathBuf>,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Converge isolation (Control→Data apply seam). Stretch: included, thin.
    Apply {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        /// Fail if no plan on disk covers this estate hash (gated apply).
        #[arg(long, default_value_t = false)]
        require_plan: bool,
        /// Explicit pack import during apply (never silent; does not edit estate.yaml).
        #[arg(long)]
        import_pack: Option<String>,
        #[arg(long, default_value = "packs")]
        packs_dir: PathBuf,
        /// Fail if covering plan against_hash does not match last apply.
        #[arg(long, default_value_t = false)]
        require_fresh_plan: bool,
    },
    /// Compare desired estate to regenerable actual-state.
    Drift {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
    /// List equal-class bindings. Does not invoke them.
    Models {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    /// Append-only plan history (human control surface).
    Plans {
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
    },
    /// Drop runtime; write durable lifecycle=suspended. Lane roots stay.
    Suspend {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Re-apply from estate files; lifecycle=running.
    Resume {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
    /// Persisted estate + durable lifecycle + disposable runtime.
    Status {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
    /// Feed plane: materialize packs, list drop zone, refuse promote.
    Feed {
        #[command(subcommand)]
        command: FeedCommand,
    },
    /// Dump portable local catalog (file SoT). Does not invoke models.
    Catalog {
        #[arg(long, default_value = ".cell/catalog.json")]
        out: PathBuf,
    },
    /// Print durable placement leases. Cloud-agent must stay unspawned.
    Leases {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Print apply-audit.jsonl (gated apply history).
    Audits {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Print append-only lifecycle.jsonl (suspend/resume/apply).
    History {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Catalog-level driver probes. Not live pings. Does not invoke models.
    Probes,
}

#[derive(Subcommand)]
enum FeedCommand {
    /// Read scrubbed events and write a candidate pack (not in the estate).
    Pack {
        #[arg(long, default_value = ".cell/feed")]
        feed_dir: PathBuf,
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
        #[arg(long, default_value = "overnight-traces")]
        id: String,
    },
    /// List candidate packs in the drop zone.
    List {
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
    },
    /// Explicit apply of a pack artifact. Does not rewrite the estate file.
    Import {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
        #[arg(long, default_value = "packs/accepted")]
        accepted_dir: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    /// Always fails. Auto-promote is locked off.
    Promote {
        #[arg(long)]
        id: String,
    },
    /// Print durable feed-cursor.json watermark.
    Cursor {
        #[arg(long, default_value = ".cell/feed")]
        feed_dir: PathBuf,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { estate } => cmd_validate(&estate),
        Command::Plan {
            estate,
            against,
            plans_dir,
            state_dir,
        } => cmd_plan(&estate, against.as_deref(), &plans_dir, &state_dir),
        Command::Apply {
            estate,
            state_dir,
            roots_base,
            plans_dir,
            require_plan,
            import_pack,
            packs_dir,
            require_fresh_plan,
        } => cmd_apply(
            &estate,
            &state_dir,
            &roots_base,
            &plans_dir,
            require_plan,
            import_pack.as_deref(),
            &packs_dir,
            require_fresh_plan,
        ),
        Command::Drift {
            estate,
            state_dir,
            roots_base,
        } => cmd_drift(&estate, &state_dir, &roots_base),
        Command::Models { estate } => cmd_models(&estate),
        Command::Plans { plans_dir } => cmd_plans(&plans_dir),
        Command::Suspend { state_dir } => cmd_suspend(&state_dir),
        Command::Resume {
            estate,
            state_dir,
            roots_base,
        } => cmd_resume(&estate, &state_dir, &roots_base),
        Command::Status {
            estate,
            state_dir,
            roots_base,
        } => cmd_status(&estate, &state_dir, &roots_base),
        Command::Feed { command } => match command {
            FeedCommand::Pack {
                feed_dir,
                drop_dir,
                id,
            } => cmd_feed_pack(&feed_dir, &drop_dir, &id),
            FeedCommand::List { drop_dir } => cmd_feed_list(&drop_dir),
            FeedCommand::Import {
                id,
                drop_dir,
                accepted_dir,
                estate,
            } => cmd_feed_import(&id, &drop_dir, &accepted_dir, &estate),
            FeedCommand::Promote { id } => cmd_feed_promote(&id),
            FeedCommand::Cursor { feed_dir } => cmd_feed_cursor(&feed_dir),
        },
        Command::Catalog { out } => cmd_catalog(&out),
        Command::Leases { state_dir } => cmd_leases(&state_dir),
        Command::Audits { state_dir } => cmd_audits(&state_dir),
        Command::History { state_dir } => cmd_history(&state_dir),
        Command::Probes => cmd_probes(),
    }
}

fn cmd_validate(path: &Path) -> Result<()> {
    let estate = match load_estate_unvalidated(path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("invalid estate (fail closed):");
            for line in e.errors() {
                eprintln!("  - {line}");
            }
            bail!("validate failed for {}", path.display());
        }
    };
    match validate(&estate) {
        Ok(()) => {
            println!("estate: {}", path.display());
            println!("{}", describe(&estate));
            println!("ok");
            Ok(())
        }
        Err(errors) => {
            eprintln!("invalid estate (fail closed):");
            for err in errors {
                eprintln!("  - {err}");
            }
            bail!("validate failed for {}", path.display());
        }
    }
}

fn cmd_plan(path: &Path, against: Option<&Path>, plans_dir: &Path, state_dir: &Path) -> Result<()> {
    let desired = load_estate(path).with_context(|| format!("desired {}", path.display()))?;
    let previous = match against {
        Some(p) => Some(load_estate(p).with_context(|| format!("against {}", p.display()))?),
        None => load_desired_snapshot(state_dir)?,
    };
    let plan = diff_estates(&desired, previous.as_ref());
    let written = write_plan(plans_dir, &plan)?;
    print!("{}", render_plan(&plan));
    println!("Wrote {}", written.display());
    Ok(())
}

fn cmd_apply(
    path: &Path,
    state_dir: &Path,
    roots_base: &Path,
    plans_dir: &Path,
    require_plan: bool,
    import_pack_id: Option<&str>,
    packs_dir: &Path,
    require_fresh_plan: bool,
) -> Result<()> {
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    let hash = estate_hash(&estate);
    let covering = covering_plan(plans_dir, &hash);
    let covering_stem = covering.as_ref().map(|c| c.stem.clone());
    if (require_plan || require_fresh_plan) && covering.is_none() {
        bail!("apply gated: no plan on disk for {hash}; run estate plan first");
    }
    let last_applied = load_desired_snapshot(state_dir)?
        .as_ref()
        .map(estate_hash);
    let fresh = covering
        .as_ref()
        .map(|c| plan_against_is_fresh(&c.plan, last_applied.as_deref()))
        .unwrap_or(true);
    if require_fresh_plan && !fresh {
        bail!(
            "apply gated: covering plan against_hash does not match last apply {}",
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
        let (rec, dest) = import_pack(packs_dir, &packs_dir.join("accepted"), id, &ids)?;
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

fn cmd_plans(plans_dir: &Path) -> Result<()> {
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

fn cmd_suspend(state_dir: &Path) -> Result<()> {
    let record = suspend(state_dir)?;
    println!("lifecycle: {}", record.state.as_str());
    println!("{}", record.note);
    Ok(())
}

fn cmd_resume(path: &Path, state_dir: &Path, roots_base: &Path) -> Result<()> {
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    let (actual, record) = resume(&estate, state_dir, roots_base)?;
    model_estate::record_bindings(&estate, state_dir)?;
    record_placements(&estate, state_dir)?;
    let _ = model_estate::write_catalog(&state_dir.join("catalog.json"));
    println!(
        "resumed {} sessions; lifecycle {}",
        actual.sessions.len(),
        record.state.as_str()
    );
    println!("{}", describe_placements(&estate));
    Ok(())
}

fn cmd_status(path: &Path, state_dir: &Path, roots_base: &Path) -> Result<()> {
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    let life = load_lifecycle(state_dir)?;
    let report = drift_with_roots(&estate, state_dir, Some(roots_base))?;
    println!("estate: {} ({})", estate.name, path.display());
    println!("lifecycle: {} (durable={})", life.state.as_str(), life.durable);
    println!("{}", life.note);
    println!("{}", describe_placements(&estate));
    if let Some(places) = load_placements(state_dir)? {
        println!("placement actual (durable, regenerable):");
        for lease in &places.leases {
            println!(
                "  {} kind={} spawned={} wired={}",
                lease.placement_id, lease.kind, lease.spawned, lease.wired
            );
        }
    }
    println!("floor in_sync: {}", report.in_sync);
    if !report.spawned_cloud_agents.is_empty() {
        bail!(
            "cloud-agent lease spawned (fail closed): {}",
            report.spawned_cloud_agents.join(", ")
        );
    }
    if !report.notes.is_empty() {
        for note in &report.notes {
            println!("  {note}");
        }
    }
    let history = list_lifecycle_events(state_dir)?;
    if !history.is_empty() {
        println!("lifecycle history: {}", history.len());
        if let Some(last) = history.last() {
            println!("  last {} -> {} ({})", last.action, last.to, last.ts);
        }
    }
    let audits = list_apply_audits(state_dir)?;
    if !audits.is_empty() {
        println!("apply audits: {}", audits.len());
        if let Some(last) = audits.last() {
            println!(
                "  last require_plan={} covering={} imported={}",
                last.require_plan,
                last.covering_plan.as_deref().unwrap_or("-"),
                last.imported_packs.len()
            );
        }
    }
    Ok(())
}

fn cmd_feed_pack(feed_dir: &Path, drop_dir: &Path, id: &str) -> Result<()> {
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

fn cmd_catalog(out: &Path) -> Result<()> {
    println!("{}", model_estate::render_catalog());
    let written = model_estate::write_catalog(out)?;
    println!("wrote catalog file {}", written.display());
    Ok(())
}

fn cmd_leases(state_dir: &Path) -> Result<()> {
    match load_placements(state_dir)? {
        None => {
            println!("no placement-actual.json under {}", state_dir.display());
            println!("run apply or resume to record leases");
            Ok(())
        }
        Some(places) => {
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

fn cmd_audits(state_dir: &Path) -> Result<()> {
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

fn cmd_feed_list(drop_dir: &Path) -> Result<()> {
    let packs = list_drop_packs(drop_dir)?;
    let _ = write_pack_index(drop_dir);
    if packs.is_empty() {
        println!("no candidate packs in {}", drop_dir.display());
        return Ok(());
    }
    for pack in packs {
        println!(
            "  {} events={} promoted={} policy={}/{}",
            pack.id, pack.from_events, pack.promoted, pack.curator, pack.policy
        );
    }
    Ok(())
}

fn cmd_feed_import(
    id: &str,
    drop_dir: &Path,
    accepted_dir: &Path,
    estate_path: &Path,
) -> Result<()> {
    let estate = load_estate(estate_path)
        .with_context(|| format!("load {}", estate_path.display()))?;
    let ids: Vec<String> = estate
        .enrich_packs
        .packs
        .iter()
        .map(|p| p.id.clone())
        .collect();
    let before = std::fs::read_to_string(estate_path).unwrap_or_default();
    let (rec, dest) = import_pack(drop_dir, accepted_dir, id, &ids)?;
    let after = std::fs::read_to_string(estate_path).unwrap_or_default();
    if before != after {
        bail!("import must not rewrite the estate file");
    }
    println!(
        "imported pack {} schema={} estate_bound={} promoted={} -> {}",
        rec.pack.id,
        rec.pack.schema,
        rec.estate_bound,
        rec.pack.promoted,
        dest.display()
    );
    println!("estate file unchanged. Jason still lists pack ids on enrich_packs by hand.");
    Ok(())
}

fn cmd_feed_promote(id: &str) -> Result<()> {
    match refuse_promote(id) {
        Ok(()) => unreachable!("promote has no success path"),
        Err(err) => bail!("{err}"),
    }
}

fn cmd_feed_cursor(feed_dir: &Path) -> Result<()> {
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

fn cmd_history(state_dir: &Path) -> Result<()> {
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

fn cmd_probes() -> Result<()> {
    let probes = model_estate::catalog_probes();
    for probe in probes {
        println!(
            "  {:<12} status={:<12} bindable={} live_probed={} host_class={}",
            probe.driver, probe.status, probe.bindable, probe.live_probed, probe.host_class
        );
        if !probe.live_probed {
            println!("    {}", probe.note);
        }
    }
    Ok(())
}

fn chrono_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

fn cmd_drift(path: &Path, state_dir: &Path, roots_base: &Path) -> Result<()> {
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

fn cmd_models(path: &Path) -> Result<()> {
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
