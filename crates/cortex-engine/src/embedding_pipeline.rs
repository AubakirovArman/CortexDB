mod backfill;
mod coverage;
mod debt;
mod payload;
mod types;

#[cfg(test)]
mod tests;

pub use coverage::{
    embedding_coverage_report_from_expected_items, embedding_coverage_report_from_files,
    embedding_coverage_report_from_jsonl, embedding_retry_ids_from_expected_items,
    embedding_retry_ids_from_jsonl,
};
pub use debt::{embedding_debt_report_from_versions, embedding_expected_items_from_versions};
pub use payload::embedding_text_hash;
pub use types::{
    EmbeddingBackfillOptions, EmbeddingBackfillProvider, EmbeddingBackfillReport,
    EmbeddingClientConfig, EmbeddingCoverageConfig, EmbeddingCoverageReport, EmbeddingDebtItem,
    EmbeddingDebtReport, EmbeddingExpectedItem, QueryEmbeddingProvider,
    DEFAULT_MIN_EMBEDDING_COVERAGE_BPS, EMBEDDING_PIPELINE_REPORT_SCHEMA,
};
