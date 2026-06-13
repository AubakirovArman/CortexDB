use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use cortex_core::CellDescriptor;
use cortex_engine::EngineAqlIndex;
use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::manifest::{ManifestSegment, StorageManifest};
use cortex_storage::segment::{SegmentCellRef, SegmentWriter};
use cortex_storage::vectors::VectorIndex;
use serde_json::{json, Value};

use crate::args::Args;
use crate::metrics::round_ms;
use crate::workload::payload;

pub(crate) fn prepare_direct_checkpoint(db_path: &Path, args: &Args) -> Result<Value, String> {
    let started = Instant::now();
    fs::create_dir_all(segments_path(db_path))
        .map_err(|error| format!("failed to create {}: {error}", db_path.display()))?;

    let mut manifest = StorageManifest::default();
    let mut next = 1usize;
    let mut segment_id = 1u64;
    while next <= args.cells {
        let end = next
            .saturating_add(args.batch_size)
            .saturating_sub(1)
            .min(args.cells);
        write_segment_batch(db_path, segment_id, next, end, args.payload_bytes)?;
        manifest.checkpoint_segment(ManifestSegment {
            id: segment_id,
            generation: segment_id,
            checkpoint_seq: end as u64,
            cell_count: u32::try_from(end - next + 1)
                .map_err(|_| "segment cell_count exceeds u32".to_owned())?,
        });
        next = end + 1;
        segment_id += 1;
    }
    manifest
        .store(manifest_path(db_path))
        .map_err(|error| format!("failed to store manifest: {error}"))?;

    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(json!({
        "name": "direct_checkpoint",
        "units": args.cells,
        "segments": manifest.live_segments.len(),
        "batch_size": args.batch_size,
        "elapsed_ms": round_ms(elapsed_ms),
        "throughput_per_sec": if elapsed_ms <= 0.0 {
            0.0
        } else {
            round_ms((args.cells as f64) / (elapsed_ms / 1000.0))
        },
    }))
}

fn write_segment_batch(
    db_path: &Path,
    segment_id: u64,
    start: usize,
    end: usize,
    payload_bytes: Option<usize>,
) -> Result<(), String> {
    let payloads = (start..=end)
        .map(|index| payload(index, payload_bytes))
        .collect::<Vec<_>>();
    let refs = payloads
        .iter()
        .enumerate()
        .map(|(offset, payload)| {
            let index = start + offset;
            let candidate_id =
                u32::try_from(index).map_err(|_| "candidate id exceeds u32".to_owned())?;
            Ok(SegmentCellRef {
                candidate_id,
                cell_id: index as u64,
                created_seq: index as u64,
                deleted_seq: None,
                descriptor: Some(CellDescriptor::from_payload_lossy(payload).encode_section_v1()),
                payload,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    SegmentWriter::write_refs(segment_path(db_path, segment_id), &refs)
        .map_err(|error| format!("failed to write segment {segment_id}: {error}"))?;
    let index = EngineAqlIndex::try_from_segment_cell_refs(&refs)
        .map_err(|error| format!("failed to build index {segment_id}: {error}"))?;
    index
        .bitmap_index()
        .write(bitmap_path(db_path, segment_id))
        .map_err(|error| format!("failed to write bitmap index {segment_id}: {error}"))?;
    index
        .lexical_index()
        .write(lexical_path(db_path, segment_id))
        .map_err(|error| format!("failed to write lexical index {segment_id}: {error}"))?;
    VectorIndex::default()
        .write(vector_path(db_path, segment_id))
        .map_err(|error| format!("failed to write vector index {segment_id}: {error}"))?;
    HnswGraphIndex::default()
        .write(hnsw_path(db_path, segment_id))
        .map_err(|error| format!("failed to write hnsw graph {segment_id}: {error}"))?;
    Ok(())
}

fn manifest_path(db_path: &Path) -> PathBuf {
    db_path.join("manifest.acm")
}

fn segments_path(db_path: &Path) -> PathBuf {
    db_path.join("segments")
}

fn segment_path(db_path: &Path, id: u64) -> PathBuf {
    segments_path(db_path).join(format!("segment-{id}.acs"))
}

fn bitmap_path(db_path: &Path, id: u64) -> PathBuf {
    segments_path(db_path).join(format!("segment-{id}.acb"))
}

fn lexical_path(db_path: &Path, id: u64) -> PathBuf {
    segments_path(db_path).join(format!("segment-{id}.aci"))
}

fn vector_path(db_path: &Path, id: u64) -> PathBuf {
    segments_path(db_path).join(format!("segment-{id}.acv"))
}

fn hnsw_path(db_path: &Path, id: u64) -> PathBuf {
    segments_path(db_path).join(format!("segment-{id}.ach"))
}
