//! `estate convey leases` does not print hop lease JSON when a cloud-mesh
//! hop lease is spawned. An unspawned file still prints. A missing mesh
//! still says there are no hop leases. The honesty stack may name the
//! estate's declared cloud placement. That name is not hop lease JSON.

use std::path::PathBuf;
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-convey-leases-spawn-{}-{}",
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

fn mesh(box_spawned: bool, cloud_spawned: bool) -> String {
    format!(
        r#"{{"schema":"cell-one.conveyor-mesh.v0","hops":[{{"id":"cell-one-box","kind":"box","capability":"lane-tool","host_class":"any","wired":true}},{{"id":"cursor-cloud","kind":"cloud-mesh","capability":"mesh-stub","host_class":"any","wired":false}}],"leases":[{{"hop_id":"cell-one-box","kind":"box","capability":"lane-tool","host_class":"any","granted":true,"spawned":{box_spawned},"durable":true,"driver":"box"}},{{"hop_id":"cursor-cloud","kind":"cloud-mesh","capability":"mesh-stub","host_class":"any","granted":false,"spawned":{cloud_spawned},"durable":true,"driver":"cloud-mesh"}}]}}
"#
    )
}

#[test]
fn convey_leases_does_not_print_a_spawned_cloud_hop() {
    let bin = env!("CARGO_BIN_EXE_estate");
    let state = tmp("estate");
    let estate = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/estate.yaml");
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    let leases = [
        "convey",
        "leases",
        "--estate",
        estate_s.as_str(),
        "--state-dir",
        state_s.as_str(),
    ];

    let (ok, text) = run(bin, &leases);
    assert!(ok, "{text}");
    assert!(text.contains("no hop leases"), "{text}");
    assert!(
        !text.contains("\"hop_id\""),
        "a missing mesh is not hop lease JSON: {text}"
    );
    assert!(!state.join("conveyor-mesh.json").exists());

    let path = state.join("conveyor-mesh.json");
    let unspawned = mesh(false, false);
    std::fs::write(&path, &unspawned).unwrap();
    let (ok, text) = run(bin, &leases);
    assert!(ok, "{text}");
    assert!(text.contains("cursor-cloud"), "{text}");
    assert!(
        text.contains("\"spawned\": false") || text.contains("\"spawned\":false"),
        "{text}"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), unspawned);

    let box_up = mesh(true, false);
    std::fs::write(&path, &box_up).unwrap();
    let (ok, text) = run(bin, &leases);
    assert!(ok, "{text}");
    assert!(text.contains("cell-one-box"), "{text}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), box_up);

    let spawned = mesh(false, true);
    std::fs::write(&path, &spawned).unwrap();
    let (ok, text) = run(bin, &leases);
    assert!(!ok, "{text}");
    assert!(
        text.contains("cloud-agent lease spawned (fail closed): cursor-cloud")
            || text.contains("refuse:cloud-spawned"),
        "{text}"
    );
    assert!(
        !text.contains("\"hop_id\""),
        "a spawned cloud hop must not print hop lease JSON: {text}"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), spawned);
}
