//! Backup and restore refuse a cell catalog that disagrees with the binding,
//! and refuse a frontier source_driver when the estate has no frontier binding.
//! `CELL_FRONTIER_MODEL` is not the binding. No live key.

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

fn quiet(cmd: &mut Command) -> &mut Command {
    cmd.env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env("CELL_FRONTIER_MODEL", "grok-4.7")
}

fn apply_cell(estate: &str, state: &str, roots: &str, plans: &str) {
    let out = quiet(&mut estate_bin())
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
fn backup_quotes_grok_only_on_catalog_disagreement_and_restore_writes_nothing() {
    let root = repo_root().join(format!(
        "target/test-backup-frontier-{}",
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
    let policy = fixture("policy/cell-one.policy.v0.yaml");
    apply_cell(
        &estate,
        &state.display().to_string(),
        &roots.display().to_string(),
        &plans.display().to_string(),
    );
    let catalog_path = state.join("catalog.json");
    let original = std::fs::read_to_string(&catalog_path).unwrap();
    assert!(
        !original.contains("grok-4.7"),
        "unbound cell catalog must not already name grok-4.7: {original}"
    );
    let tampered = original.replacen("\"model\": \"\"", "\"model\": \"grok-4.7\"", 1);
    assert!(tampered.contains("\"model\": \"grok-4.7\""));
    std::fs::write(&catalog_path, &tampered).unwrap();

    let refused = quiet(&mut estate_bin())
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
            &policy,
        ])
        .output()
        .unwrap();
    let refused_text = text(&refused);
    assert!(!refused.status.success(), "{refused_text}");
    assert!(
        refused_text.contains("refuse:frontier-model"),
        "{refused_text}"
    );
    assert!(
        refused_text.contains("cell catalog model=grok-4.7"),
        "{refused_text}"
    );
    assert!(
        refused_text.contains("binding model=-"),
        "{refused_text}"
    );
    assert!(
        !backups.exists() || std::fs::read_dir(&backups).unwrap().next().is_none(),
        "disagreement must not create an archive"
    );
    assert_eq!(std::fs::read_to_string(&catalog_path).unwrap(), tampered);

    std::fs::write(&catalog_path, &original).unwrap();
    let kept = quiet(&mut estate_bin())
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
            &policy,
        ])
        .output()
        .unwrap();
    assert!(kept.status.success(), "{}", text(&kept));
    assert!(!text(&kept).contains("grok-4.7"), "{}", text(&kept));
    let archive = std::fs::read_dir(&backups)
        .unwrap()
        .find_map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_name()?.to_str()?.to_string();
            if name.starts_with("cell-backup-") {
                Some(path)
            } else {
                None
            }
        })
        .expect("archive");
    std::fs::write(
        archive.join("cell").join("catalog.json"),
        &tampered,
    )
    .unwrap();
    let dest = root.join("restored");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("placement-actual.json"), "sentinel\n").unwrap();
    let restored = quiet(&mut estate_bin())
        .args([
            "restore",
            "--from",
            &archive.display().to_string(),
            "--state-dir",
            &dest.display().to_string(),
            "--plans-dir",
            &root.join("restored-plans").display().to_string(),
            "--estate",
            &estate,
            "--policy",
            &policy,
        ])
        .output()
        .unwrap();
    let restored_text = text(&restored);
    assert!(!restored.status.success(), "{restored_text}");
    assert!(
        restored_text.contains("refuse:frontier-model"),
        "{restored_text}"
    );
    assert!(
        restored_text.contains("cell catalog model=grok-4.7"),
        "{restored_text}"
    );
    assert_eq!(
        std::fs::read_to_string(dest.join("placement-actual.json")).unwrap(),
        "sentinel\n"
    );
    assert!(!dest.join("catalog.json").exists());

    let local_estate = root.join("local-only.yaml");
    std::fs::write(
        &local_estate,
        "version: 0\nname: local-only\ndefault_effect: deny\nagents:\n  - id: horizon\n    display_name: Horizon\n    lane: horizon\n    desktop: horizon-desktop\nlanes:\n  - id: horizon\n    root_path: lanes/horizon\n    owner_agent_id: horizon\nmodel_bindings:\n  - id: local_slm\n    class: local\n    driver: ollama\n    wired: true\n",
    )
    .unwrap();
    let local_state = root.join("local-state");
    let pack = local_state.join("feed").join("accepted");
    std::fs::create_dir_all(&pack).unwrap();
    std::fs::write(
        pack.join("pack.json"),
        "{\"source_drivers\":[\"frontier\"]}\n",
    )
    .unwrap();
    let local_out = root.join("local-backups");
    let invented = quiet(&mut estate_bin())
        .args([
            "backup",
            "--state-dir",
            &local_state.display().to_string(),
            "--plans-dir",
            &root.join("local-plans").display().to_string(),
            "--out",
            &local_out.display().to_string(),
            "--estate",
            &local_estate.display().to_string(),
            "--policy",
            &policy,
        ])
        .output()
        .unwrap();
    let invented_text = text(&invented);
    assert!(!invented.status.success(), "{invented_text}");
    assert!(
        invented_text.contains("refuse:frontier-invent"),
        "{invented_text}"
    );
    assert!(
        invented_text.contains("source_driver"),
        "{invented_text}"
    );
    assert!(!invented_text.contains("grok-4.7"), "{invented_text}");
    assert!(!local_out.exists());

    std::fs::write(pack.join("pack.json"), "{\"source_drivers\":[\"local\"]}\n").unwrap();
    let local_kept = quiet(&mut estate_bin())
        .args([
            "backup",
            "--state-dir",
            &local_state.display().to_string(),
            "--plans-dir",
            &root.join("local-plans").display().to_string(),
            "--out",
            &local_out.display().to_string(),
            "--estate",
            &local_estate.display().to_string(),
            "--policy",
            &policy,
        ])
        .output()
        .unwrap();
    let local_kept_text = text(&local_kept);
    assert!(local_kept.status.success(), "{local_kept_text}");
    assert!(!local_kept_text.contains("grok-4.7"), "{local_kept_text}");
    assert!(!local_kept_text.contains("refuse:frontier-invent"), "{local_kept_text}");

    let _ = std::fs::remove_dir_all(&root);
}
