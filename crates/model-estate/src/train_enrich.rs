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
use std::path::{Path, PathBuf};

pub const PREPARE_SCHEMA: &str = "cell-one.enrich-prepare.v0";

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
        let name = format!("cell-enrich-{}", job.pack_id);
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

/// Refuse, then write. The estate file is not opened for write.
pub fn prepare_enrich(req: &PrepareEnrichRequest<'_>) -> Result<EnrichPrepareDoc, ModelError> {
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
        artifacts: names,
        promoted: false,
        auto_apply: false,
        estate_rewritten: false,
        note: PREPARE_NOTE.into(),
    };
    let prepare_json = to_pretty(&doc)?;
    files.push(("prepare.json".into(), prepare_json));
    for (name, body) in &files {
        refuse_sacred_and_sku(name, body)?;
        refuse_raw_secrets(body).map_err(map_feed)?;
    }
    write_files(req.out_dir, &files)?;
    Ok(doc)
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
    let prepare_md = files.iter().filter(|(name, _)| name == "PREPARE.md").count();
    if prepare_md > 1 {
        return Err(ModelError::Other(
            "refuse:prepare: driver must not emit reserved file 'PREPARE.md'".into(),
        ));
    }
    for (name, _) in files {
        if name == "prepare.json" {
            return Err(ModelError::Other(
                "refuse:prepare: driver must not emit reserved file 'prepare.json'".into(),
            ));
        }
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

fn read_pack_file(path: &Path) -> Result<PackManifest, ModelError> {
    let text = std::fs::read_to_string(path).map_err(|_| {
        ModelError::Other(format!("refuse:missing-pack: {}", path.display()))
    })?;
    serde_json::from_str(&text).map_err(|err| {
        ModelError::Other(format!("refuse:pack: {} ({err})", path.display()))
    })
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
    let body = serde_json::to_string_pretty(value).map_err(|err| {
        ModelError::Other(format!("refuse:prepare: serialize: {err}"))
    })?;
    if body.trim().is_empty() || body.trim() == "{}" {
        return Err(ModelError::Other(
            "refuse:prepare: serialize: empty".into(),
        ));
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

    fn run(driver: &str, pack: &PackManifest, estate: &Estate, out: &Path, job: &str, curator: &str) -> Result<EnrichPrepareDoc, ModelError> {
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
        assert!(!manifest.to_ascii_lowercase().contains("ollama"), "{manifest}");
        assert!(manifest.contains("\"vendor\": null"), "{manifest}");
        assert!(manifest.contains("feed/events.jsonl"), "{manifest}");
        assert!(manifest.contains("\"base_model\": \"local_slm\""), "{manifest}");
        let yaml = std::fs::read_to_string(manifest_out.join("manifest.yaml")).unwrap();
        assert!(yaml.contains("dataset_paths:"), "{yaml}");
        assert!(!yaml.to_ascii_lowercase().contains("ollama"), "{yaml}");
        let ext_steps = std::fs::read_to_string(manifest_out.join("PREPARE.md")).unwrap();
        assert!(!ext_steps.contains("ollama create"), "{ext_steps}");
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
        let err = run("ollama-modelfile", &sku, &estate, &sku_out, "enrich", "jason").unwrap_err();
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
        assert!(err.to_string().contains("SKU") || err.to_string().contains("sku"), "{err}");
        assert!(!path_out.exists());

        let mut local = fixture_estate();
        local.model_bindings.retain(|b| b.class != ModelClass::Frontier);
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
        assert!(missing.to_string().contains("refuse:missing-pack"), "{missing}");

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
}
