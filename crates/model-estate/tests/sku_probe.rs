//! Probe / catalog driver ids must not encode a hardware SKU.

#[test]
fn probe_ids_refuse_hardware_sku() {
    model_estate::refuse_sku_probe_id("ollama").unwrap();
    model_estate::refuse_sku_probe_id("mlx").unwrap();
    let err = model_estate::refuse_sku_probe_id("local-5090").unwrap_err();
    assert!(err.contains("refuse:sku-banned"));
    for p in model_estate::catalog_probes() {
        model_estate::refuse_sku_probe_id(&p.driver).unwrap();
    }
}
