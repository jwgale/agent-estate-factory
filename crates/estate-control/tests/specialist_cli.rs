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
fn estate_specialist_frontier_refuses_without_key() {
    let out = bin()
        .args([
            "specialist",
            "--driver",
            "frontier",
            "--prompt",
            "Reply with the single word pong.",
        ])
        .env_remove("XAI_API_KEY")
        .env("CELL_LOCAL_ENDPOINT", "http://127.0.0.1:11434")
        .env("CELL_FRONTIER_ENDPOINT", "http://127.0.0.1:9")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("XAI_API_KEY"), "{err}");
    assert!(err.contains("grok-4.7"), "{err}");
    assert!(!err.contains("xai-"), "{err}");
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
        .env_remove("CELL_FRONTIER_MODEL")
        .env_remove("XAI_MODEL")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env("XAI_API_KEY", "test-not-a-secret")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"job\": \"complete\""), "{stdout}");
    assert!(stdout.contains("\"reason\": \"frontier completion\""), "{stdout}");
    assert!(stdout.contains("\"completion\": \"ok\""), "{stdout}");
    assert!(!stdout.contains("test-not-a-secret"), "{stdout}");
    let (path, body) = srv.last_post().expect("frontier openai chat");
    assert_eq!(path, "/v1/chat/completions");
    assert!(body.contains("Reply with the single word pong."), "{body}");
    assert!(body.contains("grok-4.7"), "{body}");
}

#[test]
fn estate_specialist_frontier_sacred_does_not_post() {
    let srv = model_estate::CompatServer::spawn(model_estate::CompatScript::OpenAi {
        models: vec!["grok-4.7".into()],
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
            "please mention cyera",
        ])
        .env("XAI_API_KEY", "test-not-a-secret")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let mix = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(mix.contains("sacred") || mix.contains("denied"), "{mix}");
    assert!(srv.last_post().is_none(), "sacred must not POST");
}

#[test]
fn estate_specialist_frontier_sku_prompt_refuses_before_post() {
    let srv = model_estate::CompatServer::spawn(model_estate::CompatScript::OpenAi {
        models: vec!["grok-4.7".into()],
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
            "use the rtx-5090 weights",
        ])
        .env("XAI_API_KEY", "test-not-a-secret")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("SKU") || err.contains("sku"), "{err}");
    assert!(srv.last_post().is_none(), "SKU prompt must not POST");
}

#[test]
fn estate_specialist_frontier_sku_model_refuses_before_post() {
    let srv = model_estate::CompatServer::spawn(model_estate::CompatScript::OpenAi {
        models: vec!["grok-4.7".into()],
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
        .env("XAI_API_KEY", "test-not-a-secret")
        .env("CELL_FRONTIER_MODEL", "rtx-5090-chat")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("SKU") || err.contains("sku"), "{err}");
    assert!(srv.last_post().is_none(), "SKU model must not POST");
}

#[test]
fn estate_specialist_local_down_does_not_call_frontier() {
    let frontier = model_estate::CompatServer::spawn(model_estate::CompatScript::OpenAi {
        models: vec!["grok-4.7".into()],
    })
    .unwrap();
    let local = model_estate::CompatServer::spawn(
        model_estate::CompatScript::OpenAiEmptyAndOllamaEmpty {
            models: vec!["llama3".into()],
        },
    )
    .unwrap();
    let out = bin()
        .args([
            "specialist",
            "--driver",
            "ollama",
            "--endpoint",
            &local.endpoint(),
            "--prompt",
            "Reply with the single word pong.",
        ])
        .env("XAI_API_KEY", "test-not-a-secret")
        .env("CELL_FRONTIER_ENDPOINT", &frontier.endpoint())
        .env("CELL_FRONTIER_MODEL", "grok-4.7")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let mix = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!mix.contains("frontier completion"), "{mix}");
    assert!(!mix.contains("test-not-a-secret"), "{mix}");
    assert!(local.last_post().is_some(), "local chat must be attempted");
    assert!(
        frontier.last_post().is_none(),
        "local down must not POST frontier"
    );
}

#[test]
fn estate_specialist_missing_local_does_not_call_frontier() {
    let frontier = model_estate::CompatServer::spawn(model_estate::CompatScript::OpenAi {
        models: vec!["grok-4.7".into()],
    })
    .unwrap();
    let out = bin()
        .args([
            "specialist",
            "--driver",
            "http-remote",
            "--prompt",
            "Reply with the single word pong.",
        ])
        .env("XAI_API_KEY", "test-not-a-secret")
        .env("CELL_FRONTIER_ENDPOINT", &frontier.endpoint())
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
    assert!(!err.contains("test-not-a-secret"), "{err}");
    assert!(
        frontier.last_post().is_none(),
        "missing local endpoint must not POST frontier"
    );
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
