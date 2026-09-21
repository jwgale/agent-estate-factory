use anyhow::{bail, Context, Result};
use estate_schema::{check_policy_file, load_and_install_sacred_file, load_estate, Estate};
use std::path::Path;

/// Present estate file must read. Empty-on-error would hide a rewrite
/// (before == after == "").
pub(crate) fn read_estate_text(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
}

/// Missing estate file is optional. A file that exists but does not parse
/// is refuse — backup/restore must not invent a locked-only sacred set.
pub(crate) fn load_estate_if_present(path: &Path) -> Result<Option<Estate>> {
    if !path.is_file() {
        return Ok(None);
    }
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    Ok(Some(estate))
}

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
        "lifecycle.jsonl",
        "apply-audit.jsonl",
        "sessions.jsonl",
        "reconcile.json",
        "reconcile.md",
        "catalog.json",
        "model-actual.json",
        "conveyor-mesh.json",
        "conveyor-hops.json",
        "conveyor-leases.json",
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

#[cfg(test)]
mod tests {
    use super::read_estate_text;
    use std::io::Write;

    #[test]
    fn read_estate_text_refuses_missing_instead_of_empty() {
        let path = std::env::temp_dir().join(format!(
            "cell-one-missing-estate-{}.yaml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let err = read_estate_text(&path).unwrap_err();
        assert!(
            !err.to_string().is_empty(),
            "missing estate must not look like empty==empty"
        );
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "name: keep").unwrap();
        assert_eq!(read_estate_text(&path).unwrap().trim(), "name: keep");
        let _ = std::fs::remove_file(&path);
    }
}
