use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use conveyor_proxy::{check, parse_kind, response_from, ProxyRequest};
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
