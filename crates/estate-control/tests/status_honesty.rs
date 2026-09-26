//! `estate status` prints the shared honesty stack after the page header
//! and the pre-stack refuses. Mismatch is `FAIL`. Deny and deny-default
//! are `note`. A match is quiet. None of those cites fail status. A
//! missing mesh is an empty cite list and Authority stays `not-enforced`
//! (`missing-mesh`). A placement-actual SKU refuses before the page via
//! `refuse_lease_host_classes`. The stack is not reached. The page does
//! not print. There is no later mesh reader after the stack.

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
    let dir = std::env::temp_dir().join(format!("cell-status-honesty-{nanos}"));
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

fn no_stack(text: &str) -> bool {
    !text.contains("Agents\n------")
        && !text.contains("Authority\n---------")
        && !text.contains("would-allow=")
        && !text.contains("refuse:hop-coverage")
        && !text.contains("Cell One status")
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
    let mut mesh = conveyor_proxy::ConveyorMesh {
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
    };
    mesh.hops.push(conveyor_proxy::HopDecl {
        id: "cell-one-box".into(),
        kind: "box".into(),
        capability: capability.into(),
        host_class: "any".into(),
        wired: true,
        note: None,
        ttl_secs: None,
        agents: vec!["research".into()],
    });
    mesh
}

fn status(estate: &Path, state: &Path, scratch_root: &Path) -> (bool, String, String) {
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    let plans = scratch_root.join("plans");
    let packs = scratch_root.join("packs");
    let roots = scratch_root.join("roots");
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
    let root_s = repo_root().display().to_string();
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

#[test]
fn stack_follows_the_page_header_and_missing_mesh_stays_not_enforced() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        section.contains("not-enforced reasons: missing-mesh="),
        "{section}"
    );
    assert!(
        section.contains("conveyor-mesh.json is absent"),
        "{section}"
    );
    assert!(no_enforced_status_token(&section), "{section}");
    let before = snapshot(&state);
    let (ok, stdout, stderr) = status(&estate_path, &state, &dir);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.starts_with("Cell One status\n"), "{stdout}");
    let cloud = "cloud-agent: declared, not spawned\n";
    let cloud_at = stdout
        .find(cloud)
        .unwrap_or_else(|| panic!("missing cloud-agent line\n{stdout}"));
    assert!(
        stdout[..cloud_at].contains("expired: placement="),
        "{stdout}"
    );
    assert_eq!(
        stdout[cloud_at + cloud.len()..].trim_end(),
        format!("{agents}\n{section}")
    );
    assert!(!stdout.contains("refuse:hop-coverage"), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("placement-actual.json").exists());
    assert!(!state.join("apply-audit.jsonl").exists());
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn canonical_placement_still_prints_the_stack_after_the_header() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let before = snapshot(&state);
    let (ok, stdout, stderr) = status(&estate_path, &state, &dir);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.starts_with("Cell One status\n"), "{stdout}");
    let cloud = "cloud-agent: declared, not spawned\n";
    let cloud_at = stdout
        .find(cloud)
        .unwrap_or_else(|| panic!("missing cloud-agent line\n{stdout}"));
    let tail = &stdout[cloud_at + cloud.len()..];
    assert!(tail.starts_with(&agents), "{tail}");
    assert!(tail.contains("\nAuthority\n---------\n"), "{tail}");
    assert!(
        tail.contains("not-enforced reasons: missing-mesh="),
        "{tail}"
    );
    assert!(tail.contains("conveyor-mesh.json is absent"), "{tail}");
    assert!(!tail.contains("refuse:hop-coverage"), "{tail}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cites_do_not_fail_status() {
    let dir = scratch();
    let estate_path = dir.join("deny.yaml");
    std::fs::write(&estate_path, estate_with_lane_tool("deny")).unwrap();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    conveyor_proxy::persist_mesh(&state, &granted_box("lane-tool")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let before = snapshot(&state);
    let (ok, stdout, stderr) = status(&estate_path, &state, &dir);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let cloud = "cloud-agent: declared, not spawned\n";
    let cloud_at = stdout.find(cloud).unwrap();
    let tail = &stdout[cloud_at + cloud.len()..];
    assert!(tail.starts_with(&agents), "{tail}");
    let cite = "  note  refuse:hop-coverage: research hop cell-one-box lane-tool: deny (deny)";
    assert!(
        tail.contains(&format!("\n{cite}\n")),
        "deny stays a note between Agents and Authority\n{tail}"
    );
    assert!(tail.contains("\nAuthority\n---------\n"), "{tail}");
    assert!(!stdout.contains("  FAIL  refuse:hop-coverage:"), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_eq!(snapshot(&state), before);

    let mut mismatch = estate_with_lane_tool("allow");
    mismatch = mismatch.replace(
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n",
    );
    let mismatch_path = dir.join("mismatch.yaml");
    std::fs::write(&mismatch_path, mismatch).unwrap();
    let mismatch_state = dir.join("mismatch-state");
    std::fs::create_dir_all(&mismatch_state).unwrap();
    conveyor_proxy::persist_mesh(&mismatch_state, &granted_box("notes-append")).unwrap();
    let mismatch_estate = estate_schema::load_estate(&mismatch_path).unwrap();
    let mismatch_agents = estate_schema::describe_agents_section(&mismatch_estate);
    let mesh_before = snapshot(&mismatch_state);
    let (ok, stdout, stderr) = status(&mismatch_path, &mismatch_state, &dir);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let cloud_at = stdout.find(cloud).unwrap();
    let tail = &stdout[cloud_at + cloud.len()..];
    assert!(tail.starts_with(&mismatch_agents), "{tail}");
    assert!(
        tail.contains("\n  FAIL  refuse:hop-coverage:") && tail.contains("(mismatch)"),
        "{tail}"
    );
    assert!(tail.contains("\nAuthority\n---------\n"), "{tail}");
    let agents_at = tail.find(&mismatch_agents).unwrap();
    let after_agents = &tail[agents_at + mismatch_agents.len()..];
    let authority_at = after_agents.find("\nAuthority\n---------\n").unwrap();
    let cites = &after_agents[..authority_at];
    assert!(cites.contains("  FAIL  "), "{cites}");
    assert!(!cites.contains("  note  refuse:hop-coverage:"), "{cites}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_eq!(snapshot(&mismatch_state), mesh_before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn placement_sku_refuses_before_the_page() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
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
    let (ok, stdout, stderr) = status(&estate_path, &state, &dir);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains(
            "refuse:bad-host-class: lease 'cell-one-box' host_class 'rtx-5090' must be consumer-nvidia|apple-silicon|rented-nvidia|any"
        ),
        "{stderr}"
    );
    assert!(
        !stderr.contains("hop host_class"),
        "lease host-class check returns before list_expired_hop_leases\n{stderr}"
    );
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(no_stack(&stderr), "{stderr}");
    assert!(!stdout.contains("Authority\n---------"), "{stdout}");
    assert!(!stderr.contains("would-allow="), "{stderr}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stderr.contains("live PASS"), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unreadable_mesh_still_refuses_before_the_page() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let mesh = state.join("conveyor-mesh.json");
    std::fs::write(&mesh, "not-json").unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let before = snapshot(&state);
    let (ok, stdout, stderr) = status(&estate_path, &state, &dir);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(stderr.contains("conveyor-mesh.json"), "{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(no_stack(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_eq!(std::fs::read(&mesh).unwrap(), b"not-json");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_names_the_stack_and_locks_hold() {
    let (ok, stdout, stderr) = run(&["status", "--help"]);
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
    assert!(stdout.contains("honesty_stack"), "{stdout}");
    assert!(stdout.contains("render_hop_coverage_cites"), "{stdout}");
    assert!(stdout.contains("would-allow"), "{stdout}");
    assert!(stdout.contains("would-deny"), "{stdout}");
    assert!(stdout.contains("not-enforced"), "{stdout}");
    assert!(stdout.contains("missing-mesh"), "{stdout}");
    assert!(stdout.contains("do not fail status"), "{stdout}");
    assert!(stdout.contains("empty cite"), "{stdout}");
    assert!(stdout.contains("refuse_lease_host_classes"), "{stdout}");
    assert!(stdout.contains("list_expired_hop_leases"), "{stdout}");
    assert!(stdout.contains("load_interpreted_mesh"), "{stdout}");
    assert!(stdout.contains("before the page"), "{stdout}");
    assert!(stdout.contains("The page does not print"), "{stdout}");
    assert!(
        stdout.contains("does not succeed with Authority omitted"),
        "{stdout}"
    );
    assert!(stdout.contains("no later mesh reader"), "{stdout}");
    assert!(stdout.contains("diverges"), "{stdout}");
    assert!(stdout.contains("examples/estate.yaml"), "{stdout}");
    assert!(stdout.contains("Does not spawn"), "{stdout}");
    assert!(stdout.contains("Does not write"), "{stdout}");
    assert!(!stdout.contains("body still prints"), "{stdout}");
    assert!(!stdout.contains("this file check succeeds"), "{stdout}");
    assert!(!stdout.contains("no second placement refuse"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");

    let (help_ok, help_out, help_err) = run(&["help", "status"]);
    assert!(help_ok, "{help_out}\n{help_err}");
    let help_at = help_out
        .find("After the hop expired count")
        .unwrap_or_else(|| panic!("help omits the status stack\n{help_out}"));
    let help_tail = &help_out[help_at..];
    let help_end = help_tail
        .find("`estate convey authority`")
        .unwrap_or_else(|| panic!("help status section did not end\n{help_tail}"));
    let help_mine = &help_tail[..help_end];
    assert!(help_mine.contains("honesty_stack"), "{help_mine}");
    assert!(
        help_mine.contains("render_hop_coverage_cites"),
        "{help_mine}"
    );
    assert!(help_mine.contains("Neither fails status."), "{help_mine}");
    assert!(help_mine.contains("do not fail status"), "{help_mine}");
    assert!(
        help_mine.contains("refuse_lease_host_classes"),
        "{help_mine}"
    );
    assert!(help_mine.contains("list_expired_hop_leases"), "{help_mine}");
    assert!(help_mine.contains("The page does not print"), "{help_mine}");
    assert!(
        help_mine.contains("does not succeed with Authority omitted"),
        "{help_mine}"
    );
    assert!(help_out.contains("missing-mesh"), "{help_out}");
    assert!(help_out.contains("Does not spawn."), "{help_out}");
    assert!(!help_mine.contains("body still prints"), "{help_mine}");
    assert!(
        !help_mine.contains("this file check succeeds"),
        "{help_mine}"
    );
    assert!(
        !help_mine.contains("no second placement refuse"),
        "{help_mine}"
    );
    assert!(!help_out.contains("READY_FOR_LIVE_TEST: yes"), "{help_out}");

    let language =
        std::fs::read_to_string(repo_root().join("docs/UBIQUITOUS_LANGUAGE.md")).unwrap();
    let at = language
        .find("`estate status` prints")
        .unwrap_or_else(|| panic!("language omits status\n{language}"));
    let mine = &language[at..];
    let end = mine
        .find("`estate reconcile`")
        .unwrap_or_else(|| panic!("language status sentence did not end\n{mine}"));
    let mine = &mine[..end];
    assert!(mine.contains("honesty_stack"), "{mine}");
    assert!(mine.contains("missing-mesh"), "{mine}");
    assert!(mine.contains("conveyor-mesh.json"), "{mine}");
    assert!(mine.contains("refuse_lease_host_classes"), "{mine}");
    assert!(mine.contains("list_expired_hop_leases"), "{mine}");
    assert!(mine.contains("load_interpreted_mesh"), "{mine}");
    assert!(mine.contains("The page does not print"), "{mine}");
    assert!(
        mine.contains("does not succeed with Authority omitted"),
        "{mine}"
    );
    assert!(mine.contains("no later mesh reader"), "{mine}");
    assert!(mine.contains("diverges"), "{mine}");
    assert!(mine.contains("do not fail status"), "{mine}");
    assert!(!mine.contains("body still prints"), "{mine}");
    assert!(!mine.contains("this file check succeeds"), "{mine}");
    assert!(!mine.contains("no second placement refuse"), "{mine}");
    assert!(!mine.contains("READY_FOR_LIVE_TEST: yes"), "{mine}");

    let changelog = std::fs::read_to_string(repo_root().join("CHANGELOG.md")).unwrap();
    let head = changelog
        .split("## This slice — estate convey call")
        .next()
        .unwrap();
    assert!(
        head.contains("estate status prints the honesty stack"),
        "{head}"
    );
    assert!(head.contains("honesty_stack"), "{head}");
    assert!(head.contains("render_hop_coverage_cites"), "{head}");
    assert!(head.contains("missing-mesh"), "{head}");
    assert!(head.contains("refuse_lease_host_classes"), "{head}");
    assert!(head.contains("list_expired_hop_leases"), "{head}");
    assert!(head.contains("load_interpreted_mesh"), "{head}");
    assert!(head.contains("MeshError::BadHostClass"), "{head}");
    assert!(head.contains("The page does not print"), "{head}");
    assert!(
        head.contains("does not succeed with Authority omitted"),
        "{head}"
    );
    assert!(head.contains("no later mesh reader"), "{head}");
    assert!(head.contains("diverges"), "{head}");
    assert!(!head.contains("body still prints"), "{head}");
    assert!(!head.contains("this file check succeeds"), "{head}");
    assert!(!head.contains("no second placement refuse"), "{head}");
    assert!(head.contains("43770130 3391"), "{head}");
    assert!(head.contains("`READY_FOR_LIVE_TEST`: no"), "{head}");
    assert!(!head.contains("READY_FOR_LIVE_TEST: yes"), "{head}");
    assert!(head.contains("does not invent a live PASS"), "{head}");
    assert_locked_cksum();
}
