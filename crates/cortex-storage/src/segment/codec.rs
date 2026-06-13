use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::atomic::{append_crc32c, verify_crc32c};
use crate::error::{StorageError, StorageResult};
use crate::format::{LEGACY_SEGMENT_MAGIC, LEGACY_SEGMENT_V2_MAGIC, SEGMENT_MAGIC};
use crate::wal::checksum::crc32c;

use super::{
    SegmentCandidateEntry, SegmentCell, SegmentCellRecord, SegmentCellRef, SegmentDescriptorEntry,
};

const SEGMENT_HEADER_LEN: usize = 8;
const SEGMENT_FILE_CRC_LEN: usize = 4;
const SEGMENT_FOOTER_MAGIC: [u8; 4] = *b"ASF2";
const SEGMENT_FOOTER_TRAILER_LEN: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FooterBounds {
    start: usize,
    trailer_start: usize,
}

pub(super) fn encode_segment_refs(cells: &[SegmentCellRef<'_>]) -> StorageResult<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(&SEGMENT_MAGIC);
    put_u32(&mut out, checked_u32(cells.len())?);

    let mut ordered = cells.to_vec();
    ordered.sort_by_key(|cell| cell.candidate_id);

    let mut entries = Vec::with_capacity(ordered.len());
    for cell in ordered {
        let payload_offset = checked_u64(out.len())?;
        out.extend_from_slice(cell.payload);
        entries.push(SegmentDescriptorEntry {
            candidate_id: cell.candidate_id,
            cell_id: cell.cell_id,
            created_seq: cell.created_seq,
            deleted_seq: cell.deleted_seq,
            descriptor: cell.descriptor,
            payload_offset,
            payload_len: checked_u64(cell.payload.len())?,
            payload_crc32c: crc32c(cell.payload),
        });
    }

    let footer = encode_footer(&entries)?;
    let footer_crc = crc32c(&footer);
    out.extend_from_slice(&footer);
    put_u64(&mut out, checked_u64(footer.len())?);
    put_u32(&mut out, footer_crc);
    out.extend_from_slice(&SEGMENT_FOOTER_MAGIC);
    append_crc32c(&mut out);
    Ok(out)
}

pub(super) fn decode_segment_records(bytes: &[u8]) -> StorageResult<Vec<SegmentCellRecord>> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidSegmentFile)?;
    if bytes.len() < SEGMENT_HEADER_LEN {
        return Err(StorageError::InvalidSegmentFile);
    }
    if bytes[..4] == LEGACY_SEGMENT_MAGIC {
        return decode_linear_segment_records(bytes, false);
    }
    if bytes[..4] == LEGACY_SEGMENT_V2_MAGIC {
        return decode_linear_segment_records(bytes, true);
    }
    if bytes[..4] != SEGMENT_MAGIC {
        return Err(StorageError::InvalidSegmentFile);
    }
    match footer_bounds(bytes)? {
        Some(bounds) => decode_footer_segment_records(bytes, bounds),
        None => Err(StorageError::InvalidSegmentFile),
    }
}

pub(super) fn decode_segment_candidate_entries(
    bytes: &[u8],
) -> StorageResult<Vec<SegmentCandidateEntry>> {
    decode_segment_descriptor_entries(bytes).map(|entries| {
        entries
            .into_iter()
            .map(|entry| SegmentCandidateEntry {
                candidate_id: entry.candidate_id,
                cell_id: entry.cell_id,
                deleted: entry.deleted_seq.is_some(),
            })
            .collect()
    })
}

pub(super) fn read_segment_descriptors(path: &Path) -> StorageResult<Vec<SegmentDescriptorEntry>> {
    match read_footer_entries_from_file(path)? {
        Some(entries) => Ok(entries),
        None => {
            let bytes = std::fs::read(path)?;
            decode_segment_descriptor_entries(&bytes)
        }
    }
}

pub(super) fn read_segment_payload_at(
    path: &Path,
    candidate_id: u32,
) -> StorageResult<Option<Vec<u8>>> {
    if let Some(entries) = read_footer_entries_from_file(path)? {
        let Some(entry) = entries
            .iter()
            .find(|entry| entry.candidate_id == candidate_id)
        else {
            return Ok(None);
        };
        let mut file = File::open(path)?;
        return read_payload_block(&mut file, entry).map(Some);
    }

    let records = decode_segment_records(&std::fs::read(path)?)?;
    Ok(records
        .into_iter()
        .find(|record| record.cell.candidate_id == candidate_id)
        .map(|record| record.cell.payload))
}

fn decode_segment_descriptor_entries(bytes: &[u8]) -> StorageResult<Vec<SegmentDescriptorEntry>> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidSegmentFile)?;
    if bytes.len() < SEGMENT_HEADER_LEN {
        return Err(StorageError::InvalidSegmentFile);
    }
    if bytes[..4] == LEGACY_SEGMENT_MAGIC {
        return decode_linear_segment_descriptors(bytes, false);
    }
    if bytes[..4] == LEGACY_SEGMENT_V2_MAGIC {
        return decode_linear_segment_descriptors(bytes, true);
    }
    if bytes[..4] != SEGMENT_MAGIC {
        return Err(StorageError::InvalidSegmentFile);
    }
    match footer_bounds(bytes)? {
        Some(bounds) => decode_footer_entries(&bytes[bounds.start..bounds.trailer_start]),
        None => Err(StorageError::InvalidSegmentFile),
    }
}

fn decode_footer_segment_records(
    bytes: &[u8],
    bounds: FooterBounds,
) -> StorageResult<Vec<SegmentCellRecord>> {
    let entries = decode_footer_entries(&bytes[bounds.start..bounds.trailer_start])?;
    let mut records = Vec::with_capacity(entries.len());
    for entry in entries {
        let payload = payload_slice(bytes, bounds.start, &entry)?.to_vec();
        if crc32c(&payload) != entry.payload_crc32c {
            return Err(StorageError::InvalidSegmentFile);
        }
        records.push(SegmentCellRecord {
            cell: SegmentCell {
                candidate_id: entry.candidate_id,
                cell_id: entry.cell_id,
                created_seq: entry.created_seq,
                deleted_seq: entry.deleted_seq,
                payload,
            },
            descriptor: entry.descriptor,
        });
    }
    Ok(records)
}

fn decode_linear_segment_records(
    bytes: &[u8],
    is_v2: bool,
) -> StorageResult<Vec<SegmentCellRecord>> {
    let mut cursor = 4;
    let count = read_u32(bytes, &mut cursor)? as usize;
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let cell_id = read_u64(bytes, &mut cursor)?;
        let candidate_id = read_u32(bytes, &mut cursor)?;
        let created_seq = read_u64(bytes, &mut cursor)?;
        let deleted_seq = decode_deleted_seq(read_u64(bytes, &mut cursor)?);
        let descriptor = if is_v2 {
            read_optional_bytes(bytes, &mut cursor)?
        } else {
            None
        };
        let len = read_u32(bytes, &mut cursor)? as usize;
        let payload = read_bytes(bytes, &mut cursor, len)?.to_vec();
        records.push(SegmentCellRecord {
            cell: SegmentCell {
                candidate_id,
                cell_id,
                created_seq,
                deleted_seq,
                payload,
            },
            descriptor,
        });
    }
    ensure_consumed(bytes, cursor)?;
    Ok(records)
}

fn decode_linear_segment_descriptors(
    bytes: &[u8],
    is_v2: bool,
) -> StorageResult<Vec<SegmentDescriptorEntry>> {
    let mut cursor = 4;
    let count = read_u32(bytes, &mut cursor)? as usize;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let cell_id = read_u64(bytes, &mut cursor)?;
        let candidate_id = read_u32(bytes, &mut cursor)?;
        let created_seq = read_u64(bytes, &mut cursor)?;
        let deleted_seq = decode_deleted_seq(read_u64(bytes, &mut cursor)?);
        let descriptor = if is_v2 {
            read_optional_bytes(bytes, &mut cursor)?
        } else {
            None
        };
        let payload_len = read_u32(bytes, &mut cursor)? as usize;
        let payload_offset = checked_u64(cursor)?;
        let payload = read_bytes(bytes, &mut cursor, payload_len)?;
        entries.push(SegmentDescriptorEntry {
            candidate_id,
            cell_id,
            created_seq,
            deleted_seq,
            descriptor,
            payload_offset,
            payload_len: checked_u64(payload_len)?,
            payload_crc32c: crc32c(payload),
        });
    }
    ensure_consumed(bytes, cursor)?;
    Ok(entries)
}

fn read_footer_entries_from_file(
    path: &Path,
) -> StorageResult<Option<Vec<SegmentDescriptorEntry>>> {
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    if file_len < (SEGMENT_HEADER_LEN + SEGMENT_FOOTER_TRAILER_LEN + SEGMENT_FILE_CRC_LEN) as u64 {
        return Ok(None);
    }

    let mut header = [0_u8; SEGMENT_HEADER_LEN];
    file.read_exact(&mut header)?;
    if header[..4] != SEGMENT_MAGIC {
        return Ok(None);
    }
    let expected_count = u32::from_le_bytes(
        header[4..8]
            .try_into()
            .map_err(|_| StorageError::InvalidSegmentFile)?,
    ) as usize;

    let mut trailer = [0_u8; SEGMENT_FOOTER_TRAILER_LEN + SEGMENT_FILE_CRC_LEN];
    file.seek(SeekFrom::End(-(trailer.len() as i64)))?;
    file.read_exact(&mut trailer)?;
    if trailer[12..16] != SEGMENT_FOOTER_MAGIC {
        return Ok(None);
    }

    let footer_len = u64::from_le_bytes(
        trailer[0..8]
            .try_into()
            .map_err(|_| StorageError::InvalidSegmentFile)?,
    );
    let footer_crc = u32::from_le_bytes(
        trailer[8..12]
            .try_into()
            .map_err(|_| StorageError::InvalidSegmentFile)?,
    );
    let trailer_and_crc_len = (SEGMENT_FOOTER_TRAILER_LEN + SEGMENT_FILE_CRC_LEN) as u64;
    let footer_start = file_len
        .checked_sub(trailer_and_crc_len)
        .and_then(|value| value.checked_sub(footer_len))
        .ok_or(StorageError::InvalidSegmentFile)?;
    if footer_start < SEGMENT_HEADER_LEN as u64 {
        return Err(StorageError::InvalidSegmentFile);
    }

    let mut footer = vec![0_u8; checked_usize(footer_len)?];
    file.seek(SeekFrom::Start(footer_start))?;
    file.read_exact(&mut footer)?;
    if crc32c(&footer) != footer_crc {
        return Err(StorageError::InvalidSegmentFile);
    }
    let entries = decode_footer_entries(&footer)?;
    if entries.len() != expected_count {
        return Err(StorageError::InvalidSegmentFile);
    }
    Ok(Some(entries))
}

fn read_payload_block(file: &mut File, entry: &SegmentDescriptorEntry) -> StorageResult<Vec<u8>> {
    let mut payload = vec![0_u8; checked_usize(entry.payload_len)?];
    file.seek(SeekFrom::Start(entry.payload_offset))?;
    file.read_exact(&mut payload)?;
    if crc32c(&payload) != entry.payload_crc32c {
        return Err(StorageError::InvalidSegmentFile);
    }
    Ok(payload)
}

fn footer_bounds(bytes: &[u8]) -> StorageResult<Option<FooterBounds>> {
    if bytes.len() < SEGMENT_HEADER_LEN + SEGMENT_FOOTER_TRAILER_LEN {
        return Ok(None);
    }
    let marker_start = bytes.len() - 4;
    if bytes[marker_start..] != SEGMENT_FOOTER_MAGIC {
        return Ok(None);
    }
    let footer_crc_start = marker_start - 4;
    let footer_len_start = footer_crc_start - 8;
    let footer_len = read_u64_at(bytes, footer_len_start)?;
    let footer_start = footer_len_start
        .checked_sub(checked_usize(footer_len)?)
        .ok_or(StorageError::InvalidSegmentFile)?;
    if footer_start < SEGMENT_HEADER_LEN {
        return Err(StorageError::InvalidSegmentFile);
    }
    let expected_crc = read_u32_at(bytes, footer_crc_start)?;
    let footer = &bytes[footer_start..footer_len_start];
    if crc32c(footer) != expected_crc {
        return Err(StorageError::InvalidSegmentFile);
    }
    Ok(Some(FooterBounds {
        start: footer_start,
        trailer_start: footer_len_start,
    }))
}

fn encode_footer(entries: &[SegmentDescriptorEntry]) -> StorageResult<Vec<u8>> {
    let mut out = Vec::new();
    put_u32(&mut out, checked_u32(entries.len())?);
    for entry in entries {
        put_u32(&mut out, entry.candidate_id);
        put_u64(&mut out, entry.cell_id);
        put_u64(&mut out, entry.created_seq);
        put_u64(&mut out, entry.deleted_seq.unwrap_or(0));
        put_u64(&mut out, entry.payload_offset);
        put_u64(&mut out, entry.payload_len);
        put_u32(&mut out, entry.payload_crc32c);
        let descriptor = entry.descriptor.as_deref().unwrap_or(&[]);
        put_u32(&mut out, checked_u32(descriptor.len())?);
        out.extend_from_slice(descriptor);
    }
    Ok(out)
}

fn decode_footer_entries(bytes: &[u8]) -> StorageResult<Vec<SegmentDescriptorEntry>> {
    let mut cursor = 0;
    let count = read_u32(bytes, &mut cursor)? as usize;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let candidate_id = read_u32(bytes, &mut cursor)?;
        let cell_id = read_u64(bytes, &mut cursor)?;
        let created_seq = read_u64(bytes, &mut cursor)?;
        let deleted_seq = decode_deleted_seq(read_u64(bytes, &mut cursor)?);
        let payload_offset = read_u64(bytes, &mut cursor)?;
        let payload_len = read_u64(bytes, &mut cursor)?;
        let payload_crc32c = read_u32(bytes, &mut cursor)?;
        let descriptor = read_optional_bytes(bytes, &mut cursor)?;
        entries.push(SegmentDescriptorEntry {
            candidate_id,
            cell_id,
            created_seq,
            deleted_seq,
            descriptor,
            payload_offset,
            payload_len,
            payload_crc32c,
        });
    }
    ensure_consumed(bytes, cursor)?;
    Ok(entries)
}

fn payload_slice<'a>(
    bytes: &'a [u8],
    footer_start: usize,
    entry: &SegmentDescriptorEntry,
) -> StorageResult<&'a [u8]> {
    let start = checked_usize(entry.payload_offset)?;
    if start < SEGMENT_HEADER_LEN {
        return Err(StorageError::InvalidSegmentFile);
    }
    let len = checked_usize(entry.payload_len)?;
    let end = start
        .checked_add(len)
        .ok_or(StorageError::InvalidSegmentFile)?;
    if end > footer_start {
        return Err(StorageError::InvalidSegmentFile);
    }
    Ok(&bytes[start..end])
}

fn decode_deleted_seq(value: u64) -> Option<u64> {
    match value {
        0 => None,
        value => Some(value),
    }
}

fn read_optional_bytes(bytes: &[u8], cursor: &mut usize) -> StorageResult<Option<Vec<u8>>> {
    let len = read_u32(bytes, cursor)? as usize;
    let value = read_bytes(bytes, cursor, len)?;
    Ok((!value.is_empty()).then(|| value.to_vec()))
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

fn read_u32_at(bytes: &[u8], offset: usize) -> StorageResult<u32> {
    let mut cursor = offset;
    read_u32(bytes, &mut cursor)
}

fn read_u64_at(bytes: &[u8], offset: usize) -> StorageResult<u64> {
    let mut cursor = offset;
    read_u64(bytes, &mut cursor)
}

fn read_array<const N: usize>(bytes: &[u8], cursor: &mut usize) -> StorageResult<[u8; N]> {
    read_bytes(bytes, cursor, N)?
        .try_into()
        .map_err(|_| StorageError::InvalidSegmentFile)
}

fn read_bytes<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> StorageResult<&'a [u8]> {
    let end = cursor
        .checked_add(len)
        .ok_or(StorageError::InvalidSegmentFile)?;
    if end > bytes.len() {
        return Err(StorageError::InvalidSegmentFile);
    }
    let value = &bytes[*cursor..end];
    *cursor = end;
    Ok(value)
}

fn ensure_consumed(bytes: &[u8], cursor: usize) -> StorageResult<()> {
    if cursor != bytes.len() {
        return Err(StorageError::InvalidSegmentFile);
    }
    Ok(())
}

fn checked_u32(value: usize) -> StorageResult<u32> {
    u32::try_from(value).map_err(|_| StorageError::InvalidSegmentFile)
}

fn checked_u64(value: usize) -> StorageResult<u64> {
    u64::try_from(value).map_err(|_| StorageError::InvalidSegmentFile)
}

fn checked_usize(value: u64) -> StorageResult<usize> {
    usize::try_from(value).map_err(|_| StorageError::InvalidSegmentFile)
}
