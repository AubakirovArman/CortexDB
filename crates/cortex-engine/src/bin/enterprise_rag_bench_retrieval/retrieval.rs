use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_engine::Database;
use cortex_storage::indexes::LexicalIndex;

#[derive(Debug)]
pub struct BenchmarkRetrievalIndex {
    lexical: LexicalIndex,
    candidate_to_doc_id: BTreeMap<u32, String>,
    candidate_to_source_type: BTreeMap<u32, String>,
    allowed: BTreeSet<u32>,
}

impl BenchmarkRetrievalIndex {
    pub fn load(db: &Database, uuid_index: &BTreeMap<String, String>) -> Result<Self, String> {
        let mut lexical = LexicalIndex::default();
        let segments_path = db.root_path().join("segments");
        for segment in &db.manifest().live_segments {
            let path = segments_path.join(format!("segment-{}.aci", segment.id));
            let segment_lexical = LexicalIndex::read(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            merge_lexical(&mut lexical, segment_lexical);
        }
        Ok(Self::from_lexical(lexical, uuid_index))
    }

    fn from_lexical(lexical: LexicalIndex, uuid_index: &BTreeMap<String, String>) -> Self {
        let (candidate_to_doc_id, candidate_to_source_type) =
            candidate_doc_maps(uuid_index, &lexical.doc_lengths);
        let allowed = candidate_to_doc_id.keys().copied().collect::<BTreeSet<_>>();
        Self {
            lexical,
            candidate_to_doc_id,
            candidate_to_source_type,
            allowed,
        }
    }

    pub fn search_doc_ids(
        &self,
        query: &str,
        source_types: &[String],
        limit: usize,
    ) -> Vec<String> {
        let preferred = self.allowed_for_source_types(source_types);
        let mut candidates = if preferred.is_empty() {
            self.search_candidates(query, &self.allowed, limit)
        } else {
            self.search_candidates(query, &preferred, limit)
        };
        if candidates.len() < limit && !preferred.is_empty() {
            let seen = candidates.iter().copied().collect::<BTreeSet<_>>();
            candidates.extend(
                self.search_candidates(query, &self.allowed, limit)
                    .into_iter()
                    .filter(|candidate| !seen.contains(candidate))
                    .take(limit - candidates.len()),
            );
        }
        candidates
            .into_iter()
            .filter_map(|candidate| self.candidate_to_doc_id.get(&candidate).cloned())
            .collect()
    }

    fn search_candidates(&self, query: &str, allowed: &BTreeSet<u32>, limit: usize) -> Vec<u32> {
        let mut scores = BTreeMap::<u32, u64>::new();
        for term in cortex_engine::search::tokenize(query) {
            let Some(posting) = self.lexical.terms.get(&term) else {
                continue;
            };
            let visible_count = posting.iter().filter(|id| allowed.contains(id)).count() as u64;
            if visible_count == 0 {
                continue;
            }
            let doc_count = allowed.len().max(1) as u64;
            let avg_len_q10 = average_len_q10(&self.lexical.doc_lengths, allowed);
            let idf_q10 = ((doc_count + 1) * 1024) / (visible_count + 1);
            for candidate in posting.iter().filter(|id| allowed.contains(id)) {
                let tf = u64::from(self.term_frequency(&term, *candidate));
                let len_q10 =
                    u64::from(*self.lexical.doc_lengths.get(candidate).unwrap_or(&1)) * 1024;
                let norm_q10 = 256 + (768 * len_q10 / avg_len_q10.max(1));
                let denom_q10 = (tf * 1024) + norm_q10;
                let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
                *scores.entry(*candidate).or_default() += idf_q10 * tf_norm_q10;
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

    fn allowed_for_source_types(&self, source_types: &[String]) -> BTreeSet<u32> {
        if source_types.is_empty() {
            return BTreeSet::new();
        }
        let source_types = source_types.iter().collect::<BTreeSet<_>>();
        self.candidate_to_source_type
            .iter()
            .filter_map(|(candidate, source_type)| {
                source_types.contains(source_type).then_some(*candidate)
            })
            .collect()
    }

    fn term_frequency(&self, term: &str, candidate: u32) -> u32 {
        self.lexical
            .term_frequencies
            .get(term)
            .and_then(|values| values.get(&candidate))
            .copied()
            .unwrap_or(1)
    }
}

fn merge_lexical(dst: &mut LexicalIndex, src: LexicalIndex) {
    for (term, values) in src.terms {
        dst.terms.entry(term).or_default().extend(values);
    }
    dst.doc_lengths.extend(src.doc_lengths);
    for (term, values) in src.term_frequencies {
        dst.term_frequencies.entry(term).or_default().extend(values);
    }
}

fn candidate_doc_maps(
    uuid_index: &BTreeMap<String, String>,
    doc_lengths: &BTreeMap<u32, u32>,
) -> (BTreeMap<u32, String>, BTreeMap<u32, String>) {
    let documents = uuid_index
        .iter()
        .map(|(doc_id, path)| (doc_id.clone(), source_type(path)))
        .collect::<Vec<_>>();
    let mut candidate_to_doc_id = BTreeMap::new();
    let mut candidate_to_source_type = BTreeMap::new();
    for candidate in doc_lengths.keys() {
        if let Some((doc_id, source_type)) = (|| {
            let ordinal = usize::try_from(*candidate).ok()?.checked_sub(1)?;
            documents.get(ordinal)
        })() {
            candidate_to_doc_id.insert(*candidate, doc_id.clone());
            candidate_to_source_type.insert(*candidate, source_type.clone());
        }
    }
    (candidate_to_doc_id, candidate_to_source_type)
}

fn source_type(path: &str) -> String {
    path.split('/').next().unwrap_or("unknown").to_owned()
}

fn average_len_q10(doc_lengths: &BTreeMap<u32, u32>, allowed: &BTreeSet<u32>) -> u64 {
    let mut count = 0u64;
    let mut total = 0u64;
    for candidate in allowed {
        if let Some(length) = doc_lengths.get(candidate) {
            count += 1;
            total += u64::from(*length);
        }
    }
    total
        .saturating_mul(1024)
        .checked_div(count)
        .unwrap_or(1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_mapping_uses_one_based_candidate_ordinal() {
        let uuid_index = BTreeMap::from([
            ("doc-a".to_owned(), "a.json".to_owned()),
            ("doc-b".to_owned(), "b.json".to_owned()),
        ]);
        let doc_lengths = BTreeMap::from([(1, 2), (2, 3), (0, 1)]);

        let (mapped, sources) = candidate_doc_maps(&uuid_index, &doc_lengths);

        assert_eq!(mapped.get(&1), Some(&"doc-a".to_owned()));
        assert_eq!(mapped.get(&2), Some(&"doc-b".to_owned()));
        assert_eq!(sources.get(&1), Some(&"a.json".to_owned()));
        assert_eq!(sources.get(&2), Some(&"b.json".to_owned()));
        assert!(!mapped.contains_key(&0));
    }

    #[test]
    fn cached_lexical_search_returns_ranked_doc_ids() {
        let uuid_index = BTreeMap::from([
            ("doc-a".to_owned(), "a.json".to_owned()),
            ("doc-b".to_owned(), "b.json".to_owned()),
        ]);
        let lexical = LexicalIndex {
            terms: BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]),
            doc_lengths: BTreeMap::from([(1, 4), (2, 4)]),
            term_frequencies: BTreeMap::from([(
                "budget".to_owned(),
                BTreeMap::from([(1, 1), (2, 4)]),
            )]),
        };
        let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

        assert_eq!(
            index.search_doc_ids("budget", &[], 2),
            vec!["doc-b", "doc-a"]
        );
    }

    #[test]
    fn source_type_filter_is_used_before_global_fill() {
        let uuid_index = BTreeMap::from([
            ("doc-a".to_owned(), "slack/a.json".to_owned()),
            ("doc-b".to_owned(), "github/b.json".to_owned()),
        ]);
        let lexical = LexicalIndex {
            terms: BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]),
            doc_lengths: BTreeMap::from([(1, 4), (2, 4)]),
            term_frequencies: BTreeMap::from([(
                "budget".to_owned(),
                BTreeMap::from([(1, 4), (2, 1)]),
            )]),
        };
        let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

        assert_eq!(
            index.search_doc_ids("budget", &["github".to_owned()], 2),
            vec!["doc-b", "doc-a"]
        );
    }
}
