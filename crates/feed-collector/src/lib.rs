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
    #[error("pack schema must be cell-one.pack.v0 (got {0})")]
    BadSchema(String),
    #[error("pack host_class '{0}' must be consumer-nvidia|apple-silicon|rented-nvidia|any")]
    BadHostClass(String),
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
    let class = ev.object_class.as_deref().unwrap_or("");
    if class == "frontier" || ev.kind.contains("frontier") {
        "frontier"
    } else if class == "local" || ev.kind.contains("local") {
        "local"
    } else if class == "proxy" || ev.kind.contains("proxy") {
        "proxy"
    } else {
        "other"
    }
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

pub fn append_event(dir: &Path, event: &ScrubbedEvent) -> Result<(), FeedError> {
    std::fs::create_dir_all(dir)?;
    let mut ev = event.clone();
    if ev.ts.is_empty() {
        ev.ts = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    }
    refuse_event(&ev)?;
    let line = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
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
    let text = std::fs::read_to_string(&path)?;
    let cursor = serde_json::from_str(&text)
        .map_err(|e| FeedError::Parse(format!("feed-cursor.json: {e}")))?;
    Ok(Some(cursor))
}

pub fn write_cursor(dir: &Path, cursor: &FeedCursor) -> Result<PathBuf, FeedError> {
    std::fs::create_dir_all(dir)?;
    let path = cursor_path(dir);
    std::fs::write(&path, serde_json::to_string_pretty(cursor).unwrap_or_default())?;
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
    writeln!(
        file,
        "{}",
        serde_json::to_string(audit).unwrap_or_default()
    )?;
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
    if pack.schema != "cell-one.pack.v0" {
        return Err(FeedError::BadSchema(pack.schema.clone()));
    }
    if !estate_schema::is_host_class(&pack.host_class) {
        return Err(FeedError::BadHostClass(pack.host_class.clone()));
    }
    Ok(())
}

pub fn write_drop_pack(drop_dir: &Path, pack: &PackManifest) -> Result<PathBuf, FeedError> {
    refuse_pack(pack)?;
    std::fs::create_dir_all(drop_dir)?;
    let path = drop_dir.join(format!("{}.pack.json", pack.id));
    std::fs::write(&path, serde_json::to_string_pretty(pack).unwrap_or_default())?;
    let _ = write_pack_index(drop_dir);
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
                "- `{}.pack.json` events={} frontier={} local={} proxy={} promoted={} host_class={} schema={}\n",
                pack.id,
                pack.from_events,
                pack.path_counts.frontier,
                pack.path_counts.local,
                pack.path_counts.proxy,
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
    refuse_pack_id(id)?;
    let mut pack = load_pack(drop_dir, id)?;
    refuse_pack(&pack)?;
    if pack.promoted {
        return Err(FeedError::NoAutoPromote);
    }
    pack.promoted = false;
    pack.policy = "manual".into();
    pack.curator = "jason".into();
    let estate_bound = estate_pack_ids.iter().any(|p| p == &pack.id);
    let imported = ImportedPack {
        pack: pack.clone(),
        estate_bound,
        imported_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        source_path: drop_dir.join(format!("{id}.pack.json")).display().to_string(),
    };
    std::fs::create_dir_all(accepted_dir)?;
    let path = accepted_dir.join(format!("{id}.pack.json"));
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&imported).unwrap_or_default(),
    )?;
    let _ = append_import_audit(
        accepted_dir,
        &ImportAudit {
            imported_at: imported.imported_at.clone(),
            pack_id: imported.pack.id.clone(),
            estate_bound: imported.estate_bound,
            source_path: imported.source_path.clone(),
            promoted: imported.pack.promoted,
        },
    );
    let _ = write_pack_index(drop_dir);
    Ok((imported, path))
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
    fn write_pack_index_lists_candidates() {
        let feed = tmp();
        let drop = feed.join("drop");
        materialize_from_feed(&feed, &drop, "overnight-traces").unwrap();
        let index = drop.join("INDEX.md");
        assert!(index.is_file());
        let text = std::fs::read_to_string(&index).unwrap();
        assert!(text.contains("overnight-traces"));
        assert!(text.contains("promoted=false"));
        let _ = std::fs::remove_dir_all(&feed);
    }
}
