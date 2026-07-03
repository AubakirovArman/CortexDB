use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::database::Database;
use crate::error::EngineResult;

use super::super::ann::{
    search_persisted_ann_cached, search_persisted_ann_sampled, search_persisted_ann_with_policy,
    AnnSearchOutcome, AnnSearchPolicy, GuardedServingMode, RecallMode, MIN_ANN_RECALL_Q16,
};
use super::super::persisted::{
    search_persisted_lexical, search_persisted_vectors, PersistedLexicalSearchIndex,
};
use super::super::{SearchMode, SearchQuery};
use super::allowed::persisted_allowed_candidates;
use super::ann_reports::{
    persisted_exact_fallback_report, persisted_graph_is_stale,
    persisted_hnsw_fault_fallback_report, persisted_hnsw_stale_fallback_report,
};
use super::diversity::select_diverse_results;
use super::persisted_rrf::{distance_metric_from_manifest, fuse_persisted_rrf};
use super::ranking::{
    database_hybrid_rrf_weights, database_weighted_reranker, rerank_database_results,
    search_rerank_candidate_limit,
};
use super::trace::trace_search;
use super::{DatabaseSearchOutcome, DatabaseSearchResult, PersistedSearchCandidate};

impl Database {
    pub(crate) fn search_persisted_query(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
        policy: Option<AnnSearchPolicy>,
        allowed_cells: Option<&std::collections::BTreeSet<CellId>>,
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
        let allowed = persisted_allowed_candidates(&state, view, allowed_cells);
        let expanded_query_text = self.corpus_synonym_expanded_query_text(query.text)?;
        let lexical_query_text = expanded_query_text.as_deref().unwrap_or(query.text);
        let analyzer = self.text_analyzer();
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
                &analyzer,
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
                if !self.feature_flags.experimental_hnsw {
                    let metric = self
                        .manifest()
                        .vector_profile
                        .map(|profile| distance_metric_from_manifest(profile.metric))
                        .unwrap_or_default();
                    let ranked =
                        self.search_disk_resident_vectors(vector, &allowed, query.limit, &metric)?;
                    ann_report = Some(persisted_exact_fallback_report(
                        query.limit,
                        allowed.len(),
                        ranked.len(),
                        policy.unwrap_or_default(),
                    ));
                    ranked
                } else {
                    let index = self.persisted_vector_index()?;
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
                                AnnSearchOutcome {
                                    results: ranked,
                                    report,
                                }
                            } else {
                                // A3.3: the guarded sampler decides whether to pay
                                // an exact recall recompute on this query (default
                                // off -> unchanged behavior).
                                self.guarded_ann_search(
                                    &index.vectors,
                                    &graph,
                                    vector,
                                    &allowed,
                                    query.limit,
                                    policy,
                                )
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
                            AnnSearchOutcome {
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
                let metric = self
                    .manifest()
                    .vector_profile
                    .map(|profile| distance_metric_from_manifest(profile.metric))
                    .unwrap_or_default();
                self.search_disk_resident_vectors(vector, &allowed, query.limit, &metric)?
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
                        &analyzer,
                    );
                    let vector =
                        self.search_disk_resident_vectors(vector, &allowed, depth, &metric)?;
                    fuse_persisted_rrf(
                        lexical,
                        vector,
                        depth,
                        database_hybrid_rrf_weights(self, query.text),
                    )
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
                        &analyzer,
                    )
                    .into_iter()
                    .map(PersistedSearchCandidate::from_lexical)
                    .collect()
                }
            }
        };
        let pin = self.pin_read_txn();
        let txn = pin.read_txn();
        let mut results = Vec::new();
        for candidate in ranked {
            let Some(cell_id) = state.candidate_to_cell.get(&candidate.candidate_id) else {
                continue;
            };
            let Some(version) = self.memtable.read(txn, *cell_id) else {
                continue;
            };
            let retrieved = self.retrieved_cell_from_version(version)?;
            let metadata = crate::query::CellMetadata::from_payload_with_descriptor(
                &retrieved.payload,
                &retrieved.descriptor,
            );
            results.push(DatabaseSearchResult {
                cell_id: *cell_id,
                score: candidate.score,
                lexical_score: candidate.lexical_score,
                vector_score: candidate.vector_score,
                metadata,
                payload: retrieved.payload,
            });
        }
        let mut diversity_diagnostics = None;
        if query.mode == SearchMode::HybridRerank {
            rerank_database_results(&mut results, query, &database_weighted_reranker(self));
            let selection = select_diverse_results(results, query);
            results = selection.results;
            diversity_diagnostics = Some(selection.diagnostics);
        }
        let results = self.expand_high_level_anchor_context(results, view, query);
        let results = self.expand_project_related_context(results, view, query);
        let results = self.expand_search_parent_context(results, view, query.limit);
        trace_search(&format!("persisted done {}", results.len()));
        Ok(Some(DatabaseSearchOutcome {
            results,
            ann_report,
            view_traces: Vec::new(),
            diversity_diagnostics,
        }))
    }

    /// A3.3: run the ANN query under the guarded-recall sampler when opted in.
    ///
    /// Default off -> the exact-recall recompute runs on every query, byte-identical
    /// to pre-A3.3. When on, a deterministic sample of queries recomputes exact
    /// recall into the per-database window (`record_sampled`); unsampled queries
    /// serve ANN with the windowed recall and skip the exact scan
    /// (`search_persisted_ann_sampled`); a collection whose window has degraded
    /// serves exact until a rebuild (detected via the manifest generation) clears
    /// it. All decisions are deterministic (no wall clock / RNG).
    fn guarded_ann_search(
        &self,
        vectors: &std::collections::BTreeMap<u32, Vec<i16>>,
        graph: &cortex_storage::hnsw::HnswGraphIndex,
        vector: &[i16],
        allowed: &std::collections::BTreeSet<u32>,
        limit: usize,
        policy: Option<AnnSearchPolicy>,
    ) -> AnnSearchOutcome {
        // A3.3 perf: reuse a cross-query cached `HnswIndex` when available so the
        // O(n) per-query rebuild + integrity walk is amortized. `None` (integrity
        // failure / poisoned lock) falls back to the per-query rebuild path, which
        // is byte-identical to pre-cache behavior. The `run` closure serves every
        // path (default / sampled / unsampled / degraded) through whichever is
        // available, so results are identical regardless of cache state.
        let cached = self.cached_hnsw_index(vectors, graph);
        let run = |recall_mode: RecallMode, policy: AnnSearchPolicy| match cached.as_ref() {
            Some(index) => search_persisted_ann_cached(
                vectors,
                graph,
                vector,
                allowed,
                limit,
                policy,
                recall_mode,
                index,
            ),
            None => match recall_mode {
                RecallMode::Exact => {
                    search_persisted_ann_with_policy(vectors, graph, vector, allowed, limit, policy)
                }
                RecallMode::Windowed(windowed) => search_persisted_ann_sampled(
                    vectors, graph, vector, allowed, limit, policy, windowed,
                ),
            },
        };

        if !self.ann_guarded_sampling.enabled {
            return run(RecallMode::Exact, policy.unwrap_or_default());
        }
        let mut guard = match self.guarded_recall.lock() {
            Ok(guard) => guard,
            Err(_) => return run(RecallMode::Exact, policy.unwrap_or_default()),
        };
        let Some(state) = guard.as_mut() else {
            return run(RecallMode::Exact, policy.unwrap_or_default());
        };

        // A rebuild (new manifest generation) reshuffles the sample set + re-arms
        // the warm-up, and recovers a degraded collection to ANN serving.
        let generation = self.manifest().checkpoint_seq;
        if state.generation() != generation {
            state.on_rebuild(generation);
        }

        let base_policy = policy.unwrap_or_default();
        let floor = base_policy.min_recall_q16.unwrap_or(MIN_ANN_RECALL_Q16);
        let query_bytes: Vec<u8> = vector
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();

        if state.serving_mode() == GuardedServingMode::ExactDegraded {
            // Degraded: a windowed recall of 0 forces the low-recall exact fallback.
            state.record_unsampled();
            return run(RecallMode::Windowed(0), base_policy);
        }

        if state.should_sample(&query_bytes) {
            let outcome = run(RecallMode::Exact, base_policy);
            match outcome.report.recall_q16 {
                Some(recall) => state.record_sampled(recall, floor),
                None => state.record_unsampled(),
            }
            outcome
        } else {
            let windowed = state.window().windowed_min().unwrap_or(u16::MAX);
            state.record_unsampled();
            run(RecallMode::Windowed(windowed), base_policy)
        }
    }
}
