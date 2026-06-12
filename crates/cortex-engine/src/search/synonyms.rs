use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use super::tokenize;
use crate::database::Database;
use crate::error::EngineResult;

const ACSYN_MAGIC: &str = "CORTEXDB_ACSYN_V1";
const ACSYN_FILE_NAME: &str = "corpus.acsyn";

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
    term_docs: BTreeMap<String, u32>,
    pair_docs: BTreeMap<(String, String), u32>,
    abbreviation_pairs: BTreeMap<(String, String), u32>,
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

pub fn expand_query_with_corpus_synonyms(
    query: &str,
    dictionary: &CorpusSynonymDictionary,
    max_synonyms: usize,
    max_synonyms_per_term: usize,
) -> Option<String> {
    if max_synonyms == 0 || max_synonyms_per_term == 0 {
        return None;
    }
    let query_terms = tokenize(query)
        .into_iter()
        .map(|term| normalize_term(&term))
        .collect::<BTreeSet<_>>();
    let mut seen = query_terms.clone();
    let mut additions = Vec::new();
    for term in query_terms {
        for synonym in dictionary
            .synonyms_for(&term)
            .into_iter()
            .take(max_synonyms_per_term)
        {
            if seen.insert(synonym.clone()) {
                additions.push(synonym);
                if additions.len() >= max_synonyms {
                    break;
                }
            }
        }
        if additions.len() >= max_synonyms {
            break;
        }
    }
    if additions.is_empty() {
        None
    } else {
        Some(format!("{query} {}", additions.join(" ")))
    }
}

impl Database {
    pub fn corpus_synonym_dictionary_path(&self) -> std::path::PathBuf {
        self.root_path().join(ACSYN_FILE_NAME)
    }

    pub fn corpus_synonym_dictionary(
        &self,
        options: CorpusSynonymOptions,
    ) -> CorpusSynonymDictionary {
        let payloads = self
            .snapshot_versions()
            .into_iter()
            .map(|version| String::from_utf8_lossy(&version.payload).into_owned())
            .collect::<Vec<_>>();
        build_corpus_synonym_dictionary(payloads.iter().map(String::as_str), options)
    }

    pub fn persist_corpus_synonym_dictionary(
        &self,
        options: CorpusSynonymOptions,
    ) -> EngineResult<CorpusSynonymDictionary> {
        let dictionary = self.corpus_synonym_dictionary(options);
        write_acsyn_dictionary(&self.corpus_synonym_dictionary_path(), &dictionary)?;
        Ok(dictionary)
    }

    pub(crate) fn publish_checkpoint_corpus_synonym_dictionary(&self) -> EngineResult<()> {
        self.persist_corpus_synonym_dictionary(CorpusSynonymOptions::default())?;
        Ok(())
    }

    pub fn read_persisted_corpus_synonym_dictionary(
        &self,
    ) -> EngineResult<Option<CorpusSynonymDictionary>> {
        let path = self.corpus_synonym_dictionary_path();
        if !path.exists() {
            return Ok(None);
        }
        read_acsyn_dictionary(&path).map(Some).map_err(Into::into)
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

pub fn write_acsyn_dictionary(
    path: &Path,
    dictionary: &CorpusSynonymDictionary,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_extension("acsyn.tmp");
    {
        let mut file = fs::File::create(&tmp_path)?;
        writeln!(file, "{ACSYN_MAGIC}")?;
        for entry in &dictionary.entries {
            write!(file, "{}\t{}", entry.term, entry.document_frequency)?;
            for synonym in &entry.synonyms {
                write!(
                    file,
                    "\t{}:{}:{}",
                    synonym.term, synonym.score_q16, synonym.cooccurrence_count
                )?;
            }
            writeln!(file)?;
        }
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    if let Some(parent) = path.parent() {
        fs::File::open(parent)?.sync_all()?;
    }
    Ok(())
}

pub fn read_acsyn_dictionary(path: &Path) -> std::io::Result<CorpusSynonymDictionary> {
    let file = fs::File::open(path)?;
    let mut lines = BufReader::new(file).lines();
    let magic = lines.next().transpose()?.unwrap_or_default();
    if magic.trim() != ACSYN_MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid ACSYN magic",
        ));
    }
    let mut entries = Vec::new();
    for line in lines {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let term = fields.next().unwrap_or_default().to_owned();
        let document_frequency = fields
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid ACSYN df")
            })?;
        let mut synonyms = Vec::new();
        for field in fields {
            let parts = field.split(':').collect::<Vec<_>>();
            if parts.len() != 3 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid ACSYN synonym entry",
                ));
            }
            synonyms.push(CorpusSynonymCandidate {
                term: parts[0].to_owned(),
                score_q16: parts[1].parse::<u16>().map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid ACSYN score")
                })?,
                cooccurrence_count: parts[2].parse::<u32>().map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid ACSYN count")
                })?,
            });
        }
        entries.push(CorpusSynonymEntry {
            term,
            document_frequency,
            synonyms,
        });
    }
    Ok(CorpusSynonymDictionary { entries })
}

fn document_terms(document: &str, max_terms: usize) -> Vec<String> {
    let document = document_body_for_synonyms(document);
    let mut seen = BTreeSet::new();
    let mut terms = Vec::new();
    for term in tokenize(document)
        .into_iter()
        .map(|term| normalize_term(&term))
        .filter(|term| is_dictionary_term(term))
    {
        if !seen.insert(term.clone()) {
            continue;
        }
        terms.push(term);
        if terms.len() >= max_terms.max(1) {
            break;
        }
    }
    terms
}

fn document_body_for_synonyms(document: &str) -> &str {
    document
        .split_once("\n\n")
        .map(|(_, body)| body)
        .unwrap_or(document)
}

fn document_abbreviation_pairs(document: &str) -> BTreeSet<(String, String)> {
    let document = document_body_for_synonyms(document);
    let mut pairs = BTreeSet::new();
    let mut search_start = 0;
    while let Some(open_offset) = document[search_start..].find('(') {
        let open = search_start + open_offset;
        let Some(close_offset) = document[open + 1..].find(')') else {
            break;
        };
        let close = open + 1 + close_offset;
        let before = &document[..open];
        let inside = &document[open + 1..close];
        collect_parenthetical_abbreviation_pairs(before, inside, &mut pairs);
        search_start = close + 1;
    }
    pairs
}

fn collect_parenthetical_abbreviation_pairs(
    before: &str,
    inside: &str,
    pairs: &mut BTreeSet<(String, String)>,
) {
    let inside_words = raw_words(inside);
    if inside_words.len() == 1 {
        if let Some(abbreviation) = normalized_abbreviation(&inside_words[0]) {
            if let Some(phrase_terms) =
                matching_phrase_suffix_terms(&raw_words(before), &abbreviation)
            {
                insert_abbreviation_pairs(pairs, &abbreviation, phrase_terms);
            }
        }
    }

    let before_words = raw_words(before);
    let Some(previous_word) = before_words.last() else {
        return;
    };
    let Some(abbreviation) = normalized_abbreviation(previous_word) else {
        return;
    };
    if let Some(phrase_terms) = matching_phrase_terms(&inside_words, &abbreviation) {
        insert_abbreviation_pairs(pairs, &abbreviation, phrase_terms);
    }
}

fn insert_abbreviation_pairs(
    pairs: &mut BTreeSet<(String, String)>,
    abbreviation: &str,
    phrase_terms: Vec<String>,
) {
    for term in phrase_terms {
        if term != abbreviation {
            pairs.insert((abbreviation.to_owned(), term));
        }
    }
}

fn raw_words(text: &str) -> Vec<String> {
    text.split(|value: char| !value.is_alphanumeric())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn normalized_abbreviation(word: &str) -> Option<String> {
    let compact = word
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    let letters = compact
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .count();
    if compact.len() < 2
        || compact.len() > 12
        || letters < 2
        || compact.chars().any(|ch| ch.is_ascii_lowercase())
    {
        return None;
    }
    Some(compact.to_ascii_lowercase())
}

fn matching_phrase_suffix_terms(words: &[String], abbreviation: &str) -> Option<Vec<String>> {
    let letters = abbreviation_letters(abbreviation);
    if words.len() < letters.len() {
        return None;
    }
    let start = words.len() - letters.len();
    matching_phrase_terms(&words[start..], abbreviation)
}

fn matching_phrase_terms(words: &[String], abbreviation: &str) -> Option<Vec<String>> {
    let letters = abbreviation_letters(abbreviation);
    if words.len() < letters.len() {
        return None;
    }
    for window in words.windows(letters.len()) {
        if window
            .iter()
            .filter_map(|word| word.chars().next())
            .map(|ch| ch.to_ascii_lowercase())
            .eq(letters.iter().copied())
        {
            let terms = window
                .iter()
                .map(|word| normalize_term(word))
                .filter(|term| is_dictionary_term(term))
                .collect::<Vec<_>>();
            if !terms.is_empty() {
                return Some(terms);
            }
        }
    }
    None
}

fn abbreviation_letters(abbreviation: &str) -> Vec<char> {
    abbreviation
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn normalize_term(term: &str) -> String {
    term.to_ascii_lowercase()
}

fn is_dictionary_term(term: &str) -> bool {
    term.len() >= 3
        && term.len() <= 40
        && term.chars().any(|ch| ch.is_ascii_alphabetic())
        && !matches!(
            term,
            "and"
                | "are"
                | "but"
                | "can"
                | "did"
                | "does"
                | "for"
                | "from"
                | "has"
                | "have"
                | "how"
                | "into"
                | "not"
                | "our"
                | "the"
                | "their"
                | "this"
                | "that"
                | "was"
                | "were"
                | "what"
                | "when"
                | "where"
                | "which"
                | "who"
                | "why"
                | "with"
        )
}

fn association_score_q16(cooccurrence_count: u32, left_docs: u32, right_docs: u32) -> u16 {
    let denominator = left_docs.min(right_docs).max(1);
    ((u64::from(cooccurrence_count) * 65_535) / u64::from(denominator)).min(65_535) as u16
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use cortex_core::CellId;

    use crate::Database;

    use super::{
        build_corpus_synonym_dictionary, expand_query_with_corpus_synonyms, read_acsyn_dictionary,
        write_acsyn_dictionary, CorpusSynonymDictionaryBuilder, CorpusSynonymOptions,
    };

    #[test]
    fn builds_cooccurrence_dictionary() {
        let docs = [
            "Apollo rollout blocker auth owner DRI",
            "Apollo launch blocker auth deadline",
            "Apollo rollout risk dependency auth",
            "Hermes billing invoice owner",
        ];
        let dictionary = build_corpus_synonym_dictionary(
            docs.iter().copied(),
            CorpusSynonymOptions {
                min_term_document_frequency: 2,
                min_pair_document_frequency: 2,
                max_synonyms_per_term: 4,
                max_terms: 100,
                max_terms_per_document: 64,
            },
        );

        let apollo = dictionary.synonyms_for("apollo");

        assert!(dictionary.terms_with_synonyms() >= 3);
        assert!(apollo.contains(&"auth".to_owned()));
        assert!(apollo.contains(&"rollout".to_owned()));
    }

    #[test]
    fn streaming_builder_matches_batch_dictionary() {
        let docs = [
            "Apollo rollout blocker auth owner DRI",
            "Apollo launch blocker auth deadline",
            "Apollo rollout risk dependency auth",
        ];
        let options = CorpusSynonymOptions {
            min_term_document_frequency: 2,
            min_pair_document_frequency: 2,
            max_synonyms_per_term: 4,
            max_terms: 100,
            max_terms_per_document: 64,
        };
        let batch = build_corpus_synonym_dictionary(docs.iter().copied(), options);
        let mut builder = CorpusSynonymDictionaryBuilder::new();
        for document in docs {
            builder.add_document(document, options);
        }

        assert_eq!(builder.finish(options), batch);
    }

    #[test]
    fn acsyn_roundtrip_preserves_entries() {
        let docs = [
            "security sso rbac permissions",
            "security auth sso policy",
            "security rbac access policy",
        ];
        let dictionary = build_corpus_synonym_dictionary(
            docs.iter().copied(),
            CorpusSynonymOptions {
                min_term_document_frequency: 2,
                min_pair_document_frequency: 2,
                max_synonyms_per_term: 8,
                max_terms: 100,
                max_terms_per_document: 64,
            },
        );
        let path = std::env::temp_dir().join(format!(
            "cortexdb-test-{}.acsyn",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        write_acsyn_dictionary(&path, &dictionary).unwrap();
        let loaded = read_acsyn_dictionary(&path).unwrap();
        let _ = std::fs::remove_file(path);

        assert_eq!(loaded, dictionary);
    }

    #[test]
    fn database_persists_live_corpus_synonym_dictionary() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"scope=default\nstatus=ready\ntype=fact\n\nApollo rollout blocker auth owner".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"scope=default\nstatus=ready\ntype=fact\n\nApollo launch blocker auth deadline"
                .to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(3),
            b"scope=default\nstatus=ready\ntype=fact\n\nApollo rollout risk auth dependency"
                .to_vec(),
        )
        .unwrap();

        let dictionary = db
            .persist_corpus_synonym_dictionary(CorpusSynonymOptions {
                min_term_document_frequency: 2,
                min_pair_document_frequency: 2,
                max_synonyms_per_term: 8,
                max_terms: 100,
                max_terms_per_document: 64,
            })
            .unwrap();
        let loaded = db.read_persisted_corpus_synonym_dictionary().unwrap();

        assert_eq!(loaded, Some(dictionary));
        assert!(db.corpus_synonym_dictionary_path().exists());
    }

    #[test]
    fn expands_query_with_persisted_corpus_synonyms() {
        let docs = [
            "zephyr quartz rollout",
            "zephyr quartz incident",
            "quartz migration note",
        ];
        let dictionary = build_corpus_synonym_dictionary(
            docs.iter().copied(),
            CorpusSynonymOptions {
                min_term_document_frequency: 2,
                min_pair_document_frequency: 2,
                max_synonyms_per_term: 4,
                max_terms: 100,
                max_terms_per_document: 64,
            },
        );

        let expanded = expand_query_with_corpus_synonyms("zephyr status", &dictionary, 4, 2)
            .expect("expected corpus synonym expansion");

        assert!(expanded.contains("quartz"));
        assert!(expanded.starts_with("zephyr status "));
    }

    #[test]
    fn mines_parenthetical_abbreviations_without_frequency_threshold() {
        let docs = [
            "The single sign on (SSO) rollout is tracked by identity.",
            "RBAC (role based access control) migration is owned by security.",
        ];
        let dictionary = build_corpus_synonym_dictionary(
            docs.iter().copied(),
            CorpusSynonymOptions {
                min_term_document_frequency: 3,
                min_pair_document_frequency: 3,
                max_synonyms_per_term: 8,
                max_terms: 100,
                max_terms_per_document: 64,
            },
        );

        let sso = dictionary.synonyms_for("SSO");
        let rbac = dictionary.synonyms_for("rbac");

        assert!(sso.contains(&"single".to_owned()));
        assert!(sso.contains(&"sign".to_owned()));
        assert!(rbac.contains(&"role".to_owned()));
        assert!(rbac.contains(&"based".to_owned()));
        assert!(rbac.contains(&"access".to_owned()));
        assert!(rbac.contains(&"control".to_owned()));
        assert!(dictionary
            .synonyms_for("single")
            .contains(&"sso".to_owned()));
    }
}
