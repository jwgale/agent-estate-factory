//! Convey hop TTL: declare with real ttl_secs → expire lists → call refuses
//! → expire --forget keeps hop decls → call restamps (lease-refresh).
//! No JSON mutation. Fixtures only. Cloud never spawned.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn estate_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
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
fn hop_ttl_expire_forget_call_restamps() {
    let root = repo_root().join(format!("target/test-hop-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let state_s = state.display().to_string();

    let hop = estate_bin()
        .args([
            "convey",
            "hop",
            "--id",
            "ttl-hop",
            "--capability",
            "lane-tool",
            "--ttl-secs",
            "1",
            "--state-dir",
            &state_s,
        ])
        .output()
        .unwrap();
    assert!(hop.status.success(), "{}", text(&hop));
    let mesh_path = state.join("conveyor-mesh.json");
    let mesh: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&mesh_path).unwrap()).unwrap();
    let lease = mesh["leases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["hop_id"] == "ttl-hop")
        .unwrap();
    assert_eq!(lease["ttl_secs"], 1);
    let max_exp = lease["expires_at"].as_u64().expect("expires_at stamped");
    assert!(max_exp > 0);

    while now_unix() < max_exp {
        std::thread::sleep(Duration::from_millis(50));
    }

    let listed = estate_bin()
        .args(["convey", "expire", "--state-dir", &state_s])
        .output()
        .unwrap();
    let listed_text = text(&listed);
    assert!(!listed.status.success(), "{listed_text}");
    assert!(listed_text.contains("refuse:expired"), "{listed_text}");
    assert!(listed_text.contains("ttl-hop"), "{listed_text}");

    let refused = estate_bin()
        .args([
            "convey",
            "call",
            "--id",
            "ttl-hop",
            "--capability",
            "lane-tool",
            "--state-dir",
            &state_s,
        ])
        .output()
        .unwrap();
    let refused_text = text(&refused);
    assert!(!refused.status.success(), "{refused_text}");
    assert!(refused_text.contains("refuse:expired"), "{refused_text}");
    assert!(
        !refused_text.contains("refuse:no-lease"),
        "expired call must not hide behind no-lease: {refused_text}"
    );

    let forgot = estate_bin()
        .args(["convey", "expire", "--forget", "--state-dir", &state_s])
        .output()
        .unwrap();
    let forgot_text = text(&forgot);
    assert!(forgot.status.success(), "{forgot_text}");
    assert!(forgot_text.contains("forgot"), "{forgot_text}");
    let after_forget: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&mesh_path).unwrap()).unwrap();
    assert!(
        after_forget["hops"]
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["id"] == "ttl-hop"),
        "forget must keep hop decls: {after_forget}"
    );
    assert!(
        after_forget["leases"]
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "forget must drop expired hop leases: {after_forget}"
    );

    let fresh = estate_bin()
        .args([
            "convey",
            "call",
            "--id",
            "ttl-hop",
            "--capability",
            "lane-tool",
            "--state-dir",
            &state_s,
        ])
        .output()
        .unwrap();
    let fresh_text = text(&fresh);
    assert!(fresh.status.success(), "{fresh_text}");
    assert!(fresh_text.contains("lease-refresh"), "{fresh_text}");
    assert!(
        !fresh_text.contains("refuse:no-lease"),
        "forget then call must restamp: {fresh_text}"
    );
    let restamped: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&mesh_path).unwrap()).unwrap();
    let row = restamped["leases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["hop_id"] == "ttl-hop")
        .unwrap();
    let issued = row["issued_at"].as_u64().expect("fresh issued_at");
    let exp = row["expires_at"].as_u64().expect("fresh expires_at");
    assert_eq!(exp, issued + 1, "{row}");
    assert_eq!(row["ttl_secs"], 1);
    assert!(!state.join("placement-actual.json").exists());
    let _ = std::fs::remove_dir_all(&root);
}
