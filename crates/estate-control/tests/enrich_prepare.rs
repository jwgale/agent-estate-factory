//! Train/enrich prepare. Fixtures only. Does not train or rewrite the estate.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn estate_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn tmp(name: &str) -> PathBuf {
    let path = repo_root().join(format!(
        "target/test-enrich-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn text(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn fixture(rel: &str) -> String {
    repo_root().join(rel).display().to_string()
}

fn estate_bytes() -> String {
    std::fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap()
}

#[test]
fn help_enrich_and_train_name_the_seam() {
    for topic in ["enrich", "train"] {
        let out = estate_bin().args(["help", topic]).output().unwrap();
        let body = text(&out);
        assert!(out.status.success(), "{topic}: {body}");
        assert!(body.contains("TrainEnrichDriver"), "{body}");
        assert!(body.contains("ollama-modelfile"), "{body}");
        assert!(body.contains("external-manifest"), "{body}");
        assert!(body.contains("make enrich-prepare"), "{body}");
        assert!(body.contains("docs/TRAIN-ENRICH.md"), "{body}");
        assert!(body.contains("--all-drivers"), "{body}");
        assert!(body.contains("estate enrich list"), "{body}");
        assert!(body.contains("import-prepared"), "{body}");
        assert!(body.contains("apply-proposal"), "{body}");
        assert!(body.contains("--verify-local-tag"), "{body}");
        assert!(!body.contains("READY_FOR_LIVE_TEST: yes"), "{body}");
    }
    let index = estate_bin().args(["help"]).output().unwrap();
    let index_text = text(&index);
    assert!(index_text.contains("estate help enrich"), "{index_text}");

    let drivers = estate_bin().args(["enrich", "drivers"]).output().unwrap();
    let listed = text(&drivers);
    assert!(drivers.status.success(), "{listed}");
    assert!(listed.contains("ollama-modelfile"), "{listed}");
    assert!(listed.contains("external-manifest"), "{listed}");
    assert!(listed.contains("live=false"), "{listed}");
}

#[test]
fn prepare_both_drivers_and_refuses_without_writing() {
    let root = tmp("cli");
    let estate = fixture("examples/estate.yaml");
    let sacred = fixture("policy/sacred.yaml");
    let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
    let before = estate_bytes();

    let ollama_out = root.join("ollama");
    let ollama = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            &pack,
            "--driver",
            "ollama-modelfile",
            "--out",
            &ollama_out.display().to_string(),
        ])
        .output()
        .unwrap();
    let ollama_text = text(&ollama);
    assert!(ollama.status.success(), "{ollama_text}");
    assert!(ollama_text.contains("promoted=false"), "{ollama_text}");
    assert!(
        ollama_text.contains("estate_rewritten=false"),
        "{ollama_text}"
    );
    let modelfile = std::fs::read_to_string(ollama_out.join("Modelfile")).unwrap();
    assert!(modelfile.contains("FROM local_slm"), "{modelfile}");
    let steps = std::fs::read_to_string(ollama_out.join("PREPARE.md")).unwrap();
    assert!(
        steps.contains("ollama create cell-enrich-overnight-traces -f Modelfile"),
        "{steps}"
    );
    let next = std::fs::read_to_string(ollama_out.join("NEXT.md")).unwrap();
    assert!(
        next.contains(&format!(
            "ollama create cell-enrich-overnight-traces -f {}",
            ollama_out.join("Modelfile").display()
        )),
        "{next}"
    );
    assert!(ollama_text.contains("import-prepared"), "{ollama_text}");
    assert!(ollama_text.contains("prepared=1"), "{ollama_text}");

    let manifest_out = root.join("manifest");
    let manifest = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            &pack,
            "--driver",
            "external-manifest",
            "--job",
            "train",
            "--out",
            &manifest_out.display().to_string(),
        ])
        .output()
        .unwrap();
    let manifest_text = text(&manifest);
    assert!(manifest.status.success(), "{manifest_text}");
    assert!(manifest_text.contains("job=train"), "{manifest_text}");
    assert!(manifest_out.join("manifest.json").is_file());
    assert!(manifest_out.join("manifest.yaml").is_file());
    let body = std::fs::read_to_string(manifest_out.join("manifest.json")).unwrap();
    assert!(body.contains("\"vendor\": null"), "{body}");
    assert!(!body.to_ascii_lowercase().contains("ollama"), "{body}");
    assert_eq!(
        estate_bytes(),
        before,
        "prepare rewrote examples/estate.yaml"
    );

    let missing_out = root.join("missing");
    let missing = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            "no-such-pack",
            "--packs-dir",
            &root.join("empty-packs").display().to_string(),
            "--out",
            &missing_out.display().to_string(),
        ])
        .output()
        .unwrap();
    let missing_text = text(&missing);
    assert!(!missing.status.success(), "{missing_text}");
    assert!(
        missing_text.contains("refuse:missing-pack"),
        "{missing_text}"
    );
    assert!(!missing_out.exists());

    let sacred_pack = root.join("sacred.pack.json");
    let mut sacred_doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&pack).unwrap()).unwrap();
    sacred_doc["note"] = serde_json::Value::String("please mention cyera".into());
    std::fs::write(
        &sacred_pack,
        serde_json::to_string_pretty(&sacred_doc).unwrap(),
    )
    .unwrap();
    let sacred_out = root.join("sacred-out");
    let sacred_run = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            &sacred_pack.display().to_string(),
            "--out",
            &sacred_out.display().to_string(),
        ])
        .output()
        .unwrap();
    let sacred_text = text(&sacred_run);
    assert!(!sacred_run.status.success(), "{sacred_text}");
    assert!(sacred_text.contains("refuse:sacred"), "{sacred_text}");
    assert!(!sacred_out.exists());

    let sku_pack = root.join("sku.pack.json");
    let mut sku_doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&pack).unwrap()).unwrap();
    sku_doc["model_hint"] = serde_json::Value::String("rtx-5090".into());
    std::fs::write(&sku_pack, serde_json::to_string_pretty(&sku_doc).unwrap()).unwrap();
    let sku_out = root.join("sku-out");
    let sku_run = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            &sku_pack.display().to_string(),
            "--out",
            &sku_out.display().to_string(),
        ])
        .output()
        .unwrap();
    let sku_text = text(&sku_run);
    assert!(!sku_run.status.success(), "{sku_text}");
    assert!(sku_text.to_ascii_lowercase().contains("sku"), "{sku_text}");
    assert!(!sku_out.exists());

    let curator = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            &pack,
            "--curator",
            "ada",
            "--out",
            &root.join("curator-out").display().to_string(),
        ])
        .output()
        .unwrap();
    let curator_text = text(&curator);
    assert!(!curator.status.success(), "{curator_text}");
    assert!(curator_text.contains("refuse:curator"), "{curator_text}");
    assert!(!root.join("curator-out").exists());

    let local_estate = root.join("local-only.yaml");
    std::fs::write(
        &local_estate,
        "version: 0\nname: local-only\ndefault_effect: deny\nagents:\n  - id: horizon\n    display_name: Horizon\n    lane: horizon\n    desktop: horizon-desktop\nlanes:\n  - id: horizon\n    root_path: lanes/horizon\n    owner_agent_id: horizon\nmodel_bindings:\n  - id: local_slm\n    class: local\n    driver: ollama\n    wired: true\nenrich_packs:\n  curator: jason\n  policy: manual\n",
    )
    .unwrap();
    let frontier_pack = root.join("frontier.pack.json");
    let mut frontier_doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&pack).unwrap()).unwrap();
    frontier_doc["source_drivers"] = serde_json::json!(["frontier"]);
    frontier_doc["path_counts"]["frontier"] = serde_json::json!(1);
    std::fs::write(
        &frontier_pack,
        serde_json::to_string_pretty(&frontier_doc).unwrap(),
    )
    .unwrap();
    let frontier_out = root.join("frontier-out");
    let frontier = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &local_estate.display().to_string(),
            "--pack",
            &frontier_pack.display().to_string(),
            "--out",
            &frontier_out.display().to_string(),
        ])
        .output()
        .unwrap();
    let frontier_text = text(&frontier);
    assert!(!frontier.status.success(), "{frontier_text}");
    assert!(
        frontier_text.contains("refuse:frontier-invent"),
        "{frontier_text}"
    );
    assert!(!frontier_out.exists());
    assert_eq!(estate_bytes(), before);
}

#[test]
fn enrich_prepare_stays_off_smoke_and_dispatch_does_not_match_drivers() {
    let root = repo_root();
    let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
    assert!(
        makefile.contains("enrich-prepare:"),
        "Makefile missing enrich-prepare"
    );
    assert!(makefile.contains("scripts/enrich-prepare.sh"));
    let script = std::fs::read_to_string(root.join("scripts/enrich-prepare.sh")).unwrap();
    assert!(script.contains("ollama-modelfile"), "{script}");
    assert!(script.contains("external-manifest"), "{script}");
    assert!(script.contains("--all-drivers"), "{script}");
    assert!(script.contains("import-prepared"), "{script}");
    assert!(script.contains("Do not add to make smoke or GitHub Actions"));
    for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("enrich-prepare"),
            "{rel} must not run enrich-prepare"
        );
    }
    let dispatch =
        std::fs::read_to_string(root.join("crates/estate-control/src/dispatch.rs")).unwrap();
    assert!(!dispatch.contains("ollama-modelfile"));
    assert!(!dispatch.contains("external-manifest"));
    let floor = std::fs::read_to_string(root.join("crates/floor-supervisor/src/lib.rs")).unwrap();
    assert!(!floor.contains("TrainEnrichDriver"));
}

#[test]
fn prepare_all_list_and_import_prepared_stay_off_the_estate() {
    let root = tmp("loop");
    let estate = fixture("examples/estate.yaml");
    let sacred = fixture("policy/sacred.yaml");
    let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
    let before = estate_bytes();
    let state = root.join("state");

    let both = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            &pack,
            "--all-drivers",
            "--driver",
            "ollama-modelfile",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    let both_text = text(&both);
    assert!(!both.status.success(), "{both_text}");
    assert!(both_text.contains("refuse:driver"), "{both_text}");
    assert!(!state.join("enrich").exists());

    let prepared = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            &pack,
            "--all-drivers",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    let prepared_text = text(&prepared);
    assert!(prepared.status.success(), "{prepared_text}");
    assert!(prepared_text.contains("prepared=2"), "{prepared_text}");
    assert!(
        prepared_text.contains("driver=ollama-modelfile"),
        "{prepared_text}"
    );
    assert!(
        prepared_text.contains("driver=external-manifest"),
        "{prepared_text}"
    );
    let ollama = state.join("enrich/overnight-traces/ollama-modelfile");
    let manifest = state.join("enrich/overnight-traces/external-manifest");
    assert!(ollama.join("Modelfile").is_file());
    assert!(ollama.join("NEXT.md").is_file());
    assert!(manifest.join("manifest.json").is_file());
    let next = std::fs::read_to_string(manifest.join("NEXT.md")).unwrap();
    assert!(!next.contains("ollama create"), "{next}");

    let listed = estate_bin()
        .args([
            "enrich",
            "list",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    let listed_text = text(&listed);
    assert!(listed.status.success(), "{listed_text}");
    assert!(listed_text.contains("count=2"), "{listed_text}");
    assert!(
        listed_text.contains("tag=cell-enrich-overnight-traces"),
        "{listed_text}"
    );
    assert!(listed_text.contains("promoted=false"), "{listed_text}");

    let missing = estate_bin()
        .args([
            "enrich",
            "list",
            "--state-dir",
            &root.join("absent").display().to_string(),
        ])
        .output()
        .unwrap();
    let missing_text = text(&missing);
    assert!(!missing.status.success(), "{missing_text}");
    assert!(
        missing_text.contains("refuse:enrich-index"),
        "{missing_text}"
    );
    assert!(!root.join("absent").exists());

    let tag = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-prepared",
            "--estate",
            &estate,
            "--prepared",
            &ollama.display().to_string(),
            "--tag",
            "other-tag",
            "--path",
            &ollama.join("Modelfile").display().to_string(),
        ])
        .output()
        .unwrap();
    let tag_text = text(&tag);
    assert!(!tag.status.success(), "{tag_text}");
    assert!(tag_text.contains("refuse:tag"), "{tag_text}");
    assert!(!ollama.join("binding-proposal.json").exists());

    let imported = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-prepared",
            "--estate",
            &estate,
            "--prepared",
            &ollama.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--path",
            &ollama.join("Modelfile").display().to_string(),
        ])
        .output()
        .unwrap();
    let imported_text = text(&imported);
    assert!(imported.status.success(), "{imported_text}");
    assert!(
        imported_text.contains("binding=local_slm"),
        "{imported_text}"
    );
    assert!(
        imported_text.contains("auto_apply=false"),
        "{imported_text}"
    );
    assert!(imported_text.contains("did not apply"), "{imported_text}");
    let proposal: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(ollama.join("binding-proposal.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(proposal["schema"], "cell-one.enrich-binding-proposal.v0");
    assert_eq!(proposal["auto_apply"], false);
    assert_eq!(proposal["promoted"], false);
    assert_eq!(proposal["estate_rewritten"], false);
    assert_eq!(
        proposal["proposed_binding"]["params"]["model"],
        "cell-enrich-overnight-traces"
    );
    assert_eq!(proposal["proposed_binding"]["id"], "local_slm");
    assert!(!ollama.join("catalog.json").exists());
    assert_eq!(estate_bytes(), before, "import-prepared rewrote the estate");
}

#[test]
fn apply_proposal_then_plan_and_require_plan_writes_only_the_lab_estate() {
    let root = tmp("apply-proposal");
    let sacred = fixture("policy/sacred.yaml");
    let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
    let example = repo_root().join("examples/estate.yaml");
    let example_before = std::fs::read(&example).unwrap();
    let lab = root.join("estate.yaml");
    std::fs::copy(&example, &lab).unwrap();
    let lab_before = std::fs::read(&lab).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");

    let prepared = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &lab.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "ollama-modelfile",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(prepared.status.success(), "{}", text(&prepared));
    let dir = state.join("enrich/overnight-traces/ollama-modelfile");

    let imported = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-prepared",
            "--estate",
            &lab.display().to_string(),
            "--prepared",
            &dir.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--path",
            &dir.join("Modelfile").display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(imported.status.success(), "{}", text(&imported));
    assert!(text(&imported).contains("apply-proposal"), "{}", text(&imported));

    let verify = estate_bin()
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_RENTED_ENDPOINT")
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "apply-proposal",
            "--estate",
            &lab.display().to_string(),
            "--prepared",
            &dir.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--state-dir",
            &root.join("verify-state").display().to_string(),
            "--verify-local-tag",
        ])
        .output()
        .unwrap();
    let verify_text = text(&verify);
    assert!(!verify.status.success(), "{verify_text}");
    assert!(verify_text.contains("refuse:local-tag"), "{verify_text}");
    assert!(!root.join("verify-state").join("enrich-stage").exists());

    let wrong = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "apply-proposal",
            "--estate",
            &lab.display().to_string(),
            "--prepared",
            &dir.display().to_string(),
            "--tag",
            "other-tag",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    let wrong_text = text(&wrong);
    assert!(!wrong.status.success(), "{wrong_text}");
    assert!(wrong_text.contains("refuse:tag"), "{wrong_text}");
    assert!(!state.join("enrich-stage").exists());

    let staged = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "apply-proposal",
            "--estate",
            &lab.display().to_string(),
            "--prepared",
            &dir.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let staged_text = text(&staged);
    assert!(staged.status.success(), "{staged_text}");
    assert!(staged_text.contains("apply-proposal did not apply"), "{staged_text}");
    assert!(staged_text.contains("auto_apply=false"), "{staged_text}");
    assert!(staged_text.contains("--require-plan"), "{staged_text}");
    assert_eq!(std::fs::read(&lab).unwrap(), lab_before);
    let staged_estate = state.join("enrich-stage/staged-estate.yaml");
    assert!(staged_estate.is_file());
    let again = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "apply-proposal",
            "--estate",
            &lab.display().to_string(),
            "--prepared",
            &dir.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    let again_text = text(&again);
    assert!(again.status.success(), "{again_text}");
    assert!(again_text.contains("no-op:"), "{again_text}");

    let status = estate_bin()
        .args([
            "status",
            "--estate",
            &lab.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--root",
            &repo_root().display().to_string(),
        ])
        .output()
        .unwrap();
    let status_text = text(&status);
    assert!(status.status.success(), "{status_text}");
    assert!(
        status_text.contains("enrich_binding: pending"),
        "{status_text}"
    );
    assert!(
        status_text.contains("enrich_stage: applied=false"),
        "{status_text}"
    );

    let doctor = estate_bin()
        .args([
            "doctor",
            "--root",
            &repo_root().display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    let doctor_text = text(&doctor);
    assert!(doctor.status.success(), "{doctor_text}");
    assert!(
        doctor_text.contains("source estate not written"),
        "{doctor_text}"
    );
    assert!(
        doctor_text.contains("binding proposal"),
        "{doctor_text}"
    );

    let plan = estate_bin()
        .args([
            "plan",
            "--estate",
            &staged_estate.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let plan_text = text(&plan);
    assert!(plan.status.success(), "{plan_text}");
    assert!(plan_text.contains("local_slm"), "{plan_text}");

    let dry = estate_bin()
        .args([
            "apply",
            "--dry-run",
            "--require-plan",
            "--estate",
            &staged_estate.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(dry.status.success(), "{}", text(&dry));
    assert_eq!(std::fs::read(&lab).unwrap(), lab_before);

    let ungated = estate_bin()
        .args([
            "apply",
            "--estate",
            &staged_estate.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
        ])
        .output()
        .unwrap();
    let ungated_text = text(&ungated);
    assert!(ungated.status.success(), "{ungated_text}");
    assert!(
        ungated_text.contains("enrich stage held"),
        "{ungated_text}"
    );
    assert_eq!(std::fs::read(&lab).unwrap(), lab_before);

    let applied = estate_bin()
        .args([
            "apply",
            "--require-plan",
            "--estate",
            &staged_estate.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
        ])
        .output()
        .unwrap();
    let applied_text = text(&applied);
    assert!(applied.status.success(), "{applied_text}");
    assert!(
        applied_text.contains("enrich stage wrote"),
        "{applied_text}"
    );
    let lab_after = std::fs::read_to_string(&lab).unwrap();
    assert!(
        lab_after.contains("cell-enrich-overnight-traces"),
        "{lab_after}"
    );
    assert_eq!(std::fs::read(&example).unwrap(), example_before);

    let joined = estate_bin()
        .args([
            "status",
            "--estate",
            &lab.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--root",
            &repo_root().display().to_string(),
        ])
        .output()
        .unwrap();
    let joined_text = text(&joined);
    assert!(joined.status.success(), "{joined_text}");
    assert!(
        joined_text.contains("enrich_binding: local_slm model=cell-enrich-overnight-traces"),
        "{joined_text}"
    );
    assert!(
        joined_text.contains("enrich_stage: applied=true"),
        "{joined_text}"
    );
}
