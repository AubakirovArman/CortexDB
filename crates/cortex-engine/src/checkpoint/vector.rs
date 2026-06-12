use std::collections::BTreeSet;

use cortex_storage::manifest::ManifestVectorProfile;
use cortex_storage::segment::{SegmentCell, SegmentCellRef, SegmentReader};
use cortex_storage::vectors::VectorIndex;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::search::vector::vector_from_payload;
use crate::search::HnswBuildConfig;

use super::{segment_path, vector_path};

pub(crate) fn vector_index_for_cells(cells: &[SegmentCell]) -> VectorIndex {
    let refs = cells.iter().map(SegmentCellRef::from).collect::<Vec<_>>();
    vector_index_for_cell_refs(&refs)
}

pub(crate) fn vector_index_for_cell_refs(cells: &[SegmentCellRef<'_>]) -> VectorIndex {
    let vectors = cells
        .iter()
        .filter(|cell| cell.deleted_seq.is_none())
        .filter_map(|cell| {
            vector_from_payload(cell.payload).map(|vector| (cell.candidate_id, vector))
        })
        .collect();
    VectorIndex { vectors }
}

pub(crate) fn vector_profile_for_cells(
    cells: &[SegmentCell],
    config: HnswBuildConfig,
) -> EngineResult<Option<ManifestVectorProfile>> {
    let refs = cells.iter().map(SegmentCellRef::from).collect::<Vec<_>>();
    vector_profile_for_cell_refs(&refs, config)
}

pub(crate) fn vector_profile_for_cell_refs(
    cells: &[SegmentCellRef<'_>],
    config: HnswBuildConfig,
) -> EngineResult<Option<ManifestVectorProfile>> {
    let mut dimension = None;
    for cell in cells.iter().filter(|cell| cell.deleted_seq.is_none()) {
        let Some(vector) = vector_from_payload(cell.payload) else {
            continue;
        };
        match dimension {
            Some(expected) if expected != vector.len() => {
                return Err(EngineError::VectorDimensionMismatch {
                    expected,
                    actual: vector.len(),
                });
            }
            Some(_) => {}
            None => dimension = Some(vector.len()),
        }
    }
    let Some(dimension) = dimension else {
        return Ok(None);
    };
    Ok(Some(ManifestVectorProfile {
        dimension: u32::try_from(dimension).map_err(|_| {
            EngineError::StorageInvariant(format!(
                "vector dimension {dimension} exceeds manifest profile range"
            ))
        })?,
        metric: config.normalized().metric as u32,
    }))
}

impl Database {
    pub(crate) fn persisted_vector_index(&self) -> EngineResult<VectorIndex> {
        let mut index = VectorIndex::default();
        let mut tombstoned = BTreeSet::new();
        for segment in &self.manifest.live_segments {
            let cells = SegmentReader::read(segment_path(&self.segments_path, segment.id))?;
            let candidates = cells.iter().map(|cell| cell.candidate_id).collect();
            remove_candidates(&mut index, &candidates);
            for cell in cells {
                if cell.deleted_seq.is_some() {
                    tombstoned.insert(cell.candidate_id);
                } else {
                    tombstoned.remove(&cell.candidate_id);
                }
            }
            merge_vector_index(
                &mut index,
                VectorIndex::read(vector_path(&self.segments_path, segment.id))?,
            );
        }
        remove_candidates(&mut index, &tombstoned);
        Ok(index)
    }
}

fn merge_vector_index(dst: &mut VectorIndex, src: VectorIndex) {
    dst.vectors.extend(src.vectors);
}

fn remove_candidates(index: &mut VectorIndex, candidates: &BTreeSet<u32>) {
    index
        .vectors
        .retain(|candidate, _| !candidates.contains(candidate));
}
