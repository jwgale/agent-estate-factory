//! Safety invariants that stay true across feature work.
//!
//! These checks do not lock tip SHAs, pull-request numbers, or changelog heads.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn posix_cksum(data: &[u8]) -> u32 {
    fn eat(crc: u32, byte: u8) -> u32 {
        let mut c = crc ^ (u32::from(byte) << 24);
        for _ in 0..8 {
            if c & 0x8000_0000 != 0 {
                c = (c << 1) ^ 0x04c1_1db7;
            } else {
                c <<= 1;
            }
        }
        c
    }
    let mut crc = 0u32;
    for &byte in data {
        crc = eat(crc, byte);
    }
    let mut len = data.len();
    while len != 0 {
        crc = eat(crc, (len & 0xff) as u8);
        len >>= 8;
    }
    !crc
}

fn is_token_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `READY_FOR_LIVE_TEST` set to yes, true, or 1.
///
/// The value match is case-insensitive. A colon, equals, or the word "is"
/// must sit next to the key (markdown wrappers allowed). Historical lines that
/// mention the setting without assigning it are listed in `READY_YES_ALLOWLIST`.
fn sets_ready_yes(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let key = "ready_for_live_test";
    let bytes = lower.as_bytes();
    let mut start = 0;
    while let Some(rel) = lower[start..].find(key) {
        let mut j = start + rel + key.len();
        while j < bytes.len() && matches!(bytes[j], b'`' | b'*' | b'_' | b' ' | b'\t') {
            j += 1;
        }
        let mut separated = false;
        if j < bytes.len() && matches!(bytes[j], b'=' | b':') {
            separated = true;
            j += 1;
        } else if lower[j..].starts_with("is")
            && (j + 2 == bytes.len() || !bytes[j + 2].is_ascii_alphanumeric())
        {
            separated = true;
            j += 2;
        }
        if separated {
            while j < bytes.len() && matches!(bytes[j], b' ' | b'\t' | b'`' | b'*' | b'"' | b'\'' | b'_')
            {
                j += 1;
            }
            for value in ["yes", "true", "1"] {
                if lower[j..].starts_with(value) {
                    let end = j + value.len();
                    if end == bytes.len() || !bytes[end].is_ascii_alphanumeric() {
                        return true;
                    }
                }
            }
        }
        start += rel + key.len();
    }
    false
}

fn scanned_file(name: &str, path: &Path) -> bool {
    if name == "Makefile" || name == "makefile" || name.starts_with(".env") {
        return true;
    }
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("md" | "yaml" | "yml" | "toml" | "sh" | "json")
    )
}

fn walk_docs_and_config(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == ".git" || name == "target" {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if name == "CHANGELOG.md" {
                continue;
            }
            if scanned_file(&name, &path) {
                out.push(path);
            }
        }
    }
    out
}

fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}


const READY_YES_ALLOWLIST: &[(&str, &str)] = &[
    ("docs/DAY90-PLUS.md", "`READY_FOR_LIVE_TEST` is yes only for a concrete command in"),
    ("docs/NORTH-STAR.md", "`READY_FOR_LIVE_TEST` is yes only for a concrete command on that page with no paste yet. Recorded rows stay recorded: frontier `pong`, 5090-class probes, 5090-class specialist `Pong`, Mac `probes --live`, and Mac `estate specialist` `\"completion\": \"Pong\"` (reason `compat completion`, MacBook Air, tip `2ab78a4`). `READY_FOR_LIVE_TEST` for that Mac command is no. Native MLX stays a stub. SKIP exits 0."),
    ("scripts/uniqueness-prove-checklist.sh", "coda_forbid \"READY_FOR_LIVE_TEST: yes\""),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/llamafactory-lora-qwen3/NEXT.md\" \"$WORKDIR/llamafactory-lora-qwen3/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/llamafactory-qlora-qwen3/NEXT.md\" \"$WORKDIR/llamafactory-qlora-qwen3/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/phi3-lora/NEXT.md\" \"$WORKDIR/phi3-lora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/llama32-qlora/NEXT.md\" \"$WORKDIR/llama32-qlora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/gemma2-qlora/NEXT.md\" \"$WORKDIR/gemma2-qlora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/gemma2-lora/NEXT.md\" \"$WORKDIR/gemma2-lora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/mistral-qlora/NEXT.md\" \"$WORKDIR/mistral-qlora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/mistral-lora/NEXT.md\" \"$WORKDIR/mistral-lora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/qwen3-qlora/NEXT.md\" \"$WORKDIR/qwen3-qlora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/qwen25-qlora/NEXT.md\" \"$WORKDIR/qwen25-qlora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/qwen25-lora/NEXT.md\" \"$WORKDIR/qwen25-lora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/deepseek-r1-distill-qlora/NEXT.md\" \"$WORKDIR/deepseek-r1-distill-qlora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/deepseek-r1-distill-lora/NEXT.md\" \"$WORKDIR/deepseek-r1-distill-lora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/glm4-chat-qlora/NEXT.md\" \"$WORKDIR/glm4-chat-qlora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/glm4-chat-lora/NEXT.md\" \"$WORKDIR/glm4-chat-lora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/qwen3-lora/NEXT.md\" \"$WORKDIR/qwen3-lora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/llama32-lora/NEXT.md\" \"$WORKDIR/llama32-lora/PREPARE.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/unsloth/NEXT.md\"; then"),
    ("scripts/train-prepare.sh", "if grep -q \"READY_FOR_LIVE_TEST: yes\" \"$WORKDIR/mlx/NEXT.md\"; then"),
    ("scripts/purpose-build-checklist.sh", "coda_forbid \"READY_FOR_LIVE_TEST: yes\""),
];
const LIVE_PASS_LINE_ALLOWLIST: &[(&str, &str)] = &[
    ("docs/OPERATOR-DAY.md", "`make enrich-prepare` uses `examples/fixtures/specialist-overnight.pack.json` and writes both drivers under `/tmp/cell-one-enrich-prepare` (or `$TMPDIR`). The copy of the estate sets `params.model: llama3`, so the Modelfile says `FROM llama3`. You also get `NEXT.md` with the `ollama create` line, a portable manifest, `estate enrich list`, and an `import-prepared` binding proposal. The script then runs `estate enrich apply-proposal`, `estate plan`, and `estate apply --require-plan` on that seated copy. Apply without `--require-plan` leaves the lab copy unchanged. It does not call Ollama. `examples/estate.yaml` stays unchanged. `make enrich-live-prove` is the opt-in that does call `ollama create` when the seat is up. `make train-prepare` writes LLaMA-Factory LoRA and QLoRA recipes and Axolotl LoRA and QLoRA recipes on another throwaway directory and prints `SKIP live train`. It does not run either trainer. `make qlora-journey` prints the Qwen QLoRA ladder (Target C) and checks the `llamafactory-qlora` prepare artifacts. It does not train, convert, or promote. `make lora-journey` prints the Qwen LoRA ladder (Target A) and checks the `llamafactory-lora` prepare artifacts. It does not train, merge, convert, or promote. `make seat-journey` prints the Target C merge, convert, seat, and import lines after fixture stubs stand in for the merged export and the GGUF. It does not train, convert, or create a model. When an SLM fits mid-software-build, or on demand, the same print-only entry is `make purpose-build-journey`. Print-only purpose-build on demand is `make purpose-build-journey`. It runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. It does not train, convert, seat, promote, or apply. It is not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no. The re-prove card stays `make uniqueness-prove-checklist`. The recorded Target C PASS in [`LIVE-PROBES.md`](LIVE-PROBES.md) stays the only live uniqueness prove. Prepare: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md) sections 15–20."),
    ("docs/GATE-90.md", "| Frontier specialist (grok-4.7) | green | Live PASS recorded. Mock-locked. Sacred Cyera and Rust classroom refuse before POST, same as local, with no invented completion. A SKU `CELL_FRONTIER_MODEL` in that same request still refuses as sacred. No key in CI. `READY_FOR_LIVE_TEST` no |"),
    ("docs/GATE-90.md", "| Frontier `grok-4.7` live PASS | Recorded. `READY_FOR_LIVE_TEST` no. Not required for `make gate-90`. |"),
    ("docs/DAY90-PLUS.md", "| 5090-class GPU | `probes --live` PASS and `estate specialist --driver ollama` `Pong` are recorded. Host class is `consumer-nvidia` or `rented-nvidia`. | Not native MLX. Not a binding id. Not required for the gate. |"),
    ("docs/DAY90-PLUS.md", "| Mac probes | Ollama-on-Mac `probes --live` PASS. | That GET is not the specialist complete. |"),
    ("docs/CELL-ONE-STATUS.md", "An operator ran that ladder outside the factory on a Linux 5090-class host on 2026-09-23. The recorded PASS is [Target C live uniqueness (5090-class)](LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). The factory did not train, convert, shell out to ollama, or promote. That prove is not in `make smoke`, `make gate-90`, or GitHub Actions, and it is not native MLX. `examples/estate.yaml` stayed hash-locked. `READY_FOR_LIVE_TEST`: no."),
    ("docs/CELL-ONE-STATUS.md", "#30 recorded the live PASS (`completion` `pong`, reason `frontier completion`) and set `READY_FOR_LIVE_TEST` back to no. The mixed fixture validates and dry-runs with no POST. Local `--driver ollama` down or unset does not call frontier."),
    ("docs/CELL-ONE-STATUS.md", "#35. `estate specialist --driver frontier` already refused without `XAI_API_KEY`. Stderr names `CELL_FRONTIER_MODEL` and `CELL_FRONTIER_ENDPOINT` for a missing key and for a hardware SKU in the model id. The SKU still refuses before any POST. `docs/GATE-90.md` and `docs/DAY90-PLUS.md` separate green factory checks, recorded live proofs, and parked rows. At #35, Mac specialist complete was optional and not recorded. It is now a recorded PASS (`Pong` on the MacBook Air). Native MLX stays a stub. Cloud-agent spawn stays off."),
    ("docs/CELL-ONE-STATUS.md", "`docs/GATE-90.md` stops calling the Ollama complete path ready. The mock completion stays green. The 5090 `Pong` stays recorded. At #53, Mac complete was still unrecorded. It is now a recorded PASS (`\"completion\": \"Pong\"` on the MacBook Air, tip `2ab78a4`). `mlx`, `vllm`, and `trt` refuse a frontier POST and are not live-ok. `docs/DAY90-PLUS.md` parks vLLM and TRT with the other stubs. `READY_FOR_LIVE_TEST` for the Mac command is no."),
    ("docs/CELL-ONE-STATUS.md", "Mac `estate specialist` complete is a recorded **PASS** on the MacBook Air (`\"completion\": \"Pong\"`, reason `compat completion`, tip `2ab78a4`). [`LIVE-PROBES.md`](LIVE-PROBES.md) keeps the copy-paste: `PATH` includes `~/.cargo/bin`, `CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434`, `CELL_LOCAL_MODEL=llama3`, then `estate specialist --driver ollama --prompt \"Reply with the single word pong.\"`. The 5090 `Pong` stays recorded. Native MLX stays a stub. `mlx`, `vllm`, and `trt` stay `not live-ok`. `READY_FOR_LIVE_TEST` for that Mac command is no."),
    ("docs/CELL-ONE-STATUS.md", "[`LIVE-PROBES.md`](LIVE-PROBES.md) holds the Mac `estate specialist` result. The pasted completion is **PASS** (`\"completion\": \"Pong\"`, reason `compat completion`, tip `2ab78a4`). `READY_FOR_LIVE_TEST` for that command is no. The 5090 `Pong` stays recorded. Native MLX stays a stub. `mlx`, `vllm`, and `trt` stay `not live-ok`."),
    ("docs/CELL-ONE-STATUS.md", "| #30 | Frontier live PASS was not on the hand-off page. | Recorded `pong` / `frontier completion`. READY no. Mixed `grok-4.7` dry-run. Local down does not POST frontier. |"),
    ("docs/CELL-ONE-STATUS.md", "| #53 | The gate called Ollama specialist complete ready, and the parking lot did not name vLLM or TRT, so a stub card could be read as a live hand-off. | The completion row is mock-locked. 5090 `Pong` stays recorded. Mac complete is a recorded PASS (`Pong`, MacBook Air, tip `2ab78a4`). vLLM and TRT are parked and not live-ok. READY no. |"),
    ("docs/CELL-ONE-STATUS.md", "| #55 | The live-probe page told the operator not to run Mac specialist complete, while that command was still unrecorded and both boxes were up. | The MacBook Air completion is a recorded PASS (`\"completion\": \"Pong\"`, reason `compat completion`, tip `2ab78a4`). READY no. The 5090 `Pong` stays recorded. |"),
    ("docs/CELL-ONE-STATUS.md", "| #57 | The live-probe page had the Mac specialist command and no place to write the result, so a later edit could mark PASS before a completion was pasted. | The result row is **PASS** (`\"completion\": \"Pong\"`). READY no. The 5090 `Pong` stays recorded. |"),
    ("docs/CELL-ONE-STATUS.md", "| target c uniqueness prove | The outside-factory 5090-class Target C ladder had no recorded PASS on the live-probes page. | PR #153 (`16cea97d56079a60c033c7a10468ddd7092a2ef1`). The recorded PASS is [Target C live uniqueness (5090-class)](LIVE-PROBES.md). The factory did not train, convert, shell out to ollama, or promote. READY no. |"),
    ("docs/CELL-ONE-STATUS.md", "| standing next estate | After `import-trained` (`trained_shape` `gguf`, `auto_apply=false`) the prove checklist did not name the plan, apply, and reconcile path, so that join was easy to misread as an apply or a promote. | PR #157 (`6a43ff12a91295739c5a9c8a8f1dc9cc9c084466`). `make uniqueness-prove-checklist` prints Standing next (estate). The proposal stays `auto_apply=false`. No promote and no auto-promote. It names `apply-proposal`, `plan`, `apply --require-plan`, and `reconcile` and does not execute them. The recorded PASS stays on LIVE-PROBES. The re-prove card is that same target. READY no. |"),
    ("docs/CELL-ONE-STATUS.md", "Mac specialist complete is a recorded **PASS** on the MacBook Air: `\"completion\": \"Pong\"`, reason `compat completion`, tip `2ab78a4`. `READY_FOR_LIVE_TEST` for that command is no. Details: [`LIVE-PROBES.md`](LIVE-PROBES.md)."),
    ("docs/LIVE-PROBES.md", "That MacBook Air run is **PASS** (`\"completion\": \"Pong\"`, reason `compat completion`)."),
    ("docs/LIVE-PROBES.md", "| Target C live uniqueness **PASS** | The factory training, converting, shelling out to ollama, or promoting. Not `estate probes --live`. Not the Mac `Pong` row. Not native MLX. Not `make smoke`, `make gate-90`, or Actions. |"),
    ("docs/NORTH-STAR.md", "Today's beachhead is curator packs, the specialist path, and `TrainEnrichDriver`. `estate enrich prepare` writes artifacts for the seated runtime (Ollama Modelfile today) and a portable manifest for a later trainer. `--all-drivers` writes every card the job allows. `estate enrich from-pack` prepares an accepted pack. Modelfile `FROM` is the seated model (`params.model` or a model-tag hint), never the binding id `local_slm`. `estate enrich list` reads `.cell/enrich`. `estate enrich import-prepared` writes a `local_slm` binding proposal. `estate enrich apply-proposal` stages it for `estate plan` and `estate apply --require-plan`. The source estate is written when that apply succeeds. The curator is Jason. `estate packs accept` writes curator edit instructions. Jason pastes them into the estate file. Promote stays refused. Train facilitation on this beachhead is `llamafactory-lora` for a LLaMA-Factory LoRA recipe with no quantization, `llamafactory-qlora` for the QLoRA recipe (train base separate from the Ollama seat tag), `axolotl-lora` for a bf16 Axolotl YAML, and `axolotl-qlora` for a 4-bit Axolotl YAML. `base_model` on both Axolotl cards is that same train base. Prepare writes the config, the operator runs `axolotl train` outside the factory, and `import-trained` returns the adapter through the existing apply-proposal loop. The factory still does not run the trainer. `--from-feed` copies dataset rows that are already under the cell state directory and does not download them. Target C is the Qwen / LLaMA-Factory QLoRA ladder: prepare, the external train and export lines, a printed GGUF convert, a printed Ollama create, and `import-trained`. Target A is the Qwen / LLaMA-Factory LoRA ladder: prepare, the external train and export lines, a printed merge, a printed GGUF convert, a printed Ollama create, and `import-trained`. `make seat-journey` prints the Target C merge, convert, seat, and import lines against fixture stubs. When an SLM fits mid-software-build, or on demand, the same print-only entry is `make purpose-build-journey`. `make purpose-build-journey` is the print-only purpose-build on-demand entry. It runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. It does not train, convert, seat, promote, or apply. It is not in `make smoke`, `make gate-90`, or GitHub Actions. The re-prove card stays `make uniqueness-prove-checklist`. The recorded Target C PASS in [`LIVE-PROBES.md`](LIVE-PROBES.md) stays the only live uniqueness prove. `READY_FOR_LIVE_TEST` stays no. Prepare: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md) sections 15–20."),
    ("docs/NORTH-STAR.md", "8. **`make purpose-build-journey`** — when an SLM fits mid-software-build, or on demand, this is the same print-only entry. Print-only purpose-build on demand. Runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. Does not train, convert, seat, promote, or apply. `READY_FOR_LIVE_TEST` stays no. The re-prove card stays `make uniqueness-prove-checklist`. The recorded Target C PASS in [`LIVE-PROBES.md`](LIVE-PROBES.md) stays the only live uniqueness prove. Off smoke, `gate-90`, and Actions. Walks: [`operator-enrich-journeys.md`](operator-enrich-journeys.md) sections 15–20."),
];

fn ready_yes_allowlisted(rel: &str, line: &str) -> bool {
    READY_YES_ALLOWLIST
        .iter()
        .any(|(file, exact)| *file == rel && *exact == line)
}

#[test]
fn ready_for_live_test_stays_no() {
    let root = repo_root();
    let status = fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
    assert!(
        status.contains("READY_FOR_LIVE_TEST`: no") || status.contains("READY_FOR_LIVE_TEST: no"),
        "CELL-ONE-STATUS.md must still state READY_FOR_LIVE_TEST no"
    );
    for path in walk_docs_and_config(&root) {
        let rel = rel_path(&root, &path);
        let text = fs::read_to_string(&path).unwrap();
        for (idx, line) in text.lines().enumerate() {
            if sets_ready_yes(line) && !ready_yes_allowlisted(&rel, line) {
                panic!(
                    "{rel}:{} sets READY_FOR_LIVE_TEST to yes/true/1: {line}",
                    idx + 1
                );
            }
        }
    }
}

#[test]
fn examples_estate_yaml_stays_byte_locked() {
    let path = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes.len(), 3391, "examples/estate.yaml length changed");
    let sum = posix_cksum(&bytes);
    assert_eq!(
        sum, 43770130,
        "examples/estate.yaml cksum changed: {sum} {}",
        bytes.len()
    );
}

fn mentions_live_topic(lower: &str) -> bool {
    lower.contains("live")
        || lower.contains("5090")
        || lower.contains("uniqueness")
        || lower.contains("specialist")
}

fn has_standalone_pass(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if &bytes[i..i + 4] == b"PASS" {
            let before_ok = i == 0 || !is_token_char(bytes[i - 1]);
            let after = i + 4;
            let after_ok = after == bytes.len() || !is_token_char(bytes[after]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn line_denies_live_pass(lower: &str) -> bool {
    lower.contains("not a live pass")
        || lower.contains("no live pass")
        || lower.contains("does not invent a live pass")
        || lower.contains("does not invent a new live pass")
        || lower.contains("do not invent a live pass")
        || lower.contains("do not claim a train, a promote, a live pass")
        || lower.contains("not a pass")
}

fn contains_live_pass_token(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    let needle = b"live pass";
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let end = i + needle.len();
            if end == bytes.len() || !is_token_char(bytes[end]) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn line_claims_live_pass(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if line_denies_live_pass(&lower) {
        return false;
    }
    let bold = line.contains("**PASS**") || line.contains("**PASS.**");
    if bold {
        return mentions_live_topic(&lower);
    }
    if contains_live_pass_token(&lower) {
        return true;
    }
    has_standalone_pass(line) && mentions_live_topic(&lower)
}

/// Recorded proofs already on the live-probes page.
/// Target C is the uniqueness prove. The other two headings are the
/// probe and Mac specialist rows that page already records.
fn live_pass_section_allowlist(rel: &str, section: &str) -> bool {
    rel == "docs/LIVE-PROBES.md"
        && matches!(
            section,
            "Recorded live proof (Jason boxes)"
                | "Target C live uniqueness (5090-class)"
                | "Mac specialist result (recorded)"
        )
}

fn live_pass_line_allowlisted(rel: &str, line: &str) -> bool {
    LIVE_PASS_LINE_ALLOWLIST
        .iter()
        .any(|(file, exact)| *file == rel && *exact == line)
}

#[test]
fn live_pass_claims_stay_on_the_recorded_proves() {
    let root = repo_root();
    for path in walk_docs_and_config(&root) {
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let rel = rel_path(&root, &path);
        let text = fs::read_to_string(&path).unwrap();
        let mut section = String::new();
        for (idx, line) in text.lines().enumerate() {
            if let Some(title) = line.strip_prefix("## ") {
                section = title.trim().to_string();
            }
            if line_claims_live_pass(line)
                && !live_pass_section_allowlist(&rel, &section)
                && !live_pass_line_allowlisted(&rel, line)
            {
                panic!(
                    "{rel}:{} claims a live PASS outside the recorded proves: {line}",
                    idx + 1
                );
            }
        }
    }
}

#[test]
fn safety_matchers_catch_each_spelling() {
    for spelling in [
        "READY_FOR_LIVE_TEST=yes",
        "READY_FOR_LIVE_TEST= yes",
        "READY_FOR_LIVE_TEST: \"yes\"",
        "ready_for_live_test: \"YES\"",
        "`READY_FOR_LIVE_TEST` is yes",
        "READY_FOR_LIVE_TEST=true",
        "READY_FOR_LIVE_TEST=TRUE",
        "READY_FOR_LIVE_TEST = 1",
        "READY_FOR_LIVE_TEST: yes",
    ] {
        assert!(sets_ready_yes(spelling), "missed ready spelling: {spelling}");
    }
    for benign in [
        "READY_FOR_LIVE_TEST: no",
        "READY_FOR_LIVE_TEST`: no",
        "READY_FOR_LIVE_TEST was yes",
        "READY_FOR_LIVE_TEST yes.",
        "READY_FOR_LIVE_TEST for that surface: yes",
        "READY_FOR_LIVE_TEST=yesterday",
        "READY_FOR_LIVE_TEST=10",
    ] {
        assert!(!sets_ready_yes(benign), "false ready hit: {benign}");
    }
    for spelling in [
        "**PASS** on the 5090",
        "see **PASS.** on the specialist row",
        "uniqueness **PASS** stays recorded",
        "the live PASS is written down",
        "5090-class PASS on that host",
        "uniqueness PASS stays recorded",
        "specialist PASS on the Mac",
        "this is a live pass",
        "this is a live pass.",
    ] {
        assert!(
            line_claims_live_pass(spelling),
            "missed live-pass spelling: {spelling}"
        );
    }
    for benign in [
        "does not invent a live PASS",
        "does not invent a new live PASS",
        "not a live PASS",
        "unit PASS with no topic",
        "**PASS**",
        "see **PASS.** here",
        "live password",
        "live password reset",
        "READY_FOR_LIVE_TEST: no",
        "PASSWORD=1",
    ] {
        assert!(
            !line_claims_live_pass(benign),
            "false live-pass hit: {benign}"
        );
    }
}

fn makefile_targets(makefile: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in makefile.lines() {
        let Some(name) = line.strip_suffix(':') else {
            continue;
        };
        if name.is_empty()
            || name.contains(char::is_whitespace)
            || name.starts_with('.')
            || name.starts_with('#')
        {
            continue;
        }
        let opt_in = name.ends_with("-journey")
            || name.starts_with("uniqueness-")
            || name.starts_with("purpose-build-")
            || name.starts_with("classify-")
            || name == "train-next"
            || name == "train-next-lora"
            || name == "lf-beachhead-prepare";
        if opt_in {
            names.push(name.to_string());
        }
    }
    names
}

fn recipe<'a>(makefile: &'a str, target: &str) -> &'a str {
    let mut offset = 0;
    while offset < makefile.len() {
        let rest = &makefile[offset..];
        let next = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
        let header = rest[..next].trim_end_matches(['\n', '\r']);
        let matched = header
            .strip_prefix(target)
            .and_then(|after_name| after_name.strip_prefix(':'))
            .is_some_and(|after| after.is_empty() || after.starts_with([' ', '\t']));
        offset += next;
        if matched {
            let body = &makefile[offset..];
            let end = body
                .lines()
                .position(|line| {
                    !line.is_empty() && !line.starts_with([' ', '\t']) && !line.starts_with('#')
                })
                .map(|i| body.lines().take(i).map(|l| l.len() + 1).sum())
                .unwrap_or(body.len());
            return &body[..end];
        }
        if next == 0 {
            break;
        }
    }
    panic!("Makefile missing {target} recipe");
}

#[test]
fn recipe_accepts_deps_and_trailing_space() {
    let with_deps = "\nsmoke: deps\n\techo hi\n\nnext:\n";
    assert!(recipe(with_deps, "smoke").contains("echo hi"));
    let spaced = "gate-90: \n\techo gate\n";
    assert!(recipe(spaced, "gate-90").contains("echo gate"));
    let plain = "\nsmoke:\n\techo plain\n";
    assert!(recipe(plain, "smoke").contains("echo plain"));
}

#[test]
fn opt_in_journeys_stay_off_smoke_gate_and_actions() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let targets = makefile_targets(&makefile);
    assert!(
        targets.iter().any(|name| name == "purpose-build-journey"),
        "purpose-build-journey must stay a Makefile target"
    );
    assert!(
        targets.iter().any(|name| name == "classify-prepare"),
        "classify-prepare must stay a Makefile target"
    );
    let mut files: Vec<(String, String)> = vec![
        ("Makefile smoke".into(), recipe(&makefile, "smoke").to_string()),
        (
            "Makefile gate-90".into(),
            recipe(&makefile, "gate-90").to_string(),
        ),
        (
            "scripts/smoke.sh".into(),
            fs::read_to_string(root.join("scripts/smoke.sh")).unwrap(),
        ),
        (
            "scripts/day90-gate.sh".into(),
            fs::read_to_string(root.join("scripts/day90-gate.sh")).unwrap(),
        ),
    ];
    for entry in fs::read_dir(root.join(".github/workflows")).unwrap().flatten() {
        let path = entry.path();
        if path.is_file() {
            let name = rel_path(&root, &path);
            files.push((name, fs::read_to_string(&path).unwrap()));
        }
    }
    for name in &targets {
        for (surface, body) in &files {
            assert!(
                !body.contains(name),
                "{surface} invokes opt-in target {name}"
            );
        }
    }
}
