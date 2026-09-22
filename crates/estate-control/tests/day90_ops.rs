//! Day 90+ operator UX, backup prune, convey policy deny.
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

fn tmp(name: &str) -> PathBuf {
    let p = repo_root().join(format!(
        "target/test-ops-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn text(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn help_topics_cover_day90_loop() {
    let index = estate_bin().args(["help"]).output().unwrap();
    let index_text = text(&index);
    assert!(index.status.success(), "{index_text}");
    assert!(index_text.contains("estate help status"));
    assert!(index_text.contains("make gate-90"));
    assert!(index_text.contains("DAY90-PLUS"));

    for topic in [
        "status",
        "plan",
        "apply",
        "reconcile",
        "feed-loop",
        "backup",
        "frontier",
        "day90-mixed",
    ] {
        let out = estate_bin().args(["help", topic]).output().unwrap();
        let body = text(&out);
        assert!(out.status.success(), "{topic}: {body}");
        assert!(
            body.contains("estate ") || body.contains("make "),
            "{topic} missing examples: {body}"
        );
    }

    let frontier = estate_bin().args(["help", "frontier"]).output().unwrap();
    let frontier_text = text(&frontier);
    assert!(frontier_text.contains("grok-4.7"), "{frontier_text}");
    assert!(frontier_text.contains("XAI_API_KEY"), "{frontier_text}");
    assert!(
        frontier_text.contains("do not POST frontier")
            || frontier_text.contains("does not POST frontier"),
        "{frontier_text}"
    );
    assert!(
        !frontier_text.contains("READY_FOR_LIVE_TEST: yes"),
        "{frontier_text}"
    );

    let mixed = estate_bin().args(["help", "day90-mixed"]).output().unwrap();
    let mixed_text = text(&mixed);
    assert!(mixed_text.contains("make day90-mixed"), "{mixed_text}");
    assert!(mixed_text.contains("Not part of make smoke"), "{mixed_text}");
    assert!(mixed_text.contains("--require-plan"), "{mixed_text}");

    let apply = estate_bin()
        .args(["apply", "--help"])
        .output()
        .unwrap();
    let apply_text = text(&apply);
    assert!(apply.status.success(), "{apply_text}");
    assert!(apply_text.contains("estate help apply"));

    for topic in ["north-star", "charter", "northstar"] {
        let out = estate_bin().args(["help", topic]).output().unwrap();
        let body = text(&out);
        assert!(out.status.success(), "{topic}: {body}");
        assert!(body.contains("Agent Estate Factory"), "{topic}: {body}");
        assert!(body.contains("Not a gateway"), "{topic}: {body}");
        assert!(body.contains("charter.md"), "{topic}: {body}");
        assert!(body.contains("make gate-90"), "{topic}: {body}");
        assert!(body.contains("\n  make day90\n"), "{topic}: {body}");
        assert!(body.contains("docs/LIVE-PROBES.md"), "{topic}: {body}");
        assert!(body.contains("purpose-built"), "{topic}: {body}");
        assert!(body.contains("local runtime"), "{topic}: {body}");
        assert!(!body.contains("Not a distillation"), "{topic}: {body}");
        assert!(!body.contains("training lab"), "{topic}: {body}");
    }

    let bad = estate_bin().args(["help", "gateway"]).output().unwrap();
    let bad_text = text(&bad);
    assert!(!bad.status.success(), "unknown topic must refuse");
    assert!(bad_text.contains("refuse:help-topic"));
}

#[test]
fn backup_prune_keeps_last_n() {
    let root = tmp("backup");
    let state = root.join("state");
    let plans = root.join("plans");
    let backups = root.join("backups");
    let apply = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(apply.status.success(), "{}", text(&apply));

    for _ in 0..3 {
        let bak = estate_bin()
            .args([
                "backup",
                "--estate",
                &fixture("examples/estate.yaml"),
                "--state-dir",
                &state.display().to_string(),
                "--plans-dir",
                &plans.display().to_string(),
                "--out",
                &backups.display().to_string(),
            ])
            .output()
            .unwrap();
        assert!(bak.status.success(), "{}", text(&bak));
    }
    let before = std::fs::read_dir(&backups)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .ok()
                .and_then(|d| d.file_name().to_str().map(|n| n.starts_with("cell-backup-")))
                .unwrap_or(false)
        })
        .count();
    assert!(before >= 3, "need at least 3 backups, got {before}");

    let zero = estate_bin()
        .args([
            "backup",
            "--prune",
            "0",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--out",
            &backups.display().to_string(),
        ])
        .output()
        .unwrap();
    let zero_text = text(&zero);
    assert!(!zero.status.success(), "prune 0 must refuse");
    assert!(zero_text.contains("refuse:prune"));

    let pruned = estate_bin()
        .args([
            "backup",
            "--prune",
            "2",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--out",
            &backups.display().to_string(),
        ])
        .output()
        .unwrap();
    let pruned_text = text(&pruned);
    assert!(pruned.status.success(), "{pruned_text}");
    assert!(pruned_text.contains("cell-one.backup-prune.v0"));
    assert!(pruned_text.contains("prune keep=2"));
    let after = std::fs::read_dir(&backups)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .ok()
                .and_then(|d| d.file_name().to_str().map(|n| n.starts_with("cell-backup-")))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(after, 2, "expected 2 archives after prune, got {after}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn convey_call_refuses_policy_deny() {
    let root = tmp("convey");
    let state = root.join("state");
    let plans = root.join("plans");
    let apply = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(apply.status.success(), "{}", text(&apply));

    let sync = estate_bin()
        .args([
            "convey",
            "sync",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(sync.status.success(), "{}", text(&sync));

    let deny = estate_bin()
        .args([
            "convey",
            "call",
            "--id",
            "cell-one-box",
            "--capability",
            "lane-tool",
            "--state-dir",
            &state.display().to_string(),
            "--policy",
            &fixture("examples/fixtures/policy-deny.yaml"),
        ])
        .output()
        .unwrap();
    let deny_text = text(&deny);
    assert!(!deny.status.success(), "policy-deny must refuse convey-call");
    assert!(
        deny_text.contains("refuse:policy"),
        "expected refuse:policy, got {deny_text}"
    );

    let allow = estate_bin()
        .args([
            "convey",
            "call",
            "--id",
            "cell-one-box",
            "--capability",
            "lane-tool",
            "--state-dir",
            &state.display().to_string(),
            "--policy",
            &fixture("examples/fixtures/policy-allow.yaml"),
        ])
        .output()
        .unwrap();
    assert!(allow.status.success(), "{}", text(&allow));
    let _ = std::fs::remove_dir_all(&root);
}
