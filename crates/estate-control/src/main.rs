use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use estate_schema::{
    describe, describe_placements, diff_estates, list_plans, load_estate, load_estate_unvalidated,
    render_plan, validate, write_plan,
};
use feed_collector::{list_drop_packs, materialize_from_feed, refuse_promote};
use floor_supervisor::{
    apply_with_profile_dir, drift_with_roots, load_desired_snapshot, load_lifecycle, mark_running,
    resume, suspend,
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
}

#[derive(Subcommand)]
enum FeedCommand {
    /// Read scrubbed events and write a candidate pack (not in the estate).
    Pack {
        #[arg(long, default_value = ".cell/feed")]
        feed_dir: PathBuf,
        #[arg(long, default_value = "examples/enrich-packs/drop")]
        drop_dir: PathBuf,
        #[arg(long, default_value = "overnight-traces")]
        id: String,
    },
    /// List candidate packs in the drop zone.
    List {
        #[arg(long, default_value = "examples/enrich-packs/drop")]
        drop_dir: PathBuf,
    },
    /// Always fails. Jason edits estate.yaml by hand.
    Promote {
        #[arg(long)]
        id: String,
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
        } => cmd_apply(&estate, &state_dir, &roots_base),
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
            FeedCommand::Promote { id } => cmd_feed_promote(&id),
        },
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

fn cmd_apply(path: &Path, state_dir: &Path, roots_base: &Path) -> Result<()> {
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    let actual = apply_with_profile_dir(&estate, state_dir, roots_base)?;
    model_estate::record_bindings(&estate, state_dir)?;
    mark_running(&estate, state_dir)?;
    println!(
        "applied {} sessions; actual-state {}",
        actual.sessions.len(),
        state_dir.join("actual-state.json").display()
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
        println!("  {}", entry.markdown);
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
    println!("floor in_sync: {}", report.in_sync);
    if !report.notes.is_empty() {
        for note in &report.notes {
            println!("  {note}");
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

fn cmd_feed_list(drop_dir: &Path) -> Result<()> {
    let packs = list_drop_packs(drop_dir)?;
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

fn cmd_feed_promote(id: &str) -> Result<()> {
    match refuse_promote(id) {
        Ok(()) => unreachable!("promote has no success path"),
        Err(err) => bail!("{err}"),
    }
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
