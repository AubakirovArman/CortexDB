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

use super::args::{AgentCommand, AgentScopeAccessArg, Cli, Command};
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
        Command::BackupVerify { backup_path } => backup::backup_verify(backup_path),
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
            dry_run,
        } => upgrade_flow::migrate(ctx, path, backup_path, drill_restore_path, dry_run),
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
        Command::Agent { command } => match command {
            AgentCommand::Create(input) => ops::create_agent(
                ctx.json,
                ops::AgentCreateInput {
                    path: ctx.resolve(&input.path).to_string_lossy().into_owned(),
                    agent_id: input.agent_id,
                    label: input.label,
                    readable_scopes: input.readable_scopes,
                    writable_scopes: input.writable_scopes,
                    readable_brains: input.readable_brains,
                    allowed_modes: input.allowed_modes,
                    allowed_memory_types: input.allowed_memory_types,
                    max_context_budget_tokens: input.max_context_budget_tokens,
                    default_context_budget_tokens: input.default_context_budget_tokens,
                    max_candidate_limit: input.max_candidate_limit,
                    default_candidate_limit: input.default_candidate_limit,
                    min_required_confidence_q16: input.min_required_confidence_q16,
                    max_ttl_seconds: input.max_ttl_seconds,
                    private_scope: input.private_scope,
                    allow_remember: !input.deny_remember,
                    allow_verify_fact: !input.deny_verify_fact,
                    allow_audit_mode: input.allow_audit_mode,
                    require_citations_by_default: input.require_citations,
                },
            ),
            AgentCommand::List { path } => {
                ops::list_agents(ctx.json, ctx.resolve(&path).to_string_lossy().into_owned())
            }
            AgentCommand::Show { path, agent_id } => ops::show_agent(
                ctx.json,
                ctx.resolve(&path).to_string_lossy().into_owned(),
                agent_id,
            ),
            AgentCommand::Grant(input) => ops::grant_agent_scope(
                ctx.json,
                ops::AgentScopeInput {
                    path: ctx.resolve(&input.path).to_string_lossy().into_owned(),
                    agent_id: input.agent_id,
                    scope: input.scope,
                    access: agent_scope_access(input.access),
                },
            ),
            AgentCommand::Revoke(input) => ops::revoke_agent_scope(
                ctx.json,
                ops::AgentScopeInput {
                    path: ctx.resolve(&input.path).to_string_lossy().into_owned(),
                    agent_id: input.agent_id,
                    scope: input.scope,
                    access: agent_scope_access(input.access),
                },
            ),
        },
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
        Command::Context { input, format } => {
            knowledge::context(ctx, input.path, input.scope, input.aql, format)
        }
        Command::Explain { input, cell_id } => {
            knowledge::explain_cell(ctx, input.path, input.scope, input.aql, cell_id)
        }
        Command::Remember { input } => knowledge::remember(ctx, input.path, input.scope, input.aql),
        Command::Forget { path, cell_id } => knowledge::forget(ctx, path, cell_id),
        Command::Verify { input, format } => {
            knowledge::verify(ctx, input.path, input.scope, input.aql, format)
        }
        Command::Aql { input, explain } => {
            knowledge::aql(ctx, input.path, input.scope, input.aql, explain)
        }
        Command::Search {
            path,
            scope,
            query,
            mode,
            vector,
            algorithm,
        } => search::search(ctx, path, scope, query, mode, vector, algorithm),
        Command::SearchVector { input, policy } => search::search_vector(
            ctx,
            search::VectorSearchInput {
                path: input.path,
                scope: input.scope,
                vector: input.vector,
                fallback: policy.fallback,
                fallback_scan_cap: policy.fallback_scan_cap,
                min_recall: policy.min_recall,
                max_visited_candidates: policy.max_visited_candidates,
                require_slo: policy.require_slo,
                no_fallback_rollout: policy.no_fallback_rollout,
                no_fallback_min_recall: policy.no_fallback_min_recall,
                use_no_fallback_profile: policy.use_no_fallback_profile,
                experimental_hnsw: policy.experimental_hnsw,
            },
        ),
        Command::SearchVectorExact { input } => {
            search::search_vector_exact(ctx, input.path, input.scope, input.vector)
        }
        Command::SearchVectorEval { input, policy } => search::search_vector_eval(
            ctx,
            search::VectorSearchInput {
                path: input.path,
                scope: input.scope,
                vector: input.vector,
                fallback: policy.fallback,
                fallback_scan_cap: policy.fallback_scan_cap,
                min_recall: policy.min_recall,
                max_visited_candidates: policy.max_visited_candidates,
                require_slo: policy.require_slo,
                no_fallback_rollout: policy.no_fallback_rollout,
                no_fallback_min_recall: policy.no_fallback_min_recall,
                use_no_fallback_profile: policy.use_no_fallback_profile,
                experimental_hnsw: policy.experimental_hnsw,
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

fn agent_scope_access(access: AgentScopeAccessArg) -> ops::AgentScopeAccess {
    match access {
        AgentScopeAccessArg::Read => ops::AgentScopeAccess::Read,
        AgentScopeAccessArg::Write => ops::AgentScopeAccess::Write,
        AgentScopeAccessArg::ReadWrite => ops::AgentScopeAccess::ReadWrite,
    }
}
