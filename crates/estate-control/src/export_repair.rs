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
use safetensors::tensor::{Dtype, SafeTensors, TensorView};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const REPAIR_SHARD: &str = "model-export-repair.safetensors";
const INDEX_NAME: &str = "model.safetensors.index.json";

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
    let base_tensors = read_tensors(base)?;
    let merged_tensors = read_tensors(merged)?;
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
    if report.tensors.is_empty() && report.files.is_empty() {
        verify_config_matches_tensors(merged)?;
        return Ok(report);
    }
    if !report.tensors.is_empty() {
        let base_tensors = read_tensors(base)?;
        write_repair_shard(merged, &base_tensors, &report.tensors)?;
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

struct StoredTensor {
    dtype: Dtype,
    shape: Vec<usize>,
    data: Vec<u8>,
}

fn read_tensors(dir: &Path) -> Result<BTreeMap<String, StoredTensor>> {
    let mut out = BTreeMap::new();
    for path in shard_paths(dir)? {
        let bytes = fs::read(&path).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-journey: export-repair cannot read {}: {err}",
                path.display()
            )
        })?;
        let tensors = SafeTensors::deserialize(&bytes).map_err(|err| {
            anyhow::anyhow!(
                "refuse:classify-journey: export-repair {} is not safetensors: {err}",
                path.display()
            )
        })?;
        for name in tensors.names() {
            let view = tensors.tensor(name).map_err(|err| {
                anyhow::anyhow!(
                    "refuse:classify-journey: export-repair cannot read tensor {name} in {}: {err}",
                    path.display()
                )
            })?;
            if out.contains_key(name) {
                bail!(
                    "refuse:classify-journey: export-repair duplicate tensor {name} in {}",
                    dir.display()
                );
            }
            out.insert(
                name.to_string(),
                StoredTensor {
                    dtype: view.dtype(),
                    shape: view.shape().to_vec(),
                    data: view.data().to_vec(),
                },
            );
        }
    }
    Ok(out)
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
    base: &BTreeMap<String, StoredTensor>,
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
        owned.push((
            name.clone(),
            tensor.dtype,
            tensor.shape.clone(),
            tensor.data.clone(),
        ));
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
        for path in shard_paths(merged)? {
            let bytes = fs::read(&path)?;
            let tensors = SafeTensors::deserialize(&bytes).map_err(|err| {
                anyhow::anyhow!(
                    "refuse:classify-journey: export-repair {} is not safetensors: {err}",
                    path.display()
                )
            })?;
            let file = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("model.safetensors")
                .to_string();
            for name in tensors.names() {
                weight_map.insert(name.to_string(), Value::String(file.clone()));
            }
        }
    }
    for name in names {
        weight_map.insert(name.clone(), Value::String(REPAIR_SHARD.to_string()));
    }
    let report = json!({
        "metadata": metadata,
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
    let tensors = read_tensors(merged)?;
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
        fs::write(base.join("config.json"), r#"{"num_hidden_layers":1}"#).unwrap();
        fs::write(base.join("tokenizer.json"), "tok").unwrap();
        write_shard(
            &merged,
            "model.safetensors",
            &[("model.layers.0.input_layernorm.weight", &weight)],
        );
        fs::write(merged.join("config.json"), r#"{"num_hidden_layers":1}"#).unwrap();
        fs::write(merged.join("tokenizer.json"), "tok").unwrap();
        let before = fs::read(merged.join("model.safetensors")).unwrap();
        let report = repair_export(&base, &merged).unwrap();
        assert!(report.tensors.is_empty() && report.files.is_empty());
        assert_eq!(report.summary(), "export-repair: nothing missing");
        assert!(!merged.join(REPAIR_SHARD).exists());
        assert!(!merged.join(INDEX_NAME).exists());
        assert_eq!(fs::read(merged.join("model.safetensors")).unwrap(), before);
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
}
