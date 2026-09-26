//! `estate models` prints the same Agents, hop-cite, and Authority stack as
//! status, doctor, reconcile, audits, and history, after bindings,
//! readiness, and the per-binding ping lines. Mismatch is `FAIL`. Deny and
//! deny-default are `note`. A match is quiet. None of those cites fail the
//! command. A placement-actual SKU omits Authority. There is no later mesh
//! reader, so that SKU does not refuse after the stack.

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
    let dir = std::env::temp_dir().join(format!("cell-models-honesty-{nanos}"));
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
        && !text.contains("  FAIL  ")
        && !text.contains("  note  ")
}

/// Last ping line of the locked example (and copies that keep its bindings).
fn models_body_end(stdout: &str) -> usize {
    let marker = "  ping local_slm: wired (control will not complete)\n";
    let at = stdout
        .find(marker)
        .unwrap_or_else(|| panic!("missing models ping line\n{stdout}"));
    at + marker.len()
}

fn assert_body_then_stack(stdout: &str) {
    let end = models_body_end(stdout);
    let body = &stdout[..end];
    let bindings = body
        .find("model bindings (equal class")
        .unwrap_or_else(|| panic!("missing bindings\n{body}"));
    let ready = body
        .find("credential readiness (names only):")
        .unwrap_or_else(|| panic!("missing readiness\n{body}"));
    let ping = body
        .find("  ping xai_grok: wired (control will not complete)\n")
        .unwrap_or_else(|| panic!("missing frontier ping\n{body}"));
    assert!(bindings < ready && ready < ping, "{body}");
    assert!(no_stack(body), "{body}");
    assert!(stdout[end..].starts_with("Agents\n------\n"), "{stdout}");
}

fn assert_body_only(stdout: &str) {
    let end = models_body_end(stdout);
    let body = &stdout[..end];
    assert!(body.contains("model bindings (equal class"), "{body}");
    assert!(
        body.contains("credential readiness (names only):"),
        "{body}"
    );
    assert!(stdout[end..].trim().is_empty(), "{stdout}");
    assert!(no_stack(stdout), "{stdout}");
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

fn models(estate: &Path, state: &Path) -> (bool, String, String) {
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    run(&["models", "--estate", &estate_s, "--state-dir", &state_s])
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

fn write_placement(state: &Path, host_class: &str, agents: &[&str]) {
    std::fs::create_dir_all(state).unwrap();
    let agents = agents
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(",");
    std::fs::write(
        state.join("placement-actual.json"),
        format!(
            r#"{{"schema":"cell-one.placement-actual.v0","desired_hash":"abc","leases":[{{"placement_id":"cell-one-box","kind":"box","host_class":"{host_class}","spawned":false,"wired":true,"agents":[{agents}]}}]}}"#
        ),
    )
    .unwrap();
}

#[test]
fn stack_follows_the_models_body_and_cites_do_not_fail() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(section.contains("would-allow="), "{section}");
    assert!(section.contains("would-deny="), "{section}");
    assert!(section.contains("not-enforced"), "{section}");
    assert!(section.contains("not-enforced reasons:"), "{section}");
    assert!(no_enforced_status_token(&section), "{section}");
    let before = snapshot(&state);
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = models(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_body_then_stack(&stdout);
    let tail = &stdout[models_body_end(&stdout)..];
    assert_eq!(tail, authority_out);
    let cites = between_agents_and_authority(&stdout, &agents);
    assert!(
        cites.contains("  note  refuse:hop-coverage:")
            && cites.contains("(deny-default)")
            && !cites.contains("(mismatch)")
            && !cites.contains("  FAIL  "),
        "{cites}"
    );
    assert!(stdout.contains(&section), "{stdout}");
    assert!(
        stdout.contains("does not show that a worker called the conveyor"),
        "{stdout}"
    );
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("reconcile.json").exists());
    assert!(!state.join("apply-audit.jsonl").exists());
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hop_mismatch_cite_does_not_fail_models() {
    let dir = scratch();
    let estate_path = dir.join("estate.yaml");
    let mut text = estate_with_lane_tool("allow");
    text = text.replace(
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n",
    );
    std::fs::write(&estate_path, text).unwrap();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let before = snapshot(&state);
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = models(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_body_then_stack(&stdout);
    assert_eq!(&stdout[models_body_end(&stdout)..], authority_out);
    let cites = between_agents_and_authority(&stdout, &agents);
    assert!(
        cites.contains("  FAIL  refuse:hop-coverage:") && cites.contains("(mismatch)"),
        "{cites}"
    );
    let agents_at = stdout.find(&agents).unwrap();
    let cite_at = stdout.find("  FAIL  refuse:hop-coverage:").unwrap();
    let auth_at = stdout.find("Authority\n---------\n").unwrap();
    assert!(
        models_body_end(&stdout) <= agents_at && agents_at < cite_at && cite_at < auth_at,
        "{stdout}"
    );
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn allow_match_stays_quiet() {
    let dir = scratch();
    let estate_path = dir.join("estate.yaml");
    std::fs::write(&estate_path, estate_with_lane_tool("allow")).unwrap();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    conveyor_proxy::persist_mesh(&state, &granted_box("lane-tool")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let before = snapshot(&state);
    let (ok, stdout, stderr) = models(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_body_then_stack(&stdout);
    assert_eq!(between_agents_and_authority(&stdout, &agents), "");
    assert!(!stdout.contains("refuse:hop-coverage"), "{stdout}");
    assert!(!stdout.contains("  FAIL  "), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_eq!(snapshot(&state), before);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_mesh_is_not_enforced_and_invents_no_cites() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    assert!(!state.join("conveyor-mesh.json").exists());
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
    assert!(section.contains("would-allow="), "{section}");
    assert!(section.contains("would-deny="), "{section}");
    let before = snapshot(&state);
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    let (ok, stdout, stderr) = models(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_body_then_stack(&stdout);
    assert_eq!(&stdout[models_body_end(&stdout)..], authority_out);
    assert_eq!(between_agents_and_authority(&stdout, &agents), "");
    assert!(!stdout.contains("refuse:hop-coverage"), "{stdout}");
    assert!(stdout.contains(&section), "{stdout}");
    assert!(
        stdout.contains("does not show that a worker called the conveyor"),
        "{stdout}"
    );
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("conveyor-leases.json").exists());
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn default_state_dir_is_dot_cell_and_does_not_create_it() {
    let dir = scratch();
    let estate_path = repo_root().join("examples/estate.yaml");
    let estate_s = estate_path.display().to_string();
    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args(["models", "--estate", &estate_s])
        .current_dir(&dir)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_body_then_stack(&stdout);
    assert!(
        stdout.contains("not-enforced reasons: missing-mesh="),
        "{stdout}"
    );
    assert!(stdout.contains("conveyor-mesh.json is absent"), "{stdout}");
    assert!(!dir.join(".cell").exists());
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agent_unplaced_refuses_before_the_stack() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    write_placement(&state, "any", &["research"]);
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
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(!authority_ok, "{authority_out}\n{authority_err}");
    assert!(authority_out.trim().is_empty(), "{authority_out}");
    let (ok, stdout, stderr) = models(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert_body_only(&stdout);
    assert!(no_stack(&stderr), "{stderr}");
    assert_eq!(stderr, authority_err);
    assert!(stderr.contains("refuse:agent-unplaced"), "{stderr}");
    assert!(stderr.contains("outsider"), "{stderr}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mesh_parse_and_mesh_host_class_refuse_before_the_stack() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    write_placement(&state, "any", &["research"]);
    let mesh = state.join("conveyor-mesh.json");
    std::fs::write(&mesh, "not-json").unwrap();
    let before = snapshot(&state);
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(!authority_ok, "{authority_out}");
    assert!(authority_out.trim().is_empty(), "{authority_out}");
    let (ok, stdout, stderr) = models(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert_body_only(&stdout);
    assert!(no_stack(&stderr), "{stderr}");
    assert_eq!(stderr, authority_err);
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(stderr.contains("conveyor-mesh.json"), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_eq!(std::fs::read(&mesh).unwrap(), b"not-json");

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[],"leases":[{"hop_id":"box","kind":"box","capability":"lane-tool","host_class":"rtx-5090","granted":true,"spawned":false,"durable":true,"driver":"box"}]}"#,
    )
    .unwrap();
    let before = snapshot(&state);
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(!authority_ok, "{authority_out}");
    let (ok, stdout, stderr) = models(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert_body_only(&stdout);
    assert!(no_stack(&stderr), "{stderr}");
    assert_eq!(stderr, authority_err);
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
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(!authority_ok, "{authority_out}");
    let (ok, stdout, stderr) = models(&estate_path, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert_body_only(&stdout);
    assert!(no_stack(&stdout), "{stdout}");
    assert_eq!(stderr, authority_err);
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
    write_placement(&state, "any", &["horizon", "research", "sanctum"]);
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let (ok, first, stderr) = models(&estate_path, &state);
    assert!(ok, "{first}\n{stderr}");
    let between = between_agents_and_authority(&first, &agents);
    assert!(
        between.contains("  note  refuse:hop-coverage:") && between.contains("(deny-default)"),
        "{between}"
    );
    assert!(first.contains("Authority\n---------\n"), "{first}");

    write_placement(&state, "rtx-5090", &["horizon", "research", "sanctum"]);
    let before = snapshot(&state);
    let (authority_ok, authority_out, authority_err) = convey_authority(&estate_path, &state);
    assert!(authority_ok, "{authority_out}\n{authority_err}");
    assert!(authority_err.is_empty(), "{authority_err}");
    assert!(
        !authority_out.contains("Authority\n---------"),
        "{authority_out}"
    );
    let (ok, stdout, stderr) = models(&estate_path, &state);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_body_then_stack(&stdout);
    assert_eq!(&stdout[models_body_end(&stdout)..], authority_out);
    assert!(stdout.contains(&agents), "{stdout}");
    assert!(
        stdout.contains("  note  refuse:hop-coverage:") && stdout.contains("(deny-default)"),
        "{stdout}"
    );
    assert!(!stdout.contains("Authority\n---------"), "{stdout}");
    assert!(!stdout.contains("not-enforced reasons:"), "{stdout}");
    assert!(!stdout.contains("would-allow="), "{stdout}");
    assert!(!stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_or_unreadable_estate_refuses_before_the_models_body() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let missing = dir.join("no-such.yaml");
    let (ok, stdout, stderr) = models(&missing, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(!stdout.contains("model bindings"), "{stdout}");
    assert!(stderr.contains("load "), "{stderr}");
    assert!(stderr.contains(&missing.display().to_string()), "{stderr}");
    assert!(no_stack(&stderr), "{stderr}");

    let unread = dir.join("estate-dir");
    std::fs::create_dir_all(&unread).unwrap();
    let (ok, stdout, stderr) = models(&unread, &state);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(!stdout.contains("model bindings"), "{stdout}");
    assert!(no_stack(&stdout), "{stdout}");
    assert!(stderr.contains("load "), "{stderr}");
    assert!(no_stack(&stderr), "{stderr}");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_names_the_stack_and_locks_hold() {
    let (ok, stdout, stderr) = run(&["models", "--help"]);
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
    assert!(
        stdout.contains("After bindings, readiness, and per-binding ping lines"),
        "{stdout}"
    );
    assert!(stdout.contains("would-allow"), "{stdout}");
    assert!(stdout.contains("would-deny"), "{stdout}");
    assert!(stdout.contains("not-enforced"), "{stdout}");
    assert!(stdout.contains("not-enforced reasons:"), "{stdout}");
    assert!(stdout.contains("missing-mesh"), "{stdout}");
    assert!(stdout.contains("do not fail"), "{stdout}");
    assert!(stdout.contains("empty cite list"), "{stdout}");
    assert!(stdout.contains("refuse:agent-unplaced"), "{stdout}");
    assert!(stdout.contains("placement-actual parse"), "{stdout}");
    assert!(
        stdout.contains("The models body is already printed"),
        "{stdout}"
    );
    assert!(stdout.contains("Authority rows are omitted"), "{stdout}");
    assert!(stdout.contains("no later mesh reader"), "{stdout}");
    assert!(
        stdout.contains("does not refuse after the stack"),
        "{stdout}"
    );
    assert!(
        stdout.contains("refuses before the models body"),
        "{stdout}"
    );
    assert!(stdout.contains("--state-dir"), "{stdout}");
    assert!(stdout.contains(".cell"), "{stdout}");
    assert!(stdout.contains("examples/estate.yaml"), "{stdout}");
    assert!(stdout.contains("Does not spawn"), "{stdout}");
    assert!(stdout.contains("Does not claim mediation"), "{stdout}");
    assert!(stdout.contains("estate help models"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!stdout.contains("body still prints"), "{stdout}");
    assert!(!stdout.contains("no second placement refuse"), "{stdout}");

    let (help_ok, help_out, help_err) = run(&["help", "models"]);
    assert!(help_ok, "{help_out}\n{help_err}");
    assert!(
        help_out.contains("After bindings, readiness, and those ping lines"),
        "{help_out}"
    );
    assert!(help_out.contains("honesty_stack"), "{help_out}");
    assert!(help_out.contains("describe_agents_section"), "{help_out}");
    assert!(help_out.contains("render_hop_coverage_cites"), "{help_out}");
    assert!(help_out.contains("hop_coverage_cites"), "{help_out}");
    assert!(
        help_out.contains("describe_authority_section"),
        "{help_out}"
    );
    assert!(help_out.contains("would-allow"), "{help_out}");
    assert!(help_out.contains("would-deny"), "{help_out}");
    assert!(help_out.contains("not-enforced"), "{help_out}");
    assert!(help_out.contains("missing-mesh"), "{help_out}");
    assert!(help_out.contains("empty cite list"), "{help_out}");
    assert!(help_out.contains("refuse:agent-unplaced"), "{help_out}");
    assert!(help_out.contains("placement-actual parse"), "{help_out}");
    assert!(
        help_out.contains("The models body is already printed"),
        "{help_out}"
    );
    assert!(
        help_out.contains("Authority rows are omitted"),
        "{help_out}"
    );
    assert!(help_out.contains("no later mesh reader"), "{help_out}");
    assert!(
        help_out.contains("does not refuse after the stack"),
        "{help_out}"
    );
    assert!(
        help_out.contains("refuses before the models body"),
        "{help_out}"
    );
    assert!(help_out.contains("--state-dir .cell"), "{help_out}");
    assert!(help_out.contains("Does not spawn"), "{help_out}");
    assert!(help_out.contains("Does not claim mediation"), "{help_out}");
    assert!(!help_out.contains("READY_FOR_LIVE_TEST: yes"), "{help_out}");
    assert!(!help_out.contains("body still prints"), "{help_out}");
    assert!(
        !help_out.contains("no second placement refuse"),
        "{help_out}"
    );

    let (index_ok, index_out, index_err) = run(&["help"]);
    assert!(index_ok, "{index_out}\n{index_err}");
    assert!(index_out.contains("estate help models"), "{index_out}");

    let language =
        std::fs::read_to_string(repo_root().join("docs/UBIQUITOUS_LANGUAGE.md")).unwrap();
    let at = language
        .find("`estate models` prints")
        .unwrap_or_else(|| panic!("language omits models\n{language}"));
    let mine = &language[at..];
    let end = mine
        .find("\n\n### control")
        .unwrap_or_else(|| panic!("language models sentence did not end\n{mine}"));
    let mine = &mine[..end];
    assert!(mine.contains("honesty_stack"), "{mine}");
    assert!(mine.contains("missing-mesh"), "{mine}");
    assert!(mine.contains("conveyor-mesh.json"), "{mine}");
    assert!(mine.contains("still reads placement-actual"), "{mine}");
    assert!(mine.contains("placement-actual parse failure"), "{mine}");
    assert!(mine.contains("refuse:agent-unplaced"), "{mine}");
    assert!(mine.contains("MeshError::BadHostClass"), "{mine}");
    assert!(
        mine.contains("The models body is already printed"),
        "{mine}"
    );
    assert!(mine.contains("Authority rows are omitted"), "{mine}");
    assert!(mine.contains("no later mesh reader"), "{mine}");
    assert!(mine.contains("does not refuse after the stack"), "{mine}");
    assert!(mine.contains("before the models body"), "{mine}");
    assert!(mine.contains("bindings, readiness"), "{mine}");
    assert!(!mine.contains("body still prints"), "{mine}");
    assert!(!mine.contains("no second placement refuse"), "{mine}");
    assert!(!mine.contains("do not depend on placement"), "{mine}");
    assert!(!mine.contains("do not read placement"), "{mine}");
    assert!(!mine.contains("READY_FOR_LIVE_TEST: yes"), "{mine}");

    let changelog = std::fs::read_to_string(repo_root().join("CHANGELOG.md")).unwrap();
    let head = changelog
        .split("## This slice — estate status")
        .next()
        .unwrap();
    assert!(
        head.contains("estate models prints the honesty stack"),
        "{head}"
    );
    assert!(head.contains("honesty_stack"), "{head}");
    assert!(head.contains("render_hop_coverage_cites"), "{head}");
    assert!(head.contains("missing-mesh"), "{head}");
    assert!(head.contains("MeshError::BadHostClass"), "{head}");
    assert!(head.contains("still reads placement-actual"), "{head}");
    assert!(head.contains("placement-actual parse failure"), "{head}");
    assert!(head.contains("bindings, readiness"), "{head}");
    assert!(
        head.contains("The models body is already printed"),
        "{head}"
    );
    assert!(head.contains("Authority rows are omitted"), "{head}");
    assert!(head.contains("no later mesh reader"), "{head}");
    assert!(head.contains("does not refuse after the stack"), "{head}");
    assert!(head.contains("before the models body"), "{head}");
    assert!(!head.contains("body still prints"), "{head}");
    assert!(!head.contains("no second placement refuse"), "{head}");
    assert!(!head.contains("this file check succeeds"), "{head}");
    assert!(!head.contains("reader still refuses"), "{head}");
    assert!(!head.contains("list_expired_hop_leases"), "{head}");
    assert!(head.contains("43770130 3391"), "{head}");
    assert!(head.contains("`READY_FOR_LIVE_TEST`: no"), "{head}");
    assert!(!head.contains("READY_FOR_LIVE_TEST: yes"), "{head}");
    assert!(head.contains("does not invent a live PASS"), "{head}");
    assert_locked_cksum();
}
