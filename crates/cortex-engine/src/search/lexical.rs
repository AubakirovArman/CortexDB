use std::collections::{BTreeMap, BTreeSet};

use crate::query::metadata::lexical_field_weight;

use super::{
    bm25_idf_q16, bm25_term_score_with_idf_q16, query_understanding, ranked, Bm25Config,
    Bm25TermInput, ScoredCandidate, TextAnalyzer, TextAnalyzerConfig,
};

#[derive(Clone, Debug, Default)]
pub struct Bm25Index {
    docs: BTreeMap<u32, BTreeMap<String, u32>>,
    doc_lengths: BTreeMap<u32, u32>,
    postings: BTreeMap<String, BTreeSet<u32>>,
    field_docs: BTreeMap<u32, BTreeMap<String, BTreeMap<String, u32>>>,
    field_doc_lengths: BTreeMap<String, BTreeMap<u32, u32>>,
    config: Bm25Config,
    analyzer: TextAnalyzer,
}

impl Bm25Index {
    pub fn with_config(config: Bm25Config) -> Self {
        Self {
            config,
            ..Self::default()
        }
    }

    pub fn with_analyzer_config(analyzer_config: TextAnalyzerConfig) -> Self {
        Self {
            analyzer: TextAnalyzer::with_config(analyzer_config),
            ..Self::default()
        }
    }

    pub fn with_analyzer(analyzer: TextAnalyzer) -> Self {
        Self {
            analyzer,
            ..Self::default()
        }
    }

    pub fn add_document(&mut self, cell_id: u32, text: &str) {
        self.add_document_fields(cell_id, &[(text, 1)]);
    }

    pub fn add_document_fields(&mut self, cell_id: u32, fields: &[(&str, u32)]) {
        let mut terms = BTreeMap::<String, u32>::new();
        for (text, weight) in fields {
            let weight = (*weight).max(1);
            for term in self.analyzer.tokenize(text) {
                *terms.entry(term).or_default() += weight;
            }
        }
        self.replace_document_terms(cell_id, terms);
        self.clear_field_terms(cell_id);
    }

    pub fn add_weighted_terms(&mut self, cell_id: u32, terms: BTreeMap<String, u32>) {
        self.replace_document_terms(cell_id, terms);
        self.clear_field_terms(cell_id);
    }

    pub fn add_field_terms(
        &mut self,
        cell_id: u32,
        fields: BTreeMap<String, BTreeMap<String, u32>>,
    ) {
        self.replace_document_terms(cell_id, weighted_terms_from_fields(&fields));
        self.replace_field_terms(cell_id, fields);
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<ScoredCandidate> {
        let analyzed =
            query_understanding::analyze_search_query_with_analyzer(query, &self.analyzer);
        let query_terms = analyzed.weighted_terms;
        let doc_count = self.docs.len();
        let avg_len_q16 = self.average_len_q16();
        let mut scores = BTreeMap::<u32, u64>::new();
        for (term, query_weight) in query_terms {
            let Some(posting) = self.postings.get(&term) else {
                continue;
            };
            let idf_q16 = bm25_idf_q16(doc_count, posting.len());
            for cell_id in posting {
                let doc = &self.docs[cell_id];
                let field_score = self.field_score_q16(*cell_id, &term, idf_q16, query_weight);
                let score = if field_score > 0 {
                    field_score
                } else {
                    document_score_q16(
                        doc,
                        &term,
                        idf_q16,
                        query_weight,
                        *self.doc_lengths.get(cell_id).unwrap_or(&1),
                        avg_len_q16,
                        self.config,
                    )
                };
                *scores.entry(*cell_id).or_default() += score;
            }
        }
        ranked(scores, limit)
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

    fn replace_field_terms(
        &mut self,
        cell_id: u32,
        fields: BTreeMap<String, BTreeMap<String, u32>>,
    ) {
        self.clear_field_terms(cell_id);
        for (field, terms) in &fields {
            self.field_doc_lengths
                .entry(field.clone())
                .or_default()
                .insert(cell_id, terms.values().copied().sum::<u32>().max(1));
        }
        self.field_docs.insert(cell_id, fields);
    }

    fn clear_field_terms(&mut self, cell_id: u32) {
        let Some(old_fields) = self.field_docs.remove(&cell_id) else {
            return;
        };
        for field in old_fields.keys() {
            if let Some(lengths) = self.field_doc_lengths.get_mut(field) {
                lengths.remove(&cell_id);
            }
        }
        self.field_doc_lengths
            .retain(|_, lengths| !lengths.is_empty());
    }

    fn field_score_q16(&self, cell_id: u32, term: &str, idf_q16: u64, query_weight: u32) -> u64 {
        let Some(fields) = self.field_docs.get(&cell_id) else {
            return 0;
        };
        fields
            .iter()
            .filter_map(|(field, terms)| terms.get(term).map(|tf| (field, *tf)))
            .map(|(field, tf)| {
                let length = self
                    .field_doc_lengths
                    .get(field)
                    .and_then(|lengths| lengths.get(&cell_id))
                    .copied()
                    .unwrap_or(1);
                let avg_len_q16 = self.average_field_len_q16(field);
                bm25_term_score_with_idf_q16(
                    Bm25TermInput {
                        tf,
                        doc_len: length,
                        avg_len_q16,
                        query_weight,
                        field_weight: lexical_field_weight(field),
                    },
                    idf_q16,
                    self.config,
                )
            })
            .sum()
    }

    fn average_len_q16(&self) -> u64 {
        let total = self
            .doc_lengths
            .values()
            .copied()
            .map(u64::from)
            .sum::<u64>();
        if self.doc_lengths.is_empty() {
            65_536
        } else {
            total * 65_536 / self.doc_lengths.len() as u64
        }
    }

    fn average_field_len_q16(&self, field: &str) -> u64 {
        let Some(lengths) = self.field_doc_lengths.get(field) else {
            return 65_536;
        };
        let total = lengths.values().copied().map(u64::from).sum::<u64>();
        if lengths.is_empty() {
            65_536
        } else {
            total * 65_536 / lengths.len() as u64
        }
    }
}

fn document_score_q16(
    doc: &BTreeMap<String, u32>,
    term: &str,
    idf_q16: u64,
    query_weight: u32,
    doc_len: u32,
    avg_len_q16: u64,
    config: Bm25Config,
) -> u64 {
    bm25_term_score_with_idf_q16(
        Bm25TermInput {
            tf: *doc.get(term).unwrap_or(&0),
            doc_len,
            avg_len_q16,
            query_weight,
            field_weight: 1,
        },
        idf_q16,
        config,
    )
}

fn weighted_terms_from_fields(
    fields: &BTreeMap<String, BTreeMap<String, u32>>,
) -> BTreeMap<String, u32> {
    let mut terms = BTreeMap::new();
    for (field, field_terms) in fields {
        let weight = lexical_field_weight(field);
        for (term, frequency) in field_terms {
            *terms.entry(term.clone()).or_default() += frequency.saturating_mul(weight);
        }
    }
    terms
}
