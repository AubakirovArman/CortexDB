use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "cortexdb",
    version,
    about = "CortexDB local CLI",
    long_about = "CortexDB local CLI for durable single-node storage, AQL retrieval, ContextPack generation, deterministic verification, ingestion, repair, backup, and operational checks.",
    after_help = "Command groups:\n  Onboarding: demo, doctor, completions, version\n  Core database: put, get, tombstone, flush, compact, stats, validate\n  Agent retrieval: context, aql, verify, remember, forget, search, search-explain\n  Vector and ANN: vector, search-vector, search-vector-exact, search-vector-eval, ann-validate, hnsw-no-fallback-profile-*\n  Maintenance: repair, backup, restore, upgrade, migrate, audit, auth-review, wal-*, manifest-*, ingest-*"
)]
pub(super) struct Cli {
    #[arg(
        long,
        global = true,
        help = "Print machine-readable JSON when supported"
    )]
    pub(super) json: bool,
    #[arg(
        long,
        global = true,
        help = "Tenant realm (subdirectory under realms/)"
    )]
    pub(super) tenant: Option<String>,
    #[command(subcommand)]
    pub(super) command: Command,
}

#[derive(Subcommand, Debug)]
pub(super) enum Command {
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
pub(super) enum ContextOutputFormat {
    Summary,
    Json,
    Prompt,
    Markdown,
}

#[derive(Subcommand, Debug)]
pub(super) enum VectorCommand {
    #[command(about = "Rebuild persisted vector and HNSW artifacts")]
    Rebuild {
        path: String,
        #[arg(long)]
        experimental_hnsw: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(super) enum UpgradeCommand {
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
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Json => "json",
            Self::Prompt => "prompt",
            Self::Markdown => "markdown",
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub(super) enum VerificationOutputFormat {
    Summary,
    Json,
    Markdown,
    Audit,
}

impl VerificationOutputFormat {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Audit => "audit",
        }
    }
}
