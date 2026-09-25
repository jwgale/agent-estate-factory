//! `estate doctor` prints the same Authority section as plan, drift, apply,
//! and convey authority, after hop cites. Print-only.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("cell-doctor-authority-cli-{nanos}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args(args)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn snapshot(state: &Path) -> Vec<(String, Vec<u8>)> {
    let mut rows = Vec::new();
    if state.is_dir() {
        for entry in std::fs::read_dir(state).unwrap().flatten() {
            let path = entry.path();
            if path.is_file() {
                rows.push((
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    std::fs::read(&path).unwrap(),
                ));
            }
        }
    }
    rows.sort();
    rows
}

fn no_enforced_status_token(text: &str) -> bool {
    !text.split_whitespace().any(|word| {
        let token = word.trim_matches(|c: char| c == ':' || c == ',' || c == '.' || c == ';');
        token == "enforced"
    })
}

fn estate_with_lane_tool(effect: &str) -> String {
    let mut text = std::fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
    text = text.replace(
        "      - id: notes-append\n        description: Append a note inside the Research lane\n",
        "      - id: notes-append\n        description: Append a note inside the Research lane\n      - id: lane-tool\n",
    );
    text = text.replace(
        "intentions: []\n",
        &format!(
            "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: {effect}\n"
        ),
    );
    text
}

fn doctor_root(estate_text: &str) -> PathBuf {
    let real = repo_root();
    let dir = scratch();
    for entry in std::fs::read_dir(&real).unwrap().flatten() {
        let name = entry.file_name();
        let label = name.to_string_lossy();
        if label == "examples" || label == "target" || label == ".git" {
            continue;
        }
        std::os::unix::fs::symlink(entry.path(), dir.join(name)).unwrap();
    }
    std::fs::create_dir_all(dir.join("examples")).unwrap();
    for entry in std::fs::read_dir(real.join("examples")).unwrap().flatten() {
        if entry.file_name() == "estate.yaml" {
            continue;
        }
        std::os::unix::fs::symlink(entry.path(), dir.join("examples").join(entry.file_name()))
            .unwrap();
    }
    std::fs::write(dir.join("examples/estate.yaml"), estate_text).unwrap();
    dir
}

fn granted_box(capability: &str) -> conveyor_proxy::ConveyorMesh {
    conveyor_proxy::ConveyorMesh {
        schema: conveyor_proxy::MESH_SCHEMA.into(),
        hops: vec![],
        leases: vec![conveyor_proxy::HopLease {
            hop_id: "cell-one-box".into(),
            kind: "box".into(),
            capability: capability.into(),
            host_class: "any".into(),
            granted: true,
            spawned: true,
            durable: true,
            driver: "box".into(),
            note: None,
            ttl_secs: None,
            issued_at: None,
            expires_at: None,
            agents: vec!["research".into()],
        }],
    }
}

fn doctor(root: &Path, state: &Path) -> (bool, String, String) {
    let root_s = root.display().to_string();
    let state_s = state.display().to_string();
    run(&["doctor", "--root", &root_s, "--state-dir", &state_s])
}

#[test]
fn doctor_prints_would_deny_for_a_granted_box_lease_with_hop_coverage_deny() {
    let root = doctor_root(&estate_with_lane_tool("deny"));
    let estate_path = root.join("examples/estate.yaml");
    let state = root.join("state");
    std::fs::create_dir_all(&state).unwrap();
    conveyor_proxy::persist_mesh(&state, &granted_box("lane-tool")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let locked = std::fs::read(repo_root().join("examples/estate.yaml")).unwrap();
    let (ok, stdout, stderr) = doctor(&root, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stdout.contains(&section), "{stdout}");
    assert!(stdout.contains("factory ready"), "{stdout}");
    let hop_at = stdout.find("hop:\n").unwrap();
    let note_at = stdout.find("  note  refuse:hop-coverage:").unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    let health_at = stdout.find("\nHealth\n").unwrap();
    assert!(
        hop_at < note_at && note_at < auth_at && auth_at < health_at,
        "{stdout}"
    );
    let denied = section
        .lines()
        .find(|line| line.contains("research lane-tool cell-one-box:"))
        .unwrap_or_else(|| panic!("missing deny row\n{section}"));
    assert!(
        denied.contains(": would-deny --")
            && denied.contains("refuse:hop-coverage")
            && denied.contains("(deny).")
            && !denied.contains("deny-default")
            && !denied.contains("(mismatch)")
            && denied.contains("Not mediated"),
        "{denied}"
    );
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&section), "{section}");
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("apply-audit.jsonl").exists());
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);
    assert_eq!(
        std::fs::read(repo_root().join("examples/estate.yaml")).unwrap(),
        locked
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn hop_mismatch_prints_authority_then_fails_and_writes_nothing() {
    let mut text = estate_with_lane_tool("allow");
    text = text.replace(
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n",
    );
    let root = doctor_root(&text);
    let estate_path = root.join("examples/estate.yaml");
    let state = root.join("state");
    std::fs::create_dir_all(&state).unwrap();
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        section.contains("research notes-append cell-one-box: would-deny --"),
        "{section}"
    );
    assert!(
        section.contains("refuse:hop-coverage") && section.contains("(mismatch)"),
        "{section}"
    );
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let (ok, stdout, stderr) = doctor(&root, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.contains(&section), "{stdout}");
    assert!(!stdout.contains("factory ready"), "{stdout}");
    let hop_at = stdout.find("hop:\n").unwrap();
    let fail_at = stdout.find("  FAIL  refuse:hop-coverage:").unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    let health_at = stdout.find("\nHealth\n").unwrap();
    assert!(
        hop_at < fail_at && fail_at < auth_at && auth_at < health_at,
        "{stdout}"
    );
    assert!(stderr.contains("doctor failed (1 check(s))"), "{stderr}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("apply-audit.jsonl").exists());
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);
    let _ = std::fs::remove_dir_all(&root);
}
