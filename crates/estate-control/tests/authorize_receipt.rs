//! Decision receipts on `estate authorize`.
//!
//! The host prepares eligible model-binding ids, a selector chooses or
//! abstains, and the host re-validates. Authorize still decides allow or
//! deny. A fallback is recorded and is not applied.

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
    let dir = std::env::temp_dir().join(format!("cell-authorize-receipt-{name}-{nanos}"));
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

fn authorize(estate: &Path, state: &Path, extra: &[&str]) -> (bool, String, String) {
    let mut args = vec![
        "authorize".to_string(),
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

fn decision_body(stdout: &str) -> serde_json::Value {
    let start = stdout
        .find("{\n")
        .unwrap_or_else(|| panic!("missing decision JSON\n{stdout}"));
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
    assert_eq!(row["surface"], "authorize");
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
    estate.intentions.push(estate_schema::Intention {
        subject_agent: "research".into(),
        object: "notes-append".into(),
        kind: estate_schema::IntentionKind::Tool,
        effect: estate_schema::Effect::Allow,
        note: None,
    });
    write_estate(root, &estate)
}

#[test]
fn one_eligible_specialty_seat_selects_and_report_counts_it() {
    let dir = scratch("specialty");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = specialty_estate(&dir);
    let (ok, stdout, stderr) = authorize(
        &estate,
        &state,
        &[
            "--agent", "research", "--kind", "model", "--object", "ag_news",
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
    let body = decision_body(&stdout);
    assert_eq!(body["decision"], "allow");
    let reason = body["reason"].as_str().unwrap();
    assert!(reason.contains("allow intention"), "{reason}");
    assert!(!reason.contains("fallback"), "{reason}");
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
    assert_eq!(
        std::fs::read(&out).unwrap(),
        std::fs::read(journal(&state)).unwrap()
    );
    let (ok, report, report_err) = run(&["decisions", "report", "--state-dir", &state_s]);
    assert!(ok, "{report}\n{report_err}");
    assert!(report.contains("decision receipts: 1\n"), "{report}");
    assert!(
        report.contains("stage prepare=1 select=0 validate=1 fallback=0\n"),
        "{report}"
    );
    assert!(
        report.contains("validation ok=1 stale=0 ineligible=0 expired=0\n"),
        "{report}"
    );
    assert!(report.contains("fallback none=1\n"), "{report}");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn frontier_and_local_both_eligible_abstain() {
    let dir = scratch("equal");
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
    let (ok, stdout, stderr) = authorize(
        &estate_path,
        &state,
        &[
            "--agent", "research", "--kind", "model", "--object", "xai_grok",
        ],
    );
    assert!(ok, "{stdout}\n{stderr}");
    let cite = cite_line(&stdout);
    assert!(cite.contains("result=abstain"), "{cite}");
    assert!(cite.contains("validation=ok"), "{cite}");
    assert!(!cite.contains("fallback="), "{cite}");
    let body = decision_body(&stdout);
    assert_eq!(body["decision"], "allow");
    let reason = body["reason"].as_str().unwrap();
    assert!(reason.contains("allow intention"), "{reason}");
    assert!(reason.contains("xai_grok"), "{reason}");
    assert!(!reason.contains("local_slm"), "{reason}");
    assert!(!reason.contains("abstain"), "{reason}");
    assert!(!reason.contains("fallback"), "{reason}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_receipt_shape(&rows[0]);
    assert_eq!(
        rows[0]["candidates"],
        serde_json::json!([{"id": "xai_grok"}, {"id": "local_slm"}])
    );
    assert_eq!(rows[0]["result"], "abstain");
    assert_eq!(rows[0]["stage"], "select");
    assert_eq!(rows[0]["validation"], "ok");
    assert!(rows[0]["fallback"].is_null());
    assert_eq!(rows[0]["outcome"], "allow");
    assert_ne!(rows[0]["result"], "local_slm");
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_hint_refuses_before_the_receipt_and_stale_hint_does_not_grant() {
    let dir = scratch("hint");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = repo_root().join("examples/estate.yaml");
    let feed = dir.join("feed");
    std::fs::create_dir_all(&feed).unwrap();
    std::fs::write(state.join("decision-select.json"), "{").unwrap();
    let feed_s = feed.display().to_string();
    let (ok, stdout, stderr) = authorize(
        &estate,
        &state,
        &[
            "--agent",
            "horizon",
            "--kind",
            "memory_read",
            "--object",
            "lane:horizon",
            "--feed-dir",
            &feed_s,
        ],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("refuse:decision-select"), "{stderr}");
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    assert!(!stdout.contains("\"decision\""), "{stdout}");
    assert!(!journal(&state).exists());
    assert!(!feed.join("events.jsonl").exists());
    no_invented_pass(&stdout, &stderr);

    let estate = specialty_estate(&dir);
    write_hint(&state, "xai_grok", None);
    let (ok, stdout, stderr) = authorize(
        &estate,
        &state,
        &[
            "--agent",
            "research",
            "--kind",
            "tool",
            "--object",
            "notes-append",
        ],
    );
    assert!(ok, "{stdout}\n{stderr}");
    let cite = cite_line(&stdout);
    assert!(cite.contains("result=xai_grok"), "{cite}");
    assert!(cite.contains("validation=ineligible"), "{cite}");
    assert!(cite.contains("fallback=ag_news"), "{cite}");
    let body = decision_body(&stdout);
    assert_eq!(body["decision"], "allow");
    let reason = body["reason"].as_str().unwrap();
    assert!(reason.contains("notes-append"), "{reason}");
    assert!(!reason.contains("xai_grok"), "{reason}");
    assert!(!reason.contains("ag_news"), "{reason}");
    assert!(!reason.contains("fallback"), "{reason}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["validation"], "ineligible");
    assert_eq!(rows[0]["result"], "xai_grok");
    assert_eq!(rows[0]["fallback"], "ag_news");
    assert_eq!(rows[0]["stage"], "fallback");
    assert_eq!(rows[0]["outcome"], "allow");
    assert_eq!(rows[0]["hop_id"], "tool");
    assert_eq!(rows[0]["capability"], "notes-append");

    write_hint(&state, "ag_news", Some("0"));
    let (ok, stdout, stderr) = authorize(
        &estate,
        &state,
        &[
            "--agent",
            "research",
            "--kind",
            "tool",
            "--object",
            "notes-append",
        ],
    );
    assert!(ok, "{stdout}\n{stderr}");
    let cite = cite_line(&stdout);
    assert!(cite.contains("result=ag_news"), "{cite}");
    assert!(cite.contains("validation=stale"), "{cite}");
    assert!(cite.contains("fallback=ag_news"), "{cite}");
    let body = decision_body(&stdout);
    assert_eq!(body["decision"], "allow");
    let reason = body["reason"].as_str().unwrap();
    assert!(reason.contains("allow intention"), "{reason}");
    assert!(reason.contains("notes-append"), "{reason}");
    assert!(!reason.contains("ag_news"), "{reason}");
    assert!(!reason.contains("fallback"), "{reason}");
    assert!(!reason.contains("stale"), "{reason}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["seq"], 2);
    assert_eq!(rows[1]["validation"], "stale");
    assert_eq!(rows[1]["result"], "ag_news");
    assert_eq!(rows[1]["fallback"], "ag_news");
    assert_eq!(rows[1]["stage"], "fallback");
    assert_eq!(rows[1]["outcome"], "allow");
    assert_receipt_shape(&rows[1]);
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stale_hint_does_not_turn_a_deny_into_an_allow() {
    let dir = scratch("stale-deny");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let mut estate = base_estate();
    push_local(&mut estate, "ag_news");
    estate
        .agents
        .iter_mut()
        .find(|agent| agent.id == "research")
        .unwrap()
        .models = vec![estate_schema::ModelUseDecl {
        id: "ag_news".into(),
        description: None,
    }];
    allow_model(&mut estate, "research", "ag_news");
    let estate_path = write_estate(&dir, &estate);
    write_hint(&state, "ag_news", Some("0"));
    let (ok, stdout, stderr) = authorize(
        &estate_path,
        &state,
        &[
            "--agent",
            "research",
            "--kind",
            "tool",
            "--object",
            "notes-append",
        ],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("authorize denied"), "{stderr}");
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    let body = decision_body(&stdout);
    assert_eq!(body["decision"], "deny");
    let reason = body["reason"].as_str().unwrap();
    assert!(reason.contains("deny-default"), "{reason}");
    assert!(!reason.contains("ag_news"), "{reason}");
    assert!(!reason.contains("fallback"), "{reason}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_receipt_shape(&rows[0]);
    assert_eq!(rows[0]["outcome"], "refuse:deny-default");
    assert_eq!(rows[0]["validation"], "stale");
    assert_eq!(rows[0]["result"], "ag_news");
    assert_eq!(rows[0]["fallback"], "ag_news");
    assert_eq!(rows[0]["stage"], "fallback");
    assert_ne!(body["decision"], "allow");
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
    let (ok, stdout, stderr) = authorize(
        &estate,
        &state,
        &[
            "--agent",
            "research",
            "--kind",
            "tool",
            "--object",
            "notes-append",
        ],
    );
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(stderr.contains("authorize denied"), "{stderr}");
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    assert!(!stdout.contains("Agents\n"), "{stdout}");
    let body = decision_body(&stdout);
    assert_eq!(body["decision"], "deny");
    assert!(
        body["reason"].as_str().unwrap().contains("deny-default"),
        "{body}"
    );
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1);
    assert_receipt_shape(&rows[0]);
    assert_eq!(rows[0]["outcome"], "refuse:deny-default");
    assert_eq!(rows[0]["result"], "abstain");
    assert_eq!(rows[0]["validation"], "ok");
    assert_eq!(rows[0]["stage"], "select");
    assert_eq!(rows[0]["candidates"], serde_json::json!([]));
    assert!(rows[0]["fallback"].is_null());
    assert_eq!(rows[0]["hop_id"], "tool");
    assert_eq!(rows[0]["capability"], "notes-append");
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn journal_write_failure_after_authorize_still_prints_the_allow() {
    let dir = scratch("journal-fail");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("decisions"), "not-a-directory\n").unwrap();
    let estate = repo_root().join("examples/estate.yaml");
    let (ok, stdout, stderr) = authorize(
        &estate,
        &state,
        &[
            "--agent",
            "horizon",
            "--kind",
            "memory_read",
            "--object",
            "lane:horizon",
        ],
    );
    assert!(ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains("decision receipt: journal write failed after authorize commit:"),
        "{stderr}"
    );
    assert!(!stderr.contains("refuse:"), "{stderr}");
    assert!(!stdout.contains("decision receipt:"), "{stdout}");
    let body = decision_body(&stdout);
    assert_eq!(body["decision"], "allow");
    assert!(
        body["reason"].as_str().unwrap().contains("owns lane"),
        "{body}"
    );
    assert_eq!(
        std::fs::read_to_string(state.join("decisions")).unwrap(),
        "not-a-directory\n"
    );
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}
