/// Locked Cell One exclusions. Also declared on the example estate so the
/// desired-state file shows them; the validator hard-denies these names either way.
pub const LOCKED_SACRED: &[(&str, &[&str])] = &[
    ("cyera-ci", &["cyera", "cyera_ci"]),
    ("rust-classroom", &["rust_classroom"]),
];

pub fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace('_', "-")
}

pub fn locked_sacred_ids() -> Vec<&'static str> {
    LOCKED_SACRED.iter().map(|(id, _)| *id).collect()
}

pub fn is_sacred_name(name: &str) -> bool {
    let stripped = name
        .trim()
        .strip_prefix("lane:")
        .or_else(|| name.trim().strip_prefix("exclusion:"))
        .or_else(|| name.trim().strip_prefix("tool:"))
        .or_else(|| name.trim().strip_prefix("mcp:"))
        .or_else(|| name.trim().strip_prefix("mount:"))
        .unwrap_or(name.trim());
    let n = normalize_name(stripped);
    LOCKED_SACRED.iter().any(|(id, aliases)| {
        normalize_name(id) == n || aliases.iter().any(|a| normalize_name(a) == n)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_names_match_aliases() {
        assert!(is_sacred_name("cyera-ci"));
        assert!(is_sacred_name("Cyera"));
        assert!(is_sacred_name("exclusion:cyera_ci"));
        assert!(is_sacred_name("rust-classroom"));
        assert!(is_sacred_name("lane:rust_classroom"));
        assert!(!is_sacred_name("sanctum"));
        assert!(!is_sacred_name("horizon"));
    }
}
