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
    let body = serialize_pretty(&actual)?;
    std::fs::write(&path, body).map_err(|e| ModelError::Other(e.to_string()))?;
    Ok(actual)
}

fn refuse_empty_blob(body: &str) -> Result<(), ModelError> {
    if body.trim().is_empty() {
        return Err(ModelError::Other("serialize: empty blob".into()));
    }
    Ok(())
}

fn serialize_pretty<T: Serialize>(value: &T) -> Result<String, ModelError> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ModelError::Other(format!("serialize: {e}")))?;
    refuse_empty_blob(&body)?;
    Ok(body)
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::ser::Error;
    use serde::Serialize;

    struct Boom;
    impl Serialize for Boom {
        fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
            Err(S::Error::custom("boom"))
        }
    }

    fn estate() -> Estate {
        estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
    }

    fn tmp(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "cell-one-model-actual-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn record_bindings_refuses_empty_serialize_and_leaves_sentinel() {
        assert!(serialize_pretty(&Boom).is_err());
        assert!(refuse_empty_blob("").is_err());
        assert!(refuse_empty_blob("  \n").is_err());

        let dir = tmp("sentinel");
        let path = dir.join("model-actual.json");
        std::fs::write(&path, "sentinel\n").unwrap();
        let body = serialize_pretty(&Boom);
        assert!(body.is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "sentinel\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_bindings_writes_nonempty_and_garbage_is_not_greenfield() {
        let estate = estate();
        let dir = tmp("roundtrip");
        let actual = record_bindings(&estate, &dir).unwrap();
        let blob = std::fs::read_to_string(dir.join("model-actual.json")).unwrap();
        assert!(!blob.trim().is_empty(), "must not write empty model-actual");
        assert!(blob.contains(&actual.desired_hash));
        assert!(drift_bindings(&estate, &dir).unwrap().in_sync);

        std::fs::write(dir.join("model-actual.json"), "not-json\n").unwrap();
        let err = drift_bindings(&estate, &dir).unwrap_err();
        assert!(
            !err.to_string().contains("run apply"),
            "garbage model-actual must refuse, not look greenfield: {err}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("model-actual.json")).unwrap(),
            "not-json\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
