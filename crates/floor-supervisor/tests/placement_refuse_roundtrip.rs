//! Placement-actual schema round-trip for every reconcile refuse code.
//! Does not grow placement.rs. No live Mac / GPU. Cloud never spawned.

use floor_supervisor::{
    claim_leases, load_placements, reconcile_placements, record_placements, write_placements,
    PlacementActual, PlacementLease,
};
use std::path::PathBuf;

const SCHEMA: &str = "cell-one.placement-actual.v0";
const CODES: &[&str] = &[
    "missing-lease",
    "extra-lease",
    "kind-mismatch",
    "host-class-mismatch",
    "cloud-spawned",
    "sacred-id",
    "expired",
    "bad-host-class",
];

fn estate() -> estate_schema::Estate {
    estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
}

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "cell-one-place-rt-{}-{}-{}",
        name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn box_lease(actual: &mut PlacementActual) -> &mut PlacementLease {
    actual
        .leases
        .iter_mut()
        .find(|l| l.placement_id == "cell-one-box")
        .expect("cell-one-box lease")
}

fn cloud_lease(actual: &mut PlacementActual) -> &mut PlacementLease {
    actual
        .leases
        .iter_mut()
        .find(|l| l.placement_id == "cursor-cloud")
        .expect("cursor-cloud lease")
}

fn mutate(code: &str, actual: &mut PlacementActual) {
    match code {
        "missing-lease" => {
            actual.leases.retain(|l| l.placement_id != "cell-one-box");
        }
        "extra-lease" => {
            let mut extra = box_lease(actual).clone();
            extra.placement_id = "bogus-box".into();
            extra.spawned = false;
            actual.leases.push(extra);
        }
        "kind-mismatch" => {
            let lease = cloud_lease(actual);
            lease.kind = "box".into();
            lease.spawned = false;
        }
        "host-class-mismatch" => {
            for lease in &mut actual.leases {
                lease.host_class = "apple-silicon".into();
            }
        }
        "cloud-spawned" => {
            let lease = cloud_lease(actual);
            lease.kind = "cloud-agent".into();
            lease.spawned = true;
        }
        "sacred-id" => {
            box_lease(actual).agents.push("cyera-ci".into());
        }
        "expired" => {
            let lease = box_lease(actual);
            lease.ttl_secs = Some(1);
            lease.issued_at = Some(0);
            lease.expires_at = Some(1);
        }
        "bad-host-class" => {
            box_lease(actual).host_class = "rtx-5090".into();
        }
        other => panic!("unknown refuse code {other}"),
    }
}

fn round_trip(actual: &PlacementActual) -> PlacementActual {
    let json = serde_json::to_string(actual).unwrap();
    let reloaded: PlacementActual = serde_json::from_str(&json).unwrap();
    assert_eq!(actual, &reloaded, "serde round-trip must be lossless");
    assert_eq!(reloaded.schema, SCHEMA);
    reloaded
}

#[test]
fn every_refuse_code_survives_placement_actual_round_trip() {
    let estate = estate();
    let baseline = claim_leases(&estate);
    assert_eq!(baseline.schema, SCHEMA);
    assert_eq!(round_trip(&baseline), baseline);

    for code in CODES {
        let dir = tmp(code);
        let mut actual = baseline.clone();
        mutate(code, &mut actual);
        let reloaded = round_trip(&actual);
        write_placements(&dir, &reloaded).unwrap();
        let from_disk = load_placements(&dir).unwrap().expect("wrote actual");
        assert_eq!(from_disk.schema, SCHEMA);
        assert_eq!(from_disk, reloaded);
        let report = reconcile_placements(&estate, &dir).unwrap();
        assert!(
            report.refuses.iter().any(|r| r.code == *code),
            "{code} missing after round-trip: {:?}",
            report.refuses
        );
        assert!(!report.in_sync, "{code} must fail closed");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn clean_actual_round_trip_has_no_refuse_codes() {
    let estate = estate();
    let dir = tmp("clean");
    let actual = round_trip(&claim_leases(&estate));
    write_placements(&dir, &actual).unwrap();
    let report = reconcile_placements(&estate, &dir).unwrap();
    for code in CODES {
        assert!(
            !report.refuses.iter().any(|r| r.code == *code),
            "clean actual must not report {code}: {:?}",
            report.refuses
        );
    }
    assert!(report.in_sync);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn record_refuses_sku_host_class_and_claim_does_not_write_any() {
    let mut estate = estate();
    let box_place = estate
        .placements
        .iter_mut()
        .find(|p| p.id == "cell-one-box")
        .expect("cell-one-box");
    box_place.host_class = Some("rtx-5090".into());
    let claimed = claim_leases(&estate);
    let box_lease = claimed
        .leases
        .iter()
        .find(|l| l.placement_id == "cell-one-box")
        .expect("claimed box");
    assert_eq!(
        box_lease.host_class, "rtx-5090",
        "claim must not launder a SKU to any"
    );
    let dir = tmp("record-sku");
    let err = record_placements(&estate, &dir).unwrap_err();
    assert!(
        err.to_string().contains("refuse:bad-host-class"),
        "{err}"
    );
    assert!(
        !dir.join("placement-actual.json").exists(),
        "record must not write after a host_class refuse"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
