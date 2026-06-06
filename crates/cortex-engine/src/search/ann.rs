use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;

use super::hnsw::{DistanceMetric, HnswIndex};
use super::persisted::search_persisted_vectors;
use super::ScoredCandidate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnSearchPath {
    HnswGraph,
    ExactFallback,
}

impl AnnSearchPath {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HnswGraph => "hnsw_graph",
            Self::ExactFallback => "exact_fallback",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnFallbackReason {
    EmptyGraph,
    InvalidGraph,
    StaleGraph,
    InsufficientResults,
    LowRecall,
    VisitBudgetExceeded,
    NoPersistedSegments,
    UncheckpointedChanges,
    HnswDisabled,
}

impl AnnFallbackReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyGraph => "empty_graph",
            Self::InvalidGraph => "invalid_graph",
            Self::StaleGraph => "stale_graph",
            Self::InsufficientResults => "insufficient_results",
            Self::LowRecall => "low_recall",
            Self::VisitBudgetExceeded => "visit_budget_exceeded",
            Self::NoPersistedSegments => "no_persisted_segments",
            Self::UncheckpointedChanges => "uncheckpointed_changes",
            Self::HnswDisabled => "hnsw_disabled",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnSloViolation {
    EmptyGraph,
    InvalidGraph,
    StaleGraph,
    InsufficientResults,
    LowRecall,
    VisitBudgetExceeded,
    NoPersistedSegments,
    UncheckpointedChanges,
    HnswDisabled,
    RecallBelowMinimum,
    WeakMultiLayerTopology,
}

impl AnnSloViolation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyGraph => "empty_graph",
            Self::InvalidGraph => "invalid_graph",
            Self::StaleGraph => "stale_graph",
            Self::InsufficientResults => "insufficient_results",
            Self::LowRecall => "low_recall",
            Self::VisitBudgetExceeded => "visit_budget_exceeded",
            Self::NoPersistedSegments => "no_persisted_segments",
            Self::UncheckpointedChanges => "uncheckpointed_changes",
            Self::HnswDisabled => "hnsw_disabled",
            Self::RecallBelowMinimum => "recall_below_minimum",
            Self::WeakMultiLayerTopology => "weak_multi_layer_topology",
        }
    }
}

/// Minimum acceptable ANN recall threshold.
/// Q16_ONE = 65_535 represents 100% recall.
/// Production default is 75% (49_151) to allow approximate results
/// while preserving correctness through exact fallback when recall drops.
pub const MIN_ANN_RECALL_Q16: u16 = 49_151;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnSearchPolicy {
    pub min_recall_q16: Option<u16>,
    pub fallback: bool,
    pub fallback_scan_cap: Option<usize>,
    pub max_visited_candidates: Option<usize>,
    pub require_slo: bool,
}

impl Default for AnnSearchPolicy {
    fn default() -> Self {
        Self {
            min_recall_q16: Some(MIN_ANN_RECALL_Q16),
            fallback: true,
            fallback_scan_cap: None,
            max_visited_candidates: None,
            require_slo: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnSearchReport {
    pub path: AnnSearchPath,
    pub fallback_reason: Option<AnnFallbackReason>,
    pub fallback_performed: bool,
    pub requested_limit: usize,
    pub allowed_candidates: usize,
    pub graph_nodes: usize,
    pub returned_candidates: usize,
    pub visited_candidates: usize,
    pub max_visited_candidates: Option<usize>,
    pub recall_q16: Option<u16>,
    pub min_recall_q16: Option<u16>,
    pub hnsw_max_neighbors: usize,
    pub hnsw_ef_search: usize,
    pub hnsw_layer_count: usize,
    pub hnsw_ef_construction: usize,
    pub upper_graph_edges: usize,
    pub require_slo: bool,
    pub production_safe: bool,
    pub slo_violations: Vec<AnnSloViolation>,
}

#[derive(Clone, Copy, Debug)]
struct HnswRuntimeConfig {
    max_neighbors: usize,
    ef_search: usize,
    layer_count: usize,
    upper_graph_edges: usize,
    metric: DistanceMetric,
    ef_construction: usize,
}

const ANN_DEFAULT_MAX_NEIGHBORS: usize = 8;
const ANN_DEFAULT_EF_SEARCH: usize = 64;
const ANN_MIN_NODES_FOR_SLO_MULTI_LAYER: usize = 4;

fn hnsw_runtime_config(graph: &HnswGraphIndex) -> HnswRuntimeConfig {
    let max_neighbors = if graph.max_neighbors == 0 {
        ANN_DEFAULT_MAX_NEIGHBORS
    } else {
        graph.max_neighbors as usize
    };
    let ef_search = if graph.ef_search == 0 {
        ANN_DEFAULT_EF_SEARCH
    } else {
        graph.ef_search as usize
    };
    let layer_count = if graph.layer_count == 0 {
        graph
            .upper_layers
            .keys()
            .next_back()
            .and_then(|layer| layer.checked_add(1))
            .map(|layer| layer as usize)
            .unwrap_or(1)
    } else {
        graph.layer_count as usize
    };
    let upper_graph_edges = graph
        .upper_layers
        .values()
        .flat_map(|links| links.values())
        .map(|neighbors| neighbors.len())
        .sum();
    HnswRuntimeConfig {
        max_neighbors,
        ef_search,
        layer_count,
        upper_graph_edges,
        metric: metric_from_graph(graph),
        ef_construction: if graph.ef_construction == 0 {
            ef_search
        } else {
            graph.ef_construction as usize
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnSearchOutcome {
    pub results: Vec<ScoredCandidate>,
    pub report: AnnSearchReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnEvaluationReport {
    pub search: AnnSearchReport,
    pub exact_top_k: Vec<u32>,
    pub ann_top_k: Vec<u32>,
    pub overlap_count: usize,
    pub recall_q16: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AnnMetrics {
    pub graph_nodes: usize,
    pub total_edges: usize,
    pub persisted_segments: usize,
    pub has_checkpoint: bool,
    pub has_uncheckpointed_changes: bool,
    pub deleted_vectors: usize,
    pub rebuild_count: u64,
}

pub fn search_persisted_ann(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
) -> AnnSearchOutcome {
    search_persisted_ann_with_policy(
        vectors,
        graph,
        query,
        allowed,
        limit,
        AnnSearchPolicy::default(),
    )
}

pub fn search_persisted_ann_with_policy(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    policy: AnnSearchPolicy,
) -> AnnSearchOutcome {
    let available = vectors
        .iter()
        .filter(|(id, vector)| allowed.contains(id) && vector.len() == query.len())
        .count();
    let expected = limit.min(available);
    let graph_nodes = graph.links.len();
    let config = hnsw_runtime_config(graph);

    if graph.links.is_empty() {
        if !policy.fallback {
            return fallback_disabled_outcome(
                AnnFallbackReason::EmptyGraph,
                vectors,
                query,
                allowed,
                limit,
                available,
                graph_nodes,
                config,
                policy,
                0,
            );
        }
        return exact(
            vectors,
            query,
            allowed,
            limit,
            available,
            graph_nodes,
            config,
            AnnFallbackReason::EmptyGraph,
            policy,
        );
    }

    let (ann, visited_candidates, budget_exceeded) = match search_hnsw(
        vectors,
        graph,
        query,
        allowed,
        limit,
        config,
        policy.max_visited_candidates,
    ) {
        Ok(value) => value,
        Err(reason) => {
            if !policy.fallback {
                return fallback_disabled_outcome(
                    reason,
                    vectors,
                    query,
                    allowed,
                    limit,
                    available,
                    graph_nodes,
                    config,
                    policy,
                    0,
                );
            }
            return exact(
                vectors,
                query,
                allowed,
                limit,
                available,
                graph_nodes,
                config,
                reason,
                policy,
            );
        }
    };

    if budget_exceeded {
        if !policy.fallback {
            return fallback_disabled_outcome(
                AnnFallbackReason::VisitBudgetExceeded,
                vectors,
                query,
                allowed,
                limit,
                available,
                graph_nodes,
                config,
                policy,
                visited_candidates,
            );
        }

        return exact(
            vectors,
            query,
            allowed,
            limit,
            available,
            graph_nodes,
            config,
            AnnFallbackReason::VisitBudgetExceeded,
            policy,
        );
    }

    if ann.len() < expected {
        if !policy.fallback {
            return fallback_disabled_outcome(
                AnnFallbackReason::InsufficientResults,
                vectors,
                query,
                allowed,
                limit,
                available,
                graph_nodes,
                config,
                policy,
                visited_candidates,
            );
        }
        return exact(
            vectors,
            query,
            allowed,
            limit,
            available,
            graph_nodes,
            config,
            AnnFallbackReason::InsufficientResults,
            policy,
        );
    }

    let exact_results = search_persisted_vectors(vectors, query, allowed, limit, &config.metric);
    let exact_set = exact_results
        .iter()
        .map(|candidate| candidate.cell_id)
        .collect::<BTreeSet<_>>();
    let overlap = ann
        .iter()
        .filter(|candidate| exact_set.contains(&candidate.cell_id))
        .count();
    let recall = recall_q16(overlap, exact_results.len());
    let effective_min_recall = policy.min_recall_q16.unwrap_or(MIN_ANN_RECALL_Q16);

    if recall < effective_min_recall {
        if !policy.fallback {
            return fallback_disabled_outcome(
                AnnFallbackReason::LowRecall,
                vectors,
                query,
                allowed,
                limit,
                available,
                graph_nodes,
                config,
                policy,
                visited_candidates,
            );
        }
        return exact_from_results(
            exact_results,
            limit,
            available,
            graph_nodes,
            config,
            AnnFallbackReason::LowRecall,
            Some(recall),
            policy,
        );
    }

    let returned_candidates = ann.len();
    AnnSearchOutcome {
        results: ann,
        report: finalize_report(
            AnnSearchReport {
                path: AnnSearchPath::HnswGraph,
                fallback_reason: None,
                fallback_performed: false,
                requested_limit: limit,
                allowed_candidates: available,
                graph_nodes,
                returned_candidates,
                visited_candidates,
                max_visited_candidates: policy.max_visited_candidates,
                recall_q16: Some(recall),
                min_recall_q16: policy.min_recall_q16,
                hnsw_max_neighbors: config.max_neighbors,
                hnsw_ef_search: config.ef_search,
                hnsw_layer_count: config.layer_count,
                hnsw_ef_construction: config.ef_construction,
                upper_graph_edges: config.upper_graph_edges,
                require_slo: policy.require_slo,
                production_safe: true,
                slo_violations: Vec::new(),
            },
            policy,
        ),
    }
}

pub fn evaluate_persisted_ann(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
) -> AnnEvaluationReport {
    evaluate_persisted_ann_with_policy(
        vectors,
        graph,
        query,
        allowed,
        limit,
        AnnSearchPolicy::default(),
    )
}

pub fn evaluate_persisted_ann_with_policy(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    policy: AnnSearchPolicy,
) -> AnnEvaluationReport {
    let exact_results =
        search_persisted_vectors(vectors, query, allowed, limit, &metric_from_graph(graph));
    let exact_top_k = exact_results
        .iter()
        .map(|candidate| candidate.cell_id)
        .collect::<Vec<_>>();
    let available = vectors
        .iter()
        .filter(|(id, vector)| allowed.contains(id) && vector.len() == query.len())
        .count();
    let graph_nodes = graph.links.len();
    let config = hnsw_runtime_config(graph);

    let ann_outcome = match search_hnsw(
        vectors,
        graph,
        query,
        allowed,
        limit,
        config,
        policy.max_visited_candidates,
    ) {
        Ok((results, visited_candidates, budget_exceeded)) => {
            if budget_exceeded {
                exact_from_results(
                    exact_results.clone(),
                    limit,
                    available,
                    graph_nodes,
                    config,
                    AnnFallbackReason::VisitBudgetExceeded,
                    None,
                    policy,
                )
            } else {
                let overlap = results
                    .iter()
                    .filter(|candidate| exact_top_k.iter().any(|exact| exact == &candidate.cell_id))
                    .count();
                let recall = recall_q16(overlap, exact_results.len());
                let max_visited_candidates = policy.max_visited_candidates;
                AnnSearchOutcome {
                    results: results.clone(),
                    report: finalize_report(
                        AnnSearchReport {
                            path: AnnSearchPath::HnswGraph,
                            fallback_reason: None,
                            fallback_performed: false,
                            requested_limit: limit,
                            allowed_candidates: available,
                            graph_nodes,
                            returned_candidates: results.len(),
                            visited_candidates,
                            max_visited_candidates,
                            recall_q16: Some(recall),
                            min_recall_q16: policy.min_recall_q16,
                            hnsw_max_neighbors: config.max_neighbors,
                            hnsw_ef_search: config.ef_search,
                            hnsw_layer_count: config.layer_count,
                            hnsw_ef_construction: config.ef_construction,
                            upper_graph_edges: config.upper_graph_edges,
                            require_slo: policy.require_slo,
                            production_safe: true,
                            slo_violations: Vec::new(),
                        },
                        policy,
                    ),
                }
            }
        }
        Err(reason) => exact_from_results(
            exact_results.clone(),
            limit,
            available,
            graph_nodes,
            config,
            reason,
            None,
            policy,
        ),
    };

    let ann_top_k = ann_outcome
        .results
        .iter()
        .map(|candidate| candidate.cell_id)
        .collect::<Vec<_>>();
    let exact_set = exact_top_k.iter().copied().collect::<BTreeSet<_>>();
    let overlap_count = ann_top_k
        .iter()
        .filter(|candidate| exact_set.contains(candidate))
        .count();
    let recall = recall_q16(overlap_count, exact_results.len());

    let mut search = ann_outcome.report;
    if search.path == AnnSearchPath::HnswGraph {
        search.recall_q16 = Some(recall);
        search.min_recall_q16 = policy.min_recall_q16;
        search = finalize_report(search, policy);
    }

    AnnEvaluationReport {
        search,
        exact_top_k,
        ann_top_k,
        overlap_count,
        recall_q16: recall,
    }
}

fn search_hnsw(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    config: HnswRuntimeConfig,
    max_visited_candidates: Option<usize>,
) -> Result<(Vec<ScoredCandidate>, usize, bool), AnnFallbackReason> {
    if graph.links.is_empty() {
        return Err(AnnFallbackReason::EmptyGraph);
    }
    let index = HnswIndex::from_graph(
        vectors.clone(),
        graph.clone(),
        config.max_neighbors,
        config.ef_search,
    );
    if !index.verify_hnsw_integrity() {
        return Err(AnnFallbackReason::InvalidGraph);
    }
    let (results, visited, budget_exceeded) =
        index.search_allowed_with_budget(query, allowed, limit, max_visited_candidates);
    Ok((results, visited, budget_exceeded))
}

#[allow(clippy::too_many_arguments)]
fn fallback_disabled_outcome(
    reason: AnnFallbackReason,
    vectors: &BTreeMap<u32, Vec<i16>>,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    available: usize,
    graph_nodes: usize,
    config: HnswRuntimeConfig,
    policy: AnnSearchPolicy,
    visited_candidates: usize,
) -> AnnSearchOutcome {
    let cap = if policy.fallback {
        policy.fallback_scan_cap.unwrap_or(limit).min(limit)
    } else {
        policy.fallback_scan_cap.unwrap_or(0).min(limit)
    };
    let ann = if cap == 0 {
        Vec::new()
    } else {
        search_persisted_vectors(vectors, query, allowed, cap, &config.metric)
    };
    let returned = ann.len();
    AnnSearchOutcome {
        results: ann,
        report: finalize_report(
            AnnSearchReport {
                path: AnnSearchPath::HnswGraph,
                fallback_reason: Some(reason),
                fallback_performed: false,
                requested_limit: limit,
                allowed_candidates: available,
                graph_nodes,
                returned_candidates: returned,
                visited_candidates,
                max_visited_candidates: policy.max_visited_candidates,
                recall_q16: None,
                min_recall_q16: policy.min_recall_q16,
                hnsw_max_neighbors: config.max_neighbors,
                hnsw_ef_search: config.ef_search,
                hnsw_layer_count: config.layer_count,
                hnsw_ef_construction: config.ef_construction,
                upper_graph_edges: config.upper_graph_edges,
                require_slo: policy.require_slo,
                production_safe: true,
                slo_violations: Vec::new(),
            },
            policy,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn exact(
    vectors: &BTreeMap<u32, Vec<i16>>,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    available: usize,
    graph_nodes: usize,
    config: HnswRuntimeConfig,
    reason: AnnFallbackReason,
    policy: AnnSearchPolicy,
) -> AnnSearchOutcome {
    let results = search_persisted_vectors(vectors, query, allowed, limit, &config.metric);
    exact_from_results(
        results,
        limit,
        available,
        graph_nodes,
        config,
        reason,
        None,
        policy,
    )
}

fn metric_from_graph(graph: &HnswGraphIndex) -> DistanceMetric {
    match graph.metric {
        1 => DistanceMetric::Cosine,
        2 => DistanceMetric::L2,
        _ => DistanceMetric::DotProduct,
    }
}

#[allow(clippy::too_many_arguments)]
fn exact_from_results(
    results: Vec<ScoredCandidate>,
    limit: usize,
    available: usize,
    graph_nodes: usize,
    config: HnswRuntimeConfig,
    reason: AnnFallbackReason,
    recall_q16: Option<u16>,
    policy: AnnSearchPolicy,
) -> AnnSearchOutcome {
    let returned = results.len();
    AnnSearchOutcome {
        results,
        report: finalize_report(
            AnnSearchReport {
                path: AnnSearchPath::ExactFallback,
                fallback_reason: Some(reason),
                fallback_performed: true,
                requested_limit: limit,
                allowed_candidates: available,
                graph_nodes,
                returned_candidates: returned,
                visited_candidates: 0,
                max_visited_candidates: policy.max_visited_candidates,
                recall_q16,
                min_recall_q16: policy.min_recall_q16,
                hnsw_max_neighbors: config.max_neighbors,
                hnsw_ef_search: config.ef_search,
                hnsw_layer_count: config.layer_count,
                hnsw_ef_construction: config.ef_construction,
                upper_graph_edges: config.upper_graph_edges,
                require_slo: policy.require_slo,
                production_safe: true,
                slo_violations: Vec::new(),
            },
            policy,
        ),
    }
}

pub(crate) fn finalize_report(
    mut report: AnnSearchReport,
    policy: AnnSearchPolicy,
) -> AnnSearchReport {
    report.fallback_performed = report.path == AnnSearchPath::ExactFallback;
    report.require_slo = policy.require_slo;
    report.slo_violations = slo_violations(&report, policy);
    report.production_safe = report.slo_violations.is_empty();
    report
}

fn slo_violations(report: &AnnSearchReport, policy: AnnSearchPolicy) -> Vec<AnnSloViolation> {
    let mut violations = Vec::new();
    if policy.require_slo
        && report.path == AnnSearchPath::HnswGraph
        && is_weak_multi_layer_topology(report)
    {
        violations.push(AnnSloViolation::WeakMultiLayerTopology);
    }
    if let Some(reason) = report.fallback_reason {
        violations.push(match reason {
            AnnFallbackReason::EmptyGraph => AnnSloViolation::EmptyGraph,
            AnnFallbackReason::InvalidGraph => AnnSloViolation::InvalidGraph,
            AnnFallbackReason::StaleGraph => AnnSloViolation::StaleGraph,
            AnnFallbackReason::InsufficientResults => AnnSloViolation::InsufficientResults,
            AnnFallbackReason::LowRecall => AnnSloViolation::LowRecall,
            AnnFallbackReason::VisitBudgetExceeded => AnnSloViolation::VisitBudgetExceeded,
            AnnFallbackReason::NoPersistedSegments => AnnSloViolation::NoPersistedSegments,
            AnnFallbackReason::UncheckpointedChanges => AnnSloViolation::UncheckpointedChanges,
            AnnFallbackReason::HnswDisabled => AnnSloViolation::HnswDisabled,
        });
    }
    violations.retain(|violation| *violation != AnnSloViolation::HnswDisabled);
    if let (Some(recall), Some(min_recall)) = (report.recall_q16, policy.min_recall_q16) {
        if recall < min_recall && !violations.contains(&AnnSloViolation::LowRecall) {
            violations.push(AnnSloViolation::RecallBelowMinimum);
        }
    }
    violations
}

fn is_weak_multi_layer_topology(report: &AnnSearchReport) -> bool {
    if report.graph_nodes < ANN_MIN_NODES_FOR_SLO_MULTI_LAYER {
        return false;
    }
    if report.hnsw_layer_count <= 1 {
        return false;
    }
    report.upper_graph_edges == 0
}

fn recall_q16(overlap_count: usize, expected_count: usize) -> u16 {
    if expected_count == 0 {
        return 65_535;
    }
    ((overlap_count as u64 * 65_535) / expected_count as u64) as u16
}

#[cfg(test)]
mod tests;
