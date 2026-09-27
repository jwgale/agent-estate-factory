//! Supervised routine runner: start / status / tick / stop.
//!
//! Detached host loop over the same idempotent tick + digest as
//! `estate routine watch`. Locked examples/estate.yaml stays untouched.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("cell-routine-runner-{name}-{nanos}"));
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

fn run_hook(args: &[&str], interval_secs: &str) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args(args)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .env_remove("CELL_LOCAL_LIVE")
        .env("CELL_ROUTINE_WATCH_INTERVAL_SECS", interval_secs)
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

fn wait_until(timeout: Duration, mut pred: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if pred() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    pred()
}

struct RunnerGuard {
    state: PathBuf,
}

impl Drop for RunnerGuard {
    fn drop(&mut self) {
        let state_s = self.state.display().to_string();
        let _ = run(&[
            "routine",
            "runner",
            "stop",
            "--state-dir",
            &state_s,
            "--runner-id",
            "default",
        ]);
    }
}

fn start_args<'a>(
    estate: &'a str,
    state: &'a str,
) -> [&'a str; 14] {
    [
        "routine",
        "runner",
        "start",
        "--id",
        "standing-classify",
        "--estate",
        estate,
        "--state-dir",
        state,
        "--mock",
        "--max-cycles",
        "30",
        "--interval",
        "5m",
    ]
}

#[test]
fn runner_start_status_tick_stop() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("loop");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();
    let _guard = RunnerGuard {
        state: state.clone(),
    };

    let (ok, stdout, stderr) = run_hook(&start_args(&estate_s, &state_s), "2");
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("routine runner started"), "{stdout}");
    assert!(stdout.contains("id=default"), "{stdout}");
    assert!(stdout.contains("live_sync=false"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!stdout.contains("XAI_API_KEY"), "{stdout}");

    let (ok, status, stderr) = run(&[
        "routine",
        "runner",
        "status",
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "{stderr}");
    assert!(status.contains("status: running"), "{status}");
    assert!(status.contains("pid:"), "{status}");
    assert!(status.contains("uptime:"), "{status}");
    assert!(status.contains("selected: standing-classify"), "{status}");
    assert!(status.contains("live_sync: false"), "{status}");

    let digest_log = state.join("routine-runner").join("default.digest.log");
    let ticked = wait_until(Duration::from_secs(15), || {
        if !digest_log.is_file() {
            return false;
        }
        let text = std::fs::read_to_string(&digest_log).unwrap_or_default();
        text.contains("routine digest ran=")
    });
    assert!(ticked, "runner never appended a digest under {}", digest_log.display());

    let digest = std::fs::read_to_string(&digest_log).unwrap();
    assert!(digest.contains("routine digest ran=1 skipped=0"), "{digest}");
    assert!(digest.contains("standing-classify status=ran"), "{digest}");
    assert!(digest.contains("package=classify-ping"), "{digest}");
    assert!(digest.contains("session_id="), "{digest}");
    assert!(digest.contains("context=applied"), "{digest}");
    assert!(!digest.contains("Grok Bot sync"), "{digest}");
    assert!(!digest.contains("live PASS"), "{digest}");
    assert!(!digest.contains("XAI_API_KEY"), "{digest}");

    let (ok, status, stderr) = run(&[
        "routine",
        "runner",
        "status",
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "{stderr}");
    assert!(status.contains("status: running"), "{status}");
    assert!(status.contains("last_digest: routine digest ran=1 skipped=0"), "{status}");
    assert!(status.contains("session_id="), "{status}");
    assert!(status.contains("context=applied"), "{status}");
    assert!(!status.contains("last_tick: -"), "{status}");

    let saved = std::fs::read_to_string(state.join("routine-state.json")).unwrap();
    assert!(saved.contains("cell-one.routine-state.v0"), "{saved}");
    assert!(saved.contains("standing-classify"), "{saved}");

    let (ok, stdout, stderr) = run(&[
        "routine",
        "runner",
        "stop",
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("routine runner stopped"), "{stdout}");
    assert!(stdout.contains("id=default"), "{stdout}");

    let (ok, status, stderr) = run(&[
        "routine",
        "runner",
        "status",
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "{stderr}");
    assert!(status.contains("status: stopped"), "{status}");
    assert!(status.contains("last_digest: routine digest ran=1 skipped=0"), "{status}");
    assert!(!status.contains("status: running"), "{status}");

    // Idempotent tick survived the runner: a second watch-style tick skips.
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
    assert_locked_cksum();
}

#[test]
fn runner_refuses_double_start() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("double");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();
    let _guard = RunnerGuard {
        state: state.clone(),
    };

    let (ok, stdout, stderr) = run_hook(&start_args(&estate_s, &state_s), "2");
    assert!(ok, "stderr={stderr}\nstdout={stdout}");

    let running = wait_until(Duration::from_secs(8), || {
        let (ok, status, _) = run(&["routine", "runner", "status", "--state-dir", &state_s]);
        ok && status.contains("status: running")
    });
    assert!(running, "first start never showed running");

    let (ok, stdout, stderr) = run_hook(&start_args(&estate_s, &state_s), "2");
    assert!(!ok, "stdout={stdout}");
    assert!(
        stderr.contains("refuse:runner-already-running"),
        "stderr={stderr}\nstdout={stdout}"
    );
    assert!(stderr.contains("id=default"), "{stderr}");

    let (ok, _, stderr) = run(&["routine", "runner", "stop", "--state-dir", &state_s]);
    assert!(ok, "{stderr}");
    assert_locked_cksum();
}

#[test]
fn runner_stop_missing_refuses() {
    assert_locked_cksum();
    let dir = scratch("missing");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&["routine", "runner", "stop", "--state-dir", &state_s]);
    assert!(!ok, "stdout={stdout}");
    assert!(
        stderr.contains("refuse:runner-not-running"),
        "stderr={stderr}\nstdout={stdout}"
    );

    let (ok, status, stderr) = run(&["routine", "runner", "status", "--state-dir", &state_s]);
    assert!(ok, "{stderr}");
    assert!(status.contains("status: stopped"), "{status}");
    assert!(status.contains("last_tick: -"), "{status}");
    assert_locked_cksum();
}

#[test]
fn runner_start_refuses_sub_five_minute_interval_without_hook() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("interval");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "routine",
        "runner",
        "start",
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
    assert!(!state.join("routine-runner").join("default.pid").exists());
    assert_locked_cksum();
}

#[test]
fn runner_restart_replaces_a_live_child() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("restart");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();
    let _guard = RunnerGuard {
        state: state.clone(),
    };

    let (ok, stdout, stderr) = run_hook(&start_args(&estate_s, &state_s), "2");
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    let first_pid = wait_until(Duration::from_secs(8), || {
        let (ok, status, _) = run(&["routine", "runner", "status", "--state-dir", &state_s]);
        ok && status.contains("status: running")
    });
    assert!(first_pid, "start never reached running");
    let (_, before, _) = run(&["routine", "runner", "status", "--state-dir", &state_s]);
    let pid_before = pid_from_status(&before);

    let (ok, stdout, stderr) = run_hook(
        &[
            "routine",
            "runner",
            "restart",
            "--id",
            "standing-classify",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
            "--mock",
            "--max-cycles",
            "30",
            "--interval",
            "5m",
        ],
        "2",
    );
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.contains("routine runner started") || stdout.contains("routine runner stopped"),
        "{stdout}"
    );

    let replaced = wait_until(Duration::from_secs(8), || {
        let (ok, status, _) = run(&["routine", "runner", "status", "--state-dir", &state_s]);
        if !ok || !status.contains("status: running") {
            return false;
        }
        let pid_after = pid_from_status(&status);
        match (pid_before, pid_after) {
            (Some(a), Some(b)) => a != b,
            _ => true,
        }
    });
    assert!(replaced, "restart did not publish a new running pid");
    assert_locked_cksum();
}

fn journal(state: &std::path::Path) -> PathBuf {
    state.join("decisions").join("receipts.jsonl")
}

fn load_receipts(state: &std::path::Path) -> Vec<serde_json::Value> {
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

fn force_due(state: &std::path::Path, routine_id: &str) {
    let path = state.join("routine-state.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut saved: serde_json::Value = serde_json::from_str(&text).unwrap();
    saved["routines"][routine_id]["next_due"] = serde_json::json!(1);
    std::fs::write(path, serde_json::to_string_pretty(&saved).unwrap()).unwrap();
}

fn pid_from_status(status: &str) -> Option<u32> {
    status
        .lines()
        .find(|line| line.trim().starts_with("pid:"))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|raw| raw.trim().parse().ok())
}

#[test]
fn runner_files_are_throwaway_under_state_dir() {
    let dir = scratch("layout");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    assert!(!state.join("routine-runner").exists());
    let state_s = state.display().to_string();
    let (ok, _, stderr) = run(&[
        "routine",
        "runner",
        "status",
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "{stderr}");
    // status of a missing runner does not invent a live pidfile
    assert!(!state.join("routine-runner").join("default.pid").exists());
    assert_locked_cksum();
}

#[test]
fn runner_tick_stitches_crew_session_on_multihop_chain() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("stitch");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();
    let _guard = RunnerGuard {
        state: state.clone(),
    };

    let (ok, stdout, stderr) = run_hook(&start_args(&estate_s, &state_s), "2");
    assert!(ok, "stderr={stderr}\nstdout={stdout}");

    let digest_log = state.join("routine-runner").join("default.digest.log");
    let ticked = wait_until(Duration::from_secs(15), || {
        let text = std::fs::read_to_string(&digest_log).unwrap_or_default();
        text.contains("context=applied") && text.contains("session_id=")
    });
    assert!(ticked, "runner never cited a session digest");

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0]["session_context"], false);
    assert_eq!(rows[1]["session_context"], true);
    let sid = rows[0]["session_id"].as_str().expect("session_id");
    assert!(sid.starts_with("sess-"), "{sid}");
    assert_eq!(rows[1]["session_id"], sid);
    assert_eq!(rows[0]["handoff_from"], "horizon");
    assert_eq!(rows[0]["handoff_to"], "research");
    assert_eq!(rows[1]["handoff_from"], "research");
    assert_eq!(rows[1]["handoff_to"], "horizon");

    let digest = std::fs::read_to_string(&digest_log).unwrap();
    assert!(digest.contains(&format!("session_id={sid}")), "{digest}");
    assert!(digest.contains("context=applied"), "{digest}");
    assert!(digest.contains("context=none"), "{digest}");

    let saved = std::fs::read_to_string(state.join("routine-state.json")).unwrap();
    assert!(saved.contains(sid), "{saved}");
    assert!(saved.contains("cell-one.routine-state.v0"), "{saved}");

    let (ok, _, stderr) = run(&["routine", "runner", "stop", "--state-dir", &state_s]);
    assert!(ok, "{stderr}");
    assert_locked_cksum();
}

#[test]
fn runner_reuses_open_session_then_mints_after_end() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("reuse");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();
    let _guard = RunnerGuard {
        state: state.clone(),
    };

    let (ok, stdout, stderr) = run_hook(&start_args(&estate_s, &state_s), "2");
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    let digest_log = state.join("routine-runner").join("default.digest.log");
    let ticked = wait_until(Duration::from_secs(15), || {
        load_receipts(&state).len() >= 2
    });
    assert!(ticked, "first tick missing under {}", digest_log.display());
    let first_sid = load_receipts(&state)[0]["session_id"]
        .as_str()
        .expect("session")
        .to_string();

    let (ok, _, stderr) = run(&["routine", "runner", "stop", "--state-dir", &state_s]);
    assert!(ok, "{stderr}");

    force_due(&state, "standing-classify");
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
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 4, "{rows:?}");
    assert_eq!(rows[2]["session_id"], first_sid);
    assert_eq!(rows[3]["session_id"], first_sid);
    assert_eq!(rows[2]["session_context"], true);
    assert_eq!(rows[3]["session_context"], true);

    let (ok, stdout, stderr) = run(&[
        "pack",
        "session",
        "end",
        "--id",
        &first_sid,
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");

    force_due(&state, "standing-classify");
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
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 6, "{rows:?}");
    let fresh = rows[4]["session_id"].as_str().expect("fresh");
    assert_ne!(fresh, first_sid);
    assert_eq!(rows[4]["session_context"], false);
    assert_eq!(rows[5]["session_id"], fresh);
    assert_eq!(rows[5]["session_context"], true);
    assert_locked_cksum();
}

#[test]
fn runner_prove_cli_stitches_and_refuses_double_start() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("prove");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "routine",
        "runner-prove",
        "--id",
        "standing-classify",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("routine runner-prove: standing-classify"), "{stdout}");
    assert!(stdout.contains("ok: yes"), "{stdout}");
    assert!(stdout.contains("hop2-context-applied: ok"), "{stdout}");
    assert!(stdout.contains("session-reuse: ok"), "{stdout}");
    assert!(stdout.contains("ended-creates-fresh: ok"), "{stdout}");
    assert!(stdout.contains("partial-start-refuse: ok"), "{stdout}");
    assert!(stdout.contains("double-start-refuse: ok"), "{stdout}");
    assert!(stdout.contains("live_sync: no"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stdout.contains("XAI_API_KEY"), "{stdout}");
    assert!(!state.join("routine-runner").join("default.pid").exists());
    assert_locked_cksum();
}

fn start_multi_args<'a>(estate: &'a str, state: &'a str) -> [&'a str; 16] {
    [
        "routine",
        "runner",
        "start",
        "--id",
        "standing-classify",
        "--id",
        "standing-once",
        "--estate",
        estate,
        "--state-dir",
        state,
        "--mock",
        "--max-cycles",
        "30",
        "--interval",
        "5m",
    ]
}

fn write_patched_fixture(dir: &std::path::Path, patch: impl FnOnce(&str) -> String) -> std::path::PathBuf {
    let src = std::fs::read_to_string(fixture()).unwrap();
    let path = dir.join("estate.yaml");
    std::fs::write(&path, patch(&src)).unwrap();
    path
}

#[test]
fn runner_multi_id_select_covers_both_and_one_pidfile() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("multi");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();
    let _guard = RunnerGuard {
        state: state.clone(),
    };

    let (ok, stdout, stderr) = run_hook(&start_multi_args(&estate_s, &state_s), "2");
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("selected=standing-classify,standing-once"), "{stdout}");
    assert!(stdout.contains("live_sync=false"), "{stdout}");

    let digest_log = state.join("routine-runner").join("default.digest.log");
    let ticked = wait_until(Duration::from_secs(15), || {
        let text = std::fs::read_to_string(&digest_log).unwrap_or_default();
        text.contains("standing-classify status=ran") && text.contains("standing-once status=ran")
    });
    assert!(ticked, "multi-id runner never digested both routines");

    let digest = std::fs::read_to_string(&digest_log).unwrap();
    assert!(digest.contains("routine digest ran=2 skipped=0"), "{digest}");
    assert!(digest.contains("standing-classify status=ran package=classify-ping"), "{digest}");
    assert!(digest.contains("standing-once status=ran package=classify-once"), "{digest}");
    assert!(digest.contains("session_id="), "{digest}");
    assert!(digest.contains("context=applied"), "{digest}");
    assert!(!digest.contains("XAI_API_KEY"), "{digest}");
    assert!(!digest.contains("live PASS"), "{digest}");

    let runner_dir = state.join("routine-runner");
    let pidfiles: Vec<_> = std::fs::read_dir(&runner_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("pid"))
        .collect();
    assert_eq!(pidfiles.len(), 1, "{pidfiles:?}");
    assert!(runner_dir.join("default.pid").exists());

    let (ok, status, stderr) = run(&["routine", "runner", "status", "--state-dir", &state_s]);
    assert!(ok, "{stderr}");
    assert!(status.contains("selected: standing-classify,standing-once"), "{status}");
    assert!(status.contains("last_outcome standing-classify: ran"), "{status}");
    assert!(status.contains("last_outcome standing-once: ran"), "{status}");
    assert!(status.contains("status: running"), "{status}");

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 3, "classify 2 hops + once 1 hop: {rows:?}");
    assert_eq!(rows.iter().filter(|r| r["routine_id"] == "standing-classify").count(), 2);
    assert_eq!(rows.iter().filter(|r| r["routine_id"] == "standing-once").count(), 1);

    let (ok, _, stderr) = run(&["routine", "runner", "stop", "--state-dir", &state_s]);
    assert!(ok, "{stderr}");
    assert_locked_cksum();
}

#[test]
fn runner_multi_id_comma_select_and_default_all() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("comma");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();
    let _guard = RunnerGuard {
        state: state.clone(),
    };

    let (ok, stdout, stderr) = run_hook(
        &[
            "routine",
            "runner",
            "start",
            "--id",
            "standing-classify,standing-once",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
            "--mock",
            "--max-cycles",
            "30",
            "--interval",
            "5m",
        ],
        "2",
    );
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("selected=standing-classify,standing-once"), "{stdout}");

    let digest_log = state.join("routine-runner").join("default.digest.log");
    let ticked = wait_until(Duration::from_secs(15), || {
        let text = std::fs::read_to_string(&digest_log).unwrap_or_default();
        text.contains("standing-once status=ran")
    });
    assert!(ticked, "comma --id never ticked standing-once");
    let (ok, _, stderr) = run(&["routine", "runner", "stop", "--state-dir", &state_s]);
    assert!(ok, "{stderr}");
    assert_locked_cksum();
}

#[test]
fn runner_start_refuses_partial_unknown_disabled_invalid() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("partial");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "routine",
        "runner",
        "start",
        "--id",
        "standing-classify",
        "--id",
        "missing-routine",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
        "--max-cycles",
        "1",
    ]);
    assert!(!ok, "stdout={stdout}");
    assert!(
        stderr.contains("refuse:runner-routine-unknown"),
        "stderr={stderr}\nstdout={stdout}"
    );
    assert!(stderr.contains("id=missing-routine"), "{stderr}");
    assert!(!state.join("routine-runner").join("default.pid").exists());

    let disabled_dir = scratch("disabled-estate");
    let disabled = write_patched_fixture(&disabled_dir, |src| {
        src.replace(
            "  - id: standing-once\n    package: classify-once\n    schedule: \"@hourly\"",
            "  - id: standing-once\n    package: classify-once\n    schedule: \"@hourly\"\n    enabled: false",
        )
    });
    let disabled_s = disabled.display().to_string();
    let (ok, stdout, stderr) = run(&[
        "routine",
        "runner",
        "start",
        "--id",
        "standing-classify,standing-once",
        "--estate",
        &disabled_s,
        "--state-dir",
        &state_s,
        "--mock",
        "--max-cycles",
        "1",
    ]);
    assert!(!ok, "stdout={stdout}");
    assert!(
        stderr.contains("refuse:runner-routine-disabled"),
        "stderr={stderr}\nstdout={stdout}"
    );
    assert!(stderr.contains("id=standing-once"), "{stderr}");
    assert!(!state.join("routine-runner").join("default.pid").exists());

    let invalid_dir = scratch("invalid-estate");
    let invalid = write_patched_fixture(&invalid_dir, |src| {
        src.replace(
            "  - id: standing-once\n    package: classify-once\n    schedule: \"@hourly\"\n    note: Single-hop standing routine for multi-id supervise. Not Grok Bot sync.",
            "  - id: standing-once\n    package: classify-once\n    note: Unscheduled; invalid for named runner start.",
        )
    });
    let invalid_s = invalid.display().to_string();
    let (ok, stdout, stderr) = run(&[
        "routine",
        "runner",
        "start",
        "--id",
        "standing-classify,standing-once",
        "--estate",
        &invalid_s,
        "--state-dir",
        &state_s,
        "--mock",
        "--max-cycles",
        "1",
    ]);
    assert!(!ok, "stdout={stdout}");
    assert!(
        stderr.contains("refuse:runner-routine-invalid"),
        "stderr={stderr}\nstdout={stdout}"
    );
    assert!(stderr.contains("id=standing-once"), "{stderr}");
    assert!(!state.join("routine-runner").join("default.pid").exists());
    assert_locked_cksum();
}

#[test]
fn runner_prove_cli_multi_id_default_covers_both() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("prove-multi");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "routine",
        "runner-prove",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.contains("routine runner-prove: standing-classify,standing-once"),
        "{stdout}"
    );
    assert!(stdout.contains("ok: yes"), "{stdout}");
    assert!(stdout.contains("digest-covers-standing-classify: ok"), "{stdout}");
    assert!(stdout.contains("digest-covers-standing-once: ok"), "{stdout}");
    assert!(stdout.contains("status-selected: ok"), "{stdout}");
    assert!(stdout.contains("status-outcome-standing-once: ok"), "{stdout}");
    assert!(stdout.contains("one-pidfile: ok"), "{stdout}");
    assert!(stdout.contains("partial-start-refuse: ok"), "{stdout}");
    assert!(stdout.contains("hop2-context-applied: ok"), "{stdout}");
    assert!(stdout.contains("double-start-refuse: ok"), "{stdout}");
    assert!(stdout.contains("live_sync: no"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!state.join("routine-runner").join("default.pid").exists());
    assert_locked_cksum();
}

#[test]
fn runner_prove_cli_dual_standing_id() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("prove-dual");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "routine",
        "runner-prove",
        "--id",
        "standing-dual",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("routine runner-prove: standing-dual"), "{stdout}");
    assert!(stdout.contains("ok: yes"), "{stdout}");
    assert!(stdout.contains("hop1-ag_news: ok"), "{stdout}");
    assert!(stdout.contains("hop2-rust_idiom: ok"), "{stdout}");
    assert!(stdout.contains("hop3-frontier_http: ok"), "{stdout}");
    assert!(stdout.contains("hop1-context-none: ok"), "{stdout}");
    assert!(stdout.contains("hop2-context-applied: ok"), "{stdout}");
    assert!(stdout.contains("hop3-context-applied: ok"), "{stdout}");
    assert!(stdout.contains("dual-same-session: ok"), "{stdout}");
    assert!(stdout.contains("dual-package: ok"), "{stdout}");
    assert!(stdout.contains("dual-chain: ok"), "{stdout}");
    assert!(stdout.contains("digest-cites-session: ok"), "{stdout}");
    assert!(stdout.contains("digest-cites-dual-package: ok"), "{stdout}");
    assert!(stdout.contains("digest-cites-dual-chain: ok"), "{stdout}");
    assert!(stdout.contains("session-reuse: ok"), "{stdout}");
    assert!(stdout.contains("ended-creates-fresh: ok"), "{stdout}");
    assert!(stdout.contains("live_sync: no"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stdout.contains("XAI_API_KEY"), "{stdout}");
    assert!(!state.join("routine-runner").join("default.pid").exists());
    assert_locked_cksum();
}

#[test]
fn runner_prove_cli_dual_flag_with_other_standing_id() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("prove-dual-flag");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "routine",
        "runner-prove",
        "--dual",
        "--id",
        "standing-once",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.contains("routine runner-prove: standing-dual,standing-once"),
        "{stdout}"
    );
    assert!(stdout.contains("ok: yes"), "{stdout}");
    assert!(stdout.contains("hop3-frontier_http: ok"), "{stdout}");
    assert!(stdout.contains("hop3-context-applied: ok"), "{stdout}");
    assert!(stdout.contains("digest-covers-standing-dual: ok"), "{stdout}");
    assert!(stdout.contains("digest-covers-standing-once: ok"), "{stdout}");
    assert!(stdout.contains("digest-cites-dual-package: ok"), "{stdout}");
    assert!(stdout.contains("digest-cites-dual-chain: ok"), "{stdout}");
    assert!(stdout.contains("status-selected: ok"), "{stdout}");
    assert!(stdout.contains("status-outcome-standing-once: ok"), "{stdout}");
    assert!(stdout.contains("one-pidfile: ok"), "{stdout}");
    assert!(stdout.contains("session-reuse: ok"), "{stdout}");
    assert!(stdout.contains("double-start-refuse: ok"), "{stdout}");
    assert!(stdout.contains("live_sync: no"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!state.join("routine-runner").join("default.pid").exists());
    assert_locked_cksum();
}
