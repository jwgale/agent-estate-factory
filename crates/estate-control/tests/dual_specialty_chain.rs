//! Dual specialty pack chain: ag_news then rust_idiom then frontier.
//!
//! `estate package run --chain` and `estate package dual-prove` under
//! `--mock`. No network. No Ollama. Not a live PASS.
//! Locked examples/estate.yaml stays untouched.

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
    let dir = std::env::temp_dir().join(format!("cell-dual-specialty-{name}-{nanos}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args(args)
        .current_dir(repo_root())
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .env_remove("CELL_LOCAL_LIVE")
        .env_remove("CELL_LOCAL_MODEL")
        .env_remove("CELL_COMPLETE_LABEL")
        .env_remove("CELL_PACK_SESSION")
        .env_remove("CELL_PACK_SESSION_MAX_TURNS")
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

fn fixture() -> PathBuf {
    repo_root().join("examples/fixtures/agent-pack-handoff.yaml")
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

fn prove_json(stdout: &str) -> serde_json::Value {
    let line = stdout
        .lines()
        .rev()
        .find(|line| line.contains("cell-one.dual-specialty-prove.v0"))
        .expect("prove json");
    serde_json::from_str(line).expect(line)
}

#[test]
fn package_run_chain_journals_both_specialties_then_frontier() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("chain");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "package",
        "run",
        "--id",
        "dual-specialty",
        "--chain",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--mock",
        "--prompt",
        "dual-specialty-hop-alpha-token",
        "--session-create",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("chain chain-dual-specialty-"), "{stdout}");
    assert!(stdout.contains("hops=3"), "{stdout}");
    assert!(stdout.contains("context=none"), "{stdout}");
    assert!(stdout.contains("context=applied"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stderr.contains("enforced"), "{stderr}");

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 3, "{rows:?}");
    let chain_id = rows[0]["chain_id"].as_str().expect("chain_id");
    assert!(chain_id.starts_with("chain-dual-specialty-"), "{chain_id}");
    let want = [
        ("horizon", "research", "ag_news", false),
        ("research", "idiom", "rust_idiom", true),
        ("idiom", "horizon", "frontier_http", true),
    ];
    for (i, (from, to, binding, context)) in want.iter().enumerate() {
        let row = &rows[i];
        assert_eq!(row["schema"], "cell-one.decision-receipt.v0");
        assert_eq!(row["surface"], "complete");
        assert_eq!(row["chain_id"], chain_id);
        assert_eq!(row["pack_id"], "dual-specialty");
        assert_eq!(row["package_id"], "dual-specialty");
        assert_eq!(row["handoff_from"], *from);
        assert_eq!(row["handoff_to"], *to);
        assert_eq!(row["agent"], *to);
        assert_eq!(row["capability"], *binding);
        assert_eq!(row["result"], *binding);
        assert_eq!(row["outcome"], "allow");
        assert_eq!(row["validation"], "ok");
        assert_eq!(row["candidates"], serde_json::json!([{"id": binding}]));
        assert!(row
            .get("rejected")
            .map(|v| v.as_array().map(|a| a.is_empty()).unwrap_or(true))
            .unwrap_or(true));
        assert!(row
            .get("completion_label")
            .map(|v| v.is_null())
            .unwrap_or(true));
        assert_eq!(row["session_context"], *context);
        let hops = row["handoffs"].as_array().expect("handoffs");
        assert_eq!(hops.len(), 3);
        assert_eq!(hops[0]["binding"], "ag_news");
        assert_eq!(hops[1]["binding"], "rust_idiom");
        assert_eq!(hops[2]["binding"], "frontier_http");
    }

    let (ok, report, stderr) = run(&[
        "decisions",
        "report",
        "--state-dir",
        &state_s,
        "--pack",
        "dual-specialty",
    ]);
    assert!(ok, "{stderr}");
    assert!(report.contains("capability=ag_news"), "{report}");
    assert!(report.contains("capability=rust_idiom"), "{report}");
    assert!(report.contains("capability=frontier_http"), "{report}");
    assert!(report.contains("surface=complete"), "{report}");
    assert!(report.contains(&format!("chain={chain_id}")), "{report}");
    assert!(report.contains("context=applied"), "{report}");
    assert_locked_cksum();
}

#[test]
fn dual_prove_prints_ok_report_for_the_mock_chain() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("prove");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "package",
        "dual-prove",
        "--id",
        "dual-specialty",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.contains("package dual-prove: dual-specialty"),
        "{stdout}"
    );
    assert!(stdout.contains("ok: yes"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert!(stdout.contains("live_sync: no"), "{stdout}");
    assert!(stdout.contains("binding=ag_news"), "{stdout}");
    assert!(stdout.contains("binding=rust_idiom"), "{stdout}");
    assert!(stdout.contains("binding=frontier_http"), "{stdout}");
    assert!(stdout.contains("context=applied"), "{stdout}");
    assert!(stdout.contains("capability=ag_news"), "{stdout}");
    assert!(stdout.contains("capability=rust_idiom"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stdout.contains("\"enforced\""), "{stdout}");

    let report = prove_json(&stdout);
    assert_eq!(report["schema"], "cell-one.dual-specialty-prove.v0");
    assert_eq!(report["ok"], true);
    assert_eq!(report["ready_for_live_test"], false);
    assert_eq!(report["live_sync"], false);
    assert_eq!(report["package_id"], "dual-specialty");
    assert_eq!(report["pack_id"], "dual-specialty");
    let hops = report["hops"].as_array().expect("hops");
    assert_eq!(hops.len(), 3);
    assert_eq!(hops[0]["binding"], "ag_news");
    assert_eq!(hops[0]["context"], "none");
    assert_eq!(hops[1]["binding"], "rust_idiom");
    assert_eq!(hops[1]["agent"], "idiom");
    assert_eq!(hops[1]["context"], "applied");
    assert_eq!(hops[2]["binding"], "frontier_http");
    for name in [
        "bindings",
        "disjoint_allow",
        "single_hop_abstain",
        "cross_seat_deny",
        "equal_class_abstain",
        "sacred",
        "chain_receipts",
        "session_context",
        "decisions_report",
        "session_refuses",
    ] {
        assert_eq!(report["checks"][name]["ok"], true, "{name} {report}");
    }

    let rows = load_receipts(&state);
    assert_eq!(
        rows.len(),
        3,
        "refuse paths must not add receipts: {rows:?}"
    );
    assert_eq!(rows[0]["result"], "ag_news");
    assert_eq!(rows[1]["result"], "rust_idiom");
    assert_eq!(rows[2]["result"], "frontier_http");
    assert_locked_cksum();
}

#[test]
fn dual_prove_refuses_a_package_that_is_not_the_dual_chain() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("wrong-pkg");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();
    let (ok, stdout, stderr) = run(&[
        "package",
        "dual-prove",
        "--id",
        "classify-ping",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
    ]);
    assert!(!ok, "stdout={stdout}");
    let combined = format!("{stdout}\n{stderr}");
    assert!(combined.contains("refuse:dual-prove"), "{combined}");
    assert!(stdout.contains("ok: no"), "{stdout}");
    assert!(stdout.contains("bindings: FAIL"), "{stdout}");
    assert!(!journal(&state).exists(), "wrong package must not journal");
    assert_locked_cksum();
}

#[test]
fn idiom_selects_rust_idiom_and_research_cannot_use_it() {
    assert_locked_cksum();
    let estate = fixture();
    let estate_s = estate.display().to_string();
    let dir = scratch("seat");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "complete",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--agent",
        "idiom",
        "--prompt",
        "ping",
        "--mock",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["result"], "rust_idiom");
    assert_eq!(rows[0]["capability"], "rust_idiom");
    assert_eq!(rows[0]["agent"], "idiom");
    assert!(rows[0]
        .get("rejected")
        .map(|v| v.as_array().map(|a| a.is_empty()).unwrap_or(true))
        .unwrap_or(true));

    let (ok, stdout, stderr) = run(&[
        "complete",
        "--estate",
        &estate_s,
        "--state-dir",
        &state_s,
        "--agent",
        "research",
        "--object",
        "rust_idiom",
        "--prompt",
        "ping",
        "--mock",
    ]);
    assert!(!ok, "stdout={stdout}");
    assert!(stderr.contains("authorize denied"), "{stderr}");
    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(
        rows[1]["outcome"].as_str().unwrap().starts_with("refuse:"),
        "{rows:?}"
    );
    assert_locked_cksum();
}

#[test]
fn two_specialties_abstain_with_and_without_equal_class() {
    assert_locked_cksum();
    let raw = std::fs::read_to_string(fixture()).unwrap();
    let two = raw.replace(
        "      - id: ag_news\n        description: Purpose specialty seat for pack handoff\n",
        "      - id: ag_news\n        description: Purpose specialty seat for pack handoff\n      - id: rust_idiom\n        description: second specialty on research\n",
    );
    let two = two.replace(
        "    note: Pack handoff complete targets the specialty seat\n",
        "    note: Pack handoff complete targets the specialty seat\n  - subject_agent: research\n    object: rust_idiom\n    kind: model\n    effect: allow\n    note: probe two specialties\n",
    );
    assert!(two.contains("object: rust_idiom"), "patch missed intention");
    let dir = scratch("abstain");
    let estate = dir.join("estate.yaml");
    std::fs::write(&estate, &two).unwrap();
    let estate_s = estate.display().to_string();

    let state = dir.join("plain");
    std::fs::create_dir_all(&state).unwrap();
    let (ok, stdout, stderr) = run(&[
        "complete",
        "--estate",
        &estate_s,
        "--state-dir",
        &state.display().to_string(),
        "--agent",
        "research",
        "--prompt",
        "ping",
        "--mock",
    ]);
    assert!(!ok, "stdout={stdout}");
    assert!(stderr.contains("refuse:decision-abstain"), "{stderr}");
    let rows = load_receipts(&state);
    assert_eq!(rows[0]["result"], "abstain");

    let mixed = dir.join("equal");
    std::fs::create_dir_all(&mixed).unwrap();
    let (ok, stdout, stderr) = run(&[
        "complete",
        "--estate",
        &estate_s,
        "--state-dir",
        &mixed.display().to_string(),
        "--agent",
        "research",
        "--select",
        "equal-class",
        "--prompt",
        "ping",
        "--mock",
    ]);
    assert!(!ok, "stdout={stdout}\n{stderr}");
    assert!(stderr.contains("refuse:decision-abstain"), "{stderr}");
    let rows = load_receipts(&mixed);
    assert_eq!(rows[0]["result"], "abstain", "{rows:?}");
    assert_locked_cksum();
}

#[test]
fn one_specialty_plus_frontier_still_needs_equal_class() {
    assert_locked_cksum();
    let raw = std::fs::read_to_string(fixture()).unwrap();
    let patched = raw.replace(
        "      - id: ag_news\n        description: Purpose specialty seat for pack handoff\n",
        "      - id: ag_news\n        description: Purpose specialty seat for pack handoff\n      - id: frontier_http\n        description: frontier peer\n",
    );
    let patched = patched.replace(
        "    note: Pack handoff complete targets the specialty seat\n",
        "    note: Pack handoff complete targets the specialty seat\n  - subject_agent: research\n    object: frontier_http\n    kind: model\n    effect: allow\n    note: probe one specialty plus frontier\n",
    );
    let dir = scratch("mixed");
    let estate = dir.join("estate.yaml");
    std::fs::write(&estate, &patched).unwrap();
    let estate_s = estate.display().to_string();

    let plain = dir.join("plain");
    std::fs::create_dir_all(&plain).unwrap();
    let (ok, stdout, stderr) = run(&[
        "complete",
        "--estate",
        &estate_s,
        "--state-dir",
        &plain.display().to_string(),
        "--agent",
        "research",
        "--prompt",
        "ping",
        "--mock",
    ]);
    assert!(!ok, "stdout={stdout}");
    assert!(stderr.contains("refuse:decision-abstain"), "{stderr}");

    let flagged = dir.join("flagged");
    std::fs::create_dir_all(&flagged).unwrap();
    let (ok, stdout, stderr) = run(&[
        "complete",
        "--estate",
        &estate_s,
        "--state-dir",
        &flagged.display().to_string(),
        "--agent",
        "research",
        "--select",
        "equal-class",
        "--prompt",
        "ping",
        "--mock",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    let rows = load_receipts(&flagged);
    assert_eq!(rows[0]["result"], "ag_news", "{rows:?}");
    assert_eq!(rows[0]["capability"], "ag_news");
    let rejected = rows[0]["rejected"].as_array().expect("rejected");
    assert!(
        rejected.iter().any(|row| row["id"] == "frontier_http"),
        "{rejected:?}"
    );
    assert_locked_cksum();
}
