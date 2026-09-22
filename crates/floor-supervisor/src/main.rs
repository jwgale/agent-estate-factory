use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use floor_supervisor::{
    apply_with_profile_dir, lifecycle_path, list_lifecycle_events, load_lifecycle, load_placements,
    resume, spawn_runtime_heartbeats, stop_runtime, suspend,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "floor",
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
    /// Drop sessions/PIDs; keep lifecycle.json and unspawned leases.
    Suspend {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Rebind from the estate file. Cloud-agent leases stay unspawned.
    Resume {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
    /// Print durable placement-actual leases.
    Leases {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Print append-only lifecycle.jsonl.
    History {
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
            if let Some(places) = load_placements(&state_dir)? {
                for lease in &places.leases {
                    println!(
                        "  lease {} kind={} driver={} spawned={}",
                        lease.placement_id, lease.kind, lease.driver, lease.spawned
                    );
                }
            }
            if spawn {
                let pids = spawn_runtime_heartbeats(&actual, &state_dir)?;
                println!("wrote disposable runtime {}", pids.display());
            }
        }
        Command::Status { estate, state_dir } => {
            let loaded = estate_schema::load_estate(&estate)
                .with_context(|| format!("load {}", estate.display()))?;
            // Missing lifecycle.json is the default record, not a file.
            // Printing that default would invent suspended and durable=true.
            // A present file that does not parse refuses before this line.
            let life = if lifecycle_path(&state_dir).exists() {
                Some(load_lifecycle(&state_dir)?)
            } else {
                None
            };
            let report = floor_supervisor::drift_with_roots(&loaded, &state_dir, None)?;
            match life {
                None => println!("lifecycle: -"),
                Some(life) => {
                    println!("lifecycle: {} durable={}", life.state.as_str(), life.durable);
                }
            }
            println!("{}", serde_json::to_string_pretty(&report)?);
            if let Some(places) = load_placements(&state_dir)? {
                println!("{}", serde_json::to_string_pretty(&places)?);
            }
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
        Command::Suspend { state_dir } => {
            let record = suspend(&state_dir)?;
            println!("lifecycle: {}", record.state.as_str());
            println!("{}", record.note);
        }
        Command::Resume {
            estate,
            state_dir,
            roots_base,
        } => {
            let loaded = estate_schema::load_estate(&estate)
                .with_context(|| format!("load {}", estate.display()))?;
            let (actual, record) = resume(&loaded, &state_dir, &roots_base)?;
            println!(
                "resumed {} sessions; lifecycle {}",
                actual.sessions.len(),
                record.state.as_str()
            );
            if let Some(places) = load_placements(&state_dir)? {
                for lease in &places.leases {
                    println!(
                        "  lease {} kind={} spawned={}",
                        lease.placement_id, lease.kind, lease.spawned
                    );
                }
            }
        }
        Command::Leases { state_dir } => {
            match load_placements(&state_dir)? {
                None => {
                    println!("no placement-actual.json under {}", state_dir.display());
                    println!("run apply or resume to record leases");
                }
                Some(places) => {
                    println!("{}", serde_json::to_string_pretty(&places)?);
                    for lease in &places.leases {
                        if lease.kind == "cloud-agent" && lease.spawned {
                            anyhow::bail!("cloud-agent lease spawned (fail closed)");
                        }
                    }
                }
            }
        }
        Command::History { state_dir } => {
            let events = list_lifecycle_events(&state_dir)?;
            if events.is_empty() {
                println!("no lifecycle.jsonl under {}", state_dir.display());
            } else {
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
            }
        }
    }
    Ok(())
}
