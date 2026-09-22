//! Status does not invent suspended when lifecycle.json is missing.
//! A present file that does not parse refuses before the page.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-status-life-{}-{}",
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

#[test]
fn status_does_not_invent_suspended_for_a_missing_lifecycle() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("life");
    let state = dir.join("state");
    let plans = dir.join("plans");
    let packs = dir.join("packs");
    std::fs::create_dir_all(&state).unwrap();
    let estate = root.join("examples/estate.yaml");
    let status = [
        "status",
        "--estate",
        &estate.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--roots-base",
        &state.display().to_string(),
        "--plans-dir",
        &plans.display().to_string(),
        "--packs-dir",
        &packs.display().to_string(),
        "--root",
        &root.display().to_string(),
    ];

    let (ok, text) = run(bin, &status);
    assert!(ok, "{text}");
    assert!(text.contains("Cell One status"), "{text}");
    assert!(text.contains("paused: -"), "{text}");
    assert!(text.contains("lifecycle: -"), "{text}");
    assert!(
        !text.contains("suspended"),
        "missing file must not invent suspended: {text}"
    );
    assert!(
        !text.contains("durable="),
        "missing file must not invent durable: {text}"
    );

    let path = state.join("lifecycle.json");
    std::fs::write(&path, "not-json\n").unwrap();
    let (ok, text) = run(bin, &status);
    assert!(!ok, "{text}");
    assert!(text.contains("lifecycle.json"), "{text}");
    assert!(!text.contains("Cell One status"), "{text}");
    assert!(!text.contains("paused: yes"), "{text}");
    assert!(!text.contains("lifecycle: suspended"), "{text}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "not-json\n");

    std::fs::write(
        &path,
        "{\"schema\":\"cell-one.lifecycle.v0\",\"version\":1,\"state\":\"suspended\",\"desired_hash\":null,\"estate_name\":null,\"suspended_at\":null,\"resumed_at\":null,\"durable\":true,\"note\":\"paused\"}\n",
    )
    .unwrap();
    let (ok, text) = run(bin, &status);
    assert!(ok, "{text}");
    assert!(text.contains("paused: yes"), "{text}");
    assert!(text.contains("lifecycle: suspended (durable=true)"), "{text}");

    std::fs::write(
        &path,
        "{\"schema\":\"cell-one.lifecycle.v0\",\"version\":1,\"state\":\"running\",\"desired_hash\":null,\"estate_name\":null,\"suspended_at\":null,\"resumed_at\":null,\"durable\":false,\"note\":\"up\"}\n",
    )
    .unwrap();
    let (ok, text) = run(bin, &status);
    assert!(ok, "{text}");
    assert!(text.contains("paused: no"), "{text}");
    assert!(
        text.contains("lifecycle: running (durable=false)"),
        "{text}"
    );
    assert!(!text.contains("lifecycle: suspended"), "{text}");
}
