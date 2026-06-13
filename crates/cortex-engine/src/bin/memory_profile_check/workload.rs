use std::path::Path;

use cortex_core::{CellDescriptor, CellId};
use cortex_engine::Database;
use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::{ManifestSegment, StorageManifest};
use cortex_storage::segment::{SegmentCellRef, SegmentWriter};
use cortex_storage::vectors::VectorIndex;

pub(super) fn put_profile_cells(
    db: &mut Database,
    cells: usize,
    payload_bytes: usize,
    batch_size: usize,
) -> Result<(), String> {
    let mut next = 1usize;
    while next <= cells {
        let end = next.saturating_add(batch_size).saturating_sub(1).min(cells);
        db.put_cells(build_cells(next, end, payload_bytes))
            .map_err(|error| error.to_string())?;
        next = end + 1;
    }
    Ok(())
}

pub(super) fn prepare_direct_checkpoint(
    root: &Path,
    cells: usize,
    payload_bytes: usize,
    batch_size: usize,
) -> Result<(), String> {
    let segments = root.join("segments");
    std::fs::create_dir_all(&segments)
        .map_err(|error| format!("failed to create {}: {error}", segments.display()))?;
    let mut manifest = StorageManifest::default();
    let mut next = 1usize;
    let mut segment_id = 1u64;

    while next <= cells {
        let end = next.saturating_add(batch_size).saturating_sub(1).min(cells);
        write_segment_batch(&segments, segment_id, next, end, payload_bytes)?;
        manifest.live_segments.push(ManifestSegment {
            id: segment_id,
            generation: segment_id,
            checkpoint_seq: end as u64,
            cell_count: u32::try_from(end - next + 1)
                .map_err(|_| "segment batch cell count overflows u32".to_owned())?,
        });
        manifest.generation = segment_id;
        manifest.checkpoint_seq = end as u64;
        next = end + 1;
        segment_id += 1;
    }

    manifest
        .store(root.join("manifest.acm"))
        .map_err(|error| error.to_string())
}

fn write_segment_batch(
    segments: &Path,
    segment_id: u64,
    start: usize,
    end: usize,
    payload_bytes: usize,
) -> Result<(), String> {
    let payloads = (start..=end)
        .map(|index| build_payload(index, payload_bytes))
        .collect::<Vec<_>>();
    let refs = payloads
        .iter()
        .enumerate()
        .map(|(offset, payload)| {
            let index = start + offset;
            Ok(SegmentCellRef {
                candidate_id: u32::try_from(index)
                    .map_err(|_| "candidate id overflows u32".to_owned())?,
                cell_id: index as u64,
                created_seq: index as u64,
                deleted_seq: None,
                descriptor: Some(CellDescriptor::from_payload_lossy(payload).encode_section_v1()),
                payload,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    SegmentWriter::write_refs(segments.join(format!("segment-{segment_id}.acs")), &refs)
        .map_err(|error| error.to_string())?;
    BitmapIndex::default()
        .write(segments.join(format!("segment-{segment_id}.acb")))
        .map_err(|error| error.to_string())?;
    LexicalIndex::default()
        .write(segments.join(format!("segment-{segment_id}.aci")))
        .map_err(|error| error.to_string())?;
    VectorIndex::default()
        .write(segments.join(format!("segment-{segment_id}.acv")))
        .map_err(|error| error.to_string())?;
    HnswGraphIndex::default()
        .write(segments.join(format!("segment-{segment_id}.ach")))
        .map_err(|error| error.to_string())
}

fn build_cells(start: usize, end: usize, payload_bytes: usize) -> Vec<(CellId, Vec<u8>)> {
    (start..=end)
        .map(|index| (CellId(index as u64), build_payload(index, payload_bytes)))
        .collect()
}

fn build_payload(index: usize, payload_bytes: usize) -> Vec<u8> {
    let mut payload = format!(
        "scope=memory:profile\nstatus=ready\ntype=fact\nsource=memory-profile-{index}\n\nmemory profile payload {index} alpha beta gamma"
    )
    .into_bytes();
    if payload_bytes <= payload.len() {
        return payload;
    }

    payload.push(b'\n');
    let filler = format!("filler-{index:08}-");
    while payload.len() < payload_bytes {
        payload.extend_from_slice(filler.as_bytes());
    }
    payload.truncate(payload_bytes);
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_payload_keeps_legacy_size_when_disabled() {
        let payload = build_payload(7, 0);
        let text = String::from_utf8(payload).unwrap();

        assert!(text.contains("source=memory-profile-7"));
        assert!(text.contains("memory profile payload 7 alpha beta gamma"));
    }

    #[test]
    fn build_payload_pads_to_requested_size() {
        let payload = build_payload(7, 4096);

        assert_eq!(payload.len(), 4096);
        assert!(payload.starts_with(b"scope=memory:profile\n"));
    }

    #[test]
    fn build_cells_uses_inclusive_range() {
        let cells = build_cells(3, 5, 0);

        assert_eq!(cells.len(), 3);
        assert_eq!(cells[0].0, CellId(3));
        assert_eq!(cells[2].0, CellId(5));
    }
}
