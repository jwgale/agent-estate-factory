//! Hardening: gate-90 stays local, layout doc matches code, no Origin URLs.
//! No live Mac / GPU. Cloud never spawned. Does not run make gate-90.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

#[test]
fn gate90_stays_local_only() {
    let script = read("scripts/day90-gate.sh");
    assert!(
        script.contains("Do not add to GitHub Actions"),
        "day90-gate.sh must stay local-only"
    );
    assert!(
        script.contains("Hosted CI stays compile-only"),
        "day90-gate.sh must say hosted CI is compile-only"
    );
    assert!(
        script.contains("scripts/smoke.sh"),
        "gate-90 wraps smoke (which runs cargo test locally)"
    );

    let makefile = read("Makefile");
    assert!(
        makefile.contains("Do not add to GitHub Actions"),
        "Makefile gate-90 comment must stay local-only"
    );

    let ci = read(".github/workflows/ci.yml");
    assert!(
        !ci.lines().any(|l| {
            let t = l.trim();
            !t.starts_with('#') && (t.contains("make gate-90") || t.contains("make smoke"))
        }),
        "ci.yml must not invoke make gate-90 / make smoke"
    );
    assert!(
        ci.contains("cargo check --workspace --locked"),
        "hosted job stays compile-only"
    );

    let why = read("docs/OPERATOR-DAY.md");
    assert!(why.contains("Why `make gate-90` stays local"));
    assert!(why.contains("cargo test --workspace"));
}

#[test]
fn cell_layout_matches_code_paths() {
    let layout = read("docs/cell-layout.md");
    for needle in [
        "placement-actual.json",
        "reconcile-suggest.md",
        "feed/feed-cursor.json",
        "sessions.jsonl",
        "conveyor-mesh.json",
        "conveyor-hops.json",
        "conveyor-leases.json",
        "audit-export/",
        "backups/cell-backup-*",
        "expired",
        "cell-one.backup-prune.v0",
        "enrich-edit.md",
    ] {
        assert!(layout.contains(needle), "cell-layout.md missing {needle}");
    }

    let backup = read("crates/floor-supervisor/src/backup.rs");
    for name in [
        "placement-actual.json",
        "lifecycle.json",
        "sessions.jsonl",
        "conveyor-mesh.json",
        "desired-snapshot.yaml",
        "reconcile.md",
        "model-actual.json",
    ] {
        assert!(
            backup.contains(name),
            "backup DURABLE_FILES drifted from {name}"
        );
        assert!(
            layout.contains(name),
            "cell-layout.md missing durable file {name}"
        );
    }
}

#[test]
fn no_origin_urls_or_src_todo_fixme() {
    let root = repo_root();
    let mut forbidden = Vec::new();
    walk(&root, &root, &mut forbidden);
    assert!(
        forbidden.is_empty(),
        "leftover Origin URL or src TODO/FIXME:\n{}",
        forbidden.join("\n")
    );
}

fn walk(root: &Path, dir: &Path, hits: &mut Vec<String>) {
    let skip = ["target", ".git", ".cell", "backups"];
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if skip.contains(&name) {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, hits);
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let rel_s = rel.to_string_lossy();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !matches!(ext, "rs" | "md" | "yml" | "yaml" | "toml" | "sh" | "json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let origin_host = format!("{}{}{}", "origin", ".", "cursor.com");
        let old_tmp = format!("{}{}{}", "tmp", "-", "c90cdc");
        if text.contains(&origin_host) || text.contains(&old_tmp) {
            hits.push(format!("{rel_s}: Origin URL"));
        }
        let under_src = rel.starts_with("crates") && rel_s.contains("/src/");
        if under_src && ext == "rs" {
            for (i, line) in text.lines().enumerate() {
                let t = line.trim_start();
                if t.starts_with("//") && (t.contains("TODO") || t.contains("FIXME")) {
                    hits.push(format!("{rel_s}:{}: {t}", i + 1));
                }
            }
        }
    }
}
