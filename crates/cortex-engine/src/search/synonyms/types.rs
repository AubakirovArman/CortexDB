use std::collections::BTreeMap;

use super::terms::normalize_term;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorpusSynonymCandidate {
    pub term: String,
    pub score_q16: u16,
    pub cooccurrence_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorpusSynonymEntry {
    pub term: String,
    pub document_frequency: u32,
    pub synonyms: Vec<CorpusSynonymCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorpusSynonymDictionary {
    pub entries: Vec<CorpusSynonymEntry>,
}

#[derive(Debug, Default)]
pub struct CorpusSynonymDictionaryBuilder {
    pub(super) term_docs: BTreeMap<String, u32>,
    pub(super) pair_docs: BTreeMap<(String, String), u32>,
    pub(super) abbreviation_pairs: BTreeMap<(String, String), u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CorpusSynonymOptions {
    pub min_term_document_frequency: u32,
    pub min_pair_document_frequency: u32,
    pub max_synonyms_per_term: usize,
    pub max_terms: usize,
    pub max_terms_per_document: usize,
}

impl Default for CorpusSynonymOptions {
    fn default() -> Self {
        Self {
            min_term_document_frequency: 3,
            min_pair_document_frequency: 2,
            max_synonyms_per_term: 8,
            max_terms: 10_000,
            max_terms_per_document: 64,
        }
    }
}

impl CorpusSynonymDictionary {
    pub fn terms_with_synonyms(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| !entry.synonyms.is_empty())
            .count()
    }

    pub fn synonyms_for(&self, term: &str) -> Vec<String> {
        let normalized = normalize_term(term);
        self.entries
            .iter()
            .find(|entry| entry.term == normalized)
            .map(|entry| {
                entry
                    .synonyms
                    .iter()
                    .map(|candidate| candidate.term.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}
