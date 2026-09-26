//! `estate pack export-plugin` — Agent Plugin stub from a pack.
//!
//! Fixture: examples/fixtures/agent-pack-handoff.yaml.
//! MCP is wired to `estate pack mcp-serve` → `estate complete`.
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
    assert!(
        stdout.contains("pack plugin stub: research-crew"),
        "{stdout}"
    );
    assert!(stdout.contains("orchestrator: horizon"), "{stdout}");
    assert!(stdout.contains("members: horizon, research"), "{stdout}");
    assert!(
        stdout.contains("skills: classify-once, classify-ping"),
        "{stdout}"
    );
    assert!(stdout.contains("standing-classify @hourly"), "{stdout}");
    assert!(stdout.contains("wired_mcp: yes"), "{stdout}");
    assert!(stdout.contains("live_sync: no"), "{stdout}");
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

    let mcp: serde_json::Value = serde_json::from_str(&read(&out.join("mcp.json"))).unwrap();
    let servers = mcp["mcpServers"].as_object().expect("mcpServers");
    assert!(servers.contains_key("horizon"), "{servers:?}");
    assert!(servers.contains_key("research"), "{servers:?}");
    assert_eq!(servers["horizon"]["command"], "estate");
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
        ping.contains("estate package run --id classify-ping"),
        "{ping}"
    );
    assert!(ping.contains("estate pack mcp-serve"), "{ping}");
    assert!(ping.contains("estate complete"), "{ping}");
    assert!(ping.contains("standing-classify"), "{ping}");
    assert!(ping.contains("schedule=@hourly"), "{ping}");
    assert!(ping.contains("0 * * * *"), "{ping}");

    let once = read(&out.join("skills/classify-once/SKILL.md"));
    assert!(once.contains("name: classify-once"), "{once}");
    assert!(once.contains("no standing routines"), "{once}");

    let mapping: serde_json::Value =
        serde_json::from_str(&read(&out.join("estate-pack.json"))).unwrap();
    assert_eq!(mapping["schema"], "cell-one.pack-plugin-export.v0");
    assert_eq!(mapping["pack_id"], "research-crew");
    assert_eq!(mapping["group"]["orchestrator"], "horizon");
    assert_eq!(mapping["live_sync"], false);
    assert_eq!(mapping["wired_mcp"], true);
    assert_eq!(mapping["mcp"]["command"], "estate");
    assert_eq!(mapping["mcp"]["tool"], "complete");
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

    let readme = read(&out.join("README.md"));
    assert!(
        readme.contains("not live Cursor / Grok Bot sync"),
        "{readme}"
    );
    assert!(readme.contains("estate pack mcp-serve"), "{readme}");
    assert!(readme.contains("wired_mcp: true"), "{readme}");
    assert!(readme.contains("live_sync: false"), "{readme}");
    assert!(readme.contains("CELL_ESTATE_PATH"), "{readme}");
    assert!(readme.contains("refuse:pack-orchestrator"), "{readme}");
    assert!(!readme.contains("command: true"), "{readme}");
    assert!(readme.contains("standing-classify"), "{readme}");
    assert!(readme.contains("0 * * * *"), "{readme}");
    assert!(!readme.contains("live PASS"), "{readme}");

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
