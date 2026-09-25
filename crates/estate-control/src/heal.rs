//! Day 90+ heal / accept / probe polish. Never auto-applies. Never rewrites the estate.

use anyhow::{bail, Context, Result};
use conveyor_proxy::{
    authority_report, describe_authority_section, load_mesh, refuse_mesh_host_classes,
};
use estate_schema::{describe_agents_section, load_estate, load_estate_unvalidated};
use feed_collector::{refuse_accept_frontier_invent, LOCKED_CURATOR};
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

/// Thin data-plane delegate. Env-gated. Fail-closed. Not a gateway.
pub(crate) fn cmd_specialist(
    endpoint: Option<String>,
    driver: &str,
    job: &str,
    agent: &str,
    kind: &str,
    prompt: Option<String>,
    text: Option<String>,
) -> Result<()> {
    let text = prompt
        .or(text)
        .ok_or_else(|| anyhow::anyhow!("set --prompt or --text"))?;
    let result = model_estate::run_http_specialist(
        endpoint.as_deref(),
        job,
        agent,
        kind,
        &text,
        driver,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    if !result.allow {
        bail!("specialist denied");
    }
    Ok(())
}

pub(crate) fn cmd_reconcile(path: &Path, state_dir: &Path, suggest: bool) -> Result<()> {
    let parsed = load_estate_unvalidated(path)
        .with_context(|| format!("load {}", path.display()))?;
    model_estate::frontier_plan_view(&parsed).map_err(|e| anyhow::anyhow!("{e}"))?;
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    // After the estate load and the frontier refuse, before reconcile.json,
    // the suggest patch, and the placement report. Same stack as status,
    // doctor, and convey authority. Hop cites do not change this command's
    // exit code. Placement drift still bails below.
    print_reconcile_honesty(&estate, state_dir)?;
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

/// Agents, hop coverage cites, then Authority. Print-only.
///
/// A present mesh that does not parse, or a bad `host_class` on that file,
/// refuses before any section. Reconcile does not invent cites, Agents, or
/// Authority rows, and does not write `reconcile.json` or a suggest patch.
/// A missing mesh is the empty mesh from `load_mesh`: the cite list is
/// empty and Authority stays `not-enforced` (`missing-mesh`).
///
/// `authority_report` also reads placement-actual. A placement host class
/// or a population that is ahead of the floor is the placement report's
/// job. This function does not invent sections on that error and does not
/// replace the drift bail. Capability mismatch prints `FAIL`. Deny and
/// deny-default print `note`. A match stays quiet. Those cites are
/// discarded here. Does not rewrite the mesh, the leases, the estate, or
/// the apply audit. Does not spawn. No `enforced` status.
fn print_reconcile_honesty(estate: &estate_schema::Estate, state_dir: &Path) -> Result<()> {
    let mesh = load_mesh(state_dir)?;
    refuse_mesh_host_classes(&mesh)?;
    let Ok(rows) = authority_report(state_dir, estate) else {
        return Ok(());
    };
    println!("{}", describe_agents_section(estate));
    let _mismatches = crate::watch::print_hop_coverage_cites(estate, &mesh);
    println!("{}", describe_authority_section(&rows, state_dir));
    Ok(())
}

pub(crate) fn cmd_packs_accept(
    id: &str,
    proposed_dir: &Path,
    accepted_dir: &Path,
    estate_path: &Path,
    curator: &str,
) -> Result<()> {
    let parsed = load_estate_unvalidated(estate_path)
        .with_context(|| format!("load {}", estate_path.display()))?;
    refuse_accept_frontier_invent(proposed_dir, id, &parsed)?;
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
