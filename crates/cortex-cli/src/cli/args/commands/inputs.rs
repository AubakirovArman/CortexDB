use clap::Args;

#[derive(Clone, Debug, Args)]
pub(in crate::cli) struct ScopedAqlArgs {
    pub(in crate::cli) path: String,
    pub(in crate::cli) scope: String,
    pub(in crate::cli) aql: String,
}

#[derive(Clone, Debug, Args)]
pub(in crate::cli) struct ScopedVectorArgs {
    pub(in crate::cli) path: String,
    pub(in crate::cli) scope: String,
    pub(in crate::cli) vector: String,
}

#[derive(Clone, Debug, Args)]
pub(in crate::cli) struct VectorSearchPolicyArgs {
    #[arg(long)]
    pub(in crate::cli) fallback: Option<String>,
    #[arg(long)]
    pub(in crate::cli) fallback_scan_cap: Option<usize>,
    #[arg(long)]
    pub(in crate::cli) min_recall: Option<String>,
    #[arg(long)]
    pub(in crate::cli) max_visited_candidates: Option<usize>,
    #[arg(long)]
    pub(in crate::cli) require_slo: bool,
    #[arg(long)]
    pub(in crate::cli) no_fallback_rollout: bool,
    #[arg(long)]
    pub(in crate::cli) no_fallback_min_recall: Option<String>,
    #[arg(long)]
    pub(in crate::cli) use_no_fallback_profile: bool,
    #[arg(long)]
    pub(in crate::cli) experimental_hnsw: bool,
}
