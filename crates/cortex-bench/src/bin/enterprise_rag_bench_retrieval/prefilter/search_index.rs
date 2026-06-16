use std::collections::{BTreeMap, BTreeSet};

use cortex_core::CellId;
use cortex_engine::search::{SearchQuery, SearchReranker};
use cortex_engine::Database;

use super::super::retrieval::BenchmarkRetrievalIndex;
use super::super::vectors::{body_vector_from_payload, vector_dot_score};
use super::super::ENGINE_PREFILTER_SHORTLIST_LIMIT;
use super::candidate::{PrefilterCandidate, PrefilterSearchOutput, SearchPrefilterContext};
use super::external::ExternalPrefilterRetrieval;
use super::scoring::{lexical_rank_score, prefilter_evidence_score};
use super::selection::select_prefilter_candidates;
use super::source_hints::inferred_source_types_from_query;

pub(crate) fn search_prefilter_payloads(
    db: &Database,
    doc_to_cell: &BTreeMap<String, CellId>,
    prefilter: &mut SearchPrefilterContext<'_>,
    question_id: &str,
    query: SearchQuery<'_>,
    top_k: usize,
    reranker: Option<&dyn SearchReranker>,
) -> PrefilterSearchOutput {
    let Some(prefilter_index) = prefilter.index else {
        return PrefilterSearchOutput {
            payloads: Vec::new(),
            diversity_diagnostics: None,
        };
    };
    let shortlist_limit = top_k.max(ENGINE_PREFILTER_SHORTLIST_LIMIT);
    let doc_ids = prefilter_doc_ids(
        prefilter_index,
        prefilter.external_retrieval,
        question_id,
        query.text,
        shortlist_limit,
    );
    let candidates = doc_ids
        .into_iter()
        .enumerate()
        .filter_map(|(rank, doc_id)| {
            let cell_id = *doc_to_cell.get(&doc_id)?;
            let payload = db.get_latest_cell(cell_id)?;
            let lexical_score = lexical_rank_score(rank, shortlist_limit);
            let vector_score = query
                .vector
                .and_then(|query_vector| {
                    let payload_vector = body_vector_from_payload(&payload);
                    let candidate_vector = payload_vector
                        .or_else(|| prefilter.document_vectors.get(&doc_id).ok().flatten())?;
                    Some(vector_dot_score(query_vector, &candidate_vector))
                })
                .unwrap_or(0);
            Some(PrefilterCandidate {
                cell_id,
                evidence_score: prefilter_evidence_score(query.text, &payload),
                payload,
                score: lexical_score.saturating_add(vector_score),
                lexical_score,
                vector_score,
            })
        })
        .collect::<Vec<_>>();
    select_prefilter_candidates(candidates, query, top_k, reranker)
}

pub(crate) fn prefilter_doc_ids(
    prefilter_index: &BenchmarkRetrievalIndex,
    external_retrieval: Option<&ExternalPrefilterRetrieval>,
    question_id: &str,
    query_text: &str,
    shortlist_limit: usize,
) -> Vec<String> {
    let mut doc_ids = Vec::with_capacity(shortlist_limit);
    let mut seen = BTreeSet::new();
    if let Some(external) = external_retrieval.and_then(|retrieval| retrieval.doc_ids(question_id))
    {
        for doc_id in external.iter().take(shortlist_limit) {
            if seen.insert(doc_id.clone()) {
                doc_ids.push(doc_id.clone());
            }
        }
    }
    if doc_ids.len() < shortlist_limit {
        let source_hints = inferred_source_types_from_query(query_text);
        for doc_id in prefilter_index.search_doc_ids(query_text, &source_hints, shortlist_limit) {
            if seen.insert(doc_id.clone()) {
                doc_ids.push(doc_id);
            }
            if doc_ids.len() >= shortlist_limit {
                break;
            }
        }
    }
    doc_ids
}
