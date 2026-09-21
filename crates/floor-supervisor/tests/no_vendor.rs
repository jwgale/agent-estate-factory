//! Floor core must not hard-code vendor ids. Needles live only in this test.

use std::path::Path;

#[test]
fn floor_sources_have_no_vendor_ids() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for name in ["lib.rs", "main.rs"] {
        let text = std::fs::read_to_string(root.join(name)).unwrap();
        let lower = text.to_ascii_lowercase();
        for needle in ["xai", "grok", "5090", "cyera"] {
            assert!(
                !lower.contains(needle),
                "{name} must not contain vendor id {needle}"
            );
        }
    }
}
