use cortex_core::memtable::CellVersion;

use crate::search::vector::vector_from_payload;

use super::payload::{embedding_payload_field, embedding_text_hash};
use super::types::{
    EmbeddingCoverageConfig, EmbeddingDebtItem, EmbeddingDebtReport, EmbeddingExpectedItem,
    SAMPLE_LIMIT,
};

pub fn embedding_expected_items_from_versions(
    versions: &[CellVersion],
) -> Vec<EmbeddingExpectedItem> {
    versions
        .iter()
        .filter(|version| version.deleted_seq.is_none())
        .map(|version| EmbeddingExpectedItem {
            doc_id: version.cell_id.0.to_string(),
            text_hash: Some(embedding_text_hash(&version.payload)),
        })
        .collect()
}

pub fn embedding_debt_report_from_versions(
    versions: &[CellVersion],
    config: EmbeddingCoverageConfig,
) -> EmbeddingDebtReport {
    let mut total_items = 0;
    let mut ready_items = 0;
    let mut missing_vector_items = 0;
    let mut dimension_mismatch_items = 0;
    let mut stale_model_items = 0;
    let mut stale_text_hash_items = 0;
    let mut debt_sample = Vec::new();

    for version in versions
        .iter()
        .filter(|version| version.deleted_seq.is_none())
    {
        total_items += 1;
        let expected_text_hash = embedding_text_hash(&version.payload);
        let observed_model = embedding_payload_field(&version.payload, "embedding_model");
        let observed_text_hash = embedding_payload_field(&version.payload, "embedding_text_hash");
        let observed_vector = vector_from_payload(&version.payload);
        let observed_dimension = observed_vector.as_ref().map(Vec::len);
        let reason = if observed_vector.is_none() {
            missing_vector_items += 1;
            Some("missing_vector")
        } else if config
            .expected_dimension
            .is_some_and(|expected| observed_dimension != Some(expected))
        {
            dimension_mismatch_items += 1;
            Some("dimension_mismatch")
        } else if config
            .expected_model
            .as_deref()
            .is_some_and(|expected| observed_model.as_deref() != Some(expected))
        {
            stale_model_items += 1;
            Some("stale_model")
        } else if observed_text_hash.as_deref() != Some(expected_text_hash.as_str()) {
            stale_text_hash_items += 1;
            Some("stale_text_hash")
        } else {
            ready_items += 1;
            None
        };

        if let Some(reason) = reason {
            push_debt_sample(
                &mut debt_sample,
                EmbeddingDebtItem {
                    cell_id: version.cell_id.0,
                    reason: reason.to_owned(),
                    expected_text_hash,
                    observed_model,
                    observed_dimension,
                },
            );
        }
    }

    EmbeddingDebtReport {
        schema_version: "cortexdb.embedding_pipeline.debt.v1".to_owned(),
        total_items,
        ready_items,
        debt_items: total_items.saturating_sub(ready_items),
        missing_vector_items,
        dimension_mismatch_items,
        stale_model_items,
        stale_text_hash_items,
        expected_model: config.expected_model,
        expected_dimension: config.expected_dimension,
        debt_sample,
    }
}

pub(super) fn embedding_debt_reason(
    version: &CellVersion,
    config: &EmbeddingCoverageConfig,
) -> Option<&'static str> {
    let expected_text_hash = embedding_text_hash(&version.payload);
    let observed_model = embedding_payload_field(&version.payload, "embedding_model");
    let observed_text_hash = embedding_payload_field(&version.payload, "embedding_text_hash");
    let observed_vector = vector_from_payload(&version.payload);
    let observed_dimension = observed_vector.as_ref().map(Vec::len);

    if observed_vector.is_none() {
        Some("missing_vector")
    } else if config
        .expected_dimension
        .is_some_and(|expected| observed_dimension != Some(expected))
    {
        Some("dimension_mismatch")
    } else if config
        .expected_model
        .as_deref()
        .is_some_and(|expected| observed_model.as_deref() != Some(expected))
    {
        Some("stale_model")
    } else if observed_text_hash.as_deref() != Some(expected_text_hash.as_str()) {
        Some("stale_text_hash")
    } else {
        None
    }
}

fn push_debt_sample(samples: &mut Vec<EmbeddingDebtItem>, value: EmbeddingDebtItem) {
    if samples.len() < SAMPLE_LIMIT {
        samples.push(value);
    }
}
