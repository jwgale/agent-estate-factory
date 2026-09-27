//! `estate routine runner-prove` — mock runner + crew session stitch.
//!
//! Starts the supervised runner on a multi-hop standing routine, waits
//! for one tick, asserts hop 2 `context=applied`, then checks session
//! reuse, ended→fresh, and double-start refuse. `--mock` stays
//! in-process. No network. No live PASS. READY_FOR_LIVE_TEST: no.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::decisions::DecisionReceipt;
use crate::routine_runner::{self, RunnerWatchArgs};
use crate::routines::{self, WATCH_INTERVAL_ENV};

pub(crate) const PROVE_SCHEMA: &str = "cell-one.routine-runner-prove.v0";
const DEFAULT_ROUTINE: &str = "standing-classify";
const LOCKED_ESTATE: &str = "examples/estate.yaml";
const LOCKED_CKSUM: &str = "43770130 3391";
const TICK_WAIT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone)]
struct ProveCheck {
    name: String,
    ok: bool,
    detail: String,
}

#[derive(Debug, Clone)]
struct ProveReport {
    ok: bool,
    routine_id: String,
    session_id: String,
    reused_session_id: String,
    fresh_session_id: String,
    state_dir: String,
    checks: Vec<ProveCheck>,
}

impl ProveReport {
    fn to_json(&self) -> Value {
        let mut checks = serde_json::Map::new();
        for c in &self.checks {
            checks.insert(
                c.name.clone(),
                json!({
                    "ok": c.ok,
                    "detail": c.detail,
                }),
            );
        }
        json!({
            "schema": PROVE_SCHEMA,
            "ok": self.ok,
            "routine_id": self.routine_id,
            "session_id": self.session_id,
            "reused_session_id": self.reused_session_id,
            "fresh_session_id": self.fresh_session_id,
            "state_dir": self.state_dir,
            "live_sync": false,
            "ready_for_live_test": false,
            "checks": Value::Object(checks),
        })
    }
}

pub(crate) fn cmd_routine_runner_prove(
    routine_id: &str,
    estate_path: &Path,
    state_dir: Option<&Path>,
) -> Result<()> {
    let state_dir = match state_dir {
        Some(dir) => dir.to_path_buf(),
        None => throwaway_state(routine_id),
    };
    std::fs::create_dir_all(&state_dir)
        .with_context(|| format!("create {}", state_dir.display()))?;

    let mut checks = Vec::new();
    let mut session_id = String::new();
    let mut reused_session_id = String::new();
    let mut fresh_session_id = String::new();
    let _guard = RunnerGuard {
        state_dir: state_dir.clone(),
    };

    push_check(
        &mut checks,
        "locked-estate-cksum",
        locked_estate_ok(),
        "examples/estate.yaml cksum 43770130 3391",
    );

    if let Err(err) = estate_schema::load_estate(estate_path) {
        push_fail(&mut checks, "estate", err.to_string());
        return finish(routine_id, &session_id, &reused_session_id, &fresh_session_id, &state_dir, checks);
    }

    let prev_interval = std::env::var(WATCH_INTERVAL_ENV).ok();
    std::env::set_var(WATCH_INTERVAL_ENV, "2");

    let start = routine_runner::cmd_runner_start(watch_args(routine_id, estate_path, &state_dir, 30));
    match start {
        Ok(()) => push_ok(&mut checks, "runner-start", "started"),
        Err(err) => {
            restore_interval(prev_interval.as_deref());
            push_fail(&mut checks, "runner-start", err.to_string());
            return finish(routine_id, &session_id, &reused_session_id, &fresh_session_id, &state_dir, checks);
        }
    }

    let ticked = wait_for_tick(&state_dir, routine_id);
    if !ticked {
        restore_interval(prev_interval.as_deref());
        let _ = routine_runner::cmd_runner_stop(&state_dir, Some("default"));
        push_fail(&mut checks, "first-tick", "runner never ticked standing routine");
        return finish(routine_id, &session_id, &reused_session_id, &fresh_session_id, &state_dir, checks);
    }
    push_ok(&mut checks, "first-tick", "digest appended");

    match inspect_chain_hops(&state_dir, routine_id) {
        Ok(hops) => {
            session_id = hops.session_id.clone();
            push_check(
                &mut checks,
                "hop1-context-none",
                hops.hop1_context == Some(false),
                format!("context={:?}", hops.hop1_context),
            );
            push_check(
                &mut checks,
                "hop2-context-applied",
                hops.hop2_context == Some(true),
                format!("context={:?}", hops.hop2_context),
            );
            push_check(
                &mut checks,
                "same-session",
                hops.same_session && !hops.session_id.is_empty(),
                hops.session_id.clone(),
            );
        }
        Err(err) => push_fail(&mut checks, "first-tick-hops", err),
    }

    let digest = crate::ops::routine_digest_text(Some(routine_id), estate_path, &state_dir)
        .unwrap_or_default();
    push_check(
        &mut checks,
        "digest-cites-session",
        digest.contains("session_id=") && digest.contains("context=applied"),
        digest.lines().next().unwrap_or(&digest).to_string(),
    );
    let status = routine_runner::status_text(&state_dir, "default", routines::now_unix());
    push_check(
        &mut checks,
        "status-cites-session",
        status.contains("session_id=") && status.contains("context=applied"),
        status.lines().find(|l| l.contains("last_digest")).unwrap_or(&status).to_string(),
    );
    push_check(
        &mut checks,
        "no-secrets",
        !digest.contains("XAI_API_KEY") && !status.contains("XAI_API_KEY"),
        "digest/status have no secret tokens",
    );
    push_check(
        &mut checks,
        "no-live-pass",
        !digest.contains("live PASS") && !status.contains("READY_FOR_LIVE_TEST: yes"),
        "no LIVE PASS wording",
    );

    if let Err(err) = routine_runner::cmd_runner_stop(&state_dir, Some("default")) {
        push_fail(&mut checks, "runner-stop", err.to_string());
    } else {
        push_ok(&mut checks, "runner-stop", "stopped");
    }

    if let Err(err) = force_due(&state_dir, routine_id) {
        push_fail(&mut checks, "reuse-force-due", err.to_string());
    }
    match crate::ops::cmd_routine_tick(
        Some(routine_id),
        None,
        None,
        None,
        estate_path,
        &state_dir,
        None,
        None,
        true,
        false,
        false,
    ) {
        Ok(()) => match inspect_latest_session(&state_dir, routine_id) {
            Ok(sid) => {
                reused_session_id = sid.clone();
                push_check(
                    &mut checks,
                    "session-reuse",
                    !session_id.is_empty() && sid == session_id,
                    format!("first={session_id} reuse={sid}"),
                );
            }
            Err(err) => push_fail(&mut checks, "session-reuse", err),
        },
        Err(err) => push_fail(&mut checks, "session-reuse-tick", err.to_string()),
    }

    if !session_id.is_empty() {
        if let Err(err) = crate::pack_session::cmd_pack_session_end(
            &session_id,
            None,
            &state_dir,
        ) {
            push_fail(&mut checks, "session-end", err.to_string());
        } else {
            push_ok(&mut checks, "session-end", session_id.clone());
        }
    }
    if let Err(err) = force_due(&state_dir, routine_id) {
        push_fail(&mut checks, "fresh-force-due", err.to_string());
    }
    match crate::ops::cmd_routine_tick(
        Some(routine_id),
        None,
        None,
        None,
        estate_path,
        &state_dir,
        None,
        None,
        true,
        false,
        false,
    ) {
        Ok(()) => match inspect_latest_session(&state_dir, routine_id) {
            Ok(sid) => {
                fresh_session_id = sid.clone();
                push_check(
                    &mut checks,
                    "ended-creates-fresh",
                    !sid.is_empty() && sid != session_id,
                    format!("ended={session_id} fresh={sid}"),
                );
            }
            Err(err) => push_fail(&mut checks, "ended-creates-fresh", err),
        },
        Err(err) => push_fail(&mut checks, "ended-fresh-tick", err.to_string()),
    }

    match routine_runner::cmd_runner_start(watch_args(routine_id, estate_path, &state_dir, 30)) {
        Ok(()) => {
            let second = routine_runner::cmd_runner_start(watch_args(
                routine_id,
                estate_path,
                &state_dir,
                30,
            ));
            match second {
                Err(err) => {
                    let text = err.to_string();
                    push_check(
                        &mut checks,
                        "double-start-refuse",
                        text.contains("refuse:runner-already-running"),
                        text,
                    );
                }
                Ok(()) => push_fail(
                    &mut checks,
                    "double-start-refuse",
                    "second start succeeded",
                ),
            }
            let _ = routine_runner::cmd_runner_stop(&state_dir, Some("default"));
        }
        Err(err) => push_fail(&mut checks, "double-start-setup", err.to_string()),
    }

    restore_interval(prev_interval.as_deref());
    finish(routine_id, &session_id, &reused_session_id, &fresh_session_id, &state_dir, checks)
}

fn finish(
    routine_id: &str,
    session_id: &str,
    reused_session_id: &str,
    fresh_session_id: &str,
    state_dir: &Path,
    checks: Vec<ProveCheck>,
) -> Result<()> {
    let ok = checks.iter().all(|c| c.ok);
    let report = ProveReport {
        ok,
        routine_id: routine_id.to_string(),
        session_id: session_id.to_string(),
        reused_session_id: reused_session_id.to_string(),
        fresh_session_id: fresh_session_id.to_string(),
        state_dir: state_dir.display().to_string(),
        checks,
    };
    print_report(&report);
    if !report.ok {
        bail!("refuse:runner-prove: one or more checks failed");
    }
    Ok(())
}

fn print_report(report: &ProveReport) {
    println!("routine runner-prove: {}", report.routine_id);
    println!("  ok: {}", if report.ok { "yes" } else { "no" });
    if !report.session_id.is_empty() {
        println!("  session: {}", report.session_id);
    }
    if !report.reused_session_id.is_empty() {
        println!("  reused: {}", report.reused_session_id);
    }
    if !report.fresh_session_id.is_empty() {
        println!("  fresh: {}", report.fresh_session_id);
    }
    println!("  state_dir: {}", report.state_dir);
    for c in &report.checks {
        let mark = if c.ok { "ok" } else { "FAIL" };
        println!("  {}: {mark} ({})", c.name, c.detail);
    }
    println!("  live_sync: no");
    println!("  READY_FOR_LIVE_TEST: no");
    println!("{}", report.to_json());
}

fn watch_args(
    routine_id: &str,
    estate: &Path,
    state_dir: &Path,
    max_cycles: u32,
) -> RunnerWatchArgs {
    RunnerWatchArgs {
        runner_id: "default".into(),
        routine_id: Some(routine_id.to_string()),
        agent: None,
        prompt: None,
        text: None,
        estate: estate.to_path_buf(),
        state_dir: state_dir.to_path_buf(),
        feed_dir: None,
        endpoint: None,
        mock: true,
        chain: false,
        interval: "5m".into(),
        max_cycles: Some(max_cycles),
    }
}

fn wait_for_tick(state_dir: &Path, routine_id: &str) -> bool {
    let digest_log = routine_runner::digest_log_path(state_dir, "default");
    let start = Instant::now();
    while start.elapsed() < TICK_WAIT {
        if let Ok(text) = std::fs::read_to_string(&digest_log) {
            if text.contains(routine_id) && text.contains("session_id=") {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

struct HopInspect {
    session_id: String,
    same_session: bool,
    hop1_context: Option<bool>,
    hop2_context: Option<bool>,
}

fn inspect_chain_hops(state_dir: &Path, routine_id: &str) -> Result<HopInspect, String> {
    let rows = load_routine_receipts(state_dir, routine_id)?;
    if rows.len() < 2 {
        return Err(format!("want ≥2 hop receipts, got {}", rows.len()));
    }
    let hop1 = &rows[0];
    let hop2 = &rows[1];
    let sid1 = hop1.session_id.clone().unwrap_or_default();
    let sid2 = hop2.session_id.clone().unwrap_or_default();
    Ok(HopInspect {
        same_session: !sid1.is_empty() && sid1 == sid2,
        session_id: sid1,
        hop1_context: hop1.session_context,
        hop2_context: hop2.session_context,
    })
}

fn inspect_latest_session(state_dir: &Path, routine_id: &str) -> Result<String, String> {
    let rows = load_routine_receipts(state_dir, routine_id)?;
    rows.last()
        .and_then(|r| r.session_id.clone())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "latest receipt has no session_id".into())
}

fn load_routine_receipts(state_dir: &Path, routine_id: &str) -> Result<Vec<DecisionReceipt>, String> {
    let want = estate_schema::normalize_name(routine_id);
    let rows = crate::decisions::load_receipts(state_dir).map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .filter(|r| {
            r.routine_id
                .as_deref()
                .map(|id| estate_schema::normalize_name(id) == want)
                .unwrap_or(false)
        })
        .collect())
}

fn force_due(state_dir: &Path, routine_id: &str) -> Result<()> {
    let mut file = routines::load_state(state_dir)?;
    let want = estate_schema::normalize_name(routine_id);
    let key = file
        .routines
        .keys()
        .find(|k| estate_schema::normalize_name(k) == want)
        .cloned();
    if let Some(key) = key {
        if let Some(row) = file.routines.get_mut(&key) {
            row.next_due = Some(1);
        }
    }
    routines::save_state(state_dir, &file)?;
    Ok(())
}

fn locked_estate_ok() -> bool {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new("cksum")
        .arg(root.join(LOCKED_ESTATE))
        .output();
    match out {
        Ok(out) => String::from_utf8_lossy(&out.stdout).starts_with(LOCKED_CKSUM),
        Err(_) => false,
    }
}

fn throwaway_state(routine_id: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("cell-routine-runner-prove-{routine_id}-{nanos}"))
}

fn restore_interval(prev: Option<&str>) {
    match prev {
        Some(v) => std::env::set_var(WATCH_INTERVAL_ENV, v),
        None => {
            let _ = std::env::remove_var(WATCH_INTERVAL_ENV);
        }
    }
}

fn push_ok(checks: &mut Vec<ProveCheck>, name: &str, detail: impl Into<String>) {
    checks.push(ProveCheck {
        name: name.into(),
        ok: true,
        detail: detail.into(),
    });
}

fn push_fail(checks: &mut Vec<ProveCheck>, name: &str, detail: impl Into<String>) {
    checks.push(ProveCheck {
        name: name.into(),
        ok: false,
        detail: detail.into(),
    });
}

fn push_check(checks: &mut Vec<ProveCheck>, name: &str, ok: bool, detail: impl Into<String>) {
    checks.push(ProveCheck {
        name: name.into(),
        ok,
        detail: detail.into(),
    });
}

struct RunnerGuard {
    state_dir: PathBuf,
}

impl Drop for RunnerGuard {
    fn drop(&mut self) {
        let _ = routine_runner::cmd_runner_stop(&self.state_dir, Some("default"));
    }
}

#[allow(dead_code)]
pub(crate) fn default_routine() -> &'static str {
    DEFAULT_ROUTINE
}
