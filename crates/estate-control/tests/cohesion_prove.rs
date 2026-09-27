//! `estate pack cohesion-prove` on one throwaway lab.
//! Fuel, decide, run, pack install, CLI smoke, improvement export,
//! and one gated specialty-seat apply. Locked `examples/estate.yaml`
//! stays put.

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
    let no_gguf = scratch("no-gguf");
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
        .env_remove("CELL_SPECIALTY_AG_NEWS_GGUF")
        .env_remove("CELL_SPECIALTY_RUST_IDIOM_GGUF")
        .env("CELL_SPECIALTY_GGUF_ROOT", no_gguf.to_str().unwrap())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run.stderr).to_string();
    let text = format!("{stdout}{stderr}");
    assert!(run.status.success(), "{text}");
    assert!(stdout.contains("cohesion-prove: control-plane"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: specialty-real"), "{stdout}");
    assert!(stdout.contains("specialty-real: skipped:gguf-absent"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: pack"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: cli-smoke"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: improvement-export"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: improvement-apply"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: decisions report"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: pack session show"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: status"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: refuse-without-plan"), "{stdout}");
    assert!(
        stdout.contains("refuse:plan: apply-package requires --require-plan"),
        "{stdout}"
    );
    assert!(stdout.contains("cohesion-prove: apply --require-plan"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: standing-next"), "{stdout}");
    assert!(stdout.contains("wrote improvement package"), "{stdout}");
    let export_at = stdout.find("cohesion-prove: improvement-export").unwrap();
    let wrote_at = stdout.find("wrote improvement package").unwrap();
    let apply_at = stdout.find("cohesion-prove: improvement-apply").unwrap();
    let refuse_at = stdout.find("cohesion-prove: refuse-without-plan").unwrap();
    let gated_at = stdout.find("cohesion-prove: apply --require-plan").unwrap();
    let decisions_at = stdout.find("cohesion-prove: decisions report").unwrap();
    let session_show_at = stdout.find("cohesion-prove: pack session show").unwrap();
    let status_at = stdout.find("cohesion-prove: status").unwrap();
    assert!(export_at < wrote_at && wrote_at < apply_at, "{stdout}");
    assert!(apply_at < refuse_at && refuse_at < gated_at, "{stdout}");
    assert!(gated_at < decisions_at, "{stdout}");
    assert!(decisions_at < session_show_at, "{stdout}");
    assert!(session_show_at < status_at, "{stdout}");
    assert!(
        stdout.contains("apply receipt: schema=cell-one.improvement-apply.v0"),
        "{stdout}"
    );
    assert!(
        stdout.contains("applied proposal specialty-seat:ag_news"),
        "{stdout}"
    );
    assert!(stdout.contains("kind=specialty-seat"), "{stdout}");
    assert!(stdout.contains("binding=ag_news"), "{stdout}");
    assert!(stdout.contains("standing=joinable: yes"), "{stdout}");
    assert!(stdout.contains("joinable=true"), "{stdout}");
    assert!(stdout.contains("require_plan=true"), "{stdout}");
    assert!(
        stdout.contains("refuse_without_plan=refuse:plan: apply-package requires --require-plan"),
        "{stdout}"
    );
    assert!(stdout.contains("auto_train=false"), "{stdout}");
    assert!(stdout.contains("train_invoked=false"), "{stdout}");
    assert!(
        !stdout.contains("improvement-apply-prove: host-validate"),
        "{stdout}"
    );
    assert!(
        !stdout.contains("decision-host-validate-prove: ok"),
        "{stdout}"
    );
    assert!(
        stdout.contains("auto_train=false train_invoked=no"),
        "{stdout}"
    );
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
        stdout.contains("cli-smoke horizon hop 1: decision receipt outcome=allow surface=complete pack=research-crew capability=ag_news result=ag_news session=sess-cohesion01 turns=1 context=none"),
        "{stdout}"
    );
    assert!(
        stdout.contains("cli-smoke horizon hop 2: decision receipt outcome=allow surface=complete pack=research-crew capability=ag_news result=ag_news session=sess-cohesion01 turns=2 context=applied saw_prior=yes"),
        "{stdout}"
    );
    assert!(
        stdout.contains("cli-smoke research: refuse:pack-orchestrator"),
        "{stdout}"
    );
    assert!(
        stdout.contains("cli-smoke horizon isolation: session=sess-cohesioniso turns=1 context=none leaked=no"),
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
    assert_eq!(report["auto_train"], false);
    assert_eq!(report["train_invoked"], false);
    let improvement = &report["improvement"];
    assert_eq!(improvement["auto_train"], false);
    assert_eq!(improvement["train_invoked"], false);
    assert_eq!(improvement["ready_for_live_test"], false);
    assert_eq!(improvement["live_pass_recorded"], false);
    assert_eq!(improvement["live_sync"], false);
    assert_eq!(improvement["composed_by"], "cohesion-prove");
    assert!(improvement["proposal_count"].as_u64().unwrap() >= 4);
    let kinds = improvement["proposal_kinds"].as_array().unwrap();
    assert!(kinds.iter().any(|row| row.as_str() == Some("specialty-seat")));
    assert!(kinds.iter().any(|row| row.as_str() == Some("dataset")));
    let package_path = improvement["package_path"].as_str().unwrap();
    assert!(
        package_path.ends_with("improvement-package.json"),
        "{package_path}"
    );
    assert!(Path::new(package_path).is_file(), "{package_path}");
    let package: Value =
        serde_json::from_str(&fs::read_to_string(package_path).unwrap()).unwrap();
    assert_eq!(package["schema"], "cell-one.improvement-package.v0");
    assert_eq!(package["auto_train"], false);
    assert_eq!(package["train_invoked"], false);
    let proposals = package["proposals"].as_array().unwrap();
    assert!(proposals.iter().all(|row| row["auto_train"] == false));
    assert!(proposals.iter().any(|row| {
        row["kind"] == "specialty-seat" && row["binding_id"] == "ag_news"
    }));
    assert!(proposals.iter().any(|row| {
        row["kind"] == "dataset" && row["dataset"] == "rust_idiom"
    }));
    let yaml = fs::read_to_string(out.join("improvement/improvement-package.yaml")).unwrap();
    assert!(yaml.contains("auto_train: false"), "{yaml}");
    assert!(yaml.contains("train_invoked: false"), "{yaml}");
    assert!(!yaml.contains("auto_train: true"), "{yaml}");
    let apply = &report["apply"];
    assert_eq!(apply["schema"], "cell-one.improvement-apply.v0");
    assert_eq!(apply["apply_schema"], "cell-one.improvement-apply.v0");
    assert_eq!(apply["ok"], true);
    assert_eq!(apply["auto_train"], false);
    assert_eq!(apply["train_invoked"], false);
    assert_eq!(apply["require_plan"], true);
    assert_eq!(apply["joinable"], true);
    assert_eq!(apply["standing"], "joinable: yes");
    assert_eq!(apply["local_slm"], true);
    assert_eq!(apply["dataset_proposals"], "proposal-only");
    assert_eq!(apply["ready_for_live_test"], false);
    assert_eq!(apply["live_pass_recorded"], false);
    assert_eq!(apply["live_sync"], false);
    assert_eq!(apply["host_validate_rerun"], false);
    assert_eq!(apply["reused_existing_package"], true);
    assert_eq!(apply["composed_by"], "cohesion-prove");
    assert_eq!(
        apply["refuse_without_plan"],
        "refuse:plan: apply-package requires --require-plan"
    );
    assert_eq!(apply["applied_proposal_id"], "specialty-seat:ag_news");
    assert_eq!(apply["applied_proposal_kind"], "specialty-seat");
    assert_eq!(apply["binding_id"], "ag_news");
    let dataset_ids = apply["dataset_proposal_ids"].as_array().unwrap();
    assert!(dataset_ids.iter().any(|row| row.as_str() == Some("dataset:ag_news")));
    assert!(dataset_ids.iter().any(|row| row.as_str() == Some("dataset:rust_idiom")));
    assert!(dataset_ids
        .iter()
        .all(|row| row.as_str().unwrap_or("").starts_with("dataset:")));
    assert!(!dataset_ids
        .iter()
        .any(|row| row == &apply["applied_proposal_id"]));
    let receipt_path = apply["apply_receipt"].as_str().unwrap();
    assert!(
        receipt_path.ends_with("apply/improvement-apply.json"),
        "{receipt_path}"
    );
    assert!(Path::new(receipt_path).is_file(), "{receipt_path}");
    let receipt: Value =
        serde_json::from_str(&fs::read_to_string(receipt_path).unwrap()).unwrap();
    assert_eq!(receipt["schema"], "cell-one.improvement-apply.v0");
    assert_eq!(receipt["proposal_id"], "specialty-seat:ag_news");
    assert_eq!(receipt["proposal_kind"], "specialty-seat");
    assert_eq!(receipt["binding_id"], "ag_news");
    assert_eq!(receipt["auto_train"], false);
    assert_eq!(receipt["train_invoked"], false);
    assert_eq!(receipt["require_plan"], true);
    assert_eq!(receipt["joinable"], true);
    assert_eq!(
        receipt["refuse_without_plan"],
        "refuse:plan: apply-package requires --require-plan"
    );
    let decisions = &report["decisions"];
    assert_eq!(decisions["cites_apply"], true);
    assert_eq!(decisions["apply_schema"], "cell-one.improvement-apply.v0");
    assert_eq!(decisions["applied_proposal_id"], "specialty-seat:ag_news");
    assert_eq!(decisions["applied_proposal_kind"], "specialty-seat");
    assert_eq!(decisions["binding_id"], "ag_news");
    assert_eq!(decisions["joinable"], true);
    assert_eq!(decisions["standing"], "joinable: yes");
    assert_eq!(decisions["require_plan"], true);
    assert_eq!(decisions["auto_train"], false);
    assert_eq!(decisions["train_invoked"], false);
    assert_eq!(
        decisions["refuse_without_plan"],
        "refuse:plan: apply-package requires --require-plan"
    );
    assert_eq!(decisions["apply_receipt"], apply["apply_receipt"]);
    let pack_session = &report["pack_session"];
    assert_eq!(pack_session["cites_apply"], true);
    assert_eq!(pack_session["session_id"], "sess-cohesion01");
    assert_eq!(pack_session["apply_schema"], "cell-one.improvement-apply.v0");
    assert_eq!(pack_session["applied_proposal_id"], "specialty-seat:ag_news");
    assert_eq!(pack_session["applied_proposal_kind"], "specialty-seat");
    assert_eq!(pack_session["binding_id"], "ag_news");
    assert_eq!(pack_session["joinable"], true);
    assert_eq!(pack_session["standing"], "joinable: yes");
    assert_eq!(pack_session["require_plan"], true);
    assert_eq!(pack_session["auto_train"], false);
    assert_eq!(pack_session["train_invoked"], false);
    assert_eq!(
        pack_session["refuse_without_plan"],
        "refuse:plan: apply-package requires --require-plan"
    );
    assert_eq!(pack_session["apply_receipt"], apply["apply_receipt"]);
    assert_eq!(pack_session["require_plan"], receipt["require_plan"]);
    assert_eq!(pack_session["auto_train"], receipt["auto_train"]);
    assert_eq!(pack_session["train_invoked"], receipt["train_invoked"]);
    assert_eq!(pack_session["joinable"], receipt["joinable"]);
    assert_eq!(pack_session["standing"], receipt["standing"]);
    assert_eq!(pack_session["refuse_without_plan"], receipt["refuse_without_plan"]);
    assert_eq!(pack_session["applied_proposal_id"], receipt["proposal_id"]);
    assert_eq!(pack_session["applied_proposal_kind"], receipt["proposal_kind"]);
    assert_eq!(pack_session["binding_id"], receipt["binding_id"]);
    let status = &report["status"];
    assert_eq!(status["cites_apply"], true);
    assert_eq!(status["apply_schema"], "cell-one.improvement-apply.v0");
    assert_eq!(status["applied_proposal_id"], "specialty-seat:ag_news");
    assert_eq!(status["applied_proposal_kind"], "specialty-seat");
    assert_eq!(status["binding_id"], "ag_news");
    assert_eq!(status["joinable"], true);
    assert_eq!(status["standing"], "joinable: yes");
    assert_eq!(status["require_plan"], true);
    assert_eq!(status["auto_train"], false);
    assert_eq!(status["train_invoked"], false);
    assert_eq!(
        status["refuse_without_plan"],
        "refuse:plan: apply-package requires --require-plan"
    );
    assert_eq!(status["apply_receipt"], apply["apply_receipt"]);
    assert_eq!(status["require_plan"], receipt["require_plan"]);
    assert_eq!(status["auto_train"], receipt["auto_train"]);
    assert_eq!(status["train_invoked"], receipt["train_invoked"]);
    assert_eq!(status["joinable"], receipt["joinable"]);
    assert_eq!(status["standing"], receipt["standing"]);
    assert_eq!(status["refuse_without_plan"], receipt["refuse_without_plan"]);
    assert_eq!(status["applied_proposal_id"], receipt["proposal_id"]);
    assert_eq!(status["applied_proposal_kind"], receipt["proposal_kind"]);
    assert_eq!(status["binding_id"], receipt["binding_id"]);
    let status_cite = status["cite"].as_str().unwrap();
    assert!(status_cite.contains("schema=cell-one.improvement-apply.v0"), "{status_cite}");
    assert!(status_cite.contains("applied proposal specialty-seat:ag_news"), "{status_cite}");
    assert!(status_cite.contains("joinable=true"), "{status_cite}");
    assert!(status_cite.contains("require_plan=true"), "{status_cite}");
    assert!(status_cite.contains("auto_train=false"), "{status_cite}");
    assert!(status_cite.contains("train_invoked=false"), "{status_cite}");
    assert!(
        status["state_dir"].as_str().unwrap().ends_with("apply/state"),
        "{}",
        status["state_dir"]
    );
    let apply_state = out.join("apply").join("state");
    assert!(apply_state.join("decisions").join("improvement-apply.json").is_file());
    let status_path = status_cite.lines().find_map(|line| {
        line.strip_prefix("apply receipt: schema=cell-one.improvement-apply.v0 path=")
    }).expect(status_cite);
    assert!(
        Path::new(status_path).starts_with(&apply_state)
            || Path::new(status_path)
                .canonicalize()
                .ok()
                .is_some_and(|p| p.starts_with(apply_state.canonicalize().unwrap())),
        "{status_cite}"
    );
    assert!(!status_cite.contains("/apply/improvement-apply.json"), "{status_cite}");
    let session_cite = pack_session["cite"].as_str().unwrap();
    assert!(session_cite.contains("schema=cell-one.improvement-apply.v0"), "{session_cite}");
    assert!(session_cite.contains("applied proposal specialty-seat:ag_news"), "{session_cite}");
    assert!(session_cite.contains("joinable=true"), "{session_cite}");
    assert!(session_cite.contains("require_plan=true"), "{session_cite}");
    assert!(session_cite.contains("auto_train=false"), "{session_cite}");
    assert!(session_cite.contains("train_invoked=false"), "{session_cite}");
    let crew_state = out.join("cli-smoke").join("crew");
    assert!(
        pack_session["state_dir"].as_str().unwrap().ends_with("cli-smoke/crew"),
        "{}",
        pack_session["state_dir"]
    );
    assert!(crew_state.join("decisions").join("improvement-apply.json").is_file());
    assert!(!session_cite.contains("/apply/improvement-apply.json"), "{session_cite}");
    assert_eq!(decisions["require_plan"], receipt["require_plan"]);
    assert_eq!(decisions["auto_train"], receipt["auto_train"]);
    assert_eq!(decisions["train_invoked"], receipt["train_invoked"]);
    assert_eq!(decisions["joinable"], receipt["joinable"]);
    assert_eq!(decisions["standing"], receipt["standing"]);
    assert_eq!(decisions["refuse_without_plan"], receipt["refuse_without_plan"]);
    assert_eq!(decisions["applied_proposal_id"], receipt["proposal_id"]);
    assert_eq!(decisions["applied_proposal_kind"], receipt["proposal_kind"]);
    assert_eq!(decisions["binding_id"], receipt["binding_id"]);
    let cite = decisions["cite"].as_str().unwrap();
    assert!(cite.contains("schema=cell-one.improvement-apply.v0"), "{cite}");
    assert!(cite.contains("applied proposal specialty-seat:ag_news"), "{cite}");
    assert!(cite.contains("joinable=true"), "{cite}");
    assert!(cite.contains("require_plan=true"), "{cite}");
    assert!(cite.contains("auto_train=false"), "{cite}");
    assert!(cite.contains("train_invoked=false"), "{cite}");
    let report_run = bin()
        .args([
            "decisions",
            "report",
            "--state-dir",
            out.join("state").to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_LOCAL_LIVE")
        .env_remove("XAI_API_KEY")
        .output()
        .unwrap();
    let report_out = String::from_utf8_lossy(&report_run.stdout).to_string();
    let report_err = String::from_utf8_lossy(&report_run.stderr).to_string();
    assert!(report_run.status.success(), "{report_out}\n{report_err}");
    assert!(
        report_out.contains("apply receipt: schema=cell-one.improvement-apply.v0"),
        "{report_out}"
    );
    assert!(
        report_out.contains("applied proposal specialty-seat:ag_news"),
        "{report_out}"
    );
    assert!(report_out.contains("kind=specialty-seat"), "{report_out}");
    assert!(report_out.contains("binding=ag_news"), "{report_out}");
    assert!(report_out.contains("standing=joinable: yes"), "{report_out}");
    assert!(report_out.contains("joinable=true"), "{report_out}");
    assert!(report_out.contains("require_plan=true"), "{report_out}");
    assert!(
        report_out.contains("refuse_without_plan=refuse:plan: apply-package requires --require-plan"),
        "{report_out}"
    );
    assert!(report_out.contains("auto_train=false"), "{report_out}");
    assert!(report_out.contains("train_invoked=false"), "{report_out}");
    assert!(report_out.contains("decision receipts:"), "{report_out}");
    let session_run = bin()
        .args([
            "pack",
            "session",
            "show",
            "--id",
            "sess-cohesion01",
            "--pack",
            "research-crew",
            "--state-dir",
            out.join("cli-smoke/crew").to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_LOCAL_LIVE")
        .env_remove("XAI_API_KEY")
        .output()
        .unwrap();
    let session_out = String::from_utf8_lossy(&session_run.stdout).to_string();
    let session_err = String::from_utf8_lossy(&session_run.stderr).to_string();
    assert!(session_run.status.success(), "{session_out}\n{session_err}");
    assert!(session_out.contains("turns=2"), "{session_out}");
    assert!(session_out.contains("unique-hop-alpha-token"), "{session_out}");
    assert!(
        session_out.contains("apply receipt: schema=cell-one.improvement-apply.v0"),
        "{session_out}"
    );
    assert!(
        session_out.contains("applied proposal specialty-seat:ag_news"),
        "{session_out}"
    );
    assert!(session_out.contains("kind=specialty-seat"), "{session_out}");
    assert!(session_out.contains("binding=ag_news"), "{session_out}");
    assert!(session_out.contains("standing=joinable: yes"), "{session_out}");
    assert!(session_out.contains("joinable=true"), "{session_out}");
    assert!(session_out.contains("require_plan=true"), "{session_out}");
    assert!(
        session_out.contains("refuse_without_plan=refuse:plan: apply-package requires --require-plan"),
        "{session_out}"
    );
    assert!(session_out.contains("auto_train=false"), "{session_out}");
    assert!(session_out.contains("train_invoked=false"), "{session_out}");
    assert!(!session_out.contains("READY_FOR_LIVE_TEST: yes"), "{session_out}");
    assert!(!session_out.contains("auto_train=true"), "{session_out}");
    assert!(!session_out.contains("train_invoked=true"), "{session_out}");
    let lab = PathBuf::from(apply["lab_estate"].as_str().unwrap());
    let status_run = bin()
        .args([
            "status",
            "--estate",
            lab.to_str().unwrap(),
            "--state-dir",
            apply_state.to_str().unwrap(),
            "--roots-base",
            out.join("apply/roots").to_str().unwrap(),
            "--plans-dir",
            out.join("apply/plans").to_str().unwrap(),
            "--packs-dir",
            repo_root().join("packs").to_str().unwrap(),
            "--policy",
            repo_root()
                .join("policy/cell-one.policy.v0.yaml")
                .to_str()
                .unwrap(),
            "--root",
            repo_root().to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_LOCAL_LIVE")
        .env_remove("XAI_API_KEY")
        .output()
        .unwrap();
    let status_out = String::from_utf8_lossy(&status_run.stdout).to_string();
    let status_err = String::from_utf8_lossy(&status_run.stderr).to_string();
    assert!(status_run.status.success(), "{status_out}\n{status_err}");
    assert!(status_out.starts_with("Cell One status\n"), "{status_out}");
    assert!(
        status_out.contains("apply receipt: schema=cell-one.improvement-apply.v0"),
        "{status_out}"
    );
    assert!(
        status_out.contains("applied proposal specialty-seat:ag_news"),
        "{status_out}"
    );
    assert!(status_out.contains("kind=specialty-seat"), "{status_out}");
    assert!(status_out.contains("binding=ag_news"), "{status_out}");
    assert!(status_out.contains("standing=joinable: yes"), "{status_out}");
    assert!(status_out.contains("joinable=true"), "{status_out}");
    assert!(status_out.contains("require_plan=true"), "{status_out}");
    assert!(
        status_out.contains("refuse_without_plan=refuse:plan: apply-package requires --require-plan"),
        "{status_out}"
    );
    assert!(status_out.contains("auto_train=false"), "{status_out}");
    assert!(status_out.contains("train_invoked=false"), "{status_out}");
    assert!(!status_out.contains("refuse:cite"), "{status_out}");
    assert!(!status_out.contains("/apply/improvement-apply.json"), "{status_out}");
    assert!(!status_out.contains("READY_FOR_LIVE_TEST: yes"), "{status_out}");
    assert!(!status_out.contains("auto_train=true"), "{status_out}");
    assert!(!status_out.contains("train_invoked=true"), "{status_out}");
    assert!(lab.ends_with("apply/lab-estate.yaml"), "{}", lab.display());
    let estate = estate_schema::load_estate(&lab).unwrap();
    assert!(estate.model_bindings.iter().any(|row| row.id == "ag_news"));
    assert!(estate.model_bindings.iter().any(|row| row.id == "local_slm"));
    assert!(!estate.model_bindings.iter().any(|row| row.id == "rust_idiom"));
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
        report["pack"]["cli_smoke"]["orchestrator"]["result"],
        "ag_news"
    );
    assert_eq!(report["specialty_real"]["stage"], "skipped");
    assert_eq!(report["specialty_real"]["reason"], "skipped:gguf-absent");
    assert_eq!(report["specialty_real"]["ok"], true);
    assert_eq!(report["specialty_real"]["ready_for_live_test"], false);
    assert_eq!(report["specialty_real"]["live_pass_recorded"], false);
    assert_eq!(report["specialty_real"]["seats"]["ag_news"]["status"], "skipped");
    assert_eq!(
        report["specialty_real"]["seats"]["ag_news"]["reason"],
        "skipped:gguf-absent"
    );
    assert_eq!(
        report["specialty_real"]["seats"]["rust_idiom"]["status"],
        "skipped"
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
    assert_eq!(report["pack"]["cli_smoke"]["member"]["session_bound"], true);
    assert_eq!(report["pack"]["cli_smoke"]["session_id"], "sess-cohesion01");
    assert_eq!(report["pack"]["cli_smoke"]["orchestrator"]["context"], "applied");
    assert_eq!(report["pack"]["cli_smoke"]["orchestrator"]["saw_prior"], true);
    assert_eq!(report["pack"]["cli_smoke"]["hops"].as_array().unwrap().len(), 2);
    assert_eq!(report["pack"]["cli_smoke"]["hops"][0]["context"], "none");
    assert_eq!(report["pack"]["cli_smoke"]["hops"][1]["context"], "applied");
    assert_eq!(report["pack"]["cli_smoke"]["hops"][1]["saw_prior"], true);
    assert_eq!(report["pack"]["cli_smoke"]["isolation"]["leaked"], false);
    assert_eq!(report["pack"]["cli_smoke"]["session_show"]["cites_hop1"], true);
    assert_eq!(report["pack"]["cli_smoke"]["session_show"]["cites_apply"], true);
    assert_eq!(report["pack"]["crew_session"]["session_id"], "sess-cohesion01");
    let composed: Value = serde_json::from_str(
        &fs::read_to_string(out.join("crew-session-prove.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(composed["schema"], "cell-one.crew-session-prove.v0");
    assert_eq!(composed["ok"], true);
    assert_eq!(composed["ready_for_live_test"], false);
    assert_eq!(composed["composed_by"], "cohesion-prove");
    assert_eq!(composed["pack"]["cli_smoke"]["session_id"], "sess-cohesion01");
    assert_eq!(composed["pack"]["cli_smoke"]["session_show"]["cites_apply"], true);
    let composed_cite = composed["pack"]["cli_smoke"]["session_show"]["apply_cite"]
        .as_str()
        .unwrap();
    assert!(composed_cite.contains("schema=cell-one.improvement-apply.v0"), "{composed_cite}");
    assert!(composed_cite.contains("applied proposal specialty-seat:ag_news"), "{composed_cite}");
    assert!(composed_cite.contains("require_plan=true"), "{composed_cite}");
    assert!(composed_cite.contains("auto_train=false"), "{composed_cite}");
    assert!(composed_cite.contains("train_invoked=false"), "{composed_cite}");
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
    let journal = fs::read_to_string(
        out.join("cli-smoke/crew/decisions/receipts.jsonl"),
    )
    .unwrap();
    let rows: Vec<Value> = journal
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 3, "{journal}");
    assert_eq!(rows[0]["outcome"], "allow");
    assert_eq!(rows[0]["surface"], "complete");
    assert_eq!(rows[0]["pack_id"], "research-crew");
    assert_eq!(rows[0]["handoff_from"], "horizon");
    assert_eq!(rows[0]["session_id"], "sess-cohesion01");
    assert_eq!(rows[0]["session_turns"], 1);
    assert_eq!(rows[0]["session_context"], false);
    assert_eq!(rows[1]["session_id"], "sess-cohesion01");
    assert_eq!(rows[1]["session_turns"], 2);
    assert_eq!(rows[1]["session_context"], true);
    assert_eq!(rows[2]["session_id"], "sess-cohesioniso");
    assert_eq!(rows[2]["session_context"], false);
    let session = fs::read_to_string(
        out.join("cli-smoke/crew/pack-sessions/research-crew/sess-cohesion01.json"),
    )
    .unwrap();
    assert!(session.contains("unique-hop-alpha-token"), "{session}");
    assert!(session.contains("\"schema\": \"cell-one.pack-session.v0\""), "{session}");
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
        let _ = fs::copy(package_path, art.join("cohesion-improvement-package.json"));
        let _ = fs::copy(
            out.join("improvement/improvement-package.yaml"),
            art.join("cohesion-improvement-package.yaml"),
        );
        let _ = fs::copy(receipt_path, art.join("cohesion-improvement-apply.json"));
    }
    let _ = fs::remove_dir_all(&out);
    let _ = fs::remove_dir_all(&sentinel);
    let _ = fs::remove_dir_all(&no_gguf);
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
    assert!(day.contains("crew-session-prove"), "{day}");
    assert!(day.contains("sess-cohesion01"), "{day}");
    assert!(day.contains("context=applied"), "{day}");
    assert!(log.contains("cell-one.cohesion-prove.v0"), "{log}");
    assert!(log.contains("cell-one.crew-session-prove.v0"), "{log}");
    assert!(day.contains("those three cites"), "{day}");
    assert!(lang.contains("those three cites"), "{lang}");
    assert!(log.contains("those three cites") || log.contains("status cite"), "{log}");
    assert!(log.contains("unique-hop-alpha-token"), "{log}");
    for (name, text) in [
        ("NORTH-STAR", north.as_str()),
        ("OPERATOR-DAY", day.as_str()),
        ("UBIQUITOUS_LANGUAGE", lang.as_str()),
        ("CHANGELOG", log.as_str()),
    ] {
        assert!(
            text.contains("skipped:gguf-absent"),
            "{name} missing specialty-real skip"
        );
        assert!(
            text.contains("named specialty") || text.contains("named seats"),
            "{name} missing named specialty seat cite"
        );
        assert!(
            text.contains("crew-session-prove") || text.contains("same session_id"),
            "{name} missing crew-session multi-hop cite"
        );
        assert!(
            text.contains("context=applied"),
            "{name} missing context=applied"
        );
        assert!(
            text.contains("improvement-export-prove") || text.contains("export-package"),
            "{name} missing improvement export stitch"
        );
        assert!(
            text.contains("auto_train"),
            "{name} missing auto_train lock"
        );
        assert!(
            text.contains("does not re-run host-validate"),
            "{name} missing cohesion apply reuse"
        );
        assert!(
            text.contains("cell-one.improvement-apply.v0"),
            "{name} missing apply receipt schema"
        );
        assert!(
            text.contains("refuse:plan"),
            "{name} missing refuse:plan"
        );
        assert!(
            text.contains("estate decisions report"),
            "{name} missing decisions report apply cite"
        );
        assert!(
            text.contains("estate pack session show"),
            "{name} missing pack session show apply cite"
        );
        assert!(
            text.contains("estate status"),
            "{name} missing estate status apply cite"
        );
        assert!(
            !text.contains("can compose this later"),
            "{name} still says the apply stitch is later"
        );
        assert!(
            !text.contains("stays export-only"),
            "{name} still says cohesion is export-only"
        );
    }
    assert!(log.contains("pack_mcp::run_command_with_timeout"), "{log}");
    assert!(!north.contains("READY_FOR_LIVE_TEST: yes"), "{north}");
}

#[test]
fn cohesion_prove_reuses_pack_mcp_timeout_helper() {
    let src = fs::read_to_string(
        repo_root().join("crates/estate-control/src/crew_session_prove.rs"),
    )
    .unwrap();
    assert!(
        src.contains("pack_mcp::run_command_with_timeout"),
        "crew-session smoke must reuse pack_mcp::run_command_with_timeout"
    );
    assert!(
        !src.contains("child.try_wait"),
        "crew-session smoke must not use a local try_wait drain loop"
    );
    assert!(
        !src.contains("Stdio::piped"),
        "crew-session smoke must not open its own piped stdio"
    );
    let cohesion = fs::read_to_string(
        repo_root().join("crates/estate-control/src/cohesion_prove.rs"),
    )
    .unwrap();
    assert!(
        cohesion.contains("crew_session_prove::run_pack_stage"),
        "cohesion-prove must compose crew-session-prove"
    );
    assert!(
        cohesion.contains("improvement_export::run_export_stage"),
        "cohesion-prove must compose improvement-export"
    );
    assert!(
        cohesion.contains("improvement_apply::run_apply_stage"),
        "cohesion-prove must compose the gated apply stage"
    );
    assert!(
        cohesion.contains("cmd_decisions_report"),
        "cohesion-prove must compose estate decisions report after apply"
    );
    assert!(
        cohesion.contains("cmd_pack_session_show"),
        "cohesion-prove must compose estate pack session show after apply"
    );
    assert!(
        cohesion.contains("cmd_status"),
        "cohesion-prove must compose estate status after apply"
    );
    assert!(
        cohesion.contains("install_apply_receipt_for_report"),
        "cohesion-prove must copy the apply receipt beside the crew journal"
    );
    assert!(
        !cohesion.contains("../apply/"),
        "cohesion pack session cite must not walk a sibling apply dir"
    );
    assert!(
        !cohesion.contains("cmd_routine_digest") && !cohesion.contains("cmd_routine_tick"),
        "cohesion-prove must not newly invoke digest or tick --report cites"
    );
    assert!(
        !cohesion.contains("cmd_runner_status"),
        "cohesion-prove must not newly invoke runner-status cites"
    );
    assert!(
        !cohesion.contains("cmd_decisions_improvement_apply_prove"),
        "cohesion-prove must not shrink into the standalone apply prove"
    );
    assert!(
        !cohesion.contains("cmd_decisions_host_validate_prove"),
        "cohesion apply must not re-run host-validate"
    );
}

#[test]
fn cohesion_prove_binds_planted_specialty_ggufs_as_named_seats() {
    let before = cksum_locked();
    assert!(
        before.starts_with("43770130 3391"),
        "examples/estate.yaml cksum drifted: {before}"
    );
    let locked_bytes = fs::read(repo_root().join("examples/estate.yaml")).unwrap();
    let fixture = repo_root().join("examples/fixtures/agent-pack-handoff.yaml");
    let fixture_bytes = fs::read(&fixture).unwrap();
    let planted = scratch("planted-gguf");
    let ag = planted.join("ag_news").join("specialist.Q4_K_M.gguf");
    let rust = planted.join("rust_idiom").join("specialist.Q4_K_M.gguf");
    fs::create_dir_all(ag.parent().unwrap()).unwrap();
    fs::create_dir_all(rust.parent().unwrap()).unwrap();
    fs::write(&ag, b"GGUF").unwrap();
    fs::write(&rust, b"GGUF").unwrap();
    let out = scratch("prove-real");
    let sentinel = scratch("sentinel-home-real");
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
        .env("CELL_SPECIALTY_AG_NEWS_GGUF", &ag)
        .env("CELL_SPECIALTY_RUST_IDIOM_GGUF", &rust)
        .env_remove("CELL_SPECIALTY_GGUF_ROOT")
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
    assert!(stdout.contains("cohesion-prove: specialty-real"), "{stdout}");
    assert!(
        stdout.contains("specialty-real ag_news: bound seat=specialist-agnews-3000 complete result=ag_news capability=ag_news"),
        "{stdout}"
    );
    assert!(
        stdout.contains("specialty-real rust_idiom: bound seat=specialist-rustidiom-3000 complete result=rust_idiom capability=rust_idiom"),
        "{stdout}"
    );
    assert!(stdout.contains("specialty-real: bound ag_news,rust_idiom"), "{stdout}");
    assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"), "{stdout}");
    let tail = stdout.trim_end();
    assert!(
        tail.ends_with("cohesion-prove: ok\nREADY_FOR_LIVE_TEST: no"),
        "{stdout}"
    );
    let report: Value =
        serde_json::from_str(&fs::read_to_string(out.join("cohesion-prove.json")).unwrap()).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["ready_for_live_test"], false);
    assert_eq!(report["live_pass_recorded"], false);
    assert_eq!(report["live_sync"], false);
    assert_eq!(report["auto_train"], false);
    assert_eq!(report["train_invoked"], false);
    assert_eq!(report["improvement"]["auto_train"], false);
    assert_eq!(report["improvement"]["train_invoked"], false);
    assert!(
        report["improvement"]["proposal_count"].as_u64().unwrap() >= 4
    );
    assert_eq!(report["apply"]["applied_proposal_id"], "specialty-seat:ag_news");
    assert_eq!(report["apply"]["applied_proposal_kind"], "specialty-seat");
    assert_eq!(report["apply"]["binding_id"], "ag_news");
    assert_eq!(report["apply"]["joinable"], true);
    assert_eq!(report["apply"]["require_plan"], true);
    assert_eq!(report["apply"]["auto_train"], false);
    assert_eq!(report["apply"]["train_invoked"], false);
    assert_eq!(report["apply"]["local_slm"], true);
    assert_eq!(report["apply"]["dataset_proposals"], "proposal-only");
    assert_eq!(
        report["apply"]["refuse_without_plan"],
        "refuse:plan: apply-package requires --require-plan"
    );
    assert_eq!(report["decisions"]["cites_apply"], true);
    assert_eq!(report["decisions"]["applied_proposal_id"], "specialty-seat:ag_news");
    assert_eq!(report["decisions"]["require_plan"], true);
    assert_eq!(report["decisions"]["auto_train"], false);
    assert_eq!(report["decisions"]["train_invoked"], false);
    assert_eq!(report["pack_session"]["cites_apply"], true);
    assert_eq!(report["pack_session"]["session_id"], "sess-cohesion01");
    assert_eq!(report["pack_session"]["applied_proposal_id"], "specialty-seat:ag_news");
    assert_eq!(report["pack_session"]["require_plan"], true);
    assert_eq!(report["pack_session"]["auto_train"], false);
    assert_eq!(report["pack_session"]["train_invoked"], false);
    assert_eq!(report["status"]["cites_apply"], true);
    assert_eq!(report["status"]["applied_proposal_id"], "specialty-seat:ag_news");
    assert_eq!(report["status"]["require_plan"], true);
    assert_eq!(report["status"]["auto_train"], false);
    assert_eq!(report["status"]["train_invoked"], false);
    assert!(
        report["status"]["state_dir"].as_str().unwrap().ends_with("apply/state"),
        "{}",
        report["status"]["state_dir"]
    );
    assert_eq!(report["pack"]["cli_smoke"]["session_show"]["cites_apply"], true);
    assert!(stdout.contains("cohesion-prove: improvement-apply"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: decisions report"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: pack session show"), "{stdout}");
    assert!(stdout.contains("cohesion-prove: status"), "{stdout}");
    assert!(
        stdout.contains("apply receipt: schema=cell-one.improvement-apply.v0"),
        "{stdout}"
    );
    assert!(stdout.contains("cohesion-prove: refuse-without-plan"), "{stdout}");
    assert_eq!(report["specialty_real"]["stage"], "bound");
    assert_eq!(report["specialty_real"]["ok"], true);
    assert_eq!(report["specialty_real"]["ready_for_live_test"], false);
    assert_eq!(report["specialty_real"]["seats"]["ag_news"]["status"], "bound");
    assert_eq!(
        report["specialty_real"]["seats"]["ag_news"]["seat_model"],
        "specialist-agnews-3000"
    );
    assert_eq!(
        report["specialty_real"]["seats"]["ag_news"]["complete"]["result"],
        "ag_news"
    );
    assert_eq!(
        report["specialty_real"]["seats"]["ag_news"]["complete"]["capability"],
        "ag_news"
    );
    assert_eq!(
        report["specialty_real"]["seats"]["rust_idiom"]["status"],
        "bound"
    );
    assert_eq!(
        report["specialty_real"]["seats"]["rust_idiom"]["complete"]["result"],
        "rust_idiom"
    );
    assert_eq!(
        report["pack"]["cli_smoke"]["orchestrator"]["result"],
        "ag_news"
    );
    assert_ne!(
        report["specialty_real"]["seats"]["ag_news"]["complete"]["result"],
        "local_slm"
    );
    assert_eq!(fs::read(repo_root().join("examples/estate.yaml")).unwrap(), locked_bytes);
    assert_eq!(fs::read(&fixture).unwrap(), fixture_bytes);
    assert_eq!(cksum_locked(), before);
    let art = PathBuf::from("/opt/cursor/artifacts");
    if fs::create_dir_all(&art).is_ok() {
        let _ = fs::write(
            art.join("cohesion-prove-specialty-real.json"),
            serde_json::to_string_pretty(&report).unwrap() + "\n",
        );
        let _ = fs::write(art.join("cohesion-prove-specialty-real.log"), &stdout);
    }
    let _ = fs::remove_dir_all(&out);
    let _ = fs::remove_dir_all(&sentinel);
    let _ = fs::remove_dir_all(&planted);
}
