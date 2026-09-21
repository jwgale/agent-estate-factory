//! Probe / catalog driver ids must not encode a hardware SKU.
//! Same refuse as convey sync: SKU is not a host_class and not an id.

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
        assert!(
            estate_schema::is_host_class(&p.host_class),
            "probe {} host_class {} must be a locked name",
            p.driver,
            p.host_class
        );
        assert!(
            !estate_schema::contains_sku(&p.host_class),
            "probe {} host_class {} encodes a SKU",
            p.driver,
            p.host_class
        );
    }
}

#[test]
fn catalog_cards_refuse_sku_in_id_like_sync() {
    assert!(model_estate::parse_host_class("rtx-5090").is_none());
    assert!(estate_schema::canonical_host_class_opt(Some("rtx-5090")).is_none());
    for card in model_estate::catalog() {
        assert!(
            !estate_schema::contains_sku(card.driver_id),
            "catalog card {} encodes a SKU",
            card.driver_id
        );
        for host in card.hosts {
            assert!(
                estate_schema::is_host_class(host.as_str()),
                "catalog card {} host {} is not locked",
                card.driver_id,
                host.as_str()
            );
        }
    }
    let file = model_estate::catalog_file();
    for card in &file.cards {
        assert!(
            !estate_schema::contains_sku(&card.driver_id),
            "catalog file card {} encodes a SKU",
            card.driver_id
        );
    }
}
