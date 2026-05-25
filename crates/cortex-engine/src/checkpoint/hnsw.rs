use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::segment::SegmentCell;

use crate::search::vector::vector_from_payload;
use crate::search::HnswIndex;

pub(super) fn hnsw_graph_for_cells(cells: &[SegmentCell]) -> HnswGraphIndex {
    let mut index = HnswIndex::default();
    for cell in cells.iter().filter(|cell| cell.deleted_seq.is_none()) {
        if let Some(vector) = vector_from_payload(&cell.payload) {
            index.add_vector(cell.candidate_id, vector);
        }
    }
    index.graph_index()
}
