//! `estate control-plane-prove` — one throwaway lab for fuel, decide, and run.
//!
//! Fuel: mock-import `ag_news` and `rust_idiom` beside `local_slm`
//! (`trained_shape` gguf, `auto_apply=false`), then apply-proposal, plan,
//! and apply --require-plan. Standing next is joinable only when the GGUF
//! is a regular file and an agent models allow-list names the binding.
//! Decide: the host-validate authorize / convey / complete --mock cases on
//! that same estate and state-dir. Run: `standing-dual` under the
//! supervised runner, three hops, one session. In-process mock. No network.
//! No Ollama. Does not rewrite `examples/estate.yaml`.
//! `READY_FOR_LIVE_TEST` stays no.

use anyhow::{bail, Context, Result};
use estate_schema::{
    AgentPack, ChainHop, Effect, Intention, IntentionKind, ModelBinding, ModelClass, ModelUseDecl,
    PackPackage, Routine, ToolDecl,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::decisions::DecisionReceipt;
use crate::routine_runner::{self, RunnerWatchArgs};
use crate::routines::WATCH_INTERVAL_ENV;
use crate::specialty_bind::{self, DualSpecialtyLab};

const LOCKED_CKSUM: &str = "43770130 3391";
const PROVE_SCHEMA: &str = "cell-one.control-plane-prove.v0";
const BINDING_AG: &str = "ag_news";
const BINDING_RUST: &str = "rust_idiom";
const BINDING_FRONTIER: &str = "frontier_http";
const SEAT_AG: &str = "specialist-agnews-3000";
const SEAT_RUST: &str = "specialist-rustidiom-3000";
const HOP_CAP: &str = "lane-tool";
const ROUTINE_ID: &str = "standing-dual";
const PACKAGE_ID: &str = "dual-specialty";
const PACK_ID: &str = "dual-specialty";
const TICK_WAIT: Duration = Duration::from_secs(20);

const DUAL_HOPS: &[(&str, &str, bool)] = &[
    ("research", BINDING_AG, false),
    ("idiom", BINDING_RUST, true),
    ("horizon", BINDING_FRONTIER, true),
];

pub(crate) fn cmd_control_plane_prove(root: &Path, out: Option<&Path>) -> Result<()> {
    std::env::remove_var("CELL_LOCAL_ENDPOINT");
    std::env::remove_var("CELL_LOCAL_LIVE");
    let root = root
        .canonicalize()
        .with_context(|| format!("refuse:root: {}", root.display()))?;
    let locked = root.join("examples/estate.yaml");
    let before =
        fs::read(&locked).with_context(|| format!("refuse:estate: {}", locked.display()))?;
    let cksum_before = file_cksum(&locked)?;
    if !cksum_before.starts_with(LOCKED_CKSUM) {
        bail!("refuse:estate: examples/estate.yaml cksum is {cksum_before}, want {LOCKED_CKSUM}");
    }

    let (out_hint, wipe) = match out {
        Some(path) => (path.to_path_buf(), false),
        None => (default_out(), true),
    };
    if refuses_locked_target(&out_hint, &root, &locked)? {
        bail!("refuse:out: control-plane-prove does not write examples/estate.yaml");
    }
    if wipe {
        let _ = fs::remove_dir_all(&out_hint);
    }
    fs::create_dir_all(&out_hint).with_context(|| format!("refuse:out: {}", out_hint.display()))?;
    let out = out_hint
        .canonicalize()
        .with_context(|| format!("refuse:out: {}", out_hint.display()))?;
    if refuses_locked_target(&out, &root, &locked)? {
        bail!("refuse:out: control-plane-prove does not write examples/estate.yaml");
    }

    println!("control-plane-prove: fuel");
    let landed = specialty_bind::land_dual_specialty_lab(&root, &out)?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: fuel rewrote examples/estate.yaml");
    }
    graft_chain_and_hop(&landed.lab)?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: chain graft rewrote examples/estate.yaml");
    }
    let (join_ag, join_rust) = confirm_joinable(&landed)?;
    if join_ag.seat_model != SEAT_AG || join_rust.seat_model != SEAT_RUST {
        bail!(
            "refuse:binding: seat models are {} and {}",
            join_ag.seat_model,
            join_rust.seat_model
        );
    }
    println!("fuel joinable: {BINDING_AG}=yes {BINDING_RUST}=yes");

    println!("control-plane-prove: decide");
    let policy = root.join("policy/cell-one.policy.v0.yaml");
    let replay = out.join("decisions-replay.jsonl");
    crate::decision_prove::run_host_validate_cases(&landed.lab, &landed.state, &policy, &replay)?;
    clear_hint(&landed.state)?;
    let decide = snapshot_surfaces(&landed.state)?;
    if decide.authorize != 4 || decide.convey != 4 || decide.complete != 5 {
        bail!(
            "refuse:decision: surfaces authorize={} convey={} complete={}",
            decide.authorize, decide.convey, decide.complete
        );
    }
    if !decide.capabilities.contains(&BINDING_AG.to_string())
        || !decide.capabilities.contains(&BINDING_RUST.to_string())
    {
        bail!(
            "refuse:decision: complete capabilities are {:?}",
            decide.capabilities
        );
    }

    println!("control-plane-prove: run");
    let run = run_dual_runner(&landed.lab, &landed.state)?;
    let journal = snapshot_surfaces(&landed.state)?;
    if journal.complete == 0 {
        bail!("refuse:decision: report complete is 0 after the runner");
    }
    if !journal.capabilities.contains(&BINDING_AG.to_string())
        || !journal.capabilities.contains(&BINDING_RUST.to_string())
    {
        bail!(
            "refuse:decision: journal capabilities are {:?}",
            journal.capabilities
        );
    }
    println!("decisions report");
    crate::decisions::cmd_decisions_report(&landed.state, None)?;

    if fs::read(&locked)? != before {
        bail!("refuse:estate: examples/estate.yaml bytes changed");
    }
    let cksum_after = file_cksum(&locked)?;
    if cksum_after != cksum_before {
        bail!("refuse:estate: examples/estate.yaml cksum changed to {cksum_after}");
    }

    let body = serde_json::json!({
        "schema": PROVE_SCHEMA,
        "ok": true,
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "live_sync": false,
        "estate_cksum": cksum_after,
        "lab_estate": landed.lab.display().to_string(),
        "state_dir": landed.state.display().to_string(),
        "fuel": {
            "binding_ids": [BINDING_AG, BINDING_RUST],
            "beside": "local_slm",
            "trained_shape": "gguf",
            "auto_apply": false,
            "joinable": {
                BINDING_AG: join_ag.joinable,
                BINDING_RUST: join_rust.joinable,
            },
            "seat_models": {
                BINDING_AG: join_ag.seat_model,
                BINDING_RUST: join_rust.seat_model,
            },
            "gguf": {
                BINDING_AG: landed.gguf_ag.display().to_string(),
                BINDING_RUST: landed.gguf_rust.display().to_string(),
            },
        },
        "decide": {
            "surface_authorize": decide.authorize,
            "surface_convey": decide.convey,
            "surface_complete": decide.complete,
            "capabilities": decide.capabilities,
            "abstain": "refuse:decision-abstain",
            "stale_fallback": BINDING_AG,
            "ineligible_fallback": BINDING_AG,
        },
        "run": {
            "routine_id": ROUTINE_ID,
            "package": PACKAGE_ID,
            "chain_id": run.chain_id,
            "session_id": run.session_id,
            "session_stitch": run.session_stitch,
            "hops": run.hops,
            "digest_cites_session": run.digest_cites_session,
            "digest_cites_package": run.digest_cites_package,
            "digest_cites_chain": run.digest_cites_chain,
            "double_start_refuse": "refuse:runner-already-running",
        },
        "journal": {
            "complete": journal.complete,
            "surface_authorize": journal.authorize,
            "surface_convey": journal.convey,
            "capabilities": journal.capabilities,
        },
        "note": "Fixture prove. One lab. Mocked GGUFs. Mock complete. Mock runner. No GPU train. No network. Not a live PASS."
    });
    let pretty = serde_json::to_string_pretty(&body)?;
    if pretty.split_whitespace().any(|word| word == "enforced") {
        bail!("refuse:control-plane: report invented enforced");
    }
    if pretty.contains("READY_FOR_LIVE_TEST: yes") || pretty.contains("\"ready_for_live_test\": true") {
        bail!("refuse:control-plane: report invented a live-test ready flag");
    }
    fs::write(out.join("control-plane-prove.json"), format!("{pretty}\n"))?;
    println!("{pretty}");
    println!("control-plane-prove: ok");
    println!("READY_FOR_LIVE_TEST: no");
    Ok(())
}

struct SurfaceSnapshot {
    authorize: usize,
    convey: usize,
    complete: usize,
    capabilities: Vec<String>,
}

fn snapshot_surfaces(state: &Path) -> Result<SurfaceSnapshot> {
    let receipts = crate::decisions::load_receipts(state)?;
    let mut capabilities: Vec<String> = receipts
        .iter()
        .filter(|row| row.surface == "complete" && (row.capability == BINDING_AG || row.capability == BINDING_RUST || row.capability == BINDING_FRONTIER))
        .map(|row| row.capability.clone())
        .collect();
    capabilities.sort();
    capabilities.dedup();
    Ok(SurfaceSnapshot {
        authorize: receipts.iter().filter(|row| row.surface == "authorize").count(),
        convey: receipts.iter().filter(|row| row.surface.is_empty()).count(),
        complete: receipts.iter().filter(|row| row.surface == "complete").count(),
        capabilities,
    })
}

fn confirm_joinable(
    landed: &DualSpecialtyLab,
) -> Result<(model_estate::SpecialtyJoin, model_estate::SpecialtyJoin)> {
    let estate = estate_schema::load_estate(&landed.lab)
        .with_context(|| format!("refuse:estate: load {}", landed.lab.display()))?;
    if !estate.model_bindings.iter().any(|row| {
        row.id == "local_slm" && row.class == ModelClass::Local
    }) {
        bail!("refuse:binding: local_slm missing after fuel");
    }
    let join_ag = assessed(&estate, &landed.prepared_ag, BINDING_AG)?;
    let join_rust = assessed(&estate, &landed.prepared_rust, BINDING_RUST)?;
    crate::enrich::cmd_enrich_standing_next(&landed.lab, &landed.prepared_ag)?;
    crate::enrich::cmd_enrich_standing_next(&landed.lab, &landed.prepared_rust)?;
    Ok((join_ag, join_rust))
}

fn assessed(
    estate: &estate_schema::Estate,
    prepared: &Path,
    binding: &str,
) -> Result<model_estate::SpecialtyJoin> {
    let join = model_estate::assess_specialty_join(estate, prepared)
        .map_err(|err| anyhow::anyhow!("{err}"))?
        .ok_or_else(|| anyhow::anyhow!("refuse:specialty-join: {binding} proposal missing"))?;
    if !join.joinable || join.binding_id != binding {
        bail!(
            "refuse:specialty-join: {binding} joinable={} binding={}",
            join.joinable,
            join.binding_id
        );
    }
    if join.trained_shape != "gguf" || join.auto_apply || join.ready_for_live_test || join.live_pass_recorded
    {
        bail!("refuse:specialty-join: {binding} is not a gguf auto_apply=false join");
    }
    if !Path::new(&join.gguf).is_file() {
        bail!("refuse:gguf: {} is not a regular file", join.gguf);
    }
    println!("{}", join.status_line());
    Ok(join)
}

fn graft_chain_and_hop(lab: &Path) -> Result<()> {
    let mut estate = estate_schema::load_estate(lab)
        .with_context(|| format!("refuse:estate: load {}", lab.display()))?;
    let local_before = local_params(&estate)?;
    if !estate
        .model_bindings
        .iter()
        .any(|row| row.id == BINDING_FRONTIER)
    {
        estate.model_bindings.push(ModelBinding {
            id: BINDING_FRONTIER.into(),
            class: ModelClass::Frontier,
            driver: "http-remote".into(),
            params: serde_json::json!({
                "hint": "openai-compat",
                "endpoint_env": "CELL_FRONTIER_ENDPOINT",
            }),
            wired: true,
        });
    }
    ensure_model(
        &mut estate,
        "horizon",
        BINDING_FRONTIER,
        "Lab copy. Horizon completes on frontier_http.",
    )?;
    ensure_allow(
        &mut estate,
        "horizon",
        BINDING_FRONTIER,
        IntentionKind::Model,
        "Lab copy. Horizon may complete on frontier_http.",
    );
    for agent in ["research", "idiom", "sanctum"] {
        ensure_tool(&mut estate, agent, HOP_CAP)?;
        ensure_allow(
            &mut estate,
            agent,
            HOP_CAP,
            IntentionKind::Tool,
            "Lab copy. Granted hop capability. The selector still does not grant a model.",
        );
    }
    if estate.pack(PACK_ID).is_none() {
        estate.packs.push(AgentPack {
            id: PACK_ID.into(),
            members: vec!["horizon".into(), "research".into(), "idiom".into()],
            orchestrator: Some("horizon".into()),
            chain: Vec::new(),
        });
    }
    if estate.pack_package(PACKAGE_ID).is_none() {
        estate.pack_packages.push(PackPackage {
            id: PACKAGE_ID.into(),
            pack: PACK_ID.into(),
            prompt: Some("ping".into()),
            binding: None,
            note: Some(
                "Two purpose seats then frontier. Mock does not call Ollama.".into(),
            ),
            chain: vec![
                ChainHop {
                    agent: "research".into(),
                    binding: BINDING_AG.into(),
                },
                ChainHop {
                    agent: "idiom".into(),
                    binding: BINDING_RUST.into(),
                },
                ChainHop {
                    agent: "horizon".into(),
                    binding: BINDING_FRONTIER.into(),
                },
            ],
        });
    }
    if estate.routine(ROUTINE_ID).is_none() {
        estate.routines.push(Routine {
            id: ROUTINE_ID.into(),
            package: PACKAGE_ID.into(),
            note: Some(
                "Dual-specialty chain under the runner. Tick journals three hops and one session."
                    .into(),
            ),
            schedule: Some("@hourly".into()),
            enabled: None,
        });
    }
    if local_params(&estate)? != local_before {
        bail!("refuse:binding: local_slm params changed while the chain was grafted");
    }
    estate_schema::validate(&estate).map_err(|errs| {
        anyhow::anyhow!(
            "refuse:estate: lab copy invalid: {}",
            errs.join("; ")
        )
    })?;
    let yaml = estate_schema::render_estate_yaml(&estate)?;
    fs::write(lab, yaml)?;
    println!("lab chain: pack={PACK_ID} package={PACKAGE_ID} routine={ROUTINE_ID}");
    println!("lab binding: {BINDING_FRONTIER} class=frontier");
    println!("lab intention: horizon model {BINDING_FRONTIER} effect=allow");
    println!("lab intention: research,idiom,sanctum tool {HOP_CAP} effect=allow");
    Ok(())
}

fn local_params(estate: &estate_schema::Estate) -> Result<serde_json::Value> {
    let binding = estate
        .model_bindings
        .iter()
        .find(|row| row.id == "local_slm")
        .ok_or_else(|| anyhow::anyhow!("refuse:binding: local_slm missing"))?;
    Ok(binding.params.clone())
}

fn ensure_model(
    estate: &mut estate_schema::Estate,
    agent_id: &str,
    id: &str,
    note: &str,
) -> Result<()> {
    let agent = estate
        .agents
        .iter_mut()
        .find(|row| row.id == agent_id)
        .ok_or_else(|| anyhow::anyhow!("refuse:agent: {agent_id} missing on lab estate"))?;
    if agent.models.iter().any(|row| row.id == id) {
        return Ok(());
    }
    agent.models.push(ModelUseDecl {
        id: id.to_string(),
        description: Some(note.to_string()),
    });
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
        description: Some("Lab hop capability. Declaring it is not a model grant.".into()),
    });
    Ok(())
}

fn ensure_allow(
    estate: &mut estate_schema::Estate,
    agent: &str,
    object: &str,
    kind: IntentionKind,
    note: &str,
) {
    let present = estate.intentions.iter().any(|row| {
        row.subject_agent == agent
            && row.object == object
            && row.kind == kind
            && row.effect == Effect::Allow
    });
    if present {
        return;
    }
    estate.intentions.push(Intention {
        subject_agent: agent.to_string(),
        object: object.to_string(),
        kind,
        effect: Effect::Allow,
        note: Some(note.to_string()),
    });
}

struct DualRun {
    chain_id: String,
    session_id: String,
    session_stitch: bool,
    hops: Vec<serde_json::Value>,
    digest_cites_session: bool,
    digest_cites_package: bool,
    digest_cites_chain: bool,
}

fn run_dual_runner(lab: &Path, state: &Path) -> Result<DualRun> {
    let ids = vec![ROUTINE_ID.to_string()];
    let prev = std::env::var(WATCH_INTERVAL_ENV).ok();
    let _interval = IntervalGuard { prev };
    std::env::set_var(WATCH_INTERVAL_ENV, "2");
    let _stop = RunnerStop {
        state: state.to_path_buf(),
    };

    routine_runner::cmd_runner_start(watch_args(&ids, lab, state, 30))?;
    if !wait_for_tick(state) {
        let log = fs::read_to_string(routine_runner::out_log_path(state, "default"))
            .unwrap_or_default();
        bail!("refuse:runner-prove: runner never ticked standing-dual\n{log}");
    }
    let hops = chain_hops(state)?;
    let session_id = hops
        .first()
        .and_then(|row| row.session_id.clone())
        .unwrap_or_default();
    let session_stitch = hops.len() == DUAL_HOPS.len()
        && !session_id.is_empty()
        && hops
            .iter()
            .all(|row| row.session_id.as_deref() == Some(session_id.as_str()));
    if !session_stitch {
        bail!("refuse:runner-prove: dual hops do not share one session_id");
    }
    let mut hop_json = Vec::new();
    for (i, (agent, binding, want_ctx)) in DUAL_HOPS.iter().enumerate() {
        let row = hops.get(i).ok_or_else(|| {
            anyhow::anyhow!("refuse:runner-prove: missing hop {}", i + 1)
        })?;
        let ctx = row.session_context;
        if row.capability != *binding || row.result != *binding {
            bail!(
                "refuse:runner-prove: hop {} capability={} result={} want {binding}",
                i + 1,
                row.capability,
                row.result
            );
        }
        if row.handoff_to.as_deref() != Some(*agent) {
            bail!(
                "refuse:runner-prove: hop {} handoff_to={:?} want {agent}",
                i + 1,
                row.handoff_to
            );
        }
        if ctx != Some(*want_ctx) {
            bail!(
                "refuse:runner-prove: hop {} context={ctx:?} want {want_ctx}",
                i + 1
            );
        }
        let word = if *want_ctx { "applied" } else { "none" };
        println!(
            "runner hop: {agent} capability={binding} context={word} session={session_id}"
        );
        hop_json.push(serde_json::json!({
            "hop": i + 1,
            "agent": agent,
            "capability": binding,
            "result": binding,
            "context": word,
            "session_id": session_id,
        }));
    }
    let chain_id = hops
        .first()
        .and_then(|row| row.chain_id.clone())
        .unwrap_or_default();
    if !chain_id.starts_with(&format!("chain-{PACKAGE_ID}-")) {
        bail!("refuse:runner-prove: chain_id is {chain_id}");
    }
    let package = hops
        .first()
        .and_then(|row| row.package_id.clone())
        .unwrap_or_default();
    if package != PACKAGE_ID {
        bail!("refuse:runner-prove: package_id is {package}");
    }

    let digest = crate::ops::routine_digest_text(&ids, lab, state).unwrap_or_default();
    let digest_cites_session = digest.contains("session_id=") && digest.contains("context=applied");
    let digest_cites_package = digest.contains(&format!("package={PACKAGE_ID}"));
    let digest_cites_chain = digest.contains(&format!("chain=chain-{PACKAGE_ID}-"));
    if !digest_cites_session || !digest_cites_package || !digest_cites_chain {
        bail!("refuse:runner-prove: digest missing session, package, or chain\n{digest}");
    }
    println!(
        "runner digest: session_id={session_id} package={PACKAGE_ID} chain={chain_id}"
    );

    let second = routine_runner::cmd_runner_start(watch_args(&ids, lab, state, 30));
    match second {
        Err(err) => {
            let text = format!("{err:#}");
            if !text.contains("refuse:runner-already-running") {
                bail!("refuse:runner-prove: double-start failed closed for the wrong reason: {text}");
            }
            println!("runner double-start: refuse:runner-already-running");
        }
        Ok(()) => bail!("refuse:runner-prove: double-start succeeded"),
    }
    routine_runner::cmd_runner_stop(state, Some("default"))?;
    Ok(DualRun {
        chain_id,
        session_id,
        session_stitch,
        hops: hop_json,
        digest_cites_session,
        digest_cites_package,
        digest_cites_chain,
    })
}

fn chain_hops(state: &Path) -> Result<Vec<DecisionReceipt>> {
    let want = estate_schema::normalize_name(ROUTINE_ID);
    let rows = crate::decisions::load_receipts(state)?;
    let rows: Vec<DecisionReceipt> = rows
        .into_iter()
        .filter(|row| {
            row.routine_id
                .as_deref()
                .map(|id| estate_schema::normalize_name(id) == want)
                .unwrap_or(false)
        })
        .collect();
    if rows.len() < DUAL_HOPS.len() {
        bail!(
            "refuse:runner-prove: want ≥{} hop receipts, got {}",
            DUAL_HOPS.len(),
            rows.len()
        );
    }
    let chain = rows[0].chain_id.clone();
    let hops: Vec<DecisionReceipt> = rows
        .into_iter()
        .take_while(|row| row.chain_id == chain)
        .collect();
    if hops.len() < DUAL_HOPS.len() {
        bail!(
            "refuse:runner-prove: want ≥{} hops on the first chain, got {}",
            DUAL_HOPS.len(),
            hops.len()
        );
    }
    Ok(hops.into_iter().take(DUAL_HOPS.len()).collect())
}

fn wait_for_tick(state: &Path) -> bool {
    let digest_log = routine_runner::digest_log_path(state, "default");
    let start = Instant::now();
    while start.elapsed() < TICK_WAIT {
        if let Ok(text) = fs::read_to_string(&digest_log) {
            if text.contains(ROUTINE_ID)
                && text.contains("session_id=")
                && text.contains(&format!("package={PACKAGE_ID}"))
                && text.contains("context=applied")
            {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn watch_args(routine_ids: &[String], estate: &Path, state_dir: &Path, max_cycles: u32) -> RunnerWatchArgs {
    RunnerWatchArgs {
        runner_id: "default".into(),
        routine_ids: routine_ids.to_vec(),
        agent: None,
        prompt: None,
        text: None,
        estate: estate.to_path_buf(),
        state_dir: state_dir.to_path_buf(),
        feed_dir: None,
        endpoint: None,
        mock: true,
        chain: false,
        interval: "5m".into(),
        max_cycles: Some(max_cycles),
    }
}

fn clear_hint(state: &Path) -> Result<()> {
    let path = state.join(crate::decisions::SELECT_FILE);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

struct IntervalGuard {
    prev: Option<String>,
}

impl Drop for IntervalGuard {
    fn drop(&mut self) {
        match self.prev.take() {
            Some(value) => std::env::set_var(WATCH_INTERVAL_ENV, value),
            None => {
                let _ = std::env::remove_var(WATCH_INTERVAL_ENV);
            }
        }
    }
}

struct RunnerStop {
    state: PathBuf,
}

impl Drop for RunnerStop {
    fn drop(&mut self) {
        let _ = routine_runner::cmd_runner_stop(&self.state, Some("default"));
    }
}

pub(crate) fn refuses_locked_target(out: &Path, root: &Path, locked: &Path) -> Result<bool> {
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
    std::env::temp_dir().join(format!("cell-one-control-plane-{token}"))
}
