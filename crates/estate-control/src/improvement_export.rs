//! Standing improvement package from the decision journal.
//!
//! `estate decisions export-package` reads host-validate / authorize /
//! convey / complete receipts and writes a JSON/YAML package that
//! proposes the next enrich (specialty seat / dataset). `auto_train`
//! stays false. The command does not train, apply, promote, or flip
//! READY.
//!
//! `estate decisions improvement-export-prove` reuses
//! `estate decisions host-validate-prove` on a throwaway lab, then
//! export-package. The report cites the package path, proposal
//! count/kind, and `auto_train=false` / no train invoked.
//! `estate pack cohesion-prove` composes the export-package stage
//! after fuel, decide, run, specialty-real, pack install, and
//! crew-session. The sibling prove stays callable alone.
//! Does not rewrite `examples/estate.yaml`. `READY_FOR_LIVE_TEST` stays no.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::control_plane_prove;
use crate::decision_prove;
use crate::decisions::{self, DecisionReceipt};

const LOCKED_CKSUM: &str = "43770130 3391";
const PACKAGE_SCHEMA: &str = "cell-one.improvement-package.v0";
const PROVE_SCHEMA: &str = "cell-one.improvement-export-prove.v0";
const PACKAGE_ID: &str = "improvement-from-decisions";
const PACKAGE_KIND: &str = "standing-improvement";
const PACKAGE_JSON: &str = "improvement-package.json";
const PACKAGE_YAML: &str = "improvement-package.yaml";
const KIND_SEAT: &str = "specialty-seat";
const KIND_DATASET: &str = "dataset";
const ACTION_ENRICH: &str = "enrich-prepare";

const SKIP_RESULTS: &[&str] = &["", "abstain", "xai_grok", "frontier_http", "local_slm"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct EnrichProposal {
    pub kind: String,
    pub binding_id: String,
    pub dataset: String,
    pub action: String,
    pub auto_train: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct JournalCite {
    pub path: String,
    pub receipts: usize,
    pub surface_authorize: usize,
    pub surface_convey: usize,
    pub surface_complete: usize,
    pub specialty_seats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ImprovementPackage {
    pub schema: String,
    pub id: String,
    pub kind: String,
    pub auto_train: bool,
    pub train_invoked: bool,
    pub ready_for_live_test: bool,
    pub live_pass_recorded: bool,
    pub live_sync: bool,
    pub journal: JournalCite,
    pub proposals: Vec<EnrichProposal>,
    pub note: String,
}

pub(crate) fn cmd_decisions_export_package(
    state_dir: &Path,
    out: &Path,
    root: &Path,
) -> Result<ImprovementPackage> {
    let root = root
        .canonicalize()
        .with_context(|| format!("refuse:root: {}", root.display()))?;
    // Always refuse cwd / root examples trees, even when `--root` has no
    // `examples/estate.yaml`. A decoy root must not open writes under
    // the locked examples tree.
    if refuses_examples_write(out, &root)? {
        bail!("refuse:out: export-package does not write examples/estate.yaml");
    }
    let locked = root.join("examples/estate.yaml");
    if locked.is_file() {
        let cksum = file_cksum(&locked)?;
        if !cksum.starts_with(LOCKED_CKSUM) {
            bail!("refuse:estate: examples/estate.yaml cksum is {cksum}, want {LOCKED_CKSUM}");
        }
        let before = fs::read(&locked)?;
        let package = write_package(state_dir, out)?;
        if fs::read(&locked)? != before {
            bail!("refuse:estate: examples/estate.yaml changed");
        }
        print_package_cite(&package, out);
        return Ok(package);
    }
    let package = write_package(state_dir, out)?;
    print_package_cite(&package, out);
    Ok(package)
}

/// Export-package stage for `estate pack cohesion-prove`.
///
/// Reads the lab decision journal already written by control-plane
/// (host-validate + runner) and writes the standing package under
/// `{out}/improvement`. Does not re-run host-validate-prove.
/// `improvement-export-prove` stays the standalone sibling.
pub(crate) fn run_export_stage(
    state_dir: &Path,
    out: &Path,
    root: &Path,
) -> Result<(ImprovementPackage, Value)> {
    let package_dir = out.join("improvement");
    let package = cmd_decisions_export_package(state_dir, &package_dir, root)?;
    require_host_validate_package(&package)?;
    let cite = export_cite(&package, &package_dir);
    Ok((package, cite))
}

pub(crate) fn export_cite(package: &ImprovementPackage, package_dir: &Path) -> Value {
    let kinds = proposal_kinds(&package.proposals);
    json!({
        "schema": PACKAGE_SCHEMA,
        "ok": true,
        "package_path": package_dir.join(PACKAGE_JSON).display().to_string(),
        "package_yaml": package_dir.join(PACKAGE_YAML).display().to_string(),
        "proposal_count": package.proposals.len(),
        "proposal_kinds": kinds,
        "auto_train": false,
        "train_invoked": false,
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "live_sync": false,
        "specialty_seats": package.journal.specialty_seats,
        "composed_by": "cohesion-prove",
        "note": "Standing improvement package from the lab decision journal. Proposes the next enrich (specialty seat / dataset). auto_train=false. Train not invoked. Not a live PASS.",
    })
}

pub(crate) fn cmd_decisions_improvement_export_prove(root: &Path, out: Option<&Path>) -> Result<()> {
    std::env::remove_var("CELL_LOCAL_ENDPOINT");
    std::env::remove_var("CELL_LOCAL_LIVE");
    let root = root
        .canonicalize()
        .with_context(|| format!("refuse:root: {}", root.display()))?;
    let locked = root.join("examples/estate.yaml");
    let before =
        fs::read(&locked).with_context(|| format!("refuse:estate: {}", locked.display()))?;
    let cksum_before = file_cksum(&locked)?;
    if !cksum_before.starts_with(LOCKED_CKSUM) {
        bail!("refuse:estate: examples/estate.yaml cksum is {cksum_before}, want {LOCKED_CKSUM}");
    }

    let (out_hint, wipe) = match out {
        Some(path) => (path.to_path_buf(), false),
        None => (default_out(), true),
    };
    if control_plane_prove::refuses_locked_target(&out_hint, &root, &locked)? {
        bail!("refuse:out: improvement-export-prove does not write examples/estate.yaml");
    }
    if wipe {
        let _ = fs::remove_dir_all(&out_hint);
    }
    fs::create_dir_all(&out_hint).with_context(|| format!("refuse:out: {}", out_hint.display()))?;
    let out = out_hint
        .canonicalize()
        .with_context(|| format!("refuse:out: {}", out_hint.display()))?;
    if control_plane_prove::refuses_locked_target(&out, &root, &locked)? {
        bail!("refuse:out: improvement-export-prove does not write examples/estate.yaml");
    }

    println!("improvement-export-prove: host-validate");
    decision_prove::cmd_decisions_host_validate_prove(&root, Some(&out))?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: host-validate rewrote examples/estate.yaml");
    }

    let state = out.join("state");
    let package_dir = out.join("improvement");
    println!("improvement-export-prove: export-package");
    let package = cmd_decisions_export_package(&state, &package_dir, &root)?;
    if fs::read(&locked)? != before {
        bail!("refuse:estate: export-package rewrote examples/estate.yaml");
    }
    require_host_validate_package(&package)?;

    let cksum_after = file_cksum(&locked)?;
    if cksum_after != cksum_before {
        bail!("refuse:estate: examples/estate.yaml cksum changed to {cksum_after}");
    }

    let kinds = proposal_kinds(&package.proposals);
    let json_path = package_dir.join(PACKAGE_JSON);
    let yaml_path = package_dir.join(PACKAGE_YAML);
    let body = json!({
        "schema": PROVE_SCHEMA,
        "ok": true,
        "ready_for_live_test": false,
        "live_pass_recorded": false,
        "live_sync": false,
        "auto_train": false,
        "train_invoked": false,
        "estate_cksum": cksum_after,
        "host_validate_report": out.join("host-validate-prove.json").display().to_string(),
        "package_path": json_path.display().to_string(),
        "package_yaml": yaml_path.display().to_string(),
        "package_schema": PACKAGE_SCHEMA,
        "proposal_count": package.proposals.len(),
        "proposal_kinds": kinds,
        "proposals": package.proposals,
        "surfaces": {
            "authorize": package.journal.surface_authorize,
            "convey": package.journal.surface_convey,
            "complete": package.journal.surface_complete,
        },
        "specialty_seats": package.journal.specialty_seats,
        "note": "Fixture prove. Reuses host-validate authorize / convey / complete --mock receipts, then writes a standing improvement package that proposes the next enrich (specialty seat / dataset). auto_train=false. Train not invoked. Not a live PASS."
    });
    write_prove_report(&out, &body)?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    println!(
        "package: {} proposals={} kinds={} auto_train=false train_invoked=no",
        json_path.display(),
        package.proposals.len(),
        kinds.join(",")
    );
    println!("improvement-export-prove: ok");
    println!("READY_FOR_LIVE_TEST: no");
    Ok(())
}

fn write_package(state_dir: &Path, out: &Path) -> Result<ImprovementPackage> {
    if looks_like_locked_estate(out) {
        bail!("refuse:out: export-package does not write examples/estate.yaml");
    }
    fs::create_dir_all(out).with_context(|| format!("refuse:out: {}", out.display()))?;
    let package = build_package(state_dir)?;
    refuse_invented_train(&package)?;
    let json_path = out.join(PACKAGE_JSON);
    let yaml_path = out.join(PACKAGE_YAML);
    let pretty = serde_json::to_string_pretty(&package)?;
    refuse_report_inventions(&pretty, "export-package")?;
    fs::write(&json_path, format!("{pretty}\n"))
        .with_context(|| format!("refuse:out: write {}", json_path.display()))?;
    let yaml = render_package_yaml(&package);
    if yaml.contains("auto_train: true") || yaml.contains("train_invoked: true") {
        bail!("refuse:improvement: yaml invented auto-train");
    }
    fs::write(&yaml_path, yaml).with_context(|| format!("refuse:out: write {}", yaml_path.display()))?;
    Ok(package)
}

fn build_package(state_dir: &Path) -> Result<ImprovementPackage> {
    let receipts = decisions::load_receipts(state_dir)?;
    let seats = specialty_seats(&receipts);
    let proposals = proposals_for_seats(&seats);
    let journal = journal_path(state_dir);
    Ok(ImprovementPackage {
        schema: PACKAGE_SCHEMA.into(),
        id: PACKAGE_ID.into(),
        kind: PACKAGE_KIND.into(),
        auto_train: false,
        train_invoked: false,
        ready_for_live_test: false,
        live_pass_recorded: false,
        live_sync: false,
        journal: JournalCite {
            path: journal.display().to_string(),
            receipts: receipts.len(),
            surface_authorize: receipts.iter().filter(|row| row.surface == "authorize").count(),
            surface_convey: receipts.iter().filter(|row| row.surface.is_empty()).count(),
            surface_complete: receipts.iter().filter(|row| row.surface == "complete").count(),
            specialty_seats: seats,
        },
        proposals,
        note: "Proposal only. Standing improvement package from the decision journal. Proposes the next enrich (specialty seat / dataset). auto_train=false. Does not train, apply, promote, or flip READY. READY_FOR_LIVE_TEST stays no.".into(),
    })
}

fn specialty_seats(receipts: &[DecisionReceipt]) -> Vec<String> {
    let mut seats = BTreeSet::new();
    for receipt in receipts {
        if receipt.validation != "ok" {
            continue;
        }
        if let Some(id) = specialty_id(&receipt.result) {
            seats.insert(id.to_string());
        }
    }
    seats.into_iter().collect()
}

fn specialty_id(raw: &str) -> Option<&str> {
    let id = raw.trim().to_ascii_lowercase();
    if SKIP_RESULTS.contains(&id.as_str())
        || matches!(
            id.as_str(),
            "xai-grok" | "frontier-http" | "local-slm"
        )
        || id.starts_with("refuse:")
    {
        return None;
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed)
}

fn proposals_for_seats(seats: &[String]) -> Vec<EnrichProposal> {
    let mut out = Vec::new();
    for seat in seats {
        out.push(EnrichProposal {
            kind: KIND_SEAT.into(),
            binding_id: seat.clone(),
            dataset: seat.clone(),
            action: ACTION_ENRICH.into(),
            auto_train: false,
            note: format!(
                "Propose next enrich for specialty seat {seat}. Does not train. auto_train=false."
            ),
        });
        out.push(EnrichProposal {
            kind: KIND_DATASET.into(),
            binding_id: seat.clone(),
            dataset: seat.clone(),
            action: ACTION_ENRICH.into(),
            auto_train: false,
            note: format!(
                "Propose next enrich dataset {seat} for that specialty seat. Does not train. auto_train=false."
            ),
        });
    }
    out
}

fn proposal_kinds(proposals: &[EnrichProposal]) -> Vec<String> {
    let mut kinds = BTreeSet::new();
    for row in proposals {
        kinds.insert(row.kind.clone());
    }
    kinds.into_iter().collect()
}

fn require_host_validate_package(package: &ImprovementPackage) -> Result<()> {
    if package.schema != PACKAGE_SCHEMA {
        bail!("refuse:improvement: package schema");
    }
    if package.auto_train || package.train_invoked {
        bail!("refuse:improvement: package invented auto-train");
    }
    if package.ready_for_live_test || package.live_pass_recorded || package.live_sync {
        bail!("refuse:improvement: package invented a live-test ready flag");
    }
    if package.journal.surface_authorize == 0
        || package.journal.surface_convey == 0
        || package.journal.surface_complete == 0
    {
        bail!("refuse:improvement: package missing host-validate surfaces");
    }
    let seats = &package.journal.specialty_seats;
    if !seats.iter().any(|id| id == "ag_news") || !seats.iter().any(|id| id == "rust_idiom") {
        bail!("refuse:improvement: package missing specialty seats ag_news / rust_idiom");
    }
    if package.proposals.is_empty() {
        bail!("refuse:improvement: package has no enrich proposals");
    }
    let kinds = proposal_kinds(&package.proposals);
    if !kinds.iter().any(|k| k == KIND_SEAT) || !kinds.iter().any(|k| k == KIND_DATASET) {
        bail!("refuse:improvement: package missing proposal kinds specialty-seat / dataset");
    }
    if package.proposals.iter().any(|row| row.auto_train) {
        bail!("refuse:improvement: a proposal invented auto-train");
    }
    if package.proposals.iter().any(|row| row.action != ACTION_ENRICH) {
        bail!("refuse:improvement: a proposal is not enrich-prepare");
    }
    Ok(())
}

fn refuse_invented_train(package: &ImprovementPackage) -> Result<()> {
    if package.auto_train || package.train_invoked {
        bail!("refuse:improvement: package invented auto-train");
    }
    if package.ready_for_live_test || package.live_pass_recorded || package.live_sync {
        bail!("refuse:improvement: package invented a live-test ready flag");
    }
    if package.proposals.iter().any(|row| row.auto_train) {
        bail!("refuse:improvement: a proposal invented auto-train");
    }
    Ok(())
}

fn write_prove_report(out: &Path, body: &Value) -> Result<()> {
    let pretty = serde_json::to_string_pretty(body)?;
    refuse_report_inventions(&pretty, "improvement")?;
    fs::write(out.join("improvement-export-prove.json"), format!("{pretty}\n"))?;
    Ok(())
}

fn refuse_report_inventions(pretty: &str, surface: &str) -> Result<()> {
    if pretty.split_whitespace().any(|word| word == "enforced") {
        bail!("refuse:{surface}: report invented enforced");
    }
    if pretty.contains("READY_FOR_LIVE_TEST: yes")
        || pretty.contains("\"ready_for_live_test\": true")
        || pretty.contains("\"live_pass_recorded\": true")
        || pretty.contains("\"live_sync\": true")
        || pretty.contains("\"auto_train\": true")
        || pretty.contains("\"train_invoked\": true")
    {
        bail!("refuse:{surface}: report invented a live-test, live_sync, or auto-train flag");
    }
    if pretty.contains("live PASS recorded") {
        bail!("refuse:{surface}: report invented a live PASS");
    }
    Ok(())
}

fn print_package_cite(package: &ImprovementPackage, out: &Path) {
    let kinds = proposal_kinds(&package.proposals);
    println!(
        "wrote improvement package {} ({}) proposals={} kinds={} auto_train=false train_invoked=no",
        out.join(PACKAGE_JSON).display(),
        out.join(PACKAGE_YAML).display(),
        package.proposals.len(),
        if kinds.is_empty() {
            "none".to_string()
        } else {
            kinds.join(",")
        }
    );
}

fn render_package_yaml(package: &ImprovementPackage) -> String {
    let mut out = String::new();
    out.push_str(&format!("schema: {}\n", yaml_scalar(&package.schema)));
    out.push_str(&format!("id: {}\n", yaml_scalar(&package.id)));
    out.push_str(&format!("kind: {}\n", yaml_scalar(&package.kind)));
    out.push_str("auto_train: false\n");
    out.push_str("train_invoked: false\n");
    out.push_str("ready_for_live_test: false\n");
    out.push_str("live_pass_recorded: false\n");
    out.push_str("live_sync: false\n");
    out.push_str("journal:\n");
    out.push_str(&format!("  path: {}\n", yaml_scalar(&package.journal.path)));
    out.push_str(&format!("  receipts: {}\n", package.journal.receipts));
    out.push_str(&format!(
        "  surface_authorize: {}\n",
        package.journal.surface_authorize
    ));
    out.push_str(&format!(
        "  surface_convey: {}\n",
        package.journal.surface_convey
    ));
    out.push_str(&format!(
        "  surface_complete: {}\n",
        package.journal.surface_complete
    ));
    out.push_str("  specialty_seats:\n");
    if package.journal.specialty_seats.is_empty() {
        out.push_str("    []\n");
    } else {
        for seat in &package.journal.specialty_seats {
            out.push_str(&format!("    - {}\n", yaml_scalar(seat)));
        }
    }
    out.push_str("proposals:\n");
    if package.proposals.is_empty() {
        out.push_str("  []\n");
    } else {
        for row in &package.proposals {
            out.push_str(&format!("  - kind: {}\n", yaml_scalar(&row.kind)));
            out.push_str(&format!("    binding_id: {}\n", yaml_scalar(&row.binding_id)));
            out.push_str(&format!("    dataset: {}\n", yaml_scalar(&row.dataset)));
            out.push_str(&format!("    action: {}\n", yaml_scalar(&row.action)));
            out.push_str("    auto_train: false\n");
            out.push_str(&format!("    note: {}\n", yaml_scalar(&row.note)));
        }
    }
    out.push_str(&format!("note: {}\n", yaml_scalar(&package.note)));
    out
}

fn yaml_scalar(raw: &str) -> String {
    if raw.is_empty() {
        return "\"\"".into();
    }
    let safe = raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | ' '));
    if safe && !raw.starts_with([' ', ':']) && !raw.contains(": ") && !raw.contains('\n') {
        raw.to_string()
    } else {
        format!("\"{}\"", raw.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn journal_path(state_dir: &Path) -> PathBuf {
    state_dir.join("decisions").join(decisions::JOURNAL_FILE)
}

fn refuses_examples_write(out: &Path, root: &Path) -> Result<bool> {
    if looks_like_locked_estate(out) {
        return Ok(true);
    }
    let cwd = std::env::current_dir().context("refuse:out: cwd")?;
    for base in [root.to_path_buf(), cwd] {
        let locked = base.join("examples").join("estate.yaml");
        if control_plane_prove::refuses_locked_target(out, &base, &locked)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn looks_like_locked_estate(out: &Path) -> bool {
    if out.components().any(|part| part.as_os_str() == "examples") {
        return true;
    }
    let text = out.to_string_lossy().replace('\\', "/");
    text == "examples"
        || text == "examples/estate.yaml"
        || text.ends_with("/examples")
        || text.ends_with("/examples/estate.yaml")
        || text.starts_with("examples/")
        || text.contains("/examples/")
}

fn file_cksum(path: &Path) -> Result<String> {
    let out = Command::new("cksum")
        .arg(path)
        .output()
        .context("refuse:estate: cksum")?;
    if !out.status.success() {
        bail!(
            "refuse:estate: cksum failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn default_out() -> PathBuf {
    let mut token = std::process::id().to_string();
    for needle in ["5090", "4090", "4080", "3090"] {
        token = token.replace(needle, "0000");
    }
    std::env::temp_dir().join(format!("cell-one-improvement-export-{token}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decisions::{CandidateSummary, RECEIPT_SCHEMA};

    fn receipt(seq: u64, surface: &str, result: &str, validation: &str) -> DecisionReceipt {
        DecisionReceipt {
            schema: RECEIPT_SCHEMA.into(),
            id: format!("r-{seq}"),
            seq,
            hop_id: if surface == "complete" {
                "model".into()
            } else {
                "model".into()
            },
            capability: result.into(),
            agent: Some("research".into()),
            surface: surface.into(),
            pack_id: None,
            handoff_from: None,
            handoff_to: None,
            package_id: None,
            routine_id: None,
            chain_id: None,
            handoffs: Vec::new(),
            stage: "validate".into(),
            candidates: vec![CandidateSummary { id: result.into() }],
            result: result.into(),
            validation: validation.into(),
            fallback: None,
            outcome: "allow".into(),
            completion_label: None,
            rejected: Vec::new(),
            session_id: None,
            session_turns: None,
            session_context: None,
        }
    }

    #[test]
    fn proposals_come_from_ok_specialty_results_not_frontier_or_abstain() {
        let receipts = vec![
            receipt(1, "authorize", "ag_news", "ok"),
            receipt(2, "complete", "rust_idiom", "ok"),
            receipt(3, "complete", "abstain", "ok"),
            receipt(4, "", "xai_grok", "ok"),
            receipt(5, "complete", "frontier_http", "ok"),
            receipt(6, "complete", "local_slm", "ok"),
            receipt(7, "complete", "ag_news", "stale"),
        ];
        let seats = specialty_seats(&receipts);
        assert_eq!(seats, vec!["ag_news".to_string(), "rust_idiom".to_string()]);
        let proposals = proposals_for_seats(&seats);
        assert_eq!(proposals.len(), 4);
        assert_eq!(proposals[0].kind, KIND_SEAT);
        assert_eq!(proposals[0].binding_id, "ag_news");
        assert_eq!(proposals[0].auto_train, false);
        assert_eq!(proposals[1].kind, KIND_DATASET);
        assert_eq!(proposals[1].dataset, "ag_news");
        assert_eq!(proposals[2].kind, KIND_SEAT);
        assert_eq!(proposals[2].binding_id, "rust_idiom");
        assert_eq!(proposals[3].kind, KIND_DATASET);
        assert!(proposals.iter().all(|row| !row.auto_train));
        assert_eq!(proposal_kinds(&proposals), vec![KIND_DATASET, KIND_SEAT]);
    }

    #[test]
    fn empty_journal_is_proposal_only_and_never_auto_train() {
        let package = ImprovementPackage {
            schema: PACKAGE_SCHEMA.into(),
            id: PACKAGE_ID.into(),
            kind: PACKAGE_KIND.into(),
            auto_train: false,
            train_invoked: false,
            ready_for_live_test: false,
            live_pass_recorded: false,
            live_sync: false,
            journal: JournalCite {
                path: "missing".into(),
                receipts: 0,
                surface_authorize: 0,
                surface_convey: 0,
                surface_complete: 0,
                specialty_seats: Vec::new(),
            },
            proposals: Vec::new(),
            note: "none".into(),
        };
        refuse_invented_train(&package).unwrap();
        let yaml = render_package_yaml(&package);
        assert!(yaml.contains("auto_train: false"), "{yaml}");
        assert!(yaml.contains("train_invoked: false"), "{yaml}");
        assert!(!yaml.contains("auto_train: true"), "{yaml}");
    }

    #[test]
    fn looks_like_locked_estate_refuses_examples_trees() {
        assert!(looks_like_locked_estate(Path::new("examples/estate.yaml")));
        assert!(looks_like_locked_estate(Path::new("/tmp/repo/examples/estate.yaml")));
        assert!(looks_like_locked_estate(Path::new("examples")));
        assert!(looks_like_locked_estate(Path::new("/tmp/repo/examples/improvement")));
        assert!(looks_like_locked_estate(Path::new("examples\\estate.yaml")));
        assert!(!looks_like_locked_estate(Path::new("/tmp/cell-one-improvement")));
        assert!(!looks_like_locked_estate(Path::new("/tmp/not-examples/estate.yaml")));
    }

    #[test]
    fn refuses_examples_write_when_root_lacks_estate_yaml() {
        let decoy = std::env::temp_dir().join(format!(
            "cell-one-improvement-decoy-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&decoy);
        fs::create_dir_all(&decoy).unwrap();
        let cwd = std::env::current_dir().unwrap();
        let under_cwd = cwd.join("examples").join("improvement-adversarial");
        assert!(
            refuses_examples_write(&under_cwd, &decoy).unwrap(),
            "cwd examples tree must refuse even when --root has no estate.yaml"
        );
        assert!(
            refuses_examples_write(Path::new("examples/estate.yaml"), &decoy).unwrap()
        );
        let throwaway = decoy.join("improvement");
        assert!(
            !refuses_examples_write(&throwaway, &decoy).unwrap(),
            "throwaway under decoy root must stay writable"
        );
        let _ = fs::remove_dir_all(&decoy);
    }
}
