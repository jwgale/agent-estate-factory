//! `estate pack crew-session-prove` — multi-hop CLI crew session smoke
//! after a throwaway `plugin-install-local`.
//!
//! After the pack lands under a throwaway HOME, successive
//! `estate complete --mock` hops on one `session_id` share context:
//! hop 1 journals `context=none`, hop 2 journals `context=applied` and
//! the mock completion cites hop 1. Research stays
//! `refuse:pack-orchestrator` even when that session is bound (no
//! receipt, session turns stay 2). A Cursor MCP loader hang is out of
//! scope. Does not rewrite `examples/estate.yaml`.
//! `READY_FOR_LIVE_TEST` stays no.
//!
//! `estate pack cohesion-prove` composes this stage after fuel, decide,
//! run, and optional specialty-real, then the improvement-export
//! package stage.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::control_plane_prove;
use crate::pack_mcp;
use crate::plugin_install::{self, MARKER_FILE};

const LOCKED_CKSUM: &str = "43770130 3391";
const PROVE_SCHEMA: &str = "cell-one.crew-session-prove.v0";
const DEFAULT_PACK: &str = "research-crew";
const SMOKE_LIMIT: Duration = Duration::from_secs(60);
pub(crate) const SESSION_ID: &str = "sess-cohesion01";
pub(crate) const ISOLATION_ID: &str = "sess-cohesioniso";
pub(crate) const HOP1_TOKEN: &str = "unique-hop-alpha-token";
pub(crate) const HOP2_PROMPT: &str = "follow-up that should see prior turn";
const ISO_PROMPT: &str = "iso-first-hop";

pub(crate) struct InstallFacts {
    pub install_path: PathBuf,
    pub plugin_name: String,
    pub skills: Value,
    pub mcp_servers: Value,
    pub runner_docs: String,
    pub session_docs: String,
    pub loaded: String,
    pub estate_bin: String,
    pub cli_smoke_docs: bool,
}

pub(crate) struct PackStage {
    pub install: InstallFacts,
    pub smoke: Value,
}

pub(crate) fn cmd_crew_session_prove(
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
        bail!("refuse:estate: crew-session-prove does not use examples/estate.yaml");
    }
    let fixture_before = fs::read(&fixture)
        .with_context(|| format!("refuse:estate: {}", fixture.display()))?;

    let (out_hint, wipe) = match out {
        Some(path) => (path.to_path_buf(), false),
        None => (default_out(), true),
    };
    if control_plane_prove::refuses_locked_target(&out_hint, &root, &locked)? {
        bail!("refuse:out: crew-session-prove does not write examples/estate.yaml");
    }
    if wipe {
        let _ = fs::remove_dir_all(&out_hint);
    }
    fs::create_dir_all(&out_hint).with_context(|| format!("refuse:out: {}", out_hint.display()))?;
    let out = out_hint
        .canonicalize()
        .with_context(|| format!("refuse:out: {}", out_hint.display()))?;
    if control_plane_prove::refuses_locked_target(&out, &root, &locked)? {
        bail!("refuse:out: crew-session-prove does not write examples/estate.yaml");
    }

    println!("crew-session-prove: pack");
    println!("crew-session-prove: cli-smoke");
    let stage = run_pack_stage(pack_id, &fixture, &out)?;
    if fs::read(&locked)? != before || fs::read(&fixture)? != fixture_before {
        bail!("refuse:estate: crew-session-prove rewrote a source estate");
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
        "pack": pack_report(pack_id, &fixture, &out, &stage),
        "note": "Fixture prove. Throwaway plugin-install-local then multi-hop CLI crew session on one session_id. Hop 2 sees hop 1. Research stays refuse:pack-orchestrator. Cursor MCP loader hang is out of scope. Not a live PASS."
    });
    write_report(&out, &body)?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    println!("crew-session-prove: ok");
    println!("READY_FOR_LIVE_TEST: no");
    Ok(())
}

pub(crate) fn run_pack_stage(pack_id: &str, fixture: &Path, out: &Path) -> Result<PackStage> {
    let home = out.join("home");
    fs::create_dir_all(&home).with_context(|| format!("refuse:home: {}", home.display()))?;
    let export_out = out.join("plugin-export");
    let install = install_under_home(pack_id, fixture, &export_out, &home)?;

    let smoke_dir = out.join("cli-smoke");
    fs::create_dir_all(&smoke_dir)?;
    let smoke = run_crew_session_smoke(pack_id, fixture, &install, &smoke_dir, &home)?;
    Ok(PackStage { install, smoke })
}

pub(crate) fn write_composed_report(
    pack_id: &str,
    fixture: &Path,
    out: &Path,
    stage: &PackStage,
    estate_cksum: &str,
) -> Result<()> {
    let body = json!({
        "schema": PROVE_SCHEMA,
        "ok": true,
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "live_sync": false,
        "estate_cksum": estate_cksum,
        "composed_by": "cohesion-prove",
        "pack": pack_report(pack_id, fixture, out, stage),
        "note": "Composed by cohesion-prove. Throwaway plugin-install-local then multi-hop CLI crew session on one session_id. Hop 2 sees hop 1. Research stays refuse:pack-orchestrator. After gated apply, estate pack session show cites the nearby cell-one.improvement-apply.v0 receipt (local journal / state only). Cursor MCP loader hang is out of scope. Not a live PASS."
    });
    write_report(out, &body)
}

pub(crate) fn pack_report(pack_id: &str, fixture: &Path, out: &Path, stage: &PackStage) -> Value {
    let home = out.join("home");
    let export_out = out.join("plugin-export");
    let crew_state = out.join("cli-smoke").join("crew");
    let mut smoke = stage.smoke.clone();
    if let Some(cite) = crate::decisions::cite_nearby_apply_receipt(&crew_state) {
        smoke["session_show"]["cites_apply"] = json!(true);
        smoke["session_show"]["apply_cite"] = json!(cite);
    }
    json!({
        "id": pack_id,
        "fixture": fixture.display().to_string(),
        "home": home.display().to_string(),
        "export_out": export_out.display().to_string(),
        "install_path": stage.install.install_path.display().to_string(),
        "plugin_name": stage.install.plugin_name,
        "install_schema": plugin_install::INSTALL_SCHEMA,
        "skills": stage.install.skills,
        "mcp_servers": stage.install.mcp_servers,
        "runner_docs": stage.install.runner_docs,
        "session_docs": stage.install.session_docs,
        "loaded": stage.install.loaded,
        "cursor_loader": "out-of-scope",
        "loader_is_live_pass": false,
        "cli_smoke_docs": stage.install.cli_smoke_docs,
        "estate_bin": stage.install.estate_bin,
        "cli_smoke": smoke,
        "crew_session": smoke,
    })
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
        bail!("refuse:cli-smoke: INSTALL.md does not document multi-hop CLI crew session");
    }
    let readme = fs::read_to_string(dest.join("README.md")).unwrap_or_default();
    if !install_mentions_cli_smoke(&readme) {
        bail!("refuse:cli-smoke: README.md does not document multi-hop CLI crew session");
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

pub(crate) fn install_mentions_cli_smoke(text: &str) -> bool {
    text.contains("CLI smoke (first-class)")
        && text.contains("estate complete --mock")
        && text.contains("Cursor MCP loader hang is out of scope")
        && text.contains("refuse:pack-orchestrator")
        && text.contains("--session")
        && text.contains("context=applied")
        && text.contains(HOP1_TOKEN)
        && text.contains("same session_id")
        && text.contains("READY_FOR_LIVE_TEST: no")
        && !text.contains("READY_FOR_LIVE_TEST: yes")
}

pub(crate) fn run_crew_session_smoke(
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
        bail!("refuse:cli-smoke: research-crew agents are {orchestrator} and {member}");
    }

    let bin = PathBuf::from(&install.estate_bin);
    let crew_state = smoke_dir.join("crew");
    fs::create_dir_all(&crew_state)?;

    let (ok, stdout, stderr) = run_estate(
        &bin,
        &[
            "pack",
            "session",
            "create",
            "--pack",
            pack_id,
            "--estate",
            &fixture.display().to_string(),
            "--state-dir",
            &crew_state.display().to_string(),
            "--id",
            SESSION_ID,
        ],
        home,
    )?;
    if !ok || !stdout.contains(&format!("created id={SESSION_ID}")) {
        bail!("refuse:cli-smoke: session create failed\n{stderr}\n{stdout}");
    }
    println!("cli-smoke session: created id={SESSION_ID} pack={pack_id}");

    let hop1 = run_horizon_hop(
        &bin,
        &orchestrator,
        pack_id,
        fixture,
        &crew_state,
        home,
        SESSION_ID,
        HOP1_TOKEN,
        1,
        "none",
        false,
    )?;
    require_default_seat(pack_id, &hop1)?;
    println!(
        "cli-smoke {orchestrator} hop 1: decision receipt outcome=allow surface=complete pack={pack_id} capability={} result={} session={SESSION_ID} turns=1 context=none",
        hop1.capability, hop1.result
    );

    let hop2 = run_horizon_hop(
        &bin,
        &orchestrator,
        pack_id,
        fixture,
        &crew_state,
        home,
        SESSION_ID,
        HOP2_PROMPT,
        2,
        "applied",
        true,
    )?;
    require_default_seat(pack_id, &hop2)?;
    if !hop2.saw_prior {
        bail!(
            "refuse:cli-smoke: hop 2 did not cite hop 1 token {HOP1_TOKEN}: {}",
            hop2.completion
        );
    }
    println!(
        "cli-smoke {orchestrator} hop 2: decision receipt outcome=allow surface=complete pack={pack_id} capability={} result={} session={SESSION_ID} turns=2 context=applied saw_prior=yes",
        hop2.capability, hop2.result
    );

    let (ok, shown, stderr) = run_estate(
        &bin,
        &[
            "pack",
            "session",
            "show",
            "--id",
            SESSION_ID,
            "--pack",
            pack_id,
            "--state-dir",
            &crew_state.display().to_string(),
        ],
        home,
    )?;
    if !ok || !shown.contains("turns=2") || !shown.contains(HOP1_TOKEN) {
        bail!("refuse:cli-smoke: session show missing hop 1 cite\n{stderr}\n{shown}");
    }

    let (ok, report, stderr) = run_estate(
        &bin,
        &[
            "decisions",
            "report",
            "--state-dir",
            &crew_state.display().to_string(),
            "--pack",
            pack_id,
        ],
        home,
    )?;
    if !ok || !report.contains(&format!("session={SESSION_ID}")) || !report.contains("context=applied")
    {
        bail!("refuse:cli-smoke: decisions report missing session cite\n{stderr}\n{report}");
    }

    let session_path = crew_state
        .join("pack-sessions")
        .join(estate_schema::normalize_name(pack_id))
        .join(format!("{SESSION_ID}.json"));
    let session = read_json(&session_path)?;
    if session.get("schema").and_then(Value::as_str) != Some(crate::pack_session::SESSION_SCHEMA) {
        bail!("refuse:cli-smoke: session schema");
    }
    let turns = session
        .get("turns")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if turns.len() != 2 {
        bail!("refuse:cli-smoke: session has {} turns, want 2", turns.len());
    }
    if turns[0].get("prompt").and_then(Value::as_str) != Some(HOP1_TOKEN) {
        bail!("refuse:cli-smoke: session turn 1 prompt");
    }
    if session.get("session_id").and_then(Value::as_str) != Some(SESSION_ID) {
        bail!("refuse:cli-smoke: session file id");
    }

    let receipts_before_member = load_receipts(&crew_state)?;
    if receipts_before_member.len() != 2 {
        bail!(
            "refuse:cli-smoke: want 2 crew receipts before member refuse, got {}",
            receipts_before_member.len()
        );
    }

    let (ok, stdout, stderr) = run_complete(
        &bin,
        &member,
        pack_id,
        fixture,
        &crew_state,
        home,
        Some(SESSION_ID),
        "ping",
    )?;
    let combined = format!("{stdout}{stderr}");
    if ok || !combined.contains("refuse:pack-orchestrator") {
        bail!(
            "refuse:cli-smoke: {member} did not fail closed with refuse:pack-orchestrator\n{combined}"
        );
    }
    let receipts_after_member = load_receipts(&crew_state)?;
    if receipts_after_member.len() != receipts_before_member.len() {
        bail!("refuse:cli-smoke: {member} wrote a receipt on pack-orchestrator refuse");
    }
    let session_after = read_json(&session_path)?;
    let turns_after = session_after
        .get("turns")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    if turns_after != 2 {
        bail!("refuse:cli-smoke: {member} mutated session turns to {turns_after}");
    }
    println!("cli-smoke {member}: refuse:pack-orchestrator session_intact=yes");

    let iso = run_horizon_hop(
        &bin,
        &orchestrator,
        pack_id,
        fixture,
        &crew_state,
        home,
        ISOLATION_ID,
        ISO_PROMPT,
        1,
        "none",
        false,
    )?;
    if iso.completion.contains(HOP1_TOKEN) {
        bail!("refuse:cli-smoke: isolation session leaked hop 1 token");
    }
    if iso.session_id != ISOLATION_ID {
        bail!("refuse:cli-smoke: isolation session_id is {}", iso.session_id);
    }
    println!(
        "cli-smoke {orchestrator} isolation: session={ISOLATION_ID} turns=1 context=none leaked=no"
    );

    println!(
        "cli-smoke {orchestrator}: decision receipt outcome=allow surface=complete pack={pack_id} capability={} result={}",
        hop2.capability, hop2.result
    );

    Ok(json!({
        "command": "estate complete --mock",
        "session_id": SESSION_ID,
        "hops": [hop1.to_json(1), hop2.to_json(2)],
        "orchestrator": {
            "agent": orchestrator,
            "ok": true,
            "outcome": "allow",
            "surface": "complete",
            "pack_id": pack_id,
            "capability": hop2.capability,
            "result": hop2.result,
            "handoff_from": hop2.handoff_from,
            "handoff_to": hop2.handoff_to,
            "receipt": true,
            "session_id": SESSION_ID,
            "session_turns": 2,
            "context": "applied",
            "saw_prior": true,
        },
        "member": {
            "agent": member,
            "ok": true,
            "refuse": "refuse:pack-orchestrator",
            "receipt_written": false,
            "session_bound": true,
            "session_turns": 2,
        },
        "isolation": {
            "session_id": ISOLATION_ID,
            "context": "none",
            "leaked": false,
            "ok": true,
        },
        "session_show": {
            "turns": 2,
            "cites_hop1": true,
        },
        "decisions_report": {
            "cites_session": true,
            "cites_applied": true,
        },
    }))
}

struct HopFacts {
    capability: String,
    result: String,
    handoff_from: String,
    handoff_to: String,
    session_id: String,
    session_turns: u64,
    context: String,
    saw_prior: bool,
    completion: String,
}

impl HopFacts {
    fn to_json(&self, seq: u64) -> Value {
        json!({
            "seq": seq,
            "ok": true,
            "outcome": "allow",
            "surface": "complete",
            "capability": self.capability,
            "result": self.result,
            "handoff_from": self.handoff_from,
            "handoff_to": self.handoff_to,
            "session_id": self.session_id,
            "session_turns": self.session_turns,
            "context": self.context,
            "saw_prior": self.saw_prior,
            "receipt": true,
        })
    }
}

fn run_horizon_hop(
    bin: &Path,
    agent: &str,
    pack_id: &str,
    fixture: &Path,
    state: &Path,
    home: &Path,
    session_id: &str,
    prompt: &str,
    expect_turns: u64,
    expect_context: &str,
    expect_prior: bool,
) -> Result<HopFacts> {
    let before = load_receipts(state).unwrap_or_default();
    let (ok, stdout, stderr) =
        run_complete(bin, agent, pack_id, fixture, state, home, Some(session_id), prompt)?;
    if !ok {
        bail!("refuse:cli-smoke: {agent} hop failed\n{stderr}\n{stdout}");
    }
    if !stdout.contains("decision receipt:") {
        bail!("refuse:cli-smoke: {agent} stdout has no decision receipt\n{stdout}");
    }
    let want_status = format!(
        "pack session: id={session_id} turns={expect_turns} context={expect_context}"
    );
    if !stdout.contains(&want_status) {
        bail!("refuse:cli-smoke: missing {want_status}\n{stdout}");
    }
    let cite = format!("session={session_id} turns={expect_turns} context={expect_context}");
    if !stdout.contains(&cite) {
        bail!("refuse:cli-smoke: receipt cite missing {cite}\n{stdout}");
    }

    let receipts = load_receipts(state)?;
    if receipts.len() != before.len() + 1 {
        bail!(
            "refuse:cli-smoke: hop wrote {} receipts, want {}",
            receipts.len(),
            before.len() + 1
        );
    }
    let receipt = receipts.last().unwrap();
    if receipt.get("outcome").and_then(Value::as_str) != Some("allow") {
        bail!("refuse:cli-smoke: {agent} outcome is not allow");
    }
    if receipt.get("surface").and_then(Value::as_str) != Some("complete") {
        bail!("refuse:cli-smoke: {agent} surface is not complete");
    }
    if receipt.get("pack_id").and_then(Value::as_str) != Some(pack_id) {
        bail!("refuse:cli-smoke: {agent} pack_id");
    }
    if receipt.get("handoff_from").and_then(Value::as_str) != Some(agent) {
        bail!("refuse:cli-smoke: {agent} handoff_from");
    }
    if receipt.get("session_id").and_then(Value::as_str) != Some(session_id) {
        bail!("refuse:cli-smoke: {agent} session_id");
    }
    if receipt.get("session_turns").and_then(Value::as_u64) != Some(expect_turns) {
        bail!(
            "refuse:cli-smoke: {agent} session_turns want {expect_turns} got {}",
            receipt.get("session_turns").cloned().unwrap_or(Value::Null)
        );
    }
    let context_applied = receipt.get("session_context").and_then(Value::as_bool);
    let want_applied = expect_context == "applied";
    if context_applied != Some(want_applied) {
        bail!(
            "refuse:cli-smoke: {agent} session_context want {want_applied} got {context_applied:?}"
        );
    }
    let capability = receipt
        .get("capability")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let result_seat = receipt
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if capability.is_empty() || result_seat.is_empty() {
        bail!("refuse:cli-smoke: orchestrator receipt missing capability/result");
    }
    if result_seat == "local_slm" || capability == "local_slm" {
        bail!("refuse:cli-smoke: orchestrator complete named generic local_slm");
    }

    let completion = completion_body(&stdout);
    if expect_prior && !completion.contains(HOP1_TOKEN) {
        bail!("refuse:cli-smoke: hop completion missing prior token: {completion}");
    }
    if !expect_prior && prompt != HOP1_TOKEN && completion.contains(HOP1_TOKEN) {
        bail!("refuse:cli-smoke: hop leaked prior token into a fresh session: {completion}");
    }
    let saw_prior = expect_prior && completion.contains(HOP1_TOKEN);

    Ok(HopFacts {
        capability,
        result: result_seat,
        handoff_from: receipt
            .get("handoff_from")
            .and_then(Value::as_str)
            .unwrap_or(agent)
            .to_string(),
        handoff_to: receipt
            .get("handoff_to")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        session_id: session_id.to_string(),
        session_turns: expect_turns,
        context: expect_context.to_string(),
        saw_prior,
        completion,
    })
}

fn require_default_seat(pack_id: &str, hop: &HopFacts) -> Result<()> {
    if pack_id == DEFAULT_PACK && hop.capability != "ag_news" {
        bail!("refuse:cli-smoke: horizon capability is {}", hop.capability);
    }
    if pack_id == DEFAULT_PACK && hop.result != "ag_news" {
        bail!("refuse:cli-smoke: horizon result is {}", hop.result);
    }
    Ok(())
}

fn run_complete(
    bin: &Path,
    agent: &str,
    pack_id: &str,
    fixture: &Path,
    state: &Path,
    home: &Path,
    session: Option<&str>,
    prompt: &str,
) -> Result<(bool, String, String)> {
    let fixture_s = fixture.display().to_string();
    let state_s = state.display().to_string();
    let mut args = vec![
        "complete",
        "--mock",
        "--agent",
        agent,
        "--pack",
        pack_id,
        "--estate",
        &fixture_s,
        "--state-dir",
        &state_s,
        "--prompt",
        prompt,
    ];
    if let Some(id) = session {
        args.push("--session");
        args.push(id);
    }
    run_estate(bin, &args, home)
}

fn run_estate(bin: &Path, args: &[&str], home: &Path) -> Result<(bool, String, String)> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .env("HOME", home)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_LOCAL_LIVE")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .env_remove("CELL_PACK_SESSION")
        .stdin(Stdio::null());
    match pack_mcp::run_command_with_timeout(&mut cmd, SMOKE_LIMIT) {
        Ok(cap) if cap.timed_out => {
            bail!("refuse:cli-smoke: estate {} exceeded 60s", args.join(" "))
        }
        Ok(cap) => Ok((cap.success, cap.stdout, cap.stderr)),
        Err(err) => bail!("refuse:cli-smoke: {err}"),
    }
}

fn load_receipts(state: &Path) -> Result<Vec<Value>> {
    let path = state.join("decisions").join("receipts.jsonl");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("refuse:cli-smoke: read {}", path.display()))?;
    text.lines()
        .filter(|line| !line.is_empty())
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()
        .context("refuse:cli-smoke: parse receipt")
}

fn completion_body(stdout: &str) -> String {
    let start = match stdout.find("{\n") {
        Some(idx) => idx,
        None => return String::new(),
    };
    match serde_json::from_str::<Value>(stdout[start..].trim()) {
        Ok(body) => body
            .get("completion")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        Err(_) => String::new(),
    }
}

fn write_report(out: &Path, body: &Value) -> Result<()> {
    let pretty = serde_json::to_string_pretty(body)?;
    if pretty.split_whitespace().any(|word| word == "enforced") {
        bail!("refuse:crew-session: report invented enforced");
    }
    if pretty.contains("READY_FOR_LIVE_TEST: yes")
        || pretty.contains("\"ready_for_live_test\": true")
        || pretty.contains("\"live_pass_recorded\": true")
        || pretty.contains("\"live_sync\": true")
        || pretty.contains("\"loader_is_live_pass\": true")
    {
        bail!("refuse:crew-session: report invented a live-test ready flag");
    }
    fs::write(out.join("crew-session-prove.json"), format!("{pretty}\n"))?;
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
    std::env::temp_dir().join(format!("cell-one-crew-session-{token}"))
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
