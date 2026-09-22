//! Portable local-runtime catalog / route / bind.
//!
//! Hardware is a **driver choice**, not a product fork. The same estate
//! contract works on (a) consumer-grade RTX, (b) Apple Silicon, (c) rented
//! latest Nvidia. Do not put SKUs (`5090`, `4090`, `m3-max`, …) in binding
//! ids or drivers.
//!
//! Locked Cell One defaults (do not reopen):
//! 1. Ollama-first; llama.cpp is swap-proof; vLLM is optional.
//! 2. Local process on a host + `CELL_LOCAL_ENDPOINT` remote pattern.
//! 3. Fail closed when estate-bound local is down — no silent frontier fallback.
//! 4. Jason curates the first specialist enrich packs (manual).
//! 5. "Supported" = Ollama (+ llama.cpp) green on the box. vLLM / TRT stay
//!    experimental until he verifies.
//!
//! Apple path: Ollama-on-Mac is the Supported Apple driver (same `ollama`
//! card). MLX is a Stub behind this same API; live Mac proof may come later.
//!
//! This crate is **not** an LM Studio / weight browser / chat UI. Bindings
//! speak a thin specialist protocol (`POST /v0/specialist`).
//! Frontier complete (A7) defaults to model `grok-4.7` (`CELL_FRONTIER_MODEL`
//! or `XAI_MODEL`). Local down does not fall through to that path.

use crate::error::ModelError;
use crate::frontier::param_str;
use crate::local::{
    probe_runtime, DownLocal, DriverProbe, ExperimentalLocal, HttpLocal, LocalDriver, MlxDriver,
    UnwiredLocal,
};
use estate_schema::ModelBinding;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalRuntime {
    Ollama,
    LlamaCpp,
    Mlx,
    Vllm,
    Trt,
    HttpRemote,
}

impl LocalRuntime {
    pub fn as_str(self) -> &'static str {
        match self {
            LocalRuntime::Ollama => "ollama",
            LocalRuntime::LlamaCpp => "llama.cpp",
            LocalRuntime::Mlx => "mlx",
            LocalRuntime::Vllm => "vllm",
            LocalRuntime::Trt => "trt",
            LocalRuntime::HttpRemote => "http-remote",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportStatus {
    /// Green on the box via the Ollama (+ llama.cpp) path.
    Supported,
    /// llama.cpp: swap-proof sibling of Ollama, same specialist protocol.
    SwapProof,
    /// vLLM / TRT — optional, not required for the first green demo.
    Experimental,
    /// Documented interface; live proof later (MLX on Apple Silicon).
    Stub,
}

impl SupportStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SupportStatus::Supported => "supported",
            SupportStatus::SwapProof => "swap-proof",
            SupportStatus::Experimental => "experimental",
            SupportStatus::Stub => "stub",
        }
    }
}

/// Portable host class. Not a SKU. Not a product fork.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostClass {
    ConsumerNvidia,
    AppleSilicon,
    RentedNvidia,
    Any,
}

impl HostClass {
    pub fn as_str(self) -> &'static str {
        match self {
            HostClass::ConsumerNvidia => "consumer-nvidia",
            HostClass::AppleSilicon => "apple-silicon",
            HostClass::RentedNvidia => "rented-nvidia",
            HostClass::Any => "any",
        }
    }
}

/// Per-driver capability flags. Stub drivers stay flag-complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DriverCaps {
    pub streaming: bool,
    pub tools: bool,
    pub vision: bool,
    pub context_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogCard {
    pub runtime: LocalRuntime,
    pub driver_id: &'static str,
    pub status: SupportStatus,
    pub hosts: &'static [HostClass],
    pub caps: DriverCaps,
    pub notes: &'static str,
}

pub const CATALOG: &[CatalogCard] = &[
    CatalogCard {
        runtime: LocalRuntime::Ollama,
        driver_id: "ollama",
        status: SupportStatus::Supported,
        hosts: &[
            HostClass::ConsumerNvidia,
            HostClass::AppleSilicon,
            HostClass::RentedNvidia,
            HostClass::Any,
        ],
        caps: DriverCaps {
            streaming: true,
            tools: true,
            vision: true,
            context_tokens: 8192,
        },
        notes: "First green local path. Ollama-on-Linux and Ollama-on-Mac share this card.",
    },
    CatalogCard {
        runtime: LocalRuntime::LlamaCpp,
        driver_id: "llama.cpp",
        status: SupportStatus::SwapProof,
        hosts: &[
            HostClass::ConsumerNvidia,
            HostClass::AppleSilicon,
            HostClass::RentedNvidia,
            HostClass::Any,
        ],
        caps: DriverCaps {
            streaming: true,
            tools: false,
            vision: false,
            context_tokens: 4096,
        },
        notes: "Swap-proof sibling of Ollama. Same /v0/specialist protocol, not a second product.",
    },
    CatalogCard {
        runtime: LocalRuntime::Mlx,
        driver_id: "mlx",
        status: SupportStatus::Stub,
        hosts: &[HostClass::AppleSilicon],
        caps: DriverCaps {
            streaming: true,
            tools: false,
            vision: false,
            context_tokens: 8192,
        },
        notes: "Apple Silicon stub. Same catalog/route/bind API; live Mac proof later.",
    },
    CatalogCard {
        runtime: LocalRuntime::Vllm,
        driver_id: "vllm",
        status: SupportStatus::Experimental,
        hosts: &[HostClass::ConsumerNvidia, HostClass::RentedNvidia],
        caps: DriverCaps {
            streaming: true,
            tools: true,
            vision: false,
            context_tokens: 32768,
        },
        notes: "Optional. Not required for the first green demo. Experimental until Jason verifies.",
    },
    CatalogCard {
        runtime: LocalRuntime::Trt,
        driver_id: "trt",
        status: SupportStatus::Experimental,
        hosts: &[HostClass::ConsumerNvidia, HostClass::RentedNvidia],
        caps: DriverCaps {
            streaming: true,
            tools: false,
            vision: false,
            context_tokens: 8192,
        },
        notes: "TensorRT-LLM. Experimental until Jason verifies.",
    },
    CatalogCard {
        runtime: LocalRuntime::HttpRemote,
        driver_id: "http-remote",
        status: SupportStatus::Supported,
        hosts: &[HostClass::Any],
        caps: DriverCaps {
            streaming: false,
            tools: true,
            vision: false,
            context_tokens: 8192,
        },
        notes: "CELL_LOCAL_ENDPOINT remote pattern. Same protocol on any host class.",
    },
];

pub fn catalog() -> &'static [CatalogCard] {
    CATALOG
}

/// Catalog-level probes. Not live pings. Control may print these.
pub fn catalog_probes() -> Vec<DriverProbe> {
    CATALOG
        .iter()
        .map(|c| {
            let host = if c.hosts.len() == 1 {
                c.hosts[0].as_str()
            } else {
                "any"
            };
            probe_runtime(c.driver_id, c.runtime, host)
        })
        .collect()
}

/// Optional live HTTP overlay. Missing endpoints stay SKIP. Never fails closed for CI.
pub fn catalog_probes_live() -> Vec<DriverProbe> {
    catalog_probes()
        .into_iter()
        .map(|p| {
            let runtime = parse_runtime(&p.driver).unwrap_or(LocalRuntime::HttpRemote);
            let endpoint = crate::local::live_endpoint(runtime);
            crate::local::enrich_with_live(p, endpoint.as_deref())
        })
        .collect()
}

pub fn card(runtime: LocalRuntime) -> &'static CatalogCard {
    CATALOG
        .iter()
        .find(|c| c.runtime == runtime)
        .expect("catalog covers every LocalRuntime")
}

pub fn parse_runtime(driver: &str) -> Option<LocalRuntime> {
    match driver.trim().to_ascii_lowercase().as_str() {
        "ollama" => Some(LocalRuntime::Ollama),
        "llama.cpp" | "llamacpp" | "llama-cpp" => Some(LocalRuntime::LlamaCpp),
        "mlx" => Some(LocalRuntime::Mlx),
        "vllm" => Some(LocalRuntime::Vllm),
        "trt" | "tensorrt" | "trt-llm" | "tensorrt-llm" => Some(LocalRuntime::Trt),
        "http-remote" | "http" | "local-http" => Some(LocalRuntime::HttpRemote),
        _ => None,
    }
}

pub fn parse_host_class(raw: &str) -> Option<HostClass> {
    match estate_schema::normalize_host_class(raw)? {
        "consumer-nvidia" => Some(HostClass::ConsumerNvidia),
        "apple-silicon" => Some(HostClass::AppleSilicon),
        "rented-nvidia" => Some(HostClass::RentedNvidia),
        "any" => Some(HostClass::Any),
        _ => None,
    }
}

/// Map a binding's driver string onto a catalog card. SKU is not an input.
pub fn route(binding: &ModelBinding) -> Result<&'static CatalogCard, ModelError> {
    let runtime = parse_runtime(&binding.driver).ok_or_else(|| {
        ModelError::Other(format!(
            "unknown local driver '{}'; catalog runtimes: ollama, llama.cpp, mlx, vllm, trt, http-remote",
            binding.driver
        ))
    })?;
    Ok(card(runtime))
}

/// Bind a local catalog card to a driver. Missing endpoint → DownLocal (fail closed later).
pub fn bind_local(binding: &ModelBinding) -> Result<Box<dyn LocalDriver>, ModelError> {
    if !binding.wired {
        return Ok(Box::new(UnwiredLocal {
            id: binding.id.clone(),
        }));
    }
    let card = route(binding)?;
    if let Some(raw) = param_str(binding, "host_class") {
        let host = parse_host_class(&raw).ok_or_else(|| {
            ModelError::Other(format!(
                "unknown host_class '{raw}'; use consumer-nvidia|apple-silicon|rented-nvidia|any"
            ))
        })?;
        if host != HostClass::Any
            && !card.hosts.contains(&host)
            && !card.hosts.contains(&HostClass::Any)
        {
            return Err(ModelError::Other(format!(
                "host_class {} is not on catalog card {} (hosts: {})",
                host.as_str(),
                card.driver_id,
                card.hosts
                    .iter()
                    .map(|h| h.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            )));
        }
    }
    match card.runtime {
        LocalRuntime::Mlx => Ok(Box::new(MlxDriver {
            id: binding.id.clone(),
        })),
        LocalRuntime::Vllm | LocalRuntime::Trt => Ok(Box::new(ExperimentalLocal {
            id: binding.id.clone(),
            runtime: card.runtime,
        })),
        LocalRuntime::Ollama | LocalRuntime::LlamaCpp | LocalRuntime::HttpRemote => {
            let env_name =
                param_str(binding, "endpoint_env").unwrap_or_else(|| "CELL_LOCAL_ENDPOINT".into());
            match std::env::var(&env_name)
                .ok()
                .or_else(|| param_str(binding, "endpoint"))
            {
                Some(endpoint) => Ok(Box::new(HttpLocal {
                    id: binding.id.clone(),
                    endpoint,
                    runtime: card.runtime,
                })),
                None => Ok(Box::new(DownLocal {
                    id: binding.id.clone(),
                    reason: ModelError::MissingEndpoint(env_name),
                    runtime: card.runtime,
                })),
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CatalogFileCard {
    pub driver_id: String,
    pub runtime: String,
    pub status: String,
    pub hosts: Vec<String>,
    pub caps: DriverCaps,
    pub notes: String,
}

/// Equal-class frontier card. Not a local runtime and not a live probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontierCard {
    pub driver_id: &'static str,
    pub model: &'static str,
    pub status: &'static str,
    pub caps: FrontierCaps,
    pub notes: &'static str,
}

/// What the factory frontier chat actually sends. Not a model context window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrontierCaps {
    pub streaming: bool,
    pub tools: bool,
    pub vision: bool,
    pub completion_tokens: u32,
}

pub const FRONTIER_CARD: FrontierCard = FrontierCard {
    driver_id: "frontier",
    model: crate::frontier::DEFAULT_FRONTIER_MODEL,
    status: "equal-class",
    caps: FrontierCaps {
        streaming: false,
        tools: false,
        vision: false,
        completion_tokens: crate::frontier::FRONTIER_COMPLETION_TOKENS,
    },
    notes: "A7 chat. Model grok-4.7. Needs XAI_API_KEY. No stream, tools, or vision. reasoning_effort xhigh is the cloud-agent standing default and is not sent. Local down does not fall through.",
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct FrontierCatalogCard {
    pub driver_id: String,
    pub model: String,
    pub status: String,
    pub caps: FrontierCaps,
    pub notes: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CatalogFile {
    pub schema: String,
    pub note: String,
    pub hosts: Vec<String>,
    pub cards: Vec<CatalogFileCard>,
    pub frontier: FrontierCatalogCard,
}

/// File SoT for the portable local catalog. Hardware is a driver choice.
pub fn catalog_file() -> CatalogFile {
    CatalogFile {
        schema: "cell-one.local-catalog.v0".into(),
        note: "Hardware is a driver choice, not a product fork. Not an LM Studio. supported = Ollama (+ llama.cpp) green on the box.".into(),
        hosts: vec![
            HostClass::ConsumerNvidia.as_str().into(),
            HostClass::AppleSilicon.as_str().into(),
            HostClass::RentedNvidia.as_str().into(),
            HostClass::Any.as_str().into(),
        ],
        cards: CATALOG
            .iter()
            .map(|card| CatalogFileCard {
                driver_id: card.driver_id.to_string(),
                runtime: card.runtime.as_str().to_string(),
                status: card.status.as_str().to_string(),
                hosts: card.hosts.iter().map(|h| h.as_str().to_string()).collect(),
                caps: card.caps,
                notes: card.notes.to_string(),
            })
            .collect(),
        frontier: FrontierCatalogCard {
            driver_id: FRONTIER_CARD.driver_id.to_string(),
            model: FRONTIER_CARD.model.to_string(),
            status: FRONTIER_CARD.status.to_string(),
            caps: FRONTIER_CARD.caps,
            notes: FRONTIER_CARD.notes.to_string(),
        },
    }
}

pub fn write_catalog(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    write_catalog_file(path, &catalog_file())
}

/// Schema dump must not replace a catalog whose frontier model is not the
/// schema card. A missing file is not a disagreement. The schema card is
/// not a binding.
pub fn refuse_schema_catalog_overwrite(path: &std::path::Path) -> Result<(), crate::ModelError> {
    if !path.is_file() {
        return Ok(());
    }
    let text = std::fs::read_to_string(path).map_err(|err| {
        crate::ModelError::Other(format!(
            "refuse:frontier-model: catalog unreadable ({err}); schema card is not the binding"
        ))
    })?;
    let existing = crate::frontier_model_from_catalog_json(&text).map_err(|err| {
        crate::ModelError::Other(format!(
            "refuse:frontier-model: catalog unreadable ({err}); schema card is not the binding"
        ))
    })?;
    let norm = |value: Option<&str>| {
        value
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    };
    let existing = norm(existing.as_deref());
    let schema = norm(Some(catalog_file().frontier.model.as_str()));
    if existing == schema {
        return Ok(());
    }
    fn show(value: &Option<String>) -> &str {
        value.as_deref().unwrap_or("-")
    }
    Err(crate::ModelError::Other(format!(
        "refuse:frontier-model: will not overwrite catalog model={} with schema card model={}; schema card is not the binding",
        show(&existing),
        show(&schema)
    )))
}

/// Cell catalog for an apply/resume. Frontier model is the binding's
/// `params.model`, or empty when the binding does not set one. Does not
/// copy the schema card default `grok-4.7`.
pub fn catalog_bound_to_estate(estate: &estate_schema::Estate) -> Result<CatalogFile, ModelError> {
    let mut file = catalog_file();
    let shown = match crate::bound_frontier_model(estate)? {
        Some(model) => model,
        None => String::new(),
    };
    file.frontier.model = shown.clone();
    let label = if shown.is_empty() { "-" } else { shown.as_str() };
    file.frontier.notes = format!(
        "A7 chat. Bound model {label}. Needs XAI_API_KEY. No stream, tools, or vision. reasoning_effort xhigh is docs-only and is not sent. Local down does not fall through."
    );
    Ok(file)
}

pub fn write_bound_catalog(
    path: &std::path::Path,
    estate: &estate_schema::Estate,
) -> std::io::Result<std::path::PathBuf> {
    let file = catalog_bound_to_estate(estate).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
    })?;
    write_catalog_file(path, &file)
}

fn write_catalog_file(path: &std::path::Path, file: &CatalogFile) -> std::io::Result<std::path::PathBuf> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let body = serde_json::to_string_pretty(file).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    })?;
    std::fs::write(path, body)?;
    Ok(path.to_path_buf())
}

pub fn render_catalog() -> String {
    let mut lines = vec![
        "local runtime catalog (hardware is a driver choice, not a product fork):".to_string(),
        "  hosts: consumer-nvidia | apple-silicon | rented-nvidia | any".to_string(),
        "  supported = Ollama (+ llama.cpp) green on the box. Not an LM Studio.".to_string(),
    ];
    for card in CATALOG {
        let hosts = card
            .hosts
            .iter()
            .map(|h| h.as_str())
            .collect::<Vec<_>>()
            .join(",");
        lines.push(format!(
            "  {:<12} status={:<12} streaming={} tools={} vision={} context={} hosts={:<32} {}",
            card.driver_id,
            card.status.as_str(),
            card.caps.streaming,
            card.caps.tools,
            card.caps.vision,
            card.caps.context_tokens,
            hosts,
            card.notes
        ));
    }
    lines.push(format!(
        "  schema {:<12} model={:<12} status={:<12} streaming={} tools={} vision={} completion={} {} (schema card, not a binding)",
        FRONTIER_CARD.driver_id,
        FRONTIER_CARD.model,
        FRONTIER_CARD.status,
        FRONTIER_CARD.caps.streaming,
        FRONTIER_CARD.caps.tools,
        FRONTIER_CARD.caps.vision,
        FRONTIER_CARD.caps.completion_tokens,
        FRONTIER_CARD.notes
    ));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local::{SpecialistJob, SpecialistRequest};
    use estate_schema::ModelClass;

    fn req() -> SpecialistRequest {
        SpecialistRequest {
            job: SpecialistJob::PolicyPrecheck,
            agent_id: "horizon".into(),
            kind: "model".into(),
            text: "ping".into(),
        }
    }

    #[test]
    fn ollama_first_llama_swap_proof_rest_not_required() {
        assert_eq!(card(LocalRuntime::Ollama).status, SupportStatus::Supported);
        assert_eq!(card(LocalRuntime::LlamaCpp).status, SupportStatus::SwapProof);
        assert_eq!(card(LocalRuntime::Mlx).status, SupportStatus::Stub);
        assert_eq!(card(LocalRuntime::Vllm).status, SupportStatus::Experimental);
        assert_eq!(card(LocalRuntime::Trt).status, SupportStatus::Experimental);
        assert!(card(LocalRuntime::Ollama)
            .hosts
            .contains(&HostClass::ConsumerNvidia));
        assert!(card(LocalRuntime::Ollama)
            .hosts
            .contains(&HostClass::AppleSilicon));
        assert!(card(LocalRuntime::Ollama)
            .hosts
            .contains(&HostClass::RentedNvidia));
        assert_eq!(card(LocalRuntime::Mlx).hosts, &[HostClass::AppleSilicon]);
        for c in CATALOG {
            assert!(c.caps.context_tokens > 0, "{}", c.driver_id);
        }
        assert!(card(LocalRuntime::Ollama).caps.tools);
        assert!(card(LocalRuntime::Mlx).caps.streaming);
        assert!(card(LocalRuntime::Vllm).caps.tools);
        assert_eq!(card(LocalRuntime::Trt).status, SupportStatus::Experimental);
    }

    #[test]
    fn parse_host_class_aliases() {
        assert_eq!(
            parse_host_class("rtx_consumer"),
            Some(HostClass::ConsumerNvidia)
        );
        assert_eq!(
            parse_host_class("rtx-consumer"),
            Some(HostClass::ConsumerNvidia)
        );
        assert_eq!(
            parse_host_class("nvidia_rental"),
            Some(HostClass::RentedNvidia)
        );
        assert_eq!(
            parse_host_class("apple_silicon"),
            Some(HostClass::AppleSilicon)
        );
        assert!(parse_host_class("not-a-host").is_none());
        assert!(parse_host_class("rtx-5090").is_none());
    }

    #[test]
    fn parse_runtime_aliases() {
        assert_eq!(parse_runtime("ollama"), Some(LocalRuntime::Ollama));
        assert_eq!(parse_runtime("llama.cpp"), Some(LocalRuntime::LlamaCpp));
        assert_eq!(parse_runtime("llama-cpp"), Some(LocalRuntime::LlamaCpp));
        assert_eq!(parse_runtime("mlx"), Some(LocalRuntime::Mlx));
        assert_eq!(parse_runtime("vllm"), Some(LocalRuntime::Vllm));
        assert_eq!(parse_runtime("trt-llm"), Some(LocalRuntime::Trt));
        assert_eq!(parse_runtime("http-remote"), Some(LocalRuntime::HttpRemote));
        assert!(parse_runtime("local-gpu").is_none());
        assert!(parse_runtime("nvidia-5090").is_none());
    }

    #[test]
    fn bind_missing_endpoint_is_down_driver() {
        let binding = ModelBinding {
            id: "local_slm".into(),
            class: ModelClass::Local,
            driver: "ollama".into(),
            params: serde_json::json!({"endpoint_env": "CELL_LOCAL_ENDPOINT_ABSENT_FOR_TEST"}),
            wired: true,
        };
        let driver = bind_local(&binding).unwrap();
        let err = driver.specialist(&req()).unwrap_err();
        assert!(err.is_local_down());
        assert_eq!(driver.runtime(), LocalRuntime::Ollama);
    }

    #[test]
    fn bind_mlx_is_stub() {
        let binding = ModelBinding {
            id: "local_slm".into(),
            class: ModelClass::Local,
            driver: "mlx".into(),
            params: serde_json::json!({}),
            wired: true,
        };
        let driver = bind_local(&binding).unwrap();
        let err = driver.specialist(&req()).unwrap_err();
        assert!(matches!(err, ModelError::Stub(_)));
        assert!(err.is_local_down());
        assert_eq!(driver.runtime(), LocalRuntime::Mlx);
        let probe = driver.probe();
        assert!(!probe.bindable);
        assert!(!probe.live_probed);
        assert_eq!(probe.host_class, "apple-silicon");
    }

    #[test]
    fn bind_http_remote_uses_cell_local_endpoint() {
        let binding = ModelBinding {
            id: "local_slm".into(),
            class: ModelClass::Local,
            driver: "http-remote".into(),
            params: serde_json::json!({
                "endpoint_env": "CELL_LOCAL_ENDPOINT_ABSENT_FOR_TEST",
                "host_class": "any"
            }),
            wired: true,
        };
        let driver = bind_local(&binding).unwrap();
        assert_eq!(driver.runtime(), LocalRuntime::HttpRemote);
        assert!(driver.specialist(&req()).unwrap_err().is_local_down());
    }

    #[test]
    fn bind_mlx_rejects_consumer_nvidia_host() {
        let binding = ModelBinding {
            id: "local_slm".into(),
            class: ModelClass::Local,
            driver: "mlx".into(),
            params: serde_json::json!({"host_class": "consumer-nvidia"}),
            wired: true,
        };
        let err = match bind_local(&binding) {
            Ok(_) => panic!("mlx + consumer-nvidia should fail closed"),
            Err(e) => e,
        };
        assert!(err.to_string().contains("host_class"));
    }

    #[test]
    fn catalog_probes_are_not_live_pings() {
        let probes = catalog_probes();
        assert_eq!(probes.len(), CATALOG.len());
        assert!(probes.iter().all(|p| !p.live_probed));
        let mlx = probes.iter().find(|p| p.driver == "mlx").unwrap();
        assert!(!mlx.bindable);
        assert_eq!(mlx.status, "stub");
        let ollama = probes.iter().find(|p| p.driver == "ollama").unwrap();
        assert!(ollama.bindable);
        assert_eq!(ollama.status, "supported");
    }

    #[test]
    fn live_probe_skips_without_endpoint() {
        let p = crate::probe_runtime("mlx", LocalRuntime::Mlx, "apple-silicon");
        let p = crate::enrich_with_live(p, None);
        assert!(!p.live_probed);
        assert!(p.note.contains("SKIP"));
    }

    #[test]
    fn live_probe_shapes_skip_vs_would_live_without_network() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../schema/live-probe-shapes.v0.json"))
                .unwrap();
        assert_eq!(fixture["schema"], "cell-one.live-probe-shape.v0");

        let mlx = catalog_probes()
            .into_iter()
            .find(|p| p.driver == fixture["mac_mlx"]["driver"])
            .unwrap();
        assert_eq!(mlx.host_class, fixture["mac_mlx"]["host_class"]);
        assert_eq!(mlx.status, fixture["mac_mlx"]["status"]);
        assert_eq!(mlx.bindable, fixture["mac_mlx"]["bindable"]);
        let mlx_envs: Vec<&str> = fixture["mac_mlx"]["endpoint_envs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(crate::live_endpoint_envs(LocalRuntime::Mlx), mlx_envs.as_slice());

        let ollama = catalog_probes()
            .into_iter()
            .find(|p| p.driver == fixture["linux_gpu"]["driver"])
            .unwrap();
        assert_eq!(ollama.host_class, fixture["linux_gpu"]["host_class"]);
        assert_eq!(ollama.status, fixture["linux_gpu"]["status"]);
        assert!(ollama.bindable);
        let gpu_envs: Vec<&str> = fixture["linux_gpu"]["endpoint_envs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(
            crate::live_endpoint_envs(LocalRuntime::Ollama),
            gpu_envs.as_slice()
        );

        let skip = crate::apply_live_overlay(mlx.clone(), crate::LiveOverlay::Skip);
        assert_eq!(skip.live_probed, fixture["skip"]["live_probed"]);
        for needle in fixture["skip"]["note_must_contain"].as_array().unwrap() {
            assert!(
                skip.note.contains(needle.as_str().unwrap()),
                "SKIP note missing {}: {}",
                needle,
                skip.note
            );
        }

        let would = crate::apply_live_overlay(ollama.clone(), crate::LiveOverlay::WouldLive);
        assert_eq!(would.live_probed, fixture["would_live"]["live_probed"]);
        for needle in fixture["would_live"]["note_must_contain"].as_array().unwrap() {
            assert!(
                would.note.contains(needle.as_str().unwrap()),
                "would-live note missing {}: {}",
                needle,
                would.note
            );
        }

        let down = crate::apply_live_overlay(mlx, crate::LiveOverlay::Down("fixture"));
        assert_eq!(down.live_probed, fixture["down"]["live_probed"]);
        for needle in fixture["down"]["note_must_contain"].as_array().unwrap() {
            assert!(
                down.note.contains(needle.as_str().unwrap()),
                "down note missing {}: {}",
                needle,
                down.note
            );
        }
        assert!(!would.note.contains("SKIP"));
        assert!(!skip.live_probed);
    }

    #[test]
    fn live_probe_ok_against_mock_and_down_is_safe() {
        let server = crate::MockLocalServer::spawn().unwrap();
        let p = crate::probe_runtime("ollama", LocalRuntime::Ollama, "any");
        let p = crate::enrich_with_live(p, Some(&server.endpoint()));
        assert!(p.live_probed, "{}", p.note);
        assert!(p.note.contains("live ok"), "{}", p.note);
        assert!(p.note.contains("/v1/models"), "{}", p.note);
        let down = crate::enrich_with_live(
            crate::probe_runtime("mlx", LocalRuntime::Mlx, "apple-silicon"),
            Some("http://127.0.0.1:1"),
        );
        assert!(!down.live_probed);
        assert!(down.note.contains("down"));
    }

    #[test]
    fn catalog_probes_live_skip_without_endpoint_env() {
        if crate::live_endpoint(LocalRuntime::Ollama).is_some()
            || crate::live_endpoint(LocalRuntime::Mlx).is_some()
            || crate::live_endpoint(LocalRuntime::Vllm).is_some()
        {
            return;
        }
        let probes = catalog_probes_live();
        assert!(probes.iter().all(|p| !p.live_probed));
        assert!(probes.iter().any(|p| p.note.contains("SKIP")));
    }

    #[test]
    fn catalog_file_is_committed_sot() {
        let snap = catalog_file();
        assert_eq!(snap.schema, "cell-one.local-catalog.v0");
        assert_eq!(snap.cards.len(), CATALOG.len());
        assert!(snap.cards.iter().any(|c| c.driver_id == "ollama" && c.status == "supported"));
        assert!(snap.cards.iter().any(|c| c.driver_id == "mlx" && c.status == "stub"));
        assert_eq!(snap.frontier.model, "grok-4.7");
        assert!(!snap.frontier.caps.streaming);
        assert!(!snap.frontier.caps.tools);
        assert!(!snap.frontier.caps.vision);
        assert_eq!(
            snap.frontier.caps.completion_tokens,
            crate::frontier::FRONTIER_COMPLETION_TOKENS
        );
        let rendered = render_catalog();
        assert!(rendered.contains("model=grok-4.7"), "{rendered}");
        assert!(rendered.contains("schema card, not a binding"), "{rendered}");
        assert!(rendered.contains("completion=64"), "{rendered}");
        let committed = include_str!("../../../schema/local-catalog.v0.json");
        let file: CatalogFile = serde_json::from_str(committed).unwrap();
        assert_eq!(file, snap);
    }

    #[test]
    fn bind_vllm_is_experimental() {
        let binding = ModelBinding {
            id: "local_slm".into(),
            class: ModelClass::Local,
            driver: "vllm".into(),
            params: serde_json::json!({}),
            wired: true,
        };
        let err = bind_local(&binding)
            .unwrap()
            .specialist(&req())
            .unwrap_err();
        assert!(matches!(err, ModelError::Experimental(_)));
        assert!(err.is_local_down());
    }

    #[test]
    fn schema_catalog_overwrite_does_not_invent_a_binding() {
        let dir = std::env::temp_dir().join(format!(
            "cell-one-schema-catalog-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fresh = dir.join("fresh.json");
        refuse_schema_catalog_overwrite(&fresh).unwrap();
        write_catalog(&fresh).unwrap();
        let body = std::fs::read_to_string(&fresh).unwrap();
        assert!(body.contains("\"model\": \"grok-4.7\""), "{body}");
        refuse_schema_catalog_overwrite(&fresh).unwrap();

        let cell = dir.join("catalog.json");
        let mut bound = catalog_file();
        bound.frontier.model.clear();
        super::write_catalog_file(&cell, &bound).unwrap();
        let err = refuse_schema_catalog_overwrite(&cell).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:frontier-model"), "{text}");
        assert!(text.contains("catalog model=-"), "{text}");
        assert!(text.contains("schema card model=grok-4.7"), "{text}");
        assert!(text.contains("schema card is not the binding"), "{text}");
        let still = std::fs::read_to_string(&cell).unwrap();
        assert!(still.contains("\"model\": \"\""), "{still}");
        assert!(!still.contains("\"model\": \"grok-4.7\""), "{still}");

        std::fs::write(&cell, "not-json").unwrap();
        let bad = refuse_schema_catalog_overwrite(&cell).unwrap_err();
        assert!(bad.to_string().contains("unreadable"), "{bad}");
        assert_eq!(std::fs::read_to_string(&cell).unwrap(), "not-json");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
