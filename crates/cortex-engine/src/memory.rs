mod lifecycle;

pub(crate) use lifecycle::MemoryLifecycleStore;

use cortex_aql::Q16;
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
    pub fn forget_cell(&mut self, cell_id: CellId) -> EngineResult<()> {
        self.tombstone_cell(cell_id)?;
        Ok(())
    }

    pub fn expired_memory_cells(&self, now_unix_seconds: u64) -> Vec<ExpiredMemoryCell> {
        self.derived_stores
            .memory_lifecycle_store
            .expired_cells(now_unix_seconds)
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
        self.derived_stores
            .memory_lifecycle_store
            .decay_scores(now_unix_seconds)
    }
}
