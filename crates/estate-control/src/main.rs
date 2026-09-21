use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use conveyor_proxy::{
    call_hop, declare_hop, forget_expired_hop_leases, hop_now_unix, list_expired_hop_leases,
    list_hop_leases, list_hops, sync_from_placements, HopDecl,
};
use estate_schema::{
    blast_grows, check_policy_file, covering_plan, describe, describe_placements, diff_estates,
    estate_hash, latest_plan, list_plans, load_estate, load_estate_unvalidated, load_plan_json,
    load_policy, mark_plan_reviewed, plan_against_is_fresh, plan_against_is_fresh_strict,
    plan_blast_width, plan_is_reviewable, policy_allows, render_plan, render_plan_diff, validate,
    write_plan,
};
use feed_collector::{
    import_pack_for, list_drop_packs, list_open_proposals, load_cursor, materialize_from_feed,
    propose_enrich, refuse_promote, write_pack_index, LOCKED_CURATOR,
};
use floor_supervisor::{
    append_apply_audit, apply_dry_run, apply_with_profile_dir, backup_cell, drift_with_roots,
    forget_expired_leases, list_apply_audits, list_expired_leases, list_lifecycle_events,
    list_session_events, load_desired_snapshot, load_lifecycle, load_placements, mark_running,
    now_unix, pause_kit_proof, reconcile_placements, record_placements, refuse_expired_leases,
    render_dry_run, render_reconcile, render_restore, restore_cell, resume, suspend,
    tail_session_events, write_reconcile, ApplyAudit,
};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "estate",
    about = "Cell One estate-control: validate, plan, apply, drift. Does not execute tools or models."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fail closed if the estate file is invalid.
    Validate {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    /// Human-readable blast-radius plan; append-only write to plans/.
    Plan {
        #[command(subcommand)]
        action: Option<PlanAction>,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        /// Previous estate YAML to diff against. Default: last apply snapshot, else greenfield.
        #[arg(long)]
        against: Option<PathBuf>,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Copy the new plan into plans/reviewed/ for a human PR.
        #[arg(long, default_value_t = false)]
        reviewed: bool,
        #[arg(long, default_value = "plans/reviewed")]
        reviewed_dir: PathBuf,
    },
    /// Converge isolation (Control→Data apply seam). Stretch: included, thin.
    Apply {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        /// Fail if no plan on disk covers this estate hash (gated apply).
        #[arg(long, default_value_t = false)]
        require_plan: bool,
        /// Explicit pack import during apply (never silent; does not edit estate.yaml).
        #[arg(long)]
        import_pack: Option<String>,
        #[arg(long, default_value = "packs")]
        packs_dir: PathBuf,
        /// Fail if covering plan against_hash does not match last apply.
        #[arg(long, default_value_t = false)]
        require_fresh_plan: bool,
        /// Print blast radius + reconcile preview. Does not write leases.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
        /// Import gate. Must match locked curator `jason`.
        #[arg(long, default_value = "jason")]
        curator: String,
    },
    /// Compare desired estate to regenerable actual-state.
    Drift {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
    /// List equal-class bindings. Does not invoke them.
    Models {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    /// Append-only plan history (human control surface).
    Plans {
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
    },
    /// Drop runtime; write durable lifecycle=suspended. Lane roots stay.
    Suspend {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Re-apply from estate files; lifecycle=running.
    Resume {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
    /// Persisted estate + durable lifecycle + disposable runtime.
    Status {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = "packs")]
        packs_dir: PathBuf,
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Feed plane: materialize packs, list drop zone, refuse promote.
    Feed {
        #[command(subcommand)]
        command: FeedCommand,
    },
    /// Dump portable local catalog (file SoT). Does not invoke models.
    Catalog {
        #[arg(long, default_value = ".cell/catalog.json")]
        out: PathBuf,
    },
    /// Print durable placement leases. Cloud-agent must stay unspawned.
    Leases {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Print apply-audit.jsonl (gated apply history).
    Audits {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Desired vs actual placement reconcile. Sacred-id deny stays.
    Reconcile {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Bundle local review artifacts. Not a remote upload.
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },
    /// Print append-only lifecycle.jsonl (suspend/resume/apply).
    History {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Catalog-level driver probes. Not live pings. Does not invoke models.
    Probes,
    /// Capability mesh: declare hop, lease-bound call. Not a gateway.
    Convey {
        #[command(subcommand)]
        command: ConveyCommand,
    },
    /// Pack curator path: list / import / refuse promote + INDEX.
    Packs {
        #[command(subcommand)]
        command: PacksCommand,
    },
    /// List expired placement leases. Apply/resume refuse them.
    Expire {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Drop expired rows so a later apply can record fresh leases. Does not spawn.
        #[arg(long, default_value_t = false)]
        forget: bool,
    },
    /// One-page health: .cell layout, schema files, quiet-hours workflows absent.
    Doctor {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Append-only `.cell/sessions.jsonl` (spawn/unspawn/suspend/resume).
    Sessions {
        #[command(subcommand)]
        command: SessionsCommand,
    },
    /// Timestamped local archive of durable `.cell/` files.
    Backup {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = "backups")]
        out: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
    },
    /// Restore a cell archive. Refuses sacred mismatch. `--dry-run` writes nothing.
    Restore {
        #[arg(long)]
        from: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
    },
    /// Check a policy pack against a known action.
    Policy {
        #[command(subcommand)]
        command: PolicyCommand,
    },
    /// Apply → suspend → drop sessions → resume. Leases stay on disk.
    PauseProof {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
}
