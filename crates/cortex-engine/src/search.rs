use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScoredCandidate {
    pub cell_id: u32,
    pub score: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Bm25Index {
    docs: BTreeMap<u32, BTreeMap<String, u32>>,
    postings: BTreeMap<String, BTreeSet<u32>>,
}

#[derive(Clone, Debug, Default)]
pub struct VectorIndex {
    vectors: BTreeMap<u32, Vec<i16>>,
}

#[derive(Clone, Debug, Default)]
pub struct HnswIndex {
    exact: VectorIndex,
}

impl Bm25Index {
    pub fn add_document(&mut self, cell_id: u32, text: &str) {
        let mut terms = BTreeMap::<String, u32>::new();
        for term in tokenize(text) {
            *terms.entry(term).or_default() += 1;
        }
        for term in terms.keys() {
            self.postings
                .entry(term.clone())
                .or_default()
                .insert(cell_id);
        }
        self.docs.insert(cell_id, terms);
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<ScoredCandidate> {
        let query_terms = tokenize(query);
        let doc_count = self.docs.len() as u64;
        let mut scores = BTreeMap::<u32, u64>::new();
        for term in query_terms {
            let Some(posting) = self.postings.get(&term) else {
                continue;
            };
            let idf_q10 = ((doc_count + 1) * 1024) / (posting.len() as u64 + 1);
            for cell_id in posting {
                let doc = &self.docs[cell_id];
                let tf = u64::from(*doc.get(&term).unwrap_or(&0));
                let len = doc.values().copied().map(u64::from).sum::<u64>().max(1);
                let tf_norm_q10 = (tf * 2048) / (tf + len);
                *scores.entry(*cell_id).or_default() += idf_q10 * tf_norm_q10;
            }
        }
        ranked(scores, limit)
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
            .map(|(cell_id, vector)| (*cell_id, dot_nonnegative(query, vector)))
            .collect();
        ranked(scores, limit)
    }
}

impl HnswIndex {
    pub fn add_vector(&mut self, cell_id: u32, vector: Vec<i16>) {
        self.exact.add_vector(cell_id, vector);
    }

    pub fn search(&self, query: &[i16], limit: usize) -> Vec<ScoredCandidate> {
        self.exact.search_dot(query, limit)
    }
}

fn ranked(scores: BTreeMap<u32, u64>, limit: usize) -> Vec<ScoredCandidate> {
    let mut values: Vec<_> = scores
        .into_iter()
        .map(|(cell_id, score)| ScoredCandidate { cell_id, score })
        .collect();
    values.sort_by_key(|candidate| (Reverse(candidate.score), candidate.cell_id));
    values.truncate(limit);
    values
}

fn dot_nonnegative(lhs: &[i16], rhs: &[i16]) -> u64 {
    lhs.iter()
        .zip(rhs)
        .map(|(left, right)| i64::from(*left) * i64::from(*right))
        .sum::<i64>()
        .max(0) as u64
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|value: char| !value.is_ascii_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
        .collect()
}
