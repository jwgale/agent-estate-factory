//! `estate pack plugin-prove` — human gate before Cursor install.
//!
//! Fixture: examples/fixtures/agent-pack-handoff.yaml.
//! Happy path: export + baked absolute estate bin + mock complete as
//! horizon (receipt) + as research (refuse:pack-orchestrator).
//! Fail when mcp.json command is bare `estate`. Stub-label absence
//! when wired. live_sync stays false. Does not invent a live PASS.
//! Locked examples/estate.yaml stays untouched.

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("cell-pack-plugin-prove-{name}-{nanos}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args(args)
        .current_dir(repo_root())
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

fn last_json_object(stdout: &str) -> Value {
    let line = stdout
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .unwrap_or_else(|| panic!("no JSON report in:\n{stdout}"));
    serde_json::from_str(line).unwrap_or_else(|e| panic!("parse {line}: {e}"))
}

#[test]
fn plugin_prove_happy_path_fixture_research_crew() {
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
        "plugin-prove",
        "--id",
        "research-crew",
        "--estate",
        &estate,
        "--out",
        &out_s,
        "--prompt",
        "ping",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("pack plugin: research-crew"), "{stdout}");
    assert!(!stdout.contains("pack plugin stub"), "{stdout}");
    assert!(
        stdout.contains("pack plugin-prove: research-crew"),
        "{stdout}"
    );
    assert!(stdout.contains("ok: yes"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert!(stdout.contains("live_sync: no"), "{stdout}");
    assert!(stdout.contains("absolute_estate_bin: ok"), "{stdout}");
    assert!(stdout.contains("horizon_complete: ok"), "{stdout}");
    assert!(stdout.contains("research_complete: ok"), "{stdout}");
    assert!(stdout.contains("honest_labels: ok"), "{stdout}");
    assert!(stdout.contains("install_md: ok"), "{stdout}");
    assert!(stdout.contains("timeout_env: ok"), "{stdout}");
    assert!(stdout.contains(&expected_bin), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!stderr.contains("live PASS"), "{stderr}");

    let report = last_json_object(&stdout);
    assert_eq!(report["schema"], "cell-one.pack-plugin-prove.v0");
    assert_eq!(report["ok"], true);
    assert_eq!(report["pack_id"], "research-crew");
    assert_eq!(report["ready_for_live_test"], false);
    assert_eq!(report["live_sync"], false);
    assert_eq!(report["wired_mcp"], true);
    assert_eq!(report["estate_bin"], expected_bin);
    assert_eq!(report["checks"]["absolute_estate_bin"]["ok"], true);
    assert_eq!(report["checks"]["horizon_complete"]["ok"], true);
    assert_eq!(report["checks"]["research_complete"]["ok"], true);
    assert!(
        report["checks"]["horizon_complete"]["detail"]
            .as_str()
            .unwrap()
            .contains("decision receipt:"),
        "{report}"
    );
    assert!(
        report["checks"]["research_complete"]["detail"]
            .as_str()
            .unwrap()
            .contains("refuse:pack-orchestrator"),
        "{report}"
    );

    let mcp: Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("mcp.json")).unwrap()).unwrap();
    assert_ne!(mcp["mcpServers"]["horizon"]["command"], "estate");
    assert_eq!(mcp["mcpServers"]["horizon"]["command"], expected_bin);
    assert_eq!(
        mcp["mcpServers"]["horizon"]["env"]["CELL_MCP_COMPLETE_TIMEOUT_SECS"],
        "120"
    );
    assert!(out.join("INSTALL.md").is_file());
    let install = std::fs::read_to_string(out.join("INSTALL.md")).unwrap();
    assert!(install.contains("READY_FOR_LIVE_TEST: no"), "{install}");
    assert!(!install.contains("plugin stub"), "{install}");
    assert_locked_cksum();
}

#[test]
fn plugin_prove_fails_when_mcp_command_is_bare_estate() {
    assert_locked_cksum();
    let dir = scratch("bare");
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
    assert!(ok, "export failed: {stderr}\n{stdout}");

    let mcp_path = out.join("mcp.json");
    let mut mcp: Value =
        serde_json::from_str(&std::fs::read_to_string(&mcp_path).unwrap()).unwrap();
    mcp["mcpServers"]["horizon"]["command"] = serde_json::json!("estate");
    mcp["mcpServers"]["research"]["command"] = serde_json::json!("estate");
    std::fs::write(&mcp_path, serde_json::to_string_pretty(&mcp).unwrap()).unwrap();

    let (ok, stdout, stderr) = run(&[
        "pack",
        "plugin-prove",
        "--id",
        "research-crew",
        "--estate",
        &estate,
        "--out",
        &out_s,
        "--check-only",
    ]);
    assert!(!ok, "bare estate must fail prove\n{stdout}\n{stderr}");
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        combined.contains("refuse:plugin-prove-estate-bin")
            || combined.contains("bare 'estate'")
            || combined.contains("absolute_estate_bin: FAIL"),
        "{combined}"
    );
    assert!(
        combined.contains("refuse:plugin-prove") || stdout.contains("ok: no"),
        "{combined}"
    );
    assert!(!combined.contains("READY_FOR_LIVE_TEST: yes"), "{combined}");
    assert_locked_cksum();
}

#[test]
fn plugin_prove_fails_when_wired_export_uses_stub_label() {
    assert_locked_cksum();
    let dir = scratch("stub");
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
    assert!(ok, "export failed: {stderr}\n{stdout}");
    assert!(!stdout.contains("pack plugin stub"), "{stdout}");

    let readme = out.join("README.md");
    let mut text = std::fs::read_to_string(&readme).unwrap();
    text = text.replacen(
        "# research-crew pack plugin",
        "# research-crew plugin stub",
        1,
    );
    std::fs::write(&readme, text).unwrap();

    let (ok, stdout, stderr) = run(&["pack", "plugin-prove", "--check-only", "--out", &out_s]);
    assert!(!ok, "stub label must fail prove\n{stdout}\n{stderr}");
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        combined.contains("honest_labels: FAIL") || combined.contains("refuse:plugin-prove-label"),
        "{combined}"
    );
    assert_locked_cksum();
}

#[test]
fn plugin_prove_defaults_to_fixture_research_crew() {
    assert_locked_cksum();
    let dir = scratch("default");
    let out = dir.join("plugin");
    let (ok, stdout, stderr) = run(&["pack", "plugin-prove", "--out", &out.display().to_string()]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.contains("pack plugin-prove: research-crew"),
        "{stdout}"
    );
    assert!(stdout.contains("ok: yes"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert_locked_cksum();
}
