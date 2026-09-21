//! Curator accept path. Copies a proposal into enrich_packs edit instructions.
//! Never rewrites estate.yaml. Never auto-applies.

use anyhow::{bail, Result};
use feed_collector::{
    refuse_curator, refuse_pack_id, EnrichProposal, FeedError, LOCKED_CURATOR,
};
use std::path::{Path, PathBuf};

pub const ACCEPT_SCHEMA: &str = "cell-one.enrich-accept.v0";

/// Hand-edit instructions. Never rewrites the estate. Never auto-applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrichAccept {
    pub schema: String,
    pub id: String,
    pub curator: String,
    pub policy: String,
    pub auto_apply: bool,
    pub applied_to_estate: bool,
    pub estate_hash: String,
    pub yaml_snippet: String,
    pub note: String,
}

pub fn render_accept(accept: &EnrichAccept) -> String {
    let mut out = String::from("Enrich accept (edit instructions only)\n");
    out.push_str("======================================\n");
    out.push_str(&format!("schema: {}\n", accept.schema));
    out.push_str(&format!("id: {}\n", accept.id));
    out.push_str(&format!("curator: {}\n", accept.curator));
    out.push_str(&format!("auto_apply: {}\n", accept.auto_apply));
    out.push_str(&format!("applied_to_estate: {}\n", accept.applied_to_estate));
    out.push_str(&format!("estate_hash: {}\n\n", accept.estate_hash));
    out.push_str("Paste under estate.enrich_packs (by hand)\n");
    out.push_str("----------------------------------------\n");
    out.push_str(&accept.yaml_snippet);
    out.push_str("\n");
    out.push_str(&accept.note);
    out.push('\n');
    out
}

/// Copy a proposal into enrich_packs *edit instructions*. Does not rewrite estate.yaml.
pub fn accept_proposal(
    proposed_dir: &Path,
    accepted_dir: &Path,
    id: &str,
    curator: &str,
    estate_curator: &str,
    estate: &estate_schema::Estate,
) -> Result<(EnrichAccept, PathBuf)> {
    refuse_curator(curator, estate_curator)?;
    refuse_pack_id(id)?;
    let src = proposed_dir.join(format!("{id}.proposal.json"));
    if !src.is_file() {
        bail!("refuse:missing-proposal: no proposal '{id}' in proposed/");
    }
    let proposal: EnrichProposal = serde_json::from_str(&std::fs::read_to_string(&src)?)
        .map_err(|e| FeedError::Parse(e.to_string()))?;
    if proposal.auto_apply {
        return Err(FeedError::NoAutoApply.into());
    }
    if proposal.curator != LOCKED_CURATOR {
        return Err(FeedError::WrongCurator {
            provided: proposal.curator,
            want: LOCKED_CURATOR.into(),
        }
        .into());
    }
    let yaml_snippet = format!(
        "  - id: {}\n    description: accepted proposal {} (paste by hand; factory will not rewrite the estate)\n",
        proposal.id, proposal.id
    );
    let accept = EnrichAccept {
        schema: ACCEPT_SCHEMA.into(),
        id: proposal.id.clone(),
        curator: LOCKED_CURATOR.into(),
        policy: "manual".into(),
        auto_apply: false,
        applied_to_estate: false,
        estate_hash: estate_schema::estate_hash(estate),
        yaml_snippet,
        note: "Jason pastes the snippet into estate.enrich_packs.packs. The factory will not rewrite the estate. auto_apply=false.".into(),
    };
    std::fs::create_dir_all(accepted_dir)?;
    let dest = accepted_dir.join(format!("{}.enrich-edit.md", proposal.id));
    std::fs::write(&dest, render_accept(&accept))?;
    let json = serde_json::json!({
        "schema": accept.schema,
        "id": accept.id,
        "curator": accept.curator,
        "policy": accept.policy,
        "auto_apply": accept.auto_apply,
        "applied_to_estate": accept.applied_to_estate,
        "estate_hash": accept.estate_hash,
        "yaml_snippet": accept.yaml_snippet,
        "note": accept.note,
    });
    let body = serde_json::to_string_pretty(&json)?;
    if body.trim().is_empty() {
        bail!("serialize: empty enrich-edit");
    }
    std::fs::write(
        accepted_dir.join(format!("{}.enrich-edit.json", proposal.id)),
        body,
    )?;
    Ok((accept, dest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use feed_collector::{materialize_from_feed, propose_enrich};

    #[test]
    fn accept_proposal_writes_edit_instructions_never_estate() {
        let feed = std::env::temp_dir().join(format!(
            "cell-one-accept-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&feed);
        let drop = feed.join("drop");
        let accepted = feed.join("accepted");
        let proposed = feed.join("proposed");
        materialize_from_feed(&feed.join("feed"), &drop, "overnight-traces").unwrap();
        let estate = estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml"))
            .unwrap();
        propose_enrich(&drop, &accepted, &proposed, "overnight-traces", &estate).unwrap();
        let before = serde_json::to_string(&estate.enrich_packs).unwrap();
        let missing = accept_proposal(
            &proposed,
            &accepted,
            "no-such",
            "jason",
            "jason",
            &estate,
        )
        .unwrap_err();
        assert!(missing.to_string().starts_with("refuse:missing-proposal"));
        let curator = accept_proposal(
            &proposed,
            &accepted,
            "overnight-traces",
            "robot",
            "jason",
            &estate,
        )
        .unwrap_err();
        assert!(curator.to_string().starts_with("refuse:curator"));
        let sku = accept_proposal(
            &proposed,
            &accepted,
            "local-5090",
            "jason",
            "jason",
            &estate,
        )
        .unwrap_err();
        assert!(sku.to_string().contains("SKU") || sku.to_string().contains("sku"));
        let (accept, dest) = accept_proposal(
            &proposed,
            &accepted,
            "overnight-traces",
            "jason",
            "jason",
            &estate,
        )
        .unwrap();
        assert!(!accept.auto_apply);
        assert!(!accept.applied_to_estate);
        assert!(dest.ends_with("overnight-traces.enrich-edit.md"));
        assert!(accept.yaml_snippet.contains("id: overnight-traces"));
        let edit = std::fs::read_to_string(accepted.join("overnight-traces.enrich-edit.json")).unwrap();
        assert!(!edit.trim().is_empty(), "accept must not write empty enrich-edit");
        assert!(edit.contains(ACCEPT_SCHEMA), "{edit}");
        assert_eq!(before, serde_json::to_string(&estate.enrich_packs).unwrap());
        let _ = std::fs::remove_dir_all(&feed);
    }
}
