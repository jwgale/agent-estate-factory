//! OpenAI-compatible / Ollama HTTP adapter.
//!
//! Live probes hit `GET /v1/models` or Ollama `GET /api/tags`. They do not
//! invent success on an empty or garbage body. Specialist work still owns
//! policy locally (`builtin_specialist`) after a real completion answers.
//! Native MLX `specialist()` stays stubbed; Mac proof is Ollama-on-Mac
//! (or any OpenAI-compatible server) on this adapter.

use crate::error::ModelError;
use crate::local::{builtin_specialist, SpecialistRequest, SpecialistResult};
use serde_json::Value;
use std::time::Duration;

const PING_TIMEOUT: Duration = Duration::from_millis(800);
const SPECIALIST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveFlavor {
    OpenAiModels,
    OllamaTags,
}

impl LiveFlavor {
    pub fn as_str(self) -> &'static str {
        match self {
            LiveFlavor::OpenAiModels => "openai /v1/models",
            LiveFlavor::OllamaTags => "ollama /api/tags",
        }
    }
}

#[derive(Debug)]
enum Transport {
    NotThisFlavor,
    Down(String),
}

/// Honest live ping. `Ok` only when a models list JSON is present.
/// Empty list is up (server running, no weights). Garbage / empty body is down.
pub fn ping_live_endpoint(endpoint: &str) -> Result<LiveFlavor, String> {
    let base = endpoint.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err("empty endpoint".into());
    }
    let openai = get_json(&format!("{base}/v1/models"), PING_TIMEOUT);
    match openai {
        Ok(v) if is_openai_models(&v) => return Ok(LiveFlavor::OpenAiModels),
        Err(Transport::Down(e)) if is_connect_fail(&e) => return Err(e),
        _ => {}
    }
    match get_json(&format!("{base}/api/tags"), PING_TIMEOUT) {
        Ok(v) if is_ollama_tags(&v) => return Ok(LiveFlavor::OllamaTags),
        Ok(_) => Err("ollama /api/tags: not a models list".into()),
        Err(Transport::NotThisFlavor) => match openai {
            Ok(_) => Err("openai /v1/models: not a models list".into()),
            Err(_) => Err("no /v1/models or /api/tags models list".into()),
        },
        Err(Transport::Down(e)) => Err(format!("ollama /api/tags: {e}")),
    }
}

/// Factory `/v0/specialist` first (mock-local). Else OpenAI/Ollama completion
/// to prove the runtime, then factory-owned policy. No silent allow.
pub fn specialist_via_adapter(
    endpoint: &str,
    req: &SpecialistRequest,
) -> Result<SpecialistResult, ModelError> {
    match post_v0_specialist(endpoint, req) {
        Ok(result) => return Ok(result),
        Err(Transport::NotThisFlavor) => {}
        Err(Transport::Down(e)) if is_connect_fail(&e) => {
            return Err(ModelError::Unreachable(e));
        }
        Err(Transport::Down(_)) => {}
    }
    prove_compat_runtime(endpoint)?;
    Ok(builtin_specialist(req))
}

fn prove_compat_runtime(endpoint: &str) -> Result<(), ModelError> {
    let model = resolve_model(endpoint)?;
    if post_openai_chat(endpoint, &model).is_ok() {
        return Ok(());
    }
    post_ollama_chat(endpoint, &model).map_err(ModelError::Unreachable)
}

fn resolve_model(endpoint: &str) -> Result<String, ModelError> {
    if let Ok(v) = std::env::var("CELL_LOCAL_MODEL") {
        let v = v.trim();
        if !v.is_empty() {
            return Ok(v.to_string());
        }
    }
    let base = endpoint.trim().trim_end_matches('/');
    if let Ok(v) = get_json(&format!("{base}/v1/models"), PING_TIMEOUT) {
        if let Some(id) = first_openai_model(&v) {
            return Ok(id);
        }
    }
    if let Ok(v) = get_json(&format!("{base}/api/tags"), PING_TIMEOUT) {
        if let Some(name) = first_ollama_model(&v) {
            return Ok(name);
        }
    }
    Err(ModelError::Unreachable(
        "compat adapter: no model id (pull a model or set CELL_LOCAL_MODEL)".into(),
    ))
}

fn post_v0_specialist(
    endpoint: &str,
    req: &SpecialistRequest,
) -> Result<SpecialistResult, Transport> {
    let url = format!("{}/v0/specialist", endpoint.trim().trim_end_matches('/'));
    let body = serde_json::json!({
        "job": req.job.as_str(),
        "agent_id": req.agent_id,
        "kind": req.kind,
        "text": req.text,
    });
    match ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(SPECIALIST_TIMEOUT)
        .send_json(body)
    {
        Ok(resp) => {
            let text = resp
                .into_string()
                .map_err(|e| Transport::Down(e.to_string()))?;
            if text.trim().is_empty() {
                return Err(Transport::Down("v0 specialist: empty body".into()));
            }
            serde_json::from_str::<SpecialistResult>(&text)
                .map_err(|e| Transport::Down(format!("v0 specialist json: {e}")))
        }
        Err(ureq::Error::Status(code, _)) if is_missing_path(code) => {
            Err(Transport::NotThisFlavor)
        }
        Err(e) => Err(Transport::Down(e.to_string())),
    }
}

fn post_openai_chat(endpoint: &str, model: &str) -> Result<(), String> {
    let url = format!(
        "{}/v1/chat/completions",
        endpoint.trim().trim_end_matches('/')
    );
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role":"user","content":"ping"}],
        "max_tokens": 1,
    });
    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(SPECIALIST_TIMEOUT)
        .send_json(body)
        .map_err(|e| e.to_string())?;
    let text = resp.into_string().map_err(|e| e.to_string())?;
    let v = parse_json_object(&text)?;
    let choices = v
        .get("choices")
        .and_then(|c| c.as_array())
        .ok_or_else(|| "openai chat: missing choices".to_string())?;
    if choices.is_empty() {
        return Err("openai chat: empty choices".into());
    }
    Ok(())
}

fn post_ollama_chat(endpoint: &str, model: &str) -> Result<(), String> {
    let url = format!("{}/api/chat", endpoint.trim().trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role":"user","content":"ping"}],
        "stream": false,
    });
    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(SPECIALIST_TIMEOUT)
        .send_json(body)
        .map_err(|e| e.to_string())?;
    let text = resp.into_string().map_err(|e| e.to_string())?;
    let v = parse_json_object(&text)?;
    let has_msg = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .is_some();
    let has_response = v.get("response").and_then(|c| c.as_str()).is_some();
    if has_msg || has_response {
        return Ok(());
    }
    Err("ollama chat: missing message.content".into())
}

fn get_json(url: &str, timeout: Duration) -> Result<Value, Transport> {
    match ureq::get(url).timeout(timeout).call() {
        Ok(resp) => {
            let text = resp
                .into_string()
                .map_err(|e| Transport::Down(e.to_string()))?;
            parse_json_object(&text).map_err(Transport::Down)
        }
        Err(ureq::Error::Status(code, _)) if is_missing_path(code) => {
            Err(Transport::NotThisFlavor)
        }
        Err(e) => Err(Transport::Down(e.to_string())),
    }
}

fn parse_json_object(text: &str) -> Result<Value, String> {
    if text.trim().is_empty() {
        return Err("empty body".into());
    }
    let v: Value = serde_json::from_str(text).map_err(|e| format!("json: {e}"))?;
    if !v.is_object() {
        return Err("body is not a JSON object".into());
    }
    Ok(v)
}

fn is_openai_models(v: &Value) -> bool {
    v.get("data").map(|d| d.is_array()).unwrap_or(false)
}

fn is_ollama_tags(v: &Value) -> bool {
    v.get("models").map(|d| d.is_array()).unwrap_or(false)
}

fn first_openai_model(v: &Value) -> Option<String> {
    v.get("data")?
        .as_array()?
        .iter()
        .find_map(|row| row.get("id").and_then(|id| id.as_str()))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn first_ollama_model(v: &Value) -> Option<String> {
    v.get("models")?
        .as_array()?
        .iter()
        .find_map(|row| {
            row.get("name")
                .or_else(|| row.get("model"))
                .and_then(|n| n.as_str())
        })
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn is_missing_path(code: u16) -> bool {
    matches!(code, 404 | 405)
}

fn is_connect_fail(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    e.contains("connection refused")
        || e.contains("connect")
        || e.contains("dns")
        || e.contains("timed out")
        || e.contains("timeout")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local::{SpecialistJob, SpecialistRequest};
    use crate::mock::{CompatScript, CompatServer};

    fn req(text: &str) -> SpecialistRequest {
        SpecialistRequest {
            job: SpecialistJob::PolicyPrecheck,
            agent_id: "probe".into(),
            kind: "probe".into(),
            text: text.into(),
        }
    }

    #[test]
    fn ping_openai_models_is_up_including_empty_list() {
        let srv = CompatServer::spawn(CompatScript::OpenAi { models: vec![] }).unwrap();
        let flavor = ping_live_endpoint(&srv.endpoint()).unwrap();
        assert_eq!(flavor, LiveFlavor::OpenAiModels);
    }

    #[test]
    fn ping_ollama_tags_is_up_when_openai_missing() {
        let srv = CompatServer::spawn(CompatScript::Ollama {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let flavor = ping_live_endpoint(&srv.endpoint()).unwrap();
        assert_eq!(flavor, LiveFlavor::OllamaTags);
    }

    #[test]
    fn ping_garbage_or_empty_is_down_not_invented() {
        let junk = CompatServer::spawn(CompatScript::Garbage).unwrap();
        let err = ping_live_endpoint(&junk.endpoint()).unwrap_err();
        assert!(!err.is_empty(), "{err}");
        let empty = CompatServer::spawn(CompatScript::EmptyBody).unwrap();
        let err = ping_live_endpoint(&empty.endpoint()).unwrap_err();
        assert!(err.contains("empty") || err.contains("not a models"), "{err}");
    }

    #[test]
    fn ping_down_host_is_down() {
        let err = ping_live_endpoint("http://127.0.0.1:1").unwrap_err();
        assert!(!err.is_empty(), "{err}");
    }

    #[test]
    fn specialist_openai_then_factory_policy() {
        let srv = CompatServer::spawn(CompatScript::OpenAi {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let allow = specialist_via_adapter(&srv.endpoint(), &req("hello")).unwrap();
        assert!(allow.allow, "{}", allow.reason);
        let deny = specialist_via_adapter(&srv.endpoint(), &req("cyera")).unwrap();
        assert!(!deny.allow, "{}", deny.reason);
        assert!(deny.reason.contains("sacred"), "{}", deny.reason);
    }

    #[test]
    fn specialist_ollama_then_factory_policy() {
        let srv = CompatServer::spawn(CompatScript::Ollama {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let allow = specialist_via_adapter(&srv.endpoint(), &req("hello")).unwrap();
        assert!(allow.allow, "{}", allow.reason);
    }

    #[test]
    fn specialist_without_models_fail_closed() {
        let srv = CompatServer::spawn(CompatScript::OpenAi { models: vec![] }).unwrap();
        let err = specialist_via_adapter(&srv.endpoint(), &req("hello")).unwrap_err();
        assert!(err.is_local_down(), "{err}");
    }
}
