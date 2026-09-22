//! Doctor fails a present lifecycle.jsonl that does not parse.
//! A missing file is not a failure, and is not invented as zero lines.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-doctor-history-{}-{}",
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
fn doctor_refuses_an_unreadable_lifecycle_log_before_factory_ready() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("history");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();

    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("lifecycle.jsonl lines="),
        "missing file must not invent a line count: {text}"
    );

    let history = state.join("lifecycle.jsonl");
    std::fs::write(&history, "not-json\n").unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("lifecycle.jsonl"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(!text.contains("lifecycle.jsonl lines="), "{text}");
    assert_eq!(std::fs::read_to_string(&history).unwrap(), "not-json\n");

    std::fs::write(
        &history,
        "{\"ts\":\"2026-09-22T00:00:00Z\",\"from\":null,\"to\":\"suspended\",\"action\":\"suspend\",\"desired_hash\":null,\"note\":\"paused\"}\n",
    )
    .unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("lifecycle.jsonl lines=1"), "{text}");
    assert!(text.contains("factory ready"), "{text}");
}
