use crate::hash::estate_hash;
use crate::types::{Estate, PlacementKind};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PlanDelta {
    pub agents: Vec<String>,
    pub lanes: Vec<String>,
    pub intentions: Vec<String>,
    pub model_bindings: Vec<String>,
    #[serde(default)]
    pub placements: Vec<String>,
    #[serde(default)]
    pub enrich_packs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EstatePlan {
    pub desired_hash: String,
    pub against_hash: Option<String>,
    pub added: PlanDelta,
    pub removed: PlanDelta,
    pub changed: PlanDelta,
    pub blast_radius_text: String,
    pub created_at: String,
}
