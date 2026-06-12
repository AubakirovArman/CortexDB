mod audit;
mod backup;
mod ingestion;
mod knowledge;
mod maintenance;
mod paths;
mod search;
mod upgrade_flow;

use clap::{error::ErrorKind, CommandFactory, Parser};

use crate::cli_ops as ops;

use super::args::{Cli, Command};
use paths::resolve_path;

#[derive(Clone, Copy)]
pub(super) struct DispatchContext<'a> {
    pub(super) tenant: Option<&'a str>,
    pub(super) json: bool,
}

impl DispatchContext<'_> {
    pub(super) fn resolve(&self, path: &str) -> std::path::PathBuf {
        resolve_path(path, self.tenant)
    }
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
    let ctx = DispatchContext {
        tenant: cli.tenant.as_deref(),
        json: cli.json,
    };
    match cli.command {
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_owned();
            let mut output = Vec::new();
            clap_complete::generate(shell, &mut cmd, name, &mut output);
            String::from_utf8(output).map_err(|error| error.to_string())
        }
        Command::Version => Ok(format!("cortexdb {}", env!("CARGO_PKG_VERSION"))),
        Command::Backup { path, backup_path } => backup::backup(ctx, path, backup_path),
        Command::BackupEncrypted {
            path,
            archive_path,
            passphrase,
            passphrase_env,
        } => backup::backup_encrypted(ctx, path, archive_path, passphrase, passphrase_env),
        Command::BackupDrill {
            path,
            backup_path,
            restore_path,
        } => backup::backup_drill(ctx, path, backup_path, restore_path),
        Command::BackupPrune {
            backup_root,
            prefix,
            keep_latest,
            dry_run,
        } => backup::backup_prune(backup_root, prefix, keep_latest, dry_run),
        Command::BackupOffsiteStage {
            backup_path,
            offsite_root,
            backup_id,
        } => backup::backup_offsite_stage(backup_path, offsite_root, backup_id),
        Command::Upgrade { command } => upgrade_flow::upgrade(ctx, command),
        Command::Migrate {
            path,
            backup_path,
            drill_restore_path,
        } => upgrade_flow::migrate(ctx, path, backup_path, drill_restore_path),
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
            audit::review(audit::AuditReviewDispatch {
                ctx,
                path: actual_path,
                route,
                status,
                action,
                tenant_filter,
                summary_only: summary || audit_verify_alias,
                redaction_check,
                verify_chain: verify_chain || audit_verify_alias,
            })
        }
        Command::AuditExportSiem {
            input_path,
            output_path,
            redaction_check,
            verify_chain,
        } => audit::export_siem(ctx, input_path, output_path, redaction_check, verify_chain),
        Command::AuthReview {
            policy_store,
            tokens_file,
            tokens,
        } => audit::auth_review(ctx, policy_store, tokens_file, tokens),
        Command::Restore {
            backup_path,
            path,
            dry_run,
        } => backup::restore(ctx, backup_path, path, dry_run),
        Command::RestoreEncrypted {
            archive_path,
            path,
            passphrase,
            passphrase_env,
        } => backup::restore_encrypted(ctx, archive_path, path, passphrase, passphrase_env),
        Command::Context {
            path,
            scope,
            aql,
            format,
        } => knowledge::context(ctx, path, scope, aql, format),
        Command::Remember { path, scope, aql } => knowledge::remember(ctx, path, scope, aql),
        Command::Forget { path, cell_id } => knowledge::forget(ctx, path, cell_id),
        Command::Verify {
            path,
            scope,
            aql,
            format,
        } => knowledge::verify(ctx, path, scope, aql, format),
        Command::Aql { path, scope, aql } => knowledge::aql(ctx, path, scope, aql),
        Command::Search {
            path,
            scope,
            query,
            mode,
            vector,
            algorithm,
        } => search::search(ctx, path, scope, query, mode, vector, algorithm),
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
        } => search::search_vector(
            ctx,
            search::VectorSearchInput {
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
            },
        ),
        Command::SearchVectorExact {
            path,
            scope,
            vector,
        } => search::search_vector_exact(ctx, path, scope, vector),
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
        } => search::search_vector_eval(
            ctx,
            search::VectorSearchInput {
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
            },
        ),
        Command::SearchExplain {
            path,
            scope,
            query,
            mode,
            vector,
        } => search::search_explain(ctx, path, scope, query, mode, vector),
        Command::LoadFixture { path, fixture_path } => {
            ingestion::load_fixture(ctx, path, fixture_path)
        }
        Command::IngestText { path, scope, file } => ingestion::text(ctx, path, scope, file),
        Command::IngestJson { path, scope, file } => ingestion::json(ctx, path, scope, file),
        Command::IngestCsv { path, scope, file } => ingestion::csv(ctx, path, scope, file),
        Command::IngestJobs { path } => ingestion::jobs(ctx, path),
        Command::IngestJob { path, job_id } => ingestion::job(ctx, path, job_id),
        Command::IngestJobCancel { path, job_id } => ingestion::cancel_job(ctx, path, job_id),
        Command::IngestJobRetry { path, job_id } => ingestion::retry_job(ctx, path, job_id),
        Command::IngestJobDelete { path, job_id } => ingestion::delete_job(ctx, path, job_id),
        command => maintenance::run(ctx, command),
    }
}
