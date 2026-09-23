//! Post-merge local seat. Validates a LLaMA-Factory merged export directory
//! or a GGUF file and prints the Ollama `create` next step.
//!
//! LLaMA-Factory `export_model` writes the merged directory and a `Modelfile`
//! whose `FROM` is `.` (`template.get_ollama_modelfile`). llama.cpp
//! `convert_hf_to_gguf.py` is the external GGUF step. This module does not
//! shell out to either tool and does not create a model.

use crate::error::ModelError;
use crate::train_enrich::{
    load_prepare_doc, local_enrich_tag, refuse_sacred_and_sku, EnrichJobKind, LLAMAFACTORY_LORA_ID,
    LLAMAFACTORY_QLORA_ID,
};
use feed_collector::{refuse_raw_secrets, FeedError};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

const MODELFILE_NAME: &str = "Modelfile";
const GGUF_MAGIC: &[u8] = b"GGUF";
const MODELFILE_MAX_BYTES: u64 = 64 * 1024;

/// Printed plan. `modelfile_on_disk` is true when `ollama create -f` can use
/// the file that is already in the weights directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSeatPlan {
    pub shape: String,
    pub seat_tag: String,
    pub local_tag: String,
    pub pack_id: String,
    pub driver: String,
    pub modelfile_on_disk: bool,
    pub create_command: String,
    /// Modelfile text the operator writes. Empty when the on-disk file is the one to pass.
    pub modelfile_text: Option<String>,
    pub report: String,
}

/// Operator card appended to LLaMA-Factory `PREPARE.md` and `NEXT.md`.
pub(crate) fn llamafactory_local_seat_note(
    out_dir: &Path,
    seat_tag: &str,
    pack_id: &str,
) -> String {
    let tag = local_enrich_tag(pack_id);
    let export_yaml = out_dir.join("export.yaml");
    let export_dir = out_dir.join("export");
    let modelfile = export_dir.join(MODELFILE_NAME);
    format!(
        "\n\
         ## Local seat after export\n\
         \n\
         Seat tag is {seat}. That is the Ollama id this cell already runs. The create name is {tag}.\n\
         \n\
         Chain, outside this factory. This factory does not shell out to ollama or llama.cpp, does not convert weights, and does not promote.\n\
         \n\
         1. `llamafactory-cli export` writes the merged directory named in export.yaml (`export_dir`). Current LLaMA-Factory `export_model` also writes `Modelfile` in that directory (`FROM .`, plus TEMPLATE from the train chat template). This factory does not write that Modelfile and does not invent a second template.\n\
         2. GGUF conversion stays on a llama.cpp checkout: `convert_hf_to_gguf.py` on that merged directory. This factory does not run that script and does not choose a quantization type.\n\
         3. Seat with Ollama. `ollama create` uses FROM the GGUF, or the merged directory when LLaMA-Factory wrote the Modelfile.\n\
         \n\
         Validate the directory or the GGUF and print the exact command:\n\
         \n\
         estate enrich local-seat --prepared {out} --weights {export_dir}\n\
         \n\
         When `{modelfile}` exists, that command prints:\n\
         \n\
         ollama create {tag} -f {modelfile}\n\
         \n\
         When you pass a .gguf file, it prints a Modelfile whose FROM is that file and the same `ollama create` line. It does not create the model.\n\
         \n\
         Then record the same path. `import-trained` accepts a merged export_dir (config.json and at least one .safetensors file whose name does not start with adapter_model, optional Modelfile) or a .gguf file. The seat tag on the proposal stays {seat}. import-trained records trained_shape and trained_paths. import-trained does not apply and does not promote.\n\
         \n\
         estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter {export_dir}\n\
         \n\
         export.yaml: {export_yaml}\n\
         READY_FOR_LIVE_TEST: no.\n",
        seat = seat_tag,
        tag = tag,
        out = out_dir.display(),
        export_dir = export_dir.display(),
        modelfile = modelfile.display(),
        export_yaml = export_yaml.display(),
    )
}

/// Read `prepare.json`, validate `weights`, and build the seat report.
/// Does not write and does not spawn a process.
pub fn plan_local_seat(prepared_dir: &Path, weights: &Path) -> Result<LocalSeatPlan, ModelError> {
    refuse_sacred_and_sku("prepared", &prepared_dir.display().to_string())?;
    refuse_sacred_and_sku("weights", &weights.display().to_string())?;
    let doc = load_prepare_doc(&prepared_dir.join("prepare.json"))?;
    if doc.driver != LLAMAFACTORY_LORA_ID && doc.driver != LLAMAFACTORY_QLORA_ID {
        return Err(ModelError::Other(format!(
            "refuse:driver: local-seat reads a llamafactory-lora or llamafactory-qlora prepare, found '{}'",
            doc.driver
        )));
    }
    if doc.job != EnrichJobKind::Train.as_str() {
        return Err(ModelError::Other(format!(
            "refuse:job: local-seat expects job train, found '{}'",
            doc.job
        )));
    }
    let seat_tag = doc
        .seat_tag
        .clone()
        .ok_or_else(|| ModelError::Other("refuse:seat: prepare.json has no seat_tag".into()))?;
    let local_tag = local_enrich_tag(&doc.pack_id);
    let shape = classify_weights(weights)?;
    let plan = render_plan(
        prepared_dir,
        &doc.driver,
        &doc.pack_id,
        &seat_tag,
        &local_tag,
        shape,
    )?;
    refuse_sacred_and_sku("local-seat report", &plan.report)?;
    refuse_raw_secrets(&plan.report).map_err(map_feed)?;
    Ok(plan)
}

enum WeightsShape {
    Merged {
        dir: PathBuf,
        modelfile: Option<PathBuf>,
    },
    Gguf {
        file: PathBuf,
        modelfile: Option<PathBuf>,
    },
}

struct DirMarkers {
    adapter_config: bool,
    config: bool,
    merged_safetensors: bool,
    adapter_weights: Vec<String>,
    ggufs: Vec<PathBuf>,
    modelfile: Option<PathBuf>,
    files: usize,
}

fn classify_weights(weights: &Path) -> Result<WeightsShape, ModelError> {
    let meta = match std::fs::symlink_metadata(weights) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(ModelError::Other(format!(
                "refuse:seat: {} is missing",
                weights.display()
            )));
        }
        Err(err) => {
            return Err(ModelError::Other(format!(
                "refuse:seat: {}: {err}",
                weights.display()
            )));
        }
    };
    if meta.file_type().is_symlink() {
        return Err(ModelError::Other(format!(
            "refuse:seat: {} is a symlink. local-seat does not follow a symlinked weights path. Pass the real directory or the real .gguf file.",
            weights.display()
        )));
    }
    if meta.is_file() {
        let file = classify_gguf_file(weights)?;
        return Ok(WeightsShape::Gguf {
            modelfile: sibling_modelfile(file.parent())?,
            file,
        });
    }
    if meta.is_dir() {
        return classify_dir(weights);
    }
    Err(ModelError::Other(format!(
        "refuse:seat: {} is not a file or directory",
        weights.display()
    )))
}

fn classify_gguf_file(path: &Path) -> Result<PathBuf, ModelError> {
    let name = file_name(path)?;
    if !name.to_ascii_lowercase().ends_with(".gguf") {
        return Err(ModelError::Other(format!(
            "refuse:seat: {} is not a GGUF file",
            path.display()
        )));
    }
    let mut header = [0u8; 4];
    let mut file = open_nofollow(path).map_err(|err| seat_open_error(path, err))?;
    let n = file
        .read(&mut header)
        .map_err(|err| ModelError::Other(format!("refuse:seat: {}: {err}", path.display())))?;
    if n < 4 || &header != GGUF_MAGIC {
        return Err(ModelError::Other(format!(
            "refuse:seat: {} does not start with GGUF magic",
            path.display()
        )));
    }
    Ok(path.to_path_buf())
}

fn classify_dir(dir: &Path) -> Result<WeightsShape, ModelError> {
    let markers = scan_dir(dir)?;
    if markers.files == 0 {
        return Err(ModelError::Other(format!(
            "refuse:seat: {} is an empty directory",
            dir.display()
        )));
    }
    let merged = markers.config && markers.merged_safetensors;
    let gguf_count = markers.ggufs.len();
    let mut shapes = Vec::new();
    if markers.adapter_config {
        shapes.push("adapter");
    }
    if merged {
        shapes.push("merged");
    }
    if gguf_count > 0 {
        shapes.push("gguf");
    }
    if shapes.len() > 1 {
        return Err(ModelError::Other(format!(
            "refuse:seat: {} matches more than one shape ({}). Point --weights at one merged export directory or one .gguf file.",
            dir.display(),
            shapes.join(", ")
        )));
    }
    if markers.adapter_config {
        return Err(ModelError::Other(format!(
            "refuse:seat: {} is an adapter directory. local-seat is the post-merge path. import-trained records the adapter. Seating an adapter uses a Modelfile FROM an Ollama model of the train base, plus ADAPTER.",
            dir.display()
        )));
    }
    if merged {
        return Ok(WeightsShape::Merged {
            dir: dir.to_path_buf(),
            modelfile: markers.modelfile,
        });
    }
    if gguf_count > 1 {
        return Err(ModelError::Other(format!(
            "refuse:seat: {} holds more than one .gguf file. Point --weights at one file.",
            dir.display()
        )));
    }
    if gguf_count == 1 {
        if markers.config {
            return Err(ModelError::Other(format!(
                "refuse:seat: {} has config.json and a .gguf file and is not a merged export. Point --weights at the .gguf file.",
                dir.display()
            )));
        }
        let file = markers.ggufs.into_iter().next().ok_or_else(|| {
            ModelError::Other(format!(
                "refuse:seat: {} is not a merged export directory or a GGUF",
                dir.display()
            ))
        })?;
        classify_gguf_file(&file)?;
        return Ok(WeightsShape::Gguf {
            file,
            modelfile: markers.modelfile,
        });
    }
    if markers.config && !markers.adapter_weights.is_empty() && !markers.merged_safetensors {
        let names = markers.adapter_weights.join(", ");
        return Err(ModelError::Other(format!(
            "refuse:seat: {} has config.json and adapter weights ({names}) and no merged weight. adapter_model.safetensors is not a merged export. A merged export_dir needs config.json and a .safetensors file whose name does not start with adapter_model. An adapter output_dir needs adapter_config.json. local-seat does not create a model.",
            dir.display()
        )));
    }
    if markers.config && !markers.merged_safetensors {
        return Err(ModelError::Other(format!(
            "refuse:seat: {} has config.json and no .safetensors file whose name does not start with adapter_model. A merged export_dir has both.",
            dir.display()
        )));
    }
    if markers.merged_safetensors && !markers.config {
        return Err(ModelError::Other(format!(
            "refuse:seat: {} has .safetensors and no config.json. A merged export_dir has both.",
            dir.display()
        )));
    }
    if !markers.adapter_weights.is_empty() {
        let names = markers.adapter_weights.join(", ");
        return Err(ModelError::Other(format!(
            "refuse:seat: {} has adapter weights ({names}) and no adapter_config.json. An adapter output_dir needs adapter_config.json. adapter_model.safetensors is not a merged export.",
            dir.display()
        )));
    }
    if markers.modelfile.is_some() {
        return Err(ModelError::Other(format!(
            "refuse:seat: {} has a Modelfile and no merged weights or GGUF",
            dir.display()
        )));
    }
    Err(ModelError::Other(format!(
        "refuse:seat: {} is not a merged export directory or a GGUF",
        dir.display()
    )))
}

fn scan_dir(dir: &Path) -> Result<DirMarkers, ModelError> {
    let mut markers = DirMarkers {
        adapter_config: false,
        config: false,
        merged_safetensors: false,
        adapter_weights: Vec::new(),
        ggufs: Vec::new(),
        modelfile: None,
        files: 0,
    };
    let entries = std::fs::read_dir(dir)
        .map_err(|err| ModelError::Other(format!("refuse:seat: {}: {err}", dir.display())))?;
    for entry in entries {
        let entry = entry
            .map_err(|err| ModelError::Other(format!("refuse:seat: {}: {err}", dir.display())))?;
        let kind = entry.file_type().map_err(|err| {
            ModelError::Other(format!("refuse:seat: {}: {err}", dir.display()))
        })?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(ModelError::Other(format!(
                "refuse:seat: {} has no utf-8 name",
                entry.path().display()
            )));
        };
        if kind.is_symlink() {
            if is_seat_marker_name(name) {
                return Err(ModelError::Other(format!(
                    "refuse:seat: {} is a symlink. local-seat does not follow marker symlinks. The marker must be a regular file inside {}.",
                    entry.path().display(),
                    dir.display()
                )));
            }
            continue;
        }
        if !kind.is_file() {
            continue;
        }
        markers.files += 1;
        let lower = name.to_ascii_lowercase();
        if lower == "adapter_config.json" {
            markers.adapter_config = true;
            continue;
        }
        if lower == "adapter_model.bin" || is_adapter_safetensors(&lower) {
            markers.adapter_weights.push(name.to_string());
            continue;
        }
        if lower == "config.json" {
            markers.config = true;
            continue;
        }
        if name == MODELFILE_NAME {
            markers.modelfile = Some(entry.path());
            continue;
        }
        if lower.ends_with(".safetensors") {
            markers.merged_safetensors = true;
            continue;
        }
        if lower.ends_with(".gguf") {
            markers.ggufs.push(entry.path());
        }
    }
    markers.adapter_weights.sort();
    markers.ggufs.sort();
    Ok(markers)
}

fn is_adapter_safetensors(lower_name: &str) -> bool {
    lower_name.ends_with(".safetensors") && lower_name.starts_with("adapter_model")
}

fn is_seat_marker_name(name: &str) -> bool {
    if name == MODELFILE_NAME {
        return true;
    }
    let lower = name.to_ascii_lowercase();
    lower == "adapter_config.json"
        || lower == "config.json"
        || lower == "adapter_model.bin"
        || lower.ends_with(".safetensors")
        || lower.ends_with(".gguf")
}

fn sibling_modelfile(parent: Option<&Path>) -> Result<Option<PathBuf>, ModelError> {
    let Some(parent) = parent else {
        return Ok(None);
    };
    let path = parent.join(MODELFILE_NAME);
    match std::fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(ModelError::Other(format!(
            "refuse:seat: {} is a symlink. local-seat does not follow marker symlinks. The marker must be a regular file inside {}.",
            path.display(),
            parent.display()
        ))),
        Ok(meta) if meta.is_file() => Ok(Some(path)),
        Ok(_) => Ok(None),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(ModelError::Other(format!(
            "refuse:seat: {}: {err}",
            path.display()
        ))),
    }
}

/// Open `path` without following a final symlink. Linux `O_NOFOLLOW` is
/// `0x20000`. macOS `O_NOFOLLOW` is `0x100`.
fn open_nofollow(path: &Path) -> std::io::Result<File> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        #[cfg(target_os = "linux")]
        const O_NOFOLLOW: i32 = 0x20000;
        #[cfg(target_os = "macos")]
        const O_NOFOLLOW: i32 = 0x100;
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW)
            .open(path)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "O_NOFOLLOW is unavailable",
        ))
    }
}

fn seat_open_error(path: &Path, err: std::io::Error) -> ModelError {
    if matches!(err.raw_os_error(), Some(40 | 62)) {
        ModelError::Other(format!(
            "refuse:seat: {} is a symlink. local-seat does not follow marker symlinks.",
            path.display()
        ))
    } else {
        ModelError::Other(format!("refuse:seat: {}: {err}", path.display()))
    }
}

fn render_plan(
    prepared_dir: &Path,
    driver: &str,
    pack_id: &str,
    seat_tag: &str,
    local_tag: &str,
    shape: WeightsShape,
) -> Result<LocalSeatPlan, ModelError> {
    match shape {
        WeightsShape::Merged { dir, modelfile } => render_merged(
            prepared_dir,
            driver,
            pack_id,
            seat_tag,
            local_tag,
            &dir,
            modelfile,
        ),
        WeightsShape::Gguf { file, modelfile } => render_gguf(
            prepared_dir,
            driver,
            pack_id,
            seat_tag,
            local_tag,
            &file,
            modelfile,
        ),
    }
}

fn render_merged(
    prepared_dir: &Path,
    driver: &str,
    pack_id: &str,
    seat_tag: &str,
    local_tag: &str,
    dir: &Path,
    modelfile: Option<PathBuf>,
) -> Result<LocalSeatPlan, ModelError> {
    let on_disk = modelfile.is_some();
    let modelfile_path = modelfile.unwrap_or_else(|| dir.join(MODELFILE_NAME));
    let create = ollama_create(local_tag, &modelfile_path);
    let import = import_trained_line(prepared_dir, local_tag, dir);
    let convert = convert_line(dir);
    let from_note = if on_disk {
        let text = read_modelfile(&modelfile_path)?;
        let from = first_from(&text).ok_or_else(|| {
            ModelError::Other(format!(
                "refuse:modelfile: {} has no FROM line",
                modelfile_path.display()
            ))
        })?;
        if from_arg_is_here(&from) {
            "LLaMA-Factory wrote this Modelfile with FROM . That names this merged directory."
                .to_string()
        } else {
            format!("This Modelfile FROM is {from}. The create line uses the file as written.")
        }
    } else {
        format!(
            "This merged directory has no Modelfile yet. Current LLaMA-Factory export_model writes {MODELFILE_NAME} here. Do not run ollama create until that file exists."
        )
    };
    let report = format!(
        "local-seat: shape=merged seat_tag={seat_tag} local_tag={local_tag} modelfile_on_disk={on_disk}\n\
         pack={pack_id}\n\
         driver={driver}\n\
         weights={weights}\n\
         modelfile={modelfile}\n\
         promoted=false auto_apply=false estate_rewritten=false\n\
         \n\
         {from_note} This factory does not run ollama and does not run llama.cpp.\n\
         \n\
         {create}\n\
         \n\
         GGUF conversion stays outside this factory, on a llama.cpp checkout. This factory does not choose a quantization type.\n\
         \n\
         {convert}\n\
         \n\
         Then run local-seat again with --weights pointing at that .gguf file.\n\
         \n\
         import-trained records this merged directory on the local_slm proposal. A merged export_dir is config.json and at least one .safetensors file whose name does not start with adapter_model. A Modelfile in that directory is part of that shape. The seat tag stays {seat_tag}. import-trained records trained_shape and trained_paths. import-trained does not apply and does not promote.\n\
         \n\
         {import}\n\
         \n\
         local-seat did not create a model.\n\
         READY_FOR_LIVE_TEST: no.\n",
        weights = dir.display(),
        modelfile = modelfile_path.display(),
    );
    Ok(LocalSeatPlan {
        shape: "merged".into(),
        seat_tag: seat_tag.to_string(),
        local_tag: local_tag.to_string(),
        pack_id: pack_id.to_string(),
        driver: driver.to_string(),
        modelfile_on_disk: on_disk,
        create_command: create,
        modelfile_text: None,
        report,
    })
}

fn render_gguf(
    prepared_dir: &Path,
    driver: &str,
    pack_id: &str,
    seat_tag: &str,
    local_tag: &str,
    file: &Path,
    modelfile: Option<PathBuf>,
) -> Result<LocalSeatPlan, ModelError> {
    let gguf_abs = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    let from_token = modelfile_from_token(&gguf_abs);
    let (modelfile_path, on_disk, text) = match modelfile {
        Some(path) => {
            let existing = read_modelfile(&path)?;
            let from = first_from(&existing).ok_or_else(|| {
                ModelError::Other(format!(
                    "refuse:modelfile: {} has no FROM line",
                    path.display()
                ))
            })?;
            if from_points_at(path.parent(), &from, &gguf_abs) {
                (path, true, None)
            } else {
                let body = gguf_modelfile_body(seat_tag, local_tag, &from_token, Some(&existing))?;
                (path, false, Some(body))
            }
        }
        None => {
            let parent = file.parent().unwrap_or_else(|| Path::new("."));
            let path = parent.join(MODELFILE_NAME);
            let body = gguf_modelfile_body(seat_tag, local_tag, &from_token, None)?;
            (path, false, Some(body))
        }
    };
    let create = ollama_create(local_tag, &modelfile_path);
    let import = import_trained_line(prepared_dir, local_tag, file);
    let write_note = if on_disk {
        "The Modelfile FROM already names this GGUF. The create line uses that file."
    } else {
        "Write the Modelfile below yourself, at the path in the create line. FROM is this GGUF. TEMPLATE and PARAMETER lines are copied when a LLaMA-Factory Modelfile is in the same directory. This factory does not write the file and does not invent a chat template."
    };
    let printed = match &text {
        Some(body) => format!("\n{body}\n"),
        None => String::new(),
    };
    let report = format!(
        "local-seat: shape=gguf seat_tag={seat_tag} local_tag={local_tag} modelfile_on_disk={on_disk}\n\
         pack={pack_id}\n\
         driver={driver}\n\
         weights={weights}\n\
         modelfile={modelfile}\n\
         promoted=false auto_apply=false estate_rewritten=false\n\
         \n\
         {write_note} Seat tag {seat_tag} is the Ollama id this cell already runs. The create name is {local_tag}. This factory does not run ollama and does not run llama.cpp.\n\
         {printed}\
         {create}\n\
         \n\
         import-trained records this GGUF on the local_slm proposal. A GGUF path is a .gguf file. The seat tag stays {seat_tag}. import-trained does not apply and does not promote.\n\
         \n\
         {import}\n\
         \n\
         local-seat did not create a model.\n\
         READY_FOR_LIVE_TEST: no.\n",
        weights = file.display(),
        modelfile = modelfile_path.display(),
    );
    Ok(LocalSeatPlan {
        shape: "gguf".into(),
        seat_tag: seat_tag.to_string(),
        local_tag: local_tag.to_string(),
        pack_id: pack_id.to_string(),
        driver: driver.to_string(),
        modelfile_on_disk: on_disk,
        create_command: create,
        modelfile_text: text,
        report,
    })
}

fn gguf_modelfile_body(
    seat_tag: &str,
    local_tag: &str,
    from_token: &str,
    existing: Option<&str>,
) -> Result<String, ModelError> {
    let header = format!("# seat_tag: {seat_tag}\n# local_tag: {local_tag}\n");
    let body = match existing {
        Some(existing) => rewrite_first_from(existing, from_token)?,
        None => format!("FROM {from_token}\n"),
    };
    Ok(format!("{header}{body}"))
}

fn rewrite_first_from(text: &str, from_token: &str) -> Result<String, ModelError> {
    let mut out = String::new();
    let mut replaced = false;
    for line in text.lines() {
        if !replaced && from_argument(line).is_some() {
            out.push_str(&format!("FROM {from_token}\n"));
            replaced = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !replaced {
        return Err(ModelError::Other(
            "refuse:modelfile: Modelfile has no FROM line".into(),
        ));
    }
    Ok(out)
}

fn read_modelfile(path: &Path) -> Result<String, ModelError> {
    let file = open_nofollow(path).map_err(|err| {
        if matches!(err.raw_os_error(), Some(40 | 62)) {
            ModelError::Other(format!(
                "refuse:seat: {} is a symlink. local-seat does not follow marker symlinks.",
                path.display()
            ))
        } else {
            ModelError::Other(format!("refuse:modelfile: {}: {err}", path.display()))
        }
    })?;
    let meta = file
        .metadata()
        .map_err(|err| ModelError::Other(format!("refuse:modelfile: {}: {err}", path.display())))?;
    if !meta.is_file() {
        return Err(ModelError::Other(format!(
            "refuse:modelfile: {} is not a file",
            path.display()
        )));
    }
    if meta.len() == 0 {
        return Err(ModelError::Other(format!(
            "refuse:modelfile: {} is empty",
            path.display()
        )));
    }
    if meta.len() > MODELFILE_MAX_BYTES {
        return Err(ModelError::Other(format!(
            "refuse:modelfile: {} is over {MODELFILE_MAX_BYTES} bytes",
            path.display()
        )));
    }
    let mut bytes = Vec::new();
    let mut file = file;
    file.read_to_end(&mut bytes)
        .map_err(|err| ModelError::Other(format!("refuse:modelfile: {}: {err}", path.display())))?;
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        ModelError::Other(format!("refuse:modelfile: {} is not utf-8", path.display()))
    })?;
    refuse_sacred_and_sku("modelfile", text)?;
    refuse_raw_secrets(text).map_err(map_feed)?;
    if first_from(text).is_none() {
        return Err(ModelError::Other(format!(
            "refuse:modelfile: {} has no FROM line",
            path.display()
        )));
    }
    Ok(text.to_string())
}

fn first_from(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let arg = from_argument(line)?;
        if arg.is_empty() {
            None
        } else {
            Some(arg)
        }
    })
}

fn from_argument(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let mut parts = trimmed.split_whitespace();
    let keyword = parts.next()?;
    if !keyword.eq_ignore_ascii_case("FROM") {
        return None;
    }
    let rest = trimmed[keyword.len()..].trim();
    if rest.is_empty() {
        return Some(String::new());
    }
    unquote_from_arg(rest)
}

fn unquote_from_arg(arg: &str) -> Option<String> {
    let arg = arg.trim();
    if let Some(rest) = arg.strip_prefix('"') {
        let end = rest.find('"')?;
        if rest[end + 1..].trim().is_empty() {
            return Some(rest[..end].to_string());
        }
        return None;
    }
    if let Some(rest) = arg.strip_prefix('\'') {
        let end = rest.find('\'')?;
        if rest[end + 1..].trim().is_empty() {
            return Some(rest[..end].to_string());
        }
        return None;
    }
    let mut parts = arg.split_whitespace();
    let first = parts.next()?.to_string();
    if parts.next().is_some() {
        return None;
    }
    Some(first)
}

fn from_arg_is_here(arg: &str) -> bool {
    matches!(arg, "." | "./")
}

fn from_points_at(modelfile_dir: Option<&Path>, from_arg: &str, gguf: &Path) -> bool {
    if from_arg.is_empty() || from_arg_is_here(from_arg) {
        return false;
    }
    let target = if Path::new(from_arg).is_absolute() {
        PathBuf::from(from_arg)
    } else {
        let Some(dir) = modelfile_dir else {
            return false;
        };
        dir.join(from_arg)
    };
    let Ok(left) = std::fs::canonicalize(&target) else {
        return false;
    };
    let Ok(right) = std::fs::canonicalize(gguf) else {
        return false;
    };
    left == right
}

fn ollama_create(local_tag: &str, modelfile: &Path) -> String {
    format!(
        "ollama create {} -f {}",
        shell_quote(local_tag),
        shell_quote(&modelfile.display().to_string())
    )
}

fn import_trained_line(prepared_dir: &Path, local_tag: &str, weights: &Path) -> String {
    format!(
        "estate enrich import-trained --estate <estate.yaml> --prepared {} --tag {} --adapter {}",
        shell_quote(&prepared_dir.display().to_string()),
        shell_quote(local_tag),
        shell_quote(&weights.display().to_string())
    )
}

fn convert_line(export_dir: &Path) -> String {
    let outfile = export_dir.join("model.gguf");
    format!(
        "python convert_hf_to_gguf.py {} --outfile {}",
        shell_quote(&export_dir.display().to_string()),
        shell_quote(&outfile.display().to_string())
    )
}

fn modelfile_from_token(path: &Path) -> String {
    let text = path.display().to_string();
    if text.chars().any(|c| c.is_whitespace()) {
        format!("\"{}\"", text.replace('"', "\\\""))
    } else {
        text
    }
}

fn shell_quote(text: &str) -> String {
    if text
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '\\' | '$' | '`'))
    {
        format!("'{}'", text.replace('\'', "'\\''"))
    } else {
        text.to_string()
    }
}

fn file_name(path: &Path) -> Result<String, ModelError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .ok_or_else(|| {
            ModelError::Other(format!("refuse:seat: {} has no utf-8 name", path.display()))
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
    use crate::train_enrich::PREPARE_SCHEMA;

    fn tmp(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cell-one-local-seat-{}-{}",
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

    fn merged(dir: &Path, modelfile: Option<&str>) {
        std::fs::write(dir.join("config.json"), "{}\n").unwrap();
        std::fs::write(dir.join("model.safetensors"), b"not-a-real-tensor").unwrap();
        if let Some(text) = modelfile {
            std::fs::write(dir.join(MODELFILE_NAME), text).unwrap();
        }
    }

    fn gguf_bytes() -> Vec<u8> {
        let mut bytes = b"GGUF".to_vec();
        bytes.extend_from_slice(&[0u8; 12]);
        bytes
    }

    fn factory_modelfile() -> &'static str {
        "# ollama modelfile auto-generated by llamafactory\n\nFROM .\n\nTEMPLATE \"\"\"hello-template\"\"\"\n\nPARAMETER num_ctx 4096\n"
    }

    #[test]
    fn note_names_the_chain_and_the_seat_tag() {
        let note = llamafactory_local_seat_note(
            Path::new("/tmp/cell-one-pack"),
            "llama3",
            "overnight-traces",
        );
        assert!(note.contains("Seat tag is llama3"), "{note}");
        assert!(note.contains("cell-enrich-overnight-traces"), "{note}");
        assert!(note.contains("estate enrich local-seat --prepared /tmp/cell-one-pack --weights /tmp/cell-one-pack/export"), "{note}");
        assert!(note.contains("convert_hf_to_gguf.py"), "{note}");
        assert!(
            note.contains(
                "ollama create cell-enrich-overnight-traces -f /tmp/cell-one-pack/export/Modelfile"
            ),
            "{note}"
        );
        assert!(note.contains("import-trained"), "{note}");
        assert!(
            note.contains(
                "config.json and at least one .safetensors file whose name does not start with adapter_model"
            ),
            "{note}"
        );
        assert!(note.contains("trained_shape"), "{note}");
        assert!(note.contains("READY_FOR_LIVE_TEST: no"), "{note}");
        assert!(!note.contains("READY_FOR_LIVE_TEST: yes"), "{note}");
        assert!(!estate_schema::contains_sku(&note), "{note}");
    }

    #[test]
    fn merged_modelfile_prints_create_and_leaves_the_file() {
        let root = tmp("merged-ok");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        std::fs::create_dir_all(&export).unwrap();
        let body = factory_modelfile();
        merged(&export, Some(body));
        let before = std::fs::read(export.join(MODELFILE_NAME)).unwrap();
        let plan = plan_local_seat(&root, &export).unwrap();
        assert_eq!(plan.shape, "merged");
        assert_eq!(plan.seat_tag, "llama3");
        assert_eq!(plan.local_tag, "cell-enrich-overnight-traces");
        assert!(plan.modelfile_on_disk);
        assert!(plan.modelfile_text.is_none());
        let modelfile = export.join(MODELFILE_NAME);
        let create = format!(
            "ollama create cell-enrich-overnight-traces -f {}",
            modelfile.display()
        );
        assert_eq!(plan.create_command, create);
        assert!(plan.report.contains(&create), "{}", plan.report);
        assert!(plan.report.contains("seat_tag=llama3"), "{}", plan.report);
        assert!(plan.report.contains("FROM ."), "{}", plan.report);
        assert!(
            plan.report.contains("convert_hf_to_gguf.py"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("import-trained"), "{}", plan.report);
        assert!(plan.report.contains("--adapter"), "{}", plan.report);
        assert!(
            plan.report.contains("local-seat did not create a model."),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("READY_FOR_LIVE_TEST: no"),
            "{}",
            plan.report
        );
        assert!(plan.report.contains("promoted=false"), "{}", plan.report);
        assert_eq!(std::fs::read(export.join(MODELFILE_NAME)).unwrap(), before);
        assert!(!export.join("model.gguf").exists());
    }

    #[test]
    fn merged_without_modelfile_names_the_missing_file() {
        let root = tmp("merged-bare");
        write_prepare(&root, LLAMAFACTORY_LORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        std::fs::create_dir_all(&export).unwrap();
        merged(&export, None);
        let plan = plan_local_seat(&root, &export).unwrap();
        assert_eq!(plan.shape, "merged");
        assert_eq!(plan.driver, LLAMAFACTORY_LORA_ID);
        assert!(!plan.modelfile_on_disk);
        assert!(
            plan.report.contains("modelfile_on_disk=false"),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("has no Modelfile yet"),
            "{}",
            plan.report
        );
        assert!(
            plan.create_command.contains("export/Modelfile"),
            "{}",
            plan.create_command
        );
        assert!(!export.join(MODELFILE_NAME).exists());
    }

    #[test]
    fn gguf_copies_template_and_sets_from() {
        let root = tmp("gguf-template");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let export = root.join("export");
        std::fs::create_dir_all(&export).unwrap();
        let gguf = export.join("model.gguf");
        std::fs::write(&gguf, gguf_bytes()).unwrap();
        let original = factory_modelfile();
        std::fs::write(export.join(MODELFILE_NAME), original).unwrap();
        let plan = plan_local_seat(&root, &gguf).unwrap();
        assert_eq!(plan.shape, "gguf");
        assert!(!plan.modelfile_on_disk);
        let text = plan.modelfile_text.unwrap();
        let abs = std::fs::canonicalize(&gguf).unwrap();
        assert!(text.contains(&format!("FROM {}", abs.display())), "{text}");
        assert!(
            text.contains("TEMPLATE \"\"\"hello-template\"\"\""),
            "{text}"
        );
        assert!(text.contains("PARAMETER num_ctx 4096"), "{text}");
        assert!(text.contains("# seat_tag: llama3"), "{text}");
        assert!(!text.lines().any(|line| line.trim() == "FROM ."), "{text}");
        assert!(
            plan.report.contains(&plan.create_command),
            "{}",
            plan.report
        );
        assert!(
            plan.report.contains("does not invent a chat template"),
            "{}",
            plan.report
        );
        assert_eq!(
            std::fs::read_to_string(export.join(MODELFILE_NAME)).unwrap(),
            original
        );
    }

    #[test]
    fn gguf_without_modelfile_prints_from_only() {
        let root = tmp("gguf-bare");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let gguf = root.join("model.gguf");
        std::fs::write(&gguf, gguf_bytes()).unwrap();
        let plan = plan_local_seat(&root, &gguf).unwrap();
        let text = plan.modelfile_text.unwrap();
        let abs = std::fs::canonicalize(&gguf).unwrap();
        assert!(text.contains(&format!("FROM {}", abs.display())), "{text}");
        assert!(!text.contains("TEMPLATE"), "{text}");
        assert!(
            plan.report.contains("does not invent a chat template"),
            "{}",
            plan.report
        );
        assert!(!root.join(MODELFILE_NAME).exists());
    }

    #[test]
    fn gguf_directory_and_matching_from_use_the_file_on_disk() {
        let root = tmp("gguf-dir");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let dir = root.join("out");
        std::fs::create_dir_all(&dir).unwrap();
        let gguf = dir.join("model.gguf");
        std::fs::write(&gguf, gguf_bytes()).unwrap();
        let abs = std::fs::canonicalize(&gguf).unwrap();
        std::fs::write(
            dir.join(MODELFILE_NAME),
            format!("FROM {}\nPARAMETER num_ctx 4096\n", abs.display()),
        )
        .unwrap();
        let plan = plan_local_seat(&root, &dir).unwrap();
        assert_eq!(plan.shape, "gguf");
        assert!(plan.modelfile_on_disk, "{}", plan.report);
        assert!(plan.modelfile_text.is_none());
        assert!(
            plan.create_command.contains("Modelfile"),
            "{}",
            plan.create_command
        );
        assert!(
            plan.report.contains("FROM already names this GGUF"),
            "{}",
            plan.report
        );
    }

    #[test]
    fn refuses_bad_weights() {
        let root = tmp("refuse-weights");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let missing = root.join("nope");
        let err = plan_local_seat(&root, &missing).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");
        assert!(err.to_string().contains("missing"), "{err}");

        let empty = root.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        let err = plan_local_seat(&root, &empty).unwrap_err();
        assert!(err.to_string().contains("empty directory"), "{err}");

        let half = root.join("half");
        std::fs::create_dir_all(&half).unwrap();
        std::fs::write(half.join("config.json"), "{}\n").unwrap();
        let err = plan_local_seat(&root, &half).unwrap_err();
        assert!(err.to_string().contains("no .safetensors"), "{err}");

        let tensors = root.join("tensors");
        std::fs::create_dir_all(&tensors).unwrap();
        std::fs::write(tensors.join("model.safetensors"), b"x").unwrap();
        let err = plan_local_seat(&root, &tensors).unwrap_err();
        assert!(err.to_string().contains("no config.json"), "{err}");

        let adapter = root.join("adapter");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        let err = plan_local_seat(&root, &adapter).unwrap_err();
        assert!(err.to_string().contains("adapter directory"), "{err}");
        assert!(err.to_string().contains("import-trained"), "{err}");

        let mixed = root.join("mixed");
        std::fs::create_dir_all(&mixed).unwrap();
        merged(&mixed, None);
        std::fs::write(mixed.join("adapter_config.json"), "{}\n").unwrap();
        let err = plan_local_seat(&root, &mixed).unwrap_err();
        assert!(err.to_string().contains("more than one shape"), "{err}");

        let both = root.join("both");
        std::fs::create_dir_all(&both).unwrap();
        merged(&both, None);
        std::fs::write(both.join("model.gguf"), gguf_bytes()).unwrap();
        let err = plan_local_seat(&root, &both).unwrap_err();
        assert!(err.to_string().contains("more than one shape"), "{err}");

        let many = root.join("many");
        std::fs::create_dir_all(&many).unwrap();
        std::fs::write(many.join("a.gguf"), gguf_bytes()).unwrap();
        std::fs::write(many.join("b.gguf"), gguf_bytes()).unwrap();
        let err = plan_local_seat(&root, &many).unwrap_err();
        assert!(err.to_string().contains("more than one .gguf"), "{err}");

        let note = root.join("notes.txt");
        std::fs::write(&note, b"hello").unwrap();
        let err = plan_local_seat(&root, &note).unwrap_err();
        assert!(err.to_string().contains("not a GGUF"), "{err}");

        let bad = root.join("bad.gguf");
        std::fs::write(&bad, b"NOPE").unwrap();
        let err = plan_local_seat(&root, &bad).unwrap_err();
        assert!(err.to_string().contains("GGUF magic"), "{err}");

        std::fs::create_dir_all(root.join("mf")).unwrap();
        std::fs::write(root.join("mf").join(MODELFILE_NAME), "FROM .\n").unwrap();
        let err = plan_local_seat(&root, &root.join("mf")).unwrap_err();
        assert!(err.to_string().contains("no merged weights"), "{err}");
    }

    #[test]
    fn sharded_adapter_weights_are_not_a_merged_export() {
        let root = tmp("adapter-shard");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let shard = root.join("shard");
        std::fs::create_dir_all(&shard).unwrap();
        std::fs::write(shard.join("config.json"), "{}\n").unwrap();
        std::fs::write(
            shard.join("adapter_model-00001-of-00002.safetensors"),
            b"w",
        )
        .unwrap();
        let err = plan_local_seat(&root, &shard).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(
            text.contains("adapter_model.safetensors is not a merged export"),
            "{text}"
        );
        assert!(
            text.contains("does not start with adapter_model"),
            "{text}"
        );
        assert!(!text.contains("ollama create"), "{text}");

        let folded = root.join("folded");
        std::fs::create_dir_all(&folded).unwrap();
        std::fs::write(folded.join("config.json"), "{}\n").unwrap();
        std::fs::write(
            folded.join("Adapter_Model-00001-of-00002.safetensors"),
            b"w",
        )
        .unwrap();
        let err = plan_local_seat(&root, &folded).unwrap_err();
        let text = err.to_string();
        assert!(
            text.contains("adapter_model.safetensors is not a merged export"),
            "{text}"
        );
        assert!(!text.contains("ollama create"), "{text}");

        let alongside = root.join("alongside");
        std::fs::create_dir_all(&alongside).unwrap();
        merged(&alongside, Some("FROM .\n"));
        std::fs::write(
            alongside.join("adapter_model-00001-of-00002.safetensors"),
            b"w",
        )
        .unwrap();
        let plan = plan_local_seat(&root, &alongside).unwrap();
        assert_eq!(plan.shape, "merged");
        assert!(plan.report.contains("ollama create"), "{}", plan.report);
        assert!(
            plan.report.contains("READY_FOR_LIVE_TEST: no"),
            "{}",
            plan.report
        );
    }

    #[test]
    fn refuses_symlinked_weights_and_markers() {
        let root = tmp("symlinks");
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let real = root.join("real");
        std::fs::create_dir_all(&real).unwrap();
        merged(&real, Some("FROM .\n"));
        let linked_dir = root.join("linked-dir");
        std::os::unix::fs::symlink(&real, &linked_dir).unwrap();
        let err = plan_local_seat(&root, &linked_dir).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:seat"), "{text}");
        assert!(text.contains("is a symlink"), "{text}");
        assert!(!text.contains("ollama create"), "{text}");

        let gguf = root.join("model.gguf");
        std::fs::write(&gguf, gguf_bytes()).unwrap();
        let linked_gguf = root.join("linked.gguf");
        std::os::unix::fs::symlink(&gguf, &linked_gguf).unwrap();
        let err = plan_local_seat(&root, &linked_gguf).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("is a symlink"), "{text}");
        assert!(!text.contains("ollama create"), "{text}");

        let marked = root.join("marked");
        std::fs::create_dir_all(&marked).unwrap();
        std::fs::write(marked.join("config.json"), "{}\n").unwrap();
        let outside = root.join("outside.safetensors");
        std::fs::write(&outside, b"escaped").unwrap();
        std::os::unix::fs::symlink(&outside, marked.join("model.safetensors")).unwrap();
        let err = plan_local_seat(&root, &marked).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("is a symlink"), "{text}");
        assert!(text.contains("marker"), "{text}");
        assert!(!text.contains("ollama create"), "{text}");

        let modelfile_link = root.join("modelfile-link");
        std::fs::create_dir_all(&modelfile_link).unwrap();
        merged(&modelfile_link, None);
        let outside_mf = root.join("outside-Modelfile");
        std::fs::write(&outside_mf, "FROM .\n").unwrap();
        std::os::unix::fs::symlink(&outside_mf, modelfile_link.join(MODELFILE_NAME)).unwrap();
        let err = plan_local_seat(&root, &modelfile_link).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("is a symlink"), "{text}");
        assert!(!text.contains("ollama create"), "{text}");

        let gguf_dir = root.join("gguf-link-dir");
        std::fs::create_dir_all(&gguf_dir).unwrap();
        std::os::unix::fs::symlink(&gguf, gguf_dir.join("model.gguf")).unwrap();
        let err = plan_local_seat(&root, &gguf_dir).unwrap_err();
        assert!(err.to_string().contains("is a symlink"), "{err}");
    }

    #[test]
    fn refuses_prepare_and_modelfile_gates() {
        let root = tmp("refuse-prepare");
        let export = root.join("export");
        std::fs::create_dir_all(&export).unwrap();
        merged(&export, Some("TEMPLATE \"\"\"x\"\"\"\n"));
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let err = plan_local_seat(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:modelfile"), "{err}");
        assert!(err.to_string().contains("no FROM"), "{err}");

        merged(&export, Some("FROM .\n"));
        write_prepare(&root, "axolotl-lora", "train", Some("llama3"), false);
        let err = plan_local_seat(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:driver"), "{err}");

        write_prepare(&root, "ollama-modelfile", "enrich", Some("llama3"), false);
        let err = plan_local_seat(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:driver"), "{err}");

        write_prepare(
            &root,
            LLAMAFACTORY_QLORA_ID,
            "enrich",
            Some("llama3"),
            false,
        );
        let err = plan_local_seat(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");

        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", None, false);
        let err = plan_local_seat(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:seat"), "{err}");
        assert!(err.to_string().contains("seat_tag"), "{err}");

        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("qwen"), false);
        let err = plan_local_seat(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:prepare"), "{err}");
        assert!(err.to_string().contains("seat_tag"), "{err}");

        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), true);
        let err = plan_local_seat(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:prepared"), "{err}");

        let sacred = root.join("cyera-export");
        std::fs::create_dir_all(&sacred).unwrap();
        write_prepare(&root, LLAMAFACTORY_QLORA_ID, "train", Some("llama3"), false);
        let err = plan_local_seat(&root, &sacred).unwrap_err();
        assert!(err.to_string().contains("refuse:sacred"), "{err}");

        let sku = root.join("model-5090.gguf");
        std::fs::write(&sku, gguf_bytes()).unwrap();
        let err = plan_local_seat(&root, &sku).unwrap_err();
        assert!(err.to_string().contains("refuse:sku-banned"), "{err}");

        merged(&export, Some("FROM .\ntoken=sk-abcdefghijklmnopqrstuv\n"));
        let err = plan_local_seat(&root, &export).unwrap_err();
        assert!(err.to_string().contains("refuse:raw-secret"), "{err}");
    }
}
