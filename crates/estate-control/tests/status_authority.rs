//! `estate status` prints the same Authority section as plan, drift, apply,
//! doctor, and convey authority, after the hop expired count and the
//! cloud-agent line. Print-only. A would-deny row does not fail status.

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
    let dir = std::env::temp_dir().join(format!("cell-status-authority-cli-{nanos}"));
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

#[test]
fn status_prints_the_same_not_enforced_section_as_the_other_surfaces() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        section.contains("authority would-allow=0 would-deny=0 not-enforced="),
        "{section}"
    );
    assert!(
        section.contains("not-enforced reasons: missing-mesh=5 cloud=1"),
        "{section}"
    );
    assert!(
        section.contains("research notes-append cell-one-box: not-enforced --"),
        "{section}"
    );
    assert!(
        section.contains("conveyor-mesh.json is absent"),
        "{section}"
    );
    assert!(
        !section.contains("no hop lease names this capability"),
        "{section}"
    );
    assert!(no_enforced_status_token(&section), "{section}");
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let (ok, stdout, stderr) = status(&estate_path, &state, &dir);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let hop_at = stdout.find("expired: placement=").unwrap();
    let cloud = "cloud-agent: declared, not spawned\n";
    let cloud_at = stdout.find(cloud).unwrap();
    assert!(hop_at < cloud_at, "{stdout}");
    assert!(stdout[hop_at..cloud_at].contains("hop="), "{stdout}");
    assert_eq!(stdout[cloud_at + cloud.len()..].trim_end(), section);
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("conveyor-leases.json").exists());
    assert!(!state.join("apply-audit.jsonl").exists());

    let (convey_ok, convey_out, convey_err) = convey_authority(&estate_path, &state);
    assert!(convey_ok, "{convey_out}\n{convey_err}");
    assert_eq!(convey_out.trim_end(), section);

    let state_s = state.display().to_string();
    let root_s = repo_root().display().to_string();
    let (doctor_ok, doctor_out, doctor_err) =
        run(&["doctor", "--root", &root_s, "--state-dir", &state_s]);
    assert!(doctor_ok, "{doctor_out}\n{doctor_err}");
    assert!(doctor_out.contains(&section), "{doctor_out}");
    let (strict_ok, strict_out, strict_err) = run(&[
        "doctor",
        "--strict",
        "--root",
        &root_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(strict_ok, "{strict_out}\n{strict_err}");
    assert!(strict_out.contains(&section), "{strict_out}");

    let estate_s = estate_path.display().to_string();
    let roots_s = dir.join("roots").display().to_string();
    let (drift_ok, drift_out, drift_err) = run(&[
        "drift",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--roots-base",
        &roots_s,
    ]);
    assert!(!drift_ok, "{drift_out}\n{drift_err}");
    assert!(drift_err.contains("drift detected"), "{drift_err}");
    assert!(drift_out.contains(&section), "{drift_out}");

    let plans_s = dir.join("plans").display().to_string();
    let (plan_ok, plan_out, plan_err) = run(&[
        "plan",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--plans-dir",
        &plans_s,
    ]);
    assert!(plan_ok, "{plan_out}\n{plan_err}");
    assert!(plan_out.contains(&section), "{plan_out}");

    let packs_s = dir.join("packs").display().to_string();
    let policy = repo_root()
        .join("policy/cell-one.policy.v0.yaml")
        .display()
        .to_string();
    let (apply_ok, apply_out, apply_err) = run(&[
        "apply",
        "--dry-run",
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
    ]);
    assert!(apply_ok, "{apply_out}\n{apply_err}");
    assert!(apply_out.contains(&section), "{apply_out}");

    assert_eq!(snapshot(&state), before);
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);
    let sum = Command::new("cksum").arg(&estate_path).output().unwrap();
    let sum_text = String::from_utf8_lossy(&sum.stdout);
    assert!(
        sum_text.starts_with("43770130 3391"),
        "examples/estate.yaml cksum changed: {sum_text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn status_prints_would_deny_without_a_new_fail_and_writes_nothing() {
    let dir = scratch();
    let estate_path = dir.join("deny.yaml");
    std::fs::write(&estate_path, estate_with_lane_tool("deny")).unwrap();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    conveyor_proxy::persist_mesh(&state, &granted_box("lane-tool")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        section.contains("research lane-tool cell-one-box: would-deny --"),
        "{section}"
    );
    assert!(
        section.contains("research notes-append cell-one-box: not-enforced --"),
        "{section}"
    );
    assert!(
        section.contains("no hop lease names this capability"),
        "{section}"
    );
    let reasons = section
        .lines()
        .find(|line| line.starts_with("not-enforced reasons:"))
        .unwrap_or_else(|| panic!("missing reason classes\n{section}"));
    assert!(
        reasons.contains("no-lease=") && reasons.contains("cloud="),
        "{reasons}"
    );
    assert!(!reasons.contains("missing-mesh"), "{reasons}");
    assert!(
        !section.contains("conveyor-mesh.json is absent"),
        "{section}"
    );
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let locked = std::fs::read(repo_root().join("examples/estate.yaml")).unwrap();
    let (ok, stdout, stderr) = status(&estate_path, &state, &dir);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let cloud = "cloud-agent: declared, not spawned\n";
    let cloud_at = stdout.find(cloud).unwrap();
    assert_eq!(stdout[cloud_at + cloud.len()..].trim_end(), section);
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    let (convey_ok, convey_out, convey_err) = convey_authority(&estate_path, &state);
    assert!(convey_ok, "{convey_out}\n{convey_err}");
    assert_eq!(convey_out.trim_end(), section);
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("apply-audit.jsonl").exists());
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);
    assert_eq!(
        std::fs::read(repo_root().join("examples/estate.yaml")).unwrap(),
        locked
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hop_mismatch_stays_a_would_deny_row_and_status_still_exits_zero() {
    let mut text = estate_with_lane_tool("allow");
    text = text.replace(
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n",
    );
    let dir = scratch();
    let estate_path = dir.join("mismatch.yaml");
    std::fs::write(&estate_path, text).unwrap();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let section = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        section.contains("research notes-append cell-one-box: would-deny --")
            && section.contains("(mismatch)"),
        "{section}"
    );
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let (ok, stdout, stderr) = status(&estate_path, &state, &dir);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains(&section), "{stdout}");
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("apply-audit.jsonl").exists());
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unreadable_mesh_invents_no_authority_rows_and_writes_nothing() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let mesh = state.join("conveyor-mesh.json");
    std::fs::write(&mesh, "not-json").unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let (ok, stdout, stderr) = status(&estate_path, &state, &dir);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        !stdout.contains("Cell One status"),
        "parse refuse stays before the page\n{stdout}"
    );
    assert!(
        !stdout.contains("Authority")
            && !stdout.contains("would-allow")
            && !stdout.contains("would-deny")
            && !stdout.contains("not-enforced"),
        "a mesh that does not parse must not invent Authority rows\n{stdout}"
    );
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(stderr.contains("conveyor-mesh.json"), "{stderr}");
    assert!(
        !stderr.contains("Authority")
            && !stderr.contains("would-allow")
            && !stderr.contains("would-deny")
            && !stderr.contains("not-enforced"),
        "{stderr}"
    );
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert_eq!(std::fs::read(&mesh).unwrap(), b"not-json");
    assert!(!state.join("conveyor-leases.json").exists());
    assert!(!state.join("conveyor-hops.json").exists());
    assert!(!state.join("apply-audit.jsonl").exists());
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[],"leases":[{"hop_id":"box","kind":"box","capability":"lane-tool","host_class":"rtx-5090","granted":true,"spawned":false,"durable":true,"driver":"box"}]}"#,
    )
    .unwrap();
    let before = snapshot(&state);
    let (ok, stdout, stderr) = status(&estate_path, &state, &dir);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(
        !stdout.contains("Authority")
            && !stdout.contains("would-allow")
            && !stdout.contains("not-enforced"),
        "a mesh authority cannot read must not invent Authority rows\n{stdout}"
    );
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert!(no_enforced_status_token(&stderr), "{stderr}");
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("conveyor-leases.json").exists());
    assert!(!state.join("apply-audit.jsonl").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
