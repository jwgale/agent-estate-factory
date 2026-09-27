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

pub(crate) fn cmd_enrich_bind_prove(root: &Path, out: Option<&Path>) -> Result<()> {
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
        None,
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
