//! Durable placement / lease actual-state.
//!
//! Placement is a driver. `box` may spawn sessions this floor binds.
//! `cloud-agent` is declared and never spawned.

use crate::SupervisorError;
use estate_schema::{estate_hash, Estate, Placement, PlacementKind};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
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
    #[serde(default)]
    pub driver: String,
    #[serde(default)]
    pub note: Option<String>,
}

fn default_placement_schema() -> String {
    "cell-one.placement-actual.v0".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlacementActual {
    #[serde(default = "default_placement_schema")]
    pub schema: String,
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
    #[serde(default)]
    pub covering_plan: Option<String>,
    /// Always false on this beachhead. Cloud-agent is declared, not spawned.
    #[serde(default)]
    pub cloud_agent_spawned: bool,
    #[serde(default)]
    pub fresh_plan: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlacementDrift {
    pub missing_leases: Vec<String>,
    pub extra_leases: Vec<String>,
    pub spawned_cloud_agents: Vec<String>,
    pub lease_kind_mismatch: Vec<String>,
    #[serde(default)]
    pub host_class_mismatch: Vec<String>,
    pub notes: Vec<String>,
}

impl PlacementDrift {
    pub fn in_sync(&self) -> bool {
        self.missing_leases.is_empty()
            && self.extra_leases.is_empty()
            && self.spawned_cloud_agents.is_empty()
            && self.lease_kind_mismatch.is_empty()
            && self.host_class_mismatch.is_empty()
    }
}

/// Swappable placement backend. Floor talks to this trait only.
pub trait PlacementDriver: Send + Sync {
    fn name(&self) -> &'static str;
    fn claim(&self, placement: &Placement) -> PlacementLease;
    /// Sessions/PIDs died. Lease file stays; spawned drops.
    fn release(&self, lease: &PlacementLease) -> PlacementLease {
        let mut next = lease.clone();
        next.spawned = false;
        next
    }
}

/// This Cell One box. Floor may mark the lease spawned when wired.
pub struct BoxDriver;

impl PlacementDriver for BoxDriver {
    fn name(&self) -> &'static str {
        "box"
    }

    fn claim(&self, placement: &Placement) -> PlacementLease {
        PlacementLease {
            placement_id: placement.id.clone(),
            kind: PlacementKind::Box.as_str().to_string(),
            host_class: placement
                .host_class
                .clone()
                .unwrap_or_else(|| "any".into()),
            agents: placement.agents.clone(),
            wired: placement.wired,
            spawned: placement.wired,
            durable: true,
            driver: self.name().to_string(),
            note: Some("box lease. Sessions are regenerable from the estate file.".into()),
        }
    }
}

/// Declared remote placement. Never spawned, even if `wired: true`.
pub struct CloudAgentDriver;

impl PlacementDriver for CloudAgentDriver {
    fn name(&self) -> &'static str {
        "cloud-agent"
    }

    fn claim(&self, placement: &Placement) -> PlacementLease {
        PlacementLease {
            placement_id: placement.id.clone(),
            kind: PlacementKind::CloudAgent.as_str().to_string(),
            host_class: placement
                .host_class
                .clone()
                .unwrap_or_else(|| "any".into()),
            agents: placement.agents.clone(),
            wired: placement.wired,
            spawned: false,
            durable: true,
            driver: self.name().to_string(),
            note: Some(
                "declared placement stub. Floor records the lease and does not spawn it.".into(),
            ),
        }
    }
}

pub fn driver_for(kind: PlacementKind) -> Box<dyn PlacementDriver> {
    match kind {
        PlacementKind::Box => Box::new(BoxDriver),
        PlacementKind::CloudAgent => Box::new(CloudAgentDriver),
    }
}

pub fn record_placements(estate: &Estate, state_dir: &Path) -> Result<PlacementActual, SupervisorError> {
    std::fs::create_dir_all(state_dir)?;
    let actual = PlacementActual {
        schema: default_placement_schema(),
        desired_hash: estate_hash(estate),
        leases: estate
            .placements
            .iter()
            .map(|p| driver_for(p.kind).claim(p))
            .collect(),
    };
    write_placements(state_dir, &actual)?;
    Ok(actual)
}

pub fn write_placements(state_dir: &Path, actual: &PlacementActual) -> Result<(), SupervisorError> {
    std::fs::create_dir_all(state_dir)?;
    std::fs::write(
        state_dir.join("placement-actual.json"),
        serde_json::to_string_pretty(actual).unwrap_or_default(),
    )?;
    Ok(())
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

/// Sessions/PIDs died. Keep the lease file; mark every lease unspawned.
pub fn mark_leases_unspawned(state_dir: &Path) -> Result<Option<PlacementActual>, SupervisorError> {
    let Some(mut actual) = load_placements(state_dir)? else {
        return Ok(None);
    };
    for lease in &mut actual.leases {
        let kind = if lease.kind == PlacementKind::CloudAgent.as_str() {
            PlacementKind::CloudAgent
        } else {
            PlacementKind::Box
        };
        *lease = driver_for(kind).release(lease);
    }
    write_placements(state_dir, &actual)?;
    Ok(Some(actual))
}

pub fn drift_placements(estate: &Estate, state_dir: &Path) -> Result<PlacementDrift, SupervisorError> {
    let desired_ids: std::collections::BTreeSet<String> =
        estate.placements.iter().map(|p| p.id.clone()).collect();
    let Some(actual) = load_placements(state_dir)? else {
        if desired_ids.is_empty() {
            return Ok(PlacementDrift::default());
        }
        return Ok(PlacementDrift {
            missing_leases: desired_ids.into_iter().collect(),
            extra_leases: vec![],
            spawned_cloud_agents: vec![],
            lease_kind_mismatch: vec![],
            host_class_mismatch: vec![],
            notes: vec!["no placement-actual.json; run apply or resume".into()],
        });
    };

    let have_ids: std::collections::BTreeSet<String> =
        actual.leases.iter().map(|l| l.placement_id.clone()).collect();
    let missing_leases: Vec<String> = desired_ids.difference(&have_ids).cloned().collect();
    let extra_leases: Vec<String> = have_ids.difference(&desired_ids).cloned().collect();

    let mut spawned_cloud_agents = Vec::new();
    let mut lease_kind_mismatch = Vec::new();
    let mut host_class_mismatch = Vec::new();
    for placement in &estate.placements {
        let Some(lease) = actual
            .leases
            .iter()
            .find(|l| l.placement_id == placement.id)
        else {
            continue;
        };
        if lease.kind != placement.kind.as_str() {
            lease_kind_mismatch.push(placement.id.clone());
        }
        let desired_host = placement
            .host_class
            .as_deref()
            .unwrap_or("any");
        if lease.host_class != desired_host {
            host_class_mismatch.push(placement.id.clone());
        }
        if placement.kind == PlacementKind::CloudAgent && lease.spawned {
            spawned_cloud_agents.push(placement.id.clone());
        }
    }
    // Tampered actual with a spawned cloud lease not on the estate still fail-closes.
    for lease in &actual.leases {
        if lease.kind == PlacementKind::CloudAgent.as_str() && lease.spawned {
            if !spawned_cloud_agents.contains(&lease.placement_id) {
                spawned_cloud_agents.push(lease.placement_id.clone());
            }
        }
    }

    let mut notes = Vec::new();
    if actual.desired_hash != estate_hash(estate) {
        notes.push("placement-actual hash differs from desired estate".into());
    }
    if !missing_leases.is_empty() {
        notes.push(format!("missing leases: {}", missing_leases.join(", ")));
    }
    if !extra_leases.is_empty() {
        notes.push(format!("extra leases: {}", extra_leases.join(", ")));
    }
    if !lease_kind_mismatch.is_empty() {
        notes.push(format!(
            "lease kind mismatch: {}",
            lease_kind_mismatch.join(", ")
        ));
    }
    if !host_class_mismatch.is_empty() {
        notes.push(format!(
            "lease host_class mismatch: {}",
            host_class_mismatch.join(", ")
        ));
    }
    if !spawned_cloud_agents.is_empty() {
        notes.push(format!(
            "cloud-agent lease spawned (fail closed): {}",
            spawned_cloud_agents.join(", ")
        ));
    }
    if notes.is_empty() {
        notes.push("placement leases match desired estate; cloud-agent not spawned".into());
    }

    Ok(PlacementDrift {
        missing_leases,
        extra_leases,
        spawned_cloud_agents,
        lease_kind_mismatch,
        host_class_mismatch,
        notes,
    })
}

pub fn append_apply_audit(
    plans_dir: &Path,
    state_dir: &Path,
    audit: &ApplyAudit,
) -> Result<std::path::PathBuf, SupervisorError> {
    if audit.cloud_agent_spawned {
        return Err(SupervisorError::Other(
            "apply audit refuse: cloud-agent must not be spawned".into(),
        ));
    }
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

pub fn list_apply_audits(state_dir: &Path) -> Result<Vec<ApplyAudit>, SupervisorError> {
    let path = state_dir.join("apply-audit.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(&path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let audit: ApplyAudit = serde_json::from_str(line).map_err(|e| {
            SupervisorError::Other(format!("apply-audit.jsonl line {}: {e}", i + 1))
        })?;
        out.push(audit);
    }
    Ok(out)
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
        assert_eq!(cloud.driver, "cloud-agent");
        let box_lease = actual
            .leases
            .iter()
            .find(|l| l.kind == "box")
            .expect("box lease");
        assert!(box_lease.spawned);
        assert_eq!(box_lease.driver, "box");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn cloud_agent_driver_never_spawns_when_wired() {
        let placement = Placement {
            id: "remote-stub".into(),
            kind: PlacementKind::CloudAgent,
            host_class: Some("any".into()),
            agents: vec!["horizon".into()],
            wired: true,
            params: serde_json::json!({}),
            note: None,
        };
        let lease = CloudAgentDriver.claim(&placement);
        assert!(!lease.spawned);
        assert_eq!(lease.driver, "cloud-agent");
        assert!(lease.wired);
        let released = CloudAgentDriver.release(&lease);
        assert!(!released.spawned);
    }

    #[test]
    fn drift_fail_closes_host_class_mismatch() {
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-place-host-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut actual = record_placements(&estate, &tmp).unwrap();
        assert_eq!(actual.schema, "cell-one.placement-actual.v0");
        for lease in &mut actual.leases {
            lease.host_class = "consumer-nvidia".into();
        }
        write_placements(&tmp, &actual).unwrap();
        let drift = drift_placements(&estate, &tmp).unwrap();
        assert!(!drift.in_sync());
        assert!(!drift.host_class_mismatch.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn drift_fail_closes_spawned_cloud_lease() {
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-place-drift-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut actual = record_placements(&estate, &tmp).unwrap();
        for lease in &mut actual.leases {
            if lease.kind == "cloud-agent" {
                lease.spawned = true;
            }
        }
        write_placements(&tmp, &actual).unwrap();
        let drift = drift_placements(&estate, &tmp).unwrap();
        assert!(!drift.in_sync());
        assert!(!drift.spawned_cloud_agents.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn apply_audit_refuses_spawned_cloud() {
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-audit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let err = append_apply_audit(
            &tmp.join("plans"),
            &tmp.join("state"),
            &ApplyAudit {
                created_at: "unix:1".into(),
                desired_hash: "sha256:abcd".into(),
                sessions: 0,
                imported_packs: vec![],
                require_plan: false,
                note: "nope".into(),
                covering_plan: None,
                cloud_agent_spawned: true,
                fresh_plan: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("cloud-agent"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
