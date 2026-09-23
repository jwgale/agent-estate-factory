//! Post-merge local seat. Validates an export directory or a GGUF and prints
//! the ollama create line. Does not create a model.

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
        "cell-one-local-seat-cli-{}-{}",
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

#[test]
fn help_names_local_seat() {
    let out = estate_bin().args(["help", "enrich"]).output().unwrap();
    let body = text(&out);
    assert!(out.status.success(), "{body}");
    assert!(body.contains("estate enrich local-seat"), "{body}");
    assert!(body.contains("estate enrich gguf-convert"), "{body}");
    assert!(body.contains("--outtype auto"), "{body}");
    assert!(body.contains("convert_hf_to_gguf.py"), "{body}");
    assert!(body.contains("does not run ollama or llama.cpp"), "{body}");
    assert!(body.contains("axolotl-lora"), "{body}");
    assert!(body.contains("outputs/merged"), "{body}");
    assert!(body.contains("estate enrich merge-adapt"), "{body}");
    assert!(body.contains("--adapter"), "{body}");
    assert!(body.contains("ADAPTER"), "{body}");
    assert!(body.contains("llama-cli -m"), "{body}");
    assert!(body.contains("llama-server -m"), "{body}");
    assert!(body.contains("--runtime llama.cpp"), "{body}");
    assert!(body.contains("refuse:runtime"), "{body}");
    assert!(body.contains("READY_FOR_LIVE_TEST stays no"), "{body}");
    assert!(!body.contains("READY_FOR_LIVE_TEST: yes"), "{body}");
}

#[test]
fn prepare_records_paths_and_local_seat_prints_the_create_line() {
    let root = tmp("prepare");
    let estate = write_train_estate(&root, "llama3", "Qwen/Qwen2.5-0.5B-Instruct");
    let pack = repo_root().join("examples/fixtures/specialist-overnight.pack.json");
    let out_dir = root.join("qlora");
    let prepared = estate_bin()
        .args([
            "enrich",
            "prepare",
            "--estate",
            estate.to_str().unwrap(),
            "--pack",
            pack.to_str().unwrap(),
            "--driver",
            "llamafactory-qlora",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let prepared_text = text(&prepared);
    assert!(prepared.status.success(), "{prepared_text}");
    let prepare_json = std::fs::read_to_string(out_dir.join("prepare.json")).unwrap();
    assert!(prepare_json.contains("\"export_yaml\":"), "{prepare_json}");
    assert!(
        prepare_json.contains(&format!(
            "\"export_yaml\": \"{}\"",
            out_dir.join("export.yaml").display()
        )),
        "{prepare_json}"
    );
    assert!(out_dir.join("export.yaml").is_file());
    assert!(!prepare_json.contains("\"modelfile\""), "{prepare_json}");
    assert!(
        prepare_json.contains("\"seat_tag\": \"llama3\""),
        "{prepare_json}"
    );
    assert!(
        prepare_json.contains("\"promoted\": false"),
        "{prepare_json}"
    );
    for name in ["NEXT.md", "PREPARE.md"] {
        let page = std::fs::read_to_string(out_dir.join(name)).unwrap();
        assert!(
            page.contains("## Local seat after export"),
            "{name}: {page}"
        );
        assert!(page.contains("estate enrich local-seat"), "{name}");
        assert!(
            page.contains(&format!(
                "estate enrich local-seat --prepared {} --adapter {}",
                out_dir.display(),
                out_dir.join("outputs").display()
            )),
            "{name}: {page}"
        );
        assert!(page.contains("estate enrich gguf-convert"), "{name}");
        assert!(
            page.contains(&format!(
                "python3 convert_hf_to_gguf.py {} --outfile {} --outtype auto",
                out_dir.join("export").display(),
                out_dir.join("export.gguf").display()
            )),
            "{name}: {page}"
        );
        assert!(page.contains("convert_hf_to_gguf.py"), "{name}");
        assert!(page.contains("Seat tag is llama3"), "{name}");
        assert!(page.contains("import-trained"), "{name}");
        assert!(page.contains("READY_FOR_LIVE_TEST: no"), "{name}");
    }

    let ollama_out = root.join("ollama");
    let ollama = estate_bin()
        .args([
            "enrich",
            "prepare",
            "--estate",
            estate.to_str().unwrap(),
            "--pack",
            pack.to_str().unwrap(),
            "--driver",
            "ollama-modelfile",
            "--out",
            ollama_out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(ollama.status.success(), "{}", text(&ollama));
    let ollama_json = std::fs::read_to_string(ollama_out.join("prepare.json")).unwrap();
    assert!(
        ollama_json.contains(&format!(
            "\"modelfile\": \"{}\"",
            ollama_out.join("Modelfile").display()
        )),
        "{ollama_json}"
    );
    assert!(!ollama_json.contains("\"export_yaml\""), "{ollama_json}");

    let export = root.join("export");
    std::fs::create_dir_all(&export).unwrap();
    std::fs::write(export.join("config.json"), "{}\n").unwrap();
    std::fs::write(export.join("model.safetensors"), b"weights").unwrap();
    std::fs::write(
        export.join("Modelfile"),
        "FROM .\nTEMPLATE \"\"\"kept\"\"\"\n",
    )
    .unwrap();
    let before = std::fs::read(export.join("Modelfile")).unwrap();
    let names_before = dir_names(&export);
    let seat = estate_bin()
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--weights",
            export.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let seat_text = text(&seat);
    assert!(seat.status.success(), "{seat_text}");
    assert!(seat_text.contains("shape=merged"), "{seat_text}");
    assert!(seat_text.contains("seat_tag=llama3"), "{seat_text}");
    assert!(
        seat_text.contains("local_tag=cell-enrich-overnight-traces"),
        "{seat_text}"
    );
    assert!(seat_text.contains("modelfile_on_disk=true"), "{seat_text}");
    assert!(
        seat_text.contains(&format!(
            "ollama create cell-enrich-overnight-traces -f {}",
            export.join("Modelfile").display()
        )),
        "{seat_text}"
    );
    assert!(seat_text.contains("import-trained"), "{seat_text}");
    assert!(
        seat_text.contains("local-seat did not create a model."),
        "{seat_text}"
    );
    assert!(seat_text.contains("READY_FOR_LIVE_TEST: no"), "{seat_text}");
    assert!(
        !seat_text.contains("READY_FOR_LIVE_TEST: yes"),
        "{seat_text}"
    );
    assert_eq!(std::fs::read(export.join("Modelfile")).unwrap(), before);
    assert_eq!(dir_names(&export), names_before);

    let missing = estate_bin()
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--weights",
            root.join("missing-export").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let missing_text = text(&missing);
    assert!(!missing.status.success(), "{missing_text}");
    assert!(missing_text.contains("refuse:seat"), "{missing_text}");
    assert!(missing_text.contains("missing"), "{missing_text}");

    let adapter = root.join("adapter");
    std::fs::create_dir_all(&adapter).unwrap();
    std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
    let refused = estate_bin()
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--weights",
            adapter.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let refused_text = text(&refused);
    assert!(!refused.status.success(), "{refused_text}");
    assert!(refused_text.contains("refuse:seat"), "{refused_text}");
    assert!(refused_text.contains("adapter directory"), "{refused_text}");
    assert!(!adapter.join("Modelfile").exists());
}

fn write_axolotl_prepare(dir: &std::path::Path, driver: &str) {
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
fn axolotl_prepare_prints_create_and_refuses_adapter_and_symlink() {
    for driver in ["axolotl-lora", "axolotl-qlora"] {
        let root = tmp(&format!("ax-{driver}"));
        write_axolotl_prepare(&root, driver);
        let merged_dir = root.join("merged");
        std::fs::create_dir_all(&merged_dir).unwrap();
        std::fs::write(merged_dir.join("config.json"), "{}\n").unwrap();
        std::fs::write(merged_dir.join("model.safetensors"), b"weights").unwrap();
        let seat = estate_bin()
            .args([
                "enrich",
                "local-seat",
                "--prepared",
                root.to_str().unwrap(),
                "--weights",
                merged_dir.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        let seat_text = text(&seat);
        assert!(seat.status.success(), "{driver}: {seat_text}");
        assert!(seat_text.contains("shape=merged"), "{driver}: {seat_text}");
        assert!(
            seat_text.contains("seat_tag=llama3"),
            "{driver}: {seat_text}"
        );
        assert!(
            seat_text.contains(&format!(
                "ollama create cell-enrich-overnight-traces -f {}",
                merged_dir.join("Modelfile").display()
            )),
            "{driver}: {seat_text}"
        );
        assert!(
            seat_text.contains("Axolotl does not write GGUF"),
            "{driver}: {seat_text}"
        );
        assert!(
            !seat_text.contains("LLaMA-Factory wrote"),
            "{driver}: {seat_text}"
        );
        assert!(
            seat_text.contains("local-seat did not create a model."),
            "{driver}: {seat_text}"
        );
        assert!(
            seat_text.contains("READY_FOR_LIVE_TEST: no"),
            "{driver}: {seat_text}"
        );
        assert!(!merged_dir.join("Modelfile").exists(), "{driver}");
        assert!(!root.join("merged.gguf").exists(), "{driver}");

        let gguf = root.join("model.gguf");
        let mut bytes = b"GGUF".to_vec();
        bytes.extend_from_slice(&[0u8; 12]);
        std::fs::write(&gguf, bytes).unwrap();
        let gguf_seat = estate_bin()
            .args([
                "enrich",
                "local-seat",
                "--prepared",
                root.to_str().unwrap(),
                "--weights",
                gguf.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        let gguf_text = text(&gguf_seat);
        assert!(gguf_seat.status.success(), "{driver}: {gguf_text}");
        assert!(gguf_text.contains("shape=gguf"), "{driver}: {gguf_text}");
        assert!(
            gguf_text.contains("Axolotl did not write this GGUF"),
            "{driver}: {gguf_text}"
        );
        assert!(gguf_text.contains("ollama create"), "{driver}: {gguf_text}");
        assert!(!root.join("Modelfile").exists(), "{driver}");

        let adapter = root.join("outputs");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        let refused = estate_bin()
            .args([
                "enrich",
                "local-seat",
                "--prepared",
                root.to_str().unwrap(),
                "--weights",
                adapter.to_str().unwrap(),
            ])
            .output()
            .unwrap();
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
            !refused_text.contains("ollama create"),
            "{driver}: {refused_text}"
        );

        let card = root.join("export-card");
        std::fs::create_dir_all(&card).unwrap();
        std::fs::write(card.join("export.yaml"), "adapter_name_or_path: outputs\n").unwrap();
        let card_seat = estate_bin()
            .args([
                "enrich",
                "local-seat",
                "--prepared",
                root.to_str().unwrap(),
                "--weights",
                card.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        let card_text = text(&card_seat);
        assert!(!card_seat.status.success(), "{driver}: {card_text}");
        assert!(card_text.contains("refuse:seat"), "{driver}: {card_text}");
        assert!(
            !card_text.contains("ollama create"),
            "{driver}: {card_text}"
        );

        let linked = root.join("linked");
        std::os::unix::fs::symlink(&merged_dir, &linked).unwrap();
        let link_seat = estate_bin()
            .args([
                "enrich",
                "local-seat",
                "--prepared",
                root.to_str().unwrap(),
                "--weights",
                linked.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        let link_text = text(&link_seat);
        assert!(!link_seat.status.success(), "{driver}: {link_text}");
        assert!(link_text.contains("refuse:seat"), "{driver}: {link_text}");
        assert!(link_text.contains("symlink"), "{driver}: {link_text}");
        assert!(
            !link_text.contains("ollama create"),
            "{driver}: {link_text}"
        );
    }
}

#[test]
fn adapter_seat_prints_the_modelfile_and_does_not_run_it() {
    let root = tmp("adapter-cli");
    let estate = write_train_estate(&root, "llama3", "Qwen/Qwen2.5-0.5B-Instruct");
    let pack = repo_root().join("examples/fixtures/specialist-overnight.pack.json");
    let out_dir = root.join("qlora");
    let prepared = estate_bin()
        .args([
            "enrich",
            "prepare",
            "--estate",
            estate.to_str().unwrap(),
            "--pack",
            pack.to_str().unwrap(),
            "--driver",
            "llamafactory-qlora",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(prepared.status.success(), "{}", text(&prepared));
    let estate_before = std::fs::read(&estate).unwrap();

    let adapter = root.join("outputs");
    std::fs::create_dir_all(&adapter).unwrap();
    std::fs::write(adapter.join("adapter_config.json"), "{\"r\":8}\n").unwrap();
    std::fs::write(adapter.join("adapter_model.safetensors"), b"weights").unwrap();
    let before = dir_names(&adapter);

    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let marker = root.join("ollama-was-run");
    let script = bin.join("ollama");
    std::fs::write(&script, format!("#!/bin/sh\ntouch {}\n", marker.display())).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = std::env::var("PATH").unwrap_or_default();
    let seat = estate_bin()
        .env("PATH", format!("{}:{path}", bin.display()))
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--adapter",
            adapter.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let seat_text = text(&seat);
    assert!(seat.status.success(), "{seat_text}");
    assert!(seat_text.contains("shape=adapter"), "{seat_text}");
    assert!(seat_text.contains("seat_tag=llama3"), "{seat_text}");
    assert!(seat_text.contains("FROM llama3\n"), "{seat_text}");
    assert!(seat_text.contains("ADAPTER "), "{seat_text}");
    assert!(!seat_text.contains("FROM Qwen/"), "{seat_text}");
    assert!(
        seat_text.contains("Qwen/Qwen2.5-0.5B-Instruct"),
        "{seat_text}"
    );
    assert!(
        seat_text.contains("ollama create cell-enrich-overnight-traces -f "),
        "{seat_text}"
    );
    assert!(seat_text.contains("was not run"), "{seat_text}");
    assert!(
        seat_text.contains("local-seat did not create a model."),
        "{seat_text}"
    );
    assert!(seat_text.contains("READY_FOR_LIVE_TEST: no"), "{seat_text}");
    assert!(
        !seat_text.contains("READY_FOR_LIVE_TEST: yes"),
        "{seat_text}"
    );
    assert!(!marker.exists(), "printed ollama create was executed");
    assert_eq!(dir_names(&adapter), before);
    assert!(!adapter.join("Modelfile").exists());
    assert_eq!(std::fs::read(&estate).unwrap(), estate_before);

    let merged = root.join("export");
    std::fs::create_dir_all(&merged).unwrap();
    std::fs::write(merged.join("config.json"), "{}\n").unwrap();
    std::fs::write(merged.join("model.safetensors"), b"merged").unwrap();
    let refused = estate_bin()
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--adapter",
            merged.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let refused_text = text(&refused);
    assert!(!refused.status.success(), "{refused_text}");
    assert!(refused_text.contains("refuse:adapter"), "{refused_text}");
    assert!(refused_text.contains("merged"), "{refused_text}");
    assert!(!refused_text.contains("ollama create"), "{refused_text}");
    assert!(!marker.exists(), "refused path executed ollama");

    let missing_config = root.join("shards");
    std::fs::create_dir_all(&missing_config).unwrap();
    std::fs::write(missing_config.join("adapter_model.safetensors"), b"w").unwrap();
    let missing = estate_bin()
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--adapter",
            missing_config.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let missing_text = text(&missing);
    assert!(!missing.status.success(), "{missing_text}");
    assert!(missing_text.contains("refuse:adapter"), "{missing_text}");
    assert!(
        missing_text.contains("no adapter_config.json"),
        "{missing_text}"
    );
    assert!(!marker.exists());

    let real = root.join("real");
    std::fs::create_dir_all(&real).unwrap();
    std::fs::write(real.join("adapter_config.json"), "{}\n").unwrap();
    std::fs::write(real.join("adapter_model.safetensors"), b"w").unwrap();
    let linked = root.join("linked");
    std::os::unix::fs::symlink(&real, &linked).unwrap();
    let link = estate_bin()
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--adapter",
            linked.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let link_text = text(&link);
    assert!(!link.status.success(), "{link_text}");
    assert!(link_text.contains("refuse:adapter"), "{link_text}");
    assert!(link_text.contains("symlink"), "{link_text}");
    assert!(!marker.exists());

    let both = estate_bin()
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--weights",
            merged.to_str().unwrap(),
            "--adapter",
            adapter.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let both_text = text(&both);
    assert!(!both.status.success(), "{both_text}");
    assert!(!marker.exists());
}

#[test]
fn gguf_seat_prints_llama_cpp_lines_and_does_not_run_them() {
    let root = tmp("gguf-cli");
    let estate = write_train_estate(&root, "llama3", "Qwen/Qwen2.5-0.5B-Instruct");
    let pack = repo_root().join("examples/fixtures/specialist-overnight.pack.json");
    let out_dir = root.join("qlora");
    let prepared = estate_bin()
        .args([
            "enrich",
            "prepare",
            "--estate",
            estate.to_str().unwrap(),
            "--pack",
            pack.to_str().unwrap(),
            "--driver",
            "llamafactory-qlora",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(prepared.status.success(), "{}", text(&prepared));
    let estate_before = std::fs::read(&estate).unwrap();

    let dir = root.join("one");
    std::fs::create_dir_all(&dir).unwrap();
    let gguf = dir.join("model.gguf");
    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&[0u8; 12]);
    std::fs::write(&gguf, &bytes).unwrap();
    let before = dir_names(&dir);

    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let marker = root.join("runtime-was-run");
    for name in ["ollama", "llama-cli", "llama-server"] {
        let script = bin.join(name);
        std::fs::write(&script, format!("#!/bin/sh\ntouch {}\n", marker.display())).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = std::env::var("PATH").unwrap_or_default();
    let seat = estate_bin()
        .env("PATH", format!("{}:{path}", bin.display()))
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--weights",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let seat_text = text(&seat);
    assert!(seat.status.success(), "{seat_text}");
    assert!(seat_text.contains("shape=gguf"), "{seat_text}");
    assert!(seat_text.contains("runtime=ollama"), "{seat_text}");
    assert!(
        seat_text.contains("ollama create cell-enrich-overnight-traces -f "),
        "{seat_text}"
    );
    assert!(seat_text.contains("llama-cli -m "), "{seat_text}");
    assert!(
        seat_text.contains(gguf.file_name().unwrap().to_str().unwrap()),
        "{seat_text}"
    );
    assert!(seat_text.contains("llama-server -m "), "{seat_text}");
    assert!(seat_text.contains("--port 8080"), "{seat_text}");
    assert!(seat_text.contains("READY_FOR_LIVE_TEST: no"), "{seat_text}");
    let ollama_at = seat_text.find("ollama create").unwrap();
    let cli_at = seat_text.find("llama-cli -m").unwrap();
    assert!(ollama_at < cli_at, "{seat_text}");
    assert!(
        !marker.exists(),
        "printed llama.cpp or ollama line was executed"
    );
    assert_eq!(dir_names(&dir), before);
    assert!(!dir.join("Modelfile").exists());
    assert_eq!(std::fs::read(&estate).unwrap(), estate_before);

    let selected = estate_bin()
        .env("PATH", format!("{}:{path}", bin.display()))
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--weights",
            gguf.to_str().unwrap(),
            "--runtime",
            "llama.cpp",
        ])
        .output()
        .unwrap();
    let selected_text = text(&selected);
    assert!(selected.status.success(), "{selected_text}");
    assert!(
        selected_text.contains("runtime=llama.cpp"),
        "{selected_text}"
    );
    assert!(selected_text.contains("llama-cli -m "), "{selected_text}");
    assert!(selected_text.contains("ollama create"), "{selected_text}");
    let cli_at = selected_text.find("llama-cli -m").unwrap();
    let ollama_at = selected_text.find("ollama create").unwrap();
    assert!(cli_at < ollama_at, "{selected_text}");
    assert!(!marker.exists(), "selected runtime executed a program");
    assert!(!root.join("Modelfile").exists());

    let export = out_dir.join("export");
    std::fs::create_dir_all(&export).unwrap();
    std::fs::write(export.join("config.json"), "{}\n").unwrap();
    std::fs::write(export.join("model.safetensors"), b"w").unwrap();
    let merged = estate_bin()
        .env("PATH", format!("{}:{path}", bin.display()))
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--weights",
            export.to_str().unwrap(),
            "--runtime",
            "llama.cpp",
        ])
        .output()
        .unwrap();
    let merged_text = text(&merged);
    assert!(merged.status.success(), "{merged_text}");
    assert!(merged_text.contains("shape=merged"), "{merged_text}");
    assert!(
        merged_text.contains("convert_hf_to_gguf.py"),
        "{merged_text}"
    );
    assert!(
        merged_text.contains("does not load this Hugging Face directory"),
        "{merged_text}"
    );
    assert!(
        !merged_text.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("llama-cli ") || trimmed.starts_with("llama-server ")
        }),
        "{merged_text}"
    );
    assert!(merged_text.contains("ollama create"), "{merged_text}");
    assert!(!export.join("export.gguf").exists());
    assert!(!out_dir.join("export.gguf").exists());
    assert!(!marker.exists());

    let adapter = root.join("outputs");
    std::fs::create_dir_all(&adapter).unwrap();
    std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
    let refused = estate_bin()
        .env("PATH", format!("{}:{path}", bin.display()))
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--adapter",
            adapter.to_str().unwrap(),
            "--runtime",
            "llama.cpp",
        ])
        .output()
        .unwrap();
    let refused_text = text(&refused);
    assert!(!refused.status.success(), "{refused_text}");
    assert!(refused_text.contains("refuse:runtime"), "{refused_text}");
    assert!(!refused_text.contains("llama-cli"), "{refused_text}");
    assert!(!refused_text.contains("ollama create"), "{refused_text}");
    assert!(!adapter.join("Modelfile").exists());
    assert!(!marker.exists());

    let other = estate_bin()
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            out_dir.to_str().unwrap(),
            "--weights",
            gguf.to_str().unwrap(),
            "--runtime",
            "mlx",
        ])
        .output()
        .unwrap();
    let other_text = text(&other);
    assert!(!other.status.success(), "{other_text}");
    assert!(other_text.contains("refuse:runtime"), "{other_text}");
    assert!(other_text.contains("mlx"), "{other_text}");
    assert!(!other_text.contains("llama-cli"), "{other_text}");
    assert!(!marker.exists());
}

#[test]
fn mlx_seats_a_gguf_file_and_refuses_the_fused_directory() {
    let root = tmp("mlx-seat");
    let body = r#"{
  "schema": "cell-one.enrich-prepare.v0",
  "driver": "mlx-lm-lora",
  "job": "train",
  "pack_id": "overnight-traces",
  "base_model": "llama3",
  "seat_tag": "llama3",
  "train_base_model": "Qwen/Qwen2.5-0.5B-Instruct",
  "purpose": "fixture",
  "host_class_affinity": "apple-silicon",
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
    std::fs::write(
        root.join("MLX.md"),
        "train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"\nseat_tag: \"llama3\"\nhost_class_affinity: apple-silicon\n",
    )
    .unwrap();
    let gguf = root.join("ggml-model-f16.gguf");
    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&[0u8; 12]);
    std::fs::write(&gguf, bytes).unwrap();

    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let marker = root.join("spawned");
    let script = format!("#!/bin/sh\ntouch {}\n", marker.display());
    for name in [
        "ollama",
        "llama-cli",
        "llama-server",
        "mlx_lm.fuse",
        "mlx_lm.lora",
    ] {
        let path = bin.join(name);
        std::fs::write(&path, &script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = std::env::var("PATH").unwrap_or_default();
    let seat = estate_bin()
        .env("PATH", format!("{}:{path}", bin.display()))
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            root.to_str().unwrap(),
            "--weights",
            gguf.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let seat_text = text(&seat);
    assert!(seat.status.success(), "{seat_text}");
    assert!(seat_text.contains("shape=gguf"), "{seat_text}");
    assert!(seat_text.contains("driver=mlx-lm-lora"), "{seat_text}");
    assert!(seat_text.contains("ollama create"), "{seat_text}");
    assert!(seat_text.contains("llama-cli -m"), "{seat_text}");
    assert!(
        seat_text.contains("mlx_lm.fuse --export-gguf"),
        "{seat_text}"
    );
    assert!(seat_text.contains("READY_FOR_LIVE_TEST: no"), "{seat_text}");
    assert!(
        !seat_text.contains("python3 convert_hf_to_gguf.py"),
        "{seat_text}"
    );
    assert!(!seat_text.contains("FROM llama3\nADAPTER"), "{seat_text}");
    assert!(!root.join("Modelfile").exists());
    assert!(!marker.exists());

    let fused = root.join("fused_model");
    std::fs::create_dir_all(&fused).unwrap();
    std::fs::write(fused.join("config.json"), "{}\n").unwrap();
    std::fs::write(fused.join("model.safetensors"), b"w").unwrap();
    let refused = estate_bin()
        .env("PATH", format!("{}:{path}", bin.display()))
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            root.to_str().unwrap(),
            "--weights",
            fused.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let refused_text = text(&refused);
    assert!(!refused.status.success(), "{refused_text}");
    assert!(refused_text.contains("refuse:seat"), "{refused_text}");
    assert!(refused_text.contains("MLX weights"), "{refused_text}");
    assert!(!refused_text.contains("ollama create"), "{refused_text}");

    let adapters = root.join("adapters");
    std::fs::create_dir_all(&adapters).unwrap();
    std::fs::write(adapters.join("adapter_config.json"), "{}\n").unwrap();
    std::fs::write(adapters.join("adapters.safetensors"), b"w").unwrap();
    let adapter_seat = estate_bin()
        .env("PATH", format!("{}:{path}", bin.display()))
        .args([
            "enrich",
            "local-seat",
            "--prepared",
            root.to_str().unwrap(),
            "--adapter",
            adapters.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let adapter_text = text(&adapter_seat);
    assert!(!adapter_seat.status.success(), "{adapter_text}");
    assert!(adapter_text.contains("refuse:adapter"), "{adapter_text}");
    assert!(!adapter_text.contains("ollama create"), "{adapter_text}");
    assert!(!adapter_text.contains("FROM llama3"), "{adapter_text}");
    assert!(!adapters.join("Modelfile").exists());
    assert!(!marker.exists());
}

fn dir_names(dir: &std::path::Path) -> Vec<String> {
    let mut names = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn unsloth_seats_merged_and_gguf_and_refuses_the_adapter() {
    let root = tmp("unsloth-seat");
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
    std::fs::write(
        root.join("UNSLOTH.md"),
        "train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"\nseat_tag: \"llama3\"\n",
    )
    .unwrap();
    let merged = root.join("merged");
    std::fs::create_dir_all(&merged).unwrap();
    std::fs::write(merged.join("config.json"), "{}\n").unwrap();
    std::fs::write(merged.join("model.safetensors"), b"w").unwrap();
    let gguf = root.join("model.gguf");
    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&[0u8; 12]);
    std::fs::write(&gguf, bytes).unwrap();
    let adapter = root.join("lora");
    std::fs::create_dir_all(&adapter).unwrap();
    std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
    std::fs::write(adapter.join("adapter_model.safetensors"), b"w").unwrap();

    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let marker = root.join("spawned");
    let script = format!("#!/bin/sh\ntouch {}\n", marker.display());
    for name in [
        "python",
        "python3",
        "ollama",
        "unsloth",
        "llama-cli",
        "llama-server",
        "convert_hf_to_gguf.py",
    ] {
        let path = bin.join(name);
        std::fs::write(&path, &script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = std::env::var("PATH").unwrap_or_default();
    let seat = |args: &[&str]| {
        estate_bin()
            .env("PATH", format!("{}:{path}", bin.display()))
            .args(args)
            .output()
            .unwrap()
    };

    let merged_out = seat(&[
        "enrich",
        "local-seat",
        "--prepared",
        root.to_str().unwrap(),
        "--weights",
        merged.to_str().unwrap(),
    ]);
    let merged_text = text(&merged_out);
    assert!(merged_out.status.success(), "{merged_text}");
    assert!(merged_text.contains("ollama create"), "{merged_text}");
    assert!(!merged_text.contains("llama-cli -m"), "{merged_text}");
    assert!(merged_text.contains("READY_FOR_LIVE_TEST: no"), "{merged_text}");
    assert!(!merged.join("Modelfile").exists());
    assert!(!marker.exists());

    let gguf_out = seat(&[
        "enrich",
        "local-seat",
        "--prepared",
        root.to_str().unwrap(),
        "--weights",
        gguf.to_str().unwrap(),
    ]);
    let gguf_text = text(&gguf_out);
    assert!(gguf_out.status.success(), "{gguf_text}");
    assert!(gguf_text.contains("ollama create"), "{gguf_text}");
    assert!(gguf_text.contains("llama-cli -m"), "{gguf_text}");
    assert!(!marker.exists());

    let adapter_out = seat(&[
        "enrich",
        "local-seat",
        "--prepared",
        root.to_str().unwrap(),
        "--adapter",
        adapter.to_str().unwrap(),
    ]);
    let adapter_text = text(&adapter_out);
    assert!(!adapter_out.status.success(), "{adapter_text}");
    assert!(adapter_text.contains("refuse:adapter"), "{adapter_text}");
    assert!(!adapter_text.contains("ADAPTER "), "{adapter_text}");
    assert!(!adapter_text.contains("ollama create"), "{adapter_text}");

    let weights_out = seat(&[
        "enrich",
        "local-seat",
        "--prepared",
        root.to_str().unwrap(),
        "--weights",
        adapter.to_str().unwrap(),
    ]);
    let weights_text = text(&weights_out);
    assert!(!weights_out.status.success(), "{weights_text}");
    assert!(weights_text.contains("refuse:seat"), "{weights_text}");
    assert!(!weights_text.contains("Pass --adapter"), "{weights_text}");
    assert!(!weights_text.contains("ADAPTER "), "{weights_text}");
    assert!(!marker.exists());
}
