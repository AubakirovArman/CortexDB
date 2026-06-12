use std::path::Path;

use cortex_core::memtable::MemTable;
use cortex_core::{CellDescriptor, CellId, CommitSeq};
use cortex_storage::manifest::StorageManifest;
use cortex_storage::segment::SegmentReader;

use crate::error::{EngineError, EngineResult};

use super::manifest_safety;
use super::paths::{manifest_path, segment_path, segments_path};
use super::types::CheckpointLoad;

pub(crate) fn load_checkpoint(root: &Path) -> EngineResult<CheckpointLoad> {
    manifest_safety::reject_missing_manifest_with_storage(root)?;
    let manifest = StorageManifest::load(manifest_path(root))?;
    let mut memtable = MemTable::default();
    for segment in &manifest.live_segments {
        let records = SegmentReader::read_records(segment_path(&segments_path(root), segment.id))?;
        for record in records {
            let cell = record.cell;
            if let Some(deleted) = cell.deleted_seq {
                memtable.record_tombstone(CellId(cell.cell_id), CommitSeq(deleted));
            } else {
                match record.descriptor {
                    Some(bytes) => {
                        let descriptor =
                            CellDescriptor::decode_section_v1(&bytes).ok_or_else(|| {
                                EngineError::StorageInvariant(format!(
                                    "segment {} contains an invalid cell descriptor for cell {}",
                                    segment.id, cell.cell_id
                                ))
                            })?;
                        memtable.put_cell_with_descriptor(
                            CellId(cell.cell_id),
                            CommitSeq(cell.created_seq),
                            cell.payload,
                            descriptor,
                        );
                    }
                    None => {
                        memtable.put_cell(
                            CellId(cell.cell_id),
                            CommitSeq(cell.created_seq),
                            cell.payload,
                        );
                    }
                }
            }
        }
    }
    Ok(CheckpointLoad { manifest, memtable })
}
