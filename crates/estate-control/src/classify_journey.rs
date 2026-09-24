//! tev1 reproduce journey: prepare, LoRA recipe, train, merge, GGUF, Ollama seat, scored eval.
//! `--print` is the default. `--run` executes. A comparison file is local output only.

use crate::classify::{cmd_classify_eval, cmd_classify_prepare, DatasetFormat};
use anyhow::{bail, Result};
use model_estate::{
    convert_hf_to_gguf_line, convert_hf_to_gguf_outfile, gguf_modelfile, llamafactory_template_name,
    ollama_create_line,
};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Hugging Face id LLaMA-Factory registers as Qwen3.5-4B-Thinking.
pub const DEFAULT_BASE: &str = "Qwen/Qwen3.5-4B";
/// Template name in LLaMA-Factory `constants.py` for that registration (`template="qwen3_5"`).
pub const QWEN35_TEMPLATE: &str = "qwen3_5";
pub const DEFAULT_BASE_TAG: &str = "qwen3.5:4b";
pub const DEFAULT_TAG: &str = "tev1-specialist";
pub const DEFAULT_DATASET: &str = "tev1_decisions";

const STEP_ORDER: &[&str] = &[
    "prepare",
    "recipe",
    "train",
    "merge-export",
    "gguf-convert",
    "ollama-create",
    "eval-base",
    "eval-specialist",
    "compare",
];

#[derive(Clone, Debug)]
pub struct JourneyPaths {
    pub out: PathBuf,
    pub recipe: PathBuf,
    pub export_yaml: PathBuf,
    pub adapter_dir: PathBuf,
    pub export_dir: PathBuf,
    pub gguf: PathBuf,
    pub modelfile: PathBuf,
    pub heldout: PathBuf,
    pub dataset_info: PathBuf,
    pub base_report: PathBuf,
    pub specialist_report: PathBuf,
    pub comparison: PathBuf,
}

impl JourneyPaths {
    pub fn new(out: &Path) -> Self {
        let export_dir = out.join("export");
        Self {
            recipe: out.join("recipe.yaml"),
            export_yaml: out.join("export.yaml"),
            adapter_dir: out.join("outputs"),
            gguf: convert_hf_to_gguf_outfile(&export_dir),
            modelfile: out.join("Modelfile"),
            heldout: out.join("heldout.jsonl"),
            dataset_info: out.join("dataset_info.json"),
            base_report: out.join("base-report.json"),
            specialist_report: out.join("specialist-report.json"),
            comparison: out.join("comparison.json"),
            export_dir,
            out: out.to_path_buf(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepAction {
    Run,
    Skip,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepPlan {
    pub name: &'static str,
    pub action: StepAction,
    pub detail: String,
}

/// Chat template for the train YAML.
/// Qwen3.5 uses `qwen3_5` from LLaMA-Factory's model list.
/// Other ids use the beachhead scanner.
pub fn train_template(base: &str) -> &'static str {
    let lower = base.to_ascii_lowercase();
    if lower.contains("qwen3.5") || lower.contains("qwen3_5") {
        QWEN35_TEMPLATE
    } else {
        llamafactory_template_name(base)
    }
}

pub fn lora_recipe_yaml(
    base: &str,
    dataset_name: &str,
    dataset_dir: &Path,
    output_dir: &Path,
    max_steps: Option<u32>,
) -> String {
    let template = train_template(base);
    let gauge = match max_steps {
        Some(steps) => format!("max_steps: {steps}\n"),
        None => String::new(),
    };
    format!(
        "\
# schema: cell-one.classify-journey.v0
# LoRA recipe for the tev1 one-letter target. Cell One does not train unless classify journey --run.
# template {template} is the LLaMA-Factory name for this base.
model_name_or_path: {base}
trust_remote_code: true
stage: sft
do_train: true
finetuning_type: lora
lora_rank: 8
lora_alpha: 16
lora_target: all
dataset: {dataset_name}
dataset_dir: {dataset_dir}
template: {template}
cutoff_len: 512
packing: false
output_dir: {output_dir}
overwrite_output_dir: true
per_device_train_batch_size: 1
gradient_accumulation_steps: 4
learning_rate: 1.0e-4
num_train_epochs: 1.0
{gauge}\
lr_scheduler_type: cosine
warmup_ratio: 0.03
bf16: true
report_to: none
",
        base = yaml_scalar(base),
        dataset_name = dataset_name,
        dataset_dir = yaml_scalar(&dataset_dir.display().to_string()),
        output_dir = yaml_scalar(&output_dir.display().to_string()),
        template = template,
        gauge = gauge,
    )
}

pub fn export_yaml(base: &str, adapter_dir: &Path, export_dir: &Path) -> String {
    let template = train_template(base);
    format!(
        "\
# schema: cell-one.classify-journey.v0
# Merge via llamafactory-cli export. Same keys as the LLaMA-Factory merge card.
model_name_or_path: {base}
adapter_name_or_path: {adapter}
template: {template}
trust_remote_code: true
finetuning_type: lora
export_dir: {export_dir}
export_size: 5
export_device: cpu
export_legacy_format: false
",
        base = yaml_scalar(base),
        adapter = yaml_scalar(&adapter_dir.display().to_string()),
        template = template,
        export_dir = yaml_scalar(&export_dir.display().to_string()),
    )
}

fn yaml_scalar(text: &str) -> String {
    if text.is_empty() || text.chars().any(|c| c.is_whitespace() || c == ':' || c == '#') {
        format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        text.to_string()
    }
}

pub fn dataset_name_in_info(info: &Value) -> Option<String> {
    info.as_object().and_then(|map| {
        map.keys()
            .next()
            .filter(|_| map.len() == 1)
            .cloned()
            .or_else(|| map.keys().find(|k| map[*k].get("file_name").is_some()).cloned())
    })
}

pub fn recipe_dataset_name(yaml: &str) -> Option<String> {
    yaml.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("dataset:")?;
        let name = rest.trim().trim_matches('"');
        if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    })
}

#[derive(Clone, Debug)]
pub struct ToolGaps {
    pub missing: Vec<String>,
}

impl ToolGaps {
    pub fn message(&self) -> String {
        format!(
            "refuse:classify-journey: {}",
            self.missing.join("; ")
        )
    }
}

pub fn tool_gaps(has: impl Fn(&str) -> bool, gpu_ok: bool) -> ToolGaps {
    let mut missing = Vec::new();
    if !has("llamafactory-cli") {
        missing.push("llamafactory-cli is not on PATH".into());
    }
    if !has("convert_hf_to_gguf.py") {
        missing.push("llama.cpp convert_hf_to_gguf.py is not on PATH".into());
    }
    if !has("ollama") {
        missing.push("ollama is not on PATH".into());
    }
    if !gpu_ok {
        missing.push("no GPU (nvidia-smi is missing or failed)".into());
    }
    ToolGaps { missing }
}

pub fn plan_steps(paths: &JourneyPaths, base_tag: &str, tag: &str) -> Vec<StepPlan> {
    let mut steps = Vec::new();
    let prepare_ready = paths.heldout.is_file()
        && paths.out.join("dataset.jsonl").is_file()
        && paths.dataset_info.is_file();
    steps.push(StepPlan {
        name: "prepare",
        action: if prepare_ready { StepAction::Skip } else { StepAction::Run },
        detail: format!("classify prepare -> {}", paths.out.display()),
    });
    let recipe_ready = paths.recipe.is_file() && paths.export_yaml.is_file();
    steps.push(StepPlan {
        name: "recipe",
        action: if recipe_ready { StepAction::Skip } else { StepAction::Run },
        detail: format!("write {}", paths.recipe.display()),
    });
    let trained = paths.adapter_dir.join("adapter_config.json").is_file();
    steps.push(StepPlan {
        name: "train",
        action: if trained { StepAction::Skip } else { StepAction::Run },
        detail: format!("llamafactory-cli train {}", paths.recipe.display()),
    });
    let merged = paths.export_dir.join("config.json").is_file();
    steps.push(StepPlan {
        name: "merge-export",
        action: if merged { StepAction::Skip } else { StepAction::Run },
        detail: format!("llamafactory-cli export {}", paths.export_yaml.display()),
    });
    let converted = paths.gguf.is_file();
    steps.push(StepPlan {
        name: "gguf-convert",
        action: if converted { StepAction::Skip } else { StepAction::Run },
        detail: convert_hf_to_gguf_line(&paths.export_dir),
    });
    steps.push(StepPlan {
        name: "ollama-create",
        action: StepAction::Run,
        detail: ollama_create_line(tag, &paths.modelfile),
    });
    steps.push(StepPlan {
        name: "eval-base",
        action: if paths.base_report.is_file() { StepAction::Skip } else { StepAction::Run },
        detail: format!("classify eval model {base_tag}"),
    });
    steps.push(StepPlan {
        name: "eval-specialist",
        action: if paths.specialist_report.is_file() {
            StepAction::Skip
        } else {
            StepAction::Run
        },
        detail: format!("classify eval model {tag}"),
    });
    steps.push(StepPlan {
        name: "compare",
        action: StepAction::Run,
        detail: format!("write {}", paths.comparison.display()),
    });
    debug_assert_eq!(
        steps.iter().map(|s| s.name).collect::<Vec<_>>(),
        STEP_ORDER
    );
    steps
}

/// `ollama-create` is skippable when `ollama show` already succeeds. The plan
/// marks it run until that check, so print mode lists the create line.
pub fn mark_ollama_skip(steps: &mut [StepPlan]) {
    if let Some(step) = steps.iter_mut().find(|s| s.name == "ollama-create") {
        step.action = StepAction::Skip;
        step.detail = format!("skip; {}", step.detail);
    }
}

#[derive(Clone, Debug)]
pub struct SideScore {
    pub accuracy: f64,
    pub invalid: u64,
    pub records: u64,
    pub p50: f64,
    pub p95: f64,
}

pub fn side_score(report: &Value) -> Result<SideScore> {
    let records = report
        .get("records")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-journey: eval report has no records"))?;
    let accuracy = report
        .get("accuracy")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-journey: eval report has no accuracy"))?;
    let invalid = report.get("invalid").and_then(Value::as_u64).unwrap_or(0);
    let p50 = report
        .pointer("/latency_ms/p50")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let p95 = report
        .pointer("/latency_ms/p95")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    Ok(SideScore {
        accuracy,
        invalid,
        records,
        p50,
        p95,
    })
}

#[derive(Clone, Debug)]
pub struct Comparison {
    pub base_accuracy: f64,
    pub specialist_accuracy: f64,
    pub delta: f64,
    pub base_invalid_rate: f64,
    pub specialist_invalid_rate: f64,
    pub base_p50: f64,
    pub specialist_p50: f64,
    pub base_p95: f64,
    pub specialist_p95: f64,
}

pub fn compare_sides(base: &SideScore, specialist: &SideScore) -> Comparison {
    Comparison {
        base_accuracy: base.accuracy,
        specialist_accuracy: specialist.accuracy,
        delta: specialist.accuracy - base.accuracy,
        base_invalid_rate: rate(base.invalid, base.records),
        specialist_invalid_rate: rate(specialist.invalid, specialist.records),
        base_p50: base.p50,
        specialist_p50: specialist.p50,
        base_p95: base.p95,
        specialist_p95: specialist.p95,
    }
}

fn rate(invalid: u64, records: u64) -> f64 {
    if records == 0 {
        0.0
    } else {
        invalid as f64 / records as f64
    }
}

/// `None` when no threshold is set. `Some(true)` when every set threshold holds.
pub fn threshold_met(
    comparison: &Comparison,
    min_delta: Option<f64>,
    min_accuracy: Option<f64>,
) -> Option<bool> {
    if min_delta.is_none() && min_accuracy.is_none() {
        return None;
    }
    let delta_ok = min_delta
        .map(|min| comparison.delta + f64::EPSILON >= min)
        .unwrap_or(true);
    let acc_ok = min_accuracy
        .map(|min| comparison.specialist_accuracy + f64::EPSILON >= min)
        .unwrap_or(true);
    Some(delta_ok && acc_ok)
}

pub struct JourneyRequest<'a> {
    pub input: &'a Path,
    pub out: &'a Path,
    pub base: &'a str,
    pub base_tag: &'a str,
    pub tag: &'a str,
    pub endpoint: &'a str,
    pub dataset_name: &'a str,
    pub seed: u64,
    pub held_out_ratio: f64,
    pub max_steps: Option<u32>,
    pub force: bool,
    pub print: bool,
    pub run: bool,
    pub min_delta: Option<f64>,
    pub min_accuracy: Option<f64>,
    pub timeout_secs: u64,
}

pub fn cmd_classify_journey(req: &JourneyRequest<'_>) -> Result<()> {
    if req.print && req.run {
        bail!("refuse:classify-journey: pass only one of --print and --run");
    }
    if req.base.trim().is_empty() {
        bail!("refuse:classify-journey: --base is empty");
    }
    if req.tag.trim().is_empty() || req.base_tag.trim().is_empty() {
        bail!("refuse:classify-journey: model tag is empty");
    }
    let paths = JourneyPaths::new(req.out);
    let template = train_template(req.base);
    let mut steps = plan_steps(&paths, req.base_tag, req.tag);
    if !req.run {
        print_plan(req, &paths, template, &steps);
        return Ok(());
    }
    let gaps = tool_gaps(tool_on_path, gpu_present());
    if !gaps.missing.is_empty() {
        bail!(gaps.message());
    }
    if ollama_has_model(req.tag) {
        mark_ollama_skip(&mut steps);
    }
    if !ollama_has_model(req.base_tag) {
        bail!(
            "refuse:classify-journey: base tag {} is not seated in Ollama. Seat it, then re-run.",
            req.base_tag
        );
    }
    execute(req, &paths, &steps)?;
    let base_report = read_json(&paths.base_report)?;
    let specialist_report = read_json(&paths.specialist_report)?;
    let comparison = compare_sides(&side_score(&base_report)?, &side_score(&specialist_report)?);
    let verdict = threshold_met(&comparison, req.min_delta, req.min_accuracy);
    write_comparison(req, &paths, template, &comparison, verdict)?;
    match verdict {
        Some(false) => bail!(
            "classify-journey: threshold missed; specialist accuracy {:.4} delta {:.4}",
            comparison.specialist_accuracy, comparison.delta
        ),
        Some(true) => println!("classify-journey: threshold met"),
        None => {}
    }
    Ok(())
}

fn print_plan(req: &JourneyRequest<'_>, paths: &JourneyPaths, template: &str, steps: &[StepPlan]) {
    println!("classify journey: print");
    println!("base: {} template: {template}", req.base);
    println!(
        "base_tag: {} specialist_tag: {} endpoint: {}",
        req.base_tag, req.tag, req.endpoint
    );
    if req.base == DEFAULT_BASE {
        println!(
            "default base {DEFAULT_BASE} is registered in LLaMA-Factory constants.py as Qwen3.5-4B-Thinking with template {QWEN35_TEMPLATE}"
        );
    }
    println!("out: {}", paths.out.display());
    for step in steps {
        let word = match step.action {
            StepAction::Run => "run",
            StepAction::Skip => "skip",
        };
        println!("{word} {:<16} {}", step.name, step.detail);
    }
    println!("compare writes {}", paths.comparison.display());
    println!("This print does not train, convert, or seat. A later report is local output. It does not record a live PASS. READY_FOR_LIVE_TEST: no.");
}

fn execute(req: &JourneyRequest<'_>, paths: &JourneyPaths, steps: &[StepPlan]) -> Result<()> {
    for step in steps {
        if step.action == StepAction::Skip && step.name != "compare" && step.name != "ollama-create"
        {
            println!("skip {}", step.name);
            continue;
        }
        match step.name {
            "prepare" => {
                if step.action == StepAction::Skip {
                    println!("skip prepare");
                    continue;
                }
                cmd_classify_prepare(
                    req.input,
                    &paths.out,
                    req.seed,
                    req.held_out_ratio,
                    DatasetFormat::Sharegpt,
                    req.dataset_name,
                    false,
                    req.force,
                )?;
            }
            "recipe" => {
                if step.action == StepAction::Skip {
                    println!("skip recipe");
                    continue;
                }
                write_recipe(req, paths)?;
            }
            "train" => {
                if step.action == StepAction::Skip {
                    println!("skip train");
                    continue;
                }
                run_tool(
                    "llamafactory-cli",
                    &["train", &paths.recipe.display().to_string()],
                )?;
            }
            "merge-export" => {
                if step.action == StepAction::Skip {
                    println!("skip merge-export");
                    continue;
                }
                run_tool(
                    "llamafactory-cli",
                    &["export", &paths.export_yaml.display().to_string()],
                )?;
            }
            "gguf-convert" => {
                if step.action == StepAction::Skip {
                    println!("skip gguf-convert");
                    continue;
                }
                let script = which("convert_hf_to_gguf.py").ok_or_else(|| {
                    anyhow::anyhow!(
                        "refuse:classify-journey: llama.cpp convert_hf_to_gguf.py is not on PATH"
                    )
                })?;
                let export = paths.export_dir.display().to_string();
                let outfile = paths.gguf.display().to_string();
                run_path(
                    &script,
                    &[&export, "--outfile", &outfile, "--outtype", "auto"],
                )?;
            }
            "ollama-create" => {
                if step.action == StepAction::Skip || ollama_has_model(req.tag) {
                    println!("skip ollama-create");
                    continue;
                }
                if !paths.gguf.is_file() {
                    bail!(
                        "refuse:classify-journey: GGUF missing at {}",
                        paths.gguf.display()
                    );
                }
                fs::write(&paths.modelfile, gguf_modelfile(&paths.gguf))?;
                let file = paths.modelfile.display().to_string();
                run_tool("ollama", &["create", req.tag, "-f", &file])?;
            }
            "eval-base" => {
                if step.action == StepAction::Skip {
                    println!("skip eval-base");
                    continue;
                }
                cmd_classify_eval(
                    &paths.heldout,
                    Some(req.endpoint),
                    req.base_tag,
                    None,
                    &paths.base_report,
                    false,
                    false,
                    req.timeout_secs,
                )?;
            }
            "eval-specialist" => {
                if step.action == StepAction::Skip {
                    println!("skip eval-specialist");
                    continue;
                }
                cmd_classify_eval(
                    &paths.heldout,
                    Some(req.endpoint),
                    req.tag,
                    None,
                    &paths.specialist_report,
                    false,
                    false,
                    req.timeout_secs,
                )?;
            }
            "compare" => {}
            other => bail!("refuse:classify-journey: unknown step {other}"),
        }
    }
    Ok(())
}

fn write_recipe(req: &JourneyRequest<'_>, paths: &JourneyPaths) -> Result<()> {
    let info_text = fs::read_to_string(&paths.dataset_info).map_err(|e| {
        anyhow::anyhow!(
            "refuse:classify-journey: cannot read {}: {e}",
            paths.dataset_info.display()
        )
    })?;
    let info: Value = serde_json::from_str(&info_text)?;
    let wired = dataset_name_in_info(&info).ok_or_else(|| {
        anyhow::anyhow!("refuse:classify-journey: dataset_info.json has no dataset name")
    })?;
    if wired != req.dataset_name {
        bail!(
            "refuse:classify-journey: dataset_info key {wired} does not match --dataset-name {}",
            req.dataset_name
        );
    }
    let recipe = lora_recipe_yaml(
        req.base,
        req.dataset_name,
        &paths.out,
        &paths.adapter_dir,
        req.max_steps,
    );
    let export = export_yaml(req.base, &paths.adapter_dir, &paths.export_dir);
    if recipe_dataset_name(&recipe).as_deref() != Some(req.dataset_name) {
        bail!("refuse:classify-journey: recipe dataset field does not match prepare output");
    }
    fs::write(&paths.recipe, recipe)?;
    fs::write(&paths.export_yaml, export)?;
    Ok(())
}

fn write_comparison(
    req: &JourneyRequest<'_>,
    paths: &JourneyPaths,
    template: &str,
    comparison: &Comparison,
    verdict: Option<bool>,
) -> Result<()> {
    let verdict_text = match verdict {
        Some(true) => "met",
        Some(false) => "missed",
        None => "unset",
    };
    let report = json!({
        "schema": "cell-one.classify-journey.v0",
        "base": req.base,
        "template": template,
        "base_tag": req.base_tag,
        "specialist_tag": req.tag,
        "endpoint": req.endpoint,
        "base_accuracy": comparison.base_accuracy,
        "specialist_accuracy": comparison.specialist_accuracy,
        "delta": comparison.delta,
        "base_invalid_rate": comparison.base_invalid_rate,
        "specialist_invalid_rate": comparison.specialist_invalid_rate,
        "latency_ms": {
            "base_p50": comparison.base_p50,
            "base_p95": comparison.base_p95,
            "specialist_p50": comparison.specialist_p50,
            "specialist_p95": comparison.specialist_p95
        },
        "min_delta": req.min_delta,
        "min_accuracy": req.min_accuracy,
        "threshold": verdict_text,
        "live_pass_recorded": false,
        "note": "Local comparison only. This file does not record a live PASS. READY_FOR_LIVE_TEST stays no."
    });
    if let Some(parent) = paths.comparison.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &paths.comparison,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).map_err(|e| {
        anyhow::anyhow!("refuse:classify-journey: cannot read {}: {e}", path.display())
    })?;
    Ok(serde_json::from_str(&text)?)
}

fn tool_on_path(name: &str) -> bool {
    which(name).is_some()
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn gpu_present() -> bool {
    let Some(bin) = which("nvidia-smi") else {
        return false;
    };
    Command::new(bin)
        .arg("-L")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn ollama_has_model(tag: &str) -> bool {
    let Some(bin) = which("ollama") else {
        return false;
    };
    Command::new(bin)
        .args(["show", tag])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn run_tool(name: &str, args: &[&str]) -> Result<()> {
    let bin = which(name).ok_or_else(|| {
        anyhow::anyhow!("refuse:classify-journey: {name} is not on PATH")
    })?;
    run_path(&bin, args)
}

fn run_path(bin: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new(bin).args(args).status().map_err(|e| {
        anyhow::anyhow!(
            "refuse:classify-journey: cannot run {}: {e}",
            bin.display()
        )
    })?;
    if !status.success() {
        bail!(
            "refuse:classify-journey: {} exited {status}",
            bin.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qwen35_template_comes_from_llamafactory_and_other_bases_use_the_scanner() {
        assert_eq!(train_template(DEFAULT_BASE), "qwen3_5");
        assert_eq!(train_template("Qwen/Qwen3.5-4B-Base"), "qwen3_5");
        assert_eq!(train_template("Qwen/Qwen3-4B-Instruct-2507"), "qwen3_nothink");
        assert_eq!(train_template("Qwen/Qwen3-4B"), "qwen3");
        assert_eq!(train_template("Qwen/Qwen2.5-3B-Instruct"), "qwen");
    }

    #[test]
    fn recipe_dataset_matches_prepare_dataset_info() {
        let dir = std::env::temp_dir().join(format!("journey-yaml-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let paths = JourneyPaths::new(&dir);
        let recipe = lora_recipe_yaml(
            DEFAULT_BASE,
            DEFAULT_DATASET,
            &dir,
            &paths.adapter_dir,
            Some(2),
        );
        assert!(recipe.contains("template: qwen3_5"), "{recipe}");
        assert!(recipe.contains("finetuning_type: lora"), "{recipe}");
        assert!(recipe.contains("max_steps: 2"), "{recipe}");
        assert_eq!(recipe_dataset_name(&recipe).as_deref(), Some(DEFAULT_DATASET));
        let info = json!({
            DEFAULT_DATASET: {
                "file_name": "dataset.jsonl",
                "formatting": "sharegpt"
            }
        });
        assert_eq!(dataset_name_in_info(&info).as_deref(), Some(DEFAULT_DATASET));
        let export = export_yaml(DEFAULT_BASE, &paths.adapter_dir, &paths.export_dir);
        assert!(export.contains("finetuning_type: lora"), "{export}");
        assert!(export.contains("template: qwen3_5"), "{export}");
        assert!(paths.gguf.ends_with("export.gguf"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn step_order_and_skip_when_outputs_exist() {
        let dir = std::env::temp_dir().join(format!("journey-plan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let paths = JourneyPaths::new(&dir);
        let fresh = plan_steps(&paths, DEFAULT_BASE_TAG, DEFAULT_TAG);
        let names: Vec<_> = fresh.iter().map(|s| s.name).collect();
        assert_eq!(names, STEP_ORDER);
        assert!(fresh.iter().any(|s| s.name == "gguf-convert" && s.detail.contains("convert_hf_to_gguf.py")));
        assert!(fresh.iter().any(|s| s.name == "ollama-create" && s.detail.contains("ollama create")));
        fs::create_dir_all(&paths.adapter_dir).unwrap();
        fs::create_dir_all(&paths.export_dir).unwrap();
        fs::write(paths.out.join("dataset.jsonl"), "{}\n").unwrap();
        fs::write(&paths.heldout, "{}\n").unwrap();
        fs::write(&paths.dataset_info, "{}\n").unwrap();
        fs::write(&paths.recipe, "dataset: tev1_decisions\n").unwrap();
        fs::write(&paths.export_yaml, "export_dir: x\n").unwrap();
        fs::write(paths.adapter_dir.join("adapter_config.json"), "{}\n").unwrap();
        fs::write(paths.export_dir.join("config.json"), "{}\n").unwrap();
        fs::write(&paths.gguf, "gguf").unwrap();
        fs::write(&paths.base_report, "{}\n").unwrap();
        fs::write(&paths.specialist_report, "{}\n").unwrap();
        let again = plan_steps(&paths, DEFAULT_BASE_TAG, DEFAULT_TAG);
        for step in &again {
            if matches!(step.name, "ollama-create" | "compare") {
                assert_eq!(step.action, StepAction::Run, "{}", step.name);
            } else {
                assert_eq!(step.action, StepAction::Skip, "{}", step.name);
            }
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_tool_messages_name_each_gap() {
        let gaps = tool_gaps(|_| false, false);
        let text = gaps.message();
        assert!(text.contains("llamafactory-cli is not on PATH"), "{text}");
        assert!(text.contains("convert_hf_to_gguf.py is not on PATH"), "{text}");
        assert!(text.contains("ollama is not on PATH"), "{text}");
        assert!(text.contains("no GPU"), "{text}");
        let partial = tool_gaps(|name| name == "ollama", true);
        let text = partial.message();
        assert!(text.contains("llamafactory-cli"));
        assert!(!text.contains("ollama is not"));
        assert!(!text.contains("no GPU"));
    }

    #[test]
    fn comparison_math_and_thresholds() {
        let base = SideScore {
            accuracy: 0.25,
            invalid: 2,
            records: 8,
            p50: 10.0,
            p95: 40.0,
        };
        let specialist = SideScore {
            accuracy: 0.75,
            invalid: 1,
            records: 8,
            p50: 12.0,
            p95: 30.0,
        };
        let comparison = compare_sides(&base, &specialist);
        assert!((comparison.delta - 0.5).abs() < 1e-9);
        assert!((comparison.base_invalid_rate - 0.25).abs() < 1e-9);
        assert!((comparison.specialist_invalid_rate - 0.125).abs() < 1e-9);
        assert_eq!(threshold_met(&comparison, None, None), None);
        assert_eq!(threshold_met(&comparison, Some(0.4), Some(0.7)), Some(true));
        assert_eq!(threshold_met(&comparison, Some(0.6), None), Some(false));
        assert_eq!(threshold_met(&comparison, None, Some(0.9)), Some(false));
    }
}
