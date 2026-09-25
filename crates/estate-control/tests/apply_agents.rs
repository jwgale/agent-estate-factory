//! `estate apply` and `estate apply --dry-run` print the same Agents section
//! as plan, before any estate, mesh, lease, or apply-audit write.

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
    let dir = std::env::temp_dir().join(format!("cell-apply-agents-{nanos}"));
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

/// Horizon on the box, Research unplaced, Sanctum on the declared cloud stub.
/// Horizon calls Research (peer, deny-default) and itself (own, deny).
fn fixture_estate(dir: &Path) -> PathBuf {
    let path = dir.join("estate.yaml");
    let mut text = std::fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
    text = text.replace("agents: [horizon, research, sanctum]", "agents: [horizon]");
    text = text.replace(
        "    agents: []\n    wired: false\n    note: Declared stub.",
        "    agents: [sanctum]\n    wired: false\n    note: Declared stub.",
    );
    let needle = "    mcp: []\n    models:\n      - id: xai_grok\n";
    let insert = "    mcp: []\n    calls:\n      - id: research\n      - id: horizon\n    models:\n      - id: xai_grok\n";
    assert!(text.contains(needle), "horizon mcp block missing");
    text = text.replacen(needle, insert, 1);
    text = text.replace(
        "intentions: []\n",
        "intentions:\n  - subject_agent: horizon\n    object: horizon\n    kind: agent\n    effect: deny\n",
    );
    std::fs::write(&path, text).unwrap();
    path
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

#[test]
fn apply_and_dry_run_print_the_plan_agents_section_and_dry_run_writes_nothing() {
    let dir = scratch();
    let estate_path = fixture_estate(&dir);
    let state = dir.join("state");
    let plans = dir.join("plans");
    let roots = dir.join("roots");
    std::fs::create_dir_all(&state).unwrap();
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
    assert!(section.contains("  placement: none"), "{section}");
    assert!(section.contains("- id: sanctum"), "{section}");
    assert!(
        section.contains("placement: cloud-agent cursor-cloud (declared, not spawned)"),
        "{section}"
    );
    assert!(
        section.contains("declared: tools=0 mcp=0 mounts=0 models=2 calls=2"),
        "{section}"
    );
    assert!(
        section.contains("horizon agent research: deny-default (peer)"),
        "{section}"
    );
    assert!(
        section.contains("horizon agent horizon: deny (own)"),
        "{section}"
    );

    let estate_s = estate_path.display().to_string();
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let roots_s = roots.display().to_string();
    let policy = dir.join("missing-policy.yaml");
    let policy_s = policy.display().to_string();

    let (plan_ok, plan_out, plan_err) = run(&[
        "plan",
        "--estate",
        &estate_s,
        "--plans-dir",
        &plans_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(plan_ok, "{plan_err}");
    assert!(plan_out.contains(&section), "{plan_out}");

    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
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
        "--policy",
        &policy_s,
    ]);
    assert!(dry_ok, "{dry_out}\n{dry_err}");
    assert!(dry_out.contains(&section), "{dry_out}");
    assert!(dry_out.contains("dry-run ok (no writes)"), "{dry_out}");
    assert_eq!(snapshot(&state), before);
    assert!(!state.join("apply-audit.jsonl").exists());
    assert!(!state.join("placement-actual.json").exists());
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);

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
        "--policy",
        &policy_s,
    ]);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stdout.contains(&section), "{stdout}");
    assert!(state.join("apply-audit.jsonl").is_file());
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn apply_still_bails_on_hop_mismatch_and_a_non_agent_call_target() {
    let dir = scratch();
    let estate_path = fixture_estate(&dir);
    let mut text = std::fs::read_to_string(&estate_path).unwrap();
    text = text.replace(
        "      - id: notes-append\n",
        "      - id: notes-append\n      - id: lane-tool\n",
    );
    text = text.replace(
        "    effect: deny\n",
        "    effect: deny\n  - subject_agent: research\n    object: lane-tool\n    kind: tool\n    effect: allow\n",
    );
    text = text.replace("agents: [horizon]", "agents: [horizon, research]");
    std::fs::write(&estate_path, &text).unwrap();
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = estate_schema::load_estate(&estate_path).unwrap();
    let section = estate_schema::describe_agents_section(&estate);
    conveyor_proxy::persist_mesh(
        &state,
        &conveyor_proxy::ConveyorMesh {
            schema: conveyor_proxy::MESH_SCHEMA.into(),
            hops: vec![],
            leases: vec![conveyor_proxy::HopLease {
                hop_id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
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
        },
    )
    .unwrap();
    let before = snapshot(&state);
    let mesh_bytes = std::fs::read(state.join(conveyor_proxy::MESH_FILE)).unwrap();
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let estate_s = estate_path.display().to_string();
    let state_s = state.display().to_string();
    let roots_s = dir.join("roots").display().to_string();
    let plans_s = dir.join("plans").display().to_string();
    let policy_s = dir.join("missing-policy.yaml").display().to_string();
    let args = vec![
        "apply".to_string(),
        "--estate".to_string(),
        estate_s,
        "--state-dir".to_string(),
        state_s,
        "--roots-base".to_string(),
        roots_s,
        "--plans-dir".to_string(),
        plans_s,
        "--policy".to_string(),
        policy_s,
    ];
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (ok, stdout, stderr) = run(&arg_refs);
    let text = format!("{stdout}{stderr}");
    assert!(!ok, "{text}");
    assert!(stdout.contains(&section), "{stdout}");
    assert!(
        text.contains("refuse:hop-coverage") && text.contains("(mismatch)"),
        "{text}"
    );
    assert_eq!(snapshot(&state), before);
    assert_eq!(
        std::fs::read(state.join(conveyor_proxy::MESH_FILE)).unwrap(),
        mesh_bytes
    );
    assert!(!state.join("apply-audit.jsonl").exists());
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);

    let mut dry_args = args.clone();
    dry_args.insert(1, "--dry-run".to_string());
    let dry_refs: Vec<&str> = dry_args.iter().map(String::as_str).collect();
    let (dry_ok, dry_out, dry_err) = run(&dry_refs);
    let dry_text = format!("{dry_out}{dry_err}");
    assert!(!dry_ok, "{dry_text}");
    assert!(dry_out.contains(&section), "{dry_out}");
    assert!(
        dry_text.contains("refuse:hop-coverage") && dry_text.contains("(mismatch)"),
        "{dry_text}"
    );
    assert!(!dry_text.contains("would-refuse"), "{dry_text}");
    assert_eq!(snapshot(&state), before);

    let ghost = dir.join("ghost.yaml");
    let mut ghost_body = String::from_utf8(estate_bytes.clone()).unwrap();
    ghost_body = ghost_body.replace(
        "    calls:\n      - id: research\n      - id: horizon\n",
        "    calls:\n      - id: ghost\n",
    );
    std::fs::write(&ghost, &ghost_body).unwrap();
    let mut ghost_args = args.clone();
    ghost_args[2] = ghost.display().to_string();
    let ghost_refs: Vec<&str> = ghost_args.iter().map(String::as_str).collect();
    let load_err = estate_schema::load_estate(&ghost).unwrap_err().to_string();
    assert!(load_err.contains("not an estate agent"), "{load_err}");
    let (ghost_ok, ghost_out, ghost_err) = run(&ghost_refs);
    let ghost_text_out = format!("{ghost_out}{ghost_err}");
    assert!(!ghost_ok, "{ghost_text_out}");
    assert!(ghost_err.contains("ghost.yaml"), "{ghost_err}");
    assert!(!state.join("apply-audit.jsonl").exists());
    assert_eq!(snapshot(&state), before);
    assert_eq!(std::fs::read(&ghost).unwrap(), ghost_body.as_bytes());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn examples_estate_yaml_stays_hash_locked() {
    let path = repo_root().join("examples/estate.yaml");
    let sum = Command::new("cksum").arg(&path).output().unwrap();
    let text = String::from_utf8_lossy(&sum.stdout);
    assert!(
        text.starts_with("43770130 3391"),
        "examples/estate.yaml cksum changed: {text}"
    );
}
