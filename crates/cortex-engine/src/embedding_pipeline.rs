use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use cortex_core::memtable::CellVersion;
use serde::{Deserialize, Serialize};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::ingestion::stable_ingestion_hash_hex;
use crate::search::vector::vector_from_payload;

pub const EMBEDDING_PIPELINE_REPORT_SCHEMA: &str = "cortexdb.embedding_pipeline.coverage.v1";
pub const DEFAULT_MIN_EMBEDDING_COVERAGE_BPS: u32 = 9_950;
const SAMPLE_LIMIT: usize = 25;

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

impl<F> EmbeddingBackfillProvider for F
where
    F: FnMut(&str) -> EngineResult<Vec<i16>>,
{
    fn embed_text(&mut self, text: &str) -> EngineResult<Vec<i16>> {
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

impl Database {
    pub fn embedding_expected_manifest(&self) -> Vec<EmbeddingExpectedItem> {
        embedding_expected_items_from_versions(&self.snapshot_versions())
    }

    pub fn embedding_debt_report(&self, config: EmbeddingCoverageConfig) -> EmbeddingDebtReport {
        embedding_debt_report_from_versions(&self.snapshot_versions(), config)
    }

    pub fn backfill_embedding_debt<P: EmbeddingBackfillProvider>(
        &mut self,
        provider: &mut P,
        options: EmbeddingBackfillOptions,
    ) -> EngineResult<EmbeddingBackfillReport> {
        let versions = self.snapshot_versions();
        let mut report = EmbeddingBackfillReport {
            schema_version: "cortexdb.embedding_pipeline.backfill.v1".to_owned(),
            scanned_items: versions.len(),
            ..EmbeddingBackfillReport::default()
        };
        let mut processed = 0usize;

        for version in versions
            .iter()
            .filter(|version| version.deleted_seq.is_none())
        {
            let Some(reason) = embedding_debt_reason(version, &options.config) else {
                continue;
            };
            report.debt_items += 1;
            if options.max_items.is_some_and(|max| processed >= max) {
                report.skipped_items += 1;
                continue;
            }
            processed += 1;

            let text = embedding_text_for_hash(&version.payload);
            let text_hash = stable_ingestion_hash_hex(text.as_bytes());
            let vector = match provider.embed_text(&text) {
                Ok(vector) => vector,
                Err(error) => {
                    report.failed_items += 1;
                    push_failure_sample(
                        &mut report.failure_samples,
                        format!("cell {} {reason}: {error}", version.cell_id.0),
                    );
                    continue;
                }
            };
            if vector.is_empty() {
                report.failed_items += 1;
                push_failure_sample(
                    &mut report.failure_samples,
                    format!(
                        "cell {} {reason}: provider returned empty vector",
                        version.cell_id.0
                    ),
                );
                continue;
            }
            if let Some(expected) = options.config.expected_dimension {
                if vector.len() != expected {
                    report.failed_items += 1;
                    push_failure_sample(
                        &mut report.failure_samples,
                        format!(
                            "cell {} {reason}: provider returned dimension {}, expected {expected}",
                            version.cell_id.0,
                            vector.len()
                        ),
                    );
                    continue;
                }
            }

            let payload = payload_with_embedding(
                &version.payload,
                options.config.expected_model.as_deref(),
                &text_hash,
                &vector,
            );
            self.patch_cell(version.cell_id, payload)?;
            report.embedded_items += 1;
        }

        report.final_debt_items = self.embedding_debt_report(options.config).debt_items;
        Ok(report)
    }
}

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

fn embedding_debt_reason(
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

fn push_debt_sample(samples: &mut Vec<EmbeddingDebtItem>, value: EmbeddingDebtItem) {
    if samples.len() < SAMPLE_LIMIT {
        samples.push(value);
    }
}

fn push_failure_sample(samples: &mut Vec<String>, value: String) {
    if samples.len() < SAMPLE_LIMIT {
        samples.push(value);
    }
}

pub fn embedding_text_hash(payload: &[u8]) -> String {
    stable_ingestion_hash_hex(embedding_text_for_hash(payload).as_bytes())
}

fn embedding_text_for_hash(payload: &[u8]) -> String {
    let text = String::from_utf8_lossy(payload);
    text.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !is_embedding_payload_line(trimmed)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn embedding_payload_field(payload: &[u8], field: &str) -> Option<String> {
    let prefix = format!("{field}=");
    String::from_utf8_lossy(payload)
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix).map(ToOwned::to_owned))
        .filter(|value| !value.is_empty())
}

fn payload_with_embedding(
    payload: &[u8],
    model: Option<&str>,
    text_hash: &str,
    vector: &[i16],
) -> Vec<u8> {
    let text = String::from_utf8_lossy(payload);
    let mut lines = text
        .lines()
        .filter(|line| !is_embedding_payload_line(line.trim()))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let mut embedding_lines = Vec::new();
    if let Some(model) = model {
        embedding_lines.push(format!(
            "embedding_model={}",
            sanitize_embedding_value(model)
        ));
    }
    embedding_lines.push(format!(
        "embedding_text_hash={}",
        sanitize_embedding_value(text_hash)
    ));
    embedding_lines.push(format!("vector={}", format_vector_literal(vector)));
    let insert_at = lines
        .iter()
        .position(|line| line.trim().is_empty())
        .unwrap_or(0);
    for (offset, line) in embedding_lines.into_iter().enumerate() {
        lines.insert(insert_at + offset, line);
    }
    (lines.join("\n") + "\n").into_bytes()
}

fn is_embedding_payload_line(line: &str) -> bool {
    line.starts_with("vector=")
        || line.starts_with("embedding_model=")
        || line.starts_with("embedding_text_hash=")
        || line.contains("_vector=")
}

fn format_vector_literal(vector: &[i16]) -> String {
    vector
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn sanitize_embedding_value(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_core::{CellId, CommitSeq};

    use crate::query::CellMetadata;

    struct TestEmbeddingProvider {
        vector: Vec<i16>,
        calls: Vec<String>,
    }

    impl EmbeddingBackfillProvider for TestEmbeddingProvider {
        fn embed_text(&mut self, text: &str) -> EngineResult<Vec<i16>> {
            self.calls.push(text.to_owned());
            Ok(self.vector.clone())
        }
    }

    fn default_report(jsonl: &str) -> EmbeddingCoverageReport {
        embedding_coverage_report_from_jsonl(
            ["doc-a".to_owned(), "doc-b".to_owned()],
            jsonl,
            EmbeddingCoverageConfig::default(),
        )
    }

    #[test]
    fn complete_embedding_jsonl_is_production_ready() {
        let report = default_report(
            r#"{"doc_id":"doc-a","vector":[1.0,2.0]}
{"doc_id":"doc-b","vector":[3.0,4.0]}
"#,
        );

        assert_eq!(report.coverage_basis_points, 10_000);
        assert_eq!(report.dimension, Some(2));
        assert!(report.production_ready);
    }

    #[test]
    fn missing_duplicate_unexpected_and_invalid_rows_are_reported() {
        let report = default_report(
            r#"{"doc_id":"doc-a","vector":[1.0,2.0]}
{"doc_id":"doc-a","vector":[1.0,2.0]}
{"doc_id":"doc-z","vector":[1.0,2.0]}
{"doc_id":"doc-b","vector":[]}
not-json
"#,
        );

        assert_eq!(report.embedded_items, 1);
        assert_eq!(report.missing_items, 1);
        assert_eq!(report.duplicate_items, 1);
        assert_eq!(report.unexpected_items, 1);
        assert_eq!(report.empty_vector_rows, 1);
        assert_eq!(report.invalid_rows, 1);
        assert!(!report.production_ready);
        assert_eq!(report.missing_ids_sample, vec!["doc-b"]);
        assert_eq!(report.duplicate_ids_sample, vec!["doc-a"]);
        assert_eq!(report.unexpected_ids_sample, vec!["doc-z"]);
    }

    #[test]
    fn dimension_mismatch_blocks_production_ready() {
        let report = embedding_coverage_report_from_jsonl(
            ["doc-a".to_owned()],
            r#"{"doc_id":"doc-a","vector":[1.0,2.0]}"#,
            EmbeddingCoverageConfig {
                expected_dimension: Some(3),
                min_coverage_basis_points: 10_000,
                expected_model: None,
            },
        );

        assert_eq!(report.embedded_items, 0);
        assert_eq!(report.dimension_mismatch_rows, 1);
        assert!(!report.production_ready);
    }

    #[test]
    fn production_ready_uses_configured_coverage_threshold() {
        let expected = (0..200).map(|index| format!("doc-{index:03}"));
        let jsonl = (0..199)
            .map(|index| format!(r#"{{"doc_id":"doc-{index:03}","vector":[1.0]}}"#))
            .collect::<Vec<_>>()
            .join("\n");

        let report = embedding_coverage_report_from_jsonl(
            expected,
            &jsonl,
            EmbeddingCoverageConfig::default(),
        );

        assert_eq!(report.coverage_basis_points, 9_950);
        assert_eq!(report.missing_items, 1);
        assert!(report.production_ready);
    }

    #[test]
    fn retry_ids_include_missing_and_invalid_vectors() {
        let retry_ids = embedding_retry_ids_from_jsonl(
            ["doc-a".to_owned(), "doc-b".to_owned(), "doc-c".to_owned()],
            r#"{"doc_id":"doc-a","vector":[1.0,2.0]}
{"doc_id":"doc-b","vector":[]}
"#,
            EmbeddingCoverageConfig::default(),
        );

        assert_eq!(retry_ids, vec!["doc-b", "doc-c"]);
    }

    #[test]
    fn stale_model_or_text_hash_are_retried() {
        let expected = [
            EmbeddingExpectedItem {
                doc_id: "doc-a".to_owned(),
                text_hash: Some("hash-a-v2".to_owned()),
            },
            EmbeddingExpectedItem {
                doc_id: "doc-b".to_owned(),
                text_hash: Some("hash-b".to_owned()),
            },
            EmbeddingExpectedItem {
                doc_id: "doc-c".to_owned(),
                text_hash: Some("hash-c".to_owned()),
            },
        ];
        let jsonl = r#"{"doc_id":"doc-a","vector":[1.0],"model":"bge-m3","text_hash":"hash-a-v1"}
{"doc_id":"doc-b","vector":[1.0],"model":"other-model","text_hash":"hash-b"}
{"doc_id":"doc-c","vector":[1.0],"model":"bge-m3","text_hash":"hash-c"}
"#;

        let report = embedding_coverage_report_from_expected_items(
            expected.clone(),
            jsonl,
            EmbeddingCoverageConfig {
                expected_dimension: Some(1),
                min_coverage_basis_points: 10_000,
                expected_model: Some("bge-m3".to_owned()),
            },
        );

        assert_eq!(report.embedded_items, 1);
        assert_eq!(report.stale_items, 2);
        assert_eq!(report.stale_ids_sample, vec!["doc-a", "doc-b"]);
        assert!(!report.production_ready);

        let retry_ids = embedding_retry_ids_from_expected_items(
            expected,
            jsonl,
            EmbeddingCoverageConfig {
                expected_dimension: Some(1),
                min_coverage_basis_points: 10_000,
                expected_model: Some("bge-m3".to_owned()),
            },
        );

        assert_eq!(retry_ids, vec!["doc-a", "doc-b"]);
    }

    #[test]
    fn live_versions_without_vectors_are_embedding_debt() {
        let versions = vec![
            CellVersion::new(CellId(10), CommitSeq(1), b"scope=docs\n\nAlpha".to_vec(), 0),
            CellVersion::new(CellId(11), CommitSeq(2), b"scope=docs\n\nBeta".to_vec(), 0),
        ];

        let report = embedding_debt_report_from_versions(
            &versions,
            EmbeddingCoverageConfig {
                expected_dimension: Some(3),
                min_coverage_basis_points: 10_000,
                expected_model: Some("bge-m3".to_owned()),
            },
        );

        assert_eq!(report.total_items, 2);
        assert_eq!(report.ready_items, 0);
        assert_eq!(report.missing_vector_items, 2);
        assert_eq!(report.debt_sample[0].cell_id, 10);
    }

    #[test]
    fn matching_vector_model_and_text_hash_clear_embedding_debt() {
        let base = b"scope=docs\n\nAlpha";
        let hash = embedding_text_hash(base);
        let payload = format!(
            "scope=docs\nembedding_model=bge-m3\nembedding_text_hash={hash}\nvector=1,2,3\n\nAlpha"
        )
        .into_bytes();
        let versions = vec![CellVersion::new(CellId(10), CommitSeq(1), payload, 0)];

        let report = embedding_debt_report_from_versions(
            &versions,
            EmbeddingCoverageConfig {
                expected_dimension: Some(3),
                min_coverage_basis_points: 10_000,
                expected_model: Some("bge-m3".to_owned()),
            },
        );

        assert_eq!(report.total_items, 1);
        assert_eq!(report.ready_items, 1);
        assert_eq!(report.debt_items, 0);
    }

    #[test]
    fn database_embedding_debt_report_tracks_new_put_cells() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(10), b"scope=docs\n\nAlpha".to_vec())
            .unwrap();

        let report = db.embedding_debt_report(EmbeddingCoverageConfig {
            expected_dimension: Some(3),
            min_coverage_basis_points: 10_000,
            expected_model: Some("bge-m3".to_owned()),
        });

        assert_eq!(report.total_items, 1);
        assert_eq!(report.missing_vector_items, 1);
        assert_eq!(report.debt_items, 1);
        assert_eq!(db.embedding_expected_manifest()[0].doc_id, "10");
    }

    #[test]
    fn database_backfill_embedding_debt_patches_cells_and_clears_debt() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(10), b"scope=docs\nstatus=ready\n\nAlpha".to_vec())
            .unwrap();
        let mut provider = TestEmbeddingProvider {
            vector: vec![1, 2, 3],
            calls: Vec::new(),
        };
        let config = EmbeddingCoverageConfig {
            expected_dimension: Some(3),
            min_coverage_basis_points: 10_000,
            expected_model: Some("bge-m3".to_owned()),
        };

        let report = db
            .backfill_embedding_debt(
                &mut provider,
                EmbeddingBackfillOptions {
                    config: config.clone(),
                    max_items: None,
                },
            )
            .unwrap();

        assert_eq!(report.debt_items, 1);
        assert_eq!(report.embedded_items, 1);
        assert_eq!(report.failed_items, 0);
        assert_eq!(report.final_debt_items, 0);
        assert_eq!(provider.calls, vec!["scope=docs\nstatus=ready\n\nAlpha"]);

        let payload = db.get_latest_cell(CellId(10)).unwrap();
        assert_eq!(vector_from_payload(&payload), Some(vec![1, 2, 3]));
        let metadata = CellMetadata::from_payload(&payload);
        assert_eq!(metadata.scope, "docs");
        assert_eq!(metadata.status, "ready");
        assert_eq!(metadata.body_text, "Alpha");
        assert_eq!(db.embedding_debt_report(config).debt_items, 0);
    }
}
