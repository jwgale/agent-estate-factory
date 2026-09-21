//! Equal-class frontier and local bindings. Cell One drivers are unwired stubs.

use estate_schema::{Estate, ModelBinding, ModelClass};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ModelError {
    #[error("model binding '{0}' is not wired (Cell One placeholder)")]
    NotWired(String),
    #[error("unknown model binding '{0}'")]
    Unknown(String),
}

pub trait FrontierDriver {
    fn id(&self) -> &str;
    fn complete(&self, prompt: &str) -> Result<String, ModelError>;
}

pub trait LocalDriver {
    fn id(&self) -> &str;
    fn complete(&self, prompt: &str) -> Result<String, ModelError>;
}

pub struct UnwiredFrontier {
    pub id: String,
}

impl FrontierDriver for UnwiredFrontier {
    fn id(&self) -> &str {
        &self.id
    }

    fn complete(&self, _prompt: &str) -> Result<String, ModelError> {
        Err(ModelError::NotWired(self.id.clone()))
    }
}

pub struct UnwiredLocal {
    pub id: String,
}

impl LocalDriver for UnwiredLocal {
    fn id(&self) -> &str {
        &self.id
    }

    fn complete(&self, _prompt: &str) -> Result<String, ModelError> {
        Err(ModelError::NotWired(self.id.clone()))
    }
}

pub fn frontier_bindings(estate: &Estate) -> Vec<&ModelBinding> {
    estate
        .model_bindings
        .iter()
        .filter(|b| b.class == ModelClass::Frontier)
        .collect()
}

pub fn local_bindings(estate: &Estate) -> Vec<&ModelBinding> {
    estate
        .model_bindings
        .iter()
        .filter(|b| b.class == ModelClass::Local)
        .collect()
}

pub fn ping(estate: &Estate, binding_id: &str) -> Result<(), ModelError> {
    let binding = estate
        .model_bindings
        .iter()
        .find(|b| b.id == binding_id)
        .ok_or_else(|| ModelError::Unknown(binding_id.to_string()))?;
    if !binding.wired {
        return Err(ModelError::NotWired(binding.id.clone()));
    }
    Ok(())
}

pub fn describe_bindings(estate: &Estate) -> String {
    let mut lines = vec!["model bindings (equal class; Cell One will not invoke):".to_string()];
    for b in &estate.model_bindings {
        lines.push(format!(
            "  {:<8} {:<16} driver={:<16} wired={}",
            b.class.as_str(),
            b.id,
            b.driver,
            b.wired
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_refuse_completion() {
        let e = estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        assert_eq!(frontier_bindings(&e).len(), 1);
        assert_eq!(local_bindings(&e).len(), 1);
        let f = UnwiredFrontier {
            id: e.model_bindings[0].id.clone(),
        };
        assert!(matches!(f.complete("hi"), Err(ModelError::NotWired(_))));
        assert!(ping(&e, "xai_grok").is_err());
        assert!(ping(&e, "gpu_5090").is_err());
    }
}
