use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn floor_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_floor"))
}

fn status(estate: &str, state: &str) -> std::process::Output {
    floor_bin()
        .args(["status", "--estate", estate, "--state-dir", state])
        .output()
        .unwrap()
}

#[test]
fn status_does_not_invent_suspended_for_a_missing_lifecycle() {
    let tmp = repo_root().join(format!(
        "target/test-floor-lifecycle-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let state = tmp.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let estate = repo_root().join("examples/estate.yaml");
    let estate = estate.display().to_string();
    let state_s = state.display().to_string();

    let missing = status(&estate, &state_s);
    let text = String::from_utf8_lossy(&missing.stdout);
    assert_eq!(missing.status.code(), Some(2), "{text}");
    assert!(text.contains("lifecycle: -"), "{text}");
    assert!(!text.contains("suspended"), "{text}");
    assert!(!text.contains("durable="), "{text}");

    let life = state.join("lifecycle.json");
    std::fs::write(&life, "not-json\n").unwrap();
    let bad = status(&estate, &state_s);
    let bad_out = String::from_utf8_lossy(&bad.stdout);
    let bad_err = String::from_utf8_lossy(&bad.stderr);
    assert!(!bad.status.success(), "{bad_out}{bad_err}");
    assert!(bad_err.contains("lifecycle.json"), "{bad_err}");
    assert!(!bad_out.contains("lifecycle:"), "{bad_out}");
    assert_eq!(std::fs::read_to_string(&life).unwrap(), "not-json\n");

    std::fs::write(
        &life,
        "{\n  \"schema\": \"cell-one.lifecycle.v0\",\n  \"version\": 1,\n  \"state\": \"suspended\",\n  \"desired_hash\": null,\n  \"estate_name\": null,\n  \"suspended_at\": null,\n  \"resumed_at\": null,\n  \"durable\": true,\n  \"note\": \"paused\"\n}\n",
    )
    .unwrap();
    let suspended = status(&estate, &state_s);
    let suspended_text = String::from_utf8_lossy(&suspended.stdout);
    assert_eq!(suspended.status.code(), Some(2), "{suspended_text}");
    assert!(
        suspended_text.contains("lifecycle: suspended durable=true"),
        "{suspended_text}"
    );

    std::fs::write(
        &life,
        "{\n  \"schema\": \"cell-one.lifecycle.v0\",\n  \"version\": 1,\n  \"state\": \"running\",\n  \"desired_hash\": null,\n  \"estate_name\": null,\n  \"suspended_at\": null,\n  \"resumed_at\": null,\n  \"durable\": false,\n  \"note\": \"running\"\n}\n",
    )
    .unwrap();
    let running = status(&estate, &state_s);
    let running_text = String::from_utf8_lossy(&running.stdout);
    assert_eq!(running.status.code(), Some(2), "{running_text}");
    assert!(
        running_text.contains("lifecycle: running durable=false"),
        "{running_text}"
    );
    assert!(
        !running_text.contains("lifecycle: suspended"),
        "{running_text}"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}
