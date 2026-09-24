//! Download a public Hugging Face classification set and write tev1 records.
//!
//! The default fetch is `hf download --repo-type dataset` into a cache directory,
//! then a pyarrow read of `train-*.parquet` and `test-*.parquet`. `--from-local`
//! reads a snapshot that is already on disk and does not write into that directory.
//! `--fetch rows-api` is the explicit datasets-server fallback. Tests inject the
//! Python and `hf` runners and never spawn a process or call the network.
//!
//! Option order for `fixed_classes` presets (ag_news, devign, rust_idiom) is the
//! class table order. Letters A, B, C, … follow that order and are not shuffled.
//! The answer letter is the class letter. `sample_distractors` (banking77, later)
//! shuffles the chosen options with the import seed, and the answer letter follows
//! that shuffle. `rust_idiom` reads CommitPackFT Rust commits (`old_contents` /
//! `new_contents`) through this same path and holds out whole commits with a fixed seed.

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

/// How a snapshot becomes native rows. Parquet presets stay one labeled row per record.
#[derive(Clone, Copy, Debug)]
enum SourceShape {
    LabeledColumns,
    /// One commit becomes NeedsFix (`old_field`) and Idiomatic (`new_field`).
    /// Empty sides are dropped. There is no official test split.
    CommitPair {
        old_field: &'static str,
        new_field: &'static str,
        lang_field: &'static str,
        lang_value: &'static str,
        /// `hf download --include` so the Rust subset is fetched, not the full pack.
        include: &'static str,
        source_commits: u64,
        usable_pairs: u64,
        holdout_seed: u64,
    },
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
    /// datasets-server config. `default` for single-config sets. `rust` for CommitPackFT.
    rows_config: &'static str,
    shape: SourceShape,
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

const DEVIGN_CLASSES: &[ClassSpec] = &[
    ClassSpec {
        key: "secure",
        description: "Secure",
    },
    ClassSpec {
        key: "insecure",
        description: "Insecure",
    },
];

const RUST_IDIOM_CLASSES: &[ClassSpec] = &[
    ClassSpec {
        key: "needs_fix",
        description: "NeedsFix",
    },
    ClassSpec {
        key: "idiomatic",
        description: "Idiomatic",
    },
];

/// CommitPackFT rust file rows. The card table lists this sample count.
const RUST_SOURCE_COMMITS: u64 = 2_996;
/// Both sides non-empty and different. Counted from `data/rust/data.jsonl`.
const RUST_USABLE_PAIRS: u64 = 2_340;
const RUST_HOLDOUT_SEED: u64 = 42;
/// Shape of the CommitPair expand and drop rules. A change invalidates native caches
/// even when the expanded row counts stay the same.
const COMMIT_PAIR_TRANSFORM: &str =
    "drop-empty-or-identical;expand-old0-new1;holdout-one-fifth-whole-commit";

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

/// Downloadable presets plus the two later rows. `ag_news`, `devign`, and `rust_idiom` download.
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
        rows_config: "default",
        shape: SourceShape::LabeledColumns,
        fetch: Fetch::RowsApi,
    },
    DatasetPreset {
        alias: "devign",
        hf_id: "google/code_x_glue_cc_defect_detection",
        slug: "devign",
        train_split: "train",
        test_split: "test",
        official_train: 21_854,
        official_test: 2_732,
        question: "Is this function secure or insecure code?",
        text_field: "func",
        label_field: "target",
        map: LabelMap::FixedClasses(DEVIGN_CLASSES),
        license_note: "Devign / CodeXGLUE defect detection (google/code_x_glue_cc_defect_detection, C-UDA) is for local training only. Do not redistribute. Credit Devign and CodeXGLUE.",
        rows_config: "default",
        shape: SourceShape::LabeledColumns,
        fetch: Fetch::RowsApi,
    },
    DatasetPreset {
        alias: "rust_idiom",
        hf_id: "bigcode/commitpackft",
        slug: "rustidiom",
        train_split: "train",
        test_split: "test",
        // Expanded rows after the seeded holdout, not the 2,996 source commits.
        // Train commits 1,872 × 2 = 3,744. Held-out commits 468 × 2 = 936.
        official_train: 3_744,
        official_test: 936,
        question: "Does this Rust snippet need a fix, or is it the idiomatic version?",
        text_field: "old_contents",
        label_field: "label",
        map: LabelMap::FixedClasses(RUST_IDIOM_CLASSES),
        license_note: "CommitPackFT rust (bigcode/commitpackft) mixes a dataset-card MIT name with per-sample repository licenses, including unknown and copyleft. This output is for local training only. Do not redistribute.",
        rows_config: "rust",
        shape: SourceShape::CommitPair {
            old_field: "old_contents",
            new_field: "new_contents",
            lang_field: "lang",
            lang_value: "Rust",
            include: "data/rust/*",
            source_commits: RUST_SOURCE_COMMITS,
            usable_pairs: RUST_USABLE_PAIRS,
            holdout_seed: RUST_HOLDOUT_SEED,
        },
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
        rows_config: "default",
        shape: SourceShape::LabeledColumns,
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
        rows_config: "default",
        shape: SourceShape::LabeledColumns,
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
        anyhow::anyhow!(
            "refuse:classify-import: size must be a positive integer or all, found {raw}"
        )
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
                "refuse:classify-import: unknown dataset {name}. This slice downloads ag_news (fancyzhx/ag_news), devign (google/code_x_glue_cc_defect_detection), and rust_idiom (bigcode/commitpackft)."
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

fn fingerprint_body(
    dataset_id: &str,
    train_size: &str,
    heldout_size: &str,
    seed: u64,
    train_hash: &str,
    held_hash: &str,
) -> String {
    format!(
        "classify-import\n{dataset_id}\ntrain={train_size}\nheldout={heldout_size}\nseed={seed}\n{train_hash}\n{held_hash}"
    )
}

fn hash_text(text: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Drop `mtime=` so a retouched parquet file does not change the journey key.
pub fn stable_source(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split(" mtime=").next().unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Pre-#205 digest. No `source=` suffix.
pub fn legacy_import_fingerprint(
    dataset_id: &str,
    train_size: &str,
    heldout_size: &str,
    seed: u64,
    train_hash: &str,
    held_hash: &str,
) -> String {
    hash_text(&fingerprint_body(
        dataset_id,
        train_size,
        heldout_size,
        seed,
        train_hash,
        held_hash,
    ))
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
    let body = fingerprint_body(
        dataset_id,
        train_size,
        heldout_size,
        seed,
        train_hash,
        held_hash,
    );
    let stable = stable_source(source);
    // Rows-api is the source those older manifests already describe.
    if stable.is_empty() || stable == "rows-api" {
        return hash_text(&body);
    }
    hash_text(&format!("{body}\nsource={stable}"))
}

/// Journey prepare key. When the native cache was built from this same source,
/// keep a stored pre-`source=` digest so an unchanged snapshot does not redo prepare.
pub fn journey_prepare_fingerprint(
    dataset_id: &str,
    train_size: &str,
    heldout_size: &str,
    seed: u64,
    train_hash: &str,
    held_hash: &str,
    source: &str,
    stored_inputs: Option<&str>,
    cached_source_fp: Option<&str>,
) -> String {
    let legacy = legacy_import_fingerprint(
        dataset_id,
        train_size,
        heldout_size,
        seed,
        train_hash,
        held_hash,
    );
    let stable = import_fingerprint(
        dataset_id,
        train_size,
        heldout_size,
        seed,
        train_hash,
        held_hash,
        source,
    );
    let exact = {
        let body = fingerprint_body(
            dataset_id,
            train_size,
            heldout_size,
            seed,
            train_hash,
            held_hash,
        );
        if source.is_empty() || source == "rows-api" {
            hash_text(&body)
        } else {
            hash_text(&format!("{body}\nsource={source}"))
        }
    };
    let bound = match cached_source_fp {
        Some(fp) => stable_source(fp) == stable_source(source),
        None => source == "rows-api" || source.is_empty(),
    };
    if bound {
        if let Some(stored) = stored_inputs {
            if stored == legacy || stored == stable || stored == exact {
                return stored.to_string();
            }
        }
        return legacy;
    }
    stable
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
    assert_known_labels(preset, train_rows, "train")?;
    assert_known_labels(preset, test_rows, "test")?;
    let train_idx = balanced_indices(train_rows, train_size, seed, "train")?;
    let held_idx = balanced_indices(
        test_rows,
        heldout_size,
        seed.wrapping_add(0xA5A5_5A5A),
        "test",
    )?;
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

fn class_table(preset: &DatasetPreset) -> &'static [ClassSpec] {
    match preset.map {
        LabelMap::FixedClasses(classes)
        | LabelMap::SampleDistractors { classes, .. }
        | LabelMap::ThreeWay { classes, .. } => classes,
    }
}

fn assert_known_labels(preset: &DatasetPreset, rows: &[NativeRow], side: &str) -> Result<()> {
    let classes = class_table(preset);
    if classes.is_empty() {
        return Ok(());
    }
    let last = classes.len() - 1;
    for row in rows {
        if classes.get(row.label).is_none() {
            bail!(
                "refuse:classify-import: {side} row {} label {} is outside the {} class table (0..={last})",
                row.index,
                row.label,
                preset.alias
            );
        }
    }
    Ok(())
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

fn heldout_kind(preset: &DatasetPreset) -> &'static str {
    match preset.shape {
        SourceShape::CommitPair { .. } => "seeded-commit-holdout",
        SourceShape::LabeledColumns => "official-test",
    }
}

fn source_commits(preset: &DatasetPreset) -> Option<u64> {
    match preset.shape {
        SourceShape::CommitPair { source_commits, .. } => Some(source_commits),
        SourceShape::LabeledColumns => None,
    }
}

fn usable_pairs(preset: &DatasetPreset) -> Option<u64> {
    match preset.shape {
        SourceShape::CommitPair { usable_pairs, .. } => Some(usable_pairs),
        SourceShape::LabeledColumns => None,
    }
}

fn heldout_note(preset: &DatasetPreset) -> String {
    match preset.shape {
        SourceShape::CommitPair {
            source_commits,
            usable_pairs,
            holdout_seed,
            ..
        } => {
            let held_commits = usable_pairs / 5;
            let train_commits = usable_pairs - held_commits;
            format!(
                "CommitPackFT rust has no test split. Source file rows: {source_commits}. Usable before/after pairs (both sides non-empty and different): {usable_pairs}. Held-out is a deterministic SplitMix holdout of whole commits at seed {holdout_seed}, one fifth of usable commits ({held_commits}). Both sides of a pair stay in the same split. Expanded rows: train {} ({train_commits} commits), held-out {} ({held_commits} commits).",
                preset.official_train, preset.official_test
            )
        }
        SourceShape::LabeledColumns => format!(
            "Held-out is the official {} split ({} rows).",
            preset.test_split, preset.official_test
        ),
    }
}

fn option_order_note(preset: &DatasetPreset) -> String {
    match preset.map {
        LabelMap::FixedClasses(classes) | LabelMap::ThreeWay { classes, .. } => {
            let letters: Vec<char> = LABELS.chars().take(classes.len()).collect();
            let pairs: Vec<String> = classes
                .iter()
                .enumerate()
                .map(|(i, class)| format!("{}={}", letters[i], class.description))
                .collect();
            format!(
                "{} options stay in class-table order ({}) and are not shuffled. The answer letter is that class letter.",
                preset.alias,
                pairs.join(", ")
            )
        }
        LabelMap::SampleDistractors { .. } => {
            "sample_distractors shuffles the chosen options with --seed and the answer letter follows the shuffle.".into()
        }
    }
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
    if distractors == 0
        || distractors + 1 > classes.len()
        || distractors + 1 > LABELS.chars().count()
    {
        bail!(
            "refuse:classify-import: distractor count {distractors} does not fit the class table"
        );
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
            "refuse:classify-import: {} is a catalog row for a later slice. This slice downloads ag_news, devign, and rust_idiom.",
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
        "option_order_note": option_order_note(preset),
        "heldout_split": preset.test_split,
        "heldout_kind": heldout_kind(preset),
        "heldout_note": heldout_note(preset),
        "official_train": preset.official_train,
        "official_test": preset.official_test,
        "source_commits": source_commits(preset),
        "usable_pairs": usable_pairs(preset),
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
    /// Set for CommitPackFT. The script writes `{index, old, new, lang}` lines.
    pair_old: Option<&'a str>,
    pair_new: Option<&'a str>,
    lang_field: Option<&'a str>,
    /// Source commit rows the injected reader should emit. Live Python ignores this and reads the file.
    commit_rows: u64,
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
            let snapshot = hf_snapshot_dir(req, preset.alias);
            if !req.force && bulk_cache_reusable(preset, &native, &snapshot)? {
                let train_final = native.join(format!("{}.jsonl", preset.train_split));
                let test_final = native.join(format!("{}.jsonl", preset.test_split));
                return Ok((read_native(&train_final)?, read_native(&test_final)?));
            }
            let snapshot = hf_snapshot_dir(req, preset.alias);
            fs::create_dir_all(&snapshot)?;
            let bin = crate::classify_journey::hf_bin_name().unwrap_or("hf");
            io.hf_download(&hf_dataset_argv(bin, preset, &snapshot))?;
            acquire_local_parquet(preset, &native, &snapshot, true, req.python, io)
        }
    }
}

fn hf_dataset_argv(bin: &str, preset: &DatasetPreset, local_dir: &Path) -> Vec<String> {
    let mut argv = vec![
        bin.to_string(),
        "download".into(),
        preset.hf_id.to_string(),
        "--repo-type".into(),
        "dataset".into(),
        "--local-dir".into(),
        local_dir.display().to_string(),
    ];
    if let SourceShape::CommitPair { include, .. } = preset.shape {
        argv.push("--include".into());
        argv.push(include.to_string());
    }
    argv
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
    if let SourceShape::CommitPair { .. } = preset.shape {
        return acquire_commit_pairs(preset, native, source_dir, force, python, io);
    }
    let source_dir = canonical_snapshot(source_dir)?;
    let source = inspect_parquet_source(&source_dir, preset.train_split, preset.test_split)?;
    let train_final = native.join(format!("{}.jsonl", preset.train_split));
    let test_final = native.join(format!("{}.jsonl", preset.test_split));
    if !force && cache_verified(preset, native)? {
        if manifest_source_fp(native)?.as_deref() == Some(source.token.as_str()) {
            return Ok((read_native(&train_final)?, read_native(&test_final)?));
        }
    }
    let (premise_field, hypothesis_field) = nli_fields(preset);
    let train_files = split_parquet_files(&source_dir, preset.train_split)?;
    let test_files = split_parquet_files(&source_dir, preset.test_split)?;
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
        pair_old: None,
        pair_new: None,
        lang_field: None,
        commit_rows: 0,
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

/// A verified native cache is reusable for Bulk only when its `source_fp` is this snapshot.
/// A rows-api cache has no `source_fp` and is not reused.
fn bulk_cache_reusable(preset: &DatasetPreset, native: &Path, snapshot: &Path) -> Result<bool> {
    if !cache_verified(preset, native)? {
        return Ok(false);
    }
    let Some(stored) = manifest_source_fp(native)? else {
        return Ok(false);
    };
    let Ok(source) = inspect_preset_source(preset, snapshot) else {
        return Ok(false);
    };
    Ok(stored == source.token)
}

pub fn cached_native_source_fp(alias: &str) -> Option<String> {
    manifest_source_fp(&native_cache_dir(alias)).ok().flatten()
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

pub fn preset_source_token(
    preset: &DatasetPreset,
    from_local: Option<&Path>,
    fetch: ImportFetch,
) -> String {
    if let Some(dir) = from_local {
        if let SourceShape::CommitPair { .. } = preset.shape {
            return inspect_commit_jsonl(dir)
                .map(|source| source.token)
                .unwrap_or_else(|_| format!("local-unreadable:{}", dir.display()));
        }
    }
    dataset_source_token(from_local, fetch, preset.train_split, preset.test_split)
}

fn inspect_preset_source(preset: &DatasetPreset, dir: &Path) -> Result<ParquetSource> {
    match preset.shape {
        SourceShape::CommitPair { .. } => inspect_commit_jsonl(dir),
        SourceShape::LabeledColumns => {
            inspect_parquet_source(dir, preset.train_split, preset.test_split)
        }
    }
}

fn canonical_snapshot(dir: &Path) -> Result<PathBuf> {
    if !dir.is_dir() {
        bail!(
            "refuse:classify-import: --from-local {} is not a directory",
            dir.display()
        );
    }
    Ok(fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf()))
}

fn file_mtime_secs(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn file_mtime_nanos(meta: &fs::Metadata) -> u128 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0)
}

fn inspect_parquet_source(
    dir: &Path,
    train_split: &str,
    test_split: &str,
) -> Result<ParquetSource> {
    let canon = fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let mut paths = split_parquet_files(&canon, train_split)?;
    paths.extend(split_parquet_files(&canon, test_split)?);
    paths.sort();
    paths.dedup();
    let cache_key = format!("{}\n{train_split}\n{test_split}", canon.display());
    let mut cheap_lines = vec![format!("dir={}", canon.display())];
    let mut stats = Vec::new();
    for path in &paths {
        let meta = fs::metadata(path).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-import: cannot stat {}: {err}",
                path.display()
            )
        })?;
        let mtime = file_mtime_secs(&meta);
        let mtime_ns = file_mtime_nanos(&meta);
        let rel = path
            .strip_prefix(&canon)
            .unwrap_or(path)
            .display()
            .to_string();
        cheap_lines.push(format!("{rel} bytes={} mtime_ns={mtime_ns}", meta.len()));
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

fn source_cache() -> &'static std::sync::Mutex<std::collections::HashMap<String, RememberedSource>>
{
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
            let followed = fs::metadata(&path).map_err(|err| {
                anyhow::anyhow!(
                    "refuse:classify-import: cannot stat {}: {err}",
                    path.display()
                )
            })?;
            if followed.is_dir() {
                bail!(
                    "refuse:classify-import: directory symlink {} is refused",
                    path.display()
                );
            }
            if followed.is_file() && is_split_parquet(&path, split) {
                out.push(path);
                continue;
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
spec = json.loads(sys.stdin.read())
pair_old = spec.get("pair_old") or ""
if pair_old:
    pair_new = spec["pair_new"]
    lang_field = spec["lang_field"]
    index = 0
    with open(spec["out"], "w", encoding="utf-8") as out:
        for path in spec["files"]:
            with open(path, "r", encoding="utf-8") as handle:
                for line in handle:
                    line = line.strip()
                    if not line:
                        continue
                    row = json.loads(line)
                    old = row.get(pair_old)
                    new = row.get(pair_new)
                    lang = row.get(lang_field)
                    out.write(json.dumps({
                        "index": index,
                        "old": "" if old is None else str(old),
                        "new": "" if new is None else str(new),
                        "lang": "" if lang is None else str(lang),
                    }, ensure_ascii=False) + "\n")
                    index += 1
    sys.exit(0)
try:
    import pyarrow.parquet as pq
except Exception:
    sys.stderr.write("pyarrow-missing\n")
    sys.exit(2)
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
            # bool is a subclass of int. Map False/True to 0/1 before the int check.
            if isinstance(label, bool):
                label = int(label)
            elif isinstance(label, numbers.Integral):
                label = int(label)
            else:
                sys.stderr.write("label is not a bool or int\n")
                sys.exit(3)
            if label < 0:
                sys.stderr.write("label is negative\n")
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
        "pair_old": job.pair_old,
        "pair_new": job.pair_new,
        "lang_field": job.lang_field,
        "commit_rows": job.commit_rows,
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

#[derive(Clone)]
struct CommitLine {
    index: u64,
    old: String,
    new: String,
    lang: String,
}

fn acquire_commit_pairs(
    preset: &DatasetPreset,
    native: &Path,
    source_dir: &Path,
    force: bool,
    python: Option<&str>,
    io: &dyn ImportIo,
) -> Result<(Vec<NativeRow>, Vec<NativeRow>)> {
    let source_dir = canonical_snapshot(source_dir)?;
    let source = inspect_commit_jsonl(&source_dir)?;
    let train_final = native.join(format!("{}.jsonl", preset.train_split));
    let test_final = native.join(format!("{}.jsonl", preset.test_split));
    if !force && cache_verified(preset, native)? {
        if manifest_source_fp(native)?.as_deref() == Some(source.token.as_str()) {
            return Ok((read_native(&train_final)?, read_native(&test_final)?));
        }
    }
    let files = commit_jsonl_files(&source_dir)?;
    let (
        old_field,
        new_field,
        lang_field,
        lang_value,
        source_commits,
    ) = commit_pair_fields(preset)?;
    let partial = native.join("commits.jsonl.partial");
    if let Some(parent) = partial.parent() {
        fs::create_dir_all(parent)?;
    }
    let _ = fs::remove_file(&partial);
    io.read_parquet(&ParquetJob {
        python,
        split: "commits",
        files: &files,
        text_field: preset.text_field,
        label_field: preset.label_field,
        premise_field: None,
        hypothesis_field: None,
        pair_old: Some(old_field),
        pair_new: Some(new_field),
        lang_field: Some(lang_field),
        commit_rows: source_commits,
        out_partial: &partial,
    })?;
    let (train_rows, test_rows) = commits_to_splits(preset, &read_commit_lines(&partial)?, lang_value)?;
    let _ = fs::remove_file(&partial);
    write_native_rows(&partial_path(&train_final), &train_rows)?;
    write_native_rows(&partial_path(&test_final), &test_rows)?;
    publish_native(&train_final)?;
    publish_native(&test_final)?;
    let manifest = commit_pair_manifest(
        preset,
        json!({
            "hf_id": preset.hf_id,
            "source": "local-jsonl",
            "source_dir": source.dir_display,
            "source_fp": source.token,
            "jsonl": source.files,
            "train_rows": train_rows.len(),
            "test_rows": test_rows.len(),
            "complete": true,
        }),
    )?;
    fs::write(
        native.join("manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok((train_rows, test_rows))
}

fn commit_pair_fields(
    preset: &DatasetPreset,
) -> Result<(&'static str, &'static str, &'static str, &'static str, u64)> {
    match preset.shape {
        SourceShape::CommitPair {
            old_field,
            new_field,
            lang_field,
            lang_value,
            source_commits,
            ..
        } => Ok((old_field, new_field, lang_field, lang_value, source_commits)),
        SourceShape::LabeledColumns => {
            bail!("refuse:classify-import: {} is not a commit-pair preset", preset.alias)
        }
    }
}

fn commits_to_splits(
    preset: &DatasetPreset,
    lines: &[CommitLine],
    lang_value: &str,
) -> Result<(Vec<NativeRow>, Vec<NativeRow>)> {
    let (source_commits, usable_pairs, holdout_seed) = match preset.shape {
        SourceShape::CommitPair {
            source_commits,
            usable_pairs,
            holdout_seed,
            ..
        } => (source_commits, usable_pairs, holdout_seed),
        SourceShape::LabeledColumns => {
            bail!("refuse:classify-import: {} is not a commit-pair preset", preset.alias)
        }
    };
    if lines.len() as u64 != source_commits {
        bail!(
            "refuse:classify-import: {} source file has {} commits but the official count is {source_commits}",
            preset.alias,
            lines.len()
        );
    }
    let mut usable = Vec::new();
    for line in lines {
        if line.lang != lang_value {
            bail!(
                "refuse:classify-import: {} commit {} lang {} is not {lang_value}",
                preset.alias,
                line.index,
                line.lang
            );
        }
        if line.old.trim().is_empty() || line.new.trim().is_empty() || line.old == line.new {
            continue;
        }
        usable.push(line);
    }
    if usable.len() as u64 != usable_pairs {
        bail!(
            "refuse:classify-import: {} has {} usable before/after pairs but the preset count is {usable_pairs}",
            preset.alias,
            usable.len()
        );
    }
    let held_commits = usable.len() / 5;
    let (mut train_at, mut test_at) = split_indices(usable.len(), holdout_seed, held_commits);
    train_at.sort_unstable();
    test_at.sort_unstable();
    let train_rows = expand_commits(&usable, &train_at);
    let test_rows = expand_commits(&usable, &test_at);
    verify_split_count(
        preset.train_split,
        train_rows.len() as u64,
        train_rows.len() as u64,
        preset.official_train,
    )?;
    verify_split_count(
        preset.test_split,
        test_rows.len() as u64,
        test_rows.len() as u64,
        preset.official_test,
    )?;
    eprintln!(
        "classify-import: {} usable pairs, train {} rows, held-out {} rows (seeded commit holdout)",
        usable.len(),
        train_rows.len(),
        test_rows.len()
    );
    Ok((train_rows, test_rows))
}

fn expand_commits(usable: &[&CommitLine], indices: &[usize]) -> Vec<NativeRow> {
    let mut rows = Vec::with_capacity(indices.len() * 2);
    for &at in indices {
        let line = usable[at];
        rows.push(NativeRow {
            index: line.index.saturating_mul(2),
            text: line.old.clone(),
            premise: None,
            hypothesis: None,
            label: 0,
        });
        rows.push(NativeRow {
            index: line.index.saturating_mul(2).saturating_add(1),
            text: line.new.clone(),
            premise: None,
            hypothesis: None,
            label: 1,
        });
    }
    rows
}

fn read_commit_lines(path: &Path) -> Result<Vec<CommitLine>> {
    let file = File::open(path).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-import: cannot read {}: {err}",
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
                "refuse:classify-import: {} line {} is not JSON: {err}",
                path.display(),
                line_no + 1
            )
        })?;
        rows.push(commit_from_value(line_no as u64, &value, "old", "new", "lang")?);
    }
    Ok(rows)
}

fn commit_from_value(
    fallback_index: u64,
    value: &Value,
    old_field: &str,
    new_field: &str,
    lang_field: &str,
) -> Result<CommitLine> {
    let index = value
        .get("index")
        .and_then(Value::as_u64)
        .unwrap_or(fallback_index);
    let text_of = |field: &str| -> String {
        match value.get(field) {
            Some(Value::String(text)) => text.clone(),
            Some(other) if !other.is_null() => other.to_string(),
            _ => String::new(),
        }
    };
    Ok(CommitLine {
        index,
        old: text_of(old_field),
        new: text_of(new_field),
        lang: text_of(lang_field),
    })
}

fn write_native_rows(path: &Path, rows: &[NativeRow]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    for row in rows {
        writeln!(file, "{}", serde_json::to_string(&native_json(row))?)?;
    }
    Ok(())
}

fn inspect_commit_jsonl(dir: &Path) -> Result<ParquetSource> {
    let canon = fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let paths = commit_jsonl_files(&canon)?;
    let cache_key = format!("jsonl\n{}", canon.display());
    let mut cheap_lines = vec![format!("dir={}", canon.display())];
    let mut stats = Vec::new();
    for path in &paths {
        let meta = fs::metadata(path).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-import: cannot stat {}: {err}",
                path.display()
            )
        })?;
        let mtime = file_mtime_secs(&meta);
        let mtime_ns = file_mtime_nanos(&meta);
        let rel = path
            .strip_prefix(&canon)
            .unwrap_or(path)
            .display()
            .to_string();
        cheap_lines.push(format!("{rel} bytes={} mtime_ns={mtime_ns}", meta.len()));
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

fn commit_jsonl_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    collect_commit_jsonl(dir, &mut seen, &mut found)?;
    let rust: Vec<PathBuf> = found
        .iter()
        .filter(|path| {
            path.components().any(|part| match part {
                std::path::Component::Normal(name) => name == "rust",
                _ => false,
            })
        })
        .cloned()
        .collect();
    let chosen = if !rust.is_empty() {
        rust
    } else if found.len() == 1 {
        found
    } else if found.is_empty() {
        bail!(
            "refuse:classify-import: no data/rust/data.jsonl under {}",
            dir.display()
        );
    } else {
        bail!(
            "refuse:classify-import: multiple data.jsonl files under {} and none is the rust subset",
            dir.display()
        );
    };
    let mut chosen = chosen;
    chosen.sort();
    Ok(chosen)
}

fn collect_commit_jsonl(
    dir: &Path,
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
            let followed = fs::metadata(&path).map_err(|err| {
                anyhow::anyhow!(
                    "refuse:classify-import: cannot stat {}: {err}",
                    path.display()
                )
            })?;
            if followed.is_dir() {
                bail!(
                    "refuse:classify-import: directory symlink {} is refused",
                    path.display()
                );
            }
            if followed.is_file() && is_commit_jsonl(&path) {
                out.push(path);
                continue;
            }
            eprintln!("classify-import: skip symlink {}", path.display());
            continue;
        }
        if meta.is_dir() {
            collect_commit_jsonl(&path, seen, out)?;
        } else if meta.is_file() && is_commit_jsonl(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_commit_jsonl(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("data.jsonl")
}

fn fetch_commit_pairs(
    preset: &DatasetPreset,
    native: &Path,
    get: &mut impl FnMut(&str) -> PageGet,
    sleep: &mut impl FnMut(Duration),
) -> Result<(Vec<NativeRow>, Vec<NativeRow>)> {
    let (old_field, new_field, lang_field, lang_value, source_commits) = commit_pair_fields(preset)?;
    let mut lines = Vec::new();
    let mut offset = 0u64;
    let mut total = None;
    loop {
        if offset >= source_commits {
            break;
        }
        let url = format!(
            "{DATASETS_SERVER}/rows?dataset={}&config={}&split={}&offset={offset}&length={ROWS_PAGE}",
            urlencoding_dataset(preset.hf_id),
            preset.rows_config,
            preset.train_split
        );
        let body = get_with_retry(&url, get, sleep)?;
        let value: Value = serde_json::from_str(&body).map_err(|err| {
            anyhow::anyhow!("refuse:classify-import: datasets-server JSON failed: {err}")
        })?;
        if total.is_none() {
            total = value.get("num_rows_total").and_then(Value::as_u64);
        }
        let reported = total.ok_or_else(|| {
            anyhow::anyhow!("refuse:classify-import: commits page has no num_rows_total")
        })?;
        if reported != source_commits {
            bail!(
                "refuse:classify-import: {} rows API reports {reported} commits but the official count is {source_commits}",
                preset.alias
            );
        }
        let page = value.get("rows").and_then(Value::as_array).ok_or_else(|| {
            anyhow::anyhow!("refuse:classify-import: datasets-server page has no rows")
        })?;
        if page.is_empty() {
            bail!(
                "refuse:classify-import: {} returned an empty page at offset {offset} before {source_commits} commits",
                preset.alias
            );
        }
        for item in page {
            let row_idx = item
                .get("row_idx")
                .and_then(Value::as_u64)
                .unwrap_or(offset);
            let native_row = item.get("row").cloned().unwrap_or(Value::Null);
            let mut line = commit_from_value(row_idx, &native_row, old_field, new_field, lang_field)?;
            line.index = row_idx;
            lines.push(line);
            offset = offset.saturating_add(1);
            if offset >= source_commits {
                break;
            }
        }
        eprintln!("classify-import: commits {offset}/{source_commits}");
        if offset < source_commits {
            sleep(PAGE_POLITENESS);
        }
    }
    let (train_rows, test_rows) = commits_to_splits(preset, &lines, lang_value)?;
    let train_final = native.join(format!("{}.jsonl", preset.train_split));
    let test_final = native.join(format!("{}.jsonl", preset.test_split));
    write_native_rows(&partial_path(&train_final), &train_rows)?;
    write_native_rows(&partial_path(&test_final), &test_rows)?;
    publish_native(&train_final)?;
    publish_native(&test_final)?;
    let manifest = commit_pair_manifest(
        preset,
        json!({
            "hf_id": preset.hf_id,
            "source": "datasets-server rows API",
            "config": preset.rows_config,
            "train_rows": train_rows.len(),
            "test_rows": test_rows.len(),
            "complete": true,
        }),
    )?;
    fs::write(
        native.join("manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok((train_rows, test_rows))
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
    if let SourceShape::CommitPair { .. } = preset.shape {
        return fetch_commit_pairs(preset, native, &mut get, &mut sleep);
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
    if train_rows != Some(preset.official_train)
        || test_rows != Some(preset.official_test)
        || count_complete_lines(&train_path)? != preset.official_train
        || count_complete_lines(&test_path)? != preset.official_test
    {
        return Ok(false);
    }
    if let SourceShape::CommitPair { holdout_seed, .. } = preset.shape {
        // Missing or mismatched seed/transform must rebuild. Row counts alone are not the split.
        if value.get("holdout_seed").and_then(Value::as_u64) != Some(holdout_seed) {
            return Ok(false);
        }
        if value.get("pair_transform").and_then(Value::as_str) != Some(COMMIT_PAIR_TRANSFORM) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn commit_pair_manifest(preset: &DatasetPreset, mut manifest: Value) -> Result<Value> {
    let SourceShape::CommitPair { holdout_seed, .. } = preset.shape else {
        bail!(
            "refuse:classify-import: {} is not a commit-pair preset",
            preset.alias
        );
    };
    let obj = manifest
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-import: native manifest is not an object"))?;
    obj.insert("holdout_seed".into(), json!(holdout_seed));
    obj.insert("pair_transform".into(), json!(COMMIT_PAIR_TRANSFORM));
    Ok(manifest)
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
            "{DATASETS_SERVER}/rows?dataset={}&config={}&split={split}&offset={offset}&length={page_len}",
            urlencoding_dataset(preset.hf_id),
            preset.rows_config
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
            let row_idx = item
                .get("row_idx")
                .and_then(Value::as_u64)
                .unwrap_or(offset);
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
        bail!("refuse:classify-import: {split} has {got} rows but num_rows_total is {reported}");
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
        let retryable =
            page.transport.is_some() || page.status == 429 || (500..600).contains(&page.status);
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
    retry_after.unwrap_or(floor).min(MAX_RETRY_AFTER).max(floor)
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

fn parse_label_value(index: u64, value: Option<&Value>) -> Result<usize> {
    let Some(value) = value else {
        bail!("refuse:classify-import: row {index} has no label");
    };
    if let Some(flag) = value.as_bool() {
        return Ok(usize::from(flag));
    }
    if let Some(n) = value.as_u64() {
        return Ok(n as usize);
    }
    if let Some(n) = value.as_i64() {
        if n < 0 {
            bail!("refuse:classify-import: row {index} label {n} is negative");
        }
        return Ok(n as usize);
    }
    bail!("refuse:classify-import: row {index} label {value} is not a bool or integer")
}

fn parse_server_row(preset: &DatasetPreset, index: u64, row: &Value) -> Result<NativeRow> {
    let label = parse_label_value(index, row.get(preset.label_field))?;
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
        let line = line
            .map_err(|err| anyhow::anyhow!("refuse:classify-import: {}: {err}", path.display()))?;
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
        rows_config: "default",
        shape: SourceShape::LabeledColumns,
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
    fn devign_aliases_map_bool_and_int_and_keep_fixed_ab_order() {
        let preset = preset_by_name("devign").unwrap();
        let by_hub = preset_by_name("google/code_x_glue_cc_defect_detection").unwrap();
        assert!(std::ptr::eq(preset, by_hub));
        assert_eq!(preset.slug, "devign");
        assert_eq!(preset.text_field, "func");
        assert_eq!(preset.label_field, "target");
        assert_eq!(preset.train_split, "train");
        assert_eq!(preset.test_split, "test");
        assert_eq!(preset.official_train, 21_854);
        assert_eq!(preset.official_test, 2_732);
        assert!(preset.license_note.contains("Do not redistribute"));
        assert!(preset.license_note.contains("Devign"));
        assert!(preset.license_note.contains("CodeXGLUE"));
        assert_eq!(
            parse_server_row(
                preset,
                0,
                &json!({"func": "int ok(){return 0;}", "target": false})
            )
            .unwrap()
            .label,
            0
        );
        assert_eq!(
            parse_server_row(preset, 1, &json!({"func": "void bad(){}", "target": true}))
                .unwrap()
                .label,
            1
        );
        assert_eq!(
            parse_server_row(preset, 2, &json!({"func": "void z(){}", "target": 0}))
                .unwrap()
                .label,
            0
        );
        assert_eq!(
            parse_server_row(preset, 3, &json!({"func": "void o(){}", "target": 1}))
                .unwrap()
                .label,
            1
        );
        let ag = preset_by_name("ag_news").unwrap();
        assert_eq!(
            parse_server_row(ag, 4, &json!({"text": "new chip", "label": 3}))
                .unwrap()
                .label,
            3
        );
        let unknown = parse_server_row(preset, 5, &json!({"func": "void x(){}", "target": "vuln"}))
            .unwrap_err()
            .to_string();
        assert!(unknown.contains("not a bool or integer"), "{unknown}");
        let negative = parse_server_row(preset, 6, &json!({"func": "void x(){}", "target": -1}))
            .unwrap_err()
            .to_string();
        assert!(negative.contains("negative"), "{negative}");
        let missing = parse_server_row(preset, 7, &json!({"func": "void x(){}"}))
            .unwrap_err()
            .to_string();
        assert!(missing.contains("has no label"), "{missing}");
        let outside = sample_records(
            preset,
            &grid(0, 2, 2),
            &[native(9, 2, "void weird(){}")],
            &SplitSize::All,
            &SplitSize::All,
            42,
        )
        .unwrap_err()
        .to_string();
        assert!(
            outside.contains("outside the devign class table"),
            "{outside}"
        );
        let sampled = sample_records(
            preset,
            &grid(0, 8, 2),
            &grid(1_000, 4, 2),
            &SplitSize::Count(4),
            &SplitSize::All,
            42,
        )
        .unwrap();
        assert_eq!(sampled.train.len(), 4);
        assert_eq!(sampled.heldout.len(), 8);
        let mut counts = [0, 0];
        for row in &sampled.train {
            let opts = row["options"].as_array().unwrap();
            assert_eq!(opts.len(), 2);
            assert_eq!(opts[0]["label"], "A");
            assert_eq!(opts[0]["key"], "secure");
            assert_eq!(opts[0]["description"], "Secure");
            assert_eq!(opts[1]["label"], "B");
            assert_eq!(opts[1]["key"], "insecure");
            assert_eq!(opts[1]["description"], "Insecure");
            assert_eq!(row["question"], "Is this function secure or insecure code?");
            match row["answer"].as_str().unwrap() {
                "A" => counts[0] += 1,
                "B" => counts[1] += 1,
                other => panic!("unexpected letter {other}"),
            }
        }
        assert_eq!(counts, [2, 2]);
        let err = preset_by_name("not-a-set").unwrap_err().to_string();
        assert!(err.contains("devign"), "{err}");
        assert!(err.contains("ag_news"), "{err}");
        let bool_at = PARQUET_SCRIPT.find("isinstance(label, bool)").unwrap();
        let integral_at = PARQUET_SCRIPT.find("numbers.Integral").unwrap();
        assert!(bool_at < integral_at);
        assert!(PARQUET_SCRIPT.contains("label = int(label)"));
        assert!(!PARQUET_SCRIPT.contains("label is not an int"));
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
            assert_eq!(
                opts[pos]["key"].as_str().unwrap(),
                row["answer_key"].as_str().unwrap()
            );
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
        assert!(nli.train[0]["state"]["premise"]
            .as_str()
            .unwrap()
            .contains("dog"));
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
        let rows = import_fingerprint("fancyzhx/ag_news", "3000", "all", 42, "t", "h", "rows-api");
        let legacy = legacy_import_fingerprint("fancyzhx/ag_news", "3000", "all", 42, "t", "h");
        assert_eq!(rows, legacy);
        let touched = import_fingerprint(
            "fancyzhx/ag_news",
            "3000",
            "all",
            42,
            "t",
            "h",
            "dir=/nas\nfile sha256=abc bytes=4 mtime=10",
        );
        let retouched = import_fingerprint(
            "fancyzhx/ag_news",
            "3000",
            "all",
            42,
            "t",
            "h",
            "dir=/nas\nfile sha256=abc bytes=4 mtime=99",
        );
        assert_eq!(touched, retouched);
        let changed = import_fingerprint(
            "fancyzhx/ag_news",
            "3000",
            "all",
            42,
            "t",
            "h",
            "dir=/nas\nfile sha256=def bytes=4 mtime=10",
        );
        assert_ne!(touched, changed);
        let stored = legacy_import_fingerprint("fancyzhx/ag_news", "3000", "all", 42, "t", "h");
        let kept = journey_prepare_fingerprint(
            "fancyzhx/ag_news",
            "3000",
            "all",
            42,
            "t",
            "h",
            "dir=/nas\nfile sha256=abc bytes=4 mtime=99",
            Some(&stored),
            Some("dir=/nas\nfile sha256=abc bytes=4 mtime=10"),
        );
        assert_eq!(kept, stored);
        let redo = journey_prepare_fingerprint(
            "fancyzhx/ag_news",
            "10000",
            "all",
            42,
            "t",
            "h",
            "dir=/nas\nfile sha256=abc bytes=4 mtime=10",
            Some(&stored),
            Some("dir=/nas\nfile sha256=abc bytes=4 mtime=10"),
        );
        assert_ne!(redo, stored);
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
        assert!(
            !dest.is_file(),
            "partial stays unpublished until both splits verify"
        );
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
        assert!(
            err.contains(&dir.join(".lock").display().to_string()),
            "{err}"
        );
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
        train_rows: u64,
        test_rows: u64,
        label_mod: u64,
    }

    fn fake_io(pyarrow_missing: bool, touch_source: bool) -> FakeIo {
        let preset = preset_by_name("ag_news").unwrap();
        FakeIo {
            hf_calls: std::cell::Cell::new(0),
            http_calls: std::cell::Cell::new(0),
            pyarrow_missing,
            touch_source,
            train_rows: preset.official_train,
            test_rows: preset.official_test,
            label_mod: 4,
        }
    }

    impl ImportIo for FakeIo {
        fn read_parquet(&self, job: &ParquetJob<'_>) -> Result<()> {
            if job.pair_old.is_some() {
                use std::io::Write;
                let mut file = std::fs::File::create(job.out_partial)?;
                let empty = job.commit_rows.saturating_sub(RUST_USABLE_PAIRS);
                for i in 0..job.commit_rows {
                    if i < empty {
                        writeln!(
                            file,
                            "{{\"index\":{i},\"old\":\"\",\"new\":\"fn kept_{i}(){{}}\",\"lang\":\"Rust\"}}"
                        )?;
                    } else {
                        writeln!(
                            file,
                            "{{\"index\":{i},\"old\":\"fn old_{i}(){{}}\",\"new\":\"fn new_{i}(){{}}\",\"lang\":\"Rust\"}}"
                        )?;
                    }
                }
                return Ok(());
            }
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
            let n = if job.split == "train" {
                self.train_rows
            } else {
                self.test_rows
            };
            use std::io::Write;
            let mut file = std::fs::File::create(job.out_partial)?;
            for i in 0..n {
                writeln!(
                    file,
                    "{{\"index\":{i},\"text\":\"row-{i}\",\"label\":{}}}",
                    i % self.label_mod
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
            if argv.iter().any(|arg| arg == "data/rust/*") {
                let rust_dir = dir.join("data").join("rust");
                fs::create_dir_all(&rust_dir)?;
                fs::write(rust_dir.join("data.jsonl"), b"{}\n")?;
            } else {
                fs::create_dir_all(dir.join("data"))?;
                fs::write(dir.join("data").join("train-00000-of-00001.parquet"), b"t")?;
                fs::write(dir.join("data").join("test-00000-of-00001.parquet"), b"e")?;
            }
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
        let io = fake_io(false, false);
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
        assert!(
            !native.join(".lock").exists(),
            "lock is dropped after the import"
        );
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
        let io = fake_io(true, false);
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
        let io = fake_io(false, false);
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
        assert!(
            train.contains("\"text\":\"row-0\""),
            "snapshot was not read"
        );
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
        let err = split_parquet_files(&source, "train")
            .unwrap_err()
            .to_string();
        assert!(err.contains("directory symlink"), "{err}");
        let cycle = root.join("cycle");
        fs::create_dir_all(&cycle).unwrap();
        fs::write(cycle.join("train-0.parquet"), b"t").unwrap();
        std::os::unix::fs::symlink(&cycle, cycle.join("again")).unwrap();
        let err = split_parquet_files(&cycle, "train")
            .unwrap_err()
            .to_string();
        assert!(err.contains("directory symlink"), "{err}");
        let bool_at = PARQUET_SCRIPT.find("isinstance(label, bool)").unwrap();
        let integral_at = PARQUET_SCRIPT.find("numbers.Integral").unwrap();
        assert!(bool_at < integral_at);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn from_local_symlink_to_a_snapshot_succeeds() {
        let root = std::env::temp_dir().join(format!(
            "import-link-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let real = snapshot_dir(&root, "nas", b"train-bytes", b"test-bytes");
        let link = root.join("ag_news");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let out = root.join("sampled");
        let cache = root.join("cache");
        let io = fake_io(false, false);
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
                from_local: Some(&link),
                fetch: ImportFetch::Bulk,
                python: None,
                cache_root: Some(&cache),
            },
            &io,
        )
        .unwrap();
        let manifest: Value = serde_json::from_str(
            &fs::read_to_string(cache.join("ag_news").join("native").join("manifest.json"))
                .unwrap(),
        )
        .unwrap();
        let source_dir = manifest["source_dir"].as_str().unwrap();
        assert!(source_dir.contains("nas"), "{source_dir}");
        assert!(!source_dir.ends_with("ag_news"), "{source_dir}");
        let outside = root.join("outside");
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, real.join("escape")).unwrap();
        let err = split_parquet_files(&real, "train").unwrap_err().to_string();
        assert!(err.contains("directory symlink"), "{err}");
        let err = match inspect_parquet_source(&link, "train", "test") {
            Err(err) => err.to_string(),
            Ok(_) => panic!("nested directory symlink was accepted"),
        };
        assert!(err.contains("directory symlink"), "{err}");
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
        let io = fake_io(false, false);
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
        let io = fake_io(false, false);
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
        assert_eq!(
            io.hf_calls.get(),
            1,
            "a matching bulk snapshot is not downloaded again"
        );
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

    #[test]
    fn bulk_does_not_reuse_a_verified_cache_from_another_source() {
        let root = std::env::temp_dir().join(format!(
            "import-bulk-source-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let cache = root.join("cache");
        let native = cache.join("ag_news").join("native");
        fs::create_dir_all(&native).unwrap();
        let preset = preset_by_name("ag_news").unwrap();
        let row = "{\"index\":0,\"text\":\"a\",\"label\":0}\n";
        fs::write(
            native.join("train.jsonl"),
            row.repeat(preset.official_train as usize),
        )
        .unwrap();
        fs::write(
            native.join("test.jsonl"),
            row.repeat(preset.official_test as usize),
        )
        .unwrap();
        fs::write(
            native.join("manifest.json"),
            format!(
                "{{\"hf_id\":{},\"source\":\"datasets-server rows API\",\"complete\":true,\"train_rows\":{},\"test_rows\":{}}}\n",
                serde_json::to_string(preset.hf_id).unwrap(),
                preset.official_train,
                preset.official_test
            ),
        )
        .unwrap();
        assert!(cache_verified(preset, &native).unwrap());
        let out = root.join("sampled");
        let io = fake_io(false, false);
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
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(native.join("manifest.json")).unwrap())
                .unwrap();
        assert!(manifest["source_fp"].as_str().unwrap().contains("sha256="));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn file_symlink_to_parquet_is_followed() {
        let root = std::env::temp_dir().join(format!(
            "import-file-link-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let real = root.join("payload");
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("train-bytes.parquet"), b"train-payload").unwrap();
        fs::write(real.join("test-bytes.parquet"), b"test-payload").unwrap();
        let source = root.join("snap").join("data");
        fs::create_dir_all(&source).unwrap();
        std::os::unix::fs::symlink(
            real.join("train-bytes.parquet"),
            source.join("train-00000-of-00001.parquet"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            real.join("test-bytes.parquet"),
            source.join("test-00000-of-00001.parquet"),
        )
        .unwrap();
        let snap = root.join("snap");
        let files = split_parquet_files(&snap, "train").unwrap();
        assert_eq!(files.len(), 1);
        let token = dataset_source_token(Some(&snap), ImportFetch::Bulk, "train", "test");
        assert!(token.contains("sha256="), "{token}");
        let again = dataset_source_token(Some(&snap), ImportFetch::Bulk, "train", "test");
        assert_eq!(token, again);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn devign_from_local_reads_test_parquet_and_skips_validation() {
        let root = std::env::temp_dir().join(format!(
            "import-devign-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let source = snapshot_dir(&root, "nas", b"train-bytes", b"test-bytes");
        fs::write(
            source
                .join("data")
                .join("validation-00000-of-00001.parquet"),
            b"validation-bytes",
        )
        .unwrap();
        let test_files = split_parquet_files(&source, "test").unwrap();
        assert_eq!(test_files.len(), 1);
        assert!(
            test_files
                .iter()
                .all(|path| !path.display().to_string().contains("validation")),
            "{test_files:?}"
        );
        let preset = preset_by_name("devign").unwrap();
        let mut io = fake_io(false, false);
        io.train_rows = preset.official_train;
        io.test_rows = preset.official_test;
        io.label_mod = 2;
        let out = root.join("sampled");
        let cache = root.join("cache");
        classify_import_with(
            &ImportRequest {
                dataset: "google/code_x_glue_cc_defect_detection",
                train_size: "4",
                heldout_size: "4",
                seed: 42,
                out: &out,
                force: false,
                native_train: None,
                native_test: None,
                from_local: Some(&source),
                fetch: ImportFetch::Bulk,
                python: Some("python3"),
                cache_root: Some(&cache),
            },
            &io,
        )
        .unwrap();
        assert_eq!(io.hf_calls.get(), 0);
        assert_eq!(io.http_calls.get(), 0);
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(out.join("import.json")).unwrap()).unwrap();
        assert_eq!(manifest["dataset"], "devign");
        assert_eq!(manifest["hf_id"], preset.hf_id);
        assert_eq!(manifest["heldout_split"], "test");
        assert_eq!(manifest["official_train"], 21_854);
        assert_eq!(manifest["official_test"], 2_732);
        assert_eq!(manifest["option_order"], "fixed");
        let note = manifest["option_order_note"].as_str().unwrap();
        assert!(note.contains("A=Secure, B=Insecure"), "{note}");
        let native: Value = serde_json::from_str(
            &fs::read_to_string(cache.join("devign").join("native").join("manifest.json")).unwrap(),
        )
        .unwrap();
        let parquet = native["parquet"].to_string();
        assert!(
            parquet.contains("train-00000-of-00001.parquet"),
            "{parquet}"
        );
        assert!(parquet.contains("test-00000-of-00001.parquet"), "{parquet}");
        assert!(!parquet.contains("validation"), "{parquet}");
        let train = fs::read_to_string(out.join("train.jsonl")).unwrap();
        assert!(train.contains("\"key\":\"secure\""));
        assert!(train.contains("\"key\":\"insecure\""));
        let later = classify_import_with(
            &ImportRequest {
                dataset: "banking77",
                train_size: "4",
                heldout_size: "4",
                seed: 42,
                out: &root.join("later"),
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
        .unwrap_err()
        .to_string();
        assert!(later.contains("later slice"), "{later}");
        assert!(later.contains("devign"), "{later}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn devign_native_jsonl_accepts_bool_and_int_labels() {
        let dir = std::env::temp_dir().join(format!(
            "import-devign-native-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let train = dir.join("native-train.jsonl");
        let test = dir.join("native-test.jsonl");
        fs::write(
            &train,
            concat!(
                "{\"index\":1,\"text\":\"void secure(){}\",\"label\":false}\n",
                "{\"index\":2,\"text\":\"void insecure(){}\",\"label\":true}\n",
                "{\"index\":3,\"text\":\"void also_secure(){}\",\"label\":0}\n",
                "{\"index\":4,\"text\":\"void also_insecure(){}\",\"label\":1}\n",
            ),
        )
        .unwrap();
        fs::write(
            &test,
            concat!(
                "{\"index\":11,\"text\":\"void held_secure(){}\",\"label\":false}\n",
                "{\"index\":12,\"text\":\"void held_insecure(){}\",\"label\":1}\n",
            ),
        )
        .unwrap();
        let out = dir.join("out");
        cmd_classify_import(&ImportRequest {
            dataset: "devign",
            train_size: "all",
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
        let body = fs::read_to_string(out.join("train.jsonl")).unwrap();
        let mut answers = BTreeMap::new();
        for line in body.lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            let state = row["state"].as_str().unwrap().to_string();
            let answer = row["answer"].as_str().unwrap().to_string();
            answers.insert(state, answer);
        }
        assert_eq!(answers["void secure(){}"], "A");
        assert_eq!(answers["void also_secure(){}"], "A");
        assert_eq!(answers["void insecure(){}"], "B");
        assert_eq!(answers["void also_insecure(){}"], "B");
        let bad = dir.join("bad.jsonl");
        fs::write(
            &bad,
            "{\"index\":1,\"text\":\"void x(){}\",\"label\":\"nope\"}\n",
        )
        .unwrap();
        let err = cmd_classify_import(&ImportRequest {
            dataset: "devign",
            train_size: "all",
            heldout_size: "all",
            seed: 42,
            out: &dir.join("bad-out"),
            force: false,
            native_train: Some(&bad),
            native_test: Some(&test),
            from_local: None,
            fetch: ImportFetch::Bulk,
            python: None,
            cache_root: None,
        })
        .unwrap_err()
        .to_string();
        assert!(err.contains("not a bool or integer"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rust_idiom_holdout_keeps_pairs_together_and_fixed_ab_order() {
        let preset = preset_by_name("rust_idiom").unwrap();
        let by_hub = preset_by_name("bigcode/commitpackft").unwrap();
        let by_slug = preset_by_name("rustidiom").unwrap();
        assert!(std::ptr::eq(preset, by_hub));
        assert!(std::ptr::eq(preset, by_slug));
        assert_eq!(preset.slug, "rustidiom");
        assert_eq!(preset.rows_config, "rust");
        assert_eq!(preset.official_train, 3_744);
        assert_eq!(preset.official_test, 936);
        assert!(preset.license_note.contains("Do not redistribute"));
        assert!(preset.license_note.contains("local training only"));
        let note = heldout_note(preset);
        assert!(note.contains("no test split"), "{note}");
        assert!(note.contains("seed 42"), "{note}");
        assert!(note.contains("2996"), "{note}");
        assert!(note.contains("2340"), "{note}");
        assert!(PARQUET_SCRIPT.contains("pair_old"));
        assert!(PARQUET_SCRIPT.contains("old_contents") == false);
        assert!(PARQUET_SCRIPT.contains("pair_new"));
        let mut lines = Vec::new();
        for i in 0..10u64 {
            lines.push(CommitLine {
                index: i,
                old: format!("fn old_{i}() {{}}"),
                new: format!("fn new_{i}() {{}}"),
                lang: "Rust".into(),
            });
        }
        lines.push(CommitLine {
            index: 10,
            old: String::new(),
            new: "fn dropped() {}".into(),
            lang: "Rust".into(),
        });
        let tiny = DatasetPreset {
            official_train: 16,
            official_test: 4,
            shape: SourceShape::CommitPair {
                old_field: "old_contents",
                new_field: "new_contents",
                lang_field: "lang",
                lang_value: "Rust",
                include: "data/rust/*",
                source_commits: 11,
                usable_pairs: 10,
                holdout_seed: RUST_HOLDOUT_SEED,
            },
            ..*preset
        };
        let (train, test) = commits_to_splits(&tiny, &lines, "Rust").unwrap();
        assert_eq!(train.len(), 16);
        assert_eq!(test.len(), 4);
        let train_idx: std::collections::BTreeSet<_> = train.iter().map(|row| row.index / 2).collect();
        let test_idx: std::collections::BTreeSet<_> = test.iter().map(|row| row.index / 2).collect();
        assert!(train_idx.is_disjoint(&test_idx));
        for idxs in [&train_idx, &test_idx] {
            for idx in idxs {
                let pair: Vec<_> = train
                    .iter()
                    .chain(test.iter())
                    .filter(|row| row.index / 2 == *idx)
                    .map(|row| row.label)
                    .collect();
                assert_eq!(pair, vec![0, 1], "commit {idx} must stay together");
            }
        }
        let sampled = sample_records(
            preset,
            &train,
            &test,
            &SplitSize::All,
            &SplitSize::All,
            42,
        )
        .unwrap();
        assert_eq!(sampled.train[0]["question"], preset.question);
        let opts = sampled.train[0]["options"].as_array().unwrap();
        assert_eq!(opts[0]["label"], "A");
        assert_eq!(opts[0]["key"], "needs_fix");
        assert_eq!(opts[0]["description"], "NeedsFix");
        assert_eq!(opts[1]["label"], "B");
        assert_eq!(opts[1]["key"], "idiomatic");
        assert_eq!(opts[1]["description"], "Idiomatic");
        let mut wrong = lines.clone();
        wrong[0].lang = "Python".into();
        let bad_lang = commits_to_splits(&tiny, &wrong, "Rust")
            .unwrap_err()
            .to_string();
        assert!(bad_lang.contains("not Rust"), "{bad_lang}");
    }

    #[test]
    fn rust_idiom_from_local_and_rows_api_use_the_seeded_holdout() {
        let root = std::env::temp_dir().join(format!(
            "import-rust-idiom-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let source = root.join("nas").join("data").join("rust");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("data.jsonl"), b"{\"lang\":\"Rust\"}\n").unwrap();
        let preset = preset_by_name("rust_idiom").unwrap();
        let io = fake_io(false, false);
        let out = root.join("sampled");
        let cache = root.join("cache");
        classify_import_with(
            &ImportRequest {
                dataset: "rust_idiom",
                train_size: "4",
                heldout_size: "4",
                seed: 42,
                out: &out,
                force: false,
                native_train: None,
                native_test: None,
                from_local: Some(&root.join("nas")),
                fetch: ImportFetch::Bulk,
                python: Some("python3"),
                cache_root: Some(&cache),
            },
            &io,
        )
        .unwrap();
        assert_eq!(io.hf_calls.get(), 0);
        assert_eq!(io.http_calls.get(), 0);
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(out.join("import.json")).unwrap()).unwrap();
        assert_eq!(manifest["dataset"], "rust_idiom");
        assert_eq!(manifest["hf_id"], "bigcode/commitpackft");
        assert_eq!(manifest["heldout_kind"], "seeded-commit-holdout");
        assert_eq!(manifest["official_train"], 3_744);
        assert_eq!(manifest["official_test"], 936);
        assert_eq!(manifest["source_commits"], 2_996);
        assert_eq!(manifest["usable_pairs"], 2_340);
        assert_eq!(manifest["option_order"], "fixed");
        let note = manifest["option_order_note"].as_str().unwrap();
        assert!(note.contains("A=NeedsFix, B=Idiomatic"), "{note}");
        let train = fs::read_to_string(out.join("train.jsonl")).unwrap();
        assert!(train.contains("\"key\":\"needs_fix\""));
        assert!(train.contains("\"key\":\"idiomatic\""));
        assert_eq!(train.lines().count(), 4);
        let native: Value = serde_json::from_str(
            &fs::read_to_string(cache.join("rust_idiom").join("native").join("manifest.json"))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(native["source"], "local-jsonl");
        assert!(native["jsonl"].to_string().contains("data.jsonl"));
        assert_eq!(native["train_rows"], 3_744);
        assert_eq!(native["test_rows"], 936);
        assert_eq!(native["holdout_seed"], RUST_HOLDOUT_SEED);
        assert_eq!(native["pair_transform"], COMMIT_PAIR_TRANSFORM);

        let rows_root = root.join("rows");
        fs::create_dir_all(&rows_root).unwrap();
        let mut calls = 0u32;
        let page = |offset: u64| {
            let mut rows = Vec::new();
            let end = (offset + ROWS_PAGE).min(RUST_SOURCE_COMMITS);
            for idx in offset..end {
                let old = if idx < RUST_SOURCE_COMMITS - RUST_USABLE_PAIRS {
                    ""
                } else {
                    "fn old() {}"
                };
                rows.push(json!({
                    "row_idx": idx,
                    "row": {
                        "old_contents": old,
                        "new_contents": format!("fn new_{idx}() {{}}"),
                        "lang": "Rust"
                    }
                }));
            }
            json!({"num_rows_total": RUST_SOURCE_COMMITS, "rows": rows}).to_string()
        };
        let (train_rows, test_rows) = fetch_commit_pairs(
            preset,
            &rows_root,
            &mut |_| {
                let offset = calls as u64 * ROWS_PAGE;
                calls += 1;
                ok_page(page(offset))
            },
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(train_rows.len(), 3_744);
        assert_eq!(test_rows.len(), 936);
        assert!(calls >= 30, "{calls}");
        let url_bits = format!("config={}", preset.rows_config);
        assert_eq!(url_bits, "config=rust");
        let argv = hf_dataset_argv("hf", preset, Path::new("/tmp/rust"));
        assert!(argv.iter().any(|arg| arg == "--include"));
        assert!(argv.iter().any(|arg| arg == "data/rust/*"));
        assert!(argv.iter().any(|arg| arg == "bigcode/commitpackft"));
        let rows_manifest: Value =
            serde_json::from_str(&fs::read_to_string(rows_root.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(rows_manifest["holdout_seed"], RUST_HOLDOUT_SEED);
        assert_eq!(rows_manifest["pair_transform"], COMMIT_PAIR_TRANSFORM);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_pair_cache_rejects_a_missing_or_mismatched_holdout_seed() {
        let preset = preset_by_name("rust_idiom").unwrap();
        let root = std::env::temp_dir().join(format!(
            "import-holdout-id-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let native = root.join("rust_idiom").join("native");
        fs::create_dir_all(&native).unwrap();
        let row = "{\"index\":0,\"text\":\"stale-holdout\",\"label\":0}\n";
        fs::write(
            native.join("train.jsonl"),
            row.repeat(preset.official_train as usize),
        )
        .unwrap();
        fs::write(
            native.join("test.jsonl"),
            row.repeat(preset.official_test as usize),
        )
        .unwrap();
        let write_manifest = |extra: &str| {
            fs::write(
                native.join("manifest.json"),
                format!(
                    "{{\"hf_id\":{},\"complete\":true,\"train_rows\":{},\"test_rows\":{}{extra}}}\n",
                    serde_json::to_string(preset.hf_id).unwrap(),
                    preset.official_train,
                    preset.official_test
                ),
            )
            .unwrap();
        };
        write_manifest("");
        assert!(
            !cache_verified(preset, &native).unwrap(),
            "a cache with no holdout_seed must not verify"
        );
        write_manifest(",\"holdout_seed\":7,\"pair_transform\":\"drop-empty-or-identical;expand-old0-new1;holdout-one-fifth-whole-commit\"");
        assert!(
            !cache_verified(preset, &native).unwrap(),
            "a mismatched holdout_seed must not verify"
        );
        write_manifest(&format!(",\"holdout_seed\":{RUST_HOLDOUT_SEED}"));
        assert!(
            !cache_verified(preset, &native).unwrap(),
            "a cache with no pair_transform must not verify"
        );
        write_manifest(&format!(
            ",\"holdout_seed\":{RUST_HOLDOUT_SEED},\"pair_transform\":\"other-transform\""
        ));
        assert!(
            !cache_verified(preset, &native).unwrap(),
            "a mismatched pair_transform must not verify"
        );
        write_manifest(&format!(
            ",\"holdout_seed\":{RUST_HOLDOUT_SEED},\"pair_transform\":{transform}",
            transform = serde_json::to_string(COMMIT_PAIR_TRANSFORM).unwrap()
        ));
        assert!(cache_verified(preset, &native).unwrap());

        let ag = preset_by_name("ag_news").unwrap();
        let ag_native = root.join("ag");
        fs::create_dir_all(&ag_native).unwrap();
        fs::write(
            ag_native.join("train.jsonl"),
            row.repeat(ag.official_train as usize),
        )
        .unwrap();
        fs::write(
            ag_native.join("test.jsonl"),
            row.repeat(ag.official_test as usize),
        )
        .unwrap();
        fs::write(
            ag_native.join("manifest.json"),
            format!(
                "{{\"hf_id\":{},\"complete\":true,\"train_rows\":{},\"test_rows\":{}}}\n",
                serde_json::to_string(ag.hf_id).unwrap(),
                ag.official_train,
                ag.official_test
            ),
        )
        .unwrap();
        assert!(
            cache_verified(ag, &ag_native).unwrap(),
            "labeled-column caches do not require holdout_seed"
        );

        write_manifest("");
        let source = root.join("nas").join("data").join("rust");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("data.jsonl"), b"{\"lang\":\"Rust\"}\n").unwrap();
        let io = fake_io(false, false);
        classify_import_with(
            &ImportRequest {
                dataset: "rust_idiom",
                train_size: "4",
                heldout_size: "4",
                seed: 42,
                out: &root.join("sampled-rebuild"),
                force: false,
                native_train: None,
                native_test: None,
                from_local: Some(&root.join("nas")),
                fetch: ImportFetch::Bulk,
                python: Some("python3"),
                cache_root: Some(&root),
            },
            &io,
        )
        .unwrap();
        let rebuilt = fs::read_to_string(native.join("train.jsonl")).unwrap();
        assert!(
            rebuilt.contains("fn old_"),
            "missing holdout_seed must rebuild the native split"
        );
        assert!(!rebuilt.contains("stale-holdout"));
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(native.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["holdout_seed"], RUST_HOLDOUT_SEED);
        assert_eq!(manifest["pair_transform"], COMMIT_PAIR_TRANSFORM);
        let _ = fs::remove_dir_all(&root);
    }
}
