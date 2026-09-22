//! Doctor fails a present lifecycle.json that does not parse.
//! A missing file is not a failure, and is not invented as suspended.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-doctor-life-{}-{}",
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
fn doctor_refuses_an_unreadable_lifecycle_before_factory_ready() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("life");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();

    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("lifecycle.json state="),
        "missing file must not invent a state: {text}"
    );

    let life = state.join("lifecycle.json");
    std::fs::write(&life, "not-json").unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("lifecycle.json"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(!text.contains("lifecycle.json state="), "{text}");
    assert_eq!(std::fs::read_to_string(&life).unwrap(), "not-json");

    std::fs::write(
        &life,
        r#"{"schema":"cell-one.lifecycle.v0","version":1,"state":"suspended","desired_hash":null,"estate_name":null,"suspended_at":null,"resumed_at":null,"durable":true,"note":"paused"}"#,
    )
    .unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("lifecycle.json state=suspended"), "{text}");
    assert!(text.contains("factory ready"), "{text}");

    std::fs::write(
        &life,
        r#"{"schema":"cell-one.lifecycle.v0","version":1,"state":"running","desired_hash":null,"estate_name":null,"suspended_at":null,"resumed_at":null,"durable":true,"note":"up"}"#,
    )
    .unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("lifecycle.json state=running"), "{text}");
    assert!(text.contains("factory ready"), "{text}");
}
