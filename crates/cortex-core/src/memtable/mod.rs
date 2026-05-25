mod accumulator;
mod version;

use std::collections::BTreeMap;

pub use accumulator::{CellAccumulator, SectionFragment};
pub use version::{CellVersion, IndexDebt};

use crate::error::{CoreError, CoreResult};
use crate::types::{CellId, CommitSeq};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadTxn {
    pub read_seq: CommitSeq,
}

#[derive(Clone, Debug, Default)]
pub struct MemTable {
    versions: BTreeMap<CellId, Vec<CellVersion>>,
}

impl MemTable {
    pub fn put_cell(&mut self, cell_id: CellId, seq: CommitSeq, payload: Vec<u8>) {
        self.versions
            .entry(cell_id)
            .or_default()
            .push(CellVersion::new(cell_id, seq, payload, 0));
    }

    pub fn patch_cell(
        &mut self,
        cell_id: CellId,
        seq: CommitSeq,
        payload: Vec<u8>,
    ) -> CoreResult<()> {
        let versions = self
            .versions
            .get_mut(&cell_id)
            .ok_or(CoreError::CellNotFound(cell_id))?;
        let depth = versions.last().map_or(0, |version| version.delta_depth + 1);
        if let Some(version) = versions.last_mut() {
            version.deleted_seq = Some(seq);
        }
        versions.push(CellVersion::new(cell_id, seq, payload, depth));
        Ok(())
    }

    pub fn tombstone_cell(&mut self, cell_id: CellId, seq: CommitSeq) -> CoreResult<()> {
        let versions = self
            .versions
            .get_mut(&cell_id)
            .ok_or(CoreError::CellNotFound(cell_id))?;
        if let Some(version) = versions.last_mut() {
            version.deleted_seq = Some(seq);
        }
        Ok(())
    }

    pub fn read(&self, txn: ReadTxn, cell_id: CellId) -> Option<&CellVersion> {
        self.versions
            .get(&cell_id)?
            .iter()
            .rev()
            .find(|version| version.visible_at(txn.read_seq))
    }

    pub fn visible_cells(&self, txn: ReadTxn) -> Vec<CellVersion> {
        self.versions
            .keys()
            .filter_map(|cell_id| self.read(txn, *cell_id).cloned())
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }

    pub fn compaction_priority(&self, cell_id: CellId) -> Option<u32> {
        self.versions
            .get(&cell_id)?
            .iter()
            .map(|version| version.delta_depth + version.index_debt.total())
            .max()
    }
}
