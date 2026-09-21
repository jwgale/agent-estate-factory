//! One operator fixture: estate apply records `local_slm`, then specialist
//! complete against the same mock. No live box. Not a new verb.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn estate_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn fixture(rel: &str) -> String {
    repo_root().join(rel).display().to_string()
}

fn text(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn apply_records_local_slm_then_specialist_complete_mock() {
    let root = repo_root().join(format!(
        "target/test-apply-specialist-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let estate = fixture("examples/estate.yaml");
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let roots_s = root.display().to_string();

    let apply = estate_bin()
        .args([
            "apply",
            "--estate",
            &estate,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots_s,
            "--plans-dir",
            &plans_s,
        ])
        .output()
        .unwrap();
    assert!(apply.status.success(), "{}", text(&apply));

    let actual_path = state.join("model-actual.json");
    assert!(actual_path.is_file(), "apply must write model-actual.json");
    let actual = std::fs::read_to_string(&actual_path).unwrap();
    assert!(actual.contains("local_slm"), "{actual}");
    assert!(actual.contains("\"driver\": \"ollama\""), "{actual}");
    assert!(actual.contains("\"wired\": true"), "{actual}");
    assert!(state.join("placement-actual.json").is_file());
    assert!(state.join("catalog.json").is_file());

    let srv = model_estate::MockLocalServer::spawn().unwrap();
    let spec = estate_bin()
        .args([
            "specialist",
            "--driver",
            "ollama",
            "--endpoint",
            &srv.endpoint(),
            "--prompt",
            "hello from apply fixture",
        ])
        .env_remove("CELL_LOCAL_ENDPOINT")
        .output()
        .unwrap();
    assert!(spec.status.success(), "{}", text(&spec));
    let stdout = String::from_utf8_lossy(&spec.stdout);
    assert!(stdout.contains("\"job\": \"complete\""), "{stdout}");
    assert!(
        stdout.contains("\"completion\": \"mock:hello from apply fixture\""),
        "{stdout}"
    );
    let (path, body) = srv.last_post().expect("factory specialist POST");
    assert_eq!(path, "/v0/specialist");
    assert!(body.contains("hello from apply fixture"), "{body}");

    let after = std::fs::read_to_string(&actual_path).unwrap();
    assert_eq!(actual, after, "specialist must not rewrite model-actual");
    let _ = std::fs::remove_dir_all(&root);
}
