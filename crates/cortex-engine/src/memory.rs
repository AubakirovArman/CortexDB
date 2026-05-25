use cortex_core::CellId;

use crate::database::Database;
use crate::error::EngineResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpiredMemoryCell {
    pub cell_id: CellId,
    pub expired_at_unix_seconds: u64,
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
}

fn memory_expired_at(payload: &[u8], now_unix_seconds: u64) -> Option<u64> {
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
    let expires_at = created_unix_seconds?.checked_add(ttl_seconds?)?;
    (is_memory && expires_at <= now_unix_seconds).then_some(expires_at)
}
