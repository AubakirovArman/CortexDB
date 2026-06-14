use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId};

use crate::verification::temporal::{
    parse_temporal_date, TemporalDate, TemporalQueryRange, TemporalStaleReason,
};

mod interval;

use interval::{TemporalIntervalIndex, TemporalZoneCache};

#[derive(Debug)]
pub(crate) struct TemporalValidityStore {
    records: BTreeMap<CellId, TemporalValidityRecord>,
    index: TemporalIntervalIndex,
    zone_cache: Mutex<TemporalZoneCache>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TemporalValidityRecord {
    pub(super) valid_from: Option<TemporalDate>,
    pub(super) valid_to: Option<TemporalDate>,
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
        Self::from_records(records)
    }

    fn from_records(records: BTreeMap<CellId, TemporalValidityRecord>) -> Self {
        let index = TemporalIntervalIndex::from_records(&records);
        let zone_cache = Mutex::new(TemporalZoneCache::from_records(&records));
        Self {
            records,
            index,
            zone_cache,
        }
    }

    pub(crate) fn apply_descriptor(&mut self, cell_id: CellId, descriptor: &CellDescriptor) {
        let old_record = self.records.remove(&cell_id);
        let new_record = Self::record_from_descriptor(descriptor);
        if let Some(record) = new_record {
            self.records.insert(cell_id, record);
        }
        self.index.replace_record(cell_id, old_record, new_record);
        self.mark_zone_cache_dirty();
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        let old_record = self.records.remove(&cell_id);
        self.index.replace_record(cell_id, old_record, None);
        self.mark_zone_cache_dirty();
    }

    #[cfg(test)]
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

    pub(crate) fn valid_cell_ids_at(&self, valid_at: &str) -> BTreeSet<CellId> {
        parse_temporal_date(valid_at)
            .map(|date| {
                let mut cache = self
                    .zone_cache
                    .lock()
                    .expect("temporal zone cache poisoned");
                if cache.dirty {
                    *cache = TemporalZoneCache::from_records(&self.records);
                }
                cache.valid_cell_ids_at(date)
            })
            .unwrap_or_default()
    }

    pub(crate) fn stale_cell_ids_for_range(&self, query: TemporalQueryRange) -> BTreeSet<CellId> {
        self.index.stale_cell_ids_for_range(query)
    }

    pub(crate) fn stale_reason_for_cell(
        &self,
        cell_id: CellId,
        query: TemporalQueryRange,
    ) -> Option<TemporalStaleReason> {
        self.records.get(&cell_id)?.stale_reason(query)
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

    fn mark_zone_cache_dirty(&self) {
        self.zone_cache
            .lock()
            .expect("temporal zone cache poisoned")
            .dirty = true;
    }
}

impl Default for TemporalValidityStore {
    fn default() -> Self {
        Self::from_records(BTreeMap::new())
    }
}

impl TemporalValidityRecord {
    #[cfg(test)]
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

    fn stale_reason(self, query: TemporalQueryRange) -> Option<TemporalStaleReason> {
        if let Some(valid_to) = self.valid_to {
            if query.start > valid_to {
                return Some(TemporalStaleReason::Expired { valid_to });
            }
        }
        if let Some(valid_from) = self.valid_from {
            if query.end < valid_from {
                return Some(TemporalStaleReason::NotYetValid { valid_from });
            }
        }
        None
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
        assert_eq!(
            store.valid_cell_ids_at("2025-06-01"),
            BTreeSet::from([CellId(1), CellId(4)])
        );
    }

    #[test]
    fn interval_index_tracks_updates_tombstones_and_stale_ranges() {
        let mut store = TemporalValidityStore::default();
        for id in 1..=160 {
            store.apply_descriptor(
                CellId(id),
                &CellDescriptor {
                    valid_to: Some("2024-12-31".to_owned()),
                    ..CellDescriptor::default()
                },
            );
        }
        store.apply_descriptor(
            CellId(200),
            &CellDescriptor {
                valid_from: Some("2025-01-01".to_owned()),
                valid_to: Some("2025-12-31".to_owned()),
                ..CellDescriptor::default()
            },
        );
        store.apply_descriptor(
            CellId(201),
            &CellDescriptor {
                valid_from: Some("2026-01-01".to_owned()),
                ..CellDescriptor::default()
            },
        );

        assert_eq!(
            store.valid_cell_ids_at("2025-06-01"),
            BTreeSet::from([CellId(200)])
        );
        let query = TemporalQueryRange {
            start: parse_temporal_date("2025-06-01").unwrap(),
            end: parse_temporal_date("2025-06-01").unwrap(),
        };
        let stale = store.stale_cell_ids_for_range(query);
        assert!(stale.contains(&CellId(1)));
        assert!(stale.contains(&CellId(160)));
        assert!(stale.contains(&CellId(201)));
        assert_eq!(
            store.stale_reason_for_cell(CellId(201), query),
            Some(TemporalStaleReason::NotYetValid {
                valid_from: parse_temporal_date("2026-01-01").unwrap()
            })
        );

        store.apply_tombstone(CellId(200));
        assert!(store.valid_cell_ids_at("2025-06-01").is_empty());
    }
}
