//! Teacher expand for the rust_idiom FixedClasses curriculum.
//!
//! `--print` writes `expand-plan.json` and does not call the network.
//! `--run` is the only path that talks to an OpenAI-compatible teacher.
//! The API key is read from the environment and is never printed.
//! Output is tev1 JSONL in a tag-suffixed import cache. The held-out file is
//! copied, not rebuilt. `live_pass_recorded` stays false.

use crate::classify::{post_chat, scrub_snippet, HttpOutcome};
use crate::classify_import::{expand_cache_dir, preset_by_name, LabelMap};
use anyhow::{bail, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DEFAULT_TEACHER_KEY_ENV: &str = "TEACHER_API_KEY";
const SCHEMA_PLAN: &str = "cell-one.classify-expand-plan.v0";
const SCHEMA_REPORT: &str = "cell-one.classify-expand.v0";
const READY: &str = "no";
const QUESTION: &str = "Does this Rust snippet need a fix, or is it the idiomatic version?";

pub struct ExpandRequest<'a> {
    pub dataset: &'a str,
    pub train: Option<&'a Path>,
    pub heldout: Option<&'a Path>,
    pub from_local: Option<&'a Path>,
    pub out: Option<&'a Path>,
    pub tag: &'a str,
    pub train_size: &'a str,
    pub seed: u64,
    pub print: bool,
    pub run: bool,
    pub endpoint: Option<&'a str>,
    pub model: Option<&'a str>,
    pub api_key_env: Option<&'a str>,
    pub timeout_secs: u64,
}

pub fn cmd_classify_expand(req: &ExpandRequest<'_>) -> Result<()> {
    let live = LiveTeacher;
    expand_with(req, &live)
}

fn expand_with(req: &ExpandRequest<'_>, teacher: &dyn Teacher) -> Result<()> {
    if req.print && req.run {
        bail!("refuse:classify-expand: pass only one of --print and --run");
    }
    if req.timeout_secs == 0 {
        bail!("refuse:classify-expand: --timeout-secs must be at least 1");
    }
    let tag = validate_expand_tag(req.tag)?;
    let preset = preset_by_name(req.dataset)?;
    if preset.alias != "rust_idiom" {
        bail!(
            "refuse:classify-expand: {} is not the rust_idiom curriculum. This command expands rust_idiom (bigcode/commitpackft) only.",
            preset.alias
        );
    }
    let LabelMap::FixedClasses(classes) = preset.map else {
        bail!("refuse:classify-expand: rust_idiom options are not FixedClasses");
    };
    if classes.len() != 2 || classes[0].key != "needs_fix" || classes[1].key != "idiomatic" {
        bail!("refuse:classify-expand: rust_idiom class table is not A=NeedsFix, B=Idiomatic");
    }
    let train_size = crate::classify_import::parse_split_size(req.train_size)?;
    if req.train.is_none() && req.from_local.is_none() {
        bail!("refuse:classify-expand: pass --train or --from-local");
    }
    refuse_parquet(req.train, "--train")?;
    refuse_parquet(req.heldout, "--heldout")?;
    refuse_parquet(req.from_local, "--from-local")?;
    let (train_path, heldout_path) = resolve_inputs(req)?;
    let holdout_seed = source_holdout_seed(req.train, req.heldout, train_path.as_deref(), heldout_path.as_deref())?;
    let heldout_rows = match &heldout_path {
        Some(path) => read_jsonl(path)?,
        None => Vec::new(),
    };
    let train_rows = match &train_path {
        Some(path) => read_jsonl(path)?,
        None => Vec::new(),
    };
    let snippets = match req.from_local {
        Some(path) => read_jsonl(path)?,
        None => Vec::new(),
    };
    let holdout = holdout_index(&heldout_rows);
    let pairs = pair_train(&train_rows)?;
    let mut kept = Vec::new();
    let mut dropped = DropCounts::default();
    for pair in &pairs {
        match local_drop(pair) {
            Some(reason) => dropped.add(reason),
            None if holdout.commits.contains(&pair.commit) || holdout.blocks_pair(pair) => {
                dropped.holdout += 1;
            }
            None => kept.push(pair.clone()),
        }
    }
    let out = req
        .out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| expand_cache_dir(preset.alias, &train_size.token(), req.seed, &tag));
    let key_env = req
        .api_key_env
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(DEFAULT_TEACHER_KEY_ENV);
    if !req.run {
        write_plan(
            req,
            &out,
            &tag,
            &train_size.token(),
            &holdout,
            pairs.len(),
            kept.len(),
            &dropped,
            snippets.len(),
            key_env,
            heldout_path.as_deref(),
            holdout_seed,
        )?;
        return Ok(());
    }
    let endpoint = req
        .endpoint
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-expand: --run needs --endpoint"))?;
    let model = req
        .model
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-expand: --run needs --model"))?;
    let key = teacher.api_key(key_env)?;
    let mut accepted: Vec<Value> = Vec::new();
    let mut teacher_kept = 0u64;
    let mut teacher_dropped = 0u64;
    let mut siblings = 0u64;
    let mut authored = 0u64;
    let mut rejected = 0u64;
    for pair in &kept {
        let prompt = pair_prompt(pair);
        let reply = teacher.complete(endpoint, model, &prompt, &key, req.timeout_secs)?;
        let parsed = parse_teacher_json(&reply);
        let Some(parsed) = parsed else {
            rejected += 1;
            teacher_kept += 1;
            accepted.extend(pair.rows.clone());
            continue;
        };
        let keep = parsed.get("keep").and_then(Value::as_bool).unwrap_or(true);
        if keep {
            teacher_kept += 1;
            accepted.extend(pair.rows.clone());
        } else {
            teacher_dropped += 1;
        }
        if let Some(sibling) = parsed.get("sibling").filter(|v| !v.is_null()) {
            if value_contains_secret(sibling, &key) {
                rejected += 1;
            } else {
                match accept_pair(sibling, &holdout, &tag, accepted.len() / 2) {
                    Some(rows) => {
                        siblings += 1;
                        accepted.extend(rows);
                    }
                    None => rejected += 1,
                }
            }
        }
    }
    for snippet in &snippets {
        let text = snippet_text(snippet);
        if text.trim().is_empty() {
            rejected += 1;
            continue;
        }
        let prompt = snippet_prompt(&text);
        let reply = teacher.complete(endpoint, model, &prompt, &key, req.timeout_secs)?;
        if reply.contains(&key) {
            rejected += 1;
            continue;
        }
        let Some(parsed) = parse_teacher_json(&reply) else {
            rejected += 1;
            continue;
        };
        match accept_pair(&parsed, &holdout, &tag, accepted.len() / 2) {
            Some(rows) => {
                authored += 1;
                accepted.extend(rows);
            }
            None => rejected += 1,
        }
    }
    if train_leaks(&accepted, &holdout) {
        bail!("refuse:classify-expand: expanded train overlaps the held-out commit set");
    }
    if accepted.is_empty() {
        bail!(
            "refuse:classify-expand: expanded train is empty. Nothing accepted from the teacher or the source pairs."
        );
    }
    fs::create_dir_all(&out)?;
    write_jsonl(&out.join("train.jsonl"), &accepted)?;
    if let Some(path) = &heldout_path {
        fs::copy(path, out.join("heldout.jsonl")).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-expand: cannot copy held-out {}: {err}",
                path.display()
            )
        })?;
    } else {
        write_jsonl(&out.join("heldout.jsonl"), &[])?;
    }
    let held_bytes = fs::read(out.join("heldout.jsonl")).unwrap_or_default();
    let report = json!({
        "schema": SCHEMA_REPORT,
        "mode": "run",
        "dataset": preset.alias,
        "hf_id": preset.hf_id,
        "question": QUESTION,
        "option_order": "fixed",
        "options": "A=NeedsFix, B=Idiomatic",
        "train_size": train_size.token(),
        "seed": req.seed,
        "holdout_seed": holdout_seed,
        "heldout_kind": "seeded-commit-holdout",
        "tag": tag,
        "cache": out.display().to_string(),
        "endpoint": endpoint,
        "model": model,
        "api_key_env": key_env,
        "source_pairs": pairs.len(),
        "local_kept": kept.len(),
        "local_dropped": dropped.total(),
        "dropped": dropped.json(),
        "teacher_kept": teacher_kept,
        "teacher_dropped": teacher_dropped,
        "siblings": siblings,
        "authored": authored,
        "rejected": rejected,
        "rows_train": accepted.len(),
        "rows_heldout": heldout_rows.len(),
        "heldout_sha256": hex_sha(&held_bytes),
        "holdout_commits": holdout.commits.len(),
        "expanded_train_overlaps_holdout": false,
        "would_call_teacher": true,
        "live_pass_recorded": false,
        "ready_for_live_test": READY,
        "license_note": preset.license_note,
        "proof": proof_note(holdout_seed),
        "note": "classify expand writes a tag-suffixed tev1 cache for local training. It does not train. The held-out file is a copy of the source holdout. This file is not a factory live PASS. READY_FOR_LIVE_TEST stays no."
    });
    write_pretty(&out.join("expand-report.json"), &report)?;
    println!(
        "classify expand: run dataset=rust_idiom tag={tag} train_rows={} heldout_rows={} cache={}",
        accepted.len(),
        heldout_rows.len(),
        out.display()
    );
    Ok(())
}

fn write_plan(
    req: &ExpandRequest<'_>,
    out: &Path,
    tag: &str,
    train_size: &str,
    holdout: &Holdout,
    source_pairs: usize,
    local_kept: usize,
    dropped: &DropCounts,
    snippets: usize,
    key_env: &str,
    heldout_path: Option<&Path>,
    holdout_seed: u64,
) -> Result<()> {
    let held_hash = heldout_path
        .and_then(|path| fs::read(path).ok())
        .map(|bytes| hex_sha(&bytes))
        .unwrap_or_else(|| "missing".into());
    let report = json!({
        "schema": SCHEMA_PLAN,
        "mode": "print",
        "dataset": "rust_idiom",
        "hf_id": "bigcode/commitpackft",
        "question": QUESTION,
        "option_order": "fixed",
        "options": "A=NeedsFix, B=Idiomatic",
        "train_size": train_size,
        "seed": req.seed,
        "holdout_seed": holdout_seed,
        "heldout_kind": "seeded-commit-holdout",
        "tag": tag,
        "cache": out.display().to_string(),
        "endpoint": req.endpoint,
        "model": req.model,
        "api_key_env": key_env,
        "source_pairs": source_pairs,
        "local_kept": local_kept,
        "local_dropped": dropped.total(),
        "dropped": dropped.json(),
        "snippets": snippets,
        "rows_heldout": holdout.rows,
        "heldout_sha256": held_hash,
        "holdout_commits": holdout.commits.len(),
        "would_call_teacher": false,
        "live_pass_recorded": false,
        "ready_for_live_test": READY,
        "proof": proof_note(holdout_seed),
        "note": "Print does not call the teacher and does not write train.jsonl. --run reads the API key from the named environment variable and never prints the value. Held-out commits stay out of the expanded train. READY_FOR_LIVE_TEST stays no."
    });
    fs::create_dir_all(out)?;
    write_pretty(&out.join("expand-plan.json"), &report)?;
    println!(
        "classify expand: print dataset=rust_idiom tag={tag} api-key-env {key_env} dry-run no network cache={}",
        out.display()
    );
    Ok(())
}

fn proof_note(seed: u64) -> String {
    format!(
        "Holdout commits are rust_idiom:test:{{index}} with commit = index/2, the same SplitMix split import writes at seed {seed}. Expanded train keeps rust_idiom:train ids only when that commit is absent, plus rust_idiom:expand:{{tag}} ids. A teacher side whose text equals a held-out state is dropped. heldout.jsonl in the cache is a byte copy, so heldout_sha256 matches the source. Journey --dataset rust_idiom --expand-tag uses this cache and does not split again."
    )
}

#[derive(Clone, Debug)]
struct Pair {
    commit: u64,
    needs_fix: String,
    idiomatic: String,
    rows: Vec<Value>,
}

#[derive(Default)]
struct DropCounts {
    empty: u64,
    identical: u64,
    non_rust: u64,
    low_signal: u64,
    holdout: u64,
}

impl DropCounts {
    fn add(&mut self, reason: DropReason) {
        match reason {
            DropReason::Empty => self.empty += 1,
            DropReason::Identical => self.identical += 1,
            DropReason::NonRust => self.non_rust += 1,
            DropReason::LowSignal => self.low_signal += 1,
        }
    }

    fn total(&self) -> u64 {
        self.empty + self.identical + self.non_rust + self.low_signal + self.holdout
    }

    fn json(&self) -> Value {
        json!({
            "empty": self.empty,
            "identical": self.identical,
            "non_rust": self.non_rust,
            "low_signal": self.low_signal,
            "holdout": self.holdout,
        })
    }
}

enum DropReason {
    Empty,
    Identical,
    NonRust,
    LowSignal,
}

struct Holdout {
    commits: BTreeSet<u64>,
    states: BTreeSet<String>,
    rows: usize,
}

impl Holdout {
    fn blocks_text(&self, text: &str) -> bool {
        self.states.contains(text.trim())
    }

    fn blocks_pair(&self, pair: &Pair) -> bool {
        self.blocks_text(&pair.needs_fix) || self.blocks_text(&pair.idiomatic)
    }
}

fn holdout_index(rows: &[Value]) -> Holdout {
    let mut commits = BTreeSet::new();
    let mut states = BTreeSet::new();
    for row in rows {
        if let Some(commit) = row_commit(row, "test") {
            commits.insert(commit);
        }
        if let Some(text) = row.get("state").and_then(Value::as_str) {
            states.insert(text.trim().to_string());
        }
    }
    Holdout {
        commits,
        states,
        rows: rows.len(),
    }
}

fn pair_train(rows: &[Value]) -> Result<Vec<Pair>> {
    let mut by_commit: Vec<(u64, Vec<Value>)> = Vec::new();
    for row in rows {
        let commit = row_commit(row, "train").ok_or_else(|| {
            anyhow::anyhow!(
                "refuse:classify-expand: train row id is not rust_idiom:train:{{index}}"
            )
        })?;
        if let Some((_, bucket)) = by_commit.iter_mut().find(|(id, _)| *id == commit) {
            bucket.push(row.clone());
        } else {
            by_commit.push((commit, vec![row.clone()]));
        }
    }
    let mut pairs = Vec::new();
    for (commit, group) in by_commit {
        let needs = group.iter().find(|row| row_letter(row) == Some('A'));
        let idiom = group.iter().find(|row| row_letter(row) == Some('B'));
        let (Some(needs), Some(idiom)) = (needs, idiom) else {
            bail!(
                "refuse:classify-expand: commit {commit} is missing an A=NeedsFix or B=Idiomatic row"
            );
        };
        if !fixed_shape(needs) || !fixed_shape(idiom) {
            bail!("refuse:classify-expand: commit {commit} is not FixedClasses A/B");
        }
        pairs.push(Pair {
            commit,
            needs_fix: needs["state"].as_str().unwrap_or("").to_string(),
            idiomatic: idiom["state"].as_str().unwrap_or("").to_string(),
            rows: vec![needs.clone(), idiom.clone()],
        });
    }
    Ok(pairs)
}

fn row_letter(row: &Value) -> Option<char> {
    row.get("answer")
        .and_then(Value::as_str)
        .and_then(|s| s.chars().next())
}

fn fixed_shape(row: &Value) -> bool {
    let options = row.get("options").and_then(Value::as_array);
    let Some(options) = options else {
        return false;
    };
    if options.len() != 2 {
        return false;
    }
    let a = &options[0];
    let b = &options[1];
    row.get("question").and_then(Value::as_str) == Some(QUESTION)
        && a.get("label").and_then(Value::as_str) == Some("A")
        && a.get("key").and_then(Value::as_str) == Some("needs_fix")
        && a.get("description").and_then(Value::as_str) == Some("NeedsFix")
        && b.get("label").and_then(Value::as_str) == Some("B")
        && b.get("key").and_then(Value::as_str) == Some("idiomatic")
        && b.get("description").and_then(Value::as_str) == Some("Idiomatic")
}

fn row_commit(row: &Value, split: &str) -> Option<u64> {
    let id = row.get("id").and_then(Value::as_str)?;
    let prefix = format!("rust_idiom:{split}:");
    let rest = id.strip_prefix(&prefix)?;
    let index: u64 = rest.parse().ok()?;
    Some(index / 2)
}

fn local_drop(pair: &Pair) -> Option<DropReason> {
    let old = pair.needs_fix.trim();
    let new = pair.idiomatic.trim();
    if old.is_empty() || new.is_empty() {
        return Some(DropReason::Empty);
    }
    if old == new {
        return Some(DropReason::Identical);
    }
    if !looks_rust(old) || !looks_rust(new) {
        return Some(DropReason::NonRust);
    }
    if low_signal(old, new) {
        return Some(DropReason::LowSignal);
    }
    None
}

fn looks_rust(text: &str) -> bool {
    const MARKERS: &[&str] = &[
        "fn ", "let ", "struct ", "impl ", "enum ", "trait ", "use ", "mod ", "pub ", "async ",
        "await", "match ", "loop ", "unsafe ", "where ", "type ", "const ", "static ", "crate",
    ];
    MARKERS.iter().any(|m| text.contains(m))
}

fn low_signal(old: &str, new: &str) -> bool {
    old.len() < 12 || new.len() < 12 || whitespace_only_change(old, new)
}

fn whitespace_only_change(old: &str, new: &str) -> bool {
    let fold = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    fold(old) == fold(new)
}

fn accept_pair(value: &Value, holdout: &Holdout, tag: &str, seq: usize) -> Option<Vec<Value>> {
    let needs = value.get("needs_fix").and_then(Value::as_str)?;
    let idiom = value.get("idiomatic").and_then(Value::as_str)?;
    let pair = Pair {
        commit: u64::MAX,
        needs_fix: needs.to_string(),
        idiomatic: idiom.to_string(),
        rows: Vec::new(),
    };
    if local_drop(&pair).is_some() || holdout.blocks_pair(&pair) {
        return None;
    }
    let base = format!("rust_idiom:expand:{tag}:{seq}");
    Some(vec![
        tev1_row(&format!("{base}:0"), &pair.needs_fix, "A", "needs_fix"),
        tev1_row(&format!("{base}:1"), &pair.idiomatic, "B", "idiomatic"),
    ])
}

fn tev1_row(id: &str, state: &str, letter: &str, key: &str) -> Value {
    json!({
        "id": id,
        "group_id": id,
        "state": state,
        "question": QUESTION,
        "options": [
            {"label": "A", "key": "needs_fix", "description": "NeedsFix"},
            {"label": "B", "key": "idiomatic", "description": "Idiomatic"}
        ],
        "answer": letter,
        "answer_key": key,
    })
}

fn train_leaks(rows: &[Value], holdout: &Holdout) -> bool {
    rows.iter().any(|row| {
        if let Some(commit) = row_commit(row, "train") {
            if holdout.commits.contains(&commit) {
                return true;
            }
        }
        if let Some(id) = row.get("id").and_then(Value::as_str) {
            if id.contains(":test:") {
                return true;
            }
        }
        row.get("state")
            .and_then(Value::as_str)
            .is_some_and(|text| holdout.blocks_text(text))
    })
}

fn pair_prompt(pair: &Pair) -> String {
    format!(
        "Filter this Rust before/after pair and optionally propose one sibling NeedsFix/Idiomatic pair about async, concurrency, or IPC. Return one JSON object only: {{\"keep\":true|false,\"sibling\":{{\"needs_fix\":\"...\",\"idiomatic\":\"...\"}}|null}}. NeedsFix is the weaker snippet. Idiomatic is the stronger one. They must differ.\nNeedsFix:\n{}\nIdiomatic:\n{}",
        pair.needs_fix, pair.idiomatic
    )
}

fn snippet_prompt(text: &str) -> String {
    format!(
        "Write one Rust before/after pair for an idiomatic async, concurrency, or IPC pattern taught by this snippet. Return one JSON object only: {{\"needs_fix\":\"...\",\"idiomatic\":\"...\"}}. The sides must be non-empty Rust and must differ.\nSnippet:\n{text}"
    )
}

fn snippet_text(row: &Value) -> String {
    for key in ["snippet", "text", "code", "old_contents"] {
        if let Some(text) = row.get(key).and_then(Value::as_str) {
            return text.to_string();
        }
    }
    String::new()
}

fn value_contains_secret(value: &Value, secret: &str) -> bool {
    if secret.is_empty() {
        return false;
    }
    value.to_string().contains(secret)
}

fn parse_teacher_json(text: &str) -> Option<Value> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end < start {
        return None;
    }
    serde_json::from_str(&text[start..=end]).ok()
}

/// `holdout_seed` comes from the source import cache (`import.json`) or native
/// cache (`manifest.json`). `--seed` still names the expand cache path.
fn source_holdout_seed(
    train_arg: Option<&Path>,
    heldout_arg: Option<&Path>,
    train_file: Option<&Path>,
    heldout_file: Option<&Path>,
) -> Result<u64> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for path in [train_arg, heldout_arg, train_file, heldout_file]
        .into_iter()
        .flatten()
    {
        let dir = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().unwrap_or(Path::new(".")).to_path_buf()
        };
        if !dirs.iter().any(|seen| seen == &dir) {
            dirs.push(dir);
        }
    }
    for dir in &dirs {
        for rel in ["import.json", "manifest.json", "native/manifest.json"] {
            let path = dir.join(rel);
            if let Some(seed) = read_holdout_seed(&path)? {
                return Ok(seed);
            }
        }
    }
    bail!(
        "refuse:classify-expand: source holdout_seed is not in the import or native manifest. Re-import rust_idiom so the cache records holdout_seed. This command does not invent one."
    )
}

fn read_holdout_seed(path: &Path) -> Result<Option<u64>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-expand: cannot read {}: {err}",
            path.display()
        )
    })?;
    let value: Value = serde_json::from_str(&text).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-expand: {} is not json ({err})",
            path.display()
        )
    })?;
    match value.get("holdout_seed").and_then(Value::as_u64) {
        Some(seed) => Ok(Some(seed)),
        None => Ok(None),
    }
}

fn resolve_inputs(req: &ExpandRequest<'_>) -> Result<(Option<PathBuf>, Option<PathBuf>)> {
    let (train, heldout_from_dir) = match req.train {
        Some(path) if path.is_dir() => {
            let train = path.join("train.jsonl");
            let held = path.join("heldout.jsonl");
            if !train.is_file() {
                bail!(
                    "refuse:classify-expand: {} has no train.jsonl",
                    path.display()
                );
            }
            (Some(train), held.is_file().then_some(held))
        }
        Some(path) => (Some(path.to_path_buf()), None),
        None => (None, None),
    };
    let heldout = req.heldout.map(Path::to_path_buf).or(heldout_from_dir);
    if train.is_some() && heldout.is_none() {
        bail!("refuse:classify-expand: --train needs a held-out JSONL (--heldout, or heldout.jsonl in the import cache)");
    }
    if req.from_local.is_some() && heldout.is_none() {
        bail!("refuse:classify-expand: --from-local needs --heldout so new pairs cannot copy a held-out commit");
    }
    Ok((train, heldout))
}

fn refuse_parquet(path: Option<&Path>, flag: &str) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    if path.extension().and_then(|ext| ext.to_str()) == Some("parquet")
        || path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".parquet"))
    {
        bail!(
            "refuse:classify-expand: {flag} {} is parquet. Export JSONL and pass that file.",
            path.display()
        );
    }
    if path.is_dir() {
        let mut entries = fs::read_dir(path).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-expand: cannot read {}: {err}",
                path.display()
            )
        })?;
        while let Some(entry) = entries.next() {
            let entry = entry.map_err(|err| {
                anyhow::anyhow!(
                    "refuse:classify-expand: cannot read {}: {err}",
                    path.display()
                )
            })?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".parquet") {
                bail!(
                    "refuse:classify-expand: {} is parquet. Export JSONL and pass that file.",
                    entry.path().display()
                );
            }
        }
    }
    Ok(())
}

pub fn validate_expand_tag(tag: &str) -> Result<String> {
    let tag = tag.trim();
    if tag.is_empty()
        || tag.len() > 64
        || !tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!(
            "refuse:classify-expand: --tag must be 1-64 characters of letters, digits, '-' or '_'"
        );
    }
    Ok(tag.to_string())
}

fn read_jsonl(path: &Path) -> Result<Vec<Value>> {
    let file = File::open(path).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-expand: cannot read {}: {err}",
            path.display()
        )
    })?;
    let mut rows = Vec::new();
    for (line_no, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-expand: {} line {} is not json ({err})",
                path.display(),
                line_no + 1
            )
        })?;
        if !value.is_object() {
            bail!(
                "refuse:classify-expand: {} line {} is not an object",
                path.display(),
                line_no + 1
            );
        }
        rows.push(value);
    }
    Ok(rows)
}

fn write_jsonl(path: &Path, rows: &[Value]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    for row in rows {
        writeln!(file, "{}", serde_json::to_string(row)?)?;
    }
    Ok(())
}

fn write_pretty(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

fn hex_sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

trait Teacher {
    fn api_key(&self, name: &str) -> Result<String>;
    fn complete(
        &self,
        endpoint: &str,
        model: &str,
        prompt: &str,
        api_key: &str,
        timeout_secs: u64,
    ) -> Result<String>;
}

struct LiveTeacher;

impl Teacher for LiveTeacher {
    fn api_key(&self, name: &str) -> Result<String> {
        read_teacher_key(name)
    }

    fn complete(
        &self,
        endpoint: &str,
        model: &str,
        prompt: &str,
        api_key: &str,
        timeout_secs: u64,
    ) -> Result<String> {
        let url = chat_url(endpoint);
        let body = json!({
            "model": model,
            "temperature": 0,
            "messages": [
                {"role": "system", "content": "Return one JSON object. Do not include the API key."},
                {"role": "user", "content": prompt}
            ]
        });
        match post_chat(
            &url,
            &body,
            Some(api_key),
            Duration::from_secs(timeout_secs),
        ) {
            HttpOutcome::Ok(text) => Ok(text),
            HttpOutcome::Fail(err) => {
                let body = scrub_snippet(&err.body, Some(api_key));
                bail!(
                    "refuse:classify-expand: teacher HTTP {}: {body}",
                    err.status
                )
            }
        }
    }
}

pub(crate) fn read_teacher_key(name: &str) -> Result<String> {
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        bail!("refuse:classify-expand: api-key-env must be an environment variable name");
    }
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => Ok(value),
        _ => bail!("refuse:classify-expand: set {name}"),
    }
}

fn chat_url(endpoint: &str) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else if trimmed.ends_with("/v1") {
        format!("{trimmed}/chat/completions")
    } else {
        format!("{trimmed}/v1/chat/completions")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Scripted {
        key: String,
        replies: Mutex<Vec<String>>,
        prompts: Mutex<Vec<String>>,
        saw_key_in_prompt: Mutex<bool>,
    }

    impl Teacher for Scripted {
        fn api_key(&self, name: &str) -> Result<String> {
            let _ = name;
            Ok(self.key.clone())
        }

        fn complete(
            &self,
            _endpoint: &str,
            _model: &str,
            prompt: &str,
            api_key: &str,
            _timeout_secs: u64,
        ) -> Result<String> {
            if prompt.contains(api_key) {
                *self.saw_key_in_prompt.lock().unwrap() = true;
            }
            self.prompts.lock().unwrap().push(prompt.to_string());
            let mut replies = self.replies.lock().unwrap();
            if replies.is_empty() {
                bail!("refuse:classify-expand: scripted teacher has no reply");
            }
            Ok(replies.remove(0))
        }
    }

    fn row(split: &str, index: u64, state: &str, letter: &str, key: &str) -> Value {
        json!({
            "id": format!("rust_idiom:{split}:{index}"),
            "group_id": format!("rust_idiom:{split}:{index}"),
            "state": state,
            "question": QUESTION,
            "options": [
                {"label": "A", "key": "needs_fix", "description": "NeedsFix"},
                {"label": "B", "key": "idiomatic", "description": "Idiomatic"}
            ],
            "answer": letter,
            "answer_key": key
        })
    }

    fn write_pair(file: &mut String, split: &str, commit: u64, old: &str, new: &str) {
        let base = commit * 2;
        file.push_str(&serde_json::to_string(&row(split, base, old, "A", "needs_fix")).unwrap());
        file.push('\n');
        file.push_str(
            &serde_json::to_string(&row(split, base + 1, new, "B", "idiomatic")).unwrap(),
        );
        file.push('\n');
    }

    fn fixture() -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "expand-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let train = root.join("train.jsonl");
        let held = root.join("heldout.jsonl");
        let mut train_body = String::new();
        write_pair(
            &mut train_body,
            "train",
            1,
            "fn weak_lock() { let _g = mutex.lock().unwrap(); }",
            "fn strong_lock() { let _g = mutex.lock().unwrap(); drop(_g); }",
        );
        write_pair(&mut train_body, "train", 2, "", "fn empty_side() {}");
        write_pair(
            &mut train_body,
            "train",
            3,
            "fn same() { let x = 1; }",
            "fn same() { let x = 1; }",
        );
        write_pair(&mut train_body, "train", 4, "print('no')", "print('rust')");
        write_pair(&mut train_body, "train", 5, "fn a(){}", "fn b(){}");
        write_pair(
            &mut train_body,
            "train",
            9,
            "fn held_body() { let x = 1; }",
            "fn held_body_new() { let x = 1; }",
        );
        fs::write(&train, train_body).unwrap();
        let mut held_body = String::new();
        write_pair(
            &mut held_body,
            "test",
            9,
            "fn held_body() { let x = 1; }",
            "fn held_body_new() { let x = 1; }",
        );
        write_pair(
            &mut held_body,
            "test",
            8,
            "async fn held_only() { let _ = channel::unbounded::<u8>(); }",
            "async fn held_only_new() { let _ = channel::unbounded::<u8>(); }",
        );
        fs::write(&held, held_body).unwrap();
        fs::write(
            root.join("import.json"),
            "{\"holdout_seed\":42,\"dataset\":\"rust_idiom\"}\n",
        )
        .unwrap();
        (root, train, held)
    }

    fn req<'a>(
        train: &'a Path,
        held: &'a Path,
        out: &'a Path,
        run: bool,
        from_local: Option<&'a Path>,
    ) -> ExpandRequest<'a> {
        ExpandRequest {
            dataset: "rust_idiom",
            train: Some(train),
            heldout: Some(held),
            from_local,
            out: Some(out),
            tag: "rev1",
            train_size: "all",
            seed: 42,
            print: !run,
            run,
            endpoint: Some("http://127.0.0.1:9"),
            model: Some("teacher-test"),
            api_key_env: Some("TEACHER_API_KEY"),
            timeout_secs: 5,
        }
    }

    #[test]
    fn print_does_not_call_the_teacher_or_write_train() {
        let (root, train, held) = fixture();
        let out = root.join("out");
        let teacher = Scripted {
            key: "secret-key-should-not-appear".into(),
            replies: Mutex::new(vec!["nope".into()]),
            prompts: Mutex::new(vec![]),
            saw_key_in_prompt: Mutex::new(false),
        };
        expand_with(&req(&train, &held, &out, false, None), &teacher).unwrap();
        assert!(teacher.prompts.lock().unwrap().is_empty());
        assert!(out.join("expand-plan.json").is_file());
        assert!(!out.join("train.jsonl").exists());
        let plan: Value =
            serde_json::from_str(&fs::read_to_string(out.join("expand-plan.json")).unwrap())
                .unwrap();
        assert_eq!(plan["mode"], "print");
        assert_eq!(plan["would_call_teacher"], false);
        assert_eq!(plan["live_pass_recorded"], false);
        assert_eq!(plan["ready_for_live_test"], "no");
        assert_eq!(plan["holdout_seed"], 42);
        assert_eq!(plan["options"], "A=NeedsFix, B=Idiomatic");
        assert_eq!(plan["dropped"]["empty"], 1);
        assert_eq!(plan["dropped"]["identical"], 1);
        assert_eq!(plan["dropped"]["non_rust"], 1);
        assert_eq!(plan["dropped"]["low_signal"], 1);
        assert_eq!(plan["dropped"]["holdout"], 1);
        assert_eq!(plan["local_kept"], 1);
        let text = fs::read_to_string(out.join("expand-plan.json")).unwrap();
        assert!(!text.contains("secret-key-should-not-appear"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_filters_keeps_shape_and_blocks_holdout_leak() {
        let (root, train, held) = fixture();
        let out = root.join("cache");
        let snippets = root.join("snippets.jsonl");
        fs::write(
            &snippets,
            concat!(
                "{\"snippet\":\"async fn demo() { tokio::spawn(work()); }\"}\n",
                "{\"code\":\"fn ipc() { let _ = std::process::Command::new(\\\"echo\\\"); }\"}\n",
            ),
        )
        .unwrap();
        let secret = "teacher-secret-zz";
        let teacher = Scripted {
            key: secret.into(),
            replies: Mutex::new(vec![
                "{\"keep\":true,\"sibling\":{\"needs_fix\":\"fn held_body() { let x = 1; }\",\"idiomatic\":\"fn other_side() { let y = 2; }\"}}".into(),
                "{\"needs_fix\":\"async fn raw_old() { let _ = std::thread::spawn(|| {}); }\",\"idiomatic\":\"async fn raw_new() { let _ = tokio::spawn(async {}); }\"}".into(),
                secret.into(),
            ]),
            prompts: Mutex::new(vec![]),
            saw_key_in_prompt: Mutex::new(false),
        };
        let request = req(&train, &held, &out, true, Some(&snippets));
        expand_with(&request, &teacher).unwrap();
        assert!(!*teacher.saw_key_in_prompt.lock().unwrap());
        let report: Value =
            serde_json::from_str(&fs::read_to_string(out.join("expand-report.json")).unwrap())
                .unwrap();
        assert_eq!(report["mode"], "run");
        assert_eq!(report["expanded_train_overlaps_holdout"], false);
        assert_eq!(report["live_pass_recorded"], false);
        assert_eq!(report["ready_for_live_test"], "no");
        assert_eq!(report["api_key_env"], "TEACHER_API_KEY");
        assert_eq!(report["teacher_kept"], 1);
        assert_eq!(report["authored"], 1);
        assert!(report["rejected"].as_u64().unwrap() >= 2);
        let body = fs::read_to_string(out.join("train.jsonl")).unwrap();
        assert!(!body.contains(secret), "{body}");
        assert!(!body.contains("held_body"), "{body}");
        assert!(!body.contains(":test:"), "{body}");
        assert!(body.contains("strong_lock"), "{body}");
        assert!(body.contains("raw_new"), "{body}");
        for line in body.lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            assert!(fixed_shape(&row), "{row}");
            assert_eq!(row["question"], QUESTION);
        }
        let copied = fs::read(out.join("heldout.jsonl")).unwrap();
        let source = fs::read(&held).unwrap();
        assert_eq!(copied, source);
        assert_eq!(report["heldout_sha256"], hex_sha(&source));
        assert_eq!(report["holdout_seed"], 42);
        assert_eq!(report["seed"], 42);
        assert!(report["proof"].as_str().unwrap().contains("seed 42"));
        let expected = expand_cache_dir("rust_idiom", "all", 42, "rev1");
        assert!(expected.ends_with("rust_idiom-all-s42-rev1"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn holdout_seed_comes_from_the_source_manifest_not_the_cache_seed() {
        let (root, train, held) = fixture();
        fs::write(
            root.join("import.json"),
            "{\"holdout_seed\":99,\"dataset\":\"rust_idiom\"}\n",
        )
        .unwrap();
        let out = root.join("out");
        let mut request = req(&train, &held, &out, false, None);
        request.seed = 7;
        expand_with(&request, &LiveTeacher).unwrap();
        let plan: Value =
            serde_json::from_str(&fs::read_to_string(out.join("expand-plan.json")).unwrap())
                .unwrap();
        assert_eq!(plan["seed"], 7);
        assert_eq!(plan["holdout_seed"], 99);
        assert!(plan["proof"].as_str().unwrap().contains("seed 99"));
        assert!(!plan["proof"].as_str().unwrap().contains("seed 7"));
        fs::write(root.join("import.json"), "{\"dataset\":\"rust_idiom\"}\n").unwrap();
        fs::create_dir_all(root.join("native")).unwrap();
        fs::write(
            root.join("native").join("manifest.json"),
            "{\"holdout_seed\":42}\n",
        )
        .unwrap();
        expand_with(&request, &LiveTeacher).unwrap();
        let again: Value =
            serde_json::from_str(&fs::read_to_string(out.join("expand-plan.json")).unwrap())
                .unwrap();
        assert_eq!(again["holdout_seed"], 42);
        assert_eq!(again["seed"], 7);
        assert!(again["proof"].as_str().unwrap().contains("seed 42"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_source_holdout_seed_is_refused() {
        let (root, train, held) = fixture();
        fs::write(root.join("import.json"), "{\"dataset\":\"rust_idiom\",\"seed\":42}\n").unwrap();
        let out = root.join("out");
        let err = expand_with(&req(&train, &held, &out, false, None), &LiveTeacher)
            .unwrap_err()
            .to_string();
        assert!(err.contains("holdout_seed"), "{err}");
        assert!(err.contains("does not invent"), "{err}");
        assert!(!out.join("expand-plan.json").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_refuses_before_write_when_the_accepted_train_is_empty() {
        let (root, train, held) = fixture();
        let out = root.join("empty-cache");
        let teacher = Scripted {
            key: "teacher-secret-zz".into(),
            replies: Mutex::new(vec!["{\"keep\":false}".into()]),
            prompts: Mutex::new(vec![]),
            saw_key_in_prompt: Mutex::new(false),
        };
        let err = expand_with(&req(&train, &held, &out, true, None), &teacher)
            .unwrap_err()
            .to_string();
        assert!(err.contains("expanded train is empty"), "{err}");
        assert!(!out.join("train.jsonl").exists());
        assert!(!out.join("expand-report.json").exists());
        assert!(!out.join("heldout.jsonl").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn refuses_parquet_both_flags_and_a_foreign_dataset() {
        let (root, train, held) = fixture();
        let out = root.join("out");
        let mut both = req(&train, &held, &out, false, None);
        both.print = true;
        both.run = true;
        let err = expand_with(&both, &LiveTeacher).unwrap_err().to_string();
        assert!(err.contains("only one of --print and --run"), "{err}");
        let parquet = root.join("rows.parquet");
        fs::write(&parquet, "not-real").unwrap();
        let mut bad = req(&parquet, &held, &out, false, None);
        bad.run = false;
        bad.print = false;
        let err = expand_with(&bad, &LiveTeacher).unwrap_err().to_string();
        assert!(err.contains("parquet"), "{err}");
        let mut other = req(&train, &held, &out, false, None);
        other.dataset = "ag_news";
        let err = expand_with(&other, &LiveTeacher).unwrap_err().to_string();
        assert!(err.contains("rust_idiom"), "{err}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_without_a_key_name_refuses_and_cache_dir_appends_the_tag() {
        let err = read_teacher_key("NOT_A_REAL_TEACHER_KEY_ZZ")
            .unwrap_err()
            .to_string();
        assert!(err.contains("set NOT_A_REAL_TEACHER_KEY_ZZ"), "{err}");
        assert!(!err.contains("sk-"));
        let dir = expand_cache_dir("rust_idiom", "3000", 42, "2026-09-25");
        assert!(
            dir.ends_with("rust_idiom-3000-s42-2026-09-25"),
            "{}",
            dir.display()
        );
    }
}
