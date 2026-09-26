//! Pack-scoped crew session / multi-hop pack memory.
//!
//! create → two hops share context; isolation across sessions;
//! bound/refuse; end/expire miss. Does not invent a live PASS.
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
    let dir = std::env::temp_dir().join(format!("cell-pack-session-{name}-{nanos}"));
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
        .env_remove("CELL_PACK_SESSION")
        .env_remove("CELL_PACK_SESSION_MAX_TURNS")
        .env_remove("CELL_PACK_SESSION_MAX_BYTES")
        .env_remove("CELL_PACK_SESSION_TTL_SECS")
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run_env(args: &[&str], env: &[(&str, &str)]) -> (bool, String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_estate"));
    cmd.args(args)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .env_remove("CELL_LOCAL_LIVE")
        .env_remove("CELL_PACK_SESSION")
        .env_remove("CELL_PACK_SESSION_MAX_TURNS")
        .env_remove("CELL_PACK_SESSION_MAX_BYTES")
        .env_remove("CELL_PACK_SESSION_TTL_SECS");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
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

fn no_invented_pass(stdout: &str, stderr: &str) {
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stderr.contains("live PASS"), "{stderr}");
    assert!(!stdout.contains("LIVE PASS"), "{stdout}");
    assert!(!stderr.contains("LIVE PASS"), "{stderr}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!stderr.contains("READY_FOR_LIVE_TEST: yes"), "{stderr}");
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

fn completion_body(stdout: &str) -> serde_json::Value {
    let start = stdout
        .find("{\n")
        .unwrap_or_else(|| panic!("missing completion JSON\n{stdout}"));
    serde_json::from_str(stdout[start..].trim()).unwrap()
}

fn complete_pack(estate: &str, state: &str, session: &str, prompt: &str) -> (bool, String, String) {
    run(&[
        "complete",
        "--estate",
        estate,
        "--state-dir",
        state,
        "--agent",
        "horizon",
        "--pack",
        "research-crew",
        "--session",
        session,
        "--prompt",
        prompt,
        "--mock",
    ])
}

#[test]
fn create_two_hops_share_context_and_receipt_surfaces_session() {
    assert_locked_cksum();
    let dir = scratch("share");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture().display().to_string();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "pack",
        "session",
        "create",
        "--pack",
        "research-crew",
        "--estate",
        &estate,
        "--state-dir",
        &state_s,
        "--id",
        "sess-crewshare01",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("created id=sess-crewshare01"), "{stdout}");
    no_invented_pass(&stdout, &stderr);

    let (ok, stdout, stderr) = complete_pack(
        &estate,
        &state_s,
        "sess-crewshare01",
        "unique-hop-alpha-token",
    );
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("pack session: id=sess-crewshare01 turns=1 context=none"), "{stdout}");
    assert!(stdout.contains("session=sess-crewshare01 turns=1 context=none"), "{stdout}");
    let body_a = completion_body(&stdout);
    assert_eq!(body_a["completion"], "mock:unique-hop-alpha-token");
    no_invented_pass(&stdout, &stderr);

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["session_id"], "sess-crewshare01");
    assert_eq!(rows[0]["session_turns"], 1);
    assert_eq!(rows[0]["session_context"], false);
    assert_eq!(rows[0]["pack_id"], "research-crew");
    assert_eq!(rows[0]["handoff_from"], "horizon");
    assert_eq!(rows[0]["handoff_to"], "research");
    assert_eq!(rows[0]["outcome"], "allow");

    let (ok, stdout, stderr) = complete_pack(
        &estate,
        &state_s,
        "sess-crewshare01",
        "follow-up that should see prior turn",
    );
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.contains("pack session: id=sess-crewshare01 turns=2 context=applied"),
        "{stdout}"
    );
    assert!(
        stdout.contains("session=sess-crewshare01 turns=2 context=applied"),
        "{stdout}"
    );
    let body_b = completion_body(&stdout);
    let completion = body_b["completion"].as_str().unwrap();
    assert!(
        completion.contains("unique-hop-alpha-token"),
        "hop 2 must see hop 1 prompt in mock context: {completion}"
    );
    assert!(
        completion.contains("follow-up that should see prior turn"),
        "{completion}"
    );
    no_invented_pass(&stdout, &stderr);

    let rows = load_receipts(&state);
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[1]["session_id"], "sess-crewshare01");
    assert_eq!(rows[1]["session_turns"], 2);
    assert_eq!(rows[1]["session_context"], true);

    let (ok, report, stderr) = run(&["decisions", "report", "--state-dir", &state_s]);
    assert!(ok, "{stderr}");
    assert!(report.contains("session=sess-crewshare01"), "{report}");
    assert!(report.contains("context=applied"), "{report}");

    let (ok, shown, stderr) = run(&[
        "pack",
        "session",
        "show",
        "--id",
        "sess-crewshare01",
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "{stderr}");
    assert!(shown.contains("turns=2"), "{shown}");
    assert!(shown.contains("unique-hop-alpha-token"), "{shown}");
    no_invented_pass(&shown, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn new_session_does_not_leak_prior_session() {
    assert_locked_cksum();
    let dir = scratch("isolate");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture().display().to_string();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) =
        complete_pack(&estate, &state_s, "sess-alpha000001", "secret-alpha-marker");
    assert!(ok, "stderr={stderr}\nstdout={stdout}");

    let (ok, stdout, stderr) =
        complete_pack(&estate, &state_s, "sess-beta00000001", "beta-first-hop");
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("context=none"), "{stdout}");
    let body = completion_body(&stdout);
    let completion = body["completion"].as_str().unwrap();
    assert!(
        !completion.contains("secret-alpha-marker"),
        "new session must not see prior session: {completion}"
    );
    assert_eq!(completion, "mock:beta-first-hop");

    let rows = load_receipts(&state);
    assert_eq!(rows[0]["session_id"], "sess-alpha000001");
    assert_eq!(rows[1]["session_id"], "sess-beta00000001");
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bound_refuses_when_max_turns_exceeded() {
    assert_locked_cksum();
    let dir = scratch("bound");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture().display().to_string();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run_env(
        &[
            "complete",
            "--estate",
            &estate,
            "--state-dir",
            &state_s,
            "--agent",
            "horizon",
            "--pack",
            "research-crew",
            "--session",
            "sess-boundturn01",
            "--prompt",
            "first-and-only",
            "--mock",
        ],
        &[("CELL_PACK_SESSION_MAX_TURNS", "1")],
    );
    assert!(ok, "stderr={stderr}\nstdout={stdout}");

    let (ok, stdout, stderr) = run_env(
        &[
            "complete",
            "--estate",
            &estate,
            "--state-dir",
            &state_s,
            "--agent",
            "horizon",
            "--pack",
            "research-crew",
            "--session",
            "sess-boundturn01",
            "--prompt",
            "should-refuse",
            "--mock",
        ],
        &[("CELL_PACK_SESSION_MAX_TURNS", "1")],
    );
    assert!(!ok, "stdout={stdout}");
    assert!(
        stderr.contains("refuse:session-bound"),
        "stderr={stderr}\nstdout={stdout}"
    );
    assert!(!stdout.contains("\"completion\""), "{stdout}");
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn end_and_expire_refuse_resume() {
    assert_locked_cksum();
    let dir = scratch("end-exp");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture().display().to_string();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) =
        complete_pack(&estate, &state_s, "sess-endthis0001", "before-end");
    assert!(ok, "stderr={stderr}\nstdout={stdout}");

    let (ok, stdout, stderr) = run(&[
        "pack",
        "session",
        "end",
        "--id",
        "sess-endthis0001",
        "--state-dir",
        &state_s,
    ]);
    assert!(ok, "{stderr}\n{stdout}");
    assert!(stdout.contains("ended id=sess-endthis0001"), "{stdout}");

    let (ok, stdout, stderr) =
        complete_pack(&estate, &state_s, "sess-endthis0001", "after-end");
    assert!(!ok, "stdout={stdout}");
    assert!(
        stderr.contains("refuse:session-ended"),
        "stderr={stderr}\nstdout={stdout}"
    );
    no_invented_pass(&stdout, &stderr);

    let (ok, stdout, stderr) = run(&[
        "pack",
        "session",
        "create",
        "--pack",
        "research-crew",
        "--estate",
        &estate,
        "--state-dir",
        &state_s,
        "--id",
        "sess-expired0001",
        "--ttl-secs",
        "1",
    ]);
    assert!(ok, "{stderr}\n{stdout}");
    std::thread::sleep(std::time::Duration::from_secs(2));
    let (ok, stdout, stderr) =
        complete_pack(&estate, &state_s, "sess-expired0001", "after-ttl");
    assert!(!ok, "stdout={stdout}");
    assert!(
        stderr.contains("refuse:session-expired"),
        "stderr={stderr}\nstdout={stdout}"
    );
    no_invented_pass(&stdout, &stderr);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn session_without_pack_refuses() {
    assert_locked_cksum();
    let dir = scratch("no-pack");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture().display().to_string();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "complete",
        "--estate",
        &estate,
        "--state-dir",
        &state_s,
        "--agent",
        "horizon",
        "--session",
        "sess-nopack00001",
        "--prompt",
        "ping",
        "--mock",
    ]);
    assert!(!ok, "stdout={stdout}");
    assert!(
        stderr.contains("refuse:session-requires-pack"),
        "stderr={stderr}\nstdout={stdout}"
    );
    no_invented_pass(&stdout, &stderr);
    assert!(!journal(&state).is_file());
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn session_create_flag_mints_and_ready_language_stays_no() {
    assert_locked_cksum();
    let dir = scratch("mint");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture().display().to_string();
    let state_s = state.display().to_string();

    let (ok, stdout, stderr) = run(&[
        "complete",
        "--estate",
        &estate,
        "--state-dir",
        &state_s,
        "--agent",
        "horizon",
        "--pack",
        "research-crew",
        "--session-create",
        "--prompt",
        "minted-session-hop",
        "--mock",
    ]);
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("pack session: created id="), "{stdout}");
    assert!(stdout.contains("context=none"), "{stdout}");
    no_invented_pass(&stdout, &stderr);
    let rows = load_receipts(&state);
    let sid = rows[0]["session_id"].as_str().unwrap();
    assert!(sid.starts_with("sess-"), "{sid}");
    assert_eq!(rows[0]["session_turns"], 1);
    assert_locked_cksum();
    let _ = std::fs::remove_dir_all(&dir);
}
