//! Day 90+ heal / accept / probe polish. No live Grok / GPU required.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn estate_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn tmp(name: &str) -> PathBuf {
    let p = repo_root().join(format!(
        "target/test-heal-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn probes_live_skip_and_refuse_sku_ids() {
    let out = estate_bin().args(["probes", "--live"]).output().unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("SKIP"));
    assert!(text.contains("live_probed=false"));
    assert!(text.contains("mlx"));
    assert!(text.contains("apple-silicon"));
    assert!(text.contains("ollama"));
    assert!(!text.contains("live_probed=true"));
    assert!(!text.contains("5090"));
    assert!(!text.contains("4090"));
}

#[test]
fn reconcile_suggest_writes_patch_never_applies() {
    let root = tmp("suggest");
    let state = root.join("state");
    let plans = root.join("plans");
    let apply = estate_bin()
        .args([
            "apply",
            "--estate",
            &repo_root().join("examples/estate.yaml").display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        apply.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&apply.stderr)
    );
    let leases = std::fs::read_to_string(state.join("placement-actual.json")).unwrap();
    let suggest = estate_bin()
        .args([
            "reconcile",
            "--suggest",
            "--estate",
            &repo_root().join("examples/estate.yaml").display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        suggest.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&suggest.stderr),
        String::from_utf8_lossy(&suggest.stdout)
    );
    let text = String::from_utf8_lossy(&suggest.stdout);
    assert!(text.contains("auto_apply: false"));
    assert!(text.contains("No patch"));
    assert!(state.join("reconcile-suggest.md").is_file());
    assert_eq!(
        leases,
        std::fs::read_to_string(state.join("placement-actual.json")).unwrap()
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn packs_accept_requires_curator_jason_and_does_not_rewrite_estate() {
    let root = tmp("accept");
    let drop = root.join("drop");
    let accepted = root.join("accepted");
    let proposed = root.join("proposed");
    let estate = repo_root().join("examples/estate.yaml");
    let before = std::fs::read_to_string(&estate).unwrap();
    let pack = estate_bin()
        .args([
            "feed",
            "pack",
            "--feed-dir",
            &root.join("feed").display().to_string(),
            "--drop-dir",
            &drop.display().to_string(),
            "--id",
            "overnight-traces",
        ])
        .output()
        .unwrap();
    assert!(
        pack.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&pack.stderr)
    );
    let propose = estate_bin()
        .args([
            "packs",
            "propose",
            "--id",
            "overnight-traces",
            "--drop-dir",
            &drop.display().to_string(),
            "--accepted-dir",
            &accepted.display().to_string(),
            "--proposed-dir",
            &proposed.display().to_string(),
            "--estate",
            &estate.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        propose.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&propose.stderr)
    );
    let missing = estate_bin()
        .args([
            "packs",
            "accept",
            "--id",
            "overnight-traces",
            "--proposed-dir",
            &proposed.display().to_string(),
            "--accepted-dir",
            &accepted.display().to_string(),
            "--estate",
            &estate.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        !missing.status.success(),
        "accept without --curator must fail"
    );
    let robot = estate_bin()
        .args([
            "packs",
            "accept",
            "--id",
            "overnight-traces",
            "--curator",
            "robot",
            "--proposed-dir",
            &proposed.display().to_string(),
            "--accepted-dir",
            &accepted.display().to_string(),
            "--estate",
            &estate.display().to_string(),
        ])
        .output()
        .unwrap();
    let robot_text = format!(
        "{}{}",
        String::from_utf8_lossy(&robot.stdout),
        String::from_utf8_lossy(&robot.stderr)
    );
    assert!(!robot.status.success());
    assert!(robot_text.contains("refuse:curator"));
    let ok = estate_bin()
        .args([
            "packs",
            "accept",
            "--id",
            "overnight-traces",
            "--curator",
            "jason",
            "--proposed-dir",
            &proposed.display().to_string(),
            "--accepted-dir",
            &accepted.display().to_string(),
            "--estate",
            &estate.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        ok.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&ok.stderr),
        String::from_utf8_lossy(&ok.stdout)
    );
    let text = String::from_utf8_lossy(&ok.stdout);
    assert!(text.contains("auto_apply: false"));
    assert!(text.contains("applied_to_estate: false"));
    assert!(accepted.join("overnight-traces.enrich-edit.md").is_file());
    assert_eq!(before, std::fs::read_to_string(&estate).unwrap());
    let _ = std::fs::remove_dir_all(&root);
}
