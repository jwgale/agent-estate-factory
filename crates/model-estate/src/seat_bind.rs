//! Live Ollama seat auto-bind for `import-trained` / apply-proposal.
//!
//! `--seat-model` stays an explicit override. When that flag is omitted and
//! the enrich tag or journey metadata names a purpose seat (for example
//! `specialist-agnews-all`), resolve against the live seated runtime. A
//! unique match becomes `params.model`. Two or more matches refuse with a
//! `--seat-model` hint. Never silent wrong bind.

use crate::error::ModelError;
use estate_schema::Estate;
use std::fs;
use std::path::Path;

/// Words that name a seat, a driver, or a class. They are not Ollama tags.
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

pub const REFUSE_SEAT_HINT: &str =
    "Pass --seat-model <live-ollama-name> (for example specialist-agnews-all). Never silent wrong bind.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeatBind {
    /// `--seat-model` won.
    Explicit(String),
    /// Unique live (or unique offline hint) purpose seat.
    Auto(String),
    /// No purpose hint; keep `cell-enrich-{pack}`.
    EnrichTag(String),
}

impl SeatBind {
    pub fn model_name(&self) -> &str {
        match self {
            SeatBind::Explicit(name) | SeatBind::Auto(name) | SeatBind::EnrichTag(name) => name,
        }
    }
}

pub fn looks_like_purpose_seat(name: &str) -> bool {
    let stem = canonical_seat_name(name);
    stem.starts_with("specialist-") || stem.starts_with("classify-")
}

pub fn canonical_seat_name(name: &str) -> String {
    let n = name.trim();
    n.strip_suffix(":latest").unwrap_or(n).to_string()
}

fn name_eq(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn is_reserved_model_word(name: &str) -> bool {
    RESERVED_MODEL_WORDS.iter().any(|word| name_eq(name, word))
}

fn is_binding_id(estate: &Estate, name: &str) -> bool {
    name_eq(name, "local_slm")
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
pub fn is_seated_model_name(estate: &Estate, name: &str) -> bool {
    let name = name.trim();
    is_from_token(name) && !is_binding_id(estate, name) && !is_reserved_model_word(name)
}

/// Exact (including `:latest`) or hyphen-bounded prefix.
pub fn hint_matches_listed(listed: &str, hint: &str) -> bool {
    let listed_c = canonical_seat_name(listed);
    let hint_c = canonical_seat_name(hint);
    if listed_c.is_empty() || hint_c.is_empty() {
        return false;
    }
    if name_eq(&listed_c, &hint_c) {
        return true;
    }
    let prefix = format!("{hint_c}-");
    listed_c
        .to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
}

fn push_unique_hint(out: &mut Vec<String>, raw: &str) {
    let name = canonical_seat_name(raw);
    if name.is_empty() || !looks_like_purpose_seat(&name) {
        return;
    }
    if out.iter().any(|have| name_eq(have, &name)) {
        return;
    }
    out.push(name);
}

fn json_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn read_json_value(path: &Path) -> Option<serde_json::Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Journey / prepare metadata that already names a live purpose seat.
pub fn discover_purpose_seat_hints(
    prepared_dir: &Path,
    enrich_tag: &str,
    extra: Option<&str>,
) -> Vec<String> {
    let mut hints = Vec::new();
    if let Some(extra) = extra {
        push_unique_hint(&mut hints, extra);
    }
    push_unique_hint(&mut hints, enrich_tag);

    let mut dirs = vec![prepared_dir.to_path_buf()];
    if let Some(parent) = prepared_dir.parent() {
        dirs.push(parent.to_path_buf());
        if let Some(grand) = parent.parent() {
            dirs.push(grand.to_path_buf());
        }
    }
    for dir in dirs {
        if let Some(doc) = read_json_value(&dir.join("prepare.json")) {
            if let Some(seat) = json_string_field(&doc, "purpose_seat") {
                push_unique_hint(&mut hints, &seat);
            }
            if let Some(seat) = json_string_field(&doc, "specialist_tag") {
                push_unique_hint(&mut hints, &seat);
            }
        }
        if let Some(doc) = read_json_value(&dir.join("purpose-seat.json")) {
            if let Some(seat) = json_string_field(&doc, "purpose_seat")
                .or_else(|| json_string_field(&doc, "specialist_tag"))
            {
                push_unique_hint(&mut hints, &seat);
            }
        }
        if let Some(doc) = read_json_value(&dir.join("comparison.json")) {
            if let Some(seat) = json_string_field(&doc, "specialist_tag") {
                push_unique_hint(&mut hints, &seat);
            }
        }
    }
    hints
}

fn match_hints_against_live(hints: &[String], live: &[String]) -> Result<Option<String>, ModelError> {
    let mut matched: Vec<String> = Vec::new();
    for listed in live {
        let canon = canonical_seat_name(listed);
        if canon.is_empty() {
            continue;
        }
        if hints.iter().any(|hint| hint_matches_listed(listed, hint))
            && !matched.iter().any(|have| name_eq(have, &canon))
        {
            matched.push(canon);
        }
    }
    match matched.len() {
        0 => Ok(None),
        1 => Ok(Some(matched.remove(0))),
        _ => {
            matched.sort();
            Err(ModelError::Other(format!(
                "refuse:seat-model: ambiguous live seats {}. {REFUSE_SEAT_HINT}",
                matched.join(", ")
            )))
        }
    }
}

/// Resolve `params.model` for import-trained / apply-proposal.
///
/// `live` `Some` is a seated-runtime listing (`/api/tags` or `/v1/models`).
/// `None` means listing was skipped (no endpoint) — a unique purpose hint
/// still auto-binds so throwaway AG News does not need a hand patch.
pub fn resolve_import_seat_model(
    estate: &Estate,
    enrich_tag: &str,
    explicit: Option<&str>,
    hints: &[String],
    live: Option<&[String]>,
) -> Result<SeatBind, ModelError> {
    if let Some(name) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        crate::train_enrich::refuse_sacred_and_sku("seat-model", name)?;
        if !is_seated_model_name(estate, name) {
            return Err(ModelError::Other(format!(
                "refuse:seat-model: '{name}' is not a seated model tag. Use the live Ollama name (for example specialist-agnews-all), not a binding id or reserved word."
            )));
        }
        if name == enrich_tag {
            return Err(ModelError::Other(format!(
                "refuse:seat-model: '{name}' is the enrich tag. Omit --seat-model when params.model should stay {enrich_tag}."
            )));
        }
        return Ok(SeatBind::Explicit(name.to_string()));
    }

    if hints.is_empty() {
        return Ok(SeatBind::EnrichTag(enrich_tag.to_string()));
    }

    if let Some(live) = live {
        if let Some(name) = match_hints_against_live(hints, live)? {
            crate::train_enrich::refuse_sacred_and_sku("seat-model", &name)?;
            if !is_seated_model_name(estate, &name) {
                return Err(ModelError::Other(format!(
                    "refuse:seat-model: '{name}' is not a seated model tag. {REFUSE_SEAT_HINT}"
                )));
            }
            return Ok(SeatBind::Auto(name));
        }
        return Err(ModelError::Other(format!(
            "refuse:seat-model: purpose seat {} is not seated (listed {}). {REFUSE_SEAT_HINT}",
            hints.join(", "),
            if live.is_empty() {
                "no models".to_string()
            } else {
                live.join(",")
            }
        )));
    }

    match hints {
        [only] => {
            crate::train_enrich::refuse_sacred_and_sku("seat-model", only)?;
            if !is_seated_model_name(estate, only) {
                return Err(ModelError::Other(format!(
                    "refuse:seat-model: '{only}' is not a seated model tag. {REFUSE_SEAT_HINT}"
                )));
            }
            Ok(SeatBind::Auto(only.clone()))
        }
        many => Err(ModelError::Other(format!(
            "refuse:seat-model: ambiguous purpose seats {}. {REFUSE_SEAT_HINT}",
            many.join(", ")
        ))),
    }
}

/// List live seats when `CELL_LOCAL_ENDPOINT` is set. Down / empty env → `None`.
pub fn try_list_live_seats() -> Option<Vec<String>> {
    let endpoint = std::env::var("CELL_LOCAL_ENDPOINT").ok()?;
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return None;
    }
    crate::adapter::list_runtime_model_names(endpoint).ok()
}

/// Write `{prepared}/purpose-seat.json` so later import/apply can auto-bind.
pub fn write_purpose_seat_sidecar(prepared_dir: &Path, purpose_seat: &str) -> Result<(), ModelError> {
    let name = canonical_seat_name(purpose_seat);
    if name.is_empty() || !looks_like_purpose_seat(&name) {
        return Ok(());
    }
    crate::train_enrich::refuse_sacred_and_sku("purpose-seat", &name)?;
    fs::create_dir_all(prepared_dir).map_err(|err| {
        ModelError::Other(format!(
            "refuse:purpose-seat: create {}: {err}",
            prepared_dir.display()
        ))
    })?;
    let body = serde_json::json!({ "purpose_seat": name });
    let text = serde_json::to_string_pretty(&body).map_err(|err| {
        ModelError::Other(format!("refuse:purpose-seat: encode: {err}"))
    })?;
    fs::write(prepared_dir.join("purpose-seat.json"), format!("{text}\n")).map_err(|err| {
        ModelError::Other(format!(
            "refuse:purpose-seat: write {}: {err}",
            prepared_dir.display()
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use estate_schema::{Estate, ModelBinding, ModelClass};

    fn estate_with(ids: &[&str]) -> Estate {
        let mut estate = estate_schema::load_estate_unvalidated(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/estate.yaml"),
        )
        .unwrap();
        for id in ids {
            if estate.model_bindings.iter().any(|b| b.id == *id) {
                continue;
            }
            estate.model_bindings.push(ModelBinding {
                id: (*id).into(),
                class: ModelClass::Local,
                driver: "ollama".into(),
                wired: true,
                params: serde_json::json!({ "model": "llama3" }),
            });
        }
        estate
    }

    #[test]
    fn unique_live_hint_auto_binds() {
        let estate = estate_with(&["ag_news"]);
        let bind = resolve_import_seat_model(
            &estate,
            "cell-enrich-qwen3-instruct-lora",
            None,
            &["specialist-agnews-all".into()],
            Some(&["llama3".into(), "specialist-agnews-all".into()]),
        )
        .unwrap();
        assert_eq!(
            bind,
            SeatBind::Auto("specialist-agnews-all".into())
        );
    }

    #[test]
    fn latest_suffix_is_the_same_seat() {
        let estate = estate_with(&["ag_news"]);
        let bind = resolve_import_seat_model(
            &estate,
            "cell-enrich-overnight-traces",
            None,
            &["specialist-agnews-all".into()],
            Some(&["specialist-agnews-all:latest".into()]),
        )
        .unwrap();
        assert_eq!(bind.model_name(), "specialist-agnews-all");
    }

    #[test]
    fn ambiguous_live_prefix_refuses_with_seat_model_hint() {
        let estate = estate_with(&["ag_news"]);
        let err = resolve_import_seat_model(
            &estate,
            "cell-enrich-qwen3-instruct-lora",
            None,
            &["specialist-agnews".into()],
            Some(&[
                "specialist-agnews-all".into(),
                "specialist-agnews-3000".into(),
                "llama3".into(),
            ]),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("refuse:seat-model: ambiguous live seats"), "{err}");
        assert!(err.contains("specialist-agnews-3000"), "{err}");
        assert!(err.contains("specialist-agnews-all"), "{err}");
        assert!(err.contains("--seat-model"), "{err}");
        assert!(err.contains("Never silent wrong bind"), "{err}");
    }

    #[test]
    fn no_hint_keeps_enrich_tag() {
        let estate = estate_with(&[]);
        let bind = resolve_import_seat_model(
            &estate,
            "cell-enrich-overnight-traces",
            None,
            &[],
            Some(&["specialist-agnews-all".into(), "llama3".into()]),
        )
        .unwrap();
        assert_eq!(
            bind,
            SeatBind::EnrichTag("cell-enrich-overnight-traces".into())
        );
    }

    #[test]
    fn explicit_seat_model_wins() {
        let estate = estate_with(&["ag_news"]);
        let bind = resolve_import_seat_model(
            &estate,
            "cell-enrich-qwen3-instruct-lora",
            Some("specialist-agnews-all"),
            &["classify-specialist".into()],
            Some(&[
                "specialist-agnews-all".into(),
                "specialist-agnews-3000".into(),
            ]),
        )
        .unwrap();
        assert_eq!(
            bind,
            SeatBind::Explicit("specialist-agnews-all".into())
        );
    }

    #[test]
    fn live_up_but_hint_absent_refuses() {
        let estate = estate_with(&["ag_news"]);
        let err = resolve_import_seat_model(
            &estate,
            "cell-enrich-qwen3-instruct-lora",
            None,
            &["specialist-agnews-all".into()],
            Some(&["llama3".into(), "classify-base".into()]),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("is not seated"), "{err}");
        assert!(err.contains("--seat-model"), "{err}");
    }

    #[test]
    fn offline_unique_hint_auto_binds() {
        let estate = estate_with(&["ag_news"]);
        let bind = resolve_import_seat_model(
            &estate,
            "cell-enrich-qwen3-instruct-lora",
            None,
            &["specialist-agnews-all".into()],
            None,
        )
        .unwrap();
        assert_eq!(bind, SeatBind::Auto("specialist-agnews-all".into()));
    }

    #[test]
    fn discover_reads_comparison_and_purpose_sidecar() {
        let root = std::env::temp_dir().join(format!(
            "cell-seat-bind-discover-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("comparison.json"),
            r#"{"specialist_tag":"specialist-agnews-all"}"#,
        )
        .unwrap();
        let prepared = root.join("prepared");
        fs::create_dir_all(&prepared).unwrap();
        fs::write(
            prepared.join("purpose-seat.json"),
            r#"{"purpose_seat":"specialist-agnews-all"}"#,
        )
        .unwrap();
        let hints = discover_purpose_seat_hints(
            &prepared,
            "cell-enrich-qwen3-instruct-lora",
            None,
        );
        assert_eq!(hints, vec!["specialist-agnews-all".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }
}
