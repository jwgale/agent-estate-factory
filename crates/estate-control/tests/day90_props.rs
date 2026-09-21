//! Day 90+ property / Makefile / dry-run / restore locks.
//! Isolated cells. Fixtures only. Cloud never spawned.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn estate_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn tmp(name: &str) -> PathBuf {
    let p = repo_root().join(format!(
        "target/test-props-{}-{}",
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

fn fixture(rel: &str) -> String {
    repo_root().join(rel).display().to_string()
}

fn walk_bytes(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, out);
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let bytes = std::fs::read(&path).unwrap_or_default();
        out.push((rel.display().to_string(), bytes));
    }
}

fn invokes_gh(text: &str) -> bool {
    text.lines().any(|line| {
        let t = line.trim();
        if t.starts_with('#') {
            return false;
        }
        t.split_whitespace()
            .any(|w| w == "gh" || w.starts_with("gh ") || w == "$(gh)")
    })
}

fn makefile_has_target(makefile: &str, name: &str) -> bool {
    makefile.lines().any(|line| {
        let t = line.trim();
        t == format!("{name}:") || t.starts_with(&format!("{name}:"))
    })
}

#[test]
fn makefile_contract_gate90_stays_local() {
    let makefile = std::fs::read_to_string(repo_root().join("Makefile")).unwrap();
    for target in [
        "gate-90",
        "smoke",
        "day90",
        "feed-loop",
        "fixtures-check",
        "doctor-strict",
    ] {
        assert!(
            makefile_has_target(&makefile, target),
            "Makefile missing target {target}"
        );
    }
    assert!(
        makefile.contains("scripts/day90-gate.sh"),
        "gate-90 must wrap day90-gate.sh"
    );
    assert!(
        makefile.contains("Do not add to GitHub Actions"),
        "gate-90 comment must stay local-only"
    );

    let gate = std::fs::read_to_string(repo_root().join("scripts/day90-gate.sh")).unwrap();
    let smoke = std::fs::read_to_string(repo_root().join("scripts/smoke.sh")).unwrap();
    let day90 = std::fs::read_to_string(repo_root().join("scripts/day90.sh")).unwrap();
    assert!(
        !invokes_gh(&makefile) && !invokes_gh(&gate) && !invokes_gh(&smoke) && !invokes_gh(&day90),
        "gate-90 / smoke / day90 must not invoke gh"
    );
    assert!(
        !gate.contains("gh workflow") && !gate.contains("gh run"),
        "day90-gate.sh must not invoke Actions"
    );
    assert!(
        gate.contains("Do not add to GitHub Actions"),
        "day90-gate.sh must stay local-only"
    );

    let ci = std::fs::read_to_string(repo_root().join(".github/workflows/ci.yml")).unwrap();
    assert!(
        !ci.lines().any(|l| {
            let t = l.trim();
            !t.starts_with('#')
                && (t.contains("make gate-90")
                    || t.contains("make smoke")
                    || t.contains("make day90")
                    || t.contains("make feed-loop")
                    || t.contains("make day90-mixed"))
        }),
        "ci.yml must not invoke local gates"
    );
    assert!(ci.contains("cargo check --workspace --locked"));
}

#[test]
fn two_dry_runs_leave_identical_tree() {
    let root = tmp("dry");
    let state = root.join("state");
    let plans = root.join("plans");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture("examples/fixtures/dual-layer-demo.yaml");
    let sacred = fixture("policy/sacred.yaml");
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let roots_s = root.display().to_string();

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
    let before = walk_bytes(&state);

    for n in 1..=2 {
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
        assert!(dry.status.success(), "dry-run {n}: {dry_text}");
        let after = walk_bytes(&state);
        assert_eq!(
            before, after,
            "dry-run {n} mutated .cell (path+bytes must match)"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dual_layer_backup_restore_round_trip() {
    let root = tmp("restore");
    let state = root.join("state");
    let plans = root.join("plans");
    let backups = root.join("backups");
    let estate = fixture("examples/fixtures/dual-layer-demo.yaml");
    let sacred = fixture("policy/sacred.yaml");
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let roots_s = root.display().to_string();
    let backups_s = backups.display().to_string();

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
    let leases = std::fs::read_to_string(state.join("placement-actual.json")).unwrap();
    assert!(leases.contains("cell-one-box"));
    assert!(
        !leases.contains("\"kind\": \"cloud-agent\"") || leases.contains("\"spawned\": false"),
        "cloud-agent must stay unspawned: {leases}"
    );

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
    let archive = std::fs::read_dir(&backups)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("cell-backup-"))
                .unwrap_or(false)
        })
        .expect("backup must write cell-backup-*");
    let meta = std::fs::read_to_string(archive.join("backup.json")).unwrap();
    assert!(meta.contains("cyera-ci"), "{meta}");
    assert!(meta.contains("rust-classroom"), "{meta}");
    assert!(!meta.contains("lab-notebook"), "overlays stay out of backup sacred_ids: {meta}");

    let _ = std::fs::remove_dir_all(&state);
    std::fs::create_dir_all(&state).unwrap();
    assert!(!state.join("placement-actual.json").is_file());

    let dry = estate_bin()
        .args([
            "restore",
            "--from",
            &archive.display().to_string(),
            "--estate",
            &estate,
            "--sacred",
            &sacred,
            "--state-dir",
            &state_s,
            "--plans-dir",
            &plans_s,
            "--dry-run",
        ])
        .output()
        .unwrap();
    let dry_text = text(&dry);
    assert!(dry.status.success(), "{dry_text}");
    assert!(!state.join("placement-actual.json").is_file());
    assert!(dry_text.contains("writes: false"), "{dry_text}");

    let restored = estate_bin()
        .args([
            "restore",
            "--from",
            &archive.display().to_string(),
            "--estate",
            &estate,
            "--sacred",
            &sacred,
            "--state-dir",
            &state_s,
            "--plans-dir",
            &plans_s,
        ])
        .output()
        .unwrap();
    let restored_text = text(&restored);
    assert!(restored.status.success(), "{restored_text}");
    assert!(
        restored_text.contains("writes: true"),
        "{restored_text}"
    );
    assert!(!restored_text.contains("refuse:sacred-mismatch"), "{restored_text}");
    let after = std::fs::read_to_string(state.join("placement-actual.json")).unwrap();
    assert_eq!(leases, after, "restore must put back placement-actual bytes");
    assert!(
        !after.contains("\"kind\": \"cloud-agent\"") || after.contains("\"spawned\": false"),
        "{after}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
