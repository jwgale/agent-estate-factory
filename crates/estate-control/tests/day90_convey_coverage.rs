//! `estate convey` enforces placement-derived hop coverage.
//! Allow still calls. Empty population, explicit deny, and cloud hop refuse.
//! Plan stays print-only. Does not spawn. Not part of smoke or gate-90.

use std::path::PathBuf;
use std::process::Command;

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

fn fixture_estate(
    root: &std::path::Path,
    name: &str,
    mutate: impl FnOnce(&mut estate_schema::Estate),
) -> PathBuf {
    let mut estate =
        estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
    mutate(&mut estate);
    let path = root.join(name);
    std::fs::write(&path, estate_schema::render_estate_yaml(&estate).unwrap()).unwrap();
    path
}

fn convey(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut cmd = estate_bin();
    cmd.current_dir(root);
    cmd.args(args);
    cmd.output().unwrap()
}

#[test]
fn convey_fails_closed_on_hop_coverage() {
    let root = repo_root().join(format!(
        "target/test-convey-coverage-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let allow_estate = fixture_estate(&root, "allow.yaml", |estate| {
        estate
            .placements
            .iter_mut()
            .find(|p| p.id == "cell-one-box")
            .unwrap()
            .agents = vec!["research".into()];
        estate
            .agents
            .iter_mut()
            .find(|a| a.id == "research")
            .unwrap()
            .tools
            .push(estate_schema::ToolDecl {
                id: "lane-tool".into(),
                description: None,
            });
        estate.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "lane-tool".into(),
            kind: estate_schema::IntentionKind::Tool,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
    });
    let allow_state = root.join("allow-state");
    let hop = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "cell-one-box",
            "--capability",
            "lane-tool",
            "--agent",
            "research",
            "--estate",
            &allow_estate.display().to_string(),
            "--state-dir",
            &allow_state.display().to_string(),
        ],
    );
    assert!(hop.status.success(), "{}", text(&hop));
    let call = convey(
        &root,
        &[
            "convey",
            "call",
            "--id",
            "cell-one-box",
            "--capability",
            "lane-tool",
            "--agent",
            "research",
            "--estate",
            &allow_estate.display().to_string(),
            "--state-dir",
            &allow_state.display().to_string(),
            "--policy",
            &repo_root()
                .join("examples/fixtures/policy-allow.yaml")
                .display()
                .to_string(),
        ],
    );
    let call_text = text(&call);
    assert!(call.status.success(), "{call_text}");
    assert!(call_text.contains("research"), "{call_text}");

    let empty_estate = fixture_estate(&root, "empty.yaml", |estate| {
        estate
            .placements
            .iter_mut()
            .find(|p| p.id == "cell-one-box")
            .unwrap()
            .agents
            .clear();
    });
    let empty = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "cell-one-box",
            "--capability",
            "lane-tool",
            "--estate",
            &empty_estate.display().to_string(),
            "--state-dir",
            &root.join("empty-state").display().to_string(),
        ],
    );
    let empty_text = text(&empty);
    assert!(!empty.status.success(), "{empty_text}");
    assert!(
        empty_text.contains("refuse:hop-coverage") && empty_text.contains("(deny-default)"),
        "{empty_text}"
    );
    assert!(!empty_text.contains("(deny)"), "{empty_text}");
    assert!(!root.join("empty-state").join("conveyor-mesh.json").exists());

    let deny_estate = fixture_estate(&root, "deny.yaml", |estate| {
        estate
            .placements
            .iter_mut()
            .find(|p| p.id == "cell-one-box")
            .unwrap()
            .agents = vec!["research".into()];
        estate
            .agents
            .iter_mut()
            .find(|a| a.id == "research")
            .unwrap()
            .tools
            .push(estate_schema::ToolDecl {
                id: "lane-tool".into(),
                description: None,
            });
        estate.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "lane-tool".into(),
            kind: estate_schema::IntentionKind::Tool,
            effect: estate_schema::Effect::Deny,
            note: None,
        });
    });
    let denied = convey(
        &root,
        &[
            "convey",
            "call",
            "--id",
            "cell-one-box",
            "--capability",
            "lane-tool",
            "--agent",
            "research",
            "--estate",
            &deny_estate.display().to_string(),
            "--state-dir",
            &root.join("deny-state").display().to_string(),
            "--policy",
            &repo_root()
                .join("examples/fixtures/policy-allow.yaml")
                .display()
                .to_string(),
        ],
    );
    let denied_text = text(&denied);
    assert!(!denied.status.success(), "{denied_text}");
    assert!(
        denied_text.contains("refuse:hop-coverage") && denied_text.contains("(deny)"),
        "{denied_text}"
    );
    assert!(!denied_text.contains("deny-default"), "{denied_text}");

    let cloud = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "cursor-cloud",
            "--kind",
            "cloud-mesh",
            "--capability",
            "mesh-stub",
            "--estate",
            &repo_root()
                .join("examples/estate.yaml")
                .display()
                .to_string(),
            "--state-dir",
            &root.join("cloud-state").display().to_string(),
        ],
    );
    let cloud_text = text(&cloud);
    assert!(!cloud.status.success(), "{cloud_text}");
    assert!(
        cloud_text.contains("refuse:hop-coverage") && cloud_text.contains("(deny)"),
        "{cloud_text}"
    );
    assert!(!cloud_text.contains("deny-default"), "{cloud_text}");
    assert!(!cloud_text.to_ascii_lowercase().contains("spawned a cloud"));

    let cloud_named = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "cursor-cloud",
            "--kind",
            "cloud-mesh",
            "--capability",
            "mesh-stub",
            "--agent",
            "research",
            "--estate",
            &repo_root()
                .join("examples/estate.yaml")
                .display()
                .to_string(),
            "--state-dir",
            &root.join("cloud-named-state").display().to_string(),
        ],
    );
    let cloud_named_text = text(&cloud_named);
    assert!(!cloud_named.status.success(), "{cloud_named_text}");
    assert!(
        cloud_named_text.contains("refuse:hop-coverage") && cloud_named_text.contains("(deny)"),
        "{cloud_named_text}"
    );
    assert!(
        !cloud_named_text.contains("deny-default"),
        "empty cloud population with --agent must be deny, got {cloud_named_text}"
    );
    assert!(!root.join("cloud-named-state").join("conveyor-mesh.json").exists());

    let pair_state = root.join("pair-state");
    let pair = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "cell-one-box",
            "--capability",
            "lane-tool",
            "--agent",
            "research",
            "--agent",
            "horizon",
            "--estate",
            &allow_estate.display().to_string(),
            "--state-dir",
            &pair_state.display().to_string(),
        ],
    );
    let pair_text = text(&pair);
    assert!(!pair.status.success(), "{pair_text}");
    assert!(
        pair_text.contains("refuse:hop-coverage") && pair_text.contains("(deny-default)"),
        "each --agent is checked; horizon stays deny-default, got {pair_text}"
    );
    assert!(pair_text.contains("horizon"), "{pair_text}");
    assert!(!pair_state.join("conveyor-mesh.json").exists());

    let plan = convey(
        &root,
        &[
            "plan",
            "--estate",
            &repo_root()
                .join("examples/estate.yaml")
                .display()
                .to_string(),
            "--plans-dir",
            &root.join("plans").display().to_string(),
            "--state-dir",
            &root.join("plan-state").display().to_string(),
        ],
    );
    let plan_text = text(&plan);
    assert!(plan.status.success(), "{plan_text}");
    assert!(
        plan_text.contains("cursor-cloud hop mesh-stub: deny"),
        "{plan_text}"
    );
    assert!(
        plan_text.contains("horizon hop cell-one-box lane-tool: deny-default"),
        "{plan_text}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
