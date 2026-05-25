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
    doc_lengths: BTreeMap<u32, u32>,
    postings: BTreeMap<String, BTreeSet<u32>>,
}

#[derive(Clone, Debug, Default)]
pub struct VectorIndex {
    vectors: BTreeMap<u32, Vec<i16>>,
}

#[derive(Clone, Debug)]
pub struct HnswIndex {
    vectors: BTreeMap<u32, Vec<i16>>,
    links: BTreeMap<u32, BTreeSet<u32>>,
    max_neighbors: usize,
    ef_search: usize,
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
    pub fn new(max_neighbors: usize, ef_search: usize) -> Self {
        Self {
            vectors: BTreeMap::new(),
            links: BTreeMap::new(),
            max_neighbors: max_neighbors.max(1),
            ef_search: ef_search.max(1),
        }
    }

    pub fn add_vector(&mut self, cell_id: u32, vector: Vec<i16>) {
        let neighbors = self.nearest_existing(&vector, self.max_neighbors);
        for neighbor in &neighbors {
            self.links.entry(*neighbor).or_default().insert(cell_id);
        }
        self.links
            .insert(cell_id, neighbors.iter().copied().collect::<BTreeSet<_>>());
        self.vectors.insert(cell_id, vector);
    }

    pub fn search(&self, query: &[i16], limit: usize) -> Vec<ScoredCandidate> {
        let Some(entry) = self.vectors.keys().next().copied() else {
            return Vec::new();
        };
        let mut visited = BTreeSet::new();
        let mut frontier = BTreeSet::from([entry]);
        let mut scores = BTreeMap::new();
        while let Some(candidate) = best_frontier(query, &frontier, &self.vectors) {
            frontier.remove(&candidate);
            if !visited.insert(candidate) || visited.len() > self.ef_search {
                continue;
            }
            let score = dot_nonnegative(query, &self.vectors[&candidate]);
            scores.insert(candidate, score);
            if let Some(neighbors) = self.links.get(&candidate) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        frontier.insert(*neighbor);
                    }
                }
            }
        }
        ranked(scores, limit)
    }

    fn nearest_existing(&self, vector: &[i16], limit: usize) -> Vec<u32> {
        let scores = self
            .vectors
            .iter()
            .map(|(cell_id, existing)| (*cell_id, dot_nonnegative(vector, existing)))
            .collect();
        ranked(scores, limit)
            .into_iter()
            .map(|candidate| candidate.cell_id)
            .collect()
    }
}

impl Default for HnswIndex {
    fn default() -> Self {
        Self::new(8, 32)
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

fn best_frontier(
    query: &[i16],
    frontier: &BTreeSet<u32>,
    vectors: &BTreeMap<u32, Vec<i16>>,
) -> Option<u32> {
    frontier
        .iter()
        .copied()
        .max_by_key(|cell_id| dot_nonnegative(query, &vectors[cell_id]))
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|value: char| !value.is_ascii_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
        .collect()
}
