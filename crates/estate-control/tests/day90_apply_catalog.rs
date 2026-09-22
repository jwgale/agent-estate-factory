//! Live apply refuses a cell catalog that disagrees with the binding
//! before any write. The schema card is not the binding.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-mixed-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn apply_refuses_a_cell_catalog_that_disagrees_with_the_binding_before_writing() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("apply-catalog-mismatch");
    let state = dir.join("state");
    let plans = dir.join("plans");
    std::fs::create_dir_all(&state).unwrap();
    let estate = root.join("examples/estate.yaml");

    let run = |args: &[&str]| {
        let out = Command::new(bin)
            .args(args)
            .env_remove("XAI_API_KEY")
            .env_remove("CELL_FRONTIER_ENDPOINT")
            .env_remove("CELL_LOCAL_ENDPOINT")
            .env("CELL_FRONTIER_MODEL", "grok-4.7")
            .output()
            .unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status.success(), text)
    };

    let (ok, text) = run(&[
        "apply",
        "--estate",
        &estate.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--roots-base",
        &state.display().to_string(),
        "--plans-dir",
        &plans.display().to_string(),
    ]);
    assert!(ok, "{text}");
    let catalog_path = state.join("catalog.json");
    let catalog = std::fs::read_to_string(&catalog_path).unwrap();
    assert!(
        catalog.contains("\"model\": \"\""),
        "unbound apply must leave the frontier model empty: {catalog}"
    );
    let actual_before = std::fs::read(state.join("actual-state.json")).unwrap();
    let snap_before = std::fs::read(state.join("desired-snapshot.yaml")).unwrap();
    let place_before = std::fs::read(state.join("placement-actual.json")).unwrap();

    let tampered = catalog.replacen("\"model\": \"\"", "\"model\": \"grok-4.7\"", 1);
    std::fs::write(&catalog_path, &tampered).unwrap();
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let cases: [&[&str]; 3] = [
        &[
            "apply",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
            "--roots-base",
            &state_s,
            "--plans-dir",
            &plans_s,
        ],
        &[
            "apply",
            "--force",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
            "--roots-base",
            &state_s,
            "--plans-dir",
            &plans_s,
        ],
        &[
            "apply",
            "--dry-run",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
            "--roots-base",
            &state_s,
            "--plans-dir",
            &plans_s,
        ],
    ];
    for args in cases {
        let (ok, text) = run(args);
        assert!(!ok, "{text}");
        assert!(text.contains("refuse:frontier-model"), "{text}");
        assert!(text.contains("cell catalog model=grok-4.7"), "{text}");
        assert!(text.contains("binding model=-"), "{text}");
        assert!(text.contains("schema card is not the binding"), "{text}");
        assert!(
            !text.contains("applied "),
            "refused apply must not report a write: {text}"
        );
        let still = std::fs::read_to_string(&catalog_path).unwrap();
        assert!(
            still.contains("\"model\": \"grok-4.7\""),
            "refused apply must not rewrite the catalog: {still}"
        );
        assert_eq!(
            std::fs::read(state.join("actual-state.json")).unwrap(),
            actual_before
        );
        assert_eq!(
            std::fs::read(state.join("desired-snapshot.yaml")).unwrap(),
            snap_before
        );
        assert_eq!(
            std::fs::read(state.join("placement-actual.json")).unwrap(),
            place_before
        );
    }

    std::fs::write(&catalog_path, "not-json").unwrap();
    let (ok, text) = run(&[
        "apply",
        "--estate",
        &estate.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--roots-base",
        &state.display().to_string(),
        "--plans-dir",
        &plans.display().to_string(),
    ]);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:frontier-model"), "{text}");
    assert!(text.contains("unreadable"), "{text}");
    assert_eq!(std::fs::read_to_string(&catalog_path).unwrap(), "not-json");
    assert_eq!(
        std::fs::read(state.join("actual-state.json")).unwrap(),
        actual_before
    );

    std::fs::write(&catalog_path, &catalog).unwrap();
    let (ok, text) = run(&[
        "apply",
        "--estate",
        &estate.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--roots-base",
        &state.display().to_string(),
        "--plans-dir",
        &plans.display().to_string(),
    ]);
    assert!(ok, "{text}");
    assert!(!text.contains("refuse:frontier-model"), "{text}");
    let restored = std::fs::read_to_string(&catalog_path).unwrap();
    assert!(
        !restored.contains("grok-4.7"),
        "a matching unbound apply must not copy the schema card: {restored}"
    );

    let mixed = root.join("examples/fixtures/mixed-frontier-local.yaml");
    let mixed_state = dir.join("mixed");
    let mixed_plans = dir.join("mixed-plans");
    let (ok, plan_text) = run(&[
        "plan",
        "--estate",
        &mixed.display().to_string(),
        "--plans-dir",
        &mixed_plans.display().to_string(),
        "--state-dir",
        &mixed_state.display().to_string(),
    ]);
    assert!(ok, "{plan_text}");
    let (ok, apply_text) = run(&[
        "apply",
        "--require-plan",
        "--estate",
        &mixed.display().to_string(),
        "--state-dir",
        &mixed_state.display().to_string(),
        "--roots-base",
        &mixed_state.display().to_string(),
        "--plans-dir",
        &mixed_plans.display().to_string(),
    ]);
    assert!(ok, "{apply_text}");
    let mixed_catalog = mixed_state.join("catalog.json");
    let body = std::fs::read_to_string(&mixed_catalog).unwrap();
    assert!(body.contains("\"model\": \"grok-4.7\""), "{body}");
    let swapped = body.replacen("\"model\": \"grok-4.7\"", "\"model\": \"other-model\"", 1);
    std::fs::write(&mixed_catalog, &swapped).unwrap();
    let (ok, text) = run(&[
        "apply",
        "--require-plan",
        "--estate",
        &mixed.display().to_string(),
        "--state-dir",
        &mixed_state.display().to_string(),
        "--roots-base",
        &mixed_state.display().to_string(),
        "--plans-dir",
        &mixed_plans.display().to_string(),
    ]);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:frontier-model"), "{text}");
    assert!(text.contains("cell catalog model=other-model"), "{text}");
    assert!(text.contains("binding model=grok-4.7"), "{text}");
    assert_eq!(std::fs::read_to_string(&mixed_catalog).unwrap(), swapped);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn status_and_doctor_refuse_an_unreadable_cell_catalog() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("catalog-unreadable");
    let state = dir.join("state");
    let plans = dir.join("plans");
    std::fs::create_dir_all(&state).unwrap();
    let estate = root.join("examples/estate.yaml");

    let run = |args: &[&str]| {
        let out = Command::new(bin)
            .args(args)
            .env_remove("XAI_API_KEY")
            .env_remove("CELL_FRONTIER_ENDPOINT")
            .env_remove("CELL_LOCAL_ENDPOINT")
            .env_remove("CELL_FRONTIER_MODEL")
            .output()
            .unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status.success(), text)
    };

    let (ok, text) = run(&[
        "apply",
        "--estate",
        &estate.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--roots-base",
        &state.display().to_string(),
        "--plans-dir",
        &plans.display().to_string(),
    ]);
    assert!(ok, "{text}");
    let catalog_path = state.join("catalog.json");
    std::fs::write(&catalog_path, "not-json").unwrap();

    let (ok, text) = run(&[
        "status",
        "--estate",
        &estate.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--roots-base",
        &state.display().to_string(),
        "--plans-dir",
        &plans.display().to_string(),
        "--root",
        &root.display().to_string(),
    ]);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:frontier-model"), "{text}");
    assert!(text.contains("unreadable"), "{text}");
    assert!(text.contains("schema card is not the binding"), "{text}");
    assert!(text.contains("doctor: FAIL"), "{text}");
    assert!(
        text.contains("catalog frontier: schema model=grok-4.7"),
        "{text}"
    );
    assert!(
        !text.contains("catalog frontier: cell"),
        "status must refuse before the cell catalog line: {text}"
    );
    assert!(
        !text.contains("in_sync:"),
        "status must refuse before the success lines: {text}"
    );
    assert_eq!(std::fs::read_to_string(&catalog_path).unwrap(), "not-json");

    let (ok, doc) = run(&[
        "doctor",
        "--root",
        &root.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
    ]);
    assert!(!ok, "{doc}");
    assert!(doc.contains("refuse:frontier-model"), "{doc}");
    assert!(doc.contains("unreadable"), "{doc}");
    assert!(doc.contains("schema card is not the binding"), "{doc}");
    assert!(
        !doc.contains("note  catalog.json frontier model unreadable"),
        "{doc}"
    );
    assert!(!doc.contains("factory ready"), "{doc}");
    assert_eq!(std::fs::read_to_string(&catalog_path).unwrap(), "not-json");

    std::fs::remove_file(&catalog_path).unwrap();
    let (ok, text) = run(&[
        "status",
        "--estate",
        &estate.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--roots-base",
        &state.display().to_string(),
        "--plans-dir",
        &plans.display().to_string(),
        "--root",
        &root.display().to_string(),
    ]);
    assert!(ok, "{text}");
    assert!(!text.contains("refuse:frontier-model"), "{text}");
    assert!(text.contains("doctor: ok"), "{text}");
    let (ok, doc) = run(&[
        "doctor",
        "--root",
        &root.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
    ]);
    assert!(ok, "{doc}");
    assert!(!doc.contains("refuse:frontier-model"), "{doc}");
    let _ = std::fs::remove_dir_all(&dir);
}
