//! `estate pack cohesion-prove` on one throwaway lab.
//! Fuel, decide, run, pack install, and CLI smoke. Locked
//! `examples/estate.yaml` stays put.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
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
    let path = std::env::temp_dir().join(format!("cell-one-cohesion-{name}-{token}"));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn cksum(path: &Path) -> String {
    let out = Command::new("cksum").arg(path).output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn cksum_locked() -> String {
    cksum(&repo_root().join("examples/estate.yaml"))
}

#[test]
fn cohesion_prove_stitches_fuel_decide_run_and_pack_cli_smoke() {
    let before = cksum_locked();
    assert!(
        before.starts_with("43770130 3391"),
        "examples/estate.yaml cksum drifted: {before}"
    );
    let locked_bytes = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
    let fixture = repo_root().join("examples/fixtures/agent-pack-handoff.yaml");
    let fixture_bytes = fs::read(&fixture).unwrap();
    let fixture_cksum = cksum(&fixture);
    let out = scratch("prove");
    let sentinel = scratch("sentinel-home");
    let run = bin()
        .args([
            "pack",
            "cohesion-prove",
            "--root",
            repo_root().to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--id",
            "research-crew",
            "--estate",
            "examples/fixtures/agent-pack-handoff.yaml",
        ])
        .env("HOME", &sentinel)
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_LOCAL_LIVE")
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_CURSOR_PLUGINS_MODULE")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run.stderr).to_string();
    let text = format!("{stdout}{stderr}");
    assert!(run.status.success(), "{text}");
    assert!(stdout.contains("cohesion-prove: control-plane"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: pack"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: cli-smoke"), "{stdout}");
    assert!(stdout.contains("joinable: yes"), "{stdout}");
    assert!(stdout.contains("binding: ag_news"), "{stdout}");
    assert!(stdout.contains("binding: rust_idiom"), "{stdout}");
    assert!(
        stdout.contains("surface authorize=4 convey=4 complete=5"),
        "{stdout}"
    );
    assert!(stdout.contains("refuse:decision-abstain"), "{stdout}");
    assert!(
        stdout.contains("runner hop: research capability=ag_news context=none"),
        "{stdout}"
    );
    assert!(
        stdout.contains("runner hop: idiom capability=rust_idiom context=applied"),
        "{stdout}"
    );
    assert!(
        stdout.contains("runner hop: horizon capability=frontier_http context=applied"),
        "{stdout}"
    );
    assert!(stdout.contains("cell-one.control-plane-prove.v0"), "{stdout}");
    assert!(
        stdout.contains("cli-smoke horizon: decision receipt outcome=allow surface=complete pack=research-crew capability=ag_news"),
        "{stdout}"
    );
    assert!(
        stdout.contains("cli-smoke research: refuse:pack-orchestrator"),
        "{stdout}"
    );
    assert!(stdout.contains("cell-one.cohesion-prove.v0"), "{stdout}");
    let tail = stdout.trim_end();
    assert!(
        tail.ends_with("cohesion-prove: ok\nREADY_FOR_LIVE_TEST: no"),
        "{stdout}"
    );
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!text.contains("live PASS recorded"), "{text}");
    assert!(!sentinel.join(".cursor").exists(), "install wrote the sentinel HOME");

    let report: Value =
        serde_json::from_str(&fs::read_to_string(out.join("cohesion-prove.json")).unwrap()).unwrap();
    assert_eq!(report["schema"], "cell-one.cohesion-prove.v0");
    assert_eq!(report["ok"], true);
    assert_eq!(report["ready_for_live_test"], false);
    assert_eq!(report["live_pass_recorded"], false);
    assert_eq!(report["live_sync"], false);
    assert_eq!(report["control_plane_schema"], "cell-one.control-plane-prove.v0");
    assert_eq!(report["fuel"]["trained_shape"], "gguf");
    assert_eq!(report["fuel"]["auto_apply"], false);
    assert_eq!(report["fuel"]["beside"], "local_slm");
    assert_eq!(report["fuel"]["joinable"]["ag_news"], true);
    assert_eq!(report["fuel"]["joinable"]["rust_idiom"], true);
    assert_eq!(report["fuel"]["seat_models"]["ag_news"], "specialist-agnews-3000");
    assert_eq!(
        report["fuel"]["seat_models"]["rust_idiom"],
        "specialist-rustidiom-3000"
    );
    assert_eq!(report["decide"]["surface_authorize"], 4);
    assert_eq!(report["decide"]["surface_convey"], 4);
    assert_eq!(report["decide"]["surface_complete"], 5);
    assert_eq!(report["decide"]["abstain"], "refuse:decision-abstain");
    assert_eq!(report["decide"]["stale_fallback"], "ag_news");
    assert_eq!(report["decide"]["ineligible_fallback"], "ag_news");
    assert_eq!(report["run"]["routine_id"], "standing-dual");
    assert_eq!(report["run"]["session_stitch"], true);
    assert_eq!(report["run"]["hops"].as_array().unwrap().len(), 3);
    assert_eq!(report["pack"]["id"], "research-crew");
    assert_eq!(report["pack"]["cursor_loader"], "out-of-scope");
    assert_eq!(report["pack"]["loader_is_live_pass"], false);
    assert_eq!(report["pack"]["cli_smoke_docs"], true);
    assert_eq!(report["pack"]["runner_docs"], "yes");
    assert_eq!(report["pack"]["session_docs"], "yes");
    assert_eq!(
        report["pack"]["cli_smoke"]["orchestrator"]["outcome"],
        "allow"
    );
    assert_eq!(
        report["pack"]["cli_smoke"]["orchestrator"]["capability"],
        "ag_news"
    );
    assert_eq!(
        report["pack"]["cli_smoke"]["orchestrator"]["agent"],
        "horizon"
    );
    assert_eq!(
        report["pack"]["cli_smoke"]["member"]["refuse"],
        "refuse:pack-orchestrator"
    );
    assert_eq!(report["pack"]["cli_smoke"]["member"]["agent"], "research");
    assert_eq!(report["pack"]["cli_smoke"]["member"]["receipt_written"], false);
    let loaded = report["pack"]["loaded"].as_str().unwrap();
    assert!(
        loaded.starts_with("skipped:") || loaded == "yes",
        "{loaded}"
    );
    let install_path = PathBuf::from(report["pack"]["install_path"].as_str().unwrap());
    let home = out.join("home");
    assert!(
        install_path.starts_with(&home),
        "{} not under {}",
        install_path.display(),
        home.display()
    );
    let meta = fs::symlink_metadata(&install_path).unwrap();
    assert!(meta.is_dir());
    assert!(!meta.file_type().is_symlink());
    assert!(install_path.join(".estate-pack-install.json").is_file());
    let install_md = fs::read_to_string(install_path.join("INSTALL.md")).unwrap();
    assert!(install_md.contains("CLI smoke (first-class)"), "{install_md}");
    assert!(
        install_md.contains("Cursor MCP loader hang is out of scope"),
        "{install_md}"
    );
    let readme = fs::read_to_string(install_path.join("README.md")).unwrap();
    assert!(readme.contains("CLI smoke (first-class)"), "{readme}");
    let horizon = fs::read_to_string(
        out.join("cli-smoke/horizon/decisions/receipts.jsonl"),
    )
    .unwrap();
    let row: Value = serde_json::from_str(horizon.lines().next().unwrap()).unwrap();
    assert_eq!(row["outcome"], "allow");
    assert_eq!(row["surface"], "complete");
    assert_eq!(row["pack_id"], "research-crew");
    assert_eq!(row["handoff_from"], "horizon");
    assert!(!out.join("cli-smoke/member/decisions/receipts.jsonl").exists());
    let cksum_report = report["estate_cksum"].as_str().unwrap();
    assert!(cksum_report.starts_with("43770130 3391"), "{cksum_report}");
    let rendered = serde_json::to_string(&report).unwrap();
    assert!(
        !rendered.split_whitespace().any(|word| word == "enforced"),
        "{rendered}"
    );
    assert_eq!(fs::read(repo_root().join("examples/estate.yaml")).unwrap(), locked_bytes);
    assert_eq!(fs::read(&fixture).unwrap(), fixture_bytes);
    assert_eq!(cksum_locked(), before);
    assert_eq!(cksum(&fixture), fixture_cksum);

    let art = PathBuf::from("/opt/cursor/artifacts");
    if fs::create_dir_all(&art).is_ok() {
        let _ = fs::write(art.join("cohesion-prove.json"), serde_json::to_string_pretty(&report).unwrap() + "\n");
        let _ = fs::write(art.join("cohesion-prove.log"), &stdout);
    }
    let _ = fs::remove_dir_all(&out);
    let _ = fs::remove_dir_all(&sentinel);
}

#[test]
fn cohesion_prove_refuses_the_locked_estate() {
    let before = cksum_locked();
    assert!(before.starts_with("43770130 3391"), "{before}");
    let locked = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&locked).unwrap();
    let run = bin()
        .args([
            "pack",
            "cohesion-prove",
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
        stderr.contains("refuse:out: cohesion-prove does not write examples/estate.yaml"),
        "{stderr}"
    );
    assert!(!stdout.contains("cohesion-prove: ok"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert_eq!(fs::read(&locked).unwrap(), bytes);
    assert_eq!(cksum_locked(), before);
}

#[test]
fn docs_document_cohesion_prove_and_cli_smoke() {
    let root = repo_root();
    let north = fs::read_to_string(root.join("docs/NORTH-STAR.md")).unwrap();
    let day = fs::read_to_string(root.join("docs/OPERATOR-DAY.md")).unwrap();
    let lang = fs::read_to_string(root.join("docs/UBIQUITOUS_LANGUAGE.md")).unwrap();
    let log = fs::read_to_string(root.join("CHANGELOG.md")).unwrap();
    for (name, text) in [
        ("NORTH-STAR", north.as_str()),
        ("OPERATOR-DAY", day.as_str()),
        ("UBIQUITOUS_LANGUAGE", lang.as_str()),
        ("CHANGELOG", log.as_str()),
    ] {
        assert!(text.contains("cohesion-prove"), "{name} missing cohesion-prove");
        assert!(
            text.contains("estate complete --mock"),
            "{name} missing CLI smoke command"
        );
        assert!(
            text.contains("Cursor MCP loader hang is out of scope"),
            "{name} missing loader scope"
        );
        assert!(text.contains("43770130 3391"), "{name} missing locked cksum");
        assert!(
            text.contains("READY_FOR_LIVE_TEST"),
            "{name} missing READY_FOR_LIVE_TEST"
        );
    }
    assert!(day.contains("## 4e. Cohesion prove"), "{day}");
    assert!(day.contains("refuse:pack-orchestrator"), "{day}");
    assert!(log.contains("cell-one.cohesion-prove.v0"), "{log}");
    assert!(!north.contains("READY_FOR_LIVE_TEST: yes"), "{north}");
}
