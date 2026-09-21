//! Dual-layer sacred KEEP on the convey path.
//! Overlay file with `locked: []` still refuses hardcoded ids.
//! lab-notebook refuses only when that overlay is installed.
//! Fixtures only. No live Grok / Mac / GPU. Cloud never spawned.

use std::path::PathBuf;
use std::process::Command;

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

fn hop(sacred: &str, id: &str, state: &str) -> std::process::Output {
    estate_bin()
        .args([
            "--sacred",
            sacred,
            "convey",
            "hop",
            "--id",
            id,
            "--capability",
            "lane-tool",
            "--state-dir",
            state,
        ])
        .output()
        .unwrap()
}

#[test]
fn omit_locked_sacred_file_keeps_hardcoded_and_adds_overlay() {
    let root = repo_root().join(format!("target/test-sacred-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let open_state = root.join("open");
    let omit_state = root.join("omit");
    std::fs::create_dir_all(&open_state).unwrap();
    std::fs::create_dir_all(&omit_state).unwrap();
    let open_s = open_state.display().to_string();
    let omit_s = omit_state.display().to_string();
    let omit = fixture("examples/fixtures/sacred-omit-locked.yaml");
    let missing = root.join("no-sacred.yaml").display().to_string();

    let open = hop(&missing, "lab-notebook", &open_s);
    let open_text = text(&open);
    assert!(
        open.status.success(),
        "lab-notebook hop must succeed without overlay: {open_text}"
    );
    assert!(
        !open_text.contains("refuse:sacred-id"),
        "{open_text}"
    );

    let overlay = hop(&omit, "lab-notebook", &omit_s);
    let overlay_text = text(&overlay);
    assert!(!overlay.status.success(), "{overlay_text}");
    assert!(overlay_text.contains("refuse:sacred-id"), "{overlay_text}");

    for locked in ["cyera-ci", "rust-classroom"] {
        let refused = hop(&omit, locked, &omit_s);
        let refused_text = text(&refused);
        assert!(
            !refused.status.success(),
            "omitting locked ids must still refuse {locked}: {refused_text}"
        );
        assert!(
            refused_text.contains("refuse:sacred-id"),
            "{locked}: {refused_text}"
        );
    }

    let allowed = hop(&omit, "research-notes", &omit_s);
    let allowed_text = text(&allowed);
    assert!(
        allowed.status.success(),
        "non-sacred hop must still declare: {allowed_text}"
    );
    assert!(!omit_state.join("placement-actual.json").exists());

    let as_agent = estate_bin()
        .args([
            "--sacred",
            &omit,
            "validate",
            "--estate",
            &fixture("examples/fixtures/refuse-sacred-as-agent.yaml"),
        ])
        .output()
        .unwrap();
    let as_agent_text = text(&as_agent);
    assert!(
        !as_agent.status.success(),
        "omit-locked must still refuse Cyera CI as agent: {as_agent_text}"
    );

    let demo = estate_bin()
        .args([
            "--sacred",
            &omit,
            "validate",
            "--estate",
            &fixture("examples/fixtures/dual-layer-demo.yaml"),
        ])
        .output()
        .unwrap();
    let demo_text = text(&demo);
    assert!(demo.status.success(), "{demo_text}");
    assert!(demo_text.contains("sanctum"), "{demo_text}");

    let omit_bytes = std::fs::read_to_string(&omit).unwrap();
    assert!(
        omit_bytes.contains("locked: []"),
        "fixture must omit locked ids"
    );
    assert!(
        omit_bytes.contains("lab-notebook"),
        "fixture must keep the overlay"
    );
    let _ = std::fs::remove_dir_all(&root);
}
