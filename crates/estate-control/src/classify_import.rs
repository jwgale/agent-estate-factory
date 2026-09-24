//! Download a public Hugging Face classification set and write tev1 records.
//!
//! The bytes come from the datasets-server rows API (plain HTTPS JSON).
//! That avoids an Arrow dependency. Tests never call the network: they pass
//! synthetic rows into [`sample_records`].
//!
//! Option order for `fixed_classes` presets (ag_news) is the class table order.
//! Letters A, B, C, … follow that order and are not shuffled. The answer letter
//! is the class letter. `sample_distractors` (banking77, later) shuffles the
//! chosen options with the import seed, and the answer letter follows that shuffle.

use crate::classify::split_indices;
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const LABELS: &str = "ABCDEFGHIJKLMNOPQRSTUVWX";
const ROWS_PAGE: u64 = 100;
const DATASETS_SERVER: &str = "https://datasets-server.huggingface.co";

pub const DEFAULT_IMPORT_ROOT: &str = ".cell/classify-import";
pub const DEFAULT_IMPORT_SEED: u64 = 42;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassSpec {
    pub key: &'static str,
    pub description: &'static str,
}

/// How native labels become tev1 options. A new dataset is one catalog row.
#[derive(Clone, Copy, Debug)]
pub enum LabelMap {
    /// Options stay in `classes` order. Not shuffled.
    FixedClasses(&'static [ClassSpec]),
    /// True label plus `distractors` other labels, shuffled with the seed.
    SampleDistractors {
        classes: &'static [ClassSpec],
        distractors: usize,
    },
    /// Premise and hypothesis fields, fixed 3-way options.
    ThreeWay {
        classes: &'static [ClassSpec],
        premise_field: &'static str,
        hypothesis_field: &'static str,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fetch {
    RowsApi,
    /// Mapping is in the table. This slice does not download it.
    Later,
}

#[derive(Clone, Copy, Debug)]
pub struct DatasetPreset {
    pub alias: &'static str,
    pub hf_id: &'static str,
    /// Tag and directory slug, for example `agnews`.
    pub slug: &'static str,
    pub train_split: &'static str,
    pub test_split: &'static str,
    #[allow(dead_code)]
    pub official_train: u64,
    #[allow(dead_code)]
    pub official_test: u64,
    pub question: &'static str,
    pub text_field: &'static str,
    pub label_field: &'static str,
    pub map: LabelMap,
    pub license_note: &'static str,
    fetch: Fetch,
}

const AG_NEWS_CLASSES: &[ClassSpec] = &[
    ClassSpec {
        key: "world",
        description: "World",
    },
    ClassSpec {
        key: "sports",
        description: "Sports",
    },
    ClassSpec {
        key: "business",
        description: "Business",
    },
    ClassSpec {
        key: "sci_tech",
        description: "Sci/Tech",
    },
];

const MULTI_NLI_CLASSES: &[ClassSpec] = &[
    ClassSpec {
        key: "entailment",
        description: "Entailment",
    },
    ClassSpec {
        key: "neutral",
        description: "Neutral",
    },
    ClassSpec {
        key: "contradiction",
        description: "Contradiction",
    },
];

/// First-class preset plus the two later rows. Only `ag_news` downloads.
const CATALOG: &[DatasetPreset] = &[
    DatasetPreset {
        alias: "ag_news",
        hf_id: "fancyzhx/ag_news",
        slug: "agnews",
        train_split: "train",
        test_split: "test",
        official_train: 120_000,
        official_test: 7_600,
        question: "Which topic is this news article?",
        text_field: "text",
        label_field: "label",
        map: LabelMap::FixedClasses(AG_NEWS_CLASSES),
        license_note: "ag_news license is unspecified on the Hugging Face dataset card; this output is for local training only, do not redistribute.",
        fetch: Fetch::RowsApi,
    },
    DatasetPreset {
        alias: "banking77",
        hf_id: "legacy-datasets/banking77",
        slug: "banking77",
        train_split: "train",
        test_split: "test",
        official_train: 10_003,
        official_test: 3_080,
        question: "Which intent matches this customer message?",
        text_field: "text",
        label_field: "label",
        map: LabelMap::SampleDistractors {
            classes: &[],
            distractors: 4,
        },
        license_note: "banking77 is not fetched in this slice.",
        fetch: Fetch::Later,
    },
    DatasetPreset {
        alias: "multi_nli",
        hf_id: "nyu-mll/multi_nli",
        slug: "multinli",
        train_split: "train",
        test_split: "validation_matched",
        official_train: 392_702,
        official_test: 9_815,
        question: "What is the relationship between the premise and the hypothesis?",
        text_field: "premise",
        label_field: "label",
        map: LabelMap::ThreeWay {
            classes: MULTI_NLI_CLASSES,
            premise_field: "premise",
            hypothesis_field: "hypothesis",
        },
        license_note: "multi_nli is not fetched in this slice.",
        fetch: Fetch::Later,
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SplitSize {
    Count(usize),
    All,
}

impl SplitSize {
    pub fn token(&self) -> String {
        match self {
            Self::All => "all".into(),
            Self::Count(n) => n.to_string(),
        }
    }
}

pub fn parse_split_size(raw: &str) -> Result<SplitSize> {
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("all") {
        return Ok(SplitSize::All);
    }
    let n: usize = raw.parse().map_err(|_| {
        anyhow::anyhow!("refuse:classify-import: size must be a positive integer or all, found {raw}")
    })?;
    if n == 0 {
        bail!("refuse:classify-import: size must be a positive integer or all");
    }
    Ok(SplitSize::Count(n))
}

pub fn preset_by_name(name: &str) -> Result<&'static DatasetPreset> {
    let name = name.trim();
    CATALOG
        .iter()
        .find(|row| row.alias == name || row.hf_id == name || row.slug == name)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "refuse:classify-import: unknown dataset {name}. This slice downloads ag_news (fancyzhx/ag_news)."
            )
        })
}

pub fn default_import_dir(alias: &str) -> PathBuf {
    PathBuf::from(DEFAULT_IMPORT_ROOT).join(alias)
}

/// `-agnews-3000` so 3k / 10k / 30k journeys can sit side by side.
pub fn tag_suffix(preset: &DatasetPreset, train_size: &SplitSize) -> String {
    format!("-{}-{}", preset.slug, train_size.token())
}

pub fn import_fingerprint(
    dataset_id: &str,
    train_size: &str,
    heldout_size: &str,
    seed: u64,
    train_hash: &str,
    held_hash: &str,
) -> String {
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(
        format!(
            "classify-import\n{dataset_id}\ntrain={train_size}\nheldout={heldout_size}\nseed={seed}\n{train_hash}\n{held_hash}"
        )
        .as_bytes(),
    );
    format!("{:x}", hasher.finalize())
}

#[derive(Clone, Debug)]
pub struct NativeRow {
    pub index: u64,
    pub text: String,
    pub premise: Option<String>,
    pub hypothesis: Option<String>,
    pub label: usize,
}

#[derive(Clone, Debug)]
pub struct Sampled {
    pub train: Vec<Value>,
    pub heldout: Vec<Value>,
}

pub fn sample_records(
    preset: &DatasetPreset,
    train_rows: &[NativeRow],
    test_rows: &[NativeRow],
    train_size: &SplitSize,
    heldout_size: &SplitSize,
    seed: u64,
) -> Result<Sampled> {
    let train_idx = balanced_indices(train_rows, train_size, seed, "train")?;
    let held_idx = balanced_indices(test_rows, heldout_size, seed.wrapping_add(0xA5A5_5A5A), "test")?;
    let train = materialize(preset, preset.train_split, train_rows, &train_idx, seed)?;
    let heldout = materialize(
        preset,
        preset.test_split,
        test_rows,
        &held_idx,
        seed.wrapping_add(0xA5A5_5A5A),
    )?;
    let mut ids = BTreeMap::new();
    for (side, rows) in [("train", &train), ("heldout", &heldout)] {
        for row in rows {
            let id = row
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                bail!("refuse:classify-import: a {side} record has no id");
            }
            if let Some(prev) = ids.insert(id.clone(), side) {
                bail!(
                    "refuse:classify-import: id {id} is in both {prev} and {side}; train and held-out must be disjoint"
                );
            }
        }
    }
    Ok(Sampled { train, heldout })
}

fn balanced_indices(
    rows: &[NativeRow],
    size: &SplitSize,
    seed: u64,
    side: &str,
) -> Result<Vec<usize>> {
    if rows.is_empty() {
        bail!("refuse:classify-import: {side} split has no rows");
    }
    let n = match size {
        SplitSize::All => return Ok((0..rows.len()).collect()),
        SplitSize::Count(n) => *n,
    };
    if n > rows.len() {
        bail!(
            "refuse:classify-import: {side} size {n} is larger than the split ({})",
            rows.len()
        );
    }
    let mut by_label: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, row) in rows.iter().enumerate() {
        by_label.entry(row.label).or_default().push(i);
    }
    let classes: Vec<usize> = by_label.keys().copied().collect();
    if classes.is_empty() {
        bail!("refuse:classify-import: {side} split has no labels");
    }
    let base = n / classes.len();
    let rem = n % classes.len();
    let mut picked = Vec::with_capacity(n);
    for (ord, label) in classes.iter().enumerate() {
        let want = base + usize::from(ord < rem);
        let pool = &by_label[label];
        if want > pool.len() {
            bail!(
                "refuse:classify-import: {side} label {label} has {} rows, need {want} for a class-balanced sample of {n}",
                pool.len()
            );
        }
        if want == 0 {
            continue;
        }
        let class_seed = seed.wrapping_add((*label as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let (keep, _) = split_indices(pool.len(), class_seed, pool.len() - want);
        for idx in keep {
            picked.push(pool[idx]);
        }
    }
    picked.sort_unstable();
    Ok(picked)
}

fn materialize(
    preset: &DatasetPreset,
    split: &str,
    rows: &[NativeRow],
    indices: &[usize],
    seed: u64,
) -> Result<Vec<Value>> {
    let mut out = Vec::with_capacity(indices.len());
    for &idx in indices {
        let row = &rows[idx];
        out.push(to_record(preset, split, row, seed)?);
    }
    Ok(out)
}

fn to_record(preset: &DatasetPreset, split: &str, row: &NativeRow, seed: u64) -> Result<Value> {
    let id = format!("{}:{split}:{}", preset.alias, row.index);
    let (state, options, answer, answer_key) = match preset.map {
        LabelMap::FixedClasses(classes) => {
            let (options, answer, key) = fixed_options(classes, row.label)?;
            (Value::String(row.text.clone()), options, answer, key)
        }
        LabelMap::SampleDistractors {
            classes,
            distractors,
        } => {
            if classes.is_empty() {
                bail!(
                    "refuse:classify-import: {} distractor labels are not in this slice",
                    preset.hf_id
                );
            }
            let (options, answer, key) =
                distractor_options(classes, row.label, distractors, seed, row.index)?;
            (Value::String(row.text.clone()), options, answer, key)
        }
        LabelMap::ThreeWay {
            classes,
            premise_field,
            hypothesis_field,
        } => {
            let _ = (premise_field, hypothesis_field);
            let premise = row
                .premise
                .clone()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| anyhow::anyhow!("refuse:classify-import: missing premise"))?;
            let hypothesis = row
                .hypothesis
                .clone()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| anyhow::anyhow!("refuse:classify-import: missing hypothesis"))?;
            let (options, answer, key) = fixed_options(classes, row.label)?;
            (
                json!({"premise": premise, "hypothesis": hypothesis}),
                options,
                answer,
                key,
            )
        }
    };
    Ok(json!({
        "id": id,
        "group_id": id,
        "state": state,
        "question": preset.question,
        "options": options,
        "answer": answer.to_string(),
        "answer_key": answer_key,
    }))
}

fn fixed_options(classes: &[ClassSpec], label: usize) -> Result<(Vec<Value>, char, String)> {
    if classes.is_empty() || classes.len() > LABELS.chars().count() {
        bail!("refuse:classify-import: class table length is invalid");
    }
    let spec = classes.get(label).ok_or_else(|| {
        anyhow::anyhow!("refuse:classify-import: label {label} is outside the class table")
    })?;
    let letters: Vec<char> = LABELS.chars().take(classes.len()).collect();
    let options = classes
        .iter()
        .enumerate()
        .map(|(i, class)| {
            json!({
                "label": letters[i].to_string(),
                "key": class.key,
                "description": class.description,
            })
        })
        .collect();
    Ok((options, letters[label], spec.key.to_string()))
}

fn distractor_options(
    classes: &[ClassSpec],
    label: usize,
    distractors: usize,
    seed: u64,
    index: u64,
) -> Result<(Vec<Value>, char, String)> {
    if classes.get(label).is_none() {
        bail!("refuse:classify-import: label {label} is outside the class table");
    }
    if distractors == 0 || distractors + 1 > classes.len() || distractors + 1 > LABELS.chars().count()
    {
        bail!("refuse:classify-import: distractor count {distractors} does not fit the class table");
    }
    let others: Vec<usize> = (0..classes.len()).filter(|i| *i != label).collect();
    let (keep, _) = split_indices(
        others.len(),
        seed.wrapping_add(index),
        others.len() - distractors,
    );
    let mut chosen: Vec<usize> = keep.into_iter().map(|i| others[i]).collect();
    chosen.push(label);
    let (order, _) = split_indices(chosen.len(), seed.wrapping_add(index).wrapping_mul(3), 0);
    let shuffled: Vec<usize> = order.into_iter().map(|i| chosen[i]).collect();
    let letters: Vec<char> = LABELS.chars().take(shuffled.len()).collect();
    let mut answer = 'A';
    let mut answer_key = String::new();
    let options = shuffled
        .iter()
        .enumerate()
        .map(|(i, class_idx)| {
            let class = classes[*class_idx];
            if *class_idx == label {
                answer = letters[i];
                answer_key = class.key.to_string();
            }
            json!({
                "label": letters[i].to_string(),
                "key": class.key,
                "description": class.description,
            })
        })
        .collect();
    Ok((options, answer, answer_key))
}

pub struct ImportRequest<'a> {
    pub dataset: &'a str,
    pub train_size: &'a str,
    pub heldout_size: &'a str,
    pub seed: u64,
    pub out: &'a Path,
    pub force: bool,
    /// Offline native JSONL. When both are set, import does not use the network.
    pub native_train: Option<&'a Path>,
    pub native_test: Option<&'a Path>,
}

pub fn cmd_classify_import(req: &ImportRequest<'_>) -> Result<()> {
    let preset = preset_by_name(req.dataset)?;
    if preset.fetch != Fetch::RowsApi && req.native_train.is_none() {
        bail!(
            "refuse:classify-import: {} is a catalog row for a later slice. This slice downloads ag_news.",
            preset.hf_id
        );
    }
    let train_size = parse_split_size(req.train_size)?;
    let heldout_size = parse_split_size(req.heldout_size)?;
    if req.out.is_file() {
        bail!(
            "refuse:classify-import: --out {} is a file",
            req.out.display()
        );
    }
    fs::create_dir_all(req.out)?;
    let (train_rows, test_rows) = if let (Some(train), Some(test)) = (req.native_train, req.native_test)
    {
        (read_native(train)?, read_native(test)?)
    } else if req.native_train.is_some() || req.native_test.is_some() {
        bail!("refuse:classify-import: pass both --native-train and --native-test, or neither");
    } else {
        download_native(preset, req.out, req.force)?
    };
    let sampled = sample_records(
        preset,
        &train_rows,
        &test_rows,
        &train_size,
        &heldout_size,
        req.seed,
    )?;
    write_jsonl(&req.out.join("train.jsonl"), &sampled.train)?;
    write_jsonl(&req.out.join("heldout.jsonl"), &sampled.heldout)?;
    let manifest = json!({
        "schema": "cell-one.classify-import.v0",
        "dataset": preset.alias,
        "hf_id": preset.hf_id,
        "train_size": train_size.token(),
        "heldout_size": heldout_size.token(),
        "seed": req.seed,
        "rows_train": sampled.train.len(),
        "rows_heldout": sampled.heldout.len(),
        "option_order": match preset.map {
            LabelMap::FixedClasses(_) | LabelMap::ThreeWay { .. } => "fixed",
            LabelMap::SampleDistractors { .. } => "shuffled-with-seed",
        },
        "option_order_note": "ag_news options stay in class-table order (A=World, B=Sports, C=Business, D=Sci/Tech) and are not shuffled. The answer letter is that class letter. A later sample_distractors row shuffles the chosen options with --seed and the answer letter follows the shuffle.",
        "heldout_split": preset.test_split,
        "official_train": preset.official_train,
        "official_test": preset.official_test,
        "license_note": preset.license_note,
        "live_train": false,
        "note": "classify import writes tev1 JSONL for local training. It does not train. Do not redistribute the rows."
    });
    fs::write(
        req.out.join("import.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    let order = match preset.map {
        LabelMap::FixedClasses(_) | LabelMap::ThreeWay { .. } => "fixed",
        LabelMap::SampleDistractors { .. } => "shuffled-with-seed",
    };
    println!("{}", preset.license_note);
    println!(
        "dataset={} hf={} train={} heldout={} seed={} option_order={order} out={}",
        preset.alias,
        preset.hf_id,
        sampled.train.len(),
        sampled.heldout.len(),
        req.seed,
        req.out.display()
    );
    Ok(())
}

fn download_native(
    preset: &DatasetPreset,
    out: &Path,
    force: bool,
) -> Result<(Vec<NativeRow>, Vec<NativeRow>)> {
    let native = out.join("native");
    let train_path = native.join(format!("{}.jsonl", preset.train_split));
    let test_path = native.join(format!("{}.jsonl", preset.test_split));
    let marker = native.join("manifest.json");
    let cached = marker.is_file() && train_path.is_file() && test_path.is_file();
    if cached && !force {
        let text = fs::read_to_string(&marker).unwrap_or_default();
        if text.contains(preset.hf_id) {
            return Ok((read_native(&train_path)?, read_native(&test_path)?));
        }
    }
    fs::create_dir_all(&native)?;
    let train_rows = fetch_split(preset, preset.train_split, &train_path)?;
    let test_rows = fetch_split(preset, preset.test_split, &test_path)?;
    let manifest = json!({
        "hf_id": preset.hf_id,
        "source": "datasets-server rows API",
        "train_rows": train_rows.len(),
        "test_rows": test_rows.len(),
    });
    fs::write(
        &marker,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok((train_rows, test_rows))
}

fn fetch_split(preset: &DatasetPreset, split: &str, dest: &Path) -> Result<Vec<NativeRow>> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(120))
        .build();
    let mut offset = 0u64;
    let mut total = None;
    let mut file = File::create(dest)?;
    let mut rows = Vec::new();
    loop {
        if let Some(total) = total {
            if offset >= total {
                break;
            }
        }
        let url = format!(
            "{DATASETS_SERVER}/rows?dataset={}&config=default&split={split}&offset={offset}&length={ROWS_PAGE}",
            urlencoding_dataset(preset.hf_id)
        );
        let response = agent
            .get(&url)
            .call()
            .map_err(|err| anyhow::anyhow!("refuse:classify-import: datasets-server {url} failed: {err}"))?;
        let body = response.into_string().map_err(|err| {
            anyhow::anyhow!("refuse:classify-import: datasets-server body failed: {err}")
        })?;
        let value: Value = serde_json::from_str(&body).map_err(|err| {
            anyhow::anyhow!("refuse:classify-import: datasets-server JSON failed: {err}")
        })?;
        if total.is_none() {
            total = value.get("num_rows_total").and_then(Value::as_u64);
        }
        let page = value
            .get("rows")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("refuse:classify-import: datasets-server page has no rows"))?;
        if page.is_empty() {
            break;
        }
        for item in page {
            let row_idx = item.get("row_idx").and_then(Value::as_u64).unwrap_or(offset);
            let native = item.get("row").cloned().unwrap_or(Value::Null);
            let parsed = parse_server_row(preset, row_idx, &native)?;
            writeln!(file, "{}", serde_json::to_string(&native_json(&parsed))?)?;
            rows.push(parsed);
            offset = offset.saturating_add(1);
        }
        eprintln!(
            "classify-import: {split} {}{}",
            rows.len(),
            total.map(|n| format!("/{n}")).unwrap_or_default()
        );
    }
    if rows.is_empty() {
        bail!("refuse:classify-import: {split} download returned no rows");
    }
    Ok(rows)
}

fn urlencoding_dataset(id: &str) -> String {
    id.replace('/', "%2F")
}

fn native_json(row: &NativeRow) -> Value {
    json!({
        "index": row.index,
        "text": row.text,
        "premise": row.premise,
        "hypothesis": row.hypothesis,
        "label": row.label,
    })
}

fn parse_server_row(preset: &DatasetPreset, index: u64, row: &Value) -> Result<NativeRow> {
    let label = row
        .get(preset.label_field)
        .and_then(Value::as_u64)
        .or_else(|| row.get(preset.label_field).and_then(Value::as_i64).map(|n| n as u64))
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-import: row {index} has no label"))?;
    let text = row
        .get(preset.text_field)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let premise = row
        .get("premise")
        .and_then(Value::as_str)
        .map(str::to_string);
    let hypothesis = row
        .get("hypothesis")
        .and_then(Value::as_str)
        .map(str::to_string);
    if text.trim().is_empty() && premise.as_deref().unwrap_or("").trim().is_empty() {
        bail!("refuse:classify-import: row {index} has no text");
    }
    Ok(NativeRow {
        index,
        text,
        premise,
        hypothesis,
        label: label as usize,
    })
}

pub fn read_native(path: &Path) -> Result<Vec<NativeRow>> {
    let file = File::open(path).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-import: cannot read {}: {err}",
            path.display()
        )
    })?;
    let mut rows = Vec::new();
    for (line_no, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|err| {
            anyhow::anyhow!("refuse:classify-import: {}: {err}", path.display())
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-import: {} line {} is not JSON: {err}",
                path.display(),
                line_no + 1
            )
        })?;
        let index = value
            .get("index")
            .and_then(Value::as_u64)
            .unwrap_or(line_no as u64);
        rows.push(parse_server_row(
            &DatasetPreset {
                label_field: "label",
                text_field: "text",
                ..*preset_by_name("ag_news").expect("ag_news")
            },
            index,
            &value,
        )?);
    }
    Ok(rows)
}

fn write_jsonl(path: &Path, rows: &[Value]) -> Result<()> {
    let mut file = File::create(path)?;
    for row in rows {
        writeln!(file, "{}", serde_json::to_string(row)?)?;
    }
    Ok(())
}

/// Classes for a later banking77-style row. Tests pass a tiny table.
#[cfg(test)]
#[allow(dead_code)]
fn leaked(text: &str) -> &'static str {
    Box::leak(text.to_string().into_boxed_str())
}

#[cfg(test)]
#[allow(dead_code)]
pub fn sample_with_map(
    alias: &str,
    question: &str,
    map: LabelMap,
    split_train: &str,
    split_test: &str,
    train_rows: &[NativeRow],
    test_rows: &[NativeRow],
    train_size: &SplitSize,
    heldout_size: &SplitSize,
    seed: u64,
) -> Result<Sampled> {
    let preset = DatasetPreset {
        alias: leaked(alias),
        hf_id: leaked(alias),
        slug: leaked(alias),
        train_split: leaked(split_train),
        test_split: leaked(split_test),
        official_train: train_rows.len() as u64,
        official_test: test_rows.len() as u64,
        question: leaked(question),
        text_field: "text",
        label_field: "label",
        map,
        license_note: "",
        fetch: Fetch::Later,
    };
    sample_records(
        &preset,
        train_rows,
        test_rows,
        train_size,
        heldout_size,
        seed,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native(index: u64, label: usize, text: &str) -> NativeRow {
        NativeRow {
            index,
            text: text.into(),
            premise: None,
            hypothesis: None,
            label,
        }
    }

    fn grid(split_base: u64, per_class: usize, classes: usize) -> Vec<NativeRow> {
        let mut rows = Vec::new();
        for label in 0..classes {
            for n in 0..per_class {
                let index = split_base + (label * per_class + n) as u64;
                rows.push(native(index, label, &format!("row-{label}-{n}")));
            }
        }
        rows
    }

    #[test]
    fn ag_news_sample_is_balanced_fixed_order_and_disjoint() {
        let preset = preset_by_name("ag_news").unwrap();
        let train = grid(0, 10, 4);
        let test = grid(10_000, 6, 4);
        let sampled = sample_records(
            preset,
            &train,
            &test,
            &SplitSize::Count(6),
            &SplitSize::Count(4),
            42,
        )
        .unwrap();
        assert_eq!(sampled.train.len(), 6);
        assert_eq!(sampled.heldout.len(), 4);
        let mut train_ids = BTreeMap::new();
        let mut counts = BTreeMap::new();
        for row in &sampled.train {
            let id = row["id"].as_str().unwrap();
            assert!(id.starts_with("ag_news:train:"), "{id}");
            assert!(train_ids.insert(id.to_string(), ()).is_none());
            let answer = row["answer"].as_str().unwrap();
            *counts.entry(answer.to_string()).or_insert(0) += 1;
            let letters: Vec<&str> = row["options"]
                .as_array()
                .unwrap()
                .iter()
                .map(|opt| opt["label"].as_str().unwrap())
                .collect();
            assert_eq!(letters, ["A", "B", "C", "D"]);
            let keys: Vec<&str> = row["options"]
                .as_array()
                .unwrap()
                .iter()
                .map(|opt| opt["key"].as_str().unwrap())
                .collect();
            assert_eq!(keys, ["world", "sports", "business", "sci_tech"]);
            let idx = answer.chars().next().unwrap() as usize - 'A' as usize;
            assert_eq!(row["answer_key"].as_str().unwrap(), keys[idx]);
        }
        assert_eq!(counts.len(), 4);
        for n in counts.values() {
            assert!(*n == 1 || *n == 2, "{counts:?}");
        }
        for row in &sampled.heldout {
            let id = row["id"].as_str().unwrap();
            assert!(id.starts_with("ag_news:test:"), "{id}");
            assert!(!train_ids.contains_key(id), "{id}");
        }
        let again = sample_records(
            preset,
            &train,
            &test,
            &SplitSize::Count(6),
            &SplitSize::All,
            42,
        )
        .unwrap();
        assert_eq!(again.heldout.len(), test.len());
        assert_eq!(
            sampled.train[0]["id"], again.train[0]["id"],
            "same seed keeps the train sample"
        );
    }

    #[test]
    fn overlapping_source_ids_are_refused() {
        let classes = AG_NEWS_CLASSES;
        let train = vec![native(1, 0, "a"), native(2, 1, "b")];
        let test = vec![native(1, 2, "c"), native(3, 3, "d")];
        let err = sample_with_map(
            "ag_news",
            "Which topic is this news article?",
            LabelMap::FixedClasses(classes),
            "train",
            "train",
            &train,
            &test,
            &SplitSize::All,
            &SplitSize::All,
            42,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("disjoint"), "{err}");
    }

    #[test]
    fn distractors_shuffle_and_three_way_stay_fixed() {
        const CLASSES: &[ClassSpec] = &[
            ClassSpec {
                key: "a",
                description: "Alpha",
            },
            ClassSpec {
                key: "b",
                description: "Beta",
            },
            ClassSpec {
                key: "c",
                description: "Gamma",
            },
            ClassSpec {
                key: "d",
                description: "Delta",
            },
        ];
        let train = vec![native(1, 0, "pay the bill"), native(2, 1, "lost card")];
        let test = vec![native(8, 2, "branch hours"), native(9, 3, "exchange rate")];
        let sampled = sample_with_map(
            "banking77",
            "Which intent?",
            LabelMap::SampleDistractors {
                classes: CLASSES,
                distractors: 2,
            },
            "train",
            "test",
            &train,
            &test,
            &SplitSize::All,
            &SplitSize::All,
            7,
        )
        .unwrap();
        assert_eq!(sampled.train.len(), 2);
        for row in sampled.train.iter().chain(sampled.heldout.iter()) {
            let opts = row["options"].as_array().unwrap();
            assert_eq!(opts.len(), 3);
            let letters: Vec<&str> = opts.iter().map(|o| o["label"].as_str().unwrap()).collect();
            assert_eq!(letters, ["A", "B", "C"]);
            let answer = row["answer"].as_str().unwrap();
            let pos = letters.iter().position(|l| *l == answer).unwrap();
            assert_eq!(opts[pos]["key"].as_str().unwrap(), row["answer_key"].as_str().unwrap());
        }
        let train_ids: Vec<_> = sampled
            .train
            .iter()
            .map(|r| r["id"].as_str().unwrap().to_string())
            .collect();
        for row in &sampled.heldout {
            assert!(!train_ids.contains(&row["id"].as_str().unwrap().to_string()));
        }

        let mut premise = native(3, 1, "");
        premise.premise = Some("A dog runs.".into());
        premise.hypothesis = Some("An animal moves.".into());
        let mut other = native(4, 2, "");
        other.premise = Some("The bank is closed.".into());
        other.hypothesis = Some("The bank is open.".into());
        let nli = sample_with_map(
            "multi_nli",
            "What is the relationship?",
            LabelMap::ThreeWay {
                classes: MULTI_NLI_CLASSES,
                premise_field: "premise",
                hypothesis_field: "hypothesis",
            },
            "train",
            "validation_matched",
            &[premise],
            &[other],
            &SplitSize::All,
            &SplitSize::All,
            42,
        )
        .unwrap();
        assert_eq!(nli.train[0]["answer"], "B");
        assert_eq!(nli.train[0]["answer_key"], "neutral");
        assert_eq!(nli.heldout[0]["answer"], "C");
        assert!(nli.train[0]["state"]["premise"].as_str().unwrap().contains("dog"));
        assert_ne!(nli.train[0]["id"], nli.heldout[0]["id"]);
    }

    #[test]
    fn fingerprint_includes_sizes_and_seed() {
        let a = import_fingerprint("fancyzhx/ag_news", "3000", "all", 42, "t", "h");
        let b = import_fingerprint("fancyzhx/ag_news", "10000", "all", 42, "t", "h");
        let c = import_fingerprint("fancyzhx/ag_news", "3000", "all", 7, "t", "h");
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn cli_import_from_native_files_stays_offline() {
        let dir = std::env::temp_dir().join(format!("classify-import-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let train = dir.join("native-train.jsonl");
        let test = dir.join("native-test.jsonl");
        fs::write(
            &train,
            concat!(
                "{\"index\":1,\"text\":\"world cup\",\"label\":0}\n",
                "{\"index\":2,\"text\":\"match day\",\"label\":1}\n",
                "{\"index\":3,\"text\":\"shares fell\",\"label\":2}\n",
                "{\"index\":4,\"text\":\"new chip\",\"label\":3}\n",
            ),
        )
        .unwrap();
        fs::write(
            &test,
            concat!(
                "{\"index\":1,\"text\":\"held world\",\"label\":0}\n",
                "{\"index\":5,\"text\":\"held sport\",\"label\":1}\n",
                "{\"index\":6,\"text\":\"held biz\",\"label\":2}\n",
                "{\"index\":7,\"text\":\"held tech\",\"label\":3}\n",
            ),
        )
        .unwrap();
        let out = dir.join("out");
        cmd_classify_import(&ImportRequest {
            dataset: "ag_news",
            train_size: "4",
            heldout_size: "all",
            seed: 42,
            out: &out,
            force: false,
            native_train: Some(&train),
            native_test: Some(&test),
        })
        .unwrap();
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(out.join("import.json")).unwrap()).unwrap();
        assert_eq!(manifest["dataset"], "ag_news");
        assert_eq!(manifest["seed"], 42);
        assert_eq!(manifest["option_order"], "fixed");
        assert!(manifest["license_note"]
            .as_str()
            .unwrap()
            .contains("do not redistribute"));
        let train_ids = fs::read_to_string(out.join("train.jsonl")).unwrap();
        let held_ids = fs::read_to_string(out.join("heldout.jsonl")).unwrap();
        for line in held_ids.lines() {
            let id = serde_json::from_str::<Value>(line).unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string();
            assert!(!train_ids.contains(&id), "{id}");
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
