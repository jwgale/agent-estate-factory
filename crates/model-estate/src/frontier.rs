use crate::error::ModelError;
use estate_schema::ModelBinding;
use serde_json::Value;
use std::time::Duration;

pub trait FrontierDriver: Send + Sync {
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

pub struct MockFrontier {
    pub id: String,
    pub reply: String,
}

impl FrontierDriver for MockFrontier {
    fn id(&self) -> &str {
        &self.id
    }

    fn complete(&self, _prompt: &str) -> Result<String, ModelError> {
        Ok(self.reply.clone())
    }
}

/// HTTP chat-completions client. Base URL and key come from env named in the binding.
pub struct HttpFrontier {
    pub id: String,
    pub base: String,
    pub key: String,
    pub model: String,
}

impl FrontierDriver for HttpFrontier {
    fn id(&self) -> &str {
        &self.id
    }

    fn complete(&self, prompt: &str) -> Result<String, ModelError> {
        let url = format!("{}/chat/completions", self.base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0
        });
        let resp = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", self.key))
            .set("Content-Type", "application/json")
            .timeout(Duration::from_secs(30))
            .send_json(body)
            .map_err(|e| ModelError::Unreachable(scrub_err(&e.to_string())))?;
        let value: Value = resp
            .into_json()
            .map_err(|e| ModelError::Other(format!("frontier json: {e}")))?;
        value
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ModelError::Other("frontier response missing content".into()))
    }
}

pub fn frontier_from_binding(binding: &ModelBinding) -> Result<Box<dyn FrontierDriver>, ModelError> {
    if !binding.wired {
        return Ok(Box::new(UnwiredFrontier {
            id: binding.id.clone(),
        }));
    }
    let key_env = param_str(binding, "api_key_env").unwrap_or_else(|| "XAI_API_KEY".into());
    let key = std::env::var(&key_env).map_err(|_| ModelError::MissingCreds(key_env))?;
    let base_env = param_str(binding, "api_base_env").unwrap_or_else(|| "XAI_API_BASE".into());
    let base = std::env::var(&base_env)
        .ok()
        .or_else(|| param_str(binding, "api_base"))
        .unwrap_or_else(|| "https://api.x.ai/v1".into());
    let model_env = param_str(binding, "model_env").unwrap_or_else(|| "XAI_MODEL".into());
    let model = std::env::var(&model_env)
        .ok()
        .or_else(|| param_str(binding, "model"))
        .unwrap_or_else(|| "grok-3-mini".into());
    Ok(Box::new(HttpFrontier {
        id: binding.id.clone(),
        base,
        key,
        model,
    }))
}

pub fn param_str(binding: &ModelBinding, key: &str) -> Option<String> {
    binding
        .params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn scrub_err(raw: &str) -> String {
    let mut out = raw.to_string();
    for needle in ["Bearer ", "sk-", "xai-"] {
        if let Some(idx) = out.find(needle) {
            out = format!("{}[redacted]", &out[..idx]);
        }
    }
    out
}
