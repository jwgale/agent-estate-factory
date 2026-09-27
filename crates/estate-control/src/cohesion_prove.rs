//! `estate pack cohesion-prove` — one throwaway lab for fuel, decide, run,
//! specialty-real seats, pack install, and multi-hop CLI crew session smoke.
//!
//! Composes `estate control-plane-prove` (dual import-trained bind,
//! host-validate authorize / convey / complete --mock, `standing-dual`
//! runner stitch) with an optional specialty-real bind: when import-trained
//! AG News / rust_idiom GGUF artifacts are present, they land as named
//! seats (`ag_news`, `rust_idiom`) beside `local_slm` and `complete --mock`
//! names those seats. When absent, the stage is skipped with a reason and
//! the prove stays ok. Then `estate pack crew-session-prove`: throwaway
//! `plugin-install-local` plus successive `estate complete --mock` hops
//! on one `session_id` (hop 2 sees hop 1; research stays
//! `refuse:pack-orchestrator`). A Cursor MCP loader hang is out of scope.
//! Does not rewrite `examples/estate.yaml`. `READY_FOR_LIVE_TEST` stays no.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::control_plane_prove;
use crate::crew_session_prove;
use crate::specialty_bind;

const LOCKED_CKSUM: &str = "43770130 3391";
const PROVE_SCHEMA: &str = "cell-one.cohesion-prove.v0";
const CONTROL_SCHEMA: &str = "cell-one.control-plane-prove.v0";
const SKIP_GGUF_ABSENT: &str = "skipped:gguf-absent";
const BINDING_AG: &str = "ag_news";
const BINDING_RUST: &str = "rust_idiom";
const ENV_AG_GGUF: &str = "CELL_SPECIALTY_AG_NEWS_GGUF";
const ENV_RUST_GGUF: &str = "CELL_SPECIALTY_RUST_IDIOM_GGUF";
const ENV_GGUF_ROOT: &str = "CELL_SPECIALTY_GGUF_ROOT";

pub(crate) fn cmd_cohesion_prove(
    root: &Path,
    out: Option<&Path>,
    pack_id: &str,
    estate: &Path,
) -> Result<()> {
    std::env::remove_var("CELL_LOCAL_ENDPOINT");
    std::env::remove_var("CELL_LOCAL_LIVE");
    let root = root
        .canonicalize()
        .with_context(|| format!("refuse:root: {}", root.display()))?;
    let locked = root.join("examples/estate.yaml");
    let before = fs::read(&locked).with_context(|| format!("refuse:estate: {}", locked.display()))?;
    let cksum_before = file_cksum(&locked)?;
    if !cksum_before.starts_with(LOCKED_CKSUM) {
        bail!("refuse:estate: examples/estate.yaml cksum is {cksum_before}, want {LOCKED_CKSUM}");
    }

    let fixture = resolve_fixture(&root, estate)?;
    if same_file(&fixture, &locked)? {
        bail!("refuse:estate: cohesion-prove pack smoke does not use examples/estate.yaml");
    }
    let fixture_before = fs::read(&fixture)
        .with_context(|| format!("refuse:estate: {}", fixture.display()))?;

    let (out_hint, wipe) = match out {
        Some(path) => (path.to_path_buf(), false),
        None => (default_out(), true),
    };
    if control_plane_prove::refuses_locked_target(&out_hint, &root, &locked)? {
        bail!("refuse:out: cohesion-prove does not write examples/estate.yaml");
    }
    if wipe {
        let _ = fs::remove_dir_all(&out_hint);
    }
    fs::create_dir_all(&out_hint).with_context(|| format!("refuse:out: {}", out_hint.display()))?;
    let out = out_hint
        .canonicalize()
        .with_context(|| format!("refuse:out: {}", out_hint.display()))?;
    if control_plane_prove::refuses_locked_target(&out, &root, &locked)? {
        bail!("refuse:out: cohesion-prove does not write examples/estate.yaml");
    }

    println!("cohesion-prove: control-plane");
    control_plane_prove::cmd_control_plane_prove(&root, Some(&out))?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: control-plane rewrote examples/estate.yaml");
    }
    let control: Value = read_json(&out.join("control-plane-prove.json"))?;
    require_control_plane(&control)?;

    println!("cohesion-prove: specialty-real");
    let specialty_real = specialty_real_stage(&root, &out)?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: specialty-real rewrote examples/estate.yaml");
    }

    println!("cohesion-prove: pack");
    println!("cohesion-prove: cli-smoke");
    let pack_stage = crew_session_prove::run_pack_stage(pack_id, &fixture, &out)?;
    if fs::read(&locked)? != before || fs::read(&fixture)? != fixture_before {
        bail!("refuse:estate: pack install or crew session rewrote a source estate");
    }
    let cksum_after = file_cksum(&locked)?;
    if cksum_after != cksum_before {
        bail!("refuse:estate: examples/estate.yaml cksum changed to {cksum_after}");
    }
    crew_session_prove::write_composed_report(pack_id, &fixture, &out, &pack_stage, &cksum_after)?;

    let body = json!({
        "schema": PROVE_SCHEMA,
        "ok": true,
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "live_sync": false,
        "estate_cksum": cksum_after,
        "control_plane_schema": CONTROL_SCHEMA,
        "control_plane_report": out.join("control-plane-prove.json").display().to_string(),
        "fuel": control["fuel"].clone(),
        "decide": control["decide"].clone(),
        "run": control["run"].clone(),
        "specialty_real": specialty_real,
        "pack": crew_session_prove::pack_report(pack_id, &fixture, &out, &pack_stage),
        "note": "Fixture prove. Composes control-plane-prove with optional specialty-real seats, then crew-session-prove (plugin-install-local + multi-hop CLI crew session). Hop 2 sees hop 1 on one session_id. Research stays refuse:pack-orchestrator. Real import-trained GGUFs bind as named seats when present; otherwise skipped:gguf-absent. Mock complete. Mock runner. Throwaway HOME. Cursor MCP loader hang is out of scope. Not a live PASS."
    });
    let pretty = serde_json::to_string_pretty(&body)?;
    if pretty.split_whitespace().any(|word| word == "enforced") {
        bail!("refuse:cohesion: report invented enforced");
    }
    if pretty.contains("READY_FOR_LIVE_TEST: yes")
        || pretty.contains("\"ready_for_live_test\": true")
        || pretty.contains("\"live_pass_recorded\": true")
        || pretty.contains("\"live_sync\": true")
        || pretty.contains("\"loader_is_live_pass\": true")
    {
        bail!("refuse:cohesion: report invented a live-test ready flag");
    }
    fs::write(out.join("cohesion-prove.json"), format!("{pretty}\n"))?;
    println!("{pretty}");
    println!("cohesion-prove: ok");
    println!("READY_FOR_LIVE_TEST: no");
    Ok(())
}

fn specialty_real_stage(root: &Path, out: &Path) -> Result<Value> {
    let found = discover_specialty_ggufs(root)?;
    let mut seats = serde_json::Map::new();
    let mut requests = Vec::new();
    for (binding, path) in [(BINDING_AG, found.ag.as_ref()), (BINDING_RUST, found.rust.as_ref())]
    {
        match path {
            None => {
                println!("specialty-real {binding}: {SKIP_GGUF_ABSENT}");
                seats.insert(
                    binding.to_string(),
                    json!({
                        "status": "skipped",
                        "reason": SKIP_GGUF_ABSENT,
                        "binding_id": binding,
                    }),
                );
            }
            Some(gguf) => match specialty_bind::named_seat_request(binding, gguf.clone()) {
                Some(req) => requests.push(req),
                None => bail!("refuse:specialty-real: unknown binding {binding}"),
            },
        }
    }
    if requests.is_empty() {
        println!("specialty-real: {SKIP_GGUF_ABSENT}");
        return Ok(json!({
            "stage": "skipped",
            "reason": SKIP_GGUF_ABSENT,
            "ok": true,
            "ready_for_live_test": false,
            "live_pass_recorded": false,
            "seats": seats,
        }));
    }

    let lab_out = out.join("specialty-real");
    let _ = fs::remove_dir_all(&lab_out);
    fs::create_dir_all(&lab_out)?;
    let landed = specialty_bind::land_named_specialty_seats(root, &lab_out, &requests)?;
    for seat in &landed.seats {
        specialty_bind::mock_complete(
            &landed.lab,
            &landed.state,
            &seat.agent,
            &format!("ping {} specialty", seat.binding_id),
        )?;
        let receipt = last_complete_receipt(&landed.state, &seat.agent)?;
        let result = receipt
            .get("result")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let capability = receipt
            .get("capability")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if result != seat.binding_id || capability != seat.binding_id {
            bail!(
                "refuse:specialty-real: {} complete named result={result} capability={capability}",
                seat.binding_id
            );
        }
        if result == "local_slm" || capability == "local_slm" {
            bail!("refuse:specialty-real: complete named generic local_slm");
        }
        if receipt.get("surface").and_then(Value::as_str) != Some("complete") {
            bail!("refuse:specialty-real: {} surface is not complete", seat.binding_id);
        }
        if receipt.get("outcome").and_then(Value::as_str) != Some("allow") {
            bail!("refuse:specialty-real: {} outcome is not allow", seat.binding_id);
        }
        println!(
            "specialty-real {}: bound seat={} complete result={} capability={}",
            seat.binding_id, seat.seat_model, result, capability
        );
        seats.insert(
            seat.binding_id.clone(),
            json!({
                "status": "bound",
                "binding_id": seat.binding_id,
                "function": seat.function,
                "seat_model": seat.seat_model,
                "agent": seat.agent,
                "gguf": seat.gguf.display().to_string(),
                "complete": {
                    "ok": true,
                    "outcome": "allow",
                    "surface": "complete",
                    "result": result,
                    "capability": capability,
                },
            }),
        );
    }
    let bound: Vec<&str> = landed.seats.iter().map(|s| s.binding_id.as_str()).collect();
    println!("specialty-real: bound {}", bound.join(","));
    Ok(json!({
        "stage": "bound",
        "reason": Value::Null,
        "ok": true,
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "lab_estate": landed.lab.display().to_string(),
        "state_dir": landed.state.display().to_string(),
        "seats": seats,
    }))
}

struct DiscoveredGgufs {
    ag: Option<PathBuf>,
    rust: Option<PathBuf>,
}

fn discover_specialty_ggufs(root: &Path) -> Result<DiscoveredGgufs> {
    let ag = env_regular_gguf(ENV_AG_GGUF)
        .or_else(|| scan_binding_gguf(scan_root(root).as_deref(), BINDING_AG));
    let rust = env_regular_gguf(ENV_RUST_GGUF)
        .or_else(|| scan_binding_gguf(scan_root(root).as_deref(), BINDING_RUST));
    Ok(DiscoveredGgufs { ag, rust })
}

fn scan_root(root: &Path) -> Option<PathBuf> {
    match std::env::var_os(ENV_GGUF_ROOT) {
        Some(raw) if !raw.is_empty() => {
            let path = PathBuf::from(raw);
            path.is_dir().then_some(path)
        }
        _ => {
            let cell = root.join(".cell");
            cell.is_dir().then_some(cell)
        }
    }
}

fn env_regular_gguf(key: &str) -> Option<PathBuf> {
    let raw = std::env::var_os(key)?;
    if raw.is_empty() {
        return None;
    }
    accept_regular_gguf(&PathBuf::from(raw))
}

fn scan_binding_gguf(root: Option<&Path>, binding: &str) -> Option<PathBuf> {
    let root = root?;
    let mut found = None;
    let mut stack = vec![(root.to_path_buf(), 0u8)];
    let mut seen = 0u32;
    while let Some((dir, depth)) = stack.pop() {
        if depth > 8 || seen > 2000 {
            break;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(rows) => rows,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            seen += 1;
            if seen > 2000 {
                break;
            }
            let path = entry.path();
            let meta = match entry.file_type() {
                Ok(kind) => kind,
                Err(_) => continue,
            };
            if meta.is_symlink() {
                continue;
            }
            if meta.is_dir() {
                stack.push((path, depth + 1));
                continue;
            }
            if !meta.is_file() {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if name == "specialty-join.json" {
                if let Some(gguf) = gguf_from_join(&path, binding) {
                    found = Some(gguf);
                }
                continue;
            }
            if name.ends_with(".gguf") && path_mentions_binding(&path, binding) {
                if let Some(gguf) = accept_regular_gguf(&path) {
                    found = Some(gguf);
                }
            }
        }
    }
    found
}

fn gguf_from_join(path: &Path, binding: &str) -> Option<PathBuf> {
    let text = fs::read_to_string(path).ok()?;
    let row: Value = serde_json::from_str(&text).ok()?;
    let function = row.get("function").and_then(Value::as_str)?;
    let binding_id = row.get("binding_id").and_then(Value::as_str).unwrap_or("");
    if function != binding && binding_id != binding {
        return None;
    }
    let gguf = row.get("gguf").and_then(Value::as_str)?;
    accept_regular_gguf(Path::new(gguf))
}

fn path_mentions_binding(path: &Path, binding: &str) -> bool {
    path.components().any(|part| {
        part.as_os_str()
            .to_str()
            .map(|name| name == binding)
            .unwrap_or(false)
    })
}

fn accept_regular_gguf(path: &Path) -> Option<PathBuf> {
    let meta = fs::symlink_metadata(path).ok()?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return None;
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !name.ends_with(".gguf") {
        return None;
    }
    path.canonicalize().ok()
}

fn last_complete_receipt(state: &Path, agent: &str) -> Result<Value> {
    let path = state.join("decisions").join("receipts.jsonl");
    let text = fs::read_to_string(&path)
        .with_context(|| format!("refuse:specialty-real: read {}", path.display()))?;
    let mut last = None;
    for line in text.lines().filter(|line| !line.is_empty()) {
        let row: Value = serde_json::from_str(line)
            .with_context(|| format!("refuse:specialty-real: parse {line}"))?;
        if row.get("agent").and_then(Value::as_str) == Some(agent)
            && row.get("surface").and_then(Value::as_str) == Some("complete")
        {
            last = Some(row);
        }
    }
    last.ok_or_else(|| {
        anyhow::anyhow!("refuse:specialty-real: no complete receipt for {agent}")
    })
}

fn require_control_plane(cp: &Value) -> Result<()> {
    if cp.get("schema").and_then(Value::as_str) != Some(CONTROL_SCHEMA) {
        bail!("refuse:cohesion: control-plane schema");
    }
    if cp.get("ok") != Some(&Value::Bool(true))
        || cp.get("ready_for_live_test") != Some(&Value::Bool(false))
        || cp.get("live_pass_recorded") != Some(&Value::Bool(false))
        || cp.get("live_sync") != Some(&Value::Bool(false))
    {
        bail!("refuse:cohesion: control-plane report is not a mock ok");
    }
    if cp["fuel"]["trained_shape"] != "gguf"
        || cp["fuel"]["auto_apply"] != false
        || cp["fuel"]["beside"] != "local_slm"
        || cp["fuel"]["joinable"]["ag_news"] != true
        || cp["fuel"]["joinable"]["rust_idiom"] != true
        || cp["fuel"]["seat_models"]["ag_news"] != "specialist-agnews-3000"
        || cp["fuel"]["seat_models"]["rust_idiom"] != "specialist-rustidiom-3000"
    {
        bail!("refuse:cohesion: fuel join is not the dual gguf bind");
    }
    if cp["decide"]["surface_authorize"] != 4
        || cp["decide"]["surface_convey"] != 4
        || cp["decide"]["surface_complete"] != 5
        || cp["decide"]["abstain"] != "refuse:decision-abstain"
        || cp["decide"]["stale_fallback"] != "ag_news"
        || cp["decide"]["ineligible_fallback"] != "ag_news"
    {
        bail!("refuse:cohesion: decide surfaces are not the host-validate counts");
    }
    let caps = cp["decide"]["capabilities"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let has = |name: &str| caps.iter().any(|row| row.as_str() == Some(name));
    if !has("ag_news") || !has("rust_idiom") {
        bail!("refuse:cohesion: decide capabilities missing specialty seats");
    }
    if cp["run"]["routine_id"] != "standing-dual"
        || cp["run"]["session_stitch"] != true
        || cp["run"]["package"] != "dual-specialty"
    {
        bail!("refuse:cohesion: run is not the standing-dual session stitch");
    }
    let chain = cp["run"]["chain_id"].as_str().unwrap_or("");
    if !chain.starts_with("chain-dual-specialty-") {
        bail!("refuse:cohesion: chain_id is {chain}");
    }
    let hops = cp["run"]["hops"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("refuse:cohesion: run hops missing"))?;
    let session = hops
        .first()
        .and_then(|row| row.get("session_id"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if session.is_empty() {
        bail!("refuse:cohesion: runner session_id is empty");
    }
    if hops.len() != 3
        || hops[0]["capability"] != "ag_news"
        || hops[0]["context"] != "none"
        || hops[1]["capability"] != "rust_idiom"
        || hops[1]["context"] != "applied"
        || hops[2]["capability"] != "frontier_http"
        || hops[2]["context"] != "applied"
        || hops[0]["session_id"] != hops[1]["session_id"]
        || hops[0]["session_id"] != hops[2]["session_id"]
    {
        bail!("refuse:cohesion: runner hops are not the 3-hop session stitch");
    }
    println!(
        "control-plane cited: joinable ag_news=yes rust_idiom=yes surfaces authorize=4 convey=4 complete=5 session_stitch=yes"
    );
    Ok(())
}

fn resolve_fixture(root: &Path, estate: &Path) -> Result<PathBuf> {
    let path = if estate.is_absolute() {
        estate.to_path_buf()
    } else {
        root.join(estate)
    };
    path.canonicalize()
        .with_context(|| format!("refuse:estate: {}", path.display()))
}

fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("refuse:read: {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("refuse:read: parse {}", path.display()))
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
    std::env::temp_dir().join(format!("cell-one-cohesion-{token}"))
}
