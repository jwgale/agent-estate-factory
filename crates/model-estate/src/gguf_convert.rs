//! Print-only llama.cpp GGUF convert card for a merged Hugging Face export.
//!
//! llama.cpp already converts that directory with `convert_hf_to_gguf.py`.
//! `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, and
//! `axolotl-qlora` prepares point at that directory. Axolotl does not write
//! the GGUF. This module prints the command. It does not vendor the script,
//! spawn it, or write a GGUF.
//!
//! `--outtype auto` is the script default in ggml-org/llama.cpp
//! `convert_hf_to_gguf.py`: choices are f32, f16, bf16, q8_0, tq1_0, tq2_0,
//! and auto, and the default is auto (the highest-fidelity 16-bit float
//! type). This card passes that default so the operator does not guess the
//! flag. It does not pass a quantization type.

use crate::error::ModelError;
use crate::local_seat::{classify_weights, shell_quote, WeightsShape};
use crate::train_enrich::{
    is_axolotl_driver, is_post_merge_print_driver, load_prepare_doc, local_enrich_tag,
    refuse_post_merge_driver, refuse_sacred_and_sku, EnrichJobKind,
};
use feed_collector::{refuse_raw_secrets, FeedError};
use std::path::{Path, PathBuf};

/// Printed plan. `outfile` is the sibling path the convert line names.
/// This command does not create that file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GgufConvertPlan {
    pub seat_tag: String,
    pub local_tag: String,
    pub pack_id: String,
    pub driver: String,
    pub convert_command: String,
    pub outfile: PathBuf,
    pub local_seat_command: String,
    pub report: String,
}

/// Sibling `{parent}/{dirname}.gguf`. A `.gguf` inside the merged directory
/// makes that directory match two shapes.
pub(crate) fn sibling_gguf_outfile(export_dir: &Path) -> PathBuf {
    let stem = export_dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("model");
    let file_name = format!("{stem}.gguf");
    match export_dir.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(file_name),
        _ => PathBuf::from(file_name),
    }
}

/// Exact `convert_hf_to_gguf.py` line. `--outtype auto` is the script default.
pub(crate) fn printed_convert_line(export_dir: &Path) -> String {
    let outfile = sibling_gguf_outfile(export_dir);
    format!(
        "python3 convert_hf_to_gguf.py {} --outfile {} --outtype auto",
        shell_quote(&export_dir.display().to_string()),
        shell_quote(&outfile.display().to_string())
    )
}

pub(crate) fn gguf_convert_cli(prepared: &Path, weights: &Path) -> String {
    format!(
        "estate enrich gguf-convert --prepared {} --weights {}",
        shell_quote(&prepared.display().to_string()),
        shell_quote(&weights.display().to_string())
    )
}

pub(crate) fn local_seat_cli(prepared: &Path, weights: &Path) -> String {
    format!(
        "estate enrich local-seat --prepared {} --weights {}",
        shell_quote(&prepared.display().to_string()),
        shell_quote(&weights.display().to_string())
    )
}

/// Read `prepare.json`, require a merged export, and build the convert report.
/// Does not write and does not spawn a process.
pub fn plan_gguf_convert(
    prepared_dir: &Path,
    weights: &Path,
) -> Result<GgufConvertPlan, ModelError> {
    refuse_sacred_and_sku("prepared", &prepared_dir.display().to_string())?;
    refuse_sacred_and_sku("weights", &weights.display().to_string())?;
    let doc = load_prepare_doc(&prepared_dir.join("prepare.json"))?;
    if !is_post_merge_print_driver(&doc.driver) {
        return Err(refuse_post_merge_driver("gguf-convert", &doc.driver));
    }
    if doc.job != EnrichJobKind::Train.as_str() {
        return Err(ModelError::Other(format!(
            "refuse:job: gguf-convert expects job train, found '{}'",
            doc.job
        )));
    }
    let seat_tag = doc
        .seat_tag
        .clone()
        .ok_or_else(|| ModelError::Other("refuse:seat: prepare.json has no seat_tag".into()))?;
    let local_tag = local_enrich_tag(&doc.pack_id);
    let shape = classify_weights(weights)?;
    let dir = match shape {
        WeightsShape::Merged { dir, .. } => dir,
        WeightsShape::Gguf { file, .. } => {
            return Err(ModelError::Other(format!(
                "refuse:seat: {} is a GGUF. gguf-convert prints the llama.cpp line for a merged export directory (config.json and a .safetensors file whose name does not start with adapter_model). Seat this file with estate enrich local-seat --prepared {} --weights {}.",
                file.display(),
                prepared_dir.display(),
                file.display()
            )));
        }
    };
    let convert = printed_convert_line(&dir);
    let outfile = sibling_gguf_outfile(&dir);
    let seat = local_seat_cli(prepared_dir, &outfile);
    let axolotl_note = if is_axolotl_driver(&doc.driver) {
        "Axolotl does not write GGUF. The operator merged this Hugging Face directory outside this factory. This factory does not invent an Axolotl converter.\n\
         \n"
    } else {
        ""
    };
    let report = format!(
        "gguf-convert: shape=merged seat_tag={seat_tag} local_tag={local_tag}\n\
         pack={pack_id}\n\
         driver={driver}\n\
         weights={weights}\n\
         outfile={outfile}\n\
         promoted=false auto_apply=false estate_rewritten=false\n\
         \n\
         {axolotl_note}\
         Run this from a llama.cpp checkout. convert_hf_to_gguf.py is that checkout's script. Its shebang is python3. --outtype auto is the script default: the highest-fidelity 16-bit float type (f16 or bf16) from the first loaded tensor. This line passes that default so the flag is not guessed. q8_0, tq1_0, and tq2_0 are quantization-style types in that script. This factory does not print them. llama-quantize is the later llama.cpp tool. This factory does not choose a quantization type and does not print a quant command. The outfile is a sibling of the merged directory. A .gguf file inside that directory makes the directory match two shapes, and local-seat and import-trained then refuse the directory.\n\
         \n\
         {convert}\n\
         \n\
         Then seat that file. local-seat prints the ollama create line and, for that GGUF, llama-cli -m and llama-server -m. It does not create the model and does not run those programs.\n\
         \n\
         {seat}\n\
         \n\
         gguf-convert did not convert and did not write {outfile}.\n\
         READY_FOR_LIVE_TEST: no.\n",
        pack_id = doc.pack_id.as_str(),
        driver = doc.driver.as_str(),
        weights = dir.display(),
        outfile = outfile.display(),
        axolotl_note = axolotl_note,
    );
    refuse_sacred_and_sku("gguf-convert report", &report)?;
    refuse_raw_secrets(&report).map_err(map_feed)?;
    Ok(GgufConvertPlan {
        seat_tag,
        local_tag,
        pack_id: doc.pack_id,
        driver: doc.driver,
        convert_command: convert,
        outfile,
        local_seat_command: seat,
        report,
    })
}

fn map_feed(err: FeedError) -> ModelError {
    let text = err.to_string();
    if text.starts_with("refuse:") {
        ModelError::Other(text)
    } else {
        ModelError::Other(format!("refuse:seat: {text}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::train_enrich::{
        AXOLOTL_LORA_ID, AXOLOTL_QLORA_ID, LLAMAFACTORY_LORA_ID, LLAMAFACTORY_QLORA_ID,
        PREPARE_SCHEMA,
    };
    use std::os::unix::fs::PermissionsExt;

    fn tmp(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cell-one-gguf-convert-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_prepare(dir: &Path, driver: &str, job: &str, seat: Option<&str>, promoted: bool) {
        let seat_line = match seat {
            Some(seat) => format!("  \"seat_tag\": \"{seat}\",\n"),
            None => String::new(),
        };
        let body = format!(
            "{{\n\
               \"schema\": \"{PREPARE_SCHEMA}\",\n\
               \"driver\": \"{driver}\",\n\
               \"job\": \"{job}\",\n\
               \"pack_id\": \"overnight-traces\",\n\
               \"base_model\": \"llama3\",\n\
               {seat_line}\
               \"purpose\": \"fixture\",\n\
               \"host_class_affinity\": \"any\",\n\
               \"source_paths\": [],\n\
               \"source_drivers\": [],\n\
               \"artifacts\": [\"export.yaml\"],\n\
               \"promoted\": {promoted},\n\
               \"auto_apply\": false,\n\
               \"estate_rewritten\": false,\n\
               \"note\": \"test\"\n\
             }}\n"
        );
        std::fs::write(dir.join("prepare.json"), body).unwrap();
    }

    fn merged(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("config.json"), "{}\n").unwrap();
        std::fs::write(dir.join("model.safetensors"), b"not-a-real-tensor").unwrap();
    }

    fn gguf_bytes() -> Vec<u8> {
        let mut bytes = b"GGUF".to_vec();
        bytes.extend_from_slice(&[0u8; 12]);
        bytes
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut found = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        found.sort();
        found
    }

    #[test]
    fn source_does_not_spawn_or_convert() {
        let src = include_str!("gguf_convert.rs");
        let command_new = format!("{}::{}", "Command", "new");
        let process_command = format!("{}::{}::{}", "std", "process", "Command");
        assert!(!src.contains(&command_new), "{src}");
        assert!(!src.contains(&process_command), "{src}");
        let seat = include_str!("local_seat.rs");
        assert!(!seat.contains(&command_new), "{seat}");
    }

    #[test]
    fn merged_export_prints_the_llama_cpp_line_and_writes_nothing() {
        let root = tmp("merged");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        merged(&export);
        std::fs::write(export.join("Modelfile"), "FROM .\n").unwrap();
        let before = names(&export);
        let plan = plan_gguf_convert(&root, &export).unwrap();
        let outfile = root.join("export.gguf");
        assert_eq!(plan.outfile, outfile);
        assert_eq!(plan.seat_tag, "llama3");
        assert_eq!(plan.local_tag, "cell-enrich-overnight-traces");
        assert_eq!(plan.driver, LLAMAFACTORY_QLORA_ID);
        let convert = format!(
            "python3 convert_hf_to_gguf.py {} --outfile {} --outtype auto",
            export.display(),
            outfile.display()
        );
        assert_eq!(plan.convert_command, convert);
        assert!(plan.report.contains(&convert), "{}", plan.report);
        assert!(plan.report.contains("shape=merged"), "{}", plan.report);
        assert!(plan.report.contains("seat_tag=llama3"), "{}", plan.report);
        assert!(
            plan.report.contains(&format!(
                "estate enrich local-seat --prepared {} --weights {}",
                root.display(),
                outfile.display()
            )),
            "{}",
            plan.report
        );
        assert_eq!(plan.local_seat_command, local_seat_cli(&root, &outfile));
        assert!(
            plan.report.contains("gguf-convert did not convert"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("READY_FOR_LIVE_TEST: no"),
            "{}",
            plan.report
        );
        assert!(
            !plan.report.contains("READY_FOR_LIVE_TEST: yes"),
            "{}",
            plan.report
        );
        assert!(
            !plan.convert_command.contains("q8_0"),
            "{}",
            plan.convert_command
        );
        assert!(
            !plan.convert_command.contains("llama-quantize"),
            "{}",
            plan.convert_command
        );
        assert!(
            plan.report.contains("does not print a quant command"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("promoted=false"), "{}", plan.report);
        assert_eq!(names(&export), before);
        assert!(!outfile.exists());
        assert!(!export.join("model.gguf").exists());
    }

    #[test]
    fn lora_prepare_uses_the_same_line() {
        let root = tmp("lora");
        write_prepare(&root, LLAMAFACTORY_LORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        merged(&export);
        let plan = plan_gguf_convert(&root, &export).unwrap();
        assert_eq!(plan.driver, LLAMAFACTORY_LORA_ID);
        assert!(
            plan.convert_command.contains("--outtype auto"),
            "{}",
            plan.convert_command
        );
    }

    #[test]
    fn broken_modelfile_still_prints_the_convert_line() {
        let root = tmp("bad-modelfile");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        merged(&export);
        std::fs::write(export.join("Modelfile"), "TEMPLATE \"\"\"x\"\"\"\n").unwrap();
        let plan = plan_gguf_convert(&root, &export).unwrap();
        assert!(
            plan.convert_command.contains("convert_hf_to_gguf.py"),
            "{}",
            plan.report
        );
        assert!(!plan.report.contains("refuse:modelfile"), "{}", plan.report);
    }

    #[test]
    fn shell_quote_wraps_the_printed_convert_line() {
        let root = tmp("quote");
        let spaced = root.join("my export");
        std::fs::create_dir_all(&spaced).unwrap();
        write_prepare(
            &spaced,
            LLAMAFACTORY_QLORA_ID,
            "train",
            Some("llama3"),
            false,
        );
        let export = spaced.join("export dir");
        merged(&export);
        let plan = plan_gguf_convert(&spaced, &export).unwrap();
        let outfile = spaced.join("export dir.gguf");
        assert_eq!(plan.outfile, outfile);
        assert!(
            plan.convert_command
                .contains(&format!("'{}'", export.display())),
            "{}",
            plan.convert_command
        );
        assert!(
            plan.convert_command
                .contains(&format!("'{}'", outfile.display())),
            "{}",
            plan.convert_command
        );
        assert!(
            plan.convert_command.contains("--outtype auto"),
            "{}",
            plan.convert_command
        );
        assert!(!outfile.exists());
    }

    #[test]
    fn refuses_mismatched_shapes() {
        let root = tmp("refuse");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);

        let missing = root.join("missing");
        let err = plan_gguf_convert(&root, &missing).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");
        assert!(err.to_string().contains("missing"), "{err}");

        let empty = root.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        let err = plan_gguf_convert(&root, &empty).unwrap_err();
        assert!(err.to_string().contains("empty directory"), "{err}");

        let adapter = root.join("adapter");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        let err = plan_gguf_convert(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");
        assert!(err.to_string().contains("adapter directory"), "{err}");
        assert!(
            !err.to_string().contains("python3 convert_hf_to_gguf.py"),
            "{err}"
        );

        let shards = root.join("shards");
        std::fs::create_dir_all(&shards).unwrap();
        std::fs::write(shards.join("config.json"), "{}\n").unwrap();
        std::fs::write(
            shards.join("adapter_model-00001-of-00002.safetensors"),
            b"a",
        )
        .unwrap();
        let err = plan_gguf_convert(&root, &shards).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");
        assert!(err.to_string().contains("adapter_model"), "{err}");
        assert!(!err.to_string().contains("--outtype"), "{err}");

        let half = root.join("half");
        std::fs::create_dir_all(&half).unwrap();
        std::fs::write(half.join("config.json"), "{}\n").unwrap();
        let err = plan_gguf_convert(&root, &half).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");

        let tensors = root.join("tensors");
        std::fs::create_dir_all(&tensors).unwrap();
        std::fs::write(tensors.join("model.safetensors"), b"w").unwrap();
        let err = plan_gguf_convert(&root, &tensors).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");

        let gguf = root.join("model.gguf");
        std::fs::write(&gguf, gguf_bytes()).unwrap();
        let err = plan_gguf_convert(&root, &gguf).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("is a GGUF"), "{text}");
        assert!(text.contains("estate enrich local-seat"), "{text}");
        assert!(!text.contains("python3 convert_hf_to_gguf.py"), "{text}");

        let gguf_dir = root.join("only-gguf");
        std::fs::create_dir_all(&gguf_dir).unwrap();
        std::fs::write(gguf_dir.join("model.gguf"), gguf_bytes()).unwrap();
        let err = plan_gguf_convert(&root, &gguf_dir).unwrap_err();
        assert!(err.to_string().contains("is a GGUF"), "{err}");

        let mixed = root.join("mixed");
        merged(&mixed);
        std::fs::write(mixed.join("adapter_config.json"), "{}\n").unwrap();
        let err = plan_gguf_convert(&root, &mixed).unwrap_err();
        assert!(err.to_string().contains("more than one shape"), "{err}");

        let linked = root.join("linked");
        std::os::unix::fs::symlink(&root.join("export"), &linked).unwrap();
        let err = plan_gguf_convert(&root, &linked).unwrap_err();
        assert!(err.to_string().contains("symlink"), "{err}");

        let marked = root.join("marked");
        merged(&marked);
        let real_cfg = root.join("real-config.json");
        std::fs::write(&real_cfg, "{}\n").unwrap();
        std::fs::remove_file(marked.join("config.json")).unwrap();
        std::os::unix::fs::symlink(&real_cfg, marked.join("config.json")).unwrap();
        let err = plan_gguf_convert(&root, &marked).unwrap_err();
        assert!(err.to_string().contains("symlink"), "{err}");
        assert!(!err.to_string().contains("python3"), "{err}");
    }

    #[test]
    fn refuses_prepare_gates() {
        let root = tmp("prepare");
        let export = root.join("export");
        merged(&export);

        write_prepare(&root, "unsloth-qlora", "train", Some("llama3"), false);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:driver"), "{err}");
        assert!(err.to_string().contains("unsloth-qlora"), "{err}");
        assert!(!err.to_string().contains("--outtype"), "{err}");

        write_prepare(&root, "mlx-lm-lora", "train", Some("llama3"), false);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:driver"), "{err}");
        assert!(err.to_string().contains("mlx-lm-lora"), "{err}");
        assert!(!err.to_string().contains("--outtype"), "{err}");

        write_prepare(&root, "ollama-modelfile", "enrich", Some("llama3"), false);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:driver"), "{err}");

        write_prepare(
            &root,
            LLAMAFACTORY_QLORA_ID,
            "enrich",
            Some("llama3"),
            false,
        );
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");

        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", None, false);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");

        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("qwen"), false);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:prepare"), "{err}");

        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), true);
        let err = plan_gguf_convert(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:prepared"), "{err}");

        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let sacred = root.join("cyera-export");
        merged(&sacred);
        let err = plan_gguf_convert(&root, &sacred).unwrap_err();
        assert!(err.to_string().contains("refuse:sacred"), "{err}");
    }

    #[test]
    fn trap_binaries_are_not_required_to_print() {
        let root = tmp("trap");
        write_prepare(&root, LLAMAFACTORY_LORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        merged(&export);
        let bin = root.join("trap-bin");
        std::fs::create_dir_all(&bin).unwrap();
        let sentinel = root.join("spawned");
        let script = format!("#!/bin/sh\necho spawned >> {}\n", sentinel.display());
        for name in [
            "python",
            "python3",
            "llama-quantize",
            "convert_hf_to_gguf.py",
        ] {
            let path = bin.join(name);
            std::fs::write(&path, &script).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let plan = plan_gguf_convert(&root, &export).unwrap();
        assert!(plan
            .convert_command
            .contains("python3 convert_hf_to_gguf.py"));
        assert!(!sentinel.exists());
        assert!(!root.join("export.gguf").exists());
    }

    #[test]
    fn axolotl_merged_dir_prints_the_llama_cpp_line() {
        for driver in [AXOLOTL_LORA_ID, AXOLOTL_QLORA_ID] {
            let root = tmp(&format!("ax-{driver}"));
            write_prepare(&root, driver, "train", Some("llama3"), false);
            assert!(!root.join("export.yaml").exists());
            let merged_dir = root.join("merged");
            merged(&merged_dir);
            let before = names(&merged_dir);
            let plan = plan_gguf_convert(&root, &merged_dir).unwrap();
            let outfile = root.join("merged.gguf");
            assert_eq!(plan.driver, driver);
            assert_eq!(plan.outfile, outfile);
            assert_eq!(plan.seat_tag, "llama3");
            assert_eq!(plan.local_tag, "cell-enrich-overnight-traces");
            let convert = format!(
                "python3 convert_hf_to_gguf.py {} --outfile {} --outtype auto",
                merged_dir.display(),
                outfile.display()
            );
            assert_eq!(plan.convert_command, convert);
            assert_eq!(plan.local_seat_command, local_seat_cli(&root, &outfile));
            assert!(plan.report.contains(&convert), "{}", plan.report);
            assert!(
                plan.report.contains("Axolotl does not write GGUF"),
                "{}",
                plan.report
            );
            assert!(
                plan.report.contains("does not invent an Axolotl converter"),
                "{}",
                plan.report
            );
            assert!(!plan.report.contains("Axolotl wrote"), "{}", plan.report);
            assert!(
                !plan.report.contains("LLaMA-Factory wrote"),
                "{}",
                plan.report
            );
            assert!(!plan.report.contains("llamafactory-cli"), "{}", plan.report);
            assert!(!plan.report.contains("merge-lora"), "{}", plan.report);
            assert!(
                plan.report.contains("READY_FOR_LIVE_TEST: no"),
                "{}",
                plan.report
            );
            assert!(
                !plan.report.contains("READY_FOR_LIVE_TEST: yes"),
                "{}",
                plan.report
            );
            assert!(!outfile.exists());
            assert_eq!(names(&merged_dir), before);

            std::fs::write(merged_dir.join("export.yaml"), "export_dir: merged\n").unwrap();
            let beside = plan_gguf_convert(&root, &merged_dir).unwrap();
            assert!(
                beside.convert_command.contains("--outtype auto"),
                "{}",
                beside.convert_command
            );
            assert!(!outfile.exists());
        }
    }

    #[test]
    fn axolotl_refuses_adapter_lf_card_and_symlink() {
        let root = tmp("ax-refuse");
        write_prepare(&root, AXOLOTL_QLORA_ID, "train", Some("llama3"), false);

        let adapter = root.join("outputs");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        std::fs::write(adapter.join("adapter_model.safetensors"), b"w").unwrap();
        let err = plan_gguf_convert(&root, &adapter).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("adapter directory"), "{text}");
        assert!(!text.contains("python3 convert_hf_to_gguf.py"), "{text}");

        let shards = root.join("shards");
        std::fs::create_dir_all(&shards).unwrap();
        std::fs::write(shards.join("config.json"), "{}\n").unwrap();
        std::fs::write(shards.join("adapter_model.safetensors"), b"a").unwrap();
        let err = plan_gguf_convert(&root, &shards).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("adapter_model"), "{text}");
        assert!(!text.contains("--outtype"), "{text}");

        let card = root.join("export-card");
        std::fs::create_dir_all(&card).unwrap();
        std::fs::write(card.join("export.yaml"), "adapter_name_or_path: outputs\n").unwrap();
        let err = plan_gguf_convert(&root, &card).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("not a merged export directory"), "{text}");
        assert!(!text.contains("--outtype"), "{text}");

        let card_file = root.join("export.yaml");
        std::fs::write(&card_file, "export_dir: export\n").unwrap();
        let err = plan_gguf_convert(&root, &card_file).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(!text.contains("python3 convert_hf_to_gguf.py"), "{text}");

        let err = plan_gguf_convert(&root, &root).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");
        assert!(
            !err.to_string().contains("python3 convert_hf_to_gguf.py"),
            "{err}"
        );

        let real = root.join("real-merged");
        merged(&real);
        let linked = root.join("linked");
        std::os::unix::fs::symlink(&real, &linked).unwrap();
        let err = plan_gguf_convert(&root, &linked).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("symlink"), "{text}");
        assert!(!text.contains("--outtype"), "{text}");

        let marked = root.join("marked");
        merged(&marked);
        let real_cfg = root.join("real-config.json");
        std::fs::write(&real_cfg, "{}\n").unwrap();
        std::fs::remove_file(marked.join("config.json")).unwrap();
        std::os::unix::fs::symlink(&real_cfg, marked.join("config.json")).unwrap();
        let err = plan_gguf_convert(&root, &marked).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("symlink"), "{text}");
        assert!(!text.contains("python3"), "{text}");

        write_prepare(&root, AXOLOTL_LORA_ID, "enrich", Some("llama3"), false);
        let err = plan_gguf_convert(&root, &real).unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");

        write_prepare(&root, AXOLOTL_LORA_ID, "train", Some("qwen"), false);
        let err = plan_gguf_convert(&root, &real).unwrap_err();
        assert!(err.to_string().contains("refuse:prepare"), "{err}");

        write_prepare(&root, AXOLOTL_LORA_ID, "train", Some("llama3"), true);
        let err = plan_gguf_convert(&root, &real).unwrap_err();
        assert!(err.to_string().contains("refuse:prepared"), "{err}");

        write_prepare(&root, AXOLOTL_QLORA_ID, "train", Some("llama3"), false);
        let sacred = root.join("cyera-merged");
        merged(&sacred);
        let err = plan_gguf_convert(&root, &sacred).unwrap_err();
        assert!(err.to_string().contains("refuse:sacred"), "{err}");
        assert!(!err.to_string().contains("--outtype"), "{err}");
    }
}
