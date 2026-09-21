//! Thin desired-state types, fail-closed validation, compiled intentions,
//! plan diff, and the memory/tool firewall used by the data plane.

mod compile;
mod error;
mod firewall;
mod hash;
mod plan;
mod sacred;
mod types;
mod validate;

pub use compile::{compile_intentions, CompiledIntention};
pub use error::EstateError;
pub use firewall::{authorize, read_lane_file, AccessRequest, Decision, Deny};
pub use hash::estate_hash;
pub use plan::{diff_estates, render_plan, write_plan, EstatePlan, PlanDelta};
pub use sacred::{is_sacred_name, locked_sacred_ids, normalize_name, LOCKED_SACRED};
pub use types::{
    Agent, Effect, EnrichPack, EnrichPacks, Estate, Intention, IntentionKind, Lane, McpDecl,
    ModelBinding, ModelClass, ModelUseDecl, MountDecl, ObjectRef, SacredExclusion, ToolDecl,
};
pub use validate::{validate, ValidateOpts};

use std::path::Path;

/// Parse YAML without running the validator (used so CLI can print every error).
pub fn parse_estate_yaml(text: &str) -> Result<Estate, EstateError> {
    serde_yaml::from_str(text).map_err(|e| EstateError::Parse(e.to_string()))
}

/// Parse and fail-closed validate.
pub fn load_estate_str(text: &str) -> Result<Estate, EstateError> {
    let estate = parse_estate_yaml(text)?;
    validate(&estate).map_err(EstateError::Invalid)?;
    Ok(estate)
}

pub fn load_estate(path: &Path) -> Result<Estate, EstateError> {
    let text = std::fs::read_to_string(path).map_err(|e| EstateError::Io {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    load_estate_str(&text)
}

pub fn load_estate_unvalidated(path: &Path) -> Result<Estate, EstateError> {
    let text = std::fs::read_to_string(path).map_err(|e| EstateError::Io {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    parse_estate_yaml(&text)
}

pub fn describe(estate: &Estate) -> String {
    let agents = estate
        .agents
        .iter()
        .map(|a| a.id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let bindings = estate
        .model_bindings
        .iter()
        .map(|b| {
            format!(
                "{} {} driver={} wired={}",
                b.class.as_str(),
                b.id,
                b.driver,
                b.wired
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let sacred = estate
        .sacred_exclusions
        .iter()
        .map(|s| s.id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "name: {}\nhash: {}\nagents: {} ({})\nlanes: {}\nintentions: {} (default_effect={})\nmodel_bindings: {} ({})\nsacred_exclusions: {}\nenrich_packs: {} / {} ({} packs)",
        estate.name,
        estate_hash(estate),
        estate.agents.len(),
        agents,
        estate.lanes.len(),
        estate.intentions.len(),
        estate.default_effect.as_str(),
        estate.model_bindings.len(),
        bindings,
        sacred,
        estate.enrich_packs.curator,
        estate.enrich_packs.policy,
        estate.enrich_packs.packs.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn example_yaml() -> &'static str {
        include_str!("../../../examples/estate.yaml")
    }

    #[test]
    fn example_estate_loads() {
        let estate = load_estate_str(example_yaml()).expect("example must be valid");
        assert_eq!(estate.agents.len(), 3);
        assert_eq!(estate.lanes.len(), 3);
        assert!(estate.intentions.is_empty());
        assert!(estate.model_bindings.iter().all(|b| b.wired));
        assert!(estate.agent("horizon").unwrap().has_model("xai_grok"));
        assert!(estate.agent("horizon").unwrap().has_model("local_slm"));
        assert_eq!(estate.enrich_packs.curator, "jason");
        assert_eq!(estate.enrich_packs.policy, "manual");
    }
}
