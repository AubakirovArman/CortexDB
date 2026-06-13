use std::time::Instant;

use cortex_core::CellId;
use cortex_engine::Database;
use serde_json::{json, Value};

pub(super) fn get_latest_latency_report(
    db: &Database,
    cells: usize,
    samples: usize,
) -> Result<Option<Value>, String> {
    if samples == 0 {
        return Ok(None);
    }
    if cells == 0 {
        return Err("--read-samples requires --cells to be positive".to_owned());
    }

    let mut values = Vec::with_capacity(samples);
    for offset in 0..samples {
        let cell_id = sampled_cell_id(cells, offset, samples);
        let started = Instant::now();
        let payload = db
            .get_latest_cell(cell_id)
            .ok_or_else(|| format!("missing sampled cell {}", cell_id.0))?;
        if payload.is_empty() {
            return Err(format!("empty sampled cell {}", cell_id.0));
        }
        values.push(started.elapsed().as_secs_f64() * 1000.0);
    }

    values.sort_by(|a, b| a.total_cmp(b));
    Ok(Some(json!({
        "samples": samples,
        "p50_ms": round_ms(percentile(&values, 0.50)),
        "p95_ms": round_ms(percentile(&values, 0.95)),
        "max_ms": round_ms(*values.last().unwrap_or(&0.0)),
    })))
}

fn sampled_cell_id(cells: usize, offset: usize, samples: usize) -> CellId {
    if samples <= 1 {
        return CellId(1);
    }
    let index = 1 + (offset * (cells.saturating_sub(1)) / (samples - 1));
    CellId(index as u64)
}

fn percentile(values: &[f64], quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let rank = ((values.len() - 1) as f64 * quantile).ceil() as usize;
    values[rank.min(values.len() - 1)]
}

fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampled_cell_id_spans_first_and_last() {
        assert_eq!(sampled_cell_id(10, 0, 3), CellId(1));
        assert_eq!(sampled_cell_id(10, 2, 3), CellId(10));
    }

    #[test]
    fn percentile_uses_ceil_rank() {
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.95), 4.0);
    }
}
