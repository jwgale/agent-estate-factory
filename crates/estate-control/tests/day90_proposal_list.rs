//! Status refuses an unreadable enrich proposal before the status page.
//! A filename is not an open proposal.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-proposal-{}-{}",
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
fn status_refuses_an_unreadable_proposal_before_the_page() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("proposal-unreadable");
    let state = dir.join("state");
    let plans = dir.join("plans");
    let packs = dir.join("packs");
    std::fs::create_dir_all(&state).unwrap();
    let estate = root.join("examples/estate.yaml");

    let (ok, text) = run(
        bin,
        &[
            "apply",
            "--estate",
            &estate.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ],
    );
    assert!(ok, "{text}");

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
    assert!(text.contains("open_proposals: 0 (-)"), "{text}");

    let proposed = packs.join("proposed");
    std::fs::create_dir_all(&proposed).unwrap();
    let garbage = proposed.join("ghost.proposal.json");
    std::fs::write(&garbage, "not-json").unwrap();

    let (ok, text) = run(bin, &status);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:proposal-unreadable"), "{text}");
    assert!(
        text.contains("a filename is not an open proposal"),
        "{text}"
    );
    assert!(
        !text.contains("Cell One status"),
        "status must refuse before the page: {text}"
    );
    assert!(
        !text.contains("open_proposals:"),
        "status must not invent a proposal count: {text}"
    );
    assert_eq!(std::fs::read_to_string(&garbage).unwrap(), "not-json");

    let mismatched = r#"{
  "id": "other",
  "curator": "jason",
  "policy": "manual",
  "auto_apply": false,
  "source_pack": "ghost.pack.json",
  "estate_name": "cell-one",
  "estate_hash": "abc",
  "current_estate_packs": [],
  "proposed_estate_packs": [],
  "diff": {
    "pack_id": "ghost",
    "estate_bound": false,
    "would_add_to_estate": false,
    "kinds": [],
    "agents": [],
    "paths": [],
    "path_counts": {},
    "source_drivers": [],
    "note": "n"
  },
  "created_at": "2026-01-01T00:00:00Z",
  "note": "n"
}"#;
    std::fs::write(&garbage, mismatched).unwrap();
    let (ok, text) = run(bin, &status);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:proposal-unreadable"), "{text}");
    assert!(text.contains("does not match ghost"), "{text}");
    assert!(!text.contains("Cell One status"), "{text}");

    let valid = mismatched.replace("\"id\": \"other\"", "\"id\": \"ghost\"");
    std::fs::write(&garbage, &valid).unwrap();
    let (ok, text) = run(bin, &status);
    assert!(ok, "{text}");
    assert!(text.contains("open_proposals: 1 (ghost)"), "{text}");
    assert!(!text.contains("refuse:proposal-unreadable"), "{text}");

    let _ = std::fs::remove_dir_all(&dir);
}
