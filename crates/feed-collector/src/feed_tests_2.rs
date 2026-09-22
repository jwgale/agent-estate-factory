use super::feed_test_support::{bound_estate, tmp};
use super::*;

#[test]
fn refuse_codes_missing_pack_and_no_auto_apply() {
    let dir = tmp();
    let err = import_pack(
        &dir.join("drop"),
        &dir.join("accepted"),
        "no-such",
        &[],
        &bound_estate(),
    )
    .unwrap_err();
    assert!(matches!(err, FeedError::MissingPack(_)));
    assert!(err.to_string().starts_with("refuse:missing-pack"));
    let apply = refuse_apply_proposal("overnight-traces").unwrap_err();
    assert!(matches!(apply, FeedError::NoAutoApply));
    assert!(apply.to_string().starts_with("refuse:no-auto-apply"));
    let raw = refuse_raw_secrets("token=sk-abcdefghijklmnopqrstuv").unwrap_err();
    assert!(raw.to_string().starts_with("refuse:raw-secret"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn redaction_report_json_never_contains_raw_secret() {
    let secret = "sk-abcdefghijklmnopqrstuv";
    let raw = format!("note key={secret} password=hunter2");
    let report = redaction_report(&raw);
    assert!(report.raw_secrets_found > 0);
    let blob = serde_json::to_string_pretty(&report).unwrap();
    assert!(!blob.contains(secret), "{blob}");
    assert!(!blob.contains("hunter2"), "{blob}");
    write_redaction_report(&tmp().join("ok.redaction.json"), &report).unwrap();
    let dir = tmp();
    let drop = dir.join("drop");
    std::fs::create_dir_all(&drop).unwrap();
    std::fs::write(
        drop.join("dirty-pack.pack.json"),
        format!(
            "{{\n  \"id\": \"dirty-pack\",\n  \"version\": 0,\n  \"schema\": \"cell-one.pack.v0\",\n  \"curator\": \"jason\",\n  \"policy\": \"manual\",\n  \"promoted\": false,\n  \"source\": \"feed\",\n  \"from_events\": 0,\n  \"kinds\": [],\n  \"paths\": [],\n  \"agents\": [],\n  \"host_class\": \"any\",\n  \"path_counts\": {{\"kinds\": 0, \"paths\": 0, \"agents\": 0}},\n  \"source_paths\": [],\n  \"created_at\": \"unix:1\",\n  \"note\": \"key={secret}\"\n}}\n"
        ),
    )
    .unwrap();
    let accepted = dir.join("accepted");
    let err = import_pack(&drop, &accepted, "dirty-pack", &[], &bound_estate()).unwrap_err();
    assert!(matches!(err, FeedError::RawSecret), "{err}");
    assert!(!accepted.join("dirty-pack.pack.json").exists());
    assert!(!accepted.join("dirty-pack.redaction.json").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn propose_refuses_frontier_source_without_a_frontier_binding() {
    let feed = tmp();
    let traces = feed.join("traces");
    let drop = feed.join("drop");
    let accepted = feed.join("accepted");
    let proposed = feed.join("proposed");
    append_event(
        &traces,
        &ScrubbedEvent {
            kind: "model.frontier.complete".into(),
            agent_id: Some("horizon".into()),
            decision: Some("allow".into()),
            object_class: Some("frontier".into()),
            note: None,
            ts: String::new(),
        },
    )
    .unwrap();
    append_event(
        &traces,
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
    materialize_from_feed(&traces, &drop, "overnight-traces").unwrap();
    let mut local_only =
        estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
    local_only
        .model_bindings
        .retain(|b| b.class != estate_schema::ModelClass::Frontier);
    std::fs::create_dir_all(&proposed).unwrap();
    std::fs::write(proposed.join("SENTINEL"), "keep\n").unwrap();
    let err = propose_enrich(&drop, &accepted, &proposed, "overnight-traces", &local_only)
        .unwrap_err();
    assert!(matches!(err, FeedError::FrontierInvent), "{err}");
    assert!(err.to_string().contains("refuse:frontier-invent"), "{err}");
    assert!(!err.to_string().contains("grok-4.7"), "{err}");
    assert!(!proposed.join("overnight-traces.proposal.json").exists());
    assert!(!proposed.join("INDEX.md").exists());
    assert_eq!(
        std::fs::read_to_string(proposed.join("SENTINEL")).unwrap(),
        "keep\n"
    );

    let local_traces = feed.join("local-traces");
    let local_drop = feed.join("local-drop");
    append_event(
        &local_traces,
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
    materialize_from_feed(&local_traces, &local_drop, "local-only").unwrap();
    let (proposal, path) =
        propose_enrich(&local_drop, &accepted, &proposed, "local-only", &local_only).unwrap();
    assert_eq!(proposal.diff.source_drivers, vec!["local".to_string()]);
    assert!(
        !proposal.diff.source_drivers.iter().any(|d| d == "frontier"),
        "local-only proposal must not invent a frontier source_driver: {:?}",
        proposal.diff.source_drivers
    );
    assert!(path.is_file());
    let _ = std::fs::remove_dir_all(&feed);
}

#[test]
fn import_refuses_frontier_source_without_a_frontier_binding() {
    let feed = tmp();
    let traces = feed.join("traces");
    let drop = feed.join("drop");
    let accepted = feed.join("accepted");
    append_event(
        &traces,
        &ScrubbedEvent {
            kind: "model.frontier.complete".into(),
            agent_id: Some("horizon".into()),
            decision: Some("allow".into()),
            object_class: Some("frontier".into()),
            note: None,
            ts: String::new(),
        },
    )
    .unwrap();
    append_event(
        &traces,
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
    materialize_from_feed(&traces, &drop, "overnight-traces").unwrap();
    let mut local_only = bound_estate();
    local_only
        .model_bindings
        .retain(|b| b.class != estate_schema::ModelClass::Frontier);
    std::fs::create_dir_all(&accepted).unwrap();
    std::fs::write(accepted.join("SENTINEL"), "keep\n").unwrap();
    let index_before = std::fs::read(drop.join("INDEX.md")).unwrap();
    let err = import_pack(&drop, &accepted, "overnight-traces", &[], &local_only).unwrap_err();
    assert!(matches!(err, FeedError::FrontierInvent), "{err}");
    assert!(err.to_string().contains("refuse:frontier-invent"), "{err}");
    assert!(!err.to_string().contains("grok-4.7"), "{err}");
    assert!(!accepted.join("overnight-traces.pack.json").exists());
    assert!(!accepted.join("overnight-traces.redaction.json").exists());
    assert!(!accepted.join("import-audit.jsonl").exists());
    assert_eq!(
        std::fs::read_to_string(accepted.join("SENTINEL")).unwrap(),
        "keep\n"
    );
    assert_eq!(std::fs::read(drop.join("INDEX.md")).unwrap(), index_before);

    let local_traces = feed.join("local-traces");
    let local_drop = feed.join("local-drop");
    append_event(
        &local_traces,
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
    materialize_from_feed(&local_traces, &local_drop, "local-only").unwrap();
    let (imported, path) =
        import_pack(&local_drop, &accepted, "local-only", &[], &local_only).unwrap();
    assert_eq!(imported.pack.source_drivers, vec!["local".to_string()]);
    assert!(!imported.pack.promoted);
    assert!(
        !imported.pack.source_drivers.iter().any(|d| d == "frontier"),
        "local-only import must not invent a frontier source_driver: {:?}",
        imported.pack.source_drivers
    );
    let pack_blob = std::fs::read_to_string(&path).unwrap();
    let report_path = accepted.join("local-only.redaction.json");
    let report = std::fs::read_to_string(&report_path).unwrap();
    assert!(report.contains("cell-one.redaction.v0"), "{report}");
    assert!(report.contains("Kind counts only"), "{report}");
    assert!(!pack_blob.contains("grok-4.7"), "{pack_blob}");
    assert!(!report.contains("grok-4.7"), "{report}");
    assert!(!pack_blob.contains("sk-"), "{pack_blob}");
    assert!(!report.contains("sk-"), "{report}");
    let _ = std::fs::remove_dir_all(&feed);
}
