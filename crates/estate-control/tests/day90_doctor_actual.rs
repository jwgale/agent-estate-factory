//! Doctor fails a present actual-state.json that does not parse.
//! A missing file is not a failure, and is not invented as zero sessions.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-doctor-actual-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn run(bin: &str, args: &[&str]) -> (bool, String) {
    let out = Command::new(bin)
        .args(args)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

fn doctor(bin: &str, root: &PathBuf, state: &PathBuf) -> (bool, String) {
    run(
        bin,
        &[
            "doctor",
            "--root",
            &root.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ],
    )
}

#[test]
fn doctor_refuses_an_unreadable_actual_state_before_factory_ready() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("actual");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();

    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("actual-state.json sessions="),
        "missing file must not invent a session count: {text}"
    );

    let actual = state.join("actual-state.json");
    std::fs::write(&actual, "not-json\n").unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("actual-state.json"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(!text.contains("actual-state.json sessions="), "{text}");
    assert_eq!(std::fs::read_to_string(&actual).unwrap(), "not-json\n");

    std::fs::write(
        &actual,
        "{\"desired_hash\":\"abc\",\"estate_name\":\"cell\",\"sessions\":[{\"agent_id\":\"a\",\"desktop\":\"d\",\"lane_id\":\"l\",\"isolation_handle\":{\"driver\":\"profile-dir\",\"key\":\"k\",\"path\":\"p\"},\"session_path\":\"s\",\"lane_root\":\"r\"}],\"generated_at\":\"2026-09-22T00:00:00Z\",\"regenerable\":true}\n",
    )
    .unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("actual-state.json sessions=1"), "{text}");
    assert!(text.contains("factory ready"), "{text}");
}
