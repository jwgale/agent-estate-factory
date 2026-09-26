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
        "api-version-unknown.yaml",
        "kind-unknown.yaml",
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
        .args([
            "leases",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(leases.status.success());
    let stdout = String::from_utf8_lossy(&leases.stdout);
    assert!(stdout.contains("cloud-agent"));
    assert!(stdout.contains("\"spawned\": false") || stdout.contains("spawned\": false"));
    let audits = estate_bin()
        .args([
            "audits",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(audits.status.success());
    assert!(String::from_utf8_lossy(&audits.stdout).contains("require_plan=true"));
    let history = estate_bin()
        .args([
            "history",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
        ])
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
            "--estate",
            &fixture("examples/estate.yaml"),
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
            "--estate",
            &fixture("examples/estate.yaml"),
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
            "--estate",
            &fixture("examples/estate.yaml"),
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
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!cloud.status.success(), "{}", String::from_utf8_lossy(&cloud.stderr));
    let cloud_err = format!(
        "{}{}",
        String::from_utf8_lossy(&cloud.stdout),
        String::from_utf8_lossy(&cloud.stderr)
    );
    assert!(
        cloud_err.contains("refuse:hop-coverage") && cloud_err.contains("(deny)"),
        "{cloud_err}"
    );
    assert!(!cloud_err.contains("deny-default"), "{cloud_err}");
    let cloud_call = estate_bin()
        .args([
            "convey",
            "call",
            "--id",
            "cursor-cloud",
            "--capability",
            "mesh-stub",
            "--estate",
            &fixture("examples/estate.yaml"),
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
        .args([
            "expire",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
        ])
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
        .args([
            "expire",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
        ])
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
            "--estate",
            &fixture("examples/estate.yaml"),
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
    assert!(doc.contains("compile-only"));
    assert!(doc.contains("specialist-pack.v0.json"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn wave5_sessions_plan_diff_convey_ttl() {
    let tmp = repo_root().join(format!("target/test-wave5-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let state = tmp.join("state");
    let plans = tmp.join("plans");

    let happy = estate_bin()
        .args(["validate", "--estate", &fixture("examples/fixtures/happy.yaml")])
        .output()
        .unwrap();
    assert!(
        happy.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&happy.stderr)
    );

    let api = estate_bin()
        .args([
            "validate",
            "--estate",
            &fixture("examples/fixtures/refuse-api-version.yaml"),
        ])
        .output()
        .unwrap();
    assert!(!api.status.success());
    let api_err = format!(
        "{}{}",
        String::from_utf8_lossy(&api.stdout),
        String::from_utf8_lossy(&api.stderr)
    );
    assert!(api_err.contains("apiVersion") || api_err.contains("upgrade"));

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
    assert!(state.join("sessions.jsonl").is_file());
    let listed = estate_bin()
        .args(["sessions", "list", "--state-dir", &state.display().to_string()])
        .output()
        .unwrap();
    assert!(listed.status.success());
    assert!(String::from_utf8_lossy(&listed.stdout).contains("spawn"));

    let same = estate_bin()
        .args([
            "plan",
            "diff",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        same.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&same.stderr)
    );

    let narrow = r#"{
      "schema": "cell-one.plan.v0",
      "desired_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "against_hash": null,
      "added": {"agents": [], "lanes": [], "intentions": [], "model_bindings": [], "placements": [], "enrich_packs": []},
      "removed": {"agents": [], "lanes": [], "intentions": [], "model_bindings": [], "placements": [], "enrich_packs": []},
      "changed": {"agents": [], "lanes": [], "intentions": [], "model_bindings": [], "placements": [], "enrich_packs": []},
      "blast_radius_text": "empty",
      "created_at": "unix:1"
    }"#;
    let wide = r#"{
      "schema": "cell-one.plan.v0",
      "desired_hash": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "against_hash": null,
      "added": {"agents": ["horizon", "research"], "lanes": ["horizon"], "intentions": [], "model_bindings": [], "placements": [], "enrich_packs": []},
      "removed": {"agents": [], "lanes": [], "intentions": [], "model_bindings": [], "placements": [], "enrich_packs": []},
      "changed": {"agents": [], "lanes": [], "intentions": [], "model_bindings": [], "placements": [], "enrich_packs": []},
      "blast_radius_text": "wider",
      "created_at": "unix:2"
    }"#;
    let from_p = tmp.join("from.json");
    let to_p = tmp.join("to.json");
    std::fs::write(&from_p, narrow).unwrap();
    std::fs::write(&to_p, wide).unwrap();
    let wider = estate_bin()
        .args([
            "plan",
            "diff",
            "--from",
            &from_p.display().to_string(),
            "--to",
            &to_p.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!wider.status.success());
    let wider_err = format!(
        "{}{}",
        String::from_utf8_lossy(&wider.stdout),
        String::from_utf8_lossy(&wider.stderr)
    );
    assert!(wider_err.contains("refuse:wider") || wider_err.contains("WIDER"));
    let allowed = estate_bin()
        .args([
            "plan",
            "diff",
            "--from",
            &from_p.display().to_string(),
            "--to",
            &to_p.display().to_string(),
            "--allow-wider",
        ])
        .output()
        .unwrap();
    assert!(
        allowed.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&allowed.stderr)
    );

    let hop = estate_bin()
        .args([
            "convey",
            "hop",
            "--id",
            "ttl-box",
            "--capability",
            "lane-tool",
            "--ttl-secs",
            "1",
            "--estate",
            &fixture("examples/estate.yaml"),
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
    let mut mesh: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap(),
    )
    .unwrap();
    if let Some(leases) = mesh.get_mut("leases").and_then(|v| v.as_array_mut()) {
        if let Some(lease) = leases.iter_mut().find(|l| l.get("hop_id").and_then(|k| k.as_str()) == Some("ttl-box"))
        {
            lease["issued_at"] = serde_json::json!(1);
            lease["expires_at"] = serde_json::json!(2);
        }
    }
    std::fs::write(
        state.join("conveyor-mesh.json"),
        serde_json::to_string_pretty(&mesh).unwrap(),
    )
    .unwrap();
    let call = estate_bin()
        .args([
            "convey",
            "call",
            "--id",
            "ttl-box",
            "--capability",
            "lane-tool",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!call.status.success());
    let call_err = format!(
        "{}{}",
        String::from_utf8_lossy(&call.stdout),
        String::from_utf8_lossy(&call.stderr)
    );
    assert!(call_err.contains("refuse:expired"));
    let expired = estate_bin()
        .args([
            "convey",
            "expire",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!expired.status.success());
    let forgot = estate_bin()
        .args([
            "convey",
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
    let forgot_text = format!(
        "{}{}",
        String::from_utf8_lossy(&forgot.stdout),
        String::from_utf8_lossy(&forgot.stderr)
    );
    assert!(forgot_text.contains("restamp"), "{forgot_text}");
    let mesh_after: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(state.join("conveyor-mesh.json")).unwrap(),
    )
    .unwrap();
    assert!(
        mesh_after["hops"]
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["id"] == "ttl-box"),
        "forget must keep hop decls: {mesh_after}"
    );
    let fresh = estate_bin()
        .args([
            "convey",
            "call",
            "--id",
            "ttl-box",
            "--capability",
            "lane-tool",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    let fresh_text = format!(
        "{}{}",
        String::from_utf8_lossy(&fresh.stdout),
        String::from_utf8_lossy(&fresh.stderr)
    );
    assert!(fresh.status.success(), "{fresh_text}");
    assert!(fresh_text.contains("lease-refresh"), "{fresh_text}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn wave6_backup_policy_catalog_pause() {
    let tmp = repo_root().join(format!("target/test-wave6-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let state = tmp.join("state");
    let plans = tmp.join("plans");
    let backups = tmp.join("backups");

    let allow = estate_bin()
        .args([
            "policy",
            "check",
            "--policy",
            &fixture("examples/fixtures/policy-allow.yaml"),
            "--action",
            "apply",
        ])
        .output()
        .unwrap();
    assert!(
        allow.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&allow.stderr)
    );

    let deny = estate_bin()
        .args([
            "policy",
            "check",
            "--policy",
            &fixture("examples/fixtures/policy-deny.yaml"),
            "--action",
            "apply",
        ])
        .output()
        .unwrap();
    assert!(!deny.status.success());
    let deny_err = format!(
        "{}{}",
        String::from_utf8_lossy(&deny.stdout),
        String::from_utf8_lossy(&deny.stderr)
    );
    assert!(deny_err.contains("refuse:policy"));

    let unknown = estate_bin()
        .args([
            "policy",
            "check",
            "--policy",
            &fixture("examples/fixtures/policy-unknown-action.yaml"),
            "--action",
            "apply",
        ])
        .output()
        .unwrap();
    assert!(!unknown.status.success());
    let unknown_err = format!(
        "{}{}",
        String::from_utf8_lossy(&unknown.stdout),
        String::from_utf8_lossy(&unknown.stderr)
    );
    assert!(unknown_err.contains("refuse:unknown-action"));

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
            "--policy",
            &fixture("examples/fixtures/policy-allow.yaml"),
        ])
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&applied.stderr)
    );

    let apply_deny = estate_bin()
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
            "--policy",
            &fixture("examples/fixtures/policy-deny.yaml"),
        ])
        .output()
        .unwrap();
    assert!(!apply_deny.status.success());

    let catalog = estate_bin()
        .args([
            "catalog",
            "--out",
            &tmp.join("catalog.json").display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(catalog.status.success());
    let cat = String::from_utf8_lossy(&catalog.stdout);
    assert!(cat.contains("streaming="));
    assert!(cat.contains("tools="));
    assert!(cat.contains("vision="));
    assert!(cat.contains("context="));
    let dumped = std::fs::read_to_string(tmp.join("catalog.json")).unwrap();
    assert!(dumped.contains("context_tokens"));
    assert!(dumped.contains("\"mlx\""));

    let backed = estate_bin()
        .args([
            "backup",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--out",
            &backups.display().to_string(),
            "--policy",
            &fixture("examples/fixtures/policy-allow.yaml"),
        ])
        .output()
        .unwrap();
    assert!(
        backed.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&backed.stderr)
    );
    let archive = std::fs::read_dir(&backups)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("cell-backup-"))
                .unwrap_or(false)
        })
        .expect("timestamped archive");
    assert!(archive.join("backup.json").is_file());
    assert!(archive.join("cell").join("placement-actual.json").is_file());

    let empty = tmp.join("empty-restore");
    std::fs::create_dir_all(&empty).unwrap();
    let dry = estate_bin()
        .args([
            "restore",
            "--from",
            &archive.display().to_string(),
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &empty.display().to_string(),
            "--plans-dir",
            &tmp.join("empty-plans").display().to_string(),
            "--dry-run",
            "--policy",
            &fixture("examples/fixtures/policy-allow.yaml"),
        ])
        .output()
        .unwrap();
    assert!(
        dry.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&dry.stderr)
    );
    assert!(!empty.join("placement-actual.json").exists());

    let tamper = tmp.join("tampered-backup");
    copy_dir(&archive, &tamper);
    std::fs::write(
        tamper.join("backup.json"),
        r#"{
  "schema": "cell-one.cell-backup.v0",
  "created_at": "unix:1",
  "state_dir": "tamper",
  "files": [],
  "sacred_ids": ["not-the-locked-set"],
  "writes": true,
  "cloud_agent_spawned": false,
  "note": "tampered sacred set"
}"#,
    )
    .unwrap();
    let mismatch = estate_bin()
        .args([
            "restore",
            "--from",
            &tamper.display().to_string(),
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &empty.display().to_string(),
            "--dry-run",
            "--policy",
            &fixture("examples/fixtures/policy-allow.yaml"),
        ])
        .output()
        .unwrap();
    assert!(!mismatch.status.success());
    let mismatch_err = format!(
        "{}{}",
        String::from_utf8_lossy(&mismatch.stdout),
        String::from_utf8_lossy(&mismatch.stderr)
    );
    assert!(mismatch_err.contains("refuse:sacred-mismatch"));
    assert!(!empty.join("placement-actual.json").exists());

    let proof = estate_bin()
        .args([
            "pause-proof",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &tmp.join("pause").display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        proof.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&proof.stderr)
    );
    let body = String::from_utf8_lossy(&proof.stdout);
    assert!(body.contains("\"leases_survived\": true"));
    assert!(body.contains("\"cloud_spawned\": false"));

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
    assert!(doctor.status.success());
    let doc = String::from_utf8_lossy(&doctor.stdout);
    assert!(doc.contains("policy.v0.json"));
    assert!(doc.contains("cell-backup.v0.json"));
    assert!(doc.contains("cell-one.policy.v0.yaml"));

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn wave7_status_curator_sync() {
    let tmp = repo_root().join(format!("target/test-wave7-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let state = tmp.join("state");
    let plans = tmp.join("plans");
    let drop = tmp.join("packs");
    std::fs::create_dir_all(&drop).unwrap();
    std::fs::copy(
        repo_root().join("examples/fixtures/overnight-traces.pack.json"),
        drop.join("overnight-traces.pack.json"),
    )
    .unwrap();

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

    let ok_imp = estate_bin()
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
            &fixture("examples/estate.yaml"),
            "--curator",
            "jason",
        ])
        .output()
        .unwrap();
    assert!(
        ok_imp.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&ok_imp.stderr)
    );
    let bad_imp = estate_bin()
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
            &fixture("examples/estate.yaml"),
            "--curator",
            "not-jason",
        ])
        .output()
        .unwrap();
    assert!(!bad_imp.status.success());
    let bad_err = format!(
        "{}{}",
        String::from_utf8_lossy(&bad_imp.stdout),
        String::from_utf8_lossy(&bad_imp.stderr)
    );
    assert!(bad_err.contains("refuse:curator"));

    estate_bin()
        .args(["convey", "sync", "--state-dir", &state.display().to_string()])
        .output()
        .unwrap();
    let hop = estate_bin()
        .args([
            "convey",
            "hop",
            "--id",
            "ttl-box",
            "--capability",
            "lane-tool",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(hop.status.success());
    let synced = estate_bin()
        .args(["convey", "sync", "--state-dir", &state.display().to_string()])
        .output()
        .unwrap();
    assert!(synced.status.success());
    assert!(String::from_utf8_lossy(&synced.stdout).contains("ttl-box"));

    let status = estate_bin()
        .args([
            "status",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--packs-dir",
            &drop.display().to_string(),
            "--policy",
            &fixture("policy/cell-one.policy.v0.yaml"),
            "--root",
            &repo_root().display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&status.stderr)
    );
    let watch = String::from_utf8_lossy(&status.stdout);
    assert!(watch.contains("paused: no"));
    assert!(watch.contains("leases:"));
    assert!(watch.contains("last_plan:"));
    assert!(watch.contains("last_apply:"));
    assert!(watch.contains("open_proposals:"));
    assert!(watch.contains("policy: present"));
    assert!(watch.contains("doctor: ok"));
    assert!(watch.contains("cloud-agent"));

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
    assert!(doctor.status.success());
    let doc = String::from_utf8_lossy(&doctor.stdout);
    assert!(doc.contains("GATE-90.md"));
    assert!(doc.contains("schema/README.md"));
    assert!(doc.contains("CHANGELOG.md"));
    assert!(doc.contains("sacred.yaml"));
    assert!(doc.contains("MORNING-BRIEF-2026-09-21.md"));
    assert!(doc.contains("PR2-DESCRIPTION.md"));

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn wave8_idempotent_export_pr_sacred_mixed() {
    let tmp = repo_root().join(format!("target/test-wave8-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let state = tmp.join("state");
    let plans = tmp.join("plans");
    let sacred = fixture("examples/fixtures/sacred-overlay.yaml");

    let mixed = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "validate",
            "--estate",
            &fixture("examples/fixtures/mixed-frontier-local.yaml"),
        ])
        .output()
        .unwrap();
    assert!(
        mixed.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&mixed.stderr)
    );

    let overlay_estate = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "validate",
            "--estate",
            &fixture("examples/fixtures/refuse-sacred-overlay.yaml"),
        ])
        .output()
        .unwrap();
    assert!(!overlay_estate.status.success());

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
        ])
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&applied.stderr)
    );

    let again = estate_bin()
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
    assert!(again.status.success());
    let again_out = String::from_utf8_lossy(&again.stdout);
    assert!(again_out.contains("unchanged"));

    let _ = std::fs::remove_dir_all(state.join("sessions").join("horizon"));
    let drifted = estate_bin()
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
    assert!(!drifted.status.success());
    let drift_err = format!(
        "{}{}",
        String::from_utf8_lossy(&drifted.stdout),
        String::from_utf8_lossy(&drifted.stderr)
    );
    assert!(drift_err.contains("refuse:drift"));

    let forced = estate_bin()
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
            "--force",
        ])
        .output()
        .unwrap();
    assert!(
        forced.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&forced.stderr)
    );

    let pr_out = tmp.join("PR.md");
    let exported = estate_bin()
        .args([
            "plan",
            "export-pr",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--out",
            &pr_out.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        exported.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&exported.stderr)
    );
    let body = std::fs::read_to_string(&pr_out).unwrap();
    assert!(body.contains("Blast radius"));
    assert!(body.contains("Refuse risks"));
    assert!(body.contains("Reviewed"));
    assert!(body.contains("covering"));

    let hop = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "convey",
            "hop",
            "--id",
            "lab-notebook",
            "--capability",
            "lane-tool",
            "--estate",
            &fixture("examples/estate.yaml"),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!hop.status.success());
    let hop_err = format!(
        "{}{}",
        String::from_utf8_lossy(&hop.stdout),
        String::from_utf8_lossy(&hop.stderr)
    );
    assert!(hop_err.contains("refuse:sacred-id"));

    let dry = estate_bin()
        .args([
            "apply",
            "--dry-run",
            "--estate",
            &fixture("examples/fixtures/mixed-frontier-local.yaml"),
            "--state-dir",
            &tmp.join("mixed-state").display().to_string(),
            "--roots-base",
            &tmp.display().to_string(),
            "--plans-dir",
            &tmp.join("mixed-plans").display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        dry.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&dry.stderr)
    );
    assert!(!tmp.join("mixed-state").join("placement-actual.json").exists());

    let _ = std::fs::remove_dir_all(&tmp);
}

fn copy_dir(src: &std::path::Path, dest: &std::path::Path) {
    std::fs::create_dir_all(dest).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let to = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &to);
        } else {
            std::fs::copy(&path, &to).unwrap();
        }
    }
}
