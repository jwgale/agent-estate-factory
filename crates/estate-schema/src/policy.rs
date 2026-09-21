//! Declarative deny/allow policy pack. Stub, fail-closed.
//!
//! File SoT: `policy/cell-one.policy.v0.yaml`. Unknown actions refuse.
//! Apply and convey-call consult this before they write or hop.

use crate::types::Effect;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const POLICY_SCHEMA: &str = "cell-one.policy.v0";
pub const POLICY_KIND: &str = "estate-policy";
pub const KNOWN_POLICY_ACTIONS: &[&str] = &["apply", "convey-call", "backup", "restore"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyRule {
    pub action: String,
    #[serde(default)]
    pub effect: Effect,
    #[serde(default)]
    pub hop: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyPack {
    #[serde(
        default,
        rename = "apiVersion",
        alias = "api_version",
        skip_serializing_if = "Option::is_none"
    )]
    pub api_version: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default = "default_policy_schema")]
    pub schema: String,
    #[serde(default, rename = "default")]
    pub default_effect: Effect,
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
    #[serde(default)]
    pub note: Option<String>,
}

fn default_policy_schema() -> String {
    POLICY_SCHEMA.to_string()
}

impl Default for PolicyPack {
    fn default() -> Self {
        Self {
            api_version: Some(POLICY_SCHEMA.into()),
            kind: Some(POLICY_KIND.into()),
            schema: POLICY_SCHEMA.into(),
            default_effect: Effect::Deny,
            rules: KNOWN_POLICY_ACTIONS
                .iter()
                .map(|action| PolicyRule {
                    action: (*action).into(),
                    effect: Effect::Allow,
                    hop: None,
                    note: Some("default allow for known factory actions".into()),
                })
                .collect(),
            note: Some("Jason-curated. Fail closed on unknown actions. Not a gateway.".into()),
        }
    }
}

pub fn is_known_policy_action(action: &str) -> bool {
    let n = action.trim().to_ascii_lowercase();
    KNOWN_POLICY_ACTIONS.iter().any(|a| *a == n)
}

pub fn refuse_policy(pack: &PolicyPack) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    match pack
        .api_version
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None | Some(POLICY_SCHEMA) | Some("v0") => {}
        Some(other) => errors.push(format!(
            "unknown apiVersion '{other}' (Cell One understands {POLICY_SCHEMA} / v0; upgrade estate-control)"
        )),
    }
    match pack
        .kind
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => {}
        Some(k) if k.eq_ignore_ascii_case(POLICY_KIND) => {}
        Some(other) => errors.push(format!(
            "unknown kind '{other}' (Cell One understands kind: {POLICY_KIND})"
        )),
    }
    if pack.schema != POLICY_SCHEMA && !pack.schema.is_empty() {
        errors.push(format!(
            "unknown policy schema '{}' (understands {POLICY_SCHEMA})",
            pack.schema
        ));
    }
    for rule in &pack.rules {
        if !is_known_policy_action(&rule.action) {
            errors.push(format!(
                "refuse:unknown-action: '{}' is not a known policy action ({})",
                rule.action,
                KNOWN_POLICY_ACTIONS.join(", ")
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn parse_policy_yaml(text: &str) -> Result<PolicyPack, String> {
    let pack: PolicyPack =
        serde_yaml::from_str(text).map_err(|e| format!("policy parse: {e}"))?;
    refuse_policy(&pack).map_err(|errs| errs.join("\n"))?;
    Ok(pack)
}

pub fn load_policy(path: &Path) -> Result<PolicyPack, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("policy {}: {e}", path.display()))?;
    parse_policy_yaml(&text)
}

/// Missing file is Ok(None) so relative defaults do not break crate-cwd tests.
pub fn load_policy_optional(path: &Path) -> Result<Option<PolicyPack>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    load_policy(path).map(Some)
}

/// Fail-closed: unknown action, or no matching allow when default is deny.
pub fn policy_allows(pack: &PolicyPack, action: &str, hop: Option<&str>) -> Result<(), String> {
    if !is_known_policy_action(action) {
        return Err(format!(
            "refuse:unknown-action: '{action}' is not a known policy action ({})",
            KNOWN_POLICY_ACTIONS.join(", ")
        ));
    }
    let action_n = action.trim().to_ascii_lowercase();
    let mut matched = false;
    let mut denied = false;
    for rule in &pack.rules {
        if rule.action.trim().to_ascii_lowercase() != action_n {
            continue;
        }
        if let (Some(want), Some(have)) = (rule.hop.as_deref(), hop) {
            if want != have {
                continue;
            }
        }
        matched = true;
        match rule.effect {
            Effect::Allow => return Ok(()),
            Effect::Deny => denied = true,
        }
    }
    if denied {
        return Err(format!("refuse:policy: action '{action}' is denied"));
    }
    if matched {
        return Ok(());
    }
    match pack.default_effect {
        Effect::Allow => Ok(()),
        Effect::Deny => Err(format!(
            "refuse:policy: action '{action}' is not allowed (default deny)"
        )),
    }
}

pub fn check_policy_file(
    path: &Path,
    action: &str,
    hop: Option<&str>,
) -> Result<Option<PolicyPack>, String> {
    let Some(pack) = load_policy_optional(path)? else {
        return Ok(None);
    };
    policy_allows(&pack, action, hop)?;
    Ok(Some(pack))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pack_allows_known_actions() {
        let pack = PolicyPack::default();
        refuse_policy(&pack).unwrap();
        policy_allows(&pack, "apply", None).unwrap();
        policy_allows(&pack, "convey-call", Some("cell-one-box")).unwrap();
        policy_allows(&pack, "backup", None).unwrap();
        policy_allows(&pack, "restore", None).unwrap();
    }

    #[test]
    fn unknown_action_fails_closed() {
        let err = policy_allows(&PolicyPack::default(), "spawn-cloud", None).unwrap_err();
        assert!(err.contains("refuse:unknown-action"));
        let bad = parse_policy_yaml(
            "apiVersion: cell-one.policy.v0\nkind: estate-policy\ndefault: deny\nrules:\n  - action: spawn-cloud\n    effect: allow\n",
        )
        .unwrap_err();
        assert!(bad.contains("refuse:unknown-action"));
    }

    #[test]
    fn default_deny_without_allow_refuses() {
        let pack = parse_policy_yaml(
            "apiVersion: cell-one.policy.v0\nkind: estate-policy\ndefault: deny\nrules: []\n",
        )
        .unwrap();
        let err = policy_allows(&pack, "apply", None).unwrap_err();
        assert!(err.contains("refuse:policy"));
    }

    #[test]
    fn hop_specific_deny() {
        let pack = parse_policy_yaml(
            "apiVersion: cell-one.policy.v0\nkind: estate-policy\ndefault: deny\nrules:\n  - action: convey-call\n    effect: allow\n    hop: cell-one-box\n  - action: convey-call\n    effect: deny\n    hop: cursor-cloud\n",
        )
        .unwrap();
        policy_allows(&pack, "convey-call", Some("cell-one-box")).unwrap();
        let err = policy_allows(&pack, "convey-call", Some("cursor-cloud")).unwrap_err();
        assert!(err.contains("denied") || err.contains("refuse:policy"));
    }
}
