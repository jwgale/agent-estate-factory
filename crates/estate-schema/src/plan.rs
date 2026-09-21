use crate::hash::estate_hash;
use crate::types::{Estate, PlacementKind};
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
    #[serde(default)]
    pub placements: Vec<String>,
    #[serde(default)]
    pub enrich_packs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EstatePlan {
    #[serde(default = "default_plan_schema")]
    pub schema: String,
    pub desired_hash: String,
    pub against_hash: Option<String>,
    pub added: PlanDelta,
    pub removed: PlanDelta,
    pub changed: PlanDelta,
    pub blast_radius_text: String,
    pub created_at: String,
}

fn default_plan_schema() -> String {
    "cell-one.plan.v0".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoveringPlan {
    pub stem: String,
    pub plan: EstatePlan,
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
                placements: names(desired.placements.iter().map(|p| p.id.as_str())),
                enrich_packs: names(desired.enrich_packs.packs.iter().map(|p| p.id.as_str())),
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
            let add_place = set_diff(ids(desired.placements.iter().map(|p| p.id.as_str())), ids(prev.placements.iter().map(|p| p.id.as_str())));
            let rem_place = set_diff(ids(prev.placements.iter().map(|p| p.id.as_str())), ids(desired.placements.iter().map(|p| p.id.as_str())));
            let add_packs = set_diff(ids(desired.enrich_packs.packs.iter().map(|p| p.id.as_str())), ids(prev.enrich_packs.packs.iter().map(|p| p.id.as_str())));
            let rem_packs = set_diff(ids(prev.enrich_packs.packs.iter().map(|p| p.id.as_str())), ids(desired.enrich_packs.packs.iter().map(|p| p.id.as_str())));
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
                placements: changed_ids(
                    desired.placements.iter().map(|p| (p.id.as_str(), fingerprint(p))),
                    prev.placements.iter().map(|p| (p.id.as_str(), fingerprint(p))),
                ),
                enrich_packs: vec![],
            };
            (
                PlanDelta {
                    agents: add_agents,
                    lanes: add_lanes,
                    intentions: add_int,
                    model_bindings: add_bind,
                    placements: add_place,
                    enrich_packs: add_packs,
                },
                PlanDelta {
                    agents: rem_agents,
                    lanes: rem_lanes,
                    intentions: rem_int,
                    model_bindings: rem_bind,
                    placements: rem_place,
                    enrich_packs: rem_packs,
                },
                changed,
            )
        }
    };

    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let blast_radius_text = blast_radius(desired, &added, &removed, &changed, against.is_none());
    EstatePlan {
        schema: default_plan_schema(),
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
    out.push_str("\n");
    out.push_str(&render_review_diff(plan));
    out.push_str("\nAdded\n");
    out.push_str(&render_delta(&plan.added));
    out.push_str("Removed\n");
    out.push_str(&render_delta(&plan.removed));
    out.push_str("Changed\n");
    out.push_str(&render_delta(&plan.changed));
    out
}

/// PR-reviewable markdown: what apply will touch. Not a gateway changelog.
pub fn render_review_diff(plan: &EstatePlan) -> String {
    let mut out = String::from("Reviewable diff (commit this file to review apply)\n");
    out.push_str("----------------------------------------------------\n");
    out.push_str(&format!("+ agents:          {}\n", fmt_list(&plan.added.agents)));
    out.push_str(&format!("- agents:          {}\n", fmt_list(&plan.removed.agents)));
    out.push_str(&format!("~ agents:          {}\n", fmt_list(&plan.changed.agents)));
    out.push_str(&format!("+ lanes:           {}\n", fmt_list(&plan.added.lanes)));
    out.push_str(&format!("- lanes:           {}\n", fmt_list(&plan.removed.lanes)));
    out.push_str(&format!("~ lanes:           {}\n", fmt_list(&plan.changed.lanes)));
    out.push_str(&format!("+ intentions:      {}\n", fmt_list(&plan.added.intentions)));
    out.push_str(&format!("- intentions:      {}\n", fmt_list(&plan.removed.intentions)));
    out.push_str(&format!("+ model_bindings:  {}\n", fmt_list(&plan.added.model_bindings)));
    out.push_str(&format!("- model_bindings:  {}\n", fmt_list(&plan.removed.model_bindings)));
    out.push_str(&format!("~ model_bindings:  {}\n", fmt_list(&plan.changed.model_bindings)));
    out.push_str(&format!("+ placements:      {}\n", fmt_list(&plan.added.placements)));
    out.push_str(&format!("- placements:      {}\n", fmt_list(&plan.removed.placements)));
    out.push_str(&format!("~ placements:      {}\n", fmt_list(&plan.changed.placements)));
    out.push_str(&format!("+ enrich_packs:    {}\n", fmt_list(&plan.added.enrich_packs)));
    out.push_str(&format!("- enrich_packs:    {}\n", fmt_list(&plan.removed.enrich_packs)));
    out.push_str("Control does not invoke models. Cloud-agent placements are not spawned.\n");
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
    let _ = write_plan_index(plans_dir);
    Ok(md_path)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanIndexEntry {
    pub stem: String,
    pub markdown: String,
    pub json: Option<String>,
    #[serde(default)]
    pub desired_hash: Option<String>,
    #[serde(default)]
    pub against_hash: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

pub fn list_plans(plans_dir: &Path) -> Result<Vec<PlanIndexEntry>, std::io::Error> {
    if !plans_dir.exists() {
        return Ok(Vec::new());
    }
    let mut stems: BTreeSet<String> = BTreeSet::new();
    for entry in std::fs::read_dir(plans_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(stem) = name.strip_suffix(".md") {
            if stem != "INDEX" && stem != "README" {
                stems.insert(stem.to_string());
            }
        } else if let Some(stem) = name.strip_suffix(".json") {
            stems.insert(stem.to_string());
        }
    }
    Ok(stems
        .into_iter()
        .rev()
        .map(|stem| {
            let json = if plans_dir.join(format!("{stem}.json")).exists() {
                Some(format!("{stem}.json"))
            } else {
                None
            };
            let loaded = json
                .as_ref()
                .and_then(|name| std::fs::read_to_string(plans_dir.join(name)).ok())
                .and_then(|text| serde_json::from_str::<EstatePlan>(&text).ok());
            PlanIndexEntry {
                markdown: format!("{stem}.md"),
                json,
                desired_hash: loaded.as_ref().map(|p| p.desired_hash.clone()),
                against_hash: loaded.as_ref().and_then(|p| p.against_hash.clone()),
                created_at: loaded.as_ref().map(|p| p.created_at.clone()),
                stem,
            }
        })
        .collect())
}

pub fn load_plan_json(path: &Path) -> Result<EstatePlan, std::io::Error> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
}

pub fn covering_plan(plans_dir: &Path, hash: &str) -> Option<CoveringPlan> {
    let entries = list_plans(plans_dir).ok()?;
    for entry in entries {
        let Some(json_name) = entry.json.clone() else {
            continue;
        };
        let Ok(plan) = load_plan_json(&plans_dir.join(json_name)) else {
            continue;
        };
        if plan.desired_hash == hash {
            return Some(CoveringPlan {
                stem: entry.stem,
                plan,
            });
        }
    }
    None
}

pub fn covering_plan_stem(plans_dir: &Path, hash: &str) -> Option<String> {
    covering_plan(plans_dir, hash).map(|c| c.stem)
}

pub fn plan_covers_hash(plans_dir: &Path, hash: &str) -> bool {
    covering_plan_stem(plans_dir, hash).is_some()
}

/// Fresh when there is no last apply, or the plan was taken against that apply.
/// A greenfield covering plan (`against_hash = None`) of the current desired hash still counts.
pub fn plan_against_is_fresh(plan: &EstatePlan, last_applied: Option<&str>) -> bool {
    match (last_applied, plan.against_hash.as_deref()) {
        (Some(applied), Some(against)) => applied == against,
        _ => true,
    }
}

pub fn write_plan_index(plans_dir: &Path) -> Result<std::path::PathBuf, std::io::Error> {
    std::fs::create_dir_all(plans_dir)?;
    let entries = list_plans(plans_dir)?;
    let mut md = String::from("# Plan history\n\nAppend-only blast-radius files. Commit a plan markdown into a PR when you want Jason to review apply.\n\n");
    if entries.is_empty() {
        md.push_str("(no plans yet)\n");
    } else {
        for entry in &entries {
            md.push_str(&format!(
                "- `{}` hash={} against={}\n",
                entry.markdown,
                entry.desired_hash.as_deref().unwrap_or("-"),
                entry.against_hash.as_deref().unwrap_or("(greenfield)")
            ));
        }
    }
    let path = plans_dir.join("INDEX.md");
    std::fs::write(&path, md)?;
    Ok(path)
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
    lines.push(format!(
        "Apply will bind {} sessions, ensure {} lane roots, and record {} model binding(s). Control does not invoke models.",
        estate.agents.len(),
        estate.lanes.len(),
        estate.model_bindings.len()
    ));
    if estate.intentions.is_empty() {
        lines.push("No cross-lane intentions: memory firewall stays deny-default.".into());
    } else {
        lines.push(format!(
            "{} compiled intention(s) will change who can cross a lane or tool.",
            estate.intentions.len()
        ));
    }
    let bindings: Vec<String> = estate
        .model_bindings
        .iter()
        .map(|b| format!("{}={} wired={}", b.class.as_str(), b.id, b.wired))
        .collect();
    if estate.model_bindings.iter().any(|b| b.wired) {
        lines.push(format!(
            "Equal-class bindings are live-capable (data plane only): {}.",
            bindings.join(", ")
        ));
    } else {
        lines.push(format!(
            "Model bindings remain placeholders: {}.",
            bindings.join(", ")
        ));
    }
    lines.push(format!(
        "Sacred exclusions stay out of the estate: {}.",
        estate
            .sacred_exclusions
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    let boxes = estate
        .placements
        .iter()
        .filter(|p| p.kind == PlacementKind::Box)
        .count();
    let cloud = estate
        .placements
        .iter()
        .filter(|p| p.kind == PlacementKind::CloudAgent)
        .count();
    lines.push(format!(
        "Placements declared: {boxes} box, {cloud} cloud-agent stub(s). Floor does not spawn cloud agents."
    ));
    if estate.enrich_packs.packs.is_empty() {
        lines.push("Enrich packs stay empty until Jason curates (manual; no auto-promote).".into());
    } else {
        lines.push(format!(
            "Enrich packs listed (still manual): {}.",
            estate
                .enrich_packs
                .packs
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
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
        "  agents: {}\n  lanes: {}\n  intentions: {}\n  model_bindings: {}\n  placements: {}\n  enrich_packs: {}\n",
        fmt_list(&delta.agents),
        fmt_list(&delta.lanes),
        fmt_list(&delta.intentions),
        fmt_list(&delta.model_bindings),
        fmt_list(&delta.placements),
        fmt_list(&delta.enrich_packs)
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
            && self.placements.is_empty()
            && self.enrich_packs.is_empty()
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
        assert!(plan.blast_radius_text.contains("3 agents") || plan.blast_radius_text.contains("3 sessions"));
        assert!(plan.blast_radius_text.contains("deny-default"));
        assert!(plan.blast_radius_text.contains("live-capable") || plan.blast_radius_text.contains("placeholder"));
    }

    #[test]
    fn identical_diff_is_empty() {
        let e = load_estate_str(crate::tests::example_yaml()).unwrap();
        let plan = diff_estates(&e, Some(&e));
        assert!(plan.added.agents.is_empty());
        assert!(plan.blast_radius_text.contains("empty"));
    }

    #[test]
    fn review_diff_names_placements() {
        let e = load_estate_str(crate::tests::example_yaml()).unwrap();
        let plan = diff_estates(&e, None);
        let review = render_review_diff(&plan);
        assert!(review.contains("+ placements:"));
        assert!(review.contains("cell-one-box") || review.contains("cursor-cloud"));
        assert!(plan.blast_radius_text.contains("cloud-agent stub"));
    }

    #[test]
    fn plan_covers_hash_reads_json_history() {
        let e = load_estate_str(crate::tests::example_yaml()).unwrap();
        let plan = diff_estates(&e, None);
        let dir = std::env::temp_dir().join(format!(
            "cell-one-plan-cover-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        write_plan(&dir, &plan).unwrap();
        assert!(plan_covers_hash(&dir, &plan.desired_hash));
        assert!(!plan_covers_hash(&dir, "sha256:deadbeef"));
        assert!(covering_plan_stem(&dir, &plan.desired_hash).is_some());
        let covering = covering_plan(&dir, &plan.desired_hash).unwrap();
        assert_eq!(covering.plan.schema, "cell-one.plan.v0");
        assert!(plan_against_is_fresh(&covering.plan, None));
        assert!(plan_against_is_fresh(&covering.plan, Some("sha256:other")));
        let mut stale = covering.plan.clone();
        stale.against_hash = Some("sha256:old".into());
        assert!(!plan_against_is_fresh(&stale, Some("sha256:new")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
