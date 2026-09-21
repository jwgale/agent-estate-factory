//! Operator lifecycle. Durable across restart. PIDs are not SoT.

use crate::{apply_with_profile_dir, stop_runtime, ActualState, SupervisorError};
use estate_schema::{estate_hash, Estate};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const LIFECYCLE_FILE: &str = "lifecycle.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Running,
    Suspended,
}

impl LifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            LifecycleState::Running => "running",
            LifecycleState::Suspended => "suspended",
        }
    }
}

/// Survives `stop_runtime` (sessions/PIDs die; this file stays).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleRecord {
    pub version: u32,
    pub state: LifecycleState,
    pub desired_hash: Option<String>,
    pub estate_name: Option<String>,
    pub suspended_at: Option<String>,
    pub resumed_at: Option<String>,
    pub durable: bool,
    pub note: String,
}

impl Default for LifecycleRecord {
    fn default() -> Self {
        Self {
            version: 0,
            state: LifecycleState::Suspended,
            desired_hash: None,
            estate_name: None,
            suspended_at: None,
            resumed_at: None,
            durable: true,
            note: "No lifecycle yet. Run estate resume or apply.".into(),
        }
    }
}

pub fn lifecycle_path(state_dir: &Path) -> std::path::PathBuf {
    state_dir.join(LIFECYCLE_FILE)
}

pub fn load_lifecycle(state_dir: &Path) -> Result<LifecycleRecord, SupervisorError> {
    let path = lifecycle_path(state_dir);
    if !path.exists() {
        return Ok(LifecycleRecord::default());
    }
    let text = std::fs::read_to_string(&path)?;
    serde_json::from_str(&text)
        .map_err(|e| SupervisorError::Other(format!("lifecycle.json: {e}")))
}

pub fn write_lifecycle(state_dir: &Path, record: &LifecycleRecord) -> Result<(), SupervisorError> {
    std::fs::create_dir_all(state_dir)?;
    std::fs::write(
        lifecycle_path(state_dir),
        serde_json::to_string_pretty(record).unwrap_or_default(),
    )?;
    Ok(())
}

pub fn suspend(state_dir: &Path) -> Result<LifecycleRecord, SupervisorError> {
    let prev = load_lifecycle(state_dir).ok();
    stop_runtime(state_dir)?;
    crate::mark_leases_unspawned(state_dir)?;
    let record = LifecycleRecord {
        version: 0,
        state: LifecycleState::Suspended,
        desired_hash: prev.as_ref().and_then(|p| p.desired_hash.clone()),
        estate_name: prev.as_ref().and_then(|p| p.estate_name.clone()),
        suspended_at: Some(now_rfc3339()),
        resumed_at: prev.and_then(|p| p.resumed_at),
        durable: true,
        note: "Suspended. Sessions and PIDs discarded. Estate, lanes, plans, lifecycle.json stay."
            .into(),
    };
    write_lifecycle(state_dir, &record)?;
    Ok(record)
}

pub fn mark_running(estate: &Estate, state_dir: &Path) -> Result<LifecycleRecord, SupervisorError> {
    let prev = load_lifecycle(state_dir).ok();
    let record = LifecycleRecord {
        version: 0,
        state: LifecycleState::Running,
        desired_hash: Some(estate_hash(estate)),
        estate_name: Some(estate.name.clone()),
        suspended_at: prev.and_then(|p| p.suspended_at),
        resumed_at: Some(now_rfc3339()),
        durable: true,
        note: "Applied from estate file. Runtime regenerable. Cloud-agent placements not spawned."
            .into(),
    };
    write_lifecycle(state_dir, &record)?;
    Ok(record)
}

pub fn resume(
    estate: &Estate,
    state_dir: &Path,
    roots_base: &Path,
) -> Result<(ActualState, LifecycleRecord), SupervisorError> {
    let actual = apply_with_profile_dir(estate, state_dir, roots_base)?;
    crate::record_placements(estate, state_dir)?;
    let record = LifecycleRecord {
        version: 0,
        state: LifecycleState::Running,
        desired_hash: Some(estate_hash(estate)),
        estate_name: Some(estate.name.clone()),
        suspended_at: load_lifecycle(state_dir)
            .ok()
            .and_then(|p| p.suspended_at),
        resumed_at: Some(now_rfc3339()),
        durable: true,
        note: "Resumed from estate file. Runtime is regenerable. Cloud-agent placements not spawned."
            .into(),
    };
    write_lifecycle(state_dir, &record)?;
    Ok((actual, record))
}

fn now_rfc3339() -> String {
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
    use crate::drift_with_roots;

    fn example() -> Estate {
        estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
    }

    fn tmp() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("cell-one-life-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn suspend_keeps_lifecycle_after_session_discard() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        resume(&estate, &state, &root).unwrap();
        assert!(drift_with_roots(&estate, &state, Some(&root)).unwrap().in_sync);
        assert_eq!(
            load_lifecycle(&state).unwrap().state,
            LifecycleState::Running
        );
        suspend(&state).unwrap();
        assert!(lifecycle_path(&state).is_file());
        assert!(!state.join("sessions").exists());
        assert!(lifecycle_path(&state).is_file());
        let leases = crate::load_placements(&state)
            .unwrap()
            .expect("leases survive suspend");
        assert!(leases.leases.iter().all(|l| !l.spawned));
        assert_eq!(
            load_lifecycle(&state).unwrap().state,
            LifecycleState::Suspended
        );
        assert!(!drift_with_roots(&estate, &state, Some(&root)).unwrap().in_sync);
        resume(&estate, &state, &root).unwrap();
        assert!(drift_with_roots(&estate, &state, Some(&root)).unwrap().in_sync);
        assert_eq!(
            load_lifecycle(&state).unwrap().state,
            LifecycleState::Running
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn resume_does_not_spawn_cloud_agent_sessions() {
        let estate = example();
        let root = tmp();
        let (actual, _) = resume(&estate, &root.join("state"), &root).unwrap();
        assert_eq!(actual.sessions.len(), estate.agents.len());
        assert!(estate
            .placements
            .iter()
            .any(|p| p.kind == estate_schema::PlacementKind::CloudAgent));
        let _ = std::fs::remove_dir_all(&root);
    }
}
