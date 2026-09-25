//! Status does not print "declared, not spawned" when a cloud-agent
//! lease is spawned. That line stays on the unspawned path.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-status-spawn-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn run(bin: &str, args: &[&str]) -> (bool, String) {
    let out = Command::new(bin)
        .args(args)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

fn placement(spawned: bool) -> String {
    format!(
        r#"{{"schema":"cell-one.placement-actual.v0","desired_hash":"abc","leases":[{{"placement_id":"cell-one-box","kind":"box","host_class":"any","agents":["horizon","research","sanctum"],"wired":true,"spawned":false,"durable":true}},{{"placement_id":"cursor-cloud","kind":"cloud-agent","host_class":"any","agents":[],"wired":false,"spawned":{spawned},"durable":true}}]}}
"#
    )
}

#[test]
fn status_does_not_call_a_spawned_cloud_lease_not_spawned() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("spawn");
    let state = dir.join("state");
    let plans = dir.join("plans");
    let packs = dir.join("packs");
    std::fs::create_dir_all(&state).unwrap();
    let estate = root.join("examples/estate.yaml");
    let status = [
        "status",
        "--estate",
        &estate.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--roots-base",
        &state.display().to_string(),
        "--plans-dir",
        &plans.display().to_string(),
        "--packs-dir",
        &packs.display().to_string(),
        "--root",
        &root.display().to_string(),
    ];

    let (ok, text) = run(bin, &status);
    assert!(ok, "{text}");
    assert!(text.contains("Cell One status"), "{text}");
    assert!(
        text.contains("cloud-agent: declared, not spawned"),
        "{text}"
    );
    assert_agents_then_authority(&text);

    let path = state.join("placement-actual.json");
    let unspawned = placement(false);
    std::fs::write(&path, &unspawned).unwrap();
    let (ok, text) = run(bin, &status);
    assert!(ok, "{text}");
    assert!(
        text.contains("cloud-agent: declared, not spawned"),
        "{text}"
    );
    assert_agents_then_authority(&text);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), unspawned);

    let spawned = placement(true);
    std::fs::write(&path, &spawned).unwrap();
    let (ok, text) = run(bin, &status);
    assert!(!ok, "{text}");
    assert!(
        text.contains("cloud-agent lease spawned (fail closed): cursor-cloud"),
        "{text}"
    );
    assert!(
        !text.contains("cloud-agent: declared, not spawned"),
        "a spawned lease must not be called not spawned: {text}"
    );
    assert!(
        !text.contains("\nAgents\n------\n"),
        "spawned refuse stays before the Agents section: {text}"
    );
    assert!(
        !text.contains("\nAuthority\n---------\n"),
        "spawned refuse stays before the Authority section: {text}"
    );
    assert!(
        !text.contains("refuse:hop-coverage"),
        "spawned refuse stays before hop coverage cites: {text}"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), spawned);
}

fn assert_agents_then_authority(text: &str) {
    let cloud_at = text
        .find("cloud-agent: declared, not spawned")
        .unwrap_or_else(|| panic!("missing cloud-agent line\n{text}"));
    let agents_at = text
        .find("\nAgents\n------\n")
        .unwrap_or_else(|| panic!("missing Agents section\n{text}"));
    let auth_at = text
        .find("\nAuthority\n---------\n")
        .unwrap_or_else(|| panic!("missing Authority section\n{text}"));
    assert!(
        cloud_at < agents_at && agents_at < auth_at,
        "Agents follows the cloud-agent line and precedes Authority\n{text}"
    );
}
