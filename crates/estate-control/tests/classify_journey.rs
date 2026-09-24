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
    write_exe(dir, "nvidia-smi", "#!/bin/sh\nexit 0\n");
    write_exe(
        dir,
        "huggingface-cli",
        r#"#!/bin/sh
set -e
printf '%s\n' "$*" >> "${JOURNEY_TOOL_LOG:?}"
local_dir=
prev=
for arg in "$@"; do
  if [ "$prev" = "--local-dir" ]; then
    local_dir=$arg
  fi
  prev=$arg
done
mkdir -p "$local_dir"
printf '%s\n' '{}' > "$local_dir/config.json"
printf '%s\n' '{}' > "$local_dir/tokenizer.json"
printf 'w\n' > "$local_dir/model.safetensors"
"#,
    );
    write_exe(
        dir,
        "llamafactory-cli",
        r#"#!/bin/sh
set -e
printf '%s\n' "$*" >> "${JOURNEY_TOOL_LOG:?}"
mode=$1
yaml=$2
if [ "$mode" = "train" ]; then
  out=$(awk -F': ' '/^output_dir:/ {print $2; exit}' "$yaml" | tr -d '"')
  mkdir -p "$out"
  printf '%s\n' '{}' > "$out/adapter_config.json"
  printf 'w\n' > "$out/adapter_model.safetensors"
  exit 0
fi
if [ "$mode" = "export" ]; then
  out=$(awk -F': ' '/^export_dir:/ {print $2; exit}' "$yaml" | tr -d '"')
  mkdir -p "$out"
  printf '%s\n' '{}' > "$out/config.json"
  printf '%s\n' '{}' > "$out/tokenizer.json"
  printf 'w\n' > "$out/model.safetensors"
  exit 0
fi
echo "unexpected $mode" >&2
exit 1
"#,
    );
    write_exe(
        dir,
        "ollama",
        r#"#!/bin/sh
stamp=${OLLAMA_STAMP:?}
printf '%s\n' "$*" >> "${JOURNEY_TOOL_LOG:?}"
if [ "$1" = "show" ]; then
  grep -qx "$2" "$stamp" && exit 0
  exit 1
fi
if [ "$1" = "rm" ]; then
  grep -vx "$2" "$stamp" > "$stamp.tmp" || true
  mv "$stamp.tmp" "$stamp"
  exit 0
fi
if [ "$1" = "create" ]; then
  printf '%s\n' "$2" >> "$stamp"
  exit 0
fi
exit 1
"#,
    );
}

fn fake_llama(dir: &std::path::Path) {
    fs::create_dir_all(dir.join("build/bin")).unwrap();
    fs::write(
        dir.join("convert_hf_to_gguf.py"),
        r#"import os, sys
log = os.environ["JOURNEY_TOOL_LOG"]
with open(log, "a", encoding="utf-8") as fh:
    fh.write("python3 " + " ".join(sys.argv) + "\n")
outfile = None
args = sys.argv[1:]
for i, arg in enumerate(args):
    if arg == "--outfile" and i + 1 < len(args):
        outfile = args[i + 1]
if outfile is None:
    sys.stderr.write("missing --outfile\n")
    sys.exit(1)
os.makedirs(os.path.dirname(outfile) or ".", exist_ok=True)
with open(outfile, "wb") as fh:
    fh.write(b"GGUF\n")
"#,
    )
    .unwrap();
    write_exe(
        dir.join("build/bin").as_path(),
        "llama-quantize",
        r#"#!/bin/sh
set -e
printf '%s\n' "$0 $*" >> "${JOURNEY_TOOL_LOG:?}"
mkdir -p "$(dirname "$2")"
printf 'GGUF quant %s\n' "$3" > "$2"
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
        "fetch-base",
        "recipe",
        "train",
        "merge-export",
        "gguf-convert-base",
        "gguf-convert-specialist",
        "quantize-base",
        "quantize-specialist",
        "ollama-create-base",
        "ollama-create-specialist",
        "eval-base",
        "eval-specialist",
        "compare",
    ] {
        assert!(stdout.contains(name), "{stdout}");
    }
    assert!(stdout.contains("qwen3_5"), "{stdout}");
    assert!(
        stdout.contains("python3 $LLAMA_CPP_DIR/convert_hf_to_gguf.py"),
        "{stdout}"
    );
    assert!(stdout.contains("--outtype f16"), "{stdout}");
    assert!(stdout.contains("Q4_K_M"), "{stdout}");
    assert!(stdout.contains("ollama-native"), "{stdout}");
    assert!(
        stdout.contains("Qwen3.5 needs a recent llama.cpp checkout"),
        "{stdout}"
    );
    assert!(!dir.exists(), "print must not write {}", dir.display());
    let warned = bin()
        .args([
            "classify",
            "journey",
            "--input",
            fixture().to_str().unwrap(),
            "--out",
            dir.to_str().unwrap(),
            "--print",
            "--base-tag",
            "qwen3.5:4b",
        ])
        .env("PATH", "/nonexistent-journey-path")
        .output()
        .unwrap();
    let warned_out = String::from_utf8_lossy(&warned.stdout);
    assert!(warned.status.success(), "{warned_out}");
    assert!(warned_out.contains("Precision may differ"), "{warned_out}");
    assert!(
        warned_out.contains("skip gguf-convert-base"),
        "{warned_out}"
    );
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
    assert!(
        err.contains("huggingface-cli and hf are not on PATH"),
        "{err}"
    );
    assert!(err.contains("LLAMA_CPP_DIR is unset"), "{err}");
    assert!(
        err.contains("Qwen3.5 needs a recent llama.cpp checkout"),
        "{err}"
    );
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
    let llama = root.join("llama.cpp");
    fake_llama(&llama);
    let stamp = root.join("ollama-models");
    fs::write(&stamp, "").unwrap();
    let log = root.join("tools.log");
    fs::write(&log, "").unwrap();
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
                "<think>\nhidden\n</think>\nA"
            };
            let payload = serde_json::json!({
                "message": {"role": "assistant", "content": content}
            });
            let header =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap();
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
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
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
        .env("JOURNEY_TOOL_LOG", log.to_str().unwrap())
        .env("HF_TOKEN", "super-secret-hf")
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
    assert!(recipe.contains("enable_thinking: false"), "{recipe}");
    assert!(work.join("outputs/adapter_config.json").is_file());
    assert!(work.join("export/config.json").is_file());
    assert!(work.join("base.f16.gguf").is_file());
    assert!(work.join("specialist.f16.gguf").is_file());
    assert!(work.join("base.Q4_K_M.gguf").is_file());
    assert!(work.join("specialist.Q4_K_M.gguf").is_file());
    let modelfile = fs::read_to_string(work.join("specialist.Modelfile")).unwrap();
    assert!(modelfile.contains("PARAMETER temperature 0"), "{modelfile}");
    assert!(modelfile.contains("PARAMETER num_predict 8"), "{modelfile}");
    assert!(modelfile.contains("<think>"), "{modelfile}");
    let tool_log = fs::read_to_string(&log).unwrap();
    let base_hf = work.join("base-hf");
    let convert = format!(
        "python3 {} {} --outfile {} --outtype f16",
        llama.join("convert_hf_to_gguf.py").display(),
        base_hf.display(),
        work.join("base.f16.gguf").display()
    );
    let download = format!(
        "huggingface-cli download Qwen/Qwen3.5-4B --local-dir {}",
        base_hf.display()
    );
    assert!(
        stdout.contains(&convert),
        "print/run diverged\n{stdout}\n{tool_log}"
    );
    assert!(tool_log.contains(&convert), "{tool_log}");
    assert!(stdout.contains(&download), "{stdout}");
    assert!(tool_log.contains("download Qwen/Qwen3.5-4B"), "{tool_log}");
    assert!(base_hf.join("config.json").is_file());
    assert!(!stdout.contains("super-secret-hf"), "{stdout}");
    assert!(!stderr.contains("super-secret-hf"), "{stderr}");
    assert!(tool_log.contains("create tev1-base"), "{tool_log}");
    assert!(tool_log.contains("create tev1-specialist"), "{tool_log}");
    let comparison: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(work.join("comparison.json")).unwrap()).unwrap();
    assert_eq!(comparison["live_pass_recorded"], false);
    assert!((comparison["specialist_accuracy"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    assert!((comparison["base_accuracy"].as_f64().unwrap() - 0.0).abs() < 1e-9);
    assert!((comparison["delta"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    assert_eq!(comparison["threshold"], "met");
    assert_eq!(comparison["api"], "ollama-native");
    assert_eq!(comparison["base_seat"], "pipeline");
    assert_eq!(comparison["quant"], "Q4_K_M");
    assert_eq!(comparison["thinking_leak"]["base"], 1);
    assert!(stdout.contains("threshold met"), "{stdout}");
    let heavy = |log: &str| {
        log.lines()
            .filter(|line| {
                line.starts_with("train ")
                    || line.starts_with("export ")
                    || line.starts_with("python3 ")
                    || line.contains("llama-quantize")
                    || line.starts_with("create ")
                    || line.starts_with("rm ")
            })
            .count()
    };
    let heavy_after_first = heavy(&tool_log);

    let second = bin()
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
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
            "--endpoint",
            &endpoint,
            "--min-accuracy",
            "2",
            "--timeout-secs",
            "5",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("OLLAMA_STAMP", stamp.to_str().unwrap())
        .env("JOURNEY_TOOL_LOG", log.to_str().unwrap())
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
    assert!(again.contains("skip gguf-convert-specialist"), "{again}");
    assert!(again.contains("skip ollama-create-specialist"), "{again}");
    assert!(again.contains("threshold missed"), "{again}");
    let skipped_log = fs::read_to_string(&log).unwrap();
    assert_eq!(heavy(&skipped_log), heavy_after_first, "{skipped_log}");

    fs::remove_file(work.join("manifests/train.json")).unwrap();
    let stale = bin()
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
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
            "--endpoint",
            &endpoint,
            "--timeout-secs",
            "5",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("OLLAMA_STAMP", stamp.to_str().unwrap())
        .env("JOURNEY_TOOL_LOG", log.to_str().unwrap())
        .output()
        .unwrap();
    let stale_out = format!(
        "{}{}",
        String::from_utf8_lossy(&stale.stdout),
        String::from_utf8_lossy(&stale.stderr)
    );
    assert!(stale.status.success(), "{stale_out}");
    assert!(
        stale_out.contains("redo train: missing manifest"),
        "{stale_out}"
    );

    let dataset = work.join("dataset.jsonl");
    let mut rows = fs::read_to_string(&dataset).unwrap();
    rows.push_str("{}\n");
    fs::write(&dataset, rows).unwrap();
    let changed = bin()
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
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
            "--endpoint",
            &endpoint,
            "--timeout-secs",
            "5",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("OLLAMA_STAMP", stamp.to_str().unwrap())
        .env("JOURNEY_TOOL_LOG", log.to_str().unwrap())
        .output()
        .unwrap();
    let changed_out = format!(
        "{}{}",
        String::from_utf8_lossy(&changed.stdout),
        String::from_utf8_lossy(&changed.stderr)
    );
    assert!(changed.status.success(), "{changed_out}");
    assert!(
        changed_out.contains("redo train: inputs changed"),
        "{changed_out}"
    );

    fs::write(work.join("specialist.Q4_K_M.gguf"), "mutated-gguf\n").unwrap();
    let reseat = bin()
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
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
            "--endpoint",
            &endpoint,
            "--timeout-secs",
            "5",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("OLLAMA_STAMP", stamp.to_str().unwrap())
        .env("JOURNEY_TOOL_LOG", log.to_str().unwrap())
        .output()
        .unwrap();
    let reseat_out = format!(
        "{}{}",
        String::from_utf8_lossy(&reseat.stdout),
        String::from_utf8_lossy(&reseat.stderr)
    );
    assert!(reseat.status.success(), "{reseat_out}");
    assert!(
        reseat_out.contains("redo ollama-create-specialist: GGUF hash changed"),
        "{reseat_out}"
    );
    let reseat_log = fs::read_to_string(&log).unwrap();
    assert!(reseat_log.contains("rm tev1-specialist"), "{reseat_log}");
    assert!(
        reseat_log.contains("create tev1-specialist"),
        "{reseat_log}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn journey_run_stops_before_compare_when_ollama_is_down() {
    let root = std::env::temp_dir().join(format!("journey-down-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let tools = root.join("bin");
    fs::create_dir_all(&tools).unwrap();
    fake_tools(&tools);
    let llama = root.join("llama.cpp");
    fake_llama(&llama);
    let stamp = root.join("ollama-models");
    fs::write(&stamp, "").unwrap();
    let log = root.join("tools.log");
    fs::write(&log, "").unwrap();
    let input = root.join("rows.jsonl");
    fs::write(&input, tiny_jsonl()).unwrap();
    let work = root.join("work");
    let out = bin()
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
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
            "--endpoint",
            "http://127.0.0.1:9",
            "--timeout-secs",
            "1",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("OLLAMA_STAMP", stamp.to_str().unwrap())
        .env("JOURNEY_TOOL_LOG", log.to_str().unwrap())
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.status.success(), "{text}");
    assert!(text.contains("unusable score"), "{text}");
    assert!(!work.join("comparison.json").is_file(), "{text}");
    assert!(!work.join("manifests/eval-base.json").is_file(), "{text}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn journey_run_local_base_does_not_download() {
    let root = std::env::temp_dir().join(format!("journey-local-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let tools = root.join("bin");
    fs::create_dir_all(&tools).unwrap();
    fake_tools(&tools);
    let llama = root.join("llama.cpp");
    fake_llama(&llama);
    let stamp = root.join("ollama-models");
    fs::write(&stamp, "").unwrap();
    let log = root.join("tools.log");
    fs::write(&log, "").unwrap();
    let input = root.join("rows.jsonl");
    fs::write(&input, tiny_jsonl()).unwrap();
    let local = root.join("local-base");
    fs::create_dir_all(&local).unwrap();
    fs::write(local.join("config.json"), "{}\n").unwrap();
    fs::write(local.join("tokenizer.json"), "{}\n").unwrap();
    fs::write(local.join("model.safetensors"), "w\n").unwrap();
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
            let payload = serde_json::json!({"message": {"role": "assistant", "content": "B"}});
            let header =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap();
            let _ = req
                .respond(tiny_http::Response::from_string(payload.to_string()).with_header(header));
        }
    });
    let out = bin()
        .args([
            "classify",
            "journey",
            "--input",
            input.to_str().unwrap(),
            "--out",
            work.to_str().unwrap(),
            "--base",
            local.to_str().unwrap(),
            "--run",
            "--max-steps",
            "1",
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
            "--endpoint",
            &format!("http://127.0.0.1:{port}"),
            "--timeout-secs",
            "5",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("OLLAMA_STAMP", stamp.to_str().unwrap())
        .env("JOURNEY_TOOL_LOG", log.to_str().unwrap())
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "{text}");
    let tool_log = fs::read_to_string(&log).unwrap();
    assert!(!tool_log.contains("download"), "{tool_log}");
    assert!(
        text.contains(&format!("local base {}", local.display())),
        "{text}"
    );
    let convert = format!(
        "python3 {} {} --outfile {} --outtype f16",
        llama.join("convert_hf_to_gguf.py").display(),
        local.display(),
        work.join("base.f16.gguf").display()
    );
    assert!(text.contains(&convert), "{text}");
    assert!(tool_log.contains(&convert), "{tool_log}");
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
            let header =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/plain"[..]).unwrap();
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
        !out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("every row failed at the HTTP level"), "{err}");
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

fn ustar(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, data) in files {
        let mut header = [0u8; 512];
        let bytes = name.as_bytes();
        header[..bytes.len()].copy_from_slice(bytes);
        let size = format!("{:o}", data.len());
        header[124..124 + size.len()].copy_from_slice(size.as_bytes());
        header[156] = b'0';
        header[257..262].copy_from_slice(b"ustar");
        out.extend_from_slice(&header);
        out.extend_from_slice(data);
        let pad = (512 - (data.len() % 512)) % 512;
        out.extend(std::iter::repeat(0).take(pad));
    }
    out.extend(std::iter::repeat(0).take(1024));
    out
}

#[test]
fn journey_print_together_does_not_use_the_network_or_print_the_key() {
    let dir = std::env::temp_dir().join(format!("journey-together-print-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let secret = "together-secret-not-for-logs-2820";
    let out = bin()
        .args([
            "classify",
            "journey",
            "--input",
            fixture().to_str().unwrap(),
            "--out",
            dir.to_str().unwrap(),
            "--print",
            "--train-driver",
            "together",
            "--together-base-url",
            "http://127.0.0.1:9",
            "--api-key-env",
            "TOGETHER_API_KEY",
        ])
        .env("PATH", "/nonexistent-journey-path")
        .env("TOGETHER_API_KEY", secret)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("train_driver: together"), "{stdout}");
    assert!(stdout.contains("TOGETHER_API_KEY"), "{stdout}");
    assert!(stdout.contains("poll 10800s"), "{stdout}");
    assert!(stdout.contains("dry-run no network"), "{stdout}");
    assert!(stdout.contains("Qwen/Qwen3.5-4B"), "{stdout}");
    assert!(stdout.contains("finetune/download"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert!(!stdout.contains(secret), "{stdout}");
    assert!(!stderr.contains(secret), "{stderr}");
    assert!(!dir.exists(), "print must not write {}", dir.display());
    let help = bin()
        .args(["classify", "journey", "--help"])
        .output()
        .unwrap();
    let help_text = String::from_utf8_lossy(&help.stdout);
    assert!(help.status.success(), "{help_text}");
    assert!(help_text.contains("--train-driver"), "{help_text}");
    assert!(help_text.contains("TOGETHER_API_KEY"), "{help_text}");
    assert!(help_text.contains("--api-key-env"), "{help_text}");
    assert!(help_text.contains("--together-poll-secs"), "{help_text}");
    assert!(help_text.contains("10800"), "{help_text}");
}

#[test]
fn journey_together_run_uploads_polls_and_downloads_adapter() {
    let root = std::env::temp_dir().join(format!("journey-together-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let tools = root.join("bin");
    fs::create_dir_all(&tools).unwrap();
    fake_tools(&tools);
    write_exe(
        &tools,
        "llama-quantize",
        "#!/bin/sh\nprintf '%s\\n' PATH-QUANT >> \"${JOURNEY_TOOL_LOG:?}\"\nexit 1\n",
    );
    let llama = root.join("llama.cpp");
    fake_llama(&llama);
    let stamp = root.join("ollama-models");
    fs::write(&stamp, "").unwrap();
    let log = root.join("tools.log");
    fs::write(&log, "").unwrap();
    let input = root.join("rows.jsonl");
    fs::write(&input, tiny_jsonl()).unwrap();
    let work = root.join("work");
    let secret = "together-secret-not-for-logs-2820";
    let hits = root.join("together-hits");
    fs::write(&hits, "").unwrap();

    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = match server.server_addr() {
        tiny_http::ListenAddr::IP(addr) => addr.port(),
        other => panic!("expected ip listen addr, got {other:?}"),
    };
    let hits_server = hits.clone();
    let archive = {
        let raw = ustar(&[
            ("adapter_config.json", b"{}\n"),
            ("adapter_model.safetensors", b"from-together\n"),
        ]);
        let compressed = zstd::stream::encode_all(std::io::Cursor::new(raw), 0).unwrap();
        assert!(
            compressed.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]),
            "mock must serve a real tar.zst"
        );
        compressed
    };
    thread::spawn(move || {
        let mut polls = 0u32;
        for mut req in server.incoming_requests() {
            let mut body = Vec::new();
            let _ = std::io::Read::read_to_end(req.as_reader(), &mut body);
            let url = req.url().to_string();
            let auth_ok = req.headers().iter().any(|header| {
                header.field.equiv("Authorization")
                    && header.value.as_str() == format!("Bearer {secret}")
            });
            let mark = if auth_ok { "auth-ok" } else { "auth-bad" };
            let mut hit = fs::OpenOptions::new()
                .append(true)
                .open(&hits_server)
                .unwrap();
            writeln!(hit, "{mark} {url}").unwrap();
            let method = req.method().as_str().to_string();
            let (status, payload, raw) = if url.starts_with("/files") && method == "POST" {
                assert!(
                    body.windows(9).any(|w| w == b"fine-tune"),
                    "purpose missing"
                );
                assert!(body.windows(8).any(|w| w == b"messages"), "dataset missing");
                (
                    200,
                    serde_json::json!({"id":"file-tev1","processing_status":"COMPLETED"})
                        .to_string(),
                    None,
                )
            } else if url == "/files/file-tev1" {
                (
                    200,
                    serde_json::json!({"id":"file-tev1","processing_status":"COMPLETED"})
                        .to_string(),
                    None,
                )
            } else if url == "/fine-tunes" && method == "POST" {
                let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(value["training_file"], "file-tev1");
                assert_eq!(value["model"], "Qwen/Qwen3.5-4B");
                assert_eq!(value["training_type"]["type"], "Lora");
                assert_eq!(value["training_type"]["lora_r"], 8);
                assert_eq!(value["training_type"]["lora_alpha"], 16);
                assert_eq!(value["n_epochs"], 1);
                assert_eq!(value["n_checkpoints"], 1);
                assert!((value["learning_rate"].as_f64().unwrap() - 0.0001).abs() < 1e-9);
                assert_eq!(value["suffix"], "tev1");
                assert!(value.get("lora").is_none(), "{value}");
                assert!(value.get("lora_r").is_none(), "{value}");
                assert!(value.get("lora_alpha").is_none(), "{value}");
                let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
                keys.sort();
                assert_eq!(
                    keys,
                    [
                        "learning_rate",
                        "model",
                        "n_checkpoints",
                        "n_epochs",
                        "suffix",
                        "training_file",
                        "training_type"
                    ]
                );
                (
                    200,
                    serde_json::json!({"id":"ft-tev1","status":"pending"}).to_string(),
                    None,
                )
            } else if url == "/fine-tunes/ft-tev1" {
                polls += 1;
                let status = if polls == 1 { "running" } else { "completed" };
                (
                    200,
                    serde_json::json!({"id":"ft-tev1","status":status}).to_string(),
                    None,
                )
            } else if url.contains("/finetune/download")
                && url.contains("checkpoint=adapter")
                && url.contains("ft-tev1")
            {
                (200, String::new(), Some(archive.clone()))
            } else if url.contains("/api/chat") {
                (
                    200,
                    serde_json::json!({"message":{"role":"assistant","content":"B"}}).to_string(),
                    None,
                )
            } else {
                (404, format!("unexpected {url}"), None)
            };
            if let Some(bytes) = raw {
                let _ = req.respond(tiny_http::Response::from_data(bytes).with_status_code(status));
            } else {
                let header =
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .unwrap();
                let resp = tiny_http::Response::from_string(payload)
                    .with_status_code(status)
                    .with_header(header);
                let _ = req.respond(resp);
            }
        }
    });

    let endpoint = format!("http://127.0.0.1:{port}");
    let ran = bin()
        .args([
            "classify",
            "journey",
            "--input",
            input.to_str().unwrap(),
            "--out",
            work.to_str().unwrap(),
            "--run",
            "--train-driver",
            "together",
            "--together-base-url",
            &endpoint,
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
            "--endpoint",
            &endpoint,
            "--max-steps",
            "1",
            "--timeout-secs",
            "5",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("OLLAMA_STAMP", stamp.to_str().unwrap())
        .env("JOURNEY_TOOL_LOG", log.to_str().unwrap())
        .env("TOGETHER_API_KEY", secret)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&ran.stdout);
    let stderr = String::from_utf8_lossy(&ran.stderr);
    assert!(ran.status.success(), "{stdout}\n{stderr}");
    assert!(!stdout.contains(secret), "{stdout}");
    assert!(!stderr.contains(secret), "{stderr}");
    let hits_text = fs::read_to_string(&hits).unwrap();
    assert!(hits_text.contains("auth-ok /files"), "{hits_text}");
    assert!(hits_text.contains("auth-ok /fine-tunes"), "{hits_text}");
    assert!(hits_text.contains("finetune/download"), "{hits_text}");
    assert!(
        hits_text
            .lines()
            .filter(|line| !line.contains("/api/chat"))
            .all(|line| line.starts_with("auth-ok")),
        "{hits_text}"
    );
    assert!(!hits_text.contains(secret), "{hits_text}");
    let weights = fs::read(work.join("outputs/adapter_model.safetensors")).unwrap();
    assert_eq!(weights, b"from-together\n");
    assert!(work.join("outputs/adapter_config.json").is_file());
    let job: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(work.join("together-job.json")).unwrap()).unwrap();
    assert_eq!(job["job_id"], "ft-tev1");
    assert_eq!(job["live_pass_recorded"], false);
    let tool_log = fs::read_to_string(&log).unwrap();
    assert!(
        !tool_log.lines().any(|line| line.starts_with("train ")),
        "{tool_log}"
    );
    assert!(tool_log.contains("export "), "{tool_log}");
    assert!(
        tool_log.contains(&llama.join("build/bin/llama-quantize").display().to_string()),
        "{tool_log}"
    );
    assert!(!tool_log.contains("PATH-QUANT"), "{tool_log}");
    let comparison = fs::read_to_string(work.join("comparison.json")).unwrap();
    assert!(
        comparison.contains("\"live_pass_recorded\": false"),
        "{comparison}"
    );
    assert!(!comparison.contains(secret), "{comparison}");

    let missing = bin()
        .args([
            "classify",
            "journey",
            "--input",
            input.to_str().unwrap(),
            "--out",
            root.join("missing-key").to_str().unwrap(),
            "--run",
            "--train-driver",
            "together",
            "--together-base-url",
            "http://127.0.0.1:9",
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
            "--max-steps",
            "1",
            "--timeout-secs",
            "5",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("OLLAMA_STAMP", stamp.to_str().unwrap())
        .env("JOURNEY_TOOL_LOG", log.to_str().unwrap())
        .env_remove("TOGETHER_API_KEY")
        .output()
        .unwrap();
    let miss = format!(
        "{}{}",
        String::from_utf8_lossy(&missing.stdout),
        String::from_utf8_lossy(&missing.stderr)
    );
    assert!(!missing.status.success(), "{miss}");
    assert!(miss.contains("set TOGETHER_API_KEY"), "{miss}");
    assert!(!miss.contains(secret), "{miss}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn skipped_fetch_validates_snapshot_and_quantize_stays_under_llama_cpp_dir() {
    let root = std::env::temp_dir().join(format!("journey-skip-fetch-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let tools = root.join("bin");
    fs::create_dir_all(&tools).unwrap();
    fake_tools(&tools);
    let llama = root.join("llama.cpp");
    fs::create_dir_all(llama.join("build/bin")).unwrap();
    fs::write(llama.join("convert_hf_to_gguf.py"), "print('x')\n").unwrap();
    write_exe(
        &tools,
        "llama-quantize",
        "#!/bin/sh\nprintf '%s\\n' PATH-QUANT >> \"${JOURNEY_TOOL_LOG:?}\"\nexit 0\n",
    );
    let log = root.join("tools.log");
    fs::write(&log, "").unwrap();
    let input = root.join("rows.jsonl");
    fs::write(&input, tiny_jsonl()).unwrap();
    let local = root.join("local-base");
    fs::create_dir_all(&local).unwrap();
    fs::write(local.join("config.json"), "{}\n").unwrap();
    let work = root.join("work");
    let refused = bin()
        .args([
            "classify",
            "journey",
            "--input",
            input.to_str().unwrap(),
            "--out",
            work.to_str().unwrap(),
            "--base",
            local.to_str().unwrap(),
            "--run",
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
            "--max-steps",
            "1",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("JOURNEY_TOOL_LOG", &log)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(!refused.status.success(), "{text}");
    assert!(text.contains("llama-quantize is missing under"), "{text}");
    assert!(!text.contains("PATH-QUANT"), "{text}");
    let log_text = fs::read_to_string(&log).unwrap();
    assert!(!log_text.contains("PATH-QUANT"), "{log_text}");

    fake_llama(&llama);
    fs::write(local.join("tokenizer.json"), "{}\n").unwrap();
    let stamp = root.join("ollama-models");
    fs::write(&stamp, "").unwrap();
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = match server.server_addr() {
        tiny_http::ListenAddr::IP(addr) => addr.port(),
        other => panic!("expected ip listen addr, got {other:?}"),
    };
    thread::spawn(move || {
        for req in server.incoming_requests() {
            let payload = serde_json::json!({"message":{"role":"assistant","content":"B"}});
            let header =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap();
            let _ = req
                .respond(tiny_http::Response::from_string(payload.to_string()).with_header(header));
        }
    });
    fs::write(local.join("model.safetensors"), "w\n").unwrap();
    let first = bin()
        .args([
            "classify",
            "journey",
            "--input",
            input.to_str().unwrap(),
            "--out",
            work.to_str().unwrap(),
            "--base",
            local.to_str().unwrap(),
            "--run",
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
            "--endpoint",
            &format!("http://127.0.0.1:{port}"),
            "--max-steps",
            "1",
            "--timeout-secs",
            "5",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("OLLAMA_STAMP", &stamp)
        .env("JOURNEY_TOOL_LOG", &log)
        .output()
        .unwrap();
    let first_text = format!(
        "{}{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(first.status.success(), "{first_text}");
    fs::remove_file(local.join("model.safetensors")).unwrap();
    let second = bin()
        .args([
            "classify",
            "journey",
            "--input",
            input.to_str().unwrap(),
            "--out",
            work.to_str().unwrap(),
            "--base",
            local.to_str().unwrap(),
            "--run",
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
            "--endpoint",
            &format!("http://127.0.0.1:{port}"),
            "--max-steps",
            "1",
            "--timeout-secs",
            "5",
        ])
        .env("PATH", format!("{}:/bin:/usr/bin", tools.display()))
        .env("OLLAMA_STAMP", &stamp)
        .env("JOURNEY_TOOL_LOG", &log)
        .output()
        .unwrap();
    let second_text = format!(
        "{}{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(!second.status.success(), "{second_text}");
    assert!(second_text.contains("skip prepare"), "{second_text}");
    assert!(!second_text.contains("run fetch-base"), "{second_text}");
    assert!(!second_text.contains("redo fetch-base"), "{second_text}");
    assert!(second_text.contains("base snapshot"), "{second_text}");
    assert!(second_text.contains("non-empty weights"), "{second_text}");
    let _ = fs::remove_dir_all(&root);
}
