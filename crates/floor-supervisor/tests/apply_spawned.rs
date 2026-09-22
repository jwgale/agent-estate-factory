//! `floor apply` and `floor resume` do not rewrite a spawned cloud-agent
//! lease into an unspawned claim.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-floor-apply-spawn-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_floor"))
        .args(args)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

fn placement(cloud_spawned: bool) -> String {
    format!(
        r#"{{"schema":"cell-one.placement-actual.v0","desired_hash":"abc","leases":[{{"placement_id":"cell-one-box","kind":"box","host_class":"any","agents":["horizon","research","sanctum"],"wired":true,"spawned":false,"durable":true}},{{"placement_id":"cursor-cloud","kind":"cloud-agent","host_class":"any","agents":[],"wired":false,"spawned":{cloud_spawned},"durable":true}}]}}
"#
    )
}

#[test]
fn floor_apply_does_not_rewrite_a_spawned_cloud_lease() {
    let root = repo_root();
    let estate = root.join("examples/estate.yaml");
    let dir = tmp("floor");
    let state = dir.join("state");
    let roots = dir.join("roots");
    std::fs::create_dir_all(&state).unwrap();
    let estate_s = estate.display().to_string();
    let state_s = state.display().to_string();
    let roots_s = roots.display().to_string();
    let path = state.join("placement-actual.json");
    let spawned = placement(true);
    std::fs::write(&path, &spawned).unwrap();

    let apply = [
        "apply",
        "--estate",
        estate_s.as_str(),
        "--state-dir",
        state_s.as_str(),
        "--roots-base",
        roots_s.as_str(),
    ];
    let (ok, text) = run(&apply);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:cloud-spawned"), "{text}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), spawned);
    assert!(!state.join("actual-state.json").exists());

    let resume = [
        "resume",
        "--estate",
        estate_s.as_str(),
        "--state-dir",
        state_s.as_str(),
        "--roots-base",
        roots_s.as_str(),
    ];
    let (ok, text) = run(&resume);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:cloud-spawned"), "{text}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), spawned);

    let unspawned = placement(false);
    std::fs::write(&path, &unspawned).unwrap();
    let (ok, text) = run(&apply);
    assert!(ok, "{text}");
    let written = std::fs::read_to_string(&path).unwrap();
    let cloud_at = written
        .find("\"placement_id\": \"cursor-cloud\"")
        .expect("cloud lease");
    let after = &written[cloud_at..];
    let spawned_at = after.find("\"spawned\":").expect("spawned field");
    assert!(
        after[spawned_at..].starts_with("\"spawned\": false"),
        "{written}"
    );
}
