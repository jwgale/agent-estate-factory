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

/// Candidate pack. `promoted` is always false when produced from the feed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackManifest {
    pub id: String,
    pub version: u32,
    pub curator: String,
    pub policy: String,
    pub promoted: bool,
    pub source: String,
    pub from_events: usize,
    pub kinds: Vec<String>,
    pub paths: Vec<String>,
    pub agents: Vec<String>,
    pub created_at: String,
    pub note: String,
}

pub fn append_event(dir: &Path, event: &ScrubbedEvent) -> Result<(), FeedError> {
    std::fs::create_dir_all(dir)?;
    let mut ev = event.clone();
    if ev.ts.is_empty() {
        ev.ts = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    }
    let line = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
    let path = dir.join("events.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
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
    for ev in events {
        kinds.insert(ev.kind.clone());
        if let Some(agent) = &ev.agent_id {
            agents.insert(agent.clone());
        }
        if let Some(class) = &ev.object_class {
            paths.insert(class.clone());
        } else if ev.kind.contains("frontier") {
            paths.insert("frontier".into());
        } else if ev.kind.contains("local") {
            paths.insert("local".into());
        } else if ev.kind.contains("proxy") {
            paths.insert("proxy".into());
        }
    }
    PackManifest {
        id: id.to_string(),
        version: 0,
        curator: "jason".into(),
        policy: "manual".into(),
        promoted: false,
        source: "feed".into(),
        from_events: events.len(),
        kinds: kinds.into_iter().collect(),
        paths: paths.into_iter().collect(),
        agents: agents.into_iter().collect(),
        created_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        note: "Candidate only. Not in estate.enrich_packs until Jason adds the id by hand. Feed never auto-promotes.".into(),
    }
}

pub fn write_drop_pack(drop_dir: &Path, pack: &PackManifest) -> Result<PathBuf, FeedError> {
    if pack.promoted {
        return Err(FeedError::NoAutoPromote);
    }
    if pack.policy != "manual" || pack.curator != "jason" {
        return Err(FeedError::NoAutoPromote);
    }
    std::fs::create_dir_all(drop_dir)?;
    let path = drop_dir.join(format!("{}.pack.json", pack.id));
    std::fs::write(&path, serde_json::to_string_pretty(pack).unwrap_or_default())?;
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

/// Always fails. The estate file is the only promotion path.
pub fn refuse_promote(_id: &str) -> Result<(), FeedError> {
    Err(FeedError::NoAutoPromote)
}

pub fn materialize_from_feed(
    feed_dir: &Path,
    drop_dir: &Path,
    id: &str,
) -> Result<(PackManifest, PathBuf), FeedError> {
    let events = read_events(feed_dir)?;
    let pack = pack_from_events(id, &events);
    let path = write_drop_pack(drop_dir, &pack)?;
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
        assert!(pack.paths.iter().any(|p| p == "local"));
        assert!(pack.paths.iter().any(|p| p == "frontier"));
        assert!(path.ends_with("overnight-traces.pack.json"));
        assert!(refuse_promote(&pack.id).is_err());
        let listed = list_drop_packs(&drop).unwrap();
        assert_eq!(listed.len(), 1);
        let _ = std::fs::remove_dir_all(&feed);
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
}
