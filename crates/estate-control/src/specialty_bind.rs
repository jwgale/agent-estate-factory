//! Throwaway prove: classify-journey GGUF stub → `local_slm` → Standing next.
//!
//! Does not train. Does not start a GPU job. Does not rewrite
//! `examples/estate.yaml`. `READY_FOR_LIVE_TEST` stays no.

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LOCKED_CKSUM: &str = "43770130 3391";
const SEAT_MODEL: &str = "specialist-agnews-3000";
const FUNCTION: &str = "ag_news";
const TAG: &str = "cell-enrich-overnight-traces";
const PROVE_SCHEMA: &str = "cell-one.specialty-bind-prove.v0";

pub(crate) fn cmd_enrich_bind_prove(root: &Path, out: Option<&Path>, dual: bool) -> Result<()> {
    if dual {
        return cmd_enrich_bind_prove_dual(root, out);
    }
    cmd_enrich_bind_prove_single(root, out)
}

fn cmd_enrich_bind_prove_single(root: &Path, out: Option<&Path>) -> Result<()> {
    std::env::remove_var("CELL_LOCAL_ENDPOINT");
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
    if wipe {
        let _ = fs::remove_dir_all(&out);
    }
    fs::create_dir_all(&out).with_context(|| format!("refuse:out: {}", out.display()))?;
    if same_file(&out, &locked)? {
        bail!("refuse:out: bind-prove does not write examples/estate.yaml");
    }

    let lab = write_lab_estate(&out, &before)?;
    let state = out.join("state");
    let plans = out.join("plans");
    let roots = out.join("roots");
    fs::create_dir_all(&state)?;
    fs::create_dir_all(&plans)?;
    fs::create_dir_all(&roots)?;
    let journey = out.join("journey");
    fs::create_dir_all(&journey)?;
    let gguf = journey.join("specialist.Q4_K_M.gguf");
    fs::write(&gguf, b"GGUF")?;
    fs::write(
        journey.join("comparison.json"),
        "{\n  \"dataset\": \"ag_news\",\n  \"live_pass_recorded\": false,\n  \"note\": \"Fixture stub. This file does not record a live PASS. READY_FOR_LIVE_TEST stays no.\"\n}\n",
    )?;

    let pack = root.join("examples/fixtures/specialist-overnight.pack.json");
    let packs_dir = root.join("packs");
    let policy = root.join("policy/cell-one.policy.v0.yaml");
    crate::enrich::cmd_enrich_prepare(
        &lab,
        &pack,
        &packs_dir,
        Some("llamafactory-lora"),
        false,
        None,
        &state,
        Some("train"),
        "jason",
        None,
        false,
        false,
    )?;
    let prepared = state.join("enrich/overnight-traces/llamafactory-lora");
    if !prepared.join("prepare.json").is_file() {
        bail!(
            "refuse:prepare: {} is missing prepare.json",
            prepared.display()
        );
    }

    crate::enrich::cmd_enrich_import_trained(
        &lab,
        &prepared,
        TAG,
        &gguf,
        "jason",
        None,
        Some(SEAT_MODEL),
        Some(SEAT_MODEL),
        Some(FUNCTION),
    )?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: import-trained rewrote examples/estate.yaml");
    }
    let proposal: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(prepared.join("binding-proposal.json"))
            .context("refuse:proposal: binding-proposal.json")?,
    )?;
    if proposal["trained_shape"] != "gguf"
        || proposal["auto_apply"] != false
        || proposal["binding_id"] != "local_slm"
        || proposal["estate_rewritten"] != false
    {
        bail!("refuse:proposal: import-trained did not record local_slm gguf auto_apply=false");
    }

    crate::enrich::cmd_enrich_apply_proposal(
        &lab,
        &prepared,
        TAG,
        &state,
        &plans,
        "jason",
        false,
    )?;
    let staged = state.join("enrich-stage/staged-estate.yaml");
    crate::plan_apply::cmd_plan(
        &staged,
        None,
        &plans,
        &state,
        false,
        &plans.join("reviewed"),
    )?;
    crate::plan_apply::cmd_apply(
        &staged,
        &state,
        &roots,
        &plans,
        true,
        None,
        &packs_dir,
        false,
        false,
        &policy,
        "jason",
        false,
    )?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: apply rewrote examples/estate.yaml");
    }

    let applied = estate_schema::load_estate(&lab)
        .with_context(|| format!("refuse:estate: load {}", lab.display()))?;
    let binding = applied
        .model_bindings
        .iter()
        .find(|row| row.id == "local_slm")
        .ok_or_else(|| anyhow::anyhow!("refuse:binding: local_slm missing after apply"))?;
    let model = binding
        .params
        .get("model")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if binding.class != estate_schema::ModelClass::Local || model != SEAT_MODEL {
        bail!("refuse:binding: local_slm model is '{model}', want {SEAT_MODEL}");
    }
    println!("binding present: local_slm class=local model={model}");

    crate::enrich::cmd_enrich_standing_next(&lab, &prepared)?;
    crate::ops::cmd_complete(
        "research",
        Some("ping specialty".into()),
        None,
        None,
        None,
        None,
        None,
        &lab,
        &state,
        None,
        None,
        true,
        None,
        None,
        None,
        false,
    )?;
    crate::decisions::cmd_decisions_report(&state, None)?;
    crate::watch::cmd_status(
        &lab,
        &state,
        &roots,
        &plans,
        &packs_dir,
        &policy,
        &root,
    )?;

    let cksum_after = file_cksum(&locked)?;
    if cksum_after != cksum_before {
        bail!("refuse:estate: examples/estate.yaml cksum changed to {cksum_after}");
    }
    let report = serde_json::json!({
        "schema": PROVE_SCHEMA,
        "ok": true,
        "binding_id": "local_slm",
        "class": "local",
        "function": FUNCTION,
        "trained_shape": "gguf",
        "auto_apply": false,
        "joinable": true,
        "seat_model": SEAT_MODEL,
        "gguf": gguf.display().to_string(),
        "agent": "research",
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "estate_cksum": cksum_after,
        "note": "Fixture prove. Mocked GGUF. No GPU train. Not a live PASS."
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    println!("specialty-bind-prove: ok");
    println!("READY_FOR_LIVE_TEST: no");
    Ok(())
}

const DUAL_SCHEMA: &str = "cell-one.specialty-bind-prove-dual.v0";
const SEAT_RUST: &str = "specialist-rustidiom-3000";
const FUNCTION_RUST: &str = "rust_idiom";
const BINDING_AG: &str = "ag_news";
const BINDING_RUST: &str = "rust_idiom";
const PACK_AG: &str = "overnight-traces";
const PACK_RUST: &str = "idiom-traces";
const DRIVER_LORA: &str = "llamafactory-lora";

pub(crate) struct DualSpecialtyLab {
    pub lab: PathBuf,
    pub state: PathBuf,
    pub prepared_ag: PathBuf,
    pub prepared_rust: PathBuf,
    pub gguf_ag: PathBuf,
    pub gguf_rust: PathBuf,
}

/// Land `ag_news` and `rust_idiom` beside `local_slm` on a lab copy.
/// Stops after Standing next. Does not write mock receipts.
pub(crate) fn land_dual_specialty_lab(root: &Path, out: &Path) -> Result<DualSpecialtyLab> {
    run_dual_specialty_bind(root, Some(out), false)?
        .ok_or_else(|| anyhow::anyhow!("refuse:specialty-bind: dual land returned no lab"))
}

fn cmd_enrich_bind_prove_dual(root: &Path, out: Option<&Path>) -> Result<()> {
    run_dual_specialty_bind(root, out, true)?;
    Ok(())
}

fn run_dual_specialty_bind(
    root: &Path,
    out: Option<&Path>,
    prove: bool,
) -> Result<Option<DualSpecialtyLab>> {
    std::env::remove_var("CELL_LOCAL_ENDPOINT");
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
        None => (default_out_dual(), true),
    };
    if wipe {
        let _ = fs::remove_dir_all(&out);
    }
    fs::create_dir_all(&out).with_context(|| format!("refuse:out: {}", out.display()))?;
    if same_file(&out, &locked)? {
        bail!("refuse:out: bind-prove does not write examples/estate.yaml");
    }

    let lab = out.join("lab-estate.yaml");
    fs::write(&lab, &before)?;
    seed_idiom_agent(&lab)?;
    let local_before = local_slm_params(&lab)?;
    let state = out.join("state");
    let plans = out.join("plans");
    let roots = out.join("roots");
    fs::create_dir_all(&state)?;
    fs::create_dir_all(&plans)?;
    fs::create_dir_all(&roots)?;
    let journey = out.join("journey");
    fs::create_dir_all(journey.join(BINDING_AG))?;
    fs::create_dir_all(journey.join(BINDING_RUST))?;
    let gguf_ag = journey.join(BINDING_AG).join("specialist.Q4_K_M.gguf");
    let gguf_rust = journey.join(BINDING_RUST).join("specialist.Q4_K_M.gguf");
    fs::write(&gguf_ag, b"GGUF")?;
    fs::write(&gguf_rust, b"GGUF")?;
    for (dir, dataset) in [(BINDING_AG, BINDING_AG), (BINDING_RUST, BINDING_RUST)] {
        fs::write(
            journey.join(dir).join("comparison.json"),
            format!(
                "{{\n  \"dataset\": \"{dataset}\",\n  \"live_pass_recorded\": false,\n  \"note\": \"Fixture stub. This file does not record a live PASS. READY_FOR_LIVE_TEST stays no.\"\n}}\n"
            ),
        )?;
    }

    let packs_dir = root.join("packs");
    let policy = root.join("policy/cell-one.policy.v0.yaml");
    let overnight = root.join("examples/fixtures/specialist-overnight.pack.json");
    // Lab packs name a seated model tag so prepare can write a job.
    // The locked estate's local_slm params stay unchanged.
    let ag_pack = write_lab_pack(&out, &overnight, PACK_AG)?;
    let rust_pack = write_lab_pack(&out, &overnight, PACK_RUST)?;
    prepare_lora(&lab, &ag_pack, &packs_dir, &state)?;
    prepare_lora(&lab, &rust_pack, &packs_dir, &state)?;
    ensure_locked(&locked, &before)?;

    let prepared_ag = prepared_lora(&state, PACK_AG);
    let prepared_rust = prepared_lora(&state, PACK_RUST);
    let tag_ag = format!("cell-enrich-{PACK_AG}");
    let tag_rust = format!("cell-enrich-{PACK_RUST}");
    land_specialty(
        &lab,
        &prepared_ag,
        &gguf_ag,
        &tag_ag,
        BINDING_AG,
        SEAT_MODEL,
        FUNCTION,
        "research",
        &state,
        &plans,
        &roots,
        &packs_dir,
        &policy,
        &locked,
        &before,
    )?;
    land_specialty(
        &lab,
        &prepared_rust,
        &gguf_rust,
        &tag_rust,
        BINDING_RUST,
        SEAT_RUST,
        FUNCTION_RUST,
        "idiom",
        &state,
        &plans,
        &roots,
        &packs_dir,
        &policy,
        &locked,
        &before,
    )?;
    let local_after = local_slm_params(&lab)?;
    if local_after != local_before {
        bail!("refuse:binding: local_slm params changed while specialty seats were added");
    }
    if !estate_has_local(&lab, "local_slm")? {
        bail!("refuse:binding: local_slm missing after dual apply");
    }
    println!("local_slm kept");

    record_dual_scope(&lab)?;
    ensure_locked(&locked, &before)?;
    let join_ag = refresh_join(&lab, &prepared_ag, FUNCTION)?;
    let join_rust = refresh_join(&lab, &prepared_rust, FUNCTION_RUST)?;
    if !join_ag.joinable || join_ag.binding_id != BINDING_AG {
        bail!(
            "refuse:specialty-join: ag_news joinable={} binding={}",
            join_ag.joinable,
            join_ag.binding_id
        );
    }
    if !join_rust.joinable || join_rust.binding_id != BINDING_RUST {
        bail!(
            "refuse:specialty-join: rust_idiom joinable={} binding={}",
            join_rust.joinable,
            join_rust.binding_id
        );
    }
    println!("{}", join_ag.status_line());
    println!("{}", join_rust.status_line());
    crate::enrich::cmd_enrich_standing_next(&lab, &prepared_ag)?;
    crate::enrich::cmd_enrich_standing_next(&lab, &prepared_rust)?;

    let landed = DualSpecialtyLab {
        lab: lab.clone(),
        state: state.clone(),
        prepared_ag,
        prepared_rust,
        gguf_ag,
        gguf_rust,
    };
    if !prove {
        return Ok(Some(landed));
    }

    mock_complete(&lab, &state, "research", "ping ag_news specialty")?;
    mock_complete(&lab, &state, "idiom", "ping rust idiom specialty")?;
    match mock_complete(&lab, &state, "sanctum", "ping both specialties") {
        Ok(()) => bail!("refuse:decision: sanctum listed two specialty ids and did not abstain"),
        Err(err) => {
            let text = format!("{err:#}");
            if !text.contains("refuse:decision-abstain") {
                bail!("refuse:decision: sanctum abstain failed: {text}");
            }
            println!("abstain: sanctum two specialty ids result=abstain");
        }
    }
    print_abstain_receipt(&state)?;
    crate::decisions::cmd_decisions_report(&state, None)?;
    crate::watch::cmd_status(
        &lab,
        &state,
        &roots,
        &plans,
        &packs_dir,
        &policy,
        &root,
    )?;

    let cksum_after = file_cksum(&locked)?;
    if cksum_after != cksum_before || fs::read(&locked)? != before {
        bail!("refuse:estate: examples/estate.yaml changed");
    }
    let report = serde_json::json!({
        "schema": DUAL_SCHEMA,
        "ok": true,
        "binding_ids": [BINDING_AG, BINDING_RUST],
        "class": "local",
        "functions": [FUNCTION, FUNCTION_RUST],
        "trained_shape": "gguf",
        "auto_apply": false,
        "joinable": {
            BINDING_AG: true,
            BINDING_RUST: true,
        },
        "seat_models": {
            BINDING_AG: SEAT_MODEL,
            BINDING_RUST: SEAT_RUST,
        },
        "agents": {
            BINDING_AG: "research",
            BINDING_RUST: "idiom",
        },
        "local_slm": "present",
        "abstain_agent": "sanctum",
        "abstain_candidates": [BINDING_AG, BINDING_RUST],
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "estate_cksum": cksum_after,
        "note": "Fixture prove. Two mocked GGUFs. No GPU train. local_slm stays. Not a live PASS."
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    println!("specialty-bind-prove-dual: ok");
    println!("READY_FOR_LIVE_TEST: no");
    Ok(None)
}

fn prepare_lora(lab: &Path, pack: &Path, packs_dir: &Path, state: &Path) -> Result<()> {
    crate::enrich::cmd_enrich_prepare(
        lab,
        pack,
        packs_dir,
        Some(DRIVER_LORA),
        false,
        None,
        state,
        Some("train"),
        "jason",
        None,
        false,
        false,
    )?;
    Ok(())
}

fn prepared_lora(state: &Path, pack_id: &str) -> PathBuf {
    state.join("enrich").join(pack_id).join(DRIVER_LORA)
}

fn land_specialty(
    lab: &Path,
    prepared: &Path,
    gguf: &Path,
    tag: &str,
    binding_id: &str,
    seat_model: &str,
    function: &str,
    agent: &str,
    state: &Path,
    plans: &Path,
    roots: &Path,
    packs_dir: &Path,
    policy: &Path,
    locked: &Path,
    before: &[u8],
) -> Result<()> {
    crate::enrich::cmd_enrich_import_trained(
        lab,
        prepared,
        tag,
        gguf,
        "jason",
        Some(binding_id),
        Some(seat_model),
        Some(seat_model),
        Some(function),
    )?;
    ensure_locked(locked, before)?;
    let proposal: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(prepared.join("binding-proposal.json"))
            .with_context(|| format!("refuse:proposal: {}", prepared.display()))?,
    )?;
    if proposal["trained_shape"] != "gguf"
        || proposal["auto_apply"] != false
        || proposal["binding_id"] != binding_id
        || proposal["estate_rewritten"] != false
    {
        bail!("refuse:proposal: {binding_id} import did not record gguf auto_apply=false");
    }
    let join: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(prepared.join("specialty-join.json"))
            .with_context(|| format!("refuse:specialty-join: {}", prepared.display()))?,
    )?;
    if join["schema"] != model_estate::SPECIALTY_JOIN_SCHEMA
        || join["binding_id"] != binding_id
        || join["function"] != function
        || join["trained_shape"] != "gguf"
        || join["auto_apply"] != false
        || join["ready_for_live_test"] != false
        || join["live_pass_recorded"] != false
    {
        bail!("refuse:specialty-join: {binding_id} record is not a gguf specialty join");
    }
    println!("specialty-join recorded: binding={binding_id} function={function} agent={agent}");

    crate::enrich::cmd_enrich_apply_proposal(lab, prepared, tag, state, plans, "jason", false)?;
    let staged = state.join("enrich-stage/staged-estate.yaml");
    crate::plan_apply::cmd_plan(staged.as_path(), None, plans, state, false, &plans.join("reviewed"))?;
    crate::plan_apply::cmd_apply(
        staged.as_path(),
        state,
        roots,
        plans,
        true,
        None,
        packs_dir,
        false,
        false,
        policy,
        "jason",
        false,
    )?;
    ensure_locked(locked, before)?;
    let applied = estate_schema::load_estate(lab)
        .with_context(|| format!("refuse:estate: load {}", lab.display()))?;
    let binding = applied
        .model_bindings
        .iter()
        .find(|row| row.id == binding_id)
        .ok_or_else(|| anyhow::anyhow!("refuse:binding: {binding_id} missing after apply"))?;
    let model = binding
        .params
        .get("model")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if binding.class != estate_schema::ModelClass::Local || model != seat_model {
        bail!("refuse:binding: {binding_id} model is '{model}', want {seat_model}");
    }
    if !applied
        .model_bindings
        .iter()
        .any(|row| row.id == "local_slm" && row.class == estate_schema::ModelClass::Local)
    {
        bail!("refuse:binding: local_slm missing after {binding_id}");
    }
    println!("binding present: {binding_id} class=local model={model}");
    Ok(())
}

fn seed_idiom_agent(lab: &Path) -> Result<()> {
    let mut estate = estate_schema::load_estate(lab)
        .with_context(|| format!("refuse:estate: load {}", lab.display()))?;
    if estate.agent("idiom").is_none() {
        estate.agents.push(estate_schema::Agent {
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
        estate.lanes.push(estate_schema::Lane {
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
    }
    let yaml = estate_schema::render_estate_yaml(&estate)?;
    fs::write(lab, yaml)?;
    println!("lab agent: idiom lane=idiom");
    Ok(())
}

fn record_dual_scope(lab: &Path) -> Result<()> {
    let mut estate = estate_schema::load_estate(lab)
        .with_context(|| format!("refuse:estate: load {}", lab.display()))?;
    for id in ["local_slm", BINDING_AG, BINDING_RUST] {
        if !estate.model_bindings.iter().any(|row| row.id == id) {
            bail!("refuse:binding: {id} missing before allow-lists");
        }
    }
    set_models(
        &mut estate,
        "research",
        &[BINDING_AG],
        "Lab copy. Research completes on the ag_news specialty seat.",
    )?;
    set_models(
        &mut estate,
        "idiom",
        &[BINDING_RUST],
        "Lab copy. Idiom completes on the rust_idiom specialty seat.",
    )?;
    set_models(
        &mut estate,
        "sanctum",
        &[BINDING_AG, BINDING_RUST],
        "Lab copy. Two specialty ids. The selector abstains.",
    )?;
    allow_model(
        &mut estate,
        "research",
        BINDING_AG,
        "Lab copy. Research may complete on ag_news.",
    );
    allow_model(
        &mut estate,
        "idiom",
        BINDING_RUST,
        "Lab copy. Idiom may complete on rust_idiom.",
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
    let yaml = estate_schema::render_estate_yaml(&estate)?;
    fs::write(lab, yaml)?;
    println!("lab intention: research model ag_news effect=allow");
    println!("lab intention: idiom model rust_idiom effect=allow");
    println!("lab intention: sanctum model ag_news,rust_idiom effect=allow");
    Ok(())
}

fn set_models(estate: &mut estate_schema::Estate, agent_id: &str, ids: &[&str], note: &str) -> Result<()> {
    let agent = estate
        .agents
        .iter_mut()
        .find(|row| row.id == agent_id)
        .ok_or_else(|| anyhow::anyhow!("refuse:agent: {agent_id} missing on lab estate"))?;
    agent.models = ids
        .iter()
        .map(|id| estate_schema::ModelUseDecl {
            id: (*id).to_string(),
            description: Some(note.to_string()),
        })
        .collect();
    Ok(())
}

fn allow_model(estate: &mut estate_schema::Estate, agent: &str, object: &str, note: &str) {
    estate.intentions.push(estate_schema::Intention {
        subject_agent: agent.to_string(),
        object: object.to_string(),
        kind: estate_schema::IntentionKind::Model,
        effect: estate_schema::Effect::Allow,
        note: Some(note.to_string()),
    });
}

fn refresh_join(
    lab: &Path,
    prepared: &Path,
    function: &str,
) -> Result<model_estate::SpecialtyJoin> {
    let estate = estate_schema::load_estate(lab)?;
    let text = fs::read_to_string(prepared.join("binding-proposal.json"))?;
    let proposal: model_estate::EnrichBindingProposal = serde_json::from_str(&text)
        .with_context(|| format!("refuse:proposal: {}", prepared.display()))?;
    model_estate::write_specialty_join(&estate, prepared, &proposal, Some(function))
        .map_err(|err| anyhow::anyhow!("{err}"))
}

fn mock_complete(lab: &Path, state: &Path, agent: &str, prompt: &str) -> Result<()> {
    crate::ops::cmd_complete(
        agent,
        Some(prompt.to_string()),
        None,
        None,
        None,
        None,
        None,
        lab,
        state,
        None,
        None,
        true,
        None,
        None,
        None,
        false,
    )
}

fn print_abstain_receipt(state: &Path) -> Result<()> {
    let path = state.join("decisions").join("receipts.jsonl");
    let text = fs::read_to_string(&path)
        .with_context(|| format!("refuse:decision: {}", path.display()))?;
    let mut found = false;
    for line in text.lines().filter(|line| !line.is_empty()) {
        let row: serde_json::Value = serde_json::from_str(line)
            .with_context(|| format!("refuse:decision: {line}"))?;
        if row["agent"] != "sanctum" || row["result"] != "abstain" {
            continue;
        }
        let empty = Vec::new();
        let listed = row["candidates"].as_array().unwrap_or(&empty);
        let ids: Vec<&str> = listed
            .iter()
            .filter_map(|item| item.get("id").and_then(|id| id.as_str()))
            .collect();
        if ids.len() != 2 || !ids.contains(&BINDING_AG) || !ids.contains(&BINDING_RUST) {
            bail!("refuse:decision: sanctum candidates are {ids:?}, want ag_news and rust_idiom");
        }
        println!(
            "abstain receipt: agent=sanctum result=abstain capability={} candidates={}",
            row["capability"].as_str().unwrap_or(""),
            ids.join(",")
        );
        found = true;
    }
    if !found {
        bail!("refuse:decision: sanctum abstain receipt missing");
    }
    Ok(())
}

fn local_slm_params(lab: &Path) -> Result<serde_json::Value> {
    let estate = estate_schema::load_estate(lab)?;
    let binding = estate
        .model_bindings
        .iter()
        .find(|row| row.id == "local_slm")
        .ok_or_else(|| anyhow::anyhow!("refuse:binding: local_slm missing"))?;
    Ok(binding.params.clone())
}

fn estate_has_local(lab: &Path, id: &str) -> Result<bool> {
    let estate = estate_schema::load_estate(lab)?;
    Ok(estate.model_bindings.iter().any(|row| {
        row.id == id && row.class == estate_schema::ModelClass::Local
    }))
}

fn write_lab_pack(dir: &Path, source: &Path, id: &str) -> Result<PathBuf> {
    let text = fs::read_to_string(source)
        .with_context(|| format!("refuse:pack: {}", source.display()))?;
    let from = "\"id\": \"overnight-traces\"";
    if !text.contains(from) {
        bail!("refuse:pack: overnight fixture id moved");
    }
    let hint = "\"model_hint\": \"local_slm\"";
    if !text.contains(hint) {
        bail!("refuse:pack: overnight fixture model_hint moved");
    }
    let body = text
        .replacen(from, &format!("\"id\": \"{id}\""), 1)
        .replacen(
            hint,
            "\"model_hint\": \"llama3\",\n  \"train_base_model\": \"Qwen/Qwen2.5-0.5B-Instruct\"",
            1,
        );
    let path = dir.join(format!("{id}.pack.json"));
    fs::write(&path, body)?;
    Ok(path)
}

fn ensure_locked(locked: &Path, before: &[u8]) -> Result<()> {
    if fs::read(locked)? != before {
        bail!("refuse:estate: examples/estate.yaml bytes changed");
    }
    Ok(())
}

fn default_out_dual() -> PathBuf {
    let mut token = std::process::id().to_string();
    for needle in ["5090", "4090", "4080", "3090"] {
        token = token.replace(needle, "0000");
    }
    std::env::temp_dir().join(format!("cell-one-specialty-bind-dual-{token}"))
}

fn write_lab_estate(dir: &Path, source: &[u8]) -> Result<PathBuf> {
    let needle = "  - id: local_slm\n    class: local\n    driver: ollama\n    params:\n";
    let src = std::str::from_utf8(source).context("refuse:estate: examples/estate.yaml is not utf-8")?;
    if !src.contains(needle) {
        bail!("refuse:estate: local_slm params block moved");
    }
    if !src.contains("intentions: []\n") {
        bail!("refuse:estate: intentions block moved");
    }
    let insert = format!(
        "{needle}      model: \"llama3\"\n      train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"\n"
    );
    let with_seat = src.replacen(needle, &insert, 1);
    // Deny-default stays on the locked estate. The lab copy records the allow
    // that lets research complete on local_slm and leave a receipt.
    let intention = "\
intentions:
  - subject_agent: research
    object: local_slm
    kind: model
    effect: allow
    note: Lab copy. Research may complete on the joinable local specialty seat.
";
    let path = dir.join("lab-estate.yaml");
    fs::write(&path, with_seat.replacen("intentions: []\n", intention, 1))?;
    println!("lab intention: research model local_slm effect=allow");
    Ok(path)
}

fn default_out() -> PathBuf {
    let mut token = std::process::id().to_string();
    for needle in ["5090", "4090", "4080", "3090"] {
        token = token.replace(needle, "0000");
    }
    std::env::temp_dir().join(format!("cell-one-specialty-bind-{token}"))
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
