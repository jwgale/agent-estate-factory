//! Agent packs + orchestrator handoff on `estate complete --pack`.
//!
//! Membership scopes handoff. Free mixed select without `--pack` stays out.
//! Does not invent a live PASS. Locked examples/estate.yaml stays untouched.

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
    let dir = std::env::temp_dir().join(format!("cell-pack-handoff-{name}-{nanos}"));
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

fn journal(state: &Path) -> PathBuf {
    state.join("decisions").join("receipts.jsonl")
}

fn load_receipts(state: &Path) -> Vec<serde_json::Value> {
    let text = std::fs::read_to_string(journal(state)).unwrap();
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).expect(line))
        .collect()
}

#[test]
fn pack_list_and_show_read_estate_packs() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let (ok, stdout, stderr) = run(&["pack", "list", "--estate", &estate_s]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("research-crew"), "{stdout}");
    assert!(stdout.contains("orchestrator=horizon"), "{stdout}");
    assert!(stdout.contains("members=[horizon,research]"), "{stdout}");

    let (ok, stdout, stderr) = run(&[
        "pack",
        "show",
        "--id",
        "research-crew",
        "--estate",
        &estate_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("pack research-crew"), "{stdout}");
    assert!(stdout.contains("orchestrator: horizon"), "{stdout}");
    assert!(stdout.contains("- research"), "{stdout}");

    let (ok, _stdout, stderr) = run(&[
        "pack",
        "show",
        "--id",
        "missing",
        "--estate",
        &estate_s,
    ]);
    assert!(!ok, "{stderr}");
    assert!(stderr.contains("refuse:unknown-pack"), "{stderr}");
    assert_locked_cksum();
}

#[test]
fn complete_pack_hands_off_to_member_and_journals_pack_fields() {
    assert_locked_cksum();
    let dir = scratch("handoff");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "complete",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--agent",
        "horizon",
        "--pack",
        "research-crew",
        "--prompt",
        "ping",
        "--mock",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("decision receipt:"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stderr.contains("live PASS"), "{stderr}");

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row["schema"], "cell-one.decision-receipt.v0");
    assert_eq!(row["surface"], "complete");
    assert_eq!(row["agent"], "research");
    assert_eq!(row["pack_id"], "research-crew");
    assert_eq!(row["handoff_from"], "horizon");
    assert_eq!(row["handoff_to"], "research");
    assert_eq!(row["capability"], "ag_news");
    assert_eq!(row["result"], "ag_news");
    assert_eq!(row["outcome"], "allow");

    let (ok, report, stderr) = run(&["decisions", "report", "--state-dir", &state_s]);
    assert!(ok, "{stderr}");
    assert!(report.contains("pack=research-crew"), "{report}");
    assert!(report.contains("handoff=horizon->research"), "{report}");

    let (ok, filtered, stderr) = run(&[
        "decisions",
        "report",
        "--state-dir",
        &state_s,
        "--pack",
        "research-crew",
    ]);
    assert!(ok, "{stderr}");
    assert!(filtered.contains("decision receipts: 1"), "{filtered}");

    let (ok, empty, stderr) = run(&[
        "decisions",
        "report",
        "--state-dir",
        &state_s,
        "--pack",
        "other",
    ]);
    assert!(ok, "{stderr}");
    assert!(empty.contains("decision receipts: 0"), "{empty}");
    assert_locked_cksum();
}

#[test]
fn complete_pack_refuses_non_orchestrator() {
    assert_locked_cksum();
    let dir = scratch("bad-orch");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "complete",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--agent",
        "research",
        "--pack",
        "research-crew",
        "--prompt",
        "ping",
        "--mock",
    ]);
    assert!(!ok, "stdout={stdout}");
    assert!(
        stderr.contains("refuse:pack-orchestrator"),
        "stderr={stderr}\nstdout={stdout}"
    );
    assert!(!journal(&state).is_file(), "no receipt on orchestrator refuse");
    assert_locked_cksum();
}

#[test]
fn locked_example_has_no_packs_and_hash_holds() {
    assert_locked_cksum();
    let estate = estate_schema::load_estate(&repo_root().join("examples/estate.yaml")).unwrap();
    assert!(estate.packs.is_empty());
    assert_eq!(
        estate_schema::estate_hash(&estate),
        "sha256:dcd7164f04c83f514185e77d2d4f6c23cae6dbb27a9b5da96a28ba1f3c724930"
    );
    let (ok, stdout, stderr) = run(&[
        "pack",
        "list",
        "--estate",
        &repo_root().join("examples/estate.yaml").display().to_string(),
    ]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("no agent packs"), "{stdout}");
}
