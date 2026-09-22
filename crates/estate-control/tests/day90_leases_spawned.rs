//! `estate leases` does not print placement JSON when a cloud-agent
//! lease is spawned. An unspawned file still prints. A missing file
//! still says there is no placement-actual.

use std::path::PathBuf;
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-leases-spawn-{}-{}",
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

fn placement(box_spawned: bool, cloud_spawned: bool) -> String {
    format!(
        r#"{{"schema":"cell-one.placement-actual.v0","desired_hash":"abc","leases":[{{"placement_id":"cell-one-box","kind":"box","host_class":"any","agents":["horizon","research","sanctum"],"wired":true,"spawned":{box_spawned},"durable":true}},{{"placement_id":"cursor-cloud","kind":"cloud-agent","host_class":"any","agents":[],"wired":false,"spawned":{cloud_spawned},"durable":true}}]}}
"#
    )
}

#[test]
fn leases_does_not_print_a_spawned_cloud_lease() {
    let bin = env!("CARGO_BIN_EXE_estate");
    let state = tmp("estate");
    let leases = ["leases", "--state-dir", &state.display().to_string()];

    let (ok, text) = run(bin, &leases);
    assert!(ok, "{text}");
    assert!(text.contains("no placement-actual.json"), "{text}");
    assert!(
        !text.contains("cell-one.placement-actual.v0"),
        "{text}"
    );

    let path = state.join("placement-actual.json");
    let unspawned = placement(false, false);
    std::fs::write(&path, &unspawned).unwrap();
    let (ok, text) = run(bin, &leases);
    assert!(ok, "{text}");
    assert!(text.contains("cell-one.placement-actual.v0"), "{text}");
    assert!(text.contains("cursor-cloud"), "{text}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), unspawned);

    let box_up = placement(true, false);
    std::fs::write(&path, &box_up).unwrap();
    let (ok, text) = run(bin, &leases);
    assert!(ok, "{text}");
    assert!(text.contains("cell-one.placement-actual.v0"), "{text}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), box_up);

    let spawned = placement(false, true);
    std::fs::write(&path, &spawned).unwrap();
    let (ok, text) = run(bin, &leases);
    assert!(!ok, "{text}");
    assert!(
        text.contains("cloud-agent lease spawned (fail closed): cursor-cloud"),
        "{text}"
    );
    assert!(
        !text.contains("cell-one.placement-actual.v0"),
        "a spawned cloud lease must not print placement JSON: {text}"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), spawned);
}
