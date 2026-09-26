//! After the SKU series: swallows, feed redaction, append-only journals.
//! Isolated cell. No new verb. Cloud never spawned.

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

fn apply_cell(estate: &str, state: &str, roots: &str, plans: &str) {
    let out = estate_bin()
        .args([
            "apply",
            "--estate",
            estate,
            "--state-dir",
            state,
            "--roots-base",
            roots,
            "--plans-dir",
            plans,
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn journals_append_only_under_suspend_resume_expire_forget() {
    let root = repo_root().join(format!("target/test-honesty-jsonl-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let roots = root.join("roots");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();
    let estate = fixture("examples/estate.yaml");
    apply_cell(
        &estate,
        &state_s,
        &roots.display().to_string(),
        &plans.display().to_string(),
    );

    let life0 = std::fs::read_to_string(state.join("lifecycle.jsonl")).unwrap();
    let sess0 = std::fs::read_to_string(state.join("sessions.jsonl")).unwrap();
    assert!(!life0.is_empty());
    assert!(!sess0.is_empty());

    let suspend = estate_bin()
        .args(["suspend", "--state-dir", &state_s])
        .output()
        .unwrap();
    assert!(suspend.status.success(), "{}", text(&suspend));
    let life1 = std::fs::read_to_string(state.join("lifecycle.jsonl")).unwrap();
    let sess1 = std::fs::read_to_string(state.join("sessions.jsonl")).unwrap();
    assert!(life1.starts_with(&life0), "suspend truncated lifecycle.jsonl");
    assert!(sess1.starts_with(&sess0), "suspend truncated sessions.jsonl");

    let resume = estate_bin()
        .args([
            "resume",
            "--estate",
            &estate,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(resume.status.success(), "{}", text(&resume));
    let life2 = std::fs::read_to_string(state.join("lifecycle.jsonl")).unwrap();
    let sess2 = std::fs::read_to_string(state.join("sessions.jsonl")).unwrap();
    assert!(life2.starts_with(&life1), "resume truncated lifecycle.jsonl");
    assert!(sess2.starts_with(&sess1), "resume truncated sessions.jsonl");

    let forget = estate_bin()
        .args(["expire", "--forget", "--estate", &estate, "--state-dir", &state_s])
        .output()
        .unwrap();
    assert!(
        forget.status.success() || text(&forget).contains("no expired"),
        "{}",
        text(&forget)
    );
    let life3 = std::fs::read_to_string(state.join("lifecycle.jsonl")).unwrap();
    let sess3 = std::fs::read_to_string(state.join("sessions.jsonl")).unwrap();
    assert_eq!(life3, life2, "expire --forget must not rewrite lifecycle.jsonl");
    assert_eq!(sess3, sess2, "expire --forget must not rewrite sessions.jsonl");

    let mut places: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(state.join("placement-actual.json")).unwrap())
            .unwrap();
    if let Some(leases) = places["leases"].as_array_mut() {
        for lease in leases {
            lease["expires_at"] = serde_json::json!(1);
        }
    }
    std::fs::write(
        state.join("placement-actual.json"),
        serde_json::to_string_pretty(&places).unwrap(),
    )
    .unwrap();
    let forget_expired = estate_bin()
        .args(["expire", "--forget", "--estate", &estate, "--state-dir", &state_s])
        .output()
        .unwrap();
    assert!(forget_expired.status.success(), "{}", text(&forget_expired));
    let life4 = std::fs::read_to_string(state.join("lifecycle.jsonl")).unwrap();
    let sess4 = std::fs::read_to_string(state.join("sessions.jsonl")).unwrap();
    assert_eq!(
        life4, life3,
        "expire --forget of elapsed leases must not truncate lifecycle.jsonl"
    );
    assert_eq!(
        sess4, sess3,
        "expire --forget of elapsed leases must not truncate sessions.jsonl"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn restore_garbage_estate_file_refuses() {
    let root = repo_root().join(format!(
        "target/test-honesty-estate-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let roots = root.join("roots");
    let backups = root.join("backups");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture("examples/estate.yaml");
    apply_cell(
        &estate,
        &state.display().to_string(),
        &roots.display().to_string(),
        &plans.display().to_string(),
    );
    let backup = estate_bin()
        .args([
            "backup",
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--out",
            &backups.display().to_string(),
            "--estate",
            &estate,
            "--policy",
            &fixture("policy/cell-one.policy.v0.yaml"),
        ])
        .output()
        .unwrap();
    assert!(backup.status.success(), "{}", text(&backup));
    let archive = std::fs::read_dir(&backups)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("cell-backup-"))
                .unwrap_or(false)
        })
        .expect("cell-backup archive");
    let dest = root.join("restored");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("placement-actual.json"), "sentinel\n").unwrap();
    let garbage = root.join("garbage-estate.yaml");
    std::fs::write(&garbage, "not: [valid estate\n").unwrap();
    let restore = estate_bin()
        .args([
            "restore",
            "--from",
            &archive.display().to_string(),
            "--state-dir",
            &dest.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--estate",
            &garbage.display().to_string(),
            "--policy",
            &fixture("policy/cell-one.policy.v0.yaml"),
        ])
        .output()
        .unwrap();
    let body = text(&restore);
    assert!(!restore.status.success(), "{body}");
    assert_eq!(
        std::fs::read_to_string(dest.join("placement-actual.json")).unwrap(),
        "sentinel\n",
        "garbage estate must not restore: {body}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn garbage_lifecycle_json_refuses_suspend_resume_apply() {
    let root = repo_root().join(format!(
        "target/test-honesty-life-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let roots = root.join("roots");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture("examples/estate.yaml");
    apply_cell(
        &estate,
        &state.display().to_string(),
        &roots.display().to_string(),
        &plans.display().to_string(),
    );
    let leases = std::fs::read_to_string(state.join("placement-actual.json")).unwrap();
    std::fs::write(state.join("lifecycle.json"), "not-json\n").unwrap();

    let suspend = estate_bin()
        .args(["suspend", "--state-dir", &state.display().to_string()])
        .output()
        .unwrap();
    let body = text(&suspend);
    assert!(!suspend.status.success(), "{body}");
    assert!(body.contains("lifecycle.json"), "{body}");
    assert_eq!(
        std::fs::read_to_string(state.join("lifecycle.json")).unwrap(),
        "not-json\n"
    );
    assert_eq!(
        std::fs::read_to_string(state.join("placement-actual.json")).unwrap(),
        leases
    );

    let resume = estate_bin()
        .args([
            "resume",
            "--estate",
            &estate,
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &roots.display().to_string(),
        ])
        .output()
        .unwrap();
    let body = text(&resume);
    assert!(!resume.status.success(), "{body}");
    assert!(body.contains("lifecycle.json"), "{body}");
    assert_eq!(
        std::fs::read_to_string(state.join("lifecycle.json")).unwrap(),
        "not-json\n"
    );

    let apply = estate_bin()
        .args([
            "apply",
            "--estate",
            &estate,
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &roots.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let body = text(&apply);
    assert!(!apply.status.success(), "{body}");
    assert!(body.contains("lifecycle.json"), "{body}");
    assert_eq!(
        std::fs::read_to_string(state.join("lifecycle.json")).unwrap(),
        "not-json\n"
    );
    assert_eq!(
        std::fs::read_to_string(state.join("placement-actual.json")).unwrap(),
        leases,
        "garbage lifecycle must not restamp leases"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn catalog_write_fails_before_print_and_probes_stay_sku_free() {
    let root = repo_root().join(format!(
        "target/test-honesty-catalog-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let out = root.join("catalog.json");
    std::fs::create_dir_all(&out).unwrap();
    let catalog = estate_bin()
        .args(["catalog", "--out", &out.display().to_string()])
        .output()
        .unwrap();
    let body = text(&catalog);
    assert!(!catalog.status.success(), "{body}");
    assert!(
        !body.contains("local runtime catalog"),
        "catalog must write first, then print: {body}"
    );

    let probes = estate_bin().args(["probes"]).output().unwrap();
    let body = text(&probes);
    assert!(probes.status.success(), "{body}");
    assert!(body.contains("ollama"), "{body}");
    assert!(!body.contains("refuse:"), "{body}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn apply_import_pack_curator_refuse_writes_no_leases() {
    let root = repo_root().join(format!(
        "target/test-honesty-import-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let roots = root.join("roots");
    let drop = root.join("drop");
    std::fs::create_dir_all(&drop).unwrap();
    std::fs::copy(
        repo_root().join("examples/fixtures/overnight-traces.pack.json"),
        drop.join("overnight-traces.pack.json"),
    )
    .unwrap();
    let estate = fixture("examples/estate.yaml");
    apply_cell(
        &estate,
        &state.display().to_string(),
        &roots.display().to_string(),
        &plans.display().to_string(),
    );
    let leases = std::fs::read_to_string(state.join("placement-actual.json")).unwrap();
    let audit = std::fs::read_to_string(state.join("apply-audit.jsonl")).unwrap();

    let live = estate_bin()
        .args([
            "apply",
            "--import-pack",
            "overnight-traces",
            "--curator",
            "robot",
            "--packs-dir",
            &drop.display().to_string(),
            "--estate",
            &estate,
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &roots.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let body = text(&live);
    assert!(!live.status.success(), "{body}");
    assert!(body.contains("refuse:curator"), "{body}");
    assert_eq!(
        std::fs::read_to_string(state.join("placement-actual.json")).unwrap(),
        leases,
        "wrong curator must not restamp leases"
    );
    assert_eq!(
        std::fs::read_to_string(state.join("apply-audit.jsonl")).unwrap(),
        audit,
        "wrong curator must not append apply-audit"
    );
    assert!(!drop.join("accepted").join("overnight-traces.pack.json").exists());
    assert!(!drop.join("accepted").join("import-audit.jsonl").exists());

    let dry = estate_bin()
        .args([
            "apply",
            "--dry-run",
            "--import-pack",
            "overnight-traces",
            "--curator",
            "robot",
            "--packs-dir",
            &drop.display().to_string(),
            "--estate",
            &estate,
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &roots.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let body = text(&dry);
    assert!(!dry.status.success(), "{body}");
    assert!(body.contains("refuse:curator"), "{body}");
    assert!(
        !body.contains("dry-run ok"),
        "dry-run must not skip curator refuse: {body}"
    );
    assert_eq!(
        std::fs::read_to_string(state.join("placement-actual.json")).unwrap(),
        leases
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn apply_writes_nonempty_placement_actual() {
    let root = repo_root().join(format!(
        "target/test-honesty-place-write-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let roots = root.join("roots");
    std::fs::create_dir_all(&state).unwrap();
    apply_cell(
        &fixture("examples/estate.yaml"),
        &state.display().to_string(),
        &roots.display().to_string(),
        &plans.display().to_string(),
    );
    let blob = std::fs::read_to_string(state.join("placement-actual.json")).unwrap();
    assert!(!blob.trim().is_empty(), "apply must not write empty placement-actual");
    assert!(blob.contains("cell-one.placement-actual.v0"), "{blob}");
    let models = std::fs::read_to_string(state.join("model-actual.json")).unwrap();
    assert!(!models.trim().is_empty(), "apply must not write empty model-actual");
    assert!(models.contains("desired_hash"), "{models}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn garbage_plan_json_is_refuse_not_empty() {
    let root = repo_root().join(format!(
        "target/test-honesty-plan-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let roots = root.join("roots");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::create_dir_all(&plans).unwrap();
    let estate = fixture("examples/estate.yaml");
    apply_cell(
        &estate,
        &state.display().to_string(),
        &roots.display().to_string(),
        &plans.display().to_string(),
    );
    let leases = std::fs::read_to_string(state.join("placement-actual.json")).unwrap();
    let garbage = plans.join("plan-unix1-deadbeef.json");
    std::fs::write(&garbage, "not-json\n").unwrap();

    let apply = estate_bin()
        .args([
            "apply",
            "--require-plan",
            "--estate",
            &estate,
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &roots.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let body = text(&apply);
    assert!(!apply.status.success(), "{body}");
    assert!(
        !body.contains("no plan on disk"),
        "unreadable plan must not look like empty: {body}"
    );
    assert_eq!(
        std::fs::read_to_string(state.join("placement-actual.json")).unwrap(),
        leases,
        "garbage plan must not restamp leases"
    );
    assert_eq!(std::fs::read_to_string(&garbage).unwrap(), "not-json\n");

    let status = estate_bin()
        .args([
            "status",
            "--estate",
            &estate,
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &roots.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let body = text(&status);
    assert!(!status.status.success(), "{body}");
    assert!(
        !body.contains("Cell One status")
            || body.contains("not-json")
            || body.contains("plan-unix1"),
        "status must refuse garbage plan before inventing last-plan: {body}"
    );
    assert_eq!(std::fs::read_to_string(&garbage).unwrap(), "not-json\n");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn garbage_feed_cursor_is_refuse_not_empty() {
    let root = repo_root().join(format!(
        "target/test-honesty-cursor-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let feed = root.join("feed");
    std::fs::create_dir_all(&feed).unwrap();
    let cursor = feed.join("feed-cursor.json");
    std::fs::write(&cursor, "not-json\n").unwrap();

    let out = estate_bin()
        .args(["feed", "cursor", "--feed-dir", &feed.display().to_string()])
        .output()
        .unwrap();
    let body = text(&out);
    assert!(!out.status.success(), "{body}");
    assert!(
        body.contains("feed-cursor.json") || body.contains("parse"),
        "{body}"
    );
    assert!(
        !body.contains("no feed-cursor.json"),
        "present garbage must not look missing: {body}"
    );
    assert_eq!(std::fs::read_to_string(&cursor).unwrap(), "not-json\n");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn apply_force_sku_actual_on_rtx_consumer_writes_consumer_nvidia() {
    let root = repo_root().join(format!(
        "target/test-honesty-force-sku-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let roots = root.join("roots");
    std::fs::create_dir_all(&state).unwrap();
    let estate = fixture("examples/hosts/rtx-consumer.yaml");
    apply_cell(
        &estate,
        &state.display().to_string(),
        &roots.display().to_string(),
        &plans.display().to_string(),
    );

    let leases_path = state.join("placement-actual.json");
    let mut places: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&leases_path).unwrap()).unwrap();
    let box_lease = places["leases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["placement_id"] == "cell-one-box")
        .expect("cell-one-box");
    assert_eq!(
        box_lease["host_class"], "consumer-nvidia",
        "rtx_consumer alias must stamp consumer-nvidia, not any"
    );

    let leases = places["leases"].as_array_mut().expect("leases");
    let box_lease = leases
        .iter_mut()
        .find(|l| l["placement_id"] == "cell-one-box")
        .expect("cell-one-box");
    box_lease["host_class"] = serde_json::Value::String("rtx-5090".into());
    let tampered = serde_json::to_string_pretty(&places).unwrap();
    std::fs::write(&leases_path, &tampered).unwrap();

    let blocked = estate_bin()
        .args([
            "apply",
            "--estate",
            &estate,
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &roots.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let body = text(&blocked);
    assert!(!blocked.status.success(), "{body}");
    assert!(body.contains("refuse:bad-host-class"), "{body}");
    assert_eq!(
        std::fs::read_to_string(&leases_path).unwrap(),
        tampered,
        "without --force, SKU actual must stay on disk"
    );

    let dry = estate_bin()
        .args([
            "apply",
            "--dry-run",
            "--force",
            "--estate",
            &estate,
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &roots.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let body = text(&dry);
    assert!(
        body.contains("host_class=consumer-nvidia"),
        "dry-run --force must preview estate class, not SKU→any: {body}"
    );
    assert!(
        !body.contains("host_class=rtx-5090") || body.contains("consumer-nvidia"),
        "{body}"
    );
    assert_eq!(
        std::fs::read_to_string(&leases_path).unwrap(),
        tampered,
        "dry-run --force must not rewrite SKU actual"
    );

    let forced = estate_bin()
        .args([
            "apply",
            "--force",
            "--estate",
            &estate,
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &roots.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
    let body = text(&forced);
    assert!(forced.status.success(), "{body}");
    let after = std::fs::read_to_string(&leases_path).unwrap();
    let written: serde_json::Value = serde_json::from_str(&after).unwrap();
    let box_lease = written["leases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["placement_id"] == "cell-one-box")
        .expect("cell-one-box after force");
    assert_eq!(
        box_lease["host_class"], "consumer-nvidia",
        "apply --force must claim estate host_class, not launder SKU to any: {after}"
    );
    assert!(!after.contains("rtx-5090"), "SKU leftover: {after}");
    let _ = std::fs::remove_dir_all(&root);
}
