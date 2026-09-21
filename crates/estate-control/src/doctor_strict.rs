//! Pre-merge operator checks. `estate doctor --strict`.
//! Local only. No live Mac / GPU. Does not spawn. Does not auto-apply.

use anyhow::{bail, Result};
use estate_schema::{
    is_sacred_name, load_estate, load_sacred_file, locked_sacred_ids, normalize_name, ModelClass,
};
use std::fs;
use std::path::Path;

const HASH_LOCK: &str = "sha256:dcd7164f04c83f514185e77d2d4f6c23cae6dbb27a9b5da96a28ba1f3c724930";
const DEMO: &str = "examples/fixtures/dual-layer-demo.yaml";
const REFUSE_BLEED: &str = "examples/fixtures/refuse-sanctum-as-cyera.yaml";
const OMIT_LOCKED: &str = "examples/fixtures/sacred-omit-locked.yaml";
const FLOOR_NEEDLES: &[&str] = &["xai", "grok", "5090", "cyera"];

pub(crate) fn cmd_doctor_strict(root: &Path, state_dir: &Path) -> Result<()> {
    crate::watch::cmd_doctor(root, state_dir)?;
    println!("\nStrict (pre-merge)");
    println!("------------------");
    let mut fails: Vec<String> = Vec::new();

    check_compile_only_ci(root, &mut fails);
    check_locked_sacred_file(root, &mut fails);
    check_dual_layer_demo(root, &mut fails);
    check_refuse_fixtures(root, &mut fails);
    check_gate90_alias(root, &mut fails);
    check_feed_loop(root, &mut fails);
    check_day90_plus(root, &mut fails);
    check_estate_hash_lock(root, &mut fails);
    check_floor_no_vendor(root, &mut fails);

    if fails.is_empty() {
        println!("  ok    pre-merge operator checks");
        Ok(())
    } else {
        for fail in &fails {
            println!("  FAIL  {fail}");
        }
        bail!("doctor --strict failed ({} check(s))", fails.len());
    }
}

fn check_compile_only_ci(root: &Path, fails: &mut Vec<String>) {
    let path = root.join(".github/workflows/ci.yml");
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            println!("  FAIL  missing .github/workflows/ci.yml");
            fails.push("missing compile-only ci.yml".into());
            return;
        }
    };
    match compile_only_ci(&text) {
        Ok(()) => println!("  ok    ci.yml compile-only (no cargo test)"),
        Err(err) => {
            println!("  FAIL  {err}");
            fails.push(err);
        }
    }
}

fn check_locked_sacred_file(root: &Path, fails: &mut Vec<String>) {
    let path = root.join("policy/sacred.yaml");
    let file = match load_sacred_file(&path) {
        Ok(f) => f,
        Err(err) => {
            println!("  FAIL  {err}");
            fails.push(err);
            return;
        }
    };
    let declared: Vec<String> = file
        .locked
        .iter()
        .map(|e| normalize_name(&e.id))
        .collect();
    let mut missing = Vec::new();
    for id in locked_sacred_ids() {
        if !declared.iter().any(|d| d == &normalize_name(id)) {
            missing.push(id.to_string());
        }
    }
    if missing.is_empty() {
        println!("  ok    policy/sacred.yaml locks cyera-ci + rust-classroom");
    } else {
        let err = format!("policy/sacred.yaml missing locked {}", missing.join(","));
        println!("  FAIL  {err}");
        fails.push(err);
    }
}

fn check_dual_layer_demo(root: &Path, fails: &mut Vec<String>) {
    let path = root.join(DEMO);
    if !path.is_file() {
        println!("  FAIL  missing {DEMO}");
        fails.push(format!("missing {DEMO}"));
        return;
    }
    let estate = match load_estate(&path) {
        Ok(e) => e,
        Err(err) => {
            println!("  FAIL  {DEMO}: {err}");
            fails.push(format!("{DEMO}: {err}"));
            return;
        }
    };
    let has_sanctum = estate.agents.iter().any(|a| normalize_name(&a.id) == "sanctum");
    let sanctum_clean = estate.agents.iter().all(|a| {
        normalize_name(&a.id) != "sanctum" || !is_sacred_name(&a.display_name)
    });
    let declared: Vec<String> = estate
        .sacred_exclusions
        .iter()
        .map(|e| normalize_name(&e.id))
        .collect();
    let locks = locked_sacred_ids()
        .into_iter()
        .all(|id| declared.iter().any(|d| d == &normalize_name(id)));
    let frontier = estate
        .model_bindings
        .iter()
        .any(|b| b.class == ModelClass::Frontier);
    let local = estate
        .model_bindings
        .iter()
        .any(|b| b.class == ModelClass::Local);
    if has_sanctum && sanctum_clean && locks && frontier && local && !is_sacred_name("sanctum") {
        println!("  ok    {DEMO} (Sanctum is not Cyera; dual-layer locks declared)");
    } else {
        let err = format!("{DEMO} must keep Sanctum first-class and declare locked exclusions");
        println!("  FAIL  {err}");
        fails.push(err);
    }
}

fn check_refuse_fixtures(root: &Path, fails: &mut Vec<String>) {
    for rel in [
        REFUSE_BLEED,
        OMIT_LOCKED,
        "examples/fixtures/refuse-sacred-as-agent.yaml",
        "examples/fixtures/refuse-missing-sacred.yaml",
    ] {
        if root.join(rel).is_file() {
            println!("  ok    {rel}");
        } else {
            println!("  FAIL  missing {rel}");
            fails.push(format!("missing {rel}"));
        }
    }
}

fn check_gate90_alias(root: &Path, fails: &mut Vec<String>) {
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap_or_default();
    let script = fs::read_to_string(root.join("scripts/day90-gate.sh")).unwrap_or_default();
    let ok = makefile.contains("gate-90")
        && makefile.contains("scripts/day90-gate.sh")
        && script.contains("smoke.sh")
        && (script.contains("GATE-90") || script.contains("checklist"));
    if ok {
        println!("  ok    make gate-90 → smoke + day90 + checklist");
    } else {
        let err = "make gate-90 must be a thin alias (smoke + checklist)".into();
        println!("  FAIL  {err}");
        fails.push(err);
    }
}

fn check_feed_loop(root: &Path, fails: &mut Vec<String>) {
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap_or_default();
    let script = root.join("scripts/feed-loop.sh");
    let doc = root.join("docs/FEED-LOOP.md");
    let script_text = fs::read_to_string(&script).unwrap_or_default();
    let ok = makefile.contains("feed-loop")
        && makefile.contains("scripts/feed-loop.sh")
        && script.is_file()
        && doc.is_file()
        && script_text.contains("source_drivers")
        && script_text.contains("unset XAI_API_KEY");
    if ok {
        println!("  ok    make feed-loop → scrubbed trace → pack → propose → accept");
    } else {
        let err = "make feed-loop fixture walk missing (script + docs + Makefile)".into();
        println!("  FAIL  {err}");
        fails.push(err);
    }
}

fn check_day90_plus(root: &Path, fails: &mut Vec<String>) {
    let rel = "docs/DAY90-PLUS.md";
    let text = fs::read_to_string(root.join(rel)).unwrap_or_default();
    let parked = text.contains("MLX")
        && text.contains("stub")
        && (text.contains("GPU") || text.contains("rented"))
        && text.contains("cloud")
        && text.contains("Mac specialist")
        && (text.contains("park") || text.contains("until Jason"));
    if root.join(rel).is_file() && parked {
        println!("  ok    {rel} parks live Mac / GPU / cloud-spawn");
    } else {
        let err = format!("{rel} must honestly park live Mac / GPU / cloud-spawn");
        println!("  FAIL  {err}");
        fails.push(err);
    }
}

fn check_estate_hash_lock(root: &Path, fails: &mut Vec<String>) {
    let text = fs::read_to_string(root.join("examples/estate.yaml")).unwrap_or_default();
    if text.contains(HASH_LOCK) {
        println!("  ok    examples/estate.yaml hash lock");
    } else {
        let err = format!("examples/estate.yaml must keep {HASH_LOCK}");
        println!("  FAIL  {err}");
        fails.push(err);
    }
}

fn check_floor_no_vendor(root: &Path, fails: &mut Vec<String>) {
    let src = root.join("crates/floor-supervisor/src");
    if !src.is_dir() {
        println!("  FAIL  missing crates/floor-supervisor/src");
        fails.push("missing floor-supervisor src".into());
        return;
    }
    let mut hits = Vec::new();
    scan_needles(&src, &mut hits);
    if hits.is_empty() {
        println!("  ok    floor src has no vendor / SKU needles");
    } else {
        for hit in &hits {
            println!("  FAIL  {hit}");
            fails.push(hit.clone());
        }
    }
}

fn scan_needles(dir: &Path, hits: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_needles(&path, hits);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let lower = text.to_ascii_lowercase();
        for needle in FLOOR_NEEDLES {
            if lower.contains(needle) {
                hits.push(format!(
                    "{} contains {needle}",
                    path.strip_prefix(dir).unwrap_or(&path).display()
                ));
            }
        }
    }
}

fn strip_hash_comments(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Compile-only CI body. Comments mentioning `cargo test` are ignored.
pub(crate) fn compile_only_ci(text: &str) -> Result<(), String> {
    let body = strip_hash_comments(text);
    if !body.contains("cargo check --workspace --locked") {
        return Err("ci.yml must run cargo check --workspace --locked".into());
    }
    if body.contains("cargo test") {
        return Err("ci.yml must not run cargo test".into());
    }
    if !body.contains("pull_request") {
        return Err("ci.yml must be pull_request only".into());
    }
    if body.lines().any(|line| {
        let t = line.trim();
        t == "push:" || t.starts_with("push:")
    }) {
        return Err("ci.yml must not trigger on push".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_only_accepts_current_ci_shape() {
        let yml = r#"
# Compile-only. Real cargo test stays local (`make smoke`).
# Do not add cargo test or push-to-main jobs.
name: check
on:
  pull_request:
    branches: [main]
jobs:
  check:
    steps:
      - run: cargo check --workspace --locked
"#;
        assert!(compile_only_ci(yml).is_ok());
    }

    #[test]
    fn compile_only_refuses_cargo_test_job() {
        let yml = r#"
on:
  pull_request:
jobs:
  check:
    steps:
      - run: cargo check --workspace --locked
      - run: cargo test --workspace
"#;
        let err = compile_only_ci(yml).unwrap_err();
        assert!(err.contains("must not run cargo test"));
    }

    #[test]
    fn compile_only_refuses_push_trigger() {
        let yml = r#"
on:
  push:
  pull_request:
jobs:
  check:
    steps:
      - run: cargo check --workspace --locked
"#;
        let err = compile_only_ci(yml).unwrap_err();
        assert!(err.contains("must not trigger on push"));
    }
}
