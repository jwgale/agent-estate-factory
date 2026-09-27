//! Routine schedule tick + watch + multi-hop package chain.
//!
//! Tick is idempotent (due / not-due). Watch loops that tick with a
//! max-cycles / env interval test hook. Chain receipts share chain_id and
//! ordered handoffs. Locked examples/estate.yaml stays untouched.

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
    let dir = std::env::temp_dir().join(format!("cell-routine-schedule-{name}-{nanos}"));
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
        .env_remove("CELL_ROUTINE_WATCH_INTERVAL_SECS")
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
    let path = journal(state);
    if !path.is_file() {
        return Vec::new();
    }
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).expect(line))
        .collect()
}

fn load_routine_state(state: &Path) -> serde_json::Value {
    let text = std::fs::read_to_string(state.join("routine-state.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn routine_status_and_tick_due_then_not_due() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("tick");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "routine",
        "status",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("standing-classify"), "{stdout}");
    assert!(stdout.contains("schedule=@hourly"), "{stdout}");
    assert!(stdout.contains("enabled=true"), "{stdout}");
    assert!(stdout.contains("last_run=-"), "{stdout}");
    assert!(stdout.contains("next_due=-"), "{stdout}");

    let (ok, stdout, stderr) = run(&[
        "routine",
        "tick",
        "--id",
        "standing-classify",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("ticked standing-classify"), "{stdout}");
    assert!(stdout.contains("ran=1"), "{stdout}");
    assert!(stdout.contains("decision receipt:"), "{stdout}");

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0]["routine_id"], "standing-classify");
    assert_eq!(rows[0]["package_id"], "classify-ping");
    assert_eq!(rows[0]["pack_id"], "research-crew");
    assert_eq!(rows[0]["handoff_from"], "horizon");
    assert_eq!(rows[0]["handoff_to"], "research");
    assert_eq!(rows[0]["session_context"], false);
    assert_eq!(rows[1]["handoff_from"], "research");
    assert_eq!(rows[1]["handoff_to"], "horizon");
    assert_eq!(rows[1]["session_context"], true);
    assert_eq!(rows[0]["session_id"], rows[1]["session_id"]);
    assert!(rows[0]["session_id"].as_str().unwrap().starts_with("sess-"));
    let saved = load_routine_state(&state);
    assert_eq!(
        saved["routines"]["standing-classify"]["session_id"],
        rows[0]["session_id"]
    );
    assert_eq!(saved["schema"], "cell-one.routine-state.v0");
    assert!(saved["routines"]["standing-classify"]["last_run"].is_number());
    assert!(saved["routines"]["standing-classify"]["next_due"].is_number());

    let (ok, stdout, stderr) = run(&[
        "routine",
        "tick",
        "--id",
        "standing-classify",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("skip standing-classify"), "{stdout}");
    assert!(stdout.contains("ran=0"), "{stdout}");
    assert!(stdout.contains("skipped=1"), "{stdout}");
    assert_eq!(load_receipts(&state).len(), 2, "second tick must not re-run");

    // Force due: next_due in the past. Tick must run again (idempotent only when not due).
    let mut forced = saved.clone();
    forced["routines"]["standing-classify"]["next_due"] = serde_json::json!(1);
    std::fs::write(
        state.join("routine-state.json"),
        serde_json::to_string_pretty(&forced).unwrap(),
    )
    .unwrap();
    let (ok, stdout, stderr) = run(&[
        "routine",
        "tick",
        "--id",
        "standing-classify",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("ticked standing-classify"), "{stdout}");
    assert_eq!(load_receipts(&state).len(), 4);

    let (ok, status, stderr) = run(&[
        "routine",
        "status",
        "--id",
        "standing-classify",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "{stderr}");
    assert!(status.contains("last_run=20"), "{status}");
    assert!(status.contains("next_due=20"), "{status}");
    assert_locked_cksum();
}

#[test]
fn routine_digest_and_tick_report_shape() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("digest");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "routine",
        "digest",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("routine digest ran=0 skipped=1"), "{stdout}");
    assert!(stdout.contains("standing-classify status=skipped"), "{stdout}");
    assert!(stdout.contains("package=classify-ping"), "{stdout}");
    assert!(stdout.contains("receipts=0"), "{stdout}");
    assert!(!stdout.contains("Grok Bot sync"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");

    let (ok, stdout, stderr) = run(&[
        "routine",
        "tick",
        "--id",
        "standing-classify",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
        "--report",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("ticked standing-classify"), "{stdout}");
    assert!(stdout.contains("routine digest ran=1 skipped=0"), "{stdout}");
    assert!(stdout.contains("standing-classify status=ran package=classify-ping"), "{stdout}");
    assert!(stdout.contains("receipt="), "{stdout}");
    assert!(stdout.contains("package=classify-ping"), "{stdout}");
    assert!(stdout.contains("chain=-") || stdout.contains("chain=chain-"), "{stdout}");
    assert!(stdout.contains("completion_label=-"), "{stdout}");

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 2, "{rows:?}");
    let receipt_id = rows[0]["id"].as_str().expect("id");
    assert!(stdout.contains(&format!("receipt={receipt_id}")), "{stdout}");
    assert!(stdout.contains("session_id="), "{stdout}");
    assert!(stdout.contains("context=applied"), "{stdout}");

    // Operator-written label still prints (local journal only).
    let mut labeled = rows[0].clone();
    labeled["completion_label"] = serde_json::json!("Sci/Tech");
    std::fs::write(
        journal(&state),
        format!("{}\n", serde_json::to_string(&labeled).unwrap()),
    )
    .unwrap();
    let (ok, digest, stderr) = run(&[
        "routine",
        "digest",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "{stderr}");
    assert!(digest.contains("completion_label=Sci/Tech"), "{digest}");
    assert!(digest.contains(&format!("receipt={receipt_id}")), "{digest}");
    assert_locked_cksum();
}

#[test]
fn package_run_chain_stamps_chain_id_and_ordered_handoffs() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("chain");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "package",
        "run",
        "--id",
        "classify-ping",
        "--chain",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("chain chain-classify-ping-"), "{stdout}");
    assert!(stdout.contains("hops=2"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 2, "{rows:?}");
    let chain_id = rows[0]["chain_id"].as_str().expect("chain_id");
    assert!(
        chain_id.starts_with("chain-classify-ping-"),
        "{chain_id}"
    );
    assert_eq!(rows[1]["chain_id"], chain_id);

    assert_eq!(rows[0]["handoff_from"], "horizon");
    assert_eq!(rows[0]["handoff_to"], "research");
    assert_eq!(rows[0]["capability"], "ag_news");
    assert_eq!(rows[0]["package_id"], "classify-ping");
    assert_eq!(rows[0]["pack_id"], "research-crew");

    assert_eq!(rows[1]["handoff_from"], "research");
    assert_eq!(rows[1]["handoff_to"], "horizon");
    assert_eq!(rows[1]["capability"], "frontier_http");

    for row in &rows {
        let hops = row["handoffs"].as_array().expect("handoffs");
        assert_eq!(hops.len(), 2, "{hops:?}");
        assert_eq!(hops[0]["handoff_from"], "horizon");
        assert_eq!(hops[0]["handoff_to"], "research");
        assert_eq!(hops[0]["binding"], "ag_news");
        assert_eq!(hops[1]["handoff_from"], "research");
        assert_eq!(hops[1]["handoff_to"], "horizon");
        assert_eq!(hops[1]["binding"], "frontier_http");
    }

    let (ok, report, stderr) = run(&["decisions", "report", "--state-dir", &state_s]);
    assert!(ok, "{stderr}");
    assert!(report.contains(&format!("chain={chain_id}")), "{report}");

    let (ok, stdout, stderr) = run(&[
        "package",
        "run",
        "--id",
        "classify-once",
        "--chain",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
    ]);
    assert!(!ok, "stdout={stdout}");
    assert!(stderr.contains("refuse:no-chain"), "{stderr}");
    assert_locked_cksum();
}

#[test]
fn package_run_without_chain_stays_single_hop() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("single");
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
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(rows[0].get("chain_id").map(|v| v.is_null()).unwrap_or(true));
    assert!(
        rows[0]
            .get("handoffs")
            .map(|v| v.as_array().map(|a| a.is_empty()).unwrap_or(true))
            .unwrap_or(true)
    );
    assert_eq!(rows[0]["handoff_from"], "horizon");
    assert_eq!(rows[0]["handoff_to"], "research");
    assert_locked_cksum();
}

#[test]
fn routine_watch_two_cycles_test_hook_no_five_minute_sleep() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("watch");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args([
            "routine",
            "watch",
            "--id",
            "standing-classify",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
            "--mock",
            "--interval",
            "5m",
            "--max-cycles",
            "2",
        ])
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .env_remove("CELL_LOCAL_LIVE")
        .env("CELL_ROUTINE_WATCH_INTERVAL_SECS", "0")
        .output()
        .unwrap();
    let ok = out.status.success();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.contains("routine watch interval=0s max_cycles=2 stop=SIGINT (Ctrl-C) live_sync=false"),
        "{stdout}"
    );
    assert!(stdout.contains("routine watch cycle=1"), "{stdout}");
    assert!(stdout.contains("routine watch cycle=2"), "{stdout}");
    assert!(!stdout.contains("routine watch cycle=3"), "{stdout}");
    assert!(stdout.contains("ticked standing-classify"), "{stdout}");
    assert!(stdout.contains("skip standing-classify"), "{stdout}");
    assert!(stdout.contains("routine digest ran=1 skipped=0"), "{stdout}");
    assert!(
        stdout.contains("standing-classify status=ran package=classify-ping"),
        "{stdout}"
    );
    assert!(stdout.contains("receipt="), "{stdout}");
    assert!(
        stdout.contains("routine watch stopped cycles=2 reason=max-cycles"),
        "{stdout}"
    );
    assert!(!stdout.contains("Grok Bot sync"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 2, "second cycle must skip; idempotent tick {rows:?}");
    assert_eq!(rows[0]["routine_id"], "standing-classify");
    assert_eq!(rows[1]["session_context"], true);
    let saved = load_routine_state(&state);
    assert_eq!(saved["schema"], "cell-one.routine-state.v0");
    assert!(saved["routines"]["standing-classify"]["last_run"].is_number());
    assert!(saved["routines"]["standing-classify"]["next_due"].is_number());
    assert_locked_cksum();
}

#[test]
fn routine_watch_refuses_sub_five_minute_interval_without_hook() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("watch-refuse");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "routine",
        "watch",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
        "--interval",
        "1m",
        "--max-cycles",
        "1",
    ]);
    assert!(!ok, "stdout={stdout}");
    assert!(stderr.contains("refuse:watch-interval"), "{stderr}");
    assert!(!state.join("routine-state.json").exists());
    assert_locked_cksum();
}

#[test]
fn routine_watch_one_cycle_is_tick_plus_digest() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("watch-once");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "routine",
        "watch",
        "--id",
        "standing-classify",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
        "--max-cycles",
        "1",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.contains("routine watch interval=5m (300s) max_cycles=1"),
        "{stdout}"
    );
    assert!(stdout.contains("routine watch cycle=1"), "{stdout}");
    assert!(stdout.contains("ticked standing-classify"), "{stdout}");
    assert!(stdout.contains("routine digest ran=1 skipped=0"), "{stdout}");
    assert!(
        stdout.contains("routine watch stopped cycles=1 reason=max-cycles"),
        "{stdout}"
    );
    assert!(!stdout.contains("Grok Bot sync"), "{stdout}");
    assert_eq!(load_receipts(&state).len(), 2);
    assert!(stdout.contains("session_id="), "{stdout}");
    assert!(stdout.contains("context=applied"), "{stdout}");
    assert_locked_cksum();
}
