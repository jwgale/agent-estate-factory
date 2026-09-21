//! Day 90+ feed-loop CLI walk. Estate stays unchanged. Cursor stays durable.
//! Fixtures only. No live Grok / Mac / GPU.

use feed_collector::{append_event, ScrubbedEvent};
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
        "target/test-feed-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn seed(feed: &std::path::Path) {
    append_event(
        feed,
        &ScrubbedEvent {
            kind: "model.local.precheck".into(),
            agent_id: Some("research".into()),
            decision: Some("allow".into()),
            object_class: Some("local".into()),
            note: Some("job=policy-precheck api_key=sk-ant-not-a-real-key".into()),
            ts: String::new(),
        },
    )
    .unwrap();
    append_event(
        feed,
        &ScrubbedEvent {
            kind: "model.frontier.complete".into(),
            agent_id: Some("horizon".into()),
            decision: Some("allow".into()),
            object_class: Some("frontier".into()),
            note: Some("bytes=4".into()),
            ts: String::new(),
        },
    )
    .unwrap();
}

#[test]
fn feed_loop_pack_propose_accept_keeps_estate_and_cursor() {
    let root = tmp("loop");
    let feed = root.join("feed");
    let drop = root.join("drop");
    let accepted = root.join("accepted");
    let proposed = root.join("proposed");
    let estate = repo_root().join("examples/estate.yaml");
    seed(&feed);
    let events = std::fs::read_to_string(feed.join("events.jsonl")).unwrap();
    assert!(events.contains("[redacted]"));
    assert!(!events.contains("sk-ant-not-a-real-key"));
    let before = std::fs::read_to_string(&estate).unwrap();

    let pack = estate_bin()
        .args([
            "feed",
            "pack",
            "--feed-dir",
            &feed.display().to_string(),
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

    let cursor = estate_bin()
        .args([
            "feed",
            "cursor",
            "--feed-dir",
            &feed.display().to_string(),
        ])
        .output()
        .unwrap();
    let cursor_text = format!(
        "{}{}",
        String::from_utf8_lossy(&cursor.stdout),
        String::from_utf8_lossy(&cursor.stderr)
    );
    assert!(cursor.status.success(), "{cursor_text}");
    assert!(cursor_text.contains("cell-one.feed-cursor.v0"));
    assert!(cursor_text.contains("overnight-traces"));
    let disk = std::fs::read_to_string(feed.join("feed-cursor.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&disk).unwrap();
    assert!(parsed["events"].as_u64().unwrap_or(0) >= 2);
    assert_eq!(
        parsed["packed_id"].as_str(),
        Some("overnight-traces")
    );

    let again = estate_bin()
        .args([
            "feed",
            "pack",
            "--feed-dir",
            &feed.display().to_string(),
            "--drop-dir",
            &drop.display().to_string(),
            "--id",
            "overnight-traces",
        ])
        .output()
        .unwrap();
    assert!(again.status.success());
    assert!(feed.join("feed-cursor.json").is_file());

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
    let accept_text = format!(
        "{}{}",
        String::from_utf8_lossy(&accept.stdout),
        String::from_utf8_lossy(&accept.stderr)
    );
    assert!(accept.status.success(), "{accept_text}");
    assert!(accept_text.contains("auto_apply: false"));
    assert!(accept_text.contains("applied_to_estate: false"));

    let promote = estate_bin()
        .args(["packs", "promote", "--id", "overnight-traces"])
        .output()
        .unwrap();
    assert!(!promote.status.success(), "promote must refuse");

    assert_eq!(before, std::fs::read_to_string(&estate).unwrap());
    assert!(feed.join("feed-cursor.json").is_file());
    let _ = std::fs::remove_dir_all(&root);
}
