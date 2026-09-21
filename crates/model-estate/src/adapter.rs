//! OpenAI-compatible / Ollama HTTP adapter.
//!
//! Live probes hit `GET /v1/models` or Ollama `GET /api/tags`. They do not
//! invent success on an empty or garbage body. Specialist chat posts the
//! real request text (not a dummy ping) to `/v1/chat/completions` or
//! `/api/chat`, then factory-owned policy (`builtin_specialist`) decides.
//! Native MLX `specialist()` stays stubbed; Mac proof is Ollama-on-Mac
//! (or any OpenAI-compatible server, including llama.cpp) on this adapter.

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
/// that carries the request text, then factory-owned policy. A 200 that is
/// not a specialist result refuses (no silent fall-through). No silent allow.
pub fn specialist_via_adapter(
    endpoint: &str,
    req: &SpecialistRequest,
) -> Result<SpecialistResult, ModelError> {
    match post_v0_specialist(endpoint, req) {
        Ok(result) => return Ok(result),
        Err(Transport::NotThisFlavor) => {}
        Err(Transport::Down(e)) => {
            return Err(ModelError::Unreachable(e));
        }
    }
    prove_compat_runtime(endpoint, req)?;
    Ok(builtin_specialist(req))
}

fn prove_compat_runtime(endpoint: &str, req: &SpecialistRequest) -> Result<(), ModelError> {
    let model = resolve_model(endpoint)?;
    match post_openai_chat(endpoint, &model, &req.text) {
        Ok(()) => return Ok(()),
        Err(e) if openai_answered_badly(&e) => {
            return Err(ModelError::Unreachable(e));
        }
        Err(_) => {}
    }
    post_ollama_chat(endpoint, &model, &req.text).map_err(ModelError::Unreachable)
}

/// 200-shaped OpenAI garbage is this flavor, not a reason to try Ollama.
fn openai_answered_badly(err: &str) -> bool {
    err.contains("missing choices")
        || err.contains("empty choices")
        || err.contains("missing message.content")
        || err.contains("empty body")
        || err.contains("json:")
        || err.contains("not a JSON object")
}

fn resolve_model(endpoint: &str) -> Result<String, ModelError> {
    if let Ok(v) = std::env::var("CELL_LOCAL_MODEL") {
        let v = v.trim();
        if !v.is_empty() {
            return accepted_model_id(v);
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

/// Refuse empty and SKU-shaped model ids. Hardware is a driver choice.
pub(crate) fn accepted_model_id(raw: &str) -> Result<String, ModelError> {
    let v = raw.trim();
    if v.is_empty() {
        return Err(ModelError::Unreachable(
            "compat adapter: empty model id".into(),
        ));
    }
    if estate_schema::contains_sku(v) {
        return Err(ModelError::Refused(format!(
            "CELL_LOCAL_MODEL '{v}' encodes a hardware SKU"
        )));
    }
    Ok(v.to_string())
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

fn post_openai_chat(endpoint: &str, model: &str, text: &str) -> Result<(), String> {
    let url = format!(
        "{}/v1/chat/completions",
        endpoint.trim().trim_end_matches('/')
    );
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role":"user","content": text}],
        "max_tokens": 16,
    });
    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(SPECIALIST_TIMEOUT)
        .send_json(body)
        .map_err(|e| e.to_string())?;
    let raw = resp.into_string().map_err(|e| e.to_string())?;
    let v = parse_json_object(&raw)?;
    let choices = v
        .get("choices")
        .and_then(|c| c.as_array())
        .ok_or_else(|| "openai chat: missing choices".to_string())?;
    if choices.is_empty() {
        return Err("openai chat: empty choices".into());
    }
    let content = choices[0]
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str());
    if content.is_none() {
        return Err("openai chat: missing message.content".into());
    }
    Ok(())
}

fn post_ollama_chat(endpoint: &str, model: &str, text: &str) -> Result<(), String> {
    let url = format!("{}/api/chat", endpoint.trim().trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role":"user","content": text}],
        "stream": false,
    });
    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(SPECIALIST_TIMEOUT)
        .send_json(body)
        .map_err(|e| e.to_string())?;
    let raw = resp.into_string().map_err(|e| e.to_string())?;
    let v = parse_json_object(&raw)?;
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
        .filter_map(|row| row.get("id").and_then(|id| id.as_str()))
        .find(|s| !s.is_empty() && !estate_schema::contains_sku(s))
        .map(|s| s.to_string())
}

fn first_ollama_model(v: &Value) -> Option<String> {
    v.get("models")?
        .as_array()?
        .iter()
        .filter_map(|row| {
            row.get("name")
                .or_else(|| row.get("model"))
                .and_then(|n| n.as_str())
        })
        .find(|s| !s.is_empty() && !estate_schema::contains_sku(s))
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
    use crate::catalog::LocalRuntime;
    use crate::local::{HttpLocal, LocalDriver, SpecialistJob, SpecialistRequest};
    use crate::mock::{CompatScript, CompatServer, MockLocalServer};

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
    fn specialist_openai_sends_request_text_then_factory_policy() {
        let srv = CompatServer::spawn(CompatScript::OpenAi {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let marker = "roundtrip-marker-not-a-dummy-ping";
        let allow = specialist_via_adapter(&srv.endpoint(), &req(marker)).unwrap();
        assert!(allow.allow, "{}", allow.reason);
        let (path, body) = srv.last_post().expect("compat chat POST");
        assert_eq!(path, "/v1/chat/completions");
        assert!(body.contains(marker), "{body}");
        assert!(
            !body.contains("\"content\":\"ping\""),
            "must not send dummy ping: {body}"
        );
        let deny = specialist_via_adapter(&srv.endpoint(), &req("cyera")).unwrap();
        assert!(!deny.allow, "{}", deny.reason);
        assert!(deny.reason.contains("sacred"), "{}", deny.reason);
    }

    #[test]
    fn specialist_ollama_sends_request_text() {
        let srv = CompatServer::spawn(CompatScript::Ollama {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let marker = "ollama-roundtrip-marker";
        let allow = specialist_via_adapter(&srv.endpoint(), &req(marker)).unwrap();
        assert!(allow.allow, "{}", allow.reason);
        let (path, body) = srv.last_post().expect("ollama chat POST");
        assert_eq!(path, "/api/chat");
        assert!(body.contains(marker), "{body}");
    }

    #[test]
    fn specialist_without_models_fail_closed() {
        let srv = CompatServer::spawn(CompatScript::OpenAi { models: vec![] }).unwrap();
        let err = specialist_via_adapter(&srv.endpoint(), &req("hello")).unwrap_err();
        assert!(err.is_local_down(), "{err}");
    }

    #[test]
    fn specialist_v0_unparseable_refuses_no_compat_fallback() {
        let srv = CompatServer::spawn(CompatScript::V0Unparseable).unwrap();
        let err = specialist_via_adapter(&srv.endpoint(), &req("hello")).unwrap_err();
        assert!(err.is_local_down(), "{err}");
        assert!(
            err.to_string().contains("v0 specialist"),
            "{err}"
        );
    }

    #[test]
    fn specialist_openai_missing_content_fail_closed() {
        let srv = CompatServer::spawn(CompatScript::OpenAiNoContent {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let err = specialist_via_adapter(&srv.endpoint(), &req("hello")).unwrap_err();
        assert!(err.is_local_down(), "{err}");
        assert!(
            err.to_string().contains("message.content"),
            "{err}"
        );
    }

    #[test]
    fn http_local_v0_roundtrip_carries_text() {
        let srv = MockLocalServer::spawn().unwrap();
        let local = HttpLocal {
            id: "local_slm".into(),
            endpoint: srv.endpoint(),
            runtime: LocalRuntime::Ollama,
        };
        let allow = local.specialist(&req("hello from the factory")).unwrap();
        assert!(allow.allow, "{}", allow.reason);
        let deny = local.specialist(&req("please mention cyera")).unwrap();
        assert!(!deny.allow, "{}", deny.reason);
        let (path, body) = srv.last_post().expect("v0 POST");
        assert_eq!(path, "/v0/specialist");
        assert!(body.contains("please mention cyera"), "{body}");
    }

    #[test]
    fn llama_cpp_openai_path_smoke_via_http_local() {
        let srv = CompatServer::spawn(CompatScript::OpenAi {
            models: vec!["ggml-model".into()],
        })
        .unwrap();
        let local = HttpLocal {
            id: "local_slm".into(),
            endpoint: srv.endpoint(),
            runtime: LocalRuntime::LlamaCpp,
        };
        let marker = "llamacpp-openai-roundtrip";
        let result = local.specialist(&req(marker)).unwrap();
        assert!(result.allow, "{}", result.reason);
        assert_eq!(local.runtime(), LocalRuntime::LlamaCpp);
        let (path, body) = srv.last_post().expect("llama.cpp openai POST");
        assert_eq!(path, "/v1/chat/completions");
        assert!(body.contains(marker), "{body}");
    }

    #[test]
    fn accepted_model_id_refuses_sku() {
        let err = accepted_model_id("llama3-rtx-5090").unwrap_err();
        assert!(err.to_string().contains("SKU"), "{err}");
        assert!(err.is_local_down(), "{err}");
        assert!(accepted_model_id("llama3").is_ok());
        assert!(accepted_model_id("  ").is_err());
    }

    #[test]
    fn listed_sku_model_is_skipped_not_bound() {
        let srv = CompatServer::spawn(CompatScript::OpenAi {
            models: vec!["rtx-5090-chat".into()],
        })
        .unwrap();
        let err = specialist_via_adapter(&srv.endpoint(), &req("hello")).unwrap_err();
        assert!(err.is_local_down(), "{err}");
        assert!(err.to_string().contains("no model id"), "{err}");
    }

    #[test]
    fn listed_sku_then_portable_model_is_used() {
        let srv = CompatServer::spawn(CompatScript::OpenAi {
            models: vec!["rtx-5090-chat".into(), "llama3".into()],
        })
        .unwrap();
        let marker = "skip-sku-use-llama3";
        let allow = specialist_via_adapter(&srv.endpoint(), &req(marker)).unwrap();
        assert!(allow.allow, "{}", allow.reason);
        let (_, body) = srv.last_post().expect("chat");
        assert!(body.contains("llama3"), "{body}");
        assert!(body.contains(marker), "{body}");
        assert!(!body.contains("5090"), "{body}");
    }
}
