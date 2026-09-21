//! Mixed path: authorize (A3–A4) → local specialist (A8) → frontier/tool (A7).
//! Not a gateway. Control plane does not call this.
//!
//! Estate-bound local work fails closed when local is down: audited deny,
//! no silent frontier fallback.

use crate::error::ModelError;
use crate::frontier::FrontierDriver;
use crate::local::{LocalDriver, SpecialistJob, SpecialistRequest, SpecialistResult};
use estate_schema::{authorize, AccessRequest, Estate, IntentionKind, ModelClass};
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
