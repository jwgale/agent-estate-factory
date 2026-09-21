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

/// Frontier chat model. Override with `CELL_FRONTIER_MODEL` or `XAI_MODEL`.
pub const DEFAULT_FRONTIER_MODEL: &str = "grok-4.7";

/// Factory specialist completion budget. Not a context-window claim.
/// `reasoning_effort` xhigh is the cloud-agent standing default and is not sent.
pub const FRONTIER_COMPLETION_TOKENS: u32 = 64;

/// xAI OpenAI-compatible base. Override with `CELL_FRONTIER_ENDPOINT` or `XAI_API_BASE`.
pub const DEFAULT_FRONTIER_BASE: &str = "https://api.x.ai/v1";

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
        frontier_chat(&self.base, &self.key, &self.model, prompt)
    }
}

/// OpenAI chat against a frontier base. `https://api.x.ai/v1` and a mock
/// origin both work. Does not fall through to Ollama. Does not log the key.
pub fn frontier_chat(base: &str, key: &str, model: &str, text: &str) -> Result<String, ModelError> {
    let url = frontier_chat_url(base);
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": text}],
        "temperature": 0,
        "max_tokens": FRONTIER_COMPLETION_TOKENS
    });
    let resp = ureq::post(&url)
        .set("Authorization", &format!("Bearer {key}"))
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(30))
        .send_json(body)
        .map_err(|e| {
            ModelError::Unreachable(format!(
                "frontier chat: {} (model {model})",
                scrub_err(&e.to_string(), key)
            ))
        })?;
    let status = resp.status();
    let value: Value = resp.into_json().map_err(|e| {
        ModelError::Unreachable(format!(
            "frontier chat json: {e} (status {status}, model {model})"
        ))
    })?;
    value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ModelError::Unreachable(format!(
                "frontier chat: empty message.content (status {status}, model {model})"
            ))
        })
}

pub fn frontier_chat_url(base: &str) -> String {
    let root = base.trim().trim_end_matches('/');
    if root.ends_with("/v1") {
        format!("{root}/chat/completions")
    } else {
        format!("{root}/v1/chat/completions")
    }
}

/// `CELL_FRONTIER_MODEL`, then `other` (named by `other_source`), else `grok-4.7`.
/// A SKU id refuses and names the setting it came from.
pub fn pick_frontier_model(
    cell: Option<&str>,
    other: Option<&str>,
    other_source: &str,
) -> Result<String, ModelError> {
    let (raw, source) = if let Some(value) = nonempty(cell) {
        (value, "CELL_FRONTIER_MODEL")
    } else if let Some(value) = nonempty(other) {
        (
            value,
            if other_source.trim().is_empty() {
                "model override"
            } else {
                other_source.trim()
            },
        )
    } else {
        (DEFAULT_FRONTIER_MODEL, "default")
    };
    if estate_schema::contains_sku(raw) {
        return Err(ModelError::Refused(format!(
            "frontier model '{raw}' from {source} encodes a hardware SKU; unset CELL_FRONTIER_MODEL or XAI_MODEL (default grok-4.7). Optional CELL_FRONTIER_ENDPOINT"
        )));
    }
    Ok(raw.to_string())
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|s| !s.is_empty())
}

pub fn frontier_from_binding(binding: &ModelBinding) -> Result<Box<dyn FrontierDriver>, ModelError> {
    if !binding.wired {
        return Ok(Box::new(UnwiredFrontier {
            id: binding.id.clone(),
        }));
    }
    let driver = binding.driver.trim().to_ascii_lowercase();
    if !matches!(
        driver.as_str(),
        "frontier-http" | "openai-compat" | "http-remote" | "openai" | "grok"
    ) {
        return Err(ModelError::Other(format!(
            "unknown frontier driver '{}'; use frontier-http or openai-compat",
            binding.driver
        )));
    }
    let key_env = param_str(binding, "api_key_env").unwrap_or_else(|| "XAI_API_KEY".into());
    let key = std::env::var(&key_env)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ModelError::MissingCreds(key_env))?;
    let base_env = param_str(binding, "api_base_env").unwrap_or_else(|| "XAI_API_BASE".into());
    let base = std::env::var(&base_env)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| param_str(binding, "api_base"))
        .unwrap_or_else(|| DEFAULT_FRONTIER_BASE.into());
    let model_env = param_str(binding, "model_env").unwrap_or_else(|| "XAI_MODEL".into());
    let from_env = std::env::var(&model_env)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let from_param = param_str(binding, "model").filter(|s| !s.trim().is_empty());
    let (other, other_source) = if let Some(value) = from_env {
        (Some(value), model_env.clone())
    } else if let Some(value) = from_param {
        (Some(value), "binding params.model".into())
    } else {
        (None, model_env.clone())
    };
    let cell = std::env::var("CELL_FRONTIER_MODEL").ok();
    let model = pick_frontier_model(cell.as_deref(), other.as_deref(), &other_source)?;
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

fn scrub_err(raw: &str, key: &str) -> String {
    let mut out = if key.trim().is_empty() {
        raw.to_string()
    } else {
        raw.replace(key, "[redacted]")
    };
    for needle in ["Bearer ", "sk-", "xai-"] {
        if let Some(idx) = out.find(needle) {
            out = format!("{}[redacted]", &out[..idx]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontier_model_defaults_to_grok_4_7() {
        assert_eq!(
            pick_frontier_model(None, None, "XAI_MODEL").unwrap(),
            "grok-4.7"
        );
        assert_eq!(
            pick_frontier_model(Some("grok-4.7"), Some("other"), "XAI_MODEL").unwrap(),
            "grok-4.7"
        );
        assert_eq!(
            pick_frontier_model(None, Some("grok-4.7"), "XAI_MODEL").unwrap(),
            "grok-4.7"
        );
        let err = pick_frontier_model(Some("rtx-5090-chat"), None, "XAI_MODEL").unwrap_err();
        let text = err.to_string();
        assert!(text.contains("SKU"), "{text}");
        assert!(text.contains("CELL_FRONTIER_MODEL"), "{text}");
        assert!(text.contains("CELL_FRONTIER_ENDPOINT"), "{text}");
        let from_xai = pick_frontier_model(None, Some("a100-chat"), "XAI_MODEL").unwrap_err();
        assert!(
            from_xai.to_string().contains("from XAI_MODEL"),
            "{from_xai}"
        );
    }

    #[test]
    fn frontier_chat_url_accepts_v1_base_and_origin() {
        assert_eq!(
            frontier_chat_url("https://api.x.ai/v1"),
            "https://api.x.ai/v1/chat/completions"
        );
        assert_eq!(
            frontier_chat_url("http://127.0.0.1:9"),
            "http://127.0.0.1:9/v1/chat/completions"
        );
    }
}
