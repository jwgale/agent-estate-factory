use crate::local::{builtin_specialist, SpecialistJob, SpecialistRequest, SpecialistResult};
use std::io::Read;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub struct MockLocalServer {
    pub addr: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl MockLocalServer {
    pub fn spawn() -> Result<Self, String> {
        let server = tiny_http::Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
        let addr = match server.server_addr() {
            tiny_http::ListenAddr::IP(a) => a,
            _ => return Err("expected ip listen addr".into()),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                match server.recv_timeout(std::time::Duration::from_millis(50)) {
                    Ok(Some(mut request)) => {
                        let path = request_path(&request);
                        if request.method().as_str() == "GET" && path == "/v1/models" {
                            let _ = request.respond(json_ok(
                                r#"{"object":"list","data":[]}"#,
                            ));
                            continue;
                        }
                        let mut body = String::new();
                        let _ = std::io::Read::read_to_string(request.as_reader(), &mut body);
                        let reply = handle_specialist(&body);
                        let _ = request.respond(json_ok(&reply));
                    }
                    _ => {}
                }
            }
        });
        Ok(Self {
            addr,
            stop,
            handle: Some(handle),
        })
    }

    pub fn endpoint(&self) -> String {
        format!("http://{}", self.addr)
    }
}

impl Drop for MockLocalServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

pub fn serve_specialist_forever(bind: &str) -> Result<(), String> {
    let server = tiny_http::Server::http(bind).map_err(|e| e.to_string())?;
    eprintln!("model-estate mock-local listening on http://{bind}  GET /v1/models  POST /v0/specialist");
    for mut request in server.incoming_requests() {
        let path = request_path(&request);
        if request.method().as_str() == "GET" && path == "/v1/models" {
            let _ = request.respond(json_ok(r#"{"object":"list","data":[]}"#));
            continue;
        }
        let mut body = String::new();
        let _ = Read::read_to_string(request.as_reader(), &mut body);
        let reply = handle_specialist(&body);
        let _ = request.respond(json_ok(&reply));
    }
    Ok(())
}

fn request_path(request: &tiny_http::Request) -> String {
    request.url().split('?').next().unwrap_or("/").to_string()
}

fn json_ok(body: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let mut response = tiny_http::Response::from_string(body.to_string()).with_status_code(200);
    if let Ok(h) = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        response = response.with_header(h);
    }
    response
}

fn json_status(status: u16, body: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let mut response = tiny_http::Response::from_string(body.to_string()).with_status_code(status);
    if let Ok(h) = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        response = response.with_header(h);
    }
    response
}

/// In-process OpenAI / Ollama stand-in for adapter tests. Not a GPU.
#[derive(Debug, Clone)]
pub enum CompatScript {
    OpenAi { models: Vec<String> },
    Ollama { models: Vec<String> },
    Garbage,
    EmptyBody,
}

pub struct CompatServer {
    pub addr: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl CompatServer {
    pub fn spawn(script: CompatScript) -> Result<Self, String> {
        let server = tiny_http::Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
        let addr = match server.server_addr() {
            tiny_http::ListenAddr::IP(a) => a,
            _ => return Err("expected ip listen addr".into()),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                match server.recv_timeout(std::time::Duration::from_millis(50)) {
                    Ok(Some(request)) => {
                        let reply = compat_reply(&script, request.method().as_str(), &request_path(&request));
                        let _ = request.respond(reply);
                    }
                    _ => {}
                }
            }
        });
        Ok(Self {
            addr,
            stop,
            handle: Some(handle),
        })
    }

    pub fn endpoint(&self) -> String {
        format!("http://{}", self.addr)
    }
}

impl Drop for CompatServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn compat_reply(
    script: &CompatScript,
    method: &str,
    path: &str,
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let get = method == "GET";
    let post = method == "POST";
    match script {
        CompatScript::Garbage => json_ok(r#"{"ok":true}"#),
        CompatScript::EmptyBody => json_ok(""),
        CompatScript::OpenAi { models } => {
            if get && path == "/v1/models" {
                return json_ok(&openai_models_json(models));
            }
            if post && path == "/v1/chat/completions" {
                return json_ok(r#"{"choices":[{"message":{"content":"ok"}}]}"#);
            }
            json_status(404, r#"{"error":"not found"}"#)
        }
        CompatScript::Ollama { models } => {
            if get && path == "/api/tags" {
                return json_ok(&ollama_tags_json(models));
            }
            if post && path == "/api/chat" {
                return json_ok(r#"{"message":{"role":"assistant","content":"ok"}}"#);
            }
            json_status(404, r#"{"error":"not found"}"#)
        }
    }
}

fn openai_models_json(models: &[String]) -> String {
    let data: Vec<serde_json::Value> = models
        .iter()
        .map(|id| serde_json::json!({"id": id, "object": "model"}))
        .collect();
    serde_json::json!({"object":"list","data": data}).to_string()
}

fn ollama_tags_json(models: &[String]) -> String {
    let rows: Vec<serde_json::Value> = models
        .iter()
        .map(|name| serde_json::json!({"name": name}))
        .collect();
    serde_json::json!({"models": rows}).to_string()
}

fn handle_specialist(body: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::json!({}));
    let job = match v.get("job").and_then(|j| j.as_str()).unwrap_or("policy-precheck") {
        "redact" => SpecialistJob::Redact,
        _ => SpecialistJob::PolicyPrecheck,
    };
    let req = SpecialistRequest {
        job,
        agent_id: v
            .get("agent_id")
            .and_then(|a| a.as_str())
            .unwrap_or("unknown")
            .to_string(),
        kind: v
            .get("kind")
            .and_then(|a| a.as_str())
            .unwrap_or("model")
            .to_string(),
        text: v
            .get("text")
            .and_then(|a| a.as_str())
            .unwrap_or("")
            .to_string(),
    };
    let result: SpecialistResult = builtin_specialist(&req);
    match serde_json::to_string_pretty(&result) {
        Ok(s) if !s.trim().is_empty() && s.trim() != "{}" => s,
        _ => {
            r#"{"allow":false,"redacted_text":"","reason":"serialize","job":"policy-precheck"}"#
                .into()
        }
    }
}

pub struct MockFrontierServer {
    pub addr: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl MockFrontierServer {
    pub fn spawn(reply: &str) -> Result<Self, String> {
        let reply = reply.to_string();
        let server = tiny_http::Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
        let addr = match server.server_addr() {
            tiny_http::ListenAddr::IP(a) => a,
            _ => return Err("expected ip listen addr".into()),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                match server.recv_timeout(std::time::Duration::from_millis(50)) {
                    Ok(Some(mut request)) => {
                        let mut _body = String::new();
                        let _ = std::io::Read::read_to_string(request.as_reader(), &mut _body);
                        let body = serde_json::json!({
                            "choices": [{"message": {"content": reply}}]
                        });
                        let mut response = tiny_http::Response::from_string(body.to_string())
                            .with_status_code(200);
                        if let Ok(h) = tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"application/json"[..],
                        ) {
                            response = response.with_header(h);
                        }
                        let _ = request.respond(response);
                    }
                    _ => {}
                }
            }
        });
        Ok(Self {
            addr,
            stop,
            handle: Some(handle),
        })
    }

    pub fn base(&self) -> String {
        format!("http://{}", self.addr)
    }
}

impl Drop for MockFrontierServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}
