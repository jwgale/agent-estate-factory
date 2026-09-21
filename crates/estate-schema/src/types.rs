use crate::sacred::{is_sacred_name, normalize_name};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Estate {
    pub version: u32,
    /// Cell One config version. Absent = legacy `version: 0`.
    #[serde(
        default,
        rename = "apiVersion",
        alias = "api_version",
        skip_serializing_if = "Option::is_none"
    )]
    pub api_version: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    pub name: String,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub default_effect: Effect,
    pub agents: Vec<Agent>,
    pub lanes: Vec<Lane>,
    #[serde(default)]
    pub intentions: Vec<Intention>,
    #[serde(default)]
    pub model_bindings: Vec<ModelBinding>,
    #[serde(default)]
    pub sacred_exclusions: Vec<SacredExclusion>,
    /// First specialist enrich packs. Jason curates; policy is manual. Not auto-promote.
    #[serde(default)]
    pub enrich_packs: EnrichPacks,
    /// Where agents run. `box` is Cell One today. `cloud-agent` is declared, not spawned.
    #[serde(default)]
    pub placements: Vec<Placement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Agent {
    pub id: String,
    pub display_name: String,
    pub lane: String,
    pub desktop: String,
    #[serde(default)]
    pub tools: Vec<ToolDecl>,
    #[serde(default)]
    pub mounts: Vec<MountDecl>,
    #[serde(default)]
    pub mcp: Vec<McpDecl>,
    /// Declared model-binding allow-list (deny-default, same class as tools).
    #[serde(default)]
    pub models: Vec<ModelUseDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lane {
    pub id: String,
    pub root_path: String,
    pub owner_agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Intention {
    pub subject_agent: String,
    pub object: String,
    pub kind: IntentionKind,
    pub effect: Effect,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentionKind {
    MemoryRead,
    Tool,
    Mcp,
    Mount,
    Model,
}

impl IntentionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            IntentionKind::MemoryRead => "memory_read",
            IntentionKind::Tool => "tool",
            IntentionKind::Mcp => "mcp",
            IntentionKind::Mount => "mount",
            IntentionKind::Model => "model",
        }
    }
}

impl FromStr for IntentionKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalize_name(s).as_str() {
            "memory_read" | "memory-read" | "memory" => Ok(IntentionKind::MemoryRead),
            "tool" => Ok(IntentionKind::Tool),
            "mcp" => Ok(IntentionKind::Mcp),
            "mount" => Ok(IntentionKind::Mount),
            "model" | "binding" => Ok(IntentionKind::Model),
            other => Err(format!("unknown intention kind '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Deny,
    Allow,
}

impl Default for Effect {
    fn default() -> Self {
        Effect::Deny
    }
}

impl Effect {
    pub fn as_str(self) -> &'static str {
        match self {
            Effect::Deny => "deny",
            Effect::Allow => "allow",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelUseDecl {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolDecl {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MountDecl {
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpDecl {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelBinding {
    pub id: String,
    pub class: ModelClass,
    pub driver: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub wired: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModelClass {
    Frontier,
    Local,
}

impl ModelClass {
    pub fn as_str(self) -> &'static str {
        match self {
            ModelClass::Frontier => "frontier",
            ModelClass::Local => "local",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnrichPack {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnrichPacks {
    #[serde(default = "default_enrich_curator")]
    pub curator: String,
    #[serde(default = "default_enrich_policy")]
    pub policy: String,
    #[serde(default)]
    pub packs: Vec<EnrichPack>,
}

fn default_enrich_curator() -> String {
    "jason".into()
}

fn default_enrich_policy() -> String {
    "manual".into()
}

impl Default for EnrichPacks {
    fn default() -> Self {
        Self {
            curator: default_enrich_curator(),
            policy: default_enrich_policy(),
            packs: Vec::new(),
        }
    }
}

/// Day-90 operator placement. Declared on the estate; floor does not spawn cloud agents.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PlacementKind {
    Box,
    CloudAgent,
}

impl PlacementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PlacementKind::Box => "box",
            PlacementKind::CloudAgent => "cloud-agent",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Placement {
    pub id: String,
    pub kind: PlacementKind,
    #[serde(default)]
    pub host_class: Option<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub wired: bool,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub note: Option<String>,
    /// Optional lease lifetime in seconds. Absent = no expiry.
    #[serde(default)]
    pub ttl_secs: Option<u64>,
}

/// Locked host_class names: `consumer-nvidia` | `apple-silicon` | `rented-nvidia` | `any`.
/// Aliases (`rtx-consumer` / `rtx_consumer`, `nvidia-rental` / `nvidia_rental`) map onto
/// those names. Do not reopen the contract.
pub fn normalize_host_class(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "consumer-nvidia" | "rtx-consumer" => Some("consumer-nvidia"),
        "apple-silicon" => Some("apple-silicon"),
        "rented-nvidia" | "nvidia-rental" => Some("rented-nvidia"),
        "any" => Some("any"),
        _ => None,
    }
}

/// Portable host class. Hardware is a driver choice, not a SKU.
pub fn is_host_class(raw: &str) -> bool {
    normalize_host_class(raw).is_some()
}

/// True when both sides normalize to the same locked host_class.
pub fn host_class_eq(a: &str, b: &str) -> bool {
    match (normalize_host_class(a), normalize_host_class(b)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

/// Map a raw host_class to the locked name.
/// Empty/unset → `Some("any")`. Unknown (SKU, garbage) → `None` (do not invent `any`).
pub fn canonical_host_class_opt(raw: Option<&str>) -> Option<&'static str> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Some("any"),
        Some(value) => normalize_host_class(value),
    }
}

/// Canonical locked name, or `"any"` when unset/empty.
/// Trusted post-validate stamps only. Untrusted disk must use
/// [`canonical_host_class_opt`] so unknown names can refuse.
pub fn canonical_host_class(raw: Option<&str>) -> &'static str {
    canonical_host_class_opt(raw).unwrap_or("any")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SacredExclusion {
    pub id: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectRef {
    Lane(String),
    Tool(String),
    Mcp(String),
    Mount(String),
    Binding(String),
    Exclusion(String),
    Bare(String),
}

impl ObjectRef {
    pub fn parse(raw: &str) -> Self {
        let raw = raw.trim();
        if let Some(rest) = raw.strip_prefix("lane:") {
            ObjectRef::Lane(rest.to_string())
        } else if let Some(rest) = raw.strip_prefix("tool:") {
            ObjectRef::Tool(rest.to_string())
        } else if let Some(rest) = raw.strip_prefix("mcp:") {
            ObjectRef::Mcp(rest.to_string())
        } else if let Some(rest) = raw.strip_prefix("mount:") {
            ObjectRef::Mount(rest.to_string())
        } else if let Some(rest) = raw.strip_prefix("binding:") {
            ObjectRef::Binding(rest.to_string())
        } else if let Some(rest) = raw.strip_prefix("exclusion:") {
            ObjectRef::Exclusion(rest.to_string())
        } else {
            ObjectRef::Bare(raw.to_string())
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ObjectRef::Lane(s)
            | ObjectRef::Tool(s)
            | ObjectRef::Mcp(s)
            | ObjectRef::Mount(s)
            | ObjectRef::Binding(s)
            | ObjectRef::Exclusion(s)
            | ObjectRef::Bare(s) => s,
        }
    }
}

impl Estate {
    pub fn agent(&self, id: &str) -> Option<&Agent> {
        let n = normalize_name(id);
        self.agents.iter().find(|a| normalize_name(&a.id) == n)
    }

    pub fn lane(&self, id: &str) -> Option<&Lane> {
        let n = normalize_name(id);
        self.lanes.iter().find(|l| normalize_name(&l.id) == n)
    }

    pub fn lane_for_agent(&self, agent_id: &str) -> Option<&Lane> {
        self.lanes
            .iter()
            .find(|l| normalize_name(&l.owner_agent_id) == normalize_name(agent_id))
    }

    pub fn is_sacred(&self, name: &str) -> bool {
        if is_sacred_name(name) {
            return true;
        }
        let n = normalize_name(ObjectRef::parse(name).name());
        self.sacred_exclusions.iter().any(|ex| {
            normalize_name(&ex.id) == n
                || ex.aliases.iter().any(|a| normalize_name(a) == n)
        })
    }

    pub fn owns_lane(&self, agent_id: &str, lane_id: &str) -> bool {
        match (self.agent(agent_id), self.lane(lane_id)) {
            (Some(agent), Some(lane)) => {
                normalize_name(&agent.lane) == normalize_name(&lane.id)
                    && normalize_name(&lane.owner_agent_id) == normalize_name(&agent.id)
            }
            _ => false,
        }
    }
}

impl Agent {
    pub fn has_tool(&self, id: &str) -> bool {
        let n = normalize_name(id);
        self.tools.iter().any(|t| normalize_name(&t.id) == n)
    }

    pub fn has_mount(&self, id: &str) -> bool {
        let n = normalize_name(id);
        self.mounts.iter().any(|m| normalize_name(&m.id) == n)
    }

    pub fn has_mcp(&self, id: &str) -> bool {
        let n = normalize_name(id);
        self.mcp.iter().any(|m| normalize_name(&m.id) == n)
    }

    pub fn has_model(&self, id: &str) -> bool {
        let n = normalize_name(id);
        self.models.iter().any(|m| normalize_name(&m.id) == n)
    }
}
