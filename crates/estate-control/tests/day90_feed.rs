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
    let packed: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(drop.join("overnight-traces.pack.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(packed["promoted"], false);
    assert_eq!(
        packed["source_drivers"],
        serde_json::json!(["frontier", "local"])
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
    let proposal: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(proposed.join("overnight-traces.proposal.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(proposal["auto_apply"], false);
    assert_eq!(
        proposal["diff"]["source_drivers"],
        serde_json::json!(["frontier", "local"])
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
    assert!(
        accept_text.contains("source_drivers: frontier, local"),
        "{accept_text}"
    );
    let edit: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(accepted.join("overnight-traces.enrich-edit.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(edit["auto_apply"], false);
    assert_eq!(edit["applied_to_estate"], false);
    assert_eq!(
        edit["source_drivers"],
        serde_json::json!(["frontier", "local"])
    );
    let edit_md = std::fs::read_to_string(accepted.join("overnight-traces.enrich-edit.md")).unwrap();
    assert!(
        edit_md.contains("source_drivers: frontier, local"),
        "{edit_md}"
    );

    let promote = estate_bin()
        .args(["packs", "promote", "--id", "overnight-traces"])
        .output()
        .unwrap();
    assert!(!promote.status.success(), "promote must refuse");

    assert_eq!(before, std::fs::read_to_string(&estate).unwrap());
    assert!(feed.join("feed-cursor.json").is_file());
    let index = std::fs::read_to_string(drop.join("INDEX.md")).unwrap();
    assert!(index.contains("drivers=frontier,local"), "{index}");
    let list = estate_bin()
        .args(["feed", "list", "--drop-dir", &drop.display().to_string()])
        .output()
        .unwrap();
    let list_text = format!(
        "{}{}",
        String::from_utf8_lossy(&list.stdout),
        String::from_utf8_lossy(&list.stderr)
    );
    assert!(list.status.success(), "{list_text}");
    assert!(list_text.contains("drivers=frontier,local"), "{list_text}");
    assert!(list_text.contains("promoted=false"), "{list_text}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn feed_loop_script_asserts_source_drivers_without_live_keys() {
    let root = repo_root();
    let script = std::fs::read_to_string(root.join("scripts/feed-loop.sh")).unwrap();
    assert!(script.contains("source_drivers"), "{script}");
    assert!(script.contains("drivers=frontier,local"), "{script}");
    assert!(script.contains("unset XAI_API_KEY"), "{script}");
    assert!(
        script.contains("Do not add to make smoke or GitHub Actions"),
        "feed-loop must stay off smoke / Actions"
    );
    let smoke = std::fs::read_to_string(root.join("scripts/smoke.sh")).unwrap();
    let gate = std::fs::read_to_string(root.join("scripts/day90-gate.sh")).unwrap();
    assert!(!smoke.contains("feed-loop.sh"), "smoke must not run feed-loop");
    assert!(!gate.contains("feed-loop.sh"), "gate-90 must not run feed-loop");

    let built = Command::new("cargo")
        .args(["build", "-p", "model-estate", "--bin", "model-estate"])
        .current_dir(&root)
        .status()
        .unwrap();
    assert!(built.success(), "model-estate build failed");
    let model = root.join("target/debug/model-estate");
    let work = root.join(format!(
        "target/test-feed-loop-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&work);
    let out = Command::new("bash")
        .arg(root.join("scripts/feed-loop.sh"))
        .current_dir(&root)
        .env("ESTATE_BIN", env!("CARGO_BIN_EXE_estate"))
        .env("MODEL_ESTATE_BIN", &model)
        .env("WORKDIR", &work)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "{text}");
    assert!(text.contains("FEED-LOOP GREEN"), "{text}");
    assert!(
        text.contains("source_drivers frontier, local"),
        "{text}"
    );
    let pack: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(work.join("packs/overnight-traces.pack.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(pack["promoted"], false);
    assert_eq!(pack["source_drivers"], serde_json::json!(["frontier", "local"]));
    let proposal: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(work.join("packs/proposed/overnight-traces.proposal.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(proposal["auto_apply"], false);
    assert_eq!(
        proposal["diff"]["source_drivers"],
        serde_json::json!(["frontier", "local"])
    );
    let index = std::fs::read_to_string(work.join("packs/INDEX.md")).unwrap();
    assert!(index.contains("drivers=frontier,local"), "{index}");
    let edit: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(work.join("packs/accepted/overnight-traces.enrich-edit.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        edit["source_drivers"],
        serde_json::json!(["frontier", "local"])
    );
    assert_eq!(edit["applied_to_estate"], false);
    let _ = std::fs::remove_dir_all(&work);
}
