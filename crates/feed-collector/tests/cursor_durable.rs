//! Feed cursor durability. Scrub → pack → rematerialize.
//! Does not grow lib.rs. No live Grok / GPU.

use feed_collector::{
    append_event, load_cursor, materialize_from_feed, read_events, refuse_promote, write_cursor,
    FeedCursor, ScrubbedEvent,
};

fn tmp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "cell-one-cursor-{}-{}-{}",
        name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn event(kind: &str, agent: &str, class: &str, note: Option<&str>) -> ScrubbedEvent {
    ScrubbedEvent {
        kind: kind.into(),
        agent_id: Some(agent.into()),
        decision: Some("allow".into()),
        object_class: Some(class.into()),
        note: note.map(|s| s.into()),
        ts: String::new(),
    }
}

#[test]
fn cursor_serde_roundtrip_preserves_schema() {
    let cursor = FeedCursor {
        schema: "cell-one.feed-cursor.v0".into(),
        events: 3,
        last_ts: Some("2026-09-21T00:00:00Z".into()),
        last_kind: Some("model.local.precheck".into()),
        packed_id: Some("overnight-traces".into()),
        updated_at: "2026-09-21T00:00:01Z".into(),
    };
    let json = serde_json::to_string(&cursor).unwrap();
    let reloaded: FeedCursor = serde_json::from_str(&json).unwrap();
    assert_eq!(cursor, reloaded);
    assert_eq!(reloaded.schema, "cell-one.feed-cursor.v0");
    assert_eq!(reloaded.packed_id.as_deref(), Some("overnight-traces"));
}

#[test]
fn append_scrubs_secret_note_and_writes_cursor() {
    let dir = tmp("scrub");
    append_event(
        &dir,
        &event(
            "model.local.precheck",
            "research",
            "local",
            Some("job=policy-precheck api_key=sk-ant-not-a-real-key"),
        ),
    )
    .unwrap();
    let stored = read_events(&dir).unwrap();
    assert_eq!(stored.len(), 1);
    let note = stored[0].note.as_deref().unwrap_or("");
    assert!(
        note.contains("[redacted]"),
        "secret-shaped note must be scrubbed, got {note}"
    );
    assert!(
        !note.contains("sk-ant-not-a-real-key"),
        "raw secret must not land in events.jsonl"
    );
    let cursor = load_cursor(&dir).unwrap().expect("cursor after append");
    assert_eq!(cursor.schema, "cell-one.feed-cursor.v0");
    assert_eq!(cursor.events, 1);
    assert_eq!(cursor.packed_id, None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pack_sets_packed_id_and_rematerialize_keeps_cursor() {
    let feed = tmp("pack");
    append_event(
        &feed,
        &event("model.local.precheck", "research", "local", Some("job=policy-precheck")),
    )
    .unwrap();
    append_event(
        &feed,
        &event(
            "model.frontier.complete",
            "horizon",
            "frontier",
            Some("bytes=4"),
        ),
    )
    .unwrap();
    let drop = feed.join("drop");
    let (pack, _) = materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
    assert!(!pack.promoted);
    let cursor = load_cursor(&feed).unwrap().expect("cursor after pack");
    assert_eq!(cursor.schema, "cell-one.feed-cursor.v0");
    assert_eq!(cursor.events, 2);
    assert_eq!(cursor.packed_id.as_deref(), Some("overnight-traces"));

    let (again, _) = materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
    assert!(!again.promoted);
    assert!(refuse_promote(&again.id).is_err());
    let after = load_cursor(&feed).unwrap().expect("cursor after rematerialize");
    assert_eq!(after.schema, "cell-one.feed-cursor.v0");
    assert_eq!(after.events, 2);
    assert_eq!(after.packed_id.as_deref(), Some("overnight-traces"));
    assert!(feed.join("feed-cursor.json").is_file());
    let _ = std::fs::remove_dir_all(&feed);
}

#[test]
fn rematerialize_does_not_auto_promote_or_drop_written_cursor() {
    let feed = tmp("keep");
    append_event(
        &feed,
        &event("proxy.tool", "horizon", "tool", None),
    )
    .unwrap();
    let drop = feed.join("drop");
    materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
    let first = std::fs::read_to_string(feed.join("feed-cursor.json")).unwrap();
    let parsed: FeedCursor = serde_json::from_str(&first).unwrap();
    write_cursor(&feed, &parsed).unwrap();
    materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
    let kept = load_cursor(&feed).unwrap().expect("durable cursor");
    assert_eq!(kept.schema, "cell-one.feed-cursor.v0");
    assert_eq!(kept.packed_id.as_deref(), Some("overnight-traces"));
    assert!(refuse_promote("overnight-traces").is_err());
    let _ = std::fs::remove_dir_all(&feed);
}

#[test]
fn garbage_cursor_is_refuse_not_empty() {
    let dir = tmp("garbage");
    let path = dir.join("feed-cursor.json");
    std::fs::write(&path, "not-json\n").unwrap();
    let err = load_cursor(&dir).unwrap_err();
    assert!(
        err.to_string().contains("feed-cursor.json"),
        "{err}"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "not-json\n");
    assert!(load_cursor(&dir.join("missing")).unwrap().is_none());

    let ok = FeedCursor {
        schema: "cell-one.feed-cursor.v0".into(),
        events: 1,
        last_ts: Some("2026-09-21T00:00:00Z".into()),
        last_kind: Some("proxy.tool".into()),
        packed_id: None,
        updated_at: "2026-09-21T00:00:01Z".into(),
    };
    write_cursor(&dir, &ok).unwrap();
    let loaded = load_cursor(&dir).unwrap().expect("rewritten cursor");
    assert_eq!(loaded.schema, "cell-one.feed-cursor.v0");
    assert_eq!(loaded.events, 1);
    let blob = std::fs::read_to_string(&path).unwrap();
    assert!(!blob.trim().is_empty());
    assert!(blob.contains("cell-one.feed-cursor.v0"));
    let _ = std::fs::remove_dir_all(&dir);
}
