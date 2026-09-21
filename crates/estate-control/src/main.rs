use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use estate_schema::{
    describe, diff_estates, load_estate, load_estate_unvalidated, render_plan, validate, write_plan,
};
use floor_supervisor::{apply_with_profile_dir, drift_with_roots, load_desired_snapshot};
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
