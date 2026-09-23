//! Print-only llama.cpp GGUF convert card for a merged Hugging Face export.
//!
//! llama.cpp already converts that directory with `convert_hf_to_gguf.py`.
//! `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, and
//! `axolotl-qlora` prepares point at that directory. Axolotl does not write
//! the GGUF. This module prints the command. It does not vendor the script,
//! spawn it, or write a GGUF.
//!
//! `mlx-lm-lora` does not use this script. mlx-lm documents
//! `mlx_lm.fuse --export-gguf`. This command refuses that prepare and names
//! that flag. It does not invent a converter.
//!
//! `unsloth-qlora` uses this script for the merged 16-bit directory.
//! The saving-to-gguf page also publishes three manual
//! `python llama.cpp/convert_hf_to_gguf.py` lines (`f16`, `bf16`, `q8_0`).
//! This command prints those lines and the factory card (`--outtype auto`).
//! It does not run either form.
//!
//! `--outtype auto` is the script default in ggml-org/llama.cpp
//! `convert_hf_to_gguf.py`: choices are f32, f16, bf16, q8_0, tq1_0, tq2_0,
//! and auto, and the default is auto (the highest-fidelity 16-bit float
//! type). This card passes that default so the operator does not guess the
//! flag. It does not pass a quantization type.
//!
//! When `tokenizer_config.json` is already in the merged directory, a JSON
//! list or JSON null under `extra_special_tokens` is `refuse:tokenizer`.
//! transformers calls `.keys()` on that value. A Qwen-family export missing
//! `vocab.json` or `merges.txt` is the same refuse. Qwen-family is
//! `config.json` `model_type` or `architectures`, or `tokenizer_class`,
//! naming Qwen.
//! That refuse names the operator restore: copy tokenizer files from the
//! HF cache snapshot already on disk, or the equivalent base checkout, into
//! the export directory, then re-run `estate enrich gguf-convert`.
//! HF hub snapshots are often symlinks into the HF cache. The sentence tells
//! the operator to copy with dereference (`cp -aL` or `cp --dereference`)
//! so the export directory holds real files. A symlinked
//! `tokenizer_config.json` stays `refuse:tokenizer`.
//! This module does not follow that symlink, does not download tokenizer
//! files, does not copy them, does not write `tokenizer_config.json.bak`,
//! and does not run the script.

use crate::error::ModelError;
use crate::local_seat::{classify_weights, shell_quote, WeightsShape};
use crate::train_enrich::{
    is_axolotl_driver, is_post_merge_print_driver, load_prepare_doc, local_enrich_tag,
    refuse_post_merge_driver, refuse_recipe_train_record, refuse_sacred_and_sku, EnrichJobKind,
    MLX_LM_LORA_ID, UNSLOTH_GGUF_DOC, UNSLOTH_QLORA_ID,
};
use feed_collector::{refuse_raw_secrets, FeedError};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// `tokenizer_config.json` above this size is `refuse:tokenizer`.
/// The check does not stream the file and does not print the convert line.
const TOKENIZER_CHECK_MAX_BYTES: u64 = 8 * 1024 * 1024;

/// Printed plan. `outfile` is the sibling path the convert line names.
/// This command does not create that file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GgufConvertPlan {
    pub seat_tag: String,
    pub local_tag: String,
    pub pack_id: String,
    pub driver: String,
    pub convert_command: String,
    pub outfile: PathBuf,
    pub local_seat_command: String,
    pub report: String,
}

/// Sibling `{parent}/{dirname}.gguf`. A `.gguf` inside the merged directory
/// makes that directory match two shapes.
pub(crate) fn sibling_gguf_outfile(export_dir: &Path) -> PathBuf {
    let stem = export_dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("model");
    let file_name = format!("{stem}.gguf");
    match export_dir.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(file_name),
        _ => PathBuf::from(file_name),
    }
}

/// Exact `convert_hf_to_gguf.py` line. `--outtype auto` is the script default.
pub(crate) fn printed_convert_line(export_dir: &Path) -> String {
    let outfile = sibling_gguf_outfile(export_dir);
    format!(
        "python3 convert_hf_to_gguf.py {} --outfile {} --outtype auto",
        shell_quote(&export_dir.display().to_string()),
        shell_quote(&outfile.display().to_string())
    )
}

pub(crate) fn gguf_convert_cli(prepared: &Path, weights: &Path) -> String {
    format!(
        "estate enrich gguf-convert --prepared {} --weights {}",
        shell_quote(&prepared.display().to_string()),
        shell_quote(&weights.display().to_string())
    )
}

pub(crate) fn local_seat_cli(prepared: &Path, weights: &Path) -> String {
    format!(
        "estate enrich local-seat --prepared {} --weights {}",
        shell_quote(&prepared.display().to_string()),
        shell_quote(&weights.display().to_string())
    )
}

/// Operator restore after `refuse:tokenizer`. Names `train_base` when the
/// prepare recorded one. HF hub snapshots are often symlinks. The sentence
/// tells the operator to copy with dereference so the export directory holds
/// real files. Does not build a cache path, does not fetch, does not copy
/// files, and does not follow a symlinked `tokenizer_config.json`.
pub(crate) fn tokenizer_restore_sentence(train_base: &str) -> String {
    let named = match train_base.trim() {
        "" => "the train base on this prepare".to_string(),
        base => format!("train base {base}"),
    };
    format!(
        "Copy the tokenizer files from {named} already on disk into the export directory. The source is the HF cache snapshot for that repo, or the equivalent base checkout (a local HF weights directory). HF hub snapshots are often symlinks into the HF cache. Copy with dereference (cp -aL or cp --dereference, or the equivalent) so the files in the export directory are real files, not symlinks. A plain cp -a leaves tokenizer_config.json as a symlink. enrich does not follow a symlinked tokenizer_config.json. Qwen2.5 train bases normally include vocab.json and merges.txt, plus tokenizer_config.json and tokenizer.json. Keep the export tokenizer_config.json as tokenizer_config.json.bak before you replace it. Then re-run estate enrich gguf-convert on that export directory. This factory does not download weights, does not copy those files, and does not run convert_hf_to_gguf.py."
    )
}

/// Standing note for LLaMA-Factory `NEXT.md`, `PREPARE.md`, and `merge-adapt`.
/// The export directory may not exist yet. This text does not scan it.
pub(crate) fn export_tokenizer_guidance(train_base: &str) -> String {
    format!(
        "After llamafactory-cli export writes the merged directory, and before convert_hf_to_gguf.py, check tokenizer_config.json in that directory. LLaMA-Factory export can save extra_special_tokens as a JSON list. transformers then raises AttributeError ('list' object has no attribute 'keys') while convert_hf_to_gguf.py loads the tokenizer. JSON null under extra_special_tokens is the same refuse:tokenizer case: transformers calls .keys() on that non-object value. The same export can omit vocab.json and merges.txt. estate enrich gguf-convert returns refuse:tokenizer for that export before the restore, for that list, for JSON null, and for a Qwen-family export that is missing vocab.json or merges.txt. Qwen-family there means config.json model_type or architectures, or tokenizer_class, names Qwen. An object extra_special_tokens with those two files present still prints the convert line. {restore}",
        restore = tokenizer_restore_sentence(train_base)
    )
}

enum LoadedTokenizerConfig {
    Absent,
    Object(serde_json::Map<String, Value>),
}

fn inspect_export_tokenizer(dir: &Path, train_base: Option<&str>) -> Result<String, ModelError> {
    let path = dir.join("tokenizer_config.json");
    let loaded = load_tokenizer_config(&path, train_base)?;
    let class = match &loaded {
        LoadedTokenizerConfig::Object(map) => {
            json_string(map, "tokenizer_class").map(str::to_string)
        }
        LoadedTokenizerConfig::Absent => None,
    };
    let config = optional_json_object(&dir.join("config.json"));
    let qwen = directory_is_qwen(config.as_ref(), class.as_deref());
    let mut problems = Vec::new();
    if let LoadedTokenizerConfig::Object(map) = &loaded {
        if let Some(problem) = extra_special_tokens_problem(map) {
            problems.push(problem);
        }
    }
    if qwen {
        for name in ["vocab.json", "merges.txt"] {
            if let Some(problem) = bpe_file_problem(dir, name)? {
                problems.push(problem);
            }
        }
    }
    if !problems.is_empty() {
        return Err(refuse_export_tokenizer(dir, train_base, &problems));
    }
    Ok(tokenizer_pass_note(&pass_facts(&loaded, qwen), train_base))
}

fn tokenizer_pass_note(facts: &str, train_base: Option<&str>) -> String {
    format!(
        "Tokenizer check passed. {facts}. A JSON list or JSON null under extra_special_tokens is refuse:tokenizer. transformers raises AttributeError ('list' object has no attribute 'keys') inside convert_hf_to_gguf.py when the value is a list. JSON null is the same refuse: transformers calls .keys() on that non-object value. A Qwen-family export missing vocab.json or merges.txt is the same refuse. Qwen-family here is config.json model_type or architectures starting with Qwen, or tokenizer_class containing Qwen. A merged export can write that list and omit those files. {restore}",
        restore = tokenizer_restore_sentence(train_base.unwrap_or(""))
    )
}

fn refuse_export_tokenizer(
    dir: &Path,
    train_base: Option<&str>,
    problems: &[String],
) -> ModelError {
    ModelError::Other(format!(
        "refuse:tokenizer: {} {}. {}",
        dir.display(),
        problems.join("; "),
        tokenizer_restore_sentence(train_base.unwrap_or(""))
    ))
}

fn pass_facts(loaded: &LoadedTokenizerConfig, qwen: bool) -> String {
    let config = match loaded {
        LoadedTokenizerConfig::Absent => "this directory has no tokenizer_config.json".to_string(),
        LoadedTokenizerConfig::Object(map) => match map.get("extra_special_tokens") {
            Some(Value::Object(_)) => "extra_special_tokens is an object".to_string(),
            _ => "extra_special_tokens is not a list".to_string(),
        },
    };
    if qwen {
        format!("{config}. vocab.json and merges.txt are in this directory")
    } else {
        config
    }
}

fn extra_special_tokens_problem(map: &serde_json::Map<String, Value>) -> Option<String> {
    let value = map.get("extra_special_tokens")?;
    match value {
        Value::Array(_) => Some(
            "tokenizer_config.json extra_special_tokens is a JSON list. transformers calls .keys() on that value and raises AttributeError ('list' object has no attribute 'keys')".into(),
        ),
        Value::Object(_) => None,
        Value::Null => Some(
            "tokenizer_config.json extra_special_tokens is JSON null. transformers calls .keys() on that value".into(),
        ),
        Value::String(_) => Some(
            "tokenizer_config.json extra_special_tokens is a JSON string. transformers calls .keys() on that value".into(),
        ),
        Value::Bool(_) => Some(
            "tokenizer_config.json extra_special_tokens is a JSON bool. transformers calls .keys() on that value".into(),
        ),
        Value::Number(_) => Some(
            "tokenizer_config.json extra_special_tokens is a JSON number. transformers calls .keys() on that value".into(),
        ),
    }
}

fn directory_is_qwen(
    config: Option<&serde_json::Map<String, Value>>,
    tokenizer_class: Option<&str>,
) -> bool {
    if let Some(map) = config {
        if let Some(qwen) = config_is_qwen(map) {
            return qwen;
        }
    }
    tokenizer_class.is_some_and(|class| {
        let name = class.trim();
        !name.is_empty() && name.to_ascii_lowercase().contains("qwen")
    })
}

/// `Some` when `model_type` or a non-empty `architectures` list decides.
/// `None` when those keys are absent, so `tokenizer_class` can still decide.
fn config_is_qwen(map: &serde_json::Map<String, Value>) -> Option<bool> {
    if let Some(model_type) = json_string(map, "model_type") {
        let name = model_type.trim();
        if !name.is_empty() {
            return Some(name.to_ascii_lowercase().starts_with("qwen"));
        }
    }
    let arch = map.get("architectures").and_then(Value::as_array)?;
    if arch.is_empty() {
        return None;
    }
    Some(
        arch.iter()
            .any(|item| item.as_str().is_some_and(|name| name.to_ascii_lowercase().starts_with("qwen"))),
    )
}

fn json_string<'a>(map: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a str> {
    map.get(key).and_then(Value::as_str)
}

fn optional_json_object(path: &Path) -> Option<serde_json::Map<String, Value>> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > TOKENIZER_CHECK_MAX_BYTES {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(map)) => Some(map),
        _ => None,
    }
}

fn load_tokenizer_config(
    path: &Path,
    train_base: Option<&str>,
) -> Result<LoadedTokenizerConfig, ModelError> {
    match std::fs::symlink_metadata(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(LoadedTokenizerConfig::Absent),
        Err(err) => Err(ModelError::Other(format!(
            "refuse:tokenizer: {}: {err}",
            path.display()
        ))),
        Ok(meta) if meta.file_type().is_symlink() => Err(ModelError::Other(format!(
            "refuse:tokenizer: {} is a symlink. {}",
            path.display(),
            tokenizer_restore_sentence(train_base.unwrap_or(""))
        ))),
        Ok(meta) if !meta.is_file() => Err(ModelError::Other(format!(
            "refuse:tokenizer: {} is not a regular file.",
            path.display()
        ))),
        Ok(meta) if meta.len() > TOKENIZER_CHECK_MAX_BYTES => Err(ModelError::Other(format!(
            "refuse:tokenizer: {} is {} bytes. gguf-convert reads at most {} bytes of tokenizer_config.json and does not print the convert line.",
            path.display(),
            meta.len(),
            TOKENIZER_CHECK_MAX_BYTES
        ))),
        Ok(_) => {
            let bytes = std::fs::read(path).map_err(|err| {
                ModelError::Other(format!("refuse:tokenizer: {}: {err}", path.display()))
            })?;
            let text = String::from_utf8(bytes).map_err(|_| {
                ModelError::Other(format!(
                    "refuse:tokenizer: {} is not utf-8. gguf-convert does not print the convert line.",
                    path.display()
                ))
            })?;
            match serde_json::from_str::<Value>(&text) {
                Ok(Value::Object(map)) => Ok(LoadedTokenizerConfig::Object(map)),
                Ok(_) => Err(ModelError::Other(format!(
                    "refuse:tokenizer: {} is not a JSON object. gguf-convert does not print the convert line.",
                    path.display()
                ))),
                Err(_) => Err(ModelError::Other(format!(
                    "refuse:tokenizer: {} is not JSON, so gguf-convert cannot see whether extra_special_tokens is a list. This factory does not rewrite the file and does not print the convert line.",
                    path.display()
                ))),
            }
        }
    }
}

fn bpe_file_problem(dir: &Path, name: &str) -> Result<Option<String>, ModelError> {
    let path = dir.join(name);
    match std::fs::symlink_metadata(&path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Some(format!("missing {name}"))),
        Err(err) => Err(ModelError::Other(format!(
            "refuse:tokenizer: {}: {err}",
            path.display()
        ))),
        Ok(meta) if meta.is_file() => Ok(None),
        Ok(meta) if meta.file_type().is_symlink() => {
            if path.is_file() {
                Ok(None)
            } else {
                Ok(Some(format!(
                    "{name} is a symlink whose target is not a file"
                )))
            }
        }
        Ok(_) => Ok(Some(format!("{name} is not a file"))),
    }
}

/// Read `prepare.json`, require a merged export, and build the convert report.
/// Does not write and does not spawn a process.
pub fn plan_gguf_convert(
    prepared_dir: &Path,
    weights: &Path,
) -> Result<GgufConvertPlan, ModelError> {
    refuse_sacred_and_sku("prepared", &prepared_dir.display().to_string())?;
    refuse_sacred_and_sku("weights", &weights.display().to_string())?;
    let doc = load_prepare_doc(&prepared_dir.join("prepare.json"))?;
    let mlx = doc.driver == MLX_LM_LORA_ID;
    let unsloth = doc.driver == UNSLOTH_QLORA_ID;
    if !mlx && !is_post_merge_print_driver(&doc.driver) {
        return Err(refuse_post_merge_driver("gguf-convert", &doc.driver));
    }
    if doc.job != EnrichJobKind::Train.as_str() {
        return Err(ModelError::Other(format!(
            "refuse:job: gguf-convert expects job train, found '{}'",
            doc.job
        )));
    }
    let seat_tag = doc
        .seat_tag
        .clone()
        .ok_or_else(|| ModelError::Other("refuse:seat: prepare.json has no seat_tag".into()))?;
    let local_tag = local_enrich_tag(&doc.pack_id);
    if mlx {
        refuse_recipe_train_record(&doc, prepared_dir)?;
        return Err(refuse_mlx_gguf_convert(weights));
    }
    if unsloth {
        refuse_recipe_train_record(&doc, prepared_dir)?;
    }
    let shape = match classify_weights(weights) {
        Ok(shape) => shape,
        Err(err) if unsloth && err.to_string().contains("is an adapter directory") => {
            return Err(crate::merge_adapt::refuse_unsloth_adapter_weights(weights));
        }
        Err(err) => return Err(err),
    };
    let dir = match shape {
        WeightsShape::Merged { dir, .. } => dir,
        WeightsShape::Gguf { file, .. } => {
            return Err(ModelError::Other(format!(
                "refuse:seat: {} is a GGUF. gguf-convert prints the llama.cpp line for a merged export directory (config.json and a .safetensors file whose name does not start with adapter_model). Seat this file with estate enrich local-seat --prepared {} --weights {}.",
                file.display(),
                prepared_dir.display(),
                file.display()
            )));
        }
    };
    let tokenizer_note = inspect_export_tokenizer(&dir, doc.train_base_model.as_deref())?;
    let convert = printed_convert_line(&dir);
    let outfile = sibling_gguf_outfile(&dir);
    let seat = local_seat_cli(prepared_dir, &outfile);
    let import = crate::local_seat::import_trained_line(prepared_dir, &local_tag, &outfile);
    let axolotl_note = if is_axolotl_driver(&doc.driver) {
        "Axolotl does not write GGUF. The operator merged this Hugging Face directory outside this factory. This factory does not invent an Axolotl converter.\n\
         \n"
    } else {
        ""
    };
    let unsloth_note = if unsloth {
        let manual = crate::merge_adapt::printed_unsloth_manual_block(&dir);
        format!(
            "Unsloth's saving-to-gguf page publishes three manual convert lines after save_pretrained_merged with save_method merged_16bit ({gguf_doc}). Those lines use python llama.cpp/convert_hf_to_gguf.py with --outtype f16, bf16, and q8_0, and --split-max-size 50G. The page's outfile names are model-F16.gguf, model-BF16.gguf, and model-Q8_0.gguf. This print uses this merged directory where the page writes merged_model. Unsloth's page does not publish --outtype auto. The python3 line below is the llama.cpp script default this factory already prints for llamafactory-lora, llamafactory-qlora, axolotl-lora, and axolotl-qlora. This factory does not run either form and does not print the page's apt-get or cmake build.\n\
             \n\
             {manual}\n\
             \n",
            gguf_doc = UNSLOTH_GGUF_DOC,
        )
    } else {
        String::new()
    };
    let quant_sentence = if unsloth {
        "The python3 line passes --outtype auto so the flag is not guessed. It does not choose a quantization type. Unsloth's page publishes q8_0 as one of the three manual lines above. This factory does not add tq1_0 or tq2_0 and does not print a llama-quantize command."
    } else {
        "q8_0, tq1_0, and tq2_0 are quantization-style types in that script. This factory does not print them. llama-quantize is the later llama.cpp tool. This factory does not choose a quantization type and does not print a quant command."
    };
    let report = format!(
        "gguf-convert: shape=merged seat_tag={seat_tag} local_tag={local_tag}\n\
         pack={pack_id}\n\
         driver={driver}\n\
         weights={weights}\n\
         outfile={outfile}\n\
         promoted=false auto_apply=false estate_rewritten=false\n\
         \n\
         {axolotl_note}\
         {unsloth_note}\
         Run this from a llama.cpp checkout. convert_hf_to_gguf.py is that checkout's script. Its shebang is python3. --outtype auto is the script default: the highest-fidelity 16-bit float type (f16 or bf16) from the first loaded tensor. This line passes that default so the flag is not guessed. {quant_sentence} The outfile is a sibling of the merged directory. A .gguf file inside that directory makes the directory match two shapes, and local-seat and import-trained then refuse the directory.\n\
         \n\
         {tokenizer_note}\n\
         \n\
         {convert}\n\
         \n\
         Then seat that file. local-seat prints the ollama create line and, for that GGUF, llama-cli -m and llama-server -m. It does not create the model and does not run those programs.\n\
         \n\
         {seat}\n\
         \n\
         After that local-seat print, run the printed ollama create line yourself. The same step stands when you already ran ollama create outside this factory. This factory did not run ollama create. The standing next step records that GGUF. trained_shape is gguf. The proposal stays auto_apply=false. import-trained does not apply the estate and does not promote.\n\
         \n\
         {import}\n\
         \n\
         gguf-convert did not convert and did not write {outfile}.\n\
         READY_FOR_LIVE_TEST: no.\n",
        pack_id = doc.pack_id.as_str(),
        driver = doc.driver.as_str(),
        weights = dir.display(),
        outfile = outfile.display(),
        axolotl_note = axolotl_note,
        unsloth_note = unsloth_note,
        quant_sentence = quant_sentence,
        tokenizer_note = tokenizer_note,
    );
    refuse_sacred_and_sku("gguf-convert report", &report)?;
    refuse_raw_secrets(&report).map_err(map_feed)?;
    Ok(GgufConvertPlan {
        seat_tag,
        local_tag,
        pack_id: doc.pack_id,
        driver: doc.driver,
        convert_command: convert,
        outfile,
        local_seat_command: seat,
        report,
    })
}

fn refuse_mlx_gguf_convert(weights: &Path) -> ModelError {
    match classify_weights(weights) {
        Ok(WeightsShape::Gguf { file, .. }) => ModelError::Other(format!(
            "refuse:seat: {} is a GGUF. gguf-convert does not print a converter for a file that is already GGUF. Seat this file with estate enrich local-seat --weights. mlx_lm.fuse --export-gguf is the documented way mlx-lm writes that file. This factory does not invent a convert script.",
            file.display()
        )),
        Ok(WeightsShape::Merged { dir, .. }) => {
            crate::merge_adapt::refuse_mlx_hf_weights("gguf-convert", &dir)
        }
        Err(err) if err.to_string().contains("is an adapter directory") => {
            crate::merge_adapt::refuse_mlx_adapter_weights(weights)
        }
        Err(err) if err.to_string().contains("more than one shape") => {
            crate::merge_adapt::refuse_mlx_mixed_weights(weights)
        }
        Err(err) => err,
    }
}

fn map_feed(err: FeedError) -> ModelError {
    let text = err.to_string();
    if text.starts_with("refuse:") {
        ModelError::Other(text)
    } else {
        ModelError::Other(format!("refuse:seat: {text}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::train_enrich::{
        AXOLOTL_LORA_ID, AXOLOTL_QLORA_ID, LLAMAFACTORY_LORA_ID, LLAMAFACTORY_QLORA_ID,
        PREPARE_SCHEMA,
    };
    use std::os::unix::fs::PermissionsExt;

    fn tmp(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cell-one-gguf-convert-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_prepare(dir: &Path, driver: &str, job: &str, seat: Option<&str>, promoted: bool) {
        let seat_line = match seat {
            Some(seat) => format!("  \"seat_tag\": \"{seat}\",\n"),
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
               \"purpose\": \"fixture\",\n\
               \"host_class_affinity\": \"any\",\n\
               \"source_paths\": [],\n\
               \"source_drivers\": [],\n\
               \"artifacts\": [\"export.yaml\"],\n\
               \"promoted\": {promoted},\n\
               \"auto_apply\": false,\n\
               \"estate_rewritten\": false,\n\
               \"note\": \"test\"\n\
             }}\n"
        );
        std::fs::write(dir.join("prepare.json"), body).unwrap();
    }

    fn merged(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("config.json"), "{}\n").unwrap();
        std::fs::write(dir.join("model.safetensors"), b"not-a-real-tensor").unwrap();
    }

    fn gguf_bytes() -> Vec<u8> {
        let mut bytes = b"GGUF".to_vec();
        bytes.extend_from_slice(&[0u8; 12]);
        bytes
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
    fn source_does_not_spawn_or_convert() {
        let src = include_str!("gguf_convert.rs");
        let command_new = format!("{}::{}", "Command", "new");
        let process_command = format!("{}::{}::{}", "std", "process", "Command");
        assert!(!src.contains(&command_new), "{src}");
        assert!(!src.contains(&process_command), "{src}");
        let seat = include_str!("local_seat.rs");
        assert!(!seat.contains(&command_new), "{seat}");
    }

    #[test]
    fn merged_export_prints_the_llama_cpp_line_and_writes_nothing() {
        let root = tmp("merged");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        merged(&export);
        std::fs::write(export.join("Modelfile"), "FROM .\n").unwrap();
        let before = names(&export);
        let plan = plan_gguf_convert(&root, &export).unwrap();
        let outfile = root.join("export.gguf");
        assert_eq!(plan.outfile, outfile);
        assert_eq!(plan.seat_tag, "llama3");
        assert_eq!(plan.local_tag, "cell-enrich-overnight-traces");
        assert_eq!(plan.driver, LLAMAFACTORY_QLORA_ID);
        let convert = format!(
            "python3 convert_hf_to_gguf.py {} --outfile {} --outtype auto",
            export.display(),
            outfile.display()
        );
        assert_eq!(plan.convert_command, convert);
        assert!(plan.report.contains(&convert), "{}", plan.report);
        assert!(plan.report.contains("shape=merged"), "{}", plan.report);
        assert!(plan.report.contains("seat_tag=llama3"), "{}", plan.report);
        assert!(
            plan.report.contains(&format!(
                "estate enrich local-seat --prepared {} --weights {}",
                root.display(),
                outfile.display()
            )),
            "{}",
            plan.report
        );
        assert_eq!(plan.local_seat_command, local_seat_cli(&root, &outfile));
        assert!(plan.report.contains("standing next step"), "{}", plan.report);
        assert!(plan.report.contains("auto_apply=false"), "{}", plan.report);
        assert!(
            plan.report.contains("did not run ollama create"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("does not apply the estate"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains(&format!(
                "estate enrich import-trained --estate <estate.yaml> --prepared {} --tag cell-enrich-overnight-traces --adapter {}",
                root.display(),
                outfile.display()
            )),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("gguf-convert did not convert"),
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
        assert!(
            !plan.convert_command.contains("q8_0"),
            "{}",
            plan.convert_command
        );
        assert!(
            !plan.convert_command.contains("llama-quantize"),
            "{}",
            plan.convert_command
        );
        assert!(
            plan.report.contains("does not print a quant command"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("promoted=false"), "{}", plan.report);
        assert!(
            plan.report.contains("Tokenizer check passed"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("this directory has no tokenizer_config.json"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("refuse:tokenizer"),
            "{}",
            plan.report
        );
        assert_eq!(names(&export), before);
        assert!(!outfile.exists());
        assert!(!export.join("model.gguf").exists());
    }

    #[test]
    fn lora_prepare_uses_the_same_line() {
        let root = tmp("lora");
        write_prepare(&root, LLAMAFACTORY_LORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        merged(&export);
        let plan = plan_gguf_convert(&root, &export).unwrap();
        assert_eq!(plan.driver, LLAMAFACTORY_LORA_ID);
        assert!(
            plan.convert_command.contains("--outtype auto"),
            "{}",
            plan.convert_command
        );
    }

    #[test]
    fn broken_modelfile_still_prints_the_convert_line() {
        let root = tmp("bad-modelfile");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        merged(&export);
        std::fs::write(export.join("Modelfile"), "TEMPLATE \"\"\"x\"\"\"\n").unwrap();
        let plan = plan_gguf_convert(&root, &export).unwrap();
        assert!(
            plan.convert_command.contains("convert_hf_to_gguf.py"),
            "{}",
            plan.report
        );
        assert!(!plan.report.contains("refuse:modelfile"), "{}", plan.report);
    }

    #[test]
    fn shell_quote_wraps_the_printed_convert_line() {
        let root = tmp("quote");
        let spaced = root.join("my export");
        std::fs::create_dir_all(&spaced).unwrap();
        write_prepare(
            &spaced,
            LLAMAFACTORY_QLORA_ID,
            "train",
            Some("llama3"),
            false,
        );
        let export = spaced.join("export dir");
        merged(&export);
        let plan = plan_gguf_convert(&spaced, &export).unwrap();
        let outfile = spaced.join("export dir.gguf");
        assert_eq!(plan.outfile, outfile);
        assert!(
            plan.convert_command
                .contains(&format!("'{}'", export.display())),
            "{}",
            plan.convert_command
        );
        assert!(
            plan.convert_command
                .contains(&format!("'{}'", outfile.display())),
            "{}",
            plan.convert_command
        );
        assert!(
            plan.convert_command.contains("--outtype auto"),
            "{}",
            plan.convert_command
        );
        assert!(!outfile.exists());
    }

    #[test]
    fn refuses_mismatched_shapes() {
        let root = tmp("refuse");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);

        let missing = root.join("missing");
        let err = plan_gguf_convert(&root, &missing).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");
        assert!(err.to_string().contains("missing"), "{err}");

        let empty = root.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        let err = plan_gguf_convert(&root, &empty).unwrap_err();
        assert!(err.to_string().contains("empty directory"), "{err}");

        let adapter = root.join("adapter");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        let err = plan_gguf_convert(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");
        assert!(err.to_string().contains("adapter directory"), "{err}");
        assert!(
            !err.to_string().contains("python3 convert_hf_to_gguf.py"),
            "{err}"
        );

        let shards = root.join("shards");
        std::fs::create_dir_all(&shards).unwrap();
        std::fs::write(shards.join("config.json"), "{}\n").unwrap();
        std::fs::write(
            shards.join("adapter_model-00001-of-00002.safetensors"),
            b"a",
        )
        .unwrap();
        let err = plan_gguf_convert(&root, &shards).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");
        assert!(err.to_string().contains("adapter_model"), "{err}");
        assert!(!err.to_string().contains("--outtype"), "{err}");

        let half = root.join("half");
        std::fs::create_dir_all(&half).unwrap();
        std::fs::write(half.join("config.json"), "{}\n").unwrap();
        let err = plan_gguf_convert(&root, &half).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");

        let tensors = root.join("tensors");
        std::fs::create_dir_all(&tensors).unwrap();
        std::fs::write(tensors.join("model.safetensors"), b"w").unwrap();
        let err = plan_gguf_convert(&root, &tensors).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");

        let gguf = root.join("model.gguf");
        std::fs::write(&gguf, gguf_bytes()).unwrap();
        let err = plan_gguf_convert(&root, &gguf).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("is a GGUF"), "{text}");
        assert!(text.contains("estate enrich local-seat"), "{text}");
        assert!(!text.contains("python3 convert_hf_to_gguf.py"), "{text}");

        let gguf_dir = root.join("only-gguf");
        std::fs::create_dir_all(&gguf_dir).unwrap();
        std::fs::write(gguf_dir.join("model.gguf"), gguf_bytes()).unwrap();
        let err = plan_gguf_convert(&root, &gguf_dir).unwrap_err();
        assert!(err.to_string().contains("is a GGUF"), "{err}");

        let mixed = root.join("mixed");
        merged(&mixed);
        std::fs::write(mixed.join("adapter_config.json"), "{}\n").unwrap();
        let err = plan_gguf_convert(&root, &mixed).unwrap_err();
        assert!(err.to_string().contains("more than one shape"), "{err}");

        let linked = root.join("linked");
        std::os::unix::fs::symlink(&root.join("export"), &linked).unwrap();
        let err = plan_gguf_convert(&root, &linked).unwrap_err();
        assert!(err.to_string().contains("symlink"), "{err}");

        let marked = root.join("marked");
        merged(&marked);
        let real_cfg = root.join("real-config.json");
        std::fs::write(&real_cfg, "{}\n").unwrap();
        std::fs::remove_file(marked.join("config.json")).unwrap();
        std::os::unix::fs::symlink(&real_cfg, marked.join("config.json")).unwrap();
        let err = plan_gguf_convert(&root, &marked).unwrap_err();
        assert!(err.to_string().contains("symlink"), "{err}");
        assert!(!err.to_string().contains("python3"), "{err}");
    }

    #[test]
    fn refuses_prepare_gates() {
        let root = tmp("prepare");
        let export = root.join("export");
        merged(&export);

        write_prepare(&root, "unsloth-qlora", "train", Some("llama3"), false);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(!err.to_string().contains("--outtype"), "{err}");
        assert!(!err.to_string().contains("python3 convert_hf_to_gguf.py"), "{err}");

        write_prepare(&root, "mlx-lm-lora", "train", Some("llama3"), false);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:host"), "{err}");
        assert!(err.to_string().contains("mlx-lm-lora"), "{err}");
        assert!(err.to_string().contains("apple-silicon"), "{err}");
        assert!(!err.to_string().contains("--outtype"), "{err}");
        assert!(
            !err.to_string().contains("python3 convert_hf_to_gguf.py"),
            "{err}"
        );
        assert!(!err.to_string().contains("mlx_lm.fuse"), "{err}");

        write_prepare(&root, "ollama-modelfile", "enrich", Some("llama3"), false);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:driver"), "{err}");

        write_prepare(
            &root,
            LLAMAFACTORY_QLORA_ID,
            "enrich",
            Some("llama3"),
            false,
        );
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");

        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", None, false);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");

        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("qwen"), false);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:prepare"), "{err}");

        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), true);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:prepared"), "{err}");

        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let sacred = root.join("cyera-export");
        merged(&sacred);
        let err = plan_gguf_convert(&root, &sacred).unwrap_err();
        assert!(err.to_string().contains("refuse:sacred"), "{err}");
    }

    #[test]
    fn trap_binaries_are_not_required_to_print() {
        let root = tmp("trap");
        write_prepare(&root, LLAMAFACTORY_LORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        merged(&export);
        let bin = root.join("trap-bin");
        std::fs::create_dir_all(&bin).unwrap();
        let sentinel = root.join("spawned");
        let script = format!("#!/bin/sh\necho spawned >> {}\n", sentinel.display());
        for name in [
            "python",
            "python3",
            "llama-quantize",
            "convert_hf_to_gguf.py",
        ] {
            let path = bin.join(name);
            std::fs::write(&path, &script).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let plan = plan_gguf_convert(&root, &export).unwrap();
        assert!(plan
            .convert_command
            .contains("python3 convert_hf_to_gguf.py"));
        assert!(!sentinel.exists());
        assert!(!root.join("export.gguf").exists());
    }

    #[test]
    fn axolotl_merged_dir_prints_the_llama_cpp_line() {
        for driver in [AXOLOTL_LORA_ID, AXOLOTL_QLORA_ID] {
            let root = tmp(&format!("ax-{driver}"));
            write_prepare(&root, driver, "train", Some("llama3"), false);
            assert!(!root.join("export.yaml").exists());
            let merged_dir = root.join("merged");
            merged(&merged_dir);
            let before = names(&merged_dir);
            let plan = plan_gguf_convert(&root, &merged_dir).unwrap();
            let outfile = root.join("merged.gguf");
            assert_eq!(plan.driver, driver);
            assert_eq!(plan.outfile, outfile);
            assert_eq!(plan.seat_tag, "llama3");
            assert_eq!(plan.local_tag, "cell-enrich-overnight-traces");
            let convert = format!(
                "python3 convert_hf_to_gguf.py {} --outfile {} --outtype auto",
                merged_dir.display(),
                outfile.display()
            );
            assert_eq!(plan.convert_command, convert);
            assert_eq!(plan.local_seat_command, local_seat_cli(&root, &outfile));
            assert!(plan.report.contains(&convert), "{}", plan.report);
            assert!(
                plan.report.contains("Axolotl does not write GGUF"),
                "{}",
                plan.report
            );
            assert!(
                plan.report.contains("does not invent an Axolotl converter"),
                "{}",
                plan.report
            );
            assert!(!plan.report.contains("Axolotl wrote"), "{}", plan.report);
            assert!(
                !plan.report.contains("LLaMA-Factory wrote"),
                "{}",
                plan.report
            );
            assert!(!plan.report.contains("llamafactory-cli"), "{}", plan.report);
            assert!(!plan.report.contains("merge-lora"), "{}", plan.report);
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
            assert!(!outfile.exists());
            assert_eq!(names(&merged_dir), before);

            std::fs::write(merged_dir.join("export.yaml"), "export_dir: merged\n").unwrap();
            let beside = plan_gguf_convert(&root, &merged_dir).unwrap();
            assert!(
                beside.convert_command.contains("--outtype auto"),
                "{}",
                beside.convert_command
            );
            assert!(!outfile.exists());
        }
    }

    #[test]
    fn axolotl_refuses_adapter_lf_card_and_symlink() {
        let root = tmp("ax-refuse");
        write_prepare(&root, AXOLOTL_QLORA_ID, "train", Some("llama3"), false);

        let adapter = root.join("outputs");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        std::fs::write(adapter.join("adapter_model.safetensors"), b"w").unwrap();
        let err = plan_gguf_convert(&root, &adapter).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("adapter directory"), "{text}");
        assert!(!text.contains("python3 convert_hf_to_gguf.py"), "{text}");

        let shards = root.join("shards");
        std::fs::create_dir_all(&shards).unwrap();
        std::fs::write(shards.join("config.json"), "{}\n").unwrap();
        std::fs::write(shards.join("adapter_model.safetensors"), b"a").unwrap();
        let err = plan_gguf_convert(&root, &shards).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("adapter_model"), "{text}");
        assert!(!text.contains("--outtype"), "{text}");

        let card = root.join("export-card");
        std::fs::create_dir_all(&card).unwrap();
        std::fs::write(card.join("export.yaml"), "adapter_name_or_path: outputs\n").unwrap();
        let err = plan_gguf_convert(&root, &card).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("not a merged export directory"), "{text}");
        assert!(!text.contains("--outtype"), "{text}");

        let card_file = root.join("export.yaml");
        std::fs::write(&card_file, "export_dir: export\n").unwrap();
        let err = plan_gguf_convert(&root, &card_file).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(!text.contains("python3 convert_hf_to_gguf.py"), "{text}");

        let err = plan_gguf_convert(&root, &root).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");
        assert!(
            !err.to_string().contains("python3 convert_hf_to_gguf.py"),
            "{err}"
        );

        let real = root.join("real-merged");
        merged(&real);
        let linked = root.join("linked");
        std::os::unix::fs::symlink(&real, &linked).unwrap();
        let err = plan_gguf_convert(&root, &linked).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("symlink"), "{text}");
        assert!(!text.contains("--outtype"), "{text}");

        let marked = root.join("marked");
        merged(&marked);
        let real_cfg = root.join("real-config.json");
        std::fs::write(&real_cfg, "{}\n").unwrap();
        std::fs::remove_file(marked.join("config.json")).unwrap();
        std::os::unix::fs::symlink(&real_cfg, marked.join("config.json")).unwrap();
        let err = plan_gguf_convert(&root, &marked).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("symlink"), "{text}");
        assert!(!text.contains("python3"), "{text}");

        write_prepare(&root, AXOLOTL_LORA_ID, "enrich", Some("llama3"), false);
        let err = plan_gguf_convert(&root, &real).unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");

        write_prepare(&root, AXOLOTL_LORA_ID, "train", Some("qwen"), false);
        let err = plan_gguf_convert(&root, &real).unwrap_err();
        assert!(err.to_string().contains("refuse:prepare"), "{err}");

        write_prepare(&root, AXOLOTL_LORA_ID, "train", Some("llama3"), true);
        let err = plan_gguf_convert(&root, &real).unwrap_err();
        assert!(err.to_string().contains("refuse:prepared"), "{err}");

        write_prepare(&root, AXOLOTL_QLORA_ID, "train", Some("llama3"), false);
        let sacred = root.join("cyera-merged");
        merged(&sacred);
        let err = plan_gguf_convert(&root, &sacred).unwrap_err();
        assert!(err.to_string().contains("refuse:sacred"), "{err}");
        assert!(!err.to_string().contains("--outtype"), "{err}");
    }

    fn write_mlx(dir: &Path, job: &str, host: &str, promoted: bool) {
        let body = format!(
            "{{\n\
               \"schema\": \"{PREPARE_SCHEMA}\",\n\
               \"driver\": \"mlx-lm-lora\",\n\
               \"job\": \"{job}\",\n\
               \"pack_id\": \"overnight-traces\",\n\
               \"base_model\": \"llama3\",\n\
               \"seat_tag\": \"llama3\",\n\
               \"train_base_model\": \"Qwen/Qwen2.5-0.5B-Instruct\",\n\
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

    fn write_mlx_md(dir: &Path) {
        std::fs::write(
            dir.join("MLX.md"),
            "train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"\nseat_tag: \"llama3\"\nhost_class_affinity: apple-silicon\n",
        )
        .unwrap();
    }

    fn assert_no_convert_line(text: &str) {
        assert!(!text.contains("python3 convert_hf_to_gguf.py"), "{text}");
        assert!(!text.contains("--outtype"), "{text}");
    }

    #[test]
    fn mlx_gguf_convert_refuses_and_names_the_documented_export() {
        let root = tmp("mlx-convert");
        write_mlx(&root, "train", "apple-silicon", false);
        write_mlx_md(&root);
        let before = names(&root);

        let gguf = root.join("ggml-model-f16.gguf");
        std::fs::write(&gguf, gguf_bytes()).unwrap();
        let err = plan_gguf_convert(&root, &gguf).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("is a GGUF"), "{text}");
        assert!(text.contains("estate enrich local-seat"), "{text}");
        assert!(text.contains("--export-gguf"), "{text}");
        assert_no_convert_line(&text);

        let fused = root.join("fused_model");
        merged(&fused);
        let err = plan_gguf_convert(&root, &fused).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("MLX weights"), "{text}");
        assert!(text.contains("ggml-model-f16.gguf"), "{text}");
        assert!(text.contains("does not invent a convert script"), "{text}");
        assert_no_convert_line(&text);
        assert!(!text.contains("estate enrich gguf-convert --"), "{text}");

        let adapter = root.join("adapters");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        std::fs::write(adapter.join("adapters.safetensors"), b"w").unwrap();
        let err = plan_gguf_convert(&root, &adapter).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("adapter directory"), "{text}");
        assert!(text.contains("adapters.safetensors"), "{text}");
        assert_no_convert_line(&text);

        std::fs::write(fused.join("ggml-model-f16.gguf"), gguf_bytes()).unwrap();
        let err = plan_gguf_convert(&root, &fused).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("more than one shape"), "{text}");
        assert!(text.contains("ggml-model-f16.gguf"), "{text}");
        assert_no_convert_line(&text);

        let linked = root.join("linked-fused");
        std::os::unix::fs::symlink(&fused, &linked).unwrap();
        let err = plan_gguf_convert(&root, &linked).unwrap_err();
        assert!(err.to_string().contains("symlink"), "{err}");
        assert_no_convert_line(&err.to_string());

        write_mlx(&root, "enrich", "apple-silicon", false);
        let err = plan_gguf_convert(&root, &gguf).unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");
        assert_no_convert_line(&err.to_string());

        write_mlx(&root, "train", "apple-silicon", false);
        std::fs::remove_file(root.join("MLX.md")).unwrap();
        let err = plan_gguf_convert(&root, &gguf).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("MLX.md"), "{err}");

        let real = root.join("real-mlx.md");
        write_mlx_md(&root);
        std::fs::rename(root.join("MLX.md"), &real).unwrap();
        std::os::unix::fs::symlink(&real, root.join("MLX.md")).unwrap();
        let err = plan_gguf_convert(&root, &gguf).unwrap_err();
        assert!(err.to_string().contains("refuse:host"), "{err}");
        assert!(err.to_string().contains("symlink"), "{err}");

        write_mlx(&root, "train", "apple-silicon", true);
        std::fs::remove_file(root.join("MLX.md")).unwrap();
        write_mlx_md(&root);
        let err = plan_gguf_convert(&root, &gguf).unwrap_err();
        assert!(err.to_string().contains("refuse:prepared"), "{err}");

        write_mlx(&root, "train", "rented-nvidia", false);
        let err = plan_gguf_convert(&root, &fused).unwrap_err();
        assert!(err.to_string().contains("refuse:host"), "{err}");
        assert!(!err.to_string().contains("MLX weights"), "{err}");

        let sacred = root.join("cyera-gguf");
        std::fs::write(&sacred, gguf_bytes()).unwrap();
        write_mlx(&root, "train", "apple-silicon", false);
        let err = plan_gguf_convert(&root, &sacred).unwrap_err();
        assert!(err.to_string().contains("refuse:sacred"), "{err}");

        assert!(before.iter().all(|name| name != "export.gguf"));
        assert!(!root.join("fused_model.gguf").exists());
    }

    fn write_unsloth(dir: &Path, job: &str, promoted: bool) {
        let body = format!(
            "{{\n\
               \"schema\": \"{PREPARE_SCHEMA}\",\n\
               \"driver\": \"unsloth-qlora\",\n\
               \"job\": \"{job}\",\n\
               \"pack_id\": \"overnight-traces\",\n\
               \"base_model\": \"llama3\",\n\
               \"seat_tag\": \"llama3\",\n\
               \"train_base_model\": \"Qwen/Qwen2.5-0.5B-Instruct\",\n\
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
        std::fs::write(
            dir.join("UNSLOTH.md"),
            "train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"\nseat_tag: \"llama3\"\n",
        )
        .unwrap();
    }

    #[test]
    fn unsloth_merged_dir_prints_both_convert_lines_and_writes_nothing() {
        let root = tmp("unsloth-convert");
        write_unsloth(&root, "train", false);
        let export = root.join("merged");
        merged(&export);
        let prepare_before = std::fs::read(root.join("prepare.json")).unwrap();
        let plan = plan_gguf_convert(&root, &export).unwrap();
        let outfile = root.join("merged.gguf");
        assert_eq!(plan.outfile, outfile);
        assert_eq!(plan.convert_command, printed_convert_line(&export));
        assert!(plan.convert_command.contains("--outtype auto"), "{}", plan.convert_command);
        assert!(plan.report.contains("--outtype f16"), "{}", plan.report);
        assert!(plan.report.contains("--outtype bf16"), "{}", plan.report);
        assert!(plan.report.contains("--outtype q8_0"), "{}", plan.report);
        assert!(plan.report.contains("model-F16.gguf"), "{}", plan.report);
        assert!(plan.report.contains("--split-max-size 50G"), "{}", plan.report);
        assert!(
            plan.report.contains("does not publish --outtype auto"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("READY_FOR_LIVE_TEST: no"), "{}", plan.report);
        assert!(!plan.report.contains("READY_FOR_LIVE_TEST: yes"), "{}", plan.report);
        assert!(!outfile.exists());
        assert_eq!(std::fs::read(root.join("prepare.json")).unwrap(), prepare_before);

        std::fs::remove_file(root.join("UNSLOTH.md")).unwrap();
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("UNSLOTH.md is missing"), "{err}");
        assert!(!err.to_string().contains("--outtype"), "{err}");

        let real = root.join("real-unsloth.md");
        std::fs::write(
            &real,
            "train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"\nseat_tag: \"llama3\"\n",
        )
        .unwrap();
        std::os::unix::fs::symlink(&real, root.join("UNSLOTH.md")).unwrap();
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("symlink"), "{err}");
        assert!(!err.to_string().contains("--outtype"), "{err}");

        std::fs::remove_file(root.join("UNSLOTH.md")).unwrap();
        std::fs::write(
            root.join("UNSLOTH.md"),
            "train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"\nseat_tag: \"llama3\"\n",
        )
        .unwrap();
        let adapter = root.join("lora");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        std::fs::write(adapter.join("adapter_model.safetensors"), b"w").unwrap();
        let err = plan_gguf_convert(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");
        assert!(!err.to_string().contains("Pass --adapter"), "{err}");
        assert!(!err.to_string().contains("--outtype"), "{err}");

        write_unsloth(&root, "enrich", false);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");

        write_unsloth(&root, "train", true);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:prepared"), "{err}");
        assert!(!outfile.exists());
    }

    fn write_prepare_train(dir: &Path, train: &str) {
        let body = format!(
            "{{\n\
               \"schema\": \"{PREPARE_SCHEMA}\",\n\
               \"driver\": \"{LLAMAFACTORY_QLORA_ID}\",\n\
               \"job\": \"train\",\n\
               \"pack_id\": \"overnight-traces\",\n\
               \"base_model\": \"llama3\",\n\
               \"seat_tag\": \"llama3\",\n\
               \"train_base_model\": \"{train}\",\n\
               \"purpose\": \"fixture\",\n\
               \"host_class_affinity\": \"any\",\n\
               \"source_paths\": [],\n\
               \"source_drivers\": [],\n\
               \"artifacts\": [\"export.yaml\"],\n\
               \"promoted\": false,\n\
               \"auto_apply\": false,\n\
               \"estate_rewritten\": false,\n\
               \"note\": \"test\"\n\
             }}\n"
        );
        std::fs::write(dir.join("prepare.json"), body).unwrap();
    }

    fn qwen_config() -> &'static str {
        "{\"model_type\":\"qwen2\",\"architectures\":[\"Qwen2ForCausalLM\"]}\n"
    }

    fn assert_names_hf_cache_restore(text: &str) {
        assert!(text.contains("HF cache snapshot"), "{text}");
        assert!(text.contains("equivalent base checkout"), "{text}");
        assert!(text.contains("into the export directory"), "{text}");
        assert!(text.contains("HF hub snapshots are often symlinks"), "{text}");
        assert!(text.contains("cp -aL"), "{text}");
        assert!(text.contains("cp --dereference"), "{text}");
        assert!(text.contains("real files, not symlinks"), "{text}");
        assert!(text.contains("plain cp -a"), "{text}");
        assert!(
            text.contains("does not follow a symlinked tokenizer_config.json"),
            "{text}"
        );
        assert!(text.contains("re-run estate enrich gguf-convert"), "{text}");
        assert!(text.contains("tokenizer_config.json.bak"), "{text}");
        assert!(text.contains("does not download weights"), "{text}");
        assert!(text.contains("does not copy those files"), "{text}");
        assert!(!text.contains("python3 convert_hf_to_gguf.py"), "{text}");
    }

    fn assert_refuse_named_before_rerun(text: &str) {
        let refuse_at = text
            .find("refuse:tokenizer")
            .unwrap_or_else(|| panic!("missing refuse:tokenizer in {text}"));
        let rerun_at = text
            .find("Then re-run estate enrich gguf-convert")
            .unwrap_or_else(|| panic!("missing re-run in {text}"));
        assert!(
            refuse_at < rerun_at,
            "refuse must be named before the re-run: {text}"
        );
        assert!(
            !text[rerun_at..].contains("returns refuse:tokenizer"),
            "the re-run must not be followed by the refuse claim: {text}"
        );
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

    fn assert_refuses_tokenizer(err: &impl std::fmt::Display) {
        let text = err.to_string();
        assert!(text.contains("refuse:tokenizer"), "{text}");
        assert_names_hf_cache_restore(&text);
        assert_refuse_named_before_rerun(&text);
        assert!(!text.contains("--outtype"), "{text}");
    }

    #[test]
    fn tokenizer_restore_sentence_names_the_hf_cache_and_the_rerun() {
        let named = super::tokenizer_restore_sentence("Qwen/Qwen2.5-0.5B-Instruct");
        assert!(
            named.contains("train base Qwen/Qwen2.5-0.5B-Instruct"),
            "{named}"
        );
        assert_names_hf_cache_restore(&named);
        let unnamed = super::tokenizer_restore_sentence("  ");
        assert!(unnamed.contains("the train base on this prepare"), "{unnamed}");
        assert_names_hf_cache_restore(&unnamed);
        let guidance = super::export_tokenizer_guidance("Qwen/Qwen2.5-0.5B-Instruct");
        assert!(guidance.contains("refuse:tokenizer"), "{guidance}");
        assert!(guidance.contains("JSON null"), "{guidance}");
        assert_names_hf_cache_restore(&guidance);
        assert_guidance_refuse_before_rerun(&guidance);
    }

    #[test]
    fn list_extra_special_tokens_refuses_and_writes_nothing() {
        let root = tmp("list-tokens");
        write_prepare_train(&root, "Qwen/Qwen2.5-0.5B-Instruct");
        let export = root.join("export");
        merged(&export);
        std::fs::write(export.join("config.json"), qwen_config()).unwrap();
        std::fs::write(
            export.join("tokenizer_config.json"),
            "{\n  \"extra_special_tokens\": [\"<|im_start|>\", \"<|im_end|>\"],\n  \"tokenizer_class\": \"Qwen2Tokenizer\"\n}\n",
        )
        .unwrap();
        std::fs::write(export.join("vocab.json"), "{}\n").unwrap();
        std::fs::write(export.join("merges.txt"), "a b\n").unwrap();
        let before = names(&export);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert_refuses_tokenizer(&err);
        let text = err.to_string();
        assert!(text.contains("JSON list"), "{text}");
        assert!(text.contains("AttributeError"), "{text}");
        assert!(text.contains("Qwen/Qwen2.5-0.5B-Instruct"), "{text}");
        assert!(!text.contains("missing vocab.json"), "{text}");
        assert_eq!(names(&export), before);
        assert!(!export.join("tokenizer_config.json.bak").exists());
        assert!(!root.join("export.gguf").exists());
    }

    #[test]
    fn null_extra_special_tokens_refuses_and_writes_nothing() {
        let root = tmp("null-tokens");
        write_prepare_train(&root, "Qwen/Qwen2.5-0.5B-Instruct");
        let export = root.join("export");
        merged(&export);
        std::fs::write(export.join("config.json"), qwen_config()).unwrap();
        std::fs::write(
            export.join("tokenizer_config.json"),
            "{\"extra_special_tokens\":null,\"tokenizer_class\":\"Qwen2Tokenizer\"}\n",
        )
        .unwrap();
        std::fs::write(export.join("vocab.json"), "{}\n").unwrap();
        std::fs::write(export.join("merges.txt"), "a b\n").unwrap();
        let before = names(&export);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert_refuses_tokenizer(&err);
        let text = err.to_string();
        assert!(text.contains("JSON null"), "{text}");
        assert!(!text.contains("JSON list"), "{text}");
        assert!(!text.contains("missing vocab.json"), "{text}");
        assert!(!text.contains("missing merges.txt"), "{text}");
        assert!(text.contains("Qwen/Qwen2.5-0.5B-Instruct"), "{text}");
        assert_eq!(names(&export), before);
        assert!(!export.join("tokenizer_config.json.bak").exists());
        assert!(!root.join("export.gguf").exists());
    }

    #[test]
    fn qwen_export_missing_bpe_files_refuses_both_names() {
        let root = tmp("missing-bpe");
        write_prepare_train(&root, "Qwen/Qwen2.5-0.5B-Instruct");
        let export = root.join("export");
        merged(&export);
        std::fs::write(export.join("config.json"), qwen_config()).unwrap();
        std::fs::write(
            export.join("tokenizer_config.json"),
            "{\"extra_special_tokens\":{\"im_end\":\"<|im_end|>\"},\"tokenizer_class\":\"Qwen2Tokenizer\"}\n",
        )
        .unwrap();
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert_refuses_tokenizer(&err);
        let text = err.to_string();
        assert!(text.contains("missing vocab.json"), "{text}");
        assert!(text.contains("missing merges.txt"), "{text}");
        assert!(!root.join("export.gguf").exists());

        std::fs::write(export.join("vocab.json"), "{}\n").unwrap();
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:tokenizer"), "{text}");
        assert!(text.contains("missing merges.txt"), "{text}");
        assert!(!text.contains("missing vocab.json"), "{text}");
        assert!(!text.contains("python3 convert_hf_to_gguf.py"), "{text}");
    }

    #[test]
    fn qwen_list_and_missing_bpe_files_are_one_refuse() {
        let root = tmp("list-and-missing");
        write_prepare(&root, LLAMAFACTORY_LORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        merged(&export);
        std::fs::write(
            export.join("config.json"),
            "{\"architectures\":[\"Qwen2ForCausalLM\"]}\n",
        )
        .unwrap();
        std::fs::write(
            export.join("tokenizer_config.json"),
            "{\"extra_special_tokens\":[]}\n",
        )
        .unwrap();
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("JSON list"), "{text}");
        assert!(text.contains("missing vocab.json"), "{text}");
        assert!(text.contains("missing merges.txt"), "{text}");
        assert!(text.contains("the train base on this prepare"), "{text}");
        assert!(!text.contains("--outtype"), "{text}");
    }

    #[test]
    fn restored_qwen_tokenizer_prints_the_convert_line() {
        let root = tmp("restored-qwen");
        write_prepare_train(&root, "Qwen/Qwen2.5-0.5B-Instruct");
        let export = root.join("export");
        merged(&export);
        std::fs::write(export.join("config.json"), qwen_config()).unwrap();
        std::fs::write(
            export.join("tokenizer_config.json"),
            "{\"extra_special_tokens\":{\"im_end\":\"<|im_end|>\"},\"tokenizer_class\":\"Qwen2Tokenizer\"}\n",
        )
        .unwrap();
        std::fs::write(export.join("vocab.json"), "{\"<|endoftext|>\":0}\n").unwrap();
        std::fs::write(export.join("merges.txt"), "a b\n").unwrap();
        let before = names(&export);
        let plan = plan_gguf_convert(&root, &export).unwrap();
        assert!(
            plan.convert_command.contains("python3 convert_hf_to_gguf.py"),
            "{}",
            plan.convert_command
        );
        assert!(plan.convert_command.contains("--outtype auto"), "{}", plan.convert_command);
        assert!(
            plan.report.contains("Tokenizer check passed"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("extra_special_tokens is an object"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("vocab.json and merges.txt are in this directory"),
            "{}",
            plan.report
        );
        assert_refuse_named_before_rerun(&plan.report);
        assert!(
            plan.report.contains("Qwen/Qwen2.5-0.5B-Instruct"),
            "{}",
            plan.report
        );
        assert_eq!(names(&export), before);
        assert!(!root.join("export.gguf").exists());
        assert!(!export.join("tokenizer_config.json.bak").exists());
    }

    #[test]
    fn llama_export_without_bpe_files_still_prints() {
        let root = tmp("llama-no-bpe");
        write_prepare_train(&root, "Qwen/Qwen2.5-0.5B-Instruct");
        let export = root.join("export");
        merged(&export);
        std::fs::write(
            export.join("config.json"),
            "{\"model_type\":\"llama\",\"architectures\":[\"LlamaForCausalLM\"]}\n",
        )
        .unwrap();
        let plan = plan_gguf_convert(&root, &export).unwrap();
        assert!(
            plan.convert_command.contains("--outtype auto"),
            "{}",
            plan.convert_command
        );
        assert!(
            !plan.report.contains("vocab.json and merges.txt are in this directory"),
            "{}",
            plan.report
        );

        std::fs::write(
            export.join("tokenizer_config.json"),
            "{\"extra_special_tokens\":[\"<s>\"],\"tokenizer_class\":\"Qwen2Tokenizer\"}\n",
        )
        .unwrap();
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("JSON list"), "{text}");
        assert!(!text.contains("missing vocab.json"), "{text}");
        assert!(!text.contains("python3 convert_hf_to_gguf.py"), "{text}");
    }

    #[test]
    fn qwen_train_base_without_directory_markers_still_prints() {
        let root = tmp("qwen-base-stub");
        write_prepare_train(&root, "Qwen/Qwen2.5-0.5B-Instruct");
        let export = root.join("export");
        merged(&export);
        let plan = plan_gguf_convert(&root, &export).unwrap();
        assert!(
            plan.convert_command.contains("python3 convert_hf_to_gguf.py"),
            "{}",
            plan.convert_command
        );
        assert!(
            plan.report.contains("Tokenizer check passed"),
            "{}",
            plan.report
        );
    }

    #[test]
    fn tokenizer_class_names_qwen_when_config_omits_model_type() {
        let root = tmp("class-only");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        merged(&export);
        std::fs::write(
            export.join("tokenizer_config.json"),
            "{\"tokenizer_class\":\"Qwen2Tokenizer\"}\n",
        )
        .unwrap();
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("missing vocab.json"), "{text}");
        assert!(text.contains("missing merges.txt"), "{text}");
    }

    #[test]
    fn symlinked_tokenizer_config_refuses_without_reading_the_target() {
        let root = tmp("tok-link");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        merged(&export);
        let real = root.join("real-tokenizer.json");
        std::fs::write(
            &real,
            "{\"extra_special_tokens\":[\"<|im_start|>\"]}\n",
        )
        .unwrap();
        std::os::unix::fs::symlink(&real, export.join("tokenizer_config.json")).unwrap();
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:tokenizer"), "{text}");
        assert!(text.contains("is a symlink"), "{text}");
        assert_eq!(
            text.matches("does not follow a symlinked tokenizer_config.json")
                .count(),
            1,
            "{text}"
        );
        assert_names_hf_cache_restore(&text);
        assert_refuse_named_before_rerun(&text);
        assert!(!text.contains("JSON list"), "{text}");
        assert!(!text.contains("python3"), "{text}");
        assert!(!root.join("export.gguf").exists());
    }

    #[test]
    fn broken_tokenizer_config_refuses() {
        let root = tmp("bad-json");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        merged(&export);
        std::fs::write(export.join("tokenizer_config.json"), "not-json\n").unwrap();
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("not JSON"), "{text}");
        assert!(!text.contains("--outtype"), "{text}");

        std::fs::write(export.join("tokenizer_config.json"), "[1,2]\n").unwrap();
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("not a JSON object"), "{err}");

        std::fs::write(
            export.join("tokenizer_config.json"),
            "{\"extra_special_tokens\":\"<|im_end|>\"}\n",
        )
        .unwrap();
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("JSON string"), "{err}");

        let big = export.join("tokenizer_config.json");
        let mut bytes = vec![b' '; (super::TOKENIZER_CHECK_MAX_BYTES as usize) + 1];
        let last = bytes.len() - 1;
        bytes[0] = b'{';
        bytes[last] = b'}';
        std::fs::write(&big, bytes).unwrap();
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("reads at most"), "{text}");
        assert!(!text.contains("python3 convert_hf_to_gguf.py"), "{text}");
    }

    #[test]
    fn qwen_bpe_symlink_to_a_file_counts_as_present() {
        let root = tmp("bpe-link");
        write_prepare_train(&root, "meta-llama/Llama-3.2-1B-Instruct");
        let export = root.join("export");
        merged(&export);
        std::fs::write(export.join("config.json"), "{\"model_type\":\"qwen2\"}\n").unwrap();
        let vocab = root.join("real-vocab.json");
        let merges = root.join("real-merges.txt");
        std::fs::write(&vocab, "{}\n").unwrap();
        std::fs::write(&merges, "a b\n").unwrap();
        std::os::unix::fs::symlink(&vocab, export.join("vocab.json")).unwrap();
        std::os::unix::fs::symlink(&merges, export.join("merges.txt")).unwrap();
        let plan = plan_gguf_convert(&root, &export).unwrap();
        assert!(
            plan.report.contains("vocab.json and merges.txt are in this directory"),
            "{}",
            plan.report
        );
        assert!(plan.convert_command.contains("--outtype auto"), "{}", plan.convert_command);
    }
}
