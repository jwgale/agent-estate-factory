//! `estate control-plane-prove` on one throwaway lab.
//! Fuel, decide, and run share that lab. Locked `examples/estate.yaml` stays put.

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
    let path = std::env::temp_dir().join(format!("cell-one-control-plane-{name}-{token}"));
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

#[test]
fn control_plane_prove_stitches_fuel_decide_and_run_on_one_lab() {
    let before = cksum_locked();
    assert!(
        before.starts_with("43770130 3391"),
        "examples/estate.yaml cksum drifted: {before}"
    );
    let locked_bytes = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
    let out = scratch("prove");
    let run = bin()
        .args([
            "control-plane-prove",
            "--root",
            repo_root().to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ])
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_LOCAL_LIVE")
        .env_remove("XAI_API_KEY")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run.stderr).to_string();
    let text = format!("{stdout}{stderr}");
    assert!(run.status.success(), "{text}");
    assert!(stdout.contains("control-plane-prove: fuel"), "{stdout}");
    assert!(stdout.contains("control-plane-prove: decide"), "{stdout}");
    assert!(stdout.contains("control-plane-prove: run"), "{stdout}");
    assert!(stdout.contains("joinable: yes"), "{stdout}");
    assert!(stdout.contains("binding: ag_news"), "{stdout}");
    assert!(stdout.contains("binding: rust_idiom"), "{stdout}");
    assert!(
        stdout.contains("surface authorize=4 convey=4 complete=5"),
        "{stdout}"
    );
    assert!(stdout.contains("refuse:decision-abstain"), "{stdout}");
    assert!(stdout.contains("runner hop: research capability=ag_news context=none"), "{stdout}");
    assert!(
        stdout.contains("runner hop: idiom capability=rust_idiom context=applied"),
        "{stdout}"
    );
    assert!(
        stdout.contains("runner hop: horizon capability=frontier_http context=applied"),
        "{stdout}"
    );
    assert!(stdout.contains("package=dual-specialty"), "{stdout}");
    assert!(stdout.contains("chain=chain-dual-specialty-"), "{stdout}");
    assert!(stdout.contains("refuse:runner-already-running"), "{stdout}");
    assert!(stdout.contains("cell-one.control-plane-prove.v0"), "{stdout}");
    let tail = stdout.trim_end();
    assert!(
        tail.ends_with("control-plane-prove: ok\nREADY_FOR_LIVE_TEST: no"),
        "{stdout}"
    );
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!text.contains("live PASS recorded"), "{text}");

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("control-plane-prove.json")).unwrap())
            .unwrap();
    assert_eq!(report["schema"], "cell-one.control-plane-prove.v0");
    assert_eq!(report["ok"], true);
    assert_eq!(report["ready_for_live_test"], false);
    assert_eq!(report["live_pass_recorded"], false);
    assert_eq!(report["fuel"]["trained_shape"], "gguf");
    assert_eq!(report["fuel"]["auto_apply"], false);
    assert_eq!(report["fuel"]["joinable"]["ag_news"], true);
    assert_eq!(report["fuel"]["joinable"]["rust_idiom"], true);
    assert_eq!(report["fuel"]["seat_models"]["ag_news"], "specialist-agnews-3000");
    assert_eq!(
        report["fuel"]["seat_models"]["rust_idiom"],
        "specialist-rustidiom-3000"
    );
    assert_eq!(report["fuel"]["beside"], "local_slm");
    assert_eq!(report["decide"]["surface_authorize"], 4);
    assert_eq!(report["decide"]["surface_convey"], 4);
    assert_eq!(report["decide"]["surface_complete"], 5);
    assert_eq!(report["decide"]["abstain"], "refuse:decision-abstain");
    assert_eq!(report["decide"]["stale_fallback"], "ag_news");
    assert_eq!(report["decide"]["ineligible_fallback"], "ag_news");
    let caps = report["decide"]["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(caps.contains(&"ag_news".to_string()), "{caps:?}");
    assert!(caps.contains(&"rust_idiom".to_string()), "{caps:?}");
    assert!(report["journal"]["complete"].as_u64().unwrap() > 0);
    assert_eq!(report["run"]["routine_id"], "standing-dual");
    assert_eq!(report["run"]["package"], "dual-specialty");
    assert_eq!(report["run"]["session_stitch"], true);
    assert!(
        report["run"]["chain_id"]
            .as_str()
            .unwrap()
            .starts_with("chain-dual-specialty-"),
        "{}",
        report["run"]["chain_id"]
    );
    assert!(!report["run"]["session_id"].as_str().unwrap().is_empty());
    assert_eq!(report["run"]["digest_cites_session"], true);
    assert_eq!(report["run"]["digest_cites_package"], true);
    assert_eq!(report["run"]["digest_cites_chain"], true);
    assert_eq!(
        report["run"]["double_start_refuse"],
        "refuse:runner-already-running"
    );
    let hops = report["run"]["hops"].as_array().unwrap();
    assert_eq!(hops.len(), 3);
    assert_eq!(hops[0]["capability"], "ag_news");
    assert_eq!(hops[0]["context"], "none");
    assert_eq!(hops[1]["capability"], "rust_idiom");
    assert_eq!(hops[1]["context"], "applied");
    assert_eq!(hops[2]["capability"], "frontier_http");
    assert_eq!(hops[2]["context"], "applied");
    assert_eq!(hops[1]["session_id"], hops[0]["session_id"]);
    assert_eq!(hops[2]["session_id"], hops[0]["session_id"]);
    let cksum = report["estate_cksum"].as_str().unwrap();
    assert!(cksum.starts_with("43770130 3391"), "{cksum}");
    let rendered = serde_json::to_string(&report).unwrap();
    assert!(
        !rendered.split_whitespace().any(|word| word == "enforced"),
        "{rendered}"
    );

    let lab = fs::read_to_string(out.join("lab-estate.yaml")).unwrap();
    assert!(lab.contains("id: ag_news"), "{lab}");
    assert!(lab.contains("id: rust_idiom"), "{lab}");
    assert!(lab.contains("id: local_slm"), "{lab}");
    assert!(lab.contains("specialist-agnews-3000"), "{lab}");
    assert!(lab.contains("specialist-rustidiom-3000"), "{lab}");
    assert!(lab.contains("standing-dual"), "{lab}");
    assert_ne!(fs::read(out.join("lab-estate.yaml")).unwrap(), locked_bytes);
    assert_eq!(
        fs::read(repo_root().join("examples/estate.yaml")).unwrap(),
        locked_bytes
    );
    assert_eq!(cksum_locked(), before);
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn control_plane_prove_refuses_the_locked_estate() {
    let before = cksum_locked();
    assert!(before.starts_with("43770130 3391"), "{before}");
    let locked = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&locked).unwrap();
    let run = bin()
        .args([
            "control-plane-prove",
            "--root",
            repo_root().to_str().unwrap(),
            "--out",
            locked.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run.stderr).to_string();
    assert!(!run.status.success(), "{stdout}\n{stderr}");
    assert!(
        stderr.contains("refuse:out: control-plane-prove does not write examples/estate.yaml"),
        "{stderr}"
    );
    assert!(!stdout.contains("control-plane-prove: ok"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert_eq!(fs::read(&locked).unwrap(), bytes);
    assert_eq!(cksum_locked(), before);
}
