//! Gate and parking-lot pages keep stubs off the live-ok column.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn gate_and_parking_lot_do_not_call_stubs_ready() {
    let root = repo_root();
    let gate = std::fs::read_to_string(root.join("docs/GATE-90.md")).unwrap();
    let plus = std::fs::read_to_string(root.join("docs/DAY90-PLUS.md")).unwrap();
    assert!(
        !gate.contains("ready for Ollama"),
        "gate must not call the Ollama complete path ready"
    );
    assert!(
        gate.contains("Mac complete is recorded"),
        "gate must record the Mac specialist completion"
    );
    assert!(
        !gate.contains("Mac complete is not recorded"),
        "gate must not leave the Mac completion unrecorded"
    );
    assert!(
        gate.contains("not live-ok"),
        "gate must say mlx/vllm/trt are not live-ok"
    );
    assert!(plus.contains("vLLM"), "{plus}");
    assert!(plus.contains("TRT"), "{plus}");
    assert!(
        plus.contains("Not live-ok"),
        "parking lot must mark vLLM and TRT not live-ok"
    );
    assert!(
        !plus.contains("READY_FOR_LIVE_TEST: yes") && !plus.contains("READY_FOR_LIVE_TEST`: yes"),
        "parking lot must not open a live hand-off"
    );
}

#[test]
fn glossary_keeps_purpose_built_slm_in_suite() {
    let root = repo_root();
    let glossary = std::fs::read_to_string(root.join("docs/UBIQUITOUS_LANGUAGE.md")).unwrap();
    for term in [
        "estate",
        "lane",
        "plan",
        "apply",
        "lease",
        "pack",
        "curator",
        "frontier",
        "local",
        "sacred",
        "refuse",
        "placement",
        "convey",
        "purpose-built",
        "local runtime",
        "lease-bound hop stub",
    ] {
        assert!(
            glossary.to_lowercase().contains(term),
            "glossary missing {term}"
        );
    }
    assert!(
        glossary.contains("ollama"),
        "glossary must name the ollama driver"
    );
    assert!(
        !glossary.contains("Not a distillation"),
        "glossary must not ban distillation"
    );

    let north = std::fs::read_to_string(root.join("docs/NORTH-STAR.md")).unwrap();
    let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
    let help = std::fs::read_to_string(root.join("crates/estate-control/src/help.rs")).unwrap();
    let plus = std::fs::read_to_string(root.join("docs/DAY90-PLUS.md")).unwrap();
    let probes = std::fs::read_to_string(root.join("docs/LIVE-PROBES.md")).unwrap();
    for (name, text) in [
        ("NORTH-STAR", north.as_str()),
        ("README", readme.as_str()),
        ("help", help.as_str()),
        ("DAY90-PLUS", plus.as_str()),
        ("LIVE-PROBES", probes.as_str()),
    ] {
        assert!(
            !text.contains("Not a distillation") && !text.contains("Distillation and an AI gateway"),
            "{name} still parks distillation"
        );
    }
    assert!(north.contains("purpose-built"));
    assert!(north.contains("UBIQUITOUS_LANGUAGE.md"));
    assert!(readme.contains("purpose-built"));
    assert!(readme.contains("docs/UBIQUITOUS_LANGUAGE.md"));
    assert!(help.contains("purpose-built"));
    assert!(plus.contains("purpose-built"));
    for slop in ["delve", "it's worth noting", "rather than"] {
        assert!(!glossary.to_lowercase().contains(slop), "glossary slop: {slop}");
        assert!(!north.to_lowercase().contains(slop), "north-star slop: {slop}");
    }
}
