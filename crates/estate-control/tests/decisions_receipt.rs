//! Decision receipts on `estate convey call`.
//!
//! The host prepares eligible model-binding ids, a selector chooses or
//! abstains, and the host re-validates. The selector does not grant
//! permission. A fallback is recorded and is not applied.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("cell-decision-receipt-{name}-{nanos}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args(args)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .env_remove("CELL_LOCAL_LIVE")
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn assert_locked_cksum() {
    let sum = Command::new("cksum")
        .arg(repo_root().join("examples/estate.yaml"))
        .output()
        .unwrap();
    let sum_text = String::from_utf8_lossy(&sum.stdout);
    assert!(
        sum_text.starts_with("43770130 3391"),
        "examples/estate.yaml cksum changed: {sum_text}"
    );
}

fn apply(estate: &Path, state: &Path, scratch_root: &Path) {
    let plans = scratch_root.join("plans");
    let roots = scratch_root.join("roots");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::create_dir_all(&roots).unwrap();
    std::fs::create_dir_all(state).unwrap();
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let roots_s = roots.display().to_string();
    let (ok, stdout, stderr) = run(&[
        "apply",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--roots-base",
        &roots_s,
        "--plans-dir",
        &plans_s,
    ]);
    assert!(ok, "{stdout}\n{stderr}");
}

fn policy() -> PathBuf {
    repo_root().join("policy/cell-one.policy.v0.yaml")
}

fn convey_call(estate: &Path, state: &Path, extra: &[&str]) -> (bool, String, String) {
    let policy = policy();
    let mut args = vec![
        "convey".to_string(),
        "call".to_string(),
        "--estate".to_string(),
        estate.display().to_string(),
        "--state-dir".to_string(),
        state.display().to_string(),
        "--policy".to_string(),
        policy.display().to_string(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_string()));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run(&refs)
}

fn granted_box(capability: &str) -> conveyor_proxy::ConveyorMesh {
    let mut mesh = conveyor_proxy::ConveyorMesh {
        schema: conveyor_proxy::MESH_SCHEMA.into(),
        hops: vec![],
        leases: vec![conveyor_proxy::HopLease {
            hop_id: "cell-one-box".into(),
            kind: "box".into(),
            capability: capability.into(),
            host_class: "any".into(),
            granted: true,
            spawned: true,
            durable: true,
            driver: "box".into(),
            note: None,
            ttl_secs: None,
            issued_at: None,
            expires_at: None,
            agents: vec!["research".into()],
        }],
    };
    mesh.hops.push(conveyor_proxy::HopDecl {
        id: "cell-one-box".into(),
        kind: "box".into(),
        capability: capability.into(),
        host_class: "any".into(),
        wired: true,
        note: None,
        ttl_secs: None,
        agents: vec!["research".into()],
    });
    mesh
}

fn ttl_decl(agents: Vec<String>) -> conveyor_proxy::HopDecl {
    conveyor_proxy::HopDecl {
        id: "ttl-hop".into(),
        kind: "box".into(),
        capability: "lane-tool".into(),
        host_class: "any".into(),
        wired: true,
        note: None,
        ttl_secs: None,
        agents,
    }
}

fn ttl_lease(agents: Vec<String>) -> conveyor_proxy::HopLease {
    conveyor_proxy::HopLease {
        hop_id: "ttl-hop".into(),
        kind: "box".into(),
        capability: "lane-tool".into(),
        host_class: "any".into(),
        granted: true,
        spawned: true,
        durable: true,
        driver: "box".into(),
        note: None,
        ttl_secs: None,
        issued_at: None,
        expires_at: None,
        agents,
    }
}

fn mesh_with_stub() -> conveyor_proxy::ConveyorMesh {
    let mut mesh = granted_box("notes-append");
    mesh.hops.push(ttl_decl(vec![]));
    mesh.leases.push(ttl_lease(vec![]));
    mesh
}

fn journal(state: &Path) -> PathBuf {
    state.join("decisions").join("receipts.jsonl")
}

fn load_receipts(state: &Path) -> Vec<serde_json::Value> {
    let text = std::fs::read_to_string(journal(state)).unwrap();
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).expect(line))
        .collect()
}

fn cite_line(stdout: &str) -> &str {
    stdout
        .lines()
        .find(|line| line.starts_with("decision receipt: "))
        .unwrap_or_else(|| panic!("missing receipt cite\n{stdout}"))
}

fn research_estate(root: &Path, frontier: bool) -> PathBuf {
    let mut estate = estate_schema::load_estate(&repo_root().join("examples/estate.yaml")).unwrap();
    let research = estate
        .agents
        .iter_mut()
        .find(|agent| agent.id == "research")
        .unwrap();
    research.tools.push(estate_schema::ToolDecl {
        id: "lane-tool".into(),
        description: None,
    });
    if frontier {
        research.models.push(estate_schema::ModelUseDecl {
            id: "xai_grok".into(),
            description: None,
        });
    }
    estate.intentions.push(estate_schema::Intention {
        subject_agent: "research".into(),
        object: "lane-tool".into(),
        kind: estate_schema::IntentionKind::Tool,
        effect: estate_schema::Effect::Allow,
        note: None,
    });
    estate.intentions.push(estate_schema::Intention {
        subject_agent: "research".into(),
        object: "class:local".into(),
        kind: estate_schema::IntentionKind::Model,
        effect: estate_schema::Effect::Allow,
        note: None,
    });
    if frontier {
        estate.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "class:frontier".into(),
            kind: estate_schema::IntentionKind::Model,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
    }
    let yaml = estate_schema::render_estate_yaml(&estate).unwrap();
    let path = root.join("estate.yaml");
    std::fs::write(&path, yaml).unwrap();
    path
}

fn write_hint(state: &Path, select: &str, digest: Option<&str>) {
    let digest = match digest {
        Some(raw) => format!("\"{raw}\""),
        None => "null".into(),
    };
    std::fs::write(
        state.join("decision-select.json"),
        format!(
            r#"{{"schema":"cell-one.decision-select.v0","select":"{select}","digest":{digest}}}"#
        ),
    )
    .unwrap();
}

fn research_mesh() -> conveyor_proxy::ConveyorMesh {
    let mut mesh = granted_box("notes-append");
    mesh.hops.push(ttl_decl(vec!["research".into()]));
    mesh.leases.push(ttl_lease(vec!["research".into()]));
    mesh
}

fn assert_no_invented_reasoning(receipt: &serde_json::Value) {
    assert!(receipt.get("reasoning").is_none(), "{receipt}");
    assert!(receipt.get("explanation").is_none(), "{receipt}");
    let outcome = receipt["outcome"].as_str().unwrap();
    assert!(
        outcome == "allow" || outcome.starts_with("refuse:"),
        "{outcome}"
    );
    assert!(
        !outcome.split_whitespace().any(|word| word == "enforced"),
        "{outcome}"
    );
}

#[test]
fn successful_unnamed_call_writes_a_receipt_and_cites_it() {
    let dir = scratch("unnamed");
    let state = dir.join("state");
    let estate = repo_root().join("examples/estate.yaml");
    apply(&estate, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &mesh_with_stub()).unwrap();
    let (ok, stdout, stderr) = convey_call(
        &estate,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let cite = cite_line(&stdout);
    assert!(cite.contains("result=abstain"), "{cite}");
    assert!(cite.contains("validation=ok"), "{cite}");
    assert!(!cite.contains("fallback="), "{cite}");
    let json_at = stdout.find("\"hop_id\"").unwrap();
    let cite_at = stdout.find(cite).unwrap();
    assert!(cite_at < json_at, "{stdout}");
    let call: serde_json::Value =
        serde_json::from_str(stdout[cite_at + cite.len()..].trim()).unwrap();
    assert_eq!(call["allow"], true);
    assert_eq!(call["reason"], "lease-bound box hop");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row["schema"], "cell-one.decision-receipt.v0");
    assert_eq!(row["seq"], 1);
    assert_eq!(row["hop_id"], "ttl-hop");
    assert_eq!(row["capability"], "lane-tool");
    assert!(row["agent"].is_null());
    assert_eq!(row["stage"], "select");
    assert_eq!(row["candidates"], serde_json::json!([]));
    assert_eq!(row["result"], "abstain");
    assert_eq!(row["validation"], "ok");
    assert!(row["fallback"].is_null());
    assert_eq!(row["outcome"], "allow");
    assert_no_invented_reasoning(row);
    assert!(cite.contains(row["id"].as_str().unwrap()), "{cite}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn named_call_selects_the_one_eligible_local_seat() {
    let dir = scratch("named");
    let state = dir.join("state");
    let estate = research_estate(&dir, false);
    apply(&estate, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &research_mesh()).unwrap();
    let (ok, stdout, stderr) = convey_call(
        &estate,
        &state,
        &[
            "--id",
            "ttl-hop",
            "--capability",
            "lane-tool",
            "--agent",
            "research",
        ],
    );
    assert!(ok, "{stdout}\n{stderr}");
    let cite = cite_line(&stdout);
    assert!(cite.contains("result=local_slm"), "{cite}");
    assert!(cite.contains("validation=ok"), "{cite}");
    assert!(!cite.contains("fallback="), "{cite}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0]["candidates"],
        serde_json::json!([{"id": "local_slm"}])
    );
    assert_eq!(rows[0]["result"], "local_slm");
    assert_eq!(rows[0]["validation"], "ok");
    assert_eq!(rows[0]["stage"], "validate");
    assert_eq!(rows[0]["outcome"], "allow");
    assert_eq!(rows[0]["agent"], "research");
    assert!(rows[0]["fallback"].is_null());
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ineligible_and_stale_hints_record_fallback_without_granting() {
    let dir = scratch("fallback");
    let state = dir.join("state");
    let estate = research_estate(&dir, false);
    apply(&estate, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &research_mesh()).unwrap();
    write_hint(&state, "xai_grok", None);
    let (ok, stdout, stderr) = convey_call(
        &estate,
        &state,
        &[
            "--id",
            "ttl-hop",
            "--capability",
            "lane-tool",
            "--agent",
            "research",
        ],
    );
    assert!(ok, "{stdout}\n{stderr}");
    let cite = cite_line(&stdout);
    assert!(cite.contains("result=xai_grok"), "{cite}");
    assert!(cite.contains("validation=ineligible"), "{cite}");
    assert!(cite.contains("fallback=local_slm"), "{cite}");
    let cite_at = stdout.find(cite).unwrap();
    let call: serde_json::Value =
        serde_json::from_str(stdout[cite_at + cite.len()..].trim()).unwrap();
    assert_eq!(call["allow"], true);
    let reason = call["reason"].as_str().unwrap();
    assert!(reason.contains("allow intention"), "{reason}");
    assert!(!reason.contains("fallback"), "{reason}");
    assert!(!reason.contains("xai_grok"), "{reason}");
    let rows = load_receipts(&state);
    assert_eq!(rows[0]["validation"], "ineligible");
    assert_eq!(rows[0]["result"], "xai_grok");
    assert_eq!(rows[0]["fallback"], "local_slm");
    assert_eq!(rows[0]["stage"], "fallback");
    assert_eq!(rows[0]["outcome"], "allow");
    assert_eq!(
        rows[0]["candidates"],
        serde_json::json!([{"id": "local_slm"}])
    );

    write_hint(&state, "local_slm", Some("0"));
    let (ok, stdout, stderr) = convey_call(
        &estate,
        &state,
        &[
            "--id",
            "ttl-hop",
            "--capability",
            "lane-tool",
            "--agent",
            "research",
        ],
    );
    assert!(ok, "{stdout}\n{stderr}");
    let cite = cite_line(&stdout);
    assert!(cite.contains("validation=stale"), "{cite}");
    assert!(cite.contains("fallback=local_slm"), "{cite}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["seq"], 2);
    assert_eq!(rows[1]["validation"], "stale");
    assert_eq!(rows[1]["result"], "local_slm");
    assert_eq!(rows[1]["fallback"], "local_slm");
    assert_eq!(rows[1]["stage"], "fallback");
    assert_eq!(rows[1]["outcome"], "allow");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn equal_class_frontier_and_local_abstain() {
    let dir = scratch("equal");
    let state = dir.join("state");
    let estate = research_estate(&dir, true);
    apply(&estate, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &research_mesh()).unwrap();
    let (ok, stdout, stderr) = convey_call(
        &estate,
        &state,
        &[
            "--id",
            "ttl-hop",
            "--capability",
            "lane-tool",
            "--agent",
            "research",
        ],
    );
    assert!(ok, "{stdout}\n{stderr}");
    let cite = cite_line(&stdout);
    assert!(cite.contains("result=abstain"), "{cite}");
    assert!(cite.contains("validation=ok"), "{cite}");
    let rows = load_receipts(&state);
    assert_eq!(
        rows[0]["candidates"],
        serde_json::json!([{"id": "xai_grok"}, {"id": "local_slm"}])
    );
    assert_eq!(rows[0]["result"], "abstain");
    assert_eq!(rows[0]["stage"], "select");
    assert!(rows[0]["fallback"].is_null());
    assert_eq!(rows[0]["outcome"], "allow");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn export_jsonl_round_trips_and_report_counts() {
    let dir = scratch("export");
    let state = dir.join("state");
    let estate = repo_root().join("examples/estate.yaml");
    apply(&estate, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &mesh_with_stub()).unwrap();
    let (ok, stdout, stderr) = convey_call(
        &estate,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(ok, "{stdout}\n{stderr}");
    let out = dir.join("replay.jsonl");
    let state_s = state.display().to_string();
    let out_s = out.display().to_string();
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "export",
        "--state-dir",
        &state_s,
        "--out",
        &out_s,
    ]);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stdout.contains("wrote 1 decision replay case"), "{stdout}");
    let journal_bytes = std::fs::read(journal(&state)).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), journal_bytes);
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["outcome"], "allow");

    let (ok, stdout, stderr) = run(&["decisions", "report", "--state-dir", &state_s]);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stdout.contains("decision receipts: 1\n"), "{stdout}");
    assert!(
        stdout.contains("stage prepare=1 select=1 validate=0 fallback=0\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("validation ok=1 stale=0 ineligible=0 expired=0\n"),
        "{stdout}"
    );
    assert!(stdout.contains("fallback none=1\n"), "{stdout}");
    assert!(
        !stdout.split_whitespace().any(|word| word == "enforced"),
        "{stdout}"
    );
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_journal_export_and_report_are_empty() {
    let dir = scratch("empty");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let out = dir.join("replay.jsonl");
    let state_s = state.display().to_string();
    let out_s = out.display().to_string();
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "export",
        "--state-dir",
        &state_s,
        "--out",
        &out_s,
    ]);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stdout.contains("wrote 0 decision replay case"), "{stdout}");
    assert_eq!(std::fs::read(&out).unwrap(), b"");
    let (ok, stdout, stderr) = run(&["decisions", "report", "--state-dir", &state_s]);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stdout.contains("decision receipts: 0\n"), "{stdout}");
    assert!(
        stdout.contains("stage prepare=0 select=0 validate=0 fallback=0\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("validation ok=0 stale=0 ineligible=0 expired=0\n"),
        "{stdout}"
    );
    assert!(stdout.contains("fallback none=0\n"), "{stdout}");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn placement_sku_mesh_parse_and_agent_unplaced_write_no_receipt() {
    let dir = scratch("refuse-early");
    let state = dir.join("state");
    let estate = repo_root().join("examples/estate.yaml");
    apply(&estate, &state, &dir);
    conveyor_proxy::persist_mesh(&state, &mesh_with_stub()).unwrap();

    let placement = state.join("placement-actual.json");
    let mut actual: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&placement).unwrap()).unwrap();
    let rows = actual["leases"].as_array_mut().unwrap();
    let row = rows
        .iter_mut()
        .find(|row| row["placement_id"] == "cell-one-box")
        .unwrap();
    row["host_class"] = serde_json::Value::String("rtx-5090".into());
    std::fs::write(&placement, serde_json::to_string_pretty(&actual).unwrap()).unwrap();
    let (ok, stdout, stderr) = convey_call(
        &estate,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("refuse:bad-host-class"), "{stderr}");
    assert!(stderr.contains("rtx-5090"), "{stderr}");
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    assert!(!stdout.contains("\"allow\""), "{stdout}");
    assert!(!journal(&state).exists());

    std::fs::write(state.join("conveyor-mesh.json"), "not-json").unwrap();
    let (ok, stdout, stderr) = convey_call(
        &estate,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(stderr.contains("parse:"), "{stderr}");
    assert!(!journal(&state).exists());

    conveyor_proxy::persist_mesh(&state, &mesh_with_stub()).unwrap();
    let mut ahead = mesh_with_stub();
    ahead.hops[0].agents = vec!["research".into(), "outsider".into()];
    ahead.leases[0].agents = vec!["research".into(), "outsider".into()];
    conveyor_proxy::persist_mesh(&state, &ahead).unwrap();
    let restored = std::fs::read_to_string(&placement).unwrap();
    let mut actual: serde_json::Value = serde_json::from_str(&restored).unwrap();
    for row in actual["leases"].as_array_mut().unwrap() {
        if row["placement_id"] == "cell-one-box" {
            row["host_class"] = serde_json::Value::String("any".into());
        }
    }
    std::fs::write(&placement, serde_json::to_string_pretty(&actual).unwrap()).unwrap();
    let (ok, stdout, stderr) = convey_call(
        &estate,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stdout.trim().is_empty(), "{stdout}");
    assert!(stderr.contains("refuse:agent-unplaced"), "{stderr}");
    assert!(stderr.contains("outsider"), "{stderr}");
    assert!(!journal(&state).exists());
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_lease_records_a_receipt_without_a_cite() {
    let dir = scratch("no-lease");
    let state = dir.join("state");
    let estate = repo_root().join("examples/estate.yaml");
    apply(&estate, &state, &dir);
    let (ok, stdout, stderr) = convey_call(
        &estate,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains("refuse:no-lease") && stderr.contains("ttl-hop"),
        "{stderr}"
    );
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    assert!(stdout.contains("Agents\n------"), "{stdout}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["outcome"], "refuse:no-lease");
    assert_eq!(rows[0]["result"], "abstain");
    assert_eq!(rows[0]["validation"], "ok");
    assert_eq!(rows[0]["stage"], "select");
    assert!(rows[0]["fallback"].is_null());
    assert!(!state.join("conveyor-mesh.json").exists());
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn expired_lease_records_expired_and_clears_fallback() {
    let dir = scratch("expired");
    let state = dir.join("state");
    let estate = research_estate(&dir, false);
    apply(&estate, &state, &dir);
    let mut mesh = research_mesh();
    let lease = mesh
        .leases
        .iter_mut()
        .find(|lease| lease.hop_id == "ttl-hop")
        .unwrap();
    lease.ttl_secs = Some(1);
    lease.issued_at = Some(1);
    lease.expires_at = Some(1);
    conveyor_proxy::persist_mesh(&state, &mesh).unwrap();
    write_hint(&state, "local_slm", Some("0"));
    let (ok, stdout, stderr) = convey_call(
        &estate,
        &state,
        &[
            "--id",
            "ttl-hop",
            "--capability",
            "lane-tool",
            "--agent",
            "research",
        ],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("refuse:expired"), "{stderr}");
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["validation"], "expired");
    assert_eq!(rows[0]["result"], "local_slm");
    assert!(rows[0]["fallback"].is_null());
    assert_eq!(rows[0]["stage"], "validate");
    assert_eq!(rows[0]["outcome"], "refuse:expired");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn malformed_hint_refuses_before_the_receipt_and_before_restamp() {
    let dir = scratch("hint");
    let state = dir.join("state");
    let estate = repo_root().join("examples/estate.yaml");
    apply(&estate, &state, &dir);
    let mut mesh = granted_box("notes-append");
    mesh.hops.push(ttl_decl(vec![]));
    conveyor_proxy::persist_mesh(&state, &mesh).unwrap();
    std::fs::write(state.join("decision-select.json"), "{").unwrap();
    let (ok, stdout, stderr) = convey_call(
        &estate,
        &state,
        &["--id", "ttl-hop", "--capability", "lane-tool"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("refuse:decision-select"), "{stderr}");
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    assert!(!journal(&state).exists());
    let body = std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap();
    let mesh: serde_json::Value = serde_json::from_str(&body).unwrap();
    let leases = mesh["leases"].as_array().unwrap();
    assert!(
        leases.iter().all(|lease| lease["hop_id"] != "ttl-hop"),
        "{body}"
    );
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}
