//! Train/enrich facilitation. Prepare artifacts for a purpose-built SLM.
//!
//! The local runtime stays a seat. Ollama is today's entrant. This module
//! does not shell out, does not POST a train job, and does not rewrite the
//! estate. A later trainer consumes `external-manifest` or adds a card here.
//! Floor and estate-control dispatch do not match driver ids.

use crate::error::ModelError;
use estate_schema::{contains_sku, is_sacred_name, Estate};
use feed_collector::{
    refuse_curator, refuse_frontier_source_on_estate, refuse_pack, refuse_raw_secrets, FeedError,
    PackManifest,
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

/// Catalog card. A third entrant is another row in `REGISTRY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainEnrichCard {
    pub driver_id: &'static str,
    pub status: &'static str,
    pub integrates: &'static str,
    pub notes: &'static str,
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
    pub base_model: String,
    pub purpose: String,
    pub system_text: String,
    pub host_class_affinity: String,
    pub source_paths: Vec<String>,
    pub source_drivers: Vec<String>,
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

const REGISTRY: &[RegisteredDriver] = &[
    RegisteredDriver {
        card: TrainEnrichCard {
            driver_id: "ollama-modelfile",
            status: "integration",
            integrates: "ollama create / Modelfile FROM+SYSTEM",
            notes: "Joins the seated Ollama runtime. Writes a Modelfile. Does not shell out. Does not train.",
        },
        build: || Box::new(OllamaModelfileDriver),
    },
    RegisteredDriver {
        card: TrainEnrichCard {
            driver_id: "external-manifest",
            status: "portable",
            integrates: "json+yaml manifest",
            notes: "Market-shift hatch. A future trainer reads the manifest. No vendor lock.",
        },
        build: || Box::new(ExternalManifestDriver),
    },
];

struct OllamaModelfileDriver;

struct ExternalManifestDriver;

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
            "# schema: {schema}\n# driver: ollama-modelfile\n# job: {kind}\n# prepare writes this file. It does not run ollama.\nFROM {base}\nSYSTEM \"\"\"{system}\"\"\"\n",
            schema = PREPARE_SCHEMA,
            kind = job.kind.as_str(),
            base = job.base_model,
            system = job.system_text,
        );
        let steps = format!(
            "This step wrote a Modelfile (FROM + SYSTEM). It did not run ollama, did not train, and did not rewrite the estate.\n\
             \n\
             On the seated local runtime (Ollama today), after FROM names a model that runtime already has:\n\
             \n\
             ollama create {name} -f Modelfile\n\
             \n\
             Dataset path hints from the pack (not downloaded):\n\
             {paths}\n",
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
    pub base_model: String,
    pub purpose: String,
    pub host_class_affinity: String,
    pub source_paths: Vec<String>,
    pub source_drivers: Vec<String>,
    pub artifacts: Vec<String>,
    pub promoted: bool,
    pub auto_apply: bool,
    pub estate_rewritten: bool,
    pub note: String,
}

pub struct PrepareEnrichRequest<'a> {
    pub estate: &'a Estate,
    pub pack: &'a PackManifest,
    pub curator: &'a str,
    pub driver_id: &'a str,
    pub job: &'a str,
    pub out_dir: &'a Path,
}

pub fn train_enrich_catalog() -> Vec<TrainEnrichCard> {
    REGISTRY.iter().map(|row| row.card).collect()
}

pub fn resolve_train_enrich_driver(id: &str) -> Result<Box<dyn TrainEnrichDriver>, ModelError> {
    let id = id.trim();
    REGISTRY
        .iter()
        .find(|row| row.card.driver_id == id)
        .map(|row| (row.build)())
        .ok_or_else(|| {
            let known = REGISTRY
                .iter()
                .map(|row| row.card.driver_id)
                .collect::<Vec<_>>()
                .join(", ");
            ModelError::Other(format!(
                "refuse:driver: unknown train/enrich driver '{id}' (catalog: {known})"
            ))
        })
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
        lines.push(format!(
            "  {:<20} status={:<12} integrates={} live={}",
            card.driver_id, card.status, card.integrates, probe.live
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
    let job = enrich_job(req.pack, kind)?;
    refuse_job_text(&job)?;
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
        purpose: job.purpose.clone(),
        host_class_affinity: job.host_class_affinity.clone(),
        source_paths: job.source_paths.clone(),
        source_drivers: job.source_drivers.clone(),
        artifacts: names.clone(),
        promoted: false,
        auto_apply: false,
        estate_rewritten: false,
        note: PREPARE_NOTE.into(),
    };
    let prepare_json = to_pretty(&doc)?;
    files.push(("prepare.json".into(), prepare_json));
    let next = next_markdown(driver.id(), &job, req.out_dir, &names, &prepared.steps);
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

fn enrich_job(pack: &PackManifest, kind: EnrichJobKind) -> Result<EnrichJob, ModelError> {
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
    let base_model = pack
        .model_hint
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("local_slm")
        .to_string();
    let host_class_affinity = pack
        .host_class_affinity
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(pack.host_class.as_str())
        .to_string();
    let job = EnrichJob {
        kind,
        pack_id: pack.id.clone(),
        base_model,
        purpose,
        system_text,
        host_class_affinity,
        source_paths: pack.source_paths.clone(),
        source_drivers: pack.source_drivers.clone(),
    };
    Ok(job)
}

fn refuse_job_text(job: &EnrichJob) -> Result<(), ModelError> {
    refuse_sacred_and_sku("pack id", &job.pack_id)?;
    refuse_sacred_and_sku("base model", &job.base_model)?;
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
    format!(
        "# Enrich prepare ({driver_id})\n\n\
         Pack: {pack}\n\
         Job: {kind}\n\
         Driver: {driver_id}\n\
         Base ref: {base}\n\
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
    let (handoff, import_path, path_note) = if driver_id == "ollama-modelfile" {
        (
            format!(
                "Run this on the seated host. This factory does not run it.\n\
                 \n\
                 ollama create {tag} -f {modelfile}\n\
                 \n\
                 FROM must already name a model that runtime has. This factory did not pull weights.\n",
                modelfile = modelfile.display(),
            ),
            modelfile,
            "Path is the Modelfile this prepare wrote.",
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
            manifest_json,
            "Point --path at the weights file you loaded. Until then, manifest.json is the portable hatch.",
        )
    } else {
        (
            format!("{steps}\n"),
            out_dir.join("prepare.json"),
            "Point --path at the file you loaded on the seated runtime.",
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
         estate enrich import-prepared --estate <estate.yaml> --prepared {out} --tag {tag} --path {import_path}\n\
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
        import_path = import_path.display(),
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
pub fn apply_proposal(
    req: &ApplyProposalRequest<'_>,
) -> Result<ApplyProposalOutcome, ModelError> {
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
    if doc.pack_id != proposal.pack_id || doc.driver != proposal.driver || doc.job != proposal.job
    {
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
    let yaml = estate_schema::render_estate_yaml(&staged_estate).map_err(|err| {
        ModelError::Other(format!("refuse:binding: staged estate: {err}"))
    })?;
    let parsed = estate_schema::load_estate_str(&yaml).map_err(|err| {
        ModelError::Other(format!("refuse:binding: staged estate: {err}"))
    })?;
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
        ModelError::Other(format!("refuse:stage-write: {}: {err}", staged_tmp.display()))
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
    std::fs::rename(&json_tmp, stage_dir.join(STAGE_JSON)).map_err(|err| {
        ModelError::Other(format!("refuse:stage-write: stage.json: {err}"))
    })?;
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
        ModelError::Other(format!(
            "refuse:stage: {}: {err}",
            staged_path.display()
        ))
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        ModelError::Other("refuse:stage: staged estate is not utf-8".into())
    })?;
    let parsed = estate_schema::load_estate_str(text).map_err(|err| {
        ModelError::Other(format!("refuse:stage: staged estate: {err}"))
    })?;
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
        std::fs::write(stage_json_path(state_dir), json).map_err(|err| {
            ModelError::Other(format!("refuse:stage-write: stage.json: {err}"))
        })?;
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
    let text = std::fs::read_to_string(path).map_err(|err| {
        ModelError::Other(format!(
            "refuse:proposal: {}: {err}",
            path.display()
        ))
    })?;
    refuse_raw_secrets(&text).map_err(map_feed)?;
    let proposal: EnrichBindingProposal = serde_json::from_str(&text).map_err(|err| {
        ModelError::Other(format!(
            "refuse:proposal: {}: {err}",
            path.display()
        ))
    })?;
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
    let class = obj.get("class").and_then(|value| value.as_str()).unwrap_or("");
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
    let driver = obj.get("driver").and_then(|value| value.as_str()).unwrap_or("");
    if driver != seat.driver || proposal.seated_driver != seat.driver || driver.trim().is_empty() {
        return Err(ModelError::Other(
            "refuse:binding: proposed driver must stay the seated local_slm driver".into(),
        ));
    }
    refuse_sacred_and_sku("seated driver", driver)?;
    let wired = obj.get("wired").and_then(|value| value.as_bool()).ok_or_else(|| {
        ModelError::Other("refuse:binding: wired must be a bool".into())
    })?;
    if wired != seat.wired {
        return Err(ModelError::Other(
            "refuse:binding: proposed wired does not match the seat".into(),
        ));
    }
    let params = obj.get("params").and_then(|value| value.as_object()).ok_or_else(|| {
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
    let model = params.get("model").and_then(|value| value.as_str()).unwrap_or("");
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
    let text = std::fs::read_to_string(&path).map_err(|err| {
        ModelError::Other(format!("refuse:stage: {}: {err}", path.display()))
    })?;
    refuse_raw_secrets(&text).map_err(map_feed)?;
    let stage: EnrichBindingStage = serde_json::from_str(&text).map_err(|err| {
        ModelError::Other(format!("refuse:stage: {}: {err}", path.display()))
    })?;
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
    let text = std::fs::read_to_string(path).map_err(|err| {
        ModelError::Other(format!("refuse:stage: {}: {err}", path.display()))
    })?;
    let estate = estate_schema::load_estate_str(&text).map_err(|err| {
        ModelError::Other(format!("refuse:stage: {}: {err}", path.display()))
    })?;
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
    std::fs::canonicalize(path).map_err(|err| {
        ModelError::Other(format!("refuse:path: {}: {err}", path.display()))
    })
}

fn canonical_string(path: &Path) -> Result<String, ModelError> {
    Ok(canonical_path(path)?.display().to_string())
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), ModelError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|err| {
                ModelError::Other(format!(
                    "refuse:stage-write: {}: {err}",
                    parent.display()
                ))
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
        let path = repo_root().join(format!(
            "target/enrich-unit-{}-{}",
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

    fn run(
        driver: &str,
        pack: &PackManifest,
        estate: &Estate,
        out: &Path,
        job: &str,
        curator: &str,
    ) -> Result<EnrichPrepareDoc, ModelError> {
        prepare_enrich(&PrepareEnrichRequest {
            estate,
            pack,
            curator,
            driver_id: driver,
            job,
            out_dir: out,
        })
    }

    #[test]
    fn catalog_registers_both_drivers_and_refuses_unknown() {
        let ids: Vec<_> = train_enrich_catalog()
            .into_iter()
            .map(|card| card.driver_id)
            .collect();
        assert!(ids.contains(&"ollama-modelfile"));
        assert!(ids.contains(&"external-manifest"));
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
        assert!(rendered.contains("live=false"), "{rendered}");
    }

    #[test]
    fn both_drivers_prepare_fixture_without_training() {
        let root = tmp("happy");
        let pack = fixture_pack();
        let estate = fixture_estate();
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
        assert_eq!(doc.base_model, "local_slm");
        let modelfile = std::fs::read_to_string(ollama_out.join("Modelfile")).unwrap();
        assert!(modelfile.contains("FROM local_slm"), "{modelfile}");
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
            manifest.contains("\"base_model\": \"local_slm\""),
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
        let estate = fixture_estate();
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
        let estate = fixture_estate();
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
            },
            PrepareEnrichRequest {
                estate: &estate,
                pack: &pack,
                curator: "jason",
                driver_id: "ollama-modelfile",
                job: "enrich",
                out_dir: &ollama_out,
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
            },
            PrepareEnrichRequest {
                estate: &estate,
                pack: &fenced,
                curator: "jason",
                driver_id: "ollama-modelfile",
                job: "enrich",
                out_dir: &blocked_o,
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
        let estate = fixture_estate();
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
        let estate = fixture_estate();
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
        let estate = fixture_estate();
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
        let source = root.join("estate.yaml");
        let example = repo_root().join("examples/estate.yaml");
        std::fs::copy(&example, &source).unwrap();
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

        let wrong = apply_proposal(&req(&estate, &source, &prepared, "other-tag", &state, None)).unwrap_err();
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
        assert!(
            stale.to_string().contains("refuse:estate-hash"),
            "{stale}"
        );
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
        assert!(facts.iter().any(|fact| fact.kind == "proposal"
            && fact.local_tag == "cell-enrich-overnight-traces"));

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
        let already =
            apply_proposal(&req(
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
        let snap: EnrichBindingStage = serde_json::from_str(include_str!(
            "../../../schema/enrich-binding-stage.v0.json"
        ))
        .unwrap();
        assert_eq!(snap.schema, BINDING_STAGE_SCHEMA);
        assert!(!snap.auto_apply && !snap.promoted && !snap.applied && !snap.estate_rewritten);
        assert_eq!(snap.binding_id, "local_slm");
    }
}
