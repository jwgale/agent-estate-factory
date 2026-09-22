//! Durable placement / lease actual-state.
//!
//! Placement is a driver. `box` may spawn sessions this floor binds.
//! `cloud-agent` is declared and never spawned.

use crate::SupervisorError;
use estate_schema::{
    canonical_host_class_opt, estate_hash, host_class_eq, is_sacred_name, Estate, Placement,
    PlacementKind,
};
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
    /// Optional lease lifetime. Absent = no expiry.
    #[serde(default)]
    pub ttl_secs: Option<u64>,
    #[serde(default)]
    pub issued_at: Option<u64>,
    #[serde(default)]
    pub expires_at: Option<u64>,
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
    /// Second apply with identical desired state and no drift.
    #[serde(default)]
    pub unchanged: bool,
    /// Operator overrode a drift refuse.
    #[serde(default)]
    pub forced: bool,
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
            host_class: lease_host_class(placement),
            agents: placement.agents.clone(),
            wired: placement.wired,
            spawned: placement.wired,
            durable: true,
            driver: self.name().to_string(),
            note: Some("box lease. Sessions are regenerable from the estate file.".into()),
            ttl_secs: None,
            issued_at: None,
            expires_at: None,
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
            host_class: lease_host_class(placement),
            agents: placement.agents.clone(),
            wired: placement.wired,
            spawned: false,
            durable: true,
            driver: self.name().to_string(),
            note: Some(
                "declared placement stub. Floor records the lease and does not spawn it.".into(),
            ),
            ttl_secs: None,
            issued_at: None,
            expires_at: None,
        }
    }
}

pub fn driver_for(kind: PlacementKind) -> Box<dyn PlacementDriver> {
    match kind {
        PlacementKind::Box => Box::new(BoxDriver),
        PlacementKind::CloudAgent => Box::new(CloudAgentDriver),
    }
}

pub fn now_unix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Locked name, or the raw string when unknown. Never invent `any` for a SKU.
fn lease_host_class(placement: &Placement) -> String {
    canonical_host_class_opt(placement.host_class.as_deref())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            placement
                .host_class
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "any".into())
        })
}

fn refuse_desired_host_class(estate: &Estate) -> Result<(), SupervisorError> {
    for placement in &estate.placements {
        if canonical_host_class_opt(placement.host_class.as_deref()).is_none() {
            return Err(SupervisorError::Other(format!(
                "refuse:bad-host-class: placement '{}' host_class '{}' must be consumer-nvidia|apple-silicon|rented-nvidia|any",
                placement.id,
                placement.host_class.as_deref().unwrap_or(""),
            )));
        }
    }
    Ok(())
}

/// Tampered `placement-actual.json` host_class must not be read as portable.
pub fn refuse_lease_host_classes(actual: &PlacementActual) -> Result<(), SupervisorError> {
    for lease in &actual.leases {
        if canonical_host_class_opt(Some(lease.host_class.as_str())).is_none() {
            return Err(SupervisorError::Other(format!(
                "refuse:bad-host-class: lease '{}' host_class '{}' must be consumer-nvidia|apple-silicon|rented-nvidia|any",
                lease.placement_id, lease.host_class
            )));
        }
    }
    Ok(())
}

fn stamp_ttl(lease: &mut PlacementLease, placement: &Placement, now: u64) {
    if let Some(ttl) = placement.ttl_secs.filter(|t| *t > 0) {
        lease.ttl_secs = Some(ttl);
        lease.issued_at = Some(now);
        lease.expires_at = Some(now.saturating_add(ttl));
    }
}

/// Claim leases from desired-state. Does not write. Used by apply and dry-run.
pub fn claim_leases(estate: &Estate) -> PlacementActual {
    let now = now_unix();
    PlacementActual {
        schema: default_placement_schema(),
        desired_hash: estate_hash(estate),
        leases: estate
            .placements
            .iter()
            .map(|p| {
                let mut lease = driver_for(p.kind).claim(p);
                stamp_ttl(&mut lease, p, now);
                lease
            })
            .collect(),
    }
}

pub fn lease_is_expired(lease: &PlacementLease, now: u64) -> bool {
    lease.expires_at.map(|exp| now >= exp).unwrap_or(false)
}

pub fn list_expired_leases(
    state_dir: &Path,
    now: u64,
) -> Result<Vec<PlacementLease>, SupervisorError> {
    let Some(actual) = load_placements(state_dir)? else {
        return Ok(Vec::new());
    };
    Ok(actual
        .leases
        .into_iter()
        .filter(|l| lease_is_expired(l, now))
        .collect())
}

pub fn refuse_expired_leases(state_dir: &Path) -> Result<(), SupervisorError> {
    let expired = list_expired_leases(state_dir, now_unix())?;
    if expired.is_empty() {
        return Ok(());
    }
    let ids = expired
        .iter()
        .map(|l| l.placement_id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(SupervisorError::Other(format!(
        "refuse:expired: lease ttl elapsed for {ids}"
    )))
}

/// Drop expired rows so apply can record fresh leases. Does not spawn.
pub fn forget_expired_leases(state_dir: &Path) -> Result<Vec<String>, SupervisorError> {
    let Some(mut actual) = load_placements(state_dir)? else {
        return Ok(Vec::new());
    };
    let now = now_unix();
    let mut forgotten = Vec::new();
    actual.leases.retain(|l| {
        if lease_is_expired(l, now) {
            forgotten.push(l.placement_id.clone());
            false
        } else {
            true
        }
    });
    if !forgotten.is_empty() {
        write_placements(state_dir, &actual)?;
    }
    Ok(forgotten)
}

/// A present spawned cloud-agent lease is a refuse before a rewrite.
/// A missing file is not a spawned lease. A file that does not parse
/// is a refuse. The caller must not replace that file with a fresh claim.
pub fn refuse_spawned_cloud_placement(state_dir: &Path) -> Result<(), SupervisorError> {
    let Some(actual) = load_placements(state_dir)? else {
        return Ok(());
    };
    let mut hits = Vec::new();
    for lease in &actual.leases {
        if lease.kind == "cloud-agent" && lease.spawned {
            hits.push(lease.placement_id.clone());
        }
    }
    if hits.is_empty() {
        return Ok(());
    }
    Err(SupervisorError::Other(format!(
        "refuse:cloud-spawned: cloud-agent lease spawned (fail closed): {}",
        hits.join(", ")
    )))
}

pub fn record_placements(estate: &Estate, state_dir: &Path) -> Result<PlacementActual, SupervisorError> {
    refuse_desired_host_class(estate)?;
    refuse_spawned_cloud_placement(state_dir)?;
    std::fs::create_dir_all(state_dir)?;
    let actual = claim_leases(estate);
    write_placements(state_dir, &actual)?;
    Ok(actual)
}

pub fn write_placements(state_dir: &Path, actual: &PlacementActual) -> Result<(), SupervisorError> {
    std::fs::create_dir_all(state_dir)?;
    crate::write_pretty_json(&state_dir.join("placement-actual.json"), actual)
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
///
/// A spawned cloud-agent lease is not a session that died. Refuse before
/// the rewrite. A missing file is not a spawned lease. A wired box lease
/// still drops `spawned`.
pub fn mark_leases_unspawned(state_dir: &Path) -> Result<Option<PlacementActual>, SupervisorError> {
    refuse_spawned_cloud_placement(state_dir)?;
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
        match canonical_host_class_opt(placement.host_class.as_deref()) {
            Some(desired_host) => {
                if !host_class_eq(&lease.host_class, desired_host) {
                    host_class_mismatch.push(placement.id.clone());
                }
            }
            None => host_class_mismatch.push(placement.id.clone()),
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
        notes.push(format!(
            "refuse:missing-lease: {}",
            missing_leases.join(", ")
        ));
    }
    if !extra_leases.is_empty() {
        notes.push(format!("refuse:extra-lease: {}", extra_leases.join(", ")));
    }
    if !lease_kind_mismatch.is_empty() {
        notes.push(format!(
            "refuse:kind-mismatch: {}",
            lease_kind_mismatch.join(", ")
        ));
    }
    if !host_class_mismatch.is_empty() {
        notes.push(format!(
            "refuse:host-class-mismatch: {}",
            host_class_mismatch.join(", ")
        ));
    }
    if !spawned_cloud_agents.is_empty() {
        notes.push(format!(
            "refuse:cloud-spawned: cloud-agent lease spawned (fail closed): {}",
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

pub const RECONCILE_SCHEMA: &str = "cell-one.reconcile.v0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Refuse {
    pub code: String,
    pub subject: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReconcileRow {
    pub id: String,
    pub side: String,
    pub kind: String,
    pub host_class: String,
    pub agents: Vec<String>,
    pub wired: bool,
    #[serde(default)]
    pub spawned: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReconcileReport {
    #[serde(default = "default_reconcile_schema")]
    pub schema: String,
    pub in_sync: bool,
    pub desired_hash: String,
    pub actual_hash: Option<String>,
    pub desired: Vec<ReconcileRow>,
    pub actual: Vec<ReconcileRow>,
    pub refuses: Vec<Refuse>,
    pub notes: Vec<String>,
}

fn default_reconcile_schema() -> String {
    RECONCILE_SCHEMA.into()
}

/// Desired vs actual placement reconcile. Sacred-id on an actual lease fail-closes.
pub fn reconcile_placements(
    estate: &Estate,
    state_dir: &Path,
) -> Result<ReconcileReport, SupervisorError> {
    let drift = drift_placements(estate, state_dir)?;
    let actual = load_placements(state_dir)?;
    let desired: Vec<ReconcileRow> = estate
        .placements
        .iter()
        .map(|p| ReconcileRow {
            id: p.id.clone(),
            side: "desired".into(),
            kind: p.kind.as_str().into(),
            host_class: canonical_host_class_opt(p.host_class.as_deref())
                .map(|s| s.to_string())
                .unwrap_or_else(|| p.host_class.clone().unwrap_or_default()),
            agents: p.agents.clone(),
            wired: p.wired,
            spawned: None,
        })
        .collect();
    let actual_rows: Vec<ReconcileRow> = actual
        .as_ref()
        .map(|a| {
            a.leases
                .iter()
                .map(|l| ReconcileRow {
                    id: l.placement_id.clone(),
                    side: "actual".into(),
                    kind: l.kind.clone(),
                    host_class: l.host_class.clone(),
                    agents: l.agents.clone(),
                    wired: l.wired,
                    spawned: Some(l.spawned),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut refuses = Vec::new();
    for id in &drift.missing_leases {
        refuses.push(Refuse {
            code: "missing-lease".into(),
            subject: id.clone(),
            reason: format!("desired placement '{id}' has no lease in placement-actual.json"),
        });
    }
    for id in &drift.extra_leases {
        refuses.push(Refuse {
            code: "extra-lease".into(),
            subject: id.clone(),
            reason: format!("actual lease '{id}' is not on the desired estate"),
        });
    }
    for id in &drift.lease_kind_mismatch {
        refuses.push(Refuse {
            code: "kind-mismatch".into(),
            subject: id.clone(),
            reason: format!("lease kind for '{id}' does not match desired placement"),
        });
    }
    for id in &drift.host_class_mismatch {
        refuses.push(Refuse {
            code: "host-class-mismatch".into(),
            subject: id.clone(),
            reason: format!("lease host_class for '{id}' drifted from desired (fail closed)"),
        });
    }
    for id in &drift.spawned_cloud_agents {
        refuses.push(Refuse {
            code: "cloud-spawned".into(),
            subject: id.clone(),
            reason: format!("cloud-agent '{id}' lease is spawned; floor must not spawn it"),
        });
    }
    if let Some(actual) = actual.as_ref() {
        let now = now_unix();
        for lease in &actual.leases {
            for agent in &lease.agents {
                if is_sacred_name(agent) || estate.is_sacred(agent) {
                    refuses.push(Refuse {
                        code: "sacred-id".into(),
                        subject: format!("{}:{}", lease.placement_id, agent),
                        reason: format!(
                            "refuse:sacred-id: lease '{}' must not bind sacred exclusion '{agent}'",
                            lease.placement_id
                        ),
                    });
                }
            }
            if lease_is_expired(lease, now) {
                refuses.push(Refuse {
                    code: "expired".into(),
                    subject: lease.placement_id.clone(),
                    reason: format!(
                        "refuse:expired: lease '{}' ttl elapsed (expires_at={})",
                        lease.placement_id,
                        lease.expires_at.unwrap_or(0)
                    ),
                });
            }
            if canonical_host_class_opt(Some(lease.host_class.as_str())).is_none() {
                refuses.push(Refuse {
                    code: "bad-host-class".into(),
                    subject: lease.placement_id.clone(),
                    reason: format!(
                        "refuse:bad-host-class: lease '{}' host_class '{}' must be consumer-nvidia|apple-silicon|rented-nvidia|any",
                        lease.placement_id, lease.host_class
                    ),
                });
            }
        }
    }

    let in_sync = drift.in_sync()
        && refuses.iter().all(|r| {
            r.code != "sacred-id" && r.code != "expired" && r.code != "bad-host-class"
        });
    let mut notes = drift.notes;
    if refuses.iter().any(|r| r.code == "sacred-id") {
        notes.push("refuse:sacred-id: actual lease binds a sacred exclusion".into());
    }
    if refuses.iter().any(|r| r.code == "expired") {
        notes.push("refuse:expired: one or more leases have elapsed ttl".into());
    }
    if refuses.iter().any(|r| r.code == "bad-host-class") {
        notes.push("refuse:bad-host-class: actual lease host_class is not a locked name".into());
    }
    Ok(ReconcileReport {
        schema: RECONCILE_SCHEMA.into(),
        in_sync,
        desired_hash: estate_hash(estate),
        actual_hash: actual.as_ref().map(|a| a.desired_hash.clone()),
        desired,
        actual: actual_rows,
        refuses,
        notes,
    })
}

pub fn render_reconcile(report: &ReconcileReport) -> String {
    let mut out = String::from("Placement reconcile (desired vs actual)\n");
    out.push_str("======================================\n");
    out.push_str(&format!("schema: {}\n", report.schema));
    out.push_str(&format!("in_sync: {}\n", report.in_sync));
    out.push_str(&format!("desired_hash: {}\n", report.desired_hash));
    out.push_str(&format!(
        "actual_hash: {}\n\n",
        report.actual_hash.as_deref().unwrap_or("(none)")
    ));
    out.push_str("Desired\n-------\n");
    if report.desired.is_empty() {
        out.push_str("(none)\n");
    } else {
        for row in &report.desired {
            out.push_str(&format!(
                "  {:<16} kind={:<12} host_class={:<16} wired={} agents={}\n",
                row.id,
                row.kind,
                row.host_class,
                row.wired,
                if row.agents.is_empty() {
                    "(none)".into()
                } else {
                    row.agents.join(",")
                }
            ));
        }
    }
    out.push_str("\nActual\n------\n");
    if report.actual.is_empty() {
        out.push_str("(no placement-actual.json; run apply)\n");
    } else {
        for row in &report.actual {
            out.push_str(&format!(
                "  {:<16} kind={:<12} host_class={:<16} wired={} spawned={} agents={}\n",
                row.id,
                row.kind,
                row.host_class,
                row.wired,
                row.spawned
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "-".into()),
                if row.agents.is_empty() {
                    "(none)".into()
                } else {
                    row.agents.join(",")
                }
            ));
        }
    }
    out.push_str("\nRefuses\n-------\n");
    if report.refuses.is_empty() {
        out.push_str("(none)\n");
    } else {
        for r in &report.refuses {
            out.push_str(&format!("  refuse:{}: {} — {}\n", r.code, r.subject, r.reason));
        }
    }
    out.push_str("\nNotes\n-----\n");
    for note in &report.notes {
        out.push_str(&format!("  {note}\n"));
    }
    out
}

pub fn write_reconcile(
    state_dir: &Path,
    report: &ReconcileReport,
) -> Result<std::path::PathBuf, SupervisorError> {
    std::fs::create_dir_all(state_dir)?;
    let json = state_dir.join("reconcile.json");
    crate::write_pretty_json(&json, report)?;
    std::fs::write(state_dir.join("reconcile.md"), render_reconcile(report))?;
    Ok(json)
}

pub const DRY_RUN_SCHEMA: &str = "cell-one.apply-dry-run.v0";

/// Desired-state apply preview. Never writes leases or sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyDryRun {
    #[serde(default = "default_dry_run_schema")]
    pub schema: String,
    pub writes: bool,
    pub desired_hash: String,
    pub blast_radius: String,
    pub preview: Vec<ReconcileRow>,
    pub expired: Vec<String>,
    pub refuses: Vec<Refuse>,
    pub would_refuse: bool,
    pub notes: Vec<String>,
}

fn default_dry_run_schema() -> String {
    DRY_RUN_SCHEMA.into()
}

/// Preview apply: blast radius + reconcile + would-refuse. Does not write.
pub fn apply_dry_run(estate: &Estate, state_dir: &Path) -> Result<ApplyDryRun, SupervisorError> {
    estate_schema::validate(estate).map_err(SupervisorError::Invalid)?;
    let previous = crate::load_desired_snapshot(state_dir)?;
    let plan = estate_schema::diff_estates(estate, previous.as_ref());
    let preview_actual = claim_leases(estate);
    let recon = reconcile_placements(estate, state_dir)?;
    let expired = list_expired_leases(state_dir, now_unix())?;
    let expired_ids: Vec<String> = expired.iter().map(|l| l.placement_id.clone()).collect();

    let mut refuses = Vec::new();
    for r in &recon.refuses {
        if matches!(
            r.code.as_str(),
            "sacred-id" | "cloud-spawned" | "expired" | "bad-host-class"
        ) {
            refuses.push(r.clone());
        }
    }
    for lease in &preview_actual.leases {
        if lease.kind == PlacementKind::CloudAgent.as_str() && lease.spawned {
            refuses.push(Refuse {
                code: "cloud-spawned".into(),
                subject: lease.placement_id.clone(),
                reason: format!(
                    "refuse:cloud-spawned: preview would spawn cloud-agent '{}'",
                    lease.placement_id
                ),
            });
        }
        for agent in &lease.agents {
            if is_sacred_name(agent) || estate.is_sacred(agent) {
                refuses.push(Refuse {
                    code: "sacred-id".into(),
                    subject: format!("{}:{}", lease.placement_id, agent),
                    reason: format!(
                        "refuse:sacred-id: preview lease '{}' would bind sacred exclusion '{agent}'",
                        lease.placement_id
                    ),
                });
            }
        }
    }
    for id in &expired_ids {
        if !refuses.iter().any(|r| r.code == "expired" && r.subject == *id) {
            refuses.push(Refuse {
                code: "expired".into(),
                subject: id.clone(),
                reason: format!("refuse:expired: lease '{id}' ttl elapsed; apply/resume refuse"),
            });
        }
    }

    let preview: Vec<ReconcileRow> = preview_actual
        .leases
        .iter()
        .map(|l| ReconcileRow {
            id: l.placement_id.clone(),
            side: "preview".into(),
            kind: l.kind.clone(),
            host_class: l.host_class.clone(),
            agents: l.agents.clone(),
            wired: l.wired,
            spawned: Some(l.spawned),
        })
        .collect();

    let mut notes = vec![
        "dry-run: no leases, sessions, snapshots, or audits written".into(),
        format!("current reconcile in_sync={}", recon.in_sync),
    ];
    notes.extend(recon.notes.iter().cloned());

    Ok(ApplyDryRun {
        schema: DRY_RUN_SCHEMA.into(),
        writes: false,
        desired_hash: estate_hash(estate),
        blast_radius: plan.blast_radius_text,
        preview,
        expired: expired_ids,
        would_refuse: !refuses.is_empty(),
        refuses,
        notes,
    })
}

pub fn render_dry_run(report: &ApplyDryRun) -> String {
    let mut out = String::from("Apply dry-run (no writes)\n");
    out.push_str("========================\n");
    out.push_str(&format!("schema: {}\n", report.schema));
    out.push_str(&format!("writes: {}\n", report.writes));
    out.push_str(&format!("would_refuse: {}\n", report.would_refuse));
    out.push_str(&format!("desired_hash: {}\n\n", report.desired_hash));
    out.push_str("Blast radius\n------------\n");
    out.push_str(&report.blast_radius);
    out.push_str("\n\nPreview leases (not written)\n----------------------------\n");
    if report.preview.is_empty() {
        out.push_str("(none)\n");
    } else {
        for row in &report.preview {
            out.push_str(&format!(
                "  {:<16} kind={:<12} host_class={:<16} wired={} spawned={} agents={}\n",
                row.id,
                row.kind,
                row.host_class,
                row.wired,
                row.spawned
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "-".into()),
                if row.agents.is_empty() {
                    "(none)".into()
                } else {
                    row.agents.join(",")
                }
            ));
        }
    }
    out.push_str("\nExpired\n-------\n");
    if report.expired.is_empty() {
        out.push_str("(none)\n");
    } else {
        for id in &report.expired {
            out.push_str(&format!("  refuse:expired: {id}\n"));
        }
    }
    out.push_str("\nWould-refuse\n------------\n");
    if report.refuses.is_empty() {
        out.push_str("(none)\n");
    } else {
        for r in &report.refuses {
            out.push_str(&format!("  refuse:{}: {} — {}\n", r.code, r.subject, r.reason));
        }
    }
    out.push_str("\nNotes\n-----\n");
    for note in &report.notes {
        out.push_str(&format!("  {note}\n"));
    }
    out
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
    let pretty = crate::serialize_pretty(audit)?;
    let line = crate::serialize_line(audit)?;
    std::fs::write(&path, pretty)?;
    let log = state_dir.join("apply-audit.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)?;
    use std::io::Write;
    writeln!(file, "{line}")?;
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
    fn write_placements_never_writes_empty_blob() {
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-place-empty-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let actual = claim_leases(&estate);
        write_placements(&tmp, &actual).unwrap();
        let blob = std::fs::read_to_string(tmp.join("placement-actual.json")).unwrap();
        assert!(!blob.trim().is_empty(), "write_placements must not wipe empty");
        assert!(blob.contains("cell-one.placement-actual.v0"), "{blob}");
        let loaded = load_placements(&tmp).unwrap().expect("placement-actual");
        assert_eq!(loaded.schema, "cell-one.placement-actual.v0");
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
            ttl_secs: None,
        };
        let lease = CloudAgentDriver.claim(&placement);
        assert!(!lease.spawned);
        assert_eq!(lease.driver, "cloud-agent");
        assert!(lease.wired);
        let released = CloudAgentDriver.release(&lease);
        assert!(!released.spawned);
    }

    #[test]
    fn unspawn_does_not_restamp_a_spawned_cloud_lease() {
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-place-spawned-{}-{}",
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
        let before = std::fs::read_to_string(tmp.join("placement-actual.json")).unwrap();
        let err = mark_leases_unspawned(&tmp).unwrap_err().to_string();
        assert!(err.contains("refuse:cloud-spawned"), "{err}");
        assert!(err.contains("cursor-cloud"), "{err}");
        let after = std::fs::read_to_string(tmp.join("placement-actual.json")).unwrap();
        assert_eq!(before, after);
        let still = load_placements(&tmp).unwrap().unwrap();
        assert!(still
            .leases
            .iter()
            .any(|l| l.kind == "cloud-agent" && l.spawned));
        let _ = std::fs::remove_dir_all(&tmp);
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
    fn drift_alias_host_class_is_in_sync() {
        let estate = estate_schema::load_estate_str(include_str!(
            "../../../examples/hosts/rtx-consumer.yaml"
        ))
        .unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-place-alias-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let actual = record_placements(&estate, &tmp).unwrap();
        let box_lease = actual
            .leases
            .iter()
            .find(|l| l.kind == "box")
            .expect("box lease");
        assert_eq!(box_lease.host_class, "consumer-nvidia");
        let drift = drift_placements(&estate, &tmp).unwrap();
        assert!(drift.in_sync(), "{:?}", drift.notes);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn drift_fail_closes_cross_matrix_host_class() {
        let estate = estate_schema::load_estate_str(include_str!(
            "../../../examples/hosts/rtx-consumer.yaml"
        ))
        .unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-place-cross-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut actual = record_placements(&estate, &tmp).unwrap();
        for lease in &mut actual.leases {
            lease.host_class = "apple-silicon".into();
        }
        write_placements(&tmp, &actual).unwrap();
        let drift = drift_placements(&estate, &tmp).unwrap();
        assert!(!drift.in_sync());
        assert!(!drift.host_class_mismatch.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn reconcile_in_sync_after_record() {
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-recon-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        record_placements(&estate, &tmp).unwrap();
        let report = reconcile_placements(&estate, &tmp).unwrap();
        assert!(report.in_sync, "{:?}", report.refuses);
        assert_eq!(report.schema, RECONCILE_SCHEMA);
        assert!(report.refuses.is_empty());
        assert_eq!(report.desired.len(), estate.placements.len());
        assert_eq!(report.actual.len(), estate.placements.len());
        let path = write_reconcile(&tmp, &report).unwrap();
        assert!(path.is_file());
        assert!(tmp.join("reconcile.md").is_file());
        let md = std::fs::read_to_string(tmp.join("reconcile.md")).unwrap();
        assert!(md.contains("in_sync: true"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn reconcile_extra_lease_and_kind_mismatch_refuse_codes() {
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-recon-extra-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut actual = record_placements(&estate, &tmp).unwrap();
        actual.leases[0].kind = "cloud-agent".into();
        actual.leases.push(PlacementLease {
            placement_id: "ghost-box".into(),
            kind: "box".into(),
            host_class: "any".into(),
            agents: vec![],
            wired: true,
            spawned: true,
            durable: true,
            driver: "box".into(),
            note: None,
            ttl_secs: None,
            issued_at: None,
            expires_at: None,
        });
        write_placements(&tmp, &actual).unwrap();
        let report = reconcile_placements(&estate, &tmp).unwrap();
        assert!(!report.in_sync);
        assert!(report.refuses.iter().any(|r| r.code == "extra-lease"));
        assert!(report.refuses.iter().any(|r| r.code == "kind-mismatch"));
        assert!(report.notes.iter().any(|n| n.contains("refuse:extra-lease")));
        assert!(report.notes.iter().any(|n| n.contains("refuse:kind-mismatch")));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn reconcile_sacred_id_on_actual_fail_closes() {
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let sacred = estate
            .sacred_exclusions
            .first()
            .map(|s| s.id.clone())
            .expect("locked sacred exclusion");
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-recon-sacred-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut actual = record_placements(&estate, &tmp).unwrap();
        if let Some(lease) = actual.leases.iter_mut().find(|l| l.kind == "box") {
            lease.agents.push(sacred.clone());
        }
        write_placements(&tmp, &actual).unwrap();
        let report = reconcile_placements(&estate, &tmp).unwrap();
        assert!(!report.in_sync);
        assert!(
            report.refuses.iter().any(|r| r.code == "sacred-id"),
            "{:?}",
            report.refuses
        );
        assert!(report
            .notes
            .iter()
            .any(|n| n.contains("refuse:sacred-id")));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn reconcile_host_mismatch_has_refuse_codes() {
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-recon-host-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut actual = record_placements(&estate, &tmp).unwrap();
        for lease in &mut actual.leases {
            lease.host_class = "apple-silicon".into();
        }
        write_placements(&tmp, &actual).unwrap();
        let drift = drift_placements(&estate, &tmp).unwrap();
        assert!(drift
            .notes
            .iter()
            .any(|n| n.starts_with("refuse:host-class-mismatch")));
        let report = reconcile_placements(&estate, &tmp).unwrap();
        assert!(!report.in_sync);
        assert!(report
            .refuses
            .iter()
            .any(|r| r.code == "host-class-mismatch"));
        let rendered = render_reconcile(&report);
        assert!(rendered.contains("refuse:host-class-mismatch"));
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
                unchanged: false,
                forced: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("cloud-agent"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn lease_ttl_expire_refuse_and_forget() {
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-ttl-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut actual = record_placements(&estate, &tmp).unwrap();
        let now = now_unix();
        if let Some(lease) = actual.leases.iter_mut().find(|l| l.kind == "box") {
            lease.ttl_secs = Some(1);
            lease.issued_at = Some(now.saturating_sub(10));
            lease.expires_at = Some(now.saturating_sub(1));
        }
        write_placements(&tmp, &actual).unwrap();
        assert!(!list_expired_leases(&tmp, now).unwrap().is_empty());
        let err = refuse_expired_leases(&tmp).unwrap_err();
        assert!(err.to_string().contains("refuse:expired"));
        let recon = reconcile_placements(&estate, &tmp).unwrap();
        assert!(!recon.in_sync);
        assert!(recon.refuses.iter().any(|r| r.code == "expired"));
        let forgotten = forget_expired_leases(&tmp).unwrap();
        assert!(!forgotten.is_empty());
        assert!(list_expired_leases(&tmp, now_unix()).unwrap().is_empty());
        refuse_expired_leases(&tmp).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn apply_dry_run_does_not_write_leases() {
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-dry-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let report = apply_dry_run(&estate, &tmp).unwrap();
        assert!(!report.writes);
        assert!(!report.would_refuse, "{:?}", report.refuses);
        assert_eq!(report.schema, DRY_RUN_SCHEMA);
        assert!(!report.blast_radius.is_empty());
        assert!(!tmp.join("placement-actual.json").exists());
        assert!(!tmp.join("actual-state.json").exists());
        assert!(!tmp.join("desired-snapshot.yaml").exists());
        assert!(!tmp.join("reconcile.json").exists());
        let rendered = render_dry_run(&report);
        assert!(rendered.contains("writes: false"));
        assert!(rendered.contains("Blast radius"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
