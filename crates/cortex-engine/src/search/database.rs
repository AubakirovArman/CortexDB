use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::memtable::CellVersion;
use cortex_core::CellId;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::{scope_id, CellMetadata};

use super::access::allowed_candidates;
use super::ann::{search_persisted_ann_with_policy, AnnSearchPolicy, AnnSearchReport};
use super::persisted::{
    search_persisted_lexical, search_persisted_vectors, PersistedLexicalSearchIndex,
};
use super::synonyms::expand_query_with_corpus_synonyms;
use super::{
    classify_search_query_intent, HybridRrfWeights, ScoredCandidate, SearchIndexes, SearchMode,
    SearchQuery, SearchQueryIntent, SearchReranker, WeightedScoreReranker,
};

const MAX_CORPUS_SYNONYM_QUERY_TERMS: usize = 12;
const MAX_CORPUS_SYNONYMS_PER_QUERY_TERM: usize = 2;

mod ann_reports;
mod context;
mod diversity;
mod persisted_rrf;
mod ranking;
#[cfg(test)]
mod tests;

use self::ann_reports::{
    persisted_exact_fallback_report, persisted_graph_is_stale,
    persisted_hnsw_fault_fallback_report, persisted_hnsw_stale_fallback_report,
    snapshot_ann_report,
};
use self::context::{
    high_level_anchor_score, is_search_parent_context_metadata, project_context_score,
    search_parent_lookup_keys,
};
use self::diversity::select_diverse_results;
use self::persisted_rrf::{distance_metric_from_manifest, fuse_persisted_rrf};
use self::ranking::{
    best_payload_vector_for_query, database_index_query, rerank_database_results,
    search_rerank_candidate_limit, search_rerank_result_limit,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PersistedSearchCandidate {
    candidate_id: u32,
    score: u64,
    lexical_score: u64,
    vector_score: u64,
}

fn metadata_for_version(version: &CellVersion) -> CellMetadata {
    CellMetadata::from_payload_with_descriptor(&version.payload, &version.descriptor)
}

impl PersistedSearchCandidate {
    fn from_lexical(candidate: ScoredCandidate) -> Self {
        Self {
            candidate_id: candidate.cell_id,
            score: candidate.score,
            lexical_score: candidate.score,
            vector_score: 0,
        }
    }

    fn from_vector(candidate: ScoredCandidate) -> Self {
        Self {
            candidate_id: candidate.cell_id,
            score: candidate.score,
            lexical_score: 0,
            vector_score: candidate.score,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchLimit(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseSearchResult {
    pub cell_id: CellId,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub metadata: CellMetadata,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseSearchOutcome {
    pub results: Vec<DatabaseSearchResult>,
    pub ann_report: Option<AnnSearchReport>,
    pub view_traces: Vec<SearchViewTrace>,
    pub diversity_diagnostics: Option<SearchDiversityDiagnostics>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchViewTrace {
    pub cell_id: CellId,
    pub candidate_id: u32,
    pub vector_view: Option<String>,
    pub vector_score: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchDiversityDiagnostics {
    pub intent: SearchQueryIntent,
    pub diversity_enabled: bool,
    pub lambda_q16: u16,
    pub input_candidates: usize,
    pub output_candidates: usize,
    pub skipped_candidates: usize,
    pub max_payload_similarity_q16: u64,
    pub max_cluster_similarity_q16: u64,
    pub selected_with_payload_similarity: usize,
    pub selected_with_cluster_similarity: usize,
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
            trace_search("persisted search skipped: no live segments");
            return Ok(None);
        }
        let checkpoint_seq = cortex_core::CommitSeq(self.manifest().checkpoint_seq);
        let changed_after_checkpoint = self.memtable.changed_cell_ids_after(checkpoint_seq);
        if !changed_after_checkpoint.is_empty() {
            trace_search(&format!(
                "persisted search skipped: {} changed cells after checkpoint seq {}",
                changed_after_checkpoint.len(),
                checkpoint_seq.0
            ));
            return Ok(None);
        }
        trace_search(&format!(
            "persisted search begin mode={:?} limit={}",
            query.mode, query.limit
        ));
        let state = self.persisted_index_state_cached()?;
        let allowed = allowed_candidates(&state.bitmap, view);
        let expanded_query_text = self.corpus_synonym_expanded_query_text(query.text)?;
        let lexical_query_text = expanded_query_text.as_deref().unwrap_or(query.text);
        let mut ann_report = None;
        let ranked = match query.mode {
            SearchMode::Keyword => search_persisted_lexical(
                PersistedLexicalSearchIndex {
                    terms: &state.lexical.terms,
                    doc_lengths: &state.lexical.doc_lengths,
                    term_frequencies: &state.lexical.term_frequencies,
                    field_doc_lengths: &state.lexical.field_doc_lengths,
                    field_term_frequencies: &state.lexical.field_term_frequencies,
                },
                lexical_query_text,
                &allowed,
                query.limit,
            )
            .into_iter()
            .map(PersistedSearchCandidate::from_lexical)
            .collect(),
            SearchMode::Vector => {
                let Some(vector) = query.vector else {
                    return Ok(Some(DatabaseSearchOutcome {
                        results: Vec::new(),
                        ann_report: None,
                        view_traces: Vec::new(),
                        diversity_diagnostics: None,
                    }));
                };
                let index = self.persisted_vector_index()?;
                if !self.feature_flags.experimental_hnsw {
                    let metric = self
                        .manifest()
                        .vector_profile
                        .map(|profile| distance_metric_from_manifest(profile.metric))
                        .unwrap_or_default();
                    let ranked = search_persisted_vectors(
                        &index.vectors,
                        vector,
                        &allowed,
                        query.limit,
                        &metric,
                    );
                    ann_report = Some(persisted_exact_fallback_report(
                        query.limit,
                        allowed.len(),
                        ranked.len(),
                        policy.unwrap_or_default(),
                    ));
                    ranked
                } else {
                    let outcome = match self.persisted_hnsw_graph() {
                        Ok(graph) => {
                            let effective_policy = policy.unwrap_or_default();
                            if persisted_graph_is_stale(&index.vectors, &graph, vector, &allowed) {
                                let metric = self
                                    .manifest()
                                    .vector_profile
                                    .map(|profile| distance_metric_from_manifest(profile.metric))
                                    .unwrap_or_default();
                                let ranked = search_persisted_vectors(
                                    &index.vectors,
                                    vector,
                                    &allowed,
                                    query.limit,
                                    &metric,
                                );
                                let report = persisted_hnsw_stale_fallback_report(
                                    &graph,
                                    query.limit,
                                    allowed.len(),
                                    ranked.len(),
                                    effective_policy,
                                );
                                super::ann::AnnSearchOutcome {
                                    results: ranked,
                                    report,
                                }
                            } else {
                                match policy {
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
                                }
                            }
                        }
                        Err(_) => {
                            let metric = self
                                .manifest()
                                .vector_profile
                                .map(|profile| distance_metric_from_manifest(profile.metric))
                                .unwrap_or_default();
                            let ranked = search_persisted_vectors(
                                &index.vectors,
                                vector,
                                &allowed,
                                query.limit,
                                &metric,
                            );
                            let policy = policy.unwrap_or_default();
                            let report = persisted_hnsw_fault_fallback_report(
                                query.limit,
                                allowed.len(),
                                ranked.len(),
                                policy,
                            );
                            super::ann::AnnSearchOutcome {
                                results: ranked,
                                report,
                            }
                        }
                    };
                    ann_report = Some(outcome.report);
                    outcome.results
                }
                .into_iter()
                .map(PersistedSearchCandidate::from_vector)
                .collect()
            }
            SearchMode::VectorExact => {
                let Some(vector) = query.vector else {
                    return Ok(Some(DatabaseSearchOutcome {
                        results: Vec::new(),
                        ann_report: None,
                        view_traces: Vec::new(),
                        diversity_diagnostics: None,
                    }));
                };
                let index = self.persisted_vector_index()?;
                let metric = self
                    .manifest()
                    .vector_profile
                    .map(|profile| distance_metric_from_manifest(profile.metric))
                    .unwrap_or_default();
                search_persisted_vectors(&index.vectors, vector, &allowed, query.limit, &metric)
                    .into_iter()
                    .map(PersistedSearchCandidate::from_vector)
                    .collect()
            }
            SearchMode::Hybrid | SearchMode::HybridRerank => {
                let depth = if query.mode == SearchMode::HybridRerank {
                    search_rerank_candidate_limit(query)
                } else {
                    query.limit
                };
                if let Some(vector) = query.vector {
                    let index = self.persisted_vector_index()?;
                    let metric = self
                        .manifest()
                        .vector_profile
                        .map(|profile| distance_metric_from_manifest(profile.metric))
                        .unwrap_or_default();
                    let lexical = search_persisted_lexical(
                        PersistedLexicalSearchIndex {
                            terms: &state.lexical.terms,
                            doc_lengths: &state.lexical.doc_lengths,
                            term_frequencies: &state.lexical.term_frequencies,
                            field_doc_lengths: &state.lexical.field_doc_lengths,
                            field_term_frequencies: &state.lexical.field_term_frequencies,
                        },
                        lexical_query_text,
                        &allowed,
                        depth,
                    );
                    let vector =
                        search_persisted_vectors(&index.vectors, vector, &allowed, depth, &metric);
                    fuse_persisted_rrf(lexical, vector, depth, HybridRrfWeights::balanced())
                } else {
                    search_persisted_lexical(
                        PersistedLexicalSearchIndex {
                            terms: &state.lexical.terms,
                            doc_lengths: &state.lexical.doc_lengths,
                            term_frequencies: &state.lexical.term_frequencies,
                            field_doc_lengths: &state.lexical.field_doc_lengths,
                            field_term_frequencies: &state.lexical.field_term_frequencies,
                        },
                        lexical_query_text,
                        &allowed,
                        depth,
                    )
                    .into_iter()
                    .map(PersistedSearchCandidate::from_lexical)
                    .collect()
                }
            }
        };
        let pin = self.pin_read_txn();
        let txn = pin.read_txn();
        let mut results = ranked
            .into_iter()
            .filter_map(|candidate| {
                let cell_id = state.candidate_to_cell.get(&candidate.candidate_id)?;
                self.memtable.read(txn, *cell_id).map(|version| {
                    let metadata = metadata_for_version(version);
                    DatabaseSearchResult {
                        cell_id: *cell_id,
                        score: candidate.score,
                        lexical_score: candidate.lexical_score,
                        vector_score: candidate.vector_score,
                        metadata,
                        payload: version.payload.clone(),
                    }
                })
            })
            .collect::<Vec<_>>();
        let mut diversity_diagnostics = None;
        if query.mode == SearchMode::HybridRerank {
            rerank_database_results(&mut results, query, &WeightedScoreReranker::default());
            let selection = select_diverse_results(results, query);
            results = selection.results;
            diversity_diagnostics = Some(selection.diagnostics);
        }
        let results = self.expand_high_level_anchor_context(results, view, query);
        let results = self.expand_project_related_context(results, view, query);
        let results = self.expand_search_parent_context(results, view, query.limit);
        trace_search(&format!(
            "persisted search done mode={:?} results={}",
            query.mode,
            results.len()
        ));
        Ok(Some(DatabaseSearchOutcome {
            results,
            ann_report,
            view_traces: Vec::new(),
            diversity_diagnostics,
        }))
    }

    pub fn search_cells(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        Ok(self.search_cells_with_report(query, view)?.results)
    }

    pub fn search_cells_with_reranker(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
        reranker: &dyn SearchReranker,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        let mut results = self
            .search_cells_with_report(
                SearchQuery {
                    limit: search_rerank_candidate_limit(query),
                    ..query
                },
                view,
            )?
            .results;
        rerank_database_results(&mut results, query, reranker);
        results.truncate(search_rerank_result_limit(query));
        Ok(results)
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
        trace_search(&format!(
            "snapshot search rebuild begin mode={:?} limit={}",
            query.mode, query.limit
        ));
        let mut indexes = SearchIndexes::default();
        let mut cells = BTreeMap::<u32, (CellId, Vec<u8>, CellMetadata)>::new();
        let mut traces = BTreeMap::<u32, SearchViewTrace>::new();
        let mut vector_candidates = 0usize;
        for (index, (version, metadata)) in self
            .snapshot_versions()
            .into_iter()
            .filter_map(|version| {
                let metadata = metadata_for_version(&version);
                view.can_read_scope(scope_id(&metadata.scope))
                    .then_some((version, metadata))
            })
            .enumerate()
        {
            let candidate =
                u32::try_from(index + 1).map_err(|_| EngineError::CandidateIdOverflow)?;
            indexes.add_field_terms(candidate, metadata.lexical_field_terms());
            if let Some(best) = best_payload_vector_for_query(&version.payload, query.vector) {
                traces.insert(
                    candidate,
                    SearchViewTrace {
                        cell_id: version.cell_id,
                        candidate_id: candidate,
                        vector_view: best.view_name,
                        vector_score: best.score,
                    },
                );
                indexes.add_vector(candidate, best.vector);
                vector_candidates += 1;
            }
            cells.insert(candidate, (version.cell_id, version.payload, metadata));
        }
        let expanded_query_text = self.corpus_synonym_expanded_query_text(query.text)?;
        let index_query_text = expanded_query_text.as_deref().unwrap_or(query.text);
        let index_query = database_index_query(SearchQuery {
            text: index_query_text,
            ..query
        });
        trace_search(&format!(
            "snapshot search indexed cells={} vector_candidates={}",
            cells.len(),
            vector_candidates
        ));
        let indexed_results = indexes
            .search(index_query)
            .into_iter()
            .filter_map(|result| {
                let candidate_id = result.cell_id;
                let (cell_id, payload, metadata) = cells.remove(&candidate_id)?;
                Some((
                    candidate_id,
                    DatabaseSearchResult {
                        cell_id,
                        score: result.score,
                        lexical_score: result.lexical_score,
                        vector_score: result.vector_score,
                        metadata,
                        payload,
                    },
                ))
            })
            .collect::<Vec<_>>();
        let traces_by_cell = indexed_results
            .iter()
            .filter_map(|(candidate_id, result)| {
                traces
                    .get(candidate_id)
                    .cloned()
                    .map(|trace| (result.cell_id, trace))
            })
            .collect::<BTreeMap<_, _>>();
        let mut raw_results = indexed_results
            .into_iter()
            .map(|(_, result)| result)
            .collect::<Vec<_>>();
        let mut diversity_diagnostics = None;
        if query.mode == SearchMode::HybridRerank {
            rerank_database_results(&mut raw_results, query, &WeightedScoreReranker::default());
            let selection = select_diverse_results(raw_results, query);
            raw_results = selection.results;
            diversity_diagnostics = Some(selection.diagnostics);
        }
        let expanded_results = self.expand_high_level_anchor_context(raw_results, view, query);
        let expanded_results = self.expand_project_related_context(expanded_results, view, query);
        let expanded_results =
            self.expand_search_parent_context(expanded_results, view, query.limit);
        let view_traces = expanded_results
            .iter()
            .filter_map(|result| traces_by_cell.get(&result.cell_id).cloned())
            .collect::<Vec<_>>();
        let ann_report = snapshot_ann_report(
            self,
            query,
            vector_candidates,
            expanded_results.len(),
            policy.unwrap_or_default(),
        );
        Ok(DatabaseSearchOutcome {
            results: expanded_results,
            ann_report,
            view_traces,
            diversity_diagnostics,
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

    fn corpus_synonym_expanded_query_text(&self, query: &str) -> EngineResult<Option<String>> {
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

    fn expand_search_parent_context(
        &self,
        results: Vec<DatabaseSearchResult>,
        view: &AgentView,
        limit: usize,
    ) -> Vec<DatabaseSearchResult> {
        if results.is_empty() || limit == 0 {
            return Vec::new();
        }
        if results.len() >= limit {
            return results.into_iter().take(limit).collect();
        }
        let parents = self.search_parent_context_candidates(view);
        if parents.is_empty() {
            return results.into_iter().take(limit).collect();
        }

        let mut expanded = Vec::with_capacity(limit);
        let mut emitted = BTreeSet::<CellId>::new();
        for result in results {
            if emitted.insert(result.cell_id) {
                expanded.push(result.clone());
            }
            if expanded.len() >= limit {
                break;
            }
            for key in search_parent_lookup_keys(&result.metadata) {
                let Some(parent) = parents.get(&key) else {
                    continue;
                };
                if parent.cell_id == result.cell_id || !emitted.insert(parent.cell_id) {
                    continue;
                }
                let mut parent = parent.clone();
                parent.score = parent.score.max(result.score.saturating_sub(1));
                expanded.push(parent);
                break;
            }
            if expanded.len() >= limit {
                break;
            }
        }
        expanded
    }

    fn expand_high_level_anchor_context(
        &self,
        results: Vec<DatabaseSearchResult>,
        view: &AgentView,
        query: SearchQuery<'_>,
    ) -> Vec<DatabaseSearchResult> {
        if query.limit == 0
            || classify_search_query_intent(query.text) != SearchQueryIntent::HighLevel
        {
            return results.into_iter().take(query.limit).collect();
        }

        let anchors = self.high_level_anchor_candidates(view);
        if anchors.is_empty() {
            return results.into_iter().take(query.limit).collect();
        }

        let mut expanded = Vec::with_capacity(query.limit);
        let mut emitted = BTreeSet::<CellId>::new();
        for anchor in anchors {
            if emitted.insert(anchor.cell_id) {
                expanded.push(anchor);
            }
            if expanded.len() >= query.limit {
                return expanded;
            }
        }
        for result in results {
            if emitted.insert(result.cell_id) {
                expanded.push(result);
            }
            if expanded.len() >= query.limit {
                break;
            }
        }
        expanded
    }

    fn expand_project_related_context(
        &self,
        results: Vec<DatabaseSearchResult>,
        view: &AgentView,
        query: SearchQuery<'_>,
    ) -> Vec<DatabaseSearchResult> {
        if query.limit == 0
            || classify_search_query_intent(query.text) != SearchQueryIntent::ProjectRelated
        {
            return results.into_iter().take(query.limit).collect();
        }
        let projects = results
            .iter()
            .filter_map(|result| result.metadata.project.clone())
            .collect::<BTreeSet<_>>();
        if projects.is_empty() {
            return results.into_iter().take(query.limit).collect();
        }

        let project_candidates = self.project_context_candidates(view, &projects);
        let mut expanded = Vec::with_capacity(query.limit);
        let mut emitted = BTreeSet::<CellId>::new();
        for result in results {
            if emitted.insert(result.cell_id) {
                expanded.push(result);
            }
            if expanded.len() >= query.limit {
                return expanded;
            }
        }
        for candidate in project_candidates {
            if emitted.insert(candidate.cell_id) {
                expanded.push(candidate);
            }
            if expanded.len() >= query.limit {
                break;
            }
        }
        expanded
    }

    fn project_context_candidates(
        &self,
        view: &AgentView,
        projects: &BTreeSet<String>,
    ) -> Vec<DatabaseSearchResult> {
        let mut candidates = self
            .snapshot_versions()
            .into_iter()
            .filter_map(|version| {
                let metadata = metadata_for_version(&version);
                if !view.can_read_scope(scope_id(&metadata.scope))
                    || !metadata
                        .project
                        .as_ref()
                        .is_some_and(|project| projects.contains(project))
                {
                    return None;
                }
                let score = project_context_score(&metadata);
                Some(DatabaseSearchResult {
                    cell_id: version.cell_id,
                    score,
                    lexical_score: score,
                    vector_score: 0,
                    metadata,
                    payload: version.payload,
                })
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|result| (Reverse(result.score), result.cell_id.0));
        candidates
    }

    fn high_level_anchor_candidates(&self, view: &AgentView) -> Vec<DatabaseSearchResult> {
        let mut candidates = self
            .snapshot_versions()
            .into_iter()
            .filter_map(|version| {
                let metadata = metadata_for_version(&version);
                if !view.can_read_scope(scope_id(&metadata.scope)) {
                    return None;
                }
                let score = high_level_anchor_score(&metadata);
                (score > 0).then_some(DatabaseSearchResult {
                    cell_id: version.cell_id,
                    score,
                    lexical_score: score,
                    vector_score: 0,
                    metadata,
                    payload: version.payload,
                })
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|result| (Reverse(result.score), result.cell_id.0));
        candidates
    }

    fn search_parent_context_candidates(
        &self,
        view: &AgentView,
    ) -> BTreeMap<String, DatabaseSearchResult> {
        let mut parents = BTreeMap::new();
        for version in self.snapshot_versions() {
            let metadata = metadata_for_version(&version);
            if !view.can_read_scope(scope_id(&metadata.scope))
                || !is_search_parent_context_metadata(&metadata)
            {
                continue;
            }
            let result = DatabaseSearchResult {
                cell_id: version.cell_id,
                score: 0,
                lexical_score: 0,
                vector_score: 0,
                metadata: metadata.clone(),
                payload: version.payload,
            };
            if let Some(chunk_id) = &metadata.chunk_id {
                parents.entry(chunk_id.clone()).or_insert(result.clone());
            }
            if let Some(document_id) = &metadata.document_id {
                parents.entry(document_id.clone()).or_insert(result);
            }
        }
        parents
    }
}

fn trace_search(message: &str) {
    if std::env::var_os("CORTEXDB_SEARCH_TRACE").is_some() {
        eprintln!("[cortexdb-search-trace] {message}");
    }
}
