use clap::{error::ErrorKind, Parser, Subcommand};

use crate::{
    cli_ann as ann, cli_audit as audit, cli_audit_siem as audit_siem,
    cli_auth_review as auth_review, cli_ingest as ingest, cli_ops as ops,
};

#[derive(Parser, Debug)]
#[command(name = "cortexdb", version, about = "CortexDB local CLI")]
struct Cli {
    #[arg(
        long,
        global = true,
        help = "Print machine-readable JSON when supported"
    )]
    json: bool,
    #[arg(
        long,
        global = true,
        help = "Tenant realm (subdirectory under realms/)"
    )]
    tenant: Option<String>,
    #[command(subcommand)]
    command: Command,
}

fn resolve_path(path: &str, tenant: Option<&str>) -> std::path::PathBuf {
    let base = std::path::PathBuf::from(path);
    match tenant {
        Some(t) if t != "default" => base.join("realms").join(t),
        _ => base,
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    Demo,
    Doctor {
        path: String,
    },
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    Version,
    Put {
        path: String,
        cell_id: String,
        payload: String,
    },
    Get {
        path: String,
        cell_id: String,
    },
    Tombstone {
        path: String,
        cell_id: String,
    },
    Flush {
        path: String,
    },
    Compact {
        path: String,
    },
    Stats {
        path: String,
    },
    Validate {
        path: String,
    },
    AnnValidate {
        path: String,
    },
    Repair {
        path: String,
        #[arg(long)]
        dry_run: bool,
    },
    Backup {
        path: String,
        backup_path: String,
    },
    BackupDrill {
        path: String,
        backup_path: String,
        restore_path: String,
    },
    BackupPrune {
        backup_root: String,
        prefix: String,
        keep_latest: usize,
    },
    BackupOffsiteStage {
        backup_path: String,
        offsite_root: String,
        backup_id: String,
    },
    Audit {
        path: String,
        #[arg(long)]
        route: Option<String>,
        #[arg(long)]
        status: Option<u16>,
        #[arg(long)]
        action: Option<String>,
        #[arg(long = "tenant-filter")]
        tenant_filter: Option<String>,
        #[arg(long)]
        summary: bool,
        #[arg(long = "redaction-check")]
        redaction_check: bool,
        #[arg(long = "verify-chain")]
        verify_chain: bool,
    },
    AuditExportSiem {
        input_path: String,
        output_path: String,
        #[arg(long = "redaction-check")]
        redaction_check: bool,
        #[arg(long = "verify-chain")]
        verify_chain: bool,
    },
    AuthReview {
        #[arg(long = "policy-store")]
        policy_store: Option<String>,
        #[arg(long = "tokens-file")]
        tokens_file: Option<String>,
        #[arg(long)]
        tokens: Option<String>,
    },
    Restore {
        backup_path: String,
        path: String,
    },
    GcRetired {
        path: String,
    },
    WalValidate {
        path: String,
    },
    WalDump {
        path: String,
    },
    WalTruncate {
        path: String,
    },
    ManifestDump {
        path: String,
    },
    ManifestValidate {
        path: String,
    },
    Context {
        path: String,
        scope: String,
        aql: String,
    },
    Remember {
        path: String,
        scope: String,
        aql: String,
    },
    Forget {
        path: String,
        cell_id: String,
    },
    Verify {
        path: String,
        scope: String,
        aql: String,
    },
    Aql {
        path: String,
        scope: String,
        aql: String,
    },
    Search {
        path: String,
        scope: String,
        query: String,
    },
    SearchVector {
        path: String,
        scope: String,
        vector: String,
        #[arg(long)]
        fallback: Option<String>,
        #[arg(long)]
        fallback_scan_cap: Option<usize>,
        #[arg(long)]
        min_recall: Option<String>,
        #[arg(long)]
        max_visited_candidates: Option<usize>,
        #[arg(long)]
        require_slo: bool,
        #[arg(long)]
        no_fallback_rollout: bool,
        #[arg(long)]
        no_fallback_min_recall: Option<String>,
    },
    SearchVectorExact {
        path: String,
        scope: String,
        vector: String,
    },
    SearchVectorEval {
        path: String,
        scope: String,
        vector: String,
        #[arg(long)]
        fallback: Option<String>,
        #[arg(long)]
        fallback_scan_cap: Option<usize>,
        #[arg(long)]
        min_recall: Option<String>,
        #[arg(long)]
        max_visited_candidates: Option<usize>,
        #[arg(long)]
        require_slo: bool,
        #[arg(long)]
        no_fallback_rollout: bool,
        #[arg(long)]
        no_fallback_min_recall: Option<String>,
    },
    SearchExplain {
        path: String,
        scope: String,
        query: String,
        #[arg(long, default_value = "keyword")]
        mode: String,
    },
    Unlock {
        path: String,
        #[arg(long)]
        force: bool,
    },
    LoadFixture {
        path: String,
        fixture_path: String,
    },
    IngestText {
        path: String,
        scope: String,
        file: String,
    },
    IngestJson {
        path: String,
        scope: String,
        file: String,
    },
    IngestCsv {
        path: String,
        scope: String,
        file: String,
    },
    IngestJobs {
        path: String,
    },
    IngestJob {
        path: String,
        job_id: u64,
    },
    IngestJobCancel {
        path: String,
        job_id: u64,
    },
    IngestJobRetry {
        path: String,
        job_id: u64,
    },
    IngestJobDelete {
        path: String,
        job_id: u64,
    },
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
    let resolved = |p: &str| resolve_path(p, cli.tenant.as_deref());
    match cli.command {
        Command::Demo => ops::run_demo(),
        Command::Doctor { path } => ops::doctor(resolved(&path).to_str().unwrap()),
        Command::Completions { shell } => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
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
        Command::Flush { path } => ops::flush(resolved(&path).to_str().unwrap()),
        Command::Compact { path } => ops::compact(resolved(&path).to_str().unwrap()),
        Command::Stats { path } => ops::stats(resolved(&path).to_str().unwrap(), cli.json),
        Command::Validate { path } => ops::validate(resolved(&path).to_str().unwrap(), cli.json),
        Command::AnnValidate { path } => {
            ops::ann_validate(resolved(&path).to_str().unwrap(), cli.json)
        }
        Command::Repair { path, dry_run } => {
            ops::repair(resolved(&path).to_str().unwrap(), dry_run)
        }
        Command::Backup { path, backup_path } => {
            ops::backup(resolved(&path).to_str().unwrap(), &backup_path)
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
        } => ops::backup_prune(&backup_root, &prefix, keep_latest),
        Command::BackupOffsiteStage {
            backup_path,
            offsite_root,
            backup_id,
        } => ops::backup_offsite_stage(&backup_path, &offsite_root, &backup_id),
        Command::Audit {
            path,
            route,
            status,
            action,
            tenant_filter,
            summary,
            redaction_check,
            verify_chain,
        } => audit::review(audit::AuditReviewOptions {
            path: &path,
            route: route.as_deref(),
            status,
            action: action.as_deref(),
            tenant: tenant_filter.as_deref(),
            summary_only: summary,
            redaction_check,
            verify_chain,
            json: cli.json,
        }),
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
        Command::Restore { backup_path, path } => {
            ops::restore(&backup_path, resolved(&path).to_str().unwrap())
        }
        Command::GcRetired { path } => ops::gc_retired(resolved(&path).to_str().unwrap()),
        Command::WalValidate { path } => ops::wal_validate(resolved(&path).to_str().unwrap()),
        Command::WalDump { path } => ops::wal_dump(resolved(&path).to_str().unwrap()),
        Command::WalTruncate { path } => ops::wal_truncate(resolved(&path).to_str().unwrap()),
        Command::ManifestDump { path } => ops::manifest_dump(resolved(&path).to_str().unwrap()),
        Command::ManifestValidate { path } => {
            ops::manifest_validate(resolved(&path).to_str().unwrap())
        }
        Command::Context { path, scope, aql } => {
            ops::context(resolved(&path).to_str().unwrap(), &scope, &aql, cli.json)
        }
        Command::Remember { path, scope, aql } => {
            ops::remember(resolved(&path).to_str().unwrap(), &scope, &aql, cli.json)
        }
        Command::Forget { path, cell_id } => {
            ops::forget(resolved(&path).to_str().unwrap(), &cell_id, cli.json)
        }
        Command::Verify { path, scope, aql } => {
            ops::verify(resolved(&path).to_str().unwrap(), &scope, &aql, cli.json)
        }
        Command::Aql { path, scope, aql } => {
            ops::aql(resolved(&path).to_str().unwrap(), &scope, &aql, cli.json)
        }
        Command::Search { path, scope, query } => {
            ops::search(resolved(&path).to_str().unwrap(), &scope, &query, cli.json)
        }
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
            ops::search_vector(
                resolved(&path).to_str().unwrap(),
                &scope,
                &vector,
                false,
                Some(policy),
                rollout_policy,
            )
        }
        Command::SearchVectorExact {
            path,
            scope,
            vector,
        } => ops::search_vector(
            resolved(&path).to_str().unwrap(),
            &scope,
            &vector,
            true,
            None,
            None,
        ),
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
            ann::search_vector_eval(
                resolved(&path).to_str().unwrap(),
                &scope,
                &vector,
                cli.json,
                Some(policy),
                rollout_policy,
            )
        }
        Command::SearchExplain {
            path,
            scope,
            query,
            mode,
        } => ops::search_explain(resolved(&path).to_str().unwrap(), &scope, &query, &mode),
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
