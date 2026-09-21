//! `model-estate specialist` shares `run_http_specialist` with `estate specialist`.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_model-estate"))
}

#[test]
fn specialist_cli_roundtrip_against_mock_http() {
    let srv = model_estate::MockLocalServer::spawn().unwrap();
    let allow = bin()
        .args([
            "specialist",
            "--endpoint",
            &srv.endpoint(),
            "--text",
            "hello from the factory",
        ])
        .output()
        .unwrap();
    assert!(
        allow.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&allow.stderr)
    );
    let stdout = String::from_utf8_lossy(&allow.stdout);
    assert!(stdout.contains("\"allow\": true"), "{stdout}");
    assert!(stdout.contains("policy-precheck allow"), "{stdout}");

    let deny = bin()
        .args([
            "specialist",
            "--endpoint",
            &srv.endpoint(),
            "--text",
            "please mention cyera",
        ])
        .output()
        .unwrap();
    assert!(!deny.status.success());
    let mix = format!(
        "{}{}",
        String::from_utf8_lossy(&deny.stdout),
        String::from_utf8_lossy(&deny.stderr)
    );
    assert!(mix.contains("sacred") || mix.contains("denied"), "{mix}");
}

#[test]
fn specialist_cli_llama_cpp_openai_path() {
    let srv = model_estate::CompatServer::spawn(model_estate::CompatScript::OpenAi {
        models: vec!["ggml-model".into()],
    })
    .unwrap();
    let out = bin()
        .args([
            "specialist",
            "--runtime",
            "llama.cpp",
            "--endpoint",
            &srv.endpoint(),
            "--text",
            "llamacpp-cli-roundtrip",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"allow\": true"), "{stdout}");
    let (path, body) = srv.last_post().expect("openai chat");
    assert_eq!(path, "/v1/chat/completions");
    assert!(body.contains("llamacpp-cli-roundtrip"), "{body}");
}

#[test]
fn specialist_cli_complete_returns_model_text() {
    let srv = model_estate::CompatServer::spawn(model_estate::CompatScript::Ollama {
        models: vec!["llama3".into()],
    })
    .unwrap();
    let out = bin()
        .args([
            "specialist",
            "--job",
            "complete",
            "--runtime",
            "ollama",
            "--endpoint",
            &srv.endpoint(),
            "--prompt",
            "ping",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"completion\": \"ok\""), "{stdout}");
    assert!(stdout.contains("\"job\": \"complete\""), "{stdout}");
}

#[test]
fn specialist_cli_refuses_missing_endpoint() {
    let out = bin()
        .args(["specialist", "--text", "hello"])
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_RENTED_ENDPOINT")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("CELL_LOCAL_ENDPOINT") || err.contains("--endpoint"),
        "{err}"
    );
}
