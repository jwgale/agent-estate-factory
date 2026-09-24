//! Copy tensors and tokenizer files the merge-export dropped.
//!
//! Qwen3.5 (`Qwen3_5ForConditionalGeneration`) stores one MTP block in
//! `text_config.mtp_num_hidden_layers` and weights named `mtp.*`
//! (`mtp.layers.0.*`, `mtp.fc`, `mtp.norm`, `mtp.pre_fc_norm_embedding`,
//! `mtp.pre_fc_norm_hidden`). llama.cpp `conversion/qwen.py` `_QwenMtpMixin`
//! adds that count to `block_count` and remaps `mtp.layers.{i}` onto
//! `model.layers.{num_hidden_layers + i}`, which the loader reads as `blk.32`.
//! LLaMA-Factory export keeps the config key and drops the tensors.

use anyhow::{bail, Result};
use safetensors::tensor::{Dtype, TensorInfo, TensorView};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub const REPAIR_SHARD: &str = "model-export-repair.safetensors";
const INDEX_NAME: &str = "model.safetensors.index.json";
/// One tensor larger than this is refused before the buffer is allocated.
const MAX_TENSOR_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairReport {
    pub tensors: Vec<String>,
    pub files: Vec<String>,
}

impl RepairReport {
    pub fn summary(&self) -> String {
        if self.tensors.is_empty() && self.files.is_empty() {
            return "export-repair: nothing missing".to_string();
        }
        format!(
            "export-repair: copied {} tensor(s) and {} file(s)",
            self.tensors.len(),
            self.files.len()
        )
    }
}

/// Names the repair would copy. Does not write.
pub fn preview_repair(base: &Path, merged: &Path) -> Result<RepairReport> {
    if !merged.is_dir() {
        return Ok(RepairReport {
            tensors: Vec::new(),
            files: Vec::new(),
        });
    }
    let base_tensors = read_catalog(base)?;
    let merged_tensors = read_catalog(merged)?;
    let mut missing: Vec<String> = base_tensors
        .keys()
        .filter(|name| !merged_tensors.contains_key(*name))
        .cloned()
        .collect();
    missing.sort();
    let files = missing_sidecar_files(base, merged);
    Ok(RepairReport {
        tensors: missing,
        files,
    })
}

/// Copy missing tensors into an extra shard and missing tokenizer files.
/// Refuses when `config.json` still declares layers the repaired dir lacks.
pub fn repair_export(base: &Path, merged: &Path) -> Result<RepairReport> {
    if !base.is_dir() {
        bail!(
            "refuse:classify-journey: export-repair base snapshot {} is not a directory",
            base.display()
        );
    }
    if !merged.is_dir() {
        bail!(
            "refuse:classify-journey: export-repair merged dir {} is not a directory",
            merged.display()
        );
    }
    let report = preview_repair(base, merged)?;
    refuse_tied_lm_head(merged, &report.tensors)?;
    let base_count = read_catalog(base)?.len();
    refuse_implausible(&report.tensors, base_count)?;
    if report.tensors.is_empty() && report.files.is_empty() {
        verify_config_matches_tensors(merged)?;
        return Ok(report);
    }
    if !report.tensors.is_empty() {
        let catalog = read_catalog(base)?;
        write_repair_shard(merged, &catalog, &report.tensors)?;
    }
    for name in &report.files {
        let from = base.join(name);
        let to = merged.join(name);
        fs::copy(&from, &to).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-journey: export-repair cannot copy {} -> {}: {err}",
                from.display(),
                to.display()
            )
        })?;
    }
    verify_config_matches_tensors(merged)?;
    Ok(report)
}

pub fn repair_fingerprint(pipeline: &str, report: &RepairReport) -> String {
    let mut names = report.tensors.clone();
    names.extend(report.files.iter().cloned());
    let body = names.join("\n");
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(format!("export-repair\n{pipeline}\n{body}").as_bytes());
    format!("{:x}", hasher.finalize())
}

fn missing_sidecar_files(base: &Path, merged: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(base) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| is_sidecar(name) && !merged.join(name).is_file())
        .collect();
    names.sort();
    names
}

fn is_sidecar(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".safetensors") || lower == INDEX_NAME || lower == "config.json" {
        return false;
    }
    lower.contains("tokenizer")
        || lower.contains("processor")
        || lower == "vocab.json"
        || lower == "merges.txt"
        || lower == "special_tokens_map.json"
        || lower == "added_tokens.json"
        || lower == "chat_template.jinja"
}

struct LocatedTensor {
    dtype: Dtype,
    shape: Vec<usize>,
    data_start: u64,
    data_end: u64,
    /// Byte length of the JSON header. Tensor bytes begin at 8 + header_len.
    header_len: u64,
    shard: PathBuf,
}

fn mtp_prefixed(name: &str) -> bool {
    name.starts_with("mtp.")
        || name.starts_with("model.mtp.")
        || name.starts_with("model.language_model.mtp.")
}

/// An all-MTP gap is the patch this step exists for. Any other missing name
/// must stay within 32 tensors and 2% of the base catalog.
fn refuse_implausible(missing: &[String], base_count: usize) -> Result<()> {
    if missing.is_empty() || missing.iter().all(|name| mtp_prefixed(name)) {
        return Ok(());
    }
    let over_count = missing.len() > 32;
    let over_ratio =
        base_count == 0 || missing.len().saturating_mul(100) > base_count.saturating_mul(2);
    if !over_count && !over_ratio {
        return Ok(());
    }
    let sample = missing
        .iter()
        .take(10)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    bail!(
        "refuse:classify-journey: export-repair missing {} tensors is not an MTP patch (allow every name starting with mtp., model.mtp., or model.language_model.mtp.; otherwise at most 32 tensors and 2% of {base_count} base tensors). Sample: {sample}",
        missing.len()
    );
}

/// Tensor names and locations from safetensors headers only. Does not read tensor bytes.
fn read_catalog(dir: &Path) -> Result<BTreeMap<String, LocatedTensor>> {
    let mut out = BTreeMap::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    for path in shard_paths(dir)? {
        let (header_len, tensors) = read_header(&path)?;
        for (name, info) in tensors {
            if out.contains_key(&name) {
                bail!(
                    "refuse:classify-journey: export-repair duplicate tensor {name} in {}",
                    dir.display()
                );
            }
            let (data_start, data_end) = info.data_offsets;
            let data_start = data_start as u64;
            let data_end = data_end as u64;
            if data_start > data_end {
                bail!(
                    "refuse:classify-journey: export-repair tensor {name} in {} has inverted data_offsets",
                    path.display()
                );
            }
            let file_len = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
            let end_pos = 8u64
                .checked_add(header_len)
                .and_then(|value| value.checked_add(data_end))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "refuse:classify-journey: export-repair tensor {name} offset overflow"
                    )
                })?;
            if end_pos > file_len {
                bail!(
                    "refuse:classify-journey: export-repair tensor {name} in {} extends past the file",
                    path.display()
                );
            }
            out.insert(
                name,
                LocatedTensor {
                    dtype: info.dtype,
                    shape: info.shape,
                    data_start,
                    data_end,
                    header_len,
                    shard: path.clone(),
                },
            );
        }
    }
    Ok(out)
}

fn read_header(path: &Path) -> Result<(u64, BTreeMap<String, TensorInfo>)> {
    let mut file = File::open(path).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-journey: export-repair cannot read {}: {err}",
            path.display()
        )
    })?;
    let mut len_buf = [0u8; 8];
    file.read_exact(&mut len_buf).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-journey: export-repair {} is not safetensors: {err}",
            path.display()
        )
    })?;
    let header_len = u64::from_le_bytes(len_buf);
    if header_len > 100_000_000 {
        bail!(
            "refuse:classify-journey: export-repair {} header is too large",
            path.display()
        );
    }
    let mut header = vec![0u8; header_len as usize];
    file.read_exact(&mut header).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-journey: export-repair {} header is truncated: {err}",
            path.display()
        )
    })?;
    let value: Value = serde_json::from_slice(&header).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-journey: export-repair {} header is not JSON: {err}",
            path.display()
        )
    })?;
    let obj = value.as_object().ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:classify-journey: export-repair {} header is not an object",
            path.display()
        )
    })?;
    let mut tensors = BTreeMap::new();
    for (name, info) in obj {
        if name == "__metadata__" {
            continue;
        }
        let parsed: TensorInfo = serde_json::from_value(info.clone()).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-journey: export-repair tensor {name} in {} has a bad header: {err}",
                path.display()
            )
        })?;
        tensors.insert(name.clone(), parsed);
    }
    Ok((header_len, tensors))
}

fn sum_nbytes(dir: &Path, weight_map: &BTreeMap<String, Value>) -> Result<u64> {
    let mut headers: BTreeMap<String, BTreeMap<String, TensorInfo>> = BTreeMap::new();
    let mut total = 0u64;
    for (name, file) in weight_map {
        let file = file.as_str().unwrap_or("");
        if !headers.contains_key(file) {
            let (_len, parsed) = read_header(&dir.join(file))?;
            headers.insert(file.to_string(), parsed);
        }
        let info = headers
            .get(file)
            .and_then(|map| map.get(name))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "refuse:classify-journey: export-repair tensor {name} is missing from {file}"
                )
            })?;
        let (start, end) = info.data_offsets;
        if start > end {
            bail!(
                "refuse:classify-journey: export-repair tensor {name} has inverted data_offsets"
            );
        }
        total += (end - start) as u64;
    }
    Ok(total)
}

fn read_tensor_bytes(tensor: &LocatedTensor) -> Result<Vec<u8>> {
    let len = tensor
        .data_end
        .checked_sub(tensor.data_start)
        .ok_or_else(|| {
            anyhow::anyhow!("refuse:classify-journey: export-repair bad data_offsets")
        })?;
    if len > MAX_TENSOR_BYTES {
        bail!(
            "refuse:classify-journey: export-repair tensor in {} claims {len} bytes",
            tensor.shard.display()
        );
    }
    let mut file = File::open(&tensor.shard)?;
    let offset = 8u64
        .checked_add(tensor.header_len)
        .and_then(|value| value.checked_add(tensor.data_start))
        .ok_or_else(|| {
            anyhow::anyhow!("refuse:classify-journey: export-repair seek offset overflow")
        })?;
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; len as usize];
    file.read_exact(&mut buf).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-journey: export-repair cannot read tensor bytes in {}: {err}",
            tensor.shard.display()
        )
    })?;
    Ok(buf)
}

fn shard_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    let index = dir.join(INDEX_NAME);
    if index.is_file() {
        let text = fs::read_to_string(&index).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-journey: export-repair cannot read {}: {err}",
                index.display()
            )
        })?;
        let value: Value = serde_json::from_str(&text).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-journey: export-repair {} is not JSON: {err}",
                index.display()
            )
        })?;
        let map = value
            .get("weight_map")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "refuse:classify-journey: export-repair {} has no weight_map",
                    index.display()
                )
            })?;
        let mut names: Vec<&str> = map.values().filter_map(Value::as_str).collect();
        names.sort_unstable();
        names.dedup();
        let paths = names
            .into_iter()
            .map(|name| dir.join(name))
            .filter(|path| path.is_file())
            .collect();
        return Ok(paths);
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(Vec::new());
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            path.is_file() && name.ends_with(".safetensors")
        })
        .collect();
    paths.sort();
    Ok(paths)
}

fn write_repair_shard(
    merged: &Path,
    base: &BTreeMap<String, LocatedTensor>,
    names: &[String],
) -> Result<()> {
    let mut views = Vec::new();
    let mut owned = Vec::new();
    for name in names {
        let Some(tensor) = base.get(name) else {
            bail!(
                "refuse:classify-journey: export-repair tensor {name} disappeared from the base snapshot"
            );
        };
        let data = read_tensor_bytes(tensor)?;
        owned.push((name.clone(), tensor.dtype, tensor.shape.clone(), data));
    }
    for (name, dtype, shape, data) in &owned {
        let view = TensorView::new(*dtype, shape.clone(), data).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-journey: export-repair cannot view tensor {name}: {err}"
            )
        })?;
        views.push((name.clone(), view));
    }
    let bytes = safetensors::serialize(views, &None).map_err(|err| {
        anyhow::anyhow!("refuse:classify-journey: export-repair cannot write shard: {err}")
    })?;
    let shard = merged.join(REPAIR_SHARD);
    fs::write(&shard, bytes).map_err(|err| {
        anyhow::anyhow!(
            "refuse:classify-journey: export-repair cannot write {}: {err}",
            shard.display()
        )
    })?;
    update_index(merged, names)?;
    Ok(())
}

fn update_index(merged: &Path, names: &[String]) -> Result<()> {
    let index_path = merged.join(INDEX_NAME);
    let mut weight_map = BTreeMap::new();
    let mut metadata = json!({});
    if index_path.is_file() {
        let existing: Value = serde_json::from_str(&fs::read_to_string(&index_path)?)?;
        if let Some(meta) = existing.get("metadata") {
            metadata = meta.clone();
        }
        if let Some(map) = existing.get("weight_map").and_then(Value::as_object) {
            for (name, file) in map {
                if file.as_str() == Some(REPAIR_SHARD) {
                    continue;
                }
                if names.iter().any(|added| added == name) {
                    bail!(
                        "refuse:classify-journey: export-repair refused to overwrite tensor {name}"
                    );
                }
                weight_map.insert(name.clone(), file.clone());
            }
        }
    } else {
        for (name, tensor) in read_catalog(merged)? {
            if tensor.shard.file_name().and_then(|n| n.to_str()) == Some(REPAIR_SHARD) {
                continue;
            }
            let file = tensor
                .shard
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("model.safetensors")
                .to_string();
            weight_map.insert(name, Value::String(file));
        }
    }
    for name in names {
        weight_map.insert(name.clone(), Value::String(REPAIR_SHARD.to_string()));
    }
    let total = sum_nbytes(merged, &weight_map)?;
    let mut meta = metadata.as_object().cloned().unwrap_or_default();
    meta.insert("total_size".into(), json!(total));
    let report = json!({
        "metadata": meta,
        "weight_map": weight_map,
    });
    fs::write(
        index_path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;
    Ok(())
}

fn verify_config_matches_tensors(merged: &Path) -> Result<()> {
    let config_path = merged.join("config.json");
    if !config_path.is_file() {
        bail!(
            "refuse:classify-journey: export-repair {} has no config.json",
            merged.display()
        );
    }
    let config: Value = serde_json::from_str(&fs::read_to_string(&config_path)?)?;
    let (layers, mtp) = declared_counts(&config);
    let tensors = read_catalog(merged)?;
    let names: Vec<&str> = tensors.keys().map(String::as_str).collect();
    if let Some(layers) = layers {
        for index in 0..layers {
            if !names.iter().any(|name| trunk_layer(name, index)) {
                bail!(
                    "refuse:classify-journey: export-repair cannot repair config num_hidden_layers={layers}: no tensor for layer {index} in {}",
                    merged.display()
                );
            }
        }
    }
    if let Some(mtp) = mtp {
        if mtp > 0 {
            for index in 0..mtp {
                if !names.iter().any(|name| mtp_layer(name, index)) {
                    bail!(
                        "refuse:classify-journey: export-repair cannot repair config mtp_num_hidden_layers={mtp}: mtp.layers.{index} is missing from the base snapshot and the merged export"
                    );
                }
            }
        }
    }
    Ok(())
}

fn is_lm_head(name: &str) -> bool {
    name.split('.').any(|part| part.starts_with("lm_head"))
}

fn refuse_tied_lm_head(merged: &Path, names: &[String]) -> Result<()> {
    let tied = names.iter().any(|name| is_lm_head(name));
    if !tied {
        return Ok(());
    }
    let config_path = merged.join("config.json");
    let text = fs::read_to_string(&config_path).unwrap_or_default();
    let config: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let flag = |value: &Value| value.get("tie_word_embeddings").and_then(Value::as_bool) == Some(true);
    if flag(&config) || config.get("text_config").map(flag).unwrap_or(false) {
        bail!(
            "refuse:classify-journey: export-repair refuses to copy lm_head tensors because {} has tie_word_embeddings true",
            config_path.display()
        );
    }
    Ok(())
}

fn declared_counts(config: &Value) -> (Option<u64>, Option<u64>) {
    let text = config.get("text_config").unwrap_or(config);
    let layers = text
        .get("num_hidden_layers")
        .and_then(Value::as_u64)
        .or_else(|| config.get("num_hidden_layers").and_then(Value::as_u64));
    let mtp = text
        .get("mtp_num_hidden_layers")
        .and_then(Value::as_u64)
        .or_else(|| config.get("mtp_num_hidden_layers").and_then(Value::as_u64));
    (layers, mtp)
}

fn trunk_layer(name: &str, index: u64) -> bool {
    if name.starts_with("mtp.") || name.contains(".mtp.") {
        return false;
    }
    layer_index(name) == Some(index)
}

fn mtp_layer(name: &str, index: u64) -> bool {
    (name.starts_with("mtp.") || name.contains(".mtp.")) && layer_index(name) == Some(index)
}

fn layer_index(name: &str) -> Option<u64> {
    let marker = ".layers.";
    let start = name.find(marker)? + marker.len();
    let rest = &name[start..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let next = rest.as_bytes().get(digits.len()).copied();
    if next != Some(b'.') {
        return None;
    }
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use safetensors::tensor::TensorView;

    fn write_shard(dir: &Path, file: &str, tensors: &[(&str, &[u8])]) {
        fs::create_dir_all(dir).unwrap();
        let mut views = Vec::new();
        for (name, data) in tensors {
            views.push((
                (*name).to_string(),
                TensorView::new(Dtype::F32, vec![data.len() / 4], data).unwrap(),
            ));
        }
        let bytes = safetensors::serialize(views, &None).unwrap();
        fs::write(dir.join(file), bytes).unwrap();
    }

    fn qwen_config() -> String {
        serde_json::json!({
            "architectures": ["Qwen3_5ForConditionalGeneration"],
            "model_type": "qwen3_5",
            "text_config": {
                "model_type": "qwen3_5_text",
                "num_hidden_layers": 1,
                "mtp_num_hidden_layers": 1
            }
        })
        .to_string()
    }

    #[test]
    fn copies_missing_mtp_tensors_and_tokenizer_and_updates_the_index() {
        let root = std::env::temp_dir().join(format!("export-repair-mtp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let base = root.join("base");
        let merged = root.join("merged");
        let trunk = [1u8, 0, 0, 0];
        let mtp = [2u8, 0, 0, 0];
        let fc = [3u8, 0, 0, 0];
        write_shard(
            &base,
            "model.safetensors",
            &[
                (
                    "model.language_model.layers.0.input_layernorm.weight",
                    &trunk,
                ),
                ("mtp.layers.0.input_layernorm.weight", &mtp),
                ("mtp.fc.weight", &fc),
                ("mtp.norm.weight", &fc),
                ("mtp.pre_fc_norm_embedding.weight", &fc),
                ("mtp.pre_fc_norm_hidden.weight", &fc),
            ],
        );
        fs::write(base.join("config.json"), qwen_config()).unwrap();
        fs::write(base.join("tokenizer.json"), "{\"base\":true}").unwrap();
        fs::write(base.join("processor_config.json"), "{}").unwrap();
        write_shard(
            &merged,
            "model.safetensors",
            &[(
                "model.language_model.layers.0.input_layernorm.weight",
                &trunk,
            )],
        );
        fs::write(merged.join("config.json"), qwen_config()).unwrap();
        let report = repair_export(&base, &merged).unwrap();
        assert!(report
            .tensors
            .iter()
            .any(|n| n == "mtp.layers.0.input_layernorm.weight"));
        assert!(report.tensors.iter().any(|n| n == "mtp.fc.weight"));
        assert_eq!(
            report.files,
            vec![
                "processor_config.json".to_string(),
                "tokenizer.json".to_string()
            ]
        );
        assert!(merged.join(REPAIR_SHARD).is_file());
        assert_eq!(
            fs::read_to_string(merged.join("tokenizer.json")).unwrap(),
            "{\"base\":true}"
        );
        let index: Value =
            serde_json::from_str(&fs::read_to_string(merged.join(INDEX_NAME)).unwrap()).unwrap();
        assert_eq!(
            index["weight_map"]["mtp.layers.0.input_layernorm.weight"],
            REPAIR_SHARD
        );
        assert_eq!(
            index["weight_map"]["model.language_model.layers.0.input_layernorm.weight"],
            "model.safetensors"
        );
        let again = repair_export(&base, &merged).unwrap();
        assert!(
            again.tensors.is_empty() && again.files.is_empty(),
            "{again:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn nothing_missing_is_a_noop() {
        let root = std::env::temp_dir().join(format!("export-repair-noop-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let base = root.join("base");
        let merged = root.join("merged");
        let weight = [4u8, 0, 0, 0];
        write_shard(
            &base,
            "model.safetensors",
            &[("model.layers.0.input_layernorm.weight", &weight)],
        );
        let deepseek =
            r#"{"architectures":["Qwen2ForCausalLM"],"model_type":"qwen2","num_hidden_layers":1}"#;
        fs::write(base.join("config.json"), deepseek).unwrap();
        fs::write(base.join("tokenizer.json"), "tok").unwrap();
        write_shard(
            &merged,
            "model.safetensors",
            &[("model.layers.0.input_layernorm.weight", &weight)],
        );
        fs::write(merged.join("config.json"), deepseek).unwrap();
        fs::write(merged.join("tokenizer.json"), "tok").unwrap();
        let before = fs::read(merged.join("model.safetensors")).unwrap();
        let report = repair_export(&base, &merged).unwrap();
        assert!(report.tensors.is_empty() && report.files.is_empty());
        assert_eq!(
            report.summary(),
            "export-repair: nothing missing",
            "DeepSeek-R1-Distill keeps the same tensors and is a no-op"
        );
        assert!(!deepseek.contains("mtp_num_hidden_layers"));
        assert!(!merged.join(REPAIR_SHARD).exists());
        assert!(!merged.join(INDEX_NAME).exists());
        assert_eq!(fs::read(merged.join("model.safetensors")).unwrap(), before);
        let _ = fs::remove_dir_all(&root);
    }

    /// Live Qwen3.5-4B export: base-hf weight_map has 738 tensors, 15 of them
    /// `mtp.*`. The merged export keeps `mtp_num_hidden_layers: 1` and has the
    /// other 723 tensors, none named `mtp`.
    const LIVE_MTP: [&str; 15] = [
        "mtp.fc.weight",
        "mtp.norm.weight",
        "mtp.pre_fc_norm_embedding.weight",
        "mtp.pre_fc_norm_hidden.weight",
        "mtp.layers.0.input_layernorm.weight",
        "mtp.layers.0.post_attention_layernorm.weight",
        "mtp.layers.0.mlp.down_proj.weight",
        "mtp.layers.0.mlp.gate_proj.weight",
        "mtp.layers.0.mlp.up_proj.weight",
        "mtp.layers.0.self_attn.q_proj.weight",
        "mtp.layers.0.self_attn.k_proj.weight",
        "mtp.layers.0.self_attn.v_proj.weight",
        "mtp.layers.0.self_attn.o_proj.weight",
        "mtp.layers.0.self_attn.q_norm.weight",
        "mtp.layers.0.self_attn.k_norm.weight",
    ];

    fn live_config() -> String {
        serde_json::json!({
            "architectures": ["Qwen3_5ForConditionalGeneration"],
            "model_type": "qwen3_5",
            "text_config": {
                "model_type": "qwen3_5_text",
                "num_hidden_layers": 32,
                "mtp_num_hidden_layers": 1,
                "mtp_use_dedicated_embeddings": false
            }
        })
        .to_string()
    }

    fn write_sharded(dir: &Path, names: &[String]) {
        fs::create_dir_all(dir).unwrap();
        let f32 = [9u8, 0, 0, 0];
        let bf16 = [0x80u8, 0x3f];
        let mid = names.len() / 2;
        let shards = [
            (
                "model.safetensors-00001-of-00002.safetensors",
                &names[..mid],
            ),
            (
                "model.safetensors-00002-of-00002.safetensors",
                &names[mid..],
            ),
        ];
        let mut weight_map = serde_json::Map::new();
        for (file, group) in shards {
            let owned: Vec<(String, Dtype, Vec<u8>)> = group
                .iter()
                .map(|name| {
                    if name == "mtp.layers.0.input_layernorm.weight" {
                        (name.clone(), Dtype::BF16, bf16.to_vec())
                    } else {
                        (name.clone(), Dtype::F32, f32.to_vec())
                    }
                })
                .collect();
            let views: Vec<_> = owned
                .iter()
                .map(|(name, dtype, data)| {
                    (
                        name.clone(),
                        TensorView::new(*dtype, vec![1], data).unwrap(),
                    )
                })
                .collect();
            let bytes = safetensors::serialize(views, &None).unwrap();
            fs::write(dir.join(file), bytes).unwrap();
            for name in group {
                weight_map.insert(name.clone(), Value::String(file.to_string()));
            }
        }
        let index = serde_json::json!({
            "metadata": {"total_size": names.len() * 4},
            "weight_map": weight_map
        });
        fs::write(
            dir.join(INDEX_NAME),
            format!("{}\n", serde_json::to_string_pretty(&index).unwrap()),
        )
        .unwrap();
    }

    #[test]
    fn copies_the_fifteen_live_mtp_tensors_into_a_sharded_export() {
        let root =
            std::env::temp_dir().join(format!("export-repair-live-738-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut trunk = Vec::new();
        for layer in 0..32 {
            for n in 0..22 {
                trunk.push(format!(
                    "model.language_model.layers.{layer}.block.{n}.weight"
                ));
            }
        }
        trunk.push("model.language_model.embed_tokens.weight".into());
        for n in 0..18 {
            trunk.push(format!("model.language_model.extra.{n}.weight"));
        }
        assert_eq!(trunk.len(), 723, "merged export weight_map");
        let mut base_names = trunk.clone();
        base_names.extend(LIVE_MTP.iter().map(|name| (*name).to_string()));
        assert_eq!(base_names.len(), 738);
        assert_eq!(
            base_names
                .iter()
                .filter(|name| name.contains("mtp"))
                .count(),
            15
        );
        let base = root.join("base-hf");
        let merged = root.join("export");
        write_sharded(&base, &base_names);
        write_sharded(&merged, &trunk);
        fs::write(base.join("config.json"), live_config()).unwrap();
        fs::write(merged.join("config.json"), live_config()).unwrap();
        let shard_a =
            fs::read(merged.join("model.safetensors-00001-of-00002.safetensors")).unwrap();
        let shard_b =
            fs::read(merged.join("model.safetensors-00002-of-00002.safetensors")).unwrap();
        let before: Value =
            serde_json::from_str(&fs::read_to_string(merged.join(INDEX_NAME)).unwrap()).unwrap();
        assert_eq!(before["weight_map"].as_object().unwrap().len(), 723);
        assert!(before["weight_map"]
            .as_object()
            .unwrap()
            .keys()
            .all(|name| !name.contains("mtp")));

        let report = repair_export(&base, &merged).unwrap();
        assert_eq!(report.tensors.len(), 15, "{:?}", report.tensors);
        for name in LIVE_MTP {
            assert!(
                report.tensors.iter().any(|copied| copied == name),
                "missing {name}"
            );
        }
        assert!(report.summary().contains("15 tensor"));
        let after: Value =
            serde_json::from_str(&fs::read_to_string(merged.join(INDEX_NAME)).unwrap()).unwrap();
        let map = after["weight_map"].as_object().unwrap();
        assert_eq!(map.len(), 738);
        assert_eq!(map.keys().filter(|name| name.contains("mtp")).count(), 15);
        for name in LIVE_MTP {
            assert_eq!(map[name], REPAIR_SHARD, "{name}");
        }
        for name in &trunk {
            assert_eq!(
                map[name], before["weight_map"][name],
                "trunk tensor {name} was overwritten"
            );
        }
        assert_eq!(
            fs::read(merged.join("model.safetensors-00001-of-00002.safetensors")).unwrap(),
            shard_a
        );
        assert_eq!(
            fs::read(merged.join("model.safetensors-00002-of-00002.safetensors")).unwrap(),
            shard_b
        );
        let config: Value =
            serde_json::from_str(&fs::read_to_string(merged.join("config.json")).unwrap()).unwrap();
        assert_eq!(config["text_config"]["mtp_num_hidden_layers"], 1);
        assert_eq!(config["text_config"]["mtp_use_dedicated_embeddings"], false);
        assert_eq!(config["text_config"]["num_hidden_layers"], 32);
        assert_eq!(after["metadata"]["total_size"], 723 * 4 + 14 * 4 + 2);
        let (_len, repaired) = read_header(&merged.join(REPAIR_SHARD)).unwrap();
        assert_eq!(
            repaired["mtp.layers.0.input_layernorm.weight"].dtype,
            Dtype::BF16
        );
        let bf = read_tensor_bytes(&LocatedTensor {
            dtype: Dtype::BF16,
            shape: vec![1],
            data_start: repaired["mtp.layers.0.input_layernorm.weight"]
                .data_offsets
                .0 as u64,
            data_end: repaired["mtp.layers.0.input_layernorm.weight"]
                .data_offsets
                .1 as u64,
            header_len: _len,
            shard: merged.join(REPAIR_SHARD),
        })
        .unwrap();
        assert_eq!(bf, [0x80, 0x3f]);
        let again = repair_export(&base, &merged).unwrap();
        assert!(again.tensors.is_empty(), "{again:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn refuses_a_prefix_mismatch_and_writes_nothing() {
        let root =
            std::env::temp_dir().join(format!("export-repair-prefix-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let base = root.join("base");
        let merged = root.join("merged");
        let data = [7u8, 0, 0, 0];
        let mut base_names = Vec::new();
        let mut merged_names = Vec::new();
        for index in 0..40 {
            base_names.push(format!("model.language_model.layers.{index}.weight"));
            merged_names.push(format!("model.layers.{index}.weight"));
        }
        let base_tensors: Vec<(&str, &[u8])> = base_names
            .iter()
            .map(|name| (name.as_str(), data.as_slice()))
            .collect();
        let merged_tensors: Vec<(&str, &[u8])> = merged_names
            .iter()
            .map(|name| (name.as_str(), data.as_slice()))
            .collect();
        write_shard(&base, "model.safetensors", &base_tensors);
        write_shard(&merged, "model.safetensors", &merged_tensors);
        fs::write(base.join("config.json"), r#"{"num_hidden_layers":40}"#).unwrap();
        fs::write(merged.join("config.json"), r#"{"num_hidden_layers":40}"#).unwrap();
        let before = fs::read(merged.join("model.safetensors")).unwrap();
        let err = repair_export(&base, &merged).unwrap_err().to_string();
        assert!(err.contains("not an MTP patch"), "{err}");
        assert!(err.contains("40"), "{err}");
        assert!(err.contains("model.language_model.layers."), "{err}");
        assert_eq!(fs::read(merged.join("model.safetensors")).unwrap(), before);
        assert!(!merged.join(REPAIR_SHARD).exists());
        assert!(!merged.join(INDEX_NAME).exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn refuses_when_declared_mtp_is_absent_from_both() {
        let root = std::env::temp_dir().join(format!("export-repair-gap-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let base = root.join("base");
        let merged = root.join("merged");
        let weight = [5u8, 0, 0, 0];
        write_shard(
            &base,
            "model.safetensors",
            &[(
                "model.language_model.layers.0.input_layernorm.weight",
                &weight,
            )],
        );
        fs::write(base.join("config.json"), qwen_config()).unwrap();
        write_shard(
            &merged,
            "model.safetensors",
            &[(
                "model.language_model.layers.0.input_layernorm.weight",
                &weight,
            )],
        );
        fs::write(merged.join("config.json"), qwen_config()).unwrap();
        let err = repair_export(&base, &merged).unwrap_err().to_string();
        assert!(err.contains("mtp_num_hidden_layers"), "{err}");
        assert!(err.contains("mtp.layers.0"), "{err}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn refuses_a_header_larger_than_100mb() {
        let dir = std::env::temp_dir().join(format!("repair-huge-header-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.safetensors");
        let mut file = File::create(&path).unwrap();
        std::io::Write::write_all(&mut file, &100_000_001u64.to_le_bytes()).unwrap();
        let err = read_header(&path).unwrap_err().to_string();
        assert!(err.contains("header is too large"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_inverted_offsets_and_a_tied_lm_head() {
        let root = std::env::temp_dir().join(format!("repair-offsets-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let base = root.join("base");
        let merged = root.join("merged");
        let weight = [0u8; 8];
        write_shard(&base, "model.safetensors", &[("lm_head.weight", &weight)]);
        write_shard(&merged, "model.safetensors", &[("model.embed_tokens.weight", &weight)]);
        fs::write(
            merged.join("config.json"),
            serde_json::json!({"tie_word_embeddings": true}).to_string(),
        )
        .unwrap();
        let err = repair_export(&base, &merged).unwrap_err().to_string();
        assert!(err.contains("tie_word_embeddings"), "{err}");
        assert!(err.contains("lm_head"), "{err}");
        assert!(!merged.join(REPAIR_SHARD).exists());

        let bytes = fs::read(base.join("model.safetensors")).unwrap();
        let header_len = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
        let header = std::str::from_utf8(&bytes[8..8 + header_len]).unwrap();
        let flipped = header.replace("\"data_offsets\":[0,8]", "\"data_offsets\":[8,0]");
        assert_ne!(flipped, header);
        let mut bad = Vec::new();
        bad.extend_from_slice(&(flipped.len() as u64).to_le_bytes());
        bad.extend_from_slice(flipped.as_bytes());
        bad.extend_from_slice(&bytes[8 + header_len..]);
        let bad_path = root.join("bad.safetensors");
        fs::write(&bad_path, bad).unwrap();
        let err = match read_catalog(&root) {
            Err(err) => err.to_string(),
            Ok(_) => panic!("inverted offsets should be refused"),
        };
        assert!(
            err.contains("inverted") || err.contains("extends past"),
            "{err}"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
