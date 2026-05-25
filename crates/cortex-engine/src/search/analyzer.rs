use std::collections::{BTreeMap, BTreeSet};

use super::{tokenize, Bm25Index};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextAnalyzer {
    field_weights: BTreeMap<String, u32>,
    stopwords: BTreeSet<String>,
}

impl Default for TextAnalyzer {
    fn default() -> Self {
        Self {
            field_weights: BTreeMap::from([
                ("title".to_owned(), 6),
                ("body".to_owned(), 1),
                ("source".to_owned(), 1),
            ]),
            stopwords: BTreeSet::new(),
        }
    }
}

impl TextAnalyzer {
    pub fn with_stopwords(mut self, stopwords: impl IntoIterator<Item = String>) -> Self {
        self.stopwords = stopwords.into_iter().collect();
        self
    }

    pub fn weighted_terms<'a>(
        &self,
        fields: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> BTreeMap<String, u32> {
        let mut terms = BTreeMap::new();
        for (field, text) in fields {
            let weight = self.field_weights.get(field).copied().unwrap_or(1);
            for term in tokenize(text) {
                if !self.stopwords.contains(&term) {
                    *terms.entry(term).or_default() += weight;
                }
            }
        }
        terms
    }

    pub fn add_document_fields<'a>(
        &self,
        index: &mut Bm25Index,
        cell_id: u32,
        fields: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) {
        index.add_weighted_terms(cell_id, self.weighted_terms(fields));
    }
}

pub fn mean_reciprocal_rank_q16(index: &Bm25Index, queries: &[(&str, u32)], limit: usize) -> u16 {
    if queries.is_empty() {
        return 0;
    }
    let total = queries
        .iter()
        .map(|(query, relevant)| reciprocal_rank_q16(index, query, *relevant, limit))
        .map(u32::from)
        .sum::<u32>();
    (total / queries.len() as u32) as u16
}

fn reciprocal_rank_q16(index: &Bm25Index, query: &str, relevant: u32, limit: usize) -> u16 {
    index
        .search(query, limit)
        .iter()
        .position(|candidate| candidate.cell_id == relevant)
        .map(|position| 65_535 / (position as u16 + 1))
        .unwrap_or(0)
}
