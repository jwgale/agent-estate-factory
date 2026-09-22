struct BoundFrontier {
    present: bool,
    model: Option<String>,
}

fn bound_frontier(estate: Option<&Estate>) -> Result<BoundFrontier, SupervisorError> {
    let Some(estate) = estate else {
        return Ok(BoundFrontier {
            present: false,
            model: None,
        });
    };
    let mut models = Vec::new();
    let mut present = false;
    for binding in &estate.model_bindings {
        if binding.class != ModelClass::Frontier {
            continue;
        }
        present = true;
        let Some(model) = binding
            .params
            .get("model")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|model| !model.is_empty())
        else {
            continue;
        };
        if contains_sku(model) {
            return Err(SupervisorError::Other(format!(
                "refuse:frontier-model: binding model '{model}' encodes a hardware SKU"
            )));
        }
        if !models.iter().any(|have: &String| have == model) {
            models.push(model.to_string());
        }
    }
    if models.len() > 1 {
        return Err(SupervisorError::Other(
            "refuse:frontier-model: frontier bindings name more than one model".into(),
        ));
    }
    Ok(BoundFrontier {
        present,
        model: models.pop(),
    })
}

/// Missing file is `Ok(None)`. A present file is `Ok(Some(model))`, and an
/// empty model is `Some(None)`. Unreadable JSON refuses. The schema card
/// is not consulted.
fn catalog_frontier_model(path: &Path) -> Result<Option<Option<String>>, SupervisorError> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).map_err(|err| {
        SupervisorError::Other(format!(
            "refuse:frontier-model: cell catalog unreadable ({err}); schema card is not the binding"
        ))
    })?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|err| {
        SupervisorError::Other(format!(
            "refuse:frontier-model: cell catalog unreadable ({err}); schema card is not the binding"
        ))
    })?;
    let model = value
        .get("frontier")
        .and_then(|frontier| frontier.get("model"))
        .and_then(|model| model.as_str())
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(|model| model.to_string());
    Ok(Some(model))
}

fn show_model(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("-")
}

fn catalog_refuses(path: &Path, bound: &BoundFrontier) -> Result<Vec<String>, SupervisorError> {
    let catalog = match catalog_frontier_model(path)? {
        None => return Ok(Vec::new()),
        Some(model) => model,
    };
    if !bound.present {
        if catalog.is_some() {
            return Ok(vec![
                "refuse:frontier-invent: no frontier binding; will not invent a frontier catalog model"
                    .into(),
            ]);
        }
        return Ok(Vec::new());
    }
    if catalog != bound.model {
        return Ok(vec![format!(
            "refuse:frontier-model: cell catalog model={} disagrees with binding model={}; schema card is not the binding",
            show_model(&catalog),
            show_model(&bound.model)
        )]);
    }
    Ok(Vec::new())
}

fn json_has_frontier_driver(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(drivers) = map.get("source_drivers").and_then(|item| item.as_array()) {
                if drivers.iter().any(|driver| driver.as_str() == Some("frontier")) {
                    return true;
                }
            }
            map.values().any(json_has_frontier_driver)
        }
        serde_json::Value::Array(items) => items.iter().any(json_has_frontier_driver),
        _ => false,
    }
}

fn walk_frontier_driver(path: &Path, found: &mut bool) -> Result<(), SupervisorError> {
    if *found || !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            walk_frontier_driver(&entry?.path(), found)?;
            if *found {
                return Ok(());
            }
        }
        return Ok(());
    }
    if !path.is_file() {
        return Ok(());
    }
    let name = path.file_name().and_then(|item| item.to_str()).unwrap_or("");
    if !name.ends_with(".json") {
        return Ok(());
    }
    let text = std::fs::read_to_string(path).map_err(|err| {
        SupervisorError::Other(format!(
            "refuse:frontier-invent: json unreadable ({}): {err}",
            path.display()
        ))
    })?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|err| {
        SupervisorError::Other(format!(
            "refuse:frontier-invent: json unreadable ({}): {err}",
            path.display()
        ))
    })?;
    if json_has_frontier_driver(&value) {
        *found = true;
    }
    Ok(())
}

fn frontier_driver_refuses(
    roots: &[PathBuf],
    bound: &BoundFrontier,
) -> Result<Vec<String>, SupervisorError> {
    if bound.present {
        return Ok(Vec::new());
    }
    let mut found = false;
    for root in roots {
        walk_frontier_driver(root, &mut found)?;
        if found {
            return Ok(vec![
                "refuse:frontier-invent: no frontier binding; will not invent a frontier source_driver"
                    .into(),
            ]);
        }
    }
    Ok(Vec::new())
}

fn snapshot_sacred_refuses(
    cell_dir: &Path,
    estate: Option<&Estate>,
) -> Result<Vec<String>, SupervisorError> {
    if !cell_dir.join("desired-snapshot.yaml").is_file() {
        return Ok(Vec::new());
    }
    let Some(snapshot) = load_desired_snapshot(cell_dir)? else {
        return Ok(Vec::new());
    };
    if sacred_mismatch(&sacred_id_set(Some(&snapshot)), &sacred_id_set(estate)) {
        return Ok(vec![
            "refuse:sacred-mismatch: desired snapshot sacred exclusions do not match this estate"
                .into(),
        ]);
    }
    Ok(Vec::new())
}

/// A missing placement file is not a spawned cloud lease. A present file
/// that does not parse is a refuse before the archive. A spawned
/// cloud-agent lease is `refuse:cloud-spawned` so the backup meta cannot
/// record `cloud_agent_spawned: false` over that file.
fn placement_spawn_refuses(cell_dir: &Path) -> Result<Vec<String>, SupervisorError> {
    let path = cell_dir.join("placement-actual.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let Some(actual) = load_placements(cell_dir)? else {
        return Ok(Vec::new());
    };
    let mut hits = Vec::new();
    for lease in &actual.leases {
        if lease.kind == "cloud-agent" && lease.spawned {
            hits.push(format!(
                "refuse:cloud-spawned: lease '{}' is spawned",
                lease.placement_id
            ));
        }
    }
    Ok(hits)
}

fn cell_inconsistencies(
    cell_dir: &Path,
    plans_dir: Option<&Path>,
    estate: Option<&Estate>,
) -> Result<Vec<String>, SupervisorError> {
    let bound = bound_frontier(estate)?;
    let mut refuses = catalog_refuses(&cell_dir.join("catalog.json"), &bound)?;
    refuses.extend(snapshot_sacred_refuses(cell_dir, estate)?);
    let mut roots = vec![cell_dir.join("feed")];
    if let Some(plans) = plans_dir {
        roots.push(plans.to_path_buf());
    }
    refuses.extend(frontier_driver_refuses(&roots, &bound)?);
    refuses.extend(placement_spawn_refuses(cell_dir)?);
    Ok(refuses)
}
