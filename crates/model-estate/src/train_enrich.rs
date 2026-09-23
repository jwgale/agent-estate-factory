//! Train/enrich facilitation. Prepare artifacts for a purpose-built SLM.
//!
//! The local runtime stays a seat. Ollama is today's entrant. This module
//! does not shell out, does not POST a train job, and does not rewrite the
//! estate. `llamafactory-qlora` writes the LLaMA-Factory QLoRA recipe the
//! operator runs outside the factory. `axolotl-lora` writes the Axolotl YAML
//! recipe. Both recipe cards write the train base, and keep the Ollama seat
//! tag for Modelfile `FROM`. Unsloth stays a NEXT.md pointer, not a card.
//! `external-manifest` stays the vendor-neutral hatch.
//! Floor and estate-control dispatch do not match driver ids.

use crate::error::ModelError;
use estate_schema::{contains_sku, is_sacred_name, Estate};
use feed_collector::{
    classify_path, refuse_curator, refuse_frontier_source_on_estate, refuse_pack,
    refuse_raw_secrets, FeedError, PackManifest, ScrubbedEvent,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const PREPARE_SCHEMA: &str = "cell-one.enrich-prepare.v0";

pub const BINDING_PROPOSAL_SCHEMA: &str = "cell-one.enrich-binding-proposal.v0";

const BINDING_PROPOSAL_JSON: &str = "binding-proposal.json";
const BINDING_PROPOSAL_MD: &str = "binding-proposal.md";

/// Local tag the seated runtime uses for a prepared pack. Ollama `create` and
/// `import-prepared` share this string. The factory does not create the model.
pub fn local_enrich_tag(pack_id: &str) -> String {
    format!("cell-enrich-{pack_id}")
}

/// First catalog card. Control uses this when the operator omits `--driver`.
pub fn default_train_enrich_driver_id() -> &'static str {
    REGISTRY[0].card.driver_id
}

const PREPARE_NOTE: &str = "Prepared artifacts only. Does not train, does not POST, does not rewrite the estate, does not auto-promote.";

const SACRED_NEEDLES: &[&str] = &["cyera", "rust-classroom", "rust_classroom"];

/// Catalog card. Another entrant is another row in `REGISTRY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainEnrichCard {
    pub driver_id: &'static str,
    pub status: &'static str,
    pub integrates: &'static str,
    pub notes: &'static str,
    /// Jobs this card writes. `--all-drivers` includes the card when the requested job is here.
    pub jobs: &'static [EnrichJobKind],
    /// Job used when the operator names this driver and omits `--job`.
    pub default_job: EnrichJobKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainEnrichProbe {
    pub driver_id: String,
    pub status: String,
    pub live: bool,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnrichJobKind {
    Enrich,
    Train,
}

impl EnrichJobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EnrichJobKind::Enrich => "enrich",
            EnrichJobKind::Train => "train",
        }
    }
}

/// Validated job handed to a driver. Drivers return bytes. They do not write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrichJob {
    pub kind: EnrichJobKind,
    pub pack_id: String,
    /// Ollama seat tag. Modelfile `FROM` uses this. It is not a Hugging Face id.
    pub base_model: String,
    /// Hugging Face repo id or a local HF weights directory. `llamafactory-qlora`
    /// writes this to `model_name_or_path`. `axolotl-lora` writes it to `base_model`.
    /// Absent until the pack or the binding sets it.
    pub train_base_model: Option<String>,
    pub purpose: String,
    pub system_text: String,
    pub host_class_affinity: String,
    pub source_paths: Vec<String>,
    pub source_drivers: Vec<String>,
    /// Directory the artifacts will occupy. Path-bearing recipes use it for absolute paths.
    pub out_dir: PathBuf,
    /// Explicit gauge cap for the LLaMA-Factory recipe. `None` leaves `max_steps` unset.
    pub max_steps: Option<u32>,
    /// Train recipe cards set this. Other cards leave it unset.
    pub dataset: Option<DatasetMaterial>,
}

pub struct DriverPrepare {
    pub files: Vec<(String, String)>,
    pub steps: String,
}

pub trait TrainEnrichDriver: Send + Sync {
    fn id(&self) -> &'static str;
    fn status(&self) -> &'static str;
    fn probe(&self) -> TrainEnrichProbe {
        TrainEnrichProbe {
            driver_id: self.id().to_string(),
            status: self.status().to_string(),
            live: false,
            note: "catalog probe; does not train, shell out, or POST".into(),
        }
    }
    fn prepare(&self, job: &EnrichJob) -> Result<DriverPrepare, ModelError>;
}

struct RegisteredDriver {
    card: TrainEnrichCard,
    build: fn() -> Box<dyn TrainEnrichDriver>,
}

const ENRICH_AND_TRAIN: &[EnrichJobKind] = &[EnrichJobKind::Enrich, EnrichJobKind::Train];
const TRAIN_ONLY: &[EnrichJobKind] = &[EnrichJobKind::Train];

const REGISTRY: &[RegisteredDriver] = &[
    RegisteredDriver {
        card: TrainEnrichCard {
            driver_id: "ollama-modelfile",
            status: "integration",
            integrates: "ollama create / Modelfile FROM+SYSTEM",
            notes: "Joins the seated Ollama runtime. Writes a Modelfile. Does not shell out. Does not train.",
            jobs: ENRICH_AND_TRAIN,
            default_job: EnrichJobKind::Enrich,
        },
        build: || Box::new(OllamaModelfileDriver),
    },
    RegisteredDriver {
        card: TrainEnrichCard {
            driver_id: "external-manifest",
            status: "portable",
            integrates: "json+yaml manifest",
            notes: "Market-shift hatch. A future trainer reads the manifest. No vendor lock.",
            jobs: ENRICH_AND_TRAIN,
            default_job: EnrichJobKind::Enrich,
        },
        build: || Box::new(ExternalManifestDriver),
    },
    RegisteredDriver {
        card: TrainEnrichCard {
            driver_id: LLAMAFACTORY_QLORA_ID,
            status: "integration",
            integrates: "llamafactory-cli train QLoRA recipe",
            notes: "Primary train card. Writes recipe.yaml (LLaMA-Factory SFT QLoRA). model_name_or_path is the train base (HF repo or local HF weights), separate from the Ollama seat tag. Default job is train. Does not shell out. Train hosts are consumer-nvidia and rented-nvidia.",
            jobs: TRAIN_ONLY,
            default_job: EnrichJobKind::Train,
        },
        build: || Box::new(LlamaFactoryQloraDriver),
    },
    RegisteredDriver {
        card: TrainEnrichCard {
            driver_id: AXOLOTL_LORA_ID,
            status: "integration",
            integrates: "axolotl train LoRA/QLoRA recipe",
            notes: "YAML recipe for a config-driven or multi-GPU run. Writes axolotl.yml and dataset.jsonl. base_model is the train base (HF repo or local HF weights), separate from the Ollama seat tag. Default job is train. Does not shell out. The durable recipe card is llamafactory-qlora.",
            jobs: TRAIN_ONLY,
            default_job: EnrichJobKind::Train,
        },
        build: || Box::new(AxolotlLoraDriver),
    },
];

struct OllamaModelfileDriver;

struct ExternalManifestDriver;

struct LlamaFactoryQloraDriver;

struct AxolotlLoraDriver;

/// Primary train card. LLaMA-Factory already trains QLoRA from a YAML recipe. This id writes that recipe.
pub const LLAMAFACTORY_QLORA_ID: &str = "llamafactory-qlora";

/// YAML train card. Axolotl already trains from a config. This id writes that recipe.
pub const AXOLOTL_LORA_ID: &str = "axolotl-lora";

const TRAIN_RECIPE_DRIVERS: &[&str] = &[LLAMAFACTORY_QLORA_ID, AXOLOTL_LORA_ID];

fn is_train_recipe_driver(id: &str) -> bool {
    TRAIN_RECIPE_DRIVERS.contains(&id)
}

/// Smoke-scale cutoff. LLaMA-Factory SFT examples use 2048 for a longer run.
const LLAMAFACTORY_CUTOFF_LEN: u32 = 512;
const LLAMAFACTORY_LORA_RANK: u32 = 16;
const LLAMAFACTORY_LORA_ALPHA: u32 = 32;
const LLAMAFACTORY_SEED: u32 = 42;
const LLAMAFACTORY_DATASET_NAME: &str = "cell_enrich";

impl TrainEnrichDriver for OllamaModelfileDriver {
    fn id(&self) -> &'static str {
        "ollama-modelfile"
    }

    fn status(&self) -> &'static str {
        "integration"
    }

    fn prepare(&self, job: &EnrichJob) -> Result<DriverPrepare, ModelError> {
        if job.system_text.contains("\"\"\"") {
            return Err(ModelError::Other(
                "refuse:modelfile: system text contains a Modelfile quote fence".into(),
            ));
        }
        let name = local_enrich_tag(&job.pack_id);
        let body = format!(
            "# schema: {schema}\n# driver: ollama-modelfile\n# job: {kind}\n# prepare writes this file. It does not run ollama.\n# FROM is the seated model name. The binding id is not a model tag.\nFROM {base}\nSYSTEM \"\"\"{system}\"\"\"\n",
            schema = PREPARE_SCHEMA,
            kind = job.kind.as_str(),
            base = job.base_model,
            system = job.system_text,
        );
        let steps = format!(
            "This step wrote a Modelfile (FROM + SYSTEM). It did not run ollama, did not train, and did not rewrite the estate.\n\
             \n\
             On the seated local runtime (Ollama today). FROM is {base}. That model must already be present. This step did not pull weights.\n\
             \n\
             ollama create {name} -f Modelfile\n\
             \n\
             Dataset path hints from the pack (not downloaded):\n\
             {paths}\n",
            base = job.base_model,
            paths = dataset_lines(&job.source_paths),
        );
        Ok(DriverPrepare {
            files: vec![("Modelfile".into(), body)],
            steps,
        })
    }
}

impl TrainEnrichDriver for ExternalManifestDriver {
    fn id(&self) -> &'static str {
        "external-manifest"
    }

    fn status(&self) -> &'static str {
        "portable"
    }

    fn prepare(&self, job: &EnrichJob) -> Result<DriverPrepare, ModelError> {
        let manifest = ExternalManifest {
            schema: PREPARE_SCHEMA.into(),
            driver: "external-manifest",
            job: job.kind.as_str().to_string(),
            pack_id: job.pack_id.clone(),
            base_model: job.base_model.clone(),
            purpose: job.purpose.clone(),
            host_class_affinity: job.host_class_affinity.clone(),
            dataset_paths: job.source_paths.clone(),
            source_drivers: job.source_drivers.clone(),
            vendor: None,
            promoted: false,
            auto_apply: false,
            estate_rewritten: false,
            note: "Portable manifest for a future trainer. No vendor lock. Cell One does not run the trainer.",
        };
        let json = to_pretty(&manifest)?;
        let yaml = external_manifest_yaml(&manifest);
        let steps = format!(
            "This step wrote manifest.json and manifest.yaml. Any future trainer can read them.\n\
             There is no vendor lock. Cell One does not POST a train job and does not shell out.\n\
             \n\
             Fields: base_model, purpose, host_class_affinity, dataset_paths.\n\
             \n\
             Dataset path hints from the pack (not downloaded):\n\
             {paths}\n\
             \n\
             A later entrant implements TrainEnrichDriver and adds one catalog card.\n\
             Floor and estate-control dispatch stay as they are.\n",
            paths = dataset_lines(&job.source_paths),
        );
        Ok(DriverPrepare {
            files: vec![
                ("manifest.json".into(), json),
                ("manifest.yaml".into(), yaml),
            ],
            steps,
        })
    }
}

impl TrainEnrichDriver for LlamaFactoryQloraDriver {
    fn id(&self) -> &'static str {
        LLAMAFACTORY_QLORA_ID
    }

    fn status(&self) -> &'static str {
        "integration"
    }

    fn prepare(&self, job: &EnrichJob) -> Result<DriverPrepare, ModelError> {
        if job.kind != EnrichJobKind::Train {
            return Err(ModelError::Other(format!(
                "refuse:job: {LLAMAFACTORY_QLORA_ID} prepares train; got {}",
                job.kind.as_str()
            )));
        }
        let train_owned = require_train_base(job)?;
        let train_base = train_owned.as_str();
        let data = require_dataset(job)?;
        let dataset = data.chat_jsonl.clone();
        let recipe = llamafactory_recipe_yaml(job, &data.mode, train_base);
        let export = llamafactory_export_yaml(job, train_base);
        let info = llamafactory_dataset_info();
        let host = llamafactory_host_note(&job.host_class_affinity);
        let data_note = dataset_card_note(CHAT_DATASET_SHAPE, data);
        let template = llamafactory_template(train_base);
        let gauge = llamafactory_gauge_note(job.max_steps);
        let steps = format!(
            "This step wrote recipe.yaml, export.yaml, dataset_info.json, and dataset.jsonl. The recipe is SFT QLoRA (`stage: sft`, `finetuning_type: lora`, `quantization_bit: 4`, `quantization_method: bnb`, LoRA rank {rank}, `cutoff_len` {cutoff}, `packing: true`). It did not run llamafactory-cli, did not train, did not download weights, and did not call CUDA.\n\
             \n\
             {host}\n\
             \n\
             This card expects CUDA LLaMA-Factory.\n\
             \n\
             {data_note}\n\
             \n\
             Seat tag is {seat}. That is the Ollama id for Modelfile FROM. It comes from params.model on the local binding, or from a pack model_hint that is already a model tag.\n\
             \n\
             Train base is {train}. recipe.yaml and export.yaml set model_name_or_path to that value. A train base is a Hugging Face repo id (namespace/name) or a local directory of HF weights. This factory did not download weights and does not map the seat tag onto a Hub repo.\n\
             \n\
             template is {template}. That hint comes from the train base name. Confirm it matches the model. Use that same chat template when you seat the model.\n\
             \n\
             {bits}\n\
             \n\
             {gauge}\n\
             \n\
             A later preference stage is a recipe flag (`stage: dpo` or `stage: orpo`, with `ranking: true` in dataset_info.json). This card does not build that dataset.\n\
             \n\
             From this directory, after LLaMA-Factory is installed on a CUDA host:\n\
             \n\
             {install}\n\
             llamafactory-cli train recipe.yaml\n\
             llamafactory-cli export export.yaml\n\
             \n\
             The copy-paste lines with absolute paths are in NEXT.md. Ollama stays the local-run seat after the adapter or the merged weights exist. This factory does not export GGUF.\n",
            rank = LLAMAFACTORY_LORA_RANK,
            cutoff = LLAMAFACTORY_CUTOFF_LEN,
            seat = job.base_model,
            train = train_base,
            template = template,
            bits = llamafactory_bitsandbytes_note(),
            install = llamafactory_install_lines(),
        );
        Ok(DriverPrepare {
            files: vec![
                ("recipe.yaml".into(), recipe),
                ("export.yaml".into(), export),
                ("dataset_info.json".into(), info),
                ("dataset.jsonl".into(), dataset),
            ],
            steps,
        })
    }
}

impl TrainEnrichDriver for AxolotlLoraDriver {
    fn id(&self) -> &'static str {
        AXOLOTL_LORA_ID
    }

    fn status(&self) -> &'static str {
        "integration"
    }

    fn prepare(&self, job: &EnrichJob) -> Result<DriverPrepare, ModelError> {
        if job.kind != EnrichJobKind::Train {
            return Err(ModelError::Other(format!(
                "refuse:job: {AXOLOTL_LORA_ID} prepares train; got {}",
                job.kind.as_str()
            )));
        }
        let train_owned = require_train_base(job)?;
        let train_base = train_owned.as_str();
        let data = require_dataset(job)?;
        let dataset = data.alpaca_jsonl.clone();
        let yaml = axolotl_recipe_yaml(job, &data.mode, train_base);
        let host = axolotl_host_note(&job.host_class_affinity);
        let data_note = dataset_card_note(ALPACA_DATASET_SHAPE, data);
        let steps = format!(
            "This step wrote axolotl.yml and dataset.jsonl. The recipe is QLoRA (`load_in_4bit: true`, `adapter: qlora`), which is Axolotl's LoRA/QLoRA class. It did not run axolotl, did not train, did not download weights, and did not rewrite the estate.\n\
             \n\
             {host}\n\
             \n\
             {data_note}\n\
             \n\
             Seat tag is {seat}. That is the Ollama id for Modelfile FROM. It comes from params.model on the local binding, or from a pack model_hint that is already a model tag.\n\
             \n\
             Train base is {train}. axolotl.yml sets base_model to that value. A train base is a Hugging Face repo id (namespace/name) or a local directory of HF weights. This factory did not download weights and does not map the seat tag onto a Hub repo.\n\
             \n\
             To train full LoRA on that host, set `load_in_8bit: true`, `load_in_4bit: false`, and `adapter: lora` in axolotl.yml before you run it.\n\
             \n\
             From this directory:\n\
             \n\
             axolotl train axolotl.yml\n\
             \n\
             The copy-paste line with the config path is in NEXT.md. Ollama stays the local-run seat after the adapter or the merged weights exist.\n",
            seat = job.base_model,
            train = train_base,
        );
        Ok(DriverPrepare {
            files: vec![
                ("axolotl.yml".into(), yaml),
                ("dataset.jsonl".into(), dataset),
            ],
            steps,
        })
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ExternalManifest {
    schema: String,
    driver: &'static str,
    job: String,
    pack_id: String,
    base_model: String,
    purpose: String,
    host_class_affinity: String,
    dataset_paths: Vec<String>,
    source_drivers: Vec<String>,
    vendor: Option<String>,
    promoted: bool,
    auto_apply: bool,
    estate_rewritten: bool,
    note: &'static str,
}

/// Envelope written as `prepare.json` for every driver.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EnrichPrepareDoc {
    pub schema: String,
    pub driver: String,
    pub job: String,
    pub pack_id: String,
    /// Ollama seat tag. Same value Modelfile `FROM` uses.
    pub base_model: String,
    /// Same string as `base_model`. Present so a reader can see the seat without guessing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat_tag: Option<String>,
    /// Hugging Face repo id or local HF weights directory for the train recipe cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub train_base_model: Option<String>,
    pub purpose: String,
    pub host_class_affinity: String,
    pub source_paths: Vec<String>,
    pub source_drivers: Vec<String>,
    pub artifacts: Vec<String>,
    pub promoted: bool,
    pub auto_apply: bool,
    pub estate_rewritten: bool,
    pub note: String,
    /// `stub`, `scaffold`, or `feed`. Present on the train recipe cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_mode: Option<String>,
    /// Rows written to `dataset.jsonl`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_rows: Option<usize>,
    /// True when `--from-feed` copied those rows from disk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_from_feed: Option<bool>,
    /// Scrubbed feed events left out because they had no note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_skipped: Option<usize>,
    /// Pack source paths actually read. Empty when the file is still a scaffold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_read_paths: Option<Vec<String>>,
}

/// How `dataset.jsonl` was built for a train recipe card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetMaterial {
    pub mode: String,
    pub rows: usize,
    pub skipped: usize,
    pub read_paths: Vec<String>,
    pub chat_jsonl: String,
    pub alpaca_jsonl: String,
    /// Shared honesty paragraph for PREPARE.md and NEXT.md.
    pub note: String,
}

const DATASET_STUB: &str = "stub";
const DATASET_SCAFFOLD: &str = "scaffold";
const DATASET_FEED: &str = "feed";
/// Per-file read cap. Prepare does not stream a trainer and does not download.
const DATASET_MAX_BYTES: u64 = 8 * 1024 * 1024;
/// All `--from-feed` sources together. Many paths cannot stack past this.
const DATASET_MAX_TOTAL_BYTES: u64 = 8 * 1024 * 1024;
/// Each in-memory copy (chat JSONL and Alpaca JSONL).
const DATASET_MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;
const DATASET_MAX_ROWS: usize = 4096;

pub struct PrepareEnrichRequest<'a> {
    pub estate: &'a Estate,
    pub pack: &'a PackManifest,
    pub curator: &'a str,
    pub driver_id: &'a str,
    pub job: &'a str,
    pub out_dir: &'a Path,
    /// LLaMA-Factory gauge cap. `None` leaves `max_steps` off the recipe.
    pub max_steps: Option<u32>,
    /// Copy instruct rows from pack `source_paths` under `state_dir`.
    pub from_feed: bool,
    /// Cell state directory. Pack source paths are relative to this directory.
    pub state_dir: &'a Path,
}

pub fn train_enrich_catalog() -> Vec<TrainEnrichCard> {
    REGISTRY.iter().map(|row| row.card).collect()
}

fn unknown_train_enrich_driver(id: &str) -> ModelError {
    let known = REGISTRY
        .iter()
        .map(|row| row.card.driver_id)
        .collect::<Vec<_>>()
        .join(", ");
    ModelError::Other(format!(
        "refuse:driver: unknown train/enrich driver '{id}' (catalog: {known})"
    ))
}

pub fn train_enrich_card(id: &str) -> Result<TrainEnrichCard, ModelError> {
    let id = id.trim();
    REGISTRY
        .iter()
        .find(|row| row.card.driver_id == id)
        .map(|row| row.card)
        .ok_or_else(|| unknown_train_enrich_driver(id))
}

pub fn resolve_train_enrich_driver(id: &str) -> Result<Box<dyn TrainEnrichDriver>, ModelError> {
    let id = id.trim();
    REGISTRY
        .iter()
        .find(|row| row.card.driver_id == id)
        .map(|row| (row.build)())
        .ok_or_else(|| unknown_train_enrich_driver(id))
}

/// Job written when the operator names one driver and omits `--job`.
pub fn driver_default_job(id: &str) -> Result<&'static str, ModelError> {
    Ok(train_enrich_card(id)?.default_job.as_str())
}

/// Cards that prepare `job`. `--all-drivers` uses this list.
pub fn train_enrich_drivers_for_job(job: &str) -> Result<Vec<&'static str>, ModelError> {
    let kind = parse_enrich_job(job)?;
    let ids: Vec<_> = REGISTRY
        .iter()
        .filter(|row| row.card.jobs.contains(&kind))
        .map(|row| row.card.driver_id)
        .collect();
    if ids.is_empty() {
        return Err(ModelError::Other(format!(
            "refuse:job: no train/enrich driver prepares '{}'",
            kind.as_str()
        )));
    }
    Ok(ids)
}

pub fn parse_enrich_job(raw: &str) -> Result<EnrichJobKind, ModelError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "enrich" => Ok(EnrichJobKind::Enrich),
        "train" => Ok(EnrichJobKind::Train),
        other => Err(ModelError::Other(format!(
            "refuse:job: '{other}' must be train or enrich"
        ))),
    }
}

pub fn default_enrich_out(state_dir: &Path, pack_id: &str, driver_id: &str) -> PathBuf {
    state_dir.join("enrich").join(pack_id).join(driver_id)
}

pub fn render_train_enrich_catalog() -> String {
    let mut lines = vec![
        "train/enrich drivers (data plane catalog; add a card to register an entrant):".to_string(),
    ];
    for card in train_enrich_catalog() {
        let probe = resolve_train_enrich_driver(card.driver_id)
            .map(|driver| driver.probe())
            .unwrap_or(TrainEnrichProbe {
                driver_id: card.driver_id.to_string(),
                status: card.status.to_string(),
                live: false,
                note: "unresolved".into(),
            });
        let jobs = card
            .jobs
            .iter()
            .map(|job| job.as_str())
            .collect::<Vec<_>>()
            .join(",");
        lines.push(format!(
            "  {:<20} status={:<12} jobs={} default={} integrates={} live={}",
            card.driver_id,
            card.status,
            jobs,
            card.default_job.as_str(),
            card.integrates,
            probe.live
        ));
        lines.push(format!("    {}", card.notes));
    }
    lines.push("probe is catalog-only. prepare does not shell out and does not POST.".into());
    lines.join("\n")
}

/// Load a pack file, or `{packs_dir}/{id}.pack.json`, or the accepted copy.
pub fn load_enrich_pack(pack: &Path, packs_dir: &Path) -> Result<PackManifest, ModelError> {
    if pack.is_file() {
        return read_pack_file(pack);
    }
    let id = pack.to_string_lossy();
    if id.ends_with(".json") || id.contains('/') || id.contains('\\') {
        return Err(ModelError::Other(format!(
            "refuse:missing-pack: {}",
            pack.display()
        )));
    }
    let drop = packs_dir.join(format!("{id}.pack.json"));
    if drop.is_file() {
        return read_pack_file(&drop);
    }
    let accepted = packs_dir.join("accepted").join(format!("{id}.pack.json"));
    if accepted.is_file() {
        return read_pack_file(&accepted);
    }
    Err(ModelError::Other(format!(
        "refuse:missing-pack: no pack '{id}' in drop or accepted"
    )))
}

struct StagedPrepare {
    out_dir: PathBuf,
    files: Vec<(String, String)>,
    doc: EnrichPrepareDoc,
}

/// Refuse, then write one driver. The estate file is not opened for write.
pub fn prepare_enrich(req: &PrepareEnrichRequest<'_>) -> Result<EnrichPrepareDoc, ModelError> {
    prepare_enrich_set(std::slice::from_ref(req))?
        .into_iter()
        .next()
        .ok_or_else(|| ModelError::Other("refuse:prepare: empty result".into()))
}

/// Stage every target, then write. A later refuse leaves no directory from this call.
pub fn prepare_enrich_set(
    reqs: &[PrepareEnrichRequest<'_>],
) -> Result<Vec<EnrichPrepareDoc>, ModelError> {
    if reqs.is_empty() {
        return Err(ModelError::Other(
            "refuse:driver: no train/enrich drivers to prepare".into(),
        ));
    }
    if reqs.iter().any(|req| req.from_feed)
        && !reqs
            .iter()
            .any(|req| is_train_recipe_driver(req.driver_id))
    {
        return Err(ModelError::Other(
            "refuse:dataset: --from-feed applies to llamafactory-qlora and axolotl-lora. This prepare has no train recipe card. Omit --from-feed to keep the other cards.".into(),
        ));
    }
    let mut drivers = BTreeSet::new();
    let mut outs = BTreeSet::new();
    for req in reqs {
        if !drivers.insert(req.driver_id) {
            return Err(ModelError::Other(format!(
                "refuse:driver: duplicate driver '{}'",
                req.driver_id
            )));
        }
        if !outs.insert(req.out_dir.to_path_buf()) {
            return Err(ModelError::Other(format!(
                "refuse:prepare: duplicate out {}",
                req.out_dir.display()
            )));
        }
    }
    let mut staged = Vec::with_capacity(reqs.len());
    for req in reqs {
        staged.push(stage_prepare(req)?);
    }
    commit_staged(&staged)
}

fn stage_prepare(req: &PrepareEnrichRequest<'_>) -> Result<StagedPrepare, ModelError> {
    refuse_curator(req.curator, &req.estate.enrich_packs.curator).map_err(map_feed)?;
    refuse_pack(req.pack).map_err(map_feed)?;
    refuse_frontier_source_on_estate(&req.pack.source_drivers, req.estate).map_err(map_feed)?;
    let kind = parse_enrich_job(req.job)?;
    let card = train_enrich_card(req.driver_id)?;
    if !card.jobs.contains(&kind) {
        let allowed = card
            .jobs
            .iter()
            .map(|job| job.as_str())
            .collect::<Vec<_>>()
            .join(",");
        return Err(ModelError::Other(format!(
            "refuse:job: {} prepares {allowed}; got {}",
            card.driver_id,
            kind.as_str()
        )));
    }
    let mut job = enrich_job(req.pack, req.estate, kind, req.out_dir, req.max_steps)?;
    refuse_job_text(&job)?;
    canonicalize_recipe_train_base(req.driver_id, &mut job)?;
    if is_train_recipe_driver(req.driver_id) {
        job.dataset = Some(materialize_dataset(req, &job)?);
    }
    refuse_sacred_and_sku("out", &req.out_dir.display().to_string())?;
    let driver = resolve_train_enrich_driver(req.driver_id)?;
    let prepared = driver.prepare(&job)?;
    let mut files = prepared.files;
    let prepare_md = prepare_markdown(driver.id(), &job, &prepared.steps);
    files.push(("PREPARE.md".into(), prepare_md));
    refuse_driver_files(&files)?;
    let mut names: Vec<String> = files.iter().map(|(name, _)| name.clone()).collect();
    names.push("prepare.json".into());
    names.push("NEXT.md".into());
    names.sort();
    names.dedup();
    let doc = EnrichPrepareDoc {
        schema: PREPARE_SCHEMA.into(),
        driver: driver.id().to_string(),
        job: job.kind.as_str().to_string(),
        pack_id: job.pack_id.clone(),
        base_model: job.base_model.clone(),
        seat_tag: Some(job.base_model.clone()),
        train_base_model: job.train_base_model.clone(),
        purpose: job.purpose.clone(),
        host_class_affinity: job.host_class_affinity.clone(),
        source_paths: job.source_paths.clone(),
        source_drivers: job.source_drivers.clone(),
        artifacts: names.clone(),
        promoted: false,
        auto_apply: false,
        estate_rewritten: false,
        note: PREPARE_NOTE.into(),
        dataset_mode: job.dataset.as_ref().map(|data| data.mode.clone()),
        dataset_rows: job.dataset.as_ref().map(|data| data.rows),
        dataset_from_feed: job.dataset.as_ref().map(|data| data.mode == DATASET_FEED),
        dataset_skipped: job.dataset.as_ref().map(|data| data.skipped),
        dataset_read_paths: job.dataset.as_ref().map(|data| data.read_paths.clone()),
    };
    let prepare_json = to_pretty(&doc)?;
    files.push(("prepare.json".into(), prepare_json));
    let next = next_markdown(driver.id(), &job, &job.out_dir, &names, &prepared.steps);
    files.push(("NEXT.md".into(), next));
    for (name, body) in &files {
        refuse_sacred_and_sku(name, body)?;
        refuse_raw_secrets(body).map_err(map_feed)?;
    }
    Ok(StagedPrepare {
        out_dir: req.out_dir.to_path_buf(),
        files,
        doc,
    })
}

fn commit_staged(staged: &[StagedPrepare]) -> Result<Vec<EnrichPrepareDoc>, ModelError> {
    for item in staged {
        write_files(&item.out_dir, &item.files)?;
    }
    Ok(staged.iter().map(|item| item.doc.clone()).collect())
}

const SEAT_ID: &str = "local_slm";

/// Words that name a seat, a driver, or a class. They are not Ollama model tags.
const RESERVED_MODEL_WORDS: &[&str] = &[
    "local_slm",
    "xai_grok",
    "ollama",
    "llama.cpp",
    "llama-cpp",
    "llama_cpp",
    "mlx",
    "vllm",
    "trt",
    "frontier-http",
    "http-remote",
    "mock-local",
    "openai-compat",
    "openai",
    "frontier",
    "local",
];

fn name_eq(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn is_reserved_model_word(name: &str) -> bool {
    RESERVED_MODEL_WORDS.iter().any(|word| name_eq(name, word))
}

fn is_binding_id(estate: &Estate, name: &str) -> bool {
    name_eq(name, SEAT_ID)
        || estate
            .model_bindings
            .iter()
            .any(|binding| name_eq(&binding.id, name))
}

fn is_from_token(name: &str) -> bool {
    !name.is_empty()
        && !name
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '#' | '\\'))
}

/// A seated model tag Ollama can `FROM`. Binding ids and driver ids are not tags.
fn is_seated_model_name(estate: &Estate, name: &str) -> bool {
    let name = name.trim();
    is_from_token(name) && !is_binding_id(estate, name) && !is_reserved_model_word(name)
}

fn select_local_binding<'a>(
    estate: &'a Estate,
    hint: Option<&str>,
) -> Result<&'a estate_schema::ModelBinding, ModelError> {
    if let Some(hint) = hint {
        if let Some(binding) = estate
            .model_bindings
            .iter()
            .find(|binding| name_eq(&binding.id, hint))
        {
            if binding.class != estate_schema::ModelClass::Local {
                return Err(ModelError::Other(format!(
                    "refuse:base-model: '{hint}' names binding '{}' which is not a local seat",
                    binding.id
                )));
            }
            return Ok(binding);
        }
    }
    if let Some(binding) = estate
        .model_bindings
        .iter()
        .find(|binding| binding.id == SEAT_ID && binding.class == estate_schema::ModelClass::Local)
    {
        return Ok(binding);
    }
    let mut locals = estate
        .model_bindings
        .iter()
        .filter(|binding| binding.class == estate_schema::ModelClass::Local);
    match (locals.next(), locals.next()) {
        (Some(only), None) => Ok(only),
        (None, _) => Err(ModelError::Other(
            "refuse:binding: estate has no local_slm seat".into(),
        )),
        (Some(_), Some(_)) => Err(ModelError::Other(
            "refuse:base-model: more than one local binding and the pack does not name one".into(),
        )),
    }
}

/// Modelfile `FROM` and manifest `base_model`. Never the binding id `local_slm`.
fn resolve_seated_base_model(estate: &Estate, pack: &PackManifest) -> Result<String, ModelError> {
    let hint = pack
        .model_hint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(hint) = hint {
        if is_seated_model_name(estate, hint) {
            return Ok(hint.to_string());
        }
        if !is_binding_id(estate, hint) && !is_reserved_model_word(hint) {
            return Err(ModelError::Other(format!(
                "refuse:base-model: model_hint '{hint}' is not a seated model tag"
            )));
        }
    }
    let binding = select_local_binding(estate, hint)?;
    let raw = binding
        .params
        .get("model")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .unwrap_or("");
    if raw.is_empty() {
        return Err(ModelError::Other(format!(
            "refuse:base-model: local binding '{}' has no params.model. Set params.model to a model the seated runtime already has (for example llama3), or set the pack model_hint to that model name. The binding id is not a Modelfile FROM.",
            binding.id
        )));
    }
    if !is_seated_model_name(estate, raw) {
        return Err(ModelError::Other(format!(
            "refuse:base-model: params.model on '{}' is '{raw}', which is not a seated model tag. The binding id is not a Modelfile FROM.",
            binding.id
        )));
    }
    Ok(raw.to_string())
}

fn enrich_job(
    pack: &PackManifest,
    estate: &Estate,
    kind: EnrichJobKind,
    out_dir: &Path,
    max_steps: Option<u32>,
) -> Result<EnrichJob, ModelError> {
    let purpose = {
        let note = pack.note.trim();
        if note.is_empty() {
            format!("purpose-built SLM for pack {}", pack.id)
        } else {
            note.to_string()
        }
    };
    let system_text = format!(
        "You are a purpose-built SLM for estate pack {}. Job is {}. {purpose}",
        pack.id,
        kind.as_str()
    );
    let base_model = resolve_seated_base_model(estate, pack)?;
    let train_base_model = resolve_train_base_model(estate, pack)?;
    let host_class_affinity = resolve_host_class_affinity(estate, pack);
    let max_steps = normalize_max_steps(max_steps)?;
    let job = EnrichJob {
        kind,
        pack_id: pack.id.clone(),
        base_model,
        train_base_model,
        purpose,
        system_text,
        host_class_affinity,
        source_paths: pack.source_paths.clone(),
        source_drivers: pack.source_drivers.clone(),
        out_dir: absolute_path(out_dir),
        max_steps,
        dataset: None,
    };
    Ok(job)
}

/// Pack `train_base_model`, then `params.train_base_model` on the seated local binding.
/// Missing is `Ok(None)`. A bare Ollama tag stored here is refused later by the train recipe cards.
fn resolve_train_base_model(
    estate: &Estate,
    pack: &PackManifest,
) -> Result<Option<String>, ModelError> {
    if let Some(from_pack) = nonempty(pack.train_base_model.as_deref()) {
        return Ok(Some(from_pack.to_string()));
    }
    let hint = nonempty(pack.model_hint.as_deref());
    let binding = select_local_binding(estate, hint)?;
    Ok(binding
        .params
        .get("train_base_model")
        .and_then(|value| value.as_str())
        .and_then(|value| nonempty(Some(value)))
        .map(str::to_string))
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn normalize_max_steps(max_steps: Option<u32>) -> Result<Option<u32>, ModelError> {
    match max_steps {
        None => Ok(None),
        Some(0) => Err(ModelError::Other(
            "refuse:max-steps: max_steps must be at least 1. Omit it for the one-epoch LLaMA-Factory recipe.".into(),
        )),
        Some(steps) => Ok(Some(steps)),
    }
}

/// Pack affinity, then the seated binding's `params.host_class`, then the pack host class.
fn resolve_host_class_affinity(estate: &Estate, pack: &PackManifest) -> String {
    if let Some(affinity) = pack
        .host_class_affinity
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return affinity.to_string();
    }
    if let Some(host) = estate
        .model_bindings
        .iter()
        .find(|binding| binding.id == SEAT_ID)
        .and_then(|binding| binding.params.get("host_class"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return host.to_string();
    }
    let host = pack.host_class.trim();
    if host.is_empty() {
        "any".to_string()
    } else {
        host.to_string()
    }
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(path),
        Err(_) => path.to_path_buf(),
    }
}

fn refuse_job_text(job: &EnrichJob) -> Result<(), ModelError> {
    refuse_sacred_and_sku("pack id", &job.pack_id)?;
    refuse_sacred_and_sku("base model", &job.base_model)?;
    if let Some(train_base) = job.train_base_model.as_deref() {
        refuse_sacred_and_sku("train base", train_base)?;
    }
    refuse_sacred_and_sku("purpose", &job.purpose)?;
    refuse_sacred_and_sku("system", &job.system_text)?;
    refuse_sacred_and_sku("host_class_affinity", &job.host_class_affinity)?;
    for path in &job.source_paths {
        refuse_sacred_and_sku("source path", path)?;
    }
    for driver in &job.source_drivers {
        refuse_sacred_and_sku("source driver", driver)?;
    }
    Ok(())
}

fn refuse_sacred_and_sku(field: &str, text: &str) -> Result<(), ModelError> {
    if text.is_empty() {
        return Ok(());
    }
    if contains_sku(text) {
        return Err(ModelError::Other(format!(
            "refuse:sku-banned: {field} encodes a hardware SKU"
        )));
    }
    let lower = text.to_ascii_lowercase();
    for token in SACRED_NEEDLES {
        if lower.contains(token) {
            return Err(ModelError::Other(format!(
                "refuse:sacred: {field} contains sacred token '{token}'"
            )));
        }
    }
    if is_sacred_name(text) {
        return Err(ModelError::Other(format!(
            "refuse:sacred: {field} is a sacred exclusion"
        )));
    }
    for word in text.split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_') {
        if word.is_empty() {
            continue;
        }
        if is_sacred_name(word) {
            return Err(ModelError::Other(format!(
                "refuse:sacred: {field} contains sacred token '{word}'"
            )));
        }
    }
    Ok(())
}

fn refuse_driver_files(files: &[(String, String)]) -> Result<(), ModelError> {
    let prepare_md = files
        .iter()
        .filter(|(name, _)| name == "PREPARE.md")
        .count();
    if prepare_md > 1 {
        return Err(ModelError::Other(
            "refuse:prepare: driver must not emit reserved file 'PREPARE.md'".into(),
        ));
    }
    for reserved in [
        "prepare.json",
        "NEXT.md",
        BINDING_PROPOSAL_JSON,
        BINDING_PROPOSAL_MD,
    ] {
        if files.iter().any(|(name, _)| name == reserved) {
            return Err(ModelError::Other(format!(
                "refuse:prepare: driver must not emit reserved file '{reserved}'"
            )));
        }
    }
    for (name, _) in files {
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name == "."
            || name == ".."
        {
            return Err(ModelError::Other(format!(
                "refuse:prepare: artifact name '{name}' is not a single file"
            )));
        }
    }
    Ok(())
}

fn prepare_markdown(driver_id: &str, job: &EnrichJob, steps: &str) -> String {
    let drivers = if job.source_drivers.is_empty() {
        "-".to_string()
    } else {
        job.source_drivers.join(",")
    };
    let train_lines = match job.train_base_model.as_deref() {
        Some(train) => format!("Seat tag: {}\nTrain base: {train}\n", job.base_model),
        None => String::new(),
    };
    let dataset_lines = match job.dataset.as_ref() {
        Some(data) => format!(
            "Dataset mode: {}\nDataset rows: {}\nDataset from feed: {}\nDataset skipped: {}\n",
            data.mode,
            data.rows,
            data.mode == DATASET_FEED,
            data.skipped,
        ),
        None => String::new(),
    };
    format!(
        "# Enrich prepare ({driver_id})\n\n\
         Pack: {pack}\n\
         Job: {kind}\n\
         Driver: {driver_id}\n\
         Base ref: {base}\n\
         {train_lines}\
         {dataset_lines}\
         Host class affinity: {host}\n\
         Source drivers: {drivers}\n\
         Promoted: false\n\
         auto_apply: false\n\
         estate_rewritten: false\n\
         \n\
         {steps}\n\
         Schema: {schema}\n",
        pack = job.pack_id,
        kind = job.kind.as_str(),
        base = job.base_model,
        host = job.host_class_affinity,
        schema = PREPARE_SCHEMA,
    )
}

fn next_markdown(
    driver_id: &str,
    job: &EnrichJob,
    out_dir: &Path,
    artifacts: &[String],
    steps: &str,
) -> String {
    let tag = local_enrich_tag(&job.pack_id);
    let mut artifact_lines = String::new();
    for name in artifacts {
        artifact_lines.push_str(&format!("- {}\n", out_dir.join(name).display()));
    }
    let modelfile = out_dir.join("Modelfile");
    let manifest_json = out_dir.join("manifest.json");
    let manifest_yaml = out_dir.join("manifest.yaml");
    let config = out_dir.join("axolotl.yml");
    let (handoff, import_line, path_note) = if driver_id == "ollama-modelfile" {
        (
            format!(
                "Run this on the seated host. This factory does not run it.\n\
                 \n\
                 ollama create {tag} -f {modelfile}\n\
                 \n\
                 FROM {base} must already be a model that runtime has. That name comes from params.model on the local binding, or from a pack model_hint that is already a model tag. The binding id is not a model tag. This factory did not pull weights.\n",
                modelfile = modelfile.display(),
                base = job.base_model,
            ),
            format!(
                "estate enrich import-prepared --estate <estate.yaml> --prepared {out} --tag {tag} --path {path}\n",
                out = out_dir.display(),
                path = modelfile.display(),
            ),
            "Path is the Modelfile this prepare wrote.".to_string(),
        )
    } else if driver_id == "external-manifest" {
        (
            format!(
                "Hand these files to a trainer outside this factory.\n\
                 \n\
                 {json}\n\
                 {yaml}\n\
                 \n\
                 Fields: base_model, purpose, host_class_affinity, dataset_paths.\n\
                 This factory does not POST a train job and does not shell out.\n\
                 When weights return, load them on the seated local runtime as tag {tag}.\n",
                json = manifest_json.display(),
                yaml = manifest_yaml.display(),
            ),
            format!(
                "estate enrich import-prepared --estate <estate.yaml> --prepared {out} --tag {tag} --path {path}\n",
                out = out_dir.display(),
                path = manifest_json.display(),
            ),
            "Point --path at the weights file you loaded. Until then, manifest.json is the portable hatch.".to_string(),
        )
    } else if driver_id == LLAMAFACTORY_QLORA_ID {
        let recipe = out_dir.join("recipe.yaml");
        let export = out_dir.join("export.yaml");
        let train_command = llamafactory_train_command(&recipe);
        let export_command = llamafactory_export_command(&export);
        let train_base = job.train_base_model.as_deref().unwrap_or("");
        let template = llamafactory_template(train_base);
        (
            format!(
                "Run this on a CUDA host (consumer-nvidia or rented-nvidia). This factory does not run it, does not download weights, and does not call CUDA.\n\
                 \n\
                 {install}\n\
                 {train_command}\n\
                 {export_command}\n\
                 \n\
                 {bits}\n\
                 \n\
                 If the torch wheel still does not match the CUDA install on the box, use the official install: https://github.com/hiyouga/LLaMA-Factory#installation\n\
                 SFT: https://llamafactory.readthedocs.io/en/latest/getting_started/sft.html\n\
                 Merge: https://llamafactory.readthedocs.io/en/latest/getting_started/merge_lora.html\n\
                 \n\
                 {host}\n\
                 \n\
                 This card expects CUDA LLaMA-Factory.\n\
                 \n\
                 {dataset}\n\
                 \n\
                 Seat tag is {seat}. That is the Ollama id for Modelfile FROM. It comes from params.model on the local binding, or from a pack model_hint that is already a model tag.\n\
                 \n\
                 Train base is {train}. recipe.yaml and export.yaml set model_name_or_path to that value. A train base is a Hugging Face repo id (namespace/name) or a local directory of HF weights. This factory did not download weights and does not map the seat tag onto a Hub repo.\n\
                 \n\
                 template in recipe.yaml and export.yaml is {template}. That hint comes from the train base name. Confirm it matches the model. Train and seat share the same chat template. When you seat on Ollama, the Modelfile TEMPLATE (or the GGUF chat template) must be that same chat format. This factory does not write a second template.\n\
                 \n\
                 {gauge}\n\
                 \n\
                 Merge with llamafactory-cli export. Do not set quantization_bit on export.yaml, and do not merge a quantized base. LLaMA-Factory does not write GGUF. After the merge, convert with llama.cpp if you want a GGUF, then seat tag {tag} on Ollama with FROM that GGUF. To load the adapter without a merge, FROM must be an Ollama model of this same train base, plus ADAPTER for the adapter directory. The seat tag {seat} is the id this cell already runs. This factory does not run ollama create.\n\
                 \n\
                 After that tag is seated, send a short prompt that checks the pack purpose. This factory does not run that smoke eval.\n\
                 \n\
                 A later preference stage is a recipe flag (stage: dpo or stage: orpo, with ranking: true in dataset_info.json). This card does not build that dataset.\n\
                 \n\
                 ## Faster single-GPU alternate\n\
                 \n\
                 On Nvidia only, Unsloth QLoRA is a faster single-GPU path. This card does not call Unsloth and does not write a script.\n\
                 https://unsloth.ai/docs/get-started/fine-tuning-llms-guide\n\
                 https://unsloth.ai/docs/get-started/install\n\
                 https://github.com/unslothai/unsloth\n\
                 \n\
                 axolotl-lora is the YAML recipe when you want a config-driven or multi-GPU run. This card does not call Axolotl.\n",
                host = llamafactory_host_note(&job.host_class_affinity),
                dataset = train_dataset_blurb(job, CHAT_DATASET_SHAPE),
                seat = job.base_model,
                train = train_base,
                template = template,
                train_command = train_command,
                export_command = export_command,
                install = llamafactory_install_lines(),
                bits = llamafactory_bitsandbytes_note(),
                gauge = llamafactory_gauge_note(job.max_steps),
            ),
            format!(
                "estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter <adapter-dir-or-gguf>\n",
                out = out_dir.display(),
            ),
            "Point --adapter at the LLaMA-Factory output directory (adapter_config.json inside it), the merged export (config.json plus safetensors), or a GGUF you converted.".to_string(),
        )
    } else if driver_id == AXOLOTL_LORA_ID {
        let command = axolotl_train_command(&config);
        let train_base = job.train_base_model.as_deref().unwrap_or("");
        (
            format!(
                "Run this on a CUDA host (consumer-nvidia or rented-nvidia). This factory does not run it, does not download weights, and does not call CUDA.\n\
                 \n\
                 {command}\n\
                 \n\
                 {host}\n\
                 \n\
                 {dataset}\n\
                 \n\
                 Seat tag is {seat}. That is the Ollama id for Modelfile FROM. It comes from params.model on the local binding, or from a pack model_hint that is already a model tag.\n\
                 \n\
                 Train base is {train}. axolotl.yml sets base_model to that value. A train base is a Hugging Face repo id (namespace/name) or a local directory of HF weights. This factory did not download weights and does not map the seat tag onto a Hub repo.\n\
                 \n\
                 Axolotl writes the adapter under the output_dir in axolotl.yml. Ollama stays the local-run seat. After you create tag {tag} on Ollama, record the join below. Use a Modelfile FROM of a merged GGUF, or FROM an Ollama model of this same train base, plus ADAPTER for the adapter directory. The seat tag {seat} is the id this cell already runs. This factory does not run ollama create.\n\
                 \n\
                 llamafactory-qlora is the durable LLaMA-Factory recipe. This card does not call LLaMA-Factory.\n\
                 Unsloth QLoRA is a faster single-GPU alternate on Nvidia only (https://github.com/unslothai/unsloth). This card does not call Unsloth.\n",
                host = axolotl_host_note(&job.host_class_affinity),
                dataset = train_dataset_blurb(job, ALPACA_DATASET_SHAPE),
                seat = job.base_model,
                train = train_base,
            ),
            format!(
                "estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter <adapter-dir-or-gguf>\n",
                out = out_dir.display(),
            ),
            "Point --adapter at the Axolotl output directory (adapter_config.json inside it) or at a merged GGUF file.".to_string(),
        )
    } else {
        (
            format!("{steps}\n"),
            format!(
                "estate enrich import-prepared --estate <estate.yaml> --prepared {out} --tag {tag} --path {path}\n",
                out = out_dir.display(),
                path = out_dir.join("prepare.json").display(),
            ),
            "Point --path at the file you loaded on the seated runtime.".to_string(),
        )
    };
    format!(
        "# Next ({driver_id})\n\
         \n\
         Prepared artifacts only. Does not train, does not POST, does not shell out, does not promote, and does not rewrite estate.yaml.\n\
         \n\
         Pack: {pack}\n\
         Job: {kind}\n\
         Driver: {driver_id}\n\
         Local tag: {tag}\n\
         Out: {out}\n\
         \n\
         Artifacts:\n\
         {artifact_lines}\
         \n\
         {handoff}\
         {path_note}\n\
         \n\
         After that model exists, record a binding proposal, then stage it. apply-proposal does not apply. estate plan and estate apply --require-plan write the source estate only when require-plan succeeds.\n\
         \n\
         {import_line}\
         \n\
         estate enrich apply-proposal --estate <estate.yaml> --prepared {out} --tag {tag} --state-dir <state-dir>\n\
         \n\
         Fail closed:\n\
         - curator must be jason (refuse:curator)\n\
         - sacred tokens refuse (refuse:sacred)\n\
         - a hardware SKU in a tag or path refuses (refuse:sku-banned)\n\
         - a frontier source with no frontier binding refuses (refuse:frontier-invent)\n\
         - promoted, auto_apply, and estate_rewritten stay false\n\
         - a missing enrich directory refuses on list (refuse:enrich-index)\n\
         \n\
         Schema: {schema}\n",
        pack = job.pack_id,
        kind = job.kind.as_str(),
        out = out_dir.display(),
        schema = PREPARE_SCHEMA,
    )
}

fn dataset_lines(paths: &[String]) -> String {
    if paths.is_empty() {
        "  - (none)".into()
    } else {
        paths
            .iter()
            .map(|path| format!("  - {path}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// CUDA hosts for LLaMA-Factory and Axolotl. Apple Silicon can hold the recipe.
const CUDA_TRAIN_HOSTS: &[&str] = &["consumer-nvidia", "rented-nvidia"];

fn cuda_train_host_note(driver_id: &str, affinity: &str, command: &str) -> String {
    let hosts_or = CUDA_TRAIN_HOSTS.join(" or ");
    let hosts_and = CUDA_TRAIN_HOSTS.join(" and ");
    match affinity {
        "consumer-nvidia" | "rented-nvidia" => format!(
            "{affinity} is a supported train host for {driver_id}. Run {command} on that CUDA host."
        ),
        "apple-silicon" => format!(
            "host_class_affinity is apple-silicon. This recipe can be prepared on that machine. This card expects a CUDA host ({hosts_or}). This factory does not write an MLX trainer."
        ),
        "any" => format!(
            "host_class_affinity is any. Run {command} on {hosts_or}. An apple-silicon machine can hold this recipe. This card expects CUDA. This factory does not write an MLX trainer."
        ),
        other => format!(
            "host_class_affinity is {other}. Supported train hosts for {driver_id} are {hosts_and}. This card expects a CUDA host. This factory does not write an MLX trainer."
        ),
    }
}

fn axolotl_host_note(affinity: &str) -> String {
    cuda_train_host_note(AXOLOTL_LORA_ID, affinity, "axolotl train")
}

fn llamafactory_host_note(affinity: &str) -> String {
    cuda_train_host_note(LLAMAFACTORY_QLORA_ID, affinity, "llamafactory-cli train")
}

const CHAT_DATASET_SHAPE: &str =
    "dataset.jsonl is instruct chat JSONL (messages of role and content).";
const ALPACA_DATASET_SHAPE: &str =
    "dataset.jsonl is Alpaca JSONL (instruction, input, output).";

fn require_dataset(job: &EnrichJob) -> Result<&DatasetMaterial, ModelError> {
    job.dataset.as_ref().ok_or_else(|| {
        ModelError::Other("refuse:dataset: train recipe has no dataset plan".into())
    })
}

fn dataset_card_note(shape: &str, data: &DatasetMaterial) -> String {
    format!("{shape}\n\n{}", data.note)
}

fn train_dataset_blurb(job: &EnrichJob, shape: &str) -> String {
    match job.dataset.as_ref() {
        Some(data) => dataset_card_note(shape, data),
        None => "dataset_mode: missing. refuse:dataset: train recipe has no dataset plan.".into(),
    }
}

fn dataset_honesty_note(
    mode: &str,
    paths: &[String],
    rows: usize,
    skipped: usize,
    state_dir: &Path,
) -> String {
    let listed = if paths.is_empty() {
        "(none)".to_string()
    } else {
        paths.join(", ")
    };
    let example = paths
        .first()
        .map(|path| state_dir.join(path))
        .unwrap_or_else(|| state_dir.join("feed/events.jsonl"));
    match mode {
        DATASET_STUB => "dataset_mode: stub. dataset.jsonl is a stub of example rows because the pack source_paths list is empty. These rows are not training data. Replace them before train. This factory did not download a dataset. Add source_paths that already exist under the cell state directory and prepare again with --from-feed. A missing file with --from-feed is refuse:dataset. Omit --from-feed to keep this stub.".into(),
        DATASET_SCAFFOLD => format!(
            "dataset_mode: scaffold. dataset.jsonl names pack source paths ({listed}) and does not contain their rows. These rows are not training data. This factory did not read those files and did not download them. Replace the rows before train, or prepare again with --from-feed when each path is already a file under the cell state directory (for example {}). A missing file with --from-feed is refuse:dataset. Omit --from-feed to keep this scaffold.",
            example.display()
        ),
        DATASET_FEED => {
            let row_word = if rows == 1 {
                "instruct row"
            } else {
                "instruct rows"
            };
            let skipped_sentence = if skipped == 1 {
                "1 scrubbed feed event had no note and was left out".to_string()
            } else {
                format!("{skipped} scrubbed feed events had no note and were left out")
            };
            format!(
                "dataset_mode: feed. dataset.jsonl has {rows} {row_word} copied from {listed} under {}. {skipped_sentence}. This factory did not download a dataset and did not invent a completion. ShareGPT messages and Alpaca instruction/output lines are copied. A scrubbed feed event becomes a row only when its note is present. A file that is missing, unreadable, or outside the cell state directory is refuse:dataset.",
                state_dir.display()
            )
        }
        other => format!("dataset_mode: {other}. refuse:dataset: unknown dataset mode."),
    }
}

fn axolotl_train_command(config: &Path) -> String {
    format!("axolotl train {}", shell_quote(config))
}

fn llamafactory_train_command(recipe: &Path) -> String {
    format!("llamafactory-cli train {}", shell_quote(recipe))
}

fn llamafactory_export_command(export_yaml: &Path) -> String {
    format!("llamafactory-cli export {}", shell_quote(export_yaml))
}

fn shell_quote(path: &Path) -> String {
    let text = path.display().to_string();
    if text
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '\\'))
    {
        format!("'{}'", text.replace('\'', "'\\''"))
    } else {
        text
    }
}

#[derive(Debug, Serialize)]
struct AlpacaRow<'a> {
    instruction: String,
    input: &'a str,
    output: &'a str,
}

fn refuse_blank_source_paths(paths: &[String]) -> Result<(), ModelError> {
    for path in paths {
        if path.trim().is_empty() {
            return Err(ModelError::Other(
                "refuse:dataset: source path is empty".into(),
            ));
        }
    }
    Ok(())
}

/// Alpaca JSONL Axolotl already reads (`type: alpaca`, `ds_type: json`).
fn axolotl_dataset_jsonl(job: &EnrichJob) -> Result<(String, bool), ModelError> {
    refuse_blank_source_paths(&job.source_paths)?;
    let stub = job.source_paths.is_empty();
    let rows: Vec<AlpacaRow<'_>> = if stub {
        vec![
            AlpacaRow {
                instruction: format!("You are the purpose-built SLM for pack {}.", job.pack_id),
                input: "",
                output: "Replace this example completion before training.",
            },
            AlpacaRow {
                instruction: "Name the pack this recipe was prepared from.".into(),
                input: "",
                output: job.pack_id.as_str(),
            },
            AlpacaRow {
                instruction: job.purpose.clone(),
                input: "",
                output: "Replace this example completion before training.",
            },
        ]
    } else {
        job.source_paths
            .iter()
            .map(|path| AlpacaRow {
                instruction: format!(
                    "Draft a specialist completion for pack {} using the source at this path.",
                    job.pack_id
                ),
                input: path.as_str(),
                output: "Replace this scaffold with a completion from that source. Cell One did not read the file.",
            })
            .collect()
    };
    if rows.is_empty() {
        return Err(ModelError::Other(
            "refuse:dataset: no dataset rows to write".into(),
        ));
    }
    let mut body = String::new();
    for row in &rows {
        let line = serde_json::to_string(row)
            .map_err(|err| ModelError::Other(format!("refuse:dataset: serialize row: {err}")))?;
        body.push_str(&line);
        body.push('\n');
    }
    Ok((body, stub))
}

#[derive(Serialize)]
struct ChatTurn {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct ChatRow {
    messages: Vec<ChatTurn>,
}

fn chat_row(user: String, assistant: String) -> ChatRow {
    ChatRow {
        messages: vec![
            ChatTurn {
                role: "user",
                content: user,
            },
            ChatTurn {
                role: "assistant",
                content: assistant,
            },
        ],
    }
}

/// Instruct chat JSONL. LLaMA-Factory reads it as sharegpt (`messages` of `role` / `content`).
fn chat_dataset_jsonl(job: &EnrichJob) -> Result<(String, bool), ModelError> {
    refuse_blank_source_paths(&job.source_paths)?;
    let stub = job.source_paths.is_empty();
    let rows: Vec<ChatRow> = if stub {
        vec![
            chat_row(
                format!("You are the purpose-built SLM for pack {}.", job.pack_id),
                "Replace this example reply before training.".into(),
            ),
            chat_row(
                "Name the pack this recipe was prepared from.".into(),
                job.pack_id.clone(),
            ),
            chat_row(
                job.purpose.clone(),
                "Replace this example reply before training.".into(),
            ),
        ]
    } else {
        job.source_paths
            .iter()
            .map(|path| {
                chat_row(
                    format!(
                        "Draft a specialist reply for pack {} using the source at {path}. Cell One did not read that file.",
                        job.pack_id
                    ),
                    "Replace this scaffold with a reply from that source.".into(),
                )
            })
            .collect()
    };
    if rows.is_empty() {
        return Err(ModelError::Other(
            "refuse:dataset: no dataset rows to write".into(),
        ));
    }
    let mut body = String::new();
    for row in &rows {
        let line = serde_json::to_string(row)
            .map_err(|err| ModelError::Other(format!("refuse:dataset: serialize row: {err}")))?;
        body.push_str(&line);
        body.push('\n');
    }
    Ok((body, stub))
}

#[derive(Serialize)]
struct AlpacaOwned {
    instruction: String,
    input: String,
    output: String,
}

struct HydratedRow {
    chat: ChatRow,
    alpaca: AlpacaOwned,
}

enum InstructLine {
    Row(HydratedRow),
    Skip,
}

fn jsonl_row_count(body: &str) -> usize {
    body.lines().filter(|line| !line.trim().is_empty()).count()
}

fn append_jsonl(body: &mut String, row: &impl Serialize) -> Result<(), ModelError> {
    let line = serde_json::to_string(row)
        .map_err(|err| ModelError::Other(format!("refuse:dataset: serialize row: {err}")))?;
    body.push_str(&line);
    body.push('\n');
    Ok(())
}

/// Default prepare keeps the scaffold. `--from-feed` copies rows already on disk.
fn materialize_dataset(
    req: &PrepareEnrichRequest<'_>,
    job: &EnrichJob,
) -> Result<DatasetMaterial, ModelError> {
    refuse_blank_source_paths(&job.source_paths)?;
    if req.from_feed {
        return hydrate_dataset(req, job);
    }
    let stub = job.source_paths.is_empty();
    let (chat_jsonl, _) = chat_dataset_jsonl(job)?;
    let (alpaca_jsonl, _) = axolotl_dataset_jsonl(job)?;
    let mode = if stub { DATASET_STUB } else { DATASET_SCAFFOLD };
    let rows = jsonl_row_count(&chat_jsonl);
    Ok(DatasetMaterial {
        mode: mode.to_string(),
        rows,
        skipped: 0,
        read_paths: Vec::new(),
        chat_jsonl,
        alpaca_jsonl,
        note: dataset_honesty_note(mode, &job.source_paths, rows, 0, req.state_dir),
    })
}

fn hydrate_dataset(
    req: &PrepareEnrichRequest<'_>,
    job: &EnrichJob,
) -> Result<DatasetMaterial, ModelError> {
    if job.source_paths.is_empty() {
        return Err(ModelError::Other(
            "refuse:dataset: --from-feed needs pack source_paths. The list is empty. This factory does not download a dataset. Omit --from-feed to keep the stub.".into(),
        ));
    }
    let mut rows = Vec::new();
    let mut skipped = 0usize;
    let mut used_bytes = 0u64;
    for relative in &job.source_paths {
        let text = read_feed_source(req.state_dir, relative, used_bytes)?;
        used_bytes = used_bytes.saturating_add(text.len() as u64);
        let (found, skip) = parse_instruct_file(req.estate, relative, &text)?;
        skipped += skip;
        rows.extend(found);
        if rows.len() > DATASET_MAX_ROWS {
            return Err(ModelError::Other(format!(
                "refuse:dataset: pack sources together exceed {DATASET_MAX_ROWS} instruct rows. Prepare does not stream a trainer and does not download a dataset."
            )));
        }
    }
    if rows.is_empty() {
        return Err(ModelError::Other(format!(
            "refuse:dataset: {} has no instruct rows ({skipped} scrubbed feed events had no note). A line is an instruct row when it is ShareGPT messages, Alpaca instruction and output, or a scrubbed feed event with a note. This factory does not invent a completion and does not download a dataset.",
            job.source_paths.join(", ")
        )));
    }
    let mut chat_jsonl = String::new();
    let mut alpaca_jsonl = String::new();
    for row in &rows {
        append_jsonl(&mut chat_jsonl, &row.chat)?;
        append_jsonl(&mut alpaca_jsonl, &row.alpaca)?;
        refuse_dataset_output(chat_jsonl.len())?;
        refuse_dataset_output(alpaca_jsonl.len())?;
    }
    let count = rows.len();
    Ok(DatasetMaterial {
        mode: DATASET_FEED.to_string(),
        rows: count,
        skipped,
        read_paths: job.source_paths.clone(),
        chat_jsonl,
        alpaca_jsonl,
        note: dataset_honesty_note(
            DATASET_FEED,
            &job.source_paths,
            count,
            skipped,
            req.state_dir,
        ),
    })
}

fn refuse_dataset_output(bytes: usize) -> Result<(), ModelError> {
    if bytes > DATASET_MAX_OUTPUT_BYTES {
        return Err(ModelError::Other(format!(
            "refuse:dataset: dataset.jsonl would be {bytes} bytes. Prepare writes at most {DATASET_MAX_OUTPUT_BYTES} bytes for the chat copy and {DATASET_MAX_OUTPUT_BYTES} bytes for the Alpaca copy. This factory does not download a dataset."
        )));
    }
    Ok(())
}

/// Open `relative` under `state_dir` and read that same file handle.
///
/// Containment uses the opened fd (`/proc/self/fd/<fd>` on Linux, `F_GETPATH`
/// on macOS), then the bytes come from that fd. A symlink swap after the
/// check cannot retarget the read. If the fd path cannot be resolved, prepare
/// refuses. `bytes_already` is the total already accepted from earlier sources.
fn read_feed_source(
    state_dir: &Path,
    relative: &str,
    bytes_already: u64,
) -> Result<String, ModelError> {
    let relative = relative.trim();
    if relative.is_empty() {
        return Err(ModelError::Other(
            "refuse:dataset: source path is empty".into(),
        ));
    }
    if Path::new(relative).is_absolute() || relative.contains("..") || relative.contains('\\') {
        return Err(ModelError::Other(format!(
            "refuse:dataset: source path '{relative}' is not a relative path under the cell state directory. This factory does not download pack sources."
        )));
    }
    let root = absolute_path(state_dir);
    let candidate = root.join(relative);
    let root_canon = match std::fs::canonicalize(&root) {
        Ok(path) => path,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(missing_feed_source(relative, &candidate));
        }
        Err(err) => {
            return Err(ModelError::Other(format!(
                "refuse:dataset: cannot resolve cell state directory {}: {err}",
                root.display()
            )));
        }
    };
    let file = std::fs::File::open(&candidate).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            missing_feed_source(relative, &candidate)
        } else {
            ModelError::Other(format!(
                "refuse:dataset: cannot read {relative} at {}: {err}. This factory does not download pack sources.",
                candidate.display()
            ))
        }
    })?;
    let pinned = opened_file_path(&file).map_err(|err| {
        ModelError::Other(format!(
            "refuse:dataset: cannot pin {relative} to the cell state directory ({err}). Prepare refuses when the opened file cannot be resolved and does not download pack sources."
        ))
    })?;
    if !pinned.is_absolute() || !path_is_within(&root_canon, &pinned) {
        return Err(ModelError::Other(format!(
            "refuse:dataset: {relative} resolves outside the cell state directory {}. This factory does not read pack sources outside that directory and does not download them.",
            root_canon.display()
        )));
    }
    let meta = file.metadata().map_err(|err| {
        ModelError::Other(format!(
            "refuse:dataset: cannot stat {relative}: {err}"
        ))
    })?;
    if !meta.is_file() {
        return Err(missing_feed_source(relative, &candidate));
    }
    let bytes = meta.len();
    if bytes > DATASET_MAX_BYTES {
        return Err(ModelError::Other(format!(
            "refuse:dataset: {relative} is {bytes} bytes. Prepare reads at most {DATASET_MAX_BYTES} bytes per source and does not download a dataset."
        )));
    }
    let total = bytes_already.saturating_add(bytes);
    if total > DATASET_MAX_TOTAL_BYTES {
        return Err(ModelError::Other(format!(
            "refuse:dataset: pack sources are {total} bytes together. Prepare reads at most {DATASET_MAX_TOTAL_BYTES} bytes across all source paths. This factory does not download a dataset."
        )));
    }
    let text = read_capped_fd(file, relative)?;
    let read_total = bytes_already.saturating_add(text.len() as u64);
    if read_total > DATASET_MAX_TOTAL_BYTES {
        return Err(ModelError::Other(format!(
            "refuse:dataset: pack sources are {read_total} bytes together. Prepare reads at most {DATASET_MAX_TOTAL_BYTES} bytes across all source paths. This factory does not download a dataset."
        )));
    }
    Ok(text)
}

fn path_is_within(root: &Path, file: &Path) -> bool {
    file.starts_with(root) && file != root
}

fn missing_feed_source(relative: &str, candidate: &Path) -> ModelError {
    ModelError::Other(format!(
        "refuse:dataset: {relative} is not a file at {}. Place that pack source under the cell state directory, or omit --from-feed to keep the scaffold. This factory does not download pack sources.",
        candidate.display()
    ))
}

fn read_capped_fd(file: std::fs::File, relative: &str) -> Result<String, ModelError> {
    use std::io::Read;
    let mut file = file.take(DATASET_MAX_BYTES.saturating_add(1));
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|err| {
        ModelError::Other(format!("refuse:dataset: cannot read {relative}: {err}"))
    })?;
    if buf.len() as u64 > DATASET_MAX_BYTES {
        return Err(ModelError::Other(format!(
            "refuse:dataset: {relative} exceeds {DATASET_MAX_BYTES} bytes. Prepare reads at most {DATASET_MAX_BYTES} bytes per source and does not download a dataset."
        )));
    }
    String::from_utf8(buf).map_err(|_| {
        ModelError::Other(format!(
            "refuse:dataset: {relative} is not UTF-8. Prepare does not download a dataset."
        ))
    })
}

/// Path of an already-opened file. Fail closed when the platform cannot name it.
#[cfg(target_os = "linux")]
fn opened_file_path(file: &std::fs::File) -> std::io::Result<PathBuf> {
    use std::os::unix::io::AsRawFd;
    std::fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))
}

#[cfg(target_os = "macos")]
fn opened_file_path(file: &std::fs::File) -> std::io::Result<PathBuf> {
    use std::os::unix::io::AsRawFd;
    const F_GETPATH: i32 = 50;
    extern "C" {
        fn fcntl(fd: i32, cmd: i32, ...) -> i32;
    }
    let mut buf = [0u8; 4096];
    let rc = unsafe { fcntl(file.as_raw_fd(), F_GETPATH, buf.as_mut_ptr()) };
    if rc == -1 {
        return Err(std::io::Error::last_os_error());
    }
    let end = buf.iter().position(|byte| *byte == 0).unwrap_or(buf.len());
    let text = std::str::from_utf8(&buf[..end]).map_err(|err| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, err)
    })?;
    Ok(PathBuf::from(text))
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn opened_file_path(_file: &std::fs::File) -> std::io::Result<PathBuf> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "opened-file path is unavailable",
    ))
}

fn parse_instruct_file(
    estate: &Estate,
    relative: &str,
    text: &str,
) -> Result<(Vec<HydratedRow>, usize), ModelError> {
    let mut rows = Vec::new();
    let mut skipped = 0usize;
    for (idx, line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(trimmed).map_err(|err| {
            ModelError::Other(format!(
                "refuse:dataset: {relative} line {line_no} is not JSON: {err}"
            ))
        })?;
        if !value.is_object() {
            return Err(ModelError::Other(format!(
                "refuse:dataset: {relative} line {line_no} is not a JSON object. Accepted lines are ShareGPT messages, Alpaca instruction and output, or a scrubbed feed event."
            )));
        }
        match classify_instruct_line(estate, relative, line_no, &value)? {
            InstructLine::Row(row) => {
                refuse_hydrated_text(relative, line_no, &row)?;
                rows.push(row);
            }
            InstructLine::Skip => skipped += 1,
        }
    }
    Ok((rows, skipped))
}

fn classify_instruct_line(
    estate: &Estate,
    relative: &str,
    line_no: usize,
    value: &serde_json::Value,
) -> Result<InstructLine, ModelError> {
    // Event policy runs before ShareGPT / Alpaca dispatch. A frontier event
    // wrapped as `messages` or `instruction` is still refuse:frontier-invent.
    refuse_record_policy(estate, relative, line_no, value)?;
    if value.get("messages").is_some() {
        return Ok(InstructLine::Row(chat_source_row(relative, line_no, value)?));
    }
    if value.get("instruction").is_some() {
        return Ok(InstructLine::Row(alpaca_source_row(
            relative, line_no, value,
        )?));
    }
    if value.get("kind").is_some() {
        return event_source_line(estate, relative, line_no, value);
    }
    Err(ModelError::Other(format!(
        "refuse:dataset: {relative} line {line_no} is not an instruct row. Accepted lines are ShareGPT messages, Alpaca instruction and output, or a scrubbed feed event with a note."
    )))
}

/// Frontier, sacred, SKU, and raw-secret checks on the source object.
/// Shape dispatch has not copied `messages` or `instruction` yet.
fn refuse_record_policy(
    estate: &Estate,
    relative: &str,
    line_no: usize,
    value: &serde_json::Value,
) -> Result<(), ModelError> {
    if let Some(event) = scrubbed_event_from_record(relative, line_no, value)? {
        if classify_path(&event) == "frontier" {
            refuse_frontier_source_on_estate(&["frontier".to_string()], estate).map_err(map_feed)?;
        }
    }
    refuse_json_text(&format!("{relative} line {line_no}"), value)
}

fn scrubbed_event_from_record(
    relative: &str,
    line_no: usize,
    value: &serde_json::Value,
) -> Result<Option<ScrubbedEvent>, ModelError> {
    let kind_value = value.get("kind");
    let class_value = value.get("object_class");
    let has_kind = kind_value.is_some() && !kind_value.unwrap().is_null();
    let has_class = class_value.is_some() && !class_value.unwrap().is_null();
    if !has_kind && !has_class {
        return Ok(None);
    }
    let kind = if has_kind {
        match kind_value {
            Some(serde_json::Value::String(kind)) if !kind.trim().is_empty() => {
                kind.trim().to_string()
            }
            Some(serde_json::Value::String(_)) => {
                return Err(ModelError::Other(format!(
                    "refuse:dataset: {relative} line {line_no} kind is empty"
                )));
            }
            _ => {
                return Err(ModelError::Other(format!(
                    "refuse:dataset: {relative} line {line_no} kind is not a string"
                )));
            }
        }
    } else {
        String::new()
    };
    Ok(Some(ScrubbedEvent {
        kind,
        agent_id: optional_text(value, "agent_id", relative, line_no)?,
        decision: optional_text(value, "decision", relative, line_no)?,
        object_class: optional_text(value, "object_class", relative, line_no)?,
        note: optional_text(value, "note", relative, line_no)?,
        ts: optional_text(value, "ts", relative, line_no)?.unwrap_or_default(),
    }))
}

fn refuse_json_text(label: &str, value: &serde_json::Value) -> Result<(), ModelError> {
    match value {
        serde_json::Value::String(text) => refuse_text_policy(label, text),
        serde_json::Value::Array(items) => {
            for item in items {
                refuse_json_text(label, item)?;
            }
            Ok(())
        }
        serde_json::Value::Object(map) => {
            for (key, item) in map {
                refuse_text_policy(label, key)?;
                refuse_json_text(label, item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn refuse_text_policy(label: &str, text: &str) -> Result<(), ModelError> {
    refuse_sacred_and_sku(label, text)?;
    refuse_raw_secrets(text).map_err(|err| {
        let text = err.to_string();
        if text.starts_with("refuse:") {
            ModelError::Other(format!("{text} ({label})"))
        } else {
            ModelError::Other(format!("refuse:dataset: {label}: {text}"))
        }
    })
}

fn chat_source_row(
    relative: &str,
    line_no: usize,
    value: &serde_json::Value,
) -> Result<HydratedRow, ModelError> {
    let messages = value.get("messages").and_then(|item| item.as_array()).ok_or_else(|| {
        ModelError::Other(format!(
            "refuse:dataset: {relative} line {line_no} messages is not an array"
        ))
    })?;
    if messages.is_empty() {
        return Err(ModelError::Other(format!(
            "refuse:dataset: {relative} line {line_no} messages is empty"
        )));
    }
    let mut turns = Vec::new();
    let mut users = 0usize;
    let mut assistants = 0usize;
    for item in messages {
        let role = item.get("role").and_then(|value| value.as_str()).unwrap_or("").trim();
        let content = item
            .get("content")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let role = match role {
            "system" => "system",
            "user" => "user",
            "assistant" => "assistant",
            other => {
                return Err(ModelError::Other(format!(
                    "refuse:dataset: {relative} line {line_no} has role '{other}'. Roles are system, user, and assistant."
                )));
            }
        };
        if content.is_empty() {
            return Err(ModelError::Other(format!(
                "refuse:dataset: {relative} line {line_no} has an empty {role} message"
            )));
        }
        if role == "user" {
            users += 1;
        }
        if role == "assistant" {
            assistants += 1;
        }
        turns.push(ChatTurn {
            role,
            content: content.to_string(),
        });
    }
    if users == 0 || assistants == 0 {
        return Err(ModelError::Other(format!(
            "refuse:dataset: {relative} line {line_no} needs a user message and an assistant message"
        )));
    }
    let alpaca = alpaca_from_turns(&turns);
    Ok(HydratedRow {
        chat: ChatRow { messages: turns },
        alpaca,
    })
}

fn alpaca_source_row(
    relative: &str,
    line_no: usize,
    value: &serde_json::Value,
) -> Result<HydratedRow, ModelError> {
    let instruction = value
        .get("instruction")
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .trim();
    let output = value
        .get("output")
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .trim();
    let input = value
        .get("input")
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .trim();
    if instruction.is_empty() || output.is_empty() {
        return Err(ModelError::Other(format!(
            "refuse:dataset: {relative} line {line_no} needs a non-empty instruction and output"
        )));
    }
    let user = if input.is_empty() {
        instruction.to_string()
    } else {
        format!("{instruction}\n\n{input}")
    };
    Ok(HydratedRow {
        chat: chat_row(user, output.to_string()),
        alpaca: AlpacaOwned {
            instruction: instruction.to_string(),
            input: input.to_string(),
            output: output.to_string(),
        },
    })
}

fn event_source_line(
    estate: &Estate,
    relative: &str,
    line_no: usize,
    value: &serde_json::Value,
) -> Result<InstructLine, ModelError> {
    let kind = match value.get("kind") {
        Some(serde_json::Value::String(kind)) if !kind.trim().is_empty() => kind.trim().to_string(),
        Some(serde_json::Value::String(_)) => {
            return Err(ModelError::Other(format!(
                "refuse:dataset: {relative} line {line_no} kind is empty"
            )));
        }
        _ => {
            return Err(ModelError::Other(format!(
                "refuse:dataset: {relative} line {line_no} kind is not a string"
            )));
        }
    };
    let event = ScrubbedEvent {
        kind,
        agent_id: optional_text(value, "agent_id", relative, line_no)?,
        decision: optional_text(value, "decision", relative, line_no)?,
        object_class: optional_text(value, "object_class", relative, line_no)?,
        note: optional_text(value, "note", relative, line_no)?,
        ts: optional_text(value, "ts", relative, line_no)?.unwrap_or_default(),
    };
    if classify_path(&event) == "frontier" {
        refuse_frontier_source_on_estate(&["frontier".to_string()], estate).map_err(map_feed)?;
    }
    let Some(note) = event.note.clone() else {
        return Ok(InstructLine::Skip);
    };
    let instruction = event_instruction(&event);
    Ok(InstructLine::Row(HydratedRow {
        chat: chat_row(instruction.clone(), note.clone()),
        alpaca: AlpacaOwned {
            instruction,
            input: String::new(),
            output: note,
        },
    }))
}

fn optional_text(
    value: &serde_json::Value,
    key: &str,
    relative: &str,
    line_no: usize,
) -> Result<Option<String>, ModelError> {
    match value.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(text)) => {
            let text = text.trim();
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(text.to_string()))
            }
        }
        Some(_) => Err(ModelError::Other(format!(
            "refuse:dataset: {relative} line {line_no} field {key} is not a string"
        ))),
    }
}

fn event_instruction(event: &ScrubbedEvent) -> String {
    let mut parts = vec![format!("kind={}", event.kind.trim())];
    if let Some(class) = event
        .object_class
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(format!("class={class}"));
    }
    if let Some(decision) = event
        .decision
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(format!("decision={decision}"));
    }
    if let Some(agent) = event
        .agent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(format!("agent={agent}"));
    }
    parts.join(" ")
}

fn alpaca_from_turns(turns: &[ChatTurn]) -> AlpacaOwned {
    let mut instruction = String::new();
    let mut output = String::new();
    for turn in turns {
        let bucket = if turn.role == "assistant" {
            &mut output
        } else {
            &mut instruction
        };
        if !bucket.is_empty() {
            bucket.push_str("\n\n");
        }
        bucket.push_str(&turn.content);
    }
    AlpacaOwned {
        instruction,
        input: String::new(),
        output,
    }
}

fn refuse_hydrated_text(relative: &str, line_no: usize, row: &HydratedRow) -> Result<(), ModelError> {
    let field = format!("{relative} line {line_no}");
    refuse_sacred_and_sku(&field, &row.alpaca.instruction)?;
    refuse_sacred_and_sku(&field, &row.alpaca.input)?;
    refuse_sacred_and_sku(&field, &row.alpaca.output)?;
    for turn in &row.chat.messages {
        refuse_sacred_and_sku(&field, &turn.content)?;
        refuse_raw_secrets(&turn.content).map_err(|err| {
            let text = err.to_string();
            if text.starts_with("refuse:") {
                ModelError::Other(format!("{text} ({field})"))
            } else {
                ModelError::Other(format!("refuse:dataset: {field}: {text}"))
            }
        })?;
    }
    Ok(())
}

const LLAMAFACTORY_SAVE_STEPS: u32 = 50;

fn llamafactory_install_lines() -> &'static str {
    "pip install llamafactory\npip install 'bitsandbytes>=0.49'"
}

fn llamafactory_bitsandbytes_note() -> &'static str {
    "QLoRA needs bitsandbytes. `pip install llamafactory` and `llamafactory[torch,metrics]` 0.9.5 did not install it. Install bitsandbytes in that same environment (`pip install 'bitsandbytes>=0.49'`). On a consumer RTX host, keep the torch CUDA wheel you already installed. A consumer RTX smoke on CUDA 12.8 used torch 2.11.0+cu128 and bitsandbytes 0.50.2. That bitsandbytes install did not replace torch. This factory does not install either package."
}

fn llamafactory_gauge_note(max_steps: Option<u32>) -> String {
    let base = "A short gauge run does not need a full epoch. Re-prepare with `--max-steps 10`. LLaMA-Factory overrides `num_train_epochs` when `max_steps` is set. When that count is under 50, this prepare sets `save_steps` to the same count so a checkpoint is written during the short run. The default recipe keeps `num_train_epochs: 1.0` and `save_steps: 50`, and leaves `max_steps` unset.";
    match max_steps {
        Some(steps) => format!("{base}\n\nThis recipe is a gauge run with max_steps {steps}."),
        None => base.to_string(),
    }
}

fn llamafactory_save_steps(max_steps: Option<u32>) -> u32 {
    match max_steps {
        Some(steps) if steps < LLAMAFACTORY_SAVE_STEPS => steps,
        _ => LLAMAFACTORY_SAVE_STEPS,
    }
}

fn require_train_base(job: &EnrichJob) -> Result<String, ModelError> {
    match job
        .train_base_model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(train_base) => canonical_train_base(train_base, &job.base_model),
        None => Err(train_base_error("", &job.base_model)),
    }
}

/// Hub ids stay as typed. A local directory is stored as an absolute path so
/// LLaMA-Factory does not resolve `model_name_or_path` against process CWD.
fn canonical_train_base(raw: &str, seat: &str) -> Result<String, ModelError> {
    let value = raw.trim();
    if is_hub_repo_id(value) {
        return Ok(value.to_string());
    }
    if !is_local_hf_dir(value) {
        return Err(train_base_error(value, seat));
    }
    if leaf_is_seat_tag(value, seat) {
        return Err(seat_leaf_error(value, seat));
    }
    let absolute = lexical_absolute(&absolute_path(Path::new(value)));
    if !absolute.starts_with('/') || !is_local_hf_dir(&absolute) {
        return Err(ModelError::Other(format!(
            "refuse:train-base: '{value}' could not be stored as an absolute directory of HF weights. Set pack field train_base_model, or params.train_base_model on the local binding, to a Hugging Face repo id (namespace/name) or to a local directory of HF weights."
        )));
    }
    if leaf_is_seat_tag(&absolute, seat) {
        return Err(seat_leaf_error(value, seat));
    }
    Ok(absolute)
}

fn lexical_absolute(path: &Path) -> String {
    let mut parts = Vec::new();
    let mut rooted = false;
    for component in path.components() {
        match component {
            std::path::Component::RootDir => rooted = true,
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::Normal(part) => parts.push(part.to_os_string()),
            std::path::Component::Prefix(_) => {}
        }
    }
    let mut out = if rooted {
        PathBuf::from("/")
    } else {
        PathBuf::new()
    };
    for part in parts {
        out.push(part);
    }
    out.display().to_string()
}

fn accept_train_base(raw: &str, seat: &str) -> Result<(), ModelError> {
    let value = raw.trim();
    if value.is_empty() || !is_train_base(value) {
        return Err(train_base_error(value, seat));
    }
    if is_local_hf_dir(value) && leaf_is_seat_tag(value, seat) {
        return Err(seat_leaf_error(value, seat));
    }
    Ok(())
}

fn is_train_base(value: &str) -> bool {
    is_hub_repo_id(value) || is_local_hf_dir(value)
}

fn is_hub_repo_id(value: &str) -> bool {
    if value.len() > 192 || value.matches('/').count() != 1 {
        return false;
    }
    let Some((namespace, name)) = value.split_once('/') else {
        return false;
    };
    is_hub_segment(namespace) && is_hub_segment(name)
}

fn is_hub_segment(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    if value.len() > 96 {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn is_local_hf_dir(value: &str) -> bool {
    if value.len() < 2 || value.len() > 512 || value.ends_with('/') {
        return false;
    }
    if value
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || matches!(c, ':' | '"' | '\'' | '#' | '\\'))
    {
        return false;
    }
    let relative = value.starts_with("./") || value.starts_with("../");
    let absolute = value.starts_with('/');
    if !relative && !absolute {
        return false;
    }
    let body = value.trim_start_matches('.').trim_start_matches('/');
    !body.is_empty()
}

fn path_leaf(value: &str) -> &str {
    value.rsplit('/').next().unwrap_or(value).trim()
}

/// Basename of a local weights path. Ollama seat tags are lowercase ids
/// (`llama3`, `qwen2.5`) or the same id with a different case (`Llama3`).
/// A mixed-case name that contains a hyphen is a Hugging Face snapshot
/// directory (`Qwen2.5-0.5B-Instruct`), so it stays a local train base.
fn leaf_is_seat_tag(path: &str, seat: &str) -> bool {
    let leaf = path_leaf(path);
    if leaf.is_empty() || leaf == "." || leaf == ".." {
        return false;
    }
    let seat_name = seat.split(':').next().unwrap_or(seat);
    if name_eq(leaf, seat) || (!seat_name.is_empty() && name_eq(leaf, seat_name)) {
        return true;
    }
    let folded = leaf.to_ascii_lowercase();
    if !looks_like_bare_seat_tag(&folded) {
        return false;
    }
    if leaf.chars().any(|c| c.is_ascii_uppercase()) && leaf.contains('-') {
        return false;
    }
    true
}

fn seat_leaf_error(raw: &str, seat: &str) -> ModelError {
    let leaf = path_leaf(raw);
    ModelError::Other(format!(
        "refuse:train-base: '{raw}' ends in '{leaf}', which looks like an Ollama seat tag. Seat tag '{seat}' is the Ollama id for Modelfile FROM. Set pack field train_base_model, or params.train_base_model on the local binding, to a Hugging Face repo id (namespace/name) or to a local directory of HF weights. This factory does not map the seat tag onto a Hub repo and does not download weights."
    ))
}

fn looks_like_bare_seat_tag(value: &str) -> bool {
    if value.contains('/') || value.is_empty() {
        return false;
    }
    let (name, tag) = match value.split_once(':') {
        Some((name, tag)) => (name, Some(tag)),
        None => (value, None),
    };
    if value.matches(':').count() > 1 {
        return false;
    }
    is_ollama_name_part(name) && tag.map(is_ollama_name_part).unwrap_or(true)
}

fn is_ollama_name_part(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn train_base_error(raw: &str, seat: &str) -> ModelError {
    let why = if raw.is_empty() {
        format!("no train base is set. Seat tag '{seat}' is the Ollama id for Modelfile FROM.")
    } else if looks_like_bare_seat_tag(raw) {
        format!(
            "'{raw}' looks like an Ollama seat tag. Seat tag '{seat}' is the Ollama id for Modelfile FROM."
        )
    } else {
        format!("'{raw}' is not a Hugging Face repo id (namespace/name) or a local directory of HF weights.")
    };
    ModelError::Other(format!(
        "refuse:train-base: {why} Set pack field train_base_model, or params.train_base_model on the local binding, to a Hugging Face repo id (namespace/name) or to a local directory of HF weights (an absolute path, or a path that starts with ./). This factory does not map the seat tag onto a Hub repo and does not download weights."
    ))
}

fn yaml_field(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some(rest) = line.strip_prefix(key) else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix(':') else {
            continue;
        };
        let rest = rest.trim();
        if rest.is_empty() {
            return None;
        }
        if let Some(inner) = rest.strip_prefix('"') {
            let inner = inner.strip_suffix('"').unwrap_or(inner);
            return Some(inner.replace("\\\"", "\"").replace("\\\\", "\\"));
        }
        return Some(rest.to_string());
    }
    None
}

fn canonicalize_recipe_train_base(driver_id: &str, job: &mut EnrichJob) -> Result<(), ModelError> {
    if !is_train_recipe_driver(driver_id) {
        return Ok(());
    }
    if let Some(raw) = job.train_base_model.clone() {
        let canonical = canonical_train_base(&raw, &job.base_model)?;
        job.train_base_model = Some(canonical);
        if let Some(train_base) = job.train_base_model.as_deref() {
            refuse_sacred_and_sku("train base", train_base)?;
        }
    }
    Ok(())
}

fn refuse_recipe_train_record(
    doc: &EnrichPrepareDoc,
    prepared_dir: &Path,
) -> Result<(), ModelError> {
    if !is_train_recipe_driver(&doc.driver) {
        return Ok(());
    }
    let train = match doc
        .train_base_model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(train) => {
            accept_train_base(train, &doc.base_model)?;
            train.to_string()
        }
        None => return Err(train_base_error("", &doc.base_model)),
    };
    let checks: &[(&str, &str)] = if doc.driver == LLAMAFACTORY_QLORA_ID {
        &[
            ("recipe.yaml", "model_name_or_path"),
            ("export.yaml", "model_name_or_path"),
        ]
    } else {
        &[("axolotl.yml", "base_model")]
    };
    for (name, key) in checks {
        let path = prepared_dir.join(name);
        let text = std::fs::read_to_string(&path).map_err(|_| {
            ModelError::Other(format!(
                "refuse:train-base: {name} is missing, so the train base cannot be checked"
            ))
        })?;
        let found = yaml_field(&text, key).unwrap_or_default();
        if found != train {
            return Err(ModelError::Other(format!(
                "refuse:train-base: {name} {key} is '{found}'. prepare.json train_base_model is '{train}'. Seat tag '{}' is the Ollama id for Modelfile FROM. These files must name the train base.",
                doc.base_model
            )));
        }
    }
    Ok(())
}

/// Chat template hint from the train base name. Confirm it before train. Seat the same chat format.
fn llamafactory_template(train_base: &str) -> &'static str {
    let name = train_base.to_ascii_lowercase();
    if name.contains("qwen3") {
        "qwen3"
    } else if name.contains("qwen") {
        "qwen"
    } else if name.contains("llama3") || name.contains("llama-3") {
        "llama3"
    } else if name.contains("mistral") {
        "mistral"
    } else if name.contains("gemma") {
        "gemma"
    } else if name.contains("phi") {
        "phi"
    } else {
        "default"
    }
}

fn llamafactory_dataset_info() -> String {
    format!(
        "{{\n  \"{name}\": {{\n    \"file_name\": \"dataset.jsonl\",\n    \"formatting\": \"sharegpt\",\n    \"columns\": {{\n      \"messages\": \"messages\"\n    }},\n    \"tags\": {{\n      \"role_tag\": \"role\",\n      \"content_tag\": \"content\",\n      \"user_tag\": \"user\",\n      \"assistant_tag\": \"assistant\"\n    }}\n  }}\n}}\n",
        name = LLAMAFACTORY_DATASET_NAME,
    )
}

fn llamafactory_recipe_yaml(job: &EnrichJob, mode: &str, train_base: &str) -> String {
    let template = llamafactory_template(train_base);
    let outputs = job.out_dir.join("outputs");
    let host = yaml_comment_line(&llamafactory_host_note(&job.host_class_affinity));
    let save_steps = llamafactory_save_steps(job.max_steps);
    let gauge = match job.max_steps {
        Some(steps) => format!(
            "# Gauge run. max_steps {steps} overrides num_train_epochs.\n\
             max_steps: {steps}\n"
        ),
        None => {
            "# One epoch. A short gauge run passes --max-steps. This file leaves max_steps unset.\n"
                .to_string()
        }
    };
    format!(
        "# schema: {schema}\n\
         # driver: {driver}\n\
         # job: train\n\
         # pack: {pack}\n\
         # seat_tag: {seat}\n\
         # train_base: {train_comment}\n\
         # Recipe only. Cell One does not run llamafactory-cli, does not download weights, and does not call CUDA.\n\
         # seat_tag is the Ollama id for Modelfile FROM.\n\
         # model_name_or_path is the train base: a Hugging Face repo id or a local directory of HF weights.\n\
         # template is inferred from the train base. Confirm it matches the model.\n\
         # Use this same chat template when you seat the model.\n\
         # This factory does not map the seat tag onto a Hub repo.\n\
         # dataset_mode: {mode}\n\
         # {host}\n\
         # QLoRA is finetuning_type lora plus quantization_bit 4.\n\
         # quantization_method is bnb. That is the LLaMA-Factory 0.9 token that selects the 4-bit bitsandbytes branch.\n\
         # Smoke-scale cutoff_len is {cutoff}. Official SFT examples use 2048 for a longer run.\n\
         model_name_or_path: {train}\n\
         trust_remote_code: true\n\
         stage: sft\n\
         # Later preference stage is a recipe flag. Not built here.\n\
         # stage: dpo\n\
         # or stage: orpo\n\
         # with ranking: true on the dataset in dataset_info.json\n\
         do_train: true\n\
         finetuning_type: lora\n\
         lora_rank: {rank}\n\
         lora_alpha: {alpha}\n\
         lora_target: all\n\
         quantization_bit: 4\n\
         quantization_method: bnb\n\
         dataset: {dataset_name}\n\
         dataset_dir: {dataset_dir}\n\
         template: {template}\n\
         cutoff_len: {cutoff}\n\
         packing: true\n\
         preprocessing_num_workers: 1\n\
         dataloader_num_workers: 1\n\
         output_dir: {outputs}\n\
         logging_steps: 1\n\
         save_steps: {save_steps}\n\
         overwrite_output_dir: true\n\
         save_only_model: false\n\
         report_to: none\n\
         per_device_train_batch_size: 1\n\
         gradient_accumulation_steps: 4\n\
         learning_rate: 1.0e-4\n\
         num_train_epochs: 1.0\n\
         {gauge}\
         lr_scheduler_type: cosine\n\
         warmup_ratio: 0.03\n\
         bf16: true\n\
         seed: {seed}\n",
        schema = PREPARE_SCHEMA,
        driver = LLAMAFACTORY_QLORA_ID,
        pack = job.pack_id,
        seat = job.base_model,
        train_comment = train_base,
        cutoff = LLAMAFACTORY_CUTOFF_LEN,
        rank = LLAMAFACTORY_LORA_RANK,
        alpha = LLAMAFACTORY_LORA_ALPHA,
        seed = LLAMAFACTORY_SEED,
        train = yaml_quote(train_base),
        dataset_name = LLAMAFACTORY_DATASET_NAME,
        dataset_dir = yaml_quote(&job.out_dir.display().to_string()),
        outputs = yaml_quote(&outputs.display().to_string()),
        template = template,
        mode = mode,
        host = host,
        save_steps = save_steps,
        gauge = gauge,
    )
}

fn llamafactory_export_yaml(job: &EnrichJob, train_base: &str) -> String {
    let template = llamafactory_template(train_base);
    let adapter = job.out_dir.join("outputs");
    let export_dir = job.out_dir.join("export");
    format!(
        "# schema: {schema}\n\
         # driver: {driver}\n\
         # seat_tag: {seat}\n\
         # train_base: {train_comment}\n\
         # Merge only. Leave this file unquantized. Do not merge a quantized base.\n\
         # model_name_or_path is the train base: the unquantized Hugging Face repo or local HF weights you trained from.\n\
         # The seat tag is the Ollama id for Modelfile FROM. It is a different field.\n\
         # adapter_name_or_path is the train output_dir.\n\
         # LLaMA-Factory does not export GGUF. Convert the merge with llama.cpp if you want an Ollama GGUF.\n\
         # template is inferred from the train base and must match recipe.yaml. Train and seat share one chat template.\n\
         model_name_or_path: {train}\n\
         adapter_name_or_path: {adapter}\n\
         template: {template}\n\
         trust_remote_code: true\n\
         finetuning_type: lora\n\
         export_dir: {export_dir}\n\
         export_size: 5\n\
         export_device: cpu\n\
         export_legacy_format: false\n",
        schema = PREPARE_SCHEMA,
        driver = LLAMAFACTORY_QLORA_ID,
        seat = job.base_model,
        train_comment = train_base,
        train = yaml_quote(train_base),
        adapter = yaml_quote(&adapter.display().to_string()),
        export_dir = yaml_quote(&export_dir.display().to_string()),
    )
}

fn axolotl_recipe_yaml(job: &EnrichJob, mode: &str, train_base: &str) -> String {
    let dataset = job.out_dir.join("dataset.jsonl");
    let prepared = job.out_dir.join("dataset_prepared");
    let outputs = job.out_dir.join("outputs");
    let host = yaml_comment_line(&axolotl_host_note(&job.host_class_affinity));
    format!(
        "# schema: {schema}\n\
         # driver: {driver}\n\
         # job: train\n\
         # pack: {pack}\n\
         # seat_tag: {seat}\n\
         # train_base: {train_comment}\n\
         # Recipe only. Cell One does not run axolotl, does not download weights, and does not rewrite the estate.\n\
         # seat_tag is the Ollama id for Modelfile FROM.\n\
         # base_model is the train base: a Hugging Face repo id or a local directory of HF weights.\n\
         # This factory does not map the seat tag onto a Hub repo.\n\
         # dataset_mode: {mode}\n\
         # {host}\n\
         base_model: {base}\n\
         load_in_8bit: false\n\
         load_in_4bit: true\n\
         adapter: qlora\n\
         lora_r: 16\n\
         lora_alpha: 32\n\
         lora_dropout: 0.05\n\
         lora_target_linear: true\n\
         datasets:\n  - path: {dataset}\n    ds_type: json\n    type: alpaca\n\
         dataset_prepared_path: {prepared}\n\
         val_set_size: 0.0\n\
         output_dir: {outputs}\n\
         sequence_len: 2048\n\
         sample_packing: false\n\
         pad_to_sequence_len: true\n\
         micro_batch_size: 1\n\
         gradient_accumulation_steps: 4\n\
         num_epochs: 1\n\
         optimizer: adamw_bnb_8bit\n\
         lr_scheduler: cosine\n\
         learning_rate: 0.0002\n\
         bf16: auto\n\
         tf32: false\n\
         gradient_checkpointing: true\n\
         warmup_ratio: 0.03\n\
         logging_steps: 1\n\
         evals_per_epoch: 0\n\
         saves_per_epoch: 1\n",
        schema = PREPARE_SCHEMA,
        driver = AXOLOTL_LORA_ID,
        pack = job.pack_id,
        seat = job.base_model,
        train_comment = train_base,
        base = yaml_quote(train_base),
        dataset = yaml_quote(&dataset.display().to_string()),
        prepared = yaml_quote(&prepared.display().to_string()),
        outputs = yaml_quote(&outputs.display().to_string()),
        mode = mode,
        host = host,
    )
}

fn yaml_comment_line(text: &str) -> String {
    text.replace(['\n', '\r'], " ")
}

fn write_files(out_dir: &Path, files: &[(String, String)]) -> Result<(), ModelError> {
    std::fs::create_dir_all(out_dir).map_err(|err| {
        ModelError::Other(format!(
            "refuse:prepare-write: {}: {err}",
            out_dir.display()
        ))
    })?;
    for (name, body) in files {
        let path = out_dir.join(name);
        std::fs::write(&path, body).map_err(|err| {
            ModelError::Other(format!("refuse:prepare-write: {}: {err}", path.display()))
        })?;
    }
    Ok(())
}

/// One prepared driver directory under `{state}/enrich/{pack}/{driver}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedEntry {
    pub pack_id: String,
    pub driver: String,
    pub job: String,
    pub local_tag: String,
    pub out_dir: PathBuf,
}

/// Read `.cell/enrich`. Does not create the directory and does not write.
pub fn list_prepared(enrich_root: &Path) -> Result<Vec<PreparedEntry>, ModelError> {
    if !enrich_root.exists() {
        return Err(ModelError::Other(format!(
            "refuse:enrich-index: {} is missing",
            enrich_root.display()
        )));
    }
    if !enrich_root.is_dir() {
        return Err(ModelError::Other(format!(
            "refuse:enrich-index: {} is not a directory",
            enrich_root.display()
        )));
    }
    let mut rows = Vec::new();
    for pack_dir in dir_children(enrich_root)? {
        if !pack_dir.is_dir() {
            return Err(ModelError::Other(format!(
                "refuse:enrich-index: unexpected file {}",
                pack_dir.display()
            )));
        }
        let pack_name = file_name(&pack_dir)?;
        for driver_dir in dir_children(&pack_dir)? {
            if !driver_dir.is_dir() {
                return Err(ModelError::Other(format!(
                    "refuse:enrich-index: unexpected file {}",
                    driver_dir.display()
                )));
            }
            let driver_name = file_name(&driver_dir)?;
            let doc = load_prepare_doc(&driver_dir.join("prepare.json"))?;
            if doc.pack_id != pack_name {
                return Err(ModelError::Other(format!(
                    "refuse:enrich-index: pack id '{}' does not match {}",
                    doc.pack_id,
                    pack_dir.display()
                )));
            }
            if doc.driver != driver_name {
                return Err(ModelError::Other(format!(
                    "refuse:enrich-index: driver '{}' does not match {}",
                    doc.driver,
                    driver_dir.display()
                )));
            }
            rows.push(PreparedEntry {
                local_tag: local_enrich_tag(&pack_name),
                pack_id: doc.pack_id,
                driver: doc.driver,
                job: doc.job,
                out_dir: driver_dir,
            });
        }
    }
    rows.sort_by(|a, b| (&a.pack_id, &a.driver).cmp(&(&b.pack_id, &b.driver)));
    Ok(rows)
}

pub fn render_prepared_index(enrich_root: &Path, rows: &[PreparedEntry]) -> String {
    let mut lines = vec![
        format!("enrich index (read-only; {})", enrich_root.display()),
        "estate not rewritten. list does not train.".to_string(),
    ];
    if rows.is_empty() {
        lines.push("(none)".to_string());
    }
    for row in rows {
        lines.push(format!(
            "  pack={} driver={} job={} tag={} out={}",
            row.pack_id,
            row.driver,
            row.job,
            row.local_tag,
            row.out_dir.display()
        ));
    }
    lines.push(format!("count={}", rows.len()));
    lines.push("promoted=false auto_apply=false estate_rewritten=false".to_string());
    lines.push(
        "handoff is NEXT.md in each out dir. join: estate enrich import-prepared --prepared <out> --tag <tag> --path <file> ; then estate enrich apply-proposal"
            .to_string(),
    );
    lines.join("\n")
}

/// Operator-supplied join. Validates prepare.json and writes a proposal for the
/// existing `local_slm` seat. Does not apply and does not rewrite the estate.
pub struct ImportPreparedRequest<'a> {
    pub estate: &'a Estate,
    pub prepared_dir: &'a Path,
    pub tag: &'a str,
    pub path: &'a Path,
    pub curator: &'a str,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
pub struct EnrichBindingProposal {
    pub schema: String,
    pub curator: String,
    pub policy: String,
    pub auto_apply: bool,
    pub promoted: bool,
    pub estate_rewritten: bool,
    pub estate_name: String,
    pub estate_hash: String,
    pub prepared_dir: String,
    pub pack_id: String,
    pub driver: String,
    pub job: String,
    pub local_tag: String,
    pub local_path: String,
    pub binding_id: String,
    pub seated_driver: String,
    pub content_scanned: bool,
    pub proposed_binding: serde_json::Value,
    pub paste_yaml: String,
    pub note: String,
}

pub fn import_prepared(
    req: &ImportPreparedRequest<'_>,
) -> Result<EnrichBindingProposal, ModelError> {
    refuse_curator(req.curator, &req.estate.enrich_packs.curator).map_err(map_feed)?;
    refuse_sacred_and_sku("prepared dir", &req.prepared_dir.display().to_string())?;
    let doc = load_prepare_doc(&req.prepared_dir.join("prepare.json"))?;
    refuse_recipe_train_record(&doc, req.prepared_dir)?;
    refuse_frontier_source_on_estate(&doc.source_drivers, req.estate).map_err(map_feed)?;
    let expected = local_enrich_tag(&doc.pack_id);
    refuse_sacred_and_sku("tag", req.tag)?;
    if req.tag != expected {
        return Err(ModelError::Other(format!(
            "refuse:tag: '{tag}' must be {expected}",
            tag = req.tag
        )));
    }
    let path_text = req.path.display().to_string();
    refuse_sacred_and_sku("operator path", &path_text)?;
    let content_scanned = scan_operator_file(req.path)?;
    let seat = req
        .estate
        .model_bindings
        .iter()
        .find(|binding| binding.id == "local_slm")
        .ok_or_else(|| ModelError::Other("refuse:binding: estate has no local_slm seat".into()))?;
    if seat.class != estate_schema::ModelClass::Local {
        return Err(ModelError::Other(
            "refuse:binding: local_slm class is not local".into(),
        ));
    }
    if seat.driver.trim().is_empty() {
        return Err(ModelError::Other(
            "refuse:binding: local_slm driver is empty".into(),
        ));
    }
    refuse_sacred_and_sku("seated driver", &seat.driver)?;
    let mut params = match &seat.params {
        serde_json::Value::Object(map) => serde_json::Value::Object(map.clone()),
        serde_json::Value::Null => serde_json::json!({}),
        _ => {
            return Err(ModelError::Other(
                "refuse:binding: local_slm params must be an object".into(),
            ))
        }
    };
    {
        let obj = params.as_object_mut().ok_or_else(|| {
            ModelError::Other("refuse:binding: local_slm params must be an object".into())
        })?;
        obj.insert(
            "model".into(),
            serde_json::Value::String(req.tag.to_string()),
        );
        obj.insert(
            "prepared_pack".into(),
            serde_json::Value::String(doc.pack_id.clone()),
        );
        obj.insert(
            "prepared_driver".into(),
            serde_json::Value::String(doc.driver.clone()),
        );
        for (key, value) in obj.iter() {
            if let Some(text) = value.as_str() {
                refuse_sacred_and_sku(&format!("params.{key}"), text)?;
            }
        }
    }
    let proposed_binding = serde_json::json!({
        "id": "local_slm",
        "class": "local",
        "driver": seat.driver,
        "wired": seat.wired,
        "params": params,
    });
    let paste_yaml = binding_paste_yaml(&proposed_binding)?;
    let proposal = EnrichBindingProposal {
        schema: BINDING_PROPOSAL_SCHEMA.into(),
        curator: "jason".into(),
        policy: "manual".into(),
        auto_apply: false,
        promoted: false,
        estate_rewritten: false,
        estate_name: req.estate.name.clone(),
        estate_hash: estate_schema::estate_hash(req.estate),
        prepared_dir: req.prepared_dir.display().to_string(),
        pack_id: doc.pack_id.clone(),
        driver: doc.driver.clone(),
        job: doc.job.clone(),
        local_tag: expected,
        local_path: path_text,
        binding_id: "local_slm".into(),
        seated_driver: seat.driver.clone(),
        content_scanned,
        proposed_binding,
        paste_yaml,
        note: format!(
            "Proposal only. auto_apply=false. Next: estate enrich apply-proposal, then estate plan and estate apply --require-plan. import-prepared does not apply, does not promote, and does not rewrite the estate. Operator file content_scanned={content_scanned}."
        ),
    };
    let json = to_pretty(&proposal)?;
    let md = render_binding_proposal(&proposal);
    refuse_sacred_and_sku("binding proposal", &json)?;
    refuse_sacred_and_sku("binding proposal", &md)?;
    refuse_raw_secrets(&json).map_err(map_feed)?;
    refuse_raw_secrets(&md).map_err(map_feed)?;
    write_files(
        req.prepared_dir,
        &[
            (BINDING_PROPOSAL_JSON.into(), json),
            (BINDING_PROPOSAL_MD.into(), md),
        ],
    )?;
    Ok(proposal)
}

/// Adapter or merged weights from a LLaMA-Factory or Axolotl run, recorded on
/// the same `local_slm` proposal `import_prepared` writes. Does not apply.
pub struct ImportTrainedRequest<'a> {
    pub estate: &'a Estate,
    pub prepared_dir: &'a Path,
    pub tag: &'a str,
    pub adapter: &'a Path,
    pub curator: &'a str,
}

pub fn import_trained(req: &ImportTrainedRequest<'_>) -> Result<EnrichBindingProposal, ModelError> {
    refuse_curator(req.curator, &req.estate.enrich_packs.curator).map_err(map_feed)?;
    refuse_sacred_and_sku("adapter", &req.adapter.display().to_string())?;
    let doc = load_prepare_doc(&req.prepared_dir.join("prepare.json"))?;
    if !TRAIN_RECIPE_DRIVERS.contains(&doc.driver.as_str()) {
        return Err(ModelError::Other(format!(
            "refuse:driver: import-trained reads a train recipe ({}), found '{}'",
            TRAIN_RECIPE_DRIVERS.join(", "),
            doc.driver
        )));
    }
    if doc.job != EnrichJobKind::Train.as_str() {
        return Err(ModelError::Other(format!(
            "refuse:job: import-trained expects job train, found '{}'",
            doc.job
        )));
    }
    let weights = resolve_adapter_artifact(req.adapter)?;
    import_prepared(&ImportPreparedRequest {
        estate: req.estate,
        prepared_dir: req.prepared_dir,
        tag: req.tag,
        path: &weights,
        curator: req.curator,
    })
}

fn resolve_adapter_artifact(path: &Path) -> Result<PathBuf, ModelError> {
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(ModelError::Other(format!(
                "refuse:adapter: {} is missing",
                path.display()
            )));
        }
        Err(err) => {
            return Err(ModelError::Other(format!(
                "refuse:adapter: {}: {err}",
                path.display()
            )));
        }
    };
    if meta.is_file() {
        if is_adapter_file(path) {
            return Ok(path.to_path_buf());
        }
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} is not a gguf, safetensors, or adapter config",
            path.display()
        )));
    }
    if meta.is_dir() {
        for name in [
            "adapter_config.json",
            "adapter_model.safetensors",
            "adapter_model.bin",
        ] {
            let candidate = path.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
        if let Some(gguf) = first_dir_file_with_suffix(path, ".gguf")? {
            return Ok(gguf);
        }
        let config = path.join("config.json");
        if config.is_file() && first_dir_file_with_suffix(path, ".safetensors")?.is_some() {
            return Ok(config);
        }
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} has no adapter_config.json, adapter weights, or gguf",
            path.display()
        )));
    }
    Err(ModelError::Other(format!(
        "refuse:adapter: {} is not a file or directory",
        path.display()
    )))
}

fn is_adapter_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".gguf")
        || lower.ends_with(".safetensors")
        || lower == "adapter_config.json"
        || lower == "adapter_model.bin"
}

fn first_dir_file_with_suffix(dir: &Path, suffix: &str) -> Result<Option<PathBuf>, ModelError> {
    let mut matches = Vec::new();
    let entries = std::fs::read_dir(dir)
        .map_err(|err| ModelError::Other(format!("refuse:adapter: {}: {err}", dir.display())))?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            ModelError::Other(format!("refuse:adapter: {}: {err}", dir.display()))
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if name.ends_with(suffix) {
            matches.push(path);
        }
    }
    matches.sort();
    Ok(matches.into_iter().next())
}

fn load_prepare_doc(path: &Path) -> Result<EnrichPrepareDoc, ModelError> {
    if !path.is_file() {
        return Err(ModelError::Other(format!(
            "refuse:missing-prepare: {}",
            path.display()
        )));
    }
    let text = std::fs::read_to_string(path).map_err(|err| {
        ModelError::Other(format!(
            "refuse:prepare-unreadable: {}: {err}",
            path.display()
        ))
    })?;
    refuse_raw_secrets(&text).map_err(map_feed)?;
    let doc: EnrichPrepareDoc = serde_json::from_str(&text).map_err(|err| {
        ModelError::Other(format!(
            "refuse:prepare-unreadable: {}: {err}",
            path.display()
        ))
    })?;
    if doc.schema != PREPARE_SCHEMA {
        return Err(ModelError::Other(format!(
            "refuse:prepare: schema '{}' is not {PREPARE_SCHEMA}",
            doc.schema
        )));
    }
    parse_enrich_job(&doc.job)?;
    if doc.promoted || doc.auto_apply || doc.estate_rewritten {
        return Err(ModelError::Other(format!(
            "refuse:prepared: {} promoted, auto_apply, and estate_rewritten must stay false",
            path.display()
        )));
    }
    if !estate_schema::is_slug(&doc.pack_id) {
        return Err(ModelError::Other(format!(
            "refuse:prepare: pack id '{}' must be a slug",
            doc.pack_id
        )));
    }
    refuse_sacred_and_sku("pack id", &doc.pack_id)?;
    refuse_sacred_and_sku("driver", &doc.driver)?;
    refuse_sacred_and_sku("base model", &doc.base_model)?;
    if let Some(seat) = doc.seat_tag.as_deref() {
        refuse_sacred_and_sku("seat tag", seat)?;
        if seat != doc.base_model {
            return Err(ModelError::Other(format!(
                "refuse:prepare: seat_tag '{seat}' does not match base_model '{}'",
                doc.base_model
            )));
        }
    }
    if let Some(train_base) = doc.train_base_model.as_deref() {
        refuse_sacred_and_sku("train base", train_base)?;
    }
    refuse_sacred_and_sku("purpose", &doc.purpose)?;
    refuse_sacred_and_sku("host_class_affinity", &doc.host_class_affinity)?;
    for source in &doc.source_paths {
        refuse_sacred_and_sku("source path", source)?;
    }
    for source in &doc.source_drivers {
        refuse_sacred_and_sku("source driver", source)?;
    }
    resolve_train_enrich_driver(&doc.driver)?;
    Ok(doc)
}

fn dir_children(dir: &Path) -> Result<Vec<PathBuf>, ModelError> {
    let mut paths = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|err| {
        ModelError::Other(format!("refuse:enrich-index: {}: {err}", dir.display()))
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            ModelError::Other(format!("refuse:enrich-index: {}: {err}", dir.display()))
        })?;
        paths.push(entry.path());
    }
    paths.sort();
    Ok(paths)
}

fn file_name(path: &Path) -> Result<String, ModelError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .ok_or_else(|| {
            ModelError::Other(format!(
                "refuse:enrich-index: {} has no utf-8 name",
                path.display()
            ))
        })
}

fn scan_operator_file(path: &Path) -> Result<bool, ModelError> {
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(ModelError::Other(format!(
                "refuse:path: {} is not a file",
                path.display()
            )));
        }
        Err(err) => {
            return Err(ModelError::Other(format!(
                "refuse:path: {}: {err}",
                path.display()
            )));
        }
    };
    if !meta.is_file() {
        return Err(ModelError::Other(format!(
            "refuse:path: {} is not a file",
            path.display()
        )));
    }
    const CAP: u64 = 1024 * 1024;
    if meta.len() > CAP {
        return Ok(false);
    }
    let bytes = std::fs::read(path)
        .map_err(|err| ModelError::Other(format!("refuse:path: {}: {err}", path.display())))?;
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Ok(false);
    };
    refuse_sacred_and_sku("operator file", text)?;
    refuse_raw_secrets(text).map_err(map_feed)?;
    Ok(true)
}

fn binding_paste_yaml(binding: &serde_json::Value) -> Result<String, ModelError> {
    let obj = binding.as_object().ok_or_else(|| {
        ModelError::Other("refuse:binding: proposed binding is not an object".into())
    })?;
    let driver = obj
        .get("driver")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ModelError::Other("refuse:binding: seated driver missing".into()))?;
    let wired = obj
        .get("wired")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| ModelError::Other("refuse:binding: wired must be a bool".into()))?;
    let params = obj
        .get("params")
        .ok_or_else(|| ModelError::Other("refuse:binding: params missing".into()))?;
    let params = params.as_object().ok_or_else(|| {
        ModelError::Other("refuse:binding: local_slm params must be an object".into())
    })?;
    let mut out = String::from(
        "# Replace the existing local_slm binding. The id stays local_slm. This file is not applied.\n",
    );
    out.push_str("- id: local_slm\n");
    out.push_str("  class: local\n");
    out.push_str(&format!("  driver: {}\n", yaml_quote(driver)));
    out.push_str(&format!(
        "  wired: {}\n",
        if wired { "true" } else { "false" }
    ));
    out.push_str("  params:\n");
    for (key, value) in params {
        out.push_str(&format!("    {key}: {}\n", yaml_scalar(value)?));
    }
    Ok(out)
}

fn yaml_scalar(value: &serde_json::Value) -> Result<String, ModelError> {
    match value {
        serde_json::Value::String(text) => Ok(yaml_quote(text)),
        serde_json::Value::Bool(flag) => Ok(if *flag { "true" } else { "false" }.to_string()),
        serde_json::Value::Number(number) => Ok(number.to_string()),
        serde_json::Value::Null => Ok("null".to_string()),
        _ => Err(ModelError::Other(
            "refuse:binding: param value is not a scalar".into(),
        )),
    }
}

fn render_binding_proposal(proposal: &EnrichBindingProposal) -> String {
    format!(
        "# Binding proposal (local_slm)\n\
         \n\
         schema: {schema}\n\
         auto_apply: false\n\
         promoted: false\n\
         estate_rewritten: false\n\
         estate: {name} ({hash})\n\
         pack: {pack}\n\
         driver: {driver}\n\
         job: {job}\n\
         local tag: {tag}\n\
         operator path: {path}\n\
         prepared: {prepared}\n\
         seated driver: {seated}\n\
         content_scanned: {scanned}\n\
         \n\
         This file is not applied. It does not rewrite estate.yaml and it does not write catalog.json.\n\
         Agents that already allow local_slm keep that id.\n\
         Next: estate enrich apply-proposal stages this binding for estate plan and estate apply --require-plan.\n\
         The source estate is written only when that apply succeeds. import-prepared does not run plan or apply.\n\
         \n\
         ```yaml\n\
         {paste}\
         ```\n\
         \n\
         {note}\n",
        schema = proposal.schema,
        name = proposal.estate_name,
        hash = proposal.estate_hash,
        pack = proposal.pack_id,
        driver = proposal.driver,
        job = proposal.job,
        tag = proposal.local_tag,
        path = proposal.local_path,
        prepared = proposal.prepared_dir,
        seated = proposal.seated_driver,
        scanned = proposal.content_scanned,
        paste = proposal.paste_yaml,
        note = proposal.note,
    )
}

fn read_pack_file(path: &Path) -> Result<PackManifest, ModelError> {
    let text = std::fs::read_to_string(path)
        .map_err(|_| ModelError::Other(format!("refuse:missing-pack: {}", path.display())))?;
    serde_json::from_str(&text)
        .map_err(|err| ModelError::Other(format!("refuse:pack: {} ({err})", path.display())))
}

fn map_feed(err: FeedError) -> ModelError {
    let text = err.to_string();
    if text.starts_with("refuse:") {
        ModelError::Other(text)
    } else {
        ModelError::Other(format!("refuse:pack: {text}"))
    }
}

fn to_pretty(value: &impl Serialize) -> Result<String, ModelError> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|err| ModelError::Other(format!("refuse:prepare: serialize: {err}")))?;
    if body.trim().is_empty() || body.trim() == "{}" {
        return Err(ModelError::Other("refuse:prepare: serialize: empty".into()));
    }
    Ok(format!("{body}\n"))
}

fn yaml_quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn yaml_list(name: &str, values: &[String]) -> String {
    if values.is_empty() {
        return format!("{name}: []\n");
    }
    let mut out = format!("{name}:\n");
    for value in values {
        out.push_str(&format!("  - {}\n", yaml_quote(value)));
    }
    out
}

fn external_manifest_yaml(manifest: &ExternalManifest) -> String {
    format!(
        "schema: {schema}\n\
         driver: {driver}\n\
         job: {job}\n\
         pack_id: {pack}\n\
         base_model: {base}\n\
         purpose: {purpose}\n\
         host_class_affinity: {host}\n\
         {datasets}\
         {drivers}\
         vendor: null\n\
         promoted: false\n\
         auto_apply: false\n\
         estate_rewritten: false\n\
         note: {note}\n",
        schema = yaml_quote(&manifest.schema),
        driver = yaml_quote(manifest.driver),
        job = yaml_quote(&manifest.job),
        pack = yaml_quote(&manifest.pack_id),
        base = yaml_quote(&manifest.base_model),
        purpose = yaml_quote(&manifest.purpose),
        host = yaml_quote(&manifest.host_class_affinity),
        datasets = yaml_list("dataset_paths", &manifest.dataset_paths),
        drivers = yaml_list("source_drivers", &manifest.source_drivers),
        note = yaml_quote(manifest.note),
    )
}

pub const BINDING_STAGE_SCHEMA: &str = "cell-one.enrich-binding-stage.v0";

const STAGED_ESTATE_FILE: &str = "staged-estate.yaml";
const STAGE_JSON: &str = "stage.json";

/// `{state_dir}/enrich-stage`. Plan and apply read `staged-estate.yaml` from here.
pub fn enrich_stage_dir(state_dir: &Path) -> PathBuf {
    state_dir.join("enrich-stage")
}

fn staged_estate_path(state_dir: &Path) -> PathBuf {
    enrich_stage_dir(state_dir).join(STAGED_ESTATE_FILE)
}

fn stage_json_path(state_dir: &Path) -> PathBuf {
    enrich_stage_dir(state_dir).join(STAGE_JSON)
}

/// `local_slm` `params.model` when that string is present.
pub fn local_slm_model_param(estate: &Estate) -> Option<String> {
    estate
        .model_bindings
        .iter()
        .find(|binding| binding.id == "local_slm")
        .and_then(|binding| binding.params.get("model"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(str::to_string)
}

/// One prepared driver under `.cell/enrich`, with a proposal when that file exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrichJoinFact {
    pub kind: String,
    pub pack_id: String,
    pub driver: String,
    pub local_tag: String,
}

/// Read-only. `Ok(None)` when `enrich/` is missing. A bad proposal or prepare refuses.
pub fn enrich_join_facts(enrich_root: &Path) -> Result<Option<Vec<EnrichJoinFact>>, ModelError> {
    if !enrich_root.exists() {
        return Ok(None);
    }
    let rows = list_prepared(enrich_root)?;
    let mut facts = Vec::new();
    for row in rows {
        let proposal_path = row.out_dir.join(BINDING_PROPOSAL_JSON);
        if proposal_path.is_file() {
            let proposal = parse_binding_proposal(&proposal_path)?;
            if proposal.pack_id != row.pack_id
                || proposal.driver != row.driver
                || proposal.job != row.job
            {
                return Err(ModelError::Other(format!(
                    "refuse:prepare: {} does not match prepare.json",
                    proposal_path.display()
                )));
            }
            if proposal.local_tag != row.local_tag {
                return Err(ModelError::Other(format!(
                    "refuse:tag: '{}' must be {}",
                    proposal.local_tag, row.local_tag
                )));
            }
            facts.push(EnrichJoinFact {
                kind: "proposal".into(),
                pack_id: row.pack_id,
                driver: row.driver,
                local_tag: row.local_tag,
            });
        } else {
            facts.push(EnrichJoinFact {
                kind: "prepare".into(),
                pack_id: row.pack_id,
                driver: row.driver,
                local_tag: row.local_tag,
            });
        }
    }
    Ok(Some(facts))
}

/// Receipt for a staged `local_slm` join. `applied` flips only after
/// `estate apply --require-plan` writes the source estate.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EnrichBindingStage {
    pub schema: String,
    pub curator: String,
    pub policy: String,
    pub auto_apply: bool,
    pub promoted: bool,
    pub estate_rewritten: bool,
    pub applied: bool,
    pub source_estate: String,
    pub staged_estate: String,
    pub prepared_dir: String,
    pub pack_id: String,
    pub driver: String,
    pub job: String,
    pub local_tag: String,
    pub binding_id: String,
    pub seated_driver: String,
    pub source_estate_hash: String,
    pub staged_estate_hash: String,
    pub binding_fingerprint: String,
    pub note: String,
}

pub struct ApplyProposalRequest<'a> {
    pub estate: &'a Estate,
    pub estate_path: &'a Path,
    pub prepared_dir: &'a Path,
    pub tag: &'a str,
    pub curator: &'a str,
    pub state_dir: &'a Path,
    /// Set only for `--verify-local-tag`. Absent skips the network.
    pub verify_endpoint: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyProposalOutcome {
    Staged(EnrichBindingStage),
    Noop { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnrichStageCommit {
    Absent,
    NotThisEstate,
    Held,
    Wrote { source: PathBuf },
    Already { source: PathBuf },
}

/// Validate `binding-proposal.json` and write a staged estate that `estate plan`
/// and `estate apply --require-plan` already consume. Does not apply. Does not
/// write the source estate.
pub fn apply_proposal(req: &ApplyProposalRequest<'_>) -> Result<ApplyProposalOutcome, ModelError> {
    if points_at_stage_file(req.estate_path, req.state_dir) {
        return Err(ModelError::Other(
            "refuse:stage: --estate must be the source estate, not enrich-stage/staged-estate.yaml"
                .into(),
        ));
    }
    refuse_curator(req.curator, &req.estate.enrich_packs.curator).map_err(map_feed)?;
    refuse_sacred_and_sku("prepared dir", &req.prepared_dir.display().to_string())?;
    let proposal_path = req.prepared_dir.join(BINDING_PROPOSAL_JSON);
    let proposal = parse_binding_proposal(&proposal_path)?;
    let doc = load_prepare_doc(&req.prepared_dir.join("prepare.json"))?;
    refuse_recipe_train_record(&doc, req.prepared_dir)?;
    if doc.pack_id != proposal.pack_id || doc.driver != proposal.driver || doc.job != proposal.job {
        return Err(ModelError::Other(
            "refuse:prepare: prepare.json does not match the binding proposal".into(),
        ));
    }
    refuse_frontier_source_on_estate(&doc.source_drivers, req.estate).map_err(map_feed)?;
    let expected = local_enrich_tag(&doc.pack_id);
    if req.tag != expected || proposal.local_tag != expected {
        return Err(ModelError::Other(format!(
            "refuse:tag: '{tag}' must be {expected}",
            tag = req.tag
        )));
    }
    if !same_file(Path::new(&proposal.prepared_dir), req.prepared_dir) {
        return Err(ModelError::Other(
            "refuse:prepared: binding proposal prepared_dir does not match --prepared".into(),
        ));
    }
    let operator_path = Path::new(&proposal.local_path);
    refuse_sacred_and_sku("operator path", &proposal.local_path)?;
    let _scanned = scan_operator_file(operator_path)?;
    let seat = req
        .estate
        .model_bindings
        .iter()
        .find(|binding| binding.id == "local_slm")
        .ok_or_else(|| ModelError::Other("refuse:binding: estate has no local_slm seat".into()))?;
    let proposed = proposed_model_binding(seat, &proposal, req.tag)?;
    let mut staged_estate = req.estate.clone();
    let index = staged_estate
        .model_bindings
        .iter()
        .position(|binding| binding.id == "local_slm")
        .ok_or_else(|| ModelError::Other("refuse:binding: estate has no local_slm seat".into()))?;
    staged_estate.model_bindings[index] = proposed.clone();
    estate_schema::validate(&staged_estate).map_err(|errors| {
        ModelError::Other(format!(
            "refuse:binding: staged estate invalid: {}",
            errors.join("; ")
        ))
    })?;
    if let Some(endpoint) = req.verify_endpoint {
        crate::runtime_lists_model(endpoint, req.tag).map_err(ModelError::Other)?;
    }
    if seat == &proposed {
        return Ok(ApplyProposalOutcome::Noop {
            reason: format!("no-op: local_slm already bound to {}", req.tag),
        });
    }
    let current_hash = estate_schema::estate_hash(req.estate);
    let fingerprint = binding_fingerprint(&proposed)?;
    if let Some(stage) = load_stage(req.state_dir)? {
        let same_intent = stage.local_tag == req.tag
            && stage.binding_fingerprint == fingerprint
            && stage.source_estate_hash == current_hash
            && stage.binding_id == "local_slm"
            && !stage.auto_apply
            && !stage.promoted;
        if same_intent && staged_file_matches(&stage)? {
            return Ok(ApplyProposalOutcome::Noop {
                reason: format!(
                    "no-op: enrich stage already holds tag={} binding=local_slm",
                    req.tag
                ),
            });
        }
        if !stage.applied {
            return Err(ModelError::Other(format!(
                "refuse:stage: pending stage tag={} hash={} does not match this proposal",
                stage.local_tag, stage.source_estate_hash
            )));
        }
    }
    if proposal.estate_hash != current_hash {
        return Err(ModelError::Other(format!(
            "refuse:estate-hash: binding proposal {} does not match estate {current_hash}",
            proposal.estate_hash
        )));
    }
    let yaml = estate_schema::render_estate_yaml(&staged_estate)
        .map_err(|err| ModelError::Other(format!("refuse:binding: staged estate: {err}")))?;
    let parsed = estate_schema::load_estate_str(&yaml)
        .map_err(|err| ModelError::Other(format!("refuse:binding: staged estate: {err}")))?;
    let staged_hash = estate_schema::estate_hash(&parsed);
    let source_estate = canonical_string(req.estate_path)?;
    let stage_dir = enrich_stage_dir(req.state_dir);
    std::fs::create_dir_all(&stage_dir).map_err(|err| {
        ModelError::Other(format!(
            "refuse:stage-write: {}: {err}",
            stage_dir.display()
        ))
    })?;
    let stage_dir = canonical_path(&stage_dir)?;
    let staged_estate_path = stage_dir.join(STAGED_ESTATE_FILE);
    if Path::new(&source_estate) == staged_estate_path {
        return Err(ModelError::Other(
            "refuse:stage: --estate must be the source estate, not enrich-stage/staged-estate.yaml"
                .into(),
        ));
    }
    let stage = EnrichBindingStage {
        schema: BINDING_STAGE_SCHEMA.into(),
        curator: "jason".into(),
        policy: "manual".into(),
        auto_apply: false,
        promoted: false,
        estate_rewritten: false,
        applied: false,
        source_estate,
        staged_estate: staged_estate_path.display().to_string(),
        prepared_dir: canonical_string(req.prepared_dir)?,
        pack_id: proposal.pack_id,
        driver: proposal.driver,
        job: proposal.job,
        local_tag: expected,
        binding_id: "local_slm".into(),
        seated_driver: proposal.seated_driver,
        source_estate_hash: current_hash,
        staged_estate_hash: staged_hash,
        binding_fingerprint: fingerprint,
        note: "Stage only. auto_apply=false. estate plan the staged estate, then estate apply --require-plan. The source estate is written only when that apply succeeds. apply-proposal does not apply.".into(),
    };
    let json = to_pretty(&stage)?;
    refuse_sacred_and_sku("binding stage", &json)?;
    refuse_raw_secrets(&json).map_err(map_feed)?;
    let staged_tmp = stage_dir.join("staged-estate.yaml.tmp");
    let json_tmp = stage_dir.join("stage.json.tmp");
    std::fs::write(&staged_tmp, &yaml).map_err(|err| {
        ModelError::Other(format!(
            "refuse:stage-write: {}: {err}",
            staged_tmp.display()
        ))
    })?;
    std::fs::write(&json_tmp, &json).map_err(|err| {
        ModelError::Other(format!("refuse:stage-write: {}: {err}", json_tmp.display()))
    })?;
    std::fs::rename(&staged_tmp, &staged_estate_path).map_err(|err| {
        ModelError::Other(format!(
            "refuse:stage-write: {}: {err}",
            staged_estate_path.display()
        ))
    })?;
    std::fs::rename(&json_tmp, stage_dir.join(STAGE_JSON))
        .map_err(|err| ModelError::Other(format!("refuse:stage-write: stage.json: {err}")))?;
    Ok(ApplyProposalOutcome::Staged(stage))
}

/// Refuse a require-plan apply of the staged estate when the receipt does not
/// cover that hash. Other estate paths are ignored. Does not write.
pub fn refuse_staged_apply(
    applied_path: &Path,
    state_dir: &Path,
    applied_hash: &str,
    require_plan: bool,
) -> Result<(), ModelError> {
    if !same_file(applied_path, &staged_estate_path(state_dir)) {
        return Ok(());
    }
    let Some(stage) = load_stage(state_dir)? else {
        return Ok(());
    };
    if require_plan && stage.staged_estate_hash != applied_hash {
        return Err(ModelError::Other(format!(
            "refuse:stage: applied hash {applied_hash} does not match enrich stage {}",
            stage.staged_estate_hash
        )));
    }
    Ok(())
}

/// After a successful apply of the staged estate, copy it onto the source
/// estate only when `--require-plan` was set.
pub fn commit_enrich_stage(
    applied_path: &Path,
    state_dir: &Path,
    require_plan: bool,
    applied_hash: &str,
) -> Result<EnrichStageCommit, ModelError> {
    let staged_path = staged_estate_path(state_dir);
    if !same_file(applied_path, &staged_path) {
        return Ok(EnrichStageCommit::NotThisEstate);
    }
    let Some(mut stage) = load_stage(state_dir)? else {
        return Ok(EnrichStageCommit::Absent);
    };
    if !require_plan {
        return Ok(EnrichStageCommit::Held);
    }
    if stage.staged_estate_hash != applied_hash {
        return Err(ModelError::Other(format!(
            "refuse:stage: applied hash {applied_hash} does not match enrich stage {}",
            stage.staged_estate_hash
        )));
    }
    let bytes = std::fs::read(&staged_path).map_err(|err| {
        ModelError::Other(format!("refuse:stage: {}: {err}", staged_path.display()))
    })?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| ModelError::Other("refuse:stage: staged estate is not utf-8".into()))?;
    let parsed = estate_schema::load_estate_str(text)
        .map_err(|err| ModelError::Other(format!("refuse:stage: staged estate: {err}")))?;
    if estate_schema::estate_hash(&parsed) != applied_hash {
        return Err(ModelError::Other(
            "refuse:stage: staged estate hash changed before write-back".into(),
        ));
    }
    let source = PathBuf::from(&stage.source_estate);
    if same_file(&source, &staged_path) {
        return Err(ModelError::Other(
            "refuse:stage: source estate is the stage file".into(),
        ));
    }
    let already = std::fs::read(&source).ok().as_deref() == Some(bytes.as_slice());
    if !already {
        write_bytes_atomic(&source, &bytes)?;
    }
    if !stage.applied || !stage.estate_rewritten {
        stage.applied = true;
        stage.estate_rewritten = true;
        stage.auto_apply = false;
        stage.promoted = false;
        let json = to_pretty(&stage)?;
        std::fs::write(stage_json_path(state_dir), json)
            .map_err(|err| ModelError::Other(format!("refuse:stage-write: stage.json: {err}")))?;
    }
    if already {
        Ok(EnrichStageCommit::Already { source })
    } else {
        Ok(EnrichStageCommit::Wrote { source })
    }
}

fn parse_binding_proposal(path: &Path) -> Result<EnrichBindingProposal, ModelError> {
    if !path.is_file() {
        return Err(ModelError::Other(format!(
            "refuse:missing-proposal: {}",
            path.display()
        )));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|err| ModelError::Other(format!("refuse:proposal: {}: {err}", path.display())))?;
    refuse_raw_secrets(&text).map_err(map_feed)?;
    let proposal: EnrichBindingProposal = serde_json::from_str(&text)
        .map_err(|err| ModelError::Other(format!("refuse:proposal: {}: {err}", path.display())))?;
    if proposal.schema != BINDING_PROPOSAL_SCHEMA {
        return Err(ModelError::Other(format!(
            "refuse:proposal: schema '{}' is not {BINDING_PROPOSAL_SCHEMA}",
            proposal.schema
        )));
    }
    if proposal.auto_apply || proposal.promoted || proposal.estate_rewritten {
        return Err(ModelError::Other(
            "refuse:proposal: auto_apply, promoted, and estate_rewritten must stay false".into(),
        ));
    }
    if proposal.curator != "jason" || proposal.policy != "manual" {
        return Err(ModelError::Other(
            "refuse:curator: binding proposal curator must be jason and policy manual".into(),
        ));
    }
    if proposal.binding_id != "local_slm" {
        return Err(ModelError::Other(
            "refuse:binding: binding proposal id is not local_slm".into(),
        ));
    }
    refuse_sacred_and_sku("tag", &proposal.local_tag)?;
    refuse_sacred_and_sku("pack id", &proposal.pack_id)?;
    refuse_sacred_and_sku("driver", &proposal.driver)?;
    refuse_sacred_and_sku("seated driver", &proposal.seated_driver)?;
    refuse_sacred_and_sku("operator path", &proposal.local_path)?;
    refuse_sacred_and_sku("binding proposal", &proposal.paste_yaml)?;
    refuse_sacred_and_sku("binding proposal", &proposal.note)?;
    Ok(proposal)
}

fn proposed_model_binding(
    seat: &estate_schema::ModelBinding,
    proposal: &EnrichBindingProposal,
    tag: &str,
) -> Result<estate_schema::ModelBinding, ModelError> {
    let obj = proposal.proposed_binding.as_object().ok_or_else(|| {
        ModelError::Other("refuse:binding: proposed binding is not an object".into())
    })?;
    let id = obj.get("id").and_then(|value| value.as_str()).unwrap_or("");
    if id != "local_slm" || seat.id != "local_slm" {
        return Err(ModelError::Other(
            "refuse:binding: proposed id must stay local_slm".into(),
        ));
    }
    let class = obj
        .get("class")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if class == "frontier" {
        return Err(ModelError::Other(
            "refuse:frontier-invent: binding proposal class is frontier".into(),
        ));
    }
    if class != "local" || seat.class != estate_schema::ModelClass::Local {
        return Err(ModelError::Other(
            "refuse:binding: local_slm class is not local".into(),
        ));
    }
    let driver = obj
        .get("driver")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if driver != seat.driver || proposal.seated_driver != seat.driver || driver.trim().is_empty() {
        return Err(ModelError::Other(
            "refuse:binding: proposed driver must stay the seated local_slm driver".into(),
        ));
    }
    refuse_sacred_and_sku("seated driver", driver)?;
    let wired = obj
        .get("wired")
        .and_then(|value| value.as_bool())
        .ok_or_else(|| ModelError::Other("refuse:binding: wired must be a bool".into()))?;
    if wired != seat.wired {
        return Err(ModelError::Other(
            "refuse:binding: proposed wired does not match the seat".into(),
        ));
    }
    let params = obj
        .get("params")
        .and_then(|value| value.as_object())
        .ok_or_else(|| {
            ModelError::Other("refuse:binding: local_slm params must be an object".into())
        })?;
    let seat_params = match &seat.params {
        serde_json::Value::Object(map) => map.clone(),
        serde_json::Value::Null => serde_json::Map::new(),
        _ => {
            return Err(ModelError::Other(
                "refuse:binding: local_slm params must be an object".into(),
            ))
        }
    };
    const MANAGED: &[&str] = &["model", "prepared_pack", "prepared_driver"];
    for key in seat_params.keys() {
        if MANAGED.contains(&key.as_str()) {
            continue;
        }
        match params.get(key) {
            Some(value) if value == &seat_params[key] => {}
            Some(_) => {
                return Err(ModelError::Other(format!(
                    "refuse:binding: params.{key} does not match the seat"
                )))
            }
            None => {
                return Err(ModelError::Other(format!(
                    "refuse:binding: params.{key} missing from the proposal"
                )))
            }
        }
    }
    for key in params.keys() {
        if MANAGED.contains(&key.as_str()) || seat_params.contains_key(key) {
            continue;
        }
        return Err(ModelError::Other(format!(
            "refuse:binding: params.{key} is not on the seat"
        )));
    }
    let model = params
        .get("model")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if model != tag || model != proposal.local_tag {
        return Err(ModelError::Other(format!(
            "refuse:binding: params.model '{model}' must be {tag}"
        )));
    }
    let prepared_pack = params
        .get("prepared_pack")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if prepared_pack != proposal.pack_id {
        return Err(ModelError::Other(
            "refuse:binding: params.prepared_pack does not match the pack".into(),
        ));
    }
    let prepared_driver = params
        .get("prepared_driver")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if prepared_driver != proposal.driver {
        return Err(ModelError::Other(
            "refuse:binding: params.prepared_driver does not match the driver".into(),
        ));
    }
    for (key, value) in params {
        if let Some(text) = value.as_str() {
            refuse_sacred_and_sku(&format!("params.{key}"), text)?;
        }
    }
    Ok(estate_schema::ModelBinding {
        id: "local_slm".into(),
        class: estate_schema::ModelClass::Local,
        driver: seat.driver.clone(),
        params: serde_json::Value::Object(params.clone()),
        wired: seat.wired,
    })
}

fn binding_fingerprint(binding: &estate_schema::ModelBinding) -> Result<String, ModelError> {
    serde_json::to_string(binding)
        .map_err(|err| ModelError::Other(format!("refuse:binding: fingerprint: {err}")))
}

/// Read `{state}/enrich-stage/stage.json`. Missing file is `Ok(None)`.
pub fn read_enrich_stage(state_dir: &Path) -> Result<Option<EnrichBindingStage>, ModelError> {
    load_stage(state_dir)
}

fn load_stage(state_dir: &Path) -> Result<Option<EnrichBindingStage>, ModelError> {
    let path = stage_json_path(state_dir);
    if !path.exists() {
        return Ok(None);
    }
    if !path.is_file() {
        return Err(ModelError::Other(format!(
            "refuse:stage: {} is not a file",
            path.display()
        )));
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|err| ModelError::Other(format!("refuse:stage: {}: {err}", path.display())))?;
    refuse_raw_secrets(&text).map_err(map_feed)?;
    let stage: EnrichBindingStage = serde_json::from_str(&text)
        .map_err(|err| ModelError::Other(format!("refuse:stage: {}: {err}", path.display())))?;
    if stage.schema != BINDING_STAGE_SCHEMA {
        return Err(ModelError::Other(format!(
            "refuse:stage: schema '{}' is not {BINDING_STAGE_SCHEMA}",
            stage.schema
        )));
    }
    if stage.auto_apply || stage.promoted {
        return Err(ModelError::Other(
            "refuse:stage: auto_apply and promoted must stay false".into(),
        ));
    }
    if stage.binding_id != "local_slm" {
        return Err(ModelError::Other(
            "refuse:binding: stage binding_id is not local_slm".into(),
        ));
    }
    refuse_sacred_and_sku("tag", &stage.local_tag)?;
    Ok(Some(stage))
}

fn staged_file_matches(stage: &EnrichBindingStage) -> Result<bool, ModelError> {
    let path = Path::new(&stage.staged_estate);
    if !path.is_file() {
        return Ok(false);
    }
    let text = std::fs::read_to_string(path)
        .map_err(|err| ModelError::Other(format!("refuse:stage: {}: {err}", path.display())))?;
    let estate = estate_schema::load_estate_str(&text)
        .map_err(|err| ModelError::Other(format!("refuse:stage: {}: {err}", path.display())))?;
    Ok(estate_schema::estate_hash(&estate) == stage.staged_estate_hash)
}

fn points_at_stage_file(path: &Path, state_dir: &Path) -> bool {
    let expected = staged_estate_path(state_dir);
    if same_file(path, &expected) {
        return true;
    }
    path.file_name() == Some(std::ffi::OsStr::new(STAGED_ESTATE_FILE))
        && path.parent().and_then(|parent| parent.file_name())
            == Some(std::ffi::OsStr::new("enrich-stage"))
}

fn same_file(left: &Path, right: &Path) -> bool {
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn canonical_path(path: &Path) -> Result<PathBuf, ModelError> {
    std::fs::canonicalize(path)
        .map_err(|err| ModelError::Other(format!("refuse:path: {}: {err}", path.display())))
}

fn canonical_string(path: &Path) -> Result<String, ModelError> {
    Ok(canonical_path(path)?.display().to_string())
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), ModelError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|err| {
                ModelError::Other(format!("refuse:stage-write: {}: {err}", parent.display()))
            })?;
        }
    }
    let tmp = path.with_extension("yaml.tmp");
    std::fs::write(&tmp, bytes).map_err(|err| {
        ModelError::Other(format!("refuse:stage-write: {}: {err}", tmp.display()))
    })?;
    std::fs::rename(&tmp, path).map_err(|err| {
        let _ = std::fs::remove_file(&tmp);
        ModelError::Other(format!("refuse:stage-write: {}: {err}", path.display()))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use estate_schema::ModelClass;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn tmp(name: &str) -> PathBuf {
        // Keep the throwaway dir off the checkout. A GPU token in the
        // checkout path would trip refuse:sku-banned on the out directory.
        let path = std::env::temp_dir().join(format!(
            "cell-one-enrich-unit-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn fixture_pack() -> PackManifest {
        let path = repo_root().join("examples/fixtures/specialist-overnight.pack.json");
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn fixture_estate() -> Estate {
        estate_schema::load_estate_unvalidated(&repo_root().join("examples/estate.yaml")).unwrap()
    }

    fn seated_estate(model: &str) -> Estate {
        let mut estate = fixture_estate();
        let binding = estate
            .model_bindings
            .iter_mut()
            .find(|binding| binding.id == "local_slm")
            .unwrap();
        binding
            .params
            .as_object_mut()
            .unwrap()
            .insert("model".into(), serde_json::Value::String(model.into()));
        estate
    }

    fn with_train_base(mut estate: Estate, train_base: &str) -> Estate {
        let binding = estate
            .model_bindings
            .iter_mut()
            .find(|binding| binding.id == "local_slm")
            .unwrap();
        binding.params.as_object_mut().unwrap().insert(
            "train_base_model".into(),
            serde_json::Value::String(train_base.into()),
        );
        estate
    }

    fn run(
        driver: &str,
        pack: &PackManifest,
        estate: &Estate,
        out: &Path,
        job: &str,
        curator: &str,
    ) -> Result<EnrichPrepareDoc, ModelError> {
        run_max(driver, pack, estate, out, job, curator, None)
    }

    fn run_max(
        driver: &str,
        pack: &PackManifest,
        estate: &Estate,
        out: &Path,
        job: &str,
        curator: &str,
        max_steps: Option<u32>,
    ) -> Result<EnrichPrepareDoc, ModelError> {
        prepare_enrich(&PrepareEnrichRequest {
            estate,
            pack,
            curator,
            driver_id: driver,
            job,
            out_dir: out,
            max_steps,
            from_feed: false,
            state_dir: Path::new(".cell"),
        })
    }

    fn run_feed(
        driver: &str,
        pack: &PackManifest,
        estate: &Estate,
        out: &Path,
        state_dir: &Path,
        from_feed: bool,
    ) -> Result<EnrichPrepareDoc, ModelError> {
        prepare_enrich(&PrepareEnrichRequest {
            estate,
            pack,
            curator: "jason",
            driver_id: driver,
            job: "train",
            out_dir: out,
            max_steps: None,
            from_feed,
            state_dir,
        })
    }

    fn write_fixture_feed(state_dir: &Path) {
        let feed = state_dir.join("feed");
        feed_collector::append_event(
            &feed,
            &feed_collector::ScrubbedEvent {
                kind: "model.local.precheck".into(),
                agent_id: Some("research".into()),
                decision: Some("allow".into()),
                object_class: Some("local".into()),
                note: Some("job=policy-precheck".into()),
                ts: "2026-09-21T00:00:00Z".into(),
            },
        )
        .unwrap();
        feed_collector::append_event(
            &feed,
            &feed_collector::ScrubbedEvent {
                kind: "model.local.skip".into(),
                agent_id: Some("research".into()),
                decision: Some("allow".into()),
                object_class: Some("local".into()),
                note: None,
                ts: "2026-09-21T00:00:01Z".into(),
            },
        )
        .unwrap();
    }

    #[test]
    fn catalog_registers_both_drivers_and_refuses_unknown() {
        let ids: Vec<_> = train_enrich_catalog()
            .into_iter()
            .map(|card| card.driver_id)
            .collect();
        assert!(ids.contains(&"ollama-modelfile"));
        assert!(ids.contains(&"external-manifest"));
        assert!(ids.contains(&LLAMAFACTORY_QLORA_ID));
        assert!(ids.contains(&AXOLOTL_LORA_ID));
        assert!(!ids.contains(&"unsloth-qlora"));
        let factory = train_enrich_card(LLAMAFACTORY_QLORA_ID).unwrap();
        assert_eq!(factory.default_job, EnrichJobKind::Train);
        assert!(factory.jobs.contains(&EnrichJobKind::Train));
        assert!(!factory.jobs.contains(&EnrichJobKind::Enrich));
        let axolotl = train_enrich_card(AXOLOTL_LORA_ID).unwrap();
        assert_eq!(axolotl.default_job, EnrichJobKind::Train);
        let enrich_ids = train_enrich_drivers_for_job("enrich").unwrap();
        assert!(!enrich_ids.contains(&LLAMAFACTORY_QLORA_ID));
        assert!(!enrich_ids.contains(&AXOLOTL_LORA_ID));
        assert_eq!(enrich_ids.len(), 2);
        let train_ids = train_enrich_drivers_for_job("train").unwrap();
        assert!(train_ids.contains(&LLAMAFACTORY_QLORA_ID));
        assert!(train_ids.contains(&AXOLOTL_LORA_ID));
        assert_eq!(train_ids.len(), 4);
        for card in train_enrich_catalog() {
            let driver = resolve_train_enrich_driver(card.driver_id).unwrap();
            assert_eq!(driver.id(), card.driver_id);
            let probe = driver.probe();
            assert!(!probe.live);
            assert_eq!(probe.status, card.status);
        }
        let err = match resolve_train_enrich_driver("future-entrant") {
            Err(err) => err,
            Ok(_) => panic!("unknown driver resolved"),
        };
        assert!(err.to_string().contains("refuse:driver"), "{err}");
        let rendered = render_train_enrich_catalog();
        assert!(rendered.contains("ollama-modelfile"), "{rendered}");
        assert!(rendered.contains("external-manifest"), "{rendered}");
        assert!(rendered.contains(LLAMAFACTORY_QLORA_ID), "{rendered}");
        assert!(!rendered.contains("unsloth-qlora"), "{rendered}");
        assert!(rendered.contains(AXOLOTL_LORA_ID), "{rendered}");
        assert!(rendered.contains("live=false"), "{rendered}");
        assert!(rendered.contains("default=train"), "{rendered}");
    }

    #[test]
    fn llamafactory_qlora_prepares_a_recipe_and_imports_the_adapter() {
        let root = tmp("llamafactory");
        let pack = fixture_pack();
        let seated = seated_estate("llama3");
        let blocked = root.join("seat-only");
        let err = run(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &seated,
            &blocked,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("llama3"), "{err}");
        assert!(!err.to_string().contains("meta-llama"), "{err}");
        assert!(!blocked.exists());

        let estate = with_train_base(seated, "Qwen/Qwen2.5-0.5B-Instruct");
        let out = root.join("recipe");
        let doc = run(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &estate,
            &out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(doc.job, "train");
        assert_eq!(doc.driver, LLAMAFACTORY_QLORA_ID);
        assert_eq!(doc.base_model, "llama3");
        assert_eq!(doc.seat_tag.as_deref(), Some("llama3"));
        assert_eq!(
            doc.train_base_model.as_deref(),
            Some("Qwen/Qwen2.5-0.5B-Instruct")
        );
        assert!(!doc.promoted && !doc.auto_apply && !doc.estate_rewritten);
        for name in [
            "recipe.yaml",
            "export.yaml",
            "dataset_info.json",
            "dataset.jsonl",
            "PREPARE.md",
            "NEXT.md",
            "prepare.json",
        ] {
            assert!(doc.artifacts.iter().any(|item| item == name), "{name}");
            assert!(out.join(name).is_file(), "{name}");
        }
        assert!(!out.join("train_unsloth.py").exists());
        let recipe = std::fs::read_to_string(out.join("recipe.yaml")).unwrap();
        assert!(
            recipe.contains("model_name_or_path: \"Qwen/Qwen2.5-0.5B-Instruct\""),
            "{recipe}"
        );
        assert!(
            !recipe.contains("model_name_or_path: \"llama3\""),
            "{recipe}"
        );
        assert!(!recipe.contains("meta-llama"), "{recipe}");
        assert!(
            !recipe
                .lines()
                .any(|line| line.trim_start().starts_with("max_steps:")),
            "{recipe}"
        );
        assert!(
            recipe.lines().any(|line| line.trim() == "save_steps: 50"),
            "{recipe}"
        );
        assert!(recipe.contains("stage: sft"), "{recipe}");
        assert!(recipe.contains("finetuning_type: lora"), "{recipe}");
        assert!(recipe.contains("quantization_bit: 4"), "{recipe}");
        assert!(recipe.contains("quantization_method: bnb"), "{recipe}");
        assert!(
            !recipe.contains("quantization_method: bitsandbytes"),
            "{recipe}"
        );
        assert!(recipe.contains("lora_rank: 16"), "{recipe}");
        assert!(recipe.contains("packing: true"), "{recipe}");
        assert!(recipe.contains("cutoff_len: 512"), "{recipe}");
        assert!(recipe.contains("template: qwen"), "{recipe}");
        assert!(!recipe.contains("template: llama3"), "{recipe}");
        assert!(recipe.contains("dataset: cell_enrich"), "{recipe}");
        assert!(recipe.contains("seed: 42"), "{recipe}");
        assert!(recipe.contains("# stage: dpo"), "{recipe}");
        assert!(recipe.contains("# or stage: orpo"), "{recipe}");
        let export = std::fs::read_to_string(out.join("export.yaml")).unwrap();
        assert!(!export.contains("quantization_bit"), "{export}");
        assert!(
            export.contains("model_name_or_path: \"Qwen/Qwen2.5-0.5B-Instruct\""),
            "{export}"
        );
        assert!(export.contains("template: qwen"), "{export}");
        assert!(!export.contains("template: llama3"), "{export}");
        assert!(export.contains("finetuning_type: lora"), "{export}");
        let info = std::fs::read_to_string(out.join("dataset_info.json")).unwrap();
        assert!(info.contains("\"formatting\": \"sharegpt\""), "{info}");
        assert!(info.contains("\"messages\": \"messages\""), "{info}");
        let jsonl = std::fs::read_to_string(out.join("dataset.jsonl")).unwrap();
        assert!(jsonl.contains("\"messages\""), "{jsonl}");
        assert!(jsonl.contains("\"role\":\"user\""), "{jsonl}");
        assert!(jsonl.contains("feed/events.jsonl"), "{jsonl}");
        assert_eq!(jsonl.lines().count(), 1, "{jsonl}");
        let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
        let recipe_path = out.join("recipe.yaml");
        let export_path = out.join("export.yaml");
        assert!(
            next.contains(&format!("llamafactory-cli train {}", recipe_path.display())),
            "{next}"
        );
        assert!(
            next.contains(&format!(
                "llamafactory-cli export {}",
                export_path.display()
            )),
            "{next}"
        );
        assert!(next.contains("pip install llamafactory"), "{next}");
        assert!(next.contains("bitsandbytes>=0.49"), "{next}");
        assert!(next.contains("2.11.0+cu128"), "{next}");
        assert!(next.contains("Seat tag is llama3"), "{next}");
        assert!(next.contains("Qwen/Qwen2.5-0.5B-Instruct"), "{next}");
        assert!(next.contains("--max-steps 10"), "{next}");
        assert!(next.contains("does not map the seat tag"), "{next}");
        assert!(next.contains("import-trained"), "{next}");
        assert!(next.contains("CUDA LLaMA-Factory"), "{next}");
        assert!(next.contains("does not write an MLX trainer"), "{next}");
        assert!(next.contains("same chat template"), "{next}");
        assert!(next.contains("smoke eval"), "{next}");
        assert!(next.contains("Faster single-GPU alternate"), "{next}");
        assert!(next.contains("unsloth.ai/docs"), "{next}");
        assert!(next.contains("axolotl-lora"), "{next}");
        assert!(!next.contains("unsloth-qlora"), "{next}");
        let prepare_md = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
        assert!(
            prepare_md.contains("llamafactory-cli train recipe.yaml"),
            "{prepare_md}"
        );
        assert!(
            prepare_md.contains("did not run llamafactory-cli"),
            "{prepare_md}"
        );
        assert!(prepare_md.contains("bitsandbytes>=0.49"), "{prepare_md}");
        assert!(
            prepare_md.contains("Train base: Qwen/Qwen2.5-0.5B-Instruct"),
            "{prepare_md}"
        );
        assert!(prepare_md.contains("Seat tag: llama3"), "{prepare_md}");
        let prepare_json = std::fs::read_to_string(out.join("prepare.json")).unwrap();
        assert!(!prepare_json.contains("llamafactory-cli"), "{prepare_json}");
        assert!(
            prepare_json.contains("\"job\": \"train\""),
            "{prepare_json}"
        );
        assert!(
            prepare_json.contains("\"seat_tag\": \"llama3\""),
            "{prepare_json}"
        );
        assert!(
            prepare_json.contains("\"train_base_model\": \"Qwen/Qwen2.5-0.5B-Instruct\""),
            "{prepare_json}"
        );
        assert!(
            prepare_json.contains("\"base_model\": \"llama3\""),
            "{prepare_json}"
        );
        assert_eq!(doc.dataset_mode.as_deref(), Some("scaffold"));
        assert_eq!(doc.dataset_from_feed, Some(false));
        assert_eq!(doc.dataset_rows, Some(1));
        assert_eq!(doc.dataset_read_paths.as_deref(), Some(&[][..]));
        assert!(next.contains("dataset_mode: scaffold"), "{next}");
        assert!(next.contains("refuse:dataset"), "{next}");
        assert!(next.contains("--from-feed"), "{next}");
        assert!(next.contains("not training data"), "{next}");
        assert!(next.contains("did not read"), "{next}");
        assert!(prepare_md.contains("Dataset mode: scaffold"), "{prepare_md}");
        assert!(prepare_md.contains("dataset_mode: scaffold"), "{prepare_md}");
        assert!(recipe.contains("dataset_mode: scaffold"), "{recipe}");

        let enrich_out = root.join("enrich-job");
        let err = run(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &estate,
            &enrich_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");
        assert!(!enrich_out.exists());

        let mut empty = pack.clone();
        empty.source_paths.clear();
        let stub_out = root.join("stub");
        run(
            LLAMAFACTORY_QLORA_ID,
            &empty,
            &estate,
            &stub_out,
            "train",
            "jason",
        )
        .unwrap();
        let stub_jsonl = std::fs::read_to_string(stub_out.join("dataset.jsonl")).unwrap();
        assert_eq!(stub_jsonl.lines().count(), 3, "{stub_jsonl}");
        let stub_next = std::fs::read_to_string(stub_out.join("NEXT.md")).unwrap();
        assert!(
            stub_next.contains("source_paths list is empty"),
            "{stub_next}"
        );
        assert!(stub_next.contains("dataset_mode: stub"), "{stub_next}");
        assert!(stub_next.contains("refuse:dataset"), "{stub_next}");
        assert!(stub_next.contains("not training data"), "{stub_next}");
        let stub_prepare = std::fs::read_to_string(stub_out.join("prepare.json")).unwrap();
        assert!(
            stub_prepare.contains("\"dataset_mode\": \"stub\""),
            "{stub_prepare}"
        );
        assert!(
            stub_prepare.contains("\"dataset_from_feed\": false"),
            "{stub_prepare}"
        );
        let stub_md = std::fs::read_to_string(stub_out.join("PREPARE.md")).unwrap();
        assert!(stub_md.contains("Dataset mode: stub"), "{stub_md}");
        assert!(stub_md.contains("dataset_mode: stub"), "{stub_md}");

        let mut apple = pack.clone();
        apple.host_class_affinity = Some("apple-silicon".into());
        let apple_out = root.join("apple");
        run(
            LLAMAFACTORY_QLORA_ID,
            &apple,
            &estate,
            &apple_out,
            "train",
            "jason",
        )
        .unwrap();
        let apple_next = std::fs::read_to_string(apple_out.join("NEXT.md")).unwrap();
        assert!(apple_next.contains("apple-silicon"), "{apple_next}");
        assert!(apple_next.contains("CUDA LLaMA-Factory"), "{apple_next}");
        assert!(
            apple_next.contains("does not write an MLX trainer"),
            "{apple_next}"
        );

        let adapter = root.join("adapter");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        let proposal = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap();
        assert_eq!(proposal.driver, LLAMAFACTORY_QLORA_ID);
        assert_eq!(proposal.binding_id, "local_slm");
        assert!(proposal.local_path.ends_with("adapter_config.json"));
        assert!(!proposal.auto_apply && !proposal.estate_rewritten);

        let recipe_path = out.join("recipe.yaml");
        let original = std::fs::read_to_string(&recipe_path).unwrap();
        std::fs::write(
            &recipe_path,
            original.replace(
                "model_name_or_path: \"Qwen/Qwen2.5-0.5B-Instruct\"",
                "model_name_or_path: \"llama3\"",
            ),
        )
        .unwrap();
        let tampered = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            tampered.to_string().contains("refuse:train-base"),
            "{tampered}"
        );
        std::fs::write(&recipe_path, original).unwrap();
    }

    #[test]
    fn llamafactory_train_base_follows_the_weights_and_refuses_a_seat_tag() {
        let root = tmp("train-base");
        let pack = fixture_pack();
        let seated = seated_estate("qwen2.5:0.5b");

        for bad in [
            "llama3",
            "llama3:latest",
            "qwen2.5:0.5b",
            "hf.co/Qwen/Qwen2.5-0.5B-Instruct",
        ] {
            let mut bad_pack = pack.clone();
            bad_pack.train_base_model = Some(bad.into());
            let out = root.join(bad.replace(':', "_").replace('/', "_").replace('.', "_"));
            let err = run(
                LLAMAFACTORY_QLORA_ID,
                &bad_pack,
                &seated,
                &out,
                "train",
                "jason",
            )
            .unwrap_err();
            assert!(
                err.to_string().contains("refuse:train-base"),
                "{bad}: {err}"
            );
            assert!(!out.exists(), "{bad}");
        }

        let mut pack_wins = pack.clone();
        pack_wins.train_base_model = Some("Qwen/Qwen2.5-0.5B-Instruct".into());
        let binding_other = with_train_base(seated.clone(), "mistralai/Mistral-7B-Instruct-v0.3");
        let win_out = root.join("pack-wins");
        run(
            LLAMAFACTORY_QLORA_ID,
            &pack_wins,
            &binding_other,
            &win_out,
            "train",
            "jason",
        )
        .unwrap();
        let win_recipe = std::fs::read_to_string(win_out.join("recipe.yaml")).unwrap();
        assert!(
            win_recipe.contains("model_name_or_path: \"Qwen/Qwen2.5-0.5B-Instruct\""),
            "{win_recipe}"
        );
        assert!(win_recipe.contains("template: qwen"), "{win_recipe}");
        assert!(!win_recipe.contains("template: mistral"), "{win_recipe}");

        let mut bad_pack = pack.clone();
        bad_pack.train_base_model = Some("llama3".into());
        let fallthrough = root.join("no-fallthrough");
        let err = run(
            LLAMAFACTORY_QLORA_ID,
            &bad_pack,
            &binding_other,
            &fallthrough,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(!fallthrough.exists());

        let supplied = with_train_base(seated.clone(), "meta-llama/Meta-Llama-3-8B-Instruct");
        let supplied_out = root.join("supplied-llama");
        let doc = run(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &supplied,
            &supplied_out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(doc.base_model, "qwen2.5:0.5b");
        assert_eq!(
            doc.train_base_model.as_deref(),
            Some("meta-llama/Meta-Llama-3-8B-Instruct")
        );
        let supplied_recipe = std::fs::read_to_string(supplied_out.join("recipe.yaml")).unwrap();
        assert!(
            supplied_recipe.contains("template: llama3"),
            "{supplied_recipe}"
        );
        assert!(
            !supplied_recipe.contains("template: qwen"),
            "{supplied_recipe}"
        );
        assert!(
            supplied_recipe.contains("model_name_or_path: \"meta-llama/Meta-Llama-3-8B-Instruct\""),
            "{supplied_recipe}"
        );

        let local = with_train_base(
            seated.clone(),
            "/tmp/cell-one-hf-weights/Qwen2.5-0.5B-Instruct",
        );
        let local_out = root.join("local-dir");
        run(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &local,
            &local_out,
            "train",
            "jason",
        )
        .unwrap();
        let local_recipe = std::fs::read_to_string(local_out.join("recipe.yaml")).unwrap();
        assert!(
            local_recipe
                .contains("model_name_or_path: \"/tmp/cell-one-hf-weights/Qwen2.5-0.5B-Instruct\""),
            "{local_recipe}"
        );
        assert!(local_recipe.contains("template: qwen"), "{local_recipe}");

        for bad in ["./llama3", "../llama3", "./weights/llama3", "./Llama3"] {
            let bad_estate = with_train_base(seated.clone(), bad);
            let out = root.join(format!(
                "leaf-{}",
                bad.trim_start_matches('.').replace('/', "_")
            ));
            let err = run(
                LLAMAFACTORY_QLORA_ID,
                &pack,
                &bad_estate,
                &out,
                "train",
                "jason",
            )
            .unwrap_err();
            assert!(
                err.to_string().contains("refuse:train-base"),
                "{bad}: {err}"
            );
            assert!(err.to_string().contains("Ollama seat tag"), "{bad}: {err}");
            assert!(!err.to_string().contains("meta-llama"), "{bad}: {err}");
            assert!(!out.exists(), "{bad}");
        }

        let relative_raw = "./weights/Qwen2.5-0.5B-Instruct";
        let expected = canonical_train_base(relative_raw, "qwen2.5:0.5b").unwrap();
        assert!(expected.starts_with('/'), "{expected}");
        assert!(
            expected.ends_with("/weights/Qwen2.5-0.5B-Instruct"),
            "{expected}"
        );
        assert!(
            !expected.contains("/./") && !expected.contains(".."),
            "{expected}"
        );
        let relative = with_train_base(local.clone(), relative_raw);
        let relative_out = root.join("relative");
        let relative_doc = run(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &relative,
            &relative_out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(
            relative_doc.train_base_model.as_deref(),
            Some(expected.as_str())
        );
        let relative_recipe = std::fs::read_to_string(relative_out.join("recipe.yaml")).unwrap();
        let quoted = format!("model_name_or_path: \"{expected}\"");
        assert!(relative_recipe.contains(&quoted), "{relative_recipe}");
        assert!(!relative_recipe.contains("./weights"), "{relative_recipe}");
        let relative_export = std::fs::read_to_string(relative_out.join("export.yaml")).unwrap();
        assert!(relative_export.contains(&quoted), "{relative_export}");
        let relative_next = std::fs::read_to_string(relative_out.join("NEXT.md")).unwrap();
        assert!(relative_next.contains(&expected), "{relative_next}");

        let gauge_out = root.join("gauge");
        run_max(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &local,
            &gauge_out,
            "train",
            "jason",
            Some(10),
        )
        .unwrap();
        let gauge = std::fs::read_to_string(gauge_out.join("recipe.yaml")).unwrap();
        assert!(
            gauge.lines().any(|line| line.trim() == "max_steps: 10"),
            "{gauge}"
        );
        assert!(
            gauge.lines().any(|line| line.trim() == "save_steps: 10"),
            "{gauge}"
        );
        assert!(gauge.contains("num_train_epochs: 1.0"), "{gauge}");
        assert!(gauge.contains("quantization_method: bnb"), "{gauge}");
        let gauge_next = std::fs::read_to_string(gauge_out.join("NEXT.md")).unwrap();
        assert!(
            gauge_next.contains("gauge run with max_steps 10"),
            "{gauge_next}"
        );

        let zero = root.join("zero-steps");
        let err = run_max(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &local,
            &zero,
            "train",
            "jason",
            Some(0),
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:max-steps"), "{err}");
        assert!(!zero.exists());

        let mut legacy: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(local_out.join("prepare.json")).unwrap())
                .unwrap();
        legacy.as_object_mut().unwrap().remove("train_base_model");
        let legacy_dir = root.join("legacy");
        std::fs::create_dir_all(&legacy_dir).unwrap();
        for name in [
            "recipe.yaml",
            "export.yaml",
            "dataset.jsonl",
            "PREPARE.md",
            "NEXT.md",
        ] {
            std::fs::copy(local_out.join(name), legacy_dir.join(name)).unwrap();
        }
        std::fs::write(
            legacy_dir.join("prepare.json"),
            serde_json::to_string_pretty(&legacy).unwrap(),
        )
        .unwrap();
        let adapter = root.join("adapter");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        let legacy_import = import_trained(&ImportTrainedRequest {
            estate: &local,
            prepared_dir: &legacy_dir,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            legacy_import.to_string().contains("refuse:train-base"),
            "{legacy_import}"
        );
    }

    #[test]
    fn axolotl_lora_prepares_a_recipe_and_imports_the_adapter() {
        let root = tmp("axolotl");
        let pack = fixture_pack();
        let seated = seated_estate("llama3");
        let blocked = root.join("seat-only");
        let err = run(AXOLOTL_LORA_ID, &pack, &seated, &blocked, "train", "jason").unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("llama3"), "{err}");
        assert!(!err.to_string().contains("meta-llama"), "{err}");
        assert!(!blocked.exists());

        for bad in ["./llama3", "../llama3", "llama3:latest"] {
            let bad_estate = with_train_base(seated.clone(), bad);
            let bad_out = root.join(bad.trim_start_matches('.').replace(['/', ':'], "_"));
            let err = run(
                AXOLOTL_LORA_ID,
                &pack,
                &bad_estate,
                &bad_out,
                "train",
                "jason",
            )
            .unwrap_err();
            assert!(
                err.to_string().contains("refuse:train-base"),
                "{bad}: {err}"
            );
            assert!(err.to_string().contains("Ollama seat tag"), "{bad}: {err}");
            assert!(!err.to_string().contains("meta-llama"), "{bad}: {err}");
            assert!(!bad_out.exists(), "{bad}");
        }

        let estate = with_train_base(seated.clone(), "Qwen/Qwen2.5-0.5B-Instruct");
        let out = root.join("recipe");
        let doc = run(AXOLOTL_LORA_ID, &pack, &estate, &out, "train", "jason").unwrap();
        assert_eq!(doc.job, "train");
        assert_eq!(doc.driver, AXOLOTL_LORA_ID);
        assert_eq!(doc.base_model, "llama3");
        assert_eq!(doc.seat_tag.as_deref(), Some("llama3"));
        assert_eq!(
            doc.train_base_model.as_deref(),
            Some("Qwen/Qwen2.5-0.5B-Instruct")
        );
        assert!(!doc.promoted && !doc.auto_apply && !doc.estate_rewritten);
        for name in [
            "axolotl.yml",
            "dataset.jsonl",
            "PREPARE.md",
            "NEXT.md",
            "prepare.json",
        ] {
            assert!(doc.artifacts.iter().any(|item| item == name), "{name}");
            assert!(out.join(name).is_file(), "{name}");
        }
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
        assert!(!yaml.contains("meta-llama"), "{yaml}");
        assert!(yaml.contains("# seat_tag: llama3"), "{yaml}");
        assert!(yaml.contains("adapter: qlora"), "{yaml}");
        assert!(yaml.contains("load_in_4bit: true"), "{yaml}");
        assert!(yaml.contains("type: alpaca"), "{yaml}");
        assert!(
            yaml.contains("  - path:") && yaml.contains("\n    ds_type: json\n    type: alpaca\n"),
            "{yaml}"
        );
        let dataset_path = out.join("dataset.jsonl");
        assert!(yaml.contains(&dataset_path.display().to_string()), "{yaml}");
        assert!(yaml.contains("dataset_mode: scaffold"), "{yaml}");
        let ax_prepare = std::fs::read_to_string(out.join("prepare.json")).unwrap();
        assert!(
            ax_prepare.contains("\"dataset_mode\": \"scaffold\""),
            "{ax_prepare}"
        );
        assert!(
            ax_prepare.contains("\"dataset_from_feed\": false"),
            "{ax_prepare}"
        );
        let jsonl = std::fs::read_to_string(&dataset_path).unwrap();
        assert!(jsonl.contains("feed/events.jsonl"), "{jsonl}");
        assert_eq!(jsonl.lines().count(), 1, "{jsonl}");
        let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
        let config = out.join("axolotl.yml");
        assert!(
            next.contains(&format!("axolotl train {}", config.display())),
            "{next}"
        );
        assert!(next.contains("Seat tag is llama3"), "{next}");
        assert!(
            next.contains("Train base is Qwen/Qwen2.5-0.5B-Instruct"),
            "{next}"
        );
        assert!(next.contains("does not map the seat tag"), "{next}");
        assert!(
            next.contains("FROM an Ollama model of this same train base, plus ADAPTER"),
            "{next}"
        );
        assert!(!next.contains("FROM llama3 plus ADAPTER"), "{next}");
        assert!(!next.contains("Edit axolotl.yml"), "{next}");
        assert!(next.contains("import-trained"), "{next}");
        assert!(next.contains("unslothai/unsloth"), "{next}");
        assert!(next.contains("does not write an MLX trainer"), "{next}");
        assert!(next.contains("consumer-nvidia"), "{next}");
        assert!(next.contains("dataset_mode: scaffold"), "{next}");
        assert!(next.contains("refuse:dataset"), "{next}");
        assert!(next.contains("--from-feed"), "{next}");
        assert!(next.contains("not training data"), "{next}");
        let ax_prepare_md = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
        assert!(
            ax_prepare_md.contains("dataset_mode: scaffold"),
            "{ax_prepare_md}"
        );

        let relative_raw = "./weights/Qwen2.5-0.5B-Instruct";
        let expected = canonical_train_base(relative_raw, "llama3").unwrap();
        let relative = with_train_base(seated.clone(), relative_raw);
        let relative_out = root.join("relative");
        let relative_doc = run(
            AXOLOTL_LORA_ID,
            &pack,
            &relative,
            &relative_out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(
            relative_doc.train_base_model.as_deref(),
            Some(expected.as_str())
        );
        assert_eq!(relative_doc.base_model, "llama3");
        let relative_yaml = std::fs::read_to_string(relative_out.join("axolotl.yml")).unwrap();
        let quoted = format!("base_model: \"{expected}\"");
        assert!(relative_yaml.contains(&quoted), "{relative_yaml}");
        assert!(!relative_yaml.contains("./weights"), "{relative_yaml}");
        let relative_next = std::fs::read_to_string(relative_out.join("NEXT.md")).unwrap();
        assert!(relative_next.contains(&expected), "{relative_next}");

        let mut pack_wins = pack.clone();
        pack_wins.train_base_model = Some("Qwen/Qwen2.5-0.5B-Instruct".into());
        let binding_other = with_train_base(seated.clone(), "mistralai/Mistral-7B-Instruct-v0.3");
        let win_out = root.join("pack-wins");
        run(
            AXOLOTL_LORA_ID,
            &pack_wins,
            &binding_other,
            &win_out,
            "train",
            "jason",
        )
        .unwrap();
        let win_yaml = std::fs::read_to_string(win_out.join("axolotl.yml")).unwrap();
        assert!(
            win_yaml.contains("base_model: \"Qwen/Qwen2.5-0.5B-Instruct\""),
            "{win_yaml}"
        );
        assert!(
            !win_yaml.contains("mistralai/Mistral-7B-Instruct-v0.3"),
            "{win_yaml}"
        );

        let drivers = train_enrich_drivers_for_job("train").unwrap();
        let all_dirs: Vec<_> = drivers
            .iter()
            .map(|id| root.join(format!("all-{id}")))
            .collect();
        let all_reqs: Vec<PrepareEnrichRequest<'_>> = drivers
            .iter()
            .zip(all_dirs.iter())
            .map(|(id, dir)| PrepareEnrichRequest {
                estate: &estate,
                pack: &pack,
                curator: "jason",
                driver_id: *id,
                job: "train",
                out_dir: dir,
                max_steps: None,
                from_feed: false,
                state_dir: Path::new(".cell"),
            })
            .collect();
        let all_docs = prepare_enrich_set(&all_reqs).unwrap();
        assert_eq!(all_docs.len(), 4);
        let all_yaml =
            std::fs::read_to_string(root.join("all-axolotl-lora").join("axolotl.yml")).unwrap();
        assert!(
            all_yaml.contains("base_model: \"Qwen/Qwen2.5-0.5B-Instruct\""),
            "{all_yaml}"
        );
        assert!(
            !all_yaml
                .lines()
                .any(|line| line.trim_start().starts_with("base_model:") && line.contains("llama3")),
            "{all_yaml}"
        );
        let all_modelfile =
            std::fs::read_to_string(root.join("all-ollama-modelfile").join("Modelfile")).unwrap();
        assert!(all_modelfile.contains("FROM llama3\n"), "{all_modelfile}");

        let missing_dirs: Vec<_> = drivers
            .iter()
            .map(|id| root.join(format!("missing-{id}")))
            .collect();
        let missing_reqs: Vec<PrepareEnrichRequest<'_>> = drivers
            .iter()
            .zip(missing_dirs.iter())
            .map(|(id, dir)| PrepareEnrichRequest {
                estate: &seated,
                pack: &pack,
                curator: "jason",
                driver_id: *id,
                job: "train",
                out_dir: dir,
                max_steps: None,
                from_feed: false,
                state_dir: Path::new(".cell"),
            })
            .collect();
        let missing_set = prepare_enrich_set(&missing_reqs).unwrap_err();
        assert!(
            missing_set.to_string().contains("refuse:train-base"),
            "{missing_set}"
        );
        for dir in &missing_dirs {
            assert!(!dir.exists(), "{}", dir.display());
        }

        let prepare_md = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
        assert!(
            prepare_md.contains("axolotl train axolotl.yml"),
            "{prepare_md}"
        );
        assert!(prepare_md.contains("did not run axolotl"), "{prepare_md}");

        let enrich_out = root.join("enrich-job");
        let err = run(
            AXOLOTL_LORA_ID,
            &pack,
            &estate,
            &enrich_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");
        assert!(!enrich_out.exists());

        let mut blank = pack.clone();
        blank.source_paths = vec!["".into()];
        let blank_out = root.join("blank-path");
        let err = run(
            AXOLOTL_LORA_ID,
            &blank,
            &estate,
            &blank_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:dataset"), "{err}");
        assert!(!blank_out.exists());

        let mut empty = pack.clone();
        empty.source_paths.clear();
        let stub_out = root.join("stub");
        let stub = run(
            AXOLOTL_LORA_ID,
            &empty,
            &estate,
            &stub_out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(stub.job, "train");
        let stub_jsonl = std::fs::read_to_string(stub_out.join("dataset.jsonl")).unwrap();
        assert_eq!(stub_jsonl.lines().count(), 3, "{stub_jsonl}");
        let stub_next = std::fs::read_to_string(stub_out.join("NEXT.md")).unwrap();
        assert!(
            stub_next.contains("source_paths list is empty"),
            "{stub_next}"
        );
        assert!(stub_next.contains("dataset_mode: stub"), "{stub_next}");
        assert!(stub_next.contains("refuse:dataset"), "{stub_next}");
        let stub_prepare = std::fs::read_to_string(stub_out.join("PREPARE.md")).unwrap();
        assert!(stub_prepare.contains("dataset_mode: stub"), "{stub_prepare}");
        let stub_doc = std::fs::read_to_string(stub_out.join("prepare.json")).unwrap();
        assert!(
            stub_doc.contains("\"dataset_mode\": \"stub\""),
            "{stub_doc}"
        );

        let mut apple = pack.clone();
        apple.host_class_affinity = Some("apple-silicon".into());
        let apple_out = root.join("apple");
        run(
            AXOLOTL_LORA_ID,
            &apple,
            &estate,
            &apple_out,
            "train",
            "jason",
        )
        .unwrap();
        let apple_next = std::fs::read_to_string(apple_out.join("NEXT.md")).unwrap();
        assert!(apple_next.contains("apple-silicon"), "{apple_next}");
        assert!(apple_next.contains("CUDA"), "{apple_next}");
        assert!(
            apple_next.contains("does not write an MLX trainer"),
            "{apple_next}"
        );

        let mut affinity_off = pack.clone();
        affinity_off.host_class_affinity = None;
        let mut nvidia = estate.clone();
        nvidia
            .model_bindings
            .iter_mut()
            .find(|binding| binding.id == "local_slm")
            .unwrap()
            .params
            .as_object_mut()
            .unwrap()
            .insert(
                "host_class".into(),
                serde_json::Value::String("consumer-nvidia".into()),
            );
        let host_out = root.join("host");
        let hosted = run(
            AXOLOTL_LORA_ID,
            &affinity_off,
            &nvidia,
            &host_out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(hosted.host_class_affinity, "consumer-nvidia");

        let adapter = root.join("adapter");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        std::fs::write(adapter.join("adapter_model.safetensors"), "weights").unwrap();
        let proposal = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap();
        assert_eq!(proposal.driver, AXOLOTL_LORA_ID);
        assert_eq!(proposal.job, "train");
        assert_eq!(proposal.binding_id, "local_slm");
        assert!(!proposal.auto_apply && !proposal.promoted && !proposal.estate_rewritten);
        assert!(proposal.local_path.ends_with("adapter_config.json"));
        assert!(out.join("binding-proposal.json").is_file());

        let yaml_path = out.join("axolotl.yml");
        let original = std::fs::read_to_string(&yaml_path).unwrap();
        std::fs::write(
            &yaml_path,
            original.replace(
                "base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"",
                "base_model: \"llama3\"",
            ),
        )
        .unwrap();
        let tampered = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            tampered.to_string().contains("refuse:train-base"),
            "{tampered}"
        );
        assert!(tampered.to_string().contains("base_model"), "{tampered}");
        std::fs::write(&yaml_path, &original).unwrap();

        let mut legacy: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(out.join("prepare.json")).unwrap())
                .unwrap();
        legacy.as_object_mut().unwrap().remove("train_base_model");
        let legacy_dir = root.join("legacy");
        std::fs::create_dir_all(&legacy_dir).unwrap();
        for name in ["axolotl.yml", "dataset.jsonl", "PREPARE.md", "NEXT.md"] {
            std::fs::copy(out.join(name), legacy_dir.join(name)).unwrap();
        }
        std::fs::write(
            legacy_dir.join("prepare.json"),
            serde_json::to_string_pretty(&legacy).unwrap(),
        )
        .unwrap();
        let legacy_import = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &legacy_dir,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            legacy_import.to_string().contains("refuse:train-base"),
            "{legacy_import}"
        );

        let gguf = root.join("merged.gguf");
        std::fs::write(&gguf, "gguf-fixture").unwrap();
        let _ = std::fs::remove_file(out.join("binding-proposal.json"));
        let from_file = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag: "cell-enrich-overnight-traces",
            adapter: &gguf,
            curator: "jason",
        })
        .unwrap();
        assert!(from_file.local_path.ends_with("merged.gguf"));

        let missing = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag: "cell-enrich-overnight-traces",
            adapter: &root.join("no-adapter"),
            curator: "jason",
        })
        .unwrap_err();
        assert!(missing.to_string().contains("refuse:adapter"), "{missing}");

        let ollama_out = root.join("ollama");
        run(
            "ollama-modelfile",
            &pack,
            &estate,
            &ollama_out,
            "enrich",
            "jason",
        )
        .unwrap();
        let wrong = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &ollama_out,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap_err();
        assert!(wrong.to_string().contains("refuse:driver"), "{wrong}");
        assert!(!ollama_out.join("binding-proposal.json").is_file());
    }

    #[test]
    fn both_drivers_prepare_fixture_without_training() {
        let root = tmp("happy");
        let pack = fixture_pack();
        let estate = seated_estate("llama3");
        let ollama_out = root.join("ollama");
        let doc = run(
            "ollama-modelfile",
            &pack,
            &estate,
            &ollama_out,
            "enrich",
            "jason",
        )
        .unwrap();
        assert_eq!(doc.schema, PREPARE_SCHEMA);
        assert!(!doc.promoted && !doc.auto_apply && !doc.estate_rewritten);
        assert_eq!(doc.base_model, "llama3");
        let modelfile = std::fs::read_to_string(ollama_out.join("Modelfile")).unwrap();
        assert!(modelfile.contains("FROM llama3\n"), "{modelfile}");
        assert!(!modelfile.contains("FROM local_slm"), "{modelfile}");
        assert!(modelfile.contains("SYSTEM \"\"\""), "{modelfile}");
        assert!(modelfile.contains("purpose-built SLM"), "{modelfile}");
        let steps = std::fs::read_to_string(ollama_out.join("PREPARE.md")).unwrap();
        assert!(
            steps.contains("ollama create cell-enrich-overnight-traces -f Modelfile"),
            "{steps}"
        );
        assert!(steps.contains("did not run ollama"), "{steps}");
        let next = std::fs::read_to_string(ollama_out.join("NEXT.md")).unwrap();
        assert!(
            next.contains(&format!(
                "ollama create cell-enrich-overnight-traces -f {}",
                ollama_out.join("Modelfile").display()
            )),
            "{next}"
        );
        assert!(next.contains("import-prepared"), "{next}");
        assert!(next.contains("refuse:enrich-index"), "{next}");
        assert!(doc.artifacts.iter().any(|name| name == "NEXT.md"));

        let manifest_out = root.join("manifest");
        let ext = run(
            "external-manifest",
            &pack,
            &estate,
            &manifest_out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(ext.job, "train");
        let manifest = std::fs::read_to_string(manifest_out.join("manifest.json")).unwrap();
        assert!(
            !manifest.to_ascii_lowercase().contains("ollama"),
            "{manifest}"
        );
        assert!(manifest.contains("\"vendor\": null"), "{manifest}");
        assert!(manifest.contains("feed/events.jsonl"), "{manifest}");
        assert!(
            manifest.contains("\"base_model\": \"llama3\""),
            "{manifest}"
        );
        let yaml = std::fs::read_to_string(manifest_out.join("manifest.yaml")).unwrap();
        assert!(yaml.contains("dataset_paths:"), "{yaml}");
        assert!(!yaml.to_ascii_lowercase().contains("ollama"), "{yaml}");
        let ext_steps = std::fs::read_to_string(manifest_out.join("PREPARE.md")).unwrap();
        assert!(!ext_steps.contains("ollama create"), "{ext_steps}");
        let ext_next = std::fs::read_to_string(manifest_out.join("NEXT.md")).unwrap();
        assert!(!ext_next.contains("ollama create"), "{ext_next}");
        assert!(ext_next.contains("manifest.json"), "{ext_next}");
        assert!(ext_next.contains("import-prepared"), "{ext_next}");
    }

    #[test]
    fn refuse_paths_write_nothing() {
        let root = tmp("refuse");
        let estate = seated_estate("llama3");
        let mut sacred = fixture_pack();
        sacred.note = "please mention cyera".into();
        let sacred_out = root.join("sacred");
        let err = run(
            "ollama-modelfile",
            &sacred,
            &estate,
            &sacred_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:sacred"), "{err}");
        assert!(!sacred_out.exists(), "sacred refuse must not create out");

        let mut sku = fixture_pack();
        sku.model_hint = Some("rtx-5090".into());
        let sku_out = root.join("sku");
        let err = run(
            "ollama-modelfile",
            &sku,
            &estate,
            &sku_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("SKU") || err.to_string().contains("sku"),
            "{err}"
        );
        assert!(!sku_out.exists());

        let mut path_sku = fixture_pack();
        path_sku.source_paths = vec!["weights/4090/set.jsonl".into()];
        let path_out = root.join("path-sku");
        let err = run(
            "external-manifest",
            &path_sku,
            &estate,
            &path_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("SKU") || err.to_string().contains("sku"),
            "{err}"
        );
        assert!(!path_out.exists());

        let mut local = fixture_estate();
        local
            .model_bindings
            .retain(|b| b.class != ModelClass::Frontier);
        let mut frontier = fixture_pack();
        frontier.source_drivers = vec!["frontier".into()];
        frontier.path_counts.frontier = 1;
        let frontier_out = root.join("frontier");
        let err = run(
            "ollama-modelfile",
            &frontier,
            &local,
            &frontier_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:frontier-invent"), "{err}");
        assert!(!frontier_out.exists());

        let curator_out = root.join("curator");
        let err = run(
            "ollama-modelfile",
            &fixture_pack(),
            &estate,
            &curator_out,
            "enrich",
            "ada",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:curator"), "{err}");
        assert!(!curator_out.exists());

        let missing = load_enrich_pack(Path::new("no-such-pack"), &root.join("packs")).unwrap_err();
        assert!(
            missing.to_string().contains("refuse:missing-pack"),
            "{missing}"
        );

        let job_out = root.join("job");
        let err = run(
            "ollama-modelfile",
            &fixture_pack(),
            &estate,
            &job_out,
            "distill",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");
        assert!(!job_out.exists());
    }

    #[test]
    fn from_feed_hydrates_fixture_rows_and_names_refuses() {
        let root = tmp("from-feed");
        let pack = fixture_pack();
        let estate = with_train_base(seated_estate("llama3"), "Qwen/Qwen2.5-0.5B-Instruct");
        let state = root.join("cell");
        write_fixture_feed(&state);

        let quiet = root.join("quiet");
        let quiet_doc = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &estate,
            &quiet,
            &state,
            false,
        )
        .unwrap();
        assert_eq!(quiet_doc.dataset_mode.as_deref(), Some("scaffold"));
        assert_eq!(quiet_doc.dataset_from_feed, Some(false));
        let quiet_jsonl = std::fs::read_to_string(quiet.join("dataset.jsonl")).unwrap();
        assert!(quiet_jsonl.contains("feed/events.jsonl"), "{quiet_jsonl}");
        assert!(
            !quiet_jsonl.contains("job=policy-precheck"),
            "{quiet_jsonl}"
        );
        assert!(
            quiet_jsonl.contains("Replace this scaffold"),
            "{quiet_jsonl}"
        );

        let missing_state = root.join("empty-cell");
        std::fs::create_dir_all(&missing_state).unwrap();
        let missing_out = root.join("missing");
        let missing = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &estate,
            &missing_out,
            &missing_state,
            true,
        )
        .unwrap_err();
        let missing_text = missing.to_string();
        assert!(missing_text.contains("refuse:dataset"), "{missing_text}");
        assert!(missing_text.contains("feed/events.jsonl"), "{missing_text}");
        assert!(
            missing_text.contains("does not download"),
            "{missing_text}"
        );
        assert!(!missing_out.exists(), "{}", missing_out.display());

        let missing_ax = root.join("missing-ax");
        let ax_err = run_feed(
            AXOLOTL_LORA_ID,
            &pack,
            &estate,
            &missing_ax,
            &missing_state,
            true,
        )
        .unwrap_err();
        assert!(ax_err.to_string().contains("refuse:dataset"), "{ax_err}");
        assert!(ax_err.to_string().contains("feed/events.jsonl"), "{ax_err}");
        assert!(!missing_ax.exists());

        let drivers = train_enrich_drivers_for_job("train").unwrap();
        let dirs: Vec<_> = drivers
            .iter()
            .map(|id| root.join(format!("set-{id}")))
            .collect();
        let reqs: Vec<_> = drivers
            .iter()
            .zip(dirs.iter())
            .map(|(id, dir)| PrepareEnrichRequest {
                estate: &estate,
                pack: &pack,
                curator: "jason",
                driver_id: *id,
                job: "train",
                out_dir: dir,
                max_steps: None,
                from_feed: true,
                state_dir: &missing_state,
            })
            .collect();
        let set_err = prepare_enrich_set(&reqs).unwrap_err();
        assert!(
            set_err.to_string().contains("refuse:dataset"),
            "{set_err}"
        );
        for dir in &dirs {
            assert!(!dir.exists(), "{}", dir.display());
        }

        for driver in [LLAMAFACTORY_QLORA_ID, AXOLOTL_LORA_ID] {
            let out = root.join(driver);
            let doc = run_feed(driver, &pack, &estate, &out, &state, true).unwrap();
            assert_eq!(doc.dataset_mode.as_deref(), Some("feed"));
            assert_eq!(doc.dataset_from_feed, Some(true));
            assert_eq!(doc.dataset_rows, Some(1));
            assert_eq!(doc.dataset_skipped, Some(1));
            assert_eq!(
                doc.dataset_read_paths.as_deref(),
                Some(&["feed/events.jsonl".to_string()][..])
            );
            let jsonl = std::fs::read_to_string(out.join("dataset.jsonl")).unwrap();
            assert!(jsonl.contains("job=policy-precheck"), "{driver} {jsonl}");
            assert!(
                jsonl.contains("kind=model.local.precheck"),
                "{driver} {jsonl}"
            );
            assert!(jsonl.contains("class=local"), "{driver} {jsonl}");
            assert!(!jsonl.contains("model.local.skip"), "{driver} {jsonl}");
            assert!(!jsonl.contains("Replace this scaffold"), "{driver} {jsonl}");
            let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
            let prepare_md = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
            for card in [&next, &prepare_md] {
                assert!(
                    card.contains("dataset_mode: feed"),
                    "{driver} {card}"
                );
                assert!(card.contains("refuse:dataset"), "{driver} {card}");
                assert!(
                    card.contains("did not invent a completion"),
                    "{driver} {card}"
                );
                assert!(
                    card.contains("1 scrubbed feed event had no note"),
                    "{driver} {card}"
                );
            }
            assert!(prepare_md.contains("Dataset mode: feed"), "{prepare_md}");
            assert!(
                prepare_md.contains("Dataset from feed: true"),
                "{prepare_md}"
            );
            if driver == AXOLOTL_LORA_ID {
                assert!(jsonl.contains("\"instruction\""), "{jsonl}");
                assert!(jsonl.contains("\"output\":\"job=policy-precheck\""), "{jsonl}");
                let yaml = std::fs::read_to_string(out.join("axolotl.yml")).unwrap();
                assert!(yaml.contains("dataset_mode: feed"), "{yaml}");
            } else {
                assert!(jsonl.contains("\"role\":\"user\""), "{jsonl}");
                assert!(jsonl.contains("\"role\":\"assistant\""), "{jsonl}");
                let recipe = std::fs::read_to_string(out.join("recipe.yaml")).unwrap();
                assert!(recipe.contains("dataset_mode: feed"), "{recipe}");
            }
        }

        let mut empty = pack.clone();
        empty.source_paths.clear();
        let stub_out = root.join("empty-stub");
        let stub = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &empty,
            &estate,
            &stub_out,
            &state,
            false,
        )
        .unwrap();
        assert_eq!(stub.dataset_mode.as_deref(), Some("stub"));
        let refused = root.join("empty-feed");
        let empty_err = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &empty,
            &estate,
            &refused,
            &state,
            true,
        )
        .unwrap_err();
        let empty_text = empty_err.to_string();
        assert!(empty_text.contains("refuse:dataset"), "{empty_text}");
        assert!(empty_text.contains("source_paths"), "{empty_text}");
        assert!(!refused.exists());

        let bad_state = root.join("bad-cell");
        std::fs::create_dir_all(bad_state.join("feed")).unwrap();
        std::fs::write(bad_state.join("feed/events.jsonl"), "not-json\n").unwrap();
        let bad_out = root.join("bad");
        let bad = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &estate,
            &bad_out,
            &bad_state,
            true,
        )
        .unwrap_err();
        assert!(bad.to_string().contains("refuse:dataset"), "{bad}");
        assert!(bad.to_string().contains("not JSON"), "{bad}");
        assert!(!bad_out.exists());

        std::fs::write(bad_state.join("feed/events.jsonl"), "{\"foo\":1}\n").unwrap();
        let unknown = run_feed(
            AXOLOTL_LORA_ID,
            &pack,
            &estate,
            &root.join("unknown"),
            &bad_state,
            true,
        )
        .unwrap_err();
        assert!(
            unknown.to_string().contains("refuse:dataset"),
            "{unknown}"
        );
        assert!(
            unknown.to_string().contains("not an instruct row"),
            "{unknown}"
        );
        assert!(!root.join("unknown").exists());

        std::fs::write(
            bad_state.join("feed/events.jsonl"),
            "{\"kind\":\"model.local.precheck\",\"object_class\":\"local\",\"note\":\"mentions cyera\",\"ts\":\"2026-09-21T00:00:00Z\"}\n",
        )
        .unwrap();
        let sacred = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &estate,
            &root.join("sacred"),
            &bad_state,
            true,
        )
        .unwrap_err();
        assert!(sacred.to_string().contains("refuse:sacred"), "{sacred}");
        assert!(!root.join("sacred").exists());

        std::fs::write(
            bad_state.join("feed/events.jsonl"),
            "{\"kind\":\"model.local.precheck\",\"object_class\":\"local\",\"note\":\"api_key=abcd\",\"ts\":\"2026-09-21T00:00:00Z\"}\n",
        )
        .unwrap();
        let secret = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &estate,
            &root.join("secret"),
            &bad_state,
            true,
        )
        .unwrap_err();
        assert!(
            secret.to_string().contains("refuse:raw-secret"),
            "{secret}"
        );
        assert!(!root.join("secret").exists());

        std::fs::write(
            bad_state.join("feed/events.jsonl"),
            "{\"kind\":\"model.local.precheck\",\"object_class\":\"local\",\"note\":\"host 5090\",\"ts\":\"2026-09-21T00:00:00Z\"}\n",
        )
        .unwrap();
        let sku = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &estate,
            &root.join("sku"),
            &bad_state,
            true,
        )
        .unwrap_err();
        assert!(sku.to_string().contains("refuse:sku-banned"), "{sku}");
        assert!(!root.join("sku").exists());

        let mut local_only = estate.clone();
        local_only
            .model_bindings
            .retain(|binding| binding.class != ModelClass::Frontier);
        std::fs::write(
            bad_state.join("feed/events.jsonl"),
            "{\"kind\":\"model.frontier.complete\",\"object_class\":\"frontier\",\"note\":\"bytes=4\",\"ts\":\"2026-09-21T00:00:00Z\"}\n",
        )
        .unwrap();
        let frontier = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &local_only,
            &root.join("frontier"),
            &bad_state,
            true,
        )
        .unwrap_err();
        assert!(
            frontier.to_string().contains("refuse:frontier-invent"),
            "{frontier}"
        );
        assert!(!root.join("frontier").exists());

        let outside = root.join("outside.jsonl");
        std::fs::write(
            &outside,
            "{\"kind\":\"model.local.precheck\",\"object_class\":\"local\",\"note\":\"job=outside\",\"ts\":\"2026-09-21T00:00:00Z\"}\n",
        )
        .unwrap();
        let link_state = root.join("link-cell");
        std::fs::create_dir_all(link_state.join("feed")).unwrap();
        std::os::unix::fs::symlink(&outside, link_state.join("feed/events.jsonl")).unwrap();
        let escaped = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &estate,
            &root.join("escaped"),
            &link_state,
            true,
        )
        .unwrap_err();
        assert!(
            escaped.to_string().contains("refuse:dataset"),
            "{escaped}"
        );
        assert!(
            escaped.to_string().contains("outside"),
            "{escaped}"
        );
        assert!(!root.join("escaped").exists());
        // The refuse above is the opened fd, not a path that is closed and
        // reopened. Pin the same helper: an inside file stays under the state
        // dir, and the symlink's fd names the outside target. A swap after
        // open cannot retarget that fd; this test does not race one.
        let inside_root = std::fs::canonicalize(&state).unwrap();
        let inside_file = std::fs::File::open(state.join("feed/events.jsonl")).unwrap();
        let inside_pin = opened_file_path(&inside_file).unwrap();
        assert!(
            path_is_within(&inside_root, &inside_pin),
            "inside fd {} is not under {}",
            inside_pin.display(),
            inside_root.display()
        );
        let link_root = std::fs::canonicalize(&link_state).unwrap();
        let outside_file = std::fs::File::open(link_state.join("feed/events.jsonl")).unwrap();
        let outside_pin = opened_file_path(&outside_file).unwrap();
        assert!(
            !path_is_within(&link_root, &outside_pin),
            "outside fd {} must not sit under {}",
            outside_pin.display(),
            link_root.display()
        );

        let mut pairs = pack.clone();
        pairs.source_paths = vec!["feed/pairs.jsonl".into()];
        let pair_state = root.join("pairs-cell");
        std::fs::create_dir_all(pair_state.join("feed")).unwrap();
        std::fs::write(
            pair_state.join("feed/pairs.jsonl"),
            "{\"instruction\":\"Name the pack\",\"input\":\"overnight\",\"output\":\"overnight-traces\"}\n",
        )
        .unwrap();
        let chat_out = root.join("pairs-chat");
        run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pairs,
            &estate,
            &chat_out,
            &pair_state,
            true,
        )
        .unwrap();
        let chat = std::fs::read_to_string(chat_out.join("dataset.jsonl")).unwrap();
        assert!(chat.contains("Name the pack"), "{chat}");
        assert!(chat.contains("overnight-traces"), "{chat}");
        assert!(chat.contains("\"role\":\"assistant\""), "{chat}");
        let alpaca_out = root.join("pairs-alpaca");
        let alpaca_doc = run_feed(
            AXOLOTL_LORA_ID,
            &pairs,
            &estate,
            &alpaca_out,
            &pair_state,
            true,
        )
        .unwrap();
        assert_eq!(alpaca_doc.dataset_rows, Some(1));
        assert_eq!(alpaca_doc.dataset_skipped, Some(0));
        let alpaca = std::fs::read_to_string(alpaca_out.join("dataset.jsonl")).unwrap();
        assert!(alpaca.contains("\"instruction\":\"Name the pack\""), "{alpaca}");
        assert!(alpaca.contains("\"input\":\"overnight\""), "{alpaca}");
        assert!(
            alpaca.contains("\"output\":\"overnight-traces\""),
            "{alpaca}"
        );

        let ollama_out = root.join("ollama-only");
        let ollama_err = prepare_enrich(&PrepareEnrichRequest {
            estate: &estate,
            pack: &pack,
            curator: "jason",
            driver_id: "ollama-modelfile",
            job: "enrich",
            out_dir: &ollama_out,
            max_steps: None,
            from_feed: true,
            state_dir: &state,
        })
        .unwrap_err();
        assert!(
            ollama_err.to_string().contains("refuse:dataset"),
            "{ollama_err}"
        );
        assert!(
            ollama_err.to_string().contains("llamafactory-qlora"),
            "{ollama_err}"
        );
        assert!(!ollama_out.exists());
    }

    #[test]
    fn from_feed_refuses_frontier_metadata_before_shape() {
        let root = tmp("frontier-wrap");
        let pack = fixture_pack();
        let estate = with_train_base(seated_estate("llama3"), "Qwen/Qwen2.5-0.5B-Instruct");
        let mut local_only = estate.clone();
        local_only
            .model_bindings
            .retain(|binding| binding.class != ModelClass::Frontier);
        let state = root.join("cell");
        std::fs::create_dir_all(state.join("feed")).unwrap();
        let sharegpt = "{\"kind\":\"model.frontier.complete\",\"object_class\":\"frontier\",\"note\":null,\"messages\":[{\"role\":\"user\",\"content\":\"hello\"},{\"role\":\"assistant\",\"content\":\"world\"}]}\n";
        std::fs::write(state.join("feed/events.jsonl"), sharegpt).unwrap();
        let wrapped = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &local_only,
            &root.join("sharegpt"),
            &state,
            true,
        )
        .unwrap_err();
        assert!(
            wrapped.to_string().contains("refuse:frontier-invent"),
            "{wrapped}"
        );
        assert!(!root.join("sharegpt").exists());

        let alpaca = "{\"kind\":\"model.frontier.complete\",\"instruction\":\"Say hi\",\"output\":\"hi\"}\n";
        std::fs::write(state.join("feed/events.jsonl"), alpaca).unwrap();
        let alpaca_err = run_feed(
            AXOLOTL_LORA_ID,
            &pack,
            &local_only,
            &root.join("alpaca"),
            &state,
            true,
        )
        .unwrap_err();
        assert!(
            alpaca_err.to_string().contains("refuse:frontier-invent"),
            "{alpaca_err}"
        );
        assert!(!root.join("alpaca").exists());

        let sacred = "{\"kind\":\"model.local.precheck\",\"object_class\":\"local\",\"note\":\"mentions cyera\",\"messages\":[{\"role\":\"user\",\"content\":\"hello\"},{\"role\":\"assistant\",\"content\":\"world\"}]}\n";
        std::fs::write(state.join("feed/events.jsonl"), sacred).unwrap();
        let sacred_err = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &local_only,
            &root.join("sacred-wrap"),
            &state,
            true,
        )
        .unwrap_err();
        assert!(
            sacred_err.to_string().contains("refuse:sacred"),
            "{sacred_err}"
        );
        assert!(!root.join("sacred-wrap").exists());

        let sku = "{\"kind\":\"box-5090\",\"object_class\":\"local\",\"messages\":[{\"role\":\"user\",\"content\":\"hello\"},{\"role\":\"assistant\",\"content\":\"world\"}]}\n";
        std::fs::write(state.join("feed/events.jsonl"), sku).unwrap();
        let sku_err = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &local_only,
            &root.join("sku-wrap"),
            &state,
            true,
        )
        .unwrap_err();
        assert!(
            sku_err.to_string().contains("refuse:sku-banned"),
            "{sku_err}"
        );
        assert!(!root.join("sku-wrap").exists());

        let secret = "{\"kind\":\"model.local.precheck\",\"object_class\":\"local\",\"note\":\"api_key=abcd\",\"messages\":[{\"role\":\"user\",\"content\":\"hello\"},{\"role\":\"assistant\",\"content\":\"world\"}]}\n";
        std::fs::write(state.join("feed/events.jsonl"), secret).unwrap();
        let secret_err = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &local_only,
            &root.join("secret-wrap"),
            &state,
            true,
        )
        .unwrap_err();
        assert!(
            secret_err.to_string().contains("refuse:raw-secret"),
            "{secret_err}"
        );
        assert!(!root.join("secret-wrap").exists());

        std::fs::write(state.join("feed/events.jsonl"), sharegpt).unwrap();
        let allowed = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &estate,
            &root.join("bound"),
            &state,
            true,
        )
        .unwrap();
        assert_eq!(allowed.dataset_mode.as_deref(), Some("feed"));
        assert_eq!(allowed.dataset_rows, Some(1));
        let jsonl = std::fs::read_to_string(root.join("bound/dataset.jsonl")).unwrap();
        assert!(jsonl.contains("world"), "{jsonl}");
        assert!(!jsonl.contains("Replace this scaffold"), "{jsonl}");
    }

    #[test]
    fn from_feed_caps_aggregate_bytes_and_output() {
        assert!(refuse_dataset_output(DATASET_MAX_OUTPUT_BYTES).is_ok());
        let over = refuse_dataset_output(DATASET_MAX_OUTPUT_BYTES + 1).unwrap_err();
        let over_text = over.to_string();
        assert!(over_text.contains("refuse:dataset"), "{over_text}");
        assert!(
            over_text.contains(&DATASET_MAX_OUTPUT_BYTES.to_string()),
            "{over_text}"
        );

        let root = tmp("aggregate");
        let mut pack = fixture_pack();
        pack.source_paths = vec!["feed/a.jsonl".into(), "feed/b.jsonl".into()];
        let estate = with_train_base(seated_estate("llama3"), "Qwen/Qwen2.5-0.5B-Instruct");
        let state = root.join("cell");
        std::fs::create_dir_all(state.join("feed")).unwrap();
        let one = sized_alpaca((DATASET_MAX_TOTAL_BYTES / 2) as usize + 1024);
        assert!(one.len() as u64 <= DATASET_MAX_BYTES);
        assert!(one.len() as u64 * 2 > DATASET_MAX_TOTAL_BYTES);
        std::fs::write(state.join("feed/a.jsonl"), &one).unwrap();
        let mut only = pack.clone();
        only.source_paths = vec!["feed/a.jsonl".into()];
        let single = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &only,
            &estate,
            &root.join("one"),
            &state,
            true,
        )
        .unwrap();
        assert_eq!(single.dataset_rows, Some(1));
        assert_eq!(single.dataset_mode.as_deref(), Some("feed"));

        std::fs::write(state.join("feed/b.jsonl"), &one).unwrap();
        let both = root.join("both");
        let err = run_feed(
            AXOLOTL_LORA_ID,
            &pack,
            &estate,
            &both,
            &state,
            true,
        )
        .unwrap_err();
        let text = err.to_string();
        assert!(text.contains("refuse:dataset"), "{text}");
        assert!(text.contains("together"), "{text}");
        assert!(
            text.contains(&DATASET_MAX_TOTAL_BYTES.to_string()),
            "{text}"
        );
        assert!(!both.exists());
    }

    fn sized_alpaca(file_bytes: usize) -> String {
        let head = "{\"instruction\":\"q\",\"input\":\"\",\"output\":\"";
        let tail = "\"}\n";
        assert!(file_bytes > head.len() + tail.len());
        let fill = file_bytes - head.len() - tail.len();
        format!("{head}{}{tail}", "a".repeat(fill))
    }

    #[test]
    fn schema_snapshot_stays_prepare_only() {
        let snap: EnrichPrepareDoc =
            serde_json::from_str(include_str!("../../../schema/train-enrich.v0.json")).unwrap();
        assert_eq!(snap.schema, PREPARE_SCHEMA);
        assert!(!snap.promoted);
        assert!(!snap.auto_apply);
        assert!(!snap.estate_rewritten);
        assert!(snap.note.to_ascii_lowercase().contains("does not"));
        assert!(snap.artifacts.iter().any(|name| name == "Modelfile"));
    }

    #[test]
    fn prepare_all_writes_both_or_neither() {
        let root = tmp("all");
        let pack = fixture_pack();
        let estate = seated_estate("llama3");
        let ollama_out = root.join("ollama-modelfile");
        let manifest_out = root.join("external-manifest");
        let docs = prepare_enrich_set(&[
            PrepareEnrichRequest {
                estate: &estate,
                pack: &pack,
                curator: "jason",
                driver_id: "external-manifest",
                job: "enrich",
                out_dir: &manifest_out,
                max_steps: None,
                from_feed: false,
                state_dir: Path::new(".cell"),
            },
            PrepareEnrichRequest {
                estate: &estate,
                pack: &pack,
                curator: "jason",
                driver_id: "ollama-modelfile",
                job: "enrich",
                out_dir: &ollama_out,
                max_steps: None,
                from_feed: false,
                state_dir: Path::new(".cell"),
            },
        ])
        .unwrap();
        assert_eq!(docs.len(), 2);
        assert!(ollama_out.join("Modelfile").is_file());
        assert!(manifest_out.join("manifest.json").is_file());
        assert!(ollama_out.join("NEXT.md").is_file());
        let external = import_prepared(&ImportPreparedRequest {
            estate: &estate,
            prepared_dir: &manifest_out,
            tag: "cell-enrich-overnight-traces",
            path: &manifest_out.join("manifest.json"),
            curator: "jason",
        })
        .unwrap();
        assert_eq!(external.driver, "external-manifest");
        assert_eq!(external.binding_id, "local_slm");
        assert!(!external.auto_apply && !external.promoted && !external.estate_rewritten);
        assert!(manifest_out.join("binding-proposal.json").is_file());

        let mut fenced = pack.clone();
        fenced.note = "system fence \"\"\" here".into();
        let blocked_o = root.join("blocked-ollama");
        let blocked_m = root.join("blocked-manifest");
        let err = prepare_enrich_set(&[
            PrepareEnrichRequest {
                estate: &estate,
                pack: &fenced,
                curator: "jason",
                driver_id: "external-manifest",
                job: "enrich",
                out_dir: &blocked_m,
                max_steps: None,
                from_feed: false,
                state_dir: Path::new(".cell"),
            },
            PrepareEnrichRequest {
                estate: &estate,
                pack: &fenced,
                curator: "jason",
                driver_id: "ollama-modelfile",
                job: "enrich",
                out_dir: &blocked_o,
                max_steps: None,
                from_feed: false,
                state_dir: Path::new(".cell"),
            },
        ])
        .unwrap_err();
        assert!(err.to_string().contains("refuse:modelfile"), "{err}");
        assert!(
            !blocked_o.exists(),
            "ollama refuse must not leave a sibling"
        );
        assert!(
            !blocked_m.exists(),
            "a later refuse must not keep the earlier driver"
        );
    }

    #[test]
    fn list_reads_enrich_tree_and_refuses_when_missing() {
        let root = tmp("list");
        let enrich = root.join("enrich");
        let missing = list_prepared(&enrich).unwrap_err();
        assert!(
            missing.to_string().contains("refuse:enrich-index"),
            "{missing}"
        );
        assert!(!enrich.exists());

        let pack = fixture_pack();
        let estate = seated_estate("llama3");
        let out = default_enrich_out(&root, &pack.id, "ollama-modelfile");
        run("ollama-modelfile", &pack, &estate, &out, "enrich", "jason").unwrap();
        let manifest = default_enrich_out(&root, &pack.id, "external-manifest");
        run(
            "external-manifest",
            &pack,
            &estate,
            &manifest,
            "train",
            "jason",
        )
        .unwrap();
        let rows = list_prepared(&enrich).unwrap();
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert_eq!(rows[0].driver, "external-manifest");
        assert_eq!(rows[1].driver, "ollama-modelfile");
        assert_eq!(rows[1].local_tag, "cell-enrich-overnight-traces");
        let rendered = render_prepared_index(&enrich, &rows);
        assert!(rendered.contains("count=2"), "{rendered}");
        assert!(rendered.contains("promoted=false"), "{rendered}");

        let prepare_path = out.join("prepare.json");
        let mut doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&prepare_path).unwrap()).unwrap();
        doc["promoted"] = serde_json::Value::Bool(true);
        std::fs::write(&prepare_path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
        let tamper = list_prepared(&enrich).unwrap_err();
        assert!(tamper.to_string().contains("refuse:prepared"), "{tamper}");
    }

    #[test]
    fn import_prepared_writes_a_proposal_and_refuses_closed() {
        let root = tmp("import");
        let pack = fixture_pack();
        let estate = seated_estate("llama3");
        let prepared = root.join("prepared");
        run(
            "ollama-modelfile",
            &pack,
            &estate,
            &prepared,
            "enrich",
            "jason",
        )
        .unwrap();
        let modelfile = prepared.join("Modelfile");
        let before = std::fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap();
        let proposal = import_prepared(&ImportPreparedRequest {
            estate: &estate,
            prepared_dir: &prepared,
            tag: "cell-enrich-overnight-traces",
            path: &modelfile,
            curator: "jason",
        })
        .unwrap();
        assert!(!proposal.auto_apply && !proposal.promoted && !proposal.estate_rewritten);
        assert_eq!(proposal.binding_id, "local_slm");
        assert_eq!(proposal.seated_driver, "ollama");
        assert_eq!(proposal.schema, BINDING_PROPOSAL_SCHEMA);
        assert!(proposal.content_scanned);
        let model = proposal
            .proposed_binding
            .get("params")
            .and_then(|params| params.get("model"))
            .and_then(|value| value.as_str())
            .unwrap();
        assert_eq!(model, "cell-enrich-overnight-traces");
        assert!(proposal.paste_yaml.contains("id: local_slm"));
        assert!(prepared.join("binding-proposal.json").is_file());
        assert!(prepared.join("binding-proposal.md").is_file());
        let md = std::fs::read_to_string(prepared.join("binding-proposal.md")).unwrap();
        assert!(md.contains("estate apply --require-plan"), "{md}");
        assert!(md.contains("does not rewrite estate.yaml"), "{md}");
        assert_eq!(
            std::fs::read_to_string(repo_root().join("examples/estate.yaml")).unwrap(),
            before
        );
        assert!(!prepared.join("catalog.json").exists());

        let wrong = import_prepared(&ImportPreparedRequest {
            estate: &estate,
            prepared_dir: &prepared,
            tag: "other-tag",
            path: &modelfile,
            curator: "jason",
        })
        .unwrap_err();
        assert!(wrong.to_string().contains("refuse:tag"), "{wrong}");

        let sku = import_prepared(&ImportPreparedRequest {
            estate: &estate,
            prepared_dir: &prepared,
            tag: "rtx-5090",
            path: &modelfile,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            sku.to_string().contains("SKU") || sku.to_string().contains("sku"),
            "{sku}"
        );

        let sacred = import_prepared(&ImportPreparedRequest {
            estate: &estate,
            prepared_dir: &prepared,
            tag: "cyera",
            path: &modelfile,
            curator: "jason",
        })
        .unwrap_err();
        assert!(sacred.to_string().contains("refuse:sacred"), "{sacred}");

        let curator = import_prepared(&ImportPreparedRequest {
            estate: &estate,
            prepared_dir: &prepared,
            tag: "cell-enrich-overnight-traces",
            path: &modelfile,
            curator: "ada",
        })
        .unwrap_err();
        assert!(curator.to_string().contains("refuse:curator"), "{curator}");

        let missing_path = root.join("missing-weights.bin");
        let missing = import_prepared(&ImportPreparedRequest {
            estate: &estate,
            prepared_dir: &prepared,
            tag: "cell-enrich-overnight-traces",
            path: &missing_path,
            curator: "jason",
        })
        .unwrap_err();
        assert!(missing.to_string().contains("refuse:path"), "{missing}");

        let dirty = root.join("dirty.txt");
        std::fs::write(&dirty, "notes mention cyera\n").unwrap();
        let dirty_err = import_prepared(&ImportPreparedRequest {
            estate: &estate,
            prepared_dir: &prepared,
            tag: "cell-enrich-overnight-traces",
            path: &dirty,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            dirty_err.to_string().contains("refuse:sacred"),
            "{dirty_err}"
        );

        let weights = root.join("weights.bin");
        std::fs::write(&weights, [0xff, 0xfe, 0x00]).unwrap();
        let binary = import_prepared(&ImportPreparedRequest {
            estate: &estate,
            prepared_dir: &prepared,
            tag: "cell-enrich-overnight-traces",
            path: &weights,
            curator: "jason",
        })
        .unwrap();
        assert!(!binary.content_scanned);

        let mut local = fixture_estate();
        local
            .model_bindings
            .retain(|binding| binding.id != "local_slm");
        let no_seat = import_prepared(&ImportPreparedRequest {
            estate: &local,
            prepared_dir: &prepared,
            tag: "cell-enrich-overnight-traces",
            path: &modelfile,
            curator: "jason",
        })
        .unwrap_err();
        assert!(no_seat.to_string().contains("refuse:binding"), "{no_seat}");

        let mut tampered: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(prepared.join("prepare.json")).unwrap())
                .unwrap();
        tampered["source_drivers"] = serde_json::json!(["frontier"]);
        std::fs::write(
            prepared.join("prepare.json"),
            serde_json::to_string_pretty(&tampered).unwrap(),
        )
        .unwrap();
        local
            .model_bindings
            .retain(|binding| binding.class != ModelClass::Frontier);
        local.model_bindings.push(estate_schema::ModelBinding {
            id: "local_slm".into(),
            class: ModelClass::Local,
            driver: "ollama".into(),
            params: serde_json::json!({}),
            wired: true,
        });
        let frontier = import_prepared(&ImportPreparedRequest {
            estate: &local,
            prepared_dir: &prepared,
            tag: "cell-enrich-overnight-traces",
            path: &modelfile,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            frontier.to_string().contains("refuse:frontier-invent"),
            "{frontier}"
        );
    }

    #[test]
    fn seated_base_never_uses_the_binding_id() {
        let root = tmp("base");
        let stock = fixture_estate();
        let pack = fixture_pack();
        let stock_out = root.join("stock");
        let err = run(
            "ollama-modelfile",
            &pack,
            &stock,
            &stock_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:base-model"), "{err}");
        assert!(!stock_out.exists(), "binding id must not become FROM");

        let hinted = {
            let mut pack = fixture_pack();
            pack.model_hint = Some("llama3".into());
            pack
        };
        let hint_out = root.join("hint");
        let doc = run(
            "ollama-modelfile",
            &hinted,
            &stock,
            &hint_out,
            "enrich",
            "jason",
        )
        .unwrap();
        assert_eq!(doc.base_model, "llama3");
        let modelfile = std::fs::read_to_string(hint_out.join("Modelfile")).unwrap();
        assert!(modelfile.contains("FROM llama3\n"), "{modelfile}");
        assert!(!modelfile.contains("FROM local_slm"), "{modelfile}");

        let tagged = seated_estate("llama3:latest");
        let tag_out = root.join("tag");
        let doc = run(
            "ollama-modelfile",
            &pack,
            &tagged,
            &tag_out,
            "enrich",
            "jason",
        )
        .unwrap();
        assert_eq!(doc.base_model, "llama3:latest");
        let tagged_file = std::fs::read_to_string(tag_out.join("Modelfile")).unwrap();
        assert!(
            tagged_file.contains("FROM llama3:latest\n"),
            "{tagged_file}"
        );

        let mut driver_hint = fixture_pack();
        driver_hint.model_hint = Some("ollama".into());
        let driver_out = root.join("driver-hint");
        let doc = run(
            "external-manifest",
            &driver_hint,
            &tagged,
            &driver_out,
            "enrich",
            "jason",
        )
        .unwrap();
        assert_eq!(doc.base_model, "llama3:latest");

        let mut empty_hint = fixture_pack();
        empty_hint.model_hint = None;
        let empty_out = root.join("empty-hint");
        let doc = run(
            "ollama-modelfile",
            &empty_hint,
            &seated_estate("llama3"),
            &empty_out,
            "enrich",
            "jason",
        )
        .unwrap();
        assert_eq!(doc.base_model, "llama3");

        let bound_id = seated_estate("local_slm");
        let bound_out = root.join("bound-id");
        let err = run(
            "ollama-modelfile",
            &pack,
            &bound_id,
            &bound_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:base-model"), "{err}");
        assert!(!bound_out.exists());

        let sku_out = root.join("sku-param");
        let err = run(
            "ollama-modelfile",
            &pack,
            &seated_estate("rtx-5090"),
            &sku_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("SKU") || err.to_string().contains("sku"),
            "{err}"
        );
        assert!(!sku_out.exists());

        let mut frontier_hint = fixture_pack();
        frontier_hint.model_hint = Some("xai_grok".into());
        let frontier_out = root.join("frontier-hint");
        let err = run(
            "ollama-modelfile",
            &frontier_hint,
            &seated_estate("llama3"),
            &frontier_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:base-model"), "{err}");
        assert!(!frontier_out.exists());
    }

    #[test]
    fn binding_proposal_snapshot_stays_unapplied() {
        let snap: EnrichBindingProposal = serde_json::from_str(include_str!(
            "../../../schema/enrich-binding-proposal.v0.json"
        ))
        .unwrap();
        assert_eq!(snap.schema, BINDING_PROPOSAL_SCHEMA);
        assert!(!snap.auto_apply && !snap.promoted && !snap.estate_rewritten);
        assert_eq!(snap.binding_id, "local_slm");
        assert!(snap.note.to_ascii_lowercase().contains("does not"));
    }

    #[test]
    fn apply_proposal_stages_plan_input_and_require_plan_writes_the_source() {
        let root = tmp("stage");
        let pack = fixture_pack();
        let estate = seated_estate("llama3");
        let state = root.join("state");
        let prepared = default_enrich_out(&state, &pack.id, "ollama-modelfile");
        run(
            "ollama-modelfile",
            &pack,
            &estate,
            &prepared,
            "enrich",
            "jason",
        )
        .unwrap();
        let from = std::fs::read_to_string(prepared.join("Modelfile")).unwrap();
        assert!(from.contains("FROM llama3\n"), "{from}");
        assert!(!from.contains("FROM local_slm"), "{from}");
        let source = root.join("estate.yaml");
        let example = repo_root().join("examples/estate.yaml");
        let seated_yaml = estate_schema::render_estate_yaml(&estate).unwrap();
        std::fs::write(&source, &seated_yaml).unwrap();
        let before = std::fs::read(&source).unwrap();
        let example_before = std::fs::read(&example).unwrap();
        let modelfile = prepared.join("Modelfile");
        import_prepared(&ImportPreparedRequest {
            estate: &estate,
            prepared_dir: &prepared,
            tag: "cell-enrich-overnight-traces",
            path: &modelfile,
            curator: "jason",
        })
        .unwrap();

        fn req<'a>(
            estate: &'a Estate,
            source: &'a Path,
            prepared: &'a Path,
            tag: &'a str,
            state_dir: &'a Path,
            verify: Option<&'a str>,
        ) -> ApplyProposalRequest<'a> {
            ApplyProposalRequest {
                estate,
                estate_path: source,
                prepared_dir: prepared,
                tag,
                curator: "jason",
                state_dir,
                verify_endpoint: verify,
            }
        }

        let missing = apply_proposal(&ApplyProposalRequest {
            estate: &estate,
            estate_path: &source,
            prepared_dir: &root.join("no-proposal"),
            tag: "cell-enrich-overnight-traces",
            curator: "jason",
            state_dir: &root.join("missing-state"),
            verify_endpoint: None,
        })
        .unwrap_err();
        assert!(
            missing.to_string().contains("refuse:missing-proposal"),
            "{missing}"
        );
        assert!(!root.join("missing-state").join("enrich-stage").exists());

        let wrong = apply_proposal(&req(&estate, &source, &prepared, "other-tag", &state, None))
            .unwrap_err();
        assert!(wrong.to_string().contains("refuse:tag"), "{wrong}");
        assert!(!state.join("enrich-stage").exists());

        let curator = apply_proposal(&ApplyProposalRequest {
            curator: "ada",
            ..req(
                &estate,
                &source,
                &prepared,
                "cell-enrich-overnight-traces",
                &state,
                None,
            )
        })
        .unwrap_err();
        assert!(curator.to_string().contains("refuse:curator"), "{curator}");

        let prepare_path = prepared.join("prepare.json");
        let prepare_body = std::fs::read_to_string(&prepare_path).unwrap();
        let mut tampered: serde_json::Value = serde_json::from_str(&prepare_body).unwrap();
        tampered["job"] = serde_json::json!("train");
        std::fs::write(
            &prepare_path,
            serde_json::to_string_pretty(&tampered).unwrap(),
        )
        .unwrap();
        let mismatch = apply_proposal(&req(
            &estate,
            &source,
            &prepared,
            "cell-enrich-overnight-traces",
            &state,
            None,
        ))
        .unwrap_err();
        assert!(
            mismatch.to_string().contains("refuse:prepare"),
            "{mismatch}"
        );
        std::fs::write(&prepare_path, &prepare_body).unwrap();
        assert!(!state.join("enrich-stage").exists());

        let proposal_path = prepared.join("binding-proposal.json");
        let proposal_body = std::fs::read_to_string(&proposal_path).unwrap();
        let mut frontier: serde_json::Value = serde_json::from_str(&proposal_body).unwrap();
        frontier["proposed_binding"]["class"] = serde_json::json!("frontier");
        std::fs::write(
            &proposal_path,
            serde_json::to_string_pretty(&frontier).unwrap(),
        )
        .unwrap();
        let frontier_err = apply_proposal(&req(
            &estate,
            &source,
            &prepared,
            "cell-enrich-overnight-traces",
            &state,
            None,
        ))
        .unwrap_err();
        assert!(
            frontier_err.to_string().contains("refuse:frontier-invent"),
            "{frontier_err}"
        );
        std::fs::write(&proposal_path, &proposal_body).unwrap();

        let mut renamed = estate.clone();
        renamed.name = "other-cell".into();
        let stale = apply_proposal(&req(
            &renamed,
            &source,
            &prepared,
            "cell-enrich-overnight-traces",
            &state,
            None,
        ))
        .unwrap_err();
        assert!(stale.to_string().contains("refuse:estate-hash"), "{stale}");
        assert!(!state.join("enrich-stage").exists());
        assert_eq!(std::fs::read(&source).unwrap(), before);

        let down_state = root.join("down-state");
        let down = apply_proposal(&req(
            &estate,
            &source,
            &prepared,
            "cell-enrich-overnight-traces",
            &down_state,
            Some("http://127.0.0.1:1"),
        ))
        .unwrap_err();
        assert!(down.to_string().contains("refuse:local-tag"), "{down}");
        assert!(down.to_string().contains("down"), "{down}");
        assert!(!root.join("down-state").join("enrich-stage").exists());

        let server = crate::mock::CompatServer::spawn(crate::mock::CompatScript::OpenAi {
            models: vec!["cell-enrich-overnight-traces".into()],
        })
        .unwrap();
        let endpoint = server.endpoint();
        let outcome = apply_proposal(&req(
            &estate,
            &source,
            &prepared,
            "cell-enrich-overnight-traces",
            &state,
            Some(&endpoint),
        ))
        .unwrap();
        let stage = match outcome {
            ApplyProposalOutcome::Staged(stage) => stage,
            ApplyProposalOutcome::Noop { reason } => panic!("{reason}"),
        };
        assert!(!stage.auto_apply && !stage.promoted && !stage.applied);
        assert!(!stage.estate_rewritten);
        assert_eq!(stage.schema, BINDING_STAGE_SCHEMA);
        assert_eq!(stage.binding_id, "local_slm");
        assert_eq!(stage.local_tag, "cell-enrich-overnight-traces");
        assert_eq!(std::fs::read(&source).unwrap(), before);
        let staged_yaml =
            std::fs::read_to_string(state.join("enrich-stage/staged-estate.yaml")).unwrap();
        let staged_estate = estate_schema::load_estate_str(&staged_yaml).unwrap();
        assert_eq!(
            local_slm_model_param(&staged_estate).as_deref(),
            Some("cell-enrich-overnight-traces")
        );
        let facts = enrich_join_facts(&state.join("enrich")).unwrap().unwrap();
        assert!(facts.iter().any(
            |fact| fact.kind == "proposal" && fact.local_tag == "cell-enrich-overnight-traces"
        ));

        let again = apply_proposal(&req(
            &estate,
            &source,
            &prepared,
            "cell-enrich-overnight-traces",
            &state,
            None,
        ))
        .unwrap();
        assert!(
            matches!(again, ApplyProposalOutcome::Noop { .. }),
            "{again:?}"
        );
        assert_eq!(std::fs::read(&source).unwrap(), before);

        let staged_path = state.join("enrich-stage/staged-estate.yaml");
        let hash = estate_schema::estate_hash(&staged_estate);
        let bad_hash = commit_enrich_stage(&staged_path, &state, true, "sha256:nope").unwrap_err();
        assert!(bad_hash.to_string().contains("refuse:stage"), "{bad_hash}");
        assert_eq!(std::fs::read(&source).unwrap(), before);

        let held = commit_enrich_stage(&staged_path, &state, false, &hash).unwrap();
        assert_eq!(held, EnrichStageCommit::Held);
        assert_eq!(std::fs::read(&source).unwrap(), before);

        let wrote = commit_enrich_stage(&staged_path, &state, true, &hash).unwrap();
        match wrote {
            EnrichStageCommit::Wrote { source: path } => {
                assert_eq!(path, std::fs::canonicalize(&source).unwrap());
            }
            other => panic!("{other:?}"),
        }
        let after = std::fs::read_to_string(&source).unwrap();
        assert!(after.contains("cell-enrich-overnight-traces"), "{after}");
        assert_ne!(after.as_bytes(), before.as_slice());
        let bound = estate_schema::load_estate_str(&after).unwrap();
        assert_eq!(
            local_slm_model_param(&bound).as_deref(),
            Some("cell-enrich-overnight-traces")
        );
        let receipt: EnrichBindingStage = serde_json::from_str(
            &std::fs::read_to_string(state.join("enrich-stage/stage.json")).unwrap(),
        )
        .unwrap();
        assert!(receipt.applied && receipt.estate_rewritten && !receipt.auto_apply);
        let already = apply_proposal(&req(
            &bound,
            &source,
            &prepared,
            "cell-enrich-overnight-traces",
            &state,
            None,
        ))
        .unwrap();
        assert!(
            matches!(already, ApplyProposalOutcome::Noop { ref reason } if reason.contains("already bound")),
            "{already:?}"
        );
        let ignored = commit_enrich_stage(&source, &state, true, &hash).unwrap();
        assert_eq!(ignored, EnrichStageCommit::NotThisEstate);
        assert_eq!(std::fs::read(&example).unwrap(), example_before);
    }

    #[test]
    fn binding_stage_snapshot_stays_unapplied() {
        let snap: EnrichBindingStage =
            serde_json::from_str(include_str!("../../../schema/enrich-binding-stage.v0.json"))
                .unwrap();
        assert_eq!(snap.schema, BINDING_STAGE_SCHEMA);
        assert!(!snap.auto_apply && !snap.promoted && !snap.applied && !snap.estate_rewritten);
        assert_eq!(snap.binding_id, "local_slm");
    }
}
