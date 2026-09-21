//! Floor supervisor: bind per-agent sessions. Does not execute tools or models.
//! Isolation is a driver. This crate must stay free of vendor identifiers.

mod backup;
mod lifecycle;
mod pause;
mod placement;
mod sessions;

use estate_schema::{estate_hash, Estate};
use isolation_driver::{BindRequest, BoundSession, IsolationDriver, ProfileDirDriver};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub use lifecycle::{
    append_lifecycle_event, lifecycle_path, list_lifecycle_events, load_lifecycle, mark_running,
    resume, suspend, write_lifecycle, LifecycleEvent, LifecycleRecord, LifecycleState,
    LIFECYCLE_FILE, LIFECYCLE_LOG, LIFECYCLE_SCHEMA, LIFECYCLE_VERSION,
};
pub use placement::{
    append_apply_audit, apply_dry_run, claim_leases, driver_for, drift_placements,
    forget_expired_leases, lease_is_expired, list_apply_audits, list_expired_leases, load_placements,
    mark_leases_unspawned, now_unix, reconcile_placements, record_placements, refuse_expired_leases,
    refuse_lease_host_classes,
    render_dry_run, render_reconcile, write_placements, write_reconcile, ApplyAudit, ApplyDryRun,
    BoxDriver, CloudAgentDriver, PlacementActual, PlacementDriver, PlacementDrift, PlacementLease,
    ReconcileReport, ReconcileRow, Refuse, DRY_RUN_SCHEMA, RECONCILE_SCHEMA,
};
pub use sessions::{
    append_session_event, journal_session, list_session_events, session_journal_path,
    tail_session_events, SessionEvent, SESSION_JOURNAL, SESSION_JOURNAL_SCHEMA,
};
pub use backup::{
    backup_cell, list_cell_backups, prune_cell_backups, render_restore, restore_cell,
    sacred_id_set, sacred_mismatch, CellBackup, PruneReport, RestoreReport, BACKUP_META,
    BACKUP_SCHEMA,
};
pub use pause::{pause_kit_proof, PauseProof};

#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error("invalid estate:\n{}", .0.join("\n"))]
    Invalid(Vec<String>),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("isolation: {0}")]
    Isolation(String),
    #[error("{0}")]
    Other(String),
}

pub(crate) fn refuse_empty_blob(body: &str) -> Result<(), SupervisorError> {
    if body.trim().is_empty() {
        return Err(SupervisorError::Other("serialize: empty".into()));
    }
    Ok(())
}

pub(crate) fn serialize_pretty<T: Serialize>(value: &T) -> Result<String, SupervisorError> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| SupervisorError::Other(format!("serialize: {e}")))?;
    refuse_empty_blob(&body)?;
    Ok(body)
}

pub(crate) fn serialize_line<T: Serialize>(value: &T) -> Result<String, SupervisorError> {
    let line = serde_json::to_string(value)
        .map_err(|e| SupervisorError::Other(format!("serialize: {e}")))?;
    refuse_empty_blob(&line)?;
    if line.trim() == "{}" {
        return Err(SupervisorError::Other("serialize: empty object".into()));
    }
    Ok(line)
}

pub(crate) fn write_pretty_json<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), SupervisorError> {
    let body = serialize_pretty(value)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, body)?;
    Ok(())
}

pub(crate) fn append_json_line<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), SupervisorError> {
    let line = serialize_line(value)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .write(true)
        .truncate(false)
        .open(path)?;
    use std::io::Write;
    writeln!(file, "{line}")?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActualState {
    pub desired_hash: String,
    pub estate_name: String,
    pub sessions: Vec<BoundSession>,
    pub generated_at: String,
    pub regenerable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriftReport {
    pub in_sync: bool,
    pub desired_hash: String,
    pub actual_hash: Option<String>,
    pub missing_agents: Vec<String>,
    pub extra_agents: Vec<String>,
    #[serde(default)]
    pub mismatched_sessions: Vec<String>,
    #[serde(default)]
    pub missing_session_dirs: Vec<String>,
    #[serde(default)]
    pub missing_lane_roots: Vec<String>,
    #[serde(default)]
    pub missing_leases: Vec<String>,
    #[serde(default)]
    pub extra_leases: Vec<String>,
    #[serde(default)]
    pub spawned_cloud_agents: Vec<String>,
    #[serde(default)]
    pub lease_kind_mismatch: Vec<String>,
    #[serde(default)]
    pub host_class_mismatch: Vec<String>,
    pub notes: Vec<String>,
}

impl DriftReport {
    /// Sessions still match. Only placement leases are missing (post
    /// `expire --forget`). Apply restamps; this is not `--force` drift.
    pub fn lease_refresh_only(&self) -> bool {
        !self.missing_leases.is_empty()
            && self.extra_leases.is_empty()
            && self.spawned_cloud_agents.is_empty()
            && self.lease_kind_mismatch.is_empty()
            && self.host_class_mismatch.is_empty()
            && self.missing_agents.is_empty()
            && self.extra_agents.is_empty()
            && self.mismatched_sessions.is_empty()
            && self.missing_session_dirs.is_empty()
            && self.missing_lane_roots.is_empty()
    }
}

pub fn apply(
    estate: &Estate,
    state_dir: &Path,
    roots_base: &Path,
    driver: &dyn IsolationDriver,
) -> Result<ActualState, SupervisorError> {
    estate_schema::validate(estate).map_err(SupervisorError::Invalid)?;
    refuse_expired_leases(state_dir)?;
    std::fs::create_dir_all(state_dir)?;
    std::fs::create_dir_all(state_dir.join("sessions"))?;

    for lane in &estate.lanes {
        let root = roots_base.join(&lane.root_path);
        std::fs::create_dir_all(&root)?;
        let owner_file = root.join(".lane-owner");
        std::fs::write(owner_file, format!("{}\n", lane.owner_agent_id))?;
    }

    let mut sessions = Vec::new();
    for agent in &estate.agents {
        let lane = estate
            .lane(&agent.lane)
            .ok_or_else(|| SupervisorError::Other(format!("missing lane {}", agent.lane)))?;
        let lane_root = roots_base.join(&lane.root_path);
        let bound = driver
            .bind(&BindRequest {
                agent_id: &agent.id,
                desktop: &agent.desktop,
                lane_id: &lane.id,
                lane_root: &lane_root,
            })
            .map_err(|e| SupervisorError::Isolation(e.to_string()))?;
        sessions.push(bound);
    }

    let actual = ActualState {
        desired_hash: estate_hash(estate),
        estate_name: estate.name.clone(),
        sessions,
        generated_at: now_stamp(),
        regenerable: true,
    };
    write_actual(state_dir, &actual)?;
    write_desired_snapshot(state_dir, estate)?;
    record_placements(estate, state_dir)?;
    mark_running(estate, state_dir)?;
    for session in &actual.sessions {
        journal_session(
            state_dir,
            "spawn",
            Some(session.agent_id.as_str()),
            Some(estate.name.as_str()),
            Some(actual.desired_hash.as_str()),
            "apply bind",
        )?;
    }
    Ok(actual)
}

pub fn apply_with_profile_dir(
    estate: &Estate,
    state_dir: &Path,
    roots_base: &Path,
) -> Result<ActualState, SupervisorError> {
    let driver = ProfileDirDriver::new(state_dir.join("sessions"));
    apply(estate, state_dir, roots_base, &driver)
}

pub fn write_actual(state_dir: &Path, actual: &ActualState) -> Result<(), SupervisorError> {
    std::fs::create_dir_all(state_dir)?;
    write_pretty_json(&state_dir.join("actual-state.json"), actual)
}

pub fn write_desired_snapshot(state_dir: &Path, estate: &Estate) -> Result<(), SupervisorError> {
    std::fs::create_dir_all(state_dir)?;
    let yaml = serde_yaml::to_string(estate)
        .map_err(|e| SupervisorError::Other(format!("snapshot: {e}")))?;
    std::fs::write(state_dir.join("desired-snapshot.yaml"), yaml)?;
    Ok(())
}

pub fn load_desired_snapshot(state_dir: &Path) -> Result<Option<Estate>, SupervisorError> {
    let path = state_dir.join("desired-snapshot.yaml");
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)?;
    let estate = serde_yaml::from_str(&text)
        .map_err(|e| SupervisorError::Other(format!("desired-snapshot.yaml: {e}")))?;
    Ok(Some(estate))
}

/// How apply should treat the current desired hash vs disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyIdentity {
    Greenfield,
    Unchanged,
    Drift { notes: Vec<String> },
    DesiredChanged,
    /// Expired rows were forgotten. Apply restamps leases. Not `--force`.
    LeaseRefresh { missing: Vec<String> },
}

/// Identical desired + in_sync → unchanged. Hash match + drift → refuse unless --force.
pub fn classify_apply(
    estate: &Estate,
    state_dir: &Path,
    roots_base: &Path,
) -> Result<ApplyIdentity, SupervisorError> {
    let Some(snap) = load_desired_snapshot(state_dir)? else {
        return Ok(ApplyIdentity::Greenfield);
    };
    if estate_hash(&snap) != estate_hash(estate) {
        return Ok(ApplyIdentity::DesiredChanged);
    }
    let report = drift_with_roots(estate, state_dir, Some(roots_base))?;
    if report.in_sync {
        Ok(ApplyIdentity::Unchanged)
    } else if report.lease_refresh_only() {
        Ok(ApplyIdentity::LeaseRefresh {
            missing: report.missing_leases,
        })
    } else {
        Ok(ApplyIdentity::Drift {
            notes: report.notes,
        })
    }
}

pub fn load_actual(state_dir: &Path) -> Result<Option<ActualState>, SupervisorError> {
    let path = state_dir.join("actual-state.json");
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)?;
    let actual = serde_json::from_str(&text)
        .map_err(|e| SupervisorError::Other(format!("actual-state.json: {e}")))?;
    Ok(Some(actual))
}

pub fn drift(estate: &Estate, state_dir: &Path) -> Result<DriftReport, SupervisorError> {
    drift_with_roots(estate, state_dir, None)
}

pub fn drift_with_roots(
    estate: &Estate,
    state_dir: &Path,
    roots_base: Option<&Path>,
) -> Result<DriftReport, SupervisorError> {
    estate_schema::validate(estate).map_err(SupervisorError::Invalid)?;
    let desired_hash = estate_hash(estate);
    let actual = load_actual(state_dir)?;
    let Some(actual) = actual else {
        let places = drift_placements(estate, state_dir)?;
        return Ok(DriftReport {
            in_sync: false,
            desired_hash,
            actual_hash: None,
            missing_agents: estate.agents.iter().map(|a| a.id.clone()).collect(),
            extra_agents: vec![],
            mismatched_sessions: vec![],
            missing_session_dirs: vec![],
            missing_lane_roots: vec![],
            missing_leases: places.missing_leases,
            extra_leases: places.extra_leases,
            spawned_cloud_agents: places.spawned_cloud_agents,
            lease_kind_mismatch: places.lease_kind_mismatch,
            host_class_mismatch: places.host_class_mismatch,
            notes: vec!["no actual-state.json; run apply".into()],
        });
    };

    let desired: std::collections::BTreeSet<_> =
        estate.agents.iter().map(|a| a.id.clone()).collect();
    let have: std::collections::BTreeSet<_> = actual
        .sessions
        .iter()
        .map(|s| s.agent_id.clone())
        .collect();
    let missing: Vec<String> = desired.difference(&have).cloned().collect();
    let extra: Vec<String> = have.difference(&desired).cloned().collect();
    let hash_match = actual.desired_hash == desired_hash;
    let mut mismatched_sessions = Vec::new();
    let mut missing_session_dirs = Vec::new();
    for session in &actual.sessions {
        if let Some(agent) = estate.agent(&session.agent_id) {
            if agent.desktop != session.desktop || agent.lane != session.lane_id {
                mismatched_sessions.push(session.agent_id.clone());
            }
        }
        if session.isolation_handle.driver == "profile-dir" && !session.session_path.exists() {
            missing_session_dirs.push(session.agent_id.clone());
        }
    }
    let mut missing_lane_roots = Vec::new();
    if let Some(base) = roots_base {
        for lane in &estate.lanes {
            if !base.join(&lane.root_path).is_dir() {
                missing_lane_roots.push(lane.id.clone());
            }
        }
    }
    let mut notes = Vec::new();
    if !hash_match {
        notes.push("desired estate hash differs from last apply".into());
    }
    if !missing.is_empty() {
        notes.push(format!("missing sessions: {}", missing.join(", ")));
    }
    if !extra.is_empty() {
        notes.push(format!("extra sessions: {}", extra.join(", ")));
    }
    if !mismatched_sessions.is_empty() {
        notes.push(format!(
            "session desktop/lane mismatch: {}",
            mismatched_sessions.join(", ")
        ));
    }
    if !missing_session_dirs.is_empty() {
        notes.push(format!(
            "session dirs missing (warm desktop discarded): {}",
            missing_session_dirs.join(", ")
        ));
    }
    if !missing_lane_roots.is_empty() {
        notes.push(format!(
            "lane roots missing: {}",
            missing_lane_roots.join(", ")
        ));
    }
    let places = drift_placements(estate, state_dir)?;
    notes.extend(places.notes.iter().cloned());
    let in_sync = hash_match
        && missing.is_empty()
        && extra.is_empty()
        && mismatched_sessions.is_empty()
        && missing_session_dirs.is_empty()
        && missing_lane_roots.is_empty()
        && places.in_sync();
    if in_sync {
        notes.push("actual-state matches desired estate".into());
    }
    Ok(DriftReport {
        in_sync,
        desired_hash,
        actual_hash: Some(actual.desired_hash),
        missing_agents: missing,
        extra_agents: extra,
        mismatched_sessions,
        missing_session_dirs,
        missing_lane_roots,
        missing_leases: places.missing_leases,
        extra_leases: places.extra_leases,
        spawned_cloud_agents: places.spawned_cloud_agents,
        lease_kind_mismatch: places.lease_kind_mismatch,
        host_class_mismatch: places.host_class_mismatch,
        notes,
    })
}

pub fn stop_runtime(state_dir: &Path) -> Result<(), SupervisorError> {
    if let Some(actual) = load_actual(state_dir)? {
        for session in &actual.sessions {
            journal_session(
                state_dir,
                "unspawn",
                Some(session.agent_id.as_str()),
                Some(actual.estate_name.as_str()),
                Some(actual.desired_hash.as_str()),
                "runtime discarded",
            )?;
        }
    }
    let runtime = state_dir.join("runtime");
    if runtime.exists() {
        std::fs::remove_dir_all(&runtime)?;
    }
    let sessions = state_dir.join("sessions");
    if sessions.exists() {
        std::fs::remove_dir_all(&sessions)?;
    }
    Ok(())
}

pub fn spawn_runtime_heartbeats(
    actual: &ActualState,
    state_dir: &Path,
) -> Result<PathBuf, SupervisorError> {
    let runtime = state_dir.join("runtime");
    std::fs::create_dir_all(&runtime)?;
    let mut heartbeats = serde_json::Map::new();
    for session in &actual.sessions {
        let hb = runtime.join(format!("{}-heartbeat", session.agent_id));
        std::fs::write(&hb, format!("disposable heartbeat for {}\n", session.agent_id))?;
        heartbeats.insert(
            session.agent_id.clone(),
            serde_json::Value::String(hb.display().to_string()),
        );
    }
    let pids = serde_json::json!({
        "disposable": true,
        "note": "PIDs and heartbeats are not source of truth",
        "heartbeats": heartbeats
    });
    let path = runtime.join("pids.json");
    write_pretty_json(&path, &pids)?;
    Ok(path)
}

fn now_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use isolation_driver::MemoryDriver;
    use serde::ser::Error;

    struct Boom;
    impl Serialize for Boom {
        fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
            Err(S::Error::custom("boom"))
        }
    }

    #[test]
    fn serialize_then_write_refuses_boom_and_leaves_sentinel() {
        assert!(serialize_pretty(&Boom).is_err());
        assert!(serialize_line(&Boom).is_err());
        assert!(refuse_empty_blob("").is_err());
        assert!(refuse_empty_blob("  \n").is_err());
        assert!(serialize_line(&serde_json::json!({})).is_err());

        let dir = tempfile();
        let path = dir.join("placement-actual.json");
        std::fs::write(&path, "sentinel\n").unwrap();
        assert!(write_pretty_json(&path, &Boom).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "sentinel\n");

        let log = dir.join("apply-audit.jsonl");
        std::fs::write(&log, "keep\n").unwrap();
        assert!(append_json_line(&log, &Boom).is_err());
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "keep\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn example() -> Estate {
        estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
    }

    #[test]
    fn apply_binds_three_sessions_with_swappable_driver() {
        let estate = example();
        let tmp = tempfile();
        let driver = MemoryDriver::default();
        let actual = apply(&estate, &tmp, &tmp, &driver).unwrap();
        assert_eq!(actual.sessions.len(), 3);
        let ids: Vec<_> = actual.sessions.iter().map(|s| s.agent_id.as_str()).collect();
        assert!(ids.contains(&"horizon"));
        assert!(ids.contains(&"research"));
        assert!(ids.contains(&"sanctum"));
        assert_eq!(driver.bound.lock().unwrap().len(), 3);
        let report = drift_with_roots(&estate, &tmp, Some(&tmp)).unwrap();
        assert!(report.in_sync);
        assert!(tmp.join("desired-snapshot.yaml").is_file());
        assert!(tmp.join("placement-actual.json").is_file());
        assert!(report.spawned_cloud_agents.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn classify_apply_unchanged_then_drift() {
        let estate = example();
        let tmp = tempfile();
        let state = tmp.join("state");
        assert_eq!(
            classify_apply(&estate, &state, &tmp).unwrap(),
            ApplyIdentity::Greenfield
        );
        apply_with_profile_dir(&estate, &state, &tmp).unwrap();
        assert_eq!(
            classify_apply(&estate, &state, &tmp).unwrap(),
            ApplyIdentity::Unchanged
        );
        stop_runtime(&state).unwrap();
        match classify_apply(&estate, &state, &tmp).unwrap() {
            ApplyIdentity::Drift { notes } => {
                assert!(!notes.is_empty());
            }
            other => panic!("expected drift, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn classify_apply_lease_refresh_after_forget() {
        let estate = example();
        let tmp = tempfile();
        let state = tmp.join("state");
        apply_with_profile_dir(&estate, &state, &tmp).unwrap();
        let mut actual = load_placements(&state).unwrap().unwrap();
        for lease in &mut actual.leases {
            lease.ttl_secs = Some(1);
            lease.issued_at = Some(1);
            lease.expires_at = Some(1);
        }
        write_placements(&state, &actual).unwrap();
        let forgotten = forget_expired_leases(&state).unwrap();
        assert!(!forgotten.is_empty());
        match classify_apply(&estate, &state, &tmp).unwrap() {
            ApplyIdentity::LeaseRefresh { missing } => {
                assert!(missing.contains(&"cell-one-box".into()));
            }
            other => panic!("expected lease-refresh, got {other:?}"),
        }
        apply_with_profile_dir(&estate, &state, &tmp).unwrap();
        assert_eq!(
            classify_apply(&estate, &state, &tmp).unwrap(),
            ApplyIdentity::Unchanged
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn drift_detects_missing_and_spawned_cloud_leases() {
        let estate = example();
        let tmp = tempfile();
        apply_with_profile_dir(&estate, &tmp, &tmp).unwrap();
        assert!(drift_with_roots(&estate, &tmp, Some(&tmp)).unwrap().in_sync);
        std::fs::remove_file(tmp.join("placement-actual.json")).unwrap();
        let missing = drift_with_roots(&estate, &tmp, Some(&tmp)).unwrap();
        assert!(!missing.in_sync);
        assert!(!missing.missing_leases.is_empty());
        record_placements(&estate, &tmp).unwrap();
        let mut actual = load_placements(&tmp).unwrap().unwrap();
        for lease in &mut actual.leases {
            if lease.kind == "cloud-agent" {
                lease.spawned = true;
            }
        }
        write_placements(&tmp, &actual).unwrap();
        let spawned = drift_with_roots(&estate, &tmp, Some(&tmp)).unwrap();
        assert!(!spawned.in_sync);
        assert!(!spawned.spawned_cloud_agents.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn drift_detects_discarded_session_dirs() {
        let estate = example();
        let tmp = tempfile();
        let state = tmp.join("state");
        apply_with_profile_dir(&estate, &state, &tmp).unwrap();
        assert!(drift_with_roots(&estate, &state, Some(&tmp)).unwrap().in_sync);
        stop_runtime(&state).unwrap();
        let report = drift_with_roots(&estate, &state, Some(&tmp)).unwrap();
        assert!(!report.in_sync);
        assert!(!report.missing_session_dirs.is_empty());
        apply_with_profile_dir(&estate, &state, &tmp).unwrap();
        assert!(drift_with_roots(&estate, &state, Some(&tmp)).unwrap().in_sync);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn profile_dir_apply_creates_desktops() {
        let estate = example();
        let tmp = tempfile();
        let actual = apply_with_profile_dir(&estate, &tmp.join("state"), &tmp).unwrap();
        assert_eq!(actual.sessions.len(), 3);
        for session in &actual.sessions {
            assert!(session.session_path.join("profile").is_dir());
            assert_eq!(session.isolation_handle.driver, "profile-dir");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    fn tempfile() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("cell-one-floor-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
