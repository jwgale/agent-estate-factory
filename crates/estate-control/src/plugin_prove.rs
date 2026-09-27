//! `estate pack plugin-prove` — human gate before Cursor install.
//!
//! Exports a fixture pack (default research-crew), asserts baked
//! absolute `estate` binary in `mcp.json` (not bare `estate`), runs
//! mock MCP `complete` as orchestrator → receipt and as a member →
//! `refuse:pack-orchestrator`, asserts exported `RUNNER.md` + `SESSION.md`
//! are present and non-thin, and prints a compact prove report
//! (`runner_docs: yes` / `session_docs: yes`). Optional cheap session
//! two-hop (create + two complete hops) is included when an orchestrator
//! server is present. Exit non-zero on any required fail. live_sync
//! stays false. READY_FOR_LIVE_TEST: no. Not a live PASS.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::export_plugin::cmd_pack_export_plugin;
use crate::pack_mcp::{read_message, write_message, COMPLETE_TIMEOUT_ENV};

pub(crate) const PROVE_SCHEMA: &str = "cell-one.pack-plugin-prove.v0";
pub(crate) const DEFAULT_PACK_ID: &str = "research-crew";
pub(crate) const RUNNER_DOC: &str = "RUNNER.md";
pub(crate) const SESSION_DOC: &str = "SESSION.md";

/// Headings / CLI tokens / refuse codes required in exported RUNNER.md.
pub(crate) const RUNNER_DOC_NEEDLES: &[&str] = &[
    "# RUNNER",
    "estate routine runner start",
    "estate routine runner stop",
    "estate routine runner status",
    "estate routine runner restart",
    "{state-dir}/routine-runner/",
    "refuse:runner-already-running",
    "refuse:runner-routine-unknown",
    "refuse:runner-routine-disabled",
    "refuse:runner-routine-invalid",
    "estate routine runner-prove",
    "runner-prove --dual",
    "session_id",
    "package",
    "chain",
    "ticks",
    "READY_FOR_LIVE_TEST: no",
    "live_sync: false",
];

/// Headings / CLI tokens / refuse codes required in exported SESSION.md.
pub(crate) const SESSION_DOC_NEEDLES: &[&str] = &[
    "# SESSION",
    "estate pack session create",
    "estate pack session show",
    "estate pack session end",
    "context=none",
    "context=applied",
    "session_id",
    "routine-state.json",
    "ag_news",
    "rust_idiom",
    "frontier_http",
    "refuse:session-ended",
    "refuse:session-expired",
    "refuse:session-bound",
    "READY_FOR_LIVE_TEST: no",
    "live_sync: false",
];
#[allow(dead_code)]
pub(crate) const DEFAULT_PROMPT: &str = "ping";
#[allow(dead_code)]
pub(crate) const DEFAULT_ESTATE: &str = "examples/fixtures/agent-pack-handoff.yaml";

const RECEIPT_NEEDLE: &str = "decision receipt:";
const REFUSE_ORCH: &str = "refuse:pack-orchestrator";
const MCP_CALL_TIMEOUT: Duration = Duration::from_secs(45);
const SESSION_HOP1: &str = "plugin-prove-hop-alpha-token";
const SESSION_HOP2: &str = "plugin-prove-follow-up";
const SESSION_ID: &str = "sess-pluginprove01";

#[derive(Debug, Clone)]
pub(crate) struct ProveCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ProveReport {
    pub ok: bool,
    pub pack_id: String,
    pub out: String,
    pub estate_bin: String,
    pub runner_docs: bool,
    pub session_docs: bool,
    pub checks: Vec<ProveCheck>,
}

impl ProveReport {
    fn to_json(&self) -> Value {
        let mut checks = serde_json::Map::new();
        for c in &self.checks {
            checks.insert(
                c.name.clone(),
                json!({
                    "ok": c.ok,
                    "detail": c.detail,
                }),
            );
        }
        json!({
            "schema": PROVE_SCHEMA,
            "ok": self.ok,
            "pack_id": self.pack_id,
            "out": self.out,
            "estate_bin": self.estate_bin,
            "wired_mcp": true,
            "live_sync": false,
            "ready_for_live_test": false,
            "runner_docs": if self.runner_docs { "yes" } else { "no" },
            "session_docs": if self.session_docs { "yes" } else { "no" },
            "checks": Value::Object(checks),
        })
    }
}

pub(crate) fn cmd_pack_plugin_prove(
    id: &str,
    out: Option<&Path>,
    estate_path: &Path,
    prompt: &str,
    complete_timeout_secs: Option<u64>,
    check_only: bool,
) -> Result<()> {
    let timeout = crate::pack_mcp::resolve_complete_timeout(
        complete_timeout_secs,
        std::env::var(COMPLETE_TIMEOUT_ENV).ok(),
    )?;
    let timeout_secs = timeout.as_secs();
    let out_dir = match out {
        Some(p) => p.to_path_buf(),
        None if check_only => {
            bail!("refuse:plugin-prove-out: --check-only requires --out");
        }
        None => throwaway_out(id),
    };

    if !check_only {
        cmd_pack_export_plugin(id, &out_dir, estate_path, Some(timeout_secs))?;
    } else if !out_dir.is_dir() {
        bail!(
            "refuse:plugin-prove-out: --out '{}' is not a directory",
            out_dir.display()
        );
    }

    let report = prove_exported_plugin(&out_dir, prompt)?;
    print_report(&report);
    if !report.ok {
        bail!("refuse:plugin-prove: one or more checks failed");
    }
    Ok(())
}

fn throwaway_out(id: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("cell-pack-plugin-prove-{id}-{nanos}"))
}

pub(crate) fn print_report(report: &ProveReport) {
    println!("pack plugin-prove: {}", report.pack_id);
    println!("  ok: {}", if report.ok { "yes" } else { "no" });
    println!("  out: {}", report.out);
    if !report.estate_bin.is_empty() {
        println!("  estate_bin: {}", report.estate_bin);
    }
    for c in &report.checks {
        let mark = if c.ok { "ok" } else { "FAIL" };
        println!("  {}: {mark} ({})", c.name, c.detail);
    }
    println!(
        "  runner_docs: {}",
        if report.runner_docs { "yes" } else { "no" }
    );
    println!(
        "  session_docs: {}",
        if report.session_docs { "yes" } else { "no" }
    );
    println!("  live_sync: no");
    println!("  READY_FOR_LIVE_TEST: no");
    println!("{}", report.to_json());
}

/// Prove an already-exported plugin directory. Used by the CLI after
/// export and by tests that mutate `mcp.json` (bare `estate` fail).
pub(crate) fn prove_exported_plugin(out: &Path, prompt: &str) -> Result<ProveReport> {
    let mut checks = Vec::new();
    let mut estate_bin = String::new();
    let mut pack_id = String::new();

    let mapping_path = out.join("estate-pack.json");
    let mapping = match read_json(&mapping_path) {
        Ok(v) => {
            pack_id = v
                .get("pack_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            push_ok(
                &mut checks,
                "mapping",
                format!(
                    "schema={}",
                    v.get("schema").and_then(Value::as_str).unwrap_or("-")
                ),
            );
            Some(v)
        }
        Err(err) => {
            push_fail(&mut checks, "mapping", err.to_string());
            None
        }
    };

    if let Some(m) = mapping.as_ref() {
        match assert_export_flags(m) {
            Ok(()) => push_ok(
                &mut checks,
                "export_flags",
                "wired_mcp + live_sync false + skill_stub false",
            ),
            Err(err) => push_fail(&mut checks, "export_flags", err.to_string()),
        }
    }

    let mcp = match read_json(&out.join("mcp.json")) {
        Ok(v) => {
            push_ok(&mut checks, "mcp_json", "parsed");
            Some(v)
        }
        Err(err) => {
            push_fail(&mut checks, "mcp_json", err.to_string());
            None
        }
    };

    if let Some(mcp) = mcp.as_ref() {
        match assert_baked_estate_bin(mcp) {
            Ok(bin) => {
                estate_bin = bin.clone();
                push_ok(&mut checks, "absolute_estate_bin", bin);
            }
            Err(err) => push_fail(&mut checks, "absolute_estate_bin", err.to_string()),
        }
        match assert_timeout_env(mcp) {
            Ok(secs) => push_ok(
                &mut checks,
                "timeout_env",
                format!("{COMPLETE_TIMEOUT_ENV}={secs}"),
            ),
            Err(err) => push_fail(&mut checks, "timeout_env", err.to_string()),
        }
    }

    match assert_honest_labels(out) {
        Ok(()) => push_ok(&mut checks, "honest_labels", "no stub label when wired"),
        Err(err) => push_fail(&mut checks, "honest_labels", err.to_string()),
    }

    match assert_install_md(&out.join("INSTALL.md")) {
        Ok(()) => push_ok(&mut checks, "install_md", "INSTALL.md steps present"),
        Err(err) => push_fail(&mut checks, "install_md", err.to_string()),
    }

    let runner_docs = match assert_runner_md(&out.join(RUNNER_DOC)) {
        Ok(()) => {
            push_ok(
                &mut checks,
                "runner_docs",
                format!("{RUNNER_DOC} operator loop present"),
            );
            true
        }
        Err(err) => {
            push_fail(&mut checks, "runner_docs", err.to_string());
            false
        }
    };

    let session_docs = match assert_session_md(&out.join(SESSION_DOC)) {
        Ok(()) => {
            push_ok(
                &mut checks,
                "session_docs",
                format!("{SESSION_DOC} operator loop present"),
            );
            true
        }
        Err(err) => {
            push_fail(&mut checks, "session_docs", err.to_string());
            false
        }
    };

    let state_dir = out.join("prove-state");
    let _ = std::fs::create_dir_all(&state_dir);

    if let Some(mcp) = mcp.as_ref() {
        if let Some((name, server)) = server_with_role(mcp, "orchestrator") {
            match call_mcp_complete(server, prompt, &state_dir, None) {
                Ok((ok, text)) if ok && text.contains(RECEIPT_NEEDLE) => {
                    push_ok(
                        &mut checks,
                        "horizon_complete",
                        format!("{name}: {RECEIPT_NEEDLE}"),
                    );
                }
                Ok((ok, text)) => {
                    push_fail(
                        &mut checks,
                        "horizon_complete",
                        format!(
                            "{name}: expected receipt, ok={ok} text={}",
                            clip(&text, 240)
                        ),
                    );
                }
                Err(err) => push_fail(&mut checks, "horizon_complete", err.to_string()),
            }
        } else {
            push_fail(
                &mut checks,
                "horizon_complete",
                "no orchestrator server in mcp.json",
            );
        }

        if let Some((name, server)) = server_with_role(mcp, "member") {
            match call_mcp_complete(server, prompt, &state_dir, None) {
                Ok((ok, text)) if !ok && text.contains(REFUSE_ORCH) => {
                    push_ok(
                        &mut checks,
                        "research_complete",
                        format!("{name}: {REFUSE_ORCH}"),
                    );
                }
                Ok((ok, text)) => {
                    push_fail(
                        &mut checks,
                        "research_complete",
                        format!(
                            "{name}: expected {REFUSE_ORCH}, ok={ok} text={}",
                            clip(&text, 240)
                        ),
                    );
                }
                Err(err) => push_fail(&mut checks, "research_complete", err.to_string()),
            }
        } else {
            push_fail(
                &mut checks,
                "research_complete",
                "no member server in mcp.json",
            );
        }

        // Cheap session two-hop. Nice-to-have: reported, but a fail here
        // does not flip the prove (required checks stay horizon + research).
        if let Some((name, server)) = server_with_role(mcp, "orchestrator") {
            match call_mcp_session_two_hop(server, &state_dir) {
                Ok(()) => push_ok(
                    &mut checks,
                    "session_two_hop",
                    format!("{name}: {SESSION_ID} context=applied"),
                ),
                Err(err) => push_fail(&mut checks, "session_two_hop", err.to_string()),
            }
        }
    }

    if pack_id.is_empty() {
        pack_id = DEFAULT_PACK_ID.into();
    }
    let ok = checks
        .iter()
        .filter(|c| c.name != "session_two_hop")
        .all(|c| c.ok);
    Ok(ProveReport {
        ok,
        pack_id,
        out: out.display().to_string(),
        estate_bin,
        runner_docs,
        session_docs,
        checks,
    })
}

/// Fail closed when any mcp server `command` is bare `estate` or not
/// an absolute file path.
pub(crate) fn assert_baked_estate_bin(mcp: &Value) -> Result<String> {
    let servers = mcp
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("refuse:plugin-prove-estate-bin: missing mcpServers"))?;
    if servers.is_empty() {
        bail!("refuse:plugin-prove-estate-bin: no mcp servers");
    }
    let mut seen: Option<String> = None;
    for (name, server) in servers {
        let cmd = server
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if cmd.is_empty() || cmd == "estate" {
            bail!(
                "refuse:plugin-prove-estate-bin: server '{name}' command is bare 'estate' — export must bake an absolute path"
            );
        }
        let path = Path::new(cmd);
        if !path.is_absolute() {
            bail!(
                "refuse:plugin-prove-estate-bin: server '{name}' command '{cmd}' is not absolute"
            );
        }
        if !path.is_file() {
            bail!("refuse:plugin-prove-estate-bin: server '{name}' command '{cmd}' is not a file");
        }
        match &seen {
            Some(prev) if prev != cmd => {
                bail!(
                    "refuse:plugin-prove-estate-bin: servers disagree on estate bin ({prev} vs {cmd})"
                );
            }
            None => seen = Some(cmd.to_string()),
            _ => {}
        }
    }
    seen.ok_or_else(|| anyhow::anyhow!("refuse:plugin-prove-estate-bin: no command"))
}

pub(crate) fn assert_timeout_env(mcp: &Value) -> Result<String> {
    let servers = mcp
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("refuse:plugin-prove-timeout: missing mcpServers"))?;
    let mut seen: Option<String> = None;
    for (name, server) in servers {
        let raw = server
            .get("env")
            .and_then(|e| e.get(COMPLETE_TIMEOUT_ENV))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "refuse:plugin-prove-timeout: server '{name}' missing {COMPLETE_TIMEOUT_ENV}"
                )
            })?;
        let secs: u64 = raw.parse().map_err(|_| {
            anyhow::anyhow!(
                "refuse:plugin-prove-timeout: server '{name}' {COMPLETE_TIMEOUT_ENV}={raw} is not an integer"
            )
        })?;
        if secs < 1 {
            bail!(
                "refuse:plugin-prove-timeout: server '{name}' {COMPLETE_TIMEOUT_ENV} must be ≥ 1"
            );
        }
        match &seen {
            Some(prev) if prev != raw => {
                bail!("refuse:plugin-prove-timeout: servers disagree on timeout");
            }
            None => seen = Some(raw.to_string()),
            _ => {}
        }
    }
    seen.ok_or_else(|| anyhow::anyhow!("refuse:plugin-prove-timeout: no servers"))
}

pub(crate) fn assert_export_flags(mapping: &Value) -> Result<()> {
    if mapping.get("wired_mcp") != Some(&Value::Bool(true)) {
        bail!("refuse:plugin-prove-flags: wired_mcp must be true");
    }
    if mapping.get("live_sync") != Some(&Value::Bool(false)) {
        bail!("refuse:plugin-prove-flags: live_sync must be false");
    }
    if mapping.get("ready_for_live_test") == Some(&Value::Bool(true)) {
        bail!("refuse:plugin-prove-flags: READY_FOR_LIVE_TEST must stay no");
    }
    if let Some(pkgs) = mapping.get("packages").and_then(Value::as_array) {
        for pkg in pkgs {
            if pkg.get("skill_stub") != Some(&Value::Bool(false)) {
                bail!(
                    "refuse:plugin-prove-flags: package skill_stub must be false when MCP is wired"
                );
            }
        }
    }
    Ok(())
}

/// When MCP is wired and skills are non-stub, export artifacts must not
/// label themselves a "pack plugin stub".
pub(crate) fn assert_honest_labels(out: &Path) -> Result<()> {
    let mapping = read_json(&out.join("estate-pack.json")).unwrap_or_else(|_| json!({}));
    let wired = mapping.get("wired_mcp") == Some(&Value::Bool(true));
    let stubs = mapping
        .get("packages")
        .and_then(Value::as_array)
        .map(|pkgs| {
            pkgs.iter()
                .any(|p| p.get("skill_stub") == Some(&Value::Bool(true)))
        })
        .unwrap_or(false);
    if !wired || stubs {
        return Ok(());
    }
    let files = [
        "plugin.json",
        "README.md",
        "INSTALL.md",
        "RUNNER.md",
        "SESSION.md",
        "estate-pack.json",
        "mcp.json",
    ];
    for name in files {
        let path = out.join(name);
        if !path.is_file() {
            continue;
        }
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        if let Some(label) = stub_label_in(&text) {
            bail!(
                "refuse:plugin-prove-label: {} labels the export as '{label}' while wired_mcp is true and skill_stub is false",
                name
            );
        }
    }
    Ok(())
}

pub(crate) fn stub_label_in(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    for label in [
        "pack plugin stub",
        "agent plugin stub",
        "plugin stub",
        "body stub",
    ] {
        if lower.contains(label) {
            return Some(label);
        }
    }
    None
}

pub(crate) fn assert_install_md(path: &Path) -> Result<()> {
    if !path.is_file() {
        bail!("refuse:plugin-prove-install: missing INSTALL.md");
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    for needle in [
        "Plugin folder",
        "mcp.json",
        "complete",
        "horizon",
        "decision receipt",
        "cell-one.decision-receipt.v0",
        "research",
        "refuse:pack-orchestrator",
        "READY_FOR_LIVE_TEST: no",
        "plugin-install-local",
        ".cursor/plugins/local",
        ".estate-pack-install.json",
        "refuse:plugin-install-symlink",
        "loaded: skipped:loader-unavailable",
    ] {
        if !text.contains(needle) {
            bail!("refuse:plugin-prove-install: INSTALL.md missing '{needle}'");
        }
    }
    if text.contains("READY_FOR_LIVE_TEST: yes") {
        bail!("refuse:plugin-prove-install: INSTALL.md must not claim a live PASS");
    }
    if stub_label_in(&text).is_some() {
        bail!("refuse:plugin-prove-install: INSTALL.md must not use stub labels");
    }
    Ok(())
}

pub(crate) fn assert_runner_md(path: &Path) -> Result<()> {
    assert_operator_doc(path, "runner-docs", RUNNER_DOC, RUNNER_DOC_NEEDLES)
}

pub(crate) fn assert_session_md(path: &Path) -> Result<()> {
    assert_operator_doc(path, "session-docs", SESSION_DOC, SESSION_DOC_NEEDLES)
}

fn assert_operator_doc(path: &Path, kind: &str, label: &str, needles: &[&str]) -> Result<()> {
    if !path.is_file() {
        bail!("refuse:plugin-prove-{kind}: missing {label}");
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    for needle in needles {
        if !text.contains(*needle) {
            bail!("refuse:plugin-prove-{kind}: {label} missing '{needle}'");
        }
    }
    if text.contains("READY_FOR_LIVE_TEST: yes") {
        bail!("refuse:plugin-prove-{kind}: {label} must not claim a live PASS");
    }
    if stub_label_in(&text).is_some() {
        bail!("refuse:plugin-prove-{kind}: {label} must not use stub labels");
    }
    Ok(())
}

fn server_with_role<'a>(mcp: &'a Value, role: &str) -> Option<(String, &'a Value)> {
    let servers = mcp.get("mcpServers")?.as_object()?;
    servers.iter().find_map(|(name, server)| {
        let got = server
            .get("env")
            .and_then(|e| e.get("CELL_ESTATE_ROLE"))
            .and_then(Value::as_str)?;
        if got == role {
            Some((name.clone(), server))
        } else {
            None
        }
    })
}

fn spawn_mcp(server: &Value, state_dir: &Path) -> Result<(std::process::Child, std::process::ChildStdin, BufReader<std::process::ChildStdout>)> {
    let command = server
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("refuse:plugin-prove-mcp: missing command"))?;
    let args: Vec<String> = server
        .get("args")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let mut cmd = Command::new(command);
    cmd.args(&args)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .env_remove("CELL_LOCAL_LIVE")
        .env_remove("CELL_PACK_SESSION")
        .env("CELL_ESTATE_STATE_DIR", state_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(env) = server.get("env").and_then(Value::as_object) {
        for (k, v) in env {
            if let Some(val) = v.as_str() {
                cmd.env(k, val);
            }
        }
    }

    let mut child = cmd
        .spawn()
        .with_context(|| format!("spawn {command} for plugin-prove"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("refuse:plugin-prove-mcp: no stdin"))?;
    let stdout = BufReader::new(
        child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("refuse:plugin-prove-mcp: no stdout"))?,
    );
    Ok((child, stdin, stdout))
}

fn call_mcp_complete(
    server: &Value,
    prompt: &str,
    state_dir: &Path,
    extra: Option<Value>,
) -> Result<(bool, String)> {
    let (mut child, mut stdin, mut stdout) = spawn_mcp(server, state_dir)?;
    let started = Instant::now();
    let result = (|| -> Result<(bool, String)> {
        write_message(
            &mut stdin,
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        )?;
        let init = read_message(&mut stdout)?
            .ok_or_else(|| anyhow::anyhow!("refuse:plugin-prove-mcp: EOF on initialize"))?;
        if init.get("error").is_some() {
            bail!("refuse:plugin-prove-mcp: initialize error {init}");
        }
        let mut arguments = json!({ "prompt": prompt, "mock": true });
        if let Some(Value::Object(extra)) = extra {
            if let Some(obj) = arguments.as_object_mut() {
                for (k, v) in extra {
                    obj.insert(k, v);
                }
            }
        }
        write_message(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "complete",
                    "arguments": arguments
                }
            }),
        )?;
        let called = read_message(&mut stdout)?
            .ok_or_else(|| anyhow::anyhow!("refuse:plugin-prove-mcp: EOF on tools/call"))?;
        parse_complete_result(&called)
    })();

    drop(stdin);
    if started.elapsed() > MCP_CALL_TIMEOUT {
        let _ = child.kill();
    }
    let _ = child.wait();
    result
}

/// One process, two hops on a pack-scoped session. Cheap because the
/// transcript is a throwaway file under prove-state.
fn call_mcp_session_two_hop(server: &Value, state_dir: &Path) -> Result<()> {
    let (mut child, mut stdin, mut stdout) = spawn_mcp(server, state_dir)?;
    let started = Instant::now();
    let result = (|| -> Result<()> {
        write_message(
            &mut stdin,
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        )?;
        let init = read_message(&mut stdout)?
            .ok_or_else(|| anyhow::anyhow!("refuse:plugin-prove-session: EOF on initialize"))?;
        if init.get("error").is_some() {
            bail!("refuse:plugin-prove-session: initialize error {init}");
        }
        write_message(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "complete",
                    "arguments": {
                        "prompt": SESSION_HOP1,
                        "mock": true,
                        "session": SESSION_ID
                    }
                }
            }),
        )?;
        let first = read_message(&mut stdout)?
            .ok_or_else(|| anyhow::anyhow!("refuse:plugin-prove-session: EOF on hop 1"))?;
        let (ok1, text1) = parse_complete_result(&first)?;
        if !ok1 || !text1.contains(RECEIPT_NEEDLE) {
            bail!(
                "refuse:plugin-prove-session: hop 1 expected receipt, ok={ok1} text={}",
                clip(&text1, 240)
            );
        }
        write_message(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "complete",
                    "arguments": {
                        "prompt": SESSION_HOP2,
                        "mock": true,
                        "session": SESSION_ID
                    }
                }
            }),
        )?;
        let second = read_message(&mut stdout)?
            .ok_or_else(|| anyhow::anyhow!("refuse:plugin-prove-session: EOF on hop 2"))?;
        let (ok2, text2) = parse_complete_result(&second)?;
        if !ok2 || !text2.contains(RECEIPT_NEEDLE) {
            bail!(
                "refuse:plugin-prove-session: hop 2 expected receipt, ok={ok2} text={}",
                clip(&text2, 240)
            );
        }
        if !text2.contains("context=applied") {
            bail!(
                "refuse:plugin-prove-session: hop 2 missing context=applied: {}",
                clip(&text2, 240)
            );
        }
        if !text2.contains(SESSION_HOP1) {
            bail!(
                "refuse:plugin-prove-session: hop 2 did not see hop 1 token: {}",
                clip(&text2, 240)
            );
        }
        Ok(())
    })();

    drop(stdin);
    if started.elapsed() > MCP_CALL_TIMEOUT {
        let _ = child.kill();
    }
    let _ = child.wait();
    result
}

fn parse_complete_result(called: &Value) -> Result<(bool, String)> {
    let result = called.get("result").cloned().unwrap_or_else(|| called.clone());
    let is_error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let text = result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(|c| c.get("text"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Ok((!is_error, text))
}

fn read_json(path: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn push_ok(checks: &mut Vec<ProveCheck>, name: &str, detail: impl Into<String>) {
    checks.push(ProveCheck {
        name: name.into(),
        ok: true,
        detail: detail.into(),
    });
}

fn push_fail(checks: &mut Vec<ProveCheck>, name: &str, detail: impl Into<String>) {
    checks.push(ProveCheck {
        name: name.into(),
        ok: false,
        detail: detail.into(),
    });
}

fn clip(text: &str, max: usize) -> String {
    let one = text.replace('\n', " ");
    if one.len() <= max {
        one
    } else {
        format!("{}…", &one[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack_mcp::DEFAULT_COMPLETE_TIMEOUT_SECS;

    fn sample_mcp(command: &str) -> Value {
        json!({
            "mcpServers": {
                "horizon": {
                    "type": "stdio",
                    "command": command,
                    "args": ["pack", "mcp-serve"],
                    "env": {
                        "CELL_ESTATE_PACK": "research-crew",
                        "CELL_ESTATE_MEMBER": "horizon",
                        "CELL_ESTATE_ROLE": "orchestrator",
                        "CELL_ESTATE_PATH": "/tmp/e.yaml",
                        "CELL_MCP_COMPLETE_TIMEOUT_SECS": "120",
                    }
                },
                "research": {
                    "type": "stdio",
                    "command": command,
                    "args": ["pack", "mcp-serve"],
                    "env": {
                        "CELL_ESTATE_PACK": "research-crew",
                        "CELL_ESTATE_MEMBER": "research",
                        "CELL_ESTATE_ROLE": "member",
                        "CELL_ESTATE_PATH": "/tmp/e.yaml",
                        "CELL_MCP_COMPLETE_TIMEOUT_SECS": "120",
                    }
                }
            }
        })
    }

    #[test]
    fn assert_baked_estate_bin_refuses_bare_estate() {
        let mcp = sample_mcp("estate");
        let err = assert_baked_estate_bin(&mcp).unwrap_err().to_string();
        assert!(err.contains("refuse:plugin-prove-estate-bin"), "{err}");
        assert!(err.contains("bare 'estate'"), "{err}");
    }

    #[test]
    fn assert_baked_estate_bin_refuses_relative_path() {
        let mcp = sample_mcp("./estate");
        let err = assert_baked_estate_bin(&mcp).unwrap_err().to_string();
        assert!(err.contains("refuse:plugin-prove-estate-bin"), "{err}");
        assert!(
            err.contains("not absolute") || err.contains("bare 'estate'"),
            "{err}"
        );
    }

    #[test]
    fn assert_baked_estate_bin_accepts_current_exe() {
        let exe = std::env::current_exe().unwrap();
        let abs = exe.canonicalize().unwrap().display().to_string();
        let mcp = sample_mcp(&abs);
        let got = assert_baked_estate_bin(&mcp).unwrap();
        assert_eq!(got, abs);
    }

    #[test]
    fn stub_label_in_catches_pack_plugin_stub() {
        assert_eq!(
            stub_label_in("pack plugin stub: research-crew"),
            Some("pack plugin stub")
        );
        assert_eq!(stub_label_in("# crew plugin stub"), Some("plugin stub"));
        assert_eq!(
            stub_label_in("Agent Plugin stub from a pack"),
            Some("agent plugin stub")
        );
        assert!(stub_label_in("pack plugin: research-crew").is_none());
        assert!(stub_label_in("They are not stubs.").is_none());
        assert!(stub_label_in("Agent Plugin export").is_none());
        assert!(stub_label_in("\"skill_stub\": false").is_none());
    }

    #[test]
    fn assert_honest_labels_fails_when_wired_and_stub_titled() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-plugin-prove-label-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("estate-pack.json"),
            r#"{"wired_mcp":true,"live_sync":false,"packages":[{"skill_stub":false}]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("README.md"), "# research-crew plugin stub\n").unwrap();
        let err = assert_honest_labels(&dir).unwrap_err().to_string();
        assert!(err.contains("refuse:plugin-prove-label"), "{err}");
        assert!(err.contains("plugin stub"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn assert_timeout_env_requires_positive_secs() {
        let mut mcp = sample_mcp("/tmp/estate-bin");
        mcp["mcpServers"]["horizon"]["env"]["CELL_MCP_COMPLETE_TIMEOUT_SECS"] = json!("0");
        let err = assert_timeout_env(&mcp).unwrap_err().to_string();
        assert!(err.contains("refuse:plugin-prove-timeout"), "{err}");
    }

    #[test]
    fn prove_schema_and_defaults_are_stable() {
        assert_eq!(PROVE_SCHEMA, "cell-one.pack-plugin-prove.v0");
        assert_eq!(DEFAULT_PACK_ID, "research-crew");
        assert_eq!(DEFAULT_PROMPT, "ping");
        assert_eq!(DEFAULT_ESTATE, "examples/fixtures/agent-pack-handoff.yaml");
        assert_eq!(DEFAULT_COMPLETE_TIMEOUT_SECS, 120);
        let report = ProveReport {
            ok: true,
            pack_id: "research-crew".into(),
            out: "/tmp/out".into(),
            estate_bin: "/abs/estate".into(),
            runner_docs: true,
            session_docs: true,
            checks: vec![ProveCheck {
                name: "absolute_estate_bin".into(),
                ok: true,
                detail: "/abs/estate".into(),
            }],
        };
        let v = report.to_json();
        assert_eq!(v["schema"], PROVE_SCHEMA);
        assert_eq!(v["ready_for_live_test"], false);
        assert_eq!(v["live_sync"], false);
        assert_eq!(v["ok"], true);
        assert_eq!(v["runner_docs"], "yes");
        assert_eq!(v["session_docs"], "yes");
    }

    #[test]
    fn assert_runner_md_requires_substance() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-plugin-prove-runner-doc-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(RUNNER_DOC);
        let err = assert_runner_md(&path).unwrap_err().to_string();
        assert!(err.contains("refuse:plugin-prove-runner-docs"), "{err}");
        assert!(err.contains("missing"), "{err}");

        std::fs::write(&path, "# RUNNER\n\nthin\n").unwrap();
        let err = assert_runner_md(&path).unwrap_err().to_string();
        assert!(err.contains("refuse:plugin-prove-runner-docs"), "{err}");
        assert!(err.contains("missing"), "{err}");

        let full = crate::export_plugin::runner_markdown("research-crew", &[]);
        std::fs::write(&path, &full).unwrap();
        assert_runner_md(&path).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn assert_session_md_requires_substance() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-plugin-prove-session-doc-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(SESSION_DOC);
        let err = assert_session_md(&path).unwrap_err().to_string();
        assert!(err.contains("refuse:plugin-prove-session-docs"), "{err}");

        std::fs::write(&path, "# SESSION\n\nthin\n").unwrap();
        let err = assert_session_md(&path).unwrap_err().to_string();
        assert!(err.contains("refuse:plugin-prove-session-docs"), "{err}");
        assert!(err.contains("missing"), "{err}");

        let full = crate::export_plugin::session_markdown("research-crew");
        std::fs::write(&path, &full).unwrap();
        assert_session_md(&path).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
