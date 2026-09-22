//! Day 61–90 beachhead toward A10–A12. No live Grok / GPU required.

use estate_schema::{
    diff_estates, load_estate_str, load_plan_json, plan_against_is_fresh_strict, plan_covers_hash,
    plan_is_reviewable, render_review_diff, render_security_iac, write_plan,
};
use feed_collector::{
    append_event, import_pack, materialize_from_feed, propose_enrich, refuse_apply_proposal,
    refuse_promote, ScrubbedEvent,
};
use floor_supervisor::{
    apply_with_profile_dir, list_lifecycle_events, load_lifecycle, load_placements,
    reconcile_placements, resume, suspend, CloudAgentDriver, LifecycleState, PlacementDriver,
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
    let (imported, dest) = import_pack(&drop, &accepted, "overnight-traces", &[], &estate).unwrap();
    assert!(!imported.estate_bound);
    assert!(!imported.pack.promoted);
    assert_eq!(imported.pack.schema, "cell-one.pack.v0");
    assert_eq!(pack.path_counts.local, 1);
    assert_eq!(pack.path_counts.frontier, 1);
    assert_eq!(
        pack.source_drivers,
        vec!["frontier".to_string(), "local".to_string()]
    );
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
    assert!(plan_covers_hash(&root, &plan.desired_hash).unwrap());
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
        ttl_secs: None,
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
            ttl_secs: None,
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

#[test]
fn wave3_multi_host_propose_reconcile() {
    let path = format!(
        "{}/../../examples/hosts/multi-host.yaml",
        env!("CARGO_MANIFEST_DIR")
    );
    let estate = estate_schema::load_estate(std::path::Path::new(&path)).unwrap();
    assert!(estate
        .model_bindings
        .iter()
        .any(|b| b.driver == "http-remote"));
    assert!(estate
        .model_bindings
        .iter()
        .any(|b| b.driver == "llama.cpp"));
    assert!(estate.model_bindings.iter().any(|b| b.driver == "mlx"));
    assert!(estate.placements.iter().any(|p| p.id == "box-rtx"));
    assert!(estate.placements.iter().any(|p| p.id == "box-apple"));
    assert!(estate.placements.iter().any(|p| p.id == "box-rental"));
    let root = tmp();
    apply_with_profile_dir(&estate, &root.join("state"), &root).unwrap();
    let report = reconcile_placements(&estate, &root.join("state")).unwrap();
    assert!(report.in_sync, "{:?}", report.refuses);
    let drop = root.join("drop");
    let accepted = root.join("accepted");
    let proposed = root.join("proposed");
    materialize_from_feed(&root.join("feed"), &drop, "overnight-traces").unwrap();
    import_pack(&drop, &accepted, "overnight-traces", &[], &estate).unwrap();
    let (proposal, _) =
        propose_enrich(&drop, &accepted, &proposed, "overnight-traces", &estate).unwrap();
    assert!(!proposal.auto_apply);
    assert!(proposal.diff.would_add_to_estate);
    assert!(refuse_apply_proposal("overnight-traces").is_err());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn wave4_dry_run_ttl_scrub_specialist() {
    let e = example();
    let root = tmp();
    let state = root.join("state");
    let dry = floor_supervisor::apply_dry_run(&e, &state).unwrap();
    assert!(!dry.writes);
    assert!(!dry.would_refuse, "{:?}", dry.refuses);
    assert!(!state.join("placement-actual.json").exists());
    apply_with_profile_dir(&e, &state, &root).unwrap();
    let mut actual = load_placements(&state).unwrap().unwrap();
    let now = floor_supervisor::now_unix();
    if let Some(lease) = actual.leases.iter_mut().find(|l| l.kind == "box") {
        lease.ttl_secs = Some(1);
        lease.issued_at = Some(now.saturating_sub(10));
        lease.expires_at = Some(now.saturating_sub(1));
    }
    floor_supervisor::write_placements(&state, &actual).unwrap();
    assert!(floor_supervisor::refuse_expired_leases(&state).is_err());
    let preview = floor_supervisor::apply_dry_run(&e, &state).unwrap();
    assert!(preview.would_refuse);
    assert!(preview.refuses.iter().any(|r| r.code == "expired"));
    floor_supervisor::forget_expired_leases(&state).unwrap();
    floor_supervisor::refuse_expired_leases(&state).unwrap();

    let raw = "ghp_abcdefghijklmnopqrstuv password=hunter2";
    let scrubbed = feed_collector::scrub_pii(raw);
    assert!(!scrubbed.contains("ghp_"));
    assert!(!scrubbed.contains("hunter2"));
    assert!(feed_collector::refuse_raw_secrets(raw).is_err());
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/fixtures/specialist-overnight.pack.json"
    ))
    .unwrap();
    let pack: feed_collector::PackManifest = serde_json::from_str(&text).unwrap();
    assert_eq!(pack.schema, "cell-one.specialist-pack.v0");
    assert_eq!(pack.model_hint.as_deref(), Some("local_slm"));
    assert!(!pack.source_paths.is_empty());
    feed_collector::refuse_pack(&pack).unwrap();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn wave5_journal_hop_ttl_plan_diff_apiver() {
    let e = example();
    assert!(e.api_version.is_none());
    let happy = estate_schema::load_estate(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/fixtures/happy.yaml"
    )))
    .unwrap();
    assert_eq!(happy.api_version.as_deref(), Some("cell-one.estate.v0"));

    let root = tmp();
    let state = root.join("state");
    apply_with_profile_dir(&e, &state, &root).unwrap();
    let journal = floor_supervisor::list_session_events(&state).unwrap();
    assert!(journal.iter().any(|ev| ev.action == "spawn"));
    assert!(state.join(floor_supervisor::SESSION_JOURNAL).is_file());

    let lease = conveyor_proxy::declare_hop(
        &state,
        conveyor_proxy::HopDecl {
            id: "ttl-box".into(),
            kind: "box".into(),
            capability: "lane-tool".into(),
            host_class: "any".into(),
            wired: true,
            note: None,
            ttl_secs: Some(30),
        },
    )
    .unwrap();
    assert_eq!(lease.ttl_secs, Some(30));
    assert!(lease.expires_at.is_some());

    let empty = estate_schema::diff_estates(&e, Some(&e));
    let green = estate_schema::diff_estates(&e, None);
    assert!(estate_schema::blast_grows(&empty, &green));
    assert!(!estate_schema::blast_grows(&green, &empty));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn wave6_backup_policy_pause_catalog() {
    let e = example();
    let pack = estate_schema::PolicyPack::default();
    estate_schema::refuse_policy(&pack).unwrap();
    estate_schema::policy_allows(&pack, "apply", None).unwrap();
    assert!(estate_schema::policy_allows(&pack, "spawn-cloud", None)
        .unwrap_err()
        .contains("refuse:unknown-action"));

    let root = tmp();
    let state = root.join("state");
    apply_with_profile_dir(&e, &state, &root).unwrap();
    let (archive, meta) =
        floor_supervisor::backup_cell(&state, None, &root.join("backups"), Some(&e)).unwrap();
    assert!(!meta.sacred_ids.is_empty());
    assert!(archive.join(floor_supervisor::BACKUP_META).is_file());
    let dry = floor_supervisor::restore_cell(&archive, &root.join("empty"), None, Some(&e), true)
        .unwrap();
    assert!(!dry.writes);
    assert!(!dry.would_refuse);
    assert!(!root.join("empty").join("placement-actual.json").exists());

    let proof = floor_supervisor::pause_kit_proof(&e, &root.join("pause"), &root).unwrap();
    assert!(proof.leases_survived);
    assert!(!proof.cloud_spawned);
    assert!(proof.in_sync);
    assert!(proof.sessions_dropped);

    let snap = model_estate::catalog_file();
    assert!(snap.cards.iter().all(|c| c.caps.context_tokens > 0));
    assert!(snap.cards.iter().any(|c| c.driver_id == "mlx" && c.caps.streaming));
    assert!(snap
        .cards
        .iter()
        .any(|c| c.driver_id == "vllm" && c.caps.tools));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn wave7_curator_sync_status() {
    feed_collector::refuse_curator("jason", "jason").unwrap();
    assert!(feed_collector::refuse_curator("robot", "jason")
        .unwrap_err()
        .to_string()
        .contains("refuse:curator"));
    assert_eq!(
        conveyor_proxy::hop_kind_for_placement("box"),
        Some("box")
    );
    assert_eq!(
        conveyor_proxy::hop_kind_for_placement("cloud-agent"),
        Some("cloud-mesh")
    );
    assert!(conveyor_proxy::hop_kind_for_placement("studio").is_none());

    let e = example();
    let root = tmp();
    apply_with_profile_dir(&e, &root.join("state"), &root).unwrap();
    conveyor_proxy::declare_hop(
        &root.join("state"),
        conveyor_proxy::HopDecl {
            id: "ttl-box".into(),
            kind: "box".into(),
            capability: "lane-tool".into(),
            host_class: "any".into(),
            wired: true,
            note: None,
            ttl_secs: Some(60),
        },
    )
    .unwrap();
    let mesh = conveyor_proxy::sync_from_placements(&root.join("state")).unwrap();
    assert!(mesh.hops.iter().any(|h| h.id == "ttl-box"));
    assert!(mesh.hops.iter().any(|h| h.id == "cell-one-box"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn wave8_apply_identity_export_pr_sacred() {
    let e = example();
    let root = tmp();
    let state = root.join("state");
    assert_eq!(
        floor_supervisor::classify_apply(&e, &state, &root).unwrap(),
        floor_supervisor::ApplyIdentity::Greenfield
    );
    apply_with_profile_dir(&e, &state, &root).unwrap();
    assert_eq!(
        floor_supervisor::classify_apply(&e, &state, &root).unwrap(),
        floor_supervisor::ApplyIdentity::Unchanged
    );

    estate_schema::clear_sacred_overlays();
    estate_schema::set_sacred_overlays(&[estate_schema::SacredFileEntry {
        id: "lab-notebook".into(),
        aliases: vec!["lab_notebook".into()],
        reason: None,
    }]);
    assert!(estate_schema::is_sacred_name("lab-notebook"));
    estate_schema::clear_sacred_overlays();
    assert!(!estate_schema::is_sacred_name("lab-notebook"));

    let plan = diff_estates(&e, None);
    let md = estate_schema::render_plan_pr(
        &plan,
        Some("plan-cover"),
        false,
        &["cloud-agent: declared, not spawned".into()],
    );
    assert!(md.contains("paste into PR body"));
    assert!(md.contains("Refuse risks"));
    assert!(md.contains("Reviewed:** no"));

    let mixed = estate_schema::load_estate(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/fixtures/mixed-frontier-local.yaml"
    )))
    .unwrap();
    assert!(mixed
        .model_bindings
        .iter()
        .any(|b| b.driver == "http-remote" && b.class == estate_schema::ModelClass::Frontier));
    assert!(mixed
        .model_bindings
        .iter()
        .any(|b| b.driver == "ollama" && b.class == estate_schema::ModelClass::Local));
    let dry = floor_supervisor::apply_dry_run(&mixed, &root.join("mixed")).unwrap();
    assert!(!dry.writes);
    assert!(!dry.would_refuse);
    let _ = std::fs::remove_dir_all(&root);
}
