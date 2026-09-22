//! Doctor fails a present conveyor mesh that does not parse.
//! A missing mesh is not a failure.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-doctor-mesh-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn run(bin: &str, args: &[&str]) -> (bool, String) {
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
}

#[test]
fn doctor_refuses_an_unreadable_mesh_before_factory_ready() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("mesh");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();

    let (ok, text) = run(
        bin,
        &[
            "doctor",
            "--root",
            &root.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ],
    );
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert!(text.contains("no expired hop leases"), "{text}");
    assert!(text.contains("note  no placement-actual.json"), "{text}");

    let mesh = state.join("conveyor-mesh.json");
    std::fs::write(&mesh, "not-json").unwrap();
    let (ok, text) = run(
        bin,
        &[
            "doctor",
            "--root",
            &root.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ],
    );
    assert!(!ok, "{text}");
    assert!(text.contains("parse:"), "{text}");
    assert!(text.contains("conveyor-mesh.json"), "{text}");
    assert!(!text.contains("note  hop leases"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert_eq!(std::fs::read_to_string(&mesh).unwrap(), "not-json");

    std::fs::write(
        &mesh,
        r#"{"schema":"cell-one.conveyor-mesh.v0","hops":[],"leases":[{"hop_id":"box","kind":"box","capability":"lane-tool","host_class":"rtx-5090","granted":true,"spawned":false,"durable":true,"driver":"box"}]}"#,
    )
    .unwrap();
    let (ok, text) = run(
        bin,
        &[
            "doctor",
            "--root",
            &root.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ],
    );
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:bad-host-class"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(!text.contains("note  hop leases"), "{text}");
}
