//! Deny-by-default conveyor. Workers call this; they do not read other lanes.
//! Capability mesh is a sibling seam (`mesh`) — declare hop, lease-bound call.

mod mesh;

pub use mesh::{
    authority_report, call_hop, call_hop_for_agent, declare_hop, declare_hop_covering,
    forget_expired_hop_leases, hop_driver, hop_from_placement, hop_is_cloud,
    hop_kind_for_placement, hop_lease_is_expired, hop_now_unix, list_expired_hop_leases,
    list_hop_leases, list_hops, load_mesh, mesh_file_sot, persist_mesh, refuse_mesh_host_classes,
    slim_parse_placement_actual, sync_from_placements, sync_from_placements_covering, AuthorityRow,
    BoxHop, CloudMeshHop, ConveyorHop, ConveyorMesh, HopCall, HopDecl, HopLease, MeshError,
    MESH_FILE, MESH_SCHEMA,
};

use estate_schema::{authorize, coverage_word, AccessRequest, Decision, Estate, IntentionKind};
use feed_collector::{append_event, ScrubbedEvent};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

/// Feed kind for one intention decision. Agent is `proxy.agent`. The other
/// intention kinds keep the same `proxy.{kind}` stamp `check` already writes.
pub fn proxy_intention_kind(kind: IntentionKind) -> String {
    format!("proxy.{}", kind.as_str())
}

/// Append one proxy allow/deny line to `{feed_dir}/events.jsonl` and restamp
/// `feed-cursor.json`. Does not materialize or promote a pack. Serialize or
/// IO failure is `refuse:proxy-audit`.
pub fn append_proxy_audit(
    feed_dir: &Path,
    kind: &str,
    agent_id: Option<&str>,
    decision: &str,
    object_class: &str,
    note: Option<&str>,
) -> Result<(), MeshError> {
    append_event(
        feed_dir,
        &ScrubbedEvent {
            kind: kind.to_string(),
            agent_id: agent_id.map(|s| s.to_string()),
            decision: Some(decision.to_string()),
            object_class: Some(object_class.to_string()),
            note: note.map(|s| s.to_string()),
            ts: String::new(),
        },
    )
    .map_err(|err| MeshError::ProxyAudit(err.to_string()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyRequest {
    pub agent_id: String,
    pub kind: String,
    pub object: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyResponse {
    pub decision: String,
    pub reason: String,
}

pub struct WorkerClient<'a> {
    estate: &'a Estate,
    feed_dir: Option<&'a Path>,
}

impl<'a> WorkerClient<'a> {
    pub fn new(estate: &'a Estate) -> Self {
        Self {
            estate,
            feed_dir: None,
        }
    }

    pub fn with_feed(mut self, dir: &'a Path) -> Self {
        self.feed_dir = Some(dir);
        self
    }

    pub fn invoke(
        &self,
        agent_id: &str,
        kind: IntentionKind,
        object: &str,
    ) -> Result<Decision, MeshError> {
        check(self.estate, agent_id, kind, object, self.feed_dir)
    }
}

pub fn check(
    estate: &Estate,
    agent_id: &str,
    kind: IntentionKind,
    object: &str,
    feed_dir: Option<&Path>,
) -> Result<Decision, MeshError> {
    let decision = authorize(
        estate,
        &AccessRequest {
            subject_agent: agent_id,
            kind,
            object,
        },
    );
    if let Some(dir) = feed_dir {
        append_proxy_audit(
            dir,
            &proxy_intention_kind(kind),
            Some(agent_id),
            coverage_word(&decision),
            kind.as_str(),
            Some(decision.reason()),
        )?;
    }
    Ok(decision)
}

pub fn parse_kind(raw: &str) -> Result<IntentionKind, String> {
    IntentionKind::from_str(raw)
}

pub fn response_from(decision: &Decision) -> ProxyResponse {
    ProxyResponse {
        decision: if decision.is_allow() {
            "allow".into()
        } else {
            "deny".into()
        },
        reason: decision.reason().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn estate() -> Estate {
        estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
    }

    #[test]
    fn worker_cannot_read_other_lane() {
        let e = estate();
        let worker = WorkerClient::new(&e);
        let d = worker
            .invoke("horizon", IntentionKind::MemoryRead, "lane:sanctum")
            .unwrap();
        assert!(!d.is_allow());
    }

    #[test]
    fn worker_declared_tool_without_allow_refuses() {
        let e = estate();
        let worker = WorkerClient::new(&e);
        let denied = worker
            .invoke("research", IntentionKind::Tool, "notes-append")
            .unwrap();
        assert!(!denied.is_allow());
        assert!(denied
            .reason()
            .contains("not covered by an allow Tool intention"));
        assert!(denied.reason().contains("deny-default"));
    }

    #[test]
    fn worker_tool_allow_intention_passes() {
        let mut e = estate();
        e.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "tool:notes-append".into(),
            kind: IntentionKind::Tool,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
        let worker = WorkerClient::new(&e);
        let allowed = worker
            .invoke("research", IntentionKind::Tool, "notes-append")
            .unwrap();
        assert!(allowed.is_allow(), "{}", allowed.reason());
    }

    #[test]
    fn worker_undeclared_mount_fails() {
        let e = estate();
        let worker = WorkerClient::new(&e);
        assert!(!worker
            .invoke("horizon", IntentionKind::Mount, "undeclared")
            .unwrap()
            .is_allow());
    }

    #[test]
    fn worker_declared_mount_without_allow_refuses() {
        let e = estate();
        let worker = WorkerClient::new(&e);
        let denied = worker
            .invoke("research", IntentionKind::Mount, "notes")
            .unwrap();
        assert!(!denied.is_allow());
        assert!(denied
            .reason()
            .contains("not covered by an allow Mount intention"));
        assert!(denied.reason().contains("deny-default"));
    }

    #[test]
    fn worker_mount_allow_intention_passes_and_deny_wins() {
        let mut e = estate();
        e.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "mount:notes".into(),
            kind: IntentionKind::Mount,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
        let worker = WorkerClient::new(&e);
        let allowed = worker
            .invoke("research", IntentionKind::Mount, "notes")
            .unwrap();
        assert!(allowed.is_allow(), "{}", allowed.reason());
        e.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "notes".into(),
            kind: IntentionKind::Mount,
            effect: estate_schema::Effect::Deny,
            note: None,
        });
        let worker = WorkerClient::new(&e);
        let denied = worker
            .invoke("research", IntentionKind::Mount, "mount:notes")
            .unwrap();
        assert!(!denied.is_allow());
        assert!(denied.reason().contains("explicit deny"));
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cell-proxy-audit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn proxy_check_allow_and_deny_leave_feed_lines() {
        let dir = scratch("check");
        let mut e = estate();
        e.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "tool:notes-append".into(),
            kind: IntentionKind::Tool,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
        let worker = WorkerClient::new(&e).with_feed(&dir);
        let allowed = worker
            .invoke("research", IntentionKind::Tool, "notes-append")
            .unwrap();
        assert!(allowed.is_allow(), "{}", allowed.reason());
        let denied = worker
            .invoke("horizon", IntentionKind::MemoryRead, "lane:sanctum")
            .unwrap();
        assert!(!denied.is_allow());
        assert!(
            denied.reason().contains("no intention"),
            "{}",
            denied.reason()
        );
        let lines = feed_collector::proxy_audit_events(&dir).unwrap();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(lines[0].decision.as_deref(), Some("allow"));
        assert_eq!(lines[0].kind, "proxy.tool");
        assert_eq!(lines[1].decision.as_deref(), Some("deny-default"));
        assert!(lines[1].kind.starts_with("proxy."));
        let cursor = feed_collector::load_cursor(&dir).unwrap().expect("cursor");
        assert_eq!(cursor.schema, "cell-one.feed-cursor.v0");
        assert_eq!(cursor.events, 2);
        assert!(cursor
            .last_kind
            .as_deref()
            .unwrap_or("")
            .starts_with("proxy."));
        assert!(cursor.packed_id.is_none());
        assert!(!dir.join("packs").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn grant_agent_call(
        estate: &mut Estate,
        subject: &str,
        target: &str,
        effect: estate_schema::Effect,
    ) {
        estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == subject)
            .unwrap()
            .calls
            .push(estate_schema::CallDecl {
                id: target.into(),
                description: None,
            });
        estate.intentions.push(estate_schema::Intention {
            subject_agent: subject.into(),
            object: format!("agent:{target}"),
            kind: IntentionKind::Agent,
            effect,
            note: None,
        });
    }

    #[test]
    fn agent_allow_and_deny_default_leave_proxy_agent_lines() {
        let dir = scratch("agent-check");
        let mut allowed = estate();
        grant_agent_call(
            &mut allowed,
            "horizon",
            "research",
            estate_schema::Effect::Allow,
        );
        let worker = WorkerClient::new(&allowed).with_feed(&dir);
        let ok = worker
            .invoke("horizon", IntentionKind::Agent, "research")
            .unwrap();
        assert!(ok.is_allow(), "{}", ok.reason());
        let mut bare = estate();
        bare.agents
            .iter_mut()
            .find(|agent| agent.id == "horizon")
            .unwrap()
            .calls
            .push(estate_schema::CallDecl {
                id: "research".into(),
                description: None,
            });
        let missing = WorkerClient::new(&bare)
            .with_feed(&dir)
            .invoke("horizon", IntentionKind::Agent, "agent:research")
            .unwrap();
        assert!(!missing.is_allow());
        assert!(
            missing.reason().contains("deny-default"),
            "{}",
            missing.reason()
        );
        let crossed = WorkerClient::new(&estate())
            .with_feed(&dir)
            .invoke("horizon", IntentionKind::Agent, "research")
            .unwrap();
        assert!(!crossed.is_allow());
        assert!(
            crossed.reason().contains("undeclared") && crossed.reason().contains("deny-default"),
            "{}",
            crossed.reason()
        );
        let lines = feed_collector::proxy_audit_events(&dir).unwrap();
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(
            lines.iter().all(|line| line.kind == "proxy.agent"),
            "{lines:?}"
        );
        assert_eq!(lines[0].decision.as_deref(), Some("allow"));
        assert_eq!(lines[0].object_class.as_deref(), Some("agent"));
        assert_eq!(lines[0].agent_id.as_deref(), Some("horizon"));
        assert_eq!(lines[1].decision.as_deref(), Some("deny-default"));
        assert!(lines[1]
            .note
            .as_deref()
            .unwrap_or("")
            .contains("not covered"));
        assert_eq!(lines[2].decision.as_deref(), Some("deny-default"));
        assert!(lines[2]
            .note
            .as_deref()
            .unwrap_or("")
            .contains("undeclared"));
        let cursor = feed_collector::load_cursor(&dir).unwrap().expect("cursor");
        assert_eq!(cursor.events, 3);
        assert_eq!(cursor.last_kind.as_deref(), Some("proxy.agent"));
        assert!(cursor.packed_id.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn agent_hop_allow_and_deny_default_leave_proxy_agent_lines() {
        let dir = scratch("agent-hop");
        declare_hop(
            &dir,
            HopDecl {
                id: "peer-hop".into(),
                kind: "box".into(),
                capability: "research".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: vec!["horizon".into()],
            },
        )
        .unwrap();
        let mut allowed = estate();
        grant_agent_call(
            &mut allowed,
            "horizon",
            "research",
            estate_schema::Effect::Allow,
        );
        let call = call_hop_for_agent(
            &dir,
            "peer-hop",
            "research",
            "horizon",
            Some(IntentionKind::Agent),
            &allowed,
        )
        .unwrap();
        assert!(call.allow, "{}", call.reason);
        let mut bare = estate();
        bare.agents
            .iter_mut()
            .find(|agent| agent.id == "horizon")
            .unwrap()
            .calls
            .push(estate_schema::CallDecl {
                id: "research".into(),
                description: None,
            });
        let missing = call_hop_for_agent(
            &dir,
            "peer-hop",
            "research",
            "horizon",
            Some(IntentionKind::Agent),
            &bare,
        )
        .unwrap_err();
        assert!(missing.to_string().contains("deny-default"), "{missing}");
        let crossed = call_hop_for_agent(
            &dir,
            "peer-hop",
            "research",
            "horizon",
            Some(IntentionKind::Agent),
            &estate(),
        )
        .unwrap_err();
        assert!(crossed.to_string().contains("undeclared"), "{crossed}");
        let lines = feed_collector::proxy_audit_events(&dir.join("feed")).unwrap();
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(
            lines.iter().all(|line| line.kind == "proxy.agent"),
            "{lines:?}"
        );
        assert_eq!(lines[0].decision.as_deref(), Some("allow"));
        assert_eq!(lines[0].object_class.as_deref(), Some("agent"));
        assert_eq!(lines[1].decision.as_deref(), Some("deny-default"));
        assert_eq!(lines[2].decision.as_deref(), Some("deny-default"));
        assert!(lines[2]
            .note
            .as_deref()
            .unwrap_or("")
            .contains("undeclared"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hop_cannot_be_granted_by_agent_collision() {
        let dir = scratch("agent-collision");
        let mut estate = estate();
        let mut peer = estate
            .agents
            .iter()
            .find(|agent| agent.id == "research")
            .unwrap()
            .clone();
        peer.id = "lane-tool".into();
        peer.display_name = "Lane Tool".into();
        peer.calls.clear();
        estate.agents.push(peer);
        grant_agent_call(
            &mut estate,
            "research",
            "lane-tool",
            estate_schema::Effect::Allow,
        );
        declare_hop(
            &dir,
            HopDecl {
                id: "cell-one-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: vec!["research".into()],
            },
        )
        .unwrap();
        let denied =
            call_hop_for_agent(&dir, "cell-one-box", "lane-tool", "research", None, &estate)
                .unwrap_err();
        assert!(denied.to_string().contains("deny-default"), "{denied}");
        assert!(!denied.to_string().contains("allow intention"), "{denied}");
        let lines = feed_collector::proxy_audit_events(&dir.join("feed")).unwrap();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines[0].kind, "proxy.hop");
        assert_eq!(lines[0].decision.as_deref(), Some("deny-default"));
        assert_ne!(lines[0].decision.as_deref(), Some("allow"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hop_cross_lane_no_intention_audits_deny_default() {
        let dir = scratch("hop-cross-lane");
        declare_hop(
            &dir,
            HopDecl {
                id: "mem-hop".into(),
                kind: "box".into(),
                capability: "sanctum".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: vec!["horizon".into()],
            },
        )
        .unwrap();
        let denied = call_hop_for_agent(
            &dir,
            "mem-hop",
            "sanctum",
            "horizon",
            Some(IntentionKind::MemoryRead),
            &estate(),
        )
        .unwrap_err();
        assert!(denied.to_string().contains("no intention"), "{denied}");
        let lines = feed_collector::proxy_audit_events(&dir.join("feed")).unwrap();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines[0].kind, "proxy.hop");
        assert_eq!(lines[0].decision.as_deref(), Some("deny-default"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn proxy_check_refuses_when_audit_append_fails() {
        let path =
            std::env::temp_dir().join(format!("cell-proxy-audit-file-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, "not-a-dir").unwrap();
        let e = estate();
        let err = WorkerClient::new(&e)
            .with_feed(&path)
            .invoke("research", IntentionKind::Tool, "notes-append")
            .unwrap_err();
        assert!(err.to_string().contains("refuse:proxy-audit"), "{err}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn hop_allow_and_deny_leave_lines_corrupt_refuses_missing_is_empty() {
        let dir = scratch("hop");
        declare_hop(
            &dir,
            HopDecl {
                id: "box-notes".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: Vec::new(),
            },
        )
        .unwrap();
        let call = call_hop(&dir, "box-notes", "notes-append").unwrap();
        assert!(call.allow);
        declare_hop(
            &dir,
            HopDecl {
                id: "cold-box".into(),
                kind: "box".into(),
                capability: "lane-tool".into(),
                host_class: "any".into(),
                wired: false,
                note: None,
                ttl_secs: None,
                agents: Vec::new(),
            },
        )
        .unwrap();
        let denied = call_hop(&dir, "cold-box", "lane-tool").unwrap_err();
        assert!(matches!(denied, MeshError::Ungranted(_)), "{denied}");
        let feed = dir.join("feed");
        let lines = feed_collector::proxy_audit_events(&feed).unwrap();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(lines[0].decision.as_deref(), Some("allow"));
        assert_eq!(lines[0].kind, "proxy.hop");
        assert_eq!(lines[0].object_class.as_deref(), Some("proxy"));
        assert_eq!(lines[1].decision.as_deref(), Some("deny"));
        assert!(lines[1].note.as_deref().unwrap_or("").contains("cold-box"));
        let cursor = feed_collector::load_cursor(&feed).unwrap().expect("cursor");
        assert_eq!(cursor.events, 2);
        assert_eq!(cursor.last_kind.as_deref(), Some("proxy.hop"));
        assert!(cursor.packed_id.is_none());
        let empty = scratch("missing-feed");
        assert!(feed_collector::proxy_audit_events(&empty.join("feed"))
            .unwrap()
            .is_empty());
        assert!(feed_collector::load_cursor(&empty.join("feed"))
            .unwrap()
            .is_none());
        let mut junk = std::fs::OpenOptions::new()
            .append(true)
            .open(feed.join("events.jsonl"))
            .unwrap();
        use std::io::Write;
        writeln!(junk, "not-json").unwrap();
        let err = feed_collector::proxy_audit_events(&feed).unwrap_err();
        assert!(err.to_string().contains("events.jsonl line"), "{err}");
        let blocked = scratch("blocked");
        declare_hop(
            &blocked,
            HopDecl {
                id: "box-notes".into(),
                kind: "box".into(),
                capability: "notes-append".into(),
                host_class: "any".into(),
                wired: true,
                note: None,
                ttl_secs: None,
                agents: Vec::new(),
            },
        )
        .unwrap();
        std::fs::write(blocked.join("feed"), "not-a-dir").unwrap();
        let err = call_hop(&blocked, "box-notes", "notes-append").unwrap_err();
        assert!(matches!(err, MeshError::ProxyAudit(_)), "{err}");
        assert!(err.to_string().contains("refuse:proxy-audit"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&empty);
        let _ = std::fs::remove_dir_all(&blocked);
    }
}
