use std::fs;
use std::time::Instant;

use cortex_core::CellId;
use cortex_engine::Database;
use serde_json::{json, Value};

pub(crate) fn memory_phase(
    name: &str,
    db: &Database,
    skip_storage_estimates: bool,
) -> Result<Value, String> {
    let (rss_bytes, peak_rss_bytes) = linux_proc_status_memory_bytes().unwrap_or((0, 0));
    let storage_estimates = if skip_storage_estimates {
        json!({
            "skipped": true,
        })
    } else {
        let stats = db.storage_stats().map_err(|error| error.to_string())?;
        json!({
            "memtable_payload_bytes": stats.memtable_payload_bytes,
            "estimated_memtable_bytes": stats.estimated_memtable_bytes,
            "estimated_index_bytes": stats.estimated_index_bytes,
            "estimated_context_pack_bytes": stats.estimated_context_pack_bytes,
            "estimated_total_memory_bytes": stats.estimated_total_memory_bytes,
            "live_segment_bytes": stats.live_segment_bytes,
            "logical_payload_bytes": stats.logical_payload_bytes,
        })
    };
    Ok(json!({
        "name": name,
        "resource_usage": {
            "rss_bytes": rss_bytes,
            "peak_rss_bytes": peak_rss_bytes,
        },
        "storage_estimates": storage_estimates,
    }))
}

pub(crate) fn measure_once<T, E, F>(name: &str, units: usize, call: F) -> Result<(T, Value), String>
where
    E: std::fmt::Display,
    F: FnOnce() -> Result<T, E>,
{
    let started = Instant::now();
    let value = call().map_err(|error| error.to_string())?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok((
        value,
        json!({
            "name": name,
            "units": units,
            "elapsed_ms": round_ms(elapsed_ms),
            "throughput_per_sec": throughput(units, elapsed_ms),
        }),
    ))
}

pub(crate) fn measure_repeated<E, F>(name: &str, units: usize, mut call: F) -> Result<Value, String>
where
    E: std::fmt::Display,
    F: FnMut(usize) -> Result<(), E>,
{
    let started = Instant::now();
    let mut latencies = Vec::with_capacity(units);
    for offset in 0..units {
        let item_started = Instant::now();
        call(offset).map_err(|error| error.to_string())?;
        latencies.push(item_started.elapsed().as_secs_f64() * 1000.0);
    }
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(json!({
        "name": name,
        "units": units,
        "elapsed_ms": round_ms(elapsed_ms),
        "throughput_per_sec": throughput(units, elapsed_ms),
        "latency": latency_summary(&latencies),
    }))
}

pub(crate) fn matrix_from_phases(phases: &[Value]) -> Value {
    let mut matrix = serde_json::Map::new();
    for phase in phases {
        let Some(name) = phase["name"].as_str() else {
            continue;
        };
        let entry = if let Some(latency) = phase.get("latency") {
            json!({
                "p50_ms": latency["p50_ms"],
                "p95_ms": latency["p95_ms"],
                "p99_ms": latency["p99_ms"],
                "max_ms": latency["max_ms"],
            })
        } else if phase.get("elapsed_ms").is_some() {
            json!({
                "elapsed_ms": phase["elapsed_ms"],
                "throughput_per_sec": phase["throughput_per_sec"],
            })
        } else if phase.get("resource_usage").is_some() {
            json!({
                "rss_bytes": phase["resource_usage"]["rss_bytes"],
                "peak_rss_bytes": phase["resource_usage"]["peak_rss_bytes"],
                "estimated_total_memory_bytes": phase["storage_estimates"]["estimated_total_memory_bytes"],
            })
        } else {
            continue;
        };
        matrix.insert(name.to_owned(), entry);
    }
    Value::Object(matrix)
}

pub(crate) fn sampled_cell_id(cells: usize, offset: usize, samples: usize) -> CellId {
    let stride = cells.max(1) / samples.max(1);
    let index = 1 + offset.saturating_mul(stride.max(1)) % cells.max(1);
    CellId(index as u64)
}

pub(crate) fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn latency_summary(values: &[f64]) -> Value {
    json!({
        "count": values.len(),
        "p50_ms": round_ms(percentile(values, 0.50)),
        "p95_ms": round_ms(percentile(values, 0.95)),
        "p99_ms": round_ms(percentile(values, 0.99)),
        "max_ms": round_ms(values.iter().copied().fold(0.0, f64::max)),
    })
}

fn percentile(values: &[f64], percent: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(|left, right| left.total_cmp(right));
    let index = ((ordered.len() - 1) as f64 * percent).floor() as usize;
    ordered[index.min(ordered.len() - 1)]
}

fn throughput(units: usize, elapsed_ms: f64) -> f64 {
    if elapsed_ms <= 0.0 {
        return 0.0;
    }
    round_ms((units as f64) / (elapsed_ms / 1000.0))
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
