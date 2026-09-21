use crate::sacred::normalize_name;
use crate::types::{Effect, Estate, IntentionKind, ObjectRef};
use std::path::{Component, Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessRequest<'a> {
    pub subject_agent: &'a str,
    pub kind: IntentionKind,
    pub object: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow { reason: String },
    Deny(Deny),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deny {
    pub reason: String,
}

impl Decision {
    pub fn is_allow(&self) -> bool {
        matches!(self, Decision::Allow { .. })
    }

    pub fn reason(&self) -> &str {
        match self {
            Decision::Allow { reason } => reason,
            Decision::Deny(d) => &d.reason,
        }
    }
}

/// Fail-closed authorization used by conveyor-proxy and memory-firewall tests.
pub fn authorize(estate: &Estate, req: &AccessRequest<'_>) -> Decision {
    if estate.agent(req.subject_agent).is_none() {
        return deny(format!(
            "unknown subject agent '{}'",
            req.subject_agent
        ));
    }
    if estate.is_sacred(req.object) {
        return deny(format!(
            "sacred exclusion '{}' is not readable or mountable from the estate",
            req.object
        ));
    }

    if let Some(decision) = explicit_intention(estate, req) {
        return decision;
    }

    match req.kind {
        IntentionKind::MemoryRead => authorize_memory(estate, req),
        IntentionKind::Tool => authorize_declared(estate, req, "tool"),
        IntentionKind::Mount => authorize_declared(estate, req, "mount"),
        IntentionKind::Mcp => authorize_declared(estate, req, "mcp"),
        IntentionKind::Model => authorize_declared(estate, req, "model"),
    }
}

fn authorize_memory(estate: &Estate, req: &AccessRequest<'_>) -> Decision {
    let object = ObjectRef::parse(req.object);
    let lane_id = object.name();
    if estate.owns_lane(req.subject_agent, lane_id) {
        return allow(format!(
            "agent '{}' owns lane '{}'",
            req.subject_agent, lane_id
        ));
    }
    deny(format!(
        "cross-lane memory read denied: {} -> {} (no intention)",
        req.subject_agent, req.object
    ))
}

fn authorize_declared(estate: &Estate, req: &AccessRequest<'_>, kind_label: &str) -> Decision {
    let Some(agent) = estate.agent(req.subject_agent) else {
        return deny(format!("unknown subject agent '{}'", req.subject_agent));
    };
    let object = ObjectRef::parse(req.object);
    let name = object.name();
    let declared = match req.kind {
        IntentionKind::Tool => agent.has_tool(name),
        IntentionKind::Mount => agent.has_mount(name),
        IntentionKind::Mcp => agent.has_mcp(name),
        IntentionKind::Model => agent.has_model(name),
        IntentionKind::MemoryRead => false,
    };
    if declared {
        return allow(format!(
            "agent '{}' has declared {kind_label} '{}'",
            req.subject_agent, name
        ));
    }
    deny(format!(
        "{kind_label} '{}' undeclared for agent '{}' (deny-default)",
        name, req.subject_agent
    ))
}

fn explicit_intention(estate: &Estate, req: &AccessRequest<'_>) -> Option<Decision> {
    let subject = normalize_name(req.subject_agent);
    let object = normalize_name(ObjectRef::parse(req.object).name());
    let mut matched: Vec<Effect> = estate
        .intentions
        .iter()
        .filter(|i| {
            normalize_name(&i.subject_agent) == subject
                && i.kind == req.kind
                && normalize_name(ObjectRef::parse(&i.object).name()) == object
        })
        .map(|i| i.effect)
        .collect();
    if matched.is_empty() {
        return None;
    }
    if matched.iter().any(|e| matches!(e, Effect::Deny)) {
        return Some(deny(format!(
            "explicit deny intention: {} {} {}",
            req.subject_agent,
            req.kind.as_str(),
            req.object
        )));
    }
    if matched.iter().any(|e| matches!(e, Effect::Allow)) {
        return Some(allow(format!(
            "allow intention: {} {} {}",
            req.subject_agent,
            req.kind.as_str(),
            req.object
        )));
    }
    let _ = matched.pop();
    None
}

/// Read a file under a lane root only after authorize() allows it.
pub fn read_lane_file(
    estate: &Estate,
    roots_base: &Path,
    subject: &str,
    lane_id: &str,
    rel: &Path,
) -> Result<Vec<u8>, Deny> {
    let object = format!("lane:{lane_id}");
    match authorize(
        estate,
        &AccessRequest {
            subject_agent: subject,
            kind: IntentionKind::MemoryRead,
            object: &object,
        },
    ) {
        Decision::Allow { .. } => {}
        Decision::Deny(d) => return Err(d),
    }
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(Deny {
            reason: "path escapes lane".into(),
        });
    }
    let lane = estate.lane(lane_id).ok_or_else(|| Deny {
        reason: format!("unknown lane '{lane_id}'"),
    })?;
    let path = roots_base.join(&lane.root_path).join(rel);
    std::fs::read(&path).map_err(|e| Deny {
        reason: format!("lane read io: {e}"),
    })
}

fn allow(reason: impl Into<String>) -> Decision {
    Decision::Allow {
        reason: reason.into(),
    }
}

fn deny(reason: impl Into<String>) -> Decision {
    Decision::Deny(Deny {
        reason: reason.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{load_estate_str, Effect, Intention};

    fn estate() -> Estate {
        load_estate_str(crate::tests::example_yaml()).unwrap()
    }

    fn req<'a>(agent: &'a str, kind: IntentionKind, object: &'a str) -> AccessRequest<'a> {
        AccessRequest {
            subject_agent: agent,
            kind,
            object,
        }
    }

    #[test]
    fn own_lane_memory_allowed() {
        let e = estate();
        assert!(authorize(&e, &req("horizon", IntentionKind::MemoryRead, "lane:horizon")).is_allow());
    }

    #[test]
    fn cross_lane_memory_denied() {
        let e = estate();
        let d = authorize(&e, &req("horizon", IntentionKind::MemoryRead, "lane:research"));
        assert!(!d.is_allow());
        assert!(d.reason().contains("cross-lane"));
    }

    #[test]
    fn sacred_cyera_denied() {
        let e = estate();
        for object in ["cyera-ci", "cyera", "exclusion:cyera-ci", "lane:rust-classroom"] {
            let d = authorize(&e, &req("horizon", IntentionKind::MemoryRead, object));
            assert!(!d.is_allow(), "{object} should be denied");
            assert!(d.reason().contains("sacred"));
        }
    }

    #[test]
    fn undeclared_tool_denied() {
        let e = estate();
        let d = authorize(&e, &req("horizon", IntentionKind::Tool, "shell"));
        assert!(!d.is_allow());
        assert!(d.reason().contains("undeclared"));
    }

    #[test]
    fn declared_tool_allowed() {
        let e = estate();
        assert!(authorize(&e, &req("research", IntentionKind::Tool, "notes-append")).is_allow());
    }

    #[test]
    fn declared_mount_allowed_undeclared_denied() {
        let e = estate();
        assert!(authorize(&e, &req("research", IntentionKind::Mount, "notes")).is_allow());
        let d = authorize(&e, &req("research", IntentionKind::Mount, "secrets"));
        assert!(!d.is_allow());
        assert!(d.reason().contains("undeclared"));
    }

    #[test]
    fn intention_can_allow_cross_lane() {
        let mut e = estate();
        e.intentions.push(Intention {
            subject_agent: "horizon".into(),
            object: "lane:research".into(),
            kind: IntentionKind::MemoryRead,
            effect: Effect::Allow,
            note: Some("test grant".into()),
        });
        assert!(authorize(&e, &req("horizon", IntentionKind::MemoryRead, "lane:research")).is_allow());
        // still deny the other way
        assert!(!authorize(&e, &req("research", IntentionKind::MemoryRead, "lane:horizon")).is_allow());
    }

    #[test]
    fn declared_model_allowed_undeclared_denied() {
        let e = estate();
        assert!(authorize(&e, &req("horizon", IntentionKind::Model, "xai_grok")).is_allow());
        assert!(authorize(&e, &req("horizon", IntentionKind::Model, "local_slm")).is_allow());
        assert!(authorize(&e, &req("research", IntentionKind::Model, "local_slm")).is_allow());
        let d = authorize(&e, &req("sanctum", IntentionKind::Model, "xai_grok"));
        assert!(!d.is_allow());
        assert!(d.reason().contains("undeclared"));
    }

    #[test]
    fn rust_classroom_not_an_agent() {
        let e = estate();
        assert!(e.agent("rust-classroom").is_none());
        assert!(e.agent("cyera-ci").is_none());
        assert!(e.is_sacred("rust-classroom"));
    }

    #[test]
    fn cross_lane_file_read_fails_closed() {
        let e = estate();
        let tmp = std::env::temp_dir().join(format!("cell-one-fw-{}", std::process::id()));
        let research = tmp.join("lanes/research");
        std::fs::create_dir_all(&research).unwrap();
        std::fs::write(research.join("secret.txt"), "research-only").unwrap();
        let err = read_lane_file(
            &e,
            &tmp,
            "horizon",
            "research",
            Path::new("secret.txt"),
        )
        .unwrap_err();
        assert!(err.reason.contains("cross-lane"));
        let own = read_lane_file(
            &e,
            &tmp,
            "research",
            "research",
            Path::new("secret.txt"),
        )
        .unwrap();
        assert_eq!(own, b"research-only");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
