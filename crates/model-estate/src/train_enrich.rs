//! Train/enrich facilitation. Prepare artifacts for a purpose-built SLM.
//!
//! The local runtime stays a seat. Ollama is today's entrant. This module
//! does not shell out, does not POST a train job, and does not rewrite the
//! estate. `llamafactory-lora` writes the LLaMA-Factory LoRA recipe (16-bit
//! base, no quantization). `llamafactory-qlora` writes the QLoRA recipe.
//! The operator runs either recipe outside the factory. `axolotl-lora` writes
//! the bf16 Axolotl LoRA YAML. `axolotl-qlora` writes the 4-bit Axolotl YAML.
//! Every recipe card writes the train base, and keeps
//! the Ollama seat tag for Modelfile `FROM`. `unsloth-qlora` is an optional
//! NEXT card: an Nvidia-only QLoRA handoff. It does not write a script.
//! `mlx-lm-lora` is an optional NEXT card: an Apple Silicon LoRA handoff.
//! It writes docs only when `host_class_affinity` is `apple-silicon`.
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
    /// Hugging Face repo id or a local HF weights directory. `llamafactory-lora`
    /// and `llamafactory-qlora` write this to `model_name_or_path`.
    /// `axolotl-lora` and `axolotl-qlora` write it to `base_model`.
    /// `unsloth-qlora` and `mlx-lm-lora` record it on the operator-owned handoff.
    /// Those cards do not write a recipe.
    /// Absent until the pack or the binding sets it.
    pub train_base_model: Option<String>,
    pub purpose: String,
    pub system_text: String,
    pub host_class_affinity: String,
    pub source_paths: Vec<String>,
    pub source_drivers: Vec<String>,
    /// Directory the artifacts will occupy. Path-bearing recipes use it for absolute paths.
    pub out_dir: PathBuf,
    /// Explicit gauge cap. LLaMA-Factory and Axolotl both write `max_steps`. `None` leaves it unset.
    pub max_steps: Option<u32>,
    /// Official SFT scale from LLaMA-Factory `examples/train_lora/qwen3_lora_sft.yaml`.
    /// `--official-scale` applies to LLaMA-Factory cards only: cutoff 2048, epochs 3.0, grad accum 8, warmup 0.1.
    /// Axolotl stays on its example files. `false` keeps the short LLaMA-Factory recipe.
    pub official_scale: bool,
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
            notes: "QLoRA train card. Writes recipe.yaml (LLaMA-Factory SFT QLoRA, 4-bit bitsandbytes). model_name_or_path is the train base (HF repo or local HF weights), separate from the Ollama seat tag. Default job is train. Does not shell out. 16-bit LoRA is llamafactory-lora. Train hosts are consumer-nvidia and rented-nvidia.",
            jobs: TRAIN_ONLY,
            default_job: EnrichJobKind::Train,
        },
        build: || Box::new(LlamaFactoryDriver { method: LlamaFactoryMethod::Qlora }),
    },
    RegisteredDriver {
        card: TrainEnrichCard {
            driver_id: LLAMAFACTORY_LORA_ID,
            status: "integration",
            integrates: "llamafactory-cli train LoRA recipe",
            notes: "LoRA train card. Writes recipe.yaml (LLaMA-Factory SFT LoRA, no quantization, rank 8). model_name_or_path is the train base (HF repo or local HF weights), separate from the Ollama seat tag. Default job is train. Does not shell out. Does not require bitsandbytes. Train hosts are consumer-nvidia and rented-nvidia.",
            jobs: TRAIN_ONLY,
            default_job: EnrichJobKind::Train,
        },
        build: || Box::new(LlamaFactoryDriver { method: LlamaFactoryMethod::Lora }),
    },
    RegisteredDriver {
        card: TrainEnrichCard {
            driver_id: AXOLOTL_LORA_ID,
            status: "integration",
            integrates: "axolotl train LoRA recipe",
            notes: "bf16 LoRA YAML for a config-driven or multi-GPU run. Writes axolotl.yml (adapter lora, load_in_4bit false) and dataset.jsonl. sequence_len, micro_batch_size, gradient_accumulation_steps, and lora_r match examples/llama-3/lora-1b.yml. base_model is the train base (HF repo or local HF weights), separate from the Ollama seat tag. Default job is train. Does not shell out. After train, merge-adapt prints axolotl merge-lora. Axolotl writes output_dir/merged. gguf-convert and local-seat print the llama.cpp and ollama lines. This card does not merge and does not write GGUF. 4-bit QLoRA is axolotl-qlora.",
            jobs: TRAIN_ONLY,
            default_job: EnrichJobKind::Train,
        },
        build: || Box::new(AxolotlDriver { method: AxolotlMethod::Lora }),
    },
    RegisteredDriver {
        card: TrainEnrichCard {
            driver_id: AXOLOTL_QLORA_ID,
            status: "integration",
            integrates: "axolotl train QLoRA recipe",
            notes: "4-bit QLoRA YAML for a config-driven or multi-GPU run. Writes axolotl.yml (adapter qlora, load_in_4bit true) and dataset.jsonl. sequence_len, micro_batch_size, gradient_accumulation_steps, and lora_r match examples/llama-3/qlora.yml. base_model is the train base (HF repo or local HF weights), separate from the Ollama seat tag. Default job is train. Does not shell out. After train, merge-adapt prints axolotl merge-lora, including the CLI --dequant line that writes a bf16 checkpoint. Axolotl writes output_dir/merged. gguf-convert and local-seat print the llama.cpp and ollama lines. This card does not merge and does not write GGUF. bf16 LoRA is axolotl-lora.",
            jobs: TRAIN_ONLY,
            default_job: EnrichJobKind::Train,
        },
        build: || Box::new(AxolotlDriver { method: AxolotlMethod::Qlora }),
    },
    RegisteredDriver {
        card: TrainEnrichCard {
            driver_id: UNSLOTH_QLORA_ID,
            status: "optional",
            integrates: "Unsloth QLoRA docs (Nvidia-only NEXT handoff)",
            notes: "Optional NEXT card. Nvidia-only QLoRA alternate for a faster single-GPU run. Writes UNSLOTH.md, an operator-owned handoff. Does not write a script, a recipe, or dataset.jsonl. Does not shell out. Does not call Unsloth. Not the product. LLaMA-Factory QLoRA is llamafactory-qlora. Axolotl QLoRA is axolotl-qlora.",
            jobs: TRAIN_ONLY,
            default_job: EnrichJobKind::Train,
        },
        build: || Box::new(UnslothQloraDriver),
    },
    RegisteredDriver {
        card: TrainEnrichCard {
            driver_id: MLX_LM_LORA_ID,
            status: "optional",
            integrates: "mlx-lm LoRA docs (apple-silicon NEXT handoff)",
            notes: "Optional NEXT card. Apple Silicon LoRA handoff. Writes MLX.md, an operator-owned handoff, only when host_class_affinity is apple-silicon. Another affinity is refuse:host and writes nothing. Does not write a script, a recipe, or dataset.jsonl. Does not shell out. Does not call mlx-lm. After train, merge-adapt prints mlx_lm.fuse and local-seat prints the GGUF file from mlx_lm.fuse --export-gguf. Not the product. LLaMA-Factory LoRA is llamafactory-lora. Axolotl LoRA is axolotl-lora.",
            jobs: TRAIN_ONLY,
            default_job: EnrichJobKind::Train,
        },
        build: || Box::new(MlxLmLoraDriver),
    },
];

struct OllamaModelfileDriver;

struct ExternalManifestDriver;

struct LlamaFactoryDriver {
    method: LlamaFactoryMethod,
}

struct AxolotlDriver {
    method: AxolotlMethod,
}

struct UnslothQloraDriver;

struct MlxLmLoraDriver;

/// QLoRA train card. LLaMA-Factory already trains 4-bit QLoRA from a YAML recipe. This id writes that recipe.
pub const LLAMAFACTORY_QLORA_ID: &str = "llamafactory-qlora";

/// LoRA train card. LLaMA-Factory already trains unquantized LoRA from a YAML recipe. This id writes that recipe.
pub const LLAMAFACTORY_LORA_ID: &str = "llamafactory-lora";

/// bf16 LoRA YAML card. Axolotl already trains that shape from `examples/llama-3/lora-1b.yml`.
pub const AXOLOTL_LORA_ID: &str = "axolotl-lora";

/// 4-bit QLoRA YAML card. Axolotl already trains that shape from `examples/llama-3/qlora.yml`.
pub const AXOLOTL_QLORA_ID: &str = "axolotl-qlora";

/// Optional NEXT card. Unsloth already documents Nvidia QLoRA. This id writes a handoff, not a script.
pub const UNSLOTH_QLORA_ID: &str = "unsloth-qlora";

/// Operator-owned pointer. Not an Unsloth config and not a training script.
const UNSLOTH_HANDOFF: &str = "UNSLOTH.md";

const UNSLOTH_INSTALL_DOC: &str = "https://unsloth.ai/docs/get-started/install";
const UNSLOTH_GUIDE_DOC: &str = "https://unsloth.ai/docs/get-started/fine-tuning-llms-guide";
const UNSLOTH_REPO: &str = "https://github.com/unslothai/unsloth";
/// Linux install line published in the Unsloth README. This factory does not run it.
const UNSLOTH_README_INSTALL: &str = "uv pip install unsloth --torch-backend=auto";

/// Optional NEXT card. mlx-lm already documents LoRA on Apple Silicon. This id writes a handoff, not a script.
pub const MLX_LM_LORA_ID: &str = "mlx-lm-lora";

/// Operator-owned pointer. Not an mlx-lm config and not a training script.
const MLX_HANDOFF: &str = "MLX.md";

const APPLE_SILICON_HOST: &str = "apple-silicon";

/// Public LoRA page. This factory does not fetch it.
pub(crate) const MLX_LORA_DOC: &str =
    "https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md";
const MLX_REPO: &str = "https://github.com/ml-explore/mlx-lm";
/// Training extra published in mlx-lm `mlx_lm/LORA.md`. This factory does not run it.
const MLX_TRAIN_INSTALL: &str = "pip install \"mlx-lm[train]\"";
/// Train command name published on that page. This factory does not choose its flags.
const MLX_LORA_COMMAND: &str = "mlx_lm.lora";
/// Fuse command published on that page. The placeholder stays a placeholder.
pub(crate) const MLX_FUSE_COMMAND: &str = "mlx_lm.fuse --model <path_to_model>";

/// Which LLaMA-Factory PEFT recipe a card writes. Selection is the driver id.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LlamaFactoryMethod {
    /// 4-bit bitsandbytes QLoRA (`quantization_bit: 4`, `quantization_method: bnb`).
    Qlora,
    /// 16-bit LoRA. The recipe omits quantization.
    Lora,
}

impl LlamaFactoryMethod {
    fn driver_id(self) -> &'static str {
        match self {
            Self::Qlora => LLAMAFACTORY_QLORA_ID,
            Self::Lora => LLAMAFACTORY_LORA_ID,
        }
    }

    /// Official `examples/train_lora/qwen3_lora_sft.yaml` uses rank 8.
    /// The QLoRA card keeps rank 16.
    fn lora_rank(self) -> u32 {
        match self {
            Self::Qlora => 16,
            Self::Lora => 8,
        }
    }

    /// LLaMA-Factory uses `lora_rank * 2` when `lora_alpha` is unset.
    fn lora_alpha(self) -> u32 {
        self.lora_rank() * 2
    }

    /// The LoRA quickstart leaves `packing` unset. LLaMA-Factory's default is false.
    /// QLoRA keeps packing on.
    fn packing(self) -> bool {
        matches!(self, Self::Qlora)
    }
}

fn llamafactory_method(driver_id: &str) -> Option<LlamaFactoryMethod> {
    match driver_id {
        LLAMAFACTORY_QLORA_ID => Some(LlamaFactoryMethod::Qlora),
        LLAMAFACTORY_LORA_ID => Some(LlamaFactoryMethod::Lora),
        _ => None,
    }
}

fn is_llamafactory_driver(id: &str) -> bool {
    llamafactory_method(id).is_some()
}

/// Which Axolotl PEFT recipe a card writes. Selection is the driver id.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AxolotlMethod {
    /// bf16 LoRA. `examples/llama-3/lora-1b.yml`.
    Lora,
    /// 4-bit QLoRA. `examples/llama-3/qlora.yml`.
    Qlora,
}

impl AxolotlMethod {
    fn driver_id(self) -> &'static str {
        match self {
            Self::Lora => AXOLOTL_LORA_ID,
            Self::Qlora => AXOLOTL_QLORA_ID,
        }
    }

    fn example_file(self) -> &'static str {
        match self {
            Self::Lora => "examples/llama-3/lora-1b.yml",
            Self::Qlora => "examples/llama-3/qlora.yml",
        }
    }

    fn adapter(self) -> &'static str {
        match self {
            Self::Lora => "lora",
            Self::Qlora => "qlora",
        }
    }

    fn load_in_4bit(self) -> bool {
        matches!(self, Self::Qlora)
    }

    /// `lora-1b.yml` sequence length. `qlora.yml` uses 4096.
    fn sequence_len(self) -> u32 {
        match self {
            Self::Lora => 2048,
            Self::Qlora => 4096,
        }
    }

    /// Both public examples use 2.
    fn micro_batch_size(self) -> u32 {
        match self {
            Self::Lora | Self::Qlora => 2,
        }
    }

    /// `lora-1b.yml` uses 2. `qlora.yml` uses 4.
    fn gradient_accumulation_steps(self) -> u32 {
        match self {
            Self::Lora => 2,
            Self::Qlora => 4,
        }
    }

    /// `lora-1b.yml` uses 16. `qlora.yml` uses 32.
    fn lora_r(self) -> u32 {
        match self {
            Self::Lora => 16,
            Self::Qlora => 32,
        }
    }

    /// `lora-1b.yml` uses 32. `qlora.yml` uses 16.
    fn lora_alpha(self) -> u32 {
        match self {
            Self::Lora => 32,
            Self::Qlora => 16,
        }
    }

    /// `lora-1b.yml` is one epoch. `qlora.yml` is four.
    fn num_epochs(self) -> u32 {
        match self {
            Self::Lora => 1,
            Self::Qlora => 4,
        }
    }

    fn optimizer(self) -> &'static str {
        match self {
            Self::Lora => "adamw_8bit",
            Self::Qlora => "paged_adamw_32bit",
        }
    }
}

fn axolotl_method(driver_id: &str) -> Option<AxolotlMethod> {
    match driver_id {
        AXOLOTL_LORA_ID => Some(AxolotlMethod::Lora),
        AXOLOTL_QLORA_ID => Some(AxolotlMethod::Qlora),
        _ => None,
    }
}

pub(crate) fn is_axolotl_driver(id: &str) -> bool {
    axolotl_method(id).is_some()
}

/// Print-only Hugging Face merge ladder for `merge-adapt`, `gguf-convert`,
/// and `local-seat`. LLaMA-Factory merges with `llamafactory-cli export`.
/// Axolotl merges with `axolotl merge-lora` into `output_dir/merged`.
/// `unsloth-qlora` stays off this ladder. `mlx-lm-lora` prints `mlx_lm.fuse`
/// on its own path and does not use `convert_hf_to_gguf.py`.
pub(crate) fn is_post_merge_print_driver(id: &str) -> bool {
    is_llamafactory_driver(id) || is_axolotl_driver(id)
}

pub(crate) fn refuse_post_merge_driver(command: &str, found: &str) -> ModelError {
    ModelError::Other(format!(
        "refuse:driver: {command} reads a llamafactory-lora, llamafactory-qlora, axolotl-lora, or axolotl-qlora prepare, found '{found}'"
    ))
}

const TRAIN_RECIPE_DRIVERS: &[&str] = &[
    LLAMAFACTORY_QLORA_ID,
    LLAMAFACTORY_LORA_ID,
    AXOLOTL_LORA_ID,
    AXOLOTL_QLORA_ID,
];

/// `import-trained` accepts a recipe card or an optional handoff (`unsloth-qlora`, `mlx-lm-lora`).
const IMPORT_TRAINED_DRIVERS: &[&str] = &[
    LLAMAFACTORY_QLORA_ID,
    LLAMAFACTORY_LORA_ID,
    AXOLOTL_LORA_ID,
    AXOLOTL_QLORA_ID,
    UNSLOTH_QLORA_ID,
    MLX_LM_LORA_ID,
];

/// `import-trained` shape: LLaMA-Factory or Axolotl `output_dir` with `adapter_config.json`.
const TRAINED_SHAPE_ADAPTER: &str = "adapter";
/// `import-trained` shape: merged `export_dir` with `config.json` and a non-adapter safetensors file.
const TRAINED_SHAPE_MERGED: &str = "merged";
/// `import-trained` shape: one `.gguf` file, or a directory with exactly one.
const TRAINED_SHAPE_GGUF: &str = "gguf";

const TRAINED_SHAPE_HINT: &str = "import-trained accepts an adapter output_dir (a directory with adapter_config.json), a merged export_dir (a directory with config.json and at least one .safetensors file whose name does not start with adapter_model, plus an optional Modelfile), or a GGUF path (one .gguf file, or a directory with exactly one top-level .gguf). Marker symlinks are refused";

fn is_train_recipe_driver(id: &str) -> bool {
    TRAIN_RECIPE_DRIVERS.contains(&id)
}

/// Recipe cards and the optional handoffs record a train base beside the seat tag.
fn records_train_base(id: &str) -> bool {
    is_train_recipe_driver(id) || id == UNSLOTH_QLORA_ID || id == MLX_LM_LORA_ID
}

/// Smoke-scale cutoff. `--official-scale` writes 2048.
const LLAMAFACTORY_CUTOFF_LEN: u32 = 512;
/// `examples/train_lora/qwen3_lora_sft.yaml` cutoff_len.
const LLAMAFACTORY_OFFICIAL_CUTOFF_LEN: u32 = 2048;
const LLAMAFACTORY_SMOKE_GRAD_ACCUM: u32 = 4;
const LLAMAFACTORY_OFFICIAL_GRAD_ACCUM: u32 = 8;
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

impl TrainEnrichDriver for LlamaFactoryDriver {
    fn id(&self) -> &'static str {
        self.method.driver_id()
    }

    fn status(&self) -> &'static str {
        "integration"
    }

    fn prepare(&self, job: &EnrichJob) -> Result<DriverPrepare, ModelError> {
        let driver_id = self.method.driver_id();
        if job.kind != EnrichJobKind::Train {
            return Err(ModelError::Other(format!(
                "refuse:job: {driver_id} prepares train; got {}",
                job.kind.as_str()
            )));
        }
        let train_owned = require_train_base(job)?;
        let train_base = train_owned.as_str();
        let data = require_dataset(job)?;
        let dataset = data.chat_jsonl.clone();
        let recipe = llamafactory_recipe_yaml(job, &data.mode, train_base, self.method);
        let export = llamafactory_export_yaml(job, train_base, driver_id);
        let info = llamafactory_dataset_info();
        let steps = llamafactory_prepare_steps(self.method, job, train_base, data);
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

impl TrainEnrichDriver for AxolotlDriver {
    fn id(&self) -> &'static str {
        self.method.driver_id()
    }

    fn status(&self) -> &'static str {
        "integration"
    }

    fn prepare(&self, job: &EnrichJob) -> Result<DriverPrepare, ModelError> {
        let driver_id = self.method.driver_id();
        if job.kind != EnrichJobKind::Train {
            return Err(ModelError::Other(format!(
                "refuse:job: {driver_id} prepares train; got {}",
                job.kind.as_str()
            )));
        }
        let train_owned = require_train_base(job)?;
        let train_base = train_owned.as_str();
        let data = require_dataset(job)?;
        let dataset = data.alpaca_jsonl.clone();
        let yaml = axolotl_recipe_yaml(job, &data.mode, train_base, self.method);
        let steps = axolotl_prepare_steps(self.method, job, train_base, data);
        Ok(DriverPrepare {
            files: vec![
                ("axolotl.yml".into(), yaml),
                ("dataset.jsonl".into(), dataset),
            ],
            steps,
        })
    }
}

impl TrainEnrichDriver for UnslothQloraDriver {
    fn id(&self) -> &'static str {
        UNSLOTH_QLORA_ID
    }

    fn status(&self) -> &'static str {
        "optional"
    }

    fn prepare(&self, job: &EnrichJob) -> Result<DriverPrepare, ModelError> {
        if job.kind != EnrichJobKind::Train {
            return Err(ModelError::Other(format!(
                "refuse:job: {UNSLOTH_QLORA_ID} prepares train; got {}",
                job.kind.as_str()
            )));
        }
        let train_owned = require_train_base(job)?;
        let train_base = train_owned.as_str();
        let handoff = unsloth_handoff_md(job, train_base);
        let steps = unsloth_prepare_steps(job, train_base);
        Ok(DriverPrepare {
            files: vec![(UNSLOTH_HANDOFF.into(), handoff)],
            steps,
        })
    }
}

impl TrainEnrichDriver for MlxLmLoraDriver {
    fn id(&self) -> &'static str {
        MLX_LM_LORA_ID
    }

    fn status(&self) -> &'static str {
        "optional"
    }

    fn prepare(&self, job: &EnrichJob) -> Result<DriverPrepare, ModelError> {
        if job.kind != EnrichJobKind::Train {
            return Err(ModelError::Other(format!(
                "refuse:job: {MLX_LM_LORA_ID} prepares train; got {}",
                job.kind.as_str()
            )));
        }
        refuse_mlx_host(&job.host_class_affinity)?;
        let train_owned = require_train_base(job)?;
        let train_base = train_owned.as_str();
        let handoff = mlx_handoff_md(job, train_base);
        let steps = mlx_prepare_steps(job, train_base);
        Ok(DriverPrepare {
            files: vec![(MLX_HANDOFF.into(), handoff)],
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
    /// Path of `export.yaml` when this prepare wrote it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_yaml: Option<String>,
    /// Path of `Modelfile` when this prepare wrote it.
    /// LLaMA-Factory writes its later Modelfile into `export_dir`, not here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modelfile: Option<String>,
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
    /// `adapter`, `merged`, or `gguf` after `import-trained` accepts an artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trained_shape: Option<String>,
    /// Operator path plus the marker files that proved `trained_shape`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trained_paths: Option<Vec<String>>,
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
    /// `--official-scale` applies to LLaMA-Factory cards only: cutoff 2048, epochs 3.0, grad accum 8, warmup 0.1.
    /// Axolotl stays on its example files. `false` keeps the short LLaMA-Factory recipe.
    /// `--max-steps` still overrides epochs.
    pub official_scale: bool,
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

/// Cards whose `jobs` list contains `job`. This is the allow-list.
/// `--all-drivers` then applies `train_enrich_drivers_for_prepare`.
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

/// Host affinity recorded on a prepare job. Pack field, then `params.host_class`, then pack `host_class`.
pub fn enrich_host_class_affinity(estate: &Estate, pack: &PackManifest) -> String {
    resolve_host_class_affinity(estate, pack)
}

fn driver_included_for_host(driver_id: &str, host_class_affinity: &str) -> bool {
    if driver_id == MLX_LM_LORA_ID {
        host_class_affinity == APPLE_SILICON_HOST
    } else {
        true
    }
}

/// Cards `--all-drivers` writes for `job` on this host affinity.
///
/// `mlx-lm-lora` is on the train allow-list and is included only when
/// `host_class_affinity` is `apple-silicon`. Any other affinity omits that
/// card so the rest of the set still prepares. Naming `--driver mlx-lm-lora`
/// on another affinity is `refuse:host` and writes nothing.
pub fn train_enrich_drivers_for_prepare(
    job: &str,
    host_class_affinity: &str,
) -> Result<Vec<&'static str>, ModelError> {
    let ids = train_enrich_drivers_for_job(job)?;
    Ok(ids
        .into_iter()
        .filter(|id| driver_included_for_host(id, host_class_affinity))
        .collect())
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
        && !reqs.iter().any(|req| is_train_recipe_driver(req.driver_id))
    {
        return Err(ModelError::Other(format!(
            "refuse:dataset: --from-feed applies to {}. This prepare has no train recipe card. Omit --from-feed to keep the other cards.",
            TRAIN_RECIPE_DRIVERS.join(", ")
        )));
    }
    if reqs.iter().any(|req| req.official_scale)
        && !reqs.iter().any(|req| is_train_recipe_driver(req.driver_id))
    {
        return Err(ModelError::Other(
            format!(
                "refuse:official-scale: --official-scale applies to {}. This prepare has no train recipe card. Omit --official-scale to keep the other cards.",
                TRAIN_RECIPE_DRIVERS.join(", ")
            ),
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
    let mut job = enrich_job(
        req.pack,
        req.estate,
        kind,
        req.out_dir,
        req.max_steps,
        req.official_scale,
    )?;
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
        export_yaml: recorded_artifact(&names, req.out_dir, "export.yaml"),
        modelfile: recorded_artifact(&names, req.out_dir, "Modelfile"),
        promoted: false,
        auto_apply: false,
        estate_rewritten: false,
        note: PREPARE_NOTE.into(),
        dataset_mode: job.dataset.as_ref().map(|data| data.mode.clone()),
        dataset_rows: job.dataset.as_ref().map(|data| data.rows),
        dataset_from_feed: job.dataset.as_ref().map(|data| data.mode == DATASET_FEED),
        dataset_skipped: job.dataset.as_ref().map(|data| data.skipped),
        dataset_read_paths: job.dataset.as_ref().map(|data| data.read_paths.clone()),
        trained_shape: None,
        trained_paths: None,
    };
    let prepare_json = to_pretty(&doc)?;
    files.push(("prepare.json".into(), prepare_json));
    let mut next = next_markdown(driver.id(), &job, &job.out_dir, &names, &prepared.steps);
    if is_llamafactory_driver(driver.id()) {
        let note = crate::local_seat::llamafactory_local_seat_note(
            &job.out_dir,
            &job.base_model,
            &job.pack_id,
        );
        next.push_str(&note);
        if let Some((_, body)) = files.iter_mut().find(|(name, _)| name == "PREPARE.md") {
            body.push_str(&note);
        }
    } else if is_axolotl_driver(driver.id()) {
        let note = crate::local_seat::axolotl_post_train_ladder(
            &job.out_dir,
            &job.base_model,
            &job.pack_id,
            driver.id(),
        );
        next.push_str(&note);
        if let Some((_, body)) = files.iter_mut().find(|(name, _)| name == "PREPARE.md") {
            body.push_str(&note);
        }
    } else if driver.id() == MLX_LM_LORA_ID {
        let train_base = job.train_base_model.as_deref().unwrap_or("");
        let note = crate::merge_adapt::mlx_post_train_ladder(
            &job.out_dir,
            &job.base_model,
            &job.pack_id,
            train_base,
        );
        next.push_str(&note);
        if let Some((_, body)) = files.iter_mut().find(|(name, _)| name == "PREPARE.md") {
            body.push_str(&note);
        }
    }
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

fn recorded_artifact(names: &[String], out_dir: &Path, file_name: &str) -> Option<String> {
    names
        .iter()
        .any(|name| name == file_name)
        .then(|| out_dir.join(file_name).display().to_string())
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
    official_scale: bool,
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
        official_scale,
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
            "refuse:max-steps: max_steps must be at least 1. Omit it for the one-epoch recipe."
                .into(),
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

pub(crate) fn refuse_sacred_and_sku(field: &str, text: &str) -> Result<(), ModelError> {
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
    } else if let Some(method) = llamafactory_method(driver_id) {
        let recipe = out_dir.join("recipe.yaml");
        let export = out_dir.join("export.yaml");
        let adapter_dir = llamafactory_adapter_dir(job);
        let export_dir = llamafactory_export_dir(job);
        let outputs = adapter_dir.clone();
        let train_command = llamafactory_train_command(&recipe);
        let export_command = llamafactory_export_command(&export);
        let train_base = job.train_base_model.as_deref().unwrap_or("");
        let template = llamafactory_template(train_base);
        let reproduce = qlora_reproduce_notes(method, train_base);
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
                 {reproduce}\
                 {shape}\n\
                 \n\
                 {gauge}\n\
                 \n\
                 This prepare did not merge. The merge has not happened. {export_dir} has no merged weights until llamafactory-cli export exits 0. Merge with llamafactory-cli export. Do not set quantization_bit on export.yaml, and do not merge a quantized base. adapter_name_or_path is {adapter_dir}, the same path as recipe.yaml output_dir. After a finished train, adapter_config.json is in that directory. An early stop may leave the adapter only under checkpoint-<step> inside that directory. Point adapter_name_or_path at that checkpoint directory. This prepare does not rewrite export.yaml after train. A Modelfile that llamafactory-cli export writes into {export_dir} is that tool's file. This factory did not write it. LLaMA-Factory does not write GGUF. After the merge, estate enrich gguf-convert prints the llama.cpp convert_hf_to_gguf.py line (--outtype auto, outfile beside the export directory). Then seat tag {tag} on Ollama with FROM that GGUF. local-seat also prints llama-cli -m and llama-server -m for that GGUF file. A merged directory is not a llama.cpp seat until that convert. To load the adapter without a merge, estate enrich local-seat --prepared {out} --adapter {adapter_dir} prints a Modelfile. FROM is seat tag {seat}. ADAPTER is that directory. That Ollama model must already be this same train base. --weights on local-seat stays the merged or GGUF path and refuses this adapter directory. This factory does not run ollama create and does not run convert_hf_to_gguf.py.\n\
                 \n\
                 After that tag is seated, send a short prompt that checks the pack purpose. This factory does not run that smoke eval.\n\
                 \n\
                 A later preference stage is a recipe flag (stage: dpo or stage: orpo, with ranking: true in dataset_info.json). This card does not build that dataset.\n\
                 \n\
                 ## Faster single-GPU alternate\n\
                 \n\
                 On Nvidia only, Unsloth QLoRA is a faster single-GPU path. The optional NEXT card is unsloth-qlora (`estate enrich prepare --driver unsloth-qlora`). That card is a handoff. It does not call Unsloth and does not write a script. This card does not call Unsloth and does not write a script.\n\
                 https://unsloth.ai/docs/get-started/fine-tuning-llms-guide\n\
                 https://unsloth.ai/docs/get-started/install\n\
                 https://github.com/unslothai/unsloth\n\
                 \n\
                 ## Apple Silicon LoRA handoff\n\
                 \n\
                 On apple-silicon, mlx-lm already documents LoRA and fuse. The optional NEXT card is mlx-lm-lora (`estate enrich prepare --driver mlx-lm-lora`). That card writes MLX.md. It does not call mlx-lm and does not write a script. This card does not call mlx-lm and does not write an MLX trainer. Naming mlx-lm-lora on another host class is refuse:host.\n\
                 https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md\n\
                 \n\
                 axolotl-lora is the bf16 LoRA YAML. axolotl-qlora is the 4-bit YAML. This card does not call Axolotl.\n",
                host = llamafactory_host_note(driver_id, &job.host_class_affinity),
                dataset = train_dataset_blurb(job, CHAT_DATASET_SHAPE),
                seat = job.base_model,
                train = train_base,
                template = template,
                reproduce = reproduce,
                train_command = train_command,
                export_command = export_command,
                install = llamafactory_install_lines(method),
                bits = llamafactory_bitsandbytes_note(method),
                shape = llamafactory_shape_note(method, job.official_scale),
                gauge = llamafactory_gauge_note(job.max_steps, job.official_scale),
                export_dir = export_dir.display(),
                adapter_dir = adapter_dir.display(),
                out = out_dir.display(),
            ),
            format!(
                "Adapter output_dir (directory contains adapter_config.json):\n\
                 \n\
                 estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter {outputs}\n\
                 \n\
                 Merged export_dir (directory contains config.json and at least one .safetensors file whose name does not start with adapter_model; Modelfile is optional):\n\
                 \n\
                 estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter {export}\n\
                 \n\
                 GGUF path (one .gguf file, or a directory with exactly one top-level .gguf; LLaMA-Factory does not write GGUF):\n\
                 \n\
                 estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter <gguf>\n",
                out = out_dir.display(),
                outputs = outputs.display(),
                export = export_dir.display(),
            ),
            "Those three commands are the import-trained handoff for this card. --adapter is the output_dir this recipe wrote, the export_dir this export.yaml wrote, or a .gguf file. A path that is none of those shapes, or more than one, is refuse:adapter. import-trained records trained_shape and trained_paths on prepare.json and on the binding proposal. It does not rewrite the estate and it does not promote.".to_string(),
        )
    } else if let Some(method) = axolotl_method(driver_id) {
        let command = axolotl_train_command(&config);
        let outputs = out_dir.join("outputs");
        let merged = outputs.join("merged");
        let train_base = job.train_base_model.as_deref().unwrap_or("");
        (
            format!(
                "Run this on a CUDA host (consumer-nvidia or rented-nvidia). This factory does not run it, does not download weights, and does not call CUDA.\n\
                 \n\
                 {command}\n\
                 \n\
                 Quickstart: https://docs.axolotl.ai/docs/getting-started.html\n\
                 The handoff command is `axolotl train` on the yaml this prepare wrote.\n\
                 \n\
                 {host}\n\
                 \n\
                 {dataset}\n\
                 \n\
                 Seat tag is {seat}. That is the Ollama id for Modelfile FROM. It comes from params.model on the local binding, or from a pack model_hint that is already a model tag.\n\
                 \n\
                 Train base is {train}. axolotl.yml sets base_model to that value. A train base is a Hugging Face repo id (namespace/name) or a local directory of HF weights. This factory did not download weights and does not map the seat tag onto a Hub repo.\n\
                 \n\
                 {shape}\n\
                 \n\
                 {gauge}\n\
                 \n\
                 Axolotl writes the adapter under the output_dir in axolotl.yml. This prepare did not merge. After train, `estate enrich merge-adapt` prints Axolotl's `axolotl merge-lora` line and the `output_dir/merged` directory. This factory does not run that merge. Axolotl does not write GGUF. After that directory exists, `gguf-convert` prints the llama.cpp `convert_hf_to_gguf.py` line (`--outtype auto`), `local-seat` prints the `ollama create` line and, when the weights are a GGUF, llama-cli -m and llama-server -m for that file, and `import-trained` records the artifact. To load the adapter without a merge, FROM an Ollama model of this same train base, plus ADAPTER for the adapter directory. The seat tag {seat} is the id this cell already runs. The create name is {tag}. This factory does not run ollama create and does not run convert_hf_to_gguf.py.\n\
                 \n\
                 llamafactory-lora is the unquantized LLaMA-Factory LoRA recipe. llamafactory-qlora is the 4-bit QLoRA recipe. This card does not call LLaMA-Factory.\n\
                 Unsloth QLoRA is a faster single-GPU alternate on Nvidia only. The optional NEXT card is unsloth-qlora (https://github.com/unslothai/unsloth). This card does not call Unsloth.\n\
                 On apple-silicon, the optional NEXT card is mlx-lm-lora (https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md). That card writes MLX.md. This card does not call mlx-lm and does not write an MLX trainer. Naming mlx-lm-lora on another host class is refuse:host.\n",
                host = axolotl_host_note(driver_id, &job.host_class_affinity),
                dataset = train_dataset_blurb(job, ALPACA_DATASET_SHAPE),
                seat = job.base_model,
                train = train_base,
                shape = axolotl_shape_note(method),
                gauge = axolotl_gauge_note(method, job.max_steps, job.official_scale),
            ),
            format!(
                "Adapter output_dir (directory contains adapter_config.json):\n\
                 \n\
                 estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter {outputs}\n\
                 \n\
                 Merged Hugging Face directory (config.json and at least one .safetensors file whose name does not start with adapter_model; Modelfile is optional). The operator owns that directory. This factory did not merge:\n\
                 \n\
                 estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter {merged}\n\
                 \n\
                 GGUF path (one .gguf file, or a directory with exactly one top-level .gguf). Axolotl does not write GGUF:\n\
                 \n\
                 estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter <gguf>\n",
                out = out_dir.display(),
                outputs = outputs.display(),
                merged = merged.display(),
            ),
            "The first command points --adapter at the output_dir in axolotl.yml. The second points --adapter at output_dir/merged, the Hugging Face directory Axolotl writes. The third points --adapter at a .gguf file. Axolotl does not write GGUF. merge-adapt prints the axolotl merge-lora line. gguf-convert prints the llama.cpp line for the merged directory. local-seat prints the ollama create line. For a GGUF it also prints llama-cli -m and llama-server -m. import-trained records trained_shape and trained_paths. It does not rewrite the estate and it does not promote.".to_string(),
        )
    } else if driver_id == UNSLOTH_QLORA_ID {
        let train_base = job.train_base_model.as_deref().unwrap_or("");
        let handoff = out_dir.join(UNSLOTH_HANDOFF);
        (
            format!(
                "Unsloth QLoRA is an optional Nvidia-only alternate for a faster single-GPU run. It is not the product. Portable local runtimes stay swappable. Ollama stays the seat.\n\
                 \n\
                 This factory does not call Unsloth, does not shell out, does not install Unsloth, does not download weights, and does not write an executable Unsloth script.\n\
                 \n\
                 Operator-owned handoff: {handoff}\n\
                 That file records the seat tag and the train base. It is not a recipe and it is not a script.\n\
                 \n\
                 Install from Unsloth's pages. The README publishes this Linux line: `{install_line}`. The venv steps are on the install page. This factory does not run that install.\n\
                 {install_doc}\n\
                 {guide}\n\
                 {repo}\n\
                 \n\
                 {host}\n\
                 \n\
                 Seat tag is {seat}. That is the Ollama id for Modelfile FROM. It comes from params.model on the local binding, or from a pack model_hint that is already a model tag.\n\
                 \n\
                 Train base is {train}. A train base is a Hugging Face repo id (namespace/name) or a local directory of HF weights. This factory did not download weights and does not map the seat tag onto a Hub repo.\n\
                 \n\
                 {flags}\n\
                 \n\
                 llamafactory-qlora writes the LLaMA-Factory QLoRA recipe. axolotl-qlora writes the Axolotl 4-bit YAML. This card does not call either trainer.\n\
                 \n\
                 After you train with Unsloth outside this factory, hand the artifact you saved to import-trained. This factory does not choose Unsloth save knobs and does not write GGUF. estate enrich local-seat reads a llamafactory-lora, llamafactory-qlora, axolotl-lora, or axolotl-qlora prepare. This card does not.\n\
                 \n\
                 READY_FOR_LIVE_TEST: no\n",
                handoff = handoff.display(),
                install_line = UNSLOTH_README_INSTALL,
                install_doc = UNSLOTH_INSTALL_DOC,
                guide = UNSLOTH_GUIDE_DOC,
                repo = UNSLOTH_REPO,
                host = unsloth_host_note(&job.host_class_affinity),
                seat = job.base_model,
                train = train_base,
                flags = unsloth_flag_note(job),
            ),
            format!(
                "Adapter directory (contains adapter_config.json):\n\
                 \n\
                 estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter <adapter-dir>\n\
                 \n\
                 Merged directory (config.json and at least one .safetensors file whose name does not start with adapter_model; Modelfile is optional):\n\
                 \n\
                 estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter <merged-dir>\n\
                 \n\
                 GGUF path (one .gguf file, or a directory with exactly one top-level .gguf):\n\
                 \n\
                 estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter <gguf>\n",
                out = out_dir.display(),
            ),
            "Point --adapter at the artifact you saved from the Unsloth guide. import-trained records trained_shape and trained_paths. It does not rewrite the estate and it does not promote. READY_FOR_LIVE_TEST: no.".to_string(),
        )
    } else if driver_id == MLX_LM_LORA_ID {
        let train_base = job.train_base_model.as_deref().unwrap_or("");
        let handoff = out_dir.join(MLX_HANDOFF);
        (
            format!(
                "mlx-lm LoRA is an optional Apple Silicon handoff. It is not the product. Portable local runtimes stay swappable. Ollama stays the seat. Native MLX stays a stub on the local runtime catalog.\n\
                 \n\
                 This factory does not call mlx-lm, does not shell out, does not install mlx-lm, does not download weights, and does not write an executable MLX script.\n\
                 \n\
                 Operator-owned handoff: {handoff}\n\
                 That file records the seat tag, the train base, and host_class_affinity apple-silicon. It is not a recipe and it is not a script.\n\
                 \n\
                 Install from the mlx-lm LoRA page. That page publishes `{install_line}`. This factory does not run that install.\n\
                 {lora_doc}\n\
                 {repo}\n\
                 \n\
                 The page names `{lora_command}` for LoRA. A quantized model on that page is QLoRA. This factory does not choose iters, rank, layers, or a data directory, and it does not write a YAML config.\n\
                 \n\
                 Fuse, from that same page: `{fuse}`.\n\
                 mlx-lm loads adapters from adapters/ and writes the fused model under fused_model/ unless you pass other flags. `mlx_lm.fuse --help` lists them. This factory does not run fuse.\n\
                 After that train, the section below names merge-adapt, the fuse line with --adapter-path and --save-path, `mlx_lm.fuse --export-gguf`, and local-seat for ggml-model-f16.gguf. The fused directory is MLX weights. The Hugging Face convert card stays on the LLaMA-Factory and Axolotl prepares.\n\
                 \n\
                 {host}\n\
                 \n\
                 Seat tag is {seat}. That is the Ollama id for Modelfile FROM. It comes from params.model on the local binding, or from a pack model_hint that is already a model tag.\n\
                 \n\
                 Train base is {train}. A train base is a Hugging Face repo id (namespace/name) or a local directory of HF weights. Pass that value as --model when you follow the LoRA page. This factory did not download weights and does not map the seat tag onto a Hub repo.\n\
                 \n\
                 {flags}\n\
                 \n\
                 llamafactory-lora writes the LLaMA-Factory LoRA recipe. axolotl-lora writes the Axolotl bf16 YAML. unsloth-qlora is the Nvidia-only QLoRA handoff. This card does not call those trainers.\n\
                 \n\
                 After you train with mlx-lm outside this factory, point merge-adapt at the adapter directory (adapter_config.json and adapters.safetensors). The LoRA page saves that directory under adapters/ unless you pass --adapter-path. This factory does not choose that path. local-seat on this card reads the GGUF file fuse --export-gguf writes. It does not read the fused MLX directory as a Hugging Face export.\n\
                 \n\
                 READY_FOR_LIVE_TEST: no\n",
                handoff = handoff.display(),
                install_line = MLX_TRAIN_INSTALL,
                lora_doc = MLX_LORA_DOC,
                repo = MLX_REPO,
                lora_command = MLX_LORA_COMMAND,
                fuse = MLX_FUSE_COMMAND,
                host = mlx_host_note(&job.host_class_affinity),
                seat = job.base_model,
                train = train_base,
                flags = mlx_flag_note(job),
            ),
            format!(
                "Adapter directory (contains adapter_config.json):\n\
                 \n\
                 estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter <adapter-dir>\n\
                 \n\
                 Merged directory (config.json and at least one .safetensors file whose name does not start with adapter_model; Modelfile is optional):\n\
                 \n\
                 estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter <merged-dir>\n\
                 \n\
                 GGUF path (one .gguf file, or a directory with exactly one top-level .gguf):\n\
                 \n\
                 estate enrich import-trained --estate <estate.yaml> --prepared {out} --tag {tag} --adapter <gguf>\n",
                out = out_dir.display(),
            ),
            "Point --adapter at the artifact you saved after following the mlx-lm LoRA page, including fuse when you fused. import-trained records trained_shape and trained_paths. It does not rewrite the estate and it does not promote. READY_FOR_LIVE_TEST: no.".to_string(),
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

fn axolotl_host_note(driver_id: &str, affinity: &str) -> String {
    cuda_train_host_note(driver_id, affinity, "axolotl train")
}

fn unsloth_host_note(affinity: &str) -> String {
    cuda_train_host_note(UNSLOTH_QLORA_ID, affinity, "the Unsloth QLoRA guide")
}

fn refuse_mlx_host(affinity: &str) -> Result<(), ModelError> {
    if affinity == APPLE_SILICON_HOST {
        return Ok(());
    }
    Err(ModelError::Other(format!(
        "refuse:host: {MLX_LM_LORA_ID} prepares on apple-silicon. host_class_affinity is '{affinity}'. This card does not write a handoff for that host. Set host_class_affinity to apple-silicon, or prepare llamafactory-lora, llamafactory-qlora, axolotl-lora, axolotl-qlora, or unsloth-qlora. This factory does not call mlx-lm."
    )))
}

fn mlx_host_note(affinity: &str) -> String {
    if affinity == APPLE_SILICON_HOST {
        format!(
            "host_class_affinity is apple-silicon. This card is the optional mlx-lm LoRA handoff for that host. consumer-nvidia, rented-nvidia, and any do not prepare this card. Naming {MLX_LM_LORA_ID} on another affinity is refuse:host and writes nothing. --all-drivers omits this card unless the affinity is apple-silicon, so the other train cards still prepare. This factory does not call mlx-lm."
        )
    } else {
        format!(
            "host_class_affinity is {affinity}. {MLX_LM_LORA_ID} prepares on apple-silicon. This file should not exist for this host."
        )
    }
}

fn unsloth_flag_note(job: &EnrichJob) -> String {
    let mut lines = vec![
        "This card does not write a recipe, so it does not write max_steps, a dataset, or an official-scale cutoff.".to_string(),
        "--from-feed on this card alone is refuse:dataset. --official-scale on this card alone is refuse:official-scale. --max-steps 0 is refuse:max-steps.".to_string(),
        "This factory does not download a dataset.".to_string(),
    ];
    if let Some(steps) = job.max_steps {
        lines.push(format!(
            "This prepare was given --max-steps {steps}. This card does not write that count and does not run a gauge."
        ));
    }
    if job.official_scale {
        lines.push(
            "This prepare was given --official-scale. That flag writes the LLaMA-Factory SFT scale on llamafactory-lora and llamafactory-qlora. This card does not implement official scale and does not change any Unsloth knob.".into(),
        );
    }
    lines.join("\n\n")
}

fn unsloth_handoff_md(job: &EnrichJob, train_base: &str) -> String {
    format!(
        "# Unsloth QLoRA handoff (operator-owned)\n\
         \n\
         This file is not an Unsloth config, not a YAML recipe, and not a training script.\n\
         Cell One does not call Unsloth, does not shell out, does not install Unsloth, and does not download weights or datasets.\n\
         Do not execute this file.\n\
         \n\
         driver: {driver}\n\
         status: optional\n\
         nvidia_only: true\n\
         executable: false\n\
         train_base_model: {train}\n\
         seat_tag: {seat}\n\
         \n\
         Seat tag is the Ollama id for Modelfile FROM. Train base is the Hugging Face repo id or local directory of HF weights you pass to Unsloth. This factory does not map the seat tag onto a Hub repo.\n\
         \n\
         This card is the optional Nvidia-only QLoRA alternate for a faster single-GPU run. It is not the product. Portable local runtimes stay swappable. Ollama stays the seat. This factory does not write an MLX trainer.\n\
         \n\
         Follow Unsloth's public pages. This factory does not choose ranks, sequence length, or save knobs.\n\
         The Unsloth README publishes this Linux install line: `{install_line}`.\n\
         The venv steps are on the install page. This factory does not run that install.\n\
         \n\
         Install: {install_doc}\n\
         Fine-tuning guide: {guide}\n\
         Repository: {repo}\n\
         \n\
         After that train finishes outside this factory, hand one artifact to estate enrich import-trained:\n\
         - an adapter directory that contains adapter_config.json\n\
         - a merged directory that contains config.json and at least one .safetensors file whose name does not start with adapter_model\n\
         - one .gguf file, or a directory with exactly one top-level .gguf\n\
         \n\
         The command is in NEXT.md. import-trained does not promote and does not rewrite estate.yaml.\n\
         \n\
         {flags}\n\
         \n\
         READY_FOR_LIVE_TEST: no\n",
        driver = UNSLOTH_QLORA_ID,
        train = yaml_quote(train_base),
        seat = yaml_quote(&job.base_model),
        install_line = UNSLOTH_README_INSTALL,
        install_doc = UNSLOTH_INSTALL_DOC,
        guide = UNSLOTH_GUIDE_DOC,
        repo = UNSLOTH_REPO,
        flags = unsloth_flag_note(job),
    )
}

fn unsloth_prepare_steps(job: &EnrichJob, train_base: &str) -> String {
    format!(
        "This step wrote {handoff}. That file is an operator-owned handoff. It is not an Unsloth config and not a training script. This step did not call Unsloth, did not shell out, did not train, did not download weights, and did not rewrite the estate.\n\
         \n\
         {host}\n\
         \n\
         Seat tag is {seat}. That is the Ollama id for Modelfile FROM.\n\
         \n\
         Train base is {train}. The handoff records that value. A train base is a Hugging Face repo id (namespace/name) or a local directory of HF weights. This factory did not download weights and does not map the seat tag onto a Hub repo.\n\
         \n\
         {flags}\n\
         \n\
         Install and the fine-tuning guide are linked from NEXT.md. The Unsloth README publishes `{install_line}`. This factory does not run that install.\n\
         \n\
         After you train outside this factory, hand the saved artifact to estate enrich import-trained. This factory does not choose the save format.\n\
         \n\
         READY_FOR_LIVE_TEST: no\n",
        handoff = UNSLOTH_HANDOFF,
        host = unsloth_host_note(&job.host_class_affinity),
        seat = job.base_model,
        train = train_base,
        flags = unsloth_flag_note(job),
        install_line = UNSLOTH_README_INSTALL,
    )
}

fn mlx_flag_note(job: &EnrichJob) -> String {
    let mut lines = vec![
        "This card does not write a recipe, so it does not write max_steps, a dataset, or an official-scale cutoff.".to_string(),
        "--from-feed on this card alone is refuse:dataset. --official-scale on this card alone is refuse:official-scale. --max-steps 0 is refuse:max-steps.".to_string(),
        "This factory does not download a dataset.".to_string(),
    ];
    if let Some(steps) = job.max_steps {
        lines.push(format!(
            "This prepare was given --max-steps {steps}. This card does not write that count and does not run a gauge."
        ));
    }
    if job.official_scale {
        lines.push(
            "This prepare was given --official-scale. That flag writes the LLaMA-Factory SFT scale on llamafactory-lora and llamafactory-qlora. This card does not implement official scale and does not change any mlx-lm knob.".into(),
        );
    }
    lines.join("\n\n")
}

fn mlx_handoff_md(job: &EnrichJob, train_base: &str) -> String {
    let mut body = format!(
        "# mlx-lm LoRA handoff (operator-owned)\n\
         \n\
         This file is not an mlx-lm config, not a YAML recipe, and not a training script.\n\
         Cell One does not call mlx-lm, does not shell out, does not install mlx-lm, and does not download weights or datasets.\n\
         Do not execute this file.\n\
         \n\
         driver: {driver}\n\
         status: optional\n\
         apple_silicon_only: true\n\
         host_class_affinity: {host}\n\
         executable: false\n\
         train_base_model: {train}\n\
         seat_tag: {seat}\n\
         \n\
         Seat tag is the Ollama id for Modelfile FROM. Train base is the Hugging Face repo id or local directory of HF weights you pass to mlx_lm.lora as --model. This factory does not map the seat tag onto a Hub repo.\n\
         \n\
         This card is the optional Apple Silicon LoRA handoff. It prepares only when host_class_affinity is apple-silicon. It is not the product. Portable local runtimes stay swappable. Ollama stays the seat. Native MLX stays a stub on the local runtime catalog. This factory does not write an MLX trainer.\n\
         \n\
         Follow the public mlx-lm LoRA page. This factory does not choose ranks, iterations, layers, or save knobs.\n\
         That page publishes this install line: `{install_line}`.\n\
         This factory does not run that install.\n\
         \n\
         LoRA page: {lora_doc}\n\
         Repository: {repo}\n\
         Train command on that page: `{lora_command}`\n\
         Fuse command on that page: `{fuse}`\n\
         \n\
         Fuse loads adapters from adapters/ and writes fused_model/ unless you pass other flags. mlx_lm.fuse --help lists them. A quantized --model on that page is QLoRA. This factory does not run fuse.\n\
         \n\
         After mlx_lm.lora, the adapter directory holds adapter_config.json and adapters.safetensors. estate enrich merge-adapt prints mlx_lm.fuse with --model set to the train base, --adapter-path set to that directory, and --save-path set to fused_model beside this prepare. The same print adds --export-gguf. That flag writes ggml-model-f16.gguf inside the save path. LORA.md limits that GGUF export to Mistral, Mixtral, and Llama style models in fp16 precision. Then estate enrich local-seat --weights points at that GGUF file. The Hugging Face convert card stays on the LLaMA-Factory and Axolotl prepares. This card does not point it at the fused MLX weights. This factory does not write those weights.\n\
         \n\
         The filled commands are in NEXT.md and PREPARE.md. import-trained still accepts one artifact:\n\
         - an adapter directory that contains adapter_config.json\n\
         - a merged directory that contains config.json and at least one .safetensors file whose name does not start with adapter_model\n\
         - one .gguf file, or a directory with exactly one top-level .gguf\n\
         \n\
         The command is in NEXT.md. import-trained does not promote and does not rewrite estate.yaml.\n\
         \n\
         {flags}\n\
         \n\
         READY_FOR_LIVE_TEST: no\n",
        driver = MLX_LM_LORA_ID,
        host = APPLE_SILICON_HOST,
        train = yaml_quote(train_base),
        seat = yaml_quote(&job.base_model),
        install_line = MLX_TRAIN_INSTALL,
        lora_doc = MLX_LORA_DOC,
        repo = MLX_REPO,
        lora_command = MLX_LORA_COMMAND,
        fuse = MLX_FUSE_COMMAND,
        flags = mlx_flag_note(job),
    );
    body.push_str(&crate::merge_adapt::mlx_post_train_ladder(
        &job.out_dir,
        &job.base_model,
        &job.pack_id,
        train_base,
    ));
    body
}

fn mlx_prepare_steps(job: &EnrichJob, train_base: &str) -> String {
    format!(
        "This step wrote {handoff}. That file is an operator-owned handoff. It is not an mlx-lm config and not a training script. This step did not call mlx-lm, did not shell out, did not train, did not download weights, and did not rewrite the estate.\n\
         \n\
         {host}\n\
         \n\
         Seat tag is {seat}. That is the Ollama id for Modelfile FROM.\n\
         \n\
         Train base is {train}. The handoff records that value. A train base is a Hugging Face repo id (namespace/name) or a local directory of HF weights. This factory did not download weights and does not map the seat tag onto a Hub repo.\n\
         \n\
         {flags}\n\
         \n\
         The LoRA page and the fuse command are linked from NEXT.md. That page publishes `{install_line}` and `{fuse}`. This factory does not run that install and does not run fuse.\n\
         \n\
         After you train outside this factory, the section below names merge-adapt, the documented fuse line, `mlx_lm.fuse --export-gguf`, and local-seat for the GGUF file. This factory does not choose the adapter path. mlx_lm.lora writes it.\n\
         \n\
         READY_FOR_LIVE_TEST: no\n",
        handoff = MLX_HANDOFF,
        host = mlx_host_note(&job.host_class_affinity),
        seat = job.base_model,
        train = train_base,
        flags = mlx_flag_note(job),
        install_line = MLX_TRAIN_INSTALL,
        fuse = MLX_FUSE_COMMAND,
    )
}

fn llamafactory_host_note(driver_id: &str, affinity: &str) -> String {
    cuda_train_host_note(driver_id, affinity, "llamafactory-cli train")
}

const CHAT_DATASET_SHAPE: &str =
    "dataset.jsonl is instruct chat JSONL (messages of role and content).";
const ALPACA_DATASET_SHAPE: &str = "dataset.jsonl is Alpaca JSONL (instruction, input, output).";

fn require_dataset(job: &EnrichJob) -> Result<&DatasetMaterial, ModelError> {
    job.dataset
        .as_ref()
        .ok_or_else(|| ModelError::Other("refuse:dataset: train recipe has no dataset plan".into()))
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
        ModelError::Other(format!("refuse:dataset: cannot stat {relative}: {err}"))
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
    let text = std::str::from_utf8(&buf[..end])
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
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
        return Ok(InstructLine::Row(chat_source_row(
            relative, line_no, value,
        )?));
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
            refuse_frontier_source_on_estate(&["frontier".to_string()], estate)
                .map_err(map_feed)?;
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
    let messages = value
        .get("messages")
        .and_then(|item| item.as_array())
        .ok_or_else(|| {
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
        let role = item
            .get("role")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
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

fn refuse_hydrated_text(
    relative: &str,
    line_no: usize,
    row: &HydratedRow,
) -> Result<(), ModelError> {
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

fn llamafactory_install_lines(method: LlamaFactoryMethod) -> &'static str {
    match method {
        LlamaFactoryMethod::Qlora => "pip install llamafactory\npip install 'bitsandbytes>=0.49'",
        LlamaFactoryMethod::Lora => "pip install llamafactory",
    }
}

fn llamafactory_bitsandbytes_note(method: LlamaFactoryMethod) -> &'static str {
    match method {
        LlamaFactoryMethod::Qlora => "QLoRA needs bitsandbytes. `pip install llamafactory` and `llamafactory[torch,metrics]` 0.9.5 did not install it. Install bitsandbytes in that same environment (`pip install 'bitsandbytes>=0.49'`). On a consumer RTX host, keep the torch CUDA wheel you already installed. A consumer RTX smoke on CUDA 12.8 used torch 2.11.0+cu128 and bitsandbytes 0.50.2. That bitsandbytes install did not replace torch. This factory does not install either package. 16-bit LoRA with no quantization is `--driver llamafactory-lora`. That path does not require bitsandbytes.",
        LlamaFactoryMethod::Lora => "This LoRA path does not require bitsandbytes. recipe.yaml leaves the base at 16-bit and omits quantization. QLoRA (`--driver llamafactory-qlora`) still requires bitsandbytes. This factory does not install LLaMA-Factory.",
    }
}

fn llamafactory_shape_note(method: LlamaFactoryMethod, official_scale: bool) -> String {
    let scale = llamafactory_scale_clause(method, official_scale);
    match method {
        LlamaFactoryMethod::Qlora => format!(
            "This card is 4-bit QLoRA (`quantization_bit: 4`, `quantization_method: bnb`, LoRA rank {}, packing on). {scale}",
            method.lora_rank()
        ),
        LlamaFactoryMethod::Lora => format!(
            "This card is 16-bit LoRA. `lora_rank` is {}, matching LLaMA-Factory `examples/train_lora/qwen3_lora_sft.yaml`. `lora_alpha` is {} (`lora_rank` times 2, the value LLaMA-Factory uses when `lora_alpha` is unset). `packing` is false, matching that file's unset packing (the LLaMA-Factory default). {scale}",
            method.lora_rank(),
            method.lora_alpha(),
        ),
    }
}

fn llamafactory_scale_clause(method: LlamaFactoryMethod, official_scale: bool) -> String {
    if official_scale {
        return "This prepare used `--official-scale`. `cutoff_len` is 2048, `num_train_epochs` is 3.0, `gradient_accumulation_steps` is 8, and `warmup_ratio` is 0.1, matching LLaMA-Factory `examples/train_lora/qwen3_lora_sft.yaml`. Rank, packing, and quantization stay on this card. `--max-steps` still overrides `num_train_epochs` when it is set.".into();
    }
    let lead = match method {
        LlamaFactoryMethod::Lora => format!(
            "`cutoff_len` stays {cutoff}, `num_train_epochs` stays 1.0, `gradient_accumulation_steps` stays 4, and `warmup_ratio` stays 0.03 so a first prepare stays short. That official file uses `cutoff_len` 2048, `num_train_epochs` 3.0, `gradient_accumulation_steps` 8, and `warmup_ratio` 0.1.",
            cutoff = LLAMAFACTORY_CUTOFF_LEN,
        ),
        LlamaFactoryMethod::Qlora => format!(
            "`cutoff_len` stays {cutoff}, `num_train_epochs` stays 1.0, `gradient_accumulation_steps` stays 4, and `warmup_ratio` stays 0.03 so a first prepare stays short. LLaMA-Factory `examples/train_lora/qwen3_lora_sft.yaml` uses `cutoff_len` 2048, `num_train_epochs` 3.0, `gradient_accumulation_steps` 8, and `warmup_ratio` 0.1.",
            cutoff = LLAMAFACTORY_CUTOFF_LEN,
        ),
    };
    format!("{lead} Re-prepare with `--official-scale` to write those four fields.")
}

fn llamafactory_prepare_steps(
    method: LlamaFactoryMethod,
    job: &EnrichJob,
    train_base: &str,
    data: &DatasetMaterial,
) -> String {
    let driver_id = method.driver_id();
    let host = llamafactory_host_note(driver_id, &job.host_class_affinity);
    let data_note = dataset_card_note(CHAT_DATASET_SHAPE, data);
    let template = llamafactory_template(train_base);
    let reproduce = qlora_reproduce_notes(method, train_base);
    let scale = llamafactory_scale(job.official_scale);
    let recipe_line = match method {
        LlamaFactoryMethod::Qlora => format!(
            "The recipe is SFT QLoRA (`stage: sft`, `finetuning_type: lora`, `quantization_bit: 4`, `quantization_method: bnb`, LoRA rank {rank}, `cutoff_len` {cutoff}, `packing: true`).",
            rank = method.lora_rank(),
            cutoff = scale.cutoff,
        ),
        LlamaFactoryMethod::Lora => format!(
            "The recipe is SFT LoRA (`stage: sft`, `finetuning_type: lora`, LoRA rank {rank}, `cutoff_len` {cutoff}, `packing: false`). It omits quantization_bit and quantization_method.",
            rank = method.lora_rank(),
            cutoff = scale.cutoff,
        ),
    };
    format!(
        "This step wrote recipe.yaml, export.yaml, dataset_info.json, and dataset.jsonl. {recipe_line} It did not run llamafactory-cli, did not train, did not merge, did not download weights, and did not call CUDA.\n\
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
         {reproduce}\
         {bits}\n\
         \n\
         {shape}\n\
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
         This prepare did not merge. llamafactory-cli export has not run. The export directory has no merged weights. export.yaml is the merge card for that later command. This prepare does not rewrite export.yaml after train.\n\
         \n\
         The copy-paste lines with absolute paths are in NEXT.md. Ollama stays the local-run seat after the adapter or the merged weights exist. This factory does not export GGUF.\n",
        seat = job.base_model,
        train = train_base,
        template = template,
        reproduce = reproduce,
        bits = llamafactory_bitsandbytes_note(method),
        shape = llamafactory_shape_note(method, job.official_scale),
        gauge = llamafactory_gauge_note(job.max_steps, job.official_scale),
        install = llamafactory_install_lines(method),
    )
}

fn llamafactory_gauge_note(max_steps: Option<u32>, official_scale: bool) -> String {
    let base = if official_scale {
        "This prepare used --official-scale. cutoff_len is 2048, num_train_epochs is 3.0, gradient_accumulation_steps is 8, and warmup_ratio is 0.1. A short gauge run passes `--max-steps 10` on the same prepare. LLaMA-Factory overrides `num_train_epochs` when `max_steps` is set. When that count is under 50, this prepare sets `save_steps` to the same count so a checkpoint is written during the short run."
    } else {
        "A short gauge run does not need a full epoch. Re-prepare with `--max-steps 10`. LLaMA-Factory overrides `num_train_epochs` when `max_steps` is set. When that count is under 50, this prepare sets `save_steps` to the same count so a checkpoint is written during the short run. The default recipe keeps `num_train_epochs: 1.0` and `save_steps: 50`, and leaves `max_steps` unset."
    };
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
    if leaf_is_layout_container(&folded) {
        return false;
    }
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

/// HF cache and export layouts put the checkpoint name above the file leaf.
/// `weights`, `snapshots`, and a hex commit id are not Ollama seat tags.
/// A leaf that equals the seated tag (`llama3`, `./weights/llama3`) still refuses.
fn leaf_is_layout_container(folded: &str) -> bool {
    if matches!(folded, "weights" | "snapshots") {
        return true;
    }
    folded.len() >= 12 && folded.chars().all(|c| c.is_ascii_hexdigit())
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
    if !records_train_base(driver_id) {
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

fn read_mlx_handoff(path: &Path) -> Result<String, ModelError> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(ModelError::Other(
                "refuse:train-base: MLX.md is missing, so the train base cannot be checked".into(),
            ));
        }
        Err(err) => {
            return Err(ModelError::Other(format!(
                "refuse:host: {}: {err}",
                path.display()
            )));
        }
    };
    if meta.file_type().is_symlink() {
        return Err(ModelError::Other(format!(
            "refuse:host: {} is a symlink. enrich does not follow a symlinked MLX.md.",
            path.display()
        )));
    }
    if !meta.is_file() {
        return Err(ModelError::Other(format!(
            "refuse:host: {} is not a regular file",
            path.display()
        )));
    }
    const MAX: u64 = 1024 * 1024;
    if meta.len() > MAX {
        return Err(ModelError::Other(format!(
            "refuse:host: {} is larger than {MAX} bytes",
            path.display()
        )));
    }
    let mut file = open_nofollow(path).map_err(|err| {
        if matches!(err.raw_os_error(), Some(40 | 62)) {
            ModelError::Other(format!(
                "refuse:host: {} is a symlink. enrich does not follow a symlinked MLX.md.",
                path.display()
            ))
        } else {
            ModelError::Other(format!("refuse:host: {}: {err}", path.display()))
        }
    })?;
    let mut buf = String::new();
    use std::io::Read;
    file.read_to_string(&mut buf)
        .map_err(|err| ModelError::Other(format!("refuse:host: {}: {err}", path.display())))?;
    Ok(buf)
}

pub(crate) fn refuse_recipe_train_record(
    doc: &EnrichPrepareDoc,
    prepared_dir: &Path,
) -> Result<(), ModelError> {
    if !records_train_base(&doc.driver) {
        return Ok(());
    }
    if doc.driver == MLX_LM_LORA_ID && doc.host_class_affinity != APPLE_SILICON_HOST {
        return Err(ModelError::Other(format!(
            "refuse:host: {MLX_LM_LORA_ID} records host_class_affinity apple-silicon. prepare.json host_class_affinity is '{}'. This card does not prepare on that host.",
            doc.host_class_affinity
        )));
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
    let checks: &[(&str, &str)] = if is_llamafactory_driver(&doc.driver) {
        &[
            ("recipe.yaml", "model_name_or_path"),
            ("export.yaml", "model_name_or_path"),
        ]
    } else if is_axolotl_driver(&doc.driver) {
        &[("axolotl.yml", "base_model")]
    } else if doc.driver == UNSLOTH_QLORA_ID {
        &[(UNSLOTH_HANDOFF, "train_base_model")]
    } else if doc.driver == MLX_LM_LORA_ID {
        &[(MLX_HANDOFF, "train_base_model")]
    } else {
        return Err(ModelError::Other(format!(
            "refuse:train-base: {} has no train-base file to check",
            doc.driver
        )));
    };
    for (name, key) in checks {
        let path = prepared_dir.join(name);
        let text = if *name == MLX_HANDOFF {
            read_mlx_handoff(&path)?
        } else {
            std::fs::read_to_string(&path).map_err(|_| {
                ModelError::Other(format!(
                    "refuse:train-base: {name} is missing, so the train base cannot be checked"
                ))
            })?
        };
        let found = yaml_field(&text, key).unwrap_or_default();
        if found != train {
            return Err(ModelError::Other(format!(
                "refuse:train-base: {name} {key} is '{found}'. prepare.json train_base_model is '{train}'. Seat tag '{}' is the Ollama id for Modelfile FROM. These files must name the train base.",
                doc.base_model
            )));
        }
        if *name == UNSLOTH_HANDOFF || *name == MLX_HANDOFF {
            let seat = yaml_field(&text, "seat_tag").unwrap_or_default();
            if seat != doc.base_model {
                return Err(ModelError::Other(format!(
                    "refuse:train-base: {name} seat_tag is '{seat}'. prepare.json seat tag is '{}'. Seat tag is the Ollama id for Modelfile FROM. The handoff must name that seat tag and the train base.",
                    doc.base_model
                )));
            }
        }
        if *name == MLX_HANDOFF {
            let host = yaml_field(&text, "host_class_affinity").unwrap_or_default();
            if host != APPLE_SILICON_HOST || host != doc.host_class_affinity {
                return Err(ModelError::Other(format!(
                    "refuse:host: {name} host_class_affinity is '{host}'. prepare.json host_class_affinity is '{}'. {MLX_LM_LORA_ID} prepares on apple-silicon.",
                    doc.host_class_affinity
                )));
            }
        }
        if *name == "export.yaml" {
            refuse_quantized_export(&text)?;
        }
    }
    Ok(())
}

/// A real yaml key. Comment lines are ignored, so a note can name the hazard
/// without refusing a card this prepare already wrote.
fn refuse_quantized_export(text: &str) -> Result<(), ModelError> {
    for key in ["quantization_bit", "quantization_method"] {
        if yaml_field(text, key).is_some() {
            return Err(ModelError::Other(format!(
                "refuse:export: export.yaml sets {key}. LLaMA-Factory refuses to merge adapters into a quantized model. Leave export.yaml unquantized. This prepare did not merge."
            )));
        }
    }
    Ok(())
}

fn train_base_segments(train_base: &str) -> Vec<&str> {
    let parts: Vec<&str> = train_base
        .split(['/', '\\'])
        .filter(|part| !part.trim().is_empty())
        .collect();
    if parts.is_empty() {
        vec![train_base]
    } else {
        parts
    }
}

/// Non-thinking Qwen3 instruct checkpoints use `qwen3_nothink`.
/// `Qwen/Qwen3-4B-Instruct-2507` is that shape in `examples/train_lora/qwen3_lora_sft.yaml`.
/// A name that contains `thinking` stays on `qwen3`. A name that contains `nothink` uses `qwen3_nothink`.
fn qwen3_uses_nothink(name: &str) -> bool {
    if name.contains("nothink") {
        return true;
    }
    name.contains("instruct") && !name.contains("thinking")
}

/// Family marker on one path segment. `None` when that segment names no known family.
fn template_for_segment(segment: &str) -> Option<&'static str> {
    let name = segment.to_ascii_lowercase();
    if name.contains("qwen3") {
        Some(if qwen3_uses_nothink(&name) {
            "qwen3_nothink"
        } else {
            "qwen3"
        })
    } else if name.contains("qwen") {
        Some("qwen")
    } else if let Some(template) = llama_template_for_segment(&name) {
        Some(template)
    } else if let Some(template) = mistral_template_for_segment(&name) {
        Some(template)
    } else if let Some(template) = gemma_template_for_segment(&name) {
        Some(template)
    } else {
        phi_template_for_segment(&name)
    }
}

/// LLaMA-Factory `register_model_group` templates for Phi.
/// Longer stems win: `phi-3` is a prefix of `phi-3.5` and `phi-3-small`, and `.` is not
/// an alphanumeric boundary, so the short stem would also match those names.
/// HF cache directories keep the repo id in one segment (`models--microsoft--Phi-3-mini-4k-instruct`).
fn phi_template_for_segment(name: &str) -> Option<&'static str> {
    const GROUPS: &[(&[&str], &str)] = &[
        (&["phi-4-mini", "phi4-mini"], "phi4_mini"),
        (&["phi-4", "phi4"], "phi4"),
        (&["phi-3-small", "phi3-small"], "phi_small"),
        (&["phi-3.5", "phi3.5"], "phi"),
        (&["phi-3", "phi3"], "phi"),
    ];
    for (stems, template) in GROUPS {
        if stems.iter().any(|stem| stem_at_boundary(name, stem)) {
            return Some(*template);
        }
    }
    None
}

/// LLaMA-Factory `register_model_group` templates for Llama.
/// Longer stems win. `llama-3` is a prefix of `llama-3.2`, and `.` is not an
/// alphanumeric boundary, so a short `contains("llama-3")` labels
/// `Llama-3.2-11B-Vision-Instruct` as `llama3`. That group is `mllama`.
/// The same short stem matches `llama3-llava-next-8b-hf` (`llava_next_llama3`).
/// `contains("llama-3")` also matches `llama-30b`, which has no template.
/// `stem_at_boundary` stops on that digit.
/// Text Llama-3, Llama-3.1, Llama-3.2 Instruct, and Llama-3.3 share `template="llama3"`.
/// There is no `llama3_2` name in `constants.py` or `template.py`.
/// HF cache directories keep the repo id in one segment
/// (`models--meta-llama--Llama-3.2-3B-Instruct`).
fn llama_template_for_segment(name: &str) -> Option<&'static str> {
    const GROUPS: &[(&[&str], &str)] = &[
        (
            &[
                "llama-3.2-11b-vision",
                "llama-3.2-90b-vision",
                "llama3.2-11b-vision",
                "llama3.2-90b-vision",
            ],
            "mllama",
        ),
        (
            &["llama3-llava-next", "llava-next-llama3"],
            "llava_next_llama3",
        ),
        (&["llama-3", "llama3"], "llama3"),
    ];
    for (stems, template) in GROUPS {
        if stems.iter().any(|stem| stem_at_boundary(name, stem)) {
            return Some(*template);
        }
    }
    None
}

/// LLaMA-Factory `register_model_group` templates for Mistral.
/// Longer stems win. `mistral` is a prefix of `mistral-small` and `mistral-nemo`,
/// and it sits inside `llava-v1.6-mistral`, so `contains("mistral")` labels those
/// groups `mistral`. `constants.py` names them `mistral_small`, `ministral`, and
/// `llava_next_mistral`. `template.py` registers those names. There is no
/// `mistral_7` name.
/// The classic Mistral-7B group (base v0.1, v0.2, and v0.3, plus Instruct v0.1,
/// v0.2, and v0.3) is `template="mistral"`. Mixtral is a different group that
/// uses that same template. `mixtral` does not contain the stem `mistral`
/// (`x` versus `s`), so the 7B stem does not take Mixtral ids.
/// HF cache directories keep the repo id in one segment
/// (`models--mistralai--Mistral-7B-Instruct-v0.3`). The org segment `mistralai`
/// is not a model id: the bytes after `mistral` are alphanumeric.
/// Ministral, Ministral-3, Codestral, Devstral, and Pixtral are other groups.
/// Their hub ids do not use these stems. This scan does not claim them.
fn mistral_template_for_segment(name: &str) -> Option<&'static str> {
    const GROUPS: &[(&[&str], &str)] = &[
        (
            &["llava-v1.6-mistral", "llava-next-mistral"],
            "llava_next_mistral",
        ),
        (&["mistral-small"], "mistral_small"),
        (&["mistral-nemo"], "ministral"),
        (&["mixtral"], "mistral"),
        (&["mistral-7b"], "mistral"),
    ];
    for (stems, template) in GROUPS {
        if stems.iter().any(|stem| stem_at_boundary(name, stem)) {
            return Some(*template);
        }
    }
    None
}

/// LLaMA-Factory `register_model_group` templates for Gemma.
/// Longer stems win. `gemma` is a prefix of `gemma-2`, and `-` is not an
/// alphanumeric boundary, so `contains("gemma")` labels `google/gemma-2-2b-it`
/// as `gemma`. That group is `template="gemma2"` in `constants.py`.
/// `template.py` registers `gemma2`. There is no `gemma_2` name.
/// `gemma-2` is also a prefix of `gemma-2b` (original Gemma 2B, template `gemma`).
/// The next byte is alphanumeric, so `stem_at_boundary` leaves `gemma-2b` and
/// `gemma-7b` on `gemma`. The same boundary leaves Gemma-3 (`gemma-3-4b-it`)
/// off `gemma2`. Current `constants.py` also puts Gemma-3 270M and 1B text in
/// the `gemma2` group. This scan does not claim those ids.
/// HF cache directories keep the repo id in one segment
/// (`models--google--gemma-2-2b-it`).
fn gemma_template_for_segment(name: &str) -> Option<&'static str> {
    const GEMMA2_STEMS: &[&str] = &["gemma-2", "gemma2"];
    if GEMMA2_STEMS.iter().any(|stem| stem_at_boundary(name, stem)) {
        return Some("gemma2");
    }
    if name.contains("gemma") {
        Some("gemma")
    } else {
        None
    }
}

fn stem_at_boundary(haystack: &str, stem: &str) -> bool {
    if stem.is_empty() {
        return false;
    }
    let bytes = haystack.as_bytes();
    let mut start = 0;
    while start < haystack.len() {
        let Some(rel) = haystack[start..].find(stem) else {
            return false;
        };
        let idx = start + rel;
        let end = idx + stem.len();
        let before_ok = idx == 0 || !bytes[idx - 1].is_ascii_alphanumeric();
        let after_ok = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        start = idx + 1;
    }
    false
}

/// QLoRA handoff when the train base is Phi-3 or Phi-3.5 Instruct.
/// Empty for other families and for the unquantized LoRA card.
const PHI_QLORA_REPRODUCE_NOTE: &str = "Reproduce target beside Qwen LoRA/QLoRA. Phi-3 mini, Phi-3 medium, and Phi-3.5 Instruct use LLaMA-Factory template phi. Phi-3-small uses template phi_small. The Qwen LoRA/QLoRA reproduce target stays template qwen, qwen3, or qwen3_nothink. This QLoRA recipe keeps quantization_method bnb and quantization_bit 4. The seat tag and the train base stay separate. This factory does not download weights.";

fn phi_qlora_reproduce_note(method: LlamaFactoryMethod, train_base: &str) -> String {
    if method != LlamaFactoryMethod::Qlora {
        return String::new();
    }
    match llamafactory_template(train_base) {
        "phi" | "phi_small" => format!("{PHI_QLORA_REPRODUCE_NOTE}\n\n"),
        _ => String::new(),
    }
}

/// QLoRA handoff when the train base is Llama-3.2 Instruct (1B or 3B).
/// Empty for vision (`mllama`), for other `llama3` families, and for the LoRA card.
const LLAMA32_QLORA_REPRODUCE_NOTE: &str = "Reproduce target beside Phi-3 and Qwen LoRA/QLoRA. Llama-3.2 Instruct (Llama-3.2-1B-Instruct and Llama-3.2-3B-Instruct) uses LLaMA-Factory template llama3. Llama-3.2 vision uses template mllama. The Phi-3 reproduce target stays template phi or phi_small. The Qwen LoRA/QLoRA reproduce target stays template qwen, qwen3, or qwen3_nothink. This QLoRA recipe keeps quantization_method bnb and quantization_bit 4. The seat tag and the train base stay separate. This factory does not download weights.";

fn llama32_qlora_reproduce_note(method: LlamaFactoryMethod, train_base: &str) -> String {
    if method != LlamaFactoryMethod::Qlora {
        return String::new();
    }
    let Some(segment) = llamafactory_template_segment(train_base) else {
        return String::new();
    };
    if segment_is_llama32_text_instruct(segment) {
        format!("{LLAMA32_QLORA_REPRODUCE_NOTE}\n\n")
    } else {
        String::new()
    }
}

/// QLoRA handoff when the train base is Gemma-2 Instruct (`-it`).
/// Empty for a Gemma-2 base, for original Gemma, for Gemma-3, and for the LoRA card.
const GEMMA2_QLORA_REPRODUCE_NOTE: &str = "Reproduce target beside Phi-3, Llama-3.2, and Qwen LoRA/QLoRA. Gemma-2 Instruct (google/gemma-2-2b-it, google/gemma-2-9b-it, and google/gemma-2-27b-it) uses LLaMA-Factory template gemma2. A Gemma-2 base checkpoint uses that same template and is not this reproduce target. Original Gemma (gemma-2b and gemma-7b) stays template gemma. The Phi-3 reproduce target stays template phi or phi_small. The Llama-3.2 Instruct reproduce target stays template llama3. The Qwen LoRA/QLoRA reproduce target stays template qwen, qwen3, or qwen3_nothink. This QLoRA recipe keeps quantization_method bnb and quantization_bit 4. The seat tag and the train base stay separate. This factory does not download weights.";

fn gemma2_qlora_reproduce_note(method: LlamaFactoryMethod, train_base: &str) -> String {
    if method != LlamaFactoryMethod::Qlora {
        return String::new();
    }
    let Some(segment) = llamafactory_template_segment(train_base) else {
        return String::new();
    };
    if segment_is_gemma2_instruct(segment) {
        format!("{GEMMA2_QLORA_REPRODUCE_NOTE}\n\n")
    } else {
        String::new()
    }
}

/// QLoRA handoff when the train base is Mistral-7B Instruct (v0.1, v0.2, or v0.3).
/// Empty for a Mistral-7B base, for Mixtral, for Mistral-Small, for Mistral-Nemo,
/// for LLaVA-NeXT-Mistral, and for the LoRA card.
const MISTRAL_QLORA_REPRODUCE_NOTE: &str = "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, and Qwen LoRA/QLoRA. Mistral Instruct (mistralai/Mistral-7B-Instruct-v0.1, mistralai/Mistral-7B-Instruct-v0.2, and mistralai/Mistral-7B-Instruct-v0.3) uses LLaMA-Factory template mistral. A Mistral-7B base checkpoint (Mistral-7B-v0.1, Mistral-7B-v0.2, and Mistral-7B-v0.3) uses that same template and is not this reproduce target. Mistral-Small uses template mistral_small. Mistral-Nemo uses template ministral. Mixtral uses template mistral and is not this reproduce target. LLaVA-NeXT-Mistral uses template llava_next_mistral. The Phi-3 reproduce target stays template phi or phi_small. The Llama-3.2 Instruct reproduce target stays template llama3. The Gemma-2 Instruct reproduce target stays template gemma2. The Qwen LoRA/QLoRA reproduce target stays template qwen, qwen3, or qwen3_nothink. This QLoRA recipe keeps quantization_method bnb and quantization_bit 4. The seat tag and the train base stay separate. This factory does not download weights.";

fn mistral_qlora_reproduce_note(method: LlamaFactoryMethod, train_base: &str) -> String {
    if method != LlamaFactoryMethod::Qlora {
        return String::new();
    }
    let Some(segment) = llamafactory_template_segment(train_base) else {
        return String::new();
    };
    if segment_is_mistral_instruct(segment) {
        format!("{MISTRAL_QLORA_REPRODUCE_NOTE}\n\n")
    } else {
        String::new()
    }
}

fn qlora_reproduce_notes(method: LlamaFactoryMethod, train_base: &str) -> String {
    let mut note = phi_qlora_reproduce_note(method, train_base);
    note.push_str(&llama32_qlora_reproduce_note(method, train_base));
    note.push_str(&gemma2_qlora_reproduce_note(method, train_base));
    note.push_str(&mistral_qlora_reproduce_note(method, train_base));
    note
}

/// The segment that supplied `llamafactory_template`. Later segments that name
/// no family are skipped, the same walk as the template itself.
fn llamafactory_template_segment(train_base: &str) -> Option<&str> {
    train_base_segments(train_base)
        .into_iter()
        .rev()
        .find(|segment| template_for_segment(segment).is_some())
}

fn segment_is_llama32_text_instruct(segment: &str) -> bool {
    let name = segment.to_ascii_lowercase();
    if llama_template_for_segment(&name) != Some("llama3") {
        return false;
    }
    let is_32 = stem_at_boundary(&name, "llama-3.2") || stem_at_boundary(&name, "llama3.2");
    is_32 && name.contains("instruct")
}

/// Hub ids end in `-it` (`google/gemma-2-2b-it`). A local directory may use the
/// LLaMA-Factory display name `Gemma-2-2B-Instruct`.
fn segment_is_gemma2_instruct(segment: &str) -> bool {
    let name = segment.to_ascii_lowercase();
    if gemma_template_for_segment(&name) != Some("gemma2") {
        return false;
    }
    stem_at_boundary(&name, "it") || stem_at_boundary(&name, "instruct")
}

/// Hub ids are `Mistral-7B-Instruct-v0.1`, `v0.2`, and `v0.3`.
/// A base id (`Mistral-7B-v0.1`, `v0.2`, `v0.3`) uses template `mistral` and is
/// not this reproduce target. Mixtral Instruct shares that template and is not
/// this target either.
fn segment_is_mistral_instruct(segment: &str) -> bool {
    let name = segment.to_ascii_lowercase();
    if mistral_template_for_segment(&name) != Some("mistral") {
        return false;
    }
    if stem_at_boundary(&name, "mixtral") {
        return false;
    }
    stem_at_boundary(&name, "mistral-7b") && name.contains("instruct")
}

/// Chat template hint from the train base path. Confirm it before train. Seat the same chat format.
/// The last segment wins when it names a family. A leaf such as `weights` or an HF snapshot hash
/// walks toward the root until a segment names one. Unknown paths stay `default`.
fn llamafactory_template(train_base: &str) -> &'static str {
    for segment in train_base_segments(train_base).into_iter().rev() {
        if let Some(template) = template_for_segment(segment) {
            return template;
        }
    }
    "default"
}

fn llamafactory_dataset_info() -> String {
    format!(
        "{{\n  \"{name}\": {{\n    \"file_name\": \"dataset.jsonl\",\n    \"formatting\": \"sharegpt\",\n    \"columns\": {{\n      \"messages\": \"messages\"\n    }},\n    \"tags\": {{\n      \"role_tag\": \"role\",\n      \"content_tag\": \"content\",\n      \"user_tag\": \"user\",\n      \"assistant_tag\": \"assistant\"\n    }}\n  }}\n}}\n",
        name = LLAMAFACTORY_DATASET_NAME,
    )
}

struct LlamaFactoryScale {
    cutoff: u32,
    grad_accum: u32,
    epochs: &'static str,
    warmup: &'static str,
}

fn llamafactory_scale(official_scale: bool) -> LlamaFactoryScale {
    if official_scale {
        LlamaFactoryScale {
            cutoff: LLAMAFACTORY_OFFICIAL_CUTOFF_LEN,
            grad_accum: LLAMAFACTORY_OFFICIAL_GRAD_ACCUM,
            epochs: "3.0",
            warmup: "0.1",
        }
    } else {
        LlamaFactoryScale {
            cutoff: LLAMAFACTORY_CUTOFF_LEN,
            grad_accum: LLAMAFACTORY_SMOKE_GRAD_ACCUM,
            epochs: "1.0",
            warmup: "0.03",
        }
    }
}

fn llamafactory_adapter_dir(job: &EnrichJob) -> PathBuf {
    job.out_dir.join("outputs")
}

fn llamafactory_export_dir(job: &EnrichJob) -> PathBuf {
    job.out_dir.join("export")
}

fn llamafactory_scale_comment(official_scale: bool, cutoff: u32) -> String {
    if official_scale {
        "# Official scale from examples/train_lora/qwen3_lora_sft.yaml: cutoff_len 2048, num_train_epochs 3.0, gradient_accumulation_steps 8, warmup_ratio 0.1.\n".into()
    } else {
        format!("# Smoke-scale cutoff_len is {cutoff}. Official SFT examples use 2048 for a longer run.\n")
    }
}

fn llamafactory_recipe_yaml(
    job: &EnrichJob,
    mode: &str,
    train_base: &str,
    method: LlamaFactoryMethod,
) -> String {
    let template = llamafactory_template(train_base);
    let outputs = llamafactory_adapter_dir(job);
    let scale = llamafactory_scale(job.official_scale);
    let host = yaml_comment_line(&llamafactory_host_note(
        method.driver_id(),
        &job.host_class_affinity,
    ));
    let save_steps = llamafactory_save_steps(job.max_steps);
    let gauge = match job.max_steps {
        Some(steps) => format!(
            "# Gauge run. max_steps {steps} overrides num_train_epochs.\n\
             max_steps: {steps}\n"
        ),
        None if job.official_scale => {
            "# Official scale keeps num_train_epochs 3.0. A short gauge run passes --max-steps. This file leaves max_steps unset.\n"
                .to_string()
        }
        None => {
            "# One epoch. A short gauge run passes --max-steps. This file leaves max_steps unset.\n"
                .to_string()
        }
    };
    let (method_comment, quant_body) = match method {
        LlamaFactoryMethod::Qlora => (
            "# QLoRA is finetuning_type lora plus quantization_bit 4.\n\
             # quantization_method is bnb. That is the LLaMA-Factory 0.9 token that selects the 4-bit bitsandbytes branch.\n",
            "quantization_bit: 4\nquantization_method: bnb\n",
        ),
        LlamaFactoryMethod::Lora => (
            "# LoRA is finetuning_type lora on a 16-bit base. This file does not quantize.\n\
             # lora_rank 8 matches examples/train_lora/qwen3_lora_sft.yaml. lora_alpha is lora_rank times 2.\n\
             # packing is false. That official file leaves packing unset. The LLaMA-Factory default is false.\n",
            "",
        ),
    };
    let packing = if method.packing() { "true" } else { "false" };
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
         # template is inferred by scanning path segments of the train base, starting at the last segment.\n\
         # A leaf such as weights or an HF snapshot hash uses the nearest ancestor that names a family.\n\
         # A Qwen3 name containing instruct and not thinking, or containing nothink, uses qwen3_nothink.\n\
         # Other Qwen3 names use qwen3. Older Qwen names use qwen.\n\
         # Phi-3 mini, Phi-3 medium, and Phi-3.5 use phi. Phi-3-small uses phi_small.\n\
         # Phi-4 uses phi4. Phi-4-mini uses phi4_mini.\n\
         # Llama-3, Llama-3.1, Llama-3.2 Instruct, and Llama-3.3 use llama3.\n\
         # Llama-3.2 vision uses mllama. llama3-llava-next uses llava_next_llama3.\n\
         # A short llama-3 stem does not label those names llama3. llama-30b stays default.\n\
         # Gemma-2 (gemma-2-2b, gemma-2-9b, gemma-2-27b, and the -it instruct ids) uses gemma2.\n\
         # A short gemma stem does not label those names gemma. gemma-2b and gemma-7b stay gemma.\n\
         # Gemma-3 is a different group. This scan does not label it gemma2.\n\
         # Mistral-7B (v0.1, v0.2, v0.3, and the Instruct ids) uses mistral.\n\
         # Mistral-7B-Instruct is the reproduce target. A Mistral-7B base uses that same template.\n\
         # Mistral-Small uses mistral_small. Mistral-Nemo uses ministral.\n\
         # Mixtral uses mistral and is not the Mistral Instruct reproduce target.\n\
         # LLaVA-NeXT-Mistral uses llava_next_mistral.\n\
         # A short mistral stem does not label Mistral-Small, Mistral-Nemo, or LLaVA-NeXT-Mistral as mistral.\n\
         # Ministral, Ministral-3, Codestral, Devstral, and Pixtral are different groups. This scan does not label them mistral.\n\
         # Use this same chat template when you seat the model.\n\
         # This factory does not map the seat tag onto a Hub repo.\n\
         # dataset_mode: {mode}\n\
         # {host}\n\
         {method_comment}\
         {scale_comment}\
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
         {quant_body}\
         dataset: {dataset_name}\n\
         dataset_dir: {dataset_dir}\n\
         template: {template}\n\
         cutoff_len: {cutoff}\n\
         packing: {packing}\n\
         preprocessing_num_workers: 1\n\
         dataloader_num_workers: 1\n\
         output_dir: {outputs}\n\
         logging_steps: 1\n\
         save_steps: {save_steps}\n\
         overwrite_output_dir: true\n\
         save_only_model: false\n\
         report_to: none\n\
         per_device_train_batch_size: 1\n\
         gradient_accumulation_steps: {grad_accum}\n\
         learning_rate: 1.0e-4\n\
         num_train_epochs: {epochs}\n\
         {gauge}\
         lr_scheduler_type: cosine\n\
         warmup_ratio: {warmup}\n\
         bf16: true\n\
         seed: {seed}\n",
        schema = PREPARE_SCHEMA,
        driver = method.driver_id(),
        pack = job.pack_id,
        seat = job.base_model,
        train_comment = train_base,
        cutoff = scale.cutoff,
        grad_accum = scale.grad_accum,
        epochs = scale.epochs,
        warmup = scale.warmup,
        scale_comment = llamafactory_scale_comment(job.official_scale, scale.cutoff),
        rank = method.lora_rank(),
        alpha = method.lora_alpha(),
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
        method_comment = method_comment,
        quant_body = quant_body,
        packing = packing,
    )
}

fn llamafactory_export_yaml(job: &EnrichJob, train_base: &str, driver_id: &str) -> String {
    let template = llamafactory_template(train_base);
    let adapter = llamafactory_adapter_dir(job);
    let export_dir = llamafactory_export_dir(job);
    format!(
        "# schema: {schema}\n\
         # driver: {driver}\n\
         # seat_tag: {seat}\n\
         # train_base: {train_comment}\n\
         # merge_status: not_run\n\
         # This prepare did not merge. The merge has not happened.\n\
         # export_dir is empty until llamafactory-cli export exits 0.\n\
         # Merge only. Leave this file unquantized. Do not merge a quantized base.\n\
         # model_name_or_path is the train base: the unquantized Hugging Face repo or local HF weights you trained from.\n\
         # The seat tag is the Ollama id for Modelfile FROM. It is a different field.\n\
         # adapter_name_or_path equals recipe.yaml output_dir.\n\
         # After a finished train, adapter_config.json is in that directory.\n\
         # An early stop may leave the adapter only under checkpoint-<step> inside that directory. Point adapter_name_or_path at that checkpoint directory. This prepare does not rewrite this file after train.\n\
         # A Modelfile that llamafactory-cli export writes into export_dir belongs to that tool. This factory did not write it.\n\
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
        driver = driver_id,
        seat = job.base_model,
        train_comment = train_base,
        train = yaml_quote(train_base),
        adapter = yaml_quote(&adapter.display().to_string()),
        export_dir = yaml_quote(&export_dir.display().to_string()),
    )
}

const AXOLOTL_SAVE_STEPS: u32 = 50;

fn yaml_bool(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn axolotl_save_steps(max_steps: u32) -> u32 {
    if max_steps < AXOLOTL_SAVE_STEPS {
        max_steps
    } else {
        AXOLOTL_SAVE_STEPS
    }
}

/// Checkpoint block. Axolotl refuses `save_steps` together with `saves_per_epoch`.
fn axolotl_checkpoint_yaml(max_steps: Option<u32>) -> String {
    match max_steps {
        Some(steps) => format!(
            "# Gauge run. max_steps {steps} precedes num_epochs. save_steps and saves_per_epoch cannot both be set.\n\
             max_steps: {steps}\n\
             save_steps: {save}\n",
            save = axolotl_save_steps(steps),
        ),
        None => "# num_epochs above is the full run. A short gauge run passes --max-steps. This file leaves max_steps unset.\n\
             saves_per_epoch: 1\n"
            .to_string(),
    }
}

fn axolotl_shape_note(method: AxolotlMethod) -> String {
    let example = method.example_file();
    match method {
        AxolotlMethod::Lora => format!(
            "This card is bf16 LoRA (`adapter: lora`, `load_in_8bit: false`, `load_in_4bit: false`). It matches Axolotl `{example}`, the file the quickstart runs with `axolotl train`. That file leaves both load_in flags unset, which is the same bf16 LoRA. This card writes them false. `sequence_len` is {}, `micro_batch_size` is {}, `gradient_accumulation_steps` is {}, and `lora_r` is {}, matching that file. `lora_alpha` is {} and `lora_dropout` is 0.05, matching that file. `lora_target_linear` is true. That file lists Llama-3 projections. This card targets every linear module so a Qwen or Llama train base loads without a module-list edit. `num_epochs` is {}, `optimizer` is {}, `learning_rate` is 0.0002, and `warmup_ratio` is 0.1, matching that file. `sample_packing` is true, matching that file. `pad_to_sequence_len` stays unset, matching that file. `val_set_size` is 0.0 and `evals_per_epoch` is 0. That file uses 0.1 and 4. A short scaffold does not split an eval set, and Axolotl refuses eval settings when `val_set_size` is 0. That file sets `attn_implementation: flash_attention_2` and a Llama pad token. This card leaves both unset. The train base is not pinned to Llama-3, and this factory does not install flash attention. 4-bit QLoRA is `--driver axolotl-qlora`.",
            method.sequence_len(),
            method.micro_batch_size(),
            method.gradient_accumulation_steps(),
            method.lora_r(),
            method.lora_alpha(),
            method.num_epochs(),
            method.optimizer(),
            example = example,
        ),
        AxolotlMethod::Qlora => format!(
            "This card is 4-bit QLoRA (`load_in_8bit: false`, `load_in_4bit: true`, `adapter: qlora`). It matches Axolotl `{example}`. `sequence_len` is {}, `micro_batch_size` is {}, `gradient_accumulation_steps` is {}, and `lora_r` is {}, matching that file. `lora_alpha` is {} and `lora_dropout` is 0.05, matching that file. `lora_target_linear` is true, matching that file. `num_epochs` is {}, `optimizer` is {}, `learning_rate` is 0.0002, and `warmup_ratio` is 0.1, matching that file. `sample_packing` is true, matching that file. `val_set_size` is 0.0, matching that file's 0. `evals_per_epoch` is 0. That file sets 4, which asks for eval on an empty split, and Axolotl refuses eval settings when `val_set_size` is 0. That file sets `attn_implementation: flash_attention_2` and a Llama pad token. This card leaves both unset. The train base is not pinned to Llama-3, and this factory does not install flash attention. bf16 LoRA is `--driver axolotl-lora`.",
            method.sequence_len(),
            method.micro_batch_size(),
            method.gradient_accumulation_steps(),
            method.lora_r(),
            method.lora_alpha(),
            method.num_epochs(),
            method.optimizer(),
            example = example,
        ),
    }
}

fn axolotl_gauge_note(
    method: AxolotlMethod,
    max_steps: Option<u32>,
    official_scale: bool,
) -> String {
    let epochs = method.num_epochs();
    let base = format!(
        "A short gauge run does not need the full num_epochs ({epochs}). Re-prepare with `--max-steps 10`. Axolotl's `max_steps` precedes `num_epochs`: when both are set, training stops at `max_steps`. When that count is under {AXOLOTL_SAVE_STEPS}, this prepare sets `save_steps` to the same count and omits `saves_per_epoch`. Axolotl refuses to set both. The default recipe keeps `num_epochs` {epochs} and `saves_per_epoch` 1, and leaves `max_steps` unset."
    );
    let mut note = match max_steps {
        Some(steps) => format!("{base}\n\nThis recipe is a gauge run with max_steps {steps}."),
        None => base,
    };
    if official_scale {
        note.push_str(&format!(
            "\n\nThis prepare used --official-scale. That flag writes the LLaMA-Factory SFT scale on llamafactory-lora and llamafactory-qlora. This card stays on {}: num_epochs {}, gradient_accumulation_steps {}, sequence_len {}, warmup_ratio 0.1.",
            method.example_file(),
            method.num_epochs(),
            method.gradient_accumulation_steps(),
            method.sequence_len(),
        ));
    }
    note
}

fn axolotl_prepare_steps(
    method: AxolotlMethod,
    job: &EnrichJob,
    train_base: &str,
    data: &DatasetMaterial,
) -> String {
    let driver_id = method.driver_id();
    let host = axolotl_host_note(driver_id, &job.host_class_affinity);
    let data_note = dataset_card_note(ALPACA_DATASET_SHAPE, data);
    let recipe_line = match method {
        AxolotlMethod::Lora => format!(
            "The recipe is bf16 LoRA (`adapter: lora`, `load_in_8bit: false`, `load_in_4bit: false`, `sequence_len` {}, `micro_batch_size` {}, `gradient_accumulation_steps` {}, `lora_r` {}).",
            method.sequence_len(),
            method.micro_batch_size(),
            method.gradient_accumulation_steps(),
            method.lora_r(),
        ),
        AxolotlMethod::Qlora => format!(
            "The recipe is 4-bit QLoRA (`load_in_8bit: false`, `load_in_4bit: true`, `adapter: qlora`, `sequence_len` {}, `micro_batch_size` {}, `gradient_accumulation_steps` {}, `lora_r` {}).",
            method.sequence_len(),
            method.micro_batch_size(),
            method.gradient_accumulation_steps(),
            method.lora_r(),
        ),
    };
    format!(
        "This step wrote axolotl.yml and dataset.jsonl. {recipe_line} It did not run axolotl, did not train, did not download weights, and did not rewrite the estate.\n\
         \n\
         {host}\n\
         \n\
         {data_note}\n\
         \n\
         Seat tag is {seat}. That is the Ollama id for Modelfile FROM. It comes from params.model on the local binding, or from a pack model_hint that is already a model tag.\n\
         \n\
         Train base is {train}. axolotl.yml sets base_model to that value. A train base is a Hugging Face repo id (namespace/name) or a local directory of HF weights. This factory did not download weights and does not map the seat tag onto a Hub repo.\n\
         \n\
         {shape}\n\
         \n\
         {gauge}\n\
         \n\
         From this directory:\n\
         \n\
         axolotl train axolotl.yml\n\
         \n\
         This prepare did not merge. After train, `estate enrich merge-adapt` prints Axolotl's `axolotl merge-lora` line and the `output_dir/merged` directory. This factory does not run that merge. Axolotl does not write GGUF. The train command and the post-train ladder (merge-adapt, gguf-convert, local-seat, import-trained) are in NEXT.md.\n",
        seat = job.base_model,
        train = train_base,
        shape = axolotl_shape_note(method),
        gauge = axolotl_gauge_note(method, job.max_steps, job.official_scale),
    )
}

fn axolotl_recipe_yaml(
    job: &EnrichJob,
    mode: &str,
    train_base: &str,
    method: AxolotlMethod,
) -> String {
    let dataset = job.out_dir.join("dataset.jsonl");
    let prepared = job.out_dir.join("dataset_prepared");
    let outputs = job.out_dir.join("outputs");
    let host = yaml_comment_line(&axolotl_host_note(
        method.driver_id(),
        &job.host_class_affinity,
    ));
    let gauge = axolotl_checkpoint_yaml(job.max_steps);
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
         # example: {example}\n\
         # {shape}\n\
         base_model: {base}\n\
         load_in_8bit: false\n\
         load_in_4bit: {load4}\n\
         adapter: {adapter}\n\
         lora_r: {rank}\n\
         lora_alpha: {alpha}\n\
         lora_dropout: 0.05\n\
         lora_target_linear: true\n\
         datasets:\n  - path: {dataset}\n    ds_type: json\n    type: alpaca\n\
         dataset_prepared_path: {prepared}\n\
         val_set_size: 0.0\n\
         output_dir: {outputs}\n\
         sequence_len: {seq}\n\
         sample_packing: true\n\
         micro_batch_size: {micro}\n\
         gradient_accumulation_steps: {accum}\n\
         num_epochs: {epochs}\n\
         optimizer: {optimizer}\n\
         lr_scheduler: cosine\n\
         learning_rate: 0.0002\n\
         bf16: auto\n\
         tf32: false\n\
         gradient_checkpointing: true\n\
         warmup_ratio: 0.1\n\
         logging_steps: 1\n\
         evals_per_epoch: 0\n\
         {gauge}",
        schema = PREPARE_SCHEMA,
        driver = method.driver_id(),
        pack = job.pack_id,
        seat = job.base_model,
        train_comment = train_base,
        example = method.example_file(),
        shape = yaml_comment_line(&axolotl_shape_note(method)),
        base = yaml_quote(train_base),
        load4 = yaml_bool(method.load_in_4bit()),
        adapter = method.adapter(),
        rank = method.lora_r(),
        alpha = method.lora_alpha(),
        dataset = yaml_quote(&dataset.display().to_string()),
        prepared = yaml_quote(&prepared.display().to_string()),
        outputs = yaml_quote(&outputs.display().to_string()),
        seq = method.sequence_len(),
        micro = method.micro_batch_size(),
        accum = method.gradient_accumulation_steps(),
        epochs = method.num_epochs(),
        optimizer = method.optimizer(),
        mode = mode,
        host = host,
        gauge = gauge,
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

/// One `prepare.json` under `{state_dir}/enrich/{pack}/{driver}`.
///
/// This is the file prepare wrote. It is not evidence that the factory
/// trained, merged, converted, or seated a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainPrepareFact {
    pub pack_id: String,
    pub driver: String,
    pub job: String,
    /// `prepare.json` `seat_tag` when that field is present.
    pub seat_tag: Option<String>,
    /// `prepare.json` `train_base_model` when that field is present.
    pub train_base: Option<String>,
    /// `adapter`, `merged`, or `gguf` when `import-trained` recorded it.
    pub trained_shape: Option<String>,
    pub out_dir: PathBuf,
}

/// In-tree train/enrich card. Status and doctor print this catalog.
/// A prepare probe is not a live run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainCatalogFact {
    pub driver_id: String,
    pub status: String,
}

const TRAIN_CATALOG_STATUSES: &[&str] = &["integration", "optional", "portable"];

/// Cards in `REGISTRY`. `live` is not a field here. Callers print `live=false`.
pub fn train_catalog_facts() -> Result<Vec<TrainCatalogFact>, ModelError> {
    let mut rows = Vec::new();
    for card in train_enrich_catalog() {
        if !TRAIN_CATALOG_STATUSES.contains(&card.status) {
            return Err(ModelError::Other(format!(
                "refuse:train-catalog: {} status '{}' is not integration, optional, or portable",
                card.driver_id, card.status
            )));
        }
        let driver = resolve_train_enrich_driver(card.driver_id)?;
        let probe = driver.probe();
        if probe.live {
            return Err(ModelError::Other(format!(
                "refuse:train-catalog: {} prepare probe set live=true. A prepare probe does not run.",
                card.driver_id
            )));
        }
        if probe.status != card.status {
            return Err(ModelError::Other(format!(
                "refuse:train-catalog: {} probe status '{}' does not match the card status '{}'",
                card.driver_id, probe.status, card.status
            )));
        }
        rows.push(TrainCatalogFact {
            driver_id: card.driver_id.to_string(),
            status: card.status.to_string(),
        });
    }
    Ok(rows)
}

/// Read-only prepare tree. `Ok(None)` when `{state_dir}/enrich` is missing.
/// A missing directory does not invent a prepare count. A symlink, a
/// `prepare.json` that does not parse, or a path that leaves `state_dir`
/// refuses.
pub fn train_prepare_facts(state_dir: &Path) -> Result<Option<Vec<TrainPrepareFact>>, ModelError> {
    let enrich = state_dir.join("enrich");
    match enrich_root_kind(&enrich)? {
        EnrichRootKind::Missing => Ok(None),
        EnrichRootKind::Directory => {
            let containment = pin_enrich_dir(state_dir, "cell state directory")?;
            Ok(Some(read_enrich_prepares(&enrich, &containment)?))
        }
    }
}

/// Read `.cell/enrich`. Does not create the directory and does not write.
pub fn list_prepared(enrich_root: &Path) -> Result<Vec<PreparedEntry>, ModelError> {
    let facts = require_enrich_prepares(enrich_root)?;
    Ok(facts
        .into_iter()
        .map(|fact| PreparedEntry {
            local_tag: local_enrich_tag(&fact.pack_id),
            pack_id: fact.pack_id,
            driver: fact.driver,
            job: fact.job,
            out_dir: fact.out_dir,
        })
        .collect())
}

enum EnrichRootKind {
    Missing,
    Directory,
}

fn enrich_root_kind(enrich_root: &Path) -> Result<EnrichRootKind, ModelError> {
    match std::fs::symlink_metadata(enrich_root) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(EnrichRootKind::Missing),
        Err(err) => Err(ModelError::Other(format!(
            "refuse:enrich-index: {}: {err}",
            enrich_root.display()
        ))),
        Ok(meta) if meta.file_type().is_symlink() => Err(symlink_enrich(enrich_root)),
        Ok(meta) if meta.is_dir() => Ok(EnrichRootKind::Directory),
        Ok(_) => Err(ModelError::Other(format!(
            "refuse:enrich-index: {} is not a directory",
            enrich_root.display()
        ))),
    }
}

fn require_enrich_prepares(enrich_root: &Path) -> Result<Vec<TrainPrepareFact>, ModelError> {
    match enrich_root_kind(enrich_root)? {
        EnrichRootKind::Missing => Err(ModelError::Other(format!(
            "refuse:enrich-index: {} is missing",
            enrich_root.display()
        ))),
        EnrichRootKind::Directory => {
            let parent = enrich_root
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            let containment = pin_enrich_dir(parent, "cell state directory")?;
            read_enrich_prepares(enrich_root, &containment)
        }
    }
}

fn read_enrich_prepares(
    enrich_root: &Path,
    state_canon: &Path,
) -> Result<Vec<TrainPrepareFact>, ModelError> {
    let enrich_canon = pin_enrich_dir(enrich_root, "enrich directory")?;
    if !path_is_within(state_canon, &enrich_canon) {
        return Err(escaped_enrich(enrich_root, &enrich_canon, state_canon));
    }
    let mut rows = Vec::new();
    for pack_dir in enrich_child_dirs(enrich_root, state_canon)? {
        let pack_name = file_name(&pack_dir)?;
        for driver_dir in enrich_child_dirs(&pack_dir, state_canon)? {
            let driver_name = file_name(&driver_dir)?;
            let prepare_path = driver_dir.join("prepare.json");
            let doc = read_prepare_contained(&prepare_path, &driver_dir, state_canon)?;
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
            rows.push(TrainPrepareFact {
                pack_id: doc.pack_id,
                driver: doc.driver,
                job: doc.job,
                seat_tag: present_text(doc.seat_tag),
                train_base: present_text(doc.train_base_model),
                trained_shape: present_text(doc.trained_shape),
                out_dir: driver_dir,
            });
        }
    }
    rows.sort_by(|a, b| (&a.pack_id, &a.driver).cmp(&(&b.pack_id, &b.driver)));
    Ok(rows)
}

fn present_text(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.trim().is_empty())
}

/// `Ok(true)` for a regular file. A symlink refuses. Missing is `Ok(false)`.
fn enrich_regular_file(path: &Path) -> Result<bool, ModelError> {
    match std::fs::symlink_metadata(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(ModelError::Other(format!(
            "refuse:proposal: {}: {err}",
            path.display()
        ))),
        Ok(meta) if meta.file_type().is_symlink() => Err(symlink_enrich(path)),
        Ok(meta) if meta.is_file() => Ok(true),
        Ok(_) => Err(ModelError::Other(format!(
            "refuse:proposal: {} is not a regular file",
            path.display()
        ))),
    }
}

fn pin_enrich_dir(path: &Path, kind: &str) -> Result<PathBuf, ModelError> {
    std::fs::canonicalize(path).map_err(|err| {
        ModelError::Other(format!(
            "refuse:enrich-index: cannot pin {kind} {}: {err}",
            path.display()
        ))
    })
}

fn symlink_enrich(path: &Path) -> ModelError {
    ModelError::Other(format!(
        "refuse:enrich-index: {} is a symlink. The enrich tree does not follow symlinks and stays inside the cell state directory.",
        path.display()
    ))
}

fn escaped_enrich(path: &Path, pinned: &Path, containment: &Path) -> ModelError {
    ModelError::Other(format!(
        "refuse:enrich-index: {} resolves to {} outside {}. The enrich tree stays inside the cell state directory.",
        path.display(),
        pinned.display(),
        containment.display()
    ))
}

fn enrich_child_dirs(dir: &Path, containment: &Path) -> Result<Vec<PathBuf>, ModelError> {
    let mut paths = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|err| {
        ModelError::Other(format!("refuse:enrich-index: {}: {err}", dir.display()))
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            ModelError::Other(format!("refuse:enrich-index: {}: {err}", dir.display()))
        })?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|err| {
            ModelError::Other(format!("refuse:enrich-index: {}: {err}", path.display()))
        })?;
        if kind.is_symlink() {
            return Err(symlink_enrich(&path));
        }
        if !kind.is_dir() {
            return Err(ModelError::Other(format!(
                "refuse:enrich-index: unexpected file {}",
                path.display()
            )));
        }
        let canon = pin_enrich_dir(&path, "enrich directory")?;
        if !path_is_within(containment, &canon) {
            return Err(escaped_enrich(&path, &canon, containment));
        }
        paths.push(path);
    }
    paths.sort();
    Ok(paths)
}

fn read_prepare_contained(
    path: &Path,
    driver_dir: &Path,
    state_canon: &Path,
) -> Result<EnrichPrepareDoc, ModelError> {
    match std::fs::symlink_metadata(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(ModelError::Other(format!(
                "refuse:missing-prepare: {}",
                path.display()
            )));
        }
        Err(err) => {
            return Err(ModelError::Other(format!(
                "refuse:prepare-unreadable: {}: {err}",
                path.display()
            )));
        }
        Ok(meta) if meta.file_type().is_symlink() => return Err(symlink_enrich(path)),
        Ok(meta) if !meta.is_file() => {
            return Err(ModelError::Other(format!(
                "refuse:prepare-unreadable: {} is not a regular file",
                path.display()
            )));
        }
        Ok(_) => {}
    }
    let file = open_nofollow(path).map_err(|err| {
        if matches!(err.raw_os_error(), Some(40 | 62)) {
            symlink_enrich(path)
        } else {
            ModelError::Other(format!(
                "refuse:prepare-unreadable: cannot open {}: {err}",
                path.display()
            ))
        }
    })?;
    let opened = opened_file_path(&file).map_err(|err| {
        ModelError::Other(format!(
            "refuse:enrich-index: cannot pin {} ({err}). The enrich tree refuses when the opened prepare.json cannot be resolved.",
            path.display()
        ))
    })?;
    let pinned = std::fs::canonicalize(&opened).map_err(|err| {
        ModelError::Other(format!(
            "refuse:enrich-index: cannot pin {} ({err}). The enrich tree refuses when the opened prepare.json cannot be resolved.",
            path.display()
        ))
    })?;
    let driver_canon = pin_enrich_dir(driver_dir, "prepare directory")?;
    if !pinned.is_absolute()
        || !path_is_within(&driver_canon, &pinned)
        || !path_is_within(state_canon, &pinned)
    {
        return Err(escaped_enrich(path, &pinned, state_canon));
    }
    let text = read_opened_utf8(file, path)?;
    parse_prepare_doc(path, &text)
}

fn read_opened_utf8(mut file: std::fs::File, path: &Path) -> Result<String, ModelError> {
    use std::io::Read;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|err| {
        ModelError::Other(format!(
            "refuse:prepare-unreadable: {}: {err}",
            path.display()
        ))
    })?;
    String::from_utf8(buf).map_err(|_| {
        ModelError::Other(format!(
            "refuse:prepare-unreadable: {} is not UTF-8",
            path.display()
        ))
    })
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
    /// `adapter`, `merged`, or `gguf` when `import-trained` accepted an artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trained_shape: Option<String>,
    /// Operator path plus the marker files that proved `trained_shape`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trained_paths: Option<Vec<String>>,
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
    let proposal = compose_import_proposal(req, None)?;
    persist_binding_proposal(req.prepared_dir, &proposal)?;
    Ok(proposal)
}

/// Build the `local_slm` proposal. `scanned_override` skips a second open of
/// the operator file when `import_trained` already pinned that handle.
fn compose_import_proposal(
    req: &ImportPreparedRequest<'_>,
    scanned_override: Option<bool>,
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
    let content_scanned = match scanned_override {
        Some(scanned) => scanned,
        None => scan_operator_file(req.path)?,
    };
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
        trained_shape: None,
        trained_paths: None,
        binding_id: "local_slm".into(),
        seated_driver: seat.driver.clone(),
        content_scanned,
        proposed_binding,
        paste_yaml,
        note: format!(
            "Proposal only. auto_apply=false. Next: estate enrich apply-proposal, then estate plan and estate apply --require-plan. import-prepared does not apply, does not promote, and does not rewrite the estate. Operator file content_scanned={content_scanned}."
        ),
    };
    Ok(proposal)
}

fn persist_binding_proposal(
    prepared_dir: &Path,
    proposal: &EnrichBindingProposal,
) -> Result<(), ModelError> {
    let json = to_pretty(proposal)?;
    let md = render_binding_proposal(proposal);
    refuse_sacred_and_sku("binding proposal", &json)?;
    refuse_sacred_and_sku("binding proposal", &md)?;
    refuse_raw_secrets(&json).map_err(map_feed)?;
    refuse_raw_secrets(&md).map_err(map_feed)?;
    write_files(
        prepared_dir,
        &[
            (BINDING_PROPOSAL_JSON.into(), json),
            (BINDING_PROPOSAL_MD.into(), md),
        ],
    )
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
    if !IMPORT_TRAINED_DRIVERS.contains(&doc.driver.as_str()) {
        return Err(ModelError::Other(format!(
            "refuse:driver: import-trained reads a train prepare ({}), found '{}'",
            IMPORT_TRAINED_DRIVERS.join(", "),
            doc.driver
        )));
    }
    if doc.job != EnrichJobKind::Train.as_str() {
        return Err(ModelError::Other(format!(
            "refuse:job: import-trained expects job train, found '{}'",
            doc.job
        )));
    }
    // Classify and scan before any proposal or prepare write. A later publish
    // failure restores the previous bytes so apply never sees a proposal that
    // omitted trained_shape and trained_paths.
    let artifact = classify_trained_artifact(req.adapter)?;
    let content_scanned = scan_trained_sidecars(&artifact)?;
    let mut proposal = compose_import_proposal(
        &ImportPreparedRequest {
            estate: req.estate,
            prepared_dir: req.prepared_dir,
            tag: req.tag,
            path: &artifact.primary,
            curator: req.curator,
        },
        Some(content_scanned),
    )?;
    proposal.trained_shape = Some(artifact.shape.to_string());
    proposal.trained_paths = Some(artifact.paths.clone());
    proposal.note = format!(
        "Proposal only. auto_apply=false. Trained shape is {shape}. Paths: {paths}. Next: estate enrich apply-proposal, then estate plan and estate apply --require-plan. import-trained does not apply, does not promote, and does not rewrite the estate. Operator file content_scanned={scanned}.",
        shape = artifact.shape,
        paths = artifact.paths.join(", "),
        scanned = proposal.content_scanned,
    );
    commit_trained_import(req.prepared_dir, &proposal, &artifact)?;
    Ok(proposal)
}

struct PinnedMarker {
    path: PathBuf,
    file: std::fs::File,
}

struct TrainedArtifact {
    shape: &'static str,
    /// File scanned and stored as `local_path`.
    primary: PathBuf,
    /// Operator path first, then the marker files that proved the shape.
    paths: Vec<String>,
    /// Marker files opened at classify time. The scan reads these handles.
    pinned: Vec<PinnedMarker>,
}

/// Classify `--adapter` and hold the marker handles used as evidence.
///
/// `DirEntry::file_type` does not follow symlinks. A marker symlink, including
/// one whose target sits outside the artifact directory, is `refuse:adapter`.
/// A symlinked `--adapter` path is the same refuse. Each accepted marker is
/// opened with `O_NOFOLLOW` and pinned with that fd (`opened_file_path`, the
/// same spirit as `--from-feed`). The pin must stay inside the artifact
/// directory. `scan_trained_sidecars` reads the Modelfile and the primary
/// file from those fds, so a later rename of the path does not retarget the
/// scan. A swap that lands between `read_dir` and `open` fails closed:
/// `O_NOFOLLOW` refuses a path that became a symlink. This suite does not
/// reproduce that race in-process. It covers a symlinked marker and a
/// symlinked GGUF as stable refusals.
fn classify_trained_artifact(path: &Path) -> Result<TrainedArtifact, ModelError> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(ModelError::Other(format!(
                "refuse:adapter: {} is missing. {TRAINED_SHAPE_HINT}",
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
    if meta.file_type().is_symlink() {
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} is a symlink. import-trained does not follow a symlinked adapter path. Pass the real directory or the real .gguf file. {TRAINED_SHAPE_HINT}",
            path.display()
        )));
    }
    if meta.is_file() {
        if is_gguf_name(path) {
            let primary = path.to_path_buf();
            let containment = canonicalize_dir(parent_dir(path))?;
            return finish_trained_artifact(
                TRAINED_SHAPE_GGUF,
                primary.clone(),
                vec![path.display().to_string()],
                vec![primary],
                &containment,
            );
        }
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} is a file and is not a GGUF. {TRAINED_SHAPE_HINT}",
            path.display()
        )));
    }
    if !meta.is_dir() {
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} is not a file or directory. {TRAINED_SHAPE_HINT}",
            path.display()
        )));
    }
    let files = top_level_files(path)?;
    let mut adapter_config = None;
    let mut model_config = None;
    let mut modelfile = None;
    let mut merged_weights = Vec::new();
    let mut ggufs = Vec::new();
    let mut adapter_weights = Vec::new();
    for file in &files {
        let name = file_name_lower(file);
        if name == "adapter_config.json" {
            adapter_config = Some(file.clone());
        } else if name == "config.json" {
            model_config = Some(file.clone());
        }
        if file.file_name().and_then(|value| value.to_str()) == Some("Modelfile") {
            modelfile = Some(file.clone());
        }
        if name.ends_with(".safetensors") {
            if is_adapter_safetensors(&name) {
                adapter_weights.push(file.clone());
            } else {
                merged_weights.push(file.clone());
            }
        }
        if is_gguf_name(file) {
            ggufs.push(file.clone());
        }
        if name == "adapter_model.bin" {
            adapter_weights.push(file.clone());
        }
    }
    if ggufs.len() > 1 {
        let names = ggufs
            .iter()
            .map(|file| file_name_lower(file))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} has more than one top-level .gguf file ({names}). Point --adapter at one .gguf file, or at a directory that holds exactly one. {TRAINED_SHAPE_HINT}",
            path.display()
        )));
    }
    let adapter_hit = adapter_config.is_some();
    let merged_hit = model_config.is_some() && !merged_weights.is_empty();
    let gguf_hit = !ggufs.is_empty();
    let hits = usize::from(adapter_hit) + usize::from(merged_hit) + usize::from(gguf_hit);
    if hits > 1 {
        let mut names = Vec::new();
        if adapter_hit {
            names.push(TRAINED_SHAPE_ADAPTER);
        }
        if merged_hit {
            names.push(TRAINED_SHAPE_MERGED);
        }
        if gguf_hit {
            names.push(TRAINED_SHAPE_GGUF);
        }
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} matches more than one trained shape ({}). Point --adapter at one artifact. {TRAINED_SHAPE_HINT}",
            path.display(),
            names.join(", ")
        )));
    }
    let containment = canonicalize_dir(path)?;
    if let Some(config) = adapter_config {
        let mut paths = vec![path.display().to_string(), config.display().to_string()];
        let mut files = vec![config.clone()];
        for weight in adapter_weights {
            let text = weight.display().to_string();
            if !paths.contains(&text) {
                paths.push(text);
                files.push(weight);
            }
        }
        return finish_trained_artifact(TRAINED_SHAPE_ADAPTER, config, paths, files, &containment);
    }
    if merged_hit {
        let config = match model_config {
            Some(config) => config,
            None => {
                return Err(ModelError::Other(format!(
                    "refuse:adapter: {} has no config.json. A merged export_dir needs config.json and at least one .safetensors file whose name does not start with adapter_model.",
                    path.display()
                )))
            }
        };
        let mut paths = vec![path.display().to_string(), config.display().to_string()];
        let mut files = vec![config.clone()];
        for file in merged_weights {
            paths.push(file.display().to_string());
            files.push(file);
        }
        if let Some(file) = modelfile {
            paths.push(file.display().to_string());
            files.push(file);
        }
        return finish_trained_artifact(TRAINED_SHAPE_MERGED, config, paths, files, &containment);
    }
    if gguf_hit {
        let primary = ggufs[0].clone();
        let mut paths = vec![path.display().to_string()];
        for file in &ggufs {
            paths.push(file.display().to_string());
        }
        return finish_trained_artifact(TRAINED_SHAPE_GGUF, primary, paths, ggufs, &containment);
    }
    if model_config.is_some() && !adapter_weights.is_empty() {
        let names = adapter_weights
            .iter()
            .map(|file| file_name_lower(file))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} has config.json and adapter weights ({names}) and no merged weight. adapter_model.safetensors is not a merged export. A merged export_dir needs config.json and a .safetensors file whose name does not start with adapter_model. An adapter output_dir needs adapter_config.json.",
            path.display()
        )));
    }
    if model_config.is_some() {
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} has config.json and no .safetensors file. A merged export_dir needs config.json and at least one .safetensors file whose name does not start with adapter_model. An adapter output_dir needs adapter_config.json.",
            path.display()
        )));
    }
    if !merged_weights.is_empty() {
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} has a .safetensors file and no config.json. A merged export_dir needs both. An adapter output_dir needs adapter_config.json.",
            path.display()
        )));
    }
    if !adapter_weights.is_empty() {
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} has adapter weights and no adapter_config.json. An adapter output_dir needs adapter_config.json.",
            path.display()
        )));
    }
    Err(ModelError::Other(format!(
        "refuse:adapter: {} is not an adapter output_dir, a merged export_dir, or a GGUF path. {TRAINED_SHAPE_HINT}",
        path.display()
    )))
}

fn finish_trained_artifact(
    shape: &'static str,
    primary: PathBuf,
    paths: Vec<String>,
    files: Vec<PathBuf>,
    containment: &Path,
) -> Result<TrainedArtifact, ModelError> {
    let mut pinned = Vec::with_capacity(files.len());
    for file in files {
        pinned.push(pin_regular_file(&file, containment)?);
    }
    if !pinned.iter().any(|marker| marker.path == primary) {
        return Err(ModelError::Other(
            "refuse:adapter: primary marker was not opened".into(),
        ));
    }
    Ok(TrainedArtifact {
        shape,
        primary,
        paths,
        pinned,
    })
}

fn is_gguf_name(path: &Path) -> bool {
    file_name_lower(path).ends_with(".gguf")
}

fn file_name_lower(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn is_adapter_safetensors(name: &str) -> bool {
    name.ends_with(".safetensors") && name.starts_with("adapter_model")
}

fn is_trained_marker_name(name: &std::ffi::OsStr) -> bool {
    let Some(text) = name.to_str() else {
        return false;
    };
    if text == "Modelfile" {
        return true;
    }
    let lower = text.to_ascii_lowercase();
    lower == "adapter_config.json"
        || lower == "config.json"
        || lower == "adapter_model.bin"
        || lower.ends_with(".safetensors")
        || lower.ends_with(".gguf")
}

fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn canonicalize_dir(path: &Path) -> Result<PathBuf, ModelError> {
    std::fs::canonicalize(path).map_err(|err| {
        ModelError::Other(format!(
            "refuse:adapter: cannot pin {}: {err}. import-trained refuses when the artifact directory cannot be resolved.",
            path.display()
        ))
    })
}

/// Open `path` without following a final symlink. Linux `O_NOFOLLOW` is
/// `0400000`. macOS `O_NOFOLLOW` is `0x0100`.
fn open_nofollow(path: &Path) -> std::io::Result<std::fs::File> {
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

fn pin_regular_file(path: &Path, containment: &Path) -> Result<PinnedMarker, ModelError> {
    let file = open_nofollow(path).map_err(|err| {
        // ELOOP: Linux 40, macOS 62. `ErrorKind::FilesystemLoop` is still
        // unstable on the 1.88 toolchain this repo pins.
        if matches!(err.raw_os_error(), Some(40 | 62)) {
            ModelError::Other(format!(
                "refuse:adapter: {} is a symlink. import-trained does not follow marker symlinks. The marker must be a regular file inside {}.",
                path.display(),
                containment.display()
            ))
        } else {
            ModelError::Other(format!(
                "refuse:adapter: cannot open {}: {err}",
                path.display()
            ))
        }
    })?;
    let opened = opened_file_path(&file).map_err(|err| {
        ModelError::Other(format!(
            "refuse:adapter: cannot pin {} ({err}). import-trained refuses when the opened marker cannot be resolved.",
            path.display()
        ))
    })?;
    let pinned = std::fs::canonicalize(&opened).map_err(|err| {
        ModelError::Other(format!(
            "refuse:adapter: cannot pin {} ({err}). import-trained refuses when the opened marker cannot be resolved.",
            path.display()
        ))
    })?;
    if !path_is_within(containment, &pinned) {
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} resolves to {} outside {}. import-trained does not follow marker symlinks out of the artifact directory.",
            path.display(),
            pinned.display(),
            containment.display()
        )));
    }
    let meta = file.metadata().map_err(|err| {
        ModelError::Other(format!(
            "refuse:adapter: cannot stat {}: {err}",
            path.display()
        ))
    })?;
    if !meta.is_file() {
        return Err(ModelError::Other(format!(
            "refuse:adapter: {} is not a regular file. {TRAINED_SHAPE_HINT}",
            path.display()
        )));
    }
    Ok(PinnedMarker {
        path: path.to_path_buf(),
        file,
    })
}

fn top_level_files(dir: &Path) -> Result<Vec<PathBuf>, ModelError> {
    let mut files = Vec::new();
    let entries = std::fs::read_dir(dir)
        .map_err(|err| ModelError::Other(format!("refuse:adapter: {}: {err}", dir.display())))?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            ModelError::Other(format!("refuse:adapter: {}: {err}", dir.display()))
        })?;
        let kind = entry.file_type().map_err(|err| {
            ModelError::Other(format!("refuse:adapter: {}: {err}", dir.display()))
        })?;
        if kind.is_symlink() {
            if is_trained_marker_name(&entry.file_name()) {
                return Err(ModelError::Other(format!(
                    "refuse:adapter: {} is a symlink. import-trained does not follow marker symlinks. The marker must be a regular file inside {}.",
                    entry.path().display(),
                    dir.display()
                )));
            }
            continue;
        }
        if kind.is_file() {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(files)
}

fn scan_trained_sidecars(artifact: &TrainedArtifact) -> Result<bool, ModelError> {
    for path_text in &artifact.paths {
        refuse_sacred_and_sku("trained path", path_text)?;
    }
    for marker in &artifact.pinned {
        if marker.path.file_name().and_then(|name| name.to_str()) != Some("Modelfile") {
            continue;
        }
        let scanned = scan_opened_file(&marker.file, &marker.path)?;
        if !scanned {
            return Err(ModelError::Other(format!(
                "refuse:adapter: {} is not scanned text. import-trained records a Modelfile only when the file is UTF-8 and at most 1 MiB.",
                marker.path.display()
            )));
        }
    }
    let primary = artifact
        .pinned
        .iter()
        .find(|marker| marker.path == artifact.primary)
        .ok_or_else(|| ModelError::Other("refuse:adapter: primary marker was not opened".into()))?;
    scan_opened_file(&primary.file, &primary.path)
}

fn scan_opened_file(file: &std::fs::File, path: &Path) -> Result<bool, ModelError> {
    use std::io::{Read, Seek, SeekFrom};
    let meta = file
        .metadata()
        .map_err(|err| ModelError::Other(format!("refuse:path: {}: {err}", path.display())))?;
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
    let mut file = file
        .try_clone()
        .map_err(|err| ModelError::Other(format!("refuse:path: {}: {err}", path.display())))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|err| ModelError::Other(format!("refuse:path: {}: {err}", path.display())))?;
    let mut buf = Vec::new();
    file.take(CAP.saturating_add(1))
        .read_to_end(&mut buf)
        .map_err(|err| ModelError::Other(format!("refuse:path: {}: {err}", path.display())))?;
    if buf.len() as u64 > CAP {
        return Ok(false);
    }
    let Ok(text) = std::str::from_utf8(&buf) else {
        return Ok(false);
    };
    refuse_sacred_and_sku("operator file", text)?;
    refuse_raw_secrets(text).map_err(map_feed)?;
    Ok(true)
}

fn trained_prepare_body(
    path: &Path,
    prior: &[u8],
    artifact: &TrainedArtifact,
) -> Result<String, ModelError> {
    let text = std::str::from_utf8(prior).map_err(|_| {
        ModelError::Other(format!(
            "refuse:prepare-unreadable: {} is not UTF-8",
            path.display()
        ))
    })?;
    let mut value: serde_json::Value = serde_json::from_str(text).map_err(|err| {
        ModelError::Other(format!(
            "refuse:prepare-unreadable: {}: {err}",
            path.display()
        ))
    })?;
    let obj = value.as_object_mut().ok_or_else(|| {
        ModelError::Other(format!(
            "refuse:prepare: {} is not an object",
            path.display()
        ))
    })?;
    for key in ["promoted", "auto_apply", "estate_rewritten"] {
        if obj.get(key).and_then(|flag| flag.as_bool()) != Some(false) {
            return Err(ModelError::Other(format!(
                "refuse:prepared: {} promoted, auto_apply, and estate_rewritten must stay false",
                path.display()
            )));
        }
    }
    obj.insert(
        "trained_shape".into(),
        serde_json::Value::String(artifact.shape.to_string()),
    );
    obj.insert(
        "trained_paths".into(),
        serde_json::Value::Array(
            artifact
                .paths
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        ),
    );
    let body = to_pretty(&value)?;
    refuse_sacred_and_sku("prepare.json", &body)?;
    refuse_raw_secrets(&body).map_err(map_feed)?;
    Ok(body)
}

const IMPORT_TEMP_NAMES: [&str; 3] = [
    "prepare.json.importing",
    "binding-proposal.json.importing",
    "binding-proposal.md.importing",
];

fn cleanup_import_temps(dir: &Path) {
    for name in IMPORT_TEMP_NAMES {
        let _ = std::fs::remove_file(dir.join(name));
    }
}

fn read_regular_file(path: &Path) -> Result<Option<Vec<u8>>, ModelError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => std::fs::read(path).map(Some).map_err(|err| {
            ModelError::Other(format!(
                "refuse:prepare-unreadable: {}: {err}",
                path.display()
            ))
        }),
        Ok(_) => Ok(None),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(ModelError::Other(format!(
            "refuse:prepare-unreadable: {}: {err}",
            path.display()
        ))),
    }
}

fn restore_or_remove(path: &Path, prior: Option<&[u8]>) {
    match prior {
        Some(bytes) => {
            let _ = std::fs::write(path, bytes);
        }
        None => {
            let is_file = std::fs::symlink_metadata(path)
                .map(|meta| meta.is_file())
                .unwrap_or(false);
            if is_file {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

/// Write the proposal and the prepare record together. Temps are checked
/// before the live names move. On a failed publish, previous prepare and
/// proposal bytes are restored and a new proposal file is removed.
fn commit_trained_import(
    prepared_dir: &Path,
    proposal: &EnrichBindingProposal,
    artifact: &TrainedArtifact,
) -> Result<(), ModelError> {
    if proposal.trained_shape.as_deref() != Some(artifact.shape)
        || proposal.trained_paths.as_ref() != Some(&artifact.paths)
    {
        return Err(ModelError::Other(
            "refuse:prepare: import-trained refuses to write a proposal without the trained shape"
                .into(),
        ));
    }
    let prepare_path = prepared_dir.join("prepare.json");
    let json_path = prepared_dir.join(BINDING_PROPOSAL_JSON);
    let md_path = prepared_dir.join(BINDING_PROPOSAL_MD);
    let prior_prepare = match read_regular_file(&prepare_path)? {
        Some(bytes) => bytes,
        None => {
            return Err(ModelError::Other(format!(
                "refuse:missing-prepare: {}",
                prepare_path.display()
            )))
        }
    };
    let prior_json = read_regular_file(&json_path)?;
    let prior_md = read_regular_file(&md_path)?;
    let prepare_body = trained_prepare_body(&prepare_path, &prior_prepare, artifact)?;
    let json = to_pretty(proposal)?;
    let md = render_binding_proposal(proposal);
    refuse_sacred_and_sku("binding proposal", &json)?;
    refuse_sacred_and_sku("binding proposal", &md)?;
    refuse_raw_secrets(&json).map_err(map_feed)?;
    refuse_raw_secrets(&md).map_err(map_feed)?;

    let prepare_tmp = prepared_dir.join(IMPORT_TEMP_NAMES[0]);
    let json_tmp = prepared_dir.join(IMPORT_TEMP_NAMES[1]);
    let md_tmp = prepared_dir.join(IMPORT_TEMP_NAMES[2]);
    let staged: Result<(), ModelError> = (|| {
        std::fs::write(&prepare_tmp, &prepare_body).map_err(|err| {
            ModelError::Other(format!(
                "refuse:prepare-write: {}: {err}",
                prepare_tmp.display()
            ))
        })?;
        std::fs::write(&json_tmp, &json).map_err(|err| {
            ModelError::Other(format!(
                "refuse:prepare-write: {}: {err}",
                json_tmp.display()
            ))
        })?;
        std::fs::write(&md_tmp, &md).map_err(|err| {
            ModelError::Other(format!("refuse:prepare-write: {}: {err}", md_tmp.display()))
        })?;
        let doc = load_prepare_doc(&prepare_tmp)?;
        if doc.trained_shape.as_deref() != Some(artifact.shape)
            || doc.trained_paths.as_ref() != Some(&artifact.paths)
            || doc.promoted
            || doc.auto_apply
            || doc.estate_rewritten
        {
            return Err(ModelError::Other(
                "refuse:prepare: trained_shape record did not round-trip".into(),
            ));
        }
        Ok(())
    })();
    if let Err(err) = staged {
        cleanup_import_temps(prepared_dir);
        return Err(err);
    }
    let published: Result<(), ModelError> = (|| {
        std::fs::rename(&prepare_tmp, &prepare_path).map_err(|err| {
            ModelError::Other(format!(
                "refuse:prepare-write: cannot replace {}: {err}",
                prepare_path.display()
            ))
        })?;
        std::fs::rename(&json_tmp, &json_path).map_err(|err| {
            ModelError::Other(format!(
                "refuse:prepare-write: cannot replace {}: {err}",
                json_path.display()
            ))
        })?;
        std::fs::rename(&md_tmp, &md_path).map_err(|err| {
            ModelError::Other(format!(
                "refuse:prepare-write: cannot replace {}: {err}",
                md_path.display()
            ))
        })?;
        Ok(())
    })();
    if let Err(err) = published {
        restore_or_remove(&prepare_path, Some(&prior_prepare));
        restore_or_remove(&json_path, prior_json.as_deref());
        restore_or_remove(&md_path, prior_md.as_deref());
        cleanup_import_temps(prepared_dir);
        return Err(ModelError::Other(format!(
            "{err} import-trained removed the partial proposal so apply cannot see an unfinished import."
        )));
    }
    Ok(())
}

fn accept_trained_record(
    kind: &str,
    shape: Option<&str>,
    paths: Option<&[String]>,
) -> Result<(), ModelError> {
    match (shape, paths) {
        (None, None) => Ok(()),
        (Some(shape), Some(paths)) => {
            if !matches!(
                shape,
                TRAINED_SHAPE_ADAPTER | TRAINED_SHAPE_MERGED | TRAINED_SHAPE_GGUF
            ) {
                return Err(ModelError::Other(format!(
                    "refuse:{kind}: trained_shape '{shape}' is not adapter, merged, or gguf"
                )));
            }
            if paths.is_empty() {
                return Err(ModelError::Other(format!(
                    "refuse:{kind}: trained_paths is empty"
                )));
            }
            for path in paths {
                refuse_sacred_and_sku("trained path", path)?;
            }
            Ok(())
        }
        (Some(_), None) => Err(ModelError::Other(format!(
            "refuse:{kind}: trained_shape is set and trained_paths is missing"
        ))),
        (None, Some(_)) => Err(ModelError::Other(format!(
            "refuse:{kind}: trained_paths is set and trained_shape is missing"
        ))),
    }
}

fn trained_records_agree(
    doc: &EnrichPrepareDoc,
    proposal: &EnrichBindingProposal,
) -> Result<(), ModelError> {
    let same = doc.trained_shape == proposal.trained_shape
        && doc.trained_paths == proposal.trained_paths
        && accept_trained_record(
            "prepare",
            doc.trained_shape.as_deref(),
            doc.trained_paths.as_deref(),
        )
        .is_ok();
    if same {
        return Ok(());
    }
    Err(ModelError::Other(
        "refuse:prepare: trained_shape and trained_paths on prepare.json do not match the binding proposal"
            .into(),
    ))
}

pub(crate) fn load_prepare_doc(path: &Path) -> Result<EnrichPrepareDoc, ModelError> {
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
    parse_prepare_doc(path, &text)
}

fn parse_prepare_doc(path: &Path, text: &str) -> Result<EnrichPrepareDoc, ModelError> {
    refuse_raw_secrets(text).map_err(map_feed)?;
    let doc: EnrichPrepareDoc = serde_json::from_str(text).map_err(|err| {
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
    accept_trained_record(
        "prepare",
        doc.trained_shape.as_deref(),
        doc.trained_paths.as_deref(),
    )?;
    Ok(doc)
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

fn trained_proposal_lines(proposal: &EnrichBindingProposal) -> String {
    match (&proposal.trained_shape, &proposal.trained_paths) {
        (Some(shape), Some(paths)) => {
            let mut lines = format!("trained shape: {shape}\ntrained paths:\n");
            for path in paths {
                lines.push_str(&format!("- {path}\n"));
            }
            lines.push('\n');
            lines
        }
        _ => String::new(),
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
         {trained}\
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
        trained = trained_proposal_lines(proposal),
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
/// A symlink at the enrich root refuses. Missing does not invent a count.
pub fn enrich_join_facts(enrich_root: &Path) -> Result<Option<Vec<EnrichJoinFact>>, ModelError> {
    match enrich_root_kind(enrich_root)? {
        EnrichRootKind::Missing => return Ok(None),
        EnrichRootKind::Directory => {}
    }
    let rows = list_prepared(enrich_root)?;
    let mut facts = Vec::new();
    for row in rows {
        let proposal_path = row.out_dir.join(BINDING_PROPOSAL_JSON);
        if enrich_regular_file(&proposal_path)? {
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
    trained_records_agree(&doc, &proposal)?;
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
    accept_trained_record(
        "proposal",
        proposal.trained_shape.as_deref(),
        proposal.trained_paths.as_deref(),
    )?;
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

    fn yaml_line(text: &str, expected: &str) -> bool {
        text.lines().any(|line| line.trim() == expected)
    }

    fn tmp(name: &str) -> PathBuf {
        // Keep the throwaway dir off the checkout. A GPU token in the
        // checkout path would trip refuse:sku-banned on the out directory.
        // A pid that contains 5090, 4090, 4080, or 3090 is the same refuse.
        let mut token = std::process::id().to_string();
        for needle in ["5090", "4090", "4080", "3090"] {
            token = token.replace(needle, "0000");
        }
        let path = std::env::temp_dir().join(format!("cell-one-enrich-unit-{name}-{token}"));
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
            official_scale: false,
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
            official_scale: false,
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
        assert!(ids.contains(&LLAMAFACTORY_LORA_ID));
        assert!(ids.contains(&AXOLOTL_LORA_ID));
        assert!(ids.contains(&AXOLOTL_QLORA_ID));
        assert!(ids.contains(&UNSLOTH_QLORA_ID));
        assert!(ids.contains(&MLX_LM_LORA_ID));
        let factory = train_enrich_card(LLAMAFACTORY_QLORA_ID).unwrap();
        assert_eq!(factory.default_job, EnrichJobKind::Train);
        assert!(factory.jobs.contains(&EnrichJobKind::Train));
        assert!(!factory.jobs.contains(&EnrichJobKind::Enrich));
        let lora = train_enrich_card(LLAMAFACTORY_LORA_ID).unwrap();
        assert_eq!(lora.default_job, EnrichJobKind::Train);
        assert!(lora.jobs.contains(&EnrichJobKind::Train));
        assert!(!lora.jobs.contains(&EnrichJobKind::Enrich));
        let axolotl = train_enrich_card(AXOLOTL_LORA_ID).unwrap();
        assert_eq!(axolotl.default_job, EnrichJobKind::Train);
        let axolotl_qlora = train_enrich_card(AXOLOTL_QLORA_ID).unwrap();
        assert_eq!(axolotl_qlora.default_job, EnrichJobKind::Train);
        assert!(axolotl_qlora.jobs.contains(&EnrichJobKind::Train));
        assert!(!axolotl_qlora.jobs.contains(&EnrichJobKind::Enrich));
        let unsloth = train_enrich_card(UNSLOTH_QLORA_ID).unwrap();
        assert_eq!(unsloth.status, "optional");
        assert_eq!(unsloth.default_job, EnrichJobKind::Train);
        assert!(unsloth.jobs.contains(&EnrichJobKind::Train));
        assert!(!unsloth.jobs.contains(&EnrichJobKind::Enrich));
        assert!(!is_train_recipe_driver(UNSLOTH_QLORA_ID));
        let mlx = train_enrich_card(MLX_LM_LORA_ID).unwrap();
        assert_eq!(mlx.status, "optional");
        assert_eq!(mlx.default_job, EnrichJobKind::Train);
        assert!(mlx.jobs.contains(&EnrichJobKind::Train));
        assert!(!mlx.jobs.contains(&EnrichJobKind::Enrich));
        assert!(!is_train_recipe_driver(MLX_LM_LORA_ID));
        let enrich_ids = train_enrich_drivers_for_job("enrich").unwrap();
        assert!(!enrich_ids.contains(&LLAMAFACTORY_QLORA_ID));
        assert!(!enrich_ids.contains(&LLAMAFACTORY_LORA_ID));
        assert!(!enrich_ids.contains(&AXOLOTL_LORA_ID));
        assert!(!enrich_ids.contains(&AXOLOTL_QLORA_ID));
        assert!(!enrich_ids.contains(&UNSLOTH_QLORA_ID));
        assert!(!enrich_ids.contains(&MLX_LM_LORA_ID));
        assert_eq!(enrich_ids.len(), 2);
        let train_ids = train_enrich_drivers_for_job("train").unwrap();
        assert!(train_ids.contains(&LLAMAFACTORY_QLORA_ID));
        assert!(train_ids.contains(&LLAMAFACTORY_LORA_ID));
        assert!(train_ids.contains(&AXOLOTL_LORA_ID));
        assert!(train_ids.contains(&AXOLOTL_QLORA_ID));
        assert!(train_ids.contains(&UNSLOTH_QLORA_ID));
        assert!(train_ids.contains(&MLX_LM_LORA_ID));
        assert_eq!(train_ids.len(), 8);
        for host in ["any", "consumer-nvidia", "rented-nvidia"] {
            let filtered = train_enrich_drivers_for_prepare("train", host).unwrap();
            assert!(!filtered.contains(&MLX_LM_LORA_ID), "{host}");
            assert_eq!(filtered.len(), 7, "{host}");
        }
        let apple_ids = train_enrich_drivers_for_prepare("train", "apple-silicon").unwrap();
        assert!(apple_ids.contains(&MLX_LM_LORA_ID));
        assert_eq!(apple_ids.len(), 8);
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
        assert!(rendered.contains(LLAMAFACTORY_LORA_ID), "{rendered}");
        assert!(rendered.contains(UNSLOTH_QLORA_ID), "{rendered}");
        assert!(rendered.contains(MLX_LM_LORA_ID), "{rendered}");
        assert!(rendered.contains("status=optional"), "{rendered}");
        assert!(rendered.contains("Nvidia-only"), "{rendered}");
        assert!(rendered.contains("apple-silicon"), "{rendered}");
        assert!(rendered.contains("refuse:host"), "{rendered}");
        assert!(rendered.contains(AXOLOTL_LORA_ID), "{rendered}");
        assert!(rendered.contains(AXOLOTL_QLORA_ID), "{rendered}");
        assert!(rendered.contains("live=false"), "{rendered}");
        assert!(rendered.contains("default=train"), "{rendered}");
    }

    #[test]
    fn unsloth_qlora_writes_a_handoff_and_refuses_a_missing_train_base() {
        let root = tmp("unsloth");
        let pack = fixture_pack();
        let seated = seated_estate("llama3");
        let blocked = root.join("seat-only");
        let err = run(UNSLOTH_QLORA_ID, &pack, &seated, &blocked, "train", "jason").unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("llama3"), "{err}");
        assert!(!blocked.exists());

        let seat_as_base = with_train_base(seated.clone(), "llama3");
        let seat_out = root.join("seat-tag");
        let err = run(
            UNSLOTH_QLORA_ID,
            &pack,
            &seat_as_base,
            &seat_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(!seat_out.exists());

        let estate = with_train_base(seated, "Qwen/Qwen2.5-0.5B-Instruct");
        let enrich_out = root.join("enrich-job");
        let err = run(
            UNSLOTH_QLORA_ID,
            &pack,
            &estate,
            &enrich_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");
        assert!(!enrich_out.exists());

        let official = root.join("official");
        let err = prepare_enrich(&PrepareEnrichRequest {
            estate: &estate,
            pack: &pack,
            curator: "jason",
            driver_id: UNSLOTH_QLORA_ID,
            job: "train",
            out_dir: &official,
            max_steps: None,
            official_scale: true,
            from_feed: false,
            state_dir: Path::new(".cell"),
        })
        .unwrap_err();
        assert!(err.to_string().contains("refuse:official-scale"), "{err}");
        assert!(!err.to_string().contains(UNSLOTH_QLORA_ID), "{err}");
        assert!(!official.exists());

        let feed_out = root.join("feed");
        let err = run_feed(UNSLOTH_QLORA_ID, &pack, &estate, &feed_out, &root, true).unwrap_err();
        assert!(err.to_string().contains("refuse:dataset"), "{err}");
        assert!(!feed_out.exists());

        let zero = root.join("zero-steps");
        let err = run_max(
            UNSLOTH_QLORA_ID,
            &pack,
            &estate,
            &zero,
            "train",
            "jason",
            Some(0),
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:max-steps"), "{err}");
        assert!(!zero.exists());

        let out = root.join("handoff");
        let doc = run(UNSLOTH_QLORA_ID, &pack, &estate, &out, "train", "jason").unwrap();
        assert_eq!(doc.driver, UNSLOTH_QLORA_ID);
        assert_eq!(doc.job, "train");
        assert_eq!(doc.base_model, "llama3");
        assert_eq!(doc.seat_tag.as_deref(), Some("llama3"));
        assert_eq!(
            doc.train_base_model.as_deref(),
            Some("Qwen/Qwen2.5-0.5B-Instruct")
        );
        assert!(doc.dataset_mode.is_none());
        assert!(doc.dataset_rows.is_none());
        assert!(doc.export_yaml.is_none());
        assert!(doc.modelfile.is_none());
        assert!(doc.trained_shape.is_none());
        assert!(!doc.promoted && !doc.auto_apply && !doc.estate_rewritten);
        let mut names: Vec<_> = std::fs::read_dir(&out)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "NEXT.md".to_string(),
                "PREPARE.md".to_string(),
                "UNSLOTH.md".to_string(),
                "prepare.json".to_string(),
            ]
        );
        for name in &names {
            let body = std::fs::read_to_string(out.join(name)).unwrap();
            assert!(!body.starts_with("#!"), "{name}");
            assert!(!body.contains("```"), "{name}");
            assert!(!body.contains("train_unsloth.py"), "{name}");
            assert!(!body.contains("std::process"), "{name}");
        }
        let handoff = std::fs::read_to_string(out.join("UNSLOTH.md")).unwrap();
        assert!(handoff.contains("operator-owned"), "{handoff}");
        assert!(handoff.contains("does not call Unsloth"), "{handoff}");
        assert!(handoff.contains("Nvidia-only"), "{handoff}");
        assert!(handoff.contains("not a training script"), "{handoff}");
        assert!(
            handoff.contains("train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\""),
            "{handoff}"
        );
        assert!(handoff.contains("seat_tag: \"llama3\""), "{handoff}");
        assert!(handoff.contains(UNSLOTH_INSTALL_DOC), "{handoff}");
        assert!(handoff.contains(UNSLOTH_GUIDE_DOC), "{handoff}");
        assert!(handoff.contains(UNSLOTH_REPO), "{handoff}");
        assert!(handoff.contains(UNSLOTH_README_INSTALL), "{handoff}");
        assert!(handoff.contains("READY_FOR_LIVE_TEST: no"), "{handoff}");
        assert!(!handoff.contains("READY_FOR_LIVE_TEST: yes"), "{handoff}");
        assert!(!handoff.contains("executable: true"), "{handoff}");
        let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
        assert!(next.contains("does not call Unsloth"), "{next}");
        assert!(next.contains("does not shell out"), "{next}");
        assert!(next.contains("import-trained"), "{next}");
        assert!(
            next.contains(&format!(
                "estate enrich import-trained --estate <estate.yaml> --prepared {} --tag cell-enrich-overnight-traces --adapter <adapter-dir>",
                out.display()
            )),
            "{next}"
        );
        assert!(next.contains("--adapter <merged-dir>"), "{next}");
        assert!(next.contains("--adapter <gguf>"), "{next}");
        assert!(next.contains("READY_FOR_LIVE_TEST: no"), "{next}");
        assert!(!next.contains("READY_FOR_LIVE_TEST: yes"), "{next}");
        assert!(next.contains("does not write an MLX trainer"), "{next}");
        let prepare_md = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
        assert!(prepare_md.contains("Seat tag: llama3"), "{prepare_md}");
        assert!(
            prepare_md.contains("Train base: Qwen/Qwen2.5-0.5B-Instruct"),
            "{prepare_md}"
        );
        assert!(prepare_md.contains("did not call Unsloth"), "{prepare_md}");
        assert!(!prepare_md.contains("Dataset mode:"), "{prepare_md}");

        let gauge = root.join("gauge");
        run_max(
            UNSLOTH_QLORA_ID,
            &pack,
            &estate,
            &gauge,
            "train",
            "jason",
            Some(10),
        )
        .unwrap();
        let gauge_next = std::fs::read_to_string(gauge.join("NEXT.md")).unwrap();
        assert!(
            gauge_next.contains("does not write that count"),
            "{gauge_next}"
        );
        assert!(gauge_next.contains("--max-steps 10"), "{gauge_next}");
        assert!(!gauge.join("train_unsloth.py").exists());

        let relative_raw = "./weights/Qwen2.5-0.5B-Instruct";
        let expected = canonical_train_base(relative_raw, "llama3").unwrap();
        let relative = with_train_base(estate.clone(), relative_raw);
        let relative_out = root.join("relative");
        let relative_doc = run(
            UNSLOTH_QLORA_ID,
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
        let relative_handoff = std::fs::read_to_string(relative_out.join("UNSLOTH.md")).unwrap();
        assert!(
            relative_handoff.contains(&format!("train_base_model: \"{expected}\"")),
            "{relative_handoff}"
        );
        assert!(
            !relative_handoff.contains("./weights"),
            "{relative_handoff}"
        );

        let tampered_dir = root.join("tampered");
        run(
            UNSLOTH_QLORA_ID,
            &pack,
            &estate,
            &tampered_dir,
            "train",
            "jason",
        )
        .unwrap();
        let tampered_path = tampered_dir.join("UNSLOTH.md");
        let tampered = std::fs::read_to_string(&tampered_path).unwrap().replace(
            "train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"",
            "train_base_model: \"meta-llama/Llama-3.2-1B-Instruct\"",
        );
        std::fs::write(&tampered_path, tampered).unwrap();
        let adapter = root.join("adapter");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        let tampered_import = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &tampered_dir,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            tampered_import.to_string().contains("refuse:train-base"),
            "{tampered_import}"
        );
        assert!(!tampered_dir.join("binding-proposal.json").exists());

        let proposal = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap();
        assert_eq!(proposal.driver, UNSLOTH_QLORA_ID);
        assert_eq!(proposal.trained_shape.as_deref(), Some("adapter"));
        assert!(!proposal.promoted && !proposal.auto_apply && !proposal.estate_rewritten);
        let recorded = std::fs::read_to_string(out.join("prepare.json")).unwrap();
        assert!(
            recorded.contains("\"trained_shape\": \"adapter\""),
            "{recorded}"
        );
        assert!(
            recorded.contains("\"estate_rewritten\": false"),
            "{recorded}"
        );
    }

    #[test]
    fn mlx_lm_lora_writes_a_handoff_on_apple_silicon_and_refuses_other_hosts() {
        let root = tmp("mlx");
        let pack = fixture_pack();
        let seated = seated_estate("llama3");
        let estate = with_train_base(seated, "Qwen/Qwen2.5-0.5B-Instruct");
        let mut apple = pack.clone();
        apple.host_class_affinity = Some(APPLE_SILICON_HOST.into());

        for host in ["any", "consumer-nvidia", "rented-nvidia", "apple_silicon"] {
            let mut wrong = pack.clone();
            wrong.host_class_affinity = Some(host.into());
            let out = root.join(format!("host-{}", host.replace('/', "_")));
            let err = run(MLX_LM_LORA_ID, &wrong, &estate, &out, "train", "jason").unwrap_err();
            assert!(err.to_string().contains("refuse:host"), "{host}: {err}");
            assert!(err.to_string().contains(host), "{host}: {err}");
            assert!(err.to_string().contains("apple-silicon"), "{host}: {err}");
            assert!(!out.exists(), "{host}");
        }

        let blocked = root.join("seat-only");
        let err = run(
            MLX_LM_LORA_ID,
            &apple,
            &seated_estate("llama3"),
            &blocked,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("llama3"), "{err}");
        assert!(!blocked.exists());

        let seat_as_base = with_train_base(seated_estate("llama3"), "llama3");
        let seat_out = root.join("seat-tag");
        let err = run(
            MLX_LM_LORA_ID,
            &apple,
            &seat_as_base,
            &seat_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(!seat_out.exists());

        let enrich_out = root.join("enrich-job");
        let err = run(
            MLX_LM_LORA_ID,
            &apple,
            &estate,
            &enrich_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");
        assert!(!enrich_out.exists());

        let official = root.join("official");
        let err = prepare_enrich(&PrepareEnrichRequest {
            estate: &estate,
            pack: &apple,
            curator: "jason",
            driver_id: MLX_LM_LORA_ID,
            job: "train",
            out_dir: &official,
            max_steps: None,
            official_scale: true,
            from_feed: false,
            state_dir: Path::new(".cell"),
        })
        .unwrap_err();
        assert!(err.to_string().contains("refuse:official-scale"), "{err}");
        assert!(!err.to_string().contains(MLX_LM_LORA_ID), "{err}");
        assert!(!official.exists());

        let feed_out = root.join("feed");
        let err = prepare_enrich(&PrepareEnrichRequest {
            estate: &estate,
            pack: &apple,
            curator: "jason",
            driver_id: MLX_LM_LORA_ID,
            job: "train",
            out_dir: &feed_out,
            max_steps: None,
            official_scale: false,
            from_feed: true,
            state_dir: &root,
        })
        .unwrap_err();
        assert!(err.to_string().contains("refuse:dataset"), "{err}");
        assert!(!err.to_string().contains(MLX_LM_LORA_ID), "{err}");
        assert!(!feed_out.exists());

        let zero = root.join("zero-steps");
        let err = run_max(
            MLX_LM_LORA_ID,
            &apple,
            &estate,
            &zero,
            "train",
            "jason",
            Some(0),
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:max-steps"), "{err}");
        assert!(!zero.exists());

        let out = root.join("handoff");
        let doc = run(MLX_LM_LORA_ID, &apple, &estate, &out, "train", "jason").unwrap();
        assert_eq!(doc.driver, MLX_LM_LORA_ID);
        assert_eq!(doc.job, "train");
        assert_eq!(doc.base_model, "llama3");
        assert_eq!(doc.seat_tag.as_deref(), Some("llama3"));
        assert_eq!(
            doc.train_base_model.as_deref(),
            Some("Qwen/Qwen2.5-0.5B-Instruct")
        );
        assert_eq!(doc.host_class_affinity, APPLE_SILICON_HOST);
        assert!(doc.dataset_mode.is_none());
        assert!(doc.dataset_rows.is_none());
        assert!(doc.export_yaml.is_none());
        assert!(doc.modelfile.is_none());
        assert!(doc.trained_shape.is_none());
        assert!(!doc.promoted && !doc.auto_apply && !doc.estate_rewritten);
        let mut names: Vec<_> = std::fs::read_dir(&out)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "MLX.md".to_string(),
                "NEXT.md".to_string(),
                "PREPARE.md".to_string(),
                "prepare.json".to_string(),
            ]
        );
        for name in &names {
            let body = std::fs::read_to_string(out.join(name)).unwrap();
            assert!(!body.starts_with("#!"), "{name}");
            assert!(!body.contains("```"), "{name}");
            assert!(!body.contains("train_mlx.py"), "{name}");
            assert!(!body.contains("std::process"), "{name}");
            assert!(!body.contains("mlx_lm.lora --config"), "{name}");
        }
        assert!(!out.join("dataset.jsonl").exists());
        assert!(!out.join("recipe.yaml").exists());
        assert!(!out.join("axolotl.yml").exists());
        let handoff = std::fs::read_to_string(out.join("MLX.md")).unwrap();
        assert!(handoff.contains("operator-owned"), "{handoff}");
        assert!(handoff.contains("does not call mlx-lm"), "{handoff}");
        assert!(handoff.contains("apple-silicon"), "{handoff}");
        assert!(handoff.contains("apple_silicon_only: true"), "{handoff}");
        assert!(
            handoff.contains("host_class_affinity: apple-silicon"),
            "{handoff}"
        );
        assert!(handoff.contains("not a training script"), "{handoff}");
        assert!(
            handoff.contains("train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\""),
            "{handoff}"
        );
        assert!(handoff.contains("seat_tag: \"llama3\""), "{handoff}");
        assert!(handoff.contains(MLX_LORA_DOC), "{handoff}");
        assert!(handoff.contains(MLX_REPO), "{handoff}");
        assert!(handoff.contains(MLX_TRAIN_INSTALL), "{handoff}");
        assert!(handoff.contains(MLX_LORA_COMMAND), "{handoff}");
        assert!(handoff.contains(MLX_FUSE_COMMAND), "{handoff}");
        assert!(handoff.contains("fused_model/"), "{handoff}");
        assert!(handoff.contains("READY_FOR_LIVE_TEST: no"), "{handoff}");
        assert!(!handoff.contains("READY_FOR_LIVE_TEST: yes"), "{handoff}");
        assert!(!handoff.contains("executable: true"), "{handoff}");
        let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
        assert!(next.contains("does not call mlx-lm"), "{next}");
        assert!(next.contains("does not shell out"), "{next}");
        assert!(next.contains("import-trained"), "{next}");
        assert!(next.contains(MLX_FUSE_COMMAND), "{next}");
        assert!(next.contains("mlx_lm.fuse --help"), "{next}");
        assert!(
            next.contains(&format!(
                "estate enrich import-trained --estate <estate.yaml> --prepared {} --tag cell-enrich-overnight-traces --adapter <adapter-dir>",
                out.display()
            )),
            "{next}"
        );
        assert!(next.contains("READY_FOR_LIVE_TEST: no"), "{next}");
        assert!(!next.contains("READY_FOR_LIVE_TEST: yes"), "{next}");
        assert!(next.contains("refuse:host"), "{next}");
        let prepare_md = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
        assert!(prepare_md.contains("Seat tag: llama3"), "{prepare_md}");
        assert!(
            prepare_md.contains("Train base: Qwen/Qwen2.5-0.5B-Instruct"),
            "{prepare_md}"
        );
        assert!(
            prepare_md.contains("Host class affinity: apple-silicon"),
            "{prepare_md}"
        );
        assert!(prepare_md.contains("did not call mlx-lm"), "{prepare_md}");
        assert!(!prepare_md.contains("Dataset mode:"), "{prepare_md}");
        for name in ["MLX.md", "NEXT.md", "PREPARE.md"] {
            let body = std::fs::read_to_string(out.join(name)).unwrap();
            assert!(body.contains("estate enrich merge-adapt"), "{name}: {body}");
            assert!(body.contains("mlx_lm.fuse --export-gguf"), "{name}: {body}");
            assert!(body.contains("adapters.safetensors"), "{name}: {body}");
            assert!(body.contains("ggml-model-f16.gguf"), "{name}: {body}");
            assert!(body.contains("estate enrich local-seat"), "{name}: {body}");
            assert!(body.contains("## After the mlx-lm train"), "{name}: {body}");
            assert!(
                !body.contains("python3 convert_hf_to_gguf.py"),
                "{name}: {body}"
            );
            assert!(
                !body.contains("estate enrich gguf-convert"),
                "{name}: {body}"
            );
            assert!(!body.contains("--dequantize"), "{name}: {body}");
            assert!(body.contains("READY_FOR_LIVE_TEST: no"), "{name}: {body}");
            assert!(!body.contains("READY_FOR_LIVE_TEST: yes"), "{name}: {body}");
        }

        let gauge = root.join("gauge");
        run_max(
            MLX_LM_LORA_ID,
            &apple,
            &estate,
            &gauge,
            "train",
            "jason",
            Some(10),
        )
        .unwrap();
        let gauge_next = std::fs::read_to_string(gauge.join("NEXT.md")).unwrap();
        assert!(
            gauge_next.contains("does not write that count"),
            "{gauge_next}"
        );
        assert!(gauge_next.contains("--max-steps 10"), "{gauge_next}");
        assert!(!gauge.join("train_mlx.py").exists());

        let lf = root.join("lf-apple");
        run(
            LLAMAFACTORY_QLORA_ID,
            &apple,
            &estate,
            &lf,
            "train",
            "jason",
        )
        .unwrap();
        let recipe = std::fs::read_to_string(lf.join("recipe.yaml")).unwrap();
        assert!(recipe.contains("quantization_method: bnb"), "{recipe}");
        assert!(recipe.contains("quantization_bit: 4"), "{recipe}");
        assert!(recipe.contains("cutoff_len: 512"), "{recipe}");
        assert!(recipe.contains("lora_rank: 16"), "{recipe}");
        assert!(!recipe.contains("mlx-lm-lora"), "{recipe}");
        assert!(!recipe.contains("mlx_lm"), "{recipe}");
        let lf_next = std::fs::read_to_string(lf.join("NEXT.md")).unwrap();
        assert!(lf_next.contains("--driver mlx-lm-lora"), "{lf_next}");
        assert!(lf_next.contains(MLX_LORA_DOC), "{lf_next}");
        assert!(
            lf_next.contains("does not write an MLX trainer"),
            "{lf_next}"
        );

        let relative_raw = "./weights/Qwen2.5-0.5B-Instruct";
        let expected = canonical_train_base(relative_raw, "llama3").unwrap();
        let relative = with_train_base(estate.clone(), relative_raw);
        let relative_out = root.join("relative");
        let relative_doc = run(
            MLX_LM_LORA_ID,
            &apple,
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
        let relative_handoff = std::fs::read_to_string(relative_out.join("MLX.md")).unwrap();
        assert!(
            relative_handoff.contains(&format!("train_base_model: \"{expected}\"")),
            "{relative_handoff}"
        );

        let tampered_dir = root.join("tampered");
        run(
            MLX_LM_LORA_ID,
            &apple,
            &estate,
            &tampered_dir,
            "train",
            "jason",
        )
        .unwrap();
        let tampered_path = tampered_dir.join("MLX.md");
        let tampered = std::fs::read_to_string(&tampered_path).unwrap().replace(
            "train_base_model: \"Qwen/Qwen2.5-0.5B-Instruct\"",
            "train_base_model: \"meta-llama/Llama-3.2-1B-Instruct\"",
        );
        std::fs::write(&tampered_path, tampered).unwrap();
        let adapter = root.join("adapter");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        let tampered_import = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &tampered_dir,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            tampered_import.to_string().contains("refuse:train-base"),
            "{tampered_import}"
        );
        assert!(!tampered_dir.join("binding-proposal.json").exists());

        let host_dir = root.join("tampered-host");
        run(MLX_LM_LORA_ID, &apple, &estate, &host_dir, "train", "jason").unwrap();
        let host_path = host_dir.join("MLX.md");
        let host_body = std::fs::read_to_string(&host_path).unwrap().replace(
            "host_class_affinity: apple-silicon",
            "host_class_affinity: consumer-nvidia",
        );
        std::fs::write(&host_path, host_body).unwrap();
        let host_import = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &host_dir,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            host_import.to_string().contains("refuse:host"),
            "{host_import}"
        );
        assert!(!host_dir.join("binding-proposal.json").exists());

        let proposal = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap();
        assert_eq!(proposal.driver, MLX_LM_LORA_ID);
        assert_eq!(proposal.trained_shape.as_deref(), Some("adapter"));
        assert!(!proposal.promoted && !proposal.auto_apply && !proposal.estate_rewritten);

        let drivers = train_enrich_drivers_for_prepare("train", APPLE_SILICON_HOST).unwrap();
        assert!(drivers.contains(&MLX_LM_LORA_ID));
        let all_dirs: Vec<_> = drivers
            .iter()
            .map(|id| root.join(format!("all-{id}")))
            .collect();
        let all_reqs: Vec<PrepareEnrichRequest<'_>> = drivers
            .iter()
            .zip(all_dirs.iter())
            .map(|(id, dir)| PrepareEnrichRequest {
                estate: &estate,
                pack: &apple,
                curator: "jason",
                driver_id: *id,
                job: "train",
                out_dir: dir,
                max_steps: None,
                official_scale: false,
                from_feed: false,
                state_dir: Path::new(".cell"),
            })
            .collect();
        let all_docs = prepare_enrich_set(&all_reqs).unwrap();
        assert_eq!(all_docs.len(), 8);
        assert!(root.join("all-mlx-lm-lora").join("MLX.md").is_file());
        assert!(!root.join("all-mlx-lm-lora").join("dataset.jsonl").exists());
        let all_recipe =
            std::fs::read_to_string(root.join("all-llamafactory-qlora").join("recipe.yaml"))
                .unwrap();
        assert!(
            all_recipe.contains("quantization_method: bnb"),
            "{all_recipe}"
        );
        assert!(!all_recipe.contains("mlx-lm-lora"), "{all_recipe}");
        assert!(!all_recipe.contains("mlx_lm"), "{all_recipe}");
        let all_unsloth =
            std::fs::read_to_string(root.join("all-unsloth-qlora").join("UNSLOTH.md")).unwrap();
        assert!(all_unsloth.contains("Nvidia-only"), "{all_unsloth}");
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
        assert!(doc.export_yaml.as_deref().unwrap().ends_with("export.yaml"));
        assert!(doc.modelfile.is_none());
        assert!(doc.trained_shape.is_none());
        assert!(doc.trained_paths.is_none());
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
        assert!(next.contains(&format!(
            "estate enrich import-trained --estate <estate.yaml> --prepared {} --tag cell-enrich-overnight-traces --adapter {}",
            out.display(),
            out.join("outputs").display()
        )), "{next}");
        assert!(next.contains(&format!(
            "estate enrich import-trained --estate <estate.yaml> --prepared {} --tag cell-enrich-overnight-traces --adapter {}",
            out.display(),
            out.join("export").display()
        )), "{next}");
        assert!(next.contains("--adapter <gguf>"), "{next}");
        assert!(
            next.contains("records trained_shape and trained_paths"),
            "{next}"
        );
        assert!(next.contains("## Local seat after export"), "{next}");
        assert!(
            next.contains(&format!(
                "estate enrich local-seat --prepared {} --adapter {}",
                out.display(),
                out.join("outputs").display()
            )),
            "{next}"
        );
        assert!(next.contains("does not shell out to ollama"), "{next}");
        assert!(next.contains("READY_FOR_LIVE_TEST: no"), "{next}");
        assert!(!next.contains("READY_FOR_LIVE_TEST: yes"), "{next}");
        assert!(next.contains("CUDA LLaMA-Factory"), "{next}");
        assert!(next.contains("does not write an MLX trainer"), "{next}");
        assert!(next.contains("same chat template"), "{next}");
        assert!(next.contains("smoke eval"), "{next}");
        assert!(next.contains("Faster single-GPU alternate"), "{next}");
        assert!(next.contains("unsloth.ai/docs"), "{next}");
        assert!(next.contains("axolotl-lora"), "{next}");
        assert!(next.contains("--driver unsloth-qlora"), "{next}");
        assert!(next.contains("does not write a script"), "{next}");
        assert!(!next.contains("train_unsloth.py"), "{next}");
        assert!(next.contains("--driver mlx-lm-lora"), "{next}");
        assert!(next.contains("mlx_lm/LORA.md"), "{next}");
        assert!(next.contains("does not write an MLX trainer"), "{next}");
        assert!(!recipe.contains("mlx-lm-lora"), "{recipe}");
        assert!(!recipe.contains("mlx_lm"), "{recipe}");
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
        assert!(
            prepare_md.contains("Dataset mode: scaffold"),
            "{prepare_md}"
        );
        assert!(
            prepare_md.contains("dataset_mode: scaffold"),
            "{prepare_md}"
        );
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
    fn llamafactory_template_follows_qwen3_thinking() {
        let cases = [
            ("Qwen/Qwen2.5-0.5B-Instruct", "qwen"),
            ("Qwen/Qwen3-4B", "qwen3"),
            ("Qwen/Qwen3-4B-Thinking-2507", "qwen3"),
            ("Qwen/Qwen3-30B-A3B-Thinking-2507", "qwen3"),
            ("Qwen/Qwen3-4B-Instruct-Thinking", "qwen3"),
            ("Qwen/Qwen3-4B-Instruct-2507", "qwen3_nothink"),
            ("Qwen/Qwen3-30B-A3B-Instruct-2507", "qwen3_nothink"),
            ("/tmp/hf/Qwen3-4B-Instruct-2507", "qwen3_nothink"),
            (
                "/tmp/hf/Qwen3-4B-Instruct-2507/weights",
                "qwen3_nothink",
            ),
            (
                "/home/user/.cache/huggingface/hub/models--Qwen--Qwen3-4B-Instruct-2507/snapshots/a1b2c3d4e5f6",
                "qwen3_nothink",
            ),
            ("/tmp/Qwen3-4B-Thinking-2507/weights", "qwen3"),
            ("/tmp/models/Qwen3-4B/snapshots/deadbeef", "qwen3"),
            ("/tmp/qwen3-parent/Qwen2.5-0.5B-Instruct", "qwen"),
            ("./weights/Qwen2.5-0.5B-Instruct", "qwen"),
            ("meta-llama/Meta-Llama-3-8B-Instruct", "llama3"),
            ("mistralai/Mistral-7B-Instruct-v0.3", "mistral"),
            ("google/gemma-2-2b-it", "gemma2"),
            ("microsoft/phi-3-mini-4k-instruct", "phi"),
            ("org/some-model", "default"),
        ];
        for (train, expect) in cases {
            assert_eq!(llamafactory_template(train), expect, "{train}");
        }
    }

    #[test]
    fn llamafactory_template_maps_phi3_and_phi35_including_nested_paths() {
        let phi = [
            "microsoft/Phi-3-mini-4k-instruct",
            "microsoft/Phi-3-mini-128k-instruct",
            "microsoft/Phi-3-medium-4k-instruct",
            "microsoft/Phi-3-medium-128k-instruct",
            "Microsoft/PHI-3-MINI-4K-INSTRUCT",
            "microsoft/Phi-3.5-mini-instruct",
            "microsoft/Phi-3.5-MoE-instruct",
            "Phi-3.5-mini-instruct",
            "/opt/hf/microsoft/Phi-3-mini-4k-instruct",
            "./weights/Phi-3.5-mini-instruct",
            "/opt/hf/microsoft/Phi-3-mini-4k-instruct/snapshots/deadbeef",
            "/home/user/.cache/huggingface/hub/models--microsoft--Phi-3-mini-4k-instruct/snapshots/abc123",
            "/home/user/.cache/huggingface/hub/models--microsoft--Phi-3.5-mini-instruct/snapshots/def456",
            "/tmp/phi-4/Phi-3-mini-4k-instruct",
            "/tmp/qwen3-parent/Phi-3.5-mini-instruct",
        ];
        for train in phi {
            assert_eq!(llamafactory_template(train), "phi", "{train}");
        }
        let small = [
            "microsoft/Phi-3-small-8k-instruct",
            "microsoft/Phi-3-small-128k-instruct",
            "/opt/hf/microsoft/Phi-3-small-8k-instruct",
            "/home/user/.cache/huggingface/hub/models--microsoft--Phi-3-small-8k-instruct/snapshots/abc",
        ];
        for train in small {
            assert_eq!(llamafactory_template(train), "phi_small", "{train}");
        }
        assert_eq!(llamafactory_template("microsoft/phi-4"), "phi4");
        assert_eq!(
            llamafactory_template("microsoft/Phi-4-mini-instruct"),
            "phi4_mini"
        );
        assert_eq!(
            llamafactory_template("/opt/hf/Phi-4-mini-instruct"),
            "phi4_mini"
        );
        assert_eq!(llamafactory_template("microsoft/phi-2"), "default");
        assert_eq!(llamafactory_template("org/phi"), "default");
        assert_eq!(llamafactory_template("Qwen/Qwen2.5-0.5B-Instruct"), "qwen");
    }

    #[test]
    fn llamafactory_template_maps_llama32_instruct_including_nested_paths() {
        let instruct = [
            "meta-llama/Llama-3.2-3B-Instruct",
            "meta-llama/Llama-3.2-1B-Instruct",
            "META-LLAMA/LLAMA-3.2-3B-INSTRUCT",
            "unsloth/Llama-3.2-3B-Instruct",
            "Llama-3.2-3B-Instruct",
            "Llama3.2-3B-Instruct",
            "/opt/hf/meta-llama/Llama-3.2-3B-Instruct",
            "./weights/Llama-3.2-1B-Instruct",
            "/opt/hf/meta-llama/Llama-3.2-3B-Instruct/weights",
            "/opt/hf/meta-llama/Llama-3.2-3B-Instruct/snapshots/deadbeef",
            "/home/user/.cache/huggingface/hub/models--meta-llama--Llama-3.2-3B-Instruct/snapshots/abc123",
            "/home/user/.cache/huggingface/hub/models--meta-llama--Llama-3.2-1B-Instruct/snapshots/def456",
            "/tmp/phi-4/Llama-3.2-3B-Instruct",
            "/tmp/qwen3-parent/Llama-3.2-1B-Instruct",
            "/tmp/llama-30b/Llama-3.2-3B-Instruct",
            "/tmp/llama3-llava-next-8b-hf/Llama-3.2-3B-Instruct",
        ];
        for train in instruct {
            assert_eq!(llamafactory_template(train), "llama3", "{train}");
            assert_ne!(llamafactory_template(train), "llama3_2", "{train}");
        }
        let vision = [
            "meta-llama/Llama-3.2-11B-Vision",
            "meta-llama/Llama-3.2-11B-Vision-Instruct",
            "meta-llama/Llama-3.2-90B-Vision",
            "meta-llama/Llama-3.2-90B-Vision-Instruct",
            "Llama3.2-11B-Vision-Instruct",
            "/opt/hf/meta-llama/Llama-3.2-11B-Vision-Instruct",
            "./weights/Llama-3.2-90B-Vision-Instruct",
            "/home/user/.cache/huggingface/hub/models--meta-llama--Llama-3.2-11B-Vision-Instruct/snapshots/abc",
            "/tmp/Llama-3.2-3B-Instruct/Llama-3.2-11B-Vision-Instruct",
            "/tmp/qwen3-parent/Llama-3.2-11B-Vision",
        ];
        for train in vision {
            assert_eq!(llamafactory_template(train), "mllama", "{train}");
        }
        assert_eq!(
            llamafactory_template("llava-hf/llama3-llava-next-8b-hf"),
            "llava_next_llama3"
        );
        assert_eq!(
            llamafactory_template("LLaVA-NeXT-Llama3-8B-Chat"),
            "llava_next_llama3"
        );
        assert_eq!(
            llamafactory_template("/opt/hf/llama3-llava-next-8b-hf/snapshots/abc"),
            "llava_next_llama3"
        );
        assert_eq!(llamafactory_template("huggyllama/llama-30b"), "default");
        assert_eq!(llamafactory_template("huggyllama/llama-7b"), "default");
        assert_eq!(llamafactory_template("/opt/hf/llama-30b"), "default");
        assert_eq!(
            llamafactory_template("meta-llama/Meta-Llama-3-8B-Instruct"),
            "llama3"
        );
        assert_eq!(
            llamafactory_template("meta-llama/Meta-Llama-3.1-8B-Instruct"),
            "llama3"
        );
        assert_eq!(
            llamafactory_template("meta-llama/Llama-3.3-70B-Instruct"),
            "llama3"
        );
        assert_eq!(llamafactory_template("meta-llama/Llama-3.2-3B"), "llama3");
        assert_eq!(
            llamafactory_template("shenzhi-wang/Llama3-8B-Chinese-Chat"),
            "llama3"
        );
        assert_eq!(
            llamafactory_template("/tmp/Llama-3.2-3B-Instruct/Phi-3-mini-4k-instruct"),
            "phi"
        );
        assert_eq!(
            llamafactory_template("/tmp/Llama-3.2-3B-Instruct/Qwen2.5-0.5B-Instruct"),
            "qwen"
        );
        assert_eq!(
            llamafactory_template("meta-llama/Llama-2-7b-chat-hf"),
            "default"
        );
    }

    #[test]
    fn llamafactory_template_maps_gemma2_instruct_including_nested_paths() {
        let instruct = [
            "google/gemma-2-2b-it",
            "google/gemma-2-9b-it",
            "google/gemma-2-27b-it",
            "GOOGLE/GEMMA-2-2B-IT",
            "unsloth/gemma-2-9b-it",
            "gemma-2-2b-it",
            "gemma2-2b-it",
            "Gemma-2-2B-Instruct",
            "/opt/hf/google/gemma-2-2b-it",
            "./weights/gemma-2-9b-it",
            "/opt/hf/google/gemma-2-2b-it/weights",
            "/opt/hf/google/gemma-2-9b-it/snapshots/deadbeef",
            "/home/user/.cache/huggingface/hub/models--google--gemma-2-2b-it/snapshots/abc123",
            "/home/user/.cache/huggingface/hub/models--google--gemma-2-9b-it/snapshots/def456",
            "/home/user/.cache/huggingface/hub/models--google--gemma-2-27b-it/snapshots/ghi789",
            "/tmp/phi-4/gemma-2-2b-it",
            "/tmp/qwen3-parent/gemma-2-9b-it",
            "/tmp/gemma-7b/gemma-2-2b-it",
            "/tmp/gemma-2b-it/gemma-2-9b-it",
            "/tmp/gemma-3-4b-it/gemma-2-2b-it",
            "/tmp/Llama-3.2-3B-Instruct/gemma-2-2b-it",
        ];
        for train in instruct {
            assert_eq!(llamafactory_template(train), "gemma2", "{train}");
            assert_ne!(llamafactory_template(train), "gemma", "{train}");
            assert_ne!(llamafactory_template(train), "gemma_2", "{train}");
        }
        let base = [
            "google/gemma-2-2b",
            "google/gemma-2-9b",
            "google/gemma-2-27b",
            "/opt/hf/google/gemma-2-2b",
            "/home/user/.cache/huggingface/hub/models--google--gemma-2-2b/snapshots/abc",
        ];
        for train in base {
            assert_eq!(llamafactory_template(train), "gemma2", "{train}");
        }
        let original = [
            "google/gemma-2b",
            "google/gemma-2b-it",
            "google/gemma-7b",
            "google/gemma-7b-it",
            "google/gemma-1.1-2b-it",
            "google/gemma-1.1-7b-it",
            "google/codegemma-7b-it",
            "google/codegemma-1.1-7b-it",
            "/opt/hf/google/gemma-2b-it",
            "/tmp/gemma-2-2b-it/gemma-7b",
            "/tmp/gemma-2-2b-it/gemma-2b-it",
        ];
        for train in original {
            assert_eq!(llamafactory_template(train), "gemma", "{train}");
        }
        let other = [
            "google/gemma-3-270m",
            "google/gemma-3-270m-it",
            "google/gemma-3-1b-it",
            "google/gemma-3-1b-pt",
            "google/gemma-3-4b-it",
            "google/gemma-3-4b-pt",
            "google/gemma-3-12b-it",
            "google/gemma-3-27b-it",
            "google/gemma-3n-E2B-it",
            "google/gemma-4-31B-it",
            "google/paligemma2-3b-pt-224",
            "google/paligemma-3b-mix-224",
            "google/medgemma-4b-it",
            "/opt/hf/google/gemma-3-4b-it",
            "/home/user/.cache/huggingface/hub/models--google--gemma-3-4b-it/snapshots/abc",
            "/tmp/gemma-2-2b-it/gemma-3-4b-it",
        ];
        for train in other {
            assert_ne!(llamafactory_template(train), "gemma2", "{train}");
        }
        assert_eq!(
            llamafactory_template("/tmp/gemma-2-2b-it/Phi-3-mini-4k-instruct"),
            "phi"
        );
        assert_eq!(
            llamafactory_template("/tmp/gemma-2-2b-it/Qwen2.5-0.5B-Instruct"),
            "qwen"
        );
        assert_eq!(
            llamafactory_template("/tmp/gemma-7b/Llama-3.2-3B-Instruct"),
            "llama3"
        );
    }

    #[test]
    fn llamafactory_template_maps_mistral_instruct_including_nested_paths() {
        let instruct = [
            "mistralai/Mistral-7B-Instruct-v0.1",
            "mistralai/Mistral-7B-Instruct-v0.2",
            "mistralai/Mistral-7B-Instruct-v0.3",
            "MISTRALAI/MISTRAL-7B-INSTRUCT-V0.3",
            "unsloth/mistral-7b-instruct-v0.2",
            "unsloth/mistral-7b-instruct-v0.3",
            "Mistral-7B-Instruct-v0.3",
            "/opt/hf/mistralai/Mistral-7B-Instruct-v0.3",
            "./weights/Mistral-7B-Instruct-v0.2",
            "/opt/hf/mistralai/Mistral-7B-Instruct-v0.3/weights",
            "/opt/hf/mistralai/Mistral-7B-Instruct-v0.2/snapshots/deadbeef",
            "/home/user/.cache/huggingface/hub/models--mistralai--Mistral-7B-Instruct-v0.2/snapshots/abc123",
            "/home/user/.cache/huggingface/hub/models--mistralai--Mistral-7B-Instruct-v0.3/snapshots/def456",
            "/home/user/.cache/huggingface/hub/models--mistralai--Mistral-7B-Instruct-v0.1/snapshots/ghi789",
            "/tmp/phi-4/Mistral-7B-Instruct-v0.3",
            "/tmp/qwen3-parent/Mistral-7B-Instruct-v0.2",
            "/tmp/gemma-2-2b-it/Mistral-7B-Instruct-v0.3",
            "/tmp/Llama-3.2-3B-Instruct/Mistral-7B-Instruct-v0.3",
            "/tmp/mistralai/Mistral-7B-Instruct-v0.3",
        ];
        for train in instruct {
            assert_eq!(llamafactory_template(train), "mistral", "{train}");
            assert_ne!(llamafactory_template(train), "mistral_small", "{train}");
            assert_ne!(llamafactory_template(train), "ministral", "{train}");
            assert_ne!(llamafactory_template(train), "mistral_7", "{train}");
        }
        assert_eq!(
            llamafactory_template("teknium/OpenHermes-2.5-Mistral-7B"),
            "mistral"
        );
        let base = [
            "mistralai/Mistral-7B-v0.1",
            "alpindale/Mistral-7B-v0.2-hf",
            "mistralai/Mistral-7B-v0.3",
            "/opt/hf/mistralai/Mistral-7B-v0.3",
            "/home/user/.cache/huggingface/hub/models--mistralai--Mistral-7B-v0.3/snapshots/abc",
        ];
        for train in base {
            assert_eq!(llamafactory_template(train), "mistral", "{train}");
        }
        let small = [
            "mistralai/Mistral-Small-24B-Base-2501",
            "mistralai/Mistral-Small-24B-Instruct-2501",
            "mistralai/Mistral-Small-3.1-24B-Base-2503",
            "mistralai/Mistral-Small-3.1-24B-Instruct-2503",
            "mistralai/Mistral-Small-3.2-24B-Instruct-2506",
            "/opt/hf/mistralai/Mistral-Small-24B-Instruct-2501",
            "/home/user/.cache/huggingface/hub/models--mistralai--Mistral-Small-3.2-24B-Instruct-2506/snapshots/abc",
            "/tmp/Mistral-7B-Instruct-v0.3/Mistral-Small-24B-Instruct-2501",
        ];
        for train in small {
            assert_eq!(llamafactory_template(train), "mistral_small", "{train}");
        }
        let nemo = [
            "mistralai/Mistral-Nemo-Base-2407",
            "mistralai/Mistral-Nemo-Instruct-2407",
            "/opt/hf/mistralai/Mistral-Nemo-Instruct-2407",
            "/home/user/.cache/huggingface/hub/models--mistralai--Mistral-Nemo-Instruct-2407/snapshots/abc",
            "/tmp/Mistral-7B-Instruct-v0.3/Mistral-Nemo-Instruct-2407",
        ];
        for train in nemo {
            assert_eq!(llamafactory_template(train), "ministral", "{train}");
        }
        let mixtral = [
            "mistralai/Mixtral-8x7B-v0.1",
            "mistralai/Mixtral-8x22B-v0.1",
            "mistralai/Mixtral-8x7B-Instruct-v0.1",
            "mistralai/Mixtral-8x22B-Instruct-v0.1",
            "/opt/hf/mistralai/Mixtral-8x7B-Instruct-v0.1",
            "/tmp/Mistral-7B-Instruct-v0.3/Mixtral-8x7B-Instruct-v0.1",
        ];
        for train in mixtral {
            assert_eq!(llamafactory_template(train), "mistral", "{train}");
        }
        assert_eq!(
            llamafactory_template("llava-hf/llava-v1.6-mistral-7b-hf"),
            "llava_next_mistral"
        );
        assert_eq!(
            llamafactory_template("LLaVA-NeXT-Mistral-7B-Chat"),
            "llava_next_mistral"
        );
        assert_eq!(
            llamafactory_template("/opt/hf/llava-v1.6-mistral-7b-hf/snapshots/abc"),
            "llava_next_mistral"
        );
        assert_eq!(
            llamafactory_template("/tmp/Mistral-7B-Instruct-v0.3/llava-v1.6-mistral-7b-hf"),
            "llava_next_mistral"
        );
        let unclaimed = [
            "mistralai/Ministral-8B-Instruct-2410",
            "mistralai/Ministral-3-3B-Instruct-2512",
            "mistralai/Ministral-3-8B-Base-2512",
            "mistralai/Codestral-22B-v0.1",
            "mistralai/Devstral-Small-2507",
            "mistral-community/pixtral-12b",
            "/opt/hf/mistral-community/pixtral-12b",
            "/tmp/mistralai/pixtral-12b",
        ];
        for train in unclaimed {
            assert_ne!(llamafactory_template(train), "mistral", "{train}");
            assert_ne!(llamafactory_template(train), "mistral_small", "{train}");
            assert_ne!(llamafactory_template(train), "ministral", "{train}");
            assert_eq!(llamafactory_template(train), "default", "{train}");
        }
        assert_eq!(
            llamafactory_template("/tmp/Mistral-7B-Instruct-v0.3/Phi-3-mini-4k-instruct"),
            "phi"
        );
        assert_eq!(
            llamafactory_template("/tmp/Mistral-7B-Instruct-v0.3/Qwen2.5-0.5B-Instruct"),
            "qwen"
        );
        assert_eq!(
            llamafactory_template("/tmp/Mistral-7B-v0.3/Llama-3.2-3B-Instruct"),
            "llama3"
        );
        assert_eq!(
            llamafactory_template("/tmp/Mistral-7B-Instruct-v0.3/gemma-2-2b-it"),
            "gemma2"
        );
    }

    #[test]
    fn phi3_qlora_prepare_emits_template_bnb_and_keeps_the_seat_split() {
        let root = tmp("phi3-qlora");
        let pack_path = repo_root().join("examples/fixtures/phi3-instruct.pack.json");
        let pack: PackManifest =
            serde_json::from_str(&std::fs::read_to_string(&pack_path).unwrap()).unwrap();
        assert_eq!(
            pack.train_base_model.as_deref(),
            Some("microsoft/Phi-3-mini-4k-instruct")
        );
        let estate = fixture_estate();
        let out = root.join("qlora");
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
            Some("microsoft/Phi-3-mini-4k-instruct")
        );
        assert!(!doc.promoted && !doc.auto_apply && !doc.estate_rewritten);
        let recipe = std::fs::read_to_string(out.join("recipe.yaml")).unwrap();
        assert!(
            recipe.lines().any(|line| line.trim() == "template: phi"),
            "{recipe}"
        );
        assert!(recipe.contains("quantization_bit: 4"), "{recipe}");
        assert!(recipe.contains("quantization_method: bnb"), "{recipe}");
        assert!(
            recipe.contains("model_name_or_path: \"microsoft/Phi-3-mini-4k-instruct\""),
            "{recipe}"
        );
        assert!(
            !recipe.lines().any(|line| {
                line.trim_start().starts_with("model_name_or_path:") && line.contains("llama3")
            }),
            "{recipe}"
        );
        let export = std::fs::read_to_string(out.join("export.yaml")).unwrap();
        assert!(
            export.lines().any(|line| line.trim() == "template: phi"),
            "{export}"
        );
        assert!(!export.contains("quantization_bit"), "{export}");
        assert!(
            export.contains("model_name_or_path: \"microsoft/Phi-3-mini-4k-instruct\""),
            "{export}"
        );
        let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
        let prepare_md = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
        for text in [&next, &prepare_md] {
            assert!(
                text.contains("Reproduce target beside Qwen LoRA/QLoRA."),
                "{text}"
            );
            assert!(text.contains("template phi"), "{text}");
            assert!(text.contains("Seat tag is llama3"), "{text}");
            assert!(text.contains("microsoft/Phi-3-mini-4k-instruct"), "{text}");
            assert!(text.contains("quantization_method bnb"), "{text}");
            assert!(text.contains("quantization_bit 4"), "{text}");
        }
        let lora_out = root.join("lora");
        let lora = run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &estate,
            &lora_out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(lora.base_model, "llama3");
        assert_eq!(
            lora.train_base_model.as_deref(),
            Some("microsoft/Phi-3-mini-4k-instruct")
        );
        let lora_recipe = std::fs::read_to_string(lora_out.join("recipe.yaml")).unwrap();
        assert!(
            lora_recipe
                .lines()
                .any(|line| line.trim() == "template: phi"),
            "{lora_recipe}"
        );
        assert!(
            !lora_recipe.contains("quantization_bit")
                && !lora_recipe.contains("quantization_method"),
            "{lora_recipe}"
        );
        let lora_next = std::fs::read_to_string(lora_out.join("NEXT.md")).unwrap();
        assert!(
            !lora_next.contains("Reproduce target beside Qwen LoRA/QLoRA."),
            "{lora_next}"
        );

        let nested_pack = fixture_pack();
        let nested_estate = with_train_base(
            seated_estate("llama3"),
            "./weights/microsoft/Phi-3.5-mini-instruct",
        );
        let nested_out = root.join("nested");
        let nested = run(
            LLAMAFACTORY_QLORA_ID,
            &nested_pack,
            &nested_estate,
            &nested_out,
            "train",
            "jason",
        )
        .unwrap();
        let nested_train = nested.train_base_model.as_deref().unwrap();
        assert!(
            nested_train.ends_with("/weights/microsoft/Phi-3.5-mini-instruct"),
            "{nested_train}"
        );
        assert_eq!(nested.base_model, "llama3");
        let nested_recipe = std::fs::read_to_string(nested_out.join("recipe.yaml")).unwrap();
        assert!(
            nested_recipe
                .lines()
                .any(|line| line.trim() == "template: phi"),
            "{nested_recipe}"
        );
        assert!(
            nested_recipe.contains("quantization_method: bnb"),
            "{nested_recipe}"
        );
        assert!(
            nested_recipe.contains("quantization_bit: 4"),
            "{nested_recipe}"
        );
        assert!(nested_recipe.contains(nested_train), "{nested_recipe}");
        let nested_next = std::fs::read_to_string(nested_out.join("NEXT.md")).unwrap();
        let nested_prepare = std::fs::read_to_string(nested_out.join("PREPARE.md")).unwrap();
        assert!(nested_next.contains("Reproduce target beside Qwen LoRA/QLoRA."));
        assert!(nested_prepare.contains("Reproduce target beside Qwen LoRA/QLoRA."));
        assert!(nested_next.contains(nested_train), "{nested_next}");

        let small_estate = with_train_base(
            seated_estate("llama3"),
            "/tmp/cell-one-hf/microsoft/Phi-3-small-8k-instruct",
        );
        let small_out = root.join("small");
        let small = run(
            LLAMAFACTORY_QLORA_ID,
            &nested_pack,
            &small_estate,
            &small_out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(small.base_model, "llama3");
        assert_eq!(
            small.train_base_model.as_deref(),
            Some("/tmp/cell-one-hf/microsoft/Phi-3-small-8k-instruct")
        );
        let small_recipe = std::fs::read_to_string(small_out.join("recipe.yaml")).unwrap();
        assert!(
            small_recipe
                .lines()
                .any(|line| line.trim() == "template: phi_small"),
            "{small_recipe}"
        );
        assert!(
            small_recipe.contains("quantization_method: bnb"),
            "{small_recipe}"
        );
        let small_next = std::fs::read_to_string(small_out.join("NEXT.md")).unwrap();
        assert!(small_next.contains("Reproduce target beside Qwen LoRA/QLoRA."));
        assert!(small_next.contains("phi_small"), "{small_next}");

        let qwen_estate = with_train_base(seated_estate("llama3"), "Qwen/Qwen2.5-0.5B-Instruct");
        let qwen_out = root.join("qwen");
        run(
            LLAMAFACTORY_QLORA_ID,
            &nested_pack,
            &qwen_estate,
            &qwen_out,
            "train",
            "jason",
        )
        .unwrap();
        let qwen_next = std::fs::read_to_string(qwen_out.join("NEXT.md")).unwrap();
        let qwen_prepare = std::fs::read_to_string(qwen_out.join("PREPARE.md")).unwrap();
        assert!(!qwen_next.contains("Reproduce target beside Qwen LoRA/QLoRA."));
        assert!(!qwen_prepare.contains("Reproduce target beside Qwen LoRA/QLoRA."));
    }

    #[test]
    fn llama32_qlora_prepare_emits_template_bnb_and_keeps_the_seat_split() {
        let root = tmp("llama32-qlora");
        let pack_path = repo_root().join("examples/fixtures/llama32-instruct.pack.json");
        let pack: PackManifest =
            serde_json::from_str(&std::fs::read_to_string(&pack_path).unwrap()).unwrap();
        assert_eq!(
            pack.train_base_model.as_deref(),
            Some("meta-llama/Llama-3.2-3B-Instruct")
        );
        assert_eq!(pack.model_hint.as_deref(), Some("llama3"));
        let estate = fixture_estate();
        let out = root.join("qlora");
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
            Some("meta-llama/Llama-3.2-3B-Instruct")
        );
        assert!(!doc.promoted && !doc.auto_apply && !doc.estate_rewritten);
        let recipe = std::fs::read_to_string(out.join("recipe.yaml")).unwrap();
        assert!(
            recipe.lines().any(|line| line.trim() == "template: llama3"),
            "{recipe}"
        );
        assert!(
            !recipe
                .lines()
                .any(|line| line.trim().starts_with("template: llama3_")),
            "{recipe}"
        );
        assert!(recipe.contains("quantization_bit: 4"), "{recipe}");
        assert!(recipe.contains("quantization_method: bnb"), "{recipe}");
        assert!(recipe.contains("Llama-3.2 vision uses mllama"), "{recipe}");
        assert!(
            recipe.contains("model_name_or_path: \"meta-llama/Llama-3.2-3B-Instruct\""),
            "{recipe}"
        );
        assert!(
            !recipe.lines().any(|line| {
                line.trim_start().starts_with("model_name_or_path:") && line.contains("\"llama3\"")
            }),
            "{recipe}"
        );
        assert!(recipe.contains("does not download weights"), "{recipe}");
        assert!(recipe.contains("does not run llamafactory-cli"), "{recipe}");
        let export = std::fs::read_to_string(out.join("export.yaml")).unwrap();
        assert!(
            export.lines().any(|line| line.trim() == "template: llama3"),
            "{export}"
        );
        assert!(!export.contains("quantization_bit"), "{export}");
        assert!(
            export.contains("model_name_or_path: \"meta-llama/Llama-3.2-3B-Instruct\""),
            "{export}"
        );
        let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
        let prepare_md = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
        for text in [&next, &prepare_md] {
            assert!(
                text.contains("Reproduce target beside Phi-3 and Qwen LoRA/QLoRA."),
                "{text}"
            );
            assert!(
                !text.contains("Reproduce target beside Qwen LoRA/QLoRA."),
                "{text}"
            );
            assert!(text.contains("template llama3"), "{text}");
            assert!(text.contains("template mllama"), "{text}");
            assert!(text.contains("Seat tag is llama3"), "{text}");
            assert!(text.contains("meta-llama/Llama-3.2-3B-Instruct"), "{text}");
            assert!(text.contains("quantization_method bnb"), "{text}");
            assert!(text.contains("quantization_bit 4"), "{text}");
            assert!(!text.contains("READY_FOR_LIVE_TEST: yes"), "{text}");
        }
        assert!(!out.join("train.py").exists());

        let lora_out = root.join("lora");
        let lora = run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &estate,
            &lora_out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(lora.base_model, "llama3");
        assert_eq!(
            lora.train_base_model.as_deref(),
            Some("meta-llama/Llama-3.2-3B-Instruct")
        );
        let lora_recipe = std::fs::read_to_string(lora_out.join("recipe.yaml")).unwrap();
        assert!(
            lora_recipe
                .lines()
                .any(|line| line.trim() == "template: llama3"),
            "{lora_recipe}"
        );
        assert!(
            !lora_recipe.contains("quantization_bit")
                && !lora_recipe.contains("quantization_method"),
            "{lora_recipe}"
        );
        let lora_next = std::fs::read_to_string(lora_out.join("NEXT.md")).unwrap();
        assert!(
            !lora_next.contains("Reproduce target beside Phi-3 and Qwen LoRA/QLoRA."),
            "{lora_next}"
        );

        let bare = fixture_pack();
        let seated = seated_estate("llama3");
        for bad in ["llama3", "llama3:latest", "./llama3", "../llama3"] {
            let bad_estate = with_train_base(seated.clone(), bad);
            let bad_out = root.join(format!(
                "seat-{}",
                bad.trim_start_matches('.').replace('/', "_")
            ));
            let err = run(
                LLAMAFACTORY_QLORA_ID,
                &bare,
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
            assert!(!err.to_string().contains("meta-llama"), "{bad}: {err}");
            assert!(!bad_out.exists(), "{bad}");
        }

        let nested_estate =
            with_train_base(seated.clone(), "./weights/meta-llama/Llama-3.2-1B-Instruct");
        let nested_out = root.join("nested-1b");
        let nested = run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &nested_estate,
            &nested_out,
            "train",
            "jason",
        )
        .unwrap();
        let nested_train = nested.train_base_model.as_deref().unwrap();
        assert!(
            nested_train.ends_with("/weights/meta-llama/Llama-3.2-1B-Instruct"),
            "{nested_train}"
        );
        assert_eq!(nested.base_model, "llama3");
        let nested_recipe = std::fs::read_to_string(nested_out.join("recipe.yaml")).unwrap();
        assert!(
            nested_recipe
                .lines()
                .any(|line| line.trim() == "template: llama3"),
            "{nested_recipe}"
        );
        assert!(
            nested_recipe.contains("quantization_method: bnb"),
            "{nested_recipe}"
        );
        assert!(
            nested_recipe.contains("quantization_bit: 4"),
            "{nested_recipe}"
        );
        assert!(nested_recipe.contains(nested_train), "{nested_recipe}");
        let nested_next = std::fs::read_to_string(nested_out.join("NEXT.md")).unwrap();
        let nested_prepare = std::fs::read_to_string(nested_out.join("PREPARE.md")).unwrap();
        assert!(nested_next.contains("Reproduce target beside Phi-3 and Qwen LoRA/QLoRA."));
        assert!(nested_prepare.contains("Reproduce target beside Phi-3 and Qwen LoRA/QLoRA."));
        assert!(
            nested_next.contains("Llama-3.2-1B-Instruct"),
            "{nested_next}"
        );

        let base_estate = with_train_base(seated.clone(), "meta-llama/Llama-3.2-3B");
        let base_out = root.join("base-3b");
        run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &base_estate,
            &base_out,
            "train",
            "jason",
        )
        .unwrap();
        let base_recipe = std::fs::read_to_string(base_out.join("recipe.yaml")).unwrap();
        assert!(
            base_recipe
                .lines()
                .any(|line| line.trim() == "template: llama3"),
            "{base_recipe}"
        );
        let base_next = std::fs::read_to_string(base_out.join("NEXT.md")).unwrap();
        assert!(
            !base_next.contains("Reproduce target beside Phi-3 and Qwen LoRA/QLoRA."),
            "{base_next}"
        );

        let older_estate = with_train_base(seated.clone(), "meta-llama/Meta-Llama-3.1-8B-Instruct");
        let older_out = root.join("llama31");
        run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &older_estate,
            &older_out,
            "train",
            "jason",
        )
        .unwrap();
        let older_recipe = std::fs::read_to_string(older_out.join("recipe.yaml")).unwrap();
        assert!(
            older_recipe
                .lines()
                .any(|line| line.trim() == "template: llama3"),
            "{older_recipe}"
        );
        let older_next = std::fs::read_to_string(older_out.join("NEXT.md")).unwrap();
        assert!(
            !older_next.contains("Reproduce target beside Phi-3 and Qwen LoRA/QLoRA."),
            "{older_next}"
        );

        let vision_estate = with_train_base(
            seated.clone(),
            "/tmp/cell-one-hf/meta-llama/Llama-3.2-11B-Vision-Instruct",
        );
        let vision_out = root.join("vision");
        let vision = run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &vision_estate,
            &vision_out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(vision.base_model, "llama3");
        assert_eq!(
            vision.train_base_model.as_deref(),
            Some("/tmp/cell-one-hf/meta-llama/Llama-3.2-11B-Vision-Instruct")
        );
        let vision_recipe = std::fs::read_to_string(vision_out.join("recipe.yaml")).unwrap();
        assert!(
            vision_recipe
                .lines()
                .any(|line| line.trim() == "template: mllama"),
            "{vision_recipe}"
        );
        assert!(
            vision_recipe.contains("quantization_method: bnb"),
            "{vision_recipe}"
        );
        assert!(
            vision_recipe.contains("quantization_bit: 4"),
            "{vision_recipe}"
        );
        let vision_next = std::fs::read_to_string(vision_out.join("NEXT.md")).unwrap();
        assert!(
            !vision_next.contains("Reproduce target beside Phi-3 and Qwen LoRA/QLoRA."),
            "{vision_next}"
        );
        assert!(
            !vision_next.contains("Reproduce target beside Qwen LoRA/QLoRA."),
            "{vision_next}"
        );

        let thirty_estate = with_train_base(seated.clone(), "huggyllama/llama-30b");
        let thirty_out = root.join("llama30");
        run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &thirty_estate,
            &thirty_out,
            "train",
            "jason",
        )
        .unwrap();
        let thirty_recipe = std::fs::read_to_string(thirty_out.join("recipe.yaml")).unwrap();
        assert!(
            thirty_recipe
                .lines()
                .any(|line| line.trim() == "template: default"),
            "{thirty_recipe}"
        );
        let thirty_next = std::fs::read_to_string(thirty_out.join("NEXT.md")).unwrap();
        assert!(
            !thirty_next.contains("Reproduce target beside Phi-3 and Qwen LoRA/QLoRA."),
            "{thirty_next}"
        );

        let llava_estate = with_train_base(seated.clone(), "llava-hf/llama3-llava-next-8b-hf");
        let llava_out = root.join("llava");
        run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &llava_estate,
            &llava_out,
            "train",
            "jason",
        )
        .unwrap();
        let llava_recipe = std::fs::read_to_string(llava_out.join("recipe.yaml")).unwrap();
        assert!(
            llava_recipe
                .lines()
                .any(|line| line.trim() == "template: llava_next_llama3"),
            "{llava_recipe}"
        );
        let llava_next = std::fs::read_to_string(llava_out.join("NEXT.md")).unwrap();
        assert!(
            !llava_next.contains("Reproduce target beside Phi-3 and Qwen LoRA/QLoRA."),
            "{llava_next}"
        );

        let qwen_estate = with_train_base(seated.clone(), "Qwen/Qwen2.5-0.5B-Instruct");
        let qwen_out = root.join("qwen");
        run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &qwen_estate,
            &qwen_out,
            "train",
            "jason",
        )
        .unwrap();
        let qwen_next = std::fs::read_to_string(qwen_out.join("NEXT.md")).unwrap();
        assert!(
            !qwen_next.contains("Reproduce target beside Phi-3 and Qwen LoRA/QLoRA."),
            "{qwen_next}"
        );

        let mut sacred_pack = pack.clone();
        sacred_pack.id = "cyera".into();
        let sacred_out = root.join("sacred-id");
        let sacred = run(
            LLAMAFACTORY_QLORA_ID,
            &sacred_pack,
            &estate,
            &sacred_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(sacred.to_string().contains("refuse:sacred"), "{sacred}");
        assert!(!sacred_out.exists());

        let sacred_base = with_train_base(seated.clone(), "cyera/Llama-3.2-3B-Instruct");
        let sacred_base_out = root.join("sacred-base");
        let sacred_train = run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &sacred_base,
            &sacred_base_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(
            sacred_train.to_string().contains("refuse:sacred"),
            "{sacred_train}"
        );
        assert!(!sacred_base_out.exists());

        let sku_base = with_train_base(
            seated.clone(),
            "/tmp/cell-one-hf/5090/Llama-3.2-3B-Instruct",
        );
        let sku_out = root.join("sku-base");
        let sku = run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &sku_base,
            &sku_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(sku.to_string().contains("refuse:sku-banned"), "{sku}");
        assert!(!sku_out.exists());

        let mut local_only = estate.clone();
        local_only
            .model_bindings
            .retain(|binding| binding.class != ModelClass::Frontier);
        let frontier_state = root.join("frontier-cell");
        std::fs::create_dir_all(frontier_state.join("feed")).unwrap();
        std::fs::write(
            frontier_state.join("feed/events.jsonl"),
            "{\"kind\":\"model.frontier.complete\",\"object_class\":\"frontier\",\"note\":\"bytes=4\",\"ts\":\"2026-09-21T00:00:00Z\"}\n",
        )
        .unwrap();
        let frontier_out = root.join("frontier");
        let frontier = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &local_only,
            &frontier_out,
            &frontier_state,
            true,
        )
        .unwrap_err();
        assert!(
            frontier.to_string().contains("refuse:frontier-invent"),
            "{frontier}"
        );
        assert!(!frontier_out.exists());
    }

    #[test]
    fn gemma2_qlora_prepare_emits_template_bnb_and_keeps_the_seat_split() {
        const GEMMA_NOTE: &str = "Reproduce target beside Phi-3, Llama-3.2, and Qwen LoRA/QLoRA.";
        let root = tmp("gemma2-qlora");
        let pack_path = repo_root().join("examples/fixtures/gemma2-instruct.pack.json");
        let pack: PackManifest =
            serde_json::from_str(&std::fs::read_to_string(&pack_path).unwrap()).unwrap();
        assert_eq!(
            pack.train_base_model.as_deref(),
            Some("google/gemma-2-2b-it")
        );
        assert_eq!(pack.model_hint.as_deref(), Some("llama3"));
        let estate = fixture_estate();
        let out = root.join("qlora");
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
            Some("google/gemma-2-2b-it")
        );
        assert!(!doc.promoted && !doc.auto_apply && !doc.estate_rewritten);
        let recipe = std::fs::read_to_string(out.join("recipe.yaml")).unwrap();
        assert!(
            recipe.lines().any(|line| line.trim() == "template: gemma2"),
            "{recipe}"
        );
        assert!(
            !recipe.lines().any(|line| line.trim() == "template: gemma"),
            "{recipe}"
        );
        assert!(
            !recipe
                .lines()
                .any(|line| line.trim().starts_with("template: gemma_")),
            "{recipe}"
        );
        assert!(recipe.contains("quantization_bit: 4"), "{recipe}");
        assert!(recipe.contains("quantization_method: bnb"), "{recipe}");
        assert!(
            recipe.contains("Gemma-3 is a different group. This scan does not label it gemma2."),
            "{recipe}"
        );
        assert!(
            recipe.contains("model_name_or_path: \"google/gemma-2-2b-it\""),
            "{recipe}"
        );
        assert!(
            !recipe.lines().any(|line| {
                line.trim_start().starts_with("model_name_or_path:") && line.contains("\"llama3\"")
            }),
            "{recipe}"
        );
        assert!(recipe.contains("does not download weights"), "{recipe}");
        assert!(recipe.contains("does not run llamafactory-cli"), "{recipe}");
        let export = std::fs::read_to_string(out.join("export.yaml")).unwrap();
        assert!(
            export.lines().any(|line| line.trim() == "template: gemma2"),
            "{export}"
        );
        assert!(!export.contains("quantization_bit"), "{export}");
        assert!(
            export.contains("model_name_or_path: \"google/gemma-2-2b-it\""),
            "{export}"
        );
        let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
        let prepare_md = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
        for text in [&next, &prepare_md] {
            assert!(text.contains(GEMMA_NOTE), "{text}");
            assert!(
                !text.contains("Reproduce target beside Qwen LoRA/QLoRA."),
                "{text}"
            );
            assert!(
                !text.contains("Reproduce target beside Phi-3 and Qwen LoRA/QLoRA."),
                "{text}"
            );
            assert!(text.contains("template gemma2"), "{text}");
            assert!(text.contains("Seat tag is llama3"), "{text}");
            assert!(text.contains("google/gemma-2-2b-it"), "{text}");
            assert!(text.contains("quantization_method bnb"), "{text}");
            assert!(text.contains("quantization_bit 4"), "{text}");
            assert!(!text.contains("READY_FOR_LIVE_TEST: yes"), "{text}");
        }
        assert!(!out.join("train.py").exists());

        let lora_out = root.join("lora");
        let lora = run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &estate,
            &lora_out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(lora.base_model, "llama3");
        assert_eq!(
            lora.train_base_model.as_deref(),
            Some("google/gemma-2-2b-it")
        );
        let lora_recipe = std::fs::read_to_string(lora_out.join("recipe.yaml")).unwrap();
        assert!(
            lora_recipe
                .lines()
                .any(|line| line.trim() == "template: gemma2"),
            "{lora_recipe}"
        );
        assert!(
            !lora_recipe.contains("quantization_bit")
                && !lora_recipe.contains("quantization_method"),
            "{lora_recipe}"
        );
        let lora_next = std::fs::read_to_string(lora_out.join("NEXT.md")).unwrap();
        let lora_prepare = std::fs::read_to_string(lora_out.join("PREPARE.md")).unwrap();
        assert!(!lora_next.contains(GEMMA_NOTE), "{lora_next}");
        assert!(!lora_prepare.contains(GEMMA_NOTE), "{lora_prepare}");

        let bare = fixture_pack();
        let seated = seated_estate("llama3");
        for bad in ["llama3", "llama3:latest", "./llama3", "../llama3"] {
            let bad_estate = with_train_base(seated.clone(), bad);
            let bad_out = root.join(format!(
                "seat-{}",
                bad.trim_start_matches('.').replace('/', "_")
            ));
            let err = run(
                LLAMAFACTORY_QLORA_ID,
                &bare,
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
            assert!(!err.to_string().contains("google/gemma"), "{bad}: {err}");
            assert!(!bad_out.exists(), "{bad}");
        }
        for bad in ["./gemma-2-2b-it", "/opt/hf/gemma-2-2b-it"] {
            let bad_estate = with_train_base(seated.clone(), bad);
            let bad_out = root.join(format!("leaf-{}", bad.replace('/', "_")));
            let err = run(
                LLAMAFACTORY_QLORA_ID,
                &bare,
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
            assert!(!bad_out.exists(), "{bad}");
        }

        let nested_estate =
            with_train_base(seated.clone(), "./weights/google/gemma-2-9b-it/weights");
        let nested_out = root.join("nested-9b");
        let nested = run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &nested_estate,
            &nested_out,
            "train",
            "jason",
        )
        .unwrap();
        let nested_train = nested.train_base_model.as_deref().unwrap();
        assert!(
            nested_train.ends_with("/weights/google/gemma-2-9b-it/weights"),
            "{nested_train}"
        );
        assert_eq!(nested.base_model, "llama3");
        let nested_recipe = std::fs::read_to_string(nested_out.join("recipe.yaml")).unwrap();
        assert!(
            nested_recipe
                .lines()
                .any(|line| line.trim() == "template: gemma2"),
            "{nested_recipe}"
        );
        assert!(
            nested_recipe.contains("quantization_method: bnb"),
            "{nested_recipe}"
        );
        assert!(
            nested_recipe.contains("quantization_bit: 4"),
            "{nested_recipe}"
        );
        assert!(nested_recipe.contains(nested_train), "{nested_recipe}");
        let nested_next = std::fs::read_to_string(nested_out.join("NEXT.md")).unwrap();
        let nested_prepare = std::fs::read_to_string(nested_out.join("PREPARE.md")).unwrap();
        assert!(nested_next.contains(GEMMA_NOTE), "{nested_next}");
        assert!(nested_prepare.contains(GEMMA_NOTE), "{nested_prepare}");

        let cases = [
            ("google/gemma-2-27b-it", "gemma2", true),
            ("google/gemma-2-2b", "gemma2", false),
            ("google/gemma-2-9b", "gemma2", false),
            ("unsloth/gemma2-2b-it", "gemma2", true),
            ("/opt/hf/Gemma-2-2B-Instruct", "gemma2", true),
            (
                "/home/user/.cache/huggingface/hub/models--google--gemma-2-9b-it/snapshots/abc123def456",
                "gemma2",
                true,
            ),
            ("/tmp/gemma-7b/Gemma-2-2B-it", "gemma2", true),
            ("/tmp/Gemma-2-2B-it/Gemma-7B", "gemma", false),
            ("/tmp/Gemma-2-2B-it/Gemma-2B-it", "gemma", false),
            ("google/gemma-2b", "gemma", false),
            ("google/gemma-2b-it", "gemma", false),
            ("google/gemma-7b", "gemma", false),
            ("google/gemma-7b-it", "gemma", false),
            ("google/gemma-1.1-7b-it", "gemma", false),
            ("google/codegemma-7b-it", "gemma", false),
            ("google/gemma-3-4b-it", "gemma", false),
            ("google/gemma-3-270m-it", "gemma", false),
            ("google/gemma-3-1b-it", "gemma", false),
            ("google/paligemma2-3b-pt-224", "gemma", false),
            ("Qwen/Qwen2.5-0.5B-Instruct", "qwen", false),
            ("microsoft/Phi-3-mini-4k-instruct", "phi", false),
            ("meta-llama/Llama-3.2-3B-Instruct", "llama3", false),
        ];
        for (idx, (train, template, note)) in cases.iter().enumerate() {
            let case_estate = with_train_base(seated.clone(), train);
            let case_out = root.join(format!("case-{idx}"));
            let case_doc = run(
                LLAMAFACTORY_QLORA_ID,
                &bare,
                &case_estate,
                &case_out,
                "train",
                "jason",
            )
            .unwrap();
            assert_eq!(case_doc.base_model, "llama3", "{train}");
            assert_eq!(case_doc.seat_tag.as_deref(), Some("llama3"), "{train}");
            assert_eq!(
                case_doc.train_base_model.as_deref(),
                Some(*train),
                "{train}"
            );
            assert!(!case_doc.promoted && !case_doc.auto_apply && !case_doc.estate_rewritten);
            let case_recipe = std::fs::read_to_string(case_out.join("recipe.yaml")).unwrap();
            let template_line = format!("template: {template}");
            assert!(
                case_recipe.lines().any(|line| line.trim() == template_line),
                "{train}\n{case_recipe}"
            );
            assert!(case_recipe.contains("quantization_method: bnb"), "{train}");
            assert!(case_recipe.contains("quantization_bit: 4"), "{train}");
            if *template != "gemma2" {
                assert!(
                    !case_recipe
                        .lines()
                        .any(|line| line.trim() == "template: gemma2"),
                    "{train}\n{case_recipe}"
                );
            }
            let case_next = std::fs::read_to_string(case_out.join("NEXT.md")).unwrap();
            let case_prepare = std::fs::read_to_string(case_out.join("PREPARE.md")).unwrap();
            assert_eq!(
                case_next.contains(GEMMA_NOTE),
                *note,
                "{train}\n{case_next}"
            );
            assert_eq!(
                case_prepare.contains(GEMMA_NOTE),
                *note,
                "{train}\n{case_prepare}"
            );
            let lora_case = root.join(format!("lora-case-{idx}"));
            run(
                LLAMAFACTORY_LORA_ID,
                &bare,
                &case_estate,
                &lora_case,
                "train",
                "jason",
            )
            .unwrap();
            let lora_case_next = std::fs::read_to_string(lora_case.join("NEXT.md")).unwrap();
            assert!(
                !lora_case_next.contains(GEMMA_NOTE),
                "{train}\n{lora_case_next}"
            );
        }

        let mut sacred_pack = pack.clone();
        sacred_pack.id = "cyera".into();
        let sacred_out = root.join("sacred-id");
        let sacred = run(
            LLAMAFACTORY_QLORA_ID,
            &sacred_pack,
            &estate,
            &sacred_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(sacred.to_string().contains("refuse:sacred"), "{sacred}");
        assert!(!sacred_out.exists());

        let sacred_base = with_train_base(seated.clone(), "cyera/gemma-2-2b-it");
        let sacred_base_out = root.join("sacred-base");
        let sacred_train = run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &sacred_base,
            &sacred_base_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(
            sacred_train.to_string().contains("refuse:sacred"),
            "{sacred_train}"
        );
        assert!(!sacred_base_out.exists());

        let sku_base = with_train_base(seated.clone(), "/tmp/cell-one-hf/5090/gemma-2-2b-it");
        let sku_out = root.join("sku-base");
        let sku = run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &sku_base,
            &sku_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(sku.to_string().contains("refuse:sku-banned"), "{sku}");
        assert!(!sku_out.exists());

        let mut local_only = estate.clone();
        local_only
            .model_bindings
            .retain(|binding| binding.class != ModelClass::Frontier);
        let frontier_state = root.join("frontier-cell");
        std::fs::create_dir_all(frontier_state.join("feed")).unwrap();
        std::fs::write(
            frontier_state.join("feed/events.jsonl"),
            "{\"kind\":\"model.frontier.complete\",\"object_class\":\"frontier\",\"note\":\"bytes=4\",\"ts\":\"2026-09-21T00:00:00Z\"}\n",
        )
        .unwrap();
        let frontier_out = root.join("frontier");
        let frontier = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &local_only,
            &frontier_out,
            &frontier_state,
            true,
        )
        .unwrap_err();
        assert!(
            frontier.to_string().contains("refuse:frontier-invent"),
            "{frontier}"
        );
        assert!(!frontier_out.exists());
    }

    #[test]
    fn mistral_qlora_prepare_emits_template_bnb_and_keeps_the_seat_split() {
        const MISTRAL_NOTE: &str =
            "Reproduce target beside Phi-3, Llama-3.2, Gemma-2, and Qwen LoRA/QLoRA.";
        let root = tmp("mistral-qlora");
        let pack_path = repo_root().join("examples/fixtures/mistral-instruct.pack.json");
        let pack: PackManifest =
            serde_json::from_str(&std::fs::read_to_string(&pack_path).unwrap()).unwrap();
        assert_eq!(
            pack.train_base_model.as_deref(),
            Some("mistralai/Mistral-7B-Instruct-v0.3")
        );
        assert_eq!(pack.model_hint.as_deref(), Some("llama3"));
        let estate = fixture_estate();
        let out = root.join("qlora");
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
            Some("mistralai/Mistral-7B-Instruct-v0.3")
        );
        assert!(!doc.promoted && !doc.auto_apply && !doc.estate_rewritten);
        let recipe = std::fs::read_to_string(out.join("recipe.yaml")).unwrap();
        assert!(
            recipe
                .lines()
                .any(|line| line.trim() == "template: mistral"),
            "{recipe}"
        );
        assert!(
            !recipe
                .lines()
                .any(|line| line.trim().starts_with("template: mistral_")),
            "{recipe}"
        );
        assert!(
            !recipe
                .lines()
                .any(|line| line.trim().starts_with("template: ministral")),
            "{recipe}"
        );
        assert!(recipe.contains("quantization_bit: 4"), "{recipe}");
        assert!(recipe.contains("quantization_method: bnb"), "{recipe}");
        assert!(
            recipe.contains("Ministral, Ministral-3, Codestral, Devstral, and Pixtral are different groups. This scan does not label them mistral."),
            "{recipe}"
        );
        assert!(
            recipe.contains("model_name_or_path: \"mistralai/Mistral-7B-Instruct-v0.3\""),
            "{recipe}"
        );
        assert!(
            !recipe.lines().any(|line| {
                line.trim_start().starts_with("model_name_or_path:") && line.contains("\"llama3\"")
            }),
            "{recipe}"
        );
        assert!(recipe.contains("does not download weights"), "{recipe}");
        assert!(recipe.contains("does not run llamafactory-cli"), "{recipe}");
        let export = std::fs::read_to_string(out.join("export.yaml")).unwrap();
        assert!(
            export
                .lines()
                .any(|line| line.trim() == "template: mistral"),
            "{export}"
        );
        assert!(!export.contains("quantization_bit"), "{export}");
        assert!(
            export.contains("model_name_or_path: \"mistralai/Mistral-7B-Instruct-v0.3\""),
            "{export}"
        );
        let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
        let prepare_md = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
        for text in [&next, &prepare_md] {
            assert!(text.contains(MISTRAL_NOTE), "{text}");
            assert!(
                text.contains("uses LLaMA-Factory template mistral."),
                "{text}"
            );
            assert!(text.contains("A Mistral-7B base checkpoint"), "{text}");
            assert!(
                !text.contains("Reproduce target beside Qwen LoRA/QLoRA."),
                "{text}"
            );
            assert!(
                !text.contains("Reproduce target beside Phi-3 and Qwen LoRA/QLoRA."),
                "{text}"
            );
            assert!(
                !text.contains("Reproduce target beside Phi-3, Llama-3.2, and Qwen LoRA/QLoRA."),
                "{text}"
            );
            assert!(text.contains("Seat tag is llama3"), "{text}");
            assert!(
                text.contains("mistralai/Mistral-7B-Instruct-v0.3"),
                "{text}"
            );
            assert!(text.contains("quantization_method bnb"), "{text}");
            assert!(text.contains("quantization_bit 4"), "{text}");
            assert!(!text.contains("READY_FOR_LIVE_TEST: yes"), "{text}");
        }
        assert!(!out.join("train.py").exists());

        let lora_out = root.join("lora");
        let lora = run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &estate,
            &lora_out,
            "train",
            "jason",
        )
        .unwrap();
        assert_eq!(lora.base_model, "llama3");
        assert_eq!(
            lora.train_base_model.as_deref(),
            Some("mistralai/Mistral-7B-Instruct-v0.3")
        );
        let lora_recipe = std::fs::read_to_string(lora_out.join("recipe.yaml")).unwrap();
        assert!(
            lora_recipe
                .lines()
                .any(|line| line.trim() == "template: mistral"),
            "{lora_recipe}"
        );
        assert!(
            !lora_recipe.contains("quantization_bit")
                && !lora_recipe.contains("quantization_method"),
            "{lora_recipe}"
        );
        let lora_next = std::fs::read_to_string(lora_out.join("NEXT.md")).unwrap();
        let lora_prepare = std::fs::read_to_string(lora_out.join("PREPARE.md")).unwrap();
        assert!(!lora_next.contains(MISTRAL_NOTE), "{lora_next}");
        assert!(!lora_prepare.contains(MISTRAL_NOTE), "{lora_prepare}");

        let bare = fixture_pack();
        let seated = seated_estate("llama3");
        for bad in ["llama3", "llama3:latest", "./llama3", "../llama3"] {
            let bad_estate = with_train_base(seated.clone(), bad);
            let bad_out = root.join(format!(
                "seat-{}",
                bad.trim_start_matches('.').replace('/', "_")
            ));
            let err = run(
                LLAMAFACTORY_QLORA_ID,
                &bare,
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
            assert!(!err.to_string().contains("mistralai/"), "{bad}: {err}");
            assert!(!bad_out.exists(), "{bad}");
        }
        for bad in [
            "./mistral-7b-instruct-v0.3",
            "/opt/hf/mistral-7b-instruct-v0.3",
        ] {
            let bad_estate = with_train_base(seated.clone(), bad);
            let bad_out = root.join(format!("leaf-{}", bad.replace('/', "_")));
            let err = run(
                LLAMAFACTORY_QLORA_ID,
                &bare,
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
            assert!(!bad_out.exists(), "{bad}");
        }

        let nested_estate = with_train_base(
            seated.clone(),
            "./weights/mistralai/Mistral-7B-Instruct-v0.2/weights",
        );
        let nested_out = root.join("nested-v02");
        let nested = run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &nested_estate,
            &nested_out,
            "train",
            "jason",
        )
        .unwrap();
        let nested_train = nested.train_base_model.as_deref().unwrap();
        assert!(
            nested_train.ends_with("/weights/mistralai/Mistral-7B-Instruct-v0.2/weights"),
            "{nested_train}"
        );
        assert_eq!(nested.base_model, "llama3");
        let nested_recipe = std::fs::read_to_string(nested_out.join("recipe.yaml")).unwrap();
        assert!(
            nested_recipe
                .lines()
                .any(|line| line.trim() == "template: mistral"),
            "{nested_recipe}"
        );
        assert!(
            nested_recipe.contains("quantization_method: bnb"),
            "{nested_recipe}"
        );
        assert!(
            nested_recipe.contains("quantization_bit: 4"),
            "{nested_recipe}"
        );
        assert!(nested_recipe.contains(nested_train), "{nested_recipe}");
        let nested_next = std::fs::read_to_string(nested_out.join("NEXT.md")).unwrap();
        let nested_prepare = std::fs::read_to_string(nested_out.join("PREPARE.md")).unwrap();
        assert!(nested_next.contains(MISTRAL_NOTE), "{nested_next}");
        assert!(nested_prepare.contains(MISTRAL_NOTE), "{nested_prepare}");

        let cases = [
            ("mistralai/Mistral-7B-Instruct-v0.1", "mistral", true),
            ("mistralai/Mistral-7B-Instruct-v0.2", "mistral", true),
            ("mistralai/Mistral-7B-v0.1", "mistral", false),
            ("mistralai/Mistral-7B-v0.3", "mistral", false),
            ("alpindale/Mistral-7B-v0.2-hf", "mistral", false),
            ("teknium/OpenHermes-2.5-Mistral-7B", "mistral", false),
            ("unsloth/mistral-7b-instruct-v0.3", "mistral", true),
            (
                "/home/user/.cache/huggingface/hub/models--mistralai--Mistral-7B-Instruct-v0.3/snapshots/abc123def456",
                "mistral",
                true,
            ),
            (
                "/tmp/qwen3-parent/Mistral-7B-Instruct-v0.2",
                "mistral",
                true,
            ),
            (
                "/tmp/Mistral-7B-Instruct-v0.3/Qwen2.5-0.5B-Instruct",
                "qwen",
                false,
            ),
            (
                "mistralai/Mistral-Small-24B-Instruct-2501",
                "mistral_small",
                false,
            ),
            (
                "mistralai/Mistral-Small-3.1-24B-Instruct-2503",
                "mistral_small",
                false,
            ),
            ("mistralai/Mistral-Nemo-Instruct-2407", "ministral", false),
            ("mistralai/Mistral-Nemo-Base-2407", "ministral", false),
            ("mistralai/Mixtral-8x7B-Instruct-v0.1", "mistral", false),
            ("mistralai/Mixtral-8x7B-v0.1", "mistral", false),
            (
                "llava-hf/llava-v1.6-mistral-7b-hf",
                "llava_next_mistral",
                false,
            ),
            ("mistralai/Ministral-8B-Instruct-2410", "default", false),
            (
                "mistralai/Ministral-3-8B-Instruct-2512",
                "default",
                false,
            ),
            ("mistralai/Codestral-22B-v0.1", "default", false),
            ("mistralai/Devstral-Small-2507", "default", false),
            ("mistral-community/pixtral-12b", "default", false),
            ("Qwen/Qwen2.5-0.5B-Instruct", "qwen", false),
            ("microsoft/Phi-3-mini-4k-instruct", "phi", false),
            ("meta-llama/Llama-3.2-3B-Instruct", "llama3", false),
            ("google/gemma-2-2b-it", "gemma2", false),
        ];
        for (idx, (train, template, note)) in cases.iter().enumerate() {
            let case_estate = with_train_base(seated.clone(), train);
            let case_out = root.join(format!("case-{idx}"));
            let case_doc = run(
                LLAMAFACTORY_QLORA_ID,
                &bare,
                &case_estate,
                &case_out,
                "train",
                "jason",
            )
            .unwrap();
            assert_eq!(case_doc.base_model, "llama3", "{train}");
            assert_eq!(case_doc.seat_tag.as_deref(), Some("llama3"), "{train}");
            assert_eq!(
                case_doc.train_base_model.as_deref(),
                Some(*train),
                "{train}"
            );
            assert!(!case_doc.promoted && !case_doc.auto_apply && !case_doc.estate_rewritten);
            let case_recipe = std::fs::read_to_string(case_out.join("recipe.yaml")).unwrap();
            let template_line = format!("template: {template}");
            assert!(
                case_recipe.lines().any(|line| line.trim() == template_line),
                "{train}\n{case_recipe}"
            );
            assert!(case_recipe.contains("quantization_method: bnb"), "{train}");
            assert!(case_recipe.contains("quantization_bit: 4"), "{train}");
            if *template != "mistral" {
                assert!(
                    !case_recipe
                        .lines()
                        .any(|line| line.trim() == "template: mistral"),
                    "{train}\n{case_recipe}"
                );
            }
            let case_export = std::fs::read_to_string(case_out.join("export.yaml")).unwrap();
            assert!(
                case_export.lines().any(|line| line.trim() == template_line),
                "{train}\n{case_export}"
            );
            assert!(!case_export.contains("quantization_bit"), "{train}");
            let case_next = std::fs::read_to_string(case_out.join("NEXT.md")).unwrap();
            let case_prepare = std::fs::read_to_string(case_out.join("PREPARE.md")).unwrap();
            assert_eq!(
                case_next.contains(MISTRAL_NOTE),
                *note,
                "{train}\n{case_next}"
            );
            assert_eq!(
                case_prepare.contains(MISTRAL_NOTE),
                *note,
                "{train}\n{case_prepare}"
            );
            let lora_case = root.join(format!("lora-case-{idx}"));
            run(
                LLAMAFACTORY_LORA_ID,
                &bare,
                &case_estate,
                &lora_case,
                "train",
                "jason",
            )
            .unwrap();
            let lora_case_next = std::fs::read_to_string(lora_case.join("NEXT.md")).unwrap();
            assert!(
                !lora_case_next.contains(MISTRAL_NOTE),
                "{train}\n{lora_case_next}"
            );
        }

        let mut sacred_pack = pack.clone();
        sacred_pack.id = "cyera".into();
        let sacred_out = root.join("sacred-id");
        let sacred = run(
            LLAMAFACTORY_QLORA_ID,
            &sacred_pack,
            &estate,
            &sacred_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(sacred.to_string().contains("refuse:sacred"), "{sacred}");
        assert!(!sacred_out.exists());

        let sacred_base = with_train_base(seated.clone(), "cyera/Mistral-7B-Instruct-v0.3");
        let sacred_base_out = root.join("sacred-base");
        let sacred_train = run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &sacred_base,
            &sacred_base_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(
            sacred_train.to_string().contains("refuse:sacred"),
            "{sacred_train}"
        );
        assert!(!sacred_base_out.exists());

        let sku_base = with_train_base(
            seated.clone(),
            "/tmp/cell-one-hf/5090/Mistral-7B-Instruct-v0.3",
        );
        let sku_out = root.join("sku-base");
        let sku = run(
            LLAMAFACTORY_QLORA_ID,
            &bare,
            &sku_base,
            &sku_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(sku.to_string().contains("refuse:sku-banned"), "{sku}");
        assert!(!sku_out.exists());

        let mut local_only = estate.clone();
        local_only
            .model_bindings
            .retain(|binding| binding.class != ModelClass::Frontier);
        let frontier_state = root.join("frontier-cell");
        std::fs::create_dir_all(frontier_state.join("feed")).unwrap();
        std::fs::write(
            frontier_state.join("feed/events.jsonl"),
            "{\"kind\":\"model.frontier.complete\",\"object_class\":\"frontier\",\"note\":\"bytes=4\",\"ts\":\"2026-09-21T00:00:00Z\"}\n",
        )
        .unwrap();
        let frontier_out = root.join("frontier");
        let frontier = run_feed(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &local_only,
            &frontier_out,
            &frontier_state,
            true,
        )
        .unwrap_err();
        assert!(
            frontier.to_string().contains("refuse:frontier-invent"),
            "{frontier}"
        );
        assert!(!frontier_out.exists());
    }

    #[test]
    fn llamafactory_lora_prepares_unquantized_and_keeps_the_seat_split() {
        let root = tmp("llamafactory-lora");
        let pack = fixture_pack();
        let seated = seated_estate("llama3");
        let blocked = root.join("seat-only");
        let err = run(
            LLAMAFACTORY_LORA_ID,
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

        let leaf = with_train_base(seated.clone(), "./llama3");
        let leaf_out = root.join("leaf");
        let err = run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &leaf,
            &leaf_out,
            "train",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(err.to_string().contains("Ollama seat tag"), "{err}");
        assert!(!leaf_out.exists());

        let estate = with_train_base(seated.clone(), "Qwen/Qwen2.5-0.5B-Instruct");
        let out = root.join("recipe");
        let doc = run(LLAMAFACTORY_LORA_ID, &pack, &estate, &out, "train", "jason").unwrap();
        assert_eq!(doc.driver, LLAMAFACTORY_LORA_ID);
        assert_eq!(doc.job, "train");
        assert_eq!(doc.dataset_mode.as_deref(), Some("scaffold"));
        assert_eq!(doc.dataset_from_feed, Some(false));
        assert_eq!(doc.base_model, "llama3");
        assert_eq!(doc.seat_tag.as_deref(), Some("llama3"));
        assert_eq!(
            doc.train_base_model.as_deref(),
            Some("Qwen/Qwen2.5-0.5B-Instruct")
        );
        let recipe = std::fs::read_to_string(out.join("recipe.yaml")).unwrap();
        assert!(recipe.contains("# driver: llamafactory-lora"), "{recipe}");
        assert!(recipe.contains("finetuning_type: lora"), "{recipe}");
        assert!(
            recipe.lines().any(|line| line.trim() == "lora_rank: 8"),
            "{recipe}"
        );
        assert!(
            recipe.lines().any(|line| line.trim() == "lora_alpha: 16"),
            "{recipe}"
        );
        assert!(
            recipe.lines().any(|line| line.trim() == "packing: false"),
            "{recipe}"
        );
        assert!(
            recipe.lines().any(|line| line.trim() == "cutoff_len: 512"),
            "{recipe}"
        );
        assert!(
            recipe.lines().any(|line| line.trim() == "template: qwen"),
            "{recipe}"
        );
        assert!(
            !recipe.contains("quantization_bit") && !recipe.contains("quantization_method"),
            "{recipe}"
        );
        assert!(
            recipe.contains("model_name_or_path: \"Qwen/Qwen2.5-0.5B-Instruct\""),
            "{recipe}"
        );
        assert!(
            !recipe.contains("model_name_or_path: \"llama3\""),
            "{recipe}"
        );
        let export = std::fs::read_to_string(out.join("export.yaml")).unwrap();
        assert!(!export.contains("quantization_bit"), "{export}");
        assert!(!export.contains("quantization_method"), "{export}");
        assert!(
            export.lines().any(|line| line.trim() == "template: qwen"),
            "{export}"
        );
        assert!(export.contains("finetuning_type: lora"), "{export}");
        assert!(export.contains("# driver: llamafactory-lora"), "{export}");
        let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
        assert!(
            next.contains(&format!(
                "llamafactory-cli train {}",
                out.join("recipe.yaml").display()
            )),
            "{next}"
        );
        assert!(
            next.contains(&format!(
                "llamafactory-cli export {}",
                out.join("export.yaml").display()
            )),
            "{next}"
        );
        assert!(next.contains("pip install llamafactory"), "{next}");
        assert!(next.contains("does not require bitsandbytes"), "{next}");
        assert!(!next.contains("bitsandbytes>=0.49"), "{next}");
        assert!(!next.contains("2.11.0+cu128"), "{next}");
        assert!(
            next.contains("examples/train_lora/qwen3_lora_sft.yaml"),
            "{next}"
        );
        assert!(next.contains("`cutoff_len` 2048"), "{next}");
        assert!(next.contains("Seat tag is llama3"), "{next}");
        assert!(next.contains("Qwen/Qwen2.5-0.5B-Instruct"), "{next}");
        assert!(
            next.contains("Do not set quantization_bit on export.yaml"),
            "{next}"
        );
        assert!(next.contains("import-trained"), "{next}");
        assert!(next.contains(&format!(
            "estate enrich import-trained --estate <estate.yaml> --prepared {} --tag cell-enrich-overnight-traces --adapter {}",
            out.display(),
            out.join("outputs").display()
        )), "{next}");
        assert!(next.contains(&format!(
            "estate enrich import-trained --estate <estate.yaml> --prepared {} --tag cell-enrich-overnight-traces --adapter {}",
            out.display(),
            out.join("export").display()
        )), "{next}");
        assert!(next.contains("--adapter <gguf>"), "{next}");
        assert!(next.contains("dataset_mode: scaffold"), "{next}");
        assert!(next.contains("not training data"), "{next}");
        assert!(next.contains("--from-feed"), "{next}");
        assert!(next.contains("refuse:dataset"), "{next}");
        let prepare_md = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
        assert!(
            prepare_md.contains("omits quantization_bit"),
            "{prepare_md}"
        );
        assert!(
            prepare_md.contains("did not run llamafactory-cli"),
            "{prepare_md}"
        );
        assert!(
            prepare_md.contains("does not require bitsandbytes"),
            "{prepare_md}"
        );
        assert!(!prepare_md.contains("bitsandbytes>=0.49"), "{prepare_md}");
        assert!(
            prepare_md.contains("dataset_mode: scaffold"),
            "{prepare_md}"
        );
        assert!(prepare_md.contains("not training data"), "{prepare_md}");
        assert!(prepare_md.contains("--from-feed"), "{prepare_md}");

        let enrich_out = root.join("enrich-job");
        let err = run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &estate,
            &enrich_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");
        assert!(!enrich_out.exists());

        let instruct = with_train_base(seated.clone(), "Qwen/Qwen3-4B-Instruct-2507");
        let instruct_out = root.join("instruct-2507");
        run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &instruct,
            &instruct_out,
            "train",
            "jason",
        )
        .unwrap();
        let instruct_recipe = std::fs::read_to_string(instruct_out.join("recipe.yaml")).unwrap();
        assert!(
            instruct_recipe
                .lines()
                .any(|line| line.trim() == "template: qwen3_nothink"),
            "{instruct_recipe}"
        );
        assert!(
            !instruct_recipe
                .lines()
                .any(|line| line.trim() == "template: qwen3" || line.trim() == "template: qwen"),
            "{instruct_recipe}"
        );
        assert!(
            !instruct_recipe.contains("quantization_bit")
                && !instruct_recipe.contains("quantization_method"),
            "{instruct_recipe}"
        );
        let instruct_export = std::fs::read_to_string(instruct_out.join("export.yaml")).unwrap();
        assert!(
            instruct_export
                .lines()
                .any(|line| line.trim() == "template: qwen3_nothink"),
            "{instruct_export}"
        );
        assert!(
            !instruct_export.contains("quantization_bit"),
            "{instruct_export}"
        );

        let qlora_out = root.join("qlora-same-base");
        run(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &instruct,
            &qlora_out,
            "train",
            "jason",
        )
        .unwrap();
        let qlora_recipe = std::fs::read_to_string(qlora_out.join("recipe.yaml")).unwrap();
        assert!(
            qlora_recipe
                .lines()
                .any(|line| line.trim() == "template: qwen3_nothink"),
            "{qlora_recipe}"
        );
        assert!(
            qlora_recipe.contains("quantization_bit: 4"),
            "{qlora_recipe}"
        );
        assert!(
            qlora_recipe.contains("quantization_method: bnb"),
            "{qlora_recipe}"
        );
        assert!(
            qlora_recipe
                .lines()
                .any(|line| line.trim() == "lora_rank: 16"),
            "{qlora_recipe}"
        );
        let qlora_next = std::fs::read_to_string(qlora_out.join("NEXT.md")).unwrap();
        assert!(qlora_next.contains("bitsandbytes>=0.49"), "{qlora_next}");

        let hybrid = with_train_base(seated.clone(), "Qwen/Qwen3-4B");
        let hybrid_out = root.join("hybrid");
        run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &hybrid,
            &hybrid_out,
            "train",
            "jason",
        )
        .unwrap();
        let hybrid_recipe = std::fs::read_to_string(hybrid_out.join("recipe.yaml")).unwrap();
        assert!(
            hybrid_recipe
                .lines()
                .any(|line| line.trim() == "template: qwen3"),
            "{hybrid_recipe}"
        );

        let nested = with_train_base(seated.clone(), "/tmp/hf/Qwen3-4B-Instruct-2507/weights");
        let nested_out = root.join("nested-weights");
        run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &nested,
            &nested_out,
            "train",
            "jason",
        )
        .unwrap();
        let nested_recipe = std::fs::read_to_string(nested_out.join("recipe.yaml")).unwrap();
        assert!(
            nested_recipe
                .lines()
                .any(|line| line.trim() == "template: qwen3_nothink"),
            "{nested_recipe}"
        );
        assert!(
            !nested_recipe
                .lines()
                .any(|line| line.trim() == "template: default"),
            "{nested_recipe}"
        );
        let nested_export = std::fs::read_to_string(nested_out.join("export.yaml")).unwrap();
        assert!(
            nested_export
                .lines()
                .any(|line| line.trim() == "template: qwen3_nothink"),
            "{nested_export}"
        );

        let snapshot = with_train_base(
            seated.clone(),
            "/home/user/.cache/huggingface/hub/models--Qwen--Qwen3-4B-Instruct-2507/snapshots/a1b2c3d4e5f6",
        );
        let snapshot_out = root.join("snapshot-hash");
        run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &snapshot,
            &snapshot_out,
            "train",
            "jason",
        )
        .unwrap();
        let snapshot_recipe = std::fs::read_to_string(snapshot_out.join("recipe.yaml")).unwrap();
        assert!(
            snapshot_recipe
                .lines()
                .any(|line| line.trim() == "template: qwen3_nothink"),
            "{snapshot_recipe}"
        );
        assert!(
            !snapshot_recipe
                .lines()
                .any(|line| line.trim() == "template: default"),
            "{snapshot_recipe}"
        );

        let gauge_out = root.join("gauge");
        run_max(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &estate,
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
        assert!(
            !gauge.contains("quantization_bit") && !gauge.contains("quantization_method"),
            "{gauge}"
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
        assert_eq!(proposal.driver, LLAMAFACTORY_LORA_ID);
        assert_eq!(proposal.binding_id, "local_slm");
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
    }

    #[test]
    fn official_scale_and_export_merge_honesty() {
        let root = tmp("official-scale");
        let pack = fixture_pack();
        let estate = with_train_base(seated_estate("llama3"), "Qwen/Qwen2.5-0.5B-Instruct");

        let smoke = root.join("smoke-lora");
        run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &estate,
            &smoke,
            "train",
            "jason",
        )
        .unwrap();
        let smoke_recipe = std::fs::read_to_string(smoke.join("recipe.yaml")).unwrap();
        for line in [
            "cutoff_len: 512",
            "num_train_epochs: 1.0",
            "gradient_accumulation_steps: 4",
            "warmup_ratio: 0.03",
            "lora_rank: 8",
            "packing: false",
        ] {
            assert!(
                smoke_recipe.lines().any(|row| row.trim() == line),
                "{line} missing from {smoke_recipe}"
            );
        }
        assert!(
            !smoke_recipe.contains("quantization_bit")
                && !smoke_recipe.contains("quantization_method"),
            "{smoke_recipe}"
        );
        let smoke_export = std::fs::read_to_string(smoke.join("export.yaml")).unwrap();
        assert!(
            smoke_export.contains("# merge_status: not_run"),
            "{smoke_export}"
        );
        assert!(
            smoke_export.contains("This prepare did not merge"),
            "{smoke_export}"
        );
        assert!(!smoke_export.contains("quantization_bit"), "{smoke_export}");
        assert!(
            !smoke_export.contains("quantization_method"),
            "{smoke_export}"
        );
        assert_eq!(
            yaml_field(&smoke_recipe, "output_dir").as_deref(),
            yaml_field(&smoke_export, "adapter_name_or_path").as_deref()
        );
        let smoke_next = std::fs::read_to_string(smoke.join("NEXT.md")).unwrap();
        assert!(
            smoke_next.contains("This prepare did not merge"),
            "{smoke_next}"
        );
        assert!(
            smoke_next.contains("The merge has not happened."),
            "{smoke_next}"
        );
        assert!(
            smoke_next.contains("Do not set quantization_bit on export.yaml"),
            "{smoke_next}"
        );
        assert!(
            smoke_next.contains("does not rewrite export.yaml after train"),
            "{smoke_next}"
        );
        let export_dir = yaml_field(&smoke_export, "export_dir").unwrap();
        assert!(smoke_next.contains(&export_dir), "{smoke_next}");
        assert!(export_dir.ends_with("/export"), "{export_dir}");
        let smoke_prepare = std::fs::read_to_string(smoke.join("PREPARE.md")).unwrap();
        assert!(
            smoke_prepare.contains("This prepare did not merge"),
            "{smoke_prepare}"
        );
        assert!(
            smoke_prepare.contains("llamafactory-cli export has not run"),
            "{smoke_prepare}"
        );

        let official = root.join("official-lora");
        prepare_enrich(&PrepareEnrichRequest {
            estate: &estate,
            pack: &pack,
            curator: "jason",
            driver_id: LLAMAFACTORY_LORA_ID,
            job: "train",
            out_dir: &official,
            max_steps: None,
            official_scale: true,
            from_feed: false,
            state_dir: Path::new(".cell"),
        })
        .unwrap();
        let official_recipe = std::fs::read_to_string(official.join("recipe.yaml")).unwrap();
        for line in [
            "cutoff_len: 2048",
            "num_train_epochs: 3.0",
            "gradient_accumulation_steps: 8",
            "warmup_ratio: 0.1",
            "lora_rank: 8",
            "packing: false",
        ] {
            assert!(
                official_recipe.lines().any(|row| row.trim() == line),
                "{line} missing from {official_recipe}"
            );
        }
        assert!(
            official_recipe.contains("examples/train_lora/qwen3_lora_sft.yaml"),
            "{official_recipe}"
        );
        assert!(
            !official_recipe
                .lines()
                .any(|row| row.trim_start().starts_with("max_steps:")),
            "{official_recipe}"
        );
        assert!(
            !official_recipe.contains("quantization_bit")
                && !official_recipe.contains("quantization_method"),
            "{official_recipe}"
        );
        let official_next = std::fs::read_to_string(official.join("NEXT.md")).unwrap();
        assert!(
            official_next.contains("This prepare used `--official-scale`"),
            "{official_next}"
        );
        assert!(
            official_next.contains("`cutoff_len` is 2048"),
            "{official_next}"
        );

        let qlora = root.join("official-qlora");
        prepare_enrich(&PrepareEnrichRequest {
            estate: &estate,
            pack: &pack,
            curator: "jason",
            driver_id: LLAMAFACTORY_QLORA_ID,
            job: "train",
            out_dir: &qlora,
            max_steps: None,
            official_scale: true,
            from_feed: false,
            state_dir: Path::new(".cell"),
        })
        .unwrap();
        let qlora_recipe = std::fs::read_to_string(qlora.join("recipe.yaml")).unwrap();
        assert!(
            qlora_recipe
                .lines()
                .any(|row| row.trim() == "cutoff_len: 2048"),
            "{qlora_recipe}"
        );
        assert!(
            qlora_recipe
                .lines()
                .any(|row| row.trim() == "num_train_epochs: 3.0"),
            "{qlora_recipe}"
        );
        assert!(
            qlora_recipe.contains("quantization_bit: 4"),
            "{qlora_recipe}"
        );
        assert!(
            qlora_recipe.contains("quantization_method: bnb"),
            "{qlora_recipe}"
        );
        assert!(
            qlora_recipe
                .lines()
                .any(|row| row.trim() == "lora_rank: 16"),
            "{qlora_recipe}"
        );
        assert!(
            qlora_recipe
                .lines()
                .any(|row| row.trim() == "packing: true"),
            "{qlora_recipe}"
        );
        let qlora_export = std::fs::read_to_string(qlora.join("export.yaml")).unwrap();
        assert!(!qlora_export.contains("quantization_bit"), "{qlora_export}");

        let gauge = root.join("official-gauge");
        prepare_enrich(&PrepareEnrichRequest {
            estate: &estate,
            pack: &pack,
            curator: "jason",
            driver_id: LLAMAFACTORY_LORA_ID,
            job: "train",
            out_dir: &gauge,
            max_steps: Some(10),
            official_scale: true,
            from_feed: false,
            state_dir: Path::new(".cell"),
        })
        .unwrap();
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
                .any(|row| row.trim() == "save_steps: 10"),
            "{gauge_recipe}"
        );
        assert!(
            gauge_recipe
                .lines()
                .any(|row| row.trim() == "cutoff_len: 2048"),
            "{gauge_recipe}"
        );
        assert!(
            gauge_recipe
                .lines()
                .any(|row| row.trim() == "num_train_epochs: 3.0"),
            "{gauge_recipe}"
        );
        assert!(
            !gauge_recipe.contains("quantization_bit")
                && !gauge_recipe.contains("quantization_method"),
            "{gauge_recipe}"
        );
        let gauge_next = std::fs::read_to_string(gauge.join("NEXT.md")).unwrap();
        assert!(
            gauge_next.contains("gauge run with max_steps 10"),
            "{gauge_next}"
        );

        let ax = root.join("axolotl-official");
        prepare_enrich(&PrepareEnrichRequest {
            estate: &estate,
            pack: &pack,
            curator: "jason",
            driver_id: AXOLOTL_LORA_ID,
            job: "train",
            out_dir: &ax,
            max_steps: None,
            official_scale: true,
            from_feed: false,
            state_dir: Path::new(".cell"),
        })
        .unwrap();
        let ax_yaml = std::fs::read_to_string(ax.join("axolotl.yml")).unwrap();
        assert!(
            ax_yaml.lines().any(|row| row.trim() == "adapter: lora"),
            "{ax_yaml}"
        );
        assert!(
            ax_yaml
                .lines()
                .any(|row| row.trim() == "load_in_4bit: false"),
            "{ax_yaml}"
        );
        assert!(
            ax_yaml.lines().any(|row| row.trim() == "num_epochs: 1"),
            "{ax_yaml}"
        );
        assert!(
            ax_yaml
                .lines()
                .any(|row| row.trim() == "gradient_accumulation_steps: 2"),
            "{ax_yaml}"
        );
        assert!(
            ax_yaml
                .lines()
                .any(|row| row.trim() == "sequence_len: 2048"),
            "{ax_yaml}"
        );
        assert!(
            ax_yaml.lines().any(|row| row.trim() == "warmup_ratio: 0.1"),
            "{ax_yaml}"
        );
        let ax_next = std::fs::read_to_string(ax.join("NEXT.md")).unwrap();
        assert!(
            ax_next.contains("This card stays on examples/llama-3/lora-1b.yml"),
            "{ax_next}"
        );
        let ax_q = root.join("axolotl-qlora-official");
        prepare_enrich(&PrepareEnrichRequest {
            estate: &estate,
            pack: &pack,
            curator: "jason",
            driver_id: AXOLOTL_QLORA_ID,
            job: "train",
            out_dir: &ax_q,
            max_steps: None,
            official_scale: true,
            from_feed: false,
            state_dir: Path::new(".cell"),
        })
        .unwrap();
        let ax_q_yaml = std::fs::read_to_string(ax_q.join("axolotl.yml")).unwrap();
        assert!(
            ax_q_yaml.lines().any(|row| row.trim() == "adapter: qlora"),
            "{ax_q_yaml}"
        );
        assert!(
            ax_q_yaml
                .lines()
                .any(|row| row.trim() == "load_in_4bit: true"),
            "{ax_q_yaml}"
        );
        assert!(
            ax_q_yaml
                .lines()
                .any(|row| row.trim() == "sequence_len: 4096"),
            "{ax_q_yaml}"
        );
        assert!(
            ax_q_yaml.lines().any(|row| row.trim() == "num_epochs: 4"),
            "{ax_q_yaml}"
        );
        let ax_default = root.join("axolotl-default");
        run(
            AXOLOTL_LORA_ID,
            &pack,
            &estate,
            &ax_default,
            "train",
            "jason",
        )
        .unwrap();
        let ax_default_yaml = std::fs::read_to_string(ax_default.join("axolotl.yml")).unwrap();
        assert!(
            ax_default_yaml
                .lines()
                .any(|row| row.trim() == "num_epochs: 1"),
            "{ax_default_yaml}"
        );
        assert!(
            ax_default_yaml
                .lines()
                .any(|row| row.trim() == "gradient_accumulation_steps: 2"),
            "{ax_default_yaml}"
        );
        assert!(
            ax_default_yaml
                .lines()
                .any(|row| row.trim() == "warmup_ratio: 0.1"),
            "{ax_default_yaml}"
        );
        assert!(
            ax_default_yaml
                .lines()
                .any(|row| row.trim() == "adapter: lora"),
            "{ax_default_yaml}"
        );
        assert!(
            !ax_default_yaml.contains("Official scale"),
            "{ax_default_yaml}"
        );

        let state = root.join("cell");
        write_fixture_feed(&state);
        let fed = root.join("feed-official");
        prepare_enrich(&PrepareEnrichRequest {
            estate: &estate,
            pack: &pack,
            curator: "jason",
            driver_id: LLAMAFACTORY_LORA_ID,
            job: "train",
            out_dir: &fed,
            max_steps: None,
            official_scale: true,
            from_feed: true,
            state_dir: &state,
        })
        .unwrap();
        let fed_doc = std::fs::read_to_string(fed.join("prepare.json")).unwrap();
        assert!(fed_doc.contains("\"dataset_mode\": \"feed\""), "{fed_doc}");
        let fed_recipe = std::fs::read_to_string(fed.join("recipe.yaml")).unwrap();
        assert!(
            fed_recipe
                .lines()
                .any(|row| row.trim() == "cutoff_len: 2048"),
            "{fed_recipe}"
        );
        assert!(
            !fed_recipe.contains("quantization_bit") && !fed_recipe.contains("quantization_method"),
            "{fed_recipe}"
        );

        let blocked = root.join("ollama-official");
        let err = prepare_enrich(&PrepareEnrichRequest {
            estate: &estate,
            pack: &pack,
            curator: "jason",
            driver_id: "ollama-modelfile",
            job: "enrich",
            out_dir: &blocked,
            max_steps: None,
            official_scale: true,
            from_feed: false,
            state_dir: Path::new(".cell"),
        })
        .unwrap_err();
        assert!(err.to_string().contains("refuse:official-scale"), "{err}");
        assert!(err.to_string().contains("axolotl-qlora"), "{err}");
        assert!(!blocked.exists());

        let host = enrich_host_class_affinity(&estate, &pack);
        let drivers = train_enrich_drivers_for_prepare("train", &host).unwrap();
        assert!(!drivers.contains(&MLX_LM_LORA_ID), "{host}");
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
                official_scale: true,
                from_feed: false,
                state_dir: Path::new(".cell"),
            })
            .collect();
        let all_docs = prepare_enrich_set(&all_reqs).unwrap();
        assert_eq!(all_docs.len(), 7);
        let all_unsloth =
            std::fs::read_to_string(root.join("all-unsloth-qlora").join("UNSLOTH.md")).unwrap();
        assert!(
            all_unsloth.contains("does not implement official scale"),
            "{all_unsloth}"
        );
        assert!(!root
            .join("all-unsloth-qlora")
            .join("train_unsloth.py")
            .exists());
        let all_modelfile =
            std::fs::read_to_string(root.join("all-ollama-modelfile").join("Modelfile")).unwrap();
        assert!(all_modelfile.contains("FROM llama3\n"), "{all_modelfile}");
        let all_lora =
            std::fs::read_to_string(root.join("all-llamafactory-lora").join("recipe.yaml"))
                .unwrap();
        assert!(
            all_lora.lines().any(|row| row.trim() == "cutoff_len: 2048"),
            "{all_lora}"
        );
        assert!(
            !all_lora.contains("quantization_bit") && !all_lora.contains("quantization_method"),
            "{all_lora}"
        );
        let all_ax =
            std::fs::read_to_string(root.join("all-axolotl-lora").join("axolotl.yml")).unwrap();
        assert!(
            all_ax.lines().any(|row| row.trim() == "adapter: lora"),
            "{all_ax}"
        );
        assert!(
            all_ax.lines().any(|row| row.trim() == "num_epochs: 1"),
            "{all_ax}"
        );
        let all_axq =
            std::fs::read_to_string(root.join("all-axolotl-qlora").join("axolotl.yml")).unwrap();
        assert!(
            all_axq.lines().any(|row| row.trim() == "adapter: qlora"),
            "{all_axq}"
        );
        assert!(
            all_axq
                .lines()
                .any(|row| row.trim() == "sequence_len: 4096"),
            "{all_axq}"
        );

        let adapter = root.join("adapter");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{}\n").unwrap();
        let export_path = official.join("export.yaml");
        let original_export = std::fs::read_to_string(&export_path).unwrap();
        let checkpoint = official.join("outputs").join("checkpoint-10");
        let retargeted = original_export.replace(
            &format!(
                "adapter_name_or_path: \"{}\"",
                official.join("outputs").display()
            ),
            &format!("adapter_name_or_path: \"{}\"", checkpoint.display()),
        );
        assert_ne!(retargeted, original_export);
        std::fs::write(&export_path, &retargeted).unwrap();
        import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &official,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap();

        let commented = format!("# quantization_bit: 4\n{original_export}");
        std::fs::write(&export_path, &commented).unwrap();
        import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &official,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap();

        std::fs::write(
            &export_path,
            format!("{original_export}quantization_bit: 4\n"),
        )
        .unwrap();
        let quant = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &official,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap_err();
        assert!(quant.to_string().contains("refuse:export"), "{quant}");
        assert!(quant.to_string().contains("quantization_bit"), "{quant}");
        assert!(
            quant.to_string().contains("This prepare did not merge"),
            "{quant}"
        );

        std::fs::write(
            &export_path,
            format!("{original_export}quantization_method: bnb\n"),
        )
        .unwrap();
        let method = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &official,
            tag: "cell-enrich-overnight-traces",
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap_err();
        assert!(method.to_string().contains("refuse:export"), "{method}");
        assert!(
            method.to_string().contains("quantization_method"),
            "{method}"
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
        assert!(yaml_line(&yaml, "adapter: lora"), "{yaml}");
        assert!(yaml_line(&yaml, "load_in_8bit: false"), "{yaml}");
        assert!(yaml_line(&yaml, "load_in_4bit: false"), "{yaml}");
        assert!(yaml_line(&yaml, "sequence_len: 2048"), "{yaml}");
        assert!(yaml_line(&yaml, "micro_batch_size: 2"), "{yaml}");
        assert!(yaml_line(&yaml, "gradient_accumulation_steps: 2"), "{yaml}");
        assert!(yaml_line(&yaml, "lora_r: 16"), "{yaml}");
        assert!(yaml_line(&yaml, "lora_alpha: 32"), "{yaml}");
        assert!(yaml_line(&yaml, "sample_packing: true"), "{yaml}");
        assert!(yaml_line(&yaml, "saves_per_epoch: 1"), "{yaml}");
        assert!(
            !yaml
                .lines()
                .any(|line| line.trim_start().starts_with("max_steps:")),
            "{yaml}"
        );
        assert!(yaml.contains("examples/llama-3/lora-1b.yml"), "{yaml}");
        assert!(!yaml.contains("adapter: qlora"), "{yaml}");
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
        let outputs = out.join("outputs");
        let merged = outputs.join("merged");
        assert!(
            next.contains(&format!(
                "estate enrich merge-adapt --prepared {} --adapter {}",
                out.display(),
                outputs.display()
            )),
            "{next}"
        );
        assert!(
            next.contains(&format!(
                "axolotl merge-lora {} --lora-model-dir={}",
                config.display(),
                outputs.display()
            )),
            "{next}"
        );
        assert!(!next.contains("--dequant"), "{next}");
        assert!(next.contains("estate enrich gguf-convert"), "{next}");
        assert!(next.contains("--outtype auto"), "{next}");
        assert!(
            next.contains(&format!(
                "estate enrich gguf-convert --prepared {} --weights {}",
                out.display(),
                merged.display()
            )),
            "{next}"
        );
        assert!(
            next.contains(&format!(
                "python3 convert_hf_to_gguf.py {} --outfile {} --outtype auto",
                merged.display(),
                outputs.join("merged.gguf").display()
            )),
            "{next}"
        );
        assert!(next.contains("estate enrich local-seat"), "{next}");
        assert!(next.contains("Axolotl does not write GGUF"), "{next}");
        assert!(next.contains("This prepare did not merge"), "{next}");
        assert!(!next.contains("<merged-hf-dir>"), "{next}");
        assert!(next.contains("READY_FOR_LIVE_TEST: no"), "{next}");
        assert!(!next.contains("READY_FOR_LIVE_TEST: yes"), "{next}");
        assert!(next.contains("unslothai/unsloth"), "{next}");
        assert!(next.contains("mlx-lm-lora"), "{next}");
        assert!(next.contains("mlx_lm/LORA.md"), "{next}");
        assert!(next.contains("does not write an MLX trainer"), "{next}");
        assert!(!yaml.contains("mlx-lm-lora"), "{yaml}");
        assert!(!yaml.contains("mlx_lm"), "{yaml}");
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

        let host = enrich_host_class_affinity(&estate, &pack);
        let drivers = train_enrich_drivers_for_prepare("train", &host).unwrap();
        assert!(!drivers.contains(&MLX_LM_LORA_ID), "{host}");
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
                official_scale: false,
                from_feed: false,
                state_dir: Path::new(".cell"),
            })
            .collect();
        let all_docs = prepare_enrich_set(&all_reqs).unwrap();
        assert_eq!(all_docs.len(), 7);
        assert!(root.join("all-unsloth-qlora").join("UNSLOTH.md").is_file());
        assert!(!root
            .join("all-unsloth-qlora")
            .join("dataset.jsonl")
            .exists());
        let all_lora =
            std::fs::read_to_string(root.join("all-llamafactory-lora").join("recipe.yaml"))
                .unwrap();
        assert!(all_lora.contains("finetuning_type: lora"), "{all_lora}");
        assert!(
            all_lora.lines().any(|line| line.trim() == "lora_rank: 8"),
            "{all_lora}"
        );
        assert!(
            all_lora.lines().any(|line| line.trim() == "packing: false"),
            "{all_lora}"
        );
        assert!(
            !all_lora.contains("quantization_bit") && !all_lora.contains("quantization_method"),
            "{all_lora}"
        );
        assert!(
            all_lora.lines().any(|line| line.trim() == "template: qwen"),
            "{all_lora}"
        );
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
                official_scale: false,
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
        assert!(
            prepare_md.contains("estate enrich gguf-convert"),
            "{prepare_md}"
        );
        assert!(
            prepare_md.contains("Axolotl does not write GGUF"),
            "{prepare_md}"
        );
        assert!(
            prepare_md.contains("This prepare did not merge"),
            "{prepare_md}"
        );
        assert!(
            prepare_md.contains("estate enrich merge-adapt"),
            "{prepare_md}"
        );
        assert!(prepare_md.contains("axolotl merge-lora"), "{prepare_md}");
        assert!(!prepare_md.contains("--dequant"), "{prepare_md}");

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
        assert!(
            stub_prepare.contains("dataset_mode: stub"),
            "{stub_prepare}"
        );
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
    fn import_trained_records_three_shapes_and_refuses_garbage() {
        let root = tmp("trained-shapes");
        let pack = fixture_pack();
        let estate = with_train_base(seated_estate("llama3"), "Qwen/Qwen2.5-0.5B-Instruct");
        let out = root.join("qlora");
        run(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &estate,
            &out,
            "train",
            "jason",
        )
        .unwrap();
        let lora_out = root.join("lora");
        run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &estate,
            &lora_out,
            "train",
            "jason",
        )
        .unwrap();
        let tag = "cell-enrich-overnight-traces";
        for (dir, driver) in [
            (&out, LLAMAFACTORY_QLORA_ID),
            (&lora_out, LLAMAFACTORY_LORA_ID),
        ] {
            let next = std::fs::read_to_string(dir.join("NEXT.md")).unwrap();
            for adapter in [dir.join("outputs"), dir.join("export")] {
                assert!(
                    next.contains(&format!(
                        "estate enrich import-trained --estate <estate.yaml> --prepared {} --tag {tag} --adapter {}",
                        dir.display(),
                        adapter.display()
                    )),
                    "{driver} {next}"
                );
            }
            assert!(
                next.contains(&format!(
                    "estate enrich import-trained --estate <estate.yaml> --prepared {} --tag {tag} --adapter <gguf>",
                    dir.display()
                )),
                "{driver} {next}"
            );
        }

        let curator = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &root.join("missing"),
            curator: "ada",
        })
        .unwrap_err();
        assert!(curator.to_string().contains("refuse:curator"), "{curator}");

        let sacred_dir = root.join("cyera");
        std::fs::create_dir_all(&sacred_dir).unwrap();
        std::fs::write(sacred_dir.join("adapter_config.json"), "{}\n").unwrap();
        let sacred = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &sacred_dir,
            curator: "jason",
        })
        .unwrap_err();
        assert!(sacred.to_string().contains("refuse:sacred"), "{sacred}");

        let garbage = root.join("garbage.txt");
        std::fs::write(&garbage, "not weights\n").unwrap();
        let garbage_err = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &garbage,
            curator: "jason",
        })
        .unwrap_err();
        let garbage_text = garbage_err.to_string();
        assert!(garbage_text.contains("refuse:adapter"), "{garbage_text}");
        assert!(
            garbage_text.contains("is a file and is not a GGUF"),
            "{garbage_text}"
        );
        assert!(
            garbage_text.contains("adapter_config.json"),
            "{garbage_text}"
        );
        assert!(garbage_text.contains("config.json"), "{garbage_text}");
        assert!(garbage_text.contains(".safetensors"), "{garbage_text}");
        assert!(garbage_text.contains(".gguf"), "{garbage_text}");
        assert!(!out.join("binding-proposal.json").is_file());
        let untouched = std::fs::read_to_string(out.join("prepare.json")).unwrap();
        assert!(!untouched.contains("trained_shape"), "{untouched}");
        assert!(untouched.contains("\"promoted\": false"), "{untouched}");
        assert!(untouched.contains("\"auto_apply\": false"), "{untouched}");
        assert!(
            untouched.contains("\"estate_rewritten\": false"),
            "{untouched}"
        );

        let empty = root.join("empty-dir");
        std::fs::create_dir_all(&empty).unwrap();
        let empty_err = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &empty,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            empty_err
                .to_string()
                .contains("is not an adapter output_dir, a merged export_dir, or a GGUF path"),
            "{empty_err}"
        );

        let half = root.join("half-export");
        std::fs::create_dir_all(&half).unwrap();
        std::fs::write(half.join("config.json"), "{}\n").unwrap();
        let half_err = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &half,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            half_err.to_string().contains("no .safetensors"),
            "{half_err}"
        );

        let lone = root.join("model.safetensors");
        std::fs::write(&lone, "weights").unwrap();
        let lone_err = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &lone,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            lone_err.to_string().contains("is a file and is not a GGUF"),
            "{lone_err}"
        );
        assert!(!out.join("binding-proposal.json").is_file());

        let adapter = root.join("outputs");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("adapter_config.json"), "{\"r\":8}\n").unwrap();
        std::fs::write(adapter.join("adapter_model.safetensors"), "weights").unwrap();
        let proposal = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &adapter,
            curator: "jason",
        })
        .unwrap();
        assert_eq!(proposal.trained_shape.as_deref(), Some("adapter"));
        assert_eq!(proposal.driver, LLAMAFACTORY_QLORA_ID);
        assert_eq!(proposal.curator, "jason");
        assert_eq!(proposal.policy, "manual");
        assert!(!proposal.auto_apply && !proposal.promoted && !proposal.estate_rewritten);
        assert!(proposal.local_path.ends_with("adapter_config.json"));
        let adapter_paths = proposal.trained_paths.clone().unwrap();
        assert_eq!(adapter_paths[0], adapter.display().to_string());
        assert!(adapter_paths
            .iter()
            .any(|path| path.ends_with("adapter_config.json")));
        assert!(adapter_paths
            .iter()
            .any(|path| path.ends_with("adapter_model.safetensors")));
        let prepare_text = std::fs::read_to_string(out.join("prepare.json")).unwrap();
        let prepare: EnrichPrepareDoc = serde_json::from_str(&prepare_text).unwrap();
        assert_eq!(prepare.trained_shape.as_deref(), Some("adapter"));
        assert_eq!(prepare.trained_paths.as_ref(), Some(&adapter_paths));
        assert!(!prepare.promoted && !prepare.auto_apply && !prepare.estate_rewritten);
        let proposal_text = std::fs::read_to_string(out.join("binding-proposal.json")).unwrap();
        let proposal_json: serde_json::Value = serde_json::from_str(&proposal_text).unwrap();
        assert_eq!(proposal_json["trained_shape"], "adapter");
        assert_eq!(proposal_json["curator"], "jason");
        assert_eq!(proposal_json["auto_apply"], false);
        assert_eq!(proposal_json["promoted"], false);
        assert_eq!(proposal_json["estate_rewritten"], false);
        let md = std::fs::read_to_string(out.join("binding-proposal.md")).unwrap();
        assert!(md.contains("trained shape: adapter"), "{md}");
        assert!(md.contains("adapter_config.json"), "{md}");
        let source = root.join("estate.yaml");
        std::fs::write(&source, estate_schema::render_estate_yaml(&estate).unwrap()).unwrap();
        let before = std::fs::read(&source).unwrap();
        let staged = apply_proposal(&ApplyProposalRequest {
            estate: &estate,
            estate_path: &source,
            prepared_dir: &out,
            tag,
            curator: "jason",
            state_dir: &root.join("state"),
            verify_endpoint: None,
        })
        .unwrap();
        assert!(matches!(staged, ApplyProposalOutcome::Staged(_)));
        assert_eq!(std::fs::read(&source).unwrap(), before);

        let export = root.join("export");
        std::fs::create_dir_all(&export).unwrap();
        std::fs::write(export.join("config.json"), "{\"model_type\":\"qwen2\"}\n").unwrap();
        std::fs::write(export.join("model.safetensors"), "merged").unwrap();
        std::fs::write(
            export.join("Modelfile"),
            "FROM cell-enrich-overnight-traces\n",
        )
        .unwrap();
        let merged = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &lora_out,
            tag,
            adapter: &export,
            curator: "jason",
        })
        .unwrap();
        assert_eq!(merged.trained_shape.as_deref(), Some("merged"));
        assert_eq!(merged.driver, LLAMAFACTORY_LORA_ID);
        assert!(merged.local_path.ends_with("config.json"));
        assert!(!merged.auto_apply && !merged.promoted && !merged.estate_rewritten);
        let merged_paths = merged.trained_paths.clone().unwrap();
        assert_eq!(merged_paths[0], export.display().to_string());
        assert!(merged_paths
            .iter()
            .any(|path| path.ends_with("config.json")));
        assert!(merged_paths
            .iter()
            .any(|path| path.ends_with("model.safetensors")));
        assert!(merged_paths.iter().any(|path| path.ends_with("Modelfile")));
        let lora_prepare: EnrichPrepareDoc =
            serde_json::from_str(&std::fs::read_to_string(lora_out.join("prepare.json")).unwrap())
                .unwrap();
        assert_eq!(lora_prepare.trained_shape.as_deref(), Some("merged"));
        assert_eq!(lora_prepare.trained_paths.as_ref(), Some(&merged_paths));
        assert!(
            !lora_prepare.promoted && !lora_prepare.auto_apply && !lora_prepare.estate_rewritten
        );
        let lora_md = std::fs::read_to_string(lora_out.join("binding-proposal.md")).unwrap();
        assert!(lora_md.contains("trained shape: merged"), "{lora_md}");

        let gguf = root.join("specialist.gguf");
        std::fs::write(&gguf, "gguf-fixture").unwrap();
        let gguf_proposal = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &gguf,
            curator: "jason",
        })
        .unwrap();
        assert_eq!(gguf_proposal.trained_shape.as_deref(), Some("gguf"));
        assert_eq!(
            gguf_proposal.trained_paths.as_ref().map(Vec::as_slice),
            Some([gguf.display().to_string()].as_slice())
        );
        assert!(gguf_proposal.local_path.ends_with("specialist.gguf"));
        assert!(
            !gguf_proposal.auto_apply && !gguf_proposal.promoted && !gguf_proposal.estate_rewritten
        );
        let gguf_prepare: EnrichPrepareDoc =
            serde_json::from_str(&std::fs::read_to_string(out.join("prepare.json")).unwrap())
                .unwrap();
        assert_eq!(gguf_prepare.trained_shape.as_deref(), Some("gguf"));
        assert_eq!(
            gguf_prepare.trained_paths.as_ref(),
            gguf_proposal.trained_paths.as_ref()
        );
        let gguf_record = std::fs::read(out.join("prepare.json")).unwrap();
        let gguf_proposal_bytes = std::fs::read(out.join("binding-proposal.json")).unwrap();

        let many = root.join("many-gguf");
        std::fs::create_dir_all(&many).unwrap();
        std::fs::write(many.join("b.gguf"), "b").unwrap();
        std::fs::write(many.join("a.gguf"), "a").unwrap();
        let many_err = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &many,
            curator: "jason",
        })
        .unwrap_err();
        let many_text = many_err.to_string();
        assert!(many_text.contains("refuse:adapter"), "{many_text}");
        assert!(
            many_text.contains("more than one top-level .gguf"),
            "{many_text}"
        );
        assert!(many_text.contains("a.gguf"), "{many_text}");
        assert!(many_text.contains("b.gguf"), "{many_text}");
        assert_eq!(
            std::fs::read(out.join("prepare.json")).unwrap(),
            gguf_record
        );
        assert_eq!(
            std::fs::read(out.join("binding-proposal.json")).unwrap(),
            gguf_proposal_bytes
        );

        let one = root.join("one-gguf");
        std::fs::create_dir_all(one.join("extra")).unwrap();
        std::fs::write(one.join("extra").join("other.gguf"), "nested").unwrap();
        std::fs::write(one.join("only.gguf"), "gguf-fixture").unwrap();
        let one_proposal = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &lora_out,
            tag,
            adapter: &one,
            curator: "jason",
        })
        .unwrap();
        assert_eq!(one_proposal.trained_shape.as_deref(), Some("gguf"));
        assert!(one_proposal.local_path.ends_with("only.gguf"));
        let one_paths = one_proposal.trained_paths.unwrap();
        assert_eq!(one_paths[0], one.display().to_string());
        assert_eq!(one_paths.len(), 2);
        assert!(one_paths.iter().any(|path| path.ends_with("only.gguf")));
        assert!(one_paths.iter().all(|path| !path.contains("other.gguf")));

        let mixed = root.join("mixed");
        std::fs::create_dir_all(&mixed).unwrap();
        std::fs::write(mixed.join("adapter_config.json"), "{}\n").unwrap();
        std::fs::write(mixed.join("config.json"), "{}\n").unwrap();
        std::fs::write(mixed.join("model.safetensors"), "merged").unwrap();
        let mixed_err = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &mixed,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            mixed_err
                .to_string()
                .contains("more than one trained shape"),
            "{mixed_err}"
        );
        let still: EnrichPrepareDoc =
            serde_json::from_str(&std::fs::read_to_string(out.join("prepare.json")).unwrap())
                .unwrap();
        assert_eq!(still.trained_shape.as_deref(), Some("gguf"));
        assert!(!still.promoted && !still.auto_apply && !still.estate_rewritten);
    }

    #[test]
    fn import_trained_refuses_adapter_weights_symlinks_and_partial_writes() {
        let root = tmp("trained-harden");
        let pack = fixture_pack();
        let estate = with_train_base(seated_estate("llama3"), "Qwen/Qwen2.5-0.5B-Instruct");
        let out = root.join("qlora");
        run(
            LLAMAFACTORY_QLORA_ID,
            &pack,
            &estate,
            &out,
            "train",
            "jason",
        )
        .unwrap();
        let lora_out = root.join("lora");
        run(
            LLAMAFACTORY_LORA_ID,
            &pack,
            &estate,
            &lora_out,
            "train",
            "jason",
        )
        .unwrap();
        let tag = "cell-enrich-overnight-traces";
        let untouched = std::fs::read(out.join("prepare.json")).unwrap();

        let disguised = root.join("disguised-export");
        std::fs::create_dir_all(&disguised).unwrap();
        std::fs::write(disguised.join("config.json"), "{}\n").unwrap();
        std::fs::write(disguised.join("adapter_model.safetensors"), "weights").unwrap();
        let disguised_err = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &disguised,
            curator: "jason",
        })
        .unwrap_err();
        let disguised_text = disguised_err.to_string();
        assert!(
            disguised_text.contains("refuse:adapter"),
            "{disguised_text}"
        );
        assert!(
            disguised_text.contains("adapter_model.safetensors is not a merged export"),
            "{disguised_text}"
        );
        assert!(!out.join("binding-proposal.json").is_file());
        assert_eq!(std::fs::read(out.join("prepare.json")).unwrap(), untouched);

        let shard = root.join("adapter-shard");
        std::fs::create_dir_all(&shard).unwrap();
        std::fs::write(shard.join("config.json"), "{}\n").unwrap();
        std::fs::write(shard.join("adapter_model-00001-of-00002.safetensors"), "w").unwrap();
        let shard_err = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &shard,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            shard_err
                .to_string()
                .contains("adapter_model.safetensors is not a merged export"),
            "{shard_err}"
        );
        assert_eq!(std::fs::read(out.join("prepare.json")).unwrap(), untouched);

        let outside = root.join("outside.safetensors");
        std::fs::write(&outside, "escaped").unwrap();
        let linked = root.join("linked-export");
        std::fs::create_dir_all(&linked).unwrap();
        std::fs::write(linked.join("config.json"), "{}\n").unwrap();
        std::os::unix::fs::symlink(&outside, linked.join("model.safetensors")).unwrap();
        let link_err = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &linked,
            curator: "jason",
        })
        .unwrap_err();
        let link_text = link_err.to_string();
        assert!(link_text.contains("refuse:adapter"), "{link_text}");
        assert!(link_text.contains("symlink"), "{link_text}");
        assert!(!out.join("binding-proposal.json").is_file());
        assert_eq!(std::fs::read(out.join("prepare.json")).unwrap(), untouched);

        let outside_cfg = root.join("outside-adapter.json");
        std::fs::write(&outside_cfg, "{}\n").unwrap();
        let linked_adapter = root.join("linked-adapter");
        std::fs::create_dir_all(&linked_adapter).unwrap();
        std::os::unix::fs::symlink(&outside_cfg, linked_adapter.join("adapter_config.json"))
            .unwrap();
        let adapter_link = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &linked_adapter,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            adapter_link.to_string().contains("symlink"),
            "{adapter_link}"
        );
        assert_eq!(std::fs::read(out.join("prepare.json")).unwrap(), untouched);

        let real_gguf = root.join("real.gguf");
        std::fs::write(&real_gguf, "gguf-fixture").unwrap();
        let link_gguf = root.join("link.gguf");
        std::os::unix::fs::symlink(&real_gguf, &link_gguf).unwrap();
        let gguf_link = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &link_gguf,
            curator: "jason",
        })
        .unwrap_err();
        assert!(gguf_link.to_string().contains("symlink"), "{gguf_link}");
        assert_eq!(std::fs::read(out.join("prepare.json")).unwrap(), untouched);

        let real_dir = root.join("real-adapter");
        std::fs::create_dir_all(&real_dir).unwrap();
        std::fs::write(real_dir.join("adapter_config.json"), "{}\n").unwrap();
        let link_dir = root.join("link-adapter");
        std::os::unix::fs::symlink(&real_dir, &link_dir).unwrap();
        let dir_link = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &link_dir,
            curator: "jason",
        })
        .unwrap_err();
        assert!(dir_link.to_string().contains("symlink"), "{dir_link}");
        assert!(!out.join("binding-proposal.json").is_file());

        std::fs::create_dir(out.join("binding-proposal.md")).unwrap();
        let partial = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &out,
            tag,
            adapter: &real_dir,
            curator: "jason",
        })
        .unwrap_err();
        let partial_text = partial.to_string();
        assert!(
            partial_text.contains("refuse:prepare-write"),
            "{partial_text}"
        );
        assert!(partial_text.contains("partial proposal"), "{partial_text}");
        assert!(!out.join("binding-proposal.json").is_file());
        assert!(!out.join("prepare.json.importing").exists());
        assert!(!out.join("binding-proposal.json.importing").exists());
        assert!(!out.join("binding-proposal.md.importing").exists());
        assert_eq!(std::fs::read(out.join("prepare.json")).unwrap(), untouched);
        let source = root.join("estate.yaml");
        std::fs::write(&source, estate_schema::render_estate_yaml(&estate).unwrap()).unwrap();
        let apply_err = apply_proposal(&ApplyProposalRequest {
            estate: &estate,
            estate_path: &source,
            prepared_dir: &out,
            tag,
            curator: "jason",
            state_dir: &root.join("state"),
            verify_endpoint: None,
        })
        .unwrap_err();
        assert!(
            apply_err.to_string().contains("refuse:missing-proposal"),
            "{apply_err}"
        );

        let export = root.join("export");
        std::fs::create_dir_all(&export).unwrap();
        std::fs::write(export.join("config.json"), "{\"model_type\":\"qwen2\"}\n").unwrap();
        std::fs::write(export.join("model.safetensors"), "merged").unwrap();
        std::fs::write(export.join("adapter_model.safetensors"), "lora").unwrap();
        std::os::unix::fs::symlink(&outside, export.join("README")).unwrap();
        let merged = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &lora_out,
            tag,
            adapter: &export,
            curator: "jason",
        })
        .unwrap();
        assert_eq!(merged.trained_shape.as_deref(), Some("merged"));
        let merged_paths = merged.trained_paths.unwrap();
        assert!(merged_paths
            .iter()
            .any(|path| path.ends_with("model.safetensors")));
        assert!(merged_paths
            .iter()
            .all(|path| !path.ends_with("adapter_model.safetensors")));

        let gguf = root.join("specialist.gguf");
        std::fs::write(&gguf, "gguf-fixture").unwrap();
        let gguf_proposal = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &lora_out,
            tag,
            adapter: &gguf,
            curator: "jason",
        })
        .unwrap();
        assert_eq!(gguf_proposal.trained_shape.as_deref(), Some("gguf"));
        let prior_prepare = std::fs::read(lora_out.join("prepare.json")).unwrap();
        let prior_json = std::fs::read(lora_out.join("binding-proposal.json")).unwrap();
        std::fs::remove_file(lora_out.join("binding-proposal.md")).unwrap();
        std::fs::create_dir(lora_out.join("binding-proposal.md")).unwrap();
        let rolled = import_trained(&ImportTrainedRequest {
            estate: &estate,
            prepared_dir: &lora_out,
            tag,
            adapter: &real_dir,
            curator: "jason",
        })
        .unwrap_err();
        assert!(
            rolled.to_string().contains("refuse:prepare-write"),
            "{rolled}"
        );
        assert_eq!(
            std::fs::read(lora_out.join("prepare.json")).unwrap(),
            prior_prepare
        );
        assert_eq!(
            std::fs::read(lora_out.join("binding-proposal.json")).unwrap(),
            prior_json
        );
        let restored: EnrichPrepareDoc =
            serde_json::from_str(&std::fs::read_to_string(lora_out.join("prepare.json")).unwrap())
                .unwrap();
        assert_eq!(restored.trained_shape.as_deref(), Some("gguf"));
        let staged = apply_proposal(&ApplyProposalRequest {
            estate: &estate,
            estate_path: &source,
            prepared_dir: &lora_out,
            tag,
            curator: "jason",
            state_dir: &root.join("state-lora"),
            verify_endpoint: None,
        })
        .unwrap();
        assert!(matches!(staged, ApplyProposalOutcome::Staged(_)));
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
    fn axolotl_qlora_matches_the_public_example_and_caps_steps() {
        let root = tmp("axolotl-qlora");
        let pack = fixture_pack();
        let seated = seated_estate("llama3");
        let blocked = root.join("seat-only");
        let err = run(AXOLOTL_QLORA_ID, &pack, &seated, &blocked, "train", "jason").unwrap_err();
        assert!(err.to_string().contains("refuse:train-base"), "{err}");
        assert!(!blocked.exists());

        let estate = with_train_base(seated, "Qwen/Qwen2.5-0.5B-Instruct");
        let out = root.join("recipe");
        let doc = run(AXOLOTL_QLORA_ID, &pack, &estate, &out, "train", "jason").unwrap();
        assert_eq!(doc.driver, AXOLOTL_QLORA_ID);
        assert_eq!(doc.base_model, "llama3");
        assert_eq!(
            doc.train_base_model.as_deref(),
            Some("Qwen/Qwen2.5-0.5B-Instruct")
        );
        let yaml = std::fs::read_to_string(out.join("axolotl.yml")).unwrap();
        assert!(
            yaml.contains("base_model: \"Qwen/Qwen2.5-0.5B-Instruct\""),
            "{yaml}"
        );
        assert!(yaml_line(&yaml, "adapter: qlora"), "{yaml}");
        assert!(yaml_line(&yaml, "load_in_8bit: false"), "{yaml}");
        assert!(yaml_line(&yaml, "load_in_4bit: true"), "{yaml}");
        assert!(yaml_line(&yaml, "sequence_len: 4096"), "{yaml}");
        assert!(yaml_line(&yaml, "micro_batch_size: 2"), "{yaml}");
        assert!(yaml_line(&yaml, "gradient_accumulation_steps: 4"), "{yaml}");
        assert!(yaml_line(&yaml, "lora_r: 32"), "{yaml}");
        assert!(yaml_line(&yaml, "lora_alpha: 16"), "{yaml}");
        assert!(yaml_line(&yaml, "num_epochs: 4"), "{yaml}");
        assert!(yaml_line(&yaml, "optimizer: paged_adamw_32bit"), "{yaml}");
        assert!(yaml_line(&yaml, "saves_per_epoch: 1"), "{yaml}");
        assert!(
            !yaml
                .lines()
                .any(|line| line.trim_start().starts_with("max_steps:")),
            "{yaml}"
        );
        assert!(yaml.contains("examples/llama-3/qlora.yml"), "{yaml}");
        assert!(
            !yaml.contains("adapter: lora\n") && yaml.contains("adapter: qlora"),
            "{yaml}"
        );
        let next = std::fs::read_to_string(out.join("NEXT.md")).unwrap();
        let config = out.join("axolotl.yml");
        assert!(
            next.contains(&format!("axolotl train {}", config.display())),
            "{next}"
        );
        assert!(next.contains("examples/llama-3/qlora.yml"), "{next}");
        assert!(next.contains("sequence_len"), "{next}");
        assert!(next.contains("--max-steps 10"), "{next}");
        assert!(next.contains("does not install flash attention"), "{next}");
        assert!(
            next.contains("attn_implementation: flash_attention_2"),
            "{next}"
        );
        assert!(!next.contains("flash_attention: true"), "{next}");
        assert!(
            !yaml.lines().any(|line| {
                let field = line.trim_start();
                field.starts_with("attn_implementation:") || field.starts_with("flash_attention:")
            }),
            "{yaml}"
        );
        assert!(!next.contains("Edit axolotl.yml"), "{next}");
        let qlora_outputs = out.join("outputs");
        let qlora_merged = qlora_outputs.join("merged");
        assert!(
            next.contains(&format!(
                "axolotl merge-lora {} --lora-model-dir={} --dequant",
                config.display(),
                qlora_outputs.display()
            )),
            "{next}"
        );
        assert!(next.contains("estate enrich merge-adapt"), "{next}");
        assert!(next.contains("estate enrich gguf-convert"), "{next}");
        assert!(next.contains("estate enrich local-seat"), "{next}");
        assert!(next.contains("--outtype auto"), "{next}");
        assert!(next.contains(&qlora_merged.display().to_string()), "{next}");
        assert!(next.contains("Axolotl does not write GGUF"), "{next}");
        assert!(next.contains("READY_FOR_LIVE_TEST: no"), "{next}");
        let qlora_prepare = std::fs::read_to_string(out.join("PREPARE.md")).unwrap();
        assert!(
            qlora_prepare.contains("estate enrich local-seat"),
            "{qlora_prepare}"
        );
        assert!(qlora_prepare.contains("--dequant"), "{qlora_prepare}");

        let gauge_out = root.join("gauge");
        run_max(
            AXOLOTL_QLORA_ID,
            &pack,
            &estate,
            &gauge_out,
            "train",
            "jason",
            Some(10),
        )
        .unwrap();
        let gauge = std::fs::read_to_string(gauge_out.join("axolotl.yml")).unwrap();
        assert!(yaml_line(&gauge, "max_steps: 10"), "{gauge}");
        assert!(yaml_line(&gauge, "save_steps: 10"), "{gauge}");
        assert!(
            !gauge
                .lines()
                .any(|line| line.trim_start().starts_with("saves_per_epoch:")),
            "{gauge}"
        );
        assert!(yaml_line(&gauge, "num_epochs: 4"), "{gauge}");
        let gauge_next = std::fs::read_to_string(gauge_out.join("NEXT.md")).unwrap();
        assert!(
            gauge_next.contains("gauge run with max_steps 10"),
            "{gauge_next}"
        );

        let zero = root.join("zero");
        let err = run_max(
            AXOLOTL_LORA_ID,
            &pack,
            &estate,
            &zero,
            "train",
            "jason",
            Some(0),
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:max-steps"), "{err}");
        assert!(!zero.exists());

        let enrich_out = root.join("enrich-job");
        let err = run(
            AXOLOTL_QLORA_ID,
            &pack,
            &estate,
            &enrich_out,
            "enrich",
            "jason",
        )
        .unwrap_err();
        assert!(err.to_string().contains("refuse:job"), "{err}");
        assert!(!enrich_out.exists());

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
        assert_eq!(proposal.driver, AXOLOTL_QLORA_ID);
    }

    #[test]
    fn from_feed_hydrates_fixture_rows_and_names_refuses() {
        let root = tmp("from-feed");
        let pack = fixture_pack();
        let estate = with_train_base(seated_estate("llama3"), "Qwen/Qwen2.5-0.5B-Instruct");
        let state = root.join("cell");
        write_fixture_feed(&state);

        let quiet = root.join("quiet");
        let quiet_doc =
            run_feed(LLAMAFACTORY_QLORA_ID, &pack, &estate, &quiet, &state, false).unwrap();
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
        assert!(missing_text.contains("does not download"), "{missing_text}");
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

        let host = enrich_host_class_affinity(&estate, &pack);
        let drivers = train_enrich_drivers_for_prepare("train", &host).unwrap();
        assert!(!drivers.contains(&MLX_LM_LORA_ID), "{host}");
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
                official_scale: false,
                from_feed: true,
                state_dir: &missing_state,
            })
            .collect();
        let set_err = prepare_enrich_set(&reqs).unwrap_err();
        assert!(set_err.to_string().contains("refuse:dataset"), "{set_err}");
        for dir in &dirs {
            assert!(!dir.exists(), "{}", dir.display());
        }

        for driver in [
            LLAMAFACTORY_LORA_ID,
            LLAMAFACTORY_QLORA_ID,
            AXOLOTL_LORA_ID,
            AXOLOTL_QLORA_ID,
        ] {
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
                assert!(card.contains("dataset_mode: feed"), "{driver} {card}");
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
            if is_axolotl_driver(driver) {
                assert!(jsonl.contains("\"instruction\""), "{jsonl}");
                assert!(
                    jsonl.contains("\"output\":\"job=policy-precheck\""),
                    "{jsonl}"
                );
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
        assert!(unknown.to_string().contains("refuse:dataset"), "{unknown}");
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
        assert!(secret.to_string().contains("refuse:raw-secret"), "{secret}");
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
        assert!(escaped.to_string().contains("refuse:dataset"), "{escaped}");
        assert!(escaped.to_string().contains("outside"), "{escaped}");
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
        assert!(
            alpaca.contains("\"instruction\":\"Name the pack\""),
            "{alpaca}"
        );
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
            official_scale: false,
            from_feed: true,
            state_dir: &state,
        })
        .unwrap_err();
        assert!(
            ollama_err.to_string().contains("refuse:dataset"),
            "{ollama_err}"
        );
        assert!(
            ollama_err.to_string().contains("llamafactory-lora"),
            "{ollama_err}"
        );
        assert!(
            ollama_err.to_string().contains("llamafactory-qlora"),
            "{ollama_err}"
        );
        assert!(
            ollama_err.to_string().contains("axolotl-lora"),
            "{ollama_err}"
        );
        assert!(
            ollama_err.to_string().contains("axolotl-qlora"),
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

        let alpaca =
            "{\"kind\":\"model.frontier.complete\",\"instruction\":\"Say hi\",\"output\":\"hi\"}\n";
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
        let err = run_feed(AXOLOTL_LORA_ID, &pack, &estate, &both, &state, true).unwrap_err();
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
                official_scale: false,
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
                official_scale: false,
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
                official_scale: false,
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
                official_scale: false,
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
    fn train_prepare_facts_stay_quiet_when_missing_and_refuse_symlinks() {
        let root = tmp("train-facts");
        let missing = train_prepare_facts(&root).unwrap();
        assert!(missing.is_none());
        assert!(!root.join("enrich").exists());

        let pack_id = "phi3-instruct";
        let driver = "llamafactory-qlora";
        let driver_dir = root.join("enrich").join(pack_id).join(driver);
        std::fs::create_dir_all(&driver_dir).unwrap();
        let prepare = driver_dir.join("prepare.json");
        std::fs::write(&prepare, sample_train_prepare(pack_id, driver, None)).unwrap();
        let facts = train_prepare_facts(&root).unwrap().unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].pack_id, pack_id);
        assert_eq!(facts[0].driver, driver);
        assert_eq!(facts[0].job, "train");
        assert_eq!(facts[0].seat_tag.as_deref(), Some("llama3"));
        assert_eq!(
            facts[0].train_base.as_deref(),
            Some("microsoft/Phi-3-mini-4k-instruct")
        );
        assert!(facts[0].trained_shape.is_none());
        assert_eq!(facts[0].out_dir, driver_dir);

        std::fs::write(
            &prepare,
            sample_train_prepare(pack_id, driver, Some("adapter")),
        )
        .unwrap();
        let shaped = train_prepare_facts(&root).unwrap().unwrap();
        assert_eq!(shaped[0].trained_shape.as_deref(), Some("adapter"));

        std::fs::write(&prepare, "not-json\n").unwrap();
        let bad = train_prepare_facts(&root).unwrap_err();
        assert!(
            bad.to_string().contains("refuse:prepare-unreadable"),
            "{bad}"
        );
        assert_eq!(std::fs::read_to_string(&prepare).unwrap(), "not-json\n");

        std::fs::write(&prepare, sample_train_prepare(pack_id, driver, None)).unwrap();
        let outside = tmp("train-facts-outside");
        let outside_prepare = outside.join("prepare.json");
        std::fs::write(
            &outside_prepare,
            sample_train_prepare("escaped-pack", driver, None),
        )
        .unwrap();
        std::fs::remove_file(&prepare).unwrap();
        std::os::unix::fs::symlink(&outside_prepare, &prepare).unwrap();
        let linked = train_prepare_facts(&root).unwrap_err();
        assert!(
            linked.to_string().contains("refuse:enrich-index"),
            "{linked}"
        );
        assert!(linked.to_string().contains("symlink"), "{linked}");
        assert!(!linked.to_string().contains("escaped-pack"), "{linked}");

        std::fs::remove_file(&prepare).unwrap();
        std::fs::remove_dir_all(root.join("enrich").join(pack_id)).unwrap();
        let outside_pack = outside.join(pack_id);
        std::fs::create_dir_all(outside_pack.join(driver)).unwrap();
        std::fs::write(
            outside_pack.join(driver).join("prepare.json"),
            sample_train_prepare("escaped-pack", driver, None),
        )
        .unwrap();
        std::os::unix::fs::symlink(&outside_pack, root.join("enrich").join(pack_id)).unwrap();
        let escaped = train_prepare_facts(&root).unwrap_err();
        assert!(
            escaped.to_string().contains("refuse:enrich-index"),
            "{escaped}"
        );
        assert!(escaped.to_string().contains("symlink"), "{escaped}");
        assert!(!escaped.to_string().contains("escaped-pack"), "{escaped}");
    }

    fn sample_train_prepare(pack_id: &str, driver: &str, shape: Option<&str>) -> String {
        let trained = match shape {
            Some(shape) => format!(
                ",\n  \"trained_shape\": \"{shape}\",\n  \"trained_paths\": [\"/tmp/cell-one-adapter\"]"
            ),
            None => String::new(),
        };
        format!(
            "{{\n  \"schema\": \"cell-one.enrich-prepare.v0\",\n  \"driver\": \"{driver}\",\n  \"job\": \"train\",\n  \"pack_id\": \"{pack_id}\",\n  \"base_model\": \"llama3\",\n  \"seat_tag\": \"llama3\",\n  \"train_base_model\": \"microsoft/Phi-3-mini-4k-instruct\",\n  \"purpose\": \"smoke\",\n  \"host_class_affinity\": \"any\",\n  \"source_paths\": [],\n  \"source_drivers\": [],\n  \"artifacts\": [\"prepare.json\"],\n  \"promoted\": false,\n  \"auto_apply\": false,\n  \"estate_rewritten\": false,\n  \"note\": \"Prepared artifacts only.\"{trained}\n}}\n"
        )
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
