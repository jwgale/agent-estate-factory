//! Placement TTL operator loop on an isolated cell.
//! apply (real ttl_secs) → expire lists → apply refuses → expire --forget → re-apply.
//! Fixtures only. No JSON mutation. No live Grok / Mac / GPU. Cloud never spawned.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[test]
fn apply_short_ttl_expire_forget_reapply() {
    let root = repo_root().join(format!("target/test-ttl-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let estate = fixture("examples/fixtures/ttl-short.yaml");
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let roots_s = root.display().to_string();

    let plan = estate_bin()
        .args([
            "plan",
            "--estate",
            &estate,
            "--plans-dir",
            &plans_s,
            "--state-dir",
            &state_s,
        ])
        .output()
        .unwrap();
    assert!(plan.status.success(), "{}", text(&plan));

    let apply = estate_bin()
        .args([
            "apply",
            "--estate",
            &estate,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots_s,
            "--plans-dir",
            &plans_s,
        ])
        .output()
        .unwrap();
    assert!(apply.status.success(), "{}", text(&apply));
    let leases_path = state.join("placement-actual.json");
    assert!(leases_path.is_file());
    let leases: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&leases_path).unwrap()).unwrap();
    let mut max_exp = 0u64;
    for lease in leases["leases"].as_array().unwrap() {
        assert_eq!(lease["ttl_secs"], 1);
        let exp = lease["expires_at"].as_u64().expect("expires_at stamped");
        max_exp = max_exp.max(exp);
    }
    assert!(max_exp > 0, "apply must stamp expires_at from ttl_secs");

    while now_unix() < max_exp {
        std::thread::sleep(Duration::from_millis(50));
    }

    let listed = estate_bin()
        .args(["expire", "--state-dir", &state_s])
        .output()
        .unwrap();
    let listed_text = text(&listed);
    assert!(!listed.status.success(), "{listed_text}");
    assert!(listed_text.contains("refuse:expired"), "{listed_text}");
    assert!(listed_text.contains("cell-one-box"), "{listed_text}");

    let refused = estate_bin()
        .args([
            "apply",
            "--estate",
            &estate,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots_s,
            "--plans-dir",
            &plans_s,
        ])
        .output()
        .unwrap();
    let refused_text = text(&refused);
    assert!(!refused.status.success(), "{refused_text}");
    assert!(refused_text.contains("refuse:expired"), "{refused_text}");
    assert!(
        !refused_text.contains("refuse:drift"),
        "expired apply must not hide behind drift: {refused_text}"
    );

    let forgot = estate_bin()
        .args(["expire", "--forget", "--state-dir", &state_s])
        .output()
        .unwrap();
    let forgot_text = text(&forgot);
    assert!(forgot.status.success(), "{forgot_text}");
    assert!(forgot_text.contains("forgot"), "{forgot_text}");
    let after_forget: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&leases_path).unwrap()).unwrap();
    assert!(
        after_forget["leases"]
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "forget must drop expired rows: {after_forget}"
    );

    let fresh = estate_bin()
        .args([
            "apply",
            "--estate",
            &estate,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots_s,
            "--plans-dir",
            &plans_s,
        ])
        .output()
        .unwrap();
    let fresh_text = text(&fresh);
    assert!(fresh.status.success(), "{fresh_text}");
    assert!(
        fresh_text.contains("lease-refresh") || fresh_text.contains("applied"),
        "{fresh_text}"
    );
    assert!(
        !fresh_text.contains("refuse:drift"),
        "forget then apply must restamp without --force: {fresh_text}"
    );
    let restamped: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&leases_path).unwrap()).unwrap();
    let rows = restamped["leases"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for lease in rows {
        let issued = lease["issued_at"].as_u64().expect("fresh issued_at");
        let exp = lease["expires_at"].as_u64().expect("fresh expires_at");
        assert_eq!(exp, issued + 1, "{lease}");
        assert_eq!(lease["ttl_secs"], 1);
        if lease["kind"] == "cloud-agent" {
            assert_eq!(lease["spawned"], false);
        }
    }

    let estate_bytes = std::fs::read_to_string(&estate).unwrap();
    assert!(
        estate_bytes.contains("ttl_secs: 1"),
        "fixture must keep the short ttl"
    );
    let _ = std::fs::remove_dir_all(&root);
}
