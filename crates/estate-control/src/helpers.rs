use anyhow::{bail, Result};
use estate_schema::{check_policy_file, load_and_install_sacred_file};
use std::path::Path;

pub(crate) fn install_sacred(path: &Path) -> Result<()> {
    match load_and_install_sacred_file(path) {
        Ok(_) => Ok(()),
        Err(err) => bail!("{err}"),
    }
}

pub(crate) fn snapshot_state_files(state_dir: &Path) -> Vec<String> {
    let names = [
        "placement-actual.json",
        "actual-state.json",
        "desired-snapshot.yaml",
        "lifecycle.json",
        "apply-audit.jsonl",
        "reconcile.json",
        "catalog.json",
        "model-actual.json",
    ];
    names
        .into_iter()
        .filter_map(|n| {
            let p = state_dir.join(n);
            if p.is_file() {
                Some(format!("{}:{}", n, p.metadata().map(|m| m.len()).unwrap_or(0)))
            } else {
                None
            }
        })
        .collect()
}

pub(crate) fn enforce_policy(path: &Path, action: &str, hop: Option<&str>) -> Result<()> {
    match check_policy_file(path, action, hop) {
        Ok(_) => Ok(()),
        Err(err) => bail!("{err}"),
    }
}

pub(crate) fn chrono_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

pub(crate) fn copy_if_exists(src: &Path, dest: &Path, copied: &mut Vec<String>) -> Result<bool> {
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

pub(crate) fn copy_tree_files(src: &Path, dest: &Path, copied: &mut Vec<String>) -> Result<()> {
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
            copy_tree_files(&path, &dest.join(path.file_name().unwrap_or_default()), copied)?;
        }
    }
    Ok(())
}
