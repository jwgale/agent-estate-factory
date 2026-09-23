//! Post-merge local seat. Validates an export directory or a GGUF and prints
//! the ollama create line. Does not create a model.

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

fn dir_names(dir: &std::path::Path) -> Vec<String> {
    let mut names = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    names
}
