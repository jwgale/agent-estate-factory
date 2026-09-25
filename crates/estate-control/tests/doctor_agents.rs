//! `estate doctor` and `estate doctor --strict` print the same Agents section
//! as plan, drift, apply, and status, from `describe_agents_section`, after
//! the hop describe lines and before hop coverage cites and Authority.
//! Print-only. Deny and deny-default notes do not fail `--strict`.

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
    let dir = std::env::temp_dir().join(format!("cell-doctor-agents-{nanos}"));
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

fn hop_describe_block(estate: &estate_schema::Estate) -> String {
    let mut block = String::from("hop:\n");
    for line in estate_schema::describe_hop_coverage(estate).lines() {
        block.push_str("  ");
        block.push_str(line);
        block.push('\n');
    }
    block
}

fn doctor(root: &Path, state: &Path, strict: bool) -> (bool, String, String) {
    let root_s = root.display().to_string();
    let state_s = state.display().to_string();
    if strict {
        run(&[
            "doctor",
            "--strict",
            "--root",
            &root_s,
            "--state-dir",
            &state_s,
        ])
    } else {
        run(&["doctor", "--root", &root_s, "--state-dir", &state_s])
    }
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

fn assert_unchanged(
    state: &Path,
    before: &[(String, Vec<u8>)],
    estate: &Path,
    estate_bytes: &[u8],
) {
    assert_eq!(snapshot(state), before);
    assert!(!state.join("apply-audit.jsonl").exists());
    assert!(!state.join("placement-actual.json").exists());
    assert!(!state.join("sessions.jsonl").exists());
    assert!(!state.join("actual-state.json").exists());
    assert_eq!(std::fs::read(estate).unwrap(), estate_bytes);
}

fn assert_locked_cksum() {
    let locked = repo_root().join("examples/estate.yaml");
    let sum = Command::new("cksum").arg(&locked).output().unwrap();
    let sum_text = String::from_utf8_lossy(&sum.stdout);
    assert!(
        sum_text.starts_with("43770130 3391"),
        "examples/estate.yaml cksum changed: {sum_text}"
    );
}

#[test]
fn doctor_prints_the_same_agents_section_as_plan_drift_apply_and_status() {
    let dir = scratch();
    let state = dir.join("state");
    let plans = dir.join("plans");
    let packs = dir.join("packs");
    let roots = dir.join("roots");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::create_dir_all(&packs).unwrap();
    std::fs::create_dir_all(&roots).unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let section = estate_schema::describe_agents_section(&estate);
    assert!(section.starts_with("Agents\n------\n"), "{section}");
    assert!(section.contains("- id: horizon"), "{section}");
    assert!(section.contains("  lane: horizon"), "{section}");
    assert!(section.contains("  desktop: horizon-desktop"), "{section}");
    assert!(
        section.contains("  placement: box cell-one-box"),
        "{section}"
    );
    assert!(section.contains("- id: research"), "{section}");
    assert!(section.contains("- id: sanctum"), "{section}");
    assert!(
        section.contains("Cloud-agent stays declared, not spawned."),
        "{section}"
    );
    assert!(section.contains("deny-default"), "{section}");
    assert!(
        section.contains("horizon agent research: deny-default (peer)"),
        "{section}"
    );
    assert!(no_enforced_status_token(&section), "{section}");
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let authority = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        authority.starts_with("Authority\n---------\n"),
        "{authority}"
    );
    let hop = hop_describe_block(&estate);
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let root = repo_root();

    for strict in [false, true] {
        let (ok, stdout, stderr) = doctor(&root, &state, strict);
        assert!(ok, "strict={strict}\n{stdout}\n{stderr}");
        assert!(stderr.is_empty(), "{stderr}");
        let hop_at = stdout
            .find(&hop)
            .unwrap_or_else(|| panic!("missing hop describe\n{stdout}"));
        let after_hop = &stdout[hop_at + hop.len()..];
        assert!(
            after_hop.starts_with(&section),
            "Agents section is describe_agents_section\n{after_hop}"
        );
        let after_agents = &after_hop[section.len()..];
        assert!(
            after_agents.starts_with(&format!("\n{authority}")),
            "Authority follows Agents\n{after_agents}"
        );
        let auth_at = stdout.find("Authority\n---------\n").unwrap();
        let health_at = stdout.find("\nHealth\n").unwrap();
        assert!(hop_at < auth_at && auth_at < health_at, "{stdout}");
        assert!(stdout.contains("factory ready"), "{stdout}");
        assert!(no_enforced_status_token(&stdout), "{stdout}");
        if strict {
            let strict_at = stdout.find("Strict (pre-merge)").expect("strict");
            assert!(health_at < strict_at, "{stdout}");
            assert!(stdout.contains("pre-merge operator checks"), "{stdout}");
        }
    }
    assert_unchanged(&state, &before, &estate_path, &estate_bytes);

    let estate_s = estate_path.display().to_string();
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let packs_s = packs.display().to_string();
    let roots_s = roots.display().to_string();
    let policy = repo_root()
        .join("policy/cell-one.policy.v0.yaml")
        .display()
        .to_string();
    let root_s = root.display().to_string();

    let (plan_ok, plan_out, plan_err) = run(&[
        "plan",
        "--estate",
        &estate_s,
        "--plans-dir",
        &plans_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(plan_ok, "{plan_out}\n{plan_err}");
    assert!(plan_out.contains(&section), "{plan_out}");

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

    let (dry_ok, dry_out, dry_err) = run(&[
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
    assert!(dry_ok, "{dry_out}\n{dry_err}");
    assert!(dry_out.contains(&section), "{dry_out}");

    let (status_ok, status_out, status_err) = run(&[
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
    ]);
    assert!(status_ok, "{status_out}\n{status_err}");
    assert!(status_out.contains(&section), "{status_out}");
    assert!(no_enforced_status_token(&status_out), "{status_out}");

    assert_unchanged(&state, &before, &estate_path, &estate_bytes);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn deny_and_deny_default_notes_do_not_fail_strict_on_the_locked_example() {
    let dir = scratch();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate_path = repo_root().join("examples/estate.yaml");
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let section = estate_schema::describe_agents_section(&estate);
    assert!(section.contains("deny-default"), "{section}");
    assert!(no_enforced_status_token(&section), "{section}");
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let authority = conveyor_proxy::describe_authority_section(&rows, &state);
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let root = repo_root();

    for strict in [false, true] {
        let (ok, stdout, stderr) = doctor(&root, &state, strict);
        assert!(ok, "strict={strict}\n{stdout}\n{stderr}");
        assert!(stdout.contains("factory ready"), "{stdout}");
        assert!(stdout.contains(&section), "{stdout}");
        assert!(stdout.contains(&authority), "{stdout}");
        let agents_at = stdout.find(&section).unwrap();
        let note_at = stdout
            .find("  note  refuse:hop-coverage:")
            .unwrap_or_else(|| panic!("missing deny-default note\n{stdout}"));
        let note_line = stdout[note_at..]
            .lines()
            .next()
            .unwrap_or_else(|| panic!("missing note line\n{stdout}"));
        assert!(
            note_line.contains("(deny-default)") && !note_line.contains("(mismatch)"),
            "{note_line}"
        );
        assert!(!stdout.contains("  FAIL  refuse:hop-coverage:"), "{stdout}");
        let auth_at = stdout.find("Authority\n---------\n").unwrap();
        let health_at = stdout.find("\nHealth\n").unwrap();
        assert!(
            agents_at < note_at && note_at < auth_at && auth_at < health_at,
            "{stdout}"
        );
        assert!(no_enforced_status_token(&stdout), "{stdout}");
        if strict {
            assert!(stdout.contains("pre-merge operator checks"), "{stdout}");
        }
    }
    assert_unchanged(&state, &before, &estate_path, &estate_bytes);
    assert!(state.join("conveyor-mesh.json").is_file());
    assert!(state.join("conveyor-leases.json").is_file());
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hop_mismatch_prints_agents_then_fails_and_writes_nothing() {
    let mut text = std::fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
    text = text.replace(
        "      - id: notes-append\n        description: Append a note inside the Research lane\n",
        "      - id: notes-append\n        description: Append a note inside the Research lane\n      - id: lane-tool\n",
    );
    text = text.replace(
        "intentions: []\n",
        "intentions:\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n  - subject_agent: research\n    object: notes-append\n    kind: tool\n    effect: allow\n",
    );
    let root = doctor_root(&text);
    let estate_path = root.join("examples/estate.yaml");
    let state = root.join("state");
    std::fs::create_dir_all(&state).unwrap();
    conveyor_proxy::persist_mesh(&state, &granted_box("notes-append")).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let section = estate_schema::describe_agents_section(&estate);
    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let authority = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        authority.contains("research notes-append cell-one-box: would-deny --"),
        "{authority}"
    );
    assert!(
        authority.contains("refuse:hop-coverage") && authority.contains("(mismatch)"),
        "{authority}"
    );
    assert!(no_enforced_status_token(&section), "{section}");
    assert!(no_enforced_status_token(&authority), "{authority}");
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();

    for strict in [false, true] {
        let (ok, stdout, stderr) = doctor(&root, &state, strict);
        assert!(!ok, "strict={strict}\n{stdout}\n{stderr}");
        assert!(stderr.contains("doctor failed (1 check(s))"), "{stderr}");
        assert!(!stdout.contains("factory ready"), "{stdout}");
        assert!(!stdout.contains("pre-merge operator checks"), "{stdout}");
        let hop = hop_describe_block(&estate);
        let hop_at = stdout
            .find(&hop)
            .unwrap_or_else(|| panic!("missing hop describe\n{stdout}"));
        let after_hop = &stdout[hop_at + hop.len()..];
        assert!(
            after_hop.starts_with(&section),
            "Agents stays visible before the fail\n{after_hop}"
        );
        let after_agents = &after_hop[section.len()..];
        assert!(
            after_agents.starts_with("\n  FAIL  refuse:hop-coverage:"),
            "{after_agents}"
        );
        assert!(
            after_agents.contains("(mismatch)") && !after_agents.starts_with("\n  note  "),
            "{after_agents}"
        );
        let fail_at = stdout.find("  FAIL  refuse:hop-coverage:").unwrap();
        let auth_at = stdout.find(&authority).unwrap();
        let health_at = stdout.find("\nHealth\n").unwrap();
        assert!(
            hop_at < fail_at && fail_at < auth_at && auth_at < health_at,
            "{stdout}"
        );
        assert!(no_enforced_status_token(&stdout), "{stdout}");
        assert!(no_enforced_status_token(&stderr), "{stderr}");
    }
    assert_unchanged(&state, &before, &estate_path, &estate_bytes);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn doctor_help_names_the_shared_agents_section_before_authority() {
    let (ok, stdout, stderr) = run(&["doctor", "--help"]);
    assert!(ok, "{stdout}\n{stderr}");
    let agents = stdout
        .find("describe_agents_section")
        .unwrap_or_else(|| panic!("help omits describe_agents_section\n{stdout}"));
    let authority = stdout
        .find("describe_authority_section")
        .unwrap_or_else(|| panic!("help omits describe_authority_section\n{stdout}"));
    assert!(agents < authority, "{stdout}");
    assert!(stdout.contains("do not fail doctor"), "{stdout}");
    assert!(stdout.contains("Does not spawn"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
}
