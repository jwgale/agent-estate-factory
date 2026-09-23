//! Print-only adapter → merged weights handoff.
//!
//! Axolotl already merges a LoRA adapter with `axolotl merge-lora`
//! ([getting started](https://docs.axolotl.ai/docs/getting-started.html)
//! section 4.4, and the [CLI page](https://docs.axolotl.ai/docs/cli.html)).
//! That command writes `{output_dir}/merged`. It does not take `--out`.
//! LLaMA-Factory's merge is `llamafactory-cli export` on the prepare's
//! `export.yaml`. The documented shape is
//! `examples/merge_lora/qwen3_lora_sft.yaml`
//! ([merge page](https://llamafactory.readthedocs.io/en/latest/getting_started/merge_lora.html),
//! [examples README](https://github.com/hiyouga/LlamaFactory/blob/main/examples/README.md)).
//! That command writes `export_dir`. This card then prints `gguf-convert`
//! and `local-seat` for that Hugging Face directory.
//!
//! `mlx-lm-lora` prints mlx-lm's documented fuse. `mlx_lm.lora` writes
//! `adapter_config.json` and `adapters.safetensors`. `mlx_lm.fuse` writes
//! `fused_model/` (`--save-path` default). `--export-gguf` writes
//! `ggml-model-f16.gguf` inside that directory
//! (https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md).
//! The fused directory is MLX weights. This card does not print
//! `convert_hf_to_gguf.py` for it. `local-seat` is named for that GGUF file.
//!
//! `unsloth-qlora` prints Unsloth's documented save lines. The vLLM guide
//! saves a LoRA with `model.save_pretrained` or `save_pretrained_merged`
//! (`save_method` `lora`), which writes `adapter_config.json` and
//! `adapter_model.safetensors`. The same guide and the GGUF page save a
//! merged 16-bit directory with `save_method` `merged_16bit`. The GGUF page
//! then names `python llama.cpp/convert_hf_to_gguf.py`. This card points
//! `gguf-convert` and `local-seat` at that Hugging Face directory. It does
//! not print an Ollama `ADAPTER` line for the PEFT directory.
//!
//! This module prints those lines. It does not merge, fuse, spawn, download,
//! or write a directory.

use crate::error::ModelError;
use crate::gguf_convert::{
    export_tokenizer_guidance, gguf_convert_cli, local_seat_cli, printed_convert_line,
};
use crate::local_seat::{classify_adapter, shell_quote};
use crate::train_enrich::{
    is_axolotl_driver, is_llamafactory_driver, is_post_merge_print_driver, load_prepare_doc,
    local_enrich_tag, refuse_post_merge_driver, refuse_recipe_train_record, refuse_sacred_and_sku,
    EnrichJobKind, EnrichPrepareDoc, AXOLOTL_LORA_ID, AXOLOTL_QLORA_ID, LLAMAFACTORY_QLORA_ID,
    MLX_LM_LORA_ID, MLX_LORA_DOC, UNSLOTH_GGUF_DOC, UNSLOTH_INFERENCE_DOC,
    UNSLOTH_OLLAMA_DOC, UNSLOTH_QLORA_ID, UNSLOTH_VLLM_DOC,
};
use feed_collector::{refuse_raw_secrets, FeedError};
use std::io::Read;
use std::path::{Path, PathBuf};

const CARD_MAX_BYTES: u64 = 1024 * 1024;

/// `--save-path` default in `mlx_lm.fuse`. LORA.md writes the fused model here.
pub(crate) const MLX_FUSED_DIR_NAME: &str = "fused_model";
/// `--gguf-path` default. `fuse.py` joins it with the save path.
pub(crate) const MLX_GGUF_FILE_NAME: &str = "ggml-model-f16.gguf";
/// Final adapter weights `mlx_lm.lora` writes and `load_adapters` reads.
pub(crate) const MLX_ADAPTER_WEIGHTS: &str = "adapters.safetensors";

/// Directory name beside the prepare. Unsloth's pages pass a directory string
/// (`merged_model` on the GGUF page, `finetuned_model` on the vLLM guide).
/// This print uses one path so `gguf-convert` names the same directory.
pub(crate) const UNSLOTH_MERGED_DIR_NAME: &str = "merged";
/// Default safetensors name in Unsloth's `save_method` `lora` docstring and the PEFT save.
pub(crate) const UNSLOTH_ADAPTER_WEIGHTS: &str = "adapter_model.safetensors";
/// Pickle name Unsloth writes when `safe_serialization` is False.
pub(crate) const UNSLOTH_ADAPTER_BIN: &str = "adapter_model.bin";

/// Documented LoRA merge example. The README runs `llamafactory-cli export` on this file.
pub(crate) const LLAMAFACTORY_MERGE_EXAMPLE: &str = "examples/merge_lora/qwen3_lora_sft.yaml";
/// Merge page that shows that example and the unquantized-merge note.
pub(crate) const LLAMAFACTORY_MERGE_DOC: &str =
    "https://llamafactory.readthedocs.io/en/latest/getting_started/merge_lora.html";
/// Examples index. The merge section runs the same export command.
pub(crate) const LLAMAFACTORY_MERGE_README: &str =
    "https://github.com/hiyouga/LlamaFactory/blob/main/examples/README.md";

/// Printed plan. `merged_dir` is the directory the external merge or fuse
/// writes. For LLaMA-Factory and Axolotl that is a Hugging Face directory.
/// For `mlx-lm-lora` it is the `mlx_lm.fuse` `--save-path` (`fused_model`).
/// This command does not create it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeAdaptPlan {
    pub seat_tag: String,
    pub local_tag: String,
    pub pack_id: String,
    pub driver: String,
    /// External merge lines. Axolotl: `axolotl merge-lora`, plus `--dequant`
    /// on the qlora card. LLaMA-Factory: `llamafactory-cli export`.
    pub merge_commands: Vec<String>,
    pub merged_dir: PathBuf,
    /// Documented convert line. LLaMA-Factory and Axolotl print
    /// `python3 convert_hf_to_gguf.py`. `mlx-lm-lora` prints
    /// `mlx_lm.fuse --export-gguf`.
    pub convert_command: String,
    /// `estate enrich gguf-convert` for a merged Hugging Face directory.
    /// Empty on `mlx-lm-lora`: that command does not convert MLX weights.
    /// Set on `unsloth-qlora`: the merged directory is a Hugging Face directory.
    pub gguf_convert_command: String,
    pub local_seat_command: String,
    pub report: String,
}

/// `estate enrich merge-adapt` line for a prepared tree and an adapter directory.
pub(crate) fn printed_merge_adapt_cli(prepared: &Path, adapter: &Path) -> String {
    format!(
        "estate enrich merge-adapt --prepared {} --adapter {}",
        shell_quote(&prepared.display().to_string()),
        shell_quote(&adapter.display().to_string())
    )
}

/// Current Axolotl CLI. Dash flags match docs.axolotl.ai. `--dequant` is the
/// CLI flag that writes a bf16 checkpoint for a quantized base.
pub(crate) fn printed_axolotl_merge_line(config: &Path, adapter: &Path, dequant: bool) -> String {
    let mut line = format!(
        "axolotl merge-lora {} --lora-model-dir={}",
        shell_quote(&config.display().to_string()),
        shell_quote(&adapter.display().to_string())
    );
    if dequant {
        line.push_str(" --dequant");
    }
    line
}

/// Read `prepare.json`, require an adapter directory, and build the merge report.
/// Does not write and does not spawn a process.
pub fn plan_merge_adapt(prepared_dir: &Path, adapter: &Path) -> Result<MergeAdaptPlan, ModelError> {
    refuse_sacred_and_sku("prepared", &prepared_dir.display().to_string())?;
    refuse_sacred_and_sku("adapter", &adapter.display().to_string())?;
    let doc = load_prepare_doc(&prepared_dir.join("prepare.json"))?;
    let mlx = doc.driver == MLX_LM_LORA_ID;
    let unsloth = doc.driver == UNSLOTH_QLORA_ID;
    if !mlx && !is_post_merge_print_driver(&doc.driver) {
        return Err(refuse_post_merge_driver("merge-adapt", &doc.driver));
    }
    if doc.job != EnrichJobKind::Train.as_str() {
        return Err(ModelError::Other(format!(
            "refuse:job: merge-adapt expects job train, found '{}'",
            doc.job
        )));
    }
    let seat_tag = doc
        .seat_tag
        .clone()
        .ok_or_else(|| ModelError::Other("refuse:seat: prepare.json has no seat_tag".into()))?;
    single_line("refuse:seat", "seat_tag", &seat_tag)?;
    if let Some(train) = doc.train_base_model.as_deref() {
        single_line("refuse:train-base", "train_base_model", train)?;
    }
    let local_tag = local_enrich_tag(&doc.pack_id);
    let plan = if mlx {
        plan_mlx(prepared_dir, &doc, &seat_tag, &local_tag, adapter)?
    } else if unsloth {
        plan_unsloth(prepared_dir, &doc, &seat_tag, &local_tag, adapter)?
    } else {
        let shape = classify_adapter(adapter)?;
        if is_axolotl_driver(&doc.driver) {
            plan_axolotl(prepared_dir, &doc, &seat_tag, &local_tag, &shape.dir)?
        } else if is_llamafactory_driver(&doc.driver) {
            plan_llamafactory(prepared_dir, &doc, &seat_tag, &local_tag, &shape.dir)?
        } else {
            return Err(refuse_post_merge_driver("merge-adapt", &doc.driver));
        }
    };
    refuse_sacred_and_sku("merge-adapt report", &plan.report)?;
    refuse_raw_secrets(&plan.report).map_err(map_feed)?;
    Ok(plan)
}

fn plan_axolotl(
    prepared_dir: &Path,
    doc: &EnrichPrepareDoc,
    seat_tag: &str,
    local_tag: &str,
    adapter_dir: &Path,
) -> Result<MergeAdaptPlan, ModelError> {
    let config = prepared_dir.join("axolotl.yml");
    let text = read_axolotl_yaml(&config)?;
    refuse_sacred_and_sku("axolotl.yml", &text)?;
    refuse_raw_secrets(&text).map_err(map_feed)?;
    let output_raw = yaml_top_scalar(&text, "output_dir")?.ok_or_else(|| {
        ModelError::Other(format!(
            "refuse:merge: {} has no output_dir. Axolotl writes the merged model to output_dir/merged (docs.axolotl.ai getting-started). This factory does not invent that directory.",
            config.display()
        ))
    })?;
    let output_dir = resolve_beside_prepare(prepared_dir, &output_raw, "output_dir")?;
    if let (Some(train), Some(yaml_base)) = (
        doc.train_base_model.as_deref(),
        yaml_top_scalar(&text, "base_model")?,
    ) {
        if yaml_base != train {
            return Err(ModelError::Other(format!(
                "refuse:train-base: axolotl.yml base_model is '{yaml_base}'. prepare.json train_base_model is '{train}'. The merge reads the yaml. These must name the same train base."
            )));
        }
    }
    refuse_card_mismatch(&doc.driver, &text)?;
    let merged = output_dir.join("merged");
    refuse_sacred_and_sku("merged", &merged.display().to_string())?;
    single_line(
        "refuse:merge",
        "merged directory",
        &merged.display().to_string(),
    )?;
    let basic = printed_axolotl_merge_line(&config, adapter_dir, false);
    let quant = doc.driver == AXOLOTL_QLORA_ID;
    let mut merge_commands = vec![basic.clone()];
    let dequant_note = if quant {
        let line = printed_axolotl_merge_line(&config, adapter_dir, true);
        merge_commands.push(line.clone());
        format!(
            "This card is axolotl-qlora. The Axolotl CLI documents `--dequant` as the flag that writes a bf16 checkpoint for a quantized base. The line without `--dequant` is the format-preserving default. Both write {merged}. `convert_hf_to_gguf.py --outtype auto` reads a 16-bit float checkpoint, so the bf16 line is the one to run before gguf-convert. Run one of these lines.\n\
             \n\
             {line}\n\
             \n",
            merged = merged.display(),
        )
    } else {
        "This card is bf16 LoRA (`axolotl-lora`). The printed line is the getting-started merge.\n\n"
            .to_string()
    };
    let convert = printed_convert_line(&merged);
    let gguf = gguf_convert_cli(prepared_dir, &merged);
    let seat = local_seat_cli(prepared_dir, &merged);
    let report = format!(
        "{header}\
         \n\
         Axolotl documents this merge on https://docs.axolotl.ai/docs/getting-started.html (section 4.4) and https://docs.axolotl.ai/docs/cli.html. The current command is `axolotl merge-lora`. `--lora-model-dir` is the adapter directory. The legacy module is `python -m axolotl.cli.merge_lora` with `--lora_model_dir`. This print uses the current CLI.\n\
         \n\
         {basic}\n\
         \n\
         {dequant_note}\
         Axolotl writes the merged Hugging Face directory to `{{output_dir}}/merged`. axolotl.yml sets output_dir to {output_dir}. The directory is {merged}. `axolotl merge-lora` does not take `--out`. This factory does not add a flag. A relative output_dir is resolved from this prepared directory. The merged directory follows output_dir, including when `--lora-model-dir` is a checkpoint inside that tree.\n\
         \n\
         {next}\
         merge-adapt did not merge and did not write {merged}.\n\
         READY_FOR_LIVE_TEST: no.\n",
        header = header(doc, seat_tag, local_tag, adapter_dir, &merged),
        output_dir = output_dir.display(),
        merged = merged.display(),
        next = next_lines(&convert, &gguf, &seat),
    );
    Ok(MergeAdaptPlan {
        seat_tag: seat_tag.to_string(),
        local_tag: local_tag.to_string(),
        pack_id: doc.pack_id.clone(),
        driver: doc.driver.clone(),
        merge_commands,
        merged_dir: merged,
        convert_command: convert,
        gguf_convert_command: gguf,
        local_seat_command: seat,
        report,
    })
}

/// Keys `examples/merge_lora/qwen3_lora_sft.yaml` sets, in that file's order.
const LLAMAFACTORY_EXPORT_KEYS: &[&str] = &[
    "model_name_or_path",
    "adapter_name_or_path",
    "template",
    "trust_remote_code",
    "export_dir",
    "export_size",
    "export_device",
    "export_legacy_format",
];

/// Real keys that turn the merge card into a quantized export.
/// Comment lines that name these words are not keys.
const LLAMAFACTORY_QUANT_KEYS: &[&str] = &[
    "quantization_bit",
    "quantization_method",
    "export_quantization_bit",
    "export_quantization_dataset",
];

struct LlamaFactoryExportCard {
    model: String,
    adapter_raw: String,
    template: String,
    trust_remote_code: String,
    export_raw: String,
    export_dir: PathBuf,
    export_size: String,
    export_device: String,
    export_legacy_format: String,
    finetuning_type: Option<String>,
    hub_model_id: Option<String>,
}

fn plan_llamafactory(
    prepared_dir: &Path,
    doc: &EnrichPrepareDoc,
    seat_tag: &str,
    local_tag: &str,
    adapter_dir: &Path,
) -> Result<MergeAdaptPlan, ModelError> {
    let train = doc.train_base_model.clone().ok_or_else(|| {
        ModelError::Other(
            "refuse:train-base: prepare.json has no train_base_model. export.yaml model_name_or_path is that train base. This factory does not invent a Hub repo and does not download weights.".into(),
        )
    })?;
    refuse_sacred_and_sku("train base", &train)?;
    let export_yaml = prepared_dir.join("export.yaml");
    let text = read_export_yaml(&export_yaml)?;
    refuse_sacred_and_sku("export.yaml", &text)?;
    refuse_raw_secrets(&text).map_err(map_feed)?;
    refuse_llamafactory_quant_keys(&text)?;
    let card = parse_llamafactory_export(prepared_dir, &text, adapter_dir)?;
    if card.model != train {
        return Err(ModelError::Other(format!(
            "refuse:train-base: export.yaml model_name_or_path is '{}'. prepare.json train_base_model is '{train}'. The merge reads model_name_or_path. These must name the same train base.",
            card.model
        )));
    }
    require_recipe_agrees(prepared_dir, &train, &card.template)?;
    let merged = card.export_dir.clone();
    refuse_sacred_and_sku("export_dir", &merged.display().to_string())?;
    single_line(
        "refuse:merge",
        "export directory",
        &merged.display().to_string(),
    )?;
    if pin_dir(&merged) == adapter_dir {
        return Err(ModelError::Other(format!(
            "refuse:merge: export_dir {} is the adapter directory. {} writes a separate export_dir. This factory does not merge into the adapter directory.",
            merged.display(),
            LLAMAFACTORY_MERGE_EXAMPLE
        )));
    }
    let export_line = printed_llamafactory_export_line(&export_yaml);
    let shape = printed_llamafactory_export_shape(&card);
    let convert = printed_convert_line(&merged);
    let gguf = gguf_convert_cli(prepared_dir, &merged);
    let seat = local_seat_cli(prepared_dir, &merged);
    let quant_note = if doc.driver == LLAMAFACTORY_QLORA_ID {
        "This card is llamafactory-qlora. The train recipe sets quantization_bit and quantization_method. The merge example says not to use a quantized model or quantization_bit when merging LoRA adapters. export.yaml omits those keys. This print does not add quantization_bit, quantization_method, export_quantization_bit, or export_quantization_dataset.\n\n"
    } else {
        "This card is llamafactory-lora. The train recipe omits quantization. The merge card stays unquantized. This print does not add quantization_bit, quantization_method, export_quantization_bit, or export_quantization_dataset.\n\n"
    };
    let finetune_note = match card.finetuning_type.as_deref() {
        Some(value) => format!(
            "This export.yaml also sets finetuning_type: {value}. {example} leaves that key unset. The command reads this file, including that key.\n\n",
            example = LLAMAFACTORY_MERGE_EXAMPLE,
        ),
        None => format!(
            "This export.yaml leaves finetuning_type unset, matching {example}.\n\n",
            example = LLAMAFACTORY_MERGE_EXAMPLE,
        ),
    };
    let hub_note = match card.hub_model_id.as_deref() {
        Some(id) => format!(
            "This export.yaml also sets export_hub_model_id: {id}. {example} does not. This factory does not push to a hub.\n\n",
            example = LLAMAFACTORY_MERGE_EXAMPLE,
        ),
        None => String::new(),
    };
    let report = format!(
        "{header}\
         \n\
         LLaMA-Factory documents this merge on {doc_url} and {readme}. The command is `llamafactory-cli export`. The README runs `{example_cmd}`. The docs page uses `llamafactory-cli export merge_config.yaml`. This prepare's card is {export_yaml}. The example says model_name_or_path must exist and match template, and adapter_name_or_path must match the adapter output path from training. This factory does not run the command, does not rewrite export.yaml, and does not download the train base.\n\
         \n\
         {export_line}\n\
         \n\
         {quant_note}\
         {finetune_note}\
         {hub_note}\
         The example keys are {keys}. The values this export.yaml sets:\n\
         \n\
         {shape}\n\
         \n\
         `llamafactory-cli export` writes export_dir. The file sets export_dir to {export_raw}. Resolved beside this prepare, that directory is {merged}. Current LLaMA-Factory export_model also writes Modelfile in that directory (FROM ., plus TEMPLATE from the train chat template). This factory does not write that Modelfile and does not invent a second template. LLaMA-Factory does not write GGUF. export_device choices in the example are cpu and auto. This file sets {device}. export_size is the shard size in gigabytes. This file sets {size}. export_legacy_format is {legacy}.\n\
         \n\
         {tokenizer}\
         \n\
         {next}\
         merge-adapt did not merge and did not write {merged}.\n\
         READY_FOR_LIVE_TEST: no.\n",
        header = header(doc, seat_tag, local_tag, adapter_dir, &merged),
        doc_url = LLAMAFACTORY_MERGE_DOC,
        readme = LLAMAFACTORY_MERGE_README,
        example_cmd = documented_llamafactory_export_example(),
        export_yaml = export_yaml.display(),
        export_raw = card.export_raw,
        merged = merged.display(),
        device = card.export_device,
        size = card.export_size,
        legacy = card.export_legacy_format,
        keys = LLAMAFACTORY_EXPORT_KEYS.join(", "),
        tokenizer = format!("{}\n", export_tokenizer_guidance(&train)),
        next = next_lines(&convert, &gguf, &seat),
    );
    Ok(MergeAdaptPlan {
        seat_tag: seat_tag.to_string(),
        local_tag: local_tag.to_string(),
        pack_id: doc.pack_id.clone(),
        driver: doc.driver.clone(),
        merge_commands: vec![export_line],
        merged_dir: merged,
        convert_command: convert,
        gguf_convert_command: gguf,
        local_seat_command: seat,
        report,
    })
}

/// `llamafactory-cli export <export.yaml>`. The README runs this on the merge example.
pub(crate) fn printed_llamafactory_export_line(export_yaml: &Path) -> String {
    format!(
        "llamafactory-cli export {}",
        shell_quote(&export_yaml.display().to_string())
    )
}

/// The README's published command. This prepare runs the same command on its own export.yaml.
pub(crate) fn documented_llamafactory_export_example() -> &'static str {
    "llamafactory-cli export examples/merge_lora/qwen3_lora_sft.yaml"
}

fn printed_llamafactory_export_shape(card: &LlamaFactoryExportCard) -> String {
    format!(
        "### model\n\
         model_name_or_path: {model}\n\
         adapter_name_or_path: {adapter}\n\
         template: {template}\n\
         trust_remote_code: {trust}\n\
         \n\
         ### export\n\
         export_dir: {export_dir}\n\
         export_size: {export_size}\n\
         export_device: {export_device}\n\
         export_legacy_format: {legacy}",
        model = card.model,
        adapter = card.adapter_raw,
        template = card.template,
        trust = card.trust_remote_code,
        export_dir = card.export_raw,
        export_size = card.export_size,
        export_device = card.export_device,
        legacy = card.export_legacy_format,
    )
}

fn parse_llamafactory_export(
    prepared_dir: &Path,
    text: &str,
    adapter_dir: &Path,
) -> Result<LlamaFactoryExportCard, ModelError> {
    let model = require_export_key(text, "model_name_or_path")?;
    refuse_sacred_and_sku("model_name_or_path", &model)?;
    let adapter_raw = require_export_key(text, "adapter_name_or_path")?;
    let adapter_resolved = resolve_beside_prepare(prepared_dir, &adapter_raw, "adapter_name_or_path")?;
    let adapter_pinned = match std::fs::canonicalize(&adapter_resolved) {
        Ok(path) => path,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(ModelError::Other(format!(
                "refuse:merge: export.yaml adapter_name_or_path is '{adapter_raw}' and {} is missing. {} says that path must be the adapter output directory. Pass that directory as --adapter. This factory does not invent the directory.",
                adapter_resolved.display(),
                LLAMAFACTORY_MERGE_EXAMPLE
            )));
        }
        Err(err) => {
            return Err(ModelError::Other(format!(
                "refuse:merge: cannot read adapter_name_or_path {}: {err}",
                adapter_resolved.display()
            )));
        }
    };
    if adapter_pinned != adapter_dir {
        return Err(ModelError::Other(format!(
            "refuse:merge: export.yaml adapter_name_or_path is '{adapter_raw}' ({}). --adapter is {}. {} says adapter_name_or_path must match the adapter output path. An early stop may leave the adapter under checkpoint-<step>. Point adapter_name_or_path at that directory and pass that directory as --adapter. This factory does not rewrite export.yaml.",
            adapter_pinned.display(),
            adapter_dir.display(),
            LLAMAFACTORY_MERGE_EXAMPLE
        )));
    }
    let template = require_export_key(text, "template")?;
    refuse_sacred_and_sku("template", &template)?;
    let trust = require_export_key(text, "trust_remote_code")?;
    if trust != "true" && trust != "false" {
        return Err(ModelError::Other(format!(
            "refuse:merge: trust_remote_code is '{trust}'. {example} sets true. This factory accepts true or false.",
            example = LLAMAFACTORY_MERGE_EXAMPLE
        )));
    }
    let export_raw = require_export_key(text, "export_dir")?;
    let export_dir = resolve_beside_prepare(prepared_dir, &export_raw, "export_dir")?;
    let export_size = require_export_key(text, "export_size")?;
    if !is_plain_positive_int(&export_size) {
        return Err(ModelError::Other(format!(
            "refuse:merge: export_size is '{export_size}'. {example} sets a shard size in gigabytes (5). This factory does not invent a size.",
            example = LLAMAFACTORY_MERGE_EXAMPLE
        )));
    }
    let export_device = require_export_key(text, "export_device")?;
    if export_device != "cpu" && export_device != "auto" {
        return Err(ModelError::Other(format!(
            "refuse:merge: export_device is '{export_device}'. {example} sets cpu and names choices cpu and auto. This factory does not invent another device.",
            example = LLAMAFACTORY_MERGE_EXAMPLE
        )));
    }
    let legacy = require_export_key(text, "export_legacy_format")?;
    if legacy != "true" && legacy != "false" {
        return Err(ModelError::Other(format!(
            "refuse:merge: export_legacy_format is '{legacy}'. {example} sets false. This factory accepts true or false.",
            example = LLAMAFACTORY_MERGE_EXAMPLE
        )));
    }
    let finetuning_type = match yaml_top_scalar(text, "finetuning_type")? {
        Some(value) if value == "lora" => Some(value),
        Some(value) => {
            return Err(ModelError::Other(format!(
                "refuse:merge: export.yaml finetuning_type is '{value}'. The LoRA merge example leaves that key unset. This prepare's cards set lora. This factory does not print a full-parameter export."
            )));
        }
        None => None,
    };
    let hub_model_id = yaml_top_scalar(text, "export_hub_model_id")?;
    if let Some(id) = hub_model_id.as_deref() {
        refuse_sacred_and_sku("export_hub_model_id", id)?;
    }
    Ok(LlamaFactoryExportCard {
        model,
        adapter_raw,
        template,
        trust_remote_code: trust,
        export_raw,
        export_dir,
        export_size,
        export_device,
        export_legacy_format: legacy,
        finetuning_type,
        hub_model_id,
    })
}

fn require_export_key(text: &str, key: &str) -> Result<String, ModelError> {
    match yaml_top_scalar(text, key)? {
        Some(value) => Ok(value),
        None => Err(ModelError::Other(format!(
            "refuse:merge: export.yaml has no {key}. {example} sets {key}. This factory does not invent that value.",
            example = LLAMAFACTORY_MERGE_EXAMPLE
        ))),
    }
}

fn refuse_llamafactory_quant_keys(text: &str) -> Result<(), ModelError> {
    for key in LLAMAFACTORY_QUANT_KEYS {
        if yaml_top_scalar(text, key)?.is_some() {
            return Err(ModelError::Other(format!(
                "refuse:export: export.yaml sets {key}. {example} says not to use a quantized model or quantization_bit when merging LoRA adapters. adapter_name_or_path and export_quantization_bit cannot both be set. Leave the merge card unquantized. This factory does not run the export.",
                example = LLAMAFACTORY_MERGE_EXAMPLE
            )));
        }
    }
    Ok(())
}

fn require_recipe_agrees(prepared: &Path, train: &str, template: &str) -> Result<(), ModelError> {
    let path = prepared.join("recipe.yaml");
    let text = match read_card(&path, "refuse:train-base") {
        Err(err) if err.to_string().contains("is missing") => {
            return Err(ModelError::Other(format!(
                "refuse:train-base: {} is missing, so the train base cannot be checked",
                path.display()
            )));
        }
        other => other?,
    };
    refuse_sacred_and_sku("recipe.yaml", &text)?;
    refuse_raw_secrets(&text).map_err(map_feed)?;
    let found = yaml_top_scalar(&text, "model_name_or_path")?.unwrap_or_default();
    if found != train {
        return Err(ModelError::Other(format!(
            "refuse:train-base: recipe.yaml model_name_or_path is '{found}'. prepare.json train_base_model is '{train}'. The merge reads the train base from export.yaml. These files must name the same train base."
        )));
    }
    let recipe_template = yaml_top_scalar(&text, "template")?.ok_or_else(|| {
        ModelError::Other(format!(
            "refuse:merge: {} has no template. export.yaml template is '{template}'. Train and export share one template. This factory does not pick a template.",
            path.display()
        ))
    })?;
    if recipe_template != template {
        return Err(ModelError::Other(format!(
            "refuse:merge: export.yaml template is '{template}'. recipe.yaml template is '{recipe_template}'. {example} says model_name_or_path must match template. Train and export share one template. This factory does not pick a template.",
            example = LLAMAFACTORY_MERGE_EXAMPLE
        )));
    }
    Ok(())
}

fn is_plain_positive_int(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_digit() && first != '0' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_digit())
}

/// Resolve a directory that may not exist yet. An existing directory is canonical.
fn pin_dir(path: &Path) -> PathBuf {
    if let Ok(canon) = std::fs::canonicalize(path) {
        return canon;
    }
    if let Some(parent) = path.parent() {
        if let Ok(parent) = std::fs::canonicalize(parent) {
            if let Some(name) = path.file_name() {
                return parent.join(name);
            }
        }
    }
    path.to_path_buf()
}

fn read_export_yaml(path: &Path) -> Result<String, ModelError> {
    match read_card(path, "refuse:merge") {
        Err(err) if err.to_string().contains("is missing") => Err(ModelError::Other(format!(
            "refuse:merge: {} is missing. The LLaMA-Factory merge card is export.yaml. {example} is the documented shape. This factory does not invent that file.",
            path.display(),
            example = LLAMAFACTORY_MERGE_EXAMPLE
        ))),
        other => other,
    }
}

fn plan_mlx(
    prepared_dir: &Path,
    doc: &EnrichPrepareDoc,
    seat_tag: &str,
    local_tag: &str,
    adapter: &Path,
) -> Result<MergeAdaptPlan, ModelError> {
    refuse_recipe_train_record(doc, prepared_dir)?;
    let train = doc.train_base_model.clone().ok_or_else(|| {
        ModelError::Other(
            "refuse:train-base: prepare.json has no train_base_model. mlx_lm.fuse --model is that train base. This factory does not invent a Hub repo and does not download weights.".into(),
        )
    })?;
    refuse_sacred_and_sku("train base", &train)?;
    let shape = classify_mlx_adapter(adapter)?;
    let fused = mlx_fused_dir(prepared_dir);
    let gguf = mlx_gguf_file(prepared_dir);
    refuse_sacred_and_sku("fused", &fused.display().to_string())?;
    refuse_sacred_and_sku("gguf", &gguf.display().to_string())?;
    single_line(
        "refuse:merge",
        "fused directory",
        &fused.display().to_string(),
    )?;
    single_line("refuse:merge", "gguf file", &gguf.display().to_string())?;
    let fuse = printed_mlx_fuse_line(&train, &shape.dir, &fused, false);
    let export = printed_mlx_fuse_line(&train, &shape.dir, &fused, true);
    let seat = local_seat_cli(prepared_dir, &gguf);
    let report = format!(
        "merge-adapt: shape=mlx-adapter seat_tag={seat_tag} local_tag={local_tag}\n\
         pack={pack_id}\n\
         driver={driver}\n\
         adapter={adapter}\n\
         fused={fused}\n\
         gguf={gguf}\n\
         promoted=false auto_apply=false estate_rewritten=false\n\
         \n\
         mlx-lm documents fuse on {lora_doc}. The page shows `{documented}`. That default loads adapters from adapters/ and writes the fused model in fused_model/. fuse.py documents --adapter-path (default adapters) and --save-path (default fused_model). This adapter directory is {adapter}. It holds adapter_config.json and adapters.safetensors. Those are the two files mlx_lm.lora writes and mlx_lm.fuse loads. This print passes --model from prepare.json train_base_model, --adapter-path for this directory, and --save-path {fused}. The directory name fused_model is the documented default. It sits beside this prepare so the later seat line names one path. This factory does not create {fused}.\n\
         \n\
         {fuse}\n\
         \n\
         GGUF export is on that same page: `mlx_lm.fuse --export-gguf`. The file is {gguf}. That is the save path plus ggml-model-f16.gguf, the --gguf-path default. This print does not pass --gguf-path. LORA.md limits that export to Mistral, Mixtral, and Llama style models in fp16 precision. fuse.py raises when model_type is not llama, mixtral, or mistral. This factory does not read the model config and does not guess model_type from the train base. The fused directory is MLX weights (model.safetensors plus config.json). That is not a merged Hugging Face directory. This print does not include a llama.cpp convert line. That convert card stays on llamafactory-lora, llamafactory-qlora, axolotl-lora, and axolotl-qlora. `mlx_lm.fuse --help` lists --dequantize and --gguf-path. This print does not add them.\n\
         \n\
         {export}\n\
         \n\
         After that file exists, local-seat prints the Ollama create line for the .gguf file, plus llama-cli -m and llama-server -m. Pass the file. A directory that also holds the fused weights matches more than one shape. --adapter does not print an Ollama ADAPTER Modelfile for an mlx adapter. import-trained records the adapter directory or that GGUF file. A fused MLX directory is refuse:adapter. It does not promote.\n\
         \n\
         {seat}\n\
         \n\
         merge-adapt did not fuse and did not write {fused} or {gguf}.\n\
         READY_FOR_LIVE_TEST: no.\n",
        pack_id = doc.pack_id.as_str(),
        driver = doc.driver.as_str(),
        adapter = shape.dir.display(),
        fused = fused.display(),
        gguf = gguf.display(),
        lora_doc = MLX_LORA_DOC,
        documented = crate::train_enrich::MLX_FUSE_COMMAND,
    );
    Ok(MergeAdaptPlan {
        seat_tag: seat_tag.to_string(),
        local_tag: local_tag.to_string(),
        pack_id: doc.pack_id.clone(),
        driver: doc.driver.clone(),
        merge_commands: vec![fuse, export.clone()],
        merged_dir: fused,
        convert_command: export,
        gguf_convert_command: String::new(),
        local_seat_command: seat,
        report,
    })
}

fn plan_unsloth(
    prepared_dir: &Path,
    doc: &EnrichPrepareDoc,
    seat_tag: &str,
    local_tag: &str,
    adapter: &Path,
) -> Result<MergeAdaptPlan, ModelError> {
    refuse_recipe_train_record(doc, prepared_dir)?;
    let shape = classify_unsloth_adapter(adapter)?;
    let merged = unsloth_merged_dir(prepared_dir);
    refuse_sacred_and_sku("merged", &merged.display().to_string())?;
    single_line(
        "refuse:merge",
        "merged directory",
        &merged.display().to_string(),
    )?;
    single_line(
        "refuse:merge",
        "adapter directory",
        &shape.dir.display().to_string(),
    )?;
    let merged_line = printed_unsloth_merged_16bit(&merged);
    let lora_lines = printed_unsloth_lora_save(&shape.dir);
    let reload = printed_unsloth_reload(&shape.dir);
    let convert = printed_convert_line(&merged);
    let gguf = gguf_convert_cli(prepared_dir, &merged);
    let seat = local_seat_cli(prepared_dir, &merged);
    let manual = printed_unsloth_manual_block(&merged);
    let direct = printed_unsloth_gguf_examples();
    let weights = shape.weights.join(", ");
    let report = format!(
        "{header}\
         \n\
         Unsloth documents the LoRA save on {vllm}. The page shows `model.save_pretrained(\"finetuned_lora\")` and `tokenizer.save_pretrained(\"finetuned_lora\")`, and `model.save_pretrained_merged(\"finetuned_model\", tokenizer, save_method = \"lora\")`. That save writes adapter_config.json and adapter_model.safetensors. adapter_model.bin is the file when safe_serialization is False. This adapter directory is {adapter}. It holds adapter_config.json and {weights}. This factory does not choose a rank and does not write a training script.\n\
         \n\
         {lora_lines}\n\
         \n\
         The same page saves a merged 16-bit directory with `model.save_pretrained_merged(\"finetuned_model\", tokenizer, save_method = \"merged_16bit\")`. The GGUF page uses the directory string merged_model for that same call ({gguf_doc}). The call is on the trained model object in that session. It does not take an adapter-directory flag. This print passes {merged} as that directory string so the later seat line names one path. The directory name merged sits beside this prepare. This factory does not create {merged} and does not write a Python file.\n\
         \n\
         {merged_line}\n\
         \n\
         When the trained model is no longer in memory, the inference page reloads a saved LoRA directory ({inference_doc}). The page sets model_name to \"lora_model\" and leaves max_seq_length, dtype, and load_in_4bit as the names from that training session. This print sets model_name to this adapter directory and does not fill those three names. It does not join the reload and the save into a script.\n\
         \n\
         {reload}\n\
         \n\
         Then call the merged_16bit line above on that model. This card does not print PeftModel.merge_and_unload. Unsloth's published 16-bit merge for this QLoRA card is save_method merged_16bit. The vLLM guide also lists save_method merged_4bit and says not to use it unless you know what the 4-bit merge is for. This print does not add merged_4bit. The GGUF page documents maximum_memory_usage on save_pretrained as a crash workaround. This print does not add that argument.\n\
         \n\
         The GGUF page's manual tab then runs llama.cpp convert_hf_to_gguf.py. The published lines are --outtype f16, bf16, and q8_0, each with --split-max-size 50G. The page's outfile names are model-F16.gguf, model-BF16.gguf, and model-Q8_0.gguf. This print uses {merged} where the page writes merged_model. It does not print the page's apt-get or cmake build. This factory does not clone llama.cpp and does not run these lines. Unsloth's page does not publish --outtype auto. The python3 line in the next card is the llama.cpp script default.\n\
         \n\
         {manual}\n\
         \n\
         The same page also publishes model.save_pretrained_gguf with quantization_method q4_k_m, q8_0, and f16. The directory argument on the page is the string directory. This print keeps that string. It does not choose one method and does not guess the .gguf file name that call writes. After the file exists, pass that file to local-seat --weights.\n\
         \n\
         {direct}\n\
         \n\
         The Ollama page ({ollama_doc}) exports to GGUF and says Unsloth writes a Modelfile. It does not publish an Ollama ADAPTER line for the PEFT directory. local-seat --adapter on this prepare is refuse:adapter.\n\
         \n\
         {next}\
         merge-adapt did not merge and did not write {merged}.\n\
         READY_FOR_LIVE_TEST: no.\n",
        header = header(doc, seat_tag, local_tag, &shape.dir, &merged),
        vllm = UNSLOTH_VLLM_DOC,
        gguf_doc = UNSLOTH_GGUF_DOC,
        inference_doc = UNSLOTH_INFERENCE_DOC,
        ollama_doc = UNSLOTH_OLLAMA_DOC,
        adapter = shape.dir.display(),
        weights = weights,
        merged = merged.display(),
        next = next_lines(&convert, &gguf, &seat),
    );
    Ok(MergeAdaptPlan {
        seat_tag: seat_tag.to_string(),
        local_tag: local_tag.to_string(),
        pack_id: doc.pack_id.clone(),
        driver: doc.driver.clone(),
        merge_commands: vec![merged_line],
        merged_dir: merged,
        convert_command: convert,
        gguf_convert_command: gguf,
        local_seat_command: seat,
        report,
    })
}

/// `save_pretrained_merged(..., save_method = "merged_16bit")` from the vLLM guide and the GGUF page.
pub(crate) fn printed_unsloth_merged_16bit(out: &Path) -> String {
    let out = json_quote(&out.display().to_string());
    format!("model.save_pretrained_merged({out}, tokenizer, save_method = \"merged_16bit\")")
}

/// LoRA save lines from the vLLM guide. The directory argument is this adapter.
pub(crate) fn printed_unsloth_lora_save(adapter: &Path) -> String {
    let adapter = json_quote(&adapter.display().to_string());
    format!(
        "model.save_pretrained({adapter})\n\
         tokenizer.save_pretrained({adapter})\n\
         model.save_pretrained_merged({adapter}, tokenizer, save_method = \"lora\")"
    )
}

/// Inference-page reload. Only `model_name` is this adapter directory.
pub(crate) fn printed_unsloth_reload(adapter: &Path) -> String {
    let adapter = json_quote(&adapter.display().to_string());
    format!(
        "from unsloth import FastLanguageModel\n\
         model, tokenizer = FastLanguageModel.from_pretrained(\n\
         model_name = {adapter},\n\
         max_seq_length = max_seq_length,\n\
         dtype = dtype,\n\
         load_in_4bit = load_in_4bit,\n\
         )"
    )
}

/// The three `save_pretrained_gguf` examples on the GGUF page. The directory string stays `directory`.
pub(crate) fn printed_unsloth_gguf_examples() -> String {
    "model.save_pretrained_gguf(\"directory\", tokenizer, quantization_method = \"q4_k_m\")\n\
     model.save_pretrained_gguf(\"directory\", tokenizer, quantization_method = \"q8_0\")\n\
     model.save_pretrained_gguf(\"directory\", tokenizer, quantization_method = \"f16\")"
        .to_string()
}

pub(crate) fn printed_unsloth_manual_convert(merged: &Path, outfile: &str, outtype: &str) -> String {
    format!(
        "python llama.cpp/convert_hf_to_gguf.py {} --outfile {} --outtype {} --split-max-size 50G",
        shell_quote(&merged.display().to_string()),
        shell_quote(outfile),
        outtype
    )
}

pub(crate) fn printed_unsloth_manual_block(merged: &Path) -> String {
    format!(
        "{f16}\n\
         {bf16}\n\
         {q8}",
        f16 = printed_unsloth_manual_convert(merged, "model-F16.gguf", "f16"),
        bf16 = printed_unsloth_manual_convert(merged, "model-BF16.gguf", "bf16"),
        q8 = printed_unsloth_manual_convert(merged, "model-Q8_0.gguf", "q8_0"),
    )
}

pub(crate) fn unsloth_merged_dir(prepared: &Path) -> PathBuf {
    prepared.join(UNSLOTH_MERGED_DIR_NAME)
}

fn json_quote(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}

/// Documented `mlx_lm.fuse` line. `--export-gguf` is the LORA.md GGUF flag.
pub(crate) fn printed_mlx_fuse_line(
    train_base: &str,
    adapter: &Path,
    save: &Path,
    export_gguf: bool,
) -> String {
    let mut line = format!(
        "mlx_lm.fuse --model {} --adapter-path {} --save-path {}",
        shell_quote(train_base),
        shell_quote(&adapter.display().to_string()),
        shell_quote(&save.display().to_string())
    );
    if export_gguf {
        line.push_str(" --export-gguf");
    }
    line
}

pub(crate) fn mlx_fused_dir(prepared: &Path) -> PathBuf {
    prepared.join(MLX_FUSED_DIR_NAME)
}

pub(crate) fn mlx_gguf_file(prepared: &Path) -> PathBuf {
    mlx_fused_dir(prepared).join(MLX_GGUF_FILE_NAME)
}

/// Operator card appended to `mlx-lm-lora` `PREPARE.md` and `NEXT.md`.
///
/// The adapter directory is chosen when the operator runs `mlx_lm.lora`.
/// This card names `merge-adapt` with `<adapter-dir>`. The fuse print fills
/// that path. The save path and the GGUF file are known from this prepare.
pub(crate) fn mlx_post_train_ladder(
    out_dir: &Path,
    seat_tag: &str,
    pack_id: &str,
    train_base: &str,
) -> String {
    let tag = local_enrich_tag(pack_id);
    let fused = mlx_fused_dir(out_dir);
    let gguf = mlx_gguf_file(out_dir);
    let merge_cli = printed_merge_adapt_cli(out_dir, Path::new("<adapter-dir>"));
    let fuse = printed_mlx_fuse_line(train_base, Path::new("<adapter-dir>"), &fused, false);
    let export = printed_mlx_fuse_line(train_base, Path::new("<adapter-dir>"), &fused, true);
    let seat_cli = local_seat_cli(out_dir, &gguf);
    let prepared = shell_quote(&out_dir.display().to_string());
    let gguf_q = shell_quote(&gguf.display().to_string());
    let tag_q = shell_quote(&tag);
    format!(
        "\n\
         ## After the mlx-lm train\n\
         \n\
         Seat tag is {seat}. That is the Ollama id this cell already runs. The create name is {tag}.\n\
         \n\
         Chain, outside this factory. This factory does not fuse, does not shell out, does not write weights, and does not promote.\n\
         \n\
         1. `mlx_lm.lora` writes the adapter directory. LORA.md says the default path is adapters/. `--adapter-path` changes it. That directory holds adapter_config.json and adapters.safetensors. `mlx_lm.fuse` loads those two files. A checkpoint named NNNNNNN_adapters.safetensors is not the file fuse loads. This factory does not choose the path.\n\
         2. Print the fuse. `merge-adapt` checks that directory and prints the documented `mlx_lm.fuse` line ({lora_doc}). The page shows `{documented}`. That default loads adapters/ and writes fused_model/ in the working directory. fuse.py documents `--adapter-path` (default adapters) and `--save-path` (default fused_model). This print passes those flags. The fused directory is {fused}, beside this prepare. This factory does not create that directory.\n\
         \n\
         {merge_cli}\n\
         \n\
         That prints:\n\
         \n\
         {fuse}\n\
         \n\
         mlx-lm writes {fused}.\n\
         3. Print the documented GGUF export. LORA.md shows `mlx_lm.fuse --export-gguf`. The file is {gguf} (`ggml-model-f16.gguf` inside the save path). `--gguf-path` changes the file name. This print keeps the default name. GGUF support on that page is limited to Mistral, Mixtral, and Llama style models in fp16 precision. fuse.py raises when model_type is not llama, mixtral, or mistral. This factory does not read the model config and does not guess model_type from the train base.\n\
         \n\
         {export}\n\
         \n\
         The fused directory is MLX weights (model.safetensors plus config.json). The Hugging Face convert card stays on the LLaMA-Factory and Axolotl prepares. This card does not point that card at {fused}. This factory does not invent a convert script.\n\
         4. Seat the GGUF file. `local-seat` already accepts one .gguf file. Point `--weights` at {gguf}. It prints `ollama create` and `llama-cli -m` and `llama-server -m` for that file. It does not create the model. A directory that holds both the fused weights and the GGUF matches more than one shape. Pass the file. `--adapter` does not print an Ollama ADAPTER Modelfile for this driver.\n\
         \n\
         {seat_cli}\n\
         \n\
         5. Record the artifact. import-trained accepts the adapter directory or that GGUF file. A fused MLX directory is refuse:adapter. The seat tag on the proposal stays {seat}. import-trained records trained_shape and trained_paths. import-trained does not apply and does not promote.\n\
         \n\
         estate enrich import-trained --estate <estate.yaml> --prepared {prepared} --tag {tag_q} --adapter '<adapter-dir>'\n\
         \n\
         estate enrich import-trained --estate <estate.yaml> --prepared {prepared} --tag {tag_q} --adapter {gguf_q}\n\
         \n\
         READY_FOR_LIVE_TEST: no.\n",
        seat = seat_tag,
        tag = tag,
        lora_doc = MLX_LORA_DOC,
        documented = crate::train_enrich::MLX_FUSE_COMMAND,
        fused = fused.display(),
        gguf = gguf.display(),
        prepared = prepared,
        tag_q = tag_q,
        gguf_q = gguf_q,
    )
}

/// Operator card appended to `unsloth-qlora` `PREPARE.md` and `NEXT.md`.
///
/// The adapter directory is chosen when the operator saves the LoRA.
/// This card names `merge-adapt` with `<adapter-dir>`. The save print fills
/// that path. The merged directory sits beside this prepare.
pub(crate) fn unsloth_post_train_ladder(out_dir: &Path, seat_tag: &str, pack_id: &str) -> String {
    let tag = local_enrich_tag(pack_id);
    let merged = unsloth_merged_dir(out_dir);
    let adapter = Path::new("<adapter-dir>");
    let merge_cli = printed_merge_adapt_cli(out_dir, adapter);
    let merged_line = printed_unsloth_merged_16bit(&merged);
    let lora_lines = printed_unsloth_lora_save(adapter);
    let convert = printed_convert_line(&merged);
    let gguf_cli = gguf_convert_cli(out_dir, &merged);
    let seat_cli = local_seat_cli(out_dir, &merged);
    let outfile = crate::gguf_convert::sibling_gguf_outfile(&merged);
    let manual = printed_unsloth_manual_block(&merged);
    let direct = printed_unsloth_gguf_examples();
    let prepared = shell_quote(&out_dir.display().to_string());
    let merged_q = shell_quote(&merged.display().to_string());
    let outfile_q = shell_quote(&outfile.display().to_string());
    let tag_q = shell_quote(&tag);
    format!(
        "\n\
         ## After the Unsloth train\n\
         \n\
         Seat tag is {seat}. That is the Ollama id this cell already runs. The create name is {tag}.\n\
         \n\
         Chain, outside this factory. This factory does not merge, does not shell out, does not write weights, and does not promote.\n\
         \n\
         1. Save the LoRA. The vLLM guide ({vllm}) publishes `model.save_pretrained(\"finetuned_lora\")` and `tokenizer.save_pretrained(\"finetuned_lora\")`, and `model.save_pretrained_merged(..., save_method = \"lora\")`. That directory holds adapter_config.json and adapter_model.safetensors. adapter_model.bin is the file when safe_serialization is False. This factory does not choose the path and does not choose a rank.\n\
         \n\
         {lora_lines}\n\
         \n\
         2. Print the 16-bit merge. `merge-adapt` checks that directory and prints `model.save_pretrained_merged(..., save_method = \"merged_16bit\")`. The vLLM guide uses the directory string finetuned_model. The GGUF page ({gguf_doc}) uses merged_model. The call is on the trained model object. It does not take an adapter-directory flag. This print passes {merged}, beside this prepare. This factory does not create that directory and does not write a Python file. It does not print PeftModel.merge_and_unload. It does not print save_method merged_4bit.\n\
         \n\
         {merge_cli}\n\
         \n\
         That prints:\n\
         \n\
         {merged_line}\n\
         \n\
         The inference page ({inference}) reloads a saved LoRA with FastLanguageModel.from_pretrained. model_name on that page is \"lora_model\". max_seq_length, dtype, and load_in_4bit stay the names from the training session. This factory does not fill those three names and does not join the reload and the save into a script.\n\
         3. Print the convert for that merged Hugging Face directory. The GGUF page's manual tab publishes `python llama.cpp/convert_hf_to_gguf.py` with --outtype f16, bf16, and q8_0, and --split-max-size 50G. This factory does not run those lines and does not print the page's apt-get or cmake build.\n\
         \n\
         {manual}\n\
         \n\
         `gguf-convert` is the same card llamafactory-lora, llamafactory-qlora, axolotl-lora, and axolotl-qlora use. It prints `python3 convert_hf_to_gguf.py` with --outtype auto, the llama.cpp script default. Unsloth's page does not publish --outtype auto. The outfile is {outfile}, a sibling of the merged directory.\n\
         \n\
         {gguf_cli}\n\
         \n\
         That prints:\n\
         \n\
         {convert}\n\
         \n\
         The GGUF page also publishes model.save_pretrained_gguf. The directory argument on the page is the string directory. This print keeps that string and does not guess the .gguf file name.\n\
         \n\
         {direct}\n\
         \n\
         4. Seat the merged directory, or the GGUF file. `local-seat --weights` prints the ollama create line. A merged directory still points at gguf-convert first. A GGUF file also prints llama-cli -m and llama-server -m. The Ollama page ({ollama}) exports to GGUF and says Unsloth writes a Modelfile. It does not publish an Ollama ADAPTER line for the PEFT directory. `local-seat --adapter` on this prepare is refuse:adapter.\n\
         \n\
         {seat_cli}\n\
         \n\
         After save_pretrained_gguf writes one .gguf file, point --weights at that file.\n\
         5. Record the artifact. import-trained accepts the adapter directory, the merged directory, or a .gguf file. The seat tag on the proposal stays {seat}. import-trained records trained_shape and trained_paths. import-trained does not apply and does not promote.\n\
         \n\
         estate enrich import-trained --estate <estate.yaml> --prepared {prepared} --tag {tag_q} --adapter '<adapter-dir>'\n\
         \n\
         estate enrich import-trained --estate <estate.yaml> --prepared {prepared} --tag {tag_q} --adapter {merged_q}\n\
         \n\
         estate enrich import-trained --estate <estate.yaml> --prepared {prepared} --tag {tag_q} --adapter {outfile_q}\n\
         \n\
         READY_FOR_LIVE_TEST: no.\n",
        seat = seat_tag,
        tag = tag,
        vllm = UNSLOTH_VLLM_DOC,
        gguf_doc = UNSLOTH_GGUF_DOC,
        inference = UNSLOTH_INFERENCE_DOC,
        ollama = UNSLOTH_OLLAMA_DOC,
        merged = merged.display(),
        outfile = outfile.display(),
        prepared = prepared,
        tag_q = tag_q,
        merged_q = merged_q,
        outfile_q = outfile_q,
    )
}

/// `mlx_lm.fuse` refuses a directory that is not the adapter it writes.
fn classify_mlx_adapter(adapter: &Path) -> Result<crate::local_seat::AdapterDir, ModelError> {
    let shape = classify_adapter(adapter)?;
    if !shape.weights.is_empty() {
        let names = shape.weights.join(", ");
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} has PEFT adapter weights ({names}). mlx_lm.lora writes {MLX_ADAPTER_WEIGHTS} beside adapter_config.json. mlx_lm.fuse loads {MLX_ADAPTER_WEIGHTS}. This factory does not treat adapter_model.safetensors as an mlx adapter.",
            shape.dir.display()
        )));
    }
    let weights = shape.dir.join(MLX_ADAPTER_WEIGHTS);
    match std::fs::symlink_metadata(&weights) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(ModelError::Other(format!(
                "refuse:adapter: {} is a symlink. enrich does not follow marker symlinks. The marker must be a regular file inside {}.",
                weights.display(),
                shape.dir.display()
            )));
        }
        Ok(meta) if meta.is_file() => {}
        Ok(_) => {
            return Err(ModelError::Other(format!(
                "refuse:adapter: {} is not a regular file. mlx_lm.fuse loads {MLX_ADAPTER_WEIGHTS} as a regular file.",
                weights.display()
            )));
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(ModelError::Other(format!(
                "refuse:adapter: {} has adapter_config.json and no {MLX_ADAPTER_WEIGHTS}. mlx_lm.lora writes both. mlx_lm.fuse loads {MLX_ADAPTER_WEIGHTS}. A checkpoint file ending in _adapters.safetensors is not that file. This factory does not invent the weight file.",
                shape.dir.display()
            )));
        }
        Err(err) => {
            return Err(ModelError::Other(format!(
                "refuse:adapter: {}: {err}",
                weights.display()
            )));
        }
    }
    let file =
        open_nofollow(&weights).map_err(|err| open_error(&weights, "refuse:adapter", err))?;
    let meta = file.metadata().map_err(|err| {
        ModelError::Other(format!(
            "refuse:adapter: cannot stat {}: {err}",
            weights.display()
        ))
    })?;
    if !meta.is_file() {
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} is not a regular file. mlx_lm.fuse loads {MLX_ADAPTER_WEIGHTS} as a regular file.",
            weights.display()
        )));
    }
    Ok(shape)
}

/// Fused MLX weights are not a Hugging Face directory `gguf-convert` accepts.
pub(crate) fn refuse_mlx_hf_weights(command: &str, dir: &Path) -> ModelError {
    ModelError::Other(format!(
        "refuse:seat: {command} does not treat {} as a merged Hugging Face directory. mlx_lm.fuse writes MLX weights (model.safetensors) and config.json under its --save-path. LORA.md documents GGUF export as mlx_lm.fuse --export-gguf, which writes {MLX_GGUF_FILE_NAME} inside that directory. GGUF support there is limited to Mistral, Mixtral, and Llama style models in fp16. Seat that file with estate enrich local-seat --weights. Pass the file, not a directory that also holds the fused weights. estate enrich merge-adapt prints the fuse line. This factory does not invent a convert script.",
        dir.display()
    ))
}

pub(crate) fn refuse_mlx_adapter_weights(dir: &Path) -> ModelError {
    ModelError::Other(format!(
        "refuse:seat: {} is an adapter directory. On mlx-lm-lora, --weights is the GGUF file mlx_lm.fuse --export-gguf writes ({MLX_GGUF_FILE_NAME}). --adapter does not print an Ollama ADAPTER Modelfile for an mlx adapter (adapter_config.json and {MLX_ADAPTER_WEIGHTS}). estate enrich merge-adapt prints the fuse line. This factory does not fuse.",
        dir.display()
    ))
}

pub(crate) fn refuse_mlx_adapter_seat() -> ModelError {
    ModelError::Other(format!(
        "refuse:adapter: mlx-lm-lora adapters are not an Ollama ADAPTER directory. mlx_lm.lora writes adapter_config.json and {MLX_ADAPTER_WEIGHTS}. Fuse with mlx_lm.fuse. estate enrich merge-adapt prints that line. Then pass {MLX_GGUF_FILE_NAME} to local-seat --weights. This factory does not print an ADAPTER Modelfile for this driver."
    ))
}

/// Unsloth's lora save writes `adapter_model.safetensors` (or `.bin`) beside `adapter_config.json`.
fn classify_unsloth_adapter(adapter: &Path) -> Result<crate::local_seat::AdapterDir, ModelError> {
    let shape = classify_adapter(adapter)?;
    if shape.weights.is_empty() {
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} has adapter_config.json and no {UNSLOTH_ADAPTER_WEIGHTS}. Unsloth's lora save writes adapter_config.json and {UNSLOTH_ADAPTER_WEIGHTS}. {UNSLOTH_ADAPTER_BIN} is that file when safe_serialization is False. This factory does not invent the weight file.",
            shape.dir.display()
        )));
    }
    Ok(shape)
}

/// `--weights` on the PEFT directory is not the Unsloth Ollama seat.
pub(crate) fn refuse_unsloth_adapter_weights(dir: &Path) -> ModelError {
    ModelError::Other(format!(
        "refuse:seat: {} is an adapter directory. On unsloth-qlora, --weights is the merged 16-bit directory (config.json and a .safetensors file whose name does not start with adapter_model) or a GGUF file. --adapter does not print an Ollama adapter Modelfile for this PEFT directory (adapter_config.json and {UNSLOTH_ADAPTER_WEIGHTS}). Unsloth documents Ollama through a GGUF. estate enrich merge-adapt prints save_pretrained_merged with save_method merged_16bit. This factory does not merge.",
        dir.display()
    ))
}

/// Ollama `ADAPTER` is not the path Unsloth publishes for this PEFT directory.
pub(crate) fn refuse_unsloth_adapter_seat() -> ModelError {
    ModelError::Other(
        "refuse:adapter: unsloth-qlora adapters are not an Ollama adapter directory. Unsloth's vLLM guide saves the LoRA as adapter_config.json and adapter_model.safetensors (model.save_pretrained, or save_pretrained_merged with save_method \"lora\"). The saving-to-gguf page and the saving-to-ollama page seat a GGUF: model.save_pretrained_gguf, or save_pretrained_merged with save_method \"merged_16bit\" then llama.cpp convert_hf_to_gguf.py. This factory does not print an Ollama adapter Modelfile for the PEFT directory. Seat the merged directory or the GGUF with local-seat --weights. estate enrich merge-adapt prints the merged_16bit line.".into(),
    )
}

/// A fused directory that also holds the exported GGUF matches two shapes.
pub(crate) fn refuse_mlx_mixed_weights(path: &Path) -> ModelError {
    ModelError::Other(format!(
        "refuse:seat: {} matches more than one shape. mlx_lm.fuse --export-gguf writes {MLX_GGUF_FILE_NAME} inside the fused directory, beside the MLX weights. Pass that file to local-seat --weights. This factory does not seat the directory and does not invent a convert script.",
        path.display()
    ))
}

fn header(
    doc: &EnrichPrepareDoc,
    seat_tag: &str,
    local_tag: &str,
    adapter_dir: &Path,
    merged: &Path,
) -> String {
    format!(
        "merge-adapt: shape=adapter seat_tag={seat_tag} local_tag={local_tag}\n\
         pack={pack_id}\n\
         driver={driver}\n\
         adapter={adapter}\n\
         merged={merged}\n\
         promoted=false auto_apply=false estate_rewritten=false\n",
        pack_id = doc.pack_id.as_str(),
        driver = doc.driver.as_str(),
        adapter = adapter_dir.display(),
        merged = merged.display(),
    )
}

fn next_lines(convert: &str, gguf: &str, seat: &str) -> String {
    format!(
        "Then print the convert and the seat for that merged directory. gguf-convert checks the directory and prints the llama.cpp line. local-seat prints the ollama create line. A merged directory still points at gguf-convert first. Neither command merges, converts, or promotes.\n\
         \n\
         {gguf}\n\
         \n\
         That prints:\n\
         \n\
         {convert}\n\
         \n\
         {seat}\n\
         \n"
    )
}

fn refuse_card_mismatch(driver: &str, text: &str) -> Result<(), ModelError> {
    let load4 = yaml_top_scalar(text, "load_in_4bit")?;
    let adapter = yaml_top_scalar(text, "adapter")?;
    let yaml_quant = load4.as_deref() == Some("true") || adapter.as_deref() == Some("qlora");
    let yaml_lora = load4.as_deref() == Some("false") && adapter.as_deref() == Some("lora");
    if driver == AXOLOTL_QLORA_ID && yaml_lora {
        return Err(ModelError::Other(
            "refuse:merge: prepare driver is axolotl-qlora and axolotl.yml says bf16 LoRA (load_in_4bit false, adapter lora). The merge line follows one card.".into(),
        ));
    }
    if driver == AXOLOTL_LORA_ID && yaml_quant {
        return Err(ModelError::Other(
            "refuse:merge: prepare driver is axolotl-lora and axolotl.yml says 4-bit QLoRA. The merge line follows one card.".into(),
        ));
    }
    Ok(())
}

fn resolve_beside_prepare(prepared: &Path, raw: &str, label: &str) -> Result<PathBuf, ModelError> {
    if raw.starts_with('~') {
        return Err(ModelError::Other(format!(
            "refuse:merge: {label} starts with ~. This factory does not expand a home directory."
        )));
    }
    single_line("refuse:merge", label, raw)?;
    let path = PathBuf::from(raw);
    let resolved = if path.is_absolute() {
        path
    } else {
        prepared.join(path)
    };
    refuse_sacred_and_sku(label, &resolved.display().to_string())?;
    Ok(resolved)
}

fn yaml_top_scalar(text: &str, key: &str) -> Result<Option<String>, ModelError> {
    let prefix = format!("{key}:");
    let mut found = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix(&prefix) else {
            continue;
        };
        if found.is_some() {
            return Err(ModelError::Other(format!(
                "refuse:merge: more than one {key} line. The merge print reads one."
            )));
        }
        let value = unquote_yaml_scalar(rest.trim())
            .map_err(|err| ModelError::Other(format!("refuse:merge: {key} {err}")))?;
        if value.is_empty() {
            return Err(ModelError::Other(format!("refuse:merge: {key} is empty")));
        }
        found = Some(value);
    }
    Ok(found)
}

fn unquote_yaml_scalar(raw: &str) -> Result<String, String> {
    if raw.starts_with('"') {
        let bytes = raw.as_bytes();
        let mut escaped = false;
        let mut end = None;
        for (i, byte) in bytes.iter().enumerate().skip(1) {
            if escaped {
                escaped = false;
                continue;
            }
            if *byte == b'\\' {
                escaped = true;
                continue;
            }
            if *byte == b'"' {
                end = Some(i);
                break;
            }
        }
        let Some(end) = end else {
            return Err("is missing a closing double quote".to_string());
        };
        let literal = &raw[..=end];
        serde_json::from_str(literal).map_err(|err| format!("is not a quoted string ({err})"))
    } else if raw.starts_with('\'') {
        if !raw.ends_with('\'') || raw.len() < 2 {
            return Err("is missing a closing single quote".to_string());
        }
        Ok(raw[1..raw.len() - 1].replace("''", "'"))
    } else {
        let token = raw.split_whitespace().next().unwrap_or("");
        if token.is_empty() {
            Err("is empty".to_string())
        } else {
            Ok(token.to_string())
        }
    }
}

fn read_axolotl_yaml(path: &Path) -> Result<String, ModelError> {
    match read_card(path, "refuse:merge") {
        Err(err) if err.to_string().contains("is missing") => Err(ModelError::Other(format!(
            "refuse:merge: {} is missing. axolotl merge-lora reads the yaml this prepare wrote.",
            path.display()
        ))),
        other => other,
    }
}

fn read_card(path: &Path, refuse: &str) -> Result<String, ModelError> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(ModelError::Other(format!(
                "{refuse}: {} is missing",
                path.display()
            )));
        }
        Err(err) => {
            return Err(ModelError::Other(format!(
                "{refuse}: {}: {err}",
                path.display()
            )));
        }
    };
    if meta.file_type().is_symlink() {
        return Err(ModelError::Other(format!(
            "{refuse}: {} is a symlink. enrich does not follow it.",
            path.display()
        )));
    }
    if !meta.is_file() {
        return Err(ModelError::Other(format!(
            "{refuse}: {} is not a regular file",
            path.display()
        )));
    }
    if meta.len() > CARD_MAX_BYTES {
        return Err(ModelError::Other(format!(
            "{refuse}: {} is larger than {CARD_MAX_BYTES} bytes",
            path.display()
        )));
    }
    let mut file = open_nofollow(path).map_err(|err| open_error(path, refuse, err))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .map_err(|err| ModelError::Other(format!("{refuse}: {}: {err}", path.display())))?;
    Ok(buf)
}

fn open_error(path: &Path, refuse: &str, err: std::io::Error) -> ModelError {
    if matches!(err.raw_os_error(), Some(40 | 62)) {
        ModelError::Other(format!(
            "{refuse}: {} is a symlink. enrich does not follow it.",
            path.display()
        ))
    } else {
        ModelError::Other(format!("{refuse}: {}: {err}", path.display()))
    }
}

/// Open `path` without following a final symlink. Linux `O_NOFOLLOW` is
/// `0x20000`. macOS `O_NOFOLLOW` is `0x100`.
fn open_nofollow(path: &Path) -> std::io::Result<std::fs::File> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        #[cfg(target_os = "linux")]
        const O_NOFOLLOW: i32 = 0x20000;
        #[cfg(target_os = "macos")]
        const O_NOFOLLOW: i32 = 0x100;
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW)
            .open(path)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "O_NOFOLLOW is unavailable",
        ))
    }
}

fn single_line(code: &str, label: &str, value: &str) -> Result<(), ModelError> {
    if value.chars().any(|c| matches!(c, '\n' | '\r' | '\0')) {
        return Err(ModelError::Other(format!(
            "{code}: {label} is not a single line"
        )));
    }
    Ok(())
}

fn map_feed(err: FeedError) -> ModelError {
    let text = err.to_string();
    if text.starts_with("refuse:") {
        ModelError::Other(text)
    } else {
        ModelError::Other(format!("refuse:merge: {text}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::train_enrich::{LLAMAFACTORY_LORA_ID, LLAMAFACTORY_QLORA_ID, PREPARE_SCHEMA};

    fn tmp(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cell-one-merge-adapt-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_prepare(
        dir: &Path,
        driver: &str,
        job: &str,
        seat: Option<&str>,
        train_base: Option<&str>,
        promoted: bool,
    ) {
        let seat_line = match seat {
            Some(seat) => format!("  \"seat_tag\": \"{seat}\",\n"),
            None => String::new(),
        };
        let train_line = match train_base {
            Some(train) => format!("  \"train_base_model\": \"{train}\",\n"),
            None => String::new(),
        };
        let body = format!(
            "{{\n\
               \"schema\": \"{PREPARE_SCHEMA}\",\n\
               \"driver\": \"{driver}\",\n\
               \"job\": \"{job}\",\n\
               \"pack_id\": \"overnight-traces\",\n\
               \"base_model\": \"llama3\",\n\
               {seat_line}\
               {train_line}\
               \"purpose\": \"fixture\",\n\
               \"host_class_affinity\": \"any\",\n\
               \"source_paths\": [],\n\
               \"source_drivers\": [],\n\
               \"artifacts\": [],\n\
               \"promoted\": {promoted},\n\
               \"auto_apply\": false,\n\
               \"estate_rewritten\": false,\n\
               \"note\": \"test\"\n\
             }}\n"
        );
        std::fs::write(dir.join("prepare.json"), body).unwrap();
    }

    fn write_yaml(dir: &Path, output_dir: &Path, adapter: &str, load4: bool) {
        let body = format!(
            "# output_dir: /wrong/from/a/comment\n\
             base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"\n\
             load_in_4bit: {load4}\n\
             adapter: {adapter}\n\
             output_dir: \"{out}\"\n",
            load4 = if load4 { "true" } else { "false" },
            out = output_dir.display(),
        );
        std::fs::write(dir.join("axolotl.yml"), body).unwrap();
    }

    fn adapter_dir(dir: &Path) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("adapter_config.json"), "{}\n").unwrap();
        dir.to_path_buf()
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut found = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        found.sort();
        found
    }

    #[test]
    fn source_does_not_spawn_or_merge() {
        let src = include_str!("merge_adapt.rs");
        let command_new = format!("{}::{}", "Command", "new");
        let process_command = format!("{}::{}::{}", "std", "process", "Command");
        assert!(!src.contains(&command_new));
        assert!(!src.contains(&process_command));
        let banned_stack = format!("{}{}", "merge", "kit");
        let banned_spawn = format!("{}{}", "sub", "process");
        assert!(!src.contains(&banned_stack));
        assert!(!src.contains(&banned_spawn));
    }

    #[test]
    fn axolotl_lora_prints_merge_lora_and_writes_nothing() {
        let root = tmp("lora");
        write_prepare(
            &root,
            AXOLOTL_LORA_ID,
            "train",
            Some("llama3"),
            Some("Qwen/Qwen2.5-0.5B-Instruct"),
            false,
        );
        let outputs = root.join("outputs");
        write_yaml(&root, &outputs, "lora", false);
        let adapter = adapter_dir(&outputs);
        let before = names(&root);
        let prepare_before = std::fs::read(root.join("prepare.json")).unwrap();
        let plan = plan_merge_adapt(&root, &adapter).unwrap();
        let merged = outputs.join("merged");
        assert_eq!(plan.merged_dir, merged);
        assert_eq!(plan.driver, AXOLOTL_LORA_ID);
        assert_eq!(plan.seat_tag, "llama3");
        assert_eq!(plan.local_tag, "cell-enrich-overnight-traces");
        assert_eq!(plan.merge_commands.len(), 1);
        let line = printed_axolotl_merge_line(&root.join("axolotl.yml"), &adapter, false);
        assert_eq!(plan.merge_commands[0], line);
        assert!(
            plan.merge_commands[0].starts_with("axolotl merge-lora "),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            plan.merge_commands[0].contains("--lora-model-dir="),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            !plan.merge_commands[0].contains("--lora_model_dir"),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            !plan.merge_commands[0].contains("--dequant"),
            "{}",
            plan.merge_commands[0]
        );
        assert!(!plan.report.contains("--dequant"), "{}", plan.report);
        assert!(plan.report.contains(&line), "{}", plan.report);
        assert!(
            plan.report.contains("does not take `--out`"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains(&merged.display().to_string()),
            "{}",
            plan.report
        );
        assert_eq!(plan.gguf_convert_command, gguf_convert_cli(&root, &merged));
        assert_eq!(plan.local_seat_command, local_seat_cli(&root, &merged));
        assert_eq!(plan.convert_command, printed_convert_line(&merged));
        assert!(
            plan.report.contains(&plan.gguf_convert_command),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains(&plan.local_seat_command),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("merge-adapt did not merge"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("READY_FOR_LIVE_TEST: no"),
            "{}",
            plan.report
        );
        assert!(!plan.report.contains("mlx_lm.fuse"), "{}", plan.report);
        assert!(
            !plan.report.contains("READY_FOR_LIVE_TEST: yes"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("promoted=false"), "{}", plan.report);
        assert!(!merged.exists());
        assert_eq!(names(&root), before);
        assert_eq!(
            std::fs::read(root.join("prepare.json")).unwrap(),
            prepare_before
        );
    }

    #[test]
    fn axolotl_qlora_prints_the_dequant_line() {
        let root = tmp("qlora");
        write_prepare(
            &root,
            AXOLOTL_QLORA_ID,
            "train",
            Some("llama3"),
            Some("Qwen/Qwen2.5-0.5B-Instruct"),
            false,
        );
        let outputs = root.join("outputs");
        write_yaml(&root, &outputs, "qlora", true);
        let adapter = adapter_dir(&outputs);
        let plan = plan_merge_adapt(&root, &adapter).unwrap();
        assert_eq!(plan.merge_commands.len(), 2);
        assert!(
            !plan.merge_commands[0].contains("--dequant"),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            plan.merge_commands[1].ends_with(" --dequant"),
            "{}",
            plan.merge_commands[1]
        );
        assert!(
            plan.report.contains(&plan.merge_commands[1]),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("bf16 checkpoint"), "{}", plan.report);
        assert_eq!(plan.merged_dir, outputs.join("merged"));
        assert!(!plan.merged_dir.exists());
    }

    #[test]
    fn relative_output_dir_resolves_beside_the_prepared_tree() {
        let root = tmp("relative");
        write_prepare(&root, AXOLOTL_LORA_ID, "train", Some("llama3"), None, false);
        std::fs::write(
            root.join("axolotl.yml"),
            "base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"\noutput_dir: outputs\n",
        )
        .unwrap();
        let adapter = adapter_dir(&root.join("outputs"));
        let plan = plan_merge_adapt(&root, &adapter).unwrap();
        assert_eq!(plan.merged_dir, root.join("outputs").join("merged"));
    }

    #[test]
    fn checkpoint_adapter_still_names_output_dir_merged() {
        let root = tmp("checkpoint");
        write_prepare(&root, AXOLOTL_LORA_ID, "train", Some("llama3"), None, false);
        let outputs = root.join("outputs");
        write_yaml(&root, &outputs, "lora", false);
        let checkpoint = adapter_dir(&outputs.join("checkpoint-100"));
        let plan = plan_merge_adapt(&root, &checkpoint).unwrap();
        assert_eq!(plan.merged_dir, outputs.join("merged"));
        assert!(
            plan.merge_commands[0].contains(&checkpoint.display().to_string()),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            !plan.merged_dir.starts_with(&checkpoint),
            "{}",
            plan.merged_dir.display()
        );
    }

    #[test]
    fn shell_quote_wraps_the_printed_merge_line() {
        let root = tmp("quote");
        let spaced = root.join("my prepare");
        std::fs::create_dir_all(&spaced).unwrap();
        write_prepare(
            &spaced,
            AXOLOTL_LORA_ID,
            "train",
            Some("llama3"),
            None,
            false,
        );
        let outputs = spaced.join("out dir");
        write_yaml(&spaced, &outputs, "lora", false);
        let adapter = adapter_dir(&outputs);
        let plan = plan_merge_adapt(&spaced, &adapter).unwrap();
        assert!(
            plan.merge_commands[0].contains(&format!("'{}'", spaced.join("axolotl.yml").display())),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            plan.merge_commands[0].contains(&format!("--lora-model-dir='{}'", adapter.display())),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            plan.gguf_convert_command
                .contains(&format!("'{}'", plan.merged_dir.display())),
            "{}",
            plan.gguf_convert_command
        );
    }

    const LF_TRAIN: &str = "Qwen/Qwen2.5-0.5B-Instruct";

    fn write_lf_cards(dir: &Path, adapter: &Path, export_dir: &Path, extra: &str) {
        write_lf_cards_with(dir, LF_TRAIN, "qwen", adapter, export_dir, extra, true);
    }

    fn write_lf_cards_with(
        dir: &Path,
        model: &str,
        template: &str,
        adapter: &Path,
        export_dir: &Path,
        extra: &str,
        finetuning_type: bool,
    ) {
        let finetune = if finetuning_type {
            "finetuning_type: lora\n"
        } else {
            ""
        };
        let export = format!(
            "# quantization_bit stays off this merge card\n\
             # DO NOT use quantized model or quantization_bit when merging lora adapters\n\
             model_name_or_path: \"{model}\"\n\
             adapter_name_or_path: \"{adapter}\"\n\
             template: {template}\n\
             trust_remote_code: true\n\
             {finetune}\
             export_dir: \"{export_dir}\"\n\
             export_size: 5\n\
             export_device: cpu\n\
             export_legacy_format: false\n\
             {extra}",
            adapter = adapter.display(),
            export_dir = export_dir.display(),
        );
        std::fs::write(dir.join("export.yaml"), export).unwrap();
        std::fs::write(
            dir.join("recipe.yaml"),
            format!(
                "model_name_or_path: \"{model}\"\ntemplate: {template}\noutput_dir: outputs\n"
            ),
        )
        .unwrap();
    }

    fn assert_guidance_refuse_before_rerun(text: &str) {
        let claim = "returns refuse:tokenizer for that export before the restore";
        let claim_at = text
            .find(claim)
            .unwrap_or_else(|| panic!("missing refuse-before-restore claim in {text}"));
        let rerun_at = text
            .find("Then re-run estate enrich gguf-convert")
            .unwrap_or_else(|| panic!("missing re-run in {text}"));
        assert!(
            claim_at < rerun_at,
            "the refuse claim must precede the re-run: {text}"
        );
        assert!(
            !text[rerun_at..].contains("returns refuse:tokenizer"),
            "the re-run must not be followed by the refuse claim: {text}"
        );
    }

    fn assert_lf_export(plan: &MergeAdaptPlan, root: &Path, export_dir: &Path, driver: &str) {
        let export_yaml = root.join("export.yaml");
        let line = printed_llamafactory_export_line(&export_yaml);
        assert_eq!(plan.driver, driver);
        assert_eq!(plan.merged_dir, export_dir);
        assert_eq!(plan.merge_commands, vec![line.clone()]);
        assert!(plan.merge_commands[0].starts_with("llamafactory-cli export "));
        assert!(!plan.merge_commands[0].contains("quantization"));
        assert!(!plan.merge_commands[0].contains("merge_and_unload"));
        assert!(plan.report.contains(&line), "{}", plan.report);
        assert!(
            plan.report.contains(documented_llamafactory_export_example()),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains(LLAMAFACTORY_MERGE_EXAMPLE),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("### model"), "{}", plan.report);
        assert!(plan.report.contains("### export"), "{}", plan.report);
        assert!(
            plan.report.contains("model_name_or_path: Qwen/Qwen2.5-0.5B-Instruct"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("template: qwen"), "{}", plan.report);
        assert!(
            plan.report.contains("trust_remote_code: true"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("export_size: 5"), "{}", plan.report);
        assert!(plan.report.contains("export_device: cpu"), "{}", plan.report);
        assert!(
            plan.report.contains("export_legacy_format: false"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains(&export_dir.display().to_string()),
            "{}",
            plan.report
        );
        assert!(!plan.report.contains("axolotl merge-lora"), "{}", plan.report);
        assert!(!plan.report.contains("mlx_lm.fuse"), "{}", plan.report);
        assert!(
            !plan.report.contains("save_pretrained_merged"),
            "{}",
            plan.report
        );
        assert!(!plan.report.contains("merge_and_unload"), "{}", plan.report);
        assert!(
            !plan.report.contains("\nquantization_bit:"),
            "{}",
            plan.report
        );
        assert!(
            !plan.report.contains("\nexport_quantization_bit:"),
            "{}",
            plan.report
        );
        assert_eq!(plan.gguf_convert_command, gguf_convert_cli(root, export_dir));
        assert_eq!(plan.local_seat_command, local_seat_cli(root, export_dir));
        assert_eq!(plan.convert_command, printed_convert_line(export_dir));
        assert!(
            plan.report.contains(&plan.gguf_convert_command),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("refuse:tokenizer"), "{}", plan.report);
        assert_guidance_refuse_before_rerun(&plan.report);
        assert!(
            plan.report.contains("extra_special_tokens"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("vocab.json"), "{}", plan.report);
        assert!(plan.report.contains("merges.txt"), "{}", plan.report);
        assert!(
            plan.report.contains("tokenizer_config.json.bak"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("HF cache snapshot"), "{}", plan.report);
        assert!(
            plan.report.contains("HF hub snapshots are often symlinks"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("cp -aL"), "{}", plan.report);
        assert!(plan.report.contains("cp --dereference"), "{}", plan.report);
        assert!(
            plan.report.contains("real files, not symlinks"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("plain cp -a"), "{}", plan.report);
        assert!(
            plan.report.contains("does not follow a symlinked tokenizer_config.json"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("equivalent base checkout"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("into the export directory"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("re-run estate enrich gguf-convert"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains(&plan.local_seat_command),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("python3 convert_hf_to_gguf.py"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("--outtype auto"), "{}", plan.report);
        assert!(
            plan.report.contains("merge-adapt did not merge"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("READY_FOR_LIVE_TEST: no"),
            "{}",
            plan.report
        );
        assert!(
            !plan.report.contains("READY_FOR_LIVE_TEST: yes"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("promoted=false"), "{}", plan.report);
        assert!(!export_dir.exists());
        assert!(!root.join("merged").exists());
    }

    #[test]
    fn llamafactory_qlora_prints_export_and_writes_nothing() {
        let root = tmp("lf-qlora");
        write_prepare(
            &root,
            LLAMAFACTORY_QLORA_ID,
            "train",
            Some("llama3"),
            Some(LF_TRAIN),
            false,
        );
        let adapter = adapter_dir(&root.join("outputs"));
        let export_dir = root.join("export");
        write_lf_cards(
            &root,
            &adapter,
            &export_dir,
            "# quantization_bit: 4\n# quantization_method: bnb\n# export_quantization_bit: 4\n",
        );
        let before = names(&root);
        let prepare_before = std::fs::read(root.join("prepare.json")).unwrap();
        let export_before = std::fs::read(root.join("export.yaml")).unwrap();
        let plan = plan_merge_adapt(&root, &adapter).unwrap();
        assert_lf_export(&plan, &root, &export_dir, LLAMAFACTORY_QLORA_ID);
        assert!(plan.report.contains("llamafactory-qlora"), "{}", plan.report);
        assert!(
            plan.report.contains("finetuning_type: lora"),
            "{}",
            plan.report
        );
        assert_eq!(names(&root), before);
        assert_eq!(
            std::fs::read(root.join("prepare.json")).unwrap(),
            prepare_before
        );
        assert_eq!(
            std::fs::read(root.join("export.yaml")).unwrap(),
            export_before
        );
    }

    #[test]
    fn llamafactory_lora_prints_the_same_export_shape() {
        let root = tmp("lf-lora");
        write_prepare(
            &root,
            LLAMAFACTORY_LORA_ID,
            "train",
            Some("llama3"),
            Some(LF_TRAIN),
            false,
        );
        let adapter = adapter_dir(&root.join("outputs"));
        let export_dir = root.join("export");
        write_lf_cards_with(&root, LF_TRAIN, "qwen", &adapter, &export_dir, "", false);
        let plan = plan_merge_adapt(&root, &adapter).unwrap();
        assert_lf_export(&plan, &root, &export_dir, LLAMAFACTORY_LORA_ID);
        assert!(plan.report.contains("llamafactory-lora"), "{}", plan.report);
        assert!(
            plan.report.contains("leaves finetuning_type unset"),
            "{}",
            plan.report
        );
        assert!(!plan.report.contains("llamafactory-qlora"), "{}", plan.report);
        assert!(!export_dir.exists());
    }

    #[test]
    fn llamafactory_checkpoint_adapter_matches_export_yaml() {
        let root = tmp("lf-checkpoint");
        write_prepare(
            &root,
            LLAMAFACTORY_QLORA_ID,
            "train",
            Some("llama3"),
            Some(LF_TRAIN),
            false,
        );
        let checkpoint = adapter_dir(&root.join("outputs").join("checkpoint-10"));
        let export_dir = root.join("export");
        write_lf_cards(&root, &checkpoint, &export_dir, "");
        let plan = plan_merge_adapt(&root, &checkpoint).unwrap();
        assert_eq!(plan.merged_dir, export_dir);
        assert!(
            plan.merge_commands[0].contains("llamafactory-cli export "),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            plan.report.contains("checkpoint-10"),
            "{}",
            plan.report
        );
        assert!(!export_dir.exists());

        let outputs = adapter_dir(&root.join("outputs"));
        let err = plan_merge_adapt(&root, &outputs).unwrap_err();
        assert!(err.to_string().contains("refuse:merge"), "{err}");
        assert!(err.to_string().contains("adapter_name_or_path"), "{err}");
        assert!(err.to_string().contains("checkpoint"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");
        assert!(!export_dir.exists());
    }

    #[test]
    fn llamafactory_relative_export_dir_resolves_beside_the_prepare() {
        let root = tmp("lf-relative");
        write_prepare(
            &root,
            LLAMAFACTORY_LORA_ID,
            "train",
            Some("llama3"),
            Some(LF_TRAIN),
            false,
        );
        let adapter = adapter_dir(&root.join("outputs"));
        std::fs::write(
            root.join("export.yaml"),
            format!(
                "model_name_or_path: \"{LF_TRAIN}\"\n\
                 adapter_name_or_path: outputs\n\
                 template: qwen\n\
                 trust_remote_code: true\n\
                 export_dir: export\n\
                 export_size: 5\n\
                 export_device: cpu\n\
                 export_legacy_format: false\n"
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("recipe.yaml"),
            format!("model_name_or_path: \"{LF_TRAIN}\"\ntemplate: qwen\n"),
        )
        .unwrap();
        let plan = plan_merge_adapt(&root, &adapter).unwrap();
        assert_eq!(plan.merged_dir, root.join("export"));
        assert!(plan.report.contains("export_dir: export"), "{}", plan.report);
        assert!(!root.join("export").exists());
    }

    #[test]
    fn llamafactory_shell_quote_wraps_the_export_line() {
        let root = tmp("lf-quote");
        let spaced = root.join("my prepare");
        std::fs::create_dir_all(&spaced).unwrap();
        write_prepare(
            &spaced,
            LLAMAFACTORY_LORA_ID,
            "train",
            Some("llama3"),
            Some(LF_TRAIN),
            false,
        );
        let adapter = adapter_dir(&spaced.join("out dir"));
        let export_dir = spaced.join("export dir");
        write_lf_cards(&spaced, &adapter, &export_dir, "");
        let plan = plan_merge_adapt(&spaced, &adapter).unwrap();
        assert!(
            plan.merge_commands[0].contains(&format!("'{}'", spaced.join("export.yaml").display())),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            plan.gguf_convert_command
                .contains(&format!("'{}'", plan.merged_dir.display())),
            "{}",
            plan.gguf_convert_command
        );
        assert!(!export_dir.exists());
    }

    #[test]
    fn refuses_shapes_and_prepare_gates() {
        let root = tmp("refuse");
        let err = plan_merge_adapt(&root, &root.join("missing")).unwrap_err();
        assert!(err.to_string().contains("refuse:missing-prepare"), "{err}");

        write_prepare(&root, "unsloth-qlora", "train", Some("llama3"), None, false);
        let adapter = adapter_dir(&root.join("adapter"));
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(!err.to_string().contains("save_pretrained_merged"), "{err}");
        assert!(!err.to_string().contains("axolotl merge-lora"), "{err}");

        write_prepare(
            &root,
            "external-manifest",
            "enrich",
            Some("llama3"),
            None,
            false,
        );
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:driver"), "{err}");
        assert!(err.to_string().contains("external-manifest"), "{err}");

        write_prepare(&root, "mlx-lm-lora", "train", Some("llama3"), None, false);
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:host"), "{err}");
        assert!(err.to_string().contains("mlx-lm-lora"), "{err}");
        assert!(err.to_string().contains("apple-silicon"), "{err}");
        assert!(!err.to_string().contains("mlx_lm.fuse"), "{err}");

        write_prepare(
            &root,
            "ollama-modelfile",
            "enrich",
            Some("llama3"),
            None,
            false,
        );
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:driver"), "{err}");

        write_prepare(
            &root,
            AXOLOTL_LORA_ID,
            "enrich",
            Some("llama3"),
            None,
            false,
        );
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");

        write_prepare(&root, AXOLOTL_LORA_ID, "train", None, None, false);
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");

        write_prepare(&root, AXOLOTL_LORA_ID, "train", Some("qwen"), None, false);
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:prepare"), "{err}");

        write_prepare(&root, AXOLOTL_LORA_ID, "train", Some("llama3"), None, true);
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:prepared"), "{err}");

        write_prepare(&root, AXOLOTL_LORA_ID, "train", Some("llama3"), None, false);
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:merge"), "{err}");
        assert!(err.to_string().contains("axolotl.yml"), "{err}");
        assert!(!err.to_string().contains("--lora-model-dir"), "{err}");

        let empty = root.join("empty-adapter");
        std::fs::create_dir_all(&empty).unwrap();
        write_yaml(&root, &root.join("outputs"), "lora", false);
        let err = plan_merge_adapt(&root, &empty).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains("adapter_config.json"), "{err}");

        let merged = root.join("merged-weights");
        std::fs::create_dir_all(&merged).unwrap();
        std::fs::write(merged.join("config.json"), "{}\n").unwrap();
        std::fs::write(merged.join("model.safetensors"), b"not-a-real-tensor").unwrap();
        let err = plan_merge_adapt(&root, &merged).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains("merged"), "{err}");
        assert!(!err.to_string().contains("axolotl merge-lora "), "{err}");

        let gguf = root.join("weights.gguf");
        let mut bytes = b"GGUF".to_vec();
        bytes.extend_from_slice(&[0u8; 12]);
        std::fs::write(&gguf, bytes).unwrap();
        let err = plan_merge_adapt(&root, &gguf).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains("GGUF"), "{err}");

        let linked = root.join("linked-adapter");
        std::os::unix::fs::symlink(&adapter, &linked).unwrap();
        let err = plan_merge_adapt(&root, &linked).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains("symlink"), "{err}");

        let marked = root.join("marked-adapter");
        std::fs::create_dir_all(&marked).unwrap();
        let outside = root.join("outside-config.json");
        std::fs::write(&outside, "{}\n").unwrap();
        std::os::unix::fs::symlink(&outside, marked.join("adapter_config.json")).unwrap();
        let err = plan_merge_adapt(&root, &marked).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains("symlink"), "{err}");

        let yaml_link = root.join("axolotl.yml");
        std::fs::remove_file(&yaml_link).unwrap();
        let real_yaml = root.join("real.yml");
        std::fs::write(&real_yaml, "output_dir: outputs\n").unwrap();
        std::os::unix::fs::symlink(&real_yaml, &yaml_link).unwrap();
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:merge"), "{err}");
        assert!(err.to_string().contains("symlink"), "{err}");

        let sacred = root.join("cyera-adapter");
        adapter_dir(&sacred);
        write_prepare(&root, AXOLOTL_LORA_ID, "train", Some("llama3"), None, false);
        std::fs::remove_file(&yaml_link).unwrap();
        write_yaml(&root, &root.join("outputs"), "lora", false);
        let err = plan_merge_adapt(&root, &sacred).unwrap_err();
        assert!(err.to_string().contains("refuse:sacred"), "{err}");

        let sku = root.join("model-5090");
        adapter_dir(&sku);
        let err = plan_merge_adapt(&root, &sku).unwrap_err();
        assert!(err.to_string().contains("refuse:sku-banned"), "{err}");

        write_prepare(
            &root,
            LLAMAFACTORY_QLORA_ID,
            "train",
            Some("llama3"),
            None,
            false,
        );
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(!err.to_string().contains("merge_and_unload"), "{err}");
        assert!(
            !err.to_string().contains("llamafactory-cli export "),
            "{err}"
        );
    }

    #[test]
    fn llamafactory_merge_refuses_closed() {
        let root = tmp("lf-refuse");
        let adapter = adapter_dir(&root.join("outputs"));
        let export_dir = root.join("export");

        write_prepare(
            &root,
            LLAMAFACTORY_QLORA_ID,
            "train",
            Some("llama3"),
            Some(LF_TRAIN),
            false,
        );
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:merge"), "{err}");
        assert!(err.to_string().contains("export.yaml"), "{err}");
        assert!(err.to_string().contains("is missing"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");
        assert!(!err.to_string().contains("merge_and_unload"), "{err}");
        assert!(!export_dir.exists());
        assert!(!root.join("merged").exists());

        write_lf_cards(&root, &adapter, &export_dir, "");
        std::fs::remove_file(root.join("recipe.yaml")).unwrap();
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("recipe.yaml"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");

        write_lf_cards(&root, &adapter, &export_dir, "quantization_bit: 4\n");
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:export"), "{err}");
        assert!(err.to_string().contains("quantization_bit"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");
        assert!(root.join("export.yaml").is_file());

        write_lf_cards(
            &root,
            &adapter,
            &export_dir,
            "export_quantization_bit: 4\nexport_quantization_dataset: data/c4_demo.json\n",
        );
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:export"), "{err}");
        assert!(err.to_string().contains("export_quantization_bit"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");

        write_lf_cards(&root, &adapter, &export_dir, "quantization_method: bnb\n");
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:export"), "{err}");
        assert!(err.to_string().contains("quantization_method"), "{err}");

        write_lf_cards_with(
            &root,
            "other/repo",
            "qwen",
            &adapter,
            &export_dir,
            "",
            true,
        );
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("other/repo"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");

        write_lf_cards(&root, &adapter, &export_dir, "");
        std::fs::write(
            root.join("recipe.yaml"),
            format!("model_name_or_path: \"other/repo\"\ntemplate: qwen\n"),
        )
        .unwrap();
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("recipe.yaml"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");

        write_lf_cards(&root, &adapter, &export_dir, "");
        std::fs::write(
            root.join("recipe.yaml"),
            format!("model_name_or_path: \"{LF_TRAIN}\"\ntemplate: llama3\n"),
        )
        .unwrap();
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:merge"), "{err}");
        assert!(err.to_string().contains("template"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");

        write_lf_cards(&root, &adapter, &adapter, "");
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:merge"), "{err}");
        assert!(err.to_string().contains("adapter directory"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");

        write_lf_cards(&root, &adapter, &export_dir, "");
        let kept = std::fs::read_to_string(root.join("export.yaml"))
            .unwrap()
            .replace("export_device: cpu", "export_device: cuda");
        std::fs::write(root.join("export.yaml"), &kept).unwrap();
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:merge"), "{err}");
        assert!(err.to_string().contains("export_device"), "{err}");
        assert!(err.to_string().contains("cpu"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");
        assert_eq!(std::fs::read_to_string(root.join("export.yaml")).unwrap(), kept);

        let full = kept
            .replace("export_device: cuda", "export_device: cpu")
            .replace("finetuning_type: lora", "finetuning_type: full");
        assert_ne!(full, kept);
        std::fs::write(root.join("export.yaml"), full).unwrap();
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:merge"), "{err}");
        assert!(err.to_string().contains("finetuning_type"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");

        std::fs::write(root.join("export.yaml"), "export_dir: export\n").unwrap();
        write_prepare(
            &root,
            LLAMAFACTORY_LORA_ID,
            "train",
            Some("llama3"),
            Some(LF_TRAIN),
            false,
        );
        std::fs::write(
            root.join("recipe.yaml"),
            format!("model_name_or_path: \"{LF_TRAIN}\"\ntemplate: qwen\n"),
        )
        .unwrap();
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:merge"), "{err}");
        assert!(err.to_string().contains("model_name_or_path"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");

        write_prepare(
            &root,
            LLAMAFACTORY_QLORA_ID,
            "enrich",
            Some("llama3"),
            Some(LF_TRAIN),
            false,
        );
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");

        write_prepare(
            &root,
            LLAMAFACTORY_QLORA_ID,
            "train",
            Some("llama3"),
            Some(LF_TRAIN),
            true,
        );
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:prepared"), "{err}");
        assert!(!err.to_string().contains("llamafactory-cli export "), "{err}");

        write_prepare(
            &root,
            LLAMAFACTORY_LORA_ID,
            "train",
            Some("llama3"),
            Some(LF_TRAIN),
            false,
        );
        let sacred = root.join("cyera-adapter");
        adapter_dir(&sacred);
        write_lf_cards(&root, &sacred, &export_dir, "");
        let err = plan_merge_adapt(&root, &sacred).unwrap_err();
        assert!(err.to_string().contains("refuse:sacred"), "{err}");

        let sku = root.join("model-5090");
        adapter_dir(&sku);
        let err = plan_merge_adapt(&root, &sku).unwrap_err();
        assert!(err.to_string().contains("refuse:sku-banned"), "{err}");
        assert!(!export_dir.exists());
        assert!(!root.join("merged").exists());
    }

    const UNSLOTH_TRAIN: &str = "Qwen/Qwen2.5-0.5B-Instruct";

    fn write_unsloth_md(dir: &Path, train: &str, seat: &str) {
        std::fs::write(
            dir.join("UNSLOTH.md"),
            format!("train_base_model: \"{train}\"\nseat_tag: \"{seat}\"\n"),
        )
        .unwrap();
    }

    fn unsloth_weights(dir: &Path, weight_name: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("adapter_config.json"), "{}\n").unwrap();
        std::fs::write(dir.join(weight_name), b"w").unwrap();
        dir.to_path_buf()
    }

    #[test]
    fn unsloth_adapter_prints_merged_16bit_and_writes_nothing() {
        let root = tmp("unsloth-print");
        write_prepare(
            &root,
            UNSLOTH_QLORA_ID,
            "train",
            Some("llama3"),
            Some(UNSLOTH_TRAIN),
            false,
        );
        write_unsloth_md(&root, UNSLOTH_TRAIN, "llama3");
        let adapter = unsloth_weights(&root.join("lora"), "adapter_model.safetensors");
        let before = names(&root);
        let prepare_before = std::fs::read(root.join("prepare.json")).unwrap();
        let plan = plan_merge_adapt(&root, &adapter).unwrap();
        let merged = root.join("merged");
        assert_eq!(plan.merged_dir, merged);
        assert_eq!(plan.driver, UNSLOTH_QLORA_ID);
        assert_eq!(plan.merge_commands.len(), 1);
        assert_eq!(
            plan.merge_commands[0],
            printed_unsloth_merged_16bit(&merged)
        );
        assert!(
            plan.merge_commands[0].contains("save_method = \"merged_16bit\""),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            !plan.merge_commands[0].contains("merge_and_unload"),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            !plan.merge_commands[0].contains("merged_4bit"),
            "{}",
            plan.merge_commands[0]
        );
        assert_eq!(plan.convert_command, printed_convert_line(&merged));
        assert!(plan.convert_command.contains("--outtype auto"), "{}", plan.convert_command);
        assert_eq!(plan.gguf_convert_command, gguf_convert_cli(&root, &merged));
        assert_eq!(plan.local_seat_command, local_seat_cli(&root, &merged));
        assert!(plan.report.contains("save_pretrained_merged"), "{}", plan.report);
        assert!(plan.report.contains("--outtype f16"), "{}", plan.report);
        assert!(plan.report.contains("--outtype bf16"), "{}", plan.report);
        assert!(plan.report.contains("--outtype q8_0"), "{}", plan.report);
        assert!(plan.report.contains("model-F16.gguf"), "{}", plan.report);
        assert!(plan.report.contains("model-BF16.gguf"), "{}", plan.report);
        assert!(plan.report.contains("model-Q8_0.gguf"), "{}", plan.report);
        assert!(plan.report.contains("--split-max-size 50G"), "{}", plan.report);
        assert!(
            plan.report.contains("does not publish --outtype auto")
                || plan.report.contains("Unsloth's page does not publish --outtype auto"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("save_pretrained_gguf"), "{}", plan.report);
        assert!(plan.report.contains("quantization_method = \"q4_k_m\""), "{}", plan.report);
        assert!(plan.report.contains("quantization_method = \"q8_0\""), "{}", plan.report);
        assert!(plan.report.contains("quantization_method = \"f16\""), "{}", plan.report);
        assert!(plan.report.contains("\"directory\""), "{}", plan.report);
        assert!(plan.report.contains("refuse:adapter"), "{}", plan.report);
        assert!(plan.report.contains("READY_FOR_LIVE_TEST: no"), "{}", plan.report);
        assert!(!plan.report.contains("READY_FOR_LIVE_TEST: yes"), "{}", plan.report);
        assert!(!plan.report.contains("merge_and_unload()"), "{}", plan.report);
        assert!(plan.report.contains("merge-adapt did not merge"), "{}", plan.report);
        assert!(!merged.exists());
        assert_eq!(names(&root), before);
        assert_eq!(std::fs::read(root.join("prepare.json")).unwrap(), prepare_before);

        let bin_adapter = unsloth_weights(&root.join("bin-lora"), "adapter_model.bin");
        let bin_plan = plan_merge_adapt(&root, &bin_adapter).unwrap();
        assert!(
            bin_plan.report.contains("adapter_model.bin"),
            "{}",
            bin_plan.report
        );
        assert!(!root.join("merged").exists());
    }

    #[test]
    fn unsloth_merge_refuses_closed() {
        let root = tmp("unsloth-refuse");
        write_prepare(
            &root,
            UNSLOTH_QLORA_ID,
            "enrich",
            Some("llama3"),
            Some(UNSLOTH_TRAIN),
            false,
        );
        write_unsloth_md(&root, UNSLOTH_TRAIN, "llama3");
        let adapter = unsloth_weights(&root.join("lora"), "adapter_model.safetensors");
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");
        assert!(!err.to_string().contains("save_pretrained_merged"), "{err}");

        write_prepare(
            &root,
            UNSLOTH_QLORA_ID,
            "train",
            Some("llama3"),
            Some(UNSLOTH_TRAIN),
            false,
        );
        std::fs::remove_file(root.join("UNSLOTH.md")).unwrap();
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("UNSLOTH.md is missing"), "{err}");
        assert!(!err.to_string().contains("save_pretrained_merged"), "{err}");

        let real = root.join("real-unsloth.md");
        std::fs::write(
            &real,
            format!("train_base_model: \"{UNSLOTH_TRAIN}\"\nseat_tag: \"llama3\"\n"),
        )
        .unwrap();
        std::os::unix::fs::symlink(&real, root.join("UNSLOTH.md")).unwrap();
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("symlink"), "{err}");
        assert!(!err.to_string().contains("save_pretrained_merged"), "{err}");

        std::fs::remove_file(root.join("UNSLOTH.md")).unwrap();
        write_unsloth_md(&root, "other/repo", "llama3");
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(!err.to_string().contains("save_pretrained_merged"), "{err}");

        write_unsloth_md(&root, UNSLOTH_TRAIN, "other");
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("seat_tag"), "{err}");

        write_unsloth_md(&root, UNSLOTH_TRAIN, "llama3");
        let bare = adapter_dir(&root.join("bare"));
        let err = plan_merge_adapt(&root, &bare).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains("adapter_model.safetensors"), "{err}");
        assert!(!err.to_string().contains("save_pretrained_merged"), "{err}");

        let mlx_weights = unsloth_weights(&root.join("mlxish"), "adapters.safetensors");
        let err = plan_merge_adapt(&root, &mlx_weights).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(!err.to_string().contains("save_pretrained_merged"), "{err}");

        let merged = root.join("merged-weights");
        std::fs::create_dir_all(&merged).unwrap();
        std::fs::write(merged.join("config.json"), "{}\n").unwrap();
        std::fs::write(merged.join("model.safetensors"), b"not-a-real-tensor").unwrap();
        let err = plan_merge_adapt(&root, &merged).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(!root.join("merged").exists() || merged.exists());

        let gguf = root.join("weights.gguf");
        let mut bytes = b"GGUF".to_vec();
        bytes.extend_from_slice(&[0u8; 12]);
        std::fs::write(&gguf, bytes).unwrap();
        let err = plan_merge_adapt(&root, &gguf).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains("GGUF"), "{err}");

        let linked = root.join("linked-lora");
        std::os::unix::fs::symlink(&adapter, &linked).unwrap();
        let err = plan_merge_adapt(&root, &linked).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains("symlink"), "{err}");

        let marked = root.join("marked-lora");
        std::fs::create_dir_all(&marked).unwrap();
        let outside = root.join("outside-config.json");
        std::fs::write(&outside, "{}\n").unwrap();
        std::os::unix::fs::symlink(&outside, marked.join("adapter_config.json")).unwrap();
        let err = plan_merge_adapt(&root, &marked).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains("symlink"), "{err}");

        let sacred = root.join("cyera-adapter");
        unsloth_weights(&sacred, "adapter_model.safetensors");
        let err = plan_merge_adapt(&root, &sacred).unwrap_err();
        assert!(err.to_string().contains("refuse:sacred"), "{err}");

        let sku = root.join("model-5090");
        unsloth_weights(&sku, "adapter_model.safetensors");
        let err = plan_merge_adapt(&root, &sku).unwrap_err();
        assert!(err.to_string().contains("refuse:sku-banned"), "{err}");

        write_prepare(
            &root,
            UNSLOTH_QLORA_ID,
            "train",
            Some("llama3"),
            Some(UNSLOTH_TRAIN),
            true,
        );
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:prepared"), "{err}");
        assert!(!root.join("merged").exists());
    }

    #[test]
    fn refuses_a_card_that_disagrees_with_the_yaml() {
        let root = tmp("mismatch");
        write_prepare(&root, AXOLOTL_LORA_ID, "train", Some("llama3"), None, false);
        write_yaml(&root, &root.join("outputs"), "qlora", true);
        let adapter = adapter_dir(&root.join("outputs"));
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:merge"), "{err}");
        assert!(!err.to_string().contains("--dequant"), "{err}");
    }

    #[test]
    fn refuses_a_symlinked_export_yaml() {
        let root = tmp("export-link");
        write_prepare(
            &root,
            LLAMAFACTORY_LORA_ID,
            "train",
            Some("llama3"),
            Some("Qwen/Qwen2.5-0.5B-Instruct"),
            false,
        );
        let real = root.join("real-export.yaml");
        std::fs::write(&real, "export_dir: export\n").unwrap();
        std::os::unix::fs::symlink(&real, root.join("export.yaml")).unwrap();
        let adapter = adapter_dir(&root.join("outputs"));
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:merge"), "{err}");
        assert!(err.to_string().contains("symlink"), "{err}");
        assert!(!err.to_string().contains("merge_and_unload"), "{err}");
    }

    const MLX_TRAIN: &str = "Qwen/Qwen2.5-0.5B-Instruct";

    fn write_mlx(dir: &Path, job: &str, host: &str, train: Option<&str>, promoted: bool) {
        let train_line = match train {
            Some(train) => format!("  \"train_base_model\": \"{train}\",\n"),
            None => String::new(),
        };
        let body = format!(
            "{{\n\
               \"schema\": \"{PREPARE_SCHEMA}\",\n\
               \"driver\": \"mlx-lm-lora\",\n\
               \"job\": \"{job}\",\n\
               \"pack_id\": \"overnight-traces\",\n\
               \"base_model\": \"llama3\",\n\
               \"seat_tag\": \"llama3\",\n\
               {train_line}\
               \"purpose\": \"fixture\",\n\
               \"host_class_affinity\": \"{host}\",\n\
               \"source_paths\": [],\n\
               \"source_drivers\": [],\n\
               \"artifacts\": [],\n\
               \"promoted\": {promoted},\n\
               \"auto_apply\": false,\n\
               \"estate_rewritten\": false,\n\
               \"note\": \"test\"\n\
             }}\n"
        );
        std::fs::write(dir.join("prepare.json"), body).unwrap();
    }

    fn write_mlx_md(dir: &Path, train: &str, host: &str) {
        std::fs::write(
            dir.join("MLX.md"),
            format!(
                "train_base_model: \"{train}\"\nseat_tag: \"llama3\"\nhost_class_affinity: {host}\n"
            ),
        )
        .unwrap();
    }

    fn mlx_adapter(dir: &Path) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("adapter_config.json"), "{}\n").unwrap();
        std::fs::write(dir.join(MLX_ADAPTER_WEIGHTS), b"not-a-real-tensor").unwrap();
        dir.to_path_buf()
    }

    fn assert_no_hf_convert(text: &str) {
        assert!(!text.contains("python3 convert_hf_to_gguf.py"), "{text}");
        assert!(!text.contains("estate enrich gguf-convert"), "{text}");
        assert!(!text.contains("--outtype"), "{text}");
    }

    #[test]
    fn mlx_adapter_prints_the_documented_fuse_and_writes_nothing() {
        let root = tmp("mlx-fuse");
        write_mlx(&root, "train", "apple-silicon", Some(MLX_TRAIN), false);
        write_mlx_md(&root, MLX_TRAIN, "apple-silicon");
        let adapter = mlx_adapter(&root.join("adapters"));
        let before = names(&root);
        let prepare_before = std::fs::read(root.join("prepare.json")).unwrap();
        let plan = plan_merge_adapt(&root, &adapter).unwrap();
        let canonical = std::fs::canonicalize(&adapter).unwrap();
        let fused = root.join(MLX_FUSED_DIR_NAME);
        let gguf = fused.join(MLX_GGUF_FILE_NAME);
        let fuse = printed_mlx_fuse_line(MLX_TRAIN, &canonical, &fused, false);
        let export = printed_mlx_fuse_line(MLX_TRAIN, &canonical, &fused, true);
        assert_eq!(plan.driver, "mlx-lm-lora");
        assert_eq!(plan.merged_dir, fused);
        assert_eq!(plan.merge_commands, vec![fuse.clone(), export.clone()]);
        assert_eq!(plan.convert_command, export);
        assert!(plan.gguf_convert_command.is_empty());
        assert_eq!(plan.local_seat_command, local_seat_cli(&root, &gguf));
        assert!(plan.report.contains("shape=mlx-adapter"), "{}", plan.report);
        assert!(plan.report.contains(&fuse), "{}", plan.report);
        assert!(plan.report.contains(&export), "{}", plan.report);
        assert!(
            plan.report.contains("merge-adapt did not fuse"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("READY_FOR_LIVE_TEST: no"),
            "{}",
            plan.report
        );
        assert!(
            !plan.report.contains("READY_FOR_LIVE_TEST: yes"),
            "{}",
            plan.report
        );
        assert_no_hf_convert(&plan.report);
        assert!(
            plan.merge_commands
                .iter()
                .all(|line| !line.contains("--dequantize") && !line.contains("--gguf-path")),
            "{:?}",
            plan.merge_commands
        );
        assert!(!fused.exists());
        assert!(!gguf.exists());
        assert_eq!(names(&root), before);
        assert_eq!(
            std::fs::read(root.join("prepare.json")).unwrap(),
            prepare_before
        );
    }

    #[test]
    fn mlx_fuse_refuses_the_wrong_shape_host_and_job() {
        let root = tmp("mlx-refuse");
        let adapter = mlx_adapter(&root.join("adapters"));

        write_mlx(&root, "enrich", "apple-silicon", Some(MLX_TRAIN), false);
        write_mlx_md(&root, MLX_TRAIN, "apple-silicon");
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");
        assert!(!err.to_string().contains("mlx_lm.fuse"), "{err}");

        write_mlx(&root, "train", "consumer-nvidia", Some(MLX_TRAIN), false);
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:host"), "{err}");
        assert!(!err.to_string().contains("--save-path"), "{err}");

        write_mlx(&root, "train", "apple-silicon", Some(MLX_TRAIN), false);
        std::fs::remove_file(root.join("MLX.md")).unwrap();
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("MLX.md"), "{err}");
        assert!(!err.to_string().contains("--save-path"), "{err}");

        write_mlx_md(&root, "other/repo", "apple-silicon");
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(!err.to_string().contains("--export-gguf"), "{err}");

        write_mlx_md(&root, MLX_TRAIN, "any");
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:host"), "{err}");

        let real_md = root.join("real-mlx.md");
        std::fs::rename(root.join("MLX.md"), &real_md).unwrap();
        std::os::unix::fs::symlink(&real_md, root.join("MLX.md")).unwrap();
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:host"), "{err}");
        assert!(err.to_string().contains("symlink"), "{err}");
        assert!(!err.to_string().contains("--save-path"), "{err}");
        std::fs::remove_file(root.join("MLX.md")).unwrap();
        write_mlx_md(&root, MLX_TRAIN, "apple-silicon");

        write_mlx(&root, "train", "apple-silicon", Some(MLX_TRAIN), true);
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:prepared"), "{err}");
        write_mlx(&root, "train", "apple-silicon", Some(MLX_TRAIN), false);

        let config_only = root.join("config-only");
        std::fs::create_dir_all(&config_only).unwrap();
        std::fs::write(config_only.join("adapter_config.json"), "{}\n").unwrap();
        std::fs::write(
            config_only.join("0000001_adapters.safetensors"),
            b"checkpoint",
        )
        .unwrap();
        let err = plan_merge_adapt(&root, &config_only).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains(MLX_ADAPTER_WEIGHTS), "{err}");
        assert!(err.to_string().contains("_adapters.safetensors"), "{err}");
        assert!(!err.to_string().contains("--save-path"), "{err}");

        let peft = root.join("peft");
        std::fs::create_dir_all(&peft).unwrap();
        std::fs::write(peft.join("adapter_config.json"), "{}\n").unwrap();
        std::fs::write(peft.join("adapter_model.safetensors"), b"peft").unwrap();
        std::fs::write(peft.join(MLX_ADAPTER_WEIGHTS), b"mlx").unwrap();
        let err = plan_merge_adapt(&root, &peft).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(
            err.to_string().contains("adapter_model.safetensors"),
            "{err}"
        );
        assert!(!err.to_string().contains("--save-path"), "{err}");

        let linked_weights = root.join("linked-weights");
        std::fs::create_dir_all(&linked_weights).unwrap();
        std::fs::write(linked_weights.join("adapter_config.json"), "{}\n").unwrap();
        let outside = root.join("outside.safetensors");
        std::fs::write(&outside, b"escaped").unwrap();
        std::os::unix::fs::symlink(&outside, linked_weights.join(MLX_ADAPTER_WEIGHTS)).unwrap();
        let err = plan_merge_adapt(&root, &linked_weights).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains("symlink"), "{err}");
        assert!(!err.to_string().contains("--save-path"), "{err}");

        let linked_dir = root.join("linked-dir");
        std::os::unix::fs::symlink(&adapter, &linked_dir).unwrap();
        let err = plan_merge_adapt(&root, &linked_dir).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains("symlink"), "{err}");

        let gguf = root.join("weights.gguf");
        let mut bytes = b"GGUF".to_vec();
        bytes.extend_from_slice(&[0u8; 12]);
        std::fs::write(&gguf, bytes).unwrap();
        let err = plan_merge_adapt(&root, &gguf).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(err.to_string().contains("GGUF"), "{err}");
        assert!(!err.to_string().contains("--export-gguf"), "{err}");

        let merged = root.join("hf-merged");
        std::fs::create_dir_all(&merged).unwrap();
        std::fs::write(merged.join("config.json"), "{}\n").unwrap();
        std::fs::write(merged.join("model.safetensors"), b"w").unwrap();
        let err = plan_merge_adapt(&root, &merged).unwrap_err();
        assert!(err.to_string().contains("refuse:adapter"), "{err}");
        assert!(!err.to_string().contains("--save-path"), "{err}");

        let sacred = root.join("cyera-mlx");
        mlx_adapter(&sacred);
        let err = plan_merge_adapt(&root, &sacred).unwrap_err();
        assert!(err.to_string().contains("refuse:sacred"), "{err}");

        let sku = root.join("model-5090");
        mlx_adapter(&sku);
        let err = plan_merge_adapt(&root, &sku).unwrap_err();
        assert!(err.to_string().contains("refuse:sku-banned"), "{err}");

        assert!(!root.join(MLX_FUSED_DIR_NAME).exists());
    }
}
