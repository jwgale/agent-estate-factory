//! Routine schedule parse: 5-field cron or `@daily` / `@hourly` / `@every Nh` / `@every Nm`.
//! Minimum interval is 5 minutes. Tick and watch CLI use this; there is no
//! cloud cron daemon.

use chrono::{Datelike, TimeZone, Timelike, Utc};
use std::collections::BTreeSet;

pub const MIN_INTERVAL_SECS: i64 = 5 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutineSchedule {
    /// Fire every `secs` seconds (≥ 300).
    Every { secs: i64 },
    /// 5-field cron (minute hour day month weekday), UTC.
    Cron(CronExpr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronExpr {
    minute: Field,
    hour: Field,
    day: Field,
    month: Field,
    weekday: Field,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Field {
    Any,
    Values(BTreeSet<u32>),
}

impl RoutineSchedule {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err("schedule is empty".into());
        }
        if raw.eq_ignore_ascii_case("@daily") {
            return Ok(RoutineSchedule::Cron(CronExpr::daily()));
        }
        if raw.eq_ignore_ascii_case("@hourly") {
            return Ok(RoutineSchedule::Cron(CronExpr::hourly()));
        }
        if let Some(rest) = strip_every(raw) {
            return parse_every(rest);
        }
        let parts: Vec<&str> = raw.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(format!(
                "schedule '{raw}' must be 5-field cron or @daily / @hourly / @every Nh / @every Nm"
            ));
        }
        let cron = CronExpr {
            minute: parse_field(parts[0], 0, 59, "minute")?,
            hour: parse_field(parts[1], 0, 23, "hour")?,
            day: parse_field(parts[2], 1, 31, "day")?,
            month: parse_field(parts[3], 1, 12, "month")?,
            weekday: parse_weekday(parts[4])?,
        };
        let interval = cron.min_interval_secs()?;
        if interval < MIN_INTERVAL_SECS {
            return Err(format!(
                "schedule '{raw}' interval is {interval}s; minimum is {}s (5m)",
                MIN_INTERVAL_SECS
            ));
        }
        Ok(RoutineSchedule::Cron(cron))
    }

    pub fn min_interval_secs(&self) -> i64 {
        match self {
            RoutineSchedule::Every { secs } => *secs,
            RoutineSchedule::Cron(cron) => cron
                .min_interval_secs()
                .unwrap_or(MIN_INTERVAL_SECS),
        }
    }

    /// Next fire strictly after `from_unix` (UTC seconds).
    pub fn next_after(&self, from_unix: i64) -> Option<i64> {
        match self {
            RoutineSchedule::Every { secs } => Some(from_unix.saturating_add(*secs)),
            RoutineSchedule::Cron(cron) => cron.next_after(from_unix),
        }
    }

    pub fn is_due(&self, last_run: Option<i64>, next_due: Option<i64>, now: i64) -> bool {
        if let Some(due) = next_due {
            return due <= now;
        }
        match last_run {
            None => true,
            Some(last) => last.saturating_add(self.min_interval_secs()) <= now,
        }
    }
}

fn strip_every(raw: &str) -> Option<&str> {
    let lower = raw.to_ascii_lowercase();
    lower.strip_prefix("@every").map(|_| {
        let rest = &raw["@every".len()..];
        rest.trim()
    })
}

fn parse_every(rest: &str) -> Result<RoutineSchedule, String> {
    let rest = rest.trim();
    let (num, unit) = split_every_token(rest)?;
    if num == 0 {
        return Err(format!("@every {rest}: count must be ≥ 1"));
    }
    let secs = match unit {
        'h' | 'H' => (num as i64).saturating_mul(3600),
        'm' | 'M' => (num as i64).saturating_mul(60),
        _ => {
            return Err(format!(
                "@every {rest}: use Nh or Nm (for example @every 1h or @every 15m)"
            ))
        }
    };
    if secs < MIN_INTERVAL_SECS {
        return Err(format!(
            "@every {rest}: interval is {secs}s; minimum is {}s (5m)",
            MIN_INTERVAL_SECS
        ));
    }
    Ok(RoutineSchedule::Every { secs })
}

fn split_every_token(rest: &str) -> Result<(u32, char), String> {
    let rest = rest.replace(' ', "");
    if rest.is_empty() {
        return Err("@every needs Nh or Nm".into());
    }
    let mut digits = String::new();
    let mut unit = None;
    for (i, ch) in rest.chars().enumerate() {
        if ch.is_ascii_digit() && unit.is_none() {
            digits.push(ch);
            continue;
        }
        if i + 1 == rest.chars().count() && matches!(ch, 'h' | 'H' | 'm' | 'M') {
            unit = Some(ch);
            continue;
        }
        return Err(format!("@every {rest}: use Nh or Nm"));
    }
    let Some(unit) = unit else {
        return Err(format!("@every {rest}: use Nh or Nm"));
    };
    let num: u32 = digits
        .parse()
        .map_err(|_| format!("@every {rest}: count is not a number"))?;
    Ok((num, unit))
}

fn parse_field(raw: &str, min: u32, max: u32, name: &str) -> Result<Field, String> {
    if raw == "*" {
        return Ok(Field::Any);
    }
    let mut values = BTreeSet::new();
    for part in raw.split(',') {
        push_part(part, min, max, name, &mut values)?;
    }
    if values.is_empty() {
        return Err(format!("cron {name} field '{raw}' is empty"));
    }
    Ok(Field::Values(values))
}

fn parse_weekday(raw: &str) -> Result<Field, String> {
    if raw == "*" {
        return Ok(Field::Any);
    }
    let mut values = BTreeSet::new();
    for part in raw.split(',') {
        push_part(part, 0, 7, "weekday", &mut values)?;
    }
    // 7 == Sunday, same as 0.
    if values.remove(&7) {
        values.insert(0);
    }
    if values.is_empty() {
        return Err(format!("cron weekday field '{raw}' is empty"));
    }
    Ok(Field::Values(values))
}

fn push_part(
    part: &str,
    min: u32,
    max: u32,
    name: &str,
    values: &mut BTreeSet<u32>,
) -> Result<(), String> {
    let part = part.trim();
    if part.is_empty() {
        return Err(format!("cron {name} field has an empty list item"));
    }
    if let Some(step_raw) = part.strip_prefix("*/") {
        let step = parse_u32(step_raw, name)?;
        if step == 0 {
            return Err(format!("cron {name} step must be ≥ 1"));
        }
        let mut v = min;
        while v <= max {
            values.insert(v);
            v = v.saturating_add(step);
            if v == 0 {
                break;
            }
        }
        return Ok(());
    }
    if let Some((range, step_raw)) = part.split_once('/') {
        let step = parse_u32(step_raw, name)?;
        if step == 0 {
            return Err(format!("cron {name} step must be ≥ 1"));
        }
        let (lo, hi) = parse_range(range, min, max, name)?;
        let mut v = lo;
        while v <= hi {
            values.insert(v);
            v = v.saturating_add(step);
            if v == 0 {
                break;
            }
        }
        return Ok(());
    }
    if part.contains('-') {
        let (lo, hi) = parse_range(part, min, max, name)?;
        for v in lo..=hi {
            values.insert(v);
        }
        return Ok(());
    }
    let v = parse_u32(part, name)?;
    if v < min || v > max {
        return Err(format!("cron {name} value {v} is outside {min}-{max}"));
    }
    values.insert(v);
    Ok(())
}

fn parse_range(raw: &str, min: u32, max: u32, name: &str) -> Result<(u32, u32), String> {
    let (lo, hi) = raw
        .split_once('-')
        .ok_or_else(|| format!("cron {name} range '{raw}' is not a-b"))?;
    let lo = parse_u32(lo, name)?;
    let hi = parse_u32(hi, name)?;
    if lo < min || hi > max || lo > hi {
        return Err(format!("cron {name} range {lo}-{hi} is outside {min}-{max}"));
    }
    Ok((lo, hi))
}

fn parse_u32(raw: &str, name: &str) -> Result<u32, String> {
    raw.trim()
        .parse::<u32>()
        .map_err(|_| format!("cron {name} '{raw}' is not a number"))
}

impl Field {
    fn matches(&self, value: u32) -> bool {
        match self {
            Field::Any => true,
            Field::Values(set) => set.contains(&value),
        }
    }
}

impl CronExpr {
    fn daily() -> Self {
        Self {
            minute: Field::Values(BTreeSet::from([0])),
            hour: Field::Values(BTreeSet::from([0])),
            day: Field::Any,
            month: Field::Any,
            weekday: Field::Any,
        }
    }

    fn hourly() -> Self {
        Self {
            minute: Field::Values(BTreeSet::from([0])),
            hour: Field::Any,
            day: Field::Any,
            month: Field::Any,
            weekday: Field::Any,
        }
    }

    fn matches_dt(&self, dt: chrono::DateTime<Utc>) -> bool {
        let minute = dt.minute();
        let hour = dt.hour();
        let day = dt.day();
        let month = dt.month();
        let weekday = dt.weekday().num_days_from_sunday();
        if !self.minute.matches(minute) || !self.hour.matches(hour) || !self.month.matches(month) {
            return false;
        }
        let day_any = matches!(self.day, Field::Any);
        let dow_any = matches!(self.weekday, Field::Any);
        match (day_any, dow_any) {
            (true, true) => true,
            (false, true) => self.day.matches(day),
            (true, false) => self.weekday.matches(weekday),
            // Both restricted: either matches (vixie).
            (false, false) => self.day.matches(day) || self.weekday.matches(weekday),
        }
    }

    fn next_after(&self, from_unix: i64) -> Option<i64> {
        let start = from_unix.saturating_add(60) / 60 * 60;
        // Scan one year of minutes. Tick is CLI, not a daemon.
        let limit = start.saturating_add(366 * 24 * 60 * 60);
        let mut t = start;
        while t <= limit {
            if let Some(dt) = Utc.timestamp_opt(t, 0).single() {
                if self.matches_dt(dt) {
                    return Some(t);
                }
            }
            t = t.saturating_add(60);
        }
        None
    }

    fn min_interval_secs(&self) -> Result<i64, String> {
        // Probe from a fixed UTC origin so validate is deterministic.
        let origin = Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .map(|d| d.timestamp())
            .unwrap_or(0);
        let first = self
            .next_after(origin - 60)
            .ok_or_else(|| "cron never fires".to_string())?;
        let second = self
            .next_after(first)
            .ok_or_else(|| "cron never fires a second time".to_string())?;
        Ok(second - first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorthands_and_every() {
        assert!(matches!(
            RoutineSchedule::parse("@daily").unwrap(),
            RoutineSchedule::Cron(_)
        ));
        assert!(matches!(
            RoutineSchedule::parse("@hourly").unwrap(),
            RoutineSchedule::Cron(_)
        ));
        assert_eq!(
            RoutineSchedule::parse("@every 1h").unwrap(),
            RoutineSchedule::Every { secs: 3600 }
        );
        assert_eq!(
            RoutineSchedule::parse("@every 15m").unwrap(),
            RoutineSchedule::Every { secs: 15 * 60 }
        );
        assert_eq!(
            RoutineSchedule::parse("@every 5m").unwrap(),
            RoutineSchedule::Every { secs: 5 * 60 }
        );
        let err = RoutineSchedule::parse("@every 4m").unwrap_err();
        assert!(err.contains("minimum"), "{err}");
        let err = RoutineSchedule::parse("@every 0h").unwrap_err();
        assert!(err.contains("≥ 1") || err.contains("minimum"), "{err}");
    }

    #[test]
    fn five_field_cron_and_min_interval() {
        let hourly = RoutineSchedule::parse("0 * * * *").unwrap();
        assert!(hourly.min_interval_secs() >= MIN_INTERVAL_SECS);
        let every_five = RoutineSchedule::parse("*/5 * * * *").unwrap();
        assert_eq!(every_five.min_interval_secs(), 5 * 60);
        let err = RoutineSchedule::parse("* * * * *").unwrap_err();
        assert!(err.contains("minimum") || err.contains("interval"), "{err}");
        let err = RoutineSchedule::parse("*/1 * * * *").unwrap_err();
        assert!(err.contains("minimum") || err.contains("interval"), "{err}");
        let daily = RoutineSchedule::parse("0 0 * * *").unwrap();
        assert!(daily.min_interval_secs() >= 23 * 3600);
    }

    #[test]
    fn due_and_next() {
        let every = RoutineSchedule::parse("@every 15m").unwrap();
        assert!(every.is_due(None, None, 1_000));
        assert!(!every.is_due(Some(1_000), Some(1_000 + 15 * 60), 1_000 + 60));
        assert!(every.is_due(Some(1_000), Some(1_000 + 15 * 60), 1_000 + 15 * 60));
        assert_eq!(every.next_after(1_000), Some(1_000 + 15 * 60));

        let hourly = RoutineSchedule::parse("@hourly").unwrap();
        let next = hourly.next_after(1_767_700_800).unwrap(); // 2026-01-06 12:00:00 UTC-ish probe
        let dt = Utc.timestamp_opt(next, 0).single().unwrap();
        assert_eq!(dt.minute(), 0);
        assert!(next > 1_767_700_800);
    }

    #[test]
    fn refuse_unknown() {
        assert!(RoutineSchedule::parse("daily").is_err());
        assert!(RoutineSchedule::parse("@weekly").is_err());
        assert!(RoutineSchedule::parse("0 0 0 0").is_err());
    }
}
