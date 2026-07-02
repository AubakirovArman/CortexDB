use clap::{Args, Subcommand, ValueEnum};

#[derive(Subcommand, Debug)]
pub(in crate::cli) enum AgentCommand {
    #[command(about = "Create or replace a persisted AgentView")]
    Create(AgentCreateArgs),
    #[command(about = "List persisted AgentViews")]
    List { path: String },
    #[command(about = "Show one persisted AgentView")]
    Show { path: String, agent_id: u64 },
    #[command(about = "Grant an AgentView scope permission")]
    Grant(AgentScopeArgs),
    #[command(about = "Revoke an AgentView scope permission")]
    Revoke(AgentScopeArgs),
}

#[derive(Args, Debug)]
pub(in crate::cli) struct AgentCreateArgs {
    pub(in crate::cli) path: String,
    pub(in crate::cli) agent_id: u64,
    #[arg(long)]
    pub(in crate::cli) label: Option<String>,
    #[arg(long = "read-scope")]
    pub(in crate::cli) readable_scopes: Vec<String>,
    #[arg(long = "write-scope")]
    pub(in crate::cli) writable_scopes: Vec<String>,
    #[arg(long = "brain")]
    pub(in crate::cli) readable_brains: Vec<u64>,
    #[arg(long = "mode")]
    pub(in crate::cli) allowed_modes: Vec<String>,
    #[arg(long = "memory-type")]
    pub(in crate::cli) allowed_memory_types: Vec<String>,
    #[arg(long, default_value_t = 4_000)]
    pub(in crate::cli) max_context_budget_tokens: u32,
    #[arg(long, default_value_t = 1_000)]
    pub(in crate::cli) default_context_budget_tokens: u32,
    #[arg(long, default_value_t = 100)]
    pub(in crate::cli) max_candidate_limit: u32,
    #[arg(long, default_value_t = 20)]
    pub(in crate::cli) default_candidate_limit: u32,
    #[arg(long, default_value_t = 0)]
    pub(in crate::cli) min_required_confidence_q16: u16,
    #[arg(long)]
    pub(in crate::cli) max_ttl_seconds: Option<u64>,
    #[arg(long)]
    pub(in crate::cli) private_scope: Option<String>,
    #[arg(long)]
    pub(in crate::cli) deny_remember: bool,
    #[arg(long)]
    pub(in crate::cli) deny_verify_fact: bool,
    #[arg(long)]
    pub(in crate::cli) allow_audit_mode: bool,
    #[arg(long)]
    pub(in crate::cli) require_citations: bool,
}

#[derive(Args, Debug)]
pub(in crate::cli) struct AgentScopeArgs {
    pub(in crate::cli) path: String,
    pub(in crate::cli) agent_id: u64,
    pub(in crate::cli) scope: String,
    #[arg(long, value_enum, default_value_t = AgentScopeAccessArg::Read)]
    pub(in crate::cli) access: AgentScopeAccessArg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(in crate::cli) enum AgentScopeAccessArg {
    Read,
    Write,
    #[value(name = "read_write", alias = "read-write", alias = "both")]
    ReadWrite,
}

#[derive(Subcommand, Debug)]
pub(in crate::cli) enum VectorCommand {
    #[command(about = "Rebuild persisted vector and HNSW artifacts")]
    Rebuild {
        path: String,
        #[arg(long)]
        experimental_hnsw: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(in crate::cli) enum ReceiptKeyCommand {
    #[command(about = "Generate an Ed25519 receipt signing key file")]
    Generate(ReceiptKeyGenerateArgs),
    #[command(about = "Export a receipt signing key public key file")]
    ExportPublic {
        key_file: String,
        public_key_file: String,
    },
    #[command(about = "Rotate receipt signing keys and write a dual-trust manifest")]
    Rotate(ReceiptKeyRotateArgs),
    #[command(about = "Verify a receipt/audit key rotation re-anchor record")]
    VerifyReanchor(ReceiptKeyVerifyReanchorArgs),
}

#[derive(Args, Debug)]
pub(in crate::cli) struct ReceiptKeyGenerateArgs {
    pub(in crate::cli) key_file: String,
    #[arg(long = "key-id")]
    pub(in crate::cli) key_id: String,
    #[arg(long = "public-key-file")]
    pub(in crate::cli) public_key_file: Option<String>,
}

#[derive(Args, Debug)]
pub(in crate::cli) struct ReceiptKeyRotateArgs {
    pub(in crate::cli) current_key_file: String,
    pub(in crate::cli) next_key_file: String,
    pub(in crate::cli) trust_file: String,
    #[arg(long = "new-key-id")]
    pub(in crate::cli) new_key_id: String,
    #[arg(long = "reanchor-file")]
    pub(in crate::cli) reanchor_file: Option<String>,
    #[arg(long = "audit-chain-head")]
    pub(in crate::cli) audit_chain_head: Option<String>,
    #[arg(long = "audit-sequence", default_value_t = 0)]
    pub(in crate::cli) audit_sequence: u64,
}

#[derive(Args, Debug)]
pub(in crate::cli) struct ReceiptKeyVerifyReanchorArgs {
    pub(in crate::cli) reanchor_file: String,
    #[arg(long = "trust-file")]
    pub(in crate::cli) trust_file: Option<String>,
}

#[derive(Subcommand, Debug)]
pub(in crate::cli) enum UpgradeCommand {
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
