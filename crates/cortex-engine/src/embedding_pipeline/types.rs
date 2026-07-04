use std::path::Path;

use cortex_storage::manifest::ManifestEmbeddingProfile;
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

/// Store-wide embedding provenance recorded in the manifest: which model built
/// the stored vectors, at what dimension, with which distance metric. Opening a
/// store with an incompatible profile fails closed instead of silently mixing
/// vector spaces. Mirrors [`ManifestEmbeddingProfile`] but lives engine-side so
/// callers do not depend on the storage crate directly.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingProfile {
    pub model: String,
    pub dimension: u32,
    pub metric: u32,
}

impl EmbeddingProfile {
    pub fn to_manifest_profile(&self) -> ManifestEmbeddingProfile {
        ManifestEmbeddingProfile {
            model: self.model.clone(),
            dimension: self.dimension,
            metric: self.metric,
        }
    }

    pub fn from_manifest_profile(profile: &ManifestEmbeddingProfile) -> Self {
        Self {
            model: profile.model.clone(),
            dimension: profile.dimension,
            metric: profile.metric,
        }
    }

    /// Stable lowercase metric name used in the human/wire-visible `embedding_ref`.
    pub fn metric_str(&self) -> &'static str {
        metric_str(self.metric)
    }

    /// The per-cell provenance string
    /// `emb1:<model>:<dimension>:<metric>:<content_hash>`.
    pub fn ref_string(&self, content_hash: &str) -> String {
        embedding_ref_string(&self.model, self.dimension, self.metric, content_hash)
    }

    /// C3-5-embref: the pack-level *profile* identity `emb1:<model>:<dimension>:<metric>`
    /// (the first four fields of the per-cell `ref_string`, without the per-cell
    /// `content_hash`). This is what a verifier must re-execute a hybrid/semantic
    /// query under to reproduce the candidate set; it enters the receipt
    /// determinism surface additively (see ADR-embedding-ref-receipt-visibility).
    pub fn profile_ref_string(&self) -> String {
        let safe_model = self.model.replace(':', "_");
        format!("emb1:{safe_model}:{}:{}", self.dimension, self.metric_str())
    }
}

/// Maps a [`DistanceMetric`](crate::search::DistanceMetric) discriminant
/// (0=dot, 1=cosine, 2=l2) to a stable lowercase name; unknown values map to
/// `"unknown"`.
pub fn metric_str(metric: u32) -> &'static str {
    match metric {
        0 => "dot_product",
        1 => "cosine",
        2 => "l2",
        _ => "unknown",
    }
}

/// Builds the per-cell `embedding_ref` value
/// `emb1:<model>:<dimension>:<metric>:<content_hash>`. Any `:` in the model is
/// replaced with `_` so the delimiter stays unambiguous; an empty model marks an
/// unlabelled/legacy embedding.
pub fn embedding_ref_string(
    model: &str,
    dimension: u32,
    metric: u32,
    content_hash: &str,
) -> String {
    let safe_model = model.replace(':', "_");
    format!(
        "emb1:{safe_model}:{dimension}:{}:{content_hash}",
        metric_str(metric)
    )
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
    pub batch_size: usize,
    pub embedding_batches: usize,
    pub write_batches: usize,
    pub max_batch_items: usize,
    pub final_debt_items: usize,
    pub failure_samples: Vec<String>,
}

pub trait EmbeddingBackfillProvider {
    fn embed_text(&mut self, text: &str) -> EngineResult<Vec<i16>>;

    fn embed_text_batch(&mut self, texts: &[String]) -> EngineResult<Vec<Vec<i16>>> {
        texts.iter().map(|text| self.embed_text(text)).collect()
    }
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
