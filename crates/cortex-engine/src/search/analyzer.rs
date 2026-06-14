use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::manifest::ManifestTextAnalyzerProfile;

use super::{tokenize, Bm25Index};

pub const TEXT_ANALYZER_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextAnalyzer {
    config: TextAnalyzerConfig,
    field_weights: BTreeMap<String, u32>,
    stopwords: BTreeSet<String>,
    lemmas: BTreeMap<String, String>,
    stemmer: Stemmer,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextAnalyzerConfig {
    pub language: Language,
    pub stemming: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Language {
    #[default]
    Neutral = 0,
    English = 1,
    Russian = 2,
    Kazakh = 3,
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
        Self::with_config(TextAnalyzerConfig::default())
    }
}

impl TextAnalyzerConfig {
    pub const fn with_stemming(mut self, stemming: bool) -> Self {
        self.stemming = stemming;
        self
    }

    pub const fn for_language(language: Language) -> Self {
        Self {
            language,
            stemming: true,
        }
    }

    pub fn manifest_profile(self) -> ManifestTextAnalyzerProfile {
        ManifestTextAnalyzerProfile {
            version: TEXT_ANALYZER_VERSION,
            language: self.language as u32,
            stemming: self.stemming,
        }
    }

    pub fn from_manifest_profile(profile: ManifestTextAnalyzerProfile) -> Option<Self> {
        if profile.version != TEXT_ANALYZER_VERSION {
            return None;
        }
        Some(Self {
            language: Language::from_profile_value(profile.language)?,
            stemming: profile.stemming,
        })
    }
}

impl Language {
    fn from_profile_value(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Neutral),
            1 => Some(Self::English),
            2 => Some(Self::Russian),
            3 => Some(Self::Kazakh),
            _ => None,
        }
    }
}

impl TextAnalyzer {
    pub fn with_config(config: TextAnalyzerConfig) -> Self {
        let stopwords = language_stopwords(config.language)
            .iter()
            .map(|word| (*word).to_owned())
            .collect();
        Self {
            config,
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
            stopwords,
            lemmas: BTreeMap::new(),
            stemmer: stemmer_for_config(config),
        }
    }

    pub fn for_language(language: Language) -> Self {
        Self::with_config(TextAnalyzerConfig::for_language(language))
    }

    pub fn config(&self) -> TextAnalyzerConfig {
        self.config
    }

    pub fn with_stopwords(mut self, stopwords: impl IntoIterator<Item = String>) -> Self {
        self.stopwords = stopwords.into_iter().collect();
        self
    }

    pub fn with_lemmas(mut self, lemmas: impl IntoIterator<Item = (String, String)>) -> Self {
        self.lemmas = lemmas.into_iter().collect();
        self
    }

    pub fn tokenize(&self, text: &str) -> Vec<String> {
        tokenize(text)
            .into_iter()
            .map(|term| self.normalize_term(term))
            .filter(|term| !self.stopwords.contains(term))
            .collect()
    }

    pub fn weighted_terms<'a>(
        &self,
        fields: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> BTreeMap<String, u32> {
        let mut terms = BTreeMap::new();
        for (field, text) in fields {
            let weight = self.field_weights.get(field).copied().unwrap_or(1);
            for term in self.tokenize(text) {
                *terms.entry(term).or_default() += weight;
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
                    "ами", "ями", "ого", "ему", "ому", "ам", "ям", "ах", "ях", "ом", "ем", "ов",
                    "ев", "ый", "ая", "ое", "ые", "а", "ы", "и", "у", "е",
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
        Language::Neutral => &[],
        Language::English => &["a", "an", "and", "for", "of", "the", "to"],
        Language::Russian => &["в", "и", "на", "по", "с", "за"],
        Language::Kazakh => &["және", "мен", "үшін", "бұл"],
    }
}

fn stemmer_for_config(config: TextAnalyzerConfig) -> Stemmer {
    if !config.stemming {
        return Stemmer::None;
    }
    match config.language {
        Language::Neutral => Stemmer::None,
        Language::English => Stemmer::EnglishLight,
        Language::Russian => Stemmer::RussianLight,
        Language::Kazakh => Stemmer::KazakhLight,
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
