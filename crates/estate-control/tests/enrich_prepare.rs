//! Train/enrich prepare. Fixtures only. Does not train or rewrite the estate.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn estate_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}
fn tmp(name: &str) -> PathBuf {
    // Keep the throwaway dir off the checkout. A GPU token in the
    // checkout path would trip refuse:sku-banned on the out directory.
    // A pid that contains 5090, 4090, 4080, or 3090 is the same refuse.
    let mut token = std::process::id().to_string();
    for needle in ["5090", "4090", "4080", "3090"] {
        token = token.replace(needle, "0000");
    }
    let path = std::env::temp_dir().join(format!("cell-one-enrich-cli-{name}-{token}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}
fn text(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
fn fixture(rel: &str) -> String {
    repo_root().join(rel).display().to_string()
}
fn estate_bytes() -> String {
    std::fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap()
}
fn write_seated_estate(dir: &std::path::Path, model: &str) -> PathBuf {
    write_train_estate(dir, model, None)
}
fn write_train_estate(dir: &std::path::Path, model: &str, train_base: Option<&str>) -> PathBuf {
    let needle = "  - id: local_slm\n    class: local\n    driver: ollama\n    params:\n";
    let src = estate_bytes();
    assert!(src.contains(needle), "local_slm params block moved");
    let mut insert = format!("{needle}      model: \"{model}\"\n");
    if let Some(train_base) = train_base {
        insert.push_str(&format!("      train_base_model: \"{train_base}\"\n"));
    }
    let seated = src.replacen(needle, &insert, 1);
    let path = dir.join("seated-estate.yaml");
    std::fs::write(&path, seated).unwrap();
    path
}
#[test]
fn help_enrich_and_train_name_the_seam() {
for topic in ["enrich", "train"] {
        let out = estate_bin().args(["help", topic]).output().unwrap();
        let body = text(&out);
        assert!(out.status.success(), "{topic}: {body}");
        assert!(body.contains("TrainEnrichDriver"), "{body}");
        assert!(body.contains("ollama-modelfile"), "{body}");
        assert!(body.contains("external-manifest"), "{body}");
        assert!(body.contains("llamafactory-lora"), "{body}");
        assert!(body.contains("llamafactory-qlora"), "{body}");
        assert!(body.contains("axolotl-lora"), "{body}");
        assert!(body.contains("axolotl-qlora"), "{body}");
        assert!(body.contains("unsloth-qlora"), "{body}");
        assert!(body.contains("mlx-lm-lora"), "{body}");
        assert!(body.contains("MLX.md"), "{body}");
        assert!(body.contains("refuse:host"), "{body}");
        assert!(body.contains("apple-silicon"), "{body}");
        assert!(body.contains("does not write a script"), "{body}");
        assert!(body.contains("Nvidia-only"), "{body}");
        assert!(body.contains("UNSLOTH.md"), "{body}");
        assert!(body.contains("does not require bitsandbytes"), "{body}");
        assert!(body.contains("import-trained"), "{body}");
        assert!(body.contains("make train-prepare"), "{body}");
        assert!(body.contains("make qlora-journey"), "{body}");
        assert!(body.contains("make lora-journey"), "{body}");
        assert!(body.contains("make seat-journey"), "{body}");
        assert!(body.contains("standing next step"), "{body}");
        assert!(body.contains("auto_apply=false"), "{body}");
        assert!(body.contains("did not run ollama create"), "{body}");
        assert!(body.contains("does not apply the estate"), "{body}");
        assert!(
            body.contains("section 10, Target C seat ladder"),
            "{body}"
        );
        assert!(
            body.contains("5090-shaped"),
            "{body}"
        );
        assert!(body.contains("Target C"), "{body}");
        assert!(body.contains("Target A"), "{body}");
        assert!(
            body.contains("section 9, Target A"),
            "{body}"
        );
        assert!(body.contains("Qwen/Qwen2.5-0.5B-Instruct"), "{body}");
        assert!(
            body.contains("examples/fixtures/qwen3-instruct.pack.json"),
            "{body}"
        );
        assert!(
            body.contains("examples/fixtures/qwen25-instruct.pack.json"),
            "{body}"
        );
        assert!(
            body.contains("examples/fixtures/qwen25-instruct-lora.pack.json"),
            "{body}"
        );
        assert!(
            body.contains("examples/fixtures/glm4-chat.pack.json"),
            "{body}"
        );
        assert!(
            body.contains("examples/fixtures/glm4-chat-lora.pack.json"),
            "{body}"
        );
        assert!(body.contains("zai-org/glm-4-9b-chat"), "{body}");
        assert!(
            body.contains("examples/fixtures/deepseek-r1-distill.pack.json"),
            "{body}"
        );
        assert!(
            body.contains("examples/fixtures/deepseek-r1-distill-lora.pack.json"),
            "{body}"
        );
        assert!(
            body.contains("deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B"),
            "{body}"
        );
        assert!(
            body.contains("examples/fixtures/qwen3-instruct-lora.pack.json"),
            "{body}"
        );
        assert!(
            body.contains("examples/fixtures/llama32-instruct-lora.pack.json"),
            "{body}"
        );
        assert!(
            body.contains("examples/fixtures/gemma2-instruct-lora.pack.json"),
            "{body}"
        );
        assert!(
            body.contains("examples/fixtures/mistral-instruct-lora.pack.json"),
            "{body}"
        );
        assert!(
            body.contains("examples/fixtures/phi3-instruct-lora.pack.json"),
            "{body}"
        );
        assert!(body.contains("Qwen/Qwen3-4B-Instruct-2507"), "{body}");
        assert!(body.contains("llamafactory-cli train"), "{body}");
        assert!(body.contains("llamafactory-cli export"), "{body}");
        assert!(body.contains("docs/operator-enrich-journeys.md"), "{body}");
        assert!(
            body.contains(
                "make purpose-build-checklist is the print-only operator path for purpose-building an SLM on demand (operator section 15)"
            ),
            "{body}"
        );
        assert!(
            body.contains(
                "make mlx-lm-lora-journey is the print-only Apple Silicon mlx-lm LoRA journey (operator section 16)"
            ),
            "{body}"
        );
        assert!(
            body.contains(
                "make purpose-build-pick is the print-only host and stack picker for purpose-build journeys (operator section 17)"
            ),
            "{body}"
        );
        assert!(
            body.contains(
                "When an SLM fits mid-software-build, or on demand, make purpose-build-journey is the print-only purpose-build on-demand entry (operator section 18)"
            ),
            "{body}"
        );
        assert!(
            body.contains("make uniqueness-mlx is the print-only chain of that journey"),
            "{body}"
        );
        assert!(body.contains("make uniqueness-prove-checklist"), "{body}");
        assert!(body.contains("make lf-beachhead-prepare"), "{body}");
        assert!(body.contains("docs/lf-beachhead-matrix.md"), "{body}");
        assert!(body.contains("make enrich-prepare"), "{body}");
        assert!(body.contains("make enrich-live-prove"), "{body}");
        assert!(body.contains("estate enrich from-pack"), "{body}");
        assert!(body.contains("refuse:base-model"), "{body}");
        assert!(body.contains("refuse:train-base"), "{body}");
        assert!(body.contains("train_base_model"), "{body}");
        assert!(body.contains("--max-steps"), "{body}");
        assert!(body.contains("--official-scale"), "{body}");
        assert!(body.contains("refuse:official-scale"), "{body}");
        assert!(body.contains("refuse:export"), "{body}");
        assert!(body.contains("params.model"), "{body}");
        assert!(body.contains("docs/TRAIN-ENRICH.md"), "{body}");
        assert!(body.contains("docs/LIVE-PROBES.md"), "{body}");
        assert!(body.contains("--all-drivers"), "{body}");
        assert!(body.contains("estate enrich list"), "{body}");
        assert!(body.contains("import-prepared"), "{body}");
        assert!(body.contains("apply-proposal"), "{body}");
        assert!(body.contains("--verify-local-tag"), "{body}");
        assert!(!body.contains("READY_FOR_LIVE_TEST: yes"), "{body}");
    }
let index = estate_bin().args(["help"]).output().unwrap();
let index_text = text(&index);
assert!(index_text.contains("estate help enrich"), "{index_text}");
assert!(
        index_text.contains("make purpose-build-checklist")
            && index_text.contains(
                "print-only operator path for purpose-building an SLM on demand (operator section 15)"
            )
            && index_text.contains("make mlx-lm-lora-journey")
            && index_text.contains(
                "print-only Apple Silicon mlx-lm LoRA journey (operator section 16)"
            )
            && index_text.contains("make purpose-build-pick")
            && index_text.contains(
                "print-only host and stack picker for purpose-build journeys (operator section 17)"
            )
            && index_text.contains("make purpose-build-journey")
            && index_text.contains(
                "when an SLM fits mid-software-build, or on demand, the same print-only purpose-build on-demand entry (operator section 18)"
            )
            && index_text.contains("make uniqueness-mlx")
            && index_text.contains(
                "print-only chain of that Apple Silicon journey (operator section 16)"
            ),
        "{index_text}"
    );
assert!(index_text.contains("make lora-journey"), "{index_text}");
assert!(index_text.contains("make seat-journey"), "{index_text}");
let drivers = estate_bin().args(["enrich", "drivers"]).output().unwrap();
let listed = text(&drivers);
assert!(drivers.status.success(), "{listed}");
assert!(listed.contains("ollama-modelfile"), "{listed}");
assert!(listed.contains("external-manifest"), "{listed}");
assert!(listed.contains("llamafactory-qlora"), "{listed}");
assert!(listed.contains("llamafactory-lora"), "{listed}");
assert!(listed.contains("unsloth-qlora"), "{listed}");
assert!(listed.contains("mlx-lm-lora"), "{listed}");
assert!(listed.contains("apple-silicon"), "{listed}");
assert!(listed.contains("refuse:host"), "{listed}");
assert!(listed.contains("status=optional"), "{listed}");
assert!(listed.contains("axolotl-lora"), "{listed}");
assert!(listed.contains("axolotl-qlora"), "{listed}");
assert!(listed.contains("live=false"), "{listed}");
assert!(listed.contains("default=train"), "{listed}");
}

#[test]
fn lf_beachhead_matrix_lists_every_smoke_fixture() {
let root = repo_root();
let matrix_rel = "docs/lf-beachhead-matrix.md";
let matrix = std::fs::read_to_string(root.join(matrix_rel)).unwrap();
let help_src = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(
        help_src.contains("include_str!(\"../../../docs/lf-beachhead-matrix.md\")"),
        "estate help must print the matrix file"
    );
assert!(
        matrix.contains("refuse:train-base"),
        "matrix must name refuse:train-base for a bare Ollama seat tag"
    );
assert!(
        matrix.contains("READY_FOR_LIVE_TEST`: no") || matrix.contains("READY_FOR_LIVE_TEST: no"),
        "{matrix}"
    );
assert!(
        !matrix.contains("READY_FOR_LIVE_TEST: yes")
            && !matrix.contains("READY_FOR_LIVE_TEST`: yes"),
        "matrix must keep READY_FOR_LIVE_TEST no"
    );
assert!(
        !matrix.to_ascii_lowercase().contains("kimi/"),
        "matrix must not add a Kimi train base"
    );
let expected = [
        (
            "Phi-3 Instruct",
            "llamafactory-qlora",
            "microsoft/Phi-3-mini-4k-instruct",
            "phi",
            "rank 16, packing true, quantization_method bnb, quantization_bit 4",
            "examples/fixtures/phi3-instruct.pack.json",
        ),
        (
            "Phi-3 Instruct",
            "llamafactory-lora",
            "microsoft/Phi-3-mini-4k-instruct",
            "phi",
            "rank 8, packing false, no quantization_bit, no quantization_method",
            "examples/fixtures/phi3-instruct-lora.pack.json",
        ),
        (
            "Llama-3.2 Instruct",
            "llamafactory-qlora",
            "meta-llama/Llama-3.2-3B-Instruct",
            "llama3",
            "rank 16, packing true, quantization_method bnb, quantization_bit 4",
            "examples/fixtures/llama32-instruct.pack.json",
        ),
        (
            "Llama-3.2 Instruct",
            "llamafactory-lora",
            "meta-llama/Llama-3.2-3B-Instruct",
            "llama3",
            "rank 8, packing false, no quantization_bit, no quantization_method",
            "examples/fixtures/llama32-instruct-lora.pack.json",
        ),
        (
            "Gemma-2 Instruct",
            "llamafactory-qlora",
            "google/gemma-2-2b-it",
            "gemma2",
            "rank 16, packing true, quantization_method bnb, quantization_bit 4",
            "examples/fixtures/gemma2-instruct.pack.json",
        ),
        (
            "Gemma-2 Instruct",
            "llamafactory-lora",
            "google/gemma-2-2b-it",
            "gemma2",
            "rank 8, packing false, no quantization_bit, no quantization_method",
            "examples/fixtures/gemma2-instruct-lora.pack.json",
        ),
        (
            "Mistral Instruct",
            "llamafactory-qlora",
            "mistralai/Mistral-7B-Instruct-v0.3",
            "mistral",
            "rank 16, packing true, quantization_method bnb, quantization_bit 4",
            "examples/fixtures/mistral-instruct.pack.json",
        ),
        (
            "Mistral Instruct",
            "llamafactory-lora",
            "mistralai/Mistral-7B-Instruct-v0.3",
            "mistral",
            "rank 8, packing false, no quantization_bit, no quantization_method",
            "examples/fixtures/mistral-instruct-lora.pack.json",
        ),
        (
            "Qwen2.5 Instruct",
            "llamafactory-qlora",
            "Qwen/Qwen2.5-0.5B-Instruct",
            "qwen",
            "rank 16, packing true, quantization_method bnb, quantization_bit 4",
            "examples/fixtures/qwen25-instruct.pack.json",
        ),
        (
            "Qwen2.5 Instruct",
            "llamafactory-lora",
            "Qwen/Qwen2.5-0.5B-Instruct",
            "qwen",
            "rank 8, packing false, no quantization_bit, no quantization_method",
            "examples/fixtures/qwen25-instruct-lora.pack.json",
        ),
        (
            "Qwen3 Instruct",
            "llamafactory-qlora",
            "Qwen/Qwen3-4B-Instruct-2507",
            "qwen3_nothink",
            "rank 16, packing true, quantization_method bnb, quantization_bit 4",
            "examples/fixtures/qwen3-instruct.pack.json",
        ),
        (
            "Qwen3 Instruct",
            "llamafactory-lora",
            "Qwen/Qwen3-4B-Instruct-2507",
            "qwen3_nothink",
            "rank 8, packing false, no quantization_bit, no quantization_method",
            "examples/fixtures/qwen3-instruct-lora.pack.json",
        ),
        (
            "DeepSeek-R1-Distill chat",
            "llamafactory-qlora",
            "deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B",
            "deepseekr1",
            "rank 16, packing true, quantization_method bnb, quantization_bit 4",
            "examples/fixtures/deepseek-r1-distill.pack.json",
        ),
        (
            "DeepSeek-R1-Distill chat",
            "llamafactory-lora",
            "deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B",
            "deepseekr1",
            "rank 8, packing false, no quantization_bit, no quantization_method",
            "examples/fixtures/deepseek-r1-distill-lora.pack.json",
        ),
        (
            "GLM-4 Chat",
            "llamafactory-qlora",
            "zai-org/glm-4-9b-chat",
            "glm4",
            "rank 16, packing true, quantization_method bnb, quantization_bit 4",
            "examples/fixtures/glm4-chat.pack.json",
        ),
        (
            "GLM-4 Chat",
            "llamafactory-lora",
            "zai-org/glm-4-9b-chat",
            "glm4",
            "rank 8, packing false, no quantization_bit, no quantization_method",
            "examples/fixtures/glm4-chat-lora.pack.json",
        ),
    ];
let parsed = beachhead_matrix_rows(&matrix);
assert_eq!(
        parsed.len(),
        expected.len(),
        "matrix row count drifted from the beachhead inventory"
    );
let mut seen = Vec::new();
for (row, expect) in parsed.iter().zip(expected) {
        let (family, card, train_base, template, knobs, fixture) = expect;
        assert_eq!(row[0], family, "family drift");
        assert_eq!(row[1], card, "card drift");
        assert_eq!(row[2], train_base, "train base drift");
        assert_eq!(row[3], template, "template drift");
        assert_eq!(row[4], knobs, "knobs drift");
        assert_eq!(row[5], fixture, "fixture drift");
        let path = root.join(fixture);
        assert!(path.is_file(), "missing beachhead fixture {fixture}");
        let pack: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(pack["train_base_model"], train_base, "{fixture}");
        assert_eq!(pack["model_hint"], "llama3", "{fixture}");
        seen.push(fixture.to_string());
    }
let mut mentioned = fixture_paths_in(&matrix);
mentioned.sort();
seen.sort();
assert_eq!(
        mentioned, seen,
        "matrix fixture paths must be exactly the beachhead rows"
    );
for rel in ["docs/TRAIN-ENRICH.md", "docs/operator-enrich-journeys.md"] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            body.contains("lf-beachhead-matrix.md"),
            "{rel} must point at the beachhead matrix"
        );
        assert!(
            body.contains("make lf-beachhead-prepare"),
            "{rel} must name the beachhead prepare walk"
        );
        assert!(
            !body.contains("READY_FOR_LIVE_TEST: yes"),
            "{rel} must keep READY_FOR_LIVE_TEST no"
        );
    }
for topic in ["enrich", "train"] {
        let out = estate_bin().args(["help", topic]).output().unwrap();
        let body = text(&out);
        assert!(out.status.success(), "{topic}: {body}");
        assert!(
            body.contains("# LLaMA-Factory LoRA and QLoRA beachhead matrix"),
            "{topic} help must print the matrix"
        );
        assert!(body.contains("docs/lf-beachhead-matrix.md"), "{body}");
        for fixture in &seen {
            assert!(body.contains(fixture), "{topic} help missing {fixture}");
        }
        let matrix_at = body
            .find("# LLaMA-Factory LoRA and QLoRA beachhead matrix")
            .unwrap();
        let pointer_at = body
            .find("The table above is docs/lf-beachhead-matrix.md")
            .expect("help pointer");
        assert!(
            matrix_at < pointer_at,
            "help must print the matrix before the path pointer"
        );
        assert!(
            body.contains("make lf-beachhead-prepare"),
            "{topic} help must name the beachhead prepare walk"
        );
        assert!(!body.contains("READY_FOR_LIVE_TEST: yes"), "{body}");
    }
}

#[test]
fn lf_beachhead_prepare_walks_the_matrix_inventory() {
let root = repo_root();
let matrix = std::fs::read_to_string(root.join("docs/lf-beachhead-matrix.md")).unwrap();
let parsed = beachhead_matrix_rows(&matrix);
assert!(
        !parsed.is_empty(),
        "matrix inventory is empty; the prepare walk would check nothing"
    );
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        makefile.lines().any(|line| line.trim() == "lf-beachhead-prepare:"),
        "Makefile missing lf-beachhead-prepare"
    );
assert!(makefile.contains("scripts/lf-beachhead-prepare.sh"));
assert!(
        makefile.contains("Do not add to smoke, gate-90, or GitHub Actions"),
        "lf-beachhead-prepare must stay off smoke, gate-90, and Actions"
    );
let phony = makefile.lines().next().unwrap_or("");
assert!(
        phony.contains("lf-beachhead-prepare"),
        "lf-beachhead-prepare must be a phony target"
    );
let script = std::fs::read_to_string(root.join("scripts/lf-beachhead-prepare.sh")).unwrap();
for needle in [
        "docs/lf-beachhead-matrix.md",
        "estate enrich prepare",
        "SKIP live train",
        "READY_FOR_LIVE_TEST: no",
        "examples/estate.yaml",
        "cksum",
        "llamafactory-qlora",
        "llamafactory-lora",
        "phi_small",
        "microsoft/Phi-3-small-8k-instruct",
        "QLoRA-only",
        "not a matrix row",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "--list",
    ] {
        assert!(script.contains(needle), "lf-beachhead-prepare missing {needle}");
    }
assert!(
        !script.contains("READY_FOR_LIVE_TEST: yes"),
        "lf-beachhead-prepare must keep READY_FOR_LIVE_TEST no"
    );
let prepares = script.lines().any(|line| {
        let trimmed = line.trim_start();
        !trimmed.starts_with('#') && trimmed.contains("enrich") && trimmed.contains("prepare")
    }
);
assert!(prepares, "walk must call estate enrich prepare");
let shells_out = script.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#')
            || trimmed.starts_with("echo")
            || trimmed.starts_with("grep")
            || trimmed.starts_with("if grep")
            || trimmed.starts_with("if ! grep")
            || trimmed.contains("[[ -e")
        {
            return false;
        }
        trimmed.contains("llamafactory-cli")
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
        "lf-beachhead-prepare must not train, merge, convert, seat, or import"
    );
let listed = Command::new("bash")
        .arg(root.join("scripts/lf-beachhead-prepare.sh"))
        .arg("--list")
        .output()
        .unwrap();
let listed_text = text(&listed);
assert!(listed.status.success(), "{listed_text}");
let walked: Vec<Vec<String>> = listed_text
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.split('\t').map(|cell| cell.to_string()).collect())
        .collect();
assert_eq!(
        walked, parsed,
        "prepare walk must list the same rows as docs/lf-beachhead-matrix.md"
    );
for row in &walked {
        assert!(
            !row[2].to_ascii_lowercase().contains("phi-3-small") && !row[5].contains("phi3-small"),
            "Phi-3-small must stay off the matrix inventory: {row:?}"
        );
        assert!(root.join(&row[5]).is_file(), "missing {}", row[5]);
    }
for rel in [
        "docs/lf-beachhead-matrix.md",
        "docs/TRAIN-ENRICH.md",
        "docs/operator-enrich-journeys.md",
        "docs/CELL-ONE-STATUS.md",
        "crates/estate-control/src/help.rs",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            body.contains("make lf-beachhead-prepare"),
            "{rel} must name the prepare walk"
        );
        assert!(
            !body.contains("READY_FOR_LIVE_TEST: yes"),
            "{rel} must keep READY_FOR_LIVE_TEST no"
        );
    }
for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("lf-beachhead-prepare"),
            "{rel} must not run lf-beachhead-prepare"
        );
    }
}

fn beachhead_matrix_rows(matrix: &str) -> Vec<Vec<String>> {
    let mut parsed = Vec::new();
    for line in matrix.lines() {
        if !line.starts_with("| ") || line.contains("---") || line.contains("Family |") {
            continue;
        }
        let cells: Vec<String> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().trim_matches('`').to_string())
            .collect();
        assert_eq!(cells.len(), 6, "matrix row must keep six columns: {line}");
        parsed.push(cells);
    }
    parsed
}
fn fixture_paths_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("examples/fixtures/") {
        let tail = &rest[start..];
        let end = tail
            .find(|c: char| {
                !(c.is_ascii_alphanumeric() || c == '/' || c == '.' || c == '-' || c == '_')
            })
            .unwrap_or(tail.len());
        let path = &tail[..end];
        if path.ends_with(".pack.json") {
            out.push(path.to_string());
        }
        rest = &tail[end..];
    }
    out
}
#[test]
fn prepare_both_drivers_and_refuses_without_writing() {
let root = tmp("cli");
let estate = fixture("examples/estate.yaml");
let seated = write_seated_estate(&root, "llama3");
let seated_path = seated.display().to_string();
let sacred = fixture("policy/sacred.yaml");
let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
let before = estate_bytes();
let stock_out = root.join("stock");
let stock = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            &pack,
            "--driver",
            "ollama-modelfile",
            "--out",
            &stock_out.display().to_string(),
        ])
        .output()
        .unwrap();
let stock_text = text(&stock);
assert!(!stock.status.success(), "{stock_text}");
assert!(stock_text.contains("refuse:base-model"), "{stock_text}");
assert!(
        !stock_out.exists(),
        "stock estate must not write FROM local_slm"
    );
let ollama_out = root.join("ollama");
let ollama = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated_path,
            "--pack",
            &pack,
            "--driver",
            "ollama-modelfile",
            "--out",
            &ollama_out.display().to_string(),
        ])
        .output()
        .unwrap();
let ollama_text = text(&ollama);
assert!(ollama.status.success(), "{ollama_text}");
assert!(ollama_text.contains("promoted=false"), "{ollama_text}");
assert!(
        ollama_text.contains("estate_rewritten=false"),
        "{ollama_text}"
    );
let modelfile = std::fs::read_to_string(ollama_out.join("Modelfile")).unwrap();
assert!(modelfile.contains("FROM llama3\n"), "{modelfile}");
assert!(!modelfile.contains("FROM local_slm"), "{modelfile}");
assert!(ollama_text.contains("base=llama3"), "{ollama_text}");
let steps = std::fs::read_to_string(ollama_out.join("PREPARE.md")).unwrap();
assert!(
        steps.contains("ollama create cell-enrich-overnight-traces -f Modelfile"),
        "{steps}"
    );
let next = std::fs::read_to_string(ollama_out.join("NEXT.md")).unwrap();
assert!(
        next.contains(&format!(
            "ollama create cell-enrich-overnight-traces -f {}",
            ollama_out.join("Modelfile").display()
        )),
        "{next}"
    );
assert!(ollama_text.contains("import-prepared"), "{ollama_text}");
assert!(ollama_text.contains("prepared=1"), "{ollama_text}");
let manifest_out = root.join("manifest");
let manifest = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated_path,
            "--pack",
            &pack,
            "--driver",
            "external-manifest",
            "--job",
            "train",
            "--out",
            &manifest_out.display().to_string(),
        ])
        .output()
        .unwrap();
let manifest_text = text(&manifest);
assert!(manifest.status.success(), "{manifest_text}");
assert!(manifest_text.contains("job=train"), "{manifest_text}");
assert!(manifest_out.join("manifest.json").is_file());
assert!(manifest_out.join("manifest.yaml").is_file());
let body = std::fs::read_to_string(manifest_out.join("manifest.json")).unwrap();
assert!(body.contains("\"vendor\": null"), "{body}");
assert!(!body.to_ascii_lowercase().contains("ollama"), "{body}");
assert_eq!(
        estate_bytes(),
        before,
        "prepare rewrote examples/estate.yaml"
    );
let missing_out = root.join("missing");
let missing = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            "no-such-pack",
            "--packs-dir",
            &root.join("empty-packs").display().to_string(),
            "--out",
            &missing_out.display().to_string(),
        ])
        .output()
        .unwrap();
let missing_text = text(&missing);
assert!(!missing.status.success(), "{missing_text}");
assert!(
        missing_text.contains("refuse:missing-pack"),
        "{missing_text}"
    );
assert!(!missing_out.exists());
let sacred_pack = root.join("sacred.pack.json");
let mut sacred_doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&pack).unwrap()).unwrap();
sacred_doc["note"] = serde_json::Value::String("please mention cyera".into());
std::fs::write(
        &sacred_pack,
        serde_json::to_string_pretty(&sacred_doc).unwrap(),
    )
    .unwrap();
let sacred_out = root.join("sacred-out");
let sacred_run = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated_path,
            "--pack",
            &sacred_pack.display().to_string(),
            "--out",
            &sacred_out.display().to_string(),
        ])
        .output()
        .unwrap();
let sacred_text = text(&sacred_run);
assert!(!sacred_run.status.success(), "{sacred_text}");
assert!(sacred_text.contains("refuse:sacred"), "{sacred_text}");
assert!(!sacred_out.exists());
let sku_pack = root.join("sku.pack.json");
let mut sku_doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&pack).unwrap()).unwrap();
sku_doc["model_hint"] = serde_json::Value::String("rtx-5090".into());
std::fs::write(&sku_pack, serde_json::to_string_pretty(&sku_doc).unwrap()).unwrap();
let sku_out = root.join("sku-out");
let sku_run = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            &sku_pack.display().to_string(),
            "--out",
            &sku_out.display().to_string(),
        ])
        .output()
        .unwrap();
let sku_text = text(&sku_run);
assert!(!sku_run.status.success(), "{sku_text}");
assert!(sku_text.to_ascii_lowercase().contains("sku"), "{sku_text}");
assert!(!sku_out.exists());
let curator = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            &pack,
            "--curator",
            "ada",
            "--out",
            &root.join("curator-out").display().to_string(),
        ])
        .output()
        .unwrap();
let curator_text = text(&curator);
assert!(!curator.status.success(), "{curator_text}");
assert!(curator_text.contains("refuse:curator"), "{curator_text}");
assert!(!root.join("curator-out").exists());
let local_estate = root.join("local-only.yaml");
std::fs::write(
        &local_estate,
        "version: 0\nname: local-only\ndefault_effect: deny\nagents:\n  - id: horizon\n    display_name: Horizon\n    lane: horizon\n    desktop: horizon-desktop\nlanes:\n  - id: horizon\n    root_path: lanes/horizon\n    owner_agent_id: horizon\nmodel_bindings:\n  - id: local_slm\n    class: local\n    driver: ollama\n    wired: true\nenrich_packs:\n  curator: jason\n  policy: manual\n",
    )
    .unwrap();
let frontier_pack = root.join("frontier.pack.json");
let mut frontier_doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&pack).unwrap()).unwrap();
frontier_doc["source_drivers"] = serde_json::json!(["frontier"]);
frontier_doc["path_counts"]["frontier"] = serde_json::json!(1);
std::fs::write(
        &frontier_pack,
        serde_json::to_string_pretty(&frontier_doc).unwrap(),
    )
    .unwrap();
let frontier_out = root.join("frontier-out");
let frontier = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &local_estate.display().to_string(),
            "--pack",
            &frontier_pack.display().to_string(),
            "--out",
            &frontier_out.display().to_string(),
        ])
        .output()
        .unwrap();
let frontier_text = text(&frontier);
assert!(!frontier.status.success(), "{frontier_text}");
assert!(
        frontier_text.contains("refuse:frontier-invent"),
        "{frontier_text}"
    );
assert!(!frontier_out.exists());
assert_eq!(estate_bytes(), before);
}

#[test]
fn enrich_prepare_stays_off_smoke_and_dispatch_does_not_match_drivers() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        makefile.contains("enrich-prepare:"),
        "Makefile missing enrich-prepare"
    );
assert!(
        makefile.contains("enrich-live-prove:"),
        "Makefile missing enrich-live-prove"
    );
assert!(makefile.contains("scripts/enrich-prepare.sh"));
assert!(makefile.contains("scripts/enrich-live-prove.sh"));
let script = std::fs::read_to_string(root.join("scripts/enrich-prepare.sh")).unwrap();
assert!(script.contains("ollama-modelfile"), "{script}");
assert!(script.contains("external-manifest"), "{script}");
assert!(script.contains("--all-drivers"), "{script}");
assert!(script.contains("import-prepared"), "{script}");
assert!(script.contains("Do not add to make smoke or GitHub Actions"));
assert!(script.contains("FROM llama3"));
assert!(script.contains("refuse:base-model"));
let live = std::fs::read_to_string(root.join("scripts/enrich-live-prove.sh")).unwrap();
assert!(live.contains("ollama create"), "{live}");
assert!(live.contains("from-pack"), "{live}");
assert!(live.contains("import-prepared"), "{live}");
assert!(live.contains("ollama show"), "{live}");
assert!(live.contains("not a factory-wide live test"), "{live}");
assert!(live.contains("Do not add to make smoke"), "{live}");
assert!(!live.contains("READY_FOR_LIVE_TEST: yes"), "{live}");
for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("enrich-prepare"),
            "{rel} must not run enrich-prepare"
        );
        assert!(
            !body.contains("enrich-live-prove"),
            "{rel} must not run enrich-live-prove"
        );
    }
let dispatch =
        std::fs::read_to_string(root.join("crates/estate-control/src/dispatch.rs")).unwrap();
assert!(!dispatch.contains("ollama-modelfile"));
assert!(!dispatch.contains("external-manifest"));
assert!(!dispatch.contains("axolotl-lora"));
assert!(!dispatch.contains("axolotl-qlora"));
assert!(!dispatch.contains("llamafactory-qlora"));
assert!(!dispatch.contains("llamafactory-lora"));
assert!(!dispatch.contains("unsloth-qlora"));
assert!(!dispatch.contains("unsloth-lora"));
assert!(!dispatch.contains("mlx-lm-lora"));
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        makefile.contains("train-prepare:"),
        "Makefile missing train-prepare"
    );
assert!(makefile.contains("scripts/train-prepare.sh"));
let train_script = std::fs::read_to_string(root.join("scripts/train-prepare.sh")).unwrap();
assert!(
        train_script.contains("llamafactory-qlora"),
        "{train_script}"
    );
assert!(train_script.contains("llamafactory-lora"), "{train_script}");
assert!(train_script.contains("qwen3_nothink"), "{train_script}");
assert!(
        train_script.contains("examples/fixtures/llama32-instruct.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains("meta-llama/Llama-3.2-3B-Instruct"),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/gemma2-instruct.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/gemma2-instruct-lora.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains(
            "Reproduce target on the unquantized LoRA card, the non-quant twin of the Gemma-2 Instruct QLoRA prepare."
        ),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/phi3-instruct-lora.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains(
            "Reproduce target on the unquantized LoRA card, the non-quant twin of the Phi-3 Instruct QLoRA prepare."
        ),
        "{train_script}"
    );
assert!(
        train_script.contains("^template: phi$"),
        "{train_script}"
    );
assert!(
        train_script.contains("google/gemma-2-2b-it"),
        "{train_script}"
    );
assert!(
        train_script.contains("^template: gemma2$"),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/mistral-instruct.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/mistral-instruct-lora.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains("mistralai/Mistral-7B-Instruct-v0.3"),
        "{train_script}"
    );
assert!(
        train_script.contains("^template: mistral$"),
        "{train_script}"
    );
assert!(
        train_script.contains(
            "Reproduce target on the unquantized LoRA card, the non-quant twin of the Mistral Instruct QLoRA prepare."
        ),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/qwen3-instruct.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/qwen25-instruct.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/glm4-chat.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains("zai-org/glm-4-9b-chat"),
        "{train_script}"
    );
assert!(
        train_script.contains("^template: glm4$"),
        "{train_script}"
    );
assert!(
        train_script.contains(
            "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, Qwen3 Instruct, and DeepSeek-R1-Distill chat QLoRA."
        ),
        "{train_script}"
    );
assert!(
        train_script.contains("glm4 fixture on the LoRA card wrote the QLoRA reproduce note"),
        "{train_script}"
    );
assert!(
        train_script.contains("glm4 prepare took the DeepSeek-R1-Distill QLoRA reproduce note"),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/glm4-chat-lora.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains(
            "Reproduce target on the unquantized LoRA card, the non-quant twin of the GLM-4 Chat QLoRA prepare."
        ),
        "{train_script}"
    );
assert!(
        train_script.contains("glm4 QLoRA prepare took the GLM-4 Chat LoRA reproduce note"),
        "{train_script}"
    );
assert!(
        train_script.contains("glm4 LoRA fixture prepare wrote the QLoRA reproduce note"),
        "{train_script}"
    );
assert!(
        train_script.contains("glm4 LoRA fixture on the QLoRA card wrote the LoRA reproduce note"),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/deepseek-r1-distill.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains("deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B"),
        "{train_script}"
    );
assert!(
        train_script.contains("^template: deepseekr1$"),
        "{train_script}"
    );
assert!(
        train_script.contains(
            "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, and Qwen3 Instruct QLoRA."
        ),
        "{train_script}"
    );
assert!(
        train_script.contains("deepseek fixture on the LoRA card wrote the QLoRA reproduce note"),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/deepseek-r1-distill-lora.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains(
            "Reproduce target on the unquantized LoRA card, the non-quant twin of the DeepSeek-R1-Distill chat QLoRA prepare."
        ),
        "{train_script}"
    );
assert!(
        train_script.contains("deepseek QLoRA prepare took the DeepSeek-R1-Distill LoRA reproduce note"),
        "{train_script}"
    );
assert!(
        train_script.contains("deepseek LoRA fixture prepare wrote the QLoRA reproduce note"),
        "{train_script}"
    );
assert!(
        train_script.contains(
            "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct QLoRA."
        ),
        "{train_script}"
    );
assert!(
        train_script.contains("qwen2.5 LoRA prepare took the Qwen2.5 Instruct QLoRA reproduce note"),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/qwen25-instruct-lora.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains(
            "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen2.5 Instruct QLoRA prepare."
        ),
        "{train_script}"
    );
assert!(
        train_script.contains("qwen2.5 QLoRA prepare took the Qwen2.5 Instruct LoRA reproduce note"),
        "{train_script}"
    );
assert!(
        train_script.contains("Qwen/Qwen3-4B-Instruct-2507"),
        "{train_script}"
    );
assert!(
        train_script.contains("^template: qwen3_nothink$"),
        "{train_script}"
    );
assert!(
        train_script.contains(
            "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x LoRA/QLoRA."
        ),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/qwen3-instruct-lora.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains(
            "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen3 Instruct QLoRA prepare."
        ),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/fixtures/llama32-instruct-lora.pack.json"),
        "{train_script}"
    );
assert!(
        train_script.contains(
            "Reproduce target on the unquantized LoRA card, the non-quant twin of the Llama-3.2 Instruct QLoRA prepare."
        ),
        "{train_script}"
    );
assert!(
        train_script.contains("^template: llama3$"),
        "{train_script}"
    );
assert!(
        train_script.contains("does not require bitsandbytes"),
        "{train_script}"
    );
assert!(
        train_script.contains("llamafactory-cli train"),
        "{train_script}"
    );
assert!(train_script.contains("axolotl-lora"), "{train_script}");
assert!(train_script.contains("axolotl-qlora"), "{train_script}");
assert!(train_script.contains("unsloth-qlora"), "{train_script}");
assert!(train_script.contains("UNSLOTH.md"), "{train_script}");
assert!(train_script.contains("mlx-lm-lora"), "{train_script}");
assert!(train_script.contains("MLX.md"), "{train_script}");
assert!(train_script.contains("refuse:host"), "{train_script}");
assert!(
        train_script.contains("does not call Unsloth"),
        "{train_script}"
    );
assert!(train_script.contains("axolotl train"), "{train_script}");
assert!(
        train_script.contains("examples/llama-3/lora-1b.yml"),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/llama-3/qlora.yml"),
        "{train_script}"
    );
assert!(train_script.contains("SKIP live train"), "{train_script}");
assert!(
        train_script.contains("job") && train_script.contains("train"),
        "{train_script}"
    );
assert!(
        train_script.contains("examples/estate.yaml") || train_script.contains("\"$ESTATE\""),
        "{train_script}"
    );
assert!(
        train_script.contains("Do not add to make smoke"),
        "{train_script}"
    );
for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("train-prepare"),
            "{rel} must not run train-prepare"
        );
    }
let floor = std::fs::read_to_string(root.join("crates/floor-supervisor/src/lib.rs")).unwrap();
assert!(!floor.contains("TrainEnrichDriver"));
}

#[test]
fn lora_journey_script_locks_the_opt_in_ladder_and_stays_off_smoke() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        makefile.lines().any(|line| line.trim() == "lora-journey:"),
        "Makefile missing lora-journey"
    );
assert!(makefile.contains("scripts/lora-journey.sh"));
assert!(
        makefile.contains("Do not add to smoke, gate-90, or GitHub Actions"),
        "lora-journey must stay off smoke, gate-90, and Actions"
    );
let script = std::fs::read_to_string(root.join("scripts/lora-journey.sh")).unwrap();
for needle in [
        "Target A",
        "llamafactory-lora",
        "Qwen/Qwen2.5-0.5B-Instruct",
        "llama3",
        "template qwen",
        "refuse:train-base",
        "refuse:seat",
        "SKIP live train",
        "READY_FOR_LIVE_TEST: no",
        "llamafactory-cli train",
        "llamafactory-cli export",
        "estate enrich merge-adapt",
        "gguf-convert",
        "local-seat",
        "import-trained",
        "python3 convert_hf_to_gguf.py",
        "--outtype auto",
        "does not require bitsandbytes",
        "lora_rank: 8",
        "packing: false",
        "examples/estate.yaml",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "trained_shape",
    ] {
        assert!(script.contains(needle), "lora-journey missing {needle}");
    }
assert!(
        !script.contains("READY_FOR_LIVE_TEST: yes"),
        "lora-journey must keep READY_FOR_LIVE_TEST no"
    );
assert!(
        script.contains("bitsandbytes>=0.49"),
        "lora-journey must refuse a bitsandbytes install line"
    );
let shells_out = script.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#')
            || trimmed.starts_with("echo")
            || trimmed.starts_with("grep")
            || trimmed.starts_with("if grep")
            || trimmed.starts_with("if ! grep")
            || trimmed.contains("[[ -e")
        {
            return false;
        }
        trimmed.contains("llamafactory-cli")
            || trimmed.contains("convert_hf_to_gguf.py")
            || trimmed.contains("ollama ")
    }
);
assert!(
        !shells_out,
        "lora-journey must not shell out to llamafactory-cli, llama.cpp, or ollama"
    );
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
assert!(journey.contains("## 9. Target A — Qwen / LLaMA-Factory LoRA to the local seat"));
assert!(journey.contains("make lora-journey"));
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
assert!(train.contains("## Target A — Qwen LoRA operator journey"));
assert!(train.contains("make lora-journey"));
for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        let scrubbed = body.replace("qlora-journey", "");
        assert!(
            !scrubbed.contains("lora-journey"),
            "{rel} must not run lora-journey"
        );
    }
}

#[test]
fn seat_journey_script_locks_the_opt_in_ladder_and_stays_off_smoke() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        makefile.lines().any(|line| line.trim() == "seat-journey:"),
        "Makefile missing seat-journey"
    );
assert!(makefile.contains("scripts/seat-journey.sh"));
assert!(
        makefile.contains("Do not add to smoke, gate-90, or GitHub Actions"),
        "seat-journey must stay off smoke, gate-90, and Actions"
    );
let script = std::fs::read_to_string(root.join("scripts/seat-journey.sh")).unwrap();
for needle in [
        "Target C",
        "llamafactory-qlora",
        "Qwen/Qwen2.5-0.5B-Instruct",
        "llama3",
        "refuse:train-base",
        "refuse:adapter",
        "refuse:seat",
        "refuse:tokenizer",
        "5090-shaped",
        "extra_special_tokens",
        "tokenizer_config.json",
        "tokenizer_config.json.bak",
        "HF cache snapshot",
        "equivalent base checkout",
        "into the export directory",
        "HF hub snapshots are often symlinks",
        "cp -aL",
        "cp --dereference",
        "real files, not symlinks",
        "plain cp -a",
        "does not follow a symlinked tokenizer_config.json",
        "re-run estate enrich gguf-convert",
        "missing vocab.json",
        "missing merges.txt",
        "model_type",
        "Qwen2ForCausalLM",
        "Tokenizer check passed",
        "this directory has no tokenizer_config.json",
        "SKIP live train",
        "SKIP live convert",
        "SKIP live seat",
        "READY_FOR_LIVE_TEST: no",
        "llamafactory-cli export",
        "merge-adapt",
        "gguf-convert",
        "local-seat",
        "import-trained",
        "standing next step",
        "auto_apply=false",
        "did not run ollama create",
        "does not apply the estate",
        "python3 convert_hf_to_gguf.py",
        "--outtype auto",
        "ollama create",
        "trained_shape",
        "adapter_config.json",
        "config.json",
        "model.safetensors",
        "GGUF",
        "CELL_SEAT_LIVE",
        "quantization_bit: 4",
        "quantization_method: bnb",
        "examples/estate.yaml",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
    ] {
        assert!(script.contains(needle), "seat-journey missing {needle}");
    }
let bad = script
        .find("-- 5090-shaped export tokenizer is refuse:tokenizer --")
        .expect("seat-journey missing the refuse:tokenizer step");
let replace = script
        .find("-- replace the broken tokenizer with the good merged stub --")
        .expect("seat-journey missing the good-stub replace");
let happy = script
        .find("-- gguf-convert prints convert_hf_to_gguf.py --")
        .expect("seat-journey missing the happy-path convert");
assert!(
        bad < replace && replace < happy,
        "refuse:tokenizer must run before the good stubs and the convert print"
    );
assert!(
        !script.contains("READY_FOR_LIVE_TEST: yes"),
        "seat-journey must keep READY_FOR_LIVE_TEST no"
    );
let shells_out = script.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#')
            || trimmed.starts_with("echo")
            || trimmed.starts_with("grep")
            || trimmed.starts_with("if grep")
            || trimmed.starts_with("if ! grep")
            || trimmed.contains("[[ -e")
        {
            return false;
        }
        trimmed.contains("llamafactory-cli")
            || trimmed.contains("convert_hf_to_gguf.py")
            || trimmed.contains("ollama ")
    }
);
assert!(
        !shells_out,
        "seat-journey must not shell out to llamafactory-cli, llama.cpp, or ollama"
    );
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
assert!(journey.contains(
        "## 10. Target C seat ladder — fixture stubs print the merge, convert, seat, and import"
    ));
assert!(journey.contains("make seat-journey"));
let section_10 = journey
        .split("## 10. Target C seat ladder")
        .nth(1)
        .expect("section 10");
assert!(
        section_10.contains("5090-shaped") && section_10.contains("refuse:tokenizer"),
        "section 10 must name the refuse:tokenizer fixture"
    );
assert!(
        section_10.contains("HF cache snapshot")
            && section_10.contains("equivalent base checkout")
            && section_10.contains("into the export directory")
            && section_10.contains("HF hub snapshots are often symlinks")
            && section_10.contains("cp -aL")
            && section_10.contains("cp --dereference")
            && section_10.contains("real files, not symlinks")
            && section_10.contains("plain `cp -a`")
            && section_10.contains("does not follow a symlinked")
            && section_10.contains("Then re-run `estate enrich gguf-convert`.")
            && section_10.contains("The refuse does not print `python3 convert_hf_to_gguf.py`."),
        "section 10 must name the HF cache restore, the dereference copy, and the gguf-convert re-run"
    );
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
assert!(train.contains("## Target C seat ladder — fixture print path"));
assert!(train.contains("make seat-journey"));
assert!(
        train.contains("5090-shaped"),
        "TRAIN-ENRICH must name the refuse:tokenizer fixture"
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
fn prepare_all_list_and_import_prepared_stay_off_the_estate() {
let root = tmp("loop");
let estate = fixture("examples/estate.yaml");
let seated = write_seated_estate(&root, "llama3");
let seated_path = seated.display().to_string();
let sacred = fixture("policy/sacred.yaml");
let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
let before = estate_bytes();
let state = root.join("state");
let both = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &estate,
            "--pack",
            &pack,
            "--all-drivers",
            "--driver",
            "ollama-modelfile",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
let both_text = text(&both);
assert!(!both.status.success(), "{both_text}");
assert!(both_text.contains("refuse:driver"), "{both_text}");
assert!(!state.join("enrich").exists());
let prepared = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated_path,
            "--pack",
            &pack,
            "--all-drivers",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
let prepared_text = text(&prepared);
assert!(prepared.status.success(), "{prepared_text}");
assert!(prepared_text.contains("prepared=2"), "{prepared_text}");
assert!(
        prepared_text.contains("driver=ollama-modelfile"),
        "{prepared_text}"
    );
assert!(
        prepared_text.contains("driver=external-manifest"),
        "{prepared_text}"
    );
let ollama = state.join("enrich/overnight-traces/ollama-modelfile");
let manifest = state.join("enrich/overnight-traces/external-manifest");
assert!(ollama.join("Modelfile").is_file());
let both_modelfile = std::fs::read_to_string(ollama.join("Modelfile")).unwrap();
assert!(both_modelfile.contains("FROM llama3\n"), "{both_modelfile}");
assert!(
        !both_modelfile.contains("FROM local_slm"),
        "{both_modelfile}"
    );
assert!(ollama.join("NEXT.md").is_file());
assert!(manifest.join("manifest.json").is_file());
let next = std::fs::read_to_string(manifest.join("NEXT.md")).unwrap();
assert!(!next.contains("ollama create"), "{next}");
let listed = estate_bin()
        .args([
            "enrich",
            "list",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
let listed_text = text(&listed);
assert!(listed.status.success(), "{listed_text}");
assert!(listed_text.contains("count=2"), "{listed_text}");
assert!(
        listed_text.contains("tag=cell-enrich-overnight-traces"),
        "{listed_text}"
    );
assert!(listed_text.contains("promoted=false"), "{listed_text}");
let missing = estate_bin()
        .args([
            "enrich",
            "list",
            "--state-dir",
            &root.join("absent").display().to_string(),
        ])
        .output()
        .unwrap();
let missing_text = text(&missing);
assert!(!missing.status.success(), "{missing_text}");
assert!(
        missing_text.contains("refuse:enrich-index"),
        "{missing_text}"
    );
assert!(!root.join("absent").exists());
let tag = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-prepared",
            "--estate",
            &estate,
            "--prepared",
            &ollama.display().to_string(),
            "--tag",
            "other-tag",
            "--path",
            &ollama.join("Modelfile").display().to_string(),
        ])
        .output()
        .unwrap();
let tag_text = text(&tag);
assert!(!tag.status.success(), "{tag_text}");
assert!(tag_text.contains("refuse:tag"), "{tag_text}");
assert!(!ollama.join("binding-proposal.json").exists());
let imported = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-prepared",
            "--estate",
            &seated_path,
            "--prepared",
            &ollama.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--path",
            &ollama.join("Modelfile").display().to_string(),
        ])
        .output()
        .unwrap();
let imported_text = text(&imported);
assert!(imported.status.success(), "{imported_text}");
assert!(
        imported_text.contains("binding=local_slm"),
        "{imported_text}"
    );
assert!(
        imported_text.contains("auto_apply=false"),
        "{imported_text}"
    );
assert!(imported_text.contains("did not apply"), "{imported_text}");
let proposal: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(ollama.join("binding-proposal.json")).unwrap(),
    )
    .unwrap();
assert_eq!(proposal["schema"], "cell-one.enrich-binding-proposal.v0");
assert_eq!(proposal["auto_apply"], false);
assert_eq!(proposal["promoted"], false);
assert_eq!(proposal["estate_rewritten"], false);
assert_eq!(
        proposal["proposed_binding"]["params"]["model"],
        "cell-enrich-overnight-traces"
    );
assert_eq!(proposal["proposed_binding"]["id"], "local_slm");
assert!(!ollama.join("catalog.json").exists());
assert_eq!(estate_bytes(), before, "import-prepared rewrote the estate");
}

#[test]
fn apply_proposal_then_plan_and_require_plan_writes_only_the_lab_estate() {
let root = tmp("apply-proposal");
let sacred = fixture("policy/sacred.yaml");
let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
let example = repo_root().join("examples/estate.yaml");
let example_before = std::fs::read(&example).unwrap();
let lab = write_seated_estate(&root, "llama3");
let lab_before = std::fs::read(&lab).unwrap();
let state = root.join("state");
let plans = root.join("plans");
let prepared = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &lab.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "ollama-modelfile",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
assert!(prepared.status.success(), "{}", text(&prepared));
let dir = state.join("enrich/overnight-traces/ollama-modelfile");
let modelfile = std::fs::read_to_string(dir.join("Modelfile")).unwrap();
assert!(modelfile.contains("FROM llama3\n"), "{modelfile}");
assert!(!modelfile.contains("FROM local_slm"), "{modelfile}");
let imported = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-prepared",
            "--estate",
            &lab.display().to_string(),
            "--prepared",
            &dir.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--path",
            &dir.join("Modelfile").display().to_string(),
        ])
        .output()
        .unwrap();
assert!(imported.status.success(), "{}", text(&imported));
assert!(
        text(&imported).contains("apply-proposal"),
        "{}",
        text(&imported)
    );
let verify = estate_bin()
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_RENTED_ENDPOINT")
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "apply-proposal",
            "--estate",
            &lab.display().to_string(),
            "--prepared",
            &dir.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--state-dir",
            &root.join("verify-state").display().to_string(),
            "--verify-local-tag",
        ])
        .output()
        .unwrap();
let verify_text = text(&verify);
assert!(!verify.status.success(), "{verify_text}");
assert!(verify_text.contains("refuse:local-tag"), "{verify_text}");
assert!(!root.join("verify-state").join("enrich-stage").exists());
let wrong = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "apply-proposal",
            "--estate",
            &lab.display().to_string(),
            "--prepared",
            &dir.display().to_string(),
            "--tag",
            "other-tag",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
let wrong_text = text(&wrong);
assert!(!wrong.status.success(), "{wrong_text}");
assert!(wrong_text.contains("refuse:tag"), "{wrong_text}");
assert!(!state.join("enrich-stage").exists());
let staged = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "apply-proposal",
            "--estate",
            &lab.display().to_string(),
            "--prepared",
            &dir.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
let staged_text = text(&staged);
assert!(staged.status.success(), "{staged_text}");
assert!(
        staged_text.contains("apply-proposal did not apply"),
        "{staged_text}"
    );
assert!(staged_text.contains("auto_apply=false"), "{staged_text}");
assert!(staged_text.contains("--require-plan"), "{staged_text}");
assert_eq!(std::fs::read(&lab).unwrap(), lab_before);
let staged_estate = state.join("enrich-stage/staged-estate.yaml");
assert!(staged_estate.is_file());
let again = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "apply-proposal",
            "--estate",
            &lab.display().to_string(),
            "--prepared",
            &dir.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
let again_text = text(&again);
assert!(again.status.success(), "{again_text}");
assert!(again_text.contains("no-op:"), "{again_text}");
let status = estate_bin()
        .args([
            "status",
            "--estate",
            &lab.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--root",
            &repo_root().display().to_string(),
        ])
        .output()
        .unwrap();
let status_text = text(&status);
assert!(status.status.success(), "{status_text}");
assert!(
        status_text.contains("enrich_binding: pending"),
        "{status_text}"
    );
assert!(
        status_text.contains("enrich_stage: applied=false"),
        "{status_text}"
    );
let doctor = estate_bin()
        .args([
            "doctor",
            "--root",
            &repo_root().display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
let doctor_text = text(&doctor);
assert!(doctor.status.success(), "{doctor_text}");
assert!(
        doctor_text.contains("source estate not written"),
        "{doctor_text}"
    );
assert!(doctor_text.contains("binding proposal"), "{doctor_text}");
let plan = estate_bin()
        .args([
            "plan",
            "--estate",
            &staged_estate.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
        ])
        .output()
        .unwrap();
let plan_text = text(&plan);
assert!(plan.status.success(), "{plan_text}");
assert!(plan_text.contains("local_slm"), "{plan_text}");
let dry = estate_bin()
        .args([
            "apply",
            "--dry-run",
            "--require-plan",
            "--estate",
            &staged_estate.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
        ])
        .output()
        .unwrap();
assert!(dry.status.success(), "{}", text(&dry));
assert_eq!(std::fs::read(&lab).unwrap(), lab_before);
let ungated = estate_bin()
        .args([
            "apply",
            "--estate",
            &staged_estate.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
        ])
        .output()
        .unwrap();
let ungated_text = text(&ungated);
assert!(ungated.status.success(), "{ungated_text}");
assert!(ungated_text.contains("enrich stage held"), "{ungated_text}");
assert_eq!(std::fs::read(&lab).unwrap(), lab_before);
let applied = estate_bin()
        .args([
            "apply",
            "--require-plan",
            "--estate",
            &staged_estate.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--roots-base",
            &root.display().to_string(),
        ])
        .output()
        .unwrap();
let applied_text = text(&applied);
assert!(applied.status.success(), "{applied_text}");
assert!(
        applied_text.contains("enrich stage wrote"),
        "{applied_text}"
    );
let lab_after = std::fs::read_to_string(&lab).unwrap();
assert!(
        lab_after.contains("cell-enrich-overnight-traces"),
        "{lab_after}"
    );
assert_eq!(std::fs::read(&example).unwrap(), example_before);
let joined = estate_bin()
        .args([
            "status",
            "--estate",
            &lab.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--plans-dir",
            &plans.display().to_string(),
            "--root",
            &repo_root().display().to_string(),
        ])
        .output()
        .unwrap();
let joined_text = text(&joined);
assert!(joined.status.success(), "{joined_text}");
assert!(
        joined_text.contains("enrich_binding: local_slm model=cell-enrich-overnight-traces"),
        "{joined_text}"
    );
assert!(
        joined_text.contains("enrich_stage: applied=true"),
        "{joined_text}"
    );
}

#[test]
fn from_pack_prepares_accepted_fixture_and_keeps_refuses() {
let root = tmp("from-pack");
let seated = write_seated_estate(&root, "llama3");
let seated_path = seated.display().to_string();
let sacred = fixture("policy/sacred.yaml");
let pack_src = repo_root().join("examples/fixtures/specialist-overnight.pack.json");
let accepted = root.join("packs/accepted");
std::fs::create_dir_all(&accepted).unwrap();
std::fs::copy(&pack_src, accepted.join("overnight-traces.pack.json")).unwrap();
let before = estate_bytes();
let state = root.join("state");
let prepared = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "from-pack",
            "--estate",
            &seated_path,
            "--pack",
            "overnight-traces",
            "--packs-dir",
            &root.join("packs").display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
let prepared_text = text(&prepared);
assert!(prepared.status.success(), "{prepared_text}");
assert!(
        prepared_text.contains("enrich from-pack:"),
        "{prepared_text}"
    );
assert!(prepared_text.contains("prepared=2"), "{prepared_text}");
assert!(prepared_text.contains("base=llama3"), "{prepared_text}");
let modelfile = state.join("enrich/overnight-traces/ollama-modelfile/Modelfile");
let body = std::fs::read_to_string(&modelfile).unwrap();
assert!(body.contains("FROM llama3\n"), "{body}");
assert!(!body.contains("FROM local_slm"), "{body}");
assert!(state
        .join("enrich/overnight-traces/external-manifest/manifest.json")
        .is_file());
assert_eq!(
        estate_bytes(),
        before,
        "from-pack rewrote examples/estate.yaml"
    );
let one = root.join("one");
let driver = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "from-pack",
            "--estate",
            &seated_path,
            "--pack",
            &pack_src.display().to_string(),
            "--driver",
            "ollama-modelfile",
            "--state-dir",
            &one.display().to_string(),
        ])
        .output()
        .unwrap();
let driver_text = text(&driver);
assert!(driver.status.success(), "{driver_text}");
assert!(driver_text.contains("prepared=1"), "{driver_text}");
assert!(one
        .join("enrich/overnight-traces/ollama-modelfile/Modelfile")
        .is_file());
assert!(!one
        .join("enrich/overnight-traces/external-manifest")
        .exists());
let both = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "from-pack",
            "--estate",
            &seated_path,
            "--pack",
            "overnight-traces",
            "--packs-dir",
            &root.join("packs").display().to_string(),
            "--all-drivers",
            "--driver",
            "ollama-modelfile",
            "--state-dir",
            &root.join("both").display().to_string(),
        ])
        .output()
        .unwrap();
let both_text = text(&both);
assert!(!both.status.success(), "{both_text}");
assert!(both_text.contains("refuse:driver"), "{both_text}");
assert!(!root.join("both/enrich").exists());
let mut sacred_doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&pack_src).unwrap()).unwrap();
sacred_doc["id"] = serde_json::json!("sacred-pack");
sacred_doc["note"] = serde_json::Value::String("please mention cyera".into());
std::fs::write(
        accepted.join("sacred-pack.pack.json"),
        serde_json::to_string_pretty(&sacred_doc).unwrap(),
    )
    .unwrap();
let sacred_run = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "from-pack",
            "--estate",
            &seated_path,
            "--pack",
            "sacred-pack",
            "--packs-dir",
            &root.join("packs").display().to_string(),
            "--state-dir",
            &root.join("sacred-state").display().to_string(),
        ])
        .output()
        .unwrap();
let sacred_text = text(&sacred_run);
assert!(!sacred_run.status.success(), "{sacred_text}");
assert!(sacred_text.contains("refuse:sacred"), "{sacred_text}");
assert!(!root.join("sacred-state/enrich").exists());
let mut sku_doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&pack_src).unwrap()).unwrap();
sku_doc["id"] = serde_json::json!("sku-pack");
sku_doc["model_hint"] = serde_json::Value::String("rtx-5090".into());
std::fs::write(
        accepted.join("sku-pack.pack.json"),
        serde_json::to_string_pretty(&sku_doc).unwrap(),
    )
    .unwrap();
let sku_run = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "from-pack",
            "--estate",
            &seated_path,
            "--pack",
            "sku-pack",
            "--packs-dir",
            &root.join("packs").display().to_string(),
            "--state-dir",
            &root.join("sku-state").display().to_string(),
        ])
        .output()
        .unwrap();
let sku_text = text(&sku_run);
assert!(!sku_run.status.success(), "{sku_text}");
assert!(sku_text.to_ascii_lowercase().contains("sku"), "{sku_text}");
assert!(!root.join("sku-state/enrich").exists());
let curator = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "from-pack",
            "--estate",
            &seated_path,
            "--pack",
            "overnight-traces",
            "--packs-dir",
            &root.join("packs").display().to_string(),
            "--curator",
            "ada",
            "--state-dir",
            &root.join("curator-state").display().to_string(),
        ])
        .output()
        .unwrap();
let curator_text = text(&curator);
assert!(!curator.status.success(), "{curator_text}");
assert!(curator_text.contains("refuse:curator"), "{curator_text}");
assert!(!root.join("curator-state/enrich").exists());
let local_estate = root.join("local-only.yaml");
std::fs::write(
        &local_estate,
        "version: 0\nname: local-only\ndefault_effect: deny\nagents:\n  - id: horizon\n    display_name: Horizon\n    lane: horizon\n    desktop: horizon-desktop\nlanes:\n  - id: horizon\n    root_path: lanes/horizon\n    owner_agent_id: horizon\nmodel_bindings:\n  - id: local_slm\n    class: local\n    driver: ollama\n    wired: true\n    params:\n      model: \"llama3\"\nenrich_packs:\n  curator: jason\n  policy: manual\n",
    )
    .unwrap();
let mut frontier_doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&pack_src).unwrap()).unwrap();
frontier_doc["id"] = serde_json::json!("frontier-pack");
frontier_doc["source_drivers"] = serde_json::json!(["frontier"]);
frontier_doc["path_counts"]["frontier"] = serde_json::json!(1);
std::fs::write(
        accepted.join("frontier-pack.pack.json"),
        serde_json::to_string_pretty(&frontier_doc).unwrap(),
    )
    .unwrap();
let frontier = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "from-pack",
            "--estate",
            &local_estate.display().to_string(),
            "--pack",
            "frontier-pack",
            "--packs-dir",
            &root.join("packs").display().to_string(),
            "--state-dir",
            &root.join("frontier-state").display().to_string(),
        ])
        .output()
        .unwrap();
let frontier_text = text(&frontier);
assert!(!frontier.status.success(), "{frontier_text}");
assert!(
        frontier_text.contains("refuse:frontier-invent"),
        "{frontier_text}"
    );
assert!(!root.join("frontier-state/enrich").exists());
assert_eq!(estate_bytes(), before);
}

#[test]
fn axolotl_lora_prepare_and_import_trained_leave_the_estate() {
let root = tmp("axolotl-cli");
let sacred = fixture("policy/sacred.yaml");
let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
let before = estate_bytes();
let seat_only = write_seated_estate(&root, "llama3");
let blocked = root.join("seat-only");
let refused = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seat_only.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "axolotl-lora",
            "--out",
            &blocked.display().to_string(),
        ])
        .output()
        .unwrap();
let refused_text = text(&refused);
assert!(!refused.status.success(), "{refused_text}");
assert!(refused_text.contains("refuse:train-base"), "{refused_text}");
assert!(!refused_text.contains("meta-llama"), "{refused_text}");
assert!(!blocked.exists());
let blocked_state = root.join("blocked-state");
let blocked_all = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seat_only.display().to_string(),
            "--pack",
            &pack,
            "--all-drivers",
            "--job",
            "train",
            "--state-dir",
            &blocked_state.display().to_string(),
        ])
        .output()
        .unwrap();
let blocked_all_text = text(&blocked_all);
assert!(!blocked_all.status.success(), "{blocked_all_text}");
assert!(
        blocked_all_text.contains("refuse:train-base"),
        "{blocked_all_text}"
    );
assert!(!blocked_state.join("enrich").exists());
let seated = write_train_estate(&root, "llama3", Some("Qwen/Qwen2.5-0.5B-Instruct"));
let seated_path = seated.display().to_string();
let out = root.join("recipe");
let prepared = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated_path,
            "--pack",
            &pack,
            "--driver",
            "axolotl-lora",
            "--out",
            &out.display().to_string(),
        ])
        .output()
        .unwrap();
let prepared_text = text(&prepared);
assert!(prepared.status.success(), "{prepared_text}");
assert!(prepared_text.contains("job=train"), "{prepared_text}");
assert!(
        prepared_text.contains("driver=axolotl-lora"),
        "{prepared_text}"
    );
assert!(prepared_text.contains("promoted=false"), "{prepared_text}");
assert!(
        prepared_text.contains("estate_rewritten=false"),
        "{prepared_text}"
    );
assert!(prepared_text.contains("axolotl train "), "{prepared_text}");
let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("prepare.json")).unwrap()).unwrap();
assert_eq!(doc["job"], "train");
assert_eq!(doc["promoted"], false);
assert_eq!(doc["auto_apply"], false);
assert_eq!(doc["estate_rewritten"], false);
assert!(out.join("axolotl.yml").is_file());
assert!(out.join("dataset.jsonl").is_file());
assert_eq!(doc["base_model"], "llama3");
assert_eq!(doc["seat_tag"], "llama3");
assert_eq!(doc["train_base_model"], "Qwen/Qwen2.5-0.5B-Instruct");
let yaml = std::fs::read_to_string(out.join("axolotl.yml")).unwrap();
assert!(
        yaml.contains("base_model: \"Qwen/Qwen2.5-0.5B-Instruct\""),
        "{yaml}"
    );
assert!(
        !yaml
            .lines()
            .any(|line| line.trim_start().starts_with("base_model:") && line.contains("llama3")),
        "{yaml}"
    );
let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
assert!(next.contains("Seat tag is llama3"), "{next}");
assert!(
        next.contains("Train base is Qwen/Qwen2.5-0.5B-Instruct"),
        "{next}"
    );
assert!(
        next.contains(&format!(
            "axolotl train {}",
            out.join("axolotl.yml").display()
        )),
        "{next}"
    );
assert_eq!(estate_bytes(), before);
let enrich_job = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated_path,
            "--pack",
            &pack,
            "--driver",
            "axolotl-lora",
            "--job",
            "enrich",
            "--out",
            &root.join("enrich-job").display().to_string(),
        ])
        .output()
        .unwrap();
let enrich_text = text(&enrich_job);
assert!(!enrich_job.status.success(), "{enrich_text}");
assert!(enrich_text.contains("refuse:job"), "{enrich_text}");
assert!(!root.join("enrich-job").exists());
let state = root.join("state");
let all_train = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated_path,
            "--pack",
            &pack,
            "--all-drivers",
            "--job",
            "train",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
let all_text = text(&all_train);
assert!(all_train.status.success(), "{all_text}");
assert!(all_text.contains("prepared=8"), "{all_text}");
assert!(all_text.contains("omit driver=mlx-lm-lora"), "{all_text}");
assert!(all_text.contains("refuse:host"), "{all_text}");
assert!(
        !all_text.contains("enrich prepare: driver=mlx-lm-lora"),
        "{all_text}"
    );
assert!(all_text.contains("driver=unsloth-qlora"), "{all_text}");
assert!(all_text.contains("driver=unsloth-lora"), "{all_text}");
assert!(all_text.contains("driver=llamafactory-qlora"), "{all_text}");
assert!(all_text.contains("driver=llamafactory-lora"), "{all_text}");
assert!(all_text.contains("driver=axolotl-lora"), "{all_text}");
assert!(all_text.contains("driver=axolotl-qlora"), "{all_text}");
let unsloth_dir = state.join("enrich/overnight-traces/unsloth-qlora");
assert!(unsloth_dir.join("UNSLOTH.md").is_file());
assert!(unsloth_dir.join("NEXT.md").is_file());
assert!(!unsloth_dir.join("train_unsloth.py").exists());
assert!(!unsloth_dir.join("dataset.jsonl").exists());
let unsloth_lora_dir = state.join("enrich/overnight-traces/unsloth-lora");
assert!(unsloth_lora_dir.join("UNSLOTH.md").is_file());
let unsloth_lora_md = std::fs::read_to_string(unsloth_lora_dir.join("UNSLOTH.md")).unwrap();
assert!(unsloth_lora_md.contains("Unsloth LoRA handoff"), "{unsloth_lora_md}");
assert!(!unsloth_lora_dir.join("dataset.jsonl").exists());
assert!(!state.join("enrich/overnight-traces/mlx-lm-lora").exists());
assert!(state
        .join("enrich/overnight-traces/llamafactory-qlora/recipe.yaml")
        .is_file());
let all_yaml =
        std::fs::read_to_string(state.join("enrich/overnight-traces/axolotl-lora/axolotl.yml"))
            .unwrap();
assert!(
        all_yaml.contains("base_model: \"Qwen/Qwen2.5-0.5B-Instruct\""),
        "{all_yaml}"
    );
assert!(
        all_yaml.lines().any(|line| line == "adapter: lora"),
        "{all_yaml}"
    );
assert!(
        all_yaml.lines().any(|line| line == "load_in_4bit: false"),
        "{all_yaml}"
    );
assert!(
        !all_yaml
            .lines()
            .any(|line| line.trim_start().starts_with("base_model:") && line.contains("llama3")),
        "{all_yaml}"
    );
let all_ax_qlora =
        std::fs::read_to_string(state.join("enrich/overnight-traces/axolotl-qlora/axolotl.yml"))
            .unwrap();
assert!(
        all_ax_qlora.contains("base_model: \"Qwen/Qwen2.5-0.5B-Instruct\""),
        "{all_ax_qlora}"
    );
assert!(
        all_ax_qlora.lines().any(|line| line == "adapter: qlora"),
        "{all_ax_qlora}"
    );
assert!(
        all_ax_qlora
            .lines()
            .any(|line| line == "load_in_4bit: true"),
        "{all_ax_qlora}"
    );
let all_modelfile =
        std::fs::read_to_string(state.join("enrich/overnight-traces/ollama-modelfile/Modelfile"))
            .unwrap();
assert!(all_modelfile.contains("FROM llama3\n"), "{all_modelfile}");
let all_lora = std::fs::read_to_string(
        state.join("enrich/overnight-traces/llamafactory-lora/recipe.yaml"),
    )
    .unwrap();
assert!(
        all_lora.lines().any(|line| line.trim() == "lora_rank: 8"),
        "{all_lora}"
    );
assert!(
        !all_lora.contains("quantization_bit") && !all_lora.contains("quantization_method"),
        "{all_lora}"
    );
let all_qlora = std::fs::read_to_string(
        state.join("enrich/overnight-traces/llamafactory-qlora/recipe.yaml"),
    )
    .unwrap();
assert!(
        all_qlora.contains("quantization_method: bnb"),
        "{all_qlora}"
    );
let adapter = root.join("adapter");
std::fs::create_dir_all(&adapter).unwrap();
std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
let imported = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-trained",
            "--estate",
            &seated_path,
            "--prepared",
            &out.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--adapter",
            &adapter.display().to_string(),
        ])
        .output()
        .unwrap();
let imported_text = text(&imported);
assert!(imported.status.success(), "{imported_text}");
assert!(
        imported_text.contains("binding=local_slm"),
        "{imported_text}"
    );
assert!(
        imported_text.contains("import-trained did not apply"),
        "{imported_text}"
    );
assert!(
        imported_text.contains("auto_apply=false"),
        "{imported_text}"
    );
assert!(out.join("binding-proposal.json").is_file());
assert_eq!(estate_bytes(), before);
let missing = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-trained",
            "--estate",
            &seated_path,
            "--prepared",
            &out.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--adapter",
            &root.join("missing-adapter").display().to_string(),
        ])
        .output()
        .unwrap();
let missing_text = text(&missing);
assert!(!missing.status.success(), "{missing_text}");
assert!(missing_text.contains("refuse:adapter"), "{missing_text}");
}

#[test]
fn llamafactory_qlora_prepare_and_import_trained_leave_the_estate() {
let root = tmp("llamafactory-cli");
let seated_only = write_seated_estate(&root, "llama3");
let seated_only_path = seated_only.display().to_string();
let sacred = fixture("policy/sacred.yaml");
let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
let before = estate_bytes();
let blocked = root.join("seat-only");
let refused = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated_only_path,
            "--pack",
            &pack,
            "--driver",
            "llamafactory-qlora",
            "--out",
            &blocked.display().to_string(),
        ])
        .output()
        .unwrap();
let refused_text = text(&refused);
assert!(!refused.status.success(), "{refused_text}");
assert!(refused_text.contains("refuse:train-base"), "{refused_text}");
assert!(!refused_text.contains("meta-llama"), "{refused_text}");
assert!(!blocked.exists());
let seated = write_train_estate(&root, "llama3", Some("Qwen/Qwen2.5-0.5B-Instruct"));
let seated_path = seated.display().to_string();
let out = root.join("recipe");
let prepared = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated_path,
            "--pack",
            &pack,
            "--driver",
            "llamafactory-qlora",
            "--out",
            &out.display().to_string(),
        ])
        .output()
        .unwrap();
let prepared_text = text(&prepared);
assert!(prepared.status.success(), "{prepared_text}");
assert!(
        prepared_text.contains("train_base=Qwen/Qwen2.5-0.5B-Instruct"),
        "{prepared_text}"
    );
assert!(prepared_text.contains("base=llama3"), "{prepared_text}");
assert!(prepared_text.contains("job=train"), "{prepared_text}");
assert!(
        prepared_text.contains("driver=llamafactory-qlora"),
        "{prepared_text}"
    );
assert!(
        prepared_text.contains("llamafactory-cli train "),
        "{prepared_text}"
    );
assert!(
        prepared_text.contains("pip install llamafactory"),
        "{prepared_text}"
    );
assert!(
        prepared_text.contains("bitsandbytes>=0.49"),
        "{prepared_text}"
    );
let recipe = std::fs::read_to_string(out.join("recipe.yaml")).unwrap();
assert!(recipe.contains("quantization_bit: 4"), "{recipe}");
assert!(recipe.contains("quantization_method: bnb"), "{recipe}");
assert!(
        !recipe.contains("quantization_method: bitsandbytes"),
        "{recipe}"
    );
assert!(recipe.contains("lora_rank: 16"), "{recipe}");
assert!(recipe.contains("cutoff_len: 512"), "{recipe}");
assert!(recipe.contains("template: qwen"), "{recipe}");
assert!(
        recipe.contains("model_name_or_path: \"Qwen/Qwen2.5-0.5B-Instruct\""),
        "{recipe}"
    );
assert!(
        !recipe
            .lines()
            .any(|line| line.trim_start().starts_with("max_steps:")),
        "{recipe}"
    );
let jsonl = std::fs::read_to_string(out.join("dataset.jsonl")).unwrap();
assert!(jsonl.contains("\"messages\""), "{jsonl}");
let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
assert!(
        next.contains(&format!(
            "llamafactory-cli train {}",
            out.join("recipe.yaml").display()
        )),
        "{next}"
    );
assert!(next.contains("llamafactory-cli export "), "{next}");
assert!(next.contains("Faster single-GPU alternate"), "{next}");
assert!(next.contains("bitsandbytes>=0.49"), "{next}");
assert!(next.contains("--max-steps 10"), "{next}");
assert_eq!(estate_bytes(), before);
let gauge = root.join("gauge");
let gauged = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated_path,
            "--pack",
            &pack,
            "--driver",
            "llamafactory-qlora",
            "--max-steps",
            "10",
            "--out",
            &gauge.display().to_string(),
        ])
        .output()
        .unwrap();
let gauge_text = text(&gauged);
assert!(gauged.status.success(), "{gauge_text}");
let gauge_recipe = std::fs::read_to_string(gauge.join("recipe.yaml")).unwrap();
assert!(
        gauge_recipe
            .lines()
            .any(|line| line.trim() == "max_steps: 10"),
        "{gauge_recipe}"
    );
assert!(
        gauge_recipe
            .lines()
            .any(|line| line.trim() == "save_steps: 10"),
        "{gauge_recipe}"
    );
assert!(
        gauge_recipe.contains("quantization_method: bnb"),
        "{gauge_recipe}"
    );
let enrich_job = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated_path,
            "--pack",
            &pack,
            "--driver",
            "llamafactory-qlora",
            "--job",
            "enrich",
            "--out",
            &root.join("enrich-job").display().to_string(),
        ])
        .output()
        .unwrap();
let enrich_text = text(&enrich_job);
assert!(!enrich_job.status.success(), "{enrich_text}");
assert!(enrich_text.contains("refuse:job"), "{enrich_text}");
assert!(!root.join("enrich-job").exists());
let adapter = root.join("adapter");
std::fs::create_dir_all(&adapter).unwrap();
std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
let imported = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-trained",
            "--estate",
            &seated_path,
            "--prepared",
            &out.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--adapter",
            &adapter.display().to_string(),
        ])
        .output()
        .unwrap();
let imported_text = text(&imported);
assert!(imported.status.success(), "{imported_text}");
assert!(
        imported_text.contains("driver=llamafactory-qlora"),
        "{imported_text}"
    );
assert!(imported_text.contains("shape=adapter"), "{imported_text}");
assert!(
        imported_text.contains("import-trained did not apply"),
        "{imported_text}"
    );
let trained: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("prepare.json")).unwrap()).unwrap();
assert_eq!(trained["trained_shape"], "adapter");
assert_eq!(trained["promoted"], false);
assert_eq!(trained["auto_apply"], false);
assert_eq!(trained["estate_rewritten"], false);
assert_eq!(estate_bytes(), before);
}

#[test]
fn llamafactory_relative_train_base_is_absolute_and_seat_leaves_refuse() {
let root = tmp("llamafactory-abs");
let sacred = fixture("policy/sacred.yaml");
let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
let before = estate_bytes();
for bad in ["./llama3", "../llama3"] {
        let seated = write_train_estate(&root, "llama3", Some(bad));
        let out = root.join(bad.trim_start_matches('.').replace('/', "_"));
        let refused = estate_bin()
            .args([
                "--sacred",
                &sacred,
                "enrich",
                "prepare",
                "--estate",
                &seated.display().to_string(),
                "--pack",
                &pack,
                "--driver",
                "llamafactory-qlora",
                "--out",
                &out.display().to_string(),
            ])
            .output()
            .unwrap();
        let refused_text = text(&refused);
        assert!(!refused.status.success(), "{bad}: {refused_text}");
        assert!(
            refused_text.contains("refuse:train-base"),
            "{bad}: {refused_text}"
        );
        assert!(
            refused_text.contains("Ollama seat tag"),
            "{bad}: {refused_text}"
        );
        assert!(
            !refused_text.contains("meta-llama"),
            "{bad}: {refused_text}"
        );
        assert!(!out.exists(), "{bad}");
    }
let seated = write_train_estate(&root, "llama3", Some("./weights/Qwen2.5-0.5B-Instruct"));
let out = root.join("relative");
let prepared = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "llamafactory-qlora",
            "--out",
            &out.display().to_string(),
        ])
        .output()
        .unwrap();
let prepared_text = text(&prepared);
assert!(prepared.status.success(), "{prepared_text}");
let recipe = std::fs::read_to_string(out.join("recipe.yaml")).unwrap();
let export = std::fs::read_to_string(out.join("export.yaml")).unwrap();
let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("prepare.json")).unwrap()).unwrap();
let train = doc["train_base_model"].as_str().unwrap();
assert!(std::path::Path::new(train).is_absolute(), "{train}");
assert!(train.ends_with("/weights/Qwen2.5-0.5B-Instruct"), "{train}");
assert!(!train.contains("/./") && !train.contains(".."), "{train}");
let quoted = format!("model_name_or_path: \"{train}\"");
assert!(recipe.contains(&quoted), "{recipe}");
assert!(export.contains(&quoted), "{export}");
assert!(recipe.contains("template: qwen"), "{recipe}");
assert!(recipe.contains("quantization_method: bnb"), "{recipe}");
assert!(!recipe.contains("./weights"), "{recipe}");
assert_eq!(doc["base_model"], "llama3");
assert_eq!(doc["seat_tag"], "llama3");
let hub = write_train_estate(&root, "llama3", Some("Qwen/Qwen2.5-0.5B-Instruct"));
let hub_out = root.join("hub");
let hub_run = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &hub.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "llamafactory-qlora",
            "--out",
            &hub_out.display().to_string(),
        ])
        .output()
        .unwrap();
assert!(hub_run.status.success(), "{}", text(&hub_run));
let hub_recipe = std::fs::read_to_string(hub_out.join("recipe.yaml")).unwrap();
assert!(
        hub_recipe.contains("model_name_or_path: \"Qwen/Qwen2.5-0.5B-Instruct\""),
        "{hub_recipe}"
    );
assert_eq!(estate_bytes(), before);
}

#[test]
fn llamafactory_lora_prepare_omits_quantization_and_imports() {
let root = tmp("llamafactory-lora-cli");
let sacred = fixture("policy/sacred.yaml");
let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
let before = estate_bytes();
let seated_only = write_seated_estate(&root, "llama3");
let blocked = root.join("seat-only");
let refused = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated_only.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "llamafactory-lora",
            "--out",
            &blocked.display().to_string(),
        ])
        .output()
        .unwrap();
let refused_text = text(&refused);
assert!(!refused.status.success(), "{refused_text}");
assert!(refused_text.contains("refuse:train-base"), "{refused_text}");
assert!(!refused_text.contains("meta-llama"), "{refused_text}");
assert!(!blocked.exists());
let seated = write_train_estate(&root, "llama3", Some("Qwen/Qwen3-4B-Instruct-2507"));
let out = root.join("recipe");
let prepared = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "llamafactory-lora",
            "--out",
            &out.display().to_string(),
        ])
        .output()
        .unwrap();
let prepared_text = text(&prepared);
assert!(prepared.status.success(), "{prepared_text}");
assert!(
        prepared_text.contains("driver=llamafactory-lora"),
        "{prepared_text}"
    );
assert!(prepared_text.contains("job=train"), "{prepared_text}");
assert!(
        prepared_text.contains("does not require bitsandbytes"),
        "{prepared_text}"
    );
assert!(
        !prepared_text.contains("bitsandbytes>=0.49"),
        "{prepared_text}"
    );
let recipe = std::fs::read_to_string(out.join("recipe.yaml")).unwrap();
assert!(recipe.contains("finetuning_type: lora"), "{recipe}");
assert!(
        recipe.lines().any(|line| line.trim() == "lora_rank: 8"),
        "{recipe}"
    );
assert!(
        recipe.lines().any(|line| line.trim() == "packing: false"),
        "{recipe}"
    );
assert!(
        recipe
            .lines()
            .any(|line| line.trim() == "template: qwen3_nothink"),
        "{recipe}"
    );
assert!(
        !recipe.contains("quantization_bit") && !recipe.contains("quantization_method"),
        "{recipe}"
    );
assert!(
        recipe.contains("model_name_or_path: \"Qwen/Qwen3-4B-Instruct-2507\""),
        "{recipe}"
    );
let export = std::fs::read_to_string(out.join("export.yaml")).unwrap();
assert!(!export.contains("quantization_bit"), "{export}");
assert!(export.contains("# merge_status: not_run"), "{export}");
assert!(export.contains("This prepare did not merge"), "{export}");
assert!(
        export
            .lines()
            .any(|line| line.trim() == "template: qwen3_nothink"),
        "{export}"
    );
let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
assert!(next.contains("does not require bitsandbytes"), "{next}");
assert!(!next.contains("bitsandbytes>=0.49"), "{next}");
assert!(
        next.contains(
            "Reproduce target on the unquantized LoRA card, the non-quant twin of the Qwen3 Instruct QLoRA prepare."
        ),
        "{next}"
    );
assert!(
        !next.contains(
            "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x LoRA/QLoRA."
        ),
        "{next}"
    );
assert!(
        !next.contains("quantization_bit: 4") && !next.contains("quantization_method: bnb"),
        "{next}"
    );
assert!(next.contains("pip install llamafactory"), "{next}");
assert!(
        next.contains(&format!(
            "llamafactory-cli train {}",
            out.join("recipe.yaml").display()
        )),
        "{next}"
    );
let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("prepare.json")).unwrap()).unwrap();
assert_eq!(doc["driver"], "llamafactory-lora");
assert_eq!(doc["base_model"], "llama3");
assert_eq!(doc["seat_tag"], "llama3");
assert_eq!(doc["train_base_model"], "Qwen/Qwen3-4B-Instruct-2507");
let adapter = root.join("adapter");
std::fs::create_dir_all(&adapter).unwrap();
std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
let imported = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-trained",
            "--estate",
            &seated.display().to_string(),
            "--prepared",
            &out.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--adapter",
            &adapter.display().to_string(),
        ])
        .output()
        .unwrap();
let imported_text = text(&imported);
assert!(imported.status.success(), "{imported_text}");
assert!(
        imported_text.contains("driver=llamafactory-lora"),
        "{imported_text}"
    );
assert!(imported_text.contains("shape=adapter"), "{imported_text}");
assert!(
        imported_text.contains("import-trained did not apply"),
        "{imported_text}"
    );
assert!(
        next.contains(&format!(
            "estate enrich import-trained --estate <estate.yaml> --prepared {} --tag cell-enrich-overnight-traces --adapter {}",
            out.display(),
            out.join("outputs").display()
        )),
        "{next}"
    );
assert!(
        next.contains(&format!(
            "estate enrich import-trained --estate <estate.yaml> --prepared {} --tag cell-enrich-overnight-traces --adapter {}",
            out.display(),
            out.join("export").display()
        )),
        "{next}"
    );
let trained: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("prepare.json")).unwrap()).unwrap();
assert_eq!(trained["trained_shape"], "adapter");
assert_eq!(trained["promoted"], false);
assert_eq!(estate_bytes(), before);
let garbage = root.join("garbage.txt");
std::fs::write(&garbage, "not weights\n").unwrap();
let refused = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-trained",
            "--estate",
            &seated.display().to_string(),
            "--prepared",
            &out.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--adapter",
            &garbage.display().to_string(),
        ])
        .output()
        .unwrap();
let refused_text = text(&refused);
assert!(!refused.status.success(), "{refused_text}");
assert!(refused_text.contains("refuse:adapter"), "{refused_text}");
assert!(
        refused_text.contains("is a file and is not a GGUF"),
        "{refused_text}"
    );
let still: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("prepare.json")).unwrap()).unwrap();
assert_eq!(still["trained_shape"], "adapter");
assert_eq!(estate_bytes(), before);
}

#[test]
fn official_scale_flag_writes_the_sft_fields_and_refuses_a_quantized_export() {
let root = tmp("official-scale");
let sacred = fixture("policy/sacred.yaml");
let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
let seated = write_train_estate(&root, "llama3", Some("Qwen/Qwen2.5-0.5B-Instruct"));
let before = estate_bytes();
let out = root.join("lora");
let prepared = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "llamafactory-lora",
            "--official-scale",
            "--out",
            &out.display().to_string(),
        ])
        .output()
        .unwrap();
let prepared_text = text(&prepared);
assert!(prepared.status.success(), "{prepared_text}");
let recipe = std::fs::read_to_string(out.join("recipe.yaml")).unwrap();
for line in [
        "cutoff_len: 2048",
        "num_train_epochs: 3.0",
        "gradient_accumulation_steps: 8",
        "warmup_ratio: 0.1",
        "lora_rank: 8",
        "packing: false",
    ] {
        assert!(
            recipe.lines().any(|row| row.trim() == line),
            "{line} missing\n{recipe}"
        );
    }
assert!(
        !recipe.contains("quantization_bit") && !recipe.contains("quantization_method"),
        "{recipe}"
    );
let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
assert!(next.contains("This prepare did not merge"), "{next}");
assert!(next.contains("The merge has not happened."), "{next}");
assert!(
        next.contains("Do not set quantization_bit on export.yaml"),
        "{next}"
    );
let export = std::fs::read_to_string(out.join("export.yaml")).unwrap();
assert!(!export.contains("quantization_bit"), "{export}");
assert!(export.contains("# merge_status: not_run"), "{export}");
let gauge = root.join("gauge");
let gauged = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "llamafactory-lora",
            "--official-scale",
            "--max-steps",
            "10",
            "--out",
            &gauge.display().to_string(),
        ])
        .output()
        .unwrap();
assert!(gauged.status.success(), "{}", text(&gauged));
let gauge_recipe = std::fs::read_to_string(gauge.join("recipe.yaml")).unwrap();
assert!(
        gauge_recipe
            .lines()
            .any(|row| row.trim() == "max_steps: 10"),
        "{gauge_recipe}"
    );
assert!(
        gauge_recipe
            .lines()
            .any(|row| row.trim() == "cutoff_len: 2048"),
        "{gauge_recipe}"
    );
assert!(!gauge_recipe.contains("quantization_bit"), "{gauge_recipe}");
let blocked = root.join("ollama");
let refused = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "ollama-modelfile",
            "--official-scale",
            "--out",
            &blocked.display().to_string(),
        ])
        .output()
        .unwrap();
let refused_text = text(&refused);
assert!(!refused.status.success(), "{refused_text}");
assert!(
        refused_text.contains("refuse:official-scale"),
        "{refused_text}"
    );
assert!(!blocked.exists());
let export_path = out.join("export.yaml");
let original = std::fs::read_to_string(&export_path).unwrap();
std::fs::write(&export_path, format!("{original}quantization_bit: 4\n")).unwrap();
let adapter = root.join("adapter");
std::fs::create_dir_all(&adapter).unwrap();
std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
let imported = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-trained",
            "--estate",
            &seated.display().to_string(),
            "--prepared",
            &out.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--adapter",
            &adapter.display().to_string(),
        ])
        .output()
        .unwrap();
let imported_text = text(&imported);
assert!(!imported.status.success(), "{imported_text}");
assert!(imported_text.contains("refuse:export"), "{imported_text}");
assert_eq!(estate_bytes(), before);
}

#[test]
fn unsloth_qlora_prepare_refuses_a_missing_train_base_and_leaves_the_estate() {
let root = tmp("unsloth-cli");
let sacred = fixture("policy/sacred.yaml");
let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
let before = estate_bytes();
let seated = write_train_estate(&root, "llama3", Some("Qwen/Qwen2.5-0.5B-Instruct"));
let seated_bytes = std::fs::read_to_string(&seated).unwrap();
let seat_dir = root.join("seat-only-src");
std::fs::create_dir_all(&seat_dir).unwrap();
let seat_only = write_seated_estate(&seat_dir, "llama3");
let blocked = root.join("blocked");
let refused = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seat_only.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "unsloth-qlora",
            "--out",
            &blocked.display().to_string(),
        ])
        .output()
        .unwrap();
let refused_text = text(&refused);
assert!(!refused.status.success(), "{refused_text}");
assert!(refused_text.contains("refuse:train-base"), "{refused_text}");
assert!(!blocked.exists());
let out = root.join("handoff");
let prepared = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "unsloth-qlora",
            "--out",
            &out.display().to_string(),
        ])
        .output()
        .unwrap();
let prepared_text = text(&prepared);
assert!(prepared.status.success(), "{prepared_text}");
assert!(
        prepared_text.contains("driver=unsloth-qlora"),
        "{prepared_text}"
    );
assert!(
        prepared_text.contains("estate_rewritten=false"),
        "{prepared_text}"
    );
assert_eq!(std::fs::read_to_string(&seated).unwrap(), seated_bytes);
assert_eq!(estate_bytes(), before);
let handoff = std::fs::read_to_string(out.join("UNSLOTH.md")).unwrap();
assert!(handoff.contains("operator-owned"), "{handoff}");
assert!(handoff.contains("does not call Unsloth"), "{handoff}");
assert!(
        handoff.contains("train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\""),
        "{handoff}"
    );
assert!(handoff.contains("seat_tag: \"llama3\""), "{handoff}");
assert!(!out.join("train_unsloth.py").exists());
assert!(!out.join("dataset.jsonl").exists());
let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
assert!(next.contains("import-trained"), "{next}");
assert!(next.contains("READY_FOR_LIVE_TEST: no"), "{next}");
assert!(!next.contains("READY_FOR_LIVE_TEST: yes"), "{next}");
assert!(
        next.contains("https://unsloth.ai/docs/get-started/install"),
        "{next}"
    );
let official = root.join("official");
let official_out = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "unsloth-qlora",
            "--official-scale",
            "--out",
            &official.display().to_string(),
        ])
        .output()
        .unwrap();
let official_text = text(&official_out);
assert!(!official_out.status.success(), "{official_text}");
assert!(
        official_text.contains("refuse:official-scale"),
        "{official_text}"
    );
assert!(!official.exists());
assert_eq!(estate_bytes(), before);
let adapter = root.join("adapter");
std::fs::create_dir_all(&adapter).unwrap();
std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
let imported = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "import-trained",
            "--estate",
            &seated.display().to_string(),
            "--prepared",
            &out.display().to_string(),
            "--tag",
            "cell-enrich-overnight-traces",
            "--adapter",
            &adapter.display().to_string(),
        ])
        .output()
        .unwrap();
let imported_text = text(&imported);
assert!(imported.status.success(), "{imported_text}");
assert!(imported_text.contains("shape=adapter"), "{imported_text}");
assert_eq!(std::fs::read_to_string(&seated).unwrap(), seated_bytes);
assert_eq!(estate_bytes(), before);
let proposal = std::fs::read_to_string(out.join("binding-proposal.json")).unwrap();
assert!(
        proposal.contains("\"driver\": \"unsloth-qlora\""),
        "{proposal}"
    );
assert!(
        proposal.contains("\"estate_rewritten\": false"),
        "{proposal}"
    );
assert!(proposal.contains("\"promoted\": false"), "{proposal}");
}

#[test]
fn mlx_lm_lora_prepare_refuses_the_wrong_host_and_writes_a_handoff_on_apple_silicon() {
let root = tmp("mlx-cli");
let sacred = fixture("policy/sacred.yaml");
let pack = fixture("examples/fixtures/specialist-overnight.pack.json");
let before = estate_bytes();
let seated = write_train_estate(&root, "llama3", Some("Qwen/Qwen2.5-0.5B-Instruct"));
let seated_bytes = std::fs::read_to_string(&seated).unwrap();
let apple_src = std::fs::read_to_string(
        repo_root().join("examples/fixtures/specialist-overnight.pack.json"),
    )
    .unwrap();
let apple_body = apple_src.replace(
        "\"host_class_affinity\": \"any\"",
        "\"host_class_affinity\": \"apple-silicon\"",
    );
assert_ne!(apple_body, apple_src);
let apple_pack = root.join("apple.pack.json");
std::fs::write(&apple_pack, apple_body).unwrap();
let blocked = root.join("wrong-host");
let refused = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated.display().to_string(),
            "--pack",
            &pack,
            "--driver",
            "mlx-lm-lora",
            "--out",
            &blocked.display().to_string(),
        ])
        .output()
        .unwrap();
let refused_text = text(&refused);
assert!(!refused.status.success(), "{refused_text}");
assert!(refused_text.contains("refuse:host"), "{refused_text}");
assert!(refused_text.contains("apple-silicon"), "{refused_text}");
assert!(!blocked.exists());
assert_eq!(estate_bytes(), before);
let seat_dir = root.join("seat-only-src");
std::fs::create_dir_all(&seat_dir).unwrap();
let seat_only = write_seated_estate(&seat_dir, "llama3");
let missing = root.join("missing-base");
let missing_out = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seat_only.display().to_string(),
            "--pack",
            &apple_pack.display().to_string(),
            "--driver",
            "mlx-lm-lora",
            "--out",
            &missing.display().to_string(),
        ])
        .output()
        .unwrap();
let missing_text = text(&missing_out);
assert!(!missing_out.status.success(), "{missing_text}");
assert!(missing_text.contains("refuse:train-base"), "{missing_text}");
assert!(!missing.exists());
let enrich_job = root.join("enrich-job");
let enrich = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated.display().to_string(),
            "--pack",
            &apple_pack.display().to_string(),
            "--driver",
            "mlx-lm-lora",
            "--job",
            "enrich",
            "--out",
            &enrich_job.display().to_string(),
        ])
        .output()
        .unwrap();
let enrich_text = text(&enrich);
assert!(!enrich.status.success(), "{enrich_text}");
assert!(enrich_text.contains("refuse:job"), "{enrich_text}");
assert!(!enrich_job.exists());
let official = root.join("official");
let official_out = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated.display().to_string(),
            "--pack",
            &apple_pack.display().to_string(),
            "--driver",
            "mlx-lm-lora",
            "--official-scale",
            "--out",
            &official.display().to_string(),
        ])
        .output()
        .unwrap();
let official_text = text(&official_out);
assert!(!official_out.status.success(), "{official_text}");
assert!(
        official_text.contains("refuse:official-scale"),
        "{official_text}"
    );
assert!(!official.exists());
let out = root.join("handoff");
let prepared = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated.display().to_string(),
            "--pack",
            &apple_pack.display().to_string(),
            "--driver",
            "mlx-lm-lora",
            "--out",
            &out.display().to_string(),
        ])
        .output()
        .unwrap();
let prepared_text = text(&prepared);
assert!(prepared.status.success(), "{prepared_text}");
assert!(
        prepared_text.contains("driver=mlx-lm-lora"),
        "{prepared_text}"
    );
assert!(
        prepared_text.contains("estate_rewritten=false"),
        "{prepared_text}"
    );
assert_eq!(std::fs::read_to_string(&seated).unwrap(), seated_bytes);
assert_eq!(estate_bytes(), before);
let handoff = std::fs::read_to_string(out.join("MLX.md")).unwrap();
assert!(handoff.contains("operator-owned"), "{handoff}");
assert!(handoff.contains("does not call mlx-lm"), "{handoff}");
assert!(
        handoff.contains("train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\""),
        "{handoff}"
    );
assert!(handoff.contains("seat_tag: \"llama3\""), "{handoff}");
assert!(
        handoff.contains("https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md"),
        "{handoff}"
    );
assert!(
        handoff.contains("mlx_lm.fuse --model <path_to_model>"),
        "{handoff}"
    );
assert!(handoff.contains("refuse:adapter"), "{handoff}");
assert!(
        !handoff
            .contains("a merged directory that contains config.json and at least one .safetensors"),
        "{handoff}"
    );
assert!(!out.join("dataset.jsonl").exists());
assert!(!out.join("recipe.yaml").exists());
assert!(out.read_dir().unwrap().all(|entry| {
        let name = entry.unwrap().file_name().into_string().unwrap();
        !name.ends_with(".py")
    }
));
let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
assert!(next.contains("import-trained"), "{next}");
assert!(next.contains("READY_FOR_LIVE_TEST: no"), "{next}");
assert!(!next.contains("READY_FOR_LIVE_TEST: yes"), "{next}");
assert!(
        next.contains("mlx_lm.fuse --model <path_to_model>"),
        "{next}"
    );
assert!(next.contains("mlx_lm.fuse --export-gguf"), "{next}");
assert!(next.contains("adapters.safetensors"), "{next}");
assert!(next.contains("ggml-model-f16.gguf"), "{next}");
assert!(next.contains("estate enrich merge-adapt"), "{next}");
assert!(next.contains("## After the mlx-lm train"), "{next}");
assert!(next.contains("fused MLX"), "{next}");
assert!(next.contains("refuse:adapter"), "{next}");
assert!(!next.contains("--adapter <merged-dir>"), "{next}");
assert!(!next.contains("including fuse when you fused"), "{next}");
assert!(!next.contains("python3 convert_hf_to_gguf.py"), "{next}");
let prepare_md = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
assert!(
        prepare_md.contains("estate enrich merge-adapt"),
        "{prepare_md}"
    );
assert!(prepare_md.contains("--export-gguf"), "{prepare_md}");
assert!(prepare_md.contains("refuse:adapter"), "{prepare_md}");
assert!(
        !prepare_md.contains("python3 convert_hf_to_gguf.py"),
        "{prepare_md}"
    );
let state = root.join("state");
let all_train = estate_bin()
        .args([
            "--sacred",
            &sacred,
            "enrich",
            "prepare",
            "--estate",
            &seated.display().to_string(),
            "--pack",
            &apple_pack.display().to_string(),
            "--all-drivers",
            "--job",
            "train",
            "--state-dir",
            &state.display().to_string(),
        ])
        .output()
        .unwrap();
let all_text = text(&all_train);
assert!(all_train.status.success(), "{all_text}");
assert!(all_text.contains("prepared=9"), "{all_text}");
assert!(
        all_text.contains("enrich prepare: driver=mlx-lm-lora"),
        "{all_text}"
    );
assert!(!all_text.contains("omit driver=mlx-lm-lora"), "{all_text}");
let mlx_dir = state.join("enrich/overnight-traces/mlx-lm-lora");
assert!(mlx_dir.join("MLX.md").is_file());
assert!(!mlx_dir.join("dataset.jsonl").exists());
let recipe = std::fs::read_to_string(
        state.join("enrich/overnight-traces/llamafactory-qlora/recipe.yaml"),
    )
    .unwrap();
assert!(recipe.contains("quantization_method: bnb"), "{recipe}");
assert!(recipe.contains("quantization_bit: 4"), "{recipe}");
assert!(!recipe.contains("mlx-lm-lora"), "{recipe}");
assert!(!recipe.contains("mlx_lm"), "{recipe}");
assert_eq!(std::fs::read_to_string(&seated).unwrap(), seated_bytes);
assert_eq!(estate_bytes(), before);
}

#[test]
fn axolotl_qlora_journey_stays_print_only_and_off_smoke() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
for target in ["axolotl-qlora-journey:", "uniqueness-axolotl:"] {
        assert!(
            makefile.lines().any(|line| line.trim() == target),
            "Makefile missing {target}"
        );
    }
assert!(makefile.contains("scripts/axolotl-qlora-journey.sh"));
assert!(makefile.contains("scripts/uniqueness-axolotl.sh"));
let phony = makefile.lines().next().unwrap_or("");
assert!(
        phony.contains("axolotl-qlora-journey") && phony.contains("uniqueness-axolotl"),
        "axolotl journey targets must be phony"
    );
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("axolotl-qlora-journey") && !gate90.contains("uniqueness-axolotl"),
        "gate-90 must not run the axolotl journey: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("axolotl-qlora-journey") && !smoke.contains("uniqueness-axolotl"),
        "smoke must not run the axolotl journey: {smoke}"
    );
let script = std::fs::read_to_string(root.join("scripts/axolotl-qlora-journey.sh")).unwrap();
for needle in [
        "axolotl-qlora",
        "Qwen/Qwen2.5-0.5B-Instruct",
        "llama3",
        "adapter: qlora",
        "load_in_4bit: true",
        "sequence_len: 4096",
        "lora_r: 32",
        "examples/llama-3/qlora.yml",
        "refuse:train-base",
        "refuse:adapter",
        "refuse:seat",
        "refuse:tokenizer",
        "adapter_config.json",
        "model.safetensors",
        "GGUF",
        "SKIP live train",
        "SKIP live convert",
        "SKIP live seat",
        "READY_FOR_LIVE_TEST: no",
        "--dequant",
        "CELL_SEAT_LIVE",
        "CELL_TRAIN_LIVE",
        "examples/estate.yaml",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "AXOLOTL_QLORA_PHASE",
    ] {
        assert!(script.contains(needle), "axolotl-qlora-journey missing {needle}");
    }
let bad = script
        .find("-- Qwen-shaped merged tokenizer is refuse:tokenizer --")
        .expect("missing refuse:tokenizer step");
let replace = script
        .find("-- replace the broken tokenizer with the good merged stub --")
        .expect("missing good-stub replace");
let happy = script
        .find("-- gguf-convert prints convert_hf_to_gguf.py --")
        .expect("missing happy-path convert");
assert!(
        bad < replace && replace < happy,
        "refuse:tokenizer must run before the good stubs and the convert print"
    );
assert!(!script.contains("READY_FOR_LIVE_TEST: yes"));
let shells_out = script.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#')
            || trimmed.starts_with("echo")
            || trimmed.starts_with("grep")
            || trimmed.starts_with("if grep")
            || trimmed.starts_with("if ! grep")
            || trimmed.starts_with("require ")
            || trimmed.contains("[[ -e")
        {
            return false;
        }
        trimmed.contains("axolotl ")
            || trimmed.contains("convert_hf_to_gguf.py")
            || trimmed.contains("ollama ")
    }
);
assert!(
        !shells_out,
        "axolotl-qlora-journey must not shell out to axolotl, llama.cpp, or ollama"
    );
let chain = std::fs::read_to_string(root.join("scripts/uniqueness-axolotl.sh")).unwrap();
assert!(chain.contains("AXOLOTL_QLORA_PHASE=prepare"));
assert!(chain.contains("AXOLOTL_QLORA_PHASE=seat"));
assert!(chain.contains("make axolotl-qlora-journey"));
assert!(chain.contains("set -euo pipefail"));
assert!(chain.contains("SKIP live train"));
assert!(!chain.contains("READY_FOR_LIVE_TEST: yes"));
assert!(
        chain.find("AXOLOTL_QLORA_PHASE=prepare") < chain.find("AXOLOTL_QLORA_PHASE=seat"),
        "uniqueness-axolotl must run prepare-assert before the seat print"
    );
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
assert!(journey.contains("## 11. Axolotl QLoRA — popular-config print journey"));
assert!(journey.contains("make axolotl-qlora-journey"));
assert!(journey.contains("make uniqueness-axolotl"));
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
assert!(train.contains("`make axolotl-qlora-journey`"));
assert!(train.contains("`make uniqueness-axolotl`"));
assert!(train.contains("adapter: qlora"));
assert!(train.contains("load_in_4bit: true"));
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains("make axolotl-qlora-journey"));
assert!(help.contains("make uniqueness-axolotl"));
assert!(help.contains("section 11"));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));

for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("axolotl-qlora-journey") && !body.contains("uniqueness-axolotl"),
            "{rel} must not run the axolotl journey"
        );
    }
}

#[test]
fn unsloth_qlora_journey_stays_print_only_and_off_smoke() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
for target in ["unsloth-qlora-journey:", "uniqueness-unsloth:"] {
        assert!(
            makefile.lines().any(|line| line.trim() == target),
            "Makefile missing {target}"
        );
    }
assert!(makefile.contains("scripts/unsloth-qlora-journey.sh"));
assert!(makefile.contains("scripts/uniqueness-unsloth.sh"));
let phony = makefile.lines().next().unwrap_or("");
assert!(
        phony.contains("unsloth-qlora-journey") && phony.contains("uniqueness-unsloth"),
        "unsloth journey targets must be phony"
    );
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("unsloth-qlora-journey") && !gate90.contains("uniqueness-unsloth"),
        "gate-90 must not run the unsloth journey: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("unsloth-qlora-journey") && !smoke.contains("uniqueness-unsloth"),
        "smoke must not run the unsloth journey: {smoke}"
    );
let script = std::fs::read_to_string(root.join("scripts/unsloth-qlora-journey.sh")).unwrap();
for needle in [
        "unsloth-qlora",
        "Qwen/Qwen2.5-0.5B-Instruct",
        "llama3",
        "UNSLOTH.md",
        "adapter_model.safetensors",
        "save_pretrained_merged",
        "merged_16bit",
        "status: optional",
        "refuse:train-base",
        "refuse:adapter",
        "refuse:seat",
        "refuse:tokenizer",
        "local-seat --adapter",
        "adapter_config.json",
        "model.safetensors",
        "GGUF",
        "SKIP live train",
        "SKIP live convert",
        "SKIP live seat",
        "READY_FOR_LIVE_TEST: no",
        "CELL_SEAT_LIVE",
        "CELL_TRAIN_LIVE",
        "examples/estate.yaml",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "UNSLOTH_QLORA_PHASE",
        "does not call Unsloth",
    ] {
        assert!(script.contains(needle), "unsloth-qlora-journey missing {needle}");
    }
assert!(
        script.contains("must not invent a script") && script.contains("train_unsloth.py"),
        "journey must forbid a training script"
    );
let bad = script
        .find("-- Qwen-shaped merged tokenizer is refuse:tokenizer --")
        .expect("missing refuse:tokenizer step");
let replace = script
        .find("-- replace the broken tokenizer with the good merged stub --")
        .expect("missing good-stub replace");
let happy = script
        .find("-- gguf-convert prints convert_hf_to_gguf.py --")
        .expect("missing happy-path convert");
assert!(
        bad < replace && replace < happy,
        "refuse:tokenizer must run before the good stubs and the convert print"
    );
let shape = script
        .find("-- adapter_config.json without adapter_model.safetensors is refuse:adapter --")
        .expect("missing wrong-shape step");
let seat_adapter = script
        .find("-- local-seat --adapter stays refuse:adapter --")
        .expect("missing local-seat --adapter refuse");
assert!(
        shape < seat_adapter,
        "wrong-shape refuse must run, and local-seat --adapter stays refuse:adapter"
    );
assert!(!script.contains("READY_FOR_LIVE_TEST: yes"));
let shells_out = script.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#')
            || trimmed.starts_with("echo")
            || trimmed.starts_with("grep")
            || trimmed.starts_with("if grep")
            || trimmed.starts_with("if ! grep")
            || trimmed.starts_with("require ")
            || trimmed.contains("[[ -e")
        {
            return false;
        }
        trimmed.contains("unsloth ")
            || trimmed.contains("convert_hf_to_gguf.py")
            || trimmed.contains("ollama ")
    }
);
assert!(
        !shells_out,
        "unsloth-qlora-journey must not shell out to Unsloth, llama.cpp, or ollama"
    );
let chain = std::fs::read_to_string(root.join("scripts/uniqueness-unsloth.sh")).unwrap();
assert!(chain.contains("UNSLOTH_QLORA_PHASE=prepare"));
assert!(chain.contains("UNSLOTH_QLORA_PHASE=seat"));
assert!(chain.contains("make unsloth-qlora-journey"));
assert!(chain.contains("set -euo pipefail"));
assert!(chain.contains("SKIP live train"));
assert!(!chain.contains("READY_FOR_LIVE_TEST: yes"));
assert!(
        chain.find("UNSLOTH_QLORA_PHASE=prepare") < chain.find("UNSLOTH_QLORA_PHASE=seat"),
        "uniqueness-unsloth must run prepare-assert before the seat print"
    );
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
assert!(journey.contains("## 12. Unsloth QLoRA — optional NEXT print journey"));
assert!(journey.contains("make unsloth-qlora-journey"));
assert!(journey.contains("make uniqueness-unsloth"));
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
assert!(train.contains("`make unsloth-qlora-journey`"));
assert!(train.contains("`make uniqueness-unsloth`"));
assert!(train.contains("save_pretrained_merged"));
assert!(train.contains("merged_16bit"));
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains("make unsloth-qlora-journey"));
assert!(help.contains("make uniqueness-unsloth"));
assert!(help.contains("section 12"));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));

for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("unsloth-qlora-journey") && !body.contains("uniqueness-unsloth"),
            "{rel} must not run the unsloth journey"
        );
    }
}

#[test]
fn axolotl_lora_journey_stays_print_only_and_off_smoke() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
for target in ["axolotl-lora-journey:", "uniqueness-axolotl-lora:"] {
        assert!(
            makefile.lines().any(|line| line.trim() == target),
            "Makefile missing {target}"
        );
    }
assert!(makefile.contains("scripts/axolotl-lora-journey.sh"));
assert!(makefile.contains("scripts/uniqueness-axolotl-lora.sh"));
let phony = makefile.lines().next().unwrap_or("");
assert!(
        phony.contains("axolotl-lora-journey") && phony.contains("uniqueness-axolotl-lora"),
        "axolotl lora journey targets must be phony"
    );
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("axolotl-lora-journey") && !gate90.contains("uniqueness-axolotl-lora"),
        "gate-90 must not run the axolotl lora journey: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("axolotl-lora-journey") && !smoke.contains("uniqueness-axolotl-lora"),
        "smoke must not run the axolotl lora journey: {smoke}"
    );
let script = std::fs::read_to_string(root.join("scripts/axolotl-lora-journey.sh")).unwrap();
for needle in [
        "axolotl-lora",
        "Qwen/Qwen2.5-0.5B-Instruct",
        "llama3",
        "adapter: lora",
        "load_in_8bit: false",
        "load_in_4bit: false",
        "sequence_len: 2048",
        "lora_r: 16",
        "examples/llama-3/lora-1b.yml",
        "refuse:train-base",
        "refuse:adapter",
        "refuse:seat",
        "refuse:tokenizer",
        "adapter_config.json",
        "model.safetensors",
        "GGUF",
        "SKIP live train",
        "SKIP live convert",
        "SKIP live seat",
        "READY_FOR_LIVE_TEST: no",
        "without --dequant",
        "CELL_SEAT_LIVE",
        "CELL_TRAIN_LIVE",
        "examples/estate.yaml",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "AXOLOTL_LORA_PHASE",
    ] {
        assert!(script.contains(needle), "axolotl-lora-journey missing {needle}");
    }
assert!(
        !script.contains("require \"adapter: qlora\"")
            && !script.contains("require \"load_in_4bit: true\""),
        "axolotl-lora-journey must not require the 4-bit card"
    );
assert!(
        script.contains("must not write adapter qlora")
            && script.contains("must not write load_in_4bit true")
            && script.contains("must not print --dequant"),
        "axolotl-lora-journey must keep the 4-bit card off this prepare"
    );
let bad = script
        .find("-- Qwen-shaped merged tokenizer is refuse:tokenizer --")
        .expect("missing refuse:tokenizer step");
let replace = script
        .find("-- replace the broken tokenizer with the good merged stub --")
        .expect("missing good-stub replace");
let happy = script
        .find("-- gguf-convert prints convert_hf_to_gguf.py --")
        .expect("missing happy-path convert");
assert!(
        bad < replace && replace < happy,
        "refuse:tokenizer must run before the good stubs and the convert print"
    );
assert!(!script.contains("READY_FOR_LIVE_TEST: yes"));
let shells_out = script.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#')
            || trimmed.starts_with("echo")
            || trimmed.starts_with("grep")
            || trimmed.starts_with("if grep")
            || trimmed.starts_with("if ! grep")
            || trimmed.starts_with("require ")
            || trimmed.contains("[[ -e")
        {
            return false;
        }
        trimmed.contains("axolotl ")
            || trimmed.contains("convert_hf_to_gguf.py")
            || trimmed.contains("ollama ")
    }
);
assert!(
        !shells_out,
        "axolotl-lora-journey must not shell out to axolotl, llama.cpp, or ollama"
    );
let chain = std::fs::read_to_string(root.join("scripts/uniqueness-axolotl-lora.sh")).unwrap();
assert!(chain.contains("AXOLOTL_LORA_PHASE=prepare"));
assert!(chain.contains("AXOLOTL_LORA_PHASE=seat"));
assert!(chain.contains("make axolotl-lora-journey"));
assert!(chain.contains("set -euo pipefail"));
assert!(chain.contains("SKIP live train"));
assert!(!chain.contains("make axolotl-qlora-journey;") && !chain.contains("axolotl-qlora-journey;"));
assert!(
        chain.find("AXOLOTL_LORA_PHASE=prepare") < chain.find("AXOLOTL_LORA_PHASE=seat"),
        "uniqueness-axolotl-lora must run prepare-assert before the seat print"
    );
assert!(!chain.contains("READY_FOR_LIVE_TEST: yes"));
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
assert!(journey.contains("## 13. Axolotl LoRA — popular-config print journey"));
assert!(journey.contains("make axolotl-lora-journey"));
assert!(journey.contains("make uniqueness-axolotl-lora"));
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
assert!(train.contains("`make axolotl-lora-journey`"));
assert!(train.contains("`make uniqueness-axolotl-lora`"));
assert!(train.contains("adapter: lora"));
assert!(train.contains("load_in_4bit: false"));
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains("make axolotl-lora-journey"));
assert!(help.contains("make uniqueness-axolotl-lora"));
assert!(help.contains("section 13"));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));

let status = std::fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
assert!(status.contains("make axolotl-lora-journey"));
assert!(status.contains("make uniqueness-axolotl-lora"));
assert!(status.contains("does not invent a live PASS"));
assert!(status.contains("make mlx-lm-lora-journey"));
assert!(status.contains("make uniqueness-mlx"));
assert!(status.contains("operator section 16"));
assert!(!status.contains("READY_FOR_LIVE_TEST: yes"));
for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("axolotl-lora-journey") && !body.contains("uniqueness-axolotl-lora"),
            "{rel} must not run the axolotl lora journey"
        );
    }
}

#[test]
fn unsloth_lora_journey_stays_print_only_and_off_smoke() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
for target in ["unsloth-lora-journey:", "uniqueness-unsloth-lora:"] {
        assert!(
            makefile.lines().any(|line| line.trim() == target),
            "Makefile missing {target}"
        );
    }
assert!(makefile.contains("scripts/unsloth-lora-journey.sh"));
assert!(makefile.contains("scripts/uniqueness-unsloth-lora.sh"));
let phony = makefile.lines().next().unwrap_or("");
assert!(
        phony.contains("unsloth-lora-journey") && phony.contains("uniqueness-unsloth-lora"),
        "unsloth lora journey targets must be phony"
    );
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("unsloth-lora-journey") && !gate90.contains("uniqueness-unsloth-lora"),
        "gate-90 must not run the unsloth lora journey: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("unsloth-lora-journey") && !smoke.contains("uniqueness-unsloth-lora"),
        "smoke must not run the unsloth lora journey: {smoke}"
    );
let script = std::fs::read_to_string(root.join("scripts/unsloth-lora-journey.sh")).unwrap();
for needle in [
        "unsloth-lora",
        "Qwen/Qwen2.5-0.5B-Instruct",
        "llama3",
        "UNSLOTH.md",
        "Unsloth LoRA handoff",
        "does not write load_in_4bit",
        "refuse:official-scale",
        "refuse:dataset",
        "adapter_model.safetensors",
        "save_pretrained_merged",
        "merged_16bit",
        "save_method = \\\"lora\\\"",
        "status: optional",
        "refuse:train-base",
        "refuse:adapter",
        "refuse:seat",
        "refuse:tokenizer",
        "local-seat --adapter",
        "SKIP live train",
        "SKIP live convert",
        "SKIP live seat",
        "READY_FOR_LIVE_TEST: no",
        "CELL_SEAT_LIVE",
        "CELL_TRAIN_LIVE",
        "examples/estate.yaml",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "UNSLOTH_LORA_PHASE",
        "does not call Unsloth",
    ] {
        assert!(script.contains(needle), "unsloth-lora-journey missing {needle}");
    }
assert!(!script.contains("READY_FOR_LIVE_TEST: yes"));
assert!(
        !script.contains("make unsloth-qlora-journey") && !script.contains("make uniqueness-unsloth"),
        "the LoRA journey must not invoke the QLoRA chain"
    );
let chain = std::fs::read_to_string(root.join("scripts/uniqueness-unsloth-lora.sh")).unwrap();
assert!(chain.contains("UNSLOTH_LORA_PHASE=prepare"));
assert!(chain.contains("UNSLOTH_LORA_PHASE=seat"));
assert!(chain.contains("make unsloth-lora-journey"));
assert!(chain.contains("set -euo pipefail"));
assert!(!chain.contains("make unsloth-qlora-journey\n") && chain.contains("Does not run make unsloth-qlora-journey"));
assert!(chain.contains("make uniqueness-unsloth"));
assert!(
        chain.find("UNSLOTH_LORA_PHASE=prepare") < chain.find("UNSLOTH_LORA_PHASE=seat"),
        "uniqueness-unsloth-lora must run prepare-assert before the seat print"
    );
assert!(
        !chain.contains("UNSLOTH_QLORA_PHASE") && !chain.contains("make axolotl-lora-journey\n"),
        "uniqueness-unsloth-lora must not run the other chains"
    );
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
assert!(journey.contains("## 14. Unsloth LoRA — optional NEXT print journey"));
assert!(journey.contains("make unsloth-lora-journey"));
assert!(journey.contains("make uniqueness-unsloth-lora"));
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
assert!(train.contains("`make unsloth-lora-journey`"));
assert!(train.contains("`make uniqueness-unsloth-lora`"));
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains("make unsloth-lora-journey"));
assert!(help.contains("make uniqueness-unsloth-lora"));
assert!(help.contains("section 14"));

for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("unsloth-lora-journey") && !body.contains("uniqueness-unsloth-lora"),
            "{rel} must not run the unsloth lora journey"
        );
    }
}

#[test]
fn mlx_lm_lora_journey_stays_print_only_and_off_smoke() {
let root = repo_root();
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
for target in ["mlx-lm-lora-journey:", "uniqueness-mlx:"] {
        assert!(
            makefile.lines().any(|line| line.trim() == target),
            "Makefile missing {target}"
        );
    }
assert!(makefile.contains("scripts/mlx-lm-lora-journey.sh"));
assert!(makefile.contains("scripts/uniqueness-mlx.sh"));
let phony = makefile.lines().next().unwrap_or("");
assert!(
        phony.contains("mlx-lm-lora-journey") && phony.contains("uniqueness-mlx"),
        "mlx journey targets must be phony"
    );
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !gate90.contains("mlx-lm-lora-journey") && !gate90.contains("uniqueness-mlx"),
        "gate-90 must not run the mlx journey: {gate90}"
    );
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(
        !smoke.contains("mlx-lm-lora-journey") && !smoke.contains("uniqueness-mlx"),
        "smoke must not run the mlx journey: {smoke}"
    );
let script = std::fs::read_to_string(root.join("scripts/mlx-lm-lora-journey.sh")).unwrap();
for needle in [
        "mlx-lm-lora",
        "apple-silicon",
        "Qwen/Qwen2.5-0.5B-Instruct",
        "llama3",
        "MLX.md",
        "adapters.safetensors",
        "adapter_model.safetensors",
        "mlx_lm.fuse",
        "--export-gguf",
        "ggml-model-f16.gguf",
        "fused_model",
        "status: optional",
        "refuse:host",
        "refuse:train-base",
        "refuse:adapter",
        "refuse:seat",
        "refuse:official-scale",
        "refuse:dataset",
        "local-seat --adapter",
        "adapter_config.json",
        "SKIP live train",
        "SKIP live convert",
        "SKIP live seat",
        "READY_FOR_LIVE_TEST: no",
        "CELL_SEAT_LIVE",
        "CELL_TRAIN_LIVE",
        "examples/estate.yaml",
        "Do not add to make smoke, make gate-90, or GitHub Actions",
        "MLX_LM_LORA_PHASE",
        "does not call mlx-lm",
    ] {
        assert!(script.contains(needle), "mlx-lm-lora-journey missing {needle}");
    }
assert!(!script.contains("READY_FOR_LIVE_TEST: yes"));
let shape = script
        .find("-- PEFT adapter_model.safetensors is the wrong shape --")
        .expect("missing wrong-shape step");
let fused = script
        .find("-- fused MLX directory is refuse:adapter and refuse:seat --")
        .expect("missing fused refuse");
let seat = script
        .find("-- local-seat prints ollama create for the GGUF stub --")
        .expect("missing seat print");
assert!(
        shape < fused && fused < seat,
        "wrong-shape and fused refuses must run before the seat print"
    );
let shells_out = script.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#')
            || trimmed.starts_with("echo")
            || trimmed.starts_with("grep")
            || trimmed.starts_with("if grep")
            || trimmed.starts_with("if ! grep")
            || trimmed.starts_with("require ")
            || trimmed.contains("[[ -e")
        {
            return false;
        }
        trimmed.contains("mlx_lm ")
            || trimmed.contains("mlx_lm.lora")
            || trimmed.contains("mlx_lm.fuse ")
            || trimmed.contains("convert_hf_to_gguf.py")
            || trimmed.contains("ollama ")
    }
);
assert!(
        !shells_out,
        "mlx-lm-lora-journey must not shell out to mlx-lm, llama.cpp, or ollama"
    );
let chain = std::fs::read_to_string(root.join("scripts/uniqueness-mlx.sh")).unwrap();
assert!(chain.contains("MLX_LM_LORA_PHASE=prepare"));
assert!(chain.contains("MLX_LM_LORA_PHASE=seat"));
assert!(chain.contains("make mlx-lm-lora-journey"));
assert!(chain.contains("set -euo pipefail"));
assert!(chain.contains("SKIP live train"));
assert!(!chain.contains("READY_FOR_LIVE_TEST: yes"));
assert!(
        chain.find("MLX_LM_LORA_PHASE=prepare") < chain.find("MLX_LM_LORA_PHASE=seat"),
        "uniqueness-mlx must run prepare-assert before the seat print"
    );
let journey = std::fs::read_to_string(root.join("docs/operator-enrich-journeys.md")).unwrap();
assert!(journey.contains("## 16. mlx-lm LoRA — optional Apple Silicon print journey"));
assert!(journey.contains("make mlx-lm-lora-journey"));
assert!(journey.contains("make uniqueness-mlx"));
let train = std::fs::read_to_string(root.join("docs/TRAIN-ENRICH.md")).unwrap();
assert!(train.contains("`make mlx-lm-lora-journey`"));
assert!(train.contains("`make uniqueness-mlx`"));
assert!(train.contains("mlx_lm.fuse"));
assert!(train.contains("--export-gguf"));
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains("make mlx-lm-lora-journey"));
assert!(help.contains("make uniqueness-mlx"));
assert!(help.contains(
        "print-only Apple Silicon mlx-lm LoRA journey (operator section 16)"
    ));
assert!(help.contains("section 16"));
let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
assert!(
        readme.contains("`make mlx-lm-lora-journey`")
            && readme.contains("`make uniqueness-mlx`")
            && readme.contains(
                "print-only Apple Silicon mlx-lm LoRA journey (operator section 16)"
            ),
        "{readme}"
    );
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(
        makefile.contains(
            "print-only Apple Silicon mlx-lm LoRA journey (operator section 16)"
        ) && makefile.contains(
            "print-only chain of that Apple Silicon mlx-lm LoRA journey (operator section 16)"
        ),
        "Makefile comments must name the mlx journey"
    );
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));
let checklist = std::fs::read_to_string(root.join("scripts/purpose-build-checklist.sh")).unwrap();
assert!(checklist.contains("make mlx-lm-lora-journey"));
assert!(checklist.contains("This checklist does not run it."));
assert!(checklist.contains("Not native MLX."));


for rel in [
        "scripts/smoke.sh",
        "scripts/day90-gate.sh",
        ".github/workflows/ci.yml",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !body.contains("mlx-lm-lora-journey") && !body.contains("uniqueness-mlx"),
            "{rel} must not run the mlx journey"
        );
    }
}

#[test]
fn deepseek_r1_distill_journey_help_and_locks_stay_print_only() {
let root = repo_root();
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains(
        "make deepseek-r1-distill-journey is the print-only DeepSeek-R1-Distill chat QLoRA journey (operator section 19)"
    ));
assert!(help.contains("make uniqueness-deepseek is the print-only chain of that journey"));
assert!(help.contains("make deepseek-r1-distill-lora-journey"));
assert!(help.contains("make uniqueness-deepseek-lora"));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(makefile.contains("DEEPSEEK_CARD=llamafactory-qlora bash scripts/deepseek-r1-distill-journey.sh"));
assert!(makefile.contains("DEEPSEEK_CARD=llamafactory-lora bash scripts/deepseek-r1-distill-journey.sh"));
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(!gate90.contains("deepseek-r1-distill-journey"));
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(!smoke.contains("deepseek-r1-distill-journey"));
let script = std::fs::read_to_string(root.join("scripts/deepseek-r1-distill-journey.sh")).unwrap();
assert!(script.contains("^template: ${TEMPLATE}$"));
assert!(script.contains("deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B"));
assert!(script.contains("examples/fixtures/deepseek-r1-distill.pack.json"));
assert!(script.contains("examples/fixtures/deepseek-r1-distill-lora.pack.json"));
assert!(!script.contains("READY_FOR_LIVE_TEST: yes"));
assert!(!script.to_ascii_lowercase().contains("kimi"));
}

#[test]
fn glm4_chat_journey_help_and_locks_stay_print_only() {
let root = repo_root();
let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
assert!(help.contains(
        "make glm4-chat-journey is the print-only GLM-4 Chat QLoRA journey (operator section 20)"
    ));
assert!(help.contains("make uniqueness-glm is the print-only chain of that journey"));
assert!(help.contains("make glm4-chat-lora-journey"));
assert!(help.contains("make uniqueness-glm-lora"));
assert!(help.contains("zai-org/glm-4-9b-chat"));
assert!(!help.contains("READY_FOR_LIVE_TEST: yes"));
assert!(!help.to_ascii_lowercase().contains("kimi"));
let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
assert!(makefile.contains("GLM_CARD=llamafactory-qlora bash scripts/glm4-chat-journey.sh"));
assert!(makefile.contains("GLM_CARD=llamafactory-lora bash scripts/glm4-chat-journey.sh"));
let gate90 = makefile
        .split("\ngate-90:\n")
        .nth(1)
        .expect("gate-90 recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(!gate90.contains("glm4-chat-journey"));
let smoke = makefile
        .split("\nsmoke:\n")
        .nth(1)
        .expect("smoke recipe")
        .split("\n\n")
        .next()
        .unwrap();
assert!(!smoke.contains("glm4-chat-journey"));
let script = std::fs::read_to_string(root.join("scripts/glm4-chat-journey.sh")).unwrap();
assert!(script.contains("^template: ${TEMPLATE}$"));
assert!(script.contains("zai-org/glm-4-9b-chat"));
assert!(script.contains("examples/fixtures/glm4-chat.pack.json"));
assert!(script.contains("examples/fixtures/glm4-chat-lora.pack.json"));
assert!(!script.contains("READY_FOR_LIVE_TEST: yes"));
assert!(!script.to_ascii_lowercase().contains("kimi"));
}
