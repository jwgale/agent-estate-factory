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
fn unsloth_doctor_status_honesty_stays_optional_and_keeps_the_tip() {
let root = repo_root();

let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
assert!(gate.contains("status=optional") && gate.contains("live=false"), "{gate}");
assert!(gate.contains("unsloth-qlora") && gate.contains("unsloth-lora"));
assert!(
        !gate.contains("READY_FOR_LIVE_TEST: yes") && !gate.contains("READY_FOR_LIVE_TEST`: yes")
    );
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
assert!(status.contains("| unsloth doctor status |"));
assert!(status.contains("status=optional") && status.contains("live=false"));
assert!(status.contains("do not claim a train, a promote, a live PASS, or a prepare count"));
}

#[test]
fn mlx_lm_doctor_status_honesty_stays_optional_and_keeps_the_tip() {
let root = repo_root();

let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
assert!(gate.contains("| mlx-lm doctor and status |"));
assert!(gate.contains("mlx-lm-lora"));
assert!(gate.contains("status=optional") && gate.contains("live=false"));
assert!(gate.contains("does not call mlx-lm"));
assert!(
        !gate.contains("READY_FOR_LIVE_TEST: yes") && !gate.contains("READY_FOR_LIVE_TEST`: yes")
    );
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
assert!(status.contains("| mlx-lm doctor status |"));
assert!(status.contains("status=optional") && status.contains("live=false"));
assert!(status.contains("do not claim a train, a promote, a live PASS, or a prepare count"));
assert!(status.contains("does not call mlx-lm"));
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
        }
)
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
    }
);
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
fn train_next_prints_target_c_recipe_and_stays_off_smoke() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        makefile.lines().any(|line| line.trim() == "train-next:"),
        "Makefile missing train-next"
    );
assert!(makefile.contains("scripts/train-next.sh"));
assert!(
        makefile.contains("Do not add to smoke, gate-90, or GitHub Actions"),
        "train-next must stay off smoke, gate-90, and Actions"
    );
let phony = makefile.lines().next().unwrap_or("");
assert!(
        phony.contains("train-next"),
        "train-next must be a phony target"
    );
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("train-next"),
        "gate-90 must not run train-next: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("train-next"),
        "smoke must not run train-next: {smoke}"
    );
let script_path = root.join("scripts/train-next.sh");
assert!(script_path.is_file(), "scripts/train-next.sh missing");
let script = std::fs::read_to_string(&script_path).unwrap();
for needle in [
        "Print-only",
        "READY_FOR_LIVE_TEST: no",
        "SKIP live train",
        "CELL_TRAIN_LIVE=1 is set. This journey stays print-only.",
        "llamafactory-qlora",
        "Qwen/Qwen2.5-0.5B-Instruct",
        "llama3",
        "cksum",
        "examples/estate.yaml",
        "pip install llamafactory",
        "pip install 'bitsandbytes>=0.49'",
        "Live train recipe from NEXT.md (not executed):",
        "SKIP  llamafactory-cli (not on PATH; informational)",
        "SKIP  bitsandbytes (not importable; informational)",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "make uniqueness-ladder stays qlora-journey then seat-journey.",
    ] {
        assert!(script.contains(needle), "train-next missing {needle}");
    }
assert!(
        !script.contains("READY_FOR_LIVE_TEST: yes"),
        "train-next must keep READY_FOR_LIVE_TEST no"
    );
let shells_out = script.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#')
            || trimmed.starts_with("echo")
            || trimmed.starts_with("grep")
            || trimmed.starts_with("if grep")
            || trimmed.starts_with("if ! grep")
            || trimmed.starts_with("if command -v")
            || trimmed.starts_with("if ! command -v")
            || trimmed.contains("[[ -e")
        {
            return false;
        }
        trimmed.contains("llamafactory-cli")
            || trimmed.contains("pip install")
            || trimmed.contains("convert_hf_to_gguf.py")
            || trimmed.contains("ollama ")
            || trimmed.contains("merge-adapt")
            || trimmed.contains("gguf-convert")
            || trimmed.contains("local-seat")
            || trimmed.contains("import-trained")
    }
);
assert!(
        !shells_out,
        "train-next must not train, merge, convert, seat, or import"
    );
let uniq_script = std::fs::read_to_string(root.join("scripts/uniqueness-ladder.sh")).unwrap();
assert!(
        uniq_script.contains("make train-next"),
        "uniqueness-ladder must name train-next as the separate opt-in"
    );
let uniq_commands: Vec<&str> = uniq_script
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with('#') && !trimmed.starts_with("echo")
        }
)
        .collect();
assert!(
        !uniq_commands.join("\n").contains("train-next"),
        "uniqueness-ladder must not run train-next"
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
        .find(|line| line.contains("| `make train-next` |"))
        .expect("remaining row for train-next");
assert!(
        row.contains("Prints the NEXT.md llamafactory-cli train recipe after prepare"),
        "{row}"
    );
assert!(row.contains("Does not train"), "{row}");
assert!(row.contains("Not in smoke or Actions"), "{row}");
assert!(row.contains("Not a live train"), "{row}");
let uniq_row = remaining
        .lines()
        .find(|line| line.contains("`make uniqueness-ladder`"))
        .expect("remaining row for uniqueness-ladder");
assert!(
        uniq_row.contains("qlora-journey then seat-journey"),
        "{uniq_row}"
    );
assert!(uniq_row.contains("Does not run train-next"), "{uniq_row}");
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
let uniq = status
        .split("## Train/enrich uniqueness (matrix PR #140, prepare walk PR #142)")
        .nth(1)
        .expect("uniqueness section")
        .split("\n## ")
        .next()
        .unwrap();
assert!(uniq.contains("make train-next"), "{uniq}");
assert!(uniq.contains("CELL_TRAIN_LIVE=1"), "{uniq}");
assert!(uniq.contains("SKIP live train"), "{uniq}");
assert!(
        !uniq.contains("READY_FOR_LIVE_TEST: yes") && !uniq.contains("READY_FOR_LIVE_TEST`: yes"),
        "uniqueness section must keep READY_FOR_LIVE_TEST no"
    );
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
assert!(journey.contains("make train-next"));
assert!(journey.contains("CELL_TRAIN_LIVE=1"));
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
assert!(train.contains("make train-next"));
assert!(train.contains("CELL_TRAIN_LIVE=1"));
assert!(
        !journey.contains("READY_FOR_LIVE_TEST: yes")
            && !train.contains("READY_FOR_LIVE_TEST: yes"),
        "operator pages must keep READY_FOR_LIVE_TEST no"
    );
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains("make train-next"));
assert!(help.contains("CELL_TRAIN_LIVE=1"));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));
for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("train-next"),
            "{rel} must not run train-next"
        );
    }
}

#[test]
fn uniqueness_full_chains_prepare_train_seat_and_leaves_ladder_unchanged() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        makefile
            .lines()
            .any(|line| line.trim() == "uniqueness-full:"),
        "Makefile missing uniqueness-full"
    );
assert!(makefile.contains("scripts/uniqueness-full.sh"));
assert!(
        makefile.contains("Do not add to smoke, gate-90, or GitHub Actions"),
        "uniqueness-full must stay off smoke, gate-90, and Actions"
    );
let phony = makefile.lines().next().unwrap_or("");
assert!(
        phony.contains("uniqueness-full"),
        "uniqueness-full must be a phony target"
    );
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("uniqueness-full"),
        "gate-90 must not run uniqueness-full: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("uniqueness-full"),
        "smoke must not run uniqueness-full: {smoke}"
    );
let ladder_recipe = makefile
        .split("\nuniqueness-ladder:\n")
        .nth(1)
        .expect("uniqueness-ladder recipe")
        .lines()
        .next()
        .unwrap()
        .trim();
assert_eq!(
        ladder_recipe, "bash scripts/uniqueness-ladder.sh",
        "uniqueness-ladder recipe must stay the qlora-then-seat script"
    );
assert!(
        makefile.contains(
            "# Opt-in print-only Target C uniqueness chain: qlora-journey then seat-journey.\n# Does not run train-next."
        ),
        "uniqueness-ladder comment must keep train-next as the separate opt-in"
    );
let script_path = root.join("scripts/uniqueness-full.sh");
assert!(script_path.is_file(), "scripts/uniqueness-full.sh missing");
let script = std::fs::read_to_string(&script_path).unwrap();
for needle in [
        "Print-only",
        "READY_FOR_LIVE_TEST: no",
        "Does not train, merge, convert, seat, or promote.",
        "make qlora-journey",
        "make train-next",
        "make seat-journey",
        "make uniqueness-ladder stays qlora-journey then seat-journey and does not run train-next.",
        "Live train, live convert, and live seat still need a human GPU host and stay skipped.",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "PASS  uniqueness-full",
    ] {
        assert!(script.contains(needle), "uniqueness-full missing {needle}");
    }
assert!(
        !script.contains("READY_FOR_LIVE_TEST: yes"),
        "uniqueness-full must keep READY_FOR_LIVE_TEST no"
    );
let qlora_invoke = script
        .find("make -C \"$ROOT\" qlora-journey")
        .expect("chain must invoke qlora-journey");
let train_invoke = script
        .find("make -C \"$ROOT\" train-next")
        .expect("chain must invoke train-next");
let seat_invoke = script
        .find("make -C \"$ROOT\" seat-journey")
        .expect("chain must invoke seat-journey");
assert!(
        qlora_invoke < train_invoke && train_invoke < seat_invoke,
        "chain order must be qlora-journey, then train-next, then seat-journey"
    );
assert!(
        script[qlora_invoke..train_invoke].contains("exit 1"),
        "qlora-journey failure must exit before train-next"
    );
assert!(
        script[train_invoke..seat_invoke].contains("exit 1"),
        "train-next failure must exit before seat-journey"
    );
assert!(
        script[seat_invoke..].contains("exit 1"),
        "seat-journey failure must exit nonzero"
    );
let commands: Vec<&str> = script
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with('#') && !trimmed.starts_with("echo")
        }
)
        .collect();
let joined = commands.join("\n");
let qlora = joined
        .find("qlora-journey")
        .expect("chain must run qlora-journey");
let train = joined
        .find("train-next")
        .expect("chain must run train-next");
let seat = joined
        .find("seat-journey")
        .expect("chain must run seat-journey");
assert!(
        qlora < train && train < seat,
        "command order must be qlora-journey, then train-next, then seat-journey"
    );
assert!(
        !joined.contains("lf-beachhead-prepare"),
        "uniqueness-full must not run lf-beachhead-prepare"
    );
let shells_out = commands.iter().any(|line| {
        line.contains("llamafactory-cli")
            || line.contains("convert_hf_to_gguf.py")
            || line.contains("ollama ")
            || line.contains("import-trained")
            || line.contains("gguf-convert")
            || line.contains("local-seat")
            || line.contains("merge-adapt")
    }
);
assert!(
        !shells_out,
        "uniqueness-full must not train, merge, convert, seat, or import"
    );
let ladder = std::fs::read_to_string(root.join("scripts/uniqueness-ladder.sh")).unwrap();
assert!(
        ladder.contains(
            "The train step is a separate opt-in: make train-next. This chain does not run it."
        ),
        "uniqueness-ladder must keep train-next as the separate middle opt-in"
    );
assert!(
        ladder.contains("Chain: make qlora-journey, then make seat-journey."),
        "uniqueness-ladder must stay qlora-journey then seat-journey"
    );
assert!(
        !ladder.contains("uniqueness-full"),
        "uniqueness-ladder script must stay free of the full chain"
    );
let ladder_commands: Vec<&str> = ladder
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with('#') && !trimmed.starts_with("echo")
        }
)
        .collect();
let ladder_joined = ladder_commands.join("\n");
let ladder_qlora = ladder_joined
        .find("qlora-journey")
        .expect("uniqueness-ladder must run qlora-journey");
let ladder_seat = ladder_joined
        .find("seat-journey")
        .expect("uniqueness-ladder must run seat-journey");
assert!(
        ladder_qlora < ladder_seat,
        "uniqueness-ladder must run qlora-journey before seat-journey"
    );
assert!(
        !ladder_joined.contains("train-next"),
        "uniqueness-ladder must not run train-next"
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
        .find(|line| line.contains("| `make uniqueness-full` |"))
        .expect("remaining row for uniqueness-full");
let row_qlora = row.find("qlora-journey").expect("row names qlora-journey");
let row_train = row.find("train-next").expect("row names train-next");
let row_seat = row.find("seat-journey").expect("row names seat-journey");
assert!(
        row_qlora < row_train && row_train < row_seat,
        "remaining row order must be qlora-journey, then train-next, then seat-journey: {row}"
    );
assert!(row.contains("Does not train"), "{row}");
assert!(row.contains("Not in smoke or Actions"), "{row}");
assert!(row.contains("Not a live train"), "{row}");
let ladder_row = remaining
        .lines()
        .find(|line| line.contains("| `make uniqueness-ladder` |"))
        .expect("remaining row for uniqueness-ladder");
assert!(
        ladder_row.contains("qlora-journey then seat-journey"),
        "{ladder_row}"
    );
assert!(
        ladder_row.contains("Does not run train-next"),
        "{ladder_row}"
    );
assert!(
        !ladder_row.contains("uniqueness-full"),
        "uniqueness-ladder remaining row must stay the short chain: {ladder_row}"
    );
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
let uniq = status
        .split("## Train/enrich uniqueness (matrix PR #140, prepare walk PR #142)")
        .nth(1)
        .expect("uniqueness section")
        .split("\n## ")
        .next()
        .unwrap();
assert!(
        uniq.contains(
            "`make uniqueness-full` runs `make qlora-journey`, then `make train-next`, then `make seat-journey`."
        ),
        "{uniq}"
    );
assert!(
        uniq.contains(
            "`make uniqueness-ladder` stays `make qlora-journey`, then `make seat-journey`, and does not run `make train-next`."
        ),
        "{uniq}"
    );
assert!(
        !uniq.contains("READY_FOR_LIVE_TEST: yes") && !uniq.contains("READY_FOR_LIVE_TEST`: yes"),
        "uniqueness section must keep READY_FOR_LIVE_TEST no"
    );
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
assert!(journey.contains(
        "`make uniqueness-full` runs the same prints with the train recipe in the middle: `make qlora-journey`, then `make train-next`, then `make seat-journey`."
    ));
assert!(journey.contains("The train step is the separate opt-in `make train-next`."));
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
assert!(train.contains(
        "`make uniqueness-full` runs `make qlora-journey`, then `make train-next`, then `make seat-journey`."
    ));
assert!(train.contains(
        "`make uniqueness-ladder` runs the qlora and seat print journeys in that order. It does not run `make train-next`."
    ));
assert!(
        !journey.contains("READY_FOR_LIVE_TEST: yes")
            && !train.contains("READY_FOR_LIVE_TEST: yes"),
        "operator pages must keep READY_FOR_LIVE_TEST no"
    );
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains(
        "make uniqueness-full runs make qlora-journey, then make train-next,\nthen make seat-journey."
    ));
assert!(help.contains("make uniqueness-ladder does not run it."));
assert!(help.contains("stays qlora-journey then seat-journey."));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));
for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("uniqueness-full"),
            "{rel} must not run uniqueness-full"
        );
    }
}

#[test]
fn uniqueness_full_lora_chains_prepare_train_seat_and_leaves_qlora_chain() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
for target in ["uniqueness-full-lora:", "train-next-lora:", "seat-journey-lora:"] {
        assert!(
            makefile.lines().any(|line| line.trim() == target),
            "Makefile missing {target}"
        );
    }
assert!(makefile.contains("scripts/uniqueness-full-lora.sh"));
assert!(makefile.contains(
        "TRAIN_CARD=llamafactory-qlora bash scripts/train-next.sh"
    ));
assert!(makefile.contains(
        "TRAIN_CARD=llamafactory-lora bash scripts/train-next.sh"
    ));
assert!(makefile.contains(
        "SEAT_CARD=llamafactory-qlora bash scripts/seat-journey.sh"
    ));
assert!(makefile.contains(
        "SEAT_CARD=llamafactory-lora bash scripts/seat-journey.sh"
    ));
let phony = makefile.lines().next().unwrap_or("");
for name in ["uniqueness-full-lora", "train-next-lora", "seat-journey-lora"] {
        assert!(phony.contains(name), "{name} must be a phony target");
    }
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
for name in ["uniqueness-full-lora", "train-next-lora", "seat-journey-lora"] {
        assert!(!gate90.contains(name), "gate-90 must not run {name}: {gate90}");
        assert!(!smoke.contains(name), "smoke must not run {name}: {smoke}");
    }
assert_eq!(
        makefile
            .split("\nuniqueness-full:\n")
            .nth(1)
            .expect("uniqueness-full recipe")
            .lines()
            .next()
            .unwrap()
            .trim(),
        "bash scripts/uniqueness-full.sh",
        "uniqueness-full recipe must stay the QLoRA chain"
    );
let script_path = root.join("scripts/uniqueness-full-lora.sh");
assert!(script_path.is_file(), "scripts/uniqueness-full-lora.sh missing");
let script = std::fs::read_to_string(&script_path).unwrap();
for needle in [
        "Print-only",
        "READY_FOR_LIVE_TEST: no",
        "Does not train, merge, convert, seat, or promote.",
        "make lora-journey",
        "make train-next-lora",
        "make seat-journey-lora",
        "make uniqueness-full stays qlora-journey, then train-next, then seat-journey.",
        "make uniqueness-ladder stays qlora-journey then seat-journey and does not run train-next.",
        "Live train, live convert, and live seat still need a human GPU host and stay skipped.",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "PASS  uniqueness-full-lora",
    ] {
        assert!(script.contains(needle), "uniqueness-full-lora missing {needle}");
    }
assert!(
        !script.contains("READY_FOR_LIVE_TEST: yes"),
        "uniqueness-full-lora must keep READY_FOR_LIVE_TEST no"
    );
let invokes: Vec<&str> = script
        .lines()
        .filter(|line| line.contains("make -C"))
        .collect();
assert_eq!(
        invokes,
        vec![
            "if ! make -C \"$ROOT\" lora-journey; then",
            "if ! make -C \"$ROOT\" train-next-lora; then",
            "if ! make -C \"$ROOT\" seat-journey-lora; then",
        ],
        "chain must be lora-journey, then train-next-lora, then seat-journey-lora"
    );
let lora_invoke = script
        .find("make -C \"$ROOT\" lora-journey")
        .expect("chain must invoke lora-journey");
let train_invoke = script
        .find("make -C \"$ROOT\" train-next-lora")
        .expect("chain must invoke train-next-lora");
let seat_invoke = script
        .find("make -C \"$ROOT\" seat-journey-lora")
        .expect("chain must invoke seat-journey-lora");
assert!(lora_invoke < train_invoke && train_invoke < seat_invoke);
assert!(script[lora_invoke..train_invoke].contains("exit 1"));
assert!(script[train_invoke..seat_invoke].contains("exit 1"));
assert!(script[seat_invoke..].contains("exit 1"));
assert!(
        !script.contains("make -C \"$ROOT\" qlora-journey")
            && !script.contains("make -C \"$ROOT\" train-next;")
            && !script.contains("make -C \"$ROOT\" seat-journey;"),
        "uniqueness-full-lora must not run the QLoRA chain"
    );
let commands: Vec<&str> = script
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with('#') && !trimmed.starts_with("echo")
        }
)
        .collect();
assert!(
        !commands.join("\n").contains("lf-beachhead-prepare"),
        "uniqueness-full-lora must not run lf-beachhead-prepare"
    );
let shells_out = script.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.starts_with("echo") {
            return false;
        }
        trimmed.contains("llamafactory-cli")
            || trimmed.contains("convert_hf_to_gguf.py")
            || trimmed.contains("ollama ")
            || trimmed.contains("import-trained")
            || trimmed.contains("gguf-convert")
            || trimmed.contains("local-seat")
            || trimmed.contains("merge-adapt")
    }
);
assert!(
        !shells_out,
        "uniqueness-full-lora must not train, merge, convert, seat, or import"
    );
let train = std::fs::read_to_string(root.join("scripts/train-next.sh")).unwrap();
for needle in [
        "TRAIN_CARD",
        "llamafactory-lora",
        "llamafactory-qlora",
        "^lora_rank: 8$",
        "^packing: false$",
        "does not require bitsandbytes",
        "must not install bitsandbytes",
        "quantization_bit: 4",
        "pip install 'bitsandbytes>=0.49'",
        "make uniqueness-ladder stays qlora-journey then seat-journey.",
        "CELL_TRAIN_LIVE=1 is set. This journey stays print-only.",
        "SKIP  bitsandbytes (not importable; informational)",
        "PASS  train-next-lora",
    ] {
        assert!(train.contains(needle), "train-next missing LoRA twin needle {needle}");
    }
assert!(train.contains("case \"$TRAIN_CARD\" in"));
assert!(train.contains("llamafactory-qlora|llamafactory-lora)"));
let seat = std::fs::read_to_string(root.join("scripts/seat-journey.sh")).unwrap();
for needle in [
        "SEAT_CARD",
        "llamafactory-lora",
        "llamafactory-qlora",
        "^lora_rank: 8$",
        "quantization_bit: 4",
        "refuse:adapter",
        "refuse:tokenizer",
        "refuse:seat",
        "5090-shaped",
        "PASS  seat-journey (Target A LoRA seat ladder printed;",
        "PASS  seat-journey (Target C seat ladder printed;",
    ] {
        assert!(seat.contains(needle), "seat-journey missing LoRA twin needle {needle}");
    }
let bad = seat
        .find("-- 5090-shaped export tokenizer is refuse:tokenizer --")
        .expect("refuse:tokenizer step");
let replace = seat
        .find("-- replace the broken tokenizer with the good merged stub --")
        .expect("good stub replace");
assert!(bad < replace, "refuse:tokenizer must stay before the good stub");
let qlora_chain = std::fs::read_to_string(root.join("scripts/uniqueness-full.sh")).unwrap();
assert!(
        !qlora_chain.contains("uniqueness-full-lora")
            && !qlora_chain.contains("train-next-lora")
            && !qlora_chain.contains("seat-journey-lora"),
        "uniqueness-full must stay the QLoRA chain"
    );
let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
let remaining = gate
        .split("## Remaining Day-90+ (honest)")
        .nth(1)
        .expect("remaining section");
let row = remaining
        .lines()
        .find(|line| line.contains("| `make uniqueness-full-lora` |"))
        .expect("remaining row for uniqueness-full-lora");
let row_lora = row.find("lora-journey").expect("row names lora-journey");
let row_train = row.find("train-next-lora").expect("row names train-next-lora");
let row_seat = row.find("seat-journey-lora").expect("row names seat-journey-lora");
assert!(row_lora < row_train && row_train < row_seat, "{row}");
assert!(row.contains("Does not train"), "{row}");
assert!(row.contains("Not in smoke or Actions"), "{row}");
assert!(row.contains("Not a live train"), "{row}");
let qlora_row = remaining
        .lines()
        .find(|line| line.contains("| `make uniqueness-full` |"))
        .expect("remaining row for uniqueness-full");
assert!(
        qlora_row.contains("qlora-journey, then train-next, then seat-journey"),
        "{qlora_row}"
    );
assert!(
        !qlora_row.contains("uniqueness-full-lora"),
        "QLoRA remaining row must stay the QLoRA chain: {qlora_row}"
    );
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
assert!(journey.contains(
        "`make uniqueness-full-lora` runs the Target A print chain in that same order on the unquantized card: `make lora-journey`, then `make train-next-lora`, then `make seat-journey-lora`."
    ));
assert!(journey.contains("`make uniqueness-full` runs the same prints with the train recipe in the middle: `make qlora-journey`, then `make train-next`, then `make seat-journey`."));
let train_doc = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
assert!(train_doc.contains(
        "`make uniqueness-full-lora` runs `make lora-journey`, then `make train-next-lora`, then `make seat-journey-lora`."
    ));
assert!(train_doc.contains(
        "`make uniqueness-full` runs `make qlora-journey`, then `make train-next`, then `make seat-journey`."
    ));
assert!(
        !journey.contains("READY_FOR_LIVE_TEST: yes")
            && !train_doc.contains("READY_FOR_LIVE_TEST: yes")
    );
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains(
        "make uniqueness-full-lora runs make lora-journey, then make\ntrain-next-lora, then make seat-journey-lora."
    ));
assert!(help.contains(
        "make uniqueness-full runs make qlora-journey, then make train-next,\nthen make seat-journey."
    ));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));
for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("uniqueness-full-lora")
                && !body.contains("train-next-lora")
                && !body.contains("seat-journey-lora"),
            "{rel} must not run the LoRA uniqueness chain"
        );
    }
}

fn extract_shell_fn(script: &str, name: &str) -> String {
    let marker = format!("{name}() {{");
    let start = script
        .find(&marker)
        .unwrap_or_else(|| panic!("missing {name}()"));
    let mut depth = 0i32;
    for (i, ch) in script[start..].char_indices() {
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return script[start..start + i + 1].to_string();
            }
        }
    }
    panic!("unclosed {name}()");
}
fn estate_resolver_body(script: &str) -> String {
    format!(
        "{}\n{}",
        extract_shell_fn(script, "resolve_estate"),
        extract_shell_fn(script, "estate")
    )
}
fn first_estate_invocation(script: &str) -> usize {
    script
        .lines()
        .position(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#')
                || trimmed.starts_with("echo")
                || trimmed.starts_with("estate()")
            {
                return false;
            }
            trimmed.starts_with("estate ") || trimmed.contains(" estate ")
        })
        .expect("script never invokes estate")
}
fn write_exec(path: &std::path::Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).unwrap();
    }
}
fn run_extracted_resolver(
    funcs: &str,
    root: &std::path::Path,
    estate_bin: Option<&std::path::Path>,
    path: &str,
) -> std::process::Output {
    let dir = std::env::temp_dir().join(format!(
        "cell-one-estate-resolve-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let harness = dir.join("harness.sh");
    let body = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\nROOT=\"$1\"\nBIN=\"${{ESTATE_BIN:-}}\"\n{funcs}\nresolve_estate\nprintf '%s\\n' \"${{ESTATE_CMD[@]}}\"\n"
    );
    std::fs::write(&harness, body).unwrap();
    let mut cmd = std::process::Command::new("bash");
    cmd.arg(&harness)
        .arg(root)
        .env("PATH", path)
        .env_remove("ESTATE_BIN")
        .env_remove("ESTATE_RESOLVED");
    if let Some(bin) = estate_bin {
        cmd.env("ESTATE_BIN", bin);
    }
    let output = cmd.output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    output
}
#[test]
fn journey_scripts_resolve_local_estate_before_cargo() {
let root = repo_root();
let rels = [
        "scripts/qlora-journey.sh",
        "scripts/train-next.sh",
        "scripts/seat-journey.sh",
        "scripts/lora-journey.sh",
        "scripts/lf-beachhead-prepare.sh",
        "scripts/train-prepare.sh",
        "scripts/axolotl-qlora-journey.sh",
        "scripts/unsloth-qlora-journey.sh",
        "scripts/unsloth-lora-journey.sh",
        "scripts/axolotl-lora-journey.sh",
    ];
let mut bodies = Vec::new();
for rel in rels {
        let script = std::fs::read_to_string(root.join(rel)).unwrap();
        let syntax = std::process::Command::new("bash")
            .arg("-n")
            .arg(root.join(rel))
            .output()
            .unwrap();
        assert!(
            syntax.status.success(),
            "{rel} failed bash -n: {}",
            String::from_utf8_lossy(&syntax.stderr)
        );
        assert!(
            script.contains(
                "Resolves estate fail-closed: executable ESTATE_BIN, then target/release/estate,"
            ),
            "{rel} must document the release fallback"
        );
        assert!(
            script.contains(
                "then target/debug/estate, then cargo on PATH. Does not invent a binary."
            ),
            "{rel} must document the debug fallback"
        );
        let body = estate_resolver_body(&script);
        let bin_exec = body
            .find("[[ -n \"$BIN\" && -x \"$BIN\" ]]")
            .unwrap_or_else(|| panic!("{rel} must prefer an executable ESTATE_BIN"));
        let release = body
            .find("[[ -x \"$ROOT/target/release/estate\" ]]")
            .unwrap_or_else(|| panic!("{rel} must fall back to target/release/estate"));
        let debug = body
            .find("[[ -x \"$ROOT/target/debug/estate\" ]]")
            .unwrap_or_else(|| panic!("{rel} must fall back to target/debug/estate"));
        let cargo_path = body
            .find("command -v cargo")
            .unwrap_or_else(|| panic!("{rel} must keep cargo on PATH as the last resort"));
        let cargo_run = body
            .find("cargo run -q -p estate-control --")
            .unwrap_or_else(|| panic!("{rel} must keep cargo run -q -p estate-control --"));
        let unresolved = body
            .find("estate binary unresolved")
            .unwrap_or_else(|| panic!("{rel} must fail closed when nothing resolves"));
        assert!(
            bin_exec < release && release < debug && debug < cargo_path && cargo_path < cargo_run,
            "{rel} resolver order drifted"
        );
        assert!(
            cargo_run < unresolved,
            "{rel} fail-closed message must follow the cargo branch"
        );
        let fail = &body[unresolved..];
        assert!(fail.contains("ESTATE_BIN"), "{rel} {fail}");
        assert!(fail.contains("target/release/estate"), "{rel} {fail}");
        assert!(fail.contains("target/debug/estate"), "{rel} {fail}");
        let non_exec = body
            .find("ESTATE_BIN is set but not executable")
            .unwrap_or_else(|| panic!("{rel} must refuse a set ESTATE_BIN that is not executable"));
        assert!(
            bin_exec < non_exec && non_exec < release,
            "{rel} a bad ESTATE_BIN must not fall through to target/release/estate"
        );
        assert!(
            body[non_exec..release].contains("exit 1"),
            "{rel} a bad ESTATE_BIN must exit nonzero"
        );
        assert!(
            body[unresolved..].contains("exit 1"),
            "{rel} an unresolved estate must exit nonzero"
        );
        let call = script
            .lines()
            .position(|line| line == "resolve_estate")
            .unwrap_or_else(|| panic!("{rel} must call resolve_estate before using estate"));
        let invoke = first_estate_invocation(&script);
        assert!(
            call < invoke,
            "{rel} resolve_estate at {call} must run before the estate invocation at {invoke}"
        );
        assert!(
            !script.contains("READY_FOR_LIVE_TEST: yes\n"),
            "{rel} must not flip READY_FOR_LIVE_TEST"
        );
        bodies.push(body);
    }
assert!(
        bodies.iter().all(|body| body == &bodies[0]),
        "journey estate resolvers must stay the same fail-closed order"
    );
let beachhead = std::fs::read_to_string(root.join("scripts/lf-beachhead-prepare.sh")).unwrap();
assert!(
        !beachhead.contains("cargo build"),
        "lf-beachhead-prepare must not require cargo build before a local estate binary"
    );
for rel in [
        "scripts/uniqueness-full.sh",
        "scripts/uniqueness-ladder.sh",
        "scripts/uniqueness-full-lora.sh",
        "scripts/uniqueness-axolotl.sh",
        "scripts/uniqueness-unsloth.sh",
        "scripts/uniqueness-axolotl-lora.sh",
    ] {
        let script = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !script.contains("resolve_estate") && !script.contains("target/release/estate"),
            "{rel} only calls make and does not resolve estate itself"
        );
        assert!(
            script.contains("READY_FOR_LIVE_TEST: no"),
            "{rel} must keep READY_FOR_LIVE_TEST no"
        );
        assert!(
            !script.contains("READY_FOR_LIVE_TEST: yes"),
            "{rel} must not flip READY_FOR_LIVE_TEST"
        );
    }
for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("resolve_estate"),
            "{rel} must not grow the print-journey resolver"
        );
    }

let funcs = &bodies[0];
let work = std::env::temp_dir().join(format!(
        "cell-one-estate-resolve-root-{}",
        std::process::id()
    ));
let _ = std::fs::remove_dir_all(&work);
let release = work.join("target/release/estate");
let debug = work.join("target/debug/estate");
let chosen = work.join("chosen-estate");
let not_exec = work.join("not-executable");
let fake_cargo = work.join("fake-bin/cargo");
write_exec(&release, "#!/bin/sh\nexit 0\n");
write_exec(&debug, "#!/bin/sh\nexit 0\n");
write_exec(&chosen, "#!/bin/sh\nexit 0\n");
std::fs::write(&not_exec, "not executable\n").unwrap();
write_exec(&fake_cargo, "#!/bin/sh\necho should-not-run >&2\nexit 99\n");
let bare_path = "/usr/bin:/bin";
let cargo_path = format!("{}:{bare_path}", fake_cargo.parent().unwrap().display());
let preferred = run_extracted_resolver(funcs, &work, Some(&chosen), bare_path);
assert!(preferred.status.success(), "{preferred:?}");
assert_eq!(
        String::from_utf8_lossy(&preferred.stdout).trim(),
        chosen.display().to_string(),
        "ESTATE_BIN must win over target/release/estate"
    );
let release_first = run_extracted_resolver(funcs, &work, None, bare_path);
assert!(release_first.status.success(), "{release_first:?}");
assert_eq!(
        String::from_utf8_lossy(&release_first.stdout).trim(),
        release.display().to_string(),
        "target/release/estate must win over target/debug/estate"
    );
std::fs::remove_file(&release).unwrap();
let debug_next = run_extracted_resolver(funcs, &work, None, bare_path);
assert!(debug_next.status.success(), "{debug_next:?}");
assert_eq!(
        String::from_utf8_lossy(&debug_next.stdout).trim(),
        debug.display().to_string(),
        "target/debug/estate must be used when release is missing and cargo is off PATH"
    );
std::fs::remove_file(&debug).unwrap();
let cargo_last = run_extracted_resolver(funcs, &work, None, &cargo_path);
assert!(cargo_last.status.success(), "{cargo_last:?}");
assert_eq!(
        String::from_utf8_lossy(&cargo_last.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["cargo", "run", "-q", "-p", "estate-control", "--"]
    );
let missing = run_extracted_resolver(funcs, &work, None, bare_path);
assert!(
        !missing.status.success(),
        "missing estate must exit nonzero: {missing:?}"
    );
let missing_err = String::from_utf8_lossy(&missing.stderr);
assert!(missing_err.contains("ESTATE_BIN"), "{missing_err}");
assert!(
        missing_err.contains("target/release/estate"),
        "{missing_err}"
    );
assert!(missing_err.contains("target/debug/estate"), "{missing_err}");
assert!(missing.stdout.is_empty(), "{missing:?}");
write_exec(&release, "#!/bin/sh\nexit 0\n");
let bad_bin = run_extracted_resolver(funcs, &work, Some(&not_exec), bare_path);
assert!(!bad_bin.status.success(), "{bad_bin:?}");
let bad_err = String::from_utf8_lossy(&bad_bin.stderr);
assert!(
        bad_err.contains("ESTATE_BIN is set but not executable"),
        "{bad_err}"
    );
assert!(bad_err.contains("target/release/estate"), "{bad_err}");
assert!(bad_err.contains("target/debug/estate"), "{bad_err}");
assert!(
        !String::from_utf8_lossy(&bad_bin.stdout).contains(&release.display().to_string()),
        "a set ESTATE_BIN must not fall through to target/release/estate"
    );
let _ = std::fs::remove_dir_all(&work);
}

#[test]
fn tokenizer_restore_names_dereference_and_keeps_tip_framing() {
let root = repo_root();
let needles = [
        "HF hub snapshots are often symlinks",
        "cp -aL",
        "cp --dereference",
        "real files, not symlinks",
        "does not follow a symlinked",
    ];
let pages = [
        "docs/TRAIN-ENRICH.md",
        "docs/local-seat.md",
        "docs/operator-enrich-journeys.md",
        "docs/CELL-ONE-STATUS.md",
        "crates/estate-control/src/help.rs",
        "crates/model-estate/src/gguf_convert.rs",
        "scripts/seat-journey.sh",
    ];
for rel in pages {
        let text = std::fs::read_to_string(root.join(rel)).unwrap();
        for needle in needles {
            assert!(text.contains(needle), "{rel} missing {needle}");
        }
        assert!(
            text.contains("plain cp -a") || text.contains("plain `cp -a`"),
            "{rel} must name a plain cp -a"
        );
    }
for rel in [
        "docs/TRAIN-ENRICH.md",
        "docs/local-seat.md",
        "docs/operator-enrich-journeys.md",
        "docs/CELL-ONE-STATUS.md",
        "crates/estate-control/src/help.rs",
        "scripts/seat-journey.sh",
    ] {
        let text = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !text.contains("READY_FOR_LIVE_TEST: yes")
                && !text.contains("READY_FOR_LIVE_TEST`: yes"),
            "{rel} must keep READY_FOR_LIVE_TEST no"
        );
    }
let source =
        std::fs::read_to_string(root.join("crates/model-estate/src/gguf_convert.rs")).unwrap();
assert!(
        source.contains("refuse:tokenizer: {} is a symlink. {}"),
        "symlink refuse must stay fail-closed"
    );
assert!(
        !source.contains("is a symlink. enrich does not follow a symlinked tokenizer_config.json."),
        "symlink refuse must not repeat the restore sentence"
    );
assert!(
        source.contains("enrich does not follow a symlinked tokenizer_config.json"),
        "the restore sentence must keep the fail-closed follow clause"
    );
assert!(
        source
            .contains("returns refuse:tokenizer for that export before the restore, for that list"),
        "guidance must name the refuse before the restore"
    );
assert!(
        !source.contains("{restore} estate enrich gguf-convert returns refuse:tokenizer"),
        "guidance must not append the refuse after the restore sentence"
    );
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
assert!(
        train.contains("names that tokenizer restore when `extra_special_tokens` is a JSON list"),
        "the merge-adapt when-clause must scope the restore"
    );
assert!(
        !train.contains("re-run `estate enrich gguf-convert` when `extra_special_tokens`"),
        "the re-run must not be scoped to the bad tokenizer shape"
    );
assert!(
        train.contains(
            "`gguf-convert` returns `refuse:tokenizer` for that export before the restore."
        ),
        "the refuse must name the bad export before the restore guidance"
    );
assert!(
        !train.contains(
            "Then re-run `estate enrich gguf-convert`. `gguf-convert` returns `refuse:tokenizer`"
        ),
        "the re-run must not be followed by the refuse claim"
    );
let journeys = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
let section_10 = journeys
        .split("## 10. Target C seat ladder")
        .nth(1)
        .expect("section 10");
assert!(
        section_10.contains("Then re-run `estate enrich gguf-convert`."),
        "{section_10}"
    );
assert!(
        section_10.contains("The refuse does not print `python3 convert_hf_to_gguf.py`."),
        "{section_10}"
    );
assert!(
        !section_10.contains("Then re-running"),
        "section 10 must use an imperative re-run"
    );
assert!(
        source.contains("meta.file_type().is_symlink()"),
        "gguf-convert must still detect a symlinked tokenizer_config.json"
    );
assert!(
        !source.contains("std::fs::copy"),
        "gguf-convert must not copy tokenizer files"
    );
assert!(source.contains("does not copy those files"), "{source}");
assert!(source.contains("does not download weights"), "{source}");
let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
assert!(
        !gate.contains("READY_FOR_LIVE_TEST: yes") && !gate.contains("READY_FOR_LIVE_TEST`: yes"),
        "GATE-90 must not flip READY_FOR_LIVE_TEST"
    );
assert!(
        !gate.contains("cp -aL"),
        "this pack must not churn the GATE-90 tip page"
    );
for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("seat-journey"),
            "{rel} must not run seat-journey"
        );
        assert!(!body.contains("cp -aL"), "{rel} must not grow a copy step");
    }
}

#[test]
fn local_seat_print_only_names_the_unwritten_modelfile() {
let root = repo_root();
let needles = [
        "local-seat is print-only",
        "does not write",
        "Modelfile",
        "from the printed contents before",
        "ollama create",
    ];
let pages = [
        "docs/local-seat.md",
        "docs/TRAIN-ENRICH.md",
        "docs/operator-enrich-journeys.md",
        "crates/estate-control/src/help.rs",
        "scripts/seat-journey.sh",
        "crates/model-estate/src/local_seat.rs",
    ];
for rel in pages {
        let text = std::fs::read_to_string(root.join(rel)).unwrap();
        let flat = text.replace('`', "");
        for needle in needles {
            assert!(flat.contains(needle), "{rel} missing {needle}");
        }
        if !rel.ends_with(".rs") {
            assert!(
                !text.contains("READY_FOR_LIVE_TEST: yes")
                    && !text.contains("READY_FOR_LIVE_TEST`: yes"),
                "{rel} must keep READY_FOR_LIVE_TEST no"
            );
        }
    }
for rel in [
        "docs/local-seat.md",
        "docs/TRAIN-ENRICH.md",
        "docs/operator-enrich-journeys.md",
        "crates/estate-control/src/help.rs",
        "scripts/seat-journey.sh",
    ] {
        let text = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            text.contains("$PREPARED/Modelfile"),
            "{rel} must name $PREPARED/Modelfile"
        );
    }
let on_disk = "report uses the on-disk Modelfile when FROM already names the artifact";
let do_not_rewrite = "Do not write $PREPARED/Modelfile again";
for rel in [
        "docs/local-seat.md",
        "docs/TRAIN-ENRICH.md",
        "docs/operator-enrich-journeys.md",
        "crates/estate-control/src/help.rs",
    ] {
        let text = std::fs::read_to_string(root.join(rel)).unwrap();
        let flat = text.replace('`', "");
        assert!(
            flat.contains(on_disk),
            "{rel} must say the report uses the on-disk Modelfile"
        );
        assert!(
            flat.contains(do_not_rewrite),
            "{rel} must not tell a merged seat to rewrite $PREPARED/Modelfile"
        );
    }
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
let general = help
        .split("estate enrich local-seat validates")
        .nth(1)
        .expect("general local-seat blurb")
        .split("Pass --adapter")
        .next()
        .unwrap();
assert!(
        general.contains("On that GGUF print-only path,"),
        "the general blurb must scope the print-only sentence to a GGUF"
    );
assert!(general.contains(
        "local-seat is print-only. It prints the Modelfile and does not write\n$PREPARED/Modelfile. Write that file from the printed contents before\nollama create."
    ));
let merged_at = general
        .find("--weights .cell/enrich/overnight-traces/llamafactory-qlora/export\n")
        .expect("merged --weights export example");
assert!(
        !general[merged_at..].contains("Write that file from the printed contents"),
        "the merged --weights export example must not carry the print-only write line"
    );
assert!(
        general[merged_at..].replace('\n', " ").contains(on_disk),
        "the merged example must say the report uses the on-disk Modelfile"
    );
let write_at = general
        .find("Write that file from the printed contents")
        .unwrap();
assert!(
        general[..write_at].contains("On that GGUF print-only path"),
        "the write line in the general blurb stays on the GGUF print-only path"
    );
let seat_page = std::fs::read_to_string(root.join("docs/local-seat.md")).unwrap();
let step4 = seat_page
        .split("4. Seat with Ollama")
        .nth(1)
        .expect("local-seat chain step 4")
        .split("5. After")
        .next()
        .unwrap()
        .replace('`', "");
assert!(step4.contains(
        "local-seat is print-only. It prints the Modelfile and does not write $PREPARED/Modelfile. Write that file from the printed contents before ollama create."
    ));
assert!(step4.contains(on_disk), "{step4}");
assert!(step4.contains(do_not_rewrite), "{step4}");
let step4_write = step4
        .find("Write that file from the printed contents")
        .unwrap();
assert!(
        step4[..step4_write].contains("GGUF"),
        "chain step 4 must name the GGUF before the write line"
    );
let source =
        std::fs::read_to_string(root.join("crates/model-estate/src/local_seat.rs")).unwrap();
let production = source
        .split("#[cfg(test)]")
        .next()
        .expect("local_seat production prefix");
assert!(
        production
            .contains("local-seat is print-only. It prints the Modelfile and does not write {}."),
        "the printed line must name the unwritten path"
    );
assert!(
        production.contains("Write that file from the printed contents before ollama create."),
        "{production}"
    );
assert!(
        !production.contains("std::fs::write"),
        "local-seat must not write the Modelfile"
    );
assert!(
        !production.contains("std::process::Command"),
        "local-seat must not shell out"
    );
assert!(
        !production.contains("READY_FOR_LIVE_TEST: yes"),
        "local-seat must keep READY_FOR_LIVE_TEST no"
    );
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
let seating = train
        .split("## Local seat after LLaMA-Factory export")
        .nth(1)
        .expect("seating section")
        .split("## Status and doctor")
        .next()
        .unwrap();
let step2_at = seating
        .find("2. `estate enrich gguf-convert`")
        .expect("seating step 2");
let step3_at = seating.find("3. `ollama create`").expect("seating step 3");
let merged_export = seating
        .find("--weights .cell/enrich/<pack-id>/llamafactory-qlora/export\n")
        .expect("merged export local-seat example");
let after_merged = &seating[merged_export..];
let gguf_write = after_merged
        .find("Write that file from the printed contents")
        .expect("GGUF print-only sentence stays after the merged example");
let between = after_merged[..gguf_write].replace('`', "");
assert!(
        between.contains("report uses the on-disk Modelfile when FROM already names the artifact"),
        "{between}"
    );
assert!(
        between.contains("Do not write $PREPARED/Modelfile again"),
        "{between}"
    );
assert!(
        !between.contains("Write that file from the printed contents"),
        "the merged export example must not carry the print-only write line"
    );
let step2 = &seating[step2_at..step3_at];
let refuse_at = step2
        .find("`gguf-convert` returns `refuse:tokenizer`")
        .expect("step 2 refuse");
let rerun_at = step2
        .find("Then re-run `estate enrich gguf-convert`")
        .expect("step 2 re-run");
assert!(
        refuse_at < rerun_at,
        "TRAIN-ENRICH seating step 2 must name the refuse before the re-run"
    );
assert!(
        !step2[rerun_at..].contains("returns `refuse:tokenizer`"),
        "the re-run must not be followed by the refuse claim"
    );
let journeys = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
let section_8 = journeys
        .split("## 8. Target C")
        .nth(1)
        .expect("section 8")
        .split("## 9.")
        .next()
        .unwrap();
let prove_at = section_8
        .find("A live 5090 prove on 2026-09-23 hit two tokenizer problems")
        .expect("section 8 tokenizer paragraph");
let next_heading = section_8[prove_at..]
        .find("### 4.")
        .expect("section 8 step 4");
let section_7 = journeys
        .split("## 7. Seat the merged export")
        .nth(1)
        .expect("section 7")
        .split("## 8.")
        .next()
        .unwrap()
        .replace('`', "");
assert!(section_7.contains(
        "On that GGUF print-only path, local-seat is print-only. It prints the Modelfile and does not write $PREPARED/Modelfile. Write that file from the printed contents before ollama create."
    ));
assert!(section_7.contains(
        "report uses the on-disk Modelfile when FROM already names the artifact. Do not write $PREPARED/Modelfile again."
    ));
let section_8_flat = section_8.replace('`', "");
let merged_clause = section_8_flat
        .find("To seat the merged directory itself")
        .expect("section 8 merged clause");
assert!(section_8_flat[merged_clause..].contains(
        "report uses the on-disk Modelfile when FROM already names the artifact. Do not write $PREPARED/Modelfile again."
    ));
let prove = &section_8[prove_at..prove_at + next_heading];
let prove_refuse = prove
        .find("`gguf-convert` returns `refuse:tokenizer`")
        .expect("section 8 refuse");
let prove_rerun = prove
        .find("Then re-run `estate enrich gguf-convert`")
        .expect("section 8 re-run");
assert!(
        prove_refuse < prove_rerun,
        "operator journey section 8 must name the refuse before the re-run"
    );
assert!(
        !prove[prove_rerun..].contains("returns `refuse:tokenizer`"),
        "the re-run must not be followed by the refuse claim"
    );
for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("seat-journey"),
            "{rel} must not run seat-journey"
        );
    }
}

#[test]
fn target_c_live_uniqueness_prove_stays_recorded() {
let root = repo_root();
let probes = std::fs::read_to_string(root.join("docs/LIVE-PROBES.md")).unwrap();
let section = probes
        .split("## Target C live uniqueness (5090-class)")
        .nth(1)
        .expect("LIVE-PROBES missing the Target C uniqueness section")
        .split("\n## ")
        .next()
        .unwrap();
for needle in [
        "**PASS.**",
        "consumer-nvidia",
        "rented-nvidia",
        "Do not put `5090` in a binding id",
        "not `estate probes --live`",
        "not the\nMac `Pong` row",
        "not native MLX",
        "/tmp/cell-one-target-c-live-20260923",
        "did not use `examples/estate.yaml` as the",
        "43770130 3391",
        "llamafactory-qlora",
        "Seat tag `llama3`",
        "Qwen/Qwen2.5-0.5B-Instruct",
        "`max_steps` 10",
        "exited 0",
        "llamafactory-qlora` tree",
        "`refuse:tokenizer`",
        "`extra_special_tokens` was a list",
        "missing `vocab.json` and `merges.txt`",
        "`cp -aL`",
        "`cp --dereference`",
        "plain `cp -a` left symlinks",
        "does not follow a symlinked `tokenizer_config.json`",
        "BF16 GGUF (~949M)",
        "ran outside the factory",
        "did not write `$PREPARED/Modelfile`",
        "from the printed contents",
        "`cell-target-c-qlora-prove`",
        "`trained_shape` `gguf`",
        "`auto_apply=false`",
        "`ollama rm`",
        "The factory did not train, convert, shell out to ollama, or promote.",
        "`READY_FOR_LIVE_TEST`: **no**",
        "not in `make smoke`",
        "`make gate-90`",
        "GitHub Actions",
        "specialist/pong-style check",
        "not the recorded `estate probes --live` row",
        "not\nthe Mac `Pong` row",
        "does not record a completion JSON blob",
    ] {
        assert!(
            section.contains(needle),
            "Target C uniqueness section missing {needle}"
        );
    }
assert!(
        !section.contains("\"completion\""),
        "the uniqueness section must not invent a completion JSON blob"
    );
assert!(
        !section.contains("READY_FOR_LIVE_TEST: yes")
            && !section.contains("READY_FOR_LIVE_TEST`: yes"),
        "the uniqueness section must keep READY_FOR_LIVE_TEST no"
    );
assert!(
        !section.to_ascii_lowercase().contains("kimi"),
        "the uniqueness section must not add Kimi"
    );
assert!(
        probes.contains(
            "| Target C live uniqueness **PASS** | The factory training, converting, shelling out to ollama, or promoting."
        ),
        "What green is not must keep the uniqueness PASS distinct from a factory run"
    );
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
let uniq = status
        .split("## Train/enrich uniqueness (matrix PR #140, prepare walk PR #142)")
        .nth(1)
        .expect("uniqueness section")
        .split("\n## ")
        .next()
        .unwrap();
assert!(
        uniq.contains("[Target C live uniqueness (5090-class)](LIVE-PROBES.md)"),
        "uniqueness section must point at the recorded prove"
    );
assert!(
        uniq.contains("The factory did not train, convert, shell out to ollama, or promote."),
        "{uniq}"
    );
assert!(
        !uniq.contains("READY_FOR_LIVE_TEST: yes") && !uniq.contains("READY_FOR_LIVE_TEST`: yes"),
        "uniqueness section must keep READY_FOR_LIVE_TEST no"
    );
for rel in ["docs/TRAIN-ENRICH.md", "docs/local-seat.md"] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            body.contains("[Target C live uniqueness (5090-class)](LIVE-PROBES.md)"),
            "{rel} must point at the recorded prove"
        );
        assert!(
            body.contains("The factory did not train, convert, shell out to ollama, or promote."),
            "{rel} must keep the factory-did-not-run sentence"
        );
        assert!(
            !body.contains("READY_FOR_LIVE_TEST: yes")
                && !body.contains("READY_FOR_LIVE_TEST`: yes"),
            "{rel} must keep READY_FOR_LIVE_TEST no"
        );
    }
let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
assert!(
        !gate.contains("/tmp/cell-one-target-c-live-20260923"),
        "this recording must not rewrite GATE-90"
    );
assert!(
        !gate.contains("READY_FOR_LIVE_TEST: yes") && !gate.contains("READY_FOR_LIVE_TEST`: yes"),
        "GATE-90 must not flip READY_FOR_LIVE_TEST"
    );

let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        !makefile.contains("cell-one-target-c-live")
            && !makefile.contains("cell-target-c-qlora-prove"),
        "Makefile must not grow a live uniqueness target"
    );
for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("cell-one-target-c-live") && !body.contains("cell-target-c-qlora-prove"),
            "{rel} must not run the recorded uniqueness prove"
        );
    }
}

#[test]
fn uniqueness_prove_checklist_prints_recorded_steps_and_stays_off_gates() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        makefile
            .lines()
            .any(|line| line.trim() == "uniqueness-prove-checklist:"),
        "Makefile missing uniqueness-prove-checklist"
    );
assert!(makefile.contains("scripts/uniqueness-prove-checklist.sh"));
assert!(
        makefile.contains("Do not add to smoke, gate-90, or GitHub Actions"),
        "uniqueness-prove-checklist must stay off smoke, gate-90, and Actions"
    );
let phony = makefile.lines().next().unwrap_or("");
assert!(
        phony.contains("uniqueness-prove-checklist"),
        "uniqueness-prove-checklist must be a phony target"
    );
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("uniqueness-prove-checklist"),
        "gate-90 must not run uniqueness-prove-checklist: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("uniqueness-prove-checklist"),
        "smoke must not run uniqueness-prove-checklist: {smoke}"
    );
assert!(
        !makefile.contains("cell-one-target-c-live")
            && !makefile.contains("cell-target-c-qlora-prove"),
        "Makefile must not grow a live uniqueness target"
    );
let script_path = root.join("scripts/uniqueness-prove-checklist.sh");
assert!(
        script_path.is_file(),
        "scripts/uniqueness-prove-checklist.sh missing"
    );
let script = std::fs::read_to_string(&script_path).unwrap();
let qlora = std::fs::read_to_string(root.join("scripts/qlora-journey.sh")).unwrap();
assert_eq!(
        extract_shell_fn(&script, "resolve_estate"),
        extract_shell_fn(&qlora, "resolve_estate"),
        "checklist estate resolver must match the print-journey fallback"
    );
assert!(
        !script
            .lines()
            .any(|line| line.trim_start().starts_with("estate()")),
        "checklist must not shell out through an estate() wrapper"
    );
for needle in [
        "Print-only",
        "READY_FOR_LIVE_TEST: no",
        "The factory does not train, convert, shell out to ollama, or promote.",
        "CELL_TRAIN_LIVE=1 stays print-only.",
        "CELL_SEAT_LIVE=1 stays print-only.",
        "CELL_TRAIN_LIVE=1 is set. This journey stays print-only.",
        "CELL_SEAT_LIVE=1 is set. This journey stays print-only.",
        "Not native MLX.",
        "SKIP live train",
        "llamafactory-qlora",
        "Seat tag llama3",
        "Qwen/Qwen2.5-0.5B-Instruct",
        "Print the NEXT.md recipe.",
        "refuse:tokenizer",
        "cp -aL",
        "cp --dereference",
        "does not write \\$PREPARED/Modelfile",
        "from the printed contents",
        "trained_shape gguf",
        "auto_apply=false",
        "ollama rm",
        "43770130 3391",
        "does not invent a new live PASS",
        "This print is not a live PASS.",
        "Target C live uniqueness (5090-class)",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "docs/LIVE-PROBES.md",
        "Standing next (estate)",
        "does not apply the estate without an explicit operator --require-plan path",
        "No promote. No auto-promote.",
        "examples/estate.yaml stays unchanged unless the operator deliberately applies a plan.",
        "Existing entrypoints (print only; this checklist does not execute them):",
        "plan --estate <lab-estate.yaml>",
        "apply --estate <lab-estate.yaml> --state-dir .cell --require-plan --curator jason",
        "reconcile --estate <lab-estate.yaml>",
        "reconcile --suggest",
        "packs accept --id <pack-id> --curator jason",
        "Always fails. Auto-promote is locked off.",
        "Re-prove card: make uniqueness-prove-checklist.",
        "Phrase-check passed:",
        "coda_forbid \"The factory applied\"",
        "coda_forbid \"The factory promoted\"",
        "coda_forbid \"The factory trained\"",
        "coda_forbid \"The factory converted\"",
        "coda_forbid \"The factory shelled out\"",
        "coda_forbid \"--estate examples/estate.yaml\"",
    ] {
        assert!(
            script.contains(needle),
            "uniqueness-prove-checklist missing {needle}"
        );
    }
assert!(
        script
            .lines()
            .filter(|line| line.contains("READY_FOR_LIVE_TEST: yes"))
            .all(|line| line.trim_start().starts_with("coda_forbid ")),
        "uniqueness-prove-checklist must keep READY_FOR_LIVE_TEST no except the coda forbid"
    );
let executed: Vec<&str> = script
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#')
                || trimmed.starts_with("echo")
                || trimmed.starts_with("require_phrase ")
                || trimmed.starts_with("coda_require ")
                || trimmed.starts_with("coda_forbid ")
                || trimmed.starts_with("coda_before ")
                || trimmed.starts_with("ESTATE_CMD=(")
                || trimmed.starts_with("prefix=")
            {
                return false;
            }
            trimmed.contains("ollama ")
                || trimmed.contains("llamafactory-cli")
                || trimmed.contains("convert_hf_to_gguf.py")
                || trimmed.contains("${ESTATE_CMD[@]}\"")
                || trimmed.starts_with("make ")
                || trimmed.contains(" estate enrich ")
        }
)
        .collect();
assert!(
        executed.is_empty(),
        "uniqueness-prove-checklist must not train, convert, seat, or shell out: {executed:?}"
    );
let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
assert!(
        !gate.contains("cp -aL"),
        "GATE-90 must not grow the copy command"
    );
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
        .find(|line| line.contains("| `make uniqueness-prove-checklist` |"))
        .expect("remaining row for uniqueness-prove-checklist");
assert!(row.contains("Does not train"), "{row}");
assert!(row.contains("Not in smoke or Actions"), "{row}");
assert!(row.contains("Not a live train"), "{row}");
assert!(row.contains("Does not invent a live PASS"), "{row}");
assert!(row.contains("Not native MLX"), "{row}");
assert!(row.contains("LIVE-PROBES.md"), "{row}");
assert!(row.contains("Standing next (estate)"), "{row}");
assert!(row.contains("auto_apply=false"), "{row}");
assert!(row.contains("--require-plan"), "{row}");
assert!(row.contains("does not execute them"), "{row}");
assert!(row.contains("reconcile"), "{row}");
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
let uniq = status
        .split("## Train/enrich uniqueness (matrix PR #140, prepare walk PR #142)")
        .nth(1)
        .expect("uniqueness section")
        .split("\n## ")
        .next()
        .unwrap();
assert!(uniq.contains("make uniqueness-prove-checklist"), "{uniq}");
assert!(uniq.contains("does not invent a new live PASS"), "{uniq}");
assert!(uniq.contains("Standing next (estate)"), "{uniq}");
assert!(
        uniq.contains("does not execute them"),
        "uniqueness section must name the print-only estate coda: {uniq}"
    );
assert!(
        uniq.contains("CELL_TRAIN_LIVE=1") && uniq.contains("CELL_SEAT_LIVE=1"),
        "{uniq}"
    );
assert!(
        !uniq.contains("READY_FOR_LIVE_TEST: yes") && !uniq.contains("READY_FOR_LIVE_TEST`: yes"),
        "uniqueness section must keep READY_FOR_LIVE_TEST no"
    );
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
assert!(journey.contains("make uniqueness-prove-checklist"));
assert!(journey.contains("does not invent a new live PASS"));
assert!(journey.contains("Standing next (estate)"));
assert!(journey.contains("does not execute them"));
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
assert!(train.contains("`make uniqueness-prove-checklist`"));
assert!(train.contains("does not invent a new live PASS"));
assert!(train.contains("Standing next (estate)"));
assert!(train.contains("does not execute them"));
let probes = std::fs::read_to_string(root.join("docs/LIVE-PROBES.md")).unwrap();
let section = probes
        .split("## Target C live uniqueness (5090-class)")
        .nth(1)
        .expect("LIVE-PROBES section")
        .split("\n## ")
        .next()
        .unwrap();
assert!(section.contains("`make uniqueness-prove-checklist`"));
assert!(section.contains("does not invent a new live PASS"));
let seat = std::fs::read_to_string(root.join("docs/local-seat.md")).unwrap();
assert!(seat.contains("make uniqueness-prove-checklist"));
assert!(seat.contains("Standing next (estate)"));
assert!(
        !journey.contains("READY_FOR_LIVE_TEST: yes")
            && !train.contains("READY_FOR_LIVE_TEST: yes")
            && !seat.contains("READY_FOR_LIVE_TEST: yes"),
        "operator pages must keep READY_FOR_LIVE_TEST no"
    );
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains("make uniqueness-prove-checklist"));
assert!(help.contains("does not invent a live PASS"));
assert!(help.contains("Standing next (estate)"));
assert!(help.contains("does not execute them"));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));


for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("uniqueness-prove-checklist"),
            "{rel} must not run uniqueness-prove-checklist"
        );
    }
let before = std::fs::read(root.join("examples/estate.yaml")).unwrap();
let syntax = std::process::Command::new("bash")
        .arg("-n")
        .arg(&script_path)
        .output()
        .unwrap();
assert!(
        syntax.status.success(),
        "bash -n failed: {}",
        String::from_utf8_lossy(&syntax.stderr)
    );
let run = |train: bool, seat: bool| {
        let mut cmd = std::process::Command::new("bash");
        cmd.arg(&script_path)
            .current_dir(&root)
            .env_remove("ESTATE_BIN")
            .env_remove("XAI_API_KEY");
        if train {
            cmd.env("CELL_TRAIN_LIVE", "1");
        } else {
            cmd.env_remove("CELL_TRAIN_LIVE");
        }
        if seat {
            cmd.env("CELL_SEAT_LIVE", "1");
        } else {
            cmd.env_remove("CELL_SEAT_LIVE");
        }
        cmd.output().unwrap()
    }
;
for (train, seat) in [(false, false), (true, true)] {
        let output = run(train, seat);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        assert!(
            output.status.success(),
            "checklist failed train={train} seat={seat}\n{stdout}\n{stderr}"
        );
        assert!(stderr.is_empty(), "checklist stderr: {stderr}");
        for needle in [
            "Print-only. READY_FOR_LIVE_TEST: no",
            "The factory does not train, convert, shell out to ollama, or promote.",
            "CELL_TRAIN_LIVE=1 stays print-only.",
            "CELL_SEAT_LIVE=1 stays print-only.",
            "Not native MLX.",
            "This checklist does not invent a new live PASS.",
            "Recorded order (not a new live PASS): Prepare →",
            "1. Prepare the LLaMA-Factory QLoRA card (llamafactory-qlora). Seat tag llama3. Train base Qwen/Qwen2.5-0.5B-Instruct.",
            "2. Train and export outside the factory. Print the NEXT.md recipe. SKIP live train.",
            "3. After refuse:tokenizer, restore tokenizer files with dereference (cp -aL or cp --dereference).",
            "enrich gguf-convert",
            "enrich local-seat",
            "does not write $PREPARED/Modelfile.",
            "6. The operator writes the Modelfile from the printed contents.",
            "7. ollama create outside the factory.",
            "enrich import-trained",
            "trained_shape gguf. auto_apply=false.",
            "9. Cleanup with ollama rm. examples/estate.yaml unchanged.",
            "cksum: 43770130 3391 ",
            "This print is not a live PASS.",
        ] {
            assert!(
                stdout.contains(needle),
                "output missing {needle} train={train} seat={seat}\n{stdout}"
            );
        }
        let marks = [
            "1. Prepare the LLaMA-Factory QLoRA card",
            "2. Train and export outside the factory",
            "3. After refuse:tokenizer",
            "4. ",
            "5. ",
            "6. The operator writes the Modelfile",
            "7. ollama create outside the factory",
            "8. ",
            "9. Cleanup with ollama rm",
        ];
        let mut prev = 0usize;
        for mark in marks {
            let at = stdout
                .find(mark)
                .unwrap_or_else(|| panic!("missing step {mark}"));
            assert!(at >= prev, "step order drifted at {mark}");
            prev = at;
        }
        let coda_marks = [
            "Standing next (estate)",
            "1. The proposal stays auto_apply=false.",
            "2. No promote. No auto-promote.",
            "3. Existing entrypoints (print only; this checklist does not execute them):",
            "enrich apply-proposal --estate <lab-estate.yaml>",
            " plan --estate <lab-estate.yaml> --plans-dir plans",
            " apply --estate <lab-estate.yaml> --state-dir .cell --require-plan --curator jason",
            " reconcile --estate <lab-estate.yaml> --state-dir .cell",
            " reconcile --suggest",
            "packs accept --id <pack-id> --curator jason",
            "packs promote --id <pack-id>",
            "feed promote --id <pack-id>",
            "4. Recorded PASS stays in docs/LIVE-PROBES.md section Target C live uniqueness (5090-class).",
            "Re-prove card: make uniqueness-prove-checklist.",
            "5. Phrase-check passed:",
        ];
        for mark in coda_marks {
            let at = stdout
                .find(mark)
                .unwrap_or_else(|| panic!("missing coda mark {mark}\n{stdout}"));
            assert!(at >= prev, "coda order drifted at {mark}");
            prev = at;
        }
        for claim in [
            "READY_FOR_LIVE_TEST: yes",
            "The factory applied",
            "The factory promoted",
            "The factory trained",
            "The factory converted",
            "The factory shelled out",
            "This checklist applied",
            "wrote examples/estate.yaml",
            "--estate examples/estate.yaml",
        ] {
            assert!(
                !stdout.contains(claim),
                "checklist claimed {claim}\n{stdout}"
            );
        }
        assert!(
            !stdout
                .lines()
                .any(|line| line.trim_start().starts_with("PASS")),
            "checklist must not invent a PASS line:\n{stdout}"
        );
        if train {
            assert!(stdout.contains("CELL_TRAIN_LIVE=1 is set. This journey stays print-only."));
        } else {
            assert!(!stdout.contains("CELL_TRAIN_LIVE=1 is set."));
        }
        if seat {
            assert!(stdout.contains("CELL_SEAT_LIVE=1 is set. This journey stays print-only."));
        } else {
            assert!(!stdout.contains("CELL_SEAT_LIVE=1 is set."));
        }
    }
let after = std::fs::read(root.join("examples/estate.yaml")).unwrap();
assert_eq!(
        before, after,
        "checklist must not write examples/estate.yaml"
    );
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
            !text.contains("Not a distillation")
                && !text.contains("Distillation and an AI gateway"),
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
        assert!(
            !glossary.to_lowercase().contains(slop),
            "glossary slop: {slop}"
        );
        assert!(
            !north.to_lowercase().contains(slop),
            "north-star slop: {slop}"
        );
    }
}

#[test]
fn purpose_build_checklist_prints_ordered_path_and_stays_off_gates() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        makefile
            .lines()
            .any(|line| line.trim() == "purpose-build-checklist:"),
        "Makefile missing purpose-build-checklist"
    );
assert!(makefile.contains("scripts/purpose-build-checklist.sh"));
assert!(
        makefile.contains("Do not add to smoke, gate-90, or GitHub Actions"),
        "purpose-build-checklist must stay off smoke, gate-90, and Actions"
    );
let phony = makefile.lines().next().unwrap_or("");
assert!(
        phony.contains("purpose-build-checklist"),
        "purpose-build-checklist must be a phony target"
    );
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("purpose-build-checklist"),
        "gate-90 must not run purpose-build-checklist: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("purpose-build-checklist"),
        "smoke must not run purpose-build-checklist: {smoke}"
    );
let script_path = root.join("scripts/purpose-build-checklist.sh");
assert!(
        script_path.is_file(),
        "scripts/purpose-build-checklist.sh missing"
    );
let script = std::fs::read_to_string(&script_path).unwrap();
let qlora = std::fs::read_to_string(root.join("scripts/qlora-journey.sh")).unwrap();
assert_eq!(
        extract_shell_fn(&script, "resolve_estate"),
        extract_shell_fn(&qlora, "resolve_estate"),
        "checklist estate resolver must match the print-journey fallback"
    );
assert!(
        !script
            .lines()
            .any(|line| line.trim_start().starts_with("estate()")),
        "checklist must not shell out through an estate() wrapper"
    );
for needle in [
        "Print-only",
        "READY_FOR_LIVE_TEST: no",
        "Purpose-build on demand",
        "not a re-prove",
        "The factory does not train, convert, shell out to ollama, or promote.",
        "The factory does not apply the estate.",
        "CELL_TRAIN_LIVE=1 stays print-only.",
        "CELL_SEAT_LIVE=1 stays print-only.",
        "CELL_TRAIN_LIVE=1 is set. This journey stays print-only.",
        "CELL_SEAT_LIVE=1 is set. This journey stays print-only.",
        "Not native MLX.",
        "SKIP live train",
        "make lf-beachhead-prepare",
        "make qlora-journey",
        "make lora-journey",
        "make train-next",
        "make axolotl-qlora-journey",
        "make axolotl-lora-journey",
        "make unsloth-qlora-journey",
        "make unsloth-lora-journey",
        "make mlx-lm-lora-journey",
        "make deepseek-r1-distill-journey",
        "make uniqueness-deepseek",
        "make deepseek-r1-distill-lora-journey",
        "make uniqueness-deepseek-lora",
        "make glm4-chat-journey",
        "make uniqueness-glm",
        "make glm4-chat-lora-journey",
        "make uniqueness-glm-lora",
        "This checklist does not run them.",
        "make purpose-build-pick",
        "This checklist does not run it.",
        "enrich merge-adapt",
        "enrich gguf-convert",
        "enrich local-seat",
        "does not write \\$PREPARED/Modelfile",
        "from the printed contents",
        "enrich import-trained",
        "trained_shape gguf",
        "auto_apply=false",
        "43770130 3391",
        "does not invent a new live PASS",
        "This print is not a live PASS.",
        "only live uniqueness prove",
        "Target C live uniqueness (5090-class)",
        "Re-prove card: make uniqueness-prove-checklist.",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "docs/LIVE-PROBES.md",
        "Standing next (estate)",
        "does not apply the estate without an explicit operator --require-plan path",
        "No promote. No auto-promote.",
        "examples/estate.yaml stays unchanged unless the operator deliberately applies a plan.",
        "Existing entrypoints (print only; this checklist does not execute them):",
        "enrich apply-proposal --estate <lab-estate.yaml>",
        "plan --estate <lab-estate.yaml>",
        "apply --estate <lab-estate.yaml> --state-dir .cell --require-plan --curator jason",
        "reconcile --estate <lab-estate.yaml>",
        "reconcile --suggest",
        "packs accept --id <pack-id> --curator jason",
        "Always fails. Auto-promote is locked off.",
        "Purpose-build card: make purpose-build-checklist.",
        "Phrase-check passed:",
        "coda_forbid \"The factory applied\"",
        "coda_forbid \"The factory promoted\"",
        "coda_forbid \"The factory trained\"",
        "coda_forbid \"The factory converted\"",
        "coda_forbid \"The factory shelled out\"",
        "coda_forbid \"--estate examples/estate.yaml\"",
    ] {
        assert!(
            script.contains(needle),
            "purpose-build-checklist missing {needle}"
        );
    }
assert!(
        script
            .lines()
            .filter(|line| line.contains("READY_FOR_LIVE_TEST: yes"))
            .all(|line| line.trim_start().starts_with("coda_forbid ")),
        "purpose-build-checklist must keep READY_FOR_LIVE_TEST no except the coda forbid"
    );
let executed: Vec<&str> = script
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#')
                || trimmed.starts_with("echo")
                || trimmed.starts_with("coda_require ")
                || trimmed.starts_with("coda_forbid ")
                || trimmed.starts_with("coda_before ")
                || trimmed.starts_with("ESTATE_CMD=(")
                || trimmed.starts_with("prefix=")
            {
                return false;
            }
            trimmed.contains("ollama ")
                || trimmed.contains("llamafactory-cli")
                || trimmed.contains("convert_hf_to_gguf.py")
                || trimmed.contains("${ESTATE_CMD[@]}\"")
                || trimmed.starts_with("make ")
                || trimmed.contains(" estate enrich ")
        }
)
        .collect();
assert!(
        executed.is_empty(),
        "purpose-build-checklist must not train, convert, seat, or shell out: {executed:?}"
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
        .find(|line| line.contains("| `make purpose-build-checklist` |"))
        .expect("remaining row for purpose-build-checklist");
assert!(row.contains("Does not train"), "{row}");
assert!(row.contains("Not in smoke or Actions"), "{row}");
assert!(row.contains("Not a live train"), "{row}");
assert!(row.contains("Does not invent a live PASS"), "{row}");
assert!(row.contains("Not native MLX"), "{row}");
assert!(row.contains("LIVE-PROBES.md"), "{row}");
assert!(row.contains("Standing next (estate)"), "{row}");
assert!(row.contains("auto_apply=false"), "{row}");
assert!(row.contains("--require-plan"), "{row}");
assert!(row.contains("does not execute them"), "{row}");
assert!(row.contains("reconcile"), "{row}");
assert!(row.contains("operator section 15"), "{row}");
assert!(row.contains("make deepseek-r1-distill-journey"), "{row}");
assert!(row.contains("make uniqueness-deepseek"), "{row}");
assert!(row.contains("make glm4-chat-journey"), "{row}");
assert!(row.contains("make uniqueness-glm"), "{row}");
assert!(row.contains("the checklist does not run them"), "{row}");
assert!(row.contains("only live uniqueness prove"), "{row}");
assert!(
        row.contains("make uniqueness-prove-checklist"),
        "purpose-build row must keep the re-prove card: {row}"
    );
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
let uniq = status
        .split("## Train/enrich uniqueness (matrix PR #140, prepare walk PR #142)")
        .nth(1)
        .expect("uniqueness section")
        .split("\n## ")
        .next()
        .unwrap();
assert!(uniq.contains("make purpose-build-checklist"), "{uniq}");
assert!(uniq.contains("does not invent a live PASS"), "{uniq}");
assert!(uniq.contains("Standing next (estate)"), "{uniq}");
assert!(uniq.contains("does not execute them"), "{uniq}");
assert!(uniq.contains("only live uniqueness prove"), "{uniq}");
assert!(
        uniq.contains("make uniqueness-prove-checklist"),
        "re-prove card stays the recorded prove checklist"
    );
assert!(
        !uniq.contains("READY_FOR_LIVE_TEST: yes") && !uniq.contains("READY_FOR_LIVE_TEST`: yes"),
        "uniqueness section must keep READY_FOR_LIVE_TEST no"
    );
assert!(status.contains("make purpose-build-checklist # opt-in:"));
assert!(status.contains("| checklist names DeepSeek and GLM |"));
assert!(uniq.contains("make deepseek-r1-distill-journey"), "{uniq}");
assert!(uniq.contains("make glm4-chat-journey"), "{uniq}");
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
let section = journey
        .split("## 15. Purpose-build on demand — operator checklist")
        .nth(1)
        .expect("operator section 15");
assert!(section.contains("make purpose-build-checklist"));
assert!(section.contains("make lf-beachhead-prepare"));
assert!(section.contains("SKIP live train"));
assert!(section.contains("merge-adapt"));
assert!(section.contains("gguf-convert"));
assert!(section.contains("local-seat"));
assert!(section.contains("does not write `$PREPARED/Modelfile`"));
assert!(section.contains("import-trained"));
assert!(section.contains("trained_shape` `gguf`"));
assert!(section.contains("auto_apply=false"));
assert!(section.contains("Standing next (estate)"));
assert!(section.contains("does not execute them"));
assert!(section.contains("does not invent a live PASS"));
assert!(section.contains("only live uniqueness prove"));
assert!(section.contains("make uniqueness-prove-checklist"));
assert!(section.contains("make deepseek-r1-distill-journey"));
assert!(section.contains("make uniqueness-deepseek"));
assert!(section.contains("make glm4-chat-journey"));
assert!(section.contains("make uniqueness-glm"));
assert!(section.contains("This checklist does not run them."));
assert!(
        !journey.contains("READY_FOR_LIVE_TEST: yes")
            && !journey.contains("READY_FOR_LIVE_TEST`: yes"),
        "operator page must keep READY_FOR_LIVE_TEST no"
    );
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains("make purpose-build-checklist"));
assert!(help.contains("does not invent a live PASS"));
assert!(help.contains("Standing next (estate)"));
assert!(help.contains("does not execute them"));
assert!(help.contains("section 15"));
assert!(help.contains("make deepseek-r1-distill-journey"));
assert!(help.contains("make uniqueness-deepseek"));
assert!(help.contains("make glm4-chat-journey"));
assert!(help.contains("make uniqueness-glm"));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));


let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
assert!(
        readme.contains("`make purpose-build-checklist`")
            && readme.contains(
                "print-only operator path for purpose-building an SLM on demand (operator section 15)"
            ),
        "{readme}"
    );
assert!(
        makefile.contains(
            "print-only operator path for purpose-building an SLM on demand (operator section 15)"
        ) && makefile.contains("DeepSeek-R1-Distill, GLM-4 Chat"),
        "Makefile comment must name the purpose-build path"
    );

for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("purpose-build-checklist"),
            "{rel} must not run purpose-build-checklist"
        );
    }
let before = std::fs::read(root.join("examples/estate.yaml")).unwrap();
let syntax = std::process::Command::new("bash")
        .arg("-n")
        .arg(&script_path)
        .output()
        .unwrap();
assert!(
        syntax.status.success(),
        "bash -n failed: {}",
        String::from_utf8_lossy(&syntax.stderr)
    );
let run = |train: bool, seat: bool| {
        let mut cmd = std::process::Command::new("bash");
        cmd.arg(&script_path)
            .current_dir(&root)
            .env_remove("ESTATE_BIN")
            .env_remove("XAI_API_KEY");
        if train {
            cmd.env("CELL_TRAIN_LIVE", "1");
        } else {
            cmd.env_remove("CELL_TRAIN_LIVE");
        }
        if seat {
            cmd.env("CELL_SEAT_LIVE", "1");
        } else {
            cmd.env_remove("CELL_SEAT_LIVE");
        }
        cmd.output().unwrap()
    }
;
for (train, seat) in [(false, false), (true, true)] {
        let output = run(train, seat);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        assert!(
            output.status.success(),
            "checklist failed train={train} seat={seat}\n{stdout}\n{stderr}"
        );
        assert!(stderr.is_empty(), "checklist stderr: {stderr}");
        for needle in [
            "Print-only. READY_FOR_LIVE_TEST: no",
            "Purpose-build on demand. This checklist is operator UX.",
            "It is not a re-prove of the recorded Target C live PASS.",
            "The factory does not train, convert, shell out to ollama, or promote.",
            "The factory does not apply the estate.",
            "That recorded PASS stays the only live uniqueness prove.",
            "This checklist does not invent a new live PASS.",
            "1. Choose and prepare a train card.",
            "make lf-beachhead-prepare",
            "DeepSeek-R1-Distill chat print pointer: make deepseek-r1-distill-journey and make uniqueness-deepseek",
            "GLM-4 Chat print pointer: make glm4-chat-journey and make uniqueness-glm",
            "This checklist does not run them.",
            "Host and stack picker: make purpose-build-pick (operator section 17).",
            "2. Train handoff, train-next style. Print the NEXT.md recipe. SKIP live train.",
            "3. Merge and export print honesty.",
            "enrich merge-adapt",
            "4. ",
            "enrich gguf-convert",
            "5. ",
            "enrich local-seat",
            "does not write $PREPARED/Modelfile.",
            "6. ",
            "enrich import-trained",
            "trained_shape gguf. auto_apply=false.",
            "7. Standing next (estate). Print only. This checklist does not execute it.",
            "cksum: 43770130 3391 ",
            "This print is not a live PASS.",
            "Purpose-build card: make purpose-build-checklist.",
        ] {
            assert!(
                stdout.contains(needle),
                "output missing {needle} train={train} seat={seat}\n{stdout}"
            );
        }
        let marks = [
            "1. Choose and prepare a train card.",
            "2. Train handoff, train-next style.",
            "3. Merge and export print honesty.",
            "4. ",
            "5. ",
            "6. ",
            "7. Standing next (estate).",
        ];
        let mut prev = 0usize;
        for mark in marks {
            let at = stdout
                .find(mark)
                .unwrap_or_else(|| panic!("missing step {mark}"));
            assert!(at >= prev, "step order drifted at {mark}");
            prev = at;
        }
        let coda_marks = [
            "Standing next (estate) — after step 6",
            "1. The proposal stays auto_apply=false.",
            "2. No promote. No auto-promote.",
            "3. Existing entrypoints (print only; this checklist does not execute them):",
            "enrich apply-proposal --estate <lab-estate.yaml>",
            " plan --estate <lab-estate.yaml> --plans-dir plans",
            " apply --estate <lab-estate.yaml> --state-dir .cell --require-plan --curator jason",
            " reconcile --estate <lab-estate.yaml> --state-dir .cell",
            " reconcile --suggest",
            "packs accept --id <pack-id> --curator jason",
            "packs promote --id <pack-id>",
            "feed promote --id <pack-id>",
            "4. Recorded PASS stays in docs/LIVE-PROBES.md section Target C live uniqueness (5090-class).",
            "Re-prove card: make uniqueness-prove-checklist.",
            "5. Phrase-check passed:",
        ];
        for mark in coda_marks {
            let at = stdout
                .find(mark)
                .unwrap_or_else(|| panic!("missing coda mark {mark}\n{stdout}"));
            assert!(at >= prev, "coda order drifted at {mark}");
            prev = at;
        }
        for claim in [
            "READY_FOR_LIVE_TEST: yes",
            "The factory applied",
            "The factory promoted",
            "The factory trained",
            "The factory converted",
            "The factory shelled out",
            "--estate examples/estate.yaml",
        ] {
            assert!(
                !stdout.contains(claim),
                "checklist claimed {claim} train={train} seat={seat}\n{stdout}"
            );
        }
        if train {
            assert!(stdout.contains("CELL_TRAIN_LIVE=1 is set. This journey stays print-only."));
        }
        if seat {
            assert!(stdout.contains("CELL_SEAT_LIVE=1 is set. This journey stays print-only."));
        }
    }
let after = std::fs::read(root.join("examples/estate.yaml")).unwrap();
assert_eq!(before, after, "checklist must not rewrite examples/estate.yaml");
}

#[test]
fn purpose_build_pick_prints_host_table_and_stays_off_gates() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        makefile
            .lines()
            .any(|line| line.trim() == "purpose-build-pick:"),
        "Makefile missing purpose-build-pick"
    );
assert!(makefile.contains("scripts/purpose-build-pick.sh"));
let phony = makefile.lines().next().unwrap_or("");
assert!(
        phony.contains("purpose-build-pick"),
        "purpose-build-pick must be a phony target"
    );
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("purpose-build-pick"),
        "gate-90 must not run purpose-build-pick: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("purpose-build-pick"),
        "smoke must not run purpose-build-pick: {smoke}"
    );
assert!(makefile.contains(
        "print-only host and stack picker for purpose-build journeys (operator section 17)"
    ));
let script_path = root.join("scripts/purpose-build-pick.sh");
let script = std::fs::read_to_string(&script_path).unwrap();
for needle in [
        "Print-only",
        "READY_FOR_LIVE_TEST: no",
        "does not run them",
        "does not resolve or execute estate",
        "make lf-beachhead-prepare",
        "make qlora-journey",
        "make uniqueness-full",
        "make unsloth-qlora-journey",
        "make uniqueness-unsloth",
        "make axolotl-qlora-journey",
        "make uniqueness-axolotl",
        "make mlx-lm-lora-journey",
        "make uniqueness-mlx",
        "refuse:host",
        "make lora-journey",
        "make uniqueness-full-lora",
        "make unsloth-lora-journey",
        "make uniqueness-unsloth-lora",
        "make axolotl-lora-journey",
        "make uniqueness-axolotl-lora",
        "make deepseek-r1-distill-journey",
        "make uniqueness-deepseek",
        "make deepseek-r1-distill-lora-journey",
        "make uniqueness-deepseek-lora",
        "DeepSeek-R1-Distill chat is print-only",
        "This picker does not run them.",
        "make purpose-build-checklist",
        "make uniqueness-prove-checklist",
        "43770130 3391",
        "does not invent a new live PASS",
        "This print is not a live PASS.",
        "only live uniqueness prove",
        "Not native MLX.",
        "CELL_TRAIN_LIVE=1 stays print-only.",
        "CELL_SEAT_LIVE=1 stays print-only.",
        "CELL_TRAIN_LIVE=1 is set. This journey stays print-only.",
        "CELL_SEAT_LIVE=1 is set. This journey stays print-only.",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "Purpose-build picker: make purpose-build-pick.",
    ] {
        assert!(script.contains(needle), "purpose-build-pick missing {needle}");
    }
assert!(!script.contains("READY_FOR_LIVE_TEST: yes"));
assert!(
        !script.to_ascii_lowercase().contains("kimi"),
        "picker must not name Kimi"
    );
let executed: Vec<&str> = script
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') || trimmed.starts_with("echo") || trimmed.starts_with("printf")
            {
                return false;
            }
            trimmed.contains("ollama ")
                || trimmed.contains("llamafactory-cli")
                || trimmed.contains("convert_hf_to_gguf.py")
                || trimmed.contains("mlx_lm")
                || trimmed.starts_with("make ")
                || trimmed.contains(" estate enrich ")
        }
)
        .collect();
assert!(
        executed.is_empty(),
        "purpose-build-pick must not train, fuse, convert, or shell out: {executed:?}"
    );
let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
assert!(!gate.contains("READY_FOR_LIVE_TEST: yes"));
let row = gate
        .lines()
        .find(|line| line.contains("| `make purpose-build-pick` |"))
        .expect("remaining row for purpose-build-pick");
assert!(row.contains("operator section 17"), "{row}");
assert!(row.contains("Does not execute them"), "{row}");
assert!(row.contains("Does not invent a live PASS"), "{row}");
assert!(row.contains("Not native MLX"), "{row}");
assert!(row.contains("Not a live train"), "{row}");
assert!(row.contains("DeepSeek-R1-Distill chat print-only"), "{row}");
assert!(row.contains("make deepseek-r1-distill-journey"), "{row}");
assert!(row.contains("make uniqueness-deepseek"), "{row}");
assert!(row.contains("make deepseek-r1-distill-lora-journey"), "{row}");
assert!(row.contains("make uniqueness-deepseek-lora"), "{row}");
assert!(row.contains("GLM-4 Chat print-only"), "{row}");
assert!(row.contains("make glm4-chat-journey"), "{row}");
assert!(row.contains("make uniqueness-glm"), "{row}");
assert!(row.contains("make glm4-chat-lora-journey"), "{row}");
assert!(row.contains("make uniqueness-glm-lora"), "{row}");
assert!(row.contains("this picker does not run them"), "{row}");
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
let section = journey
        .split("## 17. Purpose-build pick — host and stack table")
        .nth(1)
        .expect("operator section 17");
assert!(section.contains("make purpose-build-pick"));
assert!(section.contains("make uniqueness-full"));
assert!(section.contains("refuse:host"));
assert!(section.contains("make uniqueness-full-lora"));
assert!(section.contains("only live uniqueness prove"));
assert!(!journey.contains("READY_FOR_LIVE_TEST: yes"));
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains(
        "make purpose-build-pick is the print-only host and stack picker for purpose-build journeys (operator section 17)"
    ));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));
let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
assert!(readme.contains("`make purpose-build-pick`"));
assert!(readme.contains(
        "print-only host and stack picker for purpose-build journeys (operator section 17)"
    ));
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
assert!(status.contains("make purpose-build-pick # opt-in:"));

for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("purpose-build-pick"),
            "{rel} must not run purpose-build-pick"
        );
    }
let before = std::fs::read(root.join("examples/estate.yaml")).unwrap();
let syntax = std::process::Command::new("bash")
        .arg("-n")
        .arg(&script_path)
        .output()
        .unwrap();
assert!(
        syntax.status.success(),
        "{}",
        String::from_utf8_lossy(&syntax.stderr)
    );
for (train, seat) in [(false, false), (true, true)] {
        let mut cmd = std::process::Command::new("bash");
        cmd.arg(&script_path)
            .current_dir(&root)
            .env_remove("XAI_API_KEY");
        if train {
            cmd.env("CELL_TRAIN_LIVE", "1");
        } else {
            cmd.env_remove("CELL_TRAIN_LIVE");
        }
        if seat {
            cmd.env("CELL_SEAT_LIVE", "1");
        } else {
            cmd.env_remove("CELL_SEAT_LIVE");
        }
        let output = cmd.output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        assert!(
            output.status.success(),
            "picker failed train={train} seat={seat}\n{stdout}\n{stderr}"
        );
        assert!(stderr.is_empty(), "{stderr}");
        for needle in [
            "Print-only. READY_FOR_LIVE_TEST: no",
            "Nvidia / CUDA",
            "primary",
            "make lf-beachhead-prepare",
            "make qlora-journey",
            "make uniqueness-full",
            "optional",
            "make unsloth-qlora-journey",
            "integration",
            "make axolotl-qlora-journey",
            "Apple Silicon",
            "make mlx-lm-lora-journey",
            "make uniqueness-mlx",
            "refuse:host",
            "Target A LoRA twin",
            "make lora-journey",
            "make uniqueness-full-lora",
            "make unsloth-lora-journey",
            "make uniqueness-unsloth-lora",
            "make axolotl-lora-journey",
            "make uniqueness-axolotl-lora",
            "cksum: 43770130 3391 ",
            "This print is not a live PASS.",
            "Purpose-build picker: make purpose-build-pick.",
            "This picker does not run it.",
        ] {
            assert!(stdout.contains(needle), "output missing {needle}\n{stdout}");
        }
        assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"));
        if train {
            assert!(stdout.contains("CELL_TRAIN_LIVE=1 is set. This journey stays print-only."));
        }
        if seat {
            assert!(stdout.contains("CELL_SEAT_LIVE=1 is set. This journey stays print-only."));
        }
    }
let after = std::fs::read(root.join("examples/estate.yaml")).unwrap();
assert_eq!(before, after, "picker must not rewrite examples/estate.yaml");
}

#[test]
fn purpose_build_journey_chains_pick_then_checklist_and_stays_off_gates() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        makefile
            .lines()
            .any(|line| line.trim() == "purpose-build-journey:"),
        "Makefile missing purpose-build-journey"
    );
assert!(makefile.contains("scripts/purpose-build-journey.sh"));
let phony = makefile.lines().next().unwrap_or("");
assert!(
        phony.contains("purpose-build-journey"),
        "purpose-build-journey must be a phony target"
    );
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("purpose-build-journey"),
        "gate-90 must not run purpose-build-journey: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("purpose-build-journey"),
        "smoke must not run purpose-build-journey: {smoke}"
    );
assert!(makefile.contains(
        "print-only purpose-build on-demand entry (operator section 18)"
    ));
assert!(
        !makefile.contains("uniqueness-purpose-build"),
        "uniqueness-* stays a prepare-then-seat chain; no alias for this entry"
    );
let script_path = root.join("scripts/purpose-build-journey.sh");
let script = std::fs::read_to_string(&script_path).unwrap();
for needle in [
        "Print-only",
        "READY_FOR_LIVE_TEST: no",
        "make purpose-build-pick",
        "make purpose-build-checklist",
        "does not inline their bodies",
        "does not resolve or execute estate beyond what those targets already do",
        "43770130 3391",
        "does not invent a new live PASS",
        "This print is not a live PASS.",
        "only live uniqueness prove",
        "make uniqueness-prove-checklist",
        "Not native MLX.",
        "CELL_TRAIN_LIVE=1 stays print-only.",
        "CELL_SEAT_LIVE=1 stays print-only.",
        "CELL_TRAIN_LIVE=1 is set. This journey stays print-only.",
        "CELL_SEAT_LIVE=1 is set. This journey stays print-only.",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "Purpose-build journey: make purpose-build-journey.",
        "make -C \"$ROOT\" purpose-build-pick",
        "make -C \"$ROOT\" purpose-build-checklist",
    ] {
        assert!(
            script.contains(needle),
            "purpose-build-journey missing {needle}"
        );
    }
assert!(
        script.find("make -C \"$ROOT\" purpose-build-pick")
            < script.find("make -C \"$ROOT\" purpose-build-checklist"),
        "journey must run the pick before the checklist"
    );
assert!(!script.contains("READY_FOR_LIVE_TEST: yes"));
assert!(
        !script.to_ascii_lowercase().contains("kimi"),
        "journey must not name Kimi"
    );
assert!(
        !script.contains("resolve_estate"),
        "journey must not resolve estate itself"
    );
let executed: Vec<&str> = script
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') || trimmed.starts_with("echo") || trimmed.starts_with("printf")
            {
                return false;
            }
            let make_child = trimmed.contains("make -C \"$ROOT\" purpose-build-pick")
                || trimmed.contains("make -C \"$ROOT\" purpose-build-checklist");
            if make_child {
                return false;
            }
            trimmed.contains("ollama ")
                || trimmed.contains("llamafactory-cli")
                || trimmed.contains("convert_hf_to_gguf.py")
                || trimmed.contains("mlx_lm")
                || trimmed.contains("cargo ")
                || trimmed.contains("make ")
                || trimmed.contains(" estate enrich ")
        }
)
        .collect();
assert!(
        executed.is_empty(),
        "purpose-build-journey must not train, fuse, convert, or shell out: {executed:?}"
    );
let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
assert!(!gate.contains("READY_FOR_LIVE_TEST: yes"));
let row = gate
        .lines()
        .find(|line| line.contains("| `make purpose-build-journey` |"))
        .expect("remaining row for purpose-build-journey");
assert!(row.contains("operator section 18"), "{row}");
assert!(row.contains("make purpose-build-pick"), "{row}");
assert!(row.contains("make purpose-build-checklist"), "{row}");
assert!(row.contains("Does not invent a live PASS"), "{row}");
assert!(row.contains("Not native MLX"), "{row}");
assert!(row.contains("Not a live train"), "{row}");
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
let section = journey
        .split("## 18. Purpose-build journey — pick then checklist")
        .nth(1)
        .expect("operator section 18");
assert!(section.contains("make purpose-build-journey"));
assert!(section.contains("make purpose-build-pick"));
assert!(section.contains("make purpose-build-checklist"));
assert!(section.contains("only live uniqueness prove"));
assert!(!journey.contains("READY_FOR_LIVE_TEST: yes"));
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains(
        "When an SLM fits mid-software-build, or on demand, make purpose-build-journey is the print-only purpose-build on-demand entry (operator section 18)"
    ));
assert!(help.contains("operator section 18"));
assert!(help.contains("make purpose-build-checklist"));
assert!(help.contains("make purpose-build-pick"));
assert!(help.contains("make mlx-lm-lora-journey"));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));
let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
assert!(readme.contains("`make purpose-build-journey`"));
assert!(readme.contains("`make purpose-build-checklist`"));
assert!(readme.contains("`make purpose-build-pick`"));
assert!(readme.contains("`make mlx-lm-lora-journey`"));
assert!(readme.contains(
        "print-only purpose-build on-demand entry (operator section 18)"
    ));
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
assert!(status.contains("make purpose-build-journey # opt-in:"));
assert!(status.contains("make purpose-build-checklist # opt-in:"));
assert!(status.contains("make purpose-build-pick # opt-in:"));
let probes = std::fs::read_to_string(root.join("docs/LIVE-PROBES.md")).unwrap();
let recorded = probes
        .split("## Target C live uniqueness (5090-class)")
        .nth(1)
        .expect("LIVE-PROBES section")
        .split("\n## ")
        .next()
        .unwrap();
assert!(recorded.contains("`make purpose-build-journey`"));
assert!(recorded.contains("does not invent a new live PASS"));
assert!(recorded.contains("only live uniqueness prove"));

for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("purpose-build-journey"),
            "{rel} must not run purpose-build-journey"
        );
    }
let before = std::fs::read(root.join("examples/estate.yaml")).unwrap();
let syntax = std::process::Command::new("bash")
        .arg("-n")
        .arg(&script_path)
        .output()
        .unwrap();
assert!(
        syntax.status.success(),
        "{}",
        String::from_utf8_lossy(&syntax.stderr)
    );
for (train, seat) in [(false, false), (true, true)] {
        let mut cmd = std::process::Command::new("bash");
        cmd.arg(&script_path)
            .current_dir(&root)
            .env_remove("XAI_API_KEY");
        if train {
            cmd.env("CELL_TRAIN_LIVE", "1");
        } else {
            cmd.env_remove("CELL_TRAIN_LIVE");
        }
        if seat {
            cmd.env("CELL_SEAT_LIVE", "1");
        } else {
            cmd.env_remove("CELL_SEAT_LIVE");
        }
        let output = cmd.output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        assert!(
            output.status.success(),
            "journey failed train={train} seat={seat}\n{stdout}\n{stderr}"
        );
        assert!(stderr.is_empty(), "{stderr}");
        let pick_at = stdout
            .find("== purpose-build-pick")
            .expect("pick banner");
        let checklist_at = stdout
            .find("== purpose-build-checklist")
            .expect("checklist banner");
        assert!(
            pick_at < checklist_at,
            "pick must print before the checklist"
        );
        for needle in [
            "Print-only. READY_FOR_LIVE_TEST: no",
            "== purpose-build-journey (print-only purpose-build on-demand entry) ==",
            "== purpose-build-pick (print-only host and stack picker) ==",
            "== purpose-build-checklist (print-only operator steps for purpose-build on demand) ==",
            "cksum: 43770130 3391 ",
            "This print is not a live PASS.",
            "Purpose-build journey: make purpose-build-journey.",
            "This journey does not invent a new live PASS.",
        ] {
            assert!(stdout.contains(needle), "output missing {needle}\n{stdout}");
        }
        assert!(!stdout.contains("READY_FOR_LIVE_TEST: yes"));
        if train {
            assert!(stdout.contains("CELL_TRAIN_LIVE=1 is set. This journey stays print-only."));
        }
        if seat {
            assert!(stdout.contains("CELL_SEAT_LIVE=1 is set. This journey stays print-only."));
        }
    }
let after = std::fs::read(root.join("examples/estate.yaml")).unwrap();
assert_eq!(before, after, "journey must not rewrite examples/estate.yaml");
}

#[test]
fn deepseek_r1_distill_journey_stays_print_only_and_off_gates() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
for target in [
        "deepseek-r1-distill-journey:",
        "uniqueness-deepseek:",
        "deepseek-r1-distill-lora-journey:",
        "uniqueness-deepseek-lora:",
    ] {
        assert!(
            makefile.lines().any(|line| line.trim() == target),
            "Makefile missing {target}"
        );
    }
assert!(makefile.contains("scripts/deepseek-r1-distill-journey.sh"));
assert!(makefile.contains("scripts/uniqueness-deepseek.sh"));
assert!(makefile.contains("scripts/uniqueness-deepseek-lora.sh"));
assert!(makefile.contains("operator section 19"));
let phony = makefile.lines().next().unwrap_or("");
for name in [
        "deepseek-r1-distill-journey",
        "uniqueness-deepseek",
        "deepseek-r1-distill-lora-journey",
        "uniqueness-deepseek-lora",
    ] {
        assert!(phony.contains(name), "{name} must be a phony target");
    }
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("deepseek-r1-distill"),
        "gate-90 must not run the deepseek journey: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("deepseek-r1-distill"),
        "smoke must not run the deepseek journey: {smoke}"
    );
let script = std::fs::read_to_string(root.join("scripts/deepseek-r1-distill-journey.sh")).unwrap();
for needle in [
        "deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B",
        "examples/fixtures/deepseek-r1-distill.pack.json",
        "examples/fixtures/deepseek-r1-distill-lora.pack.json",
        "TEMPLATE=\"deepseekr1\"",
        "DEEPSEEK_R1_DISTILL_PHASE",
        "READY_FOR_LIVE_TEST: no",
        "SKIP live train",
        "CELL_TRAIN_LIVE or CELL_SEAT_LIVE is set. This journey stays print-only.",
        "deepseek-r1:1.5b",
        "llamafactory-qlora",
        "llamafactory-lora",
        "This print is not a live PASS.",
        "only live uniqueness prove",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
    ] {
        assert!(script.contains(needle), "journey missing {needle}");
    }
assert!(!script.contains("READY_FOR_LIVE_TEST: yes"));
assert!(!script.to_ascii_lowercase().contains("kimi"));
assert!(!script.contains("glm"));
let chain = std::fs::read_to_string(root.join("scripts/uniqueness-deepseek.sh")).unwrap();
let prepare_at = chain
        .find("DEEPSEEK_R1_DISTILL_PHASE=prepare make -C \"$ROOT\" deepseek-r1-distill-journey")
        .expect("prepare phase");
let seat_at = chain
        .find("DEEPSEEK_R1_DISTILL_PHASE=seat make -C \"$ROOT\" deepseek-r1-distill-journey")
        .expect("seat phase");
assert!(prepare_at < seat_at, "prepare-assert must run before seat-print");
assert!(chain.contains("Does not run make deepseek-r1-distill-lora-journey"));
assert!(chain.contains("READY_FOR_LIVE_TEST: no"));
let lora_chain = std::fs::read_to_string(root.join("scripts/uniqueness-deepseek-lora.sh")).unwrap();
assert!(lora_chain.contains("make -C \"$ROOT\" deepseek-r1-distill-lora-journey"));
assert!(lora_chain.contains("Does not run make deepseek-r1-distill-journey or make uniqueness-deepseek."));
let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
assert!(!gate.contains("READY_FOR_LIVE_TEST: yes"));
for row_name in [
        "`make deepseek-r1-distill-journey`",
        "`make uniqueness-deepseek`",
        "`make deepseek-r1-distill-lora-journey`",
        "`make uniqueness-deepseek-lora`",
    ] {
        let row = gate
            .lines()
            .find(|line| line.contains(&format!("| {row_name} |")))
            .unwrap_or_else(|| panic!("missing remaining row {row_name}"));
        assert!(row.contains("operator section 19"), "{row}");
        assert!(row.contains("Not a live train"), "{row}");
    }
let pbj = gate
        .lines()
        .position(|line| line.contains("| `make purpose-build-journey` |"))
        .expect("purpose-build-journey row");
let ds = gate
        .lines()
        .position(|line| line.contains("| `make deepseek-r1-distill-journey` |"))
        .expect("deepseek journey row");
assert!(
        pbj < ds,
        "operator section 18 row must sit above section 19 rows: {pbj} {ds}"
    );
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
let section = journey
        .split("## 19. DeepSeek-R1-Distill chat — LLaMA-Factory print journey")
        .nth(1)
        .expect("operator section 19");
assert!(section.contains("make deepseek-r1-distill-journey"));
assert!(section.contains("make uniqueness-deepseek"));
assert!(section.contains("deepseekr1"));
assert!(section.contains("only live uniqueness prove"));
assert!(!journey.contains("READY_FOR_LIVE_TEST: yes"));
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains(
        "make deepseek-r1-distill-journey is the print-only DeepSeek-R1-Distill chat QLoRA journey (operator section 19)"
    ));
assert!(help.contains("make uniqueness-deepseek"));
assert!(help.contains("make deepseek-r1-distill-lora-journey"));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));
let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
assert!(readme.contains("`make deepseek-r1-distill-journey`"));
assert!(readme.contains("`make uniqueness-deepseek`"));
assert!(readme.contains(
        "print-only DeepSeek-R1-Distill chat QLoRA ladder (operator section 19)"
    ));
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
assert!(status.contains("make deepseek-r1-distill-journey # opt-in:"));
assert!(status.contains("| deepseek r1 distill journey |"));

for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("deepseek-r1-distill-journey"),
            "{rel} must not run the deepseek journey"
        );
    }
}

#[test]
fn glm4_chat_journey_stays_print_only_and_off_gates() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
for target in [
        "glm4-chat-journey:",
        "uniqueness-glm:",
        "glm4-chat-lora-journey:",
        "uniqueness-glm-lora:",
    ] {
        assert!(
            makefile.lines().any(|line| line.trim() == target),
            "Makefile missing {target}"
        );
    }
assert!(makefile.contains("scripts/glm4-chat-journey.sh"));
assert!(makefile.contains("scripts/uniqueness-glm.sh"));
assert!(makefile.contains("scripts/uniqueness-glm-lora.sh"));
assert!(makefile.contains("GLM_CARD=llamafactory-qlora bash scripts/glm4-chat-journey.sh"));
assert!(makefile.contains("GLM_CARD=llamafactory-lora bash scripts/glm4-chat-journey.sh"));
assert!(makefile.contains("operator section 20"));
let phony = makefile.lines().next().unwrap_or("");
for name in [
        "glm4-chat-journey",
        "uniqueness-glm",
        "glm4-chat-lora-journey",
        "uniqueness-glm-lora",
    ] {
        assert!(phony.contains(name), "{name} must be a phony target");
    }
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("glm4-chat"),
        "gate-90 must not run the glm4 journey: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("glm4-chat"),
        "smoke must not run the glm4 journey: {smoke}"
    );
let script = std::fs::read_to_string(root.join("scripts/glm4-chat-journey.sh")).unwrap();
for needle in [
        "zai-org/glm-4-9b-chat",
        "examples/fixtures/glm4-chat.pack.json",
        "examples/fixtures/glm4-chat-lora.pack.json",
        "TEMPLATE=\"glm4\"",
        "GLM4_CHAT_PHASE",
        "READY_FOR_LIVE_TEST: no",
        "SKIP live train",
        "CELL_TRAIN_LIVE or CELL_SEAT_LIVE is set. This journey stays print-only.",
        "glm4:9b",
        "llamafactory-qlora",
        "llamafactory-lora",
        "This print is not a live PASS.",
        "only live uniqueness prove",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, Qwen3 Instruct, and DeepSeek-R1-Distill chat QLoRA.",
        "Reproduce target on the unquantized LoRA card, the non-quant twin of the GLM-4 Chat QLoRA prepare.",
    ] {
        assert!(script.contains(needle), "journey missing {needle}");
    }
assert!(!script.contains("READY_FOR_LIVE_TEST: yes"));
assert!(!script.to_ascii_lowercase().contains("kimi"));
let chain = std::fs::read_to_string(root.join("scripts/uniqueness-glm.sh")).unwrap();
let prepare_at = chain
        .find("GLM4_CHAT_PHASE=prepare make -C \"$ROOT\" glm4-chat-journey")
        .expect("prepare phase");
let seat_at = chain
        .find("GLM4_CHAT_PHASE=seat make -C \"$ROOT\" glm4-chat-journey")
        .expect("seat phase");
assert!(prepare_at < seat_at, "prepare-assert must run before seat-print");
assert!(chain.contains("Does not run make glm4-chat-lora-journey"));
assert!(chain.contains("READY_FOR_LIVE_TEST: no"));
assert!(!chain.to_ascii_lowercase().contains("kimi"));
let lora_chain = std::fs::read_to_string(root.join("scripts/uniqueness-glm-lora.sh")).unwrap();
assert!(lora_chain.contains("make -C \"$ROOT\" glm4-chat-lora-journey"));
assert!(lora_chain.contains("Does not run make glm4-chat-journey or make uniqueness-glm."));
let pick = std::fs::read_to_string(root.join("scripts/purpose-build-pick.sh")).unwrap();
for needle in [
        "make glm4-chat-journey",
        "make uniqueness-glm",
        "make glm4-chat-lora-journey",
        "make uniqueness-glm-lora",
        "GLM-4 Chat is print-only",
        "This picker does not run them.",
    ] {
        assert!(pick.contains(needle), "purpose-build-pick missing {needle}");
    }
let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
assert!(!gate.contains("READY_FOR_LIVE_TEST: yes"));
for row_name in [
        "`make glm4-chat-journey`",
        "`make uniqueness-glm`",
        "`make glm4-chat-lora-journey`",
        "`make uniqueness-glm-lora`",
    ] {
        let row = gate
            .lines()
            .find(|line| line.contains(&format!("| {row_name} |")))
            .unwrap_or_else(|| panic!("missing remaining row {row_name}"));
        assert!(row.contains("operator section 20"), "{row}");
        assert!(row.contains("Not a live train"), "{row}");
    }
let ds = gate
        .lines()
        .position(|line| line.contains("| `make uniqueness-deepseek-lora` |"))
        .expect("deepseek lora row");
let glm = gate
        .lines()
        .position(|line| line.contains("| `make glm4-chat-journey` |"))
        .expect("glm journey row");
assert!(ds < glm, "section 19 rows must sit above section 20 rows");
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
let section = journey
        .split("## 20. GLM-4 Chat — LLaMA-Factory print journey")
        .nth(1)
        .expect("operator section 20");
assert!(section.contains("make glm4-chat-journey"));
assert!(section.contains("make uniqueness-glm"));
assert!(section.contains("glm4"));
assert!(section.contains("zai-org/glm-4-9b-chat"));
assert!(section.contains("only live uniqueness prove"));
assert!(!journey.contains("READY_FOR_LIVE_TEST: yes"));
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains(
        "make glm4-chat-journey is the print-only GLM-4 Chat QLoRA journey (operator section 20)"
    ));
assert!(help.contains("make uniqueness-glm is the print-only chain of that journey"));
assert!(help.contains("make glm4-chat-lora-journey"));
assert!(help.contains("make uniqueness-glm-lora"));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));
assert!(!help.to_ascii_lowercase().contains("kimi"));
let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
assert!(readme.contains("`make glm4-chat-journey`"));
assert!(readme.contains("`make uniqueness-glm`"));
assert!(readme.contains("print-only GLM-4 Chat QLoRA ladder (operator section 20)"));
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
assert!(status.contains("make glm4-chat-journey # opt-in:"));
assert!(status.contains("| glm4 chat journey |"));
assert!(status.contains("| checklist names DeepSeek and GLM |"));
let live = std::fs::read_to_string(root.join("docs/LIVE-PROBES.md")).unwrap();
assert!(live.contains("make glm4-chat-journey"));
assert!(live.contains("operator section 20"));
assert!(live.contains("only live uniqueness prove"));
assert!(!live.contains("READY_FOR_LIVE_TEST: yes"));

for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("glm4-chat-journey"),
            "{rel} must not run the glm4 journey"
        );
    }
}

#[test]
fn purpose_build_operator_surfaces_name_the_journey() {
let root = repo_root();
let day = std::fs::read_to_string(root.join("docs/OPERATOR-DAY.md")).unwrap();
for needle in [
        "make purpose-build-journey",
        "make purpose-build-pick",
        "make purpose-build-checklist",
        "Print-only purpose-build on demand",
        "Those two targets are the parts.",
        "does not train, convert, seat, promote, or apply",
        "not in `make smoke`, `make gate-90`, or GitHub Actions",
        "READY_FOR_LIVE_TEST`: no",
        "make uniqueness-prove-checklist",
        "only live uniqueness prove",
        "sections 15–20",
    ] {
        assert!(day.contains(needle), "OPERATOR-DAY missing {needle}");
    }
assert!(!day.contains("READY_FOR_LIVE_TEST`: yes"));
assert!(!day.contains("READY_FOR_LIVE_TEST: yes"));
let north = std::fs::read_to_string(root.join("docs/NORTH-STAR.md")).unwrap();
for needle in [
        "make purpose-build-journey",
        "print-only purpose-build on-demand entry",
        "make purpose-build-pick",
        "make purpose-build-checklist",
        "Those two targets are the parts.",
        "does not train, convert, seat, promote, or apply",
        "make uniqueness-prove-checklist",
        "only live uniqueness prove",
        "READY_FOR_LIVE_TEST` stays no",
        "sections 15–20",
    ] {
        assert!(north.contains(needle), "NORTH-STAR missing {needle}");
    }
assert!(!north.contains("READY_FOR_LIVE_TEST: yes"));
assert!(!north.contains("READY_FOR_LIVE_TEST`: yes"));
let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
let row = gate
        .lines()
        .find(|line| line.contains("| `make purpose-build-pick` |"))
        .expect("remaining row for purpose-build-pick");
for needle in [
        "DeepSeek-R1-Distill chat print-only",
        "`make deepseek-r1-distill-journey` and `make uniqueness-deepseek`",
        "LoRA twin `make deepseek-r1-distill-lora-journey` and `make uniqueness-deepseek-lora`",
        "GLM-4 Chat print-only",
        "`make glm4-chat-journey` and `make uniqueness-glm`",
        "LoRA twin `make glm4-chat-lora-journey` and `make uniqueness-glm-lora`",
        "this picker does not run them",
        "Does not execute them",
    ] {
        assert!(row.contains(needle), "purpose-build-pick row missing {needle}");
    }
assert!(!gate.contains("READY_FOR_LIVE_TEST: yes"));
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
assert!(status.contains("| operator surfaces name purpose-build |"));

}

#[test]
fn purpose_build_surfaces_name_mid_software_build_entry() {
let root = repo_root();
let entry = "When an SLM fits mid-software-build, or on demand, the same print-only entry is `make purpose-build-journey`.";
let parts = "Those two targets are the parts.";
let day = std::fs::read_to_string(root.join("docs/OPERATOR-DAY.md")).unwrap();
assert!(day.contains(entry), "OPERATOR-DAY missing the mid-software-build entry");
assert!(day.contains(parts), "OPERATOR-DAY missing the parts line");
assert!(day.contains("## 4. Enrich prepare"));
let north = std::fs::read_to_string(root.join("docs/NORTH-STAR.md")).unwrap();
assert!(north.contains(entry), "NORTH-STAR suite missing the mid-software-build entry");
assert!(
        north.contains("when an SLM fits mid-software-build, or on demand, this is the same print-only entry"),
        "NORTH-STAR operator loop missing the mid-software-build entry"
    );
assert!(north.contains(parts));
let journeys = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
let section15 = journeys
        .split("## 15. Purpose-build on demand")
        .nth(1)
        .expect("section 15")
        .split("## 16.")
        .next()
        .unwrap();
let section17 = journeys
        .split("## 17. Purpose-build pick")
        .nth(1)
        .expect("section 17")
        .split("## 18.")
        .next()
        .unwrap();
let section18 = journeys
        .split("## 18. Purpose-build journey")
        .nth(1)
        .expect("section 18")
        .split("## 19.")
        .next()
        .unwrap();
for (name, body) in [
        ("section 15", section15),
        ("section 17", section17),
        ("section 18", section18),
    ] {
        assert!(body.contains(entry), "{name} missing the mid-software-build entry");
        assert!(
            body.contains("make purpose-build-pick") && body.contains("make purpose-build-checklist"),
            "{name} must keep both parts"
        );
        assert!(body.contains(parts), "{name} missing the parts line");
    }
assert!(
        !section17.contains("`make purpose-build-checklist` (section 18)"),
        "section 17 must not cite the checklist as section 18"
    );
assert!(journeys.contains(entry));
let banner = "When an SLM fits mid-software-build, or on demand, the same print-only entry is make purpose-build-journey.";
for rel in [
        "scripts/purpose-build-journey.sh",
        "scripts/purpose-build-pick.sh",
        "scripts/purpose-build-checklist.sh",
    ] {
        let script = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(script.contains(banner), "{rel} missing the mid-software-build banner");
        assert!(
            script.contains("make purpose-build-pick") && script.contains("make purpose-build-checklist"),
            "{rel} must keep both parts"
        );
        assert!(script.contains("Those two targets are the parts."), "{rel} missing the parts line");
        assert!(script.contains("READY_FOR_LIVE_TEST: no"), "{rel} must stay print-only");
    }
let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
assert!(readme.contains(entry), "README missing the mid-software-build entry");
assert!(readme.contains("`make purpose-build-journey` (when an SLM fits mid-software-build, or on demand"));
let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
let row = gate
        .lines()
        .find(|line| line.contains("| `make purpose-build-journey` |"))
        .expect("remaining row");
assert!(row.contains("mid-software-build"), "{row}");
assert!(row.contains("Those two targets are the parts."), "{row}");
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
assert!(status.contains("| mid-software-build purpose-build |"));
assert!(status.contains(entry));
let probes = std::fs::read_to_string(root.join("docs/LIVE-PROBES.md")).unwrap();
assert!(probes.contains(entry), "LIVE-PROBES missing the mid-software-build entry");
assert!(probes.contains("Those two targets are the parts."));

for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("purpose-build-journey"),
            "{rel} must not run purpose-build-journey"
        );
    }
}

#[test]
fn help_names_mid_software_build_purpose_build_entry() {
let root = repo_root();
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
let long = "When an SLM fits mid-software-build, or on demand, make purpose-build-journey is the print-only purpose-build on-demand entry (operator section 18).";
let index = "when an SLM fits mid-software-build, or on demand, the same print-only purpose-build on-demand entry (operator section 18)";
assert_eq!(help.matches(long).count(), 2, "enrich and train paragraphs both name mid-software-build");
assert!(help.contains(index), "index Journey line must name mid-software-build");
assert!(help.contains("operator section 18"));
assert!(help.contains("It runs make purpose-build-pick, then make purpose-build-checklist. It does not inline those bodies."));
assert!(help.contains("It does not train, fuse, convert, shell out to ollama, promote, or apply the estate."));
assert!(help.contains("It does not invent a live PASS."));
assert!(help.contains("Not in make smoke, make gate-90, or Actions. READY_FOR_LIVE_TEST stays no."));
assert!(help.contains("Walk: docs/operator-enrich-journeys.md (section 18)."));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));
let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
assert!(status.contains("| help names mid-software-build |"));

for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("purpose-build-journey"),
            "{rel} must not run purpose-build-journey"
        );
    }
}

#[test]
fn train_enrich_names_purpose_build_mid_software_build_entry() {
let root = repo_root();
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
let intro = train
        .split("## Target C — Qwen QLoRA operator journey")
        .next()
        .unwrap();
assert!(intro.contains(
        "When an SLM fits mid-software-build, or on demand, `make purpose-build-journey` is the print-only purpose-build on-demand entry."
    ));
assert!(intro.contains(
        "It runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts."
    ));
assert!(intro.contains("section 18"));
assert!(intro.contains("The journey runs the pick (section 17), then the checklist (section 15)."));
assert!(intro.contains(
        "DeepSeek-R1-Distill (section 19) and GLM-4 Chat (section 20) are sibling print-only journeys reachable from the pick and the checklist."
    ));
assert!(intro.contains("They are not steps of this journey."));
assert!(!intro.contains("sections 15–20"));
assert!(intro.contains("The re-prove card stays `make uniqueness-prove-checklist`."));
assert!(intro.contains("stays the only live uniqueness prove"));
assert!(
        !intro.contains("READY_FOR_LIVE_TEST: yes") && !intro.contains("READY_FOR_LIVE_TEST`: yes")
    );
let table = train
        .split("## What exists today")
        .nth(1)
        .expect("What exists today")
        .split("## Facilitated vs invented")
        .next()
        .unwrap();
let mlx = table
        .find("| `make uniqueness-mlx` |")
        .expect("uniqueness-mlx row");
let live = table
        .find("| `make enrich-live-prove` |")
        .expect("enrich-live-prove row");
let between = &table[mlx..live];
let row_markers = [
        "| `make purpose-build-checklist` |",
        "| `make purpose-build-pick` |",
        "| `make purpose-build-journey` |",
        "| `make deepseek-r1-distill-journey` |",
        "| `make uniqueness-deepseek` |",
        "| `make deepseek-r1-distill-lora-journey` |",
        "| `make uniqueness-deepseek-lora` |",
        "| `make glm4-chat-journey` |",
        "| `make uniqueness-glm` |",
        "| `make glm4-chat-lora-journey` |",
        "| `make uniqueness-glm-lora` |",
    ];
for marker in row_markers {
        let row = table
            .lines()
            .find(|line| line.contains(marker))
            .unwrap_or_else(|| panic!("What exists today missing {marker}"));
        for needle in [
            "print-only",
            "Does not train, convert, seat",
            "promote",
            "or apply",
            "Not in `make smoke`, `make gate-90`, or GitHub Actions",
            "Does not invent a live PASS",
        ] {
            assert!(row.contains(needle), "{marker} row missing {needle}: {row}");
        }
    }
assert!(!between.to_ascii_lowercase().contains("kimi"));
assert!(
        !train.contains("READY_FOR_LIVE_TEST: yes") && !train.contains("READY_FOR_LIVE_TEST`: yes")
    );
}
