use std::path::Path;

use cortex_core::memtable::{MemTable, PayloadRef};
use cortex_core::{CellDescriptor, CellId, CommitSeq};
use cortex_storage::manifest::StorageManifest;
use cortex_storage::segment::SegmentReader;

use crate::error::{EngineError, EngineResult};
use crate::options::PayloadResidency;

use super::manifest_safety;
use super::paths::{manifest_path, segment_path, segments_path};
use super::types::CheckpointLoad;

pub(crate) fn load_checkpoint(
    root: &Path,
    payload_residency: PayloadResidency,
) -> EngineResult<CheckpointLoad> {
    manifest_safety::reject_missing_manifest_with_storage(root)?;
    let manifest = StorageManifest::load(manifest_path(root))?;
    let mut memtable = MemTable::default();
    for segment in &manifest.live_segments {
        match payload_residency {
            PayloadResidency::Memory => load_segment_into_memory(root, segment.id, &mut memtable)?,
            PayloadResidency::Lazy => load_segment_lazy(root, segment.id, &mut memtable)?,
        }
    }
    Ok(CheckpointLoad { manifest, memtable })
}

fn load_segment_into_memory(
    root: &Path,
    segment_id: u64,
    memtable: &mut MemTable,
) -> EngineResult<()> {
    let records = SegmentReader::read_records(segment_path(&segments_path(root), segment_id))?;
    for record in records {
        let cell = record.cell;
        if let Some(deleted) = cell.deleted_seq {
            memtable.record_tombstone(CellId(cell.cell_id), CommitSeq(deleted));
        } else {
            match record.descriptor {
                Some(bytes) => {
                    let descriptor = decode_segment_descriptor(segment_id, cell.cell_id, &bytes)?;
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
    Ok(())
}

fn load_segment_lazy(root: &Path, segment_id: u64, memtable: &mut MemTable) -> EngineResult<()> {
    let path = segment_path(&segments_path(root), segment_id);
    for entry in SegmentReader::read_descriptors(&path)? {
        if let Some(deleted) = entry.deleted_seq {
            memtable.record_tombstone(CellId(entry.cell_id), CommitSeq(deleted));
            continue;
        }

        let Some(bytes) = entry.descriptor else {
            let payload =
                SegmentReader::read_payload_at(&path, entry.candidate_id)?.ok_or_else(|| {
                    EngineError::StorageInvariant(format!(
                        "segment {segment_id} is missing payload for candidate {}",
                        entry.candidate_id
                    ))
                })?;
            memtable.put_cell(CellId(entry.cell_id), CommitSeq(entry.created_seq), payload);
            continue;
        };

        let descriptor = decode_segment_descriptor(segment_id, entry.cell_id, &bytes)?;
        memtable.put_cell_with_payload_ref(
            CellId(entry.cell_id),
            CommitSeq(entry.created_seq),
            descriptor,
            PayloadRef::Segment {
                segment_id,
                candidate_id: entry.candidate_id,
                offset: entry.payload_offset,
                len: entry.payload_len,
                crc32c: entry.payload_crc32c,
            },
        );
    }
    Ok(())
}

fn decode_segment_descriptor(
    segment_id: u64,
    cell_id: u64,
    bytes: &[u8],
) -> EngineResult<CellDescriptor> {
    CellDescriptor::decode_section_v1(bytes).ok_or_else(|| {
        EngineError::StorageInvariant(format!(
            "segment {segment_id} contains an invalid cell descriptor for cell {cell_id}"
        ))
    })
}
