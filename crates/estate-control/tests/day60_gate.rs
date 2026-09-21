//! A5–A9 automated checks on the synthetic mixed-model estate.

use estate_schema::{diff_estates, load_estate_str};
use floor_supervisor::{apply_with_profile_dir, drift_with_roots, stop_runtime};
use model_estate::{
    record_bindings, run_task, MockFrontier, MockLocal, TaskAct, TaskRequest,
};

fn example() -> estate_schema::Estate {
    load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
}

fn tmp() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-d60-{}-{}",
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
fn a5_plan_blast_radius_names_sessions_and_bindings() {
    let e = example();
    let plan = diff_estates(&e, None);
    assert!(plan.blast_radius_text.contains("3 sessions") || plan.blast_radius_text.contains("bind"));
    assert!(plan.blast_radius_text.contains("live-capable"));
    assert!(plan.blast_radius_text.contains("Control does not invoke models"));
    let same = diff_estates(&e, Some(&e));
    assert!(same.blast_radius_text.contains("empty"));
}

#[test]
fn a6_apply_converges_and_drift_detects_pause() {
    let e = example();
    let root = tmp();
    let state = root.join("state");
    apply_with_profile_dir(&e, &state, &root).unwrap();
    record_bindings(&e, &state).unwrap();
    assert!(drift_with_roots(&e, &state, Some(&root)).unwrap().in_sync);
    assert!(state.join("desired-snapshot.yaml").is_file());
    assert!(state.join("model-actual.json").is_file());
    stop_runtime(&state).unwrap();
    assert!(!drift_with_roots(&e, &state, Some(&root)).unwrap().in_sync);
    apply_with_profile_dir(&e, &state, &root).unwrap();
    assert!(drift_with_roots(&e, &state, Some(&root)).unwrap().in_sync);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a7_a8_a9_mixed_path_obeys_firewall() {
    let e = example();
    let local = MockLocal {
        id: "local_slm".into(),
    };
    let frontier = MockFrontier {
        id: "xai_grok".into(),
        reply: "pong".into(),
    };
    let a7 = run_task(
        &e,
        &TaskRequest {
            agent_id: "horizon".into(),
            act: TaskAct::Model,
            object: "xai_grok".into(),
            payload: "Reply with pong".into(),
        },
        Some(&frontier),
        Some(&local),
        None,
    )
    .unwrap();
    assert_eq!(a7.output.as_deref(), Some("pong"));
    assert!(a7.path.iter().any(|s| s == "local:allow"));

    let a8 = run_task(
        &e,
        &TaskRequest {
            agent_id: "research".into(),
            act: TaskAct::Tool,
            object: "notes-append".into(),
            payload: "note".into(),
        },
        None,
        Some(&local),
        None,
    )
    .unwrap();
    assert!(a8.precheck.unwrap().allow);

    let denied = run_task(
        &e,
        &TaskRequest {
            agent_id: "sanctum".into(),
            act: TaskAct::Model,
            object: "xai_grok".into(),
            payload: "nope".into(),
        },
        Some(&frontier),
        Some(&local),
        None,
    )
    .unwrap();
    assert!(!denied.authorized);
}
