//! Timestamped `.cell/` archive. Pause-safe. Not a remote upload.
//!
//! Restores refuse on sacred mismatch. Dry-run writes nothing.
//! A catalog that disagrees with the binding, a frontier `source_driver`
//! with no frontier binding, and a snapshot sacred set that disagrees
//! refuse before the archive or the restore write. The schema card is
//! not the binding.

use crate::{load_desired_snapshot, load_placements, SupervisorError};
use estate_schema::{
    canonical_host_class_opt, contains_sku, is_sacred_name, locked_sacred_ids, normalize_name,
    Estate, ModelClass,
};
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
pub struct PruneReport {
    pub schema: String,
    pub keep: usize,
    pub kept: Vec<String>,
    pub removed: Vec<String>,
    pub note: String,
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

/// Symmetric match. Empty/missing backup `sacred_ids` is a mismatch against a
/// locked current set (fail closed). Aliases are already collapsed by
/// `sacred_id_set` / `normalize_name`.
pub fn sacred_mismatch(backup: &BTreeSet<String>, current: &BTreeSet<String>) -> bool {
    backup != current
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
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("unix{:010}-{:09}", dur.as_secs(), dur.subsec_nanos())
}

fn backup_sort_key(path: &Path) -> (u64, u64) {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let rest = name.strip_prefix("cell-backup-unix").unwrap_or("");
    let mut parts = rest.split('-');
    let secs = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let nanos = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (secs, nanos)
}

/// Newest-first list of `cell-backup-*` directories.
pub fn list_cell_backups(out_dir: &Path) -> Result<Vec<PathBuf>, SupervisorError> {
    if !out_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut dirs = Vec::new();
    for entry in std::fs::read_dir(out_dir)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if path.is_dir() && name.starts_with("cell-backup-") {
            dirs.push(path);
        }
    }
    dirs.sort_by(|a, b| backup_sort_key(b).cmp(&backup_sort_key(a)));
    Ok(dirs)
}

/// Keep the newest `keep` cell backups. `keep == 0` refuses. Does not spawn.
pub fn prune_cell_backups(
    out_dir: &Path,
    keep: usize,
) -> Result<PruneReport, SupervisorError> {
    if keep == 0 {
        return Err(SupervisorError::Other(
            "refuse:prune: keep must be >= 1".into(),
        ));
    }
    let dirs = list_cell_backups(out_dir)?;
    let mut kept = Vec::new();
    let mut removed = Vec::new();
    for (i, dir) in dirs.into_iter().enumerate() {
        if i < keep {
            kept.push(dir.display().to_string());
        } else {
            std::fs::remove_dir_all(&dir)?;
            removed.push(dir.display().to_string());
        }
    }
    Ok(PruneReport {
        schema: "cell-one.backup-prune.v0".into(),
        keep,
        kept,
        removed,
        note: "Newest archives kept. Older cell-backup-* dirs deleted. Local only.".into(),
    })
}

include!("backup_honesty.rs");

/// Copy durable `.cell/` files (and optional plans) to a timestamped folder.
pub fn backup_cell(
    state_dir: &Path,
    plans_dir: Option<&Path>,
    out_dir: &Path,
    estate: Option<&Estate>,
) -> Result<(PathBuf, CellBackup), SupervisorError> {
    let refuses = cell_inconsistencies(state_dir, plans_dir, estate)?;
    if !refuses.is_empty() {
        return Err(SupervisorError::Other(refuses.join("; ")));
    }
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
    crate::write_pretty_json(&dest.join(BACKUP_META), &meta)?;
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

fn scan_mesh_file_host_class(path: &Path) -> Result<Vec<String>, SupervisorError> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path)?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| SupervisorError::Other(format!("{}: {e}", path.display())))?;
    let mut hits = Vec::new();
    let mut rows = Vec::new();
    if let Some(hops) = v.get("hops").and_then(|x| x.as_array()) {
        rows.extend(hops);
    }
    if let Some(leases) = v.get("leases").and_then(|x| x.as_array()) {
        rows.extend(leases);
    }
    for row in rows {
        let host = row.get("host_class").and_then(|x| x.as_str()).unwrap_or("");
        if canonical_host_class_opt(Some(host)).is_none() {
            let id = row
                .get("id")
                .or_else(|| row.get("hop_id"))
                .and_then(|x| x.as_str())
                .unwrap_or("?");
            hits.push(format!(
                "refuse:bad-host-class: hop/lease '{id}' host_class '{host}' must be consumer-nvidia|apple-silicon|rented-nvidia|any"
            ));
        }
    }
    Ok(hits)
}

fn scan_host_class_in_archive(archive: &Path) -> Result<Vec<String>, SupervisorError> {
    let mut hits = Vec::new();
    if let Some(places) = load_placements(&archive.join("cell"))? {
        for lease in &places.leases {
            if canonical_host_class_opt(Some(lease.host_class.as_str())).is_none() {
                hits.push(format!(
                    "refuse:bad-host-class: lease '{}' host_class '{}' must be consumer-nvidia|apple-silicon|rented-nvidia|any",
                    lease.placement_id, lease.host_class
                ));
            }
        }
    }
    for name in ["conveyor-mesh.json", "conveyor-hops.json", "conveyor-leases.json"] {
        hits.extend(scan_mesh_file_host_class(&archive.join("cell").join(name))?);
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
    refuses.extend(scan_host_class_in_archive(archive)?);
    let backup_sacred: BTreeSet<String> = meta.sacred_ids.iter().map(|s| normalize_name(s)).collect();
    let current_sacred = sacred_id_set(current);
    if sacred_mismatch(&backup_sacred, &current_sacred) {
        refuses.push(
            "refuse:sacred-mismatch: backup sacred exclusions do not match this estate".into(),
        );
    }
    let cell_src = archive.join("cell");
    let plans_src_for_check = archive.join("plans");
    let plans_for_check = if plans_src_for_check.exists() {
        Some(plans_src_for_check.as_path())
    } else {
        None
    };
    refuses.extend(cell_inconsistencies(&cell_src, plans_for_check, current)?);
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
    use crate::{apply_with_profile_dir, load_desired_snapshot, load_placements, write_desired_snapshot};

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
        assert!(
            sacred_mismatch(&BTreeSet::new(), &current),
            "empty backup sacred_ids must not skip the check"
        );
        assert!(!sacred_mismatch(&BTreeSet::new(), &BTreeSet::new()));
    }

    #[test]
    fn empty_backup_sacred_ids_refuses_restore_writes() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        apply_with_profile_dir(&estate, &state, &root).unwrap();
        let (archive, _) =
            backup_cell(&state, None, &root.join("backups"), Some(&estate)).unwrap();
        let meta_path = archive.join(BACKUP_META);
        let mut meta: CellBackup =
            serde_json::from_str(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        meta.sacred_ids.clear();
        std::fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap()).unwrap();
        let dest = root.join("restored");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("placement-actual.json"), "sentinel\n").unwrap();
        let report = restore_cell(&archive, &dest, None, Some(&estate), false).unwrap();
        assert!(!report.writes);
        assert!(report.would_refuse);
        assert!(
            report
                .refuses
                .iter()
                .any(|r| r.contains("refuse:sacred-mismatch")),
            "{:?}",
            report.refuses
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("placement-actual.json")).unwrap(),
            "sentinel\n"
        );
        let _ = std::fs::remove_dir_all(&root);
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

    fn tamper_archive_box_host_class(archive: &Path, host_class: &str) {
        let path = archive.join("cell").join("placement-actual.json");
        let raw = std::fs::read_to_string(&path).unwrap();
        let mut v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let leases = v["leases"].as_array_mut().expect("leases");
        let box_lease = leases
            .iter_mut()
            .find(|l| l["placement_id"] == "cell-one-box")
            .expect("cell-one-box");
        box_lease["host_class"] = serde_json::Value::String(host_class.into());
        std::fs::write(&path, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    }

    #[test]
    fn restore_refuses_sku_placement_actual_and_writes_nothing() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        apply_with_profile_dir(&estate, &state, &root).unwrap();
        let (archive, _) =
            backup_cell(&state, None, &root.join("backups"), Some(&estate)).unwrap();
        tamper_archive_box_host_class(&archive, "not-a-host");
        let dest = root.join("restored");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("placement-actual.json"), "sentinel\n").unwrap();
        let dry = restore_cell(&archive, &dest, None, Some(&estate), true).unwrap();
        assert!(!dry.writes);
        assert!(dry.would_refuse);
        assert!(
            dry.refuses
                .iter()
                .any(|r| r.contains("refuse:bad-host-class") && r.contains("not-a-host")),
            "{:?}",
            dry.refuses
        );
        let live = restore_cell(&archive, &dest, None, Some(&estate), false).unwrap();
        assert!(!live.writes);
        assert!(live.would_refuse);
        assert_eq!(
            std::fs::read_to_string(dest.join("placement-actual.json")).unwrap(),
            "sentinel\n"
        );
        let archived = std::fs::read_to_string(archive.join("cell").join("placement-actual.json"))
            .unwrap();
        assert!(archived.contains("not-a-host"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_refuses_sku_mesh_and_writes_nothing() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        apply_with_profile_dir(&estate, &state, &root).unwrap();
        let (archive, _) =
            backup_cell(&state, None, &root.join("backups"), Some(&estate)).unwrap();
        let mesh = serde_json::json!({
            "schema": "cell-one.conveyor-mesh.v0",
            "hops": [{
                "id": "cell-one-box",
                "kind": "box",
                "capability": "lane-tool",
                "host_class": "not-a-host",
                "wired": true
            }],
            "leases": [{
                "hop_id": "cell-one-box",
                "kind": "box",
                "capability": "lane-tool",
                "host_class": "not-a-host",
                "granted": true,
                "spawned": true,
                "durable": true,
                "driver": "box"
            }]
        });
        std::fs::write(
            archive.join("cell").join("conveyor-mesh.json"),
            serde_json::to_string_pretty(&mesh).unwrap(),
        )
        .unwrap();
        let dest = root.join("restored");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("conveyor-mesh.json"), "sentinel\n").unwrap();
        let report = restore_cell(&archive, &dest, None, Some(&estate), false).unwrap();
        assert!(!report.writes);
        assert!(report.would_refuse);
        assert!(
            report
                .refuses
                .iter()
                .any(|r| r.contains("refuse:bad-host-class") && r.contains("not-a-host")),
            "{:?}",
            report.refuses
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("conveyor-mesh.json")).unwrap(),
            "sentinel\n"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_refuses_garbage_placement_actual_json() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        apply_with_profile_dir(&estate, &state, &root).unwrap();
        let (archive, _) =
            backup_cell(&state, None, &root.join("backups"), Some(&estate)).unwrap();
        std::fs::write(
            archive.join("cell").join("placement-actual.json"),
            "{not-json\n",
        )
        .unwrap();
        let dest = root.join("restored");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("placement-actual.json"), "sentinel\n").unwrap();
        let err = restore_cell(&archive, &dest, None, Some(&estate), false).unwrap_err();
        assert!(
            err.to_string().contains("placement-actual"),
            "{err}"
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("placement-actual.json")).unwrap(),
            "sentinel\n"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_refuses_garbage_mesh_json() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        apply_with_profile_dir(&estate, &state, &root).unwrap();
        let (archive, _) =
            backup_cell(&state, None, &root.join("backups"), Some(&estate)).unwrap();
        std::fs::write(
            archive.join("cell").join("conveyor-mesh.json"),
            "{not-json\n",
        )
        .unwrap();
        let dest = root.join("restored");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("conveyor-mesh.json"), "sentinel\n").unwrap();
        let err = restore_cell(&archive, &dest, None, Some(&estate), false).unwrap_err();
        assert!(
            err.to_string().contains("conveyor-mesh") || err.to_string().contains("expected"),
            "{err}"
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("conveyor-mesh.json")).unwrap(),
            "sentinel\n"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    include!("backup_honesty_tests.rs");
}
