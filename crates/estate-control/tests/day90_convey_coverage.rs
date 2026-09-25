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
        denied_text.contains("refuse:intention") && denied_text.contains("(deny)"),
        "{denied_text}"
    );
    assert!(!denied_text.contains("deny-default"), "{denied_text}");
    assert!(!denied_text.contains("refuse:hop-coverage"), "{denied_text}");

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
        pair_text.contains("refuse:intention") && pair_text.contains("(deny-default)"),
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

#[test]
fn convey_refuses_missing_and_non_file_estate() {
    let root = repo_root().join(format!(
        "target/test-convey-missing-estate-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let missing = root.join("no-such-estate.yaml");
    let missing_s = missing.display().to_string();
    let state_s = state.display().to_string();
    let policy = repo_root()
        .join("examples/fixtures/policy-allow.yaml")
        .display()
        .to_string();

    let hop = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "ttl-box",
            "--capability",
            "lane-tool",
            "--estate",
            &missing_s,
            "--state-dir",
            &state_s,
        ],
    );
    let hop_text = text(&hop);
    assert!(!hop.status.success(), "{hop_text}");
    assert!(
        hop_text.contains("refuse:hop-coverage") && hop_text.contains("estate missing"),
        "{hop_text}"
    );
    assert!(hop_text.contains("coverage is mandatory"), "{hop_text}");
    assert!(!state.join("conveyor-mesh.json").exists());

    let call = convey(
        &root,
        &[
            "convey",
            "call",
            "--id",
            "ttl-box",
            "--capability",
            "lane-tool",
            "--estate",
            &missing_s,
            "--state-dir",
            &state_s,
            "--policy",
            &policy,
        ],
    );
    let call_text = text(&call);
    assert!(!call.status.success(), "{call_text}");
    assert!(
        call_text.contains("refuse:hop-coverage") && call_text.contains("estate missing"),
        "{call_text}"
    );

    let dir = root.join("estate-dir");
    std::fs::create_dir_all(&dir).unwrap();
    let dir_s = dir.display().to_string();
    let non_file = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "ttl-box",
            "--capability",
            "lane-tool",
            "--estate",
            &dir_s,
            "--state-dir",
            &state_s,
        ],
    );
    let non_file_text = text(&non_file);
    assert!(!non_file.status.success(), "{non_file_text}");
    assert!(
        non_file_text.contains("refuse:hop-coverage")
            && non_file_text.contains("estate not a file"),
        "{non_file_text}"
    );
    assert!(!state.join("conveyor-mesh.json").exists());

    let wrong_cwd = root.join("wrong-cwd");
    std::fs::create_dir_all(&wrong_cwd).unwrap();
    let mut cmd = estate_bin();
    cmd.current_dir(&wrong_cwd);
    cmd.args([
        "convey",
        "hop",
        "--id",
        "ttl-box",
        "--capability",
        "lane-tool",
        "--state-dir",
        &state_s,
    ]);
    let skipped = cmd.output().unwrap();
    let skipped_text = text(&skipped);
    assert!(!skipped.status.success(), "{skipped_text}");
    assert!(
        skipped_text.contains("refuse:hop-coverage")
            && skipped_text.contains("examples/estate.yaml")
            && skipped_text.contains("estate missing"),
        "wrong-cwd default must refuse, got {skipped_text}"
    );
    assert!(!state.join("conveyor-mesh.json").exists());

    let plan = convey(
        &root,
        &[
            "plan",
            "--estate",
            &missing_s,
            "--plans-dir",
            &root.join("plans").display().to_string(),
            "--state-dir",
            &root.join("plan-state").display().to_string(),
        ],
    );
    let plan_text = text(&plan);
    assert!(!plan.status.success(), "{plan_text}");
    assert!(
        !plan_text.contains("coverage is mandatory"),
        "plan stays a load error, not the convey coverage refuse: {plan_text}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn convey_lane_prefix_allow_continues_past_intention_gate() {
    let root = repo_root().join(format!(
        "target/test-convey-lane-intention-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let estate = repo_root().join("examples/estate.yaml");
    let estate_s = estate.display().to_string();
    let state = root.join("state");
    let state_s = state.display().to_string();
    let policy = repo_root()
        .join("examples/fixtures/policy-allow.yaml")
        .display()
        .to_string();

    let hop = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "ttl-box",
            "--capability",
            "lane:horizon",
            "--agent",
            "horizon",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
        ],
    );
    let hop_text = text(&hop);
    assert!(
        hop.status.success(),
        "own-lane lane: allow continues past the intention gate: {hop_text}"
    );
    assert!(!hop_text.contains("refuse:intention"), "{hop_text}");
    assert!(state.join("conveyor-mesh.json").exists());

    let call = convey(
        &root,
        &[
            "convey",
            "call",
            "--id",
            "ttl-box",
            "--capability",
            "lane:horizon",
            "--agent",
            "horizon",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
            "--policy",
            &policy,
        ],
    );
    let call_text = text(&call);
    assert!(
        call.status.success(),
        "own-lane lane: call continues past the intention gate: {call_text}"
    );
    assert!(!call_text.contains("refuse:intention"), "{call_text}");

    let crossed = convey(
        &root,
        &[
            "convey",
            "call",
            "--id",
            "ttl-box",
            "--capability",
            "lane:research",
            "--agent",
            "horizon",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
            "--policy",
            &policy,
        ],
    );
    let crossed_text = text(&crossed);
    assert!(!crossed.status.success(), "{crossed_text}");
    assert!(
        crossed_text.contains("refuse:intention") && crossed_text.contains("memory_read"),
        "{crossed_text}"
    );

    let mut ambiguous = estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml"))
        .unwrap();
    ambiguous
        .agents
        .iter_mut()
        .find(|a| a.id == "research")
        .unwrap()
        .mcp
        .push(estate_schema::McpDecl {
            id: "notes-append".into(),
            description: None,
        });
    let ambiguous_path = root.join("ambiguous.yaml");
    std::fs::write(
        &ambiguous_path,
        estate_schema::render_estate_yaml(&ambiguous).unwrap(),
    )
    .unwrap();
    let ambiguous_state = root.join("ambiguous-state");
    let multi = convey(
        &root,
        &[
            "convey",
            "call",
            "--id",
            "ttl-box",
            "--capability",
            "notes-append",
            "--agent",
            "research",
            "--estate",
            &ambiguous_path.display().to_string(),
            "--state-dir",
            &ambiguous_state.display().to_string(),
            "--policy",
            &policy,
        ],
    );
    let multi_text = text(&multi);
    assert!(!multi.status.success(), "{multi_text}");
    assert!(
        multi_text.contains("refuse:intention")
            && multi_text.contains("ambiguous capability; pass kind")
            && multi_text.contains("(deny-default)"),
        "{multi_text}"
    );
    assert!(!multi_text.contains("missing intention"), "{multi_text}");
    assert!(
        !multi_text.contains("pass kind on convey call"),
        "call keeps pass kind, got {multi_text}"
    );
    assert!(!ambiguous_state.join("conveyor-mesh.json").exists());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn box_deny_unresolved_capability_is_intention_not_hop_coverage() {
    let root = repo_root().join(format!(
        "target/test-convey-box-unresolved-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let estate = fixture_estate(&root, "box-deny.yaml", |estate| {
        estate
            .placements
            .iter_mut()
            .find(|p| p.id == "cell-one-box")
            .unwrap()
            .agents = vec!["research".into()];
        let research = estate.agents.iter_mut().find(|a| a.id == "research").unwrap();
        research.tools.push(estate_schema::ToolDecl {
            id: "lane-tool".into(),
            description: None,
        });
        research.mcp.push(estate_schema::McpDecl {
            id: "notes-append".into(),
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
    let estate_s = estate.display().to_string();
    let state = root.join("state");
    let state_s = state.display().to_string();

    let undeclared = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "cell-one-box",
            "--capability",
            "not-a-tool",
            "--agent",
            "research",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
        ],
    );
    let undeclared_text = text(&undeclared);
    assert!(!undeclared.status.success(), "{undeclared_text}");
    assert!(
        undeclared_text.contains("refuse:intention")
            && undeclared_text.contains("undeclared for agent 'research' (deny-default)"),
        "{undeclared_text}"
    );
    assert!(
        !undeclared_text.contains("refuse:hop-coverage"),
        "{undeclared_text}"
    );
    assert!(!state.join("conveyor-mesh.json").exists());

    let multi = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "cell-one-box",
            "--capability",
            "notes-append",
            "--agent",
            "research",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
        ],
    );
    let multi_text = text(&multi);
    assert!(!multi.status.success(), "{multi_text}");
    assert!(
        multi_text.contains("refuse:intention")
            && multi_text.contains("ambiguous capability; pass --intention-kind")
            && multi_text.contains("(deny-default)"),
        "{multi_text}"
    );
    assert!(!multi_text.contains("refuse:hop-coverage"), "{multi_text}");
    assert!(!state.join("conveyor-mesh.json").exists());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn hop_intention_kind_resolves_ambiguous_capability() {
    let root = repo_root().join(format!(
        "target/test-convey-intention-kind-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let estate = fixture_estate(&root, "ambiguous.yaml", |estate| {
        estate
            .placements
            .iter_mut()
            .find(|p| p.id == "cell-one-box")
            .unwrap()
            .agents = vec!["research".into()];
        let research = estate.agents.iter_mut().find(|a| a.id == "research").unwrap();
        research.tools.push(estate_schema::ToolDecl {
            id: "lane-tool".into(),
            description: None,
        });
        research.mcp.push(estate_schema::McpDecl {
            id: "notes-append".into(),
            description: None,
        });
        estate.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "lane-tool".into(),
            kind: estate_schema::IntentionKind::Tool,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
        estate.intentions.push(estate_schema::Intention {
            subject_agent: "research".into(),
            object: "notes-append".into(),
            kind: estate_schema::IntentionKind::Tool,
            effect: estate_schema::Effect::Allow,
            note: None,
        });
    });
    let estate_s = estate.display().to_string();
    let state = root.join("state");
    let state_s = state.display().to_string();

    let ambiguous = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "cell-one-box",
            "--kind",
            "box",
            "--capability",
            "notes-append",
            "--agent",
            "research",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
        ],
    );
    let ambiguous_text = text(&ambiguous);
    assert!(!ambiguous.status.success(), "{ambiguous_text}");
    assert!(
        ambiguous_text.contains("refuse:intention")
            && ambiguous_text.contains("ambiguous capability; pass --intention-kind"),
        "{ambiguous_text}"
    );
    assert!(
        !ambiguous_text.contains("pass kind on convey call"),
        "{ambiguous_text}"
    );
    assert!(!state.join("conveyor-mesh.json").exists());

    let unbound = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "cell-one-box",
            "--capability",
            "notes-append",
            "--intention-kind",
            "tool",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
        ],
    );
    let unbound_text = text(&unbound);
    assert!(!unbound.status.success(), "{unbound_text}");
    assert!(
        unbound_text.contains("refuse:agent-unbound: --intention-kind requires --agent"),
        "{unbound_text}"
    );
    assert!(!state.join("conveyor-mesh.json").exists());

    let allowed = convey(
        &root,
        &[
            "convey",
            "hop",
            "--id",
            "cell-one-box",
            "--kind",
            "box",
            "--capability",
            "notes-append",
            "--agent",
            "research",
            "--intention-kind",
            "tool",
            "--estate",
            &estate_s,
            "--state-dir",
            &state_s,
        ],
    );
    let allowed_text = text(&allowed);
    assert!(allowed.status.success(), "{allowed_text}");
    assert!(allowed_text.contains("\"kind\": \"box\""), "{allowed_text}");
    assert!(state.join("conveyor-mesh.json").exists());

    let _ = std::fs::remove_dir_all(&root);
}
