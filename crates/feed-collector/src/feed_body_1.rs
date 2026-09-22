
pub fn refuse_raw_secrets(text: &str) -> Result<(), FeedError> {
    if redaction_report(text).raw_secrets_found > 0 {
        return Err(FeedError::RawSecret);
    }
    Ok(())
}

fn refuse_empty_blob(body: &str) -> Result<(), FeedError> {
    if body.trim().is_empty() {
        return Err(FeedError::Parse("serialize: empty".into()));
    }
    Ok(())
}

fn to_json<T: Serialize>(value: &T) -> Result<String, FeedError> {
    let line = serde_json::to_string(value).map_err(|e| FeedError::Parse(format!("serialize: {e}")))?;
    refuse_empty_blob(&line)?;
    Ok(line)
}

fn to_pretty_json<T: Serialize>(value: &T) -> Result<String, FeedError> {
    let body =
        serde_json::to_string_pretty(value).map_err(|e| FeedError::Parse(format!("serialize: {e}")))?;
    refuse_empty_blob(&body)?;
    Ok(body)
}

/// Journal / audit line. Inventing `"{}"` on serialize failure is refuse.
fn to_jsonl_line<T: Serialize>(value: &T) -> Result<String, FeedError> {
    let line = to_json(value)?;
    if line.trim() == "{}" {
        return Err(FeedError::Parse("serialize: empty object".into()));
    }
    Ok(line)
}

/// Kind counts only. Refuses if the serialized report itself contains a raw secret.
pub fn write_redaction_report(path: &Path, report: &RedactionReport) -> Result<(), FeedError> {
    let blob = to_pretty_json(report)?;
    refuse_raw_secrets(&blob)?;
    for kind in report.kinds.keys() {
        if looks_pii_token(kind) {
            return Err(FeedError::RawSecret);
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, blob)?;
    Ok(())
}

pub fn is_pack_schema(schema: &str) -> bool {
    schema == "cell-one.pack.v0" || schema == "cell-one.specialist-pack.v0"
}

pub fn append_event(dir: &Path, event: &ScrubbedEvent) -> Result<(), FeedError> {
    std::fs::create_dir_all(dir)?;
    let mut ev = event.clone();
    if ev.ts.is_empty() {
        ev.ts = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    }
    if let Some(note) = ev.note.take() {
        ev.note = Some(scrub_pii(&note));
    }
    refuse_event(&ev)?;
    let line = to_jsonl_line(&ev)?;
    let path = dir.join("events.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")?;
    let events = read_events(dir)?;
    write_cursor(dir, &cursor_from_events(&events, None))?;
    Ok(())
}

pub fn cursor_path(dir: &Path) -> PathBuf {
    dir.join("feed-cursor.json")
}

pub fn load_cursor(dir: &Path) -> Result<Option<FeedCursor>, FeedError> {
    let path = cursor_path(dir);
    if !path.exists() {
        return Ok(None);
    }
    if !path.is_file() {
        return Err(FeedError::Parse(format!(
            "feed-cursor.json: {} is not a file",
            path.display()
        )));
    }
    let text = std::fs::read_to_string(&path)?;
    let cursor = serde_json::from_str(&text)
        .map_err(|e| FeedError::Parse(format!("feed-cursor.json: {e}")))?;
    Ok(Some(cursor))
}

pub fn write_cursor(dir: &Path, cursor: &FeedCursor) -> Result<PathBuf, FeedError> {
    std::fs::create_dir_all(dir)?;
    let path = cursor_path(dir);
    let body = to_pretty_json(cursor)?;
    std::fs::write(&path, body)?;
    Ok(path)
}

pub fn cursor_from_events(events: &[ScrubbedEvent], packed_id: Option<&str>) -> FeedCursor {
    FeedCursor {
        schema: default_cursor_schema(),
        events: events.len(),
        last_ts: events.last().map(|e| e.ts.clone()),
        last_kind: events.last().map(|e| e.kind.clone()),
        packed_id: packed_id.map(|s| s.to_string()),
        updated_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    }
}

pub fn append_import_audit(accepted_dir: &Path, audit: &ImportAudit) -> Result<PathBuf, FeedError> {
    std::fs::create_dir_all(accepted_dir)?;
    let path = accepted_dir.join("import-audit.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let line = to_jsonl_line(audit)?;
    writeln!(file, "{line}")?;
    Ok(path)
}

pub fn read_events(dir: &Path) -> Result<Vec<ScrubbedEvent>, FeedError> {
    let path = dir.join("events.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let ev: ScrubbedEvent = serde_json::from_str(line)
            .map_err(|e| FeedError::Parse(format!("events.jsonl line {}: {e}", i + 1)))?;
        out.push(ev);
    }
    Ok(out)
}

pub fn pack_from_events(id: &str, events: &[ScrubbedEvent]) -> PackManifest {
    let mut kinds = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut agents = BTreeSet::new();
    let mut path_counts = PathCounts::default();
    for ev in events {
        kinds.insert(ev.kind.clone());
        if let Some(agent) = &ev.agent_id {
            agents.insert(agent.clone());
        }
        match classify_path(ev) {
            "frontier" => {
                paths.insert("frontier".into());
                path_counts.frontier += 1;
            }
            "local" => {
                paths.insert("local".into());
                path_counts.local += 1;
            }
            "proxy" => {
                paths.insert("proxy".into());
                path_counts.proxy += 1;
            }
            other => {
                if let Some(class) = &ev.object_class {
                    paths.insert(class.clone());
                }
                let _ = other;
                path_counts.other += 1;
            }
        }
    }
    PackManifest {
        id: id.to_string(),
        version: 0,
        schema: default_pack_schema(),
        curator: "jason".into(),
        policy: "manual".into(),
        promoted: false,
        source: "feed".into(),
        from_events: events.len(),
        kinds: kinds.into_iter().collect(),
        paths: paths.into_iter().collect(),
        agents: agents.into_iter().collect(),
        host_class: default_host_class(),
        job: Some("policy-precheck".into()),
        adapter: None,
        path_counts,
        source_paths: vec!["feed/events.jsonl".into()],
        model_hint: None,
        source_drivers: source_drivers_from_events(events),
        host_class_affinity: Some(default_host_class()),
        created_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        note: "Candidate only. Import is explicit apply. Jason still adds the id to estate.enrich_packs by hand. Feed never auto-promotes.".into(),
    }
}

pub fn refuse_pack_id(id: &str) -> Result<(), FeedError> {
    if !estate_schema::is_slug(id) {
        return Err(FeedError::BadId(id.to_string()));
    }
    if estate_schema::contains_sku(id) {
        return Err(FeedError::SkuBanned(id.to_string()));
    }
    Ok(())
}

pub fn refuse_pack(pack: &PackManifest) -> Result<(), FeedError> {
    refuse_pack_id(&pack.id)?;
    if pack.promoted {
        return Err(FeedError::NoAutoPromote);
    }
    if pack.policy != "manual" || pack.curator != "jason" {
        return Err(FeedError::NoAutoPromote);
    }
    if !is_pack_schema(&pack.schema) {
        return Err(FeedError::BadSchema(pack.schema.clone()));
    }
    if !estate_schema::is_host_class(&pack.host_class) {
        return Err(FeedError::BadHostClass(pack.host_class.clone()));
    }
    if let Some(hint) = pack.model_hint.as_deref() {
        if !hint.is_empty()
            && (!estate_schema::is_slug(hint) || estate_schema::contains_sku(hint))
        {
            return Err(FeedError::BadModelHint(hint.to_string()));
        }
    }
    if let Some(affinity) = pack.host_class_affinity.as_deref() {
        if !affinity.is_empty() && !estate_schema::is_host_class(affinity) {
            return Err(FeedError::BadHostClass(affinity.to_string()));
        }
    }
    refuse_source_drivers(pack)?;
    for path in &pack.source_paths {
        if path.is_empty() {
            continue;
        }
        if path.starts_with('/') || path.contains("..") || estate_schema::contains_sku(path) {
            return Err(FeedError::BadSourcePath(path.clone()));
        }
    }
    let blob = to_json(pack)?;
    refuse_raw_secrets(&blob)?;
    refuse_raw_secrets(&pack.note)?;
    Ok(())
}

fn refuse_source_drivers(pack: &PackManifest) -> Result<(), FeedError> {
    refuse_source_driver_list(&pack.source_drivers, &pack.path_counts)
}

/// `frontier` / `local` only, sorted, unique, and matched to `path_counts`.
/// Empty is valid when both counts are zero. Does not invent a class.
pub fn refuse_source_driver_list(
    drivers: &[String],
    counts: &PathCounts,
) -> Result<(), FeedError> {
    let mut prev = "";
    for driver in drivers {
        if driver != "frontier" && driver != "local" {
            return Err(FeedError::BadSourceDriver(format!(
                "{driver} must be frontier or local"
            )));
        }
        if driver.as_str() <= prev {
            return Err(FeedError::BadSourceDriver(format!(
                "{driver} out of order or duplicated"
            )));
        }
        prev = driver.as_str();
    }
    if counts.frontier > 0 && !drivers.iter().any(|d| d == "frontier") {
        return Err(FeedError::BadSourceDriver(
            "frontier events are missing from source_drivers".into(),
        ));
    }
    if counts.local > 0 && !drivers.iter().any(|d| d == "local") {
        return Err(FeedError::BadSourceDriver(
            "local events are missing from source_drivers".into(),
        ));
    }
    if drivers.iter().any(|d| d == "frontier") && counts.frontier == 0 {
        return Err(FeedError::BadSourceDriver(
            "frontier tag has no frontier events".into(),
        ));
    }
    if drivers.iter().any(|d| d == "local") && counts.local == 0 {
        return Err(FeedError::BadSourceDriver(
            "local tag has no local events".into(),
        ));
    }
    Ok(())
}

/// `source_drivers` may name `frontier` only when the estate has a frontier
/// binding. Does not add a driver and does not copy the schema card.
pub fn refuse_frontier_source_on_estate(
    drivers: &[String],
    estate: &estate_schema::Estate,
) -> Result<(), FeedError> {
    if !drivers.iter().any(|d| d == "frontier") {
        return Ok(());
    }
    let bound = estate
        .model_bindings
        .iter()
        .any(|b| b.class == estate_schema::ModelClass::Frontier);
    if bound {
        return Ok(());
    }
    Err(FeedError::FrontierInvent)
}

/// Load the pack propose would copy, then refuse an invented frontier driver.
/// Does not write a proposal.
pub fn refuse_propose_frontier_invent(
    drop_dir: &Path,
    accepted_dir: &Path,
    id: &str,
    estate: &estate_schema::Estate,
) -> Result<(), FeedError> {
    refuse_pack_id(id)?;
    let pack = match load_pack_loose(accepted_dir, id) {
        Ok(p) => p,
        Err(FeedError::MissingPack(_)) => load_pack_loose(drop_dir, id)?,
        Err(e) => return Err(e),
    };
    refuse_pack(&pack)?;
    refuse_frontier_source_on_estate(&pack.source_drivers, estate)
}

/// Read the proposal accept would copy. Missing file is not this refuse.
/// Does not write enrich-edit instructions.
pub fn refuse_accept_frontier_invent(
    proposed_dir: &Path,
    id: &str,
    estate: &estate_schema::Estate,
) -> Result<(), FeedError> {
    let src = proposed_dir.join(format!("{id}.proposal.json"));
    if !src.is_file() {
        return Ok(());
    }
    let proposal: EnrichProposal = serde_json::from_str(&std::fs::read_to_string(&src)?)
        .map_err(|e| FeedError::Parse(e.to_string()))?;
    refuse_frontier_source_on_estate(&proposal.diff.source_drivers, estate)
}
