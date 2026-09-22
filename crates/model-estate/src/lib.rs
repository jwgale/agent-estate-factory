//! Equal-class frontier and local drivers. Live invoke is data-plane only.
//! Local hardware is a catalog/route/bind driver choice, not a product fork.

mod actual;
mod adapter;
mod catalog;
mod error;
mod frontier;
mod local;
mod mock;
mod path;

pub use actual::{drift_bindings, record_bindings, ModelActual, ModelDrift};
pub use catalog::{
    bind_local, card, catalog, catalog_file, catalog_probes, catalog_probes_live, parse_host_class, parse_runtime,
    catalog_bound_to_estate, render_catalog, route, write_bound_catalog, write_catalog, CatalogCard, CatalogFile, CatalogFileCard, DriverCaps,
    FrontierCaps, FrontierCard, FrontierCatalogCard, HostClass, LocalRuntime, SupportStatus,
    CATALOG, FRONTIER_CARD,
};
pub use error::ModelError;
pub use frontier::{
    frontier_from_binding, FrontierDriver, HttpFrontier, MockFrontier, UnwiredFrontier,
};
pub use adapter::{ping_live_endpoint, specialist_via_adapter, LiveFlavor};
pub use local::{
    apply_live_overlay, builtin_specialist, enrich_with_live, live_endpoint, live_endpoint_envs,
    is_frontier_specialist_driver, live_probe_env_requested, local_from_binding,
    parse_specialist_job, probe_runtime, resolve_frontier_specialist_endpoint,
    resolve_specialist_endpoint, run_http_specialist, DownLocal, DriverProbe, ExperimentalLocal,
    HttpLocal, LiveOverlay, LocalDriver, MlxDriver, MockLocal, SpecialistJob, SpecialistRequest,
    SpecialistResult, UnwiredLocal,
};
pub use mock::{
    serve_specialist_forever, CompatScript, CompatServer, MockFrontierServer, MockLocalServer,
};
pub use path::{run_task, TaskAct, TaskRequest, TaskResult};

use estate_schema::{Estate, ModelBinding, ModelClass};

pub fn frontier_bindings(estate: &Estate) -> Vec<&ModelBinding> {
    estate
        .model_bindings
        .iter()
        .filter(|b| b.class == ModelClass::Frontier)
        .collect()
}

pub fn local_bindings(estate: &Estate) -> Vec<&ModelBinding> {
    estate
        .model_bindings
        .iter()
        .filter(|b| b.class == ModelClass::Local)
        .collect()
}

pub fn ping(estate: &Estate, binding_id: &str) -> Result<(), ModelError> {
    let binding = estate
        .model_bindings
        .iter()
        .find(|b| b.id == binding_id)
        .ok_or_else(|| ModelError::Unknown(binding_id.to_string()))?;
    if !binding.wired {
        return Err(ModelError::NotWired(binding.id.clone()));
    }
    Ok(())
}

/// Frontier binding ids whose `params.model` is set. No catalog default.
/// One distinct model, or none. Two different models refuse. A SKU refuses.
pub fn bound_frontier_model(estate: &Estate) -> Result<Option<String>, ModelError> {
    let mut unique = Vec::new();
    for (_, model) in estate_frontier_models(estate) {
        if estate_schema::contains_sku(&model) {
            return Err(ModelError::Other(format!(
                "refuse:frontier-model: binding model '{model}' encodes a hardware SKU"
            )));
        }
        if !unique.iter().any(|have: &String| have == &model) {
            unique.push(model);
        }
    }
    match unique.len() {
        0 => Ok(None),
        1 => Ok(unique.pop()),
        _ => Err(ModelError::Other(
            "refuse:frontier-model: frontier bindings name more than one model".into(),
        )),
    }
}

/// Frontier binding ids whose `params.model` is set. No catalog default.
pub fn estate_frontier_models(estate: &Estate) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for binding in &estate.model_bindings {
        if binding.class != ModelClass::Frontier {
            continue;
        }
        let Some(model) = binding
            .params
            .get("model")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        out.push((binding.id.clone(), model.to_string()));
    }
    out
}

/// `frontier.model` from a catalog JSON blob. Missing field is `Ok(None)`.
pub fn frontier_model_from_catalog_json(text: &str) -> Result<Option<String>, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    Ok(value
        .get("frontier")
        .and_then(|f| f.get("model"))
        .and_then(|m| m.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string()))
}

pub fn describe_bindings(estate: &Estate) -> String {
    let mut lines = vec![
        "model bindings (equal class; invoke via model-estate, not estate-control):".to_string(),
        "  local hardware is a driver choice (consumer-nvidia | apple-silicon | rented-nvidia)."
            .to_string(),
    ];
    for b in &estate.model_bindings {
        let status = match b.class {
            ModelClass::Local => parse_runtime(&b.driver)
                .map(|r| card(r).status.as_str())
                .unwrap_or("unknown"),
            ModelClass::Frontier => "frontier",
        };
        let host = b
            .params
            .get("host_class")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        let model = b
            .params
            .get("model")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("-");
        lines.push(format!(
            "  {:<8} {:<16} driver={:<16} wired={} status={:<12} host_class={} model={}",
            b.class.as_str(),
            b.id,
            b.driver,
            b.wired,
            status,
            host,
            model
        ));
    }
    lines.join("\n")
}

/// Env readiness only. Never prints secret values.
pub fn readiness(estate: &Estate) -> String {
    let mut lines = vec![
        "credential readiness (names only):".to_string(),
        "  estate-bound local work fails closed if local is down (no silent frontier fallback)."
            .to_string(),
    ];
    for b in &estate.model_bindings {
        match b.class {
            ModelClass::Frontier => {
                let key_env = b
                    .params
                    .get("api_key_env")
                    .and_then(|v| v.as_str())
                    .unwrap_or("XAI_API_KEY");
                lines.push(format!(
                    "  {} {}: {}",
                    b.id,
                    key_env,
                    if std::env::var(key_env).is_ok() {
                        "present"
                    } else {
                        "absent"
                    }
                ));
            }
            ModelClass::Local => {
                let env_name = b
                    .params
                    .get("endpoint_env")
                    .and_then(|v| v.as_str())
                    .unwrap_or("CELL_LOCAL_ENDPOINT");
                lines.push(format!(
                    "  {} {}: {}",
                    b.id,
                    env_name,
                    if std::env::var(env_name).is_ok() {
                        "present"
                    } else {
                        "absent (fail closed)"
                    }
                ));
            }
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use estate_schema::{load_estate_str, IntentionKind};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn estate() -> Estate {
        load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
    }

    struct CountingLocal {
        inner: MockLocal,
        hits: AtomicUsize,
    }

    impl LocalDriver for CountingLocal {
        fn id(&self) -> &str {
            self.inner.id()
        }
        fn specialist(&self, req: &SpecialistRequest) -> Result<SpecialistResult, ModelError> {
            self.hits.fetch_add(1, Ordering::SeqCst);
            self.inner.specialist(req)
        }
    }

    struct CountingFrontier {
        inner: MockFrontier,
        hits: AtomicUsize,
    }

    impl FrontierDriver for CountingFrontier {
        fn id(&self) -> &str {
            self.inner.id()
        }
        fn complete(&self, prompt: &str) -> Result<String, ModelError> {
            self.hits.fetch_add(1, Ordering::SeqCst);
            self.inner.complete(prompt)
        }
    }

    #[test]
    fn frontier_model_follows_the_binding_not_the_catalog_default() {
        let named = load_estate_str(include_str!(
            "../../../examples/fixtures/mixed-frontier-local.yaml"
        ))
        .unwrap();
        assert_eq!(
            estate_frontier_models(&named),
            vec![("frontier_http".into(), "grok-4.7".into())]
        );
        assert!(estate_frontier_models(&estate()).is_empty());
        let parsed = frontier_model_from_catalog_json(include_str!(
            "../../../schema/local-catalog.v0.json"
        ))
        .unwrap();
        assert_eq!(parsed.as_deref(), Some("grok-4.7"));
        assert!(frontier_model_from_catalog_json(r#"{"cards":[]}"#)
            .unwrap()
            .is_none());
        assert!(frontier_model_from_catalog_json("not-json").is_err());
    }

    #[test]
    fn bound_catalog_prints_the_binding_model_or_nothing() {
        let unbound = catalog_bound_to_estate(&estate()).unwrap();
        assert!(
            unbound.frontier.model.is_empty(),
            "unbound estate must not copy schema grok-4.7: {}",
            unbound.frontier.model
        );
        assert!(
            !unbound.frontier.notes.contains("grok-4.7"),
            "{}",
            unbound.frontier.notes
        );
        assert_eq!(unbound.cards.len(), catalog_file().cards.len());

        let named = load_estate_str(include_str!(
            "../../../examples/fixtures/mixed-frontier-local.yaml"
        ))
        .unwrap();
        assert_eq!(
            catalog_bound_to_estate(&named).unwrap().frontier.model,
            "grok-4.7"
        );

        let mut split = named.clone();
        split.model_bindings.push(estate_schema::ModelBinding {
            id: "other_frontier".into(),
            class: estate_schema::ModelClass::Frontier,
            driver: "http-remote".into(),
            params: serde_json::json!({ "model": "other-model" }),
            wired: true,
        });
        let err = catalog_bound_to_estate(&split).unwrap_err();
        assert!(err.to_string().contains("refuse:frontier-model"), "{err}");
        assert_eq!(catalog_file().frontier.model, "grok-4.7");
    }

    #[test]
    fn example_has_equal_class_wired_portable_ids() {
        let e = estate();
        assert_eq!(frontier_bindings(&e).len(), 1);
        assert_eq!(local_bindings(&e).len(), 1);
        assert!(ping(&e, "xai_grok").is_ok());
        assert!(ping(&e, "local_slm").is_ok());
        assert!(e.model_bindings.iter().all(|b| b.wired));
        let local = local_bindings(&e)[0];
        assert_eq!(local.driver, "ollama");
        assert!(!local.id.contains("5090"));
        assert!(!local.driver.contains("5090"));
        assert_eq!(e.enrich_packs.curator, "jason");
        assert_eq!(e.enrich_packs.policy, "manual");
        assert!(e.enrich_packs.packs.is_empty());
    }

    #[test]
    fn unwired_driver_still_refuses() {
        let f = UnwiredFrontier {
            id: "xai_grok".into(),
        };
        assert!(matches!(f.complete("hi"), Err(ModelError::NotWired(_))));
    }

    #[test]
    fn a7_horizon_completes_via_frontier_after_local() {
        let e = estate();
        let local = MockLocal {
            id: "local_slm".into(),
        };
        let frontier = MockFrontier {
            id: "xai_grok".into(),
            reply: "pong".into(),
        };
        let feed = std::env::temp_dir().join(format!("cell-one-feed-a7-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&feed);
        let result = run_task(
            &e,
            &TaskRequest {
                agent_id: "horizon".into(),
                act: TaskAct::Model,
                object: "xai_grok".into(),
                payload: "Reply with the single word pong.".into(),
            },
            Some(&frontier),
            Some(&local),
            Some(&feed),
        )
        .unwrap();
        assert!(result.authorized);
        assert_eq!(result.output.as_deref(), Some("pong"));
        assert!(result.path.iter().any(|s| s == "local:allow"));
        assert!(result.path.iter().any(|s| s == "frontier:complete"));
        let traces = std::fs::read_to_string(feed.join("events.jsonl")).unwrap();
        assert!(traces.contains("model.local.precheck"));
        assert!(traces.contains("model.frontier.complete"));
        assert!(!traces.contains("XAI_API_KEY"));
        assert!(!traces.contains("Reply with the single word"));
        let _ = std::fs::remove_dir_all(&feed);
    }

    #[test]
    fn feed_audit_failure_refuses_before_frontier() {
        let e = estate();
        let local = MockLocal {
            id: "local_slm".into(),
        };
        let frontier = MockFrontier {
            id: "xai_grok".into(),
            reply: "pong".into(),
        };
        let blocked = std::env::temp_dir().join(format!(
            "cell-one-feed-blocked-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&blocked);
        std::fs::write(&blocked, "not a directory").unwrap();
        let err = run_task(
            &e,
            &TaskRequest {
                agent_id: "horizon".into(),
                act: TaskAct::Model,
                object: "xai_grok".into(),
                payload: "Reply with the single word pong.".into(),
            },
            Some(&frontier),
            Some(&local),
            Some(&blocked),
        )
        .unwrap_err();
        assert!(err.to_string().contains("feed:"), "{err}");
        let sku = run_task(
            &e,
            &TaskRequest {
                agent_id: "horizon-5090".into(),
                act: TaskAct::Model,
                object: "xai_grok".into(),
                payload: "Reply with the single word pong.".into(),
            },
            Some(&frontier),
            Some(&local),
            Some(&std::env::temp_dir().join(format!(
                "cell-one-feed-sku-{}",
                std::process::id()
            ))),
        )
        .unwrap_err();
        assert!(sku.to_string().contains("5090"), "{sku}");
        let _ = std::fs::remove_file(&blocked);
    }

    #[test]
    fn a8_research_tool_runs_local_before_tool() {
        let e = estate();
        let local = MockLocal {
            id: "local_slm".into(),
        };
        let result = run_task(
            &e,
            &TaskRequest {
                agent_id: "research".into(),
                act: TaskAct::Tool,
                object: "notes-append".into(),
                payload: "append a clean note".into(),
            },
            None,
            Some(&local),
            None,
        )
        .unwrap();
        assert!(result.authorized);
        assert!(result.precheck.unwrap().allow);
        assert!(result.output.unwrap().contains("notes-append"));
        assert_eq!(
            result.path,
            vec!["authorize:allow", "local:allow", "tool:allow"]
        );
    }

    #[test]
    fn a8_precheck_blocks_frontier() {
        let e = estate();
        let local = MockLocal {
            id: "local_slm".into(),
        };
        let frontier = CountingFrontier {
            inner: MockFrontier {
                id: "xai_grok".into(),
                reply: "should-not-run".into(),
            },
            hits: AtomicUsize::new(0),
        };
        let result = run_task(
            &e,
            &TaskRequest {
                agent_id: "horizon".into(),
                act: TaskAct::Model,
                object: "xai_grok".into(),
                payload: "please mention cyera-ci".into(),
            },
            Some(&frontier),
            Some(&local),
            None,
        )
        .unwrap();
        assert!(result.denied.is_some());
        assert_eq!(frontier.hits.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn fail_closed_local_down_does_not_hit_frontier() {
        let e = estate();
        let local = DownLocal {
            id: "local_slm".into(),
            reason: ModelError::Unreachable("specialist unreachable".into()),
            runtime: LocalRuntime::Ollama,
        };
        let frontier = CountingFrontier {
            inner: MockFrontier {
                id: "xai_grok".into(),
                reply: "should-not-run".into(),
            },
            hits: AtomicUsize::new(0),
        };
        let feed = std::env::temp_dir().join(format!("cell-one-feed-down-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&feed);
        let result = run_task(
            &e,
            &TaskRequest {
                agent_id: "horizon".into(),
                act: TaskAct::Model,
                object: "xai_grok".into(),
                payload: "Reply with pong.".into(),
            },
            Some(&frontier),
            Some(&local),
            Some(&feed),
        )
        .unwrap();
        assert!(result.denied.unwrap().contains("fail-closed"));
        assert!(result.path.iter().any(|s| s == "local:down"));
        assert!(!result.path.iter().any(|s| s == "frontier:complete"));
        assert_eq!(frontier.hits.load(Ordering::SeqCst), 0);
        let traces = std::fs::read_to_string(feed.join("events.jsonl")).unwrap();
        assert!(traces.contains("model.local.down"));
        assert!(traces.contains("deny"));
        assert!(!traces.contains("model.frontier.complete"));
        let _ = std::fs::remove_dir_all(&feed);
    }

    #[test]
    fn mlx_stub_fail_closed_no_frontier_fallback() {
        let e = estate();
        let local = MlxDriver {
            id: "local_slm".into(),
        };
        let frontier = CountingFrontier {
            inner: MockFrontier {
                id: "xai_grok".into(),
                reply: "should-not-run".into(),
            },
            hits: AtomicUsize::new(0),
        };
        let result = run_task(
            &e,
            &TaskRequest {
                agent_id: "horizon".into(),
                act: TaskAct::Model,
                object: "xai_grok".into(),
                payload: "ping".into(),
            },
            Some(&frontier),
            Some(&local),
            None,
        )
        .unwrap();
        assert!(result.denied.unwrap().contains("fail-closed"));
        assert_eq!(frontier.hits.load(Ordering::SeqCst), 0);
        assert_eq!(local.runtime(), LocalRuntime::Mlx);
    }

    #[test]
    fn local_down_http_frontier_does_not_post() {
        let e = estate();
        let live = CompatServer::spawn(CompatScript::OpenAi {
            models: vec!["grok-4.7".into()],
        })
        .unwrap();
        let frontier = HttpFrontier {
            id: "xai_grok".into(),
            base: live.endpoint(),
            key: "test-not-a-secret".into(),
            model: "grok-4.7".into(),
        };
        let local = DownLocal {
            id: "local_slm".into(),
            reason: ModelError::Unreachable("specialist unreachable".into()),
            runtime: LocalRuntime::Ollama,
        };
        for act in [TaskAct::Model, TaskAct::Tool] {
            let object = match act {
                TaskAct::Model => "xai_grok",
                TaskAct::Tool => "notes-append",
            };
            let agent = match act {
                TaskAct::Model => "horizon",
                TaskAct::Tool => "research",
            };
            let result = run_task(
                &e,
                &TaskRequest {
                    agent_id: agent.into(),
                    act,
                    object: object.into(),
                    payload: "Reply with the single word pong.".into(),
                },
                Some(&frontier),
                Some(&local),
                None,
            )
            .unwrap();
            assert!(
                result.denied.unwrap().contains("fail-closed"),
                "{act:?}"
            );
            assert!(result.path.iter().any(|s| s == "local:down"));
            assert!(!result.path.iter().any(|s| s == "frontier:complete"));
        }
        assert!(
            live.last_post().is_none(),
            "local down must not POST grok-4.7"
        );
    }

    #[test]
    fn a3_a4_denied_never_hits_drivers() {
        let e = estate();
        let local = CountingLocal {
            inner: MockLocal {
                id: "local_slm".into(),
            },
            hits: AtomicUsize::new(0),
        };
        let frontier = CountingFrontier {
            inner: MockFrontier {
                id: "xai_grok".into(),
                reply: "nope".into(),
            },
            hits: AtomicUsize::new(0),
        };
        let denied_model = run_task(
            &e,
            &TaskRequest {
                agent_id: "sanctum".into(),
                act: TaskAct::Model,
                object: "xai_grok".into(),
                payload: "hi".into(),
            },
            Some(&frontier),
            Some(&local),
            None,
        )
        .unwrap();
        assert!(!denied_model.authorized);
        let denied_tool = run_task(
            &e,
            &TaskRequest {
                agent_id: "horizon".into(),
                act: TaskAct::Tool,
                object: "shell".into(),
                payload: "id".into(),
            },
            Some(&frontier),
            Some(&local),
            None,
        )
        .unwrap();
        assert!(!denied_tool.authorized);
        assert_eq!(local.hits.load(Ordering::SeqCst), 0);
        assert_eq!(frontier.hits.load(Ordering::SeqCst), 0);
        let _ = IntentionKind::Model;
    }

    #[test]
    fn a9_same_estate_not_studio_or_proxy_only() {
        let e = estate();
        assert!(!frontier_bindings(&e).is_empty());
        assert!(!local_bindings(&e).is_empty());
        assert!(e.agent("horizon").unwrap().has_model("xai_grok"));
        assert!(e.agent("horizon").unwrap().has_model("local_slm"));
        assert!(e.agent("research").unwrap().has_model("local_slm"));
        assert!(!e.agent("sanctum").unwrap().has_model("xai_grok"));
    }

    #[test]
    fn http_local_and_frontier_mocks() {
        let e = estate();
        let local_srv = MockLocalServer::spawn().unwrap();
        let front_srv = MockFrontierServer::spawn("pong").unwrap();
        let local = HttpLocal {
            id: "local_slm".into(),
            endpoint: local_srv.endpoint(),
            runtime: LocalRuntime::Ollama,
        };
        let frontier = HttpFrontier {
            id: "xai_grok".into(),
            base: front_srv.base(),
            key: "test-not-a-secret".into(),
            model: "mock".into(),
        };
        let result = run_task(
            &e,
            &TaskRequest {
                agent_id: "horizon".into(),
                act: TaskAct::Model,
                object: "xai_grok".into(),
                payload: "ping".into(),
            },
            Some(&frontier),
            Some(&local),
            None,
        )
        .unwrap();
        assert_eq!(result.output.as_deref(), Some("pong"));
        assert_eq!(local.runtime(), LocalRuntime::Ollama);
    }
}
