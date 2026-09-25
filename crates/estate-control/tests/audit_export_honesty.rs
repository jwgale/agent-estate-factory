//! `estate audit export` writes the same Agents, hop-cite, and Authority
//! stack as reconcile, status, doctor, and convey authority into `honesty.md`
//! next to `MANIFEST.md`. Mismatch is `FAIL`. Deny and deny-default are
//! `note`. A match is quiet. None of those cites fail the export.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static N: AtomicU64 = AtomicU64::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch() -> PathBuf {
    let n = N.fetch_add(1, Ordering::SeqCst);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("cell-audit-honesty-{n}-{nanos}"));
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

fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut rows = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
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
        && !text.contains("honesty.md")
        && !text.contains("refuse:hop-coverage")
        && !text.contains("Cell One audit export")
}

/// Text between the Agents section and the Authority header.
fn between_agents_and_authority<'a>(text: &'a str, agents: &str) -> &'a str {
    let at = text
        .find(agents)
        .unwrap_or_else(|| panic!("missing Agents section\n{text}"));
    let after = &text[at + agents.len()..];
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

fn write_estate(dir: &Path, text: &str) -> PathBuf {
    let path = dir.join("estate.yaml");
    std::fs::write(&path, text).unwrap();
    path
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

fn export(
    scratch_root: &Path,
    estate: &Path,
    state: &Path,
    out: &Path,
    tar: bool,
) -> (bool, String, String) {
    let plans = scratch_root.join("plans");
    let packs = scratch_root.join("packs");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::create_dir_all(&packs).unwrap();
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let packs_s = packs.display().to_string();
    let out_s = out.display().to_string();
    let mut args = vec![
        "audit",
        "export",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--plans-dir",
        &plans_s,
        "--packs-dir",
        &packs_s,
        "--out",
        &out_s,
    ];
    if tar {
        args.push("--tar");
    }
    run(&args)
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

fn assert_stack_order(text: &str, agents: &str, section: &str) {
    assert_eq!(text.matches("Agents\n------\n").count(), 1, "{text}");
    assert_eq!(text.matches("Authority\n---------\n").count(), 1, "{text}");
    let agents_at = text.find(agents).unwrap();
    let auth_at = text.find("Authority\n---------\n").unwrap();
    assert!(agents_at < auth_at, "{text}");
    assert!(text.contains(section), "{text}");
    assert!(no_enforced_status_token(text), "{text}");
    let cite = text
        .find("  note  refuse:hop-coverage:")
        .or_else(|| text.find("  FAIL  refuse:hop-coverage:"));
    if let Some(cite_at) = cite {
        assert!(agents_at < cite_at && cite_at < auth_at, "{text}");
    }
}

fn manifest_parts(manifest: &str) -> (&str, &str) {
    let mut split = manifest.split("Missing (ok if never applied)");
    let copied = split.next().unwrap();
    let missing = split
        .next()
        .unwrap_or_else(|| panic!("manifest has no Missing section\n{manifest}"));
    (copied, missing)
}

#[test]
fn export_writes_deny_default_stack_and_copies_the_mesh() {
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
    assert!(section.contains("(deny-default)"), "{section}");
    assert!(no_enforced_status_token(&section), "{section}");
    let kept = kept_files(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let (convey_ok, convey_out, convey_err) = convey(&estate_path, &state);
    assert!(convey_ok, "{convey_out}\n{convey_err}");
    assert!(convey_err.is_empty(), "{convey_err}");
    let cites = between_agents_and_authority(&convey_out, &agents);
    assert!(
        cites.contains("  note  refuse:hop-coverage:")
            && cites.contains("(deny-default)")
            && !cites.contains("(mismatch)")
            && !cites.contains("  FAIL  "),
        "{cites}"
    );
    let out = dir.join("audit-export");
    let (ok, stdout, stderr) = export(&dir, &estate_path, &state, &out, true);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("honesty: honesty.md"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    let honesty = std::fs::read_to_string(out.join("honesty.md")).unwrap();
    assert_eq!(
        honesty, convey_out,
        "honesty.md is the convey authority stack"
    );
    assert_stack_order(&honesty, &agents, &section);
    assert_eq!(between_agents_and_authority(&honesty, &agents), cites);
    let manifest = std::fs::read_to_string(out.join("MANIFEST.md")).unwrap();
    assert!(manifest.contains("honesty: honesty.md"), "{manifest}");
    assert_eq!(
        out.join("honesty.md").parent(),
        out.join("MANIFEST.md").parent()
    );
    let (copied, missing) = manifest_parts(&manifest);
    assert!(copied.contains("conveyor-mesh.json"), "{manifest}");
    assert!(!missing.contains("conveyor-mesh.json"), "{manifest}");
    assert_eq!(
        std::fs::read(out.join("conveyor-mesh.json")).unwrap(),
        std::fs::read(state.join("conveyor-mesh.json")).unwrap()
    );
    assert!(no_enforced_status_token(&manifest), "{manifest}");
    let listed = Command::new("tar")
        .args([
            "-tzf",
            &dir.join("audit-export.tar.gz").display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        listed.status.success(),
        "{}",
        String::from_utf8_lossy(&listed.stderr)
    );
    let names = String::from_utf8_lossy(&listed.stdout);
    assert!(names.contains("honesty.md"), "{names}");
    assert!(names.contains("MANIFEST.md"), "{names}");
    assert!(names.contains("conveyor-mesh.json"), "{names}");
    assert_eq!(kept_files(&state), kept);
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn export_writes_mismatch_fail_and_still_exits_zero() {
    let mut text = estate_with_lane_tool("allow");
    text = text.replace(
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n",
    );
    let dir = scratch();
    let estate_path = write_estate(&dir, &text);
    let state = dir.join("state");
    apply(&estate_path, &state, &dir);
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
    let (convey_ok, convey_out, convey_err) = convey(&estate_path, &state);
    assert!(convey_ok, "{convey_out}\n{convey_err}");
    assert!(convey_err.is_empty(), "{convey_err}");
    let cites = between_agents_and_authority(&convey_out, &agents);
    assert!(
        cites.starts_with("\n  FAIL  refuse:hop-coverage:")
            && cites.contains("(mismatch)")
            && cites.contains(
                "capability 'notes-append' does not match hop coverage capability 'lane-tool'"
            )
            && !cites.contains("  note  refuse:hop-coverage:"),
        "{cites}"
    );
    let out = dir.join("out");
    let (ok, stdout, stderr) = export(&dir, &estate_path, &state, &out, false);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(
        stderr.is_empty(),
        "a hop cite does not fail the export\n{stderr}"
    );
    let honesty = std::fs::read_to_string(out.join("honesty.md")).unwrap();
    assert_eq!(honesty, convey_out);
    assert_eq!(between_agents_and_authority(&honesty, &agents), cites);
    assert_stack_order(&honesty, &agents, &section);
    assert_eq!(kept_files(&state), kept);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn export_stays_quiet_when_hop_coverage_matches() {
    let dir = scratch();
    let estate_path = write_estate(&dir, &estate_with_lane_tool("allow"));
    let state = dir.join("state");
    apply(&estate_path, &state, &dir);
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
    let (convey_ok, convey_out, convey_err) = convey(&estate_path, &state);
    assert!(convey_ok, "{convey_out}\n{convey_err}");
    assert_eq!(between_agents_and_authority(&convey_out, &agents), "");
    let out = dir.join("out");
    let (ok, stdout, stderr) = export(&dir, &estate_path, &state, &out, false);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let honesty = std::fs::read_to_string(out.join("honesty.md")).unwrap();
    assert_eq!(honesty, convey_out);
    assert_eq!(
        between_agents_and_authority(&honesty, &agents),
        "",
        "a match adds no hop cite\n{honesty}"
    );
    assert!(!honesty.contains("refuse:hop-coverage"), "{honesty}");
    assert!(!honesty.contains("  FAIL  "), "{honesty}");
    assert_stack_order(&honesty, &agents, &section);
    assert_eq!(kept_files(&state), kept);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn export_deny_is_a_note() {
    let dir = scratch();
    let estate_path = write_estate(&dir, &estate_with_lane_tool("deny"));
    let state = dir.join("state");
    apply(&estate_path, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &granted_box("lane-tool")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let (convey_ok, convey_out, convey_err) = convey(&estate_path, &state);
    assert!(convey_ok, "{convey_out}\n{convey_err}");
    let cites = between_agents_and_authority(&convey_out, &agents);
    assert!(
        cites.contains("  note  refuse:hop-coverage:")
            && cites.contains("(deny)")
            && !cites.contains("(deny-default)")
            && !cites.contains("(mismatch)")
            && !cites.contains("  FAIL  "),
        "{cites}"
    );
    let kept = kept_files(&state);
    let out = dir.join("out");
    let (ok, stdout, stderr) = export(&dir, &estate_path, &state, &out, false);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let honesty = std::fs::read_to_string(out.join("honesty.md")).unwrap();
    assert_eq!(honesty, convey_out);
    assert_eq!(between_agents_and_authority(&honesty, &agents), cites);
    let agents_at = honesty.find(&agents).unwrap();
    let cite_at = honesty.find("  note  refuse:hop-coverage:").unwrap();
    let auth_at = honesty.find("Authority\n---------\n").unwrap();
    assert!(agents_at < cite_at && cite_at < auth_at, "{honesty}");
    assert_eq!(kept_files(&state), kept);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn export_missing_mesh_writes_empty_cites_and_not_enforced() {
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
    let (convey_ok, convey_out, convey_err) = convey(&estate_path, &state);
    assert!(convey_ok, "{convey_out}\n{convey_err}");
    assert_eq!(between_agents_and_authority(&convey_out, &agents), "");
    let out = dir.join("out");
    let (ok, stdout, stderr) = export(&dir, &estate_path, &state, &out, false);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let honesty = std::fs::read_to_string(out.join("honesty.md")).unwrap();
    assert_eq!(honesty, convey_out);
    assert_eq!(between_agents_and_authority(&honesty, &agents), "");
    assert!(!honesty.contains("refuse:hop-coverage"), "{honesty}");
    assert!(
        honesty.contains("not-enforced reasons: missing-mesh=5 cloud=1"),
        "{honesty}"
    );
    assert!(
        honesty.contains("conveyor-mesh.json is absent"),
        "{honesty}"
    );
    assert_stack_order(&honesty, &agents, &section);
    assert!(out.join("MANIFEST.md").is_file());
    assert!(!out.join("conveyor-mesh.json").exists());
    let manifest = std::fs::read_to_string(out.join("MANIFEST.md")).unwrap();
    assert!(manifest.contains("honesty: honesty.md"), "{manifest}");
    let (copied, missing) = manifest_parts(&manifest);
    assert!(!copied.contains("conveyor-mesh.json"), "{manifest}");
    assert!(missing.contains("conveyor-mesh.json"), "{manifest}");
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("conveyor-leases.json").exists());
    assert_eq!(kept_files(&state), kept);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unreadable_mesh_refuses_before_the_out_dir() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let mesh = state.join("conveyor-mesh.json");
    std::fs::write(&mesh, "not-json").unwrap();
    std::fs::write(state.join("reconcile.json"), "sentinel\n").unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("keep.txt"), "keep\n").unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = export(&dir, &estate_path, &state, &out, false);
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
    assert_eq!(
        std::fs::read_to_string(state.join("reconcile.json")).unwrap(),
        "sentinel\n"
    );
    assert_eq!(
        std::fs::read_to_string(out.join("keep.txt")).unwrap(),
        "keep\n"
    );
    assert!(!out.join("honesty.md").exists());
    assert!(!out.join("MANIFEST.md").exists());
    assert!(!dir.join("out.tar.gz").exists());

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[],"leases":[{"hop_id":"box","kind":"box","capability":"lane-tool","host_class":"rtx-5090","granted":true,"spawned":false,"durable":true,"driver":"box"}]}"#,
    )
    .unwrap();
    let fresh = dir.join("fresh-out");
    assert!(!fresh.exists());
    let before = snapshot(&state);
    let (ok, stdout, stderr) = export(&dir, &estate_path, &state, &fresh, true);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(stderr.contains("rtx-5090"), "{stderr}");
    assert!(
        !fresh.exists(),
        "refused export must not create the out dir"
    );
    assert!(!dir.join("fresh-out.tar.gz").exists());
    assert_eq!(snapshot(&state), before);
    assert_eq!(
        std::fs::read_to_string(out.join("keep.txt")).unwrap(),
        "keep\n"
    );
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agent_unplaced_refuses_before_the_out_dir() {
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
    std::fs::write(state.join("reconcile.json"), "sentinel\n").unwrap();
    let before = snapshot(&state);
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("keep.txt"), "keep\n").unwrap();
    let (ok, stdout, stderr) = export(&dir, &estate_path, &state, &out, false);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(no_sections(&stdout), "{stdout}");
    assert!(no_sections(&stderr), "{stderr}");
    assert!(stderr.contains("refuse:agent-unplaced"), "{stderr}");
    assert!(stderr.contains("outsider"), "{stderr}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_eq!(
        std::fs::read_to_string(state.join("reconcile.json")).unwrap(),
        "sentinel\n"
    );
    assert_eq!(
        std::fs::read_to_string(out.join("keep.txt")).unwrap(),
        "keep\n"
    );
    assert!(!out.join("honesty.md").exists());
    assert!(!out.join("MANIFEST.md").exists());
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn placement_sku_keeps_agents_and_cites_and_omits_authority() {
    let dir = scratch();
    let state = dir.join("state");
    let estate_path = repo_root().join("examples/estate.yaml");
    apply(&estate_path, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let agents = estate_schema::describe_agents_section(&estate);
    let out1 = dir.join("out1");
    let (ok, stdout, stderr) = export(&dir, &estate_path, &state, &out1, false);
    assert!(ok, "{stdout}\n{stderr}");
    let honesty1 = std::fs::read_to_string(out1.join("honesty.md")).unwrap();
    let (convey_ok, convey_out, convey_err) = convey(&estate_path, &state);
    assert!(convey_ok, "{convey_out}\n{convey_err}");
    assert_eq!(honesty1, convey_out);
    let between = between_agents_and_authority(&honesty1, &agents);
    assert!(
        between.contains("  note  refuse:hop-coverage:") && between.contains("(deny-default)"),
        "{between}"
    );

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
    let audit_before = std::fs::read(state.join("apply-audit.jsonl")).unwrap();
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let out2 = dir.join("out2");
    let (ok, stdout, stderr) = export(&dir, &estate_path, &state, &out2, false);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let honesty2 = std::fs::read_to_string(out2.join("honesty.md")).unwrap();
    assert!(honesty2.starts_with(&agents), "{honesty2}");
    assert_eq!(&honesty2[agents.len()..], &format!("{between}\n"));
    assert!(!honesty2.contains("Authority\n---------"), "{honesty2}");
    assert!(!honesty2.contains("not-enforced reasons:"), "{honesty2}");
    assert!(!honesty2.contains("would-allow="), "{honesty2}");
    assert!(no_enforced_status_token(&honesty2), "{honesty2}");
    let manifest = std::fs::read_to_string(out2.join("MANIFEST.md")).unwrap();
    assert!(manifest.contains("honesty: honesty.md"), "{manifest}");
    let recon = std::fs::read_to_string(out2.join("reconcile.md")).unwrap();
    assert!(recon.contains("refuse:bad-host-class"), "{recon}");
    assert!(recon.contains("rtx-5090"), "{recon}");
    assert_eq!(
        std::fs::read(out2.join("conveyor-mesh.json")).unwrap(),
        mesh_before
    );
    assert_eq!(
        std::fs::read(state.join("conveyor-mesh.json")).unwrap(),
        mesh_before
    );
    assert_eq!(std::fs::read(&leases_path).unwrap(), leases_before);
    assert_eq!(
        std::fs::read(state.join("apply-audit.jsonl")).unwrap(),
        audit_before
    );
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_names_the_snapshot_and_locks_hold() {
    let (ok, stdout, stderr) = run(&["audit", "export", "--help"]);
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
    assert!(stdout.contains("honesty.md"), "{stdout}");
    assert!(stdout.contains("conveyor-mesh.json"), "{stdout}");
    assert!(stdout.contains("do not fail"), "{stdout}");
    assert!(stdout.contains("empty cite list"), "{stdout}");
    assert!(stdout.contains("Does not spawn"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");

    let language =
        std::fs::read_to_string(repo_root().join("docs/UBIQUITOUS_LANGUAGE.md")).unwrap();
    assert!(language.contains("estate audit export"), "{language}");
    assert!(language.contains("honesty.md"), "{language}");
    assert!(language.contains("missing-mesh"), "{language}");
    assert!(!language.contains("READY_FOR_LIVE_TEST: yes"), "{language}");

    let changelog = std::fs::read_to_string(repo_root().join("CHANGELOG.md")).unwrap();
    assert!(changelog.contains("honesty.md"), "{changelog}");
    assert!(
        changelog.contains("render_hop_coverage_cites"),
        "{changelog}"
    );
    assert!(
        !changelog.contains("READY_FOR_LIVE_TEST: yes"),
        "{changelog}"
    );

    let layout = std::fs::read_to_string(repo_root().join("docs/cell-layout.md")).unwrap();
    assert!(layout.contains("honesty.md"), "{layout}");
    assert!(layout.contains("conveyor-mesh.json"), "{layout}");

    for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(repo_root().join(rel)).unwrap();
        assert!(!body.contains("honesty.md"), "{rel} wires the snapshot");
    }
    assert_locked_cksum();
}
