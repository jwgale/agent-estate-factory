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
        "frontier-only.yaml",
        "missing-sacred.yaml",
        "allow-sacred-intention.yaml",
        "default-allow.yaml",
        "sku-binding.yaml",
        "placement-sku.yaml",
        "placement-unknown-agent.yaml",
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

#[test]
fn apply_require_plan_is_gated_and_auditable() {
    let tmp = repo_root().join(format!("target/test-apply-gate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let plans = tmp.join("plans");
    let state = tmp.join("state");
    let gated = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--require-plan",
        ])
        .output()
        .unwrap();
    assert!(
        !gated.status.success(),
        "apply without a covering plan must fail; stderr={}",
        String::from_utf8_lossy(&gated.stderr)
    );
    let planned = estate_bin()
        .args([
            "plan",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--plans-dir",
            &plans.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        planned.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&planned.stderr)
    );
    let applied = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--require-plan",
        ])
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&applied.stderr)
    );
    assert!(state.join("placement-actual.json").is_file());
    let stdout = String::from_utf8_lossy(&applied.stdout);
    assert!(stdout.contains("audit"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn feed_import_does_not_rewrite_estate_and_promote_fails() {
    let tmp = repo_root().join(format!("target/test-feed-import-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("drop")).unwrap();
    let estate = repo_root().join("examples/estate.yaml");
    let before = std::fs::read_to_string(&estate).unwrap();
    let packed = estate_bin()
        .args([
            "feed",
            "pack",
            "--feed-dir",
            &tmp.join("feed").display().to_string(),
            "--drop-dir",
            &tmp.join("drop").display().to_string(),
            "--id",
            "overnight-traces",
        ])
        .output()
        .unwrap();
    assert!(
        packed.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&packed.stderr)
    );
    let imported = estate_bin()
        .args([
            "feed",
            "import",
            "--id",
            "overnight-traces",
            "--drop-dir",
            &tmp.join("drop").display().to_string(),
            "--accepted-dir",
            &tmp.join("accepted").display().to_string(),
            "--estate",
            &estate.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        imported.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&imported.stderr)
    );
    let stdout = String::from_utf8_lossy(&imported.stdout);
    assert!(stdout.contains("estate_bound=false"));
    assert!(stdout.contains("estate file unchanged"));
    assert_eq!(before, std::fs::read_to_string(&estate).unwrap());
    let promo = estate_bin()
        .args(["feed", "promote", "--id", "overnight-traces"])
        .output()
        .unwrap();
    assert!(!promo.status.success());
    let _ = std::fs::remove_dir_all(&tmp);
}
