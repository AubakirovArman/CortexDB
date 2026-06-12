use std::collections::{BTreeMap, BTreeSet};

use super::{tokenize, Bm25Index};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextAnalyzer {
    field_weights: BTreeMap<String, u32>,
    stopwords: BTreeSet<String>,
    lemmas: BTreeMap<String, String>,
    stemmer: Stemmer,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    English,
    Russian,
    Kazakh,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stemmer {
    #[default]
    None,
    EnglishLight,
    RussianLight,
    KazakhLight,
}

impl Default for TextAnalyzer {
    fn default() -> Self {
        Self {
            field_weights: BTreeMap::from([
                ("title".to_owned(), 8),
                ("path".to_owned(), 5),
                ("document_id".to_owned(), 5),
                ("section".to_owned(), 5),
                ("project".to_owned(), 4),
                ("entity".to_owned(), 4),
                ("sector".to_owned(), 4),
                ("body".to_owned(), 1),
                ("source".to_owned(), 4),
                ("chunk_id".to_owned(), 2),
            ]),
            stopwords: BTreeSet::new(),
            lemmas: BTreeMap::new(),
            stemmer: Stemmer::None,
        }
    }
}

impl TextAnalyzer {
    pub fn for_language(language: Language) -> Self {
        Self {
            stopwords: language_stopwords(language)
                .iter()
                .map(|word| (*word).to_owned())
                .collect(),
            stemmer: match language {
                Language::English => Stemmer::EnglishLight,
                Language::Russian => Stemmer::RussianLight,
                Language::Kazakh => Stemmer::KazakhLight,
            },
            ..Self::default()
        }
    }

    pub fn with_stopwords(mut self, stopwords: impl IntoIterator<Item = String>) -> Self {
        self.stopwords = stopwords.into_iter().collect();
        self
    }

    pub fn with_lemmas(mut self, lemmas: impl IntoIterator<Item = (String, String)>) -> Self {
        self.lemmas = lemmas.into_iter().collect();
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
                let term = self.normalize_term(term);
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

    fn normalize_term(&self, term: String) -> String {
        let stemmed = match self.stemmer {
            Stemmer::None => term,
            Stemmer::EnglishLight => light_english_stem(term),
            Stemmer::RussianLight => light_suffix_stem(
                term,
                &[
                    "ами", "ями", "ого", "ему", "ами", "ов", "ев", "ый", "ая", "ое", "ые", "а",
                    "ы", "и",
                ],
            ),
            Stemmer::KazakhLight => light_suffix_stem(
                term,
                &[
                    "лары", "лері", "дың", "дің", "тың", "тің", "лар", "лер", "дар", "дер", "тар",
                    "тер",
                ],
            ),
        };
        self.lemmas.get(&stemmed).cloned().unwrap_or(stemmed)
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

fn language_stopwords(language: Language) -> &'static [&'static str] {
    match language {
        Language::English => &["a", "an", "and", "for", "of", "the", "to"],
        Language::Russian => &["в", "и", "на", "по", "с", "за"],
        Language::Kazakh => &["және", "мен", "үшін", "бұл"],
    }
}

fn light_english_stem(mut term: String) -> String {
    for suffix in ["ing", "ed", "es", "s"] {
        if term.len() > suffix.len() + 3 && term.ends_with(suffix) {
            term.truncate(term.len() - suffix.len());
            break;
        }
    }
    term
}

fn light_suffix_stem(mut term: String, suffixes: &[&str]) -> String {
    for suffix in suffixes {
        if term.chars().count() > suffix.chars().count() + 3 && term.ends_with(suffix) {
            let new_len = term.len() - suffix.len();
            term.truncate(new_len);
            break;
        }
    }
    term
}
