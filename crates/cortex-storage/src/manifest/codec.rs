use crate::atomic::{append_crc32c, verify_crc32c};
use crate::error::{StorageError, StorageResult};
use crate::format::MANIFEST_MAGIC;
use crate::manifest::{
    CompactionMetadata, ManifestHnswNoFallbackProfile, ManifestHnswProfile, ManifestSegment,
    ManifestVectorProfile, StorageManifest,
};

pub(super) fn encode_manifest(manifest: &StorageManifest) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&MANIFEST_MAGIC);
    put_u64(&mut out, manifest.generation);
    put_u64(&mut out, manifest.checkpoint_seq);
    put_segments(&mut out, &manifest.live_segments);
    put_segments(&mut out, &manifest.retired_segments);
    if let Some(profile) = manifest.hnsw_profile {
        out.extend_from_slice(b"HNSW");
        put_u32(&mut out, profile.max_neighbors);
        put_u32(&mut out, profile.ef_search);
        put_u32(&mut out, profile.layer_count);
        put_u32(&mut out, profile.metric);
        put_u32(&mut out, profile.ef_construction.max(profile.ef_search));
    }
    if let Some(profile) = manifest.vector_profile {
        out.extend_from_slice(b"VECM");
        put_u32(&mut out, profile.dimension);
        put_u32(&mut out, profile.metric);
    }
    if let Some(profile) = manifest.hnsw_no_fallback_profile {
        out.extend_from_slice(b"NOFB");
        put_u32(&mut out, bool_to_u32(profile.rollout_enabled));
        put_u32(&mut out, u32::from(profile.min_recall_q16));
        put_u32(&mut out, bool_to_u32(profile.require_upper_layers));
    }
    {
        let meta = manifest.compaction_metadata;
        out.extend_from_slice(b"COMP");
        put_u64(&mut out, meta.triggered);
        put_u64(&mut out, meta.completed);
        put_u64(&mut out, meta.duration_ms);
        put_u64(&mut out, meta.cells_compacted);
        put_u64(&mut out, meta.input_bytes);
    }
    append_crc32c(&mut out);
    out
}

pub(super) fn decode_manifest(bytes: &[u8]) -> StorageResult<StorageManifest> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidManifestFile)?;
    if bytes.len() < 24 || bytes[..4] != MANIFEST_MAGIC {
        return Err(StorageError::InvalidManifestFile);
    }
    let mut cursor = 4;
    let generation = read_u64(bytes, &mut cursor)?;
    let checkpoint_seq = read_u64(bytes, &mut cursor)?;
    let live_segments = read_segments(bytes, &mut cursor)?;
    let retired_segments = read_segments(bytes, &mut cursor)?;
    let hnsw_profile = read_hnsw_profile(bytes, &mut cursor)?;
    let vector_profile = read_vector_profile(bytes, &mut cursor)?;
    let hnsw_no_fallback_profile = read_hnsw_no_fallback_profile(bytes, &mut cursor)?;
    let compaction_metadata = read_compaction_metadata(bytes, &mut cursor)?;
    if cursor > bytes.len() {
        return Err(StorageError::InvalidManifestFile);
    }
    Ok(StorageManifest {
        generation,
        checkpoint_seq,
        live_segments,
        retired_segments,
        hnsw_profile,
        vector_profile,
        hnsw_no_fallback_profile,
        compaction_metadata,
    })
}

fn put_segments(out: &mut Vec<u8>, segments: &[ManifestSegment]) {
    put_u32(out, segments.len() as u32);
    for segment in segments {
        put_u64(out, segment.id);
        put_u64(out, segment.generation);
        put_u64(out, segment.checkpoint_seq);
        put_u32(out, segment.cell_count);
    }
}

fn read_segments(bytes: &[u8], cursor: &mut usize) -> StorageResult<Vec<ManifestSegment>> {
    let count = read_u32(bytes, cursor)? as usize;
    let mut segments = Vec::with_capacity(count);
    for _ in 0..count {
        segments.push(ManifestSegment {
            id: read_u64(bytes, cursor)?,
            generation: read_u64(bytes, cursor)?,
            checkpoint_seq: read_u64(bytes, cursor)?,
            cell_count: read_u32(bytes, cursor)?,
        });
    }
    Ok(segments)
}

fn read_hnsw_profile(
    bytes: &[u8],
    cursor: &mut usize,
) -> StorageResult<Option<ManifestHnswProfile>> {
    if bytes.len().saturating_sub(*cursor) < 4 || &bytes[*cursor..*cursor + 4] != b"HNSW" {
        return Ok(None);
    }
    *cursor += 4;
    let profile = ManifestHnswProfile {
        max_neighbors: read_u32(bytes, cursor)?,
        ef_search: read_u32(bytes, cursor)?,
        layer_count: read_u32(bytes, cursor)?,
        metric: read_u32(bytes, cursor)?,
        ef_construction: 0,
    };
    if profile.max_neighbors == 0 || profile.ef_search == 0 || profile.layer_count == 0 {
        return Err(StorageError::InvalidManifestFile);
    }
    let ef_construction =
        if bytes.len().saturating_sub(*cursor) >= 4 && &bytes[*cursor..*cursor + 4] != b"VECM" {
            let value = read_u32(bytes, cursor)?;
            if value == 0 {
                return Err(StorageError::InvalidManifestFile);
            }
            value
        } else {
            profile.ef_search
        };
    Ok(Some(ManifestHnswProfile {
        ef_construction,
        ..profile
    }))
}

fn read_vector_profile(
    bytes: &[u8],
    cursor: &mut usize,
) -> StorageResult<Option<ManifestVectorProfile>> {
    if bytes.len().saturating_sub(*cursor) < 4 || &bytes[*cursor..*cursor + 4] != b"VECM" {
        return Ok(None);
    }
    *cursor += 4;
    let profile = ManifestVectorProfile {
        dimension: read_u32(bytes, cursor)?,
        metric: read_u32(bytes, cursor)?,
    };
    if profile.dimension == 0 {
        return Err(StorageError::InvalidManifestFile);
    }
    Ok(Some(profile))
}

fn read_hnsw_no_fallback_profile(
    bytes: &[u8],
    cursor: &mut usize,
) -> StorageResult<Option<ManifestHnswNoFallbackProfile>> {
    if bytes.len().saturating_sub(*cursor) < 4 || &bytes[*cursor..*cursor + 4] != b"NOFB" {
        return Ok(None);
    }
    *cursor += 4;
    let rollout_enabled = read_bool(bytes, cursor)?;
    let min_recall_raw = read_u32(bytes, cursor)?;
    let require_upper_layers = read_bool(bytes, cursor)?;
    let min_recall_q16 =
        u16::try_from(min_recall_raw).map_err(|_| StorageError::InvalidManifestFile)?;
    Ok(Some(ManifestHnswNoFallbackProfile {
        rollout_enabled,
        min_recall_q16,
        require_upper_layers,
    }))
}

fn read_compaction_metadata(bytes: &[u8], cursor: &mut usize) -> StorageResult<CompactionMetadata> {
    if bytes.len().saturating_sub(*cursor) < 4 || &bytes[*cursor..*cursor + 4] != b"COMP" {
        return Ok(CompactionMetadata::default());
    }
    *cursor += 4;
    Ok(CompactionMetadata {
        triggered: read_u64(bytes, cursor)?,
        completed: read_u64(bytes, cursor)?,
        duration_ms: read_u64(bytes, cursor)?,
        cells_compacted: read_u64(bytes, cursor)?,
        input_bytes: read_u64(bytes, cursor)?,
    })
}

fn bool_to_u32(value: bool) -> u32 {
    if value {
        1
    } else {
        0
    }
}

fn read_bool(bytes: &[u8], cursor: &mut usize) -> StorageResult<bool> {
    match read_u32(bytes, cursor)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(StorageError::InvalidManifestFile),
    }
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> StorageResult<u32> {
    Ok(u32::from_le_bytes(read_array(bytes, cursor)?))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> StorageResult<u64> {
    Ok(u64::from_le_bytes(read_array(bytes, cursor)?))
}

fn read_array<const N: usize>(bytes: &[u8], cursor: &mut usize) -> StorageResult<[u8; N]> {
    read_bytes(bytes, cursor, N)?
        .try_into()
        .map_err(|_| StorageError::InvalidManifestFile)
}

fn read_bytes<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> StorageResult<&'a [u8]> {
    let end = cursor
        .checked_add(len)
        .ok_or(StorageError::InvalidManifestFile)?;
    if end > bytes.len() {
        return Err(StorageError::InvalidManifestFile);
    }
    let value = &bytes[*cursor..end];
    *cursor = end;
    Ok(value)
}
