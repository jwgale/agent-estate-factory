use anyhow::{bail, Result};
use clap::Parser;
use estate_schema::{describe, load_estate_unvalidated, validate};
use std::path::Path;

use crate::cli::{AuditCommand, Cli, Command, ConveyCommand, EnrichCommand, FeedCommand, PacksCommand, PlanAction, PolicyCommand, SessionsCommand};
use crate::ops::*;
use crate::plan_apply::*;
use crate::watch::*;

pub(crate) fn run() -> Result<()> {
    let cli = Cli::parse();
    crate::helpers::install_sacred(&cli.sacred)?;
    match cli.command {
        Command::Help { topic } => crate::help::cmd_help(topic.as_deref()),
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
            Some(PlanAction::ExportPr {
                estate: pr_estate,
                state_dir: pr_state,
                plans_dir: pr_plans,
                reviewed_dir: pr_reviewed,
                out,
            }) => cmd_plan_export_pr(&pr_estate, &pr_state, &pr_plans, &pr_reviewed, &out),
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
            force,
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
            force,
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
        Command::Reconcile {
            estate,
            state_dir,
            suggest,
        } => crate::heal::cmd_reconcile(&estate, &state_dir, suggest),
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
        Command::Probes { live } => crate::heal::cmd_probes(live),
        Command::Specialist {
            endpoint,
            driver,
            job,
            agent,
            kind,
            prompt,
            text,
        } => crate::heal::cmd_specialist(endpoint, &driver, &job, &agent, &kind, prompt, text),
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
        Command::Enrich { command } => match command {
            EnrichCommand::Prepare {
                estate,
                pack,
                packs_dir,
                driver,
                all_drivers,
                out,
                state_dir,
                job,
                curator,
            } => crate::enrich::cmd_enrich_prepare(
                &estate,
                &pack,
                &packs_dir,
                driver.as_deref(),
                all_drivers,
                out.as_deref(),
                &state_dir,
                &job,
                &curator,
            ),
            EnrichCommand::List { state_dir } => crate::enrich::cmd_enrich_list(&state_dir),
            EnrichCommand::ImportPrepared {
                estate,
                prepared,
                tag,
                path,
                curator,
            } => crate::enrich::cmd_enrich_import_prepared(
                &estate,
                &prepared,
                &tag,
                &path,
                &curator,
            ),
            EnrichCommand::ApplyProposal {
                estate,
                prepared,
                tag,
                state_dir,
                plans_dir,
                curator,
                verify_local_tag,
            } => crate::enrich::cmd_enrich_apply_proposal(
                &estate,
                &prepared,
                &tag,
                &state_dir,
                &plans_dir,
                &curator,
                verify_local_tag,
            ),
            EnrichCommand::Drivers => crate::enrich::cmd_enrich_drivers(),
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
            PacksCommand::Accept {
                id,
                curator,
                proposed_dir,
                accepted_dir,
                estate,
            } => crate::heal::cmd_packs_accept(&id, &proposed_dir, &accepted_dir, &estate, &curator),
        },
        Command::Expire { state_dir, forget } => cmd_expire(&state_dir, forget),
        Command::Doctor {
            root,
            state_dir,
            strict,
        } => {
            if strict {
                crate::doctor_strict::cmd_doctor_strict(&root, &state_dir)
            } else {
                cmd_doctor(&root, &state_dir)
            }
        }
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
            prune,
        } => cmd_backup(&state_dir, &plans_dir, &out, &estate, &policy, prune),
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
