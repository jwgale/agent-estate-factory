//! Operator surface for train/enrich prepare.
//! Control does not train. The data plane writes artifacts.

use anyhow::{bail, Context, Result};
use estate_schema::load_estate_unvalidated;
use model_estate::{
    default_enrich_out, load_enrich_pack, prepare_enrich, render_train_enrich_catalog,
    PrepareEnrichRequest,
};
use std::path::Path;

pub(crate) fn cmd_enrich_drivers() -> Result<()> {
    println!("{}", render_train_enrich_catalog());
    Ok(())
}

pub(crate) fn cmd_enrich_prepare(
    estate_path: &Path,
    pack: &Path,
    packs_dir: &Path,
    driver: &str,
    out: Option<&Path>,
    state_dir: &Path,
    job: &str,
    curator: &str,
) -> Result<()> {
    let before = std::fs::read_to_string(estate_path)
        .with_context(|| format!("refuse:estate: read {}", estate_path.display()))?;
    let estate = load_estate_unvalidated(estate_path)
        .with_context(|| format!("refuse:estate: load {}", estate_path.display()))?;
    let manifest = load_enrich_pack(pack, packs_dir)?;
    let owned_out;
    let out_dir = match out {
        Some(path) => path,
        None => {
            owned_out = default_enrich_out(state_dir, &manifest.id, driver);
            &owned_out
        }
    };
    let doc = prepare_enrich(&PrepareEnrichRequest {
        estate: &estate,
        pack: &manifest,
        curator,
        driver_id: driver,
        job,
        out_dir,
    })?;
    let after = std::fs::read_to_string(estate_path)
        .with_context(|| format!("refuse:estate: read {}", estate_path.display()))?;
    if before != after {
        bail!("enrich prepare must not rewrite the estate file");
    }
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
    Ok(())
}
