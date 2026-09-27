//! Classify-journey GGUF → joinable `local_slm` seat.
//!
//! `import-trained` writes `specialty-join.json` beside the binding proposal.
//! Standing next and status re-check the GGUF, the proposal, and the agent
//! allow-list. `auto_apply` stays false. This file is not a live PASS.

use crate::error::ModelError;
use crate::train_enrich::{list_prepared, refuse_sacred_and_sku, EnrichBindingProposal};
use estate_schema::Estate;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const SPECIALTY_JOIN_SCHEMA: &str = "cell-one.specialty-join.v0";
pub const SPECIALTY_JOIN_JSON: &str = "specialty-join.json";
const TRAINED_SHAPE_GGUF: &str = "gguf";
const FUNCTION_UNSPECIFIED: &str = "unspecified";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecialtyJoin {
    pub schema: String,
    pub binding_id: String,
    pub class: String,
    pub function: String,
    pub trained_shape: String,
    pub auto_apply: bool,
    pub joinable: bool,
    pub reason: String,
    pub gguf: String,
    pub seat_model: String,
    pub agents: Vec<String>,
    pub local_tag: String,
    pub ready_for_live_test: bool,
    pub live_pass_recorded: bool,
    pub note: String,
}

impl SpecialtyJoin {
    pub fn status_line(&self) -> String {
        let agents = if self.agents.is_empty() {
            "-".to_string()
        } else {
            self.agents.join(",")
        };
        format!(
            "specialty_join: joinable={} binding={} class={} function={} trained_shape={} auto_apply={} agents={} seat_model={}",
            if self.joinable { "yes" } else { "no" },
            self.binding_id,
            self.class,
            self.function,
            self.trained_shape,
            self.auto_apply,
            agents,
            self.seat_model,
        )
    }

    pub fn standing_report(&self, estate: &Path, prepared: &Path) -> String {
        let mut out = String::new();
        out.push_str("Standing next (estate) — joinable specialty\n");
        out.push_str(&format!(
            "joinable: {}\n",
            if self.joinable { "yes" } else { "no" }
        ));
        out.push_str(&format!("binding: {}\n", self.binding_id));
        out.push_str(&format!("class: {}\n", self.class));
        out.push_str(&format!("function: {}\n", self.function));
        out.push_str(&format!("trained_shape: {}\n", self.trained_shape));
        out.push_str(&format!("auto_apply: {}\n", self.auto_apply));
        out.push_str(&format!("seat_model: {}\n", self.seat_model));
        out.push_str(&format!(
            "agents: {}\n",
            if self.agents.is_empty() {
                "-".to_string()
            } else {
                self.agents.join(",")
            }
        ));
        out.push_str(&format!("gguf: {}\n", self.gguf));
        out.push_str(&format!("reason: {}\n", self.reason));
        if self.joinable {
            out.push_str("The seat is joinable.\n");
            out.push_str(&format!(
                "An agent whose models allow-list names {} can use this specialty.\n",
                self.binding_id
            ));
        }
        out.push_str("The proposal stays auto_apply=false.\n");
        out.push_str(
            "The factory does not apply the estate without an explicit operator --require-plan path.\n",
        );
        out.push_str("No promote. No auto-promote.\n");
        out.push_str("Existing entrypoints (print only; this command does not execute them):\n");
        out.push_str(&format!(
            "estate enrich apply-proposal --estate {} --prepared {} --tag {} --state-dir .cell\n",
            estate.display(),
            prepared.display(),
            self.local_tag
        ));
        out.push_str(&format!(
            "estate plan --estate {} --plans-dir plans --state-dir .cell\n",
            estate.display()
        ));
        out.push_str(&format!(
            "estate apply --estate {} --state-dir .cell --require-plan --curator jason\n",
            estate.display()
        ));
        out.push_str(&format!(
            "estate reconcile --estate {} --state-dir .cell\n",
            estate.display()
        ));
        out.push_str("This command does not execute them.\n");
        out.push_str("This print is not a live PASS. READY_FOR_LIVE_TEST: no.\n");
        out.push_str("The factory does not claim it trained.\n");
        out
    }
}

/// Write `specialty-join.json` after a GGUF `import-trained`. Does not apply.
pub fn write_specialty_join(
    estate: &Estate,
    prepared_dir: &Path,
    proposal: &EnrichBindingProposal,
    explicit_function: Option<&str>,
) -> Result<SpecialtyJoin, ModelError> {
    if proposal.trained_shape.as_deref() != Some(TRAINED_SHAPE_GGUF) {
        return Err(ModelError::Other(
            "refuse:function: specialty join records a GGUF. This proposal is not trained_shape gguf."
                .into(),
        ));
    }
    if proposal.auto_apply {
        return Err(ModelError::Other(
            "refuse:specialty-join: auto_apply must stay false".into(),
        ));
    }
    let function = resolve_function(explicit_function, prepared_dir, Path::new(&proposal.local_path))?;
    let join = assemble(estate, proposal, &function)?;
    persist(prepared_dir, &join)?;
    Ok(join)
}

/// Drop a stale join file when the new artifact is not a GGUF.
pub fn remove_specialty_join(prepared_dir: &Path) -> Result<(), ModelError> {
    let path = prepared_dir.join(SPECIALTY_JOIN_JSON);
    match fs::symlink_metadata(&path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(ModelError::Other(format!(
            "refuse:specialty-join: {}: {err}",
            path.display()
        ))),
        Ok(meta) if meta.file_type().is_symlink() => Err(ModelError::Other(format!(
            "refuse:specialty-join: {} is a symlink",
            path.display()
        ))),
        Ok(meta) if meta.is_file() => fs::remove_file(&path).map_err(|err| {
            ModelError::Other(format!(
                "refuse:specialty-join: cannot remove {}: {err}",
                path.display()
            ))
        }),
        Ok(_) => Err(ModelError::Other(format!(
            "refuse:specialty-join: {} is not a file",
            path.display()
        ))),
    }
}

/// Re-check the proposal, the GGUF, and the current allow-list.
/// `Ok(None)` when this prepare dir has no GGUF specialty proposal.
pub fn assess_specialty_join(
    estate: &Estate,
    prepared_dir: &Path,
) -> Result<Option<SpecialtyJoin>, ModelError> {
    let proposal_path = prepared_dir.join("binding-proposal.json");
    let Some(proposal) = read_proposal_view(&proposal_path)? else {
        if join_file_present(prepared_dir)? {
            return Err(ModelError::Other(format!(
                "refuse:specialty-join: {} has specialty-join.json and no binding proposal",
                prepared_dir.display()
            )));
        }
        return Ok(None);
    };
    if proposal.trained_shape.as_deref() != Some(TRAINED_SHAPE_GGUF) {
        if join_file_present(prepared_dir)? {
            return Err(ModelError::Other(
                "refuse:specialty-join: specialty-join.json is present and trained_shape is not gguf"
                    .into(),
            ));
        }
        return Ok(None);
    }
    if proposal.auto_apply {
        return Err(ModelError::Other(
            "refuse:specialty-join: binding proposal auto_apply must stay false".into(),
        ));
    }
    let recorded = read_join_file(prepared_dir)?;
    if let Some(recorded) = &recorded {
        if recorded.binding_id != proposal.binding_id {
            return Err(ModelError::Other(format!(
                "refuse:specialty-join: recorded binding {} does not match proposal {}",
                recorded.binding_id, proposal.binding_id
            )));
        }
    }
    let function = match recorded {
        Some(recorded) => recorded.function,
        None => resolve_function(None, prepared_dir, Path::new(&proposal.local_path))?,
    };
    Ok(Some(assemble(estate, &proposal, &function)?))
}

/// Joins under `{state}/enrich/{pack}/{driver}` when that tree exists.
pub fn specialty_joins_in_enrich(
    enrich_root: &Path,
    estate: &Estate,
) -> Result<Option<Vec<SpecialtyJoin>>, ModelError> {
    match fs::symlink_metadata(enrich_root) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(ModelError::Other(format!(
                "refuse:specialty-join: {}: {err}",
                enrich_root.display()
            )))
        }
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(ModelError::Other(format!(
                "refuse:specialty-join: {} is a symlink",
                enrich_root.display()
            )))
        }
        Ok(meta) if !meta.is_dir() => {
            return Err(ModelError::Other(format!(
                "refuse:specialty-join: {} is not a directory",
                enrich_root.display()
            )))
        }
        Ok(_) => {}
    }
    let rows = list_prepared(enrich_root)?;
    let mut joins = Vec::new();
    for row in rows {
        if let Some(join) = assess_specialty_join(estate, &row.out_dir)? {
            joins.push(join);
        }
    }
    Ok(Some(joins))
}

fn assemble(
    estate: &Estate,
    proposal: &EnrichBindingProposal,
    function: &str,
) -> Result<SpecialtyJoin, ModelError> {
    let class = proposal
        .proposed_binding
        .get("class")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    if class != "local" {
        return Err(ModelError::Other(format!(
            "refuse:specialty-join: binding class '{class}' is not local"
        )));
    }
    let binding_id = proposal.binding_id.clone();
    if binding_id.trim().is_empty() {
        return Err(ModelError::Other(
            "refuse:specialty-join: binding id is empty".into(),
        ));
    }
    refuse_sacred_and_sku("binding", &binding_id)?;
    let seat_model = proposal
        .proposed_binding
        .get("params")
        .and_then(|params| params.get("model"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("-")
        .to_string();
    refuse_sacred_and_sku("seat-model", &seat_model)?;
    let agents = agents_allowing(estate, &binding_id);
    let gguf_block = gguf_block_reason(Path::new(&proposal.local_path))?;
    let (joinable, reason) = if let Some(reason) = gguf_block {
        (false, reason)
    } else if agents.is_empty() {
        (
            false,
            format!("no agent models allow-list names {binding_id}"),
        )
    } else {
        (
            true,
            format!("agents {} allow {binding_id}", agents.join(",")),
        )
    };
    Ok(SpecialtyJoin {
        schema: SPECIALTY_JOIN_SCHEMA.into(),
        binding_id,
        class,
        function: function.to_string(),
        trained_shape: TRAINED_SHAPE_GGUF.into(),
        auto_apply: false,
        joinable,
        reason,
        gguf: proposal.local_path.clone(),
        seat_model,
        agents,
        local_tag: proposal.local_tag.clone(),
        ready_for_live_test: false,
        live_pass_recorded: false,
        note: "Proposal only. auto_apply=false. trained_shape is gguf. The seat is joinable when an agent models allow-list names the binding. import-trained does not apply. This file is not a live PASS. READY_FOR_LIVE_TEST: no.".into(),
    })
}

fn persist(prepared_dir: &Path, join: &SpecialtyJoin) -> Result<(), ModelError> {
    if join.ready_for_live_test || join.live_pass_recorded || join.auto_apply {
        return Err(ModelError::Other(
            "refuse:specialty-join: dishonest flag".into(),
        ));
    }
    let body = serde_json::to_string_pretty(join)
        .map_err(|err| ModelError::Other(format!("refuse:specialty-join: encode: {err}")))?;
    let text = format!("{body}\n");
    refuse_sacred_and_sku("specialty join", &text)?;
    fs::create_dir_all(prepared_dir).map_err(|err| {
        ModelError::Other(format!(
            "refuse:specialty-join: create {}: {err}",
            prepared_dir.display()
        ))
    })?;
    let tmp = prepared_dir.join("specialty-join.json.tmp");
    let dest = prepared_dir.join(SPECIALTY_JOIN_JSON);
    fs::write(&tmp, &text).map_err(|err| {
        ModelError::Other(format!(
            "refuse:specialty-join: write {}: {err}",
            tmp.display()
        ))
    })?;
    fs::rename(&tmp, &dest).map_err(|err| {
        ModelError::Other(format!(
            "refuse:specialty-join: rename {}: {err}",
            dest.display()
        ))
    })?;
    Ok(())
}

fn resolve_function(
    explicit: Option<&str>,
    prepared_dir: &Path,
    gguf: &Path,
) -> Result<String, ModelError> {
    if let Some(raw) = explicit.map(str::trim).filter(|value| !value.is_empty()) {
        return require_slug(raw);
    }
    let mut paths = Vec::new();
    if let Some(parent) = gguf.parent() {
        paths.push(parent.join("comparison.json"));
    }
    paths.push(prepared_dir.join("comparison.json"));
    for path in paths {
        if let Some(dataset) = comparison_dataset(&path)? {
            if let Some(slug) = optional_slug(&dataset)? {
                return Ok(slug);
            }
        }
    }
    Ok(FUNCTION_UNSPECIFIED.into())
}

fn comparison_dataset(path: &Path) -> Result<Option<String>, ModelError> {
    match fs::symlink_metadata(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(ModelError::Other(format!(
                "refuse:specialty-join: {}: {err}",
                path.display()
            )))
        }
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(ModelError::Other(format!(
                "refuse:specialty-join: {} is a symlink",
                path.display()
            )))
        }
        Ok(meta) if !meta.is_file() => {
            return Err(ModelError::Other(format!(
                "refuse:specialty-join: {} is not a file",
                path.display()
            )))
        }
        Ok(_) => {}
    }
    let text = fs::read_to_string(path).map_err(|err| {
        ModelError::Other(format!(
            "refuse:specialty-join: {}: {err}",
            path.display()
        ))
    })?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|err| {
        ModelError::Other(format!(
            "refuse:specialty-join: {} ({err})",
            path.display()
        ))
    })?;
    if value.get("live_pass_recorded") == Some(&serde_json::Value::Bool(true)) {
        return Err(ModelError::Other(format!(
            "refuse:specialty-join: {} records a live PASS",
            path.display()
        )));
    }
    match value.get("dataset") {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(dataset)) => Ok(Some(dataset.clone())),
        Some(_) => Err(ModelError::Other(format!(
            "refuse:specialty-join: {} dataset is not a string",
            path.display()
        ))),
    }
}

fn require_slug(raw: &str) -> Result<String, ModelError> {
    match optional_slug(raw)? {
        Some(slug) => Ok(slug),
        None => Err(ModelError::Other(format!(
            "refuse:function: '{raw}' is not a specialty function slug"
        ))),
    }
}

fn optional_slug(raw: &str) -> Result<Option<String>, ModelError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    refuse_sacred_and_sku("function", raw)?;
    if is_slug(raw) {
        Ok(Some(raw.to_string()))
    } else {
        Ok(None)
    }
}

fn is_slug(raw: &str) -> bool {
    let mut chars = raw.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    raw.len() <= 64
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn agents_allowing(estate: &Estate, binding_id: &str) -> Vec<String> {
    estate
        .agents
        .iter()
        .filter(|agent| {
            agent
                .models
                .iter()
                .any(|model| model.id.eq_ignore_ascii_case(binding_id))
        })
        .map(|agent| agent.id.clone())
        .collect()
}

fn gguf_block_reason(path: &Path) -> Result<Option<String>, ModelError> {
    match fs::symlink_metadata(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Some(format!(
            "specialist GGUF {} is missing",
            path.display()
        ))),
        Err(err) => Err(ModelError::Other(format!(
            "refuse:specialty-join: {}: {err}",
            path.display()
        ))),
        Ok(meta) if meta.file_type().is_symlink() => Ok(Some(format!(
            "specialist GGUF {} is a symlink. import-trained does not follow it.",
            path.display()
        ))),
        Ok(meta) if !meta.is_file() => Ok(Some(format!(
            "specialist GGUF {} is not a regular file",
            path.display()
        ))),
        Ok(_) => Ok(None),
    }
}

fn join_file_present(prepared_dir: &Path) -> Result<bool, ModelError> {
    let path = prepared_dir.join(SPECIALTY_JOIN_JSON);
    match fs::symlink_metadata(&path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(ModelError::Other(format!(
            "refuse:specialty-join: {}: {err}",
            path.display()
        ))),
        Ok(meta) if meta.file_type().is_symlink() => Err(ModelError::Other(format!(
            "refuse:specialty-join: {} is a symlink",
            path.display()
        ))),
        Ok(meta) if meta.is_file() => Ok(true),
        Ok(_) => Err(ModelError::Other(format!(
            "refuse:specialty-join: {} is not a file",
            path.display()
        ))),
    }
}

fn read_join_file(prepared_dir: &Path) -> Result<Option<SpecialtyJoin>, ModelError> {
    let path = prepared_dir.join(SPECIALTY_JOIN_JSON);
    if !join_file_present(prepared_dir)? {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|err| {
        ModelError::Other(format!(
            "refuse:specialty-join: {}: {err}",
            path.display()
        ))
    })?;
    let join: SpecialtyJoin = serde_json::from_str(&text).map_err(|err| {
        ModelError::Other(format!(
            "refuse:specialty-join: {} ({err})",
            path.display()
        ))
    })?;
    if join.schema != SPECIALTY_JOIN_SCHEMA {
        return Err(ModelError::Other(format!(
            "refuse:specialty-join: schema '{}' is not {SPECIALTY_JOIN_SCHEMA}",
            join.schema
        )));
    }
    if join.auto_apply || join.ready_for_live_test || join.live_pass_recorded {
        return Err(ModelError::Other(
            "refuse:specialty-join: auto_apply, ready_for_live_test, and live_pass_recorded stay false"
                .into(),
        ));
    }
    if join.trained_shape != TRAINED_SHAPE_GGUF || join.class != "local" {
        return Err(ModelError::Other(
            "refuse:specialty-join: recorded shape must be gguf and class local".into(),
        ));
    }
    Ok(Some(join))
}

fn read_proposal_view(path: &Path) -> Result<Option<EnrichBindingProposal>, ModelError> {
    match fs::symlink_metadata(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(ModelError::Other(format!(
                "refuse:specialty-join: {}: {err}",
                path.display()
            )))
        }
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(ModelError::Other(format!(
                "refuse:specialty-join: {} is a symlink",
                path.display()
            )))
        }
        Ok(meta) if !meta.is_file() => {
            return Err(ModelError::Other(format!(
                "refuse:specialty-join: {} is not a file",
                path.display()
            )))
        }
        Ok(_) => {}
    }
    let text = fs::read_to_string(path).map_err(|err| {
        ModelError::Other(format!(
            "refuse:specialty-join: {}: {err}",
            path.display()
        ))
    })?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|err| {
        ModelError::Other(format!(
            "refuse:specialty-join: {} ({err})",
            path.display()
        ))
    })?;
    let trained_shape = value
        .get("trained_shape")
        .and_then(|item| item.as_str())
        .map(str::to_string);
    if trained_shape.as_deref() != Some(TRAINED_SHAPE_GGUF) {
        return Ok(Some(stub_proposal(value, trained_shape)));
    }
    let binding_id = value
        .get("binding_id")
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .to_string();
    let local_path = value
        .get("local_path")
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .to_string();
    let local_tag = value
        .get("local_tag")
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .to_string();
    let auto_apply = value
        .get("auto_apply")
        .and_then(|item| item.as_bool())
        .unwrap_or(true);
    let proposed_binding = value
        .get("proposed_binding")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Ok(Some(EnrichBindingProposal {
        schema: String::new(),
        curator: String::new(),
        policy: String::new(),
        auto_apply,
        promoted: false,
        estate_rewritten: false,
        estate_name: String::new(),
        estate_hash: String::new(),
        prepared_dir: String::new(),
        pack_id: String::new(),
        driver: String::new(),
        job: String::new(),
        local_tag,
        local_path,
        trained_shape,
        trained_paths: None,
        binding_id,
        seated_driver: String::new(),
        content_scanned: false,
        proposed_binding,
        paste_yaml: String::new(),
        note: String::new(),
    }))
}

fn stub_proposal(value: serde_json::Value, trained_shape: Option<String>) -> EnrichBindingProposal {
    EnrichBindingProposal {
        schema: String::new(),
        curator: String::new(),
        policy: String::new(),
        auto_apply: value
            .get("auto_apply")
            .and_then(|item| item.as_bool())
            .unwrap_or(false),
        promoted: false,
        estate_rewritten: false,
        estate_name: String::new(),
        estate_hash: String::new(),
        prepared_dir: String::new(),
        pack_id: String::new(),
        driver: String::new(),
        job: String::new(),
        local_tag: String::new(),
        local_path: String::new(),
        trained_shape,
        trained_paths: None,
        binding_id: value
            .get("binding_id")
            .and_then(|item| item.as_str())
            .unwrap_or("")
            .to_string(),
        seated_driver: String::new(),
        content_scanned: false,
        proposed_binding: serde_json::Value::Null,
        paste_yaml: String::new(),
        note: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let mut token = std::process::id().to_string();
        for needle in ["5090", "4090", "4080", "3090"] {
            token = token.replace(needle, "0000");
        }
        let path = std::env::temp_dir().join(format!("cell-one-join-{name}-{token}"));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn explicit_function_refuses_a_non_slug() {
        let err = require_slug("fancyzhx/ag_news").unwrap_err();
        assert!(err.to_string().contains("refuse:function"), "{err}");
    }

    #[test]
    fn ag_news_gguf_is_joinable_for_agents_that_allow_local_slm() {
        let root = scratch("agnews");
        let gguf = root.join("specialist.Q4_K_M.gguf");
        fs::write(&gguf, b"GGUF").unwrap();
        fs::write(
            root.join("comparison.json"),
            "{\"dataset\":\"ag_news\",\"live_pass_recorded\":false}\n",
        )
        .unwrap();
        let prepared = root.join("prepared");
        fs::create_dir_all(&prepared).unwrap();
        let proposal = EnrichBindingProposal {
            schema: "cell-one.enrich-binding-proposal.v0".into(),
            curator: "jason".into(),
            policy: "manual".into(),
            auto_apply: false,
            promoted: false,
            estate_rewritten: false,
            estate_name: "cell-one-synthetic".into(),
            estate_hash: "abc".into(),
            prepared_dir: prepared.display().to_string(),
            pack_id: "overnight-traces".into(),
            driver: "llamafactory-lora".into(),
            job: "train".into(),
            local_tag: "cell-enrich-overnight-traces".into(),
            local_path: gguf.display().to_string(),
            trained_shape: Some("gguf".into()),
            trained_paths: Some(vec![gguf.display().to_string()]),
            binding_id: "local_slm".into(),
            seated_driver: "ollama".into(),
            content_scanned: false,
            proposed_binding: serde_json::json!({
                "id": "local_slm",
                "class": "local",
                "params": {"model": "specialist-agnews-3000"}
            }),
            paste_yaml: String::new(),
            note: String::new(),
        };
        fs::write(
            prepared.join("binding-proposal.json"),
            serde_json::to_string(&proposal).unwrap(),
        )
        .unwrap();
        let estate = estate_schema::load_estate_unvalidated(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/estate.yaml"),
        )
        .unwrap();
        let written = write_specialty_join(&estate, &prepared, &proposal, Some("ag_news")).unwrap();
        assert!(written.joinable, "{written:?}");
        assert_eq!(written.function, "ag_news");
        assert_eq!(written.binding_id, "local_slm");
        assert!(!written.auto_apply);
        assert!(!written.ready_for_live_test);
        assert!(!written.live_pass_recorded);
        assert!(written.agents.iter().any(|id| id == "research"));
        assert!(written.agents.iter().any(|id| id == "horizon"));
        let again = assess_specialty_join(&estate, &prepared).unwrap().unwrap();
        assert!(again.joinable);
        assert_eq!(again.seat_model, "specialist-agnews-3000");
        let report = again.standing_report(Path::new("lab-estate.yaml"), &prepared);
        assert!(report.contains("joinable: yes"), "{report}");
        assert!(report.contains("The seat is joinable."), "{report}");
        assert!(report.contains("function: ag_news"), "{report}");
        assert!(report.contains("READY_FOR_LIVE_TEST: no"), "{report}");
        assert!(!report.contains("READY_FOR_LIVE_TEST: yes"), "{report}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn specialty_join_records_non_local_slm_binding_id() {
        let root = scratch("portable");
        let prepared_ag = root.join("ag");
        let prepared_rust = root.join("rust");
        fs::create_dir_all(&prepared_ag).unwrap();
        fs::create_dir_all(&prepared_rust).unwrap();
        let gguf_ag = root.join("ag.gguf");
        let gguf_rust = root.join("rust.gguf");
        fs::write(&gguf_ag, b"GGUF").unwrap();
        fs::write(&gguf_rust, b"GGUF").unwrap();
        let mut estate = estate_schema::load_estate_unvalidated(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/estate.yaml"),
        )
        .unwrap();
        let ag = proposal_for(&prepared_ag, &gguf_ag, "ag_news", "specialist-agnews-3000");
        let rust = proposal_for(
            &prepared_rust,
            &gguf_rust,
            "rust_idiom",
            "specialist-rustidiom-3000",
        );
        let bare_ag = write_specialty_join(&estate, &prepared_ag, &ag, Some("ag_news")).unwrap();
        let bare_rust =
            write_specialty_join(&estate, &prepared_rust, &rust, Some("rust_idiom")).unwrap();
        assert_eq!(bare_ag.binding_id, "ag_news");
        assert_eq!(bare_rust.binding_id, "rust_idiom");
        assert!(!bare_ag.joinable, "{bare_ag:?}");
        assert!(!bare_rust.joinable, "{bare_rust:?}");
        assert!(bare_ag.reason.contains("no agent models allow-list names ag_news"));
        assert!(bare_rust
            .reason
            .contains("no agent models allow-list names rust_idiom"));
        assert_ne!(bare_ag.binding_id, "local_slm");
        assert_ne!(bare_rust.binding_id, "local_slm");

        let research = estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap();
        research.models = vec![estate_schema::ModelUseDecl {
            id: "ag_news".into(),
            description: None,
        }];
        estate.agents.push(estate_schema::Agent {
            id: "idiom".into(),
            display_name: "Idiom".into(),
            lane: "idiom".into(),
            desktop: "idiom-desktop".into(),
            tools: Vec::new(),
            mounts: Vec::new(),
            mcp: Vec::new(),
            models: vec![estate_schema::ModelUseDecl {
                id: "rust_idiom".into(),
                description: None,
            }],
            calls: Vec::new(),
            select: None,
        });
        let joined_ag = write_specialty_join(&estate, &prepared_ag, &ag, Some("ag_news")).unwrap();
        let joined_rust =
            write_specialty_join(&estate, &prepared_rust, &rust, Some("rust_idiom")).unwrap();
        assert!(joined_ag.joinable, "{joined_ag:?}");
        assert!(joined_rust.joinable, "{joined_rust:?}");
        assert_eq!(joined_ag.agents, vec!["research".to_string()]);
        assert_eq!(joined_rust.agents, vec!["idiom".to_string()]);
        assert_eq!(joined_ag.seat_model, "specialist-agnews-3000");
        assert_eq!(joined_rust.seat_model, "specialist-rustidiom-3000");
        fs::write(
            prepared_ag.join("binding-proposal.json"),
            serde_json::to_string(&ag).unwrap(),
        )
        .unwrap();
        fs::write(
            prepared_rust.join("binding-proposal.json"),
            serde_json::to_string(&rust).unwrap(),
        )
        .unwrap();
        let again = assess_specialty_join(&estate, &prepared_ag).unwrap().unwrap();
        assert_eq!(again.binding_id, "ag_news");
        assert!(again.joinable);
        let report = again.standing_report(Path::new("lab-estate.yaml"), &prepared_ag);
        assert!(report.contains("binding: ag_news"), "{report}");
        assert!(report.contains("joinable: yes"), "{report}");
        assert!(!report.contains("READY_FOR_LIVE_TEST: yes"), "{report}");
        let _ = fs::remove_dir_all(&root);
    }

    fn proposal_for(
        prepared: &Path,
        gguf: &Path,
        binding_id: &str,
        seat_model: &str,
    ) -> EnrichBindingProposal {
        EnrichBindingProposal {
            schema: "cell-one.enrich-binding-proposal.v0".into(),
            curator: "jason".into(),
            policy: "manual".into(),
            auto_apply: false,
            promoted: false,
            estate_rewritten: false,
            estate_name: "cell-one-synthetic".into(),
            estate_hash: "abc".into(),
            prepared_dir: prepared.display().to_string(),
            pack_id: "overnight-traces".into(),
            driver: "llamafactory-lora".into(),
            job: "train".into(),
            local_tag: "cell-enrich-overnight-traces".into(),
            local_path: gguf.display().to_string(),
            trained_shape: Some("gguf".into()),
            trained_paths: Some(vec![gguf.display().to_string()]),
            binding_id: binding_id.into(),
            seated_driver: "ollama".into(),
            content_scanned: false,
            proposed_binding: serde_json::json!({
                "id": binding_id,
                "class": "local",
                "params": {"model": seat_model}
            }),
            paste_yaml: String::new(),
            note: String::new(),
        }
    }
}
