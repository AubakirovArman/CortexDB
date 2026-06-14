use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, EngineResult};

pub const EMBEDDING_PIPELINE_REPORT_SCHEMA: &str = "cortexdb.embedding_pipeline.coverage.v1";
pub const DEFAULT_MIN_EMBEDDING_COVERAGE_BPS: u32 = 9_950;
pub(super) const SAMPLE_LIMIT: usize = 25;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddingCoverageConfig {
    pub expected_dimension: Option<usize>,
    pub min_coverage_basis_points: u32,
    pub expected_model: Option<String>,
}

impl Default for EmbeddingCoverageConfig {
    fn default() -> Self {
        Self {
            expected_dimension: None,
            min_coverage_basis_points: DEFAULT_MIN_EMBEDDING_COVERAGE_BPS,
            expected_model: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingExpectedItem {
    pub doc_id: String,
    pub text_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingCoverageReport {
    pub schema_version: String,
    pub total_items: usize,
    pub embedded_items: usize,
    pub missing_items: usize,
    pub duplicate_items: usize,
    pub unexpected_items: usize,
    pub invalid_rows: usize,
    pub empty_vector_rows: usize,
    pub dimension_mismatch_rows: usize,
    pub stale_items: usize,
    pub dimension: Option<usize>,
    pub expected_dimension: Option<usize>,
    pub expected_model: Option<String>,
    pub coverage_basis_points: u32,
    pub min_coverage_basis_points: u32,
    pub production_ready: bool,
    pub missing_ids_sample: Vec<String>,
    pub duplicate_ids_sample: Vec<String>,
    pub unexpected_ids_sample: Vec<String>,
    pub stale_ids_sample: Vec<String>,
    pub invalid_row_samples: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingDebtItem {
    pub cell_id: u64,
    pub reason: String,
    pub expected_text_hash: String,
    pub observed_model: Option<String>,
    pub observed_dimension: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingDebtReport {
    pub schema_version: String,
    pub total_items: usize,
    pub ready_items: usize,
    pub debt_items: usize,
    pub missing_vector_items: usize,
    pub dimension_mismatch_items: usize,
    pub stale_model_items: usize,
    pub stale_text_hash_items: usize,
    pub expected_model: Option<String>,
    pub expected_dimension: Option<usize>,
    pub debt_sample: Vec<EmbeddingDebtItem>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EmbeddingBackfillOptions {
    pub config: EmbeddingCoverageConfig,
    pub max_items: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingBackfillReport {
    pub schema_version: String,
    pub scanned_items: usize,
    pub debt_items: usize,
    pub embedded_items: usize,
    pub failed_items: usize,
    pub skipped_items: usize,
    pub final_debt_items: usize,
    pub failure_samples: Vec<String>,
}

pub trait EmbeddingBackfillProvider {
    fn embed_text(&mut self, text: &str) -> EngineResult<Vec<i16>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddingClientConfig {
    pub url: String,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub timeout_ms: u64,
}

pub trait QueryEmbeddingProvider {
    fn embed_query(&mut self, text: &str) -> EngineResult<Vec<i16>>;
}

impl<F> EmbeddingBackfillProvider for F
where
    F: FnMut(&str) -> EngineResult<Vec<i16>>,
{
    fn embed_text(&mut self, text: &str) -> EngineResult<Vec<i16>> {
        self(text)
    }
}

impl<F> QueryEmbeddingProvider for F
where
    F: FnMut(&str) -> EngineResult<Vec<i16>>,
{
    fn embed_query(&mut self, text: &str) -> EngineResult<Vec<i16>> {
        self(text)
    }
}

impl EmbeddingCoverageReport {
    pub fn coverage_percent(&self) -> f32 {
        self.coverage_basis_points as f32 / 100.0
    }

    pub fn write_json(&self, path: impl AsRef<Path>) -> EngineResult<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|error| EngineError::InvalidAnnCorpus(error.to_string()))?;
        std::fs::write(path, content + "\n")?;
        Ok(())
    }
}
