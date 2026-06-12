use clap::{error::ErrorKind, CommandFactory, Parser};

use crate::{
    cli_ann as ann, cli_aql as aql_cmd, cli_audit as audit, cli_audit_siem as audit_siem,
    cli_auth_review as auth_review, cli_ingest as ingest, cli_ops as ops, cli_upgrade as upgrade,
};

use super::args::{Cli, Command, UpgradeCommand, VectorCommand};

fn resolve_path(path: &str, tenant: Option<&str>) -> std::path::PathBuf {
    let base = std::path::PathBuf::from(path);
    match tenant {
        Some(t) if t != "default" => base.join("realms").join(t),
        _ => base,
    }
}

fn resolve_backup_passphrase(
    passphrase: Option<String>,
    passphrase_env: Option<String>,
) -> Result<String, String> {
    if let Some(value) = passphrase {
        return Ok(value);
    }
    let env_name = passphrase_env.unwrap_or_else(|| "CORTEXDB_BACKUP_PASSPHRASE".to_owned());
    std::env::var(&env_name).map_err(|_| {
        format!(
            "encrypted backup passphrase is required; set {env_name} or pass --passphrase-env <VAR>"
        )
    })
}

pub fn run(args: Vec<String>) -> Result<String, String> {
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            return Ok(err.to_string());
        }
        Err(err) => return Err(err.to_string()),
    };
    if let Some(tenant) = cli.tenant.as_deref() {
        if !ops::validate_tenant_id(tenant) {
            return Err(
                "tenant is invalid; allowed: ASCII alphanumeric, '_' and '-', max 64 characters"
                    .to_owned(),
            );
        }
    }
    let resolved = |p: &str| resolve_path(p, cli.tenant.as_deref());
    match cli.command {
        Command::Demo => ops::run_demo(),
        Command::Doctor { path } => {
            ops::doctor(resolved(&path).to_str().unwrap(), cli.tenant.as_deref())
        }
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_owned();
            let mut output = Vec::new();
            clap_complete::generate(shell, &mut cmd, name, &mut output);
            String::from_utf8(output).map_err(|error| error.to_string())
        }
        Command::Version => Ok(format!("cortexdb {}", env!("CARGO_PKG_VERSION"))),
        Command::Put {
            path,
            cell_id,
            payload,
        } => ops::put(resolved(&path).to_str().unwrap(), &cell_id, &payload),
        Command::Get { path, cell_id } => {
            ops::get(resolved(&path).to_str().unwrap(), &cell_id, cli.json)
        }
        Command::Tombstone { path, cell_id } => {
            ops::tombstone(resolved(&path).to_str().unwrap(), &cell_id)
        }
        Command::Flush {
            path,
            experimental_hnsw,
        } => ops::flush(resolved(&path).to_str().unwrap(), experimental_hnsw),
        Command::Compact {
            path,
            experimental_hnsw,
        } => ops::compact(resolved(&path).to_str().unwrap(), experimental_hnsw),
        Command::Stats { path } => ops::stats(resolved(&path).to_str().unwrap(), cli.json),
        Command::Validate { path } => ops::validate(resolved(&path).to_str().unwrap(), cli.json),
        Command::Vector { command } => match command {
            VectorCommand::Rebuild {
                path,
                experimental_hnsw,
            } => ops::vector_rebuild(
                resolved(&path).to_str().unwrap(),
                experimental_hnsw,
                cli.json,
            ),
        },
        Command::AnnValidate { path } => {
            ops::ann_validate(resolved(&path).to_str().unwrap(), cli.json)
        }
        Command::HnswNoFallbackProfileShow { path } => {
            ops::hnsw_no_fallback_profile_show(resolved(&path).to_str().unwrap(), cli.json)
        }
        Command::HnswNoFallbackProfileSet {
            path,
            enabled,
            min_recall,
            require_upper_layers,
        } => {
            let policy = ann::parse_no_fallback_profile(enabled, min_recall, require_upper_layers)?;
            ops::hnsw_no_fallback_profile_set(resolved(&path).to_str().unwrap(), policy, cli.json)
        }
        Command::HnswNoFallbackProfileClear { path } => {
            ops::hnsw_no_fallback_profile_clear(resolved(&path).to_str().unwrap(), cli.json)
        }
        Command::Repair { path, dry_run } => {
            ops::repair(resolved(&path).to_str().unwrap(), dry_run)
        }
        Command::Backup { path, backup_path } => {
            ops::backup(resolved(&path).to_str().unwrap(), &backup_path)
        }
        Command::BackupEncrypted {
            path,
            archive_path,
            passphrase,
            passphrase_env,
        } => {
            let passphrase = resolve_backup_passphrase(passphrase, passphrase_env)?;
            ops::backup_encrypted(
                resolved(&path).to_str().unwrap(),
                &archive_path,
                &passphrase,
            )
        }
        Command::BackupDrill {
            path,
            backup_path,
            restore_path,
        } => ops::backup_drill(
            resolved(&path).to_str().unwrap(),
            &backup_path,
            &restore_path,
        ),
        Command::BackupPrune {
            backup_root,
            prefix,
            keep_latest,
            dry_run,
        } => ops::backup_prune(&backup_root, &prefix, keep_latest, dry_run),
        Command::BackupOffsiteStage {
            backup_path,
            offsite_root,
            backup_id,
        } => ops::backup_offsite_stage(&backup_path, &offsite_root, &backup_id),
        Command::Upgrade { command } => match command {
            UpgradeCommand::Prepare {
                path,
                backup_path,
                drill_restore_path,
            } => upgrade::prepare(
                resolved(&path).to_str().unwrap(),
                &backup_path,
                &drill_restore_path,
                cli.json,
            ),
            UpgradeCommand::Validate { path } => {
                upgrade::validate_after_upgrade(resolved(&path).to_str().unwrap(), cli.json)
            }
            UpgradeCommand::Rollback {
                backup_path,
                rollback_path,
            } => upgrade::rollback(
                &backup_path,
                resolved(&rollback_path).to_str().unwrap(),
                cli.json,
            ),
        },
        Command::Migrate {
            path,
            backup_path,
            drill_restore_path,
        } => upgrade::migrate(
            resolved(&path).to_str().unwrap(),
            &backup_path,
            &drill_restore_path,
            cli.json,
        ),
        Command::Audit {
            path,
            verify_path,
            route,
            status,
            action,
            tenant_filter,
            summary,
            redaction_check,
            verify_chain,
        } => {
            let audit_verify_alias = path == "verify";
            let actual_path = if audit_verify_alias {
                verify_path.as_deref().ok_or_else(|| {
                    "usage: cortexdb audit verify <audit.jsonl> [--redaction-check]".to_owned()
                })?
            } else {
                if verify_path.is_some() {
                    return Err(
                        "unexpected extra audit path; use `cortexdb audit verify <audit.jsonl>` \
                         or `cortexdb audit <audit.jsonl> --verify-chain`"
                            .to_owned(),
                    );
                }
                path.as_str()
            };
            audit::review(audit::AuditReviewOptions {
                path: actual_path,
                route: route.as_deref(),
                status,
                action: action.as_deref(),
                tenant: tenant_filter.as_deref(),
                summary_only: summary || audit_verify_alias,
                redaction_check,
                verify_chain: verify_chain || audit_verify_alias,
                json: cli.json,
            })
        }
        Command::AuditExportSiem {
            input_path,
            output_path,
            redaction_check,
            verify_chain,
        } => audit_siem::export_jsonl(
            &input_path,
            &output_path,
            redaction_check,
            verify_chain,
            cli.json,
        ),
        Command::AuthReview {
            policy_store,
            tokens_file,
            tokens,
        } => auth_review::review(auth_review::AuthReviewOptions {
            policy_store: policy_store.as_deref(),
            tokens_file: tokens_file.as_deref(),
            tokens: tokens.as_deref(),
            json: cli.json,
        }),
        Command::Restore {
            backup_path,
            path,
            dry_run,
        } => ops::restore(&backup_path, resolved(&path).to_str().unwrap(), dry_run),
        Command::RestoreEncrypted {
            archive_path,
            path,
            passphrase,
            passphrase_env,
        } => {
            let passphrase = resolve_backup_passphrase(passphrase, passphrase_env)?;
            ops::restore_encrypted(
                &archive_path,
                resolved(&path).to_str().unwrap(),
                &passphrase,
            )
        }
        Command::GcRetired { path } => ops::gc_retired(resolved(&path).to_str().unwrap()),
        Command::WalValidate { path } => ops::wal_validate(resolved(&path).to_str().unwrap()),
        Command::WalDump { path } => ops::wal_dump(resolved(&path).to_str().unwrap()),
        Command::WalTruncate { path } => ops::wal_truncate(resolved(&path).to_str().unwrap()),
        Command::ManifestDump { path } => ops::manifest_dump(resolved(&path).to_str().unwrap()),
        Command::ManifestValidate { path } => {
            ops::manifest_validate(resolved(&path).to_str().unwrap())
        }
        Command::Context {
            path,
            scope,
            aql,
            format,
        } => ops::context(
            resolved(&path).to_str().unwrap(),
            &scope,
            &aql,
            cli.json,
            format.as_str(),
        ),
        Command::Remember { path, scope, aql } => {
            ops::remember(resolved(&path).to_str().unwrap(), &scope, &aql, cli.json)
        }
        Command::Forget { path, cell_id } => {
            ops::forget(resolved(&path).to_str().unwrap(), &cell_id, cli.json)
        }
        Command::Verify {
            path,
            scope,
            aql,
            format,
        } => ops::verify(
            resolved(&path).to_str().unwrap(),
            &scope,
            &aql,
            cli.json,
            format.as_str(),
        ),
        Command::Aql { path, scope, aql } => {
            aql_cmd::aql(resolved(&path).to_str().unwrap(), &scope, &aql, cli.json)
        }
        Command::Search {
            path,
            scope,
            query,
            mode,
            vector,
            algorithm,
        } => ops::search(
            resolved(&path).to_str().unwrap(),
            &scope,
            &query,
            cli.json,
            &mode,
            vector.as_deref(),
            &algorithm,
        ),
        Command::SearchVector {
            path,
            scope,
            vector,
            fallback,
            fallback_scan_cap,
            min_recall,
            max_visited_candidates,
            require_slo,
            no_fallback_rollout,
            no_fallback_min_recall,
            use_no_fallback_profile,
            experimental_hnsw,
        } => {
            let policy = ann::parse_ann_policy(
                fallback,
                fallback_scan_cap,
                min_recall,
                max_visited_candidates,
                require_slo,
            )?;
            let rollout_policy =
                ann::parse_no_fallback_rollout_policy(no_fallback_rollout, no_fallback_min_recall)?;
            ops::search_vector(ops::SearchVectorOptions {
                path: resolved(&path).to_str().unwrap(),
                scope: &scope,
                vector: &vector,
                exact: false,
                policy: Some(policy),
                rollout_policy,
                use_no_fallback_profile,
                experimental_hnsw,
            })
        }
        Command::SearchVectorExact {
            path,
            scope,
            vector,
        } => ops::search_vector(ops::SearchVectorOptions {
            path: resolved(&path).to_str().unwrap(),
            scope: &scope,
            vector: &vector,
            exact: true,
            policy: None,
            rollout_policy: None,
            use_no_fallback_profile: false,
            experimental_hnsw: false,
        }),
        Command::SearchVectorEval {
            path,
            scope,
            vector,
            fallback,
            fallback_scan_cap,
            min_recall,
            max_visited_candidates,
            require_slo,
            no_fallback_rollout,
            no_fallback_min_recall,
            use_no_fallback_profile,
            experimental_hnsw,
        } => {
            let policy = ann::parse_ann_policy(
                fallback,
                fallback_scan_cap,
                min_recall,
                max_visited_candidates,
                require_slo,
            )?;
            let rollout_policy =
                ann::parse_no_fallback_rollout_policy(no_fallback_rollout, no_fallback_min_recall)?;
            ann::search_vector_eval(ann::SearchVectorEvalOptions {
                path: resolved(&path).to_str().unwrap(),
                scope: &scope,
                vector: &vector,
                json: cli.json,
                policy: Some(policy),
                rollout_policy,
                use_no_fallback_profile,
                experimental_hnsw,
            })
        }
        Command::SearchExplain {
            path,
            scope,
            query,
            mode,
            vector,
        } => ops::search_explain(
            resolved(&path).to_str().unwrap(),
            &scope,
            &query,
            &mode,
            vector.as_deref(),
        ),
        Command::Unlock { path, force } => ops::unlock(resolved(&path).to_str().unwrap(), force),
        Command::LoadFixture { path, fixture_path } => {
            ingest::load_fixture(resolved(&path).to_str().unwrap(), &fixture_path)
        }
        Command::IngestText { path, scope, file } => {
            ingest::text(resolved(&path).to_str().unwrap(), &scope, &file)
        }
        Command::IngestJson { path, scope, file } => {
            ingest::json(resolved(&path).to_str().unwrap(), &scope, &file)
        }
        Command::IngestCsv { path, scope, file } => {
            ingest::csv(resolved(&path).to_str().unwrap(), &scope, &file)
        }
        Command::IngestJobs { path } => ingest::jobs(resolved(&path).to_str().unwrap(), cli.json),
        Command::IngestJob { path, job_id } => {
            ingest::job(resolved(&path).to_str().unwrap(), job_id, cli.json)
        }
        Command::IngestJobCancel { path, job_id } => {
            ingest::cancel_job(resolved(&path).to_str().unwrap(), job_id, cli.json)
        }
        Command::IngestJobRetry { path, job_id } => {
            ingest::retry_job(resolved(&path).to_str().unwrap(), job_id, cli.json)
        }
        Command::IngestJobDelete { path, job_id } => {
            ingest::delete_job(resolved(&path).to_str().unwrap(), job_id)
        }
    }
}
