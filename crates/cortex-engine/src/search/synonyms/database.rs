use crate::database::Database;
use crate::error::EngineResult;
use crate::options::PayloadResidency;

use super::persistence::{read_acsyn_dictionary, write_acsyn_dictionary, ACSYN_FILE_NAME};
use super::store::CorpusSynonymStore;
use super::types::{CorpusSynonymDictionary, CorpusSynonymOptions};

impl Database {
    pub fn corpus_synonym_dictionary_path(&self) -> std::path::PathBuf {
        self.root_path().join(ACSYN_FILE_NAME)
    }

    pub fn corpus_synonym_dictionary(
        &self,
        options: CorpusSynonymOptions,
    ) -> CorpusSynonymDictionary {
        if self.payload_residency == PayloadResidency::Lazy {
            return self.lazy_corpus_synonym_store().dictionary(options);
        }
        self.derived_stores.corpus_synonym_store.dictionary(options)
    }

    fn lazy_corpus_synonym_store(&self) -> CorpusSynonymStore {
        let records = self
            .memtable
            .visible_iter(self.read_txn())
            .filter_map(|version| {
                let payload = self.payload_for_version(version).ok()?;
                Some(CorpusSynonymStore::record_from_payload(
                    version.cell_id,
                    &payload,
                ))
            });
        CorpusSynonymStore::from_records(records)
    }

    pub fn persist_corpus_synonym_dictionary(
        &self,
        options: CorpusSynonymOptions,
    ) -> EngineResult<CorpusSynonymDictionary> {
        let dictionary = self.corpus_synonym_dictionary(options);
        write_acsyn_dictionary(&self.corpus_synonym_dictionary_path(), &dictionary)?;
        Ok(dictionary)
    }

    pub(crate) fn publish_checkpoint_corpus_synonym_dictionary(&self) -> EngineResult<()> {
        self.persist_corpus_synonym_dictionary(CorpusSynonymOptions::default())?;
        Ok(())
    }

    pub fn read_persisted_corpus_synonym_dictionary(
        &self,
    ) -> EngineResult<Option<CorpusSynonymDictionary>> {
        let path = self.corpus_synonym_dictionary_path();
        if !path.exists() {
            return Ok(None);
        }
        read_acsyn_dictionary(&path).map(Some).map_err(Into::into)
    }
}
