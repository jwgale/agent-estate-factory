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
    #[error("refuse:cloud-spawned: cloud-agent lease spawned (fail closed): {0}")]
    CloudSpawned(String),
    /// The lease names a population, or the caller named an agent the lease does not.
    #[error("refuse:agent-unbound: hop '{hop}' does not bind agent '{agent}'")]
    AgentUnbound { hop: String, agent: String },
    /// Lease agents are wider than the placement row. Mesh is ahead of the floor.
    #[error("refuse:agent-unplaced: hop '{hop}' lists agents the placement does not ({agents})")]
    AgentUnplaced { hop: String, agents: String },
    /// Named agent call. Estate authorize denied the capability. Not an identity check.
    #[error("refuse:intention: agent '{agent}' capability '{capability}': {reason}")]
    Intention {
        agent: String,
        capability: String,
        reason: String,
    },
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
    /// Agents this hop binds. Empty means the stub is not a population grant.
    #[serde(default)]
    pub agents: Vec<String>,
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
    refuse_mesh_population(state_dir, &mesh)?;
    Ok(mesh)
}

/// A hop or lease that names an agent the placement row does not is ahead
/// of the floor. A missing placement file is not that claim. Writes nothing.
fn refuse_mesh_population(state_dir: &Path, mesh: &ConveyorMesh) -> Result<(), MeshError> {
    let places = slim_parse_placement_actual(&state_dir.join("placement-actual.json"))?;
    for hop in &mesh.hops {
        refuse_lease_ahead(&hop.id, &hop.agents, &places)?;
    }
    for lease in &mesh.leases {
        refuse_lease_ahead(&lease.hop_id, &lease.agents, &places)?;
    }
    Ok(())
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
    /// Copied from the hop decl. Empty is not a grant to every agent.
    #[serde(default)]
    pub agents: Vec<String>,
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
            agents: hop.agents.clone(),
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
            note: Some(
                "declared mesh stub. Conveyor records the hop and does not spawn it.".into(),
            ),
            ttl_secs: None,
            issued_at: None,
            expires_at: None,
            agents: hop.agents.clone(),
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

fn hop_is_cloud(kind: &str) -> bool {
    matches!(
        kind.trim().to_ascii_lowercase().as_str(),
        "cloud-mesh" | "cloud_mesh" | "cloud-agent"
    )
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
    refuse_hop_agents(&hop.agents)?;
    hop_driver(&hop.kind)?;
    Ok(())
}

fn agent_on(agents: &[String], agent: &str) -> bool {
    let want = estate_schema::normalize_name(agent);
    agents
        .iter()
        .any(|have| estate_schema::normalize_name(have) == want)
}

/// Agents on the lease that the placement row does not list.
/// No placement row for this hop is not a wider claim.
fn agents_ahead_of_placement<'a>(
    lease_agents: &'a [String],
    place: Option<&SlimPlacement>,
) -> Vec<&'a str> {
    let Some(place) = place else {
        return Vec::new();
    };
    lease_agents
        .iter()
        .filter(|agent| !agent_on(&place.agents, agent))
        .map(|agent| agent.as_str())
        .collect()
}

fn refuse_lease_ahead(
    hop_id: &str,
    lease_agents: &[String],
    places: &[SlimPlacement],
) -> Result<(), MeshError> {
    let place = places.iter().find(|p| p.placement_id == hop_id);
    let ahead = agents_ahead_of_placement(lease_agents, place);
    if ahead.is_empty() {
        return Ok(());
    }
    Err(MeshError::AgentUnplaced {
        hop: hop_id.to_string(),
        agents: ahead.join(", "),
    })
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

fn refuse_hop_agents(agents: &[String]) -> Result<(), MeshError> {
    let mut seen = Vec::new();
    for agent in agents {
        if is_sacred_name(agent) {
            return Err(MeshError::SacredId(agent.clone()));
        }
        if !is_slug(agent) {
            return Err(MeshError::BadId(agent.clone()));
        }
        if contains_sku(agent) {
            return Err(MeshError::SkuBanned(agent.clone()));
        }
        if seen.iter().any(|s: &String| s == agent) {
            return Err(MeshError::BadId(agent.clone()));
        }
        seen.push(agent.clone());
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
        agents: place.agents.clone(),
    }
}

/// Derive hops from durable placement leases. Pause-safe; no live spawn.
/// Matching kinds (`box` → box, `cloud-agent` → cloud-mesh) upsert hops.
/// Manually declared hops with other ids stay. Sacred ids refuse.
///
/// A spawned cloud-agent placement is still spawned. `CloudMeshHop::declare`
/// hardcodes `spawned: false`, and the box formula
/// `place.spawned && hop.wired` is not this path. Refuse before a hop lease
/// records that lie. A missing placement file is an empty list, not a spawned
/// lease. The placement file is not rewritten.
pub fn sync_from_placements(state_dir: &Path) -> Result<ConveyorMesh, MeshError> {
    let places = slim_parse_placement_actual(&state_dir.join("placement-actual.json"))?;
    let spawned: Vec<String> = places
        .iter()
        .filter(|place| hop_kind_for_placement(&place.kind) == Some("cloud-mesh") && place.spawned)
        .map(|place| place.placement_id.clone())
        .collect();
    if !spawned.is_empty() {
        return Err(MeshError::CloudSpawned(spawned.join(", ")));
    }
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
    let places = slim_parse_placement_actual(&state_dir.join("placement-actual.json"))?;
    refuse_lease_ahead(&hop.id, &hop.agents, &places)?;
    // CloudMeshHop::declare hardcodes spawned:false. A spawned placement
    // lease for this hop is still spawned. Refuse before the write.
    // A missing placement file is not a spawned lease.
    if hop_is_cloud(&hop.kind) && placement_cloud_spawned(state_dir, &hop.id)? {
        return Err(MeshError::CloudSpawned(hop.id));
    }
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

fn placement_cloud_spawned(state_dir: &Path, hop_id: &str) -> Result<bool, MeshError> {
    let places = slim_parse_placement_actual(&state_dir.join("placement-actual.json"))?;
    Ok(places.iter().any(|place| {
        place.placement_id == hop_id
            && hop_kind_for_placement(&place.kind) == Some("cloud-mesh")
            && place.spawned
    }))
}

pub fn call_hop(state_dir: &Path, hop_id: &str, capability: &str) -> Result<HopCall, MeshError> {
    call_hop_inner(state_dir, hop_id, capability, None)
}

/// Lease-bound call for one agent on the hop population.
/// The estate intention check runs only after the lease would allow.
/// This is not an identity lookup. A missing population is `refuse:agent-unbound`.
pub fn call_hop_for_agent(
    state_dir: &Path,
    hop_id: &str,
    capability: &str,
    agent_id: &str,
    kind: Option<estate_schema::IntentionKind>,
    estate: &estate_schema::Estate,
) -> Result<HopCall, MeshError> {
    if estate_schema::is_sacred_name(agent_id) || estate.is_sacred(agent_id) {
        return Err(MeshError::SacredId(agent_id.to_string()));
    }
    let mut call = call_hop_inner(state_dir, hop_id, capability, Some(agent_id))?;
    let resolved = resolve_intention_kind(estate, agent_id, capability, kind)?;
    let object =
        if resolved == estate_schema::IntentionKind::MemoryRead && !capability.contains(':') {
            format!("lane:{capability}")
        } else {
            capability.to_string()
        };
    let decision = estate_schema::authorize(
        estate,
        &estate_schema::AccessRequest {
            subject_agent: agent_id,
            kind: resolved,
            object: &object,
        },
    );
    if !decision.is_allow() {
        return Err(MeshError::Intention {
            agent: agent_id.to_string(),
            capability: capability.to_string(),
            reason: decision.reason().to_string(),
        });
    }
    call.reason = format!("agent-bound {agent_id}; {}", decision.reason());
    Ok(call)
}

fn resolve_intention_kind(
    estate: &estate_schema::Estate,
    agent_id: &str,
    capability: &str,
    kind: Option<estate_schema::IntentionKind>,
) -> Result<estate_schema::IntentionKind, MeshError> {
    if let Some(kind) = kind {
        return Ok(kind);
    }
    if capability.starts_with("lane:") {
        return Ok(estate_schema::IntentionKind::MemoryRead);
    }
    let Some(agent) = estate.agent(agent_id) else {
        return Err(MeshError::Intention {
            agent: agent_id.to_string(),
            capability: capability.to_string(),
            reason: format!("unknown subject agent '{agent_id}'"),
        });
    };
    let mut hits = Vec::new();
    if agent.has_tool(capability) {
        hits.push(estate_schema::IntentionKind::Tool);
    }
    if agent.has_mcp(capability) {
        hits.push(estate_schema::IntentionKind::Mcp);
    }
    if agent.has_mount(capability) {
        hits.push(estate_schema::IntentionKind::Mount);
    }
    if agent.has_model(capability) {
        hits.push(estate_schema::IntentionKind::Model);
    }
    match hits.as_slice() {
        [one] => Ok(*one),
        [] => Err(MeshError::Intention {
            agent: agent_id.to_string(),
            capability: capability.to_string(),
            reason: format!("undeclared for agent '{agent_id}' (deny-default)"),
        }),
        _ => Err(MeshError::Intention {
            agent: agent_id.to_string(),
            capability: capability.to_string(),
            reason: "ambiguous capability; pass kind".into(),
        }),
    }
}

fn call_hop_inner(
    state_dir: &Path,
    hop_id: &str,
    capability: &str,
    agent_id: Option<&str>,
) -> Result<HopCall, MeshError> {
    let mesh = load_interpreted_mesh(state_dir)?;
    let existing = mesh.leases.iter().find(|l| l.hop_id == hop_id).cloned();
    let decl = mesh.hops.iter().find(|h| h.id == hop_id).cloned();
    let kind = existing
        .as_ref()
        .map(|lease| lease.kind.as_str())
        .or_else(|| decl.as_ref().map(|hop| hop.kind.as_str()));
    // A spawned cloud placement is still spawned. Do not restamp a missing
    // hop lease to spawned:false, and do not say the hop is not spawned.
    // A missing placement file is not a spawned lease.
    if kind == Some("cloud-mesh") && placement_cloud_spawned(state_dir, hop_id)? {
        return Err(MeshError::CloudSpawned(hop_id.to_string()));
    }
    let mut refreshed = false;
    let lease = if let Some(lease) = existing {
        lease
    } else if let Some(hop) = decl {
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
    // A populated lease is a population grant. An unnamed call skips it.
    // An agent the lease does not name is not invented onto the grant.
    match agent_id {
        Some(agent) if !agent_on(&lease.agents, agent) => {
            return Err(MeshError::AgentUnbound {
                hop: hop_id.to_string(),
                agent: agent.to_string(),
            });
        }
        None if !lease.agents.is_empty() => {
            return Err(MeshError::AgentUnbound {
                hop: hop_id.to_string(),
                agent: "(unnamed)".into(),
            });
        }
        _ => {}
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

/// A spawned cloud hop lease is still spawned. Refuse before hop lease
/// JSON that looks like a normal list. A missing mesh is an empty list,
/// not a spawned lease. An unspawned cloud hop still lists.
pub fn list_hop_leases(state_dir: &Path) -> Result<Vec<HopLease>, MeshError> {
    let mesh = load_interpreted_mesh(state_dir)?;
    refuse_spawned_cloud_hop(&mesh.leases)?;
    Ok(mesh.leases)
}

/// A present spawned cloud hop lease is a refuse before a list that
/// would print it. A missing mesh is not a spawned lease.
fn refuse_spawned_cloud_hop(leases: &[HopLease]) -> Result<(), MeshError> {
    let spawned: Vec<String> = leases
        .iter()
        .filter(|lease| hop_is_cloud(&lease.kind) && lease.spawned)
        .map(|lease| lease.hop_id.clone())
        .collect();
    if spawned.is_empty() {
        return Ok(());
    }
    Err(MeshError::CloudSpawned(spawned.join(", ")))
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

/// An expired spawned cloud hop lease is still spawned. Refuse before a
/// list that looks droppable, and before forget rewrites the mesh.
/// A missing mesh is an empty lease list, not a spawned lease. An expired
/// box hop still drops when no spawned cloud row is in that drop.
fn refuse_expired_spawned_cloud_hop(leases: &[HopLease], now: u64) -> Result<(), MeshError> {
    let spawned: Vec<String> = leases
        .iter()
        .filter(|lease| {
            hop_lease_is_expired(lease, now) && hop_is_cloud(&lease.kind) && lease.spawned
        })
        .map(|lease| lease.hop_id.clone())
        .collect();
    if spawned.is_empty() {
        return Ok(());
    }
    Err(MeshError::CloudSpawned(spawned.join(", ")))
}

pub fn list_expired_hop_leases(state_dir: &Path, now: u64) -> Result<Vec<HopLease>, MeshError> {
    let mesh = load_interpreted_mesh(state_dir)?;
    refuse_expired_spawned_cloud_hop(&mesh.leases, now)?;
    Ok(mesh
        .leases
        .into_iter()
        .filter(|l| hop_lease_is_expired(l, now))
        .collect())
}

/// Drop expired hop leases. Hop decls stay so call can restamp. Does not spawn.
///
/// An expired spawned cloud hop lease is still spawned. Refuse before the
/// rewrite. A missing mesh is not a spawned lease. An expired box hop still
/// drops when no spawned cloud row is in that drop.
pub fn forget_expired_hop_leases(state_dir: &Path) -> Result<Vec<String>, MeshError> {
    let mut mesh = load_interpreted_mesh(state_dir)?;
    let now = hop_now_unix();
    refuse_expired_spawned_cloud_hop(&mesh.leases, now)?;
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

fn intention_allows(estate: &estate_schema::Estate, agent_id: &str, capability: &str) -> bool {
    if estate_schema::is_sacred_name(agent_id) || estate.is_sacred(agent_id) {
        return false;
    }
    let Ok(kind) = resolve_intention_kind(estate, agent_id, capability, None) else {
        return false;
    };
    let object = if kind == estate_schema::IntentionKind::MemoryRead && !capability.contains(':') {
        format!("lane:{capability}")
    } else {
        capability.to_string()
    };
    estate_schema::authorize(
        estate,
        &estate_schema::AccessRequest {
            subject_agent: agent_id,
            kind,
            object: &object,
        },
    )
    .is_allow()
}

/// File check only. `would-allow` and `would-deny` name what a later convey
/// path would do. They are not a mediated call.
/// `not-enforced` means this row is not a mediated workload.
/// This report has no status that means the conveyor mediated a call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorityRow {
    pub agent: String,
    pub hop_id: String,
    pub capability: String,
    pub status: String,
    pub reason: String,
}

enum HopAuthority {
    /// Placement hop coverage would refuse. The file check does not mediate.
    CoverageRefuse(String),
    /// Allow matched the placement capability, or this hop id is not a placement.
    Continue,
}

/// Same gate as `estate convey hop` / `call`: deny, deny-default, or a
/// capability that does not match placement coverage. `Ok(None)` stays the
/// lease stub. Does not write and does not spawn.
fn hop_coverage_authority(
    estate: &estate_schema::Estate,
    hop_id: &str,
    agent: &str,
    capability: &str,
) -> HopAuthority {
    match estate_schema::convey_hop_declared_capability(estate, hop_id, Some(agent), capability) {
        Ok(_) => HopAuthority::Continue,
        Err(gate) => HopAuthority::CoverageRefuse(coverage_refuse_reason(&gate)),
    }
}

fn lease_authority_status(
    estate: &estate_schema::Estate,
    lease: &HopLease,
    agent: &str,
) -> (String, String) {
    if hop_is_cloud(&lease.kind) {
        return (
            "not-enforced".into(),
            "cloud hop is declared, not spawned. Not mediated.".into(),
        );
    }
    if !lease.granted {
        return (
            "not-enforced".into(),
            "lease is not granted. Not mediated.".into(),
        );
    }
    match hop_coverage_authority(estate, &lease.hop_id, agent, &lease.capability) {
        HopAuthority::CoverageRefuse(reason) => ("would-deny".into(), reason),
        HopAuthority::Continue => {
            if intention_allows(estate, agent, &lease.capability) {
                (
                    "would-allow".into(),
                    "estate authorize would allow. A worker can still skip convey call. Not mediated.".into(),
                )
            } else {
                (
                    "would-deny".into(),
                    "estate authorize would deny. convey call --agent would refuse:intention. Not mediated unless that call runs.".into(),
                )
            }
        }
    }
}

fn coverage_refuse_reason(gate: &estate_schema::HopCoverageGate) -> String {
    if gate.word == "mismatch" {
        format!(
            "hop coverage would refuse:hop-coverage: {} (mismatch). Placement-derived capability is '{}'. Not mediated.",
            gate.line, gate.capability
        )
    } else {
        format!(
            "hop coverage would refuse:hop-coverage: {} ({}). Not mediated.",
            gate.line, gate.word
        )
    }
}

/// Read-only. Does not write the mesh or the estate. Identity is not resolved.
pub fn authority_report(
    state_dir: &Path,
    estate: &estate_schema::Estate,
) -> Result<Vec<AuthorityRow>, MeshError> {
    let mesh = load_interpreted_mesh(state_dir)?;
    let mut rows = Vec::new();
    for lease in &mesh.leases {
        if lease.agents.is_empty() {
            rows.push(AuthorityRow {
                agent: "(none)".into(),
                hop_id: lease.hop_id.clone(),
                capability: lease.capability.clone(),
                status: "not-enforced".into(),
                reason: "empty population is not a grant. This file check is not mediation.".into(),
            });
            continue;
        }
        for agent in &lease.agents {
            let (status, reason) = lease_authority_status(estate, lease, agent);
            rows.push(AuthorityRow {
                agent: agent.clone(),
                hop_id: lease.hop_id.clone(),
                capability: lease.capability.clone(),
                status,
                reason,
            });
        }
    }
    for place in &estate.placements {
        if place.kind != estate_schema::PlacementKind::Box {
            rows.push(AuthorityRow {
                agent: if place.agents.is_empty() {
                    "(none)".into()
                } else {
                    place.agents.join(",")
                },
                hop_id: place.id.clone(),
                capability: "mesh-stub".into(),
                status: "not-enforced".into(),
                reason: "cloud placement is declared, not spawned. Not mediated.".into(),
            });
            continue;
        }
        for agent_id in &place.agents {
            let Some(agent) = estate.agent(agent_id) else {
                continue;
            };
            let mut caps = Vec::new();
            caps.extend(agent.tools.iter().map(|t| t.id.clone()));
            caps.extend(agent.mounts.iter().map(|m| m.id.clone()));
            caps.extend(agent.mcp.iter().map(|m| m.id.clone()));
            caps.extend(agent.models.iter().map(|m| m.id.clone()));
            for cap in caps {
                let already = rows.iter().any(|row| {
                    row.hop_id == place.id && row.capability == cap && row.agent == *agent_id
                });
                if already {
                    continue;
                }
                rows.push(AuthorityRow {
                    agent: agent_id.clone(),
                    hop_id: place.id.clone(),
                    capability: cap,
                    status: "not-enforced".into(),
                    reason: "declared on the estate; no hop lease names this capability. Not mediated.".into(),
                });
            }
        }
    }
    Ok(rows)
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
                agents: Vec::new(),
            },
            HopDecl {
                id: "cursor-cloud".into(),
                kind: "cloud-mesh".into(),
                capability: "mesh-stub".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
                ttl_secs: None,
                agents: Vec::new(),
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
                agents: Vec::new(),
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
                agents: Vec::new(),
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
                agents: Vec::new(),
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
                agents: Vec::new(),
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
                agents: Vec::new(),
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
    fn call_does_not_call_a_spawned_cloud_lease_not_spawned() {
        let dir = tmp();
        declare_hop(
            &dir,
            HopDecl {
                id: "cursor-cloud".into(),
                kind: "cloud-mesh".into(),
                capability: "mesh-stub".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
                ttl_secs: None,
                agents: Vec::new(),
            },
        )
        .unwrap();
        let mesh_before = std::fs::read_to_string(dir.join(MESH_FILE)).unwrap();
        let hops_before = std::fs::read_to_string(dir.join(HOPS_FILE)).unwrap();
        let leases_before = std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap();
        let placement = serde_json::json!({
            "schema": "cell-one.placement-actual.v0",
            "leases": [{
                "placement_id": "cursor-cloud",
                "kind": "cloud-agent",
                "host_class": "any",
                "spawned": true,
                "wired": true
            }]
        })
        .to_string();
        std::fs::write(dir.join("placement-actual.json"), &placement).unwrap();

        let err = call_hop(&dir, "cursor-cloud", "mesh-stub").unwrap_err();
        assert!(matches!(err, MeshError::CloudSpawned(_)), "{err}");
        let msg = err.to_string();
        assert!(msg.starts_with("refuse:cloud-spawned"), "{msg}");
        assert!(msg.contains("cursor-cloud"), "{msg}");
        assert!(
            !msg.contains("not spawned") && !msg.contains("spawned: false"),
            "{msg}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("placement-actual.json")).unwrap(),
            placement
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(MESH_FILE)).unwrap(),
            mesh_before
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(HOPS_FILE)).unwrap(),
            hops_before
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap(),
            leases_before
        );

        let mut mesh = load_mesh(&dir).unwrap();
        mesh.leases.retain(|l| l.hop_id != "cursor-cloud");
        persist_mesh(&dir, &mesh).unwrap();
        let leases_gone = std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap();
        let err = call_hop(&dir, "cursor-cloud", "mesh-stub").unwrap_err();
        assert!(matches!(err, MeshError::CloudSpawned(_)), "{err}");
        assert!(err.to_string().starts_with("refuse:cloud-spawned"), "{err}");
        assert_eq!(
            std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap(),
            leases_gone,
            "call must not restamp a missing cloud hop lease to spawned:false"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("placement-actual.json")).unwrap(),
            placement
        );

        let unspawned = serde_json::json!({
            "schema": "cell-one.placement-actual.v0",
            "leases": [{
                "placement_id": "cursor-cloud",
                "kind": "cloud-agent",
                "host_class": "any",
                "spawned": false,
                "wired": false
            }]
        })
        .to_string();
        std::fs::write(dir.join("placement-actual.json"), &unspawned).unwrap();
        let err = call_hop(&dir, "cursor-cloud", "mesh-stub").unwrap_err();
        assert!(matches!(err, MeshError::CloudNotSpawned(_)), "{err}");
        assert!(
            err.to_string().starts_with("refuse:cloud-not-spawned"),
            "{err}"
        );

        std::fs::remove_file(dir.join("placement-actual.json")).unwrap();
        let err = call_hop(&dir, "cursor-cloud", "mesh-stub").unwrap_err();
        assert!(matches!(err, MeshError::CloudNotSpawned(_)), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn declare_does_not_invent_unspawned_on_a_spawned_cloud_lease() {
        let dir = tmp();
        declare_hop(
            &dir,
            HopDecl {
                id: "kept-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: Vec::new(),
            },
        )
        .unwrap();
        let mesh_before = std::fs::read_to_string(dir.join(MESH_FILE)).unwrap();
        let hops_before = std::fs::read_to_string(dir.join(HOPS_FILE)).unwrap();
        let leases_before = std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap();
        let placement = serde_json::json!({
            "schema": "cell-one.placement-actual.v0",
            "leases": [{
                "placement_id": "cursor-cloud",
                "kind": "cloud-agent",
                "host_class": "any",
                "spawned": true,
                "wired": true
            }]
        })
        .to_string();
        std::fs::write(dir.join("placement-actual.json"), &placement).unwrap();

        for kind in ["cloud-mesh", "cloud-agent"] {
            let err = declare_hop(
                &dir,
                HopDecl {
                    id: "cursor-cloud".into(),
                    kind: kind.into(),
                    capability: "mesh-stub".into(),
                    host_class: "any".into(),
                    wired: true,
                    note: None,
                    ttl_secs: None,
                    agents: Vec::new(),
                },
            )
            .unwrap_err();
            assert!(matches!(err, MeshError::CloudSpawned(_)), "{err}");
            let msg = err.to_string();
            assert!(msg.starts_with("refuse:cloud-spawned"), "{msg}");
            assert!(msg.contains("cursor-cloud"), "{msg}");
            assert!(!msg.contains("spawned: false"), "{msg}");
            assert_eq!(
                std::fs::read_to_string(dir.join("placement-actual.json")).unwrap(),
                placement
            );
            assert_eq!(
                std::fs::read_to_string(dir.join(MESH_FILE)).unwrap(),
                mesh_before
            );
            assert_eq!(
                std::fs::read_to_string(dir.join(HOPS_FILE)).unwrap(),
                hops_before
            );
            assert_eq!(
                std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap(),
                leases_before
            );
        }

        let unspawned = serde_json::json!({
            "schema": "cell-one.placement-actual.v0",
            "leases": [{
                "placement_id": "cursor-cloud",
                "kind": "cloud-agent",
                "host_class": "any",
                "spawned": false,
                "wired": false
            }]
        })
        .to_string();
        std::fs::write(dir.join("placement-actual.json"), &unspawned).unwrap();
        let lease = declare_hop(
            &dir,
            HopDecl {
                id: "cursor-cloud".into(),
                kind: "cloud-mesh".into(),
                capability: "mesh-stub".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
                ttl_secs: None,
                agents: Vec::new(),
            },
        )
        .unwrap();
        assert!(!lease.spawned);
        assert!(!lease.granted);
        assert_eq!(
            std::fs::read_to_string(dir.join("placement-actual.json")).unwrap(),
            unspawned
        );

        let empty = tmp();
        let fresh = declare_hop(
            &empty,
            HopDecl {
                id: "cursor-cloud".into(),
                kind: "cloud-mesh".into(),
                capability: "mesh-stub".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
                ttl_secs: None,
                agents: Vec::new(),
            },
        )
        .unwrap();
        assert!(!fresh.spawned);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&empty);
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
                agents: Vec::new(),
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
    fn sync_does_not_invent_unspawned_on_a_spawned_cloud_lease() {
        let dir = tmp();
        declare_hop(
            &dir,
            HopDecl {
                id: "kept-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: Vec::new(),
            },
        )
        .unwrap();
        let mesh_before = std::fs::read_to_string(dir.join(MESH_FILE)).unwrap();
        let hops_before = std::fs::read_to_string(dir.join(HOPS_FILE)).unwrap();
        let leases_before = std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap();

        let write_cloud = |spawned: bool, wired: bool| {
            let body = serde_json::json!({
                "schema": "cell-one.placement-actual.v0",
                "desired_hash": "sha256:test",
                "leases": [
                    {
                        "placement_id": "cell-one-box",
                        "kind": "box",
                        "host_class": "any",
                        "spawned": true,
                        "wired": true
                    },
                    {
                        "placement_id": "cursor-cloud",
                        "kind": "cloud-agent",
                        "host_class": "any",
                        "spawned": spawned,
                        "wired": wired
                    }
                ]
            })
            .to_string();
            std::fs::write(dir.join("placement-actual.json"), &body).unwrap();
            body
        };

        for wired in [false, true] {
            let placement = write_cloud(true, wired);
            let err = sync_from_placements(&dir).unwrap_err();
            assert!(matches!(err, MeshError::CloudSpawned(_)), "{err}");
            let msg = err.to_string();
            assert!(msg.starts_with("refuse:cloud-spawned"), "{msg}");
            assert!(msg.contains("cursor-cloud"), "{msg}");
            assert!(
                !msg.contains("spawned\": false") && !msg.contains("spawned: false"),
                "{msg}"
            );
            assert_eq!(
                std::fs::read_to_string(dir.join("placement-actual.json")).unwrap(),
                placement
            );
            assert_eq!(
                std::fs::read_to_string(dir.join(MESH_FILE)).unwrap(),
                mesh_before
            );
            assert_eq!(
                std::fs::read_to_string(dir.join(HOPS_FILE)).unwrap(),
                hops_before
            );
            assert_eq!(
                std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap(),
                leases_before
            );
        }

        let empty = tmp();
        let missing = sync_from_placements(&empty).unwrap();
        assert!(missing.leases.is_empty());

        let unspawned = write_cloud(false, false);
        let mesh = sync_from_placements(&dir).unwrap();
        let cloud = mesh
            .leases
            .iter()
            .find(|l| l.hop_id == "cursor-cloud")
            .unwrap();
        assert!(!cloud.spawned);
        assert!(!cloud.granted);
        assert_eq!(
            std::fs::read_to_string(dir.join("placement-actual.json")).unwrap(),
            unspawned
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&empty);
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
        assert!(err.to_string().contains("refuse:bad-host-class"), "{err}");
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
                agents: Vec::new(),
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
        assert!(err.to_string().contains("refuse:bad-host-class"), "{err}");
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
                agents: Vec::new(),
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
                agents: Vec::new(),
            },
        )
        .unwrap_err();
        assert!(matches!(again, MeshError::BadHostClass(_)));

        assert_eq!(
            std::fs::read_to_string(dir.join(MESH_FILE)).unwrap(),
            before
        );
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
                agents: Vec::new(),
            },
        )
        .unwrap();
        let mut mesh = load_mesh(&dir).unwrap();
        mesh.leases[0].host_class = "not-a-host".into();
        persist_mesh(&dir, &mesh).unwrap();
        let before = std::fs::read_to_string(dir.join(MESH_FILE)).unwrap();
        let err = call_hop(&dir, "cell-one-box", "lane-tool").unwrap_err();
        assert!(matches!(err, MeshError::BadHostClass(ref h) if h == "not-a-host"));
        assert_eq!(
            std::fs::read_to_string(dir.join(MESH_FILE)).unwrap(),
            before
        );
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
        assert!(!file
            .leases
            .iter()
            .any(|l| l.kind == "cloud-mesh" && l.spawned));
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
                agents: Vec::new(),
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
        assert!(fresh.reason.contains("lease-refresh"), "{}", fresh.reason);
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
    fn list_hop_leases_does_not_print_a_spawned_cloud_hop() {
        let dir = tmp();
        declare_hop(
            &dir,
            HopDecl {
                id: "cursor-cloud".into(),
                kind: "cloud-mesh".into(),
                capability: "mesh-stub".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
                ttl_secs: None,
                agents: Vec::new(),
            },
        )
        .unwrap();
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
                agents: Vec::new(),
            },
        )
        .unwrap();

        let unspawned = list_hop_leases(&dir).unwrap();
        assert_eq!(unspawned.len(), 2);
        assert!(unspawned.iter().any(|l| l.hop_id == "cursor-cloud"));
        assert!(!unspawned
            .iter()
            .any(|l| l.kind == "cloud-mesh" && l.spawned));

        let mut mesh = load_mesh(&dir).unwrap();
        for lease in &mut mesh.leases {
            if lease.hop_id == "cell-one-box" {
                lease.spawned = true;
            }
        }
        persist_mesh(&dir, &mesh).unwrap();
        let box_up = list_hop_leases(&dir).unwrap();
        assert!(box_up
            .iter()
            .any(|l| l.hop_id == "cell-one-box" && l.spawned));

        let mut mesh = load_mesh(&dir).unwrap();
        for lease in &mut mesh.leases {
            if lease.hop_id == "cursor-cloud" {
                lease.spawned = true;
            }
        }
        persist_mesh(&dir, &mesh).unwrap();
        let before = std::fs::read_to_string(dir.join(MESH_FILE)).unwrap();
        let leases_before = std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap();
        let err = list_hop_leases(&dir).unwrap_err();
        assert!(matches!(err, MeshError::CloudSpawned(_)), "{err}");
        assert!(err.to_string().starts_with("refuse:cloud-spawned"), "{err}");
        assert!(err.to_string().contains("cursor-cloud"), "{err}");
        assert_eq!(
            std::fs::read_to_string(dir.join(MESH_FILE)).unwrap(),
            before
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap(),
            leases_before
        );

        let empty = tmp();
        assert!(list_hop_leases(&empty).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn forget_does_not_drop_an_expired_spawned_cloud_hop() {
        let dir = tmp();
        declare_hop(
            &dir,
            HopDecl {
                id: "cursor-cloud".into(),
                kind: "cloud-mesh".into(),
                capability: "mesh-stub".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
                ttl_secs: Some(1),
                agents: Vec::new(),
            },
        )
        .unwrap();
        declare_hop(
            &dir,
            HopDecl {
                id: "short-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: Some(1),
                agents: Vec::new(),
            },
        )
        .unwrap();
        let now = hop_now_unix();
        let mut mesh = load_mesh(&dir).unwrap();
        for lease in &mut mesh.leases {
            lease.issued_at = Some(now.saturating_sub(10));
            lease.expires_at = Some(now.saturating_sub(1));
            if lease.hop_id == "cursor-cloud" {
                lease.spawned = true;
            }
        }
        persist_mesh(&dir, &mesh).unwrap();
        let before = std::fs::read_to_string(dir.join(MESH_FILE)).unwrap();
        let hops_before = std::fs::read_to_string(dir.join(HOPS_FILE)).unwrap();
        let leases_before = std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap();

        let listed = list_expired_hop_leases(&dir, now).unwrap_err();
        assert!(matches!(listed, MeshError::CloudSpawned(_)), "{listed}");
        assert!(
            listed.to_string().starts_with("refuse:cloud-spawned"),
            "{listed}"
        );
        assert!(listed.to_string().contains("cursor-cloud"), "{listed}");
        let forgotten = forget_expired_hop_leases(&dir).unwrap_err();
        assert!(
            matches!(forgotten, MeshError::CloudSpawned(_)),
            "{forgotten}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(MESH_FILE)).unwrap(),
            before
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(HOPS_FILE)).unwrap(),
            hops_before
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap(),
            leases_before
        );

        let mut mesh = load_mesh(&dir).unwrap();
        for lease in &mut mesh.leases {
            if lease.hop_id == "cursor-cloud" {
                lease.expires_at = Some(now.saturating_add(3600));
                lease.spawned = true;
            }
        }
        persist_mesh(&dir, &mesh).unwrap();
        let forgotten = forget_expired_hop_leases(&dir).unwrap();
        assert_eq!(forgotten, vec!["short-box".to_string()]);
        let after = load_mesh(&dir).unwrap();
        let cloud = after
            .leases
            .iter()
            .find(|l| l.hop_id == "cursor-cloud")
            .unwrap();
        assert!(cloud.spawned);
        assert!(after.hops.iter().any(|h| h.id == "short-box"));

        let empty = tmp();
        assert!(list_expired_hop_leases(&empty, now).unwrap().is_empty());
        assert!(forget_expired_hop_leases(&empty).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&empty);
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
                agents: Vec::new(),
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
            agents: Vec::new(),
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
            agents: Vec::new(),
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
            agents: Vec::new(),
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
            agents: Vec::new(),
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
                agents: Vec::new(),
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
                agents: Vec::new(),
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

    fn example_estate() -> estate_schema::Estate {
        estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
    }

    #[test]
    fn population_bind_refuses_ahead_of_placement_and_intention() {
        let dir = tmp();
        let placement = serde_json::json!({
            "schema": "cell-one.placement-actual.v0",
            "leases": [{
                "placement_id": "cell-one-box",
                "kind": "box",
                "host_class": "any",
                "spawned": true,
                "wired": true,
                "agents": ["horizon", "research", "sanctum"]
            }]
        })
        .to_string();
        std::fs::write(dir.join("placement-actual.json"), &placement).unwrap();
        let mesh = sync_from_placements(&dir).unwrap();
        let lease = mesh
            .leases
            .iter()
            .find(|l| l.hop_id == "cell-one-box")
            .unwrap();
        assert_eq!(
            lease.agents,
            vec![
                "horizon".to_string(),
                "research".to_string(),
                "sanctum".to_string()
            ]
        );
        let unnamed = call_hop(&dir, "cell-one-box", "lane-tool").unwrap_err();
        assert!(
            matches!(unnamed, MeshError::AgentUnbound { .. }),
            "{unnamed}"
        );
        assert!(
            unnamed.to_string().starts_with("refuse:agent-unbound"),
            "{unnamed}"
        );

        let estate = example_estate();
        // Sync stamps `lane-tool`. That string is not an estate tool.
        let stub = call_hop_for_agent(&dir, "cell-one-box", "lane-tool", "research", None, &estate)
            .unwrap_err();
        assert!(matches!(stub, MeshError::Intention { .. }), "{stub}");
        assert!(stub.to_string().starts_with("refuse:intention"), "{stub}");

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
                agents: vec!["research".into(), "horizon".into()],
            },
        )
        .unwrap();
        let uncovered =
            call_hop_for_agent(&dir, "notes-hop", "notes-append", "research", None, &estate)
                .unwrap_err();
        assert!(
            uncovered.to_string().starts_with("refuse:intention"),
            "{uncovered}"
        );
        assert!(
            uncovered
                .to_string()
                .contains("not covered by an allow Tool intention"),
            "{uncovered}"
        );
        assert!(uncovered.to_string().contains("deny-default"), "{uncovered}");
        let mut granted = estate.clone();
        granted.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "notes-append".into(),
            kind: estate_schema::IntentionKind::Tool,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
        let allowed =
            call_hop_for_agent(&dir, "notes-hop", "notes-append", "research", None, &granted)
                .unwrap();
        assert!(allowed.allow);
        assert!(
            allowed.reason.contains("agent-bound research"),
            "{}",
            allowed.reason
        );
        let denied = call_hop_for_agent(
            &dir,
            "notes-hop",
            "notes-append",
            "horizon",
            Some(estate_schema::IntentionKind::Tool),
            &estate,
        )
        .unwrap_err();
        assert!(matches!(denied, MeshError::Intention { .. }), "{denied}");
        assert!(
            denied.to_string().starts_with("refuse:intention"),
            "{denied}"
        );
        let unnamed_notes = call_hop(&dir, "notes-hop", "notes-append").unwrap_err();
        assert!(
            matches!(unnamed_notes, MeshError::AgentUnbound { .. }),
            "{unnamed_notes}"
        );

        let missing = call_hop_for_agent(
            &dir,
            "cell-one-box",
            "notes-append",
            "not-placed",
            None,
            &estate,
        )
        .unwrap_err();
        assert!(
            matches!(missing, MeshError::AgentUnbound { .. }),
            "{missing}"
        );

        let before = std::fs::read_to_string(dir.join(MESH_FILE)).unwrap();
        let mut tampered = load_mesh(&dir).unwrap();
        tampered.leases[0].agents.push("outsider".into());
        tampered.hops[0].agents.push("outsider".into());
        persist_mesh(&dir, &tampered).unwrap();
        let wider = std::fs::read_to_string(dir.join(MESH_FILE)).unwrap();
        let listed = list_hop_leases(&dir).unwrap_err();
        assert!(
            matches!(listed, MeshError::AgentUnplaced { .. }),
            "{listed}"
        );
        assert!(
            listed.to_string().starts_with("refuse:agent-unplaced"),
            "{listed}"
        );
        assert!(listed.to_string().contains("outsider"), "{listed}");
        let sync = sync_from_placements(&dir).unwrap_err();
        assert!(matches!(sync, MeshError::AgentUnplaced { .. }), "{sync}");
        assert_eq!(std::fs::read_to_string(dir.join(MESH_FILE)).unwrap(), wider);
        assert_eq!(
            std::fs::read_to_string(dir.join("placement-actual.json")).unwrap(),
            placement
        );
        let _ = before;

        std::fs::write(dir.join(MESH_FILE), &before).unwrap();
        let hops_ok = std::fs::read_to_string(dir.join(HOPS_FILE)).unwrap();
        let leases_ok = std::fs::read_to_string(dir.join(LEASES_FILE)).unwrap();
        // restore sibling files from the honest sync by rewriting via load of before mesh
        let honest: ConveyorMesh = serde_json::from_str(&before).unwrap();
        persist_mesh(&dir, &honest).unwrap();
        let _ = (hops_ok, leases_ok);

        let ahead = declare_hop(
            &dir,
            HopDecl {
                id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: vec!["horizon".into(), "outsider".into()],
            },
        )
        .unwrap_err();
        assert!(matches!(ahead, MeshError::AgentUnplaced { .. }), "{ahead}");
        assert_eq!(
            std::fs::read_to_string(dir.join("placement-actual.json")).unwrap(),
            placement
        );

        let sacred = declare_hop(
            &dir,
            HopDecl {
                id: "side-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: vec!["cyera-ci".into()],
            },
        )
        .unwrap_err();
        assert!(matches!(sacred, MeshError::SacredId(_)), "{sacred}");
        assert!(
            sacred.to_string().starts_with("refuse:sacred-id"),
            "{sacred}"
        );
        assert!(!load_mesh(&dir)
            .unwrap()
            .hops
            .iter()
            .any(|h| h.id == "side-box"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn authority_does_not_call_a_file_check_enforced() {
        let dir = tmp();
        let placement = serde_json::json!({
            "schema": "cell-one.placement-actual.v0",
            "leases": [{
                "placement_id": "cell-one-box",
                "kind": "box",
                "host_class": "any",
                "spawned": true,
                "wired": true,
                "agents": ["horizon", "research", "sanctum"]
            }]
        })
        .to_string();
        std::fs::write(dir.join("placement-actual.json"), &placement).unwrap();
        let estate = example_estate();
        let mesh = sync_from_placements(&dir).unwrap();
        assert!(mesh.leases.iter().any(|l| l.hop_id == "cell-one-box" && l.granted));
        let rows = authority_report(&dir, &estate).unwrap();
        assert!(rows.iter().all(|row| row.status != "enforced"));
        assert!(rows.iter().any(|row| {
            row.capability == "lane-tool" && row.status == "would-deny" && row.reason.contains("Not mediated")
        }));
        assert!(rows.iter().any(|row| {
            row.agent == "research"
                && row.capability == "notes-append"
                && row.status == "not-enforced"
                && row.reason.contains("no hop lease")
        }));

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
                agents: vec!["research".into(), "horizon".into()],
            },
        )
        .unwrap();
        let rows = authority_report(&dir, &estate).unwrap();
        assert!(rows.iter().all(|row| row.status != "enforced"));
        assert!(rows.iter().any(|row| {
            row.agent == "research" && row.capability == "notes-append" && row.status == "would-deny"
        }));
        let mut granted = estate.clone();
        granted.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "tool:notes-append".into(),
            kind: estate_schema::IntentionKind::Tool,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
        let rows = authority_report(&dir, &granted).unwrap();
        assert!(rows.iter().any(|row| {
            row.agent == "research" && row.capability == "notes-append" && row.status == "would-allow"
        }));
        assert!(rows.iter().any(|row| {
            row.agent == "horizon" && row.capability == "notes-append" && row.status == "would-deny"
        }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn row<'a>(
        rows: &'a [AuthorityRow],
        agent: &str,
        hop: &str,
        capability: &str,
    ) -> &'a AuthorityRow {
        rows.iter()
            .find(|row| row.agent == agent && row.hop_id == hop && row.capability == capability)
            .unwrap_or_else(|| panic!("missing {agent} {hop} {capability}"))
    }

    #[test]
    fn authority_cites_hop_coverage_before_intention_allow() {
        let dir = tmp();
        let mut allow = example_estate();
        allow
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap()
            .tools
            .push(estate_schema::ToolDecl {
                id: "lane-tool".into(),
                description: None,
            });
        allow.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "lane-tool".into(),
            kind: estate_schema::IntentionKind::Tool,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
        allow.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "notes-append".into(),
            kind: estate_schema::IntentionKind::Tool,
            effect: estate_schema::Effect::Allow,
            note: None,
        });

        declare_hop(
            &dir,
            HopDecl {
                id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: vec!["research".into()],
            },
        )
        .unwrap();
        let before = std::fs::read(dir.join(MESH_FILE)).unwrap();
        let rows = authority_report(&dir, &allow).unwrap();
        assert_eq!(std::fs::read(dir.join(MESH_FILE)).unwrap(), before);
        let mismatch = row(&rows, "research", "cell-one-box", "notes-append");
        assert_eq!(mismatch.status, "would-deny");
        assert_ne!(mismatch.status, "would-allow");
        assert!(
            mismatch.reason.contains("refuse:hop-coverage")
                && mismatch.reason.contains("(mismatch)")
                && mismatch.reason.contains("lane-tool")
                && mismatch.reason.contains("Not mediated"),
            "{}",
            mismatch.reason
        );
        assert!(!mismatch.reason.contains("estate authorize would allow"));

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
                agents: vec!["research".into()],
            },
        )
        .unwrap();
        let rows = authority_report(&dir, &allow).unwrap();
        let matched = row(&rows, "research", "cell-one-box", "lane-tool");
        assert_eq!(matched.status, "would-allow");
        assert!(
            matched.reason.contains("estate authorize would allow")
                && matched.reason.contains("Not mediated"),
            "{}",
            matched.reason
        );
        assert!(!matched.reason.contains("refuse:hop-coverage"));

        let mut denied = allow.clone();
        denied
            .intentions
            .retain(|intention| intention.object != "lane-tool");
        denied.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "lane-tool".into(),
            kind: estate_schema::IntentionKind::Tool,
            effect: estate_schema::Effect::Deny,
            note: None,
        });
        declare_hop(
            &dir,
            HopDecl {
                id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: vec!["research".into()],
            },
        )
        .unwrap();
        let rows = authority_report(&dir, &denied).unwrap();
        let hop_deny = row(&rows, "research", "cell-one-box", "notes-append");
        assert_eq!(hop_deny.status, "would-deny");
        assert!(
            hop_deny.reason.contains("refuse:hop-coverage")
                && hop_deny.reason.contains("(deny)")
                && !hop_deny.reason.contains("deny-default")
                && !hop_deny.reason.contains("(mismatch)")
                && hop_deny.reason.contains("Not mediated"),
            "{}",
            hop_deny.reason
        );

        let mut defaulted = example_estate();
        defaulted.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "notes-append".into(),
            kind: estate_schema::IntentionKind::Tool,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
        let rows = authority_report(&dir, &defaulted).unwrap();
        let hop_default = row(&rows, "research", "cell-one-box", "notes-append");
        assert_eq!(hop_default.status, "would-deny");
        assert!(
            hop_default.reason.contains("refuse:hop-coverage")
                && hop_default.reason.contains("(deny-default)")
                && hop_default.reason.contains("Not mediated"),
            "{}",
            hop_default.reason
        );
        assert!(!hop_default.reason.contains("estate authorize would allow"));

        declare_hop(
            &dir,
            HopDecl {
                id: "cursor-cloud".into(),
                kind: "cloud-mesh".into(),
                capability: "mesh-stub".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
                ttl_secs: None,
                agents: vec!["research".into()],
            },
        )
        .unwrap();
        declare_hop(
            &dir,
            HopDecl {
                id: "empty-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: Vec::new(),
            },
        )
        .unwrap();
        let rows = authority_report(&dir, &allow).unwrap();
        let cloud = row(&rows, "research", "cursor-cloud", "mesh-stub");
        assert_eq!(cloud.status, "not-enforced");
        assert!(
            cloud.reason.contains("cloud hop is declared, not spawned")
                && cloud.reason.contains("Not mediated"),
            "{}",
            cloud.reason
        );
        let empty = row(&rows, "(none)", "empty-box", "lane-tool");
        assert_eq!(empty.status, "not-enforced");
        assert!(empty.reason.contains("empty population is not a grant"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn model_class_intentions_refuse_the_same_way_for_frontier_and_local() {
        let dir = tmp();
        let estate = example_estate();
        for (hop, capability, class) in [
            ("frontier-hop", "xai_grok", "frontier"),
            ("local-hop", "local_slm", "local"),
        ] {
            declare_hop(
                &dir,
                HopDecl {
                    id: hop.into(),
                    kind: "box".into(),
                    capability: capability.into(),
                    host_class: "any".into(),
                    wired: true,
                    note: None,
                    ttl_secs: None,
                    agents: vec!["horizon".into()],
                },
            )
            .unwrap();
            let denied = call_hop_for_agent(
                &dir,
                hop,
                capability,
                "horizon",
                Some(estate_schema::IntentionKind::Model),
                &estate,
            )
            .unwrap_err();
            assert!(
                denied.to_string().starts_with("refuse:intention"),
                "{denied}"
            );
            assert!(
                denied
                    .to_string()
                    .contains(&format!("model class '{class}'")),
                "{denied}"
            );
            assert!(denied.to_string().contains("deny-default"), "{denied}");
        }

        let missing = call_hop_for_agent(
            &dir,
            "frontier-hop",
            "other_slm",
            "horizon",
            Some(estate_schema::IntentionKind::Model),
            &estate,
        )
        .unwrap_err();
        let missing = missing.to_string();
        assert!(missing.starts_with("refuse:capability"), "{missing}");

        declare_hop(
            &dir,
            HopDecl {
                id: "undeclared-hop".into(),
                kind: "box".into(),
                capability: "other_slm".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: vec!["horizon".into()],
            },
        )
        .unwrap();
        let undeclared = call_hop_for_agent(
            &dir,
            "undeclared-hop",
            "other_slm",
            "horizon",
            Some(estate_schema::IntentionKind::Model),
            &estate,
        )
        .unwrap_err();
        assert!(
            undeclared
                .to_string()
                .contains("not declared on model_bindings"),
            "{undeclared}"
        );

        let mut granted = estate.clone();
        for class in ["frontier", "local"] {
            granted.intentions.push(estate_schema::Intention {
                subject_agent: "horizon".into(),
                object: format!("class:{class}"),
                kind: estate_schema::IntentionKind::Model,
                effect: estate_schema::Effect::Allow,
                note: None,
            });
        }
        for (hop, capability, class) in [
            ("frontier-hop", "xai_grok", "frontier"),
            ("local-hop", "local_slm", "local"),
        ] {
            let allowed =
                call_hop_for_agent(&dir, hop, capability, "horizon", None, &granted).unwrap();
            assert!(allowed.allow, "{capability}");
            assert!(
                allowed.reason.contains(&format!("class {class}")),
                "{}",
                allowed.reason
            );
        }
        let local_only = {
            let mut estate = estate.clone();
            estate.intentions.push(estate_schema::Intention {
                subject_agent: "horizon".into(),
                object: "class:local".into(),
                kind: estate_schema::IntentionKind::Model,
                effect: estate_schema::Effect::Allow,
                note: None,
            });
            estate
        };
        let frontier_still = call_hop_for_agent(
            &dir,
            "frontier-hop",
            "xai_grok",
            "horizon",
            Some(estate_schema::IntentionKind::Model),
            &local_only,
        )
        .unwrap_err();
        assert!(
            frontier_still
                .to_string()
                .contains("model class 'frontier'"),
            "{frontier_still}"
        );

        declare_hop(
            &dir,
            HopDecl {
                id: "cloud-model".into(),
                kind: "cloud-mesh".into(),
                capability: "xai_grok".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
                ttl_secs: None,
                agents: vec!["horizon".into()],
            },
        )
        .unwrap();
        let cloud = call_hop_for_agent(
            &dir,
            "cloud-model",
            "xai_grok",
            "horizon",
            Some(estate_schema::IntentionKind::Model),
            &granted,
        )
        .unwrap_err();
        assert!(matches!(cloud, MeshError::CloudNotSpawned(_)), "{cloud}");
        assert!(
            cloud.to_string().starts_with("refuse:cloud-not-spawned"),
            "{cloud}"
        );

        let sacred = call_hop_for_agent(
            &dir,
            "frontier-hop",
            "xai_grok",
            "cyera-ci",
            Some(estate_schema::IntentionKind::Model),
            &granted,
        )
        .unwrap_err();
        assert!(matches!(sacred, MeshError::SacredId(_)), "{sacred}");
        let classroom = call_hop_for_agent(
            &dir,
            "local-hop",
            "local_slm",
            "rust-classroom",
            Some(estate_schema::IntentionKind::Model),
            &granted,
        )
        .unwrap_err();
        assert!(matches!(classroom, MeshError::SacredId(_)), "{classroom}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tool_mcp_and_mount_intentions_refuse_without_coverage() {
        let dir = tmp();
        let mut estate = example_estate();
        estate
            .agents
            .iter_mut()
            .find(|a| a.id == "research")
            .unwrap()
            .mcp
            .push(estate_schema::McpDecl {
                id: "docs".into(),
                description: None,
            });
        for (hop, capability, kind) in [
            ("notes-hop", "notes-append", estate_schema::IntentionKind::Tool),
            ("docs-hop", "docs", estate_schema::IntentionKind::Mcp),
            ("mount-hop", "notes", estate_schema::IntentionKind::Mount),
        ] {
            declare_hop(
                &dir,
                HopDecl {
                    id: hop.into(),
                    kind: "box".into(),
                    capability: capability.into(),
                    host_class: "any".into(),
                    wired: true,
                    note: None,
                    ttl_secs: None,
                    agents: vec!["research".into()],
                },
            )
            .unwrap();
            let denied =
                call_hop_for_agent(&dir, hop, capability, "research", Some(kind), &estate)
                    .unwrap_err();
            let text = denied.to_string();
            assert!(text.starts_with("refuse:intention"), "{text}");
            assert!(text.contains("deny-default"), "{text}");
            assert!(text.contains("not covered by an allow"), "{text}");
            if kind == estate_schema::IntentionKind::Mount {
                assert!(
                    text.contains("not covered by an allow Mount intention"),
                    "{text}"
                );
            }
        }
        let mut granted = estate.clone();
        granted.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "mcp:docs".into(),
            kind: estate_schema::IntentionKind::Mcp,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
        granted.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "docs".into(),
            kind: estate_schema::IntentionKind::Mcp,
            effect: estate_schema::Effect::Deny,
            note: None,
        });
        let denied = call_hop_for_agent(
            &dir,
            "docs-hop",
            "docs",
            "research",
            Some(estate_schema::IntentionKind::Mcp),
            &granted,
        )
        .unwrap_err();
        assert!(denied.to_string().contains("explicit deny"), "{denied}");

        let mut mount_allow = estate.clone();
        mount_allow.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "mount:notes".into(),
            kind: estate_schema::IntentionKind::Mount,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
        let allowed = call_hop_for_agent(
            &dir,
            "mount-hop",
            "notes",
            "research",
            Some(estate_schema::IntentionKind::Mount),
            &mount_allow,
        )
        .unwrap();
        assert!(allowed.allow, "{}", allowed.reason);
        assert!(allowed.reason.contains("allow intention"), "{}", allowed.reason);
        mount_allow.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "notes".into(),
            kind: estate_schema::IntentionKind::Mount,
            effect: estate_schema::Effect::Deny,
            note: None,
        });
        let mount_denied = call_hop_for_agent(
            &dir,
            "mount-hop",
            "notes",
            "research",
            None,
            &mount_allow,
        )
        .unwrap_err();
        assert!(
            mount_denied.to_string().contains("explicit deny"),
            "{mount_denied}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
