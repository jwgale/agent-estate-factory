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
    let blast_radius_text = blast_radius(desired, &added, &removed, &changed, against);
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
    out.push_str(&render_security_iac(plan));
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
    let json_body = serde_json::to_string_pretty(plan).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, format!("serialize plan: {e}"))
    })?;
    std::fs::write(&json_path, json_body)?;
    std::fs::write(
        plans_dir.join(format!("{stem}.security.md")),
        render_security_iac(plan),
    )?;
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
        // Only `plan-*` blast-radius files. `apply-*.json` audits share this dir
        // and are a different schema — do not skip-parse them as empty plans.
        if let Some(stem) = name.strip_suffix(".md") {
            if stem.starts_with("plan-") && !stem.ends_with(".security") {
                stems.insert(stem.to_string());
            }
        } else if let Some(stem) = name.strip_suffix(".json") {
            if stem.starts_with("plan-") {
                stems.insert(stem.to_string());
            }
        }
    }
    let mut out = Vec::new();
    for stem in stems.into_iter().rev() {
        let json_path = plans_dir.join(format!("{stem}.json"));
        let json = if json_path.is_file() {
            Some(format!("{stem}.json"))
        } else if json_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{} is not a file", json_path.display()),
            ));
        } else {
            None
        };
        let loaded = match &json {
            Some(name) => Some(load_plan_json(&plans_dir.join(name))?),
            None => None,
        };
        out.push(PlanIndexEntry {
            markdown: format!("{stem}.md"),
            json,
            desired_hash: loaded.as_ref().map(|p| p.desired_hash.clone()),
            against_hash: loaded.as_ref().and_then(|p| p.against_hash.clone()),
            created_at: loaded.as_ref().map(|p| p.created_at.clone()),
            stem,
        });
    }
    Ok(out)
}

pub fn load_plan_json(path: &Path) -> Result<EstatePlan, std::io::Error> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
}

pub fn covering_plan(
    plans_dir: &Path,
    hash: &str,
) -> Result<Option<CoveringPlan>, std::io::Error> {
    let entries = list_plans(plans_dir)?;
    for entry in entries {
        let Some(json_name) = entry.json.clone() else {
            continue;
        };
        let plan = load_plan_json(&plans_dir.join(json_name))?;
        if plan.desired_hash == hash {
            return Ok(Some(CoveringPlan {
                stem: entry.stem,
                plan,
            }));
        }
    }
    Ok(None)
}

pub fn covering_plan_stem(
    plans_dir: &Path,
    hash: &str,
) -> Result<Option<String>, std::io::Error> {
    Ok(covering_plan(plans_dir, hash)?.map(|c| c.stem))
}

pub fn plan_covers_hash(plans_dir: &Path, hash: &str) -> Result<bool, std::io::Error> {
    Ok(covering_plan_stem(plans_dir, hash)?.is_some())
}

/// Fresh when there is no last apply, or the plan was taken against that apply.
/// A greenfield covering plan (`against_hash = None`) of the current desired hash still counts.
pub fn plan_against_is_fresh(plan: &EstatePlan, last_applied: Option<&str>) -> bool {
    match (last_applied, plan.against_hash.as_deref()) {
        (Some(applied), Some(against)) => applied == against,
        _ => true,
    }
}

/// Strict freshness for `--require-fresh-plan`.
/// A greenfield plan (`against_hash = None`) after an apply is STALE.
pub fn plan_against_is_fresh_strict(plan: &EstatePlan, last_applied: Option<&str>) -> bool {
    match (last_applied, plan.against_hash.as_deref()) {
        (None, _) => true,
        (Some(applied), Some(against)) => applied == against,
        (Some(_), None) => false,
    }
}

/// PR-reviewable Security-as-IaC: schema, hashes, and a blast radius a human can read.
pub fn plan_is_reviewable(plan: &EstatePlan) -> bool {
    plan.schema == "cell-one.plan.v0"
        && plan.desired_hash.starts_with("sha256:")
        && !plan.blast_radius_text.trim().is_empty()
        && !plan.created_at.trim().is_empty()
}

/// Count of added + removed + changed ids. Used by `estate plan diff --allow-wider`.
pub fn plan_blast_width(plan: &EstatePlan) -> usize {
    plan.added.item_count() + plan.removed.item_count() + plan.changed.item_count()
}

pub fn blast_grows(from: &EstatePlan, to: &EstatePlan) -> bool {
    plan_blast_width(to) > plan_blast_width(from)
}

/// Newest plan JSON in `plans_dir` (list_plans is newest-first).
/// A present plan JSON that does not parse is refuse, not "no plan".
pub fn latest_plan(plans_dir: &Path) -> Result<Option<EstatePlan>, std::io::Error> {
    let entries = list_plans(plans_dir)?;
    for entry in entries {
        let Some(name) = entry.json else {
            continue;
        };
        return Ok(Some(load_plan_json(&plans_dir.join(name))?));
    }
    Ok(None)
}

/// Human-readable plan-vs-plan blast compare. Not a gateway changelog.
pub fn render_plan_diff(from: &EstatePlan, to: &EstatePlan) -> String {
    let from_w = plan_blast_width(from);
    let to_w = plan_blast_width(to);
    let verdict = if to_w > from_w {
        "WIDER"
    } else if to_w < from_w {
        "narrower"
    } else {
        "same"
    };
    let mut out = String::from("Estate plan diff\n================\n");
    out.push_str(&format!("from hash: {}\n", from.desired_hash));
    out.push_str(&format!("to hash:   {}\n", to.desired_hash));
    out.push_str(&format!(
        "blast width: {from_w} -> {to_w} ({verdict})\n\n"
    ));
    out.push_str("From\n----\n");
    out.push_str(&render_review_diff(from));
    out.push('\n');
    out.push_str("To\n--\n");
    out.push_str(&render_review_diff(to));
    out.push('\n');
    if to_w > from_w {
        out.push_str("refuse:wider unless --allow-wider\n");
    }
    out
}

/// Single markdown ready to paste into a GitHub PR body. Does not open a PR.
pub fn render_plan_pr(
    plan: &EstatePlan,
    covering_stem: Option<&str>,
    reviewed: bool,
    refuse_risks: &[String],
) -> String {
    let mut out = String::from("# Cell One plan (paste into PR body)\n\n");
    out.push_str("Not a gateway. Cloud-agent stays unspawned. Feed does not auto-promote.\n\n");
    out.push_str(&format!("- **Desired hash:** {}\n", plan.desired_hash));
    match &plan.against_hash {
        Some(h) => out.push_str(&format!("- **Against hash:** {h}\n")),
        None => out.push_str("- **Against hash:** (greenfield)\n"),
    }
    out.push_str(&format!(
        "- **Covering plan:** {}\n",
        covering_stem.unwrap_or("(none — run `estate plan` first)")
    ));
    out.push_str(&format!(
        "- **Reviewed:** {}\n",
        if reviewed { "yes" } else { "no" }
    ));
    out.push_str(&format!(
        "- **Reviewable:** {}\n",
        if plan_is_reviewable(plan) { "yes" } else { "no" }
    ));
    out.push_str(&format!(
        "- **Blast width:** {}\n\n",
        plan_blast_width(plan)
    ));
    out.push_str("## Blast radius\n\n");
    out.push_str(&plan.blast_radius_text);
    out.push_str("\n\n");
    out.push_str(&render_security_iac(plan));
    out.push('\n');
    out.push_str(&render_review_diff(plan));
    out.push_str("\n## Refuse risks\n\n");
    if refuse_risks.is_empty() {
        out.push_str("- (none recorded)\n");
    } else {
        for risk in refuse_risks {
            out.push_str(&format!("- {risk}\n"));
        }
    }
    out.push_str("\n## Rails\n\n");
    out.push_str("- GitHub is source of truth. No Origin. No auto-promote.\n");
    out.push_str("- Cloud-agent: declared, not spawned.\n");
    out.push_str("- Apply is pause-safe disk. Sacred dual-layer stays.\n");
    out
}

/// Blast-radius markdown a human can PR-review before apply.
pub fn render_security_iac(plan: &EstatePlan) -> String {
    let mut out = String::from("Security-as-IaC (PR-review this blast radius)\n");
    out.push_str("----------------------------------------------\n");
    out.push_str(&format!("schema: {}\n", plan.schema));
    out.push_str(&format!("reviewable: {}\n", plan_is_reviewable(plan)));
    out.push_str(&format!("desired_hash: {}\n", plan.desired_hash));
    match &plan.against_hash {
        Some(h) => out.push_str(&format!("against_hash: {h}\n")),
        None => out.push_str("against_hash: (greenfield — stale after the first apply)\n"),
    }
    out.push_str(&format!("created_at: {}\n\n", plan.created_at));
    out.push_str("Blast radius\n");
    out.push_str(&plan.blast_radius_text);
    out.push_str("\n\n");
    out.push_str(&format!("+ agents: {}\n", fmt_list(&plan.added.agents)));
    out.push_str(&format!("- agents: {}\n", fmt_list(&plan.removed.agents)));
    out.push_str(&format!("~ agents: {}\n", fmt_list(&plan.changed.agents)));
    out.push_str(&format!("+ placements: {}\n", fmt_list(&plan.added.placements)));
    out.push_str(&format!("- placements: {}\n", fmt_list(&plan.removed.placements)));
    out.push_str(&format!("+ enrich_packs: {}\n", fmt_list(&plan.added.enrich_packs)));
    out.push_str(&format!(
        "+ model_bindings: {}\n",
        fmt_list(&plan.added.model_bindings)
    ));
    out.push_str(&format!(
        "- model_bindings: {}\n",
        fmt_list(&plan.removed.model_bindings)
    ));
    out.push_str(&format!(
        "~ model_bindings: {}\n",
        fmt_list(&plan.changed.model_bindings)
    ));
    out.push_str(&format!("+ intentions: {}\n", fmt_list(&plan.added.intentions)));
    out.push_str(&format!("- intentions: {}\n", fmt_list(&plan.removed.intentions)));
    out.push_str("\nApply without a covering, reviewable, fresh plan is refuse.\n");
    out.push_str("Cloud-agent placements stay unspawned. Feed does not auto-promote.\n");
    out
}

/// Copy a plan's markdown/json/security files into `reviewed/` for a human PR.
pub fn mark_plan_reviewed(
    plans_dir: &Path,
    reviewed_dir: &Path,
    stem: Option<&str>,
) -> Result<std::path::PathBuf, std::io::Error> {
    std::fs::create_dir_all(reviewed_dir)?;
    let entries = list_plans(plans_dir)?;
    let entry = match stem {
        Some(want) => entries.into_iter().find(|e| e.stem == want),
        None => entries.into_iter().next(),
    }
    .ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "no covering plan to mark reviewed")
    })?;
    for ext in ["md", "json", "security.md"] {
        let src = plans_dir.join(format!("{}.{}", entry.stem, ext));
        if src.exists() {
            std::fs::copy(&src, reviewed_dir.join(format!("{}.{}", entry.stem, ext)))?;
        }
    }
    let mut index = String::from(
        "# Reviewed plans\n\nHuman-copied blast-radius files. Commit these when apply needs a PR review.\n\n",
    );
    index.push_str(&format!(
        "- `{}` hash={} against={}\n",
        format!("{}.md", entry.stem),
        entry.desired_hash.as_deref().unwrap_or("-"),
        entry.against_hash.as_deref().unwrap_or("(greenfield)")
    ));
    std::fs::write(reviewed_dir.join("INDEX.md"), index)?;
    Ok(reviewed_dir.join(format!("{}.md", entry.stem)))
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
    against: Option<&Estate>,
) -> String {
    let greenfield = against.is_none();
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
    let populations: Vec<String> = estate
        .placements
        .iter()
        .map(|p| {
            let who = if p.agents.is_empty() {
                "(none)".to_string()
            } else {
                p.agents.join(",")
            };
            format!("{}[{who}]", p.id)
        })
        .collect();
    lines.push("Authority next to the workload (desired; plan does not enforce):".into());
    let mut authority_lines = Vec::new();
    for place in &estate.placements {
        if place.kind != PlacementKind::Box {
            authority_lines.push(format!(
                "  not-enforced: {} mesh-stub (cloud placement declared, not spawned)",
                place.id
            ));
            continue;
        }
        for agent_id in &place.agents {
            let Some(agent) = estate.agent(agent_id) else {
                continue;
            };
            for tool in &agent.tools {
                authority_lines.push(format!(
                    "  not-enforced: {agent_id} tool {} on {}",
                    tool.id, place.id
                ));
            }
            for mount in &agent.mounts {
                authority_lines.push(format!(
                    "  not-enforced: {agent_id} mount {} on {}",
                    mount.id, place.id
                ));
            }
            for mcp in &agent.mcp {
                authority_lines.push(format!(
                    "  not-enforced: {agent_id} mcp {} on {}",
                    mcp.id, place.id
                ));
            }
            for model in &agent.models {
                authority_lines.push(format!(
                    "  not-enforced: {agent_id} model {} on {}",
                    model.id, place.id
                ));
            }
        }
    }
    if authority_lines.is_empty() {
        lines.push("  (no placed declarations)".into());
    } else {
        lines.extend(authority_lines);
    }
    lines.push(
        "Uncertain: these rows are declarations on the estate file. Plan does not mediate a worker. `estate convey call --agent` is a separate check, and we do not know if that check sits on the worker path. Identity stays parked. Not a gateway."
            .into(),
    );
    lines.push(format!(
        "Capability mesh bind: `estate convey sync` stamps placement agents onto hop leases ({}). `estate convey call --agent` refuses an agent the lease does not name, and refuses a capability the estate does not allow. `estate convey authority` is a file check (would-allow, would-deny, not-enforced). It does not prove mediation. Identity stays parked. Not a gateway.",
        if populations.is_empty() {
            "no placements".to_string()
        } else {
            populations.join(" ")
        }
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
    lines.push(
        "Model class coverage (frontier and local use the same allow-intention rule):".into(),
    );
    lines.push(crate::firewall::describe_model_class_coverage(estate));
    lines.push(
        "Tool, MCP, and Mount coverage (an allow intention covers the object; these have no class):"
            .into(),
    );
    lines.push(crate::firewall::describe_declared_coverage(estate));
    lines.push("Model class delta (who gained or lost which class):".into());
    lines.push(model_class_delta(estate, against));
    lines.push(describe_agents_section(estate));
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
        self.item_count() == 0
    }

    pub fn item_count(&self) -> usize {
        self.agents.len()
            + self.lanes.len()
            + self.intentions.len()
            + self.model_bindings.len()
            + self.placements.len()
            + self.enrich_packs.len()
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

/// Per-agent blast block: id, lane, desktop, placement, declared counts,
/// and that agent's allow / deny / deny-default coverage. Plan does not spawn.
pub fn describe_agents_section(estate: &Estate) -> String {
    let model = crate::firewall::describe_model_class_coverage(estate);
    let declared = crate::firewall::describe_declared_coverage(estate);
    let mut lines = vec![
        "Agents".into(),
        "------".into(),
        "First-class agents. Plan does not spawn. Cloud-agent stays declared, not spawned. Control does not complete."
            .into(),
    ];
    if estate.agents.is_empty() {
        lines.push("(no agents)".into());
        return lines.join("\n");
    }
    for agent in &estate.agents {
        lines.push(format!("- id: {}", agent.id));
        lines.push(format!("  lane: {}", agent.lane));
        lines.push(format!("  desktop: {}", agent.desktop));
        lines.push(format!(
            "  placement: {}",
            agent_placement_line(estate, &agent.id)
        ));
        lines.push(format!(
            "  declared: tools={} mcp={} mounts={} models={}",
            agent.tools.len(),
            agent.mcp.len(),
            agent.mounts.len(),
            agent.models.len()
        ));
        lines.push("  model class:".into());
        push_agent_coverage(&mut lines, &model, &agent.id);
        lines.push("  tool mcp mount:".into());
        push_agent_coverage(&mut lines, &declared, &agent.id);
    }
    lines.join("\n")
}

fn agent_placement_line(estate: &Estate, agent_id: &str) -> String {
    let want = crate::sacred::normalize_name(agent_id);
    let mut parts = Vec::new();
    for place in &estate.placements {
        let named = place
            .agents
            .iter()
            .any(|id| crate::sacred::normalize_name(id) == want);
        if !named {
            continue;
        }
        match place.kind {
            PlacementKind::Box => parts.push(format!("box {}", place.id)),
            PlacementKind::CloudAgent => {
                parts.push(format!(
                    "cloud-agent {} (declared, not spawned)",
                    place.id
                ));
            }
        }
    }
    if parts.is_empty() {
        "none".into()
    } else {
        parts.join("; ")
    }
}

fn push_agent_coverage(lines: &mut Vec<String>, coverage: &str, agent_id: &str) {
    let matched: Vec<&str> = coverage
        .lines()
        .filter(|line| line.split_whitespace().next() == Some(agent_id))
        .collect();
    if matched.is_empty() {
        lines.push("    (none)".into());
        return;
    }
    for line in matched {
        lines.push(format!("    {line}"));
    }
}

fn model_use_keys(estate: &Estate) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for agent in &estate.agents {
        for model in &agent.models {
            let class = estate
                .model_bindings
                .iter()
                .find(|b| {
                    crate::sacred::normalize_name(&b.id)
                        == crate::sacred::normalize_name(&model.id)
                })
                .map(|b| b.class.as_str())
                .unwrap_or("undeclared");
            keys.insert(format!("{} {} {}", agent.id, class, model.id));
        }
    }
    keys
}

fn model_class_delta(desired: &Estate, against: Option<&Estate>) -> String {
    let have = model_use_keys(desired);
    let prev = against.map(model_use_keys).unwrap_or_default();
    let mut lines = Vec::new();
    for key in have.difference(&prev) {
        lines.push(format!("+ {key}"));
    }
    for key in prev.difference(&have) {
        lines.push(format!("- {key}"));
    }
    if lines.is_empty() {
        "(none)".into()
    } else {
        lines.join("\n")
    }
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
        assert!(plan.blast_radius_text.contains("not-enforced: research tool notes-append on cell-one-box"));
        assert!(plan.blast_radius_text.contains("Uncertain:"));
        assert!(plan.blast_radius_text.contains("Identity stays parked"));
        assert!(plan.blast_radius_text.contains("research tool notes-append: deny-default"));
        assert!(plan.blast_radius_text.contains("research mount notes: deny-default"));
        assert!(plan.blast_radius_text.contains("these have no class"));
        assert!(plan.blast_radius_text.contains("horizon frontier xai_grok: deny-default"));
        assert!(plan.blast_radius_text.contains("horizon local local_slm: deny-default"));
        assert!(plan.blast_radius_text.contains("research local local_slm: deny-default"));
        assert!(plan.blast_radius_text.contains("+ horizon frontier xai_grok"));
        assert!(plan.blast_radius_text.contains("+ research local local_slm"));
        let iac = render_security_iac(&plan);
        assert!(iac.contains("+ model_bindings:"));
        assert!(iac.contains("+ intentions:"));
        assert!(iac.contains("\nAgents\n"));
    }

    #[test]
    fn plan_agents_section_names_placement_and_coverage() {
        let mut e = load_estate_str(crate::tests::example_yaml()).unwrap();
        e.placements
            .iter_mut()
            .find(|p| p.id == "cell-one-box")
            .unwrap()
            .agents = vec!["horizon".into()];
        e.placements
            .iter_mut()
            .find(|p| p.id == "cursor-cloud")
            .unwrap()
            .agents = vec!["sanctum".into()];
        e.agents
            .iter_mut()
            .find(|a| a.id == "horizon")
            .unwrap()
            .mcp
            .push(crate::McpDecl {
                id: "docs".into(),
                description: None,
            });
        e.intentions.push(crate::Intention {
            subject_agent: "horizon".into(),
            object: "class:frontier".into(),
            kind: crate::IntentionKind::Model,
            effect: crate::Effect::Allow,
            note: None,
        });
        e.intentions.push(crate::Intention {
            subject_agent: "horizon".into(),
            object: "local_slm".into(),
            kind: crate::IntentionKind::Model,
            effect: crate::Effect::Deny,
            note: None,
        });
        e.intentions.push(crate::Intention {
            subject_agent: "horizon".into(),
            object: "mcp:docs".into(),
            kind: crate::IntentionKind::Mcp,
            effect: crate::Effect::Allow,
            note: None,
        });
        let plan = diff_estates(&e, None);
        let text = render_plan(&plan);
        let agents = text
            .split("Agents\n------\n")
            .nth(1)
            .expect("Agents section");
        assert!(agents.contains("- id: horizon"));
        assert!(agents.contains("lane: horizon"));
        assert!(agents.contains("desktop: horizon-desktop"));
        assert!(agents.contains("placement: box cell-one-box"));
        assert!(agents.contains("declared: tools=0 mcp=1 mounts=0 models=2"));
        assert!(agents.contains("horizon frontier xai_grok: allow"));
        assert!(agents.contains("horizon local local_slm: deny"));
        assert!(agents.contains("horizon mcp docs: allow"));
        assert!(agents.contains("- id: research"));
        assert!(agents.contains("placement: none"));
        assert!(agents.contains("research tool notes-append: deny-default"));
        assert!(agents.contains("research mount notes: deny-default"));
        assert!(agents.contains("- id: sanctum"));
        assert!(agents.contains(
            "placement: cloud-agent cursor-cloud (declared, not spawned)"
        ));
        assert!(agents.contains("Control does not complete"));
        assert!(text.contains("+ model_bindings:"));
        assert!(text.contains("+ intentions:"));
        assert!(describe_agents_section(&e).contains("- id: horizon"));
    }

    #[test]
    fn example_plan_agents_are_box_and_deny_default() {
        let e = load_estate_str(crate::tests::example_yaml()).unwrap();
        let text = render_plan(&diff_estates(&e, None));
        assert!(text.contains("\nAgents\n"));
        assert!(text.contains("- id: horizon"));
        assert!(text.contains("- id: research"));
        assert!(text.contains("- id: sanctum"));
        assert!(text.contains("placement: box cell-one-box"));
        assert!(text.contains("horizon frontier xai_grok: deny-default"));
        assert!(text.contains("research tool notes-append: deny-default"));
        assert!(text.contains("research mount notes: deny-default"));
        assert!(text.contains("+ intentions:"));
        assert!(text.contains("Security-as-IaC"));
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
        std::fs::write(dir.join("apply-unix1-deadbeef.json"), "{\"note\":\"not a plan\"}\n")
            .unwrap();
        assert!(plan_covers_hash(&dir, &plan.desired_hash).unwrap());
        assert!(!plan_covers_hash(&dir, "sha256:deadbeef").unwrap());
        assert!(covering_plan_stem(&dir, &plan.desired_hash)
            .unwrap()
            .is_some());
        let covering = covering_plan(&dir, &plan.desired_hash)
            .unwrap()
            .expect("covering plan");
        assert_eq!(covering.plan.schema, "cell-one.plan.v0");
        assert!(plan_against_is_fresh(&covering.plan, None));
        assert!(plan_against_is_fresh(&covering.plan, Some("sha256:other")));
        assert!(!plan_against_is_fresh_strict(&covering.plan, Some("sha256:other")));
        assert!(plan_against_is_fresh_strict(&covering.plan, None));
        assert!(plan_is_reviewable(&covering.plan));
        let iac = render_security_iac(&covering.plan);
        assert!(iac.contains("Security-as-IaC"));
        assert!(iac.contains("reviewable: true"));
        let pr = render_plan_pr(
            &covering.plan,
            Some(&covering.stem),
            true,
            &["cloud-agent: declared, not spawned".into()],
        );
        assert!(pr.contains("paste into PR body"));
        assert!(pr.contains("Blast radius"));
        assert!(pr.contains("Refuse risks"));
        assert!(pr.contains("Reviewed:** yes"));
        assert!(dir.join(format!("{}.security.md", covering.stem)).is_file());
        let reviewed = mark_plan_reviewed(&dir, &dir.join("reviewed"), Some(&covering.stem)).unwrap();
        assert!(reviewed.is_file());
        let mut stale = covering.plan.clone();
        stale.against_hash = Some("sha256:old".into());
        assert!(!plan_against_is_fresh(&stale, Some("sha256:new")));
        assert!(!plan_against_is_fresh_strict(&stale, Some("sha256:new")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_plan_json_is_refuse_not_empty() {
        let dir = std::env::temp_dir().join(format!(
            "cell-one-plan-garbage-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plan-unix1-deadbeef.json"), "not-json\n").unwrap();
        let list = list_plans(&dir).unwrap_err();
        assert!(
            list.to_string().contains("plan-unix1-deadbeef.json")
                || list.to_string().contains("Invalid")
                || list.to_string().contains("expected"),
            "{list}"
        );
        let cover = covering_plan(&dir, "sha256:deadbeef").unwrap_err();
        assert!(
            cover.to_string().contains("plan-unix1-deadbeef.json")
                || cover.to_string().contains("Invalid")
                || cover.to_string().contains("expected"),
            "{cover}"
        );
        let latest = latest_plan(&dir).unwrap_err();
        assert!(
            latest.to_string().contains("plan-unix1-deadbeef.json")
                || latest.to_string().contains("Invalid")
                || latest.to_string().contains("expected"),
            "{latest}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("plan-unix1-deadbeef.json")).unwrap(),
            "not-json\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_fixture_fails_strict_freshness() {
        let stale: EstatePlan =
            serde_json::from_str(include_str!("../../../examples/invalid/stale-plan.json")).unwrap();
        assert!(!plan_against_is_fresh_strict(
            &stale,
            Some("sha256:0000000000000000000000000000000000000000000000000000000000000001")
        ));
        assert!(!plan_against_is_fresh_strict(&stale, Some("sha256:other")));
    }

    #[test]
    fn blast_width_grows_when_new_plan_adds_more() {
        let e = load_estate_str(crate::tests::example_yaml()).unwrap();
        let narrow = diff_estates(&e, Some(&e));
        let wide = diff_estates(&e, None);
        assert_eq!(plan_blast_width(&narrow), 0);
        assert!(plan_blast_width(&wide) > 0);
        assert!(blast_grows(&narrow, &wide));
        assert!(!blast_grows(&wide, &narrow));
        let report = render_plan_diff(&narrow, &wide);
        assert!(report.contains("WIDER"));
        assert!(report.contains("refuse:wider"));
    }
}
