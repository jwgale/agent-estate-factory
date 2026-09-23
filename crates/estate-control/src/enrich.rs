//! Operator surface for train/enrich prepare, list, and the binding proposal.
//! Control does not train. The data plane writes artifacts. Apply stays a later command.

use anyhow::{bail, Context, Result};
use estate_schema::load_estate_unvalidated;
use model_estate::{
    default_enrich_out, default_train_enrich_driver_id, driver_default_job, import_prepared,
    import_trained, list_prepared, load_enrich_pack, prepare_enrich_set, render_prepared_index,
    render_train_enrich_catalog, train_enrich_drivers_for_job, ImportPreparedRequest,
    ImportTrainedRequest, PrepareEnrichRequest,
};
use std::path::{Path, PathBuf};

pub(crate) fn cmd_enrich_drivers() -> Result<()> {
    println!("{}", render_train_enrich_catalog());
    Ok(())
}

pub(crate) fn cmd_enrich_prepare(
    estate_path: &Path,
    pack: &Path,
    packs_dir: &Path,
    driver: Option<&str>,
    all_drivers: bool,
    out: Option<&Path>,
    state_dir: &Path,
    job: Option<&str>,
    curator: &str,
    max_steps: Option<u32>,
) -> Result<()> {
    if all_drivers && driver.is_some() {
        bail!("refuse:driver: pass --driver or --all-drivers");
    }
    let job = resolve_prepare_job(driver, all_drivers, job)?;
    let before = std::fs::read_to_string(estate_path)
        .with_context(|| format!("refuse:estate: read {}", estate_path.display()))?;
    let estate = load_estate_unvalidated(estate_path)
        .with_context(|| format!("refuse:estate: load {}", estate_path.display()))?;
    let manifest = load_enrich_pack(pack, packs_dir)?;
    let targets = prepare_targets(driver, all_drivers, out, state_dir, &manifest.id, &job)?;
    let reqs: Vec<PrepareEnrichRequest<'_>> = targets
        .iter()
        .map(|(driver_id, out_dir)| PrepareEnrichRequest {
            estate: &estate,
            pack: &manifest,
            curator,
            driver_id,
            job: job.as_str(),
            out_dir,
            max_steps,
        })
        .collect();
    let docs = prepare_enrich_set(&reqs)?;
    let after = std::fs::read_to_string(estate_path)
        .with_context(|| format!("refuse:estate: read {}", estate_path.display()))?;
    if before != after {
        bail!("enrich prepare must not rewrite the estate file");
    }
    for (doc, (_, out_dir)) in docs.iter().zip(targets.iter()) {
        let train_base = match doc.train_base_model.as_deref() {
            Some(train) => format!(" train_base={train}"),
            None => String::new(),
        };
        println!(
            "enrich prepare: driver={} job={} pack={} base={}{train_base}",
            doc.driver, doc.job, doc.pack_id, doc.base_model
        );
        println!("  out: {}", out_dir.display());
        println!("  artifacts: {}", doc.artifacts.join(","));
        println!(
            "promoted={} auto_apply={} estate_rewritten={}",
            doc.promoted, doc.auto_apply, doc.estate_rewritten
        );
        let next_path = out_dir.join("NEXT.md");
        let next = std::fs::read_to_string(&next_path)
            .with_context(|| format!("refuse:prepare: read {}", next_path.display()))?;
        println!("{next}");
    }
    println!("prepared={}", docs.len());
    Ok(())
}

/// One command after an accepted pack: prepare into `{state_dir}/enrich`. Does not apply.
pub(crate) fn cmd_enrich_from_pack(
    estate_path: &Path,
    pack: &Path,
    packs_dir: &Path,
    driver: Option<&str>,
    all_drivers: bool,
    state_dir: &Path,
    job: Option<&str>,
    curator: &str,
    max_steps: Option<u32>,
) -> Result<()> {
    if all_drivers && driver.is_some() {
        bail!("refuse:driver: pass --driver or --all-drivers");
    }
    let all = driver.is_none();
    println!(
        "enrich from-pack: state={}/enrich (does not apply, does not train)",
        state_dir.display()
    );
    cmd_enrich_prepare(
        estate_path,
        pack,
        packs_dir,
        driver,
        all,
        None,
        state_dir,
        job,
        curator,
        max_steps,
    )
}

pub(crate) fn cmd_enrich_list(state_dir: &Path) -> Result<()> {
    let root = state_dir.join("enrich");
    let rows = list_prepared(&root)?;
    println!("{}", render_prepared_index(&root, &rows));
    Ok(())
}

pub(crate) fn cmd_enrich_import_prepared(
    estate_path: &Path,
    prepared_dir: &Path,
    tag: &str,
    path: &Path,
    curator: &str,
) -> Result<()> {
    let before = std::fs::read_to_string(estate_path)
        .with_context(|| format!("refuse:estate: read {}", estate_path.display()))?;
    let estate = load_estate_unvalidated(estate_path)
        .with_context(|| format!("refuse:estate: load {}", estate_path.display()))?;
    let proposal = import_prepared(&ImportPreparedRequest {
        estate: &estate,
        prepared_dir,
        tag,
        path,
        curator,
    })?;
    let after = std::fs::read_to_string(estate_path)
        .with_context(|| format!("refuse:estate: read {}", estate_path.display()))?;
    if before != after {
        bail!("enrich import-prepared must not rewrite the estate file");
    }
    let json_path = prepared_dir.join("binding-proposal.json");
    let md_path = prepared_dir.join("binding-proposal.md");
    println!(
        "enrich import-prepared: pack={} binding={} tag={} seated_driver={}",
        proposal.pack_id, proposal.binding_id, proposal.local_tag, proposal.seated_driver
    );
    println!("  prepared: {}", prepared_dir.display());
    println!("  json: {}", json_path.display());
    println!("  md: {}", md_path.display());
    println!(
        "auto_apply={} promoted={} estate_rewritten={}",
        proposal.auto_apply, proposal.promoted, proposal.estate_rewritten
    );
    println!(
        "next: estate enrich apply-proposal --estate {} --prepared {} --tag {} --state-dir <state-dir>",
        estate_path.display(),
        prepared_dir.display(),
        tag
    );
    println!("import-prepared did not apply.");
    Ok(())
}

pub(crate) fn cmd_enrich_import_trained(
    estate_path: &Path,
    prepared_dir: &Path,
    tag: &str,
    adapter: &Path,
    curator: &str,
) -> Result<()> {
    let before = std::fs::read_to_string(estate_path)
        .with_context(|| format!("refuse:estate: read {}", estate_path.display()))?;
    let estate = load_estate_unvalidated(estate_path)
        .with_context(|| format!("refuse:estate: load {}", estate_path.display()))?;
    let proposal = import_trained(&ImportTrainedRequest {
        estate: &estate,
        prepared_dir,
        tag,
        adapter,
        curator,
    })?;
    let after = std::fs::read_to_string(estate_path)
        .with_context(|| format!("refuse:estate: read {}", estate_path.display()))?;
    if before != after {
        bail!("enrich import-trained must not rewrite the estate file");
    }
    let json_path = prepared_dir.join("binding-proposal.json");
    let md_path = prepared_dir.join("binding-proposal.md");
    println!(
        "enrich import-trained: pack={} binding={} tag={} seated_driver={} driver={}",
        proposal.pack_id,
        proposal.binding_id,
        proposal.local_tag,
        proposal.seated_driver,
        proposal.driver
    );
    println!("  prepared: {}", prepared_dir.display());
    println!("  adapter: {}", adapter.display());
    println!("  weights: {}", proposal.local_path);
    println!("  json: {}", json_path.display());
    println!("  md: {}", md_path.display());
    println!(
        "auto_apply={} promoted={} estate_rewritten={}",
        proposal.auto_apply, proposal.promoted, proposal.estate_rewritten
    );
    println!(
        "next: estate enrich apply-proposal --estate {} --prepared {} --tag {} --state-dir <state-dir>",
        estate_path.display(),
        prepared_dir.display(),
        tag
    );
    println!("import-trained did not apply.");
    Ok(())
}

pub(crate) fn cmd_enrich_apply_proposal(
    estate_path: &Path,
    prepared_dir: &Path,
    tag: &str,
    state_dir: &Path,
    plans_dir: &Path,
    curator: &str,
    verify_local_tag: bool,
) -> Result<()> {
    let before = std::fs::read(estate_path)
        .with_context(|| format!("refuse:estate: read {}", estate_path.display()))?;
    let estate = load_estate_unvalidated(estate_path)
        .with_context(|| format!("refuse:estate: load {}", estate_path.display()))?;
    let verify_endpoint = if verify_local_tag {
        Some(seated_endpoint(&estate)?)
    } else {
        None
    };
    let outcome = model_estate::apply_proposal(&model_estate::ApplyProposalRequest {
        estate: &estate,
        estate_path,
        prepared_dir,
        tag,
        curator,
        state_dir,
        verify_endpoint: verify_endpoint.as_deref(),
    })?;
    let after = std::fs::read(estate_path)
        .with_context(|| format!("refuse:estate: read {}", estate_path.display()))?;
    if before != after {
        bail!("enrich apply-proposal must not rewrite the source estate");
    }
    match outcome {
        model_estate::ApplyProposalOutcome::Noop { reason } => {
            println!("{reason}");
            println!("apply-proposal did not apply.");
        }
        model_estate::ApplyProposalOutcome::Staged(stage) => {
            println!(
                "enrich apply-proposal: pack={} binding={} tag={} seated_driver={}",
                stage.pack_id, stage.binding_id, stage.local_tag, stage.seated_driver
            );
            println!("  source: {} (not rewritten)", estate_path.display());
            println!("  staged: {}", stage.staged_estate);
            println!(
                "  stage: {}",
                state_dir.join("enrich-stage").join("stage.json").display()
            );
            println!(
                "auto_apply={} promoted={} estate_rewritten={} applied={}",
                stage.auto_apply, stage.promoted, stage.estate_rewritten, stage.applied
            );
            println!(
                "next: estate plan --estate {} --state-dir {} --plans-dir {}",
                stage.staged_estate,
                state_dir.display(),
                plans_dir.display()
            );
            println!(
                "next: estate apply --estate {} --state-dir {} --plans-dir {} --roots-base . --require-plan",
                stage.staged_estate,
                state_dir.display(),
                plans_dir.display()
            );
            println!("apply-proposal did not apply.");
        }
    }
    Ok(())
}

fn seated_endpoint(estate: &estate_schema::Estate) -> Result<String> {
    let seat = estate
        .model_bindings
        .iter()
        .find(|binding| binding.id == "local_slm")
        .ok_or_else(|| anyhow::anyhow!("refuse:binding: estate has no local_slm seat"))?;
    let env_name = seat
        .params
        .get("endpoint_env")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("CELL_LOCAL_ENDPOINT");
    if estate_schema::contains_sku(env_name) || estate_schema::is_sacred_name(env_name) {
        bail!("refuse:local-tag: endpoint env '{env_name}' is not a seated endpoint name");
    }
    match std::env::var(env_name) {
        Ok(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        _ => bail!(
            "refuse:local-tag: {env_name} is unset; --verify-local-tag needs the seated endpoint"
        ),
    }
}

fn resolve_prepare_job(
    driver: Option<&str>,
    all_drivers: bool,
    job: Option<&str>,
) -> Result<String> {
    if let Some(job) = job.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(job.to_string());
    }
    if all_drivers || driver.is_none() {
        return Ok("enrich".to_string());
    }
    Ok(driver_default_job(driver.unwrap_or_default())?.to_string())
}

fn prepare_targets(
    driver: Option<&str>,
    all_drivers: bool,
    out: Option<&Path>,
    state_dir: &Path,
    pack_id: &str,
    job: &str,
) -> Result<Vec<(String, PathBuf)>> {
    if all_drivers {
        let mut targets = Vec::new();
        for driver_id in train_enrich_drivers_for_job(job)? {
            let dir = match out {
                Some(parent) => parent.join(driver_id),
                None => default_enrich_out(state_dir, pack_id, driver_id),
            };
            targets.push((driver_id.to_string(), dir));
        }
        if targets.is_empty() {
            bail!("refuse:driver: no train/enrich drivers to prepare");
        }
        return Ok(targets);
    }
    let driver_id = match driver {
        Some(id) => id.to_string(),
        None => default_train_enrich_driver_id().to_string(),
    };
    let dir = match out {
        Some(path) => path.to_path_buf(),
        None => default_enrich_out(state_dir, pack_id, &driver_id),
    };
    Ok(vec![(driver_id, dir)])
}
