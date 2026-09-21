//! Swappable isolation. Floor core talks to this trait only — no vendor ids.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("isolation driver error: {0}")]
pub struct IsolationError(pub String);

#[derive(Debug, Clone)]
pub struct BindRequest<'a> {
    pub agent_id: &'a str,
    pub desktop: &'a str,
    pub lane_id: &'a str,
    pub lane_root: &'a Path,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IsolationHandle {
    pub driver: String,
    pub key: String,
    pub path: PathBuf,
}

impl IsolationHandle {
    pub fn as_token(&self) -> String {
        format!("{}:{}", self.driver, self.key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoundSession {
    pub agent_id: String,
    pub desktop: String,
    pub lane_id: String,
    pub isolation_handle: IsolationHandle,
    pub session_path: PathBuf,
    pub lane_root: PathBuf,
}

pub trait IsolationDriver: Send + Sync {
    fn name(&self) -> &'static str;
    fn bind(&self, req: &BindRequest<'_>) -> Result<BoundSession, IsolationError>;
}

/// Cell One default: one profile directory per agent. Warm desktop, disposable.
pub struct ProfileDirDriver {
    pub sessions_root: PathBuf,
}

impl ProfileDirDriver {
    pub fn new(sessions_root: impl Into<PathBuf>) -> Self {
        Self {
            sessions_root: sessions_root.into(),
        }
    }
}

impl IsolationDriver for ProfileDirDriver {
    fn name(&self) -> &'static str {
        "profile-dir"
    }

    fn bind(&self, req: &BindRequest<'_>) -> Result<BoundSession, IsolationError> {
        let session_path = self.sessions_root.join(req.agent_id);
        let profile = session_path.join("profile");
        std::fs::create_dir_all(&profile).map_err(|e| IsolationError(e.to_string()))?;
        std::fs::create_dir_all(req.lane_root).map_err(|e| IsolationError(e.to_string()))?;

        let handle = IsolationHandle {
            driver: self.name().to_string(),
            key: req.agent_id.to_string(),
            path: session_path.clone(),
        };
        let bound = BoundSession {
            agent_id: req.agent_id.to_string(),
            desktop: req.desktop.to_string(),
            lane_id: req.lane_id.to_string(),
            isolation_handle: handle,
            session_path: session_path.clone(),
            lane_root: req.lane_root.to_path_buf(),
        };
        let record = serde_json::json!({
            "agent_id": bound.agent_id,
            "desktop": bound.desktop,
            "lane_id": bound.lane_id,
            "isolation_handle": bound.isolation_handle.as_token(),
            "lane_root": bound.lane_root,
            "note": "session dirs are disposable warm desktops; lane roots persist"
        });
        std::fs::write(
            session_path.join("session.json"),
            serde_json::to_string_pretty(&record).unwrap_or_default(),
        )
        .map_err(|e| IsolationError(e.to_string()))?;
        Ok(bound)
    }
}

/// In-memory driver for tests — proves the supervisor does not require profile dirs.
#[derive(Default)]
pub struct MemoryDriver {
    pub bound: std::sync::Mutex<Vec<BoundSession>>,
}

impl IsolationDriver for MemoryDriver {
    fn name(&self) -> &'static str {
        "memory"
    }

    fn bind(&self, req: &BindRequest<'_>) -> Result<BoundSession, IsolationError> {
        let bound = BoundSession {
            agent_id: req.agent_id.to_string(),
            desktop: req.desktop.to_string(),
            lane_id: req.lane_id.to_string(),
            isolation_handle: IsolationHandle {
                driver: self.name().to_string(),
                key: req.agent_id.to_string(),
                path: PathBuf::from(format!("memory://{}", req.agent_id)),
            },
            session_path: PathBuf::from(format!("memory://{}", req.agent_id)),
            lane_root: req.lane_root.to_path_buf(),
        };
        self.bound
            .lock()
            .map_err(|e| IsolationError(e.to_string()))?
            .push(bound.clone());
        Ok(bound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn profile_dir_creates_session() {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let tmp = std::env::temp_dir().join(format!("cell-one-iso-{n}"));
        let _ = std::fs::remove_dir_all(&tmp);
        let driver = ProfileDirDriver::new(tmp.join("sessions"));
        let lane = tmp.join("lane");
        let bound = driver
            .bind(&BindRequest {
                agent_id: "horizon",
                desktop: "horizon-desktop",
                lane_id: "horizon",
                lane_root: &lane,
            })
            .unwrap();
        assert_eq!(bound.isolation_handle.driver, "profile-dir");
        assert!(bound.session_path.join("profile").is_dir());
        assert!(bound.session_path.join("session.json").is_file());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
