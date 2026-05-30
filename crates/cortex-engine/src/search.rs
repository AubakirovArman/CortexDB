use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

mod access;
mod analyzer;
mod ann;
mod ann_drift;
mod ann_external;
mod ann_fixture;
mod ann_recall_tests;
mod ann_report;
mod database;
mod evaluation;
mod hnsw;
mod hnsw_policy;
mod persisted;
mod quality_tests;
mod tokenizer;
pub(crate) mod vector;

pub use analyzer::{mean_reciprocal_rank_q16, Language, TextAnalyzer};
pub use ann::{
    AnnEvaluationReport, AnnFallbackReason, AnnMetrics, AnnSearchPath, AnnSearchPolicy,
    AnnSearchReport, AnnSloViolation, MIN_ANN_RECALL_Q16,
};
pub use ann_drift::{
    compare_ann_drift_baseline, evaluate_ann_drift_baseline, AnnDriftBaseline, AnnDriftReport,
};
pub use ann_external::{
    evaluate_ann_external_fixture, AnnExternalFixtureBaseline, AnnExternalFixtureReport,
    AnnJsonlEntry,
};
pub use ann_fixture::{
    compare_ann_fixture_baseline, evaluate_ann_fixture_baseline, AnnRecallLatencyBaseline,
    AnnRecallLatencyGateReport,
};
pub use ann_report::{
    synthetic_ann_recall_latency_report, AnnRecallLatencyReport, SYNTHETIC_ANN_CORPUS_V1,
};
pub use database::{DatabaseSearchOutcome, DatabaseSearchResult, SearchLimit};
pub use hnsw::{integrity::HnswIntegrityReport, DistanceMetric, HnswIndex, VectorCollectionConfig};
pub use hnsw_policy::{HnswMaintenancePolicy, HnswMaintenanceReport, HnswRebuildPolicy};
pub use tokenizer::tokenize;
pub use vector::parse_vector_literal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScoredCandidate {
    pub cell_id: u32,
    pub score: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchMode {
    Keyword,
    Vector,
    VectorExact,
    Hybrid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchQuery<'a> {
    pub text: &'a str,
    pub vector: Option<&'a [i16]>,
    pub limit: usize,
    pub mode: SearchMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub cell_id: u32,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Bm25Index {
    docs: BTreeMap<u32, BTreeMap<String, u32>>,
    doc_lengths: BTreeMap<u32, u32>,
    postings: BTreeMap<String, BTreeSet<u32>>,
}

#[derive(Clone, Debug, Default)]
pub struct VectorIndex {
    vectors: BTreeMap<u32, Vec<i16>>,
    metric: hnsw::DistanceMetric,
}

#[derive(Clone, Debug, Default)]
pub struct SearchIndexes {
    pub lexical: Bm25Index,
    pub vector: VectorIndex,
}

impl Bm25Index {
    pub fn add_document(&mut self, cell_id: u32, text: &str) {
        self.add_document_fields(cell_id, &[(text, 1)]);
    }

    pub fn add_document_fields(&mut self, cell_id: u32, fields: &[(&str, u32)]) {
        let mut terms = BTreeMap::<String, u32>::new();
        for (text, weight) in fields {
            let weight = (*weight).max(1);
            for term in tokenize(text) {
                *terms.entry(term).or_default() += weight;
            }
        }
        self.replace_document_terms(cell_id, terms);
    }

    pub fn add_weighted_terms(&mut self, cell_id: u32, terms: BTreeMap<String, u32>) {
        self.replace_document_terms(cell_id, terms);
    }

    fn replace_document_terms(&mut self, cell_id: u32, terms: BTreeMap<String, u32>) {
        if let Some(old_terms) = self.docs.get(&cell_id) {
            for term in old_terms.keys() {
                if let Some(posting) = self.postings.get_mut(term) {
                    posting.remove(&cell_id);
                }
            }
            self.postings.retain(|_, posting| !posting.is_empty());
        }
        for term in terms.keys() {
            self.postings
                .entry(term.clone())
                .or_default()
                .insert(cell_id);
        }
        self.doc_lengths
            .insert(cell_id, terms.values().copied().sum::<u32>().max(1));
        self.docs.insert(cell_id, terms);
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<ScoredCandidate> {
        let query_terms = tokenize(query);
        let doc_count = self.docs.len() as u64;
        let avg_len_q10 = self.average_len_q10();
        let mut scores = BTreeMap::<u32, u64>::new();
        for term in query_terms {
            let Some(posting) = self.postings.get(&term) else {
                continue;
            };
            let idf_q10 = ((doc_count + 1) * 1024) / (posting.len() as u64 + 1);
            for cell_id in posting {
                let doc = &self.docs[cell_id];
                let tf = u64::from(*doc.get(&term).unwrap_or(&0));
                let len_q10 = u64::from(*self.doc_lengths.get(cell_id).unwrap_or(&1)) * 1024;
                let norm_q10 = 256 + (768 * len_q10 / avg_len_q10.max(1));
                let denom_q10 = (tf * 1024) + norm_q10;
                let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
                *scores.entry(*cell_id).or_default() += idf_q10 * tf_norm_q10;
            }
        }
        ranked(scores, limit)
    }

    fn average_len_q10(&self) -> u64 {
        let total = self
            .doc_lengths
            .values()
            .copied()
            .map(u64::from)
            .sum::<u64>();
        if self.doc_lengths.is_empty() {
            1024
        } else {
            total * 1024 / self.doc_lengths.len() as u64
        }
    }
}

impl SearchIndexes {
    pub fn add_document(&mut self, cell_id: u32, text: &str) {
        self.lexical.add_document(cell_id, text);
    }

    pub fn add_document_fields(&mut self, cell_id: u32, fields: &[(&str, u32)]) {
        self.lexical.add_document_fields(cell_id, fields);
    }

    pub fn add_weighted_terms(&mut self, cell_id: u32, terms: BTreeMap<String, u32>) {
        self.lexical.add_weighted_terms(cell_id, terms);
    }

    pub fn add_vector(&mut self, cell_id: u32, vector: Vec<i16>) {
        self.vector.add_vector(cell_id, vector);
    }

    pub fn search(&self, query: SearchQuery<'_>) -> Vec<SearchResult> {
        match query.mode {
            SearchMode::Keyword => self
                .lexical
                .search(query.text, query.limit)
                .into_iter()
                .map(SearchResult::from_lexical)
                .collect(),
            SearchMode::Vector | SearchMode::VectorExact => query
                .vector
                .map(|vector| self.vector.search_dot(vector, query.limit))
                .unwrap_or_default()
                .into_iter()
                .map(SearchResult::from_vector)
                .collect(),
            SearchMode::Hybrid => self.hybrid_search(query),
        }
    }

    fn hybrid_search(&self, query: SearchQuery<'_>) -> Vec<SearchResult> {
        let Some(vector) = query.vector else {
            return self.search(SearchQuery {
                mode: SearchMode::Keyword,
                ..query
            });
        };
        let lexical = self.lexical.search(query.text, query.limit.max(32));
        let vector = self.vector.search_dot(vector, query.limit.max(32));
        let mut results = BTreeMap::<u32, SearchResult>::new();
        apply_rrf(&mut results, lexical, true);
        apply_rrf(&mut results, vector, false);
        let mut values = results.into_values().collect::<Vec<_>>();
        values.sort_by_key(|result| (Reverse(result.score), result.cell_id));
        values.truncate(query.limit);
        values
    }
}

impl SearchResult {
    fn from_lexical(candidate: ScoredCandidate) -> Self {
        Self {
            cell_id: candidate.cell_id,
            score: candidate.score,
            lexical_score: candidate.score,
            vector_score: 0,
        }
    }

    fn from_vector(candidate: ScoredCandidate) -> Self {
        Self {
            cell_id: candidate.cell_id,
            score: candidate.score,
            lexical_score: 0,
            vector_score: candidate.score,
        }
    }
}

impl VectorIndex {
    pub fn add_vector(&mut self, cell_id: u32, vector: Vec<i16>) {
        self.vectors.insert(cell_id, vector);
    }

    pub fn search_dot(&self, query: &[i16], limit: usize) -> Vec<ScoredCandidate> {
        let scores = self
            .vectors
            .iter()
            .filter_map(|(cell_id, vector)| {
                self.metric
                    .distance(query, vector)
                    .map(|score| (*cell_id, score))
            })
            .collect();
        ranked(scores, limit)
    }
}

pub(super) fn ranked(scores: BTreeMap<u32, u64>, limit: usize) -> Vec<ScoredCandidate> {
    let mut values: Vec<_> = scores
        .into_iter()
        .map(|(cell_id, score)| ScoredCandidate { cell_id, score })
        .collect();
    values.sort_by_key(|candidate| (Reverse(candidate.score), candidate.cell_id));
    values.truncate(limit);
    values
}

fn apply_rrf(
    results: &mut BTreeMap<u32, SearchResult>,
    ranked: Vec<ScoredCandidate>,
    lexical: bool,
) {
    for (rank, candidate) in ranked.into_iter().enumerate() {
        let rrf = 1_000_000 / (60 + rank as u64 + 1);
        let result = results.entry(candidate.cell_id).or_insert(SearchResult {
            cell_id: candidate.cell_id,
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
