//! Mixed path: authorize (A3–A4) → local specialist (A8) → frontier/tool (A7).
//! Not a gateway. Control plane does not call [`run_task`].
//!
//! [`complete_via_binding`] is the thin estate-bound complete used by
//! `estate complete` after host prepare / select / authorize. It is not
//! the mixed path. Estate-bound local work fails closed when local is
//! down: no silent frontier fallback.

use crate::catalog::parse_runtime;
use crate::error::ModelError;
use crate::frontier::{frontier_from_binding, FrontierDriver, MockFrontier};
use crate::local::{
    builtin_specialist, local_from_binding, resolve_specialist_endpoint, run_http_specialist,
    HttpLocal, LocalDriver, MockLocal, SpecialistJob, SpecialistRequest, SpecialistResult,
};
use estate_schema::{authorize, AccessRequest, Estate, IntentionKind, ModelBinding, ModelClass};
use feed_collector::{append_event, ScrubbedEvent};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskAct {
    Tool,
    Model,
}

#[derive(Debug, Clone)]
pub struct TaskRequest {
    pub agent_id: String,
    pub act: TaskAct,
    pub object: String,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskResult {
    pub authorized: bool,
    pub precheck: Option<SpecialistResult>,
    pub output: Option<String>,
    pub denied: Option<String>,
    pub path: Vec<String>,
}

pub fn run_task(
    estate: &Estate,
    req: &TaskRequest,
    frontier: Option<&dyn FrontierDriver>,
    local: Option<&dyn LocalDriver>,
    feed_dir: Option<&Path>,
) -> Result<TaskResult, ModelError> {
    let kind = match req.act {
        TaskAct::Tool => IntentionKind::Tool,
        TaskAct::Model => IntentionKind::Model,
    };
    let decision = authorize(
        estate,
        &AccessRequest {
            subject_agent: &req.agent_id,
            kind,
            object: &req.object,
        },
    );
    feed(
        feed_dir,
        &format!("proxy.{}", kind.as_str()),
        Some(&req.agent_id),
        if decision.is_allow() { "allow" } else { "deny" },
        kind.as_str(),
        None,
    )?;
    if !decision.is_allow() {
        return Ok(TaskResult {
            authorized: false,
            precheck: None,
            output: None,
            denied: Some(decision.reason().to_string()),
            path: vec!["authorize:deny".into()],
        });
    }

    let mut path = vec!["authorize:allow".into()];
    let mut precheck = None;
    if let Some(local) = local {
        let spec = match local.specialist(&SpecialistRequest {
            job: SpecialistJob::PolicyPrecheck,
            agent_id: req.agent_id.clone(),
            kind: kind.as_str().to_string(),
            text: req.payload.clone(),
        }) {
            Ok(spec) => spec,
            Err(err) if err.is_local_down() => {
                return fail_closed_local(&req.agent_id, err, path, feed_dir);
            }
            Err(err) => return Err(err),
        };
        feed(
            feed_dir,
            "model.local.precheck",
            Some(&req.agent_id),
            if spec.allow { "allow" } else { "deny" },
            "local",
            Some(&format!("job={}", spec.job)),
        )?;
        path.push(if spec.allow {
            "local:allow".into()
        } else {
            "local:deny".into()
        });
        if !spec.allow {
            return Ok(TaskResult {
                authorized: true,
                precheck: Some(spec),
                output: None,
                denied: Some("local specialist denied before tool/frontier".into()),
                path,
            });
        }
        precheck = Some(spec);
    } else if estate
        .model_bindings
        .iter()
        .any(|b| b.class == ModelClass::Local && b.wired)
    {
        return fail_closed_local(
            &req.agent_id,
            ModelError::MissingEndpoint("CELL_LOCAL_ENDPOINT".into()),
            path,
            feed_dir,
        );
    }

    match req.act {
        TaskAct::Tool => {
            path.push("tool:allow".into());
            feed(
                feed_dir,
                "model.tool",
                Some(&req.agent_id),
                "allow",
                "tool",
                None,
            )?;
            Ok(TaskResult {
                authorized: true,
                precheck,
                output: Some(format!(
                    "tool '{}' authorized after local precheck",
                    req.object
                )),
                denied: None,
                path,
            })
        }
        TaskAct::Model => {
            let text = precheck
                .as_ref()
                .map(|p| p.redacted_text.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or(req.payload.as_str());
            let driver = frontier.ok_or_else(|| {
                ModelError::Other("frontier driver required for model act".into())
            })?;
            let output = driver.complete(text)?;
            feed(
                feed_dir,
                "model.frontier.complete",
                Some(&req.agent_id),
                "allow",
                "frontier",
                Some(&format!("bytes={}", output.len())),
            )?;
            path.push("frontier:complete".into());
            Ok(TaskResult {
                authorized: true,
                precheck,
                output: Some(output),
                denied: None,
                path,
            })
        }
    }
}

/// Complete through one selected binding's driver. Not [`run_task`].
/// Control authorizes first. Missing local endpoint or frontier key
/// fail-closes. No silent class fallback.
pub fn complete_via_binding(
    estate: &Estate,
    binding_id: &str,
    agent: &str,
    payload: &str,
    endpoint: Option<&str>,
    mock: bool,
) -> Result<SpecialistResult, ModelError> {
    let text = payload.trim();
    if text.is_empty() {
        return Err(ModelError::Refused("specialist text is empty".into()));
    }
    if estate_schema::contains_sku(text) {
        return Err(ModelError::Refused("prompt encodes a hardware SKU".into()));
    }
    let binding = estate
        .model_bindings
        .iter()
        .find(|row| row.id == binding_id)
        .ok_or_else(|| ModelError::Unknown(binding_id.into()))?;
    match binding.class {
        ModelClass::Local => complete_local(binding, agent, text, endpoint, mock),
        ModelClass::Frontier => complete_frontier(binding, agent, text, endpoint, mock),
    }
}

fn complete_local(
    binding: &ModelBinding,
    agent: &str,
    text: &str,
    endpoint: Option<&str>,
    mock: bool,
) -> Result<SpecialistResult, ModelError> {
    let req = SpecialistRequest {
        job: SpecialistJob::Complete,
        agent_id: agent.to_string(),
        kind: "model".into(),
        text: text.to_string(),
    };
    if mock {
        return MockLocal {
            id: binding.id.clone(),
        }
        .specialist(&req);
    }
    let driver = if let Some(raw) = endpoint {
        let endpoint = resolve_specialist_endpoint(Some(raw))?;
        let runtime = parse_runtime(&binding.driver).ok_or_else(|| {
            ModelError::Other(format!(
                "unknown local driver '{}'; catalog runtimes: ollama, llama.cpp, mlx, vllm, trt, http-remote",
                binding.driver
            ))
        })?;
        Box::new(HttpLocal {
            id: binding.id.clone(),
            endpoint,
            runtime,
        }) as Box<dyn LocalDriver>
    } else {
        local_from_binding(binding)?
    };
    driver.specialist(&req)
}

fn complete_frontier(
    binding: &ModelBinding,
    agent: &str,
    text: &str,
    endpoint: Option<&str>,
    mock: bool,
) -> Result<SpecialistResult, ModelError> {
    let policy = builtin_specialist(&SpecialistRequest {
        job: SpecialistJob::PolicyPrecheck,
        agent_id: agent.to_string(),
        kind: "model".into(),
        text: text.to_string(),
    });
    if !policy.allow {
        return Ok(SpecialistResult {
            job: SpecialistJob::Complete.as_str().into(),
            completion: String::new(),
            ..policy
        });
    }
    if mock {
        let frontier = MockFrontier {
            id: binding.id.clone(),
            reply: "pong".into(),
        };
        return Ok(SpecialistResult {
            allow: true,
            redacted_text: policy.redacted_text,
            reason: "frontier completion".into(),
            job: SpecialistJob::Complete.as_str().into(),
            completion: frontier.complete(text)?,
        });
    }
    if endpoint.is_some() {
        return run_http_specialist(endpoint, "complete", agent, "model", text, "frontier");
    }
    let driver = frontier_from_binding(binding)?;
    let completion = driver.complete(text)?;
    if estate_schema::contains_sku(&completion) {
        return Err(ModelError::Refused(
            "completion encodes a hardware SKU".into(),
        ));
    }
    Ok(SpecialistResult {
        allow: true,
        redacted_text: policy.redacted_text,
        reason: "frontier completion".into(),
        job: SpecialistJob::Complete.as_str().into(),
        completion,
    })
}

fn fail_closed_local(
    agent_id: &str,
    err: ModelError,
    mut path: Vec<String>,
    feed_dir: Option<&Path>,
) -> Result<TaskResult, ModelError> {
    feed(
        feed_dir,
        "model.local.down",
        Some(agent_id),
        "deny",
        "local",
        Some("fail-closed; no frontier fallback"),
    )?;
    path.push("local:down".into());
    Ok(TaskResult {
        authorized: true,
        precheck: None,
        output: None,
        denied: Some(format!(
            "fail-closed: local down; no frontier fallback ({err})"
        )),
        path,
    })
}

fn feed(
    dir: Option<&Path>,
    kind: &str,
    agent_id: Option<&str>,
    decision: &str,
    object_class: &str,
    note: Option<&str>,
) -> Result<(), ModelError> {
    let Some(dir) = dir else {
        return Ok(());
    };
    append_event(
        dir,
        &ScrubbedEvent {
            kind: kind.into(),
            agent_id: agent_id.map(|s| s.to_string()),
            decision: Some(decision.into()),
            object_class: Some(object_class.into()),
            note: note.map(|s| s.to_string()),
            ts: String::new(),
        },
    )
    .map_err(|err| ModelError::Other(format!("feed: {err}")))
}
