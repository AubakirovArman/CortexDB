use std::collections::BTreeSet;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, EngineResult};

use super::hnsw::{DistanceMetric, HnswIndex, VectorCollectionConfig};

use self::compare::compare_corpus_report;
use self::loader::load_ann_corpus;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AnnCorpusVector {
    pub candidate: u32,
    pub vector: Vec<i16>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AnnCorpusQuery {
    pub name: String,
    pub vector: Vec<i16>,
    pub limit: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AnnCorpusGroundTruth {
    pub name: String,
    pub candidates: Vec<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnCorpusOptions {
    pub metric: DistanceMetric,
    pub max_neighbors: usize,
    pub ef_search: usize,
    pub layer_count: usize,
    pub ef_construction: usize,
    pub min_recall_q16: u16,
    pub min_mean_recall_q16: u16,
    pub max_p95_latency_nanos: u128,
    pub max_max_latency_nanos: u128,
    pub require_production_safe: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnnCorpusReport {
    pub passed: bool,
    pub failures: Vec<String>,
    pub metric: String,
    pub required_min_recall_q16: u16,
    pub required_min_mean_recall_q16: u16,
    pub allowed_p95_latency_nanos: u128,
    pub allowed_max_latency_nanos: u128,
    pub require_production_safe: bool,
    pub hnsw_max_neighbors: usize,
    pub hnsw_ef_search: usize,
    pub hnsw_layer_count: usize,
    pub hnsw_ef_construction: usize,
    pub vector_count: usize,
    pub query_count: usize,
    pub dimension: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub upper_layers: usize,
    pub upper_graph_edges: usize,
    pub min_observed_recall_q16: u16,
    pub mean_recall_q16: u16,
    pub p50_latency_nanos: u128,
    pub p95_latency_nanos: u128,
    pub max_latency_nanos: u128,
    pub production_safe: bool,
    pub queries: Vec<AnnCorpusQueryReport>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnnCorpusQueryReport {
    pub name: String,
    pub limit: usize,
    pub truth_count: usize,
    pub returned_count: usize,
    pub recall_q16: u16,
    pub latency_nanos: u128,
    pub production_safe: bool,
}

impl Default for AnnCorpusOptions {
    fn default() -> Self {
        Self {
            metric: DistanceMetric::DotProduct,
            max_neighbors: 8,
            ef_search: 64,
            layer_count: 4,
            ef_construction: 64,
            min_recall_q16: 49_151,
            min_mean_recall_q16: 49_151,
            max_p95_latency_nanos: 100_000_000,
            max_max_latency_nanos: 250_000_000,
            require_production_safe: true,
        }
    }
}

impl AnnCorpusReport {
    pub fn as_json(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| "{\"error\":\"ann_corpus_report_serialization_failed\"}".to_owned())
    }
}

pub fn evaluate_ann_corpus(
    vectors_jsonl: &str,
    queries_jsonl: &str,
    ground_truth_jsonl: &str,
    options: AnnCorpusOptions,
) -> EngineResult<AnnCorpusReport> {
    let corpus = load_ann_corpus(vectors_jsonl, queries_jsonl, ground_truth_jsonl)?;
    let max_neighbors = options.max_neighbors.max(1);
    let ef_search = options.ef_search.max(1);
    let layer_count = options.layer_count.max(1);
    let ef_construction = options.ef_construction.max(max_neighbors).max(1);
    let mut index = HnswIndex::new_multilayer(max_neighbors, ef_search, layer_count);
    index.set_ef_construction(ef_construction);
    index.set_config(VectorCollectionConfig {
        dimension: corpus.dimension,
        metric: options.metric,
    });
    for (candidate, vector) in &corpus.vectors {
        index.add_vector(*candidate, vector.clone())?;
    }
    let graph = index.graph_index();
    let persisted_index = HnswIndex::from_graph(
        corpus.vectors.clone(),
        graph.clone(),
        max_neighbors,
        ef_search,
    );
    let allowed = corpus.vectors.keys().copied().collect::<BTreeSet<_>>();
    let mut latencies = Vec::with_capacity(corpus.queries.len());
    let mut recalls = Vec::with_capacity(corpus.queries.len());
    let mut query_reports = Vec::with_capacity(corpus.queries.len());
    let mut production_safe = persisted_index.verify_hnsw_integrity();
    for query in &corpus.queries {
        let truth = &corpus.ground_truth[&query.name];
        let started = Instant::now();
        let (results, _visited, budget_exceeded) =
            persisted_index.search_allowed_with_budget(&query.vector, &allowed, query.limit, None);
        let latency = started.elapsed().as_nanos();
        let recall = recall_q16(
            &results
                .iter()
                .map(|candidate| candidate.cell_id)
                .collect::<Vec<_>>(),
            truth,
            query.limit,
        );
        let query_safe = !budget_exceeded && recall >= options.min_recall_q16;
        production_safe &= query_safe;
        latencies.push(latency);
        recalls.push(recall);
        query_reports.push(AnnCorpusQueryReport {
            name: query.name.clone(),
            limit: query.limit,
            truth_count: truth.len(),
            returned_count: results.len(),
            recall_q16: recall,
            latency_nanos: latency,
            production_safe: query_safe,
        });
    }
    latencies.sort_unstable();
    recalls.sort_unstable();
    let mean_recall_q16 =
        (recalls.iter().copied().map(u64::from).sum::<u64>() / recalls.len() as u64) as u16;
    let mut report = AnnCorpusReport {
        passed: false,
        failures: Vec::new(),
        metric: metric_name(options.metric).to_owned(),
        required_min_recall_q16: options.min_recall_q16,
        required_min_mean_recall_q16: options.min_mean_recall_q16,
        allowed_p95_latency_nanos: options.max_p95_latency_nanos,
        allowed_max_latency_nanos: options.max_max_latency_nanos,
        require_production_safe: options.require_production_safe,
        hnsw_max_neighbors: max_neighbors,
        hnsw_ef_search: ef_search,
        hnsw_layer_count: layer_count,
        hnsw_ef_construction: ef_construction,
        vector_count: corpus.vectors.len(),
        query_count: corpus.queries.len(),
        dimension: corpus.dimension,
        graph_nodes: graph.links.len(),
        graph_edges: graph.links.values().map(|neighbors| neighbors.len()).sum(),
        upper_layers: graph.upper_layers.len(),
        upper_graph_edges: graph
            .upper_layers
            .values()
            .flat_map(|links| links.values())
            .map(|neighbors| neighbors.len())
            .sum(),
        min_observed_recall_q16: recalls[0],
        mean_recall_q16,
        p50_latency_nanos: percentile(&latencies, 50),
        p95_latency_nanos: percentile(&latencies, 95),
        max_latency_nanos: *latencies.last().unwrap_or(&0),
        production_safe,
        queries: query_reports,
    };
    report.failures = compare_corpus_report(options, &report);
    report.passed = report.failures.is_empty();
    Ok(report)
}

pub fn parse_ann_metric(value: &str) -> EngineResult<DistanceMetric> {
    match value {
        "dot_product" | "dotproduct" | "dot" => Ok(DistanceMetric::DotProduct),
        "cosine" => Ok(DistanceMetric::Cosine),
        "l2" | "euclidean" => Ok(DistanceMetric::L2),
        _ => Err(EngineError::InvalidAnnCorpus(format!(
            "unknown metric {value}"
        ))),
    }
}

pub fn metric_name(metric: DistanceMetric) -> &'static str {
    match metric {
        DistanceMetric::DotProduct => "dot_product",
        DistanceMetric::Cosine => "cosine",
        DistanceMetric::L2 => "l2",
    }
}

fn recall_q16(results: &[u32], truth: &[u32], limit: usize) -> u16 {
    let denominator = truth.len().min(limit);
    if denominator == 0 {
        return 65_535;
    }
    let truth = truth.iter().take(limit).copied().collect::<BTreeSet<_>>();
    let overlap = results
        .iter()
        .take(limit)
        .filter(|candidate| truth.contains(candidate))
        .count();
    ((overlap as u64 * 65_535) / denominator as u64) as u16
}

fn percentile(values: &[u128], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    values[((values.len() - 1) * percentile.min(100)) / 100]
}

mod compare;
mod loader;

#[cfg(test)]
mod tests;
