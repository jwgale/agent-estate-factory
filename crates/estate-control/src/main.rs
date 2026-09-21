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
    Validate {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    Plan {
        #[command(subcommand)]
        action: Option<PlanAction>,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long)]
        against: Option<PathBuf>,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value_t = false)]
        reviewed: bool,
        #[arg(long, default_value = "plans/reviewed")]
        reviewed_dir: PathBuf,
    },
    Apply {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value_t = false)]
        require_plan: bool,
        #[arg(long)]
        import_pack: Option<String>,
        #[arg(long, default_value = "packs")]
        packs_dir: PathBuf,
        #[arg(long, default_value_t = false)]
        require_fresh_plan: bool,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
        #[arg(long, default_value = "jason")]
        curator: String,
    },
    Drift {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
    Models {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    Plans {
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
    },
    Suspend {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    Resume {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
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
    Feed {
        #[command(subcommand)]
        command: FeedCommand,
    },
    Catalog {
        #[arg(long, default_value = ".cell/catalog.json")]
        out: PathBuf,
    },
    Leases {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    Audits {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    Reconcile {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },
    History {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    Probes,
    Convey {
        #[command(subcommand)]
        command: ConveyCommand,
    },
    Packs {
        #[command(subcommand)]
        command: PacksCommand,
    },
    Expire {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value_t = false)]
        forget: bool,
    },
    Doctor {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    Sessions {
        #[command(subcommand)]
        command: SessionsCommand,
    },
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
    Policy {
        #[command(subcommand)]
        command: PolicyCommand,
    },
    PauseProof {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
}
