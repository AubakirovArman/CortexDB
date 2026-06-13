use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database};

use crate::cli_json::{ann_validate_to_json, stats_to_json, validation_to_json};

use super::common::{fmt_engine_error, open_database, parse_cell_id};

pub fn doctor(path: &str, tenant: Option<&str>) -> Result<String, String> {
    crate::cli_doctor::doctor(path, tenant)
}

pub fn init(path: &str) -> Result<String, String> {
    let mut db = open_database(path, false)?;
    let view = starter_agent_view();
    db.save_agent_view(&view).map_err(fmt_engine_error)?;
    let sample_cell_id = CellId(1);
    let sample_written = if db.get_latest_cell(sample_cell_id).is_none() {
        db.put_cell(sample_cell_id, starter_payload().into_bytes())
            .map_err(fmt_engine_error)?;
        true
    } else {
        false
    };
    Ok(format!(
        "CortexDB initialized\npath={path}\nagent_view_id={}\nagent_label={}\nsample_scope={STARTER_SCOPE}\nsample_cell_id={} sample_written={sample_written}\nnext:\n  cortexdb doctor {path}\n  cortexdb context {path} {STARTER_SCOPE} 'RETRIEVE CONTEXT FOR TASK \"starter onboarding\" IN BRAIN default LIMIT 5 CANDIDATES;'\n  cortexdb verify {path} {STARTER_SCOPE} 'VERIFY FACT \"CortexDB starter onboarding uses ContextPack\" IN BRAIN default;'",
        view.agent_id.0,
        view.label.as_deref().unwrap_or("starter"),
        sample_cell_id.0
    ))
}

pub fn run_demo() -> Result<String, String> {
    let output = std::process::Command::new("./examples/demo/investment_projects/run.sh")
        .output()
        .map_err(|e| format!("Failed to run demo script: {e}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr))
}

const STARTER_SCOPE: &str = "project:starter";

fn starter_agent_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("starter-cli".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(STARTER_SCOPE)]),
        writable_scopes: BTreeSet::from([scope_id(STARTER_SCOPE)]),
        allowed_modes: BTreeSet::from([RetrievalMode::Fast, RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([
            MemoryType::Decision,
            MemoryType::Observation,
            MemoryType::WorkflowResult,
        ]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(2_592_000),
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

fn starter_payload() -> String {
    [
        "scope=project:starter",
        "status=ready",
        "type=fact",
        "memory_type=decision",
        "source_trust=0.90",
        "citation=init://starter",
        "",
        "CortexDB starter onboarding uses ContextPack retrieval for scoped agent memory.",
        "Use this cell to verify init, doctor, context, and verify commands before loading real data.",
    ]
    .join("\n")
}

pub fn put(path: &str, cell_id: &str, payload: &str) -> Result<String, String> {
    let cell_id = parse_cell_id(cell_id)?;
    let mut db = open_database(path, false)?;
    let seq = db
        .put_cell(cell_id, payload.as_bytes().to_vec())
        .map_err(fmt_engine_error)?;
    Ok(format!("seq={}", seq.0))
}

pub fn get(path: &str, cell_id: &str, json: bool) -> Result<String, String> {
    let cell_id = parse_cell_id(cell_id)?;
    let db = open_database(path, false)?;
    match db.get_latest_cell(cell_id) {
        Some(payload) => {
            if json {
                Ok(crate::cli_json::cell_to_json(cell_id.0, 0, &payload))
            } else {
                Ok(String::from_utf8_lossy(&payload).into_owned())
            }
        }
        None => Ok("null".to_owned()),
    }
}

pub fn tombstone(path: &str, cell_id: &str) -> Result<String, String> {
    let cell_id = parse_cell_id(cell_id)?;
    let mut db = open_database(path, false)?;
    let seq = db.tombstone_cell(cell_id).map_err(fmt_engine_error)?;
    Ok(format!("seq={}", seq.0))
}

pub fn flush(path: &str, experimental_hnsw: bool) -> Result<String, String> {
    let mut db = open_database(path, experimental_hnsw)?;
    let stats = db.checkpoint().map_err(fmt_engine_error)?;
    Ok(format!(
        "checkpoint_seq={} cells_flushed={}",
        stats.checkpoint_seq.0, stats.cells_flushed
    ))
}

pub fn compact(path: &str, experimental_hnsw: bool) -> Result<String, String> {
    let mut db = open_database(path, experimental_hnsw)?;
    let stats = db.compact().map_err(fmt_engine_error)?;
    Ok(format!(
        "checkpoint_seq={} cells_flushed={}",
        stats.checkpoint_seq.0, stats.cells_flushed
    ))
}

pub fn stats(path: &str, json: bool) -> Result<String, String> {
    let db = open_database(path, false)?;
    let stats = db.storage_stats().map_err(fmt_engine_error)?;
    if json {
        return Ok(stats_to_json(&stats));
    }
    Ok(format!(
        "current_seq={} checkpoint_seq={} live_segments={} retired_segments={} memtable_cells={} memtable_versions={} memtable_payload_bytes={} estimated_memtable_bytes={} estimated_index_bytes={} estimated_context_pack_bytes={} estimated_total_memory_bytes={} live_segment_bytes={} retired_segment_bytes={} total_segment_bytes={} durable_storage_bytes={} live_segment_payload_bytes={} logical_payload_bytes={} space_amplification_q16={} write_amplification_q16={} compaction_pressure_q16={} wal_size_bytes={} wal_writer_records={} wal_writer_bytes={} wal_writer_fsyncs={} wal_writer_batches={}",
        stats.current_seq.0,
        stats.checkpoint_seq.0,
        stats.live_segments,
        stats.retired_segments,
        stats.memtable.cell_count,
        stats.memtable.version_count,
        stats.memtable_payload_bytes,
        stats.estimated_memtable_bytes,
        stats.estimated_index_bytes,
        stats.estimated_context_pack_bytes,
        stats.estimated_total_memory_bytes,
        stats.live_segment_bytes,
        stats.retired_segment_bytes,
        stats.total_segment_bytes,
        stats.durable_storage_bytes,
        stats.live_segment_payload_bytes,
        stats.logical_payload_bytes,
        stats.space_amplification_q16,
        stats.write_amplification_q16,
        stats.compaction_pressure_q16,
        stats.wal_size_bytes,
        stats.wal_writer.records_written,
        stats.wal_writer.bytes_written,
        stats.wal_writer.fsync_count,
        stats.wal_writer.batches_committed
    ))
}

pub fn validate(path: &str, json: bool) -> Result<String, String> {
    let db = open_database(path, false)?;
    let validation = db.validate_storage().map_err(fmt_engine_error)?;
    if json {
        return Ok(validation_to_json(
            validation.live_segments_checked,
            validation.cells_checked,
            validation
                .wal_records_checked
                .try_into()
                .unwrap_or(u64::MAX),
            validation.wal_safe_truncate_offset,
            true,
        ));
    }
    Ok(format!(
        "ok live_segments_checked={} cells_checked={} wal_records_checked={} wal_safe_truncate_offset={}",
        validation.live_segments_checked,
        validation.cells_checked,
        validation.wal_records_checked,
        validation.wal_safe_truncate_offset
    ))
}

pub fn ann_validate(path: &str, json: bool) -> Result<String, String> {
    let db = open_database(path, false)?;
    let report = db.validate_storage_report();
    let ann_errors: Vec<String> = report
        .errors
        .iter()
        .filter(|e| e.contains("vector index") || e.contains("hnsw") || e.contains("HNSW"))
        .cloned()
        .collect();
    let ok = ann_errors.is_empty();
    if json {
        return Ok(ann_validate_to_json(
            report.vector_indexes_checked,
            report.hnsw_graphs_checked,
            ann_errors,
        ));
    }
    if ok {
        Ok(format!(
            "ok vector_indexes_checked={} hnsw_graphs_checked={}",
            report.vector_indexes_checked, report.hnsw_graphs_checked
        ))
    } else {
        Err(format!(
            "ANN/HNSW validation failed: {}",
            ann_errors.join("; ")
        ))
    }
}

pub fn vector_rebuild(path: &str, experimental_hnsw: bool, json: bool) -> Result<String, String> {
    let mut db = open_database(path, experimental_hnsw)?;
    let report = db
        .rebuild_vector_indexes(experimental_hnsw)
        .map_err(fmt_engine_error)?;
    if json {
        return Ok(crate::cli_json::vector_rebuild_to_json(&report));
    }
    Ok(format!(
        "vector_rebuild segments_checked={} cells_scanned={} vector_candidates={} vector_indexes_rebuilt={} hnsw_graphs_rebuilt={} hnsw_enabled={}",
        report.segments_checked,
        report.cells_scanned,
        report.vector_candidates,
        report.vector_indexes_rebuilt,
        report.hnsw_graphs_rebuilt,
        report.hnsw_enabled
    ))
}

pub fn repair(path: &str, dry_run: bool) -> Result<String, String> {
    let report = if dry_run {
        Database::repair_best_effort_dry_run(path)
    } else {
        Database::repair_best_effort(path)
    }
    .map_err(fmt_engine_error)?;
    Ok(format!(
        "dry_run={} orphan_temp_files_removed={} wal_records_preserved={} wal_safe_truncate_offset={} wal_bytes_before={} wal_bytes_after={} wal_truncated={} wal_truncation_needed={}",
        report.dry_run,
        report.orphan_temp_files_removed,
        report.wal_records_preserved,
        report.wal_safe_truncate_offset,
        report.wal_bytes_before,
        report.wal_bytes_after,
        report.wal_truncated,
        report.wal_truncation_needed
    ))
}
