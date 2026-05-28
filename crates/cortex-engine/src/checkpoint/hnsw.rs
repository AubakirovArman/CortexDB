use std::collections::BTreeSet;

use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::segment::{SegmentCell, SegmentReader};

use crate::database::Database;
use crate::error::EngineResult;
use crate::search::vector::vector_from_payload;
use crate::search::{AnnMetrics, DistanceMetric, HnswIndex, VectorCollectionConfig};

use super::{hnsw_path, segment_path};

pub(crate) fn hnsw_graph_for_cells(cells: &[SegmentCell]) -> HnswGraphIndex {
    let mut index = HnswIndex::default();
    let mut dimension = 0usize;
    for cell in cells.iter().filter(|cell| cell.deleted_seq.is_none()) {
        if let Some(vector) = vector_from_payload(&cell.payload) {
            if dimension == 0 {
                dimension = vector.len();
                index.set_config(VectorCollectionConfig {
                    dimension,
                    metric: DistanceMetric::DotProduct,
                });
            }
            index.add_vector(cell.candidate_id, vector);
        }
    }
    index.graph_index()
}

impl Database {
    pub fn ann_metrics(&self) -> AnnMetrics {
        let persisted_segments = self.manifest.live_segments.len();
        let has_checkpoint = self.manifest.checkpoint_seq > 0;
        let (graph_nodes, total_edges) = self
            .persisted_hnsw_graph()
            .map(|graph| {
                (
                    graph.links.len(),
                    graph.links.values().map(|v| v.len()).sum(),
                )
            })
            .unwrap_or((0, 0));
        let has_uncheckpointed_changes = if self.persisted_vector_index().is_ok() {
            !self
                .memtable
                .changed_cell_ids_after(cortex_core::CommitSeq(self.manifest.checkpoint_seq))
                .is_empty()
        } else {
            false
        };
        let mut deleted = BTreeSet::new();
        let mut active = BTreeSet::new();
        for segment in &self.manifest.live_segments {
            if let Ok(cells) = SegmentReader::read(segment_path(&self.segments_path, segment.id)) {
                for cell in cells {
                    if cell.deleted_seq.is_some() {
                        deleted.insert(cell.candidate_id);
                    } else {
                        active.insert(cell.candidate_id);
                        deleted.remove(&cell.candidate_id);
                    }
                }
            }
        }
        let deleted_vectors = deleted.len();
        AnnMetrics {
            graph_nodes,
            total_edges,
            persisted_segments,
            has_checkpoint,
            has_uncheckpointed_changes,
            deleted_vectors,
            rebuild_count: 0,
        }
    }

    pub(crate) fn persisted_hnsw_graph(&self) -> EngineResult<HnswGraphIndex> {
        let mut graph = HnswGraphIndex::default();
        let mut tombstoned = BTreeSet::new();
        for segment in &self.manifest.live_segments {
            let cells = SegmentReader::read(segment_path(&self.segments_path, segment.id))?;
            let candidates = cells
                .iter()
                .map(|cell| cell.candidate_id)
                .collect::<BTreeSet<_>>();
            remove_candidates(&mut graph, &candidates);
            for cell in cells {
                if cell.deleted_seq.is_some() {
                    tombstoned.insert(cell.candidate_id);
                } else {
                    tombstoned.remove(&cell.candidate_id);
                }
            }
            merge_graph(
                &mut graph,
                HnswGraphIndex::read(hnsw_path(&self.segments_path, segment.id))?,
            );
        }
        remove_candidates(&mut graph, &tombstoned);
        Ok(graph)
    }
}

fn merge_graph(dst: &mut HnswGraphIndex, src: HnswGraphIndex) {
    for (candidate, neighbors) in src.links {
        dst.links.entry(candidate).or_default().extend(neighbors);
    }
}

fn remove_candidates(graph: &mut HnswGraphIndex, candidates: &BTreeSet<u32>) {
    graph.links.retain(|id, _| !candidates.contains(id));
    for neighbors in graph.links.values_mut() {
        neighbors.retain(|id| !candidates.contains(id));
    }
}
