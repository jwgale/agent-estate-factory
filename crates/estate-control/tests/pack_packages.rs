//! Pack packages (skills) + standing routines (automations).
//!
//! `estate package run` → complete --pack with package_id on the receipt.
//! `estate routine run` declares + runs a package and stamps routine_id.
//! Security stays existing intentions. Not a Grok Bot runtime clone.
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
    let dir = std::env::temp_dir().join(format!("cell-pack-packages-{name}-{nanos}"));
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
fn package_list_show_and_run_stamp_package_id() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();

    let (ok, stdout, stderr) = run(&["package", "list", "--estate", &estate_s]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("classify-ping"), "{stdout}");
    assert!(stdout.contains("pack=research-crew"), "{stdout}");
    assert!(stdout.contains("binding=ag_news"), "{stdout}");

    let (ok, stdout, stderr) = run(&[
        "package",
        "show",
        "--id",
        "classify-ping",
        "--estate",
        &estate_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("package classify-ping"), "{stdout}");
    assert!(stdout.contains("pack: research-crew"), "{stdout}");

    let dir = scratch("pkg-run");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "package",
        "run",
        "--id",
        "classify-ping",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("decision receipt:"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row["schema"], "cell-one.decision-receipt.v0");
    assert_eq!(row["surface"], "complete");
    assert_eq!(row["agent"], "research");
    assert_eq!(row["pack_id"], "research-crew");
    assert_eq!(row["handoff_from"], "horizon");
    assert_eq!(row["handoff_to"], "research");
    assert_eq!(row["package_id"], "classify-ping");
    assert!(row.get("routine_id").map(|v| v.is_null()).unwrap_or(true));
    assert_eq!(row["capability"], "ag_news");
    assert_eq!(row["outcome"], "allow");

    let (ok, report, stderr) = run(&["decisions", "report", "--state-dir", &state_s]);
    assert!(ok, "{stderr}");
    assert!(report.contains("package=classify-ping"), "{report}");
    assert_locked_cksum();
}

#[test]
fn routine_list_show_and_run_stamp_routine_and_package() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();

    let (ok, stdout, stderr) = run(&["routine", "list", "--estate", &estate_s]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("standing-classify"), "{stdout}");
    assert!(stdout.contains("package=classify-ping"), "{stdout}");

    let (ok, stdout, stderr) = run(&[
        "routine",
        "show",
        "--id",
        "standing-classify",
        "--estate",
        &estate_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("routine standing-classify"), "{stdout}");

    let dir = scratch("routine-run");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "routine",
        "run",
        "--id",
        "standing-classify",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("decision receipt:"), "{stdout}");

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row["package_id"], "classify-ping");
    assert_eq!(row["routine_id"], "standing-classify");
    assert_eq!(row["pack_id"], "research-crew");
    assert_eq!(row["handoff_from"], "horizon");
    assert_eq!(row["handoff_to"], "research");

    let (ok, report, stderr) = run(&["decisions", "report", "--state-dir", &state_s]);
    assert!(ok, "{stderr}");
    assert!(report.contains("package=classify-ping"), "{report}");
    assert!(report.contains("routine=standing-classify"), "{report}");
    assert_locked_cksum();
}

#[test]
fn package_run_refuses_unknown_and_locked_example_stays_empty() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("pkg-miss");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "package",
        "run",
        "--id",
        "missing",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
    ]);
    assert!(!ok, "stdout={stdout}");
    assert!(stderr.contains("refuse:unknown-package"), "{stderr}");

    let (ok, stdout, stderr) = run(&[
        "routine",
        "run",
        "--id",
        "missing",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
    ]);
    assert!(!ok, "stdout={stdout}");
    assert!(stderr.contains("refuse:unknown-routine"), "{stderr}");

    let estate = estate_schema::load_estate(&repo_root().join("examples/estate.yaml")).unwrap();
    assert!(estate.packs.is_empty());
    assert!(estate.pack_packages.is_empty());
    assert!(estate.routines.is_empty());
    assert_eq!(
        estate_schema::estate_hash(&estate),
        "sha256:dcd7164f04c83f514185e77d2d4f6c23cae6dbb27a9b5da96a28ba1f3c724930"
    );

    let locked = repo_root().join("examples/estate.yaml").display().to_string();
    let (ok, stdout, stderr) = run(&["package", "list", "--estate", &locked]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("no pack packages"), "{stdout}");
    let (ok, stdout, stderr) = run(&["routine", "list", "--estate", &locked]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("no standing routines"), "{stdout}");
    assert_locked_cksum();
}
