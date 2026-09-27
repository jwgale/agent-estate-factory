//! Supervised standing-routine host runner.
//!
//! `estate routine runner start` detaches the same idempotent watch/tick
//! loop so the invoking terminal can exit. Pidfile + status live under
//! the estate state-dir (throwaway-safe, not estate SoT). Digest text
//! appends each cycle. Not a cloud cron. Not systemd install. Not live
//! Grok Bot sync. `live_sync` stays false. READY_FOR_LIVE_TEST: no.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::routines::{
    format_unix, resolve_watch_plan, run_watch_loop, WatchPlan, WatchStopReason, WATCH_INTERVAL_ENV,
};

pub(crate) const RUNNER_SCHEMA: &str = "cell-one.routine-runner.v0";
pub(crate) const RUNNER_DIR: &str = "routine-runner";
pub(crate) const DEFAULT_RUNNER_ID: &str = "default";
const ID_MAX: usize = 64;
const START_WAIT_MS: u64 = 50;
const START_WAIT_ITERS: u32 = 80;
const STOP_WAIT_MS: u64 = 100;
const STOP_WAIT_ITERS: u32 = 100;
const STOP_KILL_ITERS: u32 = 20;

static RUNNER_STOP: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
pub(crate) struct RunnerWatchArgs {
    pub runner_id: String,
    /// Empty = all enabled standing routines (same as tick default).
    pub routine_ids: Vec<String>,
    pub agent: Option<String>,
    pub prompt: Option<String>,
    pub text: Option<String>,
    pub estate: PathBuf,
    pub state_dir: PathBuf,
    pub feed_dir: Option<PathBuf>,
    pub endpoint: Option<String>,
    pub mock: bool,
    pub chain: bool,
    pub interval: String,
    pub max_cycles: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RunnerRecord {
    pub schema: String,
    pub id: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub started_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_tick: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_digest: Option<String>,
    #[serde(default)]
    pub cycles: u32,
    pub interval_secs: i64,
    pub interval_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estate: Option<String>,
    /// Empty = all enabled. Named ids are the start select.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routine_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routine_id: Option<String>,
    /// Per-id last tick outcome (`ran` / `skipped`) from the digest.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub last_outcomes: BTreeMap<String, String>,
    #[serde(default)]
    pub live_sync: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunnerLiveness {
    Running { pid: u32 },
    Stopped,
}

impl RunnerRecord {
    fn new_running(id: &str, pid: u32, plan: &WatchPlan, args: &RunnerWatchArgs, now: i64) -> Self {
        Self {
            schema: RUNNER_SCHEMA.into(),
            id: id.into(),
            status: "running".into(),
            pid: Some(pid),
            started_at: now,
            last_tick: None,
            last_digest: None,
            cycles: 0,
            interval_secs: plan.interval_secs,
            interval_label: plan.interval_label.clone(),
            estate: Some(args.estate.display().to_string()),
            routine_ids: args.routine_ids.clone(),
            routine_id: args.routine_ids.first().cloned(),
            last_outcomes: BTreeMap::new(),
            live_sync: false,
            stopped_at: None,
            stop_reason: None,
        }
    }
}

pub(crate) fn normalize_runner_id(raw: Option<&str>) -> Result<String, String> {
    let id = raw
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_RUNNER_ID);
    if id.len() > ID_MAX {
        return Err(format!(
            "refuse:runner-id: '{id}' is longer than {ID_MAX} characters"
        ));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "refuse:runner-id: '{id}' must be [A-Za-z0-9_-] (1-{ID_MAX})"
        ));
    }
    Ok(id.to_string())
}

pub(crate) fn runner_dir(state_dir: &Path) -> PathBuf {
    state_dir.join(RUNNER_DIR)
}

pub(crate) fn pidfile_path(state_dir: &Path, id: &str) -> PathBuf {
    runner_dir(state_dir).join(format!("{id}.pid"))
}

pub(crate) fn record_path(state_dir: &Path, id: &str) -> PathBuf {
    runner_dir(state_dir).join(format!("{id}.json"))
}

pub(crate) fn digest_log_path(state_dir: &Path, id: &str) -> PathBuf {
    runner_dir(state_dir).join(format!("{id}.digest.log"))
}

pub(crate) fn out_log_path(state_dir: &Path, id: &str) -> PathBuf {
    runner_dir(state_dir).join(format!("{id}.out.log"))
}

pub(crate) fn digest_cite(digest: &str) -> String {
    digest
        .lines()
        .next()
        .unwrap_or(digest)
        .trim()
        .to_string()
}

pub(crate) fn format_uptime(started_at: i64, now: i64) -> String {
    let secs = now.saturating_sub(started_at).max(0);
    if secs >= 3600 {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{}m{}s", secs / 60, secs % 60)
    } else {
        format!("{secs}s")
    }
}

pub(crate) fn pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        let rc = unsafe { unix_kill(pid as i32, 0) };
        rc == 0
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub(crate) fn looks_like_runner(pid: u32, runner_id: &str) -> bool {
    let path = PathBuf::from(format!("/proc/{pid}/cmdline"));
    let Ok(bytes) = fs::read(&path) else {
        return pid_alive(pid);
    };
    if bytes.is_empty() {
        return pid_alive(pid);
    }
    let text = String::from_utf8_lossy(&bytes);
    let has_supervise = text.contains("supervise") && text.contains("runner");
    if !has_supervise {
        return false;
    }
    if runner_id == DEFAULT_RUNNER_ID {
        return true;
    }
    text.contains(runner_id)
}

pub(crate) fn inspect_liveness(state_dir: &Path, id: &str) -> RunnerLiveness {
    let pid = match read_pidfile(state_dir, id) {
        Some(pid) => pid,
        None => {
            if let Some(rec) = load_record(state_dir, id) {
                if rec.status == "running" {
                    if let Some(pid) = rec.pid {
                        if pid_alive(pid) && looks_like_runner(pid, id) {
                            return RunnerLiveness::Running { pid };
                        }
                    }
                }
            }
            return RunnerLiveness::Stopped;
        }
    };
    if pid_alive(pid) && looks_like_runner(pid, id) {
        RunnerLiveness::Running { pid }
    } else {
        RunnerLiveness::Stopped
    }
}

fn read_pidfile(state_dir: &Path, id: &str) -> Option<u32> {
    let path = pidfile_path(state_dir, id);
    let text = fs::read_to_string(path).ok()?;
    text.trim().parse::<u32>().ok().filter(|p| *p > 0)
}

fn write_pidfile_exclusive(state_dir: &Path, id: &str, pid: u32) -> Result<()> {
    fs::create_dir_all(runner_dir(state_dir))?;
    let path = pidfile_path(state_dir, id);
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            file.write_all(format!("{pid}\n").as_bytes())
                .with_context(|| format!("write {}", path.display()))?;
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            if matches!(inspect_liveness(state_dir, id), RunnerLiveness::Running { .. }) {
                let live = read_pidfile(state_dir, id).unwrap_or(0);
                bail!("refuse:runner-already-running: id={id} pid={live}");
            }
            let _ = fs::remove_file(&path);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .with_context(|| format!("replace {}", path.display()))?;
            file.write_all(format!("{pid}\n").as_bytes())?;
            Ok(())
        }
        Err(err) => Err(err).with_context(|| format!("create {}", path.display())),
    }
}

fn remove_pidfile(state_dir: &Path, id: &str) {
    let _ = fs::remove_file(pidfile_path(state_dir, id));
}

pub(crate) fn load_record(state_dir: &Path, id: &str) -> Option<RunnerRecord> {
    let path = record_path(state_dir, id);
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn save_record(state_dir: &Path, rec: &RunnerRecord) -> Result<()> {
    fs::create_dir_all(runner_dir(state_dir))?;
    let path = record_path(state_dir, &rec.id);
    let body = serde_json::to_string_pretty(rec)?;
    fs::write(&path, format!("{body}\n"))?;
    Ok(())
}

pub(crate) fn append_digest_log(
    state_dir: &Path,
    id: &str,
    cycle: u32,
    now: i64,
    digest: &str,
) -> Result<()> {
    fs::create_dir_all(runner_dir(state_dir))?;
    let path = digest_log_path(state_dir, id);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("append {}", path.display()))?;
    write!(
        file,
        "# ts={} cycle={cycle}\n{digest}",
        format_unix(Some(now))
    )?;
    if !digest.ends_with('\n') {
        file.write_all(b"\n")?;
    }
    Ok(())
}

pub(crate) fn status_text(state_dir: &Path, id: &str, now: i64) -> String {
    let live = inspect_liveness(state_dir, id);
    let rec = load_record(state_dir, id);
    match (live, rec) {
        (RunnerLiveness::Running { pid }, Some(rec)) => {
            format!(
                "routine runner id={id}\n{}",
                status_body(
                    Some(&rec),
                    "running",
                    &pid.to_string(),
                    &format_uptime(rec.started_at, now)
                )
            )
        }
        (RunnerLiveness::Running { pid }, None) => {
            format!(
                "routine runner id={id}\n{}",
                status_body(None, "running", &pid.to_string(), "-")
            )
        }
        (RunnerLiveness::Stopped, Some(rec)) => {
            let pid = rec
                .pid
                .map(|p| p.to_string())
                .unwrap_or_else(|| "-".into());
            format!(
                "routine runner id={id}\n{}",
                status_body(Some(&rec), "stopped", &pid, "-")
            )
        }
        (RunnerLiveness::Stopped, None) => {
            format!(
                "routine runner id={id}\n{}",
                status_body(None, "stopped", "-", "-")
            )
        }
    }
}

fn status_body(rec: Option<&RunnerRecord>, status: &str, pid: &str, uptime: &str) -> String {
    let last_tick = rec
        .map(|r| format_unix(r.last_tick))
        .unwrap_or_else(|| "-".into());
    let last_digest = rec
        .and_then(|r| r.last_digest.clone())
        .unwrap_or_else(|| "-".into());
    let cycles = rec.map(|r| r.cycles).unwrap_or(0);
    let interval = rec
        .map(|r| r.interval_label.clone())
        .unwrap_or_else(|| "-".into());
    format!(
        "  status: {status}\n  pid: {pid}\n  uptime: {uptime}\n{}  last_tick: {last_tick}\n  last_digest: {last_digest}\n  cycles: {cycles}\n  interval: {interval}\n  live_sync: false\n",
        selected_block(rec)
    )
}

fn selected_block(rec: Option<&RunnerRecord>) -> String {
    let Some(rec) = rec else {
        return "  selected: -\n".into();
    };
    let selected = if rec.routine_ids.is_empty() {
        "all".to_string()
    } else {
        rec.routine_ids.join(",")
    };
    let mut out = format!("  selected: {selected}\n");
    let ids: Vec<String> = if rec.routine_ids.is_empty() {
        rec.last_outcomes.keys().cloned().collect()
    } else {
        rec.routine_ids.clone()
    };
    for id in ids {
        let outcome = rec
            .last_outcomes
            .get(&id)
            .map(|s| s.as_str())
            .unwrap_or("-");
        out.push_str(&format!("  last_outcome {id}: {outcome}\n"));
    }
    out
}

pub(crate) fn parse_digest_outcomes(digest: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in digest.lines() {
        let line = line.trim();
        let mut parts = line.split_whitespace();
        let Some(id) = parts.next() else {
            continue;
        };
        let Some(status) = parts.next() else {
            continue;
        };
        let Some(outcome) = status.strip_prefix("status=") else {
            continue;
        };
        out.insert(id.to_string(), outcome.to_string());
    }
    out
}

pub(crate) fn selected_label(ids: &[String]) -> String {
    if ids.is_empty() {
        "all".into()
    } else {
        ids.join(",")
    }
}

fn refuse_already_running(state_dir: &Path, id: &str) -> Result<()> {
    if let RunnerLiveness::Running { pid } = inspect_liveness(state_dir, id) {
        bail!("refuse:runner-already-running: id={id} pid={pid}");
    }
    Ok(())
}

fn plan_from_args(args: &RunnerWatchArgs) -> Result<WatchPlan> {
    let env_override = std::env::var(WATCH_INTERVAL_ENV).ok();
    resolve_watch_plan(
        &args.interval,
        args.max_cycles,
        env_override.as_deref(),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}

pub(crate) fn cmd_runner_start(args: RunnerWatchArgs) -> Result<()> {
    let id = normalize_runner_id(Some(args.runner_id.as_str())).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut args = args;
    args.runner_id = id.clone();
    let _plan = plan_from_args(&args)?;
    let estate = estate_schema::load_estate(&args.estate)
        .with_context(|| format!("load {}", args.estate.display()))?;
    args.routine_ids = crate::routines::resolve_runner_routines(&estate, &args.routine_ids)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    refuse_already_running(&args.state_dir, &id)?;
    fs::create_dir_all(runner_dir(&args.state_dir))?;
    let mut child = spawn_supervise(&args)?;
    for _ in 0..START_WAIT_ITERS {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                let log = fs::read_to_string(out_log_path(&args.state_dir, &id)).unwrap_or_default();
                if let Some(line) = last_refuse_line(&log) {
                    bail!("{line}");
                }
                bail!("refuse:runner-start: supervise exited");
            }
            // Finished before the parent polled (max-cycles / interval 0).
            print!("{}", status_text(&args.state_dir, &id, crate::routines::now_unix()));
            return Ok(());
        }
        if matches!(
            inspect_liveness(&args.state_dir, &id),
            RunnerLiveness::Running { .. }
        ) {
            let rec = load_record(&args.state_dir, &id);
            let pid = rec.as_ref().and_then(|r| r.pid).unwrap_or(child.id());
            let interval = rec
                .as_ref()
                .map(|r| r.interval_label.clone())
                .unwrap_or_else(|| args.interval.clone());
            println!(
                "routine runner started id={id} pid={pid} interval={interval} selected={} live_sync=false",
                selected_label(&args.routine_ids)
            );
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(START_WAIT_MS));
    }
    if matches!(
        inspect_liveness(&args.state_dir, &id),
        RunnerLiveness::Running { .. }
    ) {
        println!(
            "routine runner started id={id} pid={} interval={} selected={} live_sync=false",
            child.id(),
            args.interval,
            selected_label(&args.routine_ids)
        );
        return Ok(());
    }
    let log = fs::read_to_string(out_log_path(&args.state_dir, &id)).unwrap_or_default();
    if let Some(line) = last_refuse_line(&log) {
        bail!("{line}");
    }
    bail!("refuse:runner-start: supervise did not publish a pidfile");
}

pub(crate) fn cmd_runner_stop(state_dir: &Path, runner_id: Option<&str>) -> Result<()> {
    let id = normalize_runner_id(runner_id).map_err(|e| anyhow::anyhow!("{e}"))?;
    match inspect_liveness(state_dir, &id) {
        RunnerLiveness::Running { pid } => {
            send_term(pid);
            if !wait_dead(pid, STOP_WAIT_ITERS) {
                send_kill(pid);
                let _ = wait_dead(pid, STOP_KILL_ITERS);
            }
            mark_stopped(state_dir, &id, pid, "operator")?;
            println!("routine runner stopped id={id} pid={pid}");
            Ok(())
        }
        RunnerLiveness::Stopped => {
            if let Some(mut rec) = load_record(state_dir, &id) {
                if rec.status != "stopped" {
                    rec.status = "stopped".into();
                    rec.stopped_at = Some(crate::routines::now_unix());
                    rec.stop_reason = Some("not-running".into());
                    save_record(state_dir, &rec)?;
                }
            }
            remove_pidfile(state_dir, &id);
            bail!("refuse:runner-not-running: id={id}");
        }
    }
}

pub(crate) fn cmd_runner_status(state_dir: &Path, runner_id: Option<&str>) -> Result<()> {
    let id = normalize_runner_id(runner_id).map_err(|e| anyhow::anyhow!("{e}"))?;
    if inspect_liveness(state_dir, &id) == RunnerLiveness::Stopped {
        if let Some(mut rec) = load_record(state_dir, &id) {
            if rec.status == "running" {
                rec.status = "stopped".into();
                rec.stop_reason = rec
                    .stop_reason
                    .clone()
                    .or_else(|| Some("stale".into()));
                save_record(state_dir, &rec)?;
            }
        }
        remove_pidfile(state_dir, &id);
    }
    print!("{}", status_text(state_dir, &id, crate::routines::now_unix()));
    Ok(())
}

pub(crate) fn cmd_runner_restart(args: RunnerWatchArgs) -> Result<()> {
    let id = normalize_runner_id(Some(args.runner_id.as_str())).map_err(|e| anyhow::anyhow!("{e}"))?;
    if matches!(
        inspect_liveness(&args.state_dir, &id),
        RunnerLiveness::Running { .. }
    ) {
        let _ = cmd_runner_stop(&args.state_dir, Some(&id));
    }
    cmd_runner_start(args)
}

pub(crate) fn cmd_runner_supervise(args: RunnerWatchArgs) -> Result<()> {
    let id = normalize_runner_id(Some(args.runner_id.as_str())).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut args = args;
    args.runner_id = id.clone();
    let plan = plan_from_args(&args)?;
    let estate = estate_schema::load_estate(&args.estate)
        .with_context(|| format!("load {}", args.estate.display()))?;
    args.routine_ids = crate::routines::resolve_runner_routines(&estate, &args.routine_ids)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    detach_session();
    install_runner_signals();
    refuse_already_running(&args.state_dir, &id)?;
    let pid = std::process::id();
    write_pidfile_exclusive(&args.state_dir, &id, pid)?;
    let now = crate::routines::now_unix();
    let mut rec = RunnerRecord::new_running(&id, pid, &plan, &args, now);
    save_record(&args.state_dir, &rec)?;
    println!(
        "routine runner supervise id={id} pid={pid} interval={} selected={} live_sync=false",
        plan.interval_label,
        selected_label(&args.routine_ids)
    );
    let outcome = run_watch_loop(
        &plan,
        |cycle| -> Result<()> {
            crate::ops::cmd_routine_tick(
                &args.routine_ids,
                args.agent.as_deref(),
                args.prompt.clone(),
                args.text.clone(),
                &args.estate,
                &args.state_dir,
                args.feed_dir.as_deref(),
                args.endpoint.clone(),
                args.mock,
                args.chain,
                false,
            )?;
            let digest = crate::ops::routine_digest_text(
                &args.routine_ids,
                &args.estate,
                &args.state_dir,
            )?;
            print!("{digest}");
            let tick_now = crate::routines::now_unix();
            append_digest_log(&args.state_dir, &id, cycle, tick_now, &digest)?;
            rec.last_tick = Some(tick_now);
            rec.last_digest = Some(digest_cite(&digest));
            rec.last_outcomes = parse_digest_outcomes(&digest);
            rec.cycles = cycle;
            save_record(&args.state_dir, &rec)?;
            Ok(())
        },
        sleep_runner_interval,
        runner_stopped,
    )?;
    rec.status = "stopped".into();
    rec.stopped_at = Some(crate::routines::now_unix());
    rec.stop_reason = Some(match outcome.reason {
        WatchStopReason::MaxCycles => "max-cycles".into(),
        WatchStopReason::Signal => "signal".into(),
    });
    save_record(&args.state_dir, &rec)?;
    remove_pidfile(&args.state_dir, &id);
    println!(
        "routine runner stopped cycles={} reason={} live_sync=false",
        outcome.cycles,
        rec.stop_reason.as_deref().unwrap_or("-")
    );
    Ok(())
}

fn spawn_supervise(args: &RunnerWatchArgs) -> Result<std::process::Child> {
    let exe = std::env::current_exe().context("refuse:runner-start: cannot resolve estate binary")?;
    let out_path = out_log_path(&args.state_dir, &args.runner_id);
    let out = fs::File::create(&out_path)
        .with_context(|| format!("create {}", out_path.display()))?;
    let err = out
        .try_clone()
        .with_context(|| format!("clone {}", out_path.display()))?;
    let mut cmd = Command::new(exe);
    cmd.arg("routine")
        .arg("runner")
        .arg("supervise")
        .arg("--runner-id")
        .arg(&args.runner_id)
        .arg("--estate")
        .arg(&args.estate)
        .arg("--state-dir")
        .arg(&args.state_dir)
        .arg("--interval")
        .arg(&args.interval);
    for id in &args.routine_ids {
        cmd.arg("--id").arg(id);
    }
    if let Some(agent) = &args.agent {
        cmd.arg("--agent").arg(agent);
    }
    if let Some(prompt) = &args.prompt {
        cmd.arg("--prompt").arg(prompt);
    }
    if let Some(text) = &args.text {
        cmd.arg("--text").arg(text);
    }
    if let Some(feed) = &args.feed_dir {
        cmd.arg("--feed-dir").arg(feed);
    }
    if let Some(endpoint) = &args.endpoint {
        cmd.arg("--endpoint").arg(endpoint);
    }
    if args.mock {
        cmd.arg("--mock");
    }
    if args.chain {
        cmd.arg("--chain");
    }
    if let Some(n) = args.max_cycles {
        cmd.arg("--max-cycles").arg(n.to_string());
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err));
    cmd.spawn()
        .context("refuse:runner-start: spawn supervise")
}

fn last_refuse_line(log: &str) -> Option<String> {
    log.lines()
        .rev()
        .find(|line| line.contains("refuse:"))
        .map(|s| s.trim().to_string())
}

fn mark_stopped(state_dir: &Path, id: &str, pid: u32, reason: &str) -> Result<()> {
    let now = crate::routines::now_unix();
    if let Some(mut rec) = load_record(state_dir, id) {
        rec.status = "stopped".into();
        rec.pid = Some(pid);
        rec.stopped_at = Some(now);
        rec.stop_reason = Some(reason.into());
        save_record(state_dir, &rec)?;
    }
    remove_pidfile(state_dir, id);
    Ok(())
}

fn send_term(pid: u32) {
    #[cfg(unix)]
    unsafe {
        let _ = unix_kill(pid as i32, 15);
    }
}

fn send_kill(pid: u32) {
    #[cfg(unix)]
    unsafe {
        let _ = unix_kill(pid as i32, 9);
    }
}

fn wait_dead(pid: u32, iters: u32) -> bool {
    for _ in 0..iters {
        if !pid_alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(STOP_WAIT_MS));
    }
    !pid_alive(pid)
}

fn detach_session() {
    #[cfg(unix)]
    unsafe {
        let _ = unix_setsid();
    }
}

fn install_runner_signals() {
    RUNNER_STOP.store(false, Ordering::SeqCst);
    #[cfg(unix)]
    {
        extern "C" fn handle(_sig: i32) {
            RUNNER_STOP.store(true, Ordering::SeqCst);
        }
        unsafe {
            let _ = unix_signal(2, handle as usize);
            let _ = unix_signal(15, handle as usize);
            let _ = unix_signal(1, 1);
        }
    }
}

fn runner_stopped() -> bool {
    RUNNER_STOP.load(Ordering::SeqCst)
}

fn sleep_runner_interval(secs: i64) {
    if secs <= 0 {
        return;
    }
    let mut left = secs;
    while left > 0 && !runner_stopped() {
        std::thread::sleep(Duration::from_secs(1));
        left -= 1;
    }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "kill"]
    fn unix_kill(pid: i32, sig: i32) -> i32;
    #[link_name = "setsid"]
    fn unix_setsid() -> i32;
    #[link_name = "signal"]
    fn unix_signal(sig: i32, handler: usize) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-routine-runner-{name}-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn runner_id_default_and_refuse() {
        assert_eq!(normalize_runner_id(None).unwrap(), "default");
        assert_eq!(normalize_runner_id(Some("  ")).unwrap(), "default");
        assert_eq!(normalize_runner_id(Some("pack-a")).unwrap(), "pack-a");
        let err = normalize_runner_id(Some("bad id")).unwrap_err();
        assert!(err.contains("refuse:runner-id"), "{err}");
    }

    #[test]
    fn digest_cite_is_first_line() {
        let digest = "routine digest ran=1 skipped=0\n  standing-classify status=ran\n";
        assert_eq!(digest_cite(digest), "routine digest ran=1 skipped=0");
    }

    #[test]
    fn uptime_labels() {
        assert_eq!(format_uptime(100, 112), "12s");
        assert_eq!(format_uptime(100, 160), "1m0s");
        assert_eq!(format_uptime(100, 3700), "1h0m");
    }

    #[test]
    fn missing_runner_is_stopped() {
        let dir = scratch("missing");
        assert_eq!(inspect_liveness(&dir, "default"), RunnerLiveness::Stopped);
        let text = status_text(&dir, "default", 1_700_000_000);
        assert!(text.contains("status: stopped"), "{text}");
        assert!(text.contains("selected: -"), "{text}");
        assert!(text.contains("last_digest: -"), "{text}");
        assert!(text.contains("live_sync: false"), "{text}");
        assert!(!text.contains("live PASS"), "{text}");
        assert!(!text.contains("READY_FOR_LIVE_TEST: yes"), "{text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuse_already_running_uses_this_process_pid_when_cmdline_matches() {
        let dir = scratch("alive-self");
        // This process is not a supervise child, so looks_like_runner is false
        // unless /proc cmdline is missing. The pidfile of an unrelated live
        // pid must not count as the runner.
        fs::create_dir_all(runner_dir(&dir)).unwrap();
        fs::write(pidfile_path(&dir, "default"), format!("{}\n", std::process::id())).unwrap();
        assert_eq!(inspect_liveness(&dir, "default"), RunnerLiveness::Stopped);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_pidfile_is_not_running() {
        let dir = scratch("stale");
        fs::create_dir_all(runner_dir(&dir)).unwrap();
        fs::write(pidfile_path(&dir, "default"), "199999999\n").unwrap();
        assert_eq!(inspect_liveness(&dir, "default"), RunnerLiveness::Stopped);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn exclusive_pidfile_refuses_second_writer_when_alive_and_looks_like_runner() {
        let dir = scratch("exclusive");
        write_pidfile_exclusive(&dir, "default", 7).unwrap();
        assert_eq!(
            fs::read_to_string(pidfile_path(&dir, "default")).unwrap().trim(),
            "7"
        );
        // Stale pid 7 is not alive → replace is allowed.
        write_pidfile_exclusive(&dir, "default", 8).unwrap();
        assert_eq!(
            fs::read_to_string(pidfile_path(&dir, "default")).unwrap().trim(),
            "8"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_digest_reuses_existing_digest_text() {
        let dir = scratch("digest");
        append_digest_log(
            &dir,
            "default",
            1,
            1_700_000_000,
            "routine digest ran=1 skipped=0\n  standing-classify status=ran\n",
        )
        .unwrap();
        let text = fs::read_to_string(digest_log_path(&dir, "default")).unwrap();
        assert!(text.contains("cycle=1"), "{text}");
        assert!(text.contains("routine digest ran=1 skipped=0"), "{text}");
        assert!(text.contains("standing-classify status=ran"), "{text}");
        assert!(!text.contains("XAI_API_KEY"), "{text}");
        assert!(!text.contains("live PASS"), "{text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn interval_label_helper_stays_five_minutes() {
        assert_eq!(
            crate::routines::format_watch_interval_secs(300),
            "5m (300s)"
        );
    }

    #[test]
    fn digest_outcomes_and_status_list_selected_ids() {
        let digest = "routine digest ran=1 skipped=1\n  standing-classify status=ran package=classify-ping\n  standing-once status=skipped package=classify-once reason=not due\n";
        let outcomes = parse_digest_outcomes(digest);
        assert_eq!(outcomes.get("standing-classify").map(String::as_str), Some("ran"));
        assert_eq!(outcomes.get("standing-once").map(String::as_str), Some("skipped"));
        assert_eq!(selected_label(&[]), "all");
        assert_eq!(
            selected_label(&["standing-classify".into(), "standing-once".into()]),
            "standing-classify,standing-once"
        );

        let dir = scratch("status-selected");
        let rec = RunnerRecord {
            schema: RUNNER_SCHEMA.into(),
            id: "default".into(),
            status: "stopped".into(),
            pid: Some(9),
            started_at: 1_700_000_000,
            last_tick: Some(1_700_000_010),
            last_digest: Some("routine digest ran=1 skipped=1".into()),
            cycles: 1,
            interval_secs: 2,
            interval_label: "2s".into(),
            estate: None,
            routine_ids: vec!["standing-classify".into(), "standing-once".into()],
            routine_id: Some("standing-classify".into()),
            last_outcomes: outcomes,
            live_sync: false,
            stopped_at: Some(1_700_000_020),
            stop_reason: Some("max-cycles".into()),
        };
        save_record(&dir, &rec).unwrap();
        let text = status_text(&dir, "default", 1_700_000_030);
        assert!(text.contains("selected: standing-classify,standing-once"), "{text}");
        assert!(text.contains("last_outcome standing-classify: ran"), "{text}");
        assert!(text.contains("last_outcome standing-once: skipped"), "{text}");
        assert!(text.contains("live_sync: false"), "{text}");
        let _ = fs::remove_dir_all(&dir);
    }
}
