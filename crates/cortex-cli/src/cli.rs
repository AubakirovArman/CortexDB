use clap::{error::ErrorKind, Parser, Subcommand, ValueEnum};

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
        #[arg(long)]
        experimental_hnsw: bool,
    },
    Compact {
        path: String,
        #[arg(long)]
        experimental_hnsw: bool,
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
    HnswNoFallbackProfileShow {
        path: String,
    },
    HnswNoFallbackProfileSet {
        path: String,
        #[arg(long, default_value = "true")]
        enabled: String,
        #[arg(long)]
        min_recall: Option<String>,
        #[arg(long, default_value = "true")]
        require_upper_layers: String,
    },
    HnswNoFallbackProfileClear {
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
    BackupEncrypted {
        path: String,
        archive_path: String,
        #[arg(long)]
        passphrase: Option<String>,
        #[arg(long = "passphrase-env")]
        passphrase_env: Option<String>,
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
        #[arg(long)]
        dry_run: bool,
    },
    BackupOffsiteStage {
        backup_path: String,
        offsite_root: String,
        backup_id: String,
    },
    Audit {
        path: String,
        verify_path: Option<String>,
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
        #[arg(long)]
        dry_run: bool,
    },
    RestoreEncrypted {
        archive_path: String,
        path: String,
        #[arg(long)]
        passphrase: Option<String>,
        #[arg(long = "passphrase-env")]
        passphrase_env: Option<String>,
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
        #[arg(long, value_enum, default_value_t = ContextOutputFormat::Summary)]
        format: ContextOutputFormat,
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
        #[arg(long, value_enum, default_value_t = VerificationOutputFormat::Summary)]
        format: VerificationOutputFormat,
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
        #[arg(long, default_value = "keyword")]
        mode: String,
        #[arg(long)]
        vector: Option<String>,
        #[arg(long, default_value = "ann")]
        algorithm: String,
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
        #[arg(long)]
        use_no_fallback_profile: bool,
        #[arg(long)]
        experimental_hnsw: bool,
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
        #[arg(long)]
        use_no_fallback_profile: bool,
        #[arg(long)]
        experimental_hnsw: bool,
    },
    SearchExplain {
        path: String,
        scope: String,
        query: String,
        #[arg(long, default_value = "keyword")]
        mode: String,
        #[arg(long)]
        vector: Option<String>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum ContextOutputFormat {
    Summary,
    Json,
    Prompt,
    Markdown,
}

impl ContextOutputFormat {
    fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Json => "json",
            Self::Prompt => "prompt",
            Self::Markdown => "markdown",
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum VerificationOutputFormat {
    Summary,
    Json,
    Markdown,
    Audit,
}

impl VerificationOutputFormat {
    fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Audit => "audit",
        }
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
            ops::aql(resolved(&path).to_str().unwrap(), &scope, &aql, cli.json)
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
