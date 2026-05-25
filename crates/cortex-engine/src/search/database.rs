use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::CellId;
use cortex_storage::indexes::BitmapIndex;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::metadata::scope_handle;
use crate::query::{scope_id, CellMetadata};

use super::persisted::search_persisted_lexical;
use super::{SearchIndexes, SearchMode, SearchQuery};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchLimit(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseSearchResult {
    pub cell_id: CellId,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub payload: Vec<u8>,
}

impl Database {
    pub fn search_keyword(
        &self,
        text: &str,
        view: &AgentView,
        limit: SearchLimit,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        if let Some(results) = self.search_persisted_keyword(text, view, limit)? {
            return Ok(results);
        }
        self.search_cells(
            SearchQuery {
                text,
                vector: None,
                limit: limit.0,
                mode: SearchMode::Keyword,
            },
            view,
        )
    }

    fn search_persisted_keyword(
        &self,
        text: &str,
        view: &AgentView,
        limit: SearchLimit,
    ) -> EngineResult<Option<Vec<DatabaseSearchResult>>> {
        if self.manifest().live_segments.is_empty() {
            return Ok(None);
        }
        let checkpoint_seq = cortex_core::CommitSeq(self.manifest().checkpoint_seq);
        if !self
            .memtable
            .changed_cell_ids_after(checkpoint_seq)
            .is_empty()
        {
            return Ok(None);
        }
        let state = self.persisted_index_state()?;
        let allowed = allowed_candidates(&state.bitmap, view);
        let txn = self.read_txn();
        Ok(Some(
            search_persisted_lexical(
                &state.lexical.terms,
                &state.lexical.doc_lengths,
                text,
                &allowed,
                limit.0,
            )
            .into_iter()
            .filter_map(|candidate| {
                let cell_id = state.candidate_to_cell.get(&candidate.cell_id)?;
                self.get_cell(txn, *cell_id)
                    .map(|payload| DatabaseSearchResult {
                        cell_id: *cell_id,
                        score: candidate.score,
                        lexical_score: candidate.score,
                        vector_score: 0,
                        payload,
                    })
            })
            .collect(),
        ))
    }

    pub fn search_cells(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        let mut indexes = SearchIndexes::default();
        let mut cells = BTreeMap::<u32, (CellId, Vec<u8>)>::new();
        for (index, (version, metadata)) in self
            .snapshot_versions()
            .into_iter()
            .filter_map(|version| {
                let metadata = CellMetadata::from_payload(&version.payload);
                view.can_read_scope(scope_id(&metadata.scope))
                    .then_some((version, metadata))
            })
            .enumerate()
        {
            let candidate =
                u32::try_from(index + 1).map_err(|_| EngineError::CandidateIdOverflow)?;
            indexes.add_document(candidate, &metadata.body_text);
            if let Some(vector) = vector_line(&version.payload) {
                indexes.add_vector(candidate, vector);
            }
            cells.insert(candidate, (version.cell_id, version.payload));
        }
        Ok(indexes
            .search(query)
            .into_iter()
            .filter_map(|result| {
                let (cell_id, payload) = cells.remove(&result.cell_id)?;
                Some(DatabaseSearchResult {
                    cell_id,
                    score: result.score,
                    lexical_score: result.lexical_score,
                    vector_score: result.vector_score,
                    payload,
                })
            })
            .collect())
    }
}

fn allowed_candidates(bitmap: &BitmapIndex, view: &AgentView) -> BTreeSet<u32> {
    let mut allowed = BTreeSet::new();
    for scope in &view.readable_scopes {
        if let Some(candidates) = bitmap.bitmaps.get(&scope_handle(*scope).0) {
            allowed.extend(candidates.iter().copied());
        }
    }
    allowed
}

fn vector_line(payload: &[u8]) -> Option<Vec<i16>> {
    let text = String::from_utf8_lossy(payload);
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix("vector=")?;
        let vector = value
            .split([',', ' '])
            .filter(|part| !part.is_empty())
            .map(str::parse::<i16>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        (!vector.is_empty()).then_some(vector)
    })
}
