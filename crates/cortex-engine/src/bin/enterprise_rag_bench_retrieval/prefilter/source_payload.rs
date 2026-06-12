use std::collections::BTreeMap;
use std::path::PathBuf;

use cortex_core::CellId;
use cortex_engine::search::{SearchMode, SearchQuery, SearchReranker};

use super::super::document::{build_payload, extract_document_content};
use super::super::io::read_json;
use super::super::vectors::{body_vector_from_payload, vector_dot_score, DocumentVectorLookup};
use super::super::{doc_id_to_cell_id, ENGINE_PREFILTER_SHORTLIST_LIMIT};
use super::candidate::{PrefilterCandidate, PrefilterSearchOutput};
use super::external::ExternalPrefilterRetrieval;
use super::scoring::{lexical_rank_score, prefilter_evidence_score};
use super::selection::{empty_prefilter_diversity_diagnostics, select_prefilter_candidates};

pub(crate) struct SourcePayloadPrefilter {
    rel_paths: BTreeMap<String, String>,
    doc_to_cell: BTreeMap<String, CellId>,
    sources_dir: PathBuf,
    payload_cache: BTreeMap<String, Vec<u8>>,
}

impl SourcePayloadPrefilter {
    pub(crate) fn new(
        uuid_index: &BTreeMap<String, String>,
        sources_dir: PathBuf,
    ) -> Result<Self, String> {
        Ok(Self {
            rel_paths: uuid_index.clone(),
            doc_to_cell: doc_id_to_cell_id(uuid_index)?,
            sources_dir,
            payload_cache: BTreeMap::new(),
        })
    }

    fn candidate(
        &mut self,
        doc_id: &str,
        rank: usize,
        shortlist_limit: usize,
        query: SearchQuery<'_>,
        document_vectors: &mut DocumentVectorLookup,
    ) -> Result<Option<PrefilterCandidate>, String> {
        let Some(cell_id) = self.doc_to_cell.get(doc_id).copied() else {
            return Ok(None);
        };
        let payload = self.payload(doc_id, document_vectors)?.clone();
        let lexical_score = lexical_rank_score(rank, shortlist_limit);
        let vector_score = query
            .vector
            .and_then(|query_vector| {
                let payload_vector = body_vector_from_payload(&payload);
                let candidate_vector =
                    payload_vector.or_else(|| document_vectors.get(doc_id).ok().flatten())?;
                Some(vector_dot_score(query_vector, &candidate_vector))
            })
            .unwrap_or(0);
        Ok(Some(PrefilterCandidate {
            cell_id,
            evidence_score: prefilter_evidence_score(query.text, &payload),
            payload,
            score: lexical_score.saturating_add(vector_score),
            lexical_score,
            vector_score,
        }))
    }

    fn payload(
        &mut self,
        doc_id: &str,
        document_vectors: &mut DocumentVectorLookup,
    ) -> Result<&Vec<u8>, String> {
        if !self.payload_cache.contains_key(doc_id) {
            let rel_path = self
                .rel_paths
                .get(doc_id)
                .ok_or_else(|| format!("prefilter doc_id={doc_id} is not in uuid index"))?;
            let document = read_json(&self.sources_dir.join(rel_path))?;
            let (title, content) = extract_document_content(&document);
            let vector = document_vectors.get(doc_id)?;
            let payload = build_payload(doc_id, rel_path, &title, &content, vector.as_deref());
            self.payload_cache
                .insert(doc_id.to_owned(), payload.into_bytes());
        }
        self.payload_cache
            .get(doc_id)
            .ok_or_else(|| format!("prefilter payload cache missed doc_id={doc_id}"))
    }
}

pub(crate) fn source_prefilter_payloads(
    source_payloads: &mut SourcePayloadPrefilter,
    document_vectors: &mut DocumentVectorLookup,
    external_retrieval: &ExternalPrefilterRetrieval,
    question_id: &str,
    query: SearchQuery<'_>,
    top_k: usize,
    reranker: Option<&dyn SearchReranker>,
) -> Result<PrefilterSearchOutput, String> {
    let shortlist_limit = top_k.max(ENGINE_PREFILTER_SHORTLIST_LIMIT);
    let Some(doc_ids) = external_retrieval.doc_ids(question_id) else {
        return Ok(PrefilterSearchOutput {
            payloads: Vec::new(),
            diversity_diagnostics: if query.mode == SearchMode::HybridRerank {
                Some(empty_prefilter_diversity_diagnostics(query.text))
            } else {
                None
            },
        });
    };
    let mut candidates = Vec::new();
    for (rank, doc_id) in doc_ids.iter().take(shortlist_limit).enumerate() {
        if let Some(candidate) =
            source_payloads.candidate(doc_id, rank, shortlist_limit, query, document_vectors)?
        {
            candidates.push(candidate);
        }
    }
    Ok(select_prefilter_candidates(
        candidates, query, top_k, reranker,
    ))
}
