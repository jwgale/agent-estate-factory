//! Durable placement / lease actual-state. Cloud-agent is declared, not spawned.

use crate::SupervisorError;
use estate_schema::{estate_hash, Estate, PlacementKind};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlacementLease {
    pub placement_id: String,
    pub kind: String,
    pub host_class: String,
    pub agents: Vec<String>,
    pub wired: bool,
    /// True only for `box` sessions this floor actually bound.
    pub spawned: bool,
    pub durable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlacementActual {
    pub desired_hash: String,
    pub leases: Vec<PlacementLease>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyAudit {
    pub created_at: String,
    pub desired_hash: String,
    pub sessions: usize,
    pub imported_packs: Vec<String>,
    pub require_plan: bool,
    pub note: String,
}

pub fn record_placements(estate: &Estate, state_dir: &Path) -> Result<PlacementActual, SupervisorError> {
    std::fs::create_dir_all(state_dir)?;
    let actual = PlacementActual {
        desired_hash: estate_hash(estate),
        leases: estate
            .placements
            .iter()
            .map(|p| PlacementLease {
                placement_id: p.id.clone(),
                kind: p.kind.as_str().to_string(),
                host_class: p.host_class.clone().unwrap_or_else(|| "any".into()),
                agents: p.agents.clone(),
                wired: p.wired,
                spawned: p.kind == PlacementKind::Box && p.wired,
                durable: true,
            })
            .collect(),
    };
    std::fs::write(
        state_dir.join("placement-actual.json"),
        serde_json::to_string_pretty(&actual).unwrap_or_default(),
    )?;
    Ok(actual)
}

pub fn load_placements(state_dir: &Path) -> Result<Option<PlacementActual>, SupervisorError> {
    let path = state_dir.join("placement-actual.json");
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)?;
    let actual = serde_json::from_str(&text)
        .map_err(|e| SupervisorError::Other(format!("placement-actual.json: {e}")))?;
    Ok(Some(actual))
}

pub fn append_apply_audit(
    plans_dir: &Path,
    state_dir: &Path,
    audit: &ApplyAudit,
) -> Result<std::path::PathBuf, SupervisorError> {
    std::fs::create_dir_all(plans_dir)?;
    std::fs::create_dir_all(state_dir)?;
    let stamp = audit.created_at.replace(':', "").replace('-', "");
    let short = audit
        .desired_hash
        .trim_start_matches("sha256:")
        .chars()
        .take(8)
        .collect::<String>();
    let path = plans_dir.join(format!("apply-{stamp}-{short}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(audit).unwrap_or_default())?;
    let log = state_dir.join("apply-audit.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)?;
    use std::io::Write;
    writeln!(
        file,
        "{}",
        serde_json::to_string(audit).unwrap_or_default()
    )?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_agent_lease_is_not_spawned() {
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-place-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let actual = record_placements(&estate, &tmp).unwrap();
        let cloud = actual
            .leases
            .iter()
            .find(|l| l.kind == "cloud-agent")
            .expect("cloud-agent lease");
        assert!(!cloud.spawned);
        let box_lease = actual
            .leases
            .iter()
            .find(|l| l.kind == "box")
            .expect("box lease");
        assert!(box_lease.spawned);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
