use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

use crate::error::EngineResult;

use super::types::{
    EmbeddingCoverageConfig, EmbeddingCoverageReport, EmbeddingExpectedItem,
    EMBEDDING_PIPELINE_REPORT_SCHEMA, SAMPLE_LIMIT,
};

#[derive(Deserialize)]
struct EmbeddingJsonlRow {
    doc_id: Option<String>,
    vector: Option<Vec<f32>>,
    model: Option<String>,
    text_hash: Option<String>,
}

pub fn embedding_coverage_report_from_jsonl(
    expected_ids: impl IntoIterator<Item = String>,
    embedding_jsonl: &str,
    config: EmbeddingCoverageConfig,
) -> EmbeddingCoverageReport {
    embedding_coverage_report_from_expected_items(
        expected_ids
            .into_iter()
            .map(|doc_id| EmbeddingExpectedItem {
                doc_id,
                text_hash: None,
            }),
        embedding_jsonl,
        config,
    )
}

pub fn embedding_retry_ids_from_jsonl(
    expected_ids: impl IntoIterator<Item = String>,
    embedding_jsonl: &str,
    config: EmbeddingCoverageConfig,
) -> Vec<String> {
    embedding_retry_ids_from_expected_items(
        expected_ids
            .into_iter()
            .map(|doc_id| EmbeddingExpectedItem {
                doc_id,
                text_hash: None,
            }),
        embedding_jsonl,
        config,
    )
}

pub fn embedding_coverage_report_from_expected_items(
    expected_items: impl IntoIterator<Item = EmbeddingExpectedItem>,
    embedding_jsonl: &str,
    config: EmbeddingCoverageConfig,
) -> EmbeddingCoverageReport {
    embedding_analysis_from_expected_items(expected_items, embedding_jsonl, config).report
}

pub fn embedding_retry_ids_from_expected_items(
    expected_items: impl IntoIterator<Item = EmbeddingExpectedItem>,
    embedding_jsonl: &str,
    config: EmbeddingCoverageConfig,
) -> Vec<String> {
    embedding_analysis_from_expected_items(expected_items, embedding_jsonl, config).retry_ids
}

struct EmbeddingAnalysis {
    report: EmbeddingCoverageReport,
    retry_ids: Vec<String>,
}

fn embedding_analysis_from_expected_items(
    expected_items: impl IntoIterator<Item = EmbeddingExpectedItem>,
    embedding_jsonl: &str,
    config: EmbeddingCoverageConfig,
) -> EmbeddingAnalysis {
    let expected_items = expected_items
        .into_iter()
        .filter(|item| !item.doc_id.trim().is_empty())
        .map(|item| (item.doc_id, item.text_hash))
        .collect::<BTreeMap<_, _>>();
    let expected = expected_items.keys().cloned().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut duplicate_ids_sample = Vec::new();
    let mut unexpected_ids_sample = Vec::new();
    let mut invalid_row_samples = Vec::new();
    let mut duplicate_items = 0;
    let mut unexpected_items = 0;
    let mut invalid_rows = 0;
    let mut empty_vector_rows = 0;
    let mut dimension_mismatch_rows = 0;
    let mut stale_ids_sample = Vec::new();
    let mut stale_items = 0;
    let mut dimension = config.expected_dimension;

    for (index, line) in embedding_jsonl.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = match serde_json::from_str::<EmbeddingJsonlRow>(trimmed) {
            Ok(row) => row,
            Err(error) => {
                invalid_rows += 1;
                push_sample(
                    &mut invalid_row_samples,
                    format!("line {}: invalid json: {error}", index + 1),
                );
                continue;
            }
        };
        let Some(doc_id) = row.doc_id.clone().filter(|id| !id.trim().is_empty()) else {
            invalid_rows += 1;
            push_sample(
                &mut invalid_row_samples,
                format!("line {}: missing doc_id", index + 1),
            );
            continue;
        };
        let Some(vector) = row.vector.as_ref() else {
            invalid_rows += 1;
            push_sample(
                &mut invalid_row_samples,
                format!("line {}: missing vector for {doc_id}", index + 1),
            );
            continue;
        };
        if vector.is_empty() {
            empty_vector_rows += 1;
            push_sample(
                &mut invalid_row_samples,
                format!("line {}: empty vector for {doc_id}", index + 1),
            );
            continue;
        }
        match dimension {
            Some(expected_dimension) if expected_dimension != vector.len() => {
                dimension_mismatch_rows += 1;
                push_sample(
                    &mut invalid_row_samples,
                    format!(
                        "line {}: vector dimension {} for {doc_id}, expected {expected_dimension}",
                        index + 1,
                        vector.len()
                    ),
                );
                continue;
            }
            Some(_) => {}
            None => {
                dimension = Some(vector.len());
            }
        }
        if !expected.contains(&doc_id) {
            unexpected_items += 1;
            push_sample(&mut unexpected_ids_sample, doc_id);
            continue;
        }
        if is_stale_embedding(
            &row,
            expected_items.get(&doc_id),
            config.expected_model.as_deref(),
        ) {
            stale_items += 1;
            push_sample(&mut stale_ids_sample, doc_id);
            continue;
        }
        if !seen.insert(doc_id.clone()) {
            duplicate_items += 1;
            push_sample(&mut duplicate_ids_sample, doc_id);
        }
    }

    let missing_ids_sample = expected
        .iter()
        .filter(|id| !seen.contains(*id))
        .take(SAMPLE_LIMIT)
        .cloned()
        .collect::<Vec<_>>();
    let total_items = expected.len();
    let embedded_items = seen.len();
    let missing_items = total_items.saturating_sub(embedded_items);
    let coverage_basis_points = if total_items == 0 {
        10_000
    } else {
        ((embedded_items as u128 * 10_000) / total_items as u128) as u32
    };
    let production_ready = coverage_basis_points >= config.min_coverage_basis_points
        && duplicate_items == 0
        && unexpected_items == 0
        && invalid_rows == 0
        && empty_vector_rows == 0
        && dimension_mismatch_rows == 0
        && stale_items == 0;

    let retry_ids = expected
        .iter()
        .filter(|id| !seen.contains(*id))
        .cloned()
        .collect::<Vec<_>>();

    EmbeddingAnalysis {
        report: EmbeddingCoverageReport {
            schema_version: EMBEDDING_PIPELINE_REPORT_SCHEMA.to_owned(),
            total_items,
            embedded_items,
            missing_items,
            duplicate_items,
            unexpected_items,
            invalid_rows,
            empty_vector_rows,
            dimension_mismatch_rows,
            stale_items,
            dimension,
            expected_dimension: config.expected_dimension,
            expected_model: config.expected_model.clone(),
            coverage_basis_points,
            min_coverage_basis_points: config.min_coverage_basis_points,
            production_ready,
            missing_ids_sample,
            duplicate_ids_sample,
            unexpected_ids_sample,
            stale_ids_sample,
            invalid_row_samples,
        },
        retry_ids,
    }
}

fn is_stale_embedding(
    row: &EmbeddingJsonlRow,
    expected_text_hash: Option<&Option<String>>,
    expected_model: Option<&str>,
) -> bool {
    if let Some(expected_model) = expected_model {
        if row.model.as_deref() != Some(expected_model) {
            return true;
        }
    }
    if let Some(Some(expected_text_hash)) = expected_text_hash {
        if row.text_hash.as_deref() != Some(expected_text_hash.as_str()) {
            return true;
        }
    }
    false
}

pub fn embedding_coverage_report_from_files(
    expected_ids_path: impl AsRef<Path>,
    embeddings_path: impl AsRef<Path>,
    config: EmbeddingCoverageConfig,
) -> EngineResult<EmbeddingCoverageReport> {
    let expected_ids = std::fs::read_to_string(expected_ids_path)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let embedding_jsonl = std::fs::read_to_string(embeddings_path)?;
    Ok(embedding_coverage_report_from_jsonl(
        expected_ids,
        &embedding_jsonl,
        config,
    ))
}

fn push_sample(samples: &mut Vec<String>, value: String) {
    if samples.len() < SAMPLE_LIMIT && !samples.contains(&value) {
        samples.push(value);
    }
}
