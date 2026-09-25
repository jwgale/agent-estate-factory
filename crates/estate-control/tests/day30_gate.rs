//! Automated A1–A4 + pause-safe rebind on the synthetic estate.

use estate_schema::{
    authorize, read_lane_file, AccessRequest, IntentionKind, LOCKED_SACRED,
};
use floor_supervisor::{apply_with_profile_dir, drift, stop_runtime};
use std::path::Path;

fn example() -> estate_schema::Estate {
    estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
}

fn tmp() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-gate-{}-{}",
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
fn a1_three_agents_separate_lanes() {
    let e = example();
    assert!(e.agents.len() >= 3);
    assert!(e.lanes.len() >= 3);
    let lanes: std::collections::HashSet<_> = e.agents.iter().map(|a| a.lane.as_str()).collect();
    assert_eq!(lanes.len(), e.agents.len());
    for id in ["horizon", "research", "sanctum"] {
        assert!(e.agent(id).is_some(), "missing {id}");
        assert!(e.lane(id).is_some(), "missing lane {id}");
    }
}

#[test]
fn a2_own_desktop_and_session() {
    let e = example();
    let root = tmp();
    let actual = apply_with_profile_dir(&e, &root.join("state"), &root).unwrap();
    assert_eq!(actual.sessions.len(), 3);
    let mut desktops = std::collections::HashSet::new();
    for session in &actual.sessions {
        assert!(desktops.insert(session.desktop.clone()));
        assert!(session.session_path.join("profile").is_dir());
        assert_eq!(session.agent_id, session.lane_id);
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a3_memory_firewall_and_sacred_exclusions() {
    let e = example();
    let cross = authorize(
        &e,
        &AccessRequest {
            subject_agent: "horizon",
            kind: IntentionKind::MemoryRead,
            object: "lane:research",
        },
    );
    assert!(!cross.is_allow());

    let root = tmp();
    std::fs::create_dir_all(root.join("lanes/research")).unwrap();
    std::fs::write(root.join("lanes/research/secret.txt"), b"nope").unwrap();
    let denied = read_lane_file(
        &e,
        &root,
        "sanctum",
        "research",
        Path::new("secret.txt"),
    )
    .unwrap_err();
    assert!(denied.reason.contains("cross-lane"));

    for (id, aliases) in LOCKED_SACRED {
        assert!(e.agent(id).is_none(), "{id} must not be an agent");
        assert!(e.is_sacred(id));
        for alias in *aliases {
            let d = authorize(
                &e,
                &AccessRequest {
                    subject_agent: "horizon",
                    kind: IntentionKind::MemoryRead,
                    object: alias,
                },
            );
            assert!(!d.is_allow(), "must deny sacred {alias}");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a4_deny_default_tools_and_mounts() {
    let e = example();
    let undeclared = authorize(
        &e,
        &AccessRequest {
            subject_agent: "horizon",
            kind: IntentionKind::Tool,
            object: "shell",
        },
    );
    assert!(!undeclared.is_allow());
    let declared = authorize(
        &e,
        &AccessRequest {
            subject_agent: "research",
            kind: IntentionKind::Tool,
            object: "notes-append",
        },
    );
    assert!(!declared.is_allow());
    assert!(declared.reason().contains("not covered by an allow Tool intention"));
    let mount_ok = authorize(
        &e,
        &AccessRequest {
            subject_agent: "research",
            kind: IntentionKind::Mount,
            object: "notes",
        },
    );
    assert!(mount_ok.is_allow());
    let mount_bad = authorize(
        &e,
        &AccessRequest {
            subject_agent: "research",
            kind: IntentionKind::Mount,
            object: "secrets",
        },
    );
    assert!(!mount_bad.is_allow());
}

#[test]
fn pause_rebinds_same_estate_from_files() {
    let e = example();
    let root = tmp();
    let state = root.join("state");
    let first = apply_with_profile_dir(&e, &state, &root).unwrap();
    assert!(drift(&e, &state).unwrap().in_sync);
    std::fs::write(root.join("lanes/horizon/keep.txt"), b"persisted").unwrap();

    stop_runtime(&state).unwrap();
    assert!(!state.join("sessions").exists());
    assert!(root.join("lanes/horizon/keep.txt").is_file());

    let second = apply_with_profile_dir(&e, &state, &root).unwrap();
    assert_eq!(first.desired_hash, second.desired_hash);
    assert_eq!(first.sessions.len(), second.sessions.len());
    assert!(root.join("lanes/horizon/keep.txt").is_file());
    assert!(drift(&e, &state).unwrap().in_sync);
    let _ = std::fs::remove_dir_all(&root);
}
