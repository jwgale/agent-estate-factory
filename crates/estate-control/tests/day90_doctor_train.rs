//! Doctor fails a present prepare.json that does not parse.
//! A missing enrich directory is not a failure, and is not invented as zero packs.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cell-one-doctor-train-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn run(bin: &str, args: &[&str]) -> (bool, String) {
    let out = Command::new(bin)
        .args(args)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

fn doctor(bin: &str, root: &PathBuf, state: &PathBuf) -> (bool, String) {
    run(
        bin,
        &[
            "doctor",
            "--root",
            &root.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
        ],
    )
}

fn prepare_body(shape: Option<&str>) -> String {
    let trained = match shape {
        Some(shape) => format!(
            ",\n  \"trained_shape\": \"{shape}\",\n  \"trained_paths\": [\"/tmp/cell-one-adapter\"]"
        ),
        None => String::new(),
    };
    format!(
        "{{\n  \"schema\": \"cell-one.enrich-prepare.v0\",\n  \"driver\": \"llamafactory-qlora\",\n  \"job\": \"train\",\n  \"pack_id\": \"phi3-instruct\",\n  \"base_model\": \"llama3\",\n  \"seat_tag\": \"llama3\",\n  \"train_base_model\": \"microsoft/Phi-3-mini-4k-instruct\",\n  \"purpose\": \"smoke\",\n  \"host_class_affinity\": \"any\",\n  \"source_paths\": [],\n  \"source_drivers\": [],\n  \"artifacts\": [\"prepare.json\"],\n  \"promoted\": false,\n  \"auto_apply\": false,\n  \"estate_rewritten\": false,\n  \"note\": \"Prepared artifacts only.\"{trained}\n}}\n"
    )
}

fn prepare_for(driver: &str, pack: &str, shape: Option<&str>, promoted: bool) -> String {
    let trained = match shape {
        Some(shape) => format!(
            ",\n  \"trained_shape\": \"{shape}\",\n  \"trained_paths\": [\"/tmp/cell-one-adapter\"]"
        ),
        None => String::new(),
    };
    let promoted = if promoted { "true" } else { "false" };
    format!(
        "{{\n  \"schema\": \"cell-one.enrich-prepare.v0\",\n  \"driver\": \"{driver}\",\n  \"job\": \"train\",\n  \"pack_id\": \"{pack}\",\n  \"base_model\": \"llama3\",\n  \"seat_tag\": \"llama3\",\n  \"train_base_model\": \"Qwen/Qwen2.5-0.5B-Instruct\",\n  \"purpose\": \"smoke\",\n  \"host_class_affinity\": \"any\",\n  \"source_paths\": [],\n  \"source_drivers\": [],\n  \"artifacts\": [\"prepare.json\"],\n  \"promoted\": {promoted},\n  \"auto_apply\": false,\n  \"estate_rewritten\": false,\n  \"note\": \"Prepared artifacts only.\"{trained}\n}}\n"
    )
}

fn assert_no_invented_prepare_count(text: &str) {
    assert!(
        !text.contains("packs=0") && !text.contains("prepares=0") && !text.contains("count=0"),
        "must not invent a prepare count: {text}"
    );
}

fn assert_catalog_is_the_card(text: &str) {
    assert!(
        text.contains("train_catalog llamafactory-qlora status=integration live=false"),
        "{text}"
    );
    assert!(
        text.contains("train_catalog external-manifest status=portable live=false"),
        "{text}"
    );
    assert!(
        text.contains("train_catalog unsloth-qlora status=optional live=false")
            && text.contains("train_catalog unsloth-lora status=optional live=false"),
        "{text}"
    );
    assert_eq!(
        text.matches("train_catalog mlx-lm-lora status=optional live=false")
            .count(),
        1,
        "{text}"
    );
    assert!(
        !text.contains("train_catalog mlx-lm-lora status=integration"),
        "{text}"
    );
    assert!(
        text.contains("train_catalog axolotl-lora status=integration live=false"),
        "{text}"
    );
    assert!(
        text.contains("train_catalog axolotl-qlora status=integration live=false"),
        "{text}"
    );
    assert_eq!(
        text.matches("train_catalog axolotl-lora status=integration live=false")
            .count(),
        1,
        "{text}"
    );
    assert_eq!(
        text.matches("train_catalog axolotl-qlora status=integration live=false")
            .count(),
        1,
        "{text}"
    );
    assert!(
        !text.contains("train_catalog axolotl-lora status=optional"),
        "{text}"
    );
    assert!(
        !text.contains("train_catalog axolotl-qlora status=optional"),
        "{text}"
    );
    assert_eq!(
        text.matches("train_catalog unsloth-qlora status=optional live=false")
            .count(),
        1,
        "{text}"
    );
    assert_eq!(
        text.matches("train_catalog unsloth-lora status=optional live=false")
            .count(),
        1,
        "{text}"
    );
    assert!(
        !text.contains("train_catalog unsloth-qlora status=integration"),
        "{text}"
    );
    assert!(
        !text.contains("train_catalog unsloth-lora status=integration"),
        "{text}"
    );
    assert!(!text.contains("live=true"), "{text}");
    assert!(!text.contains("READY_FOR_LIVE_TEST: yes"), "{text}");
    assert!(
        !text.contains("factory trained") && !text.contains("train complete"),
        "catalog must not claim a train: {text}"
    );
}

fn assert_prepare_is_a_record(text: &str) {
    assert!(
        text.contains(
            "note  prepare.json record only; this factory did not train, merge, convert, or seat"
        ),
        "{text}"
    );
    assert!(!text.contains("live=true"), "{text}");
    assert!(!text.contains("READY_FOR_LIVE_TEST: yes"), "{text}");
    assert!(!text.contains("promoted=true"), "{text}");
    assert!(!text.contains("auto_apply=true"), "{text}");
    assert!(
        !text.contains("factory trained") && !text.contains("train complete"),
        "a prepare record must not claim a train: {text}"
    );
}

#[test]
fn doctor_reports_a_train_prepare_and_fails_a_broken_one_before_factory_ready() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("prepare");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();

    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("train_prepare:"),
        "missing enrich directory must stay silent: {text}"
    );
    assert_no_invented_prepare_count(&text);
    assert_catalog_is_the_card(&text);

    std::fs::create_dir_all(state.join("enrich")).unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(
        text.contains("note  enrich directory has no prepare.json"),
        "{text}"
    );
    assert!(!text.contains("train_prepare: pack="), "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert_no_invented_prepare_count(&text);

    let prepare = state.join("enrich/phi3-instruct/llamafactory-qlora/prepare.json");
    std::fs::create_dir_all(prepare.parent().unwrap()).unwrap();
    let healthy = prepare_body(None);
    std::fs::write(&prepare, &healthy).unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(
        text.contains("ok    train_prepare: pack=phi3-instruct driver=llamafactory-qlora job=train seat_tag=llama3 train_base=microsoft/Phi-3-mini-4k-instruct"),
        "{text}"
    );
    assert!(
        text.contains(&format!("out={}", prepare.parent().unwrap().display())),
        "{text}"
    );
    assert!(!text.contains("trained_shape="), "{text}");
    assert!(
        text.contains(
            "note  prepare.json record only; this factory did not train, merge, convert, or seat"
        ),
        "{text}"
    );
    assert!(text.contains("factory ready"), "{text}");
    assert!(!text.contains("live=true"), "{text}");

    std::fs::write(&prepare, prepare_body(Some("gguf"))).unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("trained_shape=gguf"), "{text}");
    assert!(text.contains("factory ready"), "{text}");

    std::fs::write(&prepare, "not-json\n").unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("prepare.json"), "{text}");
    assert!(text.contains("FAIL"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("ok    train_prepare:"),
        "garbage must not become a clean train line: {text}"
    );
    assert_eq!(std::fs::read_to_string(&prepare).unwrap(), "not-json\n");

    std::fs::write(
        &prepare,
        "{\"schema\":\"nope\",\"driver\":\"llamafactory-qlora\",\"job\":\"train\",\"pack_id\":\"phi3-instruct\",\"base_model\":\"llama3\",\"purpose\":\"smoke\",\"host_class_affinity\":\"any\",\"source_paths\":[],\"source_drivers\":[],\"artifacts\":[],\"promoted\":false,\"auto_apply\":false,\"estate_rewritten\":false,\"note\":\"x\"}\n",
    )
    .unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:prepare:"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(!text.contains("ok    train_prepare:"), "{text}");

    std::fs::write(&prepare, &healthy).unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("job=train"), "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert!(!text.contains("trained_shape="), "{text}");
}

#[test]
fn doctor_fails_an_enrich_symlink_before_factory_ready() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("symlink");
    let state = dir.join("state");
    let outside = dir.join("outside");
    std::fs::create_dir_all(state.join("enrich")).unwrap();
    let outside_pack = outside.join("phi3-instruct");
    std::fs::create_dir_all(outside_pack.join("llamafactory-qlora")).unwrap();
    std::fs::write(
        outside_pack.join("llamafactory-qlora/prepare.json"),
        prepare_body(None).replace("phi3-instruct", "escaped-pack"),
    )
    .unwrap();
    std::os::unix::fs::symlink(&outside_pack, state.join("enrich/phi3-instruct")).unwrap();

    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:enrich-index"), "{text}");
    assert!(text.contains("symlink"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("escaped-pack"),
        "a symlink must not be read as a prepare: {text}"
    );
    assert!(!text.contains("ok    train_prepare:"), "{text}");
}

#[test]
fn doctor_names_axolotl_cards_and_refuses_a_bad_prepare_before_factory_ready() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("axolotl");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();

    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert_catalog_is_the_card(&text);
    assert!(
        !text.contains("train_prepare:"),
        "missing enrich directory must stay silent: {text}"
    );

    let lora = state.join("enrich/overnight-traces/axolotl-lora/prepare.json");
    let qlora = state.join("enrich/overnight-traces/axolotl-qlora/prepare.json");
    std::fs::create_dir_all(lora.parent().unwrap()).unwrap();
    std::fs::create_dir_all(qlora.parent().unwrap()).unwrap();
    let lora_body = prepare_for("axolotl-lora", "overnight-traces", None, false);
    let qlora_body = prepare_for("axolotl-qlora", "overnight-traces", None, false);
    std::fs::write(&lora, &lora_body).unwrap();
    std::fs::write(&qlora, &qlora_body).unwrap();

    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert_catalog_is_the_card(&text);
    assert!(
        text.contains("ok    train_prepare: pack=overnight-traces driver=axolotl-lora job=train seat_tag=llama3 train_base=Qwen/Qwen2.5-0.5B-Instruct"),
        "{text}"
    );
    assert!(
        text.contains("ok    train_prepare: pack=overnight-traces driver=axolotl-qlora job=train seat_tag=llama3 train_base=Qwen/Qwen2.5-0.5B-Instruct"),
        "{text}"
    );
    assert!(
        text.contains(&format!("out={}", lora.parent().unwrap().display())),
        "{text}"
    );
    assert!(
        text.contains(&format!("out={}", qlora.parent().unwrap().display())),
        "{text}"
    );
    assert!(!text.contains("trained_shape="), "{text}");
    assert_prepare_is_a_record(&text);
    assert_eq!(text.matches("ok    train_prepare:").count(), 2, "{text}");

    std::fs::write(&qlora, "not-json\n").unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("prepare.json"), "{text}");
    assert!(text.contains("FAIL"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("ok    train_prepare:"),
        "garbage axolotl prepare must not become a clean train line: {text}"
    );
    assert_eq!(std::fs::read_to_string(&qlora).unwrap(), "not-json\n");
    assert_eq!(std::fs::read_to_string(&lora).unwrap(), lora_body);

    std::fs::write(
        &qlora,
        prepare_for("axolotl-qlora", "overnight-traces", None, true),
    )
    .unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:prepared:"), "{text}");
    assert!(text.contains("promoted"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("ok    train_prepare:"),
        "a promoted axolotl prepare must not become a clean train line: {text}"
    );

    std::fs::write(&qlora, &qlora_body).unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert!(text.contains("driver=axolotl-lora"), "{text}");
    assert!(text.contains("driver=axolotl-qlora"), "{text}");
    assert_prepare_is_a_record(&text);
    assert!(!text.contains("trained_shape="), "{text}");
}

#[test]
fn doctor_names_unsloth_cards_and_refuses_a_bad_prepare_before_factory_ready() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("unsloth");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();

    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert_catalog_is_the_card(&text);
    assert_no_invented_prepare_count(&text);
    assert!(
        !text.contains("train_prepare:"),
        "missing enrich directory must stay silent: {text}"
    );

    let lora = state.join("enrich/overnight-traces/unsloth-lora/prepare.json");
    let qlora = state.join("enrich/overnight-traces/unsloth-qlora/prepare.json");
    std::fs::create_dir_all(lora.parent().unwrap()).unwrap();
    std::fs::create_dir_all(qlora.parent().unwrap()).unwrap();
    let lora_body = prepare_for("unsloth-lora", "overnight-traces", None, false);
    let qlora_body = prepare_for("unsloth-qlora", "overnight-traces", None, false);
    std::fs::write(&lora, &lora_body).unwrap();
    std::fs::write(&qlora, &qlora_body).unwrap();

    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert_catalog_is_the_card(&text);
    assert_no_invented_prepare_count(&text);
    assert!(
        text.contains("ok    train_prepare: pack=overnight-traces driver=unsloth-lora job=train seat_tag=llama3 train_base=Qwen/Qwen2.5-0.5B-Instruct"),
        "{text}"
    );
    assert!(
        text.contains("ok    train_prepare: pack=overnight-traces driver=unsloth-qlora job=train seat_tag=llama3 train_base=Qwen/Qwen2.5-0.5B-Instruct"),
        "{text}"
    );
    assert!(
        text.contains(&format!("out={}", lora.parent().unwrap().display())),
        "{text}"
    );
    assert!(
        text.contains(&format!("out={}", qlora.parent().unwrap().display())),
        "{text}"
    );
    assert!(!text.contains("trained_shape="), "{text}");
    assert_prepare_is_a_record(&text);
    assert_eq!(text.matches("ok    train_prepare:").count(), 2, "{text}");

    std::fs::write(&qlora, "not-json\n").unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("prepare.json"), "{text}");
    assert!(text.contains("FAIL"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("ok    train_prepare:"),
        "garbage unsloth prepare must not become a clean train line: {text}"
    );
    assert_eq!(std::fs::read_to_string(&qlora).unwrap(), "not-json\n");
    assert_eq!(std::fs::read_to_string(&lora).unwrap(), lora_body);

    std::fs::write(
        &qlora,
        prepare_for("unsloth-qlora", "overnight-traces", None, true),
    )
    .unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:prepared:"), "{text}");
    assert!(text.contains("promoted"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("ok    train_prepare:"),
        "a promoted unsloth prepare must not become a clean train line: {text}"
    );

    std::fs::write(&qlora, &qlora_body).unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert!(text.contains("driver=unsloth-lora"), "{text}");
    assert!(text.contains("driver=unsloth-qlora"), "{text}");
    assert_prepare_is_a_record(&text);
    assert_no_invented_prepare_count(&text);
    assert!(!text.contains("trained_shape="), "{text}");
}

#[test]
fn doctor_names_mlx_lm_lora_and_refuses_a_bad_prepare_before_factory_ready() {
    let root = repo_root();
    let bin = env!("CARGO_BIN_EXE_estate");
    let dir = tmp("mlx");
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();

    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert_catalog_is_the_card(&text);
    assert_no_invented_prepare_count(&text);
    assert!(
        !text.contains("train_prepare:"),
        "missing enrich directory must stay silent: {text}"
    );
    assert!(!text.contains("live PASS"), "{text}");

    let prepare = state.join("enrich/overnight-traces/mlx-lm-lora/prepare.json");
    std::fs::create_dir_all(prepare.parent().unwrap()).unwrap();
    let body = prepare_for("mlx-lm-lora", "overnight-traces", None, false);
    std::fs::write(&prepare, &body).unwrap();

    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert_catalog_is_the_card(&text);
    assert_no_invented_prepare_count(&text);
    assert!(
        text.contains("ok    train_prepare: pack=overnight-traces driver=mlx-lm-lora job=train seat_tag=llama3 train_base=Qwen/Qwen2.5-0.5B-Instruct"),
        "{text}"
    );
    assert!(
        text.contains(&format!("out={}", prepare.parent().unwrap().display())),
        "{text}"
    );
    assert!(!text.contains("trained_shape="), "{text}");
    assert_prepare_is_a_record(&text);
    assert_eq!(text.matches("ok    train_prepare:").count(), 1, "{text}");
    assert!(!text.contains("live PASS"), "{text}");

    std::fs::write(&prepare, "not-json\n").unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("prepare.json"), "{text}");
    assert!(text.contains("FAIL"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("ok    train_prepare:"),
        "garbage mlx prepare must not become a clean train line: {text}"
    );
    assert_eq!(std::fs::read_to_string(&prepare).unwrap(), "not-json\n");

    std::fs::write(
        &prepare,
        prepare_for("mlx-lm-lora", "overnight-traces", None, true),
    )
    .unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(!ok, "{text}");
    assert!(text.contains("refuse:prepared:"), "{text}");
    assert!(text.contains("promoted"), "{text}");
    assert!(!text.contains("factory ready"), "{text}");
    assert!(
        !text.contains("ok    train_prepare:"),
        "a promoted mlx prepare must not become a clean train line: {text}"
    );

    std::fs::write(&prepare, &body).unwrap();
    let (ok, text) = doctor(&bin, &root, &state);
    assert!(ok, "{text}");
    assert!(text.contains("factory ready"), "{text}");
    assert!(text.contains("driver=mlx-lm-lora"), "{text}");
    assert_prepare_is_a_record(&text);
    assert_no_invented_prepare_count(&text);
    assert!(!text.contains("trained_shape="), "{text}");
    assert!(!text.contains("live PASS"), "{text}");
}
