use crate::catalog::LocalRuntime;
use crate::error::ModelError;
use estate_schema::{is_sacred_name, ModelBinding};
use serde::{Deserialize, Serialize};

/// Catalog-level driver probe. Not a live ping. Fail closed at `specialist()`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriverProbe {
    pub driver: String,
    pub runtime: String,
    pub status: String,
    pub host_class: String,
    pub bindable: bool,
    /// False unless `estate probes --live` / `CELL_LIVE_PROBE` and an endpoint answers.
    /// CI never sets this. Missing endpoints are SKIP, not a fail.
    pub live_probed: bool,
    pub note: String,
}

/// Local specialist driver. Hardware is chosen by `runtime()`, not by a SKU in the estate.
pub trait LocalDriver: Send + Sync {
    fn id(&self) -> &str;
    fn specialist(&self, req: &SpecialistRequest) -> Result<SpecialistResult, ModelError>;
    fn runtime(&self) -> LocalRuntime {
        LocalRuntime::HttpRemote
    }
    fn probe(&self) -> DriverProbe {
        probe_runtime(self.id(), self.runtime(), "any")
    }
}

pub fn probe_runtime(id: &str, runtime: LocalRuntime, host_class: &str) -> DriverProbe {
    let (status, bindable, note) = match runtime {
        LocalRuntime::Ollama => (
            "supported",
            true,
            "Ollama-first. Catalog-level probe; not a live ping.",
        ),
        LocalRuntime::LlamaCpp => (
            "swap-proof",
            true,
            "llama.cpp swap-proof sibling. Same specialist protocol.",
        ),
        LocalRuntime::HttpRemote => (
            "supported",
            true,
            "CELL_LOCAL_ENDPOINT remote pattern. Catalog-level probe.",
        ),
        LocalRuntime::Mlx => (
            "stub",
            false,
            "Native MLX specialist() is stub. Live Mac proof is Ollama-on-Mac (or OpenAI-compatible) via the HTTP adapter.",
        ),
        LocalRuntime::Vllm | LocalRuntime::Trt => (
            "experimental",
            false,
            "Experimental until Jason verifies. Fail closed; no frontier fallback.",
        ),
    };
    DriverProbe {
        driver: id.to_string(),
        runtime: runtime.as_str().to_string(),
        status: status.into(),
        host_class: host_class.into(),
        bindable,
        live_probed: false,
        note: note.into(),
    }
}

/// `CELL_LIVE_PROBE=1|true|yes` opts into live HTTP. Unset in CI.
pub fn live_probe_env_requested() -> bool {
    match std::env::var("CELL_LIVE_PROBE") {
        Ok(v) => matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"),
        Err(_) => false,
    }
}

/// Env names a Mac / rented box can set. No SKU in the name.
pub fn live_endpoint_envs(runtime: LocalRuntime) -> &'static [&'static str] {
    match runtime {
        LocalRuntime::Mlx => &["CELL_MLX_ENDPOINT", "CELL_LOCAL_ENDPOINT"],
        LocalRuntime::Vllm => &["CELL_VLLM_ENDPOINT"],
        LocalRuntime::Trt => &["CELL_TRT_ENDPOINT"],
        LocalRuntime::Ollama | LocalRuntime::LlamaCpp | LocalRuntime::HttpRemote => {
            &["CELL_LOCAL_ENDPOINT", "CELL_RENTED_ENDPOINT"]
        }
    }
}

pub fn live_endpoint(runtime: LocalRuntime) -> Option<String> {
    for key in live_endpoint_envs(runtime) {
        if let Ok(v) = std::env::var(key) {
            let v = v.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

pub use crate::adapter::{ping_live_endpoint, specialist_via_adapter};

/// Dry overlay. Tests and the runbook fixture use this so CI never opens a socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveOverlay<'a> {
    Skip,
    WouldLive,
    Down(&'a str),
}

/// Apply SKIP / would-live / down without touching the network.
pub fn apply_live_overlay(mut probe: DriverProbe, overlay: LiveOverlay<'_>) -> DriverProbe {
    match overlay {
        LiveOverlay::Skip => {
            probe.live_probed = false;
            probe.note = format!(
                "{} SKIP (no endpoint env; CI never requires a live box)",
                probe.note
            );
        }
        LiveOverlay::WouldLive => {
            probe.live_probed = true;
            probe.note = format!("{} live ok", probe.note);
        }
        LiveOverlay::Down(err) => {
            probe.live_probed = false;
            probe.note = format!("{} down: {err}", probe.note);
        }
    }
    probe
}

/// Overlay a live result. `None` endpoint is SKIP (exit-safe for CI / Mac-less boxes).
pub fn enrich_with_live(probe: DriverProbe, endpoint: Option<&str>) -> DriverProbe {
    match endpoint {
        None => apply_live_overlay(probe, LiveOverlay::Skip),
        Some(ep) => match ping_live_endpoint(ep) {
            Ok(flavor) => {
                let mut p = apply_live_overlay(probe, LiveOverlay::WouldLive);
                p.note = format!("{} ({})", p.note, flavor.as_str());
                p
            }
            Err(err) => apply_live_overlay(probe, LiveOverlay::Down(&err)),
        },
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SpecialistJob {
    PolicyPrecheck,
    Redact,
}

impl SpecialistJob {
    pub fn as_str(self) -> &'static str {
        match self {
            SpecialistJob::PolicyPrecheck => "policy-precheck",
            SpecialistJob::Redact => "redact",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecialistRequest {
    pub job: SpecialistJob,
    pub agent_id: String,
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecialistResult {
    pub allow: bool,
    pub redacted_text: String,
    pub reason: String,
    pub job: String,
}

pub struct UnwiredLocal {
    pub id: String,
}

impl LocalDriver for UnwiredLocal {
    fn id(&self) -> &str {
        &self.id
    }

    fn specialist(&self, _req: &SpecialistRequest) -> Result<SpecialistResult, ModelError> {
        Err(ModelError::NotWired(self.id.clone()))
    }
}

pub struct MockLocal {
    pub id: String,
}

impl LocalDriver for MockLocal {
    fn id(&self) -> &str {
        &self.id
    }

    fn runtime(&self) -> LocalRuntime {
        LocalRuntime::Ollama
    }

    fn specialist(&self, req: &SpecialistRequest) -> Result<SpecialistResult, ModelError> {
        Ok(builtin_specialist(req))
    }
}

/// Wired local that cannot be reached. `run_task` turns this into an audited deny.
pub struct DownLocal {
    pub id: String,
    pub reason: ModelError,
    pub runtime: LocalRuntime,
}

impl LocalDriver for DownLocal {
    fn id(&self) -> &str {
        &self.id
    }

    fn runtime(&self) -> LocalRuntime {
        self.runtime
    }

    fn specialist(&self, _req: &SpecialistRequest) -> Result<SpecialistResult, ModelError> {
        Err(self.reason.clone())
    }
}

/// Apple MLX stub — same catalog/route/bind API, live Mac proof later.
pub struct MlxDriver {
    pub id: String,
}

impl LocalDriver for MlxDriver {
    fn id(&self) -> &str {
        &self.id
    }

    fn runtime(&self) -> LocalRuntime {
        LocalRuntime::Mlx
    }

    fn specialist(&self, _req: &SpecialistRequest) -> Result<SpecialistResult, ModelError> {
        Err(ModelError::Stub(self.id.clone()))
    }

    fn probe(&self) -> DriverProbe {
        probe_runtime(&self.id, LocalRuntime::Mlx, "apple-silicon")
    }
}

/// vLLM / TRT until Jason verifies. Fail closed; not a silent frontier fallback.
pub struct ExperimentalLocal {
    pub id: String,
    pub runtime: LocalRuntime,
}

impl LocalDriver for ExperimentalLocal {
    fn id(&self) -> &str {
        &self.id
    }

    fn runtime(&self) -> LocalRuntime {
        self.runtime
    }

    fn specialist(&self, _req: &SpecialistRequest) -> Result<SpecialistResult, ModelError> {
        Err(ModelError::Experimental(self.id.clone()))
    }
}

/// Thin HTTP specialist. Used by Ollama, llama.cpp, and http-remote.
/// Factory `/v0/specialist` first; else OpenAI `/v1/chat/completions` or
/// Ollama `/api/chat`. Policy stays `builtin_specialist`.
pub struct HttpLocal {
    pub id: String,
    pub endpoint: String,
    pub runtime: LocalRuntime,
}

impl HttpLocal {
    pub fn new(id: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            endpoint: endpoint.into(),
            runtime: LocalRuntime::HttpRemote,
        }
    }
}

impl LocalDriver for HttpLocal {
    fn id(&self) -> &str {
        &self.id
    }

    fn runtime(&self) -> LocalRuntime {
        self.runtime
    }

    fn specialist(&self, req: &SpecialistRequest) -> Result<SpecialistResult, ModelError> {
        specialist_via_adapter(&self.endpoint, req)
    }
}

/// Prefer [`crate::catalog::bind_local`]. Kept as the data-plane alias.
pub fn local_from_binding(binding: &ModelBinding) -> Result<Box<dyn LocalDriver>, ModelError> {
    crate::catalog::bind_local(binding)
}

/// Bounded specialist used by mock-local and MockLocal.
/// Same protocol every host class (RTX / Apple / rented Nvidia) should speak.
pub fn builtin_specialist(req: &SpecialistRequest) -> SpecialistResult {
    const MAX: usize = 16 * 1024;
    if req.text.len() > MAX {
        return SpecialistResult {
            allow: false,
            redacted_text: String::new(),
            reason: "payload exceeds 16KiB bound".into(),
            job: req.job.as_str().into(),
        };
    }
    let lower = req.text.to_ascii_lowercase();
    for token in ["cyera", "rust-classroom", "rust_classroom"] {
        if lower.contains(token) || is_sacred_name(token) && lower.contains(token) {
            return SpecialistResult {
                allow: false,
                redacted_text: String::new(),
                reason: format!("policy-precheck denied sacred token '{token}'"),
                job: req.job.as_str().into(),
            };
        }
    }
    let redacted = redact_secrets(&req.text);
    SpecialistResult {
        allow: true,
        redacted_text: redacted,
        reason: "policy-precheck allow".into(),
        job: req.job.as_str().into(),
    }
}

fn redact_secrets(text: &str) -> String {
    let mut out = String::new();
    for word in text.split_inclusive([' ', '\n', '\t', ',', ';']) {
        let core = word.trim_end_matches([' ', '\n', '\t', ',', ';']);
        if looks_secret(core) {
            let sep = &word[core.len()..];
            out.push_str("[redacted]");
            out.push_str(sep);
        } else {
            out.push_str(word);
        }
    }
    out
}

fn looks_secret(token: &str) -> bool {
    let t = token.trim();
    (t.starts_with("sk-") || t.starts_with("xai-") || t.starts_with("xai_")) && t.len() >= 12
}
