//! `estate decisions improvement-apply-prove` and `apply-package`.
//!
//! Host-validate receipts become a standing package; one specialty-seat
//! proposal is gated-applied through plan → apply `--require-plan`.
//! No auto-train. Locked `examples/estate.yaml` stays put.

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
    let path = std::env::temp_dir().join(format!("cell-one-improvement-apply-{name}-{token}"));
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

#[test]
fn improvement_apply_prove_gates_one_specialty_seat_without_auto_train() {
    let before = cksum_locked();
    assert!(
        before.starts_with("43770130 3391"),
        "examples/estate.yaml cksum drifted: {before}"
    );
    let locked_bytes = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
    let out = scratch("prove");
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "improvement-apply-prove",
        "--root",
        repo_root().to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "{stdout}\n{stderr}");
    assert!(stdout.contains("improvement-apply-prove: host-validate"), "{stdout}");
    assert!(stdout.contains("improvement-apply-prove: export-package"), "{stdout}");
    assert!(stdout.contains("improvement-apply-prove: refuse-without-plan"), "{stdout}");
    assert!(
        stdout.contains("refuse:plan: apply-package requires --require-plan"),
        "{stdout}"
    );
    assert!(stdout.contains("improvement-apply-prove: apply --require-plan"), "{stdout}");
    assert!(stdout.contains("improvement-apply-prove: standing-next"), "{stdout}");
    assert!(stdout.contains("cell-one.improvement-apply-prove.v0"), "{stdout}");
    assert!(stdout.contains("cell-one.improvement-apply.v0"), "{stdout}");
    assert!(stdout.contains("specialty-seat:ag_news"), "{stdout}");
    assert!(stdout.contains("\"auto_train\": false"), "{stdout}");
    assert!(stdout.contains("\"train_invoked\": false"), "{stdout}");
    assert!(stdout.contains("\"ready_for_live_test\": false"), "{stdout}");
    assert!(stdout.contains("\"live_pass_recorded\": false"), "{stdout}");
    assert!(stdout.contains("\"live_sync\": false"), "{stdout}");
    assert!(stdout.contains("\"joinable\": true"), "{stdout}");
    assert!(stdout.contains("joinable: yes"), "{stdout}");
    assert!(
        stdout.contains("auto_train=false train_invoked=no"),
        "{stdout}"
    );
    assert!(!stdout.contains("\"auto_train\": true"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert!(!stdout.contains("live PASS recorded"), "{stdout}");
    let tail = stdout.trim_end();
    assert!(
        tail.ends_with("improvement-apply-prove: ok\nREADY_FOR_LIVE_TEST: no"),
        "{stdout}"
    );

    let report: Value = serde_json::from_str(
        &fs::read_to_string(out.join("improvement-apply-prove.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["schema"], "cell-one.improvement-apply-prove.v0");
    assert_eq!(report["ok"], true);
    assert_eq!(report["ready_for_live_test"], false);
    assert_eq!(report["live_pass_recorded"], false);
    assert_eq!(report["live_sync"], false);
    assert_eq!(report["auto_train"], false);
    assert_eq!(report["train_invoked"], false);
    assert_eq!(report["applied_proposal_id"], "specialty-seat:ag_news");
    assert_eq!(report["applied_proposal_kind"], "specialty-seat");
    assert_eq!(report["binding_id"], "ag_news");
    assert_eq!(report["joinable"], true);
    assert_eq!(report["standing"], "joinable: yes");
    assert_eq!(report["require_plan"], true);
    assert_eq!(
        report["refuse_without_plan"],
        "refuse:plan: apply-package requires --require-plan"
    );

    let receipt_path = report["apply_receipt"].as_str().unwrap();
    assert!(Path::new(receipt_path).is_file(), "{receipt_path}");
    let receipt: Value = serde_json::from_str(&fs::read_to_string(receipt_path).unwrap()).unwrap();
    assert_eq!(receipt["schema"], "cell-one.improvement-apply.v0");
    assert_eq!(receipt["proposal_id"], "specialty-seat:ag_news");
    assert_eq!(receipt["proposal_kind"], "specialty-seat");
    assert_eq!(receipt["auto_train"], false);
    assert_eq!(receipt["train_invoked"], false);
    assert_eq!(receipt["joinable"], true);
    assert_eq!(receipt["require_plan"], true);

    let lab = PathBuf::from(report["lab_estate"].as_str().unwrap());
    let estate = estate_schema::load_estate(&lab).unwrap();
    assert!(estate.model_bindings.iter().any(|row| row.id == "ag_news"));
    assert!(estate.model_bindings.iter().any(|row| row.id == "local_slm"));
    assert!(!estate.model_bindings.iter().any(|row| row.id == "rust_idiom"));

    assert_eq!(
        fs::read(repo_root().join("examples/estate.yaml")).unwrap(),
        locked_bytes
    );
    assert_eq!(cksum_locked(), before);

    let art = PathBuf::from("/opt/cursor/artifacts");
    if fs::create_dir_all(&art).is_ok() {
        let _ = fs::write(
            art.join("improvement-apply-prove.json"),
            serde_json::to_string_pretty(&report).unwrap() + "\n",
        );
        let _ = fs::write(art.join("improvement-apply-prove.log"), &stdout);
        let _ = fs::copy(receipt_path, art.join("improvement-apply.json"));
    }
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn apply_package_refuses_without_require_plan() {
    let before = cksum_locked();
    assert!(before.starts_with("43770130 3391"), "{before}");
    let locked = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&locked).unwrap();
    let decoy = scratch("no-plan");
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "apply-package",
        "--package",
        decoy.join("missing-package.json").to_str().unwrap(),
        "--estate",
        decoy.join("missing-estate.yaml").to_str().unwrap(),
        "--prepared",
        decoy.join("missing-prepared").to_str().unwrap(),
        "--root",
        repo_root().to_str().unwrap(),
    ]);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains("refuse:plan: apply-package requires --require-plan"),
        "{stderr}"
    );
    assert!(!stdout.contains("applied improvement proposal"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert_eq!(fs::read(&locked).unwrap(), bytes);
    assert_eq!(cksum_locked(), before);
    let _ = fs::remove_dir_all(&decoy);
}

#[test]
fn apply_package_refuses_the_locked_estate() {
    let before = cksum_locked();
    assert!(before.starts_with("43770130 3391"), "{before}");
    let locked = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&locked).unwrap();
    let decoy = scratch("locked-apply");
    fs::write(
        decoy.join("improvement-package.json"),
        r#"{
  "schema": "cell-one.improvement-package.v0",
  "id": "improvement-from-decisions",
  "kind": "standing-improvement",
  "auto_train": false,
  "train_invoked": false,
  "ready_for_live_test": false,
  "live_pass_recorded": false,
  "live_sync": false,
  "journal": {
    "path": "journal",
    "receipts": 1,
    "surface_authorize": 1,
    "surface_convey": 1,
    "surface_complete": 1,
    "specialty_seats": ["ag_news"]
  },
  "proposals": [
    {
      "id": "specialty-seat:ag_news",
      "kind": "specialty-seat",
      "binding_id": "ag_news",
      "dataset": "ag_news",
      "action": "enrich-prepare",
      "auto_train": false,
      "note": "test"
    }
  ],
  "note": "test"
}
"#,
    )
    .unwrap();
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "apply-package",
        "--package",
        decoy.join("improvement-package.json").to_str().unwrap(),
        "--estate",
        locked.to_str().unwrap(),
        "--prepared",
        decoy.join("prepared").to_str().unwrap(),
        "--require-plan",
        "--root",
        repo_root().to_str().unwrap(),
    ]);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains("refuse:out: apply-package does not write examples/estate.yaml"),
        "{stderr}"
    );
    assert!(!stdout.contains("applied improvement proposal"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert_eq!(fs::read(&locked).unwrap(), bytes);
    assert_eq!(cksum_locked(), before);
    let _ = fs::remove_dir_all(&decoy);
}

#[test]
fn improvement_apply_prove_refuses_the_locked_estate() {
    let before = cksum_locked();
    assert!(before.starts_with("43770130 3391"), "{before}");
    let locked = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&locked).unwrap();
    let (ok, stdout, stderr) = run(&[
        "decisions",
        "improvement-apply-prove",
        "--root",
        repo_root().to_str().unwrap(),
        "--out",
        locked.to_str().unwrap(),
    ]);
    assert!(!ok, "{stdout}\n{stderr}");
    assert!(
        stderr.contains(
            "refuse:out: improvement-apply-prove does not write examples/estate.yaml"
        ),
        "{stderr}"
    );
    assert!(!stdout.contains("improvement-apply-prove: ok"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    assert_eq!(fs::read(&locked).unwrap(), bytes);
    assert_eq!(cksum_locked(), before);
}

#[test]
fn docs_document_improvement_apply_without_auto_train() {
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
            text.contains("improvement-apply-prove") || text.contains("apply-package"),
            "{name} missing improvement apply"
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
    assert!(day.contains("## 4g. Improvement-apply prove"), "{day}");
    assert!(day.contains("refuse:plan"), "{day}");
    assert!(day.contains("auto_train=false"), "{day}");
    assert!(log.contains("cell-one.improvement-apply-prove.v0"), "{log}");
    assert!(log.contains("cell-one.improvement-apply.v0"), "{log}");
    assert!(north.contains("apply-package"), "{north}");
    assert!(lang.contains("apply-package"), "{lang}");
}
