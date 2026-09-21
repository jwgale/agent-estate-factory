use crate::hash::estate_hash;
use crate::types::Estate;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PlanDelta {
    pub agents: Vec<String>,
    pub lanes: Vec<String>,
    pub intentions: Vec<String>,
    pub model_bindings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EstatePlan {
    pub desired_hash: String,
    pub against_hash: Option<String>,
    pub added: PlanDelta,
    pub removed: PlanDelta,
    pub changed: PlanDelta,
    pub blast_radius_text: String,
    pub created_at: String,
}

pub fn diff_estates(desired: &Estate, against: Option<&Estate>) -> EstatePlan {
    let desired_hash = estate_hash(desired);
    let against_hash = against.map(estate_hash);
    let (added, removed, changed) = match against {
        None => (
            PlanDelta {
                agents: names(desired.agents.iter().map(|a| a.id.as_str())),
                lanes: names(desired.lanes.iter().map(|l| l.id.as_str())),
                intentions: intention_keys(desired),
                model_bindings: names(desired.model_bindings.iter().map(|b| b.id.as_str())),
            },
            PlanDelta::default(),
            PlanDelta::default(),
        ),
        Some(prev) => {
            let add_agents = set_diff(ids(desired.agents.iter().map(|a| a.id.as_str())), ids(prev.agents.iter().map(|a| a.id.as_str())));
            let rem_agents = set_diff(ids(prev.agents.iter().map(|a| a.id.as_str())), ids(desired.agents.iter().map(|a| a.id.as_str())));
            let add_lanes = set_diff(ids(desired.lanes.iter().map(|l| l.id.as_str())), ids(prev.lanes.iter().map(|l| l.id.as_str())));
            let rem_lanes = set_diff(ids(prev.lanes.iter().map(|l| l.id.as_str())), ids(desired.lanes.iter().map(|l| l.id.as_str())));
            let add_int = set_diff(intention_key_set(desired), intention_key_set(prev));
            let rem_int = set_diff(intention_key_set(prev), intention_key_set(desired));
            let add_bind = set_diff(ids(desired.model_bindings.iter().map(|b| b.id.as_str())), ids(prev.model_bindings.iter().map(|b| b.id.as_str())));
            let rem_bind = set_diff(ids(prev.model_bindings.iter().map(|b| b.id.as_str())), ids(desired.model_bindings.iter().map(|b| b.id.as_str())));
            let changed = PlanDelta {
                agents: changed_ids(
                    desired.agents.iter().map(|a| (a.id.as_str(), fingerprint(a))),
                    prev.agents.iter().map(|a| (a.id.as_str(), fingerprint(a))),
                ),
                lanes: changed_ids(
                    desired.lanes.iter().map(|l| (l.id.as_str(), fingerprint(l))),
                    prev.lanes.iter().map(|l| (l.id.as_str(), fingerprint(l))),
                ),
                intentions: vec![],
                model_bindings: changed_ids(
                    desired
                        .model_bindings
                        .iter()
                        .map(|b| (b.id.as_str(), fingerprint(b))),
                    prev.model_bindings
                        .iter()
                        .map(|b| (b.id.as_str(), fingerprint(b))),
                ),
            };
            (
                PlanDelta {
                    agents: add_agents,
                    lanes: add_lanes,
                    intentions: add_int,
                    model_bindings: add_bind,
                },
                PlanDelta {
                    agents: rem_agents,
                    lanes: rem_lanes,
                    intentions: rem_int,
                    model_bindings: rem_bind,
                },
                changed,
            )
        }
    };

    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let blast_radius_text = blast_radius(desired, &added, &removed, &changed, against.is_none());
    EstatePlan {
        desired_hash,
        against_hash,
        added,
        removed,
        changed,
        blast_radius_text,
        created_at,
    }
}

pub fn render_plan(plan: &EstatePlan) -> String {
    let mut out = String::new();
    out.push_str("Estate plan\n===========\n");
    out.push_str(&format!("desired hash: {}\n", plan.desired_hash));
    match &plan.against_hash {
        Some(h) => out.push_str(&format!("against hash: {h}\n")),
        None => out.push_str("against: (empty / greenfield)\n"),
    }
    out.push_str(&format!("created_at: {}\n\n", plan.created_at));
    out.push_str("Blast radius\n------------\n");
    out.push_str(&plan.blast_radius_text);
    out.push('\n');
    out.push_str("\nAdded\n");
    out.push_str(&render_delta(&plan.added));
    out.push_str("Removed\n");
    out.push_str(&render_delta(&plan.removed));
    out.push_str("Changed\n");
    out.push_str(&render_delta(&plan.changed));
    out
}

pub fn write_plan(plans_dir: &Path, plan: &EstatePlan) -> Result<std::path::PathBuf, std::io::Error> {
    std::fs::create_dir_all(plans_dir)?;
    let short = plan
        .desired_hash
        .trim_start_matches("sha256:")
        .chars()
        .take(8)
        .collect::<String>();
    let stamp = plan
        .created_at
        .replace(':', "")
        .replace('-', "");
    let stem = format!("plan-{stamp}-{short}");
    let md_path = plans_dir.join(format!("{stem}.md"));
    let json_path = plans_dir.join(format!("{stem}.json"));
    std::fs::write(&md_path, render_plan(plan))?;
    std::fs::write(&json_path, serde_json::to_string_pretty(plan).unwrap_or_default())?;
    Ok(md_path)
}

fn blast_radius(
    estate: &Estate,
    added: &PlanDelta,
    removed: &PlanDelta,
    changed: &PlanDelta,
    greenfield: bool,
) -> String {
    let mut lines = Vec::new();
    if greenfield {
        lines.push(format!(
            "Greenfield apply binds {} agents onto {} isolated lanes.",
            estate.agents.len(),
            estate.lanes.len()
        ));
    } else if added.is_empty() && removed.is_empty() && changed.is_empty() {
        lines.push("No desired-state drift: blast radius is empty.".into());
    } else {
        lines.push("Desired-state diff will rebind only the listed agents/lanes/intentions.".into());
    }
    if estate.intentions.is_empty() {
        lines.push("No cross-lane intentions: memory firewall stays deny-default.".into());
    } else {
        lines.push(format!(
            "{} compiled intention(s) will change who can cross a lane or tool.",
            estate.intentions.len()
        ));
    }
    let placeholders: Vec<String> = estate
        .model_bindings
        .iter()
        .map(|b| format!("{}={} wired={}", b.class.as_str(), b.id, b.wired))
        .collect();
    lines.push(format!(
        "Model bindings remain placeholders: {}.",
        placeholders.join(", ")
    ));
    lines.push(format!(
        "Sacred exclusions stay out of the estate: {}.",
        estate
            .sacred_exclusions
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    if !added.agents.is_empty() {
        lines.push(format!("Adding agents {} expands session count.", added.agents.join(", ")));
    }
    if !removed.agents.is_empty() {
        lines.push(format!(
            "Removing agents {} drops their sessions; lane roots stay on disk until deleted by hand.",
            removed.agents.join(", ")
        ));
    }
    lines.join("\n")
}

fn render_delta(delta: &PlanDelta) -> String {
    format!(
        "  agents: {}\n  lanes: {}\n  intentions: {}\n  model_bindings: {}\n",
        fmt_list(&delta.agents),
        fmt_list(&delta.lanes),
        fmt_list(&delta.intentions),
        fmt_list(&delta.model_bindings)
    )
}

fn fmt_list(items: &[String]) -> String {
    if items.is_empty() {
        "(none)".into()
    } else {
        items.join(", ")
    }
}

impl PlanDelta {
    fn is_empty(&self) -> bool {
        self.agents.is_empty()
            && self.lanes.is_empty()
            && self.intentions.is_empty()
            && self.model_bindings.is_empty()
    }
}

fn names<'a>(iter: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut v: Vec<String> = iter.map(|s| s.to_string()).collect();
    v.sort();
    v
}

fn ids<'a>(iter: impl Iterator<Item = &'a str>) -> BTreeSet<String> {
    iter.map(|s| s.to_string()).collect()
}

fn set_diff(have: BTreeSet<String>, against: BTreeSet<String>) -> Vec<String> {
    have.difference(&against).cloned().collect()
}

fn intention_keys(estate: &Estate) -> Vec<String> {
    let mut v = intention_key_set(estate).into_iter().collect::<Vec<_>>();
    v.sort();
    v
}

fn intention_key_set(estate: &Estate) -> BTreeSet<String> {
    estate
        .intentions
        .iter()
        .map(|i| format!("{}:{}:{}:{}", i.subject_agent, i.kind.as_str(), i.object, i.effect.as_str()))
        .collect()
}

fn changed_ids<'a>(
    desired: impl Iterator<Item = (&'a str, String)>,
    prev: impl Iterator<Item = (&'a str, String)>,
) -> Vec<String> {
    let prev: HashMap<String, String> = prev.map(|(k, v)| (k.to_string(), v)).collect();
    let mut out = Vec::new();
    for (id, fp) in desired {
        if let Some(old) = prev.get(id) {
            if old != &fp {
                out.push(id.to_string());
            }
        }
    }
    out.sort();
    out
}

fn fingerprint<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_estate_str;

    #[test]
    fn greenfield_plan_lists_three_agents() {
        let e = load_estate_str(crate::tests::example_yaml()).unwrap();
        let plan = diff_estates(&e, None);
        assert_eq!(plan.added.agents.len(), 3);
        assert!(plan.blast_radius_text.contains("3 agents"));
        assert!(plan.blast_radius_text.contains("deny-default"));
    }

    #[test]
    fn identical_diff_is_empty() {
        let e = load_estate_str(crate::tests::example_yaml()).unwrap();
        let plan = diff_estates(&e, Some(&e));
        assert!(plan.added.agents.is_empty());
        assert!(plan.blast_radius_text.contains("empty"));
    }
}
