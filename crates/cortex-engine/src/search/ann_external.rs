use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, EngineResult};

use super::ann::{evaluate_persisted_ann_with_policy, AnnSearchPolicy};
use super::hnsw::{DistanceMetric, HnswIndex, VectorCollectionConfig};

use self::compare::compare_external_baseline;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum AnnJsonlEntry {
    #[serde(rename = "vector")]
    Vector { candidate: u32, vector: Vec<i16> },
    #[serde(rename = "query")]
    Query {
        name: String,
        vector: Vec<i16>,
        limit: usize,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct AnnExternalFixtureBaseline {
    pub baseline_id: String,
    pub fixture_id: String,
    pub fixture_path: String,
    pub policy_min_recall_q16: u16,
    pub require_slo: bool,
    pub min_observed_recall_q16: u16,
    pub min_mean_recall_q16: u16,
    pub min_graph_nodes: usize,
    pub min_graph_edges: usize,
    pub min_upper_layers: usize,
    pub min_upper_graph_edges: usize,
    pub max_p95_latency_nanos: u128,
    pub max_p99_latency_nanos: u128,
    pub max_max_latency_nanos: u128,
    pub require_production_safe: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnnExternalFixtureReport {
    pub baseline_id: String,
    pub fixture_id: String,
    pub passed: bool,
    pub failures: Vec<String>,
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
    pub p99_latency_nanos: u128,
    pub max_latency_nanos: u128,
    pub fallback_count: usize,
    pub fallback_rate_q16: u16,
    pub production_safe: bool,
}

pub(super) struct AnnExternalFixture {
    pub(super) vectors: BTreeMap<u32, Vec<i16>>,
    pub(super) queries: Vec<AnnExternalQuery>,
    pub(super) dimension: usize,
}

pub(super) struct AnnExternalQuery {
    pub(super) vector: Vec<i16>,
    pub(super) limit: usize,
}

impl AnnExternalFixtureReport {
    pub fn as_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            "{\"error\":\"ann_external_fixture_report_serialization_failed\"}".to_owned()
        })
    }
}

pub fn evaluate_ann_external_fixture(
    baseline: &AnnExternalFixtureBaseline,
    fixture_text: &str,
) -> EngineResult<AnnExternalFixtureReport> {
    let fixture = parse_ann_jsonl_fixture(fixture_text)?;
    let mut index = HnswIndex::new_multilayer(8, 64, 4);
    index.set_config(VectorCollectionConfig {
        dimension: fixture.dimension,
        metric: DistanceMetric::DotProduct,
    });
    for (candidate, vector) in &fixture.vectors {
        index.add_vector(*candidate, vector.clone())?;
    }
    let graph = index.graph_index();
    let allowed = fixture.vectors.keys().copied().collect::<BTreeSet<_>>();
    let mut recalls = Vec::with_capacity(fixture.queries.len());
    let mut latencies = Vec::with_capacity(fixture.queries.len());
    let mut production_safe = true;
    let mut fallback_count = 0usize;
    for query in &fixture.queries {
        let started = Instant::now();
        let report = evaluate_persisted_ann_with_policy(
            &fixture.vectors,
            &graph,
            &query.vector,
            &allowed,
            query.limit,
            AnnSearchPolicy {
                min_recall_q16: Some(baseline.policy_min_recall_q16),
                require_slo: baseline.require_slo,
                ..AnnSearchPolicy::default()
            },
        );
        latencies.push(started.elapsed().as_nanos());
        recalls.push(report.recall_q16);
        if report.search.fallback_performed {
            fallback_count += 1;
        }
        production_safe &= report.search.production_safe;
    }
    latencies.sort_unstable();
    recalls.sort_unstable();
    let mean_recall_q16 =
        (recalls.iter().copied().map(u64::from).sum::<u64>() / recalls.len() as u64) as u16;
    let mut report = AnnExternalFixtureReport {
        baseline_id: baseline.baseline_id.clone(),
        fixture_id: baseline.fixture_id.clone(),
        passed: false,
        failures: Vec::new(),
        vector_count: fixture.vectors.len(),
        query_count: fixture.queries.len(),
        dimension: fixture.dimension,
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
        p99_latency_nanos: percentile(&latencies, 99),
        max_latency_nanos: *latencies.last().unwrap_or(&0),
        fallback_count,
        fallback_rate_q16: ratio_q16(fallback_count, fixture.queries.len()),
        production_safe,
    };
    report.failures = compare_external_baseline(baseline, &report);
    report.passed = report.failures.is_empty();
    Ok(report)
}

pub(super) fn parse_ann_jsonl_fixture(input: &str) -> EngineResult<AnnExternalFixture> {
    let mut vectors = BTreeMap::new();
    let mut queries = Vec::new();
    let mut dimension = None;
    for (line_index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let entry: AnnJsonlEntry = serde_json::from_str(line).map_err(|error| {
            EngineError::InvalidAnnFixture(format!("line {}: {error}", line_index + 1))
        })?;
        match entry {
            AnnJsonlEntry::Vector { candidate, vector } => {
                validate_vector(candidate, &vector, &mut dimension, line_index + 1)?;
                if vectors.insert(candidate, vector).is_some() {
                    return Err(invalid_fixture(format!("duplicate candidate {candidate}")));
                }
            }
            AnnJsonlEntry::Query {
                name: _,
                vector,
                limit,
            } => {
                validate_query(&vector, limit, &mut dimension, line_index + 1)?;
                queries.push(AnnExternalQuery { vector, limit });
            }
        }
    }
    if vectors.is_empty() || queries.is_empty() {
        return Err(invalid_fixture("fixture must contain vectors and queries"));
    }
    Ok(AnnExternalFixture {
        vectors,
        queries,
        dimension: dimension.unwrap_or(0),
    })
}

fn validate_vector(
    candidate: u32,
    vector: &[i16],
    dimension: &mut Option<usize>,
    line: usize,
) -> EngineResult<()> {
    if candidate == 0 {
        return Err(invalid_fixture(format!("line {line}: candidate id 0")));
    }
    validate_dimension(vector, dimension, line)
}

fn validate_query(
    vector: &[i16],
    limit: usize,
    dimension: &mut Option<usize>,
    line: usize,
) -> EngineResult<()> {
    if limit == 0 {
        return Err(invalid_fixture(format!("line {line}: query limit is zero")));
    }
    validate_dimension(vector, dimension, line)
}

fn validate_dimension(
    vector: &[i16],
    dimension: &mut Option<usize>,
    line: usize,
) -> EngineResult<()> {
    if vector.is_empty() {
        return Err(invalid_fixture(format!("line {line}: empty vector")));
    }
    match dimension {
        Some(expected) if *expected != vector.len() => Err(invalid_fixture(format!(
            "line {line}: vector dimension {}, expected {}",
            vector.len(),
            expected
        ))),
        Some(_) => Ok(()),
        None => {
            *dimension = Some(vector.len());
            Ok(())
        }
    }
}

fn invalid_fixture(message: impl Into<String>) -> EngineError {
    EngineError::InvalidAnnFixture(message.into())
}

fn percentile(values: &[u128], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    values[((values.len() - 1) * percentile.min(100)) / 100]
}

fn ratio_q16(numerator: usize, denominator: usize) -> u16 {
    if denominator == 0 {
        return 65_535;
    }
    ((numerator as u64 * 65_535) / denominator as u64) as u16
}

mod compare;

#[cfg(test)]
mod tests;
