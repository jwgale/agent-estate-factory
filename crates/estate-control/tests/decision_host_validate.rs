//! `estate decisions host-validate-prove` on a lab copy.
//!
//! Dual specialty seats, authorize, and a granted convey call. Does not
//! rewrite `examples/estate.yaml`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let mut token = std::process::id().to_string();
    for needle in ["5090", "4090", "4080", "3090"] {
        token = token.replace(needle, "0000");
    }
    let path = std::env::temp_dir().join(format!("cell-one-decision-host-validate-{name}-{token}"));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn cksum_locked() -> String {
    let out = Command::new("cksum")
        .arg(repo_root().join("examples/estate.yaml"))
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = bin()
        .args(args)
        .current_dir(repo_root())
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

#[test]
fn host_validate_prove_runs_authorize_and_convey_on_a_lab_copy() {
    let before = cksum_locked();
    assert!(
        before.starts_with("43770130 3391"),
        "examples/estate.yaml cksum drifted: {before}"
    );
    let locked_bytes = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
    let out = scratch("prove");
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "host-validate-prove",
        "--root",
        repo_root().to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("lab seat: ag_news class=local model=specialist-agnews-3000"), "{stdout}");
    assert!(
        stdout.contains("lab seat: rust_idiom class=local model=specialist-rustidiom-3000"),
        "{stdout}"
    );
    assert!(stdout.contains("local_slm kept"), "{stdout}");
    assert!(
        stdout.contains("lab intention: research model ag_news effect=allow"),
        "{stdout}"
    );
    assert!(
        stdout.contains("lab intention: idiom model rust_idiom effect=allow"),
        "{stdout}"
    );
    assert!(
        stdout.contains("lab intention: sanctum model ag_news,rust_idiom effect=allow"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "authorize select: agent=research result=ag_news validation=ok surface=authorize granted=yes"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("selector=refuse:decision-abstain"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "authorize fallback: agent=research validation=ineligible fallback=ag_news outcome=refuse:deny-default granted=no"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "authorize fallback: agent=research validation=stale fallback=ag_news outcome=refuse:deny-default granted=no"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("hop lease: specialty-hop granted=yes capability=lane-tool"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "convey select: agent=research result=ag_news validation=ok surface=convey granted=yes"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "convey select: agent=idiom result=rust_idiom validation=ok surface=convey granted=yes"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "convey abstain: agent=sanctum result=abstain validation=ok surface=convey selector=refuse:decision-abstain model_granted=no"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "convey fallback: agent=research validation=stale fallback=ag_news outcome=allow hop_granted=yes model_granted=no"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("decision receipt:"), "{stdout}");
    assert!(
        stdout.contains("result=ag_news") && stdout.contains("validation=ok"),
        "{stdout}"
    );
    assert!(stdout.contains("result=rust_idiom"), "{stdout}");
    assert!(stdout.contains("result=abstain"), "{stdout}");
    assert!(
        stdout.contains("surface authorize=4 convey=4 complete=0\n"),
        "{stdout}"
    );
    assert!(stdout.contains("surface=authorize"), "{stdout}");
    assert!(stdout.contains("surface=convey"), "{stdout}");
    assert!(stdout.contains("validation ok=5 stale=2 ineligible=1 expired=0\n"), "{stdout}");
    assert!(stdout.contains("cell-one.decision-host-validate-prove.v0"), "{stdout}");
    assert!(stdout.contains("\"ok\": true"), "{stdout}");
    assert!(stdout.contains("\"ready_for_live_test\": false"), "{stdout}");
    assert!(stdout.contains("\"live_pass_recorded\": false"), "{stdout}");
    assert!(stdout.contains("43770130 3391"), "{stdout}");
    assert!(stdout.contains("decision-host-validate-prove: ok"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!stdout.contains("live PASS recorded"), "{stdout}");
    assert!(!stderr.contains("live PASS"), "{stderr}");
    assert!(
        !stdout.split_whitespace().any(|word| word == "enforced"),
        "{stdout}"
    );
    let tail = stdout.trim_end();
    assert!(
        tail.ends_with("decision-host-validate-prove: ok\nREADY_FOR_LIVE_TEST: no"),
        "stdout tail:\n{}",
        tail.lines().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("host-validate-prove.json")).unwrap())
            .unwrap();
    assert_eq!(report["schema"], "cell-one.decision-host-validate-prove.v0");
    assert_eq!(report["ok"], true);
    assert_eq!(report["ready_for_live_test"], false);
    assert_eq!(report["live_pass_recorded"], false);
    assert_eq!(report["counts"]["surface_authorize"], 4);
    assert_eq!(report["counts"]["surface_convey"], 4);
    assert_eq!(report["authorize"]["select_ok"]["result"], "ag_news");
    assert_eq!(report["authorize"]["select_ok"]["validation"], "ok");
    assert_eq!(report["authorize"]["abstain"]["selector"], "refuse:decision-abstain");
    assert_eq!(report["authorize"]["ineligible"]["granted"], false);
    assert_eq!(report["authorize"]["stale"]["granted"], false);
    assert_eq!(report["convey"]["select_ok"][1]["result"], "rust_idiom");
    assert_eq!(report["convey"]["abstain"]["model_granted"], false);
    assert_eq!(report["convey"]["stale"]["model_granted"], false);
    assert_eq!(report["convey"]["stale"]["hop_granted"], true);
    assert_eq!(report["convey"]["stale"]["validation"], "stale");

    let journal = fs::read_to_string(out.join("state/decisions/receipts.jsonl")).unwrap();
    let rows: Vec<serde_json::Value> = journal
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 8);
    assert_eq!(rows[0]["schema"], "cell-one.decision-receipt.v0");
    assert_eq!(rows[0]["surface"], "authorize");
    assert_eq!(rows[0]["result"], "ag_news");
    assert_eq!(rows[0]["validation"], "ok");
    assert_eq!(rows[0]["outcome"], "allow");
    assert!(rows[1].get("surface").is_some());
    assert_eq!(rows[1]["result"], "abstain");
    assert_eq!(rows[1]["agent"], "sanctum");
    let abstain_ids: Vec<&str> = rows[1]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect();
    assert!(abstain_ids.contains(&"ag_news") && abstain_ids.contains(&"rust_idiom"));
    assert_eq!(rows[2]["validation"], "ineligible");
    assert_eq!(rows[2]["fallback"], "ag_news");
    assert_eq!(rows[2]["outcome"], "refuse:deny-default");
    assert_eq!(rows[3]["validation"], "stale");
    assert_eq!(rows[3]["outcome"], "refuse:deny-default");
    assert!(rows[4].get("surface").is_none(), "{:?}", rows[4]);
    assert_eq!(rows[4]["result"], "ag_news");
    assert_eq!(rows[4]["validation"], "ok");
    assert_eq!(rows[4]["hop_id"], "specialty-hop");
    assert_eq!(rows[5]["result"], "rust_idiom");
    assert_eq!(rows[5]["validation"], "ok");
    assert!(rows[5].get("surface").is_none());
    assert_eq!(rows[6]["result"], "abstain");
    assert_eq!(rows[6]["agent"], "sanctum");
    assert_eq!(rows[7]["validation"], "stale");
    assert_eq!(rows[7]["fallback"], "ag_news");
    assert_eq!(rows[7]["outcome"], "allow");
    assert_ne!(rows[7]["validation"], "ok");

    let replay = fs::read(out.join("decisions-replay.jsonl")).unwrap();
    assert_eq!(replay, fs::read(out.join("state/decisions/receipts.jsonl")).unwrap());
    assert!(out.join("fixtures/ag_news/specialist.Q4_K_M.gguf").is_file());
    assert!(out.join("fixtures/rust_idiom/specialist.Q4_K_M.gguf").is_file());
    assert_eq!(
        fs::read(out.join("fixtures/ag_news/specialist.Q4_K_M.gguf")).unwrap(),
        b"GGUF"
    );
    let lab = fs::read_to_string(out.join("lab-estate.yaml")).unwrap();
    assert!(lab.contains("id: ag_news"), "{lab}");
    assert!(lab.contains("id: rust_idiom"), "{lab}");
    assert!(lab.contains("id: local_slm"), "{lab}");
    assert_ne!(fs::read(out.join("lab-estate.yaml")).unwrap(), locked_bytes);
    assert_eq!(fs::read(repo_root().join("examples/estate.yaml")).unwrap(), locked_bytes);
    let after = cksum_locked();
    assert_eq!(after, before, "examples/estate.yaml cksum changed");
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn host_validate_prove_refuses_the_locked_estate() {
    let before = cksum_locked();
    assert!(before.starts_with("43770130 3391"), "{before}");
    let locked = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&locked).unwrap();
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "host-validate-prove",
        "--root",
        repo_root().to_str().unwrap(),
        "--out",
        locked.to_str().unwrap(),
    ]);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains("refuse:out: host-validate-prove does not write examples/estate.yaml"),
        "{stderr}"
    );
    assert!(!stdout.contains("decision-host-validate-prove: ok"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert_eq!(fs::read(&locked).unwrap(), bytes);
    assert_eq!(cksum_locked(), before);
}
