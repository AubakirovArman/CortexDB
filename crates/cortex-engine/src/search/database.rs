use std::collections::BTreeMap;

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::{scope_id, CellMetadata};

use super::access::allowed_candidates;
use super::ann::{
    finalize_report, search_persisted_ann_with_policy, AnnFallbackReason, AnnSearchPath,
    AnnSearchPolicy, AnnSearchReport,
};
use super::hnsw::DistanceMetric;
use super::persisted::{search_persisted_lexical, search_persisted_vectors};
use super::vector::vector_from_payload;
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseSearchOutcome {
    pub results: Vec<DatabaseSearchResult>,
    pub ann_report: Option<AnnSearchReport>,
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

    pub fn search_vector_with_report(
        &self,
        vector: &[i16],
        view: &AgentView,
        limit: SearchLimit,
    ) -> EngineResult<DatabaseSearchOutcome> {
        self.search_vector_with_report_with_policy(vector, view, limit, AnnSearchPolicy::default())
    }

    pub fn search_vector_with_report_with_policy(
        &self,
        vector: &[i16],
        view: &AgentView,
        limit: SearchLimit,
        policy: AnnSearchPolicy,
    ) -> EngineResult<DatabaseSearchOutcome> {
        self.search_cells_with_report_with_policy(
            SearchQuery {
                text: "",
                vector: Some(vector),
                limit: limit.0,
                mode: SearchMode::Vector,
            },
            view,
            Some(policy),
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
        policy: Option<AnnSearchPolicy>,
    ) -> EngineResult<Option<DatabaseSearchOutcome>> {
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
        let mut ann_report = None;
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
                    return Ok(Some(DatabaseSearchOutcome {
                        results: Vec::new(),
                        ann_report: None,
                    }));
                };
                let index = self.persisted_vector_index()?;
                let graph = self.persisted_hnsw_graph()?;
                let outcome = match policy {
                    Some(policy) => search_persisted_ann_with_policy(
                        &index.vectors,
                        &graph,
                        vector,
                        &allowed,
                        query.limit,
                        policy,
                    ),
                    None => super::ann::search_persisted_ann(
                        &index.vectors,
                        &graph,
                        vector,
                        &allowed,
                        query.limit,
                    ),
                };
                ann_report = Some(outcome.report);
                outcome.results
            }
            SearchMode::VectorExact => {
                let Some(vector) = query.vector else {
                    return Ok(Some(DatabaseSearchOutcome {
                        results: Vec::new(),
                        ann_report: None,
                    }));
                };
                let index = self.persisted_vector_index()?;
                search_persisted_vectors(
                    &index.vectors,
                    vector,
                    &allowed,
                    query.limit,
                    &DistanceMetric::default(),
                )
            }
            SearchMode::Hybrid => return Ok(None),
        };
        let txn = self.read_txn();
        Ok(Some(DatabaseSearchOutcome {
            results: ranked
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
            ann_report,
        }))
    }

    pub fn search_cells(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        Ok(self.search_cells_with_report(query, view)?.results)
    }

    pub fn search_cells_with_report(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
    ) -> EngineResult<DatabaseSearchOutcome> {
        self.search_cells_with_report_with_policy(query, view, None)
    }

    pub fn search_cells_with_report_with_policy(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
        policy: Option<AnnSearchPolicy>,
    ) -> EngineResult<DatabaseSearchOutcome> {
        if let Some(results) = self.search_persisted_query(query, view, policy)? {
            return Ok(results);
        }
        let mut indexes = SearchIndexes::default();
        let mut cells = BTreeMap::<u32, (CellId, Vec<u8>)>::new();
        let mut vector_candidates = 0usize;
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
                vector_candidates += 1;
            }
            cells.insert(candidate, (version.cell_id, version.payload));
        }
        let results = indexes
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
            .collect::<Vec<_>>();
        let ann_report = snapshot_ann_report(
            self,
            query,
            vector_candidates,
            results.len(),
            policy.unwrap_or_default(),
        );
        Ok(DatabaseSearchOutcome {
            results,
            ann_report,
        })
    }

    pub fn search_diagnostics(&self, query: &str) -> EngineResult<String> {
        let terms = crate::search::tokenize(query);
        Ok(format!(
            "query_terms_count={} terms=[{}]",
            terms.len(),
            terms.join(", ")
        ))
    }
}

fn snapshot_ann_report(
    db: &Database,
    query: SearchQuery<'_>,
    allowed_candidates: usize,
    returned_candidates: usize,
    policy: AnnSearchPolicy,
) -> Option<AnnSearchReport> {
    if query.mode != SearchMode::Vector {
        return None;
    }
    let reason = if db.manifest().live_segments.is_empty() {
        AnnFallbackReason::NoPersistedSegments
    } else {
        AnnFallbackReason::UncheckpointedChanges
    };
    Some(finalize_report(
        AnnSearchReport {
            path: AnnSearchPath::ExactFallback,
            fallback_reason: Some(reason),
            fallback_performed: true,
            requested_limit: query.limit,
            allowed_candidates,
            graph_nodes: 0,
            returned_candidates,
            visited_candidates: 0,
            max_visited_candidates: policy.max_visited_candidates,
            recall_q16: None,
            min_recall_q16: policy.min_recall_q16,
            hnsw_max_neighbors: 0,
            hnsw_ef_search: 0,
            hnsw_layer_count: 0,
            upper_graph_edges: 0,
            require_slo: policy.require_slo,
            production_safe: true,
            slo_violations: Vec::new(),
        },
        policy,
    ))
}
