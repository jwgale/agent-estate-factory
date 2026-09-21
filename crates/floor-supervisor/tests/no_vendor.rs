//! Floor core must not hard-code vendor ids. Needles live only in this test.

use std::path::Path;

fn visit(dir: &Path, hits: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            visit(&path, hits);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let lower = text.to_ascii_lowercase();
        for needle in ["xai", "grok", "5090", "cyera"] {
            if lower.contains(needle) {
                hits.push(format!("{} contains {needle}", path.display()));
            }
        }
    }
}

#[test]
fn floor_sources_have_no_vendor_ids() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut hits = Vec::new();
    visit(&root, &mut hits);
    assert!(hits.is_empty(), "{}", hits.join("\n"));
}
