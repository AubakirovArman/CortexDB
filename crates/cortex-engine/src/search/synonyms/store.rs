use std::collections::BTreeMap;

use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::CellId;

use super::builder::build_corpus_synonym_dictionary;
use super::types::{CorpusSynonymDictionary, CorpusSynonymOptions};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CorpusSynonymStore {
    documents: BTreeMap<CellId, String>,
}

impl CorpusSynonymStore {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let documents = memtable
            .visible_iter(txn)
            .map(|version| (version.cell_id, document_from_payload(&version.payload)))
            .collect();
        Self { documents }
    }

    pub(crate) fn record_from_payload(cell_id: CellId, payload: &[u8]) -> (CellId, String) {
        (cell_id, document_from_payload(payload))
    }

    pub(crate) fn apply_record(&mut self, cell_id: CellId, record: (CellId, String)) {
        self.documents.insert(cell_id, record.1);
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.documents.remove(&cell_id);
    }

    pub(crate) fn dictionary(&self, options: CorpusSynonymOptions) -> CorpusSynonymDictionary {
        build_corpus_synonym_dictionary(self.documents.values().map(String::as_str), options)
    }
}

fn document_from_payload(payload: &[u8]) -> String {
    String::from_utf8_lossy(payload).into_owned()
}
