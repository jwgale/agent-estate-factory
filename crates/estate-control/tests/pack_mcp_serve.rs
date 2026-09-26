//! `estate pack mcp-serve` — stdio MCP bridge to `estate complete`.
//!
//! Fixture: examples/fixtures/agent-pack-handoff.yaml.
//! Mock complete only. Does not invent a live PASS.
//! Locked examples/estate.yaml stays untouched.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("cell-pack-mcp-serve-{name}-{nanos}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn estate_bin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_estate"));
    cmd.env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .env_remove("CELL_LOCAL_LIVE");
    cmd
}

fn assert_locked_cksum() {
    let sum = Command::new("cksum")
        .arg(repo_root().join("examples/estate.yaml"))
        .output()
        .unwrap();
    let sum_text = String::from_utf8_lossy(&sum.stdout);
    assert!(
        sum_text.starts_with("43770130 3391"),
        "examples/estate.yaml cksum changed: {sum_text}"
    );
}

fn fixture() -> PathBuf {
    repo_root().join("examples/fixtures/agent-pack-handoff.yaml")
}

fn write_frame(out: &mut impl Write, value: &Value) {
    let body = serde_json::to_string(value).unwrap();
    write!(out, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    out.flush().unwrap();
}

fn read_frame(input: &mut impl BufRead) -> Value {
    let mut header = String::new();
    let mut content_length = None;
    loop {
        header.clear();
        let n = input.read_line(&mut header).unwrap();
        assert!(n > 0, "EOF before MCP frame");
        let line = header.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                content_length = Some(value.trim().parse::<usize>().unwrap());
            }
        }
    }
    let len = content_length.expect("Content-Length");
    let mut buf = vec![0u8; len];
    std::io::Read::read_exact(input, &mut buf).unwrap();
    serde_json::from_slice(&buf).unwrap()
}

fn journal(state: &Path) -> PathBuf {
    state.join("decisions").join("receipts.jsonl")
}

fn load_receipts(state: &Path) -> Vec<Value> {
    let text = std::fs::read_to_string(journal(state)).unwrap();
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).expect(line))
        .collect()
}

#[test]
fn mcp_serve_refuses_missing_identity_env() {
    assert_locked_cksum();
    let out = estate_bin().args(["pack", "mcp-serve"]).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("refuse:mcp-pack")
            || stderr.contains("refuse:mcp-member")
            || stderr.contains("refuse:mcp-estate"),
        "{stderr}"
    );
    assert_locked_cksum();
}

#[test]
fn mcp_serve_complete_tool_invokes_estate_complete_for_orchestrator() {
    assert_locked_cksum();
    let dir = scratch("orch");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture().canonicalize().unwrap();
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();

    let mut child = estate_bin()
        .args(["pack", "mcp-serve"])
        .env("CELL_ESTATE_PACK", "research-crew")
        .env("CELL_ESTATE_MEMBER", "horizon")
        .env("CELL_ESTATE_ROLE", "orchestrator")
        .env("CELL_ESTATE_PATH", &estate_s)
        .env("CELL_ESTATE_STATE_DIR", &state_s)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    write_frame(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    );
    let init = read_frame(&mut stdout);
    assert_eq!(init["result"]["serverInfo"]["name"], "cell-one-pack-mcp");
    assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
    let instructions = init["result"]["instructions"].as_str().unwrap();
    assert!(instructions.contains("research-crew"), "{instructions}");
    assert!(instructions.contains("horizon"), "{instructions}");
    assert!(!instructions.contains("live PASS"), "{instructions}");

    write_frame(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    );

    write_frame(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    );
    let listed = read_frame(&mut stdout);
    let tools = listed["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "complete");
    let desc = tools[0]["description"].as_str().unwrap();
    assert!(desc.contains("estate complete"), "{desc}");
    assert!(desc.contains("research-crew"), "{desc}");

    write_frame(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "complete",
                "arguments": { "prompt": "ping", "mock": true }
            }
        }),
    );
    let called = read_frame(&mut stdout);
    assert_eq!(called["result"]["isError"], false, "{called}");
    let text = called["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("decision receipt:"), "{text}");
    assert!(!text.contains("live PASS"), "{text}");

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success(), "mcp-serve exit {status}");

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["schema"], "cell-one.decision-receipt.v0");
    assert_eq!(rows[0]["surface"], "complete");
    assert_eq!(rows[0]["pack_id"], "research-crew");
    assert_eq!(rows[0]["handoff_from"], "horizon");
    assert_eq!(rows[0]["handoff_to"], "research");
    assert_eq!(rows[0]["agent"], "research");
    assert_locked_cksum();
}

#[test]
fn mcp_serve_non_orchestrator_refuses_pack_complete() {
    assert_locked_cksum();
    let dir = scratch("member");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture().canonicalize().unwrap();

    let mut child = estate_bin()
        .args(["pack", "mcp-serve"])
        .env("CELL_ESTATE_PACK", "research-crew")
        .env("CELL_ESTATE_MEMBER", "research")
        .env("CELL_ESTATE_ROLE", "member")
        .env("CELL_ESTATE_PATH", estate.display().to_string())
        .env("CELL_ESTATE_STATE_DIR", state.display().to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    write_frame(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    );
    let _init = read_frame(&mut stdout);
    write_frame(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "complete",
                "arguments": { "prompt": "ping", "mock": true }
            }
        }),
    );
    let called = read_frame(&mut stdout);
    assert_eq!(called["result"]["isError"], true, "{called}");
    let text = called["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("refuse:pack-orchestrator"), "{text}");

    drop(stdin);
    let _ = child.wait();
    assert!(
        !journal(&state).is_file(),
        "no receipt on orchestrator refuse"
    );
    assert_locked_cksum();
}
