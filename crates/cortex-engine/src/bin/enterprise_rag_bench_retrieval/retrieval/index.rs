use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use cortex_engine::Database;
use cortex_storage::indexes::LexicalIndex;

use super::cache::{
    merge_lexical, merged_lexical_cache_key, write_merged_lexical_cache, MERGED_LEXICAL_CACHE_FILE,
    MERGED_LEXICAL_CACHE_MANIFEST_FILE,
};
use super::overview::{
    expand_overview_query, is_overview_query, overview_path_score, OverviewQueryProfile,
};
use super::scoring::{
    average_field_len_q10, average_len_q10, candidate_doc_maps, lexical_field_weight,
    source_allowed_map,
};

const MAX_QUERY_TERMS_FOR_SEARCH: usize = 8;
const MAX_POSTING_SCAN: usize = 25_000;

pub struct BenchmarkRetrievalIndex {
    lexical: LexicalIndex,
    candidate_to_doc_id: BTreeMap<u32, String>,
    candidate_to_rel_path: BTreeMap<u32, String>,
    source_allowed: BTreeMap<String, BTreeSet<u32>>,
    allowed: BTreeSet<u32>,
    all_avg_len_q10: u64,
    field_avg_len_q10: BTreeMap<String, u64>,
}

impl BenchmarkRetrievalIndex {
    pub fn load(db: &Database, uuid_index: &BTreeMap<String, String>) -> Result<Self, String> {
        let cache_key = merged_lexical_cache_key(db);
        let cache_path = db.root_path().join(MERGED_LEXICAL_CACHE_FILE);
        let cache_manifest_path = db.root_path().join(MERGED_LEXICAL_CACHE_MANIFEST_FILE);
        if cache_manifest_path.exists()
            && cache_path.exists()
            && fs::read_to_string(&cache_manifest_path).ok().as_deref() == Some(cache_key.as_str())
        {
            if let Ok(lexical) = LexicalIndex::read(&cache_path) {
                return Ok(Self::from_lexical(lexical, uuid_index));
            }
        }

        let mut lexical = LexicalIndex::default();
        let segments_path = db.root_path().join("segments");
        for segment in &db.manifest().live_segments {
            let path = segments_path.join(format!("segment-{}.aci", segment.id));
            let segment_lexical = LexicalIndex::read(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            merge_lexical(&mut lexical, segment_lexical);
        }
        write_merged_lexical_cache(&cache_path, &cache_manifest_path, &cache_key, &lexical)?;
        Ok(Self::from_lexical(lexical, uuid_index))
    }

    pub fn from_lexical(lexical: LexicalIndex, uuid_index: &BTreeMap<String, String>) -> Self {
        let (candidate_to_doc_id, candidate_to_source_type, candidate_to_rel_path) =
            candidate_doc_maps(uuid_index, &lexical.doc_lengths);
        let allowed = candidate_to_doc_id.keys().copied().collect::<BTreeSet<_>>();
        let source_allowed = source_allowed_map(&candidate_to_source_type);
        let all_avg_len_q10 = average_len_q10(&lexical.doc_lengths, &allowed);
        let field_avg_len_q10 = lexical
            .field_doc_lengths
            .keys()
            .map(|field| {
                (
                    field.clone(),
                    average_field_len_q10(&lexical.field_doc_lengths, field),
                )
            })
            .collect();
        Self {
            lexical,
            candidate_to_doc_id,
            candidate_to_rel_path,
            source_allowed,
            allowed,
            all_avg_len_q10,
            field_avg_len_q10,
        }
    }

    pub fn search_doc_ids(
        &self,
        query: &str,
        source_types: &[String],
        limit: usize,
    ) -> Vec<String> {
        let original_query = query;
        let expanded_query = expand_overview_query(query);
        let query = expanded_query.as_deref().unwrap_or(query);
        let preferred = self.allowed_for_source_types(source_types);
        let mut candidates = self.metadata_candidates(original_query, limit);
        let mut seen = candidates.iter().copied().collect::<BTreeSet<_>>();
        let lexical_limit = limit.saturating_sub(candidates.len());
        let lexical_candidates = if preferred.is_empty() {
            self.search_candidates(query, &self.allowed, lexical_limit)
        } else {
            self.search_candidates(query, &preferred, lexical_limit)
        };
        candidates.extend(
            lexical_candidates
                .into_iter()
                .filter(|candidate| seen.insert(*candidate)),
        );
        if candidates.len() < limit && !preferred.is_empty() {
            candidates.extend(
                self.search_candidates(query, &self.allowed, limit)
                    .into_iter()
                    .filter(|candidate| seen.insert(*candidate))
                    .take(limit - candidates.len()),
            );
        }
        candidates
            .into_iter()
            .filter_map(|candidate| self.candidate_to_doc_id.get(&candidate).cloned())
            .collect()
    }

    fn metadata_candidates(&self, query: &str, limit: usize) -> Vec<u32> {
        if !is_overview_query(query) {
            return Vec::new();
        }
        let profile = OverviewQueryProfile::new(query);
        let mut scored = self
            .candidate_to_rel_path
            .iter()
            .filter_map(|(candidate, rel_path)| {
                let score = overview_path_score(&profile, rel_path);
                (score > 0).then_some((*candidate, score))
            })
            .collect::<Vec<_>>();
        scored.sort_by_key(|(candidate, score)| (Reverse(*score), *candidate));
        scored
            .into_iter()
            .map(|(candidate, _)| candidate)
            .take(limit)
            .collect()
    }

    fn search_candidates(&self, query: &str, allowed: &BTreeSet<u32>, limit: usize) -> Vec<u32> {
        if limit == 0 {
            return Vec::new();
        }
        let all_allowed = allowed.len() == self.allowed.len();
        let doc_count = allowed.len().max(1) as u64;
        let avg_len_q10 = if all_allowed {
            self.all_avg_len_q10
        } else {
            average_len_q10(&self.lexical.doc_lengths, allowed)
        };
        let mut scores = BTreeMap::<u32, u64>::new();
        for term in self.search_terms(query) {
            let Some(posting) = self.lexical.terms.get(&term) else {
                continue;
            };
            let visible_count = if all_allowed {
                posting.len().min(MAX_POSTING_SCAN)
            } else {
                posting
                    .iter()
                    .filter(|id| allowed.contains(id))
                    .take(MAX_POSTING_SCAN)
                    .count()
            } as u64;
            if visible_count == 0 {
                continue;
            }
            let idf_q10 = ((doc_count + 1) * 1024) / (visible_count + 1);
            let candidates = posting
                .iter()
                .filter(|id| all_allowed || allowed.contains(id));
            for candidate in candidates.take(MAX_POSTING_SCAN) {
                let field_score = self.field_score_q10(&term, *candidate, idf_q10);
                let score = if field_score > 0 {
                    field_score
                } else {
                    let tf = u64::from(self.term_frequency(&term, *candidate));
                    let len_q10 =
                        u64::from(*self.lexical.doc_lengths.get(candidate).unwrap_or(&1)) * 1024;
                    let norm_q10 = 256 + (768 * len_q10 / avg_len_q10.max(1));
                    let denom_q10 = (tf * 1024) + norm_q10;
                    let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
                    idf_q10 * tf_norm_q10
                };
                *scores.entry(*candidate).or_default() += score;
            }
        }
        let mut ranked = scores.into_iter().collect::<Vec<_>>();
        ranked.sort_by_key(|(candidate, score)| (Reverse(*score), *candidate));
        ranked
            .into_iter()
            .map(|(candidate, _)| candidate)
            .take(limit)
            .collect()
    }

    fn search_terms(&self, query: &str) -> Vec<String> {
        let mut terms = BTreeMap::<String, usize>::new();
        for term in cortex_engine::search::tokenize(query) {
            if let Some(posting) = self.lexical.terms.get(&term) {
                terms
                    .entry(term)
                    .and_modify(|size| *size = (*size).min(posting.len()))
                    .or_insert(posting.len());
            }
        }
        let mut terms = terms.into_iter().collect::<Vec<_>>();
        terms.sort_by_key(|(term, posting_size)| (*posting_size, term.clone()));
        terms
            .into_iter()
            .map(|(term, _)| term)
            .take(MAX_QUERY_TERMS_FOR_SEARCH)
            .collect()
    }

    fn allowed_for_source_types(&self, source_types: &[String]) -> BTreeSet<u32> {
        if source_types.is_empty() {
            return BTreeSet::new();
        }
        let mut allowed = BTreeSet::new();
        for source_type in source_types {
            if let Some(candidates) = self.source_allowed.get(source_type) {
                allowed.extend(candidates);
            }
        }
        allowed
    }

    fn term_frequency(&self, term: &str, candidate: u32) -> u32 {
        self.lexical
            .term_frequencies
            .get(term)
            .and_then(|values| values.get(&candidate))
            .copied()
            .unwrap_or(1)
    }

    fn field_score_q10(&self, term: &str, candidate: u32, idf_q10: u64) -> u64 {
        self.lexical
            .field_term_frequencies
            .iter()
            .filter_map(|(field, terms)| {
                let tf = terms
                    .get(term)
                    .and_then(|values| values.get(&candidate))
                    .copied()?;
                Some((field, tf))
            })
            .map(|(field, tf)| {
                let tf = u64::from(tf);
                let len_q10 = self
                    .lexical
                    .field_doc_lengths
                    .get(field)
                    .and_then(|lengths| lengths.get(&candidate))
                    .copied()
                    .map(u64::from)
                    .unwrap_or(1)
                    * 1024;
                let avg_len_q10 = self.field_avg_len_q10.get(field).copied().unwrap_or(1024);
                let norm_q10 = 256 + (768 * len_q10 / avg_len_q10.max(1));
                let denom_q10 = (tf * 1024) + norm_q10;
                let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
                idf_q10
                    .saturating_mul(tf_norm_q10)
                    .saturating_mul(u64::from(lexical_field_weight(field)))
            })
            .sum()
    }
}
