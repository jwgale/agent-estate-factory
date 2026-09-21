use anyhow::{bail, Context, Result};
use conveyor_proxy::{
    hop_now_unix, list_expired_hop_leases,
};
use estate_schema::{
    estate_hash, latest_plan, load_estate,
};
use feed_collector::list_open_proposals;
use floor_supervisor::{
    drift_with_roots,
    forget_expired_leases, list_apply_audits, list_expired_leases, load_lifecycle, load_placements,
    now_unix,
};
use std::path::Path;

pub(crate) fn cmd_expire(state_dir: &Path, forget: bool) -> Result<()> {
    let expired = list_expired_leases(state_dir, now_unix())?;
    if expired.is_empty() {
        println!("no expired leases under {}", state_dir.display());
        return Ok(());
    }
    println!("expired leases ({})", expired.len());
    for lease in &expired {
        println!(
            "  refuse:expired: {} kind={} expires_at={}",
            lease.placement_id,
            lease.kind,
            lease
                .expires_at
                .map(|e| e.to_string())
                .unwrap_or_else(|| "-".into())
        );
    }
    if forget {
        let forgotten = forget_expired_leases(state_dir)?;
        println!("forgot {} expired lease(s); apply may record fresh rows", forgotten.len());
        return Ok(());
    }
    bail!("refuse:expired: apply/resume refuse until estate expire --forget");
}

const DOCTOR_REQUIRED: &[&str] = &[
    "schema/estate.v0.schema.json",
    "schema/pack.v0.json",
    "schema/specialist-pack.v0.json",
    "schema/placement-actual.v0.json",
    "schema/reconcile.v0.json",
    "schema/conveyor-mesh.v0.json",
    "schema/local-catalog.v0.json",
    "schema/lifecycle.v0.json",
    "schema/estate-plan.v0.json",
    "schema/feed-cursor.v0.json",
    "schema/enrich-proposal.v0.json",
    "schema/apply-dry-run.v0.json",
    "schema/session-journal.v0.json",
    "schema/policy.v0.json",
    "schema/cell-backup.v0.json",
    "schema/sacred.v0.json",
    "policy/cell-one.policy.v0.yaml",
    "policy/sacred.yaml",
    "schema/README.md",
    "docs/GATE-90.md",
    "CHANGELOG.md",
    "docs/MORNING-BRIEF-2026-09-21.md",
    "docs/PR2-DESCRIPTION.md",
];

pub(crate) fn cmd_doctor(root: &Path, state_dir: &Path) -> Result<()> {
    let mut fails: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    println!("Cell One doctor\n===============");
    println!("root: {}", root.display());
    println!("state: {}\n", state_dir.display());

    println!("Schema files");
    println!("------------");
    for rel in DOCTOR_REQUIRED {
        let path = root.join(rel);
        if path.is_file() {
            println!("  ok    {rel}");
        } else {
            println!("  FAIL  {rel}");
            fails.push(format!("missing {rel}"));
        }
    }

    println!("\nHosted CI");
    println!("---------");
    match hosted_ci_status(root) {
        HostedCiStatus::CompileOnly => {
            println!("  ok    .github/workflows/ci.yml (compile-only)");
        }
        HostedCiStatus::Missing => {
            println!("  FAIL  missing .github/workflows/ci.yml");
            fails.push("missing compile-only ci.yml".into());
        }
        HostedCiStatus::Extra(extra) => {
            for name in extra {
                println!("  FAIL  .github/workflows/{name}");
                fails.push(format!("extra workflow: {name}"));
            }
        }
    }

    println!("\n.cell layout");
    println!("------------");
    let layout = [
        ("placement-actual.json", "durable lease"),
        ("lifecycle.json", "durable"),
        ("lifecycle.jsonl", "durable history"),
        ("apply-audit.jsonl", "durable"),
        ("sessions.jsonl", "durable journal"),
        ("conveyor-leases.json", "durable"),
        ("reconcile.json", "regenerable"),
        ("actual-state.json", "regenerable"),
        ("desired-snapshot.yaml", "regenerable"),
        ("catalog.json", "regenerable"),
        ("sessions/", "disposable"),
        ("runtime/", "disposable"),
    ];
    if !state_dir.exists() {
        println!("  note  {} missing (greenfield ok)", state_dir.display());
        notes.push("greenfield .cell".into());
    } else {
        for (name, kind) in layout {
            let path = state_dir.join(name.trim_end_matches('/'));
            let present = if name.ends_with('/') {
                path.is_dir()
            } else {
                path.is_file()
            };
            println!(
                "  {:<6} {:<24} {}",
                if present { "ok" } else { "note" },
                name,
                kind
            );
        }
    }

    println!("\nLeases");
    println!("------");
    match load_placements(state_dir) {
        Ok(Some(places)) => {
            for lease in &places.leases {
                if lease.kind == "cloud-agent" && lease.spawned {
                    println!("  FAIL  {} spawned cloud-agent", lease.placement_id);
                    fails.push("cloud-agent spawned".into());
                }
            }
            let expired = list_expired_leases(state_dir, now_unix())?;
            if expired.is_empty() {
                println!("  ok    no expired leases");
            } else {
                for lease in &expired {
                    println!("  FAIL  refuse:expired: {}", lease.placement_id);
                    fails.push(format!("expired {}", lease.placement_id));
                }
            }
            match list_expired_hop_leases(state_dir, hop_now_unix()) {
                Ok(hops) if hops.is_empty() => println!("  ok    no expired hop leases"),
                Ok(hops) => {
                    for hop in hops {
                        println!("  FAIL  refuse:expired: hop {}", hop.hop_id);
                        fails.push(format!("expired hop {}", hop.hop_id));
                    }
                }
                Err(err) => {
                    println!("  note  hop leases: {err}");
                }
            }
        }
        Ok(None) => println!("  note  no placement-actual.json"),
        Err(err) => {
            println!("  FAIL  {err}");
            fails.push(err.to_string());
        }
    }

    println!("\nHealth");
    println!("------");
    if fails.is_empty() {
        println!("  ok    factory ready (compile-only CI; schemas present)");
        for note in notes {
            println!("  note  {note}");
        }
        Ok(())
    } else {
        for fail in &fails {
            println!("  FAIL  {fail}");
        }
        bail!("doctor failed ({} check(s))", fails.len());
    }
}

pub(crate) fn cmd_status(
    path: &Path,
    state_dir: &Path,
    roots_base: &Path,
    plans_dir: &Path,
    packs_dir: &Path,
    policy: &Path,
    root: &Path,
) -> Result<()> {
    let estate = load_estate(path).with_context(|| format!("load {}", path.display()))?;
    let life = load_lifecycle(state_dir)?;
    let report = drift_with_roots(&estate, state_dir, Some(roots_base))?;
    let places = load_placements(state_dir)?;
    let lease_n = places.as_ref().map(|p| p.leases.len()).unwrap_or(0);
    let box_n = places
        .as_ref()
        .map(|p| p.leases.iter().filter(|l| l.kind == "box").count())
        .unwrap_or(0);
    let cloud_n = places
        .as_ref()
        .map(|p| {
            p.leases
                .iter()
                .filter(|l| l.kind == "cloud-agent")
                .count()
        })
        .unwrap_or(0);
    let spawned_n = places
        .as_ref()
        .map(|p| p.leases.iter().filter(|l| l.spawned).count())
        .unwrap_or(0);
    let expired_n = list_expired_leases(state_dir, now_unix())
        .map(|v| v.len())
        .unwrap_or(0);
    let hop_expired_n = list_expired_hop_leases(state_dir, hop_now_unix())
        .map(|v| v.len())
        .unwrap_or(0);
    let last_plan = latest_plan(plans_dir)
        .map(|p| p.desired_hash)
        .unwrap_or_else(|| "-".into());
    let audits = list_apply_audits(state_dir)?;
    let last_apply = audits.last().map(|a| {
        format!(
            "{} covering={}",
            a.desired_hash,
            a.covering_plan.as_deref().unwrap_or("-")
        )
    });
    let proposals = list_open_proposals(&packs_dir.join("proposed")).unwrap_or_default();
    let policy_present = policy.is_file();
    let doctor = doctor_summary_line(root, state_dir);
    let paused = life.state == floor_supervisor::LifecycleState::Suspended;

    println!("Cell One status");
    println!("===============");
    println!("estate: {} ({})", estate.name, path.display());
    println!("hash: {}", estate_hash(&estate));
    println!("paused: {}", if paused { "yes" } else { "no" });
    println!("lifecycle: {} (durable={})", life.state.as_str(), life.durable);
    println!(
        "leases: {lease_n} (box={box_n} cloud-agent={cloud_n} spawned={spawned_n})"
    );
    println!("expired: placement={expired_n} hop={hop_expired_n}");
    println!("last_plan: {last_plan}");
    println!(
        "last_apply: {}",
        last_apply.as_deref().unwrap_or("-")
    );
    println!(
        "open_proposals: {} ({})",
        proposals.len(),
        if proposals.is_empty() {
            "-".into()
        } else {
            proposals.join(",")
        }
    );
    println!(
        "policy: {} ({})",
        if policy_present { "present" } else { "missing" },
        policy.display()
    );
    println!("doctor: {doctor}");
    println!("in_sync: {}", report.in_sync);
    println!("cloud-agent: declared, not spawned");
    if !report.spawned_cloud_agents.is_empty() {
        bail!(
            "cloud-agent lease spawned (fail closed): {}",
            report.spawned_cloud_agents.join(", ")
        );
    }
    Ok(())
}

pub(crate) fn doctor_summary_line(root: &Path, state_dir: &Path) -> String {
    let mut fails = 0usize;
    for rel in DOCTOR_REQUIRED {
        if !root.join(rel).is_file() {
            fails += 1;
        }
    }
    match hosted_ci_status(root) {
        HostedCiStatus::CompileOnly => {}
        HostedCiStatus::Missing => fails += 1,
        HostedCiStatus::Extra(extra) => fails += extra.len(),
    }
    if let Ok(Some(places)) = load_placements(state_dir) {
        if places
            .leases
            .iter()
            .any(|l| l.kind == "cloud-agent" && l.spawned)
        {
            fails += 1;
        }
    }
    if fails == 0 {
        "ok (compile-only CI; schemas present)".into()
    } else {
        format!("FAIL ({fails})")
    }
}

const ALLOWED_WORKFLOW: &str = "ci.yml";

#[derive(Debug, PartialEq, Eq)]
enum HostedCiStatus {
    CompileOnly,
    Missing,
    Extra(Vec<String>),
}

fn list_workflow_files(root: &Path) -> Vec<String> {
    let wf = root.join(".github/workflows");
    let mut names = Vec::new();
    if wf.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&wf) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy().into_owned();
                if name.ends_with(".yml") || name.ends_with(".yaml") {
                    names.push(name);
                }
            }
        }
    }
    names.sort();
    names
}

fn hosted_ci_status(root: &Path) -> HostedCiStatus {
    let names = list_workflow_files(root);
    let extra: Vec<String> = names
        .iter()
        .filter(|n| n.as_str() != ALLOWED_WORKFLOW)
        .cloned()
        .collect();
    if !extra.is_empty() {
        return HostedCiStatus::Extra(extra);
    }
    if names.iter().any(|n| n == ALLOWED_WORKFLOW) {
        HostedCiStatus::CompileOnly
    } else {
        HostedCiStatus::Missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_root() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env.temp_dir().join(format!("cell-doctor-ci-{nanos}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".github/workflows")).unwrap();
        root
    }

    #[test]
    fn hosted_ci_allows_only_compile_only_yml() {
        let root = tmp_root();
        assert_eq!(hosted_ci_status(&root), HostedCiStatus::Missing);
        fs::write(root.join(".github/workflows/ci.yml"), "name: check\n").unwrap();
        assert_eq!(hosted_ci_status(&root), HostedCiStatus::CompileOnly);
        fs::write(root.join(".github/workflows/extra.yml"), "name: extra\n").unwrap();
        match hosted_ci_status(&root) {
            HostedCiStatus::Extra(extra) => assert_eq!(extra, vec!["extra.yml".to_string()]),
            other => panic!("{other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }
}

