//! Probe / catalog driver ids must not encode a hardware SKU.

#[test]
fn probe_ids_refuse_hardware_sku() {
    assert!(!estate_schema::contains_sku("ollama"));
    assert!(!estate_schema::contains_sku("mlx"));
    assert!(estate_schema::contains_sku("local-5090"));
    for p in model_estate::catalog_probes() {
        assert!(
            !estate_schema::contains_sku(&p.driver),
            "probe driver {} encodes a SKU",
            p.driver
        );
    }
}
