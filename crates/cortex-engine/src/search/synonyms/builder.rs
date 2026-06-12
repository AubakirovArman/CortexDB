use std::collections::BTreeMap;

use super::terms::{association_score_q16, document_abbreviation_pairs, document_terms};
use super::types::{
    CorpusSynonymCandidate, CorpusSynonymDictionary, CorpusSynonymDictionaryBuilder,
    CorpusSynonymEntry, CorpusSynonymOptions,
};

impl CorpusSynonymDictionaryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_document(&mut self, document: &str, options: CorpusSynonymOptions) {
        let terms = document_terms(document, options.max_terms_per_document);
        for term in &terms {
            *self.term_docs.entry(term.clone()).or_default() += 1;
        }
        for left_index in 0..terms.len() {
            for right in terms.iter().skip(left_index + 1) {
                let left = &terms[left_index];
                let pair = if left <= right {
                    (left.clone(), right.clone())
                } else {
                    (right.clone(), left.clone())
                };
                *self.pair_docs.entry(pair).or_default() += 1;
            }
        }
        for pair in document_abbreviation_pairs(document) {
            *self.abbreviation_pairs.entry(pair).or_default() += 1;
        }
    }

    pub fn finish(self, options: CorpusSynonymOptions) -> CorpusSynonymDictionary {
        let mut candidates_by_term = BTreeMap::<String, Vec<CorpusSynonymCandidate>>::new();
        for ((left, right), cooccurrence_count) in self.pair_docs {
            if cooccurrence_count < options.min_pair_document_frequency {
                continue;
            }
            let Some(left_docs) = self.term_docs.get(&left).copied() else {
                continue;
            };
            let Some(right_docs) = self.term_docs.get(&right).copied() else {
                continue;
            };
            if left_docs < options.min_term_document_frequency
                || right_docs < options.min_term_document_frequency
            {
                continue;
            }
            let score_q16 = association_score_q16(cooccurrence_count, left_docs, right_docs);
            push_bidirectional_candidate(
                &mut candidates_by_term,
                left,
                right,
                score_q16,
                cooccurrence_count,
            );
        }
        for ((left, right), cooccurrence_count) in self.abbreviation_pairs {
            push_bidirectional_candidate(
                &mut candidates_by_term,
                left,
                right,
                u16::MAX,
                cooccurrence_count,
            );
        }

        let mut entries = Vec::new();
        for (term, document_frequency) in self.term_docs {
            let has_abbreviation_candidate = candidates_by_term
                .get(&term)
                .map(|synonyms| {
                    synonyms
                        .iter()
                        .any(|candidate| candidate.score_q16 == u16::MAX)
                })
                .unwrap_or(false);
            if document_frequency < options.min_term_document_frequency
                && !has_abbreviation_candidate
            {
                continue;
            }
            let mut synonyms =
                deduplicate_candidates(candidates_by_term.remove(&term).unwrap_or_default());
            synonyms.sort_by_key(|candidate| {
                (
                    std::cmp::Reverse(candidate.score_q16),
                    std::cmp::Reverse(candidate.cooccurrence_count),
                    candidate.term.clone(),
                )
            });
            synonyms.truncate(options.max_synonyms_per_term);
            if synonyms.is_empty() {
                continue;
            }
            entries.push(CorpusSynonymEntry {
                term,
                document_frequency,
                synonyms,
            });
        }
        entries.sort_by_key(|entry| {
            (
                std::cmp::Reverse(entry.document_frequency),
                entry.term.clone(),
            )
        });
        entries.truncate(options.max_terms);
        CorpusSynonymDictionary { entries }
    }
}

pub fn build_corpus_synonym_dictionary<'a>(
    documents: impl IntoIterator<Item = &'a str>,
    options: CorpusSynonymOptions,
) -> CorpusSynonymDictionary {
    let mut builder = CorpusSynonymDictionaryBuilder::new();
    for document in documents {
        builder.add_document(document, options);
    }
    builder.finish(options)
}

fn push_bidirectional_candidate(
    candidates_by_term: &mut BTreeMap<String, Vec<CorpusSynonymCandidate>>,
    left: String,
    right: String,
    score_q16: u16,
    cooccurrence_count: u32,
) {
    candidates_by_term
        .entry(left.clone())
        .or_default()
        .push(CorpusSynonymCandidate {
            term: right.clone(),
            score_q16,
            cooccurrence_count,
        });
    candidates_by_term
        .entry(right)
        .or_default()
        .push(CorpusSynonymCandidate {
            term: left,
            score_q16,
            cooccurrence_count,
        });
}

fn deduplicate_candidates(candidates: Vec<CorpusSynonymCandidate>) -> Vec<CorpusSynonymCandidate> {
    let mut by_term = BTreeMap::<String, CorpusSynonymCandidate>::new();
    for candidate in candidates {
        by_term
            .entry(candidate.term.clone())
            .and_modify(|existing| {
                if (candidate.score_q16, candidate.cooccurrence_count)
                    > (existing.score_q16, existing.cooccurrence_count)
                {
                    *existing = candidate.clone();
                }
            })
            .or_insert(candidate);
    }
    by_term.into_values().collect()
}
