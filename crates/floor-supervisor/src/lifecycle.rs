//! Operator lifecycle. Durable across restart. PIDs are not SoT.

use crate::{apply_with_profile_dir, stop_runtime, ActualState, SupervisorError};
use estate_schema::{estate_hash, Estate};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const LIFECYCLE_FILE: &str = "lifecycle.json";
pub const LIFECYCLE_LOG: &str = "lifecycle.jsonl";
pub const LIFECYCLE_SCHEMA: &str = "cell-one.lifecycle.v0";
pub const LIFECYCLE_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Running,
    Suspended,
}

impl LifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            LifecycleState::Running => "running",
            LifecycleState::Suspended => "suspended",
        }
    }
}

/// Survives `stop_runtime` (sessions/PIDs die; this file stays).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleRecord {
    #[serde(default = "default_lifecycle_schema")]
    pub schema: String,
    pub version: u32,
    pub state: LifecycleState,
    pub desired_hash: Option<String>,
    pub estate_name: Option<String>,
    pub suspended_at: Option<String>,
    pub resumed_at: Option<String>,
    pub durable: bool,
    pub note: String,
}

fn default_lifecycle_schema() -> String {
    LIFECYCLE_SCHEMA.to_string()
}

/// Append-only transition log. Durable. Not estate SoT.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleEvent {
    pub ts: String,
    pub from: Option<String>,
    pub to: String,
    pub action: String,
    pub desired_hash: Option<String>,
    pub note: String,
}

impl Default for LifecycleRecord {
    fn default() -> Self {
        Self {
            schema: default_lifecycle_schema(),
            version: LIFECYCLE_VERSION,
            state: LifecycleState::Suspended,
            desired_hash: None,
            estate_name: None,
            suspended_at: None,
            resumed_at: None,
            durable: true,
            note: "No lifecycle yet. Run estate resume or apply.".into(),
        }
    }
}

pub fn lifecycle_path(state_dir: &Path) -> std::path::PathBuf {
    state_dir.join(LIFECYCLE_FILE)
}

pub fn load_lifecycle(state_dir: &Path) -> Result<LifecycleRecord, SupervisorError> {
    let path = lifecycle_path(state_dir);
    if !path.exists() {
        return Ok(LifecycleRecord::default());
    }
    let text = std::fs::read_to_string(&path)?;
    serde_json::from_str(&text)
        .map_err(|e| SupervisorError::Other(format!("lifecycle.json: {e}")))
}

pub fn write_lifecycle(state_dir: &Path, record: &LifecycleRecord) -> Result<(), SupervisorError> {
    std::fs::create_dir_all(state_dir)?;
    crate::write_pretty_json(&lifecycle_path(state_dir), record)
}

pub fn append_lifecycle_event(
    state_dir: &Path,
    event: &LifecycleEvent,
) -> Result<(), SupervisorError> {
    std::fs::create_dir_all(state_dir)?;
    crate::append_json_line(&state_dir.join(LIFECYCLE_LOG), event)
}

pub fn list_lifecycle_events(state_dir: &Path) -> Result<Vec<LifecycleEvent>, SupervisorError> {
    let path = state_dir.join(LIFECYCLE_LOG);
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
        let ev: LifecycleEvent = serde_json::from_str(line).map_err(|e| {
            SupervisorError::Other(format!("lifecycle.jsonl line {}: {e}", i + 1))
        })?;
        out.push(ev);
    }
    Ok(out)
}

fn persist_transition(
    state_dir: &Path,
    prev: Option<&LifecycleRecord>,
    record: &LifecycleRecord,
    action: &str,
) -> Result<(), SupervisorError> {
    write_lifecycle(state_dir, record)?;
    append_lifecycle_event(
        state_dir,
        &LifecycleEvent {
            ts: now_rfc3339(),
            from: prev.map(|p| p.state.as_str().to_string()),
            to: record.state.as_str().to_string(),
            action: action.to_string(),
            desired_hash: record.desired_hash.clone(),
            note: record.note.clone(),
        },
    )
}

fn prior_lifecycle(state_dir: &Path) -> Result<Option<LifecycleRecord>, SupervisorError> {
    if lifecycle_path(state_dir).exists() {
        Ok(Some(load_lifecycle(state_dir)?))
    } else {
        Ok(None)
    }
}

pub fn suspend(state_dir: &Path) -> Result<LifecycleRecord, SupervisorError> {
    // A missing file is not a prior suspended state. Journal `from` stays
    // empty. The default record is not a file.
    let prev = prior_lifecycle(state_dir)?;
    stop_runtime(state_dir)?;
    crate::mark_leases_unspawned(state_dir)?;
    let record = LifecycleRecord {
        schema: default_lifecycle_schema(),
        version: LIFECYCLE_VERSION,
        state: LifecycleState::Suspended,
        desired_hash: prev.as_ref().and_then(|p| p.desired_hash.clone()),
        estate_name: prev.as_ref().and_then(|p| p.estate_name.clone()),
        suspended_at: Some(now_rfc3339()),
        resumed_at: prev.as_ref().and_then(|p| p.resumed_at.clone()),
        durable: true,
        note: "Suspended. Sessions and PIDs discarded. Estate, lanes, plans, lifecycle.json stay."
            .into(),
    };
    persist_transition(state_dir, prev.as_ref(), &record, "suspend")?;
    crate::journal_session(
        state_dir,
        "suspend",
        None,
        record.estate_name.as_deref(),
        record.desired_hash.as_deref(),
        "Suspended. sessions.jsonl stays; sessions/ discarded.",
    )?;
    Ok(record)
}

pub fn mark_running(estate: &Estate, state_dir: &Path) -> Result<LifecycleRecord, SupervisorError> {
    let prev = prior_lifecycle(state_dir)?;
    let record = LifecycleRecord {
        schema: default_lifecycle_schema(),
        version: LIFECYCLE_VERSION,
        state: LifecycleState::Running,
        desired_hash: Some(estate_hash(estate)),
        estate_name: Some(estate.name.clone()),
        suspended_at: prev.as_ref().and_then(|p| p.suspended_at.clone()),
        resumed_at: Some(now_rfc3339()),
        durable: true,
        note: "Applied from estate file. Runtime regenerable. Cloud-agent placements not spawned."
            .into(),
    };
    persist_transition(state_dir, prev.as_ref(), &record, "apply")?;
    Ok(record)
}

pub fn resume(
    estate: &Estate,
    state_dir: &Path,
    roots_base: &Path,
) -> Result<(ActualState, LifecycleRecord), SupervisorError> {
    let actual = apply_with_profile_dir(estate, state_dir, roots_base)?;
    crate::record_placements(estate, state_dir)?;
    // Apply writes lifecycle.json first. This resume line records that
    // file, not the missing-file default.
    let prev = prior_lifecycle(state_dir)?;
    let record = LifecycleRecord {
        schema: default_lifecycle_schema(),
        version: LIFECYCLE_VERSION,
        state: LifecycleState::Running,
        desired_hash: Some(estate_hash(estate)),
        estate_name: Some(estate.name.clone()),
        suspended_at: prev.as_ref().and_then(|p| p.suspended_at.clone()),
        resumed_at: Some(now_rfc3339()),
        durable: true,
        note: "Resumed from estate file. Runtime is regenerable. Cloud-agent placements not spawned."
            .into(),
    };
    persist_transition(state_dir, prev.as_ref(), &record, "resume")?;
    crate::journal_session(
        state_dir,
        "resume",
        None,
        Some(estate.name.as_str()),
        record.desired_hash.as_deref(),
        "Resumed. Runtime regenerable. Cloud-agent placements not spawned.",
    )?;
    Ok((actual, record))
}

fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drift_with_roots;

    fn example() -> Estate {
        estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
    }

    fn tmp() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("cell-one-life-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn suspend_keeps_lifecycle_after_session_discard() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        resume(&estate, &state, &root).unwrap();
        assert!(drift_with_roots(&estate, &state, Some(&root)).unwrap().in_sync);
        assert_eq!(
            load_lifecycle(&state).unwrap().state,
            LifecycleState::Running
        );
        suspend(&state).unwrap();
        assert!(lifecycle_path(&state).is_file());
        assert!(!state.join("sessions").exists());
        assert!(lifecycle_path(&state).is_file());
        let leases = crate::load_placements(&state)
            .unwrap()
            .expect("leases survive suspend");
        assert!(leases.leases.iter().all(|l| !l.spawned));
        assert_eq!(
            load_lifecycle(&state).unwrap().state,
            LifecycleState::Suspended
        );
        assert!(state.join(LIFECYCLE_LOG).is_file());
        assert!(!list_lifecycle_events(&state).unwrap().is_empty());
        let journal = crate::list_session_events(&state).unwrap();
        assert!(journal.iter().any(|e| e.action == "spawn"));
        assert!(journal.iter().any(|e| e.action == "unspawn"));
        assert!(journal.iter().any(|e| e.action == "suspend"));
        assert!(state.join(crate::SESSION_JOURNAL).is_file());
        assert!(!drift_with_roots(&estate, &state, Some(&root)).unwrap().in_sync);
        resume(&estate, &state, &root).unwrap();
        assert!(drift_with_roots(&estate, &state, Some(&root)).unwrap().in_sync);
        assert_eq!(
            load_lifecycle(&state).unwrap().state,
            LifecycleState::Running
        );
        assert!(crate::list_session_events(&state)
            .unwrap()
            .iter()
            .any(|e| e.action == "resume"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn lifecycle_jsonl_is_append_only_across_suspend_resume() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        resume(&estate, &state, &root).unwrap();
        let after_resume = std::fs::read_to_string(state.join(LIFECYCLE_LOG)).unwrap();
        assert!(!after_resume.is_empty());
        suspend(&state).unwrap();
        let after_suspend = std::fs::read_to_string(state.join(LIFECYCLE_LOG)).unwrap();
        assert!(
            after_suspend.starts_with(&after_resume),
            "suspend must append, not truncate lifecycle.jsonl"
        );
        assert!(after_suspend.len() > after_resume.len());
        resume(&estate, &state, &root).unwrap();
        let again = std::fs::read_to_string(state.join(LIFECYCLE_LOG)).unwrap();
        assert!(
            again.starts_with(&after_suspend),
            "resume must append, not truncate lifecycle.jsonl"
        );
        let events = list_lifecycle_events(&state).unwrap();
        assert!(events.iter().any(|e| e.action == "resume"));
        assert!(events.iter().any(|e| e.action == "suspend"));
        assert_eq!(events.len(), again.lines().filter(|l| !l.is_empty()).count());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn garbage_lifecycle_json_refuses_and_writes_nothing() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        std::fs::create_dir_all(&state).unwrap();
        let garbage = "not-json\n";
        std::fs::write(state.join(LIFECYCLE_FILE), garbage).unwrap();
        let err = suspend(&state).unwrap_err();
        assert!(
            err.to_string().contains("lifecycle.json"),
            "{err}"
        );
        assert_eq!(
            std::fs::read_to_string(state.join(LIFECYCLE_FILE)).unwrap(),
            garbage,
            "suspend must not overwrite garbage lifecycle.json"
        );
        let err = resume(&estate, &state, &root).unwrap_err();
        assert!(
            err.to_string().contains("lifecycle.json"),
            "{err}"
        );
        assert_eq!(
            std::fs::read_to_string(state.join(LIFECYCLE_FILE)).unwrap(),
            garbage,
            "resume must not overwrite garbage lifecycle.json"
        );
        let err = mark_running(&estate, &state).unwrap_err();
        assert!(
            err.to_string().contains("lifecycle.json"),
            "{err}"
        );
        assert_eq!(
            std::fs::read_to_string(state.join(LIFECYCLE_FILE)).unwrap(),
            garbage,
            "apply mark_running must not overwrite garbage lifecycle.json"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_lifecycle_does_not_journal_from_suspended() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        mark_running(&estate, &state).unwrap();
        let events = list_lifecycle_events(&state).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, "apply");
        assert_eq!(events[0].from, None);
        assert_eq!(events[0].to, "running");

        let root_suspend = tmp();
        let state_suspend = root_suspend.join("state");
        std::fs::create_dir_all(&state_suspend).unwrap();
        suspend(&state_suspend).unwrap();
        let events = list_lifecycle_events(&state_suspend).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, "suspend");
        assert_eq!(events[0].from, None);
        assert_eq!(events[0].to, "suspended");

        let root_resume = tmp();
        let state_resume = root_resume.join("state");
        resume(&estate, &state_resume, &root_resume).unwrap();
        let events = list_lifecycle_events(&state_resume).unwrap();
        assert_eq!(events[0].action, "apply");
        assert_eq!(events[0].from, None);
        assert_eq!(events[0].to, "running");
        assert_eq!(events[1].action, "resume");
        assert_eq!(events[1].from.as_deref(), Some("running"));
        resume(&estate, &state_resume, &root_resume).unwrap();
        let events = list_lifecycle_events(&state_resume).unwrap();
        assert_eq!(events.last().unwrap().from.as_deref(), Some("running"));

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&root_suspend);
        let _ = std::fs::remove_dir_all(&root_resume);
    }

    #[test]
    fn resume_does_not_spawn_cloud_agent_sessions() {
        let estate = example();
        let root = tmp();
        let (actual, _) = resume(&estate, &root.join("state"), &root).unwrap();
        assert_eq!(actual.sessions.len(), estate.agents.len());
        assert!(estate
            .placements
            .iter()
            .any(|p| p.kind == estate_schema::PlacementKind::CloudAgent));
        let _ = std::fs::remove_dir_all(&root);
    }
}
