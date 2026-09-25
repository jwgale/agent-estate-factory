//! Teacher expand for rust_idiom. Injected fixtures only. No network and no live PASS.

use std::fs;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn pair(split: &str, commit: u64, old: &str, new: &str) -> String {
    let base = commit * 2;
    format!(
        "{{\"id\":\"rust_idiom:{split}:{base}\",\"group_id\":\"rust_idiom:{split}:{base}\",\"state\":{old:?},\"question\":\"Does this Rust snippet need a fix, or is it the idiomatic version?\",\"options\":[{{\"label\":\"A\",\"key\":\"needs_fix\",\"description\":\"NeedsFix\"}},{{\"label\":\"B\",\"key\":\"idiomatic\",\"description\":\"Idiomatic\"}}],\"answer\":\"A\",\"answer_key\":\"needs_fix\"}}\n{{\"id\":\"rust_idiom:{split}:{}\",\"group_id\":\"rust_idiom:{split}:{}\",\"state\":{new:?},\"question\":\"Does this Rust snippet need a fix, or is it the idiomatic version?\",\"options\":[{{\"label\":\"A\",\"key\":\"needs_fix\",\"description\":\"NeedsFix\"}},{{\"label\":\"B\",\"key\":\"idiomatic\",\"description\":\"Idiomatic\"}}],\"answer\":\"B\",\"answer_key\":\"idiomatic\"}}\n",
        base + 1,
        base + 1
    )
}

#[test]
fn expand_help_is_next_to_import_and_print_does_not_echo_the_key() {
    let help = bin()
        .args(["classify", "expand", "--help"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(help.status.success(), "{text}");
    assert!(text.contains("TEACHER_API_KEY"), "{text}");
    assert!(text.contains("OPENAI_API_KEY"), "{text}");
    assert!(text.contains("--api-key-env"), "{text}");
    assert!(text.contains("READY_FOR_LIVE_TEST"), "{text}");
    assert!(text.contains("rust_idiom"), "{text}");

    let root = std::env::temp_dir().join(format!("classify-expand-cli-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let train = root.join("train.jsonl");
    let held = root.join("heldout.jsonl");
    fs::write(
        &train,
        pair(
            "train",
            1,
            "fn weak() { let _g = mutex.lock().unwrap(); }",
            "fn strong() { let _g = mutex.lock().unwrap(); drop(_g); }",
        ),
    )
    .unwrap();
    fs::write(
        &held,
        pair(
            "test",
            8,
            "async fn held() { let _ = channel::unbounded::<u8>(); }",
            "async fn held_new() { let _ = channel::unbounded::<u8>(); }",
        ),
    )
    .unwrap();
    fs::write(root.join("import.json"), "{\"holdout_seed\":42}\n").unwrap();
    let out = root.join("plan");
    let secret = "expand-secret-not-for-logs-91";
    let printed = bin()
        .args([
            "classify",
            "expand",
            "--train",
            train.to_str().unwrap(),
            "--heldout",
            held.to_str().unwrap(),
            "--tag",
            "rev1",
            "--out",
            out.to_str().unwrap(),
            "--endpoint",
            "http://127.0.0.1:9",
            "--model",
            "teacher-test",
            "--api-key-env",
            "TEACHER_API_KEY",
        ])
        .env("TEACHER_API_KEY", secret)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&printed.stdout);
    let stderr = String::from_utf8_lossy(&printed.stderr);
    assert!(printed.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("dry-run no network"), "{stdout}");
    assert!(stdout.contains("TEACHER_API_KEY"), "{stdout}");
    assert!(!stdout.contains(secret), "{stdout}");
    assert!(!stderr.contains(secret), "{stderr}");
    let plan = fs::read_to_string(out.join("expand-plan.json")).unwrap();
    assert!(plan.contains("\"would_call_teacher\": false"), "{plan}");
    assert!(plan.contains("\"live_pass_recorded\": false"), "{plan}");
    assert!(plan.contains("\"ready_for_live_test\": \"no\""), "{plan}");
    assert!(!plan.contains(secret), "{plan}");
    assert!(!out.join("train.jsonl").exists());

    let missing = bin()
        .args([
            "classify",
            "expand",
            "--train",
            train.to_str().unwrap(),
            "--heldout",
            held.to_str().unwrap(),
            "--tag",
            "rev1",
            "--out",
            root.join("run").to_str().unwrap(),
            "--run",
            "--endpoint",
            "http://127.0.0.1:9",
            "--model",
            "teacher-test",
        ])
        .env_remove("TEACHER_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&missing.stderr);
    assert!(!missing.status.success(), "{err}");
    assert!(err.contains("set TEACHER_API_KEY"), "{err}");
    assert!(!err.contains(secret));

    let journey = bin()
        .args([
            "classify",
            "journey",
            "--dataset",
            "rust_idiom",
            "--expand-tag",
            "rev1",
            "--seed",
            "42",
            "--train-size",
            "all",
            "--print",
            "--out",
            root.join("journey").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let jout = String::from_utf8_lossy(&journey.stdout);
    let jerr = String::from_utf8_lossy(&journey.stderr);
    assert!(journey.status.success(), "{jout}\n{jerr}");
    assert!(jout.contains("expand_tag: rev1"), "{jout}");
    assert!(jout.contains("rust_idiom-all-s42-rev1"), "{jout}");
    assert!(jout.contains("no-import"), "{jout}");
    assert!(
        jout.contains("tev1-specialist-rustidiom-all-rev1"),
        "{jout}"
    );
    assert!(jout.contains("READY_FOR_LIVE_TEST: no"), "{jout}");

    let foreign = bin()
        .args([
            "classify",
            "journey",
            "--dataset",
            "ag_news",
            "--expand-tag",
            "rev1",
            "--print",
        ])
        .output()
        .unwrap();
    let ferr = String::from_utf8_lossy(&foreign.stderr);
    assert!(!foreign.status.success(), "{ferr}");
    assert!(ferr.contains("rust_idiom"), "{ferr}");
    let _ = fs::remove_dir_all(&root);
}
