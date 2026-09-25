use crate::hash::estate_hash;
use crate::sacred::normalize_name;
use crate::types::{Effect, Estate, IntentionKind, ObjectRef};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledIntention {
    pub subject_agent: String,
    pub object: String,
    pub kind: IntentionKind,
    pub effect: Effect,
    pub source_estate_hash: String,
}

pub fn compile_intentions(estate: &Estate) -> Result<Vec<CompiledIntention>, Vec<String>> {
    let errors = intention_object_errors(estate);
    if !errors.is_empty() {
        return Err(errors);
    }
    let hash = estate_hash(estate);
    Ok(estate
        .intentions
        .iter()
        .map(|i| CompiledIntention {
            subject_agent: i.subject_agent.clone(),
            object: i.object.clone(),
            kind: i.kind,
            effect: i.effect,
            source_estate_hash: hash.clone(),
        })
        .collect())
}

/// Shift-left check used by load (`validate`) and `compile_intentions`.
/// An intention object must name something that kind can resolve.
/// Allow and deny use the same rule.
pub fn intention_object_errors(estate: &Estate) -> Vec<String> {
    let mut errors = Vec::new();
    for intention in &estate.intentions {
        let agent_known = estate.agent(&intention.subject_agent).is_some();
        if !agent_known {
            errors.push(format!(
                "intention subject_agent '{}' is not an agent",
                intention.subject_agent
            ));
        }
        if intention.object.trim().is_empty() {
            errors.push("intention object must not be empty".into());
            continue;
        }
        if matches!(intention.effect, Effect::Allow) && estate.is_sacred(&intention.object) {
            errors.push(format!(
                "intention cannot allow sacred exclusion '{}'",
                intention.object
            ));
        }
        if !agent_known {
            continue;
        }
        let Some(agent) = estate.agent(&intention.subject_agent) else {
            continue;
        };
        match intention.kind {
            IntentionKind::MemoryRead => {}
            IntentionKind::Tool | IntentionKind::Mcp | IntentionKind::Mount => {
                if !declared_named(agent, intention.kind, &intention.object) {
                    errors.push(format!(
                        "intention {} object '{}' is not declared on agent '{}'",
                        intention.kind.as_str(),
                        intention.object,
                        agent.id
                    ));
                }
            }
            IntentionKind::Model => {
                if let Some(err) = model_object_error(estate, agent, &intention.object) {
                    errors.push(err);
                }
            }
            IntentionKind::Agent => {
                if let Some(err) = agent_object_error(estate, agent, &intention.object) {
                    errors.push(err);
                }
            }
        }
    }
    errors
}

fn declared_named(agent: &crate::types::Agent, kind: IntentionKind, object: &str) -> bool {
    let parsed = ObjectRef::parse(object);
    let name = parsed.name();
    if name.trim().is_empty() {
        return false;
    }
    match kind {
        IntentionKind::Tool => agent.has_tool(name),
        IntentionKind::Mcp => agent.has_mcp(name),
        IntentionKind::Mount => agent.has_mount(name),
        IntentionKind::Agent => agent.has_call(name),
        IntentionKind::Model | IntentionKind::MemoryRead => false,
    }
}

fn agent_object_error(estate: &Estate, agent: &crate::types::Agent, object: &str) -> Option<String> {
    let raw = object.trim();
    if raw.starts_with("lane:")
        || raw.starts_with("tool:")
        || raw.starts_with("mcp:")
        || raw.starts_with("mount:")
        || raw.starts_with("binding:")
        || raw.starts_with("model:")
        || raw.starts_with("class:")
        || raw.starts_with("exclusion:")
    {
        return Some(format!(
            "intention agent object '{object}' is not an agent id for agent '{}'",
            agent.id
        ));
    }
    let name = raw.strip_prefix("agent:").unwrap_or(raw).trim();
    if name.is_empty() {
        return Some(format!(
            "intention agent object '{object}' is not an estate agent"
        ));
    }
    if estate.is_sacred(name) {
        return Some(format!(
            "sacred exclusion '{name}' cannot be an agent call target"
        ));
    }
    if estate.agent(name).is_none() {
        return Some(format!(
            "intention agent object '{object}' is not an estate agent"
        ));
    }
    if !agent.has_call(name) {
        return Some(format!(
            "intention agent object '{object}' is not declared on agent '{}'",
            agent.id
        ));
    }
    None
}

fn model_object_error(estate: &Estate, agent: &crate::types::Agent, object: &str) -> Option<String> {
    let raw = object.trim();
    let class_token = raw
        .strip_prefix("class:")
        .map(|rest| (true, normalize_name(rest)));
    let (is_class, token) = if let Some(pair) = class_token {
        pair
    } else if let Some(rest) = raw.strip_prefix("binding:").or_else(|| raw.strip_prefix("model:"))
    {
        (false, normalize_name(rest))
    } else {
        let token = normalize_name(raw);
        (token == "frontier" || token == "local", token)
    };
    if token.is_empty() {
        return Some(format!(
            "intention model object '{object}' is not declared on agent '{}'",
            agent.id
        ));
    }
    if is_class {
        if token == "frontier" || token == "local" {
            return None;
        }
        return Some(format!(
            "intention model object '{object}' is not a model class for agent '{}'",
            agent.id
        ));
    }
    let on_estate = estate
        .model_bindings
        .iter()
        .any(|b| normalize_name(&b.id) == token);
    if !on_estate {
        return Some(format!(
            "intention model object '{object}' is not a model_binding for agent '{}'",
            agent.id
        ));
    }
    if !agent.has_model(&token) {
        return Some(format!(
            "intention model object '{object}' is not declared on agent '{}'",
            agent.id
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_estate_str;

    use crate::{Effect, Intention};

    fn with_intention(kind: IntentionKind, agent: &str, object: &str, effect: Effect) -> Estate {
        let mut e = load_estate_str(crate::tests::example_yaml()).unwrap();
        e.intentions.push(Intention {
            subject_agent: agent.into(),
            object: object.into(),
            kind,
            effect,
            note: None,
        });
        e
    }

    #[test]
    fn empty_intentions_compile_empty() {
        let e = load_estate_str(crate::tests::example_yaml()).unwrap();
        assert!(compile_intentions(&e).unwrap().is_empty());
        assert!(intention_object_errors(&e).is_empty());
    }

    #[test]
    fn orphan_tool_mcp_mount_model_rejected() {
        for (kind, object) in [
            (IntentionKind::Tool, "shell"),
            (IntentionKind::Mcp, "mcp:browser"),
            (IntentionKind::Mount, "secrets"),
            (IntentionKind::Model, "binding:missing"),
        ] {
            let e = with_intention(kind, "horizon", object, Effect::Allow);
            let err = compile_intentions(&e).unwrap_err();
            let text = err.join("\n");
            assert!(
                text.contains("horizon") && text.contains(kind.as_str()) && text.contains(object),
                "{text}"
            );
            let denied = with_intention(kind, "horizon", object, Effect::Deny);
            let deny_text = compile_intentions(&denied).unwrap_err().join("\n");
            assert!(deny_text.contains(object), "{deny_text}");
        }
    }

    #[test]
    fn model_on_wrong_agent_rejected() {
        let e = with_intention(IntentionKind::Model, "sanctum", "xai_grok", Effect::Allow);
        let text = compile_intentions(&e).unwrap_err().join("\n");
        assert!(text.contains("sanctum"), "{text}");
        assert!(text.contains("xai_grok"), "{text}");
    }

    #[test]
    fn declared_tool_and_model_class_compile() {
        let tool = with_intention(
            IntentionKind::Tool,
            "research",
            "tool:notes-append",
            Effect::Allow,
        );
        assert!(compile_intentions(&tool).unwrap().len() == 1);
        let class = with_intention(IntentionKind::Model, "horizon", "class:frontier", Effect::Deny);
        assert!(compile_intentions(&class).is_ok());
        let bare = with_intention(IntentionKind::Model, "research", "local", Effect::Allow);
        assert!(compile_intentions(&bare).is_ok());
        let binding = with_intention(
            IntentionKind::Model,
            "horizon",
            "model:xai_grok",
            Effect::Allow,
        );
        assert!(compile_intentions(&binding).is_ok());
    }

    #[test]
    fn load_validation_rejects_the_same_orphans() {
        let e = with_intention(
            IntentionKind::Mount,
            "research",
            "mount:missing",
            Effect::Deny,
        );
        let err = crate::validate(&e).unwrap_err();
        let text = err.join("\n");
        assert!(text.contains("research"), "{text}");
        assert!(text.contains("mount"), "{text}");
        assert!(text.contains("mount:missing"), "{text}");
        let bad_class = with_intention(
            IntentionKind::Model,
            "horizon",
            "class:nope",
            Effect::Allow,
        );
        let class_text = crate::validate(&bad_class).unwrap_err().join("\n");
        assert!(class_text.contains("not a model class"), "{class_text}");
        assert!(class_text.contains("horizon"), "{class_text}");
    }

    #[test]
    fn agent_call_object_must_be_a_real_agent() {
        let unknown = with_intention(IntentionKind::Agent, "horizon", "ghost", Effect::Allow);
        let text = compile_intentions(&unknown).unwrap_err().join("\n");
        assert!(text.contains("not an estate agent"), "{text}");
        assert!(text.contains("ghost"), "{text}");
        let sacred = with_intention(IntentionKind::Agent, "horizon", "agent:cyera-ci", Effect::Deny);
        let sacred_text = compile_intentions(&sacred).unwrap_err().join("\n");
        assert!(sacred_text.contains("sacred"), "{sacred_text}");
        let mut declared = load_estate_str(crate::tests::example_yaml()).unwrap();
        declared
            .agents
            .iter_mut()
            .find(|a| a.id == "horizon")
            .unwrap()
            .calls
            .push(crate::types::CallDecl {
                id: "research".into(),
                description: None,
            });
        declared.intentions.push(Intention {
            subject_agent: "horizon".into(),
            object: "agent:research".into(),
            kind: IntentionKind::Agent,
            effect: Effect::Allow,
            note: None,
        });
        assert!(compile_intentions(&declared).is_ok());
        assert!(crate::validate(&declared).is_ok());

        let mut unknown_call = load_estate_str(crate::tests::example_yaml()).unwrap();
        unknown_call
            .agents
            .iter_mut()
            .find(|a| a.id == "horizon")
            .unwrap()
            .calls
            .push(crate::types::CallDecl {
                id: "ghost".into(),
                description: None,
            });
        let call_text = crate::validate(&unknown_call).unwrap_err().join("\n");
        assert!(call_text.contains("not an estate agent"), "{call_text}");

        let mut sacred_call = load_estate_str(crate::tests::example_yaml()).unwrap();
        sacred_call
            .agents
            .iter_mut()
            .find(|a| a.id == "horizon")
            .unwrap()
            .calls
            .push(crate::types::CallDecl {
                id: "rust-classroom".into(),
                description: None,
            });
        let sacred_call_text = crate::validate(&sacred_call).unwrap_err().join("\n");
        assert!(sacred_call_text.contains("sacred"), "{sacred_call_text}");
    }

    #[test]
    fn unknown_agent_rejected() {
        let e = with_intention(IntentionKind::Tool, "ghost", "notes-append", Effect::Deny);
        let text = compile_intentions(&e).unwrap_err().join("\n");
        assert!(text.contains("ghost"), "{text}");
        assert!(text.contains("not an agent"), "{text}");
    }
}
