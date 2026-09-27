//! Throwaway prove: mocked classify-journey GGUF → `local_slm` → Standing next.
//! Does not train. Does not rewrite `examples/estate.yaml`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let mut token = std::process::id().to_string();
    for needle in ["5090", "4090", "4080", "3090"] {
        token = token.replace(needle, "0000");
    }
    let path = std::env::temp_dir().join(format!("cell-one-specialty-bind-{name}-{token}"));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn cksum_locked() -> String {
    let out = Command::new("cksum")
        .arg(repo_root().join("examples/estate.yaml"))
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn bind_prove_lands_gguf_as_joinable_local_slm() {
    let before = cksum_locked();
    assert!(
        before.starts_with("43770130 3391"),
        "examples/estate.yaml cksum drifted: {before}"
    );
    let out = scratch("prove");
    let run = bin()
        .args([
            "enrich",
            "bind-prove",
            "--root",
            repo_root().to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ])
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_RENTED_ENDPOINT")
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(run.status.success(), "{text}");
    assert!(text.contains("trained_shape=gguf") || text.contains("\"trained_shape\": \"gguf\""), "{text}");
    assert!(text.contains("auto_apply=false") || text.contains("\"auto_apply\": false"), "{text}");
    assert!(
        text.contains("binding present: local_slm class=local model=specialist-agnews-3000"),
        "{text}"
    );
    assert!(text.contains("Standing next (estate)"), "{text}");
    assert!(text.contains("joinable: yes"), "{text}");
    assert!(text.contains("function: ag_news"), "{text}");
    assert!(text.contains("The seat is joinable."), "{text}");
    assert!(text.contains("result=local_slm"), "{text}");
    assert!(text.contains("capability=local_slm"), "{text}");
    assert!(text.contains("specialty_join: joinable=yes"), "{text}");
    assert!(text.contains("cell-one.specialty-bind-prove.v0"), "{text}");
    assert!(text.contains("specialty-bind-prove: ok"), "{text}");
    assert!(text.contains("READY_FOR_LIVE_TEST: no"), "{text}");
    assert!(!text.contains("READY_FOR_LIVE_TEST: yes"), "{text}");
    assert!(!text.contains("live PASS recorded"), "{text}");
    let after = cksum_locked();
    assert_eq!(after, before, "examples/estate.yaml cksum changed");
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn bind_prove_dual_lands_two_specialties_beside_local_slm() {
    let before = cksum_locked();
    assert!(
        before.starts_with("43770130 3391"),
        "examples/estate.yaml cksum drifted: {before}"
    );
    let out = scratch("dual");
    let run = bin()
        .args([
            "enrich",
            "bind-prove",
            "--dual",
            "--root",
            repo_root().to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ])
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_RENTED_ENDPOINT")
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(run.status.success(), "{text}");
    assert!(text.contains("specialty-join recorded: binding=ag_news"), "{text}");
    assert!(text.contains("specialty-join recorded: binding=rust_idiom"), "{text}");
    assert!(
        text.contains("binding present: ag_news class=local model=specialist-agnews-3000"),
        "{text}"
    );
    assert!(
        text.contains("binding present: rust_idiom class=local model=specialist-rustidiom-3000"),
        "{text}"
    );
    assert!(text.contains("local_slm kept"), "{text}");
    assert!(text.contains("lab intention: research model ag_news effect=allow"), "{text}");
    assert!(text.contains("lab intention: idiom model rust_idiom effect=allow"), "{text}");
    assert!(text.contains("joinable: yes"), "{text}");
    assert!(text.contains("binding: ag_news"), "{text}");
    assert!(text.contains("binding: rust_idiom"), "{text}");
    assert!(text.contains("The seat is joinable."), "{text}");
    assert!(text.contains("result=ag_news"), "{text}");
    assert!(text.contains("result=rust_idiom"), "{text}");
    assert!(text.contains("capability=ag_news"), "{text}");
    assert!(text.contains("capability=rust_idiom"), "{text}");
    assert!(text.contains("result=abstain"), "{text}");
    assert!(text.contains("candidates=ag_news,rust_idiom") || text.contains("candidates=rust_idiom,ag_news"), "{text}");
    assert!(text.contains("cell-one.specialty-bind-prove-dual.v0"), "{text}");
    assert!(text.contains("specialty-bind-prove-dual: ok"), "{text}");
    assert!(text.contains("\"ready_for_live_test\": false"), "{text}");
    assert!(text.contains("\"live_pass_recorded\": false"), "{text}");
    assert!(text.contains("43770130 3391"), "{text}");
    assert!(text.contains("READY_FOR_LIVE_TEST: no"), "{text}");
    assert!(!text.contains("READY_FOR_LIVE_TEST: yes"), "{text}");
    assert!(!text.contains("live PASS recorded"), "{text}");
    assert!(!text.contains("\"ok\": false"), "{text}");
    let after = cksum_locked();
    assert_eq!(after, before, "examples/estate.yaml cksum changed");
    let _ = fs::remove_dir_all(&out);
}
