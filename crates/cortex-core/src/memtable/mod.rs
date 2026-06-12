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

impl ReadTxn {
    pub fn at(read_seq: CommitSeq) -> Self {
        Self { read_seq }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MemTable {
    versions: BTreeMap<CellId, Vec<CellVersion>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemTableStats {
    pub cell_count: usize,
    pub live_cell_count: usize,
    pub deleted_cell_count: usize,
    pub version_count: usize,
    pub tombstone_count: usize,
    pub max_delta_depth: u32,
    pub index_debt_total: u32,
    pub payload_bytes: usize,
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

    pub fn record_tombstone(&mut self, cell_id: CellId, seq: CommitSeq) {
        let versions = self.versions.entry(cell_id).or_default();
        if let Some(version) = versions.last_mut() {
            version.deleted_seq = Some(seq);
            return;
        }

        let mut marker = CellVersion::new(cell_id, seq, Vec::new(), 0);
        marker.deleted_seq = Some(seq);
        versions.push(marker);
    }

    pub fn read(&self, txn: ReadTxn, cell_id: CellId) -> Option<&CellVersion> {
        self.versions
            .get(&cell_id)?
            .iter()
            .rev()
            .find(|version| version.visible_at(txn.read_seq))
    }

    pub fn read_at(&self, seq: CommitSeq, cell_id: CellId) -> Option<&CellVersion> {
        self.read(ReadTxn::at(seq), cell_id)
    }

    pub fn max_cell_id(&self) -> Option<CellId> {
        self.versions.keys().next_back().copied()
    }

    pub fn visible_cells(&self, txn: ReadTxn) -> Vec<CellVersion> {
        self.visible_iter(txn).cloned().collect()
    }

    pub fn visible_iter(&self, txn: ReadTxn) -> impl Iterator<Item = &CellVersion> + '_ {
        self.versions
            .keys()
            .filter_map(move |cell_id| self.read(txn, *cell_id))
    }

    pub fn live_cell_ids(&self, txn: ReadTxn) -> Vec<CellId> {
        self.versions
            .keys()
            .filter(|cell_id| self.contains_visible(txn, **cell_id))
            .copied()
            .collect()
    }

    pub fn deleted_cell_ids(&self) -> Vec<CellId> {
        self.versions
            .iter()
            .filter_map(|(cell_id, versions)| {
                versions
                    .last()
                    .is_some_and(|version| version.deleted_seq.is_some())
                    .then_some(*cell_id)
            })
            .collect()
    }

    pub fn range_scan(&self, txn: ReadTxn, start: CellId, end: CellId) -> Vec<CellVersion> {
        self.range_iter(txn, start, end).cloned().collect()
    }

    pub fn range_iter(
        &self,
        txn: ReadTxn,
        start: CellId,
        end: CellId,
    ) -> impl Iterator<Item = &CellVersion> + '_ {
        self.versions
            .range(start..=end)
            .filter_map(move |(cell_id, _)| self.read(txn, *cell_id))
    }

    pub fn visible_cells_created_after(&self, txn: ReadTxn, seq: CommitSeq) -> Vec<CellVersion> {
        self.visible_created_after_iter(txn, seq).cloned().collect()
    }

    pub fn visible_created_after_iter(
        &self,
        txn: ReadTxn,
        seq: CommitSeq,
    ) -> impl Iterator<Item = &CellVersion> + '_ {
        self.visible_iter(txn)
            .filter(move |version| version.created_seq > seq)
    }

    pub fn tombstones_after(&self, seq: CommitSeq) -> Vec<(CellId, CommitSeq)> {
        self.versions
            .iter()
            .filter_map(|(cell_id, versions)| {
                let last = versions.last()?;
                let deleted = last.deleted_seq?;
                (deleted > seq).then_some((*cell_id, deleted))
            })
            .collect()
    }

    pub fn gc_versions_before(&mut self, seq: CommitSeq) {
        let mut to_remove = Vec::new();
        for (cell_id, versions) in self.versions.iter_mut() {
            let Some(visible_idx) = versions
                .iter()
                .enumerate()
                .rev()
                .find(|(_, version)| version.created_seq <= seq)
                .map(|(idx, _)| idx)
            else {
                continue;
            };

            *versions = versions.split_off(visible_idx);

            if versions.len() == 1 {
                let first = &versions[0];
                if first.deleted_seq.is_some_and(|deleted| deleted <= seq) {
                    to_remove.push(*cell_id);
                }
            }
        }

        for cell_id in to_remove {
            self.versions.remove(&cell_id);
        }
    }

    pub fn changed_cell_ids_after(&self, seq: CommitSeq) -> Vec<CellId> {
        self.versions
            .iter()
            .filter_map(|(cell_id, versions)| {
                versions
                    .iter()
                    .any(|version| {
                        version.created_seq > seq
                            || version.deleted_seq.is_some_and(|deleted| deleted > seq)
                    })
                    .then_some(*cell_id)
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }

    pub fn contains_visible(&self, txn: ReadTxn, cell_id: CellId) -> bool {
        self.read(txn, cell_id).is_some()
    }

    pub fn latest_seq(&self) -> CommitSeq {
        self.versions
            .values()
            .flat_map(|versions| versions.iter())
            .fold(CommitSeq(0), |latest, version| {
                latest
                    .max(version.created_seq)
                    .max(version.deleted_seq.unwrap_or(CommitSeq(0)))
            })
    }

    pub fn stats(&self) -> MemTableStats {
        let mut stats = MemTableStats {
            cell_count: self.versions.len(),
            ..MemTableStats::default()
        };
        for versions in self.versions.values() {
            stats.version_count += versions.len();
            stats.payload_bytes += versions
                .iter()
                .map(|version| version.payload.len())
                .sum::<usize>();
            if versions
                .last()
                .is_some_and(|version| version.deleted_seq.is_some())
            {
                stats.tombstone_count += 1;
                stats.deleted_cell_count += 1;
            } else {
                stats.live_cell_count += 1;
            }
            for version in versions {
                stats.max_delta_depth = stats.max_delta_depth.max(version.delta_depth);
                stats.index_debt_total += version.index_debt.total();
            }
        }
        stats
    }

    pub fn compaction_priority(&self, cell_id: CellId) -> Option<u32> {
        self.versions
            .get(&cell_id)?
            .iter()
            .map(|version| version.delta_depth + version.index_debt.total())
            .max()
    }
}
