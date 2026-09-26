//! Local routine tick state under the estate state-dir.
//! last_run / next_due are throwaway-local. Not Grok Bot sync.

use anyhow::{bail, Context, Result};
use estate_schema::{Estate, Routine, RoutineSchedule};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const STATE_SCHEMA: &str = "cell-one.routine-state.v0";
pub(crate) const STATE_FILE: &str = "routine-state.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub(crate) struct RoutineStateFile {
    pub schema: String,
    #[serde(default)]
    pub routines: BTreeMap<String, RoutineRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub(crate) struct RoutineRow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_due: Option<i64>,
}

pub(crate) fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub(crate) fn state_path(state_dir: &Path) -> std::path::PathBuf {
    state_dir.join(STATE_FILE)
}

pub(crate) fn load_state(state_dir: &Path) -> Result<RoutineStateFile> {
    let path = state_path(state_dir);
    if !path.is_file() {
        return Ok(RoutineStateFile {
            schema: STATE_SCHEMA.into(),
            routines: BTreeMap::new(),
        });
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let file: RoutineStateFile = serde_json::from_str(&text)
        .with_context(|| format!("parse {}", path.display()))?;
    if file.schema != STATE_SCHEMA {
        bail!(
            "refuse:routine-state: schema must be {STATE_SCHEMA} (got {})",
            file.schema
        );
    }
    Ok(file)
}

pub(crate) fn save_state(state_dir: &Path, file: &RoutineStateFile) -> Result<()> {
    fs::create_dir_all(state_dir)?;
    let path = state_path(state_dir);
    let body = serde_json::to_string_pretty(file)?;
    fs::write(&path, format!("{body}\n"))?;
    Ok(())
}

pub(crate) fn parsed_schedule(routine: &Routine) -> Result<Option<RoutineSchedule>, String> {
    match &routine.schedule {
        None => Ok(None),
        Some(raw) => RoutineSchedule::parse(raw).map(Some),
    }
}

pub(crate) fn row_for<'a>(file: &'a RoutineStateFile, id: &str) -> Option<&'a RoutineRow> {
    let want = estate_schema::normalize_name(id);
    file.routines
        .iter()
        .find(|(k, _)| estate_schema::normalize_name(k) == want)
        .map(|(_, v)| v)
}

pub(crate) fn is_due(routine: &Routine, row: Option<&RoutineRow>, now: i64) -> Result<bool, String> {
    if !routine.is_enabled() {
        return Ok(false);
    }
    let Some(sched) = parsed_schedule(routine)? else {
        return Ok(false);
    };
    Ok(sched.is_due(
        row.and_then(|r| r.last_run),
        row.and_then(|r| r.next_due),
        now,
    ))
}

pub(crate) fn mark_ran(
    file: &mut RoutineStateFile,
    routine: &Routine,
    now: i64,
) -> Result<RoutineRow, String> {
    let sched = parsed_schedule(routine)?
        .ok_or_else(|| format!("refuse:routine-schedule: routine '{}' has no schedule", routine.id))?;
    let next = sched
        .next_after(now)
        .ok_or_else(|| format!("refuse:routine-schedule: routine '{}' has no next fire", routine.id))?;
    let row = RoutineRow {
        last_run: Some(now),
        next_due: Some(next),
    };
    file.routines.insert(routine.id.clone(), row.clone());
    Ok(row)
}

pub(crate) fn format_unix(ts: Option<i64>) -> String {
    match ts {
        None => "-".into(),
        Some(t) => chrono_utc(t),
    }
}

fn chrono_utc(ts: i64) -> String {
    use chrono::{TimeZone, Utc};
    match Utc.timestamp_opt(ts, 0).single() {
        Some(dt) => dt.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        None => ts.to_string(),
    }
}

pub(crate) fn status_line(estate: &Estate, routine: &Routine, row: Option<&RoutineRow>) -> String {
    let schedule = routine.schedule.as_deref().unwrap_or("-");
    let enabled = if routine.is_enabled() { "true" } else { "false" };
    let last = format_unix(row.and_then(|r| r.last_run));
    let next = format_unix(row.and_then(|r| r.next_due));
    let pkg = estate
        .pack_package(&routine.package)
        .map(|p| p.id.as_str())
        .unwrap_or(routine.package.as_str());
    format!(
        "{} package={pkg} schedule={schedule} enabled={enabled} last_run={last} next_due={next}",
        routine.id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use estate_schema::Routine;

    fn routine(id: &str, schedule: Option<&str>, enabled: Option<bool>) -> Routine {
        Routine {
            id: id.into(),
            package: "classify-ping".into(),
            note: None,
            schedule: schedule.map(|s| s.into()),
            enabled,
        }
    }

    #[test]
    fn tick_due_then_not_due() {
        let r = routine("standing-classify", Some("@every 15m"), None);
        assert!(is_due(&r, None, 1_000).unwrap());
        let mut file = RoutineStateFile {
            schema: STATE_SCHEMA.into(),
            routines: BTreeMap::new(),
        };
        let row = mark_ran(&mut file, &r, 1_000).unwrap();
        assert_eq!(row.last_run, Some(1_000));
        assert_eq!(row.next_due, Some(1_000 + 15 * 60));
        assert!(!is_due(&r, Some(&row), 1_000 + 60).unwrap());
        assert!(is_due(&r, Some(&row), 1_000 + 15 * 60).unwrap());
    }

    #[test]
    fn disabled_and_unscheduled_skip() {
        let off = routine("off", Some("@hourly"), Some(false));
        assert!(!is_due(&off, None, 1_000).unwrap());
        let on_demand = routine("once", None, None);
        assert!(!is_due(&on_demand, None, 1_000).unwrap());
    }
}
