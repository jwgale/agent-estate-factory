//! Pause-kit proof: apply → suspend → drop sessions → resume from disk.
//!
//! Leases stay. Cloud-agent stays unspawned. Runtime is regenerable.

use crate::{
    apply_with_profile_dir, load_placements, resume, stop_runtime, suspend, SupervisorError,
};
use estate_schema::Estate;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PauseProof {
    pub schema: String,
    pub applied: bool,
    pub suspended: bool,
    pub sessions_dropped: bool,
    pub resumed: bool,
    pub leases_survived: bool,
    pub cloud_spawned: bool,
    pub in_sync: bool,
    pub note: String,
}

/// Automated pause-kit: kill-process simulation is `rm -rf sessions/`.
pub fn pause_kit_proof(
    estate: &Estate,
    state_dir: &Path,
    roots_base: &Path,
) -> Result<PauseProof, SupervisorError> {
    apply_with_profile_dir(estate, state_dir, roots_base)?;
    let after_apply = load_placements(state_dir)?.ok_or_else(|| {
        SupervisorError::Other("pause-proof: apply wrote no leases".into())
    })?;
    let applied_leases = after_apply.leases.len();
    if applied_leases == 0 {
        return Err(SupervisorError::Other("pause-proof: no leases after apply".into()));
    }
    suspend(state_dir)?;
    // Kill-process simulation: sessions/ already gone after suspend; drop again.
    stop_runtime(state_dir)?;
    let sessions = state_dir.join("sessions");
    if sessions.exists() {
        std::fs::remove_dir_all(&sessions)?;
    }
    let sessions_dropped = !sessions.exists();
    let mid = load_placements(state_dir)?.ok_or_else(|| {
        SupervisorError::Other("pause-proof: leases vanished after kill simulation".into())
    })?;
    let leases_survived = mid.leases.len() == applied_leases;
    if !leases_survived {
        return Err(SupervisorError::Other(
            "pause-proof: lease count changed after sessions drop".into(),
        ));
    }
    let (actual, _) = resume(estate, state_dir, roots_base)?;
    let after = load_placements(state_dir)?.ok_or_else(|| {
        SupervisorError::Other("pause-proof: no leases after resume".into())
    })?;
    let cloud_spawned = after
        .leases
        .iter()
        .any(|l| l.kind == "cloud-agent" && l.spawned);
    if cloud_spawned {
        return Err(SupervisorError::Other(
            "pause-proof: cloud-agent spawned (fail closed)".into(),
        ));
    }
    if actual.sessions.is_empty() {
        return Err(SupervisorError::Other(
            "pause-proof: resume bound no sessions".into(),
        ));
    }
    let drift = crate::drift_with_roots(estate, state_dir, Some(roots_base))?;
    Ok(PauseProof {
        schema: "cell-one.pause-proof.v0".into(),
        applied: true,
        suspended: true,
        sessions_dropped,
        resumed: true,
        leases_survived,
        cloud_spawned,
        in_sync: drift.in_sync,
        note: "Pause-kit proof. Leases restored from disk. Cloud-agent not spawned.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> Estate {
        estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
    }

    #[test]
    fn apply_suspend_drop_sessions_resume_keeps_leases() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("cell-one-pause-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let proof = pause_kit_proof(&example(), &root.join("state"), &root).unwrap();
        assert!(proof.applied);
        assert!(proof.suspended);
        assert!(proof.sessions_dropped);
        assert!(proof.resumed);
        assert!(proof.leases_survived);
        assert!(!proof.cloud_spawned);
        assert!(proof.in_sync);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unchanged_apply_then_suspend_resume_stays_in_sync() {
        use crate::{apply_with_profile_dir, classify_apply, drift_with_roots, ApplyIdentity};
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "cell-one-pause-idem-{n}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let state = root.join("state");
        let estate = example();
        apply_with_profile_dir(&estate, &state, &root).unwrap();
        assert_eq!(
            classify_apply(&estate, &state, &root).unwrap(),
            ApplyIdentity::Unchanged
        );
        suspend(&state).unwrap();
        let (actual, _) = resume(&estate, &state, &root).unwrap();
        assert!(!actual.sessions.is_empty());
        assert!(drift_with_roots(&estate, &state, Some(&root)).unwrap().in_sync);
        let proof = pause_kit_proof(&estate, &state, &root).unwrap();
        assert!(proof.leases_survived);
        assert!(!proof.cloud_spawned);
        assert!(proof.in_sync);
        let _ = std::fs::remove_dir_all(&root);
    }
}
