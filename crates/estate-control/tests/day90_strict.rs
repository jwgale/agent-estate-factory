//! Day 90+ doctor --strict, dual-layer demo, sanctum-not-Cyera refuse.
//! No live Grok / Mac / GPU required.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn estate_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

#[test]
fn doctor_strict_passes_on_repo() {
    let out = estate_bin()
        .args([
            "doctor",
            "--strict",
            "--root",
            &repo_root().display().to_string(),
            "--state-dir",
            &repo_root().join("target/test-strict-doctor").display().to_string(),
        ])
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "{text}");
    assert!(text.contains("Authority\n---------\n"), "{text}");
    assert!(text.contains("deny-default"), "{text}");
    let auth_at = text.find("Authority\n---------\n").unwrap();
    let strict_at = text.find("Strict (pre-merge)").expect("strict");
    assert!(auth_at < strict_at, "{text}");
    assert!(text.contains("pre-merge operator checks"));
    assert!(text.contains("compile-only"));
    assert!(text.contains("Sanctum is not Cyera"));
    assert!(text.contains("feed-loop"));
    assert!(text.contains("DAY90-PLUS"));
}

#[test]
fn dual_layer_demo_validates_and_sanctum_is_not_cyera() {
    let demo = repo_root().join("examples/fixtures/dual-layer-demo.yaml");
    let out = estate_bin()
        .args(["validate", "--estate", &demo.display().to_string()])
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "{text}");
    assert!(text.contains("sanctum"));
    assert!(!text.to_ascii_lowercase().contains("sanctum is cyera"));
}

#[test]
fn refuse_sanctum_as_cyera_display_name() {
    let estate = repo_root().join("examples/fixtures/refuse-sanctum-as-cyera.yaml");
    let out = estate_bin()
        .args(["validate", "--estate", &estate.display().to_string()])
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.status.success(), "sanctum-as-cyera must fail closed");
    assert!(
        text.contains("Sanctum must not be Cyera") || text.contains("sacred exclusion"),
        "{text}"
    );
}

#[test]
fn omit_locked_sacred_file_still_refuses_cyera_as_agent() {
    let sacred = repo_root().join("examples/fixtures/sacred-omit-locked.yaml");
    let estate = repo_root().join("examples/fixtures/refuse-sacred-as-agent.yaml");
    let out = estate_bin()
        .args([
            "--sacred",
            &sacred.display().to_string(),
            "validate",
            "--estate",
            &estate.display().to_string(),
        ])
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !out.status.success(),
        "omitting locked ids from the sacred file must not admit Cyera CI"
    );
    assert!(text.contains("sacred exclusion") || text.contains("cyera"), "{text}");
}

#[test]
fn doctor_strict_fails_on_empty_root() {
    let root = repo_root().join(format!("target/test-strict-empty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let out = estate_bin()
        .args([
            "doctor",
            "--strict",
            "--root",
            &root.display().to_string(),
            "--state-dir",
            &root.join("cell").display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "empty root must fail --strict");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn doctor_prints_security_iac_coverage() {
    let out = estate_bin()
        .args([
            "doctor",
            "--root",
            &repo_root().display().to_string(),
            "--state-dir",
            &repo_root()
                .join("target/test-doctor-coverage")
                .display()
                .to_string(),
        ])
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "{text}");
    assert!(text.contains("Security-as-IaC"), "{text}");
    assert!(text.contains("horizon frontier xai_grok: deny-default"), "{text}");
    assert!(text.contains("research tool notes-append: deny-default"), "{text}");
    assert!(text.contains("research mount notes: deny-default"), "{text}");
    assert!(text.contains("horizon agent horizon: deny-default (own)"), "{text}");
    assert!(text.contains("horizon agent research: deny-default (peer)"), "{text}");
    assert!(text.contains("horizon memory_read lane:horizon: allow (own-lane)"), "{text}");
    assert!(
        text.contains("horizon memory_read lane:research: deny-default (cross-lane)"),
        "{text}"
    );
    assert!(text.contains("horizon hop cell-one-box lane-tool: deny-default"), "{text}");
    assert!(
        text.contains("cursor-cloud hop mesh-stub: deny (declared, not spawned)"),
        "{text}"
    );
    let hop_at = text
        .find("horizon hop cell-one-box lane-tool: deny-default")
        .unwrap();
    let auth_at = text.find("Authority\n---------\n").expect("authority");
    let health_at = text.find("\nHealth\n").expect("health");
    assert!(hop_at < auth_at && auth_at < health_at, "{text}");
    assert!(text.contains("not-enforced"), "{text}");
    assert!(!text.contains("Strict intentions"), "{text}");
}

#[test]
fn doctor_strict_intentions_fails_on_locked_example() {
    let out = estate_bin()
        .args([
            "doctor",
            "--strict-intentions",
            "--root",
            &repo_root().display().to_string(),
            "--state-dir",
            &repo_root()
                .join("target/test-doctor-strict-intentions")
                .display()
                .to_string(),
        ])
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.status.success(), "{text}");
    assert!(text.contains("strict-intentions"), "{text}");
    assert!(text.contains("deny-default"), "{text}");
}
