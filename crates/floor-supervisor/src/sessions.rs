//! Append-only session journal. Not estate SoT.
//!
//! `.cell/sessions.jsonl` records spawn / unspawn / suspend / resume.
//! Profile dirs under `sessions/` stay disposable. This file survives pause.

use crate::SupervisorError;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

pub const SESSION_JOURNAL: &str = "sessions.jsonl";
pub const SESSION_JOURNAL_SCHEMA: &str = "cell-one.session-journal.v0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionEvent {
    #[serde(default = "default_session_schema")]
    pub schema: String,
    pub ts: String,
    pub unix: u64,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estate_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired_hash: Option<String>,
    pub note: String,
}

fn default_session_schema() -> String {
    SESSION_JOURNAL_SCHEMA.to_string()
}

pub fn session_journal_path(state_dir: &Path) -> std::path::PathBuf {
    state_dir.join(SESSION_JOURNAL)
}

fn now_parts() -> (u64, String) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (unix, format!("unix:{unix}"))
}

pub fn append_session_event(
    state_dir: &Path,
    event: &SessionEvent,
) -> Result<(), SupervisorError> {
    std::fs::create_dir_all(state_dir)?;
    let path = session_journal_path(state_dir);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(
        file,
        "{}",
        serde_json::to_string(event).unwrap_or_default()
    )?;
    Ok(())
}

pub fn journal_session(
    state_dir: &Path,
    action: &str,
    agent_id: Option<&str>,
    estate_name: Option<&str>,
    desired_hash: Option<&str>,
    note: &str,
) -> Result<(), SupervisorError> {
    let (unix, ts) = now_parts();
    append_session_event(
        state_dir,
        &SessionEvent {
            schema: default_session_schema(),
            ts,
            unix,
            action: action.to_string(),
            agent_id: agent_id.map(|s| s.to_string()),
            estate_name: estate_name.map(|s| s.to_string()),
            desired_hash: desired_hash.map(|s| s.to_string()),
            note: note.to_string(),
        },
    )
}

pub fn list_session_events(state_dir: &Path) -> Result<Vec<SessionEvent>, SupervisorError> {
    let path = session_journal_path(state_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path)?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let ev: SessionEvent = serde_json::from_str(line).map_err(|e| {
            SupervisorError::Other(format!("sessions.jsonl line {}: {e}", i + 1))
        })?;
        out.push(ev);
    }
    Ok(out)
}

pub fn tail_session_events(
    state_dir: &Path,
    n: usize,
) -> Result<Vec<SessionEvent>, SupervisorError> {
    let events = list_session_events(state_dir)?;
    if events.len() <= n {
        return Ok(events);
    }
    Ok(events[events.len() - n..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("cell-one-sess-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn journal_is_append_only_and_survives_list_tail() {
        let dir = tmp();
        journal_session(&dir, "spawn", Some("horizon"), Some("cell"), Some("sha256:a"), "bind")
            .unwrap();
        journal_session(&dir, "unspawn", Some("horizon"), None, None, "stop").unwrap();
        journal_session(&dir, "suspend", None, None, None, "pause").unwrap();
        journal_session(&dir, "resume", None, Some("cell"), None, "wake").unwrap();
        let all = list_session_events(&dir).unwrap();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0].action, "spawn");
        assert_eq!(all[3].action, "resume");
        assert!(dir.join(SESSION_JOURNAL).is_file());
        let tail = tail_session_events(&dir, 2).unwrap();
        assert_eq!(tail.len(), 2);
        assert_eq!(tail[0].action, "suspend");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
