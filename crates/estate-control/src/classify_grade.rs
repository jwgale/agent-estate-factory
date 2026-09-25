//! Compile-and-test grader for MultiPL-E Rust and HumanEvalPack Rust.
//!
//! Generation tasks are not FixedClasses A/B letter rows. This command materializes
//! a held-out task file (`prompt`, `tests`, `canonical`) and scores one completion
//! per task by compiling and running it. `--print` is the default and does not
//! compile. `--run` needs a local snapshot or a task file. It does not download
//! and it does not record a live PASS.

use crate::classify::{accuracy_of, wilson_ci95};
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const DEFAULT_GRADE_OUT: &str = ".cell/classify-grade";
static GRADE_DIRS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

const SCHEMA_PLAN: &str = "cell-one.code-grade-plan.v0";
const SCHEMA_REPORT: &str = "cell-one.code-grade.v0";
const READY: &str = "no";

const NOTE_HUMANEVALPACK: &str = "HumanEvalPack Rust (bigcode/humanevalpack, config rust) is MIT and is derived from OpenAI HumanEval (MIT). Local training proof only. Do not redistribute.";
const NOTE_MULTIPLE: &str = "MultiPL-E Rust HumanEval (nuprl/MultiPL-E, humaneval-rs) translates HumanEval. The MultiPL-E repository is BSD-3-Clause. HumanEval is MIT. This preset is that HumanEval Rust translation, not MBPP. Local training proof only. Do not redistribute.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Preset {
    HumanEvalPackRust,
    MultipleRust,
}

impl Preset {
    fn alias(self) -> &'static str {
        match self {
            Self::HumanEvalPackRust => "humanevalpack_rust",
            Self::MultipleRust => "multiple_rust",
        }
    }

    fn hub(self) -> &'static str {
        match self {
            Self::HumanEvalPackRust => "bigcode/humanevalpack",
            Self::MultipleRust => "nuprl/MultiPL-E",
        }
    }

    fn license_note(self) -> &'static str {
        match self {
            Self::HumanEvalPackRust => NOTE_HUMANEVALPACK,
            Self::MultipleRust => NOTE_MULTIPLE,
        }
    }

    fn fetch_hint(self) -> &'static str {
        match self {
            Self::HumanEvalPackRust => "hf download bigcode/humanevalpack --repo-type dataset",
            Self::MultipleRust => "hf download nuprl/MultiPL-E --repo-type dataset",
        }
    }
}

fn preset_by_name(name: &str) -> Result<Preset> {
    match name.trim() {
        "humanevalpack_rust" | "humanevalpack-rust" | "bigcode/humanevalpack" => {
            Ok(Preset::HumanEvalPackRust)
        }
        "multiple_rust" | "multiple-rust" | "multipl-e" | "nuprl/MultiPL-E" => Ok(Preset::MultipleRust),
        other => bail!(
            "refuse:classify-grade: unknown dataset {other}. This command loads humanevalpack_rust (bigcode/humanevalpack) and multiple_rust (nuprl/MultiPL-E humaneval-rs)."
        ),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Harness {
    Rustc,
    CargoTest,
}

impl Harness {
    fn as_str(self) -> &'static str {
        match self {
            Self::Rustc => "rustc",
            Self::CargoTest => "cargo-test",
        }
    }

    fn detect(tests: &str) -> Self {
        if tests.contains("fn main") {
            Self::Rustc
        } else {
            Self::CargoTest
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Task {
    id: String,
    dataset: String,
    prompt: String,
    tests: String,
    entry_point: String,
    canonical: String,
    harness: Harness,
}

pub struct GradeRequest<'a> {
    pub dataset: Option<&'a str>,
    pub from_local: Option<&'a Path>,
    pub tasks: Option<&'a Path>,
    pub completions: Option<&'a Path>,
    pub out: &'a Path,
    pub print: bool,
    pub run: bool,
    pub limit: Option<usize>,
    pub timeout_secs: u64,
    pub use_canonical: bool,
}

pub fn cmd_classify_grade(req: &GradeRequest<'_>) -> Result<()> {
    if req.print && req.run {
        bail!("refuse:classify-grade: pass only one of --print and --run");
    }
    if req.timeout_secs == 0 {
        bail!("refuse:classify-grade: --timeout-secs must be at least 1");
    }
    if req.limit == Some(0) {
        bail!("refuse:classify-grade: --limit must be at least 1");
    }
    if req.use_canonical && req.completions.is_some() {
        bail!("refuse:classify-grade: pass only one of --use-canonical and --completions");
    }
    let preset = match req.dataset {
        Some(name) => Some(preset_by_name(name)?),
        None => None,
    };
    if !req.run {
        write_plan(req, preset)?;
        return Ok(());
    }
    if req.tasks.is_none() && req.from_local.is_none() {
        let hint = preset
            .map(Preset::fetch_hint)
            .unwrap_or("hf download <dataset> --repo-type dataset");
        bail!(
            "refuse:classify-grade: --run needs --from-local or --tasks. This command does not download. Operator fetch: {hint}"
        );
    }
    if req.completions.is_none() && !req.use_canonical {
        bail!("refuse:classify-grade: --run needs --completions or --use-canonical");
    }
    let mut tasks = load_tasks(req.tasks, req.from_local, preset)?;
    if let Some(limit) = req.limit {
        if limit > tasks.len() {
            bail!(
                "refuse:classify-grade: --limit {limit} is larger than the {} tasks",
                tasks.len()
            );
        }
        tasks.truncate(limit);
    }
    let completions = if req.use_canonical {
        canonical_completions(&tasks)?
    } else {
        load_completions(req.completions.unwrap(), &tasks)?
    };
    fs::create_dir_all(req.out)?;
    write_jsonl(
        &req.out.join("tasks.jsonl"),
        &tasks.iter().map(task_json).collect::<Vec<_>>(),
    )?;
    let timeout = Duration::from_secs(req.timeout_secs);
    let mut rows = Vec::with_capacity(tasks.len());
    for task in &tasks {
        let completion = &completions[&task.id];
        rows.push(grade_one(task, completion, timeout)?);
    }
    let passed = rows.iter().filter(|row| row.passed).count();
    let report = json!({
        "schema": SCHEMA_REPORT,
        "mode": "run",
        "dataset": dataset_label(&tasks, preset),
        "harness": harness_label(&tasks),
        "records": rows.len(),
        "passed": passed,
        "correct": passed,
        "failed": rows.len() - passed,
        "invalid": 0,
        "pass_rate": accuracy_of(passed, rows.len()),
        "accuracy": accuracy_of(passed, rows.len()),
        "pass_rate_ci95": ci_json(wilson_ci95(passed as u64, rows.len() as u64)),
        "accuracy_ci95": ci_json(wilson_ci95(passed as u64, rows.len() as u64)),
        "complete": true,
        "limit": req.limit,
        "limit_kind": limit_kind(req.limit),
        "timeout_secs": req.timeout_secs,
        "reference_canonical": req.use_canonical,
        "tasks": rows.iter().map(row_json).collect::<Vec<_>>(),
        "live_pass_recorded": false,
        "ready_for_live_test": READY,
        "license_note": license_label(preset, &tasks),
        "note": "pass_rate is the share of tasks whose completion compiled and passed its tests. The 95% interval is Wilson. Same completions and the same timeout score the same way. This file is not a factory live PASS. READY_FOR_LIVE_TEST stays no."
    });
    write_pretty(&req.out.join("grade-report.json"), &report)?;
    Ok(())
}

fn write_plan(req: &GradeRequest<'_>, preset: Option<Preset>) -> Result<()> {
    let loaded = if req.tasks.is_some() || req.from_local.is_some() {
        Some(load_tasks(req.tasks, req.from_local, preset)?)
    } else {
        None
    };
    let records = loaded.as_ref().map(|rows| rows.len());
    let dataset = preset
        .map(|row| row.alias().to_string())
        .or_else(|| loaded.as_ref().map(|rows| dataset_label(rows, None)));
    let license_note = preset
        .map(|row| row.license_note().to_string())
        .or_else(|| loaded.as_ref().map(|rows| license_label(None, rows)));
    if let Some(limit) = req.limit {
        if let Some(n) = records {
            if limit > n {
                bail!("refuse:classify-grade: --limit {limit} is larger than the {n} tasks");
            }
        }
    }
    let report = json!({
        "schema": SCHEMA_PLAN,
        "mode": "print",
        "dataset": dataset,
        "hub": preset.map(Preset::hub),
        "fetch": preset.map(Preset::fetch_hint),
        "from_local": req.from_local.map(|p| p.display().to_string()),
        "tasks": req.tasks.map(|p| p.display().to_string()),
        "records": records,
        "limit": req.limit,
        "limit_kind": limit_kind(req.limit),
        "would_compile": false,
        "live_pass_recorded": false,
        "ready_for_live_test": READY,
        "license_note": license_note,
        "note": "Print does not download, compile, or record a live PASS. --run reads --from-local or --tasks and scores completions with rustc or cargo test. READY_FOR_LIVE_TEST stays no."
    });
    fs::create_dir_all(req.out)?;
    write_pretty(&req.out.join("grade-plan.json"), &report)?;
    Ok(())
}

fn limit_kind(limit: Option<usize>) -> &'static str {
    if limit.is_some() {
        "stable-id-prefix"
    } else {
        "all"
    }
}

fn dataset_label(tasks: &[Task], preset: Option<Preset>) -> String {
    if let Some(preset) = preset {
        return preset.alias().to_string();
    }
    tasks.first().map(|t| t.dataset.clone()).unwrap_or_default()
}

fn license_label(preset: Option<Preset>, tasks: &[Task]) -> String {
    if let Some(preset) = preset {
        return preset.license_note().to_string();
    }
    match tasks.first().map(|t| t.dataset.as_str()) {
        Some("humanevalpack_rust") => NOTE_HUMANEVALPACK.to_string(),
        Some("multiple_rust") => NOTE_MULTIPLE.to_string(),
        Some(other) => format!("{other} is for local training proof only. Do not redistribute."),
        None => "Local training proof only. Do not redistribute.".into(),
    }
}

fn harness_label(tasks: &[Task]) -> String {
    let mut rustc = false;
    let mut cargo = false;
    for task in tasks {
        match task.harness {
            Harness::Rustc => rustc = true,
            Harness::CargoTest => cargo = true,
        }
    }
    match (rustc, cargo) {
        (true, true) => "mixed".into(),
        (true, false) => "rustc".into(),
        (false, true) => "cargo-test".into(),
        (false, false) => "none".into(),
    }
}

fn load_tasks(
    tasks_path: Option<&Path>,
    from_local: Option<&Path>,
    preset: Option<Preset>,
) -> Result<Vec<Task>> {
    if tasks_path.is_some() && from_local.is_some() {
        bail!("refuse:classify-grade: pass only one of --tasks and --from-local");
    }
    let paths = if let Some(path) = tasks_path.or(from_local) {
        jsonl_paths(path)?
    } else {
        bail!("refuse:classify-grade: no task source");
    };
    let mut tasks = Vec::new();
    for path in paths {
        let file = File::open(&path).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-grade: cannot read {}: {err}",
                path.display()
            )
        })?;
        for (idx, line) in BufReader::new(file).lines().enumerate() {
            let line = line.map_err(|err| {
                anyhow::anyhow!(
                    "refuse:classify-grade: cannot read {}: {err}",
                    path.display()
                )
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(&line).map_err(|err| {
                anyhow::anyhow!(
                    "refuse:classify-grade: {} line {} is not json ({err})",
                    path.display(),
                    idx + 1
                )
            })?;
            let task = task_from_value(&value, preset, &path, idx + 1)?;
            if tasks.iter().any(|row: &Task| row.id == task.id) {
                bail!("refuse:classify-grade: duplicate task id {}", task.id);
            }
            tasks.push(task);
        }
    }
    if tasks.is_empty() {
        bail!("refuse:classify-grade: no tasks");
    }
    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    let dataset = &tasks[0].dataset;
    if tasks.iter().any(|row| row.dataset != *dataset) {
        bail!("refuse:classify-grade: mixed datasets in one grade");
    }
    Ok(tasks)
}

fn jsonl_paths(path: &Path) -> Result<Vec<PathBuf>> {
    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) == Some("parquet") {
            bail!(
                "refuse:classify-grade: {} is parquet. Export JSONL and pass that file.",
                path.display()
            );
        }
        return Ok(vec![path.to_path_buf()]);
    }
    if !path.is_dir() {
        bail!(
            "refuse:classify-grade: task source {} is missing",
            path.display()
        );
    }
    let mut files = Vec::new();
    let mut entries: Vec<PathBuf> = fs::read_dir(path)
        .map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-grade: cannot read {}: {err}",
                path.display()
            )
        })?
        .filter_map(|item| item.ok().map(|item| item.path()))
        .filter(|item| item.is_file())
        .collect();
    entries.sort();
    for item in entries {
        let name = item.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(".jsonl") {
            files.push(item);
        } else if name.ends_with(".parquet") {
            bail!(
                "refuse:classify-grade: {} is parquet. Export JSONL and pass that file.",
                item.display()
            );
        }
    }
    if files.is_empty() {
        bail!("refuse:classify-grade: no JSONL in {}", path.display());
    }
    Ok(files)
}

fn task_from_value(
    value: &Value,
    preset: Option<Preset>,
    path: &Path,
    line: usize,
) -> Result<Task> {
    let obj = value.as_object().ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-grade: {} line {line} is not an object",
            path.display()
        )
    })?;
    if let Some(lang) = obj.get("language").and_then(Value::as_str) {
        let lang = lang.trim().to_ascii_lowercase();
        if lang != "rust" && lang != "rs" {
            bail!(
                "refuse:classify-grade: {} line {line} language is {lang}, expected rust",
                path.display()
            );
        }
    }
    let id = first_str(obj, &["id", "task_id", "name"]).ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-grade: {} line {line} is missing id",
            path.display()
        )
    })?;
    if id.trim().is_empty() {
        bail!(
            "refuse:classify-grade: {} line {line} id is empty",
            path.display()
        );
    }
    let prompt = first_str(obj, &["prompt"]).ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-grade: {} line {line} is missing prompt",
            path.display()
        )
    })?;
    if prompt.trim().is_empty() {
        bail!(
            "refuse:classify-grade: {} line {line} prompt is empty",
            path.display()
        );
    }
    let tests = first_str(obj, &["tests", "test"]).ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-grade: {} line {line} is missing tests",
            path.display()
        )
    })?;
    if tests.trim().is_empty() {
        bail!(
            "refuse:classify-grade: {} line {line} tests are empty",
            path.display()
        );
    }
    let canonical = first_str(obj, &["canonical", "canonical_solution"]).unwrap_or("");
    let entry_point = first_str(obj, &["entry_point"]).unwrap_or("");
    let dataset = obj
        .get("dataset")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| preset.map(|p| p.alias().to_string()))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "refuse:classify-grade: {} line {line} needs --dataset or a dataset field",
                path.display()
            )
        })?;
    if let Some(preset) = preset {
        if dataset != preset.alias() {
            bail!(
                "refuse:classify-grade: {} line {line} dataset is {dataset}, expected {}",
                path.display(),
                preset.alias()
            );
        }
    }
    Ok(Task {
        id: id.to_string(),
        dataset,
        prompt: prompt.to_string(),
        tests: tests.to_string(),
        entry_point: entry_point.to_string(),
        canonical: canonical.to_string(),
        harness: Harness::detect(tests),
    })
}

fn first_str<'a>(obj: &'a serde_json::Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(text) = obj.get(*key).and_then(Value::as_str) {
            return Some(text);
        }
    }
    None
}

fn canonical_completions(tasks: &[Task]) -> Result<std::collections::BTreeMap<String, String>> {
    let mut out = std::collections::BTreeMap::new();
    for task in tasks {
        if task.canonical.trim().is_empty() {
            bail!(
                "refuse:classify-grade: task {} has no canonical completion",
                task.id
            );
        }
        out.insert(task.id.clone(), task.canonical.clone());
    }
    Ok(out)
}

fn load_completions(
    path: &Path,
    tasks: &[Task],
) -> Result<std::collections::BTreeMap<String, String>> {
    let text = fs::read_to_string(path).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-grade: cannot read {}: {err}",
            path.display()
        )
    })?;
    let mut out = std::collections::BTreeMap::new();
    for (idx, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-grade: {} line {} is not json ({err})",
                path.display(),
                idx + 1
            )
        })?;
        let obj = value.as_object().ok_or_else(|| {
            anyhow::anyhow!(
                "refuse:classify-grade: {} line {} is not an object",
                path.display(),
                idx + 1
            )
        })?;
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "refuse:classify-grade: {} line {} is missing id",
                    path.display(),
                    idx + 1
                )
            })?;
        let completion = obj
            .get("completion")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "refuse:classify-grade: {} line {} completion must be a string",
                    path.display(),
                    idx + 1
                )
            })?;
        if out.insert(id.to_string(), completion.to_string()).is_some() {
            bail!("refuse:classify-grade: duplicate completion id {id}");
        }
    }
    for task in tasks {
        if !out.contains_key(&task.id) {
            bail!(
                "refuse:classify-grade: missing completion for task {}",
                task.id
            );
        }
    }
    for id in out.keys() {
        if !tasks.iter().any(|task| task.id == *id) {
            bail!("refuse:classify-grade: completion id {id} is not in the task file");
        }
    }
    Ok(out)
}

struct Graded {
    id: String,
    passed: bool,
    status: &'static str,
    detail: String,
}

fn grade_one(task: &Task, completion: &str, timeout: Duration) -> Result<Graded> {
    let program = assemble(&task.prompt, completion, &task.tests);
    let n = GRADE_DIRS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "estate-grade-{}-{}-{}",
        std::process::id(),
        n,
        sanitize(&task.id)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let result = match task.harness {
        Harness::Rustc => grade_rustc(&dir, &program, timeout),
        Harness::CargoTest => grade_cargo(&dir, &program, timeout),
    };
    let _ = fs::remove_dir_all(&dir);
    let (passed, status, detail) = result?;
    Ok(Graded {
        id: task.id.clone(),
        passed,
        status,
        detail,
    })
}

fn assemble(prompt: &str, completion: &str, tests: &str) -> String {
    let mut program = String::with_capacity(prompt.len() + completion.len() + tests.len() + 2);
    program.push_str(prompt);
    program.push_str(completion);
    if !program.ends_with('\n') {
        program.push('\n');
    }
    program.push_str(tests);
    if !program.ends_with('\n') {
        program.push('\n');
    }
    program
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .take(48)
        .collect()
}

fn grade_rustc(
    dir: &Path,
    program: &str,
    timeout: Duration,
) -> Result<(bool, &'static str, String)> {
    let source = dir.join("task.rs");
    fs::write(&source, program)?;
    let bin = dir.join("task_bin");
    let compile = run_captured(
        Command::new("rustc")
            .arg("--edition")
            .arg("2021")
            .arg("--error-format=short")
            .arg("-o")
            .arg(&bin)
            .arg(&source)
            .current_dir(dir)
            .env_remove("RUSTFLAGS"),
        timeout,
    )?;
    if compile.timed_out {
        return Ok((false, "timeout", "rustc timed out".into()));
    }
    if !compile.success {
        return Ok((false, "compile_error", clip(&compile.stderr)));
    }
    let run = run_captured(
        Command::new(&bin).current_dir(dir).env_remove("RUSTFLAGS"),
        timeout,
    )?;
    if run.timed_out {
        return Ok((false, "timeout", "program timed out".into()));
    }
    if run.success {
        Ok((true, "pass", String::new()))
    } else {
        let detail = if run.stderr.is_empty() {
            clip(&run.stdout)
        } else {
            clip(&run.stderr)
        };
        Ok((false, "fail", detail))
    }
}

fn grade_cargo(
    dir: &Path,
    program: &str,
    timeout: Duration,
) -> Result<(bool, &'static str, String)> {
    fs::create_dir_all(dir.join("src"))?;
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"grade_task\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n",
    )?;
    fs::write(dir.join("src/lib.rs"), program)?;
    let run = run_captured(
        Command::new("cargo")
            .args(["test", "--offline", "--quiet", "--", "--test-threads=1"])
            .current_dir(dir)
            .env_remove("RUSTFLAGS")
            .env("CARGO_NET_OFFLINE", "true")
            .env("CARGO_TERM_COLOR", "never")
            .env("RUST_BACKTRACE", "0"),
        timeout,
    )?;
    if run.timed_out {
        return Ok((false, "timeout", "cargo test timed out".into()));
    }
    if run.success {
        return Ok((true, "pass", String::new()));
    }
    let detail = if run.stderr.is_empty() {
        clip(&run.stdout)
    } else {
        clip(&run.stderr)
    };
    let status = if detail.contains("error[") || detail.contains("error:") {
        "compile_error"
    } else {
        "fail"
    };
    Ok((false, status, detail))
}

struct Captured {
    success: bool,
    stdout: String,
    stderr: String,
    timed_out: bool,
}

fn run_captured(cmd: &mut Command, timeout: Duration) -> Result<Captured> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| anyhow::anyhow!("refuse:classify-grade: cannot start compiler: {err}"))?;
    let mut stdout = child.stdout.take().expect("stdout");
    let mut stderr = child.stderr.take().expect("stderr");
    let out_handle = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let err_handle = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        buf
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() > timeout => {
                kill_group(child.id());
                let _ = child.kill();
                let _ = child.wait();
                let _ = out_handle.join();
                let _ = err_handle.join();
                return Ok(Captured {
                    success: false,
                    stdout: String::new(),
                    stderr: String::new(),
                    timed_out: true,
                });
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(err) => bail!("refuse:classify-grade: compiler wait failed: {err}"),
        }
    };
    let stdout = String::from_utf8_lossy(&out_handle.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&err_handle.join().unwrap_or_default()).into_owned();
    Ok(Captured {
        success: status.success(),
        stdout,
        stderr,
        timed_out: false,
    })
}

fn kill_group(pid: u32) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-KILL", &format!("-{pid}")])
            .output();
    }
    let _ = pid;
}

fn clip(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(400).collect()
}

fn task_json(task: &Task) -> Value {
    json!({
        "id": task.id,
        "dataset": task.dataset,
        "prompt": task.prompt,
        "tests": task.tests,
        "entry_point": task.entry_point,
        "canonical": task.canonical,
        "harness": task.harness.as_str(),
    })
}

fn row_json(row: &Graded) -> Value {
    json!({
        "id": row.id,
        "passed": row.passed,
        "status": row.status,
        "detail": row.detail,
    })
}

fn ci_json(interval: Option<(f64, f64)>) -> Value {
    match interval {
        Some((low, high)) => json!({"low": low, "high": high, "method": "wilson"}),
        None => Value::Null,
    }
}

fn write_jsonl(path: &Path, rows: &[Value]) -> Result<()> {
    let mut file = File::create(path)?;
    for row in rows {
        writeln!(file, "{}", serde_json::to_string(row)?)?;
    }
    Ok(())
}

fn write_pretty(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let pretty = serde_json::to_string_pretty(value)?;
    let mut file = File::create(path)?;
    writeln!(file, "{pretty}")?;
    println!("{pretty}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_pair() -> (String, String) {
        (
            concat!(
                "{\"id\":\"add\",\"dataset\":\"humanevalpack_rust\",\"prompt\":\"fn add(a: i32, b: i32) -> i32 {\\n\",\"tests\":\"fn main() {\\n    assert_eq!(add(1, 2), 3);\\n}\\n\",\"canonical\":\"    a + b\\n}\\n\",\"entry_point\":\"add\"}\n",
            )
            .into(),
            "{\"id\":\"add\",\"completion\":\"    a + b\\n}\\n\"}\n".into(),
        )
    }

    fn write_tasks(dir: &Path, body: &str) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join("tasks.jsonl");
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn fixture_load_sorts_and_detects_harness() {
        let dir = std::env::temp_dir().join(format!("grade-load-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = write_tasks(
            &dir,
            concat!(
                "{\"name\":\"HumanEval_1\",\"language\":\"rust\",\"prompt\":\"fn sub() {}\",\"tests\":\"fn main() {}\"}\n",
                "{\"task_id\":\"Rust/0\",\"prompt\":\"fn add() {}\",\"test\":\"#[test]\\nfn t() {}\",\"canonical_solution\":\"body\"}\n",
            ),
        );
        let err = load_tasks(Some(&path), None, None).unwrap_err().to_string();
        assert!(err.contains("needs --dataset"), "{err}");
        let path = write_tasks(
            &dir,
            concat!(
                "{\"name\":\"b_task\",\"language\":\"rs\",\"prompt\":\"fn sub(a: i32) -> i32 {\\n\",\"tests\":\"fn main() { assert_eq!(sub(1), 0); }\\n\",\"canonical\":\"    a - 1\\n}\\n\"}\n",
                "{\"task_id\":\"a_task\",\"prompt\":\"fn add(a: i32, b: i32) -> i32 {\\n\",\"test\":\"#[cfg(test)]\\nmod tests { use super::*; #[test] fn t() { assert_eq!(add(1, 2), 3); } }\\n\",\"canonical_solution\":\"    a + b\\n}\\n\"}\n",
            ),
        );
        let tasks = load_tasks(Some(&path), None, Some(Preset::HumanEvalPackRust)).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "a_task");
        assert_eq!(tasks[0].harness, Harness::CargoTest);
        assert_eq!(tasks[1].id, "b_task");
        assert_eq!(tasks[1].harness, Harness::Rustc);
        assert_eq!(tasks[0].dataset, "humanevalpack_rust");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn grader_accepts_good_and_rejects_broken() {
        let dir = std::env::temp_dir().join(format!("grade-score-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let (tasks_body, good) = good_pair();
        let tasks_path = write_tasks(&dir, &tasks_body);
        let good_path = dir.join("good.jsonl");
        fs::write(&good_path, good).unwrap();
        let bad_path = dir.join("bad.jsonl");
        fs::write(
            &bad_path,
            "{\"id\":\"add\",\"completion\":\"    a - b\\n}\\n\"}\n",
        )
        .unwrap();
        let broken_path = dir.join("broken.jsonl");
        fs::write(
            &broken_path,
            "{\"id\":\"add\",\"completion\":\"not rust\"}\n",
        )
        .unwrap();
        let out_good = dir.join("out-good");
        cmd_classify_grade(&GradeRequest {
            dataset: Some("humanevalpack_rust"),
            from_local: None,
            tasks: Some(&tasks_path),
            completions: Some(&good_path),
            out: &out_good,
            print: false,
            run: true,
            limit: None,
            timeout_secs: 30,
            use_canonical: false,
        })
        .unwrap();
        let report: Value =
            serde_json::from_str(&fs::read_to_string(out_good.join("grade-report.json")).unwrap())
                .unwrap();
        assert_eq!(report["passed"], 1);
        assert_eq!(report["correct"], 1);
        assert_eq!(report["accuracy"], 1.0);
        assert_eq!(report["pass_rate"], 1.0);
        assert_eq!(report["complete"], true);
        assert_eq!(report["invalid"], 0);
        let side = crate::classify_journey::side_score(&report).unwrap();
        assert_eq!(side.correct, 1);
        assert_eq!(side.records, 1);
        assert_eq!(side.accuracy, 1.0);
        assert_eq!(report["live_pass_recorded"], false);
        assert_eq!(report["ready_for_live_test"], "no");
        assert_eq!(report["pass_rate_ci95"]["method"], "wilson");
        assert!(report["tasks.jsonl"].is_null());
        let materialized: Value = serde_json::from_str(
            &fs::read_to_string(out_good.join("tasks.jsonl"))
                .unwrap()
                .lines()
                .next()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(materialized["harness"], "rustc");
        assert_eq!(materialized["dataset"], "humanevalpack_rust");

        let out_bad = dir.join("out-bad");
        cmd_classify_grade(&GradeRequest {
            dataset: Some("bigcode/humanevalpack"),
            from_local: None,
            tasks: Some(&tasks_path),
            completions: Some(&bad_path),
            out: &out_bad,
            print: false,
            run: true,
            limit: None,
            timeout_secs: 30,
            use_canonical: false,
        })
        .unwrap();
        let bad: Value =
            serde_json::from_str(&fs::read_to_string(out_bad.join("grade-report.json")).unwrap())
                .unwrap();
        assert_eq!(bad["passed"], 0);
        assert_eq!(bad["failed"], 1);
        assert_eq!(bad["tasks"][0]["passed"], false);
        assert_eq!(bad["live_pass_recorded"], false);

        let out_compile = dir.join("out-compile");
        cmd_classify_grade(&GradeRequest {
            dataset: None,
            from_local: None,
            tasks: Some(&tasks_path),
            completions: Some(&broken_path),
            out: &out_compile,
            print: false,
            run: true,
            limit: None,
            timeout_secs: 30,
            use_canonical: false,
        })
        .unwrap();
        let compile: Value = serde_json::from_str(
            &fs::read_to_string(out_compile.join("grade-report.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(compile["tasks"][0]["status"], "compile_error");
        assert_eq!(compile["live_pass_recorded"], false);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cargo_test_harness_passes_and_fails() {
        let dir = std::env::temp_dir().join(format!("grade-cargo-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tasks_path = write_tasks(
            &dir,
            "{\"id\":\"add\",\"dataset\":\"multiple_rust\",\"prompt\":\"fn add(a: i32, b: i32) -> i32 {\\n\",\"tests\":\"#[cfg(test)]\\nmod tests {\\n    use super::*;\\n    #[test]\\n    fn t() { assert_eq!(add(1, 2), 3); }\\n}\\n\",\"canonical\":\"    a + b\\n}\\n\"}\n",
        );
        let out = dir.join("out");
        cmd_classify_grade(&GradeRequest {
            dataset: Some("multiple_rust"),
            from_local: None,
            tasks: Some(&tasks_path),
            completions: None,
            out: &out,
            print: false,
            run: true,
            limit: None,
            timeout_secs: 60,
            use_canonical: true,
        })
        .unwrap();
        let report: Value =
            serde_json::from_str(&fs::read_to_string(out.join("grade-report.json")).unwrap())
                .unwrap();
        assert_eq!(report["passed"], 1);
        assert_eq!(report["reference_canonical"], true);
        assert_eq!(report["live_pass_recorded"], false);
        assert_eq!(report["harness"], "cargo-test");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuse_paths_are_clear_and_print_does_not_score() {
        let dir = std::env::temp_dir().join(format!("grade-refuse-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let tasks_path = write_tasks(&dir, &good_pair().0);
        let err = cmd_classify_grade(&GradeRequest {
            dataset: Some("ag_news"),
            from_local: None,
            tasks: None,
            completions: None,
            out: &dir.join("nope"),
            print: true,
            run: false,
            limit: None,
            timeout_secs: 30,
            use_canonical: false,
        })
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("refuse:classify-grade: unknown dataset"),
            "{err}"
        );

        let err = cmd_classify_grade(&GradeRequest {
            dataset: Some("humanevalpack_rust"),
            from_local: None,
            tasks: Some(&tasks_path),
            completions: None,
            out: &dir.join("both"),
            print: true,
            run: true,
            limit: None,
            timeout_secs: 30,
            use_canonical: false,
        })
        .unwrap_err()
        .to_string();
        assert!(err.contains("pass only one of --print and --run"), "{err}");

        let err = cmd_classify_grade(&GradeRequest {
            dataset: Some("humanevalpack_rust"),
            from_local: None,
            tasks: None,
            completions: None,
            out: &dir.join("norun"),
            print: false,
            run: true,
            limit: None,
            timeout_secs: 30,
            use_canonical: false,
        })
        .unwrap_err()
        .to_string();
        assert!(err.contains("does not download"), "{err}");
        assert!(err.contains("hf download"), "{err}");

        let err = cmd_classify_grade(&GradeRequest {
            dataset: Some("nuprl/MultiPL-E"),
            from_local: None,
            tasks: Some(&tasks_path),
            completions: None,
            out: &dir.join("mismatch"),
            print: false,
            run: true,
            limit: None,
            timeout_secs: 30,
            use_canonical: true,
        })
        .unwrap_err()
        .to_string();
        assert!(err.contains("expected multiple_rust"), "{err}");

        let plan_out = dir.join("plan");
        cmd_classify_grade(&GradeRequest {
            dataset: Some("multiple_rust"),
            from_local: None,
            tasks: None,
            completions: None,
            out: &plan_out,
            print: false,
            run: false,
            limit: None,
            timeout_secs: 30,
            use_canonical: false,
        })
        .unwrap();
        let plan: Value =
            serde_json::from_str(&fs::read_to_string(plan_out.join("grade-plan.json")).unwrap())
                .unwrap();
        assert_eq!(plan["mode"], "print");
        assert_eq!(plan["would_compile"], false);
        assert_eq!(plan["live_pass_recorded"], false);
        assert_eq!(plan["ready_for_live_test"], "no");
        assert!(plan["license_note"]
            .as_str()
            .unwrap()
            .contains("BSD-3-Clause"));
        assert!(plan["license_note"].as_str().unwrap().contains("not MBPP"));
        assert!(!plan_out.join("grade-report.json").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn from_local_directory_and_limit_prefix() {
        let dir = std::env::temp_dir().join(format!("grade-local-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let snap = dir.join("snap");
        fs::create_dir_all(&snap).unwrap();
        fs::write(
            snap.join("b.jsonl"),
            "{\"id\":\"b\",\"dataset\":\"multiple_rust\",\"prompt\":\"fn id2(n: i32) -> i32 {\\n\",\"tests\":\"fn main() { assert_eq!(id2(2), 2); }\\n\",\"canonical\":\"    n\\n}\\n\"}\n",
        )
        .unwrap();
        fs::write(
            snap.join("a.jsonl"),
            "{\"id\":\"a\",\"dataset\":\"multiple_rust\",\"prompt\":\"fn id1(n: i32) -> i32 {\\n\",\"tests\":\"fn main() { assert_eq!(id1(1), 1); }\\n\",\"canonical\":\"    n\\n}\\n\"}\n",
        )
        .unwrap();
        let out = dir.join("out");
        cmd_classify_grade(&GradeRequest {
            dataset: Some("multiple_rust"),
            from_local: Some(&snap),
            tasks: None,
            completions: None,
            out: &out,
            print: false,
            run: true,
            limit: Some(1),
            timeout_secs: 30,
            use_canonical: true,
        })
        .unwrap();
        let report: Value =
            serde_json::from_str(&fs::read_to_string(out.join("grade-report.json")).unwrap())
                .unwrap();
        assert_eq!(report["records"], 1);
        assert_eq!(report["tasks"][0]["id"], "a");
        assert_eq!(report["passed"], 1);
        assert_eq!(report["limit_kind"], "stable-id-prefix");
        assert_eq!(report["live_pass_recorded"], false);
        let _ = fs::remove_dir_all(&dir);
    }
}
