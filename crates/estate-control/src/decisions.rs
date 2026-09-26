//! Keel-style middle layer for `estate convey call`, `estate authorize`,
//! and `estate complete`.
//!
//! The host prepares eligible model-binding candidates, a selector chooses
//! one opaque id or abstains, and the host re-validates that choice. The
//! selector does not grant permission. A convey call is still decided by
//! the hop lease and the estate intention. An authorize check is still
//! decided by `authorize`. `estate complete` still authorizes, then
//! delegates complete to the data-plane driver for the chosen binding.
//! A fallback id is recorded and not applied.

use anyhow::{bail, Context, Result};
use estate_schema::{authorize, AccessRequest, Estate, IntentionKind, ModelBinding};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub(crate) const RECEIPT_SCHEMA: &str = "cell-one.decision-receipt.v0";
pub(crate) const SELECT_SCHEMA: &str = "cell-one.decision-select.v0";
pub(crate) const SELECT_FILE: &str = "decision-select.json";
pub(crate) const JOURNAL_FILE: &str = "receipts.jsonl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectHint {
    pub select: Option<String>,
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CandidateSummary {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DecisionReceipt {
    pub schema: String,
    pub id: String,
    pub seq: u64,
    pub hop_id: String,
    pub capability: String,
    #[serde(default)]
    pub agent: Option<String>,
    /// Empty on `estate convey call`. `authorize` on `estate authorize`.
    /// `complete` on `estate complete`. Omitted from the JSON when empty
    /// so convey lines stay the same shape.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub surface: String,
    pub stage: String,
    pub candidates: Vec<CandidateSummary>,
    pub result: String,
    pub validation: String,
    pub fallback: Option<String>,
    pub outcome: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Prepared {
    id: String,
    digest: String,
}

fn journal_path(state_dir: &Path) -> std::path::PathBuf {
    state_dir.join("decisions").join(JOURNAL_FILE)
}

/// Object keys are sorted so YAML key order is not param drift.
fn canonical_json(value: &serde_json::Value) -> String {
    fn canon(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                let mut out = serde_json::Map::new();
                for key in keys {
                    out.insert(key.clone(), canon(&map[key]));
                }
                serde_json::Value::Object(out)
            }
            serde_json::Value::Array(items) => {
                serde_json::Value::Array(items.iter().map(canon).collect())
            }
            other => other.clone(),
        }
    }
    serde_json::to_string(&canon(value)).unwrap_or_else(|err| format!("\"unserializable:{err}\""))
}

/// Id, class, driver, and binding params. Param drift changes the digest.
fn binding_digest(binding: &ModelBinding) -> String {
    let mut hasher = Sha256::new();
    hasher.update(binding.id.as_bytes());
    hasher.update(b"\n");
    hasher.update(binding.class.as_str().as_bytes());
    hasher.update(b"\n");
    hasher.update(binding.driver.as_bytes());
    hasher.update(b"\n");
    hasher.update(canonical_json(&binding.params).as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Wired bindings the named agent may use. An unnamed call has no subject,
/// so the eligible set is empty. Missing coverage stays ineligible.
fn prepare_candidates(estate: &Estate, agent: Option<&str>) -> Vec<Prepared> {
    let Some(agent_id) = agent else {
        return Vec::new();
    };
    let Some(agent_row) = estate.agent(agent_id) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for binding in &estate.model_bindings {
        if !binding.wired || !agent_row.has_model(&binding.id) {
            continue;
        }
        let allowed = authorize(
            estate,
            &AccessRequest {
                subject_agent: agent_id,
                kind: IntentionKind::Model,
                object: &binding.id,
            },
        )
        .is_allow();
        if allowed {
            out.push(Prepared {
                id: binding.id.clone(),
                digest: binding_digest(binding),
            });
        }
    }
    out
}

/// One eligible id is selected. Zero or more than one abstains: frontier
/// and local are equal class, and this selector does not rank them.
/// A hint may name an id. That name is not a grant.
fn select(prepared: &[Prepared], hint: Option<&SelectHint>) -> (String, Option<String>) {
    if let Some(hint) = hint {
        return match &hint.select {
            Some(id) => (id.clone(), hint.digest.clone()),
            None => ("abstain".into(), None),
        };
    }
    match prepared {
        [] => ("abstain".into(), None),
        [one] => (one.id.clone(), Some(one.digest.clone())),
        _ => ("abstain".into(), None),
    }
}

fn revalidate(
    prepared: &[Prepared],
    result: &str,
    claimed_digest: Option<&str>,
    expired: bool,
) -> (String, Option<String>) {
    if expired {
        return ("expired".into(), None);
    }
    if result == "abstain" {
        return ("ok".into(), None);
    }
    let current = prepared.iter().find(|row| row.id == result);
    let validation = match (current, claimed_digest) {
        (None, _) => "ineligible",
        (Some(_), None) => "ok",
        (Some(row), Some(digest)) if digest == row.digest => "ok",
        (Some(_), Some(_)) => "stale",
    };
    let fallback = match validation {
        "stale" | "ineligible" => match prepared {
            [one] => Some(one.id.clone()),
            _ => None,
        },
        _ => None,
    };
    (validation.into(), fallback)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Selection {
    pub result: String,
    pub validation: String,
    pub fallback: Option<String>,
    pub stage: String,
}

/// Host prepare / select / revalidate without writing a receipt.
pub(crate) fn resolve_selection(
    estate: &Estate,
    agent: Option<&str>,
    hint: Option<&SelectHint>,
    expired: bool,
) -> Selection {
    let prepared = prepare_candidates(estate, agent);
    let (result, claimed) = select(&prepared, hint);
    let (validation, fallback) = revalidate(&prepared, &result, claimed.as_deref(), expired);
    let stage = stage_name(&result, &validation).to_string();
    Selection {
        result,
        validation,
        fallback,
        stage,
    }
}

fn stage_name(result: &str, validation: &str) -> &'static str {
    match validation {
        "stale" | "ineligible" => "fallback",
        "expired" => "validate",
        "ok" if result == "abstain" => "select",
        _ => "validate",
    }
}

pub(crate) fn outcome_stub_from_text(text: &str) -> String {
    let Some(rest) = text.strip_prefix("refuse:") else {
        return "refuse:other".into();
    };
    let class = rest.split(':').next().unwrap_or("other").trim();
    if class.is_empty() {
        "refuse:other".into()
    } else {
        format!("refuse:{class}")
    }
}

fn receipt_id(seq: u64, material: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seq.to_string().as_bytes());
    hasher.update(b"\n");
    hasher.update(material.as_bytes());
    let hex = format!("{:x}", hasher.finalize());
    format!("r-{seq}-{}", &hex[..8])
}

fn next_seq(path: &Path) -> Result<u64> {
    if !path.is_file() {
        return Ok(1);
    }
    let text = fs::read_to_string(path)?;
    let n = text.lines().filter(|line| !line.is_empty()).count() as u64;
    Ok(n + 1)
}

/// Optional `{state_dir}/decision-select.json`. Absent is the deterministic
/// selector. Present and unreadable or the wrong shape is `refuse:decision-select`.
pub(crate) fn load_select_hint(state_dir: &Path) -> Result<Option<SelectHint>> {
    let path = state_dir.join(SELECT_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("refuse:decision-select: read {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("refuse:decision-select: parse {}", path.display()))?;
    if value.get("schema").and_then(|s| s.as_str()) != Some(SELECT_SCHEMA) {
        bail!("refuse:decision-select: schema must be {SELECT_SCHEMA}");
    }
    let select = match value.get("select") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(raw)) if raw == "abstain" => None,
        Some(serde_json::Value::String(raw)) if !raw.is_empty() => Some(raw.clone()),
        _ => bail!("refuse:decision-select: select must be an id, abstain, or null"),
    };
    let digest = match value.get("digest") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(raw)) if !raw.is_empty() => Some(raw.clone()),
        _ => bail!("refuse:decision-select: digest must be a non-empty string or null"),
    };
    Ok(Some(SelectHint { select, digest }))
}

pub(crate) fn record_convey_receipt(
    estate: &Estate,
    state_dir: &Path,
    hop_id: &str,
    capability: &str,
    agent: Option<&str>,
    hint: Option<&SelectHint>,
    outcome: &str,
    expired: bool,
) -> Result<DecisionReceipt> {
    record_receipt(
        estate, state_dir, hop_id, capability, agent, hint, outcome, expired, "",
    )
}

/// Authorize check. `hop_id` is the intention kind and `capability` is the
/// object. `surface` is `authorize`. No hop-lease clock: `expired` stays false.
/// The same host prepare / select / revalidate path as a convey call.
pub(crate) fn record_authorize_receipt(
    estate: &Estate,
    state_dir: &Path,
    kind: &str,
    object: &str,
    agent: &str,
    hint: Option<&SelectHint>,
    outcome: &str,
) -> Result<DecisionReceipt> {
    record_receipt(
        estate,
        state_dir,
        kind,
        object,
        Some(agent),
        hint,
        outcome,
        false,
        "authorize",
    )
}

/// Estate-bound complete. `hop_id` is `model` and `capability` is the
/// binding that complete targeted. `surface` is `complete`. No hop-lease
/// clock: `expired` stays false. The same host prepare / select /
/// revalidate path as authorize. The selector does not grant.
pub(crate) fn record_complete_receipt(
    estate: &Estate,
    state_dir: &Path,
    object: &str,
    agent: &str,
    hint: Option<&SelectHint>,
    outcome: &str,
) -> Result<DecisionReceipt> {
    record_receipt(
        estate,
        state_dir,
        "model",
        object,
        Some(agent),
        hint,
        outcome,
        false,
        "complete",
    )
}

pub(crate) fn complete_driver_outcome(err: &model_estate::ModelError) -> String {
    match err {
        model_estate::ModelError::MissingEndpoint(_) => "refuse:missing-endpoint".into(),
        model_estate::ModelError::MissingCreds(_) | model_estate::ModelError::MissingFrontierKey => {
            "refuse:missing-creds".into()
        }
        model_estate::ModelError::NotWired(_) => "refuse:not-wired".into(),
        model_estate::ModelError::Unknown(_) => "refuse:unknown-binding".into(),
        model_estate::ModelError::Denied(_) => "refuse:denied".into(),
        model_estate::ModelError::Refused(_) => "refuse:refused".into(),
        other => outcome_stub_from_text(&other.to_string()),
    }
}

fn record_receipt(
    estate: &Estate,
    state_dir: &Path,
    hop_id: &str,
    capability: &str,
    agent: Option<&str>,
    hint: Option<&SelectHint>,
    outcome: &str,
    expired: bool,
    surface: &str,
) -> Result<DecisionReceipt> {
    let prepared = prepare_candidates(estate, agent);
    let (result, claimed) = select(&prepared, hint);
    let (validation, fallback) = revalidate(&prepared, &result, claimed.as_deref(), expired);
    let stage = stage_name(&result, &validation).to_string();
    let candidates = prepared
        .iter()
        .map(|row| CandidateSummary { id: row.id.clone() })
        .collect::<Vec<_>>();
    let dir = state_dir.join("decisions");
    fs::create_dir_all(&dir)?;
    let path = dir.join(JOURNAL_FILE);
    let seq = next_seq(&path)?;
    let candidate_ids = candidates
        .iter()
        .map(|row| row.id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let mut material = format!(
        "{hop_id}\n{capability}\n{}\n{candidate_ids}\n{result}\n{validation}\n{}\n{outcome}\n{stage}",
        agent.unwrap_or(""),
        fallback.as_deref().unwrap_or("")
    );
    if !surface.is_empty() {
        material.push('\n');
        material.push_str(surface);
    }
    let receipt = DecisionReceipt {
        schema: RECEIPT_SCHEMA.into(),
        id: receipt_id(seq, &material),
        seq,
        hop_id: hop_id.to_string(),
        capability: capability.to_string(),
        agent: agent.map(|s| s.to_string()),
        surface: surface.to_string(),
        stage,
        candidates,
        result,
        validation,
        fallback,
        outcome: outcome.to_string(),
    };
    check_receipt(&receipt)?;
    let line = serde_json::to_string(&receipt)?;
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    writeln!(file, "{line}")?;
    Ok(receipt)
}

fn check_receipt(receipt: &DecisionReceipt) -> Result<()> {
    if receipt.schema != RECEIPT_SCHEMA {
        bail!("refuse:decision-receipt: schema");
    }
    match receipt.surface.as_str() {
        "" | "authorize" | "complete" => {}
        _ => bail!("refuse:decision-receipt: surface"),
    }
    if receipt.id.is_empty() || receipt.seq == 0 {
        bail!("refuse:decision-receipt: id");
    }
    match receipt.stage.as_str() {
        "select" | "validate" | "fallback" => {}
        _ => bail!("refuse:decision-receipt: stage"),
    }
    match receipt.validation.as_str() {
        "ok" | "stale" | "ineligible" | "expired" => {}
        _ => bail!("refuse:decision-receipt: validation"),
    }
    if receipt.result.is_empty() {
        bail!("refuse:decision-receipt: result");
    }
    if receipt.outcome != "allow" && !receipt.outcome.starts_with("refuse:") {
        bail!("refuse:decision-receipt: outcome");
    }
    if receipt
        .outcome
        .split_whitespace()
        .any(|word| word == "enforced")
    {
        bail!("refuse:decision-receipt: outcome");
    }
    Ok(())
}

pub(crate) fn cite_line(receipt: &DecisionReceipt) -> String {
    match &receipt.fallback {
        Some(id) => format!(
            "decision receipt: {} result={} validation={} fallback={id}",
            receipt.id, receipt.result, receipt.validation
        ),
        None => format!(
            "decision receipt: {} result={} validation={}",
            receipt.id, receipt.result, receipt.validation
        ),
    }
}

pub(crate) fn load_receipts(state_dir: &Path) -> Result<Vec<DecisionReceipt>> {
    let path = journal_path(state_dir);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path)?;
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let receipt: DecisionReceipt = serde_json::from_str(line)
            .with_context(|| format!("refuse:decision-receipt: line {}", index + 1))?;
        check_receipt(&receipt)?;
        out.push(receipt);
    }
    Ok(out)
}

pub(crate) fn render_report(receipts: &[DecisionReceipt]) -> String {
    let mut select = 0usize;
    let mut validate = 0usize;
    let mut fallback_stage = 0usize;
    let mut ok = 0usize;
    let mut stale = 0usize;
    let mut ineligible = 0usize;
    let mut expired = 0usize;
    let mut none = 0usize;
    let mut by_id: BTreeMap<&str, usize> = BTreeMap::new();
    for receipt in receipts {
        match receipt.stage.as_str() {
            "select" => select += 1,
            "validate" => validate += 1,
            "fallback" => fallback_stage += 1,
            _ => {}
        }
        match receipt.validation.as_str() {
            "ok" => ok += 1,
            "stale" => stale += 1,
            "ineligible" => ineligible += 1,
            "expired" => expired += 1,
            _ => {}
        }
        match &receipt.fallback {
            Some(id) => *by_id.entry(id.as_str()).or_default() += 1,
            None => none += 1,
        }
    }
    let n = receipts.len();
    let mut out = format!(
        "decision receipts: {n}\nstage prepare={n} select={select} validate={validate} fallback={fallback_stage}\nvalidation ok={ok} stale={stale} ineligible={ineligible} expired={expired}\nfallback none={none}\n"
    );
    for (id, count) in by_id {
        out.push_str(&format!("fallback {id}={count}\n"));
    }
    out
}

pub(crate) fn cmd_decisions_export(state_dir: &Path, out: &Path) -> Result<()> {
    let path = journal_path(state_dir);
    let body = if path.is_file() {
        let text = fs::read_to_string(&path)?;
        for (index, line) in text.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let receipt: DecisionReceipt = serde_json::from_str(line)
                .with_context(|| format!("refuse:decision-receipt: line {}", index + 1))?;
            check_receipt(&receipt)?;
        }
        text
    } else {
        String::new()
    };
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(out, &body)?;
    let n = body.lines().filter(|line| !line.is_empty()).count();
    println!("wrote {n} decision replay case(s) to {}", out.display());
    Ok(())
}

pub(crate) fn cmd_decisions_report(state_dir: &Path) -> Result<()> {
    let receipts = load_receipts(state_dir)?;
    print!("{}", render_report(&receipts));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use estate_schema::{Effect, Intention, ModelUseDecl};

    fn example() -> Estate {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/estate.yaml");
        estate_schema::load_estate(&path).unwrap()
    }

    fn allow_local(estate: &mut Estate, agent: &str) {
        estate.intentions.push(Intention {
            subject_agent: agent.into(),
            object: "class:local".into(),
            kind: IntentionKind::Model,
            effect: Effect::Allow,
            note: None,
        });
    }

    #[test]
    fn unnamed_call_abstains_without_ranking_bindings() {
        let estate = example();
        let prepared = prepare_candidates(&estate, None);
        assert!(prepared.is_empty());
        let (result, digest) = select(&prepared, None);
        assert_eq!(result, "abstain");
        assert!(digest.is_none());
        let (validation, fallback) = revalidate(&prepared, &result, None, false);
        assert_eq!(validation, "ok");
        assert!(fallback.is_none());
        assert_eq!(stage_name(&result, &validation), "select");
    }

    #[test]
    fn one_eligible_seat_is_selected_and_revalidated() {
        let mut estate = example();
        allow_local(&mut estate, "research");
        let prepared = prepare_candidates(&estate, Some("research"));
        assert_eq!(prepared.len(), 1);
        assert_eq!(prepared[0].id, "local_slm");
        let (result, digest) = select(&prepared, None);
        assert_eq!(result, "local_slm");
        let (validation, fallback) = revalidate(&prepared, &result, digest.as_deref(), false);
        assert_eq!(validation, "ok");
        assert!(fallback.is_none());
        assert_eq!(stage_name(&result, &validation), "validate");
        let selection = resolve_selection(&estate, Some("research"), None, false);
        assert_eq!(selection.result, "local_slm");
        assert_eq!(selection.validation, "ok");
        assert!(selection.fallback.is_none());
        assert_eq!(selection.stage, "validate");
    }

    #[test]
    fn equal_class_frontier_and_local_abstain() {
        let mut estate = example();
        estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap()
            .models
            .push(ModelUseDecl {
                id: "xai_grok".into(),
                description: None,
            });
        allow_local(&mut estate, "research");
        estate.intentions.push(Intention {
            subject_agent: "research".into(),
            object: "class:frontier".into(),
            kind: IntentionKind::Model,
            effect: Effect::Allow,
            note: None,
        });
        let prepared = prepare_candidates(&estate, Some("research"));
        assert_eq!(prepared.len(), 2);
        let (result, _) = select(&prepared, None);
        assert_eq!(result, "abstain");
        let (validation, fallback) = revalidate(&prepared, &result, None, false);
        assert_eq!(validation, "ok");
        assert!(fallback.is_none());
    }

    #[test]
    fn ineligible_hint_records_the_one_eligible_fallback_and_does_not_grant() {
        let mut estate = example();
        allow_local(&mut estate, "research");
        let prepared = prepare_candidates(&estate, Some("research"));
        let hint = SelectHint {
            select: Some("xai_grok".into()),
            digest: None,
        };
        let (result, digest) = select(&prepared, Some(&hint));
        assert_eq!(result, "xai_grok");
        let (validation, fallback) = revalidate(&prepared, &result, digest.as_deref(), false);
        assert_eq!(validation, "ineligible");
        assert_eq!(fallback.as_deref(), Some("local_slm"));
        assert_eq!(stage_name(&result, &validation), "fallback");
        assert_eq!(
            outcome_stub_from_text("refuse:no-lease: no hop lease for 'ttl-hop'"),
            "refuse:no-lease"
        );
    }

    #[test]
    fn stale_digest_records_fallback_and_expiry_wins() {
        let mut estate = example();
        allow_local(&mut estate, "research");
        let prepared = prepare_candidates(&estate, Some("research"));
        let hint = SelectHint {
            select: Some("local_slm".into()),
            digest: Some("0".into()),
        };
        let (result, digest) = select(&prepared, Some(&hint));
        let (validation, fallback) = revalidate(&prepared, &result, digest.as_deref(), false);
        assert_eq!(validation, "stale");
        assert_eq!(fallback.as_deref(), Some("local_slm"));
        let (expired, no_fallback) = revalidate(&prepared, &result, digest.as_deref(), true);
        assert_eq!(expired, "expired");
        assert!(no_fallback.is_none());
        assert_eq!(stage_name(&result, &expired), "validate");
    }

    fn digest_without_params(binding: &ModelBinding) -> String {
        let mut hasher = Sha256::new();
        hasher.update(binding.id.as_bytes());
        hasher.update(b"\n");
        hasher.update(binding.class.as_str().as_bytes());
        hasher.update(b"\n");
        hasher.update(binding.driver.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    #[test]
    fn param_drift_changes_the_digest_and_marks_the_hint_stale() {
        let mut estate = example();
        allow_local(&mut estate, "research");
        let binding = estate
            .model_bindings
            .iter()
            .find(|binding| binding.id == "local_slm")
            .unwrap();
        let with_params = binding_digest(binding);
        assert_ne!(with_params, digest_without_params(binding));

        let mut reversed = binding.clone();
        let mut flipped = serde_json::Map::new();
        let mut keys: Vec<&String> = match &binding.params {
            serde_json::Value::Object(map) => map.keys().collect(),
            other => panic!("local_slm params are an object, got {other}"),
        };
        keys.reverse();
        for key in keys {
            flipped.insert(key.clone(), binding.params[key].clone());
        }
        reversed.params = serde_json::Value::Object(flipped);
        assert_eq!(binding_digest(&reversed), with_params);

        let prepared = prepare_candidates(&estate, Some("research"));
        let hint = SelectHint {
            select: Some("local_slm".into()),
            digest: Some(with_params.clone()),
        };
        let (result, digest) = select(&prepared, Some(&hint));
        let (validation, fallback) = revalidate(&prepared, &result, digest.as_deref(), false);
        assert_eq!(result, "local_slm");
        assert_eq!(validation, "ok");
        assert!(fallback.is_none());

        let binding = estate
            .model_bindings
            .iter_mut()
            .find(|binding| binding.id == "local_slm")
            .unwrap();
        binding.params["model"] = serde_json::json!("drifted-model");
        binding.params["path"] = serde_json::json!("/tmp/drifted");
        let drifted = binding_digest(binding);
        assert_ne!(drifted, with_params);
        let prepared = prepare_candidates(&estate, Some("research"));
        let hint = SelectHint {
            select: Some("local_slm".into()),
            digest: Some(with_params),
        };
        let (result, digest) = select(&prepared, Some(&hint));
        let (validation, fallback) = revalidate(&prepared, &result, digest.as_deref(), false);
        assert_eq!(result, "local_slm");
        assert_eq!(validation, "stale");
        assert_eq!(fallback.as_deref(), Some("local_slm"));
        assert_eq!(stage_name(&result, &validation), "fallback");
    }

    fn push_local(estate: &mut Estate, id: &str) {
        let mut seat = estate
            .model_bindings
            .iter()
            .find(|binding| binding.id == "local_slm")
            .unwrap()
            .clone();
        seat.id = id.into();
        estate.model_bindings.push(seat);
    }

    fn allow_model(estate: &mut Estate, agent: &str, object: &str) {
        estate.intentions.push(Intention {
            subject_agent: agent.into(),
            object: object.into(),
            kind: IntentionKind::Model,
            effect: Effect::Allow,
            note: None,
        });
    }

    #[test]
    fn specialty_seat_is_selected_and_equal_class_still_abstains() {
        let mut scoped = example();
        push_local(&mut scoped, "ag_news");
        push_local(&mut scoped, "policy_precheck");
        estate_schema::validate(&scoped).unwrap();
        scoped
            .agents
            .iter_mut()
            .find(|agent| agent.id == "horizon")
            .unwrap()
            .models
            .push(ModelUseDecl {
                id: "ag_news".into(),
                description: None,
            });
        allow_model(&mut scoped, "horizon", "ag_news");
        let prepared = prepare_candidates(&scoped, Some("horizon"));
        assert_eq!(
            prepared.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
            ["ag_news"]
        );
        assert_eq!(select(&prepared, None).0, "ag_news");

        let mut allow_list = example();
        push_local(&mut allow_list, "ag_news");
        push_local(&mut allow_list, "policy_precheck");
        allow_list
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap()
            .models = vec![ModelUseDecl {
            id: "ag_news".into(),
            description: None,
        }];
        allow_local(&mut allow_list, "research");
        let prepared = prepare_candidates(&allow_list, Some("research"));
        assert_eq!(prepared.len(), 1);
        assert_eq!(select(&prepared, None).0, "ag_news");

        let mut peers = example();
        push_local(&mut peers, "ag_news");
        peers
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap()
            .models
            .push(ModelUseDecl {
                id: "ag_news".into(),
                description: None,
            });
        allow_local(&mut peers, "research");
        let prepared = prepare_candidates(&peers, Some("research"));
        assert_eq!(prepared.len(), 2);
        assert_eq!(select(&prepared, None).0, "abstain");

        let mut open = example();
        push_local(&mut open, "ag_news");
        open.agents
            .iter_mut()
            .find(|agent| agent.id == "horizon")
            .unwrap()
            .models
            .push(ModelUseDecl {
                id: "ag_news".into(),
                description: None,
            });
        allow_local(&mut open, "horizon");
        allow_model(&mut open, "horizon", "class:frontier");
        let prepared = prepare_candidates(&open, Some("horizon"));
        let ids = prepared
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"xai_grok"), "{ids:?}");
        assert!(ids.contains(&"local_slm"), "{ids:?}");
        assert!(ids.contains(&"ag_news"), "{ids:?}");
        assert_eq!(select(&prepared, None).0, "abstain");
    }

    #[test]
    fn authorize_receipt_names_the_surface_and_convey_omits_it() {
        let mut estate = example();
        allow_local(&mut estate, "research");
        let dir =
            std::env::temp_dir().join(format!("cell-decision-surface-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let convey = record_convey_receipt(
            &estate,
            &dir,
            "ttl-hop",
            "lane-tool",
            Some("research"),
            None,
            "allow",
            false,
        )
        .unwrap();
        assert!(convey.surface.is_empty());
        assert_eq!(convey.result, "local_slm");
        let line = serde_json::to_string(&convey).unwrap();
        assert!(!line.contains("surface"), "{line}");

        let auth = record_authorize_receipt(
            &estate,
            &dir,
            "model",
            "local_slm",
            "research",
            None,
            "allow",
        )
        .unwrap();
        assert_eq!(auth.surface, "authorize");
        assert_eq!(auth.hop_id, "model");
        assert_eq!(auth.capability, "local_slm");
        assert_eq!(auth.agent.as_deref(), Some("research"));
        assert_eq!(auth.result, "local_slm");
        assert_eq!(auth.validation, "ok");
        assert_eq!(auth.stage, "validate");
        assert_eq!(auth.seq, 2);
        assert!(auth.fallback.is_none());
        let line = serde_json::to_string(&auth).unwrap();
        assert!(line.contains("\"surface\":\"authorize\""), "{line}");

        let complete = record_complete_receipt(
            &estate,
            &dir,
            "local_slm",
            "research",
            None,
            "allow",
        )
        .unwrap();
        assert_eq!(complete.surface, "complete");
        assert_eq!(complete.hop_id, "model");
        assert_eq!(complete.capability, "local_slm");
        assert_eq!(complete.agent.as_deref(), Some("research"));
        assert_eq!(complete.result, "local_slm");
        assert_eq!(complete.validation, "ok");
        assert_eq!(complete.stage, "validate");
        assert_eq!(complete.seq, 3);
        assert!(complete.fallback.is_none());
        let line = serde_json::to_string(&complete).unwrap();
        assert!(line.contains("\"surface\":\"complete\""), "{line}");
        assert_eq!(
            complete_driver_outcome(&model_estate::ModelError::MissingEndpoint(
                "CELL_LOCAL_ENDPOINT".into()
            )),
            "refuse:missing-endpoint"
        );
        assert_eq!(
            complete_driver_outcome(&model_estate::ModelError::MissingFrontierKey),
            "refuse:missing-creds"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
