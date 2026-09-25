//! `estate status` prints the same Agents section as plan, drift, and apply,
//! from `describe_agents_section`, after the cloud-agent line and before
//! Authority. Print-only. Deny and deny-default notes do not fail status.

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
    let dir = std::env::temp_dir().join(format!("cell-status-agents-{nanos}"));
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

fn no_enforced_status_token(text: &str) -> bool {
    !text.split_whitespace().any(|word| {
        let token = word.trim_matches(|c: char| c == ':' || c == ',' || c == '.' || c == ';');
        token == "enforced"
    })
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

#[test]
fn status_prints_the_same_agents_section_and_writes_nothing() {
    let dir = scratch();
    let estate_path = fixture_estate(&dir);
    let state = dir.join("state");
    let plans = dir.join("plans");
    let packs = dir.join("packs");
    let roots = dir.join("roots");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::create_dir_all(&packs).unwrap();
    std::fs::create_dir_all(&roots).unwrap();
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
    assert!(no_enforced_status_token(&section), "{section}");

    let rows = conveyor_proxy::authority_report(&state, &estate).unwrap();
    let authority = conveyor_proxy::describe_authority_section(&rows, &state);
    assert!(
        authority.starts_with("Authority\n---------\n"),
        "{authority}"
    );
    assert!(no_enforced_status_token(&authority), "{authority}");

    let estate_s = estate_path.display().to_string();
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let packs_s = packs.display().to_string();
    let roots_s = roots.display().to_string();
    let policy = repo_root()
        .join("policy/cell-one.policy.v0.yaml")
        .display()
        .to_string();
    let root_s = repo_root().display().to_string();
    let before = snapshot(&state);
    let estate_bytes = std::fs::read(&estate_path).unwrap();
    let (ok, stdout, stderr) = run(&[
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
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let cloud = "cloud-agent: declared, not spawned\n";
    let cloud_at = stdout
        .find(cloud)
        .unwrap_or_else(|| panic!("missing cloud-agent line\n{stdout}"));
    let tail = &stdout[cloud_at + cloud.len()..];
    assert!(
        tail.starts_with(&section),
        "Agents section is describe_agents_section\n{tail}"
    );
    let after_agents = &tail[section.len()..];
    assert!(
        after_agents.starts_with(&format!("\n{authority}")),
        "Authority follows Agents\n{after_agents}"
    );
    assert!(no_enforced_status_token(&stdout), "{stdout}");
    assert_eq!(snapshot(&state), before);
    assert!(
        plans.read_dir().unwrap().next().is_none(),
        "status wrote a plan"
    );
    assert!(!state.join("conveyor-mesh.json").exists());
    assert!(!state.join("conveyor-leases.json").exists());
    assert!(!state.join("placement-actual.json").exists());
    assert!(!state.join("apply-audit.jsonl").exists());
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);

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
        "--policy",
        &policy,
    ]);
    assert!(dry_ok, "{dry_out}\n{dry_err}");
    assert!(dry_out.contains(&section), "{dry_out}");
    let agents_at = dry_out.find(&section).unwrap();
    let auth_at = dry_out
        .find("Authority\n---------\n")
        .unwrap_or_else(|| panic!("missing Authority section\n{dry_out}"));
    assert!(agents_at < auth_at, "{dry_out}");

    assert_eq!(snapshot(&state), before);
    assert_eq!(std::fs::read(&estate_path).unwrap(), estate_bytes);
    let locked = repo_root().join("examples/estate.yaml");
    let sum = Command::new("cksum").arg(&locked).output().unwrap();
    let sum_text = String::from_utf8_lossy(&sum.stdout);
    assert!(
        sum_text.starts_with("43770130 3391"),
        "examples/estate.yaml cksum changed: {sum_text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn status_help_names_the_shared_agents_section_before_authority() {
    let (ok, stdout, stderr) = run(&["help", "status"]);
    assert!(ok, "{stdout}\n{stderr}");
    let agents = stdout
        .find("describe_agents_section")
        .unwrap_or_else(|| panic!("help omits describe_agents_section\n{stdout}"));
    let authority = stdout
        .find("describe_authority_section")
        .unwrap_or_else(|| panic!("help omits describe_authority_section\n{stdout}"));
    let cites = stdout
        .find("hop_coverage_cites")
        .unwrap_or_else(|| panic!("help omits hop_coverage_cites\n{stdout}"));
    assert!(agents < cites && cites < authority, "{stdout}");
    assert!(stdout.contains("do not fail status"), "{stdout}");
    assert!(stdout.contains("Neither fails status."), "{stdout}");
    assert!(stdout.contains("Does not spawn."), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
}
