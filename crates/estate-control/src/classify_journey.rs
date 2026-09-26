//! qwen reproduce journey: prepare, LoRA recipe, train, merge, GGUF, quant, Ollama seat, scored eval.
//! `--print` is the default. `--run` executes. A comparison file is local output only.
//! After compare, the command prints the `import-trained` line for the specialist GGUF, then Standing next (estate).
//! `--import-trained` records that proposal (`auto_apply=false`) only when the GGUF is a regular file.
//!
//! The base and the specialist share one convert, quant, and Modelfile shape. Only the LoRA differs.
//! `--base-tag` is an opt-in library tag and skips that base build.

use crate::classify::{
    cmd_classify_eval, cmd_classify_prepare, cmd_classify_prepare_presplit, newcombe_delta_ci95,
    wilson_ci95, DatasetFormat, EvalApi, EvalGate,
};
use anyhow::{bail, Result};
use model_estate::llamafactory_template_name;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Hugging Face id LLaMA-Factory registers as Qwen3.5-4B-Thinking.
pub const DEFAULT_BASE: &str = "Qwen/Qwen3.5-4B";
/// Template name in LLaMA-Factory `constants.py` for that registration (`template="qwen3_5"`).
pub const QWEN35_TEMPLATE: &str = "qwen3_5";
/// Ollama tag created from the built base GGUF. Not a library tag.
pub const DEFAULT_BUILT_BASE_TAG: &str = "classify-base";
pub const DEFAULT_TAG: &str = "classify-specialist";
pub const DEFAULT_DATASET: &str = "classify_decisions";
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
/// GLM-4-9B-Chat. LLaMA-Factory DEFAULT DownloadSource and template `glm4`.
pub const GLM4_CHAT_BASE: &str = "zai-org/glm-4-9b-chat";
/// Ollama tag for the GLM-4 Chat specialist GGUF.
pub const GLM4_CHAT_TAG: &str = "glm4-chat-specialist";
/// Ollama tag created from the built GLM-4 Chat base GGUF. Not a library tag.
pub const GLM4_CHAT_BUILT_BASE_TAG: &str = "glm4-chat-base";
/// `--modest` train rows when `--train-size` is omitted or still the clap default `all`.
/// `parse_split_size` accepts this positive integer. A 5090-class proof, not the full split.
pub const MODEST_TRAIN_SIZE: &str = "500";
/// Numeric cap for an explicit `--train-size` under `--modest`. Larger counts are refused.
pub const MODEST_TRAIN_CAP: usize = 500;
/// LLaMA-Factory `max_steps` written for both students when `--max-steps` is omitted.
/// Batch 1 and gradient accumulation 4 see 200 examples, short of one epoch of 500 rows.
pub const MODEST_MAX_STEPS: u32 = 50;
/// Compressed Together adapter archive cap. Larger downloads are refused.
pub const MAX_TOGETHER_ADAPTER_COMPRESSED: usize = 512 * 1024 * 1024;
/// Decompressed Together adapter tar cap. Larger unpacks are refused.
pub const MAX_TOGETHER_ADAPTER_DECOMPRESSED: usize = 2 * 1024 * 1024 * 1024;

const QWEN35_RECENT: &str = "Qwen3.5 needs a recent llama.cpp checkout";
const DEEPSEEK_LLAMA_NOTE: &str =
    "DeepSeek-R1-Distill needs a llama.cpp checkout that converts that architecture";
const GLM4_LLAMA_NOTE: &str =
    "GLM-4 Chat needs a llama.cpp checkout that converts that architecture";

/// Which letter-journey defaults `classify journey` fills in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum JourneyPreset {
    /// `Qwen/Qwen3.5-4B`, template `qwen3_5`, tag `classify-specialist`. This is the default.
    Qwen,
    /// `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`, template `deepseekr1`, tag `deepseek-r1-distill-specialist`.
    /// Train stays `llamafactory-cli` unless `--together-model` is set with `--train-driver together`.
    DeepseekR1Distill,
    /// `zai-org/glm-4-9b-chat`, template `glm4`, tag `glm4-chat-specialist`.
    /// Train stays `llamafactory-cli` unless `--together-model` is set with `--train-driver together`.
    Glm4Chat,
}

/// Modelfile chat shape. The base and the specialist share one shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeatChat {
    Qwen35,
    DeepseekR1,
    Glm4,
}

/// Defaults after `--preset` replaces an untouched default Qwen base or tag.
#[derive(Debug)]
pub struct AppliedJourney {
    pub base: String,
    pub tag: String,
    pub built_base_tag: &'static str,
    pub seat: SeatChat,
    pub llama_note: &'static str,
    pub together_model: String,
}

/// `--base` and `--tag` left at the default Qwen defaults take the preset. An explicit value wins.
/// Together on the DeepSeek and GLM-4 Chat presets needs `--together-model`. The Qwen default stays the default Qwen path.
pub fn apply_preset(
    preset: JourneyPreset,
    base: &str,
    tag: &str,
    train_driver: TrainDriver,
    together_model: Option<&str>,
) -> Result<AppliedJourney> {
    let (base, tag, built_base_tag, seat, llama_note) = match preset {
        JourneyPreset::Qwen => (
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
        JourneyPreset::Glm4Chat => {
            let base = if base == DEFAULT_BASE {
                GLM4_CHAT_BASE.to_string()
            } else {
                base.to_string()
            };
            let tag = if tag == DEFAULT_TAG {
                GLM4_CHAT_TAG.to_string()
            } else {
                tag.to_string()
            };
            (
                base,
                tag,
                GLM4_CHAT_BUILT_BASE_TAG,
                SeatChat::Glm4,
                GLM4_LLAMA_NOTE,
            )
        }
    };
    let together_model = match (preset, train_driver, together_model.map(str::trim)) {
        (
            JourneyPreset::DeepseekR1Distill | JourneyPreset::Glm4Chat,
            TrainDriver::Together,
            None | Some(""),
        ) => {
            let which = match preset {
                JourneyPreset::Glm4Chat => "GLM-4 Chat",
                JourneyPreset::DeepseekR1Distill => "DeepSeek-R1-Distill",
                JourneyPreset::Qwen => "qwen",
            };
            bail!(
                "refuse:classify-journey: {which} journey uses local llamafactory-cli train. Together stays on the default Qwen path unless --together-model is set"
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

/// Non-thinking DeepSeek-R1-Distill prompt.
/// LLaMA-Factory `deepseekr1` `format_user` is `<｜User｜>{{content}}<｜Assistant｜>`
/// with no newlines. `format_assistant` is the default `{{content}}` plus
/// `eos_token` `<｜end▁of▁sentence｜>`, also with no newline. The HF chat
/// template concatenates the same way (`'<｜User｜>' + content`, and
/// `'<｜Assistant｜>' + content + '<｜end▁of▁sentence｜>'`).
/// `thought_words` stay `("<think>\n", "\n</think>\n\n")`. `add_thought("")`
/// is `<think>\n\n</think>\n\n`, appended after the generation `<｜Assistant｜>`
/// when `enable_thinking` is false.
const DEEPSEEK_R1_TEMPLATE: &str = "\
{{ if .System }}<｜begin▁of▁sentence｜>{{ .System }}{{ else }}<｜begin▁of▁sentence｜>{{ end }}{{ range .Messages }}{{ if eq .Role \"user\" }}<｜User｜>{{ .Content }}{{ else if eq .Role \"assistant\" }}<｜Assistant｜>{{ .Content }}<｜end▁of▁sentence｜>{{ end }}{{ end }}<｜Assistant｜><think>

</think>

";

/// GLM-4 Chat prompt from LLaMA-Factory `glm4` (`template.py`).
/// `format_prefix` is `[gMASK]<sop>` (no space). `format_system` is
/// `<|system|>\n{{content}}`. `format_user` is `<|user|>\n{{content}}<|assistant|>`.
/// `format_assistant` is `\n{{content}}` (`efficient_eos` appends the eos token
/// outside the slot). `stop_words` are `<|user|>` and `<|observation|>`.
/// The GLM-4-9B-Chat eos token is `<|endoftext|>`. This is not a reasoning
/// template, so `enable_thinking: false` does not insert a think block.
const GLM4_TEMPLATE: &str = "\
[gMASK]<sop>{{ if .System }}<|system|>
{{ .System }}{{ end }}{{ range .Messages }}{{ if eq .Role \"user\" }}<|user|>
{{ .Content }}<|assistant|>{{ else if eq .Role \"assistant\" }}
{{ .Content }}{{ end }}{{ end }}";

const STEP_ORDER: &[&str] = &[
    "prepare",
    "fetch-base",
    "recipe",
    "train",
    "merge-export",
    "export-repair",
    "gguf-convert-base",
    "gguf-convert-specialist",
    "quantize-base",
    "quantize-specialist",
    "ollama-create-base",
    "ollama-create-specialist",
    "eval-base",
    "eval-base-few-shot",
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
    /// Root for Hub snapshots. A Hub id downloads to `{base_cache}/{safe-id}/`.
    pub base_cache: PathBuf,
    pub base_f16: PathBuf,
    pub specialist_f16: PathBuf,
    pub base_modelfile: PathBuf,
    pub specialist_modelfile: PathBuf,
    pub heldout: PathBuf,
    pub dataset_jsonl: PathBuf,
    pub dataset_info: PathBuf,
    pub base_report: PathBuf,
    pub few_shot_report: PathBuf,
    /// `0` keeps the base eval zero-shot and skips `eval-base-few-shot`.
    pub few_shot: u32,
    pub few_shot_seed: u64,
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
            base_cache: PathBuf::from(DEFAULT_BASE_CACHE),
            base_f16: out.join("base.f16.gguf"),
            specialist_f16: out.join("specialist.f16.gguf"),
            base_modelfile: out.join("base.Modelfile"),
            specialist_modelfile: out.join("specialist.Modelfile"),
            heldout: out.join("heldout.jsonl"),
            dataset_jsonl: out.join("dataset.jsonl"),
            dataset_info: out.join("dataset_info.json"),
            base_report: out.join("base-report.json"),
            few_shot_report: out.join("base-few-shot-report.json"),
            few_shot: 0,
            few_shot_seed: 0,
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
# LoRA recipe for the qwen one-letter target. Cell One does not train unless classify journey --run.
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
    let (stops, template): (&[&str], &str) = match seat {
        SeatChat::Qwen35 => (&["<|im_end|>"], QWEN35_NOTHINK_TEMPLATE),
        SeatChat::DeepseekR1 => (&["<｜end▁of▁sentence｜>"], DEEPSEEK_R1_TEMPLATE),
        SeatChat::Glm4 => (
            &["<|endoftext|>", "<|user|>", "<|observation|>"],
            GLM4_TEMPLATE,
        ),
    };
    let stop_lines = stops
        .iter()
        .map(|stop| format!("PARAMETER stop {stop}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "\
FROM {gguf}
PARAMETER temperature 0
PARAMETER num_predict 8
{stop_lines}
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

pub const DEFAULT_BASE_CACHE: &str = ".cell/classify-base-cache";

/// Path-safe directory name for a Hub id. `Qwen/Qwen3.5-4B` becomes `Qwen--Qwen3.5-4B`.
pub fn sanitize_hub_id(hub_id: &str) -> String {
    let mut out = String::with_capacity(hub_id.len());
    for c in hub_id.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => out.push(c),
            '/' | '\\' => out.push_str("--"),
            _ => out.push('_'),
        }
    }
    if out.is_empty() || out.chars().all(|c| c == '.') {
        "hub".into()
    } else {
        out
    }
}

pub fn hub_cache_dir(cache_root: &Path, hub_id: &str) -> PathBuf {
    cache_root.join(sanitize_hub_id(hub_id))
}

/// Directory `convert_hf_to_gguf.py` reads.
/// A local directory is that path. A Hub id is `{base_cache}/{safe-id}/`.
pub fn resolved_base_dir(base: &str, paths: &JourneyPaths) -> PathBuf {
    if base_is_local_dir(base) {
        PathBuf::from(base)
    } else {
        hub_cache_dir(&paths.base_cache, base)
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

/// `hf` from current `huggingface_hub`, then the older `huggingface-cli` only when `hf` is absent.
/// huggingface_hub 1.x leaves `huggingface-cli` as a stub that prints a deprecation line and exits 1.
pub fn hf_bin_name() -> Option<&'static str> {
    if which("hf").is_some() {
        Some("hf")
    } else if which("huggingface-cli").is_some() {
        Some("huggingface-cli")
    } else {
        None
    }
}

fn hf_bin_label() -> String {
    match hf_bin_name() {
        Some(name) => name.to_string(),
        None => "missing".into(),
    }
}

/// True when a failed downloader printed the huggingface_hub 1.x stub notice.
/// Requires `deprecated` and either `huggingface-cli` as the subject or a hint to use `hf`.
/// An unrelated failure that only says "deprecated" is not that notice.
pub fn hf_downloader_deprecated(output: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    if !lower.contains("deprecated") {
        return false;
    }
    let names_cli = lower.contains("huggingface-cli");
    let points_at_hf = lower.contains("use `hf`")
        || lower.contains("use \"hf\"")
        || lower.contains("use 'hf'")
        || lower.match_indices("use hf").any(|(index, _)| {
            let after = index + "use hf".len();
            lower
                .as_bytes()
                .get(after)
                .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_' && *byte != b'-')
        });
    names_cli || points_at_hf
}

pub(crate) const HF_DEPRECATION_HINT: &str = "Install or use `hf` (`pipx install \"huggingface_hub[cli]\"` or `pip install -U huggingface_hub` in the venv)";

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
    /// Hash of tensors and tokenizer files `export-repair` would copy. Specialist steps include it.
    repair: Option<String>,
    /// Desired recipe from the CLI. Independent of the recipe file on disk.
    recipe: Option<String>,
    /// Eval skip key: pipeline, tags, and the rendered base and specialist Modelfiles.
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

/// Hub manifests name the Hub id and the shared snapshot path.
/// A local directory keeps the path-only key.
fn fetch_inputs(base: &str, resolved: &Path) -> String {
    if base_is_local_dir(base) {
        sha256_text(base)
    } else {
        sha256_text(&format!("hub\n{base}\n{}", resolved.display()))
    }
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
    base_modelfile: &str,
    specialist_modelfile: &str,
) -> String {
    sha256_text(&format!(
        "eval\n{pipeline}\n{}\n{}\n{}",
        seat_material(specialist_tag, endpoint, library_tag, built_base_tag),
        sha256_text(base_modelfile),
        sha256_text(specialist_modelfile),
    ))
}

fn ollama_inputs(
    pipeline: &str,
    tag: &str,
    specialist_tag: &str,
    endpoint: &str,
    library_tag: Option<&str>,
    built_base_tag: &str,
    modelfile: &str,
) -> String {
    sha256_text(&format!(
        "ollama\n{pipeline}\n{tag}\n{}\n{}",
        seat_material(specialist_tag, endpoint, library_tag, built_base_tag),
        sha256_text(modelfile),
    ))
}

fn journey_import_dir(
    alias: &str,
    train_size: &str,
    seed: u64,
    expand_tag: Option<&str>,
) -> Result<PathBuf> {
    match expand_tag {
        Some(tag) => {
            let tag = crate::classify_expand::validate_expand_tag(tag)?;
            if alias != "rust_idiom" {
                bail!("refuse:classify-journey: --expand-tag is the rust_idiom curriculum cache");
            }
            Ok(crate::classify_import::expand_cache_dir(
                alias, train_size, seed, &tag,
            ))
        }
        None => Ok(crate::classify_import::sampled_import_dir(
            alias, train_size, seed,
        )),
    }
}

fn dataset_prepare_key(req: &JourneyRequest<'_>, paths: &JourneyPaths) -> Result<String> {
    let dataset = req
        .import_dataset
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-journey: dataset is unset"))?;
    let preset = crate::classify_import::preset_by_name(dataset)?;
    let size = crate::classify_import::parse_split_size(req.train_size)?;
    let import_dir = journey_import_dir(preset.alias, &size.token(), req.seed, req.expand_tag)?;
    let train_hash = file_sha(&import_dir.join("train.jsonl")).unwrap_or_else(|| "missing".into());
    let held_hash = file_sha(&import_dir.join("heldout.jsonl")).unwrap_or_else(|| "missing".into());
    let _ = paths;
    let source =
        crate::classify_import::preset_source_token(preset, req.from_local, req.import_fetch);
    let stored = read_manifest(&paths.manifest_path("prepare")).map(|manifest| manifest.inputs);
    let cached = crate::classify_import::cached_native_source_fp(preset.alias);
    Ok(crate::classify_import::journey_prepare_fingerprint(
        preset.hf_id,
        req.train_size,
        req.heldout_size,
        req.seed,
        &train_hash,
        &held_hash,
        &source,
        stored.as_deref(),
        cached.as_deref(),
    ))
}

fn load_inputs(req: &JourneyRequest<'_>, paths: &JourneyPaths) -> Result<Inputs> {
    let prepare = if req.import_dataset.is_some() {
        dataset_prepare_key(req, paths)?
    } else {
        prepare_inputs(req.input, req.seed, req.held_out_ratio)?
    };
    let resolved_dir = resolved_base_dir(req.base, paths);
    let resolved = resolved_dir.display().to_string();
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
    let base_modelfile = journey_modelfile(&paths.seated_gguf("base", req.quant), req.seat);
    let spec_modelfile = journey_modelfile(&paths.seated_gguf("specialist", req.quant), req.seat);
    let eval = pipeline.as_ref().map(|pipeline| {
        eval_inputs(
            pipeline,
            req.tag,
            req.endpoint,
            library,
            req.built_base_tag,
            &base_modelfile,
            &spec_modelfile,
        )
    });
    let ollama_base = pipeline.as_ref().map(|pipeline| {
        ollama_inputs(
            pipeline,
            req.built_base_tag,
            req.tag,
            req.endpoint,
            library,
            req.built_base_tag,
            &base_modelfile,
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
            &spec_modelfile,
        )
    });
    let repair = pipeline.as_ref().map(|pipeline| {
        let base_dir = resolved_base_dir(req.base, paths);
        match crate::export_repair::preview_repair(&base_dir, &paths.export_dir) {
            Ok(report) => crate::export_repair::repair_fingerprint(pipeline, &report),
            Err(err) => sha256_text(&format!("export-repair-error\n{pipeline}\n{err}")),
        }
    });
    Ok(Inputs {
        prepare,
        fetch: fetch_inputs(req.base, &resolved_dir),
        pipeline,
        repair,
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
            "export-repair" => "export-repair",
            "gguf-convert-base" => "gguf-convert-base",
            "gguf-convert-specialist" => "gguf-convert-specialist",
            "quantize-base" => "quantize-base",
            "quantize-specialist" => "quantize-specialist",
            "ollama-create-base" => "ollama-create-base",
            "ollama-create-specialist" => "ollama-create-specialist",
            "eval-base" => "eval-base",
            "eval-base-few-shot" => "eval-base-few-shot",
            "eval-specialist" => "eval-specialist",
            "compare" => "compare",
            other => panic!("unknown step {other}"),
        },
        action: decision.action,
        detail,
    }
}

/// Written inside a Hub snapshot only after staging validation and before the atomic publish.
pub const HUB_COMPLETE_MARKER: &str = ".complete";

fn fetch_decision(base: &str, base_dir: &Path, paths: &JourneyPaths, expected: &str) -> Decision {
    let ready = if base_is_local_dir(base) {
        base_dir.join("config.json").is_file()
    } else {
        // A partial tree can contain config.json and a non-empty weight. Skip only after publish.
        base_dir.join(HUB_COMPLETE_MARKER).is_file()
    };
    if !ready {
        return Decision {
            action: StepAction::Run,
            redo: None,
        };
    }
    let manifest = paths.manifest_path("fetch-base");
    match read_manifest(&manifest) {
        Some(found) if found.inputs == expected => Decision {
            action: StepAction::Skip,
            redo: None,
        },
        Some(_) => Decision {
            action: StepAction::Run,
            redo: Some("inputs changed".into()),
        },
        // A later `--out` has no fetch manifest yet. The published marker is enough to plan a skip.
        // `execute` re-checks the marker under the download lock and re-downloads when it fails.
        None if !base_is_local_dir(base) => Decision {
            action: StepAction::Skip,
            redo: None,
        },
        None => Decision {
            action: StepAction::Run,
            redo: Some("missing manifest".into()),
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HubFetch {
    Reused,
    Downloaded,
}

extern "C" {
    fn flock(fd: i32, op: i32) -> i32;
}

const LOCK_EX: i32 = 2;
const LOCK_UN: i32 = 8;

struct HubDownloadLock {
    file: fs::File,
}

impl HubDownloadLock {
    fn acquire(cache_root: &Path, hub_id: &str) -> Result<Self> {
        let locks = cache_root.join(".locks");
        fs::create_dir_all(&locks)?;
        let path = locks.join(format!("{}.lock", sanitize_hub_id(hub_id)));
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|err| {
                anyhow::anyhow!(
                    "refuse:classify-journey: cannot open {}: {err}",
                    path.display()
                )
            })?;
        let rc = unsafe { flock(file.as_raw_fd(), LOCK_EX) };
        if rc != 0 {
            bail!("refuse:classify-journey: cannot lock {}", path.display());
        }
        Ok(Self { file })
    }
}

impl Drop for HubDownloadLock {
    fn drop(&mut self) {
        unsafe {
            flock(self.file.as_raw_fd(), LOCK_UN);
        }
    }
}

fn snapshot_manifest_files(dir: &Path) -> Result<Vec<Value>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = fs::read_dir(&current).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-journey: cannot read {}: {err}",
                current.display()
            )
        })?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let rel = path.strip_prefix(dir).unwrap_or(&path);
            let rel_s = rel.to_string_lossy().replace('\\', "/");
            if rel_s == HUB_COMPLETE_MARKER {
                continue;
            }
            let meta = fs::symlink_metadata(&path).map_err(|err| {
                anyhow::anyhow!(
                    "refuse:classify-journey: cannot stat {}: {err}",
                    path.display()
                )
            })?;
            if meta.file_type().is_symlink() {
                let followed = fs::metadata(&path).map_err(|err| {
                    anyhow::anyhow!(
                        "refuse:classify-journey: cannot follow {}: {err}",
                        path.display()
                    )
                })?;
                if followed.is_dir() {
                    bail!(
                        "refuse:classify-journey: directory symlink {} is refused",
                        path.display()
                    );
                }
                if followed.is_file() {
                    files.push(json!({"path": rel_s, "bytes": followed.len()}));
                }
                continue;
            }
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                files.push(json!({"path": rel_s, "bytes": meta.len()}));
            }
        }
    }
    files.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    Ok(files)
}

fn write_complete_marker(dir: &Path) -> Result<()> {
    let files = snapshot_manifest_files(dir)?;
    if files.is_empty() {
        bail!(
            "refuse:classify-journey: base snapshot {} has no files to publish",
            dir.display()
        );
    }
    let body = format!("{}\n", serde_json::to_string(&json!({"files": files}))?);
    let tmp = dir.join(".complete.tmp");
    fs::write(&tmp, body)?;
    fs::rename(tmp, dir.join(HUB_COMPLETE_MARKER))?;
    Ok(())
}

/// True when the published marker lists every file at the size captured at publish time,
/// and `validate_snapshot` still accepts the tree.
fn hub_snapshot_ready(dir: &Path) -> bool {
    let Ok(text) = fs::read_to_string(dir.join(HUB_COMPLETE_MARKER)) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    let Some(files) = value.get("files").and_then(Value::as_array) else {
        return false;
    };
    if files.is_empty() {
        return false;
    }
    for file in files {
        let Some(rel) = file.get("path").and_then(Value::as_str) else {
            return false;
        };
        if rel.is_empty() || rel.contains("..") || rel.starts_with('/') || rel.contains('\\') {
            return false;
        }
        let Some(bytes) = file.get("bytes").and_then(Value::as_u64) else {
            return false;
        };
        let path = dir.join(rel);
        let Ok(meta) = fs::metadata(&path) else {
            return false;
        };
        if !meta.is_file() || meta.len() != bytes {
            return false;
        }
    }
    validate_snapshot(dir).is_ok()
}

fn publish_snapshot(staging: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    if dest.exists() {
        let name = dest
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("snapshot");
        let trash = dest.with_file_name(format!(".{name}.replacing"));
        if trash.exists() {
            fs::remove_dir_all(&trash)?;
        }
        fs::rename(dest, &trash).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-journey: cannot move {} aside: {err}",
                dest.display()
            )
        })?;
        if let Err(err) = fs::rename(staging, dest) {
            let _ = fs::rename(&trash, dest);
            bail!(
                "refuse:classify-journey: cannot publish {}: {err}",
                dest.display()
            );
        }
        fs::remove_dir_all(&trash)?;
    } else if let Err(err) = fs::rename(staging, dest) {
        bail!(
            "refuse:classify-journey: cannot publish {}: {err}",
            dest.display()
        );
    }
    Ok(())
}

/// Download into a sibling staging directory, validate, write `.complete`, then rename into `dest`.
/// The exclusive lock covers that whole sequence. A ready published tree is reused unless `force`.
fn ensure_shared_hub_snapshot(repo: &str, dest: &Path, force: bool) -> Result<HubFetch> {
    let cache_root = dest.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-journey: hub cache {} has no parent",
            dest.display()
        )
    })?;
    let _lock = HubDownloadLock::acquire(cache_root, repo)?;
    if !force && hub_snapshot_ready(dest) {
        return Ok(HubFetch::Reused);
    }
    let staging = cache_root.join(".staging").join(sanitize_hub_id(repo));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;
    let bin = hf_bin_name().ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-journey: huggingface-cli and hf are not on PATH; needed to download the base. HF_TOKEN is read from the environment and is not printed"
        )
    })?;
    println!("{}", argv_line(&hf_download_argv(bin, repo, dest)));
    let staged = hf_download_argv(bin, repo, &staging);
    if let Err(err) = run_hf_download_quiet(&staged) {
        let _ = fs::remove_dir_all(&staging);
        return Err(err);
    }
    if let Err(err) = validate_snapshot(&staging).and_then(|_| write_complete_marker(&staging)) {
        let _ = fs::remove_dir_all(&staging);
        return Err(err);
    }
    if let Err(err) = publish_snapshot(&staging, dest) {
        let _ = fs::remove_dir_all(&staging);
        return Err(err);
    }
    Ok(HubFetch::Downloaded)
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
    let fetch = fetch_decision(ctx.base, &base_dir, paths, &ctx.inputs.fetch);
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

    let repair_key = ctx.inputs.repair.clone();
    let repair = match repair_key.as_deref() {
        Some(key) if merged.is_file() => {
            decide_file(&merged, &paths.manifest_path("export-repair"), key)
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
    let base_for_repair = resolved_base_dir(ctx.base, paths);
    steps.push(step_detail(
        "export-repair",
        &repair,
        format!(
            "copy tensors and tokenizer files missing from {} that are present in {}",
            paths.export_dir.display(),
            base_for_repair.display()
        ),
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
    let specialist_pipeline = ctx
        .inputs
        .pipeline
        .as_deref()
        .map(|pipeline| with_repair(pipeline, ctx.inputs.repair.as_deref()));
    let specialist_convert = match specialist_pipeline.as_deref() {
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
        match specialist_pipeline.as_deref() {
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
        "ollama create {} -f {}; load probe POST /api/chat think false num_predict 1",
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
        "ollama create {} -f {}; load probe POST /api/chat think false num_predict 1",
        ctx.specialist_tag,
        paths.specialist_modelfile.display()
    );
    let create_spec_key = ctx
        .inputs
        .ollama_spec
        .as_deref()
        .map(|key| with_repair(key, ctx.inputs.repair.as_deref()));
    let create_spec = match create_spec_key.as_deref() {
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
    let eval_base = if paths.base_report.is_file() && !report_is_complete(&paths.base_report) {
        Decision {
            action: StepAction::Run,
            redo: Some("partial eval report".into()),
        }
    } else {
        match ctx.inputs.eval.as_deref() {
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
        }
    };
    steps.push(step_detail(
        "eval-base",
        &eval_base,
        format!("classify eval --api ollama-native model {eval_base_model}"),
    ));

    let eval_few = few_shot_decision(paths, ctx.inputs.eval.as_deref());
    let few_detail = if paths.few_shot == 0 {
        "few-shot off".to_string()
    } else {
        format!(
            "classify eval --few-shot {} --exemplars {} --seed {} --api ollama-native model {eval_base_model}",
            paths.few_shot,
            paths.dataset_jsonl.display(),
            paths.few_shot_seed
        )
    };
    steps.push(step_detail("eval-base-few-shot", &eval_few, few_detail));

    let eval_spec_key = ctx
        .inputs
        .eval
        .as_deref()
        .map(|key| with_repair(key, ctx.inputs.repair.as_deref()));
    let eval_spec =
        if paths.specialist_report.is_file() && !report_is_complete(&paths.specialist_report) {
            Decision {
                action: StepAction::Run,
                redo: Some("partial eval report".into()),
            }
        } else {
            match eval_spec_key.as_deref() {
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
            }
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
    pub correct: u64,
    pub invalid: u64,
    pub records: u64,
    pub p50: f64,
    pub p95: f64,
    pub thinking_leak: u64,
}

fn few_shot_manifest_key(
    eval: Option<&str>,
    n: u32,
    seed: u64,
    exemplars: &Path,
) -> Option<String> {
    let eval = eval?;
    let bytes = fs::read(exemplars).ok()?;
    Some(sha256_text(&format!(
        "few-shot-base\n{eval}\n{n}\n{seed}\n{}",
        sha256_bytes(&bytes)
    )))
}

fn few_shot_decision(paths: &JourneyPaths, eval: Option<&str>) -> Decision {
    if paths.few_shot == 0 {
        return Decision {
            action: StepAction::Skip,
            redo: None,
        };
    }
    if paths.few_shot_report.is_file() && !report_is_complete(&paths.few_shot_report) {
        return Decision {
            action: StepAction::Run,
            redo: Some("partial eval report".into()),
        };
    }
    match few_shot_manifest_key(
        eval,
        paths.few_shot,
        paths.few_shot_seed,
        &paths.dataset_jsonl,
    ) {
        Some(key) if paths.few_shot_report.is_file() => decide_file(
            &paths.few_shot_report,
            &paths.manifest_path("eval-base-few-shot"),
            &key,
        ),
        _ if paths.few_shot_report.is_file() => Decision {
            action: StepAction::Run,
            redo: Some("missing manifest".into()),
        },
        _ => Decision {
            action: StepAction::Run,
            redo: None,
        },
    }
}

fn report_is_complete(path: &Path) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    value.get("complete").and_then(Value::as_bool) == Some(true)
}

pub fn side_score(report: &Value) -> Result<SideScore> {
    if report.get("complete").and_then(Value::as_bool) != Some(true) {
        bail!("refuse:classify-journey: eval report is partial; redo eval");
    }
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
    let correct = report
        .get("correct")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| (accuracy * records as f64).round() as u64);
    Ok(SideScore {
        accuracy,
        correct,
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
    pub base_correct: u64,
    pub specialist_correct: u64,
    pub base_records: u64,
    pub specialist_records: u64,
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
        base_correct: base.correct,
        specialist_correct: specialist.correct,
        base_records: base.records,
        specialist_records: specialist.records,
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

/// Newcombe 95% interval for specialist accuracy minus base accuracy.
/// `None` when either side has no records.
pub fn delta_ci95(comparison: &Comparison) -> Option<(f64, f64, f64)> {
    newcombe_delta_ci95(
        comparison.base_correct,
        comparison.base_records,
        comparison.specialist_correct,
        comparison.specialist_records,
    )
}

/// True only when the Newcombe 95% lower bound is strictly greater than 0.
/// A missing interval is not a lift.
pub fn significant_lift(comparison: &Comparison) -> bool {
    delta_ci95(comparison).is_some_and(|(_, low, _)| low > 0.0)
}

/// `None` when no threshold is set. `Some(true)` when every set threshold holds.
/// `--require-significant-lift` holds only when [`significant_lift`] is true.
pub fn threshold_met(
    comparison: &Comparison,
    min_delta: Option<f64>,
    min_accuracy: Option<f64>,
    require_significant_lift: bool,
) -> Option<bool> {
    if min_delta.is_none() && min_accuracy.is_none() && !require_significant_lift {
        return None;
    }
    let delta_ok = min_delta
        .map(|min| comparison.delta + f64::EPSILON >= min)
        .unwrap_or(true);
    let acc_ok = min_accuracy
        .map(|min| comparison.specialist_accuracy + f64::EPSILON >= min)
        .unwrap_or(true);
    let lift_ok = !require_significant_lift || significant_lift(comparison);
    Some(delta_ok && acc_ok && lift_ok)
}

fn format_ci(comparison: &Comparison) -> String {
    match delta_ci95(comparison) {
        Some((_, low, high)) => format!("[{low:.4}, {high:.4}]"),
        None => "unavailable".to_string(),
    }
}

/// Fail-closed text for a missed significant-lift gate. Prints both accuracies, the delta, and the CI.
pub fn significant_lift_refusal(
    comparison: &Comparison,
    min_delta: Option<f64>,
    min_accuracy: Option<f64>,
) -> String {
    let mut message = format!(
        "refuse:classify-journey: significant lift missed; base accuracy {:.4} specialist accuracy {:.4} delta {:.4} ci95 {}",
        comparison.base_accuracy,
        comparison.specialist_accuracy,
        comparison.delta,
        format_ci(comparison)
    );
    if let Some(min) = min_delta {
        if comparison.delta + f64::EPSILON < min {
            message.push_str(&format!("; min-delta {min:.4} missed"));
        }
    }
    if let Some(min) = min_accuracy {
        if comparison.specialist_accuracy + f64::EPSILON < min {
            message.push_str(&format!("; min-accuracy {min:.4} missed"));
        }
    }
    message
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
    /// Fail unless the Newcombe 95% CI lower bound for the accuracy delta is strictly greater than 0.
    pub require_significant_lift: bool,
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
    /// `ag_news`, `devign`, or `rust_idiom` (or a Hub id) replaces the built-in fixture. Import already made the held-out split.
    pub import_dataset: Option<&'a str>,
    pub train_size: &'a str,
    pub heldout_size: &'a str,
    /// Local dataset snapshot. Read-only. Unset uses `hf download` unless `import_fetch` is rows-api.
    pub from_local: Option<&'a Path>,
    pub import_fetch: crate::classify_import::ImportFetch,
    /// Python for pyarrow when the dataset is read from parquet.
    pub python: Option<&'a str>,
    /// Shared Hub snapshot root. Ignored when `--base` is a local directory.
    pub base_cache: &'a Path,
    /// `0` scores only the zero-shot base. `N >= 1` also writes `base-few-shot-report.json`.
    pub few_shot: u32,
    /// Tag-suffixed rust_idiom cache from `classify expand`. Prepare reads that cache and does not import again.
    pub expand_tag: Option<&'a str>,
    /// Estate file named on the import-trained line. Unset prints `<estate.yaml>`.
    pub estate: Option<&'a Path>,
    /// Enrich prepare directory. Unset prints `<prepared>`.
    pub prepared: Option<&'a Path>,
    /// `cell-enrich-{pack_id}` when `--import-trained` records. Unset prints `cell-enrich-<pack-id>`.
    pub enrich_tag: Option<&'a str>,
    /// Record the specialist GGUF after a successful `--run`. The proposal stays `auto_apply=false`.
    pub import_trained: bool,
    /// Portable local binding id. `None` keeps `local_slm`.
    pub binding_id: Option<&'a str>,
}

pub const DEFAULT_JOURNEY_OUT: &str = ".cell/classify-journey";

/// Suffix the default tag and journey directory so 3k / 10k / 30k runs coexist.
/// An explicit tag or `--out` is kept.
pub fn apply_dataset_layout(
    dataset: &str,
    train_size: &str,
    applied_tag: &str,
    requested_tag: &str,
    out: &Path,
    dataset_name: &str,
    expand_tag: Option<&str>,
) -> Result<(String, PathBuf, String)> {
    let preset = crate::classify_import::preset_by_name(dataset)?;
    let size = crate::classify_import::parse_split_size(train_size)?;
    let mut suffix = crate::classify_import::tag_suffix(preset, &size);
    if let Some(tag) = expand_tag {
        let tag = crate::classify_expand::validate_expand_tag(tag)?;
        if preset.alias != "rust_idiom" {
            bail!("refuse:classify-journey: --expand-tag is the rust_idiom curriculum cache");
        }
        suffix = format!("{suffix}-{tag}");
    }
    let tag = if requested_tag == DEFAULT_TAG {
        format!("{applied_tag}{suffix}")
    } else {
        applied_tag.to_string()
    };
    let out = if out == Path::new(DEFAULT_JOURNEY_OUT) {
        PathBuf::from(format!("{DEFAULT_JOURNEY_OUT}{suffix}"))
    } else {
        out.to_path_buf()
    };
    let dataset_name = if dataset_name == DEFAULT_DATASET {
        preset.alias.to_string()
    } else {
        dataset_name.to_string()
    };
    Ok((tag, out, dataset_name))
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
    if let Some(tag) = req.expand_tag {
        let _tag = crate::classify_expand::validate_expand_tag(tag)?;
        let alias = req
            .import_dataset
            .map(crate::classify_import::preset_by_name)
            .transpose()?
            .map(|preset| preset.alias);
        if alias != Some("rust_idiom") {
            bail!("refuse:classify-journey: --expand-tag is the rust_idiom curriculum cache");
        }
    }
    if req.run && req.import_trained && !import_trained_args_ready(req) {
        bail!(
            "refuse:classify-journey: --import-trained needs --estate, --prepared, and --enrich-tag. no proposal written."
        );
    }
    model_estate::resolve_portable_binding_id(req.binding_id)?;
    let mut paths = JourneyPaths::new(req.out);
    paths.base_cache = req.base_cache.to_path_buf();
    paths.few_shot = req.few_shot;
    paths.few_shot_seed = req.seed;
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
        hf_bin: hf_bin_name().unwrap_or("hf"),
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
        print_plan(req, &paths, template, library, &steps)?;
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
    let few_score = if paths.few_shot > 0 {
        Some(side_score(&read_json(&paths.few_shot_report)?)?)
    } else {
        None
    };
    let comparison = compare_sides(&side_score(&base_report)?, &side_score(&specialist_report)?);
    let verdict = threshold_met(
        &comparison,
        req.min_delta,
        req.min_accuracy,
        req.require_significant_lift,
    );
    write_comparison(
        req,
        &paths,
        template,
        library,
        &comparison,
        few_score.as_ref(),
        verdict,
    )?;
    match verdict {
        Some(false) if req.require_significant_lift && !significant_lift(&comparison) => {
            bail!(significant_lift_refusal(
                &comparison,
                req.min_delta,
                req.min_accuracy
            ))
        }
        Some(false) => bail!(
            "classify-journey: threshold missed; specialist accuracy {:.4} delta {:.4}",
            comparison.specialist_accuracy,
            comparison.delta
        ),
        Some(true) => println!("classify-journey: threshold met"),
        None => {}
    }
    seat_handoff(req, &paths, HandoffMode::AfterRun)?;
    Ok(())
}

fn print_plan(
    req: &JourneyRequest<'_>,
    paths: &JourneyPaths,
    template: &str,
    library: Option<&str>,
    steps: &[StepPlan],
) -> Result<()> {
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
    if req.preset == JourneyPreset::Qwen && req.base == DEFAULT_BASE {
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
    if req.preset == JourneyPreset::Glm4Chat {
        println!(
            "preset glm4-chat base {GLM4_CHAT_BASE} template {template} tag {} local llamafactory-cli train",
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
    if !base_is_local_dir(req.base) {
        let cache = resolved_base_dir(req.base, paths);
        println!("base_cache: {}", cache.display());
        println!("downloader: {}", hf_bin_label());
    }
    println!("out: {}", paths.out.display());
    if paths.few_shot == 0 {
        println!("few_shot: off");
    } else {
        println!(
            "few_shot: {} exemplars from {} seed {}",
            paths.few_shot,
            paths.dataset_jsonl.display(),
            paths.few_shot_seed
        );
    }
    if let Some(steps) = req.max_steps {
        println!("max_steps: {steps}");
    }
    if let Some(dataset) = req.import_dataset {
        println!(
            "dataset: {dataset} train_size: {} heldout_size: {} seed: {} option_order: fixed no-second-split",
            req.train_size, req.heldout_size, req.seed
        );
        if let Some(tag) = req.expand_tag {
            let preset = crate::classify_import::preset_by_name(dataset)?;
            let size = crate::classify_import::parse_split_size(req.train_size)?;
            let cache = journey_import_dir(preset.alias, &size.token(), req.seed, Some(tag))?;
            println!(
                "expand_tag: {tag} cache: {} no-import dry-run no teacher",
                cache.display()
            );
        } else if let Some(dir) = req.from_local {
            println!("dataset_source: local {}", dir.display());
        } else if req.import_fetch == crate::classify_import::ImportFetch::RowsApi {
            println!("dataset_source: rows-api");
        } else {
            println!("dataset_source: hf-download");
        }
    }
    for step in steps {
        let word = match step.action {
            StepAction::Run => "run",
            StepAction::Skip => "skip",
        };
        println!("{word} {:<24} {}", step.name, step.detail);
    }
    if req.require_significant_lift {
        println!(
            "significant_lift: required; Newcombe 95% CI lower bound for specialist minus base must be > 0"
        );
    }
    println!("compare writes {}", paths.comparison.display());
    println!("export-repair runs after merge-export and copies safetensors tensors and tokenizer files present in the base snapshot but missing from the merged export. Qwen3.5 MTP weights are named mtp.*. A load probe runs after each ollama create.");
    println!("This print does not train, convert, or seat. A later report is local output. It does not record a live PASS. READY_FOR_LIVE_TEST: no.");
    seat_handoff(req, paths, HandoffMode::Plan)?;
    Ok(())
}

enum HandoffMode {
    Plan,
    AfterRun,
}

enum SpecialistGguf {
    Ready,
    Missing,
    Symlink,
}

struct HandoffDisplay {
    estate: String,
    prepared: String,
    tag: String,
    adapter_path: PathBuf,
    line: String,
    /// Dataset alias when `--dataset` is set. Otherwise the specialist tag.
    function: String,
    binding_id: String,
}

fn import_trained_args_ready(req: &JourneyRequest<'_>) -> bool {
    req.estate.is_some_and(|path| !path.as_os_str().is_empty())
        && req
            .prepared
            .is_some_and(|path| !path.as_os_str().is_empty())
        && req
            .enrich_tag
            .map(str::trim)
            .is_some_and(|tag| !tag.is_empty())
}

fn specialist_gguf_status(path: &Path) -> SpecialistGguf {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => SpecialistGguf::Symlink,
        Ok(meta) if meta.is_file() => SpecialistGguf::Ready,
        _ => SpecialistGguf::Missing,
    }
}

fn shell_quote(text: &str) -> String {
    if text.chars().any(shell_quote_char) {
        format!("'{}'", text.replace('\'', "'\\''"))
    } else {
        text.to_string()
    }
}

fn shell_quote_char(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '"' | '\''
                | '\\'
                | '$'
                | '`'
                | ';'
                | '|'
                | '&'
                | '<'
                | '>'
                | '('
                | ')'
                | '!'
                | '*'
                | '?'
        )
}

fn display_token(text: &str, placeholder: bool) -> String {
    if placeholder {
        text.to_string()
    } else {
        shell_quote(text)
    }
}

fn specialty_function(req: &JourneyRequest<'_>) -> String {
    if let Some(name) = req
        .import_dataset
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if let Ok(preset) = crate::classify_import::preset_by_name(name) {
            return preset.alias.to_string();
        }
        return name.to_string();
    }
    req.tag.to_string()
}

fn handoff_display(
    req: &JourneyRequest<'_>,
    paths: &JourneyPaths,
    binding_id: &str,
) -> HandoffDisplay {
    let adapter_path = paths.seated_gguf("specialist", req.quant);
    let (estate_raw, estate_placeholder) = match req.estate {
        Some(path) if !path.as_os_str().is_empty() => (path.display().to_string(), false),
        _ => ("<estate.yaml>".to_string(), true),
    };
    let (prepared_raw, prepared_placeholder) = match req.prepared {
        Some(path) if !path.as_os_str().is_empty() => (path.display().to_string(), false),
        _ => ("<prepared>".to_string(), true),
    };
    let (tag_raw, tag_placeholder) =
        match req.enrich_tag.map(str::trim).filter(|tag| !tag.is_empty()) {
            Some(tag) => (tag.to_string(), false),
            None => ("cell-enrich-<pack-id>".to_string(), true),
        };
    let estate = display_token(&estate_raw, estate_placeholder);
    let prepared = display_token(&prepared_raw, prepared_placeholder);
    let tag = display_token(&tag_raw, tag_placeholder);
    let adapter = shell_quote(&adapter_path.display().to_string());
    let line = if binding_id == "local_slm" {
        format!(
            "estate enrich import-trained --estate {estate} --prepared {prepared} --tag {tag} --adapter {adapter}"
        )
    } else {
        format!(
            "estate enrich import-trained --estate {estate} --prepared {prepared} --tag {tag} --adapter {adapter} --binding-id {binding_id}"
        )
    };
    HandoffDisplay {
        estate,
        prepared,
        tag,
        adapter_path,
        line,
        function: specialty_function(req),
        binding_id: binding_id.to_string(),
    }
}

fn print_standing_next(display: &HandoffDisplay) {
    println!(
        "Standing next (estate) — after import-trained (trained_shape gguf, auto_apply=false):"
    );
    println!("The proposal stays auto_apply=false.");
    println!(
        "The factory does not apply the estate without an explicit operator --require-plan path."
    );
    println!("No promote. No auto-promote.");
    println!(
        "examples/estate.yaml stays unchanged unless the operator deliberately applies a plan."
    );
    for line in specialty_seat_lines(&display.binding_id, &display.function) {
        println!("{line}");
    }
    println!("Equal-class frontier and local. This local seat is a first-class peer of frontier.");
    println!("Other local specialty bindings stay beside this one.");
    println!("This function is one specialty local seat among those peers.");
    println!("Existing entrypoints (print only; this command does not execute them):");
    println!(
        "estate enrich apply-proposal --estate {} --prepared {} --tag {} --state-dir .cell",
        display.estate, display.prepared, display.tag
    );
    println!(
        "estate plan --estate {} --plans-dir plans --state-dir .cell",
        display.estate
    );
    println!(
        "estate apply --estate {} --state-dir .cell --require-plan --curator jason",
        display.estate
    );
    println!(
        "estate reconcile --estate {} --state-dir .cell",
        display.estate
    );
    println!("This command does not execute them.");
    println!("This print is not a live PASS. READY_FOR_LIVE_TEST: no.");
    println!("The factory does not claim it trained.");
    println!("The factory does not apply the estate.");
}

fn specialty_seat_lines(binding_id: &str, function: &str) -> [String; 2] {
    if binding_id == "local_slm" {
        [
            format!("Specialty seat: local_slm, class local, function {function}."),
            "binding_id stays local_slm. trained_shape gguf. auto_apply=false.".to_string(),
        ]
    } else {
        [
            format!("Specialty seat: {binding_id}, class local, function {function}."),
            format!(
                "binding_id {binding_id}. class local. A new id is added beside local_slm. An existing local id is replaced in place. trained_shape gguf. auto_apply=false."
            ),
        ]
    }
}

fn seat_handoff(req: &JourneyRequest<'_>, paths: &JourneyPaths, mode: HandoffMode) -> Result<()> {
    let binding_id = model_estate::resolve_portable_binding_id(req.binding_id)?;
    let display = handoff_display(req, paths, &binding_id);
    match mode {
        HandoffMode::Plan => {
            println!("import-trained handoff (planned):");
            println!("{}", display.line);
            println!("trained_shape gguf. auto_apply=false.");
            match specialist_gguf_status(&display.adapter_path) {
                SpecialistGguf::Ready => {
                    println!("This print does not write a proposal.");
                }
                SpecialistGguf::Missing => {
                    println!(
                        "specialist GGUF is not on disk. This print does not invent that file and does not write a proposal."
                    );
                }
                SpecialistGguf::Symlink => {
                    println!(
                        "specialist GGUF is a symlink. import-trained does not follow it. This print does not write a proposal."
                    );
                }
            }
            if req.import_trained {
                println!(
                    "--import-trained records only after --run when the specialist GGUF is a regular file. This print does not write a proposal."
                );
            }
            print_standing_next(&display);
            Ok(())
        }
        HandoffMode::AfterRun => finish_after_run(req, &display),
    }
}

fn finish_after_run(req: &JourneyRequest<'_>, display: &HandoffDisplay) -> Result<()> {
    let skip = match specialist_gguf_status(&display.adapter_path) {
        SpecialistGguf::Ready => None,
        SpecialistGguf::Missing => Some(format!(
            "specialist GGUF {} is missing. no proposal written.",
            display.adapter_path.display()
        )),
        SpecialistGguf::Symlink => Some(format!(
            "specialist GGUF {} is a symlink. import-trained does not follow it. no proposal written.",
            display.adapter_path.display()
        )),
    };
    if let Some(detail) = skip {
        if req.import_trained {
            bail!("refuse:classify-journey: {detail}");
        }
        println!("import-trained handoff skipped: {detail}");
        return Ok(());
    }
    println!("import-trained handoff:");
    println!("{}", display.line);
    println!("trained_shape gguf. auto_apply=false.");
    if req.import_trained {
        let estate = req.estate.filter(|path| !path.as_os_str().is_empty());
        let prepared = req.prepared.filter(|path| !path.as_os_str().is_empty());
        let tag = req.enrich_tag.map(str::trim).filter(|tag| !tag.is_empty());
        let (Some(estate), Some(prepared), Some(tag)) = (estate, prepared, tag) else {
            bail!(
                "refuse:classify-journey: --import-trained needs --estate, --prepared, and --enrich-tag. no proposal written."
            );
        };
        crate::enrich::cmd_enrich_import_trained(
            estate,
            prepared,
            tag,
            &display.adapter_path,
            "jason",
            Some(display.binding_id.as_str()),
        )?;
    } else {
        println!("This command prints the import-trained line and does not write a proposal.");
    }
    print_standing_next(display);
    Ok(())
}

fn execute(req: &JourneyRequest<'_>, paths: &JourneyPaths, llama: &LlamaCpp) -> Result<()> {
    let library = library_tag(req.base_tag);
    if !base_is_local_dir(req.base) {
        println!(
            "base_cache: {}",
            resolved_base_dir(req.base, paths).display()
        );
    }
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
            hf_bin: hf_bin_name().unwrap_or("hf"),
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
        if *name == "fetch-base" && !base_is_local_dir(req.base) {
            let dir = resolved_base_dir(req.base, paths);
            let force = step.action == StepAction::Run && redo_reason(&step.detail).is_some();
            match ensure_shared_hub_snapshot(req.base, &dir, force)? {
                HubFetch::Reused => println!("skip fetch-base"),
                HubFetch::Downloaded => {
                    if step.action == StepAction::Skip {
                        println!("redo fetch-base: shared snapshot failed validation");
                    } else if let Some(reason) = redo_reason(&step.detail) {
                        println!("redo fetch-base: {reason}");
                    } else {
                        println!("run fetch-base");
                    }
                    write_manifest(
                        &paths.manifest_path("fetch-base"),
                        &fetch_inputs(req.base, &dir),
                        None,
                    )?;
                }
            }
            continue;
        }
        if step.action == StepAction::Skip {
            println!("skip {}", step.name);
            if *name == "fetch-base" {
                let dir = resolved_base_dir(req.base, paths);
                validate_snapshot(&dir)?;
            }
            continue;
        }
        if let Some(reason) = redo_reason(&step.detail) {
            println!("redo {}: {reason}", step.name);
        } else {
            println!("run {}", step.name);
        }
        match step.name {
            "prepare" => {
                if let Some(dataset) = req.import_dataset {
                    let preset = crate::classify_import::preset_by_name(dataset)?;
                    let size = crate::classify_import::parse_split_size(req.train_size)?;
                    let import_dir =
                        journey_import_dir(preset.alias, &size.token(), req.seed, req.expand_tag)?;
                    if req.expand_tag.is_some() {
                        if !import_dir.join("train.jsonl").is_file()
                            || !import_dir.join("heldout.jsonl").is_file()
                        {
                            bail!(
                                "refuse:classify-journey: expand cache {} is missing train.jsonl or heldout.jsonl. Run classify expand --run --tag first.",
                                import_dir.display()
                            );
                        }
                    } else {
                        crate::classify_import::cmd_classify_import(
                            &crate::classify_import::ImportRequest {
                                dataset: preset.alias,
                                train_size: req.train_size,
                                heldout_size: req.heldout_size,
                                seed: req.seed,
                                out: &import_dir,
                                force: req.force,
                                native_train: None,
                                native_test: None,
                                from_local: req.from_local,
                                fetch: req.import_fetch,
                                python: req.python,
                                cache_root: None,
                            },
                        )?;
                    }
                    cmd_classify_prepare_presplit(
                        &import_dir.join("train.jsonl"),
                        &import_dir.join("heldout.jsonl"),
                        &paths.out,
                        DatasetFormat::Sharegpt,
                        req.dataset_name,
                        req.force || ran_prepare || paths.dataset_jsonl.is_file(),
                    )?;
                } else {
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
                }
                let prepare = if req.import_dataset.is_some() {
                    dataset_prepare_key(req, paths)?
                } else {
                    prepare_inputs(req.input, req.seed, req.held_out_ratio)?
                };
                write_manifest(&paths.manifest_path("prepare"), &prepare, None)?;
                ran_prepare = true;
            }
            "fetch-base" => {
                let dir = resolved_base_dir(req.base, paths);
                println!("local base {}", dir.display());
                validate_snapshot(&dir)?;
                write_manifest(
                    &paths.manifest_path("fetch-base"),
                    &fetch_inputs(req.base, &dir),
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
            "export-repair" => {
                let base_dir = resolved_base_dir(req.base, paths);
                let report = crate::export_repair::repair_export(&base_dir, &paths.export_dir)?;
                println!("{}", report.summary());
                write_step_manifest(req, paths, "export-repair", None)?;
                let manifest = fs::read_to_string(paths.manifest_path("export-repair"))?;
                let mut value: Value = serde_json::from_str(&manifest)?;
                value["repaired_tensors"] = json!(report.tensors);
                value["repaired_tensor_count"] = json!(report.tensors.len());
                value["copied_files"] = json!(report.files);
                fs::write(
                    paths.manifest_path("export-repair"),
                    format!("{}\n", serde_json::to_string_pretty(&value)?),
                )?;
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
                    None,
                )?;
                write_pipeline_manifest(req, paths, "eval-base")?;
            }
            "eval-base-few-shot" => {
                let model = library.unwrap_or(req.built_base_tag);
                let shot = crate::classify::FewShot {
                    n: paths.few_shot,
                    exemplars: &paths.dataset_jsonl,
                    seed: paths.few_shot_seed,
                };
                cmd_classify_eval(
                    &paths.heldout,
                    Some(req.endpoint),
                    model,
                    None,
                    &paths.few_shot_report,
                    false,
                    false,
                    req.timeout_secs,
                    EvalApi::OllamaNative,
                    EvalGate::Journey,
                    Some(&shot),
                )?;
                write_pipeline_manifest(req, paths, "eval-base-few-shot")?;
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
                    None,
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

fn with_repair(base: &str, repair: Option<&str>) -> String {
    match repair {
        Some(repair) => sha256_text(&format!("depends-export-repair\n{base}\n{repair}")),
        None => base.to_string(),
    }
}

fn step_manifest_key(inputs: &Inputs, step: &str) -> Result<String> {
    let key = match step {
        "recipe" => inputs.recipe.clone(),
        "export-repair" => inputs.repair.clone(),
        "eval-base" => inputs.eval.clone(),
        "eval-specialist" => inputs
            .eval
            .as_deref()
            .map(|key| with_repair(key, inputs.repair.as_deref())),
        "ollama-create-base" => inputs.ollama_base.clone(),
        "ollama-create-specialist" => inputs
            .ollama_spec
            .as_deref()
            .map(|key| with_repair(key, inputs.repair.as_deref())),
        "gguf-convert-specialist" | "quantize-specialist" => inputs
            .pipeline
            .as_deref()
            .map(|key| with_repair(key, inputs.repair.as_deref())),
        _ => inputs.pipeline.clone(),
    };
    key.ok_or_else(|| anyhow::anyhow!("refuse:classify-journey: cannot hash journey inputs"))
}

fn write_step_manifest(
    req: &JourneyRequest<'_>,
    paths: &JourneyPaths,
    step: &str,
    gguf_sha256: Option<&str>,
) -> Result<()> {
    let inputs = load_inputs(req, paths)?;
    let key = if step == "eval-base-few-shot" {
        few_shot_manifest_key(
            inputs.eval.as_deref(),
            paths.few_shot,
            paths.few_shot_seed,
            &paths.dataset_jsonl,
        )
    } else {
        step_manifest_key(&inputs, step).ok()
    };
    let key =
        key.ok_or_else(|| anyhow::anyhow!("refuse:classify-journey: cannot hash journey inputs"))?;
    write_manifest(&paths.manifest_path(step), &key, gguf_sha256)
}

fn write_pipeline_manifest(
    req: &JourneyRequest<'_>,
    paths: &JourneyPaths,
    step: &str,
) -> Result<()> {
    write_step_manifest(req, paths, step, None)
}

fn probe_ollama_load(endpoint: &str, tag: &str, gguf: &Path, timeout_secs: u64) -> Result<()> {
    let base = endpoint.trim().trim_end_matches('/');
    let url = if base.ends_with("/api/chat") {
        base.to_string()
    } else {
        format!("{base}/api/chat")
    };
    let body = json!({
        "model": tag,
        "messages": [{"role": "user", "content": "."}],
        "stream": false,
        "think": false,
        "options": {"num_predict": 1}
    });
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(timeout_secs.max(1)))
        .build();
    let failure = match agent.post(&url).send_json(body) {
        Ok(resp) => {
            let text = resp.into_string().unwrap_or_default();
            match serde_json::from_str::<Value>(&text) {
                Ok(value) => value
                    .get("error")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                Err(_) => None,
            }
        }
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            let detail = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|value| {
                    value
                        .get("error")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .filter(|detail| !detail.is_empty())
                .unwrap_or(text);
            Some(format!("HTTP {code} {detail}"))
        }
        Err(err) => Some(err.to_string()),
    };
    if let Some(detail) = failure {
        let detail = detail.trim();
        bail!(
            "refuse:classify-journey: ollama load probe failed for {tag} GGUF {}: {detail}",
            gguf.display()
        );
    }
    Ok(())
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
    probe_ollama_load(req.endpoint, tag, &gguf_abs, req.timeout_secs)?;
    println!("load probe ok {tag}");
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

fn ci_field(interval: Option<(f64, f64)>) -> Value {
    match interval {
        Some((low, high)) => json!({"low": low, "high": high, "method": "wilson"}),
        None => Value::Null,
    }
}

fn delta_ci_field(comparison: &Comparison) -> Value {
    match delta_ci95(comparison) {
        Some((_, low, high)) => json!({"low": low, "high": high, "method": "newcombe-wilson"}),
        None => Value::Null,
    }
}

/// Why the significant-lift gate passed, failed, or was left off.
pub fn lift_gate_field(comparison: &Comparison, required: bool) -> Value {
    let ci = delta_ci95(comparison);
    let significant = ci.as_ref().is_some_and(|(_, low, _)| *low > 0.0);
    let verdict = if !required {
        "unset"
    } else if significant {
        "pass"
    } else {
        "fail"
    };
    json!({
        "required": required,
        "rule": "newcombe_ci95_lower_gt_0",
        "significant": significant,
        "verdict": verdict,
        "delta": ci.as_ref().map(|(delta, _, _)| *delta),
        "low": ci.as_ref().map(|(_, low, _)| *low),
        "high": ci.as_ref().map(|(_, _, high)| *high),
    })
}

fn write_comparison(
    req: &JourneyRequest<'_>,
    paths: &JourneyPaths,
    template: &str,
    library: Option<&str>,
    comparison: &Comparison,
    few_shot: Option<&SideScore>,
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
            JourneyPreset::Qwen => "qwen",
            JourneyPreset::DeepseekR1Distill => "deepseek-r1-distill",
            JourneyPreset::Glm4Chat => "glm4-chat",
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
        "few_shot_base_accuracy": few_shot.map(|score| score.accuracy),
        "delta_vs_few_shot_base": few_shot.map(|score| comparison.specialist_accuracy - score.accuracy),
        "few_shot": if paths.few_shot == 0 {
            Value::Null
        } else {
            json!({
                "n": paths.few_shot,
                "exemplar_source": paths.dataset_jsonl.display().to_string(),
                "seed": paths.few_shot_seed,
                "report": paths.few_shot_report.display().to_string(),
            })
        },
        "base_accuracy_ci95": ci_field(wilson_ci95(comparison.base_correct, comparison.base_records)),
        "specialist_accuracy_ci95": ci_field(wilson_ci95(comparison.specialist_correct, comparison.specialist_records)),
        "delta_ci95": delta_ci_field(comparison),
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
        "dataset": req.import_dataset,
        "train_size": if req.import_dataset.is_some() { json!(req.train_size) } else { Value::Null },
        "heldout_size": if req.import_dataset.is_some() { json!(req.heldout_size) } else { Value::Null },
        "import_seed": if req.import_dataset.is_some() { json!(req.seed) } else { Value::Null },
        "min_delta": req.min_delta,
        "min_accuracy": req.min_accuracy,
        "require_significant_lift": req.require_significant_lift,
        "lift_gate": lift_gate_field(comparison, req.require_significant_lift),
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
        "suffix": "qwen"
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

fn run_hf_download_quiet(argv: &[String]) -> Result<()> {
    let (bin, args) = argv
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-journey: empty command"))?;
    let bin_path = which(bin)
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-journey: {bin} is not on PATH"))?;
    let mut child = Command::new(&bin_path)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            anyhow::anyhow!(
                "refuse:classify-journey: cannot run {}: {e}",
                bin_path.display()
            )
        })?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-journey: {bin} stdout is closed"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("refuse:classify-journey: {bin} stderr is closed"))?;
    let out_task = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = std::io::stdout().write_all(&buf);
        buf
    });
    let err_task = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        let _ = std::io::stderr().write_all(&buf);
        buf
    });
    let status = child.wait().map_err(|e| {
        anyhow::anyhow!(
            "refuse:classify-journey: cannot wait for {}: {e}",
            bin_path.display()
        )
    })?;
    let out_bytes = out_task.join().unwrap_or_default();
    let err_bytes = err_task.join().unwrap_or_default();
    if !status.success() {
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out_bytes),
            String::from_utf8_lossy(&err_bytes)
        );
        if hf_downloader_deprecated(&combined) {
            bail!(
                "refuse:classify-journey: {bin} is deprecated and no longer downloads. {HF_DEPRECATION_HINT}."
            );
        }
        bail!(
            "refuse:classify-journey: {} exited {status}",
            bin_path.display()
        );
    }
    Ok(())
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

/// `--dual` trains qwen and glm4-chat on one rust_idiom expand cache.
pub struct DualJourneyRequest<'a> {
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
    pub require_significant_lift: bool,
    pub timeout_secs: u64,
    pub together_poll_secs: u64,
    pub train_driver: TrainDriver,
    pub together_model: Option<&'a str>,
    pub together_base_url: &'a str,
    pub api_key_env: Option<&'a str>,
    pub preset: JourneyPreset,
    pub import_dataset: Option<&'a str>,
    pub train_size: &'a str,
    pub heldout_size: &'a str,
    pub from_local: Option<&'a Path>,
    pub import_fetch: crate::classify_import::ImportFetch,
    pub python: Option<&'a str>,
    pub base_cache: &'a Path,
    pub few_shot: u32,
    pub expand_tag: Option<&'a str>,
    pub modest: bool,
    pub modest_note: &'a str,
    pub estate: Option<&'a Path>,
    pub prepared: Option<&'a Path>,
    pub enrich_tag: Option<&'a str>,
    pub import_trained: bool,
    /// Portable local binding id printed on each student's handoff. `None` keeps `local_slm`.
    pub binding_id: Option<&'a str>,
}

/// Scored base-vs-specialist counts for one student. Absent on `--print`.
#[derive(Clone, Debug)]
pub struct DualPresetScore {
    pub base_accuracy: f64,
    pub specialist_accuracy: f64,
    pub delta: f64,
    pub base_correct: u64,
    pub base_records: u64,
    pub specialist_correct: u64,
    pub specialist_records: u64,
}

struct DualSide {
    preset: JourneyPreset,
    dir_name: &'static str,
    base: String,
    tag: String,
    built_base_tag: &'static str,
    seat: SeatChat,
    llama_note: &'static str,
    together_model: String,
    out: PathBuf,
    dataset_name: String,
}

fn preset_token(preset: JourneyPreset) -> &'static str {
    match preset {
        JourneyPreset::Qwen => "qwen",
        JourneyPreset::DeepseekR1Distill => "deepseek-r1-distill",
        JourneyPreset::Glm4Chat => "glm4-chat",
    }
}

/// Parent directory for the two preset outs and `dual-compare.json`.
fn dual_parent_out(out: &Path, train_size: &str, expand_tag: &str) -> Result<PathBuf> {
    let (_, layout_out, _) = apply_dataset_layout(
        "rust_idiom",
        train_size,
        DEFAULT_TAG,
        DEFAULT_TAG,
        Path::new(DEFAULT_JOURNEY_OUT),
        DEFAULT_DATASET,
        Some(expand_tag),
    )?;
    if out == Path::new(DEFAULT_JOURNEY_OUT) {
        Ok(PathBuf::from(format!("{}-dual", layout_out.display())))
    } else {
        Ok(out.to_path_buf())
    }
}

fn validate_dual(req: &DualJourneyRequest<'_>) -> Result<String> {
    if req.print && req.run {
        bail!("refuse:classify-journey: pass only one of --print and --run");
    }
    if req.preset == JourneyPreset::DeepseekR1Distill {
        bail!(
            "refuse:classify-journey: --dual trains qwen and glm4-chat on one expand cache. DeepSeek is not in this pair"
        );
    }
    if req.train_driver == TrainDriver::Together
        || req
            .together_model
            .map(str::trim)
            .is_some_and(|model| !model.is_empty())
    {
        bail!(
            "refuse:classify-journey: --dual trains local llamafactory-cli students. Together stays on a single preset"
        );
    }
    if req.base != DEFAULT_BASE {
        bail!(
            "refuse:classify-journey: --dual keeps {DEFAULT_BASE} and {GLM4_CHAT_BASE}. Omit --base"
        );
    }
    if req.tag != DEFAULT_TAG {
        bail!(
            "refuse:classify-journey: --dual keeps the qwen and glm4-chat specialist tags. Omit --tag"
        );
    }
    if library_tag(req.base_tag).is_some() {
        bail!("refuse:classify-journey: --dual builds each base. Omit --base-tag");
    }
    let dataset = req.import_dataset.ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-journey: --dual needs --dataset rust_idiom and --expand-tag"
        )
    })?;
    let alias = crate::classify_import::preset_by_name(dataset)?.alias;
    if alias != "rust_idiom" {
        bail!("refuse:classify-journey: --dual reads the rust_idiom expand cache");
    }
    let tag = req.expand_tag.ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-journey: --dual needs --expand-tag on the rust_idiom cache"
        )
    })?;
    crate::classify_expand::validate_expand_tag(tag)
}

fn dual_side(
    preset: JourneyPreset,
    parent: &Path,
    train_size: &str,
    expand_tag: &str,
    dataset_name: &str,
) -> Result<DualSide> {
    let applied = apply_preset(preset, DEFAULT_BASE, DEFAULT_TAG, TrainDriver::Local, None)?;
    let (tag, _, dataset_name) = apply_dataset_layout(
        "rust_idiom",
        train_size,
        &applied.tag,
        DEFAULT_TAG,
        Path::new(DEFAULT_JOURNEY_OUT),
        dataset_name,
        Some(expand_tag),
    )?;
    let dir_name = preset_token(preset);
    Ok(DualSide {
        preset,
        dir_name,
        base: applied.base,
        tag,
        built_base_tag: applied.built_base_tag,
        seat: applied.seat,
        llama_note: applied.llama_note,
        together_model: applied.together_model,
        out: parent.join(dir_name),
        dataset_name,
    })
}

fn dual_journey_request<'a>(
    req: &'a DualJourneyRequest<'a>,
    side: &'a DualSide,
    run: bool,
) -> JourneyRequest<'a> {
    JourneyRequest {
        input: req.input,
        out: &side.out,
        base: &side.base,
        base_tag: None,
        tag: &side.tag,
        endpoint: req.endpoint,
        dataset_name: &side.dataset_name,
        seed: req.seed,
        held_out_ratio: req.held_out_ratio,
        max_steps: req.max_steps,
        quant: req.quant,
        llama_cpp_dir: req.llama_cpp_dir,
        force: req.force,
        print: !run,
        run,
        min_delta: req.min_delta,
        min_accuracy: req.min_accuracy,
        require_significant_lift: req.require_significant_lift,
        timeout_secs: req.timeout_secs,
        together_poll_secs: req.together_poll_secs,
        train_driver: TrainDriver::Local,
        together_model: &side.together_model,
        together_base_url: req.together_base_url,
        api_key_env: req.api_key_env,
        built_base_tag: side.built_base_tag,
        seat: side.seat,
        llama_note: side.llama_note,
        preset: side.preset,
        import_dataset: Some("rust_idiom"),
        train_size: req.train_size,
        heldout_size: req.heldout_size,
        from_local: req.from_local,
        import_fetch: req.import_fetch,
        python: req.python,
        base_cache: req.base_cache,
        few_shot: req.few_shot,
        expand_tag: req.expand_tag,
        estate: req.estate,
        prepared: req.prepared,
        enrich_tag: req.enrich_tag,
        import_trained: req.import_trained,
        binding_id: req.binding_id,
    }
}

fn journey_plan_value(req: &JourneyRequest<'_>, cache: &Path, held_sha: &str) -> Result<Value> {
    let mut paths = JourneyPaths::new(req.out);
    paths.base_cache = req.base_cache.to_path_buf();
    paths.few_shot = req.few_shot;
    paths.few_shot_seed = req.seed;
    let template = train_template(req.base);
    let inputs = load_inputs(req, &paths)?;
    let llama = match req.llama_cpp_dir {
        Some(dir) => resolve_llama_cpp_note(dir, req.llama_note).ok(),
        None => None,
    };
    let ctx = PlanCtx {
        paths: &paths,
        base: req.base,
        hf_bin: hf_bin_name().unwrap_or("hf"),
        quant: req.quant,
        library_tag: None,
        built_base_tag: req.built_base_tag,
        specialist_tag: req.tag,
        llama: llama.as_ref(),
        inputs: &inputs,
        check_ollama: false,
        train_driver: req.train_driver,
        together_model: req.together_model,
    };
    let steps = plan_with(&ctx);
    let steps: Vec<Value> = steps
        .iter()
        .map(|step| {
            let detail = if step.name == "recipe" {
                match req.max_steps {
                    Some(steps) => format!("{} max_steps: {steps}", step.detail),
                    None => step.detail.clone(),
                }
            } else {
                step.detail.clone()
            };
            json!({
                "name": step.name,
                "action": match step.action {
                    StepAction::Run => "run",
                    StepAction::Skip => "skip",
                },
                "detail": detail,
            })
        })
        .collect();
    Ok(json!({
        "schema": "cell-one.classify-journey-plan.v0",
        "preset": preset_token(req.preset),
        "base": req.base,
        "template": template,
        "specialist_tag": req.tag,
        "built_base_tag": req.built_base_tag,
        "out": paths.out.display().to_string(),
        "cache": cache.display().to_string(),
        "heldout": cache.join("heldout.jsonl").display().to_string(),
        "heldout_sha256": held_sha,
        "steps": steps,
        "train_size": req.train_size,
        "max_steps": match req.max_steps {
            Some(steps) => json!(steps),
            None => Value::Null,
        },
        "would_train": false,
        "network": false,
        "live_pass_recorded": false,
        "ready_for_live_test": "no",
        "note": "Print plan only. This file does not train and is not a factory live PASS. READY_FOR_LIVE_TEST stays no."
    }))
}

fn score_field(score: Option<&DualPresetScore>, field: &str) -> Value {
    let Some(score) = score else {
        return Value::Null;
    };
    match field {
        "base_accuracy" => json!(score.base_accuracy),
        "specialist_accuracy" => json!(score.specialist_accuracy),
        "delta" => json!(score.delta),
        "base_correct" => json!(score.base_correct),
        "base_records" => json!(score.base_records),
        "specialist_correct" => json!(score.specialist_correct),
        "specialist_records" => json!(score.specialist_records),
        _ => Value::Null,
    }
}

fn preset_block(side: &DualSide, score: Option<&DualPresetScore>) -> Value {
    json!({
        "preset": side.dir_name,
        "out": side.out.display().to_string(),
        "base": side.base,
        "template": train_template(&side.base),
        "specialist_tag": side.tag,
        "comparison": side.out.join("comparison.json").display().to_string(),
        "base_accuracy": score_field(score, "base_accuracy"),
        "specialist_accuracy": score_field(score, "specialist_accuracy"),
        "delta": score_field(score, "delta"),
        "base_correct": score_field(score, "base_correct"),
        "base_records": score_field(score, "base_records"),
        "specialist_correct": score_field(score, "specialist_correct"),
        "specialist_records": score_field(score, "specialist_records"),
    })
}

/// Qwen specialist accuracy minus GLM specialist accuracy, with a Newcombe interval when both scores exist.
fn qwen_minus_glm(qwen: &DualPresetScore, glm: &DualPresetScore) -> Value {
    let delta = qwen.specialist_accuracy - glm.specialist_accuracy;
    let ci = newcombe_delta_ci95(
        glm.specialist_correct,
        glm.specialist_records,
        qwen.specialist_correct,
        qwen.specialist_records,
    );
    json!({
        "specialist_accuracy": delta,
        "ci95": match ci {
            Some((_, low, high)) => json!({"low": low, "high": high, "method": "newcombe-wilson"}),
            None => Value::Null,
        }
    })
}

fn dual_report_value(
    mode: &str,
    cache: &Path,
    held_sha: &str,
    expand_tag: &str,
    train_size: &str,
    seed: u64,
    qwen: &DualSide,
    glm: &DualSide,
    qwen_score: Option<&DualPresetScore>,
    glm_score: Option<&DualPresetScore>,
    prepared_sha: Option<&str>,
    modest: bool,
    max_steps: Option<u32>,
    modest_note: &str,
) -> Value {
    let cross = match (qwen_score, glm_score) {
        (Some(qwen), Some(glm)) => json!(qwen.specialist_accuracy - glm.specialist_accuracy),
        _ => Value::Null,
    };
    let cross_detail = match (qwen_score, glm_score) {
        (Some(qwen), Some(glm)) => qwen_minus_glm(qwen, glm),
        _ => Value::Null,
    };
    let note = match mode {
        "in-progress" => "Scores stay absent until both students finish. A prior compare was cleared. This file is not a completed comparison and is not a factory live PASS. READY_FOR_LIVE_TEST stays no.",
        _ => "Local dual comparison of Qwen (qwen) and GLM-4 Chat on one rust_idiom expand cache. This file is not a factory live PASS. READY_FOR_LIVE_TEST stays no.",
    };
    json!({
        "schema": "cell-one.classify-journey-dual.v0",
        "mode": mode,
        "dataset": "rust_idiom",
        "expand_tag": expand_tag,
        "train_size": train_size,
        "max_steps": match max_steps {
            Some(steps) => json!(steps),
            None => Value::Null,
        },
        "modest": modest,
        "modest_note": if modest { json!(modest_note) } else { Value::Null },
        "seed": seed,
        "cache": cache.display().to_string(),
        "heldout": cache.join("heldout.jsonl").display().to_string(),
        "heldout_sha256": held_sha,
        "holdout_shared": true,
        "prepared_heldout_sha256": prepared_sha,
        "students": ["qwen", "glm4-chat"],
        "presets": {
            "qwen": preset_block(qwen, qwen_score),
            "glm4-chat": preset_block(glm, glm_score),
        },
        "qwen_glm_specialist_delta": cross,
        "qwen_minus_glm": cross_detail,
        "factory_live_pass": false,
        "live_pass_recorded": false,
        "ready_for_live_test": "no",
        "note": note
    })
}

fn write_dual_report(path: &Path, report: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(report)?))?;
    Ok(())
}

/// Drop a prior print stub or scored compare before either student starts.
/// A mid-pair failure must not leave that file readable as the current result.
fn mark_dual_compare_started(
    compare_path: &Path,
    cache: &Path,
    held_sha: &str,
    expand_tag: &str,
    train_size: &str,
    seed: u64,
    qwen: &DualSide,
    glm: &DualSide,
    modest: bool,
    max_steps: Option<u32>,
    modest_note: &str,
) -> Result<()> {
    if compare_path.is_file() {
        fs::remove_file(compare_path)?;
    }
    let started = dual_report_value(
        "in-progress",
        cache,
        held_sha,
        expand_tag,
        train_size,
        seed,
        qwen,
        glm,
        None,
        None,
        None,
        modest,
        max_steps,
        modest_note,
    );
    write_dual_report(compare_path, &started)
}

/// `--modest` gauge. `all` (the clap default) becomes `500`. A count above 500 is refused.
/// An omitted `--max-steps` becomes 50. An explicit `--max-steps` is kept.
pub fn apply_modest_gauge(
    train_size: &str,
    max_steps: Option<u32>,
) -> Result<(String, u32, &'static str)> {
    let parsed = crate::classify_import::parse_split_size(train_size)?;
    let (resolved, replaced_all) = match parsed {
        crate::classify_import::SplitSize::All => (MODEST_TRAIN_SIZE.to_string(), true),
        crate::classify_import::SplitSize::Count(n) if n > MODEST_TRAIN_CAP => {
            bail!(
                "refuse:classify-journey: --modest caps --train-size at {MODEST_TRAIN_CAP}. Omit --modest for a larger gauge."
            );
        }
        crate::classify_import::SplitSize::Count(n) => (n.to_string(), false),
    };
    let defaulted_steps = max_steps.is_none();
    let steps = max_steps.unwrap_or(MODEST_MAX_STEPS);
    let note = match (replaced_all, defaulted_steps) {
        (true, true) => "replaced train_size all with 500; max_steps 50",
        (true, false) => "replaced train_size all with 500; explicit max_steps kept",
        (false, true) => "explicit train_size at or under 500; max_steps 50",
        (false, false) => "explicit train_size at or under 500; explicit max_steps kept",
    };
    Ok((resolved, steps, note))
}

fn score_from_out(out: &Path) -> Result<(DualPresetScore, String)> {
    let base = side_score(&read_json(&out.join("base-report.json"))?)?;
    let specialist = side_score(&read_json(&out.join("specialist-report.json"))?)?;
    let comparison = compare_sides(&base, &specialist);
    let held = file_sha(&out.join("heldout.jsonl")).ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-journey: missing held-out file in {}",
            out.display()
        )
    })?;
    Ok((
        DualPresetScore {
            base_accuracy: comparison.base_accuracy,
            specialist_accuracy: comparison.specialist_accuracy,
            delta: comparison.delta,
            base_correct: comparison.base_correct,
            base_records: comparison.base_records,
            specialist_correct: comparison.specialist_correct,
            specialist_records: comparison.specialist_records,
        },
        held,
    ))
}

pub fn cmd_classify_journey_dual(req: &DualJourneyRequest<'_>) -> Result<()> {
    model_estate::resolve_portable_binding_id(req.binding_id)?;
    let expand_tag = validate_dual(req)?;
    let size = crate::classify_import::parse_split_size(req.train_size)?;
    let cache = crate::classify_import::expand_cache_dir(
        "rust_idiom",
        &size.token(),
        req.seed,
        &expand_tag,
    );
    if !cache.join("train.jsonl").is_file() || !cache.join("heldout.jsonl").is_file() {
        bail!(
            "refuse:classify-journey: expand cache {} is missing train.jsonl or heldout.jsonl. Run classify expand --run --tag first.",
            cache.display()
        );
    }
    let held_sha = file_sha(&cache.join("heldout.jsonl")).ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-journey: cannot hash {}",
            cache.join("heldout.jsonl").display()
        )
    })?;
    let parent = dual_parent_out(req.out, req.train_size, &expand_tag)?;
    let qwen = dual_side(
        JourneyPreset::Qwen,
        &parent,
        req.train_size,
        &expand_tag,
        req.dataset_name,
    )?;
    let glm = dual_side(
        JourneyPreset::Glm4Chat,
        &parent,
        req.train_size,
        &expand_tag,
        req.dataset_name,
    )?;
    let compare_path = parent.join("dual-compare.json");
    if !req.run {
        println!("classify journey dual: print");
        println!(
            "students: qwen glm4-chat cache: {} heldout_sha256: {held_sha} no-import dry-run no teacher",
            cache.display()
        );
        if req.modest {
            println!(
                "modest: true train_size: {} max_steps: {} {}",
                req.train_size,
                req.max_steps
                    .map(|steps| steps.to_string())
                    .unwrap_or_else(|| "unset".into()),
                req.modest_note
            );
        }
        mark_dual_compare_started(
            &compare_path,
            &cache,
            &held_sha,
            &expand_tag,
            req.train_size,
            req.seed,
            &qwen,
            &glm,
            req.modest,
            req.max_steps,
            req.modest_note,
        )?;
        println!(
            "dual-compare: {} in-progress. Scores stay absent until both students finish. This print does not train. It is not a factory live PASS. READY_FOR_LIVE_TEST: no.",
            compare_path.display()
        );
        for side in [&qwen, &glm] {
            let journey = dual_journey_request(req, side, false);
            let plan = journey_plan_value(&journey, &cache, &held_sha)?;
            if let Some(dir) = journey.out.parent() {
                fs::create_dir_all(dir)?;
            }
            fs::create_dir_all(journey.out)?;
            fs::write(
                journey.out.join("journey-plan.json"),
                format!("{}\n", serde_json::to_string_pretty(&plan)?),
            )?;
            cmd_classify_journey(&journey)?;
        }
        let report = dual_report_value(
            "print",
            &cache,
            &held_sha,
            &expand_tag,
            req.train_size,
            req.seed,
            &qwen,
            &glm,
            None,
            None,
            None,
            req.modest,
            req.max_steps,
            req.modest_note,
        );
        write_dual_report(&compare_path, &report)?;
        println!(
            "dual-compare: {} This print does not train. It is not a factory live PASS. READY_FOR_LIVE_TEST: no.",
            compare_path.display()
        );
        return Ok(());
    }
    if req.import_trained {
        bail!(
            "refuse:classify-journey: --dual prints each student's import-trained handoff and Standing next. It does not record a proposal. Omit --import-trained."
        );
    }
    println!("classify journey dual: run");
    println!(
        "students: qwen glm4-chat cache: {} heldout_sha256: {held_sha}",
        cache.display()
    );
    mark_dual_compare_started(
        &compare_path,
        &cache,
        &held_sha,
        &expand_tag,
        req.train_size,
        req.seed,
        &qwen,
        &glm,
        req.modest,
        req.max_steps,
        req.modest_note,
    )?;
    if req.modest {
        println!(
            "modest: true train_size: {} max_steps: {} {}",
            req.train_size,
            req.max_steps
                .map(|steps| steps.to_string())
                .unwrap_or_else(|| "unset".into()),
            req.modest_note
        );
    }
    println!(
        "dual-compare: {} in-progress. Scores stay absent until both students finish. This is not a factory live PASS. READY_FOR_LIVE_TEST: no.",
        compare_path.display()
    );
    for side in [&qwen, &glm] {
        let journey = dual_journey_request(req, side, true);
        cmd_classify_journey(&journey)?;
    }
    let (qwen_score, qwen_held) = score_from_out(&qwen.out)?;
    let (glm_score, glm_held) = score_from_out(&glm.out)?;
    if qwen_held != glm_held {
        bail!(
            "refuse:classify-journey: qwen and glm4-chat held-out files differ ({qwen_held} vs {glm_held})"
        );
    }
    let report = dual_report_value(
        "run",
        &cache,
        &held_sha,
        &expand_tag,
        req.train_size,
        req.seed,
        &qwen,
        &glm,
        Some(&qwen_score),
        Some(&glm_score),
        Some(&qwen_held),
        req.modest,
        req.max_steps,
        req.modest_note,
    );
    write_dual_report(&compare_path, &report)?;
    println!(
        "dual-compare: {} Local output only. This is not a factory live PASS. READY_FOR_LIVE_TEST: no.",
        compare_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modest_gauge_replaces_all_caps_larger_sizes_and_defaults_max_steps() {
        let (size, steps, note) = apply_modest_gauge("all", None).unwrap();
        assert_eq!(size, "500");
        assert_eq!(steps, 50);
        assert!(note.contains("500"), "{note}");
        let (size, steps, note) = apply_modest_gauge("200", None).unwrap();
        assert_eq!(size, "200");
        assert_eq!(steps, 50);
        assert!(note.contains("explicit train_size"), "{note}");
        let (size, steps, _) = apply_modest_gauge("all", Some(10)).unwrap();
        assert_eq!(size, "500");
        assert_eq!(steps, 10);
        let err = apply_modest_gauge("501", None).unwrap_err().to_string();
        assert!(err.contains("caps --train-size at 500"), "{err}");
        let err = apply_modest_gauge("0", None).unwrap_err().to_string();
        assert!(err.contains("positive integer"), "{err}");
    }

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
            fetch: fetch_inputs(DEFAULT_BASE, &resolved_base_dir(DEFAULT_BASE, &paths)),
            pipeline: None,
            repair: None,
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
        assert!(base.detail.contains(DEFAULT_BASE_CACHE), "{base:?}");
        assert!(
            base.detail.contains(&sanitize_hub_id(DEFAULT_BASE)),
            "{base:?}"
        );
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
        let mut paths = JourneyPaths::new(&dir);
        paths.base_cache = dir.join("base-cache");
        let shared = resolved_base_dir(DEFAULT_BASE, &paths);
        fs::create_dir_all(&shared).unwrap();
        fs::write(shared.join("config.json"), "{}\n").unwrap();
        fs::write(
            shared.join(HUB_COMPLETE_MARKER),
            "{\"files\":[{\"path\":\"config.json\",\"bytes\":3}]}\n",
        )
        .unwrap();
        fs::create_dir_all(&paths.adapter_dir).unwrap();
        fs::create_dir_all(&paths.export_dir).unwrap();
        fs::write(&paths.dataset_jsonl, "row\n").unwrap();
        fs::write(&paths.heldout, "{}\n").unwrap();
        fs::write(&paths.dataset_info, "{}\n").unwrap();
        fs::write(&paths.recipe, "dataset: classify_decisions\n").unwrap();
        fs::write(&paths.export_yaml, "export_dir: x\n").unwrap();
        fs::write(paths.adapter_dir.join("adapter_config.json"), "{}\n").unwrap();
        fs::write(paths.export_dir.join("config.json"), "{}\n").unwrap();
        fs::write(&paths.base_f16, "base-f16").unwrap();
        fs::write(&paths.specialist_f16, "spec-f16").unwrap();
        let seated_b = paths.seated_gguf("base", DEFAULT_QUANT);
        let seated_s = paths.seated_gguf("specialist", DEFAULT_QUANT);
        fs::write(&seated_b, "base-q").unwrap();
        fs::write(&seated_s, "spec-q").unwrap();
        fs::write(&paths.base_report, "{\"complete\": true}\n").unwrap();
        fs::write(&paths.specialist_report, "{\"complete\": true}\n").unwrap();
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
        let base_model = journey_modelfile(&seated_b, SeatChat::Qwen35);
        let spec_model = journey_modelfile(&seated_s, SeatChat::Qwen35);
        let eval_key = eval_inputs(
            &pipeline,
            DEFAULT_TAG,
            endpoint,
            None,
            DEFAULT_BUILT_BASE_TAG,
            &base_model,
            &spec_model,
        );
        let ollama_base = ollama_inputs(
            &pipeline,
            DEFAULT_BUILT_BASE_TAG,
            DEFAULT_TAG,
            endpoint,
            None,
            DEFAULT_BUILT_BASE_TAG,
            &base_model,
        );
        let ollama_spec = ollama_inputs(
            &pipeline,
            DEFAULT_TAG,
            DEFAULT_TAG,
            endpoint,
            None,
            DEFAULT_BUILT_BASE_TAG,
            &spec_model,
        );
        let repair = "repair-key".to_string();
        let spec_convert = with_repair(&pipeline, Some(&repair));
        let spec_eval = with_repair(&eval_key, Some(&repair));
        let spec_ollama = with_repair(&ollama_spec, Some(&repair));
        for step in [
            "prepare",
            "fetch-base",
            "recipe",
            "train",
            "merge-export",
            "export-repair",
            "gguf-convert-base",
            "gguf-convert-specialist",
            "quantize-base",
            "quantize-specialist",
            "eval-base",
            "eval-specialist",
        ] {
            let fetch_key = fetch_inputs(DEFAULT_BASE, &resolved_base_dir(DEFAULT_BASE, &paths));
            let key = match step {
                "prepare" => prepare.as_str(),
                "fetch-base" => fetch_key.as_str(),
                "recipe" => recipe_key.as_str(),
                "export-repair" => repair.as_str(),
                "eval-base" => eval_key.as_str(),
                "eval-specialist" => spec_eval.as_str(),
                "gguf-convert-specialist" | "quantize-specialist" => spec_convert.as_str(),
                _ => pipeline.as_str(),
            };
            write_manifest(&paths.manifest_path(step), key, None).unwrap();
        }
        let spec_sha = file_sha(&seated_s).unwrap();
        write_manifest(
            &paths.manifest_path("ollama-create-specialist"),
            &spec_ollama,
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
            fetch: fetch_inputs(DEFAULT_BASE, &resolved_base_dir(DEFAULT_BASE, &paths)),
            pipeline: Some(pipeline.clone()),
            repair: Some(repair),
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
            &base_model,
            &spec_model,
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
                &spec_model,
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
            &base_model,
            &spec_model,
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
            fetch: fetch_inputs(DEFAULT_BASE, &resolved_base_dir(DEFAULT_BASE, &paths)),
            pipeline: Some(changed.clone()),
            repair: None,
            recipe: Some(recipe_key),
            eval: Some(eval_inputs(
                &changed,
                DEFAULT_TAG,
                endpoint,
                None,
                DEFAULT_BUILT_BASE_TAG,
                &base_model,
                &spec_model,
            )),
            ollama_base: None,
            ollama_spec: Some(ollama_inputs(
                &changed,
                DEFAULT_TAG,
                DEFAULT_TAG,
                endpoint,
                None,
                DEFAULT_BUILT_BASE_TAG,
                &spec_model,
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
    fn changed_modelfile_invalidates_ollama_and_eval() {
        let dir = std::env::temp_dir().join(format!("journey-modelfile-fp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let paths = JourneyPaths::new(&dir);
        fs::write(&paths.base_report, "{\"complete\": true}\n").unwrap();
        fs::write(&paths.specialist_report, "{\"complete\": true}\n").unwrap();
        let seated_b = paths.seated_gguf("base", DEFAULT_QUANT);
        let seated_s = paths.seated_gguf("specialist", DEFAULT_QUANT);
        fs::write(&seated_b, "base-q").unwrap();
        fs::write(&seated_s, "spec-q").unwrap();
        let pipeline = "pipeline-key";
        let endpoint = "http://127.0.0.1:11434";
        let base_model = journey_modelfile(&seated_b, SeatChat::Qwen35);
        let spec_model = journey_modelfile(&seated_s, SeatChat::Qwen35);
        let eval_key = eval_inputs(
            pipeline,
            DEFAULT_TAG,
            endpoint,
            None,
            DEFAULT_BUILT_BASE_TAG,
            &base_model,
            &spec_model,
        );
        let ollama_base = ollama_inputs(
            pipeline,
            DEFAULT_BUILT_BASE_TAG,
            DEFAULT_TAG,
            endpoint,
            None,
            DEFAULT_BUILT_BASE_TAG,
            &base_model,
        );
        let ollama_spec = ollama_inputs(
            pipeline,
            DEFAULT_TAG,
            DEFAULT_TAG,
            endpoint,
            None,
            DEFAULT_BUILT_BASE_TAG,
            &spec_model,
        );
        write_manifest(&paths.manifest_path("eval-base"), &eval_key, None).unwrap();
        write_manifest(&paths.manifest_path("eval-specialist"), &eval_key, None).unwrap();
        write_manifest(
            &paths.manifest_path("ollama-create-base"),
            &ollama_base,
            Some(&file_sha(&seated_b).unwrap()),
        )
        .unwrap();
        write_manifest(
            &paths.manifest_path("ollama-create-specialist"),
            &ollama_spec,
            Some(&file_sha(&seated_s).unwrap()),
        )
        .unwrap();
        let inputs = Inputs {
            prepare: "prepare-key".into(),
            fetch: fetch_inputs(DEFAULT_BASE, &resolved_base_dir(DEFAULT_BASE, &paths)),
            pipeline: Some(pipeline.into()),
            repair: None,
            recipe: None,
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
        for step in plan_with(&ctx) {
            if matches!(
                step.name,
                "eval-base" | "eval-specialist" | "ollama-create-base" | "ollama-create-specialist"
            ) {
                assert_eq!(
                    step.action,
                    StepAction::Skip,
                    "{} {}",
                    step.name,
                    step.detail
                );
            }
        }

        fs::write(&paths.base_report, "{\"complete\": false}\n").unwrap();
        fs::write(&paths.specialist_report, "{\"complete\": false}\n").unwrap();
        for name in ["eval-base", "eval-specialist"] {
            let step = plan_with(&ctx)
                .into_iter()
                .find(|s| s.name == name)
                .unwrap();
            assert_eq!(step.action, StepAction::Run, "{step:?}");
            assert!(
                step.detail.contains("partial eval report"),
                "{name} {}",
                step.detail
            );
        }
        fs::write(&paths.base_report, "{\"complete\": true}\n").unwrap();
        fs::write(&paths.specialist_report, "{\"complete\": true}\n").unwrap();

        let changed_spec =
            spec_model.replace("PARAMETER num_predict 8", "PARAMETER num_predict 64");
        assert_ne!(changed_spec, spec_model);
        let changed_eval = eval_inputs(
            pipeline,
            DEFAULT_TAG,
            endpoint,
            None,
            DEFAULT_BUILT_BASE_TAG,
            &base_model,
            &changed_spec,
        );
        let changed_ollama = ollama_inputs(
            pipeline,
            DEFAULT_TAG,
            DEFAULT_TAG,
            endpoint,
            None,
            DEFAULT_BUILT_BASE_TAG,
            &changed_spec,
        );
        assert_ne!(changed_eval, eval_key);
        let changed_inputs = Inputs {
            eval: Some(changed_eval),
            ollama_spec: Some(changed_ollama),
            ..inputs.clone()
        };
        let changed_ctx = PlanCtx {
            inputs: &changed_inputs,
            ..ctx
        };
        let planned = plan_with(&changed_ctx);
        for name in ["eval-base", "eval-specialist", "ollama-create-specialist"] {
            let step = planned.iter().find(|s| s.name == name).unwrap();
            assert_eq!(step.action, StepAction::Run, "{step:?}");
            assert!(step.detail.contains("inputs changed"), "{step:?}");
        }
        let base_seat = planned
            .iter()
            .find(|s| s.name == "ollama-create-base")
            .unwrap();
        assert_eq!(base_seat.action, StepAction::Skip, "{base_seat:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn library_tag_skips_base_build_and_f16_skips_quantize() {
        let dir = std::env::temp_dir().join(format!("journey-lib-{}", std::process::id()));
        let paths = JourneyPaths::new(&dir);
        let inputs = Inputs {
            prepare: "p".into(),
            fetch: fetch_inputs(DEFAULT_BASE, &resolved_base_dir(DEFAULT_BASE, &paths)),
            pipeline: None,
            repair: None,
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
    fn deprecation_notice_is_the_stub_text() {
        assert!(hf_downloader_deprecated(
            "`huggingface-cli` is deprecated and no longer works. Use `hf` instead."
        ));
        assert!(hf_downloader_deprecated(
            "huggingface-cli is deprecated and no longer works"
        ));
        assert!(!hf_downloader_deprecated("connection reset by peer"));
        assert!(!hf_downloader_deprecated(
            "this endpoint is deprecated; retry later"
        ));
        assert!(!hf_downloader_deprecated("deprecated config key ignored"));
        assert!(!hf_downloader_deprecated(
            "deprecated; use huggingface_hub.login instead"
        ));
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
            correct: 2,
            invalid: 2,
            records: 8,
            p50: 10.0,
            p95: 40.0,
            thinking_leak: 3,
        };
        let specialist = SideScore {
            accuracy: 0.75,
            correct: 6,
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
        assert_eq!(threshold_met(&comparison, None, None, false), None);
        assert_eq!(
            threshold_met(&comparison, Some(0.4), Some(0.7), false),
            Some(true)
        );
        assert_eq!(
            threshold_met(&comparison, Some(0.6), None, false),
            Some(false)
        );
        assert_eq!(
            threshold_met(&comparison, None, Some(0.9), false),
            Some(false)
        );
        let partial = serde_json::json!({
            "complete": false,
            "records": 8,
            "accuracy": 0.5,
            "correct": 4
        });
        let err = side_score(&partial).unwrap_err().to_string();
        assert!(err.contains("partial"), "{err}");
        let missing = serde_json::json!({"records": 8, "accuracy": 0.5, "correct": 4});
        assert!(side_score(&missing).is_err());
    }

    fn score(accuracy: f64, correct: u64, records: u64) -> SideScore {
        SideScore {
            accuracy,
            correct,
            invalid: 0,
            records,
            p50: 1.0,
            p95: 2.0,
            thinking_leak: 0,
        }
    }

    #[test]
    fn significant_lift_gate_uses_newcombe_lower_bound() {
        let lifted = compare_sides(&score(0.5, 50, 100), &score(0.8, 80, 100));
        assert!(significant_lift(&lifted));
        assert_eq!(threshold_met(&lifted, None, None, true), Some(true));
        let gate = lift_gate_field(&lifted, true);
        assert_eq!(gate["verdict"], "pass");
        assert_eq!(gate["significant"], true);
        assert_eq!(gate["rule"], "newcombe_ci95_lower_gt_0");
        assert!(gate["low"].as_f64().unwrap() > 0.0);
        assert_eq!(gate["delta"].as_f64().unwrap(), lifted.delta);

        let crosses = compare_sides(&score(0.45, 45, 100), &score(0.55, 55, 100));
        assert!(crosses.delta > 0.0);
        assert!(!significant_lift(&crosses));
        assert_eq!(threshold_met(&crosses, None, None, true), Some(false));
        let missed = lift_gate_field(&crosses, true);
        assert_eq!(missed["verdict"], "fail");
        assert!(missed["low"].as_f64().unwrap() < 0.0);
        assert!(missed["high"].as_f64().unwrap() > 0.0);
        let refusal = significant_lift_refusal(&crosses, None, None);
        assert!(refusal.starts_with("refuse:classify-journey:"), "{refusal}");
        assert!(refusal.contains("base accuracy 0.4500"), "{refusal}");
        assert!(refusal.contains("specialist accuracy 0.5500"), "{refusal}");
        assert!(refusal.contains("delta 0.1000"), "{refusal}");
        assert!(refusal.contains("ci95 ["), "{refusal}");

        let worse = compare_sides(&score(0.8, 80, 100), &score(0.5, 50, 100));
        assert!(worse.delta < 0.0);
        assert!(!significant_lift(&worse));
        assert_eq!(threshold_met(&worse, None, None, true), Some(false));
        assert_eq!(lift_gate_field(&worse, true)["verdict"], "fail");
        let worse_msg = significant_lift_refusal(&worse, None, None);
        assert!(worse_msg.contains("delta -0.3000"), "{worse_msg}");

        assert_eq!(threshold_met(&worse, None, None, false), None);
        assert_eq!(lift_gate_field(&worse, false)["verdict"], "unset");
        assert_eq!(lift_gate_field(&worse, false)["required"], false);
        assert_eq!(threshold_met(&crosses, Some(0.05), None, false), Some(true));

        assert_eq!(
            threshold_met(&lifted, Some(0.2), Some(0.75), true),
            Some(true)
        );
        assert_eq!(threshold_met(&lifted, Some(0.4), None, true), Some(false));
        assert_eq!(
            threshold_met(&crosses, Some(0.05), Some(0.5), true),
            Some(false)
        );
        let both = significant_lift_refusal(&crosses, Some(0.2), Some(0.9));
        assert!(both.contains("min-delta 0.2000 missed"), "{both}");
        assert!(both.contains("min-accuracy 0.9000 missed"), "{both}");

        let empty = compare_sides(&score(0.0, 0, 0), &score(1.0, 0, 0));
        assert!(!significant_lift(&empty));
        assert_eq!(threshold_met(&empty, None, None, true), Some(false));
        let unavailable = significant_lift_refusal(&empty, None, None);
        assert!(unavailable.contains("ci95 unavailable"), "{unavailable}");
        assert!(lift_gate_field(&empty, true)["low"].is_null());
    }

    #[test]
    fn hub_id_fetches_a_local_dir_and_a_local_dir_is_passthrough() {
        let dir = std::env::temp_dir().join(format!("journey-fetch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let paths = JourneyPaths::new(&dir);
        let inputs = Inputs {
            prepare: "p".into(),
            fetch: fetch_inputs(DEFAULT_BASE, &resolved_base_dir(DEFAULT_BASE, &paths)),
            pipeline: None,
            repair: None,
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
        let shared = resolved_base_dir(DEFAULT_BASE, &paths);
        let expected = argv_line(&hf_download_argv("huggingface-cli", DEFAULT_BASE, &shared));
        assert_eq!(fetch.detail, expected);
        let convert = hub.iter().find(|s| s.name == "gguf-convert-base").unwrap();
        assert!(
            convert.detail.contains(&shared.display().to_string()),
            "{convert:?}"
        );
        assert!(
            !shared.starts_with(&dir),
            "hub snapshot is not under the journey out"
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
            fetch: fetch_inputs(&local_s, &resolved_base_dir(&local_s, &paths)),
            pipeline: None,
            repair: None,
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
        let partial = dir.join("partial-hub");
        fs::create_dir_all(&partial).unwrap();
        fs::write(partial.join("config.json"), "{}\n").unwrap();
        fs::write(partial.join("tokenizer.json"), "{}\n").unwrap();
        fs::write(partial.join("model.safetensors"), "w\n").unwrap();
        let mut partial_paths = JourneyPaths::new(&dir);
        partial_paths.base_cache = dir.clone();
        let partial_key = fetch_inputs(DEFAULT_BASE, &partial);
        let partial_decision = fetch_decision(DEFAULT_BASE, &partial, &partial_paths, &partial_key);
        assert_eq!(
            partial_decision.action,
            StepAction::Run,
            "config.json without .complete must not skip"
        );
        assert!(!hub_snapshot_ready(&partial));
        write_complete_marker(&partial).unwrap();
        assert!(hub_snapshot_ready(&partial));
        let ready = fetch_decision(DEFAULT_BASE, &partial, &partial_paths, &partial_key);
        assert_eq!(ready.action, StepAction::Skip);
        fs::write(partial.join("model.safetensors"), "x").unwrap();
        assert!(
            !hub_snapshot_ready(&partial),
            "a truncated weight must not stay ready"
        );
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
        let qwen = apply_preset(
            JourneyPreset::Qwen,
            DEFAULT_BASE,
            DEFAULT_TAG,
            TrainDriver::Together,
            None,
        )
        .unwrap();
        assert_eq!(qwen.base, DEFAULT_BASE);
        assert_eq!(qwen.tag, DEFAULT_TAG);
        assert_eq!(qwen.together_model, DEFAULT_TOGETHER_MODEL);
        assert_eq!(qwen.seat, SeatChat::Qwen35);
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
            model.contains("<｜Assistant｜><think>\n\n</think>\n\n"),
            "empty think prefill missing\n{model}"
        );
        assert!(
            model.contains("PARAMETER stop <｜end▁of▁sentence｜>"),
            "{model}"
        );
        assert!(!model.contains("<|im_end|>"), "{model}");
    }

    #[test]
    fn deepseek_system_user_prompt_matches_llamafactory() {
        let system = "Classify the row.";
        let user = "choose one letter";
        let rendered = render_ollama_template(DEEPSEEK_R1_TEMPLATE, system, &[("user", user)]);
        let expected = format!(
            "<｜begin▁of▁sentence｜>{system}<｜User｜>{user}<｜Assistant｜><think>\n\n</think>\n\n"
        );
        assert_eq!(rendered, expected);
        let history = render_ollama_template(DEEPSEEK_R1_TEMPLATE, "", &[("assistant", "A")]);
        assert!(
            history.contains("<｜Assistant｜>A<｜end▁of▁sentence｜>"),
            "{history}"
        );
        assert!(
            DEEPSEEK_R1_TEMPLATE.contains("<｜Assistant｜>{{ .Content }}<｜end▁of▁sentence｜>"),
            "format_assistant is content then eos with no newline\n{DEEPSEEK_R1_TEMPLATE}"
        );
        assert!(
            !DEEPSEEK_R1_TEMPLATE.contains("<｜User｜>\n"),
            "{DEEPSEEK_R1_TEMPLATE}"
        );
        assert!(
            !DEEPSEEK_R1_TEMPLATE.contains("<｜Assistant｜>\n"),
            "{DEEPSEEK_R1_TEMPLATE}"
        );
    }

    #[test]
    fn glm4_preset_fills_base_tag_and_template_and_refuses_default_together() {
        let applied = apply_preset(
            JourneyPreset::Glm4Chat,
            DEFAULT_BASE,
            DEFAULT_TAG,
            TrainDriver::Local,
            None,
        )
        .unwrap();
        assert_eq!(applied.base, GLM4_CHAT_BASE);
        assert_eq!(applied.tag, GLM4_CHAT_TAG);
        assert_eq!(applied.built_base_tag, GLM4_CHAT_BUILT_BASE_TAG);
        assert_eq!(applied.seat, SeatChat::Glm4);
        assert_eq!(train_template(&applied.base), "glm4");
        let kept = apply_preset(
            JourneyPreset::Glm4Chat,
            "lab/other",
            "custom-tag",
            TrainDriver::Local,
            None,
        )
        .unwrap();
        assert_eq!(kept.base, "lab/other");
        assert_eq!(kept.tag, "custom-tag");
        assert_eq!(kept.seat, SeatChat::Glm4);
        let qwen = apply_preset(
            JourneyPreset::Qwen,
            DEFAULT_BASE,
            DEFAULT_TAG,
            TrainDriver::Local,
            None,
        )
        .unwrap();
        assert_eq!(qwen.base, DEFAULT_BASE);
        assert_eq!(qwen.tag, DEFAULT_TAG);
        assert_eq!(qwen.seat, SeatChat::Qwen35);
        let err = apply_preset(
            JourneyPreset::Glm4Chat,
            DEFAULT_BASE,
            DEFAULT_TAG,
            TrainDriver::Together,
            None,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("GLM-4 Chat"), "{err}");
        assert!(err.contains("local llamafactory-cli"), "{err}");
        let hosted = apply_preset(
            JourneyPreset::Glm4Chat,
            DEFAULT_BASE,
            DEFAULT_TAG,
            TrainDriver::Together,
            Some("zai-org/glm-4-9b-chat"),
        )
        .unwrap();
        assert_eq!(hosted.together_model, "zai-org/glm-4-9b-chat");
        let dir = std::env::temp_dir().join(format!("journey-glm4-{}", std::process::id()));
        let paths = JourneyPaths::new(&dir);
        let recipe = lora_recipe_yaml(
            GLM4_CHAT_BASE,
            GLM4_CHAT_BASE,
            DEFAULT_DATASET,
            &dir,
            &paths.adapter_dir,
            None,
        );
        assert!(recipe.contains("template: glm4"), "{recipe}");
        assert!(recipe.contains("enable_thinking: false"), "{recipe}");
        let model = journey_modelfile(&paths.specialist_f16, SeatChat::Glm4);
        assert!(model.contains("[gMASK]<sop>"), "{model}");
        assert!(model.contains("<|system|>"), "{model}");
        assert!(model.contains("<|user|>"), "{model}");
        assert!(model.contains("<|assistant|>"), "{model}");
        assert!(model.contains("PARAMETER stop <|endoftext|>"), "{model}");
        assert!(model.contains("PARAMETER stop <|user|>"), "{model}");
        assert!(!model.contains("<|im_end|>"), "{model}");
        assert!(!model.contains("<think>"), "{model}");
    }

    #[test]
    fn glm4_system_user_prompt_matches_llamafactory() {
        let rendered = render_ollama_template(
            GLM4_TEMPLATE,
            "Classify the row.",
            &[("user", "choose one letter")],
        );
        // LLaMA-Factory glm4 and zai-org/glm-4-9b-chat: [gMASK]<sop>, then
        // <|system|>\n{{content}}, <|user|>\n{{content}}<|assistant|>, and
        // assistant \n{{content}}. efficient_eos keeps <|endoftext|> off the slot.
        assert_eq!(
            rendered,
            "[gMASK]<sop><|system|>\nClassify the row.<|user|>\nchoose one letter<|assistant|>"
        );
        let history = render_ollama_template(GLM4_TEMPLATE, "", &[("assistant", "A")]);
        assert_eq!(history, "[gMASK]<sop>\nA");
        assert!(!history.contains("<|endoftext|>"), "{history}");
    }

    /// Subset of Ollama's Go text/template used by the journey Modelfiles.
    fn render_ollama_template(template: &str, system: &str, messages: &[(&str, &str)]) -> String {
        let toks = tokenize_go_template(template);
        let mut i = 0;
        eval_go(&toks, &mut i, system, messages, None)
    }

    fn tokenize_go_template(template: &str) -> Vec<GoTok<'_>> {
        let mut toks = Vec::new();
        let mut rest = template;
        while let Some(start) = rest.find("{{") {
            if start > 0 {
                toks.push(GoTok::Text(&rest[..start]));
            }
            let after = &rest[start + 2..];
            let end = after.find("}}").expect("unclosed go template action");
            toks.push(GoTok::Action(after[..end].trim()));
            rest = &after[end + 2..];
        }
        if !rest.is_empty() {
            toks.push(GoTok::Text(rest));
        }
        toks
    }

    fn eval_go(
        toks: &[GoTok<'_>],
        i: &mut usize,
        system: &str,
        messages: &[(&str, &str)],
        message: Option<(&str, &str)>,
    ) -> String {
        let mut out = String::new();
        while *i < toks.len() {
            match toks[*i] {
                GoTok::Text(text) => {
                    out.push_str(text);
                    *i += 1;
                }
                GoTok::Action(action)
                    if action == "end" || action == "else" || action.starts_with("else if") =>
                {
                    break;
                }
                GoTok::Action(".System") => {
                    out.push_str(system);
                    *i += 1;
                }
                GoTok::Action(".Content") => {
                    out.push_str(message.expect("content outside a message").1);
                    *i += 1;
                }
                GoTok::Action("if .System") => {
                    *i += 1;
                    out.push_str(&take_if(
                        toks,
                        i,
                        system,
                        messages,
                        message,
                        !system.is_empty(),
                    ));
                }
                GoTok::Action("range .Messages") => {
                    *i += 1;
                    let body = *i;
                    if messages.is_empty() {
                        skip_go(toks, i);
                    } else {
                        for message in messages {
                            *i = body;
                            out.push_str(&eval_go(toks, i, system, messages, Some(*message)));
                        }
                    }
                    expect_end(toks, i);
                }
                GoTok::Action(action) if action.starts_with("if eq .Role ") => {
                    let want = action.trim_start_matches("if eq .Role ").trim_matches('"');
                    let role = message.map(|(role, _)| role).unwrap_or("");
                    *i += 1;
                    out.push_str(&take_if(toks, i, system, messages, message, role == want));
                }
                other => panic!("unexpected go action {other:?}"),
            }
        }
        out
    }

    fn take_if(
        toks: &[GoTok<'_>],
        i: &mut usize,
        system: &str,
        messages: &[(&str, &str)],
        message: Option<(&str, &str)>,
        cond: bool,
    ) -> String {
        if cond {
            let body = eval_go(toks, i, system, messages, message);
            if matches!(toks.get(*i), Some(GoTok::Action(a)) if a.starts_with("else")) {
                *i += 1;
                skip_go(toks, i);
            }
            expect_end(toks, i);
            body
        } else {
            skip_go(toks, i);
            match toks.get(*i) {
                Some(GoTok::Action("else")) => {
                    *i += 1;
                    let body = eval_go(toks, i, system, messages, message);
                    expect_end(toks, i);
                    body
                }
                Some(GoTok::Action(action)) if action.starts_with("else if eq .Role ") => {
                    let want = action
                        .trim_start_matches("else if eq .Role ")
                        .trim_matches('"');
                    let role = message.map(|(role, _)| role).unwrap_or("");
                    *i += 1;
                    take_if(toks, i, system, messages, message, role == want)
                }
                _ => {
                    expect_end(toks, i);
                    String::new()
                }
            }
        }
    }

    fn expect_end(toks: &[GoTok<'_>], i: &mut usize) {
        match toks.get(*i) {
            Some(GoTok::Action("end")) => *i += 1,
            other => panic!("expected end, got {other:?} at {i}"),
        }
    }

    fn skip_go(toks: &[GoTok<'_>], i: &mut usize) {
        let mut depth = 0;
        while *i < toks.len() {
            match toks[*i] {
                GoTok::Action(action)
                    if depth == 0
                        && (action == "end"
                            || action == "else"
                            || action.starts_with("else if")) =>
                {
                    break;
                }
                GoTok::Action(action)
                    if action.starts_with("if ") || action == "range .Messages" =>
                {
                    depth += 1;
                    *i += 1;
                }
                GoTok::Action("end") => {
                    depth -= 1;
                    *i += 1;
                }
                _ => *i += 1,
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum GoTok<'a> {
        Text(&'a str),
        Action(&'a str),
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

    #[test]
    fn manifests_from_before_export_repair_redo_specialist_steps() {
        let dir =
            std::env::temp_dir().join(format!("journey-repair-invalidate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let paths = JourneyPaths::new(&dir);
        fs::create_dir_all(&paths.export_dir).unwrap();
        fs::write(paths.export_dir.join("config.json"), "{}\n").unwrap();
        fs::write(&paths.dataset_jsonl, "row\n").unwrap();
        fs::write(&paths.recipe, "dataset: classify_decisions\n").unwrap();
        fs::write(&paths.specialist_f16, "spec-f16").unwrap();
        let seated = paths.seated_gguf("specialist", DEFAULT_QUANT);
        fs::write(&seated, "spec-q").unwrap();
        fs::write(&paths.specialist_report, "{\"complete\": true}\n").unwrap();
        fs::write(&paths.base_f16, "base-f16").unwrap();
        fs::create_dir_all(&paths.adapter_dir).unwrap();
        fs::write(paths.adapter_dir.join("adapter_config.json"), "{}\n").unwrap();
        let pipeline = "old-pipeline".to_string();
        let repair = "repair-fp".to_string();
        let eval_key = "old-eval".to_string();
        let ollama_spec = "old-ollama".to_string();
        write_manifest(&paths.manifest_path("merge-export"), &pipeline, None).unwrap();
        write_manifest(&paths.manifest_path("gguf-convert-base"), &pipeline, None).unwrap();
        write_manifest(
            &paths.manifest_path("gguf-convert-specialist"),
            &pipeline,
            None,
        )
        .unwrap();
        write_manifest(&paths.manifest_path("quantize-specialist"), &pipeline, None).unwrap();
        write_manifest(&paths.manifest_path("eval-specialist"), &eval_key, None).unwrap();
        write_manifest(
            &paths.manifest_path("ollama-create-specialist"),
            &ollama_spec,
            Some(&file_sha(&seated).unwrap()),
        )
        .unwrap();
        let inputs = Inputs {
            prepare: "prep".into(),
            fetch: "fetch".into(),
            pipeline: Some(pipeline),
            repair: Some(repair),
            recipe: None,
            eval: Some(eval_key),
            ollama_base: None,
            ollama_spec: Some(ollama_spec),
        };
        let ctx = PlanCtx {
            paths: &paths,
            base: DEFAULT_BASE,
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
        let steps = plan_with(&ctx);
        for name in [
            "gguf-convert-specialist",
            "quantize-specialist",
            "ollama-create-specialist",
            "eval-specialist",
        ] {
            let step = steps.iter().find(|s| s.name == name).unwrap();
            assert_eq!(step.action, StepAction::Run, "{step:?}");
            assert!(
                step.detail.contains("inputs changed"),
                "{name} {}",
                step.detail
            );
        }
        for name in ["merge-export", "gguf-convert-base"] {
            let step = steps.iter().find(|s| s.name == name).unwrap();
            assert_eq!(step.action, StepAction::Skip, "{step:?}");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_probe_refuses_with_the_ollama_error_and_gguf_name() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let port = match server.server_addr() {
            tiny_http::ListenAddr::IP(addr) => addr.port(),
            other => panic!("expected ip listen addr, got {other:?}"),
        };
        std::thread::spawn(move || {
            for mut req in server.incoming_requests() {
                let mut body = String::new();
                let _ = std::io::Read::read_to_string(req.as_reader(), &mut body);
                assert!(
                    body.contains("\"think\":false") || body.contains("\"think\": false"),
                    "{body}"
                );
                assert!(
                    body.contains("\"num_predict\":1") || body.contains("\"num_predict\": 1"),
                    "{body}"
                );
                let payload = "{\"error\":\"error loading model: check_tensor_dims: tensor 'blk.32.attn_norm.weight' not found\"}";
                let resp = tiny_http::Response::from_string(payload).with_status_code(500);
                let _ = req.respond(resp);
            }
        });
        let gguf = std::env::temp_dir().join(format!("probe-{}.gguf", std::process::id()));
        let err = probe_ollama_load(
            &format!("http://127.0.0.1:{port}"),
            "classify-specialist",
            &gguf,
            5,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("refuse:classify-journey"), "{err}");
        assert!(err.contains("classify-specialist"), "{err}");
        assert!(err.contains("blk.32.attn_norm.weight"), "{err}");
        assert!(err.contains(&gguf.display().to_string()), "{err}");
    }

    #[test]
    fn expand_tag_on_a_foreign_dataset_is_refused_inside_the_journey() {
        let err = journey_import_dir("ag_news", "all", 7, Some("rev1"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("rust_idiom"), "{err}");
        let rust = journey_import_dir("rust_idiom", "all", 7, Some("rev1")).unwrap();
        assert!(
            rust.ends_with("rust_idiom-all-s7-rev1"),
            "{}",
            rust.display()
        );

        let input = std::env::temp_dir().join("classify-decisions.jsonl");
        let out =
            std::env::temp_dir().join(format!("journey-expand-foreign-{}", std::process::id()));
        let base_cache = std::env::temp_dir().join("classify-base-cache");
        let err = cmd_classify_journey(&JourneyRequest {
            input: &input,
            out: &out,
            base: DEFAULT_BASE,
            base_tag: None,
            tag: DEFAULT_TAG,
            endpoint: "http://127.0.0.1:11434",
            dataset_name: DEFAULT_DATASET,
            seed: 7,
            held_out_ratio: 0.2,
            max_steps: None,
            quant: DEFAULT_QUANT,
            llama_cpp_dir: None,
            force: false,
            print: true,
            run: false,
            min_delta: None,
            min_accuracy: None,
            require_significant_lift: false,
            timeout_secs: 5,
            together_poll_secs: DEFAULT_TOGETHER_POLL_SECS,
            train_driver: TrainDriver::Local,
            together_model: DEFAULT_TOGETHER_MODEL,
            together_base_url: DEFAULT_TOGETHER_API,
            api_key_env: None,
            built_base_tag: DEFAULT_BUILT_BASE_TAG,
            seat: SeatChat::Qwen35,
            llama_note: "note",
            preset: JourneyPreset::Qwen,
            import_dataset: Some("devign"),
            train_size: "all",
            heldout_size: "all",
            from_local: None,
            import_fetch: crate::classify_import::ImportFetch::Bulk,
            python: None,
            base_cache: &base_cache,
            few_shot: 0,
            expand_tag: Some("rev1"),
            estate: None,
            prepared: None,
            enrich_tag: None,
            import_trained: false,
            binding_id: None,
        })
        .unwrap_err()
        .to_string();
        assert!(err.contains("rust_idiom"), "{err}");
        assert!(!err.contains("seed"), "{err}");
    }

    #[test]
    fn missing_specialist_gguf_does_not_invent_a_proposal() {
        let root = std::env::temp_dir().join(format!(
            "cell-one-classify-missing-gguf-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let out = root.join("out");
        let prepared = root.join("prepared");
        fs::create_dir_all(&out).unwrap();
        fs::create_dir_all(&prepared).unwrap();
        fs::write(prepared.join("prepare.json"), "{}\n").unwrap();
        let estate = root.join("estate.yaml");
        fs::write(&estate, "name: lab\n").unwrap();
        let input = root.join("in.jsonl");
        let base_cache = root.join("cache");
        let req = JourneyRequest {
            input: &input,
            out: &out,
            base: DEFAULT_BASE,
            base_tag: None,
            tag: DEFAULT_TAG,
            endpoint: "http://127.0.0.1:11434",
            dataset_name: DEFAULT_DATASET,
            seed: 1,
            held_out_ratio: 0.2,
            max_steps: None,
            quant: DEFAULT_QUANT,
            llama_cpp_dir: None,
            force: false,
            print: false,
            run: true,
            min_delta: None,
            min_accuracy: None,
            require_significant_lift: false,
            timeout_secs: 5,
            together_poll_secs: DEFAULT_TOGETHER_POLL_SECS,
            train_driver: TrainDriver::Local,
            together_model: DEFAULT_TOGETHER_MODEL,
            together_base_url: DEFAULT_TOGETHER_API,
            api_key_env: None,
            built_base_tag: DEFAULT_BUILT_BASE_TAG,
            seat: SeatChat::Qwen35,
            llama_note: "note",
            preset: JourneyPreset::Qwen,
            import_dataset: None,
            train_size: "all",
            heldout_size: "all",
            from_local: None,
            import_fetch: crate::classify_import::ImportFetch::Bulk,
            python: None,
            base_cache: &base_cache,
            few_shot: 0,
            expand_tag: None,
            estate: Some(&estate),
            prepared: Some(&prepared),
            enrich_tag: Some("cell-enrich-overnight-traces"),
            import_trained: true,
            binding_id: None,
        };
        let paths = JourneyPaths::new(&out);
        let err = seat_handoff(&req, &paths, HandoffMode::AfterRun)
            .unwrap_err()
            .to_string();
        assert!(err.contains("refuse:classify-journey"), "{err}");
        assert!(err.contains("is missing"), "{err}");
        assert!(err.contains("no proposal written"), "{err}");
        assert!(!prepared.join("binding-proposal.json").is_file());
        assert!(!out.join("specialist.Q4_K_M.gguf").is_file());
        assert_eq!(
            fs::read_to_string(prepared.join("prepare.json")).unwrap(),
            "{}\n"
        );

        let mut skip = req;
        skip.import_trained = false;
        seat_handoff(&skip, &paths, HandoffMode::AfterRun).unwrap();
        assert!(!prepared.join("binding-proposal.json").is_file());
        assert!(!out.join("specialist.Q4_K_M.gguf").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn specialty_binding_id_is_printed_and_recorded() {
        let kept = specialty_seat_lines("local_slm", "ag_news");
        assert_eq!(
            kept[0],
            "Specialty seat: local_slm, class local, function ag_news."
        );
        assert_eq!(
            kept[1],
            "binding_id stays local_slm. trained_shape gguf. auto_apply=false."
        );
        let specialty = specialty_seat_lines("ag_news", "ag_news");
        assert_eq!(
            specialty[0],
            "Specialty seat: ag_news, class local, function ag_news."
        );
        assert!(specialty[1].contains("added beside local_slm"), "{}", specialty[1]);
        assert!(specialty[1].contains("replaced in place"), "{}", specialty[1]);

        let token = std::process::id()
            .to_string()
            .replace("5090", "0000")
            .replace("4090", "0000")
            .replace("4080", "0000")
            .replace("3090", "0000");
        let root = std::env::temp_dir().join(format!("cell-one-specialty-handoff-{token}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let out = root.join("out");
        let prepared = root.join("prepared");
        let state = root.join("state");
        fs::create_dir_all(&out).unwrap();
        fs::create_dir_all(&state).unwrap();
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut estate = estate_schema::load_estate(&repo.join("examples/estate.yaml")).unwrap();
        {
            let seat = estate
                .model_bindings
                .iter_mut()
                .find(|binding| binding.id == "local_slm")
                .unwrap();
            let params = seat.params.as_object_mut().unwrap();
            params.insert("model".into(), serde_json::json!("llama3"));
            params.insert(
                "train_base_model".into(),
                serde_json::json!("Qwen/Qwen2.5-0.5B-Instruct"),
            );
        }
        let estate_path = root.join("estate.yaml");
        let rendered = estate_schema::render_estate_yaml(&estate).unwrap();
        fs::write(&estate_path, &rendered).unwrap();
        let before = fs::read(&estate_path).unwrap();
        let pack = model_estate::load_enrich_pack(
            &repo.join("examples/fixtures/specialist-overnight.pack.json"),
            &repo.join("packs"),
        )
        .unwrap();
        model_estate::prepare_enrich(&model_estate::PrepareEnrichRequest {
            estate: &estate,
            pack: &pack,
            curator: "jason",
            driver_id: model_estate::LLAMAFACTORY_QLORA_ID,
            job: "train",
            out_dir: &prepared,
            max_steps: None,
            official_scale: false,
            from_feed: false,
            state_dir: &state,
        })
        .unwrap();
        let input = root.join("in.jsonl");
        let base_cache = root.join("cache");
        let req = JourneyRequest {
            input: &input,
            out: &out,
            base: DEFAULT_BASE,
            base_tag: None,
            tag: DEFAULT_TAG,
            endpoint: "http://127.0.0.1:11434",
            dataset_name: DEFAULT_DATASET,
            seed: 1,
            held_out_ratio: 0.2,
            max_steps: None,
            quant: DEFAULT_QUANT,
            llama_cpp_dir: None,
            force: false,
            print: false,
            run: true,
            min_delta: None,
            min_accuracy: None,
            require_significant_lift: false,
            timeout_secs: 5,
            together_poll_secs: DEFAULT_TOGETHER_POLL_SECS,
            train_driver: TrainDriver::Local,
            together_model: DEFAULT_TOGETHER_MODEL,
            together_base_url: DEFAULT_TOGETHER_API,
            api_key_env: None,
            built_base_tag: DEFAULT_BUILT_BASE_TAG,
            seat: SeatChat::Qwen35,
            llama_note: "note",
            preset: JourneyPreset::Qwen,
            import_dataset: Some("ag_news"),
            train_size: "all",
            heldout_size: "all",
            from_local: None,
            import_fetch: crate::classify_import::ImportFetch::Bulk,
            python: None,
            base_cache: &base_cache,
            few_shot: 0,
            expand_tag: None,
            estate: Some(&estate_path),
            prepared: Some(&prepared),
            enrich_tag: Some("cell-enrich-overnight-traces"),
            import_trained: true,
            binding_id: Some("rtx-5090"),
        };
        let paths = JourneyPaths::new(&out);
        let err = seat_handoff(&req, &paths, HandoffMode::Plan)
            .unwrap_err()
            .to_string();
        assert!(err.contains("refuse:sku-banned"), "{err}");
        assert!(!prepared.join("binding-proposal.json").is_file());
        let mut req = req;
        req.binding_id = Some("ag_news");
        let display = handoff_display(&req, &paths, "ag_news");
        assert!(
            display.line.contains("--binding-id ag_news"),
            "{}",
            display.line
        );
        assert!(
            !handoff_display(&req, &paths, "local_slm")
                .line
                .contains("--binding-id"),
            "default handoff must keep today's command line"
        );
        fs::write(paths.seated_gguf("specialist", DEFAULT_QUANT), "gguf-fixture").unwrap();
        seat_handoff(&req, &paths, HandoffMode::AfterRun).unwrap();
        let proposal: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(prepared.join("binding-proposal.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(proposal["binding_id"], "ag_news");
        assert_eq!(proposal["auto_apply"], false);
        assert_eq!(proposal["promoted"], false);
        assert_eq!(proposal["estate_rewritten"], false);
        assert_eq!(proposal["trained_shape"], "gguf");
        assert_eq!(proposal["proposed_binding"]["id"], "ag_news");
        assert_eq!(proposal["proposed_binding"]["class"], "local");
        let paste = proposal["paste_yaml"].as_str().unwrap();
        assert!(paste.contains("Add this class:local"), "{paste}");
        assert!(paste.contains("local_slm stays"), "{paste}");
        assert_eq!(fs::read(&estate_path).unwrap(), before);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn dual_compare_report_records_the_specialist_delta_without_a_live_pass() {
        let parent = std::env::temp_dir().join(format!("dual-compare-unit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&parent);
        let qwen = DualSide {
            preset: JourneyPreset::Qwen,
            dir_name: "qwen",
            base: DEFAULT_BASE.into(),
            tag: "specialist-rustidiom-all-rev1".into(),
            built_base_tag: DEFAULT_BUILT_BASE_TAG,
            seat: SeatChat::Qwen35,
            llama_note: QWEN35_RECENT,
            together_model: DEFAULT_TOGETHER_MODEL.into(),
            out: parent.join("qwen"),
            dataset_name: "rust_idiom".into(),
        };
        let glm = DualSide {
            preset: JourneyPreset::Glm4Chat,
            dir_name: "glm4-chat",
            base: GLM4_CHAT_BASE.into(),
            tag: "glm4-chat-specialist-rustidiom-all-rev1".into(),
            built_base_tag: GLM4_CHAT_BUILT_BASE_TAG,
            seat: SeatChat::Glm4,
            llama_note: GLM4_LLAMA_NOTE,
            together_model: DEFAULT_TOGETHER_MODEL.into(),
            out: parent.join("glm4-chat"),
            dataset_name: "rust_idiom".into(),
        };
        let qwen = DualPresetScore {
            base_accuracy: 0.5,
            specialist_accuracy: 0.75,
            delta: 0.25,
            base_correct: 2,
            base_records: 4,
            specialist_correct: 3,
            specialist_records: 4,
        };
        let glm_score = DualPresetScore {
            base_accuracy: 0.25,
            specialist_accuracy: 0.25,
            delta: 0.0,
            base_correct: 1,
            base_records: 4,
            specialist_correct: 1,
            specialist_records: 4,
        };
        let cache = PathBuf::from(".cell/classify-import/rust_idiom-all-s42-rev1");
        let report = dual_report_value(
            "run",
            &cache,
            "abc",
            "rev1",
            "all",
            42,
            &qwen,
            &glm,
            Some(&qwen),
            Some(&glm_score),
            Some("def"),
            false,
            None,
            "",
        );
        assert_eq!(report["qwen_glm_specialist_delta"], 0.5);
        assert_eq!(report["presets"]["qwen"]["specialist_accuracy"], 0.75);
        assert_eq!(report["presets"]["qwen"]["base_accuracy"], 0.5);
        assert_eq!(report["presets"]["qwen"]["delta"], 0.25);
        assert_eq!(report["presets"]["glm4-chat"]["specialist_accuracy"], 0.25);
        assert_eq!(report["presets"]["glm4-chat"]["base_accuracy"], 0.25);
        assert_eq!(report["factory_live_pass"], false);
        assert_eq!(report["live_pass_recorded"], false);
        assert_eq!(report["ready_for_live_test"], "no");
        assert_eq!(report["holdout_shared"], true);
        assert_eq!(report["prepared_heldout_sha256"], "def");
        let note = report["note"].as_str().unwrap();
        assert!(note.contains("not a factory live PASS"), "{note}");
        assert!(note.contains("READY_FOR_LIVE_TEST stays no"), "{note}");
        assert!(report["qwen_minus_glm"]["ci95"]["method"]
            .as_str()
            .unwrap()
            .contains("newcombe"));
        let started = dual_report_value(
            "in-progress",
            &cache,
            "abc",
            "rev1",
            "all",
            42,
            &qwen,
            &glm,
            None,
            None,
            None,
            false,
            None,
            "",
        );
        assert_eq!(started["mode"], "in-progress");
        assert_eq!(started["modest"], false);
        assert!(started["max_steps"].is_null());
        assert!(started["modest_note"].is_null());
        assert!(started["qwen_glm_specialist_delta"].is_null());
        assert!(started["presets"]["qwen"]["specialist_accuracy"].is_null());
        assert!(started["presets"]["glm4-chat"]["base_accuracy"].is_null());
        assert_eq!(started["factory_live_pass"], false);
        assert_eq!(started["live_pass_recorded"], false);
        assert_eq!(started["ready_for_live_test"], "no");
        let started_note = started["note"].as_str().unwrap();
        assert!(
            started_note.contains("Scores stay absent"),
            "{started_note}"
        );
        assert!(
            started_note.contains("not a factory live PASS"),
            "{started_note}"
        );
        let _ = fs::remove_dir_all(&parent);
    }
}
