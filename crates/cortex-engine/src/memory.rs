use cortex_aql::{Q16, Q16_ONE, Q16_ZERO};
use cortex_core::CellId;

use crate::database::Database;
use crate::error::EngineResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpiredMemoryCell {
    pub cell_id: CellId,
    pub expired_at_unix_seconds: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryDecayScore {
    pub cell_id: CellId,
    pub freshness_q16: Q16,
    pub age_seconds: Option<u64>,
    pub ttl_seconds: Option<u64>,
}

impl Database {
    pub fn expired_memory_cells(&self, now_unix_seconds: u64) -> Vec<ExpiredMemoryCell> {
        self.snapshot_versions()
            .into_iter()
            .filter_map(|version| {
                memory_expired_at(&version.payload, now_unix_seconds).map(|expired_at| {
                    ExpiredMemoryCell {
                        cell_id: version.cell_id,
                        expired_at_unix_seconds: expired_at,
                    }
                })
            })
            .collect()
    }

    pub fn expire_memory_cells(
        &mut self,
        now_unix_seconds: u64,
    ) -> EngineResult<Vec<ExpiredMemoryCell>> {
        let expired = self.expired_memory_cells(now_unix_seconds);
        for cell in &expired {
            self.tombstone_cell(cell.cell_id)?;
        }
        Ok(expired)
    }

    pub fn memory_decay_scores(&self, now_unix_seconds: u64) -> Vec<MemoryDecayScore> {
        self.snapshot_versions()
            .into_iter()
            .filter_map(|version| {
                memory_decay_score(&version.payload, now_unix_seconds).map(|decay| {
                    MemoryDecayScore {
                        cell_id: version.cell_id,
                        freshness_q16: decay.freshness_q16,
                        age_seconds: decay.age_seconds,
                        ttl_seconds: decay.ttl_seconds,
                    }
                })
            })
            .collect()
    }
}

fn memory_expired_at(payload: &[u8], now_unix_seconds: u64) -> Option<u64> {
    let timing = memory_timing(payload)?;
    let expires_at = timing
        .created_unix_seconds?
        .checked_add(timing.ttl_seconds?)?;
    (expires_at <= now_unix_seconds).then_some(expires_at)
}

fn memory_decay_score(payload: &[u8], now_unix_seconds: u64) -> Option<MemoryDecay> {
    let metadata = memory_timing(payload)?;
    let age_seconds = metadata
        .created_unix_seconds
        .map(|created| now_unix_seconds.saturating_sub(created));
    let freshness_q16 = match (metadata.created_unix_seconds, metadata.ttl_seconds) {
        (_, None) => Q16_ONE,
        (None, Some(_)) => Q16_ONE,
        (Some(created), Some(ttl)) => freshness_for_ttl(created, ttl, now_unix_seconds),
    };
    Some(MemoryDecay {
        freshness_q16,
        age_seconds,
        ttl_seconds: metadata.ttl_seconds,
    })
}

fn freshness_for_ttl(created: u64, ttl: u64, now: u64) -> Q16 {
    let expires_at = created.saturating_add(ttl);
    if ttl == 0 || now >= expires_at {
        return Q16_ZERO;
    }
    if now <= created {
        return Q16_ONE;
    }
    let remaining = expires_at - now;
    (((u128::from(remaining) * u128::from(Q16_ONE)) + u128::from(ttl / 2)) / u128::from(ttl)) as Q16
}

fn memory_timing(payload: &[u8]) -> Option<MemoryTiming> {
    let text = String::from_utf8_lossy(payload);
    let mut is_memory = false;
    let mut ttl_seconds = None;
    let mut created_unix_seconds = None;
    for line in text.lines() {
        if line.trim() == "type=memory" {
            is_memory = true;
        } else if let Some(value) = line.strip_prefix("ttl_seconds=") {
            ttl_seconds = value.trim().parse::<u64>().ok();
        } else if let Some(value) = line.strip_prefix("created_unix_seconds=") {
            created_unix_seconds = value.trim().parse::<u64>().ok();
        }
    }
    is_memory.then_some(MemoryTiming {
        ttl_seconds,
        created_unix_seconds,
    })
}

struct MemoryTiming {
    ttl_seconds: Option<u64>,
    created_unix_seconds: Option<u64>,
}

struct MemoryDecay {
    freshness_q16: Q16,
    age_seconds: Option<u64>,
    ttl_seconds: Option<u64>,
}
