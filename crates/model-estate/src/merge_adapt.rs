//! Print-only adapter → merged Hugging Face directory handoff.
//!
//! Axolotl already merges a LoRA adapter with `axolotl merge-lora`
//! ([getting started](https://docs.axolotl.ai/docs/getting-started.html)
//! section 4.4, and the [CLI page](https://docs.axolotl.ai/docs/cli.html)).
//! That command writes `{output_dir}/merged`. It does not take `--out`.
//! LLaMA-Factory's merge card is `llamafactory-cli export` when `export.yaml`
//! is the card you run. When that card was not used, PEFT
//! `merge_and_unload` plus `save_pretrained` is the documented adapter merge
//! (https://huggingface.co/docs/peft/main/en/developer_guides/checkpoint).
//! The directory for that save sits beside `outputs/` in the prepared tree.
//!
//! This module prints those lines, then the `gguf-convert` and `local-seat`
//! lines for the merged directory. It does not merge, spawn, download, or
//! write a directory.

use crate::error::ModelError;
use crate::gguf_convert::{gguf_convert_cli, local_seat_cli, printed_convert_line};
use crate::local_seat::{classify_adapter, shell_quote};
use crate::train_enrich::{
    is_axolotl_driver, is_post_merge_print_driver, load_prepare_doc, local_enrich_tag,
    refuse_post_merge_driver, refuse_sacred_and_sku, EnrichJobKind, EnrichPrepareDoc,
    AXOLOTL_LORA_ID, AXOLOTL_QLORA_ID,
};
use feed_collector::{refuse_raw_secrets, FeedError};
use std::io::Read;
use std::path::{Path, PathBuf};

const CARD_MAX_BYTES: u64 = 1024 * 1024;

/// Printed plan. `merged_dir` is the Hugging Face directory the external
/// merge writes. This command does not create it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeAdaptPlan {
    pub seat_tag: String,
    pub local_tag: String,
    pub pack_id: String,
    pub driver: String,
    /// External merge lines. Axolotl: `axolotl merge-lora`, plus `--dequant`
    /// on the qlora card. LLaMA-Factory adapter path: the PEFT snippet.
    pub merge_commands: Vec<String>,
    pub merged_dir: PathBuf,
    /// `python3 convert_hf_to_gguf.py` line `gguf-convert` would print.
    pub convert_command: String,
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
    if !is_post_merge_print_driver(&doc.driver) {
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
    let shape = classify_adapter(adapter)?;
    let local_tag = local_enrich_tag(&doc.pack_id);
    let plan = if is_axolotl_driver(&doc.driver) {
        plan_axolotl(prepared_dir, &doc, &seat_tag, &local_tag, &shape.dir)?
    } else {
        plan_peft(prepared_dir, &doc, &seat_tag, &local_tag, &shape.dir)?
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

fn plan_peft(
    prepared_dir: &Path,
    doc: &EnrichPrepareDoc,
    seat_tag: &str,
    local_tag: &str,
    adapter_dir: &Path,
) -> Result<MergeAdaptPlan, ModelError> {
    let train = doc.train_base_model.clone().ok_or_else(|| {
        ModelError::Other(
            "refuse:train-base: prepare.json has no train_base_model. The PEFT merge loads that base with AutoModelForCausalLM.from_pretrained. This factory does not invent a Hub repo and does not download weights.".into(),
        )
    })?;
    refuse_sacred_and_sku("train base", &train)?;
    let merged = prepared_dir.join("merged");
    refuse_sacred_and_sku("merged", &merged.display().to_string())?;
    single_line(
        "refuse:merge",
        "merged directory",
        &merged.display().to_string(),
    )?;
    let export_note = export_card_note(prepared_dir)?;
    let snippet = printed_peft_merge(&train, adapter_dir, &merged);
    let convert = printed_convert_line(&merged);
    let gguf = gguf_convert_cli(prepared_dir, &merged);
    let seat = local_seat_cli(prepared_dir, &merged);
    let report = format!(
        "{header}\
         \n\
         {export_note}\
         When export.yaml was not the merge you ran, the adapter directory uses the PEFT calls on https://huggingface.co/docs/peft/main/en/developer_guides/checkpoint and the LoRA guide's `merge_and_unload` example. `merge_and_unload` is not in place, so the result is assigned. `save_pretrained` writes the merged Hugging Face directory. That path is the `--out` for this handoff: {merged}, beside `outputs/` in this prepared tree. Load the train base with no quantization flags. The LLaMA-Factory export card says not to merge a quantized base. `AutoTokenizer.save_pretrained` writes the tokenizer files into that same directory. Axolotl's merge writes the tokenizer; this PEFT path uses the transformers save for those files. This factory does not write a Python file, does not run it, and does not download the train base.\n\
         \n\
         {snippet}\n\
         \n\
         {next}\
         merge-adapt did not merge and did not write {merged}.\n\
         READY_FOR_LIVE_TEST: no.\n",
        header = header(doc, seat_tag, local_tag, adapter_dir, &merged),
        merged = merged.display(),
        next = next_lines(&convert, &gguf, &seat),
    );
    Ok(MergeAdaptPlan {
        seat_tag: seat_tag.to_string(),
        local_tag: local_tag.to_string(),
        pack_id: doc.pack_id.clone(),
        driver: doc.driver.clone(),
        merge_commands: vec![snippet],
        merged_dir: merged,
        convert_command: convert,
        gguf_convert_command: gguf,
        local_seat_command: seat,
        report,
    })
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

fn export_card_note(prepared_dir: &Path) -> Result<String, ModelError> {
    let path = prepared_dir.join("export.yaml");
    let meta = match std::fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(
                "This prepare has no export.yaml beside the train directory. `llamafactory-cli export` is the LLaMA-Factory merge when that file is the card (https://llamafactory.readthedocs.io/en/latest/getting_started/merge_lora.html).\n\n"
                    .to_string(),
            );
        }
        Err(err) => {
            return Err(ModelError::Other(format!(
                "refuse:merge: {}: {err}",
                path.display()
            )));
        }
    };
    if meta.file_type().is_symlink() {
        return Err(ModelError::Other(format!(
            "refuse:merge: {} is a symlink. enrich does not follow a symlinked export.yaml.",
            path.display()
        )));
    }
    if !meta.is_file() {
        return Err(ModelError::Other(format!(
            "refuse:merge: {} is not a regular file",
            path.display()
        )));
    }
    let text = read_card(&path, "refuse:merge")?;
    refuse_sacred_and_sku("export.yaml", &text)?;
    refuse_raw_secrets(&text).map_err(map_feed)?;
    let export_dir = match yaml_top_scalar(&text, "export_dir")? {
        Some(raw) => resolve_beside_prepare(prepared_dir, &raw, "export_dir")?,
        None => prepared_dir.join("export"),
    };
    let export_line = format!(
        "llamafactory-cli export {}",
        shell_quote(&path.display().to_string())
    );
    Ok(format!(
        "LLaMA-Factory's merge card is `llamafactory-cli export` when you run export.yaml (https://llamafactory.readthedocs.io/en/latest/getting_started/merge_lora.html). This prepare has {path}. That command writes export_dir {export_dir}. This factory does not run it.\n\
         \n\
         {export_line}\n\
         \n",
        path = path.display(),
        export_dir = export_dir.display(),
    ))
}

fn printed_peft_merge(train_base: &str, adapter: &Path, out: &Path) -> String {
    let train = serde_json::to_string(train_base).unwrap_or_else(|_| "\"\"".to_string());
    let adapter = serde_json::to_string(&adapter.display().to_string())
        .unwrap_or_else(|_| "\"\"".to_string());
    let out =
        serde_json::to_string(&out.display().to_string()).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        "from transformers import AutoModelForCausalLM, AutoTokenizer\n\
         from peft import PeftModel\n\
         base_model = AutoModelForCausalLM.from_pretrained({train})\n\
         model = PeftModel.from_pretrained(base_model, {adapter})\n\
         merged_model = model.merge_and_unload()\n\
         merged_model.save_pretrained({out})\n\
         tokenizer = AutoTokenizer.from_pretrained({train})\n\
         tokenizer.save_pretrained({out})"
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

    #[test]
    fn llamafactory_adapter_prints_peft_when_export_yaml_is_absent() {
        let root = tmp("lf-no-export");
        write_prepare(
            &root,
            LLAMAFACTORY_QLORA_ID,
            "train",
            Some("llama3"),
            Some("Qwen/Qwen2.5-0.5B-Instruct"),
            false,
        );
        let adapter = adapter_dir(&root.join("outputs"));
        let plan = plan_merge_adapt(&root, &adapter).unwrap();
        let merged = root.join("merged");
        assert_eq!(plan.merged_dir, merged);
        assert!(
            plan.merge_commands[0].contains("merge_and_unload()"),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            plan.merge_commands[0].contains("save_pretrained("),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            plan.merge_commands[0].contains("PeftModel.from_pretrained"),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            plan.merge_commands[0].contains(&merged.display().to_string()),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            plan.merge_commands[0].contains("Qwen/Qwen2.5-0.5B-Instruct"),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            !plan.report.contains("axolotl merge-lora"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("no export.yaml"), "{}", plan.report);
        assert_eq!(plan.gguf_convert_command, gguf_convert_cli(&root, &merged));
        assert_eq!(plan.local_seat_command, local_seat_cli(&root, &merged));
        assert!(
            plan.report.contains("merge-adapt did not merge"),
            "{}",
            plan.report
        );
        assert!(!merged.exists());
    }

    #[test]
    fn llamafactory_names_export_yaml_and_still_prints_peft() {
        let root = tmp("lf-export");
        write_prepare(
            &root,
            LLAMAFACTORY_LORA_ID,
            "train",
            Some("llama3"),
            Some("Qwen/Qwen2.5-0.5B-Instruct"),
            false,
        );
        let export_dir = root.join("export");
        std::fs::write(
            root.join("export.yaml"),
            format!("export_dir: \"{}\"\n", export_dir.display()),
        )
        .unwrap();
        let adapter = adapter_dir(&root.join("outputs"));
        let plan = plan_merge_adapt(&root, &adapter).unwrap();
        assert!(
            plan.report.contains(&format!(
                "llamafactory-cli export {}",
                root.join("export.yaml").display()
            )),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains(&export_dir.display().to_string()),
            "{}",
            plan.report
        );
        assert_eq!(plan.merged_dir, root.join("merged"));
        assert!(
            plan.merge_commands[0].contains("merge_and_unload()"),
            "{}",
            plan.merge_commands[0]
        );
        assert!(
            !plan.merge_commands[0].contains("llamafactory-cli"),
            "{}",
            plan.merge_commands[0]
        );
        assert!(!root.join("merged").exists());
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
        assert!(err.to_string().contains("refuse:driver"), "{err}");
        assert!(err.to_string().contains("unsloth-qlora"), "{err}");
        assert!(!err.to_string().contains("axolotl merge-lora"), "{err}");

        write_prepare(&root, "mlx-lm-lora", "train", Some("llama3"), None, false);
        let err = plan_merge_adapt(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:driver"), "{err}");

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
}
