//! Safety invariants that stay true across feature work.
//!
//! These checks do not lock tip SHAs, pull-request numbers, or changelog heads.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn posix_cksum(data: &[u8]) -> u32 {
    fn eat(crc: u32, byte: u8) -> u32 {
        let mut c = crc ^ (u32::from(byte) << 24);
        for _ in 0..8 {
            if c & 0x8000_0000 != 0 {
                c = (c << 1) ^ 0x04c1_1db7;
            } else {
                c <<= 1;
            }
        }
        c
    }
    let mut crc = 0u32;
    for &byte in data {
        crc = eat(crc, byte);
    }
    let mut len = data.len();
    while len != 0 {
        crc = eat(crc, (len & 0xff) as u8);
        len >>= 8;
    }
    !crc
}

fn sets_ready_yes(text: &str) -> Option<usize> {
    let lower = text.to_ascii_lowercase();
    let key = "ready_for_live_test";
    let bytes = lower.as_bytes();
    let mut start = 0;
    while let Some(rel) = lower[start..].find(key) {
        let mut j = start + rel + key.len();
        let mut saw_colon = false;
        while j < bytes.len() && matches!(bytes[j], b'`' | b'*' | b' ' | b'\t' | b':' | b'\n' | b'\r')
        {
            if bytes[j] == b':' {
                saw_colon = true;
            }
            j += 1;
        }
        if saw_colon && lower[j..].starts_with("yes") {
            let end = j + 3;
            if end == bytes.len() || !bytes[end].is_ascii_alphanumeric() {
                return Some(start + rel);
            }
        }
        start += rel + key.len();
    }
    None
}

fn walk_docs_and_config(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || name == "target" {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if name == "CHANGELOG.md" {
                continue;
            }
            if matches!(ext, "md" | "yaml" | "yml" | "toml") {
                out.push(path);
            }
        }
    }
    out
}

#[test]
fn ready_for_live_test_stays_no() {
    let root = repo_root();
    let status = fs::read_to_string(root.join("docs/CELL-ONE-STATUS.md")).unwrap();
    assert!(
        status.contains("READY_FOR_LIVE_TEST`: no") || status.contains("READY_FOR_LIVE_TEST: no"),
        "CELL-ONE-STATUS.md must still state READY_FOR_LIVE_TEST no"
    );
    for path in walk_docs_and_config(&root) {
        let text = fs::read_to_string(&path).unwrap();
        if let Some(at) = sets_ready_yes(&text) {
            let snippet = &text[at..text.len().min(at + 80)];
            panic!(
                "{} sets READY_FOR_LIVE_TEST to yes: {}",
                path.strip_prefix(&root).unwrap_or(&path).display(),
                snippet
            );
        }
    }
}

#[test]
fn examples_estate_yaml_stays_byte_locked() {
    let path = repo_root().join("examples/estate.yaml");
    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes.len(), 3391, "examples/estate.yaml length changed");
    let sum = posix_cksum(&bytes);
    assert_eq!(
        sum, 43770130,
        "examples/estate.yaml cksum changed: {sum} {}",
        bytes.len()
    );
}

fn line_claims_live_pass(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let negated = lower.contains("not a live pass")
        || lower.contains("no live pass")
        || lower.contains("does not invent a live pass")
        || lower.contains("does not invent a new live pass")
        || lower.contains("do not invent a live pass")
        || lower.contains("do not claim a train, a promote, a live pass")
        || lower.contains("not a pass")
        || lower.contains("without") && lower.contains("pass");
    if lower.contains("live pass") && !negated {
        let points_at_recorded = lower.contains("target c")
            || lower.contains("only live uniqueness")
            || lower.contains("recorded");
        if !points_at_recorded {
            return true;
        }
    }
    line.contains("**PASS.**")
}

/// Recorded proofs already on the live-probes page.
/// Target C is the uniqueness prove. The other two headings are the
/// probe and Mac specialist rows that page already records.
fn live_pass_allowlist(rel: &str, section: &str) -> bool {
    rel == "docs/LIVE-PROBES.md"
        && matches!(
            section,
            "Recorded live proof (Jason boxes)"
                | "Target C live uniqueness (5090-class)"
                | "Mac specialist result (recorded)"
        )
}

#[test]
fn live_pass_claims_stay_on_the_recorded_proves() {
    let root = repo_root();
    for path in walk_docs_and_config(&root) {
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path).unwrap();
        let mut section = String::new();
        for (idx, line) in text.lines().enumerate() {
            if let Some(title) = line.strip_prefix("## ") {
                section = title.trim().to_string();
            }
            if line_claims_live_pass(line) && !live_pass_allowlist(&rel, &section) {
                panic!("{rel}:{} claims a live PASS outside the recorded proves: {line}", idx + 1);
            }
        }
    }
}

fn makefile_targets(makefile: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in makefile.lines() {
        let Some(name) = line.strip_suffix(':') else {
            continue;
        };
        if name.is_empty()
            || name.contains(char::is_whitespace)
            || name.starts_with('.')
            || name.starts_with('#')
        {
            continue;
        }
        let opt_in = name.ends_with("-journey")
            || name.starts_with("uniqueness-")
            || name.starts_with("purpose-build-")
            || name.starts_with("classify-")
            || name == "train-next"
            || name == "train-next-lora"
            || name == "lf-beachhead-prepare";
        if opt_in {
            names.push(name.to_string());
        }
    }
    names
}

fn recipe<'a>(makefile: &'a str, target: &str) -> &'a str {
    let needle = format!("\n{target}:\n");
    let Some(at) = makefile.find(&needle) else {
        panic!("Makefile missing {target} recipe");
    };
    let rest = &makefile[at + needle.len()..];
    let end = rest
        .lines()
        .position(|line| {
            !line.is_empty() && !line.starts_with([' ', '\t']) && !line.starts_with('#')
        })
        .map(|i| rest.lines().take(i).map(|l| l.len() + 1).sum())
        .unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn opt_in_journeys_stay_off_smoke_gate_and_actions() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let targets = makefile_targets(&makefile);
    assert!(
        targets.iter().any(|name| name == "purpose-build-journey"),
        "purpose-build-journey must stay a Makefile target"
    );
    assert!(
        targets.iter().any(|name| name == "classify-prepare"),
        "classify-prepare must stay a Makefile target"
    );
    let mut files: Vec<(String, String)> = vec![
        ("Makefile smoke".into(), recipe(&makefile, "smoke").to_string()),
        (
            "Makefile gate-90".into(),
            recipe(&makefile, "gate-90").to_string(),
        ),
        (
            "scripts/smoke.sh".into(),
            fs::read_to_string(root.join("scripts/smoke.sh")).unwrap(),
        ),
        (
            "scripts/day90-gate.sh".into(),
            fs::read_to_string(root.join("scripts/day90-gate.sh")).unwrap(),
        ),
    ];
    for entry in fs::read_dir(root.join(".github/workflows")).unwrap().flatten() {
        let path = entry.path();
        if path.is_file() {
            let name = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            files.push((name, fs::read_to_string(&path).unwrap()));
        }
    }
    for name in &targets {
        for (surface, body) in &files {
            assert!(
                !body.contains(name),
                "{surface} invokes opt-in target {name}"
            );
        }
    }
}
