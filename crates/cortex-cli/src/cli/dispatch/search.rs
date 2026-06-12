use crate::{cli_ann as ann, cli_ops as ops};

use super::DispatchContext;

pub(super) struct VectorSearchInput {
    pub(super) path: String,
    pub(super) scope: String,
    pub(super) vector: String,
    pub(super) fallback: Option<String>,
    pub(super) fallback_scan_cap: Option<usize>,
    pub(super) min_recall: Option<String>,
    pub(super) max_visited_candidates: Option<usize>,
    pub(super) require_slo: bool,
    pub(super) no_fallback_rollout: bool,
    pub(super) no_fallback_min_recall: Option<String>,
    pub(super) use_no_fallback_profile: bool,
    pub(super) experimental_hnsw: bool,
}

pub(super) fn search(
    ctx: DispatchContext<'_>,
    path: String,
    scope: String,
    query: String,
    mode: String,
    vector: Option<String>,
    algorithm: String,
) -> Result<String, String> {
    ops::search(
        ctx.resolve(&path).to_str().unwrap(),
        &scope,
        &query,
        ctx.json,
        &mode,
        vector.as_deref(),
        &algorithm,
    )
}

pub(super) fn search_vector(
    ctx: DispatchContext<'_>,
    input: VectorSearchInput,
) -> Result<String, String> {
    let policy = ann::parse_ann_policy(
        input.fallback,
        input.fallback_scan_cap,
        input.min_recall,
        input.max_visited_candidates,
        input.require_slo,
    )?;
    let rollout_policy = ann::parse_no_fallback_rollout_policy(
        input.no_fallback_rollout,
        input.no_fallback_min_recall,
    )?;
    ops::search_vector(ops::SearchVectorOptions {
        path: ctx.resolve(&input.path).to_str().unwrap(),
        scope: &input.scope,
        vector: &input.vector,
        exact: false,
        policy: Some(policy),
        rollout_policy,
        use_no_fallback_profile: input.use_no_fallback_profile,
        experimental_hnsw: input.experimental_hnsw,
    })
}

pub(super) fn search_vector_exact(
    ctx: DispatchContext<'_>,
    path: String,
    scope: String,
    vector: String,
) -> Result<String, String> {
    ops::search_vector(ops::SearchVectorOptions {
        path: ctx.resolve(&path).to_str().unwrap(),
        scope: &scope,
        vector: &vector,
        exact: true,
        policy: None,
        rollout_policy: None,
        use_no_fallback_profile: false,
        experimental_hnsw: false,
    })
}

pub(super) fn search_vector_eval(
    ctx: DispatchContext<'_>,
    input: VectorSearchInput,
) -> Result<String, String> {
    let policy = ann::parse_ann_policy(
        input.fallback,
        input.fallback_scan_cap,
        input.min_recall,
        input.max_visited_candidates,
        input.require_slo,
    )?;
    let rollout_policy = ann::parse_no_fallback_rollout_policy(
        input.no_fallback_rollout,
        input.no_fallback_min_recall,
    )?;
    ann::search_vector_eval(ann::SearchVectorEvalOptions {
        path: ctx.resolve(&input.path).to_str().unwrap(),
        scope: &input.scope,
        vector: &input.vector,
        json: ctx.json,
        policy: Some(policy),
        rollout_policy,
        use_no_fallback_profile: input.use_no_fallback_profile,
        experimental_hnsw: input.experimental_hnsw,
    })
}

pub(super) fn search_explain(
    ctx: DispatchContext<'_>,
    path: String,
    scope: String,
    query: String,
    mode: String,
    vector: Option<String>,
) -> Result<String, String> {
    ops::search_explain(
        ctx.resolve(&path).to_str().unwrap(),
        &scope,
        &query,
        &mode,
        vector.as_deref(),
    )
}
