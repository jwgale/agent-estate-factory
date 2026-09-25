use anyhow::{bail, Context, Result};
use conveyor_proxy::{
    hop_now_unix, list_expired_hop_leases,
};
use estate_schema::{
    describe_declared_coverage, describe_model_class_coverage, estate_hash, latest_plan,
    load_estate,
};
use feed_collector::list_open_proposals;
use floor_supervisor::{
    drift_with_roots,
    forget_expired_leases, list_apply_audits, list_expired_leases, list_lifecycle_events,
    list_session_events, load_actual, load_lifecycle, load_placements,
    now_unix, refuse_lease_host_classes,
};
use std::path::Path;

pub(crate) fn cmd_expire(state_dir: &Path, forget: bool) -> Result<()> {
    let expired = list_expired_leases(state_dir, now_unix())?;
    if expired.is_empty() {
        println!("no expired leases under {}", state_dir.display());
        return Ok(());
    }
    // An expired spawned cloud-agent lease is still spawned. Refuse
    // before the list and before forget rewrites the file. A missing
    // file is not a spawned lease. An expired box still lists.
    let spawned: Vec<&str> = expired
        .iter()
        .filter(|l| l.kind == "cloud-agent" && l.spawned)
        .map(|l| l.placement_id.as_str())
        .collect();
    if !spawned.is_empty() {
        bail!(
            "refuse:cloud-spawned: cloud-agent lease spawned (fail closed): {}",
            spawned.join(", ")
        );
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
    "schema/train-enrich.v0.json",
    "schema/enrich-binding-proposal.v0.json",
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
        }
        Ok(None) => println!("  note  no placement-actual.json"),
        Err(err) => {
            println!("  FAIL  {err}");
            fails.push(err.to_string());
        }
    }
    // Missing mesh is empty, not a failure. A present file that does not
    // parse, or a hop host_class that is not a class, is FAIL. A note
    // would still print "factory ready".
    match list_expired_hop_leases(state_dir, hop_now_unix()) {
        Ok(hops) if hops.is_empty() => println!("  ok    no expired hop leases"),
        Ok(hops) => {
            for hop in hops {
                println!("  FAIL  refuse:expired: hop {}", hop.hop_id);
                fails.push(format!("expired hop {}", hop.hop_id));
            }
        }
        Err(err) => {
            println!("  FAIL  {err}");
            fails.push(err.to_string());
        }
    }
    // Missing lifecycle.json is the default record, not a file. Printing
    // that default would invent suspended. A present file that does not
    // parse is FAIL. A note would still print "factory ready".
    let life_path = floor_supervisor::lifecycle_path(state_dir);
    if life_path.exists() {
        match load_lifecycle(state_dir) {
            Ok(life) => println!("  ok    lifecycle.json state={}", life.state.as_str()),
            Err(err) => {
                println!("  FAIL  {err}");
                fails.push(err.to_string());
            }
        }
    }
    // Missing apply-audit.jsonl is not a failure. Printing a line count
    // for a missing file would invent zero. A present file that does not
    // parse is FAIL. A note would still print "factory ready".
    let audit_path = state_dir.join("apply-audit.jsonl");
    if audit_path.exists() {
        match list_apply_audits(state_dir) {
            Ok(audits) => println!("  ok    apply-audit.jsonl lines={}", audits.len()),
            Err(err) => {
                println!("  FAIL  {err}");
                fails.push(err.to_string());
            }
        }
    }
    // Missing lifecycle.jsonl is not a failure. Printing a line count
    // for a missing file would invent zero. A present file that does not
    // parse is FAIL. A note would still print "factory ready".
    let history_path = state_dir.join("lifecycle.jsonl");
    if history_path.exists() {
        match list_lifecycle_events(state_dir) {
            Ok(events) => println!("  ok    lifecycle.jsonl lines={}", events.len()),
            Err(err) => {
                println!("  FAIL  {err}");
                fails.push(err.to_string());
            }
        }
    }
    // Missing sessions.jsonl is not a failure. Printing a line count
    // for a missing file would invent zero. A present file that does not
    // parse is FAIL. A note would still print "factory ready".
    let sessions_path = state_dir.join("sessions.jsonl");
    if sessions_path.exists() {
        match list_session_events(state_dir) {
            Ok(events) => println!("  ok    sessions.jsonl lines={}", events.len()),
            Err(err) => {
                println!("  FAIL  {err}");
                fails.push(err.to_string());
            }
        }
    }
    // Missing actual-state.json is not a failure. Printing a session
    // count for a missing file would invent zero. A present file that
    // does not parse is FAIL. A note would still print "factory ready".
    let actual_path = state_dir.join("actual-state.json");
    if actual_path.exists() {
        match load_actual(state_dir) {
            Ok(Some(actual)) => {
                println!("  ok    actual-state.json sessions={}", actual.sessions.len())
            }
            Ok(None) => {}
            Err(err) => {
                println!("  FAIL  {err}");
                fails.push(err.to_string());
            }
        }
    }
    // Missing desired-snapshot.yaml is not a failure. When catalog.json
    // is present, the frontier check already reads this file. A snapshot
    // with no catalog that does not parse is still FAIL. A note would
    // still print "factory ready". A parsed file prints its name and
    // does not invent a frontier model.
    let snap_path = state_dir.join("desired-snapshot.yaml");
    if snap_path.exists() && !state_dir.join("catalog.json").is_file() {
        match floor_supervisor::load_desired_snapshot(state_dir) {
            Ok(Some(estate)) => {
                println!("  ok    desired-snapshot.yaml name={}", estate.name)
            }
            Ok(None) => {}
            Err(err) => {
                println!("  FAIL  {err}");
                fails.push(err.to_string());
            }
        }
    }
    // Missing model-actual.json is not a failure. Printing a binding
    // count for a missing file would invent zero. A present file that
    // does not parse is FAIL. A note would still print "factory ready".
    // estate drift already refused that file.
    let model_path = state_dir.join("model-actual.json");
    if model_path.exists() {
        match read_model_actual(&model_path) {
            Ok(actual) => {
                println!(
                    "  ok    model-actual.json bindings={}",
                    actual.bindings.len()
                )
            }
            Err(err) => {
                println!("  FAIL  {err}");
                fails.push(err);
            }
        }
    }

    print_doctor_enrich_join(state_dir, &mut fails);
    print_doctor_train_prepare(state_dir, &mut fails);

    println!("\nFrontier model");
    println!("--------------");
    print_doctor_frontier(
        "schema/local-catalog.v0.json",
        &root.join("schema/local-catalog.v0.json"),
        &mut fails,
    );
    doctor_cell_frontier(state_dir, &mut fails);

    println!("\nSecurity-as-IaC");
    println!("---------------");
    print_doctor_intention_coverage(root, &mut fails);

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

fn print_doctor_intention_coverage(root: &Path, fails: &mut Vec<String>) {
    let path = root.join("examples/estate.yaml");
    if !path.is_file() {
        println!("  note  examples/estate.yaml missing (coverage skipped)");
        return;
    }
    let estate = match load_estate(&path) {
        Ok(estate) => estate,
        Err(err) => {
            println!("  FAIL  examples/estate.yaml: {err}");
            fails.push(format!("examples/estate.yaml: {err}"));
            return;
        }
    };
    println!("model class:");
    for line in describe_model_class_coverage(&estate).lines() {
        println!("  {line}");
    }
    println!("tool mcp mount:");
    for line in describe_declared_coverage(&estate).lines() {
        println!("  {line}");
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
    let enrich_facts = model_estate::enrich_join_facts(&state_dir.join("enrich"))
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let train_prepares = model_estate::train_prepare_facts(state_dir)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let train_catalog = model_estate::train_catalog_facts()
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let enrich_stage = model_estate::read_enrich_stage(state_dir)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    // Missing lifecycle.json is the default record, not a file. Printing
    // that default would invent suspended and durable=true. A present
    // file that does not parse is refuse before the page. A parsed file
    // prints its state. estate doctor already refuses that file.
    let life = if floor_supervisor::lifecycle_path(state_dir).exists() {
        Some(load_lifecycle(state_dir)?)
    } else {
        None
    };
    let report = drift_with_roots(&estate, state_dir, Some(roots_base))?;
    let places = load_placements(state_dir)?;
    if let Some(actual) = places.as_ref() {
        refuse_lease_host_classes(actual)?;
    }
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
    let expired_n = list_expired_leases(state_dir, now_unix())?.len();
    let hop_expired_n = list_expired_hop_leases(state_dir, hop_now_unix())?.len();
    let last_plan = latest_plan(plans_dir)?
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
    let proposals = list_open_proposals(&packs_dir.join("proposed"))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    // Missing model-actual.json is not a failure. Printing a binding
    // count for a missing file would invent zero. A present file that
    // does not parse is refuse before the page. estate drift already
    // refused that file. A parsed file is not a status line.
    model_estate::drift_bindings(&estate, state_dir).map_err(|err| {
        anyhow::anyhow!("refuse:model-actual: model-actual.json: {err}")
    })?;
    let policy_present = policy.is_file();
    let mut doctor = doctor_summary_line(root, state_dir);
    if cell_catalog_disagrees_with_estate(&estate, &state_dir.join("catalog.json")).is_some()
        && doctor.starts_with("ok ")
    {
        doctor = "FAIL (1)".into();
    }
    println!("Cell One status");
    println!("===============");
    println!("estate: {} ({})", estate.name, path.display());
    println!("hash: {}", estate_hash(&estate));
    match life {
        None => {
            println!("paused: -");
            println!("lifecycle: -");
        }
        Some(life) => {
            let paused = life.state == floor_supervisor::LifecycleState::Suspended;
            println!("paused: {}", if paused { "yes" } else { "no" });
            println!(
                "lifecycle: {} (durable={})",
                life.state.as_str(),
                life.durable
            );
        }
    }
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
    print_status_enrich_join(enrich_facts.as_deref(), enrich_stage.as_ref(), &estate);
    print_status_train_prepare(train_prepares.as_deref());
    print_status_train_catalog(&train_catalog);
    println!(
        "policy: {} ({})",
        if policy_present { "present" } else { "missing" },
        policy.display()
    );
    println!("doctor: {doctor}");
    for (id, model) in model_estate::estate_frontier_models(&estate) {
        println!("frontier: {id} model={model}");
    }
    print_catalog_frontier("schema", &root.join("schema/local-catalog.v0.json"));
    let cell_catalog = state_dir.join("catalog.json");
    if let Some(err) = cell_catalog_disagrees_with_estate(&estate, &cell_catalog) {
        bail!("{err}");
    }
    print_catalog_frontier("cell", &cell_catalog);
    println!("in_sync: {}", report.in_sync);
    // A spawned cloud-agent lease is a refuse before the line that
    // would say it is not spawned. An empty list still prints that
    // line. The lease file is not rewritten.
    if !report.spawned_cloud_agents.is_empty() {
        bail!(
            "cloud-agent lease spawned (fail closed): {}",
            report.spawned_cloud_agents.join(", ")
        );
    }
    println!("cloud-agent: declared, not spawned");
    Ok(())
}

fn print_catalog_frontier(source: &str, path: &Path) {
    if !path.is_file() {
        return;
    }
    match read_catalog_frontier(path) {
        Ok(Some(model)) => println!("catalog frontier: {source} model={model}"),
        Ok(None) => println!("catalog frontier: {source} model=-"),
        Err(err) => println!("catalog frontier: {source} unreadable ({err})"),
    }
}

fn cell_catalog_disagrees_with_estate(
    estate: &estate_schema::Estate,
    path: &Path,
) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    let catalog = match read_catalog_frontier(path) {
        Ok(catalog) => catalog,
        Err(err) => return Some(unreadable_cell_catalog(&err)),
    };
    match model_estate::bound_frontier_model(estate) {
        Ok(bound) => model_estate::refuse_catalog_frontier_mismatch(
            bound.as_deref(),
            catalog.as_deref(),
        )
        .err()
        .map(|err| err.to_string()),
        Err(err) => Some(err.to_string()),
    }
}

fn doctor_cell_frontier(state_dir: &Path, fails: &mut Vec<String>) {
    let path = state_dir.join("catalog.json");
    if !path.is_file() {
        return;
    }
    let catalog = match read_catalog_frontier(&path) {
        Ok(model) => model,
        Err(err) => {
            let msg = unreadable_cell_catalog(&err);
            println!("  FAIL  {msg}");
            fails.push(msg);
            return;
        }
    };
    if let Some(model) = catalog.as_ref() {
        if estate_schema::contains_sku(model) {
            println!("  FAIL  catalog.json model={model} encodes a hardware SKU");
            fails.push("catalog.json frontier model encodes a SKU".into());
        }
    }
    let snap = match floor_supervisor::load_desired_snapshot(state_dir) {
        Ok(snap) => snap,
        Err(err) => {
            println!("  FAIL  desired-snapshot.yaml: {err}");
            fails.push(format!("desired-snapshot.yaml: {err}"));
            return;
        }
    };
    let Some(estate) = snap else {
        match catalog.as_deref() {
            Some(model) => println!(
                "  note  catalog.json model={model} is not a binding (no desired snapshot)"
            ),
            None => println!("  note  catalog.json has no frontier model"),
        }
        return;
    };
    match model_estate::bound_frontier_model(&estate) {
        Ok(bound) => {
            if let Err(err) = model_estate::refuse_catalog_frontier_mismatch(
                bound.as_deref(),
                catalog.as_deref(),
            ) {
                println!("  FAIL  {err}");
                fails.push(err.to_string());
                return;
            }
        }
        Err(err) => {
            println!("  FAIL  {err}");
            fails.push(err.to_string());
            return;
        }
    }
    match catalog.as_deref() {
        Some(model) => println!("  ok    catalog.json model={model}"),
        None => println!("  note  catalog.json has no frontier model"),
    }
}

fn print_doctor_frontier(label: &str, path: &Path, fails: &mut Vec<String>) {
    if !path.is_file() {
        return;
    }
    match read_catalog_frontier(path) {
        Ok(Some(model)) if estate_schema::contains_sku(&model) => {
            println!("  FAIL  {label} model={model} encodes a hardware SKU");
            fails.push(format!("{label} frontier model encodes a SKU"));
        }
        Ok(Some(model)) => println!("  ok    {label} model={model}"),
        Ok(None) => println!("  note  {label} has no frontier model"),
        Err(err) => println!("  note  {label} frontier model unreadable ({err})"),
    }
}

fn cell_catalog_binding_problem(state_dir: &Path) -> Option<String> {
    let path = state_dir.join("catalog.json");
    if !path.is_file() {
        return None;
    }
    let catalog = match read_catalog_frontier(&path) {
        Ok(catalog) => catalog,
        Err(err) => return Some(unreadable_cell_catalog(&err)),
    };
    let estate = match floor_supervisor::load_desired_snapshot(state_dir) {
        Ok(Some(estate)) => estate,
        Ok(None) => return None,
        Err(err) => return Some(format!("desired-snapshot.yaml: {err}")),
    };
    match model_estate::bound_frontier_model(&estate) {
        Ok(bound) => model_estate::refuse_catalog_frontier_mismatch(
            bound.as_deref(),
            catalog.as_deref(),
        )
        .err()
        .map(|err| err.to_string()),
        Err(err) => Some(err.to_string()),
    }
}

fn read_model_actual(path: &Path) -> Result<model_estate::ModelActual, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("model-actual.json: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("model-actual.json: {e}"))
}

fn unreadable_cell_catalog(err: &str) -> String {
    format!(
        "refuse:frontier-model: cell catalog unreadable ({err}); schema card is not the binding"
    )
}

fn read_catalog_frontier(path: &Path) -> std::result::Result<Option<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    model_estate::frontier_model_from_catalog_json(&text)
}

fn print_status_enrich_join(
    facts: Option<&[model_estate::EnrichJoinFact]>,
    stage: Option<&model_estate::EnrichBindingStage>,
    estate: &estate_schema::Estate,
) {
    if let Some(stage) = stage {
        println!(
            "enrich_stage: applied={} estate_rewritten={} tag={} binding={}",
            stage.applied, stage.estate_rewritten, stage.local_tag, stage.binding_id
        );
    }
    let Some(facts) = facts else {
        return;
    };
    let model = model_estate::local_slm_model_param(estate);
    if facts.is_empty() {
        println!("enrich_binding: enrich directory is empty");
        return;
    }
    for fact in facts {
        let shown = model.as_deref().unwrap_or("-");
        if model.as_deref() == Some(fact.local_tag.as_str()) {
            println!(
                "enrich_binding: local_slm model={} pack={} driver={}",
                fact.local_tag, fact.pack_id, fact.driver
            );
        } else {
            println!(
                "enrich_binding: pending kind={} pack={} driver={} tag={} estate_model={shown}",
                fact.kind, fact.pack_id, fact.driver, fact.local_tag
            );
        }
    }
}

const TRAIN_PREPARE_NOTE: &str =
    "prepare.json record only; this factory did not train, merge, convert, or seat";

fn print_status_train_prepare(facts: Option<&[model_estate::TrainPrepareFact]>) {
    let Some(facts) = facts else {
        return;
    };
    if facts.is_empty() {
        println!("train_prepare: enrich directory has no prepare.json");
        return;
    }
    for fact in facts {
        println!("{}", format_train_prepare_line(fact));
    }
    println!("train_prepare_note: {TRAIN_PREPARE_NOTE}");
}

fn print_status_train_catalog(rows: &[model_estate::TrainCatalogFact]) {
    for row in rows {
        println!(
            "train_catalog: {} status={} live=false",
            row.driver_id, row.status
        );
    }
}

fn format_train_prepare_line(fact: &model_estate::TrainPrepareFact) -> String {
    let mut parts = vec![
        format!("pack={}", fact.pack_id),
        format!("driver={}", fact.driver),
        format!("job={}", fact.job),
    ];
    if let Some(seat) = fact.seat_tag.as_deref() {
        parts.push(format!("seat_tag={seat}"));
    }
    if let Some(train) = fact.train_base.as_deref() {
        parts.push(format!("train_base={train}"));
    }
    if let Some(shape) = fact.trained_shape.as_deref() {
        parts.push(format!("trained_shape={shape}"));
    }
    parts.push(format!("out={}", fact.out_dir.display()));
    format!("train_prepare: {}", parts.join(" "))
}

fn print_doctor_train_prepare(state_dir: &Path, fails: &mut Vec<String>) {
    println!("\nTrain prepare");
    println!("-------------");
    match model_estate::train_catalog_facts() {
        Ok(rows) => {
            for row in &rows {
                println!(
                    "  ok    train_catalog {} status={} live=false",
                    row.driver_id, row.status
                );
            }
        }
        Err(err) => {
            println!("  FAIL  {err}");
            fails.push(err.to_string());
        }
    }
    match model_estate::train_prepare_facts(state_dir) {
        Ok(None) => {}
        Ok(Some(rows)) if rows.is_empty() => {
            println!("  note  enrich directory has no prepare.json");
        }
        Ok(Some(rows)) => {
            for fact in &rows {
                println!("  ok    {}", format_train_prepare_line(fact));
            }
            println!("  note  {TRAIN_PREPARE_NOTE}");
        }
        Err(err) => {
            println!("  FAIL  {err}");
            fails.push(err.to_string());
        }
    }
}

fn print_doctor_enrich_join(state_dir: &Path, fails: &mut Vec<String>) {
    let facts = match model_estate::enrich_join_facts(&state_dir.join("enrich")) {
        Ok(None) => return,
        Ok(Some(facts)) => facts,
        Err(err) => {
            println!("\nEnrich join");
            println!("-----------");
            println!("  FAIL  {err}");
            fails.push(err.to_string());
            return;
        }
    };
    println!("\nEnrich join");
    println!("-----------");
    let model = match floor_supervisor::load_desired_snapshot(state_dir) {
        Ok(Some(estate)) => model_estate::local_slm_model_param(&estate),
        Ok(None) => None,
        Err(err) => {
            println!("  FAIL  desired-snapshot.yaml: {err}");
            fails.push(format!("desired-snapshot.yaml: {err}"));
            return;
        }
    };
    match model_estate::read_enrich_stage(state_dir) {
        Ok(Some(stage)) if !stage.applied => {
            println!(
                "  note  enrich stage tag={} source estate not written (apply --require-plan)",
                stage.local_tag
            );
        }
        Ok(_) => {}
        Err(err) => {
            println!("  FAIL  {err}");
            fails.push(err.to_string());
            return;
        }
    }
    if facts.is_empty() {
        println!("  note  enrich directory is empty");
        return;
    }
    let shown = model.as_deref().unwrap_or("-");
    for fact in &facts {
        if model.as_deref() == Some(fact.local_tag.as_str()) {
            println!(
                "  ok    local_slm model={} pack={} driver={}",
                fact.local_tag, fact.pack_id, fact.driver
            );
        } else if fact.kind == "proposal" {
            println!(
                "  note  binding proposal pack={} driver={} tag={} estate local_slm model={shown}",
                fact.pack_id, fact.driver, fact.local_tag
            );
        } else {
            println!(
                "  note  prepare pack={} driver={} tag={} estate local_slm model={shown} (no binding proposal)",
                fact.pack_id, fact.driver, fact.local_tag
            );
        }
    }
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
    match load_placements(state_dir) {
        Ok(Some(places)) => {
            if places
                .leases
                .iter()
                .any(|l| l.kind == "cloud-agent" && l.spawned)
            {
                fails += 1;
            }
        }
        Ok(None) => {}
        Err(_) => fails += 1,
    }
    for path in [
        root.join("schema/local-catalog.v0.json"),
        state_dir.join("catalog.json"),
    ] {
        if let Ok(Some(model)) = read_catalog_frontier(&path) {
            if estate_schema::contains_sku(&model) {
                fails += 1;
            }
        }
    }
    if cell_catalog_binding_problem(state_dir).is_some() {
        fails += 1;
    }
    if model_estate::enrich_join_facts(&state_dir.join("enrich")).is_err()
        || model_estate::train_prepare_facts(state_dir).is_err()
        || model_estate::train_catalog_facts().is_err()
        || model_estate::read_enrich_stage(state_dir).is_err()
    {
        fails += 1;
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
        let root = std::env::temp_dir().join(format!("cell-doctor-ci-{nanos}"));
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
