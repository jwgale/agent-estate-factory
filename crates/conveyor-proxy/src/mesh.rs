//! Capability mesh beachhead. Same trait style as placement/local drivers.
//!
//! Declare a hop, lease-bound call, refuse without a granted lease.
//! `cloud-mesh` hops are declared and never spawned. Not a gateway.
//!
//! Slim-parses `.cell/placement-actual.json` so this crate does not depend
//! on floor-supervisor.

use estate_schema::{canonical_host_class, contains_sku, is_slug, normalize_host_class};
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
    #[error("no hop lease for '{0}'")]
    NoLease(String),
    #[error("hop '{0}' refused: no granted lease")]
    Ungranted(String),
    #[error("cloud-mesh hop '{0}' is declared, not spawned")]
    CloudNotSpawned(String),
    #[error("hop '{0}' is not live (suspended or unwired)")]
    NotLive(String),
    #[error("capability '{want}' is not on hop '{hop}' (have {have})")]
    Capability {
        hop: String,
        want: String,
        have: String,
    },
    #[error("unknown hop kind '{0}' (use box|cloud-mesh)")]
    Kind(String),
    #[error("hop id '{0}' encodes a hardware SKU")]
    SkuBanned(String),
    #[error("hop id '{0}' must match [a-z][a-z0-9_-]{{0,63}}")]
    BadId(String),
    #[error("hop host_class '{0}' must be consumer-nvidia|apple-silicon|rented-nvidia|any")]
    BadHostClass(String),
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
}

fn default_host_class() -> String {
    "any".into()
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
    pub spawned: bool,
    #[serde(default)]
    pub wired: bool,
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
            host_class: canonical_host_class(Some(hop.host_class.as_str())).into(),
            granted: hop.wired,
            spawned: hop.wired,
            durable: true,
            driver: self.name().into(),
            note: Some("box hop lease. Call refuses without granted+".into()),
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
            host_class: canonical_host_class(Some(hop.host_class.as_str())).into(),
            granted: false,
            spawned: false,
            durable: true,
            driver: self.name().into(),
            note: Some("declared mesh stub. Conveyor records the hop and does not spawn it.".into()),
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

pub fn persist_mesh(state_dir: &Path, mesh: &ConveyorMesh) -> Result<PathBuf, MeshError> {
    std::fs::create_dir_all(state_dir)?;
    let path = mesh_path(state_dir);
    std::fs::write(&path, serde_json::to_string_pretty(mesh).unwrap_or_default())?;
    let hops = serde_json::json!({
        "schema": MESH_SCHEMA,
        "hops": mesh.hops,
    });
    std::fs::write(
        state_dir.join(HOPS_FILE),
        serde_json::to_string_pretty(&hops).unwrap_or_default(),
    )?;
    let leases = serde_json::json!({
        "schema": MESH_SCHEMA,
        "leases": mesh.leases,
    });
    std::fs::write(
        state_dir.join(LEASES_FILE),
        serde_json::to_string_pretty(&leases).unwrap_or_default(),
    )?;
    Ok(path)
}

pub fn slim_parse_placement_actual(path: &Path) -> Result<Vec<SlimPlacement>, MeshError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path)?;
    let actual: SlimActual = serde_json::from_str(&text)
        .map_err(|e| MeshError::Parse(format!("placement-actual.json: {e}")))?;
    Ok(actual
        .leases
        .into_iter()
        .map(|row| SlimPlacement {
            placement_id: row.placement_id,
            kind: row.kind,
            host_class: canonical_host_class(Some(row.host_class.as_str())).into(),
            spawned: row.spawned,
            wired: row.wired,
        })
        .collect())
}

pub fn hop_from_placement(place: &SlimPlacement) -> HopDecl {
    let cloud = place.kind == "cloud-agent" || place.kind == "cloud-mesh";
    HopDecl {
        id: place.placement_id.clone(),
        kind: if cloud { "cloud-mesh".into() } else { "box".into() },
        capability: if cloud {
            "mesh-stub".into()
        } else {
            "lane-tool".into()
        },
        host_class: canonical_host_class(Some(place.host_class.as_str())).into(),
        wired: place.wired,
        note: Some("derived from placement-actual (slim parse)".into()),
    }
}

/// Derive hops from durable placement leases. Pause-safe; no live spawn.
pub fn sync_from_placements(state_dir: &Path) -> Result<ConveyorMesh, MeshError> {
    let places = slim_parse_placement_actual(&state_dir.join("placement-actual.json"))?;
    let mut mesh = ConveyorMesh::default();
    for place in &places {
        let hop = hop_from_placement(place);
        refuse_hop(&hop)?;
        let mut lease = hop_driver(&hop.kind)?.declare(&hop);
        // Box live-ness follows the placement lease (suspend unspawns).
        if hop.kind == "box" {
            lease.spawned = place.spawned && hop.wired;
            lease.granted = hop.wired;
        }
        mesh.hops.push(hop);
        mesh.leases.push(lease);
    }
    persist_mesh(state_dir, &mesh)?;
    Ok(mesh)
}

pub fn declare_hop(state_dir: &Path, hop: HopDecl) -> Result<HopLease, MeshError> {
    refuse_hop(&hop)?;
    let lease = hop_driver(&hop.kind)?.declare(&hop);
    let mut mesh = load_mesh(state_dir)?;
    mesh.hops.retain(|h| h.id != hop.id);
    mesh.leases.retain(|l| l.hop_id != hop.id);
    mesh.hops.push(hop);
    mesh.leases.push(lease.clone());
    persist_mesh(state_dir, &mesh)?;
    Ok(lease)
}

pub fn call_hop(state_dir: &Path, hop_id: &str, capability: &str) -> Result<HopCall, MeshError> {
    let mesh = load_mesh(state_dir)?;
    let lease = mesh
        .leases
        .iter()
        .find(|l| l.hop_id == hop_id)
        .cloned()
        .ok_or_else(|| MeshError::NoLease(hop_id.to_string()))?;
    if lease.kind == "cloud-mesh" {
        return Err(MeshError::CloudNotSpawned(hop_id.to_string()));
    }
    if !lease.granted {
        return Err(MeshError::Ungranted(hop_id.to_string()));
    }
    if let Ok(places) = slim_parse_placement_actual(&state_dir.join("placement-actual.json")) {
        if let Some(place) = places.iter().find(|p| p.placement_id == hop_id) {
            if !place.spawned {
                return Err(MeshError::NotLive(hop_id.to_string()));
            }
        }
    }
    hop_driver(&lease.kind)?.call(&lease, capability)
}

pub fn list_hops(state_dir: &Path) -> Result<Vec<HopDecl>, MeshError> {
    Ok(load_mesh(state_dir)?.hops)
}

pub fn list_hop_leases(state_dir: &Path) -> Result<Vec<HopLease>, MeshError> {
    Ok(load_mesh(state_dir)?.leases)
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
            },
            HopDecl {
                id: "cursor-cloud".into(),
                kind: "cloud-mesh".into(),
                capability: "mesh-stub".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
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
    fn refuse_call_without_lease() {
        let dir = tmp();
        let err = call_hop(&dir, "missing", "lane-tool").unwrap_err();
        assert!(matches!(err, MeshError::NoLease(_)));
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
            },
        )
        .unwrap();
        assert!(!lease.granted);
        assert!(!lease.spawned);
        let err = call_hop(&dir, "cursor-cloud", "mesh-stub").unwrap_err();
        assert!(matches!(err, MeshError::CloudNotSpawned(_)));
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
    fn committed_mesh_sot_matches_schema_file() {
        let committed = include_str!("../../../schema/conveyor-mesh.v0.json");
        let file: ConveyorMesh = serde_json::from_str(committed).unwrap();
        let snap = mesh_file_sot();
        assert_eq!(file.schema, snap.schema);
        assert_eq!(file.hops.len(), snap.hops.len());
        assert_eq!(file.leases.len(), snap.leases.len());
        assert!(!file.leases.iter().any(|l| l.kind == "cloud-mesh" && l.spawned));
    }
}
