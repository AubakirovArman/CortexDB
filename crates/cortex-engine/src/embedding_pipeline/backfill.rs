use crate::database::Database;
use crate::error::EngineResult;
use crate::ingestion::stable_ingestion_hash_hex;

use super::debt::{
    embedding_debt_reason, embedding_debt_report_from_versions,
    embedding_expected_items_from_versions,
};
use super::payload::{embedding_text_for_hash, payload_with_embedding};
use super::types::{
    EmbeddingBackfillOptions, EmbeddingBackfillProvider, EmbeddingBackfillReport,
    EmbeddingCoverageConfig, EmbeddingDebtReport, EmbeddingExpectedItem,
};

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

fn push_failure_sample(samples: &mut Vec<String>, value: String) {
    if samples.len() < super::types::SAMPLE_LIMIT {
        samples.push(value);
    }
}
