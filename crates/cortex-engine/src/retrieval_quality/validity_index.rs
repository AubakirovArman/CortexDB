use std::collections::BTreeMap;

use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId};

use crate::verification::temporal::{parse_temporal_date, TemporalDate, TemporalQueryRange};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TemporalValidityStore {
    records: BTreeMap<CellId, TemporalValidityRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TemporalValidityRecord {
    valid_from: Option<TemporalDate>,
    valid_to: Option<TemporalDate>,
}

impl TemporalValidityStore {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let records = memtable
            .visible_iter(txn)
            .filter_map(|version| {
                Self::record_from_descriptor(&version.descriptor)
                    .map(|record| (version.cell_id, record))
            })
            .collect();
        Self { records }
    }

    pub(crate) fn apply_descriptor(&mut self, cell_id: CellId, descriptor: &CellDescriptor) {
        if let Some(record) = Self::record_from_descriptor(descriptor) {
            self.records.insert(cell_id, record);
        } else {
            self.records.remove(&cell_id);
        }
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.records.remove(&cell_id);
    }

    pub(crate) fn is_valid_at(&self, cell_id: CellId, valid_at: Option<&str>) -> bool {
        let Some(valid_at) = valid_at else {
            return true;
        };
        let Some(valid_at) = parse_temporal_date(valid_at) else {
            return false;
        };
        let Some(record) = self.records.get(&cell_id) else {
            return true;
        };
        record.is_valid_at(valid_at)
    }

    fn record_from_descriptor(descriptor: &CellDescriptor) -> Option<TemporalValidityRecord> {
        Some(TemporalValidityRecord {
            valid_from: descriptor
                .valid_from
                .as_deref()
                .and_then(parse_temporal_date),
            valid_to: descriptor.valid_to.as_deref().and_then(parse_temporal_date),
        })
    }
}

impl TemporalValidityRecord {
    fn is_valid_at(self, valid_at: TemporalDate) -> bool {
        let query = TemporalQueryRange {
            start: valid_at,
            end: valid_at,
        };
        if let Some(valid_to) = self.valid_to {
            if query.start > valid_to {
                return false;
            }
        }
        if let Some(valid_from) = self.valid_from {
            if query.end < valid_from {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use cortex_core::CellDescriptor;

    use super::*;

    #[test]
    fn descriptor_temporal_validity_uses_inclusive_open_ranges() {
        let mut store = TemporalValidityStore::default();
        store.apply_descriptor(
            CellId(1),
            &CellDescriptor {
                valid_from: Some("2025-01-01".to_owned()),
                valid_to: Some("2025-12-31".to_owned()),
                ..CellDescriptor::default()
            },
        );
        store.apply_descriptor(
            CellId(2),
            &CellDescriptor {
                valid_to: Some("2024-12-31".to_owned()),
                ..CellDescriptor::default()
            },
        );
        store.apply_descriptor(
            CellId(3),
            &CellDescriptor {
                valid_from: Some("2026-01-01".to_owned()),
                ..CellDescriptor::default()
            },
        );
        store.apply_descriptor(CellId(4), &CellDescriptor::default());

        assert!(store.is_valid_at(CellId(1), Some("2025-06-01")));
        assert!(!store.is_valid_at(CellId(2), Some("2025-06-01")));
        assert!(!store.is_valid_at(CellId(3), Some("2025-06-01")));
        assert!(store.is_valid_at(CellId(4), Some("2025-06-01")));
    }
}
