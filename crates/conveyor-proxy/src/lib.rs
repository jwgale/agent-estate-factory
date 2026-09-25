//! Deny-by-default conveyor. Workers call this; they do not read other lanes.
//! Capability mesh is a sibling seam (`mesh`) — declare hop, lease-bound call.

mod mesh;

pub use mesh::{
    authority_report, call_hop, call_hop_for_agent, declare_hop, forget_expired_hop_leases,
    hop_driver, hop_from_placement, hop_kind_for_placement, hop_lease_is_expired, hop_now_unix,
    list_expired_hop_leases, list_hop_leases, list_hops, load_mesh, mesh_file_sot, persist_mesh,
    refuse_mesh_host_classes, slim_parse_placement_actual, sync_from_placements, AuthorityRow,
    BoxHop, CloudMeshHop, ConveyorHop, ConveyorMesh, HopCall, HopDecl, HopLease, MeshError,
    MESH_FILE, MESH_SCHEMA,
};

use estate_schema::{authorize, AccessRequest, Decision, Estate, IntentionKind};
use feed_collector::{append_event, ScrubbedEvent};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

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

    pub fn invoke(&self, agent_id: &str, kind: IntentionKind, object: &str) -> Decision {
        check(self.estate, agent_id, kind, object, self.feed_dir)
    }
}

pub fn check(
    estate: &Estate,
    agent_id: &str,
    kind: IntentionKind,
    object: &str,
    feed_dir: Option<&Path>,
) -> Decision {
    let decision = authorize(
        estate,
        &AccessRequest {
            subject_agent: agent_id,
            kind,
            object,
        },
    );
    if let Some(dir) = feed_dir {
        let _ = append_event(
            dir,
            &ScrubbedEvent {
                kind: format!("proxy.{}", kind.as_str()),
                agent_id: Some(agent_id.to_string()),
                decision: Some(if decision.is_allow() {
                    "allow".into()
                } else {
                    "deny".into()
                }),
                object_class: Some(kind.as_str().to_string()),
                note: None,
                ts: String::new(),
            },
        );
    }
    decision
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
        let d = worker.invoke("horizon", IntentionKind::MemoryRead, "lane:sanctum");
        assert!(!d.is_allow());
    }

    #[test]
    fn worker_declared_tool_without_allow_refuses() {
        let e = estate();
        let worker = WorkerClient::new(&e);
        let denied = worker.invoke("research", IntentionKind::Tool, "notes-append");
        assert!(!denied.is_allow());
        assert!(denied.reason().contains("not covered by an allow Tool intention"));
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
        let allowed = worker.invoke("research", IntentionKind::Tool, "notes-append");
        assert!(allowed.is_allow(), "{}", allowed.reason());
    }

    #[test]
    fn worker_undeclared_mount_fails() {
        let e = estate();
        let worker = WorkerClient::new(&e);
        assert!(!worker
            .invoke("horizon", IntentionKind::Mount, "undeclared")
            .is_allow());
    }
}
