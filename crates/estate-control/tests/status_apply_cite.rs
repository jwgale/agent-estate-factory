//! `estate status` cites a local `cell-one.improvement-apply.v0` receipt.
//!
//! Same discovery as decisions / pack session / digest / runner:
//! `decisions/improvement-apply.json`, then `{state-dir}/improvement-apply.json`.
//! A sibling `{state-dir}/../apply/` is not cited. Missing is silence.
//! Incomplete or wrong-typed lock fields are `refuse:cite`. Standing /
//! joinable come from the receipt as stored.

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
    let dir = std::env::temp_dir().join(format!("cell-status-apply-cite-{name}-{nanos}"));
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

fn write_apply_receipt(path: &Path, binding: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(
        path,
        format!(
            r#"{{
  "schema": "cell-one.improvement-apply.v0",
  "proposal_id": "specialty-seat:{binding}",
  "proposal_kind": "specialty-seat",
  "binding_id": "{binding}",
  "joinable": true,
  "standing": "joinable: yes",
  "require_plan": true,
  "refuse_without_plan": "refuse:plan: apply-package requires --require-plan",
  "auto_train": false,
  "train_invoked": false
}}
"#
        ),
    )
    .unwrap();
}

fn status(estate: &Path, state: &Path, scratch_root: &Path) -> (bool, String, String) {
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    let plans = scratch_root.join("plans");
    let packs = scratch_root.join("packs");
    let roots = scratch_root.join("roots");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::create_dir_all(&packs).unwrap();
    std::fs::create_dir_all(&roots).unwrap();
    let plans_s = plans.display().to_string();
    let packs_s = packs.display().to_string();
    let roots_s = roots.display().to_string();
    let policy = repo_root()
        .join("policy/cell-one.policy.v0.yaml")
        .display()
        .to_string();
    let root_s = repo_root().display().to_string();
    run(&[
        "status",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--roots-base",
        &roots_s,
        "--plans-dir",
        &plans_s,
        "--packs-dir",
        &packs_s,
        "--policy",
        &policy,
        "--root",
        &root_s,
    ])
}

fn no_invented_pass(stdout: &str, stderr: &str) {
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stderr.contains("live PASS"), "{stderr}");
    assert!(!stdout.contains("LIVE PASS"), "{stdout}");
    assert!(!stderr.contains("LIVE PASS"), "{stderr}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!stderr.contains("READY_FOR_LIVE_TEST: yes"), "{stderr}");
    assert!(!stdout.contains("\"auto_train\": true"), "{stdout}");
    assert!(!stderr.contains("\"auto_train\": true"), "{stderr}");
}

fn assert_green_apply_cite(shown: &str, binding: &str) {
    assert!(
        shown.contains("apply receipt: schema=cell-one.improvement-apply.v0"),
        "{shown}"
    );
    assert!(
        shown.contains(&format!("applied proposal specialty-seat:{binding}")),
        "{shown}"
    );
    assert!(shown.contains("kind=specialty-seat"), "{shown}");
    assert!(shown.contains(&format!("binding={binding}")), "{shown}");
    assert!(shown.contains("standing=joinable: yes"), "{shown}");
    assert!(shown.contains("joinable=true"), "{shown}");
    assert!(shown.contains("require_plan=true"), "{shown}");
    assert!(
        shown.contains("refuse_without_plan=refuse:plan: apply-package requires --require-plan"),
        "{shown}"
    );
    assert!(shown.contains("auto_train=false"), "{shown}");
    assert!(shown.contains("train_invoked=false"), "{shown}");
    assert!(!shown.contains("refuse:cite"), "{shown}");
}

fn assert_no_green_locked_cite(shown: &str) {
    assert!(
        !shown.contains("require_plan=true"),
        "must not invent require_plan=true: {shown}"
    );
    assert!(
        !shown.contains("auto_train=false"),
        "must not invent auto_train=false: {shown}"
    );
    assert!(
        !shown.contains("train_invoked=false"),
        "must not invent train_invoked=false: {shown}"
    );
    assert!(
        !shown.contains("standing=joinable: yes"),
        "must not invent standing: {shown}"
    );
    assert!(
        !shown.contains("apply receipt: schema=cell-one.improvement-apply.v0 path="),
        "must not print a green locked cite: {shown}"
    );
}

fn locked_estate() -> PathBuf {
    repo_root().join("examples/estate.yaml")
}

#[test]
fn status_cites_nearby_apply_receipt() {
    assert_locked_cksum();
    let dir = scratch("apply-cite");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    write_apply_receipt(
        &state.join("decisions").join("improvement-apply.json"),
        "ag_news",
    );
    let (ok, shown, stderr) = status(&locked_estate(), &state, &dir);
    assert!(ok, "{stderr}\n{shown}");
    assert!(shown.starts_with("Cell One status\n"), "{shown}");
    assert!(shown.contains("cloud-agent: declared, not spawned"), "{shown}");
    assert!(shown.contains("Authority\n---------"), "{shown}");
    assert_green_apply_cite(&shown, "ag_news");
    let cite_at = shown
        .find("apply receipt: schema=cell-one.improvement-apply.v0")
        .expect("green cite");
    let authority_at = shown.find("Authority\n---------").expect("authority");
    assert!(
        authority_at < cite_at,
        "apply cite must follow the honesty stack: {shown}"
    );
    no_invented_pass(&shown, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn status_prefers_local_receipt_over_sibling_apply() {
    assert_locked_cksum();
    let root = scratch("apply-local-wins");
    let state = root.join("state");
    std::fs::create_dir_all(state.join("decisions")).unwrap();
    write_apply_receipt(
        &state.join("decisions").join("improvement-apply.json"),
        "ag_news",
    );
    write_apply_receipt(
        &root.join("apply").join("improvement-apply.json"),
        "rust_idiom",
    );
    let (ok, shown, stderr) = status(&locked_estate(), &state, &root);
    assert!(ok, "{stderr}\n{shown}");
    assert_green_apply_cite(&shown, "ag_news");
    assert!(shown.contains("specialty-seat:ag_news"), "{shown}");
    assert!(!shown.contains("rust_idiom"), "{shown}");
    assert!(!shown.contains("/apply/improvement-apply.json"), "{shown}");
    no_invented_pass(&shown, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn status_does_not_cite_sibling_apply() {
    assert_locked_cksum();
    let root = scratch("apply-no-sibling");
    let state = root.join("other").join("state");
    std::fs::create_dir_all(&state).unwrap();
    write_apply_receipt(
        &root.join("apply").join("improvement-apply.json"),
        "ag_news",
    );
    let (ok, shown, stderr) = status(&locked_estate(), &state, &root);
    assert!(ok, "{stderr}\n{shown}");
    assert!(shown.starts_with("Cell One status\n"), "{shown}");
    assert!(!shown.contains("apply receipt:"), "{shown}");
    assert!(!shown.contains("refuse:cite"), "{shown}");
    assert!(!shown.contains("/apply/improvement-apply.json"), "{shown}");
    assert_no_green_locked_cite(&shown);
    no_invented_pass(&shown, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn status_silent_without_local_receipt() {
    assert_locked_cksum();
    let dir = scratch("apply-none");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let (ok, shown, stderr) = status(&locked_estate(), &state, &dir);
    assert!(ok, "{stderr}\n{shown}");
    assert!(shown.starts_with("Cell One status\n"), "{shown}");
    assert!(shown.contains("cloud-agent: declared, not spawned"), "{shown}");
    assert!(!shown.contains("apply receipt:"), "{shown}");
    assert!(!shown.contains("refuse:cite"), "{shown}");
    assert_no_green_locked_cite(&shown);
    no_invented_pass(&shown, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn status_refuses_missing_lock_fields() {
    assert_locked_cksum();
    let dir = scratch("apply-incomplete");
    let state = dir.join("state");
    std::fs::create_dir_all(state.join("decisions")).unwrap();
    std::fs::write(
        state.join("decisions").join("improvement-apply.json"),
        r#"{
  "schema": "cell-one.improvement-apply.v0",
  "proposal_id": "specialty-seat:ag_news",
  "proposal_kind": "specialty-seat",
  "binding_id": "ag_news"
}
"#,
    )
    .unwrap();
    let (ok, shown, stderr) = status(&locked_estate(), &state, &dir);
    assert!(ok, "{stderr}\n{shown}");
    assert!(shown.contains("refuse:cite:"), "{shown}");
    assert_no_green_locked_cite(&shown);
    no_invented_pass(&shown, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn status_refuses_wrong_typed_lock_fields() {
    assert_locked_cksum();
    let dir = scratch("apply-wrong-type");
    let state = dir.join("state");
    std::fs::create_dir_all(state.join("decisions")).unwrap();
    std::fs::write(
        state.join("decisions").join("improvement-apply.json"),
        r#"{
  "schema": "cell-one.improvement-apply.v0",
  "proposal_id": "specialty-seat:ag_news",
  "proposal_kind": "specialty-seat",
  "binding_id": "ag_news",
  "joinable": true,
  "standing": "joinable: yes",
  "require_plan": "true",
  "refuse_without_plan": "refuse:plan: apply-package requires --require-plan",
  "auto_train": false,
  "train_invoked": false
}
"#,
    )
    .unwrap();
    let (ok, shown, stderr) = status(&locked_estate(), &state, &dir);
    assert!(ok, "{stderr}\n{shown}");
    assert!(shown.contains("refuse:cite:"), "{shown}");
    assert!(shown.contains("require_plan"), "{shown}");
    assert!(!shown.contains("require_plan=true"), "{shown}");
    assert_no_green_locked_cite(&shown);
    no_invented_pass(&shown, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn status_mirrors_receipt_lock_fields() {
    assert_locked_cksum();
    let dir = scratch("apply-mirror");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(
        state.join("improvement-apply.json"),
        r#"{
  "schema": "cell-one.improvement-apply.v0",
  "proposal_id": "specialty-seat:rust_idiom",
  "proposal_kind": "specialty-seat",
  "binding_id": "rust_idiom",
  "joinable": false,
  "standing": "joinable: no",
  "require_plan": false,
  "refuse_without_plan": "refuse:plan: test-mirror",
  "auto_train": true,
  "train_invoked": true
}
"#,
    )
    .unwrap();
    let (ok, shown, stderr) = status(&locked_estate(), &state, &dir);
    assert!(ok, "{stderr}\n{shown}");
    assert!(shown.starts_with("Cell One status\n"), "{shown}");
    assert!(
        shown.contains("apply receipt: schema=cell-one.improvement-apply.v0"),
        "{shown}"
    );
    assert!(shown.contains("applied proposal specialty-seat:rust_idiom"), "{shown}");
    assert!(shown.contains("kind=specialty-seat"), "{shown}");
    assert!(shown.contains("binding=rust_idiom"), "{shown}");
    assert!(shown.contains("standing=joinable: no"), "{shown}");
    assert!(shown.contains("joinable=false"), "{shown}");
    assert!(shown.contains("require_plan=false"), "{shown}");
    assert!(shown.contains("refuse_without_plan=refuse:plan: test-mirror"), "{shown}");
    assert!(shown.contains("auto_train=true"), "{shown}");
    assert!(shown.contains("train_invoked=true"), "{shown}");
    assert!(!shown.contains("require_plan=true"), "{shown}");
    assert!(!shown.contains("auto_train=false"), "{shown}");
    assert!(!shown.contains("train_invoked=false"), "{shown}");
    assert!(!shown.contains("standing=joinable: yes"), "{shown}");
    assert!(!shown.contains("refuse:cite"), "{shown}");
    assert!(!shown.contains("READY_FOR_LIVE_TEST: yes"), "{shown}");
    assert!(!shown.contains("live PASS"), "{shown}");
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_status_names_the_apply_receipt_cite() {
    assert_locked_cksum();
    let (ok, shown, stderr) = run(&["help", "status"]);
    assert!(ok, "{stderr}\n{shown}");
    assert!(shown.contains("cell-one.improvement-apply.v0"), "{shown}");
    assert!(shown.contains("cite_or_refuse_nearby_apply_receipt"), "{shown}");
    assert!(shown.contains("refuse:cite"), "{shown}");
    assert!(shown.contains("decisions/improvement-apply.json"), "{shown}");
    assert!(shown.contains("../apply/"), "{shown}");
    assert!(shown.contains("Status does not claim the applied"), "{shown}");
    assert!(
        shown.contains("seat is joinable from the receipt alone."),
        "{shown}"
    );
    assert!(!shown.contains("READY_FOR_LIVE_TEST: yes"), "{shown}");
    assert!(!shown.contains("live PASS"), "{shown}");

    let (ok, clap, stderr) = run(&["status", "--help"]);
    assert!(ok, "{stderr}\n{clap}");
    assert!(clap.contains("cell-one.improvement-apply.v0"), "{clap}");
    assert!(clap.contains("refuse:cite"), "{clap}");
    assert!(clap.contains("decisions/improvement-apply.json"), "{clap}");
    assert!(!clap.contains("READY_FOR_LIVE_TEST: yes"), "{clap}");
    assert_locked_cksum();
}
