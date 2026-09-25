//! Opt-in MultiPL-E / HumanEvalPack compile-and-test grader. No network and no live PASS.

use std::fs;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

#[test]
fn grade_help_names_both_presets_and_print_is_the_default() {
    let help = bin()
        .args(["classify", "grade", "--help"])
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&help.stdout);
    assert!(help.status.success(), "{out}");
    assert!(out.contains("humanevalpack_rust"), "{out}");
    assert!(out.contains("multiple_rust"), "{out}");
    assert!(out.contains("bigcode/humanevalpack"), "{out}");
    assert!(out.contains("nuprl/MultiPL-E"), "{out}");
    assert!(
        out.contains("does not download")
            || out.contains("Does not compile")
            || out.contains("does not compile"),
        "{out}"
    );
    assert!(out.contains("READY_FOR_LIVE_TEST"), "{out}");
}

#[test]
fn cli_print_refuses_unknown_and_run_scores_a_fixture() {
    let root = std::env::temp_dir().join(format!("classify-grade-cli-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let unknown = bin()
        .args([
            "classify",
            "grade",
            "--dataset",
            "devign",
            "--print",
            "--out",
            root.join("unknown").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let unknown_err = String::from_utf8_lossy(&unknown.stderr);
    assert!(!unknown.status.success(), "{unknown_err}");
    assert!(
        unknown_err.contains("refuse:classify-grade: unknown dataset"),
        "{unknown_err}"
    );

    let plan = root.join("plan");
    let printed = bin()
        .args([
            "classify",
            "grade",
            "--dataset",
            "humanevalpack_rust",
            "--out",
            plan.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let printed_out = String::from_utf8_lossy(&printed.stdout);
    assert!(
        printed.status.success(),
        "{printed_out} {}",
        String::from_utf8_lossy(&printed.stderr)
    );
    assert!(printed_out.contains("\"mode\": \"print\""), "{printed_out}");
    assert!(
        printed_out.contains("\"live_pass_recorded\": false"),
        "{printed_out}"
    );
    assert!(
        printed_out.contains("\"would_compile\": false"),
        "{printed_out}"
    );
    assert!(!plan.join("grade-report.json").exists());

    let tasks = root.join("tasks.jsonl");
    fs::write(
        &tasks,
        "{\"id\":\"add\",\"dataset\":\"humanevalpack_rust\",\"prompt\":\"fn add(a: i32, b: i32) -> i32 {\\n\",\"tests\":\"fn main() {\\n    assert_eq!(add(1, 2), 3);\\n}\\n\",\"canonical\":\"    a + b\\n}\\n\"}\n",
    )
    .unwrap();
    let good = root.join("good.jsonl");
    fs::write(
        &good,
        "{\"id\":\"add\",\"completion\":\"    a + b\\n}\\n\"}\n",
    )
    .unwrap();
    let bad = root.join("bad.jsonl");
    fs::write(
        &bad,
        "{\"id\":\"add\",\"completion\":\"    a - b\\n}\\n\"}\n",
    )
    .unwrap();

    let good_out = root.join("good-out");
    let good_run = bin()
        .args([
            "classify",
            "grade",
            "--dataset",
            "humanevalpack_rust",
            "--tasks",
            tasks.to_str().unwrap(),
            "--completions",
            good.to_str().unwrap(),
            "--run",
            "--out",
            good_out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        good_run.status.success(),
        "{} {}",
        String::from_utf8_lossy(&good_run.stdout),
        String::from_utf8_lossy(&good_run.stderr)
    );
    let good_report = fs::read_to_string(good_out.join("grade-report.json")).unwrap();
    assert!(good_report.contains("\"passed\": 1"), "{good_report}");
    assert!(
        good_report.contains("\"live_pass_recorded\": false"),
        "{good_report}"
    );
    assert!(
        good_report.contains("\"ready_for_live_test\": \"no\""),
        "{good_report}"
    );

    let bad_out = root.join("bad-out");
    let bad_run = bin()
        .args([
            "classify",
            "grade",
            "--tasks",
            tasks.to_str().unwrap(),
            "--completions",
            bad.to_str().unwrap(),
            "--run",
            "--out",
            bad_out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        bad_run.status.success(),
        "{}",
        String::from_utf8_lossy(&bad_run.stderr)
    );
    let bad_report = fs::read_to_string(bad_out.join("grade-report.json")).unwrap();
    assert!(bad_report.contains("\"passed\": 0"), "{bad_report}");
    assert!(
        bad_report.contains("\"live_pass_recorded\": false"),
        "{bad_report}"
    );
    let _ = fs::remove_dir_all(&root);
}
