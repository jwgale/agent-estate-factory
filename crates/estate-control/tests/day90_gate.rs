//! Day 61–90 beachhead toward A10–A12. No live Grok / GPU required.

use estate_schema::{
    diff_estates, load_estate_str, load_plan_json, plan_against_is_fresh_strict, plan_covers_hash,
    plan_is_reviewable, render_review_diff, render_security_iac, write_plan,
};
use feed_collector::{
    append_event, import_pack, materialize_from_feed, refuse_promote, ScrubbedEvent,
};
use floor_supervisor::{
    apply_with_profile_dir, list_lifecycle_events, load_lifecycle, load_placements, resume, suspend,
    CloudAgentDriver, LifecycleState, PlacementDriver,
};

fn example() -> estate_schema::Estate {
    load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
}

fn tmp() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-d90-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn a10_feed_pack_from_both_paths_never_promotes() {
    let root = tmp();
    let feed = root.join("feed");
    let drop = root.join("drop");
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
    let (pack, _) = materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
    assert!(!pack.promoted);
    assert_eq!(pack.curator, "jason");
    assert!(pack.paths.iter().any(|p| p == "local"));
    assert!(pack.paths.iter().any(|p| p == "frontier"));
    assert!(refuse_promote("overnight-traces").is_err());
    let estate = example();
    assert!(estate.enrich_packs.packs.is_empty());
    let accepted = root.join("accepted");
    let (imported, dest) = import_pack(&drop, &accepted, "overnight-traces", &[]).unwrap();
    assert!(!imported.estate_bound);
    assert!(!imported.pack.promoted);
    assert_eq!(imported.pack.schema, "cell-one.pack.v0");
    assert_eq!(pack.path_counts.local, 1);
    assert_eq!(pack.path_counts.frontier, 1);
    assert!(feed.join("feed-cursor.json").is_file());
    assert!(accepted.join("import-audit.jsonl").is_file());
    assert!(dest.ends_with("overnight-traces.pack.json"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a11_suspend_resume_survives_restart() {
    let e = example();
    let root = tmp();
    let state = root.join("state");
    apply_with_profile_dir(&e, &state, &root).unwrap();
    suspend(&state).unwrap();
    assert_eq!(load_lifecycle(&state).unwrap().state, LifecycleState::Suspended);
    assert!(state.join("lifecycle.json").is_file());
    resume(&e, &state, &root).unwrap();
    assert_eq!(load_lifecycle(&state).unwrap().state, LifecycleState::Running);
    let places = load_placements(&state).unwrap().expect("placement-actual");
    let cloud = places
        .leases
        .iter()
        .find(|l| l.kind == "cloud-agent")
        .expect("cloud-agent lease");
    assert!(!cloud.spawned);
    let history = list_lifecycle_events(&state).unwrap();
    assert!(history.iter().any(|e| e.action == "suspend"));
    assert!(history.iter().any(|e| e.action == "resume" || e.action == "apply"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a12_plan_reviewable_and_placement_stub_declared() {
    let e = example();
    assert!(e
        .placements
        .iter()
        .any(|p| p.kind == estate_schema::PlacementKind::CloudAgent && !p.wired));
    let plan = diff_estates(&e, None);
    let review = render_review_diff(&plan);
    assert!(review.contains("Reviewable diff"));
    assert!(review.contains("placements"));
    let root = tmp();
    let written = write_plan(&root, &plan).unwrap();
    assert!(written.is_file());
    assert!(root.join("INDEX.md").is_file());
    assert!(plan_covers_hash(&root, &plan.desired_hash));
    assert_eq!(plan.schema, "cell-one.plan.v0");
    assert!(estate_schema::plan_against_is_fresh(&plan, None));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a10_pack_id_refuses_sku_and_writes_index() {
    let root = tmp();
    let drop = root.join("drop");
    let err = materialize_from_feed(&root.join("feed"), &drop, "local-5090").unwrap_err();
    assert!(err.to_string().contains("SKU") || err.to_string().contains("5090"));
    let (pack, _) = materialize_from_feed(&root.join("feed"), &drop, "overnight-traces").unwrap();
    assert_eq!(pack.schema, "cell-one.pack.v0");
    assert!(drop.join("INDEX.md").is_file());
    let index = std::fs::read_to_string(drop.join("INDEX.md")).unwrap();
    assert!(index.contains("overnight-traces"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a12_cloud_driver_never_spawns() {
    let placement = estate_schema::Placement {
        id: "remote-stub".into(),
        kind: estate_schema::PlacementKind::CloudAgent,
        host_class: Some("any".into()),
        agents: vec!["horizon".into()],
        wired: true,
        params: serde_json::json!({}),
        note: None,
    };
    let lease = CloudAgentDriver.claim(&placement);
    assert!(!lease.spawned);
    assert_eq!(lease.driver, "cloud-agent");
}

#[test]
fn wave2_security_iac_and_strict_fresh_plan() {
    let e = example();
    let plan = diff_estates(&e, None);
    assert!(plan_is_reviewable(&plan));
    assert!(render_security_iac(&plan).contains("Security-as-IaC"));
    assert!(!plan_against_is_fresh_strict(&plan, Some(&plan.desired_hash)));
    let fixture = load_plan_json(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/valid/covering-plan.json"
    )))
    .unwrap();
    assert!(plan_is_reviewable(&fixture));
    let stale = load_plan_json(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/invalid/stale-plan.json"
    )))
    .unwrap();
    assert!(!plan_against_is_fresh_strict(
        &stale,
        Some("sha256:0000000000000000000000000000000000000000000000000000000000000001")
    ));
}

#[test]
fn wave2_host_matrix_and_convey_mesh() {
    for rel in [
        "hosts/rtx-consumer.yaml",
        "hosts/apple-silicon.yaml",
        "hosts/nvidia-rental.yaml",
    ] {
        let path = format!(
            "{}/../../examples/{rel}",
            env!("CARGO_MANIFEST_DIR")
        );
        let estate = estate_schema::load_estate(std::path::Path::new(&path)).unwrap();
        assert!(estate.placements.iter().any(|p| p.id == "cell-one-box"));
    }
    let root = tmp();
    let lease = conveyor_proxy::declare_hop(
        &root,
        conveyor_proxy::HopDecl {
            id: "box-notes".into(),
            kind: "box".into(),
            capability: "notes-append".into(),
            host_class: "any".into(),
            wired: true,
            note: None,
        },
    )
    .unwrap();
    assert!(lease.granted);
    assert!(conveyor_proxy::call_hop(&root, "box-notes", "notes-append")
        .unwrap()
        .allow);
    assert!(conveyor_proxy::call_hop(&root, "missing", "lane-tool").is_err());
    let _ = std::fs::remove_dir_all(&root);
}
