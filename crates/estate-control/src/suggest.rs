//! Suggested patch notes for floor drift. Never mutates leases or the estate.

use floor_supervisor::ReconcileReport;
use std::path::{Path, PathBuf};

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

pub fn write_suggest(state_dir: &Path, report: &ReconcileReport) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(state_dir)?;
    let path = state_dir.join("reconcile-suggest.md");
    std::fs::write(&path, render_suggest(report))?;
    Ok(path)
}
