//! Doctor fails a present desired-snapshot.yaml that does not parse
//! when no cell catalog is present. A missing file is not a failure,
//! and is not invented as a frontier model.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-doctor-snapshot-{}-{}",
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
fn doctor_refuses_an_unreadable_snapshot_before_factory_ready() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("snapshot");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();

    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("desired-snapshot.yaml name="),
        "missing file must not invent a snapshot name: {text}"
    );

    let snap = state.join("desired-snapshot.yaml");
    std::fs::write(&snap, "not-yaml: [\n").unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("desired-snapshot.yaml"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(!text.contains("desired-snapshot.yaml name="), "{text}");
    assert_eq!(std::fs::read_to_string(&snap).unwrap(), "not-yaml: [\n");

    std::fs::write(&snap, "version: 0\nname: cell\nagents: []\nlanes: []\n").unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("desired-snapshot.yaml name=cell"), "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("catalog.json model="),
        "a snapshot name is not a frontier model: {text}"
    );
}
