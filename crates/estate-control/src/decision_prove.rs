//! Throwaway prove: host prepare → select/abstain → revalidate on authorize
//! and a granted convey call, against two specialty seats beside `local_slm`.
//!
//! Lab copy only. Mocked GGUF files. Does not train. Does not rewrite
//! `examples/estate.yaml`. `READY_FOR_LIVE_TEST` stays no.

use anyhow::{bail, Context, Result};
use estate_schema::{Agent, Effect, Intention, IntentionKind, Lane, ModelUseDecl, ToolDecl};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LOCKED_CKSUM: &str = "43770130 3391";
const PROVE_SCHEMA: &str = "cell-one.decision-host-validate-prove.v0";
const BINDING_AG: &str = "ag_news";
const BINDING_RUST: &str = "rust_idiom";
const SEAT_AG: &str = "specialist-agnews-3000";
const SEAT_RUST: &str = "specialist-rustidiom-3000";
const HOP_ID: &str = "specialty-hop";
const HOP_CAP: &str = "lane-tool";

pub(crate) fn cmd_decisions_host_validate_prove(root: &Path, out: Option<&Path>) -> Result<()> {
    std::env::remove_var("CELL_LOCAL_ENDPOINT");
    std::env::remove_var("CELL_LOCAL_LIVE");
    let root = root
        .canonicalize()
        .with_context(|| format!("refuse:root: {}", root.display()))?;
    let locked = root.join("examples/estate.yaml");
    let before = fs::read(&locked).with_context(|| format!("refuse:estate: {}", locked.display()))?;
    let cksum_before = file_cksum(&locked)?;
    if !cksum_before.starts_with(LOCKED_CKSUM) {
        bail!("refuse:estate: examples/estate.yaml cksum is {cksum_before}, want {LOCKED_CKSUM}");
    }

    let (out, wipe) = match out {
        Some(path) => (path.to_path_buf(), false),
        None => (default_out(), true),
    };
    if refuses_locked_target(&out, &root, &locked)? {
        bail!("refuse:out: host-validate-prove does not write examples/estate.yaml");
    }
    if wipe {
        let _ = fs::remove_dir_all(&out);
    }
    fs::create_dir_all(&out).with_context(|| format!("refuse:out: {}", out.display()))?;
    if refuses_locked_target(&out, &root, &locked)? {
        bail!("refuse:out: host-validate-prove does not write examples/estate.yaml");
    }

    let lab = out.join("lab-estate.yaml");
    if same_file(&lab, &locked)? {
        bail!("refuse:out: host-validate-prove does not write examples/estate.yaml");
    }
    let gguf_ag = out.join("fixtures").join(BINDING_AG).join("specialist.Q4_K_M.gguf");
    let gguf_rust = out
        .join("fixtures")
        .join(BINDING_RUST)
        .join("specialist.Q4_K_M.gguf");
    seed_lab(&lab, &before, &gguf_ag, &gguf_rust)?;
    ensure_locked(&locked, &before)?;

    let state = out.join("state");
    let _ = fs::remove_dir_all(&state);
    fs::create_dir_all(&state)?;
    let policy = root.join("policy/cell-one.policy.v0.yaml");

    clear_hint(&state)?;
    let auth_ok = run_authorize(
        &lab,
        &state,
        "research",
        "model",
        BINDING_AG,
        true,
    )?;
    expect_select(&auth_ok, "authorize", "research", BINDING_AG, BINDING_AG)?;
    println!(
        "authorize select: agent=research result={} validation=ok surface=authorize granted=yes",
        auth_ok.result
    );

    let auth_abstain = run_authorize(&lab, &state, "sanctum", "model", BINDING_AG, true)?;
    expect_abstain(&auth_abstain, "authorize", "sanctum")?;
    println!(
        "authorize abstain: agent=sanctum result=abstain validation=ok surface=authorize selector=refuse:decision-abstain model_granted=no"
    );

    write_hint(&state, "xai_grok", None)?;
    let auth_ineligible = run_authorize(&lab, &state, "research", "tool", "notes-append", false)?;
    expect_fallback(
        &auth_ineligible,
        "authorize",
        "research",
        "ineligible",
        "xai_grok",
        BINDING_AG,
        "refuse:deny-default",
    )?;
    println!(
        "authorize fallback: agent=research validation=ineligible fallback=ag_news outcome=refuse:deny-default granted=no"
    );

    write_hint(&state, BINDING_AG, Some("0"))?;
    let auth_stale = run_authorize(&lab, &state, "research", "tool", "notes-append", false)?;
    expect_fallback(
        &auth_stale,
        "authorize",
        "research",
        "stale",
        BINDING_AG,
        BINDING_AG,
        "refuse:deny-default",
    )?;
    println!(
        "authorize fallback: agent=research validation=stale fallback=ag_news outcome=refuse:deny-default granted=no"
    );

    clear_hint(&state)?;
    declare_hop(&lab, &state)?;
    let mesh = conveyor_proxy::load_mesh(&state).map_err(|err| anyhow::anyhow!("{err}"))?;
    let lease = mesh
        .leases
        .iter()
        .find(|row| row.hop_id == HOP_ID)
        .ok_or_else(|| anyhow::anyhow!("refuse:hop: {HOP_ID} lease missing"))?;
    if !lease.granted || lease.capability != HOP_CAP {
        bail!(
            "refuse:hop: {HOP_ID} granted={} capability={}",
            lease.granted,
            lease.capability
        );
    }
    for agent in ["research", "idiom", "sanctum"] {
        if !lease.agents.iter().any(|id| id == agent) {
            bail!("refuse:hop: {HOP_ID} lease missing agent {agent}");
        }
    }
    println!("hop lease: {HOP_ID} granted=yes capability={HOP_CAP}");

    let convey_ag = run_convey(&lab, &state, &policy, "research", true)?;
    expect_select(&convey_ag, "", "research", BINDING_AG, HOP_CAP)?;
    if convey_ag.hop_id != HOP_ID {
        bail!("refuse:decision: convey hop_id is {}", convey_ag.hop_id);
    }
    println!(
        "convey select: agent=research result={} validation=ok surface=convey granted=yes",
        convey_ag.result
    );

    let convey_rust = run_convey(&lab, &state, &policy, "idiom", true)?;
    expect_select(&convey_rust, "", "idiom", BINDING_RUST, HOP_CAP)?;
    println!(
        "convey select: agent=idiom result={} validation=ok surface=convey granted=yes",
        convey_rust.result
    );

    let convey_abstain = run_convey(&lab, &state, &policy, "sanctum", true)?;
    expect_abstain(&convey_abstain, "", "sanctum")?;
    println!(
        "convey abstain: agent=sanctum result=abstain validation=ok surface=convey selector=refuse:decision-abstain model_granted=no"
    );

    write_hint(&state, BINDING_AG, Some("0"))?;
    let convey_stale = run_convey(&lab, &state, &policy, "research", true)?;
    expect_fallback(
        &convey_stale,
        "",
        "research",
        "stale",
        BINDING_AG,
        BINDING_AG,
        "allow",
    )?;
    if convey_stale.validation == "ok" {
        bail!("refuse:decision: stale convey hint was granted");
    }
    println!(
        "convey fallback: agent=research validation=stale fallback=ag_news outcome=allow hop_granted=yes model_granted=no"
    );

    let receipts = crate::decisions::load_receipts(&state)?;
    if receipts.len() != 8 {
        bail!("refuse:decision: journal has {} receipts, want 8", receipts.len());
    }
    let report = crate::decisions::render_report(&receipts);
    for needle in [
        "decision receipts: 8\n",
        "stage prepare=8 select=2 validate=3 fallback=3\n",
        "validation ok=5 stale=2 ineligible=1 expired=0\n",
        "surface authorize=4 convey=4 complete=0\n",
        "fallback none=5\n",
        "fallback ag_news=3\n",
        "surface=authorize",
        "surface=convey",
        "result=ag_news",
        "result=rust_idiom",
        "result=abstain",
    ] {
        if !report.contains(needle) {
            bail!("refuse:decision: report missing {needle}");
        }
    }
    if report.split_whitespace().any(|word| word == "enforced") {
        bail!("refuse:decision: report invented enforced");
    }
    println!("decisions report");
    print!("{report}");

    let replay = out.join("decisions-replay.jsonl");
    crate::decisions::cmd_decisions_export(&state, &replay)?;
    let journal = state.join("decisions").join(crate::decisions::JOURNAL_FILE);
    if fs::read(&replay)? != fs::read(&journal)? {
        bail!("refuse:decision: export drifted from the journal");
    }

    ensure_locked(&locked, &before)?;
    let cksum_after = file_cksum(&locked)?;
    if cksum_after != cksum_before || fs::read(&locked)? != before {
        bail!("refuse:estate: examples/estate.yaml changed");
    }
    if !gguf_ag.is_file() || !gguf_rust.is_file() {
        bail!("refuse:gguf: mocked specialty files missing");
    }
    if !fs::read(&gguf_ag)?.starts_with(b"GGUF") || !fs::read(&gguf_rust)?.starts_with(b"GGUF") {
        bail!("refuse:gguf: mocked specialty files are not GGUF stubs");
    }

    let body = serde_json::json!({
        "schema": PROVE_SCHEMA,
        "ok": true,
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "estate_cksum": cksum_after,
        "lab_estate": lab.display().to_string(),
        "bindings": ["local_slm", BINDING_AG, BINDING_RUST],
        "seat_models": {
            BINDING_AG: SEAT_AG,
            BINDING_RUST: SEAT_RUST,
        },
        "agents": {
            BINDING_AG: "research",
            BINDING_RUST: "idiom",
            "abstain": "sanctum",
        },
        "gguf": {
            BINDING_AG: gguf_ag.display().to_string(),
            BINDING_RUST: gguf_rust.display().to_string(),
        },
        "hop_id": HOP_ID,
        "authorize": {
            "select_ok": {
                "agent": "research",
                "result": BINDING_AG,
                "validation": "ok",
                "surface": "authorize",
                "granted": true,
            },
            "abstain": {
                "agent": "sanctum",
                "result": "abstain",
                "validation": "ok",
                "selector": "refuse:decision-abstain",
                "model_granted": false,
            },
            "ineligible": {
                "validation": "ineligible",
                "result": "xai_grok",
                "fallback": BINDING_AG,
                "outcome": "refuse:deny-default",
                "granted": false,
            },
            "stale": {
                "validation": "stale",
                "result": BINDING_AG,
                "fallback": BINDING_AG,
                "outcome": "refuse:deny-default",
                "granted": false,
            },
        },
        "convey": {
            "select_ok": [
                {
                    "agent": "research",
                    "result": BINDING_AG,
                    "validation": "ok",
                    "surface": "convey",
                    "granted": true,
                },
                {
                    "agent": "idiom",
                    "result": BINDING_RUST,
                    "validation": "ok",
                    "surface": "convey",
                    "granted": true,
                },
            ],
            "abstain": {
                "agent": "sanctum",
                "result": "abstain",
                "validation": "ok",
                "selector": "refuse:decision-abstain",
                "model_granted": false,
            },
            "stale": {
                "validation": "stale",
                "result": BINDING_AG,
                "fallback": BINDING_AG,
                "outcome": "allow",
                "hop_granted": true,
                "model_granted": false,
            },
        },
        "counts": {
            "receipts": 8,
            "surface_authorize": 4,
            "surface_convey": 4,
            "surface_complete": 0,
            "validation_ok": 5,
            "validation_stale": 2,
            "validation_ineligible": 1,
        },
        "note": "Fixture prove. Lab copy. Two mocked GGUFs. No GPU train. Not a live PASS."
    });
    let pretty = serde_json::to_string_pretty(&body)?;
    fs::write(out.join("host-validate-prove.json"), format!("{pretty}\n"))?;
    println!("{pretty}");
    println!("decision-host-validate-prove: ok");
    println!("READY_FOR_LIVE_TEST: no");
    Ok(())
}

fn seed_lab(lab: &Path, locked_bytes: &[u8], gguf_ag: &Path, gguf_rust: &Path) -> Result<()> {
    let mut estate = estate_schema::parse_estate_yaml(
        std::str::from_utf8(locked_bytes).context("refuse:estate: examples/estate.yaml is not utf-8")?,
    )
    .map_err(|err| anyhow::anyhow!("refuse:estate: {err}"))?;
    let local_before = local_params(&estate)?;
    if estate.agent("idiom").is_some() {
        bail!("refuse:agent: idiom already on the locked estate");
    }
    push_specialty(&mut estate, BINDING_AG, SEAT_AG)?;
    push_specialty(&mut estate, BINDING_RUST, SEAT_RUST)?;
    if local_params(&estate)? != local_before {
        bail!("refuse:binding: local_slm params changed while specialty seats were added");
    }

    estate.agents.push(Agent {
        id: "idiom".into(),
        display_name: "Idiom".into(),
        lane: "idiom".into(),
        desktop: "idiom-desktop".into(),
        tools: Vec::new(),
        mounts: Vec::new(),
        mcp: Vec::new(),
        models: Vec::new(),
        calls: Vec::new(),
        select: None,
    });
    estate.lanes.push(Lane {
        id: "idiom".into(),
        root_path: "lanes/idiom".into(),
        owner_agent_id: "idiom".into(),
    });
    let placement = estate
        .placements
        .iter_mut()
        .find(|row| row.id == "cell-one-box")
        .ok_or_else(|| anyhow::anyhow!("refuse:estate: cell-one-box placement missing"))?;
    placement.agents.push("idiom".into());

    set_models(
        &mut estate,
        "research",
        &[BINDING_AG],
        "Lab copy. Research authorizes and calls the ag_news specialty seat.",
    )?;
    set_models(
        &mut estate,
        "idiom",
        &[BINDING_RUST],
        "Lab copy. Idiom calls the rust_idiom specialty seat.",
    )?;
    set_models(
        &mut estate,
        "sanctum",
        &[BINDING_AG, BINDING_RUST],
        "Lab copy. Two specialty ids. The selector abstains.",
    )?;
    for agent in ["research", "idiom", "sanctum"] {
        ensure_tool(&mut estate, agent, HOP_CAP)?;
    }
    allow_model(
        &mut estate,
        "research",
        BINDING_AG,
        "Lab copy. Research may use ag_news.",
    );
    allow_model(
        &mut estate,
        "idiom",
        BINDING_RUST,
        "Lab copy. Idiom may use rust_idiom.",
    );
    allow_model(
        &mut estate,
        "sanctum",
        BINDING_AG,
        "Lab copy. Sanctum lists both specialty seats.",
    );
    allow_model(
        &mut estate,
        "sanctum",
        BINDING_RUST,
        "Lab copy. Sanctum lists both specialty seats.",
    );
    for agent in ["research", "idiom", "sanctum"] {
        estate.intentions.push(Intention {
            subject_agent: agent.into(),
            object: HOP_CAP.into(),
            kind: IntentionKind::Tool,
            effect: Effect::Allow,
            note: Some(
                "Lab copy. Granted hop capability. The selector still does not grant a model."
                    .into(),
            ),
        });
    }

    estate_schema::validate(&estate).map_err(|errs| {
        anyhow::anyhow!("refuse:estate: lab copy invalid: {}", errs.join("; "))
    })?;
    if let Some(parent) = lab.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(lab, estate_schema::render_estate_yaml(&estate)?)?;
    for (path, dataset) in [(gguf_ag, BINDING_AG), (gguf_rust, BINDING_RUST)] {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, b"GGUF")?;
        fs::write(
            parent_file(path, "comparison.json")?,
            format!(
                "{{\n  \"dataset\": \"{dataset}\",\n  \"live_pass_recorded\": false,\n  \"note\": \"Fixture stub. This file does not record a live PASS. READY_FOR_LIVE_TEST stays no.\"\n}}\n"
            ),
        )?;
    }

    let loaded = estate_schema::load_estate(lab)
        .with_context(|| format!("refuse:estate: load {}", lab.display()))?;
    for id in ["local_slm", BINDING_AG, BINDING_RUST] {
        let binding = loaded
            .model_bindings
            .iter()
            .find(|row| row.id == id && row.class == estate_schema::ModelClass::Local)
            .ok_or_else(|| anyhow::anyhow!("refuse:binding: {id} missing on lab copy"))?;
        if !binding.wired {
            bail!("refuse:binding: {id} is not wired");
        }
    }
    let ag_model = loaded
        .model_bindings
        .iter()
        .find(|row| row.id == BINDING_AG)
        .and_then(|row| row.params.get("model"))
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let rust_model = loaded
        .model_bindings
        .iter()
        .find(|row| row.id == BINDING_RUST)
        .and_then(|row| row.params.get("model"))
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if ag_model != SEAT_AG || rust_model != SEAT_RUST {
        bail!("refuse:binding: seat models are {ag_model} and {rust_model}");
    }
    if local_params(&loaded)? != local_before {
        bail!("refuse:binding: local_slm params changed on the lab copy");
    }
    println!("lab copy: {}", lab.display());
    println!("lab seat: {BINDING_AG} class=local model={SEAT_AG}");
    println!("lab seat: {BINDING_RUST} class=local model={SEAT_RUST}");
    println!("local_slm kept");
    println!("lab intention: research model ag_news effect=allow");
    println!("lab intention: idiom model rust_idiom effect=allow");
    println!("lab intention: sanctum model ag_news,rust_idiom effect=allow");
    println!("mocked gguf: {}", gguf_ag.display());
    println!("mocked gguf: {}", gguf_rust.display());
    Ok(())
}

fn parent_file(path: &Path, name: &str) -> Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("refuse:gguf: {} has no parent", path.display()))?;
    Ok(parent.join(name))
}

fn local_params(estate: &estate_schema::Estate) -> Result<serde_json::Value> {
    let binding = estate
        .model_bindings
        .iter()
        .find(|row| row.id == "local_slm")
        .ok_or_else(|| anyhow::anyhow!("refuse:binding: local_slm missing"))?;
    Ok(binding.params.clone())
}

fn push_specialty(estate: &mut estate_schema::Estate, id: &str, model: &str) -> Result<()> {
    let mut seat = estate
        .model_bindings
        .iter()
        .find(|row| row.id == "local_slm")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("refuse:binding: local_slm missing"))?;
    seat.id = id.to_string();
    let map = seat
        .params
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("refuse:binding: local_slm params are not an object"))?;
    map.insert("model".into(), serde_json::Value::String(model.to_string()));
    seat.wired = true;
    estate.model_bindings.push(seat);
    Ok(())
}

fn set_models(
    estate: &mut estate_schema::Estate,
    agent_id: &str,
    ids: &[&str],
    note: &str,
) -> Result<()> {
    let agent = estate
        .agents
        .iter_mut()
        .find(|row| row.id == agent_id)
        .ok_or_else(|| anyhow::anyhow!("refuse:agent: {agent_id} missing on lab estate"))?;
    agent.models = ids
        .iter()
        .map(|id| ModelUseDecl {
            id: (*id).to_string(),
            description: Some(note.to_string()),
        })
        .collect();
    Ok(())
}

fn ensure_tool(estate: &mut estate_schema::Estate, agent_id: &str, tool: &str) -> Result<()> {
    let agent = estate
        .agents
        .iter_mut()
        .find(|row| row.id == agent_id)
        .ok_or_else(|| anyhow::anyhow!("refuse:agent: {agent_id} missing on lab estate"))?;
    if agent.tools.iter().any(|row| row.id == tool) {
        return Ok(());
    }
    agent.tools.push(ToolDecl {
        id: tool.to_string(),
        description: Some(
            "Lab hop capability. Declaring it is not a model grant.".into(),
        ),
    });
    Ok(())
}

fn allow_model(estate: &mut estate_schema::Estate, agent: &str, object: &str, note: &str) {
    estate.intentions.push(Intention {
        subject_agent: agent.to_string(),
        object: object.to_string(),
        kind: IntentionKind::Model,
        effect: Effect::Allow,
        note: Some(note.to_string()),
    });
}

fn run_authorize(
    lab: &Path,
    state: &Path,
    agent: &str,
    kind: &str,
    object: &str,
    expect_ok: bool,
) -> Result<crate::decisions::DecisionReceipt> {
    let before = crate::decisions::load_receipts(state)?.len();
    let result = crate::ops::cmd_authorize(agent, kind, object, lab, state, None);
    match (expect_ok, result) {
        (true, Ok(())) => {}
        (false, Err(err)) => {
            let text = format!("{err:#}");
            if !text.contains("authorize denied") {
                bail!("refuse:decision: authorize {agent} {kind} {object} failed: {text}");
            }
        }
        (true, Err(err)) => {
            bail!("refuse:decision: authorize {agent} {kind} {object} did not grant: {err:#}")
        }
        (false, Ok(())) => {
            bail!("refuse:decision: authorize {agent} {kind} {object} granted")
        }
    }
    take_new(state, before)
}

fn run_convey(
    lab: &Path,
    state: &Path,
    policy: &Path,
    agent: &str,
    expect_ok: bool,
) -> Result<crate::decisions::DecisionReceipt> {
    let before = crate::decisions::load_receipts(state)?.len();
    let result = crate::ops::cmd_convey_call(
        HOP_ID,
        HOP_CAP,
        Some(agent),
        Some("tool"),
        lab,
        state,
        policy,
    );
    match (expect_ok, result) {
        (true, Ok(())) => {}
        (true, Err(err)) => bail!("refuse:decision: convey {agent} did not grant: {err:#}"),
        (false, Ok(())) => bail!("refuse:decision: convey {agent} granted"),
        (false, Err(err)) => {
            let text = format!("{err:#}");
            if text.contains("live PASS") {
                bail!("refuse:decision: convey invented a live PASS");
            }
        }
    }
    take_new(state, before)
}

fn declare_hop(lab: &Path, state: &Path) -> Result<()> {
    let agents = vec!["research".to_string(), "idiom".to_string(), "sanctum".to_string()];
    crate::ops::cmd_convey_hop(
        HOP_ID,
        "box",
        HOP_CAP,
        "any",
        true,
        None,
        &agents,
        Some("tool"),
        lab,
        state,
    )?;
    Ok(())
}

fn take_new(state: &Path, before: usize) -> Result<crate::decisions::DecisionReceipt> {
    let rows = crate::decisions::load_receipts(state)?;
    if rows.len() != before + 1 {
        bail!(
            "refuse:decision: expected {} receipts, got {}",
            before + 1,
            rows.len()
        );
    }
    rows.into_iter()
        .next_back()
        .ok_or_else(|| anyhow::anyhow!("refuse:decision: empty journal"))
}

fn expect_select(
    receipt: &crate::decisions::DecisionReceipt,
    surface: &str,
    agent: &str,
    result: &str,
    capability: &str,
) -> Result<()> {
    reject_invented(receipt)?;
    if receipt.surface != surface {
        bail!(
            "refuse:decision: {agent} surface is '{}', want '{surface}'",
            receipt.surface
        );
    }
    if receipt.agent.as_deref() != Some(agent) {
        bail!("refuse:decision: agent is {:?}, want {agent}", receipt.agent);
    }
    if receipt.result != result || receipt.validation != "ok" || receipt.fallback.is_some() {
        bail!(
            "refuse:decision: {agent} result={} validation={} fallback={:?}",
            receipt.result,
            receipt.validation,
            receipt.fallback
        );
    }
    if receipt.outcome != "allow" || receipt.stage != "validate" {
        bail!(
            "refuse:decision: {agent} outcome={} stage={}",
            receipt.outcome,
            receipt.stage
        );
    }
    if receipt.capability != capability {
        bail!(
            "refuse:decision: {agent} capability is {}, want {capability}",
            receipt.capability
        );
    }
    let ids = candidate_ids(receipt);
    if ids != vec![result] {
        bail!("refuse:decision: {agent} candidates are {ids:?}, want [{result}]");
    }
    Ok(())
}

fn expect_abstain(
    receipt: &crate::decisions::DecisionReceipt,
    surface: &str,
    agent: &str,
) -> Result<()> {
    reject_invented(receipt)?;
    if receipt.surface != surface || receipt.agent.as_deref() != Some(agent) {
        bail!(
            "refuse:decision: abstain surface={} agent={:?}",
            receipt.surface,
            receipt.agent
        );
    }
    if receipt.result != "abstain" || receipt.validation != "ok" || receipt.stage != "select" {
        bail!(
            "refuse:decision: {agent} abstain result={} validation={} stage={}",
            receipt.result,
            receipt.validation,
            receipt.stage
        );
    }
    if receipt.fallback.is_some() || receipt.outcome != "allow" {
        bail!(
            "refuse:decision: {agent} abstain fallback={:?} outcome={}",
            receipt.fallback,
            receipt.outcome
        );
    }
    let ids = candidate_ids(receipt);
    if ids.len() != 2 || !ids.contains(&BINDING_AG) || !ids.contains(&BINDING_RUST) {
        bail!("refuse:decision: {agent} candidates are {ids:?}, want ag_news and rust_idiom");
    }
    if ids.iter().any(|id| *id == "local_slm" || *id == "xai_grok") {
        bail!("refuse:decision: {agent} abstain included a non-specialty seat");
    }
    Ok(())
}

fn expect_fallback(
    receipt: &crate::decisions::DecisionReceipt,
    surface: &str,
    agent: &str,
    validation: &str,
    result: &str,
    fallback: &str,
    outcome: &str,
) -> Result<()> {
    reject_invented(receipt)?;
    if receipt.surface != surface || receipt.agent.as_deref() != Some(agent) {
        bail!(
            "refuse:decision: fallback surface={} agent={:?}",
            receipt.surface,
            receipt.agent
        );
    }
    if receipt.validation != validation || receipt.result != result {
        bail!(
            "refuse:decision: fallback validation={} result={}, want {validation} {result}",
            receipt.validation,
            receipt.result
        );
    }
    if receipt.fallback.as_deref() != Some(fallback) || receipt.stage != "fallback" {
        bail!(
            "refuse:decision: fallback={:?} stage={}",
            receipt.fallback,
            receipt.stage
        );
    }
    if receipt.outcome != outcome {
        bail!(
            "refuse:decision: fallback outcome={}, want {outcome}",
            receipt.outcome
        );
    }
    if receipt.validation == "ok" {
        bail!("refuse:decision: stale or ineligible hint was granted");
    }
    let ids = candidate_ids(receipt);
    if ids != vec![BINDING_AG] {
        bail!("refuse:decision: fallback candidates are {ids:?}, want [ag_news]");
    }
    Ok(())
}

fn candidate_ids(receipt: &crate::decisions::DecisionReceipt) -> Vec<&str> {
    receipt
        .candidates
        .iter()
        .map(|row| row.id.as_str())
        .collect()
}

fn reject_invented(receipt: &crate::decisions::DecisionReceipt) -> Result<()> {
    if receipt.schema != crate::decisions::RECEIPT_SCHEMA {
        bail!("refuse:decision: schema {}", receipt.schema);
    }
    if receipt.outcome.split_whitespace().any(|word| word == "enforced") {
        bail!("refuse:decision: outcome invented enforced");
    }
    if receipt.outcome.contains("live PASS") {
        bail!("refuse:decision: outcome invented a live PASS");
    }
    Ok(())
}

fn write_hint(state: &Path, select: &str, digest: Option<&str>) -> Result<()> {
    let digest = match digest {
        Some(raw) => format!("\"{raw}\""),
        None => "null".into(),
    };
    fs::write(
        state.join(crate::decisions::SELECT_FILE),
        format!(
            r#"{{"schema":"cell-one.decision-select.v0","select":"{select}","digest":{digest}}}"#
        ),
    )?;
    Ok(())
}

fn clear_hint(state: &Path) -> Result<()> {
    let path = state.join(crate::decisions::SELECT_FILE);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn ensure_locked(locked: &Path, before: &[u8]) -> Result<()> {
    if fs::read(locked)? != before {
        bail!("refuse:estate: examples/estate.yaml bytes changed");
    }
    Ok(())
}

fn refuses_locked_target(out: &Path, root: &Path, locked: &Path) -> Result<bool> {
    let cwd = std::env::current_dir().context("refuse:out: cwd")?;
    let abs = if out.is_absolute() {
        out.to_path_buf()
    } else {
        cwd.join(out)
    };
    if abs == locked || same_file(&abs, locked)? {
        return Ok(true);
    }
    let examples = root.join("examples");
    let abs_c = abs.canonicalize().unwrap_or(abs);
    let examples_c = examples.canonicalize().unwrap_or(examples);
    Ok(abs_c == examples_c || abs_c.starts_with(&examples_c))
}

fn file_cksum(path: &Path) -> Result<String> {
    let out = Command::new("cksum")
        .arg(path)
        .output()
        .context("refuse:estate: cksum")?;
    if !out.status.success() {
        bail!(
            "refuse:estate: cksum failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn same_file(left: &Path, right: &Path) -> Result<bool> {
    if left == right {
        return Ok(true);
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(a), Ok(b)) => Ok(a == b),
        _ => Ok(false),
    }
}

fn default_out() -> PathBuf {
    let mut token = std::process::id().to_string();
    for needle in ["5090", "4090", "4080", "3090"] {
        token = token.replace(needle, "0000");
    }
    std::env::temp_dir().join(format!("cell-one-decision-host-validate-{token}"))
}
