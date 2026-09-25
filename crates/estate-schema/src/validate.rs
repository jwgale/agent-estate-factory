use crate::sacred::{is_sacred_name, locked_sacred_ids, normalize_name, LOCKED_SACRED};
use crate::types::{is_host_class, Effect, Estate, ModelClass, PlacementKind};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path};

#[derive(Debug, Clone)]
pub struct ValidateOpts {
    pub cell_one: bool,
}

impl Default for ValidateOpts {
    fn default() -> Self {
        Self { cell_one: true }
    }
}

pub fn validate(estate: &Estate) -> Result<(), Vec<String>> {
    validate_with(estate, ValidateOpts::default())
}

pub fn validate_with(estate: &Estate, opts: ValidateOpts) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if estate.version != 0 {
        errors.push(format!(
            "unknown estate version {} (Cell One understands v0 only; upgrade estate-control or keep version: 0)",
            estate.version
        ));
    }
    match estate
        .api_version
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => {}
        Some("cell-one.estate.v0") | Some("v0") => {}
        Some(other) => {
            errors.push(format!(
                "unknown apiVersion '{other}' (Cell One understands cell-one.estate.v0 / v0; upgrade estate-control or set apiVersion: cell-one.estate.v0)"
            ));
        }
    }
    match estate
        .kind
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => {}
        Some(k) if k.eq_ignore_ascii_case("agent-estate") => {}
        Some(other) => {
            errors.push(format!(
                "unknown kind '{other}' (Cell One understands kind: agent-estate; upgrade estate-control or set kind: agent-estate)"
            ));
        }
    }
    if estate.name.trim().is_empty() {
        errors.push("estate name must not be empty".into());
    }
    if !matches!(estate.default_effect, Effect::Deny) {
        errors.push("default_effect must be deny (deny-default is locked for Cell One)".into());
    }

    let mut agent_ids = HashSet::new();
    let mut desktops = HashSet::new();
    for agent in &estate.agents {
        check_slug("agent.id", &agent.id, &mut errors);
        if !agent_ids.insert(normalize_name(&agent.id)) {
            errors.push(format!("duplicate agent id '{}'", agent.id));
        }
        if agent.display_name.trim().is_empty() {
            errors.push(format!("agent '{}' display_name must not be empty", agent.id));
        }
        check_slug("agent.lane", &agent.lane, &mut errors);
        if agent.desktop.trim().is_empty() {
            errors.push(format!("agent '{}' desktop must not be empty", agent.id));
        } else if !desktops.insert(normalize_name(&agent.desktop)) {
            errors.push(format!(
                "duplicate desktop '{}' (each agent needs its own session)",
                agent.desktop
            ));
        }
        if is_sacred_name(&agent.id) || is_sacred_name(&agent.display_name) {
            errors.push(format!(
                "sacred exclusion '{}' cannot be declared as an agent",
                agent.id
            ));
        }
        check_unique_slugs(
            &format!("agent '{}' tools", agent.id),
            agent.tools.iter().map(|t| t.id.as_str()),
            &mut errors,
        );
        check_unique_slugs(
            &format!("agent '{}' mounts", agent.id),
            agent.mounts.iter().map(|m| m.id.as_str()),
            &mut errors,
        );
        check_unique_slugs(
            &format!("agent '{}' mcp", agent.id),
            agent.mcp.iter().map(|m| m.id.as_str()),
            &mut errors,
        );
        check_unique_slugs(
            &format!("agent '{}' models", agent.id),
            agent.models.iter().map(|m| m.id.as_str()),
            &mut errors,
        );
        for model in &agent.models {
            reject_sku(&format!("agent '{}' model", agent.id), &model.id, &mut errors);
            if estate
                .model_bindings
                .iter()
                .all(|b| normalize_name(&b.id) != normalize_name(&model.id))
            {
                errors.push(format!(
                    "agent '{}' model '{}' is not a model_binding",
                    agent.id, model.id
                ));
            }
        }
        for mount in &agent.mounts {
            if mount.path.trim().is_empty() {
                errors.push(format!(
                    "agent '{}' mount '{}' path must not be empty",
                    agent.id, mount.id
                ));
            }
            if path_escapes(&mount.path) {
                errors.push(format!(
                    "agent '{}' mount '{}' path must be a relative path without '..'",
                    agent.id, mount.id
                ));
            }
        }
    }

    let mut lane_ids = HashSet::new();
    let mut roots = HashSet::new();
    let mut owners = HashSet::new();
    for lane in &estate.lanes {
        check_slug("lane.id", &lane.id, &mut errors);
        if !lane_ids.insert(normalize_name(&lane.id)) {
            errors.push(format!("duplicate lane id '{}'", lane.id));
        }
        if !roots.insert(normalize_name(&lane.root_path)) {
            errors.push(format!("duplicate lane root_path '{}'", lane.root_path));
        }
        if path_escapes(&lane.root_path) || Path::new(&lane.root_path).is_absolute() {
            errors.push(format!(
                "lane '{}' root_path must be a relative path without '..'",
                lane.id
            ));
        }
        check_slug("lane.owner_agent_id", &lane.owner_agent_id, &mut errors);
        if !owners.insert(normalize_name(&lane.owner_agent_id)) {
            errors.push(format!(
                "lane owner '{}' owns more than one lane (Cell One is 1:1)",
                lane.owner_agent_id
            ));
        }
        if is_sacred_name(&lane.id) {
            errors.push(format!("sacred exclusion '{}' cannot be a lane", lane.id));
        }
        if estate.agent(&lane.owner_agent_id).is_none() {
            errors.push(format!(
                "lane '{}' owner_agent_id '{}' is not an agent",
                lane.id, lane.owner_agent_id
            ));
        }
    }

    let mut agent_lanes = HashSet::new();
    for agent in &estate.agents {
        if estate.lane(&agent.lane).is_none() {
            errors.push(format!(
                "agent '{}' lane '{}' does not exist",
                agent.id, agent.lane
            ));
        } else if !agent_lanes.insert(normalize_name(&agent.lane)) {
            errors.push(format!(
                "agent '{}' shares lane '{}' (A1 requires separate lanes)",
                agent.id, agent.lane
            ));
        }
        if let Some(lane) = estate.lane(&agent.lane) {
            if normalize_name(&lane.owner_agent_id) != normalize_name(&agent.id) {
                errors.push(format!(
                    "agent '{}' must own its lane '{}' (owner is '{}')",
                    agent.id, agent.lane, lane.owner_agent_id
                ));
            }
        }
    }

    errors.extend(crate::compile::intention_object_errors(estate));

    let mut binding_ids = HashSet::new();
    let mut classes = HashSet::new();
    for binding in &estate.model_bindings {
        check_slug("model_binding.id", &binding.id, &mut errors);
        if !binding_ids.insert(normalize_name(&binding.id)) {
            errors.push(format!("duplicate model_binding id '{}'", binding.id));
        }
        if binding.driver.trim().is_empty() {
            errors.push(format!("model_binding '{}' driver must not be empty", binding.id));
        }
        reject_sku("model_binding.id", &binding.id, &mut errors);
        reject_sku("model_binding.driver", &binding.driver, &mut errors);
        reject_sku_in_params(&binding.id, &binding.params, &mut errors);
        if let Some(hc) = binding.params.get("host_class").and_then(|v| v.as_str()) {
            if !is_host_class(hc) {
                errors.push(format!(
                    "model_binding '{}' host_class '{}' must be consumer-nvidia|apple-silicon|rented-nvidia|any (aliases: rtx-consumer, nvidia-rental)",
                    binding.id, hc
                ));
            }
        }
        classes.insert(binding.class);
    }

    let mut placement_ids = HashSet::new();
    for placement in &estate.placements {
        check_slug("placement.id", &placement.id, &mut errors);
        if !placement_ids.insert(normalize_name(&placement.id)) {
            errors.push(format!("duplicate placement id '{}'", placement.id));
        }
        reject_sku("placement.id", &placement.id, &mut errors);
        if let Some(hc) = &placement.host_class {
            if !is_host_class(hc) {
                errors.push(format!(
                    "placement '{}' host_class '{}' must be consumer-nvidia|apple-silicon|rented-nvidia|any (aliases: rtx-consumer, nvidia-rental)",
                    placement.id, hc
                ));
            }
        }
        for agent_id in &placement.agents {
            if estate.agent(agent_id).is_none() {
                errors.push(format!(
                    "placement '{}' agent '{}' is not an agent",
                    placement.id, agent_id
                ));
            }
        }
        if placement.kind == PlacementKind::CloudAgent {
            for agent_id in &placement.agents {
                if is_sacred_name(agent_id) || estate.is_sacred(agent_id) {
                    errors.push(format!(
                        "placement '{}' cannot assign sacred exclusion '{}' to a cloud-agent",
                        placement.id, agent_id
                    ));
                }
            }
            // Declared stub is valid even when wired:true. Floor still does not spawn it.
            if placement.agents.is_empty() && placement.wired {
                errors.push(format!(
                    "placement '{}' is a wired cloud-agent with no agents; leave wired:false until Day-90 assigns it",
                    placement.id
                ));
            }
        }
    }

    if estate.enrich_packs.policy.trim().to_ascii_lowercase() != "manual" {
        errors.push(
            "enrich_packs.policy must be manual (Jason curates first specialist packs; no auto-promote)"
                .into(),
        );
    }
    if opts.cell_one && estate.enrich_packs.curator.trim().to_ascii_lowercase() != "jason" {
        errors.push("Cell One enrich_packs.curator must be jason".into());
    }
    for pack in &estate.enrich_packs.packs {
        check_slug("enrich_packs.pack.id", &pack.id, &mut errors);
        reject_sku("enrich_packs.pack.id", &pack.id, &mut errors);
    }

    if opts.cell_one {
        if estate.agents.len() < 3 {
            errors.push(format!(
                "Cell One requires at least 3 agents (found {})",
                estate.agents.len()
            ));
        }
        if estate.lanes.len() < 3 {
            errors.push(format!(
                "Cell One requires at least 3 lanes (found {})",
                estate.lanes.len()
            ));
        }
        if !classes.contains(&ModelClass::Frontier) || !classes.contains(&ModelClass::Local) {
            errors.push(
                "Cell One requires equal-class model_bindings: at least one frontier and one local"
                    .into(),
            );
        }
        let used: HashSet<String> = estate
            .agents
            .iter()
            .flat_map(|a| a.models.iter().map(|m| normalize_name(&m.id)))
            .collect();
        let uses_frontier = estate.model_bindings.iter().any(|b| {
            b.class == ModelClass::Frontier && used.contains(&normalize_name(&b.id))
        });
        let uses_local = estate
            .model_bindings
            .iter()
            .any(|b| b.class == ModelClass::Local && used.contains(&normalize_name(&b.id)));
        if !used.is_empty() && (!uses_frontier || !uses_local) {
            errors.push(
                "A9: estate must assign at least one frontier and one local binding to agents"
                    .into(),
            );
        }
        let declared: HashSet<String> = estate
            .sacred_exclusions
            .iter()
            .flat_map(|ex| {
                std::iter::once(normalize_name(&ex.id))
                    .chain(ex.aliases.iter().map(|a| normalize_name(a)))
            })
            .collect();
        for id in locked_sacred_ids() {
            let aliases = LOCKED_SACRED
                .iter()
                .find(|(locked, _)| *locked == id)
                .map(|(_, a)| *a)
                .unwrap_or(&[]);
            let present = declared.contains(&normalize_name(id))
                || aliases.iter().any(|a| declared.contains(&normalize_name(a)));
            if !present {
                errors.push(format!(
                    "sacred exclusion '{id}' must be declared on the estate (locked default)"
                ));
            }
        }
        let names: HashMap<String, &str> = estate
            .agents
            .iter()
            .map(|a| (normalize_name(&a.id), a.display_name.as_str()))
            .collect();
        if names.contains_key("sanctum") {
            if estate.agents.iter().any(|a| {
                normalize_name(&a.id) == "sanctum" && is_sacred_name(&a.display_name)
            }) {
                errors.push("Sanctum must not be Cyera".into());
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_slug(field: &str, value: &str, errors: &mut Vec<String>) {
    if !is_slug(value) {
        errors.push(format!(
            "{field} '{value}' must match [a-z][a-z0-9_-]{{0,63}}"
        ));
    }
}

fn check_unique_slugs<'a>(
    label: &str,
    ids: impl Iterator<Item = &'a str>,
    errors: &mut Vec<String>,
) {
    let mut seen = HashSet::new();
    for id in ids {
        check_slug(label, id, errors);
        if !seen.insert(normalize_name(id)) {
            errors.push(format!("{label} has duplicate id '{id}'"));
        }
    }
}

/// Hardware SKUs must not appear in estate contracts. Hardware is a driver choice.
pub const SKU_NEEDLES: &[&str] = &[
    "5090", "4090", "4080", "3090", "a100", "h100", "b200", "m3-max", "m3max", "m2-max", "m2max",
    "m1-max", "m1max", "m4-max", "m4max",
];

pub fn contains_sku(value: &str) -> bool {
    let n = value.to_ascii_lowercase();
    SKU_NEEDLES.iter().any(|needle| n.contains(needle))
}

pub fn is_slug(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    if value.len() > 64 {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn reject_sku(field: &str, value: &str, errors: &mut Vec<String>) {
    if contains_sku(value) {
        errors.push(format!(
            "{field} '{value}' encodes a hardware SKU; hardware is a driver choice (use a portable id such as local_slm and driver ollama|llama.cpp|mlx)"
        ));
    }
}

fn reject_sku_in_params(binding_id: &str, params: &serde_json::Value, errors: &mut Vec<String>) {
    let Some(map) = params.as_object() else {
        return;
    };
    for (key, value) in map {
        if let Some(s) = value.as_str() {
            if contains_sku(s) {
                errors.push(format!(
                    "model_binding '{binding_id}' params.{key} encodes a hardware SKU; use host_class consumer-nvidia|apple-silicon|rented-nvidia|any"
                ));
            }
        }
    }
}

fn path_escapes(path: &str) -> bool {
    Path::new(path)
        .components()
        .any(|c| matches!(c, Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{load_estate_str, parse_estate_yaml};

    fn load_invalid(name: &str) -> Estate {
        let path = format!("../../examples/invalid/{name}");
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path),
        )
        .unwrap_or_else(|e| panic!("fixture {name}: {e}"));
        parse_estate_yaml(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
    }

    #[test]
    fn example_ok() {
        validate(&load_estate_str(crate::tests::example_yaml()).unwrap()).unwrap();
    }

    #[test]
    fn too_few_agents_fails() {
        let err = validate(&load_invalid("too-few-agents.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("at least 3 agents")));
    }

    #[test]
    fn shared_lane_fails() {
        let err = validate(&load_invalid("shared-lane.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("shares lane") || e.contains("more than one lane")));
    }

    #[test]
    fn sacred_as_agent_fails() {
        let err = validate(&load_invalid("sacred-as-agent.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("sacred exclusion")));
    }

    #[test]
    fn wired_true_is_allowed() {
        let estate = load_estate_str(crate::tests::example_yaml()).unwrap();
        assert!(estate.model_bindings.iter().all(|b| b.wired));
        validate(&estate).unwrap();
    }

    #[test]
    fn frontier_only_fails_equal_class() {
        let err = validate(&load_invalid("frontier-only.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("equal-class") || e.contains("local")));
    }

    #[test]
    fn missing_sacred_fails() {
        let err = validate(&load_invalid("missing-sacred.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("cyera-ci") || e.contains("rust-classroom")));
    }

    #[test]
    fn allow_sacred_intention_fails() {
        let err = validate(&load_invalid("allow-sacred-intention.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("cannot allow sacred")));
    }

    #[test]
    fn default_allow_fails() {
        let err = validate(&load_invalid("default-allow.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("default_effect")));
    }

    #[test]
    fn sku_in_binding_id_fails() {
        let err = validate(&load_invalid("sku-binding.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("hardware SKU") || e.contains("5090")));
    }

    #[test]
    fn enrich_packs_must_stay_manual() {
        let mut estate = load_estate_str(crate::tests::example_yaml()).unwrap();
        estate.enrich_packs.policy = "auto".into();
        let err = validate(&estate).unwrap_err();
        assert!(err.iter().any(|e| e.contains("manual")));
    }

    #[test]
    fn placement_sku_fails() {
        let err = validate(&load_invalid("placement-sku.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("hardware SKU")));
    }

    #[test]
    fn placement_unknown_agent_fails() {
        let err = validate(&load_invalid("placement-unknown-agent.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("not an agent")));
    }

    #[test]
    fn example_declares_cloud_agent_stub() {
        let estate = load_estate_str(crate::tests::example_yaml()).unwrap();
        assert!(estate
            .placements
            .iter()
            .any(|p| p.kind == PlacementKind::CloudAgent && !p.wired));
        validate(&estate).unwrap();
    }

    #[test]
    fn cloud_placement_refuses_sacred_agent() {
        let err = validate(&load_invalid("placement-sacred-cloud.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("sacred")));
    }

    #[test]
    fn unknown_api_version_fails_closed_with_upgrade_hint() {
        let err = validate(&load_invalid("api-version-unknown.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("unknown apiVersion")));
        assert!(err.iter().any(|e| e.contains("upgrade estate-control")));
        assert!(err.iter().any(|e| e.contains("cell-one.estate.v0")));
    }

    #[test]
    fn unknown_kind_fails_closed_with_upgrade_hint() {
        let err = validate(&load_invalid("kind-unknown.yaml")).unwrap_err();
        assert!(err.iter().any(|e| e.contains("unknown kind")));
        assert!(err.iter().any(|e| e.contains("agent-estate")));
    }

    #[test]
    fn known_api_version_is_ok() {
        let mut estate = load_estate_str(crate::tests::example_yaml()).unwrap();
        estate.api_version = Some("cell-one.estate.v0".into());
        validate(&estate).unwrap();
        estate.api_version = Some("v0".into());
        validate(&estate).unwrap();
    }
}
