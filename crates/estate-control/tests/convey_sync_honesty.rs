//! `estate convey sync` prints the same Agents, hop-cite, and Authority stack
//! as status, doctor, reconcile, leases, convey leases, convey list, convey
//! expire, audits, history, and audit export, before it writes the mesh.
//! Mismatch is `FAIL`. Deny and deny-default are `note`. A match is quiet.
//! None of those cites fail the command. A placement-actual SKU omits
//! Authority. `sync_from_placements_covering` then still refuses that SKU
//! before the mesh JSON and before the write. A missing estate file keeps
//! the lease stub sync and does not invent the stack.

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
    let dir = std::env::temp_dir().join(format!("cell-convey-sync-honesty-{nanos}"));
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
        && !text.contains("cell-one.conveyor-mesh.v0")
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

fn convey_sync(estate: &Path, state: &Path) -> (bool, String, String) {
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    run(&[
        "convey",
        "sync",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ])
}

fn convey_authority(estate: &Path, state: &Path) -> (bool, String, String) {
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

fn patch_lease(state: &Path, id: &str, patch: impl FnOnce(&mut serde_json::Value)) {
    let path = state.join("placement-actual.json");
    let mut actual: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let rows = actual["leases"].as_array_mut().expect("leases");
    let lease = rows
        .iter_mut()
        .find(|lease| lease["placement_id"] == id)
        .unwrap_or_else(|| panic!("missing placement {id}"));
    patch(lease);
    std::fs::write(&path, serde_json::to_string_pretty(&actual).unwrap()).unwrap();
}

fn assert_stack_then_mesh(stdout: &str, agents: &str, authority_out: &str, mesh_body: &str) {
    assert!(stdout.starts_with(authority_out), "{stdout}");
    assert_eq!(
        stdout.strip_prefix(authority_out).unwrap(),
        format!("{mesh_body}\n"),
        "{stdout}"
    );
    assert_eq!(stdout.matches("Agents\n------\n").count(), 1, "{stdout}");
    assert_eq!(
        stdout.matches("Authority\n---------\n").count(),
        1,
        "{stdout}"
    );
    let agents_at = stdout.find(agents).unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    let json_at = stdout
        .find("cell-one.conveyor-mesh.v0")
        .unwrap_or_else(|| panic!("missing mesh JSON\n{stdout}"));
    assert!(agents_at < auth_at && auth_at < json_at, "{stdout}");
    assert!(stdout.contains("would-allow="), "{stdout}");
    assert!(stdout.contains("would-deny="), "{stdout}");
    assert!(stdout.contains("not-enforced="), "{stdout}");
    assert!(
        stdout.contains("does not show that a worker called the conveyor"),
        "{stdout}"
    );
    assert!(no_enforced_status_token(stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
}

fn box_capability(mesh_body: &str) -> String {
    let mesh: serde_json::Value = serde_json::from_str(mesh_body).unwrap();
    mesh["hops"]
        .as_array()
        .unwrap()
        .iter()
        .find(|hop| hop["id"] == "cell-one-box")
        .unwrap()["capability"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn convey_sync_prints_agents_cites_and_authority_before_the_mesh_json() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let placement = std::fs::read(state.join("placement-actual.json")).unwrap();
    let audit = std::fs::read(state.join("apply-audit.jsonl")).unwrap();
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = convey_sync(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let cites = between_agents_and_authority(&stdout, &agents);
    assert!(
        cites.contains("  note  refuse:hop-coverage:")
            && cites.contains("(deny-default)")
            && !cites.contains("(mismatch)")
            && !cites.contains("  FAIL  "),
        "{cites}"
    );
    assert_eq!(between_agents_and_authority(&authority_out, &agents), cites);
    let mesh_body = std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap();
    assert_stack_then_mesh(&stdout, &agents, &authority_out, &mesh_body);
    assert_eq!(box_capability(&mesh_body), "lane-tool", "{mesh_body}");
    assert!(state.join("conveyor-hops.json").is_file());
    assert!(state.join("conveyor-leases.json").is_file());
    assert_eq!(
        std::fs::read(state.join("placement-actual.json")).unwrap(),
        placement
    );
    assert_eq!(
        std::fs::read(state.join("apply-audit.jsonl")).unwrap(),
        audit
    );
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hop_mismatch_cite_does_not_fail_convey_sync() {
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
    let placement = std::fs::read(state.join("placement-actual.json")).unwrap();
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = convey_sync(&estate_path, &state);
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
    let json_at = stdout.find("cell-one.conveyor-mesh.v0").unwrap();
    assert!(
        agents_at < cite_at && cite_at < auth_at && auth_at < json_at,
        "{stdout}"
    );
    let mesh_body = std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap();
    assert_eq!(
        stdout.strip_prefix(&authority_out).unwrap(),
        format!("{mesh_body}\n"),
        "{stdout}"
    );
    assert_eq!(box_capability(&mesh_body), "lane-tool", "{mesh_body}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert_eq!(
        std::fs::read(state.join("placement-actual.json")).unwrap(),
        placement
    );
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn hop_coverage_refuse_prints_the_stack_and_writes_nothing() {
    let root = doctor_root(&estate_with_lane_tool("allow"));
    let estate_path = root.join("examples/estate.yaml");
    let state = root.join("state");
    apply(&estate_path, &state, &root);
    assert!(!state.join("conveyor-mesh.json").exists());
    patch_lease(&state, "cell-one-box", |lease| {
        lease["kind"] = serde_json::Value::String("cloud-agent".into());
        lease["spawned"] = serde_json::Value::Bool(false);
    });
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_sync(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert_eq!(stdout, authority_out, "{stdout}");
    assert!(
        stderr.contains("refuse:hop-coverage")
            && stderr.contains("(mismatch)")
            && stderr.contains("mesh-stub")
            && stderr.contains("lane-tool"),
        "{stderr}"
    );
    assert!(!stdout.contains("cell-one.conveyor-mesh.v0"), "{stdout}");
    assert!(stdout.contains("Agents\n------"), "{stdout}");
    assert!(stdout.contains("Authority\n---------"), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("conveyor-hops.json").exists());
    assert!(!state.join("conveyor-leases.json").exists());
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn missing_mesh_is_not_enforced_then_sync_writes() {
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
    let placement = std::fs::read(state.join("placement-actual.json")).unwrap();
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = convey_sync(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(between_agents_and_authority(&stdout, &agents), "");
    let mesh_body = std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap();
    assert_stack_then_mesh(&stdout, &agents, &authority_out, &mesh_body);
    assert!(stdout.contains(&section), "{stdout}");
    assert!(
        authority_out.contains("conveyor-mesh.json is absent"),
        "{authority_out}"
    );
    assert_eq!(box_capability(&mesh_body), "lane-tool", "{mesh_body}");
    assert!(mesh_body.contains("mesh-stub"), "{mesh_body}");
    assert_eq!(
        std::fs::read(state.join("placement-actual.json")).unwrap(),
        placement
    );
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn spawned_cloud_prints_the_stack_and_refuses_before_write() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    assert!(!state.join("conveyor-mesh.json").exists());
    patch_lease(&state, "cursor-cloud", |lease| {
        lease["spawned"] = serde_json::Value::Bool(true);
    });
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_sync(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert_eq!(stdout, authority_out, "{stdout}");
    assert!(
        stderr.contains("refuse:cloud-spawned") && stderr.contains("cursor-cloud"),
        "{stderr}"
    );
    assert!(!stdout.contains("cell-one.conveyor-mesh.v0"), "{stdout}");
    assert!(stdout.contains("Agents\n------"), "{stdout}");
    assert!(stdout.contains("Authority\n---------"), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!state.join("conveyor-mesh.json").exists());
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agent_unplaced_refuses_before_the_mesh_write() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    let mut ahead = granted_box("notes-append");
    ahead.hops[0].agents = vec!["research".into(), "outsider".into()];
    ahead.leases[0].agents = vec!["research".into(), "outsider".into()];
    conveyor_proxy::persist_mesh(&state, &ahead).unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_sync(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("refuse:agent-unplaced"), "{stderr}");
    assert!(stderr.contains("outsider"), "{stderr}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mesh_parse_and_mesh_host_class_refuse_before_the_mesh_write() {
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
    let (ok, stdout, stderr) = convey_sync(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(stderr.contains("conveyor-mesh.json"), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_eq!(std::fs::read(&mesh).unwrap(), b"not-json");

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[{"id":"cell-one-box","kind":"box","capability":"notes-append","host_class":"rtx-5090","wired":true,"agents":["research"]}],"leases":[{"hop_id":"box","kind":"box","capability":"lane-tool","host_class":"rtx-5090","granted":true,"spawned":false,"durable":true,"driver":"box"}]}"#,
    )
    .unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_sync(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(stderr.contains("rtx-5090"), "{stderr}");
    assert_eq!(snapshot(&state), before);

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[{"id":"cell-one-box","kind":"box","capability":"notes-append","host_class":"any","wired":true,"agents":["research"]}],"leases":[]}"#,
    )
    .unwrap();
    std::fs::write(state.join("placement-actual.json"), "{").unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_sync(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(stderr.contains("placement-actual.json"), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn placement_sku_prints_agents_and_cites_omits_authority_and_refuses_before_json() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let between = between_agents_and_authority(&authority_out, &agents);
    assert!(
        between.contains("  note  refuse:hop-coverage:") && between.contains("(deny-default)"),
        "{between}"
    );

    patch_lease(&state, "cell-one-box", |lease| {
        lease["host_class"] = serde_json::Value::String("rtx-5090".into());
    });
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_sync(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(stderr.contains("rtx-5090"), "{stderr}");
    assert!(stdout.starts_with(&agents), "{stdout}");
    assert_eq!(&stdout[agents.len()..], format!("{between}\n"));
    assert!(!stdout.contains("Authority\n---------"), "{stdout}");
    assert!(!stdout.contains("not-enforced reasons:"), "{stdout}");
    assert!(!stdout.contains("would-allow="), "{stdout}");
    assert!(!stdout.contains("cell-one.conveyor-mesh.v0"), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stderr.contains("live PASS"), "{stderr}");
    assert_eq!(snapshot(&state), before);
    let mesh_body = std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap();
    assert_eq!(box_capability(&mesh_body), "notes-append", "{mesh_body}");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_estate_stub_syncs_without_a_stack() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(
        state.join("placement-actual.json"),
        r#"{"leases":[{"placement_id":"cell-one-box","kind":"cloud-agent","host_class":"any","spawned":false,"wired":false,"agents":["research"]}]}"#,
    )
    .unwrap();
    let missing = dir.join("no-such-estate.yaml");
    let (ok, stdout, stderr) = convey_sync(&missing, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.starts_with("{\n"), "{stdout}");
    assert!(!stdout.contains("Agents\n------"), "{stdout}");
    assert!(!stdout.contains("Authority\n---------"), "{stdout}");
    assert!(!stdout.contains("not-enforced"), "{stdout}");
    assert!(stdout.contains("mesh-stub"), "{stdout}");
    assert!(stdout.contains("cell-one-box"), "{stdout}");
    let mesh_body = std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap();
    assert_eq!(stdout, format!("{mesh_body}\n"));
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");

    let bad_state = dir.join("state-bad");
    std::fs::create_dir_all(&bad_state).unwrap();
    std::fs::write(
        bad_state.join("placement-actual.json"),
        std::fs::read(state.join("placement-actual.json")).unwrap(),
    )
    .unwrap();
    let bad = dir.join("bad-estate.yaml");
    std::fs::write(&bad, "not an estate: [\n").unwrap();
    let before = snapshot(&bad_state);
    let (ok, stdout, stderr) = convey_sync(&bad, &bad_state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("bad-estate.yaml"), "{stderr}");
    assert!(!bad_state.join("conveyor-mesh.json").exists());
    assert_eq!(snapshot(&bad_state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_names_the_stack_and_locks_hold() {
    let (ok, stdout, stderr) = run(&["convey", "sync", "--help"]);
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
    assert!(stdout.contains("would-allow"), "{stdout}");
    assert!(stdout.contains("would-deny"), "{stdout}");
    assert!(stdout.contains("not-enforced"), "{stdout}");
    assert!(stdout.contains("not-enforced reasons:"), "{stdout}");
    assert!(stdout.contains("do not fail"), "{stdout}");
    assert!(stdout.contains("empty cite list"), "{stdout}");
    assert!(stdout.contains("refuse:agent-unplaced"), "{stdout}");
    assert!(
        stdout.contains("Sync still refuses that SKU before the mesh JSON"),
        "{stdout}"
    );
    assert!(stdout.contains("sync_from_placements_covering"), "{stdout}");
    assert!(stdout.contains("reads placement-actual"), "{stdout}");
    assert!(stdout.contains("placement-actual parse"), "{stdout}");
    assert!(stdout.contains("diverges"), "{stdout}");
    assert!(stdout.contains("refuse:hop-coverage"), "{stdout}");
    assert!(stdout.contains("writes nothing"), "{stdout}");
    assert!(
        stdout.contains("missing estate file keeps the lease stub sync"),
        "{stdout}"
    );
    assert!(stdout.contains("before the mesh write"), "{stdout}");
    assert!(!stdout.contains("body still prints"), "{stdout}");
    assert!(!stdout.contains("no second placement refuse"), "{stdout}");
    assert!(!stdout.contains("do not depend on placement"), "{stdout}");
    assert!(!stdout.contains("do not read placement"), "{stdout}");
    assert!(!stdout.contains("ignores placement"), "{stdout}");
    assert!(stdout.contains("examples/estate.yaml"), "{stdout}");
    assert!(stdout.contains("Does not spawn"), "{stdout}");
    assert!(stdout.contains("Does not claim mediation"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");

    let language =
        std::fs::read_to_string(repo_root().join("docs/UBIQUITOUS_LANGUAGE.md")).unwrap();
    let at = language
        .find("`estate convey sync` prints")
        .unwrap_or_else(|| panic!("language omits convey sync\n{language}"));
    let mine = &language[at..];
    let end = mine
        .find("Whether a hop lease")
        .unwrap_or_else(|| panic!("language convey sync sentence did not end\n{mine}"));
    let mine = &mine[..end];
    assert!(mine.contains("missing-mesh"), "{mine}");
    assert!(mine.contains("conveyor-mesh.json"), "{mine}");
    assert!(mine.contains("still reads placement-actual"), "{mine}");
    assert!(mine.contains("placement-actual parse failure"), "{mine}");
    assert!(
        mine.contains("sync still refuses that SKU before the mesh JSON"),
        "{mine}"
    );
    assert!(mine.contains("sync_from_placements_covering"), "{mine}");
    assert!(mine.contains("diverges"), "{mine}");
    assert!(mine.contains("refuse:hop-coverage"), "{mine}");
    assert!(mine.contains("writes nothing"), "{mine}");
    assert!(
        mine.contains("missing estate file keeps the lease stub sync"),
        "{mine}"
    );
    assert!(mine.contains("before the mesh write"), "{mine}");
    assert!(!mine.contains("body still prints"), "{mine}");
    assert!(!mine.contains("no second placement refuse"), "{mine}");
    assert!(!mine.contains("do not depend on placement"), "{mine}");
    assert!(!mine.contains("do not read placement"), "{mine}");
    assert!(!mine.contains("ignores placement"), "{mine}");
    assert!(!mine.contains("READY_FOR_LIVE_TEST: yes"), "{mine}");

    let changelog = std::fs::read_to_string(repo_root().join("CHANGELOG.md")).unwrap();
    let head = changelog
        .split("## This slice — estate convey expire")
        .next()
        .unwrap();
    assert!(
        head.contains("estate convey sync prints the honesty stack"),
        "{head}"
    );
    assert!(head.contains("render_hop_coverage_cites"), "{head}");
    assert!(head.contains("still reads placement-actual"), "{head}");
    assert!(head.contains("placement-actual parse failure"), "{head}");
    assert!(
        head.contains("sync still refuses that SKU before the mesh JSON"),
        "{head}"
    );
    assert!(head.contains("sync_from_placements_covering"), "{head}");
    assert!(head.contains("diverges"), "{head}");
    assert!(head.contains("writes nothing"), "{head}");
    assert!(
        head.contains("missing estate file keeps the lease stub sync"),
        "{head}"
    );
    assert!(!head.contains("body still prints"), "{head}");
    assert!(!head.contains("no second placement refuse"), "{head}");
    assert!(!head.contains("do not depend on placement"), "{head}");
    assert!(!head.contains("do not read placement"), "{head}");
    assert!(!head.contains("ignores placement"), "{head}");
    assert!(head.contains("43770130 3391"), "{head}");
    assert!(head.contains("`READY_FOR_LIVE_TEST`: no"), "{head}");
    assert!(!head.contains("READY_FOR_LIVE_TEST: yes"), "{head}");
    assert!(head.contains("does not invent a live PASS"), "{head}");
    assert_locked_cksum();
}
