use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::{scope_id, CellMetadata};
use crate::source_trust::SourceTrust;

use super::access::allowed_candidates;
use super::ann::{
    finalize_report, search_persisted_ann_with_policy, AnnFallbackReason, AnnSearchPath,
    AnnSearchPolicy, AnnSearchReport,
};
use super::hnsw::DistanceMetric;
use super::persisted::{
    search_persisted_lexical, search_persisted_vectors, PersistedLexicalSearchIndex,
};
use super::synonyms::expand_query_with_corpus_synonyms;
use super::vector::{vector_from_payload, vectors_from_payload};
use super::{
    calibrated_hybrid_rrf_weights, classify_search_query_intent, extract_query_conditions,
    route_policy_for_query, routed_candidate_limit, routed_result_limit, HybridRrfWeights,
    ScoredCandidate, SearchIndexes, SearchMode, SearchQuery, SearchQueryIntent, SearchRerankInput,
    SearchReranker, WeightedScoreReranker,
};

const MAX_CORPUS_SYNONYM_QUERY_TERMS: usize = 12;
const MAX_CORPUS_SYNONYMS_PER_QUERY_TERM: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PersistedSearchCandidate {
    candidate_id: u32,
    score: u64,
    lexical_score: u64,
    vector_score: u64,
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
                    let weights = if query.mode == SearchMode::HybridRerank {
                        calibrated_hybrid_rrf_weights(query.text)
                    } else {
                        HybridRrfWeights::balanced()
                    };
                    fuse_persisted_rrf(lexical, vector, depth, weights)
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
        let txn = self.read_txn();
        let mut results = ranked
            .into_iter()
            .filter_map(|candidate| {
                let cell_id = state.candidate_to_cell.get(&candidate.candidate_id)?;
                self.get_cell(txn, *cell_id)
                    .map(|payload| DatabaseSearchResult {
                        cell_id: *cell_id,
                        score: candidate.score,
                        lexical_score: candidate.lexical_score,
                        vector_score: candidate.vector_score,
                        payload,
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
        let mut cells = BTreeMap::<u32, (CellId, Vec<u8>)>::new();
        let mut traces = BTreeMap::<u32, SearchViewTrace>::new();
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
            cells.insert(candidate, (version.cell_id, version.payload));
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
                let (cell_id, payload) = cells.remove(&candidate_id)?;
                Some((
                    candidate_id,
                    DatabaseSearchResult {
                        cell_id,
                        score: result.score,
                        lexical_score: result.lexical_score,
                        vector_score: result.vector_score,
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
            let result_metadata = CellMetadata::from_payload(&result.payload);
            if emitted.insert(result.cell_id) {
                expanded.push(result.clone());
            }
            if expanded.len() >= limit {
                break;
            }
            for key in search_parent_lookup_keys(&result_metadata) {
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
            .filter_map(|result| CellMetadata::from_payload(&result.payload).project)
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
                let metadata = CellMetadata::from_payload(&version.payload);
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
                let metadata = CellMetadata::from_payload(&version.payload);
                if !view.can_read_scope(scope_id(&metadata.scope)) {
                    return None;
                }
                let score = high_level_anchor_score(&metadata);
                (score > 0).then_some(DatabaseSearchResult {
                    cell_id: version.cell_id,
                    score,
                    lexical_score: score,
                    vector_score: 0,
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
            let metadata = CellMetadata::from_payload(&version.payload);
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

fn database_index_query(query: SearchQuery<'_>) -> SearchQuery<'_> {
    if query.mode == SearchMode::HybridRerank {
        SearchQuery {
            mode: SearchMode::Hybrid,
            limit: search_rerank_candidate_limit(query),
            ..query
        }
    } else {
        query
    }
}

fn search_rerank_candidate_limit(query: SearchQuery<'_>) -> usize {
    routed_candidate_limit(query.text, query.limit).max(query.limit.max(32))
}

fn search_rerank_result_limit(query: SearchQuery<'_>) -> usize {
    routed_result_limit(query.text, query.limit)
}

fn select_diverse_results(
    mut results: Vec<DatabaseSearchResult>,
    query: SearchQuery<'_>,
) -> DiverseSelection {
    let result_limit = search_rerank_result_limit(query);
    let route_policy = route_policy_for_query(query.text);
    let mut diagnostics = SearchDiversityDiagnostics {
        intent: classify_search_query_intent(query.text),
        diversity_enabled: route_policy.diversity,
        lambda_q16: route_policy.diversity_lambda_q16,
        input_candidates: results.len(),
        output_candidates: 0,
        skipped_candidates: 0,
        max_payload_similarity_q16: 0,
        max_cluster_similarity_q16: 0,
        selected_with_payload_similarity: 0,
        selected_with_cluster_similarity: 0,
    };
    if results.len() <= result_limit {
        diagnostics.output_candidates = results.len();
        return DiverseSelection {
            results,
            diagnostics,
        };
    }
    if !route_policy.diversity {
        results.truncate(result_limit);
        diagnostics.output_candidates = results.len();
        diagnostics.skipped_candidates = diagnostics
            .input_candidates
            .saturating_sub(diagnostics.output_candidates);
        return DiverseSelection {
            results,
            diagnostics,
        };
    }

    let mut selected = Vec::<DatabaseSearchResult>::with_capacity(result_limit);
    while !results.is_empty() && selected.len() < result_limit {
        let mut best = None::<(usize, DiversitySimilarity, u64)>;
        for (index, candidate) in results.iter().enumerate() {
            let similarity = diversity_similarity_q16(candidate, &selected);
            diagnostics.max_payload_similarity_q16 = diagnostics
                .max_payload_similarity_q16
                .max(similarity.payload_q16);
            diagnostics.max_cluster_similarity_q16 = diagnostics
                .max_cluster_similarity_q16
                .max(similarity.cluster_q16);
            let score = mmr_diversity_score(
                candidate,
                similarity.max_q16(),
                route_policy.diversity_lambda_q16,
            );
            if best
                .as_ref()
                .is_none_or(|(_, _, best_score)| score > *best_score)
            {
                best = Some((index, similarity, score));
            }
        }
        let (best_index, best_similarity, _) =
            best.unwrap_or((0, DiversitySimilarity::default(), 0));
        if !selected.is_empty() && best_similarity.payload_q16 > 0 {
            diagnostics.selected_with_payload_similarity += 1;
        }
        if !selected.is_empty() && best_similarity.cluster_q16 > 0 {
            diagnostics.selected_with_cluster_similarity += 1;
        }
        selected.push(results.remove(best_index));
    }
    diagnostics.output_candidates = selected.len();
    diagnostics.skipped_candidates = diagnostics
        .input_candidates
        .saturating_sub(diagnostics.output_candidates);
    DiverseSelection {
        results: selected,
        diagnostics,
    }
}

fn mmr_diversity_score(
    candidate: &DatabaseSearchResult,
    similarity_q16: u64,
    lambda_q16: u16,
) -> u64 {
    let relevance = u128::from(candidate.score).saturating_mul(u128::from(lambda_q16)) / 65_535;
    let diversity_weight = 65_535u16.saturating_sub(lambda_q16);
    let redundancy_penalty = u128::from(candidate.score)
        .saturating_mul(u128::from(diversity_weight))
        .saturating_mul(u128::from(similarity_q16))
        / (65_535u128 * 65_535u128);
    u64::try_from(relevance.saturating_sub(redundancy_penalty)).unwrap_or(u64::MAX)
}

struct DiverseSelection {
    results: Vec<DatabaseSearchResult>,
    diagnostics: SearchDiversityDiagnostics,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DiversitySimilarity {
    payload_q16: u64,
    cluster_q16: u64,
}

impl DiversitySimilarity {
    fn max_q16(self) -> u64 {
        self.payload_q16.max(self.cluster_q16)
    }
}

fn diversity_similarity_q16(
    candidate: &DatabaseSearchResult,
    selected: &[DatabaseSearchResult],
) -> DiversitySimilarity {
    selected
        .iter()
        .map(|existing| DiversitySimilarity {
            payload_q16: payload_jaccard_q16(&candidate.payload, &existing.payload),
            cluster_q16: metadata_cluster_similarity_q16(&candidate.payload, &existing.payload),
        })
        .max_by_key(|similarity| similarity.max_q16())
        .unwrap_or_default()
}

fn payload_jaccard_q16(left: &[u8], right: &[u8]) -> u64 {
    let left = payload_terms(left);
    let right = payload_terms(right);
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    let intersection = left.intersection(&right).count() as u64;
    let union = left.union(&right).count() as u64;
    intersection.saturating_mul(65_535) / union.max(1)
}

fn payload_terms(payload: &[u8]) -> BTreeSet<String> {
    CellMetadata::from_payload(payload)
        .terms
        .into_iter()
        .filter(|term| term.len() >= 3)
        .collect()
}

fn metadata_cluster_similarity_q16(left: &[u8], right: &[u8]) -> u64 {
    let left = CellMetadata::from_payload(left);
    let right = CellMetadata::from_payload(right);
    let mut score = 0;
    score = score.max(matching_cluster_score(
        left.content_hash.as_deref(),
        right.content_hash.as_deref(),
        65_535,
    ));
    score = score.max(matching_cluster_score(
        left.document_id.as_deref(),
        right.document_id.as_deref(),
        65_535,
    ));
    score = score.max(matching_cluster_score(
        left.parent_id.as_deref(),
        right.parent_id.as_deref(),
        58_982,
    ));
    score = score.max(matching_cluster_score(
        left.source_hash.as_deref(),
        right.source_hash.as_deref(),
        52_428,
    ));
    score = score.max(matching_cluster_score(
        left.path.as_deref(),
        right.path.as_deref(),
        49_152,
    ));
    score = score.max(matching_cluster_score(
        left.project.as_deref(),
        right.project.as_deref(),
        36_864,
    ));
    score = score.max(matching_cluster_score(
        left.entity.as_deref(),
        right.entity.as_deref(),
        32_768,
    ));
    score = score.max(matching_cluster_score(
        left.topic.as_deref(),
        right.topic.as_deref(),
        24_576,
    ));
    score = score.max(matching_cluster_score(
        left.source.as_deref(),
        right.source.as_deref(),
        16_384,
    ));
    score
}

fn matching_cluster_score(left: Option<&str>, right: Option<&str>, score: u64) -> u64 {
    match (left, right) {
        (Some(left), Some(right)) if !left.trim().is_empty() && left == right => score,
        _ => 0,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BestPayloadVector {
    view_name: Option<String>,
    vector: Vec<i16>,
    score: u64,
}

fn best_payload_vector_for_query(
    payload: &[u8],
    query_vector: Option<&[i16]>,
) -> Option<BestPayloadVector> {
    let Some(query_vector) = query_vector else {
        return vector_from_payload(payload).map(|vector| BestPayloadVector {
            view_name: Some("body".to_owned()),
            vector,
            score: 0,
        });
    };
    vectors_from_payload(payload)
        .into_iter()
        .filter(|view| view.vector.len() == query_vector.len())
        .filter_map(|view| {
            let score = DistanceMetric::DotProduct.distance(query_vector, &view.vector)?;
            Some(BestPayloadVector {
                view_name: Some(view.name),
                vector: view.vector,
                score,
            })
        })
        .max_by_key(|view| view.score)
}

fn search_parent_lookup_keys(metadata: &CellMetadata) -> Vec<String> {
    let mut keys = BTreeSet::new();
    if let Some(parent_id) = &metadata.parent_id {
        keys.insert(parent_id.clone());
    }
    if !is_search_parent_context_metadata(metadata) {
        if let Some(document_id) = &metadata.document_id {
            keys.insert(document_id.clone());
        }
    }
    keys.into_iter().collect()
}

fn is_search_parent_context_metadata(metadata: &CellMetadata) -> bool {
    metadata
        .chunk_role
        .as_deref()
        .map(|role| {
            role.eq_ignore_ascii_case("parent")
                || role.eq_ignore_ascii_case("document")
                || role.eq_ignore_ascii_case("summary")
        })
        .unwrap_or(false)
}

fn high_level_anchor_score(metadata: &CellMetadata) -> u64 {
    let mut score = 0u64;
    if is_search_parent_context_metadata(metadata) {
        score = score.saturating_add(8_000);
    }
    for value in [
        metadata.title.as_deref(),
        metadata.path.as_deref(),
        metadata.document_id.as_deref(),
        metadata.source.as_deref(),
        Some(metadata.body_text.as_str()),
    ]
    .into_iter()
    .flatten()
    {
        let value = value.to_ascii_lowercase();
        for term in [
            "overview", "summary", "mission", "charter", "about", "strategy", "vision", "company",
        ] {
            if value.contains(term) {
                score = score.saturating_add(2_000);
            }
        }
    }
    score
}

fn project_context_score(metadata: &CellMetadata) -> u64 {
    let mut score = 1_000u64;
    if metadata.owner.is_some() {
        score = score.saturating_add(2_000);
    }
    if metadata.status_tag.is_some() {
        score = score.saturating_add(1_500);
    }
    if metadata.event_date.is_some() {
        score = score.saturating_add(1_000);
    }
    if metadata.title.is_some() {
        score = score.saturating_add(500);
    }
    score
}

fn rerank_database_results(
    results: &mut [DatabaseSearchResult],
    query: SearchQuery<'_>,
    reranker: &dyn SearchReranker,
) {
    let recency_scores = search_result_recency_scores_q16(results);
    for (index, result) in results.iter_mut().enumerate() {
        result.score = reranker
            .rerank_score(SearchRerankInput {
                query_text: query.text,
                query_vector: query.vector,
                candidate_id: result.cell_id.0,
                lexical_score: result.lexical_score,
                vector_score: result.vector_score,
                base_score: result.score,
                payload: Some(&result.payload),
            })
            .saturating_add(search_metadata_rerank_bonus(
                query.text,
                &result.payload,
                recency_scores.get(index).copied().unwrap_or(0),
            ));
    }
    results.sort_by_key(|result| (Reverse(result.score), result.cell_id.0));
}

fn search_result_recency_scores_q16(results: &[DatabaseSearchResult]) -> Vec<u64> {
    let created = results
        .iter()
        .map(|result| CellMetadata::from_payload(&result.payload).created_unix_seconds)
        .collect::<Vec<_>>();
    let mut timestamps = created.iter().filter_map(|value| *value);
    let Some(first) = timestamps.next() else {
        return vec![0; results.len()];
    };
    let (min, max) = timestamps.fold((first, first), |(min, max), value| {
        (min.min(value), max.max(value))
    });
    if max <= min {
        return created
            .into_iter()
            .map(|value| value.map(|_| u64::from(u16::MAX)).unwrap_or(0))
            .collect();
    }
    let span = max - min;
    created
        .into_iter()
        .map(|value| {
            value
                .map(|created| (created.saturating_sub(min).min(span) * u64::from(u16::MAX)) / span)
                .unwrap_or(0)
        })
        .collect()
}

fn search_metadata_rerank_bonus(query_text: &str, payload: &[u8], recency_q16: u64) -> u64 {
    let metadata = CellMetadata::from_payload(payload);
    let trust_q16 = u64::from(
        SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class).q16,
    );
    let (trust_weight_q16, freshness_weight_q16) = metadata_rerank_weights(query_text);
    weighted_metadata_component(trust_q16, trust_weight_q16).saturating_add(
        weighted_metadata_component(recency_q16, freshness_weight_q16),
    )
}

fn metadata_rerank_weights(query_text: &str) -> (u64, u64) {
    let temporal = extract_query_conditions(query_text)
        .temporal_range
        .is_some()
        || looks_temporal_or_current(query_text);
    match classify_search_query_intent(query_text) {
        SearchQueryIntent::ConflictingInfo => (18_000, 24_000),
        SearchQueryIntent::Constrained if temporal => (12_000, 22_000),
        _ if temporal => (8_000, 18_000),
        SearchQueryIntent::InfoNotFound => (4_000, 2_000),
        _ => (3_000, 3_000),
    }
}

fn looks_temporal_or_current(query_text: &str) -> bool {
    let lower = query_text.to_ascii_lowercase();
    [
        "latest",
        "current",
        "newest",
        "recent",
        "updated",
        "as of",
        "previously",
        "before",
        "after",
        "changed",
        "change",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn weighted_metadata_component(value_q16: u64, weight_q16: u64) -> u64 {
    value_q16.saturating_mul(1024).saturating_mul(weight_q16) / u64::from(u16::MAX)
}

fn distance_metric_from_manifest(value: u32) -> DistanceMetric {
    match value {
        1 => DistanceMetric::Cosine,
        2 => DistanceMetric::L2,
        _ => DistanceMetric::DotProduct,
    }
}

fn fuse_persisted_rrf(
    lexical: Vec<ScoredCandidate>,
    vector: Vec<ScoredCandidate>,
    limit: usize,
    weights: HybridRrfWeights,
) -> Vec<PersistedSearchCandidate> {
    let mut results = BTreeMap::<u32, PersistedSearchCandidate>::new();
    apply_persisted_rrf(&mut results, lexical, true, weights.lexical_q16);
    apply_persisted_rrf(&mut results, vector, false, weights.vector_q16);
    let mut values = results.into_values().collect::<Vec<_>>();
    values.sort_by_key(|candidate| (Reverse(candidate.score), candidate.candidate_id));
    values.truncate(limit);
    values
}

fn apply_persisted_rrf(
    results: &mut BTreeMap<u32, PersistedSearchCandidate>,
    ranked: Vec<ScoredCandidate>,
    lexical: bool,
    weight_q16: u32,
) {
    for (rank, candidate) in ranked.into_iter().enumerate() {
        let rrf =
            (1_000_000 / (60 + rank as u64 + 1)).saturating_mul(u64::from(weight_q16)) / 65_535;
        let result = results
            .entry(candidate.cell_id)
            .or_insert(PersistedSearchCandidate {
                candidate_id: candidate.cell_id,
                score: 0,
                lexical_score: 0,
                vector_score: 0,
            });
        result.score += rrf;
        if lexical {
            result.lexical_score = candidate.score;
        } else {
            result.vector_score = candidate.score;
        }
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
            hnsw_ef_construction: 0,
            upper_graph_edges: 0,
            require_slo: policy.require_slo,
            production_safe: true,
            slo_violations: Vec::new(),
        },
        policy,
    ))
}

fn persisted_exact_fallback_report(
    requested_limit: usize,
    allowed_candidates: usize,
    returned_candidates: usize,
    policy: AnnSearchPolicy,
) -> AnnSearchReport {
    finalize_report(
        AnnSearchReport {
            path: AnnSearchPath::ExactFallback,
            fallback_reason: Some(AnnFallbackReason::HnswDisabled),
            fallback_performed: true,
            requested_limit,
            allowed_candidates,
            graph_nodes: 0,
            returned_candidates,
            visited_candidates: allowed_candidates,
            max_visited_candidates: policy.max_visited_candidates,
            recall_q16: None,
            min_recall_q16: policy.min_recall_q16,
            hnsw_max_neighbors: 0,
            hnsw_ef_search: 0,
            hnsw_layer_count: 0,
            hnsw_ef_construction: 0,
            upper_graph_edges: 0,
            require_slo: policy.require_slo,
            production_safe: true,
            slo_violations: Vec::new(),
        },
        policy,
    )
}

fn persisted_hnsw_fault_fallback_report(
    requested_limit: usize,
    allowed_candidates: usize,
    returned_candidates: usize,
    policy: AnnSearchPolicy,
) -> AnnSearchReport {
    finalize_report(
        AnnSearchReport {
            path: AnnSearchPath::ExactFallback,
            fallback_reason: Some(AnnFallbackReason::InvalidGraph),
            fallback_performed: true,
            requested_limit,
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
            hnsw_ef_construction: 0,
            upper_graph_edges: 0,
            require_slo: policy.require_slo,
            production_safe: true,
            slo_violations: Vec::new(),
        },
        policy,
    )
}

fn persisted_hnsw_stale_fallback_report(
    graph: &cortex_storage::hnsw::HnswGraphIndex,
    requested_limit: usize,
    allowed_candidates: usize,
    returned_candidates: usize,
    policy: AnnSearchPolicy,
) -> AnnSearchReport {
    finalize_report(
        AnnSearchReport {
            path: AnnSearchPath::ExactFallback,
            fallback_reason: Some(AnnFallbackReason::StaleGraph),
            fallback_performed: true,
            requested_limit,
            allowed_candidates,
            graph_nodes: graph.links.len(),
            returned_candidates,
            visited_candidates: 0,
            max_visited_candidates: policy.max_visited_candidates,
            recall_q16: None,
            min_recall_q16: policy.min_recall_q16,
            hnsw_max_neighbors: graph.max_neighbors as usize,
            hnsw_ef_search: graph.ef_search as usize,
            hnsw_layer_count: graph.layer_count as usize,
            hnsw_ef_construction: graph.ef_construction as usize,
            upper_graph_edges: graph
                .upper_layers
                .values()
                .flat_map(|links| links.values())
                .map(|neighbors| neighbors.len())
                .sum(),
            require_slo: policy.require_slo,
            production_safe: true,
            slo_violations: Vec::new(),
        },
        policy,
    )
}

fn persisted_graph_is_stale(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &cortex_storage::hnsw::HnswGraphIndex,
    query: &[i16],
    allowed: &std::collections::BTreeSet<u32>,
) -> bool {
    vectors.iter().any(|(candidate, vector)| {
        allowed.contains(candidate)
            && vector.len() == query.len()
            && !graph.links.contains_key(candidate)
    })
}
