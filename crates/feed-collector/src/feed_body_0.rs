use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(String),
    #[error("no auto-promote: Jason curates enrich packs by hand (policy=manual)")]
    NoAutoPromote,
    #[error("pack id '{0}' encodes a hardware SKU; hardware is a driver choice")]
    SkuBanned(String),
    #[error("pack id '{0}' must match [a-z][a-z0-9_-]{{0,63}}")]
    BadId(String),
    #[error("pack schema must be cell-one.pack.v0 or cell-one.specialist-pack.v0 (got {0})")]
    BadSchema(String),
    #[error("pack host_class '{0}' must be consumer-nvidia|apple-silicon|rented-nvidia|any")]
    BadHostClass(String),
    #[error("refuse:missing-pack: no pack '{0}' in drop or accepted")]
    MissingPack(String),
    #[error("refuse:no-auto-apply: enrich proposals are curator-only; Jason reviews and edits the estate")]
    NoAutoApply,
    #[error("refuse:raw-secret: pack must not store raw secrets")]
    RawSecret,
    #[error("refuse:model-hint: '{0}' must be a slug and must not encode a hardware SKU")]
    BadModelHint(String),
    #[error("refuse:source-path: '{0}' is not a relative path (or encodes a SKU)")]
    BadSourcePath(String),
    #[error("refuse:source-driver: '{0}'")]
    BadSourceDriver(String),
    #[error(
        "refuse:frontier-invent: no frontier binding; will not invent a frontier source_driver"
    )]
    FrontierInvent,
    #[error("refuse:curator: curator '{provided}' does not match locked curator '{want}'")]
    WrongCurator { provided: String, want: String },
}

/// Overnight lock. Jason curates. Do not reopen.
pub const LOCKED_CURATOR: &str = "jason";

pub fn refuse_curator(provided: &str, estate_curator: &str) -> Result<(), FeedError> {
    let have = provided.trim().to_ascii_lowercase();
    let estate = estate_curator.trim().to_ascii_lowercase();
    if have != LOCKED_CURATOR {
        return Err(FeedError::WrongCurator {
            provided: provided.to_string(),
            want: LOCKED_CURATOR.into(),
        });
    }
    if !estate.is_empty() && estate != LOCKED_CURATOR {
        return Err(FeedError::WrongCurator {
            provided: estate_curator.to_string(),
            want: LOCKED_CURATOR.into(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScrubbedEvent {
    pub kind: String,
    pub agent_id: Option<String>,
    pub decision: Option<String>,
    pub object_class: Option<String>,
    pub note: Option<String>,
    pub ts: String,
}

/// Stable pack schema (cell-one.pack.v0). `promoted` is never set by the feed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackManifest {
    pub id: String,
    pub version: u32,
    #[serde(default = "default_pack_schema")]
    pub schema: String,
    pub curator: String,
    pub policy: String,
    pub promoted: bool,
    pub source: String,
    pub from_events: usize,
    pub kinds: Vec<String>,
    pub paths: Vec<String>,
    pub agents: Vec<String>,
    #[serde(default = "default_host_class")]
    pub host_class: String,
    #[serde(default)]
    pub job: Option<String>,
    /// Day-90+ LoRA/adapter slot. Empty on the beachhead.
    #[serde(default)]
    pub adapter: Option<String>,
    #[serde(default)]
    pub path_counts: PathCounts,
    /// Relative paths the specialist pack was built from. Never secrets.
    #[serde(default)]
    pub source_paths: Vec<String>,
    /// Binding or driver hint (`ollama`, `local_slm`). Not a SKU.
    #[serde(default)]
    pub model_hint: Option<String>,
    /// Hugging Face repo id (`namespace/name`) or a local directory of HF weights.
    /// `llamafactory-qlora` writes this to `model_name_or_path`. An Ollama seat tag
    /// (`llama3`, `llama3:latest`) is not a train base. Empty on older packs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub train_base_model: Option<String>,
    /// Classes that contributed events. Only `frontier` and `local`, sorted.
    /// Empty when the pack was not built from those events. Not a promote flag.
    #[serde(default)]
    pub source_drivers: Vec<String>,
    /// Portable host affinity. Defaults to `host_class` / `any`.
    #[serde(default)]
    pub host_class_affinity: Option<String>,
    pub created_at: String,
    pub note: String,
}

/// Equal-class path mix on a candidate pack. Not a SKU.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PathCounts {
    #[serde(default)]
    pub frontier: usize,
    #[serde(default)]
    pub local: usize,
    #[serde(default)]
    pub proxy: usize,
    #[serde(default)]
    pub other: usize,
}

/// Durable watermark. Survives rematerialize. Not estate SoT.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedCursor {
    #[serde(default = "default_cursor_schema")]
    pub schema: String,
    pub events: usize,
    pub last_ts: Option<String>,
    pub last_kind: Option<String>,
    pub packed_id: Option<String>,
    pub updated_at: String,
}

fn default_cursor_schema() -> String {
    "cell-one.feed-cursor.v0".into()
}

/// Explicit import audit line. Never silent. Never auto-promote.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportAudit {
    pub imported_at: String,
    pub pack_id: String,
    pub estate_bound: bool,
    pub source_path: String,
    pub promoted: bool,
}

fn default_pack_schema() -> String {
    "cell-one.pack.v0".into()
}

fn default_host_class() -> String {
    "any".into()
}

/// Explicit import record. Not estate SoT. Never silent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportedPack {
    pub pack: PackManifest,
    pub estate_bound: bool,
    pub imported_at: String,
    pub source_path: String,
}

pub fn classify_path(ev: &ScrubbedEvent) -> &'static str {
    match ev.object_class.as_deref().unwrap_or("") {
        "frontier" => return "frontier",
        "local" => return "local",
        "proxy" => return "proxy",
        _ => {}
    }
    if ev.kind.contains("frontier") {
        "frontier"
    } else if ev.kind.contains("local") {
        "local"
    } else if ev.kind.contains("proxy") {
        "proxy"
    } else {
        "other"
    }
}

/// Unique `frontier` / `local` classes on the events. Sorted. Proxy stays a count.
pub fn source_drivers_from_events(events: &[ScrubbedEvent]) -> Vec<String> {
    let mut drivers = BTreeSet::new();
    for ev in events {
        match classify_path(ev) {
            "frontier" | "local" => {
                drivers.insert(classify_path(ev).to_string());
            }
            _ => {}
        }
    }
    drivers.into_iter().collect()
}

pub fn refuse_event(ev: &ScrubbedEvent) -> Result<(), FeedError> {
    for field in [
        ev.kind.as_str(),
        ev.agent_id.as_deref().unwrap_or(""),
        ev.object_class.as_deref().unwrap_or(""),
        ev.note.as_deref().unwrap_or(""),
    ] {
        if !field.is_empty() && estate_schema::contains_sku(field) {
            return Err(FeedError::SkuBanned(field.to_string()));
        }
    }
    Ok(())
}

pub const REDACTION_SCHEMA: &str = "cell-one.redaction.v0";

/// Kind counts only. Never includes the raw secret.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RedactionReport {
    #[serde(default = "default_redaction_schema")]
    pub schema: String,
    pub redacted: usize,
    pub kinds: std::collections::BTreeMap<String, usize>,
    pub raw_secrets_found: usize,
    pub note: String,
}

fn default_redaction_schema() -> String {
    REDACTION_SCHEMA.into()
}

fn is_secret_key_name(part: &str) -> bool {
    matches!(
        part.trim().to_ascii_lowercase().as_str(),
        "password"
            | "secret"
            | "token"
            | "api_key"
            | "apikey"
            | "authorization"
            | "auth"
            | "access_key"
            | "private_key"
    )
}

/// Classify a token part. Returns a kind label, never the secret.
pub fn classify_secret_part(part: &str) -> Option<&'static str> {
    let t = part.trim_matches(|c: char| {
        !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.' && c != '@' && c != '+'
    });
    if t.is_empty() {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    if (lower.starts_with("sk-")
        || lower.starts_with("sk_ant")
        || lower.starts_with("xai-")
        || lower.starts_with("xai_"))
        && t.len() >= 12
    {
        return Some("api-key");
    }
    if lower.starts_with("ghp_")
        || lower.starts_with("gho_")
        || lower.starts_with("ghu_")
        || lower.starts_with("ghs_")
        || lower.starts_with("github_pat_")
    {
        return Some("github-token");
    }
    if lower.starts_with("hf_") && t.len() >= 12 {
        return Some("hf-token");
    }
    if lower.starts_with("aiza") && t.len() >= 20 {
        return Some("google-key");
    }
    if t.starts_with("AKIA") && t.len() >= 16 {
        return Some("aws-key");
    }
    if lower.starts_with("xoxb-") || lower.starts_with("xoxp-") || lower.starts_with("xoxa-") {
        return Some("slack-token");
    }
    if lower.starts_with("nvapi-") && t.len() >= 12 {
        return Some("nv-key");
    }
    if lower.starts_with("bearer") && t.len() >= 12 {
        return Some("bearer");
    }
    if t.starts_with("eyJ") && t.len() >= 20 {
        return Some("jwt");
    }
    if lower.contains("begin") && lower.contains("private") && lower.contains("key") {
        return Some("pem");
    }
    if let Some((user, host)) = t.split_once('@') {
        if !user.is_empty() && host.contains('.') && !host.starts_with('.') && !host.ends_with('.') {
            return Some("email");
        }
    }
    None
}

/// Redact PII-ish tokens: keys, bearer, emails, PEM, cloud tokens.
pub fn looks_pii_token(token: &str) -> bool {
    let parts: Vec<&str> = token
        .split(['=', ':', '/', '&', '?', '"', '\''])
        .filter(|p| !p.is_empty())
        .collect();
    for (i, part) in parts.iter().enumerate() {
        if classify_secret_part(part).is_some() {
            return true;
        }
        if is_secret_key_name(part) {
            if parts.get(i + 1).map(|v| v.len() >= 4).unwrap_or(false) {
                return true;
            }
        }
    }
    false
}

fn redact_pem(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("-----BEGIN") {
        out.push_str(&rest[..start]);
        out.push_str("[redacted]");
        if let Some(end_rel) = rest[start..].find("-----END") {
            let after = start + end_rel + "-----END".len();
            if let Some(nl) = rest[after..].find('\n') {
                rest = &rest[after + nl + 1..];
            } else if let Some(dash) = rest[after..].find("-----") {
                rest = &rest[after + dash + 5..];
            } else {
                rest = "";
            }
        } else {
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

pub fn scrub_pii(text: &str) -> String {
    let text = redact_pem(text);
    let mut out = String::new();
    for word in text.split_inclusive([' ', '\n', '\t', ',', ';', '|']) {
        let core = word.trim_end_matches([' ', '\n', '\t', ',', ';', '|']);
        if looks_pii_token(core) {
            let sep = &word[core.len()..];
            out.push_str("[redacted]");
            out.push_str(sep);
        } else {
            out.push_str(word);
        }
    }
    out
}

pub fn redaction_report(text: &str) -> RedactionReport {
    let mut kinds = std::collections::BTreeMap::new();
    let mut redacted = 0usize;
    if text.contains("-----BEGIN") && text.to_ascii_uppercase().contains("PRIVATE KEY") {
        *kinds.entry("pem".to_string()).or_insert(0) += 1;
        redacted += 1;
    }
    for word in text.split([' ', '\n', '\t', ',', ';', '|', '"', '\'']) {
        if looks_pii_token(word) {
            redacted += 1;
            let kind = word
                .split(['=', ':', '/', '&', '?'])
                .find_map(classify_secret_part)
                .unwrap_or("secret-key");
            *kinds.entry(kind.to_string()).or_insert(0) += 1;
        }
    }
    RedactionReport {
        schema: REDACTION_SCHEMA.into(),
        redacted,
        kinds,
        raw_secrets_found: redacted,
        note: "Kind counts only. Raw secrets are never stored in this report.".into(),
    }
}
