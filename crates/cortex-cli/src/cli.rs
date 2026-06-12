use clap::{error::ErrorKind, CommandFactory, Parser, Subcommand, ValueEnum};

use crate::{
    cli_ann as ann, cli_aql as aql_cmd, cli_audit as audit, cli_audit_siem as audit_siem,
    cli_auth_review as auth_review, cli_ingest as ingest, cli_ops as ops, cli_upgrade as upgrade,
};

#[derive(Parser, Debug)]
#[command(
    name = "cortexdb",
    version,
    about = "CortexDB local CLI",
    long_about = "CortexDB local CLI for durable single-node storage, AQL retrieval, ContextPack generation, deterministic verification, ingestion, repair, backup, and operational checks.",
    after_help = "Command groups:\n  Onboarding: demo, doctor, completions, version\n  Core database: put, get, tombstone, flush, compact, stats, validate\n  Agent retrieval: context, aql, verify, remember, forget, search, search-explain\n  Vector and ANN: vector, search-vector, search-vector-exact, search-vector-eval, ann-validate, hnsw-no-fallback-profile-*\n  Maintenance: repair, backup, restore, upgrade, migrate, audit, auth-review, wal-*, manifest-*, ingest-*"
)]
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
    #[command(
        about = "Run the flagship local demo",
        long_about = "Run the flagship local CortexDB demo. The demo loads a small fixture and shows scope-safe retrieval plus deterministic numeric conflict verification."
    )]
    Demo,
    #[command(about = "Check local database health and repair advice")]
    Doctor { path: String },
    #[command(about = "Generate shell completion scripts")]
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    #[command(about = "Print the cortexdb CLI version")]
    Version,
    #[command(about = "Store or replace a cell payload")]
    Put {
        path: String,
        cell_id: String,
        payload: String,
    },
    #[command(about = "Read the latest visible payload for a cell")]
    Get { path: String, cell_id: String },
    #[command(about = "Soft-delete a cell by writing a tombstone")]
    Tombstone { path: String, cell_id: String },
    #[command(about = "Checkpoint the current WAL and MemTable state")]
    Flush {
        path: String,
        #[arg(long)]
        experimental_hnsw: bool,
    },
    #[command(about = "Compact live storage segments")]
    Compact {
        path: String,
        #[arg(long)]
        experimental_hnsw: bool,
    },
    #[command(about = "Print storage and engine statistics")]
    Stats { path: String },
    #[command(about = "Validate manifest, WAL, segments, and indexes")]
    Validate { path: String },
    #[command(about = "Vector index maintenance commands")]
    Vector {
        #[command(subcommand)]
        command: VectorCommand,
    },
    #[command(about = "Validate persisted ANN/HNSW artifacts")]
    AnnValidate { path: String },
    #[command(about = "Show the no-fallback HNSW rollout profile")]
    HnswNoFallbackProfileShow { path: String },
    #[command(about = "Set the no-fallback HNSW rollout profile")]
    HnswNoFallbackProfileSet {
        path: String,
        #[arg(long, default_value = "true")]
        enabled: String,
        #[arg(long)]
        min_recall: Option<String>,
        #[arg(long, default_value = "true")]
        require_upper_layers: String,
    },
    #[command(about = "Clear the no-fallback HNSW rollout profile")]
    HnswNoFallbackProfileClear { path: String },
    #[command(about = "Repair safe storage issues")]
    Repair {
        path: String,
        #[arg(long)]
        dry_run: bool,
    },
    #[command(about = "Create a validated offline-copy backup")]
    Backup { path: String, backup_path: String },
    #[command(about = "Create a passphrase-protected backup archive")]
    BackupEncrypted {
        path: String,
        archive_path: String,
        #[arg(long)]
        passphrase: Option<String>,
        #[arg(long = "passphrase-env")]
        passphrase_env: Option<String>,
    },
    #[command(about = "Create, restore, and validate a backup drill")]
    BackupDrill {
        path: String,
        backup_path: String,
        restore_path: String,
    },
    #[command(about = "Prune old backup directories by prefix")]
    BackupPrune {
        backup_root: String,
        prefix: String,
        keep_latest: usize,
        #[arg(long)]
        dry_run: bool,
    },
    #[command(about = "Validate and publish a backup into an offsite staging root")]
    BackupOffsiteStage {
        backup_path: String,
        offsite_root: String,
        backup_id: String,
    },
    #[command(about = "Run upgrade preflight, validation, and rollback commands")]
    Upgrade {
        #[command(subcommand)]
        command: UpgradeCommand,
    },
    #[command(about = "Run a safe offline migration rewrite")]
    Migrate {
        path: String,
        backup_path: String,
        drill_restore_path: String,
    },
    #[command(about = "Review and verify audit JSONL logs")]
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
    #[command(about = "Export normalized SIEM audit JSONL")]
    AuditExportSiem {
        input_path: String,
        output_path: String,
        #[arg(long = "redaction-check")]
        redaction_check: bool,
        #[arg(long = "verify-chain")]
        verify_chain: bool,
    },
    #[command(about = "Review auth policy and token configuration")]
    AuthReview {
        #[arg(long = "policy-store")]
        policy_store: Option<String>,
        #[arg(long = "tokens-file")]
        tokens_file: Option<String>,
        #[arg(long)]
        tokens: Option<String>,
    },
    #[command(about = "Restore a backup into a new database path")]
    Restore {
        backup_path: String,
        path: String,
        #[arg(long)]
        dry_run: bool,
    },
    #[command(about = "Restore an encrypted backup archive")]
    RestoreEncrypted {
        archive_path: String,
        path: String,
        #[arg(long)]
        passphrase: Option<String>,
        #[arg(long = "passphrase-env")]
        passphrase_env: Option<String>,
    },
    #[command(about = "Remove retired segment bundles after compaction")]
    GcRetired { path: String },
    #[command(about = "Validate WAL records")]
    WalValidate { path: String },
    #[command(about = "Dump WAL records for inspection")]
    WalDump { path: String },
    #[command(about = "Truncate WAL to the best-effort safe offset")]
    WalTruncate { path: String },
    #[command(about = "Dump the storage manifest")]
    ManifestDump { path: String },
    #[command(about = "Validate the storage manifest")]
    ManifestValidate { path: String },
    #[command(
        about = "Build a ContextPack from an AQL RETRIEVE query",
        long_about = "Build a token-budgeted, citation-aware ContextPack from an AQL RETRIEVE query.\n\nExample:\n  cortexdb context ./db project:investments 'RETRIEVE CONTEXT FOR TASK \"Solar Plant budget\" IN BRAIN default LIMIT 10 CANDIDATES;' --format json"
    )]
    Context {
        path: String,
        scope: String,
        aql: String,
        #[arg(long, value_enum, default_value_t = ContextOutputFormat::Summary)]
        format: ContextOutputFormat,
    },
    #[command(about = "Persist an AQL REMEMBER memory cell")]
    Remember {
        path: String,
        scope: String,
        aql: String,
    },
    #[command(about = "Forget a cell by writing a tombstone")]
    Forget { path: String, cell_id: String },
    #[command(
        about = "Verify a fact against scoped database evidence",
        long_about = "Run deterministic VERIFY FACT over scoped evidence and report supported, contradicted, mixed, or insufficient evidence.\n\nExample:\n  cortexdb verify ./db project:investments 'VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN default;' --format json"
    )]
    Verify {
        path: String,
        scope: String,
        aql: String,
        #[arg(long, value_enum, default_value_t = VerificationOutputFormat::Summary)]
        format: VerificationOutputFormat,
    },
    #[command(
        about = "Execute an AQL statement",
        long_about = "Execute an AQL statement against a scoped AgentView. Use this for raw RETRIEVE, VERIFY, REMEMBER, and EXPLAIN flows.\n\nExample:\n  cortexdb aql ./db project:investments 'EXPLAIN RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN default LIMIT 5 CANDIDATES;'"
    )]
    Aql {
        path: String,
        scope: String,
        aql: String,
    },
    #[command(about = "Run keyword, vector, or hybrid search")]
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
    #[command(about = "Run vector search with ANN policy controls")]
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
    #[command(about = "Run exact vector scan")]
    SearchVectorExact {
        path: String,
        scope: String,
        vector: String,
    },
    #[command(about = "Evaluate vector search against an exact baseline")]
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
    #[command(about = "Explain search ranking and contribution details")]
    SearchExplain {
        path: String,
        scope: String,
        query: String,
        #[arg(long, default_value = "keyword")]
        mode: String,
        #[arg(long)]
        vector: Option<String>,
    },
    #[command(about = "Unlock a stale local database lock")]
    Unlock {
        path: String,
        #[arg(long)]
        force: bool,
    },
    #[command(about = "Load a built-in dataset fixture")]
    LoadFixture { path: String, fixture_path: String },
    #[command(about = "Ingest a text or Markdown file")]
    IngestText {
        path: String,
        scope: String,
        file: String,
    },
    #[command(about = "Ingest facts from a JSON file")]
    IngestJson {
        path: String,
        scope: String,
        file: String,
    },
    #[command(about = "Ingest rows from a CSV file")]
    IngestCsv {
        path: String,
        scope: String,
        file: String,
    },
    #[command(about = "List ingestion jobs")]
    IngestJobs { path: String },
    #[command(about = "Show one ingestion job")]
    IngestJob { path: String, job_id: u64 },
    #[command(about = "Cancel an ingestion job")]
    IngestJobCancel { path: String, job_id: u64 },
    #[command(about = "Retry a failed ingestion job")]
    IngestJobRetry { path: String, job_id: u64 },
    #[command(about = "Delete an ingestion job record")]
    IngestJobDelete { path: String, job_id: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum ContextOutputFormat {
    Summary,
    Json,
    Prompt,
    Markdown,
}

#[derive(Subcommand, Debug)]
enum VectorCommand {
    #[command(about = "Rebuild persisted vector and HNSW artifacts")]
    Rebuild {
        path: String,
        #[arg(long)]
        experimental_hnsw: bool,
    },
}

#[derive(Subcommand, Debug)]
enum UpgradeCommand {
    #[command(about = "Create and validate a pre-upgrade backup drill")]
    Prepare {
        path: String,
        backup_path: String,
        drill_restore_path: String,
    },
    #[command(about = "Validate a database after installing a new binary")]
    Validate { path: String },
    #[command(about = "Restore a pre-upgrade backup into a rollback path")]
    Rollback {
        backup_path: String,
        rollback_path: String,
    },
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

#[cfg(test)]
mod help_contract_tests {
    use super::Cli;
    use clap::CommandFactory;

    fn assert_subcommands_have_about(command: &clap::Command) {
        for subcommand in command.get_subcommands() {
            assert!(
                subcommand.get_about().is_some(),
                "missing about for command {}",
                subcommand.get_name()
            );
            assert_subcommands_have_about(subcommand);
        }
    }

    #[test]
    fn every_cli_command_has_help_text() {
        let command = Cli::command();
        assert_subcommands_have_about(&command);
    }
}
