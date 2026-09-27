//! `estate pack cohesion-prove` — one throwaway lab for fuel, decide, run,
//! and pack CLI smoke.
//!
//! Composes `estate control-plane-prove` (dual import-trained bind,
//! host-validate authorize / convey / complete --mock, `standing-dual`
//! runner stitch) with `estate pack plugin-install-local` under a
//! throwaway `HOME`, then smokes the baked estate binary:
//! `estate complete --mock` as the pack orchestrator (decision receipt,
//! outcome allow) and as a member (`refuse:pack-orchestrator`).
//! A Cursor MCP loader hang is out of scope. Does not rewrite
//! `examples/estate.yaml`. `READY_FOR_LIVE_TEST` stays no.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::control_plane_prove;
use crate::plugin_install::{self, MARKER_FILE};

const LOCKED_CKSUM: &str = "43770130 3391";
const PROVE_SCHEMA: &str = "cell-one.cohesion-prove.v0";
const CONTROL_SCHEMA: &str = "cell-one.control-plane-prove.v0";
const DEFAULT_PACK: &str = "research-crew";
const SMOKE_LIMIT: Duration = Duration::from_secs(60);

pub(crate) fn cmd_cohesion_prove(
    root: &Path,
    out: Option<&Path>,
    pack_id: &str,
    estate: &Path,
) -> Result<()> {
    std::env::remove_var("CELL_LOCAL_ENDPOINT");
    std::env::remove_var("CELL_LOCAL_LIVE");
    let root = root
        .canonicalize()
        .with_context(|| format!("refuse:root: {}", root.display()))?;
    let locked = root.join("examples/estate.yaml");
    let before = fs::read(&locked).with_context(|| format!("refuse:estate: {}", locked.display()))?;
    let cksum_before = file_cksum(&locked)?;
    if !cksum_before.starts_with(LOCKED_CKSUM) {
        bail!("refuse:estate: examples/estate.yaml cksum is {cksum_before}, want {LOCKED_CKSUM}");
    }

    let fixture = resolve_fixture(&root, estate)?;
    if same_file(&fixture, &locked)? {
        bail!("refuse:estate: cohesion-prove pack smoke does not use examples/estate.yaml");
    }
    let fixture_before = fs::read(&fixture)
        .with_context(|| format!("refuse:estate: {}", fixture.display()))?;

    let (out_hint, wipe) = match out {
        Some(path) => (path.to_path_buf(), false),
        None => (default_out(), true),
    };
    if control_plane_prove::refuses_locked_target(&out_hint, &root, &locked)? {
        bail!("refuse:out: cohesion-prove does not write examples/estate.yaml");
    }
    if wipe {
        let _ = fs::remove_dir_all(&out_hint);
    }
    fs::create_dir_all(&out_hint).with_context(|| format!("refuse:out: {}", out_hint.display()))?;
    let out = out_hint
        .canonicalize()
        .with_context(|| format!("refuse:out: {}", out_hint.display()))?;
    if control_plane_prove::refuses_locked_target(&out, &root, &locked)? {
        bail!("refuse:out: cohesion-prove does not write examples/estate.yaml");
    }

    println!("cohesion-prove: control-plane");
    control_plane_prove::cmd_control_plane_prove(&root, Some(&out))?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: control-plane rewrote examples/estate.yaml");
    }
    let control: Value = read_json(&out.join("control-plane-prove.json"))?;
    require_control_plane(&control)?;

    println!("cohesion-prove: pack");
    let home = out.join("home");
    fs::create_dir_all(&home).with_context(|| format!("refuse:home: {}", home.display()))?;
    let export_out = out.join("plugin-export");
    let install = install_under_home(pack_id, &fixture, &export_out, &home)?;
    if fs::read(&locked)? != before || fs::read(&fixture)? != fixture_before {
        bail!("refuse:estate: pack install rewrote a source estate");
    }

    println!("cohesion-prove: cli-smoke");
    let smoke_dir = out.join("cli-smoke");
    fs::create_dir_all(&smoke_dir)?;
    let smoke = cli_smoke(pack_id, &fixture, &install, &smoke_dir, &home)?;
    if fs::read(&locked)? != before || fs::read(&fixture)? != fixture_before {
        bail!("refuse:estate: cli smoke rewrote a source estate");
    }
    let cksum_after = file_cksum(&locked)?;
    if cksum_after != cksum_before {
        bail!("refuse:estate: examples/estate.yaml cksum changed to {cksum_after}");
    }

    let body = json!({
        "schema": PROVE_SCHEMA,
        "ok": true,
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "live_sync": false,
        "estate_cksum": cksum_after,
        "control_plane_schema": CONTROL_SCHEMA,
        "control_plane_report": out.join("control-plane-prove.json").display().to_string(),
        "fuel": control["fuel"].clone(),
        "decide": control["decide"].clone(),
        "run": control["run"].clone(),
        "pack": {
            "id": pack_id,
            "fixture": fixture.display().to_string(),
            "home": home.display().to_string(),
            "export_out": export_out.display().to_string(),
            "install_path": install.install_path.display().to_string(),
            "plugin_name": install.plugin_name,
            "install_schema": plugin_install::INSTALL_SCHEMA,
            "skills": install.skills,
            "mcp_servers": install.mcp_servers,
            "runner_docs": install.runner_docs,
            "session_docs": install.session_docs,
            "loaded": install.loaded,
            "cursor_loader": "out-of-scope",
            "loader_is_live_pass": false,
            "cli_smoke_docs": install.cli_smoke_docs,
            "estate_bin": install.estate_bin,
            "cli_smoke": smoke,
        },
        "note": "Fixture prove. Composes control-plane-prove with pack plugin-install-local and CLI smoke. Mocked GGUFs. Mock complete. Mock runner. Throwaway HOME. Cursor MCP loader hang is out of scope. Not a live PASS."
    });
    let pretty = serde_json::to_string_pretty(&body)?;
    if pretty.split_whitespace().any(|word| word == "enforced") {
        bail!("refuse:cohesion: report invented enforced");
    }
    if pretty.contains("READY_FOR_LIVE_TEST: yes")
        || pretty.contains("\"ready_for_live_test\": true")
        || pretty.contains("\"live_pass_recorded\": true")
        || pretty.contains("\"loader_is_live_pass\": true")
    {
        bail!("refuse:cohesion: report invented a live-test ready flag");
    }
    fs::write(out.join("cohesion-prove.json"), format!("{pretty}\n"))?;
    println!("{pretty}");
    println!("cohesion-prove: ok");
    println!("READY_FOR_LIVE_TEST: no");
    Ok(())
}

struct InstallFacts {
    install_path: PathBuf,
    plugin_name: String,
    skills: Value,
    mcp_servers: Value,
    runner_docs: String,
    session_docs: String,
    loaded: String,
    estate_bin: String,
    cli_smoke_docs: bool,
}

fn install_under_home(pack_id: &str, fixture: &Path, export_out: &Path, home: &Path) -> Result<InstallFacts> {
    let _home = EnvSwap::set("HOME", home.as_os_str());
    let _loader = EnvSwap::remove("CELL_CURSOR_PLUGINS_MODULE");
    plugin_install::cmd_pack_plugin_install_local(
        pack_id,
        Some(export_out),
        None,
        fixture,
        "ping",
        None,
        false,
        false,
    )?;
    drop(_loader);
    drop(_home);

    let plugin = read_json(&export_out.join("plugin.json"))?;
    let plugin_name = plugin
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(pack_id)
        .to_string();
    let dest = home
        .join(".cursor")
        .join("plugins")
        .join("local")
        .join(&plugin_name);
    let dest_c = dest
        .canonicalize()
        .with_context(|| format!("refuse:plugin-install: {}", dest.display()))?;
    let home_c = home.canonicalize()?;
    if !dest_c.starts_with(&home_c) {
        bail!("refuse:plugin-install: install escaped the throwaway HOME");
    }
    if dest.symlink_metadata()?.file_type().is_symlink() {
        bail!("refuse:plugin-install-symlink: cohesion install must be a real directory");
    }
    let marker = read_json(&dest.join(MARKER_FILE))?;
    if marker.get("schema").and_then(Value::as_str) != Some(plugin_install::INSTALL_SCHEMA) {
        bail!("refuse:plugin-install: marker schema");
    }
    if marker.get("pack_id").and_then(Value::as_str) != Some(pack_id) {
        bail!("refuse:plugin-install: marker pack_id");
    }
    if marker.get("live_sync") != Some(&Value::Bool(false))
        || marker.get("ready_for_live_test") == Some(&Value::Bool(true))
    {
        bail!("refuse:plugin-install: marker invented a live sync or live PASS");
    }
    let install_md = fs::read_to_string(dest.join("INSTALL.md"))
        .with_context(|| format!("refuse:cli-smoke: read {}", dest.join("INSTALL.md").display()))?;
    let cli_smoke_docs = install_mentions_cli_smoke(&install_md);
    if !cli_smoke_docs {
        bail!("refuse:cli-smoke: INSTALL.md does not document CLI smoke as first-class");
    }
    let readme = fs::read_to_string(dest.join("README.md")).unwrap_or_default();
    if !install_mentions_cli_smoke(&readme) {
        bail!("refuse:cli-smoke: README.md does not document CLI smoke as first-class");
    }
    let estate_bin = marker
        .get("estate_bin")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if !Path::new(&estate_bin).is_absolute() || !Path::new(&estate_bin).is_file() {
        bail!("refuse:cli-smoke: installed estate_bin is not an absolute file");
    }
    println!(
        "pack install: {} loaded={}",
        dest.display(),
        marker.get("loaded").and_then(Value::as_str).unwrap_or("-")
    );
    Ok(InstallFacts {
        install_path: dest,
        plugin_name,
        skills: marker.get("skills").cloned().unwrap_or(json!([])),
        mcp_servers: marker.get("mcp_servers").cloned().unwrap_or(json!([])),
        runner_docs: marker
            .get("runner_docs")
            .and_then(Value::as_str)
            .unwrap_or("no")
            .to_string(),
        session_docs: marker
            .get("session_docs")
            .and_then(Value::as_str)
            .unwrap_or("no")
            .to_string(),
        loaded: marker
            .get("loaded")
            .and_then(Value::as_str)
            .unwrap_or("skipped:loader-unavailable")
            .to_string(),
        estate_bin,
        cli_smoke_docs,
    })
}

fn install_mentions_cli_smoke(text: &str) -> bool {
    text.contains("CLI smoke (first-class)")
        && text.contains("estate complete --mock")
        && text.contains("Cursor MCP loader hang is out of scope")
        && text.contains("refuse:pack-orchestrator")
        && text.contains("READY_FOR_LIVE_TEST: no")
        && !text.contains("READY_FOR_LIVE_TEST: yes")
}

fn cli_smoke(
    pack_id: &str,
    fixture: &Path,
    install: &InstallFacts,
    smoke_dir: &Path,
    home: &Path,
) -> Result<Value> {
    let estate = estate_schema::load_estate(fixture)
        .with_context(|| format!("refuse:estate: load {}", fixture.display()))?;
    let pack = estate.pack(pack_id).ok_or_else(|| {
        anyhow::anyhow!("refuse:unknown-pack: pack '{pack_id}' not on estate")
    })?;
    let orchestrator = pack.orchestrator.clone().ok_or_else(|| {
        anyhow::anyhow!("refuse:cli-smoke: pack '{pack_id}' has no orchestrator")
    })?;
    let member = pack
        .members
        .iter()
        .find(|name| {
            estate_schema::normalize_name(name) != estate_schema::normalize_name(&orchestrator)
        })
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("refuse:cli-smoke: pack '{pack_id}' has no member"))?;
    if pack_id == DEFAULT_PACK && (orchestrator != "horizon" || member != "research") {
        bail!(
            "refuse:cli-smoke: research-crew agents are {orchestrator} and {member}"
        );
    }

    let bin = PathBuf::from(&install.estate_bin);
    let horizon_state = smoke_dir.join("horizon");
    fs::create_dir_all(&horizon_state)?;
    let (ok, stdout, stderr) = run_complete(
        &bin,
        &orchestrator,
        pack_id,
        fixture,
        &horizon_state,
        home,
    )?;
    if !ok {
        bail!(
            "refuse:cli-smoke: {orchestrator} complete failed\n{stderr}\n{stdout}"
        );
    }
    if !stdout.contains("decision receipt:") {
        bail!("refuse:cli-smoke: {orchestrator} stdout has no decision receipt\n{stdout}");
    }
    let receipt = load_one_receipt(&horizon_state)?;
    if receipt.get("outcome").and_then(Value::as_str) != Some("allow") {
        bail!("refuse:cli-smoke: {orchestrator} outcome is not allow");
    }
    if receipt.get("surface").and_then(Value::as_str) != Some("complete") {
        bail!("refuse:cli-smoke: {orchestrator} surface is not complete");
    }
    if receipt.get("pack_id").and_then(Value::as_str) != Some(pack_id) {
        bail!("refuse:cli-smoke: {orchestrator} pack_id");
    }
    if receipt.get("handoff_from").and_then(Value::as_str) != Some(orchestrator.as_str()) {
        bail!("refuse:cli-smoke: {orchestrator} handoff_from");
    }
    let capability = receipt
        .get("capability")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if pack_id == DEFAULT_PACK && capability != "ag_news" {
        bail!("refuse:cli-smoke: horizon capability is {capability}");
    }
    if capability.is_empty() {
        bail!("refuse:cli-smoke: orchestrator receipt has no capability");
    }
    println!(
        "cli-smoke {orchestrator}: decision receipt outcome=allow surface=complete pack={pack_id} capability={capability}"
    );

    let member_state = smoke_dir.join("member");
    fs::create_dir_all(&member_state)?;
    let (ok, stdout, stderr) =
        run_complete(&bin, &member, pack_id, fixture, &member_state, home)?;
    let combined = format!("{stdout}{stderr}");
    if ok || !combined.contains("refuse:pack-orchestrator") {
        bail!(
            "refuse:cli-smoke: {member} did not fail closed with refuse:pack-orchestrator\n{combined}"
        );
    }
    let journal = member_state.join("decisions").join("receipts.jsonl");
    if journal.is_file() {
        bail!("refuse:cli-smoke: {member} wrote a receipt on pack-orchestrator refuse");
    }
    println!("cli-smoke {member}: refuse:pack-orchestrator");

    Ok(json!({
        "command": "estate complete --mock",
        "orchestrator": {
            "agent": orchestrator,
            "ok": true,
            "outcome": "allow",
            "surface": "complete",
            "pack_id": pack_id,
            "capability": capability,
            "handoff_from": receipt.get("handoff_from").and_then(Value::as_str).unwrap_or(""),
            "handoff_to": receipt.get("handoff_to").and_then(Value::as_str).unwrap_or(""),
            "receipt": true,
        },
        "member": {
            "agent": member,
            "ok": true,
            "refuse": "refuse:pack-orchestrator",
            "receipt_written": false,
        },
    }))
}

fn run_complete(
    bin: &Path,
    agent: &str,
    pack_id: &str,
    fixture: &Path,
    state: &Path,
    home: &Path,
) -> Result<(bool, String, String)> {
    let mut cmd = Command::new(bin);
    cmd.args([
        "complete",
        "--mock",
        "--agent",
        agent,
        "--pack",
        pack_id,
        "--estate",
        &fixture.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--prompt",
        "ping",
    ])
    .env("HOME", home)
    .env_remove("XAI_API_KEY")
    .env_remove("CELL_LOCAL_ENDPOINT")
    .env_remove("CELL_LOCAL_LIVE")
    .env_remove("CELL_FRONTIER_ENDPOINT")
    .env_remove("CELL_FRONTIER_MODEL")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .with_context(|| format!("refuse:cli-smoke: spawn {}", bin.display()))?;
    let start = Instant::now();
    loop {
        match child.try_wait()? {
            Some(status) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut out) = child.stdout.take() {
                    out.read_to_string(&mut stdout)?;
                }
                if let Some(mut err) = child.stderr.take() {
                    err.read_to_string(&mut stderr)?;
                }
                return Ok((status.success(), stdout, stderr));
            }
            None if start.elapsed() > SMOKE_LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                bail!("refuse:cli-smoke: estate complete --mock exceeded 60s");
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

fn load_one_receipt(state: &Path) -> Result<Value> {
    let path = state.join("decisions").join("receipts.jsonl");
    let text = fs::read_to_string(&path)
        .with_context(|| format!("refuse:cli-smoke: read {}", path.display()))?;
    let rows: Vec<Value> = text
        .lines()
        .filter(|line| !line.is_empty())
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()
        .context("refuse:cli-smoke: parse receipt")?;
    if rows.len() != 1 {
        bail!("refuse:cli-smoke: want 1 receipt, got {}", rows.len());
    }
    Ok(rows.into_iter().next().unwrap())
}

fn require_control_plane(cp: &Value) -> Result<()> {
    if cp.get("schema").and_then(Value::as_str) != Some(CONTROL_SCHEMA) {
        bail!("refuse:cohesion: control-plane schema");
    }
    if cp.get("ok") != Some(&Value::Bool(true))
        || cp.get("ready_for_live_test") != Some(&Value::Bool(false))
        || cp.get("live_pass_recorded") != Some(&Value::Bool(false))
        || cp.get("live_sync") != Some(&Value::Bool(false))
    {
        bail!("refuse:cohesion: control-plane report is not a mock ok");
    }
    if cp["fuel"]["trained_shape"] != "gguf"
        || cp["fuel"]["auto_apply"] != false
        || cp["fuel"]["beside"] != "local_slm"
        || cp["fuel"]["joinable"]["ag_news"] != true
        || cp["fuel"]["joinable"]["rust_idiom"] != true
        || cp["fuel"]["seat_models"]["ag_news"] != "specialist-agnews-3000"
        || cp["fuel"]["seat_models"]["rust_idiom"] != "specialist-rustidiom-3000"
    {
        bail!("refuse:cohesion: fuel join is not the dual gguf bind");
    }
    if cp["decide"]["surface_authorize"] != 4
        || cp["decide"]["surface_convey"] != 4
        || cp["decide"]["surface_complete"] != 5
        || cp["decide"]["abstain"] != "refuse:decision-abstain"
        || cp["decide"]["stale_fallback"] != "ag_news"
        || cp["decide"]["ineligible_fallback"] != "ag_news"
    {
        bail!("refuse:cohesion: decide surfaces are not the host-validate counts");
    }
    let caps = cp["decide"]["capabilities"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let has = |name: &str| caps.iter().any(|row| row.as_str() == Some(name));
    if !has("ag_news") || !has("rust_idiom") {
        bail!("refuse:cohesion: decide capabilities missing specialty seats");
    }
    if cp["run"]["routine_id"] != "standing-dual"
        || cp["run"]["session_stitch"] != true
        || cp["run"]["package"] != "dual-specialty"
    {
        bail!("refuse:cohesion: run is not the standing-dual session stitch");
    }
    let chain = cp["run"]["chain_id"].as_str().unwrap_or("");
    if !chain.starts_with("chain-dual-specialty-") {
        bail!("refuse:cohesion: chain_id is {chain}");
    }
    let hops = cp["run"]["hops"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("refuse:cohesion: run hops missing"))?;
    let session = hops
        .first()
        .and_then(|row| row.get("session_id"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if session.is_empty() {
        bail!("refuse:cohesion: runner session_id is empty");
    }
    if hops.len() != 3
        || hops[0]["capability"] != "ag_news"
        || hops[0]["context"] != "none"
        || hops[1]["capability"] != "rust_idiom"
        || hops[1]["context"] != "applied"
        || hops[2]["capability"] != "frontier_http"
        || hops[2]["context"] != "applied"
        || hops[0]["session_id"] != hops[1]["session_id"]
        || hops[0]["session_id"] != hops[2]["session_id"]
    {
        bail!("refuse:cohesion: runner hops are not the 3-hop session stitch");
    }
    println!(
        "control-plane cited: joinable ag_news=yes rust_idiom=yes surfaces authorize=4 convey=4 complete=5 session_stitch=yes"
    );
    Ok(())
}

fn resolve_fixture(root: &Path, estate: &Path) -> Result<PathBuf> {
    let path = if estate.is_absolute() {
        estate.to_path_buf()
    } else {
        root.join(estate)
    };
    path.canonicalize()
        .with_context(|| format!("refuse:estate: {}", path.display()))
}

fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("refuse:read: {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("refuse:read: parse {}", path.display()))
}

fn file_cksum(path: &Path) -> Result<String> {
    let out = Command::new("cksum")
        .arg(path)
        .output()
        .context("refuse:estate: cksum")?;
    if !out.status.success() {
        bail!(
            "refuse:estate: cksum failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn same_file(left: &Path, right: &Path) -> Result<bool> {
    if left == right {
        return Ok(true);
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(a), Ok(b)) => Ok(a == b),
        _ => Ok(false),
    }
}

fn default_out() -> PathBuf {
    let mut token = std::process::id().to_string();
    for needle in ["5090", "4090", "4080", "3090"] {
        token = token.replace(needle, "0000");
    }
    std::env::temp_dir().join(format!("cell-one-cohesion-{token}"))
}

struct EnvSwap {
    key: &'static str,
    prev: Option<std::ffi::OsString>,
}

impl EnvSwap {
    fn set(key: &'static str, value: &std::ffi::OsStr) -> Self {
        let prev = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, prev }
    }

    fn remove(key: &'static str) -> Self {
        let prev = std::env::var_os(key);
        std::env::remove_var(key);
        Self { key, prev }
    }
}

impl Drop for EnvSwap {
    fn drop(&mut self) {
        match self.prev.take() {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}
