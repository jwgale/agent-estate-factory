use super::feed_test_support::{bound_estate, tmp};
use super::*;

#[test]
fn index_lists_source_drivers_when_present_and_round_trips() {
    let feed = tmp();
    append_event(
        &feed,
        &ScrubbedEvent {
            kind: "model.local.precheck".into(),
            agent_id: Some("research".into()),
            decision: Some("allow".into()),
            object_class: Some("local".into()),
            note: None,
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
    materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
    let index = std::fs::read_to_string(drop.join("INDEX.md")).unwrap();
    assert!(index.contains("drivers=frontier,local"), "{index}");
    assert!(index.contains("promoted=false"), "{index}");
    let listed = list_drop_packs(&drop).unwrap();
    assert_eq!(
        listed[0].source_drivers,
        vec!["frontier".to_string(), "local".to_string()]
    );
    let blob = std::fs::read_to_string(drop.join("overnight-traces.pack.json")).unwrap();
    let back: PackManifest = serde_json::from_str(&blob).unwrap();
    assert_eq!(back.source_drivers, listed[0].source_drivers);
    let mut old: serde_json::Value = serde_json::from_str(&blob).unwrap();
    old.as_object_mut().unwrap().remove("source_drivers");
    let legacy: PackManifest = serde_json::from_value(old).unwrap();
    assert!(legacy.source_drivers.is_empty());
    let schema = include_str!("../../../schema/pack.v0.json");
    assert!(schema.contains("\"source_drivers\""));
    let specialist = include_str!("../../../schema/specialist-pack.v0.json");
    assert!(specialist.contains("\"source_drivers\""));
    let proposal = include_str!("../../../schema/enrich-proposal.v0.json");
    assert!(proposal.contains("\"source_drivers\""));
    let _ = std::fs::remove_dir_all(&feed);
}

#[test]
fn index_refuses_missing_source_drivers_when_counts_are_nonzero() {
    let feed = tmp();
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
    let events = read_events(&feed).unwrap();
    let pack = pack_from_events("overnight-traces", &events);
    let mut raw = serde_json::to_value(&pack).unwrap();
    raw.as_object_mut().unwrap().remove("source_drivers");
    let drop = feed.join("drop");
    std::fs::create_dir_all(&drop).unwrap();
    std::fs::write(
        drop.join("overnight-traces.pack.json"),
        serde_json::to_string_pretty(&raw).unwrap(),
    )
    .unwrap();
    let stale = "stale drivers=-\n";
    std::fs::write(drop.join("INDEX.md"), stale).unwrap();
    let err = write_pack_index(&drop).unwrap_err();
    assert!(
        err.to_string().contains("refuse:source-driver"),
        "{err}"
    );
    assert_eq!(
        std::fs::read_to_string(drop.join("INDEX.md")).unwrap(),
        stale,
        "a mismatched tag must not rewrite INDEX as drivers=-"
    );
    let _ = std::fs::remove_dir_all(&feed);
}

#[test]
fn propose_enrich_never_auto_applies() {
    let feed = tmp();
    let drop = feed.join("drop");
    let accepted = feed.join("accepted");
    let proposed = feed.join("proposed");
    append_event(
        &feed,
        &ScrubbedEvent {
            kind: "model.local.precheck".into(),
            agent_id: Some("research".into()),
            decision: Some("allow".into()),
            object_class: Some("local".into()),
            note: None,
            ts: String::new(),
        },
    )
    .unwrap();
    materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
    import_pack(&drop, &accepted, "overnight-traces", &[], &bound_estate()).unwrap();
    let estate =
        estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
    let estate_yaml = serde_json::to_string(&estate.enrich_packs).unwrap();
    let (proposal, path) =
        propose_enrich(&drop, &accepted, &proposed, "overnight-traces", &estate).unwrap();
    assert!(!proposal.auto_apply);
    assert_eq!(proposal.schema, PROPOSAL_SCHEMA);
    assert_eq!(proposal.curator, "jason");
    assert_eq!(proposal.policy, "manual");
    assert!(proposal.diff.would_add_to_estate);
    assert!(!proposal.diff.estate_bound);
    assert!(path.ends_with("overnight-traces.proposal.json"));
    assert!(proposed.join("overnight-traces.proposal.md").is_file());
    let index = std::fs::read_to_string(proposed.join("INDEX.md")).unwrap();
    assert!(index.contains("drivers=local"), "{index}");
    assert!(
        !index.contains("drivers=frontier"),
        "local-only proposal must not invent frontier: {index}"
    );
    let blob = std::fs::read_to_string(&path).unwrap();
    assert!(!blob.trim().is_empty(), "propose_enrich must not write empty");
    assert!(blob.contains(PROPOSAL_SCHEMA), "{blob}");
    let md = render_proposal(&proposal);
    assert!(md.contains("auto_apply: false"));
    assert!(md.contains("would_add_to_estate: true"));
    assert!(matches!(
        refuse_apply_proposal("overnight-traces"),
        Err(FeedError::NoAutoApply)
    ));
    assert_eq!(
        estate_yaml,
        serde_json::to_string(&estate.enrich_packs).unwrap()
    );
    let sku = propose_enrich(&drop, &accepted, &proposed, "local-5090", &estate).unwrap_err();
    assert!(matches!(sku, FeedError::SkuBanned(_)));
    let _ = std::fs::remove_dir_all(&feed);
}

#[test]
fn scrub_pii_covers_cloud_tokens_pem_and_password() {
    let raw = "ghp_abcdefghijklmnopqrstuv hf_abcdefghijklmnopqrstuv AKIAIOSFODNN7EXAMPLE xoxb-1234567890-token password=hunter2 -----BEGIN PRIVATE KEY-----\nMIIB\n-----END PRIVATE KEY----- keep";
    let scrubbed = scrub_pii(raw);
    assert!(!scrubbed.contains("ghp_"));
    assert!(!scrubbed.contains("hf_"));
    assert!(!scrubbed.contains("AKIA"));
    assert!(!scrubbed.contains("xoxb-"));
    assert!(!scrubbed.contains("hunter2"));
    assert!(!scrubbed.contains("BEGIN PRIVATE"));
    assert!(scrubbed.contains("[redacted]"));
    assert!(scrubbed.contains("keep"));
    let report = redaction_report(raw);
    assert!(report.raw_secrets_found > 0);
    assert_eq!(report.schema, REDACTION_SCHEMA);
    assert!(refuse_raw_secrets(raw).is_err());
    assert!(refuse_raw_secrets("plain notes only").is_ok());
}

#[test]
fn import_refuses_raw_secrets_and_accepts_specialist_fields() {
    let dir = tmp();
    let drop = dir.join("drop");
    std::fs::create_dir_all(&drop).unwrap();
    let mut pack = pack_from_events("overnight-traces", &[]);
    pack.source_paths = vec!["feed/events.jsonl".into()];
    pack.model_hint = Some("local_slm".into());
    pack.host_class_affinity = Some("any".into());
    pack.schema = "cell-one.specialist-pack.v0".into();
    write_drop_pack(&drop, &pack).unwrap();
    let accepted = dir.join("accepted");
    let (imported, _) =
        import_pack(&drop, &accepted, "overnight-traces", &[], &bound_estate()).unwrap();
    assert_eq!(imported.pack.model_hint.as_deref(), Some("local_slm"));
    assert!(accepted.join("overnight-traces.redaction.json").is_file());
    let accepted_pack =
        std::fs::read_to_string(accepted.join("overnight-traces.pack.json")).unwrap();
    let accepted_report =
        std::fs::read_to_string(accepted.join("overnight-traces.redaction.json")).unwrap();
    assert!(!accepted_pack.contains("sk-"));
    assert!(!accepted_report.contains("sk-"));
    assert!(accepted_report.contains("Kind counts only"));
    let mut dirty = pack.clone();
    dirty.id = "dirty-pack".into();
    dirty.note = "key=sk-abcdefghijklmnopqrstuv".into();
    assert!(matches!(
        write_drop_pack(&drop, &dirty),
        Err(FeedError::RawSecret)
    ));
    let mut sku_hint = pack.clone();
    sku_hint.id = "hint-pack".into();
    sku_hint.model_hint = Some("local-5090".into());
    sku_hint.note = "clean".into();
    assert!(matches!(
        write_drop_pack(&drop, &sku_hint),
        Err(FeedError::BadModelHint(_))
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn import_refuses_wrong_curator() {
    refuse_curator("jason", "jason").unwrap();
    let err = refuse_curator("not-jason", "jason").unwrap_err();
    assert!(err.to_string().contains("refuse:curator"));
    let dir = tmp();
    let drop = dir.join("drop");
    materialize_from_feed(&dir.join("feed"), &drop, "overnight-traces").unwrap();
    let err = import_pack_for(
        &drop,
        &dir.join("accepted"),
        "overnight-traces",
        &[],
        "robot",
        "jason",
        &bound_estate(),
    )
    .unwrap_err();
    assert!(matches!(err, FeedError::WrongCurator { .. }));
    assert!(!dir.join("accepted").exists());
    let err = refuse_import_pack(
        &drop,
        "overnight-traces",
        "robot",
        "jason",
        &bound_estate(),
    )
    .unwrap_err();
    assert!(matches!(err, FeedError::WrongCurator { .. }));
    assert!(!dir.join("accepted").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

