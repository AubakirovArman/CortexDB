use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::CellId;
use cortex_storage::indexes::BitmapIndex;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::metadata::scope_handle;
use crate::query::{scope_id, CellMetadata};

use super::persisted::{search_persisted_lexical, search_persisted_vectors};
use super::vector::vector_from_payload;
use super::{HnswIndex, SearchIndexes, SearchMode, SearchQuery};

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

    pub fn search_vector(
        &self,
        vector: &[i16],
        view: &AgentView,
        limit: SearchLimit,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        self.search_cells(
            SearchQuery {
                text: "",
                vector: Some(vector),
                limit: limit.0,
                mode: SearchMode::Vector,
            },
            view,
        )
    }

    pub fn search_vector_exact(
        &self,
        vector: &[i16],
        view: &AgentView,
        limit: SearchLimit,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        self.search_cells(
            SearchQuery {
                text: "",
                vector: Some(vector),
                limit: limit.0,
                mode: SearchMode::VectorExact,
            },
            view,
        )
    }

    fn search_persisted_query(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
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
        let ranked = match query.mode {
            SearchMode::Keyword => search_persisted_lexical(
                &state.lexical.terms,
                &state.lexical.doc_lengths,
                &state.lexical.term_frequencies,
                query.text,
                &allowed,
                query.limit,
            ),
            SearchMode::Vector => {
                let Some(vector) = query.vector else {
                    return Ok(Some(Vec::new()));
                };
                let index = self.persisted_vector_index()?;
                let graph = self.persisted_hnsw_graph()?;
                if graph.links.is_empty() {
                    search_persisted_vectors(&index.vectors, vector, &allowed, query.limit)
                } else {
                    HnswIndex::from_graph(index.vectors, graph, 8, 64).search_allowed(
                        vector,
                        &allowed,
                        query.limit,
                    )
                }
            }
            SearchMode::VectorExact => {
                let Some(vector) = query.vector else {
                    return Ok(Some(Vec::new()));
                };
                let index = self.persisted_vector_index()?;
                search_persisted_vectors(&index.vectors, vector, &allowed, query.limit)
            }
            SearchMode::Hybrid => return Ok(None),
        };
        let txn = self.read_txn();
        Ok(Some(
            ranked
                .into_iter()
                .filter_map(|candidate| {
                    let cell_id = state.candidate_to_cell.get(&candidate.cell_id)?;
                    self.get_cell(txn, *cell_id).map(|payload| {
                        let (lexical_score, vector_score) = match query.mode {
                            SearchMode::Keyword => (candidate.score, 0),
                            SearchMode::Vector | SearchMode::VectorExact => (0, candidate.score),
                            SearchMode::Hybrid => (0, 0),
                        };
                        DatabaseSearchResult {
                            cell_id: *cell_id,
                            score: candidate.score,
                            lexical_score,
                            vector_score,
                            payload,
                        }
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
        if let Some(results) = self.search_persisted_query(query, view)? {
            return Ok(results);
        }
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
            indexes.add_weighted_terms(candidate, metadata.weighted_lexical_terms());
            if let Some(vector) = vector_from_payload(&version.payload) {
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
