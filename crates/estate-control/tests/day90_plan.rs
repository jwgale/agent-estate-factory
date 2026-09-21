//! Day 90+ plan export-pr / diff --allow-wider exits, dry-run refuse
//! writes, curator clap vs refuse:curator. Isolated. Cloud never spawned.

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
        "target/test-plan-{}-{}",
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

fn walk_files(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
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
        out.push(rel.display().to_string());
    }
}

fn narrow_plan() -> &'static str {
    r#"{
      "schema": "cell-one.plan.v0",
      "desired_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "against_hash": null,
      "added": {"agents": [], "lanes": [], "intentions": [], "model_bindings": [], "placements": [], "enrich_packs": []},
      "removed": {"agents": [], "lanes": [], "intentions": [], "model_bindings": [], "placements": [], "enrich_packs": []},
      "changed": {"agents": [], "lanes": [], "intentions": [], "model_bindings": [], "placements": [], "enrich_packs": []},
      "blast_radius_text": "empty",
      "created_at": "unix:1"
    }"#
}

fn wide_plan() -> &'static str {
    r#"{
      "schema": "cell-one.plan.v0",
      "desired_hash": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "against_hash": null,
      "added": {"agents": ["horizon", "research"], "lanes": ["horizon"], "intentions": [], "model_bindings": [], "placements": [], "enrich_packs": []},
      "removed": {"agents": [], "lanes": [], "intentions": [], "model_bindings": [], "placements": [], "enrich_packs": []},
      "changed": {"agents": [], "lanes": [], "intentions": [], "model_bindings": [], "placements": [], "enrich_packs": []},
      "blast_radius_text": "wider",
      "created_at": "unix:2"
    }"#
}

#[test]
fn plan_diff_allow_wider_and_export_pr_exits() {
    let root = tmp("diff");
    let state = root.join("state");
    let from_p = root.join("from.json");
    let to_p = root.join("to.json");
    std::fs::write(&from_p, narrow_plan()).unwrap();
    std::fs::write(&to_p, wide_plan()).unwrap();

    let wider = estate_bin()
        .args([
            "plan",
            "diff",
            "--from",
            &from_p.display().to_string(),
            "--to",
            &to_p.display().to_string(),
        ])
        .output()
        .unwrap();
    let wider_text = text(&wider);
    assert!(!wider.status.success(), "{wider_text}");
    assert!(wider_text.contains("refuse:wider"), "{wider_text}");

    let allowed = estate_bin()
        .args([
            "plan",
            "diff",
            "--from",
            &from_p.display().to_string(),
            "--to",
            &to_p.display().to_string(),
            "--allow-wider",
        ])
        .output()
        .unwrap();
    let allowed_text = text(&allowed);
    assert!(allowed.status.success(), "{allowed_text}");
    assert!(allowed_text.contains("wider allowed"), "{allowed_text}");

    let same = estate_bin()
        .args([
            "plan",
            "diff",
            "--from",
            &from_p.display().to_string(),
            "--to",
            &from_p.display().to_string(),
        ])
        .output()
        .unwrap();
    let same_text = text(&same);
    assert!(same.status.success(), "{same_text}");
    assert!(same_text.contains("plan diff ok"), "{same_text}");

    let narrower = estate_bin()
        .args([
            "plan",
            "diff",
            "--from",
            &to_p.display().to_string(),
            "--to",
            &from_p.display().to_string(),
            "--allow-wider",
        ])
        .output()
        .unwrap();
    let narrower_text = text(&narrower);
    assert!(
        narrower.status.success(),
        "allow-wider on a narrower diff must still exit 0: {narrower_text}"
    );
    assert!(
        !narrower_text.contains("refuse:wider:"),
        "{narrower_text}"
    );

    let missing = estate_bin()
        .args([
            "plan",
            "diff",
            "--allow-wider",
            "--estate",
            &estate_yaml().display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &root.join("plans").display().to_string(),
        ])
        .output()
        .unwrap();
    let missing_text = text(&missing);
    assert!(
        !missing.status.success(),
        "--allow-wider must not skip a missing last-applied plan: {missing_text}"
    );
    assert!(
        missing_text.contains("pass --from") || missing_text.contains("no last-applied"),
        "{missing_text}"
    );

    let pr = root.join("PR.md");
    let exported = estate_bin()
        .args([
            "plan",
            "export-pr",
            "--estate",
            &estate_yaml().display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &root.join("plans").display().to_string(),
            "--out",
            &pr.display().to_string(),
        ])
        .output()
        .unwrap();
    let exported_text = text(&exported);
    assert!(
        exported.status.success(),
        "export-pr is a paste helper and must exit 0 with risks listed: {exported_text}"
    );
    let body = std::fs::read_to_string(&pr).unwrap();
    assert!(body.contains("Refuse risks"));
    assert!(body.contains("no covering plan") || body.contains("gated apply will refuse"));
    assert!(!state.join("placement-actual.json").exists());
    assert!(!state.join("desired-snapshot.yaml").exists());
    let state_files = walk_files(&state);
    assert!(
        state_files.is_empty(),
        "export-pr must not write .cell: {state_files:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn apply_dry_run_refuse_cases_do_not_mutate_cell() {
    let root = tmp("dry");
    let state = root.join("state");
    let plans = root.join("plans");

    let gated = estate_bin()
        .args([
            "apply",
            "--dry-run",
            "--require-plan",
            "--estate",
            &estate_yaml().display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let gated_text = text(&gated);
    assert!(!gated.status.success(), "{gated_text}");
    assert!(
        gated_text.contains("would-refuse") || gated_text.contains("refuse:no-plan"),
        "{gated_text}"
    );
    assert!(
        walk_files(&state).is_empty(),
        "dry-run --require-plan must not create .cell: {:?}",
        walk_files(&state)
    );

    let deny = estate_bin()
        .args([
            "apply",
            "--dry-run",
            "--estate",
            &estate_yaml().display().to_string(),
            "--state-dir",
            &root.join("deny-state").display().to_string(),
            "--roots-base",
            &root.display().to_string(),
            "--plans-dir",
            &root.join("deny-plans").display().to_string(),
            "--policy",
            &repo_root()
                .join("examples/fixtures/policy-deny.yaml")
                .display()
                .to_string(),
        ])
        .output()
        .unwrap();
    assert!(!deny.status.success(), "{}", text(&deny));
    assert!(
        walk_files(&root.join("deny-state")).is_empty(),
        "policy-deny dry-run must not write .cell"
    );

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
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(apply.status.success(), "{}", text(&apply));
    let before = std::fs::read_to_string(state.join("placement-actual.json")).unwrap();
    let before_walk = walk_files(&state);

    let mut actual: serde_json::Value = serde_json::from_str(&before).unwrap();
    if let Some(leases) = actual["leases"].as_array_mut() {
        for lease in leases {
            lease["issued_at"] = serde_json::json!(1);
            lease["expires_at"] = serde_json::json!(2);
            lease["ttl_secs"] = serde_json::json!(1);
        }
    }
    std::fs::write(
        state.join("placement-actual.json"),
        serde_json::to_string_pretty(&actual).unwrap(),
    )
    .unwrap();
    let expired_bytes = std::fs::read_to_string(state.join("placement-actual.json")).unwrap();

    let expired = estate_bin()
        .args([
            "apply",
            "--dry-run",
            "--estate",
            &estate_yaml().display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let expired_text = text(&expired);
    assert!(!expired.status.success(), "{expired_text}");
    assert!(
        expired_text.contains("refuse:expired") || expired_text.contains("would-refuse"),
        "{expired_text}"
    );
    assert_eq!(
        expired_bytes,
        std::fs::read_to_string(state.join("placement-actual.json")).unwrap()
    );
    assert_eq!(before_walk, walk_files(&state));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn curator_missing_vs_wrong_is_consistent() {
    let root = tmp("curator");
    let drop = root.join("drop");
    let accepted = root.join("accepted");
    let proposed = root.join("proposed");
    std::fs::create_dir_all(&drop).unwrap();
    std::fs::copy(
        repo_root().join("examples/fixtures/overnight-traces.pack.json"),
        drop.join("overnight-traces.pack.json"),
    )
    .unwrap();

    let feed_default = estate_bin()
        .args([
            "feed",
            "import",
            "--id",
            "overnight-traces",
            "--drop-dir",
            &drop.display().to_string(),
            "--accepted-dir",
            &root.join("feed-accepted").display().to_string(),
            "--estate",
            &estate_yaml().display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        feed_default.status.success(),
        "feed import defaults --curator jason: {}",
        text(&feed_default)
    );

    let feed_bad = estate_bin()
        .args([
            "feed",
            "import",
            "--id",
            "overnight-traces",
            "--drop-dir",
            &drop.display().to_string(),
            "--accepted-dir",
            &root.join("feed-bad").display().to_string(),
            "--estate",
            &estate_yaml().display().to_string(),
            "--curator",
            "robot",
        ])
        .output()
        .unwrap();
    let feed_bad_text = text(&feed_bad);
    assert!(!feed_bad.status.success(), "{feed_bad_text}");
    assert!(feed_bad_text.contains("refuse:curator"), "{feed_bad_text}");

    let packs_bad = estate_bin()
        .args([
            "packs",
            "import",
            "--id",
            "overnight-traces",
            "--drop-dir",
            &drop.display().to_string(),
            "--accepted-dir",
            &accepted.display().to_string(),
            "--estate",
            &estate_yaml().display().to_string(),
            "--curator",
            "not-jason",
        ])
        .output()
        .unwrap();
    let packs_bad_text = text(&packs_bad);
    assert!(!packs_bad.status.success(), "{packs_bad_text}");
    assert!(packs_bad_text.contains("refuse:curator"), "{packs_bad_text}");

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
            &estate_yaml().display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(propose.status.success(), "{}", text(&propose));

    let accept_missing = estate_bin()
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
            &estate_yaml().display().to_string(),
        ])
        .output()
        .unwrap();
    let accept_missing_text = text(&accept_missing);
    assert!(
        !accept_missing.status.success(),
        "accept without --curator must fail clap: {accept_missing_text}"
    );
    assert!(
        !accept_missing_text.contains("refuse:curator"),
        "missing accept --curator is clap, not refuse:curator: {accept_missing_text}"
    );

    let accept_bad = estate_bin()
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
            &estate_yaml().display().to_string(),
        ])
        .output()
        .unwrap();
    let accept_bad_text = text(&accept_bad);
    assert!(!accept_bad.status.success(), "{accept_bad_text}");
    assert!(
        accept_bad_text.contains("refuse:curator"),
        "{accept_bad_text}"
    );

    let apply_bad = estate_bin()
        .args([
            "apply",
            "--import-pack",
            "overnight-traces",
            "--curator",
            "robot",
            "--packs-dir",
            &drop.display().to_string(),
            "--estate",
            &estate_yaml().display().to_string(),
            "--state-dir",
            &root.join("apply-state").display().to_string(),
            "--roots-base",
            &root.display().to_string(),
            "--plans-dir",
            &root.join("apply-plans").display().to_string(),
        ])
        .output()
        .unwrap();
    let apply_bad_text = text(&apply_bad);
    assert!(!apply_bad.status.success(), "{apply_bad_text}");
    assert!(apply_bad_text.contains("refuse:curator"), "{apply_bad_text}");
    assert!(
        !root.join("apply-state").join("placement-actual.json").exists(),
        "wrong curator on apply --import-pack must not write leases"
    );
    assert!(
        !root.join("apply-state").join("apply-audit.jsonl").exists(),
        "wrong curator on apply --import-pack must not write apply-audit"
    );
    assert!(
        !drop.join("accepted").join("overnight-traces.pack.json").exists(),
        "wrong curator on apply --import-pack must not write accepted pack"
    );

    let dry = estate_bin()
        .args([
            "apply",
            "--dry-run",
            "--import-pack",
            "overnight-traces",
            "--curator",
            "robot",
            "--packs-dir",
            &drop.display().to_string(),
            "--estate",
            &estate_yaml().display().to_string(),
            "--state-dir",
            &root.join("dry-state").display().to_string(),
            "--roots-base",
            &root.display().to_string(),
            "--plans-dir",
            &root.join("dry-plans").display().to_string(),
        ])
        .output()
        .unwrap();
    let dry_text = text(&dry);
    assert!(!dry.status.success(), "{dry_text}");
    assert!(dry_text.contains("refuse:curator"), "{dry_text}");
    assert!(
        !root.join("dry-state").join("placement-actual.json").exists(),
        "dry-run wrong curator must not write leases"
    );
    let _ = std::fs::remove_dir_all(&root);
}
