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
                        let mut body = String::new();
                        let _ = std::io::Read::read_to_string(request.as_reader(), &mut body);
                        let reply = handle_specialist(&body);
                        let mut response =
                            tiny_http::Response::from_string(reply).with_status_code(200);
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
    eprintln!("model-estate mock-local listening on http://{bind}  POST /v0/specialist");
    for mut request in server.incoming_requests() {
        let mut body = String::new();
        let _ = Read::read_to_string(request.as_reader(), &mut body);
        let reply = handle_specialist(&body);
        let mut response = tiny_http::Response::from_string(reply).with_status_code(200);
        if let Ok(h) =
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        {
            response = response.with_header(h);
        }
        let _ = request.respond(response);
    }
    Ok(())
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
