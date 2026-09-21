//! Mixed estate factory path: apply --dry-run writes nothing; local
//! binding is HttpLocal against in-process mock. No live box.

use estate_schema::{load_estate, ModelClass};
use floor_supervisor::apply_dry_run;
use model_estate::{
    bind_local, LocalRuntime, MockLocalServer, SpecialistJob, SpecialistRequest,
};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn mixed_estate() -> estate_schema::Estate {
    load_estate(&repo_root().join("examples/fixtures/mixed-frontier-local.yaml")).unwrap()
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
