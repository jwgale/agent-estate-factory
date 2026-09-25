//! `estate convey authority` prints the same hop coverage cites as doctor
//! and status, from `print_hop_coverage_cites`, after Agents and before
//! Authority. Mismatch is `FAIL`. Deny and deny-default are `note`. A match
//! is quiet. None of those cites fail the command. Print-only.

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
    let dir = std::env::temp_dir().join(format!("cell-convey-authority-hop-{nanos}"));
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

/// Text between the Agents section and the Authority header. Empty when the
/// command printed no hop coverage cite.
fn between_agents_and_authority<'a>(stdout: &'a str, agents: &str) -> &'a str {
    let at = stdout
        .find(agents)
        .unwrap_or_else(|| panic!("missing Agents section\n{stdout}"));
    let after = &stdout[at + agents.len()..];
    let end = after
        .find("\nAuthority\n---------\n")
        .unwrap_or_else(|| panic!("missing Authority section\n{after}"));
    &after[..end]
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

fn status(estate: &Path, state: &Path, root: &Path, scratch_root: &Path) -> (bool, String, String) {
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    let plans = scratch_root.join("status-plans");
    let packs = scratch_root.join("status-packs");
    let roots = scratch_root.join("status-roots");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::create_dir_all(&packs).unwrap();
    std::fs::create_dir_all(&roots).unwrap();
    let plans_s = plans.display().to_string();
    let packs_s = packs.display().to_string();
    let roots_s = roots.display().to_string();
    let policy = repo_root()
        .join("policy/cell-one.policy.v0.yaml")
        .display()
        .to_string();
    let root_s = root.display().to_string();
    run(&[
        "status",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--roots-base",
        &roots_s,
        "--plans-dir",
        &plans_s,
        "--packs-dir",
        &packs_s,
        "--policy",
        &policy,
        "--root",
        &root_s,
    ])
}

fn doctor(root: &Path, state: &Path) -> (bool, String, String) {
    let root_s = root.display().to_string();
    let state_s = state.display().to_string();
    run(&["doctor", "--root", &root_s, "--state-dir", &state_s])
}

fn assert_locked_cksum() {
    let sum = Command::new("cksum")
        .arg(repo_root().join("examples/estate.yaml"))
        .output()
        .unwrap();
    let sum_text = String::from_utf8_lossy(&sum.stdout);
    assert!(
        sum_text.starts_with("43770130 3391"),
        "examples/estate.yaml cksum changed: {sum_text}"
    );
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
    assert!(!state.join("placement-actual.json").exists());
    assert_eq!(std::fs::read(estate).unwrap(), estate_bytes);
}

#[test]
fn convey_authority_prints_the_same_deny_default_hop_cite_as_doctor_and_status() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(section.contains("would-deny"), "{section}");
    assert!(no_enforced_status_token(&section), "{section}");
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let root = repo_root();
    let (doctor_ok, doctor_out, doctor_err) = doctor(&root, &state);
    assert!(doctor_ok, "{doctor_out}\n{doctor_err}");
    assert!(doctor_err.is_empty(), "{doctor_err}");
    let doctor_cites = between_agents_and_authority(&doctor_out, &agents);
    assert!(
        doctor_cites.contains("  note  refuse:hop-coverage:")
            && doctor_cites.contains("(deny-default)")
            && !doctor_cites.contains("(mismatch)")
            && !doctor_cites.contains("  FAIL  "),
        "{doctor_cites}"
    );
    let (status_ok, status_out, status_err) = status(&estate_path, &state, &root, &dir);
    assert!(status_ok, "{status_out}\n{status_err}");
    assert!(status_err.is_empty(), "{status_err}");
    let status_cites = between_agents_and_authority(&status_out, &agents);
    assert_eq!(
        status_cites, doctor_cites,
        "status cite block\n{status_cites}"
    );
    let (ok, stdout, stderr) = convey(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let cites = between_agents_and_authority(&stdout, &agents);
    assert_eq!(cites, doctor_cites, "convey cite block\n{cites}");
    assert_eq!(stdout.trim_end(), format!("{agents}{cites}\n{section}"));
    assert!(stdout.contains(&section), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_no_writes(&state, &before, &estate_path, &estate_bytes);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn convey_authority_prints_the_same_mismatch_fail_cite_and_exits_zero() {
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
    let agents = estate_schema::describe_agents_section(&estate);
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        section.contains("research notes-append cell-one-box: would-deny --")
            && section.contains("(mismatch)"),
        "{section}"
    );
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let (doctor_ok, doctor_out, doctor_err) = doctor(&root, &state);
    assert!(!doctor_ok, "{doctor_out}\n{doctor_err}");
    assert!(doctor_err.contains("doctor failed"), "{doctor_err}");
    let doctor_cites = between_agents_and_authority(&doctor_out, &agents);
    assert!(
        doctor_cites.starts_with("\n  FAIL  refuse:hop-coverage:")
            && doctor_cites.contains("(mismatch)")
            && doctor_cites.contains(
                "capability 'notes-append' does not match hop coverage capability 'lane-tool'"
            )
            && !doctor_cites.contains("  note  refuse:hop-coverage:"),
        "{doctor_cites}"
    );
    let (status_ok, status_out, status_err) = status(&estate_path, &state, &root, &root);
    assert!(status_ok, "{status_out}\n{status_err}");
    assert!(status_err.is_empty(), "{status_err}");
    assert_eq!(
        between_agents_and_authority(&status_out, &agents),
        doctor_cites,
        "status cite block"
    );
    let (ok, stdout, stderr) = convey(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(
        stderr.is_empty(),
        "a hop cite does not fail convey\n{stderr}"
    );
    let cites = between_agents_and_authority(&stdout, &agents);
    assert_eq!(cites, doctor_cites, "convey cite block\n{cites}");
    assert!(stdout.contains(&section), "{stdout}");
    let agents_at = stdout.find(&agents).unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    assert!(agents_at < auth_at, "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_no_writes(&state, &before, &estate_path, &estate_bytes);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn convey_authority_stays_quiet_when_hop_coverage_matches() {
    let root = doctor_root(&estate_with_lane_tool("allow"));
    let estate_path = root.join("examples/estate.yaml");
    let state = root.join("state");
    std::fs::create_dir_all(&state).unwrap();
    conveyor_proxy::persist_mesh(&state, &granted_box("lane-tool")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        section.contains("research lane-tool cell-one-box: would-allow --"),
        "{section}"
    );
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let (doctor_ok, doctor_out, doctor_err) = doctor(&root, &state);
    assert!(doctor_ok, "{doctor_out}\n{doctor_err}");
    assert!(doctor_err.is_empty(), "{doctor_err}");
    assert_eq!(
        between_agents_and_authority(&doctor_out, &agents),
        "",
        "doctor match adds no hop cite\n{doctor_out}"
    );
    let (status_ok, status_out, status_err) = status(&estate_path, &state, &root, &root);
    assert!(status_ok, "{status_out}\n{status_err}");
    assert!(status_err.is_empty(), "{status_err}");
    assert_eq!(
        between_agents_and_authority(&status_out, &agents),
        "",
        "status match adds no hop cite\n{status_out}"
    );
    let (ok, stdout, stderr) = convey(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(
        between_agents_and_authority(&stdout, &agents),
        "",
        "a match adds no hop cite\n{stdout}"
    );
    assert_eq!(stdout.trim_end(), format!("{agents}\n{section}"));
    assert!(!stdout.contains("refuse:hop-coverage"), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_no_writes(&state, &before, &estate_path, &estate_bytes);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&root);
}
