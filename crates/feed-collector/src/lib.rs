//! Reserved one-way feed seam. Append scrubbed traces. Never auto-promote.
//!
//! Day 61–90: traces from frontier + local can be *materialized* into a pack
//! manifest in the enrich-packs drop zone. Jason still has to edit the estate
//! by hand. `promote` always fails.

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

pub fn refuse_raw_secrets(text: &str) -> Result<(), FeedError> {
    if redaction_report(text).raw_secrets_found > 0 {
        return Err(FeedError::RawSecret);
    }
    Ok(())
}

fn refuse_empty_blob(body: &str) -> Result<(), FeedError> {
    if body.trim().is_empty() {
        return Err(FeedError::Parse("serialize: empty".into()));
    }
    Ok(())
}

fn to_json<T: Serialize>(value: &T) -> Result<String, FeedError> {
    let line = serde_json::to_string(value).map_err(|e| FeedError::Parse(format!("serialize: {e}")))?;
    refuse_empty_blob(&line)?;
    Ok(line)
}

fn to_pretty_json<T: Serialize>(value: &T) -> Result<String, FeedError> {
    let body =
        serde_json::to_string_pretty(value).map_err(|e| FeedError::Parse(format!("serialize: {e}")))?;
    refuse_empty_blob(&body)?;
    Ok(body)
}

/// Journal / audit line. Inventing `"{}"` on serialize failure is refuse.
fn to_jsonl_line<T: Serialize>(value: &T) -> Result<String, FeedError> {
    let line = to_json(value)?;
    if line.trim() == "{}" {
        return Err(FeedError::Parse("serialize: empty object".into()));
    }
    Ok(line)
}

/// Kind counts only. Refuses if the serialized report itself contains a raw secret.
pub fn write_redaction_report(path: &Path, report: &RedactionReport) -> Result<(), FeedError> {
    let blob = to_pretty_json(report)?;
    refuse_raw_secrets(&blob)?;
    for kind in report.kinds.keys() {
        if looks_pii_token(kind) {
            return Err(FeedError::RawSecret);
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, blob)?;
    Ok(())
}

pub fn is_pack_schema(schema: &str) -> bool {
    schema == "cell-one.pack.v0" || schema == "cell-one.specialist-pack.v0"
}

pub fn append_event(dir: &Path, event: &ScrubbedEvent) -> Result<(), FeedError> {
    std::fs::create_dir_all(dir)?;
    let mut ev = event.clone();
    if ev.ts.is_empty() {
        ev.ts = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    }
    if let Some(note) = ev.note.take() {
        ev.note = Some(scrub_pii(&note));
    }
    refuse_event(&ev)?;
    let line = to_jsonl_line(&ev)?;
    let path = dir.join("events.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")?;
    let events = read_events(dir)?;
    write_cursor(dir, &cursor_from_events(&events, None))?;
    Ok(())
}

pub fn cursor_path(dir: &Path) -> PathBuf {
    dir.join("feed-cursor.json")
}

pub fn load_cursor(dir: &Path) -> Result<Option<FeedCursor>, FeedError> {
    let path = cursor_path(dir);
    if !path.exists() {
        return Ok(None);
    }
    if !path.is_file() {
        return Err(FeedError::Parse(format!(
            "feed-cursor.json: {} is not a file",
            path.display()
        )));
    }
    let text = std::fs::read_to_string(&path)?;
    let cursor = serde_json::from_str(&text)
        .map_err(|e| FeedError::Parse(format!("feed-cursor.json: {e}")))?;
    Ok(Some(cursor))
}

pub fn write_cursor(dir: &Path, cursor: &FeedCursor) -> Result<PathBuf, FeedError> {
    std::fs::create_dir_all(dir)?;
    let path = cursor_path(dir);
    let body = to_pretty_json(cursor)?;
    std::fs::write(&path, body)?;
    Ok(path)
}

pub fn cursor_from_events(events: &[ScrubbedEvent], packed_id: Option<&str>) -> FeedCursor {
    FeedCursor {
        schema: default_cursor_schema(),
        events: events.len(),
        last_ts: events.last().map(|e| e.ts.clone()),
        last_kind: events.last().map(|e| e.kind.clone()),
        packed_id: packed_id.map(|s| s.to_string()),
        updated_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    }
}

pub fn append_import_audit(accepted_dir: &Path, audit: &ImportAudit) -> Result<PathBuf, FeedError> {
    std::fs::create_dir_all(accepted_dir)?;
    let path = accepted_dir.join("import-audit.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let line = to_jsonl_line(audit)?;
    writeln!(file, "{line}")?;
    Ok(path)
}

pub fn read_events(dir: &Path) -> Result<Vec<ScrubbedEvent>, FeedError> {
    let path = dir.join("events.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let ev: ScrubbedEvent = serde_json::from_str(line)
            .map_err(|e| FeedError::Parse(format!("events.jsonl line {}: {e}", i + 1)))?;
        out.push(ev);
    }
    Ok(out)
}

pub fn pack_from_events(id: &str, events: &[ScrubbedEvent]) -> PackManifest {
    let mut kinds = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut agents = BTreeSet::new();
    let mut path_counts = PathCounts::default();
    for ev in events {
        kinds.insert(ev.kind.clone());
        if let Some(agent) = &ev.agent_id {
            agents.insert(agent.clone());
        }
        match classify_path(ev) {
            "frontier" => {
                paths.insert("frontier".into());
                path_counts.frontier += 1;
            }
            "local" => {
                paths.insert("local".into());
                path_counts.local += 1;
            }
            "proxy" => {
                paths.insert("proxy".into());
                path_counts.proxy += 1;
            }
            other => {
                if let Some(class) = &ev.object_class {
                    paths.insert(class.clone());
                }
                let _ = other;
                path_counts.other += 1;
            }
        }
    }
    PackManifest {
        id: id.to_string(),
        version: 0,
        schema: default_pack_schema(),
        curator: "jason".into(),
        policy: "manual".into(),
        promoted: false,
        source: "feed".into(),
        from_events: events.len(),
        kinds: kinds.into_iter().collect(),
        paths: paths.into_iter().collect(),
        agents: agents.into_iter().collect(),
        host_class: default_host_class(),
        job: Some("policy-precheck".into()),
        adapter: None,
        path_counts,
        source_paths: vec!["feed/events.jsonl".into()],
        model_hint: None,
        source_drivers: source_drivers_from_events(events),
        host_class_affinity: Some(default_host_class()),
        created_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        note: "Candidate only. Import is explicit apply. Jason still adds the id to estate.enrich_packs by hand. Feed never auto-promotes.".into(),
    }
}

pub fn refuse_pack_id(id: &str) -> Result<(), FeedError> {
    if !estate_schema::is_slug(id) {
        return Err(FeedError::BadId(id.to_string()));
    }
    if estate_schema::contains_sku(id) {
        return Err(FeedError::SkuBanned(id.to_string()));
    }
    Ok(())
}

pub fn refuse_pack(pack: &PackManifest) -> Result<(), FeedError> {
    refuse_pack_id(&pack.id)?;
    if pack.promoted {
        return Err(FeedError::NoAutoPromote);
    }
    if pack.policy != "manual" || pack.curator != "jason" {
        return Err(FeedError::NoAutoPromote);
    }
    if !is_pack_schema(&pack.schema) {
        return Err(FeedError::BadSchema(pack.schema.clone()));
    }
    if !estate_schema::is_host_class(&pack.host_class) {
        return Err(FeedError::BadHostClass(pack.host_class.clone()));
    }
    if let Some(hint) = pack.model_hint.as_deref() {
        if !hint.is_empty()
            && (!estate_schema::is_slug(hint) || estate_schema::contains_sku(hint))
        {
            return Err(FeedError::BadModelHint(hint.to_string()));
        }
    }
    if let Some(affinity) = pack.host_class_affinity.as_deref() {
        if !affinity.is_empty() && !estate_schema::is_host_class(affinity) {
            return Err(FeedError::BadHostClass(affinity.to_string()));
        }
    }
    refuse_source_drivers(pack)?;
    for path in &pack.source_paths {
        if path.is_empty() {
            continue;
        }
        if path.starts_with('/') || path.contains("..") || estate_schema::contains_sku(path) {
            return Err(FeedError::BadSourcePath(path.clone()));
        }
    }
    let blob = to_json(pack)?;
    refuse_raw_secrets(&blob)?;
    refuse_raw_secrets(&pack.note)?;
    Ok(())
}

fn refuse_source_drivers(pack: &PackManifest) -> Result<(), FeedError> {
    let mut prev = "";
    for driver in &pack.source_drivers {
        if driver != "frontier" && driver != "local" {
            return Err(FeedError::BadSourceDriver(format!(
                "{driver} must be frontier or local"
            )));
        }
        if driver.as_str() <= prev {
            return Err(FeedError::BadSourceDriver(format!(
                "{driver} out of order or duplicated"
            )));
        }
        prev = driver.as_str();
    }
    if pack.path_counts.frontier > 0 && !pack.source_drivers.iter().any(|d| d == "frontier") {
        return Err(FeedError::BadSourceDriver(
            "frontier events are missing from source_drivers".into(),
        ));
    }
    if pack.path_counts.local > 0 && !pack.source_drivers.iter().any(|d| d == "local") {
        return Err(FeedError::BadSourceDriver(
            "local events are missing from source_drivers".into(),
        ));
    }
    if pack.source_drivers.iter().any(|d| d == "frontier") && pack.path_counts.frontier == 0 {
        return Err(FeedError::BadSourceDriver(
            "frontier tag has no frontier events".into(),
        ));
    }
    if pack.source_drivers.iter().any(|d| d == "local") && pack.path_counts.local == 0 {
        return Err(FeedError::BadSourceDriver(
            "local tag has no local events".into(),
        ));
    }
    Ok(())
}

pub fn write_drop_pack(drop_dir: &Path, pack: &PackManifest) -> Result<PathBuf, FeedError> {
    refuse_pack(pack)?;
    std::fs::create_dir_all(drop_dir)?;
    let path = drop_dir.join(format!("{}.pack.json", pack.id));
    std::fs::write(&path, to_pretty_json(pack)?)?;
    write_pack_index(drop_dir)?;
    Ok(path)
}

pub fn write_pack_index(drop_dir: &Path) -> Result<PathBuf, FeedError> {
    std::fs::create_dir_all(drop_dir)?;
    let packs = list_drop_packs(drop_dir)?;
    let mut md = String::from(
        "# Pack drop zone\n\nCandidates only. curator=jason policy=manual. Import is explicit apply. Promote fails. Pack ids must not encode a hardware SKU.\n\n",
    );
    if packs.is_empty() {
        md.push_str("(no candidate packs)\n");
    } else {
        for pack in &packs {
            md.push_str(&format!(
                "- `{}.pack.json` events={} frontier={} local={} proxy={} drivers={} promoted={} host_class={} schema={}\n",
                pack.id,
                pack.from_events,
                pack.path_counts.frontier,
                pack.path_counts.local,
                pack.path_counts.proxy,
                if pack.source_drivers.is_empty() {
                    "-".to_string()
                } else {
                    pack.source_drivers.join(",")
                },
                pack.promoted,
                pack.host_class,
                pack.schema
            ));
        }
    }
    let path = drop_dir.join("INDEX.md");
    std::fs::write(&path, md)?;
    Ok(path)
}

pub fn list_drop_packs(drop_dir: &Path) -> Result<Vec<PackManifest>, FeedError> {
    if !drop_dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut names: Vec<PathBuf> = std::fs::read_dir(drop_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("json")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|n| n.ends_with(".pack.json"))
                    .unwrap_or(false)
        })
        .collect();
    names.sort();
    for path in names {
        let text = std::fs::read_to_string(&path)?;
        let pack: PackManifest = serde_json::from_str(&text)
            .map_err(|e| FeedError::Parse(format!("{}: {e}", path.display())))?;
        out.push(pack);
    }
    Ok(out)
}

/// Always fails. Auto-promote is locked off. Use `import_pack` (explicit apply).
pub fn refuse_promote(_id: &str) -> Result<(), FeedError> {
    Err(FeedError::NoAutoPromote)
}

pub fn load_pack(dir: &Path, id: &str) -> Result<PackManifest, FeedError> {
    let path = dir.join(format!("{id}.pack.json"));
    if !path.exists() {
        return Err(FeedError::Parse(format!("missing pack {}", path.display())));
    }
    let text = std::fs::read_to_string(&path)?;
    serde_json::from_str(&text).map_err(|e| FeedError::Parse(format!("{}: {e}", path.display())))
}

/// Explicit Feed→Control import. Copies the candidate into `accepted/`.
/// Does not rewrite the estate file. `estate_bound` is true only if Jason
/// already listed the id on `enrich_packs.packs`.
pub fn import_pack(
    drop_dir: &Path,
    accepted_dir: &Path,
    id: &str,
    estate_pack_ids: &[String],
) -> Result<(ImportedPack, PathBuf), FeedError> {
    import_pack_for(
        drop_dir,
        accepted_dir,
        id,
        estate_pack_ids,
        LOCKED_CURATOR,
        LOCKED_CURATOR,
    )
}

/// Refuse curator / id / raw secret / pack schema without writing accepted files.
pub fn refuse_import_pack(
    drop_dir: &Path,
    id: &str,
    curator: &str,
    estate_curator: &str,
) -> Result<PackManifest, FeedError> {
    refuse_curator(curator, estate_curator)?;
    refuse_pack_id(id)?;
    let source = drop_dir.join(format!("{id}.pack.json"));
    let raw = std::fs::read_to_string(&source)
        .map_err(|_| FeedError::MissingPack(id.to_string()))?;
    refuse_raw_secrets(&raw)?;
    let pack = load_pack(drop_dir, id)?;
    refuse_pack(&pack)?;
    if pack.promoted {
        return Err(FeedError::NoAutoPromote);
    }
    Ok(pack)
}

/// Explicit Feed→Control import gated on the locked curator.
pub fn import_pack_for(
    drop_dir: &Path,
    accepted_dir: &Path,
    id: &str,
    estate_pack_ids: &[String],
    curator: &str,
    estate_curator: &str,
) -> Result<(ImportedPack, PathBuf), FeedError> {
    let mut pack = refuse_import_pack(drop_dir, id, curator, estate_curator)?;
    let source = drop_dir.join(format!("{id}.pack.json"));
    let raw = std::fs::read_to_string(&source)
        .map_err(|_| FeedError::MissingPack(id.to_string()))?;
    pack.promoted = false;
    pack.policy = "manual".into();
    pack.curator = "jason".into();
    pack.note = scrub_pii(&pack.note);
    let report = redaction_report(&raw);
    let estate_bound = estate_pack_ids.iter().any(|p| p == &pack.id);
    let imported = ImportedPack {
        pack: pack.clone(),
        estate_bound,
        imported_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        source_path: source.display().to_string(),
    };
    std::fs::create_dir_all(accepted_dir)?;
    let path = accepted_dir.join(format!("{id}.pack.json"));
    let written = to_pretty_json(&imported)?;
    refuse_raw_secrets(&written)?;
    std::fs::write(&path, written)?;
    write_redaction_report(
        &accepted_dir.join(format!("{id}.redaction.json")),
        &report,
    )?;
    append_import_audit(
        accepted_dir,
        &ImportAudit {
            imported_at: imported.imported_at.clone(),
            pack_id: imported.pack.id.clone(),
            estate_bound: imported.estate_bound,
            source_path: imported.source_path.clone(),
            promoted: imported.pack.promoted,
        },
    )?;
    write_pack_index(drop_dir)?;
    Ok((imported, path))
}

/// Open enrich proposals. Never auto-applied.
pub fn list_open_proposals(proposed_dir: &Path) -> Result<Vec<String>, FeedError> {
    if !proposed_dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    let mut names: Vec<PathBuf> = std::fs::read_dir(proposed_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|n| n.ends_with(".proposal.json"))
                .unwrap_or(false)
        })
        .collect();
    names.sort();
    for path in names {
        let stem = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .trim_end_matches(".proposal.json");
        if !stem.is_empty() {
            ids.push(stem.to_string());
        }
    }
    Ok(ids)
}

pub const PROPOSAL_SCHEMA: &str = "cell-one.enrich-proposal.v0";

/// Diff Jason reviews. Never applied by the factory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnrichDiff {
    pub pack_id: String,
    pub estate_bound: bool,
    pub would_add_to_estate: bool,
    pub kinds: Vec<String>,
    pub agents: Vec<String>,
    pub paths: Vec<String>,
    pub path_counts: PathCounts,
    /// Copied from the pack. Not applied.
    #[serde(default)]
    pub source_drivers: Vec<String>,
    pub note: String,
}

/// Proposal pack. `auto_apply` is always false. Not estate SoT.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnrichProposal {
    #[serde(default = "default_proposal_schema")]
    pub schema: String,
    pub id: String,
    pub curator: String,
    pub policy: String,
    pub auto_apply: bool,
    pub source_pack: String,
    pub estate_name: String,
    pub estate_hash: String,
    pub current_estate_packs: Vec<String>,
    pub proposed_estate_packs: Vec<String>,
    pub diff: EnrichDiff,
    pub created_at: String,
    pub note: String,
}

fn default_proposal_schema() -> String {
    PROPOSAL_SCHEMA.into()
}

/// Always fails. Proposals are never applied.
pub fn refuse_apply_proposal(_id: &str) -> Result<(), FeedError> {
    Err(FeedError::NoAutoApply)
}

fn load_pack_loose(dir: &Path, id: &str) -> Result<PackManifest, FeedError> {
    let path = dir.join(format!("{id}.pack.json"));
    if !path.exists() {
        return Err(FeedError::MissingPack(id.to_string()));
    }
    let text = std::fs::read_to_string(&path)?;
    if let Ok(pack) = serde_json::from_str::<PackManifest>(&text) {
        if !pack.id.is_empty() {
            return Ok(pack);
        }
    }
    if let Ok(imported) = serde_json::from_str::<ImportedPack>(&text) {
        return Ok(imported.pack);
    }
    Err(FeedError::Parse(format!(
        "{}: not a pack or imported pack",
        path.display()
    )))
}

pub fn render_proposal(proposal: &EnrichProposal) -> String {
    let mut out = String::from("Enrich proposal (curator Jason)\n");
    out.push_str("==============================\n");
    out.push_str(&format!("schema: {}\n", proposal.schema));
    out.push_str(&format!("id: {}\n", proposal.id));
    out.push_str(&format!("auto_apply: {}\n", proposal.auto_apply));
    out.push_str(&format!("source_pack: {}\n", proposal.source_pack));
    out.push_str(&format!(
        "estate: {} ({})\n",
        proposal.estate_name, proposal.estate_hash
    ));
    out.push_str(&format!(
        "current packs: {}\n",
        if proposal.current_estate_packs.is_empty() {
            "(none)".into()
        } else {
            proposal.current_estate_packs.join(", ")
        }
    ));
    out.push_str(&format!(
        "proposed packs: {}\n\n",
        if proposal.proposed_estate_packs.is_empty() {
            "(none)".into()
        } else {
            proposal.proposed_estate_packs.join(", ")
        }
    ));
    out.push_str("Diff summary\n------------\n");
    out.push_str(&format!("  pack: {}\n", proposal.diff.pack_id));
    out.push_str(&format!("  estate_bound: {}\n", proposal.diff.estate_bound));
    out.push_str(&format!(
        "  would_add_to_estate: {}\n",
        proposal.diff.would_add_to_estate
    ));
    out.push_str(&format!(
        "  kinds: {}\n",
        if proposal.diff.kinds.is_empty() {
            "(none)".into()
        } else {
            proposal.diff.kinds.join(", ")
        }
    ));
    out.push_str(&format!(
        "  agents: {}\n",
        if proposal.diff.agents.is_empty() {
            "(none)".into()
        } else {
            proposal.diff.agents.join(", ")
        }
    ));
    out.push_str(&format!(
        "  paths: {}\n",
        if proposal.diff.paths.is_empty() {
            "(none)".into()
        } else {
            proposal.diff.paths.join(", ")
        }
    ));
    out.push_str(&format!(
        "  path_counts: frontier={} local={} proxy={} other={}\n",
        proposal.diff.path_counts.frontier,
        proposal.diff.path_counts.local,
        proposal.diff.path_counts.proxy,
        proposal.diff.path_counts.other
    ));
    out.push_str(&format!(
        "  source_drivers: {}\n",
        if proposal.diff.source_drivers.is_empty() {
            "(none)".into()
        } else {
            proposal.diff.source_drivers.join(", ")
        }
    ));
    out.push_str(&format!("\n{}\n", proposal.note));
    out.push_str(&format!("{}\n", proposal.diff.note));
    out
}

pub fn write_proposal_index(proposed_dir: &Path) -> Result<PathBuf, FeedError> {
    std::fs::create_dir_all(proposed_dir)?;
    let mut names: Vec<PathBuf> = if proposed_dir.exists() {
        std::fs::read_dir(proposed_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|n| n.ends_with(".proposal.json"))
                    .unwrap_or(false)
            })
            .collect()
    } else {
        Vec::new()
    };
    names.sort();
    let mut md = String::from(
        "# Enrich proposals\n\nNever auto-applied. curator=jason policy=manual. Jason reviews the diff and edits `estate.enrich_packs` by hand. The factory will not apply these files.\n\n",
    );
    if names.is_empty() {
        md.push_str("(no proposals)\n");
    } else {
        for path in &names {
            let text = std::fs::read_to_string(path).unwrap_or_default();
            if let Ok(p) = serde_json::from_str::<EnrichProposal>(&text) {
                md.push_str(&format!(
                    "- `{}.proposal.json` estate_bound={} would_add={} auto_apply={} source={}\n",
                    p.id, p.diff.estate_bound, p.diff.would_add_to_estate, p.auto_apply, p.source_pack
                ));
            } else if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                md.push_str(&format!("- `{name}`\n"));
            }
        }
    }
    let path = proposed_dir.join("INDEX.md");
    std::fs::write(&path, md)?;
    Ok(path)
}

/// After an explicit pack import: write a proposal pack. Never applies. Never rewrites the estate.
pub fn propose_enrich(
    drop_dir: &Path,
    accepted_dir: &Path,
    proposed_dir: &Path,
    id: &str,
    estate: &estate_schema::Estate,
) -> Result<(EnrichProposal, PathBuf), FeedError> {
    refuse_pack_id(id)?;
    let pack = match load_pack_loose(accepted_dir, id) {
        Ok(p) => p,
        Err(FeedError::MissingPack(_)) => load_pack_loose(drop_dir, id)?,
        Err(e) => return Err(e),
    };
    refuse_pack(&pack)?;
    if pack.promoted {
        return Err(FeedError::NoAutoPromote);
    }
    let current: Vec<String> = estate
        .enrich_packs
        .packs
        .iter()
        .map(|p| p.id.clone())
        .collect();
    let estate_bound = current.iter().any(|p| p == &pack.id);
    let mut proposed_estate_packs = current.clone();
    if !estate_bound {
        proposed_estate_packs.push(pack.id.clone());
        proposed_estate_packs.sort();
    }
    let proposal = EnrichProposal {
        schema: PROPOSAL_SCHEMA.into(),
        id: pack.id.clone(),
        curator: "jason".into(),
        policy: "manual".into(),
        auto_apply: false,
        source_pack: format!("{}.pack.json", pack.id),
        estate_name: estate.name.clone(),
        estate_hash: estate_schema::estate_hash(estate),
        current_estate_packs: current,
        proposed_estate_packs,
        diff: EnrichDiff {
            pack_id: pack.id.clone(),
            estate_bound,
            would_add_to_estate: !estate_bound,
            kinds: pack.kinds.clone(),
            agents: pack.agents.clone(),
            paths: pack.paths.clone(),
            path_counts: pack.path_counts.clone(),
            source_drivers: pack.source_drivers.clone(),
            note: if estate_bound {
                "Already listed on estate.enrich_packs. No estate edit required.".into()
            } else {
                "Jason must add this pack id to estate.enrich_packs by hand. This file is not applied."
                    .into()
            },
        },
        created_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        note: "Proposal only. auto_apply=false. Feed never applies this file. Control will not rewrite the estate.".into(),
    };
    std::fs::create_dir_all(proposed_dir)?;
    let path = proposed_dir.join(format!("{}.proposal.json", pack.id));
    let body = to_pretty_json(&proposal)?;
    std::fs::write(&path, body)?;
    std::fs::write(
        proposed_dir.join(format!("{}.proposal.md", pack.id)),
        render_proposal(&proposal),
    )?;
    let _ = write_proposal_index(proposed_dir);
    Ok((proposal, path))
}

pub fn materialize_from_feed(
    feed_dir: &Path,
    drop_dir: &Path,
    id: &str,
) -> Result<(PackManifest, PathBuf), FeedError> {
    refuse_pack_id(id)?;
    let events = read_events(feed_dir)?;
    let pack = pack_from_events(id, &events);
    let path = write_drop_pack(drop_dir, &pack)?;
    write_cursor(feed_dir, &cursor_from_events(&events, Some(id)))?;
    Ok((pack, path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("cell-one-feed-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    struct Boom;
    impl Serialize for Boom {
        fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
            use serde::ser::Error;
            Err(S::Error::custom("boom"))
        }
    }

    #[test]
    fn serialize_helpers_refuse_boom_empty_and_object() {
        let boom = to_json(&Boom).unwrap_err();
        assert!(boom.to_string().contains("serialize"), "{boom}");
        assert!(to_pretty_json(&Boom).is_err());
        assert!(to_jsonl_line(&Boom).is_err());
        assert!(refuse_empty_blob("").is_err());
        assert!(refuse_empty_blob("  \n").is_err());
        let empty_obj = to_jsonl_line(&serde_json::json!({})).unwrap_err();
        assert!(
            empty_obj.to_string().contains("empty"),
            "{empty_obj}"
        );
    }

    #[test]
    fn append_event_never_writes_empty_object_line() {
        let dir = tmp();
        append_event(
            &dir,
            &ScrubbedEvent {
                kind: "proxy.tool".into(),
                agent_id: Some("horizon".into()),
                decision: Some("allow".into()),
                object_class: Some("tool".into()),
                note: None,
                ts: String::new(),
            },
        )
        .unwrap();
        let text = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert!(!text.trim().is_empty());
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            assert_ne!(line, "{}", "append_event must not invent empty junk");
            let _: serde_json::Value = serde_json::from_str(line).unwrap();
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn appends_jsonl() {
        let dir = tmp();
        append_event(
            &dir,
            &ScrubbedEvent {
                kind: "proxy.tool".into(),
                agent_id: Some("horizon".into()),
                decision: Some("deny".into()),
                object_class: Some("tool".into()),
                note: None,
                ts: String::new(),
            },
        )
        .unwrap();
        let text = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert!(text.contains("proxy.tool"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pack_from_frontier_and_local_is_not_promoted() {
        let feed = tmp();
        append_event(
            &feed,
            &ScrubbedEvent {
                kind: "model.local.precheck".into(),
                agent_id: Some("research".into()),
                decision: Some("allow".into()),
                object_class: Some("local".into()),
                note: Some("job=policy-precheck".into()),
                ts: String::new(),
            },
        )
        .unwrap();
        append_event(
            &feed,
            &ScrubbedEvent {
                kind: "model.frontier.complete".into(),
                agent_id: Some("horizon".into()),
                decision: Some("allow".into()),
                object_class: Some("frontier".into()),
                note: Some("bytes=4".into()),
                ts: String::new(),
            },
        )
        .unwrap();
        let drop = feed.join("drop");
        let (pack, path) = materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
        assert!(!pack.promoted);
        assert_eq!(pack.curator, "jason");
        assert_eq!(pack.policy, "manual");
        assert_eq!(pack.from_events, 2);
        assert_eq!(pack.schema, "cell-one.pack.v0");
        assert_eq!(pack.host_class, "any");
        assert!(pack.paths.iter().any(|p| p == "local"));
        assert!(pack.paths.iter().any(|p| p == "frontier"));
        assert_eq!(pack.path_counts.local, 1);
        assert_eq!(pack.path_counts.frontier, 1);
        assert_eq!(pack.source_drivers, vec!["frontier".to_string(), "local".to_string()]);
        assert!(!pack.promoted);
        assert!(feed.join("feed-cursor.json").is_file());
        assert!(path.ends_with("overnight-traces.pack.json"));
        assert!(refuse_promote(&pack.id).is_err());
        let listed = list_drop_packs(&drop).unwrap();
        assert_eq!(listed.len(), 1);
        let accepted = feed.join("accepted");
        let (imported, _) = import_pack(&drop, &accepted, "overnight-traces", &[]).unwrap();
        assert!(!imported.estate_bound);
        assert!(!imported.pack.promoted);
        assert!(accepted.join("import-audit.jsonl").is_file());
        let _ = std::fs::remove_dir_all(&feed);
    }

    #[test]
    fn local_down_tags_local_and_not_frontier() {
        let pack = pack_from_events(
            "local-down",
            &[ScrubbedEvent {
                kind: "model.local.down".into(),
                agent_id: Some("research".into()),
                decision: Some("deny".into()),
                object_class: Some("local".into()),
                note: Some("fail-closed; no frontier fallback".into()),
                ts: String::new(),
            }],
        );
        assert_eq!(pack.source_drivers, vec!["local".to_string()]);
        assert_eq!(
            classify_path(&ScrubbedEvent {
                kind: "model.frontier.complete".into(),
                agent_id: Some("research".into()),
                decision: Some("deny".into()),
                object_class: Some("local".into()),
                note: Some("no frontier fallback".into()),
                ts: String::new(),
            }),
            "local"
        );
        assert_eq!(pack.path_counts.frontier, 0);
        assert_eq!(pack.path_counts.local, 1);
        assert!(!pack.promoted);
        assert!(refuse_pack(&pack).is_ok());
    }

    #[test]
    fn source_driver_tag_must_match_counts_and_stay_unpromoted() {
        let mut pack = pack_from_events("local-only", &[]);
        assert!(pack.source_drivers.is_empty());
        assert!(!pack.promoted);
        pack.path_counts.local = 1;
        let missing = refuse_pack(&pack).unwrap_err();
        assert!(
            missing.to_string().contains("source-driver"),
            "{missing}"
        );
        pack.source_drivers = vec!["local".into()];
        refuse_pack(&pack).unwrap();
        pack.source_drivers = vec!["ollama".into()];
        assert!(refuse_pack(&pack).is_err());
        pack.source_drivers = vec!["local".into(), "frontier".into()];
        pack.path_counts.frontier = 1;
        assert!(refuse_pack(&pack).is_err(), "order must be frontier then local");
        pack.promoted = true;
        pack.source_drivers = vec!["frontier".into(), "local".into()];
        assert!(matches!(refuse_pack(&pack).unwrap_err(), FeedError::NoAutoPromote));
    }

    #[test]
    fn append_refuses_sku_in_event_and_writes_cursor() {
        let dir = tmp();
        let err = append_event(
            &dir,
            &ScrubbedEvent {
                kind: "model.local.5090".into(),
                agent_id: Some("research".into()),
                decision: Some("allow".into()),
                object_class: Some("local".into()),
                note: None,
                ts: String::new(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, FeedError::SkuBanned(_)));
        append_event(
            &dir,
            &ScrubbedEvent {
                kind: "proxy.tool".into(),
                agent_id: Some("horizon".into()),
                decision: Some("deny".into()),
                object_class: Some("tool".into()),
                note: None,
                ts: String::new(),
            },
        )
        .unwrap();
        let cursor = load_cursor(&dir).unwrap().expect("cursor");
        assert_eq!(cursor.schema, "cell-one.feed-cursor.v0");
        assert_eq!(cursor.events, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_refuses_promoted_flag() {
        let dir = tmp();
        let mut pack = pack_from_events("x", &[]);
        pack.promoted = true;
        assert!(matches!(
            write_drop_pack(&dir, &pack),
            Err(FeedError::NoAutoPromote)
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pack_id_refuses_hardware_sku() {
        let dir = tmp();
        let err = materialize_from_feed(&dir.join("feed"), &dir.join("drop"), "local-5090")
            .unwrap_err();
        assert!(matches!(err, FeedError::SkuBanned(_)));
        let mut pack = pack_from_events("ok-pack", &[]);
        pack.host_class = "not-a-host".into();
        assert!(matches!(
            write_drop_pack(&dir, &pack),
            Err(FeedError::BadHostClass(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scrub_pii_redacts_keys_bearer_and_email() {
        let raw = "key=sk-abcdefghijklmnopqrstuv xai-abcdefghijklmnopqrstuv bearer-ABCDEF123456 jason@example.com keep";
        let scrubbed = scrub_pii(raw);
        assert!(!scrubbed.contains("sk-"));
        assert!(!scrubbed.contains("xai-"));
        assert!(!scrubbed.contains("jason@example.com"));
        assert!(!scrubbed.contains("bearer-ABCDEF123456"));
        assert!(scrubbed.contains("[redacted]"));
        assert!(scrubbed.contains("keep"));
    }

    #[test]
    fn append_event_scrubs_note() {
        let dir = tmp();
        append_event(
            &dir,
            &ScrubbedEvent {
                kind: "proxy.tool".into(),
                agent_id: Some("horizon".into()),
                decision: Some("deny".into()),
                object_class: Some("tool".into()),
                note: Some("contact jason@example.com key=sk-abcdefghijklmnopqrstuv".into()),
                ts: String::new(),
            },
        )
        .unwrap();
        let text = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert!(!text.contains("sk-"));
        assert!(!text.contains("jason@example.com"));
        assert!(text.contains("[redacted]"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_pack_index_lists_candidates() {
        let feed = tmp();
        let drop = feed.join("drop");
        materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
        let index = drop.join("INDEX.md");
        assert!(index.is_file());
        let text = std::fs::read_to_string(&index).unwrap();
        assert!(text.contains("overnight-traces"));
        assert!(text.contains("promoted=false"));
        assert!(text.contains("drivers=-"), "{text}");
        assert!(
            !text.contains("drivers=frontier"),
            "empty pack must not invent a source driver: {text}"
        );
        let _ = std::fs::remove_dir_all(&feed);
    }

    #[test]
    fn index_lists_source_drivers_when_present_and_round_trips() {
        let feed = tmp();
        append_event(
            &feed,
            &ScrubbedEvent {
                kind: "model.local.precheck".into(),
                agent_id: Some("research".into()),
                decision: Some("allow".into()),
                object_class: Some("local".into()),
                note: None,
                ts: String::new(),
            },
        )
        .unwrap();
        append_event(
            &feed,
            &ScrubbedEvent {
                kind: "model.frontier.complete".into(),
                agent_id: Some("horizon".into()),
                decision: Some("allow".into()),
                object_class: Some("frontier".into()),
                note: Some("bytes=4".into()),
                ts: String::new(),
            },
        )
        .unwrap();
        let drop = feed.join("drop");
        materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
        let index = std::fs::read_to_string(drop.join("INDEX.md")).unwrap();
        assert!(index.contains("drivers=frontier,local"), "{index}");
        assert!(index.contains("promoted=false"), "{index}");
        let listed = list_drop_packs(&drop).unwrap();
        assert_eq!(
            listed[0].source_drivers,
            vec!["frontier".to_string(), "local".to_string()]
        );
        let blob = std::fs::read_to_string(drop.join("overnight-traces.pack.json")).unwrap();
        let back: PackManifest = serde_json::from_str(&blob).unwrap();
        assert_eq!(back.source_drivers, listed[0].source_drivers);
        let mut old: serde_json::Value = serde_json::from_str(&blob).unwrap();
        old.as_object_mut().unwrap().remove("source_drivers");
        let legacy: PackManifest = serde_json::from_value(old).unwrap();
        assert!(legacy.source_drivers.is_empty());
        let schema = include_str!("../../../schema/pack.v0.json");
        assert!(schema.contains("\"source_drivers\""));
        let specialist = include_str!("../../../schema/specialist-pack.v0.json");
        assert!(specialist.contains("\"source_drivers\""));
        let proposal = include_str!("../../../schema/enrich-proposal.v0.json");
        assert!(proposal.contains("\"source_drivers\""));
        let _ = std::fs::remove_dir_all(&feed);
    }

    #[test]
    fn propose_enrich_never_auto_applies() {
        let feed = tmp();
        let drop = feed.join("drop");
        let accepted = feed.join("accepted");
        let proposed = feed.join("proposed");
        append_event(
            &feed,
            &ScrubbedEvent {
                kind: "model.local.precheck".into(),
                agent_id: Some("research".into()),
                decision: Some("allow".into()),
                object_class: Some("local".into()),
                note: None,
                ts: String::new(),
            },
        )
        .unwrap();
        materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
        import_pack(&drop, &accepted, "overnight-traces", &[]).unwrap();
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let estate_yaml = serde_json::to_string(&estate.enrich_packs).unwrap();
        let (proposal, path) =
            propose_enrich(&drop, &accepted, &proposed, "overnight-traces", &estate).unwrap();
        assert!(!proposal.auto_apply);
        assert_eq!(proposal.schema, PROPOSAL_SCHEMA);
        assert_eq!(proposal.curator, "jason");
        assert_eq!(proposal.policy, "manual");
        assert!(proposal.diff.would_add_to_estate);
        assert!(!proposal.diff.estate_bound);
        assert!(path.ends_with("overnight-traces.proposal.json"));
        assert!(proposed.join("overnight-traces.proposal.md").is_file());
        assert!(proposed.join("INDEX.md").is_file());
        let blob = std::fs::read_to_string(&path).unwrap();
        assert!(!blob.trim().is_empty(), "propose_enrich must not write empty");
        assert!(blob.contains(PROPOSAL_SCHEMA), "{blob}");
        let md = render_proposal(&proposal);
        assert!(md.contains("auto_apply: false"));
        assert!(md.contains("would_add_to_estate: true"));
        assert!(matches!(
            refuse_apply_proposal("overnight-traces"),
            Err(FeedError::NoAutoApply)
        ));
        assert_eq!(
            estate_yaml,
            serde_json::to_string(&estate.enrich_packs).unwrap()
        );
        let sku = propose_enrich(&drop, &accepted, &proposed, "local-5090", &estate).unwrap_err();
        assert!(matches!(sku, FeedError::SkuBanned(_)));
        let _ = std::fs::remove_dir_all(&feed);
    }

    #[test]
    fn scrub_pii_covers_cloud_tokens_pem_and_password() {
        let raw = "ghp_abcdefghijklmnopqrstuv hf_abcdefghijklmnopqrstuv AKIAIOSFODNN7EXAMPLE xoxb-1234567890-token password=hunter2 -----BEGIN PRIVATE KEY-----\nMIIB\n-----END PRIVATE KEY----- keep";
        let scrubbed = scrub_pii(raw);
        assert!(!scrubbed.contains("ghp_"));
        assert!(!scrubbed.contains("hf_"));
        assert!(!scrubbed.contains("AKIA"));
        assert!(!scrubbed.contains("xoxb-"));
        assert!(!scrubbed.contains("hunter2"));
        assert!(!scrubbed.contains("BEGIN PRIVATE"));
        assert!(scrubbed.contains("[redacted]"));
        assert!(scrubbed.contains("keep"));
        let report = redaction_report(raw);
        assert!(report.raw_secrets_found > 0);
        assert_eq!(report.schema, REDACTION_SCHEMA);
        assert!(refuse_raw_secrets(raw).is_err());
        assert!(refuse_raw_secrets("plain notes only").is_ok());
    }

    #[test]
    fn import_refuses_raw_secrets_and_accepts_specialist_fields() {
        let dir = tmp();
        let drop = dir.join("drop");
        std::fs::create_dir_all(&drop).unwrap();
        let mut pack = pack_from_events("overnight-traces", &[]);
        pack.source_paths = vec!["feed/events.jsonl".into()];
        pack.model_hint = Some("local_slm".into());
        pack.host_class_affinity = Some("any".into());
        pack.schema = "cell-one.specialist-pack.v0".into();
        write_drop_pack(&drop, &pack).unwrap();
        let accepted = dir.join("accepted");
        let (imported, _) = import_pack(&drop, &accepted, "overnight-traces", &[]).unwrap();
        assert_eq!(imported.pack.model_hint.as_deref(), Some("local_slm"));
        assert!(accepted.join("overnight-traces.redaction.json").is_file());
        let accepted_pack =
            std::fs::read_to_string(accepted.join("overnight-traces.pack.json")).unwrap();
        let accepted_report =
            std::fs::read_to_string(accepted.join("overnight-traces.redaction.json")).unwrap();
        assert!(!accepted_pack.contains("sk-"));
        assert!(!accepted_report.contains("sk-"));
        assert!(accepted_report.contains("Kind counts only"));
        let mut dirty = pack.clone();
        dirty.id = "dirty-pack".into();
        dirty.note = "key=sk-abcdefghijklmnopqrstuv".into();
        assert!(matches!(
            write_drop_pack(&drop, &dirty),
            Err(FeedError::RawSecret)
        ));
        let mut sku_hint = pack.clone();
        sku_hint.id = "hint-pack".into();
        sku_hint.model_hint = Some("local-5090".into());
        sku_hint.note = "clean".into();
        assert!(matches!(
            write_drop_pack(&drop, &sku_hint),
            Err(FeedError::BadModelHint(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_refuses_wrong_curator() {
        refuse_curator("jason", "jason").unwrap();
        let err = refuse_curator("not-jason", "jason").unwrap_err();
        assert!(err.to_string().contains("refuse:curator"));
        let dir = tmp();
        let drop = dir.join("drop");
        materialize_from_feed(&dir.join("feed"), &drop, "overnight-traces").unwrap();
        let err = import_pack_for(
            &drop,
            &dir.join("accepted"),
            "overnight-traces",
            &[],
            "robot",
            "jason",
        )
        .unwrap_err();
        assert!(matches!(err, FeedError::WrongCurator { .. }));
        let err = refuse_import_pack(&drop, "overnight-traces", "robot", "jason").unwrap_err();
        assert!(matches!(err, FeedError::WrongCurator { .. }));
        assert!(!dir.join("accepted").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuse_codes_missing_pack_and_no_auto_apply() {
        let dir = tmp();
        let err = import_pack(&dir.join("drop"), &dir.join("accepted"), "no-such", &[])
            .unwrap_err();
        assert!(matches!(err, FeedError::MissingPack(_)));
        assert!(err.to_string().starts_with("refuse:missing-pack"));
        let apply = refuse_apply_proposal("overnight-traces").unwrap_err();
        assert!(matches!(apply, FeedError::NoAutoApply));
        assert!(apply.to_string().starts_with("refuse:no-auto-apply"));
        let raw = refuse_raw_secrets("token=sk-abcdefghijklmnopqrstuv").unwrap_err();
        assert!(raw.to_string().starts_with("refuse:raw-secret"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn redaction_report_json_never_contains_raw_secret() {
        let secret = "sk-abcdefghijklmnopqrstuv";
        let raw = format!("note key={secret} password=hunter2");
        let report = redaction_report(&raw);
        assert!(report.raw_secrets_found > 0);
        let blob = serde_json::to_string_pretty(&report).unwrap();
        assert!(!blob.contains(secret), "{blob}");
        assert!(!blob.contains("hunter2"), "{blob}");
        write_redaction_report(&tmp().join("ok.redaction.json"), &report).unwrap();
        let dir = tmp();
        let drop = dir.join("drop");
        std::fs::create_dir_all(&drop).unwrap();
        std::fs::write(
            drop.join("dirty-pack.pack.json"),
            format!(
                "{{\n  \"id\": \"dirty-pack\",\n  \"version\": 0,\n  \"schema\": \"cell-one.pack.v0\",\n  \"curator\": \"jason\",\n  \"policy\": \"manual\",\n  \"promoted\": false,\n  \"source\": \"feed\",\n  \"from_events\": 0,\n  \"kinds\": [],\n  \"paths\": [],\n  \"agents\": [],\n  \"host_class\": \"any\",\n  \"path_counts\": {{\"kinds\": 0, \"paths\": 0, \"agents\": 0}},\n  \"source_paths\": [],\n  \"created_at\": \"unix:1\",\n  \"note\": \"key={secret}\"\n}}\n"
            ),
        )
        .unwrap();
        let accepted = dir.join("accepted");
        let err = import_pack(&drop, &accepted, "dirty-pack", &[]).unwrap_err();
        assert!(matches!(err, FeedError::RawSecret), "{err}");
        assert!(!accepted.join("dirty-pack.pack.json").exists());
        assert!(!accepted.join("dirty-pack.redaction.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
