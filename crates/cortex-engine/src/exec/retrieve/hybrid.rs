use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::time::Instant;

use cortex_aql::BoundRetrievePlan;

use super::{bitmap_first_candidates, CandidateSource};
use crate::database::{CandidateResolver, Database};
use crate::error::EngineResult;
use crate::exec::pack::ExplainCollector;
use crate::exec::scans::{LexicalScan, PermissionFilter, VectorScan};
use crate::exec::trace::{drain, elapsed_nanos, MaterializedOp, PhysicalOp};
use crate::retrieval_rank::{query_vector_from_task, semantic_dot_score};
use crate::search::analyze_search_query;

pub(super) fn hybrid_candidates<P: CandidateResolver>(
    database: &Database,
    plan: &BoundRetrievePlan,
    provider: &P,
    collector: &mut ExplainCollector,
) -> EngineResult<CandidateSource> {
    let bitmap_candidates = bitmap_first_candidates(plan, provider, collector)?;

    let mut permission_filter = PermissionFilter::new(provider, bitmap_candidates);
    let permission_candidates = drain(&mut permission_filter);
    collector.push(permission_filter.trace());

    let mut lexical_scan = LexicalScan::passthrough(ranked_lexical_candidates(
        plan,
        provider,
        &permission_candidates,
    ));
    let lexical_candidates = drain(&mut lexical_scan);
    let lexical_len = lexical_candidates.len();
    collector.push(lexical_scan.trace());

    let mut vector_scan = VectorScan::passthrough(ranked_vector_candidates(
        database,
        plan,
        provider,
        &permission_candidates,
    ));
    let vector_candidates = drain(&mut vector_scan);
    let vector_len = vector_candidates.len();
    collector.push(vector_scan.trace());

    let started = Instant::now();
    let fused = if lexical_candidates.is_empty() && vector_candidates.is_empty() {
        permission_candidates
    } else {
        fuse_hybrid_rrf(lexical_candidates, vector_candidates)
    };
    let mut fusion_op = MaterializedOp::new(
        "HybridRrfOp",
        lexical_len + vector_len,
        fused,
        elapsed_nanos(started),
    );
    let candidates = drain(&mut fusion_op);
    collector.push(fusion_op.trace());

    Ok(CandidateSource {
        candidates,
        permission_applied: true,
    })
}

fn ranked_lexical_candidates<P: CandidateResolver>(
    plan: &BoundRetrievePlan,
    provider: &P,
    candidates: &[u32],
) -> Vec<u32> {
    if let Some(ranked) = provider.ranked_candidates_for_task(&plan.task, candidates) {
        return ranked;
    }

    let terms = analyze_search_query(&plan.task)
        .weighted_terms
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return Vec::new();
    }
    let Some(matches) = provider.lexical_candidates_for_terms(&terms) else {
        return Vec::new();
    };
    candidates
        .iter()
        .copied()
        .filter(|candidate| matches.contains(candidate))
        .collect()
}

fn ranked_vector_candidates<P: CandidateResolver>(
    database: &Database,
    plan: &BoundRetrievePlan,
    provider: &P,
    candidates: &[u32],
) -> Vec<u32> {
    let Some(query_vector) = query_vector_from_task(&plan.task) else {
        return Vec::new();
    };

    let pin = database.pin_read_txn();
    let txn = pin.read_txn();
    let mut scored = candidates
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, candidate)| {
            let score = provider
                .cell_id_for_candidate(candidate)
                .and_then(|cell_id| database.memtable.read(txn, cell_id))
                .and_then(|version| database.retrieved_cell_from_version(version).ok())
                .map(|cell| semantic_dot_score(&cell.payload, &query_vector))
                .unwrap_or(0);
            (score > 0).then_some((score, index, candidate))
        })
        .collect::<Vec<_>>();
    scored.sort_by_key(|(score, index, _)| (Reverse(*score), *index));
    scored
        .into_iter()
        .map(|(_, _, candidate)| candidate)
        .collect()
}

fn fuse_hybrid_rrf(lexical: Vec<u32>, vector: Vec<u32>) -> Vec<u32> {
    let mut results = BTreeMap::<u32, (u64, usize)>::new();
    apply_rrf_stream(&mut results, lexical, 32_768, 0);
    apply_rrf_stream(&mut results, vector, 32_767, usize::MAX / 2);
    let mut ranked = results
        .into_iter()
        .map(|(candidate, (score, first_seen))| (score, first_seen, candidate))
        .collect::<Vec<_>>();
    ranked.sort_by_key(|(score, first_seen, candidate)| (Reverse(*score), *first_seen, *candidate));
    ranked
        .into_iter()
        .map(|(_, _, candidate)| candidate)
        .collect()
}

fn apply_rrf_stream(
    results: &mut BTreeMap<u32, (u64, usize)>,
    ranked: Vec<u32>,
    weight_q16: u64,
    stream_offset: usize,
) {
    for (rank, candidate) in ranked.into_iter().enumerate() {
        let score = (1_000_000 / (60 + rank as u64 + 1)).saturating_mul(weight_q16) / 65_535;
        let first_seen = stream_offset.saturating_add(rank);
        let entry = results.entry(candidate).or_insert((0, first_seen));
        entry.0 = entry.0.saturating_add(score);
        entry.1 = entry.1.min(first_seen);
    }
}
