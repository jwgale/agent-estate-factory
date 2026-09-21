//! Backup rotate: keep last N `cell-backup-*` dirs. No live Mac / GPU.

use floor_supervisor::{list_cell_backups, prune_cell_backups};
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "cell-one-prune-{}-{}-{}",
        name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed(out: &std::path::Path, names: &[&str]) {
    for name in names {
        std::fs::create_dir_all(out.join(name)).unwrap();
        std::fs::write(out.join(name).join("MANIFEST.md"), name).unwrap();
    }
}

#[test]
fn prune_keeps_newest_n_and_refuses_zero() {
    let out = tmp("rotate");
    seed(
        &out,
        &[
            "cell-backup-unix0000000001-000000001",
            "cell-backup-unix0000000002-000000001",
            "cell-backup-unix0000000003-000000001",
            "cell-backup-unix0000000004-000000001",
            "not-a-backup",
        ],
    );
    let listed = list_cell_backups(&out).unwrap();
    assert_eq!(listed.len(), 4);
    assert!(listed[0]
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .contains("0000000004"));

    let err = prune_cell_backups(&out, 0).unwrap_err().to_string();
    assert!(err.contains("refuse:prune"));
    assert_eq!(list_cell_backups(&out).unwrap().len(), 4);

    let report = prune_cell_backups(&out, 2).unwrap();
    assert_eq!(report.schema, "cell-one.backup-prune.v0");
    assert_eq!(report.keep, 2);
    assert_eq!(report.kept.len(), 2);
    assert_eq!(report.removed.len(), 2);
    let left = list_cell_backups(&out).unwrap();
    assert_eq!(left.len(), 2);
    let names: Vec<String> = left
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().any(|n| n.contains("0000000004")));
    assert!(names.iter().any(|n| n.contains("0000000003")));
    assert!(!names.iter().any(|n| n.contains("0000000001")));
    assert!(out.join("not-a-backup").is_dir());
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn prune_keeps_all_when_fewer_than_n() {
    let out = tmp("short");
    seed(&out, &["cell-backup-unix0000000010-000000000"]);
    let report = prune_cell_backups(&out, 5).unwrap();
    assert_eq!(report.kept.len(), 1);
    assert!(report.removed.is_empty());
    assert_eq!(list_cell_backups(&out).unwrap().len(), 1);
    let _ = std::fs::remove_dir_all(&out);
}
