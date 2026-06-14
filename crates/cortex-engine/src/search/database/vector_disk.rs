use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::segment::SegmentReader;
use cortex_storage::vectors::VectorIndexReader;

use crate::checkpoint::{segment_path, vector_path};
use crate::database::Database;
use crate::error::EngineResult;

use super::super::hnsw::DistanceMetric;
use super::super::persisted::search_persisted_vector_reader;
use super::super::{ranked, ScoredCandidate};

impl Database {
    pub(super) fn search_disk_resident_vectors(
        &self,
        query: &[i16],
        allowed: &BTreeSet<u32>,
        limit: usize,
        metric: &DistanceMetric,
    ) -> EngineResult<Vec<ScoredCandidate>> {
        let mut scores = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for segment in self.manifest().live_segments.iter().rev() {
            let entries = SegmentReader::read_candidate_entries(segment_path(
                &self.segments_path,
                segment.id,
            ))?;
            let mut segment_allowed = BTreeSet::new();
            for entry in &entries {
                if seen.contains(&entry.candidate_id) {
                    continue;
                }
                if !entry.deleted && allowed.contains(&entry.candidate_id) {
                    segment_allowed.insert(entry.candidate_id);
                }
            }
            for entry in entries {
                seen.insert(entry.candidate_id);
            }
            if segment_allowed.is_empty() {
                continue;
            }
            let mut reader = VectorIndexReader::open(vector_path(&self.segments_path, segment.id))?;
            for candidate in
                search_persisted_vector_reader(&mut reader, query, &segment_allowed, limit, metric)?
            {
                scores.insert(candidate.cell_id, candidate.score);
            }
        }
        Ok(ranked(scores, limit))
    }
}
