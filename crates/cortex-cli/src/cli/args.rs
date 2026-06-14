use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "cortexdb",
    version,
    about = "CortexDB local CLI",
    long_about = "CortexDB local CLI for durable single-node storage, AQL retrieval, ContextPack generation, deterministic verification, ingestion, repair, backup, and operational checks.",
    after_help = "Command groups:\n  Onboarding: init, demo, doctor, completions, version\n  Core database: put, get, tombstone, flush, compact, stats, validate\n  Agent retrieval: context, explain, aql, verify, remember, forget, search, search-explain\n  Agent admin: agent create|list|show|grant|revoke\n  Vector and ANN: vector, search-vector, search-vector-exact, search-vector-eval, ann-validate, hnsw-no-fallback-profile-*\n  Maintenance: repair, backup, restore, upgrade, migrate, audit, auth-review, wal-*, manifest-*, ingest-*"
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

mod commands;

pub(super) use commands::{
    AgentCommand, AgentScopeAccessArg, Command, ContextOutputFormat, UpgradeCommand, VectorCommand,
    VerificationOutputFormat,
};
