//! Day 61–90 beachhead toward A10–A12. No live Grok / GPU required.

use estate_schema::{
    diff_estates, load_estate_str, plan_covers_hash, render_review_diff, write_plan,
};
use feed_collector::{
    append_event, import_pack, materialize_from_feed, refuse_promote, ScrubbedEvent,
};
use floor_supervisor::{
    apply_with_profile_dir, load_lifecycle, load_placements, resume, suspend, LifecycleState,
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
    let _ = std::fs::remove_dir_all(&root);
}
