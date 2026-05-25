use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::atomic::{append_crc32c, verify_crc32c, write_atomic};
use crate::error::{StorageError, StorageResult};

const MAGIC: &[u8; 4] = b"ACM0";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestSegment {
    pub id: u64,
    pub generation: u64,
    pub checkpoint_seq: u64,
    pub cell_count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorageManifest {
    pub generation: u64,
    pub checkpoint_seq: u64,
    pub live_segments: Vec<ManifestSegment>,
    pub retired_segments: Vec<ManifestSegment>,
}

impl StorageManifest {
    pub fn load(path: impl AsRef<Path>) -> StorageResult<Self> {
        match fs::read(path) {
            Ok(bytes) => decode_manifest(&bytes),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn store(&self, path: impl AsRef<Path>) -> StorageResult<()> {
        write_atomic(path.as_ref(), &encode_manifest(self))?;
        Ok(())
    }

    pub fn checkpoint_segment(&mut self, segment: ManifestSegment) {
        self.generation += 1;
        self.checkpoint_seq = segment.checkpoint_seq;
        self.live_segments.push(segment);
    }

    pub fn compact_to_segment(&mut self, segment: ManifestSegment) {
        self.generation += 1;
        self.checkpoint_seq = segment.checkpoint_seq;
        self.retired_segments.append(&mut self.live_segments);
        self.live_segments.push(segment);
    }
}

fn encode_manifest(manifest: &StorageManifest) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    put_u64(&mut out, manifest.generation);
    put_u64(&mut out, manifest.checkpoint_seq);
    put_segments(&mut out, &manifest.live_segments);
    put_segments(&mut out, &manifest.retired_segments);
    append_crc32c(&mut out);
    out
}

fn decode_manifest(bytes: &[u8]) -> StorageResult<StorageManifest> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidManifestFile)?;
    if bytes.len() < 24 || &bytes[..4] != MAGIC {
        return Err(StorageError::InvalidManifestFile);
    }
    let mut cursor = 4;
    let generation = read_u64(bytes, &mut cursor)?;
    let checkpoint_seq = read_u64(bytes, &mut cursor)?;
    let live_segments = read_segments(bytes, &mut cursor)?;
    let retired_segments = read_segments(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(StorageError::InvalidManifestFile);
    }
    Ok(StorageManifest {
        generation,
        checkpoint_seq,
        live_segments,
        retired_segments,
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

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> StorageResult<u32> {
    let raw = read_bytes(bytes, cursor, 4)?;
    Ok(u32::from_le_bytes(raw.try_into().expect("u32 width")))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> StorageResult<u64> {
    let raw = read_bytes(bytes, cursor, 8)?;
    Ok(u64::from_le_bytes(raw.try_into().expect("u64 width")))
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
