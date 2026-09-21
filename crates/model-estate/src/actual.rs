use crate::error::ModelError;
use estate_schema::{estate_hash, Estate};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelActual {
    pub desired_hash: String,
    pub bindings: Vec<BindingActual>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BindingActual {
    pub id: String,
    pub class: String,
    pub driver: String,
    pub wired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelDrift {
    pub in_sync: bool,
    pub missing: Vec<String>,
    pub wired_mismatch: Vec<String>,
    pub notes: Vec<String>,
}

pub fn record_bindings(estate: &Estate, state_dir: &Path) -> Result<ModelActual, ModelError> {
    std::fs::create_dir_all(state_dir).map_err(|e| ModelError::Other(e.to_string()))?;
    let actual = ModelActual {
        desired_hash: estate_hash(estate),
        bindings: estate
            .model_bindings
            .iter()
            .map(|b| BindingActual {
                id: b.id.clone(),
                class: b.class.as_str().to_string(),
                driver: b.driver.clone(),
                wired: b.wired,
            })
            .collect(),
    };
    let path = state_dir.join("model-actual.json");
    std::fs::write(
        path,
        serde_json::to_string_pretty(&actual).unwrap_or_default(),
    )
    .map_err(|e| ModelError::Other(e.to_string()))?;
    Ok(actual)
}

pub fn drift_bindings(estate: &Estate, state_dir: &Path) -> Result<ModelDrift, ModelError> {
    let path = state_dir.join("model-actual.json");
    if !path.exists() {
        return Ok(ModelDrift {
            in_sync: false,
            missing: estate.model_bindings.iter().map(|b| b.id.clone()).collect(),
            wired_mismatch: vec![],
            notes: vec!["no model-actual.json; run apply".into()],
        });
    }
    let text = std::fs::read_to_string(&path).map_err(|e| ModelError::Other(e.to_string()))?;
    let actual: ModelActual =
        serde_json::from_str(&text).map_err(|e| ModelError::Other(e.to_string()))?;
    let mut missing = Vec::new();
    let mut wired_mismatch = Vec::new();
    for b in &estate.model_bindings {
        match actual.bindings.iter().find(|a| a.id == b.id) {
            None => missing.push(b.id.clone()),
            Some(a) if a.wired != b.wired => wired_mismatch.push(b.id.clone()),
            Some(_) => {}
        }
    }
    let hash_ok = actual.desired_hash == estate_hash(estate);
    let in_sync = hash_ok && missing.is_empty() && wired_mismatch.is_empty();
    let mut notes = Vec::new();
    if !hash_ok {
        notes.push("model-actual hash differs from desired estate".into());
    }
    if in_sync {
        notes.push("model bindings match desired estate".into());
    }
    Ok(ModelDrift {
        in_sync,
        missing,
        wired_mismatch,
        notes,
    })
}
