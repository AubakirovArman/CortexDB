use crate::database::Database;
use crate::error::EngineResult;
use crate::ingestion::stable_ingestion_hash_hex;
use crate::operation::WriteBatch;

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
        self.backfill_embedding_debt_batched(provider, options, 1)
    }

    pub fn backfill_embedding_debt_batched<P: EmbeddingBackfillProvider>(
        &mut self,
        provider: &mut P,
        options: EmbeddingBackfillOptions,
        batch_size: usize,
    ) -> EngineResult<EmbeddingBackfillReport> {
        let batch_size = batch_size.max(1);
        let versions = self.snapshot_versions();
        let mut report = EmbeddingBackfillReport {
            schema_version: "cortexdb.embedding_pipeline.backfill.v1".to_owned(),
            scanned_items: versions.len(),
            batch_size,
            ..EmbeddingBackfillReport::default()
        };
        let mut processed = 0usize;
        let mut batch = Vec::with_capacity(batch_size);

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
            batch.push(EmbeddingBackfillWorkItem {
                cell_id: version.cell_id,
                payload: version.payload.clone(),
                reason,
                text,
                text_hash,
            });
            if batch.len() == batch_size {
                process_embedding_batch(self, provider, &options, &mut report, &mut batch)?;
            }
        }

        process_embedding_batch(self, provider, &options, &mut report, &mut batch)?;
        report.final_debt_items = self.embedding_debt_report(options.config).debt_items;
        Ok(report)
    }
}

struct EmbeddingBackfillWorkItem {
    cell_id: cortex_core::CellId,
    payload: Vec<u8>,
    reason: &'static str,
    text: String,
    text_hash: String,
}

fn process_embedding_batch<P: EmbeddingBackfillProvider>(
    db: &mut Database,
    provider: &mut P,
    options: &EmbeddingBackfillOptions,
    report: &mut EmbeddingBackfillReport,
    batch: &mut Vec<EmbeddingBackfillWorkItem>,
) -> EngineResult<()> {
    if batch.is_empty() {
        return Ok(());
    }

    report.embedding_batches += 1;
    report.max_batch_items = report.max_batch_items.max(batch.len());
    let texts = batch
        .iter()
        .map(|item| item.text.clone())
        .collect::<Vec<_>>();
    let vectors = match provider.embed_text_batch(&texts) {
        Ok(vectors) => vectors,
        Err(error) => {
            report.failed_items += batch.len();
            for item in batch.iter() {
                push_failure_sample(
                    &mut report.failure_samples,
                    format!("cell {} {}: {error}", item.cell_id.0, item.reason),
                );
            }
            batch.clear();
            return Ok(());
        }
    };
    if vectors.len() != batch.len() {
        report.failed_items += batch.len();
        push_failure_sample(
            &mut report.failure_samples,
            format!(
                "provider returned {} vectors for {} texts",
                vectors.len(),
                batch.len()
            ),
        );
        batch.clear();
        return Ok(());
    }

    let mut write_batch = WriteBatch::new();
    let mut valid_items = 0usize;
    for (item, vector) in batch.iter().zip(vectors.iter()) {
        if vector.is_empty() {
            report.failed_items += 1;
            push_failure_sample(
                &mut report.failure_samples,
                format!(
                    "cell {} {}: provider returned empty vector",
                    item.cell_id.0, item.reason
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
                        "cell {} {}: provider returned dimension {}, expected {expected}",
                        item.cell_id.0,
                        item.reason,
                        vector.len()
                    ),
                );
                continue;
            }
        }
        let payload = payload_with_embedding(
            &item.payload,
            options.config.expected_model.as_deref(),
            &item.text_hash,
            vector,
        );
        write_batch = write_batch.patch_cell(item.cell_id, payload);
        valid_items += 1;
    }

    if valid_items > 0 {
        db.write_batch(write_batch)?;
        report.write_batches += 1;
        report.embedded_items += valid_items;
    }
    batch.clear();
    Ok(())
}

fn push_failure_sample(samples: &mut Vec<String>, value: String) {
    if samples.len() < super::types::SAMPLE_LIMIT {
        samples.push(value);
    }
}
