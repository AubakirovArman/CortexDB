use cortex_core::CellId;
use cortex_engine::{parse_vector_literal, ContextPackOptions, Database, SearchLimit};

use crate::cli_json::{context_pack_to_json, verification_report_to_json};
use crate::context::{
    format_context_pack, format_retrieved_cells, format_search_results, format_verification_report,
    remember_view_for_scope, verify_view_for_scope, view_for_scope,
};
use crate::{manifest, wal};

pub fn run_demo() -> Result<String, String> {
    let output = std::process::Command::new("./examples/demo/investment_projects/run.sh")
        .output()
        .map_err(|e| format!("Failed to run demo script: {e}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr))
}

pub fn put(path: &str, cell_id: &str, payload: &str) -> Result<String, String> {
    let cell_id = parse_cell_id(cell_id)?;
    let mut db = Database::open(path).map_err(|error| error.to_string())?;
    let seq = db
        .put_cell(cell_id, payload.as_bytes().to_vec())
        .map_err(|error| error.to_string())?;
    Ok(format!("seq={}", seq.0))
}

pub fn get(path: &str, cell_id: &str) -> Result<String, String> {
    let cell_id = parse_cell_id(cell_id)?;
    let db = Database::open(path).map_err(|error| error.to_string())?;
    Ok(db
        .get_latest_cell(cell_id)
        .map(|payload| String::from_utf8_lossy(&payload).into_owned())
        .unwrap_or_else(|| "null".to_owned()))
}

pub fn tombstone(path: &str, cell_id: &str) -> Result<String, String> {
    let cell_id = parse_cell_id(cell_id)?;
    let mut db = Database::open(path).map_err(|error| error.to_string())?;
    let seq = db
        .tombstone_cell(cell_id)
        .map_err(|error| error.to_string())?;
    Ok(format!("seq={}", seq.0))
}

pub fn flush(path: &str) -> Result<String, String> {
    let mut db = Database::open(path).map_err(|error| error.to_string())?;
    let stats = db.checkpoint().map_err(|error| error.to_string())?;
    Ok(format!(
        "checkpoint_seq={} cells_flushed={}",
        stats.checkpoint_seq.0, stats.cells_flushed
    ))
}

pub fn compact(path: &str) -> Result<String, String> {
    let mut db = Database::open(path).map_err(|error| error.to_string())?;
    let stats = db.compact().map_err(|error| error.to_string())?;
    Ok(format!(
        "checkpoint_seq={} cells_flushed={}",
        stats.checkpoint_seq.0, stats.cells_flushed
    ))
}

pub fn stats(path: &str, json: bool) -> Result<String, String> {
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let stats = db.storage_stats().map_err(|error| error.to_string())?;
    if json {
        return Ok(serde_json::json!({
            "current_seq": stats.current_seq.0,
            "checkpoint_seq": stats.checkpoint_seq.0,
            "live_segments": stats.live_segments,
            "retired_segments": stats.retired_segments,
            "memtable_cells": stats.memtable.cell_count,
            "memtable_versions": stats.memtable.version_count,
            "wal_size_bytes": stats.wal_size_bytes,
            "wal_writer_records": stats.wal_writer.records_written,
            "wal_writer_bytes": stats.wal_writer.bytes_written,
            "wal_writer_fsyncs": stats.wal_writer.fsync_count,
            "wal_writer_batches": stats.wal_writer.batches_committed,
        })
        .to_string());
    }
    Ok(format!(
        "current_seq={} checkpoint_seq={} live_segments={} retired_segments={} memtable_cells={} memtable_versions={} wal_size_bytes={} wal_writer_records={} wal_writer_bytes={} wal_writer_fsyncs={} wal_writer_batches={}",
        stats.current_seq.0,
        stats.checkpoint_seq.0,
        stats.live_segments,
        stats.retired_segments,
        stats.memtable.cell_count,
        stats.memtable.version_count,
        stats.wal_size_bytes,
        stats.wal_writer.records_written,
        stats.wal_writer.bytes_written,
        stats.wal_writer.fsync_count,
        stats.wal_writer.batches_committed
    ))
}

pub fn validate(path: &str, json: bool) -> Result<String, String> {
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let validation = db.validate_storage().map_err(|error| error.to_string())?;
    if json {
        return Ok(serde_json::json!({
            "ok": true,
            "live_segments_checked": validation.live_segments_checked,
            "cells_checked": validation.cells_checked,
            "wal_records_checked": validation.wal_records_checked,
            "wal_safe_truncate_offset": validation.wal_safe_truncate_offset,
        })
        .to_string());
    }
    Ok(format!(
        "ok live_segments_checked={} cells_checked={} wal_records_checked={} wal_safe_truncate_offset={}",
        validation.live_segments_checked,
        validation.cells_checked,
        validation.wal_records_checked,
        validation.wal_safe_truncate_offset
    ))
}

pub fn repair(path: &str) -> Result<String, String> {
    let report = Database::repair_best_effort(path).map_err(|error| error.to_string())?;
    Ok(format!(
        "orphan_temp_files_removed={} wal_records_preserved={} wal_safe_truncate_offset={} wal_bytes_before={} wal_bytes_after={} wal_truncated={}",
        report.orphan_temp_files_removed,
        report.wal_records_preserved,
        report.wal_safe_truncate_offset,
        report.wal_bytes_before,
        report.wal_bytes_after,
        report.wal_truncated
    ))
}

pub fn gc_retired(path: &str) -> Result<String, String> {
    let mut db = Database::open(path).map_err(|error| error.to_string())?;
    let report = db
        .garbage_collect_retired_segments()
        .map_err(|error| error.to_string())?;
    Ok(format!(
        "retired_segments_removed={} files_removed={}",
        report.retired_segments_removed, report.files_removed
    ))
}

pub fn context(path: &str, scope: &str, aql: &str, json: bool) -> Result<String, String> {
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let pack = db
        .context_pack_from_aql(aql, &view_for_scope(scope), ContextPackOptions::default())
        .map_err(|error| error.to_string())?;
    if json {
        Ok(context_pack_to_json(&pack))
    } else {
        Ok(format_context_pack(&pack))
    }
}

pub fn remember(path: &str, scope: &str, aql: &str) -> Result<String, String> {
    let mut db = Database::open(path).map_err(|error| error.to_string())?;
    let result = db
        .remember_aql(aql, &remember_view_for_scope(scope))
        .map_err(|error| error.to_string())?;
    Ok(format!(
        "seq={} cell_id={} ttl_seconds={}",
        result.commit_seq.0,
        result.cell_id.0,
        result
            .ttl_seconds
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_owned())
    ))
}

pub fn verify(path: &str, scope: &str, aql: &str, json: bool) -> Result<String, String> {
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let report = db
        .verify_fact_aql(aql, &verify_view_for_scope(scope))
        .map_err(|error| error.to_string())?;
    if json {
        Ok(verification_report_to_json(&report, &db))
    } else {
        Ok(format_verification_report(&report))
    }
}

pub fn aql(path: &str, scope: &str, aql: &str) -> Result<String, String> {
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let cells = db
        .retrieve_aql(aql, &view_for_scope(scope))
        .map_err(|error| error.to_string())?;
    Ok(format_retrieved_cells(&cells))
}

pub fn search(path: &str, scope: &str, query: &str) -> Result<String, String> {
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let results = db
        .search_keyword(query, &view_for_scope(scope), SearchLimit(20))
        .map_err(|error| error.to_string())?;
    Ok(format_search_results(&results))
}

pub fn search_vector(path: &str, scope: &str, vector: &str, exact: bool) -> Result<String, String> {
    let vector = parse_vector_literal(vector)?;
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let view = view_for_scope(scope);
    let results = if exact {
        db.search_vector_exact(&vector, &view, SearchLimit(20))
    } else {
        db.search_vector(&vector, &view, SearchLimit(20))
    }
    .map_err(|error| error.to_string())?;
    Ok(format_search_results(&results))
}

pub fn unlock(path: &str, force: bool) -> Result<String, String> {
    if !force {
        return Err("unlock requires --force".to_owned());
    }
    Database::break_stale_lock(path).map_err(|error| error.to_string())?;
    Ok("stale lock removed".to_owned())
}

pub fn wal_validate(path: &str) -> Result<String, String> {
    wal::validate(path)
}

pub fn wal_dump(path: &str) -> Result<String, String> {
    wal::dump(path)
}

pub fn wal_truncate(path: &str) -> Result<String, String> {
    wal::truncate(path)
}

pub fn manifest_dump(path: &str) -> Result<String, String> {
    manifest::dump(path)
}

pub fn manifest_validate(path: &str) -> Result<String, String> {
    manifest::validate(path)
}

fn parse_cell_id(value: &str) -> Result<CellId, String> {
    value
        .parse::<u64>()
        .map(CellId)
        .map_err(|_| "cell_id must be u64".to_owned())
}
