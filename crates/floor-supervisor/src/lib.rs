//! Floor supervisor: bind per-agent sessions. Does not execute tools or models.
//! Isolation is a driver. This crate must stay free of vendor identifiers.

mod lifecycle;
mod placement;

use estate_schema::{estate_hash, Estate};
use isolation_driver::{BindRequest, BoundSession, IsolationDriver, ProfileDirDriver};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub use lifecycle::{
    lifecycle_path, load_lifecycle, mark_running, resume, suspend, write_lifecycle, LifecycleRecord,
    LifecycleState, LIFECYCLE_FILE,
};
pub use placement::{
    append_apply_audit, driver_for, drift_placements, list_apply_audits, load_placements,
    mark_leases_unspawned, record_placements, write_placements, ApplyAudit, BoxDriver,
    CloudAgentDriver, PlacementActual, PlacementDriver, PlacementDrift, PlacementLease,
};

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
    pub notes: Vec<String>,
}

pub fn apply(
    estate: &Estate,
    state_dir: &Path,
    roots_base: &Path,
    driver: &dyn IsolationDriver,
) -> Result<ActualState, SupervisorError> {
    estate_schema::validate(estate).map_err(SupervisorError::Invalid)?;
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
    let path = state_dir.join("actual-state.json");
    std::fs::write(path, serde_json::to_string_pretty(actual).unwrap_or_default())?;
    Ok(())
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
