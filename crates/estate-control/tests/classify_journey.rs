//! tev1 classify journey. Fake tools and an in-process chat server. No GPU and no network.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::thread;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/fixtures/tev1-decisions.jsonl")
}

fn write_exe(dir: &std::path::Path, name: &str, body: &str) {
    let path = dir.join(name);
    let mut file = fs::File::create(&path).unwrap();
    write!(file, "{body}").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
    }
}

fn fake_tools(dir: &std::path::Path) {
    write_exe(
        dir,
        "nvidia-smi",
        "#!/bin/sh\nexit 0\n",
    );
    write_exe(
        dir,
        "llamafactory-cli",
        r#"#!/bin/sh
set -e
mode=$1
yaml=$2
if [ "$mode" = "train" ]; then
  out=$(awk -F': ' '/^output_dir:/ {print $2; exit}' "$yaml" | tr -d '"')
  mkdir -p "$out"
  printf '%s\n' '{}' > "$out/adapter_config.json"
  exit 0
fi
if [ "$mode" = "export" ]; then
  out=$(awk -F': ' '/^export_dir:/ {print $2; exit}' "$yaml" | tr -d '"')
  mkdir -p "$out"
  printf '%s\n' '{}' > "$out/config.json"
  exit 0
fi
echo "unexpected $mode" >&2
exit 1
"#,
    );
    write_exe(
        dir,
        "convert_hf_to_gguf.py",
        r#"#!/bin/sh
set -e
prev=
outfile=
for arg in "$@"; do
  if [ "$prev" = "--outfile" ]; then
    outfile=$arg
  fi
  prev=$arg
done
mkdir -p "$(dirname "$outfile")"
printf 'gguf\n' > "$outfile"
"#,
    );
    write_exe(
        dir,
        "ollama",
        r#"#!/bin/sh
stamp=${OLLAMA_STAMP:?}
if [ "$1" = "show" ]; then
  grep -qx "$2" "$stamp" && exit 0
  exit 1
fi
if [ "$1" = "create" ]; then
  printf '%s\n' "$2" >> "$stamp"
  exit 0
fi
exit 1
"#,
    );
}

fn tiny_jsonl() -> String {
    let mut lines = String::new();
    for (i, question) in [
        "Which window is still open?",
        "Which ticket is still valid?",
        "Which note is still current?",
        "Which parcel is still eligible?",
    ]
    .iter()
    .enumerate()
    {
        lines.push_str(&format!(
            "{{\"state\":\"Original note {i} for a local split.\",\"question\":\"{question}\",\"options\":[{{\"label\":\"A\",\"key\":\"no\",\"description\":\"No.\"}},{{\"label\":\"B\",\"key\":\"yes\",\"description\":\"Yes.\"}}],\"answer\":\"B\",\"answer_key\":\"yes\"}}\n"
        ));
    }
    lines
}

#[test]
fn journey_print_lists_steps_without_tools() {
    let dir = std::env::temp_dir().join(format!("journey-print-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let out = bin()
        .args([
            "classify",
            "journey",
            "--input",
            fixture().to_str().unwrap(),
            "--out",
            dir.to_str().unwrap(),
            "--print",
        ])
        .env("PATH", "/nonexistent-journey-path")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stdout}\n{stderr}");
    for name in [
        "prepare",
        "recipe",
        "train",
        "merge-export",
        "gguf-convert",
        "ollama-create",
        "eval-base",
        "eval-specialist",
        "compare",
    ] {
        assert!(stdout.contains(name), "{stdout}");
    }
    assert!(stdout.contains("qwen3_5"), "{stdout}");
    assert!(stdout.contains("convert_hf_to_gguf.py"), "{stdout}");
    assert!(stdout.contains("ollama create"), "{stdout}");
    assert!(!dir.exists(), "print must not write {}", dir.display());
}

#[test]
fn journey_run_refuses_missing_tools() {
    let dir = std::env::temp_dir().join(format!("journey-miss-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let out = bin()
        .args([
            "classify",
            "journey",
            "--input",
            fixture().to_str().unwrap(),
            "--out",
            dir.to_str().unwrap(),
            "--run",
        ])
        .env("PATH", "/nonexistent-journey-path")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("refuse:classify-journey"), "{err}");
    assert!(err.contains("llamafactory-cli is not on PATH"), "{err}");
    assert!(err.contains("convert_hf_to_gguf.py is not on PATH"), "{err}");
    assert!(err.contains("ollama is not on PATH"), "{err}");
    assert!(err.contains("no GPU"), "{err}");
}

#[test]
fn journey_run_with_fake_tools_and_mock_endpoint() {
    let root = std::env::temp_dir().join(format!("journey-e2e-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let tools = root.join("bin");
    fs::create_dir_all(&tools).unwrap();
    fake_tools(&tools);
    let stamp = root.join("ollama-models");
    fs::write(&stamp, "qwen3.5:4b\n").unwrap();
    let input = root.join("rows.jsonl");
    fs::write(&input, tiny_jsonl()).unwrap();
    let work = root.join("work");

    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = match server.server_addr() {
        tiny_http::ListenAddr::IP(addr) => addr.port(),
        other => panic!("expected ip listen addr, got {other:?}"),
    };
    thread::spawn(move || {
        for mut req in server.incoming_requests() {
            let mut body = String::new();
            let _ = std::io::Read::read_to_string(req.as_reader(), &mut body);
            let content = if body.contains("tev1-specialist") {
                "B"
            } else {
                "nope"
            };
            let payload = serde_json::json!({
                "choices": [{"message": {"content": content}}]
            });
            let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
            let resp = tiny_http::Response::from_string(payload.to_string()).with_header(header);
            let _ = req.respond(resp);
        }
    });

    let endpoint = format!("http://127.0.0.1:{port}");
    let first = bin()
        .args([
            "classify",
            "journey",
            "--input",
            input.to_str().unwrap(),
            "--out",
            work.to_str().unwrap(),
            "--run",
            "--max-steps",
            "1",
            "--endpoint",
            &endpoint,
            "--min-delta",
            "0.5",
            "--min-accuracy",
            "0.9",
            "--timeout-secs",
            "5",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("OLLAMA_STAMP", stamp.to_str().unwrap())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&first.stdout);
    let stderr = String::from_utf8_lossy(&first.stderr);
    assert!(first.status.success(), "stdout {stdout}\nstderr {stderr}");
    let recipe = fs::read_to_string(work.join("recipe.yaml")).unwrap();
    let info: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(work.join("dataset_info.json")).unwrap()).unwrap();
    assert!(info.get("tev1_decisions").is_some(), "{info}");
    assert!(recipe.contains("dataset: tev1_decisions"), "{recipe}");
    assert!(recipe.contains("dataset_dir:"), "{recipe}");
    assert!(recipe.contains("template: qwen3_5"), "{recipe}");
    assert!(work.join("outputs/adapter_config.json").is_file());
    assert!(work.join("export/config.json").is_file());
    assert!(work.join("export.gguf").is_file());
    assert!(work.join("Modelfile").is_file());
    let comparison: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(work.join("comparison.json")).unwrap()).unwrap();
    assert_eq!(comparison["live_pass_recorded"], false);
    assert!((comparison["specialist_accuracy"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    assert!((comparison["base_accuracy"].as_f64().unwrap() - 0.0).abs() < 1e-9);
    assert!((comparison["delta"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    assert_eq!(comparison["threshold"], "met");
    assert!(stdout.contains("threshold met"), "{stdout}");

    let second = bin()
        .args([
            "classify",
            "journey",
            "--input",
            input.to_str().unwrap(),
            "--out",
            work.to_str().unwrap(),
            "--run",
            "--endpoint",
            &endpoint,
            "--min-accuracy",
            "2",
            "--timeout-secs",
            "5",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("OLLAMA_STAMP", stamp.to_str().unwrap())
        .output()
        .unwrap();
    let again = format!(
        "{}{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(!second.status.success(), "{again}");
    assert!(again.contains("skip train"), "{again}");
    assert!(again.contains("skip merge-export"), "{again}");
    assert!(again.contains("skip gguf-convert"), "{again}");
    assert!(again.contains("skip ollama-create"), "{again}");
    assert!(again.contains("threshold missed"), "{again}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn prepare_refuses_when_out_is_a_file() {
    let dir = std::env::temp_dir().join(format!("classify-file-out-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("not-a-dir");
    fs::write(&file, "already").unwrap();
    let refused = bin()
        .args([
            "classify",
            "prepare",
            "--input",
            fixture().to_str().unwrap(),
            "--out",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    let err = String::from_utf8_lossy(&refused.stderr);
    assert!(err.contains("is a file"), "{err}");
    assert!(err.contains("--force"), "{err}");
    assert_eq!(fs::read_to_string(&file).unwrap(), "already");
    let forced = bin()
        .args([
            "classify",
            "prepare",
            "--input",
            fixture().to_str().unwrap(),
            "--out",
            file.to_str().unwrap(),
            "--force",
        ])
        .output()
        .unwrap();
    assert!(
        forced.status.success(),
        "{}",
        String::from_utf8_lossy(&forced.stderr)
    );
    assert!(file.join("dataset.jsonl").is_file());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn eval_records_non_json_and_missing_content() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = match server.server_addr() {
        tiny_http::ListenAddr::IP(addr) => addr.port(),
        other => panic!("expected ip listen addr, got {other:?}"),
    };
    thread::spawn(move || {
        for (n, mut req) in server.incoming_requests().enumerate() {
            let mut body = String::new();
            let _ = std::io::Read::read_to_string(req.as_reader(), &mut body);
            let payload = match n {
                0 => "not-json".to_string(),
                _ => serde_json::json!({"choices": []}).to_string(),
            };
            let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/plain"[..]).unwrap();
            let resp = tiny_http::Response::from_string(payload)
                .with_status_code(200)
                .with_header(header);
            let _ = req.respond(resp);
        }
    });
    let dir = std::env::temp_dir().join(format!("classify-bad-body-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let records = dir.join("heldout.jsonl");
    fs::write(
        &records,
        concat!(
            "{\"state\":\"Note one for a bad body.\",\"question\":\"Which letter is first here?\",\"options\":[{\"label\":\"A\",\"key\":\"a\",\"description\":\"A.\"},{\"label\":\"B\",\"key\":\"b\",\"description\":\"B.\"}],\"answer\":\"A\",\"answer_key\":\"a\"}\n",
            "{\"state\":\"Note two for a bad body.\",\"question\":\"Which letter is second here?\",\"options\":[{\"label\":\"A\",\"key\":\"a\",\"description\":\"A.\"},{\"label\":\"B\",\"key\":\"b\",\"description\":\"B.\"}],\"answer\":\"B\",\"answer_key\":\"b\"}\n",
        ),
    )
    .unwrap();
    let report_path = dir.join("report.json");
    let out = bin()
        .args([
            "classify",
            "eval",
            "--records",
            records.to_str().unwrap(),
            "--endpoint",
            &format!("http://127.0.0.1:{port}"),
            "--model",
            "fixture-model",
            "--report",
            report_path.to_str().unwrap(),
            "--timeout-secs",
            "5",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report_path).unwrap()).unwrap();
    let errors = report["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 2, "{report}");
    assert_eq!(errors[0]["status"], 200);
    assert!(errors[0]["body"].as_str().unwrap().contains("not-json"));
    assert_eq!(errors[1]["status"], 200);
    assert_eq!(report["http_errors"], 2);
    assert_eq!(report["invalid"], 2);
    let _ = fs::remove_dir_all(&dir);
}
