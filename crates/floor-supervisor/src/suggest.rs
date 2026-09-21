//! Suggested patch notes for floor drift. Never mutates leases or the estate.

use crate::placement::ReconcileReport;
use crate::SupervisorError;
use std::path::Path;

pub const SUGGEST_SCHEMA: &str = "cell-one.reconcile-suggest.v0";

/// Human patch notes only. Never mutates leases or the estate.
pub fn render_suggest(report: &ReconcileReport) -> String {
    let mut out = String::from("Reconcile suggest (patch file only)\n");
    out.push_str("===================================\n");
    out.push_str(&format!("schema: {SUGGEST_SCHEMA}\n"));
    out.push_str("auto_apply: false\n");
    out.push_str("The factory will not apply this file. Jason edits estate.yaml or runs apply/expire by hand.\n\n");
    out.push_str(&format!("in_sync: {}\n", report.in_sync));
    out.push_str(&format!("desired_hash: {}\n\n", report.desired_hash));
    if report.in_sync {
        out.push_str("No patch. Desired and actual match. Cloud-agent stays unspawned.\n");
        return out;
    }
    out.push_str("Suggested hand edits\n--------------------\n");
    if report.refuses.is_empty() {
        out.push_str("  Drift without a refuse code. Re-run `estate apply` after you review.\n");
    }
    for r in &report.refuses {
        let hint = match r.code.as_str() {
            "missing-lease" => {
                "run `estate apply` or `estate resume` (do not invent a lease file)"
            }
            "extra-lease" => {
                "add this placement to estate.yaml or drop the extra lease by hand after review"
            }
            "kind-mismatch" => {
                "edit the estate placement kind, then apply; do not rewrite placement-actual.json"
            }
            "host-class-mismatch" => {
                "fail closed: fix host_class on the estate (portable name), then apply"
            }
            "cloud-spawned" => "refuse: unspawn by hand; floor must not spawn cloud-agent",
            "expired" => "run `estate expire --forget`, then apply/resume",
            "sacred-id" => "refuse:sacred-id: remove the sacred id from the lease by hand",
            other => other,
        };
        out.push_str(&format!("  refuse:{}: {} — {hint}\n", r.code, r.subject));
    }
    out.push_str("\nNot applied. No estate rewrite. No lease rewrite.\n");
    out
}

pub fn write_suggest(
    state_dir: &Path,
    report: &ReconcileReport,
) -> Result<std::path::PathBuf, SupervisorError> {
    std::fs::create_dir_all(state_dir)?;
    let path = state_dir.join("reconcile-suggest.md");
    std::fs::write(&path, render_suggest(report))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::{reconcile_placements, record_placements, write_placements};

    #[test]
    fn suggest_names_fail_closed_and_never_rewrites_leases() {
        let estate =
            estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "cell-one-suggest-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut actual = record_placements(&estate, &tmp).unwrap();
        for lease in &mut actual.leases {
            lease.host_class = "apple-silicon".into();
        }
        write_placements(&tmp, &actual).unwrap();
        let before = std::fs::read_to_string(tmp.join("placement-actual.json")).unwrap();
        let report = reconcile_placements(&estate, &tmp).unwrap();
        assert!(!report.in_sync);
        let suggest = render_suggest(&report);
        assert!(suggest.contains("auto_apply: false"));
        assert!(suggest.contains("fail closed"));
        assert!(!suggest.contains("No patch."));
        let dest = write_suggest(&tmp, &report).unwrap();
        assert!(dest.ends_with("reconcile-suggest.md"));
        assert_eq!(
            before,
            std::fs::read_to_string(tmp.join("placement-actual.json")).unwrap()
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
