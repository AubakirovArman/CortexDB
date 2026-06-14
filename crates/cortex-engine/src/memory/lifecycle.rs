use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{Q16, Q16_ONE, Q16_ZERO};
use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId, KnowledgeCellType};

use super::{ExpiredMemoryCell, MemoryDecayScore};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MemoryLifecycleStore {
    records: BTreeMap<CellId, MemoryLifecycleRecord>,
    cells_by_expiry: BTreeMap<u64, BTreeSet<CellId>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MemoryLifecycleRecord {
    created_unix_seconds: Option<u64>,
    ttl_seconds: Option<u64>,
    expires_at_unix_seconds: Option<u64>,
}

impl MemoryLifecycleStore {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let mut store = Self::default();
        for version in memtable.visible_iter(txn) {
            store.apply_descriptor(version.cell_id, &version.descriptor);
        }
        store
    }

    pub(crate) fn apply_descriptor(&mut self, cell_id: CellId, descriptor: &CellDescriptor) {
        self.remove_record(cell_id);
        if let Some(record) = Self::record_from_descriptor(descriptor) {
            self.insert_record(cell_id, record);
        }
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.remove_record(cell_id);
    }

    pub(crate) fn expired_cells(&self, now_unix_seconds: u64) -> Vec<ExpiredMemoryCell> {
        let mut expired = Vec::new();
        for (expires_at, cell_ids) in self.cells_by_expiry.range(..=now_unix_seconds) {
            for cell_id in cell_ids {
                expired.push(ExpiredMemoryCell {
                    cell_id: *cell_id,
                    expired_at_unix_seconds: *expires_at,
                });
            }
        }
        expired
    }

    pub(crate) fn decay_scores(&self, now_unix_seconds: u64) -> Vec<MemoryDecayScore> {
        self.records
            .iter()
            .map(|(cell_id, record)| record.decay_score(*cell_id, now_unix_seconds))
            .collect()
    }

    pub(crate) fn is_active_at(&self, cell_id: CellId, now_unix_seconds: u64) -> bool {
        self.records
            .get(&cell_id)
            .and_then(|record| record.expires_at_unix_seconds)
            .is_none_or(|expires_at| expires_at > now_unix_seconds)
    }

    fn record_from_descriptor(descriptor: &CellDescriptor) -> Option<MemoryLifecycleRecord> {
        if descriptor.cell_type != KnowledgeCellType::Memory {
            return None;
        }
        Some(MemoryLifecycleRecord::new(
            descriptor.created_unix_seconds,
            descriptor.ttl_seconds,
        ))
    }

    fn insert_record(&mut self, cell_id: CellId, record: MemoryLifecycleRecord) {
        if let Some(expires_at) = record.expires_at_unix_seconds {
            self.cells_by_expiry
                .entry(expires_at)
                .or_default()
                .insert(cell_id);
        }
        self.records.insert(cell_id, record);
    }

    fn remove_record(&mut self, cell_id: CellId) {
        let Some(record) = self.records.remove(&cell_id) else {
            return;
        };
        if let Some(expires_at) = record.expires_at_unix_seconds {
            let remove_entry = if let Some(cell_ids) = self.cells_by_expiry.get_mut(&expires_at) {
                cell_ids.remove(&cell_id);
                cell_ids.is_empty()
            } else {
                false
            };
            if remove_entry {
                self.cells_by_expiry.remove(&expires_at);
            }
        }
    }
}

impl MemoryLifecycleRecord {
    fn new(created_unix_seconds: Option<u64>, ttl_seconds: Option<u64>) -> Self {
        let expires_at_unix_seconds = created_unix_seconds
            .zip(ttl_seconds)
            .and_then(|(created, ttl)| created.checked_add(ttl));
        Self {
            created_unix_seconds,
            ttl_seconds,
            expires_at_unix_seconds,
        }
    }

    fn decay_score(self, cell_id: CellId, now_unix_seconds: u64) -> MemoryDecayScore {
        MemoryDecayScore {
            cell_id,
            freshness_q16: self.freshness_q16(now_unix_seconds),
            age_seconds: self
                .created_unix_seconds
                .map(|created| now_unix_seconds.saturating_sub(created)),
            ttl_seconds: self.ttl_seconds,
        }
    }

    fn freshness_q16(self, now_unix_seconds: u64) -> Q16 {
        match (self.created_unix_seconds, self.ttl_seconds) {
            (_, None) | (None, Some(_)) => Q16_ONE,
            (Some(created), Some(ttl)) => freshness_for_ttl(created, ttl, now_unix_seconds),
        }
    }
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

#[cfg(test)]
mod tests {
    use cortex_core::{CellDescriptor, CellId, KnowledgeCellType};

    use super::*;

    #[test]
    fn expired_cells_are_sorted_by_expiry_then_cell_id() {
        let mut store = MemoryLifecycleStore::default();
        store.apply_descriptor(CellId(2), &memory_descriptor(100, Some(10)));
        store.apply_descriptor(CellId(1), &memory_descriptor(90, Some(10)));
        store.apply_descriptor(CellId(3), &memory_descriptor(100, Some(200)));

        assert_eq!(
            store.expired_cells(110),
            vec![
                ExpiredMemoryCell {
                    cell_id: CellId(1),
                    expired_at_unix_seconds: 100,
                },
                ExpiredMemoryCell {
                    cell_id: CellId(2),
                    expired_at_unix_seconds: 110,
                },
            ]
        );
    }

    #[test]
    fn decay_scores_match_ttl_policy() {
        let mut store = MemoryLifecycleStore::default();
        store.apply_descriptor(CellId(1), &memory_descriptor(100, Some(100)));
        store.apply_descriptor(CellId(2), &memory_descriptor(100, None));

        let scores = store.decay_scores(150);
        assert_eq!(scores[0].freshness_q16, 32_768);
        assert_eq!(scores[0].age_seconds, Some(50));
        assert_eq!(scores[0].ttl_seconds, Some(100));
        assert_eq!(scores[1].freshness_q16, Q16_ONE);
        assert_eq!(store.decay_scores(201)[0].freshness_q16, Q16_ZERO);
    }

    #[test]
    fn non_memory_cells_are_not_indexed() {
        let mut store = MemoryLifecycleStore::default();
        store.apply_descriptor(
            CellId(1),
            &CellDescriptor {
                created_unix_seconds: Some(100),
                ttl_seconds: Some(1),
                ..CellDescriptor::default()
            },
        );

        assert!(store.expired_cells(200).is_empty());
        assert!(store.decay_scores(200).is_empty());
        assert!(store.is_active_at(CellId(1), 200));
    }

    fn memory_descriptor(created: u64, ttl: Option<u64>) -> CellDescriptor {
        CellDescriptor {
            cell_type: KnowledgeCellType::Memory,
            created_unix_seconds: Some(created),
            ttl_seconds: ttl,
            ..CellDescriptor::default()
        }
    }
}
