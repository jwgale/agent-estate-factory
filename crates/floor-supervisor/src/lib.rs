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
