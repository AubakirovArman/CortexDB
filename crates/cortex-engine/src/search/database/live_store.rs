use std::collections::BTreeMap;

use cortex_aql::AgentView;
use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId};

use crate::plan::PolicyRewrite;
use crate::query::{scope_id, CellMetadata};

use super::ranking::best_payload_vector_for_query;
use super::{DatabaseSearchResult, SearchViewTrace};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LiveSearchStore {
    records: BTreeMap<CellId, LiveSearchRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LiveSearchRecord {
    payload: Vec<u8>,
    metadata: CellMetadata,
}

impl LiveSearchStore {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let records = memtable
            .visible_iter(txn)
            .map(|version| {
                (
                    version.cell_id,
                    Self::record_from_payload(version.payload.clone(), &version.descriptor),
                )
            })
            .collect();
        Self { records }
    }

    pub(crate) fn record_from_payload(
        payload: Vec<u8>,
        descriptor: &CellDescriptor,
    ) -> LiveSearchRecord {
        let metadata = CellMetadata::from_payload_with_descriptor(&payload, descriptor);
        LiveSearchRecord { payload, metadata }
    }

    pub(crate) fn candidate_from_payload(
        cell_id: CellId,
        payload: Vec<u8>,
        descriptor: &CellDescriptor,
        query_vector: Option<&[i16]>,
    ) -> LiveSearchCandidate {
        let record = Self::record_from_payload(payload, descriptor);
        LiveSearchCandidate {
            cell_id,
            best_vector: best_payload_vector_for_query(&record.payload, query_vector),
            metadata: record.metadata,
            payload: record.payload,
        }
    }

    pub(crate) fn apply_record(&mut self, cell_id: CellId, record: LiveSearchRecord) {
        self.records.insert(cell_id, record);
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.records.remove(&cell_id);
    }

    pub(crate) fn visible_records(
        &self,
        view: &AgentView,
        query_vector: Option<&[i16]>,
    ) -> Vec<LiveSearchCandidate> {
        self.records
            .iter()
            .filter(|(_, record)| {
                PolicyRewrite::allows_scope(view, scope_id(&record.metadata.scope))
            })
            .map(|(cell_id, record)| LiveSearchCandidate {
                cell_id: *cell_id,
                payload: record.payload.clone(),
                metadata: record.metadata.clone(),
                best_vector: best_payload_vector_for_query(&record.payload, query_vector),
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LiveSearchCandidate {
    pub(crate) cell_id: CellId,
    pub(crate) payload: Vec<u8>,
    pub(crate) metadata: CellMetadata,
    pub(crate) best_vector: Option<super::ranking::BestPayloadVector>,
}

impl LiveSearchCandidate {
    pub(crate) fn into_result(
        self,
        score: u64,
        lexical_score: u64,
        vector_score: u64,
    ) -> DatabaseSearchResult {
        DatabaseSearchResult {
            cell_id: self.cell_id,
            score,
            lexical_score,
            vector_score,
            metadata: self.metadata,
            payload: self.payload,
        }
    }

    pub(crate) fn trace(&self, candidate_id: u32) -> Option<SearchViewTrace> {
        self.best_vector.as_ref().map(|best| SearchViewTrace {
            cell_id: self.cell_id,
            candidate_id,
            vector_view: best.view_name.clone(),
            vector_score: best.score,
        })
    }
}
