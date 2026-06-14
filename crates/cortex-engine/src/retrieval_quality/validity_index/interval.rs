use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Unbounded};

use cortex_core::CellId;

use crate::verification::temporal::{TemporalDate, TemporalQueryRange};

use super::TemporalValidityRecord;

const MIN_TEMPORAL_DATE: TemporalDate = TemporalDate {
    year: 1900,
    month: 1,
    day: 1,
};
const MAX_TEMPORAL_DATE: TemporalDate = TemporalDate {
    year: 2199,
    month: 12,
    day: 31,
};
const TEMPORAL_ZONE_SIZE: usize = 64;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct TemporalIntervalIndex {
    by_valid_from: BTreeMap<TemporalDate, BTreeSet<CellId>>,
    by_valid_to: BTreeMap<TemporalDate, BTreeSet<CellId>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TemporalIntervalEntry {
    cell_id: CellId,
    valid_from: TemporalDate,
    valid_to: TemporalDate,
}

impl TemporalIntervalIndex {
    pub(super) fn from_records(records: &BTreeMap<CellId, TemporalValidityRecord>) -> Self {
        let mut index = Self::default();
        for (cell_id, record) in records {
            index.add_record(*cell_id, *record);
        }
        index
    }

    pub(super) fn replace_record(
        &mut self,
        cell_id: CellId,
        old_record: Option<TemporalValidityRecord>,
        new_record: Option<TemporalValidityRecord>,
    ) {
        if let Some(record) = old_record {
            self.remove_record(cell_id, record);
        }
        if let Some(record) = new_record {
            self.add_record(cell_id, record);
        }
    }

    pub(super) fn stale_cell_ids_for_range(&self, query: TemporalQueryRange) -> BTreeSet<CellId> {
        let mut ids = BTreeSet::new();
        for cell_ids in self.by_valid_to.range(..query.start).map(|(_, ids)| ids) {
            ids.extend(cell_ids.iter().copied());
        }
        for cell_ids in self
            .by_valid_from
            .range((Excluded(query.end), Unbounded))
            .map(|(_, ids)| ids)
        {
            ids.extend(cell_ids.iter().copied());
        }
        ids
    }

    fn add_record(&mut self, cell_id: CellId, record: TemporalValidityRecord) {
        if let Some(valid_from) = record.valid_from {
            self.by_valid_from
                .entry(valid_from)
                .or_default()
                .insert(cell_id);
        }
        if let Some(valid_to) = record.valid_to {
            self.by_valid_to
                .entry(valid_to)
                .or_default()
                .insert(cell_id);
        }
    }

    fn remove_record(&mut self, cell_id: CellId, record: TemporalValidityRecord) {
        remove_index_cell(&mut self.by_valid_from, record.valid_from, cell_id);
        remove_index_cell(&mut self.by_valid_to, record.valid_to, cell_id);
    }
}

fn remove_index_cell(
    index: &mut BTreeMap<TemporalDate, BTreeSet<CellId>>,
    key: Option<TemporalDate>,
    cell_id: CellId,
) {
    let Some(key) = key else {
        return;
    };
    let Some(cell_ids) = index.get_mut(&key) else {
        return;
    };
    cell_ids.remove(&cell_id);
    if cell_ids.is_empty() {
        index.remove(&key);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct TemporalZoneCache {
    entries: Vec<TemporalIntervalEntry>,
    zones: Vec<TemporalIntervalZone>,
    pub(super) dirty: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TemporalIntervalZone {
    start: usize,
    end: usize,
    max_valid_to: TemporalDate,
}

impl TemporalZoneCache {
    pub(super) fn from_records(records: &BTreeMap<CellId, TemporalValidityRecord>) -> Self {
        let mut entries = records
            .iter()
            .map(|(cell_id, record)| TemporalIntervalEntry {
                cell_id: *cell_id,
                valid_from: record.valid_from.unwrap_or(MIN_TEMPORAL_DATE),
                valid_to: record.valid_to.unwrap_or(MAX_TEMPORAL_DATE),
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| (entry.valid_from, entry.valid_to, entry.cell_id));
        let zones = entries
            .chunks(TEMPORAL_ZONE_SIZE)
            .enumerate()
            .map(|(zone_index, chunk)| {
                let start = zone_index * TEMPORAL_ZONE_SIZE;
                let end = start + chunk.len();
                let max_valid_to = chunk
                    .iter()
                    .map(|entry| entry.valid_to)
                    .max()
                    .unwrap_or(MIN_TEMPORAL_DATE);
                TemporalIntervalZone {
                    start,
                    end,
                    max_valid_to,
                }
            })
            .collect();
        Self {
            entries,
            zones,
            dirty: false,
        }
    }

    pub(super) fn valid_cell_ids_at(&self, valid_at: TemporalDate) -> BTreeSet<CellId> {
        let end = self
            .entries
            .partition_point(|entry| entry.valid_from <= valid_at);
        let mut ids = BTreeSet::new();
        for zone in self.zones.iter().take_while(|zone| zone.start < end) {
            let zone_end = zone.end.min(end);
            if zone.max_valid_to < valid_at {
                continue;
            }
            for entry in &self.entries[zone.start..zone_end] {
                if entry.valid_to >= valid_at {
                    ids.insert(entry.cell_id);
                }
            }
        }
        ids
    }
}
