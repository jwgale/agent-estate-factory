use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn estate_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn fixture(rel: &str) -> String {
    repo_root().join(rel).display().to_string()
}

#[test]
fn validate_example_ok() {
    let out = estate_bin()
        .args(["validate", "--estate", &fixture("examples/estate.yaml")])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("horizon"));
    assert!(stdout.contains("ok"));
}

#[test]
fn validate_invalid_fails_closed() {
    for name in [
        "too-few-agents.yaml",
        "shared-lane.yaml",
        "sacred-as-agent.yaml",
        "wired-true.yaml",
        "missing-sacred.yaml",
        "allow-sacred-intention.yaml",
        "default-allow.yaml",
        "not-yaml.txt",
    ] {
        let path = fixture(&format!("examples/invalid/{name}"));
        let out = estate_bin()
            .args(["validate", "--estate", &path])
            .output()
            .unwrap();
        assert!(
            !out.status.success(),
            "{name} should fail closed; stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn plan_appends_plans_dir() {
    let tmp = repo_root().join(format!("target/test-plans-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let out = estate_bin()
        .args([
            "plan",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--plans-dir",
            &tmp.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Blast radius"));
    let entries: Vec<_> = std::fs::read_dir(&tmp)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(entries.iter().any(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md")));
    let _ = std::fs::remove_dir_all(&tmp);
}
