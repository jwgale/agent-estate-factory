//! Operator surface for train/enrich prepare, list, and the binding proposal.
//! Control does not train. The data plane writes artifacts. Apply stays a later command.

use anyhow::{bail, Context, Result};
use estate_schema::load_estate_unvalidated;
use model_estate::{
    default_enrich_out, default_train_enrich_driver_id, import_prepared, list_prepared,
    load_enrich_pack, prepare_enrich_set, render_prepared_index, render_train_enrich_catalog,
    train_enrich_catalog, ImportPreparedRequest, PrepareEnrichRequest,
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
    job: &str,
    curator: &str,
) -> Result<()> {
    if all_drivers && driver.is_some() {
        bail!("refuse:driver: pass --driver or --all-drivers");
    }
    let before = std::fs::read_to_string(estate_path)
        .with_context(|| format!("refuse:estate: read {}", estate_path.display()))?;
    let estate = load_estate_unvalidated(estate_path)
        .with_context(|| format!("refuse:estate: load {}", estate_path.display()))?;
    let manifest = load_enrich_pack(pack, packs_dir)?;
    let targets = prepare_targets(driver, all_drivers, out, state_dir, &manifest.id)?;
    let reqs: Vec<PrepareEnrichRequest<'_>> = targets
        .iter()
        .map(|(driver_id, out_dir)| PrepareEnrichRequest {
            estate: &estate,
            pack: &manifest,
            curator,
            driver_id,
            job,
            out_dir,
        })
        .collect();
    let docs = prepare_enrich_set(&reqs)?;
    let after = std::fs::read_to_string(estate_path)
        .with_context(|| format!("refuse:estate: read {}", estate_path.display()))?;
    if before != after {
        bail!("enrich prepare must not rewrite the estate file");
    }
    for (doc, (_, out_dir)) in docs.iter().zip(targets.iter()) {
        println!(
            "enrich prepare: driver={} job={} pack={}",
            doc.driver, doc.job, doc.pack_id
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
    println!("paste the local_slm snippet, then estate plan and estate apply --require-plan.");
    println!("import-prepared did not apply.");
    Ok(())
}

fn prepare_targets(
    driver: Option<&str>,
    all_drivers: bool,
    out: Option<&Path>,
    state_dir: &Path,
    pack_id: &str,
) -> Result<Vec<(String, PathBuf)>> {
    if all_drivers {
        let mut targets = Vec::new();
        for card in train_enrich_catalog() {
            let dir = match out {
                Some(parent) => parent.join(card.driver_id),
                None => default_enrich_out(state_dir, pack_id, card.driver_id),
            };
            targets.push((card.driver_id.to_string(), dir));
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
