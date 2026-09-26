//! `estate pack mcp-serve` — stdio MCP bridge to `estate complete`.
//!
//! One process is one pack member. Tool `complete` runs
//! `estate complete --agent <member> --pack <pack-id>` against
//! `CELL_ESTATE_PATH`. Orchestrator refuse stays the same as pack
//! complete. Not live Cursor / Grok Bot sync. Not a cron daemon.

use anyhow::{bail, Context, Result};
use estate_schema::normalize_name;
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) const PROTOCOL_VERSION: &str = "2024-11-05";
pub(crate) const SERVER_NAME: &str = "cell-one-pack-mcp";
pub(crate) const TOOL_NAME: &str = "complete";

#[derive(Debug)]
pub(crate) struct McpIdentity {
    pub pack: String,
    pub member: String,
    pub role: String,
    pub estate: PathBuf,
    pub state_dir: PathBuf,
    pub mock: bool,
}

pub(crate) fn cmd_pack_mcp_serve(
    estate: Option<PathBuf>,
    pack: Option<String>,
    agent: Option<String>,
    state_dir: Option<PathBuf>,
    mock: bool,
) -> Result<()> {
    let mut ident = resolve_identity(estate, pack, agent, state_dir, mock, |k| {
        std::env::var(k).ok()
    })?;
    let loaded = estate_schema::load_estate(&ident.estate)
        .with_context(|| format!("load {}", ident.estate.display()))?;
    let pack = loaded.pack(&ident.pack).ok_or_else(|| {
        anyhow::anyhow!("refuse:unknown-pack: pack '{}' not on estate", ident.pack)
    })?;
    let member_n = normalize_name(&ident.member);
    if !pack.members.iter().any(|m| normalize_name(m) == member_n) {
        bail!(
            "refuse:pack-member: --agent '{}' is not a member of pack '{}'",
            ident.member,
            pack.id
        );
    }
    ident.pack = pack.id.clone();
    if ident.role.is_empty() || ident.role == "-" {
        ident.role = pack_role(pack.orchestrator.as_deref(), &ident.member);
    }
    serve_stdio(&ident)
}

pub(crate) fn resolve_identity(
    estate: Option<PathBuf>,
    pack: Option<String>,
    agent: Option<String>,
    state_dir: Option<PathBuf>,
    mock: bool,
    env: impl Fn(&str) -> Option<String>,
) -> Result<McpIdentity> {
    let pack = first_nonempty(pack, env("CELL_ESTATE_PACK"))
        .ok_or_else(|| anyhow::anyhow!("refuse:mcp-pack: set --pack or CELL_ESTATE_PACK"))?;
    let member = first_nonempty(agent, env("CELL_ESTATE_MEMBER"))
        .or_else(|| env("CELL_ESTATE_AGENT"))
        .ok_or_else(|| anyhow::anyhow!("refuse:mcp-member: set --agent or CELL_ESTATE_MEMBER"))?;
    let estate = estate
        .or_else(|| env("CELL_ESTATE_PATH").map(PathBuf::from))
        .ok_or_else(|| anyhow::anyhow!("refuse:mcp-estate: set --estate or CELL_ESTATE_PATH"))?;
    let state_dir = state_dir
        .or_else(|| env("CELL_ESTATE_STATE_DIR").map(PathBuf::from))
        .unwrap_or_else(|| estate_default_state_dir(&estate));
    let role = env("CELL_ESTATE_ROLE").unwrap_or_default();
    let mock = mock || env_truthy(env("CELL_ESTATE_MOCK").as_deref());
    Ok(McpIdentity {
        pack,
        member,
        role,
        estate,
        state_dir,
        mock,
    })
}

pub(crate) fn pack_role(orchestrator: Option<&str>, member: &str) -> String {
    match orchestrator {
        Some(orch) if normalize_name(orch) == normalize_name(member) => "orchestrator".into(),
        _ => "member".into(),
    }
}

pub(crate) fn estate_default_state_dir(estate: &Path) -> PathBuf {
    match estate.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(".cell"),
        _ => PathBuf::from(".cell"),
    }
}

pub(crate) fn env_truthy(raw: Option<&str>) -> bool {
    matches!(
        raw.map(str::trim).map(str::to_ascii_lowercase).as_deref(),
        Some("1" | "true" | "on" | "yes")
    )
}

fn first_nonempty(flag: Option<String>, env: Option<String>) -> Option<String> {
    flag.filter(|s| !s.trim().is_empty())
        .or_else(|| env.filter(|s| !s.trim().is_empty()))
}

fn serve_stdio(ident: &McpIdentity) -> Result<()> {
    let stdin = io::stdin();
    let mut input = BufReader::new(stdin.lock());
    let mut output = io::stdout().lock();
    while let Some(msg) = read_message(&mut input)? {
        if let Some(resp) = handle_message(ident, &msg)? {
            write_message(&mut output, &resp)?;
        }
    }
    Ok(())
}

pub(crate) fn handle_message(ident: &McpIdentity, msg: &Value) -> Result<Option<Value>> {
    let method = match msg.get("method").and_then(Value::as_str) {
        Some(m) => m,
        None => return Ok(None),
    };
    let id = msg.get("id").cloned();
    if id.is_none() {
        return Ok(None);
    }
    let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
    let result = match method {
        "initialize" => initialize_result(ident),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": [complete_tool(ident)] }),
        "tools/call" => call_tool(ident, &params),
        _ => {
            return Ok(Some(jsonrpc_error(
                id,
                -32601,
                format!("method not found: {method}"),
            )))
        }
    };
    Ok(Some(jsonrpc_result(id, result)))
}

fn initialize_result(ident: &McpIdentity) -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": SERVER_NAME, "version": "0.0.0" },
        "instructions": format!(
            "Pack {} member {} ({}). Tool `{TOOL_NAME}` runs estate complete --agent {} --pack {} against {}. live_sync is false. Not Cursor/Grok Bot install.",
            ident.pack, ident.member, ident.role, ident.member, ident.pack, ident.estate.display()
        ),
    })
}

pub(crate) fn complete_tool(ident: &McpIdentity) -> Value {
    json!({
        "name": TOOL_NAME,
        "description": format!(
            "Run estate complete as pack member {} on pack {}. Prompt is required. Receipts stay on the source estate ({}). Non-orchestrator members refuse pack-orchestrator the same as estate complete --pack. Not live Cursor/Grok Bot sync.",
            ident.member, ident.pack, ident.estate.display()
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Prompt (or text) for estate complete"
                },
                "text": {
                    "type": "string",
                    "description": "Alias for prompt"
                },
                "mock": {
                    "type": "boolean",
                    "description": "Use in-process mock drivers. Not a live generate."
                },
                "object": {
                    "type": "string",
                    "description": "Optional binding id for estate complete --object"
                }
            },
            "required": ["prompt"]
        }
    })
}

fn call_tool(ident: &McpIdentity, params: &Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    if name != TOOL_NAME {
        return tool_error(format!("unknown tool: {name}"));
    }
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let prompt = args
        .get("prompt")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            args.get("text")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
        });
    let Some(prompt) = prompt else {
        return tool_error("refuse:mcp-prompt: set prompt (or text)".into());
    };
    let mock = match args.get("mock") {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(s)) => env_truthy(Some(s)),
        _ => ident.mock,
    };
    let object = args.get("object").and_then(Value::as_str);
    let (ok, text) = invoke_complete(ident, prompt, mock, object);
    if ok {
        tool_text(&text, false)
    } else {
        tool_error(text)
    }
}

fn invoke_complete(
    ident: &McpIdentity,
    prompt: &str,
    mock: bool,
    object: Option<&str>,
) -> (bool, String) {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("estate"));
    let mut cmd = Command::new(exe);
    cmd.arg("complete")
        .arg("--estate")
        .arg(&ident.estate)
        .arg("--agent")
        .arg(&ident.member)
        .arg("--pack")
        .arg(&ident.pack)
        .arg("--prompt")
        .arg(prompt)
        .arg("--state-dir")
        .arg(&ident.state_dir);
    if mock {
        cmd.arg("--mock");
    }
    if let Some(object) = object {
        cmd.arg("--object").arg(object);
    }
    match cmd.output() {
        Ok(out) => {
            let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
            let err = String::from_utf8_lossy(&out.stderr);
            if !err.is_empty() {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(&err);
            }
            (out.status.success(), text)
        }
        Err(err) => (false, format!("refuse:mcp-complete: {err}")),
    }
}

fn tool_text(text: &str, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error,
    })
}

fn tool_error(text: String) -> Value {
    tool_text(&text, true)
}

fn jsonrpc_result(id: Option<Value>, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn jsonrpc_error(id: Option<Value>, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}

pub(crate) fn write_message(out: &mut impl Write, value: &Value) -> Result<()> {
    let body = serde_json::to_string(value)?;
    write!(out, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    out.flush()?;
    Ok(())
}

pub(crate) fn read_message(input: &mut impl BufRead) -> Result<Option<Value>> {
    let mut header = String::new();
    let mut content_length: Option<usize> = None;
    loop {
        header.clear();
        let n = input.read_line(&mut header)?;
        if n == 0 {
            return if content_length.is_some() {
                bail!("refuse:mcp-frame: unexpected EOF in headers")
            } else {
                Ok(None)
            };
        }
        let line = header.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if line.starts_with('{') {
            return Ok(Some(
                serde_json::from_str(line).context("parse ndjson MCP message")?,
            ));
        }
        if let Some((key, value)) = line.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                content_length = Some(
                    value
                        .trim()
                        .parse()
                        .with_context(|| format!("Content-Length: {value}"))?,
                );
            }
        }
    }
    let len = content_length
        .ok_or_else(|| anyhow::anyhow!("refuse:mcp-frame: missing Content-Length"))?;
    let mut buf = vec![0u8; len];
    io::Read::read_exact(input, &mut buf).context("read MCP body")?;
    Ok(Some(
        serde_json::from_slice(&buf).context("parse MCP message")?,
    ))
}

/// Env keys written into exported `mcp.json` (and required by mcp-serve).
pub(crate) fn export_mcp_env(pack_id: &str, member: &str, role: &str, estate_path: &str) -> Value {
    json!({
        "CELL_ESTATE_PACK": pack_id,
        "CELL_ESTATE_MEMBER": member,
        "CELL_ESTATE_ROLE": role,
        "CELL_ESTATE_PATH": estate_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn env_map<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| {
            pairs
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn resolve_identity_reads_env_and_defaults_state_dir() {
        let ident = resolve_identity(
            None,
            None,
            None,
            None,
            false,
            env_map(&[
                ("CELL_ESTATE_PACK", "research-crew"),
                ("CELL_ESTATE_MEMBER", "horizon"),
                ("CELL_ESTATE_ROLE", "orchestrator"),
                ("CELL_ESTATE_PATH", "/tmp/fixture.yaml"),
            ]),
        )
        .unwrap();
        assert_eq!(ident.pack, "research-crew");
        assert_eq!(ident.member, "horizon");
        assert_eq!(ident.role, "orchestrator");
        assert_eq!(ident.estate, PathBuf::from("/tmp/fixture.yaml"));
        assert_eq!(ident.state_dir, PathBuf::from("/tmp/.cell"));
        assert!(!ident.mock);
    }

    #[test]
    fn resolve_identity_refuses_missing_estate() {
        let err = resolve_identity(
            None,
            Some("crew".into()),
            Some("horizon".into()),
            None,
            false,
            env_map(&[]),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("refuse:mcp-estate"), "{err}");
    }

    #[test]
    fn env_truthy_accepts_common_flags() {
        assert!(env_truthy(Some("1")));
        assert!(env_truthy(Some("TRUE")));
        assert!(!env_truthy(Some("0")));
        assert!(!env_truthy(None));
    }

    #[test]
    fn pack_role_marks_orchestrator() {
        assert_eq!(pack_role(Some("horizon"), "horizon"), "orchestrator");
        assert_eq!(pack_role(Some("horizon"), "research"), "member");
        assert_eq!(pack_role(None, "horizon"), "member");
    }

    #[test]
    fn complete_tool_names_member_and_estate() {
        let ident = McpIdentity {
            pack: "research-crew".into(),
            member: "horizon".into(),
            role: "orchestrator".into(),
            estate: PathBuf::from("/tmp/fixture.yaml"),
            state_dir: PathBuf::from("/tmp/.cell"),
            mock: false,
        };
        let tool = complete_tool(&ident);
        assert_eq!(tool["name"], TOOL_NAME);
        let desc = tool["description"].as_str().unwrap();
        assert!(desc.contains("horizon"), "{desc}");
        assert!(desc.contains("research-crew"), "{desc}");
        assert!(desc.contains("/tmp/fixture.yaml"), "{desc}");
        assert!(desc.contains("refuse pack-orchestrator"), "{desc}");
    }

    #[test]
    fn handle_initialize_and_unknown_method() {
        let ident = McpIdentity {
            pack: "research-crew".into(),
            member: "horizon".into(),
            role: "orchestrator".into(),
            estate: PathBuf::from("/tmp/fixture.yaml"),
            state_dir: PathBuf::from("/tmp/.cell"),
            mock: false,
        };
        let init = handle_message(
            &ident,
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        )
        .unwrap()
        .unwrap();
        assert_eq!(init["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(init["result"]["serverInfo"]["name"], SERVER_NAME);
        assert_eq!(
            handle_message(
                &ident,
                &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            )
            .unwrap(),
            None
        );
        let missing = handle_message(&ident, &json!({"jsonrpc":"2.0","id":2,"method":"nope"}))
            .unwrap()
            .unwrap();
        assert_eq!(missing["error"]["code"], -32601);

        let listed = handle_message(
            &ident,
            &json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}),
        )
        .unwrap()
        .unwrap();
        assert_eq!(listed["result"]["tools"][0]["name"], TOOL_NAME);

        let no_prompt = handle_message(
            &ident,
            &json!({
                "jsonrpc":"2.0",
                "id":4,
                "method":"tools/call",
                "params":{"name":"complete","arguments":{}}
            }),
        )
        .unwrap()
        .unwrap();
        assert_eq!(no_prompt["result"]["isError"], true);
        assert!(
            no_prompt["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("refuse:mcp-prompt"),
            "{no_prompt}"
        );
    }

    #[test]
    fn frame_roundtrip_content_length_and_ndjson() {
        let msg = json!({"jsonrpc":"2.0","id":1,"method":"ping"});
        let mut buf = Vec::new();
        write_message(&mut buf, &msg).unwrap();
        let framed = String::from_utf8(buf).unwrap();
        assert!(framed.starts_with("Content-Length:"), "{framed}");
        let parsed = read_message(&mut Cursor::new(framed.into_bytes()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed["method"], "ping");

        let nd = read_message(&mut Cursor::new(
            b"{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"ping\"}\n".as_slice(),
        ))
        .unwrap()
        .unwrap();
        assert_eq!(nd["id"], 7);
    }

    #[test]
    fn export_mcp_env_carries_pack_member_role_path() {
        let env = export_mcp_env("research-crew", "horizon", "orchestrator", "/tmp/e.yaml");
        assert_eq!(env["CELL_ESTATE_PACK"], "research-crew");
        assert_eq!(env["CELL_ESTATE_MEMBER"], "horizon");
        assert_eq!(env["CELL_ESTATE_ROLE"], "orchestrator");
        assert_eq!(env["CELL_ESTATE_PATH"], "/tmp/e.yaml");
    }
}
