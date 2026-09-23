//! Print-only adapter merge. Refuses a bad shape and does not spawn axolotl.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn estate_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn tmp(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "cell-one-merge-adapt-cli-{}-{}",
        name,
        std::process::id()
    ));
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

fn estate_bytes() -> String {
    std::fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap()
}

fn write_train_estate(dir: &std::path::Path, model: &str, train_base: &str) -> PathBuf {
    let needle = "  - id: local_slm\n    class: local\n    driver: ollama\n    params:\n";
    let src = estate_bytes();
    assert!(src.contains(needle), "local_slm params block moved");
    let insert =
        format!("{needle}      model: \"{model}\"\n      train_base_model: \"{train_base}\"\n");
    let seated = src.replacen(needle, &insert, 1);
    let path = dir.join("seated-estate.yaml");
    std::fs::write(&path, seated).unwrap();
    path
}

fn write_prepare(dir: &std::path::Path, driver: &str, train_base: Option<&str>) {
    let train_line = match train_base {
        Some(train) => format!("  \"train_base_model\": \"{train}\",\n"),
        None => String::new(),
    };
    let body = format!(
        "{{\n\
           \"schema\": \"cell-one.enrich-prepare.v0\",\n\
           \"driver\": \"{driver}\",\n\
           \"job\": \"train\",\n\
           \"pack_id\": \"overnight-traces\",\n\
           \"base_model\": \"llama3\",\n\
           \"seat_tag\": \"llama3\",\n\
           {train_line}\
           \"purpose\": \"fixture\",\n\
           \"host_class_affinity\": \"any\",\n\
           \"source_paths\": [],\n\
           \"source_drivers\": [],\n\
           \"artifacts\": [],\n\
           \"promoted\": false,\n\
           \"auto_apply\": false,\n\
           \"estate_rewritten\": false,\n\
           \"note\": \"test\"\n\
         }}\n"
    );
    std::fs::write(dir.join("prepare.json"), body).unwrap();
}

fn adapter_dir(dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("adapter_config.json"), "{}\n").unwrap();
}

fn install_traps(root: &std::path::Path) -> PathBuf {
    let bin = root.join("trap-bin");
    std::fs::create_dir_all(&bin).unwrap();
    let sentinel = root.join("spawned");
    let script = format!(
        "#!/bin/sh\necho spawned >> {}\nexit 0\n",
        sentinel.display()
    );
    for name in [
        "python",
        "python3",
        "ollama",
        "axolotl",
        "llamafactory-cli",
        "convert_hf_to_gguf.py",
        "llama-cli",
        "llama-server",
    ] {
        let path = bin.join(name);
        std::fs::write(&path, &script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    bin
}

fn run_merge(
    root: &std::path::Path,
    prepared: &std::path::Path,
    adapter: &std::path::Path,
) -> std::process::Output {
    let bin = install_traps(root);
    let path = match std::env::var("PATH") {
        Ok(path) => format!("{}:{path}", bin.display()),
        Err(_) => bin.display().to_string(),
    };
    estate_bin()
        .env("PATH", path)
        .args([
            "enrich",
            "merge-adapt",
            "--prepared",
            prepared.to_str().unwrap(),
            "--adapter",
            adapter.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}

#[test]
fn help_names_merge_adapt() {
    let out = estate_bin().args(["help", "enrich"]).output().unwrap();
    let body = text(&out);
    assert!(out.status.success(), "{body}");
    assert!(body.contains("estate enrich merge-adapt"), "{body}");
    assert!(body.contains("axolotl merge-lora"), "{body}");
    assert!(body.contains("--lora-model-dir"), "{body}");
    assert!(body.contains("--dequant"), "{body}");
    assert!(body.contains("outputs/merged"), "{body}");
    assert!(body.contains("merge_and_unload"), "{body}");
    assert!(body.contains("refuse:adapter"), "{body}");
    assert!(body.contains("does not merge"), "{body}");
    assert!(body.contains("READY_FOR_LIVE_TEST stays no"), "{body}");
    assert!(!body.contains("READY_FOR_LIVE_TEST: yes"), "{body}");
    assert!(!body.contains("<merged-hf-dir>"), "{body}");
}

#[test]
fn prepared_axolotl_qlora_prints_merge_lora_and_spawns_nothing() {
    let root = tmp("prepare-qlora");
    let estate = write_train_estate(&root, "llama3", "Qwen/Qwen2.5-0.5B-Instruct");
    let pack = repo_root().join("examples/fixtures/specialist-overnight.pack.json");
    let out_dir = root.join("axolotl-qlora");
    let prepared = estate_bin()
        .args([
            "enrich",
            "prepare",
            "--estate",
            estate.to_str().unwrap(),
            "--pack",
            pack.to_str().unwrap(),
            "--driver",
            "axolotl-qlora",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let prepared_text = text(&prepared);
    assert!(prepared.status.success(), "{prepared_text}");
    let next = std::fs::read_to_string(out_dir.join("NEXT.md")).unwrap();
    let outputs = out_dir.join("outputs");
    let merged = outputs.join("merged");
    assert!(
        next.contains(&format!(
            "estate enrich merge-adapt --prepared {} --adapter {}",
            out_dir.display(),
            outputs.display()
        )),
        "{next}"
    );
    assert!(
        next.contains(&format!(
            "axolotl merge-lora {} --lora-model-dir={} --dequant",
            out_dir.join("axolotl.yml").display(),
            outputs.display()
        )),
        "{next}"
    );
    adapter_dir(&outputs);
    let prepare_before = std::fs::read(out_dir.join("prepare.json")).unwrap();
    let yaml_before = std::fs::read(out_dir.join("axolotl.yml")).unwrap();
    let out = run_merge(&root, &out_dir, &outputs);
    let body = text(&out);
    assert!(out.status.success(), "{body}");
    let merge = format!(
        "axolotl merge-lora {} --lora-model-dir={}",
        out_dir.join("axolotl.yml").display(),
        outputs.display()
    );
    assert!(body.contains(&merge), "{body}");
    assert!(body.contains(&format!("{merge} --dequant")), "{body}");
    assert!(body.contains("shape=adapter"), "{body}");
    assert!(body.contains("seat_tag=llama3"), "{body}");
    assert!(body.contains(&merged.display().to_string()), "{body}");
    assert!(body.contains("does not take `--out`"), "{body}");
    let convert = format!(
        "python3 convert_hf_to_gguf.py {} --outfile {} --outtype auto",
        merged.display(),
        outputs.join("merged.gguf").display()
    );
    assert!(body.contains(&convert), "{body}");
    assert!(
        body.contains(&format!(
            "estate enrich gguf-convert --prepared {} --weights {}",
            out_dir.display(),
            merged.display()
        )),
        "{body}"
    );
    assert!(
        body.contains(&format!(
            "estate enrich local-seat --prepared {} --weights {}",
            out_dir.display(),
            merged.display()
        )),
        "{body}"
    );
    assert!(body.contains("merge-adapt did not merge"), "{body}");
    assert!(body.contains("promoted=false"), "{body}");
    assert!(body.contains("READY_FOR_LIVE_TEST: no"), "{body}");
    assert!(!body.contains("READY_FOR_LIVE_TEST: yes"), "{body}");
    assert!(!merged.exists());
    assert!(!outputs.join("merged.gguf").exists());
    assert!(!root.join("spawned").exists(), "merge tooling was spawned");
    assert_eq!(
        std::fs::read(out_dir.join("prepare.json")).unwrap(),
        prepare_before
    );
    assert_eq!(
        std::fs::read(out_dir.join("axolotl.yml")).unwrap(),
        yaml_before
    );
}

#[test]
fn prepared_axolotl_lora_omits_dequant() {
    let root = tmp("prepare-lora");
    let estate = write_train_estate(&root, "llama3", "Qwen/Qwen2.5-0.5B-Instruct");
    let pack = repo_root().join("examples/fixtures/specialist-overnight.pack.json");
    let out_dir = root.join("axolotl-lora");
    let prepared = estate_bin()
        .args([
            "enrich",
            "prepare",
            "--estate",
            estate.to_str().unwrap(),
            "--pack",
            pack.to_str().unwrap(),
            "--driver",
            "axolotl-lora",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(prepared.status.success(), "{}", text(&prepared));
    let outputs = out_dir.join("outputs");
    adapter_dir(&outputs);
    let out = run_merge(&root, &out_dir, &outputs);
    let body = text(&out);
    assert!(out.status.success(), "{body}");
    assert!(body.contains("axolotl merge-lora "), "{body}");
    assert!(!body.contains("--dequant"), "{body}");
    assert!(!out_dir.join("outputs").join("merged").exists());
    assert!(!root.join("spawned").exists());
}

#[test]
fn llamafactory_adapter_prints_peft_and_spawns_nothing() {
    let root = tmp("lf");
    write_prepare(
        &root,
        "llamafactory-qlora",
        Some("Qwen/Qwen2.5-0.5B-Instruct"),
    );
    let outputs = root.join("outputs");
    adapter_dir(&outputs);
    let out = run_merge(&root, &root, &outputs);
    let body = text(&out);
    assert!(out.status.success(), "{body}");
    assert!(body.contains("merge_and_unload()"), "{body}");
    assert!(body.contains("save_pretrained("), "{body}");
    assert!(body.contains("Qwen/Qwen2.5-0.5B-Instruct"), "{body}");
    let merged = root.join("merged");
    assert!(body.contains(&merged.display().to_string()), "{body}");
    assert!(body.contains("no export.yaml"), "{body}");
    assert!(!body.contains("axolotl merge-lora"), "{body}");
    assert!(
        body.contains(&format!(
            "estate enrich gguf-convert --prepared {} --weights {}",
            root.display(),
            merged.display()
        )),
        "{body}"
    );
    assert!(!merged.exists());
    assert!(!root.join("spawned").exists());
}

#[test]
fn refuses_merged_gguf_and_symlink_without_spawning() {
    let root = tmp("refuse");
    write_prepare(&root, "axolotl-qlora", Some("Qwen/Qwen2.5-0.5B-Instruct"));
    std::fs::write(
        root.join("axolotl.yml"),
        format!(
            "base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"\nload_in_4bit: true\nadapter: qlora\noutput_dir: \"{}\"\n",
            root.join("outputs").display()
        ),
    )
    .unwrap();
    let outputs = root.join("outputs");
    adapter_dir(&outputs);

    let missing = root.join("empty");
    std::fs::create_dir_all(&missing).unwrap();
    let missing_out = run_merge(&root, &root, &missing);
    let missing_text = text(&missing_out);
    assert!(!missing_out.status.success(), "{missing_text}");
    assert!(missing_text.contains("refuse:adapter"), "{missing_text}");
    assert!(
        missing_text.contains("adapter_config.json"),
        "{missing_text}"
    );
    assert!(
        !missing_text.contains("axolotl merge-lora "),
        "{missing_text}"
    );

    let merged = root.join("merged-weights");
    std::fs::create_dir_all(&merged).unwrap();
    std::fs::write(merged.join("config.json"), "{}\n").unwrap();
    std::fs::write(merged.join("model.safetensors"), b"not-a-real-tensor").unwrap();
    let merged_out = run_merge(&root, &root, &merged);
    let merged_text = text(&merged_out);
    assert!(!merged_out.status.success(), "{merged_text}");
    assert!(merged_text.contains("refuse:adapter"), "{merged_text}");
    assert!(merged_text.contains("merged"), "{merged_text}");

    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&[0u8; 12]);
    let gguf = root.join("weights.gguf");
    std::fs::write(&gguf, bytes).unwrap();
    let gguf_out = run_merge(&root, &root, &gguf);
    let gguf_text = text(&gguf_out);
    assert!(!gguf_out.status.success(), "{gguf_text}");
    assert!(gguf_text.contains("refuse:adapter"), "{gguf_text}");
    assert!(gguf_text.contains("GGUF"), "{gguf_text}");

    let linked = root.join("linked-adapter");
    std::os::unix::fs::symlink(&outputs, &linked).unwrap();
    let link_out = run_merge(&root, &root, &linked);
    let link_text = text(&link_out);
    assert!(!link_out.status.success(), "{link_text}");
    assert!(link_text.contains("refuse:adapter"), "{link_text}");
    assert!(link_text.contains("symlink"), "{link_text}");

    let sacred = root.join("cyera-adapter");
    adapter_dir(&sacred);
    let sacred_out = run_merge(&root, &root, &sacred);
    let sacred_text = text(&sacred_out);
    assert!(!sacred_out.status.success(), "{sacred_text}");
    assert!(sacred_text.contains("refuse:sacred"), "{sacred_text}");

    assert!(!root.join("spawned").exists());
    assert!(!root.join("outputs").join("merged").exists());
    assert!(!root.join("merged").exists());
}
