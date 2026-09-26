//! Bad-host-class on every reader of placement-actual, plus validate fixtures
//! after the convey-sync fix. Isolated cell. No new verb. Cloud never spawned.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn estate_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn fixture(rel: &str) -> String {
    repo_root().join(rel).display().to_string()
}

fn text(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn tamper_box_host_class(path: &std::path::Path, host_class: &str) -> String {
    let raw = std::fs::read_to_string(path).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let leases = v["leases"].as_array_mut().expect("leases");
    let box_lease = leases
        .iter_mut()
        .find(|l| l["placement_id"] == "cell-one-box")
        .expect("cell-one-box");
    box_lease["host_class"] = serde_json::Value::String(host_class.into());
    box_lease["spawned"] = serde_json::Value::Bool(false);
    let next = serde_json::to_string_pretty(&v).unwrap();
    std::fs::write(path, &next).unwrap();
    next
}

#[test]
fn validate_multi_host_and_mixed_still_green() {
    for name in [
        "examples/hosts/multi-host.yaml",
        "examples/hosts/rtx-consumer.yaml",
        "examples/hosts/apple-silicon.yaml",
        "examples/hosts/nvidia-rental.yaml",
        "examples/fixtures/mixed-frontier-local.yaml",
        "examples/fixtures/dual-layer-demo.yaml",
        "examples/estate.yaml",
    ] {
        let out = estate_bin()
            .args(["validate", "--estate", &fixture(name)])
            .output()
            .unwrap();
        assert!(out.status.success(), "{name} {}", text(&out));
    }
    for name in [
        "examples/fixtures/refuse-host-class.yaml",
        "examples/invalid/host-class-bad.yaml",
        "examples/fixtures/refuse-placement-sku.yaml",
        "examples/invalid/placement-sku.yaml",
    ] {
        let out = estate_bin()
            .args(["validate", "--estate", &fixture(name)])
            .output()
            .unwrap();
        assert!(!out.status.success(), "{name} should refuse {}", text(&out));
    }
}

#[test]
fn probes_and_hop_ids_refuse_sku_like_sync() {
    let probes = estate_bin().args(["probes"]).output().unwrap();
    let probes_text = text(&probes);
    assert!(probes.status.success(), "{probes_text}");
    assert!(probes_text.contains("ollama"), "{probes_text}");
    assert!(
        !probes_text.contains("5090") && !probes_text.contains("rtx-"),
        "probe output must not carry a SKU: {probes_text}"
    );

    let tmp = repo_root().join(format!("target/test-sku-probe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let hop = estate_bin()
        .args([
            "convey",
            "hop",
            "--id",
            "local-5090",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &tmp.display().to_string(),
        ])
        .output()
        .unwrap();
    let hop_text = text(&hop);
    assert!(!hop.status.success(), "{hop_text}");
    assert!(hop_text.contains("refuse:sku-banned"), "{hop_text}");
    assert!(!tmp.join("conveyor-mesh.json").exists());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn readers_refuse_tampered_sku_host_class() {
    let root = repo_root().join(format!("target/test-sku-readers-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let roots = root.join("roots");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();
    let estate = fixture("examples/estate.yaml");

    let apply = estate_bin()
        .args([
            "apply",
            "--estate",
            &estate,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(apply.status.success(), "{}", text(&apply));

    let sync = estate_bin()
        .args(["convey", "sync", "--state-dir", &state_s])
        .output()
        .unwrap();
    assert!(sync.status.success(), "{}", text(&sync));
    // Stock cell-one-box is deny-default. Rename that placement so this hop
    // id is not a coverage row. The synced lease names a population, so an
    // unnamed call is refuse:agent-unbound. Horizon is on that lease; the
    // bypass grants lane-tool so the baseline call can succeed.
    let bypass = root.join("not-this-hop.yaml");
    let mut bypass_estate =
        estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
    bypass_estate
        .placements
        .iter_mut()
        .find(|p| p.id == "cell-one-box")
        .unwrap()
        .id = "other-box".into();
    bypass_estate
        .agents
        .iter_mut()
        .find(|a| a.id == "horizon")
        .unwrap()
        .tools
        .push(estate_schema::ToolDecl {
            id: "lane-tool".into(),
            description: None,
        });
    bypass_estate.intentions.push(estate_schema::Intention {
        subject_agent: "horizon".into(),
        object: "lane-tool".into(),
        kind: estate_schema::IntentionKind::Tool,
        effect: estate_schema::Effect::Allow,
        note: None,
    });
    std::fs::write(
        &bypass,
        estate_schema::render_estate_yaml(&bypass_estate).unwrap(),
    )
    .unwrap();
    let bypass_s = bypass.display().to_string();
    let call_ok = estate_bin()
        .args([
            "convey",
            "call",
            "--id",
            "cell-one-box",
            "--agent",
            "horizon",
            "--estate",
            &bypass_s,
            "--state-dir",
            &state_s,
        ])
        .output()
        .unwrap();
    assert!(call_ok.status.success(), "{}", text(&call_ok));

    let leases_path = state.join("placement-actual.json");
    let tampered = tamper_box_host_class(&leases_path, "rtx-5090");
    assert!(tampered.contains("rtx-5090"));
    assert!(!tampered.contains("\"host_class\": \"any\"") || tampered.contains("cursor-cloud"));

    let mesh_before = std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap();

    let sync_bad = estate_bin()
        .args(["convey", "sync", "--state-dir", &state_s])
        .output()
        .unwrap();
    let sync_text = text(&sync_bad);
    assert!(!sync_bad.status.success(), "{sync_text}");
    assert!(sync_text.contains("refuse:bad-host-class"), "{sync_text}");
    assert_eq!(
        std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap(),
        mesh_before,
        "sync must not rewrite mesh after a host_class refuse"
    );

    let call_bad = estate_bin()
        .args([
            "convey",
            "call",
            "--id",
            "cell-one-box",
            "--estate",
            &bypass_s,
            "--state-dir",
            &state_s,
        ])
        .output()
        .unwrap();
    let call_text = text(&call_bad);
    assert!(!call_bad.status.success(), "{call_text}");
    assert!(call_text.contains("refuse:bad-host-class"), "{call_text}");
    assert!(
        !call_text.contains("lease-bound box hop") || call_text.contains("refuse:bad-host-class"),
        "call must not allow after swallowing slim-parse: {call_text}"
    );

    let status = estate_bin()
        .args([
            "status",
            "--estate",
            &estate,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--root",
            &repo_root().display().to_string(),
        ])
        .output()
        .unwrap();
    let status_text = text(&status);
    assert!(!status.status.success(), "{status_text}");
    assert!(status_text.contains("refuse:bad-host-class"), "{status_text}");
    assert!(
        !status_text.contains("Cell One status"),
        "status must refuse before printing: {status_text}"
    );

    let leases = estate_bin()
        .args([
            "leases",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state_s,
        ])
        .output()
        .unwrap();
    let leases_text = text(&leases);
    assert!(!leases.status.success(), "{leases_text}");
    assert!(leases_text.contains("rtx-5090"), "{leases_text}");
    assert!(leases_text.contains("refuse:bad-host-class"), "{leases_text}");
    assert!(
        !leases_text.contains("cell-one.placement"),
        "leases must refuse before printing the SKU actual: {leases_text}"
    );

    let recon = estate_bin()
        .args([
            "reconcile",
            "--estate",
            &estate,
            "--state-dir",
            &state_s,
        ])
        .output()
        .unwrap();
    let recon_text = text(&recon);
    assert!(!recon.status.success(), "{recon_text}");
    assert!(recon_text.contains("refuse:bad-host-class"), "{recon_text}");
    assert!(recon_text.contains("rtx-5090"), "{recon_text}");
    assert!(
        !recon_text.contains("host_class=any") || recon_text.contains("rtx-5090"),
        "reconcile must show the SKU, not rewrite it: {recon_text}"
    );

    let expire = estate_bin()
        .args(["expire", "--estate", &estate, "--state-dir", &state_s])
        .output()
        .unwrap();
    assert!(expire.status.success(), "{}", text(&expire));

    let apply_again = estate_bin()
        .args([
            "apply",
            "--estate",
            &estate,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let apply_text = text(&apply_again);
    assert!(!apply_again.status.success(), "{apply_text}");
    assert!(
        apply_text.contains("refuse:drift") || apply_text.contains("host-class"),
        "{apply_text}"
    );

    let after = std::fs::read_to_string(&leases_path).unwrap();
    assert_eq!(after, tampered, "readers must not rewrite SKU host_class to any");
    assert!(after.contains("rtx-5090"));
    assert!(!after.contains("\"host_class\": \"any\"") || after.contains("cursor-cloud"));

    let _ = std::fs::remove_dir_all(&root);
}
