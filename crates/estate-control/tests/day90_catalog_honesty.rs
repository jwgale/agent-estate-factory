//! `estate models` and `estate catalog` do not invent grok-4.7 from the
//! schema card when the binding sets no model.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-catalog-honesty-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
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
}

#[test]
fn models_and_catalog_do_not_invent_grok_when_the_binding_is_unset() {
    let root = repo_root();
    let estate = root.join("examples/estate.yaml");
    let (ok, models) = run(&["models", "--estate", &estate.display().to_string()]);
    assert!(ok, "{models}");
    assert!(
        models.contains("model=-"),
        "unset params.model stays model=-: {models}"
    );
    assert!(
        !models.contains("model=grok-4.7"),
        "models must not copy the schema card when the binding is unset: {models}"
    );
    assert!(
        !models.contains("CELL_FRONTIER_MODEL"),
        "models must not treat the env default as the binding: {models}"
    );

    let dir = tmp("schema");
    let side = dir.join("schema-catalog.json");
    let (ok, catalog) = run(&[
        "catalog",
        "--out",
        &side.display().to_string(),
    ]);
    assert!(ok, "{catalog}");
    assert!(
        catalog.contains("schema card, not a binding"),
        "{catalog}"
    );
    assert!(catalog.contains("model=grok-4.7"), "{catalog}");
    assert!(
        !catalog.contains("binding model=grok-4.7"),
        "schema dump must not claim a binding: {catalog}"
    );
    let side_body = std::fs::read_to_string(&side).unwrap();
    assert!(
        side_body.contains("\"model\": \"grok-4.7\""),
        "the schema file still names the card: {side_body}"
    );

    let state = dir.join("state");
    let plans = dir.join("plans");
    std::fs::create_dir_all(&state).unwrap();
    let (ok, applied) = run(&[
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
    assert!(ok, "{applied}");
    let cell = state.join("catalog.json");
    let before = std::fs::read_to_string(&cell).unwrap();
    assert!(
        before.contains("\"model\": \"\""),
        "unbound apply must leave the frontier model empty: {before}"
    );
    assert!(!before.contains("grok-4.7"), "{before}");
    let actual_before = std::fs::read(state.join("actual-state.json")).unwrap();

    let (ok, refused) = run(&["catalog", "--out", &cell.display().to_string()]);
    assert!(!ok, "{refused}");
    assert!(refused.contains("refuse:frontier-model"), "{refused}");
    assert!(refused.contains("catalog model=-"), "{refused}");
    assert!(refused.contains("schema card model=grok-4.7"), "{refused}");
    assert!(
        refused.contains("schema card is not the binding"),
        "{refused}"
    );
    assert!(
        !refused.contains("wrote catalog file"),
        "refused catalog must not report a write: {refused}"
    );
    assert_eq!(std::fs::read_to_string(&cell).unwrap(), before);
    assert_eq!(
        std::fs::read(state.join("actual-state.json")).unwrap(),
        actual_before
    );

    let (ok, again) = run(&["catalog", "--out", &side.display().to_string()]);
    assert!(ok, "{again}");
    assert!(
        std::fs::read_to_string(&side)
            .unwrap()
            .contains("\"model\": \"grok-4.7\"")
    );
    let _ = std::fs::remove_dir_all(&dir);
}
