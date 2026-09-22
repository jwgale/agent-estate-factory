//! Resume and pause-proof refuse a cell catalog that disagrees with the
//! binding, and pause-proof does not invent grok-4.7.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-pause-frontier-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args(args)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env("CELL_FRONTIER_MODEL", "grok-4.7")
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
fn pause_proof_does_not_invent_grok_and_resume_refuses_disagreement() {
    let root = repo_root();
    let estate = root.join("examples/estate.yaml");
    let dir = tmp("kit");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();

    let (ok, text) = run(&[
        "pause-proof",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--roots-base",
        &state_s,
    ]);
    assert!(ok, "{text}");
    assert!(
        !text.contains("grok-4.7"),
        "pause-proof must not print the schema card as a binding: {text}"
    );
    assert!(
        !state.join("catalog.json").exists(),
        "pause-proof must not invent a cell catalog"
    );
    let leases = std::fs::read(state.join("placement-actual.json")).unwrap();

    let planted = "{\"frontier\":{\"model\":\"grok-4.7\"}}\n";
    let catalog = state.join("catalog.json");
    std::fs::write(&catalog, planted).unwrap();
    let (ok, text) = run(&[
        "pause-proof",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--roots-base",
        &state_s,
    ]);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:frontier-model"), "{text}");
    assert!(text.contains("cell catalog model=grok-4.7"), "{text}");
    assert_eq!(std::fs::read_to_string(&catalog).unwrap(), planted);
    assert_eq!(
        std::fs::read(state.join("placement-actual.json")).unwrap(),
        leases,
        "refused pause-proof must not rewrite leases"
    );

    let fresh = dir.join("resume");
    std::fs::create_dir_all(&fresh).unwrap();
    let fresh_s = fresh.display().to_string();
    let fresh_catalog = fresh.join("catalog.json");
    std::fs::write(&fresh_catalog, planted).unwrap();
    let (ok, text) = run(&[
        "resume",
        "--estate",
        &estate_s,
        "--state-dir",
        &fresh_s,
        "--roots-base",
        &fresh_s,
    ]);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:frontier-model"), "{text}");
    assert!(!text.contains("resumed "), "{text}");
    assert_eq!(std::fs::read_to_string(&fresh_catalog).unwrap(), planted);
    assert!(
        !fresh.join("placement-actual.json").exists(),
        "refused resume must not write leases"
    );

    std::fs::write(&fresh_catalog, "{\"frontier\":{\"model\":\"\"}}\n").unwrap();
    let (ok, text) = run(&[
        "resume",
        "--estate",
        &estate_s,
        "--state-dir",
        &fresh_s,
        "--roots-base",
        &fresh_s,
    ]);
    assert!(ok, "{text}");
    let written = std::fs::read_to_string(&fresh_catalog).unwrap();
    assert!(
        !written.contains("grok-4.7"),
        "matching empty catalog must not become the schema card: {written}"
    );
    assert!(written.contains("\"model\": \"\""), "{written}");

    let _ = std::fs::remove_dir_all(&dir);
}
