//! `conveyor-proxy authority` prints the same Agents section as
//! `estate convey authority`, from `describe_agents_section`, immediately
//! before Authority. Print-only. A would-deny row does not fail the command.

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
    let dir = std::env::temp_dir().join(format!("cell-proxy-authority-agents-{nanos}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_conveyor-proxy"))
        .args(args)
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

fn no_sections(text: &str) -> bool {
    !text.contains("Agents\n------")
        && !text.contains("Authority\n---------\n")
        && !text.contains("would-allow")
        && !text.contains("would-deny")
        && !text.contains("not-enforced")
}

fn authority(estate: &Path, state: &Path) -> (bool, String, String) {
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    run(&["authority", "--estate", &estate_s, "--state-dir", &state_s])
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

#[test]
fn proxy_authority_prints_agents_then_the_shared_authority_section() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let section = estate_schema::describe_agents_section(&estate);
    assert!(section.starts_with("Agents\n------\n"), "{section}");
    assert!(section.contains("- id: horizon"), "{section}");
    assert!(section.contains("deny-default"), "{section}");
    assert!(section.contains("Plan does not spawn."), "{section}");
    assert!(no_enforced_status_token(&section), "{section}");
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let authority_text = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        authority_text.starts_with("Authority\n---------\n"),
        "{authority_text}"
    );
    assert!(
        authority_text.contains("conveyor-mesh.json is absent"),
        "{authority_text}"
    );
    assert!(
        no_enforced_status_token(&authority_text),
        "{authority_text}"
    );
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let (ok, stdout, stderr) = authority(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(stdout.trim_end(), format!("{section}\n{authority_text}"));
    let agents_at = stdout.find(&section).unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    assert!(agents_at < auth_at, "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("conveyor-leases.json").exists());
    assert!(!state.join("apply-audit.jsonl").exists());
    assert!(!state.join("sessions.jsonl").exists());
    assert!(!state.join("actual-state.json").exists());
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);
    let sum = Command::new("cksum").arg(&estate_path).output().unwrap();
    let sum_text = String::from_utf8_lossy(&sum.stdout);
    assert!(
        sum_text.starts_with("43770130 3391"),
        "examples/estate.yaml cksum changed: {sum_text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn proxy_would_deny_does_not_fail_or_write() {
    let dir = scratch();
    let estate_path = dir.join("deny.yaml");
    std::fs::write(&estate_path, estate_with_lane_tool("deny")).unwrap();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    conveyor_proxy::persist_mesh(&state, &granted_box("lane-tool")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let section = estate_schema::describe_agents_section(&estate);
    assert!(
        section.lines().any(|line| {
            line.contains("research tool lane-tool: deny") && !line.contains("deny-default")
        }),
        "{section}"
    );
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let authority_text = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        authority_text.contains("research lane-tool cell-one-box: would-deny --"),
        "{authority_text}"
    );
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let (ok, stdout, stderr) = authority(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(stdout.trim_end(), format!("{section}\n{authority_text}"));
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("apply-audit.jsonl").exists());
    assert!(!state.join("sessions.jsonl").exists());
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn proxy_unreadable_mesh_and_missing_estate_invent_no_rows() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let mesh = state.join("conveyor-mesh.json");
    std::fs::write(&mesh, "not-json").unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let before = snapshot(&state);
    let (ok, stdout, stderr) = authority(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(
        no_sections(&stdout) && no_sections(&stderr),
        "{stdout}\n{stderr}"
    );
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(stderr.contains("conveyor-mesh.json"), "{stderr}");
    assert_eq!(std::fs::read(&mesh).unwrap(), b"not-json");
    assert_eq!(snapshot(&state), before);

    let missing = dir.join("missing.yaml");
    let (ok, stdout, stderr) = authority(&missing, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(
        no_sections(&stdout) && no_sections(&stderr),
        "{stdout}\n{stderr}"
    );
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("sessions.jsonl").exists());
    assert!(!state.join("apply-audit.jsonl").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn proxy_placement_sku_omits_authority_and_mesh_sku_refuses_before_agents() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let section = estate_schema::describe_agents_section(&estate);
    let mesh = state.join("conveyor-mesh.json");
    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[],"leases":[{"hop_id":"box","kind":"box","capability":"lane-tool","host_class":"rtx-5090","granted":true,"spawned":false,"durable":true,"driver":"box","agents":["research"]}]}"#,
    )
    .unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = authority(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(
        no_sections(&stdout) && no_sections(&stderr),
        "{stdout}\n{stderr}"
    );
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(stderr.contains("rtx-5090"), "{stderr}");
    assert_eq!(snapshot(&state), before);

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[],"leases":[{"hop_id":"cell-one-box","kind":"box","capability":"notes-append","host_class":"any","granted":true,"spawned":true,"durable":true,"driver":"box","agents":["research"]}]}"#,
    )
    .unwrap();
    std::fs::write(
        state.join("placement-actual.json"),
        r#"{"leases":[{"placement_id":"cell-one-box","kind":"box","host_class":"rtx-5090","agents":["research"]}]}"#,
    )
    .unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = authority(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(stdout.trim_end(), section);
    assert!(!stdout.contains("Authority\n---------"), "{stdout}");
    assert!(!stdout.contains("would-allow="), "{stdout}");
    assert!(!stdout.contains("refuse:hop-coverage"), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("apply-audit.jsonl").exists());
    assert!(!state.join("sessions.jsonl").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn proxy_authority_help_names_the_shared_agents_section_before_authority() {
    let (ok, stdout, stderr) = run(&["authority", "--help"]);
    assert!(ok, "{stdout}\n{stderr}");
    let agents = stdout
        .find("describe_agents_section")
        .unwrap_or_else(|| panic!("help omits describe_agents_section\n{stdout}"));
    let authority = stdout
        .find("describe_authority_section")
        .unwrap_or_else(|| panic!("help omits describe_authority_section\n{stdout}"));
    assert!(agents < authority, "{stdout}");
    assert!(stdout.contains("Does not spawn"), "{stdout}");
    assert!(stdout.contains("Does not write"), "{stdout}");
    assert!(stdout.contains("do not fail"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
}
