use anyhow::{bail, Result};
use clap::Parser;
use estate_schema::{describe, load_estate_unvalidated, validate};
use std::path::{Path, PathBuf};

use crate::cli::{
    AuditCommand, ClassifyCommand, Cli, Command, ConveyCommand, EnrichCommand, FeedCommand,
    PacksCommand, PlanAction, PolicyCommand, SessionsCommand,
};
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
                agents,
                estate,
                state_dir,
            } => cmd_convey_hop(
                &id,
                &kind,
                &capability,
                &host_class,
                wired,
                ttl_secs,
                &agents,
                estate.as_deref(),
                &state_dir,
            ),
            ConveyCommand::Call {
                id,
                capability,
                agent,
                kind,
                estate,
                state_dir,
                policy,
            } => cmd_convey_call(
                &id,
                &capability,
                agent.as_deref(),
                kind.as_deref(),
                &estate,
                &state_dir,
                &policy,
            ),
            ConveyCommand::List { state_dir } => cmd_convey_list(&state_dir),
            ConveyCommand::Leases { state_dir } => cmd_convey_leases(&state_dir),
            ConveyCommand::Sync { state_dir, estate } => {
                cmd_convey_sync(&state_dir, estate.as_deref())
            }
            ConveyCommand::Authority { state_dir, estate } => {
                cmd_convey_authority(&state_dir, &estate)
            }
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
                max_steps,
                official_scale,
                from_feed,
            } => crate::enrich::cmd_enrich_prepare(
                &estate,
                &pack,
                &packs_dir,
                driver.as_deref(),
                all_drivers,
                out.as_deref(),
                &state_dir,
                job.as_deref(),
                &curator,
                max_steps,
                official_scale,
                from_feed,
            ),
            EnrichCommand::FromPack {
                estate,
                pack,
                packs_dir,
                driver,
                all_drivers,
                state_dir,
                job,
                curator,
                max_steps,
                official_scale,
                from_feed,
            } => crate::enrich::cmd_enrich_from_pack(
                &estate,
                &pack,
                &packs_dir,
                driver.as_deref(),
                all_drivers,
                &state_dir,
                job.as_deref(),
                &curator,
                max_steps,
                official_scale,
                from_feed,
            ),
            EnrichCommand::List { state_dir } => crate::enrich::cmd_enrich_list(&state_dir),
            EnrichCommand::ImportPrepared {
                estate,
                prepared,
                tag,
                path,
                curator,
            } => {
                crate::enrich::cmd_enrich_import_prepared(&estate, &prepared, &tag, &path, &curator)
            }
            EnrichCommand::ImportTrained {
                estate,
                prepared,
                tag,
                adapter,
                curator,
            } => crate::enrich::cmd_enrich_import_trained(
                &estate, &prepared, &tag, &adapter, &curator,
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
            EnrichCommand::MergeAdapt { prepared, adapter } => {
                crate::enrich::cmd_enrich_merge_adapt(&prepared, &adapter)
            }
            EnrichCommand::GgufConvert { prepared, weights } => {
                crate::enrich::cmd_enrich_gguf_convert(&prepared, &weights)
            }
            EnrichCommand::LocalSeat {
                prepared,
                weights,
                adapter,
                runtime,
            } => crate::enrich::cmd_enrich_local_seat(
                &prepared,
                weights.as_deref(),
                adapter.as_deref(),
                &runtime,
            ),
            EnrichCommand::Drivers => crate::enrich::cmd_enrich_drivers(),
        },
        Command::Classify { command } => match command {
            ClassifyCommand::Import {
                dataset,
                train_size,
                heldout_size,
                seed,
                out,
                force,
                native_train,
                native_test,
                from_local,
                fetch,
                python,
            } => {
                let preset = crate::classify_import::preset_by_name(&dataset)?;
                let size = crate::classify_import::parse_split_size(&train_size)?;
                let out = out.unwrap_or_else(|| {
                    crate::classify_import::sampled_import_dir(preset.alias, &size.token(), seed)
                });
                crate::classify_import::cmd_classify_import(
                    &crate::classify_import::ImportRequest {
                        dataset: preset.alias,
                        train_size: &train_size,
                        heldout_size: &heldout_size,
                        seed,
                        out: &out,
                        force,
                        native_train: native_train.as_deref(),
                        native_test: native_test.as_deref(),
                        from_local: from_local.as_deref(),
                        fetch,
                        python: python.as_deref(),
                        cache_root: None,
                    },
                )
            }
            ClassifyCommand::Prepare {
                input,
                out,
                seed,
                held_out_ratio,
                format,
                dataset_name,
                strict,
                force,
            } => crate::classify::cmd_classify_prepare(
                &input,
                &out,
                seed,
                held_out_ratio,
                format,
                &dataset_name,
                strict,
                force,
            ),
            ClassifyCommand::Eval {
                records,
                endpoint,
                model,
                api_key_env,
                report,
                dry_run,
                mock,
                timeout_secs,
                api,
                few_shot,
                exemplars,
                seed,
            } => {
                let report =
                    report.unwrap_or_else(|| crate::classify::default_report_path(&records));
                let shot = match few_shot {
                    Some(0) => anyhow::bail!(
                        "refuse:classify-eval: --few-shot must be at least 1; omit the flag for zero-shot"
                    ),
                    Some(_) if exemplars.is_none() => anyhow::bail!(
                        "refuse:classify-eval: --few-shot requires --exemplars"
                    ),
                    None if exemplars.is_some() => anyhow::bail!(
                        "refuse:classify-eval: --exemplars requires --few-shot"
                    ),
                    Some(n) => Some(crate::classify::FewShot {
                        n,
                        exemplars: exemplars.as_deref().unwrap(),
                        seed,
                    }),
                    None => None,
                };
                crate::classify::cmd_classify_eval(
                    &records,
                    endpoint.as_deref(),
                    &model,
                    api_key_env.as_deref(),
                    &report,
                    dry_run,
                    mock,
                    timeout_secs,
                    api,
                    crate::classify::EvalGate::Standalone,
                    shot.as_ref(),
                )
            }
            ClassifyCommand::Grade {
                dataset,
                from_local,
                tasks,
                completions,
                out,
                print,
                run,
                limit,
                timeout_secs,
                use_canonical,
            } => crate::classify_grade::cmd_classify_grade(&crate::classify_grade::GradeRequest {
                dataset: dataset.as_deref(),
                from_local: from_local.as_deref(),
                tasks: tasks.as_deref(),
                completions: completions.as_deref(),
                out: &out,
                print,
                run,
                limit,
                timeout_secs,
                use_canonical,
            }),
            ClassifyCommand::Journey {
                input,
                out,
                preset,
                base,
                base_cache,
                base_tag,
                tag,
                endpoint,
                dataset_name,
                seed,
                held_out_ratio,
                max_steps,
                quant,
                llama_cpp_dir,
                print,
                run,
                force,
                min_delta,
                min_accuracy,
                require_significant_lift,
                timeout_secs,
                together_poll_secs,
                train_driver,
                together_model,
                together_base_url,
                api_key_env,
                dataset,
                train_size,
                heldout_size,
                from_local,
                fetch,
                python,
                few_shot,
                expand_tag,
                dual,
            } => {
                let input = input
                    .unwrap_or_else(|| PathBuf::from("examples/fixtures/tev1-decisions.jsonl"));
                let llama_cpp_dir =
                    llama_cpp_dir.or_else(|| std::env::var_os("LLAMA_CPP_DIR").map(PathBuf::from));
                if dual {
                    return crate::classify_journey::cmd_classify_journey_dual(
                        &crate::classify_journey::DualJourneyRequest {
                            input: &input,
                            out: &out,
                            base: &base,
                            base_tag: base_tag.as_deref(),
                            tag: &tag,
                            endpoint: &endpoint,
                            dataset_name: &dataset_name,
                            seed,
                            held_out_ratio,
                            max_steps,
                            quant: &quant,
                            llama_cpp_dir: llama_cpp_dir.as_deref(),
                            force,
                            print,
                            run,
                            min_delta,
                            min_accuracy,
                            require_significant_lift,
                            timeout_secs,
                            together_poll_secs,
                            train_driver,
                            together_model: together_model.as_deref(),
                            together_base_url: &together_base_url,
                            api_key_env: api_key_env.as_deref(),
                            preset,
                            import_dataset: dataset.as_deref(),
                            train_size: &train_size,
                            heldout_size: &heldout_size,
                            from_local: from_local.as_deref(),
                            import_fetch: fetch,
                            python: python.as_deref(),
                            base_cache: &base_cache,
                            few_shot,
                            expand_tag: expand_tag.as_deref(),
                        },
                    );
                }
                let applied = crate::classify_journey::apply_preset(
                    preset,
                    &base,
                    &tag,
                    train_driver,
                    together_model.as_deref(),
                )?;
                let (tag, out, dataset_name) = if let Some(dataset) = dataset.as_deref() {
                    crate::classify_journey::apply_dataset_layout(
                        dataset,
                        &train_size,
                        &applied.tag,
                        &tag,
                        &out,
                        &dataset_name,
                        expand_tag.as_deref(),
                    )?
                } else {
                    (applied.tag.clone(), out.clone(), dataset_name)
                };
                crate::classify_journey::cmd_classify_journey(
                    &crate::classify_journey::JourneyRequest {
                        input: &input,
                        out: &out,
                        base: &applied.base,
                        base_tag: base_tag.as_deref(),
                        tag: &tag,
                        endpoint: &endpoint,
                        dataset_name: &dataset_name,
                        seed,
                        held_out_ratio,
                        max_steps,
                        quant: &quant,
                        llama_cpp_dir: llama_cpp_dir.as_deref(),
                        force,
                        print,
                        run,
                        min_delta,
                        min_accuracy,
                        require_significant_lift,
                        timeout_secs,
                        together_poll_secs,
                        train_driver,
                        together_model: &applied.together_model,
                        together_base_url: &together_base_url,
                        api_key_env: api_key_env.as_deref(),
                        built_base_tag: applied.built_base_tag,
                        seat: applied.seat,
                        llama_note: applied.llama_note,
                        preset,
                        import_dataset: dataset.as_deref(),
                        train_size: &train_size,
                        heldout_size: &heldout_size,
                        from_local: from_local.as_deref(),
                        import_fetch: fetch,
                        python: python.as_deref(),
                        base_cache: &base_cache,
                        few_shot,
                        expand_tag: expand_tag.as_deref(),
                    },
                )
            }
            ClassifyCommand::Expand {
                dataset,
                train,
                heldout,
                from_local,
                out,
                tag,
                train_size,
                seed,
                print,
                run,
                endpoint,
                model,
                api_key_env,
                timeout_secs,
            } => crate::classify_expand::cmd_classify_expand(
                &crate::classify_expand::ExpandRequest {
                    dataset: &dataset,
                    train: train.as_deref(),
                    heldout: heldout.as_deref(),
                    from_local: from_local.as_deref(),
                    out: out.as_deref(),
                    tag: &tag,
                    train_size: &train_size,
                    seed,
                    print,
                    run,
                    endpoint: endpoint.as_deref(),
                    model: model.as_deref(),
                    api_key_env: api_key_env.as_deref(),
                    timeout_secs,
                },
            ),
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
            } => {
                crate::heal::cmd_packs_accept(&id, &proposed_dir, &accepted_dir, &estate, &curator)
            }
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
