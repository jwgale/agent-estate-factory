//! `estate pack crew-session-prove` — throwaway install then multi-hop
//! CLI crew session. Locked `examples/estate.yaml` stays put.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let mut token = std::process::id().to_string();
    for needle in ["5090", "4090", "4080", "3090"] {
        token = token.replace(needle, "0000");
    }
    let path = std::env::temp_dir().join(format!("cell-one-crew-session-{name}-{token}"));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn cksum(path: &Path) -> String {
    let out = Command::new("cksum").arg(path).output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn cksum_locked() -> String {
    cksum(&repo_root().join("examples/estate.yaml"))
}

#[test]
fn crew_session_prove_multi_hop_complete_on_one_session() {
    let before = cksum_locked();
    assert!(
        before.starts_with("43770130 3391"),
        "examples/estate.yaml cksum drifted: {before}"
    );
    let locked_bytes = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
    let fixture = repo_root().join("examples/fixtures/agent-pack-handoff.yaml");
    let fixture_bytes = fs::read(&fixture).unwrap();
    let out = scratch("prove");
    let sentinel = scratch("sentinel-home");
    let run = bin()
        .args([
            "pack",
            "crew-session-prove",
            "--root",
            repo_root().to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--id",
            "research-crew",
            "--estate",
            "examples/fixtures/agent-pack-handoff.yaml",
        ])
        .env("HOME", &sentinel)
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_LOCAL_LIVE")
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_CURSOR_PLUGINS_MODULE")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run.stderr).to_string();
    let text = format!("{stdout}{stderr}");
    assert!(run.status.success(), "{text}");
    assert!(stdout.contains("crew-session-prove: pack"), "{stdout}");
    assert!(stdout.contains("crew-session-prove: cli-smoke"), "{stdout}");
    assert!(
        stdout.contains("cli-smoke horizon hop 1: decision receipt outcome=allow surface=complete pack=research-crew capability=ag_news result=ag_news session=sess-cohesion01 turns=1 context=none"),
        "{stdout}"
    );
    assert!(
        stdout.contains("cli-smoke horizon hop 2: decision receipt outcome=allow surface=complete pack=research-crew capability=ag_news result=ag_news session=sess-cohesion01 turns=2 context=applied saw_prior=yes"),
        "{stdout}"
    );
    assert!(
        stdout.contains("cli-smoke research: refuse:pack-orchestrator session_intact=yes"),
        "{stdout}"
    );
    assert!(
        stdout.contains("cli-smoke horizon isolation: session=sess-cohesioniso turns=1 context=none leaked=no"),
        "{stdout}"
    );
    let tail = stdout.trim_end();
    assert!(
        tail.ends_with("crew-session-prove: ok\nREADY_FOR_LIVE_TEST: no"),
        "{stdout}"
    );
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!text.contains("live PASS recorded"), "{text}");
    assert!(!sentinel.join(".cursor").exists(), "install wrote the sentinel HOME");

    let report: Value =
        serde_json::from_str(&fs::read_to_string(out.join("crew-session-prove.json")).unwrap())
            .unwrap();
    assert_eq!(report["schema"], "cell-one.crew-session-prove.v0");
    assert_eq!(report["ok"], true);
    assert_eq!(report["ready_for_live_test"], false);
    assert_eq!(report["live_pass_recorded"], false);
    assert_eq!(report["live_sync"], false);
    assert_eq!(report["pack"]["cli_smoke"]["session_id"], "sess-cohesion01");
    assert_eq!(report["pack"]["cli_smoke"]["hops"][0]["context"], "none");
    assert_eq!(report["pack"]["cli_smoke"]["hops"][1]["context"], "applied");
    assert_eq!(report["pack"]["cli_smoke"]["hops"][1]["saw_prior"], true);
    assert_eq!(
        report["pack"]["cli_smoke"]["member"]["refuse"],
        "refuse:pack-orchestrator"
    );
    assert_eq!(report["pack"]["cli_smoke"]["member"]["receipt_written"], false);
    assert_eq!(report["pack"]["cli_smoke"]["isolation"]["leaked"], false);
    assert_eq!(report["pack"]["cursor_loader"], "out-of-scope");
    assert_eq!(report["pack"]["loader_is_live_pass"], false);

    let journal = fs::read_to_string(out.join("cli-smoke/crew/decisions/receipts.jsonl")).unwrap();
    let rows: Vec<Value> = journal
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 3, "{journal}");
    assert_eq!(rows[0]["session_id"], "sess-cohesion01");
    assert_eq!(rows[1]["session_id"], "sess-cohesion01");
    assert_eq!(rows[1]["session_context"], true);
    assert_eq!(rows[2]["session_id"], "sess-cohesioniso");
    let session = fs::read_to_string(
        out.join("cli-smoke/crew/pack-sessions/research-crew/sess-cohesion01.json"),
    )
    .unwrap();
    assert!(session.contains("unique-hop-alpha-token"), "{session}");
    assert_eq!(fs::read(repo_root().join("examples/estate.yaml")).unwrap(), locked_bytes);
    assert_eq!(fs::read(&fixture).unwrap(), fixture_bytes);
    assert_eq!(cksum_locked(), before);

    let art = PathBuf::from("/opt/cursor/artifacts");
    if fs::create_dir_all(&art).is_ok() {
        let _ = fs::write(
            art.join("crew-session-prove.json"),
            serde_json::to_string_pretty(&report).unwrap() + "\n",
        );
        let _ = fs::write(art.join("crew-session-prove.log"), &stdout);
    }
    let _ = fs::remove_dir_all(&out);
    let _ = fs::remove_dir_all(&sentinel);
}

#[test]
fn crew_session_prove_refuses_the_locked_estate() {
    let before = cksum_locked();
    assert!(before.starts_with("43770130 3391"), "{before}");
    let locked = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&locked).unwrap();
    let run = bin()
        .args([
            "pack",
            "crew-session-prove",
            "--root",
            repo_root().to_str().unwrap(),
            "--out",
            locked.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run.stderr).to_string();
    assert!(!run.status.success(), "{stdout}\n{stderr}");
    assert!(
        stderr.contains("refuse:out: crew-session-prove does not write examples/estate.yaml"),
        "{stderr}"
    );
    assert!(!stdout.contains("crew-session-prove: ok"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert_eq!(fs::read(&locked).unwrap(), bytes);
    assert_eq!(cksum_locked(), before);
}

#[test]
fn crew_session_prove_refuses_the_locked_estate_as_estate_flag() {
    let before = cksum_locked();
    assert!(before.starts_with("43770130 3391"), "{before}");
    let locked = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&locked).unwrap();
    let out = scratch("estate-flag");
    let run = bin()
        .args([
            "pack",
            "crew-session-prove",
            "--root",
            repo_root().to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--estate",
            "examples/estate.yaml",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run.stderr).to_string();
    assert!(!run.status.success(), "{stdout}\n{stderr}");
    assert!(
        stderr.contains("refuse:estate: crew-session-prove does not use examples/estate.yaml"),
        "{stderr}"
    );
    assert!(!stdout.contains("crew-session-prove: ok"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert_eq!(fs::read(&locked).unwrap(), bytes);
    assert_eq!(cksum_locked(), before);
    let _ = fs::remove_dir_all(&out);
}
