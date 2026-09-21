/// Locked Cell One exclusions. Also declared on the example estate so the
/// desired-state file shows them; the validator hard-denies these names either way.
///
/// Dual-layer: hardcoded defaults always apply. `policy/sacred.yaml` overlays
/// are additive. A file cannot remove a locked id by omitting it.
pub const LOCKED_SACRED: &[(&str, &[&str])] = &[
    ("cyera-ci", &["cyera", "cyera_ci"]),
    ("rust-classroom", &["rust_classroom"]),
];

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::path::Path;

thread_local! {
    static OVERLAYS: RefCell<Vec<(String, Vec<String>)>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SacredFile {
    #[serde(default)]
    pub schema: String,
    #[serde(default)]
    pub locked: Vec<SacredFileEntry>,
    #[serde(default)]
    pub overlays: Vec<SacredFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SacredFileEntry {
    pub id: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

pub fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace('_', "-")
}

pub fn locked_sacred_ids() -> Vec<&'static str> {
    LOCKED_SACRED.iter().map(|(id, _)| *id).collect()
}

fn strip_sacred_prefix(name: &str) -> &str {
    name.trim()
        .strip_prefix("lane:")
        .or_else(|| name.trim().strip_prefix("exclusion:"))
        .or_else(|| name.trim().strip_prefix("tool:"))
        .or_else(|| name.trim().strip_prefix("mcp:"))
        .or_else(|| name.trim().strip_prefix("mount:"))
        .unwrap_or(name.trim())
}

fn matches_entry(n: &str, id: &str, aliases: &[String]) -> bool {
    normalize_name(id) == n || aliases.iter().any(|a| normalize_name(a) == n)
}

fn is_locked_sacred(n: &str) -> bool {
    LOCKED_SACRED.iter().any(|(id, aliases)| {
        normalize_name(id) == n || aliases.iter().any(|a| normalize_name(a) == n)
    })
}

pub fn is_sacred_name(name: &str) -> bool {
    let n = normalize_name(strip_sacred_prefix(name));
    if is_locked_sacred(&n) {
        return true;
    }
    OVERLAYS.with(|cell| {
        cell.borrow()
            .iter()
            .any(|(id, aliases)| matches_entry(&n, id, aliases))
    })
}

pub fn clear_sacred_overlays() {
    OVERLAYS.with(|cell| cell.borrow_mut().clear());
}

pub fn set_sacred_overlays(entries: &[SacredFileEntry]) {
    OVERLAYS.with(|cell| {
        *cell.borrow_mut() = entries
            .iter()
            .map(|e| (e.id.clone(), e.aliases.clone()))
            .collect();
    });
}

pub fn overlay_sacred_ids() -> Vec<String> {
    OVERLAYS.with(|cell| cell.borrow().iter().map(|(id, _)| id.clone()).collect())
}

pub fn parse_sacred_yaml(text: &str) -> Result<SacredFile, String> {
    let file: SacredFile =
        serde_yaml::from_str(text).map_err(|e| format!("refuse:sacred-file: parse: {e}"))?;
    if !file.schema.is_empty() && file.schema != "cell-one.sacred.v0" {
        return Err(format!(
            "refuse:sacred-schema: unknown schema '{}' (want cell-one.sacred.v0)",
            file.schema
        ));
    }
    Ok(file)
}

pub fn load_sacred_file(path: &Path) -> Result<SacredFile, String> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        format!(
            "refuse:sacred-file: read {}: {e}",
            path.display()
        )
    })?;
    parse_sacred_yaml(&text)
}

/// Install overlays from `path`. Missing file is ok (hardcoded defaults only).
/// Returns true when a file was loaded.
pub fn load_and_install_sacred_file(path: &Path) -> Result<bool, String> {
    if !path.is_file() {
        clear_sacred_overlays();
        return Ok(false);
    }
    let file = load_sacred_file(path)?;
    set_sacred_overlays(&file.overlays);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_names_match_aliases() {
        clear_sacred_overlays();
        assert!(is_sacred_name("cyera-ci"));
        assert!(is_sacred_name("Cyera"));
        assert!(is_sacred_name("exclusion:cyera_ci"));
        assert!(is_sacred_name("rust-classroom"));
        assert!(is_sacred_name("lane:rust_classroom"));
        assert!(!is_sacred_name("sanctum"));
        assert!(!is_sacred_name("horizon"));
        assert!(!is_sacred_name("lab-notebook"));
    }

    #[test]
    fn overlay_is_additive_and_cannot_drop_locked() {
        clear_sacred_overlays();
        set_sacred_overlays(&[SacredFileEntry {
            id: "lab-notebook".into(),
            aliases: vec!["lab_notebook".into()],
            reason: Some("fixture overlay".into()),
        }]);
        assert!(is_sacred_name("lab-notebook"));
        assert!(is_sacred_name("lane:lab_notebook"));
        assert!(is_sacred_name("cyera-ci"));
        assert!(!is_sacred_name("sanctum"));
        clear_sacred_overlays();
        assert!(!is_sacred_name("lab-notebook"));
    }

    #[test]
    fn unknown_schema_fail_closes() {
        let err = parse_sacred_yaml("schema: cell-one.sacred.v1\noverlays: []").unwrap_err();
        assert!(err.contains("refuse:sacred-schema"));
    }

    #[test]
    fn overlay_never_drops_locked_ids() {
        let overlays = [
            vec![],
            vec![SacredFileEntry {
                id: "lab-notebook".into(),
                aliases: vec!["lab_notebook".into()],
                reason: Some("additive".into()),
            }],
            vec![SacredFileEntry {
                id: "cyera-ci".into(),
                aliases: vec![],
                reason: Some("naming a locked id must not drop it".into()),
            }],
            vec![SacredFileEntry {
                id: "notes".into(),
                aliases: vec!["cyera".into(), "rust_classroom".into()],
                reason: Some("alias collision stays additive".into()),
            }],
        ];
        let omit = parse_sacred_yaml(
            "schema: cell-one.sacred.v0\nlocked: []\noverlays:\n  - id: lab-notebook\n    aliases: [lab_notebook]\n",
        )
        .unwrap();
        assert!(omit.locked.is_empty());
        for extra in overlays {
            clear_sacred_overlays();
            set_sacred_overlays(&extra);
            for (id, aliases) in LOCKED_SACRED {
                assert!(is_sacred_name(id), "locked id {id} dropped");
                assert!(is_sacred_name(&format!("lane:{id}")), "lane:{id} dropped");
                for alias in *aliases {
                    assert!(is_sacred_name(alias), "alias {alias} dropped");
                    assert!(
                        is_sacred_name(&alias.to_ascii_uppercase()),
                        "alias {alias} case dropped"
                    );
                }
            }
            assert!(!is_sacred_name("sanctum"));
            assert!(!is_sacred_name("horizon"));
        }
        clear_sacred_overlays();
        set_sacred_overlays(&omit.overlays);
        assert!(is_sacred_name("cyera-ci"));
        assert!(is_sacred_name("rust-classroom"));
        assert!(is_sacred_name("lab-notebook"));
        clear_sacred_overlays();
    }
}
