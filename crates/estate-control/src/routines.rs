//! Local routine tick state under the estate state-dir.
//! last_run / next_due are throwaway-local. Not Grok Bot sync.

use anyhow::{bail, Context, Result};
use estate_schema::{Estate, Routine, RoutineSchedule, MIN_INTERVAL_SECS};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const STATE_SCHEMA: &str = "cell-one.routine-state.v0";
pub(crate) const STATE_FILE: &str = "routine-state.json";
/// Test hook: integer seconds. Below 5m requires `--max-cycles`.
pub(crate) const WATCH_INTERVAL_ENV: &str = "CELL_ROUTINE_WATCH_INTERVAL_SECS";

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

/// One journal row the digest can print. Local receipts only.
#[derive(Debug, Clone)]
pub(crate) struct DigestReceipt {
    pub id: String,
    pub routine_id: Option<String>,
    pub package_id: Option<String>,
    pub chain_id: Option<String>,
    pub completion_label: Option<String>,
}

/// Glance at the last local wake: ran/skipped plus receipt ids.
/// Not Grok Bot sync.
pub(crate) fn render_digest(
    estate: &Estate,
    file: &RoutineStateFile,
    receipts: &[DigestReceipt],
    only: Option<&str>,
) -> String {
    let rows: Vec<&Routine> = match only {
        Some(want) => estate.routine(want).into_iter().collect(),
        None => estate.routines.iter().collect(),
    };
    let mut ran = 0usize;
    let mut skipped = 0usize;
    let mut body = String::new();
    for routine in &rows {
        let row = row_for(file, &routine.id);
        let ran_this = row.and_then(|r| r.last_run).is_some();
        if ran_this {
            ran += 1;
        } else {
            skipped += 1;
        }
        let status = if ran_this { "ran" } else { "skipped" };
        let reason = if ran_this {
            String::new()
        } else {
            format!(" reason={}", skip_reason_text(routine, row))
        };
        let pkg = estate
            .pack_package(&routine.package)
            .map(|p| p.id.as_str())
            .unwrap_or(routine.package.as_str());
        body.push_str(&format!(
            "  {} status={status} package={pkg}{reason} last_run={} next_due={}\n",
            routine.id,
            format_unix(row.and_then(|r| r.last_run)),
            format_unix(row.and_then(|r| r.next_due)),
        ));
        let matched: Vec<&DigestReceipt> = receipts
            .iter()
            .filter(|r| {
                r.routine_id
                    .as_deref()
                    .map(|id| estate_schema::normalize_name(id) == estate_schema::normalize_name(&routine.id))
                    .unwrap_or(false)
            })
            .collect();
        if matched.is_empty() {
            body.push_str("    receipts=0\n");
            continue;
        }
        for rec in matched {
            let chain = rec.chain_id.as_deref().unwrap_or("-");
            let pkg_id = rec.package_id.as_deref().unwrap_or(pkg);
            let label = rec.completion_label.as_deref().unwrap_or("-");
            body.push_str(&format!(
                "    receipt={} package={pkg_id} chain={chain} completion_label={label}\n",
                rec.id
            ));
        }
    }
    if rows.is_empty() {
        return "routine digest ran=0 skipped=0\n  no standing routines\n".into();
    }
    format!("routine digest ran={ran} skipped={skipped}\n{body}")
}

fn skip_reason_text(routine: &Routine, row: Option<&RoutineRow>) -> String {
    if !routine.is_enabled() {
        return "disabled".into();
    }
    if routine.schedule.is_none() {
        return "no schedule".into();
    }
    match row.and_then(|r| r.next_due) {
        Some(due) => format!("next_due={}", format_unix(Some(due))),
        None => "not due".into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WatchPlan {
    pub interval_secs: i64,
    pub interval_label: String,
    pub max_cycles: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatchStopReason {
    MaxCycles,
    Signal,
}

impl WatchStopReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::MaxCycles => "max-cycles",
            Self::Signal => "sigint",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WatchStop {
    pub cycles: u32,
    pub reason: WatchStopReason,
}

pub(crate) fn parse_watch_interval(raw: &str) -> Result<i64, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("refuse:watch-interval: interval is empty".into());
    }
    if let Ok(secs) = raw.parse::<i64>() {
        if secs < 0 {
            return Err("refuse:watch-interval: interval must be ≥ 0".into());
        }
        return Ok(secs);
    }
    parse_watch_duration(raw)
}

fn parse_watch_duration(raw: &str) -> Result<i64, String> {
    let compact = raw.replace(' ', "");
    let mut digits = String::new();
    let mut unit = None;
    let last = compact.chars().count().saturating_sub(1);
    for (i, ch) in compact.chars().enumerate() {
        if ch.is_ascii_digit() && unit.is_none() {
            digits.push(ch);
            continue;
        }
        if i == last && matches!(ch, 'h' | 'H' | 'm' | 'M' | 's' | 'S') {
            unit = Some(ch);
            continue;
        }
        return Err(format!(
            "refuse:watch-interval: '{raw}' must be 5m / 15m / 1h or integer seconds"
        ));
    }
    let Some(unit) = unit else {
        return Err(format!(
            "refuse:watch-interval: '{raw}' must be 5m / 15m / 1h or integer seconds"
        ));
    };
    if digits.is_empty() {
        return Err(format!("refuse:watch-interval: '{raw}' needs a count"));
    }
    let num: i64 = digits
        .parse()
        .map_err(|_| format!("refuse:watch-interval: '{raw}' count is not a number"))?;
    if num <= 0 {
        return Err("refuse:watch-interval: count must be ≥ 1".into());
    }
    match unit {
        'h' | 'H' => Ok(num.saturating_mul(3600)),
        'm' | 'M' => Ok(num.saturating_mul(60)),
        's' | 'S' => Ok(num),
        _ => unreachable!(),
    }
}

pub(crate) fn format_watch_interval_secs(secs: i64) -> String {
    if secs > 0 && secs % 3600 == 0 {
        format!("{}h ({}s)", secs / 3600, secs)
    } else if secs > 0 && secs % 60 == 0 {
        format!("{}m ({}s)", secs / 60, secs)
    } else {
        format!("{secs}s")
    }
}

pub(crate) fn resolve_watch_plan(
    interval: &str,
    max_cycles: Option<u32>,
    env_interval_secs: Option<&str>,
) -> Result<WatchPlan, String> {
    if let Some(0) = max_cycles {
        return Err("refuse:watch-max-cycles: --max-cycles must be ≥ 1".into());
    }
    let cli_secs = parse_watch_interval(interval)?;
    if cli_secs < MIN_INTERVAL_SECS {
        return Err(format!(
            "refuse:watch-interval: {cli_secs}s; minimum is {MIN_INTERVAL_SECS}s (5m)"
        ));
    }
    let interval_secs = if let Some(raw) = env_interval_secs {
        let env_secs = raw.trim().parse::<i64>().map_err(|_| {
            format!("refuse:watch-interval: {WATCH_INTERVAL_ENV} must be integer seconds")
        })?;
        if env_secs < 0 {
            return Err(format!(
                "refuse:watch-interval: {WATCH_INTERVAL_ENV} must be ≥ 0"
            ));
        }
        if env_secs < MIN_INTERVAL_SECS && max_cycles.is_none() {
            return Err(format!(
                "refuse:watch-interval: {WATCH_INTERVAL_ENV}={env_secs}s is below {MIN_INTERVAL_SECS}s (5m); set --max-cycles to use the test hook"
            ));
        }
        env_secs
    } else {
        cli_secs
    };
    Ok(WatchPlan {
        interval_secs,
        interval_label: format_watch_interval_secs(interval_secs),
        max_cycles,
    })
}

pub(crate) fn watch_status_line(plan: &WatchPlan) -> String {
    let max = plan
        .max_cycles
        .map(|n| n.to_string())
        .unwrap_or_else(|| "-".into());
    format!(
        "routine watch interval={} max_cycles={max} stop=SIGINT (Ctrl-C) live_sync=false",
        plan.interval_label
    )
}

/// Tick-cycle loop with injectable sleep / stop. No real 5m wait in tests.
pub(crate) fn run_watch_loop<E, Tick, Sleep, Stopped>(
    plan: &WatchPlan,
    mut tick: Tick,
    mut sleep_secs: Sleep,
    mut stopped: Stopped,
) -> Result<WatchStop, E>
where
    Tick: FnMut(u32) -> Result<(), E>,
    Sleep: FnMut(i64),
    Stopped: FnMut() -> bool,
{
    let mut cycles = 0u32;
    loop {
        if stopped() {
            return Ok(WatchStop {
                cycles,
                reason: WatchStopReason::Signal,
            });
        }
        cycles = cycles.saturating_add(1);
        tick(cycles)?;
        if let Some(max) = plan.max_cycles {
            if cycles >= max {
                return Ok(WatchStop {
                    cycles,
                    reason: WatchStopReason::MaxCycles,
                });
            }
        }
        if stopped() {
            return Ok(WatchStop {
                cycles,
                reason: WatchStopReason::Signal,
            });
        }
        sleep_secs(plan.interval_secs);
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

    #[test]
    fn digest_shape_names_ran_skipped_package_chain_receipt_and_label() {
        let estate = estate_schema::load_estate_unvalidated(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/fixtures/agent-pack-handoff.yaml"),
        )
        .unwrap();
        let mut file = RoutineStateFile {
            schema: STATE_SCHEMA.into(),
            routines: BTreeMap::new(),
        };
        file.routines.insert(
            "standing-classify".into(),
            RoutineRow {
                last_run: Some(1_700_000_000),
                next_due: Some(1_700_003_600),
            },
        );
        let text = render_digest(
            &estate,
            &file,
            &[DigestReceipt {
                id: "r-1-digest".into(),
                routine_id: Some("standing-classify".into()),
                package_id: Some("classify-ping".into()),
                chain_id: Some("chain-classify-ping-1".into()),
                completion_label: Some("Sci/Tech".into()),
            }],
            None,
        );
        assert!(text.starts_with("routine digest ran=1 skipped=0\n"), "{text}");
        assert!(text.contains("standing-classify status=ran package=classify-ping"), "{text}");
        assert!(
            text.contains("receipt=r-1-digest package=classify-ping chain=chain-classify-ping-1 completion_label=Sci/Tech"),
            "{text}"
        );
        assert!(!text.contains("Grok Bot sync"), "{text}");
        assert!(!text.contains("live PASS"), "{text}");
    }

    #[test]
    fn watch_plan_default_is_five_minutes() {
        let plan = resolve_watch_plan("5m", None, None).unwrap();
        assert_eq!(plan.interval_secs, MIN_INTERVAL_SECS);
        assert_eq!(plan.interval_label, "5m (300s)");
        assert_eq!(plan.max_cycles, None);
        let line = watch_status_line(&plan);
        assert!(line.contains("interval=5m (300s)"), "{line}");
        assert!(line.contains("max_cycles=-"), "{line}");
        assert!(line.contains("stop=SIGINT (Ctrl-C)"), "{line}");
        assert!(line.contains("live_sync=false"), "{line}");
        assert!(!line.contains("Grok Bot sync"), "{line}");
    }

    #[test]
    fn watch_plan_refuses_sub_five_minute_interval() {
        let err = resolve_watch_plan("1m", Some(1), None).unwrap_err();
        assert!(err.contains("refuse:watch-interval"), "{err}");
        assert!(err.contains("60s"), "{err}");
        let err = resolve_watch_plan("5m", Some(0), None).unwrap_err();
        assert!(err.contains("refuse:watch-max-cycles"), "{err}");
        let err = resolve_watch_plan("5m", None, Some("0")).unwrap_err();
        assert!(err.contains("CELL_ROUTINE_WATCH_INTERVAL_SECS"), "{err}");
        assert!(err.contains("--max-cycles"), "{err}");
    }

    #[test]
    fn watch_plan_test_hook_allows_zero_with_max_cycles() {
        let plan = resolve_watch_plan("5m", Some(2), Some("0")).unwrap();
        assert_eq!(plan.interval_secs, 0);
        assert_eq!(plan.max_cycles, Some(2));
        assert_eq!(plan.interval_label, "0s");
    }

    #[test]
    fn watch_loop_two_cycles_fake_sleep_no_wall_clock() {
        let plan = resolve_watch_plan("5m", Some(2), Some("0")).unwrap();
        let mut ticks = Vec::new();
        let mut sleeps = Vec::new();
        let stop = run_watch_loop(
            &plan,
            |cycle| {
                ticks.push(cycle);
                Ok::<(), String>(())
            },
            |secs| sleeps.push(secs),
            || false,
        )
        .unwrap();
        assert_eq!(stop.cycles, 2);
        assert_eq!(stop.reason, WatchStopReason::MaxCycles);
        assert_eq!(stop.reason.as_str(), "max-cycles");
        assert_eq!(ticks, vec![1, 2]);
        assert_eq!(sleeps, vec![0]);
    }

    #[test]
    fn watch_loop_stops_on_signal_after_first_cycle() {
        let plan = resolve_watch_plan("15m", Some(9), Some("7")).unwrap();
        let mut ticks = 0u32;
        let stop = run_watch_loop(
            &plan,
            |_cycle| {
                ticks += 1;
                Ok::<(), String>(())
            },
            |_secs| panic!("signal stop must not sleep"),
            || ticks >= 1,
        )
        .unwrap();
        assert_eq!(ticks, 1);
        assert_eq!(stop.cycles, 1);
        assert_eq!(stop.reason, WatchStopReason::Signal);
        assert_eq!(stop.reason.as_str(), "sigint");
    }
}
