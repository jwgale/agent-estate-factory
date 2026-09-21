use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn floor_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_floor"))
}

#[test]
fn apply_suspend_resume_leases() {
    let tmp = repo_root().join(format!("target/test-floor-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let state = tmp.join("state");
    let estate = repo_root().join("examples/estate.yaml");
    let applied = floor_bin()
        .args([
            "apply",
            "--estate",
            &estate.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&applied.stderr)
    );
    assert!(state.join("placement-actual.json").is_file());
    assert!(state.join("lifecycle.json").is_file());
    let leases = floor_bin()
        .args(["leases", "--state-dir", &state.display().to_string()])
        .output()
        .unwrap();
    assert!(leases.status.success());
    let stdout = String::from_utf8_lossy(&leases.stdout);
    assert!(stdout.contains("cloud-agent"));
    let suspended = floor_bin()
        .args(["suspend", "--state-dir", &state.display().to_string()])
        .output()
        .unwrap();
    assert!(suspended.status.success());
    assert!(!state.join("sessions").exists());
    let after = std::fs::read_to_string(state.join("placement-actual.json")).unwrap();
    assert!(after.contains("\"spawned\": false"));
    let resumed = floor_bin()
        .args([
            "resume",
            "--estate",
            &estate.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        resumed.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    let history = floor_bin()
        .args(["history", "--state-dir", &state.display().to_string()])
        .output()
        .unwrap();
    assert!(history.status.success());
    assert!(String::from_utf8_lossy(&history.stdout).contains("lifecycle history"));
    let _ = std::fs::remove_dir_all(&tmp);
}
