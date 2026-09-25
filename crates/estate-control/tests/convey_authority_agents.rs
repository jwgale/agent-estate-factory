//! `estate convey authority` prints the same Agents section as plan, drift,
//! apply, status, and doctor, from `describe_agents_section`, immediately
//! before Authority. Print-only. Deny and deny-default notes, and a
//! would-deny row, do not fail the command.

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
    let dir = std::env::temp_dir().join(format!("cell-convey-authority-agents-{nanos}"));
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

fn no_sections(text: &str) -> bool {
    !text.contains("Agents\n------")
        && !text.contains("Authority\n---------\n")
        && !text.contains("would-allow")
        && !text.contains("would-deny")
        && !text.contains("not-enforced")
}

fn assert_locked_cksum() {
    let locked = repo_root().join("examples/estate.yaml");
    let sum = Command::new("cksum").arg(&locked).output().unwrap();
    let sum_text = String::from_utf8_lossy(&sum.stdout);
    assert!(
        sum_text.starts_with("43770130 3391"),
        "examples/estate.yaml cksum changed: {sum_text}"
    );
}

fn convey(estate: &Path, state: &Path) -> (bool, String, String) {
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    run(&[
        "convey",
        "authority",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ])
}

fn assert_no_writes(
    state: &Path,
    before: &[(String, Vec<u8>)],
    estate: &Path,
    estate_bytes: &[u8],
) {
    assert_eq!(snapshot(state), before);
    assert!(!state.join("apply-audit.jsonl").exists());
    assert!(!state.join("sessions.jsonl").exists());
    assert!(!state.join("actual-state.json").exists());
    assert!(!state.join("placement-actual.json").exists());
    assert_eq!(std::fs::read(estate).unwrap(), estate_bytes);
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
fn convey_authority_prints_the_same_agents_section_as_plan_and_doctor() {
    let dir = scratch();
    let state = dir.join("state");
    let plans = dir.join("plans");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::create_dir_all(&plans).unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let section = estate_schema::describe_agents_section(&estate);
    assert!(section.starts_with("Agents\n------\n"), "{section}");
    assert!(section.contains("- id: horizon"), "{section}");
    assert!(section.contains("  lane: horizon"), "{section}");
    assert!(section.contains("  desktop: horizon-desktop"), "{section}");
    assert!(
        section.contains("  placement: box cell-one-box"),
        "{section}"
    );
    assert!(section.contains("- id: research"), "{section}");
    assert!(section.contains("- id: sanctum"), "{section}");
    assert!(
        section.contains("Cloud-agent stays declared, not spawned."),
        "{section}"
    );
    assert!(section.contains("Plan does not spawn."), "{section}");
    assert!(section.contains("deny-default"), "{section}");
    assert!(
        section.contains("horizon agent research: deny-default (peer)"),
        "{section}"
    );
    assert!(no_enforced_status_token(&section), "{section}");
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let authority = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        authority.starts_with("Authority\n---------\n"),
        "{authority}"
    );
    assert!(
        authority.contains("not-enforced reasons: missing-mesh=5 cloud=1"),
        "{authority}"
    );
    assert!(
        authority.contains("conveyor-mesh.json is absent"),
        "{authority}"
    );
    assert!(
        authority.contains("does not show that a worker called the conveyor"),
        "{authority}"
    );
    assert!(no_enforced_status_token(&authority), "{authority}");
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let (ok, stdout, stderr) = convey(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(
        stdout.trim_end(),
        format!("{section}\n{authority}"),
        "Agents then Authority is the shared printers"
    );
    assert!(stdout.starts_with(&section), "{stdout}");
    let agents_at = stdout.find(&section).unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    assert!(agents_at < auth_at, "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_no_writes(&state, &before, &estate_path, &estate_bytes);
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("conveyor-leases.json").exists());
    assert!(!state.join("conveyor-hops.json").exists());

    let estate_s = estate_path.display().to_string();
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let (plan_ok, plan_out, plan_err) = run(&[
        "plan",
        "--estate",
        &estate_s,
        "--plans-dir",
        &plans_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(plan_ok, "{plan_out}\n{plan_err}");
    let plan_at = plan_out
        .find(&section)
        .unwrap_or_else(|| panic!("plan omits describe_agents_section\n{plan_out}"));
    assert_eq!(&plan_out[plan_at..plan_at + section.len()], section);
    assert!(plan_out.contains(&authority), "{plan_out}");
    let plan_auth = plan_out.find(&authority).unwrap();
    assert!(plan_at < plan_auth, "{plan_out}");

    let root_s = repo_root().display().to_string();
    let (doctor_ok, doctor_out, doctor_err) =
        run(&["doctor", "--root", &root_s, "--state-dir", &state_s]);
    assert!(doctor_ok, "{doctor_out}\n{doctor_err}");
    let doctor_at = doctor_out
        .find(&section)
        .unwrap_or_else(|| panic!("doctor omits describe_agents_section\n{doctor_out}"));
    assert_eq!(&doctor_out[doctor_at..doctor_at + section.len()], section);
    assert!(doctor_out.contains(&authority), "{doctor_out}");
    let doctor_auth = doctor_out.find("Authority\n---------\n").unwrap();
    assert!(doctor_at < doctor_auth, "{doctor_out}");
    assert!(no_enforced_status_token(&doctor_out), "{doctor_out}");
    assert!(no_enforced_status_token(&plan_out), "{plan_out}");

    assert_no_writes(&state, &before, &estate_path, &estate_bytes);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn would_deny_stays_a_note_and_convey_authority_writes_nothing() {
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
    assert!(section.contains("deny-default"), "{section}");
    assert!(no_enforced_status_token(&section), "{section}");
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let authority = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        authority.contains("research lane-tool cell-one-box: would-deny --"),
        "{authority}"
    );
    assert!(authority.contains("refuse:hop-coverage"), "{authority}");
    assert!(no_enforced_status_token(&authority), "{authority}");
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let locked = std::fs::read(repo_root().join("examples/estate.yaml")).unwrap();
    let (ok, stdout, stderr) = convey(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(stdout.trim_end(), format!("{section}\n{authority}"));
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_no_writes(&state, &before, &estate_path, &estate_bytes);
    assert!(state.join("conveyor-mesh.json").is_file());
    assert_eq!(
        std::fs::read(repo_root().join("examples/estate.yaml")).unwrap(),
        locked
    );
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unreadable_mesh_refuses_before_agents_or_authority_and_writes_nothing() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let mesh = state.join("conveyor-mesh.json");
    std::fs::write(&mesh, "not-json").unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let (ok, stdout, stderr) = convey(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(stderr.contains("conveyor-mesh.json"), "{stderr}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_eq!(std::fs::read(&mesh).unwrap(), b"not-json");
    assert!(!state.join("apply-audit.jsonl").exists());
    assert!(!state.join("sessions.jsonl").exists());
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[],"leases":[{"hop_id":"box","kind":"box","capability":"lane-tool","host_class":"rtx-5090","granted":true,"spawned":false,"durable":true,"driver":"box"}]}"#,
    )
    .unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(no_sections(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_or_unreadable_estate_refuses_before_agents_or_authority() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let missing = dir.join("missing.yaml");
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey(&missing, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(
        stderr.contains("missing.yaml") || stderr.contains("io error"),
        "{stderr}"
    );
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("sessions.jsonl").exists());
    assert!(!state.join("apply-audit.jsonl").exists());

    let bad = dir.join("bad.yaml");
    std::fs::write(&bad, "not: [estate\n").unwrap();
    let bad_bytes = std::fs::read(&bad).unwrap();
    let (ok, stdout, stderr) = convey(&bad, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("bad.yaml"), "{stderr}");
    assert_eq!(std::fs::read(&bad).unwrap(), bad_bytes);
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn convey_authority_help_names_the_shared_agents_section_before_authority() {
    let (ok, stdout, stderr) = run(&["convey", "authority", "--help"]);
    assert!(ok, "{stdout}\n{stderr}");
    let agents = stdout
        .find("describe_agents_section")
        .unwrap_or_else(|| panic!("help omits describe_agents_section\n{stdout}"));
    let authority = stdout
        .find("describe_authority_section")
        .unwrap_or_else(|| panic!("help omits describe_authority_section\n{stdout}"));
    assert!(agents < authority, "{stdout}");
    assert!(stdout.contains("do not fail"), "{stdout}");
    assert!(stdout.contains("Does not spawn"), "{stdout}");
    assert!(stdout.contains("Does not write"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");

    let (help_ok, help_out, help_err) = run(&["help", "status"]);
    assert!(help_ok, "{help_out}\n{help_err}");
    assert!(
        help_out.contains("estate convey authority")
            && help_out.contains("immediately before that Authority section"),
        "{help_out}"
    );
    assert!(help_out.contains("do not fail the command"), "{help_out}");
    assert!(help_out.contains("does not spawn"), "{help_out}");
}
