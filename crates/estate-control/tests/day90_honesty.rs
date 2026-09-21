//! After the SKU series: swallows, feed redaction, append-only journals.
//! Isolated cell. No new verb. Cloud never spawned.

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

fn apply_cell(estate: &str, state: &str, roots: &str, plans: &str) {
    let out = estate_bin()
        .args([
            "apply",
            "--estate",
            estate,
            "--state-dir",
            state,
            "--roots-base",
            roots,
            "--plans-dir",
            plans,
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn journals_append_only_under_suspend_resume_expire_forget() {
    let root = repo_root().join(format!("target/test-honesty-jsonl-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let roots = root.join("roots");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();
    let estate = fixture("examples/estate.yaml");
    apply_cell(
        &estate,
        &state_s,
        &roots.display().to_string(),
        &plans.display().to_string(),
    );

    let life0 = std::fs::read_to_string(state.join("lifecycle.jsonl")).unwrap();
    let sess0 = std::fs::read_to_string(state.join("sessions.jsonl")).unwrap();
    assert!(!life0.is_empty());
    assert!(!sess0.is_empty());

    let suspend = estate_bin()
        .args(["suspend", "--state-dir", &state_s])
        .output()
        .unwrap();
    assert!(suspend.status.success(), "{}", text(&suspend));
    let life1 = std::fs::read_to_string(state.join("lifecycle.jsonl")).unwrap();
    let sess1 = std::fs::read_to_string(state.join("sessions.jsonl")).unwrap();
    assert!(life1.starts_with(&life0), "suspend truncated lifecycle.jsonl");
    assert!(sess1.starts_with(&sess0), "suspend truncated sessions.jsonl");

    let resume = estate_bin()
        .args([
            "resume",
            "--estate",
            &estate,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(resume.status.success(), "{}", text(&resume));
    let life2 = std::fs::read_to_string(state.join("lifecycle.jsonl")).unwrap();
    let sess2 = std::fs::read_to_string(state.join("sessions.jsonl")).unwrap();
    assert!(life2.starts_with(&life1), "resume truncated lifecycle.jsonl");
    assert!(sess2.starts_with(&sess1), "resume truncated sessions.jsonl");

    let forget = estate_bin()
        .args(["expire", "--forget", "--state-dir", &state_s])
        .output()
        .unwrap();
    assert!(
        forget.status.success() || text(&forget).contains("no expired"),
        "{}",
        text(&forget)
    );
    let life3 = std::fs::read_to_string(state.join("lifecycle.jsonl")).unwrap();
    let sess3 = std::fs::read_to_string(state.join("sessions.jsonl")).unwrap();
    assert_eq!(life3, life2, "expire --forget must not rewrite lifecycle.jsonl");
    assert_eq!(sess3, sess2, "expire --forget must not rewrite sessions.jsonl");

    let mut places: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(state.join("placement-actual.json")).unwrap())
            .unwrap();
    if let Some(leases) = places["leases"].as_array_mut() {
        for lease in leases {
            lease["expires_at"] = serde_json::json!(1);
        }
    }
    std::fs::write(
        state.join("placement-actual.json"),
        serde_json::to_string_pretty(&places).unwrap(),
    )
    .unwrap();
    let forget_expired = estate_bin()
        .args(["expire", "--forget", "--state-dir", &state_s])
        .output()
        .unwrap();
    assert!(forget_expired.status.success(), "{}", text(&forget_expired));
    let life4 = std::fs::read_to_string(state.join("lifecycle.jsonl")).unwrap();
    let sess4 = std::fs::read_to_string(state.join("sessions.jsonl")).unwrap();
    assert_eq!(
        life4, life3,
        "expire --forget of elapsed leases must not truncate lifecycle.jsonl"
    );
    assert_eq!(
        sess4, sess3,
        "expire --forget of elapsed leases must not truncate sessions.jsonl"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn restore_garbage_estate_file_refuses() {
    let root = repo_root().join(format!(
        "target/test-honesty-estate-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let roots = root.join("roots");
    let backups = root.join("backups");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture("examples/estate.yaml");
    apply_cell(
        &estate,
        &state.display().to_string(),
        &roots.display().to_string(),
        &plans.display().to_string(),
    );
    let backup = estate_bin()
        .args([
            "backup",
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--out",
            &backups.display().to_string(),
            "--estate",
            &estate,
            "--policy",
            &fixture("policy/cell-one.policy.v0.yaml"),
        ])
        .output()
        .unwrap();
    assert!(backup.status.success(), "{}", text(&backup));
    let archive = std::fs::read_dir(&backups)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("cell-backup-"))
                .unwrap_or(false)
        })
        .expect("cell-backup archive");
    let dest = root.join("restored");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("placement-actual.json"), "sentinel\n").unwrap();
    let garbage = root.join("garbage-estate.yaml");
    std::fs::write(&garbage, "not: [valid estate\n").unwrap();
    let restore = estate_bin()
        .args([
            "restore",
            "--from",
            &archive.display().to_string(),
            "--state-dir",
            &dest.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--estate",
            &garbage.display().to_string(),
            "--policy",
            &fixture("policy/cell-one.policy.v0.yaml"),
        ])
        .output()
        .unwrap();
    let body = text(&restore);
    assert!(!restore.status.success(), "{body}");
    assert_eq!(
        std::fs::read_to_string(dest.join("placement-actual.json")).unwrap(),
        "sentinel\n",
        "garbage estate must not restore: {body}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
