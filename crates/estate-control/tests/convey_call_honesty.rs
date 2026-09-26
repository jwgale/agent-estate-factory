//! `estate convey call` prints the same Agents, hop-cite, and Authority stack
//! as status, doctor, reconcile, leases, convey leases, convey list, convey
//! expire, convey sync, and convey hop, after the estate loads and before
//! `call_hop` / `call_hop_for_agent` and before the call JSON.
//! Mismatch is `FAIL`. Deny and deny-default are `note`. A match is quiet.
//! None of those cites fail the command. Intention, hop-coverage, missing
//! estate, and `--kind` without `--agent` refuse before the stack. A
//! placement-actual SKU omits Authority. `call_hop` then still refuses that
//! SKU before an allow and before any restamp.

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
    let dir = std::env::temp_dir().join(format!("cell-convey-call-honesty-{nanos}"));
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
        && !text.contains("\"hop_id\"")
}

fn no_call_json(text: &str) -> bool {
    !text.contains("\"hop_id\"")
        && !text.contains("lease-bound box hop")
        && !text.contains("lease-refresh")
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

fn ttl_decl(agents: Vec<String>) -> conveyor_proxy::HopDecl {
    conveyor_proxy::HopDecl {
        id: "ttl-hop".into(),
        kind: "box".into(),
        capability: "lane-tool".into(),
        host_class: "any".into(),
        wired: true,
        note: None,
        ttl_secs: None,
        agents,
    }
}

fn ttl_lease(agents: Vec<String>) -> conveyor_proxy::HopLease {
    conveyor_proxy::HopLease {
        hop_id: "ttl-hop".into(),
        kind: "box".into(),
        capability: "lane-tool".into(),
        host_class: "any".into(),
        granted: true,
        spawned: true,
        durable: true,
        driver: "box".into(),
        note: None,
        ttl_secs: None,
        issued_at: None,
        expires_at: None,
        agents,
    }
}

fn mesh_with_stub(capability: &str) -> conveyor_proxy::ConveyorMesh {
    let mut mesh = granted_box(capability);
    mesh.hops.push(ttl_decl(vec![]));
    mesh.leases.push(ttl_lease(vec![]));
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

fn convey_call(estate: &Path, state: &Path, extra: &[&str]) -> (bool, String, String) {
    let policy = repo_root().join("policy/cell-one.policy.v0.yaml");
    let mut args = vec![
        "convey".to_string(),
        "call".to_string(),
        "--estate".to_string(),
        estate.display().to_string(),
        "--state-dir".to_string(),
        state.display().to_string(),
        "--policy".to_string(),
        policy.display().to_string(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_string()));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run(&refs)
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

fn lease_count(mesh_body: &str, hop_id: &str) -> usize {
    let mesh: serde_json::Value = serde_json::from_str(mesh_body).unwrap();
    mesh["leases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|lease| lease["hop_id"] == hop_id)
        .count()
}

fn assert_stack_then_call(stdout: &str, agents: &str, authority_out: &str) -> serde_json::Value {
    assert!(stdout.starts_with(authority_out), "{stdout}");
    assert_eq!(stdout.matches("Agents\n------\n").count(), 1, "{stdout}");
    assert_eq!(
        stdout.matches("Authority\n---------\n").count(),
        1,
        "{stdout}"
    );
    let agents_at = stdout.find(agents).unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    let json_at = stdout
        .find("\"hop_id\"")
        .unwrap_or_else(|| panic!("missing call JSON\n{stdout}"));
    assert!(agents_at < auth_at && auth_at < json_at, "{stdout}");
    let body = stdout.strip_prefix(authority_out).unwrap();
    let call: serde_json::Value = serde_json::from_str(body.trim()).unwrap();
    assert_eq!(stdout, format!("{authority_out}{body}"), "{stdout}");
    assert!(stdout.contains("would-allow="), "{stdout}");
    assert!(stdout.contains("would-deny="), "{stdout}");
    assert!(stdout.contains("not-enforced="), "{stdout}");
    assert!(
        stdout.contains("does not show that a worker called the conveyor"),
        "{stdout}"
    );
    assert!(no_enforced_status_token(stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    call
}

#[test]
fn convey_call_prints_agents_cites_and_authority_before_the_call_json() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &mesh_with_stub("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let before = snapshot(&state);
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = convey_call(
        &estate_path,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
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
    let call = assert_stack_then_call(&stdout, &agents, &authority_out);
    assert_eq!(call["hop_id"], "ttl-hop");
    assert_eq!(call["capability"], "lane-tool");
    assert_eq!(call["allow"], true);
    assert_eq!(call["reason"], "lease-bound box hop");
    let mesh_body = std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap();
    assert_eq!(box_capability(&mesh_body), "notes-append", "{mesh_body}");
    assert_eq!(lease_count(&mesh_body, "ttl-hop"), 1, "{mesh_body}");
    assert_eq!(snapshot(&state), before, "call must not rewrite the mesh");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hop_mismatch_cite_does_not_fail_convey_call() {
    let mut text = estate_with_lane_tool("allow");
    text = text.replace(
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n",
    );
    let root = doctor_root(&text);
    let estate_path = root.join("examples/estate.yaml");
    let state = root.join("state");
    apply(&estate_path, &state, &root);
    conveyor_proxy::persist_mesh(&state, &mesh_with_stub("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let before = snapshot(&state);
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = convey_call(
        &estate_path,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
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
    let json_at = stdout.find("\"hop_id\"").unwrap();
    assert!(
        agents_at < cite_at && cite_at < auth_at && auth_at < json_at,
        "{stdout}"
    );
    let call = assert_stack_then_call(&stdout, &agents, &authority_out);
    assert_eq!(call["allow"], true);
    assert_eq!(call["reason"], "lease-bound box hop");
    let mesh_body = std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap();
    assert_eq!(box_capability(&mesh_body), "notes-append", "{mesh_body}");
    assert_eq!(snapshot(&state), before);
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn coverage_intention_and_agent_unbound_refuse_before_the_stack() {
    let mut text = estate_with_lane_tool("allow");
    text = text.replace(
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n",
    );
    let root = doctor_root(&text);
    let estate_path = root.join("examples/estate.yaml");
    let state = root.join("state");
    apply(&estate_path, &state, &root);
    assert!(!state.join("conveyor-mesh.json").exists());
    let placement = std::fs::read(state.join("placement-actual.json")).unwrap();
    let (ok, stdout, stderr) = convey_call(
        &estate_path,
        &state,
        &[
            "--id",
            "cell-one-box",
            "--capability",
            "notes-append",
            "--agent",
            "research",
        ],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(no_stack(&stderr), "{stderr}");
    assert!(no_call_json(&stdout), "{stdout}");
    assert!(
        stderr.contains("refuse:hop-coverage")
            && stderr.contains("(mismatch)")
            && stderr.contains("notes-append")
            && stderr.contains("lane-tool"),
        "{stderr}"
    );
    assert!(!state.join("conveyor-mesh.json").exists());
    assert_eq!(
        std::fs::read(state.join("placement-actual.json")).unwrap(),
        placement
    );

    let locked = repo_root().join("examples/estate.yaml");
    let intend = root.join("intend");
    apply(&locked, &intend, &root);
    let (ok, stdout, stderr) = convey_call(
        &locked,
        &intend,
        &[
            "--id",
            "notes-hop",
            "--capability",
            "notes-append",
            "--agent",
            "research",
        ],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(no_stack(&stderr), "{stderr}");
    assert!(no_call_json(&stdout), "{stdout}");
    assert!(
        stderr.contains("refuse:intention") && stderr.contains("(deny-default)"),
        "{stderr}"
    );
    assert!(!intend.join("conveyor-mesh.json").exists());

    let unbound = root.join("unbound");
    std::fs::create_dir_all(&unbound).unwrap();
    let (ok, stdout, stderr) = convey_call(
        &locked,
        &unbound,
        &[
            "--id",
            "ttl-hop",
            "--capability",
            "lane-tool",
            "--kind",
            "tool",
        ],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(no_stack(&stderr), "{stderr}");
    assert!(no_call_json(&stdout), "{stdout}");
    assert!(
        stderr.contains("refuse:agent-unbound") && stderr.contains("--kind"),
        "{stderr}"
    );
    assert!(!unbound.join("conveyor-mesh.json").exists());

    let missing = root.join("no-such-estate.yaml");
    let missing_state = root.join("missing-state");
    std::fs::create_dir_all(&missing_state).unwrap();
    let (ok, stdout, stderr) = convey_call(
        &missing,
        &missing_state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(no_stack(&stderr), "{stderr}");
    assert!(no_call_json(&stdout), "{stdout}");
    assert!(
        stderr.contains("refuse:hop-coverage") && stderr.contains("estate missing"),
        "{stderr}"
    );
    assert!(!missing_state.join("conveyor-mesh.json").exists());

    let cloud = root.join("cloud");
    apply(&locked, &cloud, &root);
    patch_lease(&cloud, "cursor-cloud", |lease| {
        lease["spawned"] = serde_json::Value::Bool(true);
    });
    let placement = std::fs::read(cloud.join("placement-actual.json")).unwrap();
    let (ok, stdout, stderr) = convey_call(
        &locked,
        &cloud,
        &["--id", "cursor-cloud", "--capability", "mesh-stub"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(no_stack(&stderr), "{stderr}");
    assert!(no_call_json(&stdout), "{stdout}");
    assert!(
        stderr.contains("refuse:hop-coverage") && stderr.contains("(deny)"),
        "{stderr}"
    );
    assert!(!stderr.contains("refuse:cloud-spawned"), "{stderr}");
    assert!(!cloud.join("conveyor-mesh.json").exists());
    assert_eq!(
        std::fs::read(cloud.join("placement-actual.json")).unwrap(),
        placement
    );
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn missing_mesh_is_not_enforced_then_call_refuses_without_inventing_one() {
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
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = convey_call(
        &estate_path,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert_eq!(stdout, authority_out, "{stdout}");
    assert_eq!(between_agents_and_authority(&stdout, &agents), "");
    assert!(stdout.contains(&section), "{stdout}");
    assert!(
        stderr.contains("refuse:no-lease") && stderr.contains("ttl-hop"),
        "{stderr}"
    );
    assert!(no_call_json(&stdout), "{stdout}");
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("conveyor-hops.json").exists());
    assert!(!state.join("conveyor-leases.json").exists());
    assert_eq!(snapshot(&state), before);
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_lease_restamps_once_after_the_stack() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    let mut mesh = granted_box("notes-append");
    mesh.hops.push(ttl_decl(vec![]));
    conveyor_proxy::persist_mesh(&state, &mesh).unwrap();
    assert_eq!(
        lease_count(
            &std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap(),
            "ttl-hop"
        ),
        0
    );
    let placement = std::fs::read(state.join("placement-actual.json")).unwrap();
    let audit = std::fs::read(state.join("apply-audit.jsonl")).unwrap();
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = convey_call(
        &estate_path,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let call = assert_stack_then_call(&stdout, &agents, &authority_out);
    assert_eq!(call["allow"], true);
    assert_eq!(call["hop_id"], "ttl-hop");
    let reason = call["reason"].as_str().unwrap();
    assert!(
        reason.contains("lease-refresh") && reason.contains("lease-bound box hop"),
        "{reason}"
    );
    let mesh_body = std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap();
    assert_eq!(lease_count(&mesh_body, "ttl-hop"), 1, "{mesh_body}");
    assert_eq!(box_capability(&mesh_body), "notes-append", "{mesh_body}");
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
fn populated_lease_agent_unbound_prints_the_stack_and_skips_the_call_json() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    let mut mesh = granted_box("notes-append");
    mesh.hops.push(ttl_decl(vec!["research".into()]));
    mesh.leases.push(ttl_lease(vec!["research".into()]));
    conveyor_proxy::persist_mesh(&state, &mesh).unwrap();
    let before = snapshot(&state);
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = convey_call(
        &estate_path,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert_eq!(stdout, authority_out, "{stdout}");
    assert!(
        stderr.contains("refuse:agent-unbound") && stderr.contains("(unnamed)"),
        "{stderr}"
    );
    assert!(no_call_json(&stdout), "{stdout}");
    assert!(stdout.contains("Agents\n------"), "{stdout}");
    assert!(stdout.contains("Authority\n---------"), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agent_unplaced_on_the_mesh_refuses_before_the_stack() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    let mut ahead = mesh_with_stub("notes-append");
    ahead.hops[0].agents = vec!["research".into(), "outsider".into()];
    ahead.leases[0].agents = vec!["research".into(), "outsider".into()];
    conveyor_proxy::persist_mesh(&state, &ahead).unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_call(
        &estate_path,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(no_stack(&stderr), "{stderr}");
    assert!(no_call_json(&stdout), "{stdout}");
    assert!(stderr.contains("refuse:agent-unplaced"), "{stderr}");
    assert!(stderr.contains("outsider"), "{stderr}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mesh_parse_and_mesh_host_class_refuse_before_the_call() {
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
    let (ok, stdout, stderr) = convey_call(
        &estate_path,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(no_stack(&stderr), "{stderr}");
    assert!(no_call_json(&stdout), "{stdout}");
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(stderr.contains("conveyor-mesh.json"), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_eq!(std::fs::read(&mesh).unwrap(), b"not-json");

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[{"id":"cell-one-box","kind":"box","capability":"notes-append","host_class":"rtx-5090","wired":true,"agents":["research"]}],"leases":[{"hop_id":"cell-one-box","kind":"box","capability":"lane-tool","host_class":"rtx-5090","granted":true,"spawned":false,"durable":true,"driver":"box"}]}"#,
    )
    .unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_call(
        &estate_path,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(no_stack(&stderr), "{stderr}");
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(stderr.contains("rtx-5090"), "{stderr}");
    assert_eq!(snapshot(&state), before);

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[{"id":"ttl-hop","kind":"box","capability":"lane-tool","host_class":"any","wired":true,"agents":[]}],"leases":[{"hop_id":"ttl-hop","kind":"box","capability":"lane-tool","host_class":"any","granted":true,"spawned":true,"durable":true,"driver":"box","agents":[]}]}"#,
    )
    .unwrap();
    std::fs::write(state.join("placement-actual.json"), "{").unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = convey_call(
        &estate_path,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(no_stack(&stderr), "{stderr}");
    assert!(no_call_json(&stdout), "{stdout}");
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
    conveyor_proxy::persist_mesh(&state, &mesh_with_stub("notes-append")).unwrap();
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
    let (ok, stdout, stderr) = convey_call(
        &estate_path,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(stderr.contains("rtx-5090"), "{stderr}");
    assert!(stdout.starts_with(&agents), "{stdout}");
    assert_eq!(&stdout[agents.len()..], format!("{between}\n"));
    assert!(!stdout.contains("Authority\n---------"), "{stdout}");
    assert!(!stdout.contains("not-enforced reasons:"), "{stdout}");
    assert!(!stdout.contains("would-allow="), "{stdout}");
    assert!(no_call_json(&stdout), "{stdout}");
    assert!(!stdout.contains("\"allow\""), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stderr.contains("live PASS"), "{stderr}");
    assert_eq!(snapshot(&state), before);
    let mesh_body = std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap();
    assert_eq!(box_capability(&mesh_body), "notes-append", "{mesh_body}");
    assert_eq!(lease_count(&mesh_body, "ttl-hop"), 1, "{mesh_body}");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_names_the_stack_and_locks_hold() {
    let (ok, stdout, stderr) = run(&["convey", "call", "--help"]);
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
        stdout.contains("still refuse that SKU before an allow"),
        "{stdout}"
    );
    assert!(stdout.contains("load_interpreted_mesh"), "{stdout}");
    assert!(stdout.contains("restamp_hop_from_decl"), "{stdout}");
    assert!(stdout.contains("reads placement-actual"), "{stdout}");
    assert!(stdout.contains("placement-actual parse"), "{stdout}");
    assert!(stdout.contains("diverges"), "{stdout}");
    assert!(stdout.contains("before the stack"), "{stdout}");
    assert!(stdout.contains("refuse:no-lease"), "{stdout}");
    assert!(stdout.contains("does not invent a mesh"), "{stdout}");
    assert!(
        stdout.contains("does not add a second mesh write"),
        "{stdout}"
    );
    assert!(stdout.contains("does not print the call JSON"), "{stdout}");
    assert!(stdout.contains("examples/estate.yaml"), "{stdout}");
    assert!(stdout.contains("Does not spawn"), "{stdout}");
    assert!(stdout.contains("Does not apply"), "{stdout}");
    assert!(stdout.contains("Does not claim mediation"), "{stdout}");
    assert!(!stdout.contains("body still prints"), "{stdout}");
    assert!(!stdout.contains("no second placement refuse"), "{stdout}");
    assert!(!stdout.contains("do not depend on placement"), "{stdout}");
    assert!(!stdout.contains("do not read placement"), "{stdout}");
    assert!(!stdout.contains("ignores placement"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");

    let language =
        std::fs::read_to_string(repo_root().join("docs/UBIQUITOUS_LANGUAGE.md")).unwrap();
    let at = language
        .find("`estate convey call` prints")
        .unwrap_or_else(|| panic!("language omits convey call\n{language}"));
    let mine = &language[at..];
    let end = mine
        .find("Whether a hop lease")
        .unwrap_or_else(|| panic!("language convey call sentence did not end\n{mine}"));
    let mine = &mine[..end];
    assert!(mine.contains("missing-mesh"), "{mine}");
    assert!(mine.contains("conveyor-mesh.json"), "{mine}");
    assert!(mine.contains("still reads placement-actual"), "{mine}");
    assert!(mine.contains("placement-actual parse failure"), "{mine}");
    assert!(
        mine.contains("still refuse that SKU before an allow"),
        "{mine}"
    );
    assert!(mine.contains("load_interpreted_mesh"), "{mine}");
    assert!(mine.contains("restamp_hop_from_decl"), "{mine}");
    assert!(mine.contains("diverges"), "{mine}");
    assert!(mine.contains("before the stack"), "{mine}");
    assert!(mine.contains("refuse:no-lease"), "{mine}");
    assert!(mine.contains("does not invent a mesh"), "{mine}");
    assert!(mine.contains("does not add a second mesh write"), "{mine}");
    assert!(mine.contains("does not print the call JSON"), "{mine}");
    assert!(!mine.contains("body still prints"), "{mine}");
    assert!(!mine.contains("no second placement refuse"), "{mine}");
    assert!(!mine.contains("do not depend on placement"), "{mine}");
    assert!(!mine.contains("do not read placement"), "{mine}");
    assert!(!mine.contains("ignores placement"), "{mine}");
    assert!(!mine.contains("READY_FOR_LIVE_TEST: yes"), "{mine}");
    assert_locked_cksum();
}
