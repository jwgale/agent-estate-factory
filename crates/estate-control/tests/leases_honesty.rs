//! `estate leases` prints the same Agents, hop-cite, and Authority stack as
//! status, doctor, reconcile, and audit export, before the placement lease
//! list. Mismatch is `FAIL`. Deny and deny-default are `note`. A match is
//! quiet. None of those cites fail the command. A placement-actual SKU
//! still refuses before the placement JSON.

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
    let dir = std::env::temp_dir().join(format!("cell-leases-honesty-{nanos}"));
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
        && !text.contains("Authority\n---------")
        && !text.contains("cell-one.placement-actual.v0")
        && !text.contains("refuse:hop-coverage")
}

/// Text between the Agents section and the Authority header.
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

fn apply(estate: &Path, state: &Path, scratch_root: &Path) {
    let plans = scratch_root.join("plans");
    let roots = scratch_root.join("roots");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::create_dir_all(&roots).unwrap();
    std::fs::create_dir_all(state).unwrap();
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let roots_s = roots.display().to_string();
    let (ok, stdout, stderr) = run(&[
        "apply",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--roots-base",
        &roots_s,
        "--plans-dir",
        &plans_s,
    ]);
    assert!(ok, "{stdout}\n{stderr}");
}

fn leases(estate: &Path, state: &Path) -> (bool, String, String) {
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    run(&["leases", "--estate", &estate_s, "--state-dir", &state_s])
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

fn assert_stack_before_leases(stdout: &str, agents: &str, section: &str) {
    assert_eq!(stdout.matches("Agents\n------\n").count(), 1, "{stdout}");
    assert_eq!(
        stdout.matches("Authority\n---------\n").count(),
        1,
        "{stdout}"
    );
    let agents_at = stdout.find(agents).unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    let json_at = stdout
        .find("cell-one.placement-actual.v0")
        .unwrap_or_else(|| panic!("missing placement JSON\n{stdout}"));
    assert!(agents_at < auth_at && auth_at < json_at, "{stdout}");
    assert!(stdout.contains(section), "{stdout}");
    assert!(no_enforced_status_token(stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
}

#[test]
fn leases_prints_agents_cites_and_authority_before_the_placement_list() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(section.contains("would-deny"), "{section}");
    assert!(section.contains("not-enforced"), "{section}");
    assert!(no_enforced_status_token(&section), "{section}");
    let before = snapshot(&state);
    let (convey_ok, convey_out, convey_err) = convey(&estate_path, &state);
    assert!(convey_ok, "{convey_out}\n{convey_err}");
    let (ok, stdout, stderr) = leases(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.starts_with(&convey_out), "{stdout}");
    let cites = between_agents_and_authority(&stdout, &agents);
    assert!(
        cites.contains("  note  refuse:hop-coverage:")
            && cites.contains("(deny-default)")
            && !cites.contains("(mismatch)")
            && !cites.contains("  FAIL  "),
        "{cites}"
    );
    assert_eq!(between_agents_and_authority(&convey_out, &agents), cites);
    assert_stack_before_leases(&stdout, &agents, &section);
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("reconcile.json").exists());
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hop_mismatch_cite_does_not_fail_leases() {
    let mut text = estate_with_lane_tool("allow");
    text = text.replace(
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n",
    );
    let root = doctor_root(&text);
    let estate_path = root.join("examples/estate.yaml");
    let state = root.join("state");
    apply(&estate_path, &state, &root);
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let before = snapshot(&state);
    let (ok, stdout, stderr) = leases(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let cites = between_agents_and_authority(&stdout, &agents);
    assert!(
        cites.contains("  FAIL  refuse:hop-coverage:") && cites.contains("(mismatch)"),
        "{cites}"
    );
    let agents_at = stdout.find(&agents).unwrap();
    let cite_at = stdout.find("  FAIL  refuse:hop-coverage:").unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    let json_at = stdout.find("cell-one.placement-actual.v0").unwrap();
    assert!(
        agents_at < cite_at && cite_at < auth_at && auth_at < json_at,
        "{stdout}"
    );
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_eq!(snapshot(&state), before);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn missing_mesh_is_not_enforced_and_invents_no_cites() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    assert!(!state.join("conveyor-mesh.json").exists());
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        section.contains("not-enforced reasons: missing-mesh=5 cloud=1"),
        "{section}"
    );
    assert!(
        section.contains("conveyor-mesh.json is absent"),
        "{section}"
    );
    let before = snapshot(&state);
    let (ok, stdout, stderr) = leases(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(between_agents_and_authority(&stdout, &agents), "");
    assert!(!stdout.contains("refuse:hop-coverage"), "{stdout}");
    assert_stack_before_leases(&stdout, &agents, &section);
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("conveyor-leases.json").exists());
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agent_unplaced_refuses_before_the_lease_list() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    let mut ahead = granted_box("notes-append");
    ahead.hops.push(conveyor_proxy::HopDecl {
        id: "cell-one-box".into(),
        kind: "box".into(),
        capability: "notes-append".into(),
        host_class: "any".into(),
        wired: true,
        note: None,
        ttl_secs: None,
        agents: vec!["research".into(), "outsider".into()],
    });
    ahead.leases[0].agents = vec!["research".into(), "outsider".into()];
    conveyor_proxy::persist_mesh(&state, &ahead).unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = leases(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("refuse:agent-unplaced"), "{stderr}");
    assert!(stderr.contains("outsider"), "{stderr}");
    assert!(!stderr.contains("cell-one.placement-actual"), "{stderr}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mesh_parse_and_mesh_host_class_refuse_before_the_lease_list() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    std::fs::write(
        state.join("placement-actual.json"),
        r#"{"schema":"cell-one.placement-actual.v0","desired_hash":"abc","leases":[]}"#,
    )
    .unwrap();
    let mesh = state.join("conveyor-mesh.json");
    std::fs::write(&mesh, "not-json").unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = leases(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(stderr.contains("conveyor-mesh.json"), "{stderr}");
    assert!(!stdout.contains("cell-one.placement-actual"), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert_eq!(std::fs::read(&mesh).unwrap(), b"not-json");

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[],"leases":[{"hop_id":"box","kind":"box","capability":"lane-tool","host_class":"rtx-5090","granted":true,"spawned":false,"durable":true,"driver":"box"}]}"#,
    )
    .unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = leases(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(stderr.contains("rtx-5090"), "{stderr}");
    assert_eq!(snapshot(&state), before);

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[],"leases":[]}"#,
    )
    .unwrap();
    std::fs::write(state.join("placement-actual.json"), "{").unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = leases(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(stderr.contains("placement-actual.json"), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn placement_sku_prints_agents_and_cites_and_omits_authority() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let (ok, first, stderr) = leases(&estate_path, &state);
    assert!(ok, "{first}\n{stderr}");
    let between = between_agents_and_authority(&first, &agents);
    assert!(
        between.contains("  note  refuse:hop-coverage:") && between.contains("(deny-default)"),
        "{between}"
    );

    let leases_path = state.join("placement-actual.json");
    let mut actual: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&leases_path).unwrap()).unwrap();
    let rows = actual["leases"].as_array_mut().expect("leases");
    let box_lease = rows
        .iter_mut()
        .find(|lease| lease["placement_id"] == "cell-one-box")
        .expect("cell-one-box");
    box_lease["host_class"] = serde_json::Value::String("rtx-5090".into());
    std::fs::write(&leases_path, serde_json::to_string_pretty(&actual).unwrap()).unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = leases(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.starts_with(&agents), "{stdout}");
    assert_eq!(&stdout[agents.len()..], format!("{between}\n"));
    assert!(!stdout.contains("Authority\n---------"), "{stdout}");
    assert!(!stdout.contains("not-enforced reasons:"), "{stdout}");
    assert!(!stdout.contains("would-allow="), "{stdout}");
    assert!(!stdout.contains("cell-one.placement-actual"), "{stdout}");
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(stderr.contains("rtx-5090"), "{stderr}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_names_the_stack_and_locks_hold() {
    let (ok, stdout, stderr) = run(&["leases", "--help"]);
    assert!(ok, "{stdout}\n{stderr}");
    let agents = stdout
        .find("describe_agents_section")
        .unwrap_or_else(|| panic!("help omits describe_agents_section\n{stdout}"));
    let cites = stdout
        .find("hop_coverage_cites")
        .unwrap_or_else(|| panic!("help omits hop_coverage_cites\n{stdout}"));
    let authority = stdout
        .find("describe_authority_section")
        .unwrap_or_else(|| panic!("help omits describe_authority_section\n{stdout}"));
    assert!(agents < cites && cites < authority, "{stdout}");
    assert!(stdout.contains("do not fail"), "{stdout}");
    assert!(stdout.contains("empty cite list"), "{stdout}");
    assert!(stdout.contains("refuse:agent-unplaced"), "{stdout}");
    assert!(stdout.contains("Does not spawn"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");

    let language =
        std::fs::read_to_string(repo_root().join("docs/UBIQUITOUS_LANGUAGE.md")).unwrap();
    assert!(language.contains("`estate leases` prints"), "{language}");
    assert!(language.contains("missing-mesh"), "{language}");
    assert!(!language.contains("READY_FOR_LIVE_TEST: yes"), "{language}");

    let changelog = std::fs::read_to_string(repo_root().join("CHANGELOG.md")).unwrap();
    assert!(
        changelog.contains("estate leases prints the honesty stack"),
        "{changelog}"
    );
    assert!(
        changelog.contains("render_hop_coverage_cites"),
        "{changelog}"
    );
    assert!(
        !changelog.contains("READY_FOR_LIVE_TEST: yes"),
        "{changelog}"
    );
    assert_locked_cksum();
}
