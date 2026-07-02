use std::time::Instant;

use cortex_core::CellId;
use cortex_engine::{
    Database, DatabaseOptions, EmbeddingBackfillOptions, EmbeddingBackfillProvider,
    EmbeddingCoverageConfig, EngineResult, WriteBatch, WriteBatchOperation,
};
use cortex_storage::wal::DurabilityMode;
use serde_json::{json, Value};

use super::args::Args;
use super::round_ms;

pub struct WorkloadReport {
    pub phases: Vec<Value>,
    pub errors: Vec<String>,
    pub ingest_docs_per_sec: f64,
    pub embedding_docs_per_sec: f64,
    pub throughput_docs_per_sec: f64,
    pub ingestion_write_batches: usize,
    pub embedding_batches: usize,
    pub embedding_write_batches: usize,
    pub max_embedding_batch_items: usize,
    pub resume_after_docs: usize,
    pub first_run_embedded_items: usize,
    pub first_run_final_debt_items: usize,
    pub resumed_embedded_items: usize,
    pub final_debt_items: usize,
    pub completed_after_reopen: bool,
    pub ready_items: usize,
    pub expected_model: String,
    pub expected_dimension: usize,
    pub validation: Value,
    pub storage: Value,
}

pub fn run_workload(args: &Args) -> Result<WorkloadReport, String> {
    let expected_model = "synthetic-c19";
    let expected_dimension = 8usize;
    let config = EmbeddingCoverageConfig {
        expected_dimension: Some(expected_dimension),
        min_coverage_basis_points: 10_000,
        expected_model: Some(expected_model.to_owned()),
    };
    let db_path = args.root.join("db");
    let total_started = Instant::now();
    let mut phases = Vec::new();
    let db_options = DatabaseOptions {
        durability_mode: DurabilityMode::Balanced,
        ..DatabaseOptions::default()
    };
    let mut db = Database::open_with_options(&db_path, db_options.clone())
        .map_err(|error| error.to_string())?;

    let (ingestion_write_batches, ingest_ms) = ingest_docs(&mut db, args, &mut phases)?;
    let resume_after_docs = args
        .resume_after_docs
        .unwrap_or_else(|| (args.docs / 2).max(1))
        .min(args.docs);
    let mut provider = SyntheticEmbeddingProvider::new(expected_dimension);
    let first_started = Instant::now();
    let first = db
        .backfill_embedding_debt_batched(
            &mut provider,
            EmbeddingBackfillOptions {
                config: config.clone(),
                max_items: Some(resume_after_docs),
            },
            args.embedding_batch_size,
        )
        .map_err(|error| error.to_string())?;
    let first_ms = elapsed_ms(first_started);
    phases.push(phase(
        "embedding_backfill_interrupted",
        first.embedded_items,
        first_ms,
    ));

    db.close().map_err(|error| error.to_string())?;
    let reopen_started = Instant::now();
    let mut db = Database::open_with_options(&db_path, db_options.clone())
        .map_err(|error| error.to_string())?;
    phases.push(phase("reopen_for_resume", 1, elapsed_ms(reopen_started)));

    let second_started = Instant::now();
    let second = db
        .backfill_embedding_debt_batched(
            &mut provider,
            EmbeddingBackfillOptions {
                config: config.clone(),
                max_items: None,
            },
            args.embedding_batch_size,
        )
        .map_err(|error| error.to_string())?;
    let second_ms = elapsed_ms(second_started);
    phases.push(phase(
        "embedding_backfill_resume",
        second.embedded_items,
        second_ms,
    ));

    let debt = db.embedding_debt_report(config);
    let validation_report = db.validate_storage_report();
    let stats = db.storage_stats().map_err(|error| error.to_string())?;
    db.close().map_err(|error| error.to_string())?;

    let embedding_ms = first_ms + second_ms;
    let total_ms = elapsed_ms(total_started);
    let mut errors = validation_report.errors.clone();
    if debt.debt_items != 0 {
        errors.push(format!("embedding debt remains: {}", debt.debt_items));
    }
    if first.final_debt_items == 0 && args.docs > 1 {
        errors.push("resume scenario did not leave partial embedding debt".to_owned());
    }

    Ok(WorkloadReport {
        phases,
        errors,
        ingest_docs_per_sec: throughput(args.docs, ingest_ms),
        embedding_docs_per_sec: throughput(
            first.embedded_items + second.embedded_items,
            embedding_ms,
        ),
        throughput_docs_per_sec: throughput(args.docs, total_ms),
        ingestion_write_batches,
        embedding_batches: first.embedding_batches + second.embedding_batches,
        embedding_write_batches: first.write_batches + second.write_batches,
        max_embedding_batch_items: first.max_batch_items.max(second.max_batch_items),
        resume_after_docs,
        first_run_embedded_items: first.embedded_items,
        first_run_final_debt_items: first.final_debt_items,
        resumed_embedded_items: second.embedded_items,
        final_debt_items: debt.debt_items,
        completed_after_reopen: debt.debt_items == 0 && second.embedded_items > 0,
        ready_items: debt.ready_items,
        expected_model: expected_model.to_owned(),
        expected_dimension,
        validation: json!({
            "manifest_ok": validation_report.manifest_ok,
            "wal_ok": validation_report.wal_ok,
            "cells_checked": validation_report.cells_checked,
            "wal_records_checked": validation_report.wal_records_checked,
            "issues": validation_report.issues.len(),
        }),
        storage: json!({
            "durability_mode": "balanced",
            "current_seq": stats.current_seq.0,
            "wal_records_written": stats.wal_writer.records_written,
            "wal_batches_committed": stats.wal_writer.batches_committed,
            "durable_storage_bytes": stats.durable_storage_bytes,
        }),
    })
}

fn ingest_docs(
    db: &mut Database,
    args: &Args,
    phases: &mut Vec<Value>,
) -> Result<(usize, f64), String> {
    let started = Instant::now();
    let mut write_batches = 0usize;
    let mut next = 1usize;
    while next <= args.docs {
        let end = (next + args.ingestion_batch_size - 1).min(args.docs);
        let operations = (next..=end)
            .map(|index| WriteBatchOperation::PutCell {
                cell_id: CellId(index as u64),
                payload: payload(index, args.payload_bytes),
            })
            .collect::<Vec<_>>();
        db.write_batch(WriteBatch::from_operations(operations))
            .map_err(|error| error.to_string())?;
        write_batches += 1;
        next = end + 1;
    }
    let elapsed = elapsed_ms(started);
    phases.push(phase("ingest_write_batch", args.docs, elapsed));
    Ok((write_batches, elapsed))
}

fn payload(index: usize, payload_bytes: usize) -> Vec<u8> {
    let header = format!("scope=ingest\nstatus=ready\ntype=document\nsource=c19-{index}\n\n");
    let target_body = payload_bytes.saturating_sub(header.len()).max(32);
    let repeated = format!("C19 synthetic document {index} embedding throughput. ");
    let mut body = String::with_capacity(target_body);
    while body.len() < target_body {
        body.push_str(&repeated);
    }
    body.truncate(target_body);
    (header + &body).into_bytes()
}

fn phase(name: &str, units: usize, elapsed_ms: f64) -> Value {
    json!({
        "name": name,
        "units": units,
        "elapsed_ms": round_ms(elapsed_ms),
        "throughput_per_sec": round_ms(throughput(units, elapsed_ms)),
    })
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn throughput(units: usize, elapsed_ms: f64) -> f64 {
    if elapsed_ms > 0.0 {
        units as f64 / (elapsed_ms / 1000.0)
    } else {
        0.0
    }
}

struct SyntheticEmbeddingProvider {
    dimension: usize,
}

impl SyntheticEmbeddingProvider {
    fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl EmbeddingBackfillProvider for SyntheticEmbeddingProvider {
    fn embed_text(&mut self, text: &str) -> EngineResult<Vec<i16>> {
        Ok(vector_for_text(text, self.dimension))
    }

    fn embed_text_batch(&mut self, texts: &[String]) -> EngineResult<Vec<Vec<i16>>> {
        Ok(texts
            .iter()
            .map(|text| vector_for_text(text, self.dimension))
            .collect())
    }
}

fn vector_for_text(text: &str, dimension: usize) -> Vec<i16> {
    let mut hash = 17u64;
    for byte in text.as_bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(u64::from(*byte));
    }
    (0..dimension)
        .map(|offset| {
            let shifted = hash.rotate_left((offset % 31) as u32);
            ((shifted % 2001) as i16) - 1000
        })
        .collect()
}
