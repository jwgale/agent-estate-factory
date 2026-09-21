//! `estate specialist` is a thin HttpLocal delegate. Not a gateway.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

#[test]
fn estate_specialist_complete_against_mock_local() {
    let srv = model_estate::MockLocalServer::spawn().unwrap();
    let out = bin()
        .args([
            "specialist",
            "--endpoint",
            &srv.endpoint(),
            "--prompt",
            "hello from the factory",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"job\": \"complete\""), "{stdout}");
    assert!(
        stdout.contains("\"completion\": \"mock:hello from the factory\""),
        "{stdout}"
    );
}

#[test]
fn estate_specialist_complete_against_compat_http() {
    let srv = model_estate::CompatServer::spawn(model_estate::CompatScript::OpenAi {
        models: vec!["llama3".into()],
    })
    .unwrap();
    let out = bin()
        .args([
            "specialist",
            "--driver",
            "ollama",
            "--endpoint",
            &srv.endpoint(),
            "--prompt",
            "Reply with the single word pong.",
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
    assert!(stdout.contains("\"job\": \"complete\""), "{stdout}");
    assert!(stdout.contains("\"completion\": \"ok\""), "{stdout}");
    assert!(!stdout.contains("rtx-5090"), "{stdout}");
    let (path, body) = srv.last_post().expect("openai chat");
    assert_eq!(path, "/v1/chat/completions");
    assert!(body.contains("Reply with the single word pong."), "{body}");
}

#[test]
fn estate_specialist_llama_cpp_openai_path() {
    let srv = model_estate::CompatServer::spawn(model_estate::CompatScript::OpenAi {
        models: vec!["ggml-model".into()],
    })
    .unwrap();
    let out = bin()
        .args([
            "specialist",
            "--driver",
            "llama.cpp",
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
}

#[test]
fn estate_specialist_refuses_missing_endpoint() {
    let out = bin()
        .args(["specialist", "--prompt", "ping"])
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_RENTED_ENDPOINT")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("CELL_LOCAL_ENDPOINT") || err.contains("endpoint"),
        "{err}"
    );
}

#[test]
fn estate_specialist_refuses_sku_endpoint() {
    let out = bin()
        .args([
            "specialist",
            "--endpoint",
            "http://rtx-5090.example:11434",
            "--prompt",
            "ping",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("SKU") || err.contains("sku"), "{err}");
}

#[test]
fn estate_specialist_frontier_requires_frontier_env_not_xai() {
    let out = bin()
        .args([
            "specialist",
            "--driver",
            "frontier",
            "--prompt",
            "Reply with the single word pong.",
        ])
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env("CELL_LOCAL_ENDPOINT", "http://127.0.0.1:11434")
        .env("XAI_API_KEY", "xai-not-a-real-key-value")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("CELL_FRONTIER_ENDPOINT"), "{err}");
    assert!(err.contains("XAI_API_KEY"), "{err}");
}

#[test]
fn estate_specialist_frontier_complete_against_compat_http() {
    let srv = model_estate::CompatServer::spawn(model_estate::CompatScript::OpenAi {
        models: vec!["gateway-model".into()],
    })
    .unwrap();
    let out = bin()
        .args([
            "specialist",
            "--driver",
            "frontier",
            "--endpoint",
            &srv.endpoint(),
            "--prompt",
            "Reply with the single word pong.",
        ])
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"job\": \"complete\""), "{stdout}");
    assert!(stdout.contains("\"completion\": \"ok\""), "{stdout}");
    let (path, body) = srv.last_post().expect("frontier openai chat");
    assert_eq!(path, "/v1/chat/completions");
    assert!(body.contains("Reply with the single word pong."), "{body}");
}

#[test]
fn estate_specialist_sacred_denies_without_inventing_text() {
    let srv = model_estate::CompatServer::spawn(model_estate::CompatScript::OpenAi {
        models: vec!["llama3".into()],
    })
    .unwrap();
    let out = bin()
        .args([
            "specialist",
            "--endpoint",
            &srv.endpoint(),
            "--prompt",
            "please mention cyera",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let mix = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(mix.contains("sacred") || mix.contains("denied"), "{mix}");
    assert!(!mix.contains("\"completion\": \"ok\""), "{mix}");
    assert!(srv.last_post().is_none());
}
