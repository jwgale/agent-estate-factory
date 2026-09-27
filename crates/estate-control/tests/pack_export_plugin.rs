//! `estate pack export-plugin` — Agent Plugin export from a pack.
//!
//! Fixture: examples/fixtures/agent-pack-handoff.yaml.
//! MCP is wired to `estate pack mcp-serve` → `estate complete`.
//! `mcp.json` `command` is the absolute estate binary resolved at export.
//! Env writes CELL_MCP_COMPLETE_TIMEOUT_SECS. INSTALL.md is the human
//! Cursor smoke checklist. RUNNER.md + SESSION.md teach the supervised
//! runner and pack session loop. Skill bodies instruct calling tool
//! `complete` with the package prompt (not stubs). When wired, labels
//! say pack plugin / Agent Plugin export — not "pack plugin stub".
//! live_sync stays false. Does not invent a live PASS.
//! Locked examples/estate.yaml stays untouched.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("cell-pack-export-plugin-{name}-{nanos}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args(args)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .env_remove("CELL_LOCAL_LIVE")
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
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

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn export_plugin_writes_agent_plugin_stub_from_fixture_pack() {
    assert_locked_cksum();
    let dir = scratch("ok");
    let out = dir.join("plugin");
    let estate = fixture().display().to_string();
    let out_s = out.display().to_string();
    let expected_bin = std::fs::canonicalize(env!("CARGO_BIN_EXE_estate"))
        .unwrap()
        .display()
        .to_string();

    let (ok, stdout, stderr) = run(&[
        "pack",
        "export-plugin",
        "--id",
        "research-crew",
        "--estate",
        &estate,
        "--out",
        &out_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("pack plugin: research-crew"), "{stdout}");
    assert!(!stdout.contains("pack plugin stub"), "{stdout}");
    assert!(!stdout.contains("plugin stub"), "{stdout}");
    assert!(stdout.contains("orchestrator: horizon"), "{stdout}");
    assert!(stdout.contains("members: horizon, research"), "{stdout}");
    assert!(
        stdout.contains("skills: classify-once, classify-ping"),
        "{stdout}"
    );
    assert!(stdout.contains("standing-classify @hourly"), "{stdout}");
    assert!(stdout.contains("standing-once @hourly"), "{stdout}");
    assert!(stdout.contains("wired_mcp: yes"), "{stdout}");
    assert!(stdout.contains("live_sync: no"), "{stdout}");
    assert!(stdout.contains("complete_timeout_secs: 120"), "{stdout}");
    assert!(stdout.contains("INSTALL.md"), "{stdout}");
    assert!(stdout.contains("RUNNER.md"), "{stdout}");
    assert!(stdout.contains("SESSION.md"), "{stdout}");
    assert!(
        stdout.contains(&format!("estate_bin: {expected_bin}")),
        "{stdout}"
    );
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!stderr.contains("live PASS"), "{stderr}");

    let plugin: serde_json::Value = serde_json::from_str(&read(&out.join("plugin.json"))).unwrap();
    assert_eq!(plugin["name"], "research-crew");
    assert_eq!(plugin["version"], "0.0.0");
    let desc = plugin["description"].as_str().unwrap();
    assert!(desc.contains("horizon"), "{desc}");
    assert!(desc.contains("research"), "{desc}");
    assert!(desc.contains("Orchestrator: horizon"), "{desc}");
    assert!(desc.contains("not live"), "{desc}");
    assert!(desc.contains("complete"), "{desc}");
    assert!(desc.contains("Agent Plugin"), "{desc}");
    assert!(!desc.contains("stub"), "{desc}");

    let mcp: serde_json::Value = serde_json::from_str(&read(&out.join("mcp.json"))).unwrap();
    let servers = mcp["mcpServers"].as_object().expect("mcpServers");
    assert!(servers.contains_key("horizon"), "{servers:?}");
    assert!(servers.contains_key("research"), "{servers:?}");
    assert_eq!(servers["horizon"]["command"], expected_bin);
    assert_eq!(servers["research"]["command"], expected_bin);
    assert!(
        Path::new(servers["horizon"]["command"].as_str().unwrap()).is_absolute(),
        "{}",
        servers["horizon"]["command"]
    );
    assert_ne!(servers["horizon"]["command"], "estate");
    assert_eq!(servers["horizon"]["type"], "stdio");
    assert_eq!(
        servers["horizon"]["args"],
        serde_json::json!(["pack", "mcp-serve"])
    );
    assert_eq!(
        servers["horizon"]["env"]["CELL_ESTATE_ROLE"],
        "orchestrator"
    );
    assert_eq!(
        servers["horizon"]["env"]["CELL_ESTATE_PACK"],
        "research-crew"
    );
    assert_eq!(servers["horizon"]["env"]["CELL_ESTATE_MEMBER"], "horizon");
    assert_eq!(
        servers["horizon"]["env"]["CELL_MCP_COMPLETE_TIMEOUT_SECS"],
        "120"
    );
    assert_eq!(
        servers["research"]["env"]["CELL_MCP_COMPLETE_TIMEOUT_SECS"],
        "120"
    );
    assert_eq!(servers["research"]["env"]["CELL_ESTATE_ROLE"], "member");
    assert_eq!(servers["research"]["env"]["CELL_ESTATE_MEMBER"], "research");
    let estate_env = servers["horizon"]["env"]["CELL_ESTATE_PATH"]
        .as_str()
        .unwrap();
    assert!(
        estate_env.ends_with("agent-pack-handoff.yaml"),
        "{estate_env}"
    );
    assert!(
        std::path::Path::new(estate_env).is_absolute(),
        "{estate_env}"
    );
    assert_eq!(
        servers["research"]["env"]["CELL_ESTATE_PATH"],
        servers["horizon"]["env"]["CELL_ESTATE_PATH"]
    );
    let mcp_text = read(&out.join("mcp.json"));
    assert!(mcp_text.contains("mcp-serve"), "{mcp_text}");
    assert!(!mcp_text.contains("\"command\": \"true\""), "{mcp_text}");

    let ping = read(&out.join("skills/classify-ping/SKILL.md"));
    assert!(ping.contains("name: classify-ping"), "{ping}");
    assert!(ping.contains("pack: research-crew"), "{ping}");
    assert!(ping.contains("binding: ag_news"), "{ping}");
    assert!(ping.contains("prompt: ping"), "{ping}");
    assert!(ping.contains("research -> ag_news"), "{ping}");
    assert!(ping.contains("horizon -> frontier_http"), "{ping}");
    assert!(
        ping.contains("Call the wired pack member MCP tool `complete`"),
        "{ping}"
    );
    assert!(
        ping.contains(r#"{ "prompt": "ping", "mock": true, "object": "ag_news" }"#),
        "{ping}"
    );
    assert!(
        ping.contains(r#"{ "prompt": "ping", "object": "ag_news" }"#),
        "{ping}"
    );
    assert!(ping.contains("estate pack mcp-serve"), "{ping}");
    assert!(ping.contains("estate complete"), "{ping}");
    assert!(ping.contains("MCP"), "{ping}");
    assert!(ping.contains("live_sync: false"), "{ping}");
    assert!(
        ping.contains("estate package run --id classify-ping"),
        "{ping}"
    );
    assert!(!ping.contains("stays a stub"), "{ping}");
    assert!(!ping.contains("body stub"), "{ping}");
    assert!(!ping.contains("Scaffold only"), "{ping}");
    assert!(ping.contains("standing-classify"), "{ping}");
    assert!(ping.contains("schedule=@hourly"), "{ping}");
    assert!(ping.contains("0 * * * *"), "{ping}");

    let once = read(&out.join("skills/classify-once/SKILL.md"));
    assert!(once.contains("name: classify-once"), "{once}");
    assert!(
        once.contains("Call the wired pack member MCP tool `complete`"),
        "{once}"
    );
    assert!(once.contains("MCP"), "{once}");
    assert!(once.contains("estate complete"), "{once}");
    assert!(!once.contains("stays a stub"), "{once}");
    assert!(!once.contains("body stub"), "{once}");
    assert!(once.contains("standing-once"), "{once}");
    assert!(once.contains("schedule=@hourly"), "{once}");
    assert!(!once.contains("no standing routines"), "{once}");

    let mapping: serde_json::Value =
        serde_json::from_str(&read(&out.join("estate-pack.json"))).unwrap();
    assert_eq!(mapping["schema"], "cell-one.pack-plugin-export.v0");
    assert_eq!(mapping["pack_id"], "research-crew");
    assert_eq!(mapping["group"]["orchestrator"], "horizon");
    assert_eq!(mapping["live_sync"], false);
    assert_eq!(mapping["wired_mcp"], true);
    assert_eq!(mapping["mcp"]["command"], expected_bin);
    assert_eq!(mapping["estate_bin"], expected_bin);
    assert_eq!(mapping["mcp"]["tool"], "complete");
    assert_eq!(mapping["mcp"]["complete_timeout_secs"], 120);
    assert_eq!(
        mapping["mcp"]["complete_timeout_env"],
        "CELL_MCP_COMPLETE_TIMEOUT_SECS"
    );
    assert_eq!(mapping["install"], "INSTALL.md");
    assert_eq!(mapping["runner_docs"], "RUNNER.md");
    assert_eq!(mapping["session_docs"], "SESSION.md");
    assert_eq!(mapping["packages"][0]["mcp_tool"], "complete");
    assert_eq!(mapping["packages"][0]["skill_stub"], false);
    assert_eq!(mapping["packages"][1]["mcp_tool"], "complete");
    assert_eq!(mapping["packages"][1]["skill_stub"], false);
    assert!(
        mapping["mcp"]["invokes"]
            .as_str()
            .unwrap()
            .contains("estate complete"),
        "{mapping}"
    );
    assert!(
        mapping["estate_path"]
            .as_str()
            .unwrap()
            .ends_with("agent-pack-handoff.yaml"),
        "{mapping}"
    );
    assert_eq!(mapping["routines"][0]["id"], "standing-classify");
    assert_eq!(mapping["routines"][0]["cron_note"], "0 * * * *");
    assert_eq!(mapping["routines"][0]["live_trigger"], false);
    assert_eq!(mapping["routines"][1]["id"], "standing-once");
    assert_eq!(mapping["routines"][1]["cron_note"], "0 * * * *");
    assert_eq!(mapping["routines"][1]["live_trigger"], false);

    let readme = read(&out.join("README.md"));
    assert!(readme.contains("# research-crew pack plugin"), "{readme}");
    assert!(!readme.contains("plugin stub"), "{readme}");
    assert!(
        readme.contains("not live Cursor / Grok Bot sync"),
        "{readme}"
    );
    assert!(readme.contains("estate pack mcp-serve"), "{readme}");
    assert!(
        readme.contains("wired member MCP tool `complete`"),
        "{readme}"
    );
    assert!(readme.contains("They are not stubs."), "{readme}");
    assert!(readme.contains("wired_mcp: true"), "{readme}");
    assert!(readme.contains("live_sync: false"), "{readme}");
    assert!(readme.contains("CELL_ESTATE_PATH"), "{readme}");
    assert!(
        readme.contains("CELL_MCP_COMPLETE_TIMEOUT_SECS"),
        "{readme}"
    );
    assert!(readme.contains("INSTALL.md"), "{readme}");
    assert!(readme.contains("RUNNER.md"), "{readme}");
    assert!(readme.contains("SESSION.md"), "{readme}");
    assert!(readme.contains("plugin-prove"), "{readme}");
    assert!(readme.contains("plugin-install-local"), "{readme}");
    assert!(readme.contains(".cursor/plugins/local"), "{readme}");
    assert!(readme.contains(".estate-pack-install.json"), "{readme}");
    assert!(readme.contains("runner_docs: yes"), "{readme}");
    assert!(readme.contains("session_docs: yes"), "{readme}");
    assert!(readme.contains("absolute estate binary"), "{readme}");
    assert!(readme.contains(&expected_bin), "{readme}");
    assert!(!readme.contains("must be on PATH"), "{readme}");
    assert!(readme.contains("refuse:pack-orchestrator"), "{readme}");
    assert!(!readme.contains("command: true"), "{readme}");
    assert!(readme.contains("standing-classify"), "{readme}");
    assert!(readme.contains("standing-once"), "{readme}");
    assert!(readme.contains("0 * * * *"), "{readme}");
    assert!(!readme.contains("live PASS"), "{readme}");

    let install = read(&out.join("INSTALL.md"));
    assert!(install.contains("# INSTALL"), "{install}");
    assert!(install.contains("Plugin folder"), "{install}");
    assert!(install.contains("mcp.json"), "{install}");
    assert!(install.contains("complete"), "{install}");
    assert!(install.contains("horizon"), "{install}");
    assert!(install.contains("decision receipt"), "{install}");
    assert!(
        install.contains("cell-one.decision-receipt.v0"),
        "{install}"
    );
    assert!(install.contains("research"), "{install}");
    assert!(install.contains("refuse:pack-orchestrator"), "{install}");
    assert!(install.contains("READY_FOR_LIVE_TEST: no"), "{install}");
    assert!(install.contains("live_sync: false"), "{install}");
    assert!(install.contains("Not a live PASS"), "{install}");
    assert!(!install.contains("READY_FOR_LIVE_TEST: yes"), "{install}");
    assert!(!install.contains("plugin stub"), "{install}");
    assert!(!install.contains("pack plugin stub"), "{install}");
    assert!(!install.contains("XAI_API_KEY"), "{install}");
    assert!(install.contains("RUNNER.md"), "{install}");
    assert!(install.contains("SESSION.md"), "{install}");
    assert!(install.contains("runner_docs: yes"), "{install}");
    assert!(install.contains("session_docs: yes"), "{install}");
    assert!(install.contains("plugin-install-local"), "{install}");
    assert!(install.contains(".cursor/plugins/local"), "{install}");
    assert!(install.contains(".estate-pack-install.json"), "{install}");
    assert!(install.contains("refuse:plugin-install-symlink"), "{install}");
    assert!(
        install.contains("loaded: skipped:loader-unavailable"),
        "{install}"
    );
    assert!(
        install.contains("does not claim Cursor Customize loaded the plugin"),
        "{install}"
    );
    assert!(install.contains("CLI smoke (first-class)"), "{install}");
    assert!(install.contains("estate complete --mock"), "{install}");
    assert!(
        install.contains("Cursor MCP loader hang is out of scope"),
        "{install}"
    );
    assert!(install.contains("cohesion-prove"), "{install}");
    assert!(install.contains("outcome: allow"), "{install}");
    assert!(readme.contains("CLI smoke (first-class)"), "{readme}");
    assert!(readme.contains("estate complete --mock"), "{readme}");
    assert!(
        readme.contains("Cursor MCP loader hang is out of scope"),
        "{readme}"
    );

    let runner = read(&out.join("RUNNER.md"));
    assert!(runner.contains("# RUNNER"), "{runner}");
    assert!(
        runner.contains("estate routine runner start"),
        "{runner}"
    );
    assert!(
        runner.contains("estate routine runner stop"),
        "{runner}"
    );
    assert!(
        runner.contains("estate routine runner status"),
        "{runner}"
    );
    assert!(
        runner.contains("estate routine runner restart"),
        "{runner}"
    );
    assert!(
        runner.contains("{state-dir}/routine-runner/"),
        "{runner}"
    );
    assert!(
        runner.contains("refuse:runner-already-running"),
        "{runner}"
    );
    assert!(
        runner.contains("refuse:runner-routine-unknown"),
        "{runner}"
    );
    assert!(
        runner.contains("refuse:runner-routine-disabled"),
        "{runner}"
    );
    assert!(
        runner.contains("refuse:runner-routine-invalid"),
        "{runner}"
    );
    assert!(runner.contains("estate routine runner-prove"), "{runner}");
    assert!(runner.contains("runner-prove --dual"), "{runner}");
    assert!(runner.contains("session_id"), "{runner}");
    assert!(runner.contains("package"), "{runner}");
    assert!(runner.contains("chain"), "{runner}");
    assert!(runner.contains("ticks"), "{runner}");
    assert!(runner.contains("READY_FOR_LIVE_TEST: no"), "{runner}");
    assert!(runner.contains("live_sync: false"), "{runner}");
    assert!(!runner.contains("READY_FOR_LIVE_TEST: yes"), "{runner}");
    assert!(runner.contains("Not a live PASS"), "{runner}");
    assert!(runner.contains("standing-classify"), "{runner}");
    assert!(runner.contains("standing-once"), "{runner}");

    let session = read(&out.join("SESSION.md"));
    assert!(session.contains("# SESSION"), "{session}");
    assert!(
        session.contains("estate pack session create"),
        "{session}"
    );
    assert!(session.contains("estate pack session show"), "{session}");
    assert!(session.contains("estate pack session end"), "{session}");
    assert!(session.contains("context=none"), "{session}");
    assert!(session.contains("context=applied"), "{session}");
    assert!(session.contains("session_id"), "{session}");
    assert!(session.contains("routine-state.json"), "{session}");
    assert!(session.contains("ag_news"), "{session}");
    assert!(session.contains("rust_idiom"), "{session}");
    assert!(session.contains("frontier_http"), "{session}");
    assert!(session.contains("refuse:session-ended"), "{session}");
    assert!(session.contains("refuse:session-expired"), "{session}");
    assert!(session.contains("refuse:session-bound"), "{session}");
    assert!(session.contains("READY_FOR_LIVE_TEST: no"), "{session}");
    assert!(session.contains("live_sync: false"), "{session}");
    assert!(!session.contains("READY_FOR_LIVE_TEST: yes"), "{session}");
    assert!(session.contains("Not a live PASS"), "{session}");

    assert_locked_cksum();
}

#[test]
fn export_plugin_refuses_unknown_pack_and_file_out() {
    assert_locked_cksum();
    let dir = scratch("refuse");
    let estate = fixture().display().to_string();
    let out = dir.join("plugin");

    let (ok, stdout, stderr) = run(&[
        "pack",
        "export-plugin",
        "--id",
        "missing",
        "--estate",
        &estate,
        "--out",
        &out.display().to_string(),
    ]);
    assert!(!ok, "{stdout}");
    assert!(stderr.contains("refuse:unknown-pack"), "{stderr}");
    assert!(!out.exists(), "no write on unknown pack");

    let locked = repo_root().join("examples/estate.yaml");
    let (ok, _stdout, stderr) = run(&[
        "pack",
        "export-plugin",
        "--id",
        "research-crew",
        "--estate",
        &locked.display().to_string(),
        "--out",
        &out.display().to_string(),
    ]);
    assert!(!ok);
    assert!(stderr.contains("refuse:unknown-pack"), "{stderr}");
    assert!(!out.exists());

    let file_out = dir.join("not-a-dir");
    std::fs::write(&file_out, "nope").unwrap();
    let (ok, _stdout, stderr) = run(&[
        "pack",
        "export-plugin",
        "--id",
        "research-crew",
        "--estate",
        &estate,
        "--out",
        &file_out.display().to_string(),
    ]);
    assert!(!ok);
    assert!(stderr.contains("refuse:export-plugin-out"), "{stderr}");
    assert_eq!(std::fs::read_to_string(&file_out).unwrap(), "nope");
    assert_locked_cksum();
}

#[test]
fn export_plugin_refuses_uncanonical_or_relative_estate_path() {
    assert_locked_cksum();
    let dir = scratch("nopath");
    let missing = dir.join("missing-estate.yaml");
    let out = dir.join("plugin");

    let (ok, stdout, stderr) = run(&[
        "pack",
        "export-plugin",
        "--id",
        "research-crew",
        "--estate",
        &missing.display().to_string(),
        "--out",
        &out.display().to_string(),
    ]);
    assert!(!ok, "{stdout}");
    assert!(stderr.contains("refuse:export-estate-path"), "{stderr}");
    assert!(
        !out.exists(),
        "no write when estate path cannot canonicalize"
    );

    let (ok, stdout, stderr) = run(&[
        "pack",
        "export-plugin",
        "--id",
        "research-crew",
        "--estate",
        "no-such-cell-export-estate.yaml",
        "--out",
        &out.display().to_string(),
    ]);
    assert!(!ok, "{stdout}");
    assert!(stderr.contains("refuse:export-estate-path"), "{stderr}");
    assert!(stderr.contains("cannot canonicalize"), "{stderr}");
    assert!(!out.exists());
    assert_locked_cksum();
}
