use super::feed_test_support::{bound_estate, tmp};
use super::*;

use super::feed_test_support::Boom;

#[test]
fn serialize_helpers_refuse_boom_empty_and_object() {
    let boom = to_json(&Boom).unwrap_err();
    assert!(boom.to_string().contains("serialize"), "{boom}");
    assert!(to_pretty_json(&Boom).is_err());
    assert!(to_jsonl_line(&Boom).is_err());
    assert!(refuse_empty_blob("").is_err());
    assert!(refuse_empty_blob("  \n").is_err());
    let empty_obj = to_jsonl_line(&serde_json::json!({})).unwrap_err();
    assert!(
        empty_obj.to_string().contains("empty"),
        "{empty_obj}"
    );
}

#[test]
fn append_event_never_writes_empty_object_line() {
    let dir = tmp();
    append_event(
        &dir,
        &ScrubbedEvent {
            kind: "proxy.tool".into(),
            agent_id: Some("horizon".into()),
            decision: Some("allow".into()),
            object_class: Some("tool".into()),
            note: None,
            ts: String::new(),
        },
    )
    .unwrap();
    let text = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
    assert!(!text.trim().is_empty());
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        assert_ne!(line, "{}", "append_event must not invent empty junk");
        let _: serde_json::Value = serde_json::from_str(line).unwrap();
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn appends_jsonl() {
    let dir = tmp();
    append_event(
        &dir,
        &ScrubbedEvent {
            kind: "proxy.tool".into(),
            agent_id: Some("horizon".into()),
            decision: Some("deny".into()),
            object_class: Some("tool".into()),
            note: None,
            ts: String::new(),
        },
    )
    .unwrap();
    let text = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
    assert!(text.contains("proxy.tool"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pack_from_frontier_and_local_is_not_promoted() {
    let feed = tmp();
    append_event(
        &feed,
        &ScrubbedEvent {
            kind: "model.local.precheck".into(),
            agent_id: Some("research".into()),
            decision: Some("allow".into()),
            object_class: Some("local".into()),
            note: Some("job=policy-precheck".into()),
            ts: String::new(),
        },
    )
    .unwrap();
    append_event(
        &feed,
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
    let drop = feed.join("drop");
    let (pack, path) = materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
    assert!(!pack.promoted);
    assert_eq!(pack.curator, "jason");
    assert_eq!(pack.policy, "manual");
    assert_eq!(pack.from_events, 2);
    assert_eq!(pack.schema, "cell-one.pack.v0");
    assert_eq!(pack.host_class, "any");
    assert!(pack.paths.iter().any(|p| p == "local"));
    assert!(pack.paths.iter().any(|p| p == "frontier"));
    assert_eq!(pack.path_counts.local, 1);
    assert_eq!(pack.path_counts.frontier, 1);
    assert_eq!(pack.source_drivers, vec!["frontier".to_string(), "local".to_string()]);
    assert!(!pack.promoted);
    assert!(feed.join("feed-cursor.json").is_file());
    assert!(path.ends_with("overnight-traces.pack.json"));
    assert!(refuse_promote(&pack.id).is_err());
    let listed = list_drop_packs(&drop).unwrap();
    assert_eq!(listed.len(), 1);
    let accepted = feed.join("accepted");
    let (imported, _) =
        import_pack(&drop, &accepted, "overnight-traces", &[], &bound_estate()).unwrap();
    assert!(!imported.estate_bound);
    assert!(!imported.pack.promoted);
    assert!(accepted.join("import-audit.jsonl").is_file());
    let _ = std::fs::remove_dir_all(&feed);
}

#[test]
fn local_down_tags_local_and_not_frontier() {
    let pack = pack_from_events(
        "local-down",
        &[ScrubbedEvent {
            kind: "model.local.down".into(),
            agent_id: Some("research".into()),
            decision: Some("deny".into()),
            object_class: Some("local".into()),
            note: Some("fail-closed; no frontier fallback".into()),
            ts: String::new(),
        }],
    );
    assert_eq!(pack.source_drivers, vec!["local".to_string()]);
    assert_eq!(
        classify_path(&ScrubbedEvent {
            kind: "model.frontier.complete".into(),
            agent_id: Some("research".into()),
            decision: Some("deny".into()),
            object_class: Some("local".into()),
            note: Some("no frontier fallback".into()),
            ts: String::new(),
        }),
        "local"
    );
    assert_eq!(pack.path_counts.frontier, 0);
    assert_eq!(pack.path_counts.local, 1);
    assert!(!pack.promoted);
    assert!(refuse_pack(&pack).is_ok());
}

#[test]
fn source_driver_tag_must_match_counts_and_stay_unpromoted() {
    let mut pack = pack_from_events("local-only", &[]);
    assert!(pack.source_drivers.is_empty());
    assert!(!pack.promoted);
    pack.path_counts.local = 1;
    let missing = refuse_pack(&pack).unwrap_err();
    assert!(
        missing.to_string().contains("source-driver"),
        "{missing}"
    );
    pack.source_drivers = vec!["local".into()];
    refuse_pack(&pack).unwrap();
    pack.source_drivers = vec!["ollama".into()];
    assert!(refuse_pack(&pack).is_err());
    pack.source_drivers = vec!["local".into(), "frontier".into()];
    pack.path_counts.frontier = 1;
    assert!(refuse_pack(&pack).is_err(), "order must be frontier then local");
    pack.promoted = true;
    pack.source_drivers = vec!["frontier".into(), "local".into()];
    assert!(matches!(refuse_pack(&pack).unwrap_err(), FeedError::NoAutoPromote));
}

#[test]
fn append_refuses_sku_in_event_and_writes_cursor() {
    let dir = tmp();
    let err = append_event(
        &dir,
        &ScrubbedEvent {
            kind: "model.local.5090".into(),
            agent_id: Some("research".into()),
            decision: Some("allow".into()),
            object_class: Some("local".into()),
            note: None,
            ts: String::new(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, FeedError::SkuBanned(_)));
    append_event(
        &dir,
        &ScrubbedEvent {
            kind: "proxy.tool".into(),
            agent_id: Some("horizon".into()),
            decision: Some("deny".into()),
            object_class: Some("tool".into()),
            note: None,
            ts: String::new(),
        },
    )
    .unwrap();
    let cursor = load_cursor(&dir).unwrap().expect("cursor");
    assert_eq!(cursor.schema, "cell-one.feed-cursor.v0");
    assert_eq!(cursor.events, 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_refuses_promoted_flag() {
    let dir = tmp();
    let mut pack = pack_from_events("x", &[]);
    pack.promoted = true;
    assert!(matches!(
        write_drop_pack(&dir, &pack),
        Err(FeedError::NoAutoPromote)
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pack_id_refuses_hardware_sku() {
    let dir = tmp();
    let err = materialize_from_feed(&dir.join("feed"), &dir.join("drop"), "local-5090")
        .unwrap_err();
    assert!(matches!(err, FeedError::SkuBanned(_)));
    let mut pack = pack_from_events("ok-pack", &[]);
    pack.host_class = "not-a-host".into();
    assert!(matches!(
        write_drop_pack(&dir, &pack),
        Err(FeedError::BadHostClass(_))
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scrub_pii_redacts_keys_bearer_and_email() {
    let raw = "key=sk-abcdefghijklmnopqrstuv xai-abcdefghijklmnopqrstuv bearer-ABCDEF123456 jason@example.com keep";
    let scrubbed = scrub_pii(raw);
    assert!(!scrubbed.contains("sk-"));
    assert!(!scrubbed.contains("xai-"));
    assert!(!scrubbed.contains("jason@example.com"));
    assert!(!scrubbed.contains("bearer-ABCDEF123456"));
    assert!(scrubbed.contains("[redacted]"));
    assert!(scrubbed.contains("keep"));
}

#[test]
fn append_event_scrubs_note() {
    let dir = tmp();
    append_event(
        &dir,
        &ScrubbedEvent {
            kind: "proxy.tool".into(),
            agent_id: Some("horizon".into()),
            decision: Some("deny".into()),
            object_class: Some("tool".into()),
            note: Some("contact jason@example.com key=sk-abcdefghijklmnopqrstuv".into()),
            ts: String::new(),
        },
    )
    .unwrap();
    let text = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
    assert!(!text.contains("sk-"));
    assert!(!text.contains("jason@example.com"));
    assert!(text.contains("[redacted]"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_pack_index_lists_candidates() {
    let feed = tmp();
    let drop = feed.join("drop");
    materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
    let index = drop.join("INDEX.md");
    assert!(index.is_file());
    let text = std::fs::read_to_string(&index).unwrap();
    assert!(text.contains("overnight-traces"));
    assert!(text.contains("promoted=false"));
    assert!(text.contains("drivers=-"), "{text}");
    assert!(
        !text.contains("drivers=frontier"),
        "empty pack must not invent a source driver: {text}"
    );
    let _ = std::fs::remove_dir_all(&feed);
}

