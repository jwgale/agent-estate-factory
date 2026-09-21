//! Tampered conveyor-mesh.json and restore of a SKU placement-actual.
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

fn tamper_mesh_box_host_class(path: &std::path::Path, host_class: &str) -> String {
    let raw = std::fs::read_to_string(path).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    if let Some(hops) = v["hops"].as_array_mut() {
        if let Some(hop) = hops.iter_mut().find(|h| h["id"] == "cell-one-box") {
            hop["host_class"] = serde_json::Value::String(host_class.into());
        }
    }
    if let Some(leases) = v["leases"].as_array_mut() {
        if let Some(lease) = leases.iter_mut().find(|l| l["hop_id"] == "cell-one-box") {
            lease["host_class"] = serde_json::Value::String(host_class.into());
        }
    }
    let next = serde_json::to_string_pretty(&v).unwrap();
    std::fs::write(path, &next).unwrap();
    next
}

fn tamper_placement_box_host_class(path: &std::path::Path, host_class: &str) -> String {
    let raw = std::fs::read_to_string(path).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let leases = v["leases"].as_array_mut().expect("leases");
    let box_lease = leases
        .iter_mut()
        .find(|l| l["placement_id"] == "cell-one-box")
        .expect("cell-one-box");
    box_lease["host_class"] = serde_json::Value::String(host_class.into());
    let next = serde_json::to_string_pretty(&v).unwrap();
    std::fs::write(path, &next).unwrap();
    next
}

#[test]
fn convey_readers_refuse_tampered_mesh_sku_host_class() {
    let root = repo_root().join(format!("target/test-mesh-sku-{}", std::process::id()));
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

    let mesh_path = state.join("conveyor-mesh.json");
    let tampered = tamper_mesh_box_host_class(&mesh_path, "rtx-5090");
    assert!(tampered.contains("rtx-5090"));
    let hops_before = std::fs::read_to_string(state.join("conveyor-hops.json")).unwrap();
    let leases_before = std::fs::read_to_string(state.join("conveyor-leases.json")).unwrap();

    for args in [
        vec!["convey", "call", "--id", "cell-one-box", "--state-dir", &state_s],
        vec!["convey", "list", "--state-dir", &state_s],
        vec!["convey", "leases", "--state-dir", &state_s],
        vec!["convey", "expire", "--state-dir", &state_s],
        vec!["convey", "expire", "--forget", "--state-dir", &state_s],
        vec!["convey", "sync", "--state-dir", &state_s],
    ] {
        let out = estate_bin().args(&args).output().unwrap();
        let body = text(&out);
        assert!(!out.status.success(), "{args:?} {body}");
        assert!(
            body.contains("refuse:bad-host-class"),
            "{args:?} {body}"
        );
        assert_eq!(
            std::fs::read_to_string(&mesh_path).unwrap(),
            tampered,
            "{args:?} must not rewrite mesh"
        );
        assert_eq!(
            std::fs::read_to_string(state.join("conveyor-hops.json")).unwrap(),
            hops_before,
            "{args:?} must not rewrite hops sidecar"
        );
        assert_eq!(
            std::fs::read_to_string(state.join("conveyor-leases.json")).unwrap(),
            leases_before,
            "{args:?} must not rewrite leases sidecar"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn restore_refuses_sku_placement_actual() {
    let root = repo_root().join(format!("target/test-restore-sku-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let roots = root.join("roots");
    let backups = root.join("backups");
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

    let backup = estate_bin()
        .args([
            "backup",
            "--state-dir",
            &state_s,
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
    let archived_places = archive.join("cell").join("placement-actual.json");
    let tampered = tamper_placement_box_host_class(&archived_places, "rtx-5090");
    assert!(tampered.contains("rtx-5090"));

    let dest = root.join("restored");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("placement-actual.json"), "sentinel\n").unwrap();
    let dest_s = dest.display().to_string();
    let archive_s = archive.display().to_string();
    let plans_s = plans.display().to_string();
    let policy = fixture("policy/cell-one.policy.v0.yaml");

    for extra in [vec!["--dry-run"], vec![]] {
        let mut args = vec![
            "restore",
            "--from",
            archive_s.as_str(),
            "--state-dir",
            dest_s.as_str(),
            "--plans-dir",
            plans_s.as_str(),
            "--estate",
            estate.as_str(),
            "--policy",
            policy.as_str(),
        ];
        args.extend(extra.iter().copied());
        let out = estate_bin().args(&args).output().unwrap();
        let body = text(&out);
        assert!(!out.status.success(), "{args:?} {body}");
        assert!(body.contains("refuse:bad-host-class"), "{args:?} {body}");
        assert_eq!(
            std::fs::read_to_string(dest.join("placement-actual.json")).unwrap(),
            "sentinel\n",
            "{args:?} must not write SKU or any"
        );
    }

    let after = std::fs::read_to_string(&archived_places).unwrap();
    assert_eq!(after, tampered, "restore must not rewrite the archive to any");
    assert!(after.contains("rtx-5090"));

    let _ = std::fs::remove_dir_all(&root);
}
