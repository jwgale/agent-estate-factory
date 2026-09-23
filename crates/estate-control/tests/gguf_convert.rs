//! Print-only GGUF convert card. Refuses a bad shape and does not spawn llama.cpp.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

fn estate_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_estate"))
}

fn tmp(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "cell-one-gguf-convert-cli-{}-{}",
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

fn write_prepare(dir: &std::path::Path) {
    let body = r#"{
  "schema": "cell-one.enrich-prepare.v0",
  "driver": "llamafactory-qlora",
  "job": "train",
  "pack_id": "overnight-traces",
  "base_model": "llama3",
  "seat_tag": "llama3",
  "purpose": "fixture",
  "host_class_affinity": "any",
  "source_paths": [],
  "source_drivers": [],
  "artifacts": ["export.yaml"],
  "promoted": false,
  "auto_apply": false,
  "estate_rewritten": false,
  "note": "test"
}
"#;
    std::fs::write(dir.join("prepare.json"), body).unwrap();
}

fn merged(dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("config.json"), "{}\n").unwrap();
    std::fs::write(dir.join("model.safetensors"), b"not-a-real-tensor").unwrap();
}

fn gguf_bytes() -> Vec<u8> {
    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&[0u8; 12]);
    bytes
}

fn install_traps(root: &std::path::Path) -> (PathBuf, PathBuf) {
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
        "llama-quantize",
        "convert_hf_to_gguf.py",
        "mlx_lm.fuse",
        "mlx_lm.lora",
        "unsloth",
    ] {
        let path = bin.join(name);
        std::fs::write(&path, &script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    (bin, sentinel)
}

fn run_convert(root: &std::path::Path, weights: &std::path::Path) -> std::process::Output {
    let (bin, _) = install_traps(root);
    let path = match std::env::var("PATH") {
        Ok(path) => format!("{}:{path}", bin.display()),
        Err(_) => bin.display().to_string(),
    };
    estate_bin()
        .env("PATH", path)
        .args([
            "enrich",
            "gguf-convert",
            "--prepared",
            root.to_str().unwrap(),
            "--weights",
            weights.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}

#[test]
fn help_names_the_convert_line() {
    let out = estate_bin().args(["help", "enrich"]).output().unwrap();
    let body = text(&out);
    assert!(out.status.success(), "{body}");
    assert!(body.contains("estate enrich gguf-convert"), "{body}");
    assert!(
        body.contains(
            "python3 convert_hf_to_gguf.py <merged-dir> --outfile <sibling>.gguf --outtype auto"
        ),
        "{body}"
    );
    assert!(body.contains("does not run llama.cpp"), "{body}");
    assert!(body.contains("refuse:tokenizer"), "{body}");
    assert!(body.contains("extra_special_tokens"), "{body}");
    assert!(body.contains("vocab.json"), "{body}");
    assert!(body.contains("merges.txt"), "{body}");
    assert!(body.contains("axolotl-lora"), "{body}");
    assert!(body.contains("axolotl-qlora"), "{body}");
    assert!(body.contains("outputs/merged"), "{body}");
    assert!(body.contains("estate enrich merge-adapt"), "{body}");
    assert!(body.contains("READY_FOR_LIVE_TEST stays no"), "{body}");
    assert!(!body.contains("READY_FOR_LIVE_TEST: yes"), "{body}");
}

#[test]
fn prints_the_convert_line_and_spawns_nothing() {
    let root = tmp("print");
    write_prepare(&root);
    let export = root.join("export");
    merged(&export);
    std::fs::write(export.join("Modelfile"), "FROM .\n").unwrap();
    let before = std::fs::read(export.join("Modelfile")).unwrap();
    let prepare_before = std::fs::read(root.join("prepare.json")).unwrap();
    let out = run_convert(&root, &export);
    let body = text(&out);
    assert!(out.status.success(), "{body}");
    let outfile = root.join("export.gguf");
    let convert = format!(
        "python3 convert_hf_to_gguf.py {} --outfile {} --outtype auto",
        export.display(),
        outfile.display()
    );
    assert!(body.contains(&convert), "{body}");
    assert!(body.contains("shape=merged"), "{body}");
    assert!(body.contains("seat_tag=llama3"), "{body}");
    assert!(
        body.contains(&format!(
            "estate enrich local-seat --prepared {} --weights {}",
            root.display(),
            outfile.display()
        )),
        "{body}"
    );
    assert!(body.contains("gguf-convert did not convert"), "{body}");
    assert!(body.contains("READY_FOR_LIVE_TEST: no"), "{body}");
    assert!(!body.contains("READY_FOR_LIVE_TEST: yes"), "{body}");
    assert!(!body.contains("--outtype q8_0"), "{body}");
    assert!(!body.contains("Q4_K_M"), "{body}");
    assert!(!outfile.exists());
    assert!(!export.join("model.gguf").exists());
    assert!(
        !root.join("spawned").exists(),
        "convert tooling was spawned"
    );
    assert_eq!(std::fs::read(export.join("Modelfile")).unwrap(), before);
    assert_eq!(
        std::fs::read(root.join("prepare.json")).unwrap(),
        prepare_before
    );
}

#[test]
fn refuses_adapter_symlink_and_gguf_without_spawning() {
    let root = tmp("refuse");
    write_prepare(&root);

    let adapter = root.join("adapter");
    std::fs::create_dir_all(&adapter).unwrap();
    std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
    let refused = run_convert(&root, &adapter);
    let refused_text = text(&refused);
    assert!(!refused.status.success(), "{refused_text}");
    assert!(refused_text.contains("refuse:seat"), "{refused_text}");
    assert!(refused_text.contains("adapter directory"), "{refused_text}");
    assert!(
        !refused_text.contains("python3 convert_hf_to_gguf.py"),
        "{refused_text}"
    );
    assert!(!root.join("spawned").exists());

    let shards = root.join("shards");
    std::fs::create_dir_all(&shards).unwrap();
    std::fs::write(shards.join("config.json"), "{}\n").unwrap();
    std::fs::write(shards.join("adapter_model.safetensors"), b"a").unwrap();
    let shards_out = run_convert(&root, &shards);
    let shards_text = text(&shards_out);
    assert!(!shards_out.status.success(), "{shards_text}");
    assert!(shards_text.contains("refuse:seat"), "{shards_text}");
    assert!(shards_text.contains("adapter_model"), "{shards_text}");
    assert!(!shards_text.contains("--outtype auto"), "{shards_text}");

    let export = root.join("export");
    merged(&export);
    let linked = root.join("linked-export");
    std::os::unix::fs::symlink(&export, &linked).unwrap();
    let link_out = run_convert(&root, &linked);
    let link_text = text(&link_out);
    assert!(!link_out.status.success(), "{link_text}");
    assert!(link_text.contains("refuse:seat"), "{link_text}");
    assert!(link_text.contains("symlink"), "{link_text}");

    let gguf = root.join("model.gguf");
    std::fs::write(&gguf, gguf_bytes()).unwrap();
    let gguf_out = run_convert(&root, &gguf);
    let gguf_text = text(&gguf_out);
    assert!(!gguf_out.status.success(), "{gguf_text}");
    assert!(gguf_text.contains("refuse:seat"), "{gguf_text}");
    assert!(gguf_text.contains("is a GGUF"), "{gguf_text}");
    assert!(
        gguf_text.contains("estate enrich local-seat"),
        "{gguf_text}"
    );
    assert!(
        !gguf_text.contains("python3 convert_hf_to_gguf.py"),
        "{gguf_text}"
    );
    assert!(!root.join("spawned").exists());
    assert!(!root.join("export.gguf").exists());
}

fn write_driver_prepare(dir: &std::path::Path, driver: &str) {
    let body = format!(
        r#"{{
  "schema": "cell-one.enrich-prepare.v0",
  "driver": "{driver}",
  "job": "train",
  "pack_id": "overnight-traces",
  "base_model": "llama3",
  "seat_tag": "llama3",
  "purpose": "fixture",
  "host_class_affinity": "any",
  "source_paths": [],
  "source_drivers": [],
  "artifacts": ["axolotl.yml"],
  "promoted": false,
  "auto_apply": false,
  "estate_rewritten": false,
  "note": "test"
}}
"#
    );
    std::fs::write(dir.join("prepare.json"), body).unwrap();
}

#[test]
fn axolotl_prepare_prints_the_convert_line_and_refuses_the_adapter() {
    for driver in ["axolotl-lora", "axolotl-qlora"] {
        let root = tmp(&format!("ax-{driver}"));
        write_driver_prepare(&root, driver);
        let merged_dir = root.join("merged");
        merged(&merged_dir);
        let out = run_convert(&root, &merged_dir);
        let body = text(&out);
        assert!(out.status.success(), "{driver}: {body}");
        let outfile = root.join("merged.gguf");
        let convert = format!(
            "python3 convert_hf_to_gguf.py {} --outfile {} --outtype auto",
            merged_dir.display(),
            outfile.display()
        );
        assert!(body.contains(&convert), "{driver}: {body}");
        assert!(
            body.contains("Axolotl does not write GGUF"),
            "{driver}: {body}"
        );
        assert!(
            body.contains("gguf-convert did not convert"),
            "{driver}: {body}"
        );
        assert!(body.contains("READY_FOR_LIVE_TEST: no"), "{driver}: {body}");
        assert!(
            !body.contains("READY_FOR_LIVE_TEST: yes"),
            "{driver}: {body}"
        );
        assert!(!body.contains("Axolotl wrote"), "{driver}: {body}");
        assert!(!outfile.exists(), "{driver}");
        assert!(!root.join("spawned").exists(), "{driver}");

        let adapter = root.join("outputs");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        let refused = run_convert(&root, &adapter);
        let refused_text = text(&refused);
        assert!(!refused.status.success(), "{driver}: {refused_text}");
        assert!(
            refused_text.contains("refuse:seat"),
            "{driver}: {refused_text}"
        );
        assert!(
            refused_text.contains("adapter directory"),
            "{driver}: {refused_text}"
        );
        assert!(
            !refused_text.contains("python3 convert_hf_to_gguf.py"),
            "{driver}: {refused_text}"
        );

        let card = root.join("export-card");
        std::fs::create_dir_all(&card).unwrap();
        std::fs::write(card.join("export.yaml"), "adapter_name_or_path: outputs\n").unwrap();
        let card_out = run_convert(&root, &card);
        let card_text = text(&card_out);
        assert!(!card_out.status.success(), "{driver}: {card_text}");
        assert!(card_text.contains("refuse:seat"), "{driver}: {card_text}");
        assert!(
            !card_text.contains("--outtype auto"),
            "{driver}: {card_text}"
        );

        let linked = root.join("linked-merged");
        std::os::unix::fs::symlink(&merged_dir, &linked).unwrap();
        let link_out = run_convert(&root, &linked);
        let link_text = text(&link_out);
        assert!(!link_out.status.success(), "{driver}: {link_text}");
        assert!(link_text.contains("refuse:seat"), "{driver}: {link_text}");
        assert!(link_text.contains("symlink"), "{driver}: {link_text}");
        assert!(!root.join("spawned").exists(), "{driver}");
    }
}

fn write_mlx(dir: &std::path::Path, host: &str) {
    let body = format!(
        "{{\n\
           \"schema\": \"cell-one.enrich-prepare.v0\",\n\
           \"driver\": \"mlx-lm-lora\",\n\
           \"job\": \"train\",\n\
           \"pack_id\": \"overnight-traces\",\n\
           \"base_model\": \"llama3\",\n\
           \"seat_tag\": \"llama3\",\n\
           \"train_base_model\": \"Qwen/Qwen2.5-0.5B-Instruct\",\n\
           \"purpose\": \"fixture\",\n\
           \"host_class_affinity\": \"{host}\",\n\
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
    std::fs::write(
        dir.join("MLX.md"),
        "train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"\nseat_tag: \"llama3\"\nhost_class_affinity: apple-silicon\n",
    )
    .unwrap();
}

#[test]
fn mlx_convert_refuses_and_does_not_spawn() {
    let root = tmp("mlx");
    write_mlx(&root, "apple-silicon");
    let gguf = root.join("ggml-model-f16.gguf");
    std::fs::write(&gguf, gguf_bytes()).unwrap();
    let gguf_out = run_convert(&root, &gguf);
    let gguf_text = text(&gguf_out);
    assert!(!gguf_out.status.success(), "{gguf_text}");
    assert!(gguf_text.contains("refuse:seat"), "{gguf_text}");
    assert!(gguf_text.contains("is a GGUF"), "{gguf_text}");
    assert!(gguf_text.contains("--export-gguf"), "{gguf_text}");
    assert!(
        !gguf_text.contains("python3 convert_hf_to_gguf.py"),
        "{gguf_text}"
    );
    assert!(!gguf_text.contains("--outtype"), "{gguf_text}");

    let fused = root.join("fused_model");
    merged(&fused);
    let fused_out = run_convert(&root, &fused);
    let fused_text = text(&fused_out);
    assert!(!fused_out.status.success(), "{fused_text}");
    assert!(fused_text.contains("refuse:seat"), "{fused_text}");
    assert!(fused_text.contains("MLX weights"), "{fused_text}");
    assert!(!fused_text.contains("--outtype"), "{fused_text}");
    assert!(!root.join("fused_model.gguf").exists());
    assert!(!root.join("spawned").exists());

    write_mlx(&root, "any");
    let host_out = run_convert(&root, &gguf);
    let host_text = text(&host_out);
    assert!(!host_out.status.success(), "{host_text}");
    assert!(host_text.contains("refuse:host"), "{host_text}");
    assert!(!host_text.contains("mlx_lm.fuse"), "{host_text}");
    assert!(!root.join("spawned").exists());
}

#[test]
fn unsloth_convert_prints_the_lines_and_does_not_spawn() {
    let root = tmp("unsloth");
    let body = r#"{
  "schema": "cell-one.enrich-prepare.v0",
  "driver": "unsloth-qlora",
  "job": "train",
  "pack_id": "overnight-traces",
  "base_model": "llama3",
  "seat_tag": "llama3",
  "train_base_model": "Qwen/Qwen2.5-0.5B-Instruct",
  "purpose": "fixture",
  "host_class_affinity": "any",
  "source_paths": [],
  "source_drivers": [],
  "artifacts": [],
  "promoted": false,
  "auto_apply": false,
  "estate_rewritten": false,
  "note": "test"
}
"#;
    std::fs::write(root.join("prepare.json"), body).unwrap();
    let export = root.join("merged");
    merged(&export);
    let missing = run_convert(&root, &export);
    let missing_text = text(&missing);
    assert!(!missing.status.success(), "{missing_text}");
    assert!(missing_text.contains("UNSLOTH.md is missing"), "{missing_text}");
    assert!(!missing_text.contains("--outtype"), "{missing_text}");
    assert!(!root.join("spawned").exists());
    assert!(!root.join("merged.gguf").exists());

    std::fs::write(
        root.join("UNSLOTH.md"),
        "train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"\nseat_tag: \"llama3\"\n",
    )
    .unwrap();
    let out = run_convert(&root, &export);
    let body = text(&out);
    assert!(out.status.success(), "{body}");
    let outfile = root.join("merged.gguf");
    assert!(
        body.contains(&format!(
            "python3 convert_hf_to_gguf.py {} --outfile {} --outtype auto",
            export.display(),
            outfile.display()
        )),
        "{body}"
    );
    assert!(body.contains("--outtype f16"), "{body}");
    assert!(body.contains("--outtype bf16"), "{body}");
    assert!(body.contains("--outtype q8_0"), "{body}");
    assert!(body.contains("does not publish --outtype auto"), "{body}");
    assert!(body.contains("READY_FOR_LIVE_TEST: no"), "{body}");
    assert!(!outfile.exists());
    assert!(!root.join("spawned").exists(), "convert tooling was spawned");
}

#[test]
fn refuses_list_extra_special_tokens_without_spawning_or_copying() {
    let root = tmp("bad-tokenizer");
    write_prepare(&root);
    let export = root.join("export");
    merged(&export);
    std::fs::write(
        export.join("config.json"),
        "{\"model_type\":\"qwen2\"}\n",
    )
    .unwrap();
    std::fs::write(
        export.join("tokenizer_config.json"),
        "{\"extra_special_tokens\":[\"<|im_start|>\",\"<|im_end|>\"],\"tokenizer_class\":\"Qwen2Tokenizer\"}\n",
    )
    .unwrap();
    let before = std::fs::read(export.join("tokenizer_config.json")).unwrap();
    let out = run_convert(&root, &export);
    let body = text(&out);
    assert!(!out.status.success(), "{body}");
    assert!(body.contains("refuse:tokenizer"), "{body}");
    assert!(body.contains("JSON list"), "{body}");
    assert!(body.contains("AttributeError"), "{body}");
    assert!(body.contains("missing vocab.json"), "{body}");
    assert!(body.contains("missing merges.txt"), "{body}");
    assert!(body.contains("tokenizer_config.json.bak"), "{body}");
    assert!(!body.contains("python3 convert_hf_to_gguf.py"), "{body}");
    assert!(!body.contains("--outtype"), "{body}");
    assert!(!root.join("export.gguf").exists());
    assert!(!export.join("tokenizer_config.json.bak").exists());
    assert!(!root.join("spawned").exists(), "convert tooling was spawned");
    assert_eq!(
        std::fs::read(export.join("tokenizer_config.json")).unwrap(),
        before
    );
}

#[test]
fn refuses_null_extra_special_tokens_without_spawning_or_copying() {
    let root = tmp("null-tokenizer");
    write_prepare(&root);
    let export = root.join("export");
    merged(&export);
    std::fs::write(
        export.join("config.json"),
        "{\"model_type\":\"qwen2\"}\n",
    )
    .unwrap();
    std::fs::write(
        export.join("tokenizer_config.json"),
        "{\"extra_special_tokens\":null,\"tokenizer_class\":\"Qwen2Tokenizer\"}\n",
    )
    .unwrap();
    std::fs::write(export.join("vocab.json"), "{}\n").unwrap();
    std::fs::write(export.join("merges.txt"), "a b\n").unwrap();
    let before = std::fs::read(export.join("tokenizer_config.json")).unwrap();
    let vocab_before = std::fs::read(export.join("vocab.json")).unwrap();
    let merges_before = std::fs::read(export.join("merges.txt")).unwrap();
    let out = run_convert(&root, &export);
    let body = text(&out);
    assert!(!out.status.success(), "{body}");
    assert!(body.contains("refuse:tokenizer"), "{body}");
    assert!(body.contains("JSON null"), "{body}");
    assert!(!body.contains("JSON list"), "{body}");
    assert!(!body.contains("missing vocab.json"), "{body}");
    assert!(!body.contains("missing merges.txt"), "{body}");
    assert!(body.contains("tokenizer_config.json.bak"), "{body}");
    assert!(!body.contains("python3 convert_hf_to_gguf.py"), "{body}");
    assert!(!body.contains("--outtype"), "{body}");
    assert!(!root.join("export.gguf").exists());
    assert!(!export.join("tokenizer_config.json.bak").exists());
    assert!(!root.join("spawned").exists(), "convert tooling was spawned");
    assert_eq!(
        std::fs::read(export.join("tokenizer_config.json")).unwrap(),
        before
    );
    assert_eq!(std::fs::read(export.join("vocab.json")).unwrap(), vocab_before);
    assert_eq!(std::fs::read(export.join("merges.txt")).unwrap(), merges_before);
}
