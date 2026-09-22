//! Status refuses a present model-actual.json that does not parse.
//! A missing file is not a failure, and is not invented as zero bindings.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-status-model-{}-{}",
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
fn status_refuses_an_unreadable_model_actual_before_the_page() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("model");
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
    assert!(
        !text.contains("model-actual.json bindings="),
        "missing file must not invent a binding count: {text}"
    );
    assert!(
        !text.contains("run apply"),
        "missing file must not become the drift note: {text}"
    );

    let path = state.join("model-actual.json");
    std::fs::write(&path, "not-json\n").unwrap();
    let (ok, text) = run(bin, &status);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:model-actual"), "{text}");
    assert!(text.contains("model-actual.json"), "{text}");
    assert!(!text.contains("Cell One status"), "{text}");
    assert!(!text.contains("run apply"), "{text}");
    assert!(!text.contains("model-actual.json bindings="), "{text}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "not-json\n");

    std::fs::write(
        &path,
        "{\"desired_hash\":\"abc\",\"bindings\":[{\"id\":\"b\",\"class\":\"local\",\"driver\":\"ollama\",\"wired\":true}]}\n",
    )
    .unwrap();
    let (ok, text) = run(bin, &status);
    assert!(ok, "{text}");
    assert!(text.contains("Cell One status"), "{text}");
    assert!(!text.contains("refuse:model-actual"), "{text}");
    assert!(
        !text.contains("model-actual.json bindings="),
        "a parsed file is not a new status line: {text}"
    );
}
