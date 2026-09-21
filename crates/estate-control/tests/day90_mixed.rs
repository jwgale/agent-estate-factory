//! Mixed estate factory path: apply --dry-run writes nothing; local
//! binding is HttpLocal against in-process mock. No live box.

use estate_schema::{load_estate, ModelClass};
use floor_supervisor::apply_dry_run;
use model_estate::{
    bind_local, LocalRuntime, MockLocalServer, SpecialistJob, SpecialistRequest,
};
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn mixed_estate() -> estate_schema::Estate {
    load_estate(&repo_root().join("examples/fixtures/mixed-frontier-local.yaml")).unwrap()
}

fn spawn_compat(script: model_estate::CompatScript) -> model_estate::CompatServer {
    for _ in 0..40 {
        let srv = model_estate::CompatServer::spawn(script.clone()).unwrap();
        if !estate_schema::contains_sku(&srv.endpoint()) {
            return srv;
        }
    }
    panic!("ephemeral port kept encoding a hardware SKU");
}

fn spawn_mock_local() -> MockLocalServer {
    for _ in 0..40 {
        let srv = MockLocalServer::spawn().unwrap();
        if !estate_schema::contains_sku(&srv.endpoint()) {
            return srv;
        }
    }
    panic!("ephemeral port kept encoding a hardware SKU");
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-mixed-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn mixed_estate_dry_run_http_local_against_mock() {
    let estate = mixed_estate();
    assert_eq!(estate.name, "cell-one-mixed-proof");
    assert!(estate
        .model_bindings
        .iter()
        .any(|b| b.class == ModelClass::Frontier && b.driver == "http-remote"));
    assert!(estate
        .model_bindings
        .iter()
        .any(|b| b.id == "local_slm" && b.driver == "ollama"));

    let state = tmp("dry");
    let report = apply_dry_run(&estate, &state).unwrap();
    assert!(!report.writes, "dry-run writes flag must stay false");
    assert!(
        !state.join("placement-actual.json").exists(),
        "mixed dry-run must not write placement-actual"
    );
    assert!(
        !state.join("catalog.json").exists(),
        "mixed dry-run must not write catalog.json"
    );
    assert!(
        report.notes.iter().any(|n| n.contains("no leases")),
        "{:?}",
        report.notes
    );

    let srv = MockLocalServer::spawn().unwrap();
    let local = estate
        .model_bindings
        .iter()
        .find(|b| b.id == "local_slm")
        .cloned()
        .unwrap();
    let mut params = local.params.clone();
    params["endpoint_env"] = serde_json::json!("CELL_LOCAL_ENDPOINT_ABSENT_FOR_MIXED_TEST");
    params["endpoint"] = serde_json::json!(srv.endpoint());
    let binding = estate_schema::ModelBinding { params, ..local };

    let driver = bind_local(&binding).unwrap();
    assert_eq!(driver.runtime(), LocalRuntime::Ollama);
    let allow = driver
        .specialist(&SpecialistRequest {
            job: SpecialistJob::PolicyPrecheck,
            agent_id: "research".into(),
            kind: "tool".into(),
            text: "hello from mixed".into(),
        })
        .unwrap();
    assert!(allow.allow, "{}", allow.reason);
    let (path, body) = srv.last_post().expect("factory /v0/specialist");
    assert_eq!(path, "/v0/specialist");
    assert!(body.contains("hello from mixed"), "{body}");

    let deny = driver
        .specialist(&SpecialistRequest {
            job: SpecialistJob::PolicyPrecheck,
            agent_id: "research".into(),
            kind: "tool".into(),
            text: "please mention cyera".into(),
        })
        .unwrap();
    assert!(!deny.allow, "{}", deny.reason);
    let (_, last) = srv.last_post().expect("sacred must not replace last post");
    assert!(
        !last.contains("please mention cyera"),
        "sacred text must not POST: {last}"
    );

    assert!(
        !state.join("placement-actual.json").exists(),
        "specialist must not write apply actual"
    );
    let _ = std::fs::remove_dir_all(&state);
}

#[test]
fn frontier_http_host_fixture_names_grok_without_touching_locked_estate() {
    let path = repo_root().join("examples/hosts/frontier-http.yaml");
    let yaml = std::fs::read_to_string(&path).unwrap();
    assert!(
        yaml.contains("model: grok-4.7"),
        "host fixture must name the frontier model"
    );
    assert!(yaml.contains("driver: http-remote"), "{yaml}");
    let estate = load_estate(&path).unwrap();
    assert_eq!(estate.name, "cell-one-frontier-http");
    let frontier = estate
        .model_bindings
        .iter()
        .find(|b| b.id == "frontier_http")
        .expect("frontier_http");
    assert_eq!(frontier.class, ModelClass::Frontier);
    assert_eq!(frontier.driver, "http-remote");
    assert_eq!(frontier.params["model"].as_str(), Some("grok-4.7"));
    assert!(estate
        .model_bindings
        .iter()
        .any(|b| b.id == "local_slm" && b.driver == "ollama"));
    let locked = std::fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
    assert!(
        locked.contains("sha256:dcd7164f04c83f514185e77d2d4f6c23cae6dbb27a9b5da96a28ba1f3c724930"),
        "hash lock comment must stay"
    );
    assert!(
        !locked.contains("model: grok-4.7"),
        "hash-locked estate.yaml must not gain a binding model"
    );
    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args(["validate", "--estate", &path.display().to_string()])
        .env_remove("XAI_API_KEY")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let models = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args(["models", "--estate", &path.display().to_string()])
        .env_remove("XAI_API_KEY")
        .output()
        .unwrap();
    let listed = format!(
        "{}{}",
        String::from_utf8_lossy(&models.stdout),
        String::from_utf8_lossy(&models.stderr)
    );
    assert!(models.status.success(), "{listed}");
    assert!(
        listed.contains("driver=http-remote") && listed.contains("model=grok-4.7"),
        "models must print the binding model: {listed}"
    );
    let default_estate = repo_root().join("examples/estate.yaml");
    let bare = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args(["models", "--estate", &default_estate.display().to_string()])
        .env_remove("XAI_API_KEY")
        .output()
        .unwrap();
    let bare_text = String::from_utf8_lossy(&bare.stdout);
    assert!(bare.status.success(), "{}", String::from_utf8_lossy(&bare.stderr));
    assert!(
        bare_text.contains("model=-"),
        "a binding with no model param stays model=-: {bare_text}"
    );
    assert!(
        !bare_text.contains("model=grok-4.7"),
        "default estate must not invent a binding model: {bare_text}"
    );
}

#[test]
fn mixed_grok_4_7_validate_and_dry_run_does_not_post() {
    let path = repo_root().join("examples/fixtures/mixed-frontier-local.yaml");
    let yaml = std::fs::read_to_string(&path).unwrap();
    assert!(yaml.contains("grok-4.7"), "fixture must name the frontier model");
    let estate = mixed_estate();
    let frontier = estate
        .model_bindings
        .iter()
        .find(|b| b.id == "frontier_http")
        .unwrap();
    assert_eq!(frontier.class, ModelClass::Frontier);
    assert_eq!(frontier.driver, "http-remote");
    assert_eq!(frontier.params["model"].as_str(), Some("grok-4.7"));
    assert!(estate
        .model_bindings
        .iter()
        .any(|b| b.id == "local_slm" && b.driver == "ollama"));

    let srv = model_estate::CompatServer::spawn(model_estate::CompatScript::OpenAi {
        models: vec!["grok-4.7".into()],
    })
    .unwrap();
    let state = tmp("cli-dry");
    let estate_path = path.display().to_string();
    let validate = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args(["validate", "--estate", &estate_path])
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_RENTED_ENDPOINT")
        .env("CELL_FRONTIER_ENDPOINT", &srv.endpoint())
        .output()
        .unwrap();
    assert!(
        validate.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&validate.stderr)
    );

    let dry = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args([
            "apply",
            "--dry-run",
            "--estate",
            &estate_path,
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &state.display().to_string(),
            "--plans-dir",
            &state.join("plans").display().to_string(),
        ])
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env("CELL_FRONTIER_ENDPOINT", &srv.endpoint())
        .env("CELL_FRONTIER_MODEL", "grok-4.7")
        .output()
        .unwrap();
    let mix = format!(
        "{}{}",
        String::from_utf8_lossy(&dry.stdout),
        String::from_utf8_lossy(&dry.stderr)
    );
    assert!(dry.status.success(), "{mix}");
    assert!(!state.join("placement-actual.json").exists(), "{mix}");
    assert!(!state.join("catalog.json").exists(), "{mix}");
    assert!(
        srv.last_post().is_none(),
        "dry-run must not POST to frontier"
    );
    let _ = std::fs::remove_dir_all(&state);
}

#[test]
fn mixed_plan_apply_then_local_specialist_skips_frontier() {
    let frontier = spawn_compat(model_estate::CompatScript::OpenAi {
        models: vec!["grok-4.7".into()],
    });
    let local = spawn_mock_local();
    let root = repo_root().join(format!(
        "target/test-mixed-operator-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let plans = root.join("plans");
    let estate = repo_root()
        .join("examples/fixtures/mixed-frontier-local.yaml")
        .display()
        .to_string();
    let sacred = repo_root()
        .join("policy/sacred.yaml")
        .display()
        .to_string();
    let state_s = state.display().to_string();
    let plans_s = plans.display().to_string();
    let roots_s = root.display().to_string();

    let plan = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args([
            "plan",
            "--estate",
            &estate,
            "--sacred",
            &sacred,
            "--plans-dir",
            &plans_s,
            "--state-dir",
            &state_s,
        ])
        .env_remove("XAI_API_KEY")
        .env("CELL_FRONTIER_ENDPOINT", &frontier.endpoint())
        .env_remove("CELL_LOCAL_ENDPOINT")
        .output()
        .unwrap();
    assert!(
        plan.status.success(),
        "{}",
        String::from_utf8_lossy(&plan.stderr)
    );
    assert!(
        std::fs::read_dir(&plans)
            .unwrap()
            .any(|e| e.unwrap().path().extension().and_then(|s| s.to_str()) == Some("json")),
        "plan must write json"
    );

    let apply = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args([
            "apply",
            "--require-plan",
            "--estate",
            &estate,
            "--sacred",
            &sacred,
            "--state-dir",
            &state_s,
            "--roots-base",
            &roots_s,
            "--plans-dir",
            &plans_s,
        ])
        .env_remove("XAI_API_KEY")
        .env("CELL_FRONTIER_ENDPOINT", &frontier.endpoint())
        .env_remove("CELL_LOCAL_ENDPOINT")
        .output()
        .unwrap();
    let apply_text = format!(
        "{}{}",
        String::from_utf8_lossy(&apply.stdout),
        String::from_utf8_lossy(&apply.stderr)
    );
    assert!(apply.status.success(), "{apply_text}");
    assert!(!apply_text.contains("test-not-a-secret"));

    let actual = std::fs::read_to_string(state.join("model-actual.json")).unwrap();
    assert!(actual.contains("frontier_http"), "{actual}");
    assert!(actual.contains("\"driver\": \"http-remote\""), "{actual}");
    assert!(actual.contains("local_slm"), "{actual}");
    assert!(actual.contains("\"driver\": \"ollama\""), "{actual}");
    let catalog = std::fs::read_to_string(state.join("catalog.json")).unwrap();
    assert!(catalog.contains("\"model\": \"grok-4.7\""), "{catalog}");
    assert!(catalog.contains("completion_tokens"), "{catalog}");
    assert!(state.join("placement-actual.json").is_file());
    assert!(
        frontier.last_post().is_none(),
        "plan/apply must not POST frontier"
    );

    let spec = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args([
            "specialist",
            "--driver",
            "ollama",
            "--endpoint",
            &local.endpoint(),
            "--prompt",
            "hello from mixed apply",
        ])
        .env_remove("XAI_API_KEY")
        .env("CELL_FRONTIER_ENDPOINT", &frontier.endpoint())
        .env_remove("CELL_LOCAL_ENDPOINT")
        .output()
        .unwrap();
    assert!(
        spec.status.success(),
        "{}",
        String::from_utf8_lossy(&spec.stderr)
    );
    let stdout = String::from_utf8_lossy(&spec.stdout);
    assert!(
        stdout.contains("\"completion\": \"mock:hello from mixed apply\""),
        "{stdout}"
    );
    assert!(local.last_post().is_some());
    assert!(
        frontier.last_post().is_none(),
        "local specialist after apply must not POST frontier"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn day90_mixed_walks_plan_apply_and_names_grok_4_7() {
    let root = repo_root();
    let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
    assert!(makefile.contains("day90-mixed:"), "Makefile missing day90-mixed");
    assert!(makefile.contains("scripts/day90-mixed.sh"));
    let script = std::fs::read_to_string(root.join("scripts/day90-mixed.sh")).unwrap();
    assert!(script.contains("--require-plan"), "{script}");
    assert!(script.contains("unset XAI_API_KEY"), "{script}");
    assert!(
        script.contains("examples/hosts/frontier-http.yaml"),
        "day90-mixed must validate the frontier-http host fixture"
    );
    assert!(
        script.contains("examples/estate.yaml changed"),
        "day90-mixed must refuse a rewrite of the hash-locked estate"
    );
    let fixtures = std::fs::read_to_string(root.join("scripts/fixtures-check.sh")).unwrap();
    assert!(
        !fixtures.contains("frontier-http.yaml"),
        "fixtures-check is inside smoke; do not add the host fixture there"
    );
    assert!(
        script.contains("Do not add to make smoke or GitHub Actions"),
        "day90-mixed must stay off smoke / Actions"
    );
    let smoke = std::fs::read_to_string(root.join("scripts/smoke.sh")).unwrap();
    let gate = std::fs::read_to_string(root.join("scripts/day90-gate.sh")).unwrap();
    let ci = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    for (name, text) in [("smoke", &smoke), ("gate-90", &gate), ("ci.yml", &ci)] {
        assert!(
            !text.contains("day90-mixed"),
            "{name} must not invoke day90-mixed"
        );
    }

    let state = root.join(format!(
        "target/test-day90-mixed-walk-{}",
        std::process::id()
    ));
    let plans = root.join(format!(
        "target/test-day90-mixed-plans-{}",
        std::process::id()
    ));
    let out = Command::new("bash")
        .arg(root.join("scripts/day90-mixed.sh"))
        .current_dir(&root)
        .env("ESTATE_BIN", env!("CARGO_BIN_EXE_estate"))
        .env("STATE_DIR", &state)
        .env("PLANS_DIR", &plans)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "{text}");
    assert!(text.contains("DAY90-MIXED GREEN"), "{text}");
    assert!(text.contains("frontier: frontier_http model=grok-4.7"), "{text}");
    assert!(text.contains("estate: cell-one-frontier-http"), "{text}");
    assert!(text.contains("catalog frontier: cell model=grok-4.7"), "{text}");
    let _ = std::fs::remove_dir_all(&state);
    let _ = std::fs::remove_dir_all(&plans);
    let _ = std::fs::remove_dir_all(root.join(format!(
        "target/test-day90-mixed-walk-{}-frontier-http",
        std::process::id()
    )));
    let _ = std::fs::remove_dir_all(root.join(format!(
        "target/test-day90-mixed-plans-{}-frontier-http",
        std::process::id()
    )));
}

#[test]
fn status_does_not_invent_frontier_model_on_the_default_estate() {
    let root = repo_root();
    let state = root.join(format!(
        "target/test-status-default-frontier-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&state);
    let out = Command::new(env!("CARGO_BIN_EXE_estate"))
        .args([
            "status",
            "--estate",
            &root.join("examples/estate.yaml").display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
            "--plans-dir",
            &state.join("plans").display().to_string(),
            "--policy",
            &root.join("policy/cell-one.policy.v0.yaml").display().to_string(),
            "--root",
            &root.display().to_string(),
        ])
        .env_remove("XAI_API_KEY")
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "{text}");
    assert!(
        text.contains("catalog frontier: schema model=grok-4.7"),
        "{text}"
    );
    assert!(
        !text.contains("frontier: xai_grok model="),
        "default estate binding has no model param: {text}"
    );
    assert!(!text.contains("catalog frontier: cell"), "{text}");
    let _ = std::fs::remove_dir_all(&state);
}

#[test]
fn live_specialist_helper_requires_endpoint_and_stays_off_smoke() {
    let root = repo_root();
    let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
    assert!(
        makefile.contains("live-specialist:"),
        "Makefile missing live-specialist"
    );
    assert!(
        makefile.contains("Do not add to smoke or GitHub Actions"),
        "live-specialist must stay off smoke / Actions"
    );

    let smoke = std::fs::read_to_string(root.join("scripts/smoke.sh")).unwrap();
    let gate = std::fs::read_to_string(root.join("scripts/day90-gate.sh")).unwrap();
    let ci = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    for (name, text) in [("smoke", &smoke), ("gate-90", &gate), ("ci.yml", &ci)] {
        assert!(
            !text.contains("live-specialist"),
            "{name} must not invoke live-specialist"
        );
    }

    let script = root.join("scripts/live-specialist.sh");
    let out = std::process::Command::new("bash")
        .arg(&script)
        .env_remove("CELL_LOCAL_ENDPOINT")
        .output()
        .unwrap();
    assert!(!out.status.success(), "unset endpoint must refuse");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("CELL_LOCAL_ENDPOINT"), "{err}");
    assert!(err.contains("not in smoke"), "{err}");
}
