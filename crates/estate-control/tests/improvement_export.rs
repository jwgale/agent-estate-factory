//! `estate decisions improvement-export-prove` and `export-package`.
//!
//! Host-validate receipts become a standing improvement package that
//! proposes the next enrich without auto-train. Locked
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
    let path = std::env::temp_dir().join(format!("cell-one-improvement-export-{name}-{token}"));
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

fn planted_receipt(id: &str, seq: u64, surface: &str, result: &str, validation: &str) -> String {
    let surface_field = if surface.is_empty() {
        String::new()
    } else {
        format!(r#","surface":"{surface}""#)
    };
    format!(
        r#"{{"schema":"cell-one.decision-receipt.v0","id":"{id}","seq":{seq},"hop_id":"model","capability":"{result}","agent":"research"{surface_field},"stage":"validate","candidates":[{{"id":"{result}"}}],"result":"{result}","validation":"{validation}","outcome":"allow"}}"#
    )
}

#[test]
fn improvement_export_prove_writes_package_and_proposals_without_auto_train() {
    let before = cksum_locked();
    assert!(
        before.starts_with("43770130 3391"),
        "examples/estate.yaml cksum drifted: {before}"
    );
    let locked_bytes = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
    let out = scratch("prove");
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "improvement-export-prove",
        "--root",
        repo_root().to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stdout.contains("improvement-export-prove: host-validate"), "{stdout}");
    assert!(stdout.contains("improvement-export-prove: export-package"), "{stdout}");
    assert!(stdout.contains("wrote improvement package"), "{stdout}");
    assert!(stdout.contains("cell-one.improvement-export-prove.v0"), "{stdout}");
    assert!(stdout.contains("cell-one.improvement-package.v0"), "{stdout}");
    assert!(stdout.contains("\"auto_train\": false"), "{stdout}");
    assert!(stdout.contains("\"train_invoked\": false"), "{stdout}");
    assert!(stdout.contains("\"ready_for_live_test\": false"), "{stdout}");
    assert!(stdout.contains("\"live_pass_recorded\": false"), "{stdout}");
    assert!(stdout.contains("\"live_sync\": false"), "{stdout}");
    assert!(stdout.contains("specialty-seat"), "{stdout}");
    assert!(stdout.contains("dataset"), "{stdout}");
    assert!(stdout.contains("ag_news"), "{stdout}");
    assert!(stdout.contains("rust_idiom"), "{stdout}");
    assert!(
        stdout.contains("auto_train=false train_invoked=no"),
        "{stdout}"
    );
    assert!(!stdout.contains("\"auto_train\": true"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!stdout.contains("live PASS recorded"), "{stdout}");
    let tail = stdout.trim_end();
    assert!(
        tail.ends_with("improvement-export-prove: ok\nREADY_FOR_LIVE_TEST: no"),
        "{stdout}"
    );

    let report: Value = serde_json::from_str(
        &fs::read_to_string(out.join("improvement-export-prove.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["schema"], "cell-one.improvement-export-prove.v0");
    assert_eq!(report["ok"], true);
    assert_eq!(report["ready_for_live_test"], false);
    assert_eq!(report["live_pass_recorded"], false);
    assert_eq!(report["live_sync"], false);
    assert_eq!(report["auto_train"], false);
    assert_eq!(report["train_invoked"], false);
    assert!(report["proposal_count"].as_u64().unwrap() >= 4);
    let kinds = report["proposal_kinds"].as_array().unwrap();
    assert!(kinds.iter().any(|row| row.as_str() == Some("specialty-seat")));
    assert!(kinds.iter().any(|row| row.as_str() == Some("dataset")));
    assert_eq!(report["surfaces"]["authorize"], 4);
    assert_eq!(report["surfaces"]["convey"], 4);
    assert_eq!(report["surfaces"]["complete"], 5);
    let package_path = report["package_path"].as_str().unwrap();
    assert!(package_path.ends_with("improvement-package.json"), "{package_path}");
    assert!(Path::new(package_path).is_file(), "{package_path}");

    let package: Value =
        serde_json::from_str(&fs::read_to_string(package_path).unwrap()).unwrap();
    assert_eq!(package["schema"], "cell-one.improvement-package.v0");
    assert_eq!(package["kind"], "standing-improvement");
    assert_eq!(package["auto_train"], false);
    assert_eq!(package["train_invoked"], false);
    assert_eq!(package["ready_for_live_test"], false);
    assert_eq!(package["live_pass_recorded"], false);
    assert_eq!(package["live_sync"], false);
    let proposals = package["proposals"].as_array().unwrap();
    assert!(proposals.len() >= 4, "{proposals:?}");
    assert!(proposals.iter().all(|row| row["auto_train"] == false));
    assert!(proposals.iter().all(|row| row["action"] == "enrich-prepare"));
    assert!(proposals.iter().any(|row| {
        row["kind"] == "specialty-seat" && row["binding_id"] == "ag_news"
    }));
    assert!(proposals.iter().any(|row| {
        row["kind"] == "dataset" && row["dataset"] == "rust_idiom"
    }));

    let yaml = fs::read_to_string(out.join("improvement/improvement-package.yaml")).unwrap();
    assert!(yaml.contains("schema: cell-one.improvement-package.v0"), "{yaml}");
    assert!(yaml.contains("auto_train: false"), "{yaml}");
    assert!(yaml.contains("train_invoked: false"), "{yaml}");
    assert!(!yaml.contains("auto_train: true"), "{yaml}");
    assert!(yaml.contains("kind: specialty-seat"), "{yaml}");
    assert!(yaml.contains("kind: dataset"), "{yaml}");

    assert!(out.join("host-validate-prove.json").is_file());
    assert!(out.join("state/decisions/receipts.jsonl").is_file());
    assert_eq!(
        fs::read(repo_root().join("examples/estate.yaml")).unwrap(),
        locked_bytes
    );
    assert_eq!(cksum_locked(), before);

    let art = PathBuf::from("/opt/cursor/artifacts");
    if fs::create_dir_all(&art).is_ok() {
        let _ = fs::write(
            art.join("improvement-export-prove.json"),
            serde_json::to_string_pretty(&report).unwrap() + "\n",
        );
        let _ = fs::write(art.join("improvement-export-prove.log"), &stdout);
        let _ = fs::copy(package_path, art.join("improvement-package.json"));
        let _ = fs::copy(
            out.join("improvement/improvement-package.yaml"),
            art.join("improvement-package.yaml"),
        );
    }
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn export_package_from_planted_journal_proposes_specialty_without_train() {
    let before = cksum_locked();
    assert!(before.starts_with("43770130 3391"), "{before}");
    let locked_bytes = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
    let root = scratch("planted");
    let state = root.join("state");
    fs::create_dir_all(state.join("decisions")).unwrap();
    let journal = [
        planted_receipt("r-1", 1, "authorize", "ag_news", "ok"),
        planted_receipt("r-2", 2, "", "rust_idiom", "ok"),
        planted_receipt("r-3", 3, "complete", "ag_news", "ok"),
        planted_receipt("r-4", 4, "complete", "abstain", "ok"),
        planted_receipt("r-5", 5, "complete", "xai_grok", "ok"),
    ]
    .join("\n")
        + "\n";
    fs::write(state.join("decisions/receipts.jsonl"), journal).unwrap();
    let out = root.join("improvement");
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "export-package",
        "--root",
        repo_root().to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stdout.contains("wrote improvement package"), "{stdout}");
    assert!(stdout.contains("auto_train=false train_invoked=no"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");

    let package: Value = serde_json::from_str(
        &fs::read_to_string(out.join("improvement-package.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(package["schema"], "cell-one.improvement-package.v0");
    assert_eq!(package["auto_train"], false);
    assert_eq!(package["train_invoked"], false);
    assert_eq!(package["ready_for_live_test"], false);
    assert_eq!(package["live_sync"], false);
    let seats = package["journal"]["specialty_seats"].as_array().unwrap();
    assert!(seats.iter().any(|row| row.as_str() == Some("ag_news")));
    assert!(seats.iter().any(|row| row.as_str() == Some("rust_idiom")));
    assert!(!seats.iter().any(|row| row.as_str() == Some("abstain")));
    assert!(!seats.iter().any(|row| row.as_str() == Some("xai_grok")));
    let proposals = package["proposals"].as_array().unwrap();
    assert_eq!(proposals.len(), 4);
    assert!(proposals.iter().all(|row| row["auto_train"] == false));
    assert_eq!(
        fs::read(repo_root().join("examples/estate.yaml")).unwrap(),
        locked_bytes
    );
    assert_eq!(cksum_locked(), before);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn improvement_export_prove_refuses_the_locked_estate() {
    let before = cksum_locked();
    assert!(before.starts_with("43770130 3391"), "{before}");
    let locked = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&locked).unwrap();
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "improvement-export-prove",
        "--root",
        repo_root().to_str().unwrap(),
        "--out",
        locked.to_str().unwrap(),
    ]);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains(
            "refuse:out: improvement-export-prove does not write examples/estate.yaml"
        ),
        "{stderr}"
    );
    assert!(!stdout.contains("improvement-export-prove: ok"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert_eq!(fs::read(&locked).unwrap(), bytes);
    assert_eq!(cksum_locked(), before);
}

#[test]
fn export_package_refuses_examples_tree_when_root_lacks_estate() {
    let before = cksum_locked();
    assert!(before.starts_with("43770130 3391"), "{before}");
    let locked = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&locked).unwrap();
    let decoy = scratch("decoy-root");
    let planted = scratch("decoy-state");
    fs::create_dir_all(planted.join("decisions")).unwrap();
    fs::write(
        planted.join("decisions/receipts.jsonl"),
        planted_receipt("r-1", 1, "complete", "ag_news", "ok") + "\n",
    )
    .unwrap();
    let under_examples = repo_root().join("examples").join("improvement-adversarial");
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "export-package",
        "--root",
        decoy.to_str().unwrap(),
        "--state-dir",
        planted.to_str().unwrap(),
        "--out",
        under_examples.to_str().unwrap(),
    ]);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains("refuse:out: export-package does not write examples/estate.yaml"),
        "{stderr}"
    );
    assert!(!stdout.contains("wrote improvement package"), "{stdout}");
    assert!(!under_examples.exists(), "wrote under examples/: {}", under_examples.display());
    assert_eq!(fs::read(&locked).unwrap(), bytes);
    assert_eq!(cksum_locked(), before);
    let _ = fs::remove_dir_all(&decoy);
    let _ = fs::remove_dir_all(&planted);
}

#[test]
fn export_package_refuses_the_locked_estate() {
    let before = cksum_locked();
    assert!(before.starts_with("43770130 3391"), "{before}");
    let locked = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&locked).unwrap();
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "export-package",
        "--root",
        repo_root().to_str().unwrap(),
        "--state-dir",
        scratch("refuse-state").to_str().unwrap(),
        "--out",
        locked.to_str().unwrap(),
    ]);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains("refuse:out: export-package does not write examples/estate.yaml"),
        "{stderr}"
    );
    assert!(!stdout.contains("wrote improvement package"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert_eq!(fs::read(&locked).unwrap(), bytes);
    assert_eq!(cksum_locked(), before);
}

#[test]
fn docs_document_improvement_export_without_auto_train() {
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
        assert!(
            text.contains("improvement-export-prove") || text.contains("export-package"),
            "{name} missing improvement export"
        );
        assert!(
            text.contains("auto_train"),
            "{name} missing auto_train lock"
        );
        assert!(text.contains("43770130 3391"), "{name} missing locked cksum");
        assert!(
            text.contains("READY_FOR_LIVE_TEST"),
            "{name} missing READY_FOR_LIVE_TEST"
        );
        assert!(!text.contains("READY_FOR_LIVE_TEST: yes"), "{name}");
    }
    assert!(day.contains("## 4f. Improvement-export prove"), "{day}");
    assert!(day.contains("auto_train=false"), "{day}");
    assert!(day.contains("cohesion-prove"), "{day}");
    assert!(log.contains("cell-one.improvement-export-prove.v0"), "{log}");
    assert!(log.contains("cell-one.improvement-package.v0"), "{log}");
    assert!(north.contains("export-package"), "{north}");
    assert!(
        north.contains("composes") || day.contains("composes that same export-package"),
        "docs must say cohesion composes the export-package stage"
    );
}
