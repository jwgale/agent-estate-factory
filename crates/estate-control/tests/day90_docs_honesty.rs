//! Gate and parking-lot pages keep stubs off the live-ok column.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn gate_and_parking_lot_do_not_call_stubs_ready() {
    let root = repo_root();
    let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
    let plus = std::fs::read_to_string(root.join("docs/DAY90-PLUS.md")).unwrap();
    assert!(
        !gate.contains("ready for Ollama"),
        "gate must not call the Ollama complete path ready"
    );
    assert!(
        gate.contains("Mac complete is recorded"),
        "gate must record the Mac specialist completion"
    );
    assert!(
        !gate.contains("Mac complete is not recorded"),
        "gate must not leave the Mac completion unrecorded"
    );
    assert!(
        gate.contains("not live-ok"),
        "gate must say mlx/vllm/trt are not live-ok"
    );
    assert!(plus.contains("vLLM"), "{plus}");
    assert!(plus.contains("TRT"), "{plus}");
    assert!(
        plus.contains("Not live-ok"),
        "parking lot must mark vLLM and TRT not live-ok"
    );
    assert!(
        !plus.contains("READY_FOR_LIVE_TEST: yes") && !plus.contains("READY_FOR_LIVE_TEST`: yes"),
        "parking lot must not open a live hand-off"
    );
}

#[test]
fn gate_90_tip_names_cell_one_through_pr_143() {
    let root = repo_root();
    let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
    assert!(
        !gate.contains("PR #1–#54 plus this slice"),
        "GATE-90 must not freeze the tip story at PR #54"
    );
    let head: String = gate.lines().take(8).collect::<Vec<_>>().join("\n");
    assert!(
        head.contains("CELL-ONE-STATUS.md"),
        "GATE-90 header must point at the live tip snapshot: {head}"
    );
    assert!(
        head.contains("through PR #143"),
        "GATE-90 header must name tip through PR #143: {head}"
    );
    assert!(
        !head.contains("through PR #142"),
        "GATE-90 header must not freeze tip at the prepare walk: {head}"
    );
    assert!(
        head.contains("3acdec3983ea581976649ba4b7cc41a4cd22d31d"),
        "GATE-90 header must name the PR #143 tip SHA: {head}"
    );
    assert!(
        head.contains("PR #140"),
        "GATE-90 header must keep the beachhead matrix at PR #140: {head}"
    );
    assert!(
        head.contains("prepare walk (PR #142)"),
        "GATE-90 header must keep the prepare walk at PR #142: {head}"
    );
    assert!(
        head.contains("make lf-beachhead-prepare"),
        "GATE-90 header must name the prepare walk: {head}"
    );
    assert!(
        head.contains("READY_FOR_LIVE_TEST`: no") || head.contains("READY_FOR_LIVE_TEST: no"),
        "GATE-90 header must keep READY_FOR_LIVE_TEST no: {head}"
    );
    assert!(
        !gate.contains("READY_FOR_LIVE_TEST: yes") && !gate.contains("READY_FOR_LIVE_TEST`: yes"),
        "GATE-90 must not flip READY_FOR_LIVE_TEST"
    );

    let remaining = gate
        .split("## Remaining Day-90+ (honest)")
        .nth(1)
        .expect("remaining section");
    for target in [
        "make qlora-journey",
        "make lora-journey",
        "make seat-journey",
        "make lf-beachhead-prepare",
    ] {
        assert!(
            remaining.contains(target),
            "remaining table missing {target}"
        );
    }
    let row = remaining
        .lines()
        .find(|line| line.contains("lf-beachhead-prepare"))
        .expect("remaining row for lf-beachhead-prepare");
    assert!(
        row.contains("16 LLaMA-Factory beachhead matrix fixtures"),
        "{row}"
    );
    assert!(row.contains("Checks prepare artifacts"), "{row}");
    assert!(row.contains("Does not train"), "{row}");
    assert!(row.contains("Not in smoke or Actions"), "{row}");
    assert!(row.contains("Not a live train"), "{row}");

    let changelog = std::fs::read_to_string(root.join("CHANGELOG.md")).unwrap();
    let slice = changelog
        .split("## This slice — Day-90 gate tip honesty through PR #142")
        .nth(1)
        .expect("CHANGELOG missing the Day-90 tip-honesty slice")
        .split("## This slice —")
        .next()
        .unwrap();
    assert!(
        slice.contains("d2dcdb97c2c960e8b93715391d77075055a8b0ce"),
        "{slice}"
    );
    assert!(slice.contains("CELL-ONE-STATUS.md"), "{slice}");
    assert!(slice.contains("make lf-beachhead-prepare"), "{slice}");
    assert!(
        slice.contains("READY_FOR_LIVE_TEST`: no") || slice.contains("READY_FOR_LIVE_TEST: no"),
        "{slice}"
    );
    assert!(
        !slice.contains("READY_FOR_LIVE_TEST: yes") && !slice.contains("READY_FOR_LIVE_TEST`: yes"),
        "{slice}"
    );
    assert!(!slice.to_ascii_lowercase().contains("kimi/"), "{slice}");
}

#[test]
fn uniqueness_ladder_chains_target_c_prints_and_stays_off_smoke() {
    let root = repo_root();
    let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
    assert!(
        makefile
            .lines()
            .any(|line| line.trim() == "uniqueness-ladder:"),
        "Makefile missing uniqueness-ladder"
    );
    assert!(makefile.contains("scripts/uniqueness-ladder.sh"));
    assert!(
        makefile.contains("Do not add to smoke, gate-90, or GitHub Actions"),
        "uniqueness-ladder must stay off smoke, gate-90, and Actions"
    );
    let phony = makefile.lines().next().unwrap_or("");
    assert!(
        phony.contains("uniqueness-ladder"),
        "uniqueness-ladder must be a phony target"
    );
    let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
    assert!(
        !gate90.contains("uniqueness-ladder"),
        "gate-90 must not run uniqueness-ladder: {gate90}"
    );
    let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
    assert!(
        !smoke.contains("uniqueness-ladder"),
        "smoke must not run uniqueness-ladder: {smoke}"
    );

    let script_path = root.join("scripts/uniqueness-ladder.sh");
    assert!(
        script_path.is_file(),
        "scripts/uniqueness-ladder.sh missing"
    );
    let script = std::fs::read_to_string(&script_path).unwrap();
    for needle in [
        "Print-only",
        "READY_FOR_LIVE_TEST: no",
        "Does not train, merge, convert, seat, or promote.",
        "make qlora-journey",
        "make seat-journey",
        "Live train, live convert, and live seat still need a human GPU host and stay skipped.",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
    ] {
        assert!(
            script.contains(needle),
            "uniqueness-ladder missing {needle}"
        );
    }
    assert!(
        !script.contains("READY_FOR_LIVE_TEST: yes"),
        "uniqueness-ladder must keep READY_FOR_LIVE_TEST no"
    );
    let commands: Vec<&str> = script
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with('#') && !trimmed.starts_with("echo")
        })
        .collect();
    let joined = commands.join("\n");
    let qlora = joined
        .find("qlora-journey")
        .expect("chain must run qlora-journey");
    let seat = joined
        .find("seat-journey")
        .expect("chain must run seat-journey");
    assert!(qlora < seat, "qlora-journey must run before seat-journey");
    assert!(
        !joined.contains("lf-beachhead-prepare"),
        "uniqueness-ladder must not run lf-beachhead-prepare"
    );
    let shells_out = commands.iter().any(|line| {
        line.contains("llamafactory-cli")
            || line.contains("convert_hf_to_gguf.py")
            || line.contains("ollama ")
            || line.contains("import-trained")
            || line.contains("gguf-convert")
            || line.contains("local-seat")
            || line.contains("merge-adapt")
    });
    assert!(
        !shells_out,
        "uniqueness-ladder must not train, merge, convert, seat, or import"
    );

    let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
    assert!(
        !gate.contains("READY_FOR_LIVE_TEST: yes") && !gate.contains("READY_FOR_LIVE_TEST`: yes"),
        "GATE-90 must not flip READY_FOR_LIVE_TEST"
    );
    let remaining = gate
        .split("## Remaining Day-90+ (honest)")
        .nth(1)
        .expect("remaining section");
    let row = remaining
        .lines()
        .find(|line| line.contains("`make uniqueness-ladder`"))
        .expect("remaining row for uniqueness-ladder");
    assert!(row.contains("qlora-journey then seat-journey"), "{row}");
    assert!(row.contains("Does not train"), "{row}");
    assert!(row.contains("Not in smoke or Actions"), "{row}");
    assert!(row.contains("Not a live train"), "{row}");

    let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
    let uniq = status
        .split("## Train/enrich uniqueness (matrix PR #140, prepare walk PR #142)")
        .nth(1)
        .expect("uniqueness section")
        .split("\n## ")
        .next()
        .unwrap();
    assert!(uniq.contains("make uniqueness-ladder"), "{uniq}");
    assert!(
        uniq.contains("make qlora-journey") && uniq.contains("make seat-journey"),
        "{uniq}"
    );
    assert!(
        !uniq.contains("READY_FOR_LIVE_TEST: yes") && !uniq.contains("READY_FOR_LIVE_TEST`: yes"),
        "uniqueness section must keep READY_FOR_LIVE_TEST no"
    );

    let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
    assert!(journey.contains("make uniqueness-ladder"));
    let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
    assert!(train.contains("make uniqueness-ladder"));
    assert!(
        !journey.contains("READY_FOR_LIVE_TEST: yes")
            && !train.contains("READY_FOR_LIVE_TEST: yes"),
        "operator pages must keep READY_FOR_LIVE_TEST no"
    );

    let changelog = std::fs::read_to_string(root.join("CHANGELOG.md")).unwrap();
    let slice = changelog
        .split("## This slice — print-only Target C uniqueness ladder")
        .nth(1)
        .expect("CHANGELOG missing the uniqueness ladder slice")
        .split("## This slice —")
        .next()
        .unwrap();
    assert!(slice.contains("make uniqueness-ladder"), "{slice}");
    assert!(slice.contains("make qlora-journey"), "{slice}");
    assert!(slice.contains("make seat-journey"), "{slice}");
    assert!(
        slice.contains("READY_FOR_LIVE_TEST`: no") || slice.contains("READY_FOR_LIVE_TEST: no"),
        "{slice}"
    );
    assert!(
        !slice.contains("READY_FOR_LIVE_TEST: yes") && !slice.contains("READY_FOR_LIVE_TEST`: yes"),
        "{slice}"
    );
    assert!(!slice.to_ascii_lowercase().contains("kimi/"), "{slice}");

    for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("uniqueness-ladder"),
            "{rel} must not run uniqueness-ladder"
        );
    }
}

#[test]
fn glossary_keeps_purpose_built_slm_in_suite() {
    let root = repo_root();
    let glossary = std::fs::read_to_string(root.join("docs/UBIQUITOUS_LANGUAGE.md")).unwrap();
    for term in [
        "estate",
        "lane",
        "plan",
        "apply",
        "lease",
        "pack",
        "curator",
        "frontier",
        "local",
        "sacred",
        "refuse",
        "placement",
        "convey",
        "purpose-built",
        "local runtime",
        "lease-bound hop stub",
        "entrant",
        "integrate-vs-invent",
        "facilitation",
    ] {
        assert!(
            glossary.to_lowercase().contains(term),
            "glossary missing {term}"
        );
    }
    assert!(
        glossary.contains("ollama"),
        "glossary must name the ollama driver"
    );
    assert!(
        glossary.contains("TrainEnrichDriver"),
        "glossary must name TrainEnrichDriver"
    );
    assert!(glossary.contains("purpose-built SLM"));
    assert!(glossary.contains("ollama-modelfile"));
    assert!(glossary.contains("external-manifest"));
    assert!(
        !glossary.contains("Not a distillation"),
        "glossary must not ban distillation"
    );

    let north = std::fs::read_to_string(root.join("docs/NORTH-STAR.md")).unwrap();
    let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
    let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
    let plus = std::fs::read_to_string(root.join("docs/DAY90-PLUS.md")).unwrap();
    let probes = std::fs::read_to_string(root.join("docs/LIVE-PROBES.md")).unwrap();
    for (name, text) in [
        ("NORTH-STAR", north.as_str()),
        ("README", readme.as_str()),
        ("help", help.as_str()),
        ("DAY90-PLUS", plus.as_str()),
        ("LIVE-PROBES", probes.as_str()),
    ] {
        assert!(
            !text.contains("Not a distillation") && !text.contains("Distillation and an AI gateway"),
            "{name} still parks distillation"
        );
    }
    assert!(north.contains("purpose-built"));
    assert!(north.contains("entrant"));
    assert!(north.contains("UBIQUITOUS_LANGUAGE.md"));
    assert!(readme.contains("purpose-built"));
    assert!(readme.contains("docs/UBIQUITOUS_LANGUAGE.md"));
    assert!(help.contains("purpose-built"));
    assert!(help.contains("estate help enrich"));
    assert!(plus.contains("purpose-built"));
    let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
    assert!(train.contains("purpose-built"));
    assert!(train.contains("TrainEnrichDriver"));
    assert!(train.contains("external-manifest"));
    assert!(train.contains("ollama create"));
    assert!(!train.contains("READY_FOR_LIVE_TEST: yes"), "{train}");
    for slop in ["delve", "it's worth noting", "rather than"] {
        assert!(!glossary.to_lowercase().contains(slop), "glossary slop: {slop}");
        assert!(!north.to_lowercase().contains(slop), "north-star slop: {slop}");
    }
}
