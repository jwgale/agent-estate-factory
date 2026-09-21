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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CatalogFile {
    pub schema: String,
    pub note: String,
    pub hosts: Vec<String>,
    pub cards: Vec<CatalogFileCard>,
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
    }
}

pub fn write_catalog(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(
        path,
        serde_json::to_string_pretty(&catalog_file()).unwrap_or_default(),
    )?;
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
    fn live_probe_ok_against_mock_and_down_is_safe() {
        let server = crate::MockLocalServer::spawn().unwrap();
        let p = crate::probe_runtime("ollama", LocalRuntime::Ollama, "any");
        let p = crate::enrich_with_live(p, Some(&server.endpoint()));
        assert!(p.live_probed, "{}", p.note);
        let down = crate::enrich_with_live(
            crate::probe_runtime("mlx", LocalRuntime::Mlx, "apple-silicon"),
            Some("http://127.0.0.1:1"),
        );
        assert!(!down.live_probed);
        assert!(down.note.contains("down"));
    }

    #[test]
    fn catalog_file_is_committed_sot() {
        let snap = catalog_file();
        assert_eq!(snap.schema, "cell-one.local-catalog.v0");
        assert_eq!(snap.cards.len(), CATALOG.len());
        assert!(snap.cards.iter().any(|c| c.driver_id == "ollama" && c.status == "supported"));
        assert!(snap.cards.iter().any(|c| c.driver_id == "mlx" && c.status == "stub"));
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
}
