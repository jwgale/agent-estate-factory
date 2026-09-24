//! Download a public Hugging Face classification set and write tev1 records.
//!
//! The default fetch is `hf download --repo-type dataset` into a cache directory,
//! then a pyarrow read of `train-*.parquet` and `test-*.parquet`. `--from-local`
//! reads a snapshot that is already on disk and does not write into that directory.
//! `--fetch rows-api` is the explicit datasets-server fallback. Tests inject the
//! Python and `hf` runners and never spawn a process or call the network.
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
use std::process::Command;
use std::time::{Duration, UNIX_EPOCH};

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

/// Sampled tev1 rows for one train size and seed. Concurrent sizes do not share this directory.
pub fn sampled_import_dir(alias: &str, train_size: &str, seed: u64) -> PathBuf {
    PathBuf::from(DEFAULT_IMPORT_ROOT).join(format!("{alias}-{train_size}-s{seed}"))
}

/// Full official splits. Shared across train sizes. A manifest exists only after both splits match.
pub fn native_cache_dir(alias: &str) -> PathBuf {
    default_import_dir(alias).join("native")
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
    source: &str,
) -> String {
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(
        format!(
            "classify-import\n{dataset_id}\ntrain={train_size}\nheldout={heldout_size}\nseed={seed}\n{train_hash}\n{held_hash}\nsource={source}"
        )
        .as_bytes(),
    );
    format!("{:x}", hasher.finalize())
}

/// How a full split is obtained when native JSONL was not passed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ImportFetch {
    /// `hf download --repo-type dataset`, then read parquet.
    #[value(name = "bulk")]
    Bulk,
    /// Datasets-server rows API. Explicit fallback.
    #[value(name = "rows-api")]
    RowsApi,
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
    /// HF dataset snapshot. Read-only. `train-*.parquet` and `test-*.parquet` anywhere under it.
    pub from_local: Option<&'a Path>,
    /// Used when `from_local` and the native JSONL pair are both unset. Default is bulk.
    pub fetch: ImportFetch,
    /// Python for pyarrow. Unset uses `ESTATE_PYTHON`, then `python3` on PATH.
    pub python: Option<&'a str>,
    /// Native cache root. Unset uses [`.cell/classify-import`](DEFAULT_IMPORT_ROOT).
    pub cache_root: Option<&'a Path>,
}

pub fn cmd_classify_import(req: &ImportRequest<'_>) -> Result<()> {
    let live = LiveIo::default();
    classify_import_with(req, &live)
}

fn classify_import_with(req: &ImportRequest<'_>, io: &dyn ImportIo) -> Result<()> {
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
    if req.from_local.is_some() && (req.native_train.is_some() || req.native_test.is_some()) {
        bail!(
            "refuse:classify-import: pass either --from-local or --native-train/--native-test, not both"
        );
    }
    fs::create_dir_all(req.out)?;
    let (train_rows, test_rows) =
        if let (Some(train), Some(test)) = (req.native_train, req.native_test) {
            (read_native(train)?, read_native(test)?)
        } else if req.native_train.is_some() || req.native_test.is_some() {
            bail!("refuse:classify-import: pass both --native-train and --native-test, or neither");
        } else {
            acquire_native(preset, req, io)?
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

const MAX_FETCH_ATTEMPTS: u32 = 8;
const PAGE_POLITENESS: Duration = Duration::from_millis(250);

struct PageGet {
    /// HTTP status. `0` is a transport failure.
    status: u16,
    retry_after: Option<Duration>,
    body: String,
    transport: Option<String>,
}

trait ImportIo {
    fn read_parquet(&self, job: &ParquetJob<'_>) -> Result<()>;
    fn hf_download(&self, argv: &[String]) -> Result<()>;
    fn http_get(&self, url: &str) -> PageGet;
    fn pause(&self, delay: Duration);
}

struct ParquetJob<'a> {
    python: Option<&'a str>,
    split: &'a str,
    files: &'a [PathBuf],
    text_field: &'a str,
    label_field: &'a str,
    premise_field: Option<&'a str>,
    hypothesis_field: Option<&'a str>,
    out_partial: &'a Path,
}

struct LiveIo {
    agent: ureq::Agent,
}

impl Default for LiveIo {
    fn default() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(120))
                .build(),
        }
    }
}

impl ImportIo for LiveIo {
    fn read_parquet(&self, job: &ParquetJob<'_>) -> Result<()> {
        live_read_parquet(job)
    }

    fn hf_download(&self, argv: &[String]) -> Result<()> {
        live_hf_download(argv)
    }

    fn http_get(&self, url: &str) -> PageGet {
        live_get(&self.agent, url)
    }

    fn pause(&self, delay: Duration) {
        std::thread::sleep(delay);
    }
}

fn cache_root<'a>(req: &'a ImportRequest<'a>) -> &'a Path {
    req.cache_root.unwrap_or(Path::new(DEFAULT_IMPORT_ROOT))
}

fn native_dir(req: &ImportRequest<'_>, alias: &str) -> PathBuf {
    match req.cache_root {
        Some(root) => root.join(alias).join("native"),
        None => native_cache_dir(alias),
    }
}

fn hf_snapshot_dir(req: &ImportRequest<'_>, alias: &str) -> PathBuf {
    cache_root(req).join(alias).join("hf-dataset")
}

fn acquire_native(
    preset: &DatasetPreset,
    req: &ImportRequest<'_>,
    io: &dyn ImportIo,
) -> Result<(Vec<NativeRow>, Vec<NativeRow>)> {
    let native = native_dir(req, preset.alias);
    fs::create_dir_all(&native)?;
    // Lock only the local native cache. Never the --from-local snapshot.
    let _lock = NativeLock::acquire(&native)?;
    if let Some(dir) = req.from_local {
        return acquire_local_parquet(preset, &native, dir, req.force, req.python, io);
    }
    match req.fetch {
        ImportFetch::RowsApi => load_or_fetch_native(
            preset,
            &native,
            req.force,
            |url| io.http_get(url),
            |delay| io.pause(delay),
        ),
        ImportFetch::Bulk => {
            if !req.force && cache_verified(preset, &native)? {
                let train_final = native.join(format!("{}.jsonl", preset.train_split));
                let test_final = native.join(format!("{}.jsonl", preset.test_split));
                return Ok((read_native(&train_final)?, read_native(&test_final)?));
            }
            let snapshot = hf_snapshot_dir(req, preset.alias);
            fs::create_dir_all(&snapshot)?;
            let bin = crate::classify_journey::hf_bin_name().unwrap_or("hf");
            io.hf_download(&hf_dataset_argv(bin, preset.hf_id, &snapshot))?;
            acquire_local_parquet(preset, &native, &snapshot, true, req.python, io)
        }
    }
}

fn hf_dataset_argv(bin: &str, repo: &str, local_dir: &Path) -> Vec<String> {
    vec![
        bin.to_string(),
        "download".into(),
        repo.to_string(),
        "--repo-type".into(),
        "dataset".into(),
        "--local-dir".into(),
        local_dir.display().to_string(),
    ]
}

#[derive(Clone)]
struct ParquetSource {
    dir_display: String,
    token: String,
    files: Vec<Value>,
}

fn acquire_local_parquet(
    preset: &DatasetPreset,
    native: &Path,
    source_dir: &Path,
    force: bool,
    python: Option<&str>,
    io: &dyn ImportIo,
) -> Result<(Vec<NativeRow>, Vec<NativeRow>)> {
    if !source_dir.is_dir() {
        bail!(
            "refuse:classify-import: --from-local {} is not a directory",
            source_dir.display()
        );
    }
    let source = inspect_parquet_source(source_dir, preset.train_split, preset.test_split)?;
    let train_final = native.join(format!("{}.jsonl", preset.train_split));
    let test_final = native.join(format!("{}.jsonl", preset.test_split));
    if !force && cache_verified(preset, native)? {
        if manifest_source_fp(native)?.as_deref() == Some(source.token.as_str()) {
            return Ok((read_native(&train_final)?, read_native(&test_final)?));
        }
    }
    let (premise_field, hypothesis_field) = nli_fields(preset);
    let train_files = split_parquet_files(source_dir, preset.train_split)?;
    let test_files = split_parquet_files(source_dir, preset.test_split)?;
    let train_rows = read_one_parquet_split(
        preset,
        preset.train_split,
        &train_files,
        &train_final,
        preset.official_train,
        python,
        premise_field,
        hypothesis_field,
        io,
    )?;
    let test_rows = read_one_parquet_split(
        preset,
        preset.test_split,
        &test_files,
        &test_final,
        preset.official_test,
        python,
        premise_field,
        hypothesis_field,
        io,
    )?;
    publish_native(&train_final)?;
    publish_native(&test_final)?;
    let manifest = json!({
        "hf_id": preset.hf_id,
        "source": "local-parquet",
        "source_dir": source.dir_display,
        "source_fp": source.token,
        "parquet": source.files,
        "train_rows": train_rows.len(),
        "test_rows": test_rows.len(),
        "complete": true,
    });
    fs::write(
        native.join("manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok((train_rows, test_rows))
}

fn nli_fields(preset: &DatasetPreset) -> (Option<&'static str>, Option<&'static str>) {
    match preset.map {
        LabelMap::ThreeWay {
            premise_field,
            hypothesis_field,
            ..
        } => (Some(premise_field), Some(hypothesis_field)),
        _ => (None, None),
    }
}

fn read_one_parquet_split(
    preset: &DatasetPreset,
    split: &str,
    files: &[PathBuf],
    final_path: &Path,
    official: u64,
    python: Option<&str>,
    premise_field: Option<&str>,
    hypothesis_field: Option<&str>,
    io: &dyn ImportIo,
) -> Result<Vec<NativeRow>> {
    let partial = partial_path(final_path);
    if let Some(parent) = partial.parent() {
        fs::create_dir_all(parent)?;
    }
    let _ = fs::remove_file(&partial);
    io.read_parquet(&ParquetJob {
        python,
        split,
        files,
        text_field: preset.text_field,
        label_field: preset.label_field,
        premise_field,
        hypothesis_field,
        out_partial: &partial,
    })?;
    let rows = read_native(&partial)?;
    verify_split_count(split, rows.len() as u64, rows.len() as u64, official)?;
    eprintln!(
        "classify-import: {split} {} rows from local parquet",
        rows.len()
    );
    Ok(rows)
}

fn manifest_source_fp(native: &Path) -> Result<Option<String>> {
    let marker = native.join("manifest.json");
    if !marker.is_file() {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(&fs::read_to_string(&marker)?)?;
    Ok(value
        .get("source_fp")
        .and_then(Value::as_str)
        .map(str::to_string))
}

/// Identity of a local snapshot. A changed file changes the import fingerprint.
pub fn dataset_source_token(
    from_local: Option<&Path>,
    fetch: ImportFetch,
    train_split: &str,
    test_split: &str,
) -> String {
    if let Some(dir) = from_local {
        return inspect_parquet_source(dir, train_split, test_split)
            .map(|source| source.token)
            .unwrap_or_else(|_| format!("local-unreadable:{}", dir.display()));
    }
    match fetch {
        ImportFetch::RowsApi => "rows-api".to_string(),
        ImportFetch::Bulk => "hf-download".to_string(),
    }
}

fn file_mtime_secs(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn inspect_parquet_source(dir: &Path, train_split: &str, test_split: &str) -> Result<ParquetSource> {
    let canon = fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let mut paths = split_parquet_files(&canon, train_split)?;
    paths.extend(split_parquet_files(&canon, test_split)?);
    paths.sort();
    paths.dedup();
    let cache_key = format!("{}\n{train_split}\n{test_split}", canon.display());
    let mut cheap_lines = vec![format!("dir={}", canon.display())];
    let mut stats = Vec::new();
    for path in &paths {
        let meta = fs::symlink_metadata(path).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-import: cannot stat {}: {err}",
                path.display()
            )
        })?;
        let mtime = file_mtime_secs(&meta);
        let rel = path
            .strip_prefix(&canon)
            .unwrap_or(path)
            .display()
            .to_string();
        cheap_lines.push(format!("{rel} bytes={} mtime={mtime}", meta.len()));
        stats.push((path.clone(), rel, meta.len(), mtime));
    }
    let cheap = cheap_lines.join("\n");
    if let Some(hit) = remembered_source(&cache_key, &cheap) {
        return Ok(hit);
    }
    let mut lines = vec![format!("dir={}", canon.display())];
    let mut files = Vec::new();
    for (path, rel, bytes, mtime) in &stats {
        let hash = sha256_file(path)?;
        lines.push(format!("{rel} sha256={hash} bytes={bytes} mtime={mtime}"));
        files.push(json!({
            "path": rel,
            "sha256": hash,
            "bytes": bytes,
            "mtime": mtime,
        }));
    }
    let source = ParquetSource {
        dir_display: canon.display().to_string(),
        token: lines.join("\n"),
        files,
    };
    remember_source(&cache_key, &cheap, &source);
    Ok(source)
}

struct RememberedSource {
    cheap: String,
    source: ParquetSource,
}

fn source_cache() -> &'static std::sync::Mutex<std::collections::HashMap<String, RememberedSource>> {
    static CACHE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, RememberedSource>>,
    > = std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn remembered_source(cache_key: &str, cheap: &str) -> Option<ParquetSource> {
    let guard = source_cache().lock().ok()?;
    let hit = guard.get(cache_key)?;
    if hit.cheap == cheap {
        Some(hit.source.clone())
    } else {
        None
    }
}

fn remember_source(cache_key: &str, cheap: &str, source: &ParquetSource) {
    if let Ok(mut guard) = source_cache().lock() {
        guard.insert(
            cache_key.to_string(),
            RememberedSource {
                cheap: cheap.to_string(),
                source: source.clone(),
            },
        );
    }
}

fn split_parquet_files(dir: &Path, split: &str) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    collect_parquet(dir, split, &mut seen, &mut found)?;
    found.sort();
    if found.is_empty() {
        bail!(
            "refuse:classify-import: no {split}-*.parquet under {}",
            dir.display()
        );
    }
    Ok(found)
}

fn collect_parquet(
    dir: &Path,
    split: &str,
    seen: &mut std::collections::BTreeSet<(u64, u64)>,
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let dir_meta = fs::symlink_metadata(dir).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-import: cannot stat {}: {err}",
            dir.display()
        )
    })?;
    if dir_meta.file_type().is_symlink() {
        bail!(
            "refuse:classify-import: directory symlink {} is refused",
            dir.display()
        );
    }
    if !seen.insert((dir_meta.dev(), dir_meta.ino())) {
        bail!(
            "refuse:classify-import: directory cycle at {}",
            dir.display()
        );
    }
    let entries = fs::read_dir(dir).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-import: cannot read {}: {err}",
            dir.display()
        )
    })?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-import: cannot stat {}: {err}",
                path.display()
            )
        })?;
        if meta.file_type().is_symlink() {
            let target_dir = fs::metadata(&path).map(|target| target.is_dir()).unwrap_or(false);
            if target_dir {
                bail!(
                    "refuse:classify-import: directory symlink {} is refused",
                    path.display()
                );
            }
            eprintln!("classify-import: skip symlink {}", path.display());
            continue;
        }
        if meta.is_dir() {
            collect_parquet(&path, split, seen, out)?;
        } else if meta.is_file() && is_split_parquet(&path, split) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_split_parquet(path: &Path, split: &str) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let prefix = format!("{split}-");
    name.starts_with(&prefix) && name.ends_with(".parquet")
}

fn sha256_file(path: &Path) -> Result<String> {
    use sha2::Digest;
    use std::io::Read;
    // Read-only stream. A gvfs SMB mount rejects lock and fchmod calls, so this
    // must not create a sidecar, a temp file, or a lock next to the parquet.
    let mut file = File::open(path).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-import: cannot read {}: {err}",
            path.display()
        )
    })?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-import: cannot read {}: {err}",
                path.display()
            )
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn pyarrow_refuse(python: &str) -> String {
    format!(
        "refuse:classify-import: pyarrow is missing for `{python}`. Install it with `{python} -m pip install pyarrow`."
    )
}

fn python_failure(python: &str, code: i32, stderr: &str) -> String {
    if code == 2 || stderr.contains("pyarrow-missing") {
        pyarrow_refuse(python)
    } else {
        format!("refuse:classify-import: {python} exited {code}: {stderr}")
    }
}

const PARQUET_SCRIPT: &str = r#"
import json, numbers, sys
try:
    import pyarrow.parquet as pq
except Exception:
    sys.stderr.write("pyarrow-missing\n")
    sys.exit(2)
spec = json.loads(sys.stdin.read())
text_field = spec["text_field"]
label_field = spec["label_field"]
premise_field = spec.get("premise_field") or ""
hypothesis_field = spec.get("hypothesis_field") or ""
index = 0
with open(spec["out"], "w", encoding="utf-8") as out:
    for path in spec["files"]:
        # Read-only. Do not lock, chmod, or write next to the parquet (gvfs has no fchmod).
        with open(path, "rb") as handle:
            table = pq.read_table(handle)
        names = table.column_names
        cols = {name: table.column(name).to_pylist() for name in names}
        n = table.num_rows
        if text_field not in cols or label_field not in cols:
            sys.stderr.write("missing column\n")
            sys.exit(3)
        for i in range(n):
            label = cols[label_field][i]
            if isinstance(label, bool) or not isinstance(label, numbers.Integral):
                sys.stderr.write("label is not an int\n")
                sys.exit(3)
            text = cols[text_field][i]
            text = "" if text is None else str(text)
            row = {"index": index, "text": text, "label": int(label)}
            if premise_field:
                val = cols.get(premise_field, [None] * n)[i]
                row["premise"] = None if val is None else str(val)
            if hypothesis_field:
                val = cols.get(hypothesis_field, [None] * n)[i]
                row["hypothesis"] = None if val is None else str(val)
            out.write(json.dumps(row, ensure_ascii=False) + "\n")
            index += 1
"#;

fn resolve_python(flag: Option<&str>) -> Result<String> {
    if let Some(flag) = flag.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(flag.to_string());
    }
    if let Ok(env) = std::env::var("ESTATE_PYTHON") {
        let env = env.trim();
        if !env.is_empty() {
            return Ok(env.to_string());
        }
    }
    if which_bin("python3").is_some() {
        return Ok("python3".into());
    }
    bail!(
        "refuse:classify-import: python3 is not on PATH. Pass --python or set ESTATE_PYTHON. pyarrow is required (`python3 -m pip install pyarrow`)."
    )
}

fn which_bin(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn live_read_parquet(job: &ParquetJob<'_>) -> Result<()> {
    let python = resolve_python(job.python)?;
    let bin = if python.contains('/') || python.contains('\\') {
        PathBuf::from(&python)
    } else {
        which_bin(&python).ok_or_else(|| {
            anyhow::anyhow!(
                "refuse:classify-import: {python} is not on PATH. Pass --python or set ESTATE_PYTHON. pyarrow is required (`{python} -m pip install pyarrow`)."
            )
        })?
    };
    let spec = json!({
        "split": job.split,
        "files": job.files.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        "text_field": job.text_field,
        "label_field": job.label_field,
        "premise_field": job.premise_field,
        "hypothesis_field": job.hypothesis_field,
        "out": job.out_partial.display().to_string(),
    });
    let mut child = Command::new(&bin)
        .arg("-c")
        .arg(PARQUET_SCRIPT)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-import: cannot run {}: {err}",
                bin.display()
            )
        })?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("refuse:classify-import: {python} stdin is closed"))?;
        stdin.write_all(spec.to_string().as_bytes())?;
    }
    let output = child.wait_with_output().map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-import: cannot wait for {}: {err}",
            bin.display()
        )
    })?;
    if !output.status.success() {
        let code = output.status.code().unwrap_or(1);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}", python_failure(&python, code, stderr.trim()));
    }
    Ok(())
}

fn live_hf_download(argv: &[String]) -> Result<()> {
    let (bin, args) = argv
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-import: empty hf command"))?;
    let bin_path = which_bin(bin).ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-import: huggingface-cli and hf are not on PATH; needed to download the dataset. HF_TOKEN is read from the environment and is not printed"
        )
    })?;
    println!(
        "{}",
        argv.iter()
            .map(|part| part.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );
    let output = Command::new(&bin_path).args(args).output().map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-import: cannot run {}: {err}",
            bin_path.display()
        )
    })?;
    let _ = std::io::stdout().write_all(&output.stdout);
    let _ = std::io::stderr().write_all(&output.stderr);
    if !output.status.success() {
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if crate::classify_journey::hf_downloader_deprecated(&combined) {
            bail!(
                "refuse:classify-import: {bin} is deprecated and no longer downloads. {}.",
                crate::classify_journey::HF_DEPRECATION_HINT
            );
        }
        bail!(
            "refuse:classify-import: {} exited {}",
            bin_path.display(),
            output.status
        );
    }
    Ok(())
}

fn load_or_fetch_native(
    preset: &DatasetPreset,
    native: &Path,
    force: bool,
    mut get: impl FnMut(&str) -> PageGet,
    mut sleep: impl FnMut(Duration),
) -> Result<(Vec<NativeRow>, Vec<NativeRow>)> {
    let train_final = native.join(format!("{}.jsonl", preset.train_split));
    let test_final = native.join(format!("{}.jsonl", preset.test_split));
    let marker = native.join("manifest.json");
    if force {
        let _ = fs::remove_file(&train_final);
        let _ = fs::remove_file(&test_final);
        let _ = fs::remove_file(&marker);
        let _ = fs::remove_file(partial_path(&train_final));
        let _ = fs::remove_file(partial_path(&test_final));
    } else if cache_verified(preset, native)? {
        return Ok((read_native(&train_final)?, read_native(&test_final)?));
    }
    let train_rows = fetch_split(
        preset,
        preset.train_split,
        &train_final,
        ROWS_PAGE,
        preset.official_train,
        &mut get,
        &mut sleep,
    )?;
    let test_rows = fetch_split(
        preset,
        preset.test_split,
        &test_final,
        ROWS_PAGE,
        preset.official_test,
        &mut get,
        &mut sleep,
    )?;
    publish_native(&train_final)?;
    publish_native(&test_final)?;
    let manifest = json!({
        "hf_id": preset.hf_id,
        "source": "datasets-server rows API",
        "train_rows": train_rows.len(),
        "test_rows": test_rows.len(),
        "complete": true,
    });
    fs::write(
        &marker,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok((train_rows, test_rows))
}

fn partial_path(final_path: &Path) -> PathBuf {
    let mut name = final_path.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    final_path.with_file_name(name)
}

fn cache_verified(preset: &DatasetPreset, native: &Path) -> Result<bool> {
    let marker = native.join("manifest.json");
    let train_path = native.join(format!("{}.jsonl", preset.train_split));
    let test_path = native.join(format!("{}.jsonl", preset.test_split));
    if !marker.is_file() || !train_path.is_file() || !test_path.is_file() {
        return Ok(false);
    }
    let value: Value = serde_json::from_str(&fs::read_to_string(&marker)?)?;
    if value.get("hf_id").and_then(Value::as_str) != Some(preset.hf_id) {
        return Ok(false);
    }
    if value.get("complete").and_then(Value::as_bool) != Some(true) {
        return Ok(false);
    }
    let train_rows = value.get("train_rows").and_then(Value::as_u64);
    let test_rows = value.get("test_rows").and_then(Value::as_u64);
    Ok(train_rows == Some(preset.official_train)
        && test_rows == Some(preset.official_test)
        && count_complete_lines(&train_path)? == preset.official_train
        && count_complete_lines(&test_path)? == preset.official_test)
}

fn publish_native(final_path: &Path) -> Result<()> {
    let partial = partial_path(final_path);
    if !partial.is_file() {
        bail!(
            "refuse:classify-import: missing partial {}",
            partial.display()
        );
    }
    fs::rename(&partial, final_path).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-import: cannot publish {}: {err}",
            final_path.display()
        )
    })?;
    Ok(())
}

fn fetch_split(
    preset: &DatasetPreset,
    split: &str,
    final_path: &Path,
    page_len: u64,
    official: u64,
    mut get: impl FnMut(&str) -> PageGet,
    mut sleep: impl FnMut(Duration),
) -> Result<Vec<NativeRow>> {
    let partial = partial_path(final_path);
    if let Some(parent) = partial.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut offset = rewind_to_complete_pages(&partial, page_len)?;
    let mut rows = if offset == 0 {
        Vec::new()
    } else {
        read_native(&partial)?
    };
    if rows.len() as u64 != offset {
        bail!(
            "refuse:classify-import: {split} partial line count {} does not match resume offset {offset}",
            rows.len()
        );
    }
    let mut total = None;
    loop {
        if let Some(total) = total {
            if offset >= total {
                break;
            }
        }
        let url = format!(
            "{DATASETS_SERVER}/rows?dataset={}&config=default&split={split}&offset={offset}&length={page_len}",
            urlencoding_dataset(preset.hf_id)
        );
        let body = get_with_retry(&url, &mut get, &mut sleep)?;
        let value: Value = serde_json::from_str(&body).map_err(|err| {
            anyhow::anyhow!("refuse:classify-import: datasets-server JSON failed: {err}")
        })?;
        if total.is_none() {
            total = value.get("num_rows_total").and_then(Value::as_u64);
        }
        let reported = total.ok_or_else(|| {
            anyhow::anyhow!("refuse:classify-import: {split} page has no num_rows_total")
        })?;
        let page = value.get("rows").and_then(Value::as_array).ok_or_else(|| {
            anyhow::anyhow!("refuse:classify-import: datasets-server page has no rows")
        })?;
        if page.is_empty() {
            if offset >= reported {
                break;
            }
            bail!(
                "refuse:classify-import: {split} returned an empty page at offset {offset} before num_rows_total {reported}"
            );
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&partial)?;
        for item in page {
            let row_idx = item.get("row_idx").and_then(Value::as_u64).unwrap_or(offset);
            let native = item.get("row").cloned().unwrap_or(Value::Null);
            let parsed = parse_server_row(preset, row_idx, &native)?;
            writeln!(file, "{}", serde_json::to_string(&native_json(&parsed))?)?;
            rows.push(parsed);
            offset = offset.saturating_add(1);
        }
        file.flush()?;
        eprintln!("classify-import: {split} {offset}/{reported}");
        if offset < reported {
            sleep(PAGE_POLITENESS);
        }
    }
    let reported = total.unwrap_or(0);
    verify_split_count(split, rows.len() as u64, reported, official)?;
    Ok(rows)
}

fn verify_split_count(split: &str, got: u64, reported: u64, official: u64) -> Result<()> {
    if got != reported {
        bail!(
            "refuse:classify-import: {split} has {got} rows but num_rows_total is {reported}"
        );
    }
    if got != official {
        bail!(
            "refuse:classify-import: {split} has {got} rows but the official count is {official}"
        );
    }
    Ok(())
}

fn get_with_retry(
    url: &str,
    get: &mut impl FnMut(&str) -> PageGet,
    sleep: &mut impl FnMut(Duration),
) -> Result<String> {
    let mut last = String::from("no response");
    for attempt in 1..=MAX_FETCH_ATTEMPTS {
        let page = get(url);
        let retryable = page.transport.is_some() || page.status == 429 || (500..600).contains(&page.status);
        if !retryable && page.status == 200 {
            return Ok(page.body);
        }
        last = if let Some(err) = page.transport {
            format!("transport error: {err}")
        } else {
            format!("HTTP {}", page.status)
        };
        if !retryable {
            bail!("refuse:classify-import: datasets-server {url} failed: {last}");
        }
        if attempt == MAX_FETCH_ATTEMPTS {
            break;
        }
        sleep(retry_wait(attempt, page.retry_after));
    }
    bail!(
        "refuse:classify-import: datasets-server {url} failed after {MAX_FETCH_ATTEMPTS} attempts: {last}"
    );
}

fn backoff_delay(attempt: u32) -> Duration {
    let shift = attempt.saturating_sub(1).min(6);
    let base = 200u64.saturating_mul(1u64 << shift);
    let jitter = u64::from(attempt) * 53 % 100;
    Duration::from_millis(base + jitter)
}

fn hf_auth_header(token: Option<&str>) -> Option<String> {
    let token = token?.trim();
    if token.is_empty() {
        None
    } else {
        Some(format!("Bearer {token}"))
    }
}

fn live_get(agent: &ureq::Agent, url: &str) -> PageGet {
    let mut req = agent.get(url);
    if let Some(header) = hf_auth_header(std::env::var("HF_TOKEN").ok().as_deref()) {
        req = req.set("Authorization", &header);
    }
    match req.call() {
        Ok(resp) => PageGet {
            status: resp.status(),
            retry_after: resp.header("retry-after").and_then(parse_retry_after),
            body: resp.into_string().unwrap_or_default(),
            transport: None,
        },
        Err(ureq::Error::Status(code, resp)) => PageGet {
            status: code,
            retry_after: resp.header("retry-after").and_then(parse_retry_after),
            body: resp.into_string().unwrap_or_default(),
            transport: None,
        },
        Err(err) => PageGet {
            status: 0,
            retry_after: None,
            body: String::new(),
            transport: Some(err.to_string()),
        },
    }
}

const MAX_RETRY_AFTER: Duration = Duration::from_secs(60);

fn parse_retry_after(header: &str) -> Option<Duration> {
    let secs = header.trim().parse::<u64>().ok()?;
    Some(Duration::from_secs(secs.min(MAX_RETRY_AFTER.as_secs())))
}

/// Honor `Retry-After`, but never wait longer than 60s or less than this attempt's backoff.
fn retry_wait(attempt: u32, retry_after: Option<Duration>) -> Duration {
    let floor = backoff_delay(attempt);
    retry_after
        .unwrap_or(floor)
        .min(MAX_RETRY_AFTER)
        .max(floor)
}

/// Exclusive `native/.lock` for the whole download, verify, and publish.
/// Rust 1.88 has no `File::lock`, so this is a `create_new` pid file.
struct NativeLock {
    path: PathBuf,
}

impl NativeLock {
    fn acquire(native: &Path) -> Result<Self> {
        fs::create_dir_all(native)?;
        let path = native.join(".lock");
        match File::options().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                writeln!(file, "{}", std::process::id())?;
                file.sync_all()?;
                Ok(Self { path })
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                let pid = fs::read_to_string(&path)
                    .ok()
                    .map(|text| text.trim().to_string())
                    .filter(|text| !text.is_empty())
                    .unwrap_or_else(|| "unknown".to_string());
                bail!(
                    "refuse:classify-import: another import is running (pid {pid}); remove {} if stale",
                    path.display()
                );
            }
            Err(err) => bail!(
                "refuse:classify-import: cannot lock {}: {err}",
                path.display()
            ),
        }
    }
}

impl Drop for NativeLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn rewind_to_complete_pages(path: &Path, page: u64) -> Result<u64> {
    if !path.is_file() {
        return Ok(0);
    }
    let bytes = fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let complete: Vec<&str> = text
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n') && !line.trim().is_empty())
        .collect();
    let n = complete.len() as u64;
    let keep = if page == 0 { n } else { n - (n % page) };
    let torn = !bytes.ends_with(b"\n");
    if keep != n || torn {
        let mut file = File::create(path)?;
        for line in complete.iter().take(keep as usize) {
            file.write_all(line.as_bytes())?;
        }
    }
    Ok(keep)
}

fn count_complete_lines(path: &Path) -> Result<u64> {
    if !path.is_file() {
        return Ok(0);
    }
    let file = File::open(path)?;
    let mut n = 0u64;
    for line in BufReader::new(file).lines() {
        let line = line?;
        if !line.trim().is_empty() {
            n += 1;
        }
    }
    Ok(n)
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
        let a = import_fingerprint("fancyzhx/ag_news", "3000", "all", 42, "t", "h", "src");
        let b = import_fingerprint("fancyzhx/ag_news", "10000", "all", 42, "t", "h", "src");
        let c = import_fingerprint("fancyzhx/ag_news", "3000", "all", 7, "t", "h", "src");
        let d = import_fingerprint("fancyzhx/ag_news", "3000", "all", 42, "t", "h", "other");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
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
            from_local: None,
            fetch: ImportFetch::Bulk,
            python: None,
            cache_root: None,
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

    fn page_body(total: u64, rows: &[(u64, &str, u64)]) -> String {
        let rows: Vec<Value> = rows
            .iter()
            .map(|(idx, text, label)| {
                json!({
                    "row_idx": idx,
                    "row": {"text": text, "label": label}
                })
            })
            .collect();
        json!({"num_rows_total": total, "rows": rows}).to_string()
    }

    fn ok_page(body: String) -> PageGet {
        PageGet {
            status: 200,
            retry_after: None,
            body,
            transport: None,
        }
    }

    #[test]
    fn empty_page_before_total_refuses_and_leaves_the_partial() {
        let dir = std::env::temp_dir().join(format!("import-empty-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("train.jsonl");
        let preset = preset_by_name("ag_news").unwrap();
        let mut calls = 0;
        let err = fetch_split(
            preset,
            "train",
            &dest,
            2,
            4,
            |_| {
                calls += 1;
                if calls == 1 {
                    ok_page(page_body(4, &[(0, "a", 0), (1, "b", 1)]))
                } else {
                    ok_page(page_body(4, &[]))
                }
            },
            |_| {},
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("empty page"), "{err}");
        assert!(err.contains("offset 2"), "{err}");
        assert!(partial_path(&dest).is_file());
        assert!(!dest.is_file());
        assert!(!dir.join("manifest.json").is_file());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn http_429_retries_then_succeeds() {
        let dir = std::env::temp_dir().join(format!("import-retry-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("train.jsonl");
        let preset = preset_by_name("ag_news").unwrap();
        let mut calls = 0;
        let mut waits = Vec::new();
        let rows = fetch_split(
            preset,
            "train",
            &dest,
            2,
            2,
            |_| {
                calls += 1;
                if calls == 1 {
                    PageGet {
                        status: 429,
                        retry_after: Some(Duration::from_secs(3)),
                        body: String::new(),
                        transport: None,
                    }
                } else {
                    ok_page(page_body(2, &[(0, "a", 0), (1, "b", 1)]))
                }
            },
            |wait| waits.push(wait),
        )
        .unwrap();
        assert_eq!(calls, 2);
        assert_eq!(rows.len(), 2);
        assert!(waits.iter().any(|wait| *wait >= Duration::from_secs(3)));
        assert!(!dest.is_file(), "partial stays unpublished until both splits verify");
        assert_eq!(count_complete_lines(&partial_path(&dest)).unwrap(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resume_reads_the_last_complete_page_and_count_mismatch_refuses() {
        let dir = std::env::temp_dir().join(format!("import-resume-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("train.jsonl");
        let partial = partial_path(&dest);
        fs::write(
            &partial,
            concat!(
                "{\"index\":0,\"text\":\"a\",\"premise\":null,\"hypothesis\":null,\"label\":0}\n",
                "{\"index\":1,\"text\":\"b\",\"premise\":null,\"hypothesis\":null,\"label\":1}\n",
                "{\"index\":2,\"text\":\"torn"
            ),
        )
        .unwrap();
        let preset = preset_by_name("ag_news").unwrap();
        let mut urls = Vec::new();
        let rows = fetch_split(
            preset,
            "train",
            &dest,
            2,
            4,
            |url| {
                urls.push(url.to_string());
                ok_page(page_body(4, &[(2, "c", 2), (3, "d", 3)]))
            },
            |_| {},
        )
        .unwrap();
        assert_eq!(rows.len(), 4);
        assert!(urls[0].contains("offset=2"), "{}", urls[0]);
        assert!(!dest.is_file());

        let mismatch = dir.join("test.jsonl");
        let err = fetch_split(
            preset,
            "test",
            &mismatch,
            2,
            7_600,
            |_| ok_page(page_body(2, &[(0, "a", 0), (1, "b", 1)])),
            |_| {},
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("official count"), "{err}");
        assert!(!mismatch.is_file());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn retry_after_clamps_huge_values_and_rejects_garbage() {
        assert_eq!(parse_retry_after("86400"), Some(Duration::from_secs(60)));
        assert_eq!(parse_retry_after("  120  "), Some(Duration::from_secs(60)));
        assert_eq!(parse_retry_after("3"), Some(Duration::from_secs(3)));
        assert!(parse_retry_after("soon").is_none());
        assert!(parse_retry_after("").is_none());
        assert!(parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT").is_none());
        assert_eq!(
            retry_wait(1, Some(Duration::from_secs(86_400))),
            Duration::from_secs(60)
        );
        let floor = backoff_delay(1);
        assert_eq!(retry_wait(1, Some(Duration::ZERO)), floor);
        assert_eq!(retry_wait(1, None), floor);
        assert!(floor < Duration::from_secs(60));
    }

    #[test]
    fn native_lock_blocks_a_second_import_and_a_verified_cache_skips_download() {
        let dir = std::env::temp_dir().join(format!("import-lock-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let held = NativeLock::acquire(&dir).unwrap();
        let err = match NativeLock::acquire(&dir) {
            Err(err) => err.to_string(),
            Ok(_) => panic!("second native lock acquired"),
        };
        assert!(err.contains("another import is running"), "{err}");
        assert!(err.contains(&std::process::id().to_string()), "{err}");
        assert!(err.contains("if stale"), "{err}");
        assert!(err.contains(&dir.join(".lock").display().to_string()), "{err}");
        assert!(dir.join(".lock").is_file());
        drop(held);
        assert!(!dir.join(".lock").is_file());
        let again = NativeLock::acquire(&dir).unwrap();
        drop(again);

        let preset = preset_by_name("ag_news").unwrap();
        let train = dir.join("train.jsonl");
        let test = dir.join("test.jsonl");
        let row = "{\"index\":0,\"text\":\"a\",\"label\":0}\n";
        fs::write(&train, row.repeat(preset.official_train as usize)).unwrap();
        fs::write(&test, row.repeat(preset.official_test as usize)).unwrap();
        fs::write(
            dir.join("manifest.json"),
            format!(
                "{{\"hf_id\":{},\"complete\":true,\"train_rows\":{},\"test_rows\":{}}}\n",
                serde_json::to_string(preset.hf_id).unwrap(),
                preset.official_train,
                preset.official_test
            ),
        )
        .unwrap();
        let mut calls = 0;
        let (train_rows, test_rows) = load_or_fetch_native(
            preset,
            &dir,
            false,
            |_| {
                calls += 1;
                ok_page(page_body(1, &[]))
            },
            |_| {},
        )
        .unwrap();
        assert_eq!(calls, 0);
        assert_eq!(train_rows.len() as u64, preset.official_train);
        assert_eq!(test_rows.len() as u64, preset.official_test);
        assert!(
            read_native(&train).is_ok(),
            "index/text/label lines stay accepted"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    struct FakeIo {
        hf_calls: std::cell::Cell<u32>,
        http_calls: std::cell::Cell<u32>,
        pyarrow_missing: bool,
        touch_source: bool,
    }

    impl ImportIo for FakeIo {
        fn read_parquet(&self, job: &ParquetJob<'_>) -> Result<()> {
            if self.pyarrow_missing {
                bail!("{}", pyarrow_refuse(job.python.unwrap_or("python3")));
            }
            if self.touch_source {
                if let Some(file) = job.files.first() {
                    if let Some(dir) = file.parent() {
                        let _ = fs::write(dir.join("written-by-import"), b"no");
                    }
                }
            }
            let preset = preset_by_name("ag_news").unwrap();
            let n = if job.split == preset.train_split {
                preset.official_train
            } else {
                preset.official_test
            };
            use std::io::Write;
            let mut file = std::fs::File::create(job.out_partial)?;
            for i in 0..n {
                writeln!(
                    file,
                    "{{\"index\":{i},\"text\":\"row-{i}\",\"label\":{}}}",
                    i % 4
                )?;
            }
            Ok(())
        }

        fn hf_download(&self, argv: &[String]) -> Result<()> {
            self.hf_calls.set(self.hf_calls.get() + 1);
            assert!(argv.iter().any(|arg| arg == "--repo-type"), "{argv:?}");
            assert!(argv.iter().any(|arg| arg == "dataset"), "{argv:?}");
            let dir = argv
                .windows(2)
                .find(|pair| pair[0] == "--local-dir")
                .map(|pair| std::path::PathBuf::from(&pair[1]))
                .ok_or_else(|| {
                    anyhow::anyhow!("refuse:classify-import: hf argv has no --local-dir")
                })?;
            fs::create_dir_all(dir.join("data"))?;
            fs::write(dir.join("data").join("train-00000-of-00001.parquet"), b"t")?;
            fs::write(dir.join("data").join("test-00000-of-00001.parquet"), b"e")?;
            Ok(())
        }

        fn http_get(&self, _url: &str) -> PageGet {
            self.http_calls.set(self.http_calls.get() + 1);
            ok_page(page_body(1, &[(0, "a", 0)]))
        }

        fn pause(&self, _delay: Duration) {}
    }

    fn snapshot_dir(root: &Path, name: &str, train_bytes: &[u8], test_bytes: &[u8]) -> PathBuf {
        let dir = root.join(name).join("data");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("train-00000-of-00001.parquet"), train_bytes).unwrap();
        fs::write(dir.join("test-00000-of-00001.parquet"), test_bytes).unwrap();
        root.join(name)
    }

    fn list_tree(dir: &Path) -> Vec<(String, Vec<u8>, u32)> {
        use std::os::unix::fs::PermissionsExt;
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            let mode = fs::metadata(&current).unwrap().permissions().mode();
            let rel = current.strip_prefix(dir).unwrap().display().to_string();
            let label = if current == dir {
                "dir:.".to_string()
            } else {
                format!("dir:{rel}")
            };
            out.push((label, Vec::new(), mode));
            let mut entries: Vec<_> = fs::read_dir(&current)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
            entries.sort();
            for path in entries {
                if path.is_dir() {
                    stack.push(path);
                } else {
                    let rel = path.strip_prefix(dir).unwrap().display().to_string();
                    let mode = fs::metadata(&path).unwrap().permissions().mode();
                    out.push((rel, fs::read(&path).unwrap(), mode));
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn from_local_parquet_verifies_counts_and_does_not_write_the_source() {
        let root = std::env::temp_dir().join(format!(
            "import-local-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let source = snapshot_dir(&root, "nas", b"train-bytes", b"test-bytes");
        let before = list_tree(&source);
        let before_names: Vec<_> = before.iter().map(|(name, _, _)| name.clone()).collect();
        let out = root.join("sampled");
        let cache = root.join("cache");
        let io = FakeIo {
            hf_calls: std::cell::Cell::new(0),
            http_calls: std::cell::Cell::new(0),
            pyarrow_missing: false,
            touch_source: false,
        };
        classify_import_with(
            &ImportRequest {
                dataset: "ag_news",
                train_size: "4",
                heldout_size: "4",
                seed: 42,
                out: &out,
                force: false,
                native_train: None,
                native_test: None,
                from_local: Some(&source),
                fetch: ImportFetch::RowsApi,
                python: Some("python3"),
                cache_root: Some(&cache),
            },
            &io,
        )
        .unwrap();
        assert_eq!(io.hf_calls.get(), 0);
        assert_eq!(io.http_calls.get(), 0);
        let after = list_tree(&source);
        let after_names: Vec<_> = after.iter().map(|(name, _, _)| name.clone()).collect();
        assert_eq!(
            after_names, before_names,
            "a file was created in --from-local"
        );
        assert_eq!(after, before, "source bytes or mode changed");
        assert!(
            after_names.iter().all(|name| {
                !name.contains(".lock") && !name.contains(".cache") && !name.contains(".partial")
            }),
            "{after_names:?}"
        );
        let native = cache.join("ag_news").join("native");
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(native.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["complete"], true);
        assert_eq!(manifest["train_rows"], 120_000);
        assert_eq!(manifest["test_rows"], 7_600);
        assert_eq!(manifest["hf_id"], "fancyzhx/ag_news");
        assert!(manifest["source_dir"].as_str().unwrap().contains("nas"));
        let parquet = manifest["parquet"].as_array().unwrap();
        assert!(parquet.len() >= 2);
        assert!(parquet
            .iter()
            .all(|row| row["sha256"].as_str().unwrap().len() == 64));
        assert!(native.join("train.jsonl").is_file());
        assert!(!native.join("train.jsonl.partial").exists());
        let train_n = count_complete_lines(&native.join("train.jsonl")).unwrap();
        assert_eq!(train_n, 120_000);
        assert!(!source.join(".lock").exists());
        assert!(!native.join(".lock").exists(), "lock is dropped after the import");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_pyarrow_refuses_with_a_pip_hint() {
        let root = std::env::temp_dir().join(format!(
            "import-pyarrow-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let source = snapshot_dir(&root, "nas", b"t", b"e");
        let out = root.join("sampled");
        let cache = root.join("cache");
        let io = FakeIo {
            hf_calls: std::cell::Cell::new(0),
            http_calls: std::cell::Cell::new(0),
            pyarrow_missing: true,
            touch_source: false,
        };
        let err = classify_import_with(
            &ImportRequest {
                dataset: "ag_news",
                train_size: "4",
                heldout_size: "4",
                seed: 42,
                out: &out,
                force: false,
                native_train: None,
                native_test: None,
                from_local: Some(&source),
                fetch: ImportFetch::Bulk,
                python: Some("/opt/llamafactory/bin/python"),
                cache_root: Some(&cache),
            },
            &io,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("pyarrow"), "{err}");
        assert!(err.contains("pip install pyarrow"), "{err}");
        assert!(err.contains("/opt/llamafactory/bin/python"), "{err}");
        assert_eq!(
            python_failure("python3", 2, "pyarrow-missing"),
            pyarrow_refuse("python3")
        );
        assert!(!cache
            .join("ag_news")
            .join("native")
            .join("train.jsonl")
            .exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn source_fingerprint_changes_when_the_snapshot_changes() {
        let root = std::env::temp_dir().join(format!(
            "import-fp-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let first = snapshot_dir(&root, "a", b"one", b"test");
        let second = snapshot_dir(&root, "b", b"two", b"test");
        let token_a = dataset_source_token(Some(&first), ImportFetch::Bulk, "train", "test");
        let again = dataset_source_token(Some(&first), ImportFetch::Bulk, "train", "test");
        assert_eq!(token_a, again, "unchanged snapshot reuses the hashed token");
        let token_b = dataset_source_token(Some(&second), ImportFetch::Bulk, "train", "test");
        assert_ne!(token_a, token_b);
        assert!(token_a.contains("sha256="), "{token_a}");
        let fp_a = import_fingerprint("fancyzhx/ag_news", "all", "all", 42, "t", "h", &token_a);
        let fp_b = import_fingerprint("fancyzhx/ag_news", "all", "all", 42, "t", "h", &token_b);
        assert_ne!(fp_a, fp_b);
        fs::write(
            first.join("data").join("train-00000-of-00001.parquet"),
            b"changed",
        )
        .unwrap();
        let token_c = dataset_source_token(Some(&first), ImportFetch::Bulk, "train", "test");
        assert_ne!(token_a, token_c);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn from_local_rereads_when_verified_cache_has_no_source_fp() {
        let root = std::env::temp_dir().join(format!(
            "import-migrate-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let source = snapshot_dir(&root, "nas", b"train-bytes", b"test-bytes");
        let cache = root.join("cache");
        let native = cache.join("ag_news").join("native");
        fs::create_dir_all(&native).unwrap();
        let preset = preset_by_name("ag_news").unwrap();
        let stale = "{\"index\":0,\"text\":\"stale\",\"label\":0}\n";
        fs::write(
            native.join("train.jsonl"),
            stale.repeat(preset.official_train as usize),
        )
        .unwrap();
        fs::write(
            native.join("test.jsonl"),
            stale.repeat(preset.official_test as usize),
        )
        .unwrap();
        fs::write(
            native.join("manifest.json"),
            format!(
                "{{\"hf_id\":{},\"complete\":true,\"train_rows\":{},\"test_rows\":{}}}\n",
                serde_json::to_string(preset.hf_id).unwrap(),
                preset.official_train,
                preset.official_test
            ),
        )
        .unwrap();
        assert!(cache_verified(preset, &native).unwrap());
        let out = root.join("sampled");
        let io = FakeIo {
            hf_calls: std::cell::Cell::new(0),
            http_calls: std::cell::Cell::new(0),
            pyarrow_missing: false,
            touch_source: false,
        };
        classify_import_with(
            &ImportRequest {
                dataset: "ag_news",
                train_size: "4",
                heldout_size: "4",
                seed: 42,
                out: &out,
                force: false,
                native_train: None,
                native_test: None,
                from_local: Some(&source),
                fetch: ImportFetch::RowsApi,
                python: None,
                cache_root: Some(&cache),
            },
            &io,
        )
        .unwrap();
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(native.join("manifest.json")).unwrap())
                .unwrap();
        assert!(
            manifest["source_fp"].as_str().unwrap().contains("sha256="),
            "{manifest}"
        );
        let train = fs::read_to_string(native.join("train.jsonl")).unwrap();
        assert!(train.contains("\"text\":\"row-0\""), "snapshot was not read");
        assert!(!train.contains("stale"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn directory_symlink_under_from_local_is_refused() {
        let root = std::env::temp_dir().join(format!(
            "import-symlink-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let source = snapshot_dir(&root, "nas", b"t", b"e");
        let outside = root.join("outside");
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, source.join("escape")).unwrap();
        let err = split_parquet_files(&source, "train").unwrap_err().to_string();
        assert!(err.contains("directory symlink"), "{err}");
        let cycle = root.join("cycle");
        fs::create_dir_all(&cycle).unwrap();
        fs::write(cycle.join("train-0.parquet"), b"t").unwrap();
        std::os::unix::fs::symlink(&cycle, cycle.join("again")).unwrap();
        let err = split_parquet_files(&cycle, "train").unwrap_err().to_string();
        assert!(err.contains("directory symlink"), "{err}");
        let bool_at = PARQUET_SCRIPT.find("isinstance(label, bool)").unwrap();
        let integral_at = PARQUET_SCRIPT.find("numbers.Integral").unwrap();
        assert!(bool_at < integral_at);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rows_api_flag_uses_http_and_not_hf_or_python() {
        let root = std::env::temp_dir().join(format!(
            "import-rows-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let out = root.join("sampled");
        let cache = root.join("cache");
        let io = FakeIo {
            hf_calls: std::cell::Cell::new(0),
            http_calls: std::cell::Cell::new(0),
            pyarrow_missing: false,
            touch_source: false,
        };
        let err = classify_import_with(
            &ImportRequest {
                dataset: "ag_news",
                train_size: "4",
                heldout_size: "4",
                seed: 42,
                out: &out,
                force: false,
                native_train: None,
                native_test: None,
                from_local: None,
                fetch: ImportFetch::RowsApi,
                python: None,
                cache_root: Some(&cache),
            },
            &io,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("official count"), "{err}");
        assert_eq!(io.http_calls.get(), 1);
        assert_eq!(io.hf_calls.get(), 0);
        assert_eq!(
            dataset_source_token(None, ImportFetch::RowsApi, "train", "test"),
            "rows-api"
        );
        assert_eq!(
            dataset_source_token(None, ImportFetch::Bulk, "train", "test"),
            "hf-download"
        );
        assert!(hf_auth_header(Some("secret")).unwrap() == "Bearer secret");
        assert!(hf_auth_header(Some("  ")).is_none());
        assert!(hf_auth_header(None).is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn bulk_fetch_runs_hf_download_and_not_the_rows_api() {
        let root = std::env::temp_dir().join(format!(
            "import-bulk-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let out = root.join("sampled");
        let cache = root.join("cache");
        let io = FakeIo {
            hf_calls: std::cell::Cell::new(0),
            http_calls: std::cell::Cell::new(0),
            pyarrow_missing: false,
            touch_source: false,
        };
        classify_import_with(
            &ImportRequest {
                dataset: "ag_news",
                train_size: "4",
                heldout_size: "4",
                seed: 42,
                out: &out,
                force: false,
                native_train: None,
                native_test: None,
                from_local: None,
                fetch: ImportFetch::Bulk,
                python: None,
                cache_root: Some(&cache),
            },
            &io,
        )
        .unwrap();
        assert_eq!(io.hf_calls.get(), 1);
        assert_eq!(io.http_calls.get(), 0);
        let snapshot = cache.join("ag_news").join("hf-dataset");
        assert!(snapshot
            .join("data")
            .join("train-00000-of-00001.parquet")
            .is_file());
        assert!(snapshot.starts_with(&cache));
        assert!(cache
            .join("ag_news")
            .join("native")
            .join("manifest.json")
            .is_file());
        let bare = ImportRequest {
            dataset: "ag_news",
            train_size: "all",
            heldout_size: "all",
            seed: 42,
            out: &out,
            force: false,
            native_train: None,
            native_test: None,
            from_local: None,
            fetch: ImportFetch::Bulk,
            python: None,
            cache_root: None,
        };
        assert_eq!(
            hf_snapshot_dir(&bare, "ag_news"),
            PathBuf::from(".cell/classify-import/ag_news/hf-dataset")
        );
        assert_eq!(
            native_dir(&bare, "ag_news"),
            PathBuf::from(".cell/classify-import/ag_news/native")
        );
        let _ = fs::remove_dir_all(&root);
    }
}
