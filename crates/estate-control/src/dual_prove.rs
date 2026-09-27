//! `estate package dual-prove` — mock dual-specialty pack chain.
//!
//! One shot: two `class: local` purpose seats (`ag_news`, `rust_idiom`)
//! then frontier, under existing intentions. `--mock` stays in-process.
//! No network. No Ollama. No live PASS. `READY_FOR_LIVE_TEST` stays no.
//! Equal-class ranking stays off: zero or two-or-more eligible ids abstain
//! unless `--select equal-class` / agent `select: equal-class`, and two
//! specialty locals still abstain under that flag.

use anyhow::{bail, Context, Result};
use estate_schema::{
    authorize, AccessRequest, Effect, Estate, Intention, IntentionKind, ModelClass, ModelUseDecl,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use crate::decisions::{
    resolve_package_chain, resolve_package_handoff, resolve_selection, MixedSelectPolicy,
    SelectHint,
};
use crate::pack_session::{self, MAX_TURNS_ENV};

pub(crate) const PROVE_SCHEMA: &str = "cell-one.dual-specialty-prove.v0";
#[allow(dead_code)]
pub(crate) const DEFAULT_PACKAGE: &str = "dual-specialty";
#[allow(dead_code)]
pub(crate) const DEFAULT_ESTATE: &str = "examples/fixtures/agent-pack-handoff.yaml";
#[allow(dead_code)]
pub(crate) const HOP_TOKEN: &str = "dual-specialty-hop-alpha-token";

/// Ordered hops: specialty, specialty, frontier. Agent is `handoff_to`.
const HOPS: &[(&str, &str)] = &[
    ("research", "ag_news"),
    ("idiom", "rust_idiom"),
    ("horizon", "frontier_http"),
];

const SEATS: &[(&str, &str)] = &[
    ("ag_news", "specialist-agnews-all"),
    ("rust_idiom", "specialist-rustidiom-all"),
];

const SACRED: &[&str] = &["cyera-ci", "rust-classroom"];

#[derive(Debug, Clone)]
struct ProveCheck {
    name: String,
    ok: bool,
    detail: String,
}

#[derive(Debug, Clone)]
struct ProveReport {
    ok: bool,
    package_id: String,
    pack_id: String,
    chain_id: String,
    session_id: String,
    state_dir: String,
    hops: Vec<Value>,
    checks: Vec<ProveCheck>,
}

impl ProveReport {
    fn to_json(&self) -> Value {
        let mut checks = serde_json::Map::new();
        for c in &self.checks {
            checks.insert(
                c.name.clone(),
                json!({
                    "ok": c.ok,
                    "detail": c.detail,
                }),
            );
        }
        json!({
            "schema": PROVE_SCHEMA,
            "ok": self.ok,
            "package_id": self.package_id,
            "pack_id": self.pack_id,
            "chain_id": self.chain_id,
            "session_id": self.session_id,
            "state_dir": self.state_dir,
            "hops": self.hops,
            "live_sync": false,
            "ready_for_live_test": false,
            "checks": Value::Object(checks),
        })
    }
}

pub(crate) fn cmd_package_dual_prove(
    package_id: &str,
    estate_path: &Path,
    state_dir: Option<&Path>,
    prompt: &str,
) -> Result<()> {
    let state_dir = match state_dir {
        Some(dir) => dir.to_path_buf(),
        None => throwaway_state(package_id),
    };
    std::fs::create_dir_all(&state_dir)
        .with_context(|| format!("create {}", state_dir.display()))?;

    let mut checks = Vec::new();
    let mut pack_id = String::new();
    let mut chain_id = String::new();
    let mut session_id = String::new();
    let mut hops_json = Vec::new();

    let estate = match estate_schema::load_estate(estate_path) {
        Ok(estate) => estate,
        Err(err) => {
            push_fail(&mut checks, "estate", err.to_string());
            return finish(
                package_id,
                &pack_id,
                &chain_id,
                &session_id,
                &state_dir,
                hops_json,
                checks,
            );
        }
    };

    match structural(&estate, package_id) {
        Ok(id) => {
            pack_id = id;
            push_ok(
                &mut checks,
                "bindings",
                "ag_news + rust_idiom class local, frontier_http frontier, seat names aligned",
            );
        }
        Err(err) => push_fail(&mut checks, "bindings", err.to_string()),
    }

    if !pack_id.is_empty() {
        match disjoint_allow(&estate, package_id) {
            Ok(detail) => push_ok(&mut checks, "disjoint_allow", detail),
            Err(err) => push_fail(&mut checks, "disjoint_allow", err.to_string()),
        }
        match single_hop_abstain(&estate, package_id) {
            Ok(detail) => push_ok(&mut checks, "single_hop_abstain", detail),
            Err(err) => push_fail(&mut checks, "single_hop_abstain", err.to_string()),
        }
        match cross_seat(&estate) {
            Ok(detail) => push_ok(&mut checks, "cross_seat_deny", detail),
            Err(err) => push_fail(&mut checks, "cross_seat_deny", err.to_string()),
        }
        match equal_class_probe(&estate) {
            Ok(detail) => push_ok(&mut checks, "equal_class_abstain", detail),
            Err(err) => push_fail(&mut checks, "equal_class_abstain", err.to_string()),
        }
        match sacred_out(&estate, &pack_id) {
            Ok(detail) => push_ok(&mut checks, "sacred", detail),
            Err(err) => push_fail(&mut checks, "sacred", err.to_string()),
        }
    }

    let structural_ok = checks.iter().all(|c| c.ok);
    if structural_ok {
        session_id = mint_session_id();
        let before = crate::decisions::load_receipts(&state_dir)
            .map(|rows| rows.len())
            .unwrap_or(0);
        match crate::ops::cmd_package_run(
            package_id,
            None,
            Some(prompt.to_string()),
            None,
            estate_path,
            &state_dir,
            None,
            None,
            true,
            true,
            Some(&session_id),
            true,
        ) {
            Ok(()) => match verify_chain(
                &state_dir,
                &pack_id,
                package_id,
                &session_id,
                prompt,
                before,
            ) {
                Ok(found) => {
                    chain_id = found.chain_id.clone();
                    hops_json = found.hops.clone();
                    push_ok(
                        &mut checks,
                        "chain_receipts",
                        format!(
                            "{} hops chain={} bindings={}",
                            found.hops.len(),
                            found.chain_id,
                            found
                                .hops
                                .iter()
                                .filter_map(|h| h.get("binding").and_then(Value::as_str))
                                .collect::<Vec<_>>()
                                .join(",")
                        ),
                    );
                    push_ok(
                        &mut checks,
                        "session_context",
                        format!("hop2 context=applied token seen session={session_id}"),
                    );
                    match decisions_report_names_specialties(&state_dir, &pack_id) {
                        Ok(detail) => push_ok(&mut checks, "decisions_report", detail),
                        Err(err) => push_fail(&mut checks, "decisions_report", err.to_string()),
                    }
                    match session_refuses(
                        package_id,
                        &pack_id,
                        &session_id,
                        estate_path,
                        &state_dir,
                        prompt,
                        before + 3,
                    ) {
                        Ok(detail) => push_ok(&mut checks, "session_refuses", detail),
                        Err(err) => push_fail(&mut checks, "session_refuses", err.to_string()),
                    }
                }
                Err(err) => push_fail(&mut checks, "chain_receipts", err.to_string()),
            },
            Err(err) => push_fail(&mut checks, "chain_receipts", err.to_string()),
        }
    } else {
        push_fail(
            &mut checks,
            "chain_receipts",
            "skipped: fixture checks failed",
        );
    }

    finish(
        package_id,
        &pack_id,
        &chain_id,
        &session_id,
        &state_dir,
        hops_json,
        checks,
    )
}

fn finish(
    package_id: &str,
    pack_id: &str,
    chain_id: &str,
    session_id: &str,
    state_dir: &Path,
    hops: Vec<Value>,
    checks: Vec<ProveCheck>,
) -> Result<()> {
    let ok = !checks.is_empty() && checks.iter().all(|c| c.ok);
    let report = ProveReport {
        ok,
        package_id: package_id.to_string(),
        pack_id: pack_id.to_string(),
        chain_id: chain_id.to_string(),
        session_id: session_id.to_string(),
        state_dir: state_dir.display().to_string(),
        hops,
        checks,
    };
    print_report(&report);
    if !report.ok {
        bail!("refuse:dual-prove: one or more checks failed");
    }
    Ok(())
}

fn print_report(report: &ProveReport) {
    println!("package dual-prove: {}", report.package_id);
    println!("  ok: {}", if report.ok { "yes" } else { "no" });
    println!(
        "  pack: {}",
        if report.pack_id.is_empty() {
            "-"
        } else {
            &report.pack_id
        }
    );
    if !report.chain_id.is_empty() {
        println!("  chain: {}", report.chain_id);
    }
    if !report.session_id.is_empty() {
        println!("  session: {}", report.session_id);
    }
    println!("  state_dir: {}", report.state_dir);
    for hop in &report.hops {
        let agent = hop.get("agent").and_then(Value::as_str).unwrap_or("-");
        let binding = hop.get("binding").and_then(Value::as_str).unwrap_or("-");
        let context = hop.get("context").and_then(Value::as_str).unwrap_or("-");
        println!("  hop: {agent} binding={binding} context={context}");
    }
    for c in &report.checks {
        let mark = if c.ok { "ok" } else { "FAIL" };
        println!("  {}: {mark} ({})", c.name, c.detail);
    }
    println!("  live_sync: no");
    println!("  READY_FOR_LIVE_TEST: no");
    println!("{}", report.to_json());
}

fn throwaway_state(package_id: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("cell-dual-specialty-prove-{package_id}-{nanos}"))
}

fn mint_session_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("sess-dual{:012x}", nanos & 0xFFFF_FFFF_FFFF)
}

fn push_ok(checks: &mut Vec<ProveCheck>, name: &str, detail: impl Into<String>) {
    checks.push(ProveCheck {
        name: name.into(),
        ok: true,
        detail: detail.into(),
    });
}

fn push_fail(checks: &mut Vec<ProveCheck>, name: &str, detail: impl Into<String>) {
    checks.push(ProveCheck {
        name: name.into(),
        ok: false,
        detail: detail.into(),
    });
}

fn structural(estate: &Estate, package_id: &str) -> Result<String> {
    let pkg = estate.pack_package(package_id).ok_or_else(|| {
        anyhow::anyhow!("refuse:unknown-package: package '{package_id}' not on estate")
    })?;
    let pack = estate.pack(&pkg.pack).ok_or_else(|| {
        anyhow::anyhow!(
            "refuse:unknown-pack: pack '{}' for package '{}' not on estate",
            pkg.pack,
            pkg.id
        )
    })?;
    let chain = pkg.resolved_chain(pack);
    if chain.len() != HOPS.len() {
        bail!(
            "refuse:dual-prove: package '{}' chain has {} hops; want {}",
            pkg.id,
            chain.len(),
            HOPS.len()
        );
    }
    for (i, (want_agent, want_binding)) in HOPS.iter().enumerate() {
        let hop = &chain[i];
        if estate_schema::normalize_name(&hop.agent) != estate_schema::normalize_name(want_agent)
            || estate_schema::normalize_name(&hop.binding)
                != estate_schema::normalize_name(want_binding)
        {
            bail!(
                "refuse:dual-prove: hop {i} is {} -> {}; want {want_agent} -> {want_binding}",
                hop.agent,
                hop.binding
            );
        }
    }
    for (id, seat) in SEATS {
        let binding = estate
            .model_bindings
            .iter()
            .find(|b| estate_schema::normalize_name(&b.id) == estate_schema::normalize_name(id))
            .ok_or_else(|| anyhow::anyhow!("refuse:dual-prove: missing binding {id}"))?;
        if binding.class != ModelClass::Local {
            bail!("refuse:dual-prove: {id} must be class local");
        }
        if !binding.wired {
            bail!("refuse:dual-prove: {id} must be wired");
        }
        if estate_schema::contains_sku(&binding.id) {
            bail!("refuse:dual-prove: binding id {id} encodes a hardware SKU");
        }
        let model = binding
            .params
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if model != *seat {
            bail!("refuse:dual-prove: {id} params.model is '{model}'; want {seat}");
        }
        if estate_schema::contains_sku(model) {
            bail!("refuse:dual-prove: {id} params.model encodes a hardware SKU");
        }
        if binding
            .params
            .get("category_codec")
            .and_then(Value::as_str)
            .is_some()
        {
            bail!(
                "refuse:dual-prove: {id} params.category_codec is set; completion_label stays opt-in"
            );
        }
    }
    let frontier = estate
        .model_bindings
        .iter()
        .find(|b| {
            estate_schema::normalize_name(&b.id) == estate_schema::normalize_name("frontier_http")
        })
        .ok_or_else(|| anyhow::anyhow!("refuse:dual-prove: missing frontier_http"))?;
    if frontier.class != ModelClass::Frontier {
        bail!("refuse:dual-prove: frontier_http must be class frontier");
    }
    if estate_schema::normalize_name(&SEATS[0].0) == estate_schema::normalize_name(&SEATS[1].0) {
        bail!("refuse:dual-prove: specialty binding ids must be distinct");
    }
    Ok(pack.id.clone())
}

fn disjoint_allow(estate: &Estate, package_id: &str) -> Result<String> {
    let pkg = estate
        .pack_package(package_id)
        .ok_or_else(|| anyhow::anyhow!("missing package"))?;
    let pack = estate
        .pack(&pkg.pack)
        .ok_or_else(|| anyhow::anyhow!("missing pack"))?;
    for (agent_id, binding) in HOPS {
        let agent = estate
            .agent(agent_id)
            .ok_or_else(|| anyhow::anyhow!("refuse:dual-prove: missing agent {agent_id}"))?;
        if agent
            .select
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_some()
        {
            bail!("refuse:dual-prove: agent {agent_id} sets select; dual chain must stay disjoint");
        }
        if !pack
            .members
            .iter()
            .any(|m| estate_schema::normalize_name(m) == estate_schema::normalize_name(agent_id))
        {
            bail!(
                "refuse:dual-prove: {agent_id} is not a member of pack {}",
                pack.id
            );
        }
        let selection = resolve_selection(estate, Some(agent_id), None, false);
        if selection.result != *binding || selection.validation != "ok" {
            bail!(
                "refuse:dual-prove: {agent_id} eligible select is {} ({}); want {binding} ok",
                selection.result,
                selection.validation
            );
        }
    }
    let (_hops, caller, _) = resolve_package_chain(estate, package_id, None, "prove", None)
        .map_err(|e| anyhow::anyhow!(e))?;
    if estate_schema::normalize_name(&caller) != "horizon" {
        bail!("refuse:dual-prove: chain caller is {caller}; want horizon");
    }
    Ok("each hop agent has one eligible id; no select: equal-class".into())
}

fn single_hop_abstain(estate: &Estate, package_id: &str) -> Result<String> {
    match resolve_package_handoff(estate, package_id, None, None) {
        Err(msg) if msg.contains("refuse:decision-abstain") => {
            Ok("package run without --chain abstains (two handoff members)".into())
        }
        Ok(_) => bail!("refuse:dual-prove: single-hop handoff selected a member"),
        Err(msg) => bail!("refuse:dual-prove: single-hop expected decision-abstain, got {msg}"),
    }
}

fn cross_seat(estate: &Estate) -> Result<String> {
    let allow = [
        ("research", "ag_news"),
        ("idiom", "rust_idiom"),
        ("horizon", "frontier_http"),
    ];
    let deny = [
        ("research", "rust_idiom"),
        ("research", "frontier_http"),
        ("idiom", "ag_news"),
        ("idiom", "frontier_http"),
        ("horizon", "ag_news"),
        ("horizon", "rust_idiom"),
    ];
    for (agent, object) in allow {
        if !model_allowed(estate, agent, object) {
            bail!("refuse:dual-prove: {agent} must allow {object}");
        }
    }
    for (agent, object) in deny {
        if model_allowed(estate, agent, object) {
            bail!("refuse:dual-prove: {agent} must deny {object}");
        }
    }
    Ok("intentions allow each hop seat and deny the other seats".into())
}

fn model_allowed(estate: &Estate, agent: &str, object: &str) -> bool {
    authorize(
        estate,
        &AccessRequest {
            subject_agent: agent,
            kind: IntentionKind::Model,
            object,
        },
    )
    .is_allow()
}

fn equal_class_probe(estate: &Estate) -> Result<String> {
    let mut two = estate.clone();
    grant_model(&mut two, "research", "rust_idiom");
    let plain = resolve_selection(&two, Some("research"), None, false);
    if plain.result != "abstain" {
        bail!(
            "refuse:dual-prove: two specialties without equal-class selected {}",
            plain.result
        );
    }
    let hint = SelectHint {
        select: None,
        digest: None,
        policy: MixedSelectPolicy::EqualClass,
    };
    let ranked = resolve_selection(&two, Some("research"), Some(&hint), false);
    if ranked.result != "abstain" {
        bail!(
            "refuse:dual-prove: two specialties under equal-class selected {} (ranking)",
            ranked.result
        );
    }

    let mut mixed = estate.clone();
    grant_model(&mut mixed, "research", "frontier_http");
    let plain = resolve_selection(&mixed, Some("research"), None, false);
    if plain.result != "abstain" {
        bail!(
            "refuse:dual-prove: specialty+frontier without equal-class selected {}",
            plain.result
        );
    }
    let chosen = resolve_selection(&mixed, Some("research"), Some(&hint), false);
    if chosen.result != "ag_news" {
        bail!(
            "refuse:dual-prove: equal-class one specialty expected ag_news, got {}",
            chosen.result
        );
    }
    Ok(
        "2 specialties abstain with and without equal-class; one specialty + frontier needs the flag"
            .into(),
    )
}

fn grant_model(estate: &mut Estate, agent: &str, binding: &str) {
    if let Some(row) = estate
        .agents
        .iter_mut()
        .find(|row| estate_schema::normalize_name(&row.id) == estate_schema::normalize_name(agent))
    {
        if !row
            .models
            .iter()
            .any(|m| estate_schema::normalize_name(&m.id) == estate_schema::normalize_name(binding))
        {
            row.models.push(ModelUseDecl {
                id: binding.to_string(),
                description: None,
            });
        }
    }
    estate.intentions.push(Intention {
        subject_agent: agent.to_string(),
        object: binding.to_string(),
        kind: IntentionKind::Model,
        effect: Effect::Allow,
        note: None,
    });
}

fn sacred_out(estate: &Estate, pack_id: &str) -> Result<String> {
    let pack = estate
        .pack(pack_id)
        .ok_or_else(|| anyhow::anyhow!("missing pack"))?;
    for id in SACRED {
        let declared = estate
            .sacred_exclusions
            .iter()
            .any(|row| estate_schema::normalize_name(&row.id) == estate_schema::normalize_name(id));
        if !declared {
            bail!("refuse:dual-prove: sacred exclusion {id} is missing");
        }
        if pack.members.iter().any(|m| {
            estate_schema::normalize_name(m) == estate_schema::normalize_name(id)
                || estate_schema::is_sacred_name(m)
        }) {
            bail!("refuse:dual-prove: pack {pack_id} members include a sacred id");
        }
        if estate.agent(id).is_some() {
            bail!("refuse:dual-prove: sacred id {id} is an agent");
        }
    }
    Ok("cyera-ci and rust-classroom stay exclusions".into())
}

struct ChainFound {
    chain_id: String,
    hops: Vec<Value>,
}

fn verify_chain(
    state_dir: &Path,
    pack_id: &str,
    package_id: &str,
    session_id: &str,
    prompt: &str,
    before: usize,
) -> Result<ChainFound> {
    let all = crate::decisions::load_receipts(state_dir)?;
    if all.len() < before + HOPS.len() {
        bail!(
            "refuse:dual-prove: expected {} new receipts, journal has {}",
            HOPS.len(),
            all.len().saturating_sub(before)
        );
    }
    let rows = &all[before..];
    if rows.len() != HOPS.len() {
        bail!(
            "refuse:dual-prove: chain wrote {} receipts; want {}",
            rows.len(),
            HOPS.len()
        );
    }
    let chain_id = rows[0]
        .chain_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("refuse:dual-prove: missing chain_id"))?;
    if !chain_id.starts_with(&format!("chain-{package_id}-")) {
        bail!("refuse:dual-prove: chain_id {chain_id}");
    }
    let mut hops = Vec::new();
    for (i, (agent, binding)) in HOPS.iter().enumerate() {
        let row = &rows[i];
        if row.chain_id.as_deref() != Some(chain_id.as_str()) {
            bail!("refuse:dual-prove: hop {i} chain_id mismatch");
        }
        if row.surface != "complete" {
            bail!("refuse:dual-prove: hop {i} surface {}", row.surface);
        }
        if row.pack_id.as_deref() != Some(pack_id) {
            bail!("refuse:dual-prove: hop {i} pack_id {:?}", row.pack_id);
        }
        if row.package_id.as_deref() != Some(package_id) {
            bail!("refuse:dual-prove: hop {i} package_id {:?}", row.package_id);
        }
        if row.handoff_to.as_deref() != Some(*agent) {
            bail!(
                "refuse:dual-prove: hop {i} handoff_to {:?} want {agent}",
                row.handoff_to
            );
        }
        if row.agent.as_deref() != Some(*agent) {
            bail!(
                "refuse:dual-prove: hop {i} agent {:?} want {agent}",
                row.agent
            );
        }
        if row.capability != *binding || row.result != *binding {
            bail!(
                "refuse:dual-prove: hop {i} capability={} result={} want {binding}",
                row.capability,
                row.result
            );
        }
        if row.outcome != "allow" || row.validation != "ok" {
            bail!(
                "refuse:dual-prove: hop {i} outcome={} validation={}",
                row.outcome,
                row.validation
            );
        }
        if row.candidates.len() != 1 || row.candidates[0].id != *binding {
            bail!(
                "refuse:dual-prove: hop {i} candidates {:?}",
                row.candidates
                    .iter()
                    .map(|c| c.id.as_str())
                    .collect::<Vec<_>>()
            );
        }
        if !row.rejected.is_empty() {
            bail!("refuse:dual-prove: hop {i} recorded rejected peers");
        }
        if row.completion_label.is_some() {
            bail!("refuse:dual-prove: hop {i} stamped completion_label; codec stays opt-in");
        }
        if row.session_id.as_deref() != Some(session_id) {
            bail!(
                "refuse:dual-prove: hop {i} session {:?} want {session_id}",
                row.session_id
            );
        }
        let want_context = i > 0;
        if row.session_context != Some(want_context) {
            bail!(
                "refuse:dual-prove: hop {i} context {:?} want {want_context}",
                row.session_context
            );
        }
        if row.handoffs.len() != HOPS.len() {
            bail!("refuse:dual-prove: hop {i} handoffs {}", row.handoffs.len());
        }
        for (h, (want_agent, want_binding)) in HOPS.iter().enumerate() {
            let hop = &row.handoffs[h];
            if hop.handoff_to != *want_agent || hop.binding.as_deref() != Some(*want_binding) {
                bail!(
                    "refuse:dual-prove: journal handoff {h} is {} -> {:?}",
                    hop.handoff_to,
                    hop.binding
                );
            }
        }
        let context = if want_context { "applied" } else { "none" };
        hops.push(json!({
            "agent": agent,
            "binding": binding,
            "handoff_from": row.handoff_from,
            "handoff_to": row.handoff_to,
            "context": context,
            "result": row.result,
        }));
    }
    if rows[0].handoff_from.as_deref() != Some("horizon")
        || rows[1].handoff_from.as_deref() != Some("research")
        || rows[2].handoff_from.as_deref() != Some("idiom")
    {
        bail!(
            "refuse:dual-prove: handoff_from {:?} -> {:?} -> {:?}",
            rows[0].handoff_from,
            rows[1].handoff_from,
            rows[2].handoff_from
        );
    }

    let path = pack_session::session_path(state_dir, pack_id, session_id);
    let session = pack_session::load_session_file(&path)?;
    if session.turns.len() != HOPS.len() {
        bail!(
            "refuse:dual-prove: session turns {} want {}",
            session.turns.len(),
            HOPS.len()
        );
    }
    let hop2 = &session.turns[1];
    if hop2.agent != "idiom" {
        bail!("refuse:dual-prove: hop 2 session agent {}", hop2.agent);
    }
    if !hop2.result.contains("1. user:") || !hop2.result.contains(prompt) {
        bail!(
            "refuse:dual-prove: hop 2 completion did not carry hop 1 context: {}",
            hop2.result
        );
    }
    Ok(ChainFound { chain_id, hops })
}

fn decisions_report_names_specialties(state_dir: &Path, pack_id: &str) -> Result<String> {
    let mut receipts = crate::decisions::load_receipts(state_dir)?;
    let want = estate_schema::normalize_name(pack_id);
    receipts.retain(|row| {
        row.pack_id
            .as_deref()
            .map(|id| estate_schema::normalize_name(id) == want)
            .unwrap_or(false)
    });
    let report = crate::decisions::render_report(&receipts);
    for needle in [
        "capability=ag_news",
        "capability=rust_idiom",
        "capability=frontier_http",
        "surface=complete",
        "context=applied",
    ] {
        if !report.contains(needle) {
            bail!("refuse:dual-prove: decisions report missing {needle}");
        }
    }
    println!("decisions report --pack {pack_id}");
    print!("{report}");
    Ok("report names ag_news, rust_idiom, and frontier_http".into())
}

fn session_refuses(
    package_id: &str,
    pack_id: &str,
    session_id: &str,
    estate_path: &Path,
    state_dir: &Path,
    prompt: &str,
    expect_receipts: usize,
) -> Result<String> {
    let prev = std::env::var(MAX_TURNS_ENV).ok();
    std::env::set_var(MAX_TURNS_ENV, "1");
    let bound = rerun(package_id, estate_path, state_dir, prompt, session_id);
    match prev {
        Some(value) => std::env::set_var(MAX_TURNS_ENV, value),
        None => std::env::remove_var(MAX_TURNS_ENV),
    }
    expect_refuse(bound, "refuse:session-bound")?;
    expect_receipts_len(state_dir, expect_receipts)?;

    let path = pack_session::session_path(state_dir, pack_id, session_id);
    let mut session = pack_session::load_session_file(&path)?;
    session.expires_at = Some(1);
    pack_session::save_session(&path, &session)?;
    expect_refuse(
        rerun(package_id, estate_path, state_dir, prompt, session_id),
        "refuse:session-expired",
    )?;
    expect_receipts_len(state_dir, expect_receipts)?;

    session.ended = true;
    pack_session::save_session(&path, &session)?;
    expect_refuse(
        rerun(package_id, estate_path, state_dir, prompt, session_id),
        "refuse:session-ended",
    )?;
    expect_receipts_len(state_dir, expect_receipts)?;
    Ok("bound, expired, and ended refuse; journal unchanged".into())
}

fn rerun(
    package_id: &str,
    estate_path: &Path,
    state_dir: &Path,
    prompt: &str,
    session_id: &str,
) -> Result<()> {
    crate::ops::cmd_package_run(
        package_id,
        None,
        Some(prompt.to_string()),
        None,
        estate_path,
        state_dir,
        None,
        None,
        true,
        true,
        Some(session_id),
        false,
    )
}

fn expect_refuse(result: Result<()>, needle: &str) -> Result<()> {
    match result {
        Ok(()) => bail!("refuse:dual-prove: expected {needle}"),
        Err(err) => {
            let msg = err.to_string();
            if !msg.contains(needle) {
                bail!("refuse:dual-prove: expected {needle}, got {msg}");
            }
            println!("session refuse: {needle}");
            Ok(())
        }
    }
}

fn expect_receipts_len(state_dir: &Path, n: usize) -> Result<()> {
    let rows = crate::decisions::load_receipts(state_dir)?;
    if rows.len() != n {
        bail!(
            "refuse:dual-prove: refuse path wrote a receipt (journal {} want {n})",
            rows.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Estate {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/fixtures/agent-pack-handoff.yaml");
        estate_schema::load_estate(&path).unwrap()
    }

    #[test]
    fn fixture_dual_chain_stays_disjoint_and_two_specialties_abstain() {
        let estate = fixture();
        structural(&estate, DEFAULT_PACKAGE).unwrap();
        disjoint_allow(&estate, DEFAULT_PACKAGE).unwrap();
        equal_class_probe(&estate).unwrap();
        cross_seat(&estate).unwrap();
        sacred_out(&estate, "dual-specialty").unwrap();
        single_hop_abstain(&estate, DEFAULT_PACKAGE).unwrap();
    }
}
