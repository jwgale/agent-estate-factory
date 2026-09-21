//! Timestamped `.cell/` archive. Pause-safe. Not a remote upload.
//!
//! Restores refuse on sacred mismatch. Dry-run writes nothing.

use crate::{load_desired_snapshot, load_placements, SupervisorError};
use estate_schema::{is_sacred_name, locked_sacred_ids, normalize_name, Estate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const BACKUP_SCHEMA: &str = "cell-one.cell-backup.v0";
pub const BACKUP_META: &str = "backup.json";

const DURABLE_FILES: &[&str] = &[
    "placement-actual.json",
    "lifecycle.json",
    "lifecycle.jsonl",
    "apply-audit.jsonl",
    "sessions.jsonl",
    "conveyor-mesh.json",
    "conveyor-hops.json",
    "conveyor-leases.json",
    "desired-snapshot.yaml",
    "actual-state.json",
    "catalog.json",
    "reconcile.json",
    "reconcile.md",
    "model-actual.json",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CellBackup {
    #[serde(default = "default_backup_schema")]
    pub schema: String,
    pub created_at: String,
    pub state_dir: String,
    pub files: Vec<String>,
    #[serde(default)]
    pub sacred_ids: Vec<String>,
    pub writes: bool,
    pub cloud_agent_spawned: bool,
    pub note: String,
}

fn default_backup_schema() -> String {
    BACKUP_SCHEMA.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoreReport {
    pub schema: String,
    pub archive: String,
    pub writes: bool,
    pub would_refuse: bool,
    pub files: Vec<String>,
    pub sacred_ids: Vec<String>,
    pub refuses: Vec<String>,
    pub note: String,
}

pub fn sacred_id_set(estate: Option<&Estate>) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = locked_sacred_ids()
        .into_iter()
        .map(normalize_name)
        .collect();
    if let Some(estate) = estate {
        for ex in &estate.sacred_exclusions {
            set.insert(normalize_name(&ex.id));
        }
    }
    set
}

pub fn sacred_mismatch(backup: &BTreeSet<String>, current: &BTreeSet<String>) -> bool {
    if backup.is_empty() || current.is_empty() {
        return false;
    }
    !backup.is_subset(current) || !current.is_subset(backup)
}

fn copy_if_exists(src: &Path, dest: &Path, copied: &mut Vec<String>) -> Result<bool, SupervisorError> {
    if !src.is_file() {
        return Ok(false);
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, dest)?;
    copied.push(src.display().to_string());
    Ok(true)
}

fn copy_tree(src: &Path, dest: &Path, copied: &mut Vec<String>) -> Result<(), SupervisorError> {
    if !src.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().unwrap_or_default();
            std::fs::copy(&path, dest.join(name))?;
            copied.push(path.display().to_string());
        } else if path.is_dir() {
            copy_tree(&path, &dest.join(path.file_name().unwrap_or_default()), copied)?;
        }
    }
    Ok(())
}

fn stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime.now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix{secs}")
}

/// Copy durable `.cell/` files (and optional plans) to a timestamped folder.
pub fn backup_cell(
    state_dir: &Path,
    plans_dir: Option<&Path>,
    out_dir: &Path,
    estate: Option<&Estate>,
) -> Result<(PathBuf, CellBackup), SupervisorError> {
    std::fs::create_dir_all(out_dir)?;
    let dest = out_dir.join(format!("cell-backup-{}", stamp()));
    std::fs::create_dir_all(dest.join("cell"))?;
    let mut files = Vec::new();
    for name in DURABLE_FILES {
        let _ = copy_if_exists(&state_dir.join(name), &dest.join("cell").join(name), &mut files)?;
    }
    let feed = state_dir.join("feed");
    if feed.is_dir() {
        copy_tree(&feed, &dest.join("cell").join("feed"), &mut files)?;
    }
    if let Some(plans) = plans_dir {
        if plans.is_dir() {
            copy_tree(plans, &dest.join("plans"), &mut files)?;
        }
    }
    let sacred: Vec<String> = sacred_id_set(estate).into_iter().collect();
    let meta = CellBackup {
        schema: BACKUP_SCHEMA.into(),
        created_at: format!("unix:{}", crate::now_unix()),
        state_dir: state_dir.display().to_string(),
        files: files.clone(),
        sacred_ids: sacred,
        writes: true,
        cloud_agent_spawned: false,
        note: "Local cell archive. Not uploaded. Restore refuses sacred mismatch.".into(),
    };
    std::fs::write(
        dest.join(BACKUP_META),
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    )?;
    let mut md = String::from("Cell One backup (local only)\n============================\n");
    md.push_str("Not a remote upload. Restore is fail-closed on sacred mismatch.\n\n");
    for f in &files {
        md.push_str(&format!("- {f}\n"));
    }
    std::fs::write(dest.join("MANIFEST.md"), md)?;
    Ok((dest, meta))
}

fn load_backup_meta(archive: &Path) -> Result<CellBackup, SupervisorError> {
    let path = archive.join(BACKUP_META);
    if !path.is_file() {
        return Err(SupervisorError::Other(format!(
            "refuse:backup: missing {BACKUP_META} under {}",
            archive.display()
        )));
    }
    let text = std::fs::read_to_string(&path)?;
    serde_json::from_str(&text)
        .map_err(|e| SupervisorError::Other(format!("backup.json: {e}")))
}

fn scan_sacred_in_archive(archive: &Path) -> Result<Vec<String>, SupervisorError> {
    let mut hits = Vec::new();
    let snap = archive.join("cell").join("desired-snapshot.yaml");
    if snap.is_file() {
        if let Some(estate) = load_desired_snapshot(&archive.join("cell"))? {
            for agent in &estate.agents {
                if is_sacred_name(&agent.id) || is_sacred_name(&agent.display_name) {
                    hits.push(format!("refuse:sacred-id: snapshot agent '{}'", agent.id));
                }
            }
        }
    }
    if let Some(places) = load_placements(&archive.join("cell"))? {
        for lease in places.leases {
            for agent in lease.agents {
                if is_sacred_name(&agent) {
                    hits.push(format!(
                        "refuse:sacred-id: lease {} binds '{}'",
                        lease.placement_id, agent
                    ));
                }
            }
        }
    }
    Ok(hits)
}

/// Restore durable files. Dry-run writes nothing. Sacred mismatch is refuse.
pub fn restore_cell(
    archive: &Path,
    state_dir: &Path,
    plans_dir: Option<&Path>,
    current: Option<&Estate>,
    dry_run: bool,
) -> Result<RestoreReport, SupervisorError> {
    let meta = load_backup_meta(archive)?;
    let mut refuses = scan_sacred_in_archive(archive)?;
    let backup_sacred: BTreeSet<String> = meta.sacred_ids.iter().map(|s| normalize_name(s)).collect();
    let current_sacred = sacred_id_set(current);
    if sacred_mismatch(&backup_sacred, &current_sacred) {
        refuses.push(
            "refuse:sacred-mismatch: backup sacred exclusions do not match this estate".into(),
        );
    }
    let cell_src = archive.join("cell");
    let mut files = Vec::new();
    if cell_src.is_dir() {
        for name in DURABLE_FILES {
            if cell_src.join(name).is_file() {
                files.push((*name).to_string());
            }
        }
        if cell_src.join("feed").is_dir() {
            files.push("feed/".into());
        }
    }
    let plans_src = archive.join("plans");
    if plans_src.is_dir() {
        files.push("plans/".into());
    }
    let would_refuse = !refuses.is_empty();
    if dry_run || would_refuse {
        return Ok(RestoreReport {
            schema: BACKUP_SCHEMA.into(),
            archive: archive.display().to_string(),
            writes: false,
            would_refuse,
            files,
            sacred_ids: meta.sacred_ids,
            refuses,
            note: if would_refuse {
                "dry-run/refuse: no files written".into()
            } else {
                "dry-run: no files written".into()
            },
        });
    }
    std::fs::create_dir_all(state_dir)?;
    for name in DURABLE_FILES {
        let src = cell_src.join(name);
        if src.is_file() {
            std::fs::copy(&src, state_dir.join(name))?;
        }
    }
    if cell_src.join("feed").is_dir() {
        copy_tree(&cell_src.join("feed"), &state_dir.join("feed"), &mut Vec::new())?;
    }
    if let Some(plans) = plans_dir {
        if plans_src.is_dir() {
            copy_tree(&plans_src, plans, &mut Vec::new())?;
        }
    }
    Ok(RestoreReport {
        schema: BACKUP_SCHEMA.into(),
        archive: archive.display().to_string(),
        writes: true,
        would_refuse: false,
        files,
        sacred_ids: meta.sacred_ids,
        refuses,
        note: "Restored durable cell files. sessions/ stay regenerable. Cloud-agent not spawned."
            .into(),
    })
}

pub fn render_restore(report: &RestoreReport) -> String {
    let mut out = String::from("Cell restore\n============\n");
    out.push_str(&format!("archive: {}\n", report.archive));
    out.push_str(&format!("writes: {}\n", report.writes));
    out.push_str(&format!("would_refuse: {}\n", report.would_refuse));
    out.push_str("\nFiles\n-----\n");
    if report.files.is_empty() {
        out.push_str("(none)\n");
    } else {
        for f in &report.files {
            out.push_str(&format!("  {f}\n"));
        }
    }
    out.push_str("\nRefuses\n-------\n");
    if report.refuses.is_empty() {
        out.push_str("(none)\n");
    } else {
        for r in &report.refuses {
            out.push_str(&format!("  {r}\n"));
        }
    }
    out.push_str(&format!("\n{}\n", report.note));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{apply_with_profile_dir, load_placements};

    fn example() -> Estate {
        estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
    }

    fn tmp() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("cell-one-bak-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn sacred_mismatch_detects_unequal_sets() {
        let current = sacred_id_set(None);
        assert!(!current.is_empty());
        let mut fake = BTreeSet::new();
        fake.insert("not-the-locked-set".into());
        assert!(sacred_mismatch(&fake, &current));
        assert!(!sacred_mismatch(&current, &current));
        assert!(!sacred_mismatch(&BTreeSet::new(), &current));
    }

    #[test]
    fn backup_and_dry_run_restore_round_trip() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        apply_with_profile_dir(&estate, &state, &root).unwrap();
        let (archive, meta) = backup_cell(&state, None, &root.join("backups"), Some(&estate)).unwrap();
        assert!(archive.join(BACKUP_META).is_file());
        assert!(!meta.sacred_ids.is_empty());
        assert!(archive.join("cell").join("placement-actual.json").is_file());
        let dry = restore_cell(&archive, &root.join("empty"), None, Some(&estate), true).unwrap();
        assert!(!dry.writes);
        assert!(!dry.would_refuse);
        assert!(!root.join("empty").join("placement-actual.json").exists());
        let restored = restore_cell(&archive, &root.join("restored"), None, Some(&estate), false)
            .unwrap();
        assert!(restored.writes);
        assert!(root.join("restored").join("placement-actual.json").is_file());
        let leases = load_placements(&root.join("restored")).unwrap().unwrap();
        assert!(leases.leases.iter().all(|l| l.kind != "cloud-agent" || !l.spawned));
        let _ = std::fs::remove_dir_all(&root);
    }
}
