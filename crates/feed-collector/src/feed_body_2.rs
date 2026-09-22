
/// Index label after the tag matches the counts. Empty stays `-`.
/// A missing tag with a nonzero frontier or local count is refuse, not `-`.
fn index_drivers_label(drivers: &[String], counts: &PathCounts) -> Result<String, FeedError> {
    refuse_source_driver_list(drivers, counts)?;
    Ok(if drivers.is_empty() {
        "-".into()
    } else {
        drivers.join(",")
    })
}

pub fn write_drop_pack(drop_dir: &Path, pack: &PackManifest) -> Result<PathBuf, FeedError> {
    refuse_pack(pack)?;
    std::fs::create_dir_all(drop_dir)?;
    let path = drop_dir.join(format!("{}.pack.json", pack.id));
    std::fs::write(&path, to_pretty_json(pack)?)?;
    write_pack_index(drop_dir)?;
    Ok(path)
}

pub fn write_pack_index(drop_dir: &Path) -> Result<PathBuf, FeedError> {
    std::fs::create_dir_all(drop_dir)?;
    let packs = list_drop_packs(drop_dir)?;
    let mut md = String::from(
        "# Pack drop zone\n\nCandidates only. curator=jason policy=manual. Import is explicit apply. Promote fails. Pack ids must not encode a hardware SKU.\n\n",
    );
    if packs.is_empty() {
        md.push_str("(no candidate packs)\n");
    } else {
        for pack in &packs {
            let drivers = index_drivers_label(&pack.source_drivers, &pack.path_counts)?;
            md.push_str(&format!(
                "- `{}.pack.json` events={} frontier={} local={} proxy={} drivers={} promoted={} host_class={} schema={}\n",
                pack.id,
                pack.from_events,
                pack.path_counts.frontier,
                pack.path_counts.local,
                pack.path_counts.proxy,
                drivers,
                pack.promoted,
                pack.host_class,
                pack.schema
            ));
        }
    }
    let path = drop_dir.join("INDEX.md");
    std::fs::write(&path, md)?;
    Ok(path)
}

pub fn list_drop_packs(drop_dir: &Path) -> Result<Vec<PackManifest>, FeedError> {
    if !drop_dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut names: Vec<PathBuf> = std::fs::read_dir(drop_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("json")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|n| n.ends_with(".pack.json"))
                    .unwrap_or(false)
        })
        .collect();
    names.sort();
    for path in names {
        let text = std::fs::read_to_string(&path)?;
        let pack: PackManifest = serde_json::from_str(&text)
            .map_err(|e| FeedError::Parse(format!("{}: {e}", path.display())))?;
        out.push(pack);
    }
    Ok(out)
}

/// Always fails. Auto-promote is locked off. Use `import_pack` (explicit apply).
pub fn refuse_promote(_id: &str) -> Result<(), FeedError> {
    Err(FeedError::NoAutoPromote)
}

pub fn load_pack(dir: &Path, id: &str) -> Result<PackManifest, FeedError> {
    let path = dir.join(format!("{id}.pack.json"));
    if !path.exists() {
        return Err(FeedError::Parse(format!("missing pack {}", path.display())));
    }
    let text = std::fs::read_to_string(&path)?;
    serde_json::from_str(&text).map_err(|e| FeedError::Parse(format!("{}: {e}", path.display())))
}

/// Explicit Feed→Control import. Copies the candidate into `accepted/`.
/// Does not rewrite the estate file. `estate_bound` is true only if Jason
/// already listed the id on `enrich_packs.packs`. A frontier `source_driver`
/// with no frontier binding is `refuse:frontier-invent` before any write.
pub fn import_pack(
    drop_dir: &Path,
    accepted_dir: &Path,
    id: &str,
    estate_pack_ids: &[String],
    estate: &estate_schema::Estate,
) -> Result<(ImportedPack, PathBuf), FeedError> {
    import_pack_for(
        drop_dir,
        accepted_dir,
        id,
        estate_pack_ids,
        LOCKED_CURATOR,
        LOCKED_CURATOR,
        estate,
    )
}

/// Refuse curator / id / raw secret / pack schema / invented frontier driver
/// without writing accepted files. The schema card is not a binding.
pub fn refuse_import_pack(
    drop_dir: &Path,
    id: &str,
    curator: &str,
    estate_curator: &str,
    estate: &estate_schema::Estate,
) -> Result<PackManifest, FeedError> {
    refuse_curator(curator, estate_curator)?;
    refuse_pack_id(id)?;
    let source = drop_dir.join(format!("{id}.pack.json"));
    let raw = std::fs::read_to_string(&source)
        .map_err(|_| FeedError::MissingPack(id.to_string()))?;
    refuse_raw_secrets(&raw)?;
    let pack = load_pack(drop_dir, id)?;
    refuse_pack(&pack)?;
    if pack.promoted {
        return Err(FeedError::NoAutoPromote);
    }
    refuse_frontier_source_on_estate(&pack.source_drivers, estate)?;
    Ok(pack)
}

/// Explicit Feed→Control import gated on the locked curator.
/// Frontier invent refuses before the accepted pack, the redaction report,
/// or an index rewrite.
pub fn import_pack_for(
    drop_dir: &Path,
    accepted_dir: &Path,
    id: &str,
    estate_pack_ids: &[String],
    curator: &str,
    estate_curator: &str,
    estate: &estate_schema::Estate,
) -> Result<(ImportedPack, PathBuf), FeedError> {
    let mut pack = refuse_import_pack(drop_dir, id, curator, estate_curator, estate)?;
    let source = drop_dir.join(format!("{id}.pack.json"));
    let raw = std::fs::read_to_string(&source)
        .map_err(|_| FeedError::MissingPack(id.to_string()))?;
    pack.promoted = false;
    pack.policy = "manual".into();
    pack.curator = "jason".into();
    pack.note = scrub_pii(&pack.note);
    let report = redaction_report(&raw);
    let estate_bound = estate_pack_ids.iter().any(|p| p == &pack.id);
    let imported = ImportedPack {
        pack: pack.clone(),
        estate_bound,
        imported_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        source_path: source.display().to_string(),
    };
    std::fs::create_dir_all(accepted_dir)?;
    let path = accepted_dir.join(format!("{id}.pack.json"));
    let written = to_pretty_json(&imported)?;
    refuse_raw_secrets(&written)?;
    std::fs::write(&path, written)?;
    write_redaction_report(
        &accepted_dir.join(format!("{id}.redaction.json")),
        &report,
    )?;
    append_import_audit(
        accepted_dir,
        &ImportAudit {
            imported_at: imported.imported_at.clone(),
            pack_id: imported.pack.id.clone(),
            estate_bound: imported.estate_bound,
            source_path: imported.source_path.clone(),
            promoted: imported.pack.promoted,
        },
    )?;
    write_pack_index(drop_dir)?;
    Ok((imported, path))
}

/// Open enrich proposals. Never auto-applied.
pub fn list_open_proposals(proposed_dir: &Path) -> Result<Vec<String>, FeedError> {
    if !proposed_dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    let mut names: Vec<PathBuf> = std::fs::read_dir(proposed_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|n| n.ends_with(".proposal.json"))
                .unwrap_or(false)
        })
        .collect();
    names.sort();
    for path in names {
        let stem = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .trim_end_matches(".proposal.json");
        if !stem.is_empty() {
            ids.push(stem.to_string());
        }
    }
    Ok(ids)
}

pub const PROPOSAL_SCHEMA: &str = "cell-one.enrich-proposal.v0";

/// Diff Jason reviews. Never applied by the factory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnrichDiff {
    pub pack_id: String,
    pub estate_bound: bool,
    pub would_add_to_estate: bool,
    pub kinds: Vec<String>,
    pub agents: Vec<String>,
    pub paths: Vec<String>,
    pub path_counts: PathCounts,
    /// Copied from the pack. Not applied.
    #[serde(default)]
    pub source_drivers: Vec<String>,
    pub note: String,
}

/// Proposal pack. `auto_apply` is always false. Not estate SoT.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnrichProposal {
    #[serde(default = "default_proposal_schema")]
    pub schema: String,
    pub id: String,
    pub curator: String,
    pub policy: String,
    pub auto_apply: bool,
    pub source_pack: String,
    pub estate_name: String,
    pub estate_hash: String,
    pub current_estate_packs: Vec<String>,
    pub proposed_estate_packs: Vec<String>,
    pub diff: EnrichDiff,
    pub created_at: String,
    pub note: String,
}

fn default_proposal_schema() -> String {
    PROPOSAL_SCHEMA.into()
}

/// Always fails. Proposals are never applied.
pub fn refuse_apply_proposal(_id: &str) -> Result<(), FeedError> {
    Err(FeedError::NoAutoApply)
}

fn load_pack_loose(dir: &Path, id: &str) -> Result<PackManifest, FeedError> {
    let path = dir.join(format!("{id}.pack.json"));
    if !path.exists() {
        return Err(FeedError::MissingPack(id.to_string()));
    }
    let text = std::fs::read_to_string(&path)?;
    if let Ok(pack) = serde_json::from_str::<PackManifest>(&text) {
        if !pack.id.is_empty() {
            return Ok(pack);
        }
    }
    if let Ok(imported) = serde_json::from_str::<ImportedPack>(&text) {
        return Ok(imported.pack);
    }
    Err(FeedError::Parse(format!(
        "{}: not a pack or imported pack",
        path.display()
    )))
}

pub fn render_proposal(proposal: &EnrichProposal) -> String {
    let mut out = String::from("Enrich proposal (curator Jason)\n");
    out.push_str("==============================\n");
    out.push_str(&format!("schema: {}\n", proposal.schema));
    out.push_str(&format!("id: {}\n", proposal.id));
    out.push_str(&format!("auto_apply: {}\n", proposal.auto_apply));
    out.push_str(&format!("source_pack: {}\n", proposal.source_pack));
    out.push_str(&format!(
        "estate: {} ({})\n",
        proposal.estate_name, proposal.estate_hash
    ));
    out.push_str(&format!(
        "current packs: {}\n",
        if proposal.current_estate_packs.is_empty() {
            "(none)".into()
        } else {
            proposal.current_estate_packs.join(", ")
        }
    ));
    out.push_str(&format!(
        "proposed packs: {}\n\n",
        if proposal.proposed_estate_packs.is_empty() {
            "(none)".into()
        } else {
            proposal.proposed_estate_packs.join(", ")
        }
    ));
    out.push_str("Diff summary\n------------\n");
    out.push_str(&format!("  pack: {}\n", proposal.diff.pack_id));
    out.push_str(&format!("  estate_bound: {}\n", proposal.diff.estate_bound));
    out.push_str(&format!(
        "  would_add_to_estate: {}\n",
        proposal.diff.would_add_to_estate
    ));
    out.push_str(&format!(
        "  kinds: {}\n",
        if proposal.diff.kinds.is_empty() {
            "(none)".into()
        } else {
            proposal.diff.kinds.join(", ")
        }
    ));
    out.push_str(&format!(
        "  agents: {}\n",
        if proposal.diff.agents.is_empty() {
            "(none)".into()
        } else {
            proposal.diff.agents.join(", ")
        }
    ));
    out.push_str(&format!(
        "  paths: {}\n",
        if proposal.diff.paths.is_empty() {
            "(none)".into()
        } else {
            proposal.diff.paths.join(", ")
        }
    ));
    out.push_str(&format!(
        "  path_counts: frontier={} local={} proxy={} other={}\n",
        proposal.diff.path_counts.frontier,
        proposal.diff.path_counts.local,
        proposal.diff.path_counts.proxy,
        proposal.diff.path_counts.other
    ));
    out.push_str(&format!(
        "  source_drivers: {}\n",
        if proposal.diff.source_drivers.is_empty() {
            "(none)".into()
        } else {
            proposal.diff.source_drivers.join(", ")
        }
    ));
    out.push_str(&format!("\n{}\n", proposal.note));
    out.push_str(&format!("{}\n", proposal.diff.note));
    out
}

pub fn write_proposal_index(proposed_dir: &Path) -> Result<PathBuf, FeedError> {
    std::fs::create_dir_all(proposed_dir)?;
    let mut names: Vec<PathBuf> = if proposed_dir.exists() {
        std::fs::read_dir(proposed_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|n| n.ends_with(".proposal.json"))
                    .unwrap_or(false)
            })
            .collect()
    } else {
        Vec::new()
    };
    names.sort();
    let mut md = String::from(
        "# Enrich proposals\n\nNever auto-applied. curator=jason policy=manual. Jason reviews the diff and edits `estate.enrich_packs` by hand. The factory will not apply these files.\n\n",
    );
    if names.is_empty() {
        md.push_str("(no proposals)\n");
    } else {
        for path in &names {
            let text = std::fs::read_to_string(path)?;
            let p: EnrichProposal = serde_json::from_str(&text).map_err(|e| {
                FeedError::Parse(format!("{}: {e}", path.display()))
            })?;
            let drivers = index_drivers_label(&p.diff.source_drivers, &p.diff.path_counts)?;
            md.push_str(&format!(
                "- `{}.proposal.json` estate_bound={} would_add={} auto_apply={} drivers={} source={}\n",
                p.id,
                p.diff.estate_bound,
                p.diff.would_add_to_estate,
                p.auto_apply,
                drivers,
                p.source_pack
            ));
        }
    }
    let path = proposed_dir.join("INDEX.md");
    std::fs::write(&path, md)?;
    Ok(path)
}
