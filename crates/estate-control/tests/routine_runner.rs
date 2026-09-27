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
