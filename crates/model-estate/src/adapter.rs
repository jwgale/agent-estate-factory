//! OpenAI-compatible / Ollama HTTP adapter.
//!
//! Live probes hit `GET /v1/models` or Ollama `GET /api/tags`. They do not
//! invent success on an empty or garbage body. Specialist chat posts the
//! real request text (not a dummy ping) to `/v1/chat/completions` or
//! `/api/chat`. Empty / missing / whitespace OpenAI `message.content`
//! falls through to Ollama `/api/chat` (probes already try both shapes).
//! Policy jobs then use factory-owned `builtin_specialist`. `complete`
//! keeps the model text. Sacred refuse happens before the chat POST.
//! Native MLX `specialist()` stays stubbed; Mac proof is Ollama-on-Mac
//! (or any OpenAI-compatible server, including llama.cpp) on this adapter.

use crate::error::ModelError;
use crate::local::{builtin_specialist, SpecialistJob, SpecialistRequest, SpecialistResult};
use serde_json::Value;
use std::time::Duration;

const PING_TIMEOUT: Duration = Duration::from_millis(800);
const SPECIALIST_TIMEOUT: Duration = Duration::from_secs(30);
const COMPLETE_MAX_TOKENS: u32 = 64;

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

/// Factory `/v0/specialist` first (mock-local). Else factory policy, then
/// OpenAI/Ollama chat. `complete` returns the model text. A 200 that is
/// not a specialist result refuses (no silent fall-through). No silent allow.
pub fn specialist_via_adapter(
    endpoint: &str,
    req: &SpecialistRequest,
) -> Result<SpecialistResult, ModelError> {
    let policy = builtin_specialist(&SpecialistRequest {
        job: SpecialistJob::PolicyPrecheck,
        agent_id: req.agent_id.clone(),
        kind: req.kind.clone(),
        text: req.text.clone(),
    });
    if !policy.allow {
        return Ok(SpecialistResult {
            job: req.job.as_str().into(),
            completion: String::new(),
            ..policy
        });
    }
    match post_v0_specialist(endpoint, req) {
        Ok(result) => return finish_v0(result, req),
        Err(Transport::NotThisFlavor) => {}
        Err(Transport::Down(e)) => {
            return Err(ModelError::Unreachable(e));
        }
    }
    match req.job {
        SpecialistJob::Complete => {
            let completion = compat_chat(endpoint, &req.text)?;
            Ok(SpecialistResult {
                allow: true,
                redacted_text: policy.redacted_text,
                reason: "compat completion".into(),
                job: req.job.as_str().into(),
                completion,
            })
        }
        SpecialistJob::PolicyPrecheck | SpecialistJob::Redact => {
            prove_compat_runtime(endpoint, &req.text)?;
            Ok(SpecialistResult {
                job: req.job.as_str().into(),
                completion: String::new(),
                ..policy
            })
        }
    }
}

fn finish_v0(
    result: SpecialistResult,
    req: &SpecialistRequest,
) -> Result<SpecialistResult, ModelError> {
    if req.job == SpecialistJob::Complete
        && result.allow
        && result.completion.trim().is_empty()
    {
        return Err(ModelError::Unreachable(
            "v0 specialist: empty completion".into(),
        ));
    }
    Ok(result)
}

fn prove_compat_runtime(endpoint: &str, text: &str) -> Result<(), ModelError> {
    compat_chat(endpoint, text).map(|_| ())
}

fn compat_chat(endpoint: &str, text: &str) -> Result<String, ModelError> {
    let model = resolve_model(endpoint)?;
    match post_openai_chat(endpoint, &model, text) {
        Ok(content) => accepted_completion(&content),
        Err(openai_err) => match post_ollama_chat(endpoint, &model, text) {
            Ok(content) => accepted_completion(&content),
            Err(ollama_err) => Err(ModelError::Unreachable(both_chat_fail(
                &openai_err,
                &ollama_err,
                &model,
            ))),
        },
    }
}

fn both_chat_fail(openai_err: &str, ollama_err: &str, model: &str) -> String {
    format!(
        "compat adapter: openai={openai_err}; ollama={ollama_err}; model={model}. Pull a model (`ollama pull llama3`) or set CELL_LOCAL_MODEL"
    )
}

fn accepted_completion(raw: &str) -> Result<String, ModelError> {
    let v = raw.trim();
    if v.is_empty() {
        return Err(ModelError::Unreachable(
            "compat adapter: empty completion".into(),
        ));
    }
    if estate_schema::contains_sku(v) {
        return Err(ModelError::Refused(
            "completion encodes a hardware SKU".into(),
        ));
    }
    Ok(v.to_string())
}

fn resolve_model(endpoint: &str) -> Result<String, ModelError> {
    if let Ok(v) = std::env::var("CELL_LOCAL_MODEL") {
        let v = v.trim();
        if !v.is_empty() {
            return accepted_model_id(v);
        }
    }
    let base = endpoint.trim().trim_end_matches('/');
    let mut saw_empty = false;
    if let Ok(v) = get_json(&format!("{base}/api/tags"), PING_TIMEOUT) {
        if is_ollama_tags(&v) {
            if let Some(name) = first_ollama_model(&v) {
                return Ok(name);
            }
            if v.get("models")
                .and_then(|m| m.as_array())
                .map(|a| a.is_empty())
                .unwrap_or(false)
            {
                saw_empty = true;
            }
        }
    }
    if let Ok(v) = get_json(&format!("{base}/v1/models"), PING_TIMEOUT) {
        if is_openai_models(&v) {
            if let Some(id) = first_openai_model(&v) {
                return Ok(id);
            }
            if v.get("data")
                .and_then(|m| m.as_array())
                .map(|a| a.is_empty())
                .unwrap_or(false)
            {
                saw_empty = true;
            }
        }
    }
    if saw_empty {
        return Err(ModelError::Unreachable(
            "compat adapter: listed no models (server is up, no weights). Pull one (`ollama pull llama3`) or set CELL_LOCAL_MODEL"
                .into(),
        ));
    }
    Err(ModelError::Unreachable(
        "compat adapter: no model id. Pull a model (`ollama pull llama3`) or set CELL_LOCAL_MODEL"
            .into(),
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

fn post_openai_chat(endpoint: &str, model: &str, text: &str) -> Result<String, String> {
    let url = format!(
        "{}/v1/chat/completions",
        endpoint.trim().trim_end_matches('/')
    );
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role":"user","content": text}],
        "max_tokens": COMPLETE_MAX_TOKENS,
    });
    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(SPECIALIST_TIMEOUT)
        .send_json(body)
        .map_err(|e| format!("openai chat: {e}"))?;
    let status = resp.status();
    let raw = resp
        .into_string()
        .map_err(|e| format!("openai chat: {e} (status {status})"))?;
    let v = parse_json_object(&raw)
        .map_err(|e| format!("openai chat: {e} (status {status})"))?;
    extract_openai_text(&v)
        .ok_or_else(|| format!("openai chat: empty message.content (status {status})"))
}

fn post_ollama_chat(endpoint: &str, model: &str, text: &str) -> Result<String, String> {
    let url = format!("{}/api/chat", endpoint.trim().trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role":"user","content": text}],
        "stream": false,
        "options": {"num_predict": COMPLETE_MAX_TOKENS},
    });
    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(SPECIALIST_TIMEOUT)
        .send_json(body)
        .map_err(|e| format!("ollama chat: {e}"))?;
    let status = resp.status();
    let raw = resp
        .into_string()
        .map_err(|e| format!("ollama chat: {e} (status {status})"))?;
    let v = parse_json_object(&raw)
        .map_err(|e| format!("ollama chat: {e} (status {status})"))?;
    extract_ollama_text(&v)
        .ok_or_else(|| format!("ollama chat: empty message.content (status {status})"))
}

fn nonempty_text(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn extract_text_value(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => nonempty_text(s),
        Value::Array(parts) => {
            let mut out = String::new();
            for part in parts {
                let chunk = match part {
                    Value::String(s) => nonempty_text(s),
                    Value::Object(_) => part
                        .get("text")
                        .or_else(|| part.get("content"))
                        .and_then(extract_text_value),
                    _ => None,
                };
                if let Some(chunk) = chunk {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(&chunk);
                }
            }
            nonempty_text(&out)
        }
        _ => None,
    }
}

/// String `content`, content-part arrays, `text`, or reasoning-only when
/// the text is clearly non-empty. Whitespace / missing is None (fall through).
fn extract_openai_text(v: &Value) -> Option<String> {
    let choice = v.get("choices")?.as_array()?.first()?;
    if let Some(msg) = choice.get("message") {
        if let Some(text) = msg.get("content").and_then(extract_text_value) {
            return Some(text);
        }
        if let Some(text) = msg.get("text").and_then(extract_text_value) {
            return Some(text);
        }
        if let Some(text) = msg
            .get("reasoning_content")
            .or_else(|| msg.get("reasoning"))
            .and_then(extract_text_value)
        {
            return Some(text);
        }
    }
    choice.get("text").and_then(extract_text_value)
}

fn extract_ollama_text(v: &Value) -> Option<String> {
    if let Some(text) = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(extract_text_value)
    {
        return Some(text);
    }
    v.get("response").and_then(extract_text_value)
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
    fn specialist_openai_missing_content_tries_ollama_then_refuses() {
        let srv = CompatServer::spawn(CompatScript::OpenAiNoContent {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let err = specialist_via_adapter(&srv.endpoint(), &req("hello")).unwrap_err();
        assert!(err.is_local_down(), "{err}");
        let msg = err.to_string();
        assert!(msg.contains("message.content"), "{err}");
        assert!(msg.contains("ollama"), "{err}");
        assert!(msg.contains("llama3"), "{err}");
        assert!(msg.contains("CELL_LOCAL_MODEL"), "{err}");
        assert!(msg.contains("ollama pull"), "{err}");
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
        let (path, body) = srv.last_post().expect("v0 POST");
        assert_eq!(path, "/v0/specialist");
        assert!(body.contains("hello from the factory"), "{body}");
        let deny = local.specialist(&req("please mention cyera")).unwrap();
        assert!(!deny.allow, "{}", deny.reason);
        let (_, body) = srv.last_post().expect("v0 POST stays the allow");
        assert!(
            !body.contains("please mention cyera"),
            "sacred text must not POST: {body}"
        );
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

    fn complete_req(text: &str) -> SpecialistRequest {
        SpecialistRequest {
            job: SpecialistJob::Complete,
            agent_id: "probe".into(),
            kind: "probe".into(),
            text: text.into(),
        }
    }

    #[test]
    fn specialist_complete_openai_returns_model_text() {
        let srv = CompatServer::spawn(CompatScript::OpenAi {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let marker = "complete-openai-marker";
        let result = specialist_via_adapter(&srv.endpoint(), &complete_req(marker)).unwrap();
        assert!(result.allow, "{}", result.reason);
        assert_eq!(result.job, "complete");
        assert_eq!(result.completion, "ok");
        assert_ne!(result.completion, marker);
        let (path, body) = srv.last_post().expect("compat chat POST");
        assert_eq!(path, "/v1/chat/completions");
        assert!(body.contains(marker), "{body}");
    }

    #[test]
    fn specialist_complete_ollama_returns_model_text() {
        let srv = CompatServer::spawn(CompatScript::Ollama {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let result = specialist_via_adapter(&srv.endpoint(), &complete_req("ping")).unwrap();
        assert!(result.allow, "{}", result.reason);
        assert_eq!(result.completion, "ok");
        let (path, _) = srv.last_post().expect("ollama chat POST");
        assert_eq!(path, "/api/chat");
    }

    #[test]
    fn specialist_complete_sacred_does_not_post() {
        let srv = CompatServer::spawn(CompatScript::OpenAi {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let deny = specialist_via_adapter(&srv.endpoint(), &complete_req("cyera")).unwrap();
        assert!(!deny.allow, "{}", deny.reason);
        assert!(deny.completion.is_empty(), "{deny:?}");
        assert!(srv.last_post().is_none(), "must not send sacred text");
    }

    #[test]
    fn specialist_openai_empty_falls_through_to_ollama() {
        let srv = CompatServer::spawn(CompatScript::OpenAiEmptyThenOllama {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let marker = "empty-openai-then-ollama";
        let result = specialist_via_adapter(&srv.endpoint(), &complete_req(marker)).unwrap();
        assert!(result.allow, "{}", result.reason);
        assert_eq!(result.completion, "ok");
        let (path, body) = srv.last_post().expect("ollama chat POST");
        assert_eq!(path, "/api/chat");
        assert!(body.contains(marker), "{body}");
    }

    #[test]
    fn specialist_openai_and_ollama_empty_refuses_clearly() {
        let srv = CompatServer::spawn(CompatScript::OpenAiEmptyAndOllamaEmpty {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let err = specialist_via_adapter(&srv.endpoint(), &complete_req("hello")).unwrap_err();
        assert!(err.is_local_down(), "{err}");
        let msg = err.to_string();
        assert!(msg.contains("openai="), "{err}");
        assert!(msg.contains("ollama="), "{err}");
        assert!(msg.contains("empty message.content"), "{err}");
        assert!(msg.contains("status 200"), "{err}");
        assert!(msg.contains("model=llama3"), "{err}");
        assert!(msg.contains("CELL_LOCAL_MODEL"), "{err}");
        assert!(msg.contains("ollama pull"), "{err}");
        let (path, _) = srv.last_post().expect("tried ollama after empty openai");
        assert_eq!(path, "/api/chat");
    }

    #[test]
    fn specialist_openai_content_parts_accepted() {
        let srv = CompatServer::spawn(CompatScript::OpenAiContentParts {
            models: vec!["llama3".into()],
        })
        .unwrap();
        let result = specialist_via_adapter(&srv.endpoint(), &complete_req("parts")).unwrap();
        assert_eq!(result.completion, "ok");
        let (path, _) = srv.last_post().expect("openai chat POST");
        assert_eq!(path, "/v1/chat/completions");
    }

    #[test]
    fn llama_cpp_complete_returns_openai_text() {
        let srv = CompatServer::spawn(CompatScript::OpenAi {
            models: vec!["ggml-model".into()],
        })
        .unwrap();
        let local = HttpLocal {
            id: "local_slm".into(),
            endpoint: srv.endpoint(),
            runtime: LocalRuntime::LlamaCpp,
        };
        let result = local.specialist(&complete_req("llamacpp-complete")).unwrap();
        assert_eq!(result.completion, "ok");
        assert_eq!(local.runtime(), LocalRuntime::LlamaCpp);
    }

    #[test]
    fn accepted_completion_refuses_empty_and_sku() {
        assert!(accepted_completion("pong").is_ok());
        assert!(accepted_completion("   ").is_err());
        let err = accepted_completion("llama3-rtx-5090").unwrap_err();
        assert!(err.to_string().contains("SKU"), "{err}");
        assert!(err.is_local_down(), "{err}");
    }

    #[test]
    fn extract_openai_text_accepts_string_parts_text_reasoning() {
        let string = serde_json::json!({"choices":[{"message":{"content":"pong"}}]});
        assert_eq!(extract_openai_text(&string).as_deref(), Some("pong"));
        let parts = serde_json::json!({
            "choices":[{"message":{"content":[{"type":"text","text":"pong"}]}}]
        });
        assert_eq!(extract_openai_text(&parts).as_deref(), Some("pong"));
        let text = serde_json::json!({"choices":[{"message":{"text":"pong"}}]});
        assert_eq!(extract_openai_text(&text).as_deref(), Some("pong"));
        let reason = serde_json::json!({
            "choices":[{"message":{"content":"","reasoning_content":"pong"}}]
        });
        assert_eq!(extract_openai_text(&reason).as_deref(), Some("pong"));
        let empty = serde_json::json!({"choices":[{"message":{"content":"   "}}]});
        assert!(extract_openai_text(&empty).is_none());
        let missing = serde_json::json!({"choices":[{"index":0}]});
        assert!(extract_openai_text(&missing).is_none());
    }
}
