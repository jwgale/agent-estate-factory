//! tev1 reproduce journey: prepare, LoRA recipe, train, merge, GGUF, quant, Ollama seat, scored eval.
//! `--print` is the default. `--run` executes. A comparison file is local output only.
//!
//! The base and the specialist share one convert, quant, and Modelfile shape. Only the LoRA differs.
//! `--base-tag` is an opt-in library tag and skips that base build.

use crate::classify::{cmd_classify_eval, cmd_classify_prepare, DatasetFormat, EvalApi, EvalGate};
use anyhow::{bail, Result};
use model_estate::llamafactory_template_name;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Hugging Face id LLaMA-Factory registers as Qwen3.5-4B-Thinking.
pub const DEFAULT_BASE: &str = "Qwen/Qwen3.5-4B";
/// Template name in LLaMA-Factory `constants.py` for that registration (`template="qwen3_5"`).
pub const QWEN35_TEMPLATE: &str = "qwen3_5";
/// Ollama tag created from the built base GGUF. Not a library tag.
pub const DEFAULT_BUILT_BASE_TAG: &str = "tev1-base";
pub const DEFAULT_TAG: &str = "tev1-specialist";
pub const DEFAULT_DATASET: &str = "tev1_decisions";
/// Default quant for both seats. `f16` skips `llama-quantize`.
pub const DEFAULT_QUANT: &str = "Q4_K_M";
/// Together fine-tune model id. Same Hub-style name as the local base. Override with `--together-model`.
pub const DEFAULT_TOGETHER_MODEL: &str = "Qwen/Qwen3.5-4B";
/// Together REST root. Fine-tune calls use this only when `--train-driver together` and `--run`.
pub const DEFAULT_TOGETHER_API: &str = "https://api.together.ai/v1";
pub const DEFAULT_TOGETHER_KEY_ENV: &str = "TOGETHER_API_KEY";
/// Wall-clock cap for Together job polling. Separate from the per-request HTTP timeout.
pub const DEFAULT_TOGETHER_POLL_SECS: u64 = 10_800;
/// DeepSeek-R1-Distill chat checkpoint. LLaMA-Factory template `deepseekr1`.
pub const DEEPSEEK_R1_DISTILL_BASE: &str = "deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B";
/// Ollama tag for the DeepSeek specialist GGUF.
pub const DEEPSEEK_R1_DISTILL_TAG: &str = "deepseek-r1-distill-specialist";
/// Ollama tag created from the built DeepSeek base GGUF. Not a library tag.
pub const DEEPSEEK_R1_DISTILL_BUILT_BASE_TAG: &str = "deepseek-r1-distill-base";
/// Compressed Together adapter archive cap. Larger downloads are refused.
pub const MAX_TOGETHER_ADAPTER_COMPRESSED: usize = 512 * 1024 * 1024;
/// Decompressed Together adapter tar cap. Larger unpacks are refused.
pub const MAX_TOGETHER_ADAPTER_DECOMPRESSED: usize = 2 * 1024 * 1024 * 1024;

const QWEN35_RECENT: &str = "Qwen3.5 needs a recent llama.cpp checkout";
const DEEPSEEK_LLAMA_NOTE: &str =
    "DeepSeek-R1-Distill needs a llama.cpp checkout that converts that architecture";

/// Which letter-journey defaults `classify journey` fills in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum JourneyPreset {
    /// `Qwen/Qwen3.5-4B`, template `qwen3_5`, tag `tev1-specialist`. This is the default.
    Tev1,
    /// `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`, template `deepseekr1`, tag `deepseek-r1-distill-specialist`.
    /// Train stays `llamafactory-cli` unless `--together-model` is set with `--train-driver together`.
    DeepseekR1Distill,
}

/// Modelfile chat shape. The base and the specialist share one shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeatChat {
    Qwen35,
    DeepseekR1,
}

/// Defaults after `--preset` replaces an untouched tev1 base or tag.
#[derive(Debug)]
pub struct AppliedJourney {
    pub base: String,
    pub tag: String,
    pub built_base_tag: &'static str,
    pub seat: SeatChat,
    pub llama_note: &'static str,
    pub together_model: String,
}

/// `--base` and `--tag` left at the tev1 defaults take the preset. An explicit value wins.
/// Together on the DeepSeek preset needs `--together-model`. The Qwen default stays the tev1 path.
pub fn apply_preset(
    preset: JourneyPreset,
    base: &str,
    tag: &str,
    train_driver: TrainDriver,
    together_model: Option<&str>,
) -> Result<AppliedJourney> {
    let (base, tag, built_base_tag, seat, llama_note) = match preset {
        JourneyPreset::Tev1 => (
            base.to_string(),
            tag.to_string(),
            DEFAULT_BUILT_BASE_TAG,
            SeatChat::Qwen35,
            QWEN35_RECENT,
        ),
        JourneyPreset::DeepseekR1Distill => {
            let base = if base == DEFAULT_BASE {
                DEEPSEEK_R1_DISTILL_BASE.to_string()
            } else {
                base.to_string()
            };
            let tag = if tag == DEFAULT_TAG {
                DEEPSEEK_R1_DISTILL_TAG.to_string()
            } else {
                tag.to_string()
            };
            (
                base,
                tag,
                DEEPSEEK_R1_DISTILL_BUILT_BASE_TAG,
                SeatChat::DeepseekR1,
                DEEPSEEK_LLAMA_NOTE,
            )
        }
    };
    let together_model = match (preset, train_driver, together_model.map(str::trim)) {
        (JourneyPreset::DeepseekR1Distill, TrainDriver::Together, None | Some("")) => {
            bail!(
                "refuse:classify-journey: DeepSeek-R1-Distill journey uses local llamafactory-cli train. Together stays on the tev1 Qwen path unless --together-model is set"
            );
        }
        (_, TrainDriver::Together, Some(model)) if !model.is_empty() => model.to_string(),
        (_, _, Some(model)) if !model.is_empty() => model.to_string(),
        _ => DEFAULT_TOGETHER_MODEL.to_string(),
    };
    Ok(AppliedJourney {
        base,
        tag,
        built_base_tag,
        seat,
        llama_note,
        together_model,
    })
}

/// Non-thinking Qwen3.5 generation prompt.
/// Official `enable_thinking=False` ends the prompt at
/// `<|im_start|>assistant\n<think>\n\n</think>\n\n` so the target is the letter only.
/// GGUF chat templates default thinking on, so the journey does not rely on the embedded template.
const QWEN35_NOTHINK_TEMPLATE: &str = "\
{{ if .System }}<|im_start|>system
{{ .System }}<|im_end|>
{{ end }}{{ range .Messages }}{{ if eq .Role \"user\" }}<|im_start|>user
{{ .Content }}<|im_end|>
{{ else if eq .Role \"assistant\" }}<|im_start|>assistant
{{ .Content }}<|im_end|>
{{ end }}{{ end }}<|im_start|>assistant
<think>

</think>

";

/// Non-thinking DeepSeek-R1-Distill prompt. Official chat tokens, generation ends at Assistant.
/// `enable_thinking: false` on the recipe keeps the one-letter target free of think tokens.
const DEEPSEEK_R1_TEMPLATE: &str = "\
{{ if .System }}<｜begin▁of▁sentence｜>{{ .System }}{{ else }}<｜begin▁of▁sentence｜>{{ end }}{{ range .Messages }}{{ if eq .Role \"user\" }}<｜User｜>
{{ .Content }}{{ else if eq .Role \"assistant\" }}<｜Assistant｜>
{{ .Content }}<｜end▁of▁sentence｜>{{ end }}{{ end }}<｜Assistant｜>
";

const STEP_ORDER: &[&str] = &[
    "prepare",
    "fetch-base",
    "recipe",
    "train",
    "merge-export",
    "gguf-convert-base",
    "gguf-convert-specialist",
    "quantize-base",
    "quantize-specialist",
    "ollama-create-base",
    "ollama-create-specialist",
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
    pub base_hf: PathBuf,
    pub base_f16: PathBuf,
    pub specialist_f16: PathBuf,
    pub base_modelfile: PathBuf,
    pub specialist_modelfile: PathBuf,
    pub heldout: PathBuf,
    pub dataset_jsonl: PathBuf,
    pub dataset_info: PathBuf,
    pub base_report: PathBuf,
    pub specialist_report: PathBuf,
    pub comparison: PathBuf,
    pub manifests: PathBuf,
}

impl JourneyPaths {
    pub fn new(out: &Path) -> Self {
        Self {
            recipe: out.join("recipe.yaml"),
            export_yaml: out.join("export.yaml"),
            adapter_dir: out.join("outputs"),
            export_dir: out.join("export"),
            base_hf: out.join("base-hf"),
            base_f16: out.join("base.f16.gguf"),
            specialist_f16: out.join("specialist.f16.gguf"),
            base_modelfile: out.join("base.Modelfile"),
            specialist_modelfile: out.join("specialist.Modelfile"),
            heldout: out.join("heldout.jsonl"),
            dataset_jsonl: out.join("dataset.jsonl"),
            dataset_info: out.join("dataset_info.json"),
            base_report: out.join("base-report.json"),
            specialist_report: out.join("specialist-report.json"),
            comparison: out.join("comparison.json"),
            manifests: out.join("manifests"),
            out: out.to_path_buf(),
        }
    }

    pub fn seated_gguf(&self, which: &str, quant: &str) -> PathBuf {
        if quant_is_f16(quant) {
            self.out.join(format!("{which}.f16.gguf"))
        } else {
            self.out.join(format!("{which}.{quant}.gguf"))
        }
    }

    fn manifest_path(&self, step: &str) -> PathBuf {
        self.manifests.join(format!("{step}.json"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum TrainDriver {
    /// `llamafactory-cli train` on this host. This is the default.
    Local,
    /// Upload the prepared dataset and launch a Together LoRA job. `--print` does not call the network.
    Together,
}

impl TrainDriver {
    fn fingerprint(self, together_model: &str) -> String {
        match self {
            TrainDriver::Local => "local".into(),
            TrainDriver::Together => format!("together\n{together_model}"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TrainDriver::Local => "local",
            TrainDriver::Together => "together",
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
    model_name: &str,
    template_from: &str,
    dataset_name: &str,
    dataset_dir: &Path,
    output_dir: &Path,
    max_steps: Option<u32>,
) -> String {
    let template = train_template(template_from);
    let gauge = match max_steps {
        Some(steps) => format!("max_steps: {steps}\n"),
        None => String::new(),
    };
    format!(
        "\
# schema: cell-one.classify-journey.v0
# LoRA recipe for the tev1 one-letter target. Cell One does not train unless classify journey --run.
# template {template} is the LLaMA-Factory name for this base.
# enable_thinking false keeps assistant targets free of think tokens.
# qwen3_5_nothink is for Instruct-only variants and is not this checkpoint.
model_name_or_path: {model}
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
enable_thinking: false
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
        model = yaml_scalar(model_name),
        dataset_name = dataset_name,
        dataset_dir = yaml_scalar(&dataset_dir.display().to_string()),
        output_dir = yaml_scalar(&output_dir.display().to_string()),
        template = template,
        gauge = gauge,
    )
}

pub fn export_yaml(
    model_name: &str,
    template_from: &str,
    adapter_dir: &Path,
    export_dir: &Path,
) -> String {
    let template = train_template(template_from);
    format!(
        "\
# schema: cell-one.classify-journey.v0
# Merge via llamafactory-cli export. Same keys as the LLaMA-Factory merge card.
model_name_or_path: {model}
adapter_name_or_path: {adapter}
template: {template}
enable_thinking: false
trust_remote_code: true
finetuning_type: lora
export_dir: {export_dir}
export_size: 5
export_device: cpu
export_legacy_format: false
",
        model = yaml_scalar(model_name),
        adapter = yaml_scalar(&adapter_dir.display().to_string()),
        template = template,
        export_dir = yaml_scalar(&export_dir.display().to_string()),
    )
}

/// Journey Modelfile. Same shape for the base and the specialist. `FROM` is the only difference.
/// Not `local_seat`'s `gguf_modelfile`. `FROM` uses the same token rules as `modelfile_token`.
pub fn journey_modelfile(gguf: &Path, seat: SeatChat) -> String {
    let gguf = absolute_gguf(gguf);
    let (stop, template) = match seat {
        SeatChat::Qwen35 => ("<|im_end|>", QWEN35_NOTHINK_TEMPLATE),
        SeatChat::DeepseekR1 => ("<｜end▁of▁sentence｜>", DEEPSEEK_R1_TEMPLATE),
    };
    format!(
        "\
FROM {gguf}
PARAMETER temperature 0
PARAMETER num_predict 8
PARAMETER stop {stop}
TEMPLATE \"\"\"{template}\"\"\"
",
        gguf = modelfile_from_token(&gguf),
    )
}

/// Absolute path for Modelfile `FROM`. Ollama resolves a relative `FROM` against the Modelfile directory.
/// Same order as `local_seat::render_gguf`: canonicalize when the file exists.
fn absolute_gguf(path: &Path) -> PathBuf {
    if let Ok(abs) = fs::canonicalize(path) {
        return abs;
    }
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

/// Quote a Modelfile token the same way `local_seat::modelfile_token` does.
fn modelfile_from_token(path: &Path) -> String {
    let text = path.display().to_string();
    if text.chars().any(|c| c.is_whitespace()) {
        format!("\"{}\"", text.replace('"', "\\\""))
    } else {
        text
    }
}

fn yaml_scalar(text: &str) -> String {
    if text.is_empty()
        || text
            .chars()
            .any(|c| c.is_whitespace() || c == ':' || c == '#')
    {
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
            .or_else(|| {
                map.keys()
                    .find(|k| map[*k].get("file_name").is_some())
                    .cloned()
            })
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

pub fn quant_is_f16(quant: &str) -> bool {
    quant.eq_ignore_ascii_case("f16")
}

#[derive(Clone, Debug)]
pub struct LlamaCpp {
    #[allow(dead_code)]
    pub dir: PathBuf,
    pub convert: PathBuf,
    pub quantize: PathBuf,
}

impl LlamaCpp {
    pub fn convert_argv(&self, src: &str, outfile: &Path) -> Vec<String> {
        vec![
            "python3".into(),
            self.convert.display().to_string(),
            src.to_string(),
            "--outfile".into(),
            outfile.display().to_string(),
            "--outtype".into(),
            "f16".into(),
        ]
    }

    pub fn quantize_argv(&self, f16: &Path, outfile: &Path, quant: &str) -> Vec<String> {
        vec![
            self.quantize.display().to_string(),
            f16.display().to_string(),
            outfile.display().to_string(),
            quant.to_string(),
        ]
    }
}

pub fn argv_line(argv: &[String]) -> String {
    argv.iter()
        .map(|arg| {
            if arg.is_empty() || arg.chars().any(|c| c.is_whitespace()) {
                format!("'{}'", arg.replace('\'', "'\\''"))
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Placeholder printed when `--llama-cpp-dir` and `LLAMA_CPP_DIR` are unset.
pub fn placeholder_convert_line(src: &str, outfile: &Path) -> String {
    argv_line(&[
        "python3".into(),
        "$LLAMA_CPP_DIR/convert_hf_to_gguf.py".into(),
        src.to_string(),
        "--outfile".into(),
        outfile.display().to_string(),
        "--outtype".into(),
        "f16".into(),
    ])
}

pub fn placeholder_quantize_line(f16: &Path, outfile: &Path, quant: &str) -> String {
    argv_line(&[
        "$LLAMA_CPP_DIR/llama-quantize".into(),
        f16.display().to_string(),
        outfile.display().to_string(),
        quant.to_string(),
    ])
}

pub fn resolve_llama_cpp_note(dir: &Path, note: &str) -> Result<LlamaCpp> {
    let convert = dir.join("convert_hf_to_gguf.py");
    let quantize = llama_quantize_bin(dir);
    let mut missing = Vec::new();
    if !convert.is_file() {
        missing.push(format!(
            "llama.cpp convert_hf_to_gguf.py is missing at {}",
            convert.display()
        ));
    }
    if quantize.is_none() {
        missing.push(format!(
            "llama-quantize is missing under {} (expected llama-quantize, build/bin/llama-quantize, or bin/llama-quantize). A PATH binary is not used when this directory is set",
            dir.display()
        ));
    }
    if !missing.is_empty() {
        bail!("refuse:classify-journey: {}; {note}", missing.join("; "));
    }
    Ok(LlamaCpp {
        dir: dir.to_path_buf(),
        convert,
        quantize: quantize.unwrap(),
    })
}

fn llama_quantize_bin(dir: &Path) -> Option<PathBuf> {
    let candidates = [
        dir.join("llama-quantize"),
        dir.join("build").join("bin").join("llama-quantize"),
        dir.join("bin").join("llama-quantize"),
    ];
    candidates.into_iter().find(|candidate| candidate.is_file())
}

#[derive(Clone, Debug)]
pub struct ToolGaps {
    pub missing: Vec<String>,
}

impl ToolGaps {
    pub fn message(&self) -> String {
        format!("refuse:classify-journey: {}", self.missing.join("; "))
    }
}

pub fn base_is_local_dir(base: &str) -> bool {
    Path::new(base).is_dir()
}

/// Directory `convert_hf_to_gguf.py` reads. A hub id downloads into `base-hf`.
pub fn resolved_base_dir(base: &str, paths: &JourneyPaths) -> PathBuf {
    if base_is_local_dir(base) {
        PathBuf::from(base)
    } else {
        paths.base_hf.clone()
    }
}

pub fn hf_download_argv(bin: &str, repo: &str, local_dir: &Path) -> Vec<String> {
    vec![
        bin.to_string(),
        "download".into(),
        repo.to_string(),
        "--local-dir".into(),
        local_dir.display().to_string(),
    ]
}

fn hf_bin_name() -> Option<&'static str> {
    if which("huggingface-cli").is_some() {
        Some("huggingface-cli")
    } else if which("hf").is_some() {
        Some("hf")
    } else {
        None
    }
}

pub fn tool_gaps(
    has: impl Fn(&str) -> bool,
    llama_dir: Option<&Path>,
    gpu_ok: bool,
    need_hf: bool,
    llama_note: &str,
) -> ToolGaps {
    let mut missing = Vec::new();
    if !has("llamafactory-cli") {
        missing.push("llamafactory-cli is not on PATH".into());
    }
    if need_hf && !has("huggingface-cli") && !has("hf") {
        missing.push(
            "huggingface-cli and hf are not on PATH; needed to download the base. HF_TOKEN is read from the environment and is not printed".into(),
        );
    }
    match llama_dir {
        None => missing.push(format!(
            "LLAMA_CPP_DIR is unset; pass --llama-cpp-dir. {llama_note}"
        )),
        Some(dir) => {
            if let Err(err) = resolve_llama_cpp_note(dir, llama_note) {
                let text = err.to_string();
                let text = text
                    .strip_prefix("refuse:classify-journey: ")
                    .unwrap_or(&text);
                missing.push(text.to_string());
            }
        }
    }
    if !has("ollama") {
        missing.push("ollama is not on PATH".into());
    }
    if !gpu_ok {
        missing.push("no GPU (nvidia-smi is missing or failed)".into());
    }
    ToolGaps { missing }
}

#[derive(Clone, Debug)]
struct Inputs {
    prepare: String,
    fetch: String,
    pipeline: Option<String>,
    /// Desired recipe from the CLI. Independent of the recipe file on disk.
    recipe: Option<String>,
    /// Eval skip key: pipeline plus tag, endpoint, and base seat.
    eval: Option<String>,
    ollama_base: Option<String>,
    ollama_spec: Option<String>,
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn sha256_text(text: &str) -> String {
    sha256_bytes(text.as_bytes())
}

fn prepare_inputs(input: &Path, seed: u64, held_out_ratio: f64) -> Result<String> {
    let bytes = fs::read(input).map_err(|e| {
        anyhow::anyhow!(
            "refuse:classify-journey: cannot read {}: {e}",
            input.display()
        )
    })?;
    Ok(sha256_text(&format!(
        "{}\n{seed}\n{held_out_ratio}",
        sha256_bytes(&bytes)
    )))
}

fn fetch_inputs(base: &str) -> String {
    sha256_text(base)
}

fn pipeline_inputs(
    base: &str,
    dataset: &Path,
    recipe: &Path,
    quant: &str,
    train_driver: TrainDriver,
    together_model: &str,
) -> Option<String> {
    let dataset_bytes = fs::read(dataset).ok()?;
    let recipe_bytes = fs::read(recipe).ok()?;
    let driver = train_driver.fingerprint(together_model);
    Some(sha256_text(&format!(
        "{base}\n{}\n{}\n{quant}\n{driver}",
        sha256_bytes(&dataset_bytes),
        sha256_bytes(&recipe_bytes)
    )))
}

/// Recipe skip key from the CLI, not from the recipe file that a previous run left behind.
fn recipe_decision_inputs(
    model_path: &str,
    template_from: &str,
    dataset_name: &str,
    dataset: &Path,
    out: &Path,
    adapter_dir: &Path,
    max_steps: Option<u32>,
) -> Option<String> {
    let dataset_bytes = fs::read(dataset).ok()?;
    let desired = lora_recipe_yaml(
        model_path,
        template_from,
        dataset_name,
        out,
        adapter_dir,
        max_steps,
    );
    Some(sha256_text(&format!(
        "recipe-decision\n{model_path}\n{}\n{dataset_name}\n{}\n{desired}",
        sha256_bytes(&dataset_bytes),
        train_template(template_from),
    )))
}

fn seat_material(
    specialist_tag: &str,
    endpoint: &str,
    library_tag: Option<&str>,
    built_base_tag: &str,
) -> String {
    let mode = if library_tag.is_some() {
        "library-tag"
    } else {
        "pipeline"
    };
    let base_tag = library_tag.unwrap_or(built_base_tag);
    format!("{specialist_tag}\n{endpoint}\n{mode}\n{base_tag}")
}

fn eval_inputs(
    pipeline: &str,
    specialist_tag: &str,
    endpoint: &str,
    library_tag: Option<&str>,
    built_base_tag: &str,
) -> String {
    sha256_text(&format!(
        "eval\n{pipeline}\n{}",
        seat_material(specialist_tag, endpoint, library_tag, built_base_tag)
    ))
}

fn ollama_inputs(
    pipeline: &str,
    tag: &str,
    specialist_tag: &str,
    endpoint: &str,
    library_tag: Option<&str>,
    built_base_tag: &str,
) -> String {
    sha256_text(&format!(
        "ollama\n{pipeline}\n{tag}\n{}",
        seat_material(specialist_tag, endpoint, library_tag, built_base_tag)
    ))
}

fn load_inputs(req: &JourneyRequest<'_>, paths: &JourneyPaths) -> Result<Inputs> {
    let prepare = prepare_inputs(req.input, req.seed, req.held_out_ratio)?;
    let resolved = resolved_base_dir(req.base, paths).display().to_string();
    let pipeline = pipeline_inputs(
        &resolved,
        &paths.dataset_jsonl,
        &paths.recipe,
        req.quant,
        req.train_driver,
        req.together_model,
    );
    let recipe = recipe_decision_inputs(
        &resolved,
        req.base,
        req.dataset_name,
        &paths.dataset_jsonl,
        &paths.out,
        &paths.adapter_dir,
        req.max_steps,
    );
    let library = library_tag(req.base_tag);
    let eval = pipeline
        .as_ref()
        .map(|pipeline| eval_inputs(pipeline, req.tag, req.endpoint, library, req.built_base_tag));
    let ollama_base = pipeline.as_ref().map(|pipeline| {
        ollama_inputs(
            pipeline,
            req.built_base_tag,
            req.tag,
            req.endpoint,
            library,
            req.built_base_tag,
        )
    });
    let ollama_spec = pipeline.as_ref().map(|pipeline| {
        ollama_inputs(
            pipeline,
            req.tag,
            req.tag,
            req.endpoint,
            library,
            req.built_base_tag,
        )
    });
    Ok(Inputs {
        prepare,
        fetch: fetch_inputs(req.base),
        pipeline,
        recipe,
        eval,
        ollama_base,
        ollama_spec,
    })
}

#[derive(Clone, Debug)]
struct Manifest {
    inputs: String,
    gguf_sha256: Option<String>,
}

fn read_manifest(path: &Path) -> Option<Manifest> {
    let text = fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    let inputs = value.get("inputs")?.as_str()?.to_string();
    let gguf_sha256 = value
        .get("gguf_sha256")
        .and_then(Value::as_str)
        .map(str::to_string);
    Some(Manifest {
        inputs,
        gguf_sha256,
    })
}

fn write_manifest(path: &Path, inputs: &str, gguf_sha256: Option<&str>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let report = json!({
        "schema": "cell-one.classify-journey-manifest.v0",
        "inputs": inputs,
        "gguf_sha256": gguf_sha256,
    });
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;
    Ok(())
}

fn file_sha(path: &Path) -> Option<String> {
    fs::read(path).ok().map(|bytes| sha256_bytes(&bytes))
}

struct Decision {
    action: StepAction,
    /// Printed as `redo {step}: {reason}` when the output is stale.
    redo: Option<String>,
}

fn decide_file(output: &Path, manifest_path: &Path, expected: &str) -> Decision {
    if !output.is_file() {
        return Decision {
            action: StepAction::Run,
            redo: None,
        };
    }
    match read_manifest(manifest_path) {
        Some(manifest) if manifest.inputs == expected => Decision {
            action: StepAction::Skip,
            redo: None,
        },
        Some(_) => Decision {
            action: StepAction::Run,
            redo: Some("inputs changed".into()),
        },
        None => Decision {
            action: StepAction::Run,
            redo: Some("missing manifest".into()),
        },
    }
}

fn decide_ollama(
    gguf: &Path,
    manifest_path: &Path,
    expected: &str,
    check_show: bool,
    tag: &str,
) -> Decision {
    let Some(gguf_sha) = file_sha(gguf) else {
        return Decision {
            action: StepAction::Run,
            redo: None,
        };
    };
    match read_manifest(manifest_path) {
        Some(manifest)
            if manifest.inputs == expected
                && manifest.gguf_sha256.as_deref() == Some(gguf_sha.as_str()) =>
        {
            if check_show && !ollama_has_model(tag) {
                Decision {
                    action: StepAction::Run,
                    redo: Some("tag missing".into()),
                }
            } else {
                Decision {
                    action: StepAction::Skip,
                    redo: None,
                }
            }
        }
        Some(manifest)
            if manifest.gguf_sha256.is_some()
                && manifest.gguf_sha256.as_deref() != Some(gguf_sha.as_str()) =>
        {
            Decision {
                action: StepAction::Run,
                redo: Some("GGUF hash changed".into()),
            }
        }
        Some(_) => Decision {
            action: StepAction::Run,
            redo: Some("inputs changed".into()),
        },
        None => Decision {
            action: StepAction::Run,
            redo: Some("missing manifest".into()),
        },
    }
}

#[derive(Clone, Copy)]
struct PlanCtx<'a> {
    paths: &'a JourneyPaths,
    base: &'a str,
    hf_bin: &'a str,
    quant: &'a str,
    library_tag: Option<&'a str>,
    built_base_tag: &'a str,
    specialist_tag: &'a str,
    llama: Option<&'a LlamaCpp>,
    inputs: &'a Inputs,
    check_ollama: bool,
    train_driver: TrainDriver,
    together_model: &'a str,
}

fn step_detail(name: &str, decision: &Decision, command: String) -> StepPlan {
    let detail = match &decision.redo {
        Some(reason) => format!("redo {name}: {reason}; {command}"),
        None => command,
    };
    StepPlan {
        name: match name {
            "prepare" => "prepare",
            "fetch-base" => "fetch-base",
            "recipe" => "recipe",
            "train" => "train",
            "merge-export" => "merge-export",
            "gguf-convert-base" => "gguf-convert-base",
            "gguf-convert-specialist" => "gguf-convert-specialist",
            "quantize-base" => "quantize-base",
            "quantize-specialist" => "quantize-specialist",
            "ollama-create-base" => "ollama-create-base",
            "ollama-create-specialist" => "ollama-create-specialist",
            "eval-base" => "eval-base",
            "eval-specialist" => "eval-specialist",
            "compare" => "compare",
            other => panic!("unknown step {other}"),
        },
        action: decision.action,
        detail,
    }
}

fn convert_line(ctx: &PlanCtx<'_>, src: &str, outfile: &Path) -> String {
    match ctx.llama {
        Some(llama) => argv_line(&llama.convert_argv(src, outfile)),
        None => placeholder_convert_line(src, outfile),
    }
}

fn train_command(ctx: &PlanCtx<'_>) -> String {
    match ctx.train_driver {
        TrainDriver::Local => format!("llamafactory-cli train {}", ctx.paths.recipe.display()),
        TrainDriver::Together => format!(
            "together upload {} model {} lora; poll; GET /finetune/download checkpoint=adapter -> {}",
            ctx.paths.dataset_jsonl.display(),
            ctx.together_model,
            ctx.paths.adapter_dir.display()
        ),
    }
}

fn quantize_line(ctx: &PlanCtx<'_>, f16: &Path, outfile: &Path) -> String {
    match ctx.llama {
        Some(llama) => argv_line(&llama.quantize_argv(f16, outfile, ctx.quant)),
        None => placeholder_quantize_line(f16, outfile, ctx.quant),
    }
}

fn plan_with(ctx: &PlanCtx<'_>) -> Vec<StepPlan> {
    let paths = ctx.paths;
    let mut steps = Vec::new();

    let prepare_ready =
        paths.heldout.is_file() && paths.dataset_jsonl.is_file() && paths.dataset_info.is_file();
    let prepare = if prepare_ready {
        decide_file(
            &paths.dataset_jsonl,
            &paths.manifest_path("prepare"),
            &ctx.inputs.prepare,
        )
    } else {
        Decision {
            action: StepAction::Run,
            redo: None,
        }
    };
    steps.push(step_detail(
        "prepare",
        &prepare,
        format!("classify prepare -> {}", paths.out.display()),
    ));

    let base_dir = resolved_base_dir(ctx.base, paths);
    let base_dir_s = base_dir.display().to_string();
    let fetch_marker = base_dir.join("config.json");
    let fetch = if fetch_marker.is_file() {
        decide_file(
            &fetch_marker,
            &paths.manifest_path("fetch-base"),
            &ctx.inputs.fetch,
        )
    } else {
        Decision {
            action: StepAction::Run,
            redo: None,
        }
    };
    let fetch_cmd = if base_is_local_dir(ctx.base) {
        format!("local base {base_dir_s}")
    } else {
        argv_line(&hf_download_argv(ctx.hf_bin, ctx.base, &base_dir))
    };
    steps.push(step_detail("fetch-base", &fetch, fetch_cmd));

    let recipe = if paths.recipe.is_file() && paths.export_yaml.is_file() {
        if let Some(recipe_key) = ctx.inputs.recipe.as_deref() {
            decide_file(&paths.recipe, &paths.manifest_path("recipe"), recipe_key)
        } else {
            Decision {
                action: StepAction::Run,
                redo: Some("missing manifest".into()),
            }
        }
    } else {
        Decision {
            action: StepAction::Run,
            redo: None,
        }
    };
    steps.push(step_detail(
        "recipe",
        &recipe,
        format!("write {}", paths.recipe.display()),
    ));

    let trained = paths.adapter_dir.join("adapter_config.json");
    let train = match ctx.inputs.pipeline.as_deref() {
        Some(pipeline) if trained.is_file() => {
            decide_file(&trained, &paths.manifest_path("train"), pipeline)
        }
        _ if trained.is_file() => Decision {
            action: StepAction::Run,
            redo: Some("missing manifest".into()),
        },
        _ => Decision {
            action: StepAction::Run,
            redo: None,
        },
    };
    steps.push(step_detail("train", &train, train_command(ctx)));

    let merged = paths.export_dir.join("config.json");
    let merge = match ctx.inputs.pipeline.as_deref() {
        Some(pipeline) if merged.is_file() => {
            decide_file(&merged, &paths.manifest_path("merge-export"), pipeline)
        }
        _ if merged.is_file() => Decision {
            action: StepAction::Run,
            redo: Some("missing manifest".into()),
        },
        _ => Decision {
            action: StepAction::Run,
            redo: None,
        },
    };
    steps.push(step_detail(
        "merge-export",
        &merge,
        format!("llamafactory-cli export {}", paths.export_yaml.display()),
    ));

    let base_convert_cmd = convert_line(ctx, &base_dir_s, &paths.base_f16);
    let base_convert = if ctx.library_tag.is_some() {
        Decision {
            action: StepAction::Skip,
            redo: None,
        }
    } else {
        match ctx.inputs.pipeline.as_deref() {
            Some(pipeline) => decide_file(
                &paths.base_f16,
                &paths.manifest_path("gguf-convert-base"),
                pipeline,
            ),
            None if paths.base_f16.is_file() => Decision {
                action: StepAction::Run,
                redo: Some("missing manifest".into()),
            },
            None => Decision {
                action: StepAction::Run,
                redo: None,
            },
        }
    };
    let base_convert_detail = if ctx.library_tag.is_some() {
        "library tag; precision may differ".to_string()
    } else {
        base_convert_cmd
    };
    steps.push(step_detail(
        "gguf-convert-base",
        &base_convert,
        base_convert_detail,
    ));

    let export_src = paths.export_dir.display().to_string();
    let specialist_convert_cmd = convert_line(ctx, &export_src, &paths.specialist_f16);
    let specialist_convert = match ctx.inputs.pipeline.as_deref() {
        Some(pipeline) => decide_file(
            &paths.specialist_f16,
            &paths.manifest_path("gguf-convert-specialist"),
            pipeline,
        ),
        None if paths.specialist_f16.is_file() => Decision {
            action: StepAction::Run,
            redo: Some("missing manifest".into()),
        },
        None => Decision {
            action: StepAction::Run,
            redo: None,
        },
    };
    steps.push(step_detail(
        "gguf-convert-specialist",
        &specialist_convert,
        specialist_convert_cmd,
    ));

    let base_seated = paths.seated_gguf("base", ctx.quant);
    let specialist_seated = paths.seated_gguf("specialist", ctx.quant);
    let f16_skip = quant_is_f16(ctx.quant);

    let quant_base = if ctx.library_tag.is_some() || f16_skip {
        Decision {
            action: StepAction::Skip,
            redo: None,
        }
    } else {
        match ctx.inputs.pipeline.as_deref() {
            Some(pipeline) => decide_file(
                &base_seated,
                &paths.manifest_path("quantize-base"),
                pipeline,
            ),
            None if base_seated.is_file() => Decision {
                action: StepAction::Run,
                redo: Some("missing manifest".into()),
            },
            None => Decision {
                action: StepAction::Run,
                redo: None,
            },
        }
    };
    let quant_base_detail = if ctx.library_tag.is_some() {
        "library tag; precision may differ".to_string()
    } else if f16_skip {
        "f16 skips llama-quantize".to_string()
    } else {
        quantize_line(ctx, &paths.base_f16, &base_seated)
    };
    steps.push(step_detail("quantize-base", &quant_base, quant_base_detail));

    let quant_specialist = if f16_skip {
        Decision {
            action: StepAction::Skip,
            redo: None,
        }
    } else {
        match ctx.inputs.pipeline.as_deref() {
            Some(pipeline) => decide_file(
                &specialist_seated,
                &paths.manifest_path("quantize-specialist"),
                pipeline,
            ),
            None if specialist_seated.is_file() => Decision {
                action: StepAction::Run,
                redo: Some("missing manifest".into()),
            },
            None => Decision {
                action: StepAction::Run,
                redo: None,
            },
        }
    };
    let quant_specialist_detail = if f16_skip {
        "f16 skips llama-quantize".to_string()
    } else {
        quantize_line(ctx, &paths.specialist_f16, &specialist_seated)
    };
    steps.push(step_detail(
        "quantize-specialist",
        &quant_specialist,
        quant_specialist_detail,
    ));

    let create_base_cmd = format!(
        "ollama create {} -f {}",
        ctx.built_base_tag,
        paths.base_modelfile.display()
    );
    let create_base = if ctx.library_tag.is_some() {
        Decision {
            action: StepAction::Skip,
            redo: None,
        }
    } else {
        match ctx.inputs.ollama_base.as_deref() {
            Some(key) => decide_ollama(
                &base_seated,
                &paths.manifest_path("ollama-create-base"),
                key,
                ctx.check_ollama,
                ctx.built_base_tag,
            ),
            None => Decision {
                action: StepAction::Run,
                redo: None,
            },
        }
    };
    let create_base_detail = if ctx.library_tag.is_some() {
        "library tag; precision may differ".to_string()
    } else {
        create_base_cmd
    };
    steps.push(step_detail(
        "ollama-create-base",
        &create_base,
        create_base_detail,
    ));

    let create_spec_cmd = format!(
        "ollama create {} -f {}",
        ctx.specialist_tag,
        paths.specialist_modelfile.display()
    );
    let create_spec = match ctx.inputs.ollama_spec.as_deref() {
        Some(key) => decide_ollama(
            &specialist_seated,
            &paths.manifest_path("ollama-create-specialist"),
            key,
            ctx.check_ollama,
            ctx.specialist_tag,
        ),
        None => Decision {
            action: StepAction::Run,
            redo: None,
        },
    };
    steps.push(step_detail(
        "ollama-create-specialist",
        &create_spec,
        create_spec_cmd,
    ));

    let eval_base_model = ctx.library_tag.unwrap_or(ctx.built_base_tag);
    let eval_base = match ctx.inputs.eval.as_deref() {
        Some(key) if paths.base_report.is_file() => {
            decide_file(&paths.base_report, &paths.manifest_path("eval-base"), key)
        }
        _ if paths.base_report.is_file() => Decision {
            action: StepAction::Run,
            redo: Some("missing manifest".into()),
        },
        _ => Decision {
            action: StepAction::Run,
            redo: None,
        },
    };
    steps.push(step_detail(
        "eval-base",
        &eval_base,
        format!("classify eval --api ollama-native model {eval_base_model}"),
    ));

    let eval_spec = match ctx.inputs.eval.as_deref() {
        Some(key) if paths.specialist_report.is_file() => decide_file(
            &paths.specialist_report,
            &paths.manifest_path("eval-specialist"),
            key,
        ),
        _ if paths.specialist_report.is_file() => Decision {
            action: StepAction::Run,
            redo: Some("missing manifest".into()),
        },
        _ => Decision {
            action: StepAction::Run,
            redo: None,
        },
    };
    steps.push(step_detail(
        "eval-specialist",
        &eval_spec,
        format!(
            "classify eval --api ollama-native model {}",
            ctx.specialist_tag
        ),
    ));

    steps.push(StepPlan {
        name: "compare",
        action: StepAction::Run,
        detail: format!("write {}", paths.comparison.display()),
    });

    debug_assert_eq!(steps.iter().map(|s| s.name).collect::<Vec<_>>(), STEP_ORDER);
    steps
}

#[derive(Clone, Debug)]
pub struct SideScore {
    pub accuracy: f64,
    pub invalid: u64,
    pub records: u64,
    pub p50: f64,
    pub p95: f64,
    pub thinking_leak: u64,
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
    let thinking_leak = report
        .get("thinking_leak")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    Ok(SideScore {
        accuracy,
        invalid,
        records,
        p50,
        p95,
        thinking_leak,
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
    pub base_thinking_leak: u64,
    pub specialist_thinking_leak: u64,
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
        base_thinking_leak: base.thinking_leak,
        specialist_thinking_leak: specialist.thinking_leak,
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
    pub base_tag: Option<&'a str>,
    pub tag: &'a str,
    pub endpoint: &'a str,
    pub dataset_name: &'a str,
    pub seed: u64,
    pub held_out_ratio: f64,
    pub max_steps: Option<u32>,
    pub quant: &'a str,
    pub llama_cpp_dir: Option<&'a Path>,
    pub force: bool,
    pub print: bool,
    pub run: bool,
    pub min_delta: Option<f64>,
    pub min_accuracy: Option<f64>,
    pub timeout_secs: u64,
    pub together_poll_secs: u64,
    pub train_driver: TrainDriver,
    pub together_model: &'a str,
    pub together_base_url: &'a str,
    pub api_key_env: Option<&'a str>,
    pub built_base_tag: &'a str,
    pub seat: SeatChat,
    pub llama_note: &'a str,
    pub preset: JourneyPreset,
}

fn library_tag(base_tag: Option<&str>) -> Option<&str> {
    base_tag.map(str::trim).filter(|tag| !tag.is_empty())
}

pub fn cmd_classify_journey(req: &JourneyRequest<'_>) -> Result<()> {
    if req.print && req.run {
        bail!("refuse:classify-journey: pass only one of --print and --run");
    }
    if req.base.trim().is_empty() {
        bail!("refuse:classify-journey: --base is empty");
    }
    if req.train_driver == TrainDriver::Together && req.together_model.trim().is_empty() {
        bail!("refuse:classify-journey: --together-model is empty");
    }
    if req.train_driver == TrainDriver::Together && req.together_base_url.trim().is_empty() {
        bail!("refuse:classify-journey: --together-base-url is empty");
    }
    if req.tag.trim().is_empty() {
        bail!("refuse:classify-journey: model tag is empty");
    }
    if req.quant.trim().is_empty()
        || req
            .quant
            .chars()
            .any(|c| c.is_whitespace() || c == '/' || c == '\\')
    {
        bail!("refuse:classify-journey: --quant is empty or not a llama-quantize type");
    }
    let paths = JourneyPaths::new(req.out);
    let template = train_template(req.base);
    let inputs = load_inputs(req, &paths)?;
    let llama = match req.llama_cpp_dir {
        Some(dir) if req.run => Some(resolve_llama_cpp_note(dir, req.llama_note)?),
        Some(dir) => resolve_llama_cpp_note(dir, req.llama_note).ok(),
        None => None,
    };
    let library = library_tag(req.base_tag);
    let ctx = PlanCtx {
        paths: &paths,
        base: req.base,
        hf_bin: hf_bin_name().unwrap_or("huggingface-cli"),
        quant: req.quant,
        library_tag: library,
        built_base_tag: req.built_base_tag,
        specialist_tag: req.tag,
        llama: llama.as_ref(),
        inputs: &inputs,
        check_ollama: false,
        train_driver: req.train_driver,
        together_model: req.together_model,
    };
    if !req.run {
        let steps = plan_with(&ctx);
        print_plan(req, &paths, template, library, &steps);
        return Ok(());
    }
    let gaps = tool_gaps(
        tool_on_path,
        req.llama_cpp_dir,
        gpu_present(),
        !base_is_local_dir(req.base),
        req.llama_note,
    );
    if !gaps.missing.is_empty() {
        bail!(gaps.message());
    }
    let llama = llama.ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-journey: LLAMA_CPP_DIR is unset; pass --llama-cpp-dir. {}",
            req.llama_note
        )
    })?;
    execute(req, &paths, &llama)?;
    let base_report = read_json(&paths.base_report)?;
    let specialist_report = read_json(&paths.specialist_report)?;
    let comparison = compare_sides(&side_score(&base_report)?, &side_score(&specialist_report)?);
    let verdict = threshold_met(&comparison, req.min_delta, req.min_accuracy);
    write_comparison(req, &paths, template, library, &comparison, verdict)?;
    match verdict {
        Some(false) => bail!(
            "classify-journey: threshold missed; specialist accuracy {:.4} delta {:.4}",
            comparison.specialist_accuracy,
            comparison.delta
        ),
        Some(true) => println!("classify-journey: threshold met"),
        None => {}
    }
    Ok(())
}

fn print_plan(
    req: &JourneyRequest<'_>,
    paths: &JourneyPaths,
    template: &str,
    library: Option<&str>,
    steps: &[StepPlan],
) {
    println!("classify journey: print");
    println!("base: {} template: {template}", req.base);
    let base_seat = library.unwrap_or(req.built_base_tag);
    let seat_kind = if library.is_some() {
        "library-tag"
    } else {
        "pipeline"
    };
    println!(
        "base_tag: {base_seat} base_seat: {seat_kind} specialist_tag: {} quant: {} endpoint: {} api: ollama-native",
        req.tag, req.quant, req.endpoint
    );
    if library.is_some() {
        println!("warning: --base-tag skips the shared convert and quant. Precision may differ from the specialist.");
    }
    if req.preset == JourneyPreset::Tev1 && req.base == DEFAULT_BASE {
        println!(
            "default base {DEFAULT_BASE} is registered in LLaMA-Factory constants.py as Qwen3.5-4B-Thinking with template {QWEN35_TEMPLATE} and enable_thinking false"
        );
    }
    if req.preset == JourneyPreset::DeepseekR1Distill {
        println!(
            "preset deepseek-r1-distill base {DEEPSEEK_R1_DISTILL_BASE} template {template} tag {} local llamafactory-cli train",
            req.tag
        );
    }
    if req.llama_cpp_dir.is_none() {
        println!(
            "{}. Set --llama-cpp-dir or LLAMA_CPP_DIR before --run.",
            req.llama_note
        );
    }
    println!(
        "train_driver: {} {}",
        req.train_driver.as_str(),
        match req.train_driver {
            TrainDriver::Local => "llamafactory-cli train".to_string(),
            TrainDriver::Together => format!(
                "model {} api-key-env {} poll {}s dry-run no network",
                req.together_model,
                req.api_key_env.unwrap_or(DEFAULT_TOGETHER_KEY_ENV),
                req.together_poll_secs
            ),
        }
    );
    println!("out: {}", paths.out.display());
    for step in steps {
        let word = match step.action {
            StepAction::Run => "run",
            StepAction::Skip => "skip",
        };
        println!("{word} {:<24} {}", step.name, step.detail);
    }
    println!("compare writes {}", paths.comparison.display());
    println!("This print does not train, convert, or seat. A later report is local output. It does not record a live PASS. READY_FOR_LIVE_TEST: no.");
}

fn execute(req: &JourneyRequest<'_>, paths: &JourneyPaths, llama: &LlamaCpp) -> Result<()> {
    let library = library_tag(req.base_tag);
    // Re-read inputs around each step so a rewritten dataset or YAML changes the fingerprint.
    let mut ran_prepare = false;
    for name in STEP_ORDER {
        if *name == "compare" {
            continue;
        }
        let inputs = load_inputs(req, paths)?;
        let ctx = PlanCtx {
            paths,
            base: req.base,
            hf_bin: hf_bin_name().unwrap_or("huggingface-cli"),
            quant: req.quant,
            library_tag: library,
            built_base_tag: req.built_base_tag,
            specialist_tag: req.tag,
            llama: Some(llama),
            inputs: &inputs,
            check_ollama: name.starts_with("ollama-create"),
            train_driver: req.train_driver,
            together_model: req.together_model,
        };
        let steps = plan_with(&ctx);
        let step = steps.iter().find(|step| step.name == *name).expect("step");
        if step.action == StepAction::Skip {
            if *name == "fetch-base" {
                let dir = resolved_base_dir(req.base, paths);
                validate_snapshot(&dir)?;
            }
            println!("skip {}", step.name);
            continue;
        }
        if let Some(reason) = redo_reason(&step.detail) {
            println!("redo {}: {reason}", step.name);
        } else {
            println!("run {}", step.name);
        }
        match step.name {
            "prepare" => {
                cmd_classify_prepare(
                    req.input,
                    &paths.out,
                    req.seed,
                    req.held_out_ratio,
                    DatasetFormat::Sharegpt,
                    req.dataset_name,
                    false,
                    req.force || ran_prepare || paths.dataset_jsonl.is_file(),
                )?;
                let prepare = prepare_inputs(req.input, req.seed, req.held_out_ratio)?;
                write_manifest(&paths.manifest_path("prepare"), &prepare, None)?;
                ran_prepare = true;
            }
            "fetch-base" => {
                let dir = resolved_base_dir(req.base, paths);
                if base_is_local_dir(req.base) {
                    println!("local base {}", dir.display());
                } else {
                    let bin = hf_bin_name().ok_or_else(|| {
                        anyhow::anyhow!(
                            "refuse:classify-journey: huggingface-cli and hf are not on PATH; needed to download the base. HF_TOKEN is read from the environment and is not printed"
                        )
                    })?;
                    run_argv(&hf_download_argv(bin, req.base, &dir))?;
                }
                validate_snapshot(&dir)?;
                write_manifest(
                    &paths.manifest_path("fetch-base"),
                    &fetch_inputs(req.base),
                    None,
                )?;
            }
            "recipe" => {
                write_recipe(req, paths)?;
                write_step_manifest(req, paths, "recipe", None)?;
            }
            "train" => {
                match req.train_driver {
                    TrainDriver::Local => {
                        run_argv(&[
                            "llamafactory-cli".into(),
                            "train".into(),
                            paths.recipe.display().to_string(),
                        ])?;
                    }
                    TrainDriver::Together => run_together_train(req, paths)?,
                }
                validate_adapter(&paths.adapter_dir)?;
                write_pipeline_manifest(req, paths, "train")?;
            }
            "merge-export" => {
                run_argv(&[
                    "llamafactory-cli".into(),
                    "export".into(),
                    paths.export_yaml.display().to_string(),
                ])?;
                validate_export(&paths.export_dir)?;
                write_pipeline_manifest(req, paths, "merge-export")?;
            }
            "gguf-convert-base" => {
                let src = resolved_base_dir(req.base, paths).display().to_string();
                let argv = llama.convert_argv(&src, &paths.base_f16);
                run_argv(&argv)?;
                validate_gguf(&paths.base_f16)?;
                write_pipeline_manifest(req, paths, "gguf-convert-base")?;
            }
            "gguf-convert-specialist" => {
                let src = paths.export_dir.display().to_string();
                let argv = llama.convert_argv(&src, &paths.specialist_f16);
                run_argv(&argv)?;
                validate_gguf(&paths.specialist_f16)?;
                write_pipeline_manifest(req, paths, "gguf-convert-specialist")?;
            }
            "quantize-base" => {
                let seated = paths.seated_gguf("base", req.quant);
                let argv = llama.quantize_argv(&paths.base_f16, &seated, req.quant);
                run_argv(&argv)?;
                validate_gguf(&seated)?;
                write_pipeline_manifest(req, paths, "quantize-base")?;
            }
            "quantize-specialist" => {
                let seated = paths.seated_gguf("specialist", req.quant);
                let argv = llama.quantize_argv(&paths.specialist_f16, &seated, req.quant);
                run_argv(&argv)?;
                validate_gguf(&seated)?;
                write_pipeline_manifest(req, paths, "quantize-specialist")?;
            }
            "ollama-create-base" => {
                seat_gguf(
                    req,
                    paths,
                    "base",
                    req.built_base_tag,
                    &paths.base_modelfile,
                    "ollama-create-base",
                )?;
            }
            "ollama-create-specialist" => {
                seat_gguf(
                    req,
                    paths,
                    "specialist",
                    req.tag,
                    &paths.specialist_modelfile,
                    "ollama-create-specialist",
                )?;
            }
            "eval-base" => {
                let model = library.unwrap_or(req.built_base_tag);
                cmd_classify_eval(
                    &paths.heldout,
                    Some(req.endpoint),
                    model,
                    None,
                    &paths.base_report,
                    false,
                    false,
                    req.timeout_secs,
                    EvalApi::OllamaNative,
                    EvalGate::Journey,
                )?;
                write_pipeline_manifest(req, paths, "eval-base")?;
            }
            "eval-specialist" => {
                cmd_classify_eval(
                    &paths.heldout,
                    Some(req.endpoint),
                    req.tag,
                    None,
                    &paths.specialist_report,
                    false,
                    false,
                    req.timeout_secs,
                    EvalApi::OllamaNative,
                    EvalGate::Journey,
                )?;
                write_pipeline_manifest(req, paths, "eval-specialist")?;
            }
            other => bail!("refuse:classify-journey: unknown step {other}"),
        }
    }
    Ok(())
}

fn redo_reason(detail: &str) -> Option<&str> {
    let rest = detail.strip_prefix("redo ")?;
    let (_, reason) = rest.split_once(": ")?;
    Some(reason.split(';').next().unwrap_or(reason).trim())
}

fn step_manifest_key(inputs: &Inputs, step: &str) -> Result<String> {
    let key = match step {
        "recipe" => inputs.recipe.as_deref(),
        "eval-base" | "eval-specialist" => inputs.eval.as_deref(),
        "ollama-create-base" => inputs.ollama_base.as_deref(),
        "ollama-create-specialist" => inputs.ollama_spec.as_deref(),
        _ => inputs.pipeline.as_deref(),
    };
    key.map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-journey: cannot hash journey inputs"))
}

fn write_step_manifest(
    req: &JourneyRequest<'_>,
    paths: &JourneyPaths,
    step: &str,
    gguf_sha256: Option<&str>,
) -> Result<()> {
    let inputs = load_inputs(req, paths)?;
    let key = step_manifest_key(&inputs, step)?;
    write_manifest(&paths.manifest_path(step), &key, gguf_sha256)
}

fn write_pipeline_manifest(
    req: &JourneyRequest<'_>,
    paths: &JourneyPaths,
    step: &str,
) -> Result<()> {
    write_step_manifest(req, paths, step, None)
}

fn seat_gguf(
    req: &JourneyRequest<'_>,
    paths: &JourneyPaths,
    which: &str,
    tag: &str,
    modelfile: &Path,
    step: &str,
) -> Result<()> {
    let gguf = paths.seated_gguf(which, req.quant);
    if !gguf.is_file() {
        bail!(
            "refuse:classify-journey: GGUF missing at {}",
            gguf.display()
        );
    }
    let gguf_sha = file_sha(&gguf).ok_or_else(|| {
        anyhow::anyhow!("refuse:classify-journey: cannot hash {}", gguf.display())
    })?;
    let prior = read_manifest(&paths.manifest_path(step));
    if prior
        .as_ref()
        .and_then(|manifest| manifest.gguf_sha256.as_deref())
        != Some(gguf_sha.as_str())
        && ollama_has_model(tag)
    {
        run_argv(&["ollama".into(), "rm".into(), tag.into()])?;
    }
    let gguf_abs = fs::canonicalize(&gguf).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-journey: cannot canonicalize {}: {err}",
            gguf.display()
        )
    })?;
    fs::write(modelfile, journey_modelfile(&gguf_abs, req.seat))?;
    run_argv(&[
        "ollama".into(),
        "create".into(),
        tag.into(),
        "-f".into(),
        modelfile.display().to_string(),
    ])?;
    if !ollama_has_model(tag) {
        bail!("refuse:classify-journey: ollama show {tag} failed after create");
    }
    write_step_manifest(req, paths, step, Some(&gguf_sha))?;
    Ok(())
}

fn snapshot_tokenizer(dir: &Path) -> bool {
    ["tokenizer.json", "tokenizer.model", "tokenizer_config.json"]
        .iter()
        .any(|name| dir.join(name).is_file())
}

fn snapshot_has_weights(dir: &Path) -> bool {
    let index = dir.join("model.safetensors.index.json");
    if index.is_file() {
        let Ok(text) = fs::read_to_string(&index) else {
            return false;
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            return false;
        };
        let Some(map) = value.get("weight_map").and_then(Value::as_object) else {
            return false;
        };
        let mut shards: Vec<&str> = map.values().filter_map(Value::as_str).collect();
        shards.sort_unstable();
        shards.dedup();
        return !shards.is_empty() && shards.iter().all(|name| file_nonempty(&dir.join(name)));
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(|entry| entry.ok()).any(|entry| {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        name.ends_with(".safetensors") && file_nonempty(&entry.path())
    })
}

fn validate_snapshot(dir: &Path) -> Result<()> {
    if !dir.join("config.json").is_file() || !snapshot_tokenizer(dir) || !snapshot_has_weights(dir)
    {
        bail!(
            "refuse:classify-journey: base snapshot {} needs config.json, tokenizer files, and non-empty weights",
            dir.display()
        );
    }
    Ok(())
}

fn file_nonempty(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.is_file() && meta.len() > 0)
        .unwrap_or(false)
}

fn dir_has_weights(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(|entry| entry.ok()).any(|entry| {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        (name.ends_with(".safetensors") || name.ends_with(".bin")) && file_nonempty(&entry.path())
    })
}

fn validate_adapter(dir: &Path) -> Result<()> {
    if !dir.join("adapter_config.json").is_file() || !dir_has_weights(dir) {
        bail!(
            "refuse:classify-journey: adapter {} needs adapter_config.json and weights",
            dir.display()
        );
    }
    Ok(())
}

fn validate_export(dir: &Path) -> Result<()> {
    let tokenizer = ["tokenizer.json", "tokenizer.model", "tokenizer_config.json"]
        .iter()
        .any(|name| dir.join(name).is_file());
    if !dir.join("config.json").is_file() || !tokenizer || !dir_has_weights(dir) {
        bail!(
            "refuse:classify-journey: export {} needs config.json, tokenizer files, and weights",
            dir.display()
        );
    }
    Ok(())
}

fn validate_gguf(path: &Path) -> Result<()> {
    let bytes = fs::read(path).unwrap_or_default();
    if bytes.len() < 4 || &bytes[..4] != b"GGUF" {
        bail!(
            "refuse:classify-journey: {} is not a non-empty GGUF",
            path.display()
        );
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
    let model = resolved_base_dir(req.base, paths).display().to_string();
    let recipe = lora_recipe_yaml(
        &model,
        req.base,
        req.dataset_name,
        &paths.out,
        &paths.adapter_dir,
        req.max_steps,
    );
    let export = export_yaml(&model, req.base, &paths.adapter_dir, &paths.export_dir);
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
    library: Option<&str>,
    comparison: &Comparison,
    verdict: Option<bool>,
) -> Result<()> {
    let verdict_text = match verdict {
        Some(true) => "met",
        Some(false) => "missed",
        None => "unset",
    };
    let base_seat = if library.is_some() {
        "library-tag"
    } else {
        "pipeline"
    };
    let precision_warning = library
        .map(|_| "base-tag is a library tag. Its precision may differ from the specialist quant.");
    let report = json!({
        "schema": "cell-one.classify-journey.v0",
        "base": req.base,
        "template": template,
        "base_tag": library.unwrap_or(req.built_base_tag),
        "preset": match req.preset {
            JourneyPreset::Tev1 => "tev1",
            JourneyPreset::DeepseekR1Distill => "deepseek-r1-distill",
        },
        "base_seat": base_seat,
        "specialist_tag": req.tag,
        "quant": req.quant,
        "endpoint": req.endpoint,
        "api": EvalApi::OllamaNative.as_str(),
        "precision_warning": precision_warning,
        "base_accuracy": comparison.base_accuracy,
        "specialist_accuracy": comparison.specialist_accuracy,
        "delta": comparison.delta,
        "base_invalid_rate": comparison.base_invalid_rate,
        "specialist_invalid_rate": comparison.specialist_invalid_rate,
        "thinking_leak": {
            "base": comparison.base_thinking_leak,
            "specialist": comparison.specialist_thinking_leak
        },
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
        anyhow::anyhow!(
            "refuse:classify-journey: cannot read {}: {e}",
            path.display()
        )
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

fn together_key_env<'a>(req: &'a JourneyRequest<'_>) -> &'a str {
    req.api_key_env
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(DEFAULT_TOGETHER_KEY_ENV)
}

fn read_together_key(name: &str) -> Result<String> {
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        bail!("refuse:classify-journey: api-key-env must be an environment variable name");
    }
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => Ok(value),
        _ => bail!("refuse:classify-journey: set {name}"),
    }
}

fn scrub_secret(text: &str, secret: &str) -> String {
    if secret.is_empty() {
        text.to_string()
    } else {
        text.replace(secret, "[redacted]")
    }
}

fn together_root(base: &str) -> String {
    base.trim().trim_end_matches('/').to_string()
}

fn run_together_train(req: &JourneyRequest<'_>, paths: &JourneyPaths) -> Result<()> {
    let env_name = together_key_env(req);
    let key = read_together_key(env_name)?;
    let root = together_root(req.together_base_url);
    let dataset = fs::read(&paths.dataset_jsonl).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-journey: cannot read {}: {err}",
            paths.dataset_jsonl.display()
        )
    })?;
    println!(
        "together upload {} model {} (key env {env_name}; value not printed)",
        paths.dataset_jsonl.display(),
        req.together_model
    );
    let file_id = together_upload(&root, &key, &dataset, req.timeout_secs)?;
    together_wait_file(&root, &key, &file_id, req.timeout_secs)?;
    let job_id = together_create_job(&root, &key, &file_id, req.together_model, req.timeout_secs)?;
    println!("together job {job_id}");
    let status = together_poll_job(
        &root,
        &key,
        &job_id,
        req.timeout_secs,
        req.together_poll_secs,
    )?;
    if status != "completed" {
        bail!("refuse:classify-journey: together job {job_id} ended {status}");
    }
    let bytes = together_download_adapter(&root, &key, &job_id, req.timeout_secs)?;
    let tar = decompress_adapter_archive(&bytes)?;
    unpack_adapter_tar(&tar, &paths.adapter_dir)?;
    let note = json!({
        "schema": "cell-one.classify-journey-together.v0",
        "job_id": job_id,
        "file_id": file_id,
        "model": req.together_model,
        "status": status,
        "checkpoint": "adapter",
        "live_pass_recorded": false,
    });
    fs::write(
        paths.out.join("together-job.json"),
        format!("{}\n", serde_json::to_string_pretty(&note)?),
    )?;
    Ok(())
}

fn together_agent(timeout_secs: u64) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(timeout_secs.max(1)))
        .build()
}

fn together_json(
    method: &str,
    url: &str,
    key: &str,
    body: Option<&Value>,
    timeout_secs: u64,
) -> Result<Value> {
    let agent = together_agent(timeout_secs);
    let send = |req: ureq::Request| -> Result<ureq::Response> {
        let req = req
            .set("Authorization", &format!("Bearer {key}"))
            .set("Accept", "application/json");
        match body {
            Some(value) => req.send_json(value.clone()),
            None => req.call(),
        }
        .map_err(|err| together_http_err(url, key, err))
    };
    let resp = match method {
        "POST" => send(agent.post(url))?,
        "GET" => send(agent.get(url))?,
        other => bail!("refuse:classify-journey: together method {other}"),
    };
    let raw = resp.into_string().unwrap_or_default();
    serde_json::from_str(&raw).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-journey: together response is not json ({err}): {}",
            scrub_secret(&raw.chars().take(180).collect::<String>(), key)
        )
    })
}

fn together_http_err(url: &str, key: &str, err: ureq::Error) -> anyhow::Error {
    let (status, body) = match err {
        ureq::Error::Status(code, resp) => (code, resp.into_string().unwrap_or_default()),
        other => (0, other.to_string()),
    };
    let snippet: String = body.chars().take(180).collect();
    anyhow::anyhow!(
        "refuse:classify-journey: together {url} status {status}: {}",
        scrub_secret(&snippet, key)
    )
}

fn together_upload(root: &str, key: &str, dataset: &[u8], timeout_secs: u64) -> Result<String> {
    let boundary = "----cell-one-together";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"purpose\"\r\n\r\nfine-tune\r\n",
    );
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"dataset.jsonl\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: application/jsonl\r\n\r\n");
    body.extend_from_slice(dataset);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let url = format!("{root}/files");
    let agent = together_agent(timeout_secs);
    let resp = agent
        .post(&url)
        .set("Authorization", &format!("Bearer {key}"))
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={boundary}"),
        )
        .send_bytes(&body)
        .map_err(|err| together_http_err(&url, key, err))?;
    let raw = resp.into_string().unwrap_or_default();
    let value: Value = serde_json::from_str(&raw).map_err(|_| {
        anyhow::anyhow!(
            "refuse:classify-journey: together upload response is not json: {}",
            scrub_secret(&raw.chars().take(180).collect::<String>(), key)
        )
    })?;
    value
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            anyhow::anyhow!("refuse:classify-journey: together upload returned no file id")
        })
}

fn together_wait_file(root: &str, key: &str, file_id: &str, timeout_secs: u64) -> Result<()> {
    let url = format!("{root}/files/{file_id}");
    for _ in 0..150 {
        let value = together_json("GET", &url, key, None, timeout_secs)?;
        match value.get("processing_status").and_then(Value::as_str) {
            None => return Ok(()),
            Some("COMPLETED") => return Ok(()),
            Some(status) if status.eq_ignore_ascii_case("invalid_format") || status == "FAILED" => {
                bail!("refuse:classify-journey: together file {file_id} {status}");
            }
            Some(_) => std::thread::sleep(std::time::Duration::from_millis(200)),
        }
    }
    bail!("refuse:classify-journey: together file {file_id} did not finish processing")
}

fn together_create_job(
    root: &str,
    key: &str,
    file_id: &str,
    model: &str,
    timeout_secs: u64,
) -> Result<String> {
    let url = format!("{root}/fine-tunes");
    let body = json!({
        "training_file": file_id,
        "model": model,
        "training_type": {"type": "Lora", "lora_r": 8, "lora_alpha": 16},
        "n_epochs": 1,
        "n_checkpoints": 1,
        "learning_rate": 0.0001,
        "suffix": "tev1"
    });
    let value = together_json("POST", &url, key, Some(&body), timeout_secs)?;
    value
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            anyhow::anyhow!("refuse:classify-journey: together create returned no job id")
        })
}

fn together_poll_job(
    root: &str,
    key: &str,
    job_id: &str,
    timeout_secs: u64,
    poll_secs: u64,
) -> Result<String> {
    let url = format!("{root}/fine-tunes/{job_id}");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(poll_secs);
    loop {
        if std::time::Instant::now() >= deadline {
            bail!(
                "refuse:classify-journey: together job {job_id} exceeded poll deadline of {poll_secs}s"
            );
        }
        let value = together_json("GET", &url, key, None, timeout_secs)?;
        let status = value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        match status.as_str() {
            "completed" | "error" | "cancelled" | "user_error" => return Ok(status),
            "pending" | "queued" | "running" | "compressing" | "uploading" | "cancel_requested" => {
                let left = deadline.saturating_duration_since(std::time::Instant::now());
                if left.is_zero() {
                    bail!(
                        "refuse:classify-journey: together job {job_id} exceeded poll deadline of {poll_secs}s"
                    );
                }
                std::thread::sleep(left.min(std::time::Duration::from_millis(200)));
            }
            "" => bail!("refuse:classify-journey: together job {job_id} has no status"),
            other => bail!("refuse:classify-journey: together job {job_id} status {other}"),
        }
    }
}

fn together_download_adapter(
    root: &str,
    key: &str,
    job_id: &str,
    timeout_secs: u64,
) -> Result<Vec<u8>> {
    let url = format!("{root}/finetune/download?ft_id={job_id}&checkpoint=adapter");
    let agent = together_agent(timeout_secs.max(30));
    let resp = agent
        .get(&url)
        .set("Authorization", &format!("Bearer {key}"))
        .call()
        .map_err(|err| together_http_err(&url, key, err))?;
    read_limited(
        resp.into_reader(),
        MAX_TOGETHER_ADAPTER_COMPRESSED,
        "download",
    )
}

fn read_limited(mut reader: impl Read, cap: usize, what: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(|err| {
            anyhow::anyhow!("refuse:classify-journey: together adapter {what} read failed: {err}")
        })?;
        if n == 0 {
            break;
        }
        if bytes.len().saturating_add(n) > cap {
            bail!("refuse:classify-journey: together adapter {what} exceeds {cap} bytes");
        }
        bytes.extend_from_slice(&buf[..n]);
    }
    Ok(bytes)
}

fn decompress_adapter_archive(bytes: &[u8]) -> Result<Vec<u8>> {
    decompress_adapter_archive_capped(
        bytes,
        MAX_TOGETHER_ADAPTER_COMPRESSED,
        MAX_TOGETHER_ADAPTER_DECOMPRESSED,
    )
}

fn decompress_adapter_archive_capped(
    bytes: &[u8],
    compressed_cap: usize,
    decompressed_cap: usize,
) -> Result<Vec<u8>> {
    if bytes.len() > compressed_cap {
        bail!("refuse:classify-journey: together adapter download exceeds {compressed_cap} bytes");
    }
    if is_zstd(bytes) {
        let decoder = zstd::stream::Decoder::new(bytes).map_err(|err| {
            anyhow::anyhow!("refuse:classify-journey: together download zstd decode failed: {err}")
        })?;
        return read_limited(decoder, decompressed_cap, "decompressed");
    }
    if is_gzip(bytes) {
        return read_limited(
            flate2::read::GzDecoder::new(bytes),
            decompressed_cap,
            "decompressed",
        );
    }
    if is_unknown_codec(bytes) {
        bail!("refuse:classify-journey: together download uses an unknown compression codec");
    }
    Ok(bytes.to_vec())
}

fn is_zstd(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0] == 0x28 && bytes[1] == 0xB5 && bytes[2] == 0x2F && bytes[3] == 0xFD
}

fn is_gzip(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == 0x1F && bytes[1] == 0x8B
}

fn is_unknown_codec(bytes: &[u8]) -> bool {
    let xz = bytes.len() >= 6 && bytes.starts_with(&[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00]);
    let bzip2 = bytes.len() >= 3 && bytes.starts_with(b"BZh");
    let zip = bytes.len() >= 4 && bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]);
    let lz4 = bytes.len() >= 4 && bytes.starts_with(&[0x04, 0x22, 0x4D, 0x18]);
    xz || bzip2 || zip || lz4
}

fn unpack_adapter_tar(bytes: &[u8], dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    if bytes.len() < 512 {
        bail!("refuse:classify-journey: together adapter download is not a tar");
    }
    let mut offset = 0;
    let mut wrote = 0;
    while offset + 512 <= bytes.len() {
        let header = &bytes[offset..offset + 512];
        if header.iter().all(|b| *b == 0) {
            break;
        }
        let magic = &header[257..262];
        if magic != b"ustar" {
            bail!("refuse:classify-journey: together adapter download is not a ustar archive");
        }
        let name = tar_name(header)?;
        let size = tar_size(&header[124..136])?;
        let typeflag = header[156];
        offset += 512;
        let data_end = offset + size;
        if data_end > bytes.len() {
            bail!("refuse:classify-journey: together adapter tar is truncated");
        }
        let data = &bytes[offset..data_end];
        offset = data_end + ((512 - (size % 512)) % 512);
        if name.is_empty() || tar_type_skipped(typeflag) {
            continue;
        }
        if typeflag == b'1' || typeflag == b'2' {
            bail!(
                "refuse:classify-journey: together adapter tar refuses hardlink or symlink entries"
            );
        }
        if typeflag != b'0' && typeflag != 0 {
            bail!(
                "refuse:classify-journey: together adapter tar type {} is not a regular file",
                typeflag as char
            );
        }
        if name.contains("..") || name.starts_with('/') || name.contains('\\') {
            bail!("refuse:classify-journey: together adapter tar path is refused");
        }
        let file_name = Path::new(&name).file_name().ok_or_else(|| {
            anyhow::anyhow!("refuse:classify-journey: together adapter tar path is refused")
        })?;
        fs::write(dir.join(file_name), data)?;
        wrote += 1;
    }
    if wrote == 0 {
        bail!("refuse:classify-journey: together adapter tar has no files");
    }
    Ok(())
}

fn tar_type_skipped(typeflag: u8) -> bool {
    matches!(typeflag, b'5' | b'L' | b'K' | b'g' | b'x' | b'X')
}

fn tar_name(header: &[u8]) -> Result<String> {
    let short = tar_str(&header[0..100]);
    let prefix = tar_str(&header[345..500]);
    if prefix.is_empty() {
        Ok(short)
    } else {
        Ok(format!("{prefix}/{short}"))
    }
}

fn tar_str(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

fn tar_size(bytes: &[u8]) -> Result<usize> {
    let text = tar_str(bytes);
    if text.is_empty() {
        return Ok(0);
    }
    usize::from_str_radix(text.trim(), 8).map_err(|_| {
        anyhow::anyhow!("refuse:classify-journey: together adapter tar size is not octal")
    })
}

fn run_argv(argv: &[String]) -> Result<()> {
    let line = argv_line(argv);
    println!("{line}");
    let (bin, args) = argv
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-journey: empty command"))?;
    let bin_path = if bin.contains('/') || bin.contains('\\') {
        PathBuf::from(bin)
    } else {
        which(bin)
            .ok_or_else(|| anyhow::anyhow!("refuse:classify-journey: {bin} is not on PATH"))?
    };
    let status = Command::new(&bin_path).args(args).status().map_err(|e| {
        anyhow::anyhow!(
            "refuse:classify-journey: cannot run {}: {e}",
            bin_path.display()
        )
    })?;
    if !status.success() {
        bail!(
            "refuse:classify-journey: {} exited {status}",
            bin_path.display()
        );
    }
    let _ = line;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qwen35_template_comes_from_llamafactory_and_other_bases_use_the_scanner() {
        assert_eq!(train_template(DEFAULT_BASE), "qwen3_5");
        assert_eq!(train_template("Qwen/Qwen3.5-4B-Base"), "qwen3_5");
        assert_eq!(
            train_template("Qwen/Qwen3-4B-Instruct-2507"),
            "qwen3_nothink"
        );
        assert_eq!(train_template("Qwen/Qwen3-4B"), "qwen3");
        assert_eq!(train_template("Qwen/Qwen2.5-3B-Instruct"), "qwen");
    }

    #[test]
    fn recipe_disables_thinking_and_modelfile_is_nonthinking() {
        let dir = std::env::temp_dir().join(format!("journey-yaml-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let paths = JourneyPaths::new(&dir);
        let recipe = lora_recipe_yaml(
            DEFAULT_BASE,
            DEFAULT_BASE,
            DEFAULT_DATASET,
            &dir,
            &paths.adapter_dir,
            Some(2),
        );
        assert!(recipe.contains("template: qwen3_5"), "{recipe}");
        assert!(recipe.contains("enable_thinking: false"), "{recipe}");
        assert!(!recipe.contains("qwen3_5_nothink:"), "{recipe}");
        assert!(recipe.contains("finetuning_type: lora"), "{recipe}");
        assert!(recipe.contains("max_steps: 2"), "{recipe}");
        assert_eq!(
            recipe_dataset_name(&recipe).as_deref(),
            Some(DEFAULT_DATASET)
        );
        let info = json!({
            DEFAULT_DATASET: {
                "file_name": "dataset.jsonl",
                "formatting": "sharegpt"
            }
        });
        assert_eq!(
            dataset_name_in_info(&info).as_deref(),
            Some(DEFAULT_DATASET)
        );
        let export = export_yaml(
            DEFAULT_BASE,
            DEFAULT_BASE,
            &paths.adapter_dir,
            &paths.export_dir,
        );
        assert!(export.contains("enable_thinking: false"), "{export}");
        assert!(export.contains("template: qwen3_5"), "{export}");
        let model = journey_modelfile(
            &paths.seated_gguf("specialist", DEFAULT_QUANT),
            SeatChat::Qwen35,
        );
        assert!(model.contains("PARAMETER temperature 0"), "{model}");
        assert!(model.contains("PARAMETER num_predict 8"), "{model}");
        assert!(model.contains("<think>\n\n</think>"), "{model}");
        assert!(model.starts_with("FROM "), "{model}");
        let spaced = dir.join("seat dir").join("specialist Q4.gguf");
        let quoted = journey_modelfile(&spaced, SeatChat::Qwen35);
        let shared = model_estate::gguf_modelfile(&spaced);
        assert!(quoted.contains("FROM \""), "{quoted}");
        assert!(shared.starts_with("FROM \""), "{shared}");
        assert!(quoted.contains("specialist Q4.gguf"));
        let from = quoted
            .lines()
            .next()
            .unwrap()
            .trim_start_matches("FROM ")
            .trim_matches('"');
        assert!(Path::new(from).is_absolute(), "{quoted}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn relative_out_modelfile_from_is_absolute() {
        let rel = PathBuf::from(".cell/classify-journey");
        let _ = fs::remove_dir_all(&rel);
        let paths = JourneyPaths::new(&rel);
        fs::create_dir_all(&paths.out).unwrap();
        for which in ["base", "specialist"] {
            let gguf = paths.seated_gguf(which, DEFAULT_QUANT);
            fs::write(&gguf, b"GGUF").unwrap();
            assert!(gguf.is_relative(), "{}", gguf.display());
            let text = journey_modelfile(&gguf, SeatChat::Qwen35);
            let from = text
                .lines()
                .next()
                .unwrap()
                .trim_start_matches("FROM ")
                .trim_matches('"');
            assert!(Path::new(from).is_absolute(), "{text}");
            assert!(
                from.contains(&gguf.file_name().unwrap().to_string_lossy().to_string()),
                "{text}"
            );
        }
        let _ = fs::remove_dir_all(&rel);
    }

    #[test]
    fn fresh_plan_converts_and_quants_both_seats() {
        let dir = std::env::temp_dir().join(format!("journey-plan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let paths = JourneyPaths::new(&dir);
        let inputs = Inputs {
            prepare: "prep".into(),
            fetch: fetch_inputs(DEFAULT_BASE),
            pipeline: None,
            recipe: None,
            eval: None,
            ollama_base: None,
            ollama_spec: None,
        };
        let ctx = PlanCtx {
            paths: &paths,
            base: DEFAULT_BASE,
            hf_bin: "huggingface-cli",
            quant: DEFAULT_QUANT,
            library_tag: None,
            built_base_tag: DEFAULT_BUILT_BASE_TAG,
            specialist_tag: DEFAULT_TAG,
            llama: None,
            inputs: &inputs,
            check_ollama: false,
            train_driver: TrainDriver::Local,
            together_model: DEFAULT_TOGETHER_MODEL,
        };
        let fresh = plan_with(&ctx);
        let names: Vec<_> = fresh.iter().map(|s| s.name).collect();
        assert_eq!(names, STEP_ORDER);
        let base = fresh
            .iter()
            .find(|s| s.name == "gguf-convert-base")
            .unwrap();
        assert!(
            base.detail
                .contains("python3 $LLAMA_CPP_DIR/convert_hf_to_gguf.py"),
            "{base:?}"
        );
        assert!(base.detail.contains("--outtype f16"), "{base:?}");
        assert!(base.detail.contains("base-hf"), "{base:?}");
        assert!(!base.detail.contains(DEFAULT_BASE), "{base:?}");
        let fetch = fresh.iter().find(|s| s.name == "fetch-base").unwrap();
        assert!(
            fetch.detail.contains("huggingface-cli download"),
            "{fetch:?}"
        );
        assert!(fetch.detail.contains(DEFAULT_BASE), "{fetch:?}");
        assert!(fetch.detail.contains("--local-dir"), "{fetch:?}");
        let spec = fresh
            .iter()
            .find(|s| s.name == "gguf-convert-specialist")
            .unwrap();
        assert!(
            spec.detail
                .contains("python3 $LLAMA_CPP_DIR/convert_hf_to_gguf.py"),
            "{spec:?}"
        );
        let quant = fresh
            .iter()
            .find(|s| s.name == "quantize-specialist")
            .unwrap();
        assert!(quant.detail.contains("Q4_K_M"), "{quant:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn matching_manifest_skips_and_stale_or_changed_inputs_redo() {
        let dir = std::env::temp_dir().join(format!("journey-manifest-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let paths = JourneyPaths::new(&dir);
        fs::create_dir_all(&paths.base_hf).unwrap();
        fs::write(paths.base_hf.join("config.json"), "{}\n").unwrap();
        fs::create_dir_all(&paths.adapter_dir).unwrap();
        fs::create_dir_all(&paths.export_dir).unwrap();
        fs::write(&paths.dataset_jsonl, "row\n").unwrap();
        fs::write(&paths.heldout, "{}\n").unwrap();
        fs::write(&paths.dataset_info, "{}\n").unwrap();
        fs::write(&paths.recipe, "dataset: tev1_decisions\n").unwrap();
        fs::write(&paths.export_yaml, "export_dir: x\n").unwrap();
        fs::write(paths.adapter_dir.join("adapter_config.json"), "{}\n").unwrap();
        fs::write(paths.export_dir.join("config.json"), "{}\n").unwrap();
        fs::write(&paths.base_f16, "base-f16").unwrap();
        fs::write(&paths.specialist_f16, "spec-f16").unwrap();
        let seated_b = paths.seated_gguf("base", DEFAULT_QUANT);
        let seated_s = paths.seated_gguf("specialist", DEFAULT_QUANT);
        fs::write(&seated_b, "base-q").unwrap();
        fs::write(&seated_s, "spec-q").unwrap();
        fs::write(&paths.base_report, "{}\n").unwrap();
        fs::write(&paths.specialist_report, "{}\n").unwrap();
        let prepare = String::from("prepare-key");
        let pipeline = pipeline_inputs(
            DEFAULT_BASE,
            &paths.dataset_jsonl,
            &paths.recipe,
            DEFAULT_QUANT,
            TrainDriver::Local,
            DEFAULT_TOGETHER_MODEL,
        )
        .unwrap();
        let endpoint = "http://127.0.0.1:11434";
        let model_path = resolved_base_dir(DEFAULT_BASE, &paths)
            .display()
            .to_string();
        let recipe_key = recipe_decision_inputs(
            &model_path,
            DEFAULT_BASE,
            DEFAULT_DATASET,
            &paths.dataset_jsonl,
            &paths.out,
            &paths.adapter_dir,
            None,
        )
        .unwrap();
        let eval_key = eval_inputs(
            &pipeline,
            DEFAULT_TAG,
            endpoint,
            None,
            DEFAULT_BUILT_BASE_TAG,
        );
        let ollama_base = ollama_inputs(
            &pipeline,
            DEFAULT_BUILT_BASE_TAG,
            DEFAULT_TAG,
            endpoint,
            None,
            DEFAULT_BUILT_BASE_TAG,
        );
        let ollama_spec = ollama_inputs(
            &pipeline,
            DEFAULT_TAG,
            DEFAULT_TAG,
            endpoint,
            None,
            DEFAULT_BUILT_BASE_TAG,
        );
        for step in [
            "prepare",
            "fetch-base",
            "recipe",
            "train",
            "merge-export",
            "gguf-convert-base",
            "gguf-convert-specialist",
            "quantize-base",
            "quantize-specialist",
            "eval-base",
            "eval-specialist",
        ] {
            let fetch_key = fetch_inputs(DEFAULT_BASE);
            let key = match step {
                "prepare" => prepare.as_str(),
                "fetch-base" => fetch_key.as_str(),
                "recipe" => recipe_key.as_str(),
                "eval-base" | "eval-specialist" => eval_key.as_str(),
                _ => pipeline.as_str(),
            };
            write_manifest(&paths.manifest_path(step), key, None).unwrap();
        }
        let spec_sha = file_sha(&seated_s).unwrap();
        write_manifest(
            &paths.manifest_path("ollama-create-specialist"),
            &ollama_spec,
            Some(&spec_sha),
        )
        .unwrap();
        write_manifest(
            &paths.manifest_path("ollama-create-base"),
            &ollama_base,
            Some(&file_sha(&seated_b).unwrap()),
        )
        .unwrap();
        let inputs = Inputs {
            prepare: prepare.clone(),
            fetch: fetch_inputs(DEFAULT_BASE),
            pipeline: Some(pipeline.clone()),
            recipe: Some(recipe_key.clone()),
            eval: Some(eval_key.clone()),
            ollama_base: Some(ollama_base),
            ollama_spec: Some(ollama_spec),
        };
        let ctx = PlanCtx {
            paths: &paths,
            base: DEFAULT_BASE,
            hf_bin: "huggingface-cli",
            quant: DEFAULT_QUANT,
            library_tag: None,
            built_base_tag: DEFAULT_BUILT_BASE_TAG,
            specialist_tag: DEFAULT_TAG,
            llama: None,
            inputs: &inputs,
            check_ollama: false,
            train_driver: TrainDriver::Local,
            together_model: DEFAULT_TOGETHER_MODEL,
        };
        let again = plan_with(&ctx);
        for step in &again {
            if step.name == "compare" {
                assert_eq!(step.action, StepAction::Run);
            } else {
                assert_eq!(
                    step.action,
                    StepAction::Skip,
                    "{} {}",
                    step.name,
                    step.detail
                );
            }
        }

        let bumped = recipe_decision_inputs(
            &model_path,
            DEFAULT_BASE,
            DEFAULT_DATASET,
            &paths.dataset_jsonl,
            &paths.out,
            &paths.adapter_dir,
            Some(4),
        )
        .unwrap();
        assert_ne!(bumped, recipe_key);
        let bumped_inputs = Inputs {
            recipe: Some(bumped),
            ..inputs.clone()
        };
        let bumped_ctx = PlanCtx {
            inputs: &bumped_inputs,
            ..ctx
        };
        let recipe_step = plan_with(&bumped_ctx)
            .into_iter()
            .find(|s| s.name == "recipe")
            .unwrap();
        assert_eq!(recipe_step.action, StepAction::Run, "{recipe_step:?}");
        assert!(
            recipe_step.detail.contains("inputs changed"),
            "{recipe_step:?}"
        );

        let retagged = eval_inputs(
            &pipeline,
            "other-tag",
            endpoint,
            None,
            DEFAULT_BUILT_BASE_TAG,
        );
        assert_ne!(retagged, eval_key);
        let retag_inputs = Inputs {
            eval: Some(retagged.clone()),
            ollama_spec: Some(ollama_inputs(
                &pipeline,
                "other-tag",
                "other-tag",
                endpoint,
                None,
                DEFAULT_BUILT_BASE_TAG,
            )),
            ..inputs.clone()
        };
        let retag_ctx = PlanCtx {
            inputs: &retag_inputs,
            specialist_tag: "other-tag",
            ..ctx
        };
        let planned = plan_with(&retag_ctx);
        let eval_step = planned
            .iter()
            .find(|s| s.name == "eval-specialist")
            .unwrap();
        assert_eq!(eval_step.action, StepAction::Run, "{eval_step:?}");
        let seat_step = planned
            .iter()
            .find(|s| s.name == "ollama-create-specialist")
            .unwrap();
        assert_eq!(seat_step.action, StepAction::Run, "{seat_step:?}");

        let moved = eval_inputs(
            &pipeline,
            DEFAULT_TAG,
            "http://127.0.0.1:9",
            Some("qwen3.5:4b"),
            DEFAULT_BUILT_BASE_TAG,
        );
        assert_ne!(moved, eval_key);
        let moved_inputs = Inputs {
            eval: Some(moved),
            ..inputs.clone()
        };
        let moved_ctx = PlanCtx {
            inputs: &moved_inputs,
            library_tag: Some("qwen3.5:4b"),
            ..ctx
        };
        let eval_base = plan_with(&moved_ctx)
            .into_iter()
            .find(|s| s.name == "eval-base")
            .unwrap();
        assert_eq!(eval_base.action, StepAction::Run, "{eval_base:?}");

        fs::remove_file(paths.manifest_path("train")).unwrap();
        let stale = plan_with(&ctx);
        let train = stale.iter().find(|s| s.name == "train").unwrap();
        assert_eq!(train.action, StepAction::Run);
        assert!(
            train.detail.contains("redo train: missing manifest"),
            "{train:?}"
        );

        write_manifest(&paths.manifest_path("train"), &pipeline, None).unwrap();
        fs::write(&paths.dataset_jsonl, "row-changed\n").unwrap();
        let changed = pipeline_inputs(
            DEFAULT_BASE,
            &paths.dataset_jsonl,
            &paths.recipe,
            DEFAULT_QUANT,
            TrainDriver::Local,
            DEFAULT_TOGETHER_MODEL,
        )
        .unwrap();
        assert_ne!(changed, pipeline);
        let inputs = Inputs {
            prepare,
            fetch: fetch_inputs(DEFAULT_BASE),
            pipeline: Some(changed.clone()),
            recipe: Some(recipe_key),
            eval: Some(eval_inputs(
                &changed,
                DEFAULT_TAG,
                endpoint,
                None,
                DEFAULT_BUILT_BASE_TAG,
            )),
            ollama_base: None,
            ollama_spec: Some(ollama_inputs(
                &changed,
                DEFAULT_TAG,
                DEFAULT_TAG,
                endpoint,
                None,
                DEFAULT_BUILT_BASE_TAG,
            )),
        };
        let ctx = PlanCtx {
            inputs: &inputs,
            ..ctx
        };
        let redone = plan_with(&ctx);
        let train = redone.iter().find(|s| s.name == "train").unwrap();
        assert!(
            train.detail.contains("redo train: inputs changed"),
            "{train:?}"
        );
        fs::write(&seated_s, "spec-q-mutated").unwrap();
        let gguf_changed = plan_with(&ctx);
        let seat = gguf_changed
            .iter()
            .find(|s| s.name == "ollama-create-specialist")
            .unwrap();
        assert!(
            seat.detail
                .contains("redo ollama-create-specialist: GGUF hash changed"),
            "{seat:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn library_tag_skips_base_build_and_f16_skips_quantize() {
        let dir = std::env::temp_dir().join(format!("journey-lib-{}", std::process::id()));
        let paths = JourneyPaths::new(&dir);
        let inputs = Inputs {
            prepare: "p".into(),
            fetch: fetch_inputs(DEFAULT_BASE),
            pipeline: None,
            recipe: None,
            eval: None,
            ollama_base: None,
            ollama_spec: None,
        };
        let ctx = PlanCtx {
            paths: &paths,
            base: DEFAULT_BASE,
            hf_bin: "huggingface-cli",
            quant: "f16",
            library_tag: Some("qwen3.5:4b"),
            built_base_tag: DEFAULT_BUILT_BASE_TAG,
            specialist_tag: DEFAULT_TAG,
            llama: None,
            inputs: &inputs,
            check_ollama: false,
            train_driver: TrainDriver::Local,
            together_model: DEFAULT_TOGETHER_MODEL,
        };
        let steps = plan_with(&ctx);
        for name in ["gguf-convert-base", "quantize-base", "ollama-create-base"] {
            let step = steps.iter().find(|s| s.name == name).unwrap();
            assert_eq!(step.action, StepAction::Skip, "{step:?}");
            assert!(step.detail.contains("precision may differ"), "{step:?}");
        }
        let quant = steps
            .iter()
            .find(|s| s.name == "quantize-specialist")
            .unwrap();
        assert_eq!(quant.action, StepAction::Skip);
        assert!(
            quant.detail.contains("f16 skips llama-quantize"),
            "{quant:?}"
        );
    }

    #[test]
    fn missing_tool_messages_name_each_gap() {
        let gaps = tool_gaps(|_| false, None, false, true, QWEN35_RECENT);
        let text = gaps.message();
        assert!(text.contains("llamafactory-cli is not on PATH"), "{text}");
        assert!(text.contains("LLAMA_CPP_DIR is unset"), "{text}");
        assert!(text.contains(QWEN35_RECENT), "{text}");
        assert!(
            text.contains("huggingface-cli and hf are not on PATH"),
            "{text}"
        );
        assert!(text.contains("HF_TOKEN"), "{text}");
        assert!(text.contains("ollama is not on PATH"), "{text}");
        assert!(text.contains("no GPU"), "{text}");
        let partial = tool_gaps(|name| name == "ollama", None, true, false, QWEN35_RECENT);
        let text = partial.message();
        assert!(text.contains("llamafactory-cli"));
        assert!(!text.contains("ollama is not"));
        assert!(!text.contains("no GPU"));
    }

    #[test]
    fn convert_print_matches_argv() {
        let dir = std::env::temp_dir().join(format!("journey-llama-{}", std::process::id()));
        fs::create_dir_all(dir.join("build/bin")).unwrap();
        fs::write(dir.join("convert_hf_to_gguf.py"), "print('x')\n").unwrap();
        fs::write(dir.join("build/bin/llama-quantize"), "").unwrap();
        assert!(resolve_llama_cpp_note(&dir, QWEN35_RECENT)
            .unwrap()
            .quantize
            .ends_with("build/bin/llama-quantize"));
        let bin_dir = dir.join("bin-only");
        fs::create_dir_all(bin_dir.join("bin")).unwrap();
        fs::write(bin_dir.join("convert_hf_to_gguf.py"), "print('x')\n").unwrap();
        fs::write(bin_dir.join("bin/llama-quantize"), "").unwrap();
        assert!(resolve_llama_cpp_note(&bin_dir, QWEN35_RECENT)
            .unwrap()
            .quantize
            .ends_with("bin/llama-quantize"));
        let llama = resolve_llama_cpp_note(&dir, QWEN35_RECENT).unwrap();
        let outfile = dir.join("base.f16.gguf");
        let export = dir.join("export");
        fs::create_dir_all(&export).unwrap();
        let argv = llama.convert_argv(&export.display().to_string(), &outfile);
        let line = argv_line(&argv);
        assert_eq!(
            line,
            format!(
                "python3 {} {} --outfile {} --outtype f16",
                llama.convert.display(),
                export.display(),
                outfile.display()
            )
        );
        assert!(!argv.iter().any(|arg| arg == DEFAULT_BASE));
        let missing = dir.join("nope");
        let err = resolve_llama_cpp_note(&missing, QWEN35_RECENT)
            .unwrap_err()
            .to_string();
        assert!(err.contains(QWEN35_RECENT), "{err}");
        assert!(err.contains("convert_hf_to_gguf.py"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn comparison_math_and_thresholds() {
        let base = SideScore {
            accuracy: 0.25,
            invalid: 2,
            records: 8,
            p50: 10.0,
            p95: 40.0,
            thinking_leak: 3,
        };
        let specialist = SideScore {
            accuracy: 0.75,
            invalid: 1,
            records: 8,
            p50: 12.0,
            p95: 30.0,
            thinking_leak: 0,
        };
        let comparison = compare_sides(&base, &specialist);
        assert!((comparison.delta - 0.5).abs() < 1e-9);
        assert!((comparison.base_invalid_rate - 0.25).abs() < 1e-9);
        assert!((comparison.specialist_invalid_rate - 0.125).abs() < 1e-9);
        assert_eq!(comparison.base_thinking_leak, 3);
        assert_eq!(comparison.specialist_thinking_leak, 0);
        assert_eq!(threshold_met(&comparison, None, None), None);
        assert_eq!(threshold_met(&comparison, Some(0.4), Some(0.7)), Some(true));
        assert_eq!(threshold_met(&comparison, Some(0.6), None), Some(false));
        assert_eq!(threshold_met(&comparison, None, Some(0.9)), Some(false));
    }

    #[test]
    fn hub_id_fetches_a_local_dir_and_a_local_dir_is_passthrough() {
        let dir = std::env::temp_dir().join(format!("journey-fetch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let paths = JourneyPaths::new(&dir);
        let inputs = Inputs {
            prepare: "p".into(),
            fetch: fetch_inputs(DEFAULT_BASE),
            pipeline: None,
            recipe: None,
            eval: None,
            ollama_base: None,
            ollama_spec: None,
        };
        let ctx = PlanCtx {
            paths: &paths,
            base: DEFAULT_BASE,
            hf_bin: "huggingface-cli",
            quant: DEFAULT_QUANT,
            library_tag: None,
            built_base_tag: DEFAULT_BUILT_BASE_TAG,
            specialist_tag: DEFAULT_TAG,
            llama: None,
            inputs: &inputs,
            check_ollama: false,
            train_driver: TrainDriver::Local,
            together_model: DEFAULT_TOGETHER_MODEL,
        };
        let hub = plan_with(&ctx);
        let fetch = hub.iter().find(|s| s.name == "fetch-base").unwrap();
        let expected = argv_line(&hf_download_argv(
            "huggingface-cli",
            DEFAULT_BASE,
            &paths.base_hf,
        ));
        assert_eq!(fetch.detail, expected);
        let convert = hub.iter().find(|s| s.name == "gguf-convert-base").unwrap();
        assert!(
            convert
                .detail
                .contains(&paths.base_hf.display().to_string()),
            "{convert:?}"
        );
        assert!(
            !convert.detail.contains("Qwen/Qwen3.5-4B --outfile"),
            "{convert:?}"
        );

        let local = dir.join("already");
        fs::create_dir_all(&local).unwrap();
        let local_s = local.display().to_string();
        let inputs = Inputs {
            prepare: "p".into(),
            fetch: fetch_inputs(&local_s),
            pipeline: None,
            recipe: None,
            eval: None,
            ollama_base: None,
            ollama_spec: None,
        };
        let ctx = PlanCtx {
            paths: &paths,
            base: &local_s,
            hf_bin: "hf",
            quant: DEFAULT_QUANT,
            library_tag: None,
            built_base_tag: DEFAULT_BUILT_BASE_TAG,
            specialist_tag: DEFAULT_TAG,
            llama: None,
            inputs: &inputs,
            check_ollama: false,
            train_driver: TrainDriver::Local,
            together_model: DEFAULT_TOGETHER_MODEL,
        };
        let passed = plan_with(&ctx);
        let fetch = passed.iter().find(|s| s.name == "fetch-base").unwrap();
        assert_eq!(fetch.detail, format!("local base {local_s}"));
        assert!(!fetch.detail.contains("download"), "{fetch:?}");
        let convert = passed
            .iter()
            .find(|s| s.name == "gguf-convert-base")
            .unwrap();
        assert!(convert.detail.contains(&local_s), "{convert:?}");
        assert!(validate_snapshot(&local).is_err());
        fs::write(local.join("config.json"), "{}\n").unwrap();
        fs::write(local.join("tokenizer.json"), "{}\n").unwrap();
        fs::write(local.join("model.safetensors"), "w\n").unwrap();
        assert!(validate_snapshot(&local).is_ok());
        fs::remove_file(local.join("model.safetensors")).unwrap();
        fs::write(
            local.join("model.safetensors.index.json"),
            "{\"weight_map\":{\"w\":\"model-00001-of-00001.safetensors\"}}\n",
        )
        .unwrap();
        assert!(validate_snapshot(&local).is_err());
        fs::write(local.join("model-00001-of-00001.safetensors"), "shard\n").unwrap();
        assert!(validate_snapshot(&local).is_ok());
        fs::write(
            local.join("model.safetensors.index.json"),
            "{\"weight_map\":{\"a\":\"model-00001-of-00002.safetensors\",\"b\":\"model-00002-of-00002.safetensors\"}}\n",
        )
        .unwrap();
        fs::write(local.join("model-00001-of-00002.safetensors"), "one\n").unwrap();
        assert!(
            !snapshot_has_weights(&local),
            "an index requires every named shard"
        );
        fs::write(local.join("model-00002-of-00002.safetensors"), "").unwrap();
        assert!(
            !snapshot_has_weights(&local),
            "an empty shard is not weights"
        );
        fs::write(local.join("model-00002-of-00002.safetensors"), "two\n").unwrap();
        assert!(snapshot_has_weights(&local));
        assert!(validate_gguf(&paths.base_f16).is_err());
        fs::write(&paths.base_f16, b"GGUF").unwrap();
        assert!(validate_gguf(&paths.base_f16).is_ok());
        let _ = fs::remove_dir_all(&dir);
    }

    fn ustar_entry(name: &str, typeflag: u8, data: &[u8]) -> Vec<u8> {
        let mut header = [0u8; 512];
        let bytes = name.as_bytes();
        header[..bytes.len()].copy_from_slice(bytes);
        let size = format!("{:<7o}", data.len());
        header[124..124 + size.len()].copy_from_slice(size.as_bytes());
        header[156] = typeflag;
        header[257..262].copy_from_slice(b"ustar");
        let mut out = header.to_vec();
        out.extend_from_slice(data);
        let pad = (512 - (data.len() % 512)) % 512;
        out.extend(std::iter::repeat(0).take(pad));
        out
    }

    fn ustar(entries: &[(&str, u8, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, typeflag, data) in entries {
            out.extend(ustar_entry(name, *typeflag, data));
        }
        out.extend(std::iter::repeat(0).take(1024));
        out
    }

    #[test]
    fn adapter_tar_skips_directories_and_refuses_links_and_other_types() {
        let dir = std::env::temp_dir().join(format!("journey-tar-ok-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let archive = ustar(&[
            ("weights", b'5', b""),
            ("weights/adapter_config.json", b'0', b"{}\n"),
            ("./pax", b'x', b"path=weights/adapter_config.json\n"),
        ]);
        unpack_adapter_tar(&archive, &dir).unwrap();
        assert_eq!(fs::read(dir.join("adapter_config.json")).unwrap(), b"{}\n");

        let link = ustar(&[("adapter_model.safetensors", b'2', b"elsewhere")]);
        let err = unpack_adapter_tar(&link, &dir).unwrap_err().to_string();
        assert!(err.contains("hardlink or symlink"), "{err}");

        let hard = ustar(&[("adapter_model.safetensors", b'1', b"adapter_config.json")]);
        let err = unpack_adapter_tar(&hard, &dir).unwrap_err().to_string();
        assert!(err.contains("hardlink or symlink"), "{err}");

        let fifo = ustar(&[("adapter_model.safetensors", b'6', b"")]);
        let err = unpack_adapter_tar(&fifo, &dir).unwrap_err().to_string();
        assert!(err.contains("not a regular file"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn adapter_archive_decompresses_zstd_and_gzip_and_refuses_unknown_codecs() {
        let raw = ustar(&[("adapter_config.json", b'0', b"{}\n")]);
        let zst = zstd::stream::encode_all(std::io::Cursor::new(&raw), 0).unwrap();
        assert!(is_zstd(&zst));
        assert_eq!(decompress_adapter_archive(&zst).unwrap(), raw);
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut gzip, &raw).unwrap();
        let gz = gzip.finish().unwrap();
        assert!(is_gzip(&gz));
        assert_eq!(decompress_adapter_archive(&gz).unwrap(), raw);
        let err = decompress_adapter_archive(&[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00])
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown compression codec"), "{err}");
        assert_eq!(decompress_adapter_archive(&raw).unwrap(), raw);
    }

    #[test]
    fn deepseek_preset_fills_base_tag_and_template_and_refuses_default_together() {
        let applied = apply_preset(
            JourneyPreset::DeepseekR1Distill,
            DEFAULT_BASE,
            DEFAULT_TAG,
            TrainDriver::Local,
            None,
        )
        .unwrap();
        assert_eq!(applied.base, DEEPSEEK_R1_DISTILL_BASE);
        assert_eq!(applied.tag, DEEPSEEK_R1_DISTILL_TAG);
        assert_eq!(applied.built_base_tag, DEEPSEEK_R1_DISTILL_BUILT_BASE_TAG);
        assert_eq!(applied.seat, SeatChat::DeepseekR1);
        assert_eq!(train_template(&applied.base), "deepseekr1");
        let kept = apply_preset(
            JourneyPreset::DeepseekR1Distill,
            "lab/other",
            "custom-tag",
            TrainDriver::Local,
            None,
        )
        .unwrap();
        assert_eq!(kept.base, "lab/other");
        assert_eq!(kept.tag, "custom-tag");
        assert_eq!(kept.seat, SeatChat::DeepseekR1);
        let tev1 = apply_preset(
            JourneyPreset::Tev1,
            DEFAULT_BASE,
            DEFAULT_TAG,
            TrainDriver::Together,
            None,
        )
        .unwrap();
        assert_eq!(tev1.base, DEFAULT_BASE);
        assert_eq!(tev1.tag, DEFAULT_TAG);
        assert_eq!(tev1.together_model, DEFAULT_TOGETHER_MODEL);
        assert_eq!(tev1.seat, SeatChat::Qwen35);
        let err = apply_preset(
            JourneyPreset::DeepseekR1Distill,
            DEFAULT_BASE,
            DEFAULT_TAG,
            TrainDriver::Together,
            None,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("local llamafactory-cli"), "{err}");
        let hosted = apply_preset(
            JourneyPreset::DeepseekR1Distill,
            DEFAULT_BASE,
            DEFAULT_TAG,
            TrainDriver::Together,
            Some("deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B"),
        )
        .unwrap();
        assert_eq!(
            hosted.together_model,
            "deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B"
        );
        let dir = std::env::temp_dir().join(format!("journey-deepseek-{}", std::process::id()));
        let paths = JourneyPaths::new(&dir);
        let recipe = lora_recipe_yaml(
            DEEPSEEK_R1_DISTILL_BASE,
            DEEPSEEK_R1_DISTILL_BASE,
            DEFAULT_DATASET,
            &dir,
            &paths.adapter_dir,
            None,
        );
        assert!(recipe.contains("template: deepseekr1"), "{recipe}");
        assert!(recipe.contains("enable_thinking: false"), "{recipe}");
        let model = journey_modelfile(&paths.specialist_f16, SeatChat::DeepseekR1);
        assert!(model.contains("<｜User｜>"), "{model}");
        assert!(model.contains("<｜Assistant｜>"), "{model}");
        assert!(
            model.contains("PARAMETER stop <｜end▁of▁sentence｜>"),
            "{model}"
        );
        assert!(!model.contains("<|im_end|>"), "{model}");
    }

    #[test]
    fn together_adapter_refuses_oversized_compressed_and_decompressed() {
        let raw = b"adapter-bytes-that-are-longer-than-the-test-cap";
        let err = decompress_adapter_archive_capped(raw, 8, 64)
            .unwrap_err()
            .to_string();
        assert!(err.contains("download exceeds 8 bytes"), "{err}");
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut gzip, raw).unwrap();
        let gz = gzip.finish().unwrap();
        assert!(gz.len() < raw.len() || gz.len() < 64);
        let err = decompress_adapter_archive_capped(&gz, gz.len(), 8)
            .unwrap_err()
            .to_string();
        assert!(err.contains("decompressed exceeds 8 bytes"), "{err}");
        let err = read_limited(std::io::Cursor::new(&raw[..]), 8, "download")
            .unwrap_err()
            .to_string();
        assert!(err.contains("download exceeds 8 bytes"), "{err}");
    }
}
