//! Decision receipts on `estate complete`.
//!
//! Host prepare → select → authorize → data-plane complete → journal.
//! The selector does not grant. Missing key or endpoint fail-closes.

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
    let dir = std::env::temp_dir().join(format!("cell-complete-receipt-{name}-{nanos}"));
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

fn complete(estate: &Path, state: &Path, extra: &[&str]) -> (bool, String, String) {
    let mut args = vec![
        "complete".to_string(),
        "--estate".to_string(),
        estate.display().to_string(),
        "--state-dir".to_string(),
        state.display().to_string(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_string()));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run(&refs)
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

fn completion_body(stdout: &str) -> serde_json::Value {
    let start = stdout
        .find("{\n")
        .unwrap_or_else(|| panic!("missing completion JSON\n{stdout}"));
    serde_json::from_str(stdout[start..].trim()).unwrap()
}

fn no_invented_pass(stdout: &str, stderr: &str) {
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stderr.contains("live PASS"), "{stderr}");
    for text in [stdout, stderr] {
        assert!(
            !text.split_whitespace().any(|word| word == "enforced"),
            "{text}"
        );
    }
}

fn assert_receipt_shape(row: &serde_json::Value) {
    assert_eq!(row["schema"], "cell-one.decision-receipt.v0");
    assert_eq!(row["surface"], "complete");
    assert!(row.get("reasoning").is_none(), "{row}");
    assert!(row.get("explanation").is_none(), "{row}");
    let outcome = row["outcome"].as_str().unwrap();
    assert!(
        outcome == "allow" || outcome.starts_with("refuse:"),
        "{outcome}"
    );
    assert!(
        !outcome.split_whitespace().any(|word| word == "enforced"),
        "{outcome}"
    );
}

fn write_estate(root: &Path, estate: &estate_schema::Estate) -> PathBuf {
    estate_schema::validate(estate).unwrap();
    let yaml = estate_schema::render_estate_yaml(estate).unwrap();
    let path = root.join("estate.yaml");
    std::fs::write(&path, yaml).unwrap();
    path
}

fn base_estate() -> estate_schema::Estate {
    estate_schema::load_estate(&repo_root().join("examples/estate.yaml")).unwrap()
}

fn push_local(estate: &mut estate_schema::Estate, id: &str) {
    let mut seat = estate
        .model_bindings
        .iter()
        .find(|binding| binding.id == "local_slm")
        .unwrap()
        .clone();
    seat.id = id.into();
    estate.model_bindings.push(seat);
}

fn allow_model(estate: &mut estate_schema::Estate, agent: &str, object: &str) {
    estate.intentions.push(estate_schema::Intention {
        subject_agent: agent.into(),
        object: object.into(),
        kind: estate_schema::IntentionKind::Model,
        effect: estate_schema::Effect::Allow,
        note: None,
    });
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

fn specialty_estate(root: &Path) -> PathBuf {
    let mut estate = base_estate();
    push_local(&mut estate, "ag_news");
    push_local(&mut estate, "policy_precheck");
    let research = estate
        .agents
        .iter_mut()
        .find(|agent| agent.id == "research")
        .unwrap();
    research.models = vec![estate_schema::ModelUseDecl {
        id: "ag_news".into(),
        description: None,
    }];
    allow_model(&mut estate, "research", "ag_news");
    write_estate(root, &estate)
}

fn spawn_compat(script: model_estate::CompatScript) -> model_estate::CompatServer {
    for _ in 0..40 {
        let srv = model_estate::CompatServer::spawn(script.clone()).unwrap();
        if !estate_schema::contains_sku(&srv.endpoint()) {
            return srv;
        }
    }
    panic!("ephemeral port kept encoding a hardware SKU");
}

#[test]
fn one_eligible_specialty_seat_completes_and_journals() {
    let dir = scratch("specialty");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = specialty_estate(&dir);
    let (ok, stdout, stderr) = complete(
        &estate,
        &state,
        &[
            "--agent",
            "research",
            "--prompt",
            "hello from the factory",
            "--mock",
        ],
    );
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    let cite = cite_line(&stdout);
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("decision receipt: "))
            .count(),
        1,
        "{stdout}"
    );
    assert!(cite.contains("result=ag_news"), "{cite}");
    assert!(cite.contains("validation=ok"), "{cite}");
    assert!(!cite.contains("fallback="), "{cite}");
    let cite_at = stdout.find(cite).unwrap();
    let json_at = stdout.find("{\n").unwrap();
    assert!(cite_at < json_at, "{stdout}");
    let body = completion_body(&stdout);
    assert_eq!(body["allow"], true);
    assert_eq!(body["job"], "complete");
    assert_eq!(body["completion"], "mock:hello from the factory");
    assert!(!stdout.contains("Agents\n"), "{stdout}");
    assert!(!stdout.contains("not-enforced"), "{stdout}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_receipt_shape(row);
    assert_eq!(row["seq"], 1);
    assert_eq!(row["hop_id"], "model");
    assert_eq!(row["capability"], "ag_news");
    assert_eq!(row["agent"], "research");
    assert_eq!(row["stage"], "validate");
    assert_eq!(row["candidates"], serde_json::json!([{"id": "ag_news"}]));
    assert_eq!(row["result"], "ag_news");
    assert_eq!(row["validation"], "ok");
    assert!(row["fallback"].is_null());
    assert_eq!(row["outcome"], "allow");
    assert!(cite.contains(row["id"].as_str().unwrap()), "{cite}");
    no_invented_pass(&stdout, &stderr);
    assert!(!state.join("conveyor-mesh.json").exists());

    let out = dir.join("replay.jsonl");
    let state_s = state.display().to_string();
    let out_s = out.display().to_string();
    let (ok, export_out, export_err) = run(&[
        "decisions",
        "export",
        "--state-dir",
        &state_s,
        "--out",
        &out_s,
    ]);
    assert!(ok, "{export_out}\n{export_err}");
    assert!(
        export_out.contains("wrote 1 decision replay case"),
        "{export_out}"
    );
    let (ok, report, report_err) = run(&["decisions", "report", "--state-dir", &state_s]);
    assert!(ok, "{report}\n{report_err}");
    assert!(report.contains("decision receipts: 1\n"), "{report}");
    assert!(
        report.contains("stage prepare=1 select=0 validate=1 fallback=0\n"),
        "{report}"
    );
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn http_local_complete_writes_receipt() {
    let dir = scratch("http-local");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = specialty_estate(&dir);
    let srv = model_estate::MockLocalServer::spawn().unwrap();
    let endpoint = srv.endpoint();
    let (ok, stdout, stderr) = complete(
        &estate,
        &state,
        &[
            "--agent",
            "research",
            "--prompt",
            "hello from the factory",
            "--endpoint",
            &endpoint,
        ],
    );
    assert!(ok, "{stdout}\n{stderr}");
    let cite = cite_line(&stdout);
    assert!(cite.contains("result=ag_news"), "{cite}");
    let body = completion_body(&stdout);
    assert_eq!(body["allow"], true);
    assert_eq!(body["job"], "complete");
    assert_eq!(body["completion"], "mock:hello from the factory");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_receipt_shape(&rows[0]);
    assert_eq!(rows[0]["outcome"], "allow");
    assert_eq!(rows[0]["result"], "ag_news");
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn http_frontier_complete_writes_receipt() {
    let dir = scratch("http-frontier");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let mut estate = base_estate();
    let horizon = estate
        .agents
        .iter_mut()
        .find(|agent| agent.id == "horizon")
        .unwrap();
    horizon.models = vec![estate_schema::ModelUseDecl {
        id: "xai_grok".into(),
        description: None,
    }];
    allow_model(&mut estate, "horizon", "xai_grok");
    let estate_path = write_estate(&dir, &estate);
    let srv = spawn_compat(model_estate::CompatScript::OpenAi {
        models: vec!["gateway-model".into()],
    });
    let endpoint = srv.endpoint();
    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args([
            "complete",
            "--estate",
            &estate_path.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--agent",
            "horizon",
            "--prompt",
            "Reply with the single word pong.",
            "--endpoint",
            &endpoint,
        ])
        .env("XAI_API_KEY", "test-not-a-secret")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stdout}\n{stderr}");
    let cite = cite_line(&stdout);
    assert!(cite.contains("result=xai_grok"), "{cite}");
    let body = completion_body(&stdout);
    assert_eq!(body["allow"], true);
    assert_eq!(body["job"], "complete");
    assert_eq!(body["completion"], "ok");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_receipt_shape(&rows[0]);
    assert_eq!(rows[0]["result"], "xai_grok");
    assert_eq!(rows[0]["outcome"], "allow");
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_local_endpoint_fail_closes() {
    let dir = scratch("no-endpoint");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = specialty_estate(&dir);
    let (ok, stdout, stderr) = complete(
        &estate,
        &state,
        &["--agent", "research", "--prompt", "ping"],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains("CELL_LOCAL_ENDPOINT") || stderr.contains("endpoint"),
        "{stderr}"
    );
    assert!(!stdout.contains("\"completion\""), "{stdout}");
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_receipt_shape(&rows[0]);
    assert_eq!(rows[0]["outcome"], "refuse:missing-endpoint");
    assert_eq!(rows[0]["result"], "ag_news");
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_frontier_key_fail_closes() {
    let dir = scratch("no-key");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let mut estate = base_estate();
    let horizon = estate
        .agents
        .iter_mut()
        .find(|agent| agent.id == "horizon")
        .unwrap();
    horizon.models = vec![estate_schema::ModelUseDecl {
        id: "xai_grok".into(),
        description: None,
    }];
    allow_model(&mut estate, "horizon", "xai_grok");
    let estate_path = write_estate(&dir, &estate);
    let (ok, stdout, stderr) = complete(
        &estate_path,
        &state,
        &[
            "--agent",
            "horizon",
            "--prompt",
            "Reply with the single word pong.",
        ],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("XAI_API_KEY"), "{stderr}");
    assert!(!stderr.contains("xai-"), "{stderr}");
    assert!(!stdout.contains("\"completion\""), "{stdout}");
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_receipt_shape(&rows[0]);
    assert_eq!(rows[0]["outcome"], "refuse:missing-creds");
    assert_eq!(rows[0]["result"], "xai_grok");
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn equal_class_abstain_refuses_without_object() {
    let dir = scratch("abstain");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let mut estate = base_estate();
    estate
        .agents
        .iter_mut()
        .find(|agent| agent.id == "research")
        .unwrap()
        .models
        .push(estate_schema::ModelUseDecl {
            id: "xai_grok".into(),
            description: None,
        });
    allow_model(&mut estate, "research", "class:local");
    allow_model(&mut estate, "research", "class:frontier");
    let estate_path = write_estate(&dir, &estate);
    let (ok, stdout, stderr) = complete(
        &estate_path,
        &state,
        &[
            "--agent",
            "research",
            "--prompt",
            "ping",
            "--mock",
        ],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("refuse:decision-abstain"), "{stderr}");
    assert!(!stdout.contains("\"completion\""), "{stdout}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_receipt_shape(&rows[0]);
    assert_eq!(rows[0]["result"], "abstain");
    assert_eq!(rows[0]["outcome"], "refuse:decision-abstain");
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn named_object_completes_when_selector_abstains() {
    let dir = scratch("named");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let mut estate = base_estate();
    estate
        .agents
        .iter_mut()
        .find(|agent| agent.id == "research")
        .unwrap()
        .models
        .push(estate_schema::ModelUseDecl {
            id: "xai_grok".into(),
            description: None,
        });
    allow_model(&mut estate, "research", "class:local");
    allow_model(&mut estate, "research", "class:frontier");
    let estate_path = write_estate(&dir, &estate);
    let (ok, stdout, stderr) = complete(
        &estate_path,
        &state,
        &[
            "--agent",
            "research",
            "--object",
            "local_slm",
            "--prompt",
            "hello from the factory",
            "--mock",
        ],
    );
    assert!(ok, "{stdout}\n{stderr}");
    let cite = cite_line(&stdout);
    assert!(cite.contains("result=abstain"), "{cite}");
    let body = completion_body(&stdout);
    assert_eq!(body["completion"], "mock:hello from the factory");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_receipt_shape(&rows[0]);
    assert_eq!(rows[0]["result"], "abstain");
    assert_eq!(rows[0]["capability"], "local_slm");
    assert_eq!(rows[0]["outcome"], "allow");
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_hint_refuses_before_the_receipt() {
    let dir = scratch("hint");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = specialty_estate(&dir);
    std::fs::write(state.join("decision-select.json"), "{").unwrap();
    let (ok, stdout, stderr) = complete(
        &estate,
        &state,
        &[
            "--agent",
            "research",
            "--prompt",
            "ping",
            "--mock",
        ],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("refuse:decision-select"), "{stderr}");
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    assert!(!journal(&state).exists());
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn authorize_deny_fails_closed_and_still_records() {
    let dir = scratch("deny");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = repo_root().join("examples/estate.yaml");
    let (ok, stdout, stderr) = complete(
        &estate,
        &state,
        &[
            "--agent",
            "research",
            "--object",
            "local_slm",
            "--prompt",
            "ping",
            "--mock",
        ],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("authorize denied"), "{stderr}");
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    let body = completion_body(&stdout);
    assert_eq!(body["decision"], "deny");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_receipt_shape(&rows[0]);
    assert_eq!(rows[0]["outcome"], "refuse:deny-default");
    assert_eq!(rows[0]["capability"], "local_slm");
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn journal_write_failure_after_complete_still_prints() {
    let dir = scratch("journal-fail");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("decisions"), "not-a-directory\n").unwrap();
    let estate = specialty_estate(&dir);
    let (ok, stdout, stderr) = complete(
        &estate,
        &state,
        &[
            "--agent",
            "research",
            "--prompt",
            "hello from the factory",
            "--mock",
        ],
    );
    assert!(ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains("decision receipt: journal write failed after complete commit:"),
        "{stderr}"
    );
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    let body = completion_body(&stdout);
    assert_eq!(body["completion"], "mock:hello from the factory");
    assert_eq!(
        std::fs::read_to_string(state.join("decisions")).unwrap(),
        "not-a-directory\n"
    );
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stale_hint_does_not_grant_without_object() {
    let dir = scratch("stale");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = specialty_estate(&dir);
    write_hint(&state, "ag_news", Some("0"));
    let (ok, stdout, stderr) = complete(
        &estate,
        &state,
        &[
            "--agent",
            "research",
            "--prompt",
            "ping",
            "--mock",
        ],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("refuse:decision-stale"), "{stderr}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_receipt_shape(&rows[0]);
    assert_eq!(rows[0]["validation"], "stale");
    assert_eq!(rows[0]["result"], "ag_news");
    assert_eq!(rows[0]["outcome"], "refuse:decision-stale");
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}
