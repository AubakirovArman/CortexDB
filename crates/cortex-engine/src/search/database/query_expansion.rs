use crate::database::Database;
use crate::error::EngineResult;

use super::super::synonyms::expand_query_with_corpus_synonyms;

const MAX_CORPUS_SYNONYM_QUERY_TERMS: usize = 12;
const MAX_CORPUS_SYNONYMS_PER_QUERY_TERM: usize = 2;

impl Database {
    pub(crate) fn corpus_synonym_expanded_query_text(
        &self,
        query: &str,
    ) -> EngineResult<Option<String>> {
        let Some(dictionary) = self.read_persisted_corpus_synonym_dictionary()? else {
            return Ok(None);
        };
        Ok(expand_query_with_corpus_synonyms(
            query,
            &dictionary,
            MAX_CORPUS_SYNONYM_QUERY_TERMS,
            MAX_CORPUS_SYNONYMS_PER_QUERY_TERM,
        ))
    }
}
