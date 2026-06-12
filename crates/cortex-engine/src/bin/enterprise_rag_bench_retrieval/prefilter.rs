use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Instant;

use cortex_core::CellId;
use cortex_engine::search::{
    analyze_search_query, condition_payload_bonus, covered_requirement_ids,
    decompose_enterprise_rag_question, extract_query_conditions, map_query_to_scope,
    route_policy_for_query, scope_mapping_payload_bonus, tokenize, SearchDiversityDiagnostics,
    SearchMode, SearchQuery, SearchQueryIntent, SearchRerankInput, SearchReranker,
    WeightedScoreReranker,
};
use cortex_engine::Database;
use serde_json::{json, Value};

use super::args::{Args, BenchmarkRetrievalMode};
use super::document::{build_payload, extract_document_content};
use super::io::read_json;
use super::logger::RunLogger;
use super::metrics::{DiversityRunMetrics, SearchRetrievalOutput};
use super::reporting::{bench_view, required_str};
use super::retrieval::BenchmarkRetrievalIndex;

mod diversity;
use super::vectors::{
    body_vector_from_payload, vector_dot_score, BenchmarkSearchIndex, DocumentVectorLookup,
};
use super::{
    doc_id_to_cell_id, retrieval_row, round_ms, ENGINE_PREFILTER_DEFAULT_DOC_LIMIT,
    ENGINE_PREFILTER_LEXICAL_HEAD_COUNT, ENGINE_PREFILTER_SHORTLIST_LIMIT,
    ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE,
};
pub(super) use diversity::select_diverse_prefilter_candidates;

#[derive(Clone)]
pub(super) struct PrefilterCandidate {
    pub(super) cell_id: CellId,
    pub(super) payload: Vec<u8>,
    pub(super) score: u64,
    pub(super) lexical_score: u64,
    pub(super) vector_score: u64,
    pub(super) evidence_score: u32,
}

pub(super) struct PrefilterSearchOutput {
    pub(super) payloads: Vec<Vec<u8>>,
    pub(super) diversity_diagnostics: Option<SearchDiversityDiagnostics>,
}

pub(super) struct SearchPrefilterContext<'a> {
    pub(super) index: Option<&'a BenchmarkRetrievalIndex>,
    pub(super) source_payloads: Option<&'a mut SourcePayloadPrefilter>,
    pub(super) document_vectors: &'a mut DocumentVectorLookup,
    pub(super) external_retrieval: Option<&'a ExternalPrefilterRetrieval>,
}

#[derive(Debug)]
pub(super) struct ExternalPrefilterRetrieval {
    pub(super) by_question_id: BTreeMap<String, Vec<String>>,
    pub(super) rows: usize,
}

impl ExternalPrefilterRetrieval {
    pub(super) fn doc_ids(&self, question_id: &str) -> Option<&[String]> {
        self.by_question_id.get(question_id).map(Vec::as_slice)
    }
}

pub(super) struct SourcePayloadPrefilter {
    rel_paths: BTreeMap<String, String>,
    doc_to_cell: BTreeMap<String, CellId>,
    sources_dir: PathBuf,
    payload_cache: BTreeMap<String, Vec<u8>>,
}

impl SourcePayloadPrefilter {
    pub(super) fn new(
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

pub(super) fn should_use_source_payload_prefilter(
    args: &Args,
    engine_search_mode: bool,
    external_prefilter_retrieval: &Option<ExternalPrefilterRetrieval>,
) -> bool {
    engine_search_mode
        && external_prefilter_retrieval.is_some()
        && !args.disable_search_prefilter
        && args.official_clean
}

pub(super) fn source_prefilter_payloads(
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

pub(super) fn search_prefilter_payloads(
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

pub(super) fn select_prefilter_candidates(
    mut candidates: Vec<PrefilterCandidate>,
    query: SearchQuery<'_>,
    top_k: usize,
    reranker: Option<&dyn SearchReranker>,
) -> PrefilterSearchOutput {
    let mut lexical_candidates = candidates.clone();
    lexical_candidates.sort_by_key(|candidate| {
        (
            std::cmp::Reverse(candidate.lexical_score),
            candidate.cell_id.0,
        )
    });
    if let Some(reranker) = reranker {
        for candidate in &mut candidates {
            candidate.score = reranker.rerank_score(SearchRerankInput {
                query_text: query.text,
                query_vector: query.vector,
                candidate_id: candidate.cell_id.0,
                lexical_score: candidate.lexical_score,
                vector_score: candidate.vector_score,
                base_score: candidate.score,
                metadata: None,
                payload: Some(&candidate.payload),
            });
        }
    }
    candidates.sort_by_key(|candidate| (std::cmp::Reverse(candidate.score), candidate.cell_id.0));
    let (selected, diversity_diagnostics) = if query.mode == SearchMode::HybridRerank {
        let selection =
            select_diverse_prefilter_candidates(lexical_candidates, candidates, query.text, top_k);
        (selection.candidates, Some(selection.diagnostics))
    } else {
        (
            merge_prefilter_candidates(query.text, lexical_candidates, candidates, top_k),
            None,
        )
    };
    PrefilterSearchOutput {
        payloads: selected
            .into_iter()
            .map(|candidate| candidate.payload)
            .collect(),
        diversity_diagnostics,
    }
}

pub(super) fn empty_prefilter_diversity_diagnostics(
    query_text: &str,
) -> SearchDiversityDiagnostics {
    let route_policy = route_policy_for_query(query_text);
    SearchDiversityDiagnostics {
        intent: cortex_engine::search::classify_search_query_intent(query_text),
        diversity_enabled: route_policy.diversity,
        lambda_q16: route_policy.diversity_lambda_q16,
        input_candidates: 0,
        output_candidates: 0,
        skipped_candidates: 0,
        max_payload_similarity_q16: 0,
        max_cluster_similarity_q16: 0,
        selected_with_payload_similarity: 0,
        selected_with_cluster_similarity: 0,
    }
}

pub(super) fn prefilter_doc_ids(
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

fn lexical_rank_score(rank: usize, shortlist_limit: usize) -> u64 {
    u64::try_from(shortlist_limit.saturating_sub(rank))
        .unwrap_or(0)
        .saturating_mul(1_000_000)
}

pub(super) fn merge_prefilter_candidates(
    query_text: &str,
    lexical_candidates: Vec<PrefilterCandidate>,
    reranked_candidates: Vec<PrefilterCandidate>,
    top_k: usize,
) -> Vec<PrefilterCandidate> {
    let lexical_head = top_k.min(prefilter_lexical_head_count(query_text, top_k));
    let vector_promotions = top_k.saturating_sub(lexical_head);
    let pool_limit = top_k.saturating_mul(4).max(top_k);
    let mut selected = Vec::with_capacity(pool_limit);
    let mut seen = BTreeSet::new();
    push_unique_prefilter_candidates(
        &mut selected,
        &mut seen,
        lexical_candidates.iter().take(lexical_head),
        pool_limit,
    );
    push_unique_prefilter_candidates(
        &mut selected,
        &mut seen,
        reranked_candidates.iter().take(vector_promotions),
        pool_limit,
    );
    push_unique_prefilter_candidates(
        &mut selected,
        &mut seen,
        lexical_candidates.iter().skip(lexical_head),
        pool_limit,
    );
    push_unique_prefilter_candidates(
        &mut selected,
        &mut seen,
        reranked_candidates.iter().skip(vector_promotions),
        pool_limit,
    );
    prune_weak_prefilter_tail(query_text, selected, top_k)
}

pub(super) fn push_unique_prefilter_candidates<'a>(
    selected: &mut Vec<PrefilterCandidate>,
    seen: &mut BTreeSet<u64>,
    candidates: impl Iterator<Item = &'a PrefilterCandidate>,
    top_k: usize,
) {
    for candidate in candidates {
        if selected.len() >= top_k {
            break;
        }
        if seen.insert(candidate.cell_id.0) {
            selected.push(candidate.clone());
        }
    }
}

pub(super) fn prefilter_lexical_head_count(query_text: &str, top_k: usize) -> usize {
    let decomposition = decompose_enterprise_rag_question(query_text);
    let multipart = decomposition.requirements.len() > 1;
    let head = match cortex_engine::search::classify_search_query_intent(query_text) {
        SearchQueryIntent::Lookup
        | SearchQueryIntent::InfoNotFound
        | SearchQueryIntent::Constrained => ENGINE_PREFILTER_LEXICAL_HEAD_COUNT,
        SearchQueryIntent::Semantic if multipart => top_k,
        SearchQueryIntent::Semantic => ENGINE_PREFILTER_LEXICAL_HEAD_COUNT,
        SearchQueryIntent::ProjectRelated
        | SearchQueryIntent::HighLevel
        | SearchQueryIntent::ConflictingInfo
        | SearchQueryIntent::Completeness => top_k,
    };
    top_k.min(head.max(1))
}

fn prune_weak_prefilter_tail(
    query_text: &str,
    candidates: Vec<PrefilterCandidate>,
    top_k: usize,
) -> Vec<PrefilterCandidate> {
    let default_limit = top_k.min(prefilter_default_doc_limit(query_text, top_k));
    if candidates.len() <= default_limit {
        return candidates;
    }
    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            if index < default_limit
                || candidate.evidence_score >= ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE
            {
                Some(candidate)
            } else {
                None
            }
        })
        .take(top_k)
        .collect()
}

pub(super) fn prefilter_default_doc_limit(query_text: &str, top_k: usize) -> usize {
    let decomposition = decompose_enterprise_rag_question(query_text);
    let multipart = decomposition.requirements.len() > 1;
    let limit = match cortex_engine::search::classify_search_query_intent(query_text) {
        SearchQueryIntent::Lookup => ENGINE_PREFILTER_DEFAULT_DOC_LIMIT,
        SearchQueryIntent::InfoNotFound => 3,
        SearchQueryIntent::Constrained => ENGINE_PREFILTER_DEFAULT_DOC_LIMIT,
        SearchQueryIntent::Semantic if multipart => top_k,
        SearchQueryIntent::Semantic => ENGINE_PREFILTER_DEFAULT_DOC_LIMIT.saturating_add(1),
        SearchQueryIntent::ProjectRelated
        | SearchQueryIntent::HighLevel
        | SearchQueryIntent::ConflictingInfo
        | SearchQueryIntent::Completeness => top_k,
    };
    top_k.min(limit.max(1))
}

pub(super) fn prefilter_evidence_score(query_text: &str, payload: &[u8]) -> u32 {
    let payload_text = String::from_utf8_lossy(payload);
    let payload_lower = payload_text.to_lowercase();
    let mut score = 0u32;
    let analyzed = analyze_search_query(query_text);
    for anchor in analyzed.anchors {
        if anchor.terms.iter().any(|term| payload_lower.contains(term)) {
            score = score.saturating_add(4);
        }
    }
    if analyzed
        .source_hints
        .iter()
        .any(|source| payload_lower.contains(source))
    {
        score = score.saturating_add(4);
    }
    let scope_mapping = map_query_to_scope(query_text);
    if scope_mapping_payload_bonus(&scope_mapping, payload) > 0 {
        score = score.saturating_add(4);
    }
    let conditions = extract_query_conditions(query_text);
    if condition_payload_bonus(&conditions, payload) > 0 {
        score = score.saturating_add(4);
    }
    let decomposition = decompose_enterprise_rag_question(query_text);
    score = score.saturating_add(
        u32::try_from(covered_requirement_ids(&decomposition, &payload_text).len())
            .unwrap_or(u32::MAX)
            .saturating_mul(2),
    );
    for term in tokenize(query_text)
        .into_iter()
        .filter(|term| is_prefilter_evidence_term(term))
    {
        if payload_lower.contains(&term) {
            score = score.saturating_add(1);
        }
    }
    score
}

fn is_prefilter_evidence_term(term: &str) -> bool {
    term.len() >= 3
        && !matches!(
            term,
            "who"
                | "what"
                | "which"
                | "where"
                | "when"
                | "why"
                | "how"
                | "give"
                | "tell"
                | "show"
                | "find"
                | "list"
                | "all"
                | "any"
                | "does"
                | "did"
                | "was"
                | "were"
                | "are"
                | "for"
                | "with"
                | "from"
                | "into"
                | "about"
                | "this"
                | "that"
                | "project"
                | "team"
                | "docs"
                | "document"
        )
}

pub(super) fn inferred_source_types_from_query(query: &str) -> Vec<String> {
    let lower = query.to_lowercase();
    let mut values = Vec::<String>::new();
    let mut push = |source: &str| {
        if !values.iter().any(|value| value == source) {
            values.push(source.to_owned());
        }
    };
    if contains_query_marker(&lower, &["slack", "slack thread", "channel"]) {
        push("slack");
    }
    if contains_query_marker(&lower, &["gmail", "email", "mail thread", "customer email"]) {
        push("gmail");
    }
    if contains_query_marker(&lower, &["jira", "jira issue", "jira ticket"]) {
        push("jira");
    }
    if contains_query_marker(
        &lower,
        &["github", "pull request", "pr #", "repository", "repo"],
    ) {
        push("github");
    }
    if contains_query_marker(
        &lower,
        &["google drive", "drive doc", "drive document", "drive file"],
    ) {
        push("google_drive");
    }
    if contains_query_marker(&lower, &["linear", "linear issue"]) {
        push("linear");
    }
    if contains_query_marker(&lower, &["hubspot", "account note", "crm"]) {
        push("hubspot");
    }
    if contains_query_marker(
        &lower,
        &["fireflies", "meeting transcript", "call transcript"],
    ) {
        push("fireflies");
    }
    if contains_query_marker(&lower, &["confluence", "wiki page", "runbook", "adr"]) {
        push("confluence");
    }
    values
}

fn contains_query_marker(query: &str, markers: &[&str]) -> bool {
    markers
        .iter()
        .any(|marker| contains_query_marker_value(query, marker))
}

fn contains_query_marker_value(query: &str, marker: &str) -> bool {
    if marker.chars().any(|ch| !ch.is_ascii_alphanumeric()) {
        return query.contains(marker);
    }
    let mut start = 0usize;
    while let Some(relative) = query[start..].find(marker) {
        let index = start + relative;
        let end = index + marker.len();
        let before = query[..index].chars().next_back();
        let after = query[end..].chars().next();
        let left_boundary = before.is_none_or(|ch| !ch.is_ascii_alphanumeric());
        let right_boundary = after.is_none_or(|ch| !ch.is_ascii_alphanumeric());
        if left_boundary && right_boundary {
            return true;
        }
        start = end;
    }
    false
}

pub(super) fn retrieve_search_questions(
    db: &Database,
    uuid_index: &BTreeMap<String, String>,
    questions: &[Value],
    query_vectors: &BTreeMap<String, Vec<i16>>,
    mut prefilter: SearchPrefilterContext<'_>,
    args: &Args,
    logger: &RunLogger,
) -> Result<SearchRetrievalOutput, String> {
    let mut rows = Vec::with_capacity(questions.len());
    let mut diversity = DiversityRunMetrics::default();
    let view = bench_view();
    let reranker = WeightedScoreReranker::default();
    let doc_to_cell = if prefilter.index.is_some() {
        Some(doc_id_to_cell_id(uuid_index)?)
    } else {
        None
    };
    let reusable_index = if args.skip_checkpoint && prefilter.index.is_none() {
        logger.log("build reusable search index for skip-checkpoint retrieval");
        logger.status(
            "build_reusable_search_index",
            "running",
            "build reusable in-memory search index",
            None,
            Some(uuid_index.len()),
            &[],
        );
        let started = Instant::now();
        let index = BenchmarkSearchIndex::load(db, uuid_index, &view, logger)?;
        logger.log(&format!(
            "reusable search index built in {} ms",
            round_ms(started.elapsed().as_secs_f64() * 1000.0)
        ));
        logger.status(
            "build_reusable_search_index",
            "done",
            "reusable in-memory search index built",
            Some(uuid_index.len()),
            Some(uuid_index.len()),
            &[],
        );
        Some(index)
    } else {
        None
    };
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let (mode, vector) = match args.retrieval_mode {
            BenchmarkRetrievalMode::EngineKeyword => (SearchMode::Keyword, None),
            BenchmarkRetrievalMode::EngineHybrid => {
                let vector = query_vectors.get(qid).ok_or_else(|| {
                    format!("engine-hybrid requires query vector for question_id={qid}")
                })?;
                (SearchMode::Hybrid, Some(vector.as_slice()))
            }
            BenchmarkRetrievalMode::EngineHybridRerank => {
                let vector = query_vectors.get(qid).ok_or_else(|| {
                    format!("engine-hybrid-rerank requires query vector for question_id={qid}")
                })?;
                (SearchMode::HybridRerank, Some(vector.as_slice()))
            }
            BenchmarkRetrievalMode::CachedLexical | BenchmarkRetrievalMode::EngineAql => {
                unreachable!("cached and AQL modes handled separately")
            }
        };
        let search_query = SearchQuery {
            text: query,
            vector,
            limit: args.top_k,
            mode,
        };
        let mode_label = args.retrieval_mode.as_str();
        logger.status(
            "retrieve_questions",
            "running",
            "retrieve engine question row",
            Some(index),
            Some(questions.len()),
            &[
                ("current_question_id", json!(qid)),
                ("retrieval_mode", json!(mode_label)),
                ("rerank_mode", json!(args.rerank_mode.as_str())),
            ],
        );
        logger.log(&format!(
            "retrieve engine question {}/{} question_id={} mode={} rerank={}",
            index + 1,
            questions.len(),
            qid,
            mode_label,
            args.rerank_mode.as_str()
        ));
        let question_started = Instant::now();
        let payloads = if let (Some(source_payloads), Some(external_retrieval)) = (
            prefilter.source_payloads.as_deref_mut(),
            prefilter.external_retrieval,
        ) {
            let output = source_prefilter_payloads(
                source_payloads,
                prefilter.document_vectors,
                external_retrieval,
                qid,
                search_query,
                args.top_k,
                args.rerank_mode.is_enabled().then_some(&reranker),
            )?;
            if let Some(diagnostics) = &output.diversity_diagnostics {
                diversity.record(diagnostics);
            }
            output.payloads
        } else if let (Some(_), Some(doc_to_cell)) = (prefilter.index, doc_to_cell.as_ref()) {
            let output = search_prefilter_payloads(
                db,
                doc_to_cell,
                &mut prefilter,
                qid,
                search_query,
                args.top_k,
                args.rerank_mode.is_enabled().then_some(&reranker),
            );
            if let Some(diagnostics) = &output.diversity_diagnostics {
                diversity.record(diagnostics);
            }
            output.payloads
        } else if let Some(reusable_index) = &reusable_index {
            reusable_index.search_payloads(
                db,
                search_query,
                args.top_k,
                args.rerank_mode.is_enabled().then_some(&reranker),
            )
        } else if args.rerank_mode.is_enabled() {
            db.search_cells_with_reranker(search_query, &view, &reranker)
                .map_err(|error| {
                    format!("engine {mode_label} rerank search failed for {qid}: {error}")
                })?
                .into_iter()
                .map(|result| result.payload)
                .collect()
        } else {
            let outcome = db
                .search_cells_with_report(search_query, &view)
                .map_err(|error| format!("engine {mode_label} search failed for {qid}: {error}"))?;
            if let Some(diagnostics) = &outcome.diversity_diagnostics {
                diversity.record(diagnostics);
            }
            outcome
                .results
                .into_iter()
                .map(|result| result.payload)
                .collect()
        };
        rows.push(retrieval_row(qid, query, payloads));
        let question_duration_ms = round_ms(question_started.elapsed().as_secs_f64() * 1000.0);
        if args.progress_every > 0
            && ((index + 1).is_multiple_of(args.progress_every)
                || questions.len() <= args.progress_every)
        {
            logger.log(&format!(
                "retrieved {}/{} last_question_ms={question_duration_ms}",
                index + 1,
                questions.len()
            ));
            logger.status(
                "retrieve_questions",
                "running",
                "retrieve engine question rows",
                Some(index + 1),
                Some(questions.len()),
                &[
                    ("last_question_id", json!(qid)),
                    ("last_question_ms", json!(question_duration_ms)),
                    ("retrieval_mode", json!(mode_label)),
                    ("rerank_mode", json!(args.rerank_mode.as_str())),
                ],
            );
        }
    }
    Ok(SearchRetrievalOutput { rows, diversity })
}
