use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use floor_supervisor::{apply_with_profile_dir, drift, spawn_runtime_heartbeats, stop_runtime};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "floor-supervisor",
    about = "Bind per-agent sessions from an estate file. Does not call models."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Apply {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
        #[arg(long, default_value_t = false)]
        spawn: bool,
    },
    Status {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    Stop {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Apply {
            estate,
            state_dir,
            roots_base,
            spawn,
        } => {
            let loaded = estate_schema::load_estate(&estate)
                .with_context(|| format!("load {}", estate.display()))?;
            let actual = apply_with_profile_dir(&loaded, &state_dir, &roots_base)?;
            println!(
                "bound {} sessions from {} (hash {})",
                actual.sessions.len(),
                loaded.name,
                actual.desired_hash
            );
            for session in &actual.sessions {
                println!(
                    "  {} desktop={} lane={} handle={}",
                    session.agent_id,
                    session.desktop,
                    session.lane_id,
                    session.isolation_handle.as_token()
                );
            }
            if spawn {
                let pids = spawn_runtime_heartbeats(&actual, &state_dir)?;
                println!("wrote disposable runtime {}", pids.display());
            }
        }
        Command::Status { estate, state_dir } => {
            let loaded = estate_schema::load_estate(&estate)
                .with_context(|| format!("load {}", estate.display()))?;
            let report = drift(&loaded, &state_dir)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            if !report.in_sync {
                std::process::exit(2);
            }
        }
        Command::Stop { state_dir } => {
            stop_runtime(&state_dir)?;
            println!(
                "stopped runtime under {} (lane roots and estate file untouched)",
                state_dir.display()
            );
        }
    }
    Ok(())
}
