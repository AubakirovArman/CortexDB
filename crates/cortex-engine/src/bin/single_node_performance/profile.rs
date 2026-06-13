use std::collections::BTreeSet;
use std::fs;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, ContextPackOptions, Database, DatabaseOptions, SearchLimit};
use cortex_storage::wal::DurabilityMode;
use serde_json::{json, Value};

use super::args::Args;
use super::latency::{check_phase_thresholds, measure_repeated, single_node_latency_thresholds};

pub(super) fn run_profile(
    label: &str,
    durability_mode: DurabilityMode,
    args: &Args,
    root: &std::path::Path,
) -> Result<Value, String> {
    let cells = args.cells;
    let db_path = root.join(format!("{label}-{}", unique_id()));
    let options = DatabaseOptions {
        durability_mode,
        ..DatabaseOptions::default()
    };
    let mut phases = Vec::new();
    let mut errors = Vec::new();
    let view = perf_view();
    let query = r#"RETRIEVE CONTEXT FOR TASK "performance budget" IN BRAIN default WHERE space = perf AND status = "ready" LIMIT 10 CANDIDATES;"#;

    let (mut db, phase) = measure("open_empty", 1, || {
        Database::open_with_options(&db_path, options)
    })?;
    phases.push(phase);

    let payloads = (1..=cells)
        .map(|index| (CellId(index as u64), payload(index)))
        .collect::<Vec<_>>();
    let (_, phase) = measure("put_batch", cells, || db.put_cells(payloads))?;
    let ingest_throughput = phase["throughput_per_sec"].as_f64().unwrap_or(0.0);
    if ingest_throughput < args.min_ingest_cells_per_sec {
        errors.push(format!(
            "{label} ingest throughput below threshold: {ingest_throughput:.3} < {:.3}",
            args.min_ingest_cells_per_sec
        ));
    }
    phases.push(phase);

    let write_samples = 10;
    let (_, phase) = measure_repeated("put_single", write_samples, |offset| {
        let index = cells + offset + 1;
        db.put_cell(CellId(index as u64), payload(index))
            .map(|_| ())
    })?;
    phases.push(phase);
    let total_cells = cells + write_samples;

    let (_, phase) = measure_repeated("get_latest", cells, |offset| {
        let index = offset + 1;
        let payload = db
            .get_latest_cell(CellId(index as u64))
            .ok_or_else(|| format!("missing cell after put: {index}"))?;
        if payload.is_empty() {
            return Err(format!("empty payload after put: {index}"));
        }
        Ok(())
    })?;
    phases.push(phase);

    let (_, phase) = measure_repeated("keyword_search", 25, |_| {
        let results = db
            .search_keyword("budget ready", &view, SearchLimit(10))
            .map_err(|error| error.to_string())?;
        if results.is_empty() {
            return Err("keyword search returned no results".to_owned());
        }
        Ok(())
    })?;
    phases.push(phase);

    let (_, phase) = measure_repeated("context_pack", 10, |_| {
        let pack = db
            .context_pack_from_aql(query, &view, ContextPackOptions::default())
            .map_err(|error| error.to_string())?;
        if pack.cells.is_empty() {
            return Err("ContextPack returned no cells".to_owned());
        }
        Ok(())
    })?;
    phases.push(phase);

    let verify_query = r#"VERIFY FACT "budget ready" IN BRAIN default;"#;
    let (_, phase) = measure_repeated("verify_fact", 10, |_| {
        let report = db
            .verify_fact_aql(verify_query, &view)
            .map_err(|error| error.to_string())?;
        if report.evidence.is_empty() {
            return Err("VerifyFact returned no evidence".to_owned());
        }
        Ok(())
    })?;
    phases.push(phase);

    if let Err(error) = check_phase_thresholds(&phases) {
        errors.push(format!("{label} {error}"));
    }

    let (_, phase) = measure("checkpoint", total_cells, || db.checkpoint())?;
    phases.push(phase);

    let (_, phase) = measure("compact", total_cells, || db.compact())?;
    phases.push(phase);

    let (_, phase) = measure("close", 1, || db.close())?;
    phases.push(phase);

    let (db, phase) = measure("restart_open", total_cells, || {
        Database::open_with_options(&db_path, options)
    })?;
    phases.push(phase);

    let validation = db.validate_storage_report();
    if !validation.errors.is_empty() {
        return Err(format!(
            "{label} validation failed: {}",
            validation.errors.join("; ")
        ));
    }
    let stats = db
        .storage_stats()
        .map_err(|error| format!("{label} stats failed: {error}"))?;
    db.close()
        .map_err(|error| format!("{label} final close failed: {error}"))?;

    let resource_usage = process_memory_report();
    let peak_rss_bytes = resource_usage["peak_rss_bytes"].as_u64().unwrap_or(0);
    if peak_rss_bytes > args.max_rss_bytes {
        errors.push(format!(
            "{label} peak RSS exceeded threshold: {peak_rss_bytes} > {}",
            args.max_rss_bytes
        ));
    }

    Ok(json!({
        "name": label,
        "durability_mode": format!("{durability_mode:?}").to_lowercase(),
        "latency_thresholds": single_node_latency_thresholds(),
        "slo_thresholds": {
            "min_ingest_cells_per_sec": args.min_ingest_cells_per_sec,
            "max_rss_bytes": args.max_rss_bytes,
        },
        "slo": {
            "passed": errors.is_empty(),
            "errors": errors,
        },
        "ingest": {
            "cells": cells,
            "throughput_per_sec": round_ms(ingest_throughput),
            "min_throughput_per_sec": args.min_ingest_cells_per_sec,
        },
        "resource_usage": resource_usage,
        "phases": phases,
        "validation": {
            "manifest_ok": validation.manifest_ok,
            "wal_ok": validation.wal_ok,
            "live_segments_checked": validation.live_segments_checked,
            "cells_checked": validation.cells_checked,
        },
        "stats": {
            "current_seq": stats.current_seq.0,
            "checkpoint_seq": stats.checkpoint_seq.0,
            "live_segments": stats.live_segments,
            "wal_size_bytes": stats.wal_size_bytes,
            "durable_storage_bytes": stats.durable_storage_bytes,
            "live_segment_bytes": stats.live_segment_bytes,
            "total_segment_bytes": stats.total_segment_bytes,
        }
    }))
}

pub(super) fn collect_profile_errors(profile: &Value, errors: &mut Vec<String>) {
    for error in profile["slo"]["errors"].as_array().into_iter().flatten() {
        if let Some(text) = error.as_str() {
            errors.push(text.to_owned());
        }
    }
}

fn measure<T, E, F>(name: &str, units: usize, call: F) -> Result<(T, Value), String>
where
    E: std::fmt::Display,
    F: FnOnce() -> Result<T, E>,
{
    let start = Instant::now();
    let value = call().map_err(|error| error.to_string())?;
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let throughput = if elapsed_ms > 0.0 {
        (units as f64) / (elapsed_ms / 1000.0)
    } else {
        0.0
    };
    Ok((
        value,
        json!({
            "name": name,
            "units": units,
            "elapsed_ms": round_ms(elapsed_ms),
            "throughput_per_sec": round_ms(throughput),
        }),
    ))
}

fn payload(index: usize) -> Vec<u8> {
    format!(
        "scope=perf\nstatus=ready\ntype=fact\nsource=single-node-{index}\n\nperformance budget ready cell {index}"
    )
    .into_bytes()
}

fn perf_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("single-node-performance".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("perf")]),
        writable_scopes: BTreeSet::from([scope_id("perf")]),
        allowed_modes: BTreeSet::from([RetrievalMode::Fast, RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([
            MemoryType::Decision,
            MemoryType::Preference,
            MemoryType::WorkflowResult,
            MemoryType::ErrorLog,
            MemoryType::Observation,
        ]),
        max_context_budget_tokens: 10_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 1_000,
        default_candidate_limit: 10,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: Some(ScopeId(999)),
    }
}

pub(super) fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn unique_id() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn process_memory_report() -> Value {
    let (rss_bytes, peak_rss_bytes) = linux_proc_status_memory_bytes().unwrap_or((0, 0));
    json!({
        "rss_bytes": rss_bytes,
        "peak_rss_bytes": peak_rss_bytes,
    })
}

fn linux_proc_status_memory_bytes() -> Option<(u64, u64)> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let mut rss_bytes = 0;
    let mut peak_rss_bytes = 0;
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            rss_bytes = parse_status_kib(value).unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("VmHWM:") {
            peak_rss_bytes = parse_status_kib(value).unwrap_or(0);
        }
    }
    Some((rss_bytes, peak_rss_bytes.max(rss_bytes)))
}

fn parse_status_kib(value: &str) -> Option<u64> {
    let kib = value.split_whitespace().next()?.parse::<u64>().ok()?;
    kib.checked_mul(1024)
}

#[cfg(test)]
mod tests {
    use super::parse_status_kib;

    #[test]
    fn parses_linux_status_kib_as_bytes() {
        assert_eq!(parse_status_kib(" 123 kB"), Some(125_952));
        assert_eq!(parse_status_kib("invalid"), None);
    }
}
