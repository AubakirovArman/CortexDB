use std::collections::BTreeSet;

use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::segment::{SegmentCell, SegmentReader};

use crate::database::Database;
use crate::error::EngineResult;
use crate::search::vector::vector_from_payload;
use crate::search::HnswIndex;

use super::{hnsw_path, segment_path};

pub(crate) fn hnsw_graph_for_cells(cells: &[SegmentCell]) -> HnswGraphIndex {
    let mut index = HnswIndex::default();
    for cell in cells.iter().filter(|cell| cell.deleted_seq.is_none()) {
        if let Some(vector) = vector_from_payload(&cell.payload) {
            index.add_vector(cell.candidate_id, vector);
        }
    }
    index.graph_index()
}

impl Database {
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
