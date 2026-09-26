//! Dual qwen + glm4-chat journey on one rust_idiom expand cache. No network and no live PASS.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pair(split: &str, commit: u64) -> String {
    let base = commit * 2;
    format!(
        "{{\"id\":\"rust_idiom:{split}:{base}\",\"state\":\"fn old() {{}}\",\"question\":\"Does this Rust snippet need a fix, or is it the idiomatic version?\",\"options\":[{{\"label\":\"A\",\"key\":\"needs_fix\"}},{{\"label\":\"B\",\"key\":\"idiomatic\"}}],\"answer\":\"A\"}}\n{{\"id\":\"rust_idiom:{split}:{}\",\"state\":\"fn new() {{}}\",\"question\":\"Does this Rust snippet need a fix, or is it the idiomatic version?\",\"options\":[{{\"label\":\"A\",\"key\":\"needs_fix\"}},{{\"label\":\"B\",\"key\":\"idiomatic\"}}],\"answer\":\"B\"}}\n",
        base + 1
    )
}

#[test]
fn dual_print_writes_plans_and_a_compare_stub_for_one_holdout() {
    let help = bin()
        .args(["classify", "journey", "--help"])
        .output()
        .unwrap();
    let help_text = String::from_utf8_lossy(&help.stdout);
    assert!(help.status.success(), "{help_text}");
    assert!(help_text.contains("--dual"), "{help_text}");
    assert!(help_text.contains("READY_FOR_LIVE_TEST"), "{help_text}");
    assert!(help_text.contains("glm4-chat"), "{help_text}");

    let tag = format!("dual{}", std::process::id());
    let cache = PathBuf::from(format!(".cell/classify-import/rust_idiom-all-s42-{tag}"));
    let _ = fs::remove_dir_all(&cache);
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join("train.jsonl"), pair("train", 1)).unwrap();
    let held = pair("test", 9);
    fs::write(cache.join("heldout.jsonl"), &held).unwrap();

    let out = std::env::temp_dir().join(format!("classify-dual-{}", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    let secret = "dual-secret-not-for-logs-91";
    let printed = bin()
        .args([
            "classify",
            "journey",
            "--dual",
            "--dataset",
            "rust_idiom",
            "--expand-tag",
            &tag,
            "--seed",
            "42",
            "--train-size",
            "all",
            "--print",
            "--out",
            out.to_str().unwrap(),
        ])
        .env("TOGETHER_API_KEY", secret)
        .env("TEACHER_API_KEY", secret)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&printed.stdout);
    let stderr = String::from_utf8_lossy(&printed.stderr);
    assert!(printed.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("classify journey dual: print"), "{stdout}");
    assert!(stdout.contains("no-import"), "{stdout}");
    assert!(stdout.contains("dry-run no teacher"), "{stdout}");
    assert!(stdout.contains("preset glm4-chat"), "{stdout}");
    assert!(stdout.contains("Qwen/Qwen3.5-4B"), "{stdout}");
    assert!(stdout.contains("zai-org/glm-4-9b-chat"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert!(!stdout.contains(secret), "{stdout}");
    assert!(!stderr.contains(secret), "{stderr}");

    let qwen_plan = fs::read_to_string(out.join("qwen/journey-plan.json")).unwrap();
    let glm_plan = fs::read_to_string(out.join("glm4-chat/journey-plan.json")).unwrap();
    let compare = fs::read_to_string(out.join("dual-compare.json")).unwrap();

    assert!(qwen_plan.contains("\"preset\": \"qwen\""), "{qwen_plan}");
    assert!(
        qwen_plan.contains("\"base\": \"Qwen/Qwen3.5-4B\""),
        "{qwen_plan}"
    );
    assert!(
        qwen_plan.contains("\"template\": \"qwen3_5\""),
        "{qwen_plan}"
    );
    assert!(glm_plan.contains("\"preset\": \"glm4-chat\""), "{glm_plan}");
    assert!(
        glm_plan.contains("\"base\": \"zai-org/glm-4-9b-chat\""),
        "{glm_plan}"
    );
    assert!(glm_plan.contains("\"template\": \"glm4\""), "{glm_plan}");
    assert!(qwen_plan.contains("\"out\":"), "{qwen_plan}");
    assert!(!qwen_plan.contains("glm4-chat"), "{qwen_plan}");
    assert!(
        glm_plan.contains("/glm4-chat") || glm_plan.contains("\\glm4-chat"),
        "{glm_plan}"
    );
    let qwen_sha = qwen_plan
        .lines()
        .find(|line| line.contains("heldout_sha256"))
        .unwrap();
    let glm_sha = glm_plan
        .lines()
        .find(|line| line.contains("heldout_sha256"))
        .unwrap();
    assert_eq!(qwen_sha.trim(), glm_sha.trim(), "{qwen_sha} vs {glm_sha}");
    assert!(compare.contains(qwen_sha.trim()), "{compare}");
    assert!(compare.contains("\"holdout_shared\": true"), "{compare}");
    assert!(compare.contains("\"mode\": \"print\""), "{compare}");
    assert!(
        compare.contains("\"qwen_glm_specialist_delta\": null"),
        "{compare}"
    );
    assert!(compare.contains("\"base_accuracy\": null"), "{compare}");
    assert!(
        compare.contains("\"factory_live_pass\": false"),
        "{compare}"
    );
    assert!(
        compare.contains("\"live_pass_recorded\": false"),
        "{compare}"
    );
    assert!(
        compare.contains("\"ready_for_live_test\": \"no\""),
        "{compare}"
    );
    assert!(compare.contains("not a factory live PASS"), "{compare}");
    assert!(qwen_plan.contains("\"network\": false"), "{qwen_plan}");
    assert!(glm_plan.contains("\"network\": false"), "{glm_plan}");
    assert!(
        qwen_plan.contains("\"live_pass_recorded\": false"),
        "{qwen_plan}"
    );
    assert!(!out.join("qwen/comparison.json").exists());
    assert!(!out.join("glm4-chat/comparison.json").exists());

    let foreign = bin()
        .args([
            "classify",
            "journey",
            "--dual",
            "--dataset",
            "ag_news",
            "--expand-tag",
            "rev1",
            "--print",
            "--out",
            out.join("foreign").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let ferr = String::from_utf8_lossy(&foreign.stderr);
    assert!(!foreign.status.success(), "{ferr}");
    assert!(ferr.contains("rust_idiom"), "{ferr}");

    let missing = bin()
        .args([
            "classify",
            "journey",
            "--dual",
            "--dataset",
            "rust_idiom",
            "--expand-tag",
            "missing-dual-cache",
            "--print",
            "--out",
            out.join("missing").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let merr = String::from_utf8_lossy(&missing.stderr);
    assert!(!missing.status.success(), "{merr}");
    assert!(merr.contains("expand cache"), "{merr}");
    assert!(!out.join("missing/dual-compare.json").exists());

    let deepseek = bin()
        .args([
            "classify",
            "journey",
            "--dual",
            "--preset",
            "deepseek-r1-distill",
            "--dataset",
            "rust_idiom",
            "--expand-tag",
            &tag,
            "--print",
            "--out",
            out.join("deepseek").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let derr = String::from_utf8_lossy(&deepseek.stderr);
    assert!(!deepseek.status.success(), "{derr}");
    assert!(derr.contains("DeepSeek"), "{derr}");

    let default_print = bin()
        .args([
            "classify",
            "journey",
            "--dual",
            "--dataset",
            "rust_idiom",
            "--expand-tag",
            &tag,
            "--seed",
            "42",
            "--train-size",
            "all",
            "--print",
        ])
        .output()
        .unwrap();
    let default_out = String::from_utf8_lossy(&default_print.stdout);
    let default_err = String::from_utf8_lossy(&default_print.stderr);
    assert!(
        default_print.status.success(),
        "{default_out}\n{default_err}"
    );
    let default_parent = PathBuf::from(format!(".cell/classify-journey-rustidiom-all-{tag}-dual"));
    assert!(
        default_parent.join("dual-compare.json").is_file(),
        "{}",
        default_parent.display()
    );
    assert!(default_parent.join("qwen/journey-plan.json").is_file());
    assert!(default_parent.join("glm4-chat/journey-plan.json").is_file());
    let _ = fs::remove_dir_all(&default_parent);

    let sum = Command::new("cksum")
        .arg(workspace().join("examples/estate.yaml"))
        .output()
        .unwrap();
    let sum_text = String::from_utf8_lossy(&sum.stdout);
    assert!(sum.status.success(), "{sum_text}");
    assert!(sum_text.starts_with("43770130 3391"), "{sum_text}");

    let _ = fs::remove_dir_all(&cache);
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn dual_run_replaces_a_stale_compare_before_a_student_fails() {
    let tag = format!("dualfail{}", std::process::id());
    let cache = PathBuf::from(format!(".cell/classify-import/rust_idiom-all-s42-{tag}"));
    let _ = fs::remove_dir_all(&cache);
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join("train.jsonl"), pair("train", 3)).unwrap();
    fs::write(cache.join("heldout.jsonl"), pair("test", 11)).unwrap();

    let out = std::env::temp_dir().join(format!("classify-dual-fail-{}", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    let printed = bin()
        .args([
            "classify",
            "journey",
            "--dual",
            "--dataset",
            "rust_idiom",
            "--expand-tag",
            &tag,
            "--seed",
            "42",
            "--train-size",
            "all",
            "--print",
            "--out",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        printed.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&printed.stdout),
        String::from_utf8_lossy(&printed.stderr)
    );
    let compare_path = out.join("dual-compare.json");
    let stub = fs::read_to_string(&compare_path).unwrap();
    assert!(stub.contains("\"mode\": \"print\""), "{stub}");

    // An older scored compare is the same hazard as a print stub.
    fs::write(
        &compare_path,
        "{\n  \"mode\": \"run\",\n  \"factory_live_pass\": false,\n  \"presets\": {\"qwen\": {\"specialist_accuracy\": 0.99}}\n}\n",
    )
    .unwrap();

    let llama = out.join("empty-llama");
    fs::create_dir_all(&llama).unwrap();
    let ran = bin()
        .args([
            "classify",
            "journey",
            "--dual",
            "--dataset",
            "rust_idiom",
            "--expand-tag",
            &tag,
            "--seed",
            "42",
            "--train-size",
            "all",
            "--run",
            "--out",
            out.to_str().unwrap(),
            "--llama-cpp-dir",
            llama.to_str().unwrap(),
        ])
        .env_remove("LLAMA_CPP_DIR")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&ran.stdout);
    let stderr = String::from_utf8_lossy(&ran.stderr);
    assert!(!ran.status.success(), "{stdout}\n{stderr}");
    assert!(stderr.contains("refuse:classify-journey"), "{stderr}");
    assert!(stdout.contains("in-progress"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");

    let compare = fs::read_to_string(&compare_path).unwrap();
    assert!(compare.contains("\"mode\": \"in-progress\""), "{compare}");
    assert!(!compare.contains("\"mode\": \"print\""), "{compare}");
    assert!(!compare.contains("\"mode\": \"run\""), "{compare}");
    assert!(!compare.contains("0.99"), "{compare}");
    assert!(
        compare.contains("\"qwen_glm_specialist_delta\": null"),
        "{compare}"
    );
    assert!(compare.contains("\"base_accuracy\": null"), "{compare}");
    assert!(
        compare.contains("\"specialist_accuracy\": null"),
        "{compare}"
    );
    assert!(
        compare.contains("\"factory_live_pass\": false"),
        "{compare}"
    );
    assert!(
        compare.contains("\"live_pass_recorded\": false"),
        "{compare}"
    );
    assert!(
        compare.contains("\"ready_for_live_test\": \"no\""),
        "{compare}"
    );
    assert!(compare.contains("not a factory live PASS"), "{compare}");
    assert!(compare.contains("Scores stay absent"), "{compare}");
    assert!(!out.join("glm4-chat/comparison.json").exists());

    let _ = fs::remove_dir_all(&cache);
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn dual_print_replaces_a_stale_compare_before_a_student_fails() {
    let tag = format!("dualprint{}", std::process::id());
    let cache = PathBuf::from(format!(".cell/classify-import/rust_idiom-all-s42-{tag}"));
    let _ = fs::remove_dir_all(&cache);
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join("train.jsonl"), pair("train", 5)).unwrap();
    fs::write(cache.join("heldout.jsonl"), pair("test", 13)).unwrap();

    let out = std::env::temp_dir().join(format!("classify-dual-print-fail-{}", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).unwrap();
    let compare_path = out.join("dual-compare.json");
    fs::write(
        &compare_path,
        "{\n  \"mode\": \"run\",\n  \"factory_live_pass\": false,\n  \"presets\": {\"qwen\": {\"specialist_accuracy\": 0.99}}\n}\n",
    )
    .unwrap();
    // Second student cannot be created, so print stops after qwen.
    fs::write(out.join("glm4-chat"), b"not-a-directory").unwrap();

    let printed = bin()
        .args([
            "classify",
            "journey",
            "--dual",
            "--dataset",
            "rust_idiom",
            "--expand-tag",
            &tag,
            "--seed",
            "42",
            "--train-size",
            "all",
            "--print",
            "--out",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&printed.stdout);
    let stderr = String::from_utf8_lossy(&printed.stderr);
    assert!(!printed.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("classify journey dual: print"), "{stdout}");
    assert!(stdout.contains("in-progress"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert!(out.join("qwen/journey-plan.json").is_file());
    assert!(!out.join("glm4-chat/journey-plan.json").exists());

    let compare = fs::read_to_string(&compare_path).unwrap();
    assert!(compare.contains("\"mode\": \"in-progress\""), "{compare}");
    assert!(!compare.contains("\"mode\": \"print\""), "{compare}");
    assert!(!compare.contains("\"mode\": \"run\""), "{compare}");
    assert!(!compare.contains("0.99"), "{compare}");
    assert!(
        compare.contains("\"qwen_glm_specialist_delta\": null"),
        "{compare}"
    );
    assert!(compare.contains("\"base_accuracy\": null"), "{compare}");
    assert!(
        compare.contains("\"specialist_accuracy\": null"),
        "{compare}"
    );
    assert!(
        compare.contains("\"factory_live_pass\": false"),
        "{compare}"
    );
    assert!(
        compare.contains("\"live_pass_recorded\": false"),
        "{compare}"
    );
    assert!(
        compare.contains("\"ready_for_live_test\": \"no\""),
        "{compare}"
    );
    assert!(compare.contains("not a factory live PASS"), "{compare}");
    assert!(compare.contains("Scores stay absent"), "{compare}");

    let _ = fs::remove_dir_all(&cache);
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn dual_modest_print_writes_the_short_gauge_on_both_plans() {
    let help = bin()
        .args(["classify", "journey", "--help"])
        .output()
        .unwrap();
    let help_text = String::from_utf8_lossy(&help.stdout);
    assert!(help.status.success(), "{help_text}");
    assert!(help_text.contains("--modest"), "{help_text}");
    assert!(help_text.contains("500"), "{help_text}");
    assert!(help_text.contains("max_steps"), "{help_text}");

    let tag = format!("modest{}", std::process::id());
    let cache = PathBuf::from(format!(".cell/classify-import/rust_idiom-500-s42-{tag}"));
    let _ = fs::remove_dir_all(&cache);
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join("train.jsonl"), pair("train", 2)).unwrap();
    fs::write(cache.join("heldout.jsonl"), pair("test", 8)).unwrap();

    let out = std::env::temp_dir().join(format!("classify-dual-modest-{}", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    let secret = "modest-secret-not-for-logs-91";
    let printed = bin()
        .args([
            "classify",
            "journey",
            "--dual",
            "--modest",
            "--dataset",
            "rust_idiom",
            "--expand-tag",
            &tag,
            "--seed",
            "42",
            "--print",
            "--out",
            out.to_str().unwrap(),
        ])
        .env("TOGETHER_API_KEY", secret)
        .env("TEACHER_API_KEY", secret)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&printed.stdout);
    let stderr = String::from_utf8_lossy(&printed.stderr);
    assert!(printed.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("modest: true"), "{stdout}");
    assert!(stdout.contains("train_size: 500"), "{stdout}");
    assert!(stdout.contains("max_steps: 50"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert!(!stdout.contains(secret), "{stdout}");
    assert!(!stderr.contains(secret), "{stderr}");

    let qwen_plan = fs::read_to_string(out.join("qwen/journey-plan.json")).unwrap();
    let glm_plan = fs::read_to_string(out.join("glm4-chat/journey-plan.json")).unwrap();
    let compare = fs::read_to_string(out.join("dual-compare.json")).unwrap();
    for plan in [&qwen_plan, &glm_plan] {
        assert!(plan.contains("\"train_size\": \"500\""), "{plan}");
        assert!(plan.contains("\"max_steps\": 50"), "{plan}");
        assert!(plan.contains("max_steps: 50"), "{plan}");
        assert!(plan.contains("\"ready_for_live_test\": \"no\""), "{plan}");
        assert!(plan.contains("\"live_pass_recorded\": false"), "{plan}");
        assert!(plan.contains("\"network\": false"), "{plan}");
    }
    assert!(compare.contains("\"modest\": true"), "{compare}");
    assert!(compare.contains("\"train_size\": \"500\""), "{compare}");
    assert!(compare.contains("\"max_steps\": 50"), "{compare}");
    assert!(
        compare.contains("\"factory_live_pass\": false"),
        "{compare}"
    );
    assert!(
        compare.contains("\"live_pass_recorded\": false"),
        "{compare}"
    );
    assert!(
        compare.contains("\"ready_for_live_test\": \"no\""),
        "{compare}"
    );
    assert!(compare.contains("not a factory live PASS"), "{compare}");
    assert!(!compare.contains(secret), "{compare}");

    let bare = bin()
        .args([
            "classify",
            "journey",
            "--modest",
            "--dataset",
            "rust_idiom",
            "--expand-tag",
            &tag,
            "--print",
            "--out",
            out.join("bare").to_str().unwrap(),
        ])
        .env("TOGETHER_API_KEY", secret)
        .env("TEACHER_API_KEY", secret)
        .output()
        .unwrap();
    let bare_out = String::from_utf8_lossy(&bare.stdout);
    let bare_err = String::from_utf8_lossy(&bare.stderr);
    assert!(!bare.status.success(), "{bare_out}\n{bare_err}");
    assert!(
        bare_err.contains("--modest is only valid with --dual"),
        "{bare_err}"
    );
    assert!(!bare_out.contains(secret), "{bare_out}");
    assert!(!bare_err.contains(secret), "{bare_err}");

    let fat = bin()
        .args([
            "classify",
            "journey",
            "--dual",
            "--modest",
            "--dataset",
            "rust_idiom",
            "--expand-tag",
            &tag,
            "--train-size",
            "1000",
            "--print",
            "--out",
            out.join("fat").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let fat_err = String::from_utf8_lossy(&fat.stderr);
    assert!(!fat.status.success(), "{fat_err}");
    assert!(fat_err.contains("caps --train-size at 500"), "{fat_err}");

    let small_cache = PathBuf::from(format!(".cell/classify-import/rust_idiom-200-s42-{tag}"));
    let _ = fs::remove_dir_all(&small_cache);
    fs::create_dir_all(&small_cache).unwrap();
    fs::write(small_cache.join("train.jsonl"), pair("train", 4)).unwrap();
    fs::write(small_cache.join("heldout.jsonl"), pair("test", 6)).unwrap();
    let small_out = out.join("small");
    let small = bin()
        .args([
            "classify",
            "journey",
            "--dual",
            "--modest",
            "--dataset",
            "rust_idiom",
            "--expand-tag",
            &tag,
            "--seed",
            "42",
            "--train-size",
            "200",
            "--max-steps",
            "10",
            "--print",
            "--out",
            small_out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let small_stdout = String::from_utf8_lossy(&small.stdout);
    let small_stderr = String::from_utf8_lossy(&small.stderr);
    assert!(small.status.success(), "{small_stdout}\n{small_stderr}");
    let small_plan = fs::read_to_string(small_out.join("qwen/journey-plan.json")).unwrap();
    let small_glm = fs::read_to_string(small_out.join("glm4-chat/journey-plan.json")).unwrap();
    let small_compare = fs::read_to_string(small_out.join("dual-compare.json")).unwrap();
    assert!(
        small_plan.contains("\"train_size\": \"200\""),
        "{small_plan}"
    );
    assert!(small_plan.contains("\"max_steps\": 10"), "{small_plan}");
    assert!(small_glm.contains("\"train_size\": \"200\""), "{small_glm}");
    assert!(small_glm.contains("\"max_steps\": 10"), "{small_glm}");
    assert!(
        small_compare.contains("\"modest\": true"),
        "{small_compare}"
    );
    assert!(
        small_compare.contains("\"train_size\": \"200\""),
        "{small_compare}"
    );
    assert!(
        small_compare.contains("\"max_steps\": 10"),
        "{small_compare}"
    );
    assert!(
        small_compare.contains("explicit max_steps kept"),
        "{small_compare}"
    );

    let _ = fs::remove_dir_all(&cache);
    let _ = fs::remove_dir_all(&small_cache);
    let _ = fs::remove_dir_all(&out);
}
