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
use estate_schema::{authorize, AccessRequest, Estate, IntentionKind, ModelBinding, ModelClass};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) const RECEIPT_SCHEMA: &str = "cell-one.decision-receipt.v0";
pub(crate) const SELECT_SCHEMA: &str = "cell-one.decision-select.v0";
pub(crate) const SELECT_FILE: &str = "decision-select.json";
pub(crate) const JOURNAL_FILE: &str = "receipts.jsonl";
pub(crate) const EQUAL_CLASS_SELECT: &str = "equal-class";
pub(crate) const APPLY_RECEIPT_SCHEMA: &str = "cell-one.improvement-apply.v0";
pub(crate) const APPLY_RECEIPT_FILE: &str = "improvement-apply.json";
const REFUSE_WITHOUT_PLAN: &str = "refuse:plan: apply-package requires --require-plan";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MixedSelectPolicy {
    /// Zero or two-or-more eligible ids abstain (existing default).
    #[default]
    Disjoint,
    /// One complete turn may choose a specialty seat among equal-class peers.
    EqualClass,
}

impl MixedSelectPolicy {
    pub(crate) fn parse(raw: &str) -> Result<Self> {
        match estate_schema::normalize_name(raw).as_str() {
            "" => Ok(Self::Disjoint),
            "equal-class" => Ok(Self::EqualClass),
            other => bail!(
                "refuse:decision-select: select policy must be {EQUAL_CLASS_SELECT} (got '{other}')"
            ),
        }
    }

    pub(crate) fn is_equal_class(self) -> bool {
        matches!(self, Self::EqualClass)
    }
}

fn merge_policy(left: MixedSelectPolicy, right: MixedSelectPolicy) -> MixedSelectPolicy {
    if left.is_equal_class() || right.is_equal_class() {
        MixedSelectPolicy::EqualClass
    } else {
        MixedSelectPolicy::Disjoint
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectHint {
    pub select: Option<String>,
    pub digest: Option<String>,
    pub policy: MixedSelectPolicy,
}

impl Default for SelectHint {
    fn default() -> Self {
        Self {
            select: None,
            digest: None,
            policy: MixedSelectPolicy::Disjoint,
        }
    }
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
    /// Agent pack id when `estate complete --pack` journals a handoff.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_id: Option<String>,
    /// Orchestrator (or calling) agent that handed off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff_from: Option<String>,
    /// Member agent that received the handoff and completed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff_to: Option<String>,
    /// Pack package id when `estate package run` (or complete --package) stamps a skill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
    /// Standing routine id when `estate routine run` stamps an automation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routine_id: Option<String>,
    /// Shared id for a multi-hop package/pack chain. Human-legible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,
    /// Ordered handoffs for a chain. Reuses pack handoff field names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handoffs: Vec<HandoffHop>,
    pub stage: String,
    pub candidates: Vec<CandidateSummary>,
    pub result: String,
    pub validation: String,
    pub fallback: Option<String>,
    pub outcome: String,
    /// Opt-in category label from `estate complete` when a codec decoded the letter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_label: Option<String>,
    /// Peer candidates considered and not chosen on equal-class mixed select.
    /// Empty stays off the wire so existing receipts keep their shape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rejected: Vec<CandidateSummary>,
    /// Pack-scoped crew session when `estate complete --session` is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Turn count on that session after this receipt (or current on refuse).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_turns: Option<u64>,
    /// True when prior session turns were prepended into this complete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_context: Option<bool>,
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

fn is_specialty_local(estate: &Estate, binding_id: &str) -> bool {
    let want = estate_schema::normalize_name(binding_id);
    if want == estate_schema::normalize_name("local_slm") {
        return false;
    }
    estate.model_bindings.iter().any(|binding| {
        estate_schema::normalize_name(&binding.id) == want && binding.class == ModelClass::Local
    })
}

fn agent_select_policy(estate: &Estate, agent: Option<&str>) -> MixedSelectPolicy {
    let Some(agent_id) = agent else {
        return MixedSelectPolicy::Disjoint;
    };
    let Some(row) = estate.agent(agent_id) else {
        return MixedSelectPolicy::Disjoint;
    };
    match row.select.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => MixedSelectPolicy::parse(raw).unwrap_or(MixedSelectPolicy::Disjoint),
        None => MixedSelectPolicy::Disjoint,
    }
}

/// One eligible id is selected. Zero or more than one abstains unless
/// equal-class mixed select is opted in: then exactly one specialty local
/// among mixed peers is chosen. A hint may name an id. That name is not a grant.
fn select(
    prepared: &[Prepared],
    hint: Option<&SelectHint>,
    policy: MixedSelectPolicy,
    estate: &Estate,
) -> (String, Option<String>) {
    let effective = merge_policy(policy, hint.map(|h| h.policy).unwrap_or_default());
    if let Some(hint) = hint {
        if let Some(id) = &hint.select {
            return (id.clone(), hint.digest.clone());
        }
        if !effective.is_equal_class() {
            return ("abstain".into(), None);
        }
    }
    match prepared {
        [] => ("abstain".into(), None),
        [one] => (one.id.clone(), Some(one.digest.clone())),
        many if effective.is_equal_class() => {
            let specialties: Vec<&Prepared> = many
                .iter()
                .filter(|row| is_specialty_local(estate, &row.id))
                .collect();
            match specialties.as_slice() {
                [one] => (one.id.clone(), Some(one.digest.clone())),
                _ => ("abstain".into(), None),
            }
        }
        _ => ("abstain".into(), None),
    }
}

fn rejected_peers(prepared: &[Prepared], result: &str) -> Vec<CandidateSummary> {
    if result == "abstain" || prepared.len() < 2 {
        return Vec::new();
    }
    prepared
        .iter()
        .filter(|row| row.id != result)
        .map(|row| CandidateSummary { id: row.id.clone() })
        .collect()
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
    let policy = agent_select_policy(estate, agent);
    let (result, claimed) = select(&prepared, hint, policy, estate);
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
    let mut policy = MixedSelectPolicy::Disjoint;
    if let Some(raw) = value.get("policy").and_then(|v| v.as_str()) {
        policy = MixedSelectPolicy::parse(raw)?;
    }
    let select = match value.get("select") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(raw)) if raw == "abstain" => None,
        Some(serde_json::Value::String(raw)) if estate_schema::normalize_name(raw) == EQUAL_CLASS_SELECT => {
            policy = MixedSelectPolicy::EqualClass;
            None
        }
        Some(serde_json::Value::String(raw)) if !raw.is_empty() => Some(raw.clone()),
        _ => bail!("refuse:decision-select: select must be an id, abstain, equal-class, or null"),
    };
    let digest = match value.get("digest") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(raw)) if !raw.is_empty() => Some(raw.clone()),
        _ => bail!("refuse:decision-select: digest must be a non-empty string or null"),
    };
    Ok(Some(SelectHint {
        select,
        digest,
        policy,
    }))
}

/// Merge a CLI `--select equal-class` (or documented flag) into the hint.
pub(crate) fn apply_select_policy(hint: &mut Option<SelectHint>, raw: Option<&str>) -> Result<()> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let policy = MixedSelectPolicy::parse(raw)?;
    match hint {
        Some(existing) => existing.policy = merge_policy(existing.policy, policy),
        None => {
            *hint = Some(SelectHint {
                select: None,
                digest: None,
                policy,
            });
        }
    }
    Ok(())
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
        estate, state_dir, hop_id, capability, agent, hint, outcome, expired, "", None, None,
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
        None,
        None,
    )
}

/// Estate-bound complete. `hop_id` is `model` and `capability` is the
/// binding that complete targeted. `surface` is `complete`. No hop-lease
/// clock: `expired` stays false. The same host prepare / select /
/// revalidate path as authorize. The selector does not grant.
/// One hop in an ordered chain. Same names as a single-hop receipt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct HandoffHop {
    pub handoff_from: String,
    pub handoff_to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackHandoff {
    pub pack_id: String,
    pub handoff_from: String,
    pub handoff_to: String,
    pub package_id: Option<String>,
    pub routine_id: Option<String>,
    pub chain_id: Option<String>,
    pub handoffs: Vec<HandoffHop>,
}

impl PackHandoff {
    pub(crate) fn single(pack_id: String, handoff_from: String, handoff_to: String) -> Self {
        Self {
            pack_id,
            handoff_from,
            handoff_to,
            package_id: None,
            routine_id: None,
            chain_id: None,
            handoffs: Vec::new(),
        }
    }
}

#[allow(dead_code)]
pub(crate) fn record_complete_receipt(
    estate: &Estate,
    state_dir: &Path,
    object: &str,
    agent: &str,
    hint: Option<&SelectHint>,
    outcome: &str,
) -> Result<DecisionReceipt> {
    record_complete_receipt_pack(estate, state_dir, object, agent, hint, outcome, None, None)
}

pub(crate) fn record_complete_receipt_pack(
    estate: &Estate,
    state_dir: &Path,
    object: &str,
    agent: &str,
    hint: Option<&SelectHint>,
    outcome: &str,
    pack: Option<&PackHandoff>,
    session: Option<&crate::pack_session::SessionStamp>,
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
        pack,
        session,
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
    pack: Option<&PackHandoff>,
    session: Option<&crate::pack_session::SessionStamp>,
) -> Result<DecisionReceipt> {
    let prepared = prepare_candidates(estate, agent);
    let policy = agent_select_policy(estate, agent);
    let (result, claimed) = select(&prepared, hint, policy, estate);
    let (validation, fallback) = revalidate(&prepared, &result, claimed.as_deref(), expired);
    let stage = stage_name(&result, &validation).to_string();
    let candidates = prepared
        .iter()
        .map(|row| CandidateSummary { id: row.id.clone() })
        .collect::<Vec<_>>();
    let rejected = rejected_peers(&prepared, &result);
    let dir = state_dir.join("decisions");
    fs::create_dir_all(&dir)?;
    let path = dir.join(JOURNAL_FILE);
    let seq = next_seq(&path)?;
    let candidate_ids = candidates
        .iter()
        .map(|row| row.id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let rejected_ids = rejected
        .iter()
        .map(|row| row.id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let mut material = format!(
        "{hop_id}\n{capability}\n{}\n{candidate_ids}\n{result}\n{rejected_ids}\n{validation}\n{}\n{outcome}\n{stage}",
        agent.unwrap_or(""),
        fallback.as_deref().unwrap_or("")
    );
    if !surface.is_empty() {
        material.push('\n');
        material.push_str(surface);
    }
    if let Some(pack) = pack {
        material.push('\n');
        material.push_str(&pack.pack_id);
        material.push('\n');
        material.push_str(&pack.handoff_from);
        material.push('\n');
        material.push_str(&pack.handoff_to);
        if let Some(pkg) = &pack.package_id {
            material.push('\n');
            material.push_str(pkg);
        }
        if let Some(rid) = &pack.routine_id {
            material.push('\n');
            material.push_str(rid);
        }
        if let Some(cid) = &pack.chain_id {
            material.push('\n');
            material.push_str(cid);
        }
        for hop in &pack.handoffs {
            material.push('\n');
            material.push_str(&hop.handoff_from);
            material.push('>');
            material.push_str(&hop.handoff_to);
            if let Some(b) = &hop.binding {
                material.push(':');
                material.push_str(b);
            }
        }
    }
    if let Some(session) = session {
        material.push('\n');
        material.push_str(&session.session_id);
        material.push('\n');
        material.push_str(&session.session_turns.to_string());
        material.push('\n');
        material.push_str(if session.session_context {
            "applied"
        } else {
            "none"
        });
    }
    let receipt = DecisionReceipt {
        schema: RECEIPT_SCHEMA.into(),
        id: receipt_id(seq, &material),
        seq,
        hop_id: hop_id.to_string(),
        capability: capability.to_string(),
        agent: agent.map(|s| s.to_string()),
        surface: surface.to_string(),
        pack_id: pack.map(|p| p.pack_id.clone()),
        handoff_from: pack.map(|p| p.handoff_from.clone()),
        handoff_to: pack.map(|p| p.handoff_to.clone()),
        package_id: pack.and_then(|p| p.package_id.clone()),
        routine_id: pack.and_then(|p| p.routine_id.clone()),
        chain_id: pack.and_then(|p| p.chain_id.clone()),
        handoffs: pack.map(|p| p.handoffs.clone()).unwrap_or_default(),
        stage,
        candidates,
        result,
        validation,
        fallback,
        outcome: outcome.to_string(),
        completion_label: None,
        rejected,
        session_id: session.map(|s| s.session_id.clone()),
        session_turns: session.map(|s| s.session_turns),
        session_context: session.map(|s| s.session_context),
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
    let mut line = match &receipt.fallback {
        Some(id) => format!(
            "decision receipt: {} result={} validation={} fallback={id}",
            receipt.id, receipt.result, receipt.validation
        ),
        None => format!(
            "decision receipt: {} result={} validation={}",
            receipt.id, receipt.result, receipt.validation
        ),
    };
    if let Some(sid) = &receipt.session_id {
        let turns = receipt.session_turns.unwrap_or(0);
        let context = if receipt.session_context.unwrap_or(false) {
            "applied"
        } else {
            "none"
        };
        line.push_str(&format!(" session={sid} turns={turns} context={context}"));
    }
    line
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
    let mut surface_authorize = 0usize;
    let mut surface_convey = 0usize;
    let mut surface_complete = 0usize;
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
        match receipt.surface.as_str() {
            "authorize" => surface_authorize += 1,
            "complete" => surface_complete += 1,
            "" => surface_convey += 1,
            _ => {}
        }
        match &receipt.fallback {
            Some(id) => *by_id.entry(id.as_str()).or_default() += 1,
            None => none += 1,
        }
    }
    let n = receipts.len();
    let mut out = format!(
        "decision receipts: {n}\nstage prepare={n} select={select} validate={validate} fallback={fallback_stage}\nvalidation ok={ok} stale={stale} ineligible={ineligible} expired={expired}\nsurface authorize={surface_authorize} convey={surface_convey} complete={surface_complete}\nfallback none={none}\n"
    );
    for (id, count) in by_id {
        out.push_str(&format!("fallback {id}={count}\n"));
    }
    // Recent rows name agent + capability + surface for multi-agent journals.
    const RECENT: usize = 8;
    if !receipts.is_empty() {
        out.push_str("recent:\n");
        let start = receipts.len().saturating_sub(RECENT);
        for receipt in &receipts[start..] {
            let agent = receipt.agent.as_deref().unwrap_or("-");
            let surface = if receipt.surface.is_empty() {
                "convey"
            } else {
                receipt.surface.as_str()
            };
            let pack = match &receipt.pack_id {
                Some(id) => format!(" pack={id}"),
                None => String::new(),
            };
            let handoff = match (&receipt.handoff_from, &receipt.handoff_to) {
                (Some(frm), Some(to)) => format!(" handoff={frm}->{to}"),
                _ => String::new(),
            };
            let package = match &receipt.package_id {
                Some(id) => format!(" package={id}"),
                None => String::new(),
            };
            let routine = match &receipt.routine_id {
                Some(id) => format!(" routine={id}"),
                None => String::new(),
            };
            let chain = match &receipt.chain_id {
                Some(id) => format!(" chain={id}"),
                None => String::new(),
            };
            let session = match &receipt.session_id {
                Some(id) => {
                    let turns = receipt.session_turns.unwrap_or(0);
                    let context = if receipt.session_context.unwrap_or(false) {
                        "applied"
                    } else {
                        "none"
                    };
                    format!(" session={id} turns={turns} context={context}")
                }
                None => String::new(),
            };
            let rejected = if receipt.rejected.is_empty() {
                String::new()
            } else {
                let ids = receipt
                    .rejected
                    .iter()
                    .map(|row| row.id.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                format!(" rejected={ids}")
            };
            out.push_str(&format!(
                "  {id} agent={agent} capability={cap} surface={surface}{pack}{handoff}{package}{routine}{chain}{session} result={result}{rejected} outcome={outcome}\n",
                id = receipt.id,
                cap = receipt.capability,
                result = receipt.result,
                outcome = receipt.outcome,
            ));
        }
    }
    out
}

/// Nearby gated-apply receipt, if the lab already wrote one.
///
/// Looks beside the decision journal (`{state-dir}/decisions/improvement-apply.json`),
/// then at the throwaway apply lab (`{state-dir}/../apply/improvement-apply.json`
/// and `{state-dir}/../improvement-apply.json`). Missing or a foreign schema
/// is silence — this is a cite, not a grant.
pub(crate) fn discover_apply_receipt(state_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        state_dir.join("..").join("apply").join(APPLY_RECEIPT_FILE),
        state_dir.join("..").join(APPLY_RECEIPT_FILE),
        state_dir.join("decisions").join(APPLY_RECEIPT_FILE),
        state_dir.join(APPLY_RECEIPT_FILE),
    ];
    for path in candidates {
        let Ok(canon) = path.canonicalize() else {
            continue;
        };
        if !canon.is_file() {
            continue;
        }
        if load_apply_receipt(&canon).is_some() {
            return Some(canon);
        }
    }
    None
}

fn load_apply_receipt(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    let receipt: Value = serde_json::from_str(&text).ok()?;
    (receipt.get("schema").and_then(Value::as_str) == Some(APPLY_RECEIPT_SCHEMA)).then_some(receipt)
}

fn receipt_str<'a>(receipt: &'a Value, keys: &[&str]) -> &'a str {
    keys.iter()
        .find_map(|key| receipt.get(*key).and_then(Value::as_str))
        .unwrap_or("-")
}

/// One operator line for a `cell-one.improvement-apply.v0` receipt.
pub(crate) fn render_apply_receipt_cite(path: &Path, receipt: &Value) -> String {
    let proposal_id = receipt_str(receipt, &["proposal_id", "applied_proposal_id"]);
    let kind = receipt_str(receipt, &["proposal_kind", "applied_proposal_kind"]);
    let binding = receipt_str(receipt, &["binding_id"]);
    let standing = receipt_str(receipt, &["standing"]);
    let standing = if standing == "-" {
        "joinable: yes"
    } else {
        standing
    };
    let refuse = receipt
        .get("refuse_without_plan")
        .and_then(Value::as_str)
        .unwrap_or(REFUSE_WITHOUT_PLAN);
    format!(
        "apply receipt: schema={APPLY_RECEIPT_SCHEMA} path={path}\napplied proposal {proposal_id} kind={kind} binding={binding} standing={standing} require_plan=true refuse_without_plan={refuse} auto_train=false train_invoked=false\n",
        path = path.display()
    )
}

/// Cite a nearby apply receipt when the lab has one. None if absent.
pub(crate) fn cite_nearby_apply_receipt(state_dir: &Path) -> Option<String> {
    let path = discover_apply_receipt(state_dir)?;
    let receipt = load_apply_receipt(&path)?;
    Some(render_apply_receipt_cite(&path, &receipt))
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

pub(crate) fn cmd_decisions_report(state_dir: &Path, pack: Option<&str>) -> Result<()> {
    let mut receipts = load_receipts(state_dir)?;
    if let Some(pack_id) = pack {
        let want = estate_schema::normalize_name(pack_id);
        receipts.retain(|r| {
            r.pack_id
                .as_deref()
                .map(|id| estate_schema::normalize_name(id) == want)
                .unwrap_or(false)
        });
    }
    print!("{}", render_report(&receipts));
    if let Some(cite) = cite_nearby_apply_receipt(state_dir) {
        print!("{cite}");
    }
    Ok(())
}

/// Resolve pack handoff under pack policy.
/// `--agent` must be the pack orchestrator when set, else a member.
/// Handoff candidates are members other than the calling agent.
/// One candidate selects that member. Multiple need a select hint naming
/// a member id. Zero candidates refuse. Opt-in mixed select without
/// `--pack` is `--select equal-class` / agent `select: equal-class`.
pub(crate) fn resolve_pack_handoff(
    estate: &Estate,
    pack_id: &str,
    agent: &str,
    hint: Option<&SelectHint>,
) -> Result<PackHandoff, String> {
    let pack = estate
        .pack(pack_id)
        .ok_or_else(|| format!("refuse:unknown-pack: pack '{pack_id}' not on estate"))?;
    let agent_n = estate_schema::normalize_name(agent);
    if let Some(orch) = &pack.orchestrator {
        if estate_schema::normalize_name(orch) != agent_n {
            return Err(format!(
                "refuse:pack-orchestrator: --agent '{agent}' is not orchestrator '{orch}' for pack '{}'",
                pack.id
            ));
        }
    } else if !pack
        .members
        .iter()
        .any(|m| estate_schema::normalize_name(m) == agent_n)
    {
        return Err(format!(
            "refuse:pack-member: --agent '{agent}' is not a member of pack '{}'",
            pack.id
        ));
    }
    let candidates: Vec<&str> = pack
        .members
        .iter()
        .map(String::as_str)
        .filter(|m| estate_schema::normalize_name(m) != agent_n)
        .collect();
    let handoff_to = match candidates.as_slice() {
        [] => {
            return Err(format!(
                "refuse:pack-empty-handoff: pack '{}' has no member to hand off to from '{agent}'",
                pack.id
            ));
        }
        [only] => (*only).to_string(),
        many => {
            let claimed = hint.and_then(|h| h.select.as_deref());
            match claimed {
                Some(sel)
                    if many
                        .iter()
                        .any(|m| estate_schema::normalize_name(m) == estate_schema::normalize_name(sel)) =>
                {
                    // Prefer the estate spelling.
                    many
                        .iter()
                        .find(|m| {
                            estate_schema::normalize_name(m) == estate_schema::normalize_name(sel)
                        })
                        .map(|m| (*m).to_string())
                        .unwrap()
                }
                Some(_) => {
                    return Err(format!(
                        "refuse:decision-ineligible: select hint is not a handoff member of pack '{}'",
                        pack.id
                    ));
                }
                None => {
                    return Err(format!(
                        "refuse:decision-abstain: pack '{}' has {} handoff members; set decision-select.json or use a one-member pack",
                        pack.id,
                        many.len()
                    ));
                }
            }
        }
    };
    Ok(PackHandoff::single(pack.id.clone(), agent.to_string(), handoff_to))
}

/// Resolve a pack package (skill) and attach it to a pack handoff.
/// `--agent` defaults to the pack orchestrator when set; otherwise required.
/// Prompt comes from the package unless the caller overrides later.
pub(crate) fn resolve_package_handoff(
    estate: &Estate,
    package_id: &str,
    agent: Option<&str>,
    hint: Option<&SelectHint>,
) -> Result<(PackHandoff, String, Option<String>, Option<String>), String> {
    let pkg = estate.pack_package(package_id).ok_or_else(|| {
        format!("refuse:unknown-package: package '{package_id}' not on estate")
    })?;
    let pack = estate.pack(&pkg.pack).ok_or_else(|| {
        format!(
            "refuse:unknown-pack: pack '{}' for package '{}' not on estate",
            pkg.pack, pkg.id
        )
    })?;
    let agent = match agent {
        Some(a) => a.to_string(),
        None => pack
            .orchestrator
            .clone()
            .ok_or_else(|| {
                format!(
                    "refuse:package-agent: package '{}' pack '{}' has no orchestrator; set --agent",
                    pkg.id, pack.id
                )
            })?,
    };
    let mut handoff = resolve_pack_handoff(estate, &pack.id, &agent, hint)?;
    handoff.package_id = Some(pkg.id.clone());
    Ok((
        handoff,
        agent,
        pkg.prompt.clone(),
        pkg.binding.clone(),
    ))
}

/// Resolve a standing routine → pack package handoff.
pub(crate) fn resolve_routine_handoff(
    estate: &Estate,
    routine_id: &str,
    agent: Option<&str>,
    hint: Option<&SelectHint>,
) -> Result<(PackHandoff, String, Option<String>, Option<String>), String> {
    let routine = estate.routine(routine_id).ok_or_else(|| {
        format!("refuse:unknown-routine: routine '{routine_id}' not on estate")
    })?;
    let (mut handoff, agent, prompt, binding) =
        resolve_package_handoff(estate, &routine.package, agent, hint)?;
    handoff.routine_id = Some(routine.id.clone());
    Ok((handoff, agent, prompt, binding))
}

/// Resolve an ordered package/pack chain into per-hop handoffs sharing `chain_id`.
/// First hop is from the orchestrator (or `--agent`); later hops are from the
/// previous hop's agent. Each hop completes as `handoff_to` on that binding.
pub(crate) fn resolve_package_chain(
    estate: &Estate,
    package_id: &str,
    agent: Option<&str>,
    chain_id: &str,
    routine_id: Option<&str>,
) -> Result<(Vec<PackHandoff>, String, Option<String>), String> {
    let pkg = estate.pack_package(package_id).ok_or_else(|| {
        format!("refuse:unknown-package: package '{package_id}' not on estate")
    })?;
    let pack = estate.pack(&pkg.pack).ok_or_else(|| {
        format!(
            "refuse:unknown-pack: pack '{}' for package '{}' not on estate",
            pkg.pack, pkg.id
        )
    })?;
    let hops = pkg.resolved_chain(pack);
    if hops.len() < 2 {
        return Err(format!(
            "refuse:no-chain: package '{}' (pack '{}') has no chain:/steps: with two or more hops",
            pkg.id, pack.id
        ));
    }
    let caller = match agent {
        Some(a) => a.to_string(),
        None => pack.orchestrator.clone().ok_or_else(|| {
            format!(
                "refuse:package-agent: package '{}' pack '{}' has no orchestrator; set --agent",
                pkg.id, pack.id
            )
        })?,
    };
    let caller_n = estate_schema::normalize_name(&caller);
    if let Some(orch) = &pack.orchestrator {
        if estate_schema::normalize_name(orch) != caller_n {
            return Err(format!(
                "refuse:pack-orchestrator: --agent '{caller}' is not orchestrator '{orch}' for pack '{}'",
                pack.id
            ));
        }
    } else if !pack
        .members
        .iter()
        .any(|m| estate_schema::normalize_name(m) == caller_n)
    {
        return Err(format!(
            "refuse:pack-member: --agent '{caller}' is not a member of pack '{}'",
            pack.id
        ));
    }
    let mut ordered = Vec::new();
    let mut from = caller.clone();
    for hop in hops {
        ordered.push(HandoffHop {
            handoff_from: from.clone(),
            handoff_to: hop.agent.clone(),
            binding: Some(hop.binding.clone()),
        });
        from = hop.agent.clone();
    }
    let mut out = Vec::new();
    for hop in &ordered {
        out.push(PackHandoff {
            pack_id: pack.id.clone(),
            handoff_from: hop.handoff_from.clone(),
            handoff_to: hop.handoff_to.clone(),
            package_id: Some(pkg.id.clone()),
            routine_id: routine_id.map(|s| s.to_string()),
            chain_id: Some(chain_id.to_string()),
            handoffs: ordered.clone(),
        });
    }
    Ok((out, caller, pkg.prompt.clone()))
}

pub(crate) fn mint_chain_id(package_id: &str, now_unix: i64) -> String {
    format!("chain-{package_id}-{now_unix}")
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
        let (result, digest) = select(&prepared, None, MixedSelectPolicy::Disjoint, &estate);
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
        let (result, digest) = select(&prepared, None, MixedSelectPolicy::Disjoint, &estate);
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
        let (result, _) = select(&prepared, None, MixedSelectPolicy::Disjoint, &estate);
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
            policy: MixedSelectPolicy::Disjoint,
        };
        let (result, digest) = select(&prepared, Some(&hint), MixedSelectPolicy::Disjoint, &estate);
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
            policy: MixedSelectPolicy::Disjoint,
        };
        let (result, digest) = select(&prepared, Some(&hint), MixedSelectPolicy::Disjoint, &estate);
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
            policy: MixedSelectPolicy::Disjoint,
        };
        let (result, digest) = select(&prepared, Some(&hint), MixedSelectPolicy::Disjoint, &estate);
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
            policy: MixedSelectPolicy::Disjoint,
        };
        let (result, digest) = select(&prepared, Some(&hint), MixedSelectPolicy::Disjoint, &estate);
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
        assert_eq!(
            select(&prepared, None, MixedSelectPolicy::Disjoint, &scoped).0,
            "ag_news"
        );

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
        assert_eq!(
            select(&prepared, None, MixedSelectPolicy::Disjoint, &allow_list).0,
            "ag_news"
        );

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
        assert_eq!(
            select(&prepared, None, MixedSelectPolicy::Disjoint, &peers).0,
            "abstain"
        );

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
        assert_eq!(
            select(&prepared, None, MixedSelectPolicy::Disjoint, &open).0,
            "abstain"
        );
        assert_eq!(
            select(&prepared, None, MixedSelectPolicy::EqualClass, &open).0,
            "ag_news"
        );
        assert_eq!(
            rejected_peers(&prepared, "ag_news")
                .iter()
                .map(|row| row.id.as_str())
                .collect::<Vec<_>>(),
            ["xai_grok", "local_slm"]
        );
    }

    #[test]
    fn equal_class_mixed_select_chooses_specialty_and_names_rejected_peers() {
        let mut estate = example();
        push_local(&mut estate, "ag_news");
        estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap()
            .models = vec![
            ModelUseDecl {
                id: "ag_news".into(),
                description: None,
            },
            ModelUseDecl {
                id: "xai_grok".into(),
                description: None,
            },
        ];
        estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap()
            .select = Some(EQUAL_CLASS_SELECT.into());
        allow_local(&mut estate, "research");
        estate.intentions.push(Intention {
            subject_agent: "research".into(),
            object: "class:frontier".into(),
            kind: IntentionKind::Model,
            effect: Effect::Allow,
            note: None,
        });
        estate_schema::validate(&estate).unwrap();

        let prepared = prepare_candidates(&estate, Some("research"));
        assert_eq!(
            prepared.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
            ["xai_grok", "ag_news"]
        );
        assert_eq!(
            select(&prepared, None, MixedSelectPolicy::Disjoint, &estate).0,
            "abstain"
        );
        let selection = resolve_selection(&estate, Some("research"), None, false);
        assert_eq!(selection.result, "ag_news");
        assert_eq!(selection.validation, "ok");
        assert!(selection.fallback.is_none());

        let dir = std::env::temp_dir().join(format!(
            "cell-equal-class-mixed-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let receipt = record_complete_receipt(
            &estate,
            &dir,
            "ag_news",
            "research",
            None,
            "allow",
        )
        .unwrap();
        assert_eq!(receipt.result, "ag_news");
        assert_eq!(
            receipt
                .rejected
                .iter()
                .map(|row| row.id.as_str())
                .collect::<Vec<_>>(),
            ["xai_grok"]
        );
        let line = serde_json::to_string(&receipt).unwrap();
        assert!(line.contains("\"rejected\""), "{line}");
        assert!(line.contains("xai_grok"), "{line}");

        estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap()
            .select = None;
        let without = resolve_selection(&estate, Some("research"), None, false);
        assert_eq!(without.result, "abstain");
        let hint = SelectHint {
            select: None,
            digest: None,
            policy: MixedSelectPolicy::EqualClass,
        };
        let with_hint = resolve_selection(&estate, Some("research"), Some(&hint), false);
        assert_eq!(with_hint.result, "ag_news");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn report_counts_authorize_and_convey_surfaces() {
        let estate = example();
        let dir = std::env::temp_dir().join(format!(
            "cell-decision-surface-counts-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        record_authorize_receipt(
            &estate,
            &dir,
            "model",
            "local_slm",
            "research",
            None,
            "allow",
        )
        .unwrap();
        record_convey_receipt(
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
        let text = render_report(&load_receipts(&dir).unwrap());
        assert!(
            text.contains("surface authorize=1 convey=1 complete=0\n"),
            "{text}"
        );
        assert!(text.contains("surface=authorize"), "{text}");
        assert!(text.contains("surface=convey"), "{text}");
        assert!(
            !text.split_whitespace().any(|word| word == "enforced"),
            "{text}"
        );
        assert!(cite_nearby_apply_receipt(&dir).is_none(), "{dir:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn write_apply_receipt(path: &std::path::Path, binding: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(
            path,
            format!(
                r#"{{
  "schema": "cell-one.improvement-apply.v0",
  "proposal_id": "specialty-seat:{binding}",
  "proposal_kind": "specialty-seat",
  "binding_id": "{binding}",
  "joinable": true,
  "standing": "joinable: yes",
  "require_plan": true,
  "refuse_without_plan": "refuse:plan: apply-package requires --require-plan",
  "auto_train": false,
  "train_invoked": false
}}
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn decisions_report_cites_nearby_apply_receipt() {
        let dir = std::env::temp_dir().join(format!(
            "cell-decision-apply-cite-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("decisions")).unwrap();
        write_apply_receipt(
            &dir.join("decisions").join("improvement-apply.json"),
            "ag_news",
        );
        let cite = cite_nearby_apply_receipt(&dir).expect("apply cite");
        assert!(cite.contains("schema=cell-one.improvement-apply.v0"), "{cite}");
        assert!(cite.contains("applied proposal specialty-seat:ag_news"), "{cite}");
        assert!(cite.contains("kind=specialty-seat"), "{cite}");
        assert!(cite.contains("binding=ag_news"), "{cite}");
        assert!(cite.contains("standing=joinable: yes"), "{cite}");
        assert!(cite.contains("require_plan=true"), "{cite}");
        assert!(
            cite.contains("refuse_without_plan=refuse:plan: apply-package requires --require-plan"),
            "{cite}"
        );
        assert!(cite.contains("auto_train=false"), "{cite}");
        assert!(cite.contains("train_invoked=false"), "{cite}");
        assert!(cite.contains("path="), "{cite}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn decisions_report_discovers_sibling_apply_lab_receipt() {
        let root = std::env::temp_dir().join(format!(
            "cell-decision-apply-sibling-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let state = root.join("state");
        std::fs::create_dir_all(&state).unwrap();
        write_apply_receipt(
            &root.join("apply").join("improvement-apply.json"),
            "ag_news",
        );
        let path = discover_apply_receipt(&state).expect("sibling apply receipt");
        assert!(
            path.ends_with("apply/improvement-apply.json"),
            "{}",
            path.display()
        );
        let cite = cite_nearby_apply_receipt(&state).expect("apply cite");
        assert!(cite.contains("specialty-seat:ag_news"), "{cite}");
        assert!(cite.contains("cell-one.improvement-apply.v0"), "{cite}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn decisions_report_ignores_foreign_apply_schema() {
        let dir = std::env::temp_dir().join(format!(
            "cell-decision-apply-foreign-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("decisions")).unwrap();
        std::fs::write(
            dir.join("decisions").join("improvement-apply.json"),
            r#"{"schema":"not-an-apply-receipt","proposal_id":"specialty-seat:ag_news"}"#,
        )
        .unwrap();
        assert!(discover_apply_receipt(&dir).is_none());
        assert!(cite_nearby_apply_receipt(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
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
