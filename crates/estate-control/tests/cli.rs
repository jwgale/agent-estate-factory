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

#[test]
fn validate_example_ok() {
    let out = estate_bin()
        .args(["validate", "--estate", &fixture("examples/estate.yaml")])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("horizon"));
    assert!(stdout.contains("ok"));
}

#[test]
fn validate_invalid_fails_closed() {
    for name in [
        "too-few-agents.yaml",
        "shared-lane.yaml",
        "sacred-as-agent.yaml",
        "frontier-only.yaml",
        "missing-sacred.yaml",
        "allow-sacred-intention.yaml",
        "default-allow.yaml",
        "sku-binding.yaml",
        "placement-sku.yaml",
        "placement-unknown-agent.yaml",
        "placement-sacred-cloud.yaml",
        "host-class-bad.yaml",
        "not-yaml.txt",
    ] {
        let path = fixture(&format!("examples/invalid/{name}"));
        let out = estate_bin()
            .args(["validate", "--estate", &path])
            .output()
            .unwrap();
        assert!(
            !out.status.success(),
            "{name} should fail closed; stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn plan_appends_plans_dir() {
    let tmp = repo_root().join(format!("target/test-plans-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let out = estate_bin()
        .args([
            "plan",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--plans-dir",
            &tmp.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Blast radius"));
    let entries: Vec<_> = std::fs::read_dir(&tmp)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(entries.iter().any(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md")));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn apply_require_plan_is_gated_and_auditable() {
    let tmp = repo_root().join(format!("target/test-apply-gate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let plans = tmp.join("plans");
    let state = tmp.join("state");
    let gated = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--require-plan",
        ])
        .output()
        .unwrap();
    assert!(
        !gated.status.success(),
        "apply without a covering plan must fail; stderr={}",
        String::from_utf8_lossy(&gated.stderr)
    );
    let planned = estate_bin()
        .args([
            "plan",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--plans-dir",
            &plans.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        planned.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&planned.stderr)
    );
    let applied = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--require-plan",
        ])
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&applied.stderr)
    );
    assert!(state.join("placement-actual.json").is_file());
    let stdout = String::from_utf8_lossy(&applied.stdout);
    assert!(stdout.contains("audit"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn feed_import_does_not_rewrite_estate_and_promote_fails() {
    let tmp = repo_root().join(format!("target/test-feed-import-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("drop")).unwrap();
    let estate = repo_root().join("examples/estate.yaml");
    let before = std::fs::read_to_string(&estate).unwrap();
    let packed = estate_bin()
        .args([
            "feed",
            "pack",
            "--feed-dir",
            &tmp.join("feed").display().to_string(),
            "--drop-dir",
            &tmp.join("drop").display().to_string(),
            "--id",
            "overnight-traces",
        ])
        .output()
        .unwrap();
    assert!(
        packed.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&packed.stderr)
    );
    let imported = estate_bin()
        .args([
            "feed",
            "import",
            "--id",
            "overnight-traces",
            "--drop-dir",
            &tmp.join("drop").display().to_string(),
            "--accepted-dir",
            &tmp.join("accepted").display().to_string(),
            "--estate",
            &estate.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        imported.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&imported.stderr)
    );
    let stdout = String::from_utf8_lossy(&imported.stdout);
    assert!(stdout.contains("estate_bound=false"));
    assert!(stdout.contains("estate file unchanged"));
    assert_eq!(before, std::fs::read_to_string(&estate).unwrap());
    let promo = estate_bin()
        .args(["feed", "promote", "--id", "overnight-traces"])
        .output()
        .unwrap();
    assert!(!promo.status.success());
    let sku = estate_bin()
        .args([
            "feed",
            "pack",
            "--feed-dir",
            &tmp.join("feed").display().to_string(),
            "--drop-dir",
            &tmp.join("drop").display().to_string(),
            "--id",
            "local-5090",
        ])
        .output()
        .unwrap();
    assert!(!sku.status.success(), "SKU pack id must fail closed");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn catalog_and_leases_are_file_sot() {
    let tmp = repo_root().join(format!("target/test-catalog-leases-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let catalog = tmp.join("catalog.json");
    let dumped = estate_bin()
        .args(["catalog", "--out", &catalog.display().to_string()])
        .output()
        .unwrap();
    assert!(
        dumped.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&dumped.stderr)
    );
    assert!(catalog.is_file());
    let text = std::fs::read_to_string(&catalog).unwrap();
    assert!(text.contains("cell-one.local-catalog.v0"));
    assert!(text.contains("ollama"));
    assert!(!text.contains("5090"));

    let plans = tmp.join("plans");
    let state = tmp.join("state");
    let planned = estate_bin()
        .args([
            "plan",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--plans-dir",
            &plans.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(planned.status.success());
    let applied = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--require-plan",
        ])
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&applied.stderr)
    );
    assert!(state.join("catalog.json").is_file());
    let leases = estate_bin()
        .args(["leases", "--state-dir", &state.display().to_string()])
        .output()
        .unwrap();
    assert!(leases.status.success());
    let stdout = String::from_utf8_lossy(&leases.stdout);
    assert!(stdout.contains("cloud-agent"));
    assert!(stdout.contains("\"spawned\": false") || stdout.contains("spawned\": false"));
    let audits = estate_bin()
        .args(["audits", "--state-dir", &state.display().to_string()])
        .output()
        .unwrap();
    assert!(audits.status.success());
    assert!(String::from_utf8_lossy(&audits.stdout).contains("require_plan=true"));
    let history = estate_bin()
        .args(["history", "--state-dir", &state.display().to_string()])
        .output()
        .unwrap();
    assert!(history.status.success());
    assert!(String::from_utf8_lossy(&history.stdout).contains("lifecycle history"));
    let probes = estate_bin().args(["probes"]).output().unwrap();
    assert!(probes.status.success());
    let probe_out = String::from_utf8_lossy(&probes.stdout);
    assert!(probe_out.contains("ollama"));
    assert!(probe_out.contains("live_probed=false"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn host_matrix_validates_and_bad_host_fails() {
    for name in [
        "examples/hosts/rtx-consumer.yaml",
        "examples/hosts/apple-silicon.yaml",
        "examples/hosts/nvidia-rental.yaml",
        "examples/hosts/multi-host.yaml",
    ] {
        let out = estate_bin()
            .args(["validate", "--estate", &fixture(name)])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{name} stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn require_fresh_plan_refuses_greenfield_after_apply() {
    let tmp = repo_root().join(format!("target/test-fresh-plan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let plans = tmp.join("plans");
    let state = tmp.join("state");
    let planned = estate_bin()
        .args([
            "plan",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--plans-dir",
            &plans.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--reviewed",
            "--reviewed-dir",
            &tmp.join("reviewed").display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        planned.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&planned.stderr)
    );
    let stdout = String::from_utf8_lossy(&planned.stdout);
    assert!(stdout.contains("Security-as-IaC"));
    assert!(tmp.join("reviewed").join("INDEX.md").is_file() || {
        std::fs::read_dir(tmp.join("reviewed"))
            .unwrap()
            .any(|e| e.unwrap().path().extension().and_then(|s| s.to_str()) == Some("md"))
    });
    let applied = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--require-plan",
        ])
        .output()
        .unwrap();
    assert!(applied.status.success());
    let stale = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--require-fresh-plan",
        ])
        .output()
        .unwrap();
    assert!(
        !stale.status.success(),
        "greenfield covering plan after apply must be stale; stdout={} stderr={}",
        String::from_utf8_lossy(&stale.stdout),
        String::from_utf8_lossy(&stale.stderr)
    );
    let replanned = estate_bin()
        .args([
            "plan",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--plans-dir",
            &plans.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(replanned.status.success());
    let fresh = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--require-fresh-plan",
        ])
        .output()
        .unwrap();
    assert!(
        fresh.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&fresh.stderr)
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn convey_lease_bound_and_packs_refuse_promote() {
    let tmp = repo_root().join(format!("target/test-convey-packs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("drop")).unwrap();
    let state = tmp.join("state");
    let hop = estate_bin()
        .args([
            "convey",
            "hop",
            "--id",
            "box-notes",
            "--kind",
            "box",
            "--capability",
            "notes-append",
            "--host-class",
            "rtx_consumer",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        hop.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&hop.stderr)
    );
    let call = estate_bin()
        .args([
            "convey",
            "call",
            "--id",
            "box-notes",
            "--capability",
            "notes-append",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(call.status.success());
    let missing = estate_bin()
        .args([
            "convey",
            "call",
            "--id",
            "no-such-hop",
            "--capability",
            "lane-tool",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!missing.status.success());
    let cloud = estate_bin()
        .args([
            "convey",
            "hop",
            "--id",
            "cursor-cloud",
            "--kind",
            "cloud-mesh",
            "--capability",
            "mesh-stub",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(cloud.status.success());
    let cloud_call = estate_bin()
        .args([
            "convey",
            "call",
            "--id",
            "cursor-cloud",
            "--capability",
            "mesh-stub",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!cloud_call.status.success());
    let drop = tmp.join("drop");
    std::fs::copy(
        repo_root().join("examples/fixtures/overnight-traces.pack.json"),
        drop.join("overnight-traces.pack.json"),
    )
    .unwrap();
    let listed = estate_bin()
        .args(["packs", "list", "--drop-dir", &drop.display().to_string()])
        .output()
        .unwrap();
    assert!(listed.status.success());
    assert!(String::from_utf8_lossy(&listed.stdout).contains("overnight-traces"));
    let promo = estate_bin()
        .args(["packs", "promote", "--id", "overnight-traces"])
        .output()
        .unwrap();
    assert!(!promo.status.success());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn reconcile_propose_and_audit_export() {
    let tmp = repo_root().join(format!("target/test-wave3-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("drop")).unwrap();
    let plans = tmp.join("plans");
    let state = tmp.join("state");
    let planned = estate_bin()
        .args([
            "plan",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--plans-dir",
            &plans.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        planned.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&planned.stderr)
    );
    let applied = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--require-plan",
        ])
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let recon = estate_bin()
        .args([
            "reconcile",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        recon.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&recon.stderr)
    );
    let recon_out = String::from_utf8_lossy(&recon.stdout);
    assert!(recon_out.contains("in_sync: true"));
    assert!(state.join("reconcile.md").is_file());

    let drop = tmp.join("drop");
    std::fs::copy(
        repo_root().join("examples/fixtures/overnight-traces.pack.json"),
        drop.join("overnight-traces.pack.json"),
    )
    .unwrap();
    let estate = repo_root().join("examples/estate.yaml");
    let before = std::fs::read_to_string(&estate).unwrap();
    let imported = estate_bin()
        .args([
            "packs",
            "import",
            "--id",
            "overnight-traces",
            "--drop-dir",
            &drop.display().to_string(),
            "--accepted-dir",
            &drop.join("accepted").display().to_string(),
            "--estate",
            &estate.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(imported.status.success());
    let proposed = estate_bin()
        .args([
            "packs",
            "propose",
            "--id",
            "overnight-traces",
            "--drop-dir",
            &drop.display().to_string(),
            "--accepted-dir",
            &drop.join("accepted").display().to_string(),
            "--proposed-dir",
            &drop.join("proposed").display().to_string(),
            "--estate",
            &estate.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        proposed.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&proposed.stderr)
    );
    let prop_out = String::from_utf8_lossy(&proposed.stdout);
    assert!(prop_out.contains("auto_apply: false"));
    assert_eq!(before, std::fs::read_to_string(&estate).unwrap());
    assert!(drop.join("proposed").join("overnight-traces.proposal.json").is_file());

    let export = estate_bin()
        .args([
            "audit",
            "export",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--packs-dir",
            &drop.display().to_string(),
            "--out",
            &tmp.join("audit-export").display().to_string(),
            "--tar",
        ])
        .output()
        .unwrap();
    assert!(
        export.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&export.stderr)
    );
    assert!(tmp.join("audit-export").join("MANIFEST.md").is_file());
    assert!(tmp.join("audit-export").join("placement-actual.json").is_file());
    assert!(tmp.join("audit-export").join("import-audit.jsonl").is_file());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn apply_dry_run_expire_and_doctor() {
    let tmp = repo_root().join(format!("target/test-wave4-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let state = tmp.join("state");
    let plans = tmp.join("plans");

    let dry = estate_bin()
        .args([
            "apply",
            "--dry-run",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        dry.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let stdout = String::from_utf8_lossy(&dry.stdout);
    assert!(stdout.contains("writes: false"));
    assert!(stdout.contains("Blast radius"));
    assert!(!state.join("placement-actual.json").exists());
    assert!(!state.join("actual-state.json").exists());

    let planned = estate_bin()
        .args([
            "plan",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--plans-dir",
            &plans.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(planned.status.success());
    let applied = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--require-plan",
        ])
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&applied.stderr)
    );

    let expire = estate_bin()
        .args(["expire", "--state-dir", &state.display().to_string()])
        .output()
        .unwrap();
    assert!(
        expire.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&expire.stderr)
    );

    let mut actual: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(state.join("placement-actual.json")).unwrap(),
    )
    .unwrap();
    if let Some(leases) = actual.get_mut("leases").and_then(|v| v.as_array_mut()) {
        if let Some(lease) = leases.iter_mut().find(|l| l.get("kind").and_then(|k| k.as_str()) == Some("box"))
        {
            lease["ttl_secs"] = serde_json::json!(1);
            lease["issued_at"] = serde_json::json!(1);
            lease["expires_at"] = serde_json::json!(2);
        }
    }
    std::fs::write(
        state.join("placement-actual.json"),
        serde_json::to_string_pretty(&actual).unwrap(),
    )
    .unwrap();
    let listed = estate_bin()
        .args(["expire", "--state-dir", &state.display().to_string()])
        .output()
        .unwrap();
    assert!(!listed.status.success());
    assert!(String::from_utf8_lossy(&listed.stderr).contains("refuse:expired")
        || String::from_utf8_lossy(&listed.stdout).contains("refuse:expired"));
    let refused = estate_bin()
        .args([
            "apply",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    let dry_ref = estate_bin()
        .args([
            "apply",
            "--dry-run",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!dry_ref.status.success());
    let forgot = estate_bin()
        .args([
            "expire",
            "--forget",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        forgot.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&forgot.stderr)
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
    assert!(
        doctor.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&doctor.stderr),
        String::from_utf8_lossy(&doctor.stdout)
    );
    let doc = String::from_utf8_lossy(&doctor.stdout);
    assert!(doc.contains("no .github/workflows/*.yml"));
    assert!(doc.contains("specialist-pack.v0.json"));
    let _ = std::fs::remove_dir_all(&tmp);
}
