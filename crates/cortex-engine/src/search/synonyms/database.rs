use crate::database::Database;
use crate::error::EngineResult;

use super::builder::build_corpus_synonym_dictionary;
use super::persistence::{read_acsyn_dictionary, write_acsyn_dictionary, ACSYN_FILE_NAME};
use super::types::{CorpusSynonymDictionary, CorpusSynonymOptions};

impl Database {
    pub fn corpus_synonym_dictionary_path(&self) -> std::path::PathBuf {
        self.root_path().join(ACSYN_FILE_NAME)
    }

    pub fn corpus_synonym_dictionary(
        &self,
        options: CorpusSynonymOptions,
    ) -> CorpusSynonymDictionary {
        let payloads = self
            .snapshot_versions()
            .into_iter()
            .map(|version| String::from_utf8_lossy(&version.payload).into_owned())
            .collect::<Vec<_>>();
        build_corpus_synonym_dictionary(payloads.iter().map(String::as_str), options)
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
