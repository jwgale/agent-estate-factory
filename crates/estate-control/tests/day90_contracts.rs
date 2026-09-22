//! Day 90+ contract hunts: pack leftover state, suggest never mutates,
//! restore empty sacred_ids, vanilla doctor vs --strict.
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
        "target/test-contracts-{}-{}",
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

fn estate_yaml() -> PathBuf {
    repo_root().join("examples/estate.yaml")
}

/// Vanilla doctor required files. Must not grow into --strict extras.
const VANILLA_REQUIRED: &[&str] = &[
    "schema/estate.v0.schema.json",
    "schema/pack.v0.json",
    "schema/specialist-pack.v0.json",
    "schema/placement-actual.v0.json",
    "schema/reconcile.v0.json",
    "schema/conveyor-mesh.v0.json",
    "schema/local-catalog.v0.json",
    "schema/lifecycle.v0.json",
    "schema/estate-plan.v0.json",
    "schema/feed-cursor.v0.json",
    "schema/enrich-proposal.v0.json",
    "schema/train-enrich.v0.json",
    "schema/apply-dry-run.v0.json",
    "schema/session-journal.v0.json",
    "schema/policy.v0.json",
    "schema/cell-backup.v0.json",
    "schema/sacred.v0.json",
    "policy/cell-one.policy.v0.yaml",
    "policy/sacred.yaml",
    "schema/README.md",
    "docs/GATE-90.md",
    "CHANGELOG.md",
    "docs/MORNING-BRIEF-2026-09-21.md",
    "docs/PR2-DESCRIPTION.md",
];

const STRICT_ONLY: &[&str] = &[
    "docs/DAY90-PLUS.md",
    "docs/CELL-ONE-STATUS.md",
    "docs/OPERATOR-DAY.md",
    "docs/FEED-LOOP.md",
    "examples/fixtures/dual-layer-demo.yaml",
    "scripts/feed-loop.sh",
    "crates/estate-control/tests/day90_e2e.rs",
    "crates/estate-control/tests/day90_hop.rs",
];

fn walk_json_promoted(dir: &Path, hits: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_json_promoted(&path, hits);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if text.contains("\"promoted\": true") || text.contains("\"promoted\":true") {
            hits.push(path.display().to_string());
        }
    }
}

#[test]
fn packs_propose_accept_promote_leaves_no_promoted_state() {
    let root = tmp("packs");
    let drop = root.join("drop");
    let accepted = root.join("accepted");
    let proposed = root.join("proposed");
    let estate = estate_yaml();
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
    assert!(pack.status.success(), "{}", text(&pack));

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
    assert!(propose.status.success(), "{}", text(&propose));

    let accept = estate_bin()
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
    assert!(accept.status.success(), "{}", text(&accept));

    let before_promote = snapshot_files(&root);
    let promote = estate_bin()
        .args(["packs", "promote", "--id", "overnight-traces"])
        .output()
        .unwrap();
    assert!(!promote.status.success(), "promote must refuse");
    let promo = text(&promote);
    assert!(
        promo.contains("refuse:") || promo.contains("auto-promote") || promo.contains("promote"),
        "{promo}"
    );
    assert_eq!(before_promote, snapshot_files(&root), "promote must write nothing");

    assert_eq!(before, std::fs::read_to_string(&estate).unwrap());
    assert!(!root.join("state").join("placement-actual.json").exists());
    assert!(!accepted.join("overnight-traces.pack.json").exists());
    let proposal: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(proposed.join("overnight-traces.proposal.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(proposal["auto_apply"], false);
    assert_eq!(proposal["diff"]["would_add_to_estate"], true);
    let edit = std::fs::read_to_string(accepted.join("overnight-traces.enrich-edit.md")).unwrap();
    assert!(edit.contains("applied_to_estate: false"));
    let pack_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(drop.join("overnight-traces.pack.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(pack_json["promoted"], false);
    let mut hits = Vec::new();
    walk_json_promoted(&root, &mut hits);
    assert!(hits.is_empty(), "leftover promoted=true: {hits:?}");
    let _ = std::fs::remove_dir_all(&root);
}

fn snapshot_files(dir: &Path) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    walk_files(dir, dir, &mut out);
    out.sort();
    out
}

fn walk_files(root: &Path, dir: &Path, out: &mut Vec<(String, u64)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_files(root, &path, out);
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        out.push((rel.display().to_string(), len));
    }
}

#[test]
fn reconcile_suggest_on_drift_does_not_rewrite_leases() {
    let root = tmp("suggest");
    let state = root.join("state");
    let apply = estate_bin()
        .args([
            "apply",
            "--estate",
            &estate_yaml().display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
            "--plans-dir",
            &root.join("plans").display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(apply.status.success(), "{}", text(&apply));

    let leases_path = state.join("placement-actual.json");
    let mut actual: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&leases_path).unwrap()).unwrap();
    let leases = actual["leases"].as_array_mut().expect("leases");
    leases.retain(|l| l["placement_id"] != "cell-one-box");
    std::fs::write(&leases_path, serde_json::to_string_pretty(&actual).unwrap()).unwrap();
    let before = std::fs::read_to_string(&leases_path).unwrap();

    let suggest = estate_bin()
        .args([
            "reconcile",
            "--suggest",
            "--estate",
            &estate_yaml().display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    let body = text(&suggest);
    assert!(!suggest.status.success(), "drift must fail closed: {body}");
    assert!(body.contains("auto_apply: false"), "{body}");
    assert!(
        body.contains("missing-lease") || body.contains("refuse:missing-lease"),
        "{body}"
    );
    assert!(state.join("reconcile-suggest.md").is_file());
    assert_eq!(before, std::fs::read_to_string(&leases_path).unwrap());
    let patch = std::fs::read_to_string(state.join("reconcile-suggest.md")).unwrap();
    assert!(patch.contains("auto_apply: false"));
    assert!(patch.contains("Not applied"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn restore_empty_sacred_ids_refuses_and_writes_nothing() {
    let root = tmp("restore");
    let state = root.join("state");
    let backups = root.join("backups");
    let apply = estate_bin()
        .args([
            "apply",
            "--estate",
            &estate_yaml().display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
            "--plans-dir",
            &root.join("plans").display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(apply.status.success(), "{}", text(&apply));

    let backup = estate_bin()
        .args([
            "backup",
            "--estate",
            &estate_yaml().display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &root.join("plans").display().to_string(),
            "--out",
            &backups.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(backup.status.success(), "{}", text(&backup));
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
        .expect("archive");

    let dest = root.join("dest");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("placement-actual.json"), "sentinel\n").unwrap();

    let dry_ok = estate_bin()
        .args([
            "restore",
            "--from",
            &archive.display().to_string(),
            "--estate",
            &estate_yaml().display().to_string(),
            "--state-dir",
            &root.join("dry-empty").display().to_string(),
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(dry_ok.status.success(), "{}", text(&dry_ok));
    assert!(!root.join("dry-empty").join("placement-actual.json").exists());

    let meta_path = archive.join("backup.json");
    let mut meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
    meta["sacred_ids"] = serde_json::json!([]);
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap()).unwrap();

    for dry in [true, false] {
        let dest_s = dest.display().to_string();
        let mut args = vec![
            "restore".into(),
            "--from".into(),
            archive.display().to_string(),
            "--estate".into(),
            estate_yaml().display().to_string(),
            "--state-dir".into(),
            dest_s,
        ];
        if dry {
            args.push("--dry-run".into());
        }
        let out = estate_bin().args(&args).output().unwrap();
        let body = text(&out);
        assert!(!out.status.success(), "empty sacred_ids must refuse: {body}");
        assert!(body.contains("refuse:sacred-mismatch"), "{body}");
        assert_eq!(
            std::fs::read_to_string(dest.join("placement-actual.json")).unwrap(),
            "sentinel\n"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn doctor_vanilla_stays_thinner_than_strict() {
    let watch = std::fs::read_to_string(repo_root().join("crates/estate-control/src/watch.rs"))
        .unwrap();
    let start = watch
        .find("const DOCTOR_REQUIRED")
        .expect("DOCTOR_REQUIRED");
    let block = &watch[start..];
    let end = block.find("];").expect("end");
    let required = &block[..=end];
    for extra in STRICT_ONLY {
        assert!(
            !required.contains(extra),
            "vanilla DOCTOR_REQUIRED grew into {extra}"
        );
    }
    for rel in VANILLA_REQUIRED {
        assert!(required.contains(rel), "vanilla lost {rel}");
    }

    let vanilla = estate_bin()
        .args([
            "doctor",
            "--root",
            &repo_root().display().to_string(),
            "--state-dir",
            &repo_root().join("target/test-contracts-doctor").display().to_string(),
        ])
        .output()
        .unwrap();
    let vanilla_text = text(&vanilla);
    assert!(vanilla.status.success(), "{vanilla_text}");
    assert!(vanilla_text.contains("Cell One doctor"), "{vanilla_text}");
    assert!(
        !vanilla_text.contains("Strict (pre-merge)"),
        "vanilla must not run --strict extras"
    );
    assert!(
        !vanilla_text.contains("Sanctum is not Cyera"),
        "{vanilla_text}"
    );
    assert!(!vanilla_text.contains("DAY90-PLUS"), "{vanilla_text}");

    let strict = estate_bin()
        .args([
            "doctor",
            "--strict",
            "--root",
            &repo_root().display().to_string(),
            "--state-dir",
            &repo_root().join("target/test-contracts-doctor").display().to_string(),
        ])
        .output()
        .unwrap();
    let strict_text = text(&strict);
    assert!(strict.status.success(), "{strict_text}");
    assert!(strict_text.contains("Cell One doctor"), "{strict_text}");
    assert!(strict_text.contains("Strict (pre-merge)"), "{strict_text}");
    assert!(strict_text.contains("Sanctum is not Cyera"), "{strict_text}");
    assert!(strict_text.contains("DAY90-PLUS"), "{strict_text}");

    let slim = tmp("doctor-slim");
    for rel in VANILLA_REQUIRED {
        let src = repo_root().join(rel);
        let dest = slim.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::copy(&src, &dest).unwrap();
    }
    let wf = slim.join(".github/workflows");
    std::fs::create_dir_all(&wf).unwrap();
    std::fs::copy(
        repo_root().join(".github/workflows/ci.yml"),
        wf.join("ci.yml"),
    )
    .unwrap();

    let slim_vanilla = estate_bin()
        .args([
            "doctor",
            "--root",
            &slim.display().to_string(),
            "--state-dir",
            &slim.join("cell").display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        slim_vanilla.status.success(),
        "vanilla must pass on schema+CI root: {}",
        text(&slim_vanilla)
    );
    assert!(!text(&slim_vanilla).contains("Strict (pre-merge)"));

    let slim_strict = estate_bin()
        .args([
            "doctor",
            "--strict",
            "--root",
            &slim.display().to_string(),
            "--state-dir",
            &slim.join("cell").display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        !slim_strict.status.success(),
        "strict must fail without demo/feed-loop/DAY90-PLUS"
    );
    let slim_strict_text = text(&slim_strict);
    assert!(
        slim_strict_text.contains("FAIL") || slim_strict_text.contains("strict"),
        "{slim_strict_text}"
    );
    let _ = std::fs::remove_dir_all(&slim);
}
