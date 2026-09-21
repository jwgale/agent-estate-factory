use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use conveyor_proxy::{
    call_hop, check, declare_hop, list_hop_leases, list_hops, parse_kind, response_from,
    sync_from_placements, HopDecl, ProxyRequest,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "conveyor-proxy",
    about = "Deny-by-default proxy. Workers must come through here."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Check {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        object: String,
        #[arg(long)]
        feed_dir: Option<PathBuf>,
    },
    Serve {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = "127.0.0.1:47821")]
        bind: String,
        #[arg(long)]
        feed_dir: Option<PathBuf>,
    },
    /// Declare a capability hop (lease written under --state-dir).
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
    /// Lease-bound hop call. Refuses without a granted lease.
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
    /// List hop leases.
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Check {
            estate,
            agent,
            kind,
            object,
            feed_dir,
        } => {
            let loaded = estate_schema::load_estate(&estate)
                .with_context(|| format!("load {}", estate.display()))?;
            let kind = parse_kind(&kind).map_err(anyhow::Error::msg)?;
            let decision = check(
                &loaded,
                &agent,
                kind,
                &object,
                feed_dir.as_deref(),
            );
            let resp = response_from(&decision);
            println!("{}", serde_json::to_string_pretty(&resp)?);
            if !decision.is_allow() {
                std::process::exit(2);
            }
        }
        Command::Serve {
            estate,
            bind,
            feed_dir,
        } => {
            let loaded = estate_schema::load_estate(&estate)
                .with_context(|| format!("load {}", estate.display()))?;
            let server = tiny_http::Server::http(&bind)
                .map_err(|e| anyhow::anyhow!("bind {bind}: {e}"))?;
            eprintln!("conveyor-proxy listening on http://{bind}  POST /v0/check");
            for mut request in server.incoming_requests() {
                let mut body = String::new();
                let _ = std::io::Read::read_to_string(request.as_reader(), &mut body);
                let reply = handle(&loaded, request.url(), &body, feed_dir.as_deref());
                let status = if reply.0 { 200 } else { 403 };
                let mut response = tiny_http::Response::from_string(reply.1).with_status_code(status);
                let header = tiny_http::Header::from_bytes(
                    &b"Content-Type"[..],
                    &b"application/json"[..],
                )
                .expect("header");
                response = response.with_header(header);
                let _ = request.respond(response);
            }
        }
        Command::Hop {
            id,
            kind,
            capability,
            host_class,
            wired,
            state_dir,
        } => {
            let lease = declare_hop(
                &state_dir,
                HopDecl {
                    id,
                    kind,
                    capability,
                    host_class,
                    wired,
                    note: None,
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&lease)?);
        }
        Command::Call {
            id,
            capability,
            state_dir,
        } => {
            let call = call_hop(&state_dir, &id, &capability)?;
            println!("{}", serde_json::to_string_pretty(&call)?);
            if !call.allow {
                bail!("hop call denied");
            }
        }
        Command::List { state_dir } => {
            let hops = list_hops(&state_dir)?;
            if hops.is_empty() {
                println!("no hops under {}", state_dir.display());
            } else {
                println!("{}", serde_json::to_string_pretty(&hops)?);
            }
        }
        Command::Leases { state_dir } => {
            let leases = list_hop_leases(&state_dir)?;
            if leases.is_empty() {
                println!("no hop leases under {}", state_dir.display());
            } else {
                println!("{}", serde_json::to_string_pretty(&leases)?);
            }
        }
        Command::Sync { state_dir } => {
            let mesh = sync_from_placements(&state_dir)?;
            println!("{}", serde_json::to_string_pretty(&mesh)?);
        }
    }
    Ok(())
}

fn handle(estate: &estate_schema::Estate, url: &str, body: &str, feed: Option<&std::path::Path>) -> (bool, String) {
    if url != "/v0/check" && url != "/v0/check/" {
        return (
            false,
            serde_json::json!({"decision":"deny","reason":"unknown path; workers POST /v0/check"})
                .to_string(),
        );
    }
    let req: ProxyRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return (
                false,
                serde_json::json!({"decision":"deny","reason":format!("bad request: {e}")})
                    .to_string(),
            );
        }
    };
    let kind = match parse_kind(&req.kind) {
        Ok(k) => k,
        Err(e) => {
            return (
                false,
                serde_json::json!({"decision":"deny","reason":e}).to_string(),
            );
        }
    };
    let decision = check(estate, &req.agent_id, kind, &req.object, feed);
    let resp = response_from(&decision);
    (
        decision.is_allow(),
        serde_json::to_string_pretty(&resp).unwrap_or_default(),
    )
}
