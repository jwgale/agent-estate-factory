//! Capability mesh beachhead. Same trait style as placement/local drivers.
//!
//! Declare a hop, lease-bound call, refuse without a granted lease.
//! `cloud-mesh` hops are declared and never spawned. Not a gateway.
//!
//! Slim-parses `.cell/placement-actual.json` so this crate does not depend
//! on floor-supervisor.

use estate_schema::{
    canonical_host_class_opt, contains_sku, is_sacred_name, is_slug, normalize_host_class,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const MESH_SCHEMA: &str = "cell-one.conveyor-mesh.v0";
pub const MESH_FILE: &str = "conveyor-mesh.json";
pub const HOPS_FILE: &str = "conveyor-hops.json";
pub const LEASES_FILE: &str = "conveyor-leases.json";

#[derive(Debug, Error)]
pub enum MeshError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(String),
    #[error("refuse:no-lease: no hop lease for '{0}'")]
    NoLease(String),
    #[error("refuse:ungranted: hop '{0}' has no granted lease")]
    Ungranted(String),
    #[error("refuse:cloud-not-spawned: cloud-mesh hop '{0}' is declared, not spawned")]
    CloudNotSpawned(String),
    #[error("refuse:not-live: hop '{0}' is not live (suspended or unwired)")]
    NotLive(String),
    #[error("refuse:capability: capability '{want}' is not on hop '{hop}' (have {have})")]
    Capability {
        hop: String,
        want: String,
        have: String,
    },
    #[error("refuse:kind: unknown hop kind '{0}' (use box|cloud-mesh)")]
    Kind(String),
    #[error("refuse:sku-banned: hop id '{0}' encodes a hardware SKU")]
    SkuBanned(String),
    #[error("refuse:bad-id: hop id '{0}' must match [a-z][a-z0-9_-]{{0,63}}")]
    BadId(String),
    #[error("refuse:bad-host-class: hop host_class '{0}' must be consumer-nvidia|apple-silicon|rented-nvidia|any")]
    BadHostClass(String),
    #[error("refuse:expired: hop lease ttl elapsed for '{0}'")]
    Expired(String),
    #[error("refuse:sacred-id: hop/placement '{0}' is a sacred exclusion")]
    SacredId(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HopDecl {
    pub id: String,
    pub kind: String,
    pub capability: String,
    #[serde(default = "default_host_class")]
    pub host_class: String,
    #[serde(default)]
    pub wired: bool,
    #[serde(default)]
    pub note: Option<String>,
    /// Optional hop-lease lifetime. Absent = no expiry.
    #[serde(default)]
    pub ttl_secs: Option<u64>,
}

fn default_host_class() -> String {
    "any".into()
}

/// Locked name, or the raw string. Never invents `any` for a SKU.
fn stamp_host_class(raw: &str) -> String {
    canonical_host_class_opt(Some(raw))
        .map(|s| s.to_string())
        .unwrap_or_else(|| raw.to_string())
}

/// Interpreting readers refuse unknown / SKU host_class on disk.
/// `load_mesh` stays permissive so a later overwrite can still land.
pub fn refuse_mesh_host_classes(mesh: &ConveyorMesh) -> Result<(), MeshError> {
    for hop in &mesh.hops {
        if canonical_host_class_opt(Some(hop.host_class.as_str())).is_none() {
            return Err(MeshError::BadHostClass(hop.host_class.clone()));
        }
    }
    for lease in &mesh.leases {
        if canonical_host_class_opt(Some(lease.host_class.as_str())).is_none() {
            return Err(MeshError::BadHostClass(lease.host_class.clone()));
        }
    }
    Ok(())
}

fn load_interpreted_mesh(state_dir: &Path) -> Result<ConveyorMesh, MeshError> {
    let mesh = load_mesh(state_dir)?;
    refuse_mesh_host_classes(&mesh)?;
    Ok(mesh)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HopLease {
    pub hop_id: String,
    pub kind: String,
    pub capability: String,
    pub host_class: String,
    pub granted: bool,
    pub spawned: bool,
    pub durable: bool,
    pub driver: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub ttl_secs: Option<u64>,
    #[serde(default)]
    pub issued_at: Option<u64>,
    #[serde(default)]
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HopCall {
    pub hop_id: String,
    pub capability: String,
    pub allow: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConveyorMesh {
    #[serde(default = "default_mesh_schema")]
    pub schema: String,
    pub hops: Vec<HopDecl>,
    pub leases: Vec<HopLease>,
}

fn default_mesh_schema() -> String {
    MESH_SCHEMA.into()
}

impl Default for ConveyorMesh {
    fn default() -> Self {
        Self {
            schema: MESH_SCHEMA.into(),
            hops: Vec::new(),
            leases: Vec::new(),
        }
    }
}

/// Slim placement-actual row. Avoids a floor-supervisor dependency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlimPlacement {
    pub placement_id: String,
    pub kind: String,
    #[serde(default = "default_host_class")]
    pub host_class: String,
    #[serde(default)]
    pub spawned: bool,
    #[serde(default)]
    pub wired: bool,
    #[serde(default)]
    pub ttl_secs: Option<u64>,
    #[serde(default)]
    pub agents: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SlimActual {
    #[serde(default)]
    leases: Vec<SlimLeaseRow>,
}

#[derive(Debug, Clone, Deserialize)]
struct SlimLeaseRow {
    placement_id: String,
    kind: String,
    #[serde(default = "default_host_class")]
    host_class: String,
    #[serde(default)]
    spawned: bool,
    #[serde(default)]
    wired: bool,
    #[serde(default)]
    ttl_secs: Option<u64>,
    #[serde(default)]
    agents: Vec<String>,
}

/// Swappable hop backend. Conveyor talks to this trait only.
pub trait ConveyorHop: Send + Sync {
    fn name(&self) -> &'static str;
    fn declare(&self, hop: &HopDecl) -> HopLease;
    fn call(&self, lease: &HopLease, capability: &str) -> Result<HopCall, MeshError>;
}

/// This Cell One box. May grant a lease when the hop is wired.
pub struct BoxHop;

impl ConveyorHop for BoxHop {
    fn name(&self) -> &'static str {
        "box"
    }

    fn declare(&self, hop: &HopDecl) -> HopLease {
        HopLease {
            hop_id: hop.id.clone(),
            kind: "box".into(),
            capability: hop.capability.clone(),
            // refuse_hop already locked the name. Do not invent `any`.
            host_class: stamp_host_class(&hop.host_class),
            granted: hop.wired,
            spawned: hop.wired,
            durable: true,
            driver: self.name().into(),
            note: Some("box hop lease. Call refuses without granted+".into()),
            ttl_secs: None,
            issued_at: None,
            expires_at: None,
        }
    }

    fn call(&self, lease: &HopLease, capability: &str) -> Result<HopCall, MeshError> {
        if !lease.granted {
            return Err(MeshError::Ungranted(lease.hop_id.clone()));
        }
        if !lease.spawned {
            return Err(MeshError::NotLive(lease.hop_id.clone()));
        }
        if lease.capability != capability {
            return Err(MeshError::Capability {
                hop: lease.hop_id.clone(),
                want: capability.into(),
                have: lease.capability.clone(),
            });
        }
        Ok(HopCall {
            hop_id: lease.hop_id.clone(),
            capability: capability.into(),
            allow: true,
            reason: "lease-bound box hop".into(),
        })
    }
}

/// Declared remote mesh hop. Never granted, never spawned.
pub struct CloudMeshHop;

impl ConveyorHop for CloudMeshHop {
    fn name(&self) -> &'static str {
        "cloud-mesh"
    }

    fn declare(&self, hop: &HopDecl) -> HopLease {
        HopLease {
            hop_id: hop.id.clone(),
            kind: "cloud-mesh".into(),
            capability: hop.capability.clone(),
            // refuse_hop already locked the name. Do not invent `any`.
            host_class: stamp_host_class(&hop.host_class),
            granted: false,
            spawned: false,
            durable: true,
            driver: self.name().into(),
            note: Some("declared mesh stub. Conveyor records the hop and does not spawn it.".into()),
            ttl_secs: None,
            issued_at: None,
            expires_at: None,
        }
    }

    fn call(&self, lease: &HopLease, _capability: &str) -> Result<HopCall, MeshError> {
        Err(MeshError::CloudNotSpawned(lease.hop_id.clone()))
    }
}

pub fn hop_driver(kind: &str) -> Result<Box<dyn ConveyorHop>, MeshError> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "box" => Ok(Box::new(BoxHop)),
        "cloud-mesh" | "cloud_mesh" | "cloud-agent" => Ok(Box::new(CloudMeshHop)),
        other => Err(MeshError::Kind(other.to_string())),
    }
}

pub fn refuse_hop(hop: &HopDecl) -> Result<(), MeshError> {
    if is_sacred_name(&hop.id) {
        return Err(MeshError::SacredId(hop.id.clone()));
    }
    if !is_slug(&hop.id) {
        return Err(MeshError::BadId(hop.id.clone()));
    }
    if contains_sku(&hop.id) || contains_sku(&hop.capability) {
        return Err(MeshError::SkuBanned(hop.id.clone()));
    }
    if normalize_host_class(&hop.host_class).is_none() {
        return Err(MeshError::BadHostClass(hop.host_class.clone()));
    }
    hop_driver(&hop.kind)?;
    Ok(())
}

pub fn mesh_path(state_dir: &Path) -> PathBuf {
    state_dir.join(MESH_FILE)
}

pub fn load_mesh(state_dir: &Path) -> Result<ConveyorMesh, MeshError> {
    let path = mesh_path(state_dir);
    if !path.exists() {
        return Ok(ConveyorMesh::default());
    }
    let text = std::fs::read_to_string(&path)?;
    serde_json::from_str(&text).map_err(|e| MeshError::Parse(format!("{MESH_FILE}: {e}")))
}

fn serialize_pretty<T: Serialize>(value: &T) -> Result<String, MeshError> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| MeshError::Parse(format!("serialize: {e}")))?;
    if body.trim().is_empty() {
        return Err(MeshError::Parse("serialize: empty".into()));
    }
    Ok(body)
}

pub fn persist_mesh(state_dir: &Path, mesh: &ConveyorMesh) -> Result<PathBuf, MeshError> {
    std::fs::create_dir_all(state_dir)?;
    let path = mesh_path(state_dir);
    let hops = serde_json::json!({
        "schema": MESH_SCHEMA,
        "hops": mesh.hops,
    });
    let leases = serde_json::json!({
        "schema": MESH_SCHEMA,
        "leases": mesh.leases,
    });
    let mesh_body = serialize_pretty(mesh)?;
    let hops_body = serialize_pretty(&hops)?;
    let leases_body = serialize_pretty(&leases)?;
    std::fs::write(&path, mesh_body)?;
    std::fs::write(state_dir.join(HOPS_FILE), hops_body)?;
    std::fs::write(state_dir.join(LEASES_FILE), leases_body)?;
    Ok(path)
}

pub fn slim_parse_placement_actual(path: &Path) -> Result<Vec<SlimPlacement>, MeshError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path)?;
    let actual: SlimActual = serde_json::from_str(&text)
        .map_err(|e| MeshError::Parse(format!("placement-actual.json: {e}")))?;
    let mut out = Vec::new();
    for row in actual.leases {
        let host_class = match canonical_host_class_opt(Some(row.host_class.as_str())) {
            Some(host) => host.to_string(),
            None => return Err(MeshError::BadHostClass(row.host_class)),
        };
        out.push(SlimPlacement {
            placement_id: row.placement_id,
            kind: row.kind,
            host_class,
            spawned: row.spawned,
            wired: row.wired,
            ttl_secs: row.ttl_secs,
            agents: row.agents,
        });
    }
    Ok(out)
}

/// Placement kind → hop kind. Unmatched kinds are not seeded.
pub fn hop_kind_for_placement(kind: &str) -> Option<&'static str> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "box" => Some("box"),
        "cloud-agent" | "cloud_agent" | "cloud-mesh" | "cloud_mesh" => Some("cloud-mesh"),
        _ => None,
    }
}

fn refuse_placement_seed(place: &SlimPlacement) -> Result<(), MeshError> {
    if is_sacred_name(&place.placement_id) {
        return Err(MeshError::SacredId(place.placement_id.clone()));
    }
    for agent in &place.agents {
        if is_sacred_name(agent) {
            return Err(MeshError::SacredId(agent.clone()));
        }
    }
    Ok(())
}

pub fn hop_from_placement(place: &SlimPlacement) -> HopDecl {
    let kind = hop_kind_for_placement(&place.kind).unwrap_or("box");
    let cloud = kind == "cloud-mesh";
    HopDecl {
        id: place.placement_id.clone(),
        kind: kind.into(),
        capability: if cloud {
            "mesh-stub".into()
        } else {
            "lane-tool".into()
        },
        host_class: canonical_host_class_opt(Some(place.host_class.as_str()))
            .map(|s| s.to_string())
            .unwrap_or_else(|| place.host_class.clone()),
        wired: place.wired,
        note: Some("derived from placement-actual (slim parse)".into()),
        ttl_secs: place.ttl_secs,
    }
}

/// Derive hops from durable placement leases. Pause-safe; no live spawn.
/// Matching kinds (`box` → box, `cloud-agent` → cloud-mesh) upsert hops.
/// Manually declared hops with other ids stay. Sacred ids refuse.
pub fn sync_from_placements(state_dir: &Path) -> Result<ConveyorMesh, MeshError> {
    let places = slim_parse_placement_actual(&state_dir.join("placement-actual.json"))?;
    let mut mesh = load_interpreted_mesh(state_dir)?;
    for place in &places {
        let Some(_) = hop_kind_for_placement(&place.kind) else {
            continue;
        };
        refuse_placement_seed(place)?;
        let hop = hop_from_placement(place);
        refuse_hop(&hop)?;
        let mut lease = hop_driver(&hop.kind)?.declare(&hop);
        stamp_hop_ttl(&mut lease, &hop, hop_now_unix());
        // Box live-ness follows the placement lease (suspend unspawns).
        if hop.kind == "box" {
            lease.spawned = place.spawned && hop.wired;
            lease.granted = hop.wired;
        }
        mesh.hops.retain(|h| h.id != hop.id);
        mesh.leases.retain(|l| l.hop_id != hop.id);
        mesh.hops.push(hop);
        mesh.leases.push(lease);
    }
    persist_mesh(state_dir, &mesh)?;
    Ok(mesh)
}

pub fn declare_hop(state_dir: &Path, hop: HopDecl) -> Result<HopLease, MeshError> {
    refuse_hop(&hop)?;
    let mut lease = hop_driver(&hop.kind)?.declare(&hop);
    stamp_hop_ttl(&mut lease, &hop, hop_now_unix());
    let mut mesh = load_interpreted_mesh(state_dir)?;
    mesh.hops.retain(|h| h.id != hop.id);
    mesh.leases.retain(|l| l.hop_id != hop.id);
    mesh.hops.push(hop);
    mesh.leases.push(lease.clone());
    persist_mesh(state_dir, &mesh)?;
    Ok(lease)
}

/// Restamp a hop lease from the remaining hop decl. Does not spawn.
fn restamp_hop_from_decl(state_dir: &Path, hop: &HopDecl) -> Result<HopLease, MeshError> {
    refuse_hop(hop)?;
    let mut lease = hop_driver(&hop.kind)?.declare(hop);
    stamp_hop_ttl(&mut lease, hop, hop_now_unix());
    let mut mesh = load_interpreted_mesh(state_dir)?;
    mesh.leases.retain(|l| l.hop_id != hop.id);
    mesh.leases.push(lease.clone());
    persist_mesh(state_dir, &mesh)?;
    Ok(lease)
}

pub fn call_hop(state_dir: &Path, hop_id: &str, capability: &str) -> Result<HopCall, MeshError> {
    let mesh = load_interpreted_mesh(state_dir)?;
    let mut refreshed = false;
    let lease = if let Some(lease) = mesh.leases.iter().find(|l| l.hop_id == hop_id).cloned() {
        lease
    } else if let Some(hop) = mesh.hops.iter().find(|h| h.id == hop_id).cloned() {
        refreshed = true;
        restamp_hop_from_decl(state_dir, &hop)?
    } else {
        return Err(MeshError::NoLease(hop_id.to_string()));
    };
    if lease.kind == "cloud-mesh" {
        return Err(MeshError::CloudNotSpawned(hop_id.to_string()));
    }
    if hop_lease_is_expired(&lease, hop_now_unix()) {
        return Err(MeshError::Expired(hop_id.to_string()));
    }
    if !lease.granted {
        return Err(MeshError::Ungranted(hop_id.to_string()));
    }
    let places = slim_parse_placement_actual(&state_dir.join("placement-actual.json"))?;
    if let Some(place) = places.iter().find(|p| p.placement_id == hop_id) {
        if !place.spawned {
            return Err(MeshError::NotLive(hop_id.to_string()));
        }
    }
    let mut call = hop_driver(&lease.kind)?.call(&lease, capability)?;
    if refreshed {
        call.reason = format!("lease-refresh; {}", call.reason);
    }
    Ok(call)
}

pub fn list_hops(state_dir: &Path) -> Result<Vec<HopDecl>, MeshError> {
    Ok(load_interpreted_mesh(state_dir)?.hops)
}

pub fn list_hop_leases(state_dir: &Path) -> Result<Vec<HopLease>, MeshError> {
    Ok(load_interpreted_mesh(state_dir)?.leases)
}

pub fn hop_now_unix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn stamp_hop_ttl(lease: &mut HopLease, hop: &HopDecl, now: u64) {
    if let Some(ttl) = hop.ttl_secs.filter(|t| *t > 0) {
        lease.ttl_secs = Some(ttl);
        lease.issued_at = Some(now);
        lease.expires_at = Some(now.saturating_add(ttl));
    }
}

pub fn hop_lease_is_expired(lease: &HopLease, now: u64) -> bool {
    lease.expires_at.map(|exp| now >= exp).unwrap_or(false)
}

pub fn list_expired_hop_leases(
    state_dir: &Path,
    now: u64,
) -> Result<Vec<HopLease>, MeshError> {
    Ok(load_interpreted_mesh(state_dir)?
        .leases
        .into_iter()
        .filter(|l| hop_lease_is_expired(l, now))
        .collect())
}

/// Drop expired hop leases. Hop decls stay so call can restamp. Does not spawn.
pub fn forget_expired_hop_leases(state_dir: &Path) -> Result<Vec<String>, MeshError> {
    let mut mesh = load_interpreted_mesh(state_dir)?;
    let now = hop_now_unix();
    let mut forgotten = Vec::new();
    mesh.leases.retain(|l| {
        if hop_lease_is_expired(l, now) {
            forgotten.push(l.hop_id.clone());
            false
        } else {
            true
        }
    });
    if !forgotten.is_empty() {
        persist_mesh(state_dir, &mesh)?;
    }
    Ok(forgotten)
}

/// File SoT committed next to the crate must match the schema snapshot.
pub fn mesh_file_sot() -> ConveyorMesh {
    ConveyorMesh {
        schema: MESH_SCHEMA.into(),
        hops: vec![
            HopDecl {
                id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
            },
            HopDecl {
                id: "cursor-cloud".into(),
                kind: "cloud-mesh".into(),
                capability: "mesh-stub".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
                ttl_secs: None,
            },
        ],
        leases: vec![
            HopLease {
                hop_id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                granted: true,
                spawned: true,
                durable: true,
                driver: "box".into(),
                note: None,
                ttl_secs: None,
                issued_at: None,
                expires_at: None,
            },
            HopLease {
                hop_id: "cursor-cloud".into(),
                kind: "cloud-mesh".into(),
                capability: "mesh-stub".into(),
                host_class: "any".into(),
                granted: false,
                spawned: false,
                durable: true,
                driver: "cloud-mesh".into(),
                note: None,
                ttl_secs: None,
                issued_at: None,
                expires_at: None,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("cell-one-mesh-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn declare_and_call_box_hop() {
        let dir = tmp();
        let lease = declare_hop(
            &dir,
            HopDecl {
                id: "box-notes".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "rtx_consumer".into(),
                wired: true,
                note: None,
                ttl_secs: None,
            },
        )
        .unwrap();
        assert!(lease.granted);
        assert_eq!(lease.host_class, "consumer-nvidia");
        let call = call_hop(&dir, "box-notes", "notes-append").unwrap();
        assert!(call.allow);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn persist_mesh_serializes_before_any_write() {
        let dir = tmp();
        let mesh = ConveyorMesh::default();
        persist_mesh(&dir, &mesh).unwrap();
        for name in [MESH_FILE, HOPS_FILE, LEASES_FILE] {
            let blob = std::fs::read_to_string(dir.join(name)).unwrap();
            assert!(!blob.trim().is_empty(), "{name} must not be empty");
            let _: serde_json::Value = serde_json::from_str(&blob).unwrap();
        }

        struct Boom;
        impl Serialize for Boom {
            fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
                use serde::ser::Error;
                Err(S::Error::custom("boom"))
            }
        }
        let err = serialize_pretty(&Boom).unwrap_err();
        assert!(err.to_string().contains("serialize"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuse_call_without_lease() {
        let dir = tmp();
        let err = call_hop(&dir, "missing", "lane-tool").unwrap_err();
        assert!(matches!(err, MeshError::NoLease(_)));
        assert!(err.to_string().starts_with("refuse:no-lease"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuse_call_without_granted_lease() {
        let dir = tmp();
        declare_hop(
            &dir,
            HopDecl {
                id: "cold-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
                ttl_secs: None,
            },
        )
        .unwrap();
        let err = call_hop(&dir, "cold-box", "lane-tool").unwrap_err();
        assert!(matches!(err, MeshError::Ungranted(_)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cloud_mesh_declare_ok_call_refuses() {
        let dir = tmp();
        let lease = declare_hop(
            &dir,
            HopDecl {
                id: "cursor-cloud".into(),
                kind: "cloud-mesh".into(),
                capability: "mesh-stub".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
            },
        )
        .unwrap();
        assert!(!lease.granted);
        assert!(!lease.spawned);
        let err = call_hop(&dir, "cursor-cloud", "mesh-stub").unwrap_err();
        assert!(matches!(err, MeshError::CloudNotSpawned(_)));
        assert!(err.to_string().starts_with("refuse:cloud-not-spawned"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sku_hop_id_refuses() {
        let dir = tmp();
        let err = declare_hop(
            &dir,
            HopDecl {
                id: "gpu-5090-hop".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, MeshError::SkuBanned(_)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn slim_parse_and_sync_from_placement() {
        let dir = tmp();
        let actual = serde_json::json!({
            "schema": "cell-one.placement-actual.v0",
            "desired_hash": "sha256:test",
            "leases": [
                {
                    "placement_id": "cell-one-box",
                    "kind": "box",
                    "host_class": "rtx_consumer",
                    "spawned": true,
                    "wired": true
                },
                {
                    "placement_id": "cursor-cloud",
                    "kind": "cloud-agent",
                    "host_class": "any",
                    "spawned": false,
                    "wired": false
                }
            ]
        });
        std::fs::write(dir.join("placement-actual.json"), actual.to_string()).unwrap();
        let mesh = sync_from_placements(&dir).unwrap();
        assert_eq!(mesh.schema, MESH_SCHEMA);
        assert_eq!(mesh.hops.len(), 2);
        let box_lease = mesh
            .leases
            .iter()
            .find(|l| l.hop_id == "cell-one-box")
            .unwrap();
        assert!(box_lease.granted);
        assert_eq!(box_lease.host_class, "consumer-nvidia");
        let cloud = mesh
            .leases
            .iter()
            .find(|l| l.hop_id == "cursor-cloud")
            .unwrap();
        assert!(!cloud.granted);
        assert!(dir.join(MESH_FILE).is_file());
        assert!(dir.join(HOPS_FILE).is_file());
        assert!(dir.join(LEASES_FILE).is_file());
        let call = call_hop(&dir, "cell-one-box", "lane-tool").unwrap();
        assert!(call.allow);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tampered_sku_host_class_does_not_become_any() {
        let dir = tmp();
        let actual = serde_json::json!({
            "schema": "cell-one.placement-actual.v0",
            "desired_hash": "sha256:test",
            "leases": [
                {
                    "placement_id": "cell-one-box",
                    "kind": "box",
                    "host_class": "rtx-5090",
                    "spawned": true,
                    "wired": true
                }
            ]
        });
        std::fs::write(dir.join("placement-actual.json"), actual.to_string()).unwrap();
        let err = sync_from_placements(&dir).unwrap_err();
        assert!(
            matches!(err, MeshError::BadHostClass(ref h) if h == "rtx-5090"),
            "{err}"
        );
        assert!(
            err.to_string().contains("refuse:bad-host-class"),
            "{err}"
        );
        assert!(
            !dir.join(MESH_FILE).is_file(),
            "sync must not write hops after a host_class refuse"
        );
        let garbage = serde_json::json!({
            "schema": "cell-one.placement-actual.v0",
            "desired_hash": "sha256:test",
            "leases": [
                {
                    "placement_id": "cell-one-box",
                    "kind": "box",
                    "host_class": "not-a-host",
                    "spawned": true,
                    "wired": true
                }
            ]
        });
        std::fs::write(dir.join("placement-actual.json"), garbage.to_string()).unwrap();
        let err = slim_parse_placement_actual(&dir.join("placement-actual.json")).unwrap_err();
        assert!(matches!(err, MeshError::BadHostClass(ref h) if h == "not-a-host"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn call_does_not_skip_liveness_when_host_class_is_sku() {
        let dir = tmp();
        declare_hop(
            &dir,
            HopDecl {
                id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
            },
        )
        .unwrap();
        let actual = serde_json::json!({
            "schema": "cell-one.placement-actual.v0",
            "desired_hash": "sha256:test",
            "leases": [
                {
                    "placement_id": "cell-one-box",
                    "kind": "box",
                    "host_class": "rtx-5090",
                    "spawned": false,
                    "wired": true
                }
            ]
        });
        std::fs::write(dir.join("placement-actual.json"), actual.to_string()).unwrap();
        let before = std::fs::read_to_string(dir.join("placement-actual.json")).unwrap();
        let err = call_hop(&dir, "cell-one-box", "lane-tool").unwrap_err();
        assert!(
            matches!(err, MeshError::BadHostClass(ref h) if h == "rtx-5090"),
            "{err}"
        );
        assert!(
            err.to_string().contains("refuse:bad-host-class"),
            "{err}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("placement-actual.json")).unwrap(),
            before,
            "call must not rewrite a SKU host_class to any"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tampered_mesh_sku_host_class_refuses_readers_and_writes_nothing() {
        let dir = tmp();
        declare_hop(
            &dir,
            HopDecl {
                id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
            },
        )
        .unwrap();
        let mut mesh = load_mesh(&dir).unwrap();
        mesh.hops[0].host_class = "rtx-5090".into();
        mesh.leases[0].host_class = "rtx-5090".into();
        persist_mesh(&dir, &mesh).unwrap();
        let before = std::fs::read_to_string(dir.join(MESH_FILE)).unwrap();
        let hops_before = std::fs::read_to_string(dir.join(HOPS_FILE)).unwrap();
        let leases_before = std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap();

        let call = call_hop(&dir, "cell-one-box", "lane-tool").unwrap_err();
        assert!(
            matches!(call, MeshError::BadHostClass(ref h) if h == "rtx-5090"),
            "{call}"
        );
        assert!(call.to_string().contains("refuse:bad-host-class"), "{call}");

        let listed = list_hops(&dir).unwrap_err();
        assert!(matches!(listed, MeshError::BadHostClass(ref h) if h == "rtx-5090"));
        let hop_leases = list_hop_leases(&dir).unwrap_err();
        assert!(matches!(hop_leases, MeshError::BadHostClass(ref h) if h == "rtx-5090"));
        let expired = list_expired_hop_leases(&dir, hop_now_unix()).unwrap_err();
        assert!(matches!(expired, MeshError::BadHostClass(_)));
        let forgotten = forget_expired_hop_leases(&dir).unwrap_err();
        assert!(matches!(forgotten, MeshError::BadHostClass(_)));

        std::fs::write(
            dir.join("placement-actual.json"),
            serde_json::json!({
                "schema": "cell-one.placement-actual.v0",
                "leases": [{
                    "placement_id": "cell-one-box",
                    "kind": "box",
                    "host_class": "any",
                    "spawned": true,
                    "wired": true
                }]
            })
            .to_string(),
        )
        .unwrap();
        let sync = sync_from_placements(&dir).unwrap_err();
        assert!(
            matches!(sync, MeshError::BadHostClass(ref h) if h == "rtx-5090"),
            "{sync}"
        );

        let again = declare_hop(
            &dir,
            HopDecl {
                id: "other-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
            },
        )
        .unwrap_err();
        assert!(matches!(again, MeshError::BadHostClass(_)));

        assert_eq!(std::fs::read_to_string(dir.join(MESH_FILE)).unwrap(), before);
        assert_eq!(
            std::fs::read_to_string(dir.join(HOPS_FILE)).unwrap(),
            hops_before
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap(),
            leases_before
        );
        assert!(before.contains("rtx-5090"));
        assert!(!before.contains("\"host_class\": \"any\""));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tampered_mesh_unknown_host_class_refuses_like_sku() {
        let dir = tmp();
        declare_hop(
            &dir,
            HopDecl {
                id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
            },
        )
        .unwrap();
        let mut mesh = load_mesh(&dir).unwrap();
        mesh.leases[0].host_class = "not-a-host".into();
        persist_mesh(&dir, &mesh).unwrap();
        let before = std::fs::read_to_string(dir.join(MESH_FILE)).unwrap();
        let err = call_hop(&dir, "cell-one-box", "lane-tool").unwrap_err();
        assert!(matches!(err, MeshError::BadHostClass(ref h) if h == "not-a-host"));
        assert_eq!(std::fs::read_to_string(dir.join(MESH_FILE)).unwrap(), before);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn committed_mesh_sot_matches_schema_file() {
        let committed = include_str!("../../../schema/conveyor-mesh.v0.json");
        let file: ConveyorMesh = serde_json::from_str(committed).unwrap();
        let snap = mesh_file_sot();
        assert_eq!(file.schema, snap.schema);
        assert_eq!(file.hops.len(), snap.hops.len());
        assert_eq!(file.leases.len(), snap.leases.len());
        assert!(!file.leases.iter().any(|l| l.kind == "cloud-mesh" && l.spawned));
    }

    #[test]
    fn expired_hop_lease_refuses_call() {
        let dir = tmp();
        let lease = declare_hop(
            &dir,
            HopDecl {
                id: "short-hop".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: Some(1),
            },
        )
        .unwrap();
        assert_eq!(lease.ttl_secs, Some(1));
        assert!(lease.expires_at.is_some());
        let mut mesh = load_mesh(&dir).unwrap();
        let now = hop_now_unix();
        mesh.leases[0].issued_at = Some(now.saturating_sub(10));
        mesh.leases[0].expires_at = Some(now.saturating_sub(1));
        persist_mesh(&dir, &mesh).unwrap();
        let err = call_hop(&dir, "short-hop", "lane-tool").unwrap_err();
        assert!(matches!(err, MeshError::Expired(_)));
        assert!(err.to_string().starts_with("refuse:expired"));
        let listed = list_expired_hop_leases(&dir, now).unwrap();
        assert_eq!(listed.len(), 1);
        let forgotten = forget_expired_hop_leases(&dir).unwrap();
        assert_eq!(forgotten, vec!["short-hop".to_string()]);
        let after = load_mesh(&dir).unwrap();
        assert!(
            after.hops.iter().any(|h| h.id == "short-hop"),
            "forget must keep hop decls"
        );
        assert!(after.leases.is_empty());
        let fresh = call_hop(&dir, "short-hop", "lane-tool").unwrap();
        assert!(fresh.allow);
        assert!(
            fresh.reason.contains("lease-refresh"),
            "{}",
            fresh.reason
        );
        let restamped = load_mesh(&dir).unwrap();
        let lease = restamped
            .leases
            .iter()
            .find(|l| l.hop_id == "short-hop")
            .unwrap();
        assert_eq!(lease.ttl_secs, Some(1));
        assert!(lease.expires_at.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sync_keeps_manual_hops_and_skips_unknown_kinds() {
        let dir = tmp();
        declare_hop(
            &dir,
            HopDecl {
                id: "ttl-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: Some(3600),
            },
        )
        .unwrap();
        let actual = serde_json::json!({
            "schema": "cell-one.placement-actual.v0",
            "leases": [
                {
                    "placement_id": "cell-one-box",
                    "kind": "box",
                    "host_class": "any",
                    "spawned": true,
                    "wired": true,
                    "agents": ["horizon"]
                },
                {
                    "placement_id": "cursor-cloud",
                    "kind": "cloud-agent",
                    "host_class": "any",
                    "spawned": false,
                    "wired": false
                },
                {
                    "placement_id": "odd-kind",
                    "kind": "studio",
                    "host_class": "any",
                    "spawned": false,
                    "wired": false
                }
            ]
        });
        std::fs::write(dir.join("placement-actual.json"), actual.to_string()).unwrap();
        let mesh = sync_from_placements(&dir).unwrap();
        assert!(mesh.hops.iter().any(|h| h.id == "ttl-box"));
        assert!(mesh.hops.iter().any(|h| h.id == "cell-one-box"));
        assert!(mesh.hops.iter().any(|h| h.id == "cursor-cloud"));
        assert!(!mesh.hops.iter().any(|h| h.id == "odd-kind"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sync_refuses_sacred_placement() {
        let dir = tmp();
        let sacred = estate_schema::locked_sacred_ids()[0];
        let actual = serde_json::json!({
            "schema": "cell-one.placement-actual.v0",
            "leases": [{
                "placement_id": sacred,
                "kind": "box",
                "host_class": "any",
                "spawned": false,
                "wired": true,
                "agents": []
            }]
        });
        std::fs::write(dir.join("placement-actual.json"), actual.to_string()).unwrap();
        let err = sync_from_placements(&dir).unwrap_err();
        assert!(matches!(err, MeshError::SacredId(_)));
        assert!(err.to_string().starts_with("refuse:sacred-id"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuse_codes_kind_id_host_capability_not_live() {
        let kind = refuse_hop(&HopDecl {
            id: "studio-hop".into(),
            kind: "studio".into(),
            capability: "lane-tool".into(),
            host_class: "any".into(),
            wired: true,
            note: None,
            ttl_secs: None,
        })
        .unwrap_err();
        assert!(matches!(kind, MeshError::Kind(_)));
        assert!(kind.to_string().starts_with("refuse:kind"));

        let bad_id = refuse_hop(&HopDecl {
            id: "Not A Slug".into(),
            kind: "box".into(),
            capability: "lane-tool".into(),
            host_class: "any".into(),
            wired: true,
            note: None,
            ttl_secs: None,
        })
        .unwrap_err();
        assert!(matches!(bad_id, MeshError::BadId(_)));
        assert!(bad_id.to_string().starts_with("refuse:bad-id"));

        let host = refuse_hop(&HopDecl {
            id: "ok-hop".into(),
            kind: "box".into(),
            capability: "lane-tool".into(),
            host_class: "not-a-host".into(),
            wired: true,
            note: None,
            ttl_secs: None,
        })
        .unwrap_err();
        assert!(matches!(host, MeshError::BadHostClass(_)));
        assert!(host.to_string().starts_with("refuse:bad-host-class"));

        let sku = refuse_hop(&HopDecl {
            id: "gpu-5090-hop".into(),
            kind: "box".into(),
            capability: "lane-tool".into(),
            host_class: "any".into(),
            wired: true,
            note: None,
            ttl_secs: None,
        })
        .unwrap_err();
        assert!(sku.to_string().starts_with("refuse:sku-banned"));

        let dir = tmp();
        declare_hop(
            &dir,
            HopDecl {
                id: "notes-hop".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
            },
        )
        .unwrap();
        let cap = call_hop(&dir, "notes-hop", "other-cap").unwrap_err();
        assert!(matches!(cap, MeshError::Capability { .. }));
        assert!(cap.to_string().starts_with("refuse:capability"));

        let ungranted = call_hop(&dir, "missing-grant", "lane-tool").unwrap_err();
        assert!(ungranted.to_string().starts_with("refuse:no-lease"));

        declare_hop(
            &dir,
            HopDecl {
                id: "cold-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
                ttl_secs: None,
            },
        )
        .unwrap();
        let denied = call_hop(&dir, "cold-box", "lane-tool").unwrap_err();
        assert!(denied.to_string().starts_with("refuse:ungranted"));

        std::fs::write(
            dir.join("placement-actual.json"),
            serde_json::json!({
                "schema": "cell-one.placement-actual.v0",
                "leases": [{
                    "placement_id": "notes-hop",
                    "kind": "box",
                    "host_class": "any",
                    "spawned": false,
                    "wired": true
                }]
            })
            .to_string(),
        )
        .unwrap();
        let dead = call_hop(&dir, "notes-hop", "notes-append").unwrap_err();
        assert!(matches!(dead, MeshError::NotLive(_)));
        assert!(dead.to_string().starts_with("refuse:not-live"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
