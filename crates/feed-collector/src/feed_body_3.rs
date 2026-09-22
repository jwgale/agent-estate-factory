
/// After an explicit pack import: write a proposal pack. Never applies. Never rewrites the estate.
pub fn propose_enrich(
    drop_dir: &Path,
    accepted_dir: &Path,
    proposed_dir: &Path,
    id: &str,
    estate: &estate_schema::Estate,
) -> Result<(EnrichProposal, PathBuf), FeedError> {
    refuse_pack_id(id)?;
    let pack = match load_pack_loose(accepted_dir, id) {
        Ok(p) => p,
        Err(FeedError::MissingPack(_)) => load_pack_loose(drop_dir, id)?,
        Err(e) => return Err(e),
    };
    refuse_pack(&pack)?;
    refuse_frontier_source_on_estate(&pack.source_drivers, estate)?;
    if pack.promoted {
        return Err(FeedError::NoAutoPromote);
    }
    let current: Vec<String> = estate
        .enrich_packs
        .packs
        .iter()
        .map(|p| p.id.clone())
        .collect();
    let estate_bound = current.iter().any(|p| p == &pack.id);
    let mut proposed_estate_packs = current.clone();
    if !estate_bound {
        proposed_estate_packs.push(pack.id.clone());
        proposed_estate_packs.sort();
    }
    let proposal = EnrichProposal {
        schema: PROPOSAL_SCHEMA.into(),
        id: pack.id.clone(),
        curator: "jason".into(),
        policy: "manual".into(),
        auto_apply: false,
        source_pack: format!("{}.pack.json", pack.id),
        estate_name: estate.name.clone(),
        estate_hash: estate_schema::estate_hash(estate),
        current_estate_packs: current,
        proposed_estate_packs,
        diff: EnrichDiff {
            pack_id: pack.id.clone(),
            estate_bound,
            would_add_to_estate: !estate_bound,
            kinds: pack.kinds.clone(),
            agents: pack.agents.clone(),
            paths: pack.paths.clone(),
            path_counts: pack.path_counts.clone(),
            source_drivers: pack.source_drivers.clone(),
            note: if estate_bound {
                "Already listed on estate.enrich_packs. No estate edit required.".into()
            } else {
                "Jason must add this pack id to estate.enrich_packs by hand. This file is not applied."
                    .into()
            },
        },
        created_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        note: "Proposal only. auto_apply=false. Feed never applies this file. Control will not rewrite the estate.".into(),
    };
    std::fs::create_dir_all(proposed_dir)?;
    let path = proposed_dir.join(format!("{}.proposal.json", pack.id));
    let body = to_pretty_json(&proposal)?;
    std::fs::write(&path, body)?;
    std::fs::write(
        proposed_dir.join(format!("{}.proposal.md", pack.id)),
        render_proposal(&proposal),
    )?;
    write_proposal_index(proposed_dir)?;
    Ok((proposal, path))
}

pub fn materialize_from_feed(
    feed_dir: &Path,
    drop_dir: &Path,
    id: &str,
) -> Result<(PackManifest, PathBuf), FeedError> {
    refuse_pack_id(id)?;
    let events = read_events(feed_dir)?;
    let pack = pack_from_events(id, &events);
    let path = write_drop_pack(drop_dir, &pack)?;
    write_cursor(feed_dir, &cursor_from_events(&events, Some(id)))?;
    Ok((pack, path))
}
