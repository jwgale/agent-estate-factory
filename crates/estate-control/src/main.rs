use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use conveyor_proxy::{
    call_hop, declare_hop, list_hop_leases, list_hops, sync_from_placements, HopDecl,
};
use estate_schema::{
    covering_plan, describe, describe_placements, diff_estates, estate_hash, list_plans,
    load_estate, load_estate_unvalidated, mark_plan_reviewed, plan_against_is_fresh,
    plan_against_is_fresh_strict, plan_is_reviewable, render_plan, validate, write_plan,
};
use feed_collector::{
    import_pack, list_drop_packs, load_cursor, materialize_from_feed, propose_enrich,
    refuse_promote, write_pack_index,
};
use floor_supervisor::{
    append_apply_audit, apply_dry_run, apply_with_profile_dir, drift_with_roots,
    forget_expired_leases, list_apply_audits, list_expired_leases, list_lifecycle_events,
    load_desired_snapshot, load_lifecycle, load_placements, mark_running, now_unix,
    reconcile_placements, record_placements, refuse_expired_leases, render_dry_run,
    render_reconcile, resume, suspend, write_reconcile, ApplyAudit,
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
        /// Copy the new plan into plans/reviewed/ for a human PR.
        #[arg(long, default_value_t = false)]
        reviewed: bool,
        #[arg(long, default_value = "plans/reviewed")]
        reviewed_dir: PathBuf,
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
        /// Print blast radius + reconcile preview. Does not write leases.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
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
    /// Desired vs actual placement reconcile. Sacred-id deny stays.
    Reconcile {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Bundle local review artifacts. Not a remote upload.
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },
    /// Print append-only lifecycle.jsonl (suspend/resume/apply).
    History {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Catalog-level driver probes. Not live pings. Does not invoke models.
    Probes,
    /// Capability mesh: declare hop, lease-bound call. Not a gateway.
    Convey {
        #[command(subcommand)]
        command: ConveyCommand,
    },
    /// Pack curator path: list / import / refuse promote + INDEX.
    Packs {
        #[command(subcommand)]
        command: PacksCommand,
    },
    /// List expired placement leases. Apply/resume refuse them.
    Expire {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Drop expired rows so a later apply can record fresh leases. Does not spawn.
        #[arg(long, default_value_t = false)]
        forget: bool,
    },
    /// One-page health: .cell layout, schema files, quiet-hours workflows absent.
    Doctor {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
}

#[derive(Subcommand)]
enum ConveyCommand {
    /// Declare a hop and write a durable lease.
    Hop {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "box")]
        kind: String,
        #[arg(long, default_value = "lane-tool")]
        capability: String,
        #[arg(long, default_value = "any")]
        host_class: String,
        #[arg(long, default_value_t = true)]
        wired: bool,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Lease-bound call. Refuses without a granted lease.
    Call {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "lane-tool")]
        capability: String,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// List declared hops.
    List {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Print hop leases.
    Leases {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Derive hops from placement-actual.json (slim parse).
    Sync {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
}

#[derive(Subcommand)]
enum PacksCommand {
    /// List candidate packs and rewrite INDEX.md.
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
    /// Rewrite packs/INDEX.md.
    Index {
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
    },
    /// Write a proposal pack. Never auto-applies. Jason reviews the diff.
    Propose {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
        #[arg(long, default_value = "packs/accepted")]
        accepted_dir: PathBuf,
        #[arg(long, default_value = "packs/proposed")]
        proposed_dir: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
}

#[derive(Subcommand)]
enum AuditCommand {
    /// Write a reviewable folder (and optional tarball) of local audit files.
    Export {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = "packs")]
        packs_dir: PathBuf,
        #[arg(long, default_value = ".cell/audit-export")]
        out: PathBuf,
        /// Also write `{out}.tar.gz` when `tar` is on PATH.
        #[arg(long, default_value_t = false)]
        tar: bool,
    },
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
            reviewed,
            reviewed_dir,
        } => cmd_plan(
            &estate,
            against.as_deref(),
            &plans_dir,
            &state_dir,
            reviewed,
            &reviewed_dir,
        ),
        Command::Apply {
            estate,
            state_dir,
            roots_base,
            plans_dir,
            require_plan,
            import_pack,
            packs_dir,
            require_fresh_plan,
            dry_run,
        } => cmd_apply(
            &estate,
            &state_dir,
            &roots_base,
            &plans_dir,
            require_plan,
            import_pack.as_deref(),
            &packs_dir,
            require_fresh_plan,
            dry_run,
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
        Command::Reconcile { estate, state_dir } => cmd_reconcile(&estate, &state_dir),
        Command::Audit { command } => match command {
            AuditCommand::Export {
                estate,
                state_dir,
                plans_dir,
                packs_dir,
                out,
                tar,
            } => cmd_audit_export(&estate, &state_dir, &plans_dir, &packs_dir, &out, tar),
        },
        Command::History { state_dir } => cmd_history(&state_dir),
        Command::Probes => cmd_probes(),
        Command::Convey { command } => match command {
            ConveyCommand::Hop {
                id,
                kind,
                capability,
                host_class,
                wired,
                state_dir,
            } => cmd_convey_hop(&id, &kind, &capability, &host_class, wired, &state_dir),
            ConveyCommand::Call {
                id,
                capability,
                state_dir,
            } => cmd_convey_call(&id, &capability, &state_dir),
            ConveyCommand::List { state_dir } => cmd_convey_list(&state_dir),
            ConveyCommand::Leases { state_dir } => cmd_convey_leases(&state_dir),
            ConveyCommand::Sync { state_dir } => cmd_convey_sync(&state_dir),
        },
        Command::Packs { command } => match command {
            PacksCommand::List { drop_dir } => cmd_feed_list(&drop_dir),
            PacksCommand::Import {
                id,
                drop_dir,
                accepted_dir,
                estate,
            } => cmd_feed_import(&id, &drop_dir, &accepted_dir, &estate),
            PacksCommand::Promote { id } => cmd_feed_promote(&id),
            PacksCommand::Index { drop_dir } => cmd_packs_index(&drop_dir),
            PacksCommand::Propose {
                id,
                drop_dir,
                accepted_dir,
                proposed_dir,
                estate,
            } => cmd_packs_propose(&id, &drop_dir, &accepted_dir, &proposed_dir, &estate),
        },
        Command::Expire { state_dir, forget } => cmd_expire(&state_dir, forget),
        Command::Doctor { root, state_dir } => cmd_doctor(&root, &state_dir),
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

fn cmd_plan(
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

fn cmd_apply(
    path: &Path,
    state_dir: &Path,
    roots_base: &Path,
    plans_dir: &Path,
    require_plan: bool,
    import_pack_id: Option<&str>,
    packs_dir: &Path,
    require_fresh_plan: bool,
    dry_run: bool,
) -> Result<()> {
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    if dry_run {
        return cmd_apply_dry_run(&estate, path, state_dir, plans_dir, require_plan, require_fresh_plan);
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

fn cmd_apply_dry_run(
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

fn snapshot_state_files(state_dir: &Path) -> Vec<String> {
    let names = [
        "placement-actual.json",
        "actual-state.json",
        "desired-snapshot.yaml",
        "lifecycle.json",
        "apply-audit.jsonl",
        "reconcile.json",
        "catalog.json",
        "model-actual.json",
    ];
    names
        .into_iter()
        .filter_map(|n| {
            let p = state_dir.join(n);
            if p.is_file() {
                Some(format!("{}:{}", n, p.metadata().map(|m| m.len()).unwrap_or(0)))
            } else {
                None
            }
        })
        .collect()
}

fn cmd_expire(state_dir: &Path, forget: bool) -> Result<()> {
    let expired = list_expired_leases(state_dir, now_unix())?;
    if expired.is_empty() {
        println!("no expired leases under {}", state_dir.display());
        return Ok(());
    }
    println!("expired leases ({})", expired.len());
    for lease in &expired {
        println!(
            "  refuse:expired: {} kind={} expires_at={}",
            lease.placement_id,
            lease.kind,
            lease
                .expires_at
                .map(|e| e.to_string())
                .unwrap_or_else(|| "-".into())
        );
    }
    if forget {
        let forgotten = forget_expired_leases(state_dir)?;
        println!("forgot {} expired lease(s); apply may record fresh rows", forgotten.len());
        return Ok(());
    }
    bail!("refuse:expired: apply/resume refuse until estate expire --forget");
}

fn cmd_doctor(root: &Path, state_dir: &Path) -> Result<()> {
    let mut fails: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    println!("Cell One doctor\n===============");
    println!("root: {}", root.display());
    println!("state: {}\n", state_dir.display());

    println!("Schema files");
    println!("------------");
    let required = [
        "schema/estate.v0.schema.json",
        "schema/pack.v0.json",
        "schema/specialist-pack.v0.json",
        "schema/placement-actual.v0.json",
        "schema/reconcile.v0.json",
        "schema/conveyor-mesh.v0.json",
        "schema/local-catalog.v0.json",
        "schema/lifecycle.v0.json",
        "schema/estate-plan.v0.json",
        "schema/feed-cursor.v0.json",
        "schema/enrich-proposal.v0.json",
        "schema/apply-dry-run.v0.json",
    ];
    for rel in required {
        let path = root.join(rel);
        if path.is_file() {
            println!("  ok    {rel}");
        } else {
            println!("  FAIL  {rel}");
            fails.push(format!("missing {rel}"));
        }
    }

    println!("\nQuiet hours");
    println!("-----------");
    let wf = root.join(".github/workflows");
    let mut yml = 0usize;
    if wf.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&wf) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.ends_with(".yml") || name.ends_with(".yaml") {
                    yml += 1;
                    println!("  FAIL  .github/workflows/{name}");
                    fails.push(format!("workflow present: {name}"));
                }
            }
        }
    }
    if yml == 0 {
        println!("  ok    no .github/workflows/*.yml (quiet hours)");
    }

    println!("\n.cell layout");
    println!("------------");
    let layout = [
        ("placement-actual.json", "durable lease"),
        ("lifecycle.json", "durable"),
        ("lifecycle.jsonl", "durable history"),
        ("apply-audit.jsonl", "durable"),
        ("conveyor-leases.json", "durable"),
        ("reconcile.json", "regenerable"),
        ("actual-state.json", "regenerable"),
        ("desired-snapshot.yaml", "regenerable"),
        ("catalog.json", "regenerable"),
        ("sessions/", "disposable"),
        ("runtime/", "disposable"),
    ];
    if !state_dir.exists() {
        println!("  note  {} missing (greenfield ok)", state_dir.display());
        notes.push("greenfield .cell".into());
    } else {
        for (name, kind) in layout {
            let path = state_dir.join(name.trim_end_matches('/'));
            let present = if name.ends_with('/') {
                path.is_dir()
            } else {
                path.is_file()
            };
            println!(
                "  {:<6} {:<24} {}",
                if present { "ok" } else { "note" },
                name,
                kind
            );
        }
    }

    println!("\nLeases");
    println!("------");
    match load_placements(state_dir) {
        Ok(Some(places)) => {
            for lease in &places.leases {
                if lease.kind == "cloud-agent" && lease.spawned {
                    println!("  FAIL  {} spawned cloud-agent", lease.placement_id);
                    fails.push("cloud-agent spawned".into());
                }
            }
            let expired = list_expired_leases(state_dir, now_unix())?;
            if expired.is_empty() {
                println!("  ok    no expired leases");
            } else {
                for lease in &expired {
                    println!("  FAIL  refuse:expired: {}", lease.placement_id);
                    fails.push(format!("expired {}", lease.placement_id));
                }
            }
        }
        Ok(None) => println!("  note  no placement-actual.json"),
        Err(err) => {
            println!("  FAIL  {err}");
            fails.push(err.to_string());
        }
    }

    println!("\nHealth");
    println!("------");
    if fails.is_empty() {
        println!("  ok    factory ready (no workflow yml; schemas present)");
        for note in notes {
            println!("  note  {note}");
        }
        Ok(())
    } else {
        for fail in &fails {
            println!("  FAIL  {fail}");
        }
        bail!("doctor failed ({} check(s))", fails.len());
    }
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

fn cmd_convey_hop(
    id: &str,
    kind: &str,
    capability: &str,
    host_class: &str,
    wired: bool,
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
        },
    )?;
    println!("{}", serde_json::to_string_pretty(&lease)?);
    Ok(())
}

fn cmd_convey_call(id: &str, capability: &str, state_dir: &Path) -> Result<()> {
    let call = call_hop(state_dir, id, capability)?;
    println!("{}", serde_json::to_string_pretty(&call)?);
    if !call.allow {
        bail!("hop call denied");
    }
    Ok(())
}

fn cmd_convey_list(state_dir: &Path) -> Result<()> {
    let hops = list_hops(state_dir)?;
    if hops.is_empty() {
        println!("no hops under {}", state_dir.display());
        return Ok(());
    }
    println!("{}", serde_json::to_string_pretty(&hops)?);
    Ok(())
}

fn cmd_convey_leases(state_dir: &Path) -> Result<()> {
    let leases = list_hop_leases(state_dir)?;
    if leases.is_empty() {
        println!("no hop leases under {}", state_dir.display());
        return Ok(());
    }
    println!("{}", serde_json::to_string_pretty(&leases)?);
    Ok(())
}

fn cmd_convey_sync(state_dir: &Path) -> Result<()> {
    let mesh = sync_from_placements(state_dir)?;
    println!("{}", serde_json::to_string_pretty(&mesh)?);
    Ok(())
}

fn cmd_packs_index(drop_dir: &Path) -> Result<()> {
    let path = write_pack_index(drop_dir)?;
    println!("wrote {}", path.display());
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

fn cmd_reconcile(path: &Path, state_dir: &Path) -> Result<()> {
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    let report = reconcile_placements(&estate, state_dir)?;
    let written = write_reconcile(state_dir, &report)?;
    print!("{}", render_reconcile(&report));
    println!("Wrote {}", written.display());
    if !report.in_sync {
        bail!("reconcile drift (fail closed)");
    }
    Ok(())
}

fn cmd_packs_propose(
    id: &str,
    drop_dir: &Path,
    accepted_dir: &Path,
    proposed_dir: &Path,
    estate_path: &Path,
) -> Result<()> {
    let estate = load_estate(estate_path)
        .with_context(|| format!("load {}", estate_path.display()))?;
    let before = std::fs::read_to_string(estate_path).unwrap_or_default();
    let (proposal, dest) = propose_enrich(drop_dir, accepted_dir, proposed_dir, id, &estate)?;
    let after = std::fs::read_to_string(estate_path).unwrap_or_default();
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

fn copy_if_exists(src: &Path, dest: &Path, copied: &mut Vec<String>) -> Result<bool> {
    if !src.is_file() {
        return Ok(false);
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, dest)?;
    copied.push(src.display().to_string());
    Ok(true)
}

fn copy_tree_files(src: &Path, dest: &Path, copied: &mut Vec<String>) -> Result<()> {
    if !src.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().unwrap_or_default();
            std::fs::copy(&path, dest.join(name))?;
            copied.push(path.display().to_string());
        } else if path.is_dir() {
            copy_tree_files(&path, &dest.join(path.file_name().unwrap_or_default()), copied)?;
        }
    }
    Ok(())
}

fn cmd_audit_export(
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
