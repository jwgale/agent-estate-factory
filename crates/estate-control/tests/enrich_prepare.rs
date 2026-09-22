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
    assert!(ollama_text.contains("estate_rewritten=false"), "{ollama_text}");
    let modelfile = std::fs::read_to_string(ollama_out.join("Modelfile")).unwrap();
    assert!(modelfile.contains("FROM local_slm"), "{modelfile}");
    let steps = std::fs::read_to_string(ollama_out.join("PREPARE.md")).unwrap();
    assert!(
        steps.contains("ollama create cell-enrich-overnight-traces -f Modelfile"),
        "{steps}"
    );

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
    assert_eq!(estate_bytes(), before, "prepare rewrote examples/estate.yaml");

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
    assert!(missing_text.contains("refuse:missing-pack"), "{missing_text}");
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
    assert!(
        sku_text.to_ascii_lowercase().contains("sku"),
        "{sku_text}"
    );
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
    let dispatch = std::fs::read_to_string(root.join("crates/estate-control/src/dispatch.rs")).unwrap();
    assert!(!dispatch.contains("ollama-modelfile"));
    assert!(!dispatch.contains("external-manifest"));
    let floor = std::fs::read_to_string(root.join("crates/floor-supervisor/src/lib.rs")).unwrap();
    assert!(!floor.contains("TrainEnrichDriver"));
}
