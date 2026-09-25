//! `estate reconcile` prints the same hop coverage cites as doctor, status,
//! and convey authority, from `print_hop_coverage_cites`, after Agents and
//! before Authority, and before the placement report. Mismatch is `FAIL`.
//! Deny and deny-default are `note`. A match is quiet. None of those cites
//! fail the command. Placement drift still fails closed after the sections.

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
    let dir = std::env::temp_dir().join(format!("cell-reconcile-honesty-{nanos}"));
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
        && !text.contains("Placement reconcile")
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

fn reconcile(estate: &Path, state: &Path, suggest: bool) -> (bool, String, String) {
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    if suggest {
        run(&[
            "reconcile",
            "--suggest",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
        ])
    } else {
        run(&["reconcile", "--estate", &estate_s, "--state-dir", &state_s])
    }
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

fn assert_stack_once_before_report(stdout: &str, agents: &str, section: &str) {
    assert_eq!(stdout.matches("Agents\n------\n").count(), 1, "{stdout}");
    assert_eq!(
        stdout.matches("Authority\n---------\n").count(),
        1,
        "{stdout}"
    );
    let agents_at = stdout.find(agents).unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    let report_at = stdout
        .find("Placement reconcile (desired vs actual)")
        .unwrap_or_else(|| panic!("missing placement report\n{stdout}"));
    assert!(agents_at < auth_at && auth_at < report_at, "{stdout}");
    assert!(stdout.contains(section), "{stdout}");
    assert!(no_enforced_status_token(stdout), "{stdout}");
}

fn kept_files(state: &Path) -> Vec<(String, Vec<u8>)> {
    let keep = [
        "placement-actual.json",
        "conveyor-mesh.json",
        "conveyor-hops.json",
        "conveyor-leases.json",
        "apply-audit.jsonl",
        "sessions.jsonl",
    ];
    let mut rows = Vec::new();
    for name in keep {
        let path = state.join(name);
        if path.is_file() {
            rows.push((name.to_string(), std::fs::read(path).unwrap()));
        }
    }
    rows
}

#[test]
fn reconcile_prints_the_same_deny_default_hop_cite_as_doctor_status_and_convey() {
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
    let kept = kept_files(&state);
    let root = repo_root();
    let (doctor_ok, doctor_out, doctor_err) = doctor(&root, &state);
    assert!(doctor_ok, "{doctor_out}\n{doctor_err}");
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
    assert_eq!(
        between_agents_and_authority(&status_out, &agents),
        doctor_cites
    );
    let (convey_ok, convey_out, convey_err) = convey(&estate_path, &state);
    assert!(convey_ok, "{convey_out}\n{convey_err}");
    assert_eq!(
        between_agents_and_authority(&convey_out, &agents),
        doctor_cites
    );
    let (ok, stdout, stderr) = reconcile(&estate_path, &state, false);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(
        between_agents_and_authority(&stdout, &agents),
        doctor_cites,
        "reconcile cite block\n{stdout}"
    );
    assert_stack_once_before_report(&stdout, &agents, &section);
    assert!(state.join("reconcile.json").is_file());
    assert!(state.join("reconcile.md").is_file());
    assert!(!state.join("reconcile-suggest.md").exists());
    assert_eq!(kept_files(&state), kept);
    let (suggest_ok, suggest_out, suggest_err) = reconcile(&estate_path, &state, true);
    assert!(suggest_ok, "{suggest_out}\n{suggest_err}");
    assert!(suggest_err.is_empty(), "{suggest_err}");
    assert_eq!(
        between_agents_and_authority(&suggest_out, &agents),
        doctor_cites
    );
    assert_stack_once_before_report(&suggest_out, &agents, &section);
    assert!(suggest_out.contains("auto_apply: false"), "{suggest_out}");
    assert!(suggest_out.contains("No patch"), "{suggest_out}");
    let report_at = suggest_out
        .find("Placement reconcile (desired vs actual)")
        .unwrap();
    let suggest_at = suggest_out
        .find("Reconcile suggest (patch file only)")
        .unwrap();
    assert!(report_at < suggest_at, "{suggest_out}");
    assert!(state.join("reconcile-suggest.md").is_file());
    assert_eq!(
        kept_files(&state),
        kept,
        "suggest rewrote leases or the mesh"
    );
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reconcile_prints_the_same_mismatch_fail_cite_and_exits_zero() {
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
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        section.contains("research notes-append cell-one-box: would-deny --")
            && section.contains("(mismatch)"),
        "{section}"
    );
    let kept = kept_files(&state);
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
        doctor_cites
    );
    let (convey_ok, convey_out, convey_err) = convey(&estate_path, &state);
    assert!(convey_ok, "{convey_out}\n{convey_err}");
    assert!(convey_err.is_empty(), "{convey_err}");
    assert_eq!(
        between_agents_and_authority(&convey_out, &agents),
        doctor_cites
    );
    let (ok, stdout, stderr) = reconcile(&estate_path, &state, false);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(
        stderr.is_empty(),
        "a hop cite does not fail reconcile\n{stderr}"
    );
    assert_eq!(
        between_agents_and_authority(&stdout, &agents),
        doctor_cites,
        "reconcile cite block\n{stdout}"
    );
    assert_stack_once_before_report(&stdout, &agents, &section);
    assert_eq!(kept_files(&state), kept);
    let (suggest_ok, suggest_out, suggest_err) = reconcile(&estate_path, &state, true);
    assert!(suggest_ok, "{suggest_out}\n{suggest_err}");
    assert!(suggest_err.is_empty(), "{suggest_err}");
    assert_eq!(
        between_agents_and_authority(&suggest_out, &agents),
        doctor_cites
    );
    assert_stack_once_before_report(&suggest_out, &agents, &section);
    assert_eq!(kept_files(&state), kept);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn reconcile_stays_quiet_when_hop_coverage_matches() {
    let root = doctor_root(&estate_with_lane_tool("allow"));
    let estate_path = root.join("examples/estate.yaml");
    let state = root.join("state");
    apply(&estate_path, &state, &root);
    conveyor_proxy::persist_mesh(&state, &granted_box("lane-tool")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        section.contains("research lane-tool cell-one-box: would-allow --"),
        "{section}"
    );
    let kept = kept_files(&state);
    let (doctor_ok, doctor_out, doctor_err) = doctor(&root, &state);
    assert!(doctor_ok, "{doctor_out}\n{doctor_err}");
    assert_eq!(between_agents_and_authority(&doctor_out, &agents), "");
    let (status_ok, status_out, status_err) = status(&estate_path, &state, &root, &root);
    assert!(status_ok, "{status_out}\n{status_err}");
    assert_eq!(between_agents_and_authority(&status_out, &agents), "");
    let (convey_ok, convey_out, convey_err) = convey(&estate_path, &state);
    assert!(convey_ok, "{convey_out}\n{convey_err}");
    assert_eq!(between_agents_and_authority(&convey_out, &agents), "");
    let (ok, stdout, stderr) = reconcile(&estate_path, &state, false);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(
        between_agents_and_authority(&stdout, &agents),
        "",
        "a match adds no hop cite\n{stdout}"
    );
    assert!(!stdout.contains("refuse:hop-coverage"), "{stdout}");
    assert_stack_once_before_report(&stdout, &agents, &section);
    let (suggest_ok, suggest_out, suggest_err) = reconcile(&estate_path, &state, true);
    assert!(suggest_ok, "{suggest_out}\n{suggest_err}");
    assert_eq!(between_agents_and_authority(&suggest_out, &agents), "");
    assert!(
        !suggest_out.contains("refuse:hop-coverage"),
        "{suggest_out}"
    );
    assert_stack_once_before_report(&suggest_out, &agents, &section);
    assert_eq!(kept_files(&state), kept);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn reconcile_missing_mesh_prints_empty_cites_and_not_enforced() {
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
    assert!(no_enforced_status_token(&section), "{section}");
    let kept = kept_files(&state);
    let (ok, stdout, stderr) = reconcile(&estate_path, &state, false);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(between_agents_and_authority(&stdout, &agents), "");
    assert!(!stdout.contains("refuse:hop-coverage"), "{stdout}");
    assert_stack_once_before_report(&stdout, &agents, &section);
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("conveyor-leases.json").exists());
    assert_eq!(kept_files(&state), kept);
    let (suggest_ok, suggest_out, suggest_err) = reconcile(&estate_path, &state, true);
    assert!(suggest_ok, "{suggest_out}\n{suggest_err}");
    assert_eq!(between_agents_and_authority(&suggest_out, &agents), "");
    assert!(suggest_out.contains(&section), "{suggest_out}");
    assert!(!state.join("conveyor-mesh.json").exists());
    assert_eq!(kept_files(&state), kept);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unreadable_mesh_refuses_before_sections_and_writes_nothing() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let mesh = state.join("conveyor-mesh.json");
    std::fs::write(&mesh, "not-json").unwrap();
    std::fs::write(state.join("reconcile.json"), "sentinel\n").unwrap();
    std::fs::write(state.join("reconcile-suggest.md"), "sentinel-suggest\n").unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let before = snapshot(&state);
    for suggest in [false, true] {
        let (ok, stdout, stderr) = reconcile(&estate_path, &state, suggest);
        assert!(!ok, "suggest={suggest} {stdout}\n{stderr}");
        assert!(stdout.trim().is_empty(), "{stdout}");
        assert!(no_sections(&stdout), "{stdout}");
        assert!(no_sections(&stderr), "{stderr}");
        assert!(stderr.contains("parse:"), "{stderr}");
        assert!(stderr.contains("conveyor-mesh.json"), "{stderr}");
        assert!(no_enforced_status_token(&stdout), "{stdout}");
        assert!(no_enforced_status_token(&stderr), "{stderr}");
        assert_eq!(snapshot(&state), before);
        assert_eq!(std::fs::read(&mesh).unwrap(), b"not-json");
        assert_eq!(
            std::fs::read_to_string(state.join("reconcile.json")).unwrap(),
            "sentinel\n"
        );
        assert_eq!(
            std::fs::read_to_string(state.join("reconcile-suggest.md")).unwrap(),
            "sentinel-suggest\n"
        );
    }

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[],"leases":[{"hop_id":"box","kind":"box","capability":"lane-tool","host_class":"rtx-5090","granted":true,"spawned":false,"durable":true,"driver":"box"}]}"#,
    )
    .unwrap();
    let before = snapshot(&state);
    for suggest in [false, true] {
        let (ok, stdout, stderr) = reconcile(&estate_path, &state, suggest);
        assert!(!ok, "suggest={suggest} {stdout}\n{stderr}");
        assert!(stdout.trim().is_empty(), "{stdout}");
        assert!(no_sections(&stdout), "{stdout}");
        assert!(no_sections(&stderr), "{stderr}");
        assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
        assert!(stderr.contains("rtx-5090"), "{stderr}");
        assert_eq!(snapshot(&state), before);
    }
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn placement_drift_fails_closed_after_sections_and_suggest_does_not_rewrite_leases() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let leases_path = state.join("placement-actual.json");
    let mut actual: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&leases_path).unwrap()).unwrap();
    let leases = actual["leases"].as_array_mut().expect("leases");
    leases.retain(|lease| lease["placement_id"] != "cell-one-box");
    std::fs::write(&leases_path, serde_json::to_string_pretty(&actual).unwrap()).unwrap();
    let leases_before = std::fs::read(&leases_path).unwrap();
    let mesh_before = std::fs::read(state.join("conveyor-mesh.json")).unwrap();
    let audit_before = std::fs::read(state.join("apply-audit.jsonl")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    let (ok, stdout, stderr) = reconcile(&estate_path, &state, false);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("reconcile drift (fail closed)"), "{stderr}");
    assert!(
        !stderr.contains("refuse:hop-coverage"),
        "a hop cite must not become the bail\n{stderr}"
    );
    let cites = between_agents_and_authority(&stdout, &agents);
    assert!(
        cites.contains("  note  refuse:hop-coverage:") && cites.contains("(deny-default)"),
        "{cites}"
    );
    assert_stack_once_before_report(&stdout, &agents, &section);
    assert!(
        stdout.contains("refuse:missing-lease"),
        "report still follows the sections\n{stdout}"
    );
    let report_at = stdout
        .find("Placement reconcile (desired vs actual)")
        .unwrap();
    let missing_at = stdout.find("refuse:missing-lease").unwrap();
    assert!(report_at < missing_at, "{stdout}");
    assert_eq!(std::fs::read(&leases_path).unwrap(), leases_before);
    assert_eq!(
        std::fs::read(state.join("conveyor-mesh.json")).unwrap(),
        mesh_before
    );
    assert_eq!(
        std::fs::read(state.join("apply-audit.jsonl")).unwrap(),
        audit_before
    );
    assert!(state.join("reconcile.json").is_file());
    let body = std::fs::read_to_string(state.join("reconcile.json")).unwrap();
    assert!(body.contains("missing-lease"), "{body}");
    assert!(!state.join("reconcile-suggest.md").exists());

    let (suggest_ok, suggest_out, suggest_err) = reconcile(&estate_path, &state, true);
    assert!(!suggest_ok, "{suggest_out}\n{suggest_err}");
    assert!(
        suggest_err.contains("reconcile drift (fail closed)"),
        "{suggest_err}"
    );
    assert!(
        !suggest_err.contains("refuse:hop-coverage"),
        "{suggest_err}"
    );
    assert_eq!(between_agents_and_authority(&suggest_out, &agents), cites);
    assert_stack_once_before_report(&suggest_out, &agents, &section);
    assert!(suggest_out.contains("auto_apply: false"), "{suggest_out}");
    assert!(suggest_out.contains("Not applied"), "{suggest_out}");
    assert_eq!(suggest_out.matches("Agents\n------\n").count(), 1);
    assert_eq!(std::fs::read(&leases_path).unwrap(), leases_before);
    assert_eq!(
        std::fs::read(state.join("conveyor-mesh.json")).unwrap(),
        mesh_before
    );
    assert_eq!(
        std::fs::read(state.join("apply-audit.jsonl")).unwrap(),
        audit_before
    );
    assert!(state.join("reconcile-suggest.md").is_file());
    let patch = std::fs::read_to_string(state.join("reconcile-suggest.md")).unwrap();
    assert!(patch.contains("auto_apply: false"));
    assert!(patch.contains("Not applied"));
    assert!(patch.contains("missing-lease"));
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn placement_sku_still_prints_the_reconcile_report_and_does_not_rewrite_leases() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let leases_path = state.join("placement-actual.json");
    let mut actual: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&leases_path).unwrap()).unwrap();
    let leases = actual["leases"].as_array_mut().expect("leases");
    let box_lease = leases
        .iter_mut()
        .find(|lease| lease["placement_id"] == "cell-one-box")
        .expect("cell-one-box");
    box_lease["host_class"] = serde_json::Value::String("rtx-5090".into());
    std::fs::write(&leases_path, serde_json::to_string_pretty(&actual).unwrap()).unwrap();
    let leases_before = std::fs::read(&leases_path).unwrap();
    let mesh_before = std::fs::read(state.join("conveyor-mesh.json")).unwrap();
    let (ok, stdout, stderr) = reconcile(&estate_path, &state, false);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains("reconcile drift (fail closed)"),
        "{stderr}"
    );
    assert!(stdout.contains("Placement reconcile (desired vs actual)"), "{stdout}");
    assert!(stdout.contains("rtx-5090"), "{stdout}");
    assert!(stdout.contains("refuse:bad-host-class"), "{stdout}");
    assert!(state.join("reconcile.json").is_file());
    assert_eq!(std::fs::read(&leases_path).unwrap(), leases_before);
    assert_eq!(
        std::fs::read(state.join("conveyor-mesh.json")).unwrap(),
        mesh_before
    );
    let (suggest_ok, suggest_out, suggest_err) = reconcile(&estate_path, &state, true);
    assert!(!suggest_ok, "{suggest_out}\n{suggest_err}");
    assert!(suggest_err.contains("reconcile drift (fail closed)"), "{suggest_err}");
    assert!(suggest_out.contains("auto_apply: false"), "{suggest_out}");
    assert_eq!(std::fs::read(&leases_path).unwrap(), leases_before);
    assert_eq!(
        std::fs::read(state.join("conveyor-mesh.json")).unwrap(),
        mesh_before
    );
    assert!(state.join("reconcile-suggest.md").is_file());
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reconcile_help_names_the_shared_stack_before_the_report() {
    let (ok, stdout, stderr) = run(&["reconcile", "--help"]);
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
    assert!(stdout.contains("do not"), "{stdout}");
    assert!(stdout.contains("empty cite list"), "{stdout}");
    assert!(stdout.contains("Does not spawn"), "{stdout}");
    assert!(stdout.contains("does not rewrite leases"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");

    let (help_ok, help_out, help_err) = run(&["help", "reconcile"]);
    assert!(help_ok, "{help_out}\n{help_err}");
    let agents = help_out
        .find("describe_agents_section")
        .unwrap_or_else(|| panic!("topic omits describe_agents_section\n{help_out}"));
    let cites = help_out
        .find("hop_coverage_cites")
        .unwrap_or_else(|| panic!("topic omits hop_coverage_cites\n{help_out}"));
    let authority = help_out
        .find("describe_authority_section")
        .unwrap_or_else(|| panic!("topic omits describe_authority_section\n{help_out}"));
    assert!(agents < cites && cites < authority, "{help_out}");
    assert!(help_out.contains("print_hop_coverage_cites"), "{help_out}");
    assert!(help_out.contains("empty cite list"), "{help_out}");
    assert!(help_out.contains("missing-mesh"), "{help_out}");
    assert!(help_out.contains("does not spawn"), "{help_out}");
    assert!(help_out.contains("does not rewrite leases"), "{help_out}");
    assert!(
        help_out.contains("Placement drift still fails closed"),
        "{help_out}"
    );
    assert!(!help_out.contains("READY_FOR_LIVE_TEST: yes"), "{help_out}");
    assert_locked_cksum();
}
