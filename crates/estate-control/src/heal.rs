//! Day 90+ heal / accept / probe polish. Never auto-applies. Never rewrites the estate.

use anyhow::{bail, Context, Result};
use estate_schema::load_estate;
use feed_collector::LOCKED_CURATOR;
use floor_supervisor::{
    load_placements, reconcile_placements, render_reconcile, write_reconcile,
};
use std::path::Path;

use crate::accept::{accept_proposal, render_accept};
use crate::suggest::{render_suggest, write_suggest};

fn refuse_catalog_probes(probes: &[model_estate::DriverProbe]) -> Result<()> {
    for probe in probes {
        if estate_schema::contains_sku(&probe.driver) {
            bail!(
                "refuse:sku-banned: probe id '{}' encodes a hardware SKU",
                probe.driver
            );
        }
        if !estate_schema::is_host_class(&probe.host_class) {
            bail!(
                "refuse:bad-host-class: probe '{}' host_class '{}' must be consumer-nvidia|apple-silicon|rented-nvidia|any",
                probe.driver,
                probe.host_class
            );
        }
    }
    Ok(())
}

pub(crate) fn cmd_probes(live: bool) -> Result<()> {
    let live = live || model_estate::live_probe_env_requested();
    let probes = if live {
        model_estate::catalog_probes_live()
    } else {
        model_estate::catalog_probes()
    };
    refuse_catalog_probes(&probes)?;
    if live {
        println!("live probe (SKIP without endpoints; not used in CI)");
    }
    for probe in probes {
        println!(
            "  {:<12} status={:<12} bindable={} live_probed={} host_class={}",
            probe.driver, probe.status, probe.bindable, probe.live_probed, probe.host_class
        );
        println!("    {}", probe.note);
    }
    Ok(())
}

pub(crate) fn cmd_reconcile(path: &Path, state_dir: &Path, suggest: bool) -> Result<()> {
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    let before = load_placements(state_dir)?;
    let report = reconcile_placements(&estate, state_dir)?;
    let written = write_reconcile(state_dir, &report)?;
    print!("{}", render_reconcile(&report));
    println!("Wrote {}", written.display());
    if suggest {
        let dest = write_suggest(state_dir, &report)?;
        print!("{}", render_suggest(&report));
        println!("Wrote {} (not applied)", dest.display());
        let after = load_placements(state_dir)?;
        if before != after {
            bail!("reconcile --suggest must not rewrite leases");
        }
    }
    if !report.in_sync {
        bail!("reconcile drift (fail closed)");
    }
    Ok(())
}

pub(crate) fn cmd_packs_accept(
    id: &str,
    proposed_dir: &Path,
    accepted_dir: &Path,
    estate_path: &Path,
    curator: &str,
) -> Result<()> {
    let estate = load_estate(estate_path)
        .with_context(|| format!("load {}", estate_path.display()))?;
    let before = crate::helpers::read_estate_text(estate_path)?;
    let (accept, dest) = accept_proposal(
        proposed_dir,
        accepted_dir,
        id,
        curator,
        &estate.enrich_packs.curator,
        &estate,
    )?;
    let after = crate::helpers::read_estate_text(estate_path)?;
    if before != after {
        bail!("packs accept must not rewrite the estate file");
    }
    if accept.auto_apply || accept.applied_to_estate {
        bail!("accept must stay auto_apply=false and applied_to_estate=false");
    }
    print!("{}", render_accept(&accept));
    println!("Wrote {}", dest.display());
    println!(
        "edit instructions only; curator={} (locked={}). estate unchanged.",
        curator, LOCKED_CURATOR
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::refuse_catalog_probes;
    use model_estate::DriverProbe;

    fn probe(driver: &str, host_class: &str) -> DriverProbe {
        DriverProbe {
            driver: driver.into(),
            runtime: "ollama".into(),
            status: "supported".into(),
            host_class: host_class.into(),
            bindable: true,
            live_probed: false,
            note: "test".into(),
        }
    }

    #[test]
    fn refuse_catalog_probes_checks_all_before_any_print() {
        refuse_catalog_probes(&model_estate::catalog_probes()).unwrap();
        let mixed = vec![
            probe("ollama", "any"),
            probe("not-a-host", "not-a-host"),
        ];
        let err = refuse_catalog_probes(&mixed).unwrap_err();
        let text = err.to_string();
        assert!(
            text.contains("refuse:bad-host-class"),
            "{text}"
        );
    }
}
