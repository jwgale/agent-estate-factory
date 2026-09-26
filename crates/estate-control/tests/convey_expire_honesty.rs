//! `estate convey expire` prints the same Agents, hop-cite, and Authority stack
//! as status, doctor, reconcile, leases, convey leases, convey list, audits,
//! history, and audit export, before the expired hop lease list.
//! Mismatch is `FAIL`. Deny and deny-default are `note`. A match is quiet.
//! None of those cites fail the command. A placement-actual SKU omits
//! Authority. `list_expired_hop_leases` then still refuses that SKU before
//! the expired list and before `--forget` rewrites the mesh.

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
    let dir = std::env::temp_dir().join(format!("cell-convey-expire-honesty-{nanos}"));
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
        && !text.contains("expired hop leases")
        && !text.contains("forgot ")
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

fn expired_box(capability: &str) -> conveyor_proxy::ConveyorMesh {
    let mut mesh = granted_box(capability);
    mesh.hops[0].ttl_secs = Some(1);
    mesh.leases[0].ttl_secs = Some(1);
    mesh.leases[0].issued_at = Some(1);
    mesh.leases[0].expires_at = Some(2);
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

fn convey_expire(estate: &Path, state: &Path) -> (bool, String, String) {
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    run(&[
        "convey",
        "expire",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ])
}

fn convey_expire_forget(estate: &Path, state: &Path) -> (bool, String, String) {
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    run(&[
        "convey",
        "expire",
        "--forget",
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

fn expired_body() -> String {
    "expired hop leases (1)\n  refuse:expired: cell-one-box kind=box expires_at=2\n".into()
}

fn assert_stack_before_expired(stdout: &str, agents: &str, section: &str) {
    assert_eq!(stdout.matches("Agents\n------\n").count(), 1, "{stdout}");
    assert_eq!(
        stdout.matches("Authority\n---------\n").count(),
        1,
        "{stdout}"
    );
    let agents_at = stdout.find(agents).unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    let list_at = stdout
        .find("expired hop leases (")
        .unwrap_or_else(|| panic!("missing expired hop lease list\n{stdout}"));
    assert!(agents_at < auth_at && auth_at < list_at, "{stdout}");
    assert!(stdout.contains(section), "{stdout}");
    assert!(stdout.contains("would-allow="), "{stdout}");
    assert!(stdout.contains("would-deny="), "{stdout}");
    assert!(stdout.contains("not-enforced="), "{stdout}");
    assert!(
        stdout.contains("does not show that a worker called the conveyor"),
        "{stdout}"
    );
    assert!(stdout.contains(&expired_body()), "{stdout}");
    assert!(no_enforced_status_token(stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stdout.contains("forgot "), "{stdout}");
}

#[test]
fn convey_expire_prints_agents_cites_and_authority_before_the_expired_list() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    let mesh = expired_box("notes-append");
    conveyor_proxy::persist_mesh(&state, &mesh).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(section.contains("would-allow="), "{section}");
    assert!(section.contains("would-deny"), "{section}");
    assert!(section.contains("not-enforced"), "{section}");
    assert!(section.contains("not-enforced reasons:"), "{section}");
    assert!(no_enforced_status_token(&section), "{section}");
    let before = snapshot(&state);
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = convey_expire(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains("refuse:expired: convey call refuses until estate convey expire --forget"),
        "{stderr}"
    );
    assert!(stdout.starts_with(&authority_out), "{stdout}");
    let cites = between_agents_and_authority(&stdout, &agents);
    assert!(
        cites.contains("  note  refuse:hop-coverage:")
            && cites.contains("(deny-default)")
            && !cites.contains("(mismatch)")
            && !cites.contains("  FAIL  "),
        "{cites}"
    );
    assert_eq!(between_agents_and_authority(&authority_out, &agents), cites);
    assert_stack_before_expired(&stdout, &agents, &section);
    assert_eq!(
        stdout.strip_prefix(&authority_out).unwrap(),
        expired_body(),
        "{stdout}"
    );
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("reconcile.json").exists());
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn forget_prints_the_stack_then_drops_expired_leases() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &expired_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let placement = std::fs::read(state.join("placement-actual.json")).unwrap();
    let audit = std::fs::read(state.join("apply-audit.jsonl")).unwrap();
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = convey_expire_forget(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.starts_with(&authority_out), "{stdout}");
    let agents_at = stdout.find(&agents).unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    let list_at = stdout.find("expired hop leases (").unwrap();
    let forgot_at = stdout
        .find("forgot 1 expired hop lease(s); call restamps from remaining hop decls")
        .unwrap_or_else(|| panic!("missing forget line\n{stdout}"));
    assert!(
        agents_at < auth_at && auth_at < list_at && list_at < forgot_at,
        "{stdout}"
    );
    assert_eq!(
        stdout.strip_prefix(&authority_out).unwrap(),
        format!(
            "{}forgot 1 expired hop lease(s); call restamps from remaining hop decls\n",
            expired_body()
        ),
        "{stdout}"
    );
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    let mesh: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap())
            .unwrap();
    assert!(
        mesh["hops"]
            .as_array()
            .unwrap()
            .iter()
            .any(|hop| hop["id"] == "cell-one-box"),
        "{mesh}"
    );
    assert!(mesh["leases"].as_array().unwrap().is_empty(), "{mesh}");
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
fn hop_mismatch_cite_does_not_fail_convey_expire() {
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
    let (ok, stdout, stderr) = convey_expire(&estate_path, &state);
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
    let list_at = stdout.find("no expired hop leases under").unwrap();
    assert!(
        agents_at < cite_at && cite_at < auth_at && auth_at < list_at,
        "{stdout}"
    );
    assert!(
        stdout.ends_with(&format!(
            "no expired hop leases under {}\n",
            state.display()
        )),
        "{stdout}"
    );
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stdout.contains("forgot "), "{stdout}");
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
    assert!(section.contains("would-allow="), "{section}");
    assert!(section.contains("would-deny="), "{section}");
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_expire(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(between_agents_and_authority(&stdout, &agents), "");
    assert!(!stdout.contains("refuse:hop-coverage"), "{stdout}");
    let agents_at = stdout.find(&agents).unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    let list_at = stdout
        .find("no expired hop leases under")
        .unwrap_or_else(|| panic!("missing empty expired list\n{stdout}"));
    assert!(agents_at < auth_at && auth_at < list_at, "{stdout}");
    assert!(stdout.contains(&section), "{stdout}");
    assert!(
        stdout.contains("does not show that a worker called the conveyor"),
        "{stdout}"
    );
    assert!(stdout.ends_with(&format!(
        "no expired hop leases under {}\n",
        state.display()
    )));
    assert!(!stdout.contains("forgot "), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("conveyor-leases.json").exists());
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agent_unplaced_refuses_before_the_expired_list() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    let mut ahead = granted_box("notes-append");
    ahead.hops[0].agents = vec!["research".into(), "outsider".into()];
    ahead.leases[0].agents = vec!["research".into(), "outsider".into()];
    conveyor_proxy::persist_mesh(&state, &ahead).unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_expire(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("refuse:agent-unplaced"), "{stderr}");
    assert!(stderr.contains("outsider"), "{stderr}");
    assert!(!stdout.contains("expired hop leases"), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mesh_parse_and_mesh_host_class_refuse_before_the_expired_list() {
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
    let (ok, stdout, stderr) = convey_expire(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(stderr.contains("conveyor-mesh.json"), "{stderr}");
    assert!(!stdout.contains("expired hop leases"), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert_eq!(std::fs::read(&mesh).unwrap(), b"not-json");

    let (ok, stdout, stderr) = convey_expire_forget(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(!stdout.contains("forgot "), "{stdout}");
    assert!(stderr.contains("parse:"), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_eq!(std::fs::read(&mesh).unwrap(), b"not-json");

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[{"id":"cell-one-box","kind":"box","capability":"notes-append","host_class":"rtx-5090","wired":true,"agents":["research"]}],"leases":[{"hop_id":"box","kind":"box","capability":"lane-tool","host_class":"rtx-5090","granted":true,"spawned":false,"durable":true,"driver":"box"}]}"#,
    )
    .unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_expire(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(stderr.contains("rtx-5090"), "{stderr}");
    assert!(!stdout.contains("expired hop leases"), "{stdout}");
    assert_eq!(snapshot(&state), before);

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[{"id":"cell-one-box","kind":"box","capability":"notes-append","host_class":"any","wired":true,"agents":["research"]}],"leases":[]}"#,
    )
    .unwrap();
    std::fs::write(state.join("placement-actual.json"), "{").unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_expire(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(stderr.contains("placement-actual.json"), "{stderr}");
    assert!(!stdout.contains("expired hop leases"), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn placement_sku_prints_agents_and_cites_omits_authority_and_refuses_before_forget() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let (ok, first, stderr) = convey_expire(&estate_path, &state);
    assert!(ok, "{first}\n{stderr}");
    let between = between_agents_and_authority(&first, &agents);
    assert!(
        between.contains("  note  refuse:hop-coverage:") && between.contains("(deny-default)"),
        "{between}"
    );
    assert!(first.contains("no expired hop leases under"), "{first}");

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
    let (ok, stdout, stderr) = convey_expire(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(stderr.contains("rtx-5090"), "{stderr}");
    assert!(stdout.starts_with(&agents), "{stdout}");
    assert_eq!(&stdout[agents.len()..], format!("{between}\n"));
    assert!(!stdout.contains("Authority\n---------"), "{stdout}");
    assert!(!stdout.contains("not-enforced reasons:"), "{stdout}");
    assert!(!stdout.contains("would-allow="), "{stdout}");
    assert!(!stdout.contains("expired hop leases"), "{stdout}");
    assert!(!stdout.contains("forgot "), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stderr.contains("live PASS"), "{stderr}");
    assert_eq!(snapshot(&state), before);

    let (ok, forgot_out, forgot_err) = convey_expire_forget(&estate_path, &state);
    assert!(!ok, "{forgot_out}\n{forgot_err}");
    assert!(forgot_err.contains("refuse:bad-host-class"), "{forgot_err}");
    assert!(forgot_out.starts_with(&agents), "{forgot_out}");
    assert!(!forgot_out.contains("Authority\n---------"), "{forgot_out}");
    assert!(!forgot_out.contains("expired hop leases"), "{forgot_out}");
    assert!(!forgot_out.contains("forgot "), "{forgot_out}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_estate_refuses_before_the_expired_list() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let missing = dir.join("no-such-estate.yaml");
    let (ok, stdout, stderr) = convey_expire(&missing, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("no-such-estate.yaml"), "{stderr}");
    assert!(!state.join("conveyor-mesh.json").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_names_the_stack_and_locks_hold() {
    let (ok, stdout, stderr) = run(&["convey", "expire", "--help"]);
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
        stdout.contains("expired-lease reader still refuses that SKU before the expired list"),
        "{stdout}"
    );
    assert!(stdout.contains("list_expired_hop_leases"), "{stdout}");
    assert!(stdout.contains("reads placement-actual"), "{stdout}");
    assert!(stdout.contains("placement-actual parse"), "{stdout}");
    assert!(stdout.contains("diverges"), "{stdout}");
    assert!(stdout.contains("before `--forget` writes"), "{stdout}");
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
        .find("`estate convey expire` prints")
        .unwrap_or_else(|| panic!("language omits convey expire\n{language}"));
    let mine = &language[at..];
    let end = mine
        .find("Whether a hop lease")
        .unwrap_or_else(|| panic!("language convey expire sentence did not end\n{mine}"));
    let mine = &mine[..end];
    assert!(mine.contains("missing-mesh"), "{mine}");
    assert!(mine.contains("conveyor-mesh.json"), "{mine}");
    assert!(mine.contains("still reads placement-actual"), "{mine}");
    assert!(mine.contains("placement-actual parse failure"), "{mine}");
    assert!(
        mine.contains("expired-lease reader still refuses that SKU before the expired list"),
        "{mine}"
    );
    assert!(mine.contains("list_expired_hop_leases"), "{mine}");
    assert!(mine.contains("diverges"), "{mine}");
    assert!(mine.contains("before `--forget` writes"), "{mine}");
    assert!(!mine.contains("body still prints"), "{mine}");
    assert!(!mine.contains("no second placement refuse"), "{mine}");
    assert!(!mine.contains("do not depend on placement"), "{mine}");
    assert!(!mine.contains("do not read placement"), "{mine}");
    assert!(!mine.contains("ignores placement"), "{mine}");
    assert!(!mine.contains("READY_FOR_LIVE_TEST: yes"), "{mine}");

    let changelog = std::fs::read_to_string(repo_root().join("CHANGELOG.md")).unwrap();
    let head = changelog
        .split("## This slice — estate convey list")
        .next()
        .unwrap();
    assert!(
        head.contains("estate convey expire prints the honesty stack"),
        "{head}"
    );
    assert!(head.contains("render_hop_coverage_cites"), "{head}");
    assert!(head.contains("still reads placement-actual"), "{head}");
    assert!(head.contains("placement-actual parse failure"), "{head}");
    assert!(
        head.contains("expired-lease reader still refuses that SKU before the expired list"),
        "{head}"
    );
    assert!(head.contains("list_expired_hop_leases"), "{head}");
    assert!(head.contains("diverges"), "{head}");
    assert!(head.contains("before `--forget` writes"), "{head}");
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
