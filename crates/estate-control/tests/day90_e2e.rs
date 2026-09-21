//! Dual-layer-demo operator loop on an isolated cell.
//! validate → plan → dry-run → apply → status → reconcile → backup → prune.
//! Fixtures only. No live Grok / Mac / GPU. Cloud never spawned.

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

fn text(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn dual_layer_demo_operator_loop() {
    let root = repo_root().join(format!(
        "target/test-e2e-dual-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let backups = root.join("backups");
    let estate = fixture("examples/fixtures/dual-layer-demo.yaml");
    let sacred = fixture("policy/sacred.yaml");
    let estate_before = std::fs::read_to_string(&estate).unwrap();
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let roots_s = root.display().to_string();
    let backups_s = backups.display().to_string();

    let validate = estate_bin()
        .args(["validate", "--estate", &estate, "--sacred", &sacred])
        .output()
        .unwrap();
    let validate_text = text(&validate);
    assert!(validate.status.success(), "{validate_text}");
    assert!(validate_text.contains("horizon"), "{validate_text}");
    assert!(validate_text.contains("research"), "{validate_text}");
    assert!(validate_text.contains("sanctum"), "{validate_text}");
    assert!(
        !validate_text.to_lowercase().contains("cyera-ci is an agent"),
        "Sanctum must not read as Cyera: {validate_text}"
    );

    let plan = estate_bin()
        .args([
            "plan",
            "--estate",
            &estate,
            "--sacred",
            &sacred,
            "--plans-dir",
            &plans_s,
            "--state-dir",
            &state_s,
        ])
        .output()
        .unwrap();
    assert!(plan.status.success(), "{}", text(&plan));
    let plan_files: Vec<_> = std::fs::read_dir(&plans)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "md" || s == "json")
                .unwrap_or(false)
        })
        .collect();
    assert!(!plan_files.is_empty(), "plan must write under plans/");

    let dry = estate_bin()
        .args([
            "apply",
            "--dry-run",
            "--estate",
            &estate,
            "--sacred",
            &sacred,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots_s,
            "--plans-dir",
            &plans_s,
        ])
        .output()
        .unwrap();
    let dry_text = text(&dry);
    assert!(dry.status.success(), "{dry_text}");
    assert!(
        !state.join("placement-actual.json").is_file(),
        "dry-run must not write leases"
    );

    let apply = estate_bin()
        .args([
            "apply",
            "--estate",
            &estate,
            "--sacred",
            &sacred,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots_s,
            "--plans-dir",
            &plans_s,
        ])
        .output()
        .unwrap();
    assert!(apply.status.success(), "{}", text(&apply));
    assert!(state.join("placement-actual.json").is_file());
    let leases = std::fs::read_to_string(state.join("placement-actual.json")).unwrap();
    assert!(leases.contains("cell-one-box"));
    assert!(leases.contains("cursor-cloud"));
    assert!(
        !leases.contains("\"spawned\": true") || leases.contains("cloud-agent"),
        "{leases}"
    );
    assert!(
        !leases.contains("\"kind\": \"cloud-agent\"") || leases.contains("\"spawned\": false"),
        "cloud-agent must stay unspawned: {leases}"
    );

    let status = estate_bin()
        .args([
            "status",
            "--estate",
            &estate,
            "--sacred",
            &sacred,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots_s,
            "--plans-dir",
            &plans_s,
        ])
        .output()
        .unwrap();
    let status_text = text(&status);
    assert!(status.status.success(), "{status_text}");
    assert!(status_text.contains("paused: no"), "{status_text}");
    assert!(
        status_text.contains("declared, not spawned"),
        "{status_text}"
    );

    let recon = estate_bin()
        .args([
            "reconcile",
            "--estate",
            &estate,
            "--sacred",
            &sacred,
            "--state-dir",
            &state_s,
        ])
        .output()
        .unwrap();
    let recon_text = text(&recon);
    assert!(recon.status.success(), "{recon_text}");
    assert!(recon_text.contains("in_sync: true"), "{recon_text}");
    assert!(state.join("reconcile.md").is_file());

    let bak = estate_bin()
        .args([
            "backup",
            "--estate",
            &estate,
            "--sacred",
            &sacred,
            "--state-dir",
            &state_s,
            "--plans-dir",
            &plans_s,
            "--out",
            &backups_s,
        ])
        .output()
        .unwrap();
    assert!(bak.status.success(), "{}", text(&bak));

    let pruned = estate_bin()
        .args([
            "backup",
            "--prune",
            "1",
            "--estate",
            &estate,
            "--sacred",
            &sacred,
            "--state-dir",
            &state_s,
            "--plans-dir",
            &plans_s,
            "--out",
            &backups_s,
        ])
        .output()
        .unwrap();
    let pruned_text = text(&pruned);
    assert!(pruned.status.success(), "{pruned_text}");
    assert!(pruned_text.contains("cell-one.backup-prune.v0"), "{pruned_text}");
    assert!(pruned_text.contains("prune keep=1"), "{pruned_text}");
    let left = std::fs::read_dir(&backups)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .ok()
                .and_then(|d| d.file_name().to_str().map(|n| n.starts_with("cell-backup-")))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(left, 1, "prune 1 must leave one archive");

    let estate_after = std::fs::read_to_string(&estate).unwrap();
    assert_eq!(
        estate_before, estate_after,
        "loop must not rewrite the demo estate"
    );
    let _ = std::fs::remove_dir_all(&root);
}
