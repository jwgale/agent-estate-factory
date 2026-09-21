//! Day-90 operator topic pages. `estate help [topic]`.
//! Does not apply, spawn, or promote. Local only.

use anyhow::{bail, Result};

const TOPICS: &[&str] = &["status", "plan", "apply", "reconcile", "feed-loop", "backup"];

pub(crate) fn cmd_help(topic: Option<&str>) -> Result<()> {
    match topic.map(str::trim).filter(|t| !t.is_empty()) {
        None => {
            print!("{INDEX}");
            Ok(())
        }
        Some("status") => {
            print!("{STATUS}");
            Ok(())
        }
        Some("plan") => {
            print!("{PLAN}");
            Ok(())
        }
        Some("apply") => {
            print!("{APPLY}");
            Ok(())
        }
        Some("reconcile") => {
            print!("{RECONCILE}");
            Ok(())
        }
        Some("feed-loop") | Some("feed") => {
            print!("{FEED_LOOP}");
            Ok(())
        }
        Some("backup") => {
            print!("{BACKUP}");
            Ok(())
        }
        Some(other) => {
            eprintln!("unknown help topic: {other}");
            eprintln!("topics: {}", TOPICS.join(", "));
            bail!("refuse:help-topic: unknown '{other}'");
        }
    }
}

const INDEX: &str = "\
Cell One — Day-90 operator help
===============================
Local only. Hosted CI is compile-only. Cloud-agent is declared, not spawned.
Live Mac / GPU wait in docs/DAY90-PLUS.md. Do not fake them.

  estate help status
  estate help plan
  estate help apply
  estate help reconcile
  estate help feed-loop
  estate help backup

Entrypoint: make gate-90
Loop:       make day90
Mixed:      make day90-mixed
Feed walk:  make feed-loop
";

const STATUS: &str = "\
estate status — one-pager
=========================
paused?, lease counts, expired, last plan, last apply, open proposals,
policy present?, doctor line. Frontier model id prints when the estate
binding or a catalog file names one. Cloud-agent stays \"declared, not spawned\".

  estate status --estate examples/estate.yaml --state-dir .cell
  make day90

Does not apply. Does not spawn. Does not require a Mac or a GPU.
";

const PLAN: &str = "\
estate plan — human control surface
===================================
Writes reviewable markdown under plans/. Does not apply.

  estate plan --estate examples/estate.yaml --plans-dir plans
  estate plan --reviewed
  estate plan diff --estate examples/estate.yaml
  estate plan export-pr --out plans/PR.md

apply --require-plan / --require-fresh-plan read these files.
";

const APPLY: &str = "\
estate apply — converge desired state
=====================================
Binds box sessions. Records placement leases. Does not spawn cursor-cloud.

  estate apply --dry-run --estate examples/estate.yaml --state-dir .cell
  estate apply --estate examples/estate.yaml --state-dir .cell
  estate apply --require-plan --require-fresh-plan
  estate apply --force          # only when drift refuses

Second identical apply is a no-op (unchanged). Expired leases refuse.
After `estate expire --forget`, apply restamps leases. That is not `--force`.
";

const RECONCILE: &str = "\
estate reconcile — desired vs actual
====================================
Report only. --suggest writes a patch file. Never auto-applies.

  estate reconcile --estate examples/estate.yaml --state-dir .cell
  estate reconcile --suggest --estate examples/estate.yaml --state-dir .cell

Refuse codes: missing-lease, extra-lease, kind-mismatch,
host-class-mismatch, cloud-spawned, sacred-id, expired.
Jason still edits leases / the estate by hand.
";

const FEED_LOOP: &str = "\
feed-loop — scrubbed trace → pack → propose → accept
====================================================
Fixtures only. Isolated target/feed-loop-cell. Not part of make smoke.

  make feed-loop
  estate feed pack --feed-dir .cell/feed --drop-dir packs
  estate feed cursor --feed-dir .cell/feed
  estate packs propose --id overnight-traces
  estate packs accept --id overnight-traces --curator jason

Cursor is a watermark. Rematerialize does not auto-promote.
estate.yaml is never rewritten. Promote stays locked off.
See docs/FEED-LOOP.md.
";

const BACKUP: &str = "\
estate backup — local cell archive
==================================
Timestamped backups/cell-backup-*. Restore refuses sacred mismatch.

  estate backup --state-dir .cell --out backups
  estate backup --prune 5 --state-dir .cell --out backups
  estate restore --from backups/cell-backup-... --dry-run

--prune N keeps the newest N archives and deletes the rest.
N=0 refuses. Does not upload. Does not spawn.
";
