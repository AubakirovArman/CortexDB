use std::cmp::Reverse;
use std::collections::BTreeMap;

use super::overview::{
    is_overview_candidate_path, is_overview_query, overview_path_score, OverviewQueryProfile,
};

pub struct BenchmarkMetadataIndex {
    doc_paths: Vec<(String, String)>,
}

impl BenchmarkMetadataIndex {
    pub fn from_uuid_index(uuid_index: &BTreeMap<String, String>) -> Self {
        Self {
            doc_paths: uuid_index
                .iter()
                .filter_map(|(doc_id, rel_path)| {
                    let rel_path = rel_path.to_lowercase();
                    is_overview_candidate_path(&rel_path).then_some((doc_id.clone(), rel_path))
                })
                .collect(),
        }
    }

    pub fn search_doc_ids(&self, query: &str, limit: usize) -> Vec<String> {
        if !is_overview_query(query) || limit == 0 {
            return Vec::new();
        }
        let profile = OverviewQueryProfile::new(query);
        let mut scored = self
            .doc_paths
            .iter()
            .filter_map(|(doc_id, rel_path)| {
                let score = overview_path_score(&profile, rel_path);
                (score > 0).then_some((doc_id.clone(), score))
            })
            .collect::<Vec<_>>();
        scored.sort_by_key(|(doc_id, score)| (Reverse(*score), doc_id.clone()));
        scored
            .into_iter()
            .map(|(doc_id, _)| doc_id)
            .take(limit)
            .collect()
    }
}
