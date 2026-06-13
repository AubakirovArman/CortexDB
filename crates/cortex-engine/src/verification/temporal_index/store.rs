use std::collections::BTreeMap;

use cortex_aql::AgentView;
use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId};

use crate::query::{scope_id, CellMetadata};

use super::{
    metadata_value, temporal_date_for_record, temporal_fact_key, temporal_sort_key,
    TemporalFactIndex, TemporalFactKey, TemporalFactRecord,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TemporalFactStore {
    records: BTreeMap<CellId, TemporalFactIndexEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TemporalFactIndexEntry {
    scope: String,
    record: TemporalFactRecord,
}

impl TemporalFactStore {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let records = memtable
            .visible_iter(txn)
            .filter_map(|version| {
                Self::record_from_payload(version.cell_id, &version.payload, &version.descriptor)
            })
            .collect();
        Self { records }
    }

    pub(crate) fn record_from_payload(
        cell_id: CellId,
        payload: &[u8],
        descriptor: &CellDescriptor,
    ) -> Option<(CellId, TemporalFactIndexEntry)> {
        let metadata = CellMetadata::from_payload_with_descriptor(payload, descriptor);
        let key = temporal_fact_key(&metadata)?;
        Some((
            cell_id,
            TemporalFactIndexEntry {
                scope: metadata.scope.clone(),
                record: TemporalFactRecord {
                    cell_id,
                    key,
                    value: metadata_value(&metadata.body_text, &["value"]),
                    as_of: temporal_date_for_record(&metadata),
                    created_unix_seconds: metadata.created_unix_seconds,
                    superseded_by: None,
                    explicit_supersedes: metadata.supersedes,
                    explicit_superseded_by: metadata.superseded_by,
                    latest: false,
                },
            },
        ))
    }

    pub(crate) fn apply_record(
        &mut self,
        cell_id: CellId,
        record: Option<(CellId, TemporalFactIndexEntry)>,
    ) {
        if let Some((_, record)) = record {
            self.records.insert(cell_id, record);
        } else {
            self.records.remove(&cell_id);
        }
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.records.remove(&cell_id);
    }

    pub(crate) fn fact_index(&self, view: &AgentView) -> TemporalFactIndex {
        let mut groups = BTreeMap::<TemporalFactKey, Vec<TemporalFactRecord>>::new();
        for entry in self.records.values() {
            if !view.can_read_scope(scope_id(&entry.scope)) {
                continue;
            }
            groups
                .entry(entry.record.key.clone())
                .or_default()
                .push(entry.record.clone());
        }

        let mut records = Vec::new();
        for (_, mut group) in groups {
            group.sort_by_key(temporal_sort_key);
            let latest_cell_id = group.last().map(|record| record.cell_id);
            for record in &mut group {
                record.latest = Some(record.cell_id) == latest_cell_id;
                if !record.latest {
                    record.superseded_by = latest_cell_id;
                }
            }
            records.extend(group);
        }
        records.sort_by_key(|record| {
            (
                record.key.clone(),
                record.as_of,
                record.created_unix_seconds,
                record.cell_id,
            )
        });
        TemporalFactIndex { records }
    }
}
