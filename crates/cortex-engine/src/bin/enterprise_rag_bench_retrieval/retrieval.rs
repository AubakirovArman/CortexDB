use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_engine::Database;
use cortex_storage::indexes::LexicalIndex;

#[derive(Debug)]
pub struct BenchmarkRetrievalIndex {
    lexical: LexicalIndex,
    candidate_to_doc_id: BTreeMap<u32, String>,
    allowed: BTreeSet<u32>,
    doc_count: u64,
    avg_len_q10: u64,
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
        let candidate_to_doc_id = candidate_doc_map(uuid_index, &lexical.doc_lengths);
        let allowed = candidate_to_doc_id.keys().copied().collect::<BTreeSet<_>>();
        let doc_count = allowed.len() as u64;
        let avg_len_q10 = average_len_q10(&lexical.doc_lengths, &allowed);
        Self {
            lexical,
            candidate_to_doc_id,
            allowed,
            doc_count,
            avg_len_q10,
        }
    }

    pub fn search_doc_ids(&self, query: &str, limit: usize) -> Vec<String> {
        let mut scores = BTreeMap::<u32, u64>::new();
        for term in cortex_engine::search::tokenize(query) {
            let Some(posting) = self.lexical.terms.get(&term) else {
                continue;
            };
            let visible_count = posting
                .iter()
                .filter(|id| self.allowed.contains(id))
                .count() as u64;
            if visible_count == 0 {
                continue;
            }
            let idf_q10 = ((self.doc_count + 1) * 1024) / (visible_count + 1);
            for candidate in posting.iter().filter(|id| self.allowed.contains(id)) {
                let tf = u64::from(self.term_frequency(&term, *candidate));
                let len_q10 =
                    u64::from(*self.lexical.doc_lengths.get(candidate).unwrap_or(&1)) * 1024;
                let norm_q10 = 256 + (768 * len_q10 / self.avg_len_q10.max(1));
                let denom_q10 = (tf * 1024) + norm_q10;
                let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
                *scores.entry(*candidate).or_default() += idf_q10 * tf_norm_q10;
            }
        }
        let mut ranked = scores.into_iter().collect::<Vec<_>>();
        ranked.sort_by_key(|(candidate, score)| (Reverse(*score), *candidate));
        ranked
            .into_iter()
            .filter_map(|(candidate, _)| self.candidate_to_doc_id.get(&candidate).cloned())
            .take(limit)
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

fn candidate_doc_map(
    uuid_index: &BTreeMap<String, String>,
    doc_lengths: &BTreeMap<u32, u32>,
) -> BTreeMap<u32, String> {
    let doc_ids = uuid_index.keys().cloned().collect::<Vec<_>>();
    doc_lengths
        .keys()
        .filter_map(|candidate| {
            let ordinal = usize::try_from(*candidate).ok()?.checked_sub(1)?;
            doc_ids
                .get(ordinal)
                .map(|doc_id| (*candidate, doc_id.clone()))
        })
        .collect()
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

        let mapped = candidate_doc_map(&uuid_index, &doc_lengths);

        assert_eq!(mapped.get(&1), Some(&"doc-a".to_owned()));
        assert_eq!(mapped.get(&2), Some(&"doc-b".to_owned()));
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

        assert_eq!(index.search_doc_ids("budget", 2), vec!["doc-b", "doc-a"]);
    }
}
