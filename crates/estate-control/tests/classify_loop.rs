//! tev1-style classify prepare and eval. No network except an in-process mock server.

use std::path::PathBuf;
use std::process::Command;
use std::thread;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/fixtures/tev1-decisions.jsonl")
}

#[test]
fn prepare_is_deterministic_and_llama_factory_shaped() {
    let dir = std::env::temp_dir().join(format!(
        "classify-prepare-{}-{}",
        std::process::id(),
        "a"
    ));
    let dir_b = std::env::temp_dir().join(format!(
        "classify-prepare-{}-{}",
        std::process::id(),
        "b"
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir_b);
    for out in [&dir, &dir_b] {
        let status = bin()
            .args([
                "classify",
                "prepare",
                "--input",
                fixture().to_str().unwrap(),
                "--out",
                out.to_str().unwrap(),
                "--seed",
                "20260920",
                "--held-out-ratio",
                "0.2",
                "--format",
                "sharegpt",
            ])
            .status()
            .unwrap();
        assert!(status.success());
    }
    let a = std::fs::read(dir.join("dataset.jsonl")).unwrap();
    let b = std::fs::read(dir_b.join("dataset.jsonl")).unwrap();
    assert_eq!(a, b);
    let held_a = std::fs::read(dir.join("heldout.jsonl")).unwrap();
    let held_b = std::fs::read(dir_b.join("heldout.jsonl")).unwrap();
    assert_eq!(held_a, held_b);
    let train: Vec<_> = String::from_utf8(a)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .collect();
    let held: Vec<_> = String::from_utf8(held_a)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .collect();
    assert_eq!(train.len() + held.len(), 36);
    assert_eq!(held.len(), 7);
    for row in &train {
        let msgs = row["messages"].as_array().unwrap();
        let letter = msgs
            .iter()
            .rev()
            .find(|m| m["role"] == "assistant")
            .unwrap()["content"]
            .as_str()
            .unwrap();
        assert_eq!(letter.chars().count(), 1);
        assert!(letter.chars().next().unwrap().is_ascii_uppercase());
    }
    let info: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("dataset_info.json")).unwrap())
            .unwrap();
    assert_eq!(info["tev1_decisions"]["formatting"], "sharegpt");
    assert_eq!(info["tev1_decisions"]["file_name"], "dataset.jsonl");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir_b);
}

#[test]
fn strict_refuses_bad_rows_and_skip_writes() {
    let base = std::env::temp_dir().join(format!("classify-bad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let input = base.join("mixed.jsonl");
    std::fs::write(
        &input,
        concat!(
            "{\"state\":\"The note is blank on purpose but not empty.\",\"question\":\"Pick a letter.\",\"options\":[{\"label\":\"A\",\"key\":\"one\",\"description\":\"First.\"},{\"label\":\"B\",\"key\":\"two\",\"description\":\"Second.\"}],\"answer\":\"A\",\"answer_key\":\"one\"}\n",
            "{\"state\":\"\",\"question\":\"Pick a letter.\",\"options\":[],\"answer\":\"\"}\n",
            "{\"state\":\"Another short original note.\",\"question\":\"Pick a letter.\",\"options\":[{\"label\":\"A\",\"key\":\"one\",\"description\":\"First.\"},{\"label\":\"B\",\"key\":\"two\",\"description\":\"Second.\"}],\"answer\":\"B\",\"answer_key\":\"two\"}\n",
        ),
    )
    .unwrap();
    let strict_out = base.join("strict");
    let strict = bin()
        .args([
            "classify",
            "prepare",
            "--input",
            input.to_str().unwrap(),
            "--out",
            strict_out.to_str().unwrap(),
            "--strict",
        ])
        .output()
        .unwrap();
    assert!(!strict.status.success());
    let err = String::from_utf8_lossy(&strict.stderr);
    assert!(err.contains("refuse:classify"), "{err}");
    assert!(err.contains("1 bad"), "{err}");
    assert!(err.contains("line 2"), "{err}");
    assert!(!strict_out.join("dataset.jsonl").exists());

    let skip_out = base.join("skip");
    let skip = bin()
        .args([
            "classify",
            "prepare",
            "--input",
            input.to_str().unwrap(),
            "--out",
            skip_out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(skip.status.success(), "{}", String::from_utf8_lossy(&skip.stderr));
    let err = String::from_utf8_lossy(&skip.stderr);
    assert!(err.contains("skipped 1 bad"), "{err}");
    assert!(err.contains("line 2"), "{err}");
    let train = std::fs::read_to_string(skip_out.join("dataset.jsonl")).unwrap();
    let held = std::fs::read_to_string(skip_out.join("heldout.jsonl")).unwrap();
    let n = train.lines().filter(|l| !l.is_empty()).count()
        + held.lines().filter(|l| !l.is_empty()).count();
    assert_eq!(n, 2);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn eval_dry_run_and_mock_stay_offline() {
    let dir = std::env::temp_dir().join(format!("classify-mock-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let prep = bin()
        .args([
            "classify",
            "prepare",
            "--input",
            fixture().to_str().unwrap(),
            "--out",
            dir.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(prep.success());
    let held = dir.join("heldout.jsonl");
    let dry = dir.join("dry.json");
    let dry_out = bin()
        .args([
            "classify",
            "eval",
            "--records",
            held.to_str().unwrap(),
            "--model",
            "mock-model",
            "--dry-run",
            "--report",
            dry.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(dry_out.status.success(), "{}", String::from_utf8_lossy(&dry_out.stderr));
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&dry).unwrap()).unwrap();
    assert_eq!(report["mode"], "dry_run");
    assert_eq!(report["records"], 7);
    assert!(report["accuracy"].is_null());
    assert_eq!(report["live_pass_recorded"], false);
    assert!(report["sample_request"]["temperature"].as_i64() == Some(0) || report["sample_request"]["temperature"].as_f64() == Some(0.0));
    let sample = serde_json::to_string(&report["sample_request"]).unwrap();
    assert!(!sample.to_ascii_lowercase().contains("bearer"));
    assert!(sample.contains("enable_thinking"));

    let mock_path = dir.join("mock.json");
    let mock = bin()
        .args([
            "classify",
            "eval",
            "--records",
            held.to_str().unwrap(),
            "--model",
            "mock",
            "--mock",
            "--report",
            mock_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(mock.status.success(), "{}", String::from_utf8_lossy(&mock.stderr));
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&mock_path).unwrap()).unwrap();
    assert_eq!(report["mode"], "mock");
    assert_eq!(report["records"], 7);
    assert!(report["accuracy"].as_f64().unwrap() > 0.0);
    assert!(report["accuracy"].as_f64().unwrap() < 1.0);
    assert!(report["invalid"].as_u64().unwrap() >= 1);
    assert!(report["confusion"]["A"].is_object() || report["confusion"]["B"].is_object());
    assert_eq!(report["live_pass_recorded"], false);
    assert_eq!(report["latency_measured"], false);
    let stdout = String::from_utf8_lossy(&mock.stdout);
    assert!(!stdout.contains("TOGETHER_API_KEY="));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn eval_scores_in_process_http_server() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = match server.server_addr() {
        tiny_http::ListenAddr::IP(addr) => addr.port(),
        other => panic!("expected ip listen addr, got {other:?}"),
    };
    thread::spawn(move || {
        for (n, mut req) in server.incoming_requests().enumerate() {
            let mut body = String::new();
            let _ = req.as_reader().read_to_string(&mut body);
            let content = match n % 4 {
                0 => "B",
                1 => " b.",
                2 => "Answer: C",
                _ => "garbage",
            };
            let payload = serde_json::json!({
                "choices": [{"message": {"content": content}}]
            });
            let header = tiny_http::Header::from_bytes(
                &b"Content-Type"[..],
                &b"application/json"[..],
            )
            .unwrap();
            let resp = tiny_http::Response::from_string(payload.to_string()).with_header(header);
            let _ = req.respond(resp);
        }
    });

    let dir = std::env::temp_dir().join(format!("classify-http-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let records = dir.join("heldout.jsonl");
    std::fs::write(
        &records,
        concat!(
            "{\"id\":\"h1\",\"state\":\"Note one about a blue ticket.\",\"question\":\"Which letter?\",\"options\":[{\"label\":\"A\",\"key\":\"a\",\"description\":\"A.\"},{\"label\":\"B\",\"key\":\"b\",\"description\":\"B.\"},{\"label\":\"C\",\"key\":\"c\",\"description\":\"C.\"}],\"answer\":\"B\",\"answer_key\":\"b\"}\n",
            "{\"id\":\"h2\",\"state\":\"Note two about a green ticket.\",\"question\":\"Which letter?\",\"options\":[{\"label\":\"A\",\"key\":\"a\",\"description\":\"A.\"},{\"label\":\"B\",\"key\":\"b\",\"description\":\"B.\"},{\"label\":\"C\",\"key\":\"c\",\"description\":\"C.\"}],\"answer\":\"B\",\"answer_key\":\"b\"}\n",
            "{\"id\":\"h3\",\"state\":\"Note three about a red ticket.\",\"question\":\"Which letter?\",\"options\":[{\"label\":\"A\",\"key\":\"a\",\"description\":\"A.\"},{\"label\":\"B\",\"key\":\"b\",\"description\":\"B.\"},{\"label\":\"C\",\"key\":\"c\",\"description\":\"C.\"}],\"answer\":\"C\",\"answer_key\":\"c\"}\n",
            "{\"id\":\"h4\",\"state\":\"Note four about a yellow ticket.\",\"question\":\"Which letter?\",\"options\":[{\"label\":\"A\",\"key\":\"a\",\"description\":\"A.\"},{\"label\":\"B\",\"key\":\"b\",\"description\":\"B.\"},{\"label\":\"C\",\"key\":\"c\",\"description\":\"C.\"}],\"answer\":\"A\",\"answer_key\":\"a\"}\n",
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
        .env_remove("TOGETHER_API_KEY")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stdout {}\nstderr {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    assert_eq!(report["mode"], "http");
    assert_eq!(report["records"], 4);
    assert_eq!(report["correct"], 3);
    assert_eq!(report["invalid"], 1);
    assert_eq!(report["http_errors"], 0);
    assert!((report["accuracy"].as_f64().unwrap() - 0.75).abs() < 1e-9);
    assert_eq!(report["confusion"]["B"]["B"], 2);
    assert_eq!(report["confusion"]["C"]["C"], 1);
    assert_eq!(report["confusion"]["A"]["invalid"], 1);
    assert!(report["latency_ms"]["p50"].as_f64().unwrap() >= 0.0);
    assert!(report["latency_ms"]["p95"].as_f64().unwrap() >= report["latency_ms"]["p50"].as_f64().unwrap());
    assert_eq!(report["latency_measured"], true);
    assert_eq!(report["live_pass_recorded"], false);
    let rendered = serde_json::to_string(&report).unwrap();
    assert!(!rendered.contains("sk-"));
    let _ = std::fs::remove_dir_all(&dir);
}
