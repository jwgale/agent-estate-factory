use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use estate_schema::{
    describe, load_estate_unvalidated, validate,
};
use std::path::{Path, PathBuf};

mod helpers;
mod ops;
mod plan_apply;
mod watch;

use ops::*;
use plan_apply::*;
use watch::*;

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

#[derive(Subcommand)]
enum PolicyCommand {
    Check {
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
        #[arg(long, default_value = "apply")]
        action: String,
        #[arg(long)]
        hop: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConveyCommand {
    /// Declare a hop and write a durable lease.
    Hop {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "box")]
        kind: String,
        #[arg(long, default_value = "lane-tool")]
        capability: String,
        #[arg(long, default_value = "any")]
        host_class: String,
        #[arg(long, default_value_t = true)]
        wired: bool,
        #[arg(long)]
        ttl_secs: Option<u64>,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Lease-bound call. Refuses without a granted lease.
    Call {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "lane-tool")]
        capability: String,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
    },
    /// List declared hops.
    List {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Print hop leases.
    Leases {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Derive hops from placement-actual.json (slim parse).
    Sync {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// List expired hop leases. Call refuses them. `--forget` drops rows.
    Expire {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value_t = false)]
        forget: bool,
    },
}

#[derive(Subcommand)]
enum PlanAction {
    /// Compare two plans or last-applied vs new. Exit 1 if blast grows.
    Diff {
        /// Previous plan JSON or estate YAML.
        #[arg(long)]
        from: Option<PathBuf>,
        /// New plan JSON or estate YAML.
        #[arg(long)]
        to: Option<PathBuf>,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        /// Allow a wider blast radius (otherwise refuse:wider).
        #[arg(long, default_value_t = false)]
        allow_wider: bool,
    },
}

#[derive(Subcommand)]
enum SessionsCommand {
    /// Print the full session journal.
    List {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Print the last N journal events.
    Tail {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value_t = 20)]
        n: usize,
    },
}

#[derive(Subcommand)]
enum PacksCommand {
    /// List candidate packs and rewrite INDEX.md.
    List {
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
    },
    /// Explicit apply of a pack artifact. Does not rewrite the estate file.
    Import {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
        #[arg(long, default_value = "packs/accepted")]
        accepted_dir: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = "jason")]
        curator: String,
    },
    /// Always fails. Auto-promote is locked off.
    Promote {
        #[arg(long)]
        id: String,
    },
    /// Rewrite packs/INDEX.md.
    Index {
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
    },
    /// Write a proposal pack. Never auto-applies. Jason reviews the diff.
    Propose {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
        #[arg(long, default_value = "packs/accepted")]
        accepted_dir: PathBuf,
        #[arg(long, default_value = "packs/proposed")]
        proposed_dir: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
}

#[derive(Subcommand)]
enum AuditCommand {
    /// Write a reviewable folder (and optional tarball) of local audit files.
    Export {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = "packs")]
        packs_dir: PathBuf,
        #[arg(long, default_value = ".cell/audit-export")]
        out: PathBuf,
        /// Also write `{out}.tar.gz` when `tar` is on PATH.
        #[arg(long, default_value_t = false)]
        tar: bool,
    },
}

#[derive(Subcommand)]
enum FeedCommand {
    /// Read scrubbed events and write a candidate pack (not in the estate).
    Pack {
        #[arg(long, default_value = ".cell/feed")]
        feed_dir: PathBuf,
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
        #[arg(long, default_value = "overnight-traces")]
        id: String,
    },
    /// List candidate packs in the drop zone.
    List {
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
    },
    /// Explicit apply of a pack artifact. Does not rewrite the estate file.
    Import {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
        #[arg(long, default_value = "packs/accepted")]
        accepted_dir: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = "jason")]
        curator: String,
    },
    /// Always fails. Auto-promote is locked off.
    Promote {
        #[arg(long)]
        id: String,
    },
    /// Print durable feed-cursor.json watermark.
    Cursor {
        #[arg(long, default_value = ".cell/feed")]
        feed_dir: PathBuf,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { estate } => cmd_validate(&estate),
        Command::Plan {
            action,
            estate,
            against,
            plans_dir,
            state_dir,
            reviewed,
            reviewed_dir,
        } => match action {
            Some(PlanAction::Diff {
                from,
                to,
                estate: diff_estate,
                state_dir: diff_state,
                plans_dir: diff_plans,
                allow_wider,
            }) => cmd_plan_diff(
                from.as_deref(),
                to.as_deref(),
                &diff_estate,
                &diff_state,
                &diff_plans,
                allow_wider,
            ),
            None => cmd_plan(
                &estate,
                against.as_deref(),
                &plans_dir,
                &state_dir,
                reviewed,
                &reviewed_dir,
            ),
        },
        Command::Apply {
            estate,
            state_dir,
            roots_base,
            plans_dir,
            require_plan,
            import_pack,
            packs_dir,
            require_fresh_plan,
            dry_run,
            policy,
            curator,
        } => cmd_apply(
            &estate,
            &state_dir,
            &roots_base,
            &plans_dir,
            require_plan,
            import_pack.as_deref(),
            &packs_dir,
            require_fresh_plan,
            dry_run,
            &policy,
            &curator,
        ),
        Command::Drift {
            estate,
            state_dir,
            roots_base,
        } => cmd_drift(&estate, &state_dir, &roots_base),
        Command::Models { estate } => cmd_models(&estate),
        Command::Plans { plans_dir } => cmd_plans(&plans_dir),
        Command::Suspend { state_dir } => cmd_suspend(&state_dir),
        Command::Resume {
            estate,
            state_dir,
            roots_base,
        } => cmd_resume(&estate, &state_dir, &roots_base),
        Command::Status {
            estate,
            state_dir,
            roots_base,
            plans_dir,
            packs_dir,
            policy,
            root,
        } => cmd_status(
            &estate,
            &state_dir,
            &roots_base,
            &plans_dir,
            &packs_dir,
            &policy,
            &root,
        ),
        Command::Feed { command } => match command {
            FeedCommand::Pack {
                feed_dir,
                drop_dir,
                id,
            } => cmd_feed_pack(&feed_dir, &drop_dir, &id),
            FeedCommand::List { drop_dir } => cmd_feed_list(&drop_dir),
            FeedCommand::Import {
                id,
                drop_dir,
                accepted_dir,
                estate,
                curator,
            } => cmd_feed_import(&id, &drop_dir, &accepted_dir, &estate, &curator),
            FeedCommand::Promote { id } => cmd_feed_promote(&id),
            FeedCommand::Cursor { feed_dir } => cmd_feed_cursor(&feed_dir),
        },
        Command::Catalog { out } => cmd_catalog(&out),
        Command::Leases { state_dir } => cmd_leases(&state_dir),
        Command::Audits { state_dir } => cmd_audits(&state_dir),
        Command::Reconcile { estate, state_dir } => cmd_reconcile(&estate, &state_dir),
        Command::Audit { command } => match command {
            AuditCommand::Export {
                estate,
                state_dir,
                plans_dir,
                packs_dir,
                out,
                tar,
            } => cmd_audit_export(&estate, &state_dir, &plans_dir, &packs_dir, &out, tar),
        },
        Command::History { state_dir } => cmd_history(&state_dir),
        Command::Probes => cmd_probes(),
        Command::Convey { command } => match command {
            ConveyCommand::Hop {
                id,
                kind,
                capability,
                host_class,
                wired,
                ttl_secs,
                state_dir,
            } => cmd_convey_hop(&id, &kind, &capability, &host_class, wired, ttl_secs, &state_dir),
            ConveyCommand::Call {
                id,
                capability,
                state_dir,
                policy,
            } => cmd_convey_call(&id, &capability, &state_dir, &policy),
            ConveyCommand::List { state_dir } => cmd_convey_list(&state_dir),
            ConveyCommand::Leases { state_dir } => cmd_convey_leases(&state_dir),
            ConveyCommand::Sync { state_dir } => cmd_convey_sync(&state_dir),
            ConveyCommand::Expire { state_dir, forget } => cmd_convey_expire(&state_dir, forget),
        },
        Command::Packs { command } => match command {
            PacksCommand::List { drop_dir } => cmd_feed_list(&drop_dir),
            PacksCommand::Import {
                id,
                drop_dir,
                accepted_dir,
                estate,
                curator,
            } => cmd_feed_import(&id, &drop_dir, &accepted_dir, &estate, &curator),
            PacksCommand::Promote { id } => cmd_feed_promote(&id),
            PacksCommand::Index { drop_dir } => cmd_packs_index(&drop_dir),
            PacksCommand::Propose {
                id,
                drop_dir,
                accepted_dir,
                proposed_dir,
                estate,
            } => cmd_packs_propose(&id, &drop_dir, &accepted_dir, &proposed_dir, &estate),
        },
        Command::Expire { state_dir, forget } => cmd_expire(&state_dir, forget),
        Command::Doctor { root, state_dir } => cmd_doctor(&root, &state_dir),
        Command::Sessions { command } => match command {
            SessionsCommand::List { state_dir } => cmd_sessions_list(&state_dir),
            SessionsCommand::Tail { state_dir, n } => cmd_sessions_tail(&state_dir, n),
        },
        Command::Backup {
            state_dir,
            plans_dir,
            out,
            estate,
            policy,
        } => cmd_backup(&state_dir, &plans_dir, &out, &estate, &policy),
        Command::Restore {
            from,
            state_dir,
            plans_dir,
            estate,
            dry_run,
            policy,
        } => cmd_restore(&from, &state_dir, &plans_dir, &estate, dry_run, &policy),
        Command::Policy { command } => match command {
            PolicyCommand::Check {
                policy,
                action,
                hop,
            } => cmd_policy_check(&policy, &action, hop.as_deref()),
        },
        Command::PauseProof {
            estate,
            state_dir,
            roots_base,
        } => cmd_pause_proof(&estate, &state_dir, &roots_base),
    }
}

fn cmd_validate(path: &Path) -> Result<()> {
    let estate = match load_estate_unvalidated(path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("invalid estate (fail closed):");
            for line in e.errors() {
                eprintln!("  - {line}");
            }
            bail!("validate failed for {}", path.display());
        }
    };
    match validate(&estate) {
        Ok(()) => {
            println!("estate: {}", path.display());
            println!("{}", describe(&estate));
            println!("ok");
            Ok(())
        }
        Err(errors) => {
            eprintln!("invalid estate (fail closed):");
            for err in errors {
                eprintln!("  - {err}");
            }
            bail!("validate failed for {}", path.display());
        }
    }
}

