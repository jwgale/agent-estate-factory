use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use estate_schema::ModelClass;
use model_estate::{
    frontier_from_binding, local_bindings, local_from_binding, readiness, render_catalog, run_task,
    serve_specialist_forever, TaskAct, TaskRequest,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "model-estate",
    about = "Data-plane mixed model path. Not an AI gateway. `estate specialist` is a thin delegate to this helper."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Authorize → local specialist → tool or frontier. Emits scrubbed feed events.
    Task {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        act: String,
        #[arg(long)]
        object: String,
        #[arg(long, default_value = "Reply with the single word pong.")]
        payload: String,
        #[arg(long)]
        feed_dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        mock: bool,
    },
    /// Speak the portable specialist protocol without a GPU (operator / gate stand-in).
    MockLocal {
        #[arg(long, default_value = "127.0.0.1:47831")]
        bind: String,
    },
    /// Print the portable local-runtime catalog. Not a model library.
    Catalog {
        /// Optional file SoT dump (JSON). Regenerable.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Ready {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    /// Catalog-level driver probes. Not live pings.
    Probe,
    /// Policy-precheck / redact / complete through HttpLocal.
    /// `estate specialist` is the same helper with `--job complete`.
    Specialist {
        /// Override. Default: CELL_LOCAL_ENDPOINT.
        #[arg(long)]
        endpoint: Option<String>,
        #[arg(long, default_value = "policy-precheck")]
        job: String,
        #[arg(long, default_value = "cli")]
        agent: String,
        #[arg(long, default_value = "model")]
        kind: String,
        #[arg(long)]
        text: Option<String>,
        /// Alias for `--text`.
        #[arg(long)]
        prompt: Option<String>,
        /// ollama | llama.cpp | http-remote (same adapter). Default ollama.
        #[arg(long, default_value = "ollama")]
        runtime: String,
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
        Command::Task {
            estate,
            agent,
            act,
            object,
            payload,
            feed_dir,
            mock,
        } => cmd_task(
            &estate,
            &agent,
            &act,
            &object,
            &payload,
            feed_dir.as_ref(),
            mock,
        ),
        Command::MockLocal { bind } => serve_specialist_forever(&bind).map_err(anyhow::Error::msg),
        Command::Catalog { out } => {
            println!("{}", render_catalog());
            if let Some(path) = out {
                let written = model_estate::write_catalog(&path)?;
                println!("wrote catalog file {}", written.display());
            }
            Ok(())
        }
        Command::Ready { estate } => {
            let loaded = estate_schema::load_estate(&estate)?;
            println!("{}", model_estate::describe_bindings(&loaded));
            println!("{}", readiness(&loaded));
            Ok(())
        }
        Command::Probe => {
            for probe in model_estate::catalog_probes() {
                println!(
                    "  {:<12} status={:<12} bindable={} live_probed={} host={}",
                    probe.driver, probe.status, probe.bindable, probe.live_probed, probe.host_class
                );
            }
            Ok(())
        }
        Command::Specialist {
            endpoint,
            job,
            agent,
            kind,
            text,
            prompt,
            runtime,
        } => cmd_specialist(endpoint, &job, &agent, &kind, prompt, text, &runtime),
    }
}

fn cmd_specialist(
    endpoint: Option<String>,
    job: &str,
    agent: &str,
    kind: &str,
    prompt: Option<String>,
    text: Option<String>,
    runtime: &str,
) -> Result<()> {
    let text = prompt
        .or(text)
        .ok_or_else(|| anyhow::anyhow!("set --prompt or --text"))?;
    let result = model_estate::run_http_specialist(
        endpoint.as_deref(),
        job,
        agent,
        kind,
        &text,
        runtime,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    if !result.allow {
        bail!("specialist denied");
    }
    Ok(())
}

fn cmd_task(
    estate: &PathBuf,
    agent: &str,
    act: &str,
    object: &str,
    payload: &str,
    feed_dir: Option<&PathBuf>,
    mock: bool,
) -> Result<()> {
    let loaded = estate_schema::load_estate(estate).with_context(|| estate.display().to_string())?;
    let act = match act {
        "tool" => TaskAct::Tool,
        "model" => TaskAct::Model,
        other => bail!("act must be tool or model, got {other}"),
    };
    let req = TaskRequest {
        agent_id: agent.to_string(),
        act,
        object: object.to_string(),
        payload: payload.to_string(),
    };

    if mock {
        let local_id = local_bindings(&loaded)
            .first()
            .map(|b| b.id.as_str())
            .unwrap_or("local_slm")
            .to_string();
        let frontier_id = loaded
            .model_bindings
            .iter()
            .find(|b| b.class == ModelClass::Frontier)
            .map(|b| b.id.as_str())
            .unwrap_or("xai_grok")
            .to_string();
        let local = model_estate::MockLocal { id: local_id };
        let frontier = model_estate::MockFrontier {
            id: frontier_id,
            reply: "pong".into(),
        };
        let result = run_task(
            &loaded,
            &req,
            Some(&frontier),
            Some(&local),
            feed_dir.map(|p| p.as_path()),
        )?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        if result.denied.is_some() || !result.authorized {
            bail!("task denied");
        }
        return Ok(());
    }

    let local_box = if let Some(binding) = local_bindings(&loaded).first() {
        Some(local_from_binding(binding)?)
    } else {
        None
    };

    let frontier_box = if matches!(act, TaskAct::Model) {
        let binding = loaded
            .model_bindings
            .iter()
            .find(|b| b.id == object)
            .ok_or_else(|| anyhow::anyhow!("unknown binding {object}"))?;
        match frontier_from_binding(binding) {
            Ok(driver) => Some(driver),
            Err(err) => {
                // Local may still fail-closed; audit that before surfacing missing frontier creds.
                if local_box.is_some() {
                    None
                } else {
                    return Err(err.into());
                }
            }
        }
    } else {
        None
    };

    let result = run_task(
        &loaded,
        &req,
        frontier_box.as_deref(),
        local_box.as_deref(),
        feed_dir.map(|p| p.as_path()),
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    if result.denied.is_some() || !result.authorized {
        bail!("task denied");
    }
    Ok(())
}
