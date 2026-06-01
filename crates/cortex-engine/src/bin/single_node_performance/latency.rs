use std::time::Instant;

use serde_json::{json, Value};

pub fn measure_repeated<E, F>(name: &str, units: usize, mut call: F) -> Result<((), Value), String>
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
    let throughput = if elapsed_ms > 0.0 {
        (units as f64) / (elapsed_ms / 1000.0)
    } else {
        0.0
    };
    Ok((
        (),
        json!({
            "name": name,
            "units": units,
            "elapsed_ms": round_ms(elapsed_ms),
            "throughput_per_sec": round_ms(throughput),
            "latency": latency_summary(&latencies),
        }),
    ))
}

pub fn single_node_latency_thresholds() -> Value {
    json!({
        "put_single": {"p95_ms": 1000.0, "p99_ms": 2000.0},
        "get_latest": {"p95_ms": 100.0, "p99_ms": 250.0},
        "keyword_search": {"p95_ms": 1500.0, "p99_ms": 3000.0},
        "context_pack": {"p95_ms": 1500.0, "p99_ms": 3000.0},
        "verify_fact": {"p95_ms": 1500.0, "p99_ms": 3000.0},
    })
}

pub fn check_phase_thresholds(phases: &[Value]) -> Result<(), String> {
    let thresholds = single_node_latency_thresholds();
    for phase in phases {
        let Some(name) = phase["name"].as_str() else {
            continue;
        };
        let Some(threshold) = thresholds.get(name) else {
            continue;
        };
        let Some(latency) = phase.get("latency") else {
            continue;
        };
        for metric in ["p95_ms", "p99_ms"] {
            let observed = latency[metric].as_f64().unwrap_or(0.0);
            let allowed = threshold[metric].as_f64().unwrap_or(f64::MAX);
            if observed > allowed {
                return Err(format!(
                    "{name} {metric} exceeded threshold: {observed:.3} > {allowed:.3}"
                ));
            }
        }
    }
    Ok(())
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

fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}
